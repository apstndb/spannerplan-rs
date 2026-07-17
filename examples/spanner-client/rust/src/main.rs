//! Fetch QueryPlan via google-cloud-spanner (gRPC) and render with native spannerplan.

use std::env;
use std::fmt;
use std::fs;
use std::path::PathBuf;

use google_cloud_googleapis::spanner::v1;
use google_cloud_spanner::client::Spanner;
use google_cloud_spanner::model::execute_sql_request::QueryMode;
use google_cloud_spanner::model::{PlanNode, QueryPlan};
use google_cloud_spanner::statement::Statement;
use prost::Message;
use spannerplan::core::reference::{
    parse_format, parse_render_mode, render_tree_table_with_config, RenderConfig,
};
use spannerplan::core::wire::decode_plan_nodes;

struct CliOptions {
    query_mode: String,
    project: String,
    instance: String,
    database: String,
    sql: String,
}

fn env_or_none(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn default_query_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("query.sql")
}

fn load_sql(
    query: Option<&str>,
    query_file: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(sql) = query {
        return Ok(sql.trim().to_string());
    }
    let path = query_file
        .map(PathBuf::from)
        .unwrap_or_else(default_query_file);
    Ok(fs::read_to_string(path)?.trim().to_string())
}

fn print_usage() {
    eprintln!(
        "usage: spanner-client-example [options]\n  \
         --query-mode PLAN|PROFILE   Spanner execute-sql mode (default: PLAN)\n  \
         --project PROJECT           GCP project id\n  \
         --instance INSTANCE         Spanner instance id\n  \
         --database DATABASE         Spanner database id\n  \
         --query SQL                 SQL text (overrides --query-file)\n  \
         --query-file PATH           SQL file (default: ../query.sql)\n\n\
         Environment (when flags omitted):\n  \
         SPANNER_QUERY_MODE, SPANNER_PROJECT_ID, SPANNER_INSTANCE_ID,\n  \
         SPANNER_DATABASE_ID, SPANNER_QUERY, SPANNER_QUERY_FILE"
    );
}

fn parse_cli_options() -> Result<CliOptions, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut query_mode = env_or_none("SPANNER_QUERY_MODE").unwrap_or_else(|| "PLAN".into());
    let mut project = env_or_none("SPANNER_PROJECT_ID");
    let mut instance = env_or_none("SPANNER_INSTANCE_ID");
    let mut database = env_or_none("SPANNER_DATABASE_ID");
    let mut query = env_or_none("SPANNER_QUERY");
    let mut query_file = env_or_none("SPANNER_QUERY_FILE");

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            "--query-mode" if i + 1 < args.len() => {
                query_mode = args[i + 1].clone();
                i += 2;
            }
            "--project" if i + 1 < args.len() => {
                project = Some(args[i + 1].clone());
                i += 2;
            }
            "--instance" if i + 1 < args.len() => {
                instance = Some(args[i + 1].clone());
                i += 2;
            }
            "--database" if i + 1 < args.len() => {
                database = Some(args[i + 1].clone());
                i += 2;
            }
            "--query" if i + 1 < args.len() => {
                query = Some(args[i + 1].clone());
                i += 2;
            }
            "--query-file" if i + 1 < args.len() => {
                query_file = Some(args[i + 1].clone());
                i += 2;
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }

    let query_mode = query_mode.to_uppercase();
    if query_mode != "PLAN" && query_mode != "PROFILE" {
        return Err(format!("query mode must be PLAN or PROFILE, got: {query_mode}").into());
    }
    let project = project.ok_or("missing required value: set --project or SPANNER_PROJECT_ID")?;
    let instance =
        instance.ok_or("missing required value: set --instance or SPANNER_INSTANCE_ID")?;
    let database =
        database.ok_or("missing required value: set --database or SPANNER_DATABASE_ID")?;
    let sql = load_sql(query.as_deref(), query_file.as_deref())?;

    Ok(CliOptions {
        query_mode,
        project,
        instance,
        database,
        sql,
    })
}

