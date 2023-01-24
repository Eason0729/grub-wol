pub mod grub;
pub mod route;
pub mod state;

pub mod prelude {
    use super::*;
    pub use grub::machine::Server;
    pub use state::AppState;
}
