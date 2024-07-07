use log::{error, info};
use serde::de::DeserializeOwned;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite},
    net::TcpStream,
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;

use crate::{
    node_io::NodeIO,
    tor::{circuit_manager::Directional, tor_message::NetworkMessage},
};

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
    let next = message.next.clone();

    back_sender
        .send(message)
        .await
        .expect("Channel isn't closed");

    tokio::spawn(reader_task(
        back_node_reader,
        cancellation_token.clone(),
        back_sender,
    ));

    let (front_read, forward_write) = tokio::io::split(match next {
        Next::Node(n) => TcpStream::connect(n).await?,
        Next::Server(n) => TcpStream::connect(n).await?,
    });

    let front_write: NodeIO<_, (), MoveAlongMessage> = NodeIO::new(forward_write);

    let (front_sender, front_receiver) = mpsc::channel(10);

    if next.is_server() {
        tokio::spawn(server_reader_task(
            front_read,
            cancellation_token.clone(),
            front_sender,
        ));
    } else {
        let front_read: NodeIO<_, TorMessage, ()> = NodeIO::new(front_read);
        tokio::spawn(reader_task(
            front_read,
            cancellation_token.clone(),
            front_sender,
        ));
    }
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

    async fn handle_message(
        circuit_manager: &mut CircuitManager,
        message: Directional<MoveAlongMessage, TorMessage>,
        forward_write: &mut NodeIO<impl AsyncWrite + Unpin, (), MoveAlongMessage>,
        back_write: &mut NodeIO<impl AsyncWrite + Unpin, (), TorMessage>,
    ) -> anyhow::Result<()> {
        let a = circuit_manager.message(message)?;
        match a {
            Directional::Back(m) => {
                info!("Writing backward: {:?}", m);
                back_write.node_write(m).await
            }
            Directional::Forward(NetworkMessage::TorMessage(m)) => {
                info!("Writing forward: {:?}", m);
                forward_write.node_write(m).await
            }
            Directional::Forward(NetworkMessage::ServerMessage(data)) => {
                info!("Writing to server: {:?}", data);
                forward_write.write_raw(&data).await
            }
        }
    }

    loop {
        tokio::select! {
            Some(forward_msg) = front_receiver.recv() => {
                info!("Received message from back: {:?}",forward_msg);
                // Read from the front: direction is backward
                if let Err(err) = handle_message(&mut circuit_manager, Directional::Back(forward_msg), &mut forward_write, &mut back_write).await {
                    error!("{:?}",err);
                    break
                }
            },
            Some(back_msg) = back_receiver.recv() => {
                info!("Received message from back: {:?}",back_msg);
                // Read from the back: direction is forward
                if let Err(err) = handle_message(&mut circuit_manager, Directional::Forward(back_msg), &mut forward_write, &mut back_write).await {
                    error!("Failed sending message: {:?}",err);
                    break
                }
            },
            else => {
                break;
            }
        }
    }
    cancellation.cancel();
}

async fn server_reader_task(
    mut reader: impl AsyncRead + Unpin,
    cancellation: CancellationToken,
    new_data_sender: mpsc::Sender<TorMessage>,
) {
    let mut buf = vec![0; 1024];
    loop {
        tokio::select! {
            _ = cancellation.cancelled() => {
                info!("Cancellation requested, shutting down server_reader_task.");
                break;
            }
            Ok(len) = reader.read(&mut buf) => {
                if len == 0 {
                    break;
                }

                let message = buf[0..len].to_vec();
                let message = TorMessage::NotForYou { data: message };
                if new_data_sender.send(message).await.is_err() {
                    error!("Failed sending to channel");
                    break;
                }
            }
            else => {
                info!("Failed reading from server closing");
                cancellation.cancel();
                break;
            }
        };
    }
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
                info!("Cancellation requested, shutting down reader_task.");
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
}

mod tests;
// Example
