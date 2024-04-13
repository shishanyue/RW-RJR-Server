use std::{collections::HashMap, sync::Arc};

use tokio::{
    net::{tcp::OwnedWriteHalf, TcpStream},
    runtime::{Builder, Runtime},
    sync::{mpsc, RwLock},
    task::JoinHandle,
};

use crate::{error::BasicDummyError, packet::Packet, worker_pool::receiver::receiver_fn};

lazy_static! {
    static ref DUMMY_RUNTIME:Runtime = Builder::new_multi_thread()
    .enable_time()
    .worker_threads(10)
    // no timer!
    .build()
    .expect("creat block runtime error");

}

pub enum BasicDummyAPI {
    WriteHalf(Box<dyn FnOnce(&mut OwnedWriteHalf) + 'static + Sync + Send>),
}

pub enum DummyVersion {
    Version1_15,
    Version1_16,
}

pub enum TriggerFnWhenPacket {
    All,
    On(Packet),
}

pub struct BasicDummy {
    pub dummy_name: String,
    pub domain: String,
    pub packet_version: u32,
    pub client_version: u32,
    pub net_handle: Option<JoinHandle<()>>,
    basic_dummy_api_tx: async_channel::Sender<BasicDummyAPI>,
    basic_dummy_api_rx: async_channel::Receiver<BasicDummyAPI>,
    when_receiver_fn: HashMap<TriggerFnWhenPacket, Vec<Box<dyn FnOnce() + Send + Sync + 'static>>>,
}

impl BasicDummy {
    pub fn new(dummy_name: &str, domain: &str, version: DummyVersion) -> Arc<RwLock<Self>> {
        let (basic_dummy_api_tx, basic_dummy_api_rx) = async_channel::bounded(10);
        match version {
            DummyVersion::Version1_15 => Arc::new(RwLock::new(Self {
                dummy_name: dummy_name.to_string(),
                domain: domain.to_string(),
                packet_version: 4,
                client_version: 163,
                net_handle: None,
                when_receiver_fn: HashMap::new(),
                basic_dummy_api_tx,
                basic_dummy_api_rx,
            })),
            DummyVersion::Version1_16 => todo!(),
        }
    }

    pub async fn write_half_fn(
        &self,
        clouse: Box<dyn FnOnce(&mut OwnedWriteHalf) + 'static + Sync + Send>,
    ) -> Result<(), async_channel::SendError<BasicDummyAPI>> {
        self.basic_dummy_api_tx
            .send(BasicDummyAPI::WriteHalf(clouse))
            .await?;
        Ok(())
    }
    pub async fn connect_to_relay(
        dummy: Arc<RwLock<Self>>,
        addr: &str,
    ) -> Result<(), BasicDummyError> {
        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                let api_rx = dummy.read().await.basic_dummy_api_rx.clone();
                dummy.write().await.net_handle =
                    Some(DUMMY_RUNTIME.spawn(basic_dummy_fn(stream, dummy.clone(), api_rx)));
                Ok(())
            }
            Err(_) => Err(BasicDummyError::JoinRelayError("connect error")),
        }
    }
}

async fn basic_dummy_fn(
    stream: TcpStream,
    dummy: Arc<RwLock<BasicDummy>>,
    api_rx: async_channel::Receiver<BasicDummyAPI>,
) {
    let (mut read_half, mut write_half) = stream.into_split();

    loop {
        tokio::select! {
            recv = receiver_fn(&mut read_half) => {
                match recv {
                    Ok(packet) => {
                        for (fn_type ,function) in dummy.write().await.when_receiver_fn.iter(){

                        }
                        continue;
                    }
                    Err(_) => {break;}
                };
            }

            Ok(api) = api_rx.recv() => {
                match api {
                    BasicDummyAPI::WriteHalf(f) => f(&mut write_half),
                }
            }
        }
    }
}
