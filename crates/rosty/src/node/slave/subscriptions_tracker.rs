use crate::node::error::SubscriptionError;
use crate::rosxmlrpc;
use crate::rosxmlrpc::{Response, ResponseError};
use crate::tcpros::{Message, Subscriber};
use futures::TryFutureExt;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct SubscriptionsTracker {
    mapping: Mutex<HashMap<String, Subscriber>>,
}

impl SubscriptionsTracker {
    /// Tries to add a subscription to the tracker
    pub async fn add<T, F>(
        &self,
        name: &str,
        topic: &str,
        queue_size: usize,
        callback: F,
    ) -> Result<(), SubscriptionError>
    where
        T: Message,
        F: Fn(T, &str) + Send + 'static,
    {
        match self.mapping.lock().await.entry(String::from(topic)) {
            Entry::Occupied(..) => Err(SubscriptionError::DuplicateSubscription {
                topic: topic.to_owned(),
            }),
            Entry::Vacant(entry) => {
                let subscriber = Subscriber::new::<F, T>(name, topic, queue_size, callback);
                entry.insert(subscriber);
                Ok(())
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
        if let Some(mut subscription) = self.mapping.lock().await.get_mut(topic) {
            let publishers = futures::future::join_all(
                publishers
                    .filter(|publisher| !subscription.is_connected_to(publisher))
                    .map(|publisher| {
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
                    }),
            )
            .await;
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
        match last_error_message {
            None => Ok(()),
            Some(err) => Err(err),
        }
    }
}

async fn request_topic(
    publisher_uri: String,
    caller_id: &str,
    topic: &str,
) -> Response<(String, String, i32)> {
    let uri = publisher_uri
        .parse()
        .map_err(|e| ResponseError::Client(format!("invalid uri '{}'", publisher_uri)))?;
    rosxmlrpc::Client::new(uri)
        .request("requestTopic", &(caller_id, topic, [["TCPROS"]]))
        .await
}
