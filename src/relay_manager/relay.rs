use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Weak,
    },
};

use tokio::{runtime::Runtime, sync::mpsc, task::JoinHandle};

use crate::{
    connection::{player_net_api::CustomRelayData, shared_connection::SharedConnection},
    packet::Packet,
};
#[derive(Debug)]
pub struct SharedRelayRoomData {
    pub id: String,
    pub start_game: Arc<AtomicBool>,
    pub custom: CustomRelayData,
}

#[derive(Debug)]
pub enum RelayRoomAPI {
    SendToHost(Packet),
    AddRelayPlayer(Arc<SharedConnection>),
    SendToOthers(u32, Packet),
}
#[derive(Debug)]
pub struct RelayRoom {
    pub player_map: HashMap<u32, Weak<SharedConnection>>,
    pub admin: Weak<SharedConnection>,
    pub site: AtomicU32,
    pub shared_relay_room: Arc<SharedRelayRoom>,
}

impl RelayRoom {
    pub fn new(admin: Weak<SharedConnection>, shared_relay_room: Arc<SharedRelayRoom>) -> Self {
        RelayRoom {
            player_map: HashMap::new(),
            admin,
            site: AtomicU32::new(2),
            shared_relay_room,
        }
    }
}

#[derive(Debug)]
pub struct SharedRelayRoom {
    pub shared_data: Arc<SharedRelayRoomData>,
    // TODO: handle the error
    _handle: JoinHandle<()>,
    relay_api_tx: mpsc::Sender<RelayRoomAPI>,
}

impl SharedRelayRoomData {
    pub fn new(id: String, custom: CustomRelayData) -> Self {
        Self {
            id,
            start_game: Arc::new(AtomicBool::new(false)),
            custom,
        }
    }
}

impl SharedRelayRoom {
    pub async fn new_shared(
        runtime: &Arc<Runtime>,
        admin: Weak<SharedConnection>,
        id: String,
        custom: CustomRelayData,
    ) -> Arc<SharedRelayRoom> {
        let shared_data = Arc::new(SharedRelayRoomData::new(id, custom));
        let (relay_api_tx, relay_api_rx) = mpsc::channel(10);
        let (relay_room_tx, relay_room_rx) = std::sync::mpsc::channel();

        let handle = runtime.spawn(async move {
            let relay_room_rx = relay_room_rx;
            let mut relay_api_rx = relay_api_rx;
            let mut relay_room: RelayRoom = relay_room_rx.recv().expect("relay room recv error");

            //let shared_relay_room =
            loop {
                match relay_api_rx.recv().await.expect("Relay API recv error") {
                    RelayRoomAPI::SendToHost(packet) => match relay_room.admin.upgrade() {
                        Some(admin) => admin.send_packet(packet).await,
                        None => todo!(),
                    },
                    RelayRoomAPI::AddRelayPlayer(shared_con) => {
                        let index = relay_room.site.fetch_add(1, Ordering::SeqCst);

                        shared_con.set_room_index(index).await;
                        relay_room
                            .player_map
                            .insert(index, Arc::downgrade(&shared_con));

                        shared_con.add_relay_connect().await;
                    }

                    RelayRoomAPI::SendToOthers(index, packet) => {
                        match relay_room.player_map.get(&index) {
                            Some(weak_shared) => {
                                if let Some(other) = weak_shared.upgrade() {
                                    other.send_packet(packet).await
                                }
                            }
                            None => todo!(),
                        }
                    }
                }
            }
        });

        let shared_relay_room = Arc::new(Self {
            shared_data,
            _handle: handle,
            relay_api_tx,
        });
        let relay_room = RelayRoom::new(admin, shared_relay_room.clone());
        // TODO: handle the error
        let _ = relay_room_tx.send(relay_room);
        shared_relay_room
    }

    pub async fn add_relay_player(&self, shared_con: Arc<SharedConnection>) {
        // TODO: handle the error
        let _ = self
            .relay_api_tx
            .send(RelayRoomAPI::AddRelayPlayer(shared_con))
            .await;
    }

    pub async fn send_packet_to_host(&self, packet: Packet) {
        // TODO: handle the error
        let _ = self
            .relay_api_tx
            .send(RelayRoomAPI::SendToHost(packet))
            .await;
    }

    pub async fn send_packet_to_others(&self, index: u32, packet: Packet) {
        // TODO: handle the error
        let _ = self
            .relay_api_tx
            .send(RelayRoomAPI::SendToOthers(index, packet))
            .await;
    }
}
