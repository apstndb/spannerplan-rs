//! Execution-statistics types and extraction. Port of `stats/types.go` +
//! `stats/extract.go`.
//!
//! Go extracts by JSON-round-tripping the `execution_stats` protobuf Struct
//! into typed structs; here extraction reads the already-decoded
//! [`Metadata`] map directly (`DESIGN.md` §6.5 option B), keeping this in
//! the `no_std` core.
//!
//! Intentional divergences from Go's `encoding/json` behavior, both
//! unobservable with real Spanner data:
//! - key matching is exact-case (Go matches JSON keys case-insensitively);
//! - `num_checkpoints` accepts a string or a number (Go's `json.Number`
//!   accepts only numbers);
//! - integral f64 values ≥ 2^63 format via Rust float display (`1e20`) rather
//!   than a JSON integer literal, because `as i64` saturates.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::model::{Metadata, MetadataValue, PlanNode};

/// One bucket of a stat-value histogram.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExecutionStatsHistogram {
    pub count: String,
    pub percentage: String,
    pub lower_bound: String,
    pub upper_bound: String,
}

/// One statistic: a total (as Spanner-formatted text) with optional unit,
/// mean, standard deviation, and histogram.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExecutionStatsValue {
    pub unit: String,
    pub total: String,
    pub mean: String,
    pub std_deviation: String,
    pub histogram: Vec<ExecutionStatsHistogram>,
}

impl core::fmt::Display for ExecutionStatsValue {
    /// `total` alone when there is no unit, else `total unit` — e.g.
    /// `12.25 msecs`. Mirrors Go `ExecutionStatsValue.String()`.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.unit.is_empty() {
            f.write_str(&self.total)
        } else {
            write!(f, "{} {}", self.total, self.unit)
        }
    }
}

/// The `execution_summary` object.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExecutionStatsSummary {
    pub num_executions: String,
    pub checkpoint_time: String,
    pub execution_end_timestamp: String,
    pub execution_start_timestamp: String,
    pub num_checkpoints: String,
}

/// All execution statistics attached to one plan node. Field JSON keys match
/// `stats/types.go` exactly, including the spaced display-style keys.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExecutionStats {
    /// `Disk Usage (KBytes)`
    pub disk_usage_kbytes: ExecutionStatsValue,
    /// `Disk Write Latency (msecs)`
    pub disk_write_latency_msecs: ExecutionStatsValue,
    /// `Peak Buffering Memory Usage (KBytes)`
    pub peak_buffering_memory_usage_kbytes: ExecutionStatsValue,
    /// `Peak Memory Usage (KBytes)`
    pub peak_memory_usage_kbytes: ExecutionStatsValue,
    /// `Rows Spooled`
    pub rows_spooled: ExecutionStatsValue,
    /// `rows`
    pub rows: ExecutionStatsValue,
    /// `latency`
    pub latency: ExecutionStatsValue,
    /// `cpu_time`
    pub cpu_time: ExecutionStatsValue,
    /// `deleted_rows`
    pub deleted_rows: ExecutionStatsValue,
    /// `filesystem_delay_seconds`
    pub filesystem_delay_seconds: ExecutionStatsValue,
    /// `filtered_rows`
    pub filtered_rows: ExecutionStatsValue,
    /// `remote_calls`
    pub remote_calls: ExecutionStatsValue,
    /// `scanned_rows`
    pub scanned_rows: ExecutionStatsValue,
    /// `execution_summary`
    pub execution_summary: ExecutionStatsSummary,
    /// `Number of Batches`
    pub number_of_batches: ExecutionStatsValue,
}

/// Errors from [`extract`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatsError {
    /// A field held a JSON type the schema doesn't allow (mirrors Go's
    /// `json: cannot unmarshal ...` errors).
    TypeMismatch { key: String, expected: &'static str },
    /// An unknown key was found while `disallow_unknown_fields` was set.
    UnknownField { key: String },
}

impl core::fmt::Display for StatsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StatsError::TypeMismatch { key, expected } => {
                write!(
                    f,
                    "stats: cannot unmarshal field {key:?}: expected {expected}"
                )
            }
            StatsError::UnknownField { key } => {
                write!(f, "stats: unknown field {key:?}")
            }
        }
    }
}

