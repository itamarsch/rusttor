use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use anyhow::Result;
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use x25519_dalek::{EphemeralSecret, PublicKey};

const NONCE_LENGTH: usize = 12;

pub struct KeyPair {
    secret: EphemeralSecret,
    public: PublicKey,
}

impl KeyPair {
    pub fn initial_public_message(&self) -> &[u8; 32] {
        self.public.as_bytes()
    }
    pub fn handshake(self, other_publickey: &PublicKey) -> Encryptor {
        let shared = self.secret.diffie_hellman(other_publickey);
        let shared_secret = shared.as_bytes();

        let key = Sha256::digest(shared_secret);
        let cipher = Aes256Gcm::new_from_slice(&key).expect("Key is valid");

        Encryptor { cipher }
    }
}

impl Default for KeyPair {
    fn default() -> Self {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }
}

pub struct Encryptor {
    cipher: Aes256Gcm,
}

impl Encryptor {
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

        let nonce = Nonce::from_slice(&bytes[0..NONCE_LENGTH]);
        let message = &bytes[NONCE_LENGTH..];

        let result = self.cipher.decrypt(nonce, message)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_to_end() -> Result<()> {
        let alice = KeyPair::default();
        let bob = KeyPair::default();
        let alice_pub = alice.public;
        let bob_pub = bob.public;

        let alice = alice.handshake(&bob_pub);
        let bob = bob.handshake(&alice_pub);

        let message = "Hello world!";

        let encrypted = alice.encrypt(message.as_bytes());

        let decrypted = bob.decrypt(encrypted.as_slice())?;

        assert_eq!(decrypted.as_slice(), message.as_bytes());
        Ok(())
    }
}
