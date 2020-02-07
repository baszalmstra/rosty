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
        caller_api: &str
    ) -> Result<Self, failure::Error> {
        Ok(Master {
            client: rosxmlrpc::Client::new(master_uri.to_owned())?,
            client_id: client_id.to_owned(),
            caller_api: caller_api.to_owned()
        })
    }

    /// Get the URI of the master.
    pub async fn get_uri(&self) -> Response<String> {
        self.client.request("getUri", &(&self.client_id)).await
    }
}