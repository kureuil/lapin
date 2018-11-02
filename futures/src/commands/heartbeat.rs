use amq_protocol::frame::AMQPFrame;
use lapin_async::connection::Connection;

use commands::Command;
use error::Error;

/// Heartbeat command.
///
/// Used to signal that the current peer is still alive to the RabbitMQ server.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Heartbeat;

impl Heartbeat {
	/// Create a new Heartbeat instance.
	pub(crate) fn new() -> Self { Heartbeat }
}

impl Command for Heartbeat {
	fn execute(&mut self, conn: &mut Connection) -> Result<(), Error> {
		conn.frame_queue.push_back(AMQPFrame::Heartbeat(0));
		Ok(())
	}

	fn has_finished(&self, _conn: &mut Connection) -> bool { true }
}
