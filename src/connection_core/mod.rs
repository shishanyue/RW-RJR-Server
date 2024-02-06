pub mod permission_status;
pub mod player_net_api;

use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
};

use log::{info, warn};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    join,
    net::TcpStream,
    sync::{
        broadcast,
        mpsc::{self},
        oneshot, watch, Mutex, RwLock, Semaphore,
    },
};
use uuid::Uuid;

use crate::{
    connection_core::player_net_api::{read_if_is_string, read_string},
    core::{ConnectionManage, RelayManage, RelayRoomData, ServerCommand, WorkersSender},
    packet_core::{Packet, PacketType},
    worker_pool_core::{receiver_core::ReceiverData, sender_core::SenderData},
};

use self::{
    permission_status::PermissionStatus,
    player_net_api::{
        write_is_string, write_string, CustomRelayData, PlayerAllInfo, RelayDirectInspection,
    },
};

static NEW_RELAY_PROTOCOL_VERSION: u32 = 172;

#[derive(Debug, Default)]
pub struct PlayerInfo {
    pub permission_status: PermissionStatus,
    pub player_name: String,
}

#[derive(Debug, Default)]
pub struct ConnectionInfo {
    pub client_version: u32,
    pub is_beta_version: bool,
}

#[derive(Debug)]
pub struct Connection {
    pub con: Option<Arc<RwLock<Connection>>>,
    pub ip: Option<SocketAddr>,
    command_sender: broadcast::Sender<ServerCommand>,
    command_receiver: broadcast::Receiver<ServerCommand>,
    receiver: Option<mpsc::Sender<ReceiverData>>,
    sender: Option<mpsc::Sender<SenderData>>,
    pub packet_sender: mpsc::Sender<Packet>,
    packet_receiver: Option<mpsc::Receiver<Packet>>,
    pub player_info: Arc<RwLock<PlayerInfo>>,
    pub connection_info: Arc<RwLock<ConnectionInfo>>,
    pub cache_packet: Option<Packet>,
    pub packet: Option<Packet>,
    pub relay_mg: Arc<RwLock<RelayManage>>,
    pub connection_mg: Arc<RwLock<ConnectionManage>>,
    pub room: Option<RelayRoomData>,
    pub site: Option<u32>,
    back_worker_sender: mpsc::Sender<WorkersSender>,
    pub disconnect_del_con_sender: mpsc::Sender<SocketAddr>,
    pub is_disconnected: Semaphore,
}

impl Connection {
    pub async fn new(
        command_sender: broadcast::Sender<ServerCommand>,
        command_receiver: broadcast::Receiver<ServerCommand>,
        receiver: mpsc::Sender<ReceiverData>,
        sender: mpsc::Sender<SenderData>,
        relay_mg: Arc<RwLock<RelayManage>>,
        back_worker_sender: mpsc::Sender<WorkersSender>,
        disconnect_del_con_sender: mpsc::Sender<SocketAddr>,
        connection_mg: Arc<RwLock<ConnectionManage>>,
    ) -> Arc<RwLock<Self>> {
        let (packet_sender, packet_receiver) = mpsc::channel(10);

        let con = Arc::new(RwLock::new(Connection {
            con: None,
            ip: None,
            command_sender,
            command_receiver,
            receiver: Some(receiver),
            sender: Some(sender),
            packet_sender: packet_sender.clone(),
            packet_receiver: Some(packet_receiver),
            player_info: Arc::new(RwLock::new(PlayerInfo::default())),
            connection_info: Arc::new(RwLock::new(ConnectionInfo::default())),
            cache_packet: None,
            packet: None,
            relay_mg,
            connection_mg,
            room: None,
            site: None,
            back_worker_sender,
            disconnect_del_con_sender,
            is_disconnected: Semaphore::new(1),
        }));
        con.write().await.con = Some(con.clone());
        con
    }

    pub async fn bind(&mut self, ip: SocketAddr, socket: TcpStream) -> anyhow::Result<()> {
        self.ip = Some(ip);
        let (read_half, write_half) = socket.into_split();

        match join!(
            self.receiver.as_ref().unwrap().send((
                self.command_receiver.resubscribe(),
                self.con.clone().unwrap(),
                read_half,
                self.packet_sender.clone()
            )),
            self.sender.as_ref().unwrap().send((
                self.command_receiver.resubscribe(),
                write_half,
                self.packet_receiver.take().unwrap()
            ))
        ) {
            (Ok(_), Ok(_)) => Ok(()),
            _ => {
                panic!()
            }
        }
    }

