use futures::{Async, Future, Poll};

use commands::Command;

pub(crate) struct IssueFuture<C>
where
	C: Command
{
	command: C,
}

impl<C> IssueFuture<C>
where
	C: Command
{
	pub(crate) fn new(command: C) -> Self {
		IssueFuture {
			command,
		}
	}
}

impl<C> Future for IssueFuture<C>
where
	C: Command
{
	fn poll(&mut self) -> Poll<Self::Item, Self::Error> {}
}
