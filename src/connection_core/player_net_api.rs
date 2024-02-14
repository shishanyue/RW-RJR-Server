use std::sync::Arc;

use crate::packet_core::Packet;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::RwLock,
};

use super::{ConnectionInfo, PlayerInfo};

pub type PlayerAllInfo = (Arc<RwLock<PlayerInfo>>, Arc<RwLock<ConnectionInfo>>);

pub struct RelayDirectInspection {
    pub client_version: u32,
    pub is_beta_version: bool,
    pub query_string: Option<String>,
    pub player_name: Option<String>,
}

pub struct CustomRelayData {
    pub max_player_size: i32,
    pub max_unit_size: u32,
    pub income: f32,
}

impl Default for CustomRelayData {
    fn default() -> Self {
        Self {
            max_player_size: -1,
            max_unit_size: 300,
            income: 1.0,
        }
    }
}

//(u32,bool,Option<String>,Option<String>);
