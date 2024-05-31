use std::sync::Arc;

use tokio::{io::AsyncWriteExt, task::JoinHandle};

use crate::packet::Packet;

use super::{Dummy, DummyAPI, DummyConf, DUMMY_RUNTIME};

pub struct SharedDummy {
    basic_dummy_api_tx: async_channel::Sender<DummyAPI>,
    handle: JoinHandle<()>,
    pub conf: DummyConf,
}

impl SharedDummy {
    pub fn new(dummy: Dummy) -> Arc<Self> {
        let (basic_dummy_api_tx, basic_dummy_api_rx) = async_channel::bounded(10);
        let conf = dummy.conf.clone();
        let handle = DUMMY_RUNTIME.spawn(async move {
            let mut dummy = dummy;
            let basic_dummy_api_rx = basic_dummy_api_rx;
            loop {
                if let Ok(api_type) = basic_dummy_api_rx.recv().await {
                    match api_type {
                        DummyAPI::SendPacket(mut packet) => {
                            packet.prepare().await;
                            println!("{:?}", packet);
                            dummy
                                .write_half
                                .write_all(&packet.packet_buffer.into_inner())
                                .await
                                .expect("dummy send packet error");
                        }
                        DummyAPI::SendPackets(packets) => {
                            for mut packet in packets {
                                packet.prepare().await;
                                println!("{:?}", packet);
                                dummy
                                    .write_half
                                    .write_all(&packet.packet_buffer.into_inner())
                                    .await
                                    .expect("dummy send packet error");
                            }
                        }
                    }
                }
            }
        });
        Arc::new(SharedDummy {
            basic_dummy_api_tx,
            handle,
            conf
        })
    }

    pub async fn send_packet(&self, packet: Packet) {
        self.basic_dummy_api_tx
            .send(DummyAPI::SendPacket(packet))
            .await
            .unwrap()
    }
}
