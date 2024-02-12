use std::sync::{atomic::AtomicU32, Arc};

use tokio::{runtime::Runtime, sync::{
    mpsc,
    Mutex, RwLock,
}};

use crate::{
    connection_core::{permission_status::PermissionStatus, Connection},
    packet_core::{Packet, PacketType},
};

use super::new_worker_pool;

pub type ProcesseorData = ();


pub async fn processor(mut data: mpsc::Receiver<ProcesseorData>) -> anyhow::Result<()> {
}
