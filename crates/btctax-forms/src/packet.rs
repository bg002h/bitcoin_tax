//! **The assembled packet** (P6.3b) — every form of one filed return, in the order the filer staples them.
//!
//! ★ **All-or-nothing.** If any member filler refuses, ZERO bytes come back. A 1040 whose line 2b cites a
//! Schedule B that is not attached is a *wrong return*, so partial emission would be a fail-OPEN — the
//! one failure mode this crate exists to prevent. Every filler runs before anything reaches the caller.
//!
//! ★ **Exhaustive destructure, no `..`.** [`fill_full_return`] pattern-matches every field of
//! [`PrintedReturn`], so a form ADDED to the packet without a filler here is a COMPILE error, and so is
//! the reverse. That is the anti-drift mechanism the architect prescribed, and it only works if nothing
//! is allowed to fall through silently.
//!
//! ★ **Attachment Sequence No. order.** The IRS prints a sequence number on every schedule's header, and
//! the packet is emitted in that order (1040 first, then ascending) — read off the bundled TY2024 PDFs,
//! not from memory. The filer gets their stapling order for free.

use crate::error::FormsError;
use btctax_core::tax::packet::PrintedReturn;

/// One filled form: its short name (the map/PDF stem) and its serialized bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedForm {
    /// The form's file stem — `"f1040"`, `"f1040s1"`, `"schedule_d"`, …
    pub name: String,
    /// IRS **Attachment Sequence No.** as printed on the form (`"01"`, `"12A"`, `"71"`; the 1040 itself
    /// has none and sorts first). Carried so the manifest can show the filer their stapling order.
    pub attachment_sequence: Option<&'static str>,
    /// The serialized PDF.
    pub bytes: Vec<u8>,
}

/// Fill every form of an assembled [`PrintedReturn`], in IRS Attachment Sequence order.
///
/// All-or-nothing: any member filler's refusal (a Schedule B that overflows its payer rows, a value too
/// long for its comb cell, a map missing its identity block) aborts the whole packet with zero bytes
/// written, and the error names WHICH form refused.
/// A non-PDF page the return must carry — an IRS-mandated continuation statement.
///
/// ★★★ It rides INSIDE `fill_full_return`, produced by the same call that checks the box it belongs to.
/// A parallel `write_*_txt` helper was the alternative and it carries a known cost: `write_form_8275_txt`
/// must be reached from four call sites plus the TUI (`render.rs:937-952` documents it). A missing
/// Form 8275 copy is a redundancy; **a missing dependents statement is a filed return with a checked
/// box and no attachment.** The box and the page cannot be produced by different code paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedStatement {
    /// The file stem — `"dependents_statement"`.
    pub name: String,
    /// The rendered page.
    pub body: String,
}

/// The complete filed packet: PDFs plus any statements they oblige.
#[derive(Debug, Clone, Default)]
pub struct FiledPacket {
    pub forms: Vec<NamedForm>,
    pub statements: Vec<NamedStatement>,
}

