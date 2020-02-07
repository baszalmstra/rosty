use super::Response;
use crate::rosxmlrpc::response_info::ResponseInfo;
use futures::FutureExt;
use std::future::Future;
use std::net::SocketAddr;

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
        self.inner.register_value_async(name, move |args| {
            handler(args).map(move |r| ResponseInfo::from_response(r, msg).into())
        });
    }

    /// Constructs the actual `Server` by creating a binding to the specific `SocketAddr`.
    pub fn bind<F>(
        self,
        addr: &SocketAddr,
        shutdown_signal: F,
    ) -> Result<(impl Future<Output = Result<(), failure::Error>>, SocketAddr), failure::Error>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.inner.bind(addr, shutdown_signal)
    }
}
