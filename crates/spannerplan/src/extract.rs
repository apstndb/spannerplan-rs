//! Ports `extract.go`'s `ExtractQueryPlan`: detect which of the three
//! top-level input shapes (`queryPlan`, `planNodes`, or `stats`) a YAML/JSON
//! document uses, and pull out the plan-node list.
//!
//! Reduced scope for now: returns `Vec<PlanNode>` rather than the full
//! `ResultSetStats` + row-type pair Go's `ExtractQueryPlan` returns, since
//! nothing downstream needs the rest yet. Widen this once `stats.rs` /
//! `reference.rs` need `ResultSetStats`/`StructType` typed data.
//!
//! Input is parsed as YAML throughout (not JSON-then-YAML-fallback): JSON is
//! a syntactic subset of YAML, so this mirrors `protoyaml.YAMLToJSON`, which
//! always goes through the YAML parser regardless of whether the input is
//! "really" JSON.

use spannerplan_core::model::{PlanNode, QueryPlanMessage};

#[derive(Debug)]
pub enum ExtractError {
    Parse(serde_yaml_ng::Error),
    UnknownFormat,
}

impl core::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ExtractError::Parse(e) => write!(f, "failed to parse input as YAML/JSON: {e}"),
            ExtractError::UnknownFormat => write!(f, "unknown input format"),
        }
    }
}

impl std::error::Error for ExtractError {}

impl From<serde_yaml_ng::Error> for ExtractError {
    fn from(e: serde_yaml_ng::Error) -> Self {
        ExtractError::Parse(e)
    }
}

/// Extracts the plan-node list from a YAML or JSON document, dispatching on
/// the top-level shape exactly like `extract.go`'s `ExtractQueryPlan`:
/// `queryPlan` (a `ResultSetStats`), `planNodes` (a bare `QueryPlan`), or
/// `stats` (a `ResultSet`, whose `.stats.queryPlan` is used).
pub fn extract_plan_nodes(input: &[u8]) -> Result<Vec<PlanNode>, ExtractError> {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_slice(input)?;

    if let Some(query_plan) = get_any(&value, &["queryPlan", "query_plan"]) {
        let msg: QueryPlanMessage = serde_yaml_ng::from_value(query_plan.clone())?;
        return Ok(msg.plan_nodes);
    }
    if let Some(plan_nodes) = get_any(&value, &["planNodes", "plan_nodes"]) {
        let nodes: Vec<PlanNode> = serde_yaml_ng::from_value(plan_nodes.clone())?;
        return Ok(nodes);
    }
    if let Some(stats) = value.get("stats") {
        if let Some(query_plan) = get_any(stats, &["queryPlan", "query_plan"]) {
            let msg: QueryPlanMessage = serde_yaml_ng::from_value(query_plan.clone())?;
            return Ok(msg.plan_nodes);
        }
    }
    Err(ExtractError::UnknownFormat)
}

fn get_any<'a>(value: &'a serde_yaml_ng::Value, keys: &[&str]) -> Option<&'a serde_yaml_ng::Value> {
    keys.iter().find_map(|k| value.get(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "yaml")]
    fn extracts_plan_nodes_from_result_set_shaped_fixture() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testdata/reference/dca.yaml"
        );
        let bytes = std::fs::read(path).unwrap();
        let nodes = extract_plan_nodes(&bytes).unwrap();
        assert!(!nodes.is_empty(), "expected at least one plan node");
        // PlanNode.Index is documented to match its slice position (see
        // DESIGN.md §6.1 / queryplan.go New()); spot-check that here too.
        assert_eq!(nodes[0].get_index(), 0);
    }

    #[test]
    #[cfg(feature = "yaml")]
    fn extracts_plan_nodes_from_bare_plan_nodes_shape() {
        let input = br#"{"planNodes": [{"index": 0, "displayName": "Scan"}]}"#;
        let nodes = extract_plan_nodes(input).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].get_display_name(), "Scan");
    }

    #[test]
    #[cfg(feature = "yaml")]
    fn extracts_plan_nodes_from_query_plan_shape() {
        let input = br#"{"queryPlan": {"planNodes": [{"index": 0}]}}"#;
        let nodes = extract_plan_nodes(input).unwrap();
        assert_eq!(nodes.len(), 1);
    }

    #[test]
    #[cfg(feature = "yaml")]
    fn errors_on_unrecognized_shape() {
        let input = br#"{"somethingElse": true}"#;
        let err = extract_plan_nodes(input).unwrap_err();
        assert!(matches!(err, ExtractError::UnknownFormat));
    }
}
