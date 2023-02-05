#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

pub(crate) mod grub;
pub(crate) mod web;
pub use crate::grub::api;
