//! **Form 4868** (Application for Automatic Extension of Time To File U.S. Individual Income Tax
//! Return) fill — the extension application, read back through the flat-form geometric oracle
//! (column-x cluster + ordinal-y descent + /MaxLen + no-unmapped).
//!
//! **This module does no tax arithmetic.** Every printed figure is READ OFF the already-printed
//! return: line 4 is Form 1040 line 24, line 5 is Form 1040 line 33 less Schedule 3 line 10, and
//! line 6 subtracts one from the other exactly as the form says. A second, independent derivation
//! here is how a filed extension comes to disagree with the return it extends — so there isn't one.
//! The instructions ask for an *estimate* "of total tax liability"; btctax's estimate is the return
//! as it stands today, which is the most accurate one it has.
//!
//! **The two filer choices** are line 7 (how much to pay with the application) and line 8 (the
//! "out of the country" box). Both arrive in [`Form4868Choices`]; neither is inferred. Line 8 in
//! particular is an assertion the filer makes about themselves — silence FORGOES the extra two
//! months and asserts nothing, which is why an unchecked box is simply never written.
//!
//! **What is deliberately never written**, and is therefore held by the read-back's no-unmapped
//! scan: the fiscal-year header (btctax is calendar-year only, so the "For calendar year" default
//! the form already prints stands) and line 9 (`c1_2`, the Form 1040-NR box — btctax produces no
//! Form 1040-NR, and checking it would assert a return this build cannot file).

use crate::error::FormsError;
use crate::fmt_money;
use crate::map::Form4868Map;
use crate::pdf::{self, FieldValue};
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::packet::PrintedReturn;
use btctax_core::tax::types::FilingStatus;
use btctax_core::Usd;

/// Logical Form 4868 column: col 0 = Part II's money column (lines 4–7).
const F4868_COL_MONEY: usize = 0;

/// Hand-pinned column-x cluster, MEASURED from the blank PDFs with `xtask dump-fields`: lines 4–7
/// (`f1_11`…`f1_14`) all occupy `x 496.8 … 576.0` on **both** the TY2024 and the TY2025 revision.
/// This is the geometry ORACLE — deliberately code-side, NEVER read from the (distrusted) map, so a
/// map that pointed a money line at a Part I identity cell fails closed instead of printing a
/// balance due across the filer's address.
const F4868_CLUSTERS: &[(f32, f32)] = &[(496.8, 576.0)];

/// Descent group 0 — Part II's money lines 4, 5, 6, 7, top to bottom.
const GRP_MONEY: u32 = 0;
/// Descent group 1 — Part I's left column: the name line, the street, the city, the SSN row.
const GRP_IDENTITY: u32 = 1;

/// The two Form 4868 entries no return can answer — the filer's own choices (spec R2).
///
/// Neither has a safe default that could be inferred: paying nothing is a legitimate choice and so
/// is paying more than line 6, and "out of the country" is a fact about the filer that only they
/// hold. Both therefore arrive here explicitly, and `Default` is *pay nothing chosen, box
/// unchecked* — which is the silence the form reads as "no assertion made".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Form4868Choices {
    /// L7 — "Amount you're paying (see instructions)". `None` takes the default the spec's box-7 row
    /// names: the PRINTED Schedule 3 line 10 when the return already records an extension payment,
    /// else line 6. Whole dollars only; a negative or a value with cents is REFUSED.
    pub pay: Option<Usd>,
    /// L8 — "Check here if you're "out of the country" and a U.S. citizen or resident." `false`
    /// leaves the box unwritten (not "unchecked": on paper those are the same mark, and the
    /// difference is exactly that btctax asserts nothing).
    pub out_of_country: bool,
}

/// Form 4868 Part II, line by line — what this application will PRINT, with `None` meaning **blank**
/// and `Some(0)` meaning a printed `0`.
///
/// Split out of the fill so the arithmetic can be asserted without a PDF, and so `btctax extension`
/// can tell the filer what it filled using the same numbers the paper carries (two derivations of
/// "what is on line 6" is exactly how a screen and a form come to disagree).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Form4868Lines {
    /// L4 — "Estimate of total tax liability for <year>". Form 1040 line 24. The instructions:
    /// *"Enter on line 4 the total tax liability you expect to report on your"* … *"Form 1040,
    /// 1040-SR, or 1040-NR, line 24"*, and *"If you expect this amount to be zero, enter -0-."* — so
    /// this line is NEVER blank; a zero prints as a zero.
    pub line4: Usd,
    /// L5 — "Total <year> payments". *"Enter on line 5 the total payments you expect to report on
    /// your"* … *"Form 1040, 1040-SR, or 1040-NR, line 33 (excluding Schedule 3, line 10)"*, and
    /// *"Don't include on line 5 the amount you're paying with this"* *"Form 4868."* No `-0-` clause,
    /// so a zero is BLANK (`None`).
    pub line5: Option<Usd>,
    /// L6 — "Balance due. Subtract line 5 from line 4." *"If line 5 is more than line 4, enter -0-."*
    /// — so this line is never blank and never negative.
    pub line6: Usd,
    /// L7 — "Amount you're paying (see instructions)". Blank (`None`) when nothing is being paid;
    /// paying less than line 6 is explicitly allowed (*"If you find you can't pay the amount shown
    /// on line 6, you can still"* *"get the extension."*), and so is paying more.
    pub line7: Option<Usd>,
    /// L8 — the "out of the country" checkbox. `false` ⇒ the box is not written at all.
    pub line8: bool,
}

