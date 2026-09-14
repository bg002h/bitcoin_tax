//! Form 8283 (Rev. 12-2025) fill: donee/appraiser IDENTITY + per-row property data, read back through
//! the SP2 flat oracle (per-column x-cluster + PER-COLUMN ordinal-y descent [R0-M1] + no-unmapped).
//!
//! **Scope (a fill/blank table; [R0-I4]):** we FILL from `form_8283()`/`DonationDetails` — the donee
//! name/EIN/address (Part V identity), the donee/date/description/FMV/cost per row, the appraiser
//! identity name/address/TIN (Part IV identity), and (Section B) the "**k Digital assets**" property-
//! type box. We leave BLANK every OTHER party's declaration/signature: the Part II restriction
//! questions, the Part III taxpayer signature, the Part IV appraiser SIGNATURE/date, and the Part V
//! donee ACKNOWLEDGMENT (receipt date, "unrelated use?", authorized signature/title/date). A Section-B
//! 8283 without a signed Part IV/V is NOT filing-ready — the CLI says so and escalates when any row
//! `needs_review`.
//!
//! **Conditional + overflow:** written only when donations exist; one row per `RemovalLeg`, so a
//! multi-lot donation overflows the 4 Section-A / 3 Section-B rows onto additional form copies via
//! [`crate::overflow::merge_copies`] ("Attach one or more Forms 8283" sanctions it).

use crate::cells::push_money;
use crate::error::FormsError;
use crate::map::Form8283Map;
use crate::verify::{verify_flat, FlatPlacement};
use crate::{fmt_date, overflow, pdf};
use btctax_core::tax::form8283_section_a::{DateAcquiredByDonor, SectionAColumnsEfg, SectionARow};
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::{DonationDetails, Form8283HowAcquired, Form8283Row, Form8283Section};
use time::macros::format_description;

/// Section A column x-clusters (hand-pinned), **per form revision**: donee(a), desc(c), date_contrib(d),
/// date_acq(e), how(f), cost(g), fmv(h), method(i).
///
/// ★ The `SEC_A_CLUSTERS_2017` / `SEC_B_CLUSTERS_2017` bands were dropped 2026-09-06 with the TY2017
/// form package (owner ruling S9). On that Rev. 12-2014 form the money columns were dollars+cents
/// pairs whose clusters had to EXCLUDE the narrow cents widget (Section A dollars cx: cost≈317,
/// fmv≈403, cents cx≈360/446; Section B dollars cx: fmv≈526, cost≈266, deduction≈439, cents
/// cx≈569/309/482) — the recorded evidence for why the panic below refuses to widen to a wildcard.
const SEC_A_CLUSTERS_2023: &[(f32, f32)] = &[
    (58.0, 230.0),
    (404.0, 576.0),
    (58.0, 122.0),
    (123.0, 186.0),
    (188.0, 280.0),
    (281.0, 352.0),
    (353.0, 424.0),
    (426.0, 576.0),
];
/// Section B column x-clusters (hand-pinned), **per form revision**: desc(a), fmv(c), date_acq(d),
/// how(e), cost(f), deduction(i).
const SEC_B_CLUSTERS_2023: &[(f32, f32)] = &[
    (59.0, 258.0),
    (504.0, 576.0),
    (58.0, 130.0),
    (131.0, 287.0),
    (288.0, 359.0),
    (504.0, 576.0),
];

fn sec_clusters(year: i32, section: Form8283Section) -> &'static [(f32, f32)] {
    match (year, section) {
        // ★★★ **THE THIRD WILDCARD, and it was found by a reviewer holding all three at once.**
        //
        //     `form1040.rs` and `schedule_se.rs` had byte-for-byte this defect and both were fixed
        //     earlier the same day — and this one was not, because no agent held both files. That is
        //     CLAUDE.md B3's own case: **the failure mode is a field of view, not ignorance.**
        //
        //     A wildcard does not merely leave a year unchecked; it DISARMS the dollars/cents column
        //     guard, because an unsupported year silently inherits another revision's x-bands and
        //     the guard then "passes" against the wrong column.
        (2024 | 2025, Form8283Section::A) => SEC_A_CLUSTERS_2023,
        (2024 | 2025, Form8283Section::B) => SEC_B_CLUSTERS_2023,
        (other, _) => panic!(
            "sec_clusters: no Form 8283 amount-column band recorded for TY{other}. Measure it off \
             that year's blank PDF (xtask dump-fields) and add the arm — never widen this to a \
             wildcard, which hands the year another revision's bands and disarms the column guard."
        ),
    }
}

