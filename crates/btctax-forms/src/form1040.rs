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
/// 7/7a sits at x ≈ [504,576] (single field); the 2017 line-13 dollars field sits at cx ≈ 518 and its
/// cluster must EXCLUDE the adjacent narrow cents widget (cx ≈ 565) so a dollars↔cents swap fails
/// closed. Geometry ORACLE — code-side, never from the (distrusted) map.
const F1040_COL_AMOUNT: usize = 0;
const F1040_CLUSTERS_UNIFIED: &[(f32, f32)] = &[(504.0, 576.0)];
const F1040_CLUSTERS_2017: &[(f32, f32)] = &[(482.0, 555.0)];

fn f1040_clusters(year: i32) -> &'static [(f32, f32)] {
    match year {
        2017 => F1040_CLUSTERS_2017,
        _ => F1040_CLUSTERS_UNIFIED,
    }
}

/// The btctax-evidenced signals that drive the two Form 1040 cells.
#[derive(Debug, Clone, Copy)]
pub struct Form1040Inputs {
    /// Digital-Asset question = YES iff there is any btctax-evidenced qualifying activity: any
    /// `form_8949` disposal ∨ any `income_recognized` ∨ any Gift/Donate removal. When `false` there is
    /// no reportable activity and the whole 1040 is skipped.
    pub da_yes: bool,
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

/// Fill the Form 1040 capital-gains cells. Returns `Ok(None)` — skip the whole 1040 — when there is no
/// reportable activity (`da_yes == false`). Otherwise the DA question is answered YES and line 7a is
/// filled per the active/line-16 rules. Read back through the geometric verifier (fails closed).
pub fn fill_form_1040_capgains(
    inputs: &Form1040Inputs,
    map: &Form1040Map,
) -> Result<Option<Form1040Fill>, FormsError> {
    // Produce/skip decision, per revision:
    //  • DA years (2024/2025): [R0-C4] produce iff there is reportable digital-asset activity (never
    //    fill "No" — btctax cannot vouch for the filer's full digital-asset universe).
    //  • 2017 (no DA question): produce iff there is reportable capital activity that yields a line-13
    //    entry — a gain or an active-and-netted-to-zero "-0-". Income-only / net-loss years ⇒ skip.
    if map.da_present {
        if !inputs.da_yes {
            return Ok(None);
        }
    } else if !(inputs.schedule_d_active && inputs.schedule_d_line16 >= Usd::ZERO) {
        return Ok(None);
    }

    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // Digital-Asset question = YES — only on years whose 1040 carries it (left member of the same-y
    // {/1,/2} pair, on-state /1). The 2017 form has none, so nothing is written.
    if map.da_present {
        let da_yes = map.da_yes.as_ref().ok_or_else(|| {
            FormsError::Structure("1040 map has da_present=true but no da_yes field".into())
        })?;
        writes.push((
            da_yes.field.clone(),
            pdf::FieldValue::Check {
                on: da_yes.on.clone(),
            },
        ));
        placements.push(FlatPlacement::check(da_yes.field.clone(), 0));
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
        let da_yes = map.da_yes.as_ref().expect("checked above");
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
