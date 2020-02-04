use failure::Error;
use std::collections::HashMap;
use std::net::SocketAddr;

#[macro_use]
extern crate serde_derive;

mod xmlfmt;

use failure::_core::task::{Context, Poll};
use futures::future;
use hyper::server::conn::AddrIncoming;
use hyper::service::Service;
use hyper::{http, Body, Request};
use std::sync::Arc;
pub use xmlfmt::{Fault, Response, Value};

type Handler = Box<dyn Fn(Vec<Value>) -> Response + Sync + Send>;
type HandlerMap = HashMap<String, Handler>;

struct ServerHandlers {
    handlers: HandlerMap,
    on_missing_method: Handler,
}

impl Default for ServerHandlers {
    fn default() -> Self {
        ServerHandlers {
            handlers: HashMap::new(),
            on_missing_method: Box::new(on_missing_method),
        }
    }
}

/// Helper method that returns an error response indicating a missing method.
fn on_missing_method(_: Vec<Value>) -> Response {
    Err(Fault::new(404, "Requested method does not exist"))
}

/// Helper method that returns an error response indicating a decoding failure.
pub fn on_decode_fail(err: &Error) -> Response {
    Err(Fault::new(
        400,
        format!("Failed to decode request: {}", err),
    ))
}

/// Helper method that returns an error response indicating an encoding failure.
pub fn on_encode_fail(err: &Error) -> Response {
    Err(Fault::new(
        500,
        format!("Failed to encode response: {}", err),
    ))
}

/// A builder to construct an XMLRPC server.
pub struct ServerBuilder {
    handlers: ServerHandlers,
}

impl Default for ServerBuilder {
    fn default() -> Self {
        ServerBuilder {
            handlers: ServerHandlers::default(),
        }
    }
}

impl ServerBuilder {
    pub fn new() -> Self {
        ServerBuilder::default()
    }

    pub fn register_value<K, T>(&mut self, name: K, handler: T)
    where
        K: Into<String>,
        T: Fn(Vec<Value>) -> Response + Send + Sync + 'static,
    {
        self.handlers
            .handlers
            .insert(name.into(), Box::new(handler));
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
        self.handlers.on_missing_method = Box::new(handler);
    }

    pub async fn bind(self, addr: &SocketAddr) -> Result<Server, failure::Error> {
        let handlers = Arc::new(self.handlers);
        let service = ConnectionService {
            handlers: handlers.clone(),
        };
        let server = hyper::Server::try_bind(addr)?.serve(service);
        Ok(Server { server })
    }
}

/// A service that handles requests from remote connections
#[derive(Clone)]
struct HandlerService(Arc<ServerHandlers>);

impl Service<Request<Body>> for HandlerService {
    type Response = http::Response<Body>;
    type Error = hyper::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, _req: Request<Body>) -> Self::Future {
        unimplemented!()
    }
}

/// A service that handles connection requests.
struct ConnectionService {
    handlers: Arc<ServerHandlers>,
}

impl<T> Service<T> for ConnectionService {
    type Response = HandlerService;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, _: T) -> Self::Future {
        future::ok(HandlerService(self.handlers.clone()))
    }
}

/// Server that manages XMLRPC connection requests
pub struct Server {
    server: hyper::Server<AddrIncoming, ConnectionService>,
}
