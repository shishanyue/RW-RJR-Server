use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

use log::{info, warn};
use tokio::sync::RwLock;

use crate::{
    dummy::DummyVersion,
    event::{EventType, EVENT_CHANNEL_MULTIPLE},
    module::{easy_dummy::EasyDummy, Module, MODULE_RUNTIME}, packet::PacketType,
};

pub mod image;

pub struct RwEngine {
    dummy_map: Arc<RwLock<HashMap<u32, EasyDummy>>>,
    dummy_size: Arc<AtomicU32>,
}
impl RwEngine {
    pub fn new() -> Self {
        let dummy_size = Arc::new(AtomicU32::new(0));
        let dummy_map = Arc::new(RwLock::new(HashMap::new()));
        let dummy_size_move = dummy_size.clone();
        let dummy_map_move = dummy_map.clone();
        info!("RwEnige 正在启动!");
        MODULE_RUNTIME.spawn(async move {
            let dummy_size = dummy_size_move;
            let dummy_map = dummy_map_move;

            let mut event_reader = EVENT_CHANNEL_MULTIPLE.1.resubscribe();
            loop {
                let Ok(event) = event_reader.recv().await else {
                    warn!("event recv error");
                    continue;
                };
                match event.event_type {
                    EventType::NewRoomAndHostOk(room) => {
                        info!("ID为{}的房间已创建,正在添加假人.", room.shared_data.id);

                        let new_dummy = EasyDummy::new(
                            dummy_size.load(Ordering::Relaxed),
                            "RwEngineSystem",
                            "com.shishanyue.rwe",
                            DummyVersion::Version1_15,
                        )
                        .await;

                        dummy_size.fetch_add(1, Ordering::Relaxed);

                        let is_connect = new_dummy.get_is_connect_arc().await;

                        new_dummy
                            .connect_to_relay("127.0.0.1:5123")
                            .await
                            .expect("connet to relay error");
                        
                        loop {
                            if is_connect.load(Ordering::SeqCst){
                                new_dummy.join_to_room(&room.shared_data.id)
                                .await
                                .expect("join to room error");
                            break;
                            }
                        }
                

                        dummy_map
                            .write()
                            .await
                            .insert(dummy_size.load(Ordering::Relaxed), new_dummy);
                    }
                    EventType::NewPacket(_, _,PacketType::CHAT) => {
                        
                    },
                    _ => {}
                }
            }
        });
        RwEngine {
            dummy_size,
            dummy_map,
        }
    }
}
impl Module for RwEngine {}
