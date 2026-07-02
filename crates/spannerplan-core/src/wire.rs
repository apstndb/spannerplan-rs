//! Protobuf wire decode into [`crate::model`] types. Generated prost messages
//! are an internal detail; callers only see the shared model.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use prost::Message;
use prost_types::{value::Kind, ListValue, Struct, Value};

use crate::model::{
    ChildLink, Kind as ModelKind, Metadata, MetadataValue, PlanNode, ShortRepresentation,
};

mod generated {
    include!(concat!(env!("OUT_DIR"), "/google.spanner.v1.rs"));
}

use generated::{
    plan_node, PlanNode as WirePlanNode, QueryPlan as WireQueryPlan, ResultSet, ResultSetStats,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    Decode(prost::DecodeError),
    UnknownInputFormat,
}

impl core::fmt::Display for WireError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WireError::Decode(e) => write!(f, "failed to decode protobuf wire input: {e}"),
            WireError::UnknownInputFormat => write!(f, "unknown input format"),
        }
    }
}

/// Extracts plan nodes from protobuf wire bytes, mirroring `extract.go`'s shape
/// detection: bare `QueryPlan`, `ResultSetStats.query_plan`, or
/// `ResultSet.stats.query_plan`.
///
/// Detection tries each message shape in order. Protobuf decoding is lenient, so
/// the same bytes can often decode as multiple shapes; the `plan_nodes` non-empty
/// check disambiguates in practice because Spanner responses always include nodes.
pub fn decode_plan_nodes(input: &[u8]) -> Result<Vec<PlanNode>, WireError> {
    if let Ok(plan) = WireQueryPlan::decode(input) {
        if !plan.plan_nodes.is_empty() {
            return Ok(plan.plan_nodes.into_iter().map(PlanNode::from).collect());
        }
    }

    if let Ok(stats) = ResultSetStats::decode(input) {
        if let Some(query_plan) = stats.query_plan {
            if !query_plan.plan_nodes.is_empty() {
                return Ok(query_plan
                    .plan_nodes
                    .into_iter()
                    .map(PlanNode::from)
                    .collect());
            }
        }
    }

    if let Ok(result_set) = ResultSet::decode(input) {
        if let Some(stats) = result_set.stats {
            if let Some(query_plan) = stats.query_plan {
                if !query_plan.plan_nodes.is_empty() {
                    return Ok(query_plan
                        .plan_nodes
                        .into_iter()
                        .map(PlanNode::from)
                        .collect());
                }
            }
        }
    }

    Err(WireError::UnknownInputFormat)
}

impl From<WirePlanNode> for PlanNode {
    fn from(node: WirePlanNode) -> Self {
        PlanNode {
            index: node.index,
            kind: wire_kind_to_model(node.kind),
            display_name: node.display_name,
            child_links: node.child_links.into_iter().map(ChildLink::from).collect(),
            short_representation: node.short_representation.map(ShortRepresentation::from),
            metadata: struct_to_metadata(node.metadata),
            execution_stats: node.execution_stats.and_then(|s| {
                let metadata = struct_to_metadata(Some(s));
                (!metadata.is_empty()).then_some(metadata)
            }),
        }
    }
}

impl From<plan_node::ChildLink> for ChildLink {
    fn from(link: plan_node::ChildLink) -> Self {
        ChildLink {
            child_index: link.child_index,
            r#type: link.r#type,
            variable: link.variable,
        }
    }
}

impl From<plan_node::ShortRepresentation> for ShortRepresentation {
    fn from(sr: plan_node::ShortRepresentation) -> Self {
        ShortRepresentation {
            description: sr.description,
        }
    }
}

fn wire_kind_to_model(kind: i32) -> ModelKind {
    match plan_node::Kind::try_from(kind) {
        Ok(plan_node::Kind::Relational) => ModelKind::Relational,
        Ok(plan_node::Kind::Scalar) => ModelKind::Scalar,
        _ => ModelKind::Unspecified,
    }
}

fn struct_to_metadata(s: Option<Struct>) -> Metadata {
    s.map(|st| {
        st.fields
            .into_iter()
            .map(|(k, v)| (k, value_to_metadata(v)))
            .collect()
    })
    .unwrap_or_default()
}

fn value_to_metadata(v: Value) -> MetadataValue {
    match v.kind {
        Some(Kind::NullValue(_)) => MetadataValue::Null,
        Some(Kind::BoolValue(b)) => MetadataValue::Bool(b),
        Some(Kind::NumberValue(n)) => MetadataValue::Number(n),
        Some(Kind::StringValue(s)) => MetadataValue::String(s),
        Some(Kind::StructValue(st)) => MetadataValue::Struct(
            st.fields
                .into_iter()
                .map(|(k, v)| (k, value_to_metadata(v)))
                .collect(),
        ),
        Some(Kind::ListValue(ListValue { values })) => {
            MetadataValue::List(values.into_iter().map(value_to_metadata).collect())
        }
        None => MetadataValue::Null,
    }
}

