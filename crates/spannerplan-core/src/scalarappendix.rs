//! Appendix sections (Predicates / Ordering / Aggregates / raw dumps)
//! printed after a rendered tree table, including scalar-variable
//! resolution. Port of `internal/scalarappendix/appendix.go`.
//!
//! The Go implementation matches `$var` references with regexes
//! (`\$[A-Za-z0-9_']+(?:\.[A-Za-z0-9_']+)*`); this port uses an equivalent
//! hand-written scanner to stay dependency-free in the `no_std` core
//! (`DESIGN.md` §6.8).

use alloc::borrow::ToOwned;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::asciitable::{self, AppendixSpec};
use crate::plantree::{RowWithPredicates, ScalarChildLink};

/// One appendix section. Go models this as a string type whose invalid
/// values are rejected at render time; the enum makes those unrepresentable,
/// so `Render`'s unsupported-section error path has no equivalent here
/// (duplicate / combination validation still applies).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Section {
    /// Predicate-like scalar links.
    Predicates,
    /// Ordering scalar links for sort operators.
    Ordering,
    /// Grouping and aggregate scalar links for aggregate operators.
    Aggregate,
    /// All typed scalar links, as a raw debug dump.
    Typed,
    /// All scalar links, including unnamed links, as a raw debug dump.
    Full,
}

impl Section {
    fn name(&self) -> &'static str {
        match self {
            Section::Predicates => "predicates",
            Section::Ordering => "ordering",
            Section::Aggregate => "aggregate",
            Section::Typed => "typed",
            Section::Full => "full",
        }
    }
}

impl core::fmt::Display for Section {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.name())
    }
}

/// An intent-based appendix section set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// Predicate-like scalar links.
    Basic,
    /// Predicate, ordering, and aggregate sections.
    Enhanced,
    /// All scalar links, including unnamed links.
    Full,
    /// No appendix sections.
    None,
}

/// Configures appendix rendering.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// Appendix sections. `None` uses the default `[Predicates]`; an
    /// explicitly empty vec renders no appendix sections.
    pub sections: Option<Vec<Section>>,
    /// Print scalar assignment variable names in semantic appendix sections.
    pub show_scalar_vars: bool,
    /// Replace direct scalar variable aliases in semantic appendix sections.
    pub resolve_scalar_vars: bool,
    /// Recursively resolve scalar variable aliases in semantic appendix
    /// sections.
    pub resolve_scalar_vars_recursive: bool,
}

/// Parse / validation error carrying the Go-compatible message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalarAppendixError(pub String);

impl core::fmt::Display for ScalarAppendixError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

fn err(msg: String) -> ScalarAppendixError {
    ScalarAppendixError(msg)
}

/// Parses one print preset name: `basic`, `enhanced`, `full`, or `none`
/// (case-insensitive). Empty input is not a preset; use [`parse_sections`]
/// for explicit-empty appendix semantics.
pub fn parse_preset(s: &str) -> Result<Preset, ScalarAppendixError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "basic" => Ok(Preset::Basic),
        "enhanced" => Ok(Preset::Enhanced),
        "full" => Ok(Preset::Full),
        "none" => Ok(Preset::None),
        _ => Err(err(format!("unknown print preset: {s:?}"))),
    }
}

impl Preset {
    /// The appendix sections for this preset.
    pub fn sections(&self) -> Vec<Section> {
        match self {
            Preset::Basic => alloc::vec![Section::Predicates],
            Preset::Enhanced => {
                alloc::vec![Section::Predicates, Section::Ordering, Section::Aggregate]
            }
            Preset::Full => alloc::vec![Section::Full],
            Preset::None => Vec::new(),
        }
    }
}

/// Parses one print-section name: `predicates`, `ordering`, `aggregate`,
/// `typed`, or `full` (case-insensitive).
pub fn parse_section(s: &str) -> Result<Section, ScalarAppendixError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "predicates" => Ok(Section::Predicates),
        "ordering" => Ok(Section::Ordering),
        "aggregate" => Ok(Section::Aggregate),
        "typed" => Ok(Section::Typed),
        "full" => Ok(Section::Full),
        _ => Err(err(format!("unknown print section: {s}"))),
    }
}

