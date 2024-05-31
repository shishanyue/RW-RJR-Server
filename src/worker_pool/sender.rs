use std::sync::{atomic::{AtomicI64, Ordering}, Arc};

use tokio::{
    io::AsyncWriteExt,
    net::tcp::OwnedWriteHalf,
    sync::{broadcast},
};

use crate::{
    connection::shared_connection::SharedConnection,
    core::ServerCommand,
    packet::Packet,
};

pub type SenderData = (
    Arc<SharedConnection>,
    async_channel::Receiver<Packet>,
    OwnedWriteHalf,
    broadcast::Receiver<ServerCommand>,
);

pub async fn sender(
    data: async_channel::Receiver<(SenderData, Arc<AtomicI64>)>,
) -> anyhow::Result<()> {
    loop {
        match data.recv().await {
            Ok(((shared_con, packet_rx, mut write_half, mut command_rx), permit)) => {
                permit.fetch_add(1, Ordering::Relaxed);
                loop {
                    tokio::select! {
                        data = packet_rx.recv() => {
                            match data {
                                Ok(mut packet) => {

                                    packet.prepare().await;

                                    match write_half
                                    .write_all(&packet.packet_buffer.into_inner())
                                    .await {
                                    Ok(_) => {
                                    },
                                    Err(_) => {shared_con.disconnect().await;break;},
                                    }
                                    },
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
