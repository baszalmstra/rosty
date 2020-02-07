use crate::server::{Fault, SyncFailure};
use bytes::buf::BufExt;
use hyper::client::HttpConnector;
use hyper::{self, Body, Client as HyperClient, Method, Request};
use xmlrpc_fmt::{from_params, into_params, parse, Call, Params};
use xmlrpc_fmt::{Deserialize, Serialize};

use crate::Response;
pub use hyper::Uri;

/// An XML-Rpc client
pub struct Client {
    hyper_client: HyperClient<HttpConnector>,
}

impl Default for Client {
    fn default() -> Self {
        Client {
            hyper_client: HyperClient::default(),
        }
    }
}

impl Client {
    pub fn new() -> Self {
        Self::default()
    }

    /// Internal call for call with params
    pub async fn call_with_params<TKey: Into<String>>(
        &mut self,
        uri: &hyper::Uri,
        name: TKey,
        params: Params,
    ) -> Result<Response, failure::Error> {
        use xmlrpc_fmt::value::ToXml;
        // Convert the body to a xml-rpc call
        let body_str = Call {
            name: name.into(),
            params,
        }
        .to_xml();

        // Build the actual request
        let req = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header("content-type", "text/xml")
            .body(Body::from(body_str))
            .expect("Cannot create hyper request");

        // Do the actual request
        let response = self.hyper_client.request(req).await?;
        let response = hyper::body::aggregate(response).await?;

        parse::response(response.reader())
            .map_err(SyncFailure::new)
            .map_err(Into::into)
    }

    /// Do a an xml-rpc call for a number of parameters
    pub async fn call<'a, TKey, TParams, TResponse>(
        &mut self,
        uri: &hyper::Uri,
        name: TKey,
        params: TParams,
    ) -> Result<Result<TResponse, Fault>, failure::Error>
    where
        TKey: Into<String>,
        TParams: Serialize,
        TResponse: Deserialize<'a>,
    {
        let into_params_result = into_params(&params).map_err(SyncFailure::new)?;
        let parsed_response = self.call_with_params(uri, name, into_params_result).await;
        match parsed_response {
            // Request was Ok
            Ok(Ok(v)) => from_params(v)
                .map(Ok)
                .map_err(SyncFailure::new)
                .map_err(Into::into),
            Ok(Err(e)) => Ok(Err(e)),
            Err(v) => Err(v),
        }
    }
}

pub async fn call_with_params<Tkey>(
    uri: &Uri,
    name: Tkey,
    params: Params,
) -> Result<Response, failure::Error>
where
    Tkey: Into<String>,
{
    Client::new().call_with_params(uri, name, params).await
}

pub async fn call<'a, Tkey, Treq, Tres>(
    uri: &Uri,
    name: Tkey,
    req: Treq,
) -> Result<std::result::Result<Tres, Fault>, failure::Error>
where
    Tkey: Into<String>,
    Treq: Serialize,
    Tres: Deserialize<'a>,
{
    Client::new().call(uri, name, req).await
}
