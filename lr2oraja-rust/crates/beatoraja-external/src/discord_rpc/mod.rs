pub mod connection;
pub mod rich_presence;

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

pub use connection::IPCConnection;
pub use rich_presence::*;