fn spanner_query_mode(mode: &str) -> QueryMode {
    if mode == "PROFILE" {
        QueryMode::Profile
    } else {
        QueryMode::Plan
    }
}

fn render_mode_for(mode: &str) -> &'static str {
    if mode == "PROFILE" {
        "PROFILE"
    } else {
        "PLAN"
    }
}

#[derive(Debug, PartialEq, Eq)]
struct PlanConversionError {
    plan_node_position: usize,
    plan_node_index: i32,
    path: String,
    message: String,
}

impl PlanConversionError {
    fn new(
        plan_node_position: usize,
        plan_node_index: i32,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            plan_node_position,
            plan_node_index,
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for PlanConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "plan_nodes[{}] (PlanNode index {}) {}: {}",
            self.plan_node_position, self.plan_node_index, self.path, self.message
        )
    }
}

impl std::error::Error for PlanConversionError {}

fn encode_query_plan(plan: QueryPlan) -> Result<Vec<u8>, PlanConversionError> {
    let plan_nodes = plan
        .plan_nodes
        .into_iter()
        .enumerate()
        .map(|(position, node)| encode_plan_node(node, position))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v1::QueryPlan { plan_nodes }.encode_to_vec())
}

fn encode_plan_node(
    node: PlanNode,
    plan_node_position: usize,
) -> Result<v1::PlanNode, PlanConversionError> {
    let plan_node_index = node.index;
    let kind = node.kind.value().ok_or_else(|| {
        PlanConversionError::new(
            plan_node_position,
            plan_node_index,
            "kind",
            format!(
                "cannot map enum value {} to its numeric protobuf value",
                node.kind
            ),
        )
    })?;
    let child_links = node
        .child_links
        .into_iter()
        .map(|link| v1::plan_node::ChildLink {
            child_index: link.child_index,
            r#type: link.r#type,
            variable: link.variable,
        })
        .collect();
    let short_representation =
        node.short_representation
            .map(|short| v1::plan_node::ShortRepresentation {
                description: short.description,
                subqueries: short.subqueries,
            });
    let metadata = node
        .metadata
        .map(|value| encode_struct(value, plan_node_position, plan_node_index, "metadata"))
        .transpose()?;
    let execution_stats = node
        .execution_stats
        .map(|value| {
            encode_struct(
                value,
                plan_node_position,
                plan_node_index,
                "execution_stats",
            )
        })
        .transpose()?;
    Ok(v1::PlanNode {
        index: plan_node_index,
        kind,
        display_name: node.display_name,
        child_links,
        short_representation,
        metadata,
        execution_stats,
    })
}

fn encode_struct(
    value: serde_json::Map<String, serde_json::Value>,
    plan_node_position: usize,
    plan_node_index: i32,
    path: &str,
) -> Result<prost_types::Struct, PlanConversionError> {
    let fields = value
        .into_iter()
        .map(|(key, value)| {
            let value_path = format!("{path}.{key}");
            encode_value(value, plan_node_position, plan_node_index, &value_path)
                .map(|value| (key, value))
        })
        .collect::<Result<_, _>>()?;
    Ok(prost_types::Struct { fields })
}

fn encode_value(
    value: serde_json::Value,
    plan_node_position: usize,
    plan_node_index: i32,
    path: &str,
) -> Result<prost_types::Value, PlanConversionError> {
    use prost_types::value::Kind;

    let kind = match value {
        serde_json::Value::Null => Kind::NullValue(0),
        serde_json::Value::Bool(value) => Kind::BoolValue(value),
        serde_json::Value::Number(value) => Kind::NumberValue(encode_number(
            &value,
            plan_node_position,
            plan_node_index,
            path,
        )?),
        serde_json::Value::String(value) => Kind::StringValue(value),
        serde_json::Value::Array(values) => Kind::ListValue(prost_types::ListValue {
            values: values
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    encode_value(
                        value,
                        plan_node_position,
                        plan_node_index,
                        &format!("{path}[{index}]"),
                    )
                })
                .collect::<Result<_, _>>()?,
        }),
        serde_json::Value::Object(value) => Kind::StructValue(encode_struct(
            value,
            plan_node_position,
            plan_node_index,
            path,
        )?),
    };
    Ok(prost_types::Value { kind: Some(kind) })
}

