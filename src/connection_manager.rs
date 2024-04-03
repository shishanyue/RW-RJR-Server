mod connection_lib;

use std::{net::SocketAddr, sync::Arc};

use async_channel::Sender;
use log::info;
use tokio::{join, net::TcpStream, runtime::Runtime, sync::mpsc, task::JoinHandle};

use crate::connection::ConnectionLibAPI;
use crate::packet::{self, Packet};
use crate::relay_manager::SharedRelayManager;
use crate::worker_pool::sender::sender;
use crate::{worker_pool::receiver::receiver};
use crate::{
    connection::Connection,
    server::ServerConfig,
    worker_pool::{new_worker_pool, processor::processor},
};

use self::connection_lib::ConnectionLib;
use crate::core::creat_block_runtime;

type NewConnectionData = (TcpStream, SocketAddr);

#[derive(Debug)]
pub struct ConnectionManager {
    new_con_tx: Option<mpsc::Sender<NewConnectionData>>,
    handle_vec: Vec<JoinHandle<()>>,
    runtime: Option<Arc<Runtime>>,
    connection_runtime: Option<Runtime>,
    shared_relay_mg: Arc<SharedRelayManager>,
    pub con_lib_api_tx:Option<mpsc::Sender<ConnectionLibAPI>>
}

pub enum By {
    Addr(String),
    Name(String)
}

