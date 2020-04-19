use super::simtime::SimTime;
use std::time::{SystemTime};
use crate::node::{Node, SubscriptionError};
use rosty_msg::Time;

pub struct Clock {
    pub(crate) sim_time: Option<SimTime>
}

impl Clock {
    /// Returns 'now' as a Time object
    /// # Situations
    /// * If the node is run normally the current time is returned. Ros calls this WallTime.
    ///   This can panic if the SystemTime cannot be retrieved
    /// * If the node is run in simulated time i.e. `/use_sim_time` is true. Then the simulated
    ///   time is returned. In this case a panic could occur if the `/clock` topic has not
    ///   been published
    pub fn now(&self) -> Option<Time> {
        match &self.sim_time {
            Some(sim_time) => sim_time.now(),
            None => {
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .ok()
                    .map(|d| Time::from_nanos(d.as_nanos() as i64))
            }
        }
    }

    pub(crate) async fn init(&self, node: &Node) -> Result<(), SubscriptionError>{
        if let Some(sim_time) = &self.sim_time {
            sim_time.init(node).await?;
        }
        Ok(())
    }

    /// Returns true if the node is using simulated time
    pub fn is_using_sim_time(&self) -> bool {
        self.sim_time.is_some()
    }
}


