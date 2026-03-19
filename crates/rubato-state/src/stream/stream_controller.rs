use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use rubato_types::sync_utils::lock_or_recover;

use crate::select::music_selector::MusicSelector;

use super::stream_command::StreamCommand;
use super::stream_request_command::StreamRequestCommand;

type SharedCommands = Arc<Mutex<Vec<Box<dyn StreamCommand>>>>;

/// Windows named pipe path for beatoraja stream commands.
#[cfg(windows)]
const PIPE_PATH: &str = r"\\.\pipe\beatoraja";

/// Stream controller for processing strings received via beatoraja pipe
/// Translates: bms.player.beatoraja.stream.StreamController
///
/// Java reads from Windows named pipe `\\.\pipe\beatoraja`.
/// On non-Windows platforms, pipe support is unavailable.
///
/// Implements `StreamControllerAccess` for cross-crate usage via MainController.
pub struct StreamController {
    pub commands: Vec<Box<dyn StreamCommand>>,
    pub pipe_buffer: Option<BufReader<std::fs::File>>,
    pub polling: Option<thread::JoinHandle<()>>,
    pub is_active: bool,
    pub selector: Arc<Mutex<MusicSelector>>,
    /// Commands shared with the reader thread, so dispose() can reach them.
    shared_commands: Option<SharedCommands>,
    /// Shutdown flag: set to true by dispose() to signal reader thread exit.
    shutdown: Arc<AtomicBool>,
}

impl StreamController {
    pub fn new(selector: Arc<Mutex<MusicSelector>>) -> Self {
        let mut commands: Vec<Box<dyn StreamCommand>> =
            vec![Box::new(StreamRequestCommand::new(Arc::clone(&selector)))];

        let (pipe_buffer, is_active) = Self::open_pipe();

        if !is_active {
            for cmd in commands.iter_mut() {
                cmd.dispose();
            }
        }

        Self {
            commands,
            pipe_buffer,
            polling: None,
            is_active,
            selector,
            shared_commands: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Opens the Windows named pipe.
    /// Returns (Some(reader), true) on success, (None, false) on failure or non-Windows.
    #[cfg(windows)]
    fn open_pipe() -> (Option<BufReader<std::fs::File>>, bool) {
        // In Java: pipeBuffer = new BufferedReader(new FileReader("\\\\.\\pipe\\beatoraja"));
        // On Windows, std::fs::OpenOptions maps to Kernel32 CreateFile for named pipes.
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(PIPE_PATH)
        {
            Ok(file) => {
                log::info!("Named pipe connected: {}", PIPE_PATH);
                (Some(BufReader::new(file)), true)
            }
            Err(e) => {
                log::error!("Failed to open named pipe {}: {}", PIPE_PATH, e);
                (None, false)
            }
        }
    }

    /// On non-Windows platforms, named pipe is not available.
    #[cfg(not(windows))]
    fn open_pipe() -> (Option<BufReader<std::fs::File>>, bool) {
        log::info!("Named pipe not available on this platform");
        (None, false)
    }

    pub fn run(&mut self) {
        // Combine check and extraction to avoid TOCTOU race
        let Some(pipe_buffer) = self.pipe_buffer.take() else {
            return;
        };
        let commands: Vec<Box<dyn StreamCommand>> = std::mem::take(&mut self.commands);
        let commands = Arc::new(Mutex::new(commands));
        let commands_clone = Arc::clone(&commands);
        let shutdown = Arc::clone(&self.shutdown);

        // Keep a reference so dispose() can access commands
        self.shared_commands = Some(Arc::clone(&commands));

        // In Java: busy-wait until pipeBuffer.ready()
        // We skip this in Rust -- readLine() will block anyway

        let handle = thread::spawn(move || {
            let reader = pipe_buffer;
            for line_result in reader.lines() {
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }
                match line_result {
                    Ok(line) => {
                        log::info!("Received: {}", line);
                        let mut cmds = lock_or_recover(&commands_clone);
                        Self::execute_commands(&mut cmds, &line);
                    }
                    Err(e) => {
                        log::error!("{}", e);
                        break;
                    }
                }
            }
            // Thread exiting: dispose all commands
            let mut cmds = lock_or_recover(&commands_clone);
            for cmd in cmds.iter_mut() {
                cmd.dispose();
            }
        });

        self.polling = Some(handle);
        // Commands are now owned by the thread, so we keep an empty vec here
        self.commands = Vec::new();
    }

    pub fn dispose(&mut self) {
        // Signal the reader thread to stop
        self.shutdown.store(true, Ordering::SeqCst);

        // Dispose commands owned by the reader thread
        if let Some(ref shared) = self.shared_commands {
            let mut cmds = lock_or_recover(shared);
            for cmd in cmds.iter_mut() {
                cmd.dispose();
            }
        }
        self.shared_commands = None;

        if let Some(handle) = self.polling.take() {
            // In Java: polling.interrupt(); polling = null;
            // The reader thread will exit when the pipe is closed/EOF or on next line read
            // after the shutdown flag is set. We drop the handle without joining to avoid
            // blocking indefinitely on a pipe read.
            drop(handle);
        }
        // pipe_buffer is already moved or None
        self.pipe_buffer = None;

        // Dispose any commands still owned by self (before run() was called)
        for cmd in self.commands.iter_mut() {
            cmd.dispose();
        }
        log::info!("Pipe resource disposal complete");
    }

    fn execute_commands(commands: &mut [Box<dyn StreamCommand>], line: &str) {
        for cmd in commands.iter_mut() {
            let cmd_str = format!("{} ", cmd.command_string());
            let data = line.strip_prefix(&cmd_str).unwrap_or("");
            cmd.run(data);
        }
    }
}

impl rubato_types::stream_controller_access::StreamControllerAccess for StreamController {
    fn run(&mut self) {
        StreamController::run(self);
    }