/// Encodes model plan nodes as a wire `QueryPlan`. Hidden helper for parity tests.
#[doc(hidden)]
pub fn encode_query_plan_for_test(nodes: &[PlanNode]) -> WireQueryPlan {
    WireQueryPlan {
        plan_nodes: nodes.iter().cloned().map(plan_node_to_wire).collect(),
    }
}

/// Encodes model plan nodes as `ResultSetStats{query_plan}`. Test helper.
#[doc(hidden)]
pub fn encode_result_set_stats_for_test(nodes: &[PlanNode]) -> ResultSetStats {
    ResultSetStats {
        query_plan: Some(encode_query_plan_for_test(nodes)),
        query_stats: None,
    }
}

/// Encodes model plan nodes as `ResultSet{stats{query_plan}}`. Test helper.
#[doc(hidden)]
pub fn encode_result_set_for_test(nodes: &[PlanNode]) -> ResultSet {
    ResultSet {
        stats: Some(encode_result_set_stats_for_test(nodes)),
    }
}

fn plan_node_to_wire(node: PlanNode) -> WirePlanNode {
    WirePlanNode {
        index: node.index,
        kind: match node.kind {
            ModelKind::Relational => plan_node::Kind::Relational as i32,
            ModelKind::Scalar => plan_node::Kind::Scalar as i32,
            ModelKind::Unspecified => plan_node::Kind::Unspecified as i32,
        },
        display_name: node.display_name,
        child_links: node
            .child_links
            .into_iter()
            .map(|cl| plan_node::ChildLink {
                child_index: cl.child_index,
                r#type: cl.r#type,
                variable: cl.variable,
            })
            .collect(),
        short_representation: node
            .short_representation
            .map(|sr| plan_node::ShortRepresentation {
                description: sr.description,
                subqueries: BTreeMap::new(),
            }),
        metadata: metadata_to_wire_struct(node.metadata),
        execution_stats: node.execution_stats.and_then(metadata_to_wire_struct),
    }
}

fn metadata_to_wire_struct(metadata: Metadata) -> Option<Struct> {
    if metadata.is_empty() {
        return None;
    }
    Some(Struct {
        fields: metadata
            .into_iter()
            .map(|(k, v)| (k, metadata_value_to_wire(v)))
            .collect(),
    })
}

fn metadata_value_to_wire(v: MetadataValue) -> Value {
    use prost_types::NullValue;

    let kind = match v {
        MetadataValue::Null => Kind::NullValue(NullValue::NullValue as i32),
        MetadataValue::Bool(b) => Kind::BoolValue(b),
        MetadataValue::Number(n) => Kind::NumberValue(n),
        MetadataValue::String(s) => Kind::StringValue(s),
        MetadataValue::Struct(fields) => Kind::StructValue(Struct {
            fields: fields
                .into_iter()
                .map(|(k, v)| (k, metadata_value_to_wire(v)))
                .collect(),
        }),
        MetadataValue::List(values) => Kind::ListValue(ListValue {
            values: values.into_iter().map(metadata_value_to_wire).collect(),
        }),
    };
    Value { kind: Some(kind) }
}

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeMap;

    use super::*;
    use prost::Message;

    #[test]
    fn round_trip_plan_node_fields() {
        let original = PlanNode {
            index: 2,
            kind: ModelKind::Relational,
            display_name: "Scan".into(),
            child_links: vec![ChildLink {
                child_index: 1,
                r#type: "Input".into(),
                variable: "v".into(),
            }],
            short_representation: Some(ShortRepresentation {
                description: "x = 1".into(),
            }),
            metadata: BTreeMap::from([(
                "scan_type".into(),
                MetadataValue::String("TableScan".into()),
            )]),
            execution_stats: Some(BTreeMap::from([(
                "latency".into(),
                MetadataValue::String("1 msec".into()),
            )])),
        };

        let wire = encode_query_plan_for_test(std::slice::from_ref(&original)).encode_to_vec();
        let decoded = decode_plan_nodes(&wire).unwrap();
        assert_eq!(decoded, vec![original]);
    }

    #[test]
    fn decode_result_set_stats_shape() {
        let original = PlanNode {
            index: 0,
            kind: ModelKind::Relational,
            display_name: "Root".into(),
            child_links: vec![],
            short_representation: None,
            metadata: BTreeMap::new(),
            execution_stats: None,
        };

        let wire =
            encode_result_set_stats_for_test(std::slice::from_ref(&original)).encode_to_vec();
        let decoded = decode_plan_nodes(&wire).unwrap();
        assert_eq!(decoded, vec![original]);
    }

    #[test]
    fn decode_result_set_shape() {
        let original = PlanNode {
            index: 0,
            kind: ModelKind::Relational,
            display_name: "Root".into(),
            child_links: vec![],
            short_representation: None,
            metadata: BTreeMap::new(),
            execution_stats: None,
        };

        let wire = encode_result_set_for_test(std::slice::from_ref(&original)).encode_to_vec();
        let decoded = decode_plan_nodes(&wire).unwrap();
        assert_eq!(decoded, vec![original]);
    }
}
