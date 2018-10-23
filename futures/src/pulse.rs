use futures::{task, sink, Async, Future, Poll, Sink, Stream};
use futures::sync::mpsc;
use std::time::{Instant, Duration};
use tokio_timer::Interval;

use commands::{self, Command};
use error::{Error, ErrorKind};

/// A future that sends a heartbeat frame at the given interval.
#[must_use = "futures do nothing unless polled"]
pub(crate) struct Pulse {
    interval: Interval,
    chan: mpsc::Sender<Box<dyn Command>>,
    task: Option<sink::Send<mpsc::Sender<Box<dyn Command>>>>
}

impl Pulse {
    /// Create a new `Pulse` future instance.
    pub(crate) fn new(interval: Duration, chan: mpsc::Sender<Box<dyn Command>>) -> Self {
        let interval = Interval::new(Instant::now(), interval);
        Pulse {
            interval,
            chan,
            task: None,
        }
    }
}

impl Future for Pulse {
    type Item = ();

    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if self.task.is_some() {
            let mut task = self.task.take().unwrap();
            match task.poll() {
                Ok(Async::NotReady) => {
                    self.task = Some(task);
                    return Ok(Async::NotReady)
                },
                Ok(Async::Ready(_)) => (),
                Err(_) => error!("Couldn't send the heartbeat to the background task"),
            };
        }
        match self.interval.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(Some(_))) => {
                let heartbeat = commands::heartbeat::Heartbeat::new();
                self.task = Some(self.chan.clone().send(Box::new(heartbeat)));
                Ok(Async::NotReady)
            },
            Ok(Async::Ready(None)) => Ok(Async::Ready(())),
            Err(e) => if e.is_at_capacity() {
                task::current().notify();
                Ok(Async::NotReady)
            } else {
                error!("The timer instance has been dropped.");
                Err(ErrorKind::TimerDropped.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_support::*;

    use tokio::runtime::current_thread::Runtime;

    #[test]
    fn test_pulse_sending_message_at_configured_interval() {
        mocked(|timer, time| {
            let (tx, mut rx) = mpsc::channel(16);
            let duration = Duration::from_secs(60);
            let interval = Interval::new(time.now(), duration);
            let mut pulse = Pulse {
                interval,
                chan: tx,
                task: None,
            };

            // Enqueue sending task
            assert_not_ready!(pulse);
            assert!(pulse.task.is_some());
            assert_not_ready!(rx);

            // Pulse should have send command to channel
            assert_not_ready!(pulse);
            assert_ready_eq!(rx, Some(()));

            // Should not enqueue task if called before interval duration
            assert_not_ready!(pulse);
            assert!(pulse.task.is_none());

            // Advance by interval duration
            advance(timer, duration);

            // Enqueue sending task
            assert_not_ready!(pulse);
            assert!(pulse.task.is_some());
            assert_not_ready!(rx);

            // Pulse should have send command to channel
            assert_not_ready!(pulse);
            assert_ready_eq!(rx, Some(()));
        });
    }
}
