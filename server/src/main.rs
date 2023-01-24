#[macro_use]
extern crate lazy_static;

pub mod auth;
mod test;
pub mod web;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, App, HttpServer};
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    SimpleLogger::new().init().unwrap();

    let api_service = web::route::api_entry().await;
    let secret_key = Key::generate();
    HttpServer::new(move || {
        App::new()
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .service(api_service)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
