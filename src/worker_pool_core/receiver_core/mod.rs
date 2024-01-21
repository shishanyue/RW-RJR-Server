use std::io::Cursor;

use std::{sync::Arc, usize};

use crate::connection_core::Connection;
use crate::core::ServerCommand;
use crate::packet_core::{Packet, PacketType};


use tokio::sync::watch;

use tokio::{
    io::AsyncReadExt,
    net::tcp::OwnedReadHalf,
    sync::{
        mpsc, RwLock,
    },
};

use super::processor_core::ProcesseorData;

pub type ReceiverData = (
    watch::Receiver<ServerCommand>,
    Arc<RwLock<Connection>>,
    OwnedReadHalf,
    mpsc::Sender<Packet>,
);

pub async fn receiver(
    mut read_h_receiver: mpsc::Receiver<ReceiverData>,
    sorter_sender: mpsc::Sender<ProcesseorData>,
) -> anyhow::Result<()> {
    loop {
        match read_h_receiver.recv().await {
            Some((mut command_receiver, con, mut read_half, packet_sender)) => loop {
                
                tokio::select! {
                    packet_length = read_half.read_i32() => {
                        match packet_length {
                            Ok(packet_length) => {
                                
                                let packet_type = PacketType::try_from(read_half.read_u32().await.unwrap()).unwrap_or_default();


                                if packet_length <= 0{
                                    con.write().await.disconnect().await;
                                    continue;
                                }

                                if packet_length >= 1024*50||packet_type == PacketType::NOT_RESOLVED{
                                    con.write().await.disconnect().await;
                                    continue;
                                }

                                

                                let mut packet_buffer = vec![0; packet_length as usize];
                                read_half.read_exact(&mut packet_buffer).await.unwrap();

                                let packet = Packet::decode_from_buffer(
                                    packet_length as u32,
                                    packet_type,
                                    Cursor::new(packet_buffer),).await;
                                sorter_sender
                                    .send((con.clone(), packet_sender.clone(), packet))
                                    .await
                                    .unwrap();
                            },
                            Err(_) => con.write().await.disconnect().await,
                        }
                    }

                    command_changed = command_receiver.changed() => {
                        if command_changed.is_ok(){
                            let command = *command_receiver.borrow();
                            match command {
                                ServerCommand::Disconnect => {
                                    break;
                                },
                                ServerCommand::None => {},
                            }
                        }
                    }
                }
            },
            None => {continue;},
            //
        }
    }
}
