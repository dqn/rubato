use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

use super::connection::IPCConnection;
use anyhow::Result;

/// Translates: UnixIPCConnection.java
///
/// ```java
/// class UnixIPCConnection implements IPCConnection {
///     private SocketChannel socket;
/// ```
pub struct UnixIPCConnection {
    socket: Option<UnixStream>,
}

impl UnixIPCConnection {
    pub fn new() -> Self {
        UnixIPCConnection { socket: None }
    }
}

impl Default for UnixIPCConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl IPCConnection for UnixIPCConnection {
    /// Translates:
    /// ```java
    /// public void connect() throws IOException {
    ///     String[] envVars = {"XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"};
    ///     String basePath = null;
    ///
    ///     for (String envVar : envVars) {
    ///         basePath = System.getenv(envVar);
    ///         if (basePath != null) break;
    ///     }
    ///
    ///     String ipcPath = (basePath != null ? basePath : "/tmp") + "/discord-ipc-0";
    ///     socket = SocketChannel.open(StandardProtocolFamily.UNIX);
    ///     socket.connect(UnixDomainSocketAddress.of(Path.of(ipcPath)));
    /// }
    /// ```
    fn connect(&mut self) -> Result<()> {
        let env_vars = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"];
        let mut base_path: Option<String> = None;

        for env_var in env_vars {
            base_path = std::env::var(env_var).ok();
            if base_path.is_some() {
                break;
            }
        }

        let ipc_path = format!(
            "{}/discord-ipc-0",
            if let Some(ref bp) = base_path {
                bp.as_str()
            } else {
                "/tmp"
            }
        );
        let stream = UnixStream::connect(&ipc_path)?;
        self.socket = Some(stream);
        Ok(())
    }

    /// Translates:
    /// ```java
    /// public void write(ByteBuffer buffer) throws IOException {
    ///     while (buffer.hasRemaining()) {
    ///         socket.write(buffer);
    ///     }
    /// }
    /// ```
    fn write(&mut self, buffer: &[u8]) -> Result<()> {
        if let Some(ref mut socket) = self.socket {
            socket.write_all(buffer)?;
        }
        Ok(())
    }

    /// Translates:
    /// ```java
    /// public ByteBuffer read(int size) throws IOException {
    ///     ByteBuffer buffer = ByteBuffer.allocate(size);
    ///     while (buffer.hasRemaining()) {
    ///         socket.read(buffer);
    ///     }
    ///     buffer.flip();
    ///     return buffer;
    /// }
    /// ```
    fn read(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; size];
        if let Some(ref mut socket) = self.socket {
            socket.read_exact(&mut buffer)?;
        }
        Ok(buffer)
    }

    /// Translates:
    /// ```java
    /// public void close() {
    ///     if (socket != null) {
    ///         try {
    ///             socket.close();
    ///         } catch (IOException e) {
    ///             logger.warn("Failed to close Unix socket: {}", e.getMessage());
    ///         }
    ///     }
    /// }
    /// ```
    fn close(&mut self) {
        if let Some(socket) = self.socket.take() {
            // UnixStream::drop handles close.
            // Java version catches IOException from socket.close() and logs a warning.
            // In Rust, Drop for UnixStream is infallible, so we just drop it.
            drop(socket);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let conn = UnixIPCConnection::new();
        assert!(conn.socket.is_none());
    }

    #[test]
    fn test_default() {
        let conn = UnixIPCConnection::default();
        assert!(conn.socket.is_none());
    }

    #[test]
    fn test_close_when_not_connected() {
        let mut conn = UnixIPCConnection::new();
        // Should not panic when closing without a connection
        conn.close();
        assert!(conn.socket.is_none());
    }
}
