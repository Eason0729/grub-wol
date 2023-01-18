use std::env;
use std::io;
use std::path::Path;
use std::sync::Arc;

use super::grub::prelude as grub;
use super::state::AppState;
use super::state::AuthMiddleware;
use rand::Rng;
use tide::prelude::*;
use tide::utils::Before;
use tide::Request;

lazy_static! {
    /// This is an example for using doc comment attributes
    static ref SAVE_PATH: &'static Path = Path::new("./");
    static ref PASSWORD: String = env::var("password").unwrap();
}

async fn start() -> Result<(), Error> {
    let mut app = tide::with_state(Arc::new(AppState::new().await));

    // app.with(tide::log::LogMiddleware::new());

    app.with(tide::sessions::SessionMiddleware::new(
        tide::sessions::MemoryStore::new(),
        &rand::thread_rng().gen::<[u8; 32]>(),
    ));
    app.at("/api").nest({
        let mut api = tide::new();
        api.with(AuthMiddleware::new());
        // api.at("/GET/machines").post(|mut Request|{

        // });
        api
    });
    app.listen("127.0.0.1:8000").await?;
    Ok(())
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Server Failure")]
    IoError(#[from] io::Error),
}
