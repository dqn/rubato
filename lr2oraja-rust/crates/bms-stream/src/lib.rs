//! Streaming integration via IPC for external stream controllers.
//!
//! Provides [`controller::StreamController`] for bidirectional communication
//! with stream overlay software. Uses Windows named pipes (`\\.\pipe\beatoraja`)
//! on Windows and Unix domain sockets (`/tmp/beatoraja.sock`) on other platforms.
//! Commands are defined in [`command::StreamCommand`].

pub mod command;
pub mod controller;
