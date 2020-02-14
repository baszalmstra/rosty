use std::fmt::Debug;

mod msgs;
mod rosmsg;
mod time;

pub use msgs::*;
pub use time::{Duration, Time};

use rosmsg::RosMsg;

pub trait Message: Clone + Debug + Default + PartialEq + RosMsg + Send + Sync + 'static {
    fn msg_definition() -> String;
    fn md5sum() -> String;
    fn msg_type() -> String;
    fn header(&self) -> Option<&std_msgs::Header> {
        None
    }
    fn header_mut(&mut self) -> Option<&mut std_msgs::Header> {
        None
    }
}

pub trait ServicePair: Clone + Debug + Default + PartialEq + Message {
    type Request: RosMsg + Send + 'static;
    type Response: RosMsg + Send + 'static;
}