const TOP_LEVEL_KEYS: [&str; 15] = [
    "Disk Usage (KBytes)",
    "Disk Write Latency (msecs)",
    "Peak Buffering Memory Usage (KBytes)",
    "Peak Memory Usage (KBytes)",
    "Rows Spooled",
    "rows",
    "latency",
    "cpu_time",
    "deleted_rows",
    "filesystem_delay_seconds",
    "filtered_rows",
    "remote_calls",
    "scanned_rows",
    "execution_summary",
    "Number of Batches",
];

const VALUE_KEYS: [&str; 5] = ["unit", "total", "mean", "std_deviation", "histogram"];

const SUMMARY_KEYS: [&str; 5] = [
    "num_executions",
    "checkpoint_time",
    "execution_end_timestamp",
    "execution_start_timestamp",
    "num_checkpoints",
];

const HISTOGRAM_KEYS: [&str; 4] = ["count", "percentage", "lower_bound", "upper_bound"];

/// Extracts typed [`ExecutionStats`] from a node's `execution_stats` Struct.
/// A node without stats yields the all-zero default (matching Go, where
/// round-tripping nil leaves the target zero-valued). With
/// `disallow_unknown_fields`, any unknown key at any nesting level is an
/// error (mirrors `json.Decoder.DisallowUnknownFields`, which applies
/// recursively).
pub fn extract(
    node: &PlanNode,
    disallow_unknown_fields: bool,
) -> Result<ExecutionStats, StatsError> {
    let Some(stats) = node.get_execution_stats() else {
        return Ok(ExecutionStats::default());
    };
    extract_from_metadata(stats, disallow_unknown_fields)
}

/// [`extract`] on an already-plucked `execution_stats` map.
pub fn extract_from_metadata(
    stats: &Metadata,
    disallow_unknown_fields: bool,
) -> Result<ExecutionStats, StatsError> {
    if disallow_unknown_fields {
        check_known_keys(stats, &TOP_LEVEL_KEYS)?;
    }

    Ok(ExecutionStats {
        disk_usage_kbytes: value_field(stats, "Disk Usage (KBytes)", disallow_unknown_fields)?,
        disk_write_latency_msecs: value_field(
            stats,
            "Disk Write Latency (msecs)",
            disallow_unknown_fields,
        )?,
        peak_buffering_memory_usage_kbytes: value_field(
            stats,
            "Peak Buffering Memory Usage (KBytes)",
            disallow_unknown_fields,
        )?,
        peak_memory_usage_kbytes: value_field(
            stats,
            "Peak Memory Usage (KBytes)",
            disallow_unknown_fields,
        )?,
        rows_spooled: value_field(stats, "Rows Spooled", disallow_unknown_fields)?,
        rows: value_field(stats, "rows", disallow_unknown_fields)?,
        latency: value_field(stats, "latency", disallow_unknown_fields)?,
        cpu_time: value_field(stats, "cpu_time", disallow_unknown_fields)?,
        deleted_rows: value_field(stats, "deleted_rows", disallow_unknown_fields)?,
        filesystem_delay_seconds: value_field(
            stats,
            "filesystem_delay_seconds",
            disallow_unknown_fields,
        )?,
        filtered_rows: value_field(stats, "filtered_rows", disallow_unknown_fields)?,
        remote_calls: value_field(stats, "remote_calls", disallow_unknown_fields)?,
        scanned_rows: value_field(stats, "scanned_rows", disallow_unknown_fields)?,
        execution_summary: summary_field(stats, "execution_summary", disallow_unknown_fields)?,
        number_of_batches: value_field(stats, "Number of Batches", disallow_unknown_fields)?,
    })
}

fn check_known_keys(map: &Metadata, known: &[&str]) -> Result<(), StatsError> {
    for key in map.keys() {
        if !known.contains(&key.as_str()) {
            return Err(StatsError::UnknownField { key: key.clone() });
        }
    }
    Ok(())
}

/// Reads a nested object field; absent / null → `None`; non-object → error.
fn object_field<'m>(map: &'m Metadata, key: &str) -> Result<Option<&'m Metadata>, StatsError> {
    match map.get(key) {
        None | Some(MetadataValue::Null) => Ok(None),
        Some(MetadataValue::Struct(m)) => Ok(Some(m)),
        Some(_) => Err(StatsError::TypeMismatch {
            key: key.to_string(),
            expected: "object",
        }),
    }
}

/// Reads a string field; absent / null → `""`; non-string → error (mirrors
/// Go decoding a non-string JSON value into a `string` field).
fn string_field(map: &Metadata, key: &str) -> Result<String, StatsError> {
    match map.get(key) {
        None | Some(MetadataValue::Null) => Ok(String::new()),
        Some(MetadataValue::String(s)) => Ok(s.clone()),
        Some(_) => Err(StatsError::TypeMismatch {
            key: key.to_string(),
            expected: "string",
        }),
    }
}

