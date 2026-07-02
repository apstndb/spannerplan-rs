//! `std` convenience layer for `spannerplan-core`: input decoding (YAML/JSON)
//! that the `no_std` core intentionally does not own. See `DESIGN.md` §5.

#[cfg(feature = "yaml")]
pub mod extract;

#[cfg(not(feature = "yaml"))]
pub mod extract_json;

#[cfg(feature = "yaml")]
pub use extract::{extract_plan_nodes, ExtractError};

#[cfg(not(feature = "yaml"))]
pub use extract_json::{extract_plan_nodes, ExtractError};

pub use spannerplan_core as core;
