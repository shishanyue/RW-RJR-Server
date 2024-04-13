use std::sync::Arc;

use log::info;
use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf, sync::RwLock};

use crate::{
    dummy::{BasicDummy, BasicDummyAPI, DummyVersion},
    error::BasicDummyError, packet::common_packet::CommonPacket,
};

use super::Module;

pub struct EasyDummy {
    dummy: Arc<RwLock<BasicDummy>>,
}
impl EasyDummy {
    pub fn new(dummy_name: &str, domain: &str, version: DummyVersion) -> Self {
        EasyDummy {
            dummy: BasicDummy::new(dummy_name, domain, version),
        }
    }

    pub async fn write_half_fn(
        &self,
        clouse: Box<dyn FnOnce(&mut OwnedWriteHalf) + 'static + Sync + Send>,
    ) -> Result<(), async_channel::SendError<BasicDummyAPI>> {
        self.dummy.write().await.write_half_fn(clouse).await?;
        Ok(())
    }
    pub async fn connect_to_relay(&self, addr: &str) -> Result<(), BasicDummyError> {
        BasicDummy::connect_to_relay(self.dummy.clone(), addr).await?;
        self.dummy
            .write()
            .await
            .write_half_fn(Box::new(|write_half| {
                let packet = CommonPacket::get_preregister_info(dummy, query_string).await;

                write_half.write(packet)
            }))
            .await
            .expect("msg");
        Ok(())
    }
}
impl Module for EasyDummy {}
