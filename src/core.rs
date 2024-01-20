use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use dashmap::DashMap;
use rand::{Rng, SeedableRng};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    join,
    runtime::{Builder, Runtime},
    sync::{
        mpsc::{self},
        oneshot, watch, Mutex, RwLock,
    },
};

use crate::{
    connection_core::{
        player_net_api::{read_stream_bytes, CustomRelayData, PlayerAllInfo},
        Connection,
    },
    packet_core::{Packet, PacketType},
    worker_pool_core::{
        new_worker_pool,
        processor_core::init_processor_sorter,
        receiver_core::{receiver, ReceiverData},
        sender_core::{sender, SenderData},
    },
};
pub type BlockRuntime = Arc<Mutex<Runtime>>;

#[derive(Debug, Clone, Copy)]
pub enum ServerCommand {
    Disconnect,
    None,
}

pub type WorkersSender = (mpsc::Sender<ReceiverData>, mpsc::Sender<SenderData>);

#[derive(Debug)]
pub struct ConnectionManage {
    pub connections: Arc<DashMap<SocketAddr, Arc<RwLock<Connection>>>>,
    pub get_worker_sender: mpsc::Sender<oneshot::Sender<WorkersSender>>,
    pub back_worker_sender: mpsc::Sender<WorkersSender>,
    pub disconnect_del_con_sender: mpsc::Sender<SocketAddr>,
}

impl ConnectionManage {
    pub async fn new(
        sender_rt: BlockRuntime,
        receiver_rt: BlockRuntime,
        processor_rt: BlockRuntime,
    ) -> Arc<RwLock<Self>> {
        let processor_sorter_sender = tokio::spawn(init_processor_sorter(processor_rt.clone()))
            .await
            .unwrap();

        match join!(
            new_worker_pool(
                100,
                move |s_receiver, _| Box::pin(sender(s_receiver)),
                sender_rt,
                ()
            ),
            new_worker_pool(
                100,
                move |r_receiver, sorter_sender| Box::pin(receiver(r_receiver, sorter_sender)),
                receiver_rt,
                processor_sorter_sender
            ),
        ) {
            (sender_pool, receiver_pool) => {
                //let sender_semaphore = Arc::new(Semaphore::new(3));
                let (get_worker_sender, get_worker_receiver) = mpsc::channel(10);
                let (back_worker_sender, back_worker_receiver) = mpsc::channel(10);
                tokio::spawn(async move {
                    let mut sender_pool = sender_pool;
                    let mut receiver_pool = receiver_pool;

                    let mut get_worker_receiver: mpsc::Receiver<oneshot::Sender<WorkersSender>> =
                        get_worker_receiver;

                    let mut back_worker_receiver: mpsc::Receiver<WorkersSender> =
                        back_worker_receiver;

                    loop {
                        tokio::select! {
                            get_worker_receiver = get_worker_receiver.recv() => {
                                match get_worker_receiver{
                                    Some(worker_sender) => {
                                        let receiver = receiver_pool.get_free_worker().await;

                                        let packet_sender = sender_pool.get_free_worker().await;
                                        worker_sender.send((receiver, packet_sender)).unwrap();
                                    }
                                    None => todo!(),
                                }
                            }
                            back_worker_receiver = back_worker_receiver.recv() => {
                                match back_worker_receiver{
                                    Some((receiver,packet_sender)) => {
                                        receiver_pool.push_free_worker(receiver).await;
                                        sender_pool.push_free_worker(packet_sender).await;
                                    }
                                    None => todo!(),
                                }
                            }
                        }
                    }
                });
                let (disconnect_del_con_sender, disconnect_del_con_receiver) = mpsc::channel(10);
                let connection_mg = Arc::new(RwLock::new(ConnectionManage {
                    connections: Arc::new(DashMap::new()),
                    get_worker_sender,
                    back_worker_sender,
                    disconnect_del_con_sender,
                }));

                let connection_mg_remove = connection_mg.clone();
                tokio::spawn(async move {
                    let connection_mg_remove = connection_mg_remove;
                    let mut disconnect_del_con_receiver = disconnect_del_con_receiver;
                    loop {
                        match disconnect_del_con_receiver.recv().await {
                            Some(ip) => {
                                connection_mg_remove.write().await.connections.remove(&ip).unwrap();
                            },
                            None => todo!(),
                        }
                    }
                });

                connection_mg
            }
        }

        //static SENDER_POOL = new_worker_pool(10, worker_fn, sender_rt);
    }

