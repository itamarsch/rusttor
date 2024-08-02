use log::info;
use std::{iter, net::SocketAddr};
use tokio::io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf};
use tokio::net::TcpStream;

use crate::tor::onion::onion_wrap_connect_to;
use crate::{
    encryption::{Encryptor, KeyPair},
    node_io::NodeIO,
    tor::onion::{decrypt_onion_layers, onion_wrap_handshake},
};

use super::{
    onion::onion_wrap_packet,
    tor_message::{Next, TorMessage},
};
type NetworkIO<T> = NodeIO<T, TorMessage, TorMessage>;
pub struct TorClient<T> {
    nodes: Vec<(Encryptor, Next)>,

    stream: NetworkIO<T>,
}

pub async fn nodes_handshake(
    mut nodes: Vec<SocketAddr>,
    server: SocketAddr,
) -> anyhow::Result<(
    TorClient<ReadHalf<TcpStream>>,
    TorClient<WriteHalf<TcpStream>>,
)> {
    assert!(!nodes.is_empty(), "Can't run a request on zero nodes");
    let stream = TcpStream::connect(nodes[0]).await?;
    let (reader, writer) = tokio::io::split(stream);

    let mut reader: NetworkIO<_> = NodeIO::new(reader);
    let mut writer: NetworkIO<_> = NodeIO::new(writer);

    nodes.remove(0);
    let mut nodes = iter::repeat(None)
        .zip(nodes.into_iter().map(Next::Node))
        .collect::<Vec<_>>();
    nodes.push((None, Next::Server(server)));

    for i in 0..nodes.len() {
        let my_pubkey = KeyPair::default();

        writer
            .node_write(
                onion_wrap_handshake(&nodes[..], my_pubkey.initial_public_message()).unwrap(),
            )
            .await?;

        let encrypted_nodes = nodes
            .iter()
            .map(|(encryptor, _)| encryptor.as_ref())
            .take_while(|a| a.is_some())
            .map(|a| a.unwrap())
            .collect::<Vec<_>>();

        info!("Waiting for handshake");
        let TorMessage::HandShake(other_pubkey) =
            decrypt_onion_layers(&encrypted_nodes[..], reader.read().await?)?
        else {
            anyhow::bail!("Expected handshake");
        };

        let (encryptor, _) = &mut nodes[i];

        *encryptor = Some(my_pubkey.handshake(other_pubkey));

        writer
            .node_write(onion_wrap_connect_to(&nodes[..]).unwrap())
            .await?;
    }

    let reader_nodes = nodes
        .into_iter()
        .map(|(encryptor, next)| (encryptor.unwrap(), next))
        .collect::<Vec<_>>();

    let writer_nodes = reader_nodes.clone();

    Ok((
        TorClient {
            nodes: reader_nodes,
            stream: reader,
        },
        TorClient {
            nodes: writer_nodes,
            stream: writer,
        },
    ))
}
impl<T> TorClient<T>
where
    T: AsyncWrite + Unpin,
{
    pub async fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.stream
            .node_write(onion_wrap_packet(&self.nodes[..], data).expect("Isn't empty"))
            .await?;
        Ok(())
    }
}

impl<T> TorClient<T>
where
    T: AsyncRead + Unpin,
{
    pub async fn read(&mut self) -> anyhow::Result<Vec<u8>> {
        let message = self.stream.read().await?;

        let encryptors = self
            .nodes
            .iter()
            .map(|(encryptor, _)| encryptor)
            .collect::<Vec<_>>();

        let TorMessage::NotForYou { data: message } =
            decrypt_onion_layers(&encryptors[..], message)?
        else {
            anyhow::bail!("Invalid response")
        };
        Ok(message)
    }
}
