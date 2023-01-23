use std::env;
use std::io;
use std::path::Path;
use std::sync::Arc;

use super::grub::prelude as grub;
use super::state::AuthMiddleware;
use super::state::State;
use rand::Rng;
use tide::http::mime;
use tide::prelude::*;
use tide::utils::Before;
use tide::Request;
use tide::Response;
use tide::StatusCode;

lazy_static! {
    /// This is an example for using doc comment attributes
    static ref SAVE_PATH: &'static Path = Path::new("./");
    static ref PASSWORD: String = env::var("password").unwrap();
}
macro_rules! log_err {
    ($e:expr) => {
        match $e {
            Ok(o)=>o,
            Err(err) {
                log::error!(err);
                Ok(Response::new(StatusCode::InternalServerError))
            }
        }
    };
}

async fn start() -> Result<(), Error> {
    let mut app = tide::new();

    app.with(tide::sessions::SessionMiddleware::new(
        tide::sessions::MemoryStore::new(),
        &rand::thread_rng().gen::<[u8; 32]>(),
    ));
    app.at("/api").nest({
        let mut api = tide::with_state(State::new().await);
        api.with(AuthMiddleware::new());
        // api.at("/op/up").post(|mut req: Request<State>| async move {
        //     let state = req.state().clone();

        //     let body = req.body_bytes().await?;
        //     let param: web::BootReq = bincode::deserialize_from(body.as_slice()).unwrap();

        //     let response = bincode::serialize(
        //         &(state.grub().boot(&param.mac_address, param.os).await).unwrap(),
        //     )
        //     .unwrap();

        //     let response = Response::builder(203)
        //         .body(response)
        //         .content_type(mime::ANY)
        //         .build();

        //     Ok(response)
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
