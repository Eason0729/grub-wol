use std::{future::Future, io, pin::Pin, rc::Rc};

use hyper::{server::conn::Http, service::service_fn, Body, Method, Request, Response};
use indexmap::IndexMap;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use smol::Async;
use std::net::{SocketAddr, TcpListener};

use super::compat::{SmolExecutor, SmolStream};

type PFC<I, O> = dyn Fn(I) -> Pin<Box<dyn Future<Output = Result<O, Error>> + Send + Sync>>;

#[derive(Clone)]
struct Handler {
    f: Rc<PFC<Request<Body>, Response<Body>>>,
}

impl Handler {
    fn new(f: Box<PFC<Request<Body>, Response<Body>>>) -> Self {
        Self { f: Rc::new(f) }
    }
    fn json<F, I, O>(f: &'static F) -> Self
    where
        F: Fn(I) -> Pin<Box<dyn Future<Output = Result<O, Error>> + Send + Sync>> + Send + Sync,
        I: for<'c> Deserialize<'c> + Send + Sync,
        O: Serialize,
    {
        Self {
            f: Rc::new(|req| {
                Box::pin(async {
                    let bytes = hyper::body::to_bytes(req.into_body()).await?;
                    let data: I =
                        serde_json::from_slice(&bytes).map_err(|e| Error::DeserializeError(e))?;

                    let output = serde_json::to_vec(&f(data).await.unwrap())
                        .map_err(|e| Error::SerializeError(e))?;

                    Response::builder()
                        .header("Content-Type", "application/json")
                        .body(Body::from(output))
                        .map_err(|e| e.into())
                })
            }),
        }
    }
    fn execute(
        &self,
        req: Request<Body>,
    ) -> Pin<Box<dyn Future<Output = Result<Response<Body>, Error>> + Send + Sync>> {
        (self.f)(req)
    }
}

pub struct WebServer<'a> {
    route: IndexMap<Route<'a>, Handler>,
}

impl<'a> WebServer<'a> {
    fn new() -> Self {
        Self {
            route: Default::default(),
        }
    }
    fn add_route(&mut self, route: Route<'a>, handler: Handler) {
        assert!(self.route.insert(route, handler).is_none());
    }
    pub async fn listen(&'a self, socket: SocketAddr) -> Result<(), Error> {
        let listener: Async<TcpListener> = Async::<TcpListener>::bind(socket)?;
        let http = Http::new().with_executor(SmolExecutor);

        loop {
            let (stream, _) = listener.accept().await?;
            let stream = SmolStream::from_plain(stream);

            if let Err(err) = http
                .serve_connection(
                    stream,
                    service_fn(move |request| {
                        if let Some(f) = self.get_handler(&request) {
                            f.execute(request)
                        } else {
                            default_handler()
                        }
                    }),
                )
                .await
            {
                warn!("Hyper throw an error at HTTP Parsing: {}", err);
            };
        }
    }
    fn get_handler(&self, request: &Request<Body>) -> Option<Handler> {
        let route = Route::from_request(&request);
        match self.route.get(&route) {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }
}

fn default_handler() -> Pin<Box<dyn Future<Output = Result<Response<Body>, Error>> + Send + Sync>> {
    Box::pin(async { Ok(Response::new(Body::from("No such route!"))) })
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub enum Route<'a> {
    GET(&'a str),
    POST(&'a str),
    OTHER(&'a str),
}

impl<'a> Route<'a> {
    fn from_request(request: &'a Request<Body>) -> Self {
        if request.method() == &Method::GET {
            Route::GET(request.uri().path())
        } else if request.method() == &Method::POST {
            Route::POST(request.uri().path())
        } else {
            Route::OTHER(request.uri().path())
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error")]
    IoError(#[from] io::Error),
    #[error("hyper error")]
    HyperError(#[from] hyper::Error),
    #[error("Client probably not follow protocal")]
    HyperHttpError(#[from] hyper::http::Error),
    #[error("Client probably not follow protocal")]
    SerializeError(serde_json::Error),
    #[error("Client probably not follow protocal")]
    DeserializeError(serde_json::Error),
}

#[cfg(test)]
mod test {
    use std::{
        cell::RefCell,
        net::{IpAddr, Ipv4Addr},
    };

    use super::*;

    fn stiaic() {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);

        let mut server = WebServer::new();
        server.add_route(
            Route::GET("/helloworld.html"),
            Handler::new(Box::new(|req| {
                Box::pin(async {
                    let res = Response::builder()
                        .header("Content-Type", "text/html")
                        .body(Body::from(
                            include_bytes!("test/helloworld.html").as_slice(),
                        ));
                    res.map_err(|e| e.into())
                })
            })),
        );
        smol::block_on(server.listen(socket)).unwrap();
    }
    #[test]
    fn read_body() {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);

        let mut server = WebServer::new();
        let cc = Box::leak(Box::new(RefCell::new(0_usize)));
        server.add_route(
            Route::POST("/read_body"),
            Handler::new(Box::new(|req| {
                Box::pin(async {
                    let body = req.into_body();
                    let bytes = hyper::body::to_bytes(body).await?;

                    println!("{:?}", bytes);

                    let res = Response::builder()
                        .header("Content-Type", "text/html")
                        .body(Body::from(
                            include_bytes!("test/helloworld.html").as_slice(),
                        ));
                    res.map_err(|e| e.into())
                })
            })),
        );
        smol::block_on(server.listen(socket)).unwrap();
    }
}
