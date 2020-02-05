mod args;
mod slave;
mod shutdown_token;

pub use args::NodeArgs;
use shutdown_token::{ShutdownToken};

/// Represents a ROS node.
///
/// A ROS node has several APIs:
///  * A slave API. The slave API is an XMLRPC API that has two roles: receiving callbacks from the
///    master, and negotiating connections with other nodes.
///  * A topic transport protocol
pub struct Node {
    _slave: slave::Slave,
    pub shutdown_token: ShutdownToken
}

impl Node {
    pub async fn new(args: NodeArgs) -> Result<Self, failure::Error> {
        let shutdown_token = ShutdownToken::default();

        // Construct a slave XMLRPC server
        let slave = slave::Slave::new(&args, shutdown_token.clone()).await?;

        Ok(Node { _slave: slave, shutdown_token })
    }
}