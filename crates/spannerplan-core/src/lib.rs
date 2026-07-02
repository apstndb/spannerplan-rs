//! `no_std` rendering pipeline for Spanner query plans.
//!
//! Port of <https://github.com/apstndb/spannerplan>. See `DESIGN.md` in the
//! workspace root for the full specification this crate implements against.
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod asciitable;
pub mod model;
pub mod plantree;
pub mod queryplan;
pub mod reference;
pub mod scalarappendix;
pub mod stats;
pub mod textwidth;
pub mod treerender;

#[cfg(feature = "wire")]
pub mod wire;
