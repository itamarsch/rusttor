use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{Arc, Mutex},
};

use log::info;
use rustor::tor::client::{nodes_handshake, TorClient};

macro_rules! localhost {
    ($name:ident,$port:expr) => {
        const $name: SocketAddr =
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), $port));
    };
}

localhost!(NODE1, 10000);
localhost!(NODE2, 10001);
localhost!(NODE3, 10002);
localhost!(NODE4, 10003);
localhost!(NODE5, 10004);
localhost!(NODE6, 10005);
localhost!(FAKE_SERVER, 12345);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder().format_timestamp(None).init();

    info!("Connected to node!");

    let (mut reader, mut writer) =
        nodes_handshake(vec![NODE1, NODE2, NODE3, NODE4, NODE5, NODE6], FAKE_SERVER).await?;

    info!("Finised handshake with!");
    tokio::spawn(async move {
        for i in 1..20 {
            let message = format!("Hello {}", i);
            writer.write(message.as_bytes().to_vec()).await.unwrap();
        }
    });

    loop {
        let message = reader.read().await?;
        let message = String::from_utf8(message)?;
        info!("Received message: {:?}", message);
    }
}
