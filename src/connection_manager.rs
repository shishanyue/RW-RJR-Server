use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{atomic::AtomicU32, Arc},
};

use tokio::{
    join, net::TcpStream, runtime::Builder, select, sync::{mpsc, RwLock}
};

use crate::{
    connection_core::{Connection, ConnectionAPI}, server_core::ServerConfig, worker_pool_core::{new_worker_pool, processor_core::processor}, worker_pool_manager::WorkerPoolManager
};
use crate::worker_pool_core::sender_core::sender;
use crate::worker_pool_core::receiver_core::receiver;
use crate::creat_block_runtime;
pub struct ConnectionLib {
    addr_map: HashMap<SocketAddr, Connection>, //main key
    player_name_map: HashMap<String, SocketAddr>,
}

impl ConnectionLib {
    pub fn new() -> Self {
        ConnectionLib {
            addr_map: HashMap::new(),
            player_name_map: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct ConnectionManager {
    pub new_connection_sender: mpsc::Sender<(TcpStream, SocketAddr)>,
}

impl ConnectionManager {
    pub async fn new(server_config: ServerConfig) -> Self {

        let (receiver_block_rt, processor_block_rt, packet_sender_block_rt) = match join!(
            creat_block_runtime(server_config.thread_number),
            creat_block_runtime(server_config.thread_number),
            creat_block_runtime(server_config.thread_number)
        ) {
            (Ok(receiver_block_rt), Ok(processor_block_rt), Ok(packet_sender_block_rt)) => (
                receiver_block_rt,
                processor_block_rt,
                packet_sender_block_rt,
            ),
            _ => {
                panic!("creating_runtime_error")
            }
        };

        let connection_mg_rt = Builder::new_multi_thread()
            .enable_time()
            .worker_threads(1)
            .build()
            .expect("worker manager create error!");


        let (new_connection_sender, new_connection_receiver) =
            mpsc::channel::<(TcpStream, SocketAddr)>(10);


            let processor_pool = new_worker_pool(
                1,
                move |p_receiver, _| Box::pin(processor(p_receiver)),
                processor_block_rt,
                (),
            )
            .await;


        connection_mg_rt.spawn(async move {
            let connection_lib = ConnectionLib::new();

            


            let mut new_connection_receiver = new_connection_receiver;
            loop {
                select! {
                    Some((socket, addr)) = new_connection_receiver.recv() => {
                        let (read_half, write_half) = socket.into_split();
                        //worker_pool_mg.receiver.send(read_half).await.expect("发送OwnedReadHalf错误");
                    }

                }
            }
        });

        connection_mg_rt.spawn(async move {});

        ConnectionManager {
            new_connection_sender: new_connection_sender,
        }
    }
}
