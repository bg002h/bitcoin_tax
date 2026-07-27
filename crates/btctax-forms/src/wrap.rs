//! Helvetica-Bold 8pt text measurement + greedy word-wrap for Form 8275's Part II ("Detailed
//! Explanation") + Part IV ("Explanations (continued from Parts I and/or II)") continuation lines.
//!
//! Every continuation field (`p1-t80[0]`..`p1-t85[0]`, `p2-t1[0]`..`p2-t27[0]`) is a SINGLE-LINE,
//! non-multiline, `DoNotScroll` AcroForm text field with no `/MaxLen` — nothing at the PDF-data level
//! stops an over-long `/V` from being WRITTEN (see `form8275.rs`'s doc comment for why `verify_flat`
//! cannot see this), but a viewer honoring the widget's own `/Rect` and `/DA`
//! (`/HelveticaLTStd-Bold 8.00 Tf`, verified against the bundled Rev. 10-2024 asset) can only DISPLAY
//! what fits in that box. This module measures at that same font/size so the FILL decides where to
//! break a line, instead of leaving it to an unpredictable, silent, per-viewer clip.
//!
//! T-f8275-part-ii-overflow.

/// Helvetica-Bold glyph widths (Adobe Core-14 AFM metrics, 1000 units/em), ASCII printable range
/// 0x20..=0x7E, `code - 0x20` indexed. This is the same public-domain table every PDF toolchain ships
/// for the standard-14 fonts (which carry no embedded glyph program of their own to measure instead).
const HELV_BOLD_ASCII: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 278, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611, 975, 722, 722, 722, 722, 667,
    611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 333, 278, 333, 584, 556, 278, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556,
    278, 889, 611, 611, 611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
];

/// A handful of non-ASCII glyphs this codebase's own domain text already uses (`Part1Item::line`
/// carries an em dash) or a filer's word processor commonly substitutes ("smart" punctuation).
fn extra_glyph_width(c: char) -> Option<u16> {
    Some(match c {
        '\u{2014}' => 1000,             // em dash
        '\u{2013}' => 556,              // en dash
        '\u{2018}' | '\u{2019}' => 278, // single quotes / apostrophe
        '\u{201C}' | '\u{201D}' => 500, // double quotes
        '\u{00A7}' => 556,              // section sign
        '\u{2022}' => 350,              // bullet
        '\u{2026}' => 1000,             // ellipsis
        _ => return None,
    })
}

/// The widest glyph in the whole Helvetica-Bold Core-14 metrics (1000/1000 em — several glyphs share
/// it, e.g. em dash and ellipsis). Assigned to any character neither table above knows about —
/// deliberately the WIDEST possible, so an unmodeled glyph can only make wrapping MORE conservative
/// (an earlier line break), never under-measure a line that then overflows its field's physical box.
const UNKNOWN_GLYPH_WIDTH: u16 = 1000;

fn glyph_width(c: char) -> u16 {
    if c.is_ascii() {
        let code = c as u32;
        if (0x20..=0x7E).contains(&code) {
            return HELV_BOLD_ASCII[(code - 0x20) as usize];
        }
    }
    extra_glyph_width(c).unwrap_or(UNKNOWN_GLYPH_WIDTH)
}

/// Width, in PDF points, of `s` set in Helvetica-Bold at `size_pt`.
pub(crate) fn text_width_pts(s: &str, size_pt: f32) -> f32 {
    let units: u32 = s.chars().map(|c| glyph_width(c) as u32).sum();
    units as f32 * size_pt / 1000.0
}

/// Float slop for a `<=` width comparison — text measurement here is exact arithmetic on integer glyph
/// units, so this only absorbs f32 rounding, not any real uncertainty.
const EPS: f32 = 0.01;

