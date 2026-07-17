//! `rendertree` CLI. Port of `cmd/rendertree/impl/impl.go` (minus custom
//! columns / inline-stats — deferred per `DESIGN.md` non-goals).

use std::io::{Read, Write};

use spannerplan::core::asciitable::{self, Alignment, Column, TableSpec};
use spannerplan::core::plantree::{self, ProcessPlanOptions, RowWithPredicates};
use spannerplan::core::queryplan::{
    has_stats, parse_execution_method_format, parse_known_flag_format,
    parse_target_metadata_format, ExecutionMethodFormat, KnownFlagFormat, NodeTitleOptions,
    QueryPlan, TargetMetadataFormat,
};
use spannerplan::core::reference::{parse_print_sections, parse_render_mode, RenderMode};
use spannerplan::core::scalarappendix;
use spannerplan::extract::extract_plan_nodes;

/// Usage / flag-validation failures exit with code 2, matching Go `usageError`.
#[derive(Debug)]
pub struct UsageError {
    message: String,
}

impl UsageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for UsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for UsageError {}

/// Outcome of [`run_collecting`]: rendered stdout/stderr, or help text on stderr only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunCollectResult {
    Rendered { stdout: String, stderr: String },
    Help { stderr: String },
}

/// Errors from [`run_collecting`]. Usage failures correspond to exit code 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunCollectError {
    Usage { stderr: String, message: String },
    Failed { stderr: String, message: String },
}

impl std::fmt::Display for RunCollectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunCollectError::Usage { message, .. } | RunCollectError::Failed { message, .. } => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for RunCollectError {}

/// Whether [`run`] finished after printing help or rendering output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Help,
    Rendered,
}

/// Like [`run`], but takes input bytes and returns captured stdout/stderr for
/// bindings (WASM / JS CLI).
pub fn run_collecting(args: &[&str], input: &[u8]) -> Result<RunCollectResult, RunCollectError> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    match run(args, input, &mut stdout, &mut stderr) {
        Ok(RunStatus::Help) => Ok(RunCollectResult::Help {
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        }),
        Ok(RunStatus::Rendered) => Ok(RunCollectResult::Rendered {
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        }),
        Err(err) if err.downcast_ref::<UsageError>().is_some() => {
            let message = err.to_string();
            Err(RunCollectError::Usage {
                stderr: String::from_utf8_lossy(&stderr).into_owned(),
                message,
            })
        }
        Err(err) => Err(RunCollectError::Failed {
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            message: err.to_string(),
        }),
    }
}

pub fn run<I, O, E>(
    args: &[&str],
    mut stdin: I,
    stdout: &mut O,
    stderr: &mut E,
) -> Result<RunStatus, Box<dyn std::error::Error>>
where
    I: Read,
    O: Write,
    E: Write,
{
    let parsed = match parse_args(args, stderr) {
        Ok(Some(p)) => p,
        Ok(None) => return Ok(RunStatus::Help),
        Err(e) => return Err(Box::new(e)),
    };

    let mut input = Vec::new();
    stdin.read_to_end(&mut input)?;

    let plan_nodes = match extract_plan_nodes(&input) {
        Ok(nodes) => nodes,
        Err(err) => {
            const JSON_SNIPPET_LEN: usize = 140;
            let collapsed = if input.len() > JSON_SNIPPET_LEN {
                "(collapsed)"
            } else {
                ""
            };
            let snippet = String::from_utf8_lossy(&input[..input.len().min(JSON_SNIPPET_LEN)]);
            return Err(format!(
                "invalid input at protoyaml.Unmarshal:\nerror: {err}\ninput: {}{collapsed}",
                snippet.trim()
            )
            .into());
        }
    };

    let node_title = NodeTitleOptions::default()
        .with_execution_method_format(parsed.execution_method)
        .with_target_metadata_format(parsed.target_metadata)
        .with_known_flag_format(parsed.known_flag);

    let mut process_opts = ProcessPlanOptions::default().with_query_plan_options(node_title);
    if parsed.compact {
        process_opts = process_opts.enable_compact();
    }
    if parsed.disallow_unknown_stats {
        process_opts = process_opts.disallow_unknown_stats();
    }
    if parsed.wrap_width > 0 {
        process_opts = process_opts.with_wrap_width(parsed.wrap_width);
    }
    if parsed.hanging_indent {
        process_opts = process_opts.with_hanging_indent();
    }

    let qp = QueryPlan::new(plan_nodes).map_err(|e| e.to_string())?;
    let rows = plantree::process_plan(&qp, &process_opts).map_err(|e| e.to_string())?;

    let with_stats = match parsed.mode {
        RenderMode::Plan => false,
        RenderMode::Profile => true,
        RenderMode::Auto => has_stats(qp.plan_nodes()),
    };

    let mut output = render_cli_table(&rows, with_stats).map_err(|e| e.to_string())?;
    let appendix = scalarappendix::render(
        &rows,
        &scalarappendix::Options {
            sections: parsed.print_sections,
            show_scalar_vars: parsed.show_scalar_vars,
            resolve_scalar_vars: parsed.resolve_scalar_vars,
            resolve_scalar_vars_recursive: parsed.resolve_scalar_vars_recursive,
        },
    )
    .map_err(|e| e.to_string())?;
    if !appendix.is_empty() {
        output.push('\n');
        output.push_str(&appendix);
    }

    stdout.write_all(output.as_bytes())?;
    Ok(RunStatus::Rendered)
}

