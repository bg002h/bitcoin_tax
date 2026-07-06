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

use crate::error::FormsError;
use crate::map::Form1040Map;
use crate::verify::{topmost_yes_no_pair, verify_flat, FlatPlacement};
use crate::{fmt_money, pdf};
use btctax_core::Usd;

/// Hand-pinned Form 1040 amount column-x cluster (line 7a f1_70 = [504,90,576,102]).
const F1040_COL_AMOUNT: usize = 0;
const F1040_CLUSTERS: &[(f32, f32)] = &[(504.0, 576.0)];

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
    // [R0-C4] No reportable activity → do not produce the form at all (never fill "No").
    if !inputs.da_yes {
        return Ok(None);
    }

    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // Digital-Asset question = YES (left member of the same-y {/1,/2} pair, on-state /1).
    writes.push((
        map.da_yes.field.clone(),
        pdf::FieldValue::Check {
            on: map.da_yes.on.clone(),
        },
    ));
    placements.push(FlatPlacement::check(map.da_yes.field.clone(), 0));

    // Line 7a — only when Schedule D is ACTIVE and line 16 ≥ 0.
    let mut filled_7a = false;
    let mut active_zero = false;
    let mut loss = false;
    if inputs.schedule_d_active {
        if inputs.schedule_d_line16 < Usd::ZERO {
            loss = true; // net loss → 7a BLANK + notice (§1211 line-21 cap is the filer's).
        } else {
            let value = if inputs.schedule_d_line16.is_zero() {
                active_zero = true;
                "-0-".to_string() // active-and-netted-to-zero → the "-0-" literal.
            } else {
                fmt_money(inputs.schedule_d_line16)
            };
            writes.push((map.line7a.clone(), pdf::FieldValue::Text(value)));
            placements.push(FlatPlacement::col_only(
                map.line7a.clone(),
                0,
                F1040_COL_AMOUNT,
            ));
            filled_7a = true;
        }
    }
    // else: Schedule D INACTIVE (income-only / donation-only year) → 7a BLANK even though DA = YES.

    let mut doc = pdf::load(pdf::F1040_PDF_2025)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // Read back the SERIALIZED output.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, F1040_CLUSTERS)?;
    // Map-independent DA-question guard: the map's Yes/No must BE the left/right members of the
    // top-most same-y {/1,/2} pair — a Yes/No swap in the map fails closed here.
    let (yes_fqn, no_fqn) = topmost_yes_no_pair(&check, &fields, 0)?;
    if yes_fqn != map.da_yes.field {
        return Err(FormsError::Geometry(format!(
            "1040 DA 'Yes' map field {:?} is not the LEFT member of the top-most same-y {{/1,/2}} pair ({yes_fqn:?})",
            map.da_yes.field
        )));
    }
    if no_fqn != map.da_no.field {
        return Err(FormsError::Geometry(format!(
            "1040 DA 'No' map field {:?} is not the right member of the top-most same-y {{/1,/2}} pair ({no_fqn:?})",
            map.da_no.field
        )));
    }

    Ok(Some(Form1040Fill {
        pdf: bytes,
        filled_7a,
        active_zero,
        loss,
    }))
}
