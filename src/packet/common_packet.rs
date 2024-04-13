use crate::dummy::BasicDummy;

use super::{Packet, PacketReadWriteExt, PacketType};

pub struct CommonPacket;

impl CommonPacket {
    pub async fn get_preregister_info(
        dummy_name: &str,
        domain: &str,
        packet_version: u32,
        client_version: u32,
        query_string: &str,
    ) -> std::io::Result<Packet> {
        let mut packet = Packet::new(PacketType::PREREGISTER_INFO_RECEIVE).await;
        packet.write_string(&domain).await.unwrap();

        packet.write_u32(packet_version).await.unwrap();
        packet.write_u32(client_version).await.unwrap();

        if packet_version >= 1 {
            packet.write_u32(2).await.unwrap();
        }
        if packet_version >= 2 {
            packet.write_is_string(query_string).await.unwrap();
        }

        if packet_version >= 3 {
            packet.write_string(&dummy_name).await.unwrap();
        }

        packet.write_string("zh").await.unwrap();
        packet.write_u16(0).await.unwrap();

        Ok(packet)
    }
}
