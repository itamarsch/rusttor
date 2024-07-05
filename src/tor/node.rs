use serde::de::DeserializeOwned;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;

use crate::{node_io::NodeIO, tor::circuit_manager::Directional};

use super::{
    circuit_manager::CircuitManager,
    tor_message::{MoveAlongMessage, Next, TorMessage},
};

pub async fn handle_connection(stream: TcpStream) -> anyhow::Result<()> {
    let cancellation_token = CancellationToken::new();

    let (back_read, back_write) = tokio::io::split(stream);
    let back_write: NodeIO<_, (), TorMessage> = NodeIO::new(back_write);

    let mut back_node_reader: NodeIO<_, MoveAlongMessage, ()> = NodeIO::new(back_read);
    let (back_sender, back_receiver) = mpsc::channel(10);

    let message = back_node_reader.read().await?;
    tokio::spawn(reader_task(
        back_node_reader,
        cancellation_token.clone(),
        back_sender,
    ));

    let (forward_read, forward_write) = tokio::io::split(match message.next {
        Next::Node(n) => TcpStream::connect(n).await?,
        Next::Server(n) => TcpStream::connect(n).await?,
    });

    let front_read: NodeIO<_, TorMessage, ()> = NodeIO::new(forward_read);
    let front_write: NodeIO<_, (), MoveAlongMessage> = NodeIO::new(forward_write);

    let (front_sender, front_receiver) = mpsc::channel(10);
    tokio::spawn(reader_task(
        front_read,
        cancellation_token.clone(),
        front_sender,
    ));

    tor_node(
        cancellation_token,
        front_write,
        front_receiver,
        back_write,
        back_receiver,
    )
    .await;
    Ok(())
}

async fn tor_node(
    cancellation: CancellationToken,
    mut forward_write: NodeIO<impl AsyncWrite + Unpin, (), MoveAlongMessage>,
    mut front_receiver: mpsc::Receiver<TorMessage>,
    mut back_write: NodeIO<impl AsyncWrite + Unpin, (), TorMessage>,
    mut back_receiver: mpsc::Receiver<MoveAlongMessage>,
) {
    let mut circuit_manager = CircuitManager::default();

    loop {
        tokio::select! {
            Some(forward_msg) = front_receiver.recv() => {
                // Read from the front: direction is backward
                let Ok(a) = circuit_manager.message(Directional::Back(forward_msg)) else {
                    break;
                };
                let Ok(()) = (match a {
                    Directional::Back(m) => back_write.write(m).await,
                    Directional::Forward(m) => forward_write.write(m).await,
                }) else {
                    break;
                };
            },
            Some(back_msg) = back_receiver.recv() => {
                // Read from the back: direction is forward
                let Ok(a) = circuit_manager.message(Directional::Forward(back_msg)) else {
                    break;
                };
                let Ok(()) = (match a {
                    Directional::Back(m) => back_write.write(m).await,
                    Directional::Forward(m) => forward_write.write(m).await,
                }) else {
                    break;
                };
            },
            else => {
                break;
            }
        }
    }
    cancellation.cancel();
}

async fn reader_task<V, G>(
    mut reader: NodeIO<impl AsyncRead + Unpin, V, G>,
    cancellation: CancellationToken,
    new_data_sender: mpsc::Sender<V>,
) where
    V: DeserializeOwned,
{
    loop {
        tokio::select! {
            _ = cancellation.cancelled() => {
                println!("Cancellation requested, shutting down reader_task.");
                break;
            },
            result = reader.read() => {
                match result {
                    Ok(data) => {
                        let Ok(()) = new_data_sender.send(data).await else {
                            cancellation.cancel();
                            break;
                        };

                    }
                    Err(_) => {
                        cancellation.cancel();
                        break;
                    }
                }
            },
        }
    }

    println!("reader_task has been gracefully shut down.");
}

// Example