/// Parses a named preset or a comma-separated print-section list. Empty or
/// blank input returns an empty list, which renders no appendix sections.
pub fn parse_sections(s: &str) -> Result<Vec<Section>, ScalarAppendixError> {
    let trimmed = s.trim();
    let sections = if trimmed.is_empty() {
        Vec::new()
    } else if !trimmed.contains(',') {
        if let Ok(preset) = parse_preset(trimmed) {
            preset.sections()
        } else if let Ok(section) = parse_section(trimmed) {
            alloc::vec![section]
        } else {
            return Err(err(format!("unknown print preset or section: {trimmed:?}")));
        }
    } else {
        // Comma-separated input is section-list syntax. Unknown tokens
        // intentionally keep the section-list error shape used before
        // presets were added.
        let mut sections = Vec::new();
        for raw in trimmed.split(',') {
            let token = raw.trim();
            if token.is_empty() {
                return Err(err("print section must not be empty".to_owned()));
            }
            match parse_section(token) {
                Ok(section) => sections.push(section),
                Err(e) => {
                    if parse_preset(token).is_ok() {
                        return Err(err(format!(
                            "print preset {token:?} cannot be combined with section list"
                        )));
                    }
                    return Err(e);
                }
            }
        }
        sections
    };

    validate_sections(&sections)?;
    Ok(sections)
}

/// Validates an ordered print-section list: no duplicates, and the raw dumps
/// (`typed` / `full`) cannot be combined with other sections.
pub fn validate_sections(sections: &[Section]) -> Result<(), ScalarAppendixError> {
    let mut seen = BTreeSet::new();
    for section in sections {
        if !seen.insert(*section) {
            return Err(err(format!("duplicate print section: {section}")));
        }
    }
    if sections.len() > 1 {
        for section in sections {
            if matches!(section, Section::Typed | Section::Full) {
                return Err(err(format!(
                    "print section {:?} cannot be combined with other sections",
                    section.name()
                )));
            }
        }
    }
    Ok(())
}

/// Renders the configured scalar appendices without a leading separator.
/// Non-empty section parts are joined with one blank line between them.
pub fn render(rows: &[RowWithPredicates], opts: &Options) -> Result<String, ScalarAppendixError> {
    let sections = match &opts.sections {
        None => alloc::vec![Section::Predicates],
        Some(sections) => {
            validate_sections(sections)?;
            sections.clone()
        }
    };

    let resolve_vars = opts.resolve_scalar_vars || opts.resolve_scalar_vars_recursive;
    let resolver = if resolve_vars
        && sections
            .iter()
            .any(|s| matches!(s, Section::Ordering | Section::Aggregate))
    {
        Some(ScalarLinkResolver::new(rows))
    } else {
        None
    };

    let mut out = String::new();
    for section in &sections {
        let part = match section {
            Section::Full | Section::Typed => {
                let include_all = *section == Section::Full;
                render_section(rows, "Node Parameters(identified by ID):", |row| {
                    scalar_link_lines(
                        row,
                        |_, link| include_all || !link.r#type.is_empty(),
                        format_raw_scalar_link,
                    )
                })?
            }
            Section::Predicates => render_section(rows, "Predicates(identified by ID):", |row| {
                row.predicates.clone()
            })?,
            Section::Ordering => {
                let format = |link: &ScalarChildLink| -> String {
                    let desc = match &resolver {
                        Some(r) => {
                            r.format_key_scalar_link(link, opts.resolve_scalar_vars_recursive)
                        }
                        None => normalize_key_order_suffix(&link.description),
                    };
                    semantic_format(opts.show_scalar_vars, link, desc)
                };
                render_section(rows, "Ordering(identified by ID):", |row| {
                    scalar_link_lines(row, is_ordering_scalar_link, format)
                })?
            }
            Section::Aggregate => {
                let format = |link: &ScalarChildLink| -> String {
                    let desc = match &resolver {
                        Some(r) => {
                            r.format_aggregate_scalar_link(link, opts.resolve_scalar_vars_recursive)
                        }
                        None => link.description.clone(),
                    };
                    semantic_format(opts.show_scalar_vars, link, desc)
                };
                render_section(rows, "Aggregates(identified by ID):", |row| {
                    scalar_link_lines(row, is_aggregate_scalar_link, format)
                })?
            }
        };
        if !part.is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&part);
        }
    }
    Ok(out)
}

fn render_section(
    rows: &[RowWithPredicates],
    title: &str,
    items: impl Fn(&RowWithPredicates) -> Vec<String>,
) -> Result<String, ScalarAppendixError> {
    let id = |row: &RowWithPredicates| row.id as u32;
    let spec = AppendixSpec {
        title: title.to_owned(),
        // Spanner PlanNode indexes are zero-based node positions, so they
        // are non-negative when used as appendix display IDs.
        id: &id,
        items: &items,
    };
    asciitable::render_appendix(rows, &spec).map_err(|e| err(e.to_string()))
}

