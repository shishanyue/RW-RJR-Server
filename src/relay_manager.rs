pub mod relay;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use rand::{Rng, SeedableRng};
use tokio::{
    runtime::Runtime,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use crate::{
    connection::{player_net_api::CustomRelayData, shared_connection::SharedConnection},
    core::creat_block_runtime,
    NOW,
};

use self::relay::SharedRelayRoom;

#[derive(Debug)]
pub enum RelayManagerAPI {
    GetRelay(String, oneshot::Sender<Option<Arc<SharedRelayRoom>>>),
    InsertNewRelay(Arc<SharedRelayRoom>),
}

#[derive(Debug)]
struct RelayManager {
    pub room_map: HashMap<String, Arc<SharedRelayRoom>>,
}

#[derive(Debug)]
pub struct SharedRelayManager {
    relay_mg_api_tx: mpsc::Sender<RelayManagerAPI>,
    handle: JoinHandle<()>,
    pub relay_rt: Arc<Runtime>,
    pub id_rand: Arc<RwLock<rand::rngs::StdRng>>,
}

impl SharedRelayManager {
    pub async fn new(relay_mg_thread_number: usize) -> Arc<SharedRelayManager> {
        let (relay_mg_api_tx, relay_mg_api_rx) = mpsc::channel(10);

        let relay_mg = RelayManager::new().await;

        let runtime = Arc::new(
            creat_block_runtime(relay_mg_thread_number)
                .await
                .expect("create relay manager runtime error"),
        );

        let handle = runtime.spawn(async move {
            let mut relay_mg = relay_mg;
            let mut relay_mg_api_rx = relay_mg_api_rx;

            loop {
                match relay_mg_api_rx
                    .recv()
                    .await
                    .expect("recv relay mg api error")
                {
                    RelayManagerAPI::GetRelay(id, relay_index_tx) => {
                        let id = format!("S{}", id);

                        let mut shared_relay = None;
                        for (relay_id, shared) in relay_mg.room_map.iter() {
                            if relay_id == &id {
                                shared_relay = Some(shared.clone().to_owned());
                            }
                        }
                        relay_index_tx
                            .send(shared_relay)
                            .expect("send relay index error");
                    }
                    RelayManagerAPI::InsertNewRelay(new_shared) => {
                        relay_mg
                            .room_map
                            .insert(new_shared.shared_data.id.clone(), new_shared);
                    }
                }
            }
        });

        Arc::new(Self {
            relay_mg_api_tx,
            handle,
            relay_rt: runtime,
            id_rand: Arc::new(RwLock::new(SeedableRng::seed_from_u64(
                NOW.elapsed().as_secs(),
            ))),
        })
    }

    pub async fn get_relay(&self, id: &str) -> Option<Arc<SharedRelayRoom>> {
        let (shared_relay_tx, shared_relay_rx) = oneshot::channel();
        self.relay_mg_api_tx
            .send(RelayManagerAPI::GetRelay(id.to_string(), shared_relay_tx))
            .await
            .expect("send relay mg api tx error");
        shared_relay_rx.await.expect("recv relay index error")
    }

    pub async fn new_relay_id(
        &self,
        admin: Arc<SharedConnection>,
        id: Option<String>,
        mut custom: CustomRelayData,
    ) -> Arc<SharedRelayRoom> {
        if custom.max_player_size == -1 {
            custom.max_player_size = 10;
        }

        let id = if let Some(id) = id {
            id
        } else {
            loop {
                let tmp_id = self
                    .id_rand
                    .write()
                    .expect("get rand id the write lock error")
                    .gen_range(100..9999)
                    .to_string();
                if (self.get_relay(&tmp_id).await).is_none() {
                    break format!("S{}", tmp_id);
                }
            }
        };

        let shared_relay_room =
            SharedRelayRoom::new_shared(&self.relay_rt, Arc::downgrade(&admin), id, custom).await;

        self.relay_mg_api_tx
            .send(RelayManagerAPI::InsertNewRelay(shared_relay_room.clone()))
            .await
            .expect("send relay mg api tx error");

        shared_relay_room
    }
}

impl RelayManager {
    pub async fn new() -> Self {
        Self {
            room_map: HashMap::new(),
        }
    }
}
