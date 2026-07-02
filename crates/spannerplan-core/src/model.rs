//! Data model for `google.spanner.v1` query plan messages (the subset this
//! crate needs). See `DESIGN.md` §5.5/§5.6.
//!
//! Field accessors mirror Go's `proto.Get*()` nil-safety: absent optional
//! data returns a zero value (empty string, 0, empty slice) rather than
//! `None`/panicking, so the porting of `queryplan.rs`/`plantree.rs` stays
//! mechanical. See `DESIGN.md` §7 item 2.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Top-level `{ "planNodes": [...] }` wire shape, i.e. `google.spanner.v1.QueryPlan`.
///
/// Distinct from [`crate::queryplan::QueryPlan`] (added in a later phase),
/// which is the validated, parent-linked graph built from this node list.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct QueryPlanMessage {
    #[cfg_attr(
        feature = "serde",
        serde(default, alias = "plan_nodes", rename = "planNodes")
    )]
    pub plan_nodes: Vec<PlanNode>,
}

/// `google.spanner.v1.PlanNode`.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct PlanNode {
    #[cfg_attr(
        feature = "serde",
        serde(default, deserialize_with = "de_i32_str_or_num")
    )]
    pub index: i32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub kind: Kind,
    #[cfg_attr(
        feature = "serde",
        serde(default, alias = "display_name", rename = "displayName")
    )]
    pub display_name: String,
    #[cfg_attr(
        feature = "serde",
        serde(default, alias = "child_links", rename = "childLinks")
    )]
    pub child_links: Vec<ChildLink>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            alias = "short_representation",
            rename = "shortRepresentation"
        )
    )]
    pub short_representation: Option<ShortRepresentation>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub metadata: Metadata,
    #[cfg_attr(
        feature = "serde",
        serde(default, alias = "execution_stats", rename = "executionStats")
    )]
    pub execution_stats: Option<Metadata>,
}

impl PlanNode {
    /// Mirrors Go `PlanNode.GetIndex()`.
    pub fn get_index(&self) -> i32 {
        self.index
    }

    /// Mirrors Go `PlanNode.GetKind()`.
    pub fn get_kind(&self) -> Kind {
        self.kind
    }

    /// Mirrors Go `PlanNode.GetDisplayName()`.
    pub fn get_display_name(&self) -> &str {
        &self.display_name
    }

    /// Mirrors Go `PlanNode.GetChildLinks()`.
    pub fn get_child_links(&self) -> &[ChildLink] {
        &self.child_links
    }

    /// Mirrors Go `PlanNode.GetShortRepresentation().GetDescription()`.
    pub fn get_short_representation_description(&self) -> &str {
        self.short_representation
            .as_ref()
            .map(|sr| sr.description.as_str())
            .unwrap_or("")
    }

    /// Mirrors Go `PlanNode.GetMetadata().GetFields()[key].GetStringValue()`.
    pub fn get_metadata_str(&self, key: &str) -> &str {
        self.metadata
            .get(key)
            .map(MetadataValue::as_str)
            .unwrap_or("")
    }

    /// Mirrors Go `PlanNode.GetExecutionStats()` (nil-safe: `None` when absent).
    pub fn get_execution_stats(&self) -> Option<&Metadata> {
        self.execution_stats.as_ref()
    }
}

/// `google.spanner.v1.PlanNode.Kind`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Unspecified,
    Relational,
    Scalar,
}

/// `google.spanner.v1.PlanNode.ChildLink`.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct ChildLink {
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "de_i32_str_or_num",
            alias = "child_index",
            rename = "childIndex"
        )
    )]
    pub child_index: i32,
    #[cfg_attr(feature = "serde", serde(default, rename = "type"))]
    pub r#type: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub variable: String,
}

impl ChildLink {
    pub fn get_child_index(&self) -> i32 {
        self.child_index
    }

    pub fn get_type(&self) -> &str {
        &self.r#type
    }

    pub fn get_variable(&self) -> &str {
        &self.variable
    }
}

/// `google.spanner.v1.PlanNode.ShortRepresentation`.
///
/// `subqueries` (`map<string, int32>`) is omitted: nothing in the rendering
/// pipeline reads it. Add it back if a future feature needs it.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct ShortRepresentation {
    #[cfg_attr(feature = "serde", serde(default))]
    pub description: String,
}

/// A decoded `google.protobuf.Struct`: `metadata` and `execution_stats` both
/// use this shape.
pub type Metadata = BTreeMap<String, MetadataValue>;

