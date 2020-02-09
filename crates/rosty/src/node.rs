mod args;
mod master;
mod shutdown_token;
mod slave;
mod topic;

pub use args::NodeArgs;
use master::{Master};
use shutdown_token::ShutdownToken;
use slave::Slave;

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::rosxmlrpc::Response;

pub use master::Topic;

/// Represents a ROS node.
///
/// A ROS node has several APIs:
///  * A slave API. The slave API is an XMLRPC API that has two roles: receiving callbacks from the
///    master, and negotiating connections with other nodes.
///  * A topic transport protocol
pub struct Node {
    slave: Slave,
    master: Master,
    hostname: String,
    bind_address: String,
    name: String,
    result: Arc<Mutex<Option<Result<(), failure::Error>>>>,
    pub shutdown_token: ShutdownToken,
}

impl Node {
    pub async fn new(args: NodeArgs) -> Result<Self, failure::Error> {
        let shutdown_token = ShutdownToken::default();

        // Bind to all addresses if the hostname is not localhost
        let bind_host = {
            if args.hostname == "localhost" || args.hostname.starts_with("127.") {
                &args.hostname
            } else {
                "0.0.0.0"
            }
        };

        let namespace = args.namespace.trim_end_matches('/');
        let name = &args.name;
        if name.contains('/') {
            bail!(
                "Illegal character in node name '{}' - limited to letters, numbers and underscores",
                name
            )
        }
        let name = format!("{}/{}", namespace, name);

        // Construct a slave XMLRPC server
        let (slave, slave_future) = Slave::new(
            &args.master_uri,
            &args.hostname,
            &bind_host,
            0,
            &name,
            shutdown_token.clone(),
        )
        .await?;

        // Construct the master API client
        let master = Master::new(&args.master_uri, &name, &slave.uri())?;
        let result_mutex = Arc::new(Mutex::new(None));

        let join_handle_mutex = result_mutex.clone();
        tokio::spawn(async move {
            let mut mutex_guard = join_handle_mutex.lock().await;
            *mutex_guard = Some(tokio::try_join!(slave_future).map(|_| ()))
        });

        Ok(Node {
            slave,
            master,
            hostname: args.hostname.to_owned(),
            bind_address: bind_host.to_owned(),
            name,
            result: result_mutex,
            shutdown_token,
        })
    }

    /// Returns the URI of this node
    pub fn uri(&self) -> &str {
        self.slave.uri()
    }

    /// Returns the name of this node
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the hostname of the node
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Returns the bind address of the node
    pub fn bind_address(&self) -> &str {
        &self.bind_address
    }

    /// Returns a future that is resolved when the node shuts down
    pub async fn run(&self) {
        loop {
            let lock = self.result.lock().await;
            if lock.is_some() {
                return;
            }
        }
    }

    /// Returns a list of all topics
    pub async fn topics(&self) -> Response<Vec<Topic>> {
        self.master
            .get_topic_types()
            .await
    }

//    pub async fn get_param(&self, key: &str) -> Response<Value> {
//        self.master.
//    }
}
