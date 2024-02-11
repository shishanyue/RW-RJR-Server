pub mod processor_core;
pub mod receiver_core;
pub mod sender_core;

use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use tokio::{
    runtime::Runtime,
    sync::{mpsc, Mutex},
    task::JoinHandle,
};

pub struct WorkerPool<D, R, F, EWA>
where
    Self: 'static,
    R: Sync + Send + 'static,
    F: FnOnce(
            mpsc::Receiver<D>,
            EWA,
        ) -> Pin<Box<dyn Future<Output = Result<R, anyhow::Error>> + Send + 'static>>
        + Copy,
    EWA: Clone,
{
    worker_handle: Vec<JoinHandle<anyhow::Result<R>>>,
    pub free_worker: Vec<mpsc::Sender<D>>,
    pub back_worker_channel:(mpsc::Sender<mpsc::Sender<D>>,mpsc::Receiver<mpsc::Sender<D>>),
    worker_fn: F,
    runtime: Runtime,
    extra_worker_arg: EWA,
    pub worker_size: Arc<AtomicU32>,
}

async fn create_worker<D, R, F, EWA>(
    worker_fn: F,
    runtime: &mut Runtime,
    extra_worker_arg: EWA,
) -> (JoinHandle<anyhow::Result<R>>, mpsc::Sender<D>)
where
    F: FnOnce(
            mpsc::Receiver<D>,
            EWA,
        ) -> Pin<Box<dyn Future<Output = Result<R, anyhow::Error>> + Send + 'static>>
        + Copy,
    R: Sync + Send + 'static,
    EWA: Clone,
{
    let (sender, receiver) = mpsc::channel::<D>(10);
    (
        runtime.spawn((worker_fn)(receiver, extra_worker_arg)),
        sender,
    )
}

pub async fn new_worker_pool<D, R, F, EWA>(
    default_worker: usize,
    worker_fn: F,
    mut runtime: Runtime,
    extra_worker_arg: EWA,
) -> WorkerPool<D, R, F, EWA>
where
    F: FnOnce(
            mpsc::Receiver<D>,
            EWA,
        ) -> Pin<Box<dyn Future<Output = Result<R, anyhow::Error>> + Send + 'static>>
        + Copy,
    R: Sync + Send + 'static,
    EWA: Clone,
{
    let mut handle_vec = Vec::with_capacity(default_worker);

    let mut worker_sender_vec = Vec::with_capacity(default_worker);

    for _ in 0..=default_worker {
        let new_worker = create_worker(worker_fn, &mut runtime, extra_worker_arg.clone()).await;
        handle_vec.push(new_worker.0);
        worker_sender_vec.push(new_worker.1);
    }

    WorkerPool::<D, R, F, EWA> {
        worker_handle: handle_vec,
        free_worker: worker_sender_vec,
        back_worker_channel:mpsc::channel(10),
        worker_fn,
        runtime,
        extra_worker_arg: extra_worker_arg.clone(),
        worker_size: Arc::new(AtomicU32::new(default_worker as u32)), //create_worker:create_workers
    }
}
