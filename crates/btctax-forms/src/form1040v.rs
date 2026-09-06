//! **Form 1040-V** (Payment Voucher) fill — the page that rides LOOSE in the envelope with a check,
//! read back through the flat-form geometric oracle (column-x cluster + ordinal-y descent + /MaxLen
//! + no-unmapped).
//!
//! **This module does no tax arithmetic.** Box 3 is the amount the caller is paying — Form 1040
//! line 37 as printed, or the partial payment the filer chose — and every other box is the return's
//! own identity, transcribed: *"Line 4. Enter your name(s) and address exactly as shown on"* *"your
//! return. Please print clearly."*
//!
//! **The voucher is never stapled and never attached.** It says so on its own face: *"Do not staple
//! or attach this voucher to your payment or return."* — and the instructions again: *"Don't staple
//! or otherwise attach your payment or Form 1040-V"* *"to your return or to each other. Instead,
//! just put them loose in"* *"the envelope."* That is why nothing here produces a `NamedForm` and no
//! code path hands either new stem to `FiledPacket::stapled`: the packet IS the stapling order, and
//! a voucher inside it would be an instruction to do the one thing the form forbids.
//!
//! **What is deliberately never written**: the three foreign-address boxes. `ReturnHeader` carries
//! no foreign address, so they are left blank — and that blank is CORRECT (the TY2024 Form 1040 map
//! censuses its own three the same way). `Apt. no.` is the same shape: the header has no apartment
//! field, so the cell stays blank rather than guessing at a street string's tail.

use crate::error::FormsError;
use crate::fmt_money;
use crate::form4868::refuse_unless_whole_nonnegative;
use crate::map::Form1040VMap;
use crate::pdf::{self, FieldValue};
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::packet::PrintedReturn;
use btctax_core::tax::types::FilingStatus;
use btctax_core::Usd;

/// Logical Form 1040-V column: col 0 = box 3, the amount column.
const F1040V_COL_AMOUNT: usize = 0;

/// Hand-pinned column-x cluster, MEASURED from the blank PDFs with `xtask dump-fields`: box 3
/// (`f1_3`) occupies `x 460.8 … 574.0` on both bundled revisions (the 15 field names AND rects are
/// identical across TY2024 and TY2025). Code-side, never read from the (distrusted) map — a map that
/// pointed box 3 at an SSN or a name cell fails closed instead of printing the amount owed into it.
const F1040V_CLUSTERS: &[(f32, f32)] = &[(460.8, 574.0)];

/// Descent group 0 — the LEFT print column, top to bottom: your first name, the spouse's first name,
/// the home address.
const GRP_LEFT: u32 = 0;
/// Descent group 1 — the RIGHT print column: your last name, then the spouse's last name.
const GRP_RIGHT: u32 = 1;

/// Fill Form 1040-V for `year` from the assembled [`PrintedReturn`], paying `amount`.
///
/// `amount` is a caller decision, not a derivation: the export hook passes Form 1040 line 37 as
/// printed, or the partial payment the filer chose with `--pay`. It is refused unless it is a
/// POSITIVE WHOLE-DOLLAR amount — a voucher for $0 is not a payment (the export writes none on a
/// return that owes nothing), a negative one is not a payment either, and cents on a voucher whose
/// return is printed in whole dollars (§3.1) would make the two disagree about what is owed.
///
/// The YEAR is a separate argument, exactly as [`crate::fill_full_return`] takes it: `PrintedReturn`
/// carries no year, and the year selects the map and the bundled template. Every written value is
/// read back from the SERIALIZED bytes through the geometric oracle, so a mis-mapped cell FAILS
/// CLOSED with no bytes returned.
pub fn fill_form_1040v(pr: &PrintedReturn, year: i32, amount: Usd) -> Result<Vec<u8>, FormsError> {
    fill_form_1040v_with_map(pr, amount, &Form1040VMap::for_year(year)?)
}

