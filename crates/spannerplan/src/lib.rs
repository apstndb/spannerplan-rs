//! `std` convenience layer for `spannerplan-core`: input decoding (YAML/JSON)
//! that the `no_std` core intentionally does not own. See `DESIGN.md` §5.

pub mod extract;

pub use spannerplan_core as core;