/// ★ The form's printed **"Attachment Sequence No."** — the stapling order, and the packet's filename
/// prefix (`admin.rs`: *"the prefix IS the stapling order"*). One function, keyed by `(stem, year)`,
/// because the number is a property of the REVISION the year ships, not of the form: Form 8283 was
/// **155** on Rev. 12-2014 / Rev. 12-2023 (TY2017, TY2024) and is **36** on Rev. 12-2025 (TY2025).
/// Until 2026-09-05 this was sixteen per-form literals with `"155"` for every year, so the TY2025
/// packet stapled Form 8283 last instead of between Form 6251 (32) and Form 8995 (55) — found by
/// building the year-package row's `attachment_sequence` kill (design r2 §4) before a single row had
/// been typed.
///
/// `None` is the 1040 itself, which carries no sequence number. Every value here is held to the map
/// row's `attachment_sequence` — which is read off the archived extract, never typed — by
/// `tests/map_rows.rs::packet_sequences_agree_with_every_map_row`; a literal that drifts from the form
/// reds there. (Design r2 §9 retires these literals into the row; this function is the one place
/// they live until then.)
pub fn attachment_sequence(stem: &str, year: i32) -> Option<&'static str> {
    match stem {
        "f1040" => None,
        "f1040s1" => Some("01"),
        "f1040s1a" => Some("1A"),
        "f1040s2" => Some("02"),
        "f1040s3" => Some("03"),
        "f1040sa" => Some("07"),
        "f1040sb" => Some("08"),
        "f1040sc" => Some("09"),
        "schedule_d" => Some("12"),
        "f8949" => Some("12A"),
        "schedule_se" => Some("17"),
        "f6251" => Some("32"),
        "f8995" => Some("55"),
        "f8995a" => Some("55A"),
        "f8959" => Some("71"),
        "f8960" => Some("72"),
        "f8275" => Some("92"),
        // Rev. 12-2025 renumbered Form 8283 from 155 to 36; TY2017 (Rev. 12-2014) and TY2024
        // (Rev. 12-2023) print 155.
        "f8283" => Some(if year >= 2025 { "36" } else { "155" }),
        _ => None,
    }
}

