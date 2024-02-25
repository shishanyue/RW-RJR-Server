use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use crate::connection::shared_connection::SharedConnection;

pub struct ConnectionLib {
    addr_map: HashMap<Arc<SocketAddr>, Arc<SharedConnection>>, //main key
    // TODO: use it
    _player_name_map: HashMap<String, SocketAddr>,
}

impl ConnectionLib {
    pub fn new() -> Self {
        ConnectionLib {
            addr_map: HashMap::new(),
            _player_name_map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, shared_con: Arc<SharedConnection>) {
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

    pub fn remove_by_addr(&mut self, addr: SocketAddr) {
        self.addr_map.remove(&addr);
    }
}
