use super::header;
use crate::rosxmlrpc::ResponseError;
use crate::shutdown_token::ShutdownToken;
use crate::tcpros::Message;
use crate::Topic;
use failure::_core::marker::PhantomData;
use futures::{StreamExt};
use std::collections::HashMap;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::broadcast::{self, RecvError};
use tracing_futures::Instrument;

#[derive(Debug, Fail)]
pub enum PublisherError {
    #[fail(display = "bind error")]
    BindError(#[fail(cause)] std::io::Error),

    #[fail(display = "could not get local address")]
    LocalAddressError(#[fail(cause)] std::io::Error),

    #[fail(display = "transport error")]
    TransportError(#[fail(cause)] std::io::Error),

    #[fail(display = "registration error")]
    RegistrationError(#[fail(cause)] ResponseError),
}

#[derive(Debug, Fail)]
pub enum PublisherSubcribeError {
    #[fail(display = "transport error")]
    TransportError(#[fail(cause)] std::io::Error),

    #[fail(display = "invalid header: {}", 0)]
    InvalidHeader(header::InvalidHeaderError),
}

#[derive(Debug, Fail)]
pub enum PublisherSendError {
    #[fail(display = "encoding error")]
    EncodingError(#[fail(cause)] std::io::Error),
}

impl From<std::io::Error> for PublisherSubcribeError {
    fn from(e: std::io::Error) -> Self {
        PublisherSubcribeError::TransportError(e)
    }
}

impl From<header::InvalidHeaderError> for PublisherSubcribeError {
    fn from(e: header::InvalidHeaderError) -> Self {
        PublisherSubcribeError::InvalidHeader(e)
    }
}

pub struct Publisher {
    pub topic: Topic,
    pub port: u16,
    shutdown_token: ShutdownToken,

    sender: broadcast::Sender<Vec<u8>>,
}

impl Drop for Publisher {
    fn drop(&mut self) {
        self.shutdown_token.shutdown();
    }
}

impl Publisher {
    pub async fn new<T, U>(
        address: U,
        topic: &str,
        queue_size: usize,
        caller_id: &str,
    ) -> Result<Publisher, PublisherError>
    where
        T: Message,
        U: ToSocketAddrs,
    {
        let shutdown_token = ShutdownToken::default();
        let mut listener = TcpListener::bind(address)
            .await
            .map_err(PublisherError::BindError)?;
        let socket_addr = listener
            .local_addr()
            .map_err(PublisherError::LocalAddressError)?;
        let port = socket_addr.port();

        let (sender, _) = broadcast::channel(queue_size);

        // Construct a future that will accept incoming connections
        let topic_str = topic.to_owned();
        let caller_id_str = caller_id.to_owned();

        // Accept connections until the publisher is shut down
        let accept_shutdown_token = shutdown_token.clone();
        let sender_for_receivers = sender.clone();
        tokio::spawn(async move {
            let accept_future = listener
                .incoming()
                .for_each_concurrent(None, move |stream| {
                    let topic_str = topic_str.clone();
                    let topic_str2 = topic_str.clone();
                    let caller_id_str = caller_id_str.to_owned();
                    let receiver = sender_for_receivers.subscribe();
                    async move {
                        match stream {
                            Ok(stream) => {
                                let remote = stream.peer_addr().expect("must have a peer addr").to_string();
                                tokio::spawn(async move {
                                    let topic_str = topic_str.clone();
                                    let caller_id_str = caller_id_str.to_owned();
                                    process_subscriber::<T, _>(&topic_str, stream, &caller_id_str, receiver)
                                        .await;
                                }.instrument(tracing::info_span!(
                                    "publisher",
                                    topic=topic_str2.as_str(),
                                    remote=remote.as_str()
                                )));
                            }
                            Err(e) => error!("incoming connection failed: {}", e),
                        };
                    }
                });
            tokio::select!(
                _ = accept_future => {},
                _ = accept_shutdown_token => {});
        });

        Ok(Publisher {
            topic: Topic {
                name: topic.to_owned(),
                data_type: T::msg_type(),
            },
            sender,
            port,
            shutdown_token,
        })
    }

    pub fn stream<T: Message>(
        &self,
        _queue_size: usize,
    ) -> Result<PublisherStream<T>, PublisherError> {
        let stream = PublisherStream {
            datatype: PhantomData::default(),
            sender: self.sender.clone(),
        };
        Ok(stream)
    }
}

async fn process_subscriber<T, U>(topic: &str, mut stream: U, pub_caller_id: &str, mut receiver: broadcast::Receiver<Vec<u8>>)
where
    T: Message,
    U: AsyncWrite + AsyncRead + Send + Unpin,
{
    info!("incoming connection");

    let caller_id = match handshake::<T, _>(&mut stream, pub_caller_id, topic).await {
        Ok(caller_id) => caller_id,
        Err(e) => {
            error!("handshake error: {}, aborting..", e);
            return;
        }
    };

    async {
        info!("connected");

        while let Some(data) = receiver.next().await {
            match data {
                Ok(message) => {
                    match stream.write_all(&message).await {
                        Ok(_) => {},
                        Err(e) => {
                            error!("error sending message: {}, disconnecting..", e);
                            return;
                        }
                    }
                },
                Err(RecvError::Closed) => {
                    info!("publisher closed");
                    break;
                }
                Err(RecvError::Lagged(i)) => {
                    warn!("skipped {} message", i);
                }
            }
        }
    }.instrument(tracing::info_span!(
        "caller",
        id=caller_id.as_str()
    )).await
}

async fn handshake<T: Message, U: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut U,
    pub_caller_id: &str,
    topic: &str,
) -> Result<String, PublisherSubcribeError> {
    let caller_id = read_handshake_request::<T, U>(stream, topic).await?;
    write_handshake_response::<T, U>(stream, pub_caller_id).await?;
    Ok(caller_id)
}

async fn read_handshake_request<T: Message, U: AsyncRead + Unpin>(
    mut stream: &mut U,
    topic: &str,
) -> Result<String, PublisherSubcribeError> {
    let fields = header::read_and_decode(&mut stream).await?;
    header::match_field(&fields, "md5sum", &T::md5sum())?;
    header::match_field(&fields, "type", &T::msg_type())?;
    header::match_field(&fields, "topic", topic)?;
    Ok(fields
        .get("callerid")
        .ok_or_else(|| header::InvalidHeaderError::MissingField("callerid".into()))?
        .clone())
}

async fn write_handshake_response<T: Message, U: AsyncWrite + Unpin>(
    mut stream: &mut U,
    caller_id: &str,
) -> Result<(), PublisherSubcribeError> {
    let mut fields = HashMap::<String, String>::new();
    fields.insert(String::from("md5sum"), T::md5sum());
    fields.insert(String::from("type"), T::msg_type());
    fields.insert(String::from("callerid"), caller_id.into());
    fields.insert(String::from("message_definition"), T::msg_definition());
    header::encode_and_write(&mut stream, &fields)
        .await
        .map_err(Into::into)
}

#[derive(Clone)]
pub struct PublisherStream<T: Message> {
    datatype: PhantomData<T>,
    sender: broadcast::Sender<Vec<u8>>
}

impl<T: Message> PublisherStream<T> {
    pub async fn send(&self, message: T) -> Result<(), PublisherSendError> {
        let bytes = message.encode_vec().map_err(PublisherSendError::EncodingError)?;

        // TODO: latching??

        let _ = self.sender.send(bytes);
        Ok(())
    }
}
