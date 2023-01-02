use std::{
    future::Future, io, marker::PhantomData, num::NonZeroUsize, pin::Pin, sync::Arc,
    thread::available_parallelism,
};

use hyper::{
    server::conn::Http,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response,
};
use indexmap::IndexMap;
use log::{info, warn};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use smol::Async;
use std::net::{SocketAddr, TcpListener};

use super::compat::{SmolExecutor, SmolListener, SmolStream};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error")]
    IoError(#[from] io::Error),
    #[error("hyper error")]
    HyperError(#[from] hyper::Error),
    #[error("Client probably not follow http protocal")]
    HyperHttpError(#[from] hyper::http::Error),
    #[error("Error serializing json")]
    SerializeError(serde_json::Error),
    #[error("Error deserializing json")]
    DeserializeError(serde_json::Error),
}

type PinFut<O> = Pin<Box<dyn Future<Output = O> + Send + Sync>>;

pub struct Handler<S> {
    f: Arc<dyn Fn(Request<Body>, Arc<S>) -> PinFut<Result<Response<Body>, Error>> + Send + Sync>,
}

impl<S> Clone for Handler<S> {
    fn clone(&self) -> Self {
        Self { f: self.f.clone() }
    }
}

impl<S> Handler<S> {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(Request<Body>, Arc<S>) -> PinFut<Result<Response<Body>, Error>>
            + Send
            + Sync
            + 'static,
    {
        Self { f: Arc::new(f) }
    }
    fn execute(
        &self,
        req: Request<Body>,
        app_state: Arc<S>,
    ) -> Pin<Box<dyn Future<Output = Result<Response<Body>, Error>> + Send + Sync>> {
        (self.f)(req, app_state)
    }
}

// struct ts<S>where S:Send{
//     s:PhantomData<S>
// }

// impl ts<Handler<'static,()>>{}

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

pub struct Builder<'a, S>
where
    S: Sync + Send,
{
    routes: IndexMap<Route<'static>, Handler<S>>,
    ex: SmolExecutor<'a>,
    state: Arc<S>,
}

impl<'a, S> Builder<'a, S>
where
    S: Sync + Send,
{
    pub fn new(app_state: S) -> Self {
        Self {
            routes: Default::default(),
            ex: SmolExecutor::new(),
            state: Arc::new(app_state),
        }
    }
    pub fn add_route(&mut self, route: Route<'static>, handler: Handler<S>) {
        assert!(self.routes.insert(route, handler).is_none());
    }
    pub fn finish(self) -> Server<'a, S> {
        Server {
            data: Arc::new(self),
        }
    }
    // fn add_json_route<I, O, F>(&mut self, route: Route<'static>, f:F)
    // where
    //     O: Serialize,
    //     I: DeserializeOwned + Send + Sync,
    //     F: Fn(I, &S) -> PinFut<O> + Send + Sync,
    // {
    //     let handler = Handler::new(|request, app_state| {
    //         Box::pin(async move{
    //             let body = request.into_body();
    //             let bytes = hyper::body::to_bytes(body).await?;

    //             let input: I =
    //                 serde_json::from_slice(&bytes).map_err(|e| Error::DeserializeError(e))?;
    //             let raw_output = f(input, &app_state).await;
    //             let output =
    //                 serde_json::to_vec(&raw_output).map_err(|e| Error::SerializeError(e))?;

    //             let res = Response::builder()
    //                 .header("Content-Type", "application/json")
    //                 .body(Body::from(output));
    //             res.map_err(|e| e.into())
    //         })
    //     });
    //     self.add_route(route, handler);
    // }
}

pub struct Server<'a, S>
where
    S: Sync + Send,
{
    data: Arc<Builder<'a, S>>,
}

impl<'a, S> Clone for Server<'a, S>
where
    S: Sync + Send,
{
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

impl<'a, S> Server<'a, S>
where
    S: Sync + Send + 'a,
{
    fn get_handler(&self, request: &Request<Body>) -> Option<Handler<S>> {
        let route = Route::from_request(&request);
        self.data.routes.get(&route).map(|item| item.clone())
    }
    fn accpet(&self, request: Request<Body>) -> PinFut<Result<Response<Body>, Error>> {
        let default_handler = Handler::new(|_: Request<Body>, _: Arc<S>| {
            Box::pin(async { Ok(Response::new(Body::from("No such route!"))) })
        });
        let handler = self.get_handler(&request).unwrap_or(default_handler);

        handler.execute(request, self.data.state.clone())
    }
    pub fn listen_block(self, socket: SocketAddr) {
        let thread = usize::from(available_parallelism().unwrap_or(NonZeroUsize::new(1).unwrap()));

        self.data.ex.clone().detach_run(
            async move {
                let listener = Async::<TcpListener>::bind(socket).expect("error opening socket");
                loop {
                    let self_ = self.clone();
                    // try to accept tcp connection
                    let stream = match listener.accept().await {
                        Ok((stream, _)) => stream,
                        Err(_) => {
                            warn!("tcp connection hit the port but close before established");
                            continue;
                        }
                    };
                    // dispatch to executor
                    self.data.ex.clone().spawn_detach(async move {
                        let self_ = self_.clone();
                        let stream = SmolStream::from_plain(stream);
                        let http = Http::new();

                        let service = service_fn(move |request| self_.accpet(request));

                        if let Err(err) = http.serve_connection(stream, service).await {
                            warn!("Hyper throw an error at HTTP Parsing: {}", err);
                        };
                    })
                }
            },
            thread,
        );
    }
}

#[cfg(test)]
mod test {
    use std::{
        cell::RefCell,
        net::{IpAddr, Ipv4Addr},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    fn static_file() {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8001);

        let mut builder = Builder::new(());
        builder.add_route(
            Route::GET("/helloworld.html"),
            Handler::new(|_, _| {
                Box::pin(async {
                    let res = Response::builder()
                        .header("Content-Type", "text/html")
                        .body(Body::from(
                            include_bytes!("test/helloworld.html").as_slice(),
                        ));
                    res.map_err(|e| e.into())
                })
            }),
        );
        let server = builder.finish();
        server.listen_block(socket);
    }
    fn read_body() {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8001);
        let mut builder = Builder::new(());
        builder.add_route(
            Route::POST("/read_body"),
            Handler::new(|request, _| {
                Box::pin(async {
                    let body = request.into_body();
                    let bytes = hyper::body::to_bytes(body).await?;
                    let content = String::from_utf8_lossy(&bytes);

                    let res = Response::builder()
                        .header("Content-Type", "text/plain")
                        .body(Body::from(format!("your body: {}", content)));
                    res.map_err(|e| e.into())
                })
            }),
        );
        let server = builder.finish();
        server.listen_block(socket);
    }
    fn counter() {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8001);
        let mut builder = Builder::new(AtomicUsize::new(0));
        builder.add_route(
            Route::GET("/counter"),
            Handler::new(|request, counter: Arc<AtomicUsize>| {
                Box::pin(async move {
                    let res = Response::builder()
                        .header("Content-Type", "text/plain")
                        .body(Body::from(format!(
                            "counter: {}",
                            counter.fetch_add(1, Ordering::Release)
                        )));
                    res.map_err(|e| e.into())
                })
            }),
        );
        let server = builder.finish();
        server.listen_block(socket);
    }
}
