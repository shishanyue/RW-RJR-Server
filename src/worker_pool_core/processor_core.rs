use std::sync::{atomic::AtomicU32, Arc};

use tokio::{runtime::Runtime, sync::{
    mpsc,
    Mutex, RwLock,
}};

use crate::{
    connection_core::{permission_status::PermissionStatus, Connection, ConnectionChannel},
    packet_core::{Packet, PacketType},
};

use super::new_worker_pool;

pub type ProcesseorData = (Arc<ConnectionChannel>,Packet);


pub async fn processor(mut data: mpsc::Receiver<ProcesseorData>) -> anyhow::Result<()> {
    loop {
        let Some((mut con_channel, mut packet)) = data.recv().await else { continue; };
        println!("Packet:{:?}",packet);
    }
}
