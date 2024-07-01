use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};
use x25519_dalek::PublicKey;

use crate::encryption::{Encryptor, KeyPair};

pub struct NetworkHandler<T> {
    reader: ReadHalf<T>,
    writer: WriteHalf<T>,
    encryptor: Encryptor,
}

impl<T> NetworkHandler<T>
where
    T: AsyncWrite + AsyncRead,
{
    pub async fn perform_handshake(
        mut reader: ReadHalf<T>,
        mut writer: WriteHalf<T>,
    ) -> anyhow::Result<NetworkHandler<T>> {
        let encryptor = KeyPair::default();
        writer.write_all(encryptor.initial_public_message()).await?;

        let mut pubkey = [0; 32];
        reader.read_exact(&mut pubkey).await?;
        let other_publickey = PublicKey::from(pubkey);

        let encryptor = encryptor.handshake(&other_publickey);
        Ok(NetworkHandler {
            reader,
            writer,
            encryptor,
        })
    }

    pub async fn write_buf_encrypt(&mut self, buf: &[u8]) -> anyhow::Result<()> {
        let encrypted_message = self.encryptor.encrypt(buf);

        let len_bytes = (encrypted_message.len() as u32).to_le_bytes();
        let mut buf = Vec::with_capacity(len_bytes.len() + encrypted_message.len());

        buf.extend_from_slice(&len_bytes);
        buf.extend_from_slice(&encrypted_message);

        self.writer.write_all(&buf).await?;
        Ok(())
    }

    pub async fn read_buf_decrypt(&mut self) -> anyhow::Result<Vec<u8>> {
        let len = self.reader.read_u32_le().await? as usize;

        let mut buf = vec![0; len];
        self.reader.read_exact(&mut buf[..]).await?;

        let message = self.encryptor.decrypt(&buf)?;
        Ok(message)
    }
}
