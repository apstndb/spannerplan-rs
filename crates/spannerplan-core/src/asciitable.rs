//! ASCII table and appendix rendering for caller-owned rows. Port of
//! `asciitable/asciitable.go`.
//!
//! The Go implementation delegates the table to `olekukonko/tablewriter`
//! (StyleASCII, TrimSpace off, header auto-format off, header left-aligned,
//! per-column row alignment, no auto-wrap); this module reimplements exactly
//! that output shape so golden parity holds without a table dependency. See
//! `DESIGN.md` §6.7.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::textwidth::{fill_left, string_width};

/// Horizontal alignment for a rendered column. Go models this as a string
/// with a render-time validity check; a Rust enum makes invalid alignments
/// unrepresentable, so that error path has no equivalent here.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Alignment {
    /// Left-align cell text (Go's zero value).
    #[default]
    Left,
    /// Right-align cell text.
    Right,
    /// Center cell text.
    Center,
}

/// One rendered table column. `cell` returns the cell text for `(row, row
/// index)`; it is a required reference, so Go's nil-`Cell` error path has no
/// equivalent here.
pub struct Column<'a, T> {
    /// The column header text. Rendered verbatim (header auto-formatting is
    /// off) and always left-aligned regardless of [`Column::alignment`].
    pub header: String,
    /// The cell alignment for data rows.
    pub alignment: Alignment,
    /// Returns the rendered cell text for each row.
    pub cell: &'a dyn Fn(&T, usize) -> String,
}

/// The columns of an ASCII table.
pub struct TableSpec<'a, T> {
    /// The ordered list of table columns.
    pub columns: Vec<Column<'a, T>>,
}

/// How appendices read row IDs and item lines. `id` and `items` are required
/// references, so Go's nil-callback error paths have no equivalent here.
pub struct AppendixSpec<'a, T> {
    /// Printed before item lines. Must be non-empty.
    pub title: String,
    /// Returns the non-negative display ID used in the appendix.
    pub id: &'a dyn Fn(&T) -> u32,
    /// Returns the item lines associated with the row.
    pub items: &'a dyn Fn(&T) -> Vec<String>,
}

/// Errors from [`render_table`] / [`render_appendix`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsciiTableError {
    /// A table spec must contain at least one column.
    NoColumns,
    /// An appendix spec has an empty title.
    EmptyAppendixTitle,
}

impl core::fmt::Display for AsciiTableError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AsciiTableError::NoColumns => {
                f.write_str("table spec must contain at least one column")
            }
            AsciiTableError::EmptyAppendixTitle => f.write_str("appendix spec has empty Title"),
        }
    }
}

/// Renders `rows` as an ASCII table using `spec`. Output ends with a
/// trailing newline, matching tablewriter.
///
/// Layout rules (mirroring the tablewriter configuration Go uses):
/// - column width = max display width over the header and every visual line
///   of every cell in that column;
/// - borders are `+--...--+` with `width + 2` dashes per column;
/// - each cell is padded to the column width (per [`Column::alignment`];
///   headers always left) and surrounded by one space on each side;
/// - a cell containing `\n` occupies multiple visual lines within its
///   logical row; other columns are blank-padded on the extra lines;
/// - structure: top border, header, separator, data rows, bottom border.
pub fn render_table<T>(rows: &[T], spec: &TableSpec<'_, T>) -> Result<String, AsciiTableError> {
    if spec.columns.is_empty() {
        return Err(AsciiTableError::NoColumns);
    }

    // Evaluate every cell exactly once.
    let mut table_cells: Vec<Vec<String>> = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let mut row_cells = Vec::with_capacity(spec.columns.len());
        for col in &spec.columns {
            row_cells.push((col.cell)(row, i));
        }
        table_cells.push(row_cells);
    }

    // Column widths over headers and all cell lines.
    let mut widths: Vec<usize> = spec
        .columns
        .iter()
        .map(|c| string_width(&c.header))
        .collect();
    for row_cells in &table_cells {
        for (j, cell) in row_cells.iter().enumerate() {
            for line in cell.split('\n') {
                widths[j] = widths[j].max(string_width(line));
            }
        }
    }

    let border = {
        let mut b = String::new();
        for w in &widths {
            b.push('+');
            for _ in 0..w + 2 {
                b.push('-');
            }
        }
        b.push('+');
        b.push('\n');
        b
    };

    let mut out = String::new();
    out.push_str(&border);

    // Header (always left-aligned).
    out.push_str(&render_visual_line(
        &spec
            .columns
            .iter()
            .map(|c| c.header.as_str())
            .collect::<Vec<_>>(),
        &widths,
        &spec
            .columns
            .iter()
            .map(|_| Alignment::Left)
            .collect::<Vec<_>>(),
    ));
    out.push_str(&border);

    let alignments: Vec<Alignment> = spec.columns.iter().map(|c| c.alignment).collect();
    for row_cells in &table_cells {
        let split: Vec<Vec<&str>> = row_cells.iter().map(|c| c.split('\n').collect()).collect();
        let n_lines = split.iter().map(Vec::len).max().unwrap_or(1);
        for line_idx in 0..n_lines {
            let line_cells: Vec<&str> = split
                .iter()
                .map(|lines| lines.get(line_idx).copied().unwrap_or(""))
                .collect();
            out.push_str(&render_visual_line(&line_cells, &widths, &alignments));
        }
    }

    out.push_str(&border);
    Ok(out)
}

