// Streaming integration
//
// Provides IPC for stream request handling.
// On Windows, uses named pipe (\\.\pipe\beatoraja).
// On Unix, uses domain socket (/tmp/beatoraja.sock).

pub mod command;
pub mod controller;
