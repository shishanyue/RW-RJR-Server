use crate::{creat_block_runtime, server_core::ServerConfig};
use tokio::{
    join, net::tcp::OwnedReadHalf, runtime::{Builder, Runtime}, sync::mpsc
};

use crate::worker_pool_core::{
    new_worker_pool,
    processor_core::{init_processor_sorter, processor},
    receiver_core::init_receiver_sorter,
};

pub struct WorkerPoolManager {
    pub receiver:mpsc::Sender<OwnedReadHalf>

}
impl WorkerPoolManager {
    pub async fn new(server_config: ServerConfig) -> anyhow::Result<Self> {
        let (receiver_block_rt, processor_block_rt, packet_sender_block_rt) = match join!(
            creat_block_runtime(server_config.thread_number),
            creat_block_runtime(server_config.thread_number),
            creat_block_runtime(server_config.thread_number)
        ) {
            (Ok(receiver_block_rt), Ok(processor_block_rt), Ok(packet_sender_block_rt)) => (
                receiver_block_rt,
                processor_block_rt,
                packet_sender_block_rt,
            ),
            _ => {
                panic!("creating_runtime_error")
            }
        };

        let worker_mg_rt = Builder::new_multi_thread()
            .enable_time()
            .worker_threads(3)
            // no timer!
            .build()
            .expect("worker manager create error!");

        let receiver = worker_mg_rt
            .spawn(init_receiver_sorter(receiver_block_rt))
            .await?;

        let processor_pool_h = worker_mg_rt
            .spawn(init_processor_sorter(processor_block_rt))
            .await?;

        Ok(WorkerPoolManager { receiver })
    }
}
