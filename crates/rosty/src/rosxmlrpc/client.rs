use crate::rosxmlrpc::response_info::ResponseInfo;
use crate::rosxmlrpc::{Params, Response, ResponseError, Value};
use serde::{Deserialize, Serialize};

pub struct Client {
    master_uri: xmlrpc::Uri,
}

impl Client {
    pub fn new(master_uri: String) -> Result<Client, failure::Error> {
        let master_uri = master_uri.parse()?;
        Ok(Client { master_uri })
    }

    pub async fn request_tree_with_tree(&self, name: &str, params: Params) -> Response<Value> {
        let call_result = xmlrpc::call_with_params(&self.master_uri, name, params).await;

        let server_response = call_result.map_err(|err| {
            ResponseError::Client(format!("Failed to perform call to server: {}", err))
        })?;

        let response_parameters = server_response.map_err(|fault| {
            ResponseError::Client(format!(
                "Unexpected fault #{} received from server: {}",
                fault.code, fault.message
            ))
        })?;

        let response_parameters = remove_array_wrappers(&response_parameters[..]);

        ResponseInfo::try_from_array(response_parameters)?.into()
    }

    pub async fn request_tree<S>(&self, name: &str, params: &S) -> Response<Value>
    where
        S: Serialize,
    {
        let params = xmlrpc::into_params(params).map_err(bad_request_structure)?;
        self.request_tree_with_tree(name, params).await
    }

    pub async fn request<'a, S, D>(&self, name: &str, params: &S) -> Response<D>
    where
        S: Serialize,
        D: Deserialize<'a>,
    {
        let data = self.request_tree(name, params).await?;
        Deserialize::deserialize(data).map_err(bad_response_structure)
    }
}

fn bad_request_structure<T: ::std::fmt::Display>(err: T) -> ResponseError {
    ResponseError::Client(format!("Failed to serialize parameters: {}", err))
}

fn bad_response_structure<T: ::std::fmt::Display>(err: T) -> ResponseError {
    ResponseError::Server(format!("Response data has unexpected structure: {}", err))
}

fn remove_array_wrappers(mut data: &[Value]) -> &[Value] {
    while let [Value::Array(ref children)] = data[..] {
        data = children;
    }
    data
}
