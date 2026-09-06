//! Schedule SE (Form 1040) fill: the filled §1401 self-employment-tax line chain, read back through
//! the SP2 flat-form geometric oracle (column-x cluster + ordinal-y descent + no-unmapped).
//!
//! **[★ R0-C1] Line 12 = lines 10 + 11 (SS + regular Medicare ONLY).** `SeTaxResult.total` also
//! includes the 0.9% Additional Medicare Tax — which is a **Form 8959** item, NOT on Schedule SE
//! ("12 … Add lines 10 and 11"). So line 12 := `ss + medicare` and line 13 := `deductible_half`
//! (consistent by construction: `deductible_half = (ss + medicare) / 2`). When `addl > 0` the CLI
//! prints a loud Form 8959 advisory.
//!
//! **[★ R0-I2] $400 floor:** the form line 4c says "if less than $400, STOP; you don't owe SE tax",
//! but `compute_se_tax` has no $400 threshold. SP2 SKIPS Schedule SE (returns `None`) when the net SE
//! earnings (line 4c = `base`) are below $400.
//!
//! The full self-consistent chain (like SP1's Schedule D 7/15/16) is filled: 2, 3, 4a, 4c, 6, 8a, 8d,
//! 9, 10, 11, 12, 13. Line 9 uses the threaded `ss_wage_base` ($176,100 for 2025). Per the form, if
//! W-2 SS wages (line 8a) ≥ `ss_wage_base`, lines 8b–10 are skipped (8d/9/10 blank, matching `ss == 0`).

use crate::cells::push_money;
use crate::error::FormsError;
use crate::map::ScheduleSeMap;
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::{SeTaxResult, Usd};
use rust_decimal_macros::dec;

/// Logical Schedule SE columns: col 0 = MID (lines 8a, 13), col 1 = AMOUNT (all other lines).
const SE_COL_MID: usize = 0;
const SE_COL_AMOUNT: usize = 1;
/// Hand-pinned column-x clusters (measured from the blank PDF), **per form revision**. On the 2024/
/// 2025 unified SE the amount fields sit at x ≈ [504,576] / MID ≈ [410,482]; on the OLD 2017 §B long
/// form the (dollars) fields sit further left and each cluster must EXCLUDE its narrow cents widget so
/// a dollars↔cents swap fails closed (2017 dollars: MID cx ≈ 392, AMOUNT cx ≈ 514; cents cx ≈ 443 /
/// 566). These are the geometry ORACLE — deliberately code-side, never taken from the (distrusted) map.
const SE_CLUSTERS_UNIFIED: &[(f32, f32)] = &[(410.0, 482.0), (504.0, 576.0)];
const SE_CLUSTERS_2017: &[(f32, f32)] = &[(350.0, 433.0), (476.0, 554.0)];

