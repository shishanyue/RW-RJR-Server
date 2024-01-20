pub mod config;
pub mod utils;

use std::net::SocketAddr;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub ip: String,
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
