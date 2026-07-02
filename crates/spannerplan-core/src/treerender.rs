//! Generic ASCII operator-tree renderer with optional wrapping and
//! hanging-indent support. Port of `treerender/treerender.go`. See
//! `DESIGN.md` §6.4.

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::textwidth::{string_width, truncate};

/// One vertex in a logical tree rendered as ASCII edges. Convenience type for
/// [`render`]; [`render_tree_with_options`] walks any caller-owned tree via
/// accessor closures.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Node {
    /// The node label rendered after the tree prefix.
    pub text: String,
    /// The child nodes rendered below this node.
    pub children: Vec<Node>,
}

/// One rendered tree row: a tree prefix per visual line plus node text lines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Row {
    /// Everything rendered before `node_text` on each visual line: the ASCII
    /// tree drawing plus any continuation padding added by the renderer (for
    /// example, hanging-indent spacing). Joined with newlines using the same
    /// line structure as `node_text.split('\n')`.
    pub tree_part: String,
    /// The rendered node label, possibly split across visual lines.
    pub node_text: String,
}

impl Row {
    /// Returns the full rendered row text, with the tree prefix prepended to
    /// each node text line. If a manually constructed row has mismatched tree
    /// and node line counts, all lines are preserved.
    pub fn text(&self) -> String {
        let tree_lines: Vec<&str> = self.tree_part.split('\n').collect();
        let node_lines: Vec<&str> = self.node_text.split('\n').collect();
        let num_lines = tree_lines.len().max(node_lines.len());
        let mut out = String::new();
        for i in 0..num_lines {
            if i > 0 {
                out.push('\n');
            }
            if let Some(t) = tree_lines.get(i) {
                out.push_str(t);
            }
            if let Some(n) = node_lines.get(i) {
                out.push_str(n);
            }
        }
        out
    }

    /// Splits [`Row::tree_part`] into one prefix per visual line. Rows
    /// produced by this module align these prefixes with the lines in
    /// [`Row::node_text`].
    pub fn tree_part_lines(&self) -> Vec<&str> {
        self.tree_part.split('\n').collect()
    }
}

/// Selects how wrapped continuation lines align to the tree rail.
///
/// Go's `ContinuationIndent` is an integer type whose invalid values are
/// rejected at render time; a Rust enum makes those unrepresentable, so that
/// error path has no equivalent here.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ContinuationIndent {
    /// Keep wrapped lines aligned only under the tree prefix.
    #[default]
    Tree,
    /// Hang continuation lines after a node-local prefix.
    Anchor,
}

/// Controls display-width handling for wrapped text. Replaces the parts of
/// Go's `tabwrap.Condition` this renderer actually uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrapCondition {
    /// Trim trailing spaces/tabs from each wrapped chunk (stable diffs / CLI
    /// output). Matches the Go default used throughout the pipeline.
    pub trim_trailing_space: bool,
}

impl Default for WrapCondition {
    fn default() -> Self {
        WrapCondition {
            trim_trailing_space: true,
        }
    }
}

/// Configures the optional wrapping behavior of [`render_tree_with_options`].
pub struct RenderOptions<'a, T> {
    /// Returns the node-local prefix used for hanging continuation lines.
    /// Required when [`ContinuationIndent::Anchor`] is selected and
    /// `wrap_width` is positive.
    pub get_continuation_anchor: Option<&'a dyn Fn(&T) -> String>,
    /// The maximum total rendered line width. A non-positive value disables
    /// wrapping.
    pub wrap_width: i32,
    /// Display-width calculation / truncation behavior for wrapped text.
    pub wrap_condition: WrapCondition,
    /// How wrapped continuation lines align.
    pub continuation_indent: ContinuationIndent,
}

impl<T> Default for RenderOptions<'_, T> {
    fn default() -> Self {
        RenderOptions {
            get_continuation_anchor: None,
            wrap_width: 0,
            wrap_condition: WrapCondition::default(),
            continuation_indent: ContinuationIndent::default(),
        }
    }
}

