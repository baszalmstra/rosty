use crate::Params;
use bytes::buf::ext::BufExt;
use failure::Error;
use futures::future;
use hyper::{http, server::conn::AddrIncoming, service::Service, Body, Request};
use std::future::Future;
use std::pin::Pin;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    task::{Context, Poll},
};
pub use xmlrpc_fmt::{value::ToXml, Fault, Response, Value};

type Handler = Box<dyn (Fn(Vec<Value>) -> Box<dyn Future<Output = Response> + Send>) + Sync + Send>;
type HandlerMap = HashMap<String, Handler>;

struct ServerHandlers {
    handlers: HandlerMap,
    on_missing_method: Handler,
}

impl Default for ServerHandlers {
    fn default() -> Self {
        ServerHandlers {
            handlers: HashMap::new(),
            on_missing_method: Box::new(|req| Box::new(on_missing_method(req))),
        }
    }
}

/// Helper method that returns an error response indicating a missing method.
pub async fn on_missing_method(_: Vec<Value>) -> Response {
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

    /// Registers a XLMRPC call handler
    pub fn register_value<K, T, R>(&mut self, name: K, handler: T)
    where
        K: Into<String>,
        R: Future<Output = Response> + Send + 'static,
        T: (Fn(Vec<Value>) -> R) + Send + Sync + 'static,
    {
        self.handlers
            .handlers
            .insert(name.into(), Box::new(move |req| Box::new(handler(req))));
    }

    /// Sets the handler that is called if none of the other handlers match.
    pub fn set_on_missing<T, R>(&mut self, handler: T)
    where
        R: Future<Output = Response> + Send + 'static,
        T: (Fn(Params) -> R) + Send + Sync + 'static,
    {
        self.handlers.on_missing_method = Box::new(move |req| Box::new(handler(req)));
    }

    /// Constructs the actual `Server` by creating a binding to the specific `SocketAddr`.
    pub async fn bind(self, addr: &SocketAddr) -> Result<Server, failure::Error> {
        let service = ConnectionService {
            handlers: Arc::new(self.handlers),
        };
        let server = hyper::Server::try_bind(addr)?.serve(service);
        Ok(Server { _server: server })
    }
}

/// A service that handles requests from remote connections
#[derive(Clone)]
struct HandlerService(Arc<ServerHandlers>);

impl Service<Request<Body>> for HandlerService {
    type Response = http::Response<Body>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        Box::pin(self.clone().call_inner(req))
    }
}

impl HandlerService {
    async fn call_inner(self, req: Request<Body>) -> Result<http::Response<Body>, hyper::Error> {
        let body = hyper::body::aggregate(req).await?;

        // Parse the body as an XMLRPC call
        let call = match xmlrpc_fmt::parse::call(body.reader()) {
            Ok(call) => call,
            Err(_) => {
                return Ok(http::Response::builder()
                    .status(400)
                    .body("".into())
                    .unwrap())
            }
        };

        // Handle the call
        let call_result = self.handle(call).await;
        Ok(http::Response::builder()
            .status(hyper::StatusCode::OK)
            .header(hyper::header::CONTENT_TYPE, "text/xml")
            .body(call_result.to_xml().into())
            .unwrap())
    }

    fn handle(
        &self,
        req: xmlrpc_fmt::Call,
    ) -> Pin<Box<dyn Future<Output = xmlrpc_fmt::Response> + Send>> {
        Pin::from(self
            .0
            .handlers
            .get(&req.name)
            .unwrap_or(&self.0.on_missing_method)(req.params))
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
    _server: hyper::Server<AddrIncoming, ConnectionService>,
}
