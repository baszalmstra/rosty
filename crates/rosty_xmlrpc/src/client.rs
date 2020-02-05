use crate::server::Fault;
use bytes::buf::BufExt;
use bytes::Buf;
use hyper::client::HttpConnector;
use hyper::{self, Body, Client as HyperClient, Method, Request};
use xmlrpc_fmt::{from_params, into_params, parse, Call, Params};
use xmlrpc_fmt::{Deserialize, Serialize};

/// An XML-Rpc client
struct Client {
    hyper_client: HyperClient<HttpConnector>,
}

impl Client {
    pub fn new() -> Client {
        Client {
            hyper_client: HyperClient::new(),
        }
    }

    /// Internal call for call with params
    async fn call_with_params<TKey: Into<String>>(
        &mut self,
        uri: &hyper::Uri,
        name: TKey,
        params: Params,
    ) -> Result<impl Buf, hyper::Error> {
        use xmlrpc_fmt::value::ToXml;
        // Convert the body to a xml-rpc call
        let body_str = Call {
            name: name.into(),
            params,
        }
        .to_xml();

        // Build the actual request
        let req = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("content-type", "text/xml")
            .body(Body::from(body_str))
            .expect("Cannot create hyper request");

        // Do the actual request
        let response = self.hyper_client.request(req).await?;
        hyper::body::aggregate(response).await
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
        let into_params_result =
            into_params(&params).map_err(|e| failure::format_err!("{}", e.description()))?;
        let response = self.call_with_params(uri, name, into_params_result).await?;

        let parsed_response = parse::response(response.reader());
        match parsed_response {
            // Request was Ok
            Ok(Ok(v)) => from_params(v)
                .map(Ok)
                .map_err(|e| failure::format_err!("{}", e.description()))?,
            Ok(Err(e)) => Ok(Err(e)),
            Err(v) => Err(failure::format_err!("{}", v.description())),
        }
    }
}
