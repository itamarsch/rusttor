use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Serialize};
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub struct NodeIO<T, R, W> {
    inner: T,
    _phantom_data: PhantomData<(R, W)>,
}

impl<T, W, R> NodeIO<T, W, R> {
    pub fn new(inner: T) -> Self {
        NodeIO {
            inner,
            _phantom_data: PhantomData,
        }
    }
}

impl<T, R, W> NodeIO<T, R, W>
where
    T: AsyncRead + Unpin,
    R: DeserializeOwned,
{
    async fn read_length_prefixed(&mut self) -> io::Result<Vec<u8>> {
        let len = self.inner.read_u32_le().await? as usize;

        let mut buf = vec![0u8; len];
        self.inner.read_exact(&mut buf).await?;
        Ok(buf)
    }

    pub async fn read(&mut self) -> anyhow::Result<R> {
        let result = self.read_length_prefixed().await?;
        let value: R = bincode::deserialize(&result[..])?;
        Ok(value)
    }
}

impl<T, R, W> NodeIO<T, R, W>
where
    T: AsyncWrite + Unpin,
    W: Serialize,
{
    async fn write_length_prefiexed(&mut self, buf: &[u8]) -> io::Result<()> {
        let len = buf.len() as u32;
        self.inner.write_all(&len.to_le_bytes()).await?;
        self.inner.write_all(buf).await?;
        self.inner.flush().await?;
        Ok(())
    }

    pub async fn node_write(&mut self, value: W) -> anyhow::Result<()> {
        let bytes = bincode::serialize(&value)?;
        self.write_length_prefiexed(&bytes[..]).await?;
        Ok(())
    }
    pub async fn write_raw(&mut self, value: &[u8]) -> anyhow::Result<()> {
        self.inner.write_all(value).await?;
        self.inner.flush().await?;
        Ok(())
    }
}
