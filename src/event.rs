use std::sync::Arc;

use log::info;
use tokio::sync::{broadcast, mpsc};

use crate::relay_manager::relay::SharedRelayRoom;

#[derive(Clone)]
pub struct Event {
    pub event_name: String,
    pub event_type: EventType,
}
impl Event {
    pub fn new(event_name: &str, event_type: EventType) -> Self {
        Self {
            event_name:event_name.to_string(),
            event_type,
        }
    }
}
#[derive(Clone)]
pub enum EventType {
    NewRoomAndHostOk(Arc<SharedRelayRoom>),
}

lazy_static! {
    pub static ref EVENT_CHANNEL: (async_channel::Sender<Event>, async_channel::Receiver<Event>) =
        async_channel::unbounded();
    pub static ref EVENT_CHANNEL_MULTIPLE: (broadcast::Sender<Event>, broadcast::Receiver<Event>) =
        broadcast::channel(100);
}

pub fn init_event_system() -> anyhow::Result<()> {
    info!("事件系统正在初始化");
    
    tokio::spawn(async move {
        match EVENT_CHANNEL.1.recv().await {
            Ok(event) => EVENT_CHANNEL_MULTIPLE.0.send(event),
            Err(_) => todo!(),
        }
    });
    Ok(())
}
