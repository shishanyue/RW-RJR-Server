use tokio::{
    io::AsyncWriteExt,
    net::tcp::OwnedWriteHalf,
    sync::{broadcast, mpsc},
};

use crate::{
    core::ServerCommand,
    packet_core::{Packet, PacketType},
};

pub type SenderData = (
    broadcast::Receiver<ServerCommand>,
    OwnedWriteHalf,
    mpsc::Receiver<Packet>,
);

pub async fn sender(mut data: mpsc::Receiver<SenderData>) -> anyhow::Result<()> {
    loop {
        match data.recv().await {
            Some((mut command_receiver, mut write_half, mut packet_receiver)) => {
                loop {
                    tokio::select! {
                        packet_receiver = packet_receiver.recv() => {
                            if let Some(mut packet) = packet_receiver {

                                packet.prepare().await;
                                if packet.packet_type!= PacketType::HEART_BEAT || packet.packet_type!= PacketType::TEAM_LIST|| packet.packet_type!= PacketType::HEART_BEAT_RESPONSE {
                                //println!("{:?}",packet);
                                }
                                //println!("{:?}",packet);

                                write_half
                                        .write_all(&packet.packet_buffer.into_inner())
                                        .await
                                        .unwrap();

                            }
                        }

                        command = command_receiver.recv() => {
                            match command {
                                Ok(command) => {
                                    match command {
                                        ServerCommand::Disconnect => {
                                            break;
                                        },
                                        ServerCommand::None => {},
                                    }
                                },
                                Err(e) => panic!("{}", e),
                            }
                        }



                    }
                }
            }

            None => {
                continue;
            }
        }
    }
}
