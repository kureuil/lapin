use futures::{Async, Future, Poll, Stream};
use futures::sync::{oneshot, mpsc};
use std::time::{Duration, Instant};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_timer::Interval;

use commands::Command;
use error::{Error, ErrorKind};
use pulse::Pulse;
use transport::AMQPTransport;

/// The background task responsible for communicating with RabbitMQ.
#[must_use = "futures do nothing unless polled"]
pub struct Background<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + 'static
{
    /// The underlying socket used to communicate with RabbitMQ.
    transport: AMQPTransport<T>,
    /// As soon as a message is sent using this channel, the task will stop.
    shutdown: oneshot::Receiver<()>,
    /// The handle to the shutdown channel, used to stop the task.
    handle: Option<BackgroundHandle>,
    /// The channel used to receive commands that should be sent to RabbitMQ.
    commands: mpsc::Receiver<Box<dyn Command>>,
    /// The channel used to send commands to the background task.
    sender: mpsc::Sender<Box<dyn Command>>,
    /// The task that sends a heartbeat command to RabbitMQ at a defined interval.
    pulse: Pulse,
}

/// The handle to the background task.
///
/// When this value is dropped (or goes out of scope), the background task is signaled to stop.
#[derive(Debug)]
pub struct BackgroundHandle(Option<oneshot::Sender<()>>);

impl<T> Background<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + 'static
{
    /// Create a new background task from a transport.
    pub(crate) fn new(transport: AMQPTransport<T>) -> Self {
        let (shutdown_tx, shutdown) = oneshot::channel();
        let handle = Some(BackgroundHandle(Some(shutdown_tx)));
        let (sender, commands) = mpsc::channel(1024);
        let interval = Interval::new(Instant::now(), Duration::from_secs(60));
        let pulse = Pulse::new(interval, sender.clone());
        Background {
            transport,
            shutdown,
            handle,
            commands,
            sender,
            pulse,
        }
    }

    /// Create a new command channel instance.
    pub(crate) fn channel(&self) -> mpsc::Sender<Box<dyn Command>> {
        self.sender.clone()
    }

    /// Get the handle for this task.
    ///
    /// Because there can only be one handle for this task, it returns an `Option`. When the handle
    /// is dropped, the task is signaled to stop.
    pub fn handle(&mut self) -> Option<BackgroundHandle> {
        self.handle.take()
    }
}

impl<T> Future for Background<T>
where
    T: AsyncRead + AsyncWrite + Send + Sync + 'static
{
    type Item = ();

    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.shutdown.poll() {
            Ok(Async::NotReady) => (),
            Ok(Async::Ready(_)) => return Ok(Async::Ready(())),
            Err(_) => return Err(ErrorKind::HandleDropped.into()),
        };
        match self.pulse.poll() {
            Ok(Async::Ready(_)) => unreachable!(),
            Ok(Async::NotReady) => (),
            Err(e) => return Err(e),
        };
        match self.commands.poll() {
            Ok(Async::Ready(Some(mut command))) => {
                // FIXME: communicate result back to caller via oneshot
                command.execute(&mut self.transport.conn).unwrap();
            },
            Ok(Async::Ready(None)) => unreachable!(),
            Ok(Async::NotReady) => (),
            Err(_) => unreachable!(),
        };
        match self.transport.poll() {
            Ok(Async::Ready(_)) => return Ok(Async::Ready(())),
            Ok(Async::NotReady) => (),
            Err(e) => return Err(ErrorKind::Transport(e).into()),
        };
        Ok(Async::NotReady)
    }
}

impl Drop for BackgroundHandle {
    fn drop(&mut self) {
        let chan = self.0.take().expect("cannot be dropped twice");
        match chan.send(()) {
            Ok(_) => debug!("Signaling heartbeat task to stop"),
            Err(_) => warn!("Heartbeat task already shut down"),
        };
    }
}