/// Go `impl.withStatsToRenderDefMap` + `secsToS`: the CLI uses a `Latency`
/// column (not `Total Latency`) and rewrites trailing `secs` in the unit to
/// `s` (so `msecs` becomes `ms`).
fn render_cli_table(
    rows: &[RowWithPredicates],
    with_stats: bool,
) -> Result<String, asciitable::AsciiTableError> {
    let id_col: Column<'_, RowWithPredicates> = Column {
        header: "ID".to_string(),
        alignment: Alignment::Right,
        cell: &|row, _| row.format_id(),
    };
    let operator_col: Column<'_, RowWithPredicates> = Column {
        header: "Operator".to_string(),
        alignment: Alignment::Left,
        cell: &|row, _| row.text(),
    };

    let mut columns: Vec<Column<'_, RowWithPredicates>> = vec![id_col, operator_col];
    if with_stats {
        columns.push(Column {
            header: "Rows".to_string(),
            alignment: Alignment::Right,
            cell: &|row, _| row.execution_stats.rows.total.clone(),
        });
        columns.push(Column {
            header: "Exec.".to_string(),
            alignment: Alignment::Right,
            cell: &|row, _| row.execution_stats.execution_summary.num_executions.clone(),
        });
        columns.push(Column {
            header: "Latency".to_string(),
            alignment: Alignment::Right,
            cell: &|row, _| secs_to_s(&row.execution_stats.latency),
        });
    }

    asciitable::render_table(rows, &TableSpec { columns })
}

fn secs_to_s(latency: &spannerplan::core::stats::ExecutionStatsValue) -> String {
    let s = latency.to_string();
    if let Some(prefix) = s.strip_suffix("secs") {
        format!("{prefix}s")
    } else {
        s
    }
}

struct ParsedArgs {
    mode: RenderMode,
    print_sections: Option<Vec<spannerplan::core::reference::PrintSection>>,
    show_scalar_vars: bool,
    resolve_scalar_vars: bool,
    resolve_scalar_vars_recursive: bool,
    disallow_unknown_stats: bool,
    execution_method: ExecutionMethodFormat,
    target_metadata: TargetMetadataFormat,
    known_flag: KnownFlagFormat,
    compact: bool,
    wrap_width: i32,
    hanging_indent: bool,
}

