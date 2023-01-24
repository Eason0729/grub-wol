use super::grub::adaptor::Convert;

use super::{grub, state::AppState};
use actix_session::SessionExt;
use actix_web::dev::{Service, ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::error::ErrorUnauthorized;
use actix_web::{body::BoxBody, http::header::ContentType, post, web, HttpResponse, Responder};
use actix_web::{Resource, Scope};
use futures_lite::Future;
use serde::Deserialize;
use website;

pub async fn api_entry() -> Scope<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let state=AppState::new().await;
    web::scope("/api")
        .wrap_fn(|req, srv| {
            let session = req.get_session();
            let auth = match session.get::<bool>("auth") {
                Ok(x) => match x {
                    Some(x) => x,
                    None => false,
                },
                Err(_) => false,
            };
            if auth {
                srv.call(req)
            } else {
                session.clear();
                Box::pin(async { Err(ErrorUnauthorized("unauthorized")) })
            }
        })
        .app_data(state)
        .service(boot)
        .service(list_machine)
        .service(list_os)
        .service(info_machine)
        .service(new_machine)
}

#[post("/op/boot")]
async fn boot(state: web::Data<AppState>, payload: web::Bytes) -> BinaryResponder {
    BinaryResponder::parse(async move {
        let payload: website::BootReq = check_payload(payload)?;
        state
            .grub
            .boot(payload.os, &payload.mac_address)
            .await
            .convert()
            .await
            .map_err(|err| Error::InternalError(err))
    })
    .await
}

#[post("/get/machines")]
async fn list_machine(state: web::Data<AppState>) -> BinaryResponder {
    BinaryResponder::parse(async move {
        state
            .grub
            .list_machine()
            .convert()
            .await
            .map_err(|err| Error::InternalError(err))
    })
    .await
}

#[post("/get/machine")]
async fn info_machine(state: web::Data<AppState>, payload: web::Bytes) -> BinaryResponder {
    BinaryResponder::parse(async move {
        let payload: website::MachineInfoReq = check_payload(payload)?;
        state
            .grub
            .info_machine(&payload.mac_address)
            .await
            .convert()
            .await
            .map_err(|err| Error::InternalError(err))
    })
    .await
}

#[post("/get/oss")]
async fn list_os(state: web::Data<AppState>, payload: web::Bytes) -> BinaryResponder {
    BinaryResponder::parse(async move {
        let payload: website::OsListReq = check_payload(payload)?;
        state
            .grub
            .list_os(&payload.mac_address)
            .await
            .convert()
            .await
            .map_err(|err| Error::InternalError(err))
    })
    .await
}

#[post("/op/new")]
async fn new_machine(state: web::Data<AppState>, payload: web::Bytes) -> BinaryResponder {
    BinaryResponder::parse(async move {
        let payload: website::NewMachineReq = check_payload(payload)?;
        state
            .grub
            .init_machine(*payload.mac_address, payload.display_name.to_string())
            .await
            .convert()
            .await
            .map_err(|err| Error::InternalError(err))
    })
    .await
}

fn check_payload<T>(payload: web::Bytes) -> Result<T, Error>
where
    T: for<'a> Deserialize<'a>,
{
    if payload.len() > 1024 {
        Err(Error::EntityTooLarge)
    } else {
        bincode::deserialize(&payload).map_err(|err| Error::DeserializeError(err))
    }
}

enum BinaryResponder {
    Ok(Vec<u8>),
    Err(Error),
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Deserialize Error")]
    DeserializeError(bincode::Error),
    #[error("Internal Error")]
    InternalError(grub::Error),
    #[error("Entity Too Large")]
    EntityTooLarge,
}

impl BinaryResponder {
    async fn parse(f: impl Future<Output = Result<Vec<u8>, Error>>) -> BinaryResponder {
        f.await.into()
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

impl Responder for BinaryResponder {
    type Body = BoxBody;

    fn respond_to(self, req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        match self {
            BinaryResponder::Ok(x) => HttpResponse::Ok()
                .content_type(ContentType::octet_stream())
                .body(x),
            BinaryResponder::Err(err) => match err {
                Error::DeserializeError(err) => {
                    log::warn!("Error deserializing data from client: {}", err);
                    HttpResponse::BadRequest().body("See log for more infomation")
                }
                Error::InternalError(err) => {
                    match err {
                        grub::Error::UndefinedClientBehavior => {
                            log::warn!("Client(host) behavior falsely")
                        }
                        _ => log::error!("unexpected error: {}", err),
                    };
                    HttpResponse::BadRequest().body("See log for more infomation")
                }
                Error::EntityTooLarge => {
                    log::warn!("Client send a very large payload");
                    HttpResponse::PayloadTooLarge().body("See log for more infomation")
                }
            },
        }
    }
}
