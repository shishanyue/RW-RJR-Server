pub mod permission_status;
pub mod player_net_api;

use std::{net::SocketAddr, sync::Arc};

use log::{info, warn};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    join,
    net::TcpStream,
    runtime::Runtime,
    sync::{broadcast, mpsc, oneshot, RwLock, Semaphore},
    task::JoinHandle,
};
use uuid::Uuid;

use crate::{
    connection_core::player_net_api::{read_if_is_string, read_string},
    core::{RelayManage, RelayRoomData, ServerCommand, WorkersSender},
    packet_core::{Packet, PacketType},
    worker_pool_core::{
        processor_core::ProcesseorData, receiver_core::ReceiverData, sender_core::SenderData,
    },
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
pub struct ConnectionChannel {
    pub receiver: mpsc::Sender<ReceiverData>,
    pub sender: mpsc::Sender<SenderData>,
    pub processor_sorter_sender: mpsc::Sender<ProcesseorData>,
    pub connection_api_sender: mpsc::Sender<ConnectionAPI>,
}

#[derive(Debug)]
pub enum ConnectionAPI {
    Disconnect,
}

#[derive(Debug)]
pub struct Connection {
    pub connection_channel: Arc<ConnectionChannel>,
    pub addr: SocketAddr,
}

impl ConnectionChannel {
    pub fn new(
        receiver: mpsc::Sender<ReceiverData>,
        sender: mpsc::Sender<SenderData>,
        processor_sorter_sender: mpsc::Sender<ProcesseorData>,
        connection_api_sender: mpsc::Sender<ConnectionAPI>,
    ) -> Self {
        ConnectionChannel {
            receiver,
            sender,
            processor_sorter_sender,
            connection_api_sender,
        }
    }
}



pub type SharedConnection = (Arc<ConnectionChannel>, JoinHandle<()>);

impl Connection {
    pub fn new(
        runtime: &mut Runtime,
        new_receiver: mpsc::Sender<ReceiverData>,
        new_sender: mpsc::Sender<SenderData>,
        processor_sorter_sender: mpsc::Sender<ProcesseorData>,
        addr: SocketAddr,
    ) -> SharedConnection {
        let (connection_api_sender, connection_api_receiver) = mpsc::channel(10);

        let connection_channel = Arc::new(ConnectionChannel::new(
            new_receiver,
            new_sender,
            processor_sorter_sender,
            connection_api_sender,
        ));

        let con = Connection {
            connection_channel: connection_channel.clone(),
            addr: addr,
        };

        (
            connection_channel,
            runtime.spawn(async move {
                let con = con;
                let mut connection_api_receiver = connection_api_receiver;
                loop {
                    match connection_api_receiver.recv().await.expect("Connection接收错误") {
                        ConnectionAPI::Disconnect => {

                        },
                    }
                }
            }),
        )
    }
}