fn parse_args<E: Write>(args: &[&str], stderr: &mut E) -> Result<Option<ParsedArgs>, UsageError> {
    let mut mode = "AUTO".to_string();
    let mut print = "basic".to_string();
    let mut show_scalar_vars = false;
    let mut resolve_scalar_vars = false;
    let mut resolve_scalar_vars_recursive = false;
    let mut disallow_unknown_stats = false;
    let mut execution_method = "angle".to_string();
    let mut target_metadata = "on".to_string();
    let mut known_flag = String::new();
    let mut compact = false;
    let mut wrap_width: i32 = 0;
    let mut hanging_indent = false;
    let mut custom_column: Vec<String> = Vec::new();
    let mut custom_file = String::new();

    let mut i = 0;
    while i < args.len() {
        let (flag, value) = split_flag(args[i]);
        match flag {
            "-h" | "-help" | "--help" => {
                print_usage(stderr);
                return Ok(None);
            }
            "-mode" | "--mode" => {
                mode = value_or_next(value, args, &mut i, "-mode")?;
            }
            "-print" | "--print" => {
                print = value_or_next(value, args, &mut i, "-print")?;
            }
            "-show-vars" | "--show-vars" => show_scalar_vars = parse_bool_flag(value)?,
            "-resolve-vars" | "--resolve-vars" => resolve_scalar_vars = parse_bool_flag(value)?,
            "-resolve-vars-recursive" | "--resolve-vars-recursive" => {
                resolve_scalar_vars_recursive = parse_bool_flag(value)?
            }
            "-disallow-unknown-stats" | "--disallow-unknown-stats" => {
                disallow_unknown_stats = parse_bool_flag(value)?
            }
            "-execution-method" | "--execution-method" => {
                execution_method = value_or_next(value, args, &mut i, "-execution-method")?;
            }
            "-target-metadata" | "--target-metadata" => {
                target_metadata = value_or_next(value, args, &mut i, "-target-metadata")?;
            }
            "-known-flag" | "--known-flag" => {
                known_flag = value_or_next(value, args, &mut i, "-known-flag")?;
            }
            "-compact" | "--compact" => compact = parse_bool_flag(value)?,
            "-inline-stats" | "--inline-stats" => {
                let _ = parse_bool_flag(value)?;
                return Err(usage_error(
                    "--inline-stats is not implemented in spannerplan-rs (see DESIGN.md §12)",
                ));
            }
            "-wrap-width" | "--wrap-width" => {
                let s = value_or_next(value, args, &mut i, "-wrap-width")?;
                wrap_width = s.parse::<i32>().map_err(|_| {
                    let msg = format!("invalid int value {s:?} for -wrap-width");
                    usage_error(msg)
                })?;
            }
            "-hanging-indent" | "--hanging-indent" => hanging_indent = parse_bool_flag(value)?,
            "-custom-column" | "--custom-column" => {
                custom_column.push(value_or_next(value, args, &mut i, "-custom-column")?);
            }
            "-custom-file" | "--custom-file" => {
                custom_file = value_or_next(value, args, &mut i, "-custom-file")?;
            }
            other if other.starts_with('-') => {
                let msg = format!("flag provided but not defined: {other}");
                writeln!(stderr, "{msg}").ok();
                print_usage(stderr);
                return Err(usage_error(msg));
            }
            _ => {
                let msg = format!("unexpected argument: {flag}");
                writeln!(stderr, "{msg}").ok();
                return Err(usage_error(msg));
            }
        }
        i += 1;
    }

    if !custom_column.is_empty() && !custom_file.is_empty() {
        let msg = "--custom-column and --custom-file are mutually exclusive";
        writeln!(stderr, "{msg}").ok();
        print_usage(stderr);
        return Err(UsageError::new(msg));
    }
    if !custom_column.is_empty() || !custom_file.is_empty() {
        return Err(UsageError::new(
            "custom table columns are not yet implemented in spannerplan-rs (see DESIGN.md §12)",
        ));
    }

    let parsed_mode = match parse_render_mode_cli(&mode) {
        Ok(m) => m,
        Err(e) => {
            writeln!(stderr, "Invalid value for -mode flag: {e}.").ok();
            print_usage(stderr);
            return Err(UsageError::new(e));
        }
    };

    let print_sections = match parse_print_sections(&print) {
        Ok(sections) => Some(sections),
        Err(e) => {
            writeln!(stderr, "Invalid value for -print flag: {e}").ok();
            print_usage(stderr);
            return Err(UsageError::new(e.to_string()));
        }
    };

    let em = match parse_execution_method_format(&execution_method) {
        Ok(v) => v,
        Err(e) => {
            writeln!(stderr, "Invalid value for -execution-method flag: {e}.").ok();
            print_usage(stderr);
            return Err(UsageError::new(e.to_string()));
        }
    };

    let tm = match parse_target_metadata_format(&target_metadata) {
        Ok(v) => v,
        Err(e) => {
            writeln!(stderr, "Invalid value for -target-metadata flag: {e}.").ok();
            print_usage(stderr);
            return Err(UsageError::new(e.to_string()));
        }
    };

    let kf = if known_flag.is_empty() {
        KnownFlagFormat::Label
    } else {
        match parse_known_flag_format(&known_flag) {
            Ok(v) => v,
            Err(e) => {
                writeln!(stderr, "Invalid value for -known-flag: {e}.").ok();
                print_usage(stderr);
                return Err(UsageError::new(e.to_string()));
            }
        }
    };

    Ok(Some(ParsedArgs {
        mode: parsed_mode,
        print_sections,
        show_scalar_vars,
        resolve_scalar_vars,
        resolve_scalar_vars_recursive,
        disallow_unknown_stats,
        execution_method: em,
        target_metadata: tm,
        known_flag: kf,
        compact,
        wrap_width,
        hanging_indent,
    }))
}

