use std::{io::Read, sync::Arc};


use crate::{
    connection_manager::{ConnectionManager}, event::{EVENT_CHANNEL_MULTIPLE}
};

pub async fn command_center(shared_connection_mg: Arc<ConnectionManager>) {
    let event_receiver = EVENT_CHANNEL_MULTIPLE.1.resubscribe();

    //MODULE_MANAGER
    //    .write()
    //    .unwrap()
    //    .init_module(ModuleType::RwEngine);

    let mut std_in = std::io::stdin();
    let mut admin_command = String::new();

    
    loop {
        std_in.read_to_string(&mut admin_command).unwrap();



        admin_command.clear();
    }

    //loop {
        /*
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
                EventType::NewPacket(_, packet, PacketType::CHAT) => {
                }
                _ => {}
            },
            Err(e) => panic!("{}", e),
        };
        
         */
        

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
    //}
    //std::thread::sleep(Duration::from_millis(u64::MAX));
}
