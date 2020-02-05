use super::Response;
use crate::rosxmlrpc::response_info::ResponseInfo;
use futures::FutureExt;
use std::future::Future;
use std::net::SocketAddr;
pub use xmlrpc::Server;

/// Wraps an `xmlrpc::ServerBuilder` to hide the details of the ROS XMLRPC protocol.
pub struct ServerBuilder {
    inner: xmlrpc::ServerBuilder,
}

impl ServerBuilder {
    pub fn new() -> Self {
        ServerBuilder {
            inner: xmlrpc::ServerBuilder::new(),
        }
    }

    /// Registers a XLMRPC call handler
    pub fn register_value<K, T, R>(&mut self, name: K, msg: &'static str, handler: T)
    where
        K: Into<String>,
        R: Future<Output = Response<xmlrpc::Value>> + Send + 'static,
        T: (Fn(xmlrpc::Params) -> R) + Send + Sync + 'static,
    {
        self.inner.register_value(name, move |args| {
            handler(args)
                .map(move |r| {
                    ResponseInfo::from_response(r, msg).into()
                })
        });
    }

    /// Constructs the actual `Server` by creating a binding to the specific `SocketAddr`.
    pub async fn bind(self, addr: &SocketAddr) -> Result<Server, failure::Error> {
        self.inner.bind(addr).await
    }
}
