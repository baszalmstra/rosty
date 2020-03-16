use crate::node::error::SubscriptionError;
use crate::rosxmlrpc;
use crate::rosxmlrpc::{Response, ResponseError};
use crate::tcpros::{IncomingMessage, Message, Subscriber};
use futures::TryFutureExt;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use tokio::sync::{mpsc, Mutex};

#[derive(Default)]
pub struct SubscriptionsTracker {
    mapping: Mutex<HashMap<String, Subscriber>>,
}

impl SubscriptionsTracker {
    /// Tries to add a subscription to the tracker
    pub async fn add<T: Message>(
        &self,
        name: &str,
        topic: &str,
        queue_size: usize,
    ) -> Result<mpsc::Receiver<IncomingMessage<T>>, SubscriptionError> {
        match self.mapping.lock().await.entry(String::from(topic)) {
            Entry::Occupied(..) => Err(SubscriptionError::DuplicateSubscription {
                topic: topic.to_owned(),
            }),
            Entry::Vacant(entry) => {
                let (subscriber, channel) = Subscriber::new::<T>(name, topic, queue_size);
                entry.insert(subscriber);
                Ok(channel)
            }
        }
    }

    /// Notifies this instance that the given publishers
    pub async fn add_publishers<T>(
        &self,
        topic: &str,
        name: &str,
        publishers: T,
    ) -> Result<(), SubscriptionError>
    where
        T: Iterator<Item = String>,
    {
        let mut last_error_message = None;
        if let Some(subscription) = self.mapping.lock().await.get_mut(topic) {
            // Get information on the topic from all publishers concurrently
            let publishers = futures::future::join_all(
                publishers
                    .filter(|publisher| !subscription.is_connected_to(publisher))
                    .map(|publisher| get_publisher_topic_info(topic, name, publisher)),
            )
            .await;

            // For every publisher, try to connect to it or if there was an error, report the error.
            for publisher in publishers {
                let result = match publisher {
                    Ok((publisher, hostname, port)) => subscription
                        .connect_to(&publisher, (hostname.as_str(), port as u16))
                        .await
                        .map_err(SubscriptionError::PublisherConnectError),
                    Err(e) => Err(e),
                };

                if let Err(e) = result {
                    last_error_message = Some(e)
                }
            }
        }

        // Returns the last error
        match last_error_message {
            None => Ok(()),
            Some(err) => Err(err),
        }
    }

    /// Removes the specified subscription
    pub async fn remove(&self, topic: &str) {
        self.mapping.lock().await.remove(topic);
    }
}

async fn get_publisher_topic_info(
    topic: &str,
    name: &str,
    publisher: String,
) -> Result<(String, String, i32), SubscriptionError> {
    let publisher_result = publisher.clone();
    request_topic(publisher, name, topic)
        .map_err(SubscriptionError::RequestTopicError)
        .and_then(|(protocol, hostname, port)| {
            async move {
                if protocol != "TCPROS" {
                    return Err(SubscriptionError::ProtocolMismatch(protocol));
                }
                Ok((publisher_result, hostname, port))
            }
        })
        .await
}

async fn request_topic(
    publisher_uri: String,
    caller_id: &str,
    topic: &str,
) -> Response<(String, String, i32)> {
    let uri = publisher_uri
        .parse()
        .map_err(|_| ResponseError::Client(format!("invalid uri '{}'", publisher_uri)))?;
    rosxmlrpc::Client::new(uri)
        .request("requestTopic", &(caller_id, topic, [["TCPROS"]]))
        .await
}