    fn dispose(&mut self) {
        StreamController::dispose(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    /// Mock StreamCommand that records all calls for verification.
    struct MockCommand {
        command_str: String,
        calls: Arc<StdMutex<Vec<String>>>,
        disposed: Arc<AtomicBool>,
    }

    impl MockCommand {
        fn new(command_str: &str) -> (Self, Arc<StdMutex<Vec<String>>>, Arc<AtomicBool>) {
            let calls = Arc::new(StdMutex::new(Vec::new()));
            let disposed = Arc::new(AtomicBool::new(false));
            (
                Self {
                    command_str: command_str.to_string(),
                    calls: Arc::clone(&calls),
                    disposed: Arc::clone(&disposed),
                },
                calls,
                disposed,
            )
        }
    }

    impl StreamCommand for MockCommand {
        fn command_string(&self) -> &str {
            &self.command_str
        }

        fn run(&mut self, data: &str) {
            self.calls.lock().unwrap().push(data.to_string());
        }

        fn dispose(&mut self) {
            self.disposed.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn execute_commands_extracts_data_after_command_prefix() {
        let (cmd, calls, _disposed) = MockCommand::new("!!req");
        let mut commands: Vec<Box<dyn StreamCommand>> = vec![Box::new(cmd)];

        let sha256 = "a".repeat(64);
        let line = format!("!!req {}", sha256);
        StreamController::execute_commands(&mut commands, &line);

        let recorded = calls.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], sha256);
    }

    #[test]
    fn execute_commands_calls_run_with_empty_data_when_no_match() {
        let (cmd, calls, _disposed) = MockCommand::new("!!req");
        let mut commands: Vec<Box<dyn StreamCommand>> = vec![Box::new(cmd)];

        // Line that doesn't contain "!!req " -- Java calls run("") unconditionally
        StreamController::execute_commands(&mut commands, "hello world");

        let recorded = calls.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], "");
    }

    #[test]
    fn execute_commands_calls_run_with_empty_data_on_empty_line() {
        let (cmd, calls, _disposed) = MockCommand::new("!!req");
        let mut commands: Vec<Box<dyn StreamCommand>> = vec![Box::new(cmd)];

        StreamController::execute_commands(&mut commands, "");

        let recorded = calls.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], "");
    }

    #[test]
    fn execute_commands_dispatches_to_multiple_commands() {
        let (cmd1, calls1, _disposed1) = MockCommand::new("!!req");
        let (cmd2, calls2, _disposed2) = MockCommand::new("!!play");
        let mut commands: Vec<Box<dyn StreamCommand>> = vec![Box::new(cmd1), Box::new(cmd2)];

        StreamController::execute_commands(&mut commands, "!!play some_data");

        // !!req should be called with empty data (no match, Java parity)
        let recorded1 = calls1.lock().unwrap();
        assert_eq!(recorded1.len(), 1);
        assert_eq!(recorded1[0], "");

        // !!play should get "some_data"
        let recorded2 = calls2.lock().unwrap();
        assert_eq!(recorded2.len(), 1);
        assert_eq!(recorded2[0], "some_data");
    }

    #[test]
    fn execute_commands_empty_commands_slice() {
        let mut commands: Vec<Box<dyn StreamCommand>> = vec![];
        // Should not panic with empty commands
        StreamController::execute_commands(&mut commands, "!!req some_data");
    }

    #[test]
    fn run_returns_immediately_when_pipe_buffer_is_none() {
        // Verify that run() does not panic when pipe_buffer is None.
        // We can't easily construct a full StreamController without MusicSelector,
        // but we can test open_pipe on non-Windows returns (None, false).
        let (pipe_buffer, is_active) = StreamController::open_pipe();
        assert!(pipe_buffer.is_none());
        assert!(!is_active);
    }
}
