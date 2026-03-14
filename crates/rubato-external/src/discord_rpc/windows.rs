use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

use anyhow::Result;

use super::connection::IPCConnection;

/// Translates: WindowsIPCConnection.java
///
/// On Windows, opens the Discord named pipe via std::fs::File.
/// OpenOptions::new().read(true).write(true) maps to:
///   CreateFile(path, GENERIC_READ | GENERIC_WRITE, 0, null, OPEN_EXISTING, 0, null)
/// On non-Windows platforms, the pipe path does not exist so connect() will fail at runtime.
const PIPE_PATH: &str = r"\\.\pipe\discord-ipc-0";

/// Read/write timeout in milliseconds for named pipe I/O.
/// Matches the 100ms timeout used by UnixIPCConnection to prevent blocking
/// the render thread when Discord IPC is slow or hung.
const PIPE_TIMEOUT_MS: u32 = 100;

pub struct WindowsIPCConnection {
    file: Option<File>,
}

impl WindowsIPCConnection {
    pub fn new() -> Self {
        WindowsIPCConnection { file: None }
    }
}

impl Default for WindowsIPCConnection {
    fn default() -> Self {
        Self::new()
    }
}

/// Set read/write timeouts on a Windows named pipe handle.
/// Uses SetNamedPipeHandleState to configure PIPE_TIMEOUT_MS timeout.
#[cfg(windows)]
fn set_pipe_timeouts(file: &File) -> Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Pipes::PIPE_WAIT;
    use windows_sys::Win32::System::Pipes::SetNamedPipeHandleState;

    let handle = file.as_raw_handle() as isize;
    let mut mode: u32 = PIPE_WAIT;
    let mut timeout: u32 = PIPE_TIMEOUT_MS;
    let ret = unsafe {
        SetNamedPipeHandleState(
            handle,
            &mut mode,
            std::ptr::null_mut(),
            &mut timeout as *mut u32 as *mut i32,
        )
    };
    if ret == 0 {
        anyhow::bail!(
            "SetNamedPipeHandleState failed: {}",
            std::io::Error::last_os_error()
        );
    }
    Ok(())
}

/// No-op on non-Windows platforms (pipe timeout is set via UnixStream timeouts).
#[cfg(not(windows))]
fn set_pipe_timeouts(_file: &File) -> Result<()> {
    Ok(())
}

impl IPCConnection for WindowsIPCConnection {
    /// Connect to the Discord IPC named pipe.
    /// Translated from: WindowsIPCConnection.connect()
    fn connect(&mut self) -> Result<()> {
        let file = OpenOptions::new().read(true).write(true).open(PIPE_PATH)?;
        // Set timeouts to prevent blocking the render thread if Discord
        // IPC is slow or hung (matching Unix's 100ms timeout).
        if let Err(e) = set_pipe_timeouts(&file) {
            log::warn!("Failed to set pipe timeouts: {}", e);
        }
        self.file = Some(file);
        Ok(())
    }

    /// Write data to the named pipe.
    /// Translated from: WindowsIPCConnection.write()
    fn write(&mut self, buffer: &[u8]) -> Result<()> {
        if let Some(ref mut file) = self.file {
            file.write_all(buffer)?;
        }
        Ok(())
    }

    /// Read data from the named pipe.
    /// Translated from: WindowsIPCConnection.read()
    fn read(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; size];
        if let Some(ref mut file) = self.file {
            file.read_exact(&mut buffer)?;
        }
        Ok(buffer)
    }

    /// Close the named pipe connection.
    /// Translated from: WindowsIPCConnection.close()
    fn close(&mut self) {
        if let Some(file) = self.file.take() {
            drop(file);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let _conn = WindowsIPCConnection::new();
    }

    #[test]
    fn test_default() {
        let _conn = WindowsIPCConnection::default();
    }

    #[test]
    fn test_connect_fails_without_discord() {
        // Named pipe does not exist unless Discord is running (or not on Windows)
        let mut conn = WindowsIPCConnection::new();
        let result = conn.connect();
        assert!(result.is_err());
    }

    #[test]
    fn test_close_no_panic() {
        let mut conn = WindowsIPCConnection::new();
        conn.close();
    }
}
