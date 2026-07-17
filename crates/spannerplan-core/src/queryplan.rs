//! `QueryPlan`: the validated, parent-linked graph built from a plan-node
//! list, plus `NodeTitle` formatting and child-link classification. Port of
//! root `queryplan.go`. See `DESIGN.md` §6.1/§6.2.

use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::model::{ChildLink, Kind, PlanNode};

/// Errors from [`QueryPlan::new`].
///
/// Go's `New` also has `ErrNilPlanNode`/`ErrNilChildLink` variants; those
/// don't apply here because `Vec<PlanNode>`/`Vec<ChildLink>` can't hold a
/// "nil" element the way a Go slice of pointers can.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryPlanError {
    EmptyPlanNodes,
    PlanNodeIndexMismatch {
        position: usize,
        expected: i32,
        got: i32,
    },
    ChildLinkIndexOutOfRange {
        parent_index: i32,
        link_index: usize,
        child_index: i32,
        len: usize,
    },
}

impl core::fmt::Display for QueryPlanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            QueryPlanError::EmptyPlanNodes => {
                f.write_str("spannerplan: planNodes cannot be empty")
            }
            QueryPlanError::PlanNodeIndexMismatch {
                position,
                expected,
                got,
            } => write!(
                f,
                "spannerplan: planNode index must match slice position: at slice position {position} expected index {expected}, got {got}"
            ),
            QueryPlanError::ChildLinkIndexOutOfRange {
                parent_index,
                link_index,
                child_index,
                len,
            } => write!(
                f,
                "spannerplan: childLink childIndex out of range: parent node {parent_index} childLinks[{link_index}] has childIndex {child_index}, len(planNodes)={len}"
            ),
        }
    }
}

/// A parent `PlanNode` and the `ChildLink` through which it points at a
/// particular child. Returned by [`QueryPlan::parent_links`].
#[derive(Debug, Clone, Copy)]
pub struct ResolvedParentLink<'a> {
    pub parent: &'a PlanNode,
    pub child_link: &'a ChildLink,
}

/// A child `PlanNode` and the `ChildLink` a parent used to reach it.
/// Returned by [`QueryPlan::resolve_child_link`].
#[derive(Debug, Clone, Copy)]
pub struct ResolvedChildLink<'a> {
    pub child_link: &'a ChildLink,
    pub child: &'a PlanNode,
}

#[derive(Debug, Clone, Copy)]
struct ParentLinkRef {
    parent_index: i32,
    child_link_index: usize,
}

/// The validated, parent-linked graph built from a `PlanNode` list. Port of
/// Go's `QueryPlan`.
#[derive(Debug)]
pub struct QueryPlan {
    plan_nodes: Vec<PlanNode>,
    parent_map: BTreeMap<i32, i32>,
    parent_links_map: BTreeMap<i32, Vec<ParentLinkRef>>,
}

impl QueryPlan {
    /// Constructs a `QueryPlan` from `sppb.QueryPlan.PlanNodes`.
    ///
    /// The input must be the original `PlanNodes` slice from Cloud Spanner's
    /// `sppb.QueryPlan`: each node's `index` must match its position in the
    /// slice, and every `ChildLink.child_index` must be in range. Mirrors Go
    /// `New`.
    pub fn new(plan_nodes: Vec<PlanNode>) -> Result<Self, QueryPlanError> {
        if plan_nodes.is_empty() {
            return Err(QueryPlanError::EmptyPlanNodes);
        }

        for (i, node) in plan_nodes.iter().enumerate() {
            if node.get_index() != i as i32 {
                return Err(QueryPlanError::PlanNodeIndexMismatch {
                    position: i,
                    expected: i as i32,
                    got: node.get_index(),
                });
            }
        }

        let mut parent_map = BTreeMap::new();
        let mut parent_links_map: BTreeMap<i32, Vec<ParentLinkRef>> = BTreeMap::new();
        for node in &plan_nodes {
            for (j, child_link) in node.get_child_links().iter().enumerate() {
                let child_index = child_link.get_child_index();
                if child_index < 0 || child_index as usize >= plan_nodes.len() {
                    return Err(QueryPlanError::ChildLinkIndexOutOfRange {
                        parent_index: node.get_index(),
                        link_index: j,
                        child_index,
                        len: plan_nodes.len(),
                    });
                }
                parent_map.insert(child_index, node.get_index());
                parent_links_map
                    .entry(child_index)
                    .or_default()
                    .push(ParentLinkRef {
                        parent_index: node.get_index(),
                        child_link_index: j,
                    });
            }
        }

        Ok(QueryPlan {
            plan_nodes,
            parent_map,
            parent_links_map,
        })
    }

