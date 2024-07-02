use serde::{Deserialize, Serialize};

use super::Node;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum TorMessage {
    NotForYou { data: Vec<u8> },
    HandShake([u8; 32]),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct MoveAlongMessage {
    pub next: Node,
    pub data: TorMessage,
}