fn render_visual_line(cells: &[&str], widths: &[usize], alignments: &[Alignment]) -> String {
    let mut out = String::new();
    for ((cell, &width), &alignment) in cells.iter().zip(widths).zip(alignments) {
        out.push_str("| ");
        out.push_str(&pad(cell, width, alignment));
        out.push(' ');
    }
    out.push('|');
    out.push('\n');
    out
}

fn pad(s: &str, width: usize, alignment: Alignment) -> String {
    let sw = string_width(s);
    let total = width.saturating_sub(sw);
    let (left, right) = match alignment {
        Alignment::Left => (0, total),
        Alignment::Right => (total, 0),
        Alignment::Center => (total / 2, total - total / 2),
    };
    let mut out = String::with_capacity(s.len() + total);
    for _ in 0..left {
        out.push(' ');
    }
    out.push_str(s);
    for _ in 0..right {
        out.push(' ');
    }
    out
}

/// Formats an appendix for rows with associated items. Returns `""` when no
/// row has items (no title is printed). `id` and `items` are read exactly
/// once per row; the max-ID width is computed over ALL rows' IDs, including
/// rows without items (matching Go).
pub fn render_appendix<T>(
    rows: &[T],
    spec: &AppendixSpec<'_, T>,
) -> Result<String, AsciiTableError> {
    if spec.title.is_empty() {
        return Err(AsciiTableError::EmptyAppendixTitle);
    }

    let mut max_id_length = 0usize;
    let mut resolved: Vec<(u32, Vec<String>)> = Vec::new();
    for row in rows {
        let id = (spec.id)(row);
        let items = (spec.items)(row);
        max_id_length = max_id_length.max(id.to_string().len());
        if !items.is_empty() {
            resolved.push((id, items));
        }
    }
    if resolved.is_empty() {
        return Ok(String::new());
    }

    let mut out = String::new();
    out.push_str(&spec.title);
    out.push('\n');
    for (id, items) in &resolved {
        for (i, item) in items.iter().enumerate() {
            let id_part = if i == 0 {
                format!("{id}:")
            } else {
                String::new()
            };
            let prefix = fill_left(&id_part, max_id_length + 1);
            out.push_str(&format!(" {prefix} {item}\n"));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    struct TestRow {
        id: u32,
        id_text: &'static str,
        text: &'static str,
        rows: &'static str,
        predicates: Vec<String>,
    }

    fn test_row(id: u32, id_text: &'static str, text: &'static str, rows: &'static str) -> TestRow {
        TestRow {
            id,
            id_text,
            text,
            rows,
            predicates: Vec::new(),
        }
    }

    fn pred_row(id: u32, text: &'static str, predicates: &[&str]) -> TestRow {
        TestRow {
            id,
            id_text: "",
            text,
            rows: "",
            predicates: predicates.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn id_column<'a>() -> Column<'a, TestRow> {
        Column {
            header: "ID".to_string(),
            alignment: Alignment::Right,
            cell: &|row: &TestRow, _| row.id_text.to_string(),
        }
    }

    fn operator_column<'a>() -> Column<'a, TestRow> {
        Column {
            header: "Operator".to_string(),
            alignment: Alignment::Left,
            cell: &|row: &TestRow, _| row.text.to_string(),
        }
    }

    fn appendix_spec<'a>(title: &str) -> AppendixSpec<'a, TestRow> {
        AppendixSpec {
            title: title.to_string(),
            id: &|row: &TestRow| row.id,
            items: &|row: &TestRow| row.predicates.clone(),
        }
    }

    #[test]
    fn render_table_basic() {
        let rows = vec![
            test_row(1, "1", "Root", "10"),
            test_row(2, "2", "+- Child", "3"),
        ];
        let spec = TableSpec {
            columns: vec![
                id_column(),
                operator_column(),
                Column {
                    header: "Rows".to_string(),
                    alignment: Alignment::Right,
                    cell: &|row: &TestRow, _| row.rows.to_string(),
                },
            ],
        };
        let got = render_table(&rows, &spec).unwrap();
        let want = "\
+----+----------+------+
| ID | Operator | Rows |
+----+----------+------+
|  1 | Root     |   10 |
|  2 | +- Child |    3 |
+----+----------+------+
";
        assert_eq!(got, want);
    }

    #[test]
    fn render_table_row_index() {
        let rows = vec![test_row(1, "1", "Root", ""), test_row(2, "2", "Child", "")];
        let spec = TableSpec {
            columns: vec![
                id_column(),
                Column {
                    header: "Index".to_string(),
                    alignment: Alignment::Right,
                    cell: &|_: &TestRow, index| index.to_string(),
                },
            ],
        };
        let got = render_table(&rows, &spec).unwrap();
        let want = "\
+----+-------+
| ID | Index |
+----+-------+
|  1 |     0 |
|  2 |     1 |
+----+-------+
";
        assert_eq!(got, want);
    }

    #[test]
    fn render_table_preserves_multiline_cells() {
        let rows = vec![test_row(1, "1", "Root\n+- Child", "")];
        let spec = TableSpec {
            columns: vec![id_column(), operator_column()],
        };
        let got = render_table(&rows, &spec).unwrap();
        let want = "\
+----+----------+
| ID | Operator |
+----+----------+
|  1 | Root     |
|    | +- Child |
+----+----------+
";
        assert_eq!(got, want);
    }

    #[test]
    fn render_table_rejects_empty_columns() {
        let rows: Vec<TestRow> = Vec::new();
        let spec: TableSpec<'_, TestRow> = TableSpec { columns: vec![] };
        assert_eq!(
            render_table(&rows, &spec).unwrap_err(),
            AsciiTableError::NoColumns
        );
    }

    #[test]
    fn render_table_wide_chars_align() {
        // "日本語" is 6 display columns; alignment must use display width.
        let rows = vec![
            test_row(1, "1", "日本語", ""),
            test_row(2, "2", "ascii", ""),
        ];
        let spec = TableSpec {
            columns: vec![id_column(), operator_column()],
        };
        let got = render_table(&rows, &spec).unwrap();
        let want = "\
+----+----------+
| ID | Operator |
+----+----------+
|  1 | 日本語   |
|  2 | ascii    |
+----+----------+
";
        assert_eq!(got, want);
    }

    #[test]
    fn render_appendix_basic() {
        let rows = vec![
            pred_row(3, "Filter", &["Filter: a = 1", "Expression: b"]),
            pred_row(12, "Scan", &["Seek Condition: k = 1"]),
        ];
        let got = render_appendix(&rows, &appendix_spec("Predicates(identified by ID):")).unwrap();
        let want = "\
Predicates(identified by ID):
  3: Filter: a = 1
     Expression: b
 12: Seek Condition: k = 1
";
        assert_eq!(got, want);
    }

    #[test]
    fn render_appendix_custom_title() {
        let rows = vec![pred_row(3, "Filter", &["Filter: a = 1"])];
        let got = render_appendix(&rows, &appendix_spec("Filters:")).unwrap();
        assert_eq!(got, "Filters:\n 3: Filter: a = 1\n");
    }

    #[test]
    fn render_appendix_multi_digit_ids() {
        let rows = vec![
            pred_row(3, "Filter", &["Filter: a = 1", "Expression: b"]),
            pred_row(120, "Scan", &["Seek Condition: k = 1"]),
        ];
        let got = render_appendix(&rows, &appendix_spec("Predicates(identified by ID):")).unwrap();
        let want = "\
Predicates(identified by ID):
   3: Filter: a = 1
      Expression: b
 120: Seek Condition: k = 1
";
        assert_eq!(got, want);
    }

    #[test]
    fn render_appendix_none_returns_empty() {
        let rows = vec![pred_row(1, "Root", &[])];
        let got = render_appendix(&rows, &appendix_spec("Predicates(identified by ID):")).unwrap();
        assert_eq!(got, "");
    }

    #[test]
    fn render_appendix_reads_each_row_once() {
        use core::cell::Cell;
        let rows = vec![
            pred_row(1, "Root", &[]),
            pred_row(2, "Filter", &["Filter: true"]),
        ];
        let id_calls = Cell::new(0usize);
        let items_calls = Cell::new(0usize);
        let id = |row: &TestRow| {
            id_calls.set(id_calls.get() + 1);
            row.id
        };
        let items = |row: &TestRow| {
            items_calls.set(items_calls.get() + 1);
            row.predicates.clone()
        };
        let spec = AppendixSpec {
            title: "Predicates(identified by ID):".to_string(),
            id: &id,
            items: &items,
        };
        render_appendix(&rows, &spec).unwrap();
        assert_eq!(id_calls.get(), rows.len());
        assert_eq!(items_calls.get(), rows.len());
    }

    #[test]
    fn render_appendix_rejects_empty_title() {
        let rows = vec![pred_row(1, "Root", &["Filter: true"])];
        assert_eq!(
            render_appendix(&rows, &appendix_spec("")).unwrap_err(),
            AsciiTableError::EmptyAppendixTitle
        );
    }

    #[test]
    fn render_appendix_max_id_width_counts_rows_without_items() {
        // Row 100 has no items, but its ID still widens the alignment column
        // (Go computes maxIDLength across every row).
        let rows = vec![
            pred_row(3, "Filter", &["Filter: a = 1"]),
            pred_row(100, "Root", &[]),
        ];
        let got = render_appendix(&rows, &appendix_spec("Predicates(identified by ID):")).unwrap();
        let want = "\
Predicates(identified by ID):
   3: Filter: a = 1
";
        assert_eq!(got, want);
    }
}
