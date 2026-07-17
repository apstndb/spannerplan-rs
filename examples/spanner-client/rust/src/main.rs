//! Fetch QueryPlan via google-cloud-spanner (gRPC) and render with native spannerplan.

use std::env;
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

fn encode_query_plan(plan: QueryPlan) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let plan_nodes = plan
        .plan_nodes
        .into_iter()
        .map(encode_plan_node)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v1::QueryPlan { plan_nodes }.encode_to_vec())
}

fn encode_plan_node(node: PlanNode) -> Result<v1::PlanNode, Box<dyn std::error::Error>> {
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
    Ok(v1::PlanNode {
        index: node.index,
        kind: node.kind.value().unwrap_or_default(),
        display_name: node.display_name,
        child_links,
        short_representation,
        metadata: node.metadata.map(encode_struct),
        execution_stats: node.execution_stats.map(encode_struct),
    })
}

fn encode_struct(value: serde_json::Map<String, serde_json::Value>) -> prost_types::Struct {
    prost_types::Struct {
        fields: value
            .into_iter()
            .map(|(key, value)| (key, encode_value(value)))
            .collect(),
    }
}

fn encode_value(value: serde_json::Value) -> prost_types::Value {
    use prost_types::value::Kind;

    let kind = match value {
        serde_json::Value::Null => Kind::NullValue(0),
        serde_json::Value::Bool(value) => Kind::BoolValue(value),
        serde_json::Value::Number(value) => Kind::NumberValue(value.as_f64().unwrap_or_default()),
        serde_json::Value::String(value) => Kind::StringValue(value),
        serde_json::Value::Array(values) => Kind::ListValue(prost_types::ListValue {
            values: values.into_iter().map(encode_value).collect(),
        }),
        serde_json::Value::Object(value) => Kind::StructValue(encode_struct(value)),
    };
    prost_types::Value { kind: Some(kind) }
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
    encode_query_plan(plan)
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
