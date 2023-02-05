#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

pub (crate)mod web;
pub (crate)mod grub;
pub use crate::grub::api;