fn split_flag(arg: &str) -> (&str, Option<&str>) {
    if let Some((flag, value)) = arg.split_once('=') {
        (flag, Some(value))
    } else {
        (arg, None)
    }
}

fn value_or_next(
    inline: Option<&str>,
    args: &[&str],
    i: &mut usize,
    flag: &str,
) -> Result<String, UsageError> {
    if let Some(v) = inline {
        Ok(v.to_string())
    } else {
        next_value(args, i, flag)
    }
}

fn parse_bool_flag(value: Option<&str>) -> Result<bool, UsageError> {
    match value {
        None => Ok(true),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(v) => Err(usage_error(format!("invalid boolean value {v:?}"))),
    }
}

fn parse_render_mode_cli(s: &str) -> Result<RenderMode, String> {
    parse_render_mode(s).map_err(|_| {
        format!("invalid input: {s}. Must be one of AUTO, PLAN, PROFILE (case-insensitive)")
    })
}

fn next_value(args: &[&str], i: &mut usize, flag: &str) -> Result<String, UsageError> {
    *i += 1;
    args.get(*i)
        .map(|s| s.to_string())
        .ok_or_else(|| usage_error(format!("flag needs an argument: {flag}")))
}

fn usage_error(message: impl Into<String>) -> UsageError {
    UsageError::new(message)
}

