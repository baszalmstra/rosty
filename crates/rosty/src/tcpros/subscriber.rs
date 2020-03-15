use super::Message;
use crate::tcpros::header;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::net::ToSocketAddrs;
use tokio::sync::mpsc;

struct MessageInfo {
    caller_id: String,
    data: Vec<u8>,
}

struct Subscriber {
    /// Sender end of a channel that receives updates about publishers
    publisher_tx: mpsc::Sender<SocketAddr>,
}

impl Subscriber {
    pub fn new<F, T>(caller_id: &str, topic: &str, queue_size: usize, callback: F) -> Self
    where
        T: Message,
        F: Fn(T, &str) + Send + 'static,
    {
        let (data_tx, data_rx) = mpsc::channel(queue_size);
        let (publisher_tx, mut publisher_rx) = mpsc::channel(8);

        let caller_id = String::from(caller_id);
        let topic_name = String::from(topic);

        tokio::spawn(async move {
            while let Some(addr) = publisher_rx.next().await {
                let data_tx = data_tx.clone();
                tokio::spawn(connect_to_publisher::<T>(
                    addr,
                    caller_id.clone(),
                    topic_name.clone(),
                    data_tx,
                ));
            }
        });

        Subscriber { publisher_tx }
    }

    /// Connect to node that publishes the subscribed topic
    pub async fn connect_to<U: ToSocketAddrs>(&mut self, publisher: &str, addresses: U) {
        for address in addresses.to_socket_addrs().await {
            //self.publisher_tx.send(address).expect("connection future has died")
        }
    }
}

/// Connects to the publisher that is listening at the specified address
async fn connect_to_publisher<T: Message>(
    addr: SocketAddr,
    caller_id: String,
    topic: String,
    data_tx: mpsc::Sender<T>,
) -> Result<(), io::Error> {
    // Connect to the publisher
    let mut stream = TcpStream::connect(addr).await?;

    // Exchange header information to describe what the subscriber will listen to
    let pub_caller_id = handshake::<T, _>(&mut stream, &caller_id, &topic).await?;

    Ok(())
}

/// Performs a handshake after the initial connection has been made to let the publisher know what
/// we are interested in. Returns the caller_id of the publisher on a successful connection.
async fn handshake<T: Message, U: AsyncRead + AsyncWrite + Unpin>(
    mut stream: &mut U,
    caller_id: &str,
    topic: &str,
) -> Result<Option<String>, io::Error> {
    write_handshake_request::<T, U>(stream, caller_id, topic).await?;
    read_handshake_response::<T, U>(stream).await
}

/// Write the request message to the given stream
async fn write_handshake_request<T: Message, U: AsyncWrite + Unpin>(
    mut stream: &mut U,
    caller_id: &str,
    topic: &str,
) -> Result<(), io::Error> {
    let mut fields = HashMap::<String, String>::new();
    fields.insert(String::from("message_definition"), T::msg_definition());
    fields.insert(String::from("callerid"), String::from(caller_id));
    fields.insert(String::from("topic"), String::from(topic));
    fields.insert(String::from("md5sum"), T::md5sum());
    fields.insert(String::from("type"), T::msg_type());
    header::encode(&mut stream, &fields).await
}

/// Read the handshake response from the publisher
fn read_handshake_response<T: Message, U: AsyncRead + Unpin>(
    mut stream: &mut U,
) -> Result<Option<String>, io::Error> {
    let fields = header::decode(&mut stream)?;
    header::match_field(&fields, "md5sum", &T::md5sum())?;
    header::match_field(&fields, "type", &T::msg_type())?;
    Ok(fields.get("callerid").cloned())
}
