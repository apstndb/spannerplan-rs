//! Protobuf wire decode into [`crate::model`] types. Generated prost messages
//! are an internal detail; callers only see the shared model.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
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
    /// A `google.protobuf.Struct`, `Value`, or `ListValue` carried a state
    /// that prost would otherwise discard or normalize before the structural
    /// signature can reject it.
    InvalidWellKnownType(&'static str),
}

impl core::fmt::Display for WireError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WireError::Decode(e) => write!(f, "failed to decode protobuf wire input: {e}"),
            WireError::UnknownInputFormat => write!(f, "unknown input format"),
            WireError::InvalidWellKnownType(reason) => {
                write!(f, "invalid protobuf well-known metadata value: {reason}")
            }
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
            guard_query_plan(input)?;
            return Ok(plan.plan_nodes.into_iter().map(PlanNode::from).collect());
        }
    }

    if let Ok(stats) = ResultSetStats::decode(input) {
        if let Some(query_plan) = stats.query_plan {
            if !query_plan.plan_nodes.is_empty() {
                guard_result_set_stats(input)?;
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
                    guard_result_set(input)?;
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

// Prost deliberately discards unknown protobuf fields. That is normally the
// right forward-compatibility choice, but the structural-signature contract
// must fail closed for Struct/Value/ListValue: otherwise a Go signature can
// reject a capture that Rust silently canonicalizes. These bounded walkers run
// only on the detected envelope and only inspect signature-included PlanNode
// metadata. The generated types remain the source of truth for all conversion.
#[derive(Clone, Copy)]
struct RawField<'a> {
    number: u32,
    wire_type: u8,
    payload: &'a [u8],
}

fn next_raw_field<'a>(input: &mut &'a [u8]) -> Result<Option<RawField<'a>>, WireError> {
    if input.is_empty() {
        return Ok(None);
    }
    let key = read_raw_varint(input)?;
    let number = (key >> 3) as u32;
    let wire_type = (key & 0x07) as u8;
    if number == 0 {
        return Err(WireError::InvalidWellKnownType("field number zero"));
    }
    let payload = match wire_type {
        0 => {
            let start = *input;
            read_raw_varint(input)?;
            &start[..start.len() - input.len()]
        }
        1 => take_raw_bytes(input, 8)?,
        2 => {
            let len = read_raw_varint(input)?;
            let len = usize::try_from(len)
                .map_err(|_| WireError::InvalidWellKnownType("length overflows usize"))?;
            take_raw_bytes(input, len)?
        }
        5 => take_raw_bytes(input, 4)?,
        _ => {
            return Err(WireError::InvalidWellKnownType(
                "unsupported protobuf wire type",
            ))
        }
    };
    Ok(Some(RawField {
        number,
        wire_type,
        payload,
    }))
}

fn read_raw_varint(input: &mut &[u8]) -> Result<u64, WireError> {
    let mut result = 0u64;
    for shift in (0..64).step_by(7) {
        let Some((&byte, rest)) = input.split_first() else {
            return Err(WireError::InvalidWellKnownType("truncated protobuf varint"));
        };
        *input = rest;
        if shift == 63 && byte > 1 {
            return Err(WireError::InvalidWellKnownType(
                "protobuf varint overflows u64",
            ));
        }
        result |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
    }
    Err(WireError::InvalidWellKnownType(
        "protobuf varint overflows u64",
    ))
}

fn take_raw_bytes<'a>(input: &mut &'a [u8], len: usize) -> Result<&'a [u8], WireError> {
    if input.len() < len {
        return Err(WireError::InvalidWellKnownType("truncated protobuf field"));
    }
    let (head, tail) = input.split_at(len);
    *input = tail;
    Ok(head)
}

fn guard_query_plan(mut input: &[u8]) -> Result<(), WireError> {
    while let Some(field) = next_raw_field(&mut input)? {
        if field.number == 1 && field.wire_type == 2 {
            guard_plan_node(field.payload)?;
        }
    }
    Ok(())
}

