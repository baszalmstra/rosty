use crossbeam::sync::ShardedLock;
use failure::bail;
use once_cell::sync::Lazy;

mod node;

use node::{Node, NodeArgs};

/// The instance that represents this node.
static NODE: Lazy<ShardedLock<Option<Node>>> = Lazy::new(|| ShardedLock::new(None));

/// Initializes the ROS node
pub async fn init<S: AsRef<str>>(default_name: S) -> Result<(), failure::Error> {
    init_with_args(NodeArgs::new(default_name)).await
}

/// Initializes the ROS node
pub async fn init_with_args(args: NodeArgs) -> Result<(), failure::Error> {
    let mut singleton = NODE
        .write()
        .expect("Could not acquire write lock to singleton ROS node");
    if singleton.is_some() {
        bail!("There can only be a single ROS node");
    }

    let node = Node::new(args).await?;
    *singleton = Some(node);

    Ok(())
}

/// Returns true if the ROS node has been initialized
pub fn is_initialized() -> bool {
    NODE.read()
        .expect("Could not acquire read lock to singleton ROS node")
        .is_some()
}
