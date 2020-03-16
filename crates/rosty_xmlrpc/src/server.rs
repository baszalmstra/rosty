use crate::{from_params, into_params, Params};
use bytes::buf::ext::BufExt;
use failure::Error;
pub use failure::SyncFailure;
use futures::{future, TryFutureExt};
use hyper::{http, service::Service, Body, Request};
use std::future::Future;
use std::pin::Pin;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    task::{Context, Poll},
};
pub use xmlrpc_fmt::{value::ToXml, Deserialize, Fault, Response, Serialize, Value};

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

    /// Registers an async XLMRPC call handler that is passed raw xmlrpc Values.
    pub fn register_value_async<K, T, R>(&mut self, name: K, handler: T)
    where
        K: Into<String>,
        R: Future<Output = Response> + Send + 'static,
        T: (Fn(Vec<Value>) -> R) + Send + Sync + 'static,
    {
        self.handlers
            .handlers
            .insert(name.into(), Box::new(move |req| Box::new(handler(req))));
    }

    pub fn register_async<'a, K, Treq, Tres, Thandler, Tef, Tdf, R>(
        &mut self,
        name: K,
        handler: Thandler,
        encode_fail: Tef,
        decode_fail: Tdf,
    ) where
        K: Into<String>,
        Treq: Deserialize<'a> + Send + Sync,
        Tres: Serialize,
        R: Future<Output = std::result::Result<Tres, Fault>> + Send + 'static,
        Thandler: (Fn(Treq) -> R) + Send + Sync + Copy + 'static,
        Tef: (Fn(&Error) -> Response) + Send + Sync + Copy + 'static,
        Tdf: (Fn(&Error) -> Response) + Send + Sync + Copy + 'static,
    {
        self.register_value_async(name, move |req| {
            async move {
                let params = match from_params(req) {
                    Ok(v) => v,
                    Err(err) => {
                        let err = SyncFailure::new(err);
                        return decode_fail(&err.into());
                    }
                };
                let response = handler(params).await?;
                into_params(&response).or_else(|v| encode_fail(&SyncFailure::new(v).into()))
            }
        });
    }

    pub fn register_simple_async<'a, K, Treq, Tres, Thandler, R>(
        &mut self,
        name: K,
        handler: Thandler,
    ) where
        K: Into<String>,
        Treq: Deserialize<'a> + Send + Sync,
        Tres: Serialize,
        R: Future<Output = std::result::Result<Tres, Fault>> + Send + 'static,
        Thandler: Fn(Treq) -> R + Send + Sync + Copy + 'static,
    {
        self.register_async(name, handler, on_encode_fail, on_decode_fail);
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
    pub fn bind<F>(
        self,
        addr: &SocketAddr,
        shutdown_signal: F,
    ) -> Result<(impl Future<Output = Result<(), failure::Error>>, SocketAddr), failure::Error>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let service = ConnectionService {
            handlers: Arc::new(self.handlers),
        };
        let server = hyper::Server::try_bind(addr)?.serve(service);
        let addr = server.local_addr();
        Ok((
            server
                .with_graceful_shutdown(shutdown_signal)
                .map_err(Into::into),
            addr,
        ))
    }
}

/// A service that handles requests from remote connections
#[derive(Clone)]
struct HandlerService(Arc<ServerHandlers>);

type ServerHandlerFuture =
    Pin<Box<dyn Future<Output = Result<http::Response<Body>, hyper::Error>> + Send>>;

impl Service<Request<Body>> for HandlerService {
    type Response = http::Response<Body>;
    type Error = hyper::Error;
    type Future = ServerHandlerFuture;

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