/// Greedily word-wrap `text` into lines no wider than `max_width_pts` at `size_pt`. Breaks only on
/// whitespace (any run of whitespace, including newlines, collapses to a single space) — this never
/// hyphenates or splits a word, so a run of non-whitespace characters wider than `max_width_pts` on its
/// own becomes an over-width line by itself; [`wrap_to_capacity`] checks for and refuses that.
fn word_wrap(text: &str, max_width_pts: f32, size_pt: f32) -> Vec<String> {
    let space_w = text_width_pts(" ", size_pt);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_w = 0.0_f32;
    for word in text.split_whitespace() {
        let word_w = text_width_pts(word, size_pt);
        if current.is_empty() {
            current = word.to_string();
            current_w = word_w;
        } else if current_w + space_w + word_w <= max_width_pts + EPS {
            current.push(' ');
            current.push_str(word);
            current_w += space_w + word_w;
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
            current_w = word_w;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Wrap `text` across up to `capacity` lines, each no wider than `max_width_pts` at `size_pt`pt
/// Helvetica-Bold — and FAIL CLOSED (never truncate) if it does not fit, either because more than
/// `capacity` lines are needed, or because a single unbreakable run of non-whitespace text is wider
/// than `max_width_pts` on its own (so no number of lines would ever hold it — every continuation line
/// this is called with shares one conservative width, so "too wide for this line" means "too wide for
/// every line").
///
/// On failure, returns how many lines the text actually needs (or, for the unbreakable-run case, a
/// count guaranteed to exceed `capacity`) — mirrors the shape of the Part I row-count
/// [`crate::error::FormsError::Overflow`] refusal (`rows` vs `capacity`).
pub(crate) fn wrap_to_capacity(
    text: &str,
    capacity: usize,
    max_width_pts: f32,
    size_pt: f32,
) -> Result<Vec<String>, usize> {
    let lines = word_wrap(text, max_width_pts, size_pt);
    let too_wide = lines
        .iter()
        .any(|l| text_width_pts(l, size_pt) > max_width_pts + EPS);
    if too_wide {
        // At least one line — necessarily a single word too wide to share a line with anything else —
        // could never fit no matter how many lines were on offer. Report a count guaranteed to read as
        // "will not fit" regardless of what `capacity` is, rather than a possibly-in-range line count.
        return Err(lines.len().max(capacity + 1));
    }
    if lines.len() > capacity {
        return Err(lines.len());
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_known_widths() {
        // "MM" (two 833-unit glyphs) at 8pt = 2 * 833 * 8 / 1000 = 13.328pt.
        assert!((text_width_pts("MM", 8.0) - 13.328).abs() < 0.01);
        // A capital-heavy string measures wider than the same length of narrow lowercase.
        assert!(text_width_pts("WWWW", 8.0) > text_width_pts("iiii", 8.0));
        assert_eq!(text_width_pts("", 8.0), 0.0);
    }

    #[test]
    fn unknown_glyphs_use_the_widest_known_width_never_narrower() {
        // A made-up private-use character must not be measured as narrower than any real glyph in the
        // table — this is the "never under-measure" guarantee `UNKNOWN_GLYPH_WIDTH` exists for.
        let widest_known = *HELV_BOLD_ASCII.iter().max().unwrap();
        assert!(UNKNOWN_GLYPH_WIDTH as u32 >= widest_known as u32);
        assert_eq!(glyph_width('\u{E000}'), UNKNOWN_GLYPH_WIDTH);
    }

    #[test]
    fn word_wrap_never_splits_a_word_and_preserves_every_word_in_order() {
        let text = "one two three four five six seven eight nine ten";
        let lines = word_wrap(text, 60.0, 8.0); // narrow: forces several short lines
        assert!(lines.len() > 1, "fixture premise: must actually wrap");
        let rejoined: Vec<&str> = lines.iter().flat_map(|l| l.split_whitespace()).collect();
        let original: Vec<&str> = text.split_whitespace().collect();
        assert_eq!(
            rejoined, original,
            "no word may be dropped, split, or reordered"
        );
        for line in &lines {
            assert!(
                text_width_pts(line, 8.0) <= 60.0 + EPS,
                "line {line:?} exceeds the 60pt budget"
            );
        }
    }

    #[test]
    fn whitespace_runs_including_newlines_collapse_to_single_spaces() {
        let lines = word_wrap("a\n\nb   c\td", 1000.0, 8.0);
        assert_eq!(lines, vec!["a b c d".to_string()]);
    }

    #[test]
    fn wrap_to_capacity_fits_within_the_line_budget() {
        let text = "one two three four five six seven eight nine ten";
        let lines = wrap_to_capacity(text, 10, 60.0, 8.0).expect("must fit in 10 lines of 60pt");
        assert!(lines.len() <= 10);
        let rejoined: Vec<&str> = lines.iter().flat_map(|l| l.split_whitespace()).collect();
        assert_eq!(rejoined, text.split_whitespace().collect::<Vec<_>>());
    }

    #[test]
    fn wrap_to_capacity_fails_closed_when_more_lines_are_needed_than_available() {
        let text = "one two three four five six seven eight nine ten";
        // Force each word onto its own line (word width << 60pt but each line holds ~1 word at a much
        // smaller budget), then offer fewer lines than the 10 words need.
        let err = wrap_to_capacity(text, 3, 20.0, 8.0).expect_err("must not fit in only 3 lines");
        assert!(
            err > 3,
            "reported rows-needed {err} must exceed the 3-line capacity"
        );
    }

    #[test]
    fn wrap_to_capacity_fails_closed_on_a_single_word_wider_than_any_line() {
        // A single 50-character unbreakable token — wider than a narrow line budget no matter how many
        // lines are offered.
        let token = "a".repeat(50);
        let err = wrap_to_capacity(&token, 100, 20.0, 8.0)
            .expect_err("an unbreakable token wider than the line budget can never fit");
        assert!(
            err > 100,
            "an unfittable single token must report a rows-needed count that exceeds capacity (100), \
             got {err}"
        );
    }

    #[test]
    fn wrap_to_capacity_of_empty_text_yields_no_lines() {
        assert_eq!(
            wrap_to_capacity("", 33, 518.4, 8.0).unwrap(),
            Vec::<String>::new()
        );
    }
}