/// Refuse a payment a whole-dollar money line cannot carry.
///
/// Both halves matter and neither is cosmetic. A NEGATIVE payment is not a payment; printing one on
/// line 7 would tell the Service the filer is paying a negative amount with an application they are
/// signing. CENTS collide with the form's own all-or-nothing rounding rule (*"You can round off
/// cents to whole dollars on Form 4868. If you do round to whole dollars, you must"* *"round all
/// amounts."*) — lines 4–6 here are already whole dollars (§3.1), so a line 7 with cents is a form
/// contradicting itself. btctax will not round it silently: the filer chose the number.
pub(crate) fn refuse_unless_whole_nonnegative(
    pay: Usd,
    form: &'static str,
    line: &'static str,
) -> Result<(), FormsError> {
    if pay < Usd::ZERO {
        return Err(FormsError::InvalidValue {
            form,
            line,
            detail: format!("a payment cannot be negative (got {pay})"),
        });
    }
    if pay != pay.trunc() {
        return Err(FormsError::InvalidValue {
            form,
            line,
            detail: format!(
                "the money lines around it are whole dollars, so a payment with cents ({pay}) would \
                 make the form contradict its own rounding rule — enter whole dollars"
            ),
        });
    }
    Ok(())
}

/// Derive Form 4868's Part II from the already-printed return plus the filer's two choices.
///
/// Pure: no PDF, no map, no year. Every figure is a printed 1040/Schedule 3 line, transcribed.
pub fn form_4868_lines(
    pr: &PrintedReturn,
    choices: Form4868Choices,
) -> Result<Form4868Lines, FormsError> {
    if let Some(pay) = choices.pay {
        refuse_unless_whole_nonnegative(pay, "4868", "7")?;
    }

    // L4 — 1040 line 24, printed even when it is zero (the form's own `-0-` clause).
    let line4 = pr.forms.f1040.line24;

    // The Schedule 3 line 10 the RETURN prints ("Amount paid with request for extension to file").
    // `sch_3 = None` is the common case — no foreign tax credit, no extension payment, no excess
    // Social Security — and subtracting 0 is exactly right there.
    let extension_payment = pr.forms.sch_3.map_or(Usd::ZERO, |s| s.line10);

    // L5 — 1040 line 33 EXCLUDING Schedule 3 line 10. Line 33 already contains line 10 (via line 31
    // ← Schedule 3 line 15), which is why the instruction says to exclude it: the amount paid with
    // *this* application must not also be claimed as a payment already made.
    //
    // The floor is defensive, not arithmetic: line 33 contains line 10, so the difference cannot go
    // negative on a self-consistent return. Should one ever arrive, a blank line 5 says nothing,
    // where a printed negative would be false testimony.
    let line5_value = (pr.forms.f1040.line33 - extension_payment).max(Usd::ZERO);
    let line5 = (line5_value > Usd::ZERO).then_some(line5_value);

    // L6 — "Subtract line 5 from line 4. If line 5 is more than line 4, enter -0-."
    let line6 = (line4 - line5_value).max(Usd::ZERO);

    // L7 — the filer's choice; else the payment the return already records (so a filer who recorded
    // the payment first and printed second gets the number they already entered), else line 6.
    let line7_value = match choices.pay {
        Some(p) => p,
        None if extension_payment > Usd::ZERO => extension_payment,
        None => line6,
    };
    let line7 = (line7_value > Usd::ZERO).then_some(line7_value);

    Ok(Form4868Lines {
        line4,
        line5,
        line6,
        line7,
        line8: choices.out_of_country,
    })
}

/// Fill Form 4868 for `year` from the assembled [`PrintedReturn`] and the filer's choices, and
/// return the serialized PDF bytes.
///
/// The YEAR is a separate argument, exactly as [`crate::fill_full_return`] takes it: `PrintedReturn`
/// carries no year, and the year selects the map, the bundled template and the line set. Every
/// written value is read back from the SERIALIZED bytes through the geometric oracle, so a
/// mis-mapped cell FAILS CLOSED with no bytes returned.
pub fn fill_form_4868(
    pr: &PrintedReturn,
    year: i32,
    choices: Form4868Choices,
) -> Result<Vec<u8>, FormsError> {
    fill_form_4868_with_map(pr, choices, &Form4868Map::for_year(year)?)
}

