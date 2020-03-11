use tokio::net::ToSocketAddrs;
use tokio::sync::mpsc;
use tokio::net::TcpStream;
use std::net::SocketAddr;
use futures::stream::StreamExt;
use std::io;
use super::Message;

struct MessageInfo {
    caller_id: String,
    data: Vec<u8>
}

struct Subscriber {
    /// Sender end of a channel that receives updates about publishers
    publisher_tx: mpsc::Sender<SocketAddr>,
}

impl Subscriber {
    pub fn new<F, T>(caller_id: &str, topic: &str, queue_size: usize, callback: F) -> Self
    where
        T: Message,
        F: Fn(T, &str) + Send + 'static
    {
        let (data_tx, data_rx) = mpsc::channel(queue_size);
        let (publisher_tx, mut publisher_rx) = mpsc::channel(8);

        let caller_id = String::from(caller_id);
        let topic_name = String::from(topic);

        tokio::spawn(async move {
            while let Some(addr) = publisher_rx.next().await {
                let data_tx = data_tx.clone();
                tokio::spawn(connect_to_publisher::<T>(addr, caller_id.clone(), topic_name.clone(), data_tx));
            }
        });
        
        Subscriber {
            publisher_tx
        }
    }

    /// Connect to node that publishes the subscribed topic
    pub async fn connect_to<U: ToSocketAddrs>(&mut self, publisher: &str, addresses: U) {
        for address in addresses.to_socket_addrs().await {
            //self.publisher_tx.send(address).expect("connection future has died")
        }
    }
}

/// Connects to the publisher that is listening at the specified address
async fn connect_to_publisher<T: Message>(addr: SocketAddr, caller_id: String, topic: String, data_tx: mpsc::Sender<T>) -> Result<(), io::Error>{
    // Connect to the publisher
    let mut stream = TcpStream::connect(addr).await?;

    // Exchange header information to describe what the subscriber will listen to
    let pub_caller_id = handshake::<T>(&mut stream, &caller_id, &topic).await?;

    Ok(())
}

/// Performs a handshake after the initial connection has been made to let the publisher know what
/// we are interested in.
async fn handshake<T: Message>(stream: &mut TcpStream, caller_id: &str, topic: &str) -> Result<(), io::Error> {
    Ok(())
}
