use lapin_async::connection::Connection;

use commands::Command;
use error::{Error, ErrorKind};

/// command used to open a channel against RabbitMQ.
pub(crate) struct Open {
	request_id: Option<RequestId>,
}

impl Open {
	pub(crate) fn new() -> Self {
		Open {
			request_id: None,
		}
	}
}

impl Command for Open {
	fn execute(&mut self, conn: &mut Connection) -> Result<(), Error> {
		let channel_id = match conn.create_channel() {
			Some(channel_id) => channel_id,
			None => return Err(ErrorKind::ChannelLimitReached.into()),
		};
		self.request_id = Some(
			conn.channel_open(channel_id, "".into())
				.map_err(|e| ErrorKind::ProtocolError(e).into())?
		);
		Ok(())
	}

	fn has_finished(&self, conn: &mut Connection) -> bool {
		conn.is_finished(self.request_id).unwrap_or(false)
	}
}
