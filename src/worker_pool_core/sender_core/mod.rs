use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use tokio::{
    io::AsyncWriteExt,
    net::tcp::OwnedWriteHalf,
    sync::{broadcast, mpsc, watch, Mutex, RwLock},
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
        loop {
            match data.recv().await {
                Some((mut command_receiver, mut write_half, mut packet_receiver)) => {
                    loop {
                        tokio::select! {
                            packet_receiver = packet_receiver.recv() => {
                                match packet_receiver {
                                    Some(mut packet) => {

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
                                    None => {},
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
}
