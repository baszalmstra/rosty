use async_std::net::ToSocketAddrs;
use nix::unistd::getpid;

use crate::node::shutdown_token::ShutdownToken;
use crate::rosxmlrpc::{Params, ResponseError, ServerBuilder, Value};
use futures::future::TryFutureExt;
use std::future::Future;

fn unwrap_array_case(params: Params) -> Params {
    if let Some(&Value::Array(ref items)) = params.get(0) {
        return items.clone();
    }
    params
}

/// Slave API for a ROS node. The slave API is an XMLRPC API that has two roles: receiving callbacks
/// from the master, and negotiating connections with other nodes.
pub struct Slave {
    name: String,
    uri: String,
}

impl Slave {
    /// Constructs a new slave for a node with the given arguments.
    pub async fn new(
        master_uri: &str,
        hostname: &str,
        bind_address: &str,
        port: u16,
        name: &str,
        shutdown_signal: ShutdownToken,
    ) -> Result<(Slave, impl Future<Output = Result<(), failure::Error>>), failure::Error> {
        // Resolve the hostname to an address. 0 for the port indicates that the slave can bind to
        // any port that is available
        let addr = format!("{}:{}", bind_address, port)
            .to_socket_addrs()
            .await?
            .next()
            .ok_or_else(|| {
                failure::format_err!(
                    "Could not resolve '{}:{}' to a valid socket address",
                    bind_address,
                    port
                )
            })?;

        // Construct the server and bind it to the address
        let mut server = ServerBuilder::new();

        let master_uri = master_uri.to_owned();
        server.register_value("getMasterUri", "Master URI", move |_args| {
            let master_uri = master_uri.clone();
            async { Ok(Value::String(master_uri)) }
        });

        server.register_value("getPid", "PID", |_args| async {
            Ok(Value::Int(getpid().into()))
        });

        let rpc_shutdown_signal = shutdown_signal.clone();
        server.register_value("shutdown", "Shutdown", move |args| {
            let shutdown_signal = rpc_shutdown_signal.clone();
            async move {
                let mut args = unwrap_array_case(args).into_iter();
                let _caller_id = args
                    .next()
                    .ok_or_else(|| ResponseError::Client("Missing argument 'caller_id'".into()))?;
                let message = match args.next() {
                    Some(Value::String(message)) => message,
                    _ => return Err(ResponseError::Client("Missing argument 'message'".into())),
                };
                info!("Server is shutting down because: {}", message);
                shutdown_signal.shutdown();
                Ok(Value::Int(0))
            }
        });

        // Start listening for server requests
        let (server, addr) = server.bind(&addr, shutdown_signal.clone())?;
        let server = tokio::spawn(server).unwrap_or_else(|e| Err(e.into()));

        Ok((
            Slave {
                name: name.to_owned(),
                uri: format!("http://{}:{}/", hostname, addr.port()),
            },
            server,
        ))
    }

    /// Returns the listen URI of the slave
    pub fn uri(&self) -> &str {
        &self.uri
    }
}
