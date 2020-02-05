mod client;
mod server;

pub use client::Client;
pub use server::{on_decode_fail, on_encode_fail, on_missing_method, Server, ServerBuilder};
pub use xmlrpc_fmt::{from_params, into_params, Fault, Params, Response, Value};
