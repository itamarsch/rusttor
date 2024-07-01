use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};

mod node_directory;

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub struct Node {
    pub ip: Ipv4Addr,
    pub port: u16,
}
