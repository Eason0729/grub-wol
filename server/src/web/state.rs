use std::{env, path::Path, sync::Arc};

use super::grub::{adaptor::Convert, prelude as grub};

lazy_static! {
    static ref SAVE_PATH: &'static Path = Path::new("./");
    static ref PASSWORD: String = env::var("password").unwrap();
}

#[derive(Clone)]
pub struct AppState {
    pub grub: Arc<grub::Server>,
}

impl AppState {
    pub async fn new() -> AppState {
        let grub_server = grub::Server::load(&SAVE_PATH).await.unwrap();

        AppState {
            grub: Arc::new(grub_server),
        }
    }
    // pub async fn list_os(&'a self,mac_address:[u8;6]) -> Vec<u8>{
    //     self.grub.list_os(&'a mac_address).await.convert().await.unwrap()
    // }
}
