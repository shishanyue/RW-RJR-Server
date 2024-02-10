pub mod config;
pub mod utils;

use std::net::SocketAddr;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: usize,
    pub thread_number: usize,
    pub enable_web: bool,
}
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UplistApi {}
#[derive(Debug, Serialize, Deserialize, Default)]
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
        write!(
            f,
            "端口:{}\n线程总数:{}",
            self.server.port, self.server.thread_number
        )
    }
}
