use rustor::tor::node::handle_connection;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bind to a TCP port (let OS choose the port)
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let local_addr = listener.local_addr()?;
    println!("Listening on {}", local_addr);

    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };

        tokio::spawn(handle_connection(stream)).await??;
    }
}
