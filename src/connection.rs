pub mod permission_status;
pub mod player_net_api;
pub mod shared_connection;

use std::{
    net::SocketAddr,
    sync::{
        self,
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, RwLock,
    },
};

use log::warn;
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    runtime::Runtime,
    sync::{broadcast, mpsc, oneshot, Semaphore},
};
use uuid::Uuid;

use crate::{
    connection_manager::By,
    core::ServerCommand,
    event::{Event, EventType, EVENT_CHANNEL},
    packet::{Packet, PacketReadWriteExt, PacketType},
    relay_manager::{relay::SharedRelayRoom, SharedRelayManager},
    worker_pool::{processor::ProcesseorData, receiver::ReceiverData, sender::SenderData},
};

use self::{
    permission_status::PermissionStatus,
    player_net_api::{CustomRelayData, RelayDirectInspection},
    shared_connection::{SharedConnection, SharedConnectionData},
};

static NEW_RELAY_PROTOCOL_VERSION: u32 = 172;

#[derive(Debug, Default)]
pub struct PlayerInfo {
    pub permission_status: Arc<RwLock<PermissionStatus>>,
    pub player_name: Arc<RwLock<String>>,
}

#[derive(Debug, Default)]
pub struct ConnectionInfo {
    pub addr: sync::Weak<SocketAddr>,
    pub client_version: Arc<AtomicU32>,
    pub is_beta_version: Arc<AtomicBool>,
}

#[derive(Debug)]
pub struct ConnectionChannel {
    pub processor_sorter_tx: mpsc::Sender<ProcesseorData>,
    pub con_api_tx: mpsc::Sender<ConnectionAPI>,
    pub packet_tx: async_channel::Sender<Packet>,
    pub packet_rx: async_channel::Receiver<Packet>,

    pub command_tx: broadcast::Sender<ServerCommand>,
    pub command_rx: broadcast::Receiver<ServerCommand>,
}

#[derive(Debug)]
pub enum ConnectionAPI {
    Disconnect,
    SetPacket(Packet),
    SetCachePacket(Packet),
    SetRoomIndex(Option<u32>),
    SendRelayServerInfo,
    RelayDirectInspection(oneshot::Sender<Option<RelayDirectInspection>>),
    SendRelayHallMessage(String),
    SendRelayServerTypeReply,
    GetPingData,
    SendPacketToHost(Packet),
    SendPacketToHostRaw(Packet),
    SendPacketToOthers(Packet),
    AddRelayConnect,
}

#[derive(Debug)]
pub struct Connection {
    pub shared_con: Option<Arc<SharedConnection>>,
    pub addr: Arc<SocketAddr>,
    pub packet: Option<Packet>,
    pub cache_packet: Option<Packet>,
    shared_relay_mg: Arc<SharedRelayManager>,
    pub room_index: Option<u32>,
    pub shared_relay_room: Option<Arc<SharedRelayRoom>>,
    pub is_disconnected: Semaphore,
    con_lib_api_tx: mpsc::Sender<ConnectionLibAPI>,
}

#[derive(Debug)]
pub enum ConnectionLibAPI {
    InsertConnection(Arc<SharedConnection>),
    RemoveConnectionBy(By),
    SendPacketToPlayerBy(By, Packet),
}

impl ConnectionChannel {
    pub fn new(
        processor_sorter_tx: mpsc::Sender<ProcesseorData>,
        con_api_tx: mpsc::Sender<ConnectionAPI>,
    ) -> Self {
        let (packet_tx, packet_rx) = async_channel::bounded(10);
        let (command_tx, command_rx) = broadcast::channel(5);

        ConnectionChannel {
            processor_sorter_tx,
            con_api_tx,
            packet_rx,
            packet_tx,
            command_rx,
            command_tx,
        }
    }
}
#[allow(clippy::too_many_arguments)]
impl Connection {
    pub fn new_shared(
        runtime: &Arc<Runtime>,
        processor_sorter_sender: mpsc::Sender<ProcesseorData>,
        addr: SocketAddr,
        shared_relay_mg: Arc<SharedRelayManager>,
        con_lib_api_tx: mpsc::Sender<ConnectionLibAPI>,
    ) -> Arc<SharedConnection> {
        let (con_api_tx, con_api_rx) = mpsc::channel(10);

        let con = Connection {
            addr: Arc::new(addr),
            packet: None,
            cache_packet: None,
            shared_con: None,
            shared_relay_mg,
            room_index: None,
            shared_relay_room: None,
            is_disconnected: Semaphore::new(1),
            con_lib_api_tx,
        };

        let shared_channel = Arc::new(ConnectionChannel::new(processor_sorter_sender, con_api_tx));

        let shared_data = Arc::new(SharedConnectionData::new_with_addr(Arc::downgrade(
            &con.addr,
        )));

        SharedConnection::new(con, con_api_rx, runtime, shared_data, shared_channel)
    }

