use crate::node::{Node, SubscriptionError};

use futures::StreamExt;
use std::sync::{Arc, RwLock};
use rosty_msg::Time;

#[derive(Debug)]
/// The Simulated Time struct, holds a reference to the last message received on the clock interface
pub struct SimTime {
    last_clock_msg: Arc<RwLock<Option<Time>>>,
}

impl SimTime {
    pub fn new() -> Self {
        let last_clock_msg = Arc::new(RwLock::new(None));
        SimTime { last_clock_msg }
    }

    /// Initialize to the subscribing of simulated time
    pub async fn init(&self, node: &Node) -> Result<(), SubscriptionError> {
        let last_clock_msg_cloned = self.last_clock_msg.clone();
        let future = node
            .subscribe::<rosty_msg::rosgraph_msgs::Clock>("/clock", 1)
            .await?
            .for_each(move |(_, clock_msg)| {
                let local = last_clock_msg_cloned.clone();
                // Set the last variable as a member
                async move {
                    let mut guard = local.write().unwrap();
                    *guard = Some(clock_msg.clock);
                }
            });

        tokio::spawn(future);
        Ok(())
    }

    /// Returns the last received time
    pub fn now(&self) -> Option<Time> {
        let message = self.last_clock_msg.read().unwrap().clone();
        message.map(Into::into)
    }
}
