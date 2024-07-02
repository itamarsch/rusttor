use tokio::{
    io::{ReadHalf, WriteHalf},
    net::TcpStream,
};

use super::circuit_manager::CircuitManager;

type TcpReadWriter = (ReadHalf<TcpStream>, WriteHalf<TcpStream>);

async fn handle_connection(stream: TcpStream) -> anyhow::Result<()> {
    let (back_reader, back_writer) = tokio::io::split(stream);

    let circuit_manager = CircuitManager::default();

    Ok(())
}
