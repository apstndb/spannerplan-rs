//! JSON-only plan extraction (no `serde_yaml_ng`). Used when the `yaml` feature
//! is disabled — e.g. slim WASM builds that expect pre-parsed JSON from the host.

use spannerplan_core::model::{PlanNode, QueryPlanMessage};

#[derive(Debug)]
pub enum ExtractError {
    Parse(serde_json::Error),
    UnknownFormat,
}

impl core::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ExtractError::Parse(e) => write!(f, "failed to parse input as JSON: {e}"),
            ExtractError::UnknownFormat => write!(f, "unknown input format"),
        }
    }
}

impl std::error::Error for ExtractError {}

impl From<serde_json::Error> for ExtractError {
    fn from(e: serde_json::Error) -> Self {
        ExtractError::Parse(e)
    }
}

/// Same shape detection as [`super::extract_plan_nodes`] but via `serde_json` only.
pub fn extract_plan_nodes(input: &[u8]) -> Result<Vec<PlanNode>, ExtractError> {
    let value: serde_json::Value = serde_json::from_slice(input)?;

    if let Some(query_plan) = get_any(&value, &["queryPlan", "query_plan"]) {
        let msg: QueryPlanMessage = serde_json::from_value(query_plan.clone())?;
        return Ok(msg.plan_nodes);
    }
    if let Some(plan_nodes) = get_any(&value, &["planNodes", "plan_nodes"]) {
        let nodes: Vec<PlanNode> = serde_json::from_value(plan_nodes.clone())?;
        return Ok(nodes);
    }
    if let Some(stats) = value.get("stats") {
        if let Some(query_plan) = get_any(stats, &["queryPlan", "query_plan"]) {
            let msg: QueryPlanMessage = serde_json::from_value(query_plan.clone())?;
            return Ok(msg.plan_nodes);
        }
    }
    Err(ExtractError::UnknownFormat)
}

fn get_any<'a>(value: &'a serde_json::Value, keys: &[&str]) -> Option<&'a serde_json::Value> {
    keys.iter().find_map(|k| value.get(k))
}
