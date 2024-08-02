use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

use gerevs::{
    auth::NoAuthAuthenticator,
    method_handlers::{AssociateDenier, BindDenier, Connect, SocksSocketAddr},
    Socks5Error, Socks5Socket,
};
use rustor::tor::client::{nodes_handshake, TorClient};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = TcpListener::bind("0.0.0.0:1080").await?;
    loop {
        let (client, _addr) = server.accept().await?;

        tokio::spawn(async move {
            let result = handle_connection(client).await;
            if let Err(err) = result {
                eprintln!("Failed: {:?}", err);
            }
        });
    }
}

async fn handle_connection(client: TcpStream) -> gerevs::Result<()> {
    let socks5_stream = Socks5Socket::new(
        client,
        NoAuthAuthenticator,
        TorConnect,
        BindDenier,
        AssociateDenier,
    );
    socks5_stream.run().await
}

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
struct TorConnect;

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
        let destination = &*destination.to_socket_addr().await?;
        let (reader, writer) = nodes_handshake(
            vec![NODE1, NODE2, NODE3, NODE4, NODE5, NODE6],
            destination[0],
        )
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

        // Buffer to read data from client

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
                    server_writer.write(buf[..n].to_vec()).await.map_err(|_| {
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
