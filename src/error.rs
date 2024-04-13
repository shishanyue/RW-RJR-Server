use std::sync::Arc;

use thiserror::Error;
use tokio::sync::mpsc;

use crate::{connection::shared_connection::SharedConnection, packet::Packet};

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("data store disconnected")]
    TypeError(#[from] mpsc::error::SendError<(Arc<SharedConnection>, Packet)>),
}

#[derive(Error, Debug)]
pub enum ReceiverError {
    #[error("`{0}`")]
    InvalidInput(String)
}
#[derive(Error, Debug)]
pub enum BasicDummyError {
    #[error("join to relay error mag: {0}")]
    JoinRelayError(&'static str)
}
