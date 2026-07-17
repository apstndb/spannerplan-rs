//! `process_plan`: converts a [`QueryPlan`] into rendered tree rows with
//! predicate and execution metadata. Port of `plantree/plantree.go`. See
//! `DESIGN.md` §6.6.
//!
//! The deprecated Go fields `RowWithPredicates.Keys` / `.ChildLinks` (kept
//! there only for source compatibility) are not ported;
//! [`RowWithPredicates::scalar_child_links`] is their replacement.

use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::model::Kind;
use crate::queryplan::{node_title, NodeTitleOptions, QueryPlan};
use crate::stats::{self, ExecutionStats, StatsError};
use crate::treerender::{
    self, compact_style, default_style, ContinuationIndent, RenderOptions, WrapCondition,
};

/// Conservative first-alpha maximum visible parent-to-child edges in one
/// rendered Plantree path.
///
/// The root has depth zero, so a node at depth 257 is rejected. Keeping this
/// hard cap in the core protects native and WASM callers from stack exhaustion
/// on untrusted plans. It may be raised non-breakingly when real captures show
/// that a higher bound is needed.
pub const MAX_PLANTREE_DEPTH: usize = 256;

/// Conservative first-alpha maximum visible node occurrences materialized by
/// one Plantree traversal.
///
/// A shared DAG can legitimately repeat a PlanNode below different parents;
/// this bounds that expansion before it can exhaust a native process or a
/// browser tab. It may be raised non-breakingly when real captures show that a
/// higher bound is needed.
pub const MAX_PLANTREE_OCCURRENCES: usize = 4096;

/// One rendered plan row plus predicate and execution metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowWithPredicates {
    /// Viewer-internal occurrence identity derived from the visible traversal
    /// path. Unlike `id`, this remains unique when a DAG node is rendered
    /// through more than one parent link.
    pub row_id: String,
    /// The parent occurrence identity, or `None` for the root occurrence.
    pub parent_row_id: Option<String>,
    /// The Spanner PlanNode index for this row.
    pub id: i32,
    /// Everything rendered before `node_text` on each visual line: the ASCII
    /// tree prefix plus any continuation padding inserted by the renderer for
    /// wrapping / hanging indent. Newline-separated, aligned with
    /// `node_text`'s lines.
    pub tree_part: String,
    /// The rendered operator title, possibly split across visual lines.
    pub node_text: String,
    /// The raw Spanner PlanNode display name, before metadata is folded into
    /// `node_text`.
    pub display_name: String,
    /// Filter predicate text associated with this row.
    pub predicates: Vec<String>,
    /// Execution statistics associated with this row.
    pub execution_stats: ExecutionStats,
    /// This row's scalar child links in original PlanNode.ChildLinks order.
    pub scalar_child_links: Vec<ScalarChildLink>,
}

impl RowWithPredicates {
    /// The full rendered row text, with the tree prefix prepended to each
    /// node text line.
    pub fn text(&self) -> String {
        treerender::Row {
            tree_part: self.tree_part.clone(),
            node_text: self.node_text.clone(),
        }
        .text()
    }

    /// One tree prefix per visual line.
    pub fn tree_part_lines(&self) -> Vec<&str> {
        self.tree_part.split('\n').collect()
    }

    /// The display ID, prefixed with `*` when the row has predicates.
    pub fn format_id(&self) -> String {
        if self.predicates.is_empty() {
            self.id.to_string()
        } else {
            format!("*{}", self.id)
        }
    }
}

/// A scalar child link attached to a rendered plan row.
///
/// Keeps raw-ish child-link fields so callers can group links by the parent
/// row's `display_name` and the child-link `type`. The same `type` can have
/// different semantics under different parent operators, for example Sort
/// Key versus Aggregate Key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScalarChildLink {
    /// The ChildLink type, such as `"Condition"`, `"Key"`, or `"Agg"`.
    pub r#type: String,
    /// The ChildLink variable, when Spanner provides one.
    pub variable: String,
    /// The scalar child node's short-representation description.
    pub description: String,
    /// The scalar child node's raw PlanNode display name.
    pub display_name: String,
    /// The scalar child node's PlanNode index.
    pub child_index: i32,
    /// Whether this child link is a filter predicate according to
    /// [`QueryPlan::is_predicate`].
    pub is_predicate: bool,
}

