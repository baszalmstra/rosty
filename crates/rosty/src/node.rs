mod args;
mod error;
mod master;
mod shutdown_token;
mod slave;
mod subscriber;
mod topic;

pub use args::NodeArgs;
use master::Master;
use shutdown_token::ShutdownToken;
use slave::Slave;

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::rosxmlrpc::Response;

pub use self::error::SubscriptionError;
pub use self::subscriber::Subscriber;
use crate::tcpros::Message;
pub use master::Topic;
use tracing_futures::Instrument;

/// Represents a ROS node.
///
/// A ROS node has several APIs:
///  * A slave API. The slave API is an XMLRPC API that has two roles: receiving callbacks from the
///    master, and negotiating connections with other nodes.
///  * A topic transport protocol
pub struct Node {
    slave: Arc<Slave>,
    master: Arc<Master>,
    hostname: String,
    bind_address: String,
    name: String,
    result: Arc<Mutex<Option<Result<(), failure::Error>>>>,
    pub shutdown_token: ShutdownToken,
}

impl Node {
    pub async fn new(args: NodeArgs) -> Result<Self, failure::Error> {
        let shutdown_token = ShutdownToken::default();

        // Bind to all addresses if the hostname is not localhost
        let bind_host = {
            if args.hostname == "localhost" || args.hostname.starts_with("127.") {
                &args.hostname
            } else {
                "0.0.0.0"
            }
        };

        let namespace = args.namespace.trim_end_matches('/');
        let name = &args.name;
        if name.contains('/') {
            bail!(
                "Illegal character in node name '{}' - limited to letters, numbers and underscores",
                name
            )
        }
        let name = format!("{}/{}", namespace, name);

        // Construct a slave XMLRPC server
        let (slave, slave_future) = Slave::new(
            &args.master_uri,
            &args.hostname,
            &bind_host,
            0,
            &name,
            shutdown_token.clone(),
        )
        .await?;

        // Construct the master API client
        let master = Master::new(&args.master_uri, &name, &slave.uri())?;

        // Get the URI of the master to check if the master is available
        master.get_uri().await?;

        // Start the slave
        let result_mutex = Arc::new(Mutex::new(None));
        let join_handle_mutex = result_mutex.clone();
        tokio::spawn(async move {
            let mut mutex_guard = join_handle_mutex.lock().await;
            *mutex_guard = Some(tokio::try_join!(slave_future).map(|_| ()))
        });

        Ok(Node {
            slave: Arc::new(slave),
            master: Arc::new(master),
            hostname: args.hostname.to_owned(),
            bind_address: bind_host.to_owned(),
            name,
            result: result_mutex,
            shutdown_token,
        })
    }

    /// Returns the URI of this node
    pub fn uri(&self) -> &str {
        self.slave.uri()
    }

    /// Returns the name of this node
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the hostname of the node
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Returns the bind address of the node
    pub fn bind_address(&self) -> &str {
        &self.bind_address
    }

    /// Returns a future that is resolved when the node shuts down
    pub async fn run(&self) {
        loop {
            let lock = self.result.lock().await;
            if lock.is_some() {
                return;
            }
        }
    }

    /// Returns a list of all topics
    pub async fn topics(&self) -> Response<Vec<Topic>> {
        self.master.get_topic_types().await
    }

    /// Connect to a topic
    pub async fn subscribe<T: Message>(
        &self,
        topic: &str,
        queue_size: usize,
    ) -> Result<Subscriber<T>, SubscriptionError> {
        let queue_size = if queue_size == 0 {
            usize::max_value()
        } else {
            queue_size
        };
        Subscriber::new(
            Arc::clone(&self.master),
            Arc::clone(&self.slave),
            topic,
            queue_size,
        )
        .instrument(tracing::info_span!("subscribe", topic = topic))
        .await
    }
}