fn se_clusters(year: i32) -> &'static [(f32, f32)] {
    // ★★★ **ENUMERATED, not a wildcard.** This was `_ => SE_CLUSTERS_UNIFIED`, which handed
    //     2024/2025 geometry to ANY year — including a year whose form nobody has looked at. These
    //     clusters are the MAP-INDEPENDENT geometry oracle that `verify_flat` checks a filled form
    //     against, so a wildcard makes the instrument built to catch a mis-mapped cell the one thing
    //     that was never told the year changed.
    //
    //     ★ **What a wildcard actually costs, measured on the one revision pair this repo holds
    //       both sides of.** Centre-x read off the bundled blank PDF
    //       (`./target/debug/xtask dump-fields crates/btctax-forms/forms/2017/schedule_se.pdf`),
    //       membership decided by the same `verify::in_band` the oracle uses. BOTH columns lose the
    //       guard, and the MID column additionally rejects the real cell:
    //
    //         line 8a dollars (MID)     cx 392.4  IN     SE_CLUSTERS_2017[MID]    [350,433]
    //         line 8a cents   (MID)     cx 442.8  NOT IN SE_CLUSTERS_2017[MID]    [350,433]
    //         line 8a cents   (MID)     cx 442.8  IN     SE_CLUSTERS_UNIFIED[MID] [410,482]  ★
    //         line 8a dollars (MID)     cx 392.4  NOT IN SE_CLUSTERS_UNIFIED[MID] [410,482]  ★
    //         line 2  dollars (AMOUNT)  cx 514.8  IN     SE_CLUSTERS_2017[AMT]    [476,554]
    //         line 2  cents   (AMOUNT)  cx 566.2  NOT IN SE_CLUSTERS_2017[AMT]    [476,554]
    //         line 2  cents   (AMOUNT)  cx 566.2  IN     SE_CLUSTERS_UNIFIED[AMT] [504,576]  ★
    //
    //       Run 2024/2025's bands over the 2017 §B long form and each cents widget passes AS its
    //       dollars cell. A wildcard does not merely skip a check here; it disarms the dollars/cents
    //       guard the bands exist to be.
    //       `the_2017_bands_exclude_the_cents_widgets_the_unified_bands_admit` below re-derives
    //       every one of these numbers from the PDF and the map on every run.
    //
    //       ★ Stated honestly: TY2017 has its OWN arm, so this is not a live path — it is the only
    //         cross-revision measurement this repo can make, and it measures exactly the thing a
    //         wildcard assumes away, that one revision's x-bands still mean what they meant.
    //
    //     ★ **"The names still resolve" is evidence of nothing.**
    //       `./target/debug/xtask form-delta f6251--2024 f6251--2025` — the calibration pair, both
    //       sides real finals: 61 common fields, 1 added, **0 removed and 0 renamed** — and **30**
    //       of the **59** fields whose printed label resolves on BOTH sides now sit beside a
    //       different line.
    //
    //     ★ **Retracted — do not re-cite.** This comment previously read *"the 2026 drafts move 31
    //       line bindings while renaming nothing."* That 31 was an artifact of a page off-by-one
    //       introduced by the drafts' IRS cover sheet, and it cited Form 1040, not this form.
    //       Post-correction, Schedule SE's own delta
    //       (`form-delta f1040sse--2025 f1040sse--2026-DRAFT`, and the draft IS genuinely TY2026 —
    //       "Schedule SE (Form 1040) 2026 Created 4/27/26") is: 25 common fields, **2 renamed**
    //       (`Line5a_ReadOrder[0].f1_10[0]` → `f1_10[0]`, `Line8a_ReadOrder[0].f1_14[0]` →
    //       `f1_14[0]`; both keep their printed label), and **0** of the 22 label-comparable fields
    //       moved line. The FORM still changed — line 7's wage base goes $176,100 → $184,500 — so a
    //       quiet TY2026 geometry match would still be the wrong reason to skip looking.
    //
    //     ★ A new year must be added HERE, deliberately, after someone has compared the form. The
    //       panic is the point: silently reusing last year's x-bands is how a wrong cell passes.
    match year {
        2017 => SE_CLUSTERS_2017,
        2024 | 2025 => SE_CLUSTERS_UNIFIED,
        other => panic!(
            "se_clusters: no geometry recorded for TY{other}. The unified bands cover TY2024-2025 \
             only; add an arm after measuring the year's MID + AMOUNT columns off its blank PDF \
             (xtask dump-fields), never widen this to a wildcard — the unified bands admit BOTH of \
             the TY2017 form's cents widgets (cx 442.8 and 566.2), so a wildcard turns the \
             dollars/cents guard off."
        ),
    }
}

/// The §1401 net-earnings STOP floor: Schedule SE is not owed when line 4c (`base`) is below $400.
pub const SE_FLOOR: Usd = dec!(400);

