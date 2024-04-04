use std::{
    net::SocketAddr,
    sync::{Arc, Weak},
};

use log::info;
use tokio::{
    net::TcpStream,
    runtime::Runtime,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use crate::packet::Packet;

use super::{
    player_net_api::RelayDirectInspection, Connection, ConnectionAPI, ConnectionChannel,
    ConnectionInfo, PlayerInfo,
};

#[derive(Debug)]
pub struct SharedConnectionData {
    pub player_info: Arc<PlayerInfo>,
    pub connection_info: Arc<ConnectionInfo>,
}

#[derive(Debug)]
pub struct SharedConnection {
    pub shared_data: Arc<SharedConnectionData>,
    pub shared_channel: Arc<ConnectionChannel>,
    handle: JoinHandle<()>,
}

impl SharedConnectionData {
    pub fn new_with_addr(addr: Weak<SocketAddr>) -> Self {
        Self {
            player_info: Arc::new(PlayerInfo::default()),
            connection_info: Arc::new(ConnectionInfo {
                addr,
                ..Default::default()
            }),
        }
    }
}

impl SharedConnection {
    pub fn new(
        mut con: Connection,
        con_api_rx: mpsc::Receiver<ConnectionAPI>,
        runtime: &Arc<Runtime>,
        shared_data: Arc<SharedConnectionData>,
        shared_channel: Arc<ConnectionChannel>,
    ) -> Arc<Self> {
        let (con_tx, con_rx) = std::sync::mpsc::channel();

        let handle = runtime.spawn(async move {
            let con_rx = con_rx;
            let mut con: Connection = con_rx.recv().expect("con recv error");
            let mut con_api_rx = con_api_rx;
            loop {
                match con_api_rx.recv().await.expect("Connection API recv error") {
                    ConnectionAPI::Disconnect => con.disconnect().await,
                    ConnectionAPI::SetPacket(packet) => con.packet = Some(packet),
                    ConnectionAPI::SetCachePacket(packet) => con.cache_packet = Some(packet),
                    ConnectionAPI::SendRelayHallMessage(msg) => {
                        con.send_relay_hall_message(&msg).await
                    }
                    ConnectionAPI::SendRelayServerInfo => con.send_relay_server_info().await,
                    ConnectionAPI::RelayDirectInspection(inspection_data_tx) => inspection_data_tx
                        .send(con.relay_direct_inspection().await)
                        .expect("send inspection_data error"),

                    ConnectionAPI::SendRelayServerTypeReply => {
                        con.send_relay_server_type_reply().await
                    }
                    ConnectionAPI::SetRoomIndex(index) => con.room_index = index,
                    ConnectionAPI::GetPingData => con.get_ping_data().await,
                    ConnectionAPI::AddRelayConnect => con.add_relay_connect().await,
                    ConnectionAPI::SendPacketToOthers(packet) => {
                        con.send_packet_to_others(packet).await
                    }
                    ConnectionAPI::SendPacketToHost(packet) => {
                        con.send_packet_to_host(packet).await
                    }
                    ConnectionAPI::SendPacketToHostRaw(packet) => {
                        con.shared_relay_room
                            .as_ref()
                            .expect("room is None")
                            .send_packet_to_host(packet)
                            .await
                    }
                }
            }
        });

        let shared_con = Arc::new(Self {
            shared_data,
            shared_channel,
            handle,
        });

        con.shared_con = Some(shared_con.clone());

        con_tx.send(con).expect("con send error");

        shared_con
    }

    pub async fn bind(&self, shared_self: Arc<SharedConnection>, socket: TcpStream) {
        let (read_half, write_half) = socket.into_split();

        self.shared_channel
            .receiver
            .send((
                shared_self.clone(),
                read_half,
                self.shared_channel.command_rx.resubscribe(),
            ))
            .await
            .expect("bind receiver error");

        self.shared_channel
            .sender
            .send((
                shared_self,
                self.shared_channel.packet_rx.clone(),
                write_half,
                self.shared_channel.command_rx.resubscribe(),
            ))
            .await
            .expect("bind sender error");
    }

    pub async fn type_relay(&self, shared_self: Arc<SharedConnection>, packet: Packet) {
        self.shared_channel
            .processor_sorter_tx
            .send((shared_self, packet))
            .await;
    }

    pub async fn set_packet(&self, packet: Packet) {
        self.shared_channel
            .con_api_tx
            .send(ConnectionAPI::SetPacket(packet))
            .await;
    }

    pub async fn set_cache_packet(&self, cache_packet: Packet) {
        self.shared_channel
            .con_api_tx
            .send(ConnectionAPI::SetCachePacket(cache_packet))
            .await;
    }

    pub async fn send_relay_server_info(&self) {
        self.shared_channel
            .con_api_tx
            .send(ConnectionAPI::SendRelayServerInfo)
            .await;
    }

    pub async fn get_ping_data(&self) {
        self.shared_channel
            .con_api_tx
            .send(ConnectionAPI::GetPingData)
            .await;
    }

    pub async fn send_packet(&self, packet: Packet) {
        self.shared_channel
            .packet_tx
            .send(packet)
            .await
            .expect("send packet to sender error");
    }

    pub async fn send_relay_hall_message(&self, msg: &str) {
        self.shared_channel
            .con_api_tx
            .send(ConnectionAPI::SendRelayHallMessage(msg.to_string()))
            .await;
    }

    pub async fn set_room_index(&self, index: u32) {
        self.shared_channel
            .con_api_tx
            .send(ConnectionAPI::SetRoomIndex(Some(index)))
            .await;
    }

    pub async fn add_relay_connect(&self) {
        self.shared_channel
            .con_api_tx
            .send(ConnectionAPI::AddRelayConnect)
            .await;
    }

    pub async fn disconnect(&self) {
        self.shared_channel
            .con_api_tx
            .send(ConnectionAPI::Disconnect)
            .await;
    }

    pub async fn relay_direct_inspection(&self) -> Option<RelayDirectInspection> {
        let (inspection_data_tx, inspection_data_rx) = oneshot::channel();
        self.shared_channel
            .con_api_tx
            .send(ConnectionAPI::RelayDirectInspection(inspection_data_tx))
            .await;

        inspection_data_rx
            .await
            .expect("recv inspection data error")
    }

    pub async fn send_relay_server_type_reply(&self) {
        self.shared_channel
            .con_api_tx
            .send(ConnectionAPI::SendRelayServerTypeReply)
            .await;
    }

    pub async fn send_packet_to_host(&self, packet: Packet) {
        self.shared_channel
            .con_api_tx
            .send(ConnectionAPI::SendPacketToHost(packet))
            .await;
    }

    pub async fn send_packet_to_host_raw(&self, packet: Packet) {
        self.shared_channel
            .con_api_tx
            .send(ConnectionAPI::SendPacketToHostRaw(packet))
            .await;
    }

    pub async fn send_packet_to_others(&self, packet: Packet) {
        self.shared_channel
            .con_api_tx
            .send(ConnectionAPI::SendPacketToOthers(packet))
            .await;
    }
}

impl Drop for SharedConnection {
    fn drop(&mut self) {
        if !self.shared_channel.receiver.is_closed() && !self.shared_channel.sender.is_closed() {
            info!(
                "{}断开连接",
                self.shared_data
                    .connection_info
                    .addr
                    .upgrade()
                    .expect("drop shared_con get addr error")
            );
            self.handle.abort();
        }
    }
}
