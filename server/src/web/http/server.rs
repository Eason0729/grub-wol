use std::{future::Future, io, pin::Pin};

use hyper::{Body, Request, Response, Server, Method};
use indexmap::IndexMap;

struct WebServer<'a,F>
where
    F: Fn(&Request<Body>) -> Pin<Box<dyn Future<Output = Result<Response<Body>, Error>>>>,
{
    route: IndexMap<Route<'a>, F>,
}

impl<'a, F> WebServer<'a, F>
where
    F: Fn(&Request<Body>) -> Pin<Box<dyn Future<Output = Result<Response<Body>, Error>>>>,
{
    fn new()->Self{
        Self{route:Default::default()}
    }
    fn serve(self){

        todo!()
    }
    async fn accept(&self,request:Request<Body>)->Result<Response<Body>, Error>{
        let route=Route::from_request(&request);

        if let Some(f)=self.route.get(&route){
            f(&request).await
        }else{
            Ok(Response::new(Body::from("Couldn't find service matching uri")))
        }
    }
    fn add_route(&mut self,route:Route<'a>,f:F){
        self.route.insert(route, f).unwrap();
    }
}

#[derive(Hash,PartialEq,Eq)]
enum Route<'a> {
    GET(&'a str),
    POST(&'a str),
    OTHER(&'a str)
}

impl<'a> Route<'a> {
    fn from_request(request:&'a Request<Body>)->Self{
        if request.method()==&Method::GET{
            Route::GET(request.uri().path())
        }else if request.method()==&Method::POST{
            Route::POST(request.uri().path())
        }else{
            Route::OTHER(request.uri().path())
        }
    }
}

macro_rules! StaticRoute {
    ($server:expr ,$route:expr ,$file:expr ) => {
        $server.add_route(Route::GET($route),|_|{
            Box::pin(Ok(Response::new(Body::from(include_bytes!($file)))))
        })
    };
}

#[derive(thiserror::Error, Debug)]
pub enum Error { 
    #[error("io error")]
    IoError(#[from] io::Error),
    #[error("hyper error")]
    HyperError(#[from] hyper::Error),
}

#[cfg(test)]
mod test{
    use super::*;

    #[test]
    fn stiaic(){
        let mut server=WebServer::new();
        StaticRoute!(server,"/hellowrold.html","test/helloworld.html");
    }
}