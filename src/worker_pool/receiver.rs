use std::io::Cursor;

use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};
use std::{sync::Arc, usize};

use crate::connection::shared_connection::SharedConnection;

use crate::core::ServerCommand;
use crate::error::ReceiverError;
use crate::packet::{Packet, PacketType};

use tokio::sync::{broadcast, SemaphorePermit};

use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};

pub type ReceiverData = (
    Arc<SharedConnection>,
    OwnedReadHalf,
    broadcast::Receiver<ServerCommand>,
);

pub async fn receiver_fn(read_half: &mut OwnedReadHalf) -> anyhow::Result<Packet> {
    let packet_length = read_half.read_i32().await?;

    let packet_type = PacketType::try_from(read_half.read_u32().await?).unwrap_or_default();

    if packet_length <= 0 {
        return Err(ReceiverError::InvalidInput("packet length below zero".to_string()).into());
    }
    if packet_length >= 1024 * 50 || packet_type == PacketType::NOT_RESOLVED {
        return Err(ReceiverError::InvalidInput("packet length too long".to_string()).into());
    }

    let mut packet_buffer = vec![0; packet_length as usize];
    read_half.read_exact(&mut packet_buffer).await?;

    Ok(Packet::decode_from_buffer(
        packet_length as u32,
        packet_type,
        Cursor::new(packet_buffer),
    ))
}

pub async fn receiver(
    data: async_channel::Receiver<(ReceiverData, Arc<AtomicI64>)>,
) -> anyhow::Result<()> {
    loop {
        match data.recv().await {
            Ok(((shared_con, mut read_half, mut command_rx), permit)) => {
                permit.fetch_add(1, Ordering::Relaxed);
                loop {
                    tokio::select! {
                        recv = receiver_fn(&mut read_half) => {
                            match recv {
                                Ok(packet) => {
                                    //println!("PermissionStatus:{:?}ReceiverPacket:{:?}\n",shared_con.shared_data.player_info.permission_status.read(),packet);
                                    shared_con.type_relay(shared_con.clone(), packet).await;
                                    continue;
                                }
                                Err(_) => {shared_con.disconnect().await;break;}
                            };
                        }

                        command = command_rx.recv() => {
                            match command {
                                Ok(command) => {
                                    match command {
                                        ServerCommand::Disconnect => {
                                            break;
                                        },
                                    }
                                },
                                Err(_) => {shared_con.disconnect().await;break;},
                            }
                        }
                    }
                }
                permit.fetch_add(-1, Ordering::Relaxed);
            }
            Err(_) => continue,
        }
    }
}
