use std::io::Cursor;

use std::{sync::Arc, usize};

use crate::connection_core::{Connection, ConnectionAPI, ConnectionChannel};
use crate::core::ServerCommand;
use crate::packet_core::{Packet, PacketType};

use tokio::runtime::Runtime;
use tokio::select;
use tokio::sync::broadcast;

use tokio::{
    io::AsyncReadExt,
    net::tcp::OwnedReadHalf,
    sync::{mpsc, RwLock},
};

use super::processor_core::ProcesseorData;

pub type ReceiverData = (Arc<ConnectionChannel>, OwnedReadHalf);

pub async fn receiver(mut data: mpsc::Receiver<ReceiverData>) -> anyhow::Result<()> {
    loop {
        let Some((con_channel, mut read_half)) = data.recv().await else {
            continue;
        };
        
        tokio::select! {
            packet_length = read_half.read_i32() => {
                match packet_length {
                    Ok(packet_length) => {

                        let packet_type = PacketType::try_from(read_half.read_u32().await.unwrap()).unwrap_or_default();


                        if packet_length <= 0{        
                            con_channel.connection_api_sender.send(ConnectionAPI::Disconnect).await.unwrap();
                            continue;
                        }

                        if packet_length >= 1024*50||packet_type == PacketType::NOT_RESOLVED{
                            con_channel.connection_api_sender.send(ConnectionAPI::Disconnect).await.unwrap();
                            continue;
                        }

                        let mut packet_buffer = vec![0; packet_length as usize];
                        read_half.read_exact(&mut packet_buffer).await.unwrap();


                        let packet = Packet::decode_from_buffer(
                            packet_length as u32,
                            packet_type,
                            Cursor::new(packet_buffer)).await;


                    },
                    Err(_) => con_channel.connection_api_sender.send(ConnectionAPI::Disconnect).await.unwrap(),
                }
            }
        }
    }
}