    pub async fn send_relay_server_info(&self) {
        let mut packet = Packet::new(PacketType::RELAY_VERSION_INFO).await;
        packet.write_u8(0).await.unwrap();
        packet.write_u32(151).await.unwrap();
        packet.write_u32(1).await.unwrap();
        packet.write_u8(0).await.unwrap();
        self.shared_con.as_ref().unwrap().send_packet(packet).await;
    }

    pub async fn send_packet_to_others(&self, mut packet: Packet) {
        let index = packet.packet_buffer.read_u32().await.unwrap();
        let packet_type = packet.packet_buffer.read_u32().await.unwrap();

        let bytes = packet.read_stream_bytes().await;

        if packet_type == PacketType::DISCONNECT as u32 {
            return;
        }

        let mut send_packet =
            Packet::new(PacketType::try_from(packet_type).unwrap_or_default()).await;

        send_packet.write_all(&bytes).await.unwrap();

        if packet_type == PacketType::KICK as u32 {
            // TODO
        }
        if packet_type == PacketType::START_GAME as u32 {
            // TODO
        }

        self.shared_relay_room
            .as_ref()
            .expect("room is None")
            .send_packet_to_others(index, send_packet)
            .await;
    }

    pub async fn send_packet_to_host(&self, packet: Packet) {
        let mut send_packet = Packet::new(PacketType::PACKET_FORWARD_CLIENT_FROM).await;

        send_packet
            .packet_buffer
            .write_u32(*self.room_index.as_ref().unwrap())
            .await
            .unwrap();
        send_packet
            .packet_buffer
            .write_u32(packet.packet_length + 8)
            .await
            .unwrap();
        send_packet
            .packet_buffer
            .write_u32(packet.packet_length)
            .await
            .unwrap();

        send_packet
            .packet_buffer
            .write_u32(packet.packet_type as u32)
            .await
            .unwrap();
        send_packet
            .packet_buffer
            .write_all(&packet.packet_buffer.into_inner())
            .await
            .unwrap();

        self.shared_relay_room
            .as_ref()
            .expect("room is None")
            .send_packet_to_host(send_packet)
            .await;
    }

    pub async fn get_ping_data(&mut self) {
        let mut packet = Packet::new(PacketType::HEART_BEAT_RESPONSE).await;
        packet
            .packet_buffer
            .write_u64(
                self.packet
                    .take()
                    .unwrap()
                    .packet_buffer
                    .read_u64()
                    .await
                    .unwrap(),
            )
            .await
            .unwrap();
        packet.write_u8(1).await.unwrap();
        packet.write_u8(60).await.unwrap();

        self.shared_con.as_ref().unwrap().send_packet(packet).await;
    }

    pub async fn relay_direct_inspection(&self) -> Option<RelayDirectInspection> {
        let mut cache_packet = self.cache_packet.clone().unwrap();
        if cache_packet.packet_length != 0 {
            cache_packet.read_string().await.unwrap();

            let get_beta_version = |version: u32| (152..=175).contains(&version);

            let packet_version = cache_packet.packet_buffer.read_u32().await.unwrap();
            let client_version = cache_packet.packet_buffer.read_u32().await.unwrap();

            if packet_version >= 1 {
                cache_packet
                    .packet_buffer
                    .seek(std::io::SeekFrom::Current(4))
                    .await
                    .unwrap();
            }

            let query_string = if packet_version >= 2 {
                cache_packet.read_if_is_string().await
            } else {
                None
            };

            let player_name = if packet_version >= 3 {
                cache_packet.read_string().await
            } else {
                None
            };

            cache_packet.packet_buffer.read_u32().await.unwrap();

            Some(RelayDirectInspection {
                client_version,
                is_beta_version: get_beta_version(client_version),
                query_string,
                player_name,
            })
        } else {
            None
        }
    }

    pub async fn send_relay_hall_message(&self, msg: &str) {
        let mut packet = Packet::new(PacketType::RELAY_117).await;
        packet.write_u8(1).await.unwrap();
        packet.write_u32(5).await.unwrap();

        packet.write_string(msg).await.unwrap();

        self.shared_con.as_ref().unwrap().send_packet(packet).await;
    }

    pub async fn chat_message_packet_internal(&mut self, sender: &str, msg: &str, team: u32) {
        let mut packet = Packet::new(PacketType::CHAT).await;

        packet.write_string(msg).await.unwrap();

        packet.write_u8(3).await.unwrap();

        packet.write_is_string(sender).await.unwrap();

        packet.write_u32(team).await.unwrap();
        packet.write_u32(team).await.unwrap();

        self.shared_con.as_ref().unwrap().send_packet(packet).await;
    }

