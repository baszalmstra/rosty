use crossbeam::sync::ShardedLock;
use failure::bail;
use once_cell::sync::Lazy;

#[macro_use]
extern crate failure;
#[macro_use]
extern crate tracing;

mod node;
mod rosxmlrpc;
mod simtime;
mod tcpros;
mod time;

pub use crate::node::Topic;
use crate::node::{Subscriber, SubscriptionError};
use crate::rosxmlrpc::Response;
use crate::tcpros::Message;
use node::{Node, NodeArgs, Param};

use serde::Deserialize;
use std::time::SystemTime;
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

    // Initialize the use of simtime
    if node.is_using_sim_time() {
        node.init_sim_time().await?;
    }

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

/// Returns 'now' as a Time object
/// # Situations
/// * If the node is run normally the current time is returned. Ros calls this WallTime.
///   This can panic if the SystemTime cannot be retrieved
/// * If the node is run in simulated time i.e. `/use_sim_time` is true. Then the simulated
///   time is returned. In this case a panic could occur if the `/clock` topic has not
///   been published
pub fn now() -> Duration {
    if !node!().is_using_sim_time() {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Could not get the SystemTime")
            .into()
    } else {
        node!()
            .get_last_sim_clock()
            .expect("No /clock message received")
            .into()
    }
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

/// Returns wether the node is running with simualated time
pub fn is_using_sim_time() -> bool {
    node!().is_using_sim_time()
}

pub async fn topics() -> Response<Vec<Topic>> {
    node!().topics().await
}

pub async fn param_names() -> Response<Vec<String>> {
    node!().get_all_param_names().await
}

pub fn param(key: impl AsRef<str>) -> Param {
    node!().param(key)
}

/// Find closest parameter name,
/// starting in the private namespace and searching upwards to the global namespace.
///If this code appears in the node /foo/bar, rospy.search_param will try to find the parameters:
/// * /foo/bar/global_example
/// * /foo/global_example
/// * /global_example
///in this order.
pub async fn search_param<'a, T: Deserialize<'a>, S: AsRef<str>>(key: S) -> Response<T> {
    node!().search_param(key.as_ref()).await
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
