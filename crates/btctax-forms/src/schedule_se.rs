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
    match year {
        2017 => SE_CLUSTERS_2017,
        _ => SE_CLUSTERS_UNIFIED,
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
