use amq_protocol::uri::AMQPUri;
use lapin_async;
use std::default::Default;
use std::io;
use std::str::FromStr;
use futures::{future, Future};
use futures::sync::mpsc;
use tokio_io::{AsyncRead, AsyncWrite};

use transport::*;
use background::Background;
use channel::{Channel, ConfirmSelectOptions};
use commands::Command;

/// the Client structures connects to a server and creates channels
#[derive(Clone)]
pub struct Client {
    channel:           mpsc::Sender<Box<dyn Command>>,
    pub configuration: ConnectionConfiguration,
}

#[derive(Clone,Debug,PartialEq)]
pub struct ConnectionOptions {
    pub username:  String,
    pub password:  String,
    pub vhost:     String,
    pub frame_max: u32,
    pub heartbeat: u16,
}

impl ConnectionOptions {
    pub fn from_uri(uri: AMQPUri) -> ConnectionOptions {
        ConnectionOptions {
            username: uri.authority.userinfo.username,
            password: uri.authority.userinfo.password,
            vhost: uri.vhost,
            frame_max: uri.query.frame_max.unwrap_or(0),
            heartbeat: uri.query.heartbeat.unwrap_or(0),
        }
    }
}

impl Default for ConnectionOptions {
    fn default() -> ConnectionOptions {
        ConnectionOptions {
            username:  "guest".to_string(),
            password:  "guest".to_string(),
            vhost:     "/".to_string(),
            frame_max: 0,
            heartbeat: 0,
        }
    }
}

impl FromStr for ConnectionOptions {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uri = AMQPUri::from_str(s)?;
        Ok(ConnectionOptions::from_uri(uri))
    }
}

pub type ConnectionConfiguration = lapin_async::connection::Configuration;

impl Client {
    /// Takes a stream (TCP, TLS, unix socket, etc) and uses it to connect to an AMQP server.
    ///
    /// This function returns a future that resolves once the connection handshake is done.
    /// The result is a tuple containing a `Client` that can be used to create `Channel`s and a
    /// `Heartbeat` instance. The heartbeat is a task (it implements `Future`) that should be
    /// spawned independently of the other futures.
    ///
    /// To stop the heartbeat task, see `HeartbeatHandle`.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate lapin_futures;
    /// # extern crate tokio;
    /// #
    /// # use tokio::prelude::*;
    /// #
    /// # fn main() {
    /// use tokio::net::TcpStream;
    /// use tokio::runtime::Runtime;
    /// use lapin_futures::client::{Client, ConnectionOptions};
    ///
    /// let addr = "127.0.0.1:5672".parse().unwrap();
    /// let f = TcpStream::connect(&addr)
    ///     .and_then(|stream| {
    ///         Client::connect(stream, ConnectionOptions::default())
    ///     })
    ///     .and_then(|(client, mut background)| {
    ///         let handle = background.handle().unwrap();
    ///         tokio::spawn(
    ///             background.map_err(|e| eprintln!("Lapin background task errored: {}", e))
    ///         );
    ///
    ///         /// ...
    ///
    ///         handle.stop();
    ///         Ok(())
    ///     });
    /// Runtime::new().unwrap().block_on_all(
    ///     f.map_err(|e| eprintln!("An error occured: {}", e))
    /// ).expect("runtime exited with failure");
    /// # }
    /// ```
    pub fn connect<T>(stream: T, options: ConnectionOptions) ->
        impl Future<Item = (Self, Background<T>), Error = io::Error> + Send + 'static
    where
        T: AsyncRead + AsyncWrite + Send + Sync + 'static
    {
        AMQPTransport::connect(stream, options).and_then(|transport| {
            debug!("got client service");
            let configuration = transport.conn.configuration.clone();
            let background = Background::new(transport);
            let client = Client {
                configuration,
                channel: background.channel()
            };
            Ok((client, background))
        })
    }

    /// creates a new channel
    ///
    /// returns a future that resolves to a `Channel` once the method succeeds
    pub fn create_channel(&self) -> impl Future<Item = Channel, Error = io::Error> + Send + 'static {
        Channel::create(self.channel.clone())
    }

    /// returns a future that resolves to a `Channel` once the method succeeds
    /// the channel will support RabbitMQ's confirm extension
    pub fn create_confirm_channel(&self, options: ConfirmSelectOptions) -> impl Future<Item = (), Error = io::Error> + Send + 'static {
        // FIXME: maybe the confirm channel should be a separate type
        // especially, if we implement transactions, the methods should be available on the original channel
        // but not on the confirm channel. And the basic publish method should have different results
        // self.create_channel().and_then(move |channel| {
        //   let ch = channel.clone();

        //   channel.confirm_select(options).map(|_| ch)
        // })
        future::ok(())
    }
}
