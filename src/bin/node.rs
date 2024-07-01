use rustor::network_handler::NetworkHandler;
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bind to a TCP port (let OS choose the port)
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let local_addr = listener.local_addr()?;
    println!("Listening on {}", local_addr);

    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        println!("Accepted client");

        tokio::spawn(handle_connection(stream)).await??;
    }
}

async fn handle_connection(stream: TcpStream) -> anyhow::Result<()> {
    let (reader, writer) = tokio::io::split(stream);
    let mut network_handler = NetworkHandler::perform_handshake(reader, writer).await?;

    let message = network_handler.read_buf_decrypt().await?;

    let message = String::from_utf8(message)?;
    println!("{message:?}");

    Ok(())
}
