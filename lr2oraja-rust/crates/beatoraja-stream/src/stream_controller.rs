use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::thread;

use beatoraja_select::music_selector::MusicSelector;

use crate::stream_command::StreamCommand;
use crate::stream_request_command::StreamRequestCommand;

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
        if self.pipe_buffer.is_none() {
            return;
        }

        // Move pipe_buffer out for the thread
        let pipe_buffer = self.pipe_buffer.take().unwrap();
        let commands: Vec<Box<dyn StreamCommand>> = std::mem::take(&mut self.commands);
        let commands = Arc::new(Mutex::new(commands));
        let commands_clone = Arc::clone(&commands);

        // In Java: busy-wait until pipeBuffer.ready()
        // We skip this in Rust — readLine() will block anyway

        let handle = thread::spawn(move || {
            let reader = pipe_buffer;
            for line_result in reader.lines() {
                match line_result {
                    Ok(line) => {
                        log::info!("Received: {}", line);
                        let mut cmds = commands_clone.lock().unwrap();
                        Self::execute_commands(&mut cmds, &line);
                    }
                    Err(e) => {
                        log::error!("{}", e);
                        break;
                    }
                }
            }
        });

        self.polling = Some(handle);
        // Commands are now owned by the thread, so we keep an empty vec here
        self.commands = Vec::new();
    }

    pub fn dispose(&mut self) {
        if let Some(handle) = self.polling.take() {
            // In Java: polling.interrupt(); polling = null;
            drop(handle);
        }
        // pipe_buffer is already moved or None
        self.pipe_buffer = None;

        for cmd in self.commands.iter_mut() {
            cmd.dispose();
        }
        log::info!("Pipe resource disposal complete");
    }

    fn execute_commands(commands: &mut [Box<dyn StreamCommand>], line: &str) {
        for i in 0..commands.len() {
            let cmd_str = format!("{} ", commands[i].command_string());
            let split_line: Vec<&str> = line.split(&cmd_str).collect();
            let data = if split_line.len() == 2 {
                split_line[1]
            } else {
                ""
            };
            commands[i].run(data);
        }
    }
}

impl beatoraja_types::stream_controller_access::StreamControllerAccess for StreamController {
    fn run(&mut self) {
        StreamController::run(self);
    }

    fn dispose(&mut self) {
        StreamController::dispose(self);
    }
}
