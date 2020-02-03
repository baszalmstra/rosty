use std::net::SocketAddr;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use failure::Error;

#[macro_use]
extern crate serde_derive;

mod xmlfmt;

pub use xmlfmt::{Value, Fault, Response};
use hyper::{Body, Request, http};
use hyper::service::{Service, service_fn, make_service_fn};
use failure::_core::task::{Context, Poll};
use std::future::Future;
use std::sync::Arc;
use failure::_core::convert::Infallible;

type Handler = Box<dyn Fn(Vec<Value>) -> Response + Sync + Send>;
type HandlerMap = HashMap<String, Handler>;

pub struct Server {
    handlers: HandlerMap,
    on_missing_method: Handler,
}

fn on_missing_method(_: Vec<Value>) -> Response {
    Err(Fault::new(404, "Requested method does not exist"))
}

pub fn on_decode_fail(err: &Error) -> Response {
    Err(Fault::new(
        400,
        format!("Failed to decode request: {}", err),
    ))
}

pub fn on_encode_fail(err: &Error) -> Response {
    Err(Fault::new(
        500,
        format!("Failed to encode response: {}", err),
    ))
}

impl Default for Server {
    fn default() -> Self {
        Server {
            handlers: HashMap::new(),
            on_missing_method: Box::new(on_missing_method)
        }
    }
}

impl Server {
    pub fn new() -> Self {
        Server::default()
    }

    pub fn register_value<K, T>(&mut self, name: K, handler: T)
        where
            K: Into<String>,
            T: Fn(Vec<Value>) -> Response + Send + Sync + 'static,
    {
        self.handlers.insert(name.into(), Box::new(handler));
    }
//
//    pub fn register<'a, K, Treq, Tres, Thandler, Tef, Tdf>(
//        &mut self,
//        name: K,
//        handler: Thandler,
//        encode_fail: Tef,
//        decode_fail: Tdf,
//    ) where
//        K: Into<String>,
//        Treq: Deserialize<'a>,
//        Tres: Serialize,
//        Thandler: Fn(Treq) -> std::result::Result<Tres, Fault> + Send + Sync + 'static,
//        Tef: Fn(&failure::Error) -> Response + Send + Sync + 'static,
//        Tdf: Fn(&failure::Error) -> Response + Send + Sync + 'static,
//    {
//        self.register_value(name, move |req| {
//            let params = match from_params(req) {
//                Ok(v) => v,
//                Err(err) => return decode_fail(&err),
//            };
//            let response = handler(params)?;
//            into_params(&response).or_else(|v| encode_fail(&v))
//        });
//    }
//
//    pub fn register_simple<'a, K, Treq, Tres, Thandler>(&mut self, name: K, handler: Thandler)
//        where
//            K: Into<String>,
//            Treq: Deserialize<'a>,
//            Tres: Serialize,
//            Thandler: Fn(Treq) -> std::result::Result<Tres, Fault> + Send + Sync + 'static,
//    {
//        self.register(name, handler, on_encode_fail, on_decode_fail);
//    }

    pub fn set_on_missing<T>(&mut self, handler: T)
        where
            T: Fn(Vec<Value>) -> Response + Send + Sync + 'static,
    {
        self.on_missing_method = Box::new(handler);
    }

    pub async fn bind(self, addr: &SocketAddr) -> Result<BoundServer, failure::Error> {
        let server = Arc::new(self);
        let handle = hyper::Server::bind(addr).serve(make_service_fn(move |transport| {
            let inner = server.clone();
            futures::future::ok::<_, Infallible>(service_fn(move |req| {
                futures::future::ok(http::Response::new(Body::empty()))
            }))
        }));
        Ok(BoundServer { })
    }
}

struct BoundServer {

}