fn scalar_link_lines(
    row: &RowWithPredicates,
    include: impl Fn(&RowWithPredicates, &ScalarChildLink) -> bool,
    format: impl Fn(&ScalarChildLink) -> String,
) -> Vec<String> {
    // Groups keyed by link type, in first-seen order (do NOT sort; see
    // DESIGN.md §4.3).
    let mut group_by_type: BTreeMap<&str, usize> = BTreeMap::new();
    let mut groups: Vec<(&str, Vec<String>)> = Vec::new();

    for link in &row.scalar_child_links {
        if !include(row, link) {
            continue;
        }
        let group_index = *group_by_type
            .entry(link.r#type.as_str())
            .or_insert_with(|| {
                groups.push((link.r#type.as_str(), Vec::new()));
                groups.len() - 1
            });
        groups[group_index].1.push(format(link));
    }

    let mut lines = Vec::with_capacity(groups.len());
    for (typ, values) in groups {
        let joined = values.join(", ");
        if joined.is_empty() {
            continue;
        }
        if typ.is_empty() {
            lines.push(joined);
        } else {
            lines.push(format!("{typ}: {joined}"));
        }
    }
    lines
}

fn format_raw_scalar_link(link: &ScalarChildLink) -> String {
    if link.variable.is_empty() {
        link.description.clone()
    } else {
        format!("${}={}", link.variable, link.description)
    }
}

fn semantic_format(show_vars: bool, link: &ScalarChildLink, desc: String) -> String {
    if show_vars && !link.variable.is_empty() {
        format!("${}={desc}", link.variable)
    } else {
        desc
    }
}

fn is_ordering_scalar_link(row: &RowWithPredicates, link: &ScalarChildLink) -> bool {
    match row.display_name.as_str() {
        "Sort" | "Sort Limit" => link.r#type == "Key",
        "Minor Sort" | "Minor Sort Limit" => link.r#type == "MajorKey" || link.r#type == "MinorKey",
        _ => false,
    }
}

fn is_aggregate_scalar_link(row: &RowWithPredicates, link: &ScalarChildLink) -> bool {
    row.display_name == "Aggregate" && (link.r#type == "Key" || link.r#type == "Agg")
}

struct ScalarLinkResolver {
    variable_to_description: BTreeMap<String, String>,
}

impl ScalarLinkResolver {
    fn new(rows: &[RowWithPredicates]) -> Self {
        let mut variable_to_description = BTreeMap::new();
        for row in rows {
            for link in &row.scalar_child_links {
                if link.variable.is_empty() {
                    continue;
                }
                variable_to_description.insert(link.variable.clone(), link.description.clone());
            }
        }
        ScalarLinkResolver {
            variable_to_description,
        }
    }

    fn format_key_scalar_link(&self, link: &ScalarChildLink, recursive: bool) -> String {
        self.resolve_key_description(&link.description, recursive)
    }

    fn format_aggregate_scalar_link(&self, link: &ScalarChildLink, recursive: bool) -> String {
        if link.r#type == "Key" {
            self.resolve_key_description(&link.description, recursive)
        } else {
            link.description.clone()
        }
    }

    fn resolve_key_description(&self, desc: &str, recursive: bool) -> String {
        if !recursive {
            normalize_key_order_suffix(&self.resolve_direct(desc))
        } else {
            let mut seen = BTreeSet::new();
            normalize_key_order_suffix(&self.resolve_recursive(desc, &mut seen))
        }
    }

    fn resolve_direct(&self, desc: &str) -> String {
        replace_all_var_refs(desc, |var_ref| {
            let var_name = &var_ref[1..]; // strip '$'
            match self.variable_to_description.get(var_name) {
                Some(resolved) => resolved.trim().to_owned(),
                None => var_ref.to_owned(),
            }
        })
    }

    fn resolve_recursive(&self, desc: &str, seen: &mut BTreeSet<String>) -> String {
        replace_all_var_refs(desc, |var_ref| self.lookup_var_recursive(var_ref, seen))
    }

    fn lookup_var_recursive(&self, var_ref: &str, seen: &mut BTreeSet<String>) -> String {
        let Some(var_name) = var_ref.strip_prefix('$') else {
            return var_ref.to_owned();
        };
        if seen.contains(var_name) {
            return var_ref.to_owned();
        }
        let Some(desc) = self.variable_to_description.get(var_name) else {
            return var_ref.to_owned();
        };

        seen.insert(var_name.to_owned());
        let desc = desc.trim();
        let result = if is_single_var_reference(desc) {
            self.lookup_var_recursive(desc, seen)
        } else {
            self.resolve_recursive(desc, seen)
        };
        seen.remove(var_name);
        result
    }
}

/// Trailing ` (ASC)` / ` (DESC)` becomes ` ASC` / ` DESC` (after trimming
/// surrounding whitespace).
fn normalize_key_order_suffix(s: &str) -> String {
    let s = s.trim();
    for suffix in ["(ASC)", "(DESC)"] {
        if let Some(prefix) = s.strip_suffix(suffix) {
            if let Some(prefix) = prefix.strip_suffix(' ') {
                return format!("{prefix} {}", suffix.trim_matches(['(', ')']));
            }
        }
    }
    s.to_owned()
}

// --- $var reference scanning (replaces Go's regexes) -----------------------

fn is_var_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '\''
}

/// Finds the next `\$[A-Za-z0-9_']+(?:\.[A-Za-z0-9_']+)*` match at or after
/// `start`, returning its byte range.
fn find_var_ref(s: &str, start: usize) -> Option<(usize, usize)> {
    let bytes = s.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] != b'$' {
            i += 1;
            continue;
        }
        let after = &s[i + 1..];
        let run = after.chars().take_while(|&c| is_var_char(c)).count();
        if run == 0 {
            i += 1;
            continue;
        }
        // All var chars are ASCII, so char count == byte count here.
        let mut end = i + 1 + run;
        // Optional `.`-separated segments; a '.' counts only when followed
        // by at least one var char.
        loop {
            let rest = &s[end..];
            if let Some(after_dot) = rest.strip_prefix('.') {
                let seg = after_dot.chars().take_while(|&c| is_var_char(c)).count();
                if seg > 0 {
                    end += 1 + seg;
                    continue;
                }
            }
            break;
        }
        return Some((i, end));
    }
    None
}

