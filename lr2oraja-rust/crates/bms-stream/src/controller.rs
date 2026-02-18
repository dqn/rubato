// Stream controller
//
// On Windows, connects to \\.\pipe\beatoraja named pipe and dispatches
// incoming lines to registered StreamCommand handlers.
// On Unix platforms, listens on a per-user Unix domain socket.

use std::sync::Arc;

use tokio::sync::watch;
use tracing::{info, warn};

use crate::command::StreamCommand;

/// Manages the named pipe connection and dispatches commands.
pub struct StreamController {
    commands: Vec<Arc<dyn StreamCommand + Send + Sync>>,
    shutdown_tx: Option<watch::Sender<bool>>,
}

impl StreamController {
    pub fn new(commands: Vec<Arc<dyn StreamCommand + Send + Sync>>) -> Self {
        Self {
            commands,
            shutdown_tx: None,
        }
    }

    /// Start the stream controller.
    ///
    /// On Windows, spawns a tokio task that reads from the named pipe.
    /// On other platforms, logs a warning and returns immediately.
    #[cfg(target_os = "windows")]
    pub fn start(&mut self) {
        let (tx, rx) = watch::channel(false);
        self.shutdown_tx = Some(tx);

        let commands = self.commands.clone();
        tokio::spawn(async move {
            if let Err(e) = run_pipe_listener(commands, rx).await {
                warn!("Stream pipe listener error: {}", e);
            }
        });

        info!("Stream controller started (Windows named pipe)");
    }

    /// Start the stream controller (Unix domain socket).
    #[cfg(not(target_os = "windows"))]
    pub fn start(&mut self) {
        let (tx, rx) = watch::channel(false);
        self.shutdown_tx = Some(tx);

        let commands = self.commands.clone();
        tokio::spawn(async move {
            if let Err(e) = run_unix_listener(commands, rx).await {
                warn!("Stream Unix listener error: {}", e);
            }
        });

        info!("Stream controller started (Unix domain socket)");
    }

    /// Stop the stream controller by sending a shutdown signal.
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
            info!("Stream controller stop signal sent");
        }
    }

    /// Return a reference to the registered commands.
    pub fn commands(&self) -> &[Arc<dyn StreamCommand + Send + Sync>] {
        &self.commands
    }
}

/// Dispatch a received line to all registered commands.
fn dispatch_line(commands: &[Arc<dyn StreamCommand + Send + Sync>], line: &str) {
    for cmd in commands {
        let prefix = cmd.command_string();
        let full_prefix = format!("{} ", prefix);
        if let Some(args) = line.strip_prefix(&full_prefix) {
            match cmd.run(args) {
                Ok(Some(response)) => {
                    info!("Command '{}' response: {}", prefix, response);
                }
                Ok(None) => {}
                Err(e) => {
                    warn!("Command '{}' error: {}", prefix, e);
                }
            }
        } else if line == prefix {
            // Command with no arguments
            match cmd.run("") {
                Ok(Some(response)) => {
                    info!("Command '{}' response: {}", prefix, response);
                }
                Ok(None) => {}
                Err(e) => {
                    warn!("Command '{}' error: {}", prefix, e);
                }
            }
        }
    }
}

