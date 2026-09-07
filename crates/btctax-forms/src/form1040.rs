//! Form 1040 capital-gains cells ONLY: line 7a + the Digital-Asset Yes/No question. Read back through
//! the SP2 flat oracle (amount-column x-cluster) + the map-independent same-y `/Btn` pair predicate.
//!
//! **[★ R0-C2 + I★1] Line 7a** (renumbered in 2025; a 7b checkbox pair is new). btctax vouches for
//! exactly two cells here, so:
//! - **Fill 7a ONLY when Schedule D is ACTIVE (there are capital disposals) AND line 16 ≥ 0.** A gain
//!   → the line-16 amount; **active-and-netted-to-zero → the "-0-" literal**.
//! - **Schedule D INACTIVE** (income-only / donation-only year; the DA answer may be YES but there are
//!   no capital disposals) → **7a BLANK**. Stamping "-0-" against a blank Schedule D line 16 would be
//!   an unearned zero-capital-gains claim.
//! - **NET LOSS** → **7a BLANK** + a loud §1211 notice (the $3,000/$1,500-MFS cap on Schedule D line
//!   21 is the filer's; SP1 scoped out line 21).
//! - **7b checkboxes stay untouched.**
//!
//! **[★ R0-C4] Digital-Asset question = YES only with btctax-evidenced qualifying activity** (any
//! disposal ∨ any income_recognized ∨ any gift/donate removal). Otherwise **skip the whole 1040** (No
//! is never filled — btctax cannot know the filer's full digital-asset universe).

use crate::cells::{push_literal, push_money};
use crate::error::FormsError;
use crate::map::Form1040Map;
use crate::pdf;
use crate::verify::{topmost_yes_no_pair, verify_flat, FlatPlacement};
use btctax_core::Usd;

/// Hand-pinned Form 1040 capital-gain amount column-x cluster, **per form revision**. 2024/2025 line
/// 7/7a sits at x ≈ [504,576] (single field). Geometry ORACLE — code-side, never from the
/// (distrusted) map.
const F1040_COL_AMOUNT: usize = 0;
const F1040_CLUSTERS_UNIFIED: &[(f32, f32)] = &[(504.0, 576.0)];

