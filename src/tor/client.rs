use std::{iter, net::SocketAddr};

use tokio::net::TcpStream;

use crate::{
    encryption::{Encryptor, KeyPair},
    node_io::NodeIO,
    tor::onion::{decrypt_onion_layers, onion_wrap_handshake},
};

use super::{
    onion::onion_wrap_packet,
    tor_message::{MoveAlongMessage, Next, TorMessage},
};

type NetworkNodeIO = NodeIO<TcpStream, TorMessage, MoveAlongMessage>;

pub struct TorClient {
    nodes: Vec<(Encryptor, Next)>,

    stream: NetworkNodeIO,
}

impl TorClient {
    pub async fn nodes_handshake(
        mut nodes: Vec<SocketAddr>,
        server: SocketAddr,
    ) -> anyhow::Result<TorClient> {
        assert!(!nodes.is_empty(), "Can't run a request on zero nodes");
        let stream = TcpStream::connect(nodes[0]).await?;

        let mut stream = NetworkNodeIO::new(stream);
        nodes.remove(0);
        let mut nodes = iter::repeat(None)
            .zip(nodes.into_iter().map(Next::Node))
            .collect::<Vec<_>>();
        nodes.push((None, Next::Server(server)));

        for i in 0..nodes.len() {
            let my_pubkey = KeyPair::default();

            stream
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

            let TorMessage::HandShake(other_pubkey) =
                decrypt_onion_layers(&encrypted_nodes[..], stream.read().await?)?
            else {
                anyhow::bail!("Expected handshake");
            };

            let (encryptor, _) = &mut nodes[i];

            *encryptor = Some(my_pubkey.handshake(other_pubkey));
        }

        let nodes = nodes
            .into_iter()
            .map(|(encryptor, next)| (encryptor.unwrap(), next))
            .collect::<Vec<_>>();

        Ok(TorClient { nodes, stream })
    }

    pub async fn write(&mut self, data: Vec<u8>) -> anyhow::Result<()> {
        self.stream
            .node_write(onion_wrap_packet(&self.nodes[..], data).expect("Isn't empty"))
            .await?;
        Ok(())
    }

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
