use anyhow::Result;

/// Translates: IPCConnection.java
/// ```java
/// interface IPCConnection {
///     void connect() throws IOException;
///     void write(ByteBuffer buffer) throws IOException;
///     ByteBuffer read(int size) throws IOException;
///     void close();
/// }
/// ```
pub trait IPCConnection: Send {
    fn connect(&mut self) -> Result<()>;
    fn write(&mut self, buffer: &[u8]) -> Result<()>;
    fn read(&mut self, size: usize) -> Result<Vec<u8>>;
    fn close(&mut self);
}
