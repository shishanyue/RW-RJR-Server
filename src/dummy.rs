pub mod shared_dummy;

use std::sync::{Arc};

use tokio::{
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    runtime::{Builder, Runtime},
};

use crate::{
    error::DummyError,
    packet::{Packet},
};

use self::shared_dummy::SharedDummy;

lazy_static! {
    static ref DUMMY_RUNTIME:Runtime = Builder::new_multi_thread()
    .enable_time()
    .worker_threads(10)
    .enable_io()
    // no timer!
    .build()
    .expect("creat block runtime error");

}

pub enum DummyAPI {
    SendPacket(Packet),
    SendPackets(Vec<Packet>),
}

#[derive(Debug,Clone, Copy)]
pub enum DummyVersion {
    Version1_15,
    Version1_16,
}


#[derive(Debug,Clone)]
pub struct DummyConf {
    pub dummy_name: String,
    pub domain: String,
    pub version: DummyVersion,
    pub packet_version: u32,
    pub client_version: u32,
}
pub struct Dummy {
    pub conf: DummyConf,
    pub is_connect: bool,
    pub read_half: OwnedReadHalf,
    pub write_half: OwnedWriteHalf,
}

impl DummyConf {
    pub fn new(dummy_name: String, domain: String, version: DummyVersion) -> Self {
        match version {
            DummyVersion::Version1_15 => {
                Self{
                    dummy_name,
                    domain,
                    version,
                    packet_version: 4,
                    client_version: 172,
                }
            },
            DummyVersion::Version1_16 => todo!(),
        }
    }
}

impl Dummy {
    pub async fn new_shared(
        dummy_conf: DummyConf,
        addr: &str,
    ) -> Result<Arc<SharedDummy>, DummyError> {
        match dummy_conf.version {
            DummyVersion::Version1_15 => {
                let stream = TcpStream::connect(&addr).await?;

                let (read_half, write_half) = stream.into_split();

                let dummy = Self {
                    conf: dummy_conf,
                    is_connect: false,
                    read_half,
                    write_half,
                };
                Ok(SharedDummy::new(dummy))
            }
            DummyVersion::Version1_16 => todo!(),
        }
    }

    pub async fn connect_to_relay(&mut self, addr: &str) {}

    pub async fn join_to_room(&self, id: &str) {}
}

/*

async fn process_api(api: DummyAPI, write_half: &mut OwnedWriteHalf) {
    match api {
        DummyAPI::SendPacket(mut packet) => {
            packet.prepare().await;
            println!("{:?}", packet);
            write_half
                .write_all(&packet.packet_buffer.into_inner())
                .await
                .expect("dummy send packet error");
        }
        DummyAPI::SendPackets(packets) => {
            for mut packet in packets {
                packet.prepare().await;
                println!("{:?}", packet);
                write_half
                    .write_all(&packet.packet_buffer.into_inner())
                    .await
                    .expect("dummy send packet error");
            }
        }
    }
}


*/