/// Error from [`render_tree_with_options`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// [`ContinuationIndent::Anchor`] with positive `wrap_width` requires
    /// [`RenderOptions::get_continuation_anchor`].
    AnchorCallbackRequired,
}

impl core::fmt::Display for RenderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RenderError::AnchorCallbackRequired => {
                f.write_str("GetContinuationAnchor is required with ContinuationIndentAnchor")
            }
        }
    }
}

/// Configures ASCII edge glyphs and indentation between rails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Style {
    /// The ancestor rail glyph used for rows that have following siblings.
    pub edge_link: String,
    /// The edge glyph used for non-last children.
    pub edge_mid: String,
    /// The edge glyph used for last children.
    pub edge_end: String,
    /// Inserted between an edge glyph and node text.
    pub edge_separator: String,
    /// The number of spaces between ancestor rails. Negative values are
    /// treated as 0.
    pub indent_size: i32,
}

/// The default `"+-"` / `"|"` tree drawing style.
pub fn default_style() -> Style {
    Style {
        edge_link: String::from("|"),
        edge_mid: String::from("+-"),
        edge_end: String::from("+-"),
        edge_separator: String::from(" "),
        indent_size: 2,
    }
}

/// A compact tree style with minimal edge glyphs.
pub fn compact_style() -> Style {
    Style {
        edge_link: String::from("|"),
        edge_mid: String::from("+"),
        edge_end: String::from("+"),
        edge_separator: String::new(),
        indent_size: 0,
    }
}

/// Display widths and derived rail segments for a [`Style`], computed once
/// per render.
struct StyleWidths<'s> {
    style: &'s Style,
    w_link: usize,
    w_mid: usize,
    w_end: usize,
    w_sep: usize,
    indent: usize,
    seg_has_next: String,
    seg_no_next: String,
}

impl StyleWidths<'_> {
    fn new(style: &Style) -> StyleWidths<'_> {
        let indent = style.indent_size.max(0) as usize;
        let w_link = string_width(&style.edge_link);
        let mut seg_has_next = style.edge_link.clone();
        for _ in 0..indent {
            seg_has_next.push(' ');
        }
        let seg_no_next: String = core::iter::repeat_n(' ', indent + w_link).collect();
        StyleWidths {
            style,
            w_link,
            w_mid: string_width(&style.edge_mid),
            w_end: string_width(&style.edge_end),
            w_sep: string_width(&style.edge_separator),
            indent,
            seg_has_next,
            seg_no_next,
        }
    }

    fn segment(&self, has_next: bool) -> &str {
        if has_next {
            &self.seg_has_next
        } else {
            &self.seg_no_next
        }
    }

    fn continuation_segment(&self, is_last: bool) -> &str {
        self.segment(!is_last)
    }
}

fn edge_for_row(is_last: bool, style: &Style) -> &str {
    if is_last {
        &style.edge_end
    } else {
        &style.edge_mid
    }
}

/// Renders a [`Node`] tree with the default (no-wrap) options.
pub fn render(root: Option<&Node>, style: &Style) -> Vec<Row> {
    render_tree(
        root,
        style,
        |n: &Node| n.text.as_str(),
        |n: &Node| n.children.as_slice(),
    )
}

/// Walks an existing tree without copying it into [`Node`], using accessors
/// for `&T` values, with the default (no-wrap) options.
pub fn render_tree<T>(
    root: Option<&T>,
    style: &Style,
    get_text: impl Fn(&T) -> &str,
    get_children: impl Fn(&T) -> &[T],
) -> Vec<Row> {
    // Default options never hit the anchor-required error.
    render_tree_impl(
        root,
        style,
        &get_text,
        &get_children,
        &RenderOptions::default(),
    )
}

