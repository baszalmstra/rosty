use nix::unistd::getpid;

use crate::node::error::SubscriptionError;
use crate::node::master::Master;
use crate::node::slave::publications_tracker::PublicationsTracker;
use crate::node::slave::subscriptions_tracker::SubscriptionsTracker;
use crate::rosxmlrpc::{Params, Response, ResponseError, ServerBuilder, Value};
use crate::shutdown_token::ShutdownToken;
use crate::tcpros::{IncomingMessage, Message, PublisherError, PublisherStream};
use futures::future::TryFutureExt;
use futures::StreamExt;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing_futures::Instrument;

mod publications_tracker;
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
    master: Arc<Master>,
    subscriptions: Arc<SubscriptionsTracker>,
    publications: Arc<PublicationsTracker>,
}

impl Slave {
    /// Constructs a new slave for a node with the given arguments.
    pub async fn new(
        master_uri: &str,
        hostname: &str,
        bind_address: &str,
        port: u16,
        name: &str,
        master: Arc<Master>,
        shutdown_signal: ShutdownToken,
    ) -> Result<(Slave, impl Future<Output = Result<(), failure::Error>>), failure::Error> {
        let subscriptions = Arc::new(SubscriptionsTracker::default());
        let publications = Arc::new(PublicationsTracker::default());

        // Resolve the hostname to an address. 0 for the port indicates that the slave can bind to
        // any port that is available
        let addr = tokio::net::lookup_host((bind_address, port))
            .await?
            .next()
            .ok_or_else(|| format_err!("could not resolve hostname"))?;

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

        let pubs = publications.clone();
        let hostname_string = String::from(hostname);
        server.register_value("requestTopic", "Chosen protocol", move |args| {
            let pubs = pubs.clone();
            let hostname_string = hostname_string.clone();
            async move {
                let mut args = unwrap_array_case(args).into_iter();
                let _caller_id = args.next().ok_or_else(|| ResponseError::Client("missing argument 'caller_id'".into()))?;
                let topic = match args.next() {
                    Some(Value::String(topic)) => topic,
                    _ => return Err(ResponseError::Client("missing argument 'topic'".into()))
                };
                let protocols = match args.next() {
                    Some(Value::Array(protocols)) => protocols,
                    Some(_) => {
                        return Err(ResponseError::Client("protocols need to be provided as [[String, XmlRpcLegalValue]]".into()))
                    }
                    None => return Err(ResponseError::Client("missing argument 'protocols'".into())),
                };
                let port = pubs.get_port(&topic).await.ok_or_else(|| {
                    ResponseError::Client("requested topic not published by node".into())
                })?;
                let ip = hostname_string.clone();
                let has_tcpros = protocols.iter().any(|p| {
                    if let Value::Array(protocol) = p {
                        if let Some(&Value::String(ref name)) = protocol.get(0) {
                            return name == "TCPROS";
                        }
                    }
                    false
                });
                if has_tcpros {
                    Ok(Value::Array(vec![
                        Value::String("TCPROS".into()),
                        Value::String(ip),
                        Value::Int(port as i32),
                    ]))
                } else {
                    Err(ResponseError::Server(
                        "no matching protocols available".into()
                    ))
                }
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
                info!("server is shutting down because: {}", message);
                shutdown_signal.shutdown();
                Ok(Value::Int(0))
            }
        });

        // Start listening for server requests
        let (server, addr) = server.bind(&addr, shutdown_signal.clone())?;
        let uri = format!("http://{}:{}/", hostname, addr.port());

        // Create a future that awaits the server shutdown and then performs cleanup
        let subs = subscriptions.clone();
        let pubs = publications.clone();
        let master_clone = master.clone();
        let caller_api = uri.clone();
        let server = tokio::spawn(async move {
            // Wait for the server to shut down
            server.await?;

            // Release all the subscriptions and publications. Drain the trackers and then tell the
            // master about all the released subscriptions/publications
            let master = master_clone.as_ref();

            futures::stream::iter(subs.remove_all().await.iter())
                .for_each_concurrent(None, |topic| {
                    unregister_subscriber(master, &topic, &caller_api)
                }).await;

            futures::stream::iter(pubs.remove_all().await.iter())
                .for_each_concurrent(None, |topic| {
                    unregister_publisher(master, &topic, &caller_api)
                }).await;

            Ok(())
        })
        .unwrap_or_else(|e| Err(e.into()));

        Ok((
            Slave {
                name: name.to_owned(),
                uri,
                master,
                subscriptions,
                publications,
            },
            server,
        ))
    }