fn f1040_clusters(year: i32) -> &'static [(f32, f32)] {
    // ★★★ **ENUMERATED, not a wildcard.** This was `_ => F1040_CLUSTERS_UNIFIED`, which handed
    //     2024/2025 geometry to ANY year — including a year whose form nobody has looked at. These
    //     clusters are the MAP-INDEPENDENT geometry oracle that `verify_flat` checks a filled form
    //     against, so a wildcard makes the instrument built to catch a mis-mapped cell the one thing
    //     that was never told the year changed.
    //
    //     ★ **What a wildcard actually costs — MEASURED, on the one revision pair this repo used
    //       to hold both sides of.** Centre-x read off the then-bundled blank
    //       `forms/2017/f1040.pdf` (`xtask dump-fields`), membership decided by the same
    //       `verify::in_band` the oracle uses:
    //
    //         TY2017 line-13 dollars  cx 518.1  IN     F1040_CLUSTERS_2017     [482,555]  <- real cell
    //         TY2017 line-13 cents    cx 565.2  NOT IN F1040_CLUSTERS_2017     [482,555]  <- swap fails closed
    //         TY2017 line-13 cents    cx 565.2  IN     F1040_CLUSTERS_UNIFIED  [504,576]  ★
    //
    //       Run 2024/2025's band over the 2017 form and the cents widget passes AS the dollars cell.
    //       A wildcard does not merely skip a check here; it disarms the dollars/cents guard the
    //       band exists to be.
    //
    //     ★★ **This measurement is now HISTORY, not a live check** (2026-09-06, owner ruling S9).
    //       Dropping the TY2017 form package took `forms/2017/f1040.pdf`, its map, and the test
    //       `the_2017_band_excludes_a_cents_widget_the_unified_band_admits` that re-derived all
    //       three numbers on every run. The numbers above were true when measured and are kept as
    //       the recorded evidence for the panic below; they can no longer be re-derived in-suite,
    //       and nothing in this repo holds two revisions of Form 1040 with a dollars+cents split
    //       any more. What SURVIVES is
    //       `geometry_is_recorded_for_exactly_the_supported_years`, which still reds in both
    //       directions if the wildcard comes back.
    //
    //     ★ **"The names still resolve" is evidence of nothing.**
    //       `./target/debug/xtask form-delta f6251--2024 f6251--2025` — the calibration pair, both
    //       sides real finals: 61 common fields, 1 added, **0 removed and 0 renamed** — and **30**
    //       of the **59** fields whose printed label resolves on BOTH sides now sit beside a
    //       different line.
    //
    //     ★ **Retracted — do not re-cite.** This comment previously read *"the 2026 drafts move 31
    //       line bindings while renaming nothing."* That 31 was an artifact of a page off-by-one
    //       introduced by the drafts' IRS cover sheet, not a property of the forms. Post-correction:
    //       `form-delta f6251--2025 f6251--2026-DRAFT` → 0 renamed, **1** moved (`f1_5`: line 2a → 2).
    //       For Form 1040 there is **no TY2026 measurement at all**: the archived
    //       `design/forms/2026/f1040--2026-DRAFT.pdf` is the TY2025 form (its own text layer reads
    //       "Form 1040 (2025) Created 9/5/25"), so the "no field changed" that
    //       `form-delta f1040--2025 f1040--2026-DRAFT` prints is TY2025 compared with itself.
    //       The panic below stands on the TY2017 measurement above, which needs no draft.
    //
    //     ★ A new year must be added HERE, deliberately, after someone has compared the form. The
    //       panic is the point: silently reusing last year's x-bands is how a wrong cell passes.
    match year {
        2024 | 2025 => F1040_CLUSTERS_UNIFIED,
        other => panic!(
            "f1040_clusters: no geometry recorded for TY{other}. The unified bands cover TY2024-2025 \
             only; add an arm after measuring the year's amount column off its blank PDF (xtask \
             dump-fields), never widen this to a wildcard — the unified band [504,576] admitted the \
             TY2017 cents widget at cx 565.2 (measured before the S9 drop; see the comment above), \
             so a wildcard turns the dollars/cents guard off."
        ),
    }
}

/// The signals that drive the two Form 1040 cells this fill writes.
#[derive(Debug, Clone, Copy)]
pub struct Form1040Inputs {
    /// ★★★ **The FILER'S ANSWER to the Digital Assets question — never a ledger reading.**
    ///
    /// `Some(true)` → *Yes*, `Some(false)` → *No*, `None` → **NEITHER box is written**, and the
    /// caller names the unanswered box (`export-irs-pdf`'s hand-mark list).
    ///
    /// ★★★ **(T6 seam review C-1) This was a `bool` fed from a LEDGER PREDICATE**, so a filer who
    ///     had answered `No` got a page with *Yes* checked and a filer who had never been asked got
    ///     one too — btctax answering a §6065 declaration for a human, on the one arm the product
    ///     can print for this filing season. An entry is testimony; a blank is no testimony; and the
    ///     two are indistinguishable on the printed page, which is why this is an `Option` and not a
    ///     defaulted `bool`.
    pub digital_asset_answer: Option<bool>,
    /// ★ The PRODUCE/SKIP decision for the whole page, and it is a separate question from the
    /// answer: is there any btctax-evidenced reportable activity — a `form_8949` disposal ∨
    /// recognized income ∨ a Gift/Donate removal, in the year. When `false` there is nothing for
    /// this worksheet to say and the whole 1040 is skipped, exactly as before T6, whatever the
    /// answer is.
    pub reportable_activity: bool,
    /// Schedule D is ACTIVE — there are capital disposals (some ST or LT part has activity).
    pub schedule_d_active: bool,
    /// Schedule D line 16 = ST gain + LT gain (raw, pre-netting). Only consulted when active.
    pub schedule_d_line16: Usd,
}

