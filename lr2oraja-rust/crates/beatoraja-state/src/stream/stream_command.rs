/// Stream command trait
/// Translates: bms.player.beatoraja.stream.command.StreamCommand
///
/// Java:
/// ```java
/// public abstract class StreamCommand {
///     public String COMMAND_STRING;
///     abstract public void run(String data);
///     abstract public void dispose();
/// }
/// ```
pub trait StreamCommand: Send {
    /// The command string that triggers this command (e.g. "!!req")
    fn command_string(&self) -> &str;

    /// Execute the command with the given data
    fn run(&mut self, data: &str);

    /// Dispose of resources
    fn dispose(&mut self);
}