    /// Returns the listen URI of the slave
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Adds a new subscription to list of tracked subscriptions
    pub async fn add_subscription<T: Message>(
        &self,
        topic: &str,
        queue_size: usize,
    ) -> Result<mpsc::Receiver<IncomingMessage<T>>, SubscriptionError> {
        // Add the subscriptions to the list of subscribers
        let receiver = self
            .subscriptions
            .add(&self.name, topic, queue_size)
            .await?;

        // Notify the master that we are subscribing to the given topic. The master will return
        // a list of publishers that publish to the topic we want to subscribe to.
        let publishers = self
            .master
            .register_subscriber(topic, &T::msg_type(), self.uri())
            .await
            .map_err(SubscriptionError::MasterCommunicationError)?;

        info!(topic = topic, "successfully registered subscriber");

        // Let the slave know which nodes are publishing data for the topic so that the slave will
        // connect to them to receive the data
        self.add_publishers_to_subscription(topic, publishers.into_iter())
            .await?;

        Ok(receiver)
    }

    /// Removes the specified subscription
    pub async fn remove_subscription(&self, topic: &str) {
        // Remove the subscription from the list of subscriptions
        if self.subscriptions.remove(topic).await {
            // Notify the master about the unsubscription
            unregister_subscriber(&self.master, topic, self.uri()).await
        }
    }

    /// Tell the slave that the specified `publishers` publish data to the given topic. The slave
    /// will try to connect to the publishers.
    async fn add_publishers_to_subscription<T>(
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

    pub async fn add_publication<T>(
        &self,
        hostname: &str,
        topic: &str,
        queue_size: usize,
    ) -> Result<PublisherStream<T>, PublisherError>
    where
        T: Message,
    {
        // Create the publisher object to be able to actually publish data
        let publisher = self
            .publications
            .add(hostname, topic, queue_size, &self.name)
            .await?;

        // Register the publisher with the master
        self.master
            .register_publisher(topic, &T::msg_type(), &self.uri)
            .await
            .map_err(PublisherError::RegistrationError)?;

        info!(topic = topic, "successfully registered publisher");

        Ok(publisher)
    }

    /// Removes the specified publisher
    pub async fn remove_publisher(&self, topic: &str) {
        // Remove the publisher from the list of publications
        if self.publications.remove(topic).await {
            // Notify the master about the unsubscription
            unregister_publisher(&self.master, topic, self.uri()).await
        }
    }
}

/// Unregister the given topic from the master and report on it
async fn unregister_subscriber(master: &Master, topic: &str, caller_api: &str) {
    match master.unregister_subscriber(&topic, caller_api).await {
        Err(e) => error!(
            topic = topic,
            "error unregistering subscriber with master: {}", e
        ),
        _ => info!(topic = topic, "successfully unregistered subscriber"),
    };
}

/// Unregister the given topic from the master and report on it
async fn unregister_publisher(master: &Master, topic: &str, caller_api: &str) {
    match master.unregister_publisher(&topic, caller_api).await {
        Err(e) => error!(
            topic = topic,
            "error unregistering publisher from master: {}", e
        ),
        _ => info!(topic = topic, "successfully unregistered publisher"),
    };
}