/// Fill Schedule SE for `year` from the computed `SeTaxResult`, the filer's Form W-2 Social Security
/// wages (`w2_ss_wages`, line 8a), and the year's Social Security wage base (`ss_wage_base`, line 7).
/// Returns `Ok(None)` when net SE earnings are **below the $400 floor** (no SE tax owed — skip the
/// form). Otherwise returns the serialized PDF bytes, read back through the geometric verifier (a
/// mis-mapped cell FAILS CLOSED).
pub fn fill_schedule_se_with_map(
    se: &SeTaxResult,
    w2_ss_wages: Usd,
    ss_wage_base: Usd,
    map: &ScheduleSeMap,
) -> Result<Option<Vec<u8>>, FormsError> {
    // [R0-I2] $400 floor — the form's line-4c STOP. `base` is line 4c (net SE earnings × 92.35%).
    if se.base < SE_FLOOR {
        return Ok(None);
    }

    // Line 9 = line 7 (ss_wage_base) − line 8d (= W-2 SS wages), floored at 0.
    let line9 = {
        let v = ss_wage_base - w2_ss_wages;
        if v < Usd::ZERO {
            Usd::ZERO
        } else {
            v
        }
    };
    // Per the form: if line 8a (W-2 SS wages) ≥ the wage base, skip lines 8b–10 (8d/9/10 blank).
    let skip_8b_to_10 = w2_ss_wages >= ss_wage_base;

    let line12 = se.ss + se.medicare; // ★ SS + regular Medicare ONLY (addl is a Form 8959 item).

    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();
    // (value, column, include?) parallel to `map.lines()` — one shared chain for every revision; the
    // 2017 form's fields are dollars+cents pairs, which `push_money` emits transparently.
    let plan: [(Usd, usize, bool); 12] = [
        (se.net_se, SE_COL_AMOUNT, true),             // 2
        (se.net_se, SE_COL_AMOUNT, true),             // 3
        (se.base, SE_COL_AMOUNT, true),               // 4a
        (se.base, SE_COL_AMOUNT, true),               // 4c
        (se.base, SE_COL_AMOUNT, true),               // 6
        (w2_ss_wages, SE_COL_MID, true),              // 8a
        (w2_ss_wages, SE_COL_AMOUNT, !skip_8b_to_10), // 8d
        (line9, SE_COL_AMOUNT, !skip_8b_to_10),       // 9
        (se.ss, SE_COL_AMOUNT, !skip_8b_to_10),       // 10
        (se.medicare, SE_COL_AMOUNT, true),           // 11
        (line12, SE_COL_AMOUNT, true),                // 12
        (se.deductible_half, SE_COL_MID, true),       // 13
    ];
    for (ord, (cell, (value, col, include))) in map.lines().iter().zip(plan).enumerate() {
        if !include {
            continue;
        }
        push_money(
            &mut writes,
            &mut placements,
            cell,
            value,
            col,
            Some((0, ord as u32)),
        );
    }
    // Pre-filled factory constants (2017 §B line 7 = 127,200/00, line 14 = 5,200/00): authorize them
    // so the blank's own `/V` values don't trip `no_unmapped_filled`. Never written, only exempted.
    for fqn in &map.prefilled_exempt {
        placements.push(FlatPlacement::free(fqn.clone(), crate::cells::page_of(fqn)));
    }

    let mut doc = pdf::load(pdf::schedule_se_pdf(map.year)?)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // True read-back: re-parse the SERIALIZED output and verify geometry against the PDF's own rects.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, se_clusters(map.year))?;
    Ok(Some(bytes))
}

#[cfg(test)]
mod cluster_year_guard {
    use super::*;
    use crate::map::{MoneyCell, ScheduleSeMap};
    use crate::verify::in_band;

    /// Run the lookup with panic output silenced, restoring the previous hook. Nothing here asserts
    /// on a panic *message*, so swallowing it costs nothing and keeps a passing run readable.
    fn resolves(year: i32) -> bool {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let ok = std::panic::catch_unwind(|| se_clusters(year)).is_ok();
        std::panic::set_hook(prev);
        ok
    }