pub fn fill_full_return(pr: &PrintedReturn, year: i32) -> Result<FiledPacket, FormsError> {
    // ★ NO `..` — adding a member to `PrintedForms` without filling it here is a compile error.
    let PrintedReturn {
        header,
        filing_status,
        forms:
            btctax_core::tax::packet::PrintedForms {
                f1040,
                sch_1,
                sch_2,
                sch_3,
                sch_a,
                sch_b,
                sch_c,
                sch_d,
                f8949,
                sch_se,
                f8959,
                f8960,
                f8995,
                f8995a,
                f6251,
                f8283,
                f8275,
            },
    } = pr;

    let mut out: Vec<NamedForm> = Vec::new();
    let mut push = |name: &str, seq: Option<&'static str>, bytes: Vec<u8>| {
        out.push(NamedForm {
            name: name.to_string(),
            attachment_sequence: seq,
            bytes,
        });
    };

    // The 1040 itself — no sequence number; it IS the return.
    push(
        "f1040",
        attachment_sequence("f1040", year),
        crate::fill_form_1040_full(f1040, header, *filing_status, year)?,
    );

    // …then ascending Attachment Sequence No., as printed on each form.
    if let Some(l) = sch_1 {
        push(
            "f1040s1",
            attachment_sequence("f1040s1", year),
            crate::fill_schedule_1(l, header, year)?,
        );
    }
    if let Some(l) = sch_2 {
        push(
            "f1040s2",
            attachment_sequence("f1040s2", year),
            crate::fill_schedule_2(l, header, year)?,
        );
    }
    if let Some(l) = sch_3 {
        push(
            "f1040s3",
            attachment_sequence("f1040s3", year),
            crate::fill_schedule_3(l, header, year)?,
        );
    }
    if let Some(l) = sch_a {
        push(
            "f1040sa",
            attachment_sequence("f1040sa", year),
            crate::fill_schedule_a(l, header, year)?,
        );
    }
    if let Some(l) = sch_b {
        push(
            "f1040sb",
            attachment_sequence("f1040sb", year),
            crate::fill_schedule_b(l, header, year)?,
        );
    }
    if let Some(l) = sch_c {
        push(
            "f1040sc",
            attachment_sequence("f1040sc", year),
            crate::fill_schedule_c(l, header, year)?,
        );
    }

    // Schedule D files only when there IS capital activity — a W-2-only return with no disposals, no
    // capital-gain distributions and no carryover has no Schedule D to attach (the decision is a CORE
    // fact: `ScheduleDLines::must_file`). Form 8949 is the detail its lines 3/10 CITE.
    if sch_d.must_file() {
        push(
            "schedule_d",
            attachment_sequence("schedule_d", year),
            crate::fill_schedule_d_full(sch_d, header, year)?,
        );
        if let Some(p) = f8949 {
            push(
                "f8949",
                attachment_sequence("f8949", year),
                crate::fill_8949_full(p, header, year)?,
            );
        }
    }
    if let Some(l) = sch_se {
        push(
            "schedule_se",
            attachment_sequence("schedule_se", year),
            crate::fill_schedule_se_full(l, header, year)?,
        );
    }
    // ★★★ §G-6 — Form 6251, **Attachment Sequence No. 32** (read off the form itself, not guessed),
    //     so it staples between Schedule SE ("17") and Form 8995 ("55"). `Some` exactly when i6251's
    //     Who Must File condition 1 holds — core decides that, the filler transcribes it.
    if let Some(amt) = f6251 {
        push(
            "f6251",
            attachment_sequence("f6251", year),
            crate::form6251::fill_form_6251_with_map(
                amt,
                header,
                // FR/audit I-1: select by YEAR. A hardcoded 2024 map on a 2025 PDF writes the
                // AMT into line 10's box — wrong number, right-looking paper.
                &crate::map::Form6251Map::for_year(year)?,
            )?,
        );
    }
    if let Some(l) = f8995 {
        push(
            "f8995",
            attachment_sequence("f8995", year),
            crate::fill_form_8995(l, header, year)?,
        );
    }
    // ★★★ §G-28/B1a — Form 8995-A, filed INSTEAD of the simplified 8995 above the §199A(e)(2)
    //     threshold. Core guarantees exactly one of the two is `Some`; filing both would claim the
    //     deduction twice on paper. Attachment Sequence No. 55A, so it staples immediately after where
    //     8995 would have gone.
    if let Some(p4) = f8995a {
        push(
            "f8995a",
            attachment_sequence("f8995a", year),
            crate::form8995a::fill_form_8995a_with_map(
                &p4.part_iv,
                p4.parts_i_to_iii.as_ref(),
                header,
                &crate::map::Form8995AMap::for_year(year)?,
            )?,
        );
    }
    // Form 8959's filing decision is a CORE fact (`must_file`), not the filler's — the chain is built
    // either way because Schedule 2 and the 1040 read its printed lines.
    if let Some(bytes) = crate::fill_form_8959(f8959, header, year)? {
        push("f8959", attachment_sequence("f8959", year), bytes);
    }
    if let Some(l) = f8960 {
        push(
            "f8960",
            attachment_sequence("f8960", year),
            crate::fill_form_8960(l, header, year)?,
        );
    }
    // Form 8275 (Task 16) — Attachment Sequence No. 92. `Ok(None)` only when `printed.part_i` is
    // empty, which cannot happen here (`f8275` is `Some` only when core's `disclosure_8275` found a
    // promoted disposal leg, and that always yields a non-empty Part I) — the `if let` is defensive,
    // mirroring the same belt-and-suspenders pattern `fill_form_8959` uses above.
    if let Some(p) = f8275 {
        if let Some(bytes) = crate::fill_form_8275(p, header, year)? {
            push("f8275", attachment_sequence("f8275", year), bytes);
        }
    }
    if let Some(rows) = f8283 {
        if let Some(bytes) = crate::fill_form_8283_full(rows, header, year)? {
            push("f8283", attachment_sequence("f8283", year), bytes);
        }
    }

    // ★★★ The continuation statement, produced by the SAME call that checked the box on page 1.
    //     `more_than_four_dependents()` is the one predicate both read, so "box checked, no statement"
    //     and "statement, no box" are not expressible.
    let statements = btctax_core::tax::dependents_statement::dependents_statement(header, year)
        .map(|st| NamedStatement {
            name: "dependents_statement".to_string(),
            body: st.render(),
        })
        .into_iter()
        .collect();

    Ok(FiledPacket {
        forms: out,
        statements,
    })
}
