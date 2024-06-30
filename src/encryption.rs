use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use anyhow::Result;
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use x25519_dalek::{EphemeralSecret, PublicKey};

const NONCE_LENGTH: usize = 12;

struct KeyPair {
    secret: EphemeralSecret,
    public: PublicKey,
}

impl KeyPair {
    pub fn new() -> Self {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    pub fn initial_public_message(&self) -> &[u8; 32] {
        self.public.as_bytes()
    }
}

pub struct Encryptor {
    cipher: Aes256Gcm,
}

impl Encryptor {
    pub fn new(other_publickey: &PublicKey, keypair: KeyPair) -> Self {
        let shared = keypair.secret.diffie_hellman(other_publickey);
        let shared_secret = shared.as_bytes();

        let key = Sha256::digest(shared_secret);
        let cipher = Aes256Gcm::new_from_slice(&key).expect("Key is valid");

        Self { cipher }
    }

    pub fn encrypt(&self, bytes: &[u8]) -> Vec<u8> {
        let mut nonce_bytes = [0u8; NONCE_LENGTH];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self.cipher.encrypt(nonce, bytes).expect("Leys are valid");

        let mut message = nonce_bytes.to_vec();
        message.extend(ciphertext);

        message
    }

    pub fn decrypt(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        if bytes.len() < NONCE_LENGTH {
            anyhow::bail!("Invalid message for encryption")
        }

        let nonce = Nonce::from_slice(&bytes[0..12]);
        let message = &bytes[12..];

        let result = self.cipher.decrypt(nonce, message)?;
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_to_end() -> Result<()> {
        let alice_keypair = KeyPair::new();
        let bob_keypair = KeyPair::new();
        let alice_pub = alice_keypair.public;
        let bob_pub = bob_keypair.public;

        let alice_encryptor = Encryptor::new(&bob_pub, alice_keypair);
        let bob_encryptor = Encryptor::new(&alice_pub, bob_keypair);

        let message = "Hello world!";

        let encrypted = alice_encryptor.encrypt(message.as_bytes());

        let decrypted = bob_encryptor.decrypt(encrypted.as_slice())?;

        assert_eq!(decrypted.as_slice(), message.as_bytes());
        Ok(())
    }
}