/// Renders a tree with optional wrapping and continuation-indent behavior.
/// Returns an error if hanging-indent wrapping is requested without
/// [`RenderOptions::get_continuation_anchor`].
///
/// Options are validated before the root is examined, so a `None` root with
/// invalid options still errors (matching Go).
pub fn render_tree_with_options<T>(
    root: Option<&T>,
    style: &Style,
    get_text: impl Fn(&T) -> &str,
    get_children: impl Fn(&T) -> &[T],
    opts: &RenderOptions<'_, T>,
) -> Result<Vec<Row>, RenderError> {
    if opts.continuation_indent == ContinuationIndent::Anchor
        && opts.wrap_width > 0
        && opts.get_continuation_anchor.is_none()
    {
        return Err(RenderError::AnchorCallbackRequired);
    }
    Ok(render_tree_impl(
        root,
        style,
        &get_text,
        &get_children,
        opts,
    ))
}

fn render_tree_impl<T>(
    root: Option<&T>,
    style: &Style,
    get_text: &impl Fn(&T) -> &str,
    get_children: &impl Fn(&T) -> &[T],
    opts: &RenderOptions<'_, T>,
) -> Vec<Row> {
    let Some(root) = root else {
        return Vec::new();
    };

    let sw = StyleWidths::new(style);
    let mut rows = Vec::new();
    walk(
        root,
        "",
        true,
        true,
        &sw,
        get_text,
        get_children,
        opts,
        &mut rows,
    );
    rows
}