    pub async fn add_relay_connect(&mut self) {
        let shared_data = self.shared_con.as_ref().unwrap().shared_data.as_ref();

        *shared_data
            .player_info
            .permission_status
            .write()
            .expect("write permission status error") = PermissionStatus::PlayerPermission;

        let mut packet = Packet::new(PacketType::FORWARD_CLIENT_ADD).await;

        if shared_data
            .connection_info
            .client_version
            .load(Ordering::Relaxed)
            >= NEW_RELAY_PROTOCOL_VERSION
        {
            packet.write_u8(1).await.unwrap();
            packet
                .packet_buffer
                .write_u32(*self.room_index.as_ref().unwrap())
                .await
                .unwrap();

            packet
                .write_string(&Uuid::new_v4().to_string())
                .await
                .expect("write packet error");

            packet.write_u8(0).await.unwrap();

            packet
                .write_is_string(&self.addr.ip().to_string())
                .await
                .expect("write packet error");
        } else {
            todo!()
        }

        self.shared_con
            .as_ref()
            .unwrap()
            .send_packet_to_host_raw(packet)
            .await;
        self.shared_con
            .as_ref()
            .unwrap()
            .send_packet_to_host(self.cache_packet.clone().expect("cache packet error"))
            .await;

        self.chat_message_packet_internal("RJR Server:", "欢迎", 5)
            .await;
    }

    pub async fn send_relay_server_type_reply(&mut self) {
        let shared_data = self.shared_con.as_ref().unwrap().shared_data.as_ref();

        let mut packet = self.packet.take().unwrap();
        //跳过无用的一个byte和一个int32
        packet.packet_buffer.set_position(5);

        let player_command = packet.read_string().await.unwrap();

        if player_command.is_empty() {
            self.send_relay_hall_message("你还什么都没输呢").await
        } else {
            if let Some(id) = player_command.strip_prefix('S') {
                match self.shared_relay_mg.get_relay(id).await {
                    Some(shared_relay) => {
                        shared_relay
                            .add_relay_player(self.shared_con.clone().unwrap())
                            .await;

                        self.shared_relay_room = Some(shared_relay);
                    }
                    None => {
                        self.send_relay_hall_message(&format!("{}此房间不存在", player_command))
                            .await
                    }
                }
                return;
            }

            let uplist = false;
            let mut mods = false;
            let mut new_room = false;

            if player_command.starts_with("new") || player_command.starts_with("news") {
                new_room = true;
            } else if player_command.starts_with("mod") || player_command.starts_with("mods") {
                new_room = true;
                mods = true;
            } else {
                self.send_relay_hall_message("不懂").await
            }

            if new_room {
                let custom = CustomRelayData::new(
                    mods,
                    uplist,
                    shared_data
                        .connection_info
                        .is_beta_version
                        .load(Ordering::Relaxed),
                    shared_data
                        .connection_info
                        .client_version
                        .load(Ordering::Relaxed),
                );
                let new_shared_room = self
                    .shared_relay_mg
                    .new_relay_id(self.shared_con.clone().unwrap(), None, custom)
                    .await;

                self.shared_relay_room = Some(new_shared_room);

                self.send_relay_server_id().await;
            }
        }
    }

    pub async fn send_relay_server_id(&mut self) {
        let shared_data = self.shared_con.as_ref().unwrap().shared_data.as_ref();
        let shared_relay_room = self.shared_relay_room.as_ref().unwrap();

        self.room_index = Some(0);

        *shared_data
            .player_info
            .permission_status
            .write()
            .expect("write permission status error") = PermissionStatus::HostPermission;

        let mut packet = Packet::new(PacketType::RELAY_BECOME_SERVER).await;

        let public = false;

        if shared_relay_room.shared_data.custom.version >= NEW_RELAY_PROTOCOL_VERSION {
            packet.write_u8(2).await.unwrap();

            packet.write_u8(1).await.unwrap();
            packet.write_u8(1).await.unwrap();
            packet.write_u8(1).await.unwrap();

            packet
                .write_string("RJR Team")
                .await
                .expect("write packet error");

            packet
                .packet_buffer
                .write_u8(shared_relay_room.shared_data.custom.mods as u8)
                .await
                .unwrap();

            packet.write_u8(public as u8).await.unwrap();
            packet.write_u8(1).await.unwrap();

            packet
                .write_string(&format!(
                    "{{RW-RJR Relay}}.Room ID : {}",
                    shared_relay_room.shared_data.id
                ))
                .await
                .expect("write packet error");

            packet.write_u8(public as u8).await.unwrap();

            packet
                .write_is_string(&Uuid::new_v4().to_string())
                .await
                .expect("write packet error");
        } else {
            todo!()
        }

        self.shared_con.as_ref().unwrap().send_packet(packet).await;

        EVENT_CHANNEL
            .0
            .send(Event::new(
                "abab",
                EventType::NewRoomAndHostOk(self.shared_relay_room.as_ref().unwrap().clone()),
            ))
            .await.expect("send event error");
    }

    pub async fn disconnect(&mut self) {
        if (self.is_disconnected.acquire().await).is_ok() {
            self.is_disconnected.close();

            let shared_channel = self.shared_con.as_ref().unwrap().shared_channel.as_ref();

            shared_channel
                .command_tx
                .send(ServerCommand::Disconnect)
                .unwrap();

            self.con_lib_api_tx
                .send(ConnectionLibAPI::RemoveConnectionBy(By::Addr(
                    self.addr.to_string(),
                )))
                .await
                .expect("remove con error");

            self.shared_con
                .take()
                .expect("remove shared_con error when disconnect");
        }
    }
}
