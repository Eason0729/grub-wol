use std::io;
use std::net::TcpListener;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use smol::Async;

use super::compat::{SmolExampleExecutor, SmolListener};

/// Serves a request and returns a response.
async fn serve(req: Request<Body>) -> Result<Response<Body>, Error> {
    // req.uri().path()
    Ok(Response::new(Body::from("Hello from hyper!")))
}

/// Listens for incoming connections and serves them.
async fn listen(listener: Async<TcpListener>) -> Result<(), Error> {
    println!("Listening on http://{}", listener.get_ref().local_addr()?);

    Server::builder(SmolListener::new(&listener))
        .executor(SmolExampleExecutor)
        .serve(make_service_fn(move |_| async {
            Ok::<_, Error>(service_fn(move |req| serve(req)))
        }))
        .await?;

    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error")]
    IoError(#[from] io::Error),
    #[error("hyper error")]
    HyperError(#[from] hyper::Error),
}

#[cfg(test)]
mod test {
    use super::*;
    // #[test]
    fn original_main() {
        smol::block_on(async {
            let http = listen(Async::<TcpListener>::bind(([127, 0, 0, 1], 8000)).unwrap());
            http.await.unwrap();
        });
    }
}
