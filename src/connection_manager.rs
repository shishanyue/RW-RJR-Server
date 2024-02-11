use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{atomic::AtomicU32, Arc},
};

use tokio::{
    net::TcpStream,
    runtime::Builder,
    sync::{mpsc, RwLock},
};

use crate::{connection_core::Connection, core::GetAndBackChannel};

pub struct ConnectionLib {
    //pub ip_map:HashMap<SocketAddr,>
}

#[derive(Debug)]
pub struct ConnectionManager {
    pub connection_set: HashSet<Connection>,
    pub new_connection_sender: mpsc::Sender<(TcpStream, SocketAddr)>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        let connection_mg_rt = Builder::new_multi_thread()
            .enable_time()
            .worker_threads(1)
            // no timer!
            .build()
            .expect("worker manager create error!");

        let (new_connection_sender, new_connection_receiver) = mpsc::channel(10);

        connection_mg_rt.spawn(async move {
            let new_connection_receiver = new_connection_receiver;
            loop {
                
                match new_connection_receiver.recv().await {
                    Some(_) => {},
                    None => {},
                }
            }
        });

       
    }
}
