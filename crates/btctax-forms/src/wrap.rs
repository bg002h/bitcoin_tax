//! Helvetica-Bold 8pt text measurement + paragraph-aware word-wrap for Form 8275's Part II line 1 +
//! Part IV ("Explanations (continued from Parts I and/or II)") continuation lines.
//!
//! Every continuation field (`p1-t80[0]` on page 1, `p2-t1[0]`..`p2-t27[0]` on page 2) is a
//! SINGLE-LINE, non-multiline, `DoNotScroll` AcroForm text field with no `/MaxLen` — nothing at the
//! PDF-data level stops an over-long `/V` from being WRITTEN (see `form8275.rs`'s doc comment for why
//! `verify_flat` cannot see this), but a viewer honoring the widget's own `/Rect` and `/DA`
//! (`/HelveticaLTStd-Bold 8.00 Tf`, verified against the bundled Rev. 10-2024 asset) can only DISPLAY
//! what fits in that box. This module measures at that same font/size so the FILL decides where to
//! break a line, instead of leaving it to an unpredictable, silent, per-viewer clip.
//!
//! ★ **Only Part II's OWN line 1 is ever written** (`form8275.rs`'s module doc explains why: the
//! bundled PDF's XFA numbers Part II lines 1-6 beside Part I rows 1-6, so spreading one combined
//! narrative across them misattributes sentence fragments to items they do not explain). Everything
//! past line 1 goes to Part IV, prefixed on its first line with the IRS-required cross-reference
//! ([`PART_IV_CROSS_REFERENCE_PREFIX`]). See [`wrap_part_ii`].
//!
//! T-f8275-part-ii-overflow (round 2 — the two-lens review that produced findings 1/3/4/5/6/7).

/// Helvetica-Bold glyph widths, ASCII printable range 0x20..=0x7E (`code - 0x20` indexed) — extracted
/// DIRECTLY from the bundled `f8275.pdf`'s own embedded font (`/DR/Font/HelveticaLTStd-Bold`, a
/// `/Type1` font with `/Encoding /WinAnsiEncoding`, `/FirstChar 0`/`/LastChar 255`, and a full 256-entry
/// `/Widths` array covering this whole range directly) — NOT the generic Adobe Core-14 Helvetica-Bold
/// AFM table. The two differ at two codepoints this asset's own font actually ships: `'` (0x27) is 238
/// here vs the generic AFM's 278 (this table is narrower — safe, the over-measuring direction), and
/// `` ` `` (0x60) is 333 here vs the generic AFM's 278 (this table is WIDER — the generic AFM
/// under-measured backtick, the dangerous direction that can let a line overflow its box). Re-derive
/// with the one-off dump documented in `design/f8275-part-ii-overflow/BUILD-REPORT.md` if the bundled
/// asset ever changes.
const HELV_BOLD_ASCII: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611, 975, 722, 722, 722, 722, 667,
    611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 333, 278, 333, 584, 556, 333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556,
    278, 889, 611, 611, 611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
];

/// A handful of non-ASCII glyphs this codebase's own domain text already uses (`Part1Item::line`
/// carries an em dash) or a filer's word processor commonly substitutes ("smart" punctuation). Widths
/// are the SAME bundled asset's `/Widths` entries at each glyph's `/WinAnsiEncoding` byte code (0x91
/// quoteleft, 0x92 quoteright, 0x93 quotedblleft, 0x94 quotedblright, 0x95 bullet, 0x96 endash, 0x97
/// emdash, 0x85 ellipsis, 0xA7 section) — all independently confirmed to match the values already used
/// here (only the two ASCII codepoints above needed correction).
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

/// The widest glyph in the whole Helvetica-Bold table above (1000/1000 em — several glyphs share it,
/// e.g. em dash and ellipsis; confirmed the actual max of the asset's full 256-entry `/Widths` array,
/// not just the ASCII slice). Assigned to any character neither table above knows about — deliberately
/// the WIDEST possible, so an unmodeled glyph can only make wrapping MORE conservative (an earlier line
/// break), never under-measure a line that then overflows its field's physical box.
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

/// Split `text` into paragraphs on blank-line boundaries — a run of one or more lines containing only
/// whitespace ends the current paragraph. Mirrors how `disclosure_8275` joins multiple promoted
/// tranches' own narratives with `"\n\n"` (`btctax-core/src/tax/form8275.rs`): each tranche's account
/// is a distinct, independent statement and must never blend into the next mid-line on the filed form.
/// A paragraph's own internal line breaks are NOT preserved (rewrapped freely); only the boundary
/// BETWEEN paragraphs is a hard break. Empty/whitespace-only input yields no paragraphs.
fn split_paragraphs(text: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join(" "));
                current.clear();
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }
    paragraphs
}

