use std::{collections::HashMap, net::SocketAddr};

use crate::connection_core::{Connection, SharedConnection};

pub struct ConnectionLib {
    addr_map: HashMap<SocketAddr, SharedConnection>, //main key
    player_name_map: HashMap<String, SocketAddr>,
}

impl ConnectionLib {
    pub fn new() -> Self {
        ConnectionLib {
            addr_map: HashMap::new(),
            player_name_map: HashMap::new(),
        }
    }

    pub fn insert(&mut self,addr:SocketAddr,con:SharedConnection){
        self.addr_map.insert(addr, con);
    }
}