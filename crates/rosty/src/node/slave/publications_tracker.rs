use crate::tcpros::{Message, Publisher, PublisherError, PublisherStream};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct PublicationsTracker {
    mapping: Mutex<HashMap<String, Publisher>>,
}

impl PublicationsTracker {
    pub async fn add<T: Message>(
        &self,
        hostname: &str,
        topic: &str,
        queue_size: usize,
        caller_id: &str,
    ) -> Result<PublisherStream<T>, PublisherError> {
        match self.mapping.lock().await.entry(topic.to_owned()) {
            Entry::Occupied(entry) => entry.get().stream::<T>(queue_size),
            Entry::Vacant(entry) => {
                let publisher = Publisher::new::<T, _>(
                    format!("{}:0", hostname).as_str(),
                    topic,
                    queue_size,
                    caller_id,
                )
                .await?;
                entry.insert(publisher).stream::<T>(queue_size)
            }
        }
    }

    /// Returns the port that the publisher of the specified topic is listening on, or None if no
    /// such publisher exists.
    pub async fn get_port(&self, topic: &str) -> Option<u16> {
        self.mapping
            .lock()
            .await
            .get(topic)
            .map(|publisher| publisher.port)
    }

    /// Removes the specified publications
    pub async fn remove(&self, topic: &str) -> bool {
        self.mapping.lock().await.remove(topic).is_some()
    }

    /// Removes all the publications and returns an iterator of all the topics that were released.
    pub async fn remove_all(&self) -> Vec<String> {
        self.mapping.lock().await.drain().map(|(k, _)| k).collect()
    }
}
