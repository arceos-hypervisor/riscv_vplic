#![cfg_attr(not(test), no_std)]

mod consts;
mod utils;
mod vplic;
mod devops_impl;

pub use consts::*;
pub use vplic::VPlicGlobal;