fn guard_result_set_stats(mut input: &[u8]) -> Result<(), WireError> {
    while let Some(field) = next_raw_field(&mut input)? {
        if field.number == 1 && field.wire_type == 2 {
            guard_query_plan(field.payload)?;
        }
    }
    Ok(())
}

fn guard_result_set(mut input: &[u8]) -> Result<(), WireError> {
    while let Some(field) = next_raw_field(&mut input)? {
        if field.number == 3 && field.wire_type == 2 {
            guard_result_set_stats(field.payload)?;
        }
    }
    Ok(())
}

fn guard_plan_node(mut input: &[u8]) -> Result<(), WireError> {
    let mut metadata = StructState::default();
    let mut has_metadata = false;
    while let Some(field) = next_raw_field(&mut input)? {
        if field.number == 6 && field.wire_type == 2 {
            has_metadata = true;
            metadata.merge(field.payload)?;
        }
    }
    if has_metadata {
        metadata.validate()
    } else {
        Ok(())
    }
}

#[derive(Default)]
struct StructState {
    has_unknown: bool,
    fields: BTreeMap<String, ValueState>,
}

impl StructState {
    fn merge(&mut self, mut input: &[u8]) -> Result<(), WireError> {
        while let Some(field) = next_raw_field(&mut input)? {
            if field.number == 1 && field.wire_type == 2 {
                let (key, value) = parse_struct_entry(field.payload)?;
                // Protobuf maps use the last entry for a decoded key. Replacing
                // the whole value here is what lets an overwritten invalid
                // value disappear before signature validation, as it does in
                // Go's decoded structpb.Struct.
                self.fields.insert(key, value);
            } else {
                // Unknown fields are message state and survive message merges.
                self.has_unknown = true;
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), WireError> {
        if self.has_unknown {
            return Err(WireError::InvalidWellKnownType(
                "protobuf Struct contains unknown fields",
            ));
        }
        for (key, value) in &self.fields {
            // This key identifies a PlanNode ID and is excluded recursively by
            // the canonical signature. Its effective value is intentionally
            // not inspected, while unknown fields on the containing Struct
            // remain fatal above.
            if key != "subquery_cluster_node" {
                value.validate()?;
            }
        }
        Ok(())
    }
}

fn parse_struct_entry(mut input: &[u8]) -> Result<(String, ValueState), WireError> {
    let mut key = String::new();
    let mut value = ValueState::default();
    while let Some(field) = next_raw_field(&mut input)? {
        match (field.number, field.wire_type) {
            (1, 2) => {
                key = core::str::from_utf8(field.payload)
                    .map_err(|_| WireError::InvalidWellKnownType("invalid protobuf map key"))?
                    .to_string();
            }
            (2, 2) => value.merge(field.payload)?,
            // Synthetic map-entry messages do not expose retained unknown
            // fields through Struct. Go therefore ignores them too.
            _ => {}
        }
    }
    Ok((key, value))
}

#[derive(Default)]
struct ValueState {
    has_unknown: bool,
    kind: Option<ValueKindState>,
}

enum ValueKindState {
    Null(i32),
    Number,
    String,
    Bool,
    Struct(StructState),
    List(ListState),
}

impl ValueState {
    fn merge(&mut self, mut input: &[u8]) -> Result<(), WireError> {
        while let Some(field) = next_raw_field(&mut input)? {
            match (field.number, field.wire_type) {
                (1, 0) => {
                    let mut null_value = field.payload;
                    let value = read_raw_varint(&mut null_value)?;
                    if !null_value.is_empty() {
                        return Err(WireError::InvalidWellKnownType(
                            "invalid protobuf null Value",
                        ));
                    }
                    // Generated protobuf enum storage is i32. Match the
                    // decoder's truncation before validating NULL_VALUE (0),
                    // rather than imposing a stricter raw-u64 rule.
                    self.kind = Some(ValueKindState::Null(value as i32));
                }
                (2, 1) => self.kind = Some(ValueKindState::Number),
                (3, 2) => self.kind = Some(ValueKindState::String),
                (4, 0) => self.kind = Some(ValueKindState::Bool),
                (5, 2) => match &mut self.kind {
                    Some(ValueKindState::Struct(value)) => value.merge(field.payload)?,
                    _ => {
                        let mut value = StructState::default();
                        value.merge(field.payload)?;
                        self.kind = Some(ValueKindState::Struct(value));
                    }
                },
                (6, 2) => match &mut self.kind {
                    Some(ValueKindState::List(value)) => value.merge(field.payload)?,
                    _ => {
                        let mut value = ListState::default();
                        value.merge(field.payload)?;
                        self.kind = Some(ValueKindState::List(value));
                    }
                },
                _ => {
                    // Value-level unknown fields survive all oneof replacement
                    // and same-message merge operations.
                    self.has_unknown = true;
                }
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), WireError> {
        if self.has_unknown {
            return Err(WireError::InvalidWellKnownType(
                "protobuf Value contains unknown fields",
            ));
        }
        match self.kind.as_ref() {
            Some(ValueKindState::Null(0))
            | Some(ValueKindState::Number)
            | Some(ValueKindState::String)
            | Some(ValueKindState::Bool) => Ok(()),
            Some(ValueKindState::Null(_)) => Err(WireError::InvalidWellKnownType(
                "invalid protobuf null Value",
            )),
            Some(ValueKindState::Struct(value)) => value.validate(),
            Some(ValueKindState::List(value)) => value.validate(),
            None => Err(WireError::InvalidWellKnownType(
                "protobuf Value kind is unset",
            )),
        }
    }
}

#[derive(Default)]
struct ListState {
    has_unknown: bool,
    values: Vec<ValueState>,
}

impl ListState {
    fn merge(&mut self, mut input: &[u8]) -> Result<(), WireError> {
        while let Some(field) = next_raw_field(&mut input)? {
            if field.number == 1 && field.wire_type == 2 {
                let mut value = ValueState::default();
                value.merge(field.payload)?;
                // ListValue.values is repeated, so later message segments
                // append rather than replace existing values.
                self.values.push(value);
            } else {
                self.has_unknown = true;
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), WireError> {
        if self.has_unknown {
            return Err(WireError::InvalidWellKnownType(
                "protobuf ListValue contains unknown fields",
            ));
        }
        for value in &self.values {
            value.validate()?;
        }
        Ok(())
    }
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

    fn push_varint(out: &mut Vec<u8>, mut value: u64) {
        while value >= 0x80 {
            out.push(value as u8 | 0x80);
            value >>= 7;
        }
        out.push(value as u8);
    }

    fn push_length_delimited(out: &mut Vec<u8>, field: u8, payload: &[u8]) {
        out.push(field << 3 | 2);
        push_varint(out, payload.len() as u64);
        out.extend_from_slice(payload);
    }

    fn string_value(value: &[u8]) -> Vec<u8> {
        let mut encoded = Vec::new();
        push_length_delimited(&mut encoded, 3, value);
        encoded
    }

    fn struct_value(value: &[u8]) -> Vec<u8> {
        let mut encoded = Vec::new();
        push_length_delimited(&mut encoded, 5, value);
        encoded
    }

    fn list_value(value: &[u8]) -> Vec<u8> {
        let mut encoded = Vec::new();
        push_length_delimited(&mut encoded, 6, value);
        encoded
    }

    fn struct_entry(key: &[u8], value_segments: &[&[u8]], suffix: &[u8]) -> Vec<u8> {
        let mut entry = Vec::new();
        push_length_delimited(&mut entry, 1, key);
        for value in value_segments {
            push_length_delimited(&mut entry, 2, value);
        }
        entry.extend_from_slice(suffix);
        entry
    }

    fn metadata_struct(entries: &[Vec<u8>], suffix: &[u8]) -> Vec<u8> {
        let mut metadata = Vec::new();
        for entry in entries {
            push_length_delimited(&mut metadata, 1, entry);
        }
        metadata.extend_from_slice(suffix);
        metadata
    }

    fn query_plan_with_structs(metadata: &[Vec<u8>], execution_stats: &[Vec<u8>]) -> Vec<u8> {
        let mut node = vec![0x10, 0x01]; // index 0 and RELATIONAL kind.
        push_length_delimited(&mut node, 3, b"Scan");
        for value in metadata {
            push_length_delimited(&mut node, 6, value);
        }
        for value in execution_stats {
            push_length_delimited(&mut node, 7, value);
        }

        let mut query_plan = Vec::new();
        push_length_delimited(&mut query_plan, 1, &node);
        query_plan
    }

    fn query_plan_with_metadata_value(value: &[u8], struct_suffix: &[u8]) -> Vec<u8> {
        let entry = struct_entry(b"future", &[value], &[]);
        let metadata = metadata_struct(&[entry], struct_suffix);
        query_plan_with_structs(&[metadata], &[])
    }

    fn wrap_stats_with_query_stats(query_plan: &[u8], query_stats: &[Vec<u8>]) -> Vec<u8> {
        let mut stats = Vec::new();
        push_length_delimited(&mut stats, 1, query_plan);
        for value in query_stats {
            push_length_delimited(&mut stats, 2, value);
        }
        stats
    }

    fn wrap_stats(query_plan: &[u8]) -> Vec<u8> {
        wrap_stats_with_query_stats(query_plan, &[])
    }

    fn wrap_result_set(stats: &[u8]) -> Vec<u8> {
        let mut result_set = Vec::new();
        push_length_delimited(&mut result_set, 3, stats);
        result_set
    }

    #[test]
    fn rejects_unknown_or_invalid_well_known_metadata_before_conversion() {
        let valid_string = [0x1a, 0x02, b'o', b'k'];
        let unknown_value = [0x38, 0x01];
        let unknown_list = [0x10, 0x01];
        let unset_value = [];
        let invalid_null = [0x08, 0x01];
        let list_value = [0x32, 0x02, 0x10, 0x01];

        let cases = [
            (
                "unknown Struct field in QueryPlan",
                query_plan_with_metadata_value(&valid_string, &[0x10, 0x01]),
                "protobuf Struct contains unknown fields",
            ),
            (
                "unknown Value field in ResultSetStats",
                wrap_stats(&query_plan_with_metadata_value(&unknown_value, &[])),
                "protobuf Value contains unknown fields",
            ),
            (
                "unknown ListValue field in ResultSet",
                wrap_result_set(&wrap_stats(&query_plan_with_metadata_value(
                    &list_value,
                    &[],
                ))),
                "protobuf ListValue contains unknown fields",
            ),
            (
                "unset Value",
                query_plan_with_metadata_value(&unset_value, &[]),
                "protobuf Value kind is unset",
            ),
            (
                "invalid null enum",
                query_plan_with_metadata_value(&invalid_null, &[]),
                "invalid protobuf null Value",
            ),
        ];

        for (name, bytes, reason) in cases {
            let error = decode_plan_nodes(&bytes).expect_err(name);
            assert_eq!(error, WireError::InvalidWellKnownType(reason), "{name}");
        }

        // Make sure this intentionally adversarial fixture did not accidentally
        // turn into a valid list while changing the test encoder above.
        assert_eq!(unknown_list, [0x10, 0x01]);
    }

    #[test]
    fn validates_signature_metadata_in_all_detected_envelopes() {
        let unknown_value = [0x38, 0x01];
        let query_plan = query_plan_with_metadata_value(&unknown_value, &[]);
        let stats = wrap_stats(&query_plan);
        let result_set = wrap_result_set(&stats);

        for (name, bytes) in [
            ("QueryPlan", query_plan),
            ("ResultSetStats", stats),
            ("ResultSet", result_set),
        ] {
            assert_eq!(
                decode_plan_nodes(&bytes).expect_err(name),
                WireError::InvalidWellKnownType("protobuf Value contains unknown fields"),
                "{name}",
            );
        }
    }

    #[test]
    fn ignores_excluded_execution_and_query_stats() {
        let unknown_value = [0x38, 0x01];
        let invalid_entry = struct_entry(b"future", &[&unknown_value], &[]);
        let invalid_struct = metadata_struct(&[invalid_entry], &[]);
        let valid_entry = struct_entry(b"future", &[&string_value(b"ok")], &[]);
        let valid_metadata = metadata_struct(&[valid_entry], &[]);

        let query_plan = query_plan_with_structs(
            core::slice::from_ref(&valid_metadata),
            core::slice::from_ref(&invalid_struct),
        );
        assert!(decode_plan_nodes(&query_plan).is_ok());

        let stats =
            wrap_stats_with_query_stats(&query_plan, core::slice::from_ref(&invalid_struct));
        assert!(decode_plan_nodes(&stats).is_ok());
        assert!(decode_plan_nodes(&wrap_result_set(&stats)).is_ok());
    }

    #[test]
    fn skips_effective_subquery_cluster_node_values_recursively() {
        let unknown_value = [0x38, 0x01];
        let excluded_entry = struct_entry(b"subquery_cluster_node", &[&unknown_value], &[]);
        let top_level = metadata_struct(core::slice::from_ref(&excluded_entry), &[]);
        assert!(decode_plan_nodes(&query_plan_with_structs(&[top_level], &[])).is_ok());

        let nested_struct = metadata_struct(&[excluded_entry], &[]);
        let nested_value = struct_value(&nested_struct);
        let outer_entry = struct_entry(b"future", &[&nested_value], &[]);
        let outer_struct = metadata_struct(&[outer_entry], &[]);
        assert!(decode_plan_nodes(&query_plan_with_structs(&[outer_struct], &[])).is_ok());
    }

    #[test]
    fn duplicate_map_keys_use_the_last_entry_across_struct_segments() {
        let unknown_value = [0x38, 0x01];
        let invalid = struct_entry(b"future", &[&unknown_value], &[]);
        let valid_string = string_value(b"ok");
        let valid = struct_entry(b"future", &[&valid_string], &[]);

        let invalid_then_valid = [
            metadata_struct(core::slice::from_ref(&invalid), &[]),
            metadata_struct(core::slice::from_ref(&valid), &[]),
        ];
        assert!(decode_plan_nodes(&query_plan_with_structs(&invalid_then_valid, &[])).is_ok());

        let valid_then_invalid = [
            metadata_struct(&[valid], &[]),
            metadata_struct(&[invalid], &[]),
        ];
        assert_eq!(
            decode_plan_nodes(&query_plan_with_structs(&valid_then_invalid, &[])).unwrap_err(),
            WireError::InvalidWellKnownType("protobuf Value contains unknown fields"),
        );
    }

    #[test]
    fn value_oneof_replacement_defers_validation_until_the_effective_kind() {
        let invalid_null = [0x08, 0x01];
        let valid_string = string_value(b"ok");

        let mut invalid_then_valid = invalid_null.to_vec();
        invalid_then_valid.extend_from_slice(&valid_string);
        assert!(
            decode_plan_nodes(&query_plan_with_metadata_value(&invalid_then_valid, &[])).is_ok()
        );

        let mut valid_then_invalid = valid_string.clone();
        valid_then_invalid.extend_from_slice(&invalid_null);
        assert_eq!(
            decode_plan_nodes(&query_plan_with_metadata_value(&valid_then_invalid, &[]))
                .unwrap_err(),
            WireError::InvalidWellKnownType("invalid protobuf null Value"),
        );

        let unknown_nested_struct = [0x10, 0x01];
        let mut invalid_struct_then_string = struct_value(&unknown_nested_struct);
        invalid_struct_then_string.extend_from_slice(&valid_string);
        assert!(decode_plan_nodes(&query_plan_with_metadata_value(
            &invalid_struct_then_string,
            &[]
        ))
        .is_ok());
    }

    #[test]
    fn null_enum_validation_matches_generated_i32_truncation() {
        let mut truncates_to_zero = vec![0x08];
        push_varint(&mut truncates_to_zero, 1_u64 << 32);
        assert!(
            decode_plan_nodes(&query_plan_with_metadata_value(&truncates_to_zero, &[])).is_ok()
        );

        let mut truncates_to_one = vec![0x08];
        push_varint(&mut truncates_to_one, (1_u64 << 32) + 1);
        assert_eq!(
            decode_plan_nodes(&query_plan_with_metadata_value(&truncates_to_one, &[])).unwrap_err(),
            WireError::InvalidWellKnownType("invalid protobuf null Value"),
        );
    }

    #[test]
    fn repeated_singular_map_value_fields_merge_before_validation() {
        let invalid_null = [0x08, 0x01];
        let valid_string = string_value(b"ok");

        let valid_last = struct_entry(b"future", &[&invalid_null, &valid_string], &[]);
        let metadata = metadata_struct(&[valid_last], &[]);
        assert!(decode_plan_nodes(&query_plan_with_structs(&[metadata], &[])).is_ok());

        let invalid_last = struct_entry(b"future", &[&valid_string, &invalid_null], &[]);
        let metadata = metadata_struct(&[invalid_last], &[]);
        assert_eq!(
            decode_plan_nodes(&query_plan_with_structs(&[metadata], &[])).unwrap_err(),
            WireError::InvalidWellKnownType("invalid protobuf null Value"),
        );
    }

    #[test]
    fn repeated_same_message_oneof_members_merge() {
        let unknown_struct = [0x10, 0x01];
        let mut repeated_struct = struct_value(&unknown_struct);
        repeated_struct.extend_from_slice(&struct_value(&[]));
        assert_eq!(
            decode_plan_nodes(&query_plan_with_metadata_value(&repeated_struct, &[])).unwrap_err(),
            WireError::InvalidWellKnownType("protobuf Struct contains unknown fields"),
        );

        let unknown_list = [0x10, 0x01];
        let mut repeated_list = list_value(&unknown_list);
        repeated_list.extend_from_slice(&list_value(&[]));
        assert_eq!(
            decode_plan_nodes(&query_plan_with_metadata_value(&repeated_list, &[])).unwrap_err(),
            WireError::InvalidWellKnownType("protobuf ListValue contains unknown fields"),
        );

        let mut first_list = Vec::new();
        push_length_delimited(&mut first_list, 1, &[]);
        let mut second_list = Vec::new();
        push_length_delimited(&mut second_list, 1, &string_value(b"ok"));
        let mut appended_list = list_value(&first_list);
        appended_list.extend_from_slice(&list_value(&second_list));
        assert_eq!(
            decode_plan_nodes(&query_plan_with_metadata_value(&appended_list, &[])).unwrap_err(),
            WireError::InvalidWellKnownType("protobuf Value kind is unset"),
        );
    }

    #[test]
    fn ignores_map_entry_unknown_fields_but_retains_struct_unknown_fields() {
        let valid_string = string_value(b"ok");
        let entry_unknown = struct_entry(b"future", &[&valid_string], &[0x18, 0x01]);
        let metadata = metadata_struct(&[entry_unknown], &[]);
        assert!(decode_plan_nodes(&query_plan_with_structs(&[metadata], &[])).is_ok());

        let entry = struct_entry(b"future", &[&valid_string], &[]);
        let first_segment = metadata_struct(&[entry], &[0x10, 0x01]);
        let second_segment = metadata_struct(&[], &[]);
        assert_eq!(
            decode_plan_nodes(&query_plan_with_structs(
                &[first_segment, second_segment],
                &[]
            ))
            .unwrap_err(),
            WireError::InvalidWellKnownType("protobuf Struct contains unknown fields"),
        );
    }
}