/// [`fill_form_4868`] against a CALLER-SUPPLIED map — the fault-injection seam the KATs need. The
/// template year comes from the map's own `year` key, so a doctored map still meets its own PDF.
///
/// ★ This is what makes the geometric read-back testable rather than merely present: a committed map
/// cannot be corrupted in place, so without this entry point "a mis-mapped cell fails closed" would
/// be an assertion nobody had ever watched discriminate (harness B1).
pub fn fill_form_4868_with_map(
    pr: &PrintedReturn,
    choices: Form4868Choices,
    map: &Form4868Map,
) -> Result<Vec<u8>, FormsError> {
    let lines = form_4868_lines(pr, choices)?;

    // Load FIRST: the SSN cells' own /MaxLen decides hyphenated-vs-digits (`cells::render_ssn`).
    let mut doc = pdf::load(Form4868Map::bundled_pdf(map.year)?)?;
    let blank_fields = pdf::collect_fields(&doc)?;
    let max_len_of = |fqn: &str| -> Option<usize> {
        blank_fields
            .iter()
            .find(|f| f.fqn == fqn)
            .and_then(|f| f.max_len)
    };

    let mut writes: Vec<(String, FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // ── Part I — Identification ─────────────────────────────────────────────────────────────────
    // Bound BY NAME (spec R1's recorded deviation): the printed numbers 1/2/3 never form a label
    // column the reader can join, so the map spells these cells `name_line` / `taxpayer_ssn` / … and
    // carries the printed line number in each comment.
    //
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

    // L1 — "Your name(s) (see instructions)". The RETURN's own name line, so a joint extension
    // carries both spouses' names in the order the return shows them (the instructions:
    // *"If you plan to file a joint return, include both spouses' names in the order in which they
    // will appear on the return."*).
    text(
        &map.name_line,
        &pr.header.name_line,
        Some((GRP_IDENTITY, 0)),
    );
    text(
        &map.address_street,
        &pr.header.address_street,
        Some((GRP_IDENTITY, 1)),
    );
    text(
        &map.address_city,
        &pr.header.address_city,
        Some((GRP_IDENTITY, 2)),
    );
    // State and ZIP sit on the city's own printed row, so they carry no ordering of their own.
    text(&map.address_state, &pr.header.address_state, None);
    text(&map.address_zip, &pr.header.address_zip, None);

    // L2 — "Your social security number" (/MaxLen 11 ⇒ hyphenated).
    let taxpayer_ssn =
        crate::cells::render_ssn(&pr.header.taxpayer.ssn, max_len_of(&map.taxpayer_ssn))?;
    text(&map.taxpayer_ssn, &taxpayer_ssn, Some((GRP_IDENTITY, 3)));

    // L3 — "Spouse's social security number". ★ JOINT RETURNS ONLY. `ReturnHeader.spouse` is present
    // for MFS too (the spouse's name has its own 1040 cell there), so the gate is the FILING STATUS,
    // never the presence of a spouse. The instructions say what line 3 is for: *"If you plan to file
    // a joint return, enter on line 2 the social security"* … *"the other SSN to be shown on the
    // joint return."* An MFS extension that printed the spouse's SSN would claim a joint return.
    if pr.filing_status == FilingStatus::Mfj {
        if let Some(sp) = &pr.header.spouse {
            let spouse_ssn = crate::cells::render_ssn(&sp.ssn, max_len_of(&map.spouse_ssn))?;
            text(&map.spouse_ssn, &spouse_ssn, None);
        }
    }

    // ── Part II — Individual Income Tax ─────────────────────────────────────────────────────────
    let mut money = |fqn: &str, value: Usd, ord: u32| {
        writes.push((fqn.to_string(), FieldValue::Text(fmt_money(value))));
        placements.push(FlatPlacement::cell(
            fqn.to_string(),
            0,
            F4868_COL_MONEY,
            GRP_MONEY,
            ord,
        ));
    };
    money(&map.line4, lines.line4, 0);
    if let Some(v) = lines.line5 {
        money(&map.line5, v, 1);
    }
    money(&map.line6, lines.line6, 2);
    if let Some(v) = lines.line7 {
        money(&map.line7, v, 3);
    }

    // L8 — the checkbox. An UNCHECKED box is not written at all; on the printed page an unchecked
    // box and a never-written box are the same mark, and only the read-back test tells them apart.
    if lines.line8 {
        writes.push((
            map.line8.field.clone(),
            FieldValue::Check {
                on: map.line8.on.clone(),
            },
        ));
        placements.push(FlatPlacement::check(map.line8.field.clone(), 0));
    }

    let index = pdf::index(&blank_fields);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // True read-back: re-parse the SERIALIZED output and verify geometry against the PDF's own rects.
    // The no-unmapped leg is what holds "the fiscal-year header stays blank" and "line 9 is never
    // checked" — neither is in `placements`, so a stray write of either fails the fill closed.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, F4868_CLUSTERS)?;
    Ok(bytes)
}
