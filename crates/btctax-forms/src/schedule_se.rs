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

use crate::error::FormsError;
use crate::map::ScheduleSeMap;
use crate::verify::{verify_flat, FlatPlacement};
use crate::{fmt_money, pdf};
use btctax_core::{SeTaxResult, Usd};
use rust_decimal_macros::dec;

/// Hand-pinned Schedule SE column-x clusters (measured from the blank PDF): col 0 = MID [410,482],
/// col 1 = AMOUNT [504,576].
const SE_COL_MID: usize = 0;
const SE_COL_AMOUNT: usize = 1;
const SE_CLUSTERS: &[(f32, f32)] = &[(410.0, 482.0), (504.0, 576.0)];

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
    // (fqn, value, column, descent-ordinal, include?)
    let chain: [(&str, Usd, usize, u32, bool); 12] = [
        (&map.line2, se.net_se, SE_COL_AMOUNT, 0, true),
        (&map.line3, se.net_se, SE_COL_AMOUNT, 1, true),
        (&map.line4a, se.base, SE_COL_AMOUNT, 2, true),
        (&map.line4c, se.base, SE_COL_AMOUNT, 3, true),
        (&map.line6, se.base, SE_COL_AMOUNT, 4, true),
        (&map.line8a, w2_ss_wages, SE_COL_MID, 5, true),
        (&map.line8d, w2_ss_wages, SE_COL_AMOUNT, 6, !skip_8b_to_10),
        (&map.line9, line9, SE_COL_AMOUNT, 7, !skip_8b_to_10),
        (&map.line10, se.ss, SE_COL_AMOUNT, 8, !skip_8b_to_10),
        (&map.line11, se.medicare, SE_COL_AMOUNT, 9, true),
        (&map.line12, line12, SE_COL_AMOUNT, 10, true),
        (&map.line13, se.deductible_half, SE_COL_MID, 11, true),
    ];
    for (fqn, value, col, ord, include) in chain {
        if !include {
            continue;
        }
        writes.push((fqn.to_string(), pdf::FieldValue::Text(fmt_money(value))));
        placements.push(FlatPlacement::cell(fqn.to_string(), 0, col, 0, ord));
    }

    let mut doc = pdf::load(pdf::SCHEDULE_SE_PDF_2025)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // True read-back: re-parse the SERIALIZED output and verify geometry against the PDF's own rects.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, SE_CLUSTERS)?;
    Ok(Some(bytes))
}
