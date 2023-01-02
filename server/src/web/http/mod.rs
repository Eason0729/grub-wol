mod compat;
mod server;
mod test;

pub mod prelude {
    pub use super::server::{Builder, Error, Handler, Route, Server};
}
