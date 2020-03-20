use super::Response;
use crate::rosxmlrpc::response_info::ResponseInfo;
use futures::FutureExt;
use std::future::Future;
use std::net::SocketAddr;
use tracing_futures::Instrument;
use xmlrpc::Fault;

/// Wraps an `xmlrpc::ServerBuilder` to hide the details of the ROS XMLRPC protocol.
pub struct ServerBuilder {
    inner: xmlrpc::ServerBuilder,
}

impl ServerBuilder {
    pub fn new() -> Self {
        let mut builder = xmlrpc::ServerBuilder::new();
        builder.set_on_missing(|params| async move {
            error!(params=?params, "unhandled xmlrpc request");
            Err(Fault::new(404, "Requested method does not exist"))
        });

        ServerBuilder { inner: builder }
    }

    /// Registers a XLMRPC call handler
    pub fn register_value<K, T, R>(&mut self, name: K, msg: &'static str, handler: T)
    where
        K: Into<String>,
        R: Future<Output = Response<xmlrpc::Value>> + Send + 'static,
        T: (Fn(xmlrpc::Params) -> R) + Send + Sync + 'static,
    {
        let name = name.into();
        self.inner.register_value_async(&name.clone(), move |args| {
            handler(args.clone())
                .instrument(tracing::trace_span!("handle_call", name=name.as_str(), args=?args))
                .map(move |r| ResponseInfo::from_response(r, msg).into())
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
