use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};

pub mod circuit_manager;
pub mod node;
mod node_directory;
pub mod tor_message;

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Ord, Eq, Clone, Copy)]
pub struct Node {
    pub ip: Ipv4Addr,
    pub port: u16,
}