/// Reads `num_checkpoints`: a number (formatted like its JSON literal) or a
/// string.
fn number_or_string_field(map: &Metadata, key: &str) -> Result<String, StatsError> {
    match map.get(key) {
        None | Some(MetadataValue::Null) => Ok(String::new()),
        Some(MetadataValue::String(s)) => Ok(s.clone()),
        Some(MetadataValue::Number(n)) => Ok(format_json_number(*n)),
        Some(_) => Err(StatsError::TypeMismatch {
            key: key.to_string(),
            expected: "number or string",
        }),
    }
}

/// Formats an f64 the way its JSON integer literal would look when integral
/// (`5` not `5.0`), falling back to Rust's shortest-roundtrip float display.
/// (Written without `f64::fract`/`abs`, which live in `std`, not `core`.)
fn format_json_number(n: f64) -> String {
    let truncated = n as i64; // saturating cast; NaN -> 0
    if truncated as f64 == n {
        format!("{truncated}")
    } else {
        format!("{n}")
    }
}

fn value_field(
    map: &Metadata,
    key: &str,
    disallow_unknown_fields: bool,
) -> Result<ExecutionStatsValue, StatsError> {
    let Some(obj) = object_field(map, key)? else {
        return Ok(ExecutionStatsValue::default());
    };
    if disallow_unknown_fields {
        check_known_keys(obj, &VALUE_KEYS)?;
    }
    Ok(ExecutionStatsValue {
        unit: string_field(obj, "unit")?,
        total: string_field(obj, "total")?,
        mean: string_field(obj, "mean")?,
        std_deviation: string_field(obj, "std_deviation")?,
        histogram: histogram_field(obj, "histogram", disallow_unknown_fields)?,
    })
}

fn histogram_field(
    map: &Metadata,
    key: &str,
    disallow_unknown_fields: bool,
) -> Result<Vec<ExecutionStatsHistogram>, StatsError> {
    let list = match map.get(key) {
        None | Some(MetadataValue::Null) => return Ok(Vec::new()),
        Some(MetadataValue::List(l)) => l,
        Some(_) => {
            return Err(StatsError::TypeMismatch {
                key: key.to_string(),
                expected: "array",
            })
        }
    };
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let MetadataValue::Struct(obj) = item else {
            return Err(StatsError::TypeMismatch {
                key: key.to_string(),
                expected: "array of objects",
            });
        };
        if disallow_unknown_fields {
            check_known_keys(obj, &HISTOGRAM_KEYS)?;
        }
        out.push(ExecutionStatsHistogram {
            count: string_field(obj, "count")?,
            percentage: string_field(obj, "percentage")?,
            lower_bound: string_field(obj, "lower_bound")?,
            upper_bound: string_field(obj, "upper_bound")?,
        });
    }
    Ok(out)
}

