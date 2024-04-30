use std::{sync::Arc, time::Duration};

use log::{info, warn};

use crate::{
    connection_manager::{By, ConnectionManager},
    event::{self, EventType, EVENT_CHANNEL_MULTIPLE},
    module::{ModuleType, MODULE_MANAGER},
    packet::{self, super_packet::SuperPacket, PacketReadWriteExt, PacketType},
};

pub async fn command_center(shared_connection_mg: Arc<ConnectionManager>) {
    std::thread::sleep(Duration::from_millis(u64::MAX));
    let mut event_receiver = EVENT_CHANNEL_MULTIPLE.1.resubscribe();

    //MODULE_MANAGER
    //    .write()
    //    .unwrap()
    //    .init_module(ModuleType::RwEngine);

    let std_in = std::io::stdin();
    let mut admin_command = String::new();

    std_in.read_line(&mut admin_command).unwrap();
    let admin_command = admin_command.trim().to_string();
    //println!("Ok");
    loop {
        match event_receiver.recv().await {
            Ok(event) => match event.event_type {
                EventType::NewPacket(_, mut packet, PacketType::CHAT_RECEIVE) => {
                    let message = packet.read_string().await.unwrap();

                    let mut arg = message.split_whitespace();
                    
                    if arg.next() == Some("RwEnigne"){
                        let packet = SuperPacket::set_terrain(arg.next().unwrap().parse().unwrap(), arg.next().unwrap().parse().unwrap(), "1").await;

                        shared_connection_mg
                            .send_packet_to_player_by(By::Addr(admin_command.clone()), packet)
                            .await;
                    }

                }
                EventType::NewPacket(_, mut packet, PacketType::CHAT) => {
                }
                _ => {}
            },
            Err(e) => panic!("{}", e),
        };

        /*
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
         */
    }
    //std::thread::sleep(Duration::from_millis(u64::MAX));
}