/// Whether `s` is exactly one var reference (anchored match of the same
/// pattern).
fn is_single_var_reference(s: &str) -> bool {
    matches!(find_var_ref(s, 0), Some((0, end)) if end == s.len())
}

/// Replaces every var reference in `desc` with `f(reference)` (the
/// reference includes its `$`).
fn replace_all_var_refs(desc: &str, mut f: impl FnMut(&str) -> String) -> String {
    let mut out = String::with_capacity(desc.len());
    let mut pos = 0;
    while let Some((start, end)) = find_var_ref(desc, pos) {
        out.push_str(&desc[pos..start]);
        out.push_str(&f(&desc[start..end]));
        pos = end;
    }
    out.push_str(&desc[pos..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn link(r#type: &str, variable: &str, description: &str) -> ScalarChildLink {
        ScalarChildLink {
            r#type: r#type.to_string(),
            variable: variable.to_string(),
            description: description.to_string(),
            ..ScalarChildLink::default()
        }
    }

    fn scalar_appendix_rows() -> Vec<RowWithPredicates> {
        vec![
            RowWithPredicates {
                id: 0,
                display_name: "Sort".to_string(),
                node_text: "Sort".to_string(),
                scalar_child_links: vec![
                    link("Key", "sort_count", "$SongCount (DESC)"),
                    link("Key", "sort_genre", "$group_SongGenre'"),
                ],
                ..RowWithPredicates::default()
            },
            RowWithPredicates {
                id: 1,
                display_name: "Aggregate".to_string(),
                node_text: "Aggregate".to_string(),
                scalar_child_links: vec![
                    link("Key", "group_SongGenre'", "$group_SongGenre"),
                    link("Agg", "SongCount", "COUNT_FINAL($v1)"),
                ],
                ..RowWithPredicates::default()
            },
            RowWithPredicates {
                id: 2,
                display_name: "Filter".to_string(),
                node_text: "Filter".to_string(),
                predicates: vec!["Condition: ($SingerId = $SingerId_1)".to_string()],
                scalar_child_links: vec![link("Condition", "", "($SingerId = $SingerId_1)")],
                ..RowWithPredicates::default()
            },
            RowWithPredicates {
                id: 3,
                display_name: "Scan".to_string(),
                node_text: "Scan".to_string(),
                scalar_child_links: vec![
                    link("", "group_SongGenre", "$SongGenre"),
                    link("", "SongGenre", "SongGenre"),
                    link("", "v1", "COUNT()"),
                ],
                ..RowWithPredicates::default()
            },
        ]
    }

    // --- parse_sections (ports TestParseSections) --------------------------

    #[test]
    fn parse_sections_table() {
        let ok_cases: &[(&str, &[Section])] = &[
            ("predicates", &[Section::Predicates]),
            (
                " Predicates, Ordering, aggregate ",
                &[Section::Predicates, Section::Ordering, Section::Aggregate],
            ),
            ("basic", &[Section::Predicates]),
            (
                " Enhanced ",
                &[Section::Predicates, Section::Ordering, Section::Aggregate],
            ),
            ("full", &[Section::Full]),
            ("none", &[]),
            ("", &[]),
            (" \t ", &[]),
        ];
        for (input, want) in ok_cases {
            let got = parse_sections(input).unwrap();
            assert_eq!(got.as_slice(), *want, "input {input:?}");
        }

        let err_cases: &[(&str, &str)] = &[
            ("predicates,", "print section must not be empty"),
            ("broken", "unknown print preset or section: \"broken\""),
            (
                "basic,ordering",
                "print preset \"basic\" cannot be combined with section list",
            ),
            (
                "predicates,predicates",
                "duplicate print section: predicates",
            ),
            (
                "predicates,full",
                "print section \"full\" cannot be combined with other sections",
            ),
        ];
        for (input, want) in err_cases {
            let got = parse_sections(input).unwrap_err().to_string();
            assert!(
                got.contains(want),
                "input {input:?}: error {got:?} should contain {want:?}"
            );
        }
    }

    // --- parse_preset (ports TestParsePreset) -------------------------------

    #[test]
    fn parse_preset_table() {
        assert_eq!(parse_preset(" BASIC ").unwrap(), Preset::Basic);
        assert_eq!(
            parse_preset("").unwrap_err().to_string(),
            "unknown print preset: \"\""
        );
        assert_eq!(
            parse_preset("broken").unwrap_err().to_string(),
            "unknown print preset: \"broken\""
        );
        assert_eq!(
            parse_preset("enhanced,ordering").unwrap_err().to_string(),
            "unknown print preset: \"enhanced,ordering\""
        );
    }

    // --- render (ports TestRender & friends) --------------------------------

    #[test]
    fn render_three_sections() {
        let rows = scalar_appendix_rows();
        let opts = Options {
            sections: Some(vec![
                Section::Predicates,
                Section::Ordering,
                Section::Aggregate,
            ]),
            ..Options::default()
        };
        let want = "\
Predicates(identified by ID):
 2: Condition: ($SingerId = $SingerId_1)

Ordering(identified by ID):
 0: Key: $SongCount DESC, $group_SongGenre'

Aggregates(identified by ID):
 1: Key: $group_SongGenre
    Agg: COUNT_FINAL($v1)
";
        assert_eq!(render(&rows, &opts).unwrap(), want);
    }

    #[test]
    fn render_default_and_empty_sections() {
        let rows = scalar_appendix_rows();
        let want = "\
Predicates(identified by ID):
 2: Condition: ($SingerId = $SingerId_1)
";
        assert_eq!(render(&rows, &Options::default()).unwrap(), want);

        let opts = Options {
            sections: Some(vec![]),
            ..Options::default()
        };
        assert_eq!(render(&rows, &opts).unwrap(), "");
    }

    #[test]
    fn render_resolve_scalar_vars_recursive() {
        let rows = scalar_appendix_rows();
        let opts = Options {
            sections: Some(vec![Section::Ordering, Section::Aggregate]),
            show_scalar_vars: true,
            resolve_scalar_vars_recursive: true,
            ..Options::default()
        };
        let want = "\
Ordering(identified by ID):
 0: Key: $sort_count=COUNT_FINAL(COUNT()) DESC, $sort_genre=SongGenre

Aggregates(identified by ID):
 1: Key: $group_SongGenre'=SongGenre
    Agg: $SongCount=COUNT_FINAL($v1)
";
        assert_eq!(render(&rows, &opts).unwrap(), want);
    }

    #[test]
    fn render_resolve_scalar_vars_direct_only() {
        let rows = scalar_appendix_rows();
        let opts = Options {
            sections: Some(vec![Section::Aggregate]),
            resolve_scalar_vars: true,
            ..Options::default()
        };
        // Direct (non-recursive) resolution follows one level only:
        // $group_SongGenre -> "$SongGenre" (not further to "SongGenre").
        let want = "\
Aggregates(identified by ID):
 1: Key: $SongGenre
    Agg: COUNT_FINAL($v1)
";
        assert_eq!(render(&rows, &opts).unwrap(), want);
    }

    #[test]
    fn render_raw_sections_typed_and_full() {
        let rows = scalar_appendix_rows();

        let typed = Options {
            sections: Some(vec![Section::Typed]),
            ..Options::default()
        };
        let want_typed = "\
Node Parameters(identified by ID):
 0: Key: $sort_count=$SongCount (DESC), $sort_genre=$group_SongGenre'
 1: Key: $group_SongGenre'=$group_SongGenre
    Agg: $SongCount=COUNT_FINAL($v1)
 2: Condition: ($SingerId = $SingerId_1)
";
        assert_eq!(render(&rows, &typed).unwrap(), want_typed);

        let full = Options {
            sections: Some(vec![Section::Full]),
            ..Options::default()
        };
        let want_full = "\
Node Parameters(identified by ID):
 0: Key: $sort_count=$SongCount (DESC), $sort_genre=$group_SongGenre'
 1: Key: $group_SongGenre'=$group_SongGenre
    Agg: $SongCount=COUNT_FINAL($v1)
 2: Condition: ($SingerId = $SingerId_1)
 3: $group_SongGenre=$SongGenre, $SongGenre=SongGenre, $v1=COUNT()
";
        assert_eq!(render(&rows, &full).unwrap(), want_full);
    }

    #[test]
    fn render_rejects_invalid_section_combination() {
        let opts = Options {
            sections: Some(vec![Section::Predicates, Section::Full]),
            ..Options::default()
        };
        let got = render(&[], &opts).unwrap_err().to_string();
        assert!(got.contains("cannot be combined"), "error: {got}");
    }

    // --- var-ref scanner ----------------------------------------------------

    #[test]
    fn var_ref_scanner_matches_go_regex_semantics() {
        // Basic reference with trailing non-var chars.
        assert_eq!(find_var_ref("$abc (DESC)", 0), Some((0, 4)));
        // Apostrophe and dot-separated segments are part of the reference.
        assert_eq!(find_var_ref("$a'.b_2 rest", 0), Some((0, 7)));
        // A '$' not followed by a var char is skipped.
        assert_eq!(find_var_ref("$$x", 0), Some((1, 3)));
        // A trailing dot is not consumed.
        assert_eq!(find_var_ref("$a.", 0), Some((0, 2)));
        assert_eq!(find_var_ref("no refs", 0), None);

        assert!(is_single_var_reference("$abc"));
        assert!(is_single_var_reference("$a.b"));
        assert!(!is_single_var_reference("$abc "));
        assert!(!is_single_var_reference("x$abc"));
        assert!(!is_single_var_reference("$"));

        assert_eq!(
            replace_all_var_refs("($a = $b)", |r| r[1..].to_ascii_uppercase()),
            "(A = B)"
        );
    }

    #[test]
    fn normalize_key_order_suffix_cases() {
        assert_eq!(normalize_key_order_suffix("$x (DESC)"), "$x DESC");
        assert_eq!(normalize_key_order_suffix("$x (ASC)"), "$x ASC");
        assert_eq!(normalize_key_order_suffix(" $x "), "$x");
        // No space before the suffix: left alone.
        assert_eq!(normalize_key_order_suffix("$x(DESC)"), "$x(DESC)");
        assert_eq!(normalize_key_order_suffix("(DESC)"), "(DESC)");
    }

    #[test]
    fn recursive_resolution_guards_against_cycles() {
        let rows = vec![RowWithPredicates {
            id: 0,
            display_name: "Sort".to_string(),
            scalar_child_links: vec![link("Key", "a", "$b"), link("Key", "b", "$a")],
            ..RowWithPredicates::default()
        }];
        let opts = Options {
            sections: Some(vec![Section::Ordering]),
            resolve_scalar_vars_recursive: true,
            ..Options::default()
        };
        // $b -> $a -> $b(cycle; stops) — must terminate, not overflow.
        let got = render(&rows, &opts).unwrap();
        assert!(got.contains("Key:"), "output: {got}");
    }
}
