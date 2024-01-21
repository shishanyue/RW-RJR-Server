pub mod config;
pub mod utils;

use std::net::SocketAddr;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: usize,
    pub worker_number: usize,
    pub enable_web: bool,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct UplistApi {}
#[derive(Debug, Serialize, Deserialize)]
pub struct GameConfig {}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct AllConfig {
    pub server: ServerConfig,
    pub uplist: UplistApi,
    pub game: GameConfig,
    pub banlist: Vec<SocketAddr>,
}

impl std::fmt::Display for AllConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "端口:{}", self.server.port)
    }
}



