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
    sync::{mpsc, Mutex},
    task::JoinHandle,
};

use crate::core::BlockRuntime;

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
    pub free_worker: Arc<Mutex<Vec<mpsc::Sender<D>>>>,
    worker_fn: F,
    runtime: BlockRuntime, //create_worker:G
    extra_worker_arg: EWA,
    pub worker_size: Arc<AtomicU32>,
}

impl<'a, D, R, F, EWA> WorkerPool<D, R, F, EWA>
where
    Self: 'static,
    F: FnOnce(
            mpsc::Receiver<D>,
            EWA,
        ) -> Pin<Box<dyn Future<Output = Result<R, anyhow::Error>> + Send + 'static>>
        + Copy,
    R: Sync + Send + 'static,
    EWA: Clone, //G: Fn() -> (JoinHandle<anyhow::Result<R>>,mpsc::Sender<D>)
{
    pub async fn get_free_worker(&mut self) -> mpsc::Sender<D> {
        match self.free_worker.lock().await.pop() {
            Some(receiver) => receiver,
            None => {
                let new_receiver = create_worker(
                    self.worker_fn,
                    self.runtime.clone(),
                    self.extra_worker_arg.clone(),
                )
                .await;
                self.worker_handle.push(new_receiver.0);
                self.worker_size.fetch_add(1, Ordering::Relaxed);
                new_receiver.1
            }
        }
    }

    pub async fn push_free_worker(&mut self, worker: mpsc::Sender<D>) {
        self.free_worker.lock().await.push(worker);
    }
}

async fn create_worker<D, R, F, EWA>(
    worker_fn: F,
    runtime: BlockRuntime,
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
        runtime
            .lock()
            .await
            .spawn((worker_fn)(receiver, extra_worker_arg)),
        sender,
    )
}

pub async fn new_worker_pool<D, R, F, EWA>(
    default_worker: usize,
    worker_fn: F,
    runtime: BlockRuntime,
    extra_worker_arg: EWA,
) -> (WorkerPool<D, R, F, EWA>, Arc<AtomicU32>)
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
        let new_worker = create_worker(worker_fn, runtime.clone(), extra_worker_arg.clone()).await;
        handle_vec.push(new_worker.0);
        worker_sender_vec.push(new_worker.1);
    }

    let worker_size = Arc::new(AtomicU32::new(default_worker as u32));
    (
        WorkerPool::<D, R, F, EWA> {
            worker_handle: handle_vec,
            free_worker: Arc::new(Mutex::new(worker_sender_vec)),
            worker_fn,
            runtime,
            extra_worker_arg: extra_worker_arg.clone(),
            worker_size: worker_size.clone(), //create_worker:create_workers
        },
        worker_size,
    )
}
/*
mod test {
    use std::{future::Future, pin::Pin, process::Output};

    use tokio::sync::mpsc::Receiver;

    use super::{new_worker_pool, WorkerPool};
    fn a() {
        let ab = new_worker_pool::<i128, (), _>(23, move |a| Box::pin(receiver(a)),).await;


        let b = ab.get_free_worker();

    }

    fn b() {
        let b = a();
    }

    pub async fn receiver(mut read_h_receiver: Receiver<i128>) -> anyhow::Result<()> {
        Ok(())
    }
}*/