    pub async fn send_relay_server_info(&mut self) {
        let mut packet = Packet::new(PacketType::RELAY_VERSION_INFO).await;
        packet.packet_buffer.write_u8(0).await.unwrap();
        packet.packet_buffer.write_u32(151).await.unwrap();
        packet.packet_buffer.write_u32(1).await.unwrap();
        packet.packet_buffer.write_u8(0).await.unwrap();
        self.packet_sender.send(packet).await.unwrap();
    }

    pub async fn send_relay_hall_message(&mut self, msg: &str) {
        let mut packet = Packet::new(PacketType::RELAY_117).await;
        packet.packet_buffer.write_u8(1).await.unwrap();
        packet.packet_buffer.write_u32(5).await.unwrap();

        write_string(&mut packet, msg).await.unwrap();

        self.packet_sender.send(packet).await.unwrap();
    }

    pub async fn relay_direct_inspection(&mut self) -> Option<RelayDirectInspection> {
        let mut cache_packet = self.cache_packet.clone().unwrap();
        if cache_packet.packet_length != 0 {
            read_string(&mut cache_packet).await.unwrap();

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
                read_if_is_string(&mut cache_packet).await
            } else {
                None
            };

            let player_name = if packet_version >= 3 {
                read_string(&mut cache_packet).await
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

    pub async fn send_relay_server_type_reply(&mut self, _info: PlayerAllInfo) {
        let mut packet = self.packet.take().unwrap();
        //跳过无用的一个byte和一个int32
        packet.packet_buffer.set_position(5);

        let player_command = read_string(&mut packet).await.unwrap();

        if player_command.is_empty() {
            self.send_relay_hall_message("你还什么都没输呢").await
        } else {
            if let Some(id) = player_command.strip_prefix('S') {
                let relay = self.relay_mg.write().await.get_relay(id);
                match relay {
                    Some(room) => {
                        let (site_sender, site_receiver) = oneshot::channel();

                        room.relay_add_con_sender
                            .send((
                                (self.con.clone().unwrap(), self.packet_sender.clone()),
                                site_sender,
                            ))
                            .await
                            .unwrap();
                        self.room = Some(room);
                        self.add_relay_connect(site_receiver).await;
                    }
                    None => {
                        self.send_relay_hall_message(&format!("{}此房间不存在", player_command))
                            .await;
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
                let custom = CustomRelayData::default();
                let new_room = self
                    .relay_mg
                    .write()
                    .await
                    .new_relay_id(
                        self.con.clone().unwrap(),
                        None,
                        mods,
                        uplist,
                        custom,
                        (self.player_info.clone(), self.connection_info.clone()),
                        self.packet_sender.clone(),
                    )
                    .await;

                match new_room {
                    Ok(room_data) => {
                        self.room = Some(room_data);
                        self.send_relay_server_id().await;
                    }
                    Err(e) => {
                        warn!("{}", e)
                    }
                }
            }
        }
    }

    pub async fn send_relay_server_id(&mut self) {
        self.player_info.write().await.permission_status = PermissionStatus::HostPermission;
        self.site = Some(1);

        let mut packet = Packet::new(PacketType::RELAY_BECOME_SERVER).await;

        let room_lock = self.room.as_ref().unwrap().relay_room.read().await;

        let public = false;

        if room_lock.version >= NEW_RELAY_PROTOCOL_VERSION {
            packet.packet_buffer.write_u8(2).await.unwrap();

            packet.packet_buffer.write_u8(1).await.unwrap();
            packet.packet_buffer.write_u8(1).await.unwrap();
            packet.packet_buffer.write_u8(1).await.unwrap();

            write_string(&mut packet, "RJR Team").await.unwrap();

            packet
                .packet_buffer
                .write_u8(room_lock.mods as u8)
                .await
                .unwrap();

            packet.packet_buffer.write_u8(public as u8).await.unwrap();
            packet.packet_buffer.write_u8(1).await.unwrap();

            write_string(
                &mut packet,
                &format!(
                    "{{RW-RJR Relay}}.Room ID : S{}",
                    self.room.as_ref().unwrap().id
                ),
            )
            .await
            .unwrap();

            packet.packet_buffer.write_u8(public as u8).await.unwrap();

            write_is_string(&mut packet, &Uuid::new_v4().to_string())
                .await
                .unwrap();
        } else {
            todo!()
        }

        self.packet_sender.send(packet).await.unwrap();
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
        packet.packet_buffer.write_u8(1).await.unwrap();
        packet.packet_buffer.write_u8(60).await.unwrap();

        self.packet_sender.send(packet).await.unwrap();
    }

    pub async fn add_relay_connect(&mut self, site_receiver: oneshot::Receiver<u32>) {
        self.player_info.write().await.permission_status = PermissionStatus::PlayerPermission;

        let mut packet = Packet::new(PacketType::FORWARD_CLIENT_ADD).await;

        self.site = Some(site_receiver.await.unwrap());

        if self.connection_info.read().await.client_version >= NEW_RELAY_PROTOCOL_VERSION {
            packet.packet_buffer.write_u8(1).await.unwrap();
            packet
                .packet_buffer
                .write_u32(*self.site.as_ref().unwrap())
                .await
                .unwrap();

            write_string(&mut packet, &Uuid::new_v4().to_string())
                .await
                .unwrap();

            packet.packet_buffer.write_u8(0).await.unwrap();

            write_is_string(&mut packet, &self.ip.unwrap().ip().to_string())
                .await
                .unwrap();
        } else {
            todo!()
        }

        self.room
            .as_ref()
            .unwrap()
            .relay_room
            .read()
            .await
            .admin_packet_sender
            .send(packet)
            .await
            .unwrap();

        self.send_package_to_host(self.cache_packet.as_ref().unwrap().clone())
            .await;
        self.chat_message_packet_internal("RJR Server:", "欢迎", 5)
            .await;
    }

    pub async fn send_package_to_host(&mut self, packet: Packet) {
        let mut send_packet = Packet::new(PacketType::PACKET_FORWARD_CLIENT_FROM).await;

        send_packet
            .packet_buffer
            .write_u32(*self.site.as_ref().unwrap())
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

        //todo player玩家在host断开后会panic
        let _ = self
            .room
            .as_ref()
            .unwrap()
            .relay_room
            .read()
            .await
            .admin_packet_sender
            .send(send_packet)
            .await;
    }

    pub async fn chat_message_packet_internal(&mut self, sender: &str, msg: &str, team: u32) {
        let mut packet = Packet::new(PacketType::CHAT).await;
        write_string(&mut packet, msg).await.unwrap();

        packet.packet_buffer.write_u8(3).await.unwrap();

        write_is_string(&mut packet, sender).await.unwrap();

        packet.packet_buffer.write_u32(team).await.unwrap();
        packet.packet_buffer.write_u32(team).await.unwrap();

        self.packet_sender.send(packet).await.unwrap();
    }

    pub async fn receive_chat(&mut self) {
        let packet = self.packet.take().unwrap();

        self.send_package_to_host(packet).await;
    }

    pub async fn disconnect(&mut self) {
        match self.is_disconnected.acquire().await {
            Ok(_) => {
                if !self.is_disconnected.is_closed() {
                    self.is_disconnected.close();
                    //println!("is_disconnected");
                    self.command_sender.send(ServerCommand::Disconnect).unwrap();
                    self.back_worker_sender
                        .send((self.receiver.take().unwrap(), self.sender.take().unwrap()))
                        .await
                        .unwrap();

                    self.connection_mg
                        .write()
                        .await
                        .remove_con(self.ip.unwrap())
                        .await;

                    if self.room.is_some() {
                        self.room
                            .as_ref()
                            .unwrap()
                            .relay_room
                            .write()
                            .await
                            .remove_con(*self.site.as_ref().unwrap())
                            .await;
                    }

                    info!("{}断开连接", self.ip.as_ref().unwrap());
                }
            }
            Err(_) => {}
        };
    }
}
