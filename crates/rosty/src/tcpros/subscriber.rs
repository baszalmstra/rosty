use super::Message;
use crate::tcpros::header;
use crate::Topic;
use futures::stream::StreamExt;
use futures::TryFutureExt;
use rosty_msg::RosMsg;
use std::collections::{BTreeSet, HashMap};
use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::net::ToSocketAddrs;
use tokio::sync::mpsc;
use tracing_futures::Instrument;

#[derive(Fail, Debug)]
enum SubscriberError {
    #[fail(display = "transport error")]
    TransportError(io::Error),

    #[fail(display = "invalid header: {}", 0)]
    InvalidHeader(header::InvalidHeaderError),
}

#[derive(Fail, Debug)]
pub enum PublisherConnectError {
    #[fail(display = "connection queue is full")]
    ConnectionQueueFull,

    #[fail(display = "transport error")]
    TransportError(io::Error),
}

impl From<io::Error> for SubscriberError {
    fn from(err: io::Error) -> Self {
        SubscriberError::TransportError(err)
    }
}

impl From<header::InvalidHeaderError> for SubscriberError {
    fn from(err: header::InvalidHeaderError) -> Self {
        SubscriberError::InvalidHeader(err)
    }
}

impl From<io::Error> for PublisherConnectError {
    fn from(err: io::Error) -> Self {
        PublisherConnectError::TransportError(err)
    }
}

/// A struct with raw message info received from a publisher
struct RawMessage {
    /// The publisher that send the message
    caller_id: Arc<String>,

    /// The data of the message
    data: Vec<u8>,
}

/// A subscriber on a ros topic. Manages connecting to publishers and receiving data from them.
pub struct Subscriber {
    /// Sender end of a channel that receives updates about publishers
    publisher_tx: mpsc::Sender<SocketAddr>,

    /// A set of publishers this subscriber is connecting or connected to.
    connected_publishers: BTreeSet<String>,

    /// The topic that this `Subscriber` subscribes to
    topic: Topic,
}

pub type IncomingMessage<T> = (String, T);

impl Subscriber {
    pub fn new<T>(
        caller_id: &str,
        topic: &str,
        queue_size: usize,
    ) -> (Self, mpsc::Receiver<IncomingMessage<T>>)
    where
        T: Message,
    {
        let (mut topic_tx, topic_rx) = mpsc::channel(queue_size);
        let (data_tx, mut data_rx) = mpsc::channel(queue_size);
        let (publisher_tx, mut publisher_rx) = mpsc::channel(8);

        let caller_id = String::from(caller_id);
        let topic_name = String::from(topic);

        let data_sender = data_tx.clone();
        tokio::spawn(
            async move {
                while let Some(addr) = publisher_rx.next().await {
                    let data_tx = data_sender.clone();
                    tokio::spawn(
                        connect_to_publisher::<T>(
                            addr,
                            caller_id.clone(),
                            topic_name.clone(),
                            data_tx,
                        )
                        .map_err(|e| {
                            error!("error connecting to publisher: {}", e);
                            e
                        })
                        .instrument(tracing::info_span!(
                            "connect_to_publisher",
                            addr = tracing::field::display(addr)
                        )),
                    );
                }
            }
            .instrument(tracing::info_span!(
                "topic_connection_handler",
                topic = topic
            )),
        );

        tokio::spawn(
            async move {
                while let Some(buffer) = data_rx.recv().await {
                    match RosMsg::decode_slice(&buffer.data) {
                        Ok(value) => {
                            if topic_tx
                                .try_send(((*buffer.caller_id).clone(), value))
                                .is_err()
                            {
                                error!("queue is full!");
                            }
                        }
                        Err(err) => error!("failed to decode message: {}", err),
                    }
                }
            }
            .instrument(tracing::info_span!("handle_data", topic = topic)),
        );

        (
            Subscriber {
                publisher_tx,
                connected_publishers: Default::default(),
                topic: Topic {
                    name: topic.to_owned(),
                    data_type: T::msg_type(),
                },
            },
            topic_rx,
        )
    }

