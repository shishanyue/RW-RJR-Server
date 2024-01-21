
use tokio::{
    io::AsyncWriteExt,
    net::tcp::OwnedWriteHalf,
    sync::{mpsc, watch},
};

use crate::{
    core::ServerCommand,
    packet_core::Packet,
};

pub type SenderData = (
    watch::Receiver<ServerCommand>,
    OwnedWriteHalf,
    mpsc::Receiver<Packet>,
);

pub async fn sender(mut data: mpsc::Receiver<SenderData>) -> anyhow::Result<()> {
    //let write_half = data.2.recv().await;
    loop {
        match data.recv().await {
            Some((mut command_receiver, mut write_half, mut packet_receiver)) => loop {
                
                tokio::select! {
                    packet_receiver = packet_receiver.recv() => {
                        match packet_receiver {
                            Some(mut packet) => {
                                packet.prepare().await;
                                //if packet.packet_type!= PacketType::HEART_BEAT || packet.packet_type!= PacketType::TEAM_LIST|| packet.packet_type!= PacketType::HEART_BEAT_RESPONSE {
                                //}
                                write_half
                                    .write_all(&packet.packet_buffer.into_inner())
                                    .await
                                    .unwrap();
                            }
                            None => todo!(),
                        }
                    }

                    command_changed = command_receiver.changed() => {
                        if command_changed.is_ok(){
                            let command = *command_receiver.borrow();
                            match command {
                                ServerCommand::Disconnect => {
                                    break;
                                },
                                ServerCommand::None => todo!(),
                            }
                        }
                    }
                }
            }

            None => {continue;},
        }
    }
}
