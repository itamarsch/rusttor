use crate::encryption::{Encryptor, PublicKeyBytes};

use super::tor_message::{MoveAlongMessage, Next, TorMessage};

pub fn onion_wrap_tor_message(
    nodes: &[(Option<&Encryptor>, Next)],
    tor_message_build: impl Fn(Option<&Encryptor>, Next) -> TorMessage,
) -> Option<MoveAlongMessage> {
    nodes
        .iter()
        .rev()
        .fold(None, |message, (encryptor, next)| match message {
            None => {
                let tor_message = tor_message_build(*encryptor, *next);
                Some(MoveAlongMessage {
                    next: *next,
                    data: tor_message,
                })
            }
            Some(curr_message) => {
                let curr_message_bytes = bincode::serialize(&curr_message).unwrap();
                let curr_message_encrypted = encryptor.unwrap().encrypt(&curr_message_bytes);
                Some(MoveAlongMessage {
                    next: *next,
                    data: TorMessage::NotForYou {
                        data: curr_message_encrypted,
                    },
                })
            }
        })
}
pub fn onion_wrap_packet(nodes: &[(&Encryptor, Next)], data: Vec<u8>) -> Option<MoveAlongMessage> {
    let nodes = nodes
        .iter()
        .map(|(encryptor, next)| (Some(*encryptor), *next))
        .collect::<Vec<_>>();

    onion_wrap_tor_message(&nodes[..], |encryptor, next| {
        assert!(next.is_server());
        let data = encryptor.unwrap().encrypt(&data[..]);
        TorMessage::NotForYou { data }
    })
}

pub fn onion_wrap_handshake(
    nodes: &[(Option<&Encryptor>, Next)],

    pubkey: PublicKeyBytes,
) -> Option<MoveAlongMessage> {
    let nodes = nodes
        .iter()
        .scan(false, |predicate_broken, x| {
            if *predicate_broken {
                None
            } else {
                if x.0.is_none() {
                    *predicate_broken = true;
                }
                Some(x)
            }
        })
        .map(|(encryptor, next)| (*encryptor, *next))
        .collect::<Vec<_>>();

    onion_wrap_tor_message(&nodes[..], |_, _| TorMessage::HandShake(pubkey))
}

pub fn decrypt_onion_layers() {}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddrV4};

    use super::*;
    use crate::encryption::KeyPair;

    const BOB_NODE: Next = Next::Node(std::net::SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(1, 1, 1, 1),
        1,
    )));

    const SERVER: Next = Next::Server(std::net::SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(1, 1, 1, 1),
        2,
    )));
    #[test]
    fn test_build_packet() -> anyhow::Result<()> {
        // Setup Alice and Bob as the two nodes
        let client_alice = KeyPair::default();
        let alice = KeyPair::default();

        let client_bob = KeyPair::default();
        let bob = KeyPair::default();

        // Alice and Bob perform handshake
        let bob_encryptor = client_bob.handshake(bob.initial_public_message());
        let alice_encryptor = client_alice.handshake(alice.initial_public_message());

        // Nodes and data for packet construction
        let nodes = &[(&alice_encryptor, BOB_NODE), (&bob_encryptor, (SERVER))];
        let data = b"test data".to_vec();

        let result = onion_wrap_packet(nodes, data.clone());
        assert!(result.is_some());

        let message: MoveAlongMessage = result.unwrap();
        assert_eq!(message.next, BOB_NODE);
        let TorMessage::NotForYou { data: encrypted } = message.data else {
            panic!("Message should be Not for you");
        };

        let message: MoveAlongMessage =
            bincode::deserialize(&alice_encryptor.decrypt(&encrypted)?[..])?;
        assert_eq!(message.next, SERVER);
        let TorMessage::NotForYou { data: encrypted } = message.data else {
            panic!("Handshake?");
        };

        let final_result = bob_encryptor.decrypt(&encrypted)?;
        assert_eq!(final_result, data);

        Ok(())
    }

    #[test]
    fn test_build_handshake() -> anyhow::Result<()> {
        // Setup Alice and Bob as the two nodes
        let alice = KeyPair::default();
        let client_alice = KeyPair::default();
        let alice_encryptor = client_alice.handshake(alice.initial_public_message());

        // Nodes for handshake construction
        let nodes = &[(Some(&alice_encryptor), BOB_NODE), (None, SERVER)];

        let bob = KeyPair::default();

        let result = onion_wrap_handshake(nodes, bob.initial_public_message());

        let message: MoveAlongMessage = result.unwrap();
        assert_eq!(message.next, BOB_NODE);
        let TorMessage::NotForYou { data: encrypted } = message.data else {
            panic!("Message should be Not for you");
        };

        let message: MoveAlongMessage =
            bincode::deserialize(&alice_encryptor.decrypt(&encrypted)?[..])?;

        let TorMessage::HandShake(pubkey) = message.data else {
            panic!("Handshake?");
        };
        assert_eq!(message.next, SERVER);
        assert_eq!(pubkey, bob.initial_public_message());
        Ok(())
    }

    #[test]
    fn test_build_handshake_one_layer() {
        let nodes = &[(None, BOB_NODE), (None, SERVER)];
        let bob = KeyPair::default();

        let message = onion_wrap_handshake(nodes, bob.initial_public_message()).unwrap();

        let TorMessage::HandShake(pubkey) = message.data else {
            panic!("Handshake?");
        };
        assert_eq!(message.next, BOB_NODE);
        assert_eq!(pubkey, bob.initial_public_message());
    }
}
