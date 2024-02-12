mod connection_lib;

use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{atomic::AtomicU32, Arc},
};

use tokio::{
    join,
    net::TcpStream,
    runtime::{Builder, Runtime},
    select,
    sync::{mpsc, RwLock},
    task::JoinHandle,
};

use crate::worker_pool_core::receiver_core::receiver;
use crate::worker_pool_core::sender_core::sender;
use crate::{connection_core::SharedConnection, worker_pool_core::processor_core::ProcesseorData};
use crate::{
    connection_core::{Connection, ConnectionAPI},
    server_core::ServerConfig,
    worker_pool_core::{new_worker_pool, processor_core::processor},
};

use self::connection_lib::ConnectionLib;
use crate::core::creat_block_runtime;

#[derive(Debug)]
pub struct ConnectionManager {
    pub new_connection_sender: mpsc::Sender<(TcpStream, SocketAddr)>,
    pub handle_vec: Vec<JoinHandle<()>>,
    runtime:Runtime
}

impl ConnectionManager {
    pub async fn new(server_config: ServerConfig) -> Self {
        //创建RJR的三个WorkerPool,他们各自持有自己的runtime

        let (receiver_block_rt, processor_block_rt, sender_block_rt) = match join!(
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

        let receiver_pool = new_worker_pool(
            1,
            move |r_receiver, _| Box::pin(receiver(r_receiver)),
            receiver_block_rt,
            (),
        )
        .await;

        let sender_pool = new_worker_pool(
            1,
            move |s_receiver, _| Box::pin(sender(s_receiver)),
            sender_block_rt,
            (),
        )
        .await;

        let processor_pool = new_worker_pool(
            1,
            move |p_receiver, _| Box::pin(processor(p_receiver)),
            processor_block_rt,
            (),
        )
        .await;

        //创建ConnectionManager的异步运行时
        let connection_mg_rt = creat_block_runtime(3)
            .await
            .expect("worker manager create error!");

        //接收新的连接
        let (new_connection_sender, new_connection_receiver) =
            mpsc::channel::<(TcpStream, SocketAddr)>(10);

        //将连接添加到Map里
        //SharedConnection只有管道
        //真正的Connection在new时便被move进一个线程
        let (insert_connection_sender, insert_connection_receiver) =
            mpsc::channel::<(SocketAddr, SharedConnection)>(10);

        //处理Packet的channel
        let (processor_sorter_sender, processor_sorter_receiver) =
            mpsc::channel::<ProcesseorData>(10);

        let mut handle_vec = Vec::new();

        handle_vec.push(connection_mg_rt.spawn(async move {
            let mut connection_rt = creat_block_runtime(10)
                .await
                .expect("worker manager create error!");

            let mut receiver_pool = receiver_pool;
            let mut sender_pool = sender_pool;

            let mut new_connection_receiver = new_connection_receiver;
            let processor_sorter_sender = processor_sorter_sender;
            let insert_connection_sender = insert_connection_sender;

            loop {
                let new_receiver = receiver_pool.get_free_worker().await;
                let new_sender = sender_pool.get_free_worker().await;

                let Some((socket, addr)) = new_connection_receiver.recv().await else {
                    continue;
                };

                let new_con = Connection::new(
                    &mut connection_rt,
                    new_receiver,
                    new_sender,
                    processor_sorter_sender.clone(),
                    addr,
                );
                let (read_half, write_half) = socket.into_split();
                new_con
                    .0
                    .receiver
                    .send((new_con.0.clone(), read_half))
                    .await
                    .unwrap();
                new_con
                    .0
                    .sender
                    .send((new_con.0.clone(), write_half))
                    .await
                    .unwrap();
                insert_connection_sender
                    .send((addr, new_con))
                    .await
                    .unwrap();
            }
        }));

        handle_vec.push(connection_mg_rt.spawn(async move {
            let mut processor_pool = processor_pool;
            let mut connection_lib = ConnectionLib::new();
            let mut insert_connection_receiver = insert_connection_receiver;

            loop {
                let Some((addr, con)) = insert_connection_receiver.recv().await else {
                    continue;
                };

                connection_lib.insert(addr, con);
            }
        }));

        handle_vec.push(connection_mg_rt.spawn(async move {}));

        ConnectionManager {
            new_connection_sender: new_connection_sender,
            handle_vec,
            runtime:connection_mg_rt
        }
    }
}
