//! High-level reference renderer: Spanner plan nodes -> ASCII table +
//! appendix sections. Port of `plantree/reference/reference.go` +
//! `appendix.go`.
//!
//! Go exposes three entry points (`RenderTreeTable`,
//! `RenderTreeTableWithOptions`, `RenderTreeTableWithConfig`); the
//! functional-options variant collapses into [`RenderConfig`] here, so this
//! port has [`render_tree_table`] (wrap-width shortcut) and
//! [`render_tree_table_with_config`].
//!
//! Go's `PrintSection`/`PrintPreset` wrappers around the internal
//! scalarappendix types are 1:1; this port re-exports
//! [`crate::scalarappendix`]'s [`crate::scalarappendix::Section`]/[`crate::scalarappendix::Preset`] and parse functions
//! directly instead.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::asciitable::{self, Alignment, Column, TableSpec};
use crate::model::PlanNode;
use crate::plantree::{self, ProcessPlanOptions, RowWithPredicates};
use crate::queryplan::{
    has_stats, ExecutionMethodFormat, KnownFlagFormat, NodeTitleOptions, QueryPlan, QueryPlanError,
    TargetMetadataFormat,
};
use crate::scalarappendix::{self, ScalarAppendixError};

pub use crate::scalarappendix::{
    parse_preset as parse_print_preset, parse_section as parse_print_section,
    parse_sections as parse_print_sections, Preset as PrintPreset, Section as PrintSection,
};

/// How to render the query plan output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// Automatically determine whether to show statistics based on
    /// availability.
    Auto,
    /// Show only the query plan without statistics.
    Plan,
    /// Show the query plan with execution statistics.
    Profile,
}

/// Parses a string into a [`RenderMode`]. Valid values are `AUTO`, `PLAN`,
/// and `PROFILE` (case-insensitive).
pub fn parse_render_mode(s: &str) -> Result<RenderMode, RenderTreeTableError> {
    if s.eq_ignore_ascii_case("AUTO") {
        Ok(RenderMode::Auto)
    } else if s.eq_ignore_ascii_case("PLAN") {
        Ok(RenderMode::Plan)
    } else if s.eq_ignore_ascii_case("PROFILE") {
        Ok(RenderMode::Profile)
    } else {
        Err(RenderTreeTableError::UnknownRenderMode(s.to_string()))
    }
}

/// The formatting style for the query plan output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Raw metadata format in node titles.
    Traditional,
    /// Modern formatting with labels and angle brackets.
    Current,
    /// Compact tree rendering with minimal spacing.
    Compact,
}

/// Parses a string into a [`Format`]. Valid values are `TRADITIONAL`,
/// `CURRENT`, and `COMPACT` (case-insensitive).
pub fn parse_format(s: &str) -> Result<Format, RenderTreeTableError> {
    if s.eq_ignore_ascii_case("TRADITIONAL") {
        Ok(Format::Traditional)
    } else if s.eq_ignore_ascii_case("CURRENT") {
        Ok(Format::Current)
    } else if s.eq_ignore_ascii_case("COMPACT") {
        Ok(Format::Compact)
    } else {
        Err(RenderTreeTableError::UnknownFormat(s.to_string()))
    }
}

/// Optional rendering behavior, with serialization-friendly fields for
/// cross-language callers (WASM / FFI configs decode into this).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase", default))]
pub struct RenderConfig {
    /// Maximum total rendered line width, including the tree prefix. `0`
    /// disables wrapping; negative values make
    /// [`render_tree_table_with_config`] return an error.
    pub wrap_width: i32,
    /// Hang wrapped continuation lines after node-local prefixes such as
    /// `[Input] ` instead of the default tree-aligned indentation.
    pub hanging_indent: bool,
    /// Appendix sections printed after the rendered tree table. `None` uses
    /// the default (predicates); an explicit empty vec prints no appendix
    /// sections. (In JSON: omit or `null` for the default, `[]` for none.)
    pub print_sections: Option<Vec<PrintSection>>,
    /// Print scalar assignment variable names in semantic appendix sections.
    pub show_scalar_vars: bool,
    /// Replace direct scalar variable aliases in semantic appendix sections.
    pub resolve_scalar_vars: bool,
    /// Recursively resolve scalar variable aliases in semantic appendix
    /// sections.
    pub resolve_scalar_vars_recursive: bool,
    /// Fail on unknown execution-stat keys (Go `--disallow-unknown-stats`).
    pub disallow_unknown_stats: bool,
    /// When set, used instead of deriving [`ProcessPlanOptions`] from
    /// [`Format`] (the `rendertree` CLI passes per-flag options here).
    #[cfg_attr(feature = "serde", serde(skip))]
    pub process_plan_options: Option<ProcessPlanOptions>,
}