    pub fn has_stats(&self) -> bool {
        has_stats(self.plan_nodes())
    }

    pub fn plan_nodes(&self) -> &[PlanNode] {
        &self.plan_nodes
    }

    /// # Panics
    ///
    /// Panics if `id` is negative or out of bounds for `plan_nodes`.
    pub fn get_node_by_index(&self, id: i32) -> &PlanNode {
        &self.plan_nodes[id as usize]
    }

    /// Returns the `PlanNode` `link` points at. `link = None` represents the
    /// root node (index 0): Go's nil-safe `link.GetChildIndex()` returns `0`
    /// for a nil link, and this mirrors that by indexing with `0`.
    ///
    /// # Panics
    ///
    /// Panics if `link` is `Some` and its child index is out of bounds, or if
    /// `link` is `None` and the plan has no nodes.
    pub fn get_node_by_child_link(&self, link: Option<&ChildLink>) -> &PlanNode {
        let idx = link.map(ChildLink::get_child_index).unwrap_or(0);
        &self.plan_nodes[idx as usize]
    }

    /// Mirrors Go `GetParentNodeByChildIndex`, including its map-zero-value
    /// quirk: if `index` was never registered as a child of any node (e.g.
    /// `index` is the root's own index, or an unreachable node), this
    /// returns `plan_nodes[0]` (the root) rather than `None`, because Go's
    /// `parentMap[index]` silently returns the zero value `0` for a missing
    /// key and `GetParentNodeByChildIndex` indexes with that value
    /// unconditionally. Real call sites only ever pass indices that come
    /// from an actual `ChildLink`, which `new()` always registers, so this
    /// only matters for out-of-tree probing.
    pub fn get_parent_node_by_child_index(&self, index: i32) -> &PlanNode {
        let parent_index = self.parent_map.get(&index).copied().unwrap_or(0);
        &self.plan_nodes[parent_index as usize]
    }

    pub fn get_parent_node_by_child_link(&self, link: Option<&ChildLink>) -> &PlanNode {
        let idx = link.map(ChildLink::get_child_index).unwrap_or(0);
        self.get_parent_node_by_child_index(idx)
    }

