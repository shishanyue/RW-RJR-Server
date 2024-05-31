
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
        packet.write_string(domain).await?;

        packet.write_u32(packet_version).await?;
        packet.write_u32(client_version).await?;

        if packet_version >= 1 {
            packet.write_u32(2).await?;
        }
        if packet_version >= 2 {
            packet.write_is_string(query_string).await?;
        }

        if packet_version >= 3 {
            packet.write_string(dummy_name).await?;
        }

        packet.write_string("zh").await?;
        packet.write_u16(0).await?;

        Ok(packet)
    }

    pub async fn get_relay_hall_command(command:&str)-> std::io::Result<Packet> {
        let mut packet = Packet::new(PacketType::RELAY_118_117_RETURN).await;
        packet.write_u8(1).await?;
        packet.write_u32(0).await?;
        packet.write_string(command).await?;
        Ok(packet)
    }
}