/// Errors from the render entry points and parse functions. `Display`
/// output matches the Go error strings the upstream tests assert on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderTreeTableError {
    EmptyPlanNodes,
    NegativeWrapWidth(i32),
    UnknownRenderMode(String),
    UnknownFormat(String),
    QueryPlan(QueryPlanError),
    Process(plantree::ProcessPlanError),
    Appendix(ScalarAppendixError),
    Table(asciitable::AsciiTableError),
}

impl core::fmt::Display for RenderTreeTableError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RenderTreeTableError::EmptyPlanNodes => f.write_str("planNodes cannot be empty"),
            RenderTreeTableError::NegativeWrapWidth(w) => {
                write!(f, "wrapWidth cannot be negative: {w}")
            }
            RenderTreeTableError::UnknownRenderMode(s) => write!(f, "unknown render mode: {s}"),
            RenderTreeTableError::UnknownFormat(s) => write!(f, "unknown format: {s}"),
            RenderTreeTableError::QueryPlan(e) => write!(f, "failed to create query plan: {e}"),
            RenderTreeTableError::Process(e) => write!(f, "{e}"),
            RenderTreeTableError::Appendix(e) => write!(f, "{e}"),
            RenderTreeTableError::Table(e) => write!(f, "{e}"),
        }
    }
}

/// Renders Spanner plan nodes as an ASCII table with the given mode, format,
/// and wrap width (`0` disables wrapping). Mirrors Go `RenderTreeTable`.
pub fn render_tree_table(
    plan_nodes: &[PlanNode],
    mode: RenderMode,
    format: Format,
    wrap_width: i32,
) -> Result<String, RenderTreeTableError> {
    render_tree_table_with_config(
        plan_nodes,
        mode,
        format,
        &RenderConfig {
            wrap_width,
            ..RenderConfig::default()
        },
    )
}

/// Renders Spanner plan nodes as an ASCII table using serialization-friendly
/// rendering configuration. Mirrors Go `RenderTreeTableWithConfig` (and the
/// functional-options variant, which collapses into [`RenderConfig`] here).
pub fn render_tree_table_with_config(
    plan_nodes: &[PlanNode],
    mode: RenderMode,
    format: Format,
    config: &RenderConfig,
) -> Result<String, RenderTreeTableError> {
    if plan_nodes.is_empty() {
        return Err(RenderTreeTableError::EmptyPlanNodes);
    }
    if config.wrap_width < 0 {
        return Err(RenderTreeTableError::NegativeWrapWidth(config.wrap_width));
    }

    let with_stats = match mode {
        RenderMode::Auto => has_stats(plan_nodes),
        RenderMode::Plan => false,
        RenderMode::Profile => true,
    };

    let rendered = process_tree(plan_nodes, format, config)?;

    let table_part = render_table_part(&rendered, with_stats)?;

    let appendix_part = scalarappendix::render(
        &rendered,
        &scalarappendix::Options {
            sections: config.print_sections.clone(),
            show_scalar_vars: config.show_scalar_vars,
            resolve_scalar_vars: config.resolve_scalar_vars,
            resolve_scalar_vars_recursive: config.resolve_scalar_vars_recursive,
        },
    )
    .map_err(RenderTreeTableError::Appendix)?;

    Ok(format!("{table_part}{appendix_part}"))
}

/// Converts Spanner plan nodes into rendered rows for the given format.
fn process_tree(
    plan_nodes: &[PlanNode],
    format: Format,
    config: &RenderConfig,
) -> Result<Vec<RowWithPredicates>, RenderTreeTableError> {
    let qp = QueryPlan::new(plan_nodes.to_vec()).map_err(RenderTreeTableError::QueryPlan)?;

    let mut opts = match &config.process_plan_options {
        Some(o) => o.clone(),
        None => opts_for_format(format),
    };
    if config.disallow_unknown_stats {
        opts.disallow_unknown_stats = true;
    }
    if config.wrap_width > 0 {
        opts = opts.with_wrap_width(config.wrap_width);
    }
    if config.hanging_indent {
        opts = opts.with_hanging_indent();
    }

    plantree::process_plan(&qp, &opts).map_err(RenderTreeTableError::Process)
}

/// The rendering options for the given format.
fn opts_for_format(format: Format) -> ProcessPlanOptions {
    let current = ProcessPlanOptions::default().with_query_plan_options(
        NodeTitleOptions::default()
            .with_known_flag_format(KnownFlagFormat::Label)
            .with_execution_method_format(ExecutionMethodFormat::Angle)
            .with_target_metadata_format(TargetMetadataFormat::On),
    );
    match format {
        Format::Traditional => ProcessPlanOptions::default(),
        Format::Current => current,
        Format::Compact => current.enable_compact(),
    }
}

