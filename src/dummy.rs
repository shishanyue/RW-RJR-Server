use std::{collections::HashMap, sync::{atomic::AtomicBool, Arc}};

use async_trait::async_trait;
use log::info;
use tokio::{
    io::AsyncWriteExt,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    runtime::{Builder, Runtime},
    sync::{mpsc, RwLock},
    task::JoinHandle,
};

use crate::{error::BasicDummyError, packet::{Packet, PacketReadWriteExt, PacketType}, worker_pool::receiver::receiver_fn};

lazy_static! {
    static ref DUMMY_RUNTIME:Runtime = Builder::new_multi_thread()
    .enable_time()
    .worker_threads(10)
    .enable_io()
    // no timer!
    .build()
    .expect("creat block runtime error");

}

#[async_trait]
pub trait BasicDummyTrait
where
    Self: Sync + Send + 'static,
{
    async fn packet_receiver(&self);
    async fn startup(
        &self,
        dummy: Arc<RwLock<BasicDummy>>,
        api_tx: async_channel::Sender<BasicDummyAPI>,
    );
    async fn startup_connected(
        &self,
        dummy: Arc<RwLock<BasicDummy>>,
        api_tx: async_channel::Sender<BasicDummyAPI>,
    );
}

pub enum BasicDummyAPI {
    SendPacket(Packet),
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
    pub is_connect:Arc<AtomicBool>,
    pub net_handle: Option<JoinHandle<()>>,
    arc_self: Option<Arc<RwLock<Self>>>,
    dummy_trait: Arc<Box<dyn BasicDummyTrait + 'static>>,
    basic_dummy_api_tx: async_channel::Sender<BasicDummyAPI>,
    basic_dummy_api_rx: async_channel::Receiver<BasicDummyAPI>,
}

impl BasicDummy {
    pub async fn new(
        dummy_name: &str,
        domain: &str,
        version: DummyVersion,
        dummy_trait: Box<dyn BasicDummyTrait + 'static>,
    ) -> Arc<RwLock<Self>> {
        let (basic_dummy_api_tx, basic_dummy_api_rx) = async_channel::bounded(10);
        match version {
            DummyVersion::Version1_15 => {
                let dummy = Arc::new(RwLock::new(Self {
                    dummy_name: dummy_name.to_string(),
                    domain: domain.to_string(),
                    packet_version: 4,
                    client_version: 172,
                    net_handle: None,
                    is_connect:Arc::new(AtomicBool::new(false)),
                    arc_self: None,
                    basic_dummy_api_tx,
                    basic_dummy_api_rx,
                    dummy_trait: Arc::new(dummy_trait),
                }));

                dummy.write().await.set_arc_self(dummy.clone());
                dummy
            }
            DummyVersion::Version1_16 => todo!(),
        }
    }

    fn set_arc_self(&mut self, arc_self: Arc<RwLock<Self>>) {
        self.arc_self = Some(arc_self)
    }

    pub async fn connect_to_relay(&mut self, addr: &str) -> Result<(), BasicDummyError> {
        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                let api_rx = self.basic_dummy_api_rx.clone();
                let api_tx = self.basic_dummy_api_tx.clone();
                self.net_handle = Some(DUMMY_RUNTIME.spawn(basic_dummy_fn(
                    stream,
                    self.arc_self.as_ref().unwrap().clone(),
                    api_rx,
                    api_tx,
                )));

                Ok(())
            }
            Err(_) => Err(BasicDummyError::JoinRelayError("connect error")),
        }
    }

    pub async fn join_to_room(&self,id:&str) ->  Result<(), BasicDummyError>{
        let mut packet = Packet::new(PacketType::RELAY_118_117_RETURN).await;
        packet.write_u8(1).await.unwrap();
        packet.write_u32(0).await.unwrap();
        packet.write_string(id).await.unwrap();

        self.basic_dummy_api_tx.send(BasicDummyAPI::SendPacket(packet)).await.unwrap();
        Ok(())
    }
}

async fn basic_dummy_fn(
    stream: TcpStream,
    dummy: Arc<RwLock<BasicDummy>>,
    api_rx: async_channel::Receiver<BasicDummyAPI>,
    api_tx: async_channel::Sender<BasicDummyAPI>,
) {
    let (mut read_half, mut write_half) = stream.into_split();

    let dummy_trait = dummy.read().await.dummy_trait.clone();

    dummy_trait
        .startup_connected(dummy.clone(), api_tx.clone())
        .await;


    loop {
        tokio::select! {
            recv = receiver_fn(&mut read_half) => {
                match recv {
                    Ok(packet) => {

                        continue;
                    }
                    Err(_) => {break;}
                };
            }

            Ok(api) = api_rx.recv() => {
                match api {
                    BasicDummyAPI::SendPacket(mut packet) => {
                        println!("{:?}",packet);
                        packet.prepare().await;
                        write_half
                            .write_all(&packet.packet_buffer.into_inner())
                            .await
                            .expect("dummy send packet error");
                    },
                }
            }
        }
    }
}
