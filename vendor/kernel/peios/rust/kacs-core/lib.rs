//! Cargo entry point for the same semantic source compiled by Kbuild.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

use peios_uapi;

#[path = "../../../security/pkm/kacs/kacs_core/mod.rs"]
mod kacs_core;

pub use kacs_core::*;
