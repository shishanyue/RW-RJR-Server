use std::{future::poll_fn, sync::{atomic::{AtomicBool, Ordering}, Arc}};

use async_trait::async_trait;
use log::info;
use tokio::{
    io::AsyncWriteExt,
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::RwLock,
};

use crate::{
    dummy::{BasicDummy, BasicDummyAPI, BasicDummyTrait, DummyVersion},
    error::BasicDummyError,
    packet::common_packet::CommonPacket,
};

use super::Module;

pub struct EasyDummy {
    index: u32,
    dummy: Arc<RwLock<BasicDummy>>,
}

struct EasyDummyTrait;

impl EasyDummyTrait {
    pub fn new() -> Self {
        EasyDummyTrait {}
    }
}

#[async_trait]
impl BasicDummyTrait for EasyDummyTrait {
    async fn packet_receiver(&self) {
        todo!()
    }

    async fn startup(
        &self,
        dummy: Arc<RwLock<BasicDummy>>,
        api_tx: async_channel::Sender<BasicDummyAPI>,
    ) {
        todo!()
    }

    async fn startup_connected(
        &self,
        dummy: Arc<RwLock<BasicDummy>>,
        api_tx: async_channel::Sender<BasicDummyAPI>,
    ) {
        let packet = CommonPacket::get_preregister_info(
            &dummy.read().await.dummy_name,
            &dummy.read().await.domain,
            dummy.read().await.packet_version,
            dummy.read().await.client_version,
            "",
        )
        .await
        .unwrap();

        api_tx
            .send(BasicDummyAPI::SendPacket(packet))
            .await
            .unwrap();
        dummy.write().await.is_connect.fetch_or(true, Ordering::SeqCst);
    }
}

impl EasyDummy {
    pub async fn new(index: u32, dummy_name: &str, domain: &str, version: DummyVersion) -> Self {
        EasyDummy {
            index,
            dummy: BasicDummy::new(dummy_name, domain, version, Box::new(EasyDummyTrait::new()))
                .await,
        }
    }

    pub async fn get_is_connect_arc(&self) -> Arc<AtomicBool>{
        self.dummy.read().await.is_connect.clone()
    }

    pub async fn connect_to_relay(&self, addr: &str) -> Result<&Self, BasicDummyError> {
        self.dummy.write().await.connect_to_relay(addr).await?;
        Ok(self)
    }
    pub async fn join_to_room(&self, id: &str) -> Result<&Self, BasicDummyError> {
        self.dummy.write().await.join_to_room(id).await?;
        Ok(self)
    }
}
impl Module for EasyDummy {}
