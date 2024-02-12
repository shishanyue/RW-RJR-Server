use std::{
    collections::HashSet, net::SocketAddr, sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    }
};

use dashmap::{DashMap, DashSet};
use log::info;
use rand::{Rng, SeedableRng};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    join,
    runtime::{Builder, Runtime},
    sync::{
        broadcast,
        mpsc::{self},
        oneshot, Mutex, RwLock,
    },
    task::JoinHandle,
};

use crate::{
    connection_core::{
        player_net_api::{read_stream_bytes, CustomRelayData, PlayerAllInfo},
        Connection,
    },
    packet_core::{Packet, PacketType},
    worker_pool_core::{
        new_worker_pool,
        receiver_core::{receiver, ReceiverData},
        sender_core::{sender, SenderData},
    },
};



#[derive(Debug, Clone, Copy)]
pub enum ServerCommand {
    Disconnect,
    #[allow(dead_code)]
    None,
}

pub type WorkersSender = (mpsc::Sender<ReceiverData>, mpsc::Sender<SenderData>);



pub async fn creat_block_runtime(threads: usize) -> anyhow::Result<Runtime> {
    Ok(Builder::new_multi_thread()
        .enable_time()
        .worker_threads(threads)
        // no timer!
        .build()
        .unwrap())
}

pub type RelayConData = (
    (Arc<RwLock<Connection>>, mpsc::Sender<Packet>),
    oneshot::Sender<u32>,
);

#[derive(Debug)]
pub struct RelayRoom {
    pub player_map: DashMap<u32, (Arc<RwLock<Connection>>, mpsc::Sender<Packet>)>,
    pub admin: Arc<RwLock<Connection>>,
    pub admin_packet_sender: mpsc::Sender<Packet>,
    pub site: AtomicU32,
    pub id: String,
    pub mods: bool,
    pub uplist: bool,
    pub beta_game_version: bool,
    pub version: u32,
    pub max_player: i32,
    pub handle: Option<JoinHandle<()>>,
    relay_mg: Arc<RwLock<RelayManage>>,
    //pub relay_broadcast_sender: broadcast::Sender<Packet>
}

#[derive(Debug, Clone)]
pub struct RelayRoomData {
    pub relay_room: Arc<RwLock<RelayRoom>>,
    pub id: String,
    pub relay_add_con_sender: mpsc::Sender<RelayConData>,
    pub relay_group_packet_sender: mpsc::Sender<Packet>,
}

#[allow(clippy::too_many_arguments)]
impl RelayRoom {
    pub async fn new(
        admin: Arc<RwLock<Connection>>,
        id: String,
        mods: bool,
        uplist: bool,
        beta_game_version: bool,
        version: u32,
        max_player: i32,
        admin_packet_sender: mpsc::Sender<Packet>,
        relay_mg: Arc<RwLock<RelayManage>>,
    ) -> Arc<RwLock<Self>> {
        let player_map = DashMap::new();
        let site = AtomicU32::new(2);
        player_map.insert(1, (admin.clone(), admin_packet_sender.clone()));
        Arc::new(RwLock::new(RelayRoom {
            player_map,
            admin,
            admin_packet_sender,
            site,
            id,
            mods,
            uplist,
            beta_game_version,
            version,
            max_player,
            handle: None,
            relay_mg,
        }))
    }
    pub async fn add_connect(&mut self, con_data: RelayConData) {
        let pos = self.site.fetch_add(1, Ordering::Acquire);

        self.player_map.insert(pos, con_data.0);
        con_data.1.send(pos).unwrap();
    }

    pub async fn remove_con(&mut self, site: u32) {
        if self.player_map.contains_key(&site) {
            self.player_map.remove(&site);
            if self.player_map.is_empty() {
                self.handle.take().unwrap().abort();
                self.relay_mg.write().await.remove_room(&self.id);
            }
        }
    }