/// Render Form 8283 "how acquired by donor" as the form word. `Review` (acquisition origin lost) is an
/// honest blank — the row is separately flagged `needs_review`.
fn how_str(h: Form8283HowAcquired) -> &'static str {
    match h {
        Form8283HowAcquired::Purchased => "Purchased",
        Form8283HowAcquired::Gift => "Gift",
        Form8283HowAcquired::Other => "Other",
        Form8283HowAcquired::Review => "",
    }
}

/// Format a date as **MM/YYYY** — Form 8283's "(mo., yr.)" date-acquired format (NOT SP1's MM/DD/YYYY).
fn fmt_mo_yr(d: btctax_core::TaxDate) -> Result<String, FormsError> {
    let fmt = format_description!("[month]/[year]");
    d.format(&fmt)
        .map_err(|e| FormsError::Structure(format!("mo/yr date format: {e}")))
}

/// Render Section A column **(e)** — either a `MM/YYYY` date, or the form's own word for a group of
/// similar items acquired on various dates (i8283 column (e)). The word is
/// [`DateAcquiredByDonor::VARIOUS_WORD`], so the spelling has one definition.
fn fmt_date_acquired(e: DateAcquiredByDonor) -> Result<String, FormsError> {
    match e {
        DateAcquiredByDonor::On(d) => fmt_mo_yr(d),
        DateAcquiredByDonor::Various => Ok(DateAcquiredByDonor::VARIOUS_WORD.to_string()),
    }
}

/// Fill Form 8283 from the projected donation rows. `Ok(None)` when there are no donation rows.
///
/// **Section A** (≤ $5,000) count-overflows the flat rows: it has a per-row donee COLUMN and no Part
/// IV/V identity block, so each row already names its own donee — pagination is purely by count.
///
/// **Section B** (> $5,000) is "one donee's donation of similar property per form", so a year with
/// donations to MULTIPLE distinct donees needs one official 8283 per donee. Donations are grouped by
/// the donee + appraiser IDENTITY (the Part V donee AND the Part IV appraiser are both read from one
/// `DonationDetails`, so a same-donee/different-appraiser pair splits), then count-overflowed WITHIN
/// each group — and the group's `details` is passed EXPLICITLY into every physical copy, so a donee
/// whose legs overflow carries its identity on every page.
///
/// **[byte-identity]** the single-physical-copy case (the common single-donee year) returns the lone
/// `fill_one` result DIRECTLY; only ≥ 2 copies are routed through `merge_copies` (which re-loads/saves
/// and would otherwise break the byte-golden for that common case).
pub fn fill_form_8283(
    rows: &[Form8283Row],
    map: &Form8283Map,
) -> Result<Option<Vec<u8>>, FormsError> {
    // The crypto slice writes no filer identity and no Section B declarations — and it has no
    // non-crypto noncash gifts at all: its 8283 rides beside a return btctax did not produce, so there
    // is no Schedule A to read them from.
    fill_form_8283_inner(rows, map, None, None, &[])
}

/// The **full-return** Form 8283: whole-dollar rows (`Printed8283Rows` — a newtype precisely so a CENTS
/// row cannot be handed here by accident) plus the FILER's identity block, which the slice never writes.
pub fn fill_form_8283_full(
    printed: &btctax_core::tax::printed::Printed8283Rows,
    header: &ReturnHeader,
    map: &Form8283Map,
) -> Result<Option<Vec<u8>>, FormsError> {
    fill_form_8283_inner(
        printed.rows(),
        map,
        Some(header),
        printed.restrictions_answer(),
        printed.section_a_noncash(),
    )
}