    /// Returns all parent child-links that point to `child_index`, in plan
    /// node traversal order, preserving each parent's `ChildLinks` order.
    /// Freshly constructed each call, so (unlike Go slice semantics) there is
    /// no shared internal storage a caller could mutate.
    pub fn parent_links(&self, child_index: i32) -> Vec<ResolvedParentLink<'_>> {
        self.parent_links_map
            .get(&child_index)
            .map(|refs| {
                refs.iter()
                    .map(|r| ResolvedParentLink {
                        parent: &self.plan_nodes[r.parent_index as usize],
                        child_link: &self.plan_nodes[r.parent_index as usize].child_links
                            [r.child_link_index],
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn resolve_child_link<'a>(&'a self, item: &'a ChildLink) -> ResolvedChildLink<'a> {
        ResolvedChildLink {
            child_link: item,
            child: self.get_node_by_child_link(Some(item)),
        }
    }

    /// A child link should be rendered as part of the operator tree iff its
    /// child is a `RELATIONAL` node, or the link itself has type `"Scalar"`
    /// (which marks scalar-subquery-like operator subtrees). `link = None`
    /// represents the root node.
    pub fn is_visible(&self, link: Option<&ChildLink>) -> bool {
        self.get_node_by_child_link(link).get_kind() == Kind::Relational
            || link.map(ChildLink::get_type).unwrap_or("") == "Scalar"
    }

    pub fn visible_child_links<'a>(&self, node: &'a PlanNode) -> Vec<&'a ChildLink> {
        node.get_child_links()
            .iter()
            .filter(|l| self.is_visible(Some(l)))
            .collect()
    }

    pub fn is_function(&self, link: Option<&ChildLink>) -> bool {
        self.get_node_by_child_link(link).get_display_name() == "Function"
    }

    /// Known predicates are Search Predicate (Full Text Search),
    /// Condition (Filter, Hash Join), Seek Condition (FilterScan),
    /// Residual Condition (FilterScan, Hash Join), or Split Range
    /// (Distributed Union). Agg (Aggregate) is a Function but not a
    /// predicate.
    pub fn is_predicate(&self, link: Option<&ChildLink>) -> bool {
        let link_type = link.map(ChildLink::get_type).unwrap_or("");
        if link_type == "Search Predicate" {
            return self.get_node_by_child_link(link).get_kind() == Kind::Scalar;
        }
        if !self.is_function(link) {
            return false;
        }
        link_type.ends_with("Condition") || link_type == "Split Range"
    }

    /// If `link` has an explicit type, returns it. Otherwise applies the
    /// Apply-input workaround: treats the first child link of an `*Apply`
    /// operator (Cross Apply, Anti Semi Apply, Semi Apply, Outer Apply, and
    /// their Distributed variants) as `"Input"`, matching the official query
    /// plan operator docs.
    pub fn get_link_type<'a>(&self, link: Option<&'a ChildLink>) -> &'a str {
        if let Some(l) = link {
            if !l.get_type().is_empty() {
                return l.get_type();
            }
        }

        let parent = self.get_parent_node_by_child_link(link);
        let link_child_index = link.map(ChildLink::get_child_index).unwrap_or(0);
        if parent.get_display_name().ends_with("Apply")
            && !parent.get_child_links().is_empty()
            && parent.get_child_links()[0].get_child_index() == link_child_index
        {
            return "Input";
        }
        ""
    }
}

/// `nodes[0].ExecutionStats != nil`: only the first node is checked, matching
/// Go `HasStats`.
pub fn has_stats(nodes: &[PlanNode]) -> bool {
    nodes
        .first()
        .map(|n| n.get_execution_stats().is_some())
        .unwrap_or(false)
}

// --- NodeTitle formatting options -----------------------------------------

/// A parse error for the string-encoded `NodeTitle` option enums
/// (`ExecutionMethodFormat`, `TargetMetadataFormat`, `KnownFlagFormat`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOptionError(String);

impl core::fmt::Display for ParseOptionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExecutionMethodFormat {
    /// Print `execution_method` metadata as is.
    #[default]
    Raw,
    /// Print `execution_method` metadata after `display_name` with angle
    /// brackets, e.g. `Scan <Row>`.
    Angle,
}

pub fn parse_execution_method_format(s: &str) -> Result<ExecutionMethodFormat, ParseOptionError> {
    if s.eq_ignore_ascii_case("RAW") {
        Ok(ExecutionMethodFormat::Raw)
    } else if s.eq_ignore_ascii_case("ANGLE") {
        Ok(ExecutionMethodFormat::Angle)
    } else {
        Err(ParseOptionError(format!(
            "invalid ExecutionMethodFormat, expect RAW or ANGLE: {s}"
        )))
    }
}