impl ConnectionManager {
    async fn init_worker_pool(
        &mut self,
        receiver_thread_number: usize,
        processor_thread_number: usize,
        sender_thread_number: usize,
    ) {
        //创建RJR的三个WorkerPool,他们各自持有自己的runtime

        let (receiver_block_rt, processor_block_rt, sender_block_rt) = match join!(
            creat_block_runtime(receiver_thread_number),
            creat_block_runtime(processor_thread_number),
            creat_block_runtime(sender_thread_number)
        ) {
            (Ok(receiver_block_rt), Ok(processor_block_rt), Ok(packet_sender_block_rt)) => (
                receiver_block_rt,
                processor_block_rt,
                packet_sender_block_rt,
            ),
            _ => {
                panic!("创建WorkerPool异步运行时失败")
            }
        };

        let receiver_pool = new_worker_pool(
            1,
            move |r_receiver, _| Box::pin(receiver(r_receiver)),
            receiver_block_rt,
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

        let sender_pool = new_worker_pool(
            1,
            move |s_receiver, _| Box::pin(sender(s_receiver)),
            sender_block_rt,
            (),
        )
        .await;

        info!("Receiver注册成功");
        info!("Processor注册成功");
        info!("Sender注册成功");

        //接收新的Connection的channel
        let (new_con_tx, new_con_rx) = mpsc::channel(10);

        //将连接添加到Map里
        //SharedConnection只有管道
        //真正的Connection在new时便被move进一个线程

        //初始化Connection Lib
        let con_lib_api_tx = self.init_con_lib().await;

        self.con_lib_api_tx = Some(con_lib_api_tx.clone());
        
        //处理Packet的channel
        //因为processor与receiver和sender的进程不同
        let (processor_sorter_tx, processor_sorter_rx) = mpsc::channel(10);

        //处理新连接的进程
        let runtime = self.runtime.clone().unwrap();
        let shared_relay_mg = self.shared_relay_mg.clone();

        self.handle_vec
            .push(self.runtime.as_ref().unwrap().spawn(async move {
                //移入receiver和sender的池
                let mut receiver_pool = receiver_pool;
                let mut sender_pool = sender_pool;

                let (back_worker_tx, mut back_worker_rx) = mpsc::channel(10);

                //移入接受新连接的channel
                let mut new_con_rx = new_con_rx;
                //用于发送Packet到另一个进程的channel
                let processor_sorter_tx = processor_sorter_tx;
                //用于发送新连接到另一个进程的channel
                let con_lib_api_tx = con_lib_api_tx;

                //
                let shared_relay_mg = shared_relay_mg;

                let runtime = runtime;
                loop {
                    //提前获取新连接需要的sender和receiver
                    let new_receiver = receiver_pool.get_free_worker().await;
                    let new_sender = sender_pool.get_free_worker().await;

                    tokio::select! {
                        
                        //接受到新连接
                        Some((socket, addr)) = new_con_rx.recv() => {
                            //建立连接并返回SharedConnection
                            let new_shared_con = Connection::new_shared(
                                &runtime,
                                new_receiver.clone(),
                                new_sender.clone(),
                                processor_sorter_tx.clone(),
                                addr,
                                shared_relay_mg.clone(),
                                back_worker_tx.clone(),
                                con_lib_api_tx.clone()
                            );

                            //receiver_pool.push_free_worker(new_receiver).await;
                            //sender_pool.push_free_worker(new_sender).await;
                            //绑定socket到receiver和sender上
                            new_shared_con.bind(new_shared_con.clone(), socket).await;

                            //将新连接存储到Lib里
                            con_lib_api_tx.send(ConnectionLibAPI::InsertConnection(new_shared_con)).await.unwrap();
                        }
                    }
                }
            }));

        self.handle_vec
            .push(self.runtime.as_ref().unwrap().spawn(async move {
                let mut processor_pool = processor_pool;
                let mut processor_sorter_rx = processor_sorter_rx;

                loop {
                    let processor = processor_pool.get_free_worker().await;

                    let Some((shared_con, packet)) = processor_sorter_rx.recv().await else {
                        continue;
                    };

                    processor
                        .send((shared_con, packet))
                        .await
                        .expect("send packet to processor error");

                    processor_pool.push_free_worker(processor).await;
                }
            }));

        self.new_con_tx = Some(new_con_tx);
    }

    async fn init_con_lib(&mut self) -> mpsc::Sender<ConnectionLibAPI> {
        let (con_lib_api_tx, con_lib_api_rx) = mpsc::channel(10);
        self.handle_vec
            .push(self.runtime.as_ref().unwrap().spawn(async move {
                let mut connection_lib = ConnectionLib::new();
                let mut con_lib_api_rx = con_lib_api_rx;
                loop {
                    match con_lib_api_rx
                        .recv()
                        .await
                        .expect("Connection API recv error")
                    {
                        ConnectionLibAPI::RemoveConnectionByAddr(addr) => {
                            connection_lib.remove_by_addr(addr)
                        },
                        ConnectionLibAPI::InsertConnection(shared_con) => {
                            connection_lib.insert(shared_con)
                        }
                        ConnectionLibAPI::SendPacketToPlayerByUUID() => todo!(),
                        ConnectionLibAPI::SendPacketToPlayerByName(name, packet) => todo!(),
                        ConnectionLibAPI::SendPacketToPlayerByAddr(addr, packet) => {
                            connection_lib.send_packet_to_player_by_addr(addr, packet).await
                        },
                    }
                }
            }));
        con_lib_api_tx
    }

    pub async fn send_packet_to_player_by(&self,by:By,packet:Packet){
        match by {
            By::Addr(addr) => self.con_lib_api_tx.as_ref().unwrap().send(ConnectionLibAPI::SendPacketToPlayerByAddr(addr,packet)).await.expect(""),
            By::Name(_) => todo!(),
        }
    }

    async fn ne_new(
        con_mg_thread_number: usize,
        con_thread_number: usize,
        shared_relay_mg: Arc<SharedRelayManager>,
    ) -> Self {
        ConnectionManager {
            runtime: Some(Arc::new(
                creat_block_runtime(con_mg_thread_number)
                    .await
                    .expect("connection manager runtime create error!"),
            )),
            connection_runtime: Some(
                creat_block_runtime(con_thread_number)
                    .await
                    .expect("connection runtime create error!"),
            ),
            new_con_tx: None,
            handle_vec: Vec::new(),
            shared_relay_mg,
            con_lib_api_tx:None
        }
    }

    pub async fn new(
        server_config: ServerConfig,
        shared_relay_mg: Arc<SharedRelayManager>,
    ) -> Self {
        let mut connection_mg = ConnectionManager::ne_new(3, 10, shared_relay_mg).await;

        connection_mg
            .init_worker_pool(
                server_config.thread_number,
                server_config.thread_number,
                server_config.thread_number,
            )
            .await;

        connection_mg
    }

    pub async fn new_connection(&self, con_data: NewConnectionData) {
        self.new_con_tx
            .as_ref()
            .unwrap()
            .send(con_data)
            .await
            .expect("send new con error");
    }
}
