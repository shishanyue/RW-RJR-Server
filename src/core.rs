




use tokio::{
    runtime::{Builder, Runtime},
    sync::{
        mpsc::{self},
    },
};

use crate::{
    worker_pool::{
        receiver::{ReceiverData},
        sender::{SenderData},
    },
};

#[derive(Debug, Clone, Copy)]
pub enum ServerCommand {
    Disconnect
}


pub async fn creat_block_runtime(threads: usize) -> anyhow::Result<Runtime> {
    Ok(Builder::new_multi_thread()
        .enable_time()
        .worker_threads(threads)
        // no timer!
        .build()
        .unwrap())
}