/// Controls how to render target metadata: `scan_target`, `distribution_table`,
/// and `table`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TargetMetadataFormat {
    /// Print target metadata as is.
    #[default]
    Raw,
    /// Print target metadata as `on <target>`.
    On,
}

pub fn parse_target_metadata_format(s: &str) -> Result<TargetMetadataFormat, ParseOptionError> {
    if s.eq_ignore_ascii_case("RAW") {
        Ok(TargetMetadataFormat::Raw)
    } else if s.eq_ignore_ascii_case("ON") {
        Ok(TargetMetadataFormat::On)
    } else {
        Err(ParseOptionError(format!(
            "invalid TargetMetadataFormat, expect RAW or ON: {s}"
        )))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KnownFlagFormat {
    /// Print known boolean flag metadata as is.
    #[default]
    Raw,
    /// Print known boolean flag metadata without a value if true, or omit if false.
    Label,
}

pub fn parse_known_flag_format(s: &str) -> Result<KnownFlagFormat, ParseOptionError> {
    if s.eq_ignore_ascii_case("RAW") {
        Ok(KnownFlagFormat::Raw)
    } else if s.eq_ignore_ascii_case("LABEL") {
        Ok(KnownFlagFormat::Label)
    } else {
        Err(ParseOptionError(format!(
            "invalid KnownFlagFormat, expect RAW or LABEL: {s}"
        )))
    }
}

/// Options for [`node_title`]. A plain struct with builder methods, rather
/// than a literal port of Go's closure-based functional-options `Option`
/// type: idiomatic Rust, same observable behavior.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeTitleOptions {
    pub execution_method_format: ExecutionMethodFormat,
    pub target_metadata_format: TargetMetadataFormat,
    pub known_flag_format: KnownFlagFormat,
    pub compact: bool,
    pub hide_metadata: bool,
    // Go's `option.inlineStatsFunc` (wired up by `--inline-stats`) is
    // deferred; see DESIGN.md §12 (non-goals).
}

impl NodeTitleOptions {
    pub fn with_execution_method_format(mut self, f: ExecutionMethodFormat) -> Self {
        self.execution_method_format = f;
        self
    }

    pub fn with_target_metadata_format(mut self, f: TargetMetadataFormat) -> Self {
        self.target_metadata_format = f;
        self
    }

    pub fn with_known_flag_format(mut self, f: KnownFlagFormat) -> Self {
        self.known_flag_format = f;
        self
    }

    /// Mirrors Go `EnableCompact` (the `NodeTitle`-only part; `plantree`'s
    /// `EnableCompact` also switches the tree-render style, in a later phase).
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Hides all metadata and labels even if `KnownFlagFormat::Label` is set.
    pub fn hide_metadata(mut self) -> Self {
        self.hide_metadata = true;
        self
    }
}

const KNOWN_BOOLEAN_FLAG_KEYS: [&str; 2] = ["Full scan", "split_ranges_aligned"];
const TARGET_METADATA_KEYS: [&str; 3] = ["scan_target", "distribution_table", "table"];

/// Renders a `PlanNode`'s operator title with its metadata, e.g.
/// `Distributed Union on AlbumsByAlbumTitle <Row> (Split Range: ...)`. Port
/// of Go `NodeTitle`. See `DESIGN.md` §6.2 for the field-by-field mapping to
/// the Go source this mirrors line-for-line (including branches that are
/// redundant given an earlier guard, kept for exact parity / easy diffing).
pub fn node_title(node: &PlanNode, opts: &NodeTitleOptions) -> String {
    let sep = if opts.compact { "" } else { " " };

    let execution_method = node.get_metadata_str("execution_method");

    let target = TARGET_METADATA_KEYS
        .iter()
        .map(|k| node.get_metadata_str(k))
        .find(|v| !v.is_empty())
        .unwrap_or("");

    let scan_type = node.get_metadata_str("scan_type");
    let scan_type_trimmed = scan_type.strip_suffix("Scan").unwrap_or(scan_type);

    let on_target = if opts.target_metadata_format == TargetMetadataFormat::On && !target.is_empty()
    {
        format!("on {target}")
    } else {
        String::new()
    };

    let call_type = node.get_metadata_str("call_type");
    let iterator_type = node.get_metadata_str("iterator_type");
    let display_name = node.get_display_name();
    let operator = join_if_not_empty(
        " ",
        &[
            call_type,
            iterator_type,
            scan_type_trimmed,
            display_name,
            on_target.as_str(),
        ],
    );

    let execution_method_part = if opts.execution_method_format == ExecutionMethodFormat::Angle
        && !execution_method.is_empty()
    {
        format!("<{execution_method}>")
    } else {
        String::new()
    };

    let mut labels: Vec<String> = Vec::new();
    let mut fields: Vec<String> = Vec::new();

    if !opts.hide_metadata {
        for (k, v) in node.metadata.iter() {
            let k = k.as_str();
            if opts.target_metadata_format != TargetMetadataFormat::Raw
                && TARGET_METADATA_KEYS.contains(&k)
            {
                continue;
            }

            match k {
                "call_type" | "iterator_type" | "scan_type" | "subquery_cluster_node" => continue,
                "scan_target" => {
                    if opts.target_metadata_format != TargetMetadataFormat::Raw {
                        continue;
                    }
                    fields.push(format!("{scan_type_trimmed}: {}", v.as_str()));
                    continue;
                }
                "execution_method" => {
                    if opts.execution_method_format != ExecutionMethodFormat::Raw {
                        continue;
                    }
                }
                "distribution_table" | "table"
                    if opts.target_metadata_format != TargetMetadataFormat::Raw =>
                {
                    continue;
                }
                _ => {}
            }

            if opts.known_flag_format != KnownFlagFormat::Raw
                && KNOWN_BOOLEAN_FLAG_KEYS.contains(&k)
            {
                if v.as_str() == "true" {
                    labels.push(k.to_owned());
                }
                continue;
            }
            fields.push(format!("{k}:{sep}{}", v.as_str()));
        }
    }

    labels.sort_unstable();
    fields.sort_unstable();

    let mut items: Vec<&str> = Vec::with_capacity(labels.len() + fields.len());
    items.extend(labels.iter().map(String::as_str));
    items.extend(fields.iter().map(String::as_str));

    let item_sep = format!(",{sep}");
    let joined_items = items.join(item_sep.as_str());
    let parenthesized = enclose_if_not_empty("(", &joined_items, ")");

    join_if_not_empty(
        sep,
        &[
            operator.as_str(),
            execution_method_part.as_str(),
            parenthesized.as_str(),
        ],
    )
}

fn enclose_if_not_empty(open: &str, input: &str, close: &str) -> String {
    if input.is_empty() {
        String::new()
    } else {
        format!("{open}{input}{close}")
    }
}

fn join_if_not_empty(sep: &str, parts: &[&str]) -> String {
    let filtered: Vec<&str> = parts.iter().copied().filter(|s| !s.is_empty()).collect();
    filtered.join(sep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Metadata, MetadataValue, ShortRepresentation};

    fn node(index: i32, kind: Kind, display_name: &str, child_links: Vec<ChildLink>) -> PlanNode {
        PlanNode {
            index,
            kind,
            display_name: display_name.to_string(),
            child_links,
            short_representation: None,
            metadata: Metadata::new(),
            execution_stats: None,
        }
    }

    fn link(child_index: i32, r#type: &str) -> ChildLink {
        ChildLink {
            child_index,
            r#type: r#type.to_string(),
            variable: String::new(),
        }
    }

    // --- QueryPlan::new (ports TestNew, minus the nil-node/nil-child-link
    // cases that don't apply to Vec<PlanNode>/Vec<ChildLink>) -------------

    #[test]
    fn new_rejects_empty_plan_nodes() {
        assert_eq!(
            QueryPlan::new(Vec::new()).unwrap_err(),
            QueryPlanError::EmptyPlanNodes
        );
    }

    #[test]
    fn new_rejects_index_mismatch() {
        let nodes = alloc::vec![node(1, Kind::Relational, "", Vec::new())];
        assert_eq!(
            QueryPlan::new(nodes).unwrap_err(),
            QueryPlanError::PlanNodeIndexMismatch {
                position: 0,
                expected: 0,
                got: 1
            }
        );
    }

    #[test]
    fn new_rejects_child_link_index_out_of_range() {
        let nodes = alloc::vec![
            node(0, Kind::Relational, "", alloc::vec![link(2, "")]),
            node(1, Kind::Relational, "", Vec::new()),
        ];
        assert_eq!(
            QueryPlan::new(nodes).unwrap_err(),
            QueryPlanError::ChildLinkIndexOutOfRange {
                parent_index: 0,
                link_index: 0,
                child_index: 2,
                len: 2,
            }
        );
    }

    #[test]
    fn new_valid_query_plan_builds_parent_map() {
        let nodes = alloc::vec![
            node(0, Kind::Relational, "", alloc::vec![link(1, "")]),
            node(1, Kind::Relational, "", Vec::new()),
        ];
        let qp = QueryPlan::new(nodes).unwrap();
        assert_eq!(qp.get_parent_node_by_child_index(1).get_index(), 0);
    }

    // --- has_stats (ports TestHasStats) -----------------------------------

    #[test]
    fn has_stats_true_when_first_node_has_stats() {
        let mut n = node(0, Kind::Relational, "", Vec::new());
        n.execution_stats = Some(Metadata::new());
        assert!(has_stats(&[n]));
    }

    #[test]
    fn has_stats_false_when_first_node_has_no_stats() {
        let n = node(0, Kind::Relational, "", Vec::new());
        assert!(!has_stats(&[n]));
    }

    #[test]
    fn has_stats_false_when_empty() {
        assert!(!has_stats(&[]));
    }

    // --- parent_links (ports TestParentLinks) -----------------------------

    #[test]
    fn parent_links_returns_all_links_in_traversal_order() {
        let first_link = link(2, "Input");
        let second_link = link(2, "Scalar");
        let third_link = link(2, "Condition");

        let nodes = alloc::vec![
            node(
                0,
                Kind::Relational,
                "",
                alloc::vec![first_link.clone(), second_link.clone()],
            ),
            node(1, Kind::Relational, "", alloc::vec![third_link.clone()]),
            node(2, Kind::Relational, "", Vec::new()),
        ];
        let qp = QueryPlan::new(nodes).unwrap();

        let got = qp.parent_links(2);
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].parent.get_index(), 0);
        assert_eq!(got[0].child_link.get_type(), "Input");
        assert_eq!(got[1].parent.get_index(), 0);
        assert_eq!(got[1].child_link.get_type(), "Scalar");
        assert_eq!(got[2].parent.get_index(), 1);
        assert_eq!(got[2].child_link.get_type(), "Condition");

        assert_eq!(qp.get_parent_node_by_child_index(2).get_index(), 1);
        assert!(qp.parent_links(0).is_empty());
        assert!(qp.parent_links(99).is_empty());
    }

    // --- is_predicate (ports TestIsPredicate) -----------------------------

    fn scalar_node(index: i32, display_name: &str) -> PlanNode {
        node(index, Kind::Scalar, display_name, Vec::new())
    }

    #[test]
    fn is_predicate_search_predicate_scalar_node() {
        let l = link(1, "Search Predicate");
        let qp = QueryPlan::new(alloc::vec![
            node(0, Kind::Relational, "", alloc::vec![l.clone()]),
            scalar_node(1, "Search Predicate"),
        ])
        .unwrap();
        assert!(qp.is_predicate(Some(&l)));
    }

    #[test]
    fn is_predicate_compound_search_predicate_function() {
        let l = link(1, "Search Predicate");
        let qp = QueryPlan::new(alloc::vec![
            node(0, Kind::Relational, "", alloc::vec![l.clone()]),
            scalar_node(1, "Function"),
        ])
        .unwrap();
        assert!(qp.is_predicate(Some(&l)));
    }

    #[test]
    fn is_predicate_search_predicate_link_to_relational_node_is_false() {
        let l = link(1, "Search Predicate");
        let qp = QueryPlan::new(alloc::vec![
            node(0, Kind::Relational, "", alloc::vec![l.clone()]),
            node(1, Kind::Relational, "Scan", Vec::new()),
        ])
        .unwrap();
        assert!(!qp.is_predicate(Some(&l)));
    }

    #[test]
    fn is_predicate_condition_function_is_predicate() {
        let l = link(1, "Seek Condition");
        let qp = QueryPlan::new(alloc::vec![
            node(0, Kind::Relational, "", alloc::vec![l.clone()]),
            scalar_node(1, "Function"),
        ])
        .unwrap();
        assert!(qp.is_predicate(Some(&l)));
    }

    #[test]
    fn is_predicate_aggregate_function_is_not_predicate() {
        let l = link(1, "Agg");
        let qp = QueryPlan::new(alloc::vec![
            node(0, Kind::Relational, "", alloc::vec![l.clone()]),
            scalar_node(1, "Function"),
        ])
        .unwrap();
        assert!(!qp.is_predicate(Some(&l)));
    }

    // --- get_link_type: Apply-input workaround -----------------------------

    #[test]
    fn get_link_type_returns_explicit_type() {
        let l = link(1, "Condition");
        let qp = QueryPlan::new(alloc::vec![
            node(0, Kind::Relational, "", alloc::vec![l.clone()]),
            node(1, Kind::Relational, "", Vec::new()),
        ])
        .unwrap();
        assert_eq!(qp.get_link_type(Some(&l)), "Condition");
    }

    #[test]
    fn get_link_type_treats_first_apply_child_as_input() {
        let untyped = link(1, "");
        let qp = QueryPlan::new(alloc::vec![
            node(
                0,
                Kind::Relational,
                "Cross Apply",
                alloc::vec![untyped.clone()]
            ),
            node(1, Kind::Relational, "", Vec::new()),
        ])
        .unwrap();
        assert_eq!(qp.get_link_type(Some(&untyped)), "Input");
    }

    #[test]
    fn get_link_type_untyped_non_apply_link_is_empty() {
        let untyped = link(1, "");
        let qp = QueryPlan::new(alloc::vec![
            node(0, Kind::Relational, "Filter", alloc::vec![untyped.clone()]),
            node(1, Kind::Relational, "", Vec::new()),
        ])
        .unwrap();
        assert_eq!(qp.get_link_type(Some(&untyped)), "");
    }

    // --- NodeTitle ---------------------------------------------------------

    fn node_with_metadata(display_name: &str, metadata: &[(&str, &str)]) -> PlanNode {
        let mut n = node(0, Kind::Relational, display_name, Vec::new());
        for (k, v) in metadata {
            n.metadata
                .insert((*k).to_string(), MetadataValue::String((*v).to_string()));
        }
        n
    }

    #[test]
    fn node_title_plain_display_name_no_metadata() {
        let n = node(0, Kind::Relational, "Filter", Vec::new());
        assert_eq!(node_title(&n, &NodeTitleOptions::default()), "Filter");
    }

    #[test]
    fn node_title_scan_type_folds_into_operator_not_fields() {
        let n = node_with_metadata("Scan", &[("scan_type", "TableScan"), ("full_scan", "true")]);
        // scan_type is always skipped as a field and instead prefixes the
        // operator via display_name (e.g. "TableScan" -> "Table" + "Scan");
        // full_scan isn't a known boolean flag key, so it's a plain field
        // rendered as "key: value" (non-compact mode puts a space after ':').
        assert_eq!(
            node_title(&n, &NodeTitleOptions::default()),
            "Table Scan (full_scan: true)"
        );
    }

    #[test]
    fn node_title_metadata_fields_are_sorted_alphabetically() {
        let n = node_with_metadata("Scan", &[("zeta", "1"), ("alpha", "2")]);
        assert_eq!(
            node_title(&n, &NodeTitleOptions::default()),
            "Scan (alpha: 2, zeta: 1)"
        );
    }

    #[test]
    fn node_title_execution_method_angle_format() {
        let n = node_with_metadata("Scan", &[("execution_method", "Row")]);
        let opts =
            NodeTitleOptions::default().with_execution_method_format(ExecutionMethodFormat::Angle);
        assert_eq!(node_title(&n, &opts), "Scan <Row>");
    }

    #[test]
    fn node_title_target_metadata_on_format() {
        let n = node_with_metadata("Scan", &[("scan_target", "Albums")]);
        let opts =
            NodeTitleOptions::default().with_target_metadata_format(TargetMetadataFormat::On);
        assert_eq!(node_title(&n, &opts), "Scan on Albums");
    }

    #[test]
    fn node_title_target_metadata_raw_format_shows_scan_target_as_field() {
        let n = node_with_metadata(
            "Scan",
            &[("scan_target", "Albums"), ("scan_type", "TableScan")],
        );
        assert_eq!(
            node_title(&n, &NodeTitleOptions::default()),
            "Table Scan (Table: Albums)"
        );
    }

    #[test]
    fn node_title_known_flag_label_format_omits_false_and_labels_true() {
        let n = node_with_metadata("Scan", &[("Full scan", "true")]);
        let opts = NodeTitleOptions::default().with_known_flag_format(KnownFlagFormat::Label);
        assert_eq!(node_title(&n, &opts), "Scan (Full scan)");

        let n_false = node_with_metadata("Scan", &[("Full scan", "false")]);
        assert_eq!(node_title(&n_false, &opts), "Scan");
    }

    #[test]
    fn node_title_hide_metadata_suppresses_all_fields() {
        let n = node_with_metadata("Scan", &[("Full scan", "true"), ("other", "x")]);
        let opts = NodeTitleOptions::default().hide_metadata();
        assert_eq!(node_title(&n, &opts), "Scan");
    }

    #[test]
    fn node_title_compact_mode_uses_no_separators() {
        let n = node_with_metadata("Scan", &[("a", "1"), ("b", "2")]);
        let opts = NodeTitleOptions::default().compact();
        assert_eq!(node_title(&n, &opts), "Scan(a:1,b:2)");
    }

    #[test]
    fn node_title_operator_join_uses_fixed_space_even_in_compact_mode() {
        // The operator segment (call_type/iterator_type/scan_type/display_name)
        // always joins with a literal space, unlike the metadata-field
        // separator, which respects compact mode. Regression guard for that
        // asymmetry (DESIGN.md §6.2 step 3 vs step 5).
        let mut n = node(0, Kind::Relational, "Scan", Vec::new());
        n.metadata.insert(
            "iterator_type".to_string(),
            MetadataValue::String("Stream".to_string()),
        );
        let opts = NodeTitleOptions::default().compact();
        assert_eq!(node_title(&n, &opts), "Stream Scan");
    }

    #[test]
    fn short_representation_description_getter_is_nil_safe() {
        let mut n = node(0, Kind::Scalar, "Reference", Vec::new());
        assert_eq!(n.get_short_representation_description(), "");
        n.short_representation = Some(ShortRepresentation {
            description: "$x".to_string(),
        });
        assert_eq!(n.get_short_representation_description(), "$x");
    }
}