/// Greedily word-wrap `paragraphs` into lines, where the i-th produced line's max width is
/// `line_widths[i]` (or `line_widths`'s LAST entry, for any line beyond the array — used only so the
/// overflow diagnostic can keep measuring how many lines a too-long narrative would actually need).
/// Breaks only on whitespace (never mid-word). A **paragraph boundary is a hard break**: the line being
/// built is flushed (even if not full) before the next paragraph's first word, so two paragraphs never
/// share a field.
fn wrap_paragraphs(paragraphs: &[String], line_widths: &[f32], size_pt: f32) -> Vec<String> {
    debug_assert!(!line_widths.is_empty());
    let space_w = text_width_pts(" ", size_pt);
    let mut lines: Vec<String> = Vec::new();
    for paragraph in paragraphs {
        let mut current = String::new();
        let mut current_w = 0.0_f32;
        for word in paragraph.split_whitespace() {
            let word_w = text_width_pts(word, size_pt);
            let idx = lines.len();
            let max_w = line_widths
                .get(idx)
                .copied()
                .unwrap_or_else(|| *line_widths.last().unwrap());
            if current.is_empty() {
                current = word.to_string();
                current_w = word_w;
            } else if current_w + space_w + word_w <= max_w + EPS {
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
        // Hard break: the next paragraph (if any) always starts a FRESH `current`, above — it never
        // appends to this paragraph's last line.
    }
    lines
}

/// The cross-reference Form 8275's Rev. 10-2024 Specific Instructions require on Part IV: *"Use Part IV
/// on page 2 if you need more space for Parts I and/or II. Include the corresponding part and line
/// number from page 1."* Always refers to "line 1" because Part II now writes ONLY its own line 1 (see
/// the module doc) — there is never a different Part II line number to cite.
pub const PART_IV_CROSS_REFERENCE_PREFIX: &str = "Part II, line 1 (continued): ";

/// The wrapped placement of a Part II narrative: Part II's own line 1 (possibly empty, iff the
/// narrative itself was empty/whitespace-only), then 0 or more Part IV continuation lines — the first
/// of which, if any, carries [`PART_IV_CROSS_REFERENCE_PREFIX`].
#[derive(Debug, Clone, Default)]
pub(crate) struct PartIiWrap {
    pub part_ii_line1: String,
    pub part_iv_lines: Vec<String>,
}

/// Detail for a narrative that will not fit — everything a caller needs to build a helpful refusal
/// (see `btctax-forms::part_ii_capacity_check`, used by `btctax-cli`'s export pre-flight).
#[derive(Debug, Clone, Copy)]
pub struct PartIiOverflow {
    /// How many single-line fields the narrative actually needs (Part II's line 1 + however many Part
    /// IV lines) — may UNDER-report for the (pathological, follow-up-tracked) case of a single
    /// unbreakable run of text wider than any one line's budget, where it is instead inflated to a
    /// value guaranteed to exceed `capacity` regardless of the true "need."
    pub rows_needed: usize,
    /// The total capacity available: 1 (Part II line 1) + the Part IV continuation-line count.
    pub capacity: usize,
    /// How many characters of the narrative (NOT counting the inserted Part IV cross-reference prefix)
    /// would land across the `capacity` lines that ARE available — a real count reconstructed from the
    /// same wrap the fill itself performs, not a fabricated estimate.
    pub chars_fit: usize,
}

/// Wrap `narrative` for Form 8275: `part_ii_width_pts` is Part II line 1's own usable width (already
/// inset — see `form8275.rs::usable_width`); `part_iv_width_pts` is a Part IV line's usable width;
/// `part_iv_capacity` is how many Part IV lines are available (27 on the bundled Rev. 10-2024 asset).
///
/// Single pass: line 0 (Part II) always gets the full `part_ii_width_pts`. Line 1 (the first Part IV
/// line, if the wrap ever reaches it) is budgeted `part_iv_width_pts` MINUS
/// [`PART_IV_CROSS_REFERENCE_PREFIX`]'s own width, reserving room for it — this reservation only ever
/// matters when a second line is actually produced, which is exactly when the cross-reference is
/// needed, so there is no need to detect "will we need Part IV?" up front. Lines 2+ get the full
/// `part_iv_width_pts`.
pub(crate) fn wrap_part_ii(
    narrative: &str,
    part_ii_width_pts: f32,
    part_iv_width_pts: f32,
    part_iv_capacity: usize,
    size_pt: f32,
) -> Result<PartIiWrap, PartIiOverflow> {
    let paragraphs = split_paragraphs(narrative);
    if paragraphs.is_empty() {
        return Ok(PartIiWrap::default());
    }

    let capacity = 1 + part_iv_capacity;
    let prefix_w = text_width_pts(PART_IV_CROSS_REFERENCE_PREFIX, size_pt);
    let line_widths = [
        part_ii_width_pts,
        (part_iv_width_pts - prefix_w).max(0.0),
        part_iv_width_pts,
    ];
    let lines = wrap_paragraphs(&paragraphs, &line_widths, size_pt);

    let too_wide = lines.iter().enumerate().any(|(i, l)| {
        let budget = line_widths
            .get(i)
            .copied()
            .unwrap_or_else(|| *line_widths.last().unwrap());
        text_width_pts(l, size_pt) > budget + EPS
    });

    if too_wide || lines.len() > capacity {
        let rows_needed = if too_wide {
            lines.len().max(capacity + 1)
        } else {
            lines.len()
        };
        let fit_count = capacity.min(lines.len());
        let mut fit: Vec<String> = lines[..fit_count].to_vec();
        if let Some(first_iv) = fit.get_mut(1) {
            if let Some(stripped) = first_iv.strip_prefix(PART_IV_CROSS_REFERENCE_PREFIX) {
                *first_iv = stripped.to_string();
            }
        }
        let chars_fit = fit.join(" ").chars().count();
        return Err(PartIiOverflow {
            rows_needed,
            capacity,
            chars_fit,
        });
    }

    let part_ii_line1 = lines[0].clone();
    let mut part_iv_lines: Vec<String> = lines.get(1..).unwrap_or(&[]).to_vec();
    if let Some(first) = part_iv_lines.first_mut() {
        *first = format!("{PART_IV_CROSS_REFERENCE_PREFIX}{first}");
    }
    Ok(PartIiWrap {
        part_ii_line1,
        part_iv_lines,
    })
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
    fn backtick_and_apostrophe_use_the_assets_own_widths_not_the_generic_afm() {
        // ★ Round-2 finding 7: the generic Adobe Core-14 AFM has backtick at 278 (this asset's font
        // says 333 — under-measuring, the dangerous direction) and apostrophe at 278 (this asset says
        // 238 — over-measuring, safe, but still worth pinning so a future edit can't silently drift).
        assert_eq!(text_width_pts("`", 8.0), 333.0 * 8.0 / 1000.0);
        assert_eq!(text_width_pts("'", 8.0), 238.0 * 8.0 / 1000.0);
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
    fn split_paragraphs_treats_a_blank_line_as_a_boundary() {
        let paras = split_paragraphs("first account of events\n\nsecond, unrelated account");
        assert_eq!(
            paras,
            vec![
                "first account of events".to_string(),
                "second, unrelated account".to_string(),
            ]
        );
    }

    #[test]
    fn split_paragraphs_collapses_multiple_blank_lines_to_one_boundary() {
        let paras = split_paragraphs("a\n\n\n\nb");
        assert_eq!(paras, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn split_paragraphs_of_empty_text_yields_no_paragraphs() {
        assert_eq!(split_paragraphs(""), Vec::<String>::new());
        assert_eq!(split_paragraphs("   \n  \n"), Vec::<String>::new());
    }

    #[test]
    fn wrap_paragraphs_never_splits_a_word_and_preserves_every_word_in_order() {
        let paras = split_paragraphs("one two three four five six seven eight nine ten");
        let lines = wrap_paragraphs(&paras, &[60.0], 8.0); // narrow: forces several lines
        assert!(lines.len() > 1, "fixture premise: must actually wrap");
        let rejoined: Vec<&str> = lines.iter().flat_map(|l| l.split_whitespace()).collect();
        let original: Vec<&str> = "one two three four five six seven eight nine ten"
            .split_whitespace()
            .collect();
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
    fn wrap_paragraphs_hard_breaks_between_paragraphs_never_shares_a_line() {
        // Two short paragraphs that would EASILY fit on one combined line at this width — but the
        // paragraph boundary must still force a new line.
        let paras = split_paragraphs("short one\n\nshort two");
        let lines = wrap_paragraphs(&paras, &[1000.0], 8.0);
        assert_eq!(
            lines,
            vec!["short one".to_string(), "short two".to_string()],
            "a paragraph boundary must start a new line even when there is room to share one"
        );
    }

    #[test]
    fn wrap_paragraphs_of_no_paragraphs_yields_no_lines() {
        assert_eq!(wrap_paragraphs(&[], &[518.4], 8.0), Vec::<String>::new());
    }

    #[test]
    fn wrap_part_ii_fits_entirely_on_part_ii_line_1_when_short() {
        let wrapped = wrap_part_ii("a short narrative", 514.4, 536.0, 27, 8.0).unwrap();
        assert_eq!(wrapped.part_ii_line1, "a short narrative");
        assert!(wrapped.part_iv_lines.is_empty());
    }

    #[test]
    fn wrap_part_ii_of_empty_narrative_writes_nothing() {
        let wrapped = wrap_part_ii("", 514.4, 536.0, 27, 8.0).unwrap();
        assert_eq!(wrapped.part_ii_line1, "");
        assert!(wrapped.part_iv_lines.is_empty());
    }

    #[test]
    fn wrap_part_ii_spills_to_part_iv_with_the_cross_reference_prefix() {
        // A narrow Part II line (forces early overflow) with plenty of Part IV room.
        let wrapped = wrap_part_ii(
            "one two three four five six seven eight",
            60.0,
            1000.0,
            27,
            8.0,
        )
        .unwrap();
        assert!(!wrapped.part_iv_lines.is_empty(), "must have spilled");
        assert!(
            wrapped.part_iv_lines[0].starts_with(PART_IV_CROSS_REFERENCE_PREFIX),
            "the FIRST Part IV line must carry the IRS-required cross-reference: {:?}",
            wrapped.part_iv_lines[0]
        );
        for later in &wrapped.part_iv_lines[1..] {
            assert!(
                !later.starts_with(PART_IV_CROSS_REFERENCE_PREFIX),
                "only the FIRST Part IV line carries the cross-reference: {later:?}"
            );
        }
        // No word lost: Part II line 1 + all Part IV lines (cross-reference stripped) reproduce the
        // original word sequence in order.
        let mut got: Vec<&str> = wrapped.part_ii_line1.split_whitespace().collect();
        for (i, l) in wrapped.part_iv_lines.iter().enumerate() {
            let text = if i == 0 {
                l.strip_prefix(PART_IV_CROSS_REFERENCE_PREFIX).unwrap()
            } else {
                l.as_str()
            };
            got.extend(text.split_whitespace());
        }
        assert_eq!(
            got,
            "one two three four five six seven eight"
                .split_whitespace()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn wrap_part_ii_treats_a_paragraph_break_as_a_hard_break_across_the_wrap() {
        // Two short paragraphs, each far under Part II's own line width — without the hard-break rule
        // they would share Part II's line 1. With it, the second paragraph must start Part IV.
        let wrapped =
            wrap_part_ii("first account\n\nsecond account", 200.0, 1000.0, 27, 8.0).unwrap();
        assert_eq!(wrapped.part_ii_line1, "first account");
        assert_eq!(wrapped.part_iv_lines.len(), 1);
        assert_eq!(
            wrapped.part_iv_lines[0],
            format!("{PART_IV_CROSS_REFERENCE_PREFIX}second account")
        );
    }

    #[test]
    fn wrap_part_ii_fails_closed_when_more_lines_are_needed_than_available() {
        let text = "word ".repeat(900);
        let err = wrap_part_ii(&text, 514.4, 536.0, 27, 8.0)
            .expect_err("900 words must not fit in 1 + 27 = 28 lines");
        assert_eq!(err.capacity, 28);
        assert!(
            err.rows_needed > err.capacity,
            "rows_needed {} must exceed capacity {}",
            err.rows_needed,
            err.capacity
        );
        assert!(
            err.chars_fit > 0 && err.chars_fit < text.chars().count(),
            "chars_fit {} must be a real partial count, not 0 or the whole narrative ({})",
            err.chars_fit,
            text.chars().count()
        );
    }

    #[test]
    fn wrap_part_ii_fails_closed_on_a_single_word_wider_than_any_line() {
        let token = "a".repeat(50);
        let err = wrap_part_ii(&token, 20.0, 20.0, 27, 8.0)
            .expect_err("an unbreakable token wider than every line can never fit");
        assert!(err.rows_needed > err.capacity);
    }
}