fn fill_form_8283_inner(
    rows: &[Form8283Row],
    map: &Form8283Map,
    filer: Option<&ReturnHeader>,
    // ★ §G-21 — the filer's answer to lines 5a/5b/5c. `Some(false)` ⇒ all three print No; `None` ⇒ all
    // three stay BLANK. The crypto slice always passes `None`: it writes no Section B declarations.
    no_restrictions: Option<bool>,
    // ★★★ FR-200(b) — the TRANSCRIBED Section A rows for non-crypto noncash gifts.
    noncash_section_a: &[SectionARow],
) -> Result<Option<Vec<u8>>, FormsError> {
    // ★ BOTH halves: a $600 bag of clothes and no ledger donation at all is exactly the FR-200(b) case,
    //   and testing only `rows` would drop the whole form.
    if rows.is_empty() && noncash_section_a.is_empty() {
        return Ok(None);
    }

    // ★★★ **PARTITION BY SECTION, rather than reading ONE section off the first carrier.**
    //
    //     Until FR-200(b) every row on the form came from the ledger, where the section is uniform
    //     across the year (all BTC is one "similar property" class), so the first carrier's section was
    //     the form's section. A non-crypto noncash gift breaks that: a year with >$5,000 of donated
    //     bitcoin (Section B) and a $600 bag of clothes (Section A) needs BOTH sections, and reading one
    //     section off the first carrier would have printed the clothes into Section B's property table —
    //     a row on the wrong section of the form, under a qualified-appraisal declaration nobody made.
    //
    //     ★ It is a no-op for every input that existed before: the ledger's rows all carry one section,
    //       so one partition is empty and the surviving branch is byte-for-byte today's behaviour.
    let (ledger_a, ledger_b) = partition_by_section(rows);

    // ── Section A: ONE list, derived — the ledger's Section A rows transcribed into the form's own
    //    columns, then the noncash gifts' transcriptions. Overflow chunks THIS list, so a fifth gift
    //    cannot fall off the end of a four-row table: it starts row A of a second copy.
    let mut section_a: Vec<SectionARow> =
        Vec::with_capacity(ledger_a.len() + noncash_section_a.len());
    for r in &ledger_a {
        section_a.push(ledger_section_a_row(r)?);
    }
    section_a.extend(noncash_section_a.iter().cloned());

    // Per-copy row capacity = the number of rows the year's map ENUMERATES (4 Section A / 3 Section B on
    // 2024/2025) — per-year DATA, not a hard-coded constant.
    let cap_a = map.section_a.rows.len().max(1);
    let cap_b = map.section_b.rows.len().max(1);

    // Build the physical copies. Each copy is filled on ORIGINAL field names + geometry-verified
    // (fails closed) inside `fill_one`; ≥ 2 copies are merged (per-copy root rename) afterwards.
    let mut copies: Vec<Vec<u8>> = Vec::new();
    if !section_a.is_empty() {
        let n_copies = section_a.len().div_ceil(cap_a);
        for k in 0..n_copies {
            let chunk: Vec<&SectionARow> = section_a.iter().skip(k * cap_a).take(cap_a).collect();
            copies.push(fill_one(
                Chunk::A(&chunk),
                map,
                None,
                filer,
                no_restrictions,
            )?);
        }
    }
    // Section B: group donations by donee + appraiser identity, then count-overflow each group.
    for group in group_section_b(&ledger_b) {
        let n = group.rows.len().div_ceil(cap_b).max(1);
        for k in 0..n {
            let chunk: Vec<&Form8283Row> = group
                .rows
                .iter()
                .skip(k * cap_b)
                .take(cap_b)
                .copied()
                .collect();
            copies.push(fill_one(
                Chunk::B(&chunk),
                map,
                group.details,
                filer,
                no_restrictions,
            )?);
        }
    }

    // [byte-identity] a single physical copy is returned DIRECTLY (no re-load/save through
    // `merge_copies`); only ≥ 2 copies are merged.
    if copies.len() == 1 {
        Ok(Some(copies.into_iter().next().expect("exactly one copy")))
    } else {
        Ok(Some(overflow::merge_copies(&copies)?))
    }
}

/// Split the ledger's rows into the Section A donations' rows and the Section B donations' rows,
/// keeping every non-carrier leg with its carrier.
///
/// ★ `row.section.is_some()` is the canonical carrier signal (`form_8283()` sets it unconditionally on
/// the first leg), the same one [`group_section_b`] partitions on. Leg rows preceding any carrier — a
/// degenerate input `form_8283()` never emits — go to Section A, which is where the old
/// `unwrap_or(Form8283Section::A)` fallback put them; either way **nothing is dropped**, which is the
/// property this function exists to keep.
///
/// ★★ **STATED HONESTLY, because a B1 plant measured it** (`CLAUDE.md`: *"state, in the source, exactly
/// what it covers and what it does not"*): the MIXED-ledger case — some donations Section A and some
/// Section B in one year — is **not reachable from any input `form_8283()` produces**, because the
/// ledger's section is decided once from the year aggregate and is uniform across the year. So this
/// function is defensive against a future row builder, and no test kills its mixed branch. What IS
/// killed, by `f8283_section_a.rs::a_section_b_crypto_year_with_a_noncash_gift_prints_both_sections`, is
/// the property that actually matters today: a non-crypto Section A gift prints in Section A whatever
/// section the LEDGER is in.
fn partition_by_section(rows: &[Form8283Row]) -> (Vec<&Form8283Row>, Vec<&Form8283Row>) {
    let mut a: Vec<&Form8283Row> = Vec::new();
    let mut b: Vec<&Form8283Row> = Vec::new();
    let mut current = Form8283Section::A;
    for row in rows {
        if let Some(s) = row.section {
            current = s;
        }
        match current {
            Form8283Section::A => a.push(row),
            Form8283Section::B => b.push(row),
        }
    }
    (a, b)
}

