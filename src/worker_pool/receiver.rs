mod error;

use std::io::Cursor;

use std::{sync::Arc, usize};

use crate::connection::shared_connection::SharedConnection;

use crate::core::ServerCommand;
use crate::packet::{Packet, PacketType};
use crate::worker_pool::receiver::error::ErrorKind;

use tokio::sync::broadcast;

use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf, sync::mpsc};

use self::error::Error;

pub type ReceiverData = (
    Arc<SharedConnection>,
    OwnedReadHalf,
    broadcast::Receiver<ServerCommand>,
);

async fn receiver_fn(read_half: &mut OwnedReadHalf) -> anyhow::Result<Packet> {
    let packet_length = read_half.read_i32().await?;

    let packet_type = PacketType::try_from(read_half.read_u32().await?).unwrap_or_default();

    if packet_length <= 0 {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "packet length below zero",
        ))?
    }
    if packet_length >= 1024 * 50 || packet_type == PacketType::NOT_RESOLVED {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "packet length too long",
        ))?
    }

    let mut packet_buffer = vec![0; packet_length as usize];
    read_half.read_exact(&mut packet_buffer).await?;

    Ok(Packet::decode_from_buffer(
        packet_length as u32,
        packet_type,
        Cursor::new(packet_buffer),
    ))
}

pub async fn receiver(mut data: mpsc::Receiver<ReceiverData>) -> anyhow::Result<()> {
    loop {
        match data.recv().await {
            Some((shared_con, mut read_half, mut command_rx)) => {
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
            }
            None => continue,
        }
    }
}
