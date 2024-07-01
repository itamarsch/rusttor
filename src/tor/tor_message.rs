use serde::{Deserialize, Serialize};
use x25519_dalek::PublicKey;

use super::Node;

#[derive(Serialize, Deserialize)]
pub enum TorMessage {
    NotForYou { data: Vec<u8> },
    HandShake(PublicKey),
}

#[derive(Serialize, Deserialize)]
pub struct MoveAlongMessage {
    pub next: Node,
    pub not_for_you_data: TorMessage,
}
