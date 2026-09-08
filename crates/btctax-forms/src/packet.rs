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
#[derive(Debug, Clone)]
// ★ no `Default` either (fold review L7): `default()` + `pub` fields was a second way to an unsorted packet
#[non_exhaustive] // ★ a literal `FiledPacket { .. }` is not constructible outside this crate — go through `stapled` (steps-4/5 review R8)
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
        // ★ T16 — Form 8889, "Attachment Sequence No. 52", read off the printed form
        //   (`design/forms/extract/f8889--2024.txt:8` and `--2025.txt:8`, identical).
        "f8889" => Some("52"),
        "f8959" => Some("71"),
        "f8960" => Some("72"),
        "f8275" => Some("92"),
        // Rev. 12-2025 renumbered Form 8283 from 155 to 36; TY2017 (Rev. 12-2014) and TY2024
        // (Rev. 12-2023) print 155.
        "f8283" => Some(if year >= 2025 { "36" } else { "155" }),
        // ★ NEITHER OF THESE IS A PACKET MEMBER, and each is `None` by DECISION rather than by
        // falling through the catch-all below — the shape this repo distrusts, since `None == None`
        // makes the row gate green either way (`map_rows.rs::packet_sequences_agree_with_every_map_row`).
        // Both are measured: `grep -c "Sequence No"` on all four committed extracts is 0.
        //
        // Form 4868 is MAILED SEPARATELY, before the return exists — its own page 2 says *"Don't
        // attach a copy of Form 4868 to your return."* Form 1040-V rides in the return's envelope but
        // is ENCLOSED LOOSE, never stapled: *"Do not staple or attach this voucher to your payment or
        // return."* A sequence number is a stapling position, so a number on either would instruct
        // the filer to do the one thing the form forbids.
        "f4868" | "f1040v" => None,
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
                f8889,
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
    // ★★★ T16 — Form 8889. `Some` exactly when the §223 trigger declaration is affirmed, decided in
    //     CORE (`Form8889::must_file`), never here: the filler only obeys it.
    if let Some(l) = f8889 {
        push(
            "f8889",
            attachment_sequence("f8889", year),
            crate::fill_form_8889(l, header, year)?,
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

    // ★ P2 (step-1 phase review): the packet's order IS the stapling order the filer is handed
    //   ("the prefix IS the stapling order", admin.rs), and until 2026-09-05 it was the literal push
    //   order — so a renumbered form (8283: 155 → 36 on Rev. 12-2025) was pushed in its OLD place.
    //   Derive it: stable-sort by the printed sequence number, the 1040 (no number) first. For TY2024
    //   this is a no-op (the push order was already ascending), so no golden moves.
    // ★ The way to build a packet is the constructor that sorts (steps-2/3 review Q2). Outside this
    //   crate the struct is `#[non_exhaustive]`, so a literal cannot bypass it; inside this crate the
    //   guarantee is HELD BY TEST (`the_only_packet_constructor_staples_in_sequence_order` and the
    //   derived per-year test), not by the compiler — stated honestly per steps-4/5 review R8.
    Ok(FiledPacket::stapled(out, statements))
}

impl FiledPacket {
    /// The constructor: forms go in in ANY order and come out in stapling order (stable sort by the
    /// printed attachment sequence, the 1040 first). `fill_full_return` returns through it; the
    /// struct is `#[non_exhaustive]` so no downstream literal can skip the sort, and the two sequence
    /// tests hold the in-crate path.
    pub fn stapled(mut forms: Vec<NamedForm>, statements: Vec<NamedStatement>) -> Self {
        sort_by_attachment_sequence(&mut forms);
        Self { forms, statements }
    }
}

/// The IRS attachment-sequence ORDER key: the numeric part, then the letter suffix — so `"01" <
/// "1A" < "02"`, `"12" < "12A" < "17"`, `"55" < "55A"`. A plain string sort gets `"1A"` wrong (it
/// would land after `"12"`), which is why this is a function and not `.sort()`.
pub fn sequence_key(seq: &str) -> (u32, String) {
    let digits: String = seq.chars().take_while(|c| c.is_ascii_digit()).collect();
    let suffix: String = seq.chars().skip_while(|c| c.is_ascii_digit()).collect();
    (digits.parse().unwrap_or(0), suffix)
}

/// Stable-sort a packet into stapling order: forms with no sequence number (the 1040) first, then
/// ascending [`sequence_key`]. Stable, so two forms with one number (none today) keep push order.
pub fn sort_by_attachment_sequence(forms: &mut [NamedForm]) {
    forms.sort_by_key(|f| f.attachment_sequence.map(sequence_key));
}

#[cfg(test)]
mod sequence_order_tests {
    use super::*;

    fn nf(name: &str, seq: Option<&'static str>) -> NamedForm {
        NamedForm {
            name: name.to_string(),
            attachment_sequence: seq,
            bytes: Vec::new(),
        }
    }

    /// The comparator on the cases a string sort gets wrong.
    #[test]
    fn sequence_key_orders_letter_suffixes_after_their_number_and_1a_after_01() {
        let mut v = vec![
            "12A", "55A", "1A", "02", "12", "55", "01", "155", "36", "92", "17", "32",
        ];
        v.sort_by_key(|s| sequence_key(s));
        assert_eq!(
            v,
            ["01", "1A", "02", "12", "12A", "17", "32", "36", "55", "55A", "92", "155"]
        );
    }

    /// ★ Derived, for EVERY bundled year: take the sequence numbers off the rows (never a hand list),
    /// shuffle them into the worst order, sort, and the result is non-decreasing with the 1040 first.
    #[test]
    fn a_shuffled_packet_sorts_into_stapling_order_for_every_bundled_year() {
        for &year in crate::bundled::bundled_years() {
            let mut forms: Vec<NamedForm> = crate::bundled::BUNDLED
                .iter()
                .filter(|(_, y)| *y == year)
                .map(|(stem, y)| {
                    let row =
                        crate::map::MapRow::read(crate::bundled::map_text(*stem, *y).unwrap())
                            .unwrap();
                    nf(
                        stem.file_stem(),
                        attachment_sequence(stem.file_stem(), year),
                    )
                    .tap_check(&row)
                })
                .collect();
            if forms.is_empty() {
                // ★ a year with NOTHING to staple is lawful only while its record says `preparing`
                //   (TY2026 since spec 1099-DA T0); any other status with zero forms is a red
                let record = crate::year_record::YearRecord::for_year(year).expect("bundled year");
                assert_eq!(
                    record.status,
                    crate::year_record::YearStatus::Preparing,
                    "TY{year}: zero bundled forms is only lawful while preparing"
                );
                continue;
            }
            forms.reverse(); // worst case: descending
            sort_by_attachment_sequence(&mut forms);
            assert_eq!(
                forms[0].attachment_sequence, None,
                "TY{year}: the 1040 staples first"
            );
            let keys: Vec<_> = forms
                .iter()
                .filter_map(|f| f.attachment_sequence.map(sequence_key))
                .collect();
            assert!(
                keys.windows(2).all(|w| w[0] <= w[1]),
                "TY{year}: not in stapling order: {keys:?}"
            );
            assert!(
                keys.len() >= 4,
                "TY{year}: the walk found only {} sequenced forms",
                keys.len()
            );
        }
    }

    /// The constructor is the sort: a descending Vec comes out ascending, through the ONLY way a
    /// `FiledPacket` can be built.
    #[test]
    fn the_only_packet_constructor_staples_in_sequence_order() {
        let forms = vec![
            nf("f8283", attachment_sequence("f8283", 2025)),
            nf("f6251", attachment_sequence("f6251", 2025)),
            nf("f1040", None),
        ];
        let p = FiledPacket::stapled(forms, Vec::new());
        let names: Vec<&str> = p.forms.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["f1040", "f6251", "f8283"]);
    }

    /// The TY2025 8283 lands between 6251 (32) and 8995 (55) — the position half of FR-55.
    #[test]
    fn ty2025_form_8283_staples_after_6251_and_before_8995() {
        let mut forms = vec![
            nf("f8995", attachment_sequence("f8995", 2025)),
            nf("f8283", attachment_sequence("f8283", 2025)),
            nf("f6251", attachment_sequence("f6251", 2025)),
            nf("f1040", attachment_sequence("f1040", 2025)),
        ];
        sort_by_attachment_sequence(&mut forms);
        let names: Vec<&str> = forms.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["f1040", "f6251", "f8283", "f8995"]);
    }

    trait TapCheck {
        fn tap_check(self, row: &crate::map::MapRow) -> Self;
    }
    impl TapCheck for NamedForm {
        /// The packet's literal and the row's printed number are one fact (held by
        /// `tests/map_rows.rs::packet_sequences_agree_with_every_map_row`); re-asserted here so this
        /// derived test cannot pass on a literal the row disagrees with.
        fn tap_check(self, row: &crate::map::MapRow) -> Self {
            assert_eq!(
                self.attachment_sequence.map(String::from),
                row.attachment_sequence,
                "{}",
                self.name
            );
            self
        }
    }
}
