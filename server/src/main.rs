#[macro_use]
extern crate lazy_static;

mod test;
pub mod web;
use simple_logger::SimpleLogger;

fn main() {
    SimpleLogger::new().init().unwrap();
}