/// Transcribe one LEDGER row into Form 8283's own Section A columns.
///
/// ★★ Columns (e), (f) and (g) are always `Completed` for a ledger leg: a lot carries an acquisition
/// date, a `BasisSource` and a basis, so btctax is never in the position the per-item carve-out exists
/// for. Printing all three where the form does not require them is what the form invites — the note says
/// *"you do not **have to** complete"*, not "leave blank" — so this is behaviour-preserving as well as
/// correct.
///
/// ★ Column (i) can still be empty here (Section A's `fmv_method` is `""` unless the filer stored an
/// override), and `push_cell` leaves an empty cell unwritten. That is the pre-existing honest gap for
/// LEDGER donations; a NONCASH gift's empty (i) refuses instead
/// (`NoncashGiftRefusal::FmvMethodEmpty`), because that path has a filer to ask.
fn ledger_section_a_row(r: &Form8283Row) -> Result<SectionARow, FormsError> {
    Ok(SectionARow {
        col_a_donee_name_and_address: r.donee.clone(),
        col_c_description_and_condition: r.description.clone(),
        col_d_date_of_contribution: r.date_contributed,
        cols_efg: SectionAColumnsEfg::Completed {
            col_e_date_acquired_by_donor: DateAcquiredByDonor::On(r.date_acquired),
            col_f_how_acquired_by_donor: how_str(r.how_acquired).to_string(),
            col_g_cost_or_adjusted_basis: r.cost_basis,
        },
        col_h_fair_market_value: r.fmv,
        col_i_method_used_to_determine_fmv: r.fmv_method.clone(),
    })
}

/// One physical copy's payload: a Section A chunk (transcribed rows) or a Section B chunk (ledger rows).
///
/// ★ An enum rather than a `(section, rows_a, rows_b)` triple so the two cannot both be non-empty on one
/// copy — Section A and Section B live on different property tables of the form, and a copy is one or the
/// other.
enum Chunk<'a> {
    A(&'a [&'a SectionARow]),
    B(&'a [&'a Form8283Row]),
}

impl Chunk<'_> {
    /// Which section this copy fills — the one thing both arms have to answer, read off the variant
    /// rather than passed alongside it.
    fn section(&self) -> Form8283Section {
        match self {
            Chunk::A(_) => Form8283Section::A,
            Chunk::B(_) => Form8283Section::B,
        }
    }
}

/// A Section-B donee/appraiser identity group: all donations sharing one Part V donee AND Part IV
/// appraiser identity (first-seen order preserved), carrying the FIRST-SEEN carrier's `details`.
struct SectionBGroup<'a> {
    rows: Vec<&'a Form8283Row>,
    details: Option<&'a DonationDetails>,
}

/// The identity a Section-B donation is grouped by. The Part V donee (name + EIN) AND the Part IV
/// appraiser (name + TIN/PTIN) are both read from one `DonationDetails`, so grouping keys on both —
/// same donee, different appraiser ⇒ separate forms (a shared form would print a wrong Part IV). A
/// carrier with no captured `DonationDetails` keys on its donee LABEL only (a `None` return means an
/// empty label ⇒ its own singleton, never merged with another anonymous donee).
#[derive(PartialEq, Eq)]
enum IdentityKey {
    /// A carrier WITH `DonationDetails`: full donee + appraiser identity.
    Detailed {
        donee_name: String,
        donee_ein: Option<String>,
        appraiser_name: String,
        appraiser_id: Option<String>,
    },
    /// A carrier with NO details but a non-empty donee label.
    DoneeLabel(String),
}

