mod client;
mod server;

pub use server::{on_decode_fail, on_encode_fail, on_missing_method, Server, ServerBuilder};
pub use xmlrpc_fmt::{into_params, Params, Value, Response};