/// Options for [`process_plan`]. A plain struct with builder methods rather
/// than Go's closure-based functional options; nil options / nil wrapper
/// fallbacks are unrepresentable here. The deprecated Go
/// `WithContinuationIndent` is not ported (use
/// [`ProcessPlanOptions::with_hanging_indent`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcessPlanOptions {
    /// Fail on unknown execution-stat keys (Go `DisallowUnknownStats`).
    pub disallow_unknown_stats: bool,
    /// Node-title formatting options forwarded to
    /// [`node_title`] (Go `WithQueryPlanOptions`).
    pub node_title_options: NodeTitleOptions,
    /// Compact node titles and compact tree style (Go `EnableCompact`).
    pub compact: bool,
    /// Hang wrapped continuation lines after a node-local prefix such as
    /// `[Input] ` (Go `WithHangingIndent`).
    pub hanging_indent: bool,
    /// Maximum total rendered line width, including the tree prefix.
    /// `Some(0)` (or `None`) disables wrapping; negative values make
    /// [`process_plan`] return an error (Go `WithWrapWidth`).
    pub wrap_width: Option<i32>,
}

impl ProcessPlanOptions {
    pub fn disallow_unknown_stats(mut self) -> Self {
        self.disallow_unknown_stats = true;
        self
    }

    pub fn with_query_plan_options(mut self, opts: NodeTitleOptions) -> Self {
        self.node_title_options = opts;
        self
    }

    pub fn enable_compact(mut self) -> Self {
        self.compact = true;
        self
    }

    pub fn with_hanging_indent(mut self) -> Self {
        self.hanging_indent = true;
        self
    }

    pub fn with_wrap_width(mut self, width: i32) -> Self {
        self.wrap_width = Some(width);
        self
    }
}

/// Errors from [`process_plan`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessPlanError {
    /// `wrap_width` was negative.
    NegativeWrapWidth(i32),
    /// Execution-stat extraction failed.
    Stats(StatsError),
    /// A relational node was reached twice on the current traversal path.
    Cycle(i32),
    /// A visible traversal path exceeded [`MAX_PLANTREE_DEPTH`].
    DepthLimitExceeded { limit: usize, node_index: i32 },
    /// Visible DAG expansion exceeded [`MAX_PLANTREE_OCCURRENCES`].
    OccurrenceLimitExceeded { limit: usize, node_index: i32 },
    /// An internal renderer invariant was violated (mirrors Go's defensive
    /// row-count / line-count consistency errors).
    Internal(&'static str),
}

impl core::fmt::Display for ProcessPlanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProcessPlanError::NegativeWrapWidth(w) => {
                write!(f, "wrap width cannot be negative: {w}")
            }
            ProcessPlanError::Stats(e) => write!(f, "failed to extract execution stats: {e}"),
            ProcessPlanError::Cycle(id) => {
                write!(f, "cycle detected at PlanNode index {id}")
            }
            ProcessPlanError::DepthLimitExceeded { limit, node_index } => {
                write!(
                    f,
                    "plan exceeds the renderer depth budget {limit} at PlanNode index {node_index}"
                )
            }
            ProcessPlanError::OccurrenceLimitExceeded { limit, node_index } => {
                write!(
                    f,
                    "plan exceeds the renderer occurrence budget {limit} at PlanNode index {node_index}"
                )
            }
            ProcessPlanError::Internal(msg) => write!(f, "internal invariant violated: {msg}"),
        }
    }
}

impl From<StatsError> for ProcessPlanError {
    fn from(e: StatsError) -> Self {
        ProcessPlanError::Stats(e)
    }
}

