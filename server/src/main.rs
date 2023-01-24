#[macro_use]
extern crate lazy_static;

pub mod auth;
mod test;
pub mod web;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, App, HttpServer, rt::spawn};
use simple_logger::SimpleLogger;
use web::{state::AppState, route::api_entry, grub::machine};


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    SimpleLogger::new().init().unwrap();

    let state=AppState::new().await;
    spawn(machine::Server::start(state.grub.clone()));

    let secret_key = Key::generate();

    HttpServer::new(move ||App::new().app_data(state.clone())
    .wrap(SessionMiddleware::new(
        CookieSessionStore::default(),
        secret_key.clone(),
    ))
    .configure(api_entry))
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}
