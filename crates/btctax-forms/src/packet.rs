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
pub fn fill_full_return(pr: &PrintedReturn, year: i32) -> Result<Vec<NamedForm>, FormsError> {
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
        None,
        crate::fill_form_1040_full(f1040, header, *filing_status, year)?,
    );

    // …then ascending Attachment Sequence No., as printed on each form.
    if let Some(l) = sch_1 {
        push(
            "f1040s1",
            Some("01"),
            crate::fill_schedule_1(l, header, year)?,
        );
    }
    if let Some(l) = sch_2 {
        push(
            "f1040s2",
            Some("02"),
            crate::fill_schedule_2(l, header, year)?,
        );
    }
    if let Some(l) = sch_3 {
        push(
            "f1040s3",
            Some("03"),
            crate::fill_schedule_3(l, header, year)?,
        );
    }
    if let Some(l) = sch_a {
        push(
            "f1040sa",
            Some("07"),
            crate::fill_schedule_a(l, header, year)?,
        );
    }
    if let Some(l) = sch_b {
        push(
            "f1040sb",
            Some("08"),
            crate::fill_schedule_b(l, header, year)?,
        );
    }
    if let Some(l) = sch_c {
        push(
            "f1040sc",
            Some("09"),
            crate::fill_schedule_c(l, header, year)?,
        );
    }

    // Schedule D files only when there IS capital activity — a W-2-only return with no disposals, no
    // capital-gain distributions and no carryover has no Schedule D to attach (the decision is a CORE
    // fact: `ScheduleDLines::must_file`). Form 8949 is the detail its lines 3/10 CITE.
    if sch_d.must_file() {
        push(
            "schedule_d",
            Some("12"),
            crate::fill_schedule_d_full(sch_d, header, year)?,
        );
        if let Some(p) = f8949 {
            push(
                "f8949",
                Some("12A"),
                crate::fill_8949_full(p, header, year)?,
            );
        }
    }
    if let Some(l) = sch_se {
        push(
            "schedule_se",
            Some("17"),
            crate::fill_schedule_se_full(l, header, year)?,
        );
    }
    if let Some(l) = f8995 {
        push("f8995", Some("55"), crate::fill_form_8995(l, header, year)?);
    }
    // Form 8959's filing decision is a CORE fact (`must_file`), not the filler's — the chain is built
    // either way because Schedule 2 and the 1040 read its printed lines.
    if let Some(bytes) = crate::fill_form_8959(f8959, header, year)? {
        push("f8959", Some("71"), bytes);
    }
    if let Some(l) = f8960 {
        push("f8960", Some("72"), crate::fill_form_8960(l, header, year)?);
    }
    // Form 8275 (Task 16) — Attachment Sequence No. 92. `Ok(None)` only when `printed.part_i` is
    // empty, which cannot happen here (`f8275` is `Some` only when core's `disclosure_8275` found a
    // promoted disposal leg, and that always yields a non-empty Part I) — the `if let` is defensive,
    // mirroring the same belt-and-suspenders pattern `fill_form_8959` uses above.
    if let Some(p) = f8275 {
        if let Some(bytes) = crate::fill_form_8275(p, header, year)? {
            push("f8275", Some("92"), bytes);
        }
    }
    if let Some(rows) = f8283 {
        if let Some(bytes) = crate::fill_form_8283_full(rows, header, year)? {
            push("f8283", Some("155"), bytes);
        }
    }

    Ok(out)
}
