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

pub async fn read_string(packet: &mut Packet) -> Option<String> {
    let str_len = packet.packet_buffer.read_u16().await.unwrap();
    let mut str = vec![0; str_len as usize];
    packet.packet_buffer.read_exact(&mut str).await.unwrap();
    Some(String::from_utf8_lossy(&str).to_string())
}

pub async fn read_if_is_string(packet: &mut Packet) -> Option<String> {
    if packet.packet_buffer.read_u8().await.unwrap() == 1 {
        read_string(packet).await
    } else {
        None
    }
}

pub async fn write_string(packet: &mut Packet, str: &str) -> std::io::Result<usize> {
    packet
        .packet_buffer
        .write_u16(str.len() as u16)
        .await
        .unwrap();
    packet.packet_buffer.write(str.as_bytes()).await
}

pub async fn write_is_string(packet: &mut Packet, str: &str) -> std::io::Result<usize> {
    if str.is_empty() {
        packet.packet_buffer.write_u8(0).await.unwrap();
        Ok(0)
    } else {
        packet.packet_buffer.write_u8(1).await.unwrap();
        write_string(packet, str).await
    }
}


pub async fn read_stream_bytes(packet: &mut Packet) -> Vec<u8>{
    let mut packet_bytes = vec![0; packet.packet_buffer.read_u32().await.unwrap() as usize];

    packet.packet_buffer.read_exact(&mut packet_bytes).await.unwrap();

    packet_bytes
}

