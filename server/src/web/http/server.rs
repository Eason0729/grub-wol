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

struct Handler<S> {
    f: Arc<dyn Fn(Request<Body>, &S) -> PinFut<Result<Response<Body>, Error>> + Send + Sync>,
}

impl<S> Clone for Handler<S> {
    fn clone(&self) -> Self {
        Self { f: self.f.clone() }
    }
}

impl<S> Handler<S> {
    fn new<F>(f: F) -> Self
    where
        F: Fn(Request<Body>, &S) -> PinFut<Result<Response<Body>, Error>> + Send + Sync + 'static,
    {
        Self { f: Arc::new(f) }
    }
    fn execute(
        &self,
        req: Request<Body>,
        app_state: &S,
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

struct ServerData<'a, S> {
    routes: IndexMap<Route<'static>, Handler<S>>,
    ex: SmolExecutor<'a>,
    state: S,
}

struct Server<'a, S>
where
    S: Sync+Send,
{
    data: Arc<ServerData<'a, S>>,
}

impl<'a, S> Clone for Server<'a, S>
where
    S: Sync+Send,
{
    fn clone(&self) -> Self {
        Self { data: self.data.clone() }
    }
}

impl<'a, S> Server<'a, S>
where
    S: Sync+Send,
{
    fn get_handler(&self, request: &Request<Body>) -> Option<Handler<S>> {
        let route = Route::from_request(&request);
        self.data.routes.get(&route).map(|item| item.clone())
    }
    async fn accpet(&self, request: Request<Body>) -> Result<Response<Body>, Error> {
        let default_handler = Handler::new(|_: Request<Body>, _: &S| {
            Box::pin(async { Ok(Response::new(Body::from("No such route!"))) })
        });
        let handler = self.get_handler(&request).unwrap_or(default_handler);

        handler.execute(request, &self.data.state).await
    }
    fn listen_block(&self, socket: SocketAddr) {
        let thread = usize::from(available_parallelism().unwrap_or(NonZeroUsize::new(1).unwrap()));

        self.data.ex.clone().detach_run(
            async move {
                let listener = Async::<TcpListener>::bind(socket).expect("error opening socket");
                loop {
                    let self_=self.clone();
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
                        let self_=self_.clone();
                        let stream = SmolStream::from_plain(stream);
                        let http = Http::new();

                        let service=service_fn(move |request| self_.accpet(request));

                        if let Err(err) = http
                            .serve_connection(
                                stream,
                                service,
                            )
                            .await
                        {
                            warn!("Hyper throw an error at HTTP Parsing: {}", err);
                        };
                    })
                }
            },
            thread,
        );
    }
}
