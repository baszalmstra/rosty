use async_std::net::ToSocketAddrs;
use nix::unistd::getpid;

use super::NodeArgs;
use crate::node::shutdown_token::ShutdownToken;
use crate::rosxmlrpc::{Params, ResponseError, ServerBuilder, Value};

fn unwrap_array_case(params: Params) -> Params {
    if let Some(&Value::Array(ref items)) = params.get(0) {
        return items.clone();
    }
    params
}

/// Slave API for a ROS node. The slave API is an XMLRPC API that has two roles: receiving callbacks
/// from the master, and negotiating connections with other nodes.
pub struct Slave {
    _server: xmlrpc::Server,
}

impl Slave {
    /// Constructs a new slave for a node with the given arguments.
    pub async fn new(
        args: &NodeArgs,
        shutdown_signal: ShutdownToken,
    ) -> Result<Slave, failure::Error> {
        // Resolve the hostname to an address. 0 for the port indicates that the slave can bind to
        // any port that is available
        let addr = format!("{}:{}", args.hostname, 0)
            .to_socket_addrs()
            .await?
            .next()
            .ok_or_else(|| {
                failure::format_err!(
                    "Could not resolve '{}' to a valid socket address",
                    args.hostname
                )
            })?;

        // Construct the server and bind it to the address
        let mut server = ServerBuilder::new();

        let master_uri = args.master_uri.clone();
        server.register_value("getMasterUri", "Master URI", move |_args| {
            let master_uri = master_uri.clone();
            async { Ok(Value::String(master_uri)) }
        });

        server.register_value("getPid", "PID", |_args| {
            async { Ok(Value::Int(getpid().into())) }
        });

        server.register_value("shutdown", "Shutdown", move |args| {
            let shutdown_signal = shutdown_signal.clone();
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

        let server = server.bind(&addr).await?;

        Ok(Slave { _server: server })
    }
}
