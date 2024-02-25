use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc,
};

use crate::worker_pool::{receiver::ReceiverData, sender::SenderData};

#[derive(Debug, Clone, Copy)]
pub enum ServerCommand {
    Disconnect,
    #[allow(dead_code)]
    None,
}

// TODO: use it
pub type _WorkersSender = (mpsc::Sender<ReceiverData>, mpsc::Sender<SenderData>);

pub async fn creat_block_runtime(threads: usize) -> anyhow::Result<Runtime> {
    Ok(Builder::new_multi_thread()
        .enable_time()
        .worker_threads(threads)
        // no timer!
        .build()
        .unwrap())
}
