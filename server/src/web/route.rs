use super::grub::adaptor::Convert;

use super::{grub, state::AppState};
use actix_web::{
    body::{BoxBody, MessageBody},
    http::header::ContentType,
    post, web, HttpResponse, Responder,
};
use futures_lite::Future;
use serde::Deserialize;
use website;

// #[post("/get/oss")]
// async fn index(state: web::Data<AppState<'_>>, payload: web::Bytes) -> BinaryResponder {
//     BinaryResponder::parse(async move {
//         let payload: website::OsListReq = check_payload(payload)?;
//         state
//             .grub
//             .list_os(&payload.mac_address)
//             .await
//             .convert()
//             .await
//             .map_err(|err| Error::InternalError(err))
//     })
//     .await
// }

#[post("/get/oss")]
async fn index<'a>(state: web::Data<AppState<'a>>, payload: web::Bytes) -> BinaryResponder {
    let payload: website::OsListReq = check_payload(payload).unwrap();
    let handler = state.grub.list_os(&payload.mac_address).await;
    let result = handler.convert().await;
    result.map_err(|err| Error::InternalError(err)).into()
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