    pub async fn prepare_new_con(
        &mut self,
        relay_mg: Arc<Mutex<RelayManage>>,
    ) -> Arc<RwLock<Connection>> {
        let (command_sender, command_receiver) = watch::channel(ServerCommand::None);

        let get_worker = oneshot::channel();

        self.get_worker_sender.send(get_worker.0).await.unwrap();

        let Ok((receiver, sender)) = get_worker.1.await else {
            todo!()
        };

        Connection::new(
            command_sender,
            command_receiver,
            receiver,
            sender,
            relay_mg,
            self.back_worker_sender.clone(),
            self.disconnect_del_con_sender.clone(),
        )
        .await
    }

    pub async fn insert_con(&mut self, ip: SocketAddr, con: Arc<RwLock<Connection>>) {
        self.connections.insert(ip, con);
    }
}

pub async fn creat_block_runtime() -> anyhow::Result<BlockRuntime> {
    Ok(Arc::new(Mutex::new(
        Builder::new_multi_thread()
            .enable_time()
            .worker_threads(1)
            // no timer!
            .build()
            .unwrap(),
    )))
}

pub type RelayConData = (
    (Arc<RwLock<Connection>>, mpsc::Sender<Packet>),
    oneshot::Sender<u32>,
);

#[derive(Debug)]
pub struct RelayRoom {
    pub player_map: DashMap<u32, (Arc<RwLock<Connection>>, mpsc::Sender<Packet>)>,
    pub admin: Arc<RwLock<Connection>>,
    pub admin_packet_sender: mpsc::Sender<Packet>,
    pub site: AtomicU32,
    pub id: String,
    pub mods: bool,
    pub uplist: bool,
    pub beta_game_version: bool,
    pub version: u32,
    pub max_player: i32,
    //pub relay_broadcast_sender: broadcast::Sender<Packet>
}

#[derive(Debug, Clone)]
pub struct RelayRoomData {
    pub relay_room: Arc<RwLock<RelayRoom>>,
    pub id: String,
    pub relay_add_con_sender: mpsc::Sender<RelayConData>,
    pub relay_group_packet_sender: mpsc::Sender<Packet>,
}

impl RelayRoom {
    pub async fn new(
        admin: Arc<RwLock<Connection>>,
        id: String,
        mods: bool,
        uplist: bool,
        beta_game_version: bool,
        version: u32,
        max_player: i32,
        admin_packet_sender: mpsc::Sender<Packet>,
    ) -> Arc<RwLock<Self>> {
        let player_map = DashMap::new();
        let site = AtomicU32::new(2);
        player_map.insert(1, (admin.clone(), admin_packet_sender.clone()));
        Arc::new(RwLock::new(RelayRoom {
            player_map,
            admin,
            admin_packet_sender,
            site,
            id,
            mods,
            uplist,
            beta_game_version,
            version,
            max_player,
        }))
    }
    pub async fn add_connect(&mut self, con_data: RelayConData) {
        let pos = self.site.fetch_add(1, Ordering::SeqCst);

        self.player_map.insert(pos, con_data.0);
        con_data.1.send(pos).unwrap();
    }