/// Windows named pipe listener implementation.
#[cfg(target_os = "windows")]
async fn run_pipe_listener(
    commands: Vec<Arc<dyn StreamCommand + Send + Sync>>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    use tokio::io::AsyncBufReadExt;
    use tokio::net::windows::named_pipe::ClientOptions;

    let pipe_name = r"\\.\pipe\beatoraja";
    info!("Connecting to named pipe: {}", pipe_name);

    let pipe = ClientOptions::new().open(pipe_name)?;
    let reader = tokio::io::BufReader::new(pipe);
    let mut lines = reader.lines();

    loop {
        tokio::select! {
            result = lines.next_line() => {
                match result {
                    Ok(Some(line)) => {
                        info!("Received: {}", line);
                        dispatch_line(&commands, &line);
                    }
                    Ok(None) => {
                        info!("Named pipe closed");
                        break;
                    }
                    Err(e) => {
                        warn!("Error reading from pipe: {}", e);
                        break;
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("Shutdown signal received");
                    break;
                }
            }
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn sanitize_socket_user_suffix(user: &str) -> String {
    let mut out = String::with_capacity(user.len());
    for c in user.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "user".to_string()
    } else {
        out
    }
}

#[cfg(not(target_os = "windows"))]
fn unix_socket_path_from_parts(
    runtime_dir: Option<&std::path::Path>,
    user: Option<&str>,
) -> std::path::PathBuf {
    if let Some(dir) = runtime_dir {
        return dir.join("brs").join("beatoraja.sock");
    }

    let suffix = sanitize_socket_user_suffix(user.unwrap_or("user"));
    std::env::temp_dir().join(format!("beatoraja-{suffix}.sock"))
}

#[cfg(not(target_os = "windows"))]
fn unix_socket_path() -> std::path::PathBuf {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from);
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok();
    unix_socket_path_from_parts(runtime_dir.as_deref(), user.as_deref())
}

/// Unix domain socket listener implementation.
#[cfg(not(target_os = "windows"))]
async fn run_unix_listener(
    commands: Vec<Arc<dyn StreamCommand + Send + Sync>>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    use tokio::io::AsyncBufReadExt;
    use tokio::net::UnixListener;

    let socket_path = unix_socket_path();

    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
        // Keep runtime dir private for local IPC endpoint.
        let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
    }

    // Remove stale socket file if it exists
    let _ = std::fs::remove_file(&socket_path);

    let listener = UnixListener::bind(&socket_path)?;
    let _ = std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600));
    info!("Listening on Unix socket: {}", socket_path.display());

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => {
                        let cmds = commands.clone();
                        let mut conn_shutdown_rx = shutdown_rx.clone();
                        tokio::spawn(async move {
                            let reader = tokio::io::BufReader::new(stream);
                            let mut lines = reader.lines();
                            loop {
                                tokio::select! {
                                    line_result = lines.next_line() => {
                                        match line_result {
                                            Ok(Some(line)) => {
                                                info!("Received: {}", line);
                                                dispatch_line(&cmds, &line);
                                            }
                                            Ok(None) => break,
                                            Err(e) => {
                                                warn!("Error reading from connection: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                    _ = conn_shutdown_rx.changed() => {
                                        if *conn_shutdown_rx.borrow() {
                                            break;
                                        }
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => {
                        warn!("Error accepting connection: {}", e);
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("Shutdown signal received");
                    break;
                }
            }
        }
    }

    // Clean up socket file
    let _ = std::fs::remove_file(&socket_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::StreamRequestCommand;

    #[test]
    fn test_new_controller() {
        let cmd = Arc::new(StreamRequestCommand::default());
        let controller = StreamController::new(vec![cmd.clone()]);
        assert_eq!(controller.commands().len(), 1);
    }

    #[test]
    fn test_dispatch_line_with_args() {
        let cmd = Arc::new(StreamRequestCommand::default());
        let commands: Vec<Arc<dyn StreamCommand + Send + Sync>> = vec![cmd.clone()];

        let hash = "a".repeat(64);
        let line = format!("!!req {}", hash);
        dispatch_line(&commands, &line);

        assert_eq!(cmd.pending_count(), 1);
        let requests = cmd.poll_requests();
        assert_eq!(requests[0], hash);
    }

    #[test]
    fn test_dispatch_line_no_match() {
        let cmd = Arc::new(StreamRequestCommand::default());
        let commands: Vec<Arc<dyn StreamCommand + Send + Sync>> = vec![cmd.clone()];

        dispatch_line(&commands, "!!unknown something");
        assert_eq!(cmd.pending_count(), 0);
    }

    #[test]
    fn test_dispatch_line_no_args() {
        let cmd = Arc::new(StreamRequestCommand::default());
        let commands: Vec<Arc<dyn StreamCommand + Send + Sync>> = vec![cmd.clone()];

        // Command with no args (empty string) - hash validation will reject it
        dispatch_line(&commands, "!!req");
        assert_eq!(cmd.pending_count(), 0);
    }

    #[test]
    fn test_dispatch_multiple_commands() {
        let cmd1 = Arc::new(StreamRequestCommand::new(10));
        let cmd2 = Arc::new(StreamRequestCommand::new(10));
        let commands: Vec<Arc<dyn StreamCommand + Send + Sync>> = vec![cmd1.clone(), cmd2.clone()];

        let hash = "b".repeat(64);
        let line = format!("!!req {}", hash);
        dispatch_line(&commands, &line);

        // Both commands have the same prefix, so both receive the message
        assert_eq!(cmd1.pending_count(), 1);
        assert_eq!(cmd2.pending_count(), 1);
    }

    #[test]
    fn test_stop_without_start() {
        let cmd = Arc::new(StreamRequestCommand::default());
        let mut controller = StreamController::new(vec![cmd]);
        // Should not panic
        controller.stop();
    }

    #[test]
    fn test_dispatch_invalid_hash() {
        let cmd = Arc::new(StreamRequestCommand::default());
        let commands: Vec<Arc<dyn StreamCommand + Send + Sync>> = vec![cmd.clone()];

        dispatch_line(&commands, "!!req tooshort");
        assert_eq!(cmd.pending_count(), 0);
    }

    #[test]
    fn test_dispatch_line_extra_spaces() {
        let cmd = Arc::new(StreamRequestCommand::default());
        let commands: Vec<Arc<dyn StreamCommand + Send + Sync>> = vec![cmd.clone()];

        // Extra leading spaces in args - the command prefix match uses strip_prefix
        // so "!!req  <hash>" would have args " <hash>" with leading space,
        // which trim() in run() handles
        let hash = "c".repeat(64);
        let line = format!("!!req  {}", hash);
        dispatch_line(&commands, &line);
        // The args will be " <hash>" which trim() handles
        assert_eq!(cmd.pending_count(), 1);
    }

    #[test]
    fn test_concurrent_dispatch() {
        let cmd = Arc::new(StreamRequestCommand::new(100));
        let commands: Vec<Arc<dyn StreamCommand + Send + Sync>> = vec![cmd.clone()];

        let mut handles = Vec::new();
        for i in 0..20u64 {
            let cmds = commands.clone();
            handles.push(std::thread::spawn(move || {
                let hash = format!("{:0>64x}", i);
                let line = format!("!!req {}", hash);
                dispatch_line(&cmds, &line);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(cmd.pending_count(), 20);
        let requests = cmd.poll_requests();
        assert_eq!(requests.len(), 20);
    }

    #[test]
    fn test_controller_start_stop_cycle() {
        let cmd = Arc::new(StreamRequestCommand::default());
        let mut controller = StreamController::new(vec![cmd]);

        // stop without start - should not panic
        controller.stop();

        // Multiple stop calls should not panic
        controller.stop();
        controller.stop();
    }

    #[test]
    fn test_dispatch_empty_line() {
        let cmd = Arc::new(StreamRequestCommand::default());
        let commands: Vec<Arc<dyn StreamCommand + Send + Sync>> = vec![cmd.clone()];

        // Empty string
        dispatch_line(&commands, "");
        assert_eq!(cmd.pending_count(), 0);

        // Newline-only
        dispatch_line(&commands, "\n");
        assert_eq!(cmd.pending_count(), 0);

        // Whitespace only
        dispatch_line(&commands, "   ");
        assert_eq!(cmd.pending_count(), 0);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_unix_socket_path_prefers_runtime_dir() {
        let path =
            unix_socket_path_from_parts(Some(std::path::Path::new("/tmp/runtime")), Some("dqn"));
        assert_eq!(
            path,
            std::path::Path::new("/tmp/runtime/brs/beatoraja.sock")
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_unix_socket_path_fallback_uses_user_suffix() {
        let path = unix_socket_path_from_parts(None, Some("dqn@example"));
        let file_name = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
        assert_eq!(file_name, "beatoraja-dqn_example.sock");
    }

    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    async fn test_unix_socket_connection() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        use tokio::net::UnixStream;

        let socket_path = format!("/tmp/beatoraja_test_{}.sock", std::process::id());

        let cmd = Arc::new(StreamRequestCommand::default());
        let commands: Vec<Arc<dyn StreamCommand + Send + Sync>> = vec![cmd.clone()];

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Remove stale socket
        let _ = std::fs::remove_file(&socket_path);

        // Start listener with custom path
        let listener = tokio::net::UnixListener::bind(&socket_path).unwrap();
        let cmds = commands.clone();
        let listener_handle = tokio::spawn(async move {
            let mut rx = shutdown_rx;
            tokio::select! {
                result = listener.accept() => {
                    if let Ok((stream, _)) = result {
                        let reader = tokio::io::BufReader::new(stream);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            dispatch_line(&cmds, &line);
                        }
                    }
                }
                _ = rx.changed() => {}
            }
        });

        // Give the listener time to start
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Connect and send a command
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let hash = "d".repeat(64);
        let msg = format!("!!req {}\n", hash);
        stream.write_all(msg.as_bytes()).await.unwrap();
        stream.shutdown().await.unwrap();

        // Wait for processing
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert_eq!(cmd.pending_count(), 1);
        let requests = cmd.poll_requests();
        assert_eq!(requests[0], hash);

        // Shutdown
        let _ = shutdown_tx.send(true);
        let _ = listener_handle.await;
        let _ = std::fs::remove_file(&socket_path);
    }
}
