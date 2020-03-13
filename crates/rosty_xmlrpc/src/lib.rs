mod client;
mod server;

pub use client::{call, call_with_params, Client, Uri};
pub use server::{on_decode_fail, on_encode_fail, on_missing_method, ServerBuilder};
pub use xmlrpc_fmt::{from_params, into_params, Fault, Params, Response, Value};
