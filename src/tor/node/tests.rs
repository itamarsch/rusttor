#![allow(dead_code, unused_imports)]

use log::info;
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};
use tokio::{
    process::{Child, Command},
    time::sleep,
};

use crate::{
    encryption::KeyPair,
    node_io::NodeIO,
    tor::{
        client::TorClient,
        onion::{decrypt_onion_layers, onion_wrap_handshake, onion_wrap_packet},
        tor_message::{MoveAlongMessage, Next, TorMessage},
    },
};

const NODE1_PORT: u16 = 10000;
const NODE2_PORT: u16 = 10001;
const NODE3_PORT: u16 = 10002;
const NODE4_PORT: u16 = 10003;

const FAKE_SERVER_PORT: u16 = 12345;
const NODE1: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), NODE1_PORT));

const NODE2: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), NODE2_PORT));

const NODE3: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), NODE3_PORT));

const NODE4: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), NODE4_PORT));

const FAKE_SERVER: SocketAddr = SocketAddr::V4(SocketAddrV4::new(
    Ipv4Addr::new(127, 0, 0, 1),
    FAKE_SERVER_PORT,
));

async fn start_node(port: u16) -> anyhow::Result<Child> {
    let proc = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("node")
        .arg("-q")
        .arg("--")
        .arg("-p")
        .arg(port.to_string())
        .spawn()?;
    Ok(proc)
}

async fn start_fake_server(port: u16) -> anyhow::Result<Child> {
    let server = Command::new("python3")
        .arg("./src/tor/node/test_server.py")
        .arg(port.to_string())
        .spawn()
        .expect("Hello");
    Ok(server)
}

async fn end_to_end() -> anyhow::Result<()> {
    info!("Connected to node!");

    let mut client =
        TorClient::nodes_handshake(vec![NODE1, NODE2, NODE3, NODE4], FAKE_SERVER).await?;

    info!("Finised handshake with node2!");

    let message = "Hello";
    client.write(message.as_bytes().to_vec()).await?;

    let message = client.read().await?;
    let message = String::from_utf8(message)?;
    info!("Reponse is: {}", message);

    Ok(())
}

#[tokio::test]
async fn test_end_to_end() -> anyhow::Result<()> {
    env_logger::init();

    let mut node_1_proc = start_node(NODE1_PORT).await?;
    let mut node_2_proc = start_node(NODE2_PORT).await?;
    let mut node_3_proc = start_node(NODE3_PORT).await?;
    let mut node_4_proc = start_node(NODE4_PORT).await?;
    let mut server = start_fake_server(FAKE_SERVER_PORT).await?;
    sleep(Duration::from_secs_f32(1.5)).await;
    let result = end_to_end().await;
    node_1_proc.kill().await?;
    node_2_proc.kill().await?;
    node_3_proc.kill().await?;
    node_4_proc.kill().await?;
    server.kill().await?;
    result
}
