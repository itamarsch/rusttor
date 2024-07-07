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
        packet_builder::{build_handshake, build_packet},
        tor_message::{MoveAlongMessage, Next, TorMessage},
    },
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

    const NODE1_PORT: u16 = 10000;
    const NODE2_PORT: u16 = 10001;

    const FAKE_SERVER_PORT: u16 = 12345;
    let fake_server = Next::Server(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(127, 0, 0, 1),
        FAKE_SERVER_PORT,
    )));

    let mut node_1_proc = start_node(NODE1_PORT).await?;
    let mut server = start_fake_server(FAKE_SERVER_PORT).await?;

    sleep(Duration::from_secs_f32(1.5)).await;

    let encryption_1 = KeyPair::default();

    let stream = tokio::net::TcpStream::connect(format!("localhost:{}", NODE1_PORT)).await?;
    info!("Connected to node!");
    tokio::time::sleep(tokio::time::Duration::from_secs_f32(3.0)).await;

    let mut writer: NodeIO<_, TorMessage, MoveAlongMessage> = NodeIO::new(stream);
    writer
        .node_write(
            build_handshake(
                &[(None, fake_server)],
                encryption_1.initial_public_message(),
            )
            .unwrap(),
        )
        .await?;
    info!("Wrote message to node!");

    let TorMessage::HandShake(pubkey) = writer.read().await? else {
        panic!("Didn't receive handshake back");
    };
    let encryption_1 = encryption_1.handshake(pubkey);

    let message = "Hello";
    writer
        .node_write(
            build_packet(&[(&encryption_1, fake_server)], message.as_bytes().to_vec())
                .expect("Isn't empty"),
        )
        .await?;

    let TorMessage::NotForYou { data: response } = writer.read().await? else {
        panic!("Unexpected handshake");
    };
    let message = encryption_1.decrypt(&response)?;
    let message = String::from_utf8(message)?;
    info!("Reponse is: {}", message);

    sleep(Duration::from_secs_f32(3.0)).await;

    node_1_proc.kill().await?;
    server.kill().await?;
    Ok(())
}
