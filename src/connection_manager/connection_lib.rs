use std::{
    collections::{hash_map::Iter, HashMap},
    net::SocketAddr,
    sync::Arc,
};

use crate::{connection::shared_connection::SharedConnection, packet::Packet};

pub struct ConnectionLib {
    addr_map: HashMap<Arc<SocketAddr>, Arc<SharedConnection>>, //main key
    player_name_map: HashMap<String, SocketAddr>,
}

impl ConnectionLib {
    pub fn new() -> Self {
        ConnectionLib {
            addr_map: HashMap::new(),
            player_name_map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, shared_con: Arc<SharedConnection>) {
        println!(
            "addr:{}",
            shared_con
                .shared_data
                .connection_info
                .addr
                .upgrade()
                .unwrap()
        );

        self.addr_map.insert(
            shared_con
                .shared_data
                .connection_info
                .addr
                .upgrade()
                .unwrap(),
            shared_con,
        );
    }

    pub fn get_iter(&self) -> Iter<'_, Arc<SocketAddr>, Arc<SharedConnection>> {
        self.addr_map.iter()
    }

    pub fn send_packet_to_player_by_name(&self, name: String, packet: Packet) {
        todo!()
    }

    pub async fn send_packet_to_player_by_addr(&self, addr: String, packet: Packet) {
        for (con_addr, con) in self.addr_map.iter() {
            println!("{}", &con_addr.to_string());
            if con_addr.to_string() == addr {
                println!("superPacket:{:?}", packet);
                con.send_packet(packet.clone()).await
            }
        }
    }

    pub fn remove_by_addr(&mut self, addr: SocketAddr) {
        self.addr_map.remove(&addr);
    }
}