/// The grouping key for a carrier, or `None` for an empty-key donation (no details + empty donee
/// label) that must occupy its own singleton group (two anonymous donees can never be merged).
fn identity_key(details: Option<&DonationDetails>, donee: &str) -> Option<IdentityKey> {
    match details {
        Some(d) => Some(IdentityKey::Detailed {
            donee_name: d.donee_name.clone(),
            donee_ein: d.donee_ein.clone(),
            appraiser_name: d.appraiser_name.clone(),
            appraiser_id: d.appraiser_tin.clone().or_else(|| d.appraiser_ptin.clone()),
        }),
        None if !donee.is_empty() => Some(IdentityKey::DoneeLabel(donee.to_string())),
        None => None,
    }
}

/// Partition Section-B `rows` into donations at carrier boundaries (`row.section.is_some()` — the
/// canonical carrier signal, set unconditionally by `form_8283()`, NOT `details.is_some()`), then
/// group the donations by donee + appraiser identity (first-seen order; split-on-difference; an
/// anonymous no-details donee is its own singleton). Leg rows (`section: None`) attach to their
/// carrier's group; any leading leg-rows before the first carrier (shouldn't occur — `form_8283()`
/// emits the carrier first) seed the first group so nothing is dropped.
fn group_section_b<'a>(rows: &[&'a Form8283Row]) -> Vec<SectionBGroup<'a>> {
    let mut groups: Vec<SectionBGroup<'a>> = Vec::new();
    let mut keys: Vec<Option<IdentityKey>> = Vec::new();
    let mut current: Option<usize> = None;
    for row in rows.iter().copied() {
        if row.section.is_some() {
            // A carrier begins a new donation; group it by donee + appraiser identity (an empty key
            // never merges — `and_then` short-circuits to a fresh group).
            let key = identity_key(row.details.as_ref(), &row.donee);
            let existing = key
                .as_ref()
                .and_then(|k| keys.iter().position(|gk| gk.as_ref() == Some(k)));
            current = Some(match existing {
                Some(i) => {
                    groups[i].rows.push(row);
                    i
                }
                None => {
                    groups.push(SectionBGroup {
                        rows: vec![row],
                        details: row.details.as_ref(),
                    });
                    keys.push(key);
                    groups.len() - 1
                }
            });
        } else {
            // A leg row attaches to its carrier's group (or seeds the first group if it precedes any
            // carrier — a degenerate input that `form_8283()` never emits).
            match current {
                Some(i) => groups[i].rows.push(row),
                None => {
                    groups.push(SectionBGroup {
                        rows: vec![row],
                        details: None,
                    });
                    keys.push(None);
                    current = Some(0);
                }
            }
        }
    }
    groups
}

/// A property-table text cell: written + authorized only when non-empty. `col` is both the x-cluster
/// index and the per-column ordinal-y descent group; `ord` is the row index (rows descend in y). The
/// page is derived from the fqn (the 2017 Section B property table is on page 2, not page 1).
fn push_cell(
    w: &mut Vec<(String, pdf::FieldValue)>,
    p: &mut Vec<FlatPlacement>,
    fqn: &str,
    value: String,
    col: usize,
    ord: u32,
) {
    if value.is_empty() {
        return;
    }
    w.push((fqn.to_string(), pdf::FieldValue::Text(value)));
    p.push(FlatPlacement::cell(
        fqn.to_string(),
        crate::cells::page_of(fqn),
        col,
        col as u32,
        ord,
    ));
}

/// A free-text identity cell (geometry-exempt, page-derived): written + authorized only when non-empty.
fn push_free(
    w: &mut Vec<(String, pdf::FieldValue)>,
    p: &mut Vec<FlatPlacement>,
    fqn: &str,
    value: &str,
) {
    if value.is_empty() {
        return;
    }
    w.push((fqn.to_string(), pdf::FieldValue::Text(value.to_string())));
    p.push(FlatPlacement::free(
        fqn.to_string(),
        crate::cells::page_of(fqn),
    ));
}

