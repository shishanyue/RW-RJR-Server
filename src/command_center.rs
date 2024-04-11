use std::{sync::Arc, time::Duration};

use crate::{connection_manager::{By, ConnectionManager}, packet::super_packet::SuperPacket};


pub async fn command_center(shared_connection_mg:Arc<ConnectionManager>){
    let std_in = std::io::stdin();
    let mut admin_command = String::new();
    loop {
        std_in.read_line(&mut admin_command).unwrap();
        let admin_command = admin_command.trim().to_string();

        for ten1 in 1..20 {
            let packet = SuperPacket::set_terrain(ten1 as f32 * 20. + 20., 500., "1").await;

            shared_connection_mg
                .send_packet_to_player_by(By::Addr(admin_command.clone()), packet)
                .await;
        }

        for ten2 in 1..20 {
            let packet = SuperPacket::set_terrain(100., ten2 as f32 * 20. + 200., "1").await;

            shared_connection_mg
                .send_packet_to_player_by(By::Addr(admin_command.clone()), packet)
                .await;
        }
    }
    //std::thread::sleep(Duration::from_millis(u64::MAX));
}