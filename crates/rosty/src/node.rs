mod args;
mod slave;

pub use args::NodeArgs;

/// Represents a ROS node.
///
/// A ROS node has several APIs:
///  * A slave API. The slave API is an XMLRPC API that has two roles: receiving callbacks from the
///    master, and negotiating connections with other nodes.
///  * A topic transport protocol
pub struct Node {
    _slave: slave::Slave,
}

impl Node {
    pub async fn new(args: NodeArgs) -> Result<Self, failure::Error> {
        // Construct a slave XMLRPC server
        let slave = slave::Slave::new(&args).await?;

        Ok(Node { _slave: slave })
    }
}
