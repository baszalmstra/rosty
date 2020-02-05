//! ROS uses a protocol on top of XMLRPC to encode return values. XMLRPC calls always return a tuple
//! in the form of: `(code, statusMessage, value)`:
//!
//! * `code` is the status code of the call,
//! * `statusMessage` is a a human-readable string describing the return status
//! * `value` is further defined by individual API calls.
//!
//! This module wraps the rosty_xmlrpc crate to hide the details of this protocol.
//!
//! For more information read: https://wiki.ros.org/ROS/Master_Slave_APIs

mod server;
mod response_info;

pub use server::{Server, ServerBuilder};
pub use xmlrpc::Value;

pub type Response<T> = Result<T, ResponseError>;

/// A `ResponseError` is an error that is either caused by the client or by the server.
#[derive(Debug, Clone, Eq, PartialEq, Fail)]
pub enum ResponseError {
    #[fail(display = "client error: {}", 0)]
    Client(String),
    #[fail(display = "server error: {}", 0)]
    Server(String),
}

const ERROR_CODE: i32 = -1;
const FAILURE_CODE: i32 = 0;
const SUCCESS_CODE: i32 = 1;