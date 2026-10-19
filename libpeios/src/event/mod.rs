//! `<peios/event.h>` — KMES events.
//!
//! KMES is Peios's sole event path: the kernel stamps each event with trusted
//! metadata and writes it into a per-CPU lock-free ring buffer. This module is
//! divided into emission ([`emit`]), consumption ([`consume`]), and emission
//! policy ([`policy`]). Payloads are MessagePack (`crate::msgpack`).

pub mod consume;
pub mod emit;
pub mod policy;
