use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum TorMessage {
    NotForYou { data: Vec<u8> },
    NextNode { next_encrypted: Vec<u8> },
    HandShake([u8; 32]),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy)]
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
pub enum NetworkMessage<T> {
    TorMessage(T),
    ServerMessage(Vec<u8>),
    ConnectTo(Next),
}
