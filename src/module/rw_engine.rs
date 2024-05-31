use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use log::{info, warn};
use tokio::sync::RwLock;

use crate::{
    dummy::{shared_dummy::SharedDummy, Dummy, DummyConf, DummyVersion},
    event::{EventType, EVENT_CHANNEL_MULTIPLE},
    module::{Module, MODULE_RUNTIME},
    packet::{common_packet::CommonPacket, PacketType},
};

pub mod image;

pub struct RwEngine {
    dummy_map: Arc<RwLock<HashMap<u32, Arc<SharedDummy>>>>,
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

                        let new_dummy = Dummy::new_shared(
                            DummyConf::new(
                                "RwEngineSystem".to_string(),
                                "com.shishanyue.rwe".to_string(),
                                DummyVersion::Version1_15,
                            ),
                            "127.0.0.1:5124",
                        )
                        .await
                        .unwrap();

                        let packet = CommonPacket::get_preregister_info(
                            &new_dummy.conf.dummy_name,
                            &new_dummy.conf.domain,
                            new_dummy.conf.packet_version,
                            new_dummy.conf.client_version,
                            "",
                        )
                        .await
                        .unwrap();

                        new_dummy.send_packet(packet).await;

                        new_dummy
                            .send_packet(
                                CommonPacket::get_relay_hall_command(&room.shared_data.id)
                                    .await
                                    .unwrap(),
                            )
                            .await;

                        dummy_size.fetch_add(1, Ordering::Relaxed);

                        dummy_map
                            .write()
                            .await
                            .insert(dummy_size.load(Ordering::Relaxed), new_dummy);
                    }
                    EventType::NewPacket(_, _, PacketType::CHAT) => {}
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
