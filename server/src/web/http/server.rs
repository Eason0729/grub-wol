use std::{future::Future, io, pin::Pin};

use hyper::{server::conn::Http, service::service_fn, Body, Method, Request, Response};
use indexmap::IndexMap;
use smol::Async;
use std::net::{SocketAddr, TcpListener};

use super::compat::{SmolExecutor, SmolStream};

pub struct WebServer<'a, F>
where
    F: Fn(
        &Request<Body>,
    ) -> Pin<Box<dyn Future<Output = Result<Response<Body>, Error>> + Send + Sync>>,
{
    route: IndexMap<Route<'a>, F>,
}

fn default_handler() -> Pin<Box<dyn Future<Output = Result<Response<Body>, Error>> + Send + Sync>> {
    Box::pin(async { Ok(Response::new(Body::from("No such route!"))) })
}

impl<'a, F> WebServer<'a, F>
where
    F: Fn(
        &Request<Body>,
    ) -> Pin<Box<dyn Future<Output = Result<Response<Body>, Error>> + Send + Sync>>,
{
    pub fn new() -> Self {
        Self {
            route: Default::default(),
        }
    }
    pub async fn listen(&'a self, socket: SocketAddr) -> Result<(), Error> {
        let listener: Async<TcpListener> = Async::<TcpListener>::bind(socket)?;
        let http = Http::new().with_executor(SmolExecutor);

        loop {
            let (stream, _) = listener.accept().await?;
            let stream = SmolStream::from_plain(stream);

            http.serve_connection(
                stream,
                service_fn(move |request| match self.get_handler(&request) {
                    Some(f) => f(&request),
                    None => default_handler(),
                }),
            )
            .await?;
        }
    }
    fn get_handler<'b>(&'b self, request: &'b Request<Body>) -> Option<&F> {
        let route = Route::from_request(&request);
        self.route.get(&route)
    }
    pub fn add_route(&mut self, route: Route<'a>, f: F) {
        assert!(self.route.insert(route, f).is_none());
    }
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
}

#[cfg(test)]
mod test {
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;

    fn stiaic() {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);

        let mut server = WebServer::new();
        server.add_route(Route::GET("/helloworld.html"), |req| {
            Box::pin(async {
                let res = Response::builder()
                    .header("Content-Type", "text/html")
                    .body(Body::from(
                        include_bytes!("test/helloworld.html").as_slice(),
                    ));
                res.map_err(|e| e.into())
            })
        });

        smol::block_on(server.listen(socket)).unwrap();
    }
}