/// The result of a Form 1040 cap-gains fill: the bytes + what was actually written (drives the CLI's
/// partial-scope + loss notices).
#[derive(Debug, Clone)]
pub struct Form1040Fill {
    /// The serialized PDF bytes.
    pub pdf: Vec<u8>,
    /// Whether line 7a received a value (a gain amount or the "-0-" literal).
    pub filled_7a: bool,
    /// Active-and-netted-to-zero → line 7a is the "-0-" literal.
    pub active_zero: bool,
    /// Net loss → line 7a left BLANK; the caller prints the §1211 line-21 notice.
    pub loss: bool,
}

/// Fill the Form 1040 capital-gains cells. Returns `Ok(None)` — skip the whole 1040 — when there is
/// no reportable activity ([`Form1040Inputs::reportable_activity`] `== false`). Otherwise the
/// Digital Assets question is written FROM THE FILER'S ANSWER (`Some(true)` → the *Yes* box,
/// `Some(false)` → the *No* box, `None` → neither) and line 7a is filled per the active/line-16
/// rules. Read back through the geometric verifier (fails closed).
pub fn fill_form_1040_capgains(
    inputs: &Form1040Inputs,
    map: &Form1040Map,
) -> Result<Option<Form1040Fill>, FormsError> {
    // Produce/skip decision, per revision:
    //  • DA years (2024/2025): produce iff there is reportable digital-asset activity.
    //    ★★★ (T6 seam review C-1) This predicate is `reportable_activity`, its OWN field, and no
    //        longer the answer. Before the fold the two were one `bool`, so "the ledger says
    //        nothing happened" and "the filer said No" were the same fact — which is exactly how a
    //        `No` came to print as *Yes*. They are different questions: what to print is the
    //        filer's testimony, whether to print at all is the ledger's.
    //  • 2017 (no DA question): produce iff there is reportable capital activity that yields a line-13
    //    entry — a gain or an active-and-netted-to-zero "-0-". Income-only / net-loss years ⇒ skip.
    if map.da_present {
        if !inputs.reportable_activity {
            return Ok(None);
        }
    } else if !(inputs.schedule_d_active && inputs.schedule_d_line16 >= Usd::ZERO) {
        return Ok(None);
    }

    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // ★★★ The Digital Assets question — THE FILER'S ANSWER, on years whose 1040 carries it (the
    //     left/right members of the same-y {/1,/2} pair, on-state /1 and /2). The 2017 form has
    //     none, so nothing is written whatever the answer says.
    //
    // ★★★ `None` writes NEITHER box. That is the one state the pre-T6 `bool` could not express, and
    //     it is not the same page as a *No*: a blank is no testimony, and btctax may not swear a
    //     §6065 declaration a filer never made. The caller says so out loud — `export-irs-pdf`
    //     carries the box in its hand-mark list — because a blank the filer never sees is exactly
    //     the defect this fold closes, one step later.
    if map.da_present {
        let field = match inputs.digital_asset_answer {
            Some(true) => Some(map.da_yes.as_ref().ok_or_else(|| {
                FormsError::Structure("1040 map has da_present=true but no da_yes field".into())
            })?),
            Some(false) => Some(map.da_no.as_ref().ok_or_else(|| {
                FormsError::Structure("1040 map has da_present=true but no da_no field".into())
            })?),
            None => None,
        };
        if let Some(cell) = field {
            writes.push((
                cell.field.clone(),
                pdf::FieldValue::Check {
                    on: cell.on.clone(),
                },
            ));
            placements.push(FlatPlacement::check(cell.field.clone(), 0));
        }
    }

    // Capital-gain line (7a in 2025 / 7 in 2024 / **13 in 2017**) — only when Schedule D is ACTIVE and
    // line 16 ≥ 0. The cell is single (2024/2025) or a dollars+cents pair (2017), handled uniformly.
    let mut filled_7a = false;
    let mut active_zero = false;
    let mut loss = false;
    if inputs.schedule_d_active {
        if inputs.schedule_d_line16 < Usd::ZERO {
            loss = true; // net loss → line BLANK + notice (§1211 line-21 cap is the filer's).
        } else if inputs.schedule_d_line16.is_zero() {
            active_zero = true; // active-and-netted-to-zero → the "-0-" literal.
            push_literal(
                &mut writes,
                &mut placements,
                &map.line7a,
                "-0-",
                F1040_COL_AMOUNT,
            );
            filled_7a = true;
        } else {
            push_money(
                &mut writes,
                &mut placements,
                &map.line7a,
                inputs.schedule_d_line16,
                F1040_COL_AMOUNT,
                None,
            );
            filled_7a = true;
        }
    }
    // else: Schedule D INACTIVE (income-only / donation-only DA year) → line BLANK even though DA = YES.

    let mut doc = pdf::load(pdf::f1040_pdf(map.year)?)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // Read back the SERIALIZED output.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, f1040_clusters(map.year))?;
    // Map-independent DA-question guard (only on years whose 1040 HAS the question): the map's Yes/No
    // must BE the left/right members of the top-most horizontally-ADJACENT same-y {/1,/2} pair — a
    // Yes/No swap in the map fails closed here. The no-DA 2017 form skips it.
    if map.da_present {
        // ★ Both members are resolved HERE, not "checked above": since the C-1 fold an unanswered
        //   question writes NEITHER box, so neither field is necessarily touched by the write pass
        //   — and a half-populated map must still fail LOUD rather than skip its own guard.
        let da_yes = map.da_yes.as_ref().ok_or_else(|| {
            FormsError::Structure("1040 map has da_present=true but no da_yes field".into())
        })?;
        let da_no = map.da_no.as_ref().ok_or_else(|| {
            FormsError::Structure("1040 map has da_present=true but no da_no field".into())
        })?;
        let (yes_fqn, no_fqn) = topmost_yes_no_pair(&check, &fields, 0)?;
        if yes_fqn != da_yes.field {
            return Err(FormsError::Geometry(format!(
                "1040 DA 'Yes' map field {:?} is not the LEFT member of the top-most adjacent {{/1,/2}} pair ({yes_fqn:?})",
                da_yes.field
            )));
        }
        if no_fqn != da_no.field {
            return Err(FormsError::Geometry(format!(
                "1040 DA 'No' map field {:?} is not the right member of the top-most adjacent {{/1,/2}} pair ({no_fqn:?})",
                da_no.field
            )));
        }
    }

    Ok(Some(Form1040Fill {
        pdf: bytes,
        filled_7a,
        active_zero,
        loss,
    }))
}

