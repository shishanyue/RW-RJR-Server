use std::time::Duration;

use log::{info, warn};

use crate::{
    dummy::DummyVersion,
    event::{EventType, EVENT_CHANNEL_MULTIPLE},
    module::{easy_dummy::EasyDummy, Module, MODULE_RUNTIME},
};

pub mod image;

pub struct RwEngine {}
impl RwEngine {
    pub fn new() -> Self {
        info!("RwEnige 正在启动!");
        MODULE_RUNTIME.spawn(async move {
            let mut event_reader = EVENT_CHANNEL_MULTIPLE.1.resubscribe();
            loop {
                let Ok(event) = event_reader.recv().await else {
                    warn!("event recv error");
                    continue;
                };
                match event.event_type {
                    EventType::NewRoomAndHostOk(room) => {
                        let new_dummy = EasyDummy::new(
                            "RwEngineSystem",
                            "com.shishanyue.rwe",
                            DummyVersion::Version1_15,
                        );
                        new_dummy
                            .connect_to_relay("127.0.0.1:5123")
                            .await
                            .expect("connet to relay error");
                        loop {
                            std::thread::sleep(Duration::from_millis(u64::MAX));
                        }
                    }
                }
            }
        });
        RwEngine {}
    }
}
impl Module for RwEngine {}
