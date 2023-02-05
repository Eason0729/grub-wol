use crate::grub::adaptor::Convert;

use crate::grub::{self,api};
use super::state::AppState;
use async_trait::async_trait;
use bincode::config::{Bounded, WithOtherLimit};
use bincode::{DefaultOptions, Options};
use futures_lite::Future;
use serde::Deserialize;
use tide::{Middleware, Next, Request, Response};

lazy_static! {
    static ref BINCODE: WithOtherLimit<DefaultOptions, Bounded> = bincode::DefaultOptions::new().with_limit(4096);
    // TODO: replace PASSWORD with env after test
    static ref PASSWORD:&'static str="abc";
}

pub async fn boot(mut req: Request<AppState>) -> Result<Response, tide::Error> {
    BinaryResponder::parse(async move {
        let payload = req.body_bytes().await.map_err(|e| Error::Tide(e))?;
        let payload: api::BootReq = check_payload(payload)?;
        let state = req.state();
        state
            .grub
            .boot(payload.os, &payload.mac_address)
            .await
            .convert()
            .await
            .map_err(|err| Error::Internal(err))
    })
    .await
}

pub async fn list_machine(req: Request<AppState>) -> Result<Response, tide::Error> {
    BinaryResponder::parse(async move {
        let state = req.state();
        state
            .grub
            .list_machine()
            .convert()
            .await
            .map_err(|err| Error::Internal(err))
    })
    .await
}

pub async fn info_machine(mut req: Request<AppState>) -> Result<Response, tide::Error> {
    BinaryResponder::parse(async move {
        let payload = req.body_bytes().await.map_err(|e| Error::Tide(e))?;
        let payload: api::MachineInfoReq = check_payload(payload)?;
        let state = req.state();
        state
            .grub
            .info_machine(&payload.mac_address)
            .await
            .convert()
            .await
            .map_err(|err| Error::Internal(err))
    })
    .await
}

pub async fn list_os(mut req: Request<AppState>) -> Result<Response, tide::Error> {
    BinaryResponder::parse(async move {
        let payload = req.body_bytes().await.map_err(|e| Error::Tide(e))?;
        let payload: api::OsListReq = check_payload(payload)?;
        let state = req.state();
        state
            .grub
            .list_os(&payload.mac_address)
            .await
            .convert()
            .await
            .map_err(|err| Error::Internal(err))
    })
    .await
}

pub async fn new_machine(mut req: Request<AppState>) -> Result<Response, tide::Error> {
    BinaryResponder::parse(async move {
        let payload = req.body_bytes().await.map_err(|e| Error::Tide(e))?;
        let payload: api::NewMachineReq = check_payload(payload)?;
        let state = req.state();
        state
            .grub
            .init_machine(*payload.mac_address, payload.display_name.to_string())
            .await
            .convert()
            .await
            .map_err(|err| Error::Internal(err))
    })
    .await
}

pub async fn login(mut req: Request<()>) -> Result<Response, tide::Error> {
    BinaryResponder::parse(async move {
        let payload = req.body_bytes().await.map_err(|e| Error::Tide(e))?;
        let payload: api::LoginReq = check_payload(payload)?;

        Ok(bincode::serialize(&if payload.password == *PASSWORD {
            req.session_mut()
                .insert("authed", true)
                .map_err(|_| Error::Tide(tide::Error::from_str(500, "Error inserting session")))?;
            api::LoginRes::Success
        } else {
            api::LoginRes::Fail
        })
        .unwrap())
    })
    .await
}

pub struct AuthMiddleware;

#[async_trait]
impl<State: Clone + Send + Sync + 'static> Middleware<State> for AuthMiddleware {
    async fn handle(&self, req: Request<State>, next: Next<'_, State>) -> tide::Result {
        let authed = req.session().get("authed").unwrap_or(false);
        if authed {
            Ok(next.run(req).await)
        } else {
            Err(tide::Error::from_str(403, "Forbidden"))
        }
    }
}

fn check_payload<T>(payload: Vec<u8>) -> Result<T, Error>
where
    T: for<'a> Deserialize<'a>,
{
    if payload.len() > 1024 {
        Err(Error::EntityTooLarge)
    } else {
        serde_json::from_slice(&payload).map_err(|err| Error::Deserialize(err))
    }
}

enum BinaryResponder {
    Ok(Vec<u8>),
    Err(Error),
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Deserialize Error")]
    Deserialize(serde_json::Error),
    #[error("Internal Error")]
    Internal(grub::Error),
    #[error("Tide Error")]
    Tide(tide::Error),
    #[error("Entity Too Large")]
    EntityTooLarge,
}

impl BinaryResponder {
    async fn parse(
        f: impl Future<Output = Result<Vec<u8>, Error>>,
    ) -> Result<Response, tide::Error> {
        let self_: BinaryResponder = f.await.into();
        Ok(self_.respond())
    }
    fn respond(self) -> Response {
        match self {
            BinaryResponder::Ok(x) => Response::builder(200)
                .body(x)
                .content_type("application/octet-stream")
                .build(),
            BinaryResponder::Err(err) => match err {
                Error::Deserialize(err) => {
                    log::warn!("Error deserializing data from client: {}", err);
                    Response::builder(400)
                        .body("See log for more infomation")
                        .build()
                }
                Error::Internal(err) => {
                    match err {
                        grub::Error::UndefinedClientBehavior => {
                            log::warn!("Client(host) behavior falsely")
                        }
                        _ => log::error!("unexpected error: {:?}", err),
                    };
                    Response::builder(500)
                        .body("See log for more infomation")
                        .build()
                }
                Error::EntityTooLarge => {
                    log::warn!("Client send a very large payload");
                    Response::builder(413)
                        .body("See log for more infomation")
                        .build()
                }
                Error::Tide(err) => {
                    log::error!("unexpected tide error: {:?}", err);
                    Response::builder(500)
                        .body("See log for more infomation")
                        .build()
                }
            },
        }
    }
}

impl From<Result<Vec<u8>, Error>> for BinaryResponder {
    fn from(result: Result<Vec<u8>, Error>) -> Self {
        match result {
            Ok(x) => BinaryResponder::Ok(x),
            Err(x) => BinaryResponder::Err(x),
        }
    }
}

// #[derive(Default)]
// struct RequestCounterMiddleware {
//     requests_counted: Arc<AtomicUsize>,
// }
