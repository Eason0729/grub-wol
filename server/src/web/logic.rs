use super::grub::adaptor::Convert;
use super::grub::prelude as grub;
use super::state::AppState;
use std::env;
use std::path::Path;

lazy_static! {
    /// This is an example for using doc comment attributes
    static ref SAVE_PATH: &'static Path = Path::new("./");
    static ref PASSWORD: String = env::var("password").unwrap();
}

// async fn my_test(mut req: Request<State<'_>>)->tide::Result<Response>{
//     let body = req.body_bytes().await?;
//     let param: web::BootReq = bincode::deserialize_from(body.as_slice())
//         .map_err(|_| tide::Error::from_str(400, "Deserialization Error"))?;
//     let state=req.state().clone();

//     let response = state.grub
//         .boot(param.os, &param.mac_address)
//         .await
//         .convert()
//         .await;

//     match response {
//         Ok(x) => Ok(Response::builder(203)
//             .body(x)
//             .content_type(mime::ANY)
//             .build()),
//         Err(e) => Err(tide::Error::from_str(500, e)),
//     }
// }

// #[derive(thiserror::Error, Debug)]
// enum Error {
//     #[error("Server Failure")]
//     IoError(#[from] io::Error),
// }
