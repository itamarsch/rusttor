use clap::Parser;
use log::info;
use rustor::tor::node::handle_connection;
use tokio::net::TcpListener;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Port to use for tor node
    #[arg(short, long, default_value_t = 0)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder().format_timestamp(None).init();
    let args = Args::parse();

    let listener = TcpListener::bind(format!("127.0.0.1:{}", args.port)).await?;

    let local_addr = listener.local_addr()?;
    println!("Listening on {}", local_addr);

    loop {
        let Ok((stream, addr)) = listener.accept().await else {
            continue;
        };
        info!("New connection!, {}", addr);

        tokio::spawn(handle_connection(stream));
    }
}
