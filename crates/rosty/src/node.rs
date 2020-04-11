mod args;
mod error;
mod master;
mod shutdown_token;
mod slave;
mod subscriber;
mod topic;

pub use args::NodeArgs;
use master::Master;
use shutdown_token::ShutdownToken;
use slave::Slave;

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::rosxmlrpc::Response;

pub use self::error::SubscriptionError;
pub use self::subscriber::Subscriber;
use crate::simtime::SimTime;
use crate::tcpros::Message;
use crate::Duration;
pub use master::Topic;
use serde::{Deserialize, Serialize};
use tracing_futures::Instrument;

/// Represents a param on the parameter server
pub struct Param {
    name: String,
    master: Arc<Master>,
}

impl Param {
    fn new(name: impl AsRef<str>, master: Arc<Master>) -> Param {
        Param {
            name: name.as_ref().to_string(),
            master,
        }
    }

    /// Get the value from the parameter server
    pub async fn get<'a, T: Deserialize<'a>>(&self) -> Response<T> {
        self.master.get_param(&self.name).await
    }

    /// Set the value on the ROS parameter server
    pub async fn set<T: Serialize>(&self, value: &T) -> Response<()> {
        self.master
            .set_param(&self.name, value)
            .await
            // We can ignore the i32, because the ROS standard says it is ignorable
            .map(|_val: i32| ())
    }

    /// Delete the parameter from the ROS parameter server
    pub async fn delete(&self) -> Response<()> {
        self.master
            .delete_param(&self.name)
            .await
            // We can ignore the i32, because the ROS standard says it is ignorable
            .map(|_val: i32| ())
    }

    /// Check if this parameter already exists on the ROS parameter server
    pub async fn exists(&self) -> Response<bool> {
        self.master.has_param(&self.name).await
    }
}

/// Represents a ROS node.
///
/// A ROS node has several APIs:
///  * A slave API. The slave API is an XMLRPC API that has two roles: receiving callbacks from the
///    master, and negotiating connections with other nodes.
///  * A topic transport protocol
pub struct Node {
    slave: Arc<Slave>,
    master: Arc<Master>,
    hostname: String,
    bind_address: String,
    name: String,
    result: Arc<Mutex<Option<Result<(), failure::Error>>>>,
    sim_time: Option<SimTime>,
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

        // Construct the master API client
        let master = Arc::new(Master::new(&args.master_uri, &name)?);

        // Construct a slave XMLRPC server
        let (slave, slave_future) = Slave::new(
            &args.master_uri,
            &args.hostname,
            &bind_host,
            0,
            &name,
            master.clone(),
            shutdown_token.clone(),
        )
        .await?;

        // Get the URI of the master to check if the master is available
        master.get_uri().await?;

        // Start the slave
        let result_mutex = Arc::new(Mutex::new(None));
        let join_handle_mutex = result_mutex.clone();
        tokio::spawn(async move {
            let mut mutex_guard = join_handle_mutex.lock().await;
            *mutex_guard = Some(tokio::try_join!(slave_future).map(|_| ()))
        });

        // Check if we need to use simtime
        let param = Param::new("/use_sim_time", master.clone());

        // Try to get the sim_time, and open a topic if we are waiting for it
        let sim_time = if param.exists().await? && param.get::<bool>().await? {
            Some(SimTime::new())
        } else {
            None
        };

        Ok(Node {
            slave: Arc::new(slave),
            master,
            hostname: args.hostname.to_owned(),
            bind_address: bind_host.to_owned(),
            name,
            result: result_mutex,
            shutdown_token,
            sim_time,
        })
    }

    /// Initializes the receiving of the simulated time
    /// This function panics if the '/use_sim_time'
    /// parameter was not found
    pub async fn init_sim_time(&self) -> Result<(), SubscriptionError> {
        self.sim_time
            .as_ref()
            .expect("SimTime was None, this means the /use_sim_time topic was not found")
            .init(self)
            .await
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

    /// Returns true if this node is using the simulated time
    pub fn is_using_sim_time(&self) -> bool {
        self.sim_time.is_some()
    }

    /// Returns the last simulated clock if it is available
    pub fn get_last_sim_clock(&self) -> Option<Duration> {
        self.sim_time.as_ref()?.duration()
    }

    /// Returns a list of all topics
    pub async fn topics(&self) -> Response<Vec<Topic>> {
        self.master.get_topic_types().await
    }

    /// Returns a list of all parameter names
    pub async fn get_all_param_names(&self) -> Response<Vec<String>> {
        self.master.get_all_param_names().await
    }

    /// Return a parameter
    pub fn param(&self, key: impl AsRef<str>) -> Param {
        Param::new(key.as_ref(), self.master.clone())
    }

    pub async fn search_param<'a, T: Deserialize<'a>>(&self, key: impl AsRef<str>) -> Response<T> {
        self.master.search_param(key.as_ref()).await
    }

    /// Connect to a topic
    pub async fn subscribe<T: Message>(
        &self,
        topic: &str,
        queue_size: usize,
    ) -> Result<Subscriber<T>, SubscriptionError> {
        let queue_size = if queue_size == 0 {
            usize::max_value()
        } else {
            queue_size
        };
        Subscriber::new(Arc::clone(&self.slave), topic, queue_size)
            .instrument(tracing::info_span!("subscribe", topic = topic))
            .await
    }
}
