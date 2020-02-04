mod slave;
mod args;

pub use args::NodeArgs;

/// Represents a ROS node.
pub struct Node {
    _slave: slave::Slave,
}

impl Node {
    pub async fn new(args: NodeArgs) -> Result<Self, failure::Error> {
        // Construct a slave XMLRPC server
        let slave = slave::Slave::new(&args)
            .await?;

        Ok(Node {
            _slave: slave
        })
    }
}
