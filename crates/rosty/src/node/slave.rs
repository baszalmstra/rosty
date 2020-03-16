use async_std::net::ToSocketAddrs;
use nix::unistd::getpid;

use crate::node::error::SubscriptionError;
use crate::node::shutdown_token::ShutdownToken;
use crate::node::slave::subscriptions_tracker::SubscriptionsTracker;
use crate::rosxmlrpc::{Params, Response, ResponseError, ServerBuilder, Value};
use crate::tcpros::Message;
use futures::future::TryFutureExt;
use std::future::Future;
use std::sync::Arc;
use tracing_futures::Instrument;

mod subscriptions_tracker;

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
    subscriptions: Arc<SubscriptionsTracker>,
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
        let subscriptions = Arc::new(SubscriptionsTracker::default());

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

        server.register_value("getPid", "PID", |_args| {
            async { Ok(Value::Int(getpid().into())) }
        });

        let name_string = String::from(name);
        let subs = subscriptions.clone();
        server.register_value("publisherUpdate", "Publishers updated", move |args| {
            let subs = subs.clone();
            let name_string = name_string.clone();
            async move {
                let mut args = unwrap_array_case(args).into_iter();
                let caller_id = match args
                    .next() {
                    Some(Value::String(caller_id)) => caller_id,
                    _ => return Err(ResponseError::Client("missing argument 'caller_id'".to_owned()))
                };
                let topic = match args.next() {
                    Some(Value::String(topic)) => topic,
                    _ => return Err(ResponseError::Client("missing argument 'topic'".to_owned()))
                };
                let publishers = match args.next() {
                    Some(Value::Array(publishers)) => publishers,
                    _ => return Err(ResponseError::Client("missing argument 'publishers'".to_owned()))
                };
                let publishers = publishers.into_iter().map(|v| match v {
                    Value::String(x) => Ok(x),
                    _ => Err(ResponseError::Client("publishers need to be strings".to_owned()))
                })
                    .collect::<Response<Vec<String>>>()?;
                subs.add_publishers(&topic, &name_string, publishers.iter().cloned())
                    .instrument(tracing::trace_span!("publisherUpdate", caller_id=caller_id.as_str(), topic=topic.as_str(), publishers=?publishers))
                    .await
                    .map_err(|v| {
                        ResponseError::Server(format!("failed to handle publishers: {}", v))
                    })?;
                Ok(Value::Int(0))
            }
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
                subscriptions,
            },
            server,
        ))
    }

    /// Returns the listen URI of the slave
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Adds a new subscription to list of tracked subscriptions
    pub async fn add_subscription<T, F>(
        &self,
        topic: &str,
        queue_size: usize,
        callback: F,
    ) -> Result<(), SubscriptionError>
    where
        T: Message,
        F: Fn(T, &str) + Send + 'static,
    {
        self.subscriptions
            .add(&self.name, topic, queue_size, callback)
            .await
    }

    /// Tell the slave that the specified `publishers` publish data to the given topic. The slave
    /// will try to connect to the publishers.
    pub async fn add_publishers_to_subscription<T>(
        &self,
        topic: &str,
        publishers: T,
    ) -> Result<(), SubscriptionError>
    where
        T: Iterator<Item = String>,
    {
        self.subscriptions
            .add_publishers(topic, &self.name, publishers)
            .await
    }
}