/// [`fill_form_1040v`] against a CALLER-SUPPLIED map — the fault-injection seam the KATs need. The
/// template year comes from the map's own `year` key, so a doctored map still meets its own PDF.
///
/// ★ This is what makes the geometric read-back testable rather than merely present: a committed map
/// cannot be corrupted in place, so without this entry point "a mis-mapped cell fails closed" would
/// be an assertion nobody had ever watched discriminate (harness B1). It matters more here than
/// anywhere: measured in T1, FOUR of Form 4868's field names (`f1_11`…`f1_14`) also exist in the
/// voucher's AcroForm, so an existence check alone cannot tell these two maps apart.
pub fn fill_form_1040v_with_map(
    pr: &PrintedReturn,
    amount: Usd,
    map: &Form1040VMap,
) -> Result<Vec<u8>, FormsError> {
    refuse_unless_whole_nonnegative(amount, "1040-V", "3")?;
    if amount == Usd::ZERO {
        return Err(FormsError::InvalidValue {
            form: "1040-V",
            line: "3",
            detail: "a payment voucher for $0 is not a payment — a return that owes nothing needs \
                     no Form 1040-V"
                .to_string(),
        });
    }

    // Load FIRST: the SSN cells' own /MaxLen decides hyphenated-vs-digits (`cells::render_ssn`).
    let mut doc = pdf::load(Form1040VMap::bundled_pdf(map.year)?)?;
    let blank_fields = pdf::collect_fields(&doc)?;
    let max_len_of = |fqn: &str| -> Option<usize> {
        blank_fields
            .iter()
            .find(|f| f.fqn == fqn)
            .and_then(|f| f.max_len)
    };

    let mut writes: Vec<(String, FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // An EMPTY value is left blank rather than written as "" — a written empty string is a filled
    // field to `assert_only_filled` and a blank to a human, which is the worst of both.
    let mut text = |fqn: &str, value: &str, descent: Option<(u32, u32)>| {
        if value.is_empty() {
            return;
        }
        writes.push((fqn.to_string(), FieldValue::Text(value.to_string())));
        placements.push(match descent {
            Some((g, o)) => FlatPlacement::free_ordered(fqn.to_string(), 0, g, o),
            None => FlatPlacement::free(fqn.to_string(), 0),
        });
    };

    // Box 1 — *"Line 1. Enter your social security number (SSN)."* *"If you are filing a joint
    // return, enter the SSN shown first on"* *"your return."* — which is `taxpayer`, the person the
    // 1040 header already prints first.
    let taxpayer_ssn =
        crate::cells::render_ssn(&pr.header.taxpayer.ssn, max_len_of(&map.box1_ssn))?;
    text(&map.box1_ssn, &taxpayer_ssn, None);

    // Box 4 — *"Enter your name(s) and address exactly as shown on your return."*
    text(
        &map.box4_first_name,
        &pr.header.taxpayer.first_name,
        Some((GRP_LEFT, 0)),
    );
    text(
        &map.box4_last_name,
        &pr.header.taxpayer.last_name,
        Some((GRP_RIGHT, 0)),
    );

    // ★ THE SPOUSE ROW IS A JOINT-RETURN ROW, gated on the FILING STATUS and never on the presence
    // of a spouse. `ReturnHeader.spouse` is present for MFS too — the spouse's name has its own 1040
    // cell there — and box 2 asks only *"If a joint return, SSN shown second"* *"on your return."*
    // An MFS voucher carrying the spouse's SSN would claim a joint return that was not filed.
    if pr.filing_status == FilingStatus::Mfj {
        if let Some(sp) = &pr.header.spouse {
            let spouse_ssn = crate::cells::render_ssn(&sp.ssn, max_len_of(&map.box2_spouse_ssn))?;
            text(&map.box2_spouse_ssn, &spouse_ssn, None);
            text(&map.spouse_first_name, &sp.first_name, Some((GRP_LEFT, 1)));
            text(&map.spouse_last_name, &sp.last_name, Some((GRP_RIGHT, 1)));
        }
    }

    text(
        &map.address_street,
        &pr.header.address_street,
        Some((GRP_LEFT, 2)),
    );
    // `Apt. no.` — RECORDED BLANK. `ReturnHeader` has no apartment field (the street line carries
    // whatever the filer typed), so nothing can populate this cell; splitting a street string on a
    // guess would put an invented apartment number on a filed payment.
    let _ = &map.address_apt;
    text(&map.address_city, &pr.header.address_city, None);
    text(&map.address_state, &pr.header.address_state, None);
    text(&map.address_zip, &pr.header.address_zip, None);

    // Box 3 — *"Line 3. Enter the amount you are paying by check or money"* *"order."*
    writes.push((map.box3_amount.clone(), FieldValue::Text(fmt_money(amount))));
    placements.push(FlatPlacement::col_only(
        map.box3_amount.clone(),
        0,
        F1040V_COL_AMOUNT,
    ));

    let index = pdf::index(&blank_fields);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // True read-back: re-parse the SERIALIZED output and verify geometry against the PDF's own rects.
    // The no-unmapped leg is what holds "the three foreign boxes and `Apt. no.` stay blank" — none is
    // in `placements`, so a stray write of any of them fails the fill closed.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, F1040V_CLUSTERS)?;
    Ok(bytes)
}
