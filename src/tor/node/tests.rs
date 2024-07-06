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
    tor::tor_message::{MoveAlongMessage, Next, TorMessage},
};

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

#[tokio::test]
async fn network_handshake() -> anyhow::Result<()> {
    env_logger::init();

    const PORT: u16 = 10000;
    const FAKE_SERVER_PORT: u16 = 12345;

    let mut node = start_node(PORT).await?;
    let mut server = start_fake_server(FAKE_SERVER_PORT).await?;

    sleep(Duration::from_secs_f32(1.5)).await;

    let encryption = KeyPair::default();

    let stream = tokio::net::TcpStream::connect(format!("localhost:{}", PORT)).await?;
    info!("Connected to node!");
    tokio::time::sleep(tokio::time::Duration::from_secs_f32(3.0)).await;

    let mut writer: NodeIO<_, TorMessage, MoveAlongMessage> = NodeIO::new(stream);
    writer
        .write(MoveAlongMessage {
            next: Next::Server(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(127, 0, 0, 1),
                FAKE_SERVER_PORT,
            ))),
            data: TorMessage::HandShake(encryption.initial_public_message()),
        })
        .await?;
    info!("Wrote message to node!");

    let TorMessage::HandShake(pubkey) = writer.read().await? else {
        panic!("Didn't receive handshake back");
    };
    let encryption = encryption.handshake(pubkey);
    let message = "Hello";
    let encrypted = encryption.encrypt(message.as_bytes());
    writer
        .write(MoveAlongMessage {
            next: Next::Server(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(127, 0, 0, 1),
                FAKE_SERVER_PORT,
            ))),
            data: TorMessage::NotForYou { data: encrypted },
        })
        .await?;

    sleep(Duration::from_secs_f32(3.0)).await;

    node.kill().await?;
    server.kill().await?;
    Ok(())
}
