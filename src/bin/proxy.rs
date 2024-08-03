use gerevs::{
    auth::NoAuthAuthenticator,
    method_handlers::{AssociateDenier, BindDenier},
    Socks5Socket,
};
use rustor::proxy::TorConnect;
use tokio::net::{TcpListener, TcpStream};

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