    pub fn get_con_sender(&mut self, index: u32) -> Option<mpsc::Sender<Packet>> {
        if self.player_map.contains_key(&index) {
            let con = self.player_map.get(&index).unwrap();
            Some(con.1.clone())
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct RelayManage {
    pub room_map: DashMap<String, RelayRoomData>,
    pub relay_rt: Runtime,
    pub id_rand: rand::rngs::StdRng,
}

impl RelayManage {
    pub async fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(RelayManage {
            room_map: DashMap::new(),
            relay_rt: Builder::new_multi_thread()
                .enable_time()
                .worker_threads(1)
                // no timer!
                .build()
                .unwrap(),
            id_rand: SeedableRng::from_entropy(),
        }))
    }

    pub fn get_relay(&mut self, id: &str) -> Option<RelayRoomData> {
        if self.room_map.contains_key(&id.to_string()) {
            let room = self.room_map.get(&id.to_string()).unwrap();
            Some(room.clone())
        } else {
            None
        }
        //*self.room_map.try_get(&id.to_string()).unwrap()
    }

    pub async fn new_relay_id(
        &mut self,
        con: Arc<RwLock<Connection>>,
        _id: Option<String>,
        mods: bool,
        uplist: bool,
        custom: CustomRelayData,
        info: PlayerAllInfo,
        admin_packet_sender: mpsc::Sender<Packet>,
    ) -> anyhow::Result<RelayRoomData> {
        let max_player = if custom.max_player_size == -1 {
            10
        } else {
            custom.max_player_size
        };

        let new_relay = self
            .new_relay(
                con,
                None,
                &info.0.read().await.player_name,
                mods,
                uplist,
                info.1.read().await.is_beta_version,
                info.1.read().await.client_version,
                max_player,
                admin_packet_sender,
            )
            .await;

        self.room_map
            .insert(new_relay.id.clone(), new_relay.clone());
        //self.room_map.insert(key, value)
        Ok(new_relay)
    }

    pub async fn new_relay(
        &mut self,
        con: Arc<RwLock<Connection>>,
        id: Option<String>,
        _player_name: &str,
        is_mod: bool,
        uplist: bool,
        beta_game_version: bool,
        version: u32,
        max_player: i32,
        admin_packet_sender: mpsc::Sender<Packet>,
    ) -> RelayRoomData {
        let (relay_add_con_sender, mut relay_add_con_receiver) = mpsc::channel(10);
        let (relay_group_packet_sender, mut relay_group_packet_receiver) = mpsc::channel(10);

        let id = if let Some(id) = id {
            id
        } else {
            self.id_rand.gen_range(100..9999).to_string()
        };

        let relay_room = RelayRoom::new(
            con,
            id.clone(),
            is_mod,
            uplist,
            beta_game_version,
            version,
            max_player,
            admin_packet_sender,
        )
        .await;

        let room = relay_room.clone();

        self.relay_rt.spawn(async move {
            let room = room.clone();
            loop {
                tokio::select! {
                    add_con_data = relay_add_con_receiver.recv() => {
                        match add_con_data{
                            Some(add_con) => {

                                room.write().await.add_connect(add_con).await;
                                //add_con.1.send(pos);
                            },
                            None => todo!(),
                            }

                        },
                    packet = relay_group_packet_receiver.recv() => {
                    match packet{
                            Some(packet) => {
                                relay_packet_sender(room.clone(),packet).await;
                            },
                            None => todo!(),
                            }
                    },
                }
            }
        });

        RelayRoomData {
            relay_room,
            id,
            relay_add_con_sender,
            relay_group_packet_sender,
        }
    }
}

pub async fn relay_packet_sender(room: Arc<RwLock<RelayRoom>>, mut packet: Packet) {
    let target = packet.packet_buffer.read_u32().await.unwrap();
    let packet_type = packet.packet_buffer.read_u32().await.unwrap();

    let bytes = read_stream_bytes(&mut packet).await;

    if packet_type == PacketType::DISCONNECT as u32 {
        return;
    }

    let mut send_packet = Packet::new_by_num(packet_type).await;

    send_packet.packet_buffer.write_all(&bytes).await.unwrap();

    let target_con_sender = room.write().await.get_con_sender(target).unwrap();

    if packet_type == PacketType::KICK as u32{

    }
    if packet_type == PacketType::START_GAME as u32{
        
    }

    target_con_sender.send(send_packet).await.unwrap();
}