struct RenderedNode {
    row_id: String,
    parent_row_id: Option<String>,
    id: i32,
    continuation_anchor: String,
    node_text: String,
    display_name: String,
    predicates: Vec<String>,
    execution_stats: ExecutionStats,
    scalar_child_links: Vec<ScalarChildLink>,
    children: Vec<RenderedNode>,
}

#[derive(Clone, Copy)]
struct IncomingEdge<'a> {
    parent: Option<&'a crate::model::PlanNode>,
    raw_child_link_index: Option<usize>,
}

struct OccurrencePosition {
    row_id: String,
    parent_row_id: Option<String>,
    depth: usize,
}

#[derive(Default)]
struct TraversalState {
    ancestors: BTreeSet<i32>,
    occurrences: usize,
}

/// Converts a query plan into rendered tree rows with predicate and
/// execution metadata. Port of Go `ProcessPlan`.
pub fn process_plan(
    qp: &QueryPlan,
    opts: &ProcessPlanOptions,
) -> Result<Vec<RowWithPredicates>, ProcessPlanError> {
    let wrap_width = opts.wrap_width.unwrap_or(0);
    if wrap_width < 0 {
        return Err(ProcessPlanError::NegativeWrapWidth(wrap_width));
    }

    let mut title_opts = opts.node_title_options.clone();
    if opts.compact {
        title_opts.compact = true;
    }

    let Some(root) = build_rendered_tree(
        qp,
        IncomingEdge {
            parent: None,
            raw_child_link_index: None,
        },
        OccurrencePosition {
            row_id: "0".to_string(),
            parent_row_id: None,
            depth: 0,
        },
        opts,
        &title_opts,
        &mut TraversalState::default(),
    )?
    else {
        return Ok(Vec::new());
    };

    let style = if opts.compact {
        compact_style()
    } else {
        default_style()
    };

    let get_anchor = |n: &RenderedNode| n.continuation_anchor.clone();
    let render_rows = treerender::render_tree_with_options(
        Some(&root),
        &style,
        |n: &RenderedNode| n.node_text.as_str(),
        |n: &RenderedNode| n.children.as_slice(),
        &RenderOptions {
            get_continuation_anchor: Some(&get_anchor),
            wrap_width,
            wrap_condition: WrapCondition::default(),
            continuation_indent: if opts.hanging_indent {
                ContinuationIndent::Anchor
            } else {
                ContinuationIndent::Tree
            },
        },
    )
    // The anchor callback is always provided above, so the only treerender
    // error (missing anchor callback) cannot fire.
    .map_err(|_| ProcessPlanError::Internal("tree renderer rejected options"))?;

    let mut nodes = Vec::new();
    flatten_preorder(root, &mut nodes);
    if render_rows.len() != nodes.len() {
        return Err(ProcessPlanError::Internal("unexpected rendered row count"));
    }

    let mut result = Vec::with_capacity(nodes.len());
    for (node, row) in nodes.into_iter().zip(render_rows) {
        let got_lines = row.node_text.split('\n').count();
        let want_tree_lines = row.tree_part.split('\n').count();
        if got_lines != want_tree_lines {
            return Err(ProcessPlanError::Internal(
                "unexpected rendered row line count",
            ));
        }
        result.push(RowWithPredicates {
            row_id: node.row_id,
            parent_row_id: node.parent_row_id,
            id: node.id,
            tree_part: row.tree_part,
            node_text: row.node_text,
            display_name: node.display_name,
            predicates: node.predicates,
            execution_stats: node.execution_stats,
            scalar_child_links: node.scalar_child_links,
        });
    }
    Ok(result)
}

