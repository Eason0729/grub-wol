use std::{env, path::Path};

use serde::{Deserialize, Serialize};
use tide::{Middleware, Request, Response, StatusCode};

use super::grub::prelude as grub;

lazy_static! {
    /// This is an example for using doc comment attributes
    static ref SAVE_PATH: &'static Path = Path::new("./");
    static ref PASSWORD: String = env::var("password").unwrap();
}

pub struct AppState<'a> {
    grub: grub::Server<'a>,
}

impl<'a> AppState<'a> {
    pub async fn new() -> AppState<'a> {
        let grub_server = grub::Server::load(&SAVE_PATH).await.unwrap();

        AppState { grub: grub_server }
    }
}

pub struct AuthMiddleware {}

impl AuthMiddleware {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Deserialize, Serialize)]
enum UserStatus {
    Login,
    Guest,
}

#[tide::utils::async_trait]
impl<AppState: Clone + Send + Sync + 'static> Middleware<AppState> for AuthMiddleware {
    async fn handle(
        &self,
        mut req: Request<AppState>,
        next: tide::Next<'_, AppState>,
    ) -> tide::Result {
        let is_login: UserStatus = req.session().get("UserStatus").unwrap_or(UserStatus::Guest);
        match is_login {
            UserStatus::Login => Ok(next.run(req).await),
            UserStatus::Guest => {
                req.session_mut()
                    .insert("UserStatus", UserStatus::Guest)
                    .unwrap();
                Ok(Response::new(StatusCode::Unauthorized))
            }
        }
    }
}