fn summary_field(
    map: &Metadata,
    key: &str,
    disallow_unknown_fields: bool,
) -> Result<ExecutionStatsSummary, StatsError> {
    let Some(obj) = object_field(map, key)? else {
        return Ok(ExecutionStatsSummary::default());
    };
    if disallow_unknown_fields {
        check_known_keys(obj, &SUMMARY_KEYS)?;
    }
    Ok(ExecutionStatsSummary {
        num_executions: string_field(obj, "num_executions")?,
        checkpoint_time: string_field(obj, "checkpoint_time")?,
        execution_end_timestamp: string_field(obj, "execution_end_timestamp")?,
        execution_start_timestamp: string_field(obj, "execution_start_timestamp")?,
        num_checkpoints: number_or_string_field(obj, "num_checkpoints")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn s(v: &str) -> MetadataValue {
        MetadataValue::String(v.to_string())
    }

    fn obj(entries: &[(&str, MetadataValue)]) -> MetadataValue {
        MetadataValue::Struct(metadata(entries))
    }

    fn metadata(entries: &[(&str, MetadataValue)]) -> Metadata {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn node_with_stats(stats: Metadata) -> PlanNode {
        PlanNode {
            execution_stats: Some(stats),
            ..PlanNode::default()
        }
    }

    fn realistic_stats() -> Metadata {
        metadata(&[
            (
                "rows",
                obj(&[("total", s("386")), ("unit", s("rows")), ("mean", s("386"))]),
            ),
            (
                "latency",
                obj(&[("total", s("12.25")), ("unit", s("msecs"))]),
            ),
            (
                "execution_summary",
                obj(&[
                    ("num_executions", s("1")),
                    ("execution_start_timestamp", s("1694527838.129552")),
                ]),
            ),
            (
                "cpu_time",
                obj(&[("total", s("11.63")), ("unit", s("msecs"))]),
            ),
        ])
    }

    #[test]
    fn extract_without_stats_yields_default() {
        let node = PlanNode::default();
        assert_eq!(extract(&node, true).unwrap(), ExecutionStats::default());
    }

    #[test]
    fn extract_realistic_stats() {
        let node = node_with_stats(realistic_stats());
        let stats = extract(&node, true).unwrap();
        assert_eq!(stats.rows.total, "386");
        assert_eq!(stats.rows.unit, "rows");
        assert_eq!(stats.latency.to_string(), "12.25 msecs");
        assert_eq!(stats.execution_summary.num_executions, "1");
        assert_eq!(stats.cpu_time.total, "11.63");
    }

    #[test]
    fn value_display_without_unit_is_total_only() {
        let v = ExecutionStatsValue {
            total: "42".to_string(),
            ..ExecutionStatsValue::default()
        };
        assert_eq!(v.to_string(), "42");
        assert_eq!(ExecutionStatsValue::default().to_string(), "");
    }

    #[test]
    fn unknown_top_level_field_errors_only_when_disallowed() {
        let mut m = realistic_stats();
        m.insert("brand_new_stat".to_string(), s("1"));
        let node = node_with_stats(m);
        assert_eq!(
            extract(&node, true).unwrap_err(),
            StatsError::UnknownField {
                key: "brand_new_stat".to_string()
            }
        );
        assert!(extract(&node, false).is_ok());
    }

    #[test]
    fn unknown_nested_field_errors_when_disallowed() {
        let m = metadata(&[(
            "rows",
            obj(&[("total", s("1")), ("weird_subfield", s("x"))]),
        )]);
        let node = node_with_stats(m);
        assert_eq!(
            extract(&node, true).unwrap_err(),
            StatsError::UnknownField {
                key: "weird_subfield".to_string()
            }
        );
        assert!(extract(&node, false).is_ok());
    }

    #[test]
    fn non_string_total_is_a_type_mismatch() {
        // Mirrors Go: a JSON number cannot unmarshal into a string field.
        let m = metadata(&[("rows", obj(&[("total", MetadataValue::Number(386.0))]))]);
        let node = node_with_stats(m);
        assert_eq!(
            extract(&node, false).unwrap_err(),
            StatsError::TypeMismatch {
                key: "total".to_string(),
                expected: "string"
            }
        );
    }

    #[test]
    fn null_fields_are_tolerated_as_zero_values() {
        let m = metadata(&[
            ("rows", MetadataValue::Null),
            ("latency", obj(&[("total", MetadataValue::Null)])),
        ]);
        let node = node_with_stats(m);
        let stats = extract(&node, false).unwrap();
        assert_eq!(stats.rows, ExecutionStatsValue::default());
        assert_eq!(stats.latency.total, "");
    }

    #[test]
    fn histogram_parses_buckets() {
        let m = metadata(&[(
            "latency",
            obj(&[
                ("total", s("10")),
                (
                    "histogram",
                    MetadataValue::List(vec![
                        obj(&[
                            ("count", s("5")),
                            ("percentage", s("50")),
                            ("lower_bound", s("0")),
                            ("upper_bound", s("1")),
                        ]),
                        obj(&[("count", s("5")), ("percentage", s("50"))]),
                    ]),
                ),
            ]),
        )]);
        let node = node_with_stats(m);
        let stats = extract(&node, true).unwrap();
        assert_eq!(stats.latency.histogram.len(), 2);
        assert_eq!(stats.latency.histogram[0].count, "5");
        assert_eq!(stats.latency.histogram[0].upper_bound, "1");
        assert_eq!(stats.latency.histogram[1].lower_bound, "");
    }

    #[test]
    fn num_checkpoints_accepts_number_or_string() {
        for value in [MetadataValue::Number(5.0), s("5")] {
            let m = metadata(&[("execution_summary", obj(&[("num_checkpoints", value)]))]);
            let node = node_with_stats(m);
            let stats = extract(&node, true).unwrap();
            assert_eq!(stats.execution_summary.num_checkpoints, "5");
        }
    }
}