fn build_rendered_tree(
    qp: &QueryPlan,
    incoming: IncomingEdge<'_>,
    occurrence: OccurrencePosition,
    opts: &ProcessPlanOptions,
    title_opts: &NodeTitleOptions,
    state: &mut TraversalState,
) -> Result<Option<RenderedNode>, ProcessPlanError> {
    let link = match (incoming.parent, incoming.raw_child_link_index) {
        (Some(parent), Some(index)) => Some(&parent.get_child_links()[index]),
        (None, None) => None,
        // This is internal call-site state, not malformed user input.
        _ => return Err(ProcessPlanError::Internal("incomplete incoming edge")),
    };
    if !qp.is_visible(link) {
        return Ok(None);
    }

    let sep = if opts.compact { "" } else { " " };

    let node = qp.get_node_by_child_link(link);
    let node_index = node.get_index();
    if state.ancestors.contains(&node_index) {
        return Err(ProcessPlanError::Cycle(node_index));
    }
    if occurrence.depth > MAX_PLANTREE_DEPTH {
        return Err(ProcessPlanError::DepthLimitExceeded {
            limit: MAX_PLANTREE_DEPTH,
            node_index,
        });
    }
    if state.occurrences == MAX_PLANTREE_OCCURRENCES {
        return Err(ProcessPlanError::OccurrenceLimitExceeded {
            limit: MAX_PLANTREE_OCCURRENCES,
            node_index,
        });
    }
    state.ancestors.insert(node_index);
    state.occurrences += 1;
    let link_type = match (incoming.parent, incoming.raw_child_link_index) {
        (Some(parent), Some(index)) => qp.link_type_in_parent(parent, index),
        (None, None) => "",
        _ => unreachable!("incoming edge was checked above"),
    };
    let continuation_anchor = if link_type.is_empty() {
        String::new()
    } else {
        format!("[{link_type}]{sep}")
    };
    let node_text = format!("{continuation_anchor}{}", node_title(node, title_opts));

    let mut predicates = Vec::new();
    for cl in node.get_child_links() {
        if !qp.is_predicate(Some(cl)) {
            continue;
        }
        predicates.push(format!(
            "{}: {}",
            cl.get_type(),
            qp.get_node_by_child_link(Some(cl))
                .get_short_representation_description()
        ));
    }

    let mut scalar_child_links = Vec::new();
    for cl in node.get_child_links() {
        let child = qp.get_node_by_child_link(Some(cl));
        if child.get_kind() != Kind::Scalar {
            continue;
        }
        scalar_child_links.push(ScalarChildLink {
            r#type: cl.get_type().to_string(),
            variable: cl.get_variable().to_string(),
            description: child.get_short_representation_description().to_string(),
            display_name: child.get_display_name().to_string(),
            child_index: child.get_index(),
            is_predicate: qp.is_predicate(Some(cl)),
        });
    }

    let execution_stats = stats::extract(node, opts.disallow_unknown_stats)?;

    let mut children = Vec::new();
    let mut visible_child_index = 0;
    for (raw_child_link_index, child_link) in node.get_child_links().iter().enumerate() {
        if !qp.is_visible(Some(child_link)) {
            continue;
        }
        let child_row_id = format!("{}.{visible_child_index}", occurrence.row_id);
        visible_child_index += 1;
        if let Some(rendered) = build_rendered_tree(
            qp,
            IncomingEdge {
                parent: Some(node),
                raw_child_link_index: Some(raw_child_link_index),
            },
            OccurrencePosition {
                row_id: child_row_id,
                parent_row_id: Some(occurrence.row_id.clone()),
                depth: occurrence.depth + 1,
            },
            opts,
            title_opts,
            state,
        )? {
            children.push(rendered);
        }
    }

    state.ancestors.remove(&node_index);

    Ok(Some(RenderedNode {
        row_id: occurrence.row_id,
        parent_row_id: occurrence.parent_row_id,
        id: node.get_index(),
        continuation_anchor,
        node_text,
        display_name: node.get_display_name().to_string(),
        predicates,
        execution_stats,
        scalar_child_links,
        children,
    }))
}