#[allow(clippy::too_many_arguments)]
fn walk<T>(
    node: &T,
    ancestor_prefix: &str,
    is_last: bool,
    is_root: bool,
    sw: &StyleWidths<'_>,
    get_text: &impl Fn(&T) -> &str,
    get_children: &impl Fn(&T) -> &[T],
    opts: &RenderOptions<'_, T>,
    rows: &mut Vec<Row>,
) {
    let children = get_children(node);
    let last_idx = children.len().checked_sub(1);
    let text = get_text(node);

    let anchor = match opts.get_continuation_anchor {
        Some(get_anchor)
            if opts.wrap_width > 0 && opts.continuation_indent == ContinuationIndent::Anchor =>
        {
            get_anchor(node)
        }
        _ => String::new(),
    };

    rows.push(render_row(
        ancestor_prefix,
        text,
        &anchor,
        last_idx.is_some(),
        is_last,
        is_root,
        sw,
        opts,
    ));

    let next = if is_root {
        String::from(ancestor_prefix)
    } else {
        let mut s = String::from(ancestor_prefix);
        s.push_str(sw.segment(!is_last));
        s
    };
    for (i, child) in children.iter().enumerate() {
        walk(
            child,
            &next,
            Some(i) == last_idx,
            false,
            sw,
            get_text,
            get_children,
            opts,
            rows,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_row<T>(
    ancestor_prefix: &str,
    text: &str,
    anchor: &str,
    has_children: bool,
    is_last: bool,
    is_root: bool,
    sw: &StyleWidths<'_>,
    opts: &RenderOptions<'_, T>,
) -> Row {
    if opts.wrap_width <= 0 {
        return Row {
            tree_part: prefix_lines_from_ancestor(ancestor_prefix, text, is_last, is_root, sw)
                .join("\n"),
            node_text: String::from(text),
        };
    }

    let (first_prefix, continuation_prefix) = row_prefixes(ancestor_prefix, is_last, is_root, sw);
    let (tree_lines, node_lines) = wrap_row_lines(
        text,
        anchor,
        &first_prefix,
        &continuation_prefix,
        has_children,
        &sw.style.edge_link,
        opts.wrap_width,
        &opts.wrap_condition,
        opts.continuation_indent,
    );
    Row {
        tree_part: tree_lines.join("\n"),
        node_text: node_lines.join("\n"),
    }
}

fn row_prefixes(
    ancestor_prefix: &str,
    is_last: bool,
    is_root: bool,
    sw: &StyleWidths<'_>,
) -> (String, String) {
    if is_root {
        return (String::new(), String::new());
    }
    let first = format!(
        "{ancestor_prefix}{}{}",
        edge_for_row(is_last, sw.style),
        sw.style.edge_separator
    );
    let continuation = format!("{ancestor_prefix}{}", sw.continuation_segment(is_last));
    (first, continuation)
}

#[allow(clippy::too_many_arguments)]
fn wrap_row_lines(
    text: &str,
    anchor: &str,
    first_prefix: &str,
    continuation_prefix: &str,
    has_children: bool,
    child_guide: &str,
    wrap_width: i32,
    wrap_condition: &WrapCondition,
    continuation_indent: ContinuationIndent,
) -> (Vec<String>, Vec<String>) {
    let (anchor, anchor_width, text) = if continuation_indent == ContinuationIndent::Anchor
        && !anchor.is_empty()
        && text.starts_with(anchor)
    {
        (anchor, string_width(anchor), &text[anchor.len()..])
    } else {
        ("", 0, text)
    };

    let first_budget = budget(wrap_width, string_width(first_prefix) + anchor_width);
    let continuation_budget = budget(wrap_width, string_width(continuation_prefix) + anchor_width);
    let mut node_lines = wrap_chunks(text, first_budget, continuation_budget, wrap_condition);
    if node_lines.is_empty() {
        node_lines.push(String::new());
    }
    node_lines[0] = format!("{anchor}{}", node_lines[0]);

    let mut tree_lines = Vec::with_capacity(node_lines.len());
    tree_lines.push(String::from(first_prefix));
    let mut continuation_tree = String::from(continuation_prefix);
    if anchor_width > 0 {
        continuation_tree.push_str(&hanging_indent_padding(
            anchor_width,
            has_children,
            child_guide,
        ));
    }
    for _ in 1..node_lines.len() {
        tree_lines.push(continuation_tree.clone());
    }
    (tree_lines, node_lines)
}

/// `max(1, wrap_width - used)` as a usize budget.
fn budget(wrap_width: i32, used: usize) -> usize {
    (wrap_width as i64 - used as i64).max(1) as usize
}

fn hanging_indent_padding(anchor_width: usize, has_children: bool, child_guide: &str) -> String {
    if anchor_width == 0 {
        return String::new();
    }
    if !has_children || child_guide.is_empty() {
        return core::iter::repeat_n(' ', anchor_width).collect();
    }

    let mut guide = child_guide;
    let mut guide_width = string_width(guide);
    if guide_width > anchor_width {
        guide = truncate(guide, anchor_width);
        if guide.is_empty() {
            return core::iter::repeat_n(' ', anchor_width).collect();
        }
        guide_width = string_width(guide);
    }
    let mut out = String::from(guide);
    for _ in 0..anchor_width.saturating_sub(guide_width) {
        out.push(' ');
    }
    out
}

fn wrap_chunks(
    text: &str,
    first_budget: usize,
    continuation_budget: usize,
    wrap_condition: &WrapCondition,
) -> Vec<String> {
    let raw_lines: Vec<&str> = text.split('\n').collect();
    let mut lines = Vec::with_capacity(raw_lines.len());
    let mut budget = first_budget;
    for raw_line in raw_lines {
        if raw_line.is_empty() {
            lines.push(String::new());
            budget = continuation_budget;
            continue;
        }
        let mut raw_line = raw_line;
        while !raw_line.is_empty() {
            let mut raw_chunk = truncate(raw_line, budget);
            if raw_chunk.is_empty() {
                // Even one grapheme exceeds the budget: force one char of
                // progress (Go takes one rune via utf8.DecodeRuneInString).
                let size = raw_line.chars().next().map(char::len_utf8).unwrap_or(1);
                raw_chunk = &raw_line[..size];
            }
            let chunk = if wrap_condition.trim_trailing_space {
                raw_chunk.trim_end_matches([' ', '\t'])
            } else {
                raw_chunk
            };
            lines.push(String::from(chunk));
            // Advance by the untrimmed chunk length (matches Go).
            raw_line = &raw_line[raw_chunk.len()..];
            budget = continuation_budget;
        }
    }
    lines
}

fn prefix_lines_from_ancestor(
    ancestor_prefix: &str,
    text: &str,
    is_last: bool,
    is_root: bool,
    sw: &StyleWidths<'_>,
) -> Vec<String> {
    let n_lines = text.split('\n').count();
    if is_root {
        return vec![String::new(); n_lines];
    }

    let mut prefixes = Vec::with_capacity(n_lines);
    prefixes.push(format!(
        "{ancestor_prefix}{}{}",
        edge_for_row(is_last, sw.style),
        sw.style.edge_separator
    ));
    let cont = format!("{ancestor_prefix}{}", sw.continuation_segment(is_last));
    for _ in 1..n_lines {
        prefixes.push(cont.clone());
    }
    prefixes
}

/// Caches display widths for a [`Style`] so callers that need prefix width at
/// many depths avoid recomputing widths per node.
pub struct PrefixMetrics {
    w_link: usize,
    w_mid: usize,
    w_end: usize,
    w_sep: usize,
    indent: usize,
}

impl PrefixMetrics {
    /// Precomputes widths for `style` once; use
    /// [`PrefixMetrics::max_width_for_depth`] per level.
    pub fn new(style: &Style) -> Self {
        let sw = StyleWidths::new(style);
        PrefixMetrics {
            w_link: sw.w_link,
            w_mid: sw.w_mid,
            w_end: sw.w_end,
            w_sep: sw.w_sep,
            indent: sw.indent,
        }
    }

    /// The maximum display width of the prefix added by the renderer for a
    /// node at the given depth, including tree edges and the separator after
    /// the edge.
    pub fn max_width_for_depth(&self, depth: i32) -> usize {
        if depth <= 0 {
            return 0;
        }
        let seg_wide = self.w_link + self.indent;
        let ancestor_wide = (depth as usize - 1) * seg_wide;
        let first_line = ancestor_wide + self.w_mid.max(self.w_end) + self.w_sep;
        let cont_line = ancestor_wide + seg_wide;
        first_line.max(cont_line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    fn leaf(text: &str) -> Node {
        Node {
            text: text.to_string(),
            children: Vec::new(),
        }
    }

    fn sample_tree() -> Node {
        Node {
            text: "root".to_string(),
            children: vec![
                Node {
                    text: "left\ncont".to_string(),
                    children: vec![leaf("leaf-a"), leaf("leaf-b")],
                },
                leaf("right"),
            ],
        }
    }

    fn row(tree_part: &str, node_text: &str) -> Row {
        Row {
            tree_part: tree_part.to_string(),
            node_text: node_text.to_string(),
        }
    }

    fn default_style_sample_expected_rows() -> Vec<Row> {
        vec![
            row("", "root"),
            row("+- \n|  ", "left\ncont"),
            row("|  +- ", "leaf-a"),
            row("|  +- ", "leaf-b"),
            row("+- ", "right"),
        ]
    }

    #[test]
    fn render_tree_default_style() {
        let tree = sample_tree();
        let got = render_tree(
            Some(&tree),
            &default_style(),
            |n: &Node| n.text.as_str(),
            |n: &Node| n.children.as_slice(),
        );
        assert_eq!(got, default_style_sample_expected_rows());
    }

    #[test]
    fn render_tree_none_root_returns_empty() {
        let got = render_tree(
            None::<&Node>,
            &default_style(),
            |n: &Node| n.text.as_str(),
            |n: &Node| n.children.as_slice(),
        );
        assert!(got.is_empty());
    }

    #[test]
    fn render_default_style() {
        let tree = sample_tree();
        assert_eq!(
            render(Some(&tree), &default_style()),
            default_style_sample_expected_rows()
        );
    }

    #[test]
    fn render_compact_style() {
        let tree = sample_tree();
        let want = vec![
            row("", "root"),
            row("+\n|", "left\ncont"),
            row("|+", "leaf-a"),
            row("|+", "leaf-b"),
            row("+", "right"),
        ];
        assert_eq!(render(Some(&tree), &compact_style()), want);
    }

    #[test]
    fn render_negative_indent_does_not_panic() {
        let mut style = default_style();
        style.indent_size = -1;
        let tree = sample_tree();
        let _ = render(Some(&tree), &style);
    }

    #[test]
    fn render_custom_style() {
        let style = Style {
            edge_link: "..".to_string(),
            edge_mid: "=>".to_string(),
            edge_end: "--".to_string(),
            edge_separator: String::new(),
            indent_size: 1,
        };
        let tree = sample_tree();
        let want = vec![
            row("", "root"),
            row("=>\n.. ", "left\ncont"),
            row(".. =>", "leaf-a"),
            row(".. --", "leaf-b"),
            row("--", "right"),
        ];
        assert_eq!(render(Some(&tree), &style), want);
    }

    #[test]
    fn row_text_joins_tree_and_node_lines() {
        let r = row("+- \n|  ", "left\ncont");
        assert_eq!(r.text(), "+- left\n|  cont");
        assert_eq!(r.tree_part_lines(), vec!["+- ", "|  "]);
    }

    #[test]
    fn row_text_preserves_mismatched_lines() {
        assert_eq!(row("|  \n+- ", "root").text(), "|  root\n+- ");
        assert_eq!(row("+- ", "root\ncont").text(), "+- root\ncont");
    }

    #[test]
    fn prefix_metrics_max_width_for_depth_default_style() {
        let metrics = PrefixMetrics::new(&default_style());
        assert_eq!(metrics.max_width_for_depth(0), 0);
        assert_eq!(metrics.max_width_for_depth(1), 3); // "+- "
        assert_eq!(metrics.max_width_for_depth(2), 6); // "   +- "
        assert_eq!(metrics.max_width_for_depth(3), 9); // "   |  +- "
    }

    fn input_map_anchor(n: &Node) -> String {
        if n.text.starts_with("[Input] ") {
            "[Input] ".to_string()
        } else if n.text.starts_with("[Map] ") {
            "[Map] ".to_string()
        } else {
            String::new()
        }
    }

    fn render_hanging(root: &Node, wrap_width: i32) -> Vec<Row> {
        render_tree_with_options(
            Some(root),
            &default_style(),
            |n: &Node| n.text.as_str(),
            |n: &Node| n.children.as_slice(),
            &RenderOptions {
                get_continuation_anchor: Some(&input_map_anchor),
                wrap_width,
                wrap_condition: WrapCondition::default(),
                continuation_indent: ContinuationIndent::Anchor,
            },
        )
        .unwrap()
    }

    #[test]
    fn hanging_indent_anchor() {
        let root = Node {
            text: "root".to_string(),
            children: vec![leaf("[Input] Batch Scan <Row>"), leaf("tail")],
        };
        let want = vec![
            row("", "root"),
            row("+- \n|          ", "[Input] Batch Scan\n <Row>"),
            row("+- ", "tail"),
        ];
        assert_eq!(render_hanging(&root, 21), want);
    }

    #[test]
    fn hanging_indent_anchor_keeps_child_guide_non_last() {
        let root = Node {
            text: "root".to_string(),
            children: vec![
                Node {
                    text: "[Input] Batch Scan <Row>".to_string(),
                    children: vec![leaf("leaf")],
                },
                leaf("tail"),
            ],
        };
        let want = vec![
            row("", "root"),
            row("+- \n|  |       ", "[Input] Batch Scan\n <Row>"),
            row("|  +- ", "leaf"),
            row("+- ", "tail"),
        ];
        assert_eq!(render_hanging(&root, 21), want);
    }

    #[test]
    fn hanging_indent_anchor_keeps_child_guide_last() {
        let root = Node {
            text: "root".to_string(),
            children: vec![
                leaf("head"),
                Node {
                    text: "[Map] Local Distributed Union <Row>".to_string(),
                    children: vec![leaf("leaf")],
                },
            ],
        };
        let want = vec![
            row("", "root"),
            row("+- ", "head"),
            row(
                "+- \n   |     \n   |     ",
                "[Map] Local Distri\nbuted Union\n<Row>",
            ),
            row("   +- ", "leaf"),
        ];
        assert_eq!(render_hanging(&root, 21), want);
    }

    #[test]
    fn tiny_budget_keeps_utf8_valid() {
        let root = Node {
            text: "root".to_string(),
            children: vec![leaf("あい")],
        };
        let got = render_tree_with_options(
            Some(&root),
            &default_style(),
            |n: &Node| n.text.as_str(),
            |n: &Node| n.children.as_slice(),
            &RenderOptions {
                wrap_width: 4, // child budget becomes 1 after "+- "
                ..RenderOptions::default()
            },
        )
        .unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[1].node_text, "あ\nい");
    }

    #[test]
    fn skips_anchor_callback_when_unused() {
        use core::cell::Cell;
        let root = Node {
            text: "root".to_string(),
            children: vec![leaf("[Input] child")],
        };
        let calls = Cell::new(0u32);
        let get_anchor = |_: &Node| {
            calls.set(calls.get() + 1);
            "[Input] ".to_string()
        };

        // Wrapping disabled: anchor callback must not run.
        render_tree_with_options(
            Some(&root),
            &default_style(),
            |n: &Node| n.text.as_str(),
            |n: &Node| n.children.as_slice(),
            &RenderOptions {
                get_continuation_anchor: Some(&get_anchor),
                continuation_indent: ContinuationIndent::Anchor,
                ..RenderOptions::default()
            },
        )
        .unwrap();
        assert_eq!(calls.get(), 0);

        // Tree-aligned continuation: anchor callback must not run.
        render_tree_with_options(
            Some(&root),
            &default_style(),
            |n: &Node| n.text.as_str(),
            |n: &Node| n.children.as_slice(),
            &RenderOptions {
                get_continuation_anchor: Some(&get_anchor),
                wrap_width: 20,
                continuation_indent: ContinuationIndent::Tree,
                ..RenderOptions::default()
            },
        )
        .unwrap();
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn anchor_indent_requires_anchor_callback() {
        let root = Node {
            text: "root".to_string(),
            children: vec![leaf("[Input] child")],
        };
        let err = render_tree_with_options(
            Some(&root),
            &default_style(),
            |n: &Node| n.text.as_str(),
            |n: &Node| n.children.as_slice(),
            &RenderOptions {
                wrap_width: 20,
                continuation_indent: ContinuationIndent::Anchor,
                ..RenderOptions::default()
            },
        )
        .unwrap_err();
        assert_eq!(err, RenderError::AnchorCallbackRequired);
    }

    #[test]
    fn anchor_indent_without_callback_noops_when_wrapping_disabled() {
        let root = Node {
            text: "root".to_string(),
            children: vec![leaf("[Input] child")],
        };
        render_tree_with_options(
            Some(&root),
            &default_style(),
            |n: &Node| n.text.as_str(),
            |n: &Node| n.children.as_slice(),
            &RenderOptions {
                continuation_indent: ContinuationIndent::Anchor,
                ..RenderOptions::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn none_root_still_validates_options() {
        // Options are validated before the root is examined (matches Go).
        let err = render_tree_with_options(
            None::<&Node>,
            &default_style(),
            |n: &Node| n.text.as_str(),
            |n: &Node| n.children.as_slice(),
            &RenderOptions {
                wrap_width: 20,
                continuation_indent: ContinuationIndent::Anchor,
                ..RenderOptions::default()
            },
        )
        .unwrap_err();
        assert_eq!(err, RenderError::AnchorCallbackRequired);
    }
}
