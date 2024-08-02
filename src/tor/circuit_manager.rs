use serde::Serialize;

use super::tor_message::{NetworkMessage, Next, TorMessage};
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

pub type IncomingMessage = Directional<TorMessage, TorMessage>;
pub type OutgoingMessage = Directional<NetworkMessage<TorMessage>, TorMessage>;

#[derive(Default)]
pub struct CircuitManager {
    encryptor: Option<Encryptor>,
    next: Option<Next>,
}

impl CircuitManager {
    pub fn message(&mut self, message: IncomingMessage) -> anyhow::Result<OutgoingMessage> {
        match message {
            Directional::Forward(TorMessage::HandShake(public_key)) => self.handshake(public_key),
            Directional::Forward(TorMessage::NotForYou { data }) => self.push_onward(data),
            Directional::Forward(TorMessage::NextNode { next_encrypted }) => {
                self.connect(&next_encrypted[..])
            }
            Directional::Back(message) => self.push_response_back(message),
        }
    }

    fn handshake(&mut self, other_public_key: [u8; 32]) -> anyhow::Result<OutgoingMessage> {
        if self.encryptor.is_some() {
            anyhow::bail!("Received handshake after handshake complete")
        }

        let (encryptor, my_public) = Encryptor::from_public(other_public_key);

        self.encryptor = Some(encryptor);

        Ok(Directional::Back(TorMessage::HandShake(my_public)))
    }

    pub fn connect(&mut self, encrypted_addr: &[u8]) -> anyhow::Result<OutgoingMessage> {
        let Some(ref encryptor) = &self.encryptor else {
            anyhow::bail!("received notforyou before handshake")
        };

        let addr = encryptor.decrypt(encrypted_addr)?;
        let addr: Next = bincode::deserialize(&addr[..])?;
        self.next = Some(addr);
        Ok(Directional::Forward(NetworkMessage::ConnectTo(addr)))
    }

    pub fn push_onward(&mut self, onioned_data: Vec<u8>) -> anyhow::Result<OutgoingMessage> {
        let Some(ref encryptor) = &self.encryptor else {
            anyhow::bail!("received notforyou before handshake")
        };
        let Some(next) = &self.next else {
            anyhow::bail!("received notforyou before connect")
        };

        let deonionized = encryptor.decrypt(&onioned_data[..])?;
        let next_message = if next.is_server() {
            NetworkMessage::ServerMessage(deonionized)
        } else {
            NetworkMessage::TorMessage(bincode::deserialize(&deonionized[..])?)
        };

        Ok(Directional::Forward(next_message))
    }

    fn push_response_back<T>(&mut self, message: T) -> anyhow::Result<OutgoingMessage>
    where
        T: Serialize,
    {
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

#[cfg(test)]
mod tests {

    use super::CircuitManager;
    use crate::{
        encryption::{Encryptor, KeyPair, PublicKeyBytes},
        tor::{
            circuit_manager::Directional,
            tor_message::{NetworkMessage, Next, TorMessage},
        },
    };
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    impl CircuitManager {
        fn handshook(other_publickey: PublicKeyBytes) -> (Self, PublicKeyBytes) {
            let (encryptor, my_public) = Encryptor::from_public(other_publickey);

            (
                CircuitManager {
                    encryptor: Some(encryptor),
                    next: None,
                },
                my_public,
            )
        }
    }

    const NEXT_NODE: Next = Next::Node(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(1, 1, 1, 1),
        1,
    )));

    const NEXT_SERVER: Next = Next::Server(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(1, 1, 1, 1),
        1,
    )));

    #[test]
    fn node_forward_message() -> anyhow::Result<()> {
        let move_along = TorMessage::NotForYou { data: vec![1] };

        let bob = KeyPair::default();

        let (mut circuit_manager, alice_pub) =
            CircuitManager::handshook(bob.initial_public_message());

        let bob = bob.handshake(alice_pub);

        let next_bytes = bincode::serialize(&NEXT_NODE)?;
        let next_encrypted = bob.encrypt(&next_bytes);

        let next_node_reponse =
            circuit_manager.message(Directional::Forward(TorMessage::NextNode {
                next_encrypted,
            }))?;

        assert!(matches!(
            next_node_reponse,
            Directional::Forward(NetworkMessage::ConnectTo(NEXT_NODE))
        ));

        let message = TorMessage::NotForYou {
            data: bob.encrypt(&bincode::serialize(&move_along)?[..]),
        };

        let Directional::Forward(NetworkMessage::TorMessage(result)) =
            circuit_manager.message(Directional::Forward(message))?
        else {
            panic!("Unexpected message received")
        };

        assert_eq!(result, move_along);
        Ok(())
    }

    #[test]
    fn server_forward_message() -> anyhow::Result<()> {
        let data = vec![1];
        let bob = KeyPair::default();

        let (mut circuit_manager, alice_pub) =
            CircuitManager::handshook(bob.initial_public_message());

        let bob = bob.handshake(alice_pub);

        let next_bytes = bincode::serialize(&NEXT_SERVER)?;
        let next_encrypted = bob.encrypt(&next_bytes);

        let next_node_reponse =
            circuit_manager.message(Directional::Forward(TorMessage::NextNode {
                next_encrypted,
            }))?;

        assert!(matches!(
            next_node_reponse,
            Directional::Forward(NetworkMessage::ConnectTo(NEXT_SERVER))
        ));

        let message = TorMessage::NotForYou {
            data: bob.encrypt(&data[..]),
        };

        let Directional::Forward(NetworkMessage::ServerMessage(result)) =
            circuit_manager.message(Directional::Forward(message))?
        else {
            panic!("Unexpected message received")
        };

        assert_eq!(result, data);
        Ok(())
    }
    #[test]
    fn handshake() -> anyhow::Result<()> {
        let mut circuit_manager = CircuitManager::default();

        // Send handshake message forward
        let bob = KeyPair::default();
        let handshake = TorMessage::HandShake(bob.initial_public_message());

        let Directional::Back(TorMessage::HandShake(pubkey)) =
            circuit_manager.message(Directional::Forward(handshake))?
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
        }) = circuit_manager.message(Directional::Back(TorMessage::NotForYou {
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
