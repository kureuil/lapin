use std::error::Error as StdError;
use std::fmt;
use std::io;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Debug)]
pub(crate) enum ErrorKind {
    HandleDropped,
    TimerDropped,
    Transport(io::Error),
    ChannelLimitReached,
    ProtocolError(lapin_async::error::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl StdError for Error {}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error { kind }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::HandleDropped => write!(f, "the handle to the background task was dropped without signaling it to stop"),
            ErrorKind::TimerDropped => write!(f, "the timer used for the heartbeat has been dropped"),
            ErrorKind::Transport(e) => write!(f, "an error occured in the transport: {}", e),
            ErrorKind::ChannelLimitReached => write!(f, "open channel limit reached"),
            ErrorKind::ProtocolError(e) => write!(f, "a protocol error occured: {:?}", e),
        }
    }
}