    /// ★★★ **The enumeration IS the guard, and the expected answer is DERIVED.**
    ///
    /// The probe below is a domain to sweep, never an expected set: for each year the answer comes
    /// from [`crate::SUPPORTED_YEARS`], so this reds in BOTH directions —
    ///
    /// * a year wired into the product with no column bands measured for it, and
    /// * bands handed to a year the product does not support (i.e. the wildcard is back).
    ///
    /// **Planted-defect check (B1):** restore `other => SE_CLUSTERS_UNIFIED` and this test reds on
    /// the first unsupported probe year with "TY2010 is NOT in SUPPORTED_YEARS yet se_clusters
    /// answered".
    #[test]
    fn geometry_is_recorded_for_exactly_the_supported_years() {
        for year in 2010..=2040 {
            let supported = crate::SUPPORTED_YEARS.contains(&year);
            let answered = resolves(year);
            assert_eq!(
                answered,
                supported,
                "TY{year}: SUPPORTED_YEARS.contains = {supported} but se_clusters {} — {}",
                if answered { "answered" } else { "panicked" },
                if supported {
                    "a supported year with no bands recorded: measure the MID + AMOUNT columns off \
                     the year's blank PDF (xtask dump-fields) and add the arm"
                } else {
                    "an unsupported year must NOT inherit another revision's x-bands; the wildcard \
                     is back"
                }
            );
        }
    }

    /// ★★★ **Why the wildcard is not merely "unchecked" but WRONG — measured, not asserted.**
    ///
    /// A band's job is that a map whose `dollars_field` actually names the narrow cents widget fails
    /// the column check. On the TY2017 §B long form BOTH logical columns lose that property under
    /// the unified bands, and the MID column additionally stops admitting its own dollars field.
    /// Every centre-x is read from the bundled blank TY2017 PDF and the committed TY2017 map, so
    /// this is the comment above re-derived rather than repeated.
    ///
    /// **Planted-defect check (B1), observed:** set `SE_CLUSTERS_2017 = SE_CLUSTERS_UNIFIED` (what
    /// the wildcard did) and this reds on the MID column's FIRST assertion — "TY2017 line-8a
    /// dollars cx 392.4 is outside its own band (410.0, 482.0)". The unified MID band does not even
    /// contain the 2017 dollars field, so it fails before reaching the cents case; the cents
    /// assertions are what red on a band that merely widened.
    #[test]
    fn the_2017_bands_exclude_the_cents_widgets_the_unified_bands_admit() {
        let map = ScheduleSeMap::for_year(2017).expect("TY2017 schedule_se map");
        let doc = pdf::load(pdf::schedule_se_pdf(2017).expect("TY2017 SE PDF")).expect("parse");
        let fields = pdf::collect_fields(&doc).expect("fields");
        let cx = |fqn: &str| {
            fields
                .iter()
                .find(|f| f.fqn == fqn)
                .unwrap_or_else(|| panic!("{fqn} not in the bundled TY2017 PDF"))
                .cx()
                .unwrap_or_else(|| panic!("{fqn} has no widget rect"))
        };

        // One cell per logical column, named by the line the form prints: 8a is the MID column,
        // 2 is the AMOUNT column (see the `SE_COL_*` constants).
        for (line, cell, col) in [
            ("8a", &map.line8a, SE_COL_MID),
            ("2", &map.line2, SE_COL_AMOUNT),
        ] {
            let MoneyCell::Pair(pair) = cell else {
                panic!("TY2017 line {line} is a dollars+cents pair — the whole point of this test");
            };
            let (dollars, cents) = (cx(&pair.dollars_field), cx(&pair.cents_field));
            let own = SE_CLUSTERS_2017[col];
            let unified = SE_CLUSTERS_UNIFIED[col];

            assert!(
                in_band(dollars, own),
                "TY2017 line-{line} dollars cx {dollars:.1} is outside its own band {own:?}"
            );
            assert!(
                !in_band(cents, own),
                "TY2017 line-{line} cents widget cx {cents:.1} is INSIDE the band recorded for \
                 TY2017 {own:?} — a map that swapped dollars and cents would pass the column check"
            );
            assert!(
                in_band(cents, unified),
                "the premise of the panic has changed: the unified band {unified:?} no longer \
                 admits the TY2017 line-{line} cents widget at cx {cents:.1}, so reusing it across \
                 revisions is no longer demonstrably unsafe — re-measure before rewriting the \
                 comment above"
            );
        }
    }
}
