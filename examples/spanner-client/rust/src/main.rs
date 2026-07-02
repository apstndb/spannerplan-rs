//! Fetch QueryPlan via google-cloud-spanner (gRPC) and render with native spannerplan.

use std::env;
use std::fs;
use std::path::PathBuf;

use google_cloud_gax::conn::ConnectionOptions;
use google_cloud_googleapis::spanner::v1::execute_sql_request::QueryMode;
use google_cloud_googleapis::spanner::v1::{CreateSessionRequest, ExecuteSqlRequest};
use google_cloud_spanner::apiv1::conn_pool::ConnectionManager;
use google_cloud_spanner::apiv1::spanner_client::Client as SpannerGrpcClient;
use google_cloud_spanner::client::ClientConfig;
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

fn load_sql(query: Option<&str>, query_file: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
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
    let instance = instance.ok_or("missing required value: set --instance or SPANNER_INSTANCE_ID")?;
    let database = database.ok_or("missing required value: set --database or SPANNER_DATABASE_ID")?;
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

async fn fetch_query_plan_wire(
    database: &str,
    sql: &str,
    query_mode: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let config = ClientConfig::default().with_auth().await?;
    let conn_options = ConnectionOptions {
        timeout: Some(config.channel_config.timeout),
        connect_timeout: Some(config.channel_config.connect_timeout),
    };
    let cm = ConnectionManager::new(
        config.channel_config.num_channels,
        &config.environment,
        &config.endpoint,
        &conn_options,
    )
    .await?;
    let mut client: SpannerGrpcClient = cm.conn();

    let session = client
        .create_session(
            CreateSessionRequest {
                database: database.to_string(),
                session: None,
            },
            None,
        )
        .await?
        .into_inner();

    let result = client
        .execute_sql(
            ExecuteSqlRequest {
                session: session.name,
                transaction: None,
                sql: sql.to_string(),
                params: None,
                param_types: Default::default(),
                resume_token: vec![],
                query_mode: spanner_query_mode(query_mode).into(),
                partition_token: vec![],
                seqno: 0,
                query_options: None,
                request_options: None,
                data_boost_enabled: false,
                directed_read_options: None,
            },
            None,
        )
        .await?
        .into_inner();

    let plan = result
        .stats
        .and_then(|stats| stats.query_plan)
        .ok_or_else(|| format!("QueryPlan missing from {query_mode}-mode execute_sql response"))?;
    Ok(plan.encode_to_vec())
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
