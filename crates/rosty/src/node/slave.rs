use async_std::net::ToSocketAddrs;
use nix::unistd::getpid;

use super::NodeArgs;
use crate::rosxmlrpc::{ServerBuilder, Value};

/// Slave API for a ROS node. The slave API is an XMLRPC API that has two roles: receiving callbacks
/// from the master, and negotiating connections with other nodes.
pub struct Slave {
    _server: xmlrpc::Server,
}

impl Slave {
    /// Constructs a new slave for a node with the given arguments.
    pub async fn new(args: &NodeArgs) -> Result<Slave, failure::Error> {
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

        let server = server.bind(&addr).await?;

        Ok(Slave { _server: server })
    }
}
