pub mod super_packet;

use std::io::Cursor;

use num_enum::TryFromPrimitive;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Default, Clone, Copy)]
#[repr(u32)]
pub enum PacketType {
    /**
     * CUSTOM PACKET
     */
    /* DEBUG */
    SERVER_DEBUG_RECEIVE = 2000,
    SERVER_DEBUG = 2001,

    /* Ex */
    GET_SERVER_INFO_RECEIVE = 3000,
    GET_SERVER_INFO = 3001,
    UPDATA_CLASS_RECEIVE = 3010,
    STATUS_RESULT = 3999,

    /**
     * Game Core Packet
     */
    /* Preregister */
    PREREGISTER_INFO_RECEIVE = 160,
    PREREGISTER_INFO = 161,
    PASSWD_ERROR = 113,
    REGISTER_PLAYER = 110,

    /* Server Info */
    SERVER_INFO = 106,
    TEAM_LIST = 115,

    /* Heart */
    HEART_BEAT = 108,
    HEART_BEAT_RESPONSE = 109,

    /* Chat */
    CHAT_RECEIVE = 140,
    CHAT = 141,

    /* Net Status */
    PACKET_DOWNLOAD_PENDING = 4,
    KICK = 150,
    DISCONNECT = 111,

    /* StartGame */
    START_GAME = 120,
    ACCEPT_START_GAME = 112,
    RETURN_TO_BATTLEROOM = 122,

    /* GameStart Commands */
    TICK = 10,
    GAMECOMMAND_RECEIVE = 20,
    SYNCCHECKSUM_STATUS = 31,
    SYNC_CHECK = 30,
    SYNC = 35,

    /* Relay */
    RELAY_117 = 117,
    RELAY_118_117_RETURN = 118,
    RELAY_POW = 151,
    RELAY_POW_RECEIVE = 152,

    RELAY_VERSION_INFO = 163,
    RELAY_BECOME_SERVER = 170,
    FORWARD_CLIENT_ADD = 172,
    FORWARD_CLIENT_REMOVE = 173,
    PACKET_FORWARD_CLIENT_FROM = 174,
    PACKET_FORWARD_CLIENT_TO = 175,
    PACKET_FORWARD_CLIENT_TO_REPEATED = 176,
    PACKET_RECONNECT_TO = 178,

    EMPTYP_ACKAGE = 0,

    #[default]
    NOT_RESOLVED = u32::MAX,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub packet_type: PacketType,
    pub packet_length: u32,
    pub packet_buffer: Cursor<Vec<u8>>,
    pub is_prepared: bool,
    
}



impl Packet {
    pub async fn new(packet_type: PacketType) -> Self {
        let mut buffer = Cursor::new(Vec::new());
        buffer.write_u64(0).await.unwrap();
        Packet {
            packet_type,
            packet_length: 0,
            packet_buffer: buffer,
            is_prepared: false,
        }
    }

    pub fn decode_from_buffer(
        packet_length: u32,
        packet_type: PacketType,
        packet_buffer: Cursor<Vec<u8>>,
    ) -> Self {
        Packet {
            packet_type,
            packet_length,
            packet_buffer,
            is_prepared: true,
        }
    }
    pub async fn prepare(&mut self) {
        if !self.is_prepared {
            let packet_type = self.packet_type as u32;

            self.packet_length = self.packet_buffer.position() as u32 - 8;
            self.packet_buffer.set_position(0);

            self.packet_buffer
                .write_u32(self.packet_length)
                .await
                .unwrap();
            self.packet_buffer.write_u32(packet_type).await.unwrap();
            self.is_prepared = true;
        }
    }

}

pub trait PacketReadWriteExt {
    async fn read_string(&mut self) -> Option<String>;
    async fn read_if_is_string(&mut self) -> Option<String>;
    async fn write_string(&mut self, s: &str) -> std::io::Result<usize>;
    async fn write_is_string(&mut self, s: &str) -> std::io::Result<usize>;
    async fn read_stream_bytes(&mut self) -> Vec<u8>;
}

impl PacketReadWriteExt for Packet {
    async fn read_string(&mut self) -> Option<String> {
        let str_len = self.packet_buffer.read_u16().await.unwrap();
        let mut str = vec![0; str_len as usize];
        self.packet_buffer.read_exact(&mut str).await.unwrap();
        Some(String::from_utf8_lossy(&str).to_string())
    }
    async fn read_if_is_string(&mut self) -> Option<String> {
        if self.packet_buffer.read_u8().await.unwrap() == 1 {
            
            self.read_string().await
        } else {
            None
        }
    }
    async fn write_string(&mut self, s: &str) -> std::io::Result<usize> {
        self.packet_buffer.write_u16(s.len() as u16).await.unwrap();
        self.packet_buffer.write(s.as_bytes()).await
    }
    async fn write_is_string(&mut self, s: &str) -> std::io::Result<usize> {
        if s.is_empty() {
            self.packet_buffer.write_u8(0).await.unwrap();
            Ok(0)
        } else {
            self.packet_buffer.write_u8(1).await.unwrap();
            self.write_string(s).await
        }
    }
    async fn read_stream_bytes(&mut self) -> Vec<u8> {
        let mut packet_bytes = vec![0; self.packet_buffer.read_u32().await.unwrap() as usize];

        self.packet_buffer
            .read_exact(&mut packet_bytes)
            .await
            .unwrap();

        packet_bytes
    }
}
