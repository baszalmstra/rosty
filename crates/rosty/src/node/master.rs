use crate::rosxmlrpc;
use crate::rosxmlrpc::Response;
use serde::{Deserialize, Serialize};

/// Implements an API to communicate with the ROS master
pub struct Master {
    client: rosxmlrpc::Client,
    client_id: String,
}

impl Master {
    pub fn new(master_uri: &str, client_id: &str) -> Result<Self, failure::Error> {
        Ok(Master {
            client: rosxmlrpc::Client::new(master_uri.parse()?),
            client_id: client_id.to_owned(),
        })
    }

    /// Get the URI of the master.
    pub async fn get_uri(&self) -> Response<String> {
        self.client.request("getUri", &(&self.client_id)).await
    }

    /// Get all the topics currently registered with the ROS master
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

    /// Get the all the ros parameter names on the server
    pub async fn get_all_param_names(&self) -> Response<Vec<String>> {
        self.client
            .request("getParamNames", &(&self.client_id))
            .await
    }

    /// Get a ros parameter from the parameter server
    pub async fn get_param<'a, T: Deserialize<'a>>(&self, key: impl AsRef<str>) -> Response<T> {
        self.client
            .request("getParam", &(&self.client_id, key.as_ref()))
            .await
    }

    /// Find closest parameter name,
    /// starting in the private namespace and searching upwards to the global namespace.
    ///If this code appears in the node /foo/bar, rospy.search_param will try to find the parameters:
    /// * /foo/bar/global_example
    /// * /foo/global_example
    /// * /global_example
    ///in this order.
    pub async fn search_param<'a, T: Deserialize<'a>>(&self, key: impl AsRef<str>) -> Response<T> {
        self.client
            .request("searchParam", &(&self.client_id, key.as_ref()))
            .await
    }

    /// Delete a parameter from the parameter server
    pub async fn delete_param(&self, key: impl AsRef<str>) -> Response<i32> {
        self.client
            .request("deleteParam", &(&self.client_id, key.as_ref()))
            .await
    }

    /// Set a ros parameter from the parameter server
    pub async fn set_param<T: Serialize>(&self, key: impl AsRef<str>, value: &T) -> Response<i32> {
        self.client
            .request("setParam", &(&self.client_id, key.as_ref(), value))
            .await
    }

    /// Set a ros parameter from the parameter server
    pub async fn has_param(&self, key: impl AsRef<str>) -> Response<bool> {
        self.client
            .request("hasParam", &(&self.client_id, key.as_ref()))
            .await
    }

    #[allow(dead_code)]
    pub async fn lookup_node(&self, node_name: &str) -> Response<String> {
        self.client
            .request("lookupNode", &(&self.client_id, &node_name))
            .await
    }

    /// Subscribe the caller to the specified topic. In addition to receiving a list of current
    /// publishers, the subscriber will also receive notifications of new publishers via the
    /// publisherUpdate API.
    pub async fn register_subscriber(
        &self,
        topic: &str,
        topic_type: &str,
        caller_api: &str,
    ) -> Response<Vec<String>> {
        self.client
            .request(
                "registerSubscriber",
                &(&self.client_id, topic, topic_type, caller_api),
            )
            .await
    }

    /// Unregister the caller as a publisher of the topic
    pub async fn unregister_subscriber(&self, topic: &str, caller_api: &str) -> Response<i32> {
        self.client
            .request(
                "unregisterSubscriber",
                &(&self.client_id, topic, caller_api),
            )
            .await
    }

    /// Register the publisher of the given topic with the master
    pub async fn register_publisher(&self, topic: &str, topic_type: &str, caller_api: &str) -> Response<Vec<String>> {
        self.client
            .request(
                "registerPublisher",
                &(&self.client_id, topic, topic_type, caller_api),
            )
            .await
    }

    /// Deregister the publisher of the given topic from the master
    pub async fn unregister_publisher(&self, topic: &str, caller_api: &str) -> Response<i32> {
        self.client
            .request(
                "unregisterPublisher",
                &(&self.client_id, topic, caller_api),
            )
            .await
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Topic {
    pub name: String,
    pub data_type: String,
}