/// Fill one physical Form 8283 copy (a chunk of ≤ cap rows) and read it back geometrically. For
/// Section B, `details` (the copy's donee/appraiser identity, passed in by the caller — NOT sniffed
/// from a row in the chunk) fills the Part IV/V identity block, so every overflow page of a donee
/// carries that donee's identity.
fn fill_one(
    chunk: Chunk<'_>,
    map: &Form8283Map,
    details: Option<&DonationDetails>,
    filer: Option<&ReturnHeader>,
    // ★ §G-21 — see `fill_form_8283_inner`.
    no_restrictions: Option<bool>,
) -> Result<Vec<u8>, FormsError> {
    let section = chunk.section();
    let mut w: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut p: Vec<FlatPlacement> = Vec::new();

    match chunk {
        // ★★★ **Section A, straight off the transcription** — one `push` per lettered column, in the
        //     form's own order, reading `SectionARow`'s `col_<letter>_…` fields. Column (b) is the
        //     vehicle box: unmapped, and a vehicle refuses upstream, so nothing writes it.
        Chunk::A(rows) => {
            for (i, row) in rows.iter().enumerate() {
                let m = &map.section_a.rows[i];
                let ord = i as u32;
                // (a) Name and address of the donee organization
                push_cell(
                    &mut w,
                    &mut p,
                    &m.donee,
                    row.col_a_donee_name_and_address.clone(),
                    0,
                    ord,
                );
                // (c) Description and condition of donated property
                push_cell(
                    &mut w,
                    &mut p,
                    &m.desc,
                    row.col_c_description_and_condition.clone(),
                    1,
                    ord,
                );
                // (d) Date of the contribution
                push_cell(
                    &mut w,
                    &mut p,
                    &m.date_contrib,
                    fmt_date(row.col_d_date_of_contribution)?,
                    2,
                    ord,
                );
                // ★★★ (e)/(f)/(g) — the per-item carve-out, and the ONLY place an empty (e)(f)(g)
                //     comes from. `NotRequired…` writes nothing at all: no cell, no placement, no
                //     zero. A `$0` in column (g) would be testimony the filer never gave about a basis
                //     the form did not ask for — and the form says it does not have to be completed,
                //     not that it is zero.
                match &row.cols_efg {
                    SectionAColumnsEfg::Completed {
                        col_e_date_acquired_by_donor,
                        col_f_how_acquired_by_donor,
                        col_g_cost_or_adjusted_basis,
                    } => {
                        push_cell(
                            &mut w,
                            &mut p,
                            &m.date_acq,
                            fmt_date_acquired(*col_e_date_acquired_by_donor)?,
                            3,
                            ord,
                        );
                        push_cell(
                            &mut w,
                            &mut p,
                            &m.how,
                            col_f_how_acquired_by_donor.clone(),
                            4,
                            ord,
                        );
                        // (g) cost — a dollars+cents pair on older revisions.
                        push_money(
                            &mut w,
                            &mut p,
                            &m.cost,
                            *col_g_cost_or_adjusted_basis,
                            5,
                            Some((5, ord)),
                        );
                    }
                    SectionAColumnsEfg::NotRequiredDeductionAtOrUnderFiveHundred => {}
                }
                // (h) Fair market value
                push_money(
                    &mut w,
                    &mut p,
                    &m.fmv,
                    row.col_h_fair_market_value,
                    6,
                    Some((6, ord)),
                );
                // (i) Method used to determine the fair market value
                push_cell(
                    &mut w,
                    &mut p,
                    &m.method,
                    row.col_i_method_used_to_determine_fmv.clone(),
                    7,
                    ord,
                );
            }
        }
        Chunk::B(rows) => {
            let b = &map.section_b;
            // [★] The BTC property-type box: "k Digital assets" (2024/2025) or "j Other" (2017).
            w.push((
                b.k_digital_assets.field.clone(),
                pdf::FieldValue::Check {
                    on: b.k_digital_assets.on.clone(),
                },
            ));
            p.push(FlatPlacement::check(
                b.k_digital_assets.field.clone(),
                crate::cells::page_of(&b.k_digital_assets.field),
            ));
            for (i, row) in rows.iter().enumerate() {
                let m = &b.rows[i];
                let ord = i as u32;
                // A revision with no "k Digital assets" box (the Rev. 12-2014 form, where BTC went
                // under "j Other") gives no category, so the map carries a printed note prepended to
                // the FIRST row's (a) description. NO bundled map sets `btc_property_note` since S9
                // dropped the TY2017 package (2026-09-06) — the branch is the map's to switch on,
                // and stays because an older revision is exactly what it is for.
                let desc = match (i, &b.btc_property_note) {
                    (0, Some(note)) => format!("{note}: {}", row.description),
                    _ => row.description.clone(),
                };
                push_cell(&mut w, &mut p, &m.desc, desc, 0, ord);
                push_money(&mut w, &mut p, &m.fmv, row.fmv, 1, Some((1, ord)));
                push_cell(
                    &mut w,
                    &mut p,
                    &m.date_acq,
                    fmt_mo_yr(row.date_acquired)?,
                    2,
                    ord,
                );
                push_cell(
                    &mut w,
                    &mut p,
                    &m.how,
                    how_str(row.how_acquired).to_string(),
                    3,
                    ord,
                );
                push_money(&mut w, &mut p, &m.cost, row.cost_basis, 4, Some((4, ord)));
                // ★★★ P3 — "Amount claimed as a deduction" is written ONLY on a revision whose map
                // carries the cell. From 2026-08-21 that was the Rev. 12-2014 (TY2017) map alone, and
                // since S9 dropped that package (2026-09-06) it is NO bundled map: nothing writes
                // column (i) today. The condition stays because it is the map's to make.
                //
                // i8283, verbatim: "Complete column (i), amount claimed as a deduction, if you are a
                // pass-through entity or a member of a pass-through entity." An individual donating
                // their own bitcoin is neither, and btctax models no pass-through entity at all — the
                // header entity-name/TIN cells and the family-PTE box are censused `unmodeled` for
                // that same reason, and the 2024 cells now join them.
                //
                // It used to print `row.claimed_deduction`, which is the PRE-CEILING figure: on the
                // observed packet, $1,000,000 beside a Schedule A line 12 of $600,000. An entry the
                // form did not ask for, contradicting the return's own claimed deduction, on the
                // highest-scrutiny line of the highest-scrutiny form — and sworn to under §6065.
                // A blank forgoes nothing: the deduction is claimed on Schedule A line 12, not here.
                if let (Some(cell), Some(ded)) = (&m.deduction, row.claimed_deduction) {
                    push_money(&mut w, &mut p, cell, ded, 5, Some((5, ord)));
                }
            }
            // Part IV/III (appraiser) + Part V/IV (donee) IDENTITY — the copy's group identity,
            // passed in EXPLICITLY (so an overflow page carries it too; not sniffed from the chunk).
            if let Some(details) = details {
                // Appraiser printed-name field is absent on the Rev. 12-2014 form (identity = the
                // handwritten signature, left blank), so this write is conditional on the map.
                if let Some(name_field) = &b.appraiser_name {
                    push_free(&mut w, &mut p, name_field, &details.appraiser_name);
                }
                if let Some(a) = &details.appraiser_address {
                    push_free(&mut w, &mut p, &b.appraiser_address, a);
                }
                // §6695A appraiser identifier: TIN, else PTIN.
                if let Some(tin) = details
                    .appraiser_tin
                    .as_ref()
                    .or(details.appraiser_ptin.as_ref())
                {
                    push_free(&mut w, &mut p, &b.appraiser_tin, tin);
                }
                push_free(&mut w, &mut p, &b.donee_name, &details.donee_name);
                if let Some(ein) = &details.donee_ein {
                    push_free(&mut w, &mut p, &b.donee_ein, ein);
                }
                if let Some(addr) = &details.donee_address {
                    push_free(&mut w, &mut p, &b.donee_address, addr);
                }
            }
        }
    };
    // The blank's fields are needed for the identity cells' /MaxLen.
    let blank_for_identity = pdf::collect_fields(&pdf::load(pdf::f8283_pdf(map.year)?)?)?;
    // ★ The FILER's identity — "Name(s) shown on your income tax return" + identifying number. The
    // crypto slice never wrote it (its 8283 rides beside a return btctax did not produce); a FULL
    // return may not attach an unnamed substantiation form, so the full path passes a header and the
    // map must carry the cells (ARCH-P6.3a D7).
    if let Some(header) = filer {
        let blank = &blank_for_identity;
        let identity = map.identity.as_ref().ok_or_else(|| {
            FormsError::Geometry(format!(
                "the {} Form 8283 map has no [identity] block — a full return cannot file an unnamed \
                 Form 8283",
                map.year
            ))
        })?;
        crate::cells::push_identity(
            &mut w,
            &mut p,
            identity,
            &header.name_line,
            &header.taxpayer.ssn,
            blank,
        )?;
        // ★ …and PAGE 2's own header (§G-13). The form repeats it so a detached Section B page can
        // still be tied to its return, and btctax held the name and TIN all along — it simply had no
        // cells to write them into. Required in the same breath as page 1's, and for the same reason:
        // a full return may not attach an unnamed page of a substantiation form.
        let identity_page2 = map.identity_page2.as_ref().ok_or_else(|| {
            FormsError::Geometry(format!(
                "the {} Form 8283 map has no [identity_page2] block — page 2 would go out with no \
                 identifying header, so a detached Section B page could not be tied to its return",
                map.year
            ))
        })?;
        crate::cells::push_identity(
            &mut w,
            &mut p,
            identity_page2,
            &header.name_line,
            &header.taxpayer.ssn,
            blank,
        )?;

        // ★★★ §G-21 — Section B lines 5a/5b/5c, from the filer's ONE return-level answer.
        //
        // `Some(false)` — "no donation had strings attached" — is a UNIVERSAL, so each box's answer
        // follows for every donation on the form. `None` writes NOTHING: not asked and answered-no are
        // different marks on the page, and a printed "No" the filer never gave is fabricated testimony
        // (the `3b22ca1` defect class). `Some(true)` cannot arrive — `screen_absolute` refuses
        // the year, because the §170 deduction btctax computed at full FMV would then be too large.
        // ★★ r3 M-1 — SECTION B ONLY. Lines 5a/5b/5c live in Section B Part II and the form scopes
        // them explicitly: "Complete lines 5a through 5c if conditions were placed on a contribution
        // listed in Section B, Part I". This block sits outside the `match section` above (it shares
        // the filer-identity window), so without this guard a Section A return printed three answers
        // to questions about a Part I that is EMPTY — the same "a mark the form has no place for"
        // class `3bcf3a0` fixed for Schedule B's FBAR pair.
        if section == Form8283Section::B && no_restrictions == Some(false) {
            for pair in [&map.line5a, &map.line5b, &map.line5c] {
                let pair = pair.as_ref().ok_or_else(|| {
                    FormsError::Geometry(format!(
                        "the {} Form 8283 map has no 5a/5b/5c pair — a full return may not file a \
                         Section B with its restriction questions blank when the filer HAS answered",
                        map.year
                    ))
                })?;
                w.push((
                    pair.no.field.clone(),
                    pdf::FieldValue::Check {
                        on: pair.no.on.clone(),
                    },
                ));
                p.push(FlatPlacement::check(
                    pair.no.field.clone(),
                    crate::cells::page_of(&pair.no.field),
                ));
            }
        }
    }

    let clusters = sec_clusters(map.year, section);
    let writes = w;
    let placements = p;

    let mut doc = pdf::load(pdf::f8283_pdf(map.year)?)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // Read back the SERIALIZED output.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, clusters)?;
    Ok(bytes)
}

