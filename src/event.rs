use std::sync::Arc;

use log::info;
use tokio::sync::{broadcast, mpsc};

use crate::{
    connection::shared_connection::SharedConnection,
    packet::{Packet, PacketType},
    relay_manager::relay::SharedRelayRoom,
};

#[derive(Clone, Debug)]
pub struct Event {
    pub event_name: String,
    pub event_type: EventType,
}
impl Event {
    pub fn new(event_name: &str, event_type: EventType) -> Self {
        Self {
            event_name: event_name.to_string(),
            event_type,
        }
    }
}
#[derive(Clone, Debug)]
pub enum EventType {
    NewRoomAndHostOk(Arc<SharedRelayRoom>),
    NewPacket(Arc<SharedConnection>, Packet, PacketType),
}

lazy_static! {
    pub static ref EVENT_CHANNEL: Arc<(async_channel::Sender<Event>, async_channel::Receiver<Event>)> =
        Arc::new(async_channel::unbounded());
    pub static ref EVENT_CHANNEL_MULTIPLE: Arc<(broadcast::Sender<Event>, broadcast::Receiver<Event>)> =
        Arc::new(broadcast::channel(100));
}

pub fn init_event_system() -> anyhow::Result<()> {
    info!("事件系统正在初始化");

    tokio::spawn(async move {
        let multiple_sender = EVENT_CHANNEL_MULTIPLE.0.clone();
        loop {
            match EVENT_CHANNEL.1.recv().await {
                Ok(event) => {
                    multiple_sender.send(event).unwrap();
                }
                Err(_) => todo!(),
            }
        }
    });
    Ok(())
}