#[cfg(test)]
mod cluster_year_guard {
    use super::*;

    /// Run `f` with panic output silenced, restoring the previous hook. Nothing here asserts on a
    /// panic *message*, so swallowing it costs nothing and keeps a passing run readable.
    fn resolves(year: i32) -> bool {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let ok = std::panic::catch_unwind(|| f1040_clusters(year)).is_ok();
        std::panic::set_hook(prev);
        ok
    }

    /// ★★★ **The enumeration IS the guard, and the expected answer is DERIVED.**
    ///
    /// The probe below is a domain to sweep, never an expected set: for each year the answer comes
    /// from [`crate::SUPPORTED_YEARS`], so this reds in BOTH directions —
    ///
    /// * a year wired into the product with no amount-column band measured for it, and
    /// * a band handed to a year the product does not support (i.e. the wildcard is back).
    ///
    /// **Planted-defect check (B1):** restore `other => F1040_CLUSTERS_UNIFIED` and this test reds
    /// on the first unsupported probe year with "TY2010 is NOT in SUPPORTED_YEARS yet
    /// f1040_clusters answered".
    #[test]
    fn geometry_is_recorded_for_exactly_the_supported_years() {
        for year in 2010..=2040 {
            let supported = crate::SUPPORTED_YEARS.contains(&year);
            let answered = resolves(year);
            assert_eq!(
                answered,
                supported,
                "TY{year}: SUPPORTED_YEARS.contains = {supported} but f1040_clusters {} — {}",
                if answered { "answered" } else { "panicked" },
                if supported {
                    "a supported year with no band recorded: measure the amount column off the \
                     year's blank PDF (xtask dump-fields) and add the arm"
                } else {
                    "an unsupported year must NOT inherit another revision's x-band; the wildcard \
                     is back"
                }
            );
        }
    }
}