#[cfg(test)]
mod geometry_year_tests {
    use super::*;

    fn resolves(year: i32, section: Form8283Section) -> bool {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let ok = std::panic::catch_unwind(|| sec_clusters(year, section)).is_ok();
        std::panic::set_hook(prev);
        ok
    }

    /// ★★★ **The enumeration IS the guard, and the expected answer is DERIVED.**
    ///
    /// Transplanted from `form1040.rs`, where this test was written twice today while THIS file —
    /// carrying byte-for-byte the same wildcard — was missed, because no agent held both. That is
    /// `CLAUDE.md` B3: the failure mode is a field of view, not ignorance. So it lives here too,
    /// deriving its expected set from [`crate::SUPPORTED_YEARS`] rather than a hand-list, and reds
    /// in BOTH directions:
    ///
    /// * a year wired into the product with no amount-column band measured for it, and
    /// * a band handed to a year the product does not support — i.e. the wildcard is back.
    ///
    /// **Planted-defect check (B1):** restore `(_, Form8283Section::A) => SEC_A_CLUSTERS_2023` and
    /// this reds on the first unsupported probe year.
    #[test]
    fn form8283_geometry_is_recorded_for_exactly_the_supported_years() {
        for year in 2010..=2040 {
            let supported = crate::SUPPORTED_YEARS.contains(&year);
            for section in [Form8283Section::A, Form8283Section::B] {
                let answered = resolves(year, section);
                assert_eq!(
                    answered,
                    supported,
                    "TY{year} {section:?}: SUPPORTED_YEARS.contains = {supported} but sec_clusters \
                     {} — {}",
                    if answered { "answered" } else { "panicked" },
                    if supported {
                        "a supported year with no band recorded: measure the amount column off that \
                         year's blank PDF and add the arm"
                    } else {
                        "an unsupported year must NOT inherit another revision's x-band; the \
                         wildcard is back, and it DISARMS the dollars/cents column guard"
                    }
                );
            }
        }
    }
}
