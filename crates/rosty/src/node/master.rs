use crate::rosxmlrpc;
use crate::rosxmlrpc::Response;

/// Implements an API to communicate with the ROS master
pub struct Master {
    client: rosxmlrpc::Client,
    client_id: String,
    caller_api: String,
}

impl Master {
    pub fn new(
        master_uri: &str,
        client_id: &str,
        caller_api: &str,
    ) -> Result<Self, failure::Error> {
        Ok(Master {
            client: rosxmlrpc::Client::new(master_uri.parse()?),
            client_id: client_id.to_owned(),
            caller_api: caller_api.to_owned(),
        })
    }

    /// Get the URI of the master.
    pub async fn get_uri(&self) -> Response<String> {
        self.client.request("getUri", &(&self.client_id)).await
    }

    pub async fn get_topic_types(&self) -> Response<Vec<Topic>> {
        self.client
            .request("getTopicTypes", &(&self.client_id))
            .await
            .map(|v: Vec<(String, String)>| {
                v.into_iter()
                    .map(|(name, data_type)| Topic { name, data_type })
                    .collect()
            })
    }

    #[allow(dead_code)]
    pub async fn lookup_node(&self, node_name: &str) -> Response<String> {
        self.client
            .request("lookupNode", &(&self.client_id, &node_name))
            .await
    }

    pub async fn register_subscriber(
        &self,
        topic: &str,
        topic_type: &str,
    ) -> Response<Vec<String>> {
        self.client
            .request(
                "registerSubscriber",
                &(&self.client_id, topic, topic_type, &self.caller_api),
            )
            .await
    }

    pub async fn unregister_subscriber(&self, topic: &str) -> Response<i32> {
        self.client
            .request(
                "unregisterSubscriber",
                &(&self.client_id, topic, &self.caller_api),
            )
            .await
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Topic {
    pub name: String,
    pub data_type: String,
}
