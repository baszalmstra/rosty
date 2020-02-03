mod slave;
mod args;

pub use args::NodeArgs;

/// Represents a ROS node.
pub struct Node {
    //slave: slave::Slave,
}

impl Node {
    pub fn new(args: NodeArgs) -> Self {
        Node {}
    }
}