    // /// Returns the number of publishers
    // pub fn publisher_count(&self) -> usize {
    //     self.connected_publishers.len()
    // }

    // /// Returns an iterator over all publishers
    // pub fn publishers(&self) -> impl Iterator<Item = String> + '_ {
    //     self.connected_publishers.iter().cloned()
    // }

    /// Connect to node that publishes the subscribed topic
    pub async fn connect_to<U: ToSocketAddrs>(
        &mut self,
        publisher: &str,
        addresses: U,
    ) -> Result<(), PublisherConnectError> {
        for address in addresses.to_socket_addrs().await? {
            info!(topic=?self.topic, publisher=publisher, address=tracing::field::display(address), "connecting");
            self.publisher_tx
                .send(address)
                .await
                .map_err(|_| PublisherConnectError::ConnectionQueueFull)?;
        }
        self.connected_publishers.insert(publisher.to_owned());
        Ok(())
    }

    /// Returns true if the subscriber is connecto to the given publisher
    pub fn is_connected_to(&self, publisher: &str) -> bool {
        self.connected_publishers.contains(publisher)
    }
}

/// Connects to the publisher that is listening at the specified address
async fn connect_to_publisher<T: Message>(
    addr: SocketAddr,
    caller_id: String,
    topic: String,
    mut data_tx: mpsc::Sender<RawMessage>,
) -> Result<(), SubscriberError> {
    // Connect to the publisher
    let mut stream = TcpStream::connect(addr).await?;

    // Exchange header information to describe what the subscriber will listen to
    let pub_caller_id = Arc::new(
        handshake::<T, _>(&mut stream, &caller_id, &topic)
            .await?
            .unwrap_or_default(),
    );

    async {
        info!("connected");

        // Read packets from the stream
        loop {
            match super::read_packet(&mut stream).await {
                Ok(package) => {
                    if let Err(mpsc::error::TrySendError::Closed(_)) =
                        data_tx.try_send(RawMessage {
                            caller_id: pub_caller_id.clone(),
                            data: package,
                        })
                    {
                        // If the channel is closed, break out of the loop, effectively disconnecting
                        info!("subscriber cancelled");
                        break;
                    }
                }
                Err(e) => {
                    match e.kind() {
                        ErrorKind::UnexpectedEof => info!("socket closed"),
                        _ => error!("disconnecting: {}", e),
                    }
                    break;
                }
            }
        }
    }
    .instrument(tracing::info_span!(
        "data_loop",
        pub_caller_id = tracing::field::display(&pub_caller_id),
    ))
    .await;

    Ok(())
}

/// Performs a handshake after the initial connection has been made to let the publisher know what
/// we are interested in. Returns the caller_id of the publisher on a successful connection.
async fn handshake<T: Message, U: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut U,
    caller_id: &str,
    topic: &str,
) -> Result<Option<String>, SubscriberError> {
    write_handshake_request::<T, U>(stream, caller_id, topic).await?;
    read_handshake_response::<T, U>(stream).await
}

/// Write the request message to the given stream
async fn write_handshake_request<T: Message, U: AsyncWrite + Unpin>(
    mut stream: &mut U,
    caller_id: &str,
    topic: &str,
) -> Result<(), SubscriberError> {
    let mut fields = HashMap::<String, String>::new();
    fields.insert(String::from("message_definition"), T::msg_definition());
    fields.insert(String::from("callerid"), String::from(caller_id));
    fields.insert(String::from("topic"), String::from(topic));
    fields.insert(String::from("md5sum"), T::md5sum());
    fields.insert(String::from("type"), T::msg_type());
    header::encode_and_write(&mut stream, &fields)
        .await
        .map_err(Into::into)
}

/// Read the handshake response from the publisher
async fn read_handshake_response<T: Message, U: AsyncRead + Unpin>(
    mut stream: &mut U,
) -> Result<Option<String>, SubscriberError> {
    let fields = header::read_and_decode(&mut stream).await?;
    header::match_field(&fields, "md5sum", &T::md5sum())?;
    header::match_field(&fields, "type", &T::msg_type())?;
    Ok(fields.get("callerid").cloned())
}
