use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum TorMessage {
    NotForYou { data: Vec<u8> },
    HandShake([u8; 32]),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum Next {
    Node(SocketAddr),
    Server(SocketAddr),
}
impl Next {
    pub fn is_server(&self) -> bool {
        match self {
            Next::Node(_) => false,
            Next::Server(_) => true,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum NetworkMessage {
    TorMessage(MoveAlongMessage),
    ServerMessage(Vec<u8>),
}
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct MoveAlongMessage {
    pub next: Next,
    pub data: TorMessage,
}
