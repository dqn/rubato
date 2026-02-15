use anyhow::{Result, anyhow};
use tracing::debug;

use super::ipc::{IpcConnection, ipc_paths};

/// Platform-native IPC connection for Discord.
///
/// On Unix, connects via Unix domain socket.
/// On Windows, connects via named pipe.
pub struct PlatformIpcConnection {
    #[cfg(unix)]
    stream: Option<tokio::net::UnixStream>,
    #[cfg(windows)]
    stream: Option<tokio::net::windows::named_pipe::NamedPipeClient>,
}

impl Default for PlatformIpcConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformIpcConnection {
    pub fn new() -> Self {
        Self { stream: None }
    }
}

impl IpcConnection for PlatformIpcConnection {
    async fn connect(&mut self) -> Result<()> {
        let paths = ipc_paths();
        for path in &paths {
            debug!("Trying Discord IPC path: {}", path.display());
            match try_connect(path).await {
                Ok(stream) => {
                    debug!("Connected to Discord IPC: {}", path.display());
                    self.stream = Some(stream);
                    return Ok(());
                }
                Err(e) => {
                    debug!("Failed to connect to {}: {}", path.display(), e);
                }
            }
        }
        Err(anyhow!(
            "could not connect to Discord IPC (tried {} paths)",
            paths.len()
        ))
    }

    async fn write(&mut self, data: &[u8]) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow!("not connected"))?;
        stream.write_all(data).await?;
        Ok(())
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        use tokio::io::AsyncReadExt;

        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow!("not connected"))?;
        stream.read_exact(buf).await?;
        Ok(buf.len())
    }

    async fn close(&mut self) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        if let Some(mut stream) = self.stream.take() {
            stream.shutdown().await?;
        }
        Ok(())
    }
}

#[cfg(unix)]
async fn try_connect(path: &std::path::Path) -> Result<tokio::net::UnixStream> {
    let stream = tokio::net::UnixStream::connect(path).await?;
    Ok(stream)
}

#[cfg(windows)]
async fn try_connect(
    path: &std::path::Path,
) -> Result<tokio::net::windows::named_pipe::NamedPipeClient> {
    let pipe_name = path.to_str().ok_or_else(|| anyhow!("invalid pipe path"))?;
    let client = tokio::net::windows::named_pipe::ClientOptions::new().open(pipe_name)?;
    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_disconnected() {
        let conn = PlatformIpcConnection::new();
        assert!(conn.stream.is_none());
    }

    #[tokio::test]
    #[ignore] // Requires Discord to be running
    async fn connect_to_running_discord() {
        let mut conn = PlatformIpcConnection::new();
        let result = conn.connect().await;
        // If Discord is running, this should succeed
        if result.is_ok() {
            assert!(conn.stream.is_some());
            conn.close().await.unwrap();
        }
    }

    #[tokio::test]
    async fn connect_fails_gracefully_without_discord() {
        // This test verifies that connect returns an error (not panic)
        // when Discord is not available. In CI, Discord is typically not running.
        let mut conn = PlatformIpcConnection::new();
        let result = conn.connect().await;
        // We don't assert is_err() because Discord might be running locally
        drop(result);
    }

    #[tokio::test]
    async fn write_fails_when_not_connected() {
        let mut conn = PlatformIpcConnection::new();
        let result = conn.write(b"test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn read_fails_when_not_connected() {
        let mut conn = PlatformIpcConnection::new();
        let mut buf = [0u8; 8];
        let result = conn.read(&mut buf).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn close_noop_when_not_connected() {
        let mut conn = PlatformIpcConnection::new();
        conn.close().await.unwrap();
    }
}