/// A decoded `google.protobuf.Value`.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum MetadataValue {
    #[default]
    Null,
    Bool(bool),
    /// `google.protobuf.Value.number_value` is always a JSON number (f64);
    /// see `stats.rs` (a later phase) for how execution-stat numbers, which
    /// Spanner actually sends as strings, are handled instead via `String`.
    Number(f64),
    String(String),
    List(Vec<MetadataValue>),
    Struct(Metadata),
}

impl MetadataValue {
    /// Mirrors Go `structpb.Value.GetStringValue()`: `""` for any non-string value.
    pub fn as_str(&self) -> &str {
        match self {
            MetadataValue::String(s) => s.as_str(),
            _ => "",
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Kind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct KindVisitor;

        impl serde::de::Visitor<'_> for KindVisitor {
            type Value = Kind;

            fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.write_str("a Kind string (RELATIONAL|SCALAR|KIND_UNSPECIFIED) or numeric code")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Kind, E> {
                Ok(match v {
                    "RELATIONAL" => Kind::Relational,
                    "SCALAR" => Kind::Scalar,
                    _ => Kind::Unspecified,
                })
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Kind, E> {
                Ok(match v {
                    1 => Kind::Relational,
                    2 => Kind::Scalar,
                    _ => Kind::Unspecified,
                })
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Kind, E> {
                Ok(match v {
                    1 => Kind::Relational,
                    2 => Kind::Scalar,
                    _ => Kind::Unspecified,
                })
            }
        }

        deserializer.deserialize_any(KindVisitor)
    }
}

/// protojson may encode `int32` fields (`index`, `childIndex`) as either a
/// JSON number or a JSON string; accept both. See `DESIGN.md` §5.2.
#[cfg(feature = "serde")]
fn de_i32_str_or_num<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct I32Visitor;

    impl serde::de::Visitor<'_> for I32Visitor {
        type Value = i32;

        fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            f.write_str("an integer or a string containing an integer")
        }

        fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<i32, E> {
            i32::try_from(v).map_err(|_| E::custom("integer out of range for i32"))
        }

        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<i32, E> {
            i32::try_from(v).map_err(|_| E::custom("integer out of range for i32"))
        }

        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<i32, E> {
            v.parse::<i32>()
                .map_err(|_| E::custom("invalid integer string"))
        }
    }

    deserializer.deserialize_any(I32Visitor)
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;

    #[test]
    fn decodes_index_and_child_index_from_number_or_string() {
        let node: PlanNode = serde_json::from_str(
            r#"{"index": "3", "displayName": "Scan", "childLinks": [{"childIndex": 1, "type": "Input"}]}"#,
        )
        .unwrap();
        assert_eq!(node.get_index(), 3);
        assert_eq!(node.get_display_name(), "Scan");
        assert_eq!(node.get_child_links()[0].get_child_index(), 1);
    }

    #[test]
    fn accepts_both_camel_and_snake_case_field_names() {
        let node: PlanNode =
            serde_json::from_str(r#"{"index": 0, "display_name": "Filter", "child_links": []}"#)
                .unwrap();
        assert_eq!(node.get_display_name(), "Filter");
    }

    #[test]
    fn kind_parses_known_strings_and_defaults_unknown_to_unspecified() {
        let node: PlanNode = serde_json::from_str(r#"{"index": 0, "kind": "RELATIONAL"}"#).unwrap();
        assert_eq!(node.get_kind(), Kind::Relational);

        let node: PlanNode =
            serde_json::from_str(r#"{"index": 0, "kind": "SOMETHING_NEW"}"#).unwrap();
        assert_eq!(node.get_kind(), Kind::Unspecified);
    }

    #[test]
    fn missing_optional_fields_default_to_zero_values() {
        let node: PlanNode = serde_json::from_str(r#"{"index": 0}"#).unwrap();
        assert_eq!(node.get_display_name(), "");
        assert_eq!(node.get_short_representation_description(), "");
        assert_eq!(node.get_metadata_str("scan_type"), "");
        assert!(node.get_execution_stats().is_none());
    }

    #[test]
    fn metadata_string_value_accessor_matches_go_get_string_value_semantics() {
        let node: PlanNode = serde_json::from_str(
            r#"{"index": 0, "metadata": {"scan_type": "TableScan", "seekable_key_size": 1}}"#,
        )
        .unwrap();
        assert_eq!(node.get_metadata_str("scan_type"), "TableScan");
        // Non-string metadata values return "", mirroring GetStringValue() on a non-string Value.
        assert_eq!(node.get_metadata_str("seekable_key_size"), "");
    }

    #[test]
    fn query_plan_message_decodes_plan_nodes_array() {
        let msg: QueryPlanMessage =
            serde_json::from_str(r#"{"planNodes": [{"index": 0}, {"index": 1}]}"#).unwrap();
        assert_eq!(msg.plan_nodes.len(), 2);
    }
}
