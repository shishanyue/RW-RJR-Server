use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use dashmap::{DashMap};

use rand::{Rng, SeedableRng};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    runtime::{Builder, Runtime},
    sync::{
        mpsc::{self},
        oneshot, RwLock,
    },
    task::JoinHandle,
};

use crate::{
    connection::{
        player_net_api::{CustomRelayData},
        Connection,
    },
    packet::{Packet, PacketReadWriteExt, PacketType},
    worker_pool::{
        receiver::{ReceiverData},
        sender::{SenderData},
    },
};

#[derive(Debug, Clone, Copy)]
pub enum ServerCommand {
    Disconnect,
    #[allow(dead_code)]
    None,
}

pub type WorkersSender = (mpsc::Sender<ReceiverData>, mpsc::Sender<SenderData>);

pub async fn creat_block_runtime(threads: usize) -> anyhow::Result<Runtime> {
    Ok(Builder::new_multi_thread()
        .enable_time()
        .worker_threads(threads)
        // no timer!
        .build()
        .unwrap())
}

