mod client;
mod server;

pub use client::{Uri, Client, call, call_with_params};
pub use server::{on_decode_fail, on_encode_fail, on_missing_method, ServerBuilder};
pub use xmlrpc_fmt::{from_params, into_params, Fault, Params, Response, Value};
