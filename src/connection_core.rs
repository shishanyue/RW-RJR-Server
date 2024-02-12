pub mod permission_status;
pub mod player_net_api;

use std::{net::SocketAddr, sync::Arc};

use log::{info, warn};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    join,
    net::TcpStream,
    sync::{broadcast, mpsc, oneshot, RwLock, Semaphore},
};
use uuid::Uuid;

use crate::{
    connection_core::player_net_api::{read_if_is_string, read_string},
    core::{ RelayManage, RelayRoomData, ServerCommand, WorkersSender},
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

pub struct ConnectionChannel {
}

pub enum ConnectionAPI {}

#[derive(Debug)]
pub struct Connection {}

impl Connection {
 
}