fn print_usage<E: Write>(stderr: &mut E) {
    writeln!(stderr, "Usage of rendertree:").ok();
    writeln!(
        stderr,
        "  -compact\n    \tEnable compact format\n  -disallow-unknown-stats\n    \terror on unknown stats field\n  -execution-method string\n    \tFormat execution method metadata: 'angle' or 'raw' (default: angle)\n  -hanging-indent\n    \tEnable hanging indent for wrapped lines after node-local prefixes such as [Input] and [Map]\n  -help\n    \tShow this help message\n  -known-flag string\n    \tFormat known flags: 'label' or 'raw' (default: label)\n  -mode string\n    \tPROFILE, PLAN, AUTO(ignore case) (default \"AUTO\")\n  -print string\n    \tprint appendix preset (basic, enhanced, full, none; empty value suppresses appendices) or comma-separated sections (predicates, ordering, aggregate, typed, full); presets are standalone; typed/full cannot be combined (default \"basic\")\n  -resolve-vars\n    \tEXPERIMENTAL: resolve scalar variable aliases in semantic appendix sections\n  -resolve-vars-recursive\n    \tEXPERIMENTAL: recursively resolve scalar variable aliases in semantic appendix sections\n  -show-vars\n    \tshow scalar variable assignments in semantic appendix sections\n  -target-metadata string\n    \tFormat target metadata: 'on' or 'raw' (default: on)\n  -wrap-width int\n    \tNumber of characters at which to wrap the Operator column content. 0 means no wrapping. (default 0)"
    )
    .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn testdata(rel: &str) -> String {
        format!("{}/../../testdata/{rel}", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn help_returns_ok_without_output() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(
            run(&["-h"], &[] as &[u8], &mut stdout, &mut stderr).unwrap(),
            super::RunStatus::Help
        );
        assert!(stdout.is_empty());
        assert!(String::from_utf8_lossy(&stderr).contains("-mode"));
    }

    #[test]
    fn usage_error_on_unknown_flag() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let err = run(&["-unknown"], &[] as &[u8], &mut stdout, &mut stderr).unwrap_err();
        assert!(err
            .to_string()
            .contains("flag provided but not defined: -unknown"));
        assert!(String::from_utf8_lossy(&stderr).contains("Usage of rendertree:"));
    }

    #[test]
    fn inline_stats_is_rejected_instead_of_ignored() {
        for args in [
            ["--inline-stats"].as_slice(),
            ["--inline-stats=true"].as_slice(),
            ["--inline-stats=false"].as_slice(),
        ] {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let err = run(args, &[] as &[u8], &mut stdout, &mut stderr).unwrap_err();
            assert!(err
                .to_string()
                .contains("--inline-stats is not implemented"));
            assert!(stdout.is_empty());
            assert!(stderr.is_empty());
        }

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        run(&["--help"], &[] as &[u8], &mut stdout, &mut stderr).unwrap();
        assert!(!String::from_utf8_lossy(&stderr).contains("-inline-stats"));
    }

    #[test]
    fn usage_error_on_unexpected_positional_writes_stderr() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let err = run(&["rendertree"], &[] as &[u8], &mut stdout, &mut stderr).unwrap_err();
        assert!(err.to_string().contains("unexpected argument"));
        assert!(!String::from_utf8_lossy(&stderr).is_empty());
    }

    #[test]
    fn empty_print_suppresses_appendix() {
        let input = std::fs::read(testdata("reference/dca.yaml")).unwrap();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        run(
            &["-print", "", "-mode", "plan"],
            &input[..],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();
        let out = String::from_utf8_lossy(&stdout);
        assert!(!out.contains("Predicates(identified by ID):"));
        assert!(out.contains("Distributed Cross Apply"));
    }

    #[test]
    fn matches_go_cli_on_dca_plan() {
        let input = std::fs::read(testdata("reference/dca.yaml")).unwrap();
        let got = run_cli(&["-mode", "plan"], &input);
        let Some(want) = go_cli_output(&["-mode", "plan"], &input) else {
            return;
        };
        assert_eq!(got, want);
    }

    #[test]
    fn matches_go_cli_on_dcaplan_profile() {
        let input = std::fs::read(testdata("reference/distributed_cross_apply.yaml")).unwrap();
        let got = run_cli(&["-mode", "profile"], &input);
        let Some(want) = go_cli_output(&["-mode", "profile"], &input) else {
            return;
        };
        assert_eq!(got, want);
    }

    #[test]
    fn wrap_and_hanging_indent_match_go_cli() {
        let input = std::fs::read(testdata("reference/dca.yaml")).unwrap();
        for width in [50, 80] {
            let width_s = width.to_string();
            let args = ["-mode", "plan", "-wrap-width", &width_s];
            let Some(want) = go_cli_output(&args, &input) else {
                return;
            };
            assert_eq!(run_cli(&args, &input), want);

            let args = ["-mode", "plan", "-wrap-width", &width_s, "-hanging-indent"];
            let Some(want) = go_cli_output(&args, &input) else {
                return;
            };
            assert_eq!(run_cli(&args, &input), want);
        }
    }

    fn run_cli(args: &[&str], input: &[u8]) -> String {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(
            run(args, input, &mut stdout, &mut stderr).unwrap(),
            super::RunStatus::Rendered
        );
        assert!(
            stderr.is_empty(),
            "stderr: {}",
            String::from_utf8_lossy(&stderr)
        );
        String::from_utf8_lossy(&stdout).into_owned()
    }

    fn go_cli_output(args: &[&str], input: &[u8]) -> Option<String> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let binary = match std::env::var_os("SPANNERPLAN_GO_RENDERTREE") {
            Some(binary) => binary,
            None if std::env::var("SPANNERPLAN_GO_PARITY").is_ok() => {
                panic!("SPANNERPLAN_GO_RENDERTREE must name the Go rendertree binary when SPANNERPLAN_GO_PARITY is set")
            }
            None => {
                eprintln!(
                    "note: skipping Go CLI parity test (SPANNERPLAN_GO_RENDERTREE is not set)"
                );
                return None;
            }
        };
        let mut child = match Command::new(&binary)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(err) => panic!(
                "failed to spawn Go rendertree at {}: {err}",
                std::path::Path::new(&binary).display()
            ),
        };
        child.stdin.take().unwrap().write_all(input).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "rendertree failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Some(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}
