use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub struct LengthPrefixedIO<T> {
    inner: T,
}

impl<T> LengthPrefixedIO<T> {
    pub fn new(inner: T) -> Self {
        LengthPrefixedIO { inner }
    }
}

impl<T> LengthPrefixedIO<T>
where
    T: AsyncRead + Unpin,
{
    pub async fn read_message(&mut self) -> io::Result<Vec<u8>> {
        let len = self.inner.read_u32().await? as usize;

        let mut buf = vec![0u8; len];
        self.inner.read_exact(&mut buf).await?;
        Ok(buf)
    }
}

impl<T> LengthPrefixedIO<T>
where
    T: AsyncWrite + Unpin,
{
    pub async fn write_message(&mut self, buf: &[u8]) -> io::Result<()> {
        let len = buf.len() as u32;
        self.inner.write_all(&len.to_le_bytes()).await?;
        self.inner.write_all(buf).await?;
        self.inner.flush().await?;
        Ok(())
    }
}
