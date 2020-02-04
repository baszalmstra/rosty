use async_std::net::ToSocketAddrs;

use super::NodeArgs;


/// Slave API for a ROS node
pub struct Slave {
    server: xmlrpc::Server,
}

impl Slave {
    /// Constructs a new slave for a node with the given arguments.
    pub async fn new(args: &NodeArgs) -> Result<Slave, failure::Error> {

        // Resolve the hostname to an address
        let addr = format!("{}:{}", args.hostname, 0).to_socket_addrs().await?.next().ok_or_else(|| {
            failure::format_err!(
                "Could not resolve '{}' to a valid socket address",
                args.hostname
            )
        })?;

        // Construct the server and bind it to the address
        let server = xmlrpc::ServerBuilder::new()
            .bind(&addr)
            .await?;

        Ok(Slave {
            server
        })
    }
}
