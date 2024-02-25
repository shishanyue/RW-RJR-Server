use std::sync::Arc;

use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf, sync::{broadcast, mpsc}};

use crate::{connection::shared_connection::SharedConnection, core::ServerCommand, packet::Packet};

pub type SenderData = (
    Arc<SharedConnection>,
    async_channel::Receiver<Packet>,
    OwnedWriteHalf,
    broadcast::Receiver<ServerCommand>,
);

pub async fn sender(mut data: mpsc::Receiver<SenderData>) -> anyhow::Result<()> {
    loop {
        let Some((shared_con, packet_rx,mut write_half,mut command_rx)) = data.recv().await else {
            continue;
        };
        loop {

            tokio::select! {
                data = packet_rx.recv() => {
                    match data {
                        Ok(mut packet) => {
                            
                            packet.prepare().await;
                            println!("PermissionStatus:{:?}SendPacket:{:?}\n",shared_con.shared_data.player_info.permission_status.read(),packet);
                            write_half
                            .write_all(&packet.packet_buffer.into_inner())
                            .await
                            .unwrap()},
                        Err(_) => {shared_con.disconnect().await;break;},
                    }
                }
                command = command_rx.recv() => {
                    match command {
                        Ok(command) => {
                            match command {
                                ServerCommand::Disconnect => {
                                    break;
                                },
                                ServerCommand::None => {},
                            }
                        },
                        Err(_) => {shared_con.disconnect().await;break;},
                    }
                }
            }
            
        }
    }
}
