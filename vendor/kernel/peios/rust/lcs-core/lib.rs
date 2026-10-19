//! Cargo entry point for the same semantic source compiled by Kbuild.

#![cfg_attr(not(test), no_std)]

use peios_uapi;
use kacs_core;

#[path = "../../../security/pkm/lcs/lcs_core/mod.rs"]
mod lcs_core;

pub use lcs_core::*;