    pub fn get_con_sender(&mut self, index: u32) -> Option<mpsc::Sender<Packet>> {
        if self.player_map.contains_key(&index) {
            let con = self.player_map.get(&index).unwrap();
            Some(con.1.clone())
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct RelayManage {
    pub room_map: DashMap<String, RelayRoomData>,
    pub relay_rt: Runtime,
    pub id_rand: rand::rngs::StdRng,
    pub arc_self: Option<Arc<RwLock<Self>>>,
}

impl RelayManage {
    pub async fn new() -> Arc<RwLock<Self>> {
        let relay_mg = Arc::new(RwLock::new(RelayManage {
            room_map: DashMap::new(),
            relay_rt: Builder::new_multi_thread()
                .enable_time()
                .worker_threads(1)
                // no timer!
                .build()
                .unwrap(),
            id_rand: SeedableRng::from_entropy(),
            arc_self: None,
        }));
        relay_mg.write().await.arc_self = Some(relay_mg.clone());
        relay_mg
    }

    pub fn get_relay(&mut self, id: &str) -> Option<RelayRoomData> {
        if self.room_map.contains_key(&id.to_string()) {
            let room = self.room_map.get(&id.to_string()).unwrap();
            Some(room.clone())
        } else {
            None
        }
        //*self.room_map.try_get(&id.to_string()).unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn new_relay_id(
        &mut self,
        con: Arc<RwLock<Connection>>,
        _id: Option<String>,
        mods: bool,
        uplist: bool,
        custom: CustomRelayData,
        info: PlayerAllInfo,
        admin_packet_sender: mpsc::Sender<Packet>,
    ) -> anyhow::Result<RelayRoomData> {
        let max_player = if custom.max_player_size == -1 {
            10
        } else {
            custom.max_player_size
        };

        let new_relay = self
            .new_relay(
                con,
                None,
                &info.0.read().await.player_name,
                mods,
                uplist,
                info.1.read().await.is_beta_version,
                info.1.read().await.client_version,
                max_player,
                admin_packet_sender,
            )
            .await;

        self.room_map
            .insert(new_relay.id.clone(), new_relay.clone());
        //self.room_map.insert(key, value)
        Ok(new_relay)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn new_relay(
        &mut self,
        con: Arc<RwLock<Connection>>,
        id: Option<String>,
        _player_name: &str,
        is_mod: bool,
        uplist: bool,
        beta_game_version: bool,
        version: u32,
        max_player: i32,
        admin_packet_sender: mpsc::Sender<Packet>,
    ) -> RelayRoomData {
        let (relay_add_con_sender, mut relay_add_con_receiver) = mpsc::channel(10);
        let (relay_group_packet_sender, mut relay_group_packet_receiver) = mpsc::channel(10);

        let id = if let Some(id) = id {
            id
        } else {
            loop {
                let tmp_id = self.id_rand.gen_range(100..9999).to_string();
                if !self.room_map.contains_key(&tmp_id) {
                    break tmp_id;
                }
            }
        };

        let relay_room = RelayRoom::new(
            con,
            id.clone(),
            is_mod,
            uplist,
            beta_game_version,
            version,
            max_player,
            admin_packet_sender,
            self.arc_self.as_ref().unwrap().clone(),
        )
        .await;

        let room = relay_room.clone();

        let handle = self.relay_rt.spawn(async move {
            let room = room.clone();
            loop {
                tokio::select! {
                    add_con_data = relay_add_con_receiver.recv() => {
                        if let Some(add_con) = add_con_data {

                            room.write().await.add_connect(add_con).await;
                            //add_con.1.send(pos);
                        }

                        },
                    packet = relay_group_packet_receiver.recv() => {
                        if let Some(packet) = packet {
                            relay_packet_sender(room.clone(),packet).await;
                        }
                    },
                }
            }
        });

        relay_room.write().await.handle = Some(handle);

        RelayRoomData {
            relay_room,
            id,
            relay_add_con_sender,
            relay_group_packet_sender,
        }
    }

    pub fn remove_room(&mut self, id: &String) {
        if self.room_map.contains_key(id) {
            self.room_map.remove(id);
        }
    }
}

pub async fn relay_packet_sender(room: Arc<RwLock<RelayRoom>>, mut packet: Packet) {
    let target = packet.packet_buffer.read_u32().await.unwrap();
    let packet_type = packet.packet_buffer.read_u32().await.unwrap();

    let bytes = read_stream_bytes(&mut packet).await;

    if packet_type == PacketType::DISCONNECT as u32 {
        return;
    }
    let target_con_sender = room.write().await.get_con_sender(target).unwrap();

    let mut send_packet = Packet::new(PacketType::try_from(packet_type).unwrap_or_default()).await;

    send_packet.packet_buffer.write_all(&bytes).await.unwrap();

    if packet_type == PacketType::KICK as u32 {
        // TODO
    }
    if packet_type == PacketType::START_GAME as u32 {
        // TODO
    }

    target_con_sender.send(send_packet).await.unwrap();
}
