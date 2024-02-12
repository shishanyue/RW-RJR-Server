use std::sync::Arc;

use tokio::{
    io::AsyncWriteExt,
    net::tcp::OwnedWriteHalf,
    sync::{broadcast, mpsc},
};

use crate::{
    connection_core::{ConnectionAPI, ConnectionChannel}, core::ServerCommand, packet_core::{Packet, PacketType}
};

pub type SenderData = (Arc<ConnectionChannel>, OwnedWriteHalf);

pub async fn sender(mut data: mpsc::Receiver<SenderData>) -> anyhow::Result<()> {
    loop {
        let Some((mut con_channel, mut read_half)) = data.recv().await else { continue; };
        std::thread::sleep(std::time::Duration::from_millis(u64::MAX));

    }
}
