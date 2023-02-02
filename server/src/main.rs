#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

#[cfg(test)]
mod test;
pub mod web;
use rand::Rng;
use simple_logger::SimpleLogger;
use web::prelude::*;

use crate::web::route;

#[async_std::main]
async fn main() {
    #[cfg(debug_assertions)]
    SimpleLogger::new().init().unwrap();
    #[cfg(not(debug_assertions))]
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let app_state = AppState::new().await;
    app_state.start_grub();

    let mut app = tide::new();

    app.with(tide::log::LogMiddleware::new());

    let cookie_secret: [u8; 32] = rand::thread_rng().gen();
    app.with(tide::sessions::SessionMiddleware::new(
        tide::sessions::MemoryStore::new(),
        &cookie_secret,
    ));

    app.at("/login").post(route::login);
    app.at("/api").nest({
        let mut api = tide::with_state(app_state);
        api.with(route::AuthMiddleware);
        api.at("/op/boot").post(route::boot);
        api.at("/get/machines").post(route::list_machine);
        api.at("/get/machine").post(route::info_machine);
        api.at("/get/oss").post(route::list_os);
        api.at("/op/new").post(route::new_machine);
        api.at("/auth")
            .get(|_| async { Ok("User is authenticated") });
        api
    });
    app.at("/").serve_dir("static").unwrap();

    app.listen("0.0.0.0:8000").await.unwrap();
}
