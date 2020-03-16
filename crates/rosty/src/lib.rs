use crossbeam::sync::ShardedLock;
use failure::bail;
use once_cell::sync::Lazy;

#[macro_use]
extern crate failure;
#[macro_use]
extern crate tracing;

mod node;
mod rosxmlrpc;
mod tcpros;
mod time;

pub use crate::node::Topic;
use crate::node::{Subscriber, SubscriptionError};
use crate::rosxmlrpc::Response;
use crate::tcpros::Message;
use node::{Node, NodeArgs};

pub use time::{Duration, Time};

/// The instance that represents this node.
static NODE: Lazy<ShardedLock<Option<Node>>> = Lazy::new(|| ShardedLock::new(None));

/// Initializes the ROS node
pub async fn init<S: AsRef<str>>(default_name: S) -> Result<(), failure::Error> {
    init_with_args(NodeArgs::new(default_name), true).await
}

/// Initializes the ROS node
pub async fn init_with_args(args: NodeArgs, capture_sigint: bool) -> Result<(), failure::Error> {
    let mut singleton = NODE
        .write()
        .expect("Could not acquire write lock to singleton ROS node");
    if singleton.is_some() {
        bail!("There can only be a single ROS node");
    }

    let node = Node::new(args).await?;

    if capture_sigint {
        let shutdown_sender = node.shutdown_token.clone();
        ctrlc::set_handler(move || {
            shutdown_sender.shutdown();
        })?;
    }

    *singleton = Some(node);

    Ok(())
}

/// Returns true if the ROS node has been initialized
pub fn is_initialized() -> bool {
    NODE.read()
        .expect("Could not acquire read lock to singleton ROS node")
        .is_some()
}

/// Returns the singleton node
macro_rules! node {
    () => {
        NODE.read()
            .expect("Could not acquire read lock to singleton ROS node")
            .as_ref()
            .expect("ROS Node has not yet been initialized")
    };
}

/// Returns the URI of this node
pub fn uri() -> String {
    node!().uri().to_owned()
}

/// Returns the name of this node
pub fn name() -> String {
    node!().name().to_owned()
}

/// Returns the hostname of the node
pub fn hostname() -> String {
    node!().hostname().to_owned()
}

/// Returns the bind address of the node
pub fn bind_address() -> String {
    node!().bind_address().to_owned()
}

pub async fn topics() -> Response<Vec<Topic>> {
    node!().topics().await
}

pub async fn run() {
    node!().run().await
}

pub fn shutdown() {
    node!().shutdown_token.shutdown();
}

/// Connect to a topic
pub async fn subscribe<T: Message>(
    topic: &str,
    queue_size: usize,
) -> Result<Subscriber<T>, SubscriptionError> {
    node!().subscribe::<T>(topic, queue_size).await
}
