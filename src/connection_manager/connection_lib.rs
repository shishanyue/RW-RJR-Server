use std::{collections::HashMap, sync::Arc};

use crate::{connection::shared_connection::SharedConnection, packet::Packet};

use super::By;

pub struct ConnectionLib {
    addr_map: HashMap<String, Arc<SharedConnection>>, //main key
    player_name_map: HashMap<String, String>,
}

impl ConnectionLib {
    pub fn new() -> Self {
        ConnectionLib {
            addr_map: HashMap::new(),
            player_name_map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, shared_con: Arc<SharedConnection>) {
        

        self.addr_map.insert(
            shared_con
                .shared_data
                .connection_info
                .addr
                .upgrade()
                .expect("get con addr error")
                .to_string(),
            shared_con,
        );
    }

    pub async fn send_packet_to_player_by(&self, by: By, packet: Packet) {
        match by {
            By::Addr(addr) => {
                for (con_addr,con) in self.addr_map.iter() {
                    dbg!(*con_addr == addr);
                    if *con_addr == addr{
                        con.send_packet(packet.clone()).await;
                    }
                }
            }
            By::Name(_) => todo!(),
        }
    }

    pub fn remove_by(&mut self, by: By) {
        match by {
            By::Addr(addr) => {
                self.addr_map.remove(&addr);
            }
            By::Name(_) => todo!(),
        }
    }
}
