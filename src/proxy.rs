use crate::tor::{
    client::{nodes_handshake, TorClient},
    node_directory::get_nodes,
};
use gerevs::{
    method_handlers::{Connect, SocksSocketAddr},
    Socks5Error,
};
use rand::{rngs::OsRng, seq::SliceRandom, Rng};
use std::{io, net::SocketAddr};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    task::JoinHandle,
};

async fn get_nodes_randomized() -> anyhow::Result<Vec<SocketAddr>> {
    const MIN_NODES: u8 = 5;
    const MAX_NODES: u8 = 15;
    let amount_of_nodes: u8 = rand::thread_rng().gen_range(MIN_NODES..=MAX_NODES);
    let mut nodes = get_nodes(amount_of_nodes).await?;

    // Shuffle the nodes
    nodes.shuffle(&mut OsRng);
    Ok(nodes)
}

pub struct TorConnect;

impl Connect<()> for TorConnect {
    type ServerConnection = (
        TorClient<ReadHalf<TcpStream>>,
        TorClient<WriteHalf<TcpStream>>,
    );

    async fn establish_connection(
        &mut self,
        destination: SocksSocketAddr,
        _: (),
    ) -> gerevs::Result<Self::ServerConnection> {
        println!("New connection: {:?}", destination);
        let nodes_local = get_nodes_randomized()
            .await
            .map_err(|_| Socks5Error::IoError(io::ErrorKind::ConnectionRefused.into()))?;

        let destination = &*destination.to_socket_addr().await?;
        let (reader, writer) = nodes_handshake(nodes_local, destination[0])
            .await
            .map_err(|_| gerevs::Socks5Error::IoError(io::ErrorKind::ConnectionAborted.into()))?;
        Ok((reader, writer))
    }

    async fn start_listening<T>(
        self,
        client: T,
        connection: Self::ServerConnection,
    ) -> gerevs::Result<()>
    where
        T: tokio::io::AsyncWrite + tokio::io::AsyncRead + Send + Unpin + 'static,
    {
        let (mut server_reader, mut server_writer) = connection;

        let (mut client_reader, mut client_writer) = tokio::io::split(client);

        let client_to_server =
            tokio::spawn(async move {
                let mut buf = vec![0u8; 1024];

                loop {
                    let n = client_reader.read(&mut buf).await.map_err(|_| {
                        Socks5Error::IoError(io::ErrorKind::ConnectionAborted.into())
                    })?;
                    if n == 0 {
                        break;
                    }
                    server_writer.write(&buf[..n]).await.map_err(|_| {
                        Socks5Error::IoError(io::ErrorKind::ConnectionAborted.into())
                    })?;
                }

                Ok::<_, Socks5Error>(())
            });

        let server_to_client: JoinHandle<Result<(), Socks5Error>> =
            tokio::spawn(async move {
                loop {
                    let n = server_reader.read().await.map_err(|_| {
                        Socks5Error::IoError(io::ErrorKind::ConnectionAborted.into())
                    })?;

                    client_writer.write_all(&n[..]).await.map_err(|_| {
                        Socks5Error::IoError(io::ErrorKind::ConnectionAborted.into())
                    })?;
                }
            });
        drop(server_to_client);
        drop(client_to_server);

        Ok(())
    }
}