fn encode_number(
    value: &serde_json::Number,
    plan_node_position: usize,
    plan_node_index: i32,
    path: &str,
) -> Result<f64, PlanConversionError> {
    let exact_integer = if let Some(value) = value.as_u64() {
        integer_magnitude_is_exact_f64(value)
    } else if let Some(value) = value.as_i64() {
        integer_magnitude_is_exact_f64(value.unsigned_abs())
    } else {
        true
    };
    if !exact_integer {
        return Err(PlanConversionError::new(
            plan_node_position,
            plan_node_index,
            path,
            format!(
                "JSON integer {value} cannot be represented exactly as protobuf number_value (f64)"
            ),
        ));
    }

    value.as_f64().ok_or_else(|| {
        PlanConversionError::new(
            plan_node_position,
            plan_node_index,
            path,
            format!("JSON number {value} cannot be represented as protobuf number_value (f64)"),
        )
    })
}

fn integer_magnitude_is_exact_f64(value: u64) -> bool {
    if value == 0 {
        return true;
    }
    let significant_bits = u64::BITS - value.leading_zeros();
    significant_bits <= f64::MANTISSA_DIGITS
        || value.trailing_zeros() >= significant_bits - f64::MANTISSA_DIGITS
}

async fn fetch_query_plan_wire(
    database: &str,
    sql: &str,
    query_mode: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let client = Spanner::builder().build().await?;
    let database_client = client.database_client(database).build().await?;
    let transaction = database_client.single_use().build();
    let statement = Statement::builder(sql)
        .set_query_mode(spanner_query_mode(query_mode))
        .build();
    let mut result = transaction.execute_query(statement).await?;
    while let Some(row) = result.next().await {
        row?;
    }
    let plan = result
        .stats()
        .and_then(|stats| stats.query_plan.clone())
        .ok_or_else(|| format!("QueryPlan missing from {query_mode}-mode execute_sql response"))?;
    Ok(encode_query_plan(plan)?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = parse_cli_options()?;
    let database = format!(
        "projects/{}/instances/{}/databases/{}",
        opts.project, opts.instance, opts.database
    );
    let wire = fetch_query_plan_wire(&database, &opts.sql, &opts.query_mode).await?;
    let plan_nodes = decode_plan_nodes(&wire).map_err(|e| e.to_string())?;
    let output = render_tree_table_with_config(
        &plan_nodes,
        parse_render_mode(render_mode_for(&opts.query_mode)).map_err(|e| e.to_string())?,
        parse_format("CURRENT").map_err(|e| e.to_string())?,
        &RenderConfig::default(),
    )
    .map_err(|e| e.to_string())?;

    print!("{output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use google_cloud_spanner::model::plan_node::{ChildLink, Kind, ShortRepresentation};
    use prost_types::value::Kind as ValueKind;
    use serde_json::json;

    use super::*;

    fn struct_map(value: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        value
            .as_object()
            .expect("test value must be an object")
            .clone()
    }

    #[test]
    fn rejects_unmappable_plan_node_kind_with_node_context() {
        let node = PlanNode::new()
            .set_index(17)
            .set_kind(Kind::from("FUTURE_KIND"));
        let plan = QueryPlan::new().set_plan_nodes([PlanNode::new(), node]);

        let error = encode_query_plan(plan).unwrap_err();

        assert_eq!(error.plan_node_position, 1);
        assert_eq!(error.plan_node_index, 17);
        assert_eq!(error.path, "kind");
        assert!(error.to_string().contains("FUTURE_KIND"));
    }

    #[test]
    fn preserves_unknown_numeric_plan_node_kind() {
        let node = PlanNode::new().set_kind(Kind::from(99));

        let encoded = encode_plan_node(node, 0).unwrap();

        assert_eq!(encoded.kind, 99);
    }

    #[test]
    fn rejects_inexact_integer_with_nested_metadata_path() {
        // serde_json is built without arbitrary_precision here, so as_f64() is
        // always Some. This is the stronger failure mode: the conversion exists
        // but would silently round 2^53 + 1.
        let large_integer = 9_007_199_254_740_993_u64;
        let number = serde_json::Number::from(large_integer);
        assert_eq!(number.as_f64(), Some(9_007_199_254_740_992.0));
        let node = PlanNode::new()
            .set_index(23)
            .set_metadata(struct_map(json!({
                "outer": [{ "large": large_integer }]
            })));

        let error = encode_plan_node(node, 4).unwrap_err();

        assert_eq!(error.plan_node_position, 4);
        assert_eq!(error.plan_node_index, 23);
        assert_eq!(error.path, "metadata.outer[0].large");
        assert!(error.to_string().contains("cannot be represented exactly"));
    }

    #[test]
    fn reports_nested_execution_stats_path() {
        let node = PlanNode::new()
            .set_index(29)
            .set_execution_stats(struct_map(json!({
                "metrics": { "samples": [1, 9_007_199_254_740_993_u64] }
            })));

        let error = encode_plan_node(node, 5).unwrap_err();

        assert_eq!(error.plan_node_position, 5);
        assert_eq!(error.plan_node_index, 29);
        assert_eq!(error.path, "execution_stats.metrics.samples[1]");
    }

    #[test]
    fn preserves_all_plan_node_fields() {
        let node = PlanNode::new()
            .set_index(7)
            .set_kind(Kind::Relational)
            .set_display_name("Distributed Union")
            .set_child_links([ChildLink::new()
                .set_child_index(8)
                .set_type("Child")
                .set_variable("row")])
            .set_short_representation(
                ShortRepresentation::new()
                    .set_description("short")
                    .set_subqueries([("subquery", 9)]),
            )
            .set_metadata(struct_map(json!({
                "flag": true,
                "nested": { "value": "metadata" }
            })))
            .set_execution_stats(struct_map(json!({
                "rows": 42,
                "ratio": 1.5,
                "exact_large_integer": 9_223_372_036_854_775_808_u64
            })));

        let encoded = encode_plan_node(node, 0).unwrap();

        assert_eq!(encoded.index, 7);
        assert_eq!(encoded.kind, 1);
        assert_eq!(encoded.display_name, "Distributed Union");
        assert_eq!(encoded.child_links.len(), 1);
        assert_eq!(encoded.child_links[0].child_index, 8);
        assert_eq!(encoded.child_links[0].r#type, "Child");
        assert_eq!(encoded.child_links[0].variable, "row");
        let short = encoded.short_representation.unwrap();
        assert_eq!(short.description, "short");
        assert_eq!(short.subqueries["subquery"], 9);
        let metadata = encoded.metadata.unwrap();
        assert!(matches!(
            metadata.fields["flag"].kind,
            Some(ValueKind::BoolValue(true))
        ));
        let stats = encoded.execution_stats.unwrap();
        assert!(matches!(
            stats.fields["rows"].kind,
            Some(ValueKind::NumberValue(42.0))
        ));
        assert!(matches!(
            stats.fields["ratio"].kind,
            Some(ValueKind::NumberValue(1.5))
        ));
        assert!(matches!(
            stats.fields["exact_large_integer"].kind,
            Some(ValueKind::NumberValue(9_223_372_036_854_775_808.0))
        ));
    }
}