fn render_table_part(
    rendered: &[RowWithPredicates],
    with_stats: bool,
) -> Result<String, RenderTreeTableError> {
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

    let mut columns = alloc::vec![id_col, operator_col];
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
            header: "Total Latency".to_string(),
            alignment: Alignment::Left,
            cell: &|row, _| row.execution_stats.latency.to_string(),
        });
    }

    asciitable::render_table(rendered, &TableSpec { columns }).map_err(RenderTreeTableError::Table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChildLink, Kind, ShortRepresentation};
    use alloc::vec;

    fn relational(index: i32, display_name: &str, child_links: Vec<ChildLink>) -> PlanNode {
        PlanNode {
            index,
            kind: Kind::Relational,
            display_name: display_name.to_string(),
            child_links,
            ..PlanNode::default()
        }
    }

    fn scalar_node(index: i32, description: &str) -> PlanNode {
        PlanNode {
            index,
            kind: Kind::Scalar,
            display_name: "Reference".to_string(),
            short_representation: Some(ShortRepresentation {
                description: description.to_string(),
            }),
            ..PlanNode::default()
        }
    }

    fn var_link(child_index: i32, r#type: &str, variable: &str) -> ChildLink {
        ChildLink {
            child_index,
            r#type: r#type.to_string(),
            variable: variable.to_string(),
        }
    }

    fn link(child_index: i32) -> ChildLink {
        ChildLink {
            child_index,
            ..ChildLink::default()
        }
    }

    /// The synthetic plan from Go `reference_test.go`
    /// `scalarAppendixPlanNodes()`.
    fn scalar_appendix_plan_nodes() -> Vec<PlanNode> {
        vec![
            relational(
                0,
                "Sort",
                vec![
                    var_link(1, "Key", "sort_count"),
                    var_link(2, "Key", "sort_genre"),
                    link(3),
                ],
            ),
            scalar_node(1, "$SongCount (DESC)"),
            scalar_node(2, "$group_SongGenre'"),
            relational(
                3,
                "Aggregate",
                vec![
                    var_link(4, "Key", "group_SongGenre'"),
                    var_link(5, "Agg", "SongCount"),
                    link(6),
                ],
            ),
            scalar_node(4, "$group_SongGenre"),
            scalar_node(5, "COUNT_FINAL($v1)"),
            relational(
                6,
                "Scan",
                vec![
                    var_link(7, "", "group_SongGenre"),
                    var_link(8, "", "SongGenre"),
                    var_link(9, "", "v1"),
                ],
            ),
            scalar_node(7, "$SongGenre"),
            scalar_node(8, "SongGenre"),
            scalar_node(9, "COUNT()"),
        ]
    }

    #[test]
    fn parse_render_mode_table() {
        assert_eq!(parse_render_mode("AUTO").unwrap(), RenderMode::Auto);
        assert_eq!(parse_render_mode("auto").unwrap(), RenderMode::Auto);
        assert_eq!(parse_render_mode("PLAN").unwrap(), RenderMode::Plan);
        assert_eq!(parse_render_mode("PROFILE").unwrap(), RenderMode::Profile);
        assert_eq!(parse_render_mode("pRoFiLe").unwrap(), RenderMode::Profile);
        for bad in ["INVALID", ""] {
            let err = parse_render_mode(bad).unwrap_err().to_string();
            assert!(err.contains("unknown render mode"), "error: {err}");
        }
    }

    #[test]
    fn parse_format_table() {
        assert_eq!(parse_format("TRADITIONAL").unwrap(), Format::Traditional);
        assert_eq!(parse_format("traditional").unwrap(), Format::Traditional);
        assert_eq!(parse_format("CURRENT").unwrap(), Format::Current);
        assert_eq!(parse_format("COMPACT").unwrap(), Format::Compact);
        assert_eq!(parse_format("CoMpAcT").unwrap(), Format::Compact);
        for bad in ["INVALID", ""] {
            let err = parse_format(bad).unwrap_err().to_string();
            assert!(err.contains("unknown format"), "error: {err}");
        }
    }

    #[test]
    fn input_validation() {
        let err = render_tree_table(&[], RenderMode::Auto, Format::Current, 0).unwrap_err();
        assert_eq!(err.to_string(), "planNodes cannot be empty");

        let err = render_tree_table(
            &[PlanNode::default()],
            RenderMode::Auto,
            Format::Current,
            -1,
        )
        .unwrap_err();
        assert!(err.to_string().contains("wrapWidth cannot be negative"));
    }

    #[test]
    fn print_sections_semantic_resolution() {
        // Ports TestRenderTreeTableWithOptions_PrintSections.
        let got = render_tree_table_with_config(
            &scalar_appendix_plan_nodes(),
            RenderMode::Plan,
            Format::Current,
            &RenderConfig {
                print_sections: Some(vec![PrintSection::Ordering, PrintSection::Aggregate]),
                resolve_scalar_vars_recursive: true,
                ..RenderConfig::default()
            },
        )
        .unwrap();
        for want in [
            "Ordering(identified by ID):",
            " 0: Key: COUNT_FINAL(COUNT()) DESC, SongGenre",
            "Aggregates(identified by ID):",
            " 3: Key: SongGenre",
            "    Agg: COUNT_FINAL($v1)",
        ] {
            assert!(got.contains(want), "missing {want:?} in:\n{got}");
        }
        assert!(!got.contains("Predicates(identified by ID):"), "{got}");
    }

    #[test]
    fn print_sections_show_vars_direct_resolution() {
        // Ports TestRenderTreeTableWithConfig_PrintSections.
        let got = render_tree_table_with_config(
            &scalar_appendix_plan_nodes(),
            RenderMode::Plan,
            Format::Current,
            &RenderConfig {
                print_sections: Some(vec![PrintSection::Ordering, PrintSection::Aggregate]),
                show_scalar_vars: true,
                resolve_scalar_vars: true,
                ..RenderConfig::default()
            },
        )
        .unwrap();
        for want in [
            "Ordering(identified by ID):",
            " 0: Key: $sort_count=COUNT_FINAL($v1) DESC, $sort_genre=$group_SongGenre",
            "Aggregates(identified by ID):",
            " 3: Key: $group_SongGenre'=$SongGenre",
            "    Agg: $SongCount=COUNT_FINAL($v1)",
        ] {
            assert!(got.contains(want), "missing {want:?} in:\n{got}");
        }
    }

    #[test]
    fn print_sections_raw_dumps() {
        // Ports TestRenderTreeTableWithOptions_RawPrintSections.
        let full = render_tree_table_with_config(
            &scalar_appendix_plan_nodes(),
            RenderMode::Plan,
            Format::Current,
            &RenderConfig {
                print_sections: Some(vec![PrintSection::Full]),
                ..RenderConfig::default()
            },
        )
        .unwrap();
        for want in [
            "Node Parameters(identified by ID):",
            " 0: Key: $sort_count=$SongCount (DESC), $sort_genre=$group_SongGenre'",
            " 3: Key: $group_SongGenre'=$group_SongGenre",
            "    Agg: $SongCount=COUNT_FINAL($v1)",
            " 6: $group_SongGenre=$SongGenre, $SongGenre=SongGenre, $v1=COUNT()",
        ] {
            assert!(full.contains(want), "missing {want:?} in:\n{full}");
        }

        let typed = render_tree_table_with_config(
            &scalar_appendix_plan_nodes(),
            RenderMode::Plan,
            Format::Current,
            &RenderConfig {
                print_sections: Some(vec![PrintSection::Typed]),
                ..RenderConfig::default()
            },
        )
        .unwrap();
        assert!(!typed.contains("$group_SongGenre=$SongGenre"), "{typed}");
    }

    #[test]
    fn print_section_validation_and_explicit_empty() {
        // Ports TestRenderTreeTableWithOptions_PrintSectionValidation.
        let err = render_tree_table_with_config(
            &scalar_appendix_plan_nodes(),
            RenderMode::Plan,
            Format::Current,
            &RenderConfig {
                print_sections: Some(vec![PrintSection::Predicates, PrintSection::Full]),
                ..RenderConfig::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            "print section \"full\" cannot be combined with other sections"
        );

        let got = render_tree_table_with_config(
            &scalar_appendix_plan_nodes(),
            RenderMode::Plan,
            Format::Current,
            &RenderConfig {
                print_sections: Some(vec![]),
                ..RenderConfig::default()
            },
        )
        .unwrap();
        assert!(!got.contains("identified by ID"), "{got}");
    }

    #[test]
    fn full_text_search_predicate() {
        // Ports TestRenderTreeTable_FullTextSearchPredicate.
        let plan_nodes = vec![
            PlanNode {
                index: 0,
                kind: Kind::Relational,
                display_name: "Scan".to_string(),
                child_links: vec![ChildLink {
                    child_index: 1,
                    r#type: "Search Predicate".to_string(),
                    variable: String::new(),
                }],
                ..PlanNode::default()
            },
            PlanNode {
                index: 1,
                kind: Kind::Scalar,
                display_name: "Search Predicate".to_string(),
                short_representation: Some(ShortRepresentation {
                    description: "SEARCH(Tokens, 'blue')".to_string(),
                }),
                ..PlanNode::default()
            },
        ];
        let got = render_tree_table(&plan_nodes, RenderMode::Plan, Format::Current, 0).unwrap();
        for want in [
            "Predicates(identified by ID):",
            " 0: Search Predicate: SEARCH(Tokens, 'blue')",
        ] {
            assert!(got.contains(want), "missing {want:?} in:\n{got}");
        }
    }
}
