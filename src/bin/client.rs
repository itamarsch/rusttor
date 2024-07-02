use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the server
    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    println!("Connected to server!");

    let (reader, writer) = tokio::io::split(stream);
    // let mut network_handler = NetworkHandler::perform_handshake(reader, writer).await?;

    let message = "HI".as_bytes();

    // network_handler.write_buf_encrypt(message).await?;

    Ok(())
}
