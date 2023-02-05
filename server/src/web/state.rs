use std::{env, path::Path, sync::Arc};

use async_std::task::spawn;

use crate::grub::{machine, prelude as grub};

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
    pub fn start_grub(&self) {
        spawn(machine::Server::start(self.grub.clone()));
    }
}