fn flatten_preorder(mut node: RenderedNode, out: &mut Vec<RenderedNode>) {
    let children = core::mem::take(&mut node.children);
    out.push(node);
    for child in children {
        flatten_preorder(child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChildLink, Metadata, MetadataValue, PlanNode, ShortRepresentation};
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

    fn scalar(index: i32, display_name: &str, description: &str) -> PlanNode {
        PlanNode {
            index,
            kind: Kind::Scalar,
            display_name: display_name.to_string(),
            short_representation: Some(ShortRepresentation {
                description: description.to_string(),
            }),
            ..PlanNode::default()
        }
    }

    fn scalar_with_links(
        index: i32,
        display_name: &str,
        description: &str,
        child_links: Vec<ChildLink>,
    ) -> PlanNode {
        PlanNode {
            child_links,
            ..scalar(index, display_name, description)
        }
    }

    fn link(child_index: i32, r#type: &str) -> ChildLink {
        ChildLink {
            child_index,
            r#type: r#type.to_string(),
            variable: String::new(),
        }
    }

    fn var_link(child_index: i32, r#type: &str, variable: &str) -> ChildLink {
        ChildLink {
            child_index,
            r#type: r#type.to_string(),
            variable: variable.to_string(),
        }
    }

    fn row_by_id(rows: &[RowWithPredicates], id: i32) -> &RowWithPredicates {
        rows.iter().find(|r| r.id == id).expect("row not found")
    }

    #[test]
    fn invisible_root_returns_empty() {
        let qp = QueryPlan::new(vec![PlanNode {
            index: 0,
            kind: Kind::Scalar,
            display_name: "Scalar Root".to_string(),
            ..PlanNode::default()
        }])
        .unwrap();
        let rows = process_plan(&qp, &ProcessPlanOptions::default()).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn negative_wrap_width_errors() {
        let qp = QueryPlan::new(vec![relational(0, "Scan", vec![])]).unwrap();
        let err =
            process_plan(&qp, &ProcessPlanOptions::default().with_wrap_width(-1)).unwrap_err();
        assert_eq!(err, ProcessPlanError::NegativeWrapWidth(-1));
    }

    #[test]
    fn occurrence_ids_distinguish_a_shared_visible_node() {
        let qp = QueryPlan::new(vec![
            relational(0, "Root", vec![link(1, ""), link(2, "")]),
            relational(1, "Left", vec![link(3, "")]),
            relational(2, "Right", vec![link(3, "")]),
            relational(3, "Shared", vec![]),
        ])
        .unwrap();

        let rows = process_plan(&qp, &ProcessPlanOptions::default()).unwrap();
        let identities: Vec<_> = rows
            .iter()
            .map(|row| (row.id, row.row_id.as_str(), row.parent_row_id.as_deref()))
            .collect();

        assert_eq!(
            identities,
            vec![
                (0, "0", None),
                (1, "0.0", Some("0")),
                (3, "0.0.0", Some("0.0")),
                (2, "0.1", Some("0")),
                (3, "0.1.0", Some("0.1")),
            ]
        );
    }

    #[test]
    fn shared_occurrences_use_the_actual_parent_edge_for_implicit_input() {
        for (apply_at_first_child, expected) in [
            (true, vec!["[Input] Shared", "Shared"]),
            (false, vec!["Shared", "[Input] Shared"]),
        ] {
            let first_parent = if apply_at_first_child {
                "Cross Apply"
            } else {
                "Other"
            };
            let second_parent = if apply_at_first_child {
                "Other"
            } else {
                "Cross Apply"
            };
            let qp = QueryPlan::new(vec![
                relational(0, "Root", vec![link(1, ""), link(2, "")]),
                relational(1, first_parent, vec![link(3, "")]),
                relational(2, second_parent, vec![link(3, "")]),
                relational(3, "Shared", vec![]),
            ])
            .unwrap();

            let rows = process_plan(&qp, &ProcessPlanOptions::default()).unwrap();
            assert_eq!(
                rows.iter()
                    .filter(|row| row.id == 3)
                    .map(|row| row.node_text.as_str())
                    .collect::<Vec<_>>(),
                expected,
                "apply_at_first_child={apply_at_first_child}",
            );
        }
    }

    #[test]
    fn only_the_first_actual_child_link_of_apply_is_implicit_input() {
        let qp = QueryPlan::new(vec![
            relational(0, "Cross Apply", vec![link(1, ""), link(1, "")]),
            relational(1, "Shared", vec![]),
        ])
        .unwrap();

        let rows = process_plan(&qp, &ProcessPlanOptions::default()).unwrap();
        assert_eq!(
            rows.iter()
                .filter(|row| row.id == 1)
                .map(|row| (row.row_id.as_str(), row.node_text.as_str()))
                .collect::<Vec<_>>(),
            vec![("0.0", "[Input] Shared"), ("0.1", "Shared")],
        );
    }

    #[test]
    fn relational_cycle_is_rejected_without_deduplicating_a_dag() {
        let qp = QueryPlan::new(vec![
            relational(0, "Root", vec![link(1, "")]),
            relational(1, "Child", vec![link(0, "")]),
        ])
        .unwrap();

        let err = process_plan(&qp, &ProcessPlanOptions::default()).unwrap_err();
        assert_eq!(err, ProcessPlanError::Cycle(0));
    }

    #[test]
    fn cycle_is_checked_before_occurrence_budget() {
        let mut root_links = vec![link(1, ""); MAX_PLANTREE_OCCURRENCES];
        root_links[MAX_PLANTREE_OCCURRENCES - 1] = link(0, "");
        let qp = QueryPlan::new(vec![
            relational(0, "Root", root_links),
            relational(1, "Shared", vec![]),
        ])
        .unwrap();

        let err = process_plan(&qp, &ProcessPlanOptions::default()).unwrap_err();
        assert_eq!(err, ProcessPlanError::Cycle(0));
    }

    #[test]
    fn depth_limit_rejects_depth_257() {
        let nodes = (0..=MAX_PLANTREE_DEPTH + 1)
            .map(|index| {
                relational(
                    index as i32,
                    "Chain",
                    if index == MAX_PLANTREE_DEPTH + 1 {
                        vec![]
                    } else {
                        vec![link(index as i32 + 1, "")]
                    },
                )
            })
            .collect();
        let qp = QueryPlan::new(nodes).unwrap();

        let err = std::thread::Builder::new()
            .name("plantree-depth-limit".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || process_plan(&qp, &ProcessPlanOptions::default()).unwrap_err())
            .unwrap()
            .join()
            .unwrap();

        assert_eq!(
            err,
            ProcessPlanError::DepthLimitExceeded {
                limit: MAX_PLANTREE_DEPTH,
                node_index: (MAX_PLANTREE_DEPTH + 1) as i32,
            }
        );
        assert_eq!(
            err.to_string(),
            "plan exceeds the renderer depth budget 256 at PlanNode index 257"
        );
    }

    #[test]
    fn occurrence_limit_rejects_shared_dag_expansion() {
        let branch_count: i32 = 12;
        let mut nodes = Vec::with_capacity(branch_count as usize + 1);
        for index in 0..=branch_count {
            nodes.push(relational(
                index,
                "Branch",
                if index == branch_count {
                    vec![]
                } else {
                    vec![link(index + 1, ""), link(index + 1, "")]
                },
            ));
        }
        let qp = QueryPlan::new(nodes).unwrap();

        let err = process_plan(&qp, &ProcessPlanOptions::default()).unwrap_err();
        assert_eq!(
            err,
            ProcessPlanError::OccurrenceLimitExceeded {
                limit: MAX_PLANTREE_OCCURRENCES,
                node_index: 1,
            }
        );
        assert_eq!(
            err.to_string(),
            "plan exceeds the renderer occurrence budget 4096 at PlanNode index 1"
        );
    }

    #[test]
    fn simple_search_predicate() {
        let qp = QueryPlan::new(vec![
            relational(0, "Scan", vec![link(1, "Search Predicate")]),
            scalar(1, "Search Predicate", "SEARCH(Tokens, 'blue')"),
        ])
        .unwrap();
        let rows = process_plan(&qp, &ProcessPlanOptions::default()).unwrap();
        assert_eq!(
            row_by_id(&rows, 0).predicates,
            vec!["Search Predicate: SEARCH(Tokens, 'blue')".to_string()]
        );
    }

    #[test]
    fn compound_search_predicate_function() {
        let qp = QueryPlan::new(vec![
            relational(0, "Scan", vec![link(1, "Search Predicate")]),
            scalar_with_links(
                1,
                "Function",
                "(SEARCH(Tokens, 'blue') AND SEARCH(Tokens, 'green'))",
                vec![link(2, "Search Predicate"), link(3, "Search Predicate")],
            ),
            scalar(2, "Search Predicate", "SEARCH(Tokens, 'blue')"),
            scalar(3, "Search Predicate", "SEARCH(Tokens, 'green')"),
        ])
        .unwrap();
        let rows = process_plan(&qp, &ProcessPlanOptions::default()).unwrap();
        assert_eq!(
            row_by_id(&rows, 0).predicates,
            vec![
                "Search Predicate: (SEARCH(Tokens, 'blue') AND SEARCH(Tokens, 'green'))"
                    .to_string()
            ]
        );
    }

    #[test]
    fn scalar_child_links_preserve_parent_context_and_order() {
        let qp = QueryPlan::new(vec![
            relational(
                0,
                "Sort",
                vec![
                    var_link(1, "Key", "sort_key"),
                    var_link(2, "Value", "sort_value"),
                    link(3, ""),
                ],
            ),
            scalar(1, "Reference", "$SongGenre"),
            scalar(2, "Reference", "$SongName"),
            relational(
                3,
                "Aggregate",
                vec![
                    var_link(4, "Key", "group_key"),
                    var_link(5, "Agg", "song_count"),
                ],
            ),
            scalar(4, "Reference", "$SingerId"),
            scalar(5, "Function", "COUNT(*)"),
        ])
        .unwrap();
        let rows = process_plan(&qp, &ProcessPlanOptions::default()).unwrap();

        let sort_row = row_by_id(&rows, 0);
        assert_eq!(sort_row.display_name, "Sort");
        assert_eq!(
            sort_row.scalar_child_links,
            vec![
                ScalarChildLink {
                    r#type: "Key".to_string(),
                    variable: "sort_key".to_string(),
                    description: "$SongGenre".to_string(),
                    display_name: "Reference".to_string(),
                    child_index: 1,
                    is_predicate: false,
                },
                ScalarChildLink {
                    r#type: "Value".to_string(),
                    variable: "sort_value".to_string(),
                    description: "$SongName".to_string(),
                    display_name: "Reference".to_string(),
                    child_index: 2,
                    is_predicate: false,
                },
            ]
        );

        let aggregate_row = row_by_id(&rows, 3);
        assert_eq!(
            aggregate_row.scalar_child_links,
            vec![
                ScalarChildLink {
                    r#type: "Key".to_string(),
                    variable: "group_key".to_string(),
                    description: "$SingerId".to_string(),
                    display_name: "Reference".to_string(),
                    child_index: 4,
                    is_predicate: false,
                },
                ScalarChildLink {
                    r#type: "Agg".to_string(),
                    variable: "song_count".to_string(),
                    description: "COUNT(*)".to_string(),
                    display_name: "Function".to_string(),
                    child_index: 5,
                    is_predicate: false,
                },
            ]
        );
    }

    #[test]
    fn scalar_child_links_classify_predicates_without_changing_order() {
        let qp = QueryPlan::new(vec![
            relational(
                0,
                "Filter Scan",
                vec![
                    var_link(1, "Key", "key"),
                    link(2, "Condition"),
                    var_link(3, "Value", "value"),
                    link(4, "Search Predicate"),
                ],
            ),
            scalar(1, "Reference", "$SingerId"),
            scalar(2, "Function", "SingerId = 1"),
            scalar(3, "Reference", "$SongName"),
            scalar(4, "Search Predicate", "SEARCH(Tokens, 'blue')"),
        ])
        .unwrap();

        let rows = process_plan(&qp, &ProcessPlanOptions::default()).unwrap();
        let row = row_by_id(&rows, 0);
        assert_eq!(
            row.scalar_child_links
                .iter()
                .map(|link| (link.child_index, link.is_predicate))
                .collect::<Vec<_>>(),
            vec![(1, false), (2, true), (3, false), (4, true)]
        );
        assert_eq!(
            row.predicates,
            vec![
                "Condition: SingerId = 1".to_string(),
                "Search Predicate: SEARCH(Tokens, 'blue')".to_string(),
            ]
        );
        assert_eq!(
            row.scalar_child_links
                .iter()
                .filter(|link| link.is_predicate)
                .map(|link| format!("{}: {}", link.r#type, link.description))
                .collect::<Vec<_>>(),
            row.predicates
        );
    }

    fn current_options() -> ProcessPlanOptions {
        use crate::queryplan::{ExecutionMethodFormat, KnownFlagFormat, TargetMetadataFormat};
        ProcessPlanOptions::default().with_query_plan_options(
            NodeTitleOptions::default()
                .with_target_metadata_format(TargetMetadataFormat::On)
                .with_execution_method_format(ExecutionMethodFormat::Angle)
                .with_known_flag_format(KnownFlagFormat::Label),
        )
    }

    fn hanging_indent_plan() -> QueryPlan {
        let mut batch_scan = relational(1, "Batch Scan", vec![]);
        batch_scan.metadata = Metadata::from_iter([(
            "execution_method".to_string(),
            MetadataValue::String("Row".to_string()),
        )]);
        QueryPlan::new(vec![
            relational(0, "Cross Apply", vec![link(1, ""), link(2, "Map")]),
            batch_scan,
            relational(2, "Serialize Result", vec![]),
        ])
        .unwrap()
    }

    fn hanging_indent_child_guide_plan() -> QueryPlan {
        let mut batch_scan = relational(1, "Batch Scan", vec![link(3, "")]);
        batch_scan.metadata = Metadata::from_iter([(
            "execution_method".to_string(),
            MetadataValue::String("Row".to_string()),
        )]);
        QueryPlan::new(vec![
            relational(0, "Cross Apply", vec![link(1, ""), link(2, "Map")]),
            batch_scan,
            relational(2, "Serialize Result", vec![]),
            relational(3, "Filter Scan", vec![]),
        ])
        .unwrap()
    }

    #[test]
    fn hanging_indent() {
        let opts = current_options().with_wrap_width(21).with_hanging_indent();
        let rows = process_plan(&hanging_indent_plan(), &opts).unwrap();
        let got = row_by_id(&rows, 1);
        assert_eq!(got.tree_part, "+- \n|          ");
        assert_eq!(got.node_text, "[Input] Batch Scan\n <Row>");
    }

    #[test]
    fn hanging_indent_keeps_child_guide() {
        let opts = current_options().with_wrap_width(21).with_hanging_indent();
        let rows = process_plan(&hanging_indent_child_guide_plan(), &opts).unwrap();
        let got = row_by_id(&rows, 1);
        assert_eq!(got.tree_part, "+- \n|  |       ");
        assert_eq!(got.node_text, "[Input] Batch Scan\n <Row>");
    }

    #[test]
    fn format_id_stars_rows_with_predicates() {
        let row = RowWithPredicates {
            id: 31,
            predicates: vec!["Condition: x".to_string()],
            ..RowWithPredicates::default()
        };
        assert_eq!(row.format_id(), "*31");
        let plain = RowWithPredicates {
            id: 4,
            ..RowWithPredicates::default()
        };
        assert_eq!(plain.format_id(), "4");
    }

    #[test]
    fn tree_part_accessors() {
        let r = RowWithPredicates {
            tree_part: "  +- \n|  ".to_string(),
            node_text: "a\nb".to_string(),
            ..RowWithPredicates::default()
        };
        assert_eq!(r.tree_part_lines(), vec!["  +- ", "|  "]);
        assert_eq!(r.text(), "  +- a\n|  b");
    }
}
