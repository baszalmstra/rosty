use crate::node::{Node, SubscriptionError};

use crate::Duration;
use futures::StreamExt;
use rosty_msg::rosgraph_msgs::Clock;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
/// The Simulated Time struct, holds a reference to the last message received on the clock interface
pub struct SimTime {
    last_clock_msg: Arc<RwLock<Option<Clock>>>,
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
                    *guard = Some(clock_msg);
                }
            });

        tokio::spawn(future);
        Ok(())
    }

    /// Returns the last received time
    pub fn duration(&self) -> Option<Duration> {
        let message = self.last_clock_msg.read().unwrap().clone();
        message.and_then(|message| Some(message.into()))
    }
}
