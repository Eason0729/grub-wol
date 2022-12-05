#[macro_use]
extern crate lazy_static;

pub mod common;
pub mod constant;
mod encode;

pub mod conn {
    use crate::{common, encode};

    pub type Conn = encode::TcpConn<common::Packet>;
}

pub mod prelude {
    pub use super::common::*;
    pub use super::conn::Conn;
    pub use super::constant::*;
    pub use super::encode::Error;
}
