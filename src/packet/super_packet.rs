use tokio::io::AsyncWriteExt;

use super::{Packet, PacketReadWriteExt, PacketType};

/*
02 00 00 00 45 00 00 8e 21 9d 40 00 40 06 00 00
7f 00 00 01 7f 00 00 01 14 03 c7 56 dd d5 db 8a
57 65 74 d8 50 18 27 96 30 22 00 00 00 00 00 5e
00 00 00 ae 00 00 00 00 00 00 00 56 00 00 00 4e



02 00 00 00 45 00 00 8e 8f af 40 00 40 06 00 00
7f 00 00 01 7f 00 00 01 14



03 c8 9a 49 57 b3 61
7b d7 0c 8e 50 18 27 d0 49 b3 00 00 00 00 00 5e
00 00 00 ae 00 00 00 00 00 00 00 56 00 00 00 4e
00 00 00 14

0x00, 0x01, 0x63, 0x00, 0x00, 0x00, 0x47, 0x00, 0x00, 0x00, 0x00, 0xff
0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00
0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xd9, 0x01, 0x00, 0x01



43 6a 00 00 45 07 d0 00
ff ff ff ff ff ff ff ff 00 0f 53 65 74 54 65 72
72 61 69 6e 54 79 70 65 32 00 00 00 00 00 00 00
00 00



*/

static SET_TERRAIN_PACKET_1: [u8; 36] = [
    0x00, 0x01, 0x63, 0x00, 0x00, 0x00, 0x47, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xd9, 0x01, 0x00, 0x01,
];

static SET_TERRAIN_PACKET_2: [u8; 8] = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
pub struct SuperPacket;

impl SuperPacket {
    pub async fn set_terrain(x: f32, y: f32, terrain_name: &str) -> Packet {
        let mut packet = Packet::new(PacketType::GAMECOMMAND_RECEIVE).await;

        packet
            .packet_buffer
            .write_all(&SET_TERRAIN_PACKET_1)
            .await
            .expect("error");

        packet.write_f32(x).await.expect("error");
        packet.write_f32(y).await.expect("error");

        packet
            .packet_buffer
            .write_all(&SET_TERRAIN_PACKET_2)
            .await
            .expect("error");

        packet
            .write_string(&format!("SetTerrainType{}", terrain_name))
            .await
            .expect("error");
        packet.write_u32(0).await.unwrap();
        packet.write_u32(0).await.unwrap();
        packet.write_u8(0).await.unwrap();

        Self::packet_to_host(packet).await
    }

    pub async fn packet_to_host(mut packet: Packet) -> Packet {
        packet.prepare().await;
        let mut send_packet = Packet::new(PacketType::PACKET_FORWARD_CLIENT_FROM).await;

        send_packet.write_u32(0).await.unwrap();
        send_packet
            .packet_buffer
            .write_u32(packet.packet_length + 8)
            .await
            .unwrap();

        send_packet
            .packet_buffer
            .write_all(&packet.packet_buffer.into_inner())
            .await
            .unwrap();

        send_packet
    }
}
