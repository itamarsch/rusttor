use tokio::sync::mpsc::Sender;

use super::tor_message::{MoveAlongMessage, TorMessage};
use crate::encryption::{Encryptor, KeyPair};

pub struct CircuitManager {
    pub encryptor: Option<Encryptor>,
    pub forward_sender: Sender<MoveAlongMessage>,
    pub backward_sender: Sender<Vec<u8>>,
}

impl CircuitManager {
    pub async fn on_forward_message(&mut self, message: TorMessage) -> anyhow::Result<()> {
        match message {
            TorMessage::NotForYou { data } => {
                let Some(ref encryptor) = &self.encryptor else {
                    anyhow::bail!("received notforyou before handshake")
                };

                let deonionized = encryptor.decrypt(&data[..])?;
                let next_message: MoveAlongMessage = bincode::deserialize(&deonionized[..])?;

                self.forward_sender.send(next_message).await?;
                Ok(())
            }

            TorMessage::HandShake(other_public_key) => {
                if self.encryptor.is_some() {
                    anyhow::bail!("Received handshake after handshake complete")
                }

                let encryptor = KeyPair::default();
                let public_back_message = encryptor.initial_public_message().to_vec();

                self.backward_sender.send(public_back_message).await?;

                let encryptor = encryptor.handshake(&other_public_key);
                self.encryptor = Some(encryptor);

                Ok(())
            }
        }
    }

    pub async fn on_backward_message(&mut self, message: Vec<u8>) -> anyhow::Result<()> {
        let Some(ref encryptor) = &self.encryptor else {
            anyhow::bail!("received notforyou before handshake")
        };

        let encrypted_back = encryptor.encrypt(&message[..]);

        self.backward_sender.send(encrypted_back).await?;
        Ok(())
    }
}
