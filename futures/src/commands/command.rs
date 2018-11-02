use lapin_async::connection::Connection;
use std::fmt;

use error::Error;

/// A command that can be sent to RabbitMQ.
pub(crate) trait Command: fmt::Debug + Send {
    /// Executes the command on the given protocol state machine.
    fn execute(&mut self, conn: &mut Connection) -> Result<(), Error>;

    /// Determines whether the request has finished.
    fn has_finished(&self, conn: &mut Connection) -> bool;
}
