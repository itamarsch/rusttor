use serde::Serialize;

use super::tor_message::{MoveAlongMessage, TorMessage};
use crate::encryption::Encryptor;

#[derive(Debug, PartialEq, Eq)]
pub enum Directional<F, B>
where
    F: Serialize,
    B: Serialize,
{
    Back(B),
    Forward(F),
}

pub type ResultMessage = Directional<MoveAlongMessage, TorMessage>;
pub type RequestMessage = Directional<TorMessage, TorMessage>;

#[derive(Default)]
pub struct CircuitManager {
    encryptor: Option<Encryptor>,
}

impl CircuitManager {
    pub fn on_message(&mut self, message: RequestMessage) -> anyhow::Result<ResultMessage> {
        match message {
            Directional::Forward(TorMessage::NotForYou { data }) => {
                let Some(ref encryptor) = &self.encryptor else {
                    anyhow::bail!("received notforyou before handshake")
                };

                let deonionized = encryptor.decrypt(&data[..])?;
                let next_message: MoveAlongMessage = bincode::deserialize(&deonionized[..])?;

                Ok(Directional::Forward(next_message))
            }

            Directional::Forward(TorMessage::HandShake(other_public_key)) => {
                if self.encryptor.is_some() {
                    anyhow::bail!("Received handshake after handshake complete")
                }

                let (encryptor, my_public) = Encryptor::from_public(other_public_key);

                self.encryptor = Some(encryptor);

                Ok(Directional::Back(TorMessage::HandShake(my_public)))
            }
            Directional::Back(message) => {
                let Some(ref encryptor) = &self.encryptor else {
                    anyhow::bail!("received notforyou before handshake")
                };

                let data = bincode::serialize(&message)?;

                let encrypted_back = encryptor.encrypt(&data[..]);

                Ok(Directional::Back(TorMessage::NotForYou {
                    data: encrypted_back,
                }))
            }
        }
    }

    // Used for test purposes only!
}

#[cfg(test)]
mod tests {

    use super::CircuitManager;
    use crate::{
        encryption::{Encryptor, KeyPair, PublicKeyBytes},
        tor::{
            circuit_manager::Directional,
            tor_message::{MoveAlongMessage, TorMessage},
            Node,
        },
    };
    use std::net::Ipv4Addr;

    impl CircuitManager {
        fn handshook(other_publickey: PublicKeyBytes) -> (Self, PublicKeyBytes) {
            let (encryptor, my_public) = Encryptor::from_public(other_publickey);

            (
                CircuitManager {
                    encryptor: Some(encryptor),
                },
                my_public,
            )
        }
    }

    #[test]
    fn forward_message() -> anyhow::Result<()> {
        let move_along = MoveAlongMessage {
            next: Node {
                ip: Ipv4Addr::new(1, 1, 1, 1),
                port: 1,
            },
            not_for_you_data: TorMessage::NotForYou { data: vec![1] },
        };

        let bob = KeyPair::default();

        let (mut circuit_manager, alice_pub) =
            CircuitManager::handshook(bob.initial_public_message());

        let bob = bob.handshake(alice_pub);

        let message = TorMessage::NotForYou {
            data: bob.encrypt(&bincode::serialize(&move_along)?[..]),
        };

        let result = circuit_manager.on_message(super::Directional::Forward(message))?;

        assert_eq!(result, Directional::Forward(move_along));
        Ok(())
    }

    #[test]
    fn handshake() -> anyhow::Result<()> {
        let mut circuit_manager = CircuitManager::default();

        // Send handshake message forward
        let bob = KeyPair::default();
        let handshake = TorMessage::HandShake(bob.initial_public_message());

        let Directional::Back(TorMessage::HandShake(pubkey)) =
            circuit_manager.on_message(Directional::Forward(handshake))?
        else {
            panic!("Handshake response wasn't sent back")
        };

        assert!(circuit_manager.encryptor.is_some());

        let bob = bob.handshake(pubkey);

        let message = "Hello".as_bytes().to_vec();
        let node_encrypted = circuit_manager.encryptor.unwrap().encrypt(&message[..]);

        let bob_decrypted = bob.decrypt(&node_encrypted[..])?;

        assert_eq!(message, bob_decrypted);

        Ok(())
    }

    #[test]
    fn backward() -> anyhow::Result<()> {
        let bob = KeyPair::default();

        let (mut circuit_manager, alice_pub) =
            CircuitManager::handshook(bob.initial_public_message());

        let bob = bob.handshake(alice_pub);

        let data = vec![1, 2, 3];
        let Directional::Back(TorMessage::NotForYou {
            data: encrypted_data,
        }) = circuit_manager.on_message(Directional::Back(TorMessage::NotForYou {
            data: data.clone(),
        }))?
        else {
            panic!("Unexpected behavior")
        };

        let result = bob.decrypt(&encrypted_data[..])?;
        let TorMessage::NotForYou { data: result } = bincode::deserialize(&result[..])? else {
            panic!("Invalid tor message")
        };

        assert_eq!(data, result);

        Ok(())
    }
}
