use async_std::net::ToSocketAddrs;

use super::NodeArgs;



use server::Server;


/// Slave API for a ROS node
pub struct Slave {
    server: Server,
}

impl Slave {
    /// Constructs a new slave for a node with the given arguments.
    pub async fn new(args: &NodeArgs) -> Result<Slave, failure::Error> {

        // Resolve the hostname to an address
        let addr = args.hostname.to_socket_addrs().await?.next().ok_or(|| {
            failure::format_err!(
                "Could not resolve '{}' to a valid socket address",
                args.hostname
            )
        });

        // Construct the server and bind it to the address
        let server = Server::new(addr).await?;
    }
}
