//! Display-width, truncation, and fill helpers, replacing the width/truncate
//! parts of Go's `apstndb/go-tabwrap` (see `DESIGN.md` §6.3). Iterates
//! grapheme clusters (not `char`s) so multi-codepoint clusters — combining
//! marks, some emoji — are never split mid-cluster by [`truncate`].

use alloc::string::String;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Sum of display-cell widths of `s`'s grapheme clusters. Handles East Asian
/// wide characters (width 2) and zero-width combining marks (width 0).
pub fn string_width(s: &str) -> usize {
    s.graphemes(true).map(UnicodeWidthStr::width).sum()
}

/// Returns the longest prefix of `s`, on a grapheme-cluster boundary, whose
/// display width does not exceed `budget`.
///
/// Returns `""` if even the first grapheme cluster's width exceeds `budget`;
/// callers that need to make forward progress in that case (see
/// `treerender.rs`'s `wrap_chunks`, a later phase) fall back to force-taking
/// one `char`.
pub fn truncate(s: &str, budget: usize) -> &str {
    let mut width = 0usize;
    let mut end = 0usize;
    for g in s.graphemes(true) {
        let w = UnicodeWidthStr::width(g);
        if width + w > budget {
            break;
        }
        width += w;
        end += g.len();
    }
    &s[..end]
}

/// Left-pads `s` with spaces so its total display width is at least `width`
/// (i.e. right-aligns `s` within a field of `width` columns). Mirrors Go
/// `tabwrap.FillLeft`. Returns `s` unchanged (as an owned `String`) if it is
/// already at least `width` columns wide.
pub fn fill_left(s: &str, width: usize) -> String {
    let w = string_width(s);
    let pad = width.saturating_sub(w);
    let mut out = String::with_capacity(pad + s.len());
    for _ in 0..pad {
        out.push(' ');
    }
    out.push_str(s);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn string_width_ascii() {
        assert_eq!(string_width(""), 0);
        assert_eq!(string_width("hello"), 5);
    }

    #[test]
    fn string_width_east_asian_wide() {
        // Each of these three CJK characters occupies 2 display columns.
        assert_eq!(string_width("日本語"), 6);
    }

    #[test]
    fn string_width_combining_mark_grapheme_cluster() {
        // "e" + COMBINING ACUTE ACCENT (U+0301) is one grapheme cluster
        // rendered as a single "é" cell.
        assert_eq!(string_width("e\u{0301}"), 1);
    }

    #[test]
    fn truncate_ascii_budget() {
        assert_eq!(truncate("hello world", 5), "hello");
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 0), "");
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn truncate_never_splits_a_wide_character() {
        // Budget 3 with 2-wide characters: the first character (width 2) fits
        // (running width 2 <= 3), but the second would bring it to 4 > 3, so
        // truncation stops after exactly one character, not mid-character.
        assert_eq!(truncate("日本語", 3), "日");
        assert_eq!(truncate("日本語", 4), "日本");
    }

    #[test]
    fn truncate_never_splits_a_combining_mark_grapheme_cluster() {
        // If this iterated by `char` instead of grapheme cluster, a budget of
        // 1 could return just "e", stranding the combining accent for the
        // next chunk and corrupting the rendered glyph.
        assert_eq!(truncate("e\u{0301}f", 1), "e\u{0301}");
    }

    #[test]
    fn fill_left_pads_to_width() {
        assert_eq!(fill_left("42", 5), "   42");
        assert_eq!(fill_left("hello", 3), "hello");
        assert_eq!(fill_left("", 3), "   ");
    }

    #[test]
    fn fill_left_uses_display_width_not_byte_or_char_count() {
        // "日" is 1 char / 3 bytes but 2 display columns; fill_left(_, 5)
        // should pad by 3 spaces (5 - 2), not 4 (5 - 1 char) or 2 (5 - 3 bytes).
        assert_eq!(fill_left("日", 5), "   日".to_string());
    }
}
