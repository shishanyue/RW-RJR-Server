pub mod processor;
pub mod receiver;
pub mod sender;

use std::{
    future::Future,
    pin::Pin,
    sync::{atomic::{AtomicI64, Ordering}, Arc},
};

use tokio::{
    runtime::Runtime,
    task::JoinHandle,
};

pub struct WorkerPool<D, R, F, EWA>
where
    R: Sync + Send + 'static,
    F: FnOnce(
            async_channel::Receiver<(D, Arc<AtomicI64>)>,
            EWA,
        ) -> Pin<Box<dyn Future<Output = Result<R, anyhow::Error>> + Send + 'static>>
        + Copy,
    EWA: Clone,
{
    worker_handle: Vec<JoinHandle<anyhow::Result<R>>>,
    working_num: Arc<AtomicI64>,
    task_sender: async_channel::Sender<(D, Arc<AtomicI64>)>,
    task_receiver: async_channel::Receiver<(D, Arc<AtomicI64>)>,
    worker_fn: F,
    runtime: Runtime,
    extra_worker_arg: EWA,
    worker_size: u32,
}

impl<D, R, F, EWA> WorkerPool<D, R, F, EWA>
where
    F: FnOnce(
            async_channel::Receiver<(D, Arc<AtomicI64>)>,
            EWA,
        ) -> Pin<Box<dyn Future<Output = Result<R, anyhow::Error>> + Send + 'static>>
        + Copy,
    R: Sync + Send + 'static,
    EWA: Clone,
{
    pub async fn push_task(
        &mut self,
        task_data: D,
    ) -> Result<(), async_channel::SendError<(D, Arc<AtomicI64>)>> {

        if self.working_num.load(Ordering::Relaxed) as u32 >= self.worker_size{
            self.new_worker(1).await;
        }

        self.task_sender.send((task_data, self.working_num.clone())).await
    }

    async fn create_worker(
        worker_fn: F,
        task_receiver: async_channel::Receiver<(D, Arc<AtomicI64>)>,
        runtime: &mut Runtime,
        extra_worker_arg: EWA,
    ) -> JoinHandle<anyhow::Result<R>>
    where
        F: FnOnce(
                async_channel::Receiver<(D, Arc<AtomicI64>)>,
                EWA,
            )
                -> Pin<Box<dyn Future<Output = Result<R, anyhow::Error>> + Send + 'static>>
            + Copy,
        R: Sync + Send + 'static,
        EWA: Clone,
    {
        runtime.spawn((worker_fn)(task_receiver, extra_worker_arg))
    }

    async fn new_worker(&mut self, default_worker: usize) {
        self.worker_size += default_worker as u32;

        for _ in 0..=default_worker {
            let new_worker = WorkerPool::create_worker(
                self.worker_fn,
                self.task_receiver.clone(),
                &mut self.runtime,
                self.extra_worker_arg.clone(),
            )
            .await;
            self.worker_handle.push(new_worker);
        }
    }
}

pub async fn new_worker_pool<D, R, F, EWA>(
    default_worker: usize,
    worker_fn: F,
    runtime: Runtime,
    extra_worker_arg: EWA,
) -> WorkerPool<D, R, F, EWA>
where
    R: Sync + Send + 'static,
    F: FnOnce(
            async_channel::Receiver<(D, Arc<AtomicI64>)>,
            EWA,
        ) -> Pin<Box<dyn Future<Output = Result<R, anyhow::Error>> + Send + 'static>>
        + Copy,
    EWA: Clone,
{
    let (task_sender, task_receiver) = async_channel::unbounded();
    let mut pool = WorkerPool::<D, R, F, EWA> {
        worker_handle: Vec::with_capacity(default_worker),
        task_sender,
        task_receiver: task_receiver.clone(),
        worker_fn,
        runtime,
        working_num: Arc::new(AtomicI64::new(0)),
        extra_worker_arg: extra_worker_arg.clone(),
        worker_size: default_worker as u32, //create_worker:create_workers
    };

    pool.new_worker(default_worker).await;

    pool
}
