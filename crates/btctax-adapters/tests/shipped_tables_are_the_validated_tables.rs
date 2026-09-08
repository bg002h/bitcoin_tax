//! ★★★ THE VALIDATED ARTIFACT MUST BE THE SHIPPED ARTIFACT — FOR **EVERY YEAR THE BINARY SHIPS**.
//!
//! btctax has **two** tax tables per validated year:
//!
//! - `btctax_core::tax::testonly::ty2024_table()` — hand-built, and what nearly every core test, KAT
//!   and golden-matrix vector is computed against. **This is the thing that is validated.**
//! - `btctax_adapters::tax_tables::BundledTaxTables::load()` — read from the bundled data, and what
//!   the **binary actually uses** when a filer runs `btctax report`.
//!
//! **Nothing bound them.** So the entire two-oracle corpus, every KAT, and the byte-pinned golden
//! matrix could all have been validating a table the shipped binary never loads — and every one of
//! them would stay green while a filer's return used different numbers. That is not a bug in either
//! table; it is a missing *edge* between them, and the class is:
//!
//! > **an assurance surface that does not touch the artifact it is claimed to assure.**
//!
//! ★ The same shape as `§G-11`'s coverage checker being blind to the module about to be written, and
//! as the oracle harness's excuse lists keyed by vector name: the instrument is real, and points
//! slightly to one side of the thing.
//!
//! ★★ **Direction matters here.** A divergence would not fail loudly — the tests would keep passing
//! and only the *filer's* numbers would move. There is no oracle for this: both engines are handed
//! whatever table we give them, so their agreement says nothing (`CLAUDE.md`'s standing limit — *a
//! value the oracles take as INPUT is never validated by their agreement*). This equality is the only
//! witness there can be.
//!
//! ---
//!
//! ## ★★★ 2026-09-05 — THE SAME CLASS, ONE LEVEL OUT: this file pointed at ONE of the FOUR shipped years
//!
//! Every test here read `table_for(2024)` / `full_return_for(2024)` — a literal — while
//! `BundledTaxTables::load()` inserts **2017, 2024, 2025 and 2026**. So the file that exists to close
//! *"an assurance surface that does not touch the artifact it is claimed to assure"* had become an
//! example of it for three of four years, and a fifth year added tomorrow would have been covered by
//! nothing while every test here reported success. That is the port report's **FALLS BACK** class
//! (`design/TY2026_PORT_REPORT.md` §2 row 25): the product fails closed, the instruments fail open.
//!
//! **The fix is that no year is named in an assertion.** §1 below derives the shipped year set from
//! the shipping artifact itself — twice, by two mechanisms that must agree — and every test loops it.
//! Adding `by_year.insert(2027, ty2027())` to `tax_tables.rs` therefore *immediately* puts TY2027
//! under every comparison in this file, and — because §2's counterpart lookup fails closed — reds
//! [`every_shipped_year_has_a_validated_counterpart`] until somebody transcribes the other side.
//!
//! ★★ **What that gate reds on TODAY is the finding, not a nuisance.** TY2017, TY2025 and TY2026 ship
//! ordinary-rate brackets and §1(h) breakpoints to filers with **no validated counterpart anywhere in
//! the corpus** — `testonly` defines `ty2024_table` and `ty2024_params` and nothing else. The
//! remedy is *not* to copy `tax_tables.rs::ty2025()` into `testonly`: two artifacts agree only if they
//! were derived independently, and a copied table makes the equality vacuous. It is to transcribe the
//! other side from the revenue procedure each shipped table's own `source` field cites — the way
//! TY2024's MFS and HoH schedules were closed on 2026-08-02.
//!
//! ★ **The three unwitnessed years are not equally exposed, and the gate's message does not say so.**
//! Measured 2026-09-05 by matching every shipped ordinary-bracket `lower` literal against the
//! assertions in `tax_tables.rs`'s own `mod tests` (an UPPER bound — a literal may be matched by
//! another status's identical figure):
//!
//! | year | ordinary bracket thresholds appearing in ANY assertion | full 28-edge KAT? | `testonly` twin? |
//! |---|---|---|---|
//! | TY2017 | 28/28 | yes (`ty2017_table_matches_rev_proc_2016_55`) | no |
//! | TY2024 | 28/28 | yes (`ty2024_full_schedule_equality_all_28_edges_and_ltcg`) | **yes** |
//! | TY2025 | **8/28** | no | no |
//! | TY2026 | 24/28 | no (per-status spot KATs; the four misses are each schedule's `$0` floor) | no |
//!
//! So **TY2025 is the live exposure**: twenty bracket thresholds the binary would apply to a filer —
//! the whole interior of the MFJ and HoH schedules among them — are asserted by nothing at all. TY2017
//! and TY2026 are pinned in-crate but by a KAT living in the same file as the data it checks, which is
//! a weaker independence than a second artifact, and none of the three is bound to the corpus that
//! computes returns.

use btctax_adapters::tax_tables::{BundledFullReturnTables, BundledTaxTables};
use btctax_core::tax::tables::{FullReturnParams, FullReturnTables, TaxTable, TaxTables};
use btctax_core::tax::testonly::{
    ty2017_table, ty2024_params, ty2024_table, ty2025_table, ty2026_table,
};
use btctax_core::tax::types::FilingStatus;
use btctax_core::Usd;
use std::collections::{BTreeMap, BTreeSet};

const STATUSES: [FilingStatus; 5] = [
    FilingStatus::Single,
    FilingStatus::Mfj,
    FilingStatus::Mfs,
    FilingStatus::HoH,
    FilingStatus::Qss,
];

// ══ §1. THE SHIPPING SURFACE — derived from the artifact, never typed in ═══════════════════════════
//
// The year set must come from the thing that ships, so that a year added to `BundledTaxTables::load()`
// is covered here without anybody remembering to edit this file. `by_year` is private, so there are
// exactly two things a test can ask the artifact:
//
//   A. **What does the public lookup ANSWER to?** — probe `table_for(y)`. This is the surface a filer
//      actually reaches, but a probe can only cover a window, and a window is a numeric range: the
//      failure mode `CLAUDE.md` names by hand.
//   B. **What does the artifact ADVERTISE?** — its derived `Debug` rendering prints every `BTreeMap`
//      entry, so this enumeration is exhaustive by construction, with no window at all.
//
// Neither alone is trustworthy — A can miss a year outside the window, B can go silent if a `Debug`
// impl is ever hand-written — so **both are computed and they must agree**. A year outside the probe
// window is then *detected* (advertised, never answered) rather than missed, and a `Debug` that stops
// parsing yields an empty set, which is an error rather than a vacuously green loop.
//
// ★ A third reading falls out for free and is worth having on its own: every table also carries its own
// `year` FIELD, so the map key and the table's self-declaration are cross-checked. That is exactly the
// defect a year port makes — `by_year.insert(2026, ty2025())`, one token, and the binary hands a filer
// last year's brackets under this year's heading.

/// The probe window for mechanism A. Deliberately absurd in both directions: 1861 is the first US
/// income tax (Revenue Act of 1861) and 2200 is far past any table anyone will bundle. It is **not**
/// the source of truth — mechanism B is — so a year outside it reds instead of vanishing.
const PROBE_LO: i32 = 1861;
const PROBE_HI: i32 = 2200;

/// Mechanism B — every `BTreeMap` KEY the artifact prints in its own `Debug` rendering.
///
/// `BTreeMap`'s derived `Debug` is `{<key>: <Value> { … }, …}`, so the keys are the ASCII digit runs
/// immediately preceding each `": <Value> {"`. Returns the empty set if the marker never appears,
/// which every caller treats as an error — a parser that silently finds nothing is the blind
/// instrument this file exists to argue against.
fn keys_in_debug(rendered: &str, value_type: &str) -> BTreeSet<i32> {
    let marker = format!(": {value_type} {{");
    let bytes = rendered.as_bytes();
    let mut out = BTreeSet::new();
    let mut from = 0usize;
    while let Some(rel) = rendered[from..].find(&marker) {
        let at = from + rel;
        let mut start = at;
        while start > 0 && bytes[start - 1].is_ascii_digit() {
            start -= 1;
        }
        if start < at {
            if let Ok(year) = rendered[start..at].parse::<i32>() {
                out.insert(year);
            }
        }
        from = at + marker.len();
    }
    out
}

/// Every `year: NNNN` a bundled value declares about ITSELF, from the same `Debug` rendering.
///
/// `by_year: {` also matches the `"year: "` needle and contributes nothing, because what follows it is
/// `{`, not a digit — the digit requirement is what makes that harmless.
fn self_declared_years(rendered: &str) -> BTreeSet<i32> {
    let mut out = BTreeSet::new();
    for (at, needle) in rendered.match_indices("year: ") {
        let rest = &rendered[at + needle.len()..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(year) = digits.parse::<i32>() {
            out.insert(year);
        }
    }
    out
}

/// The years an artifact ships, or a named reason the two derivations disagree.
///
/// `answers_for` is the artifact's own public lookup (`table_for` / `full_return_for`), passed in so
/// this works for both bundles and for a planted `BTreeMap` in the kill tests below.
fn derive_shipped_years<T: std::fmt::Debug>(
    artifact: &T,
    value_type: &str,
    answers_for: impl Fn(i32) -> bool,
) -> Result<BTreeSet<i32>, String> {
    let rendered = format!("{artifact:?}");
    let advertised = keys_in_debug(&rendered, value_type);
    if advertised.is_empty() {
        return Err(format!(
            "no `<year>: {value_type} {{` entry could be read out of the artifact's own Debug \
             rendering, so the shipped year set derived to EMPTY and every loop over it would pass \
             vacuously. Either the bundle really is empty, or the Debug shape changed and this \
             derivation must be re-pointed at whatever now enumerates the years."
        ));
    }
    let answered: BTreeSet<i32> = (PROBE_LO..=PROBE_HI).filter(|y| answers_for(*y)).collect();
    if advertised != answered {
        return Err(format!(
            "the artifact advertises years {advertised:?} but its public lookup answers to \
             {answered:?} over {PROBE_LO}..={PROBE_HI}. A year advertised and not answered is \
             unreachable to a filer or outside this probe window; a year answered and not advertised \
             means this derivation can no longer enumerate the shipped set."
        ));
    }
    let declared = self_declared_years(&rendered);
    if declared != advertised {
        return Err(format!(
            "the artifact is keyed by years {advertised:?} but the values filed under those keys \
             declare themselves to be {declared:?}. A table filed under the wrong year is the \
             one-token year-port defect — `by_year.insert(2026, ty2025())` — and it hands a filer \
             last year's numbers under this year's heading."
        ));
    }
    Ok(advertised)
}

/// The shipped tax-table years, or a panic naming why they could not be derived.
fn shipped_table_years(bundle: &BundledTaxTables) -> BTreeSet<i32> {
    derive_shipped_years(bundle, "TaxTable", |y| bundle.table_for(y).is_some())
        .unwrap_or_else(|why| panic!("BundledTaxTables: {why}"))
}

/// The shipped full-return-params years, or a panic naming why they could not be derived.
fn shipped_param_years(bundle: &BundledFullReturnTables) -> BTreeSet<i32> {
    derive_shipped_years(bundle, "FullReturnParams", |y| {
        bundle.full_return_for(y).is_some()
    })
    .unwrap_or_else(|why| panic!("BundledFullReturnTables: {why}"))
}

// ══ §2. THE VALIDATED SIDE — a lookup that FAILS CLOSED ════════════════════════════════════════════
//
// The shipped side is enumerable (§1); the validated side is a Rust module, and a test cannot ask a
// module which functions it defines. So this half IS a written mapping — but it is written in the one
// direction that cannot go quiet: an unlisted year resolves to `None`, and `None` is a **named test
// failure**, never a skip. Measured 2026-09-05: `btctax_core::tax::testonly` defines exactly
// `ty2024_params` (`testonly.rs:53`) and `ty2024_table` (`testonly.rs:111`) — no other year.

/// The independently transcribed table the corpus computes against for `year`, if one exists.
fn validated_table_for(year: i32) -> Option<TaxTable> {
    match year {
        2017 => Some(ty2017_table()),
        2024 => Some(ty2024_table()),
        2025 => Some(ty2025_table()),
        2026 => Some(ty2026_table()),
        _ => None,
    }
}

/// The independently transcribed full-return params the corpus computes against for `year`.
fn validated_params_for(year: i32) -> Option<FullReturnParams> {
    match year {
        2024 => Some(ty2024_params()),
        _ => None,
    }
}

/// Which of `years` reach a filer with nothing to compare them to. Split out from the test so the kill
/// below can watch it discriminate in **both** directions on planted input.
fn years_without_a_validated_table(years: &BTreeSet<i32>) -> Vec<i32> {
    years
        .iter()
        .copied()
        .filter(|y| validated_table_for(*y).is_none())
        .collect()
}

/// [`years_without_a_validated_table`] for the full-return parameters.
fn years_without_validated_params(years: &BTreeSet<i32>) -> Vec<i32> {
    years
        .iter()
        .copied()
        .filter(|y| validated_params_for(*y).is_none())
        .collect()
}

// ══ §3. THE COMPARISONS ════════════════════════════════════════════════════════════════════════════

/// A per-status schedule that one side carries and the other does not.
///
/// ★★★ THE GATE IS THE MECHANISM, NOT A COUNT. A count cannot tell *"the binary ships brackets nobody
/// compared"* from *"neither side speaks, by law"* — and only the first is a defect. The **one** lawful
/// shape is a `Qss` schedule ABSENT FROM BOTH sides: §1(a)/§2(a) give a qualifying surviving spouse the
/// joint schedule, and `TaxTable::key` normalises `Qss → Mfj` at lookup, so a table may legitimately
/// omit it. Every other shape fails, however few there are.
///
/// ★ This replaces a `MAX_UNVALIDATED = 2` ratchet that was correct for a one-year test and does not
/// compose over a year SET (each validated year contributes its own two lawful `Qss` rows, so the
/// ratchet would have had to be multiplied by a year count — an outcome tally, exactly what
/// `CLAUDE.md` says never to enumerate). It is strictly stronger, not weaker: under the old count a
/// `Single/ordinary` row missing from both sides passed; here it fails.
fn is_lawful_absence(status: FilingStatus, in_shipped: bool, in_validated: bool) -> bool {
    status == FilingStatus::Qss && !in_shipped && !in_validated
}

fn describe_absence(
    year: i32,
    status: FilingStatus,
    schedule: &str,
    in_shipped: bool,
    in_validated: bool,
) -> String {
    let kind = match (in_shipped, in_validated) {
        (true, false) => "shipped only — UNWITNESSED",
        (false, true) => "validated only",
        (false, false) => "neither",
        (true, true) => "present on both (not an absence)",
    };
    format!("TY{year} {status:?}/{schedule} ({kind})")
}

/// Compare one year's shipped table with its validated counterpart, field by field.
///
/// ★ Compared structurally rather than with one `assert_eq!` on the whole value, so a failure names the
/// bracket that moved instead of dumping two tables and leaving a human to diff them. The lesson is
/// `CLAUDE.md`'s: an instrument that cannot say *what* changed gets ignored the first time it reds on
/// something innocuous.
///
/// ★★ Both sides are **destructured exhaustively**. Adding a field to `TaxTable` then fails to compile
/// here (E0027) until somebody decides whether the new field is part of the equality — the compiler
/// doing the review, rather than a future author silently shipping an uncompared field.
fn compare_tables(
    year: i32,
    shipped: &TaxTable,
    validated: &TaxTable,
    blocking: &mut Vec<String>,
    lawful: &mut Vec<String>,
) {
    let TaxTable {
        year: shipped_year,
        // ★ NOT compared, and that is the point: the two artifacts must cite DIFFERENT provenance
        //   ("TEST-TY2024" vs the revenue procedure). Equal `source` strings would mean one was copied
        //   from the other, which is the failure this whole file argues against.
        source: _,
        ordinary: shipped_ordinary,
        ltcg: shipped_ltcg,
        gift_annual_exclusion: shipped_gift_annual,
        ss_wage_base: shipped_ss_wage_base,
        gift_lifetime_exclusion: shipped_gift_lifetime,
    } = shipped;
    let TaxTable {
        year: validated_year,
        source: _,
        ordinary: validated_ordinary,
        ltcg: validated_ltcg,
        gift_annual_exclusion: validated_gift_annual,
        ss_wage_base: validated_ss_wage_base,
        gift_lifetime_exclusion: validated_gift_lifetime,
    } = validated;

    assert_eq!(
        shipped_year, validated_year,
        "TY{year}: the two tables disagree about their own year"
    );
    assert_eq!(
        *shipped_year, year,
        "the shipped table filed under key {year} declares itself to be TY{shipped_year}"
    );

    for st in STATUSES {
        // ── Ordinary-income brackets ────────────────────────────────────────────────────────────
        // ★ Symmetric: NEITHER side is required to carry every status. QSS, for one, takes the MFJ
        //   schedule by law (§1(a)/§2(a)), so a table may legitimately omit it and normalise at
        //   lookup. What must never happen is the two DISAGREEING where both speak.
        // ★★ The two schedules are checked INDEPENDENTLY. They used to share a `continue`: a status
        //    missing an ordinary schedule skipped the §1(h) comparison entirely, so its breakpoint gap
        //    was neither compared NOR counted — the instrument's own failure class, one level in. MFS
        //    and HoH were in exactly that state.
        let in_shipped = shipped_ordinary.contains_key(&st);
        let in_validated = validated_ordinary.contains_key(&st);
        if in_shipped && in_validated {
            let s = &shipped_ordinary[&st];
            let v = &validated_ordinary[&st];
            assert_eq!(
                s.brackets.len(),
                v.brackets.len(),
                "TY{year} {st:?}: bracket COUNT differs — shipped {} vs validated {}",
                s.brackets.len(),
                v.brackets.len()
            );
            for (i, (sb, vb)) in s.brackets.iter().zip(v.brackets.iter()).enumerate() {
                assert_eq!(
                    (sb.lower, sb.rate),
                    (vb.lower, vb.rate),
                    "TY{year} {st:?} ordinary bracket {i}: shipped ({}, {}) vs validated ({}, {}) — \
                     the binary would tax a filer differently from every test that says it is correct",
                    sb.lower,
                    sb.rate,
                    vb.lower,
                    vb.rate
                );
            }
        } else {
            let note = describe_absence(year, st, "ordinary", in_shipped, in_validated);
            if is_lawful_absence(st, in_shipped, in_validated) {
                lawful.push(note);
            } else {
                blocking.push(note);
            }
        }

        // ── §1(h) long-term capital-gain breakpoints ────────────────────────────────────────────
        let in_shipped = shipped_ltcg.contains_key(&st);
        let in_validated = validated_ltcg.contains_key(&st);
        if in_shipped && in_validated {
            let s = &shipped_ltcg[&st];
            let v = &validated_ltcg[&st];
            assert_eq!(
                (s.max_zero, s.max_fifteen),
                (v.max_zero, v.max_fifteen),
                "TY{year} {st:?} §1(h) breakpoints: shipped (0% ≤ {}, 15% ≤ {}) vs validated \
                 (0% ≤ {}, 15% ≤ {}) — this moves the rate on every crypto disposal",
                s.max_zero,
                s.max_fifteen,
                v.max_zero,
                v.max_fifteen
            );
        } else {
            let note = describe_absence(year, st, "ltcg", in_shipped, in_validated);
            if is_lawful_absence(st, in_shipped, in_validated) {
                lawful.push(note);
            } else {
                blocking.push(note);
            }
        }
    }

    // ── The scalars ─────────────────────────────────────────────────────────────────────────────
    assert_eq!(
        shipped_ss_wage_base, validated_ss_wage_base,
        "TY{year}: the §3121(a)(1) / 42 U.S.C. §430 Social Security wage base differs — this moves \
         Schedule SE on every self-employed return"
    );
    assert_eq!(
        shipped_gift_annual, validated_gift_annual,
        "TY{year}: the §2503(b) gift-tax annual exclusion per donee differs — this moves the Form 709 \
         over-annual-exclusion advisory"
    );
    assert_eq!(
        shipped_gift_lifetime, validated_gift_lifetime,
        "TY{year}: the §2010(c)(3) basic exclusion amount differs — this moves the §2505 \
         lifetime-exclusion consumption advisory"
    );
}

/// Rev. Proc. §.27's published *"Phase-in range amount"* — the TOP of the §199A range — per year.
///
/// Fails closed like §2: a year whose published figure has not been read off the revenue procedure
/// returns `None`, and the caller turns that into a named failure rather than skipping the invariant.
fn published_qbi_phase_in_top(year: i32, status: FilingStatus) -> Option<Usd> {
    match (year, status) {
        // Rev. Proc. 2023-34 §.27 (TY2024).
        (2024, FilingStatus::Mfj) => Some(rust_decimal_macros::dec!(483900)),
        (2024, FilingStatus::Single | FilingStatus::Mfs | FilingStatus::HoH) => {
            Some(rust_decimal_macros::dec!(241950))
        }
        // ★ QSS is not a joint return, and `FullReturnParams::qbi_ti_threshold` gives it the unmarried
        //   base; the Rev. Proc. prints "All Other Returns" for that row.
        (2024, FilingStatus::Qss) => Some(rust_decimal_macros::dec!(241950)),
        _ => None,
    }
}

/// The §199A phase-in findings for one year's params: `(mismatches, statuses with no published
/// figure)`.
///
/// ★ A pure function over a `FullReturnParams` VALUE rather than an assertion buried in the loop, so
/// the kill below can hand it a tampered clone and watch it discriminate. An invariant that can only
/// be exercised by editing the shipped table is an invariant nobody has ever seen work.
fn qbi_phase_in_findings(year: i32, p: &FullReturnParams) -> (Vec<String>, Vec<String>) {
    let mut mismatched = Vec::new();
    let mut unchecked = Vec::new();
    for st in STATUSES {
        match published_qbi_phase_in_top(year, st) {
            // Skipping is not passing: a year whose published figure nobody has read off the revenue
            // procedure is REPORTED by name, never counted as fine.
            None => unchecked.push(format!("TY{year} {st:?}")),
            Some(published) => {
                let computed = p.qbi_phase_in_top(st);
                if computed != published {
                    mismatched.push(format!(
                        "TY{year} {st:?}: threshold + width = {computed}, but the revenue \
                         procedure publishes a \"Phase-in range amount\" of {published}"
                    ));
                }
            }
        }
    }
    (mismatched, unchecked)
}

// ══ §4. THE TESTS ══════════════════════════════════════════════════════════════════════════════════

/// For **every shipped year with a validated counterpart**, the table the tests validate IS the table
/// the binary ships, field by field.
#[test]
fn every_shipped_tax_table_equals_the_one_the_corpus_validates() {
    let bundle = BundledTaxTables::load();
    let years = shipped_table_years(&bundle);
    assert!(
        !years.is_empty(),
        "the shipped year set derived to empty — see derive_shipped_years"
    );

    // ★★★ WHAT THIS MEASURED ON ITS FIRST RUN, AND WHAT CLOSED IT:
    //
    //     Mfs/ordinary  — shipped only, UNWITNESSED   ← CLOSED 2026-08-02
    //     HoH/ordinary  — shipped only, UNWITNESSED   ← CLOSED 2026-08-02
    //     Qss/ordinary  — neither, and legitimately so: §1(a)/§2(a) give a qualifying surviving
    //                     spouse the JOINT schedule, so both tables normalise at lookup
    //
    // The ordinary rate brackets the binary applied to **married-filing-separately and
    // head-of-household filers had never been compared to anything.** Not a defect in the shipped
    // numbers — an absence of witness, invisible until an edge was drawn between the two artifacts.
    //
    // ★★ CLOSING IT WAS NOT A COPY. Pasting the shipped brackets into `testonly::ty2024_table()`
    // would have made this pass by construction and proved nothing — two artifacts agree only if they
    // were derived independently. Both statuses are now TRANSCRIBED from Rev. Proc. 2023-34 §3.01
    // Tables 2 and 4 and §3.03, the source the shipped table's own `source` field cites. Transcribing
    // Single and MFJ in the same pass reproduced the committed values exactly, which is what says the
    // reading is right.
    let mut blocking: Vec<String> = Vec::new();
    let mut lawful: Vec<String> = Vec::new();
    let mut compared: Vec<i32> = Vec::new();

    for year in &years {
        let shipped = bundle
            .table_for(*year)
            .expect("derived from the bundle's own year set");
        let Some(validated) = validated_table_for(*year) else {
            // Named, counted, and gated by `every_shipped_year_has_a_validated_counterpart` — NOT
            // silently skipped here. Skipping is not passing.
            continue;
        };
        compare_tables(*year, shipped, &validated, &mut blocking, &mut lawful);
        compared.push(*year);
    }

    assert!(
        !compared.is_empty(),
        "no shipped year had a validated counterpart, so this test compared NOTHING while reporting \
         success. Shipped years: {years:?}"
    );
    assert!(
        blocking.is_empty(),
        "{} schedule(s) are carried by one artifact and not the other, and none of them is the one \
         lawful shape (a Qss schedule absent from BOTH sides, §1(a)/§2(a)): {:?}. A `shipped only` \
         entry is brackets the binary applies to a real return that no test has ever compared — \
         close it by transcribing the status from the year's revenue procedure, never by copying the \
         shipped table.",
        blocking.len(),
        blocking
    );
    // The lawful residue is printed, never asserted away — a growing set means the two artifacts are
    // drifting apart in scope even though every individual row is legal.
    if !lawful.is_empty() {
        println!("lawful absences (Qss, normalised at lookup): {lawful:?}");
    }
}

/// ★★★ The SECOND artifact, and the one that carries more of a return.
///
/// `FullReturnParams` holds the standard deduction, the §63(f) aged/blind additions, the SALT
/// limitation, the §199A thresholds and every AMT parameter. A divergence here moves line 12 — and
/// therefore taxable income — on **every** return, while the whole validated corpus stays green,
/// because the corpus is computed against `ty2024_params()` and the binary loads the bundled one.
///
/// ★ Compared field by field for the same reason as the table: a failure must name the parameter. And
/// destructured exhaustively for the same reason: a new `FullReturnParams` field must not be able to
/// join a filer's return without passing through this equality.
#[test]
fn every_shipped_full_return_params_equal_the_ones_the_corpus_validates() {
    let bundle = BundledFullReturnTables::load();
    let years = shipped_param_years(&bundle);
    assert!(
        !years.is_empty(),
        "the shipped params year set derived to empty — see derive_shipped_years"
    );

    let mut compared: Vec<i32> = Vec::new();
    let mut qbi_mismatches: Vec<String> = Vec::new();
    let mut unchecked_qbi_top: Vec<String> = Vec::new();

    for year in &years {
        let s = bundle
            .full_return_for(*year)
            .expect("derived from the bundle's own year set");
        let Some(v) = validated_params_for(*year) else {
            continue; // gated by `every_shipped_year_has_a_validated_counterpart`
        };
        let year = *year;

        let FullReturnParams {
            year: shipped_year,
            std_deduction: s_std_deduction,
            std_aged_blind_married: s_aged_blind_married,
            std_aged_blind_unmarried: s_aged_blind_unmarried,
            dependent_std_floor: s_dependent_floor,
            dependent_std_earned_addon: s_dependent_addon,
            salt: s_salt,
            kiddie_unearned_threshold: s_kiddie,
            qualifying_relative_gross_income_limit: s_qr_gross_income,
            child_tax_credit_per_child: s_ctc_per_child,
            credit_for_other_dependents_per_person: s_odc_per_person,
            elective_deferral_limit: s_deferral,
            ftc_ceiling: s_ftc,
            qbi_ti_threshold_unmarried: s_qbi_thr_unmarried,
            qbi_ti_threshold_married: s_qbi_thr_married,
            qbi_phase_in_range_unmarried: s_qbi_range_unmarried,
            qbi_phase_in_range_married: s_qbi_range_married,
            student_loan_phaseout_unmarried: s_sl_unmarried,
            student_loan_phaseout_married: s_sl_married,
            amt: s_amt,
            hsa: s_hsa,
        } = s;
        let FullReturnParams {
            year: validated_year,
            std_deduction: v_std_deduction,
            std_aged_blind_married: v_aged_blind_married,
            std_aged_blind_unmarried: v_aged_blind_unmarried,
            dependent_std_floor: v_dependent_floor,
            dependent_std_earned_addon: v_dependent_addon,
            salt: v_salt,
            kiddie_unearned_threshold: v_kiddie,
            qualifying_relative_gross_income_limit: v_qr_gross_income,
            child_tax_credit_per_child: v_ctc_per_child,
            credit_for_other_dependents_per_person: v_odc_per_person,
            elective_deferral_limit: v_deferral,
            ftc_ceiling: v_ftc,
            qbi_ti_threshold_unmarried: v_qbi_thr_unmarried,
            qbi_ti_threshold_married: v_qbi_thr_married,
            qbi_phase_in_range_unmarried: v_qbi_range_unmarried,
            qbi_phase_in_range_married: v_qbi_range_married,
            student_loan_phaseout_unmarried: v_sl_unmarried,
            student_loan_phaseout_married: v_sl_married,
            amt: v_amt,
            hsa: v_hsa,
        } = &v;

        assert_eq!(
            shipped_year, validated_year,
            "TY{year}: the two params disagree about their year"
        );
        assert_eq!(
            *shipped_year, year,
            "the shipped params filed under key {year} declare themselves to be TY{shipped_year}"
        );
        for st in STATUSES {
            assert_eq!(
                s_std_deduction.get(&st),
                v_std_deduction.get(&st),
                "TY{year} {st:?}: STANDARD DEDUCTION differs — this moves 1040 line 12, and therefore \
                 taxable income, on every return of that status"
            );
        }
        assert_eq!(
            (s_aged_blind_married, s_aged_blind_unmarried),
            (v_aged_blind_married, v_aged_blind_unmarried),
            "TY{year}: the §63(f) aged/blind additions differ"
        );
        assert_eq!(
            (s_dependent_floor, s_dependent_addon),
            (v_dependent_floor, v_dependent_addon),
            "TY{year}: the §63(c)(5) dependent standard-deduction floor/add-on differ"
        );
        assert_eq!(
            s_salt, v_salt,
            "TY{year}: the §164(b) SALT limitation differs — and it is an ENUM, so a differing \
             variant means the two artifacts disagree about which Schedule A question the year asks"
        );
        assert_eq!(
            (s_ctc_per_child, s_odc_per_person),
            (v_ctc_per_child, v_odc_per_person),
            "TY{year}: the §24(h)(2)/(h)(4) per-person credit ceilings differ — nothing computes \
             from them, but the R12 panel SIZES the line-19 forgo with them, so a divergence puts \
             two different dollar figures in front of the filer for one forgone credit"
        );
        assert_eq!(
            s_kiddie, v_kiddie,
            "TY{year}: the §1(g) kiddie-tax unearned threshold differs"
        );
        assert_eq!(
            (s_qbi_thr_unmarried, s_qbi_thr_married),
            (v_qbi_thr_unmarried, v_qbi_thr_married),
            "TY{year}: the §199A(e)(2) thresholds differ — this decides whether a return REFUSES for \
             8995-A"
        );
        assert_eq!(
            (s_qbi_range_unmarried, s_qbi_range_married),
            (v_qbi_range_unmarried, v_qbi_range_married),
            "TY{year}: the Form 8995-A line 23 phase-in RANGE WIDTHS differ — this scales the \
             phase-in percentage and therefore the whole deduction"
        );

        // ★★★ THE INVARIANT THAT MAKES THE §199A TRAP UNCATCHABLE-BY-EYE INTO A FAILING TEST.
        //
        // Rev. Proc. 2023-34 §.27 publishes TWO columns — "Threshold amount" and "Phase-in range
        // amount":
        //
        //     Married Individuals Filing Joint Returns    $383,900     $483,900
        //     Married Individuals Filing Separate Returns $191,950     $241,950
        //     All Other Returns                           $191,950     $241,950
        //
        // ★★ The second column is the TOP OF THE RANGE, not its width. Form 8995-A line 23 asks for
        //    the WIDTH — "Enter $50,000 ($100,000 if married filing jointly)". Pasting the published
        //    amount into the width field puts $483,900 where $100,000 belongs, off by roughly 5×, and
        //    it CROSS-FOOTS PERFECTLY because line 24 merely divides by it: the phase-in percentage
        //    collapses toward zero, the deduction is barely reduced, and the tax is UNDERSTATED.
        //    Nothing downstream looks wrong.
        //
        // ★ And the width is STATUTORY (§199A(b)(3)(B)(ii)(II)) while the threshold beside it is
        //   INDEXED (§199A(e)(2)(B)) — so a future table update that "indexes" the width alongside its
        //   neighbour is also wrong. Both mistakes break this equality, because it ties the two
        //   together through the Rev. Proc.'s OWN published figure rather than through anyone's
        //   arithmetic.
        let (mismatched, unchecked) = qbi_phase_in_findings(year, s);
        qbi_mismatches.extend(mismatched);
        unchecked_qbi_top.extend(unchecked);

        assert_eq!(
            (s_sl_unmarried, s_sl_married),
            (v_sl_unmarried, v_sl_married),
            "TY{year}: the §221 student-loan phase-out bands differ"
        );
        assert_eq!(
            s_deferral, v_deferral,
            "TY{year}: the §402(g) elective-deferral limit differs"
        );
        assert_eq!(s_ftc, v_ftc, "TY{year}: the FTC ceiling differs");
        assert_eq!(
            s_amt, v_amt,
            "TY{year}: the AMT parameters differ — exemption, phase-out and the 28% breakpoint all \
             move Form 6251"
        );
        // ★★★ T16 — the §223(b) HSA limitation. It moves Form 8889 line 3, therefore line 13's
        //     deduction on Schedule 1 line 13, AND the excess-contribution refusal: a limit that is
        //     too high both over-deducts and stops refusing on contributions that need Form 5329.
        //     ★ It is the ONE figure here that does not come from the autumn inflation Rev. Proc. —
        //     §223(g) requires publication by June 1 of the PRECEDING year, so TY2024's authority is
        //     Rev. Proc. 2023-23 §2.01(1), transcribed independently on each side.
        // ★★★ T7 / R6 — §152(d)(1)(B). It is QUOTED IN A PROMPT the filer answers, so a wrong figure
        //     is not a wrong number on a worksheet: it is a question that asks about the wrong
        //     threshold, and the answer given to it is the filer's own testimony.
        assert_eq!(
            s_qr_gross_income, v_qr_gross_income,
            "TY{year}: the §152(d)(1)(B) qualifying-relative gross income limit differs — it is \
             quoted verbatim in the Step 4 gate's prompt, so the two sides disagree about what the \
             filer was asked"
        );
        assert_eq!(
            s_hsa, v_hsa,
            "TY{year}: the §223(b) HSA contribution limitation differs — it sets Form 8889 line 3, \
             the Schedule 1 line 13 deduction, and the threshold above which excess contributions \
             refuse for Form 5329"
        );
        compared.push(year);
    }

    assert!(
        !compared.is_empty(),
        "no shipped params year had a validated counterpart, so this test compared NOTHING while \
         reporting success. Shipped params years: {years:?}"
    );
    assert!(
        qbi_mismatches.is_empty(),
        "the §199A threshold + width does not reconcile with the revenue procedure's own published \
         \"Phase-in range amount\" for {qbi_mismatches:?}. Either the WIDTH was set from that \
         published column (it is the range's TOP, not its width — off by roughly 5×, and it \
         cross-foots perfectly) or the width was inflation-indexed (it is statutory and must not be)."
    );
    assert!(
        unchecked_qbi_top.is_empty(),
        "the §199A published-\"Phase-in range amount\" invariant has no primary-source figure for \
         {unchecked_qbi_top:?}, so for those it checked nothing. Read the year's Rev. Proc. §.27 \
         two-column table and add it to `published_qbi_phase_in_top` — do NOT compute the top from \
         the threshold and width being tested, which would make the equality tautological."
    );
}

/// ★★★ THE YEAR-COVERAGE GATE — the one this file was missing, and the one that reds today.
///
/// A shipped year with no validated counterpart is not an inconvenience; it is the whole class this
/// file exists to close, at year granularity. The binary hands those brackets to a filer, and the
/// corpus — 260 references to `ty2024_table`/`ty2024_params` across `crates/`, zero to any other year
/// (measured 2026-09-05) — computes nothing against them.
///
/// ★ **Do not close this by copying `tax_tables.rs::tyNNNN()` into `testonly.rs`.** Two artifacts agree
/// only if they were derived independently; a copy makes the equality vacuous and would delete the
/// only witness there can be. Transcribe the other side from the revenue procedure that the shipped
/// table's own `source` field cites.
#[test]
fn every_shipped_year_has_a_validated_counterpart() {
    let tables = BundledTaxTables::load();
    let table_years = shipped_table_years(&tables);
    let params = BundledFullReturnTables::load();
    let param_years = shipped_param_years(&params);

    let missing_tables: Vec<String> = years_without_a_validated_table(&table_years)
        .into_iter()
        .map(|y| {
            let t = tables.table_for(y).expect("from the bundle's own year set");
            let brackets: usize = t.ordinary.values().map(|s| s.brackets.len()).sum();
            format!(
                "TY{y}: {} ordinary schedule(s)/{brackets} bracket(s) + {} §1(h) breakpoint pair(s), \
                 ss_wage_base {} — source {:?}",
                t.ordinary.len(),
                t.ltcg.len(),
                t.ss_wage_base,
                t.source
            )
        })
        .collect();
    let missing_params: Vec<i32> = years_without_validated_params(&param_years);

    // ★★★ **A RATCHET, not a pass — and the difference is the whole point.**
    //
    //     The honest state, measured the day this check was written: the binary ships tax tables for
    //     {2017, 2024, 2025, 2026} and only **TY2024** has a validated counterpart. TY2017, TY2025
    //     and TY2026 are numbers a filer's return is computed FROM that no test has ever compared to
    //     anything. That gap PREDATES this test; the test is what made it visible.
    //
    //     ★ Asserting `is_empty()` today would only mean deleting the test or faking three
    //       corpora, so it records the CURRENT gap by year and reds when the gap GROWS — a fourth
    //       year shipping unvalidated, or a validated corpus disappearing. Closing a year is a
    //       transcription from that year's revenue procedure into `btctax_core::tax::testonly`;
    //       when one lands, REMOVE it from this list. The list may only shrink.
    //
    //     ★★ It must NEVER be closed by copying `tax_tables.rs` into the corpus. That is an echo,
    //        not a witness, and it would make this test assert that a number equals itself.
    const KNOWN_UNVALIDATED_TABLE_YEARS: &[i32] = &[];
    let unexpected: Vec<&String> = missing_tables
        .iter()
        .filter(|d| {
            !KNOWN_UNVALIDATED_TABLE_YEARS
                .iter()
                .any(|y| d.starts_with(&format!("TY{y}:")))
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "a shipped year has NO validated table and is not in the recorded gap: {unexpected:#?}\n\
         Either transcribe its counterpart from that year's revenue procedure, or add it here WITH \
         a reason. Never widen this list to make a red go away."
    );
    let closed: Vec<i32> = KNOWN_UNVALIDATED_TABLE_YEARS
        .iter()
        .copied()
        .filter(|y| {
            !missing_tables
                .iter()
                .any(|d| d.starts_with(&format!("TY{y}:")))
        })
        .collect();
    assert!(
        closed.is_empty(),
        "TY{closed:?} now HAS a validated table — remove it from KNOWN_UNVALIDATED_TABLE_YEARS so \
         the ratchet keeps its teeth. A stale excuse is how a closed gap reopens unnoticed."
    );

    assert!(
        missing_params.is_empty(),
        "the binary ships tax tables for {table_years:?} and full-return params for {param_years:?}; \
         the validated corpus has NO counterpart for these:\n  tables: {missing_tables:#?}\
         \n  params: {missing_params:?}\nEach entry is numbers a filer's return is computed from that \
         no test has ever compared to anything. Close it by TRANSCRIBING the other side from that \
         year's revenue procedure into `btctax_core::tax::testonly` — never by copying \
         `tax_tables.rs`."
    );
}

// ══ §5. THE KILLS — B1: no checker exists until it has been watched go RED on a planted defect ══════

/// ★★ **The B1 kill for the field-by-field comparison, and the original reason this file exists.**
///
/// A checker never watched go red does not exist. This plants the exact divergence
/// [`every_shipped_tax_table_equals_the_one_the_corpus_validates`] guards — one bracket threshold
/// moved by a dollar — and asserts the comparison rejects it.
///
/// It operates on CLONES rather than by mutating either real table, so it pins the RULE (structural
/// inequality is detected, field by field) rather than today's data. A future author who edits one
/// table and not the other reds the test above; this proves the test above can red at all.
#[test]
fn a_single_moved_bracket_is_detected() {
    let shipped_all = BundledTaxTables::load();
    let shipped = shipped_all.table_for(2024).unwrap();
    let mut tampered = ty2024_table();

    let s = tampered
        .ordinary
        .get_mut(&FilingStatus::Single)
        .expect("Single has an ordinary schedule");
    let original = s.brackets[2].lower;
    s.brackets[2].lower = original + rust_decimal_macros::dec!(1);

    let real = shipped.ordinary.get(&FilingStatus::Single).unwrap();
    let planted = tampered.ordinary.get(&FilingStatus::Single).unwrap();

    assert_ne!(
        (real.brackets[2].lower, real.brackets[2].rate),
        (planted.brackets[2].lower, planted.brackets[2].rate),
        "★ a one-dollar bracket move MUST be visible to the field-by-field comparison — if this \
         passes, the test above is decorative and the shipped table is unwitnessed"
    );
    // …and every OTHER bracket still matches, so the comparison is not just always-unequal.
    for (i, (a, b)) in real
        .brackets
        .iter()
        .zip(planted.brackets.iter())
        .enumerate()
    {
        if i == 2 {
            continue;
        }
        assert_eq!(
            (a.lower, a.rate),
            (b.lower, b.rate),
            "bracket {i} should be untouched — a comparison that reds on everything reds on nothing"
        );
    }
}

/// ★★ B1 kill for §1's PARSER — it must both find years and fail to find them.
///
/// A parser watched only on input it succeeds at is the blind-checker shape: `cite_check` read only
/// `*"…"*` spans and reported success on a table it never looked at.
#[test]
fn the_debug_year_parser_discriminates() {
    let rendered = "BundledTaxTables { by_year: {2017: TaxTable { year: 2017, source: \"x\" }, \
                    2024: TaxTable { year: 2024, source: \"y\" }} }";
    assert_eq!(
        keys_in_debug(rendered, "TaxTable"),
        BTreeSet::from([2017, 2024]),
        "the key parser must read every map key"
    );
    assert_eq!(
        self_declared_years(rendered),
        BTreeSet::from([2017, 2024]),
        "the self-declaration parser must read every value's own `year` field, and must not be \
         confused by the enclosing `by_year:` field name"
    );
    assert!(
        keys_in_debug(rendered, "FullReturnParams").is_empty(),
        "a marker that does not occur must yield NOTHING — which callers treat as an error, not as a \
         clean bill of health"
    );
}

/// ★★ B1 kill for §1's DERIVATION — a year the public lookup cannot reach must be an error.
///
/// The planted defect is the probe window's own blind spot: a year outside `PROBE_LO..=PROBE_HI`. It
/// is advertised by the artifact and never answered by the probe, so the two derivations disagree and
/// the derivation reds instead of quietly returning a short year set — which is what would let a
/// shipped year escape every loop in this file.
#[test]
fn the_year_derivation_reds_on_a_year_outside_the_probe_window() {
    let mut clean: BTreeMap<i32, TaxTable> = BTreeMap::new();
    clean.insert(2024, ty2024_table());
    assert_eq!(
        derive_shipped_years(&clean, "TaxTable", |y| clean.table_for(y).is_some())
            .expect("a well-formed bundle must derive cleanly"),
        BTreeSet::from([2024]),
        "the derivation must SUCCEED on well-formed input, or its failures mean nothing"
    );

    let mut planted = clean.clone();
    let mut far_future = ty2024_table();
    far_future.year = PROBE_HI + 800;
    planted.insert(PROBE_HI + 800, far_future);
    let why = derive_shipped_years(&planted, "TaxTable", |y| planted.table_for(y).is_some())
        .expect_err("a year outside the probe window must be an ERROR, not an omission");
    assert!(
        why.contains(&(PROBE_HI + 800).to_string()),
        "the error must NAME the year the probe could not reach: {why}"
    );
}

/// ★★ B1 kill for the cross-check between a map KEY and the value's own `year` field — the one-token
/// year-port defect (`by_year.insert(2026, ty2025())`).
#[test]
fn the_year_derivation_reds_when_a_table_is_filed_under_the_wrong_year() {
    let mut planted: BTreeMap<i32, TaxTable> = BTreeMap::new();
    planted.insert(2026, ty2024_table()); // the key says 2026; the table says 2024
    let why = derive_shipped_years(&planted, "TaxTable", |y| planted.table_for(y).is_some())
        .expect_err("a table filed under the wrong year must be an ERROR");
    assert!(
        why.contains("2026") && why.contains("2024"),
        "the error must name BOTH the key and the year the value declares: {why}"
    );
}

/// ★★ B1 kill for the year-coverage gate itself — it must name a year with no witness, and must NOT
/// name one that has a witness. A gate that reds on everything reds on nothing.
#[test]
fn the_counterpart_gate_discriminates() {
    assert!(
        years_without_a_validated_table(&BTreeSet::from([2024])).is_empty(),
        "TY2024 has `testonly::ty2024_table()` — the gate must not report it"
    );
    assert_eq!(
        years_without_a_validated_table(&BTreeSet::from([2024, 2999])),
        vec![2999],
        "a shipped year with no validated counterpart must be NAMED — this is the planted 'a year \
         was added to the bundle and nobody transcribed the other side' defect"
    );
    assert!(years_without_validated_params(&BTreeSet::from([2024])).is_empty());
    assert_eq!(
        years_without_validated_params(&BTreeSet::from([2024, 2999])),
        vec![2999]
    );
}

/// ★★ B1 kill for the §199A published-"Phase-in range amount" invariant.
///
/// The planted defect is the exact trap the invariant exists to catch: the revenue procedure's
/// published amount ($483,900 MFJ) pasted into the WIDTH field where the form asks for $100,000. It
/// cross-foots perfectly downstream — line 24 merely divides by it — so nothing else in the corpus
/// can see it. Operates on a CLONE, so it pins the RULE rather than today's data.
#[test]
fn the_qbi_phase_in_top_invariant_catches_the_published_amount_paste() {
    let clean = ty2024_params();
    let (mismatched, unchecked) = qbi_phase_in_findings(2024, &clean);
    assert!(
        mismatched.is_empty() && unchecked.is_empty(),
        "the invariant must be SILENT on correct params, or its noise means nothing: \
         {mismatched:?} / {unchecked:?}"
    );

    let mut tampered = clean.clone();
    tampered.qbi_phase_in_range_married = rust_decimal_macros::dec!(483900);
    let (mismatched, unchecked) = qbi_phase_in_findings(2024, &tampered);
    assert!(unchecked.is_empty());
    assert_eq!(
        mismatched.len(),
        1,
        "exactly the MFJ row uses the married width — a check that reds on every status reds on \
         none: {mismatched:?}"
    );
    assert!(
        mismatched[0].contains("Mfj"),
        "the finding must NAME the status: {mismatched:?}"
    );

    // ★ And a year whose published figure has never been read off the revenue procedure is REPORTED,
    //   not silently skipped — the difference between "this encodes no decision" and "we forgot".
    let (_, unchecked) = qbi_phase_in_findings(2025, &clean);
    assert_eq!(
        unchecked.len(),
        STATUSES.len(),
        "every status of a year with no published figure must be named: {unchecked:?}"
    );
}

// ══ §5. THE §24(h)(2) PER-CHILD CEILING — HELD AGAINST EVERY YEAR THE BUNDLE REGISTERS ═══════════

/// ★★★ **T8 seam review I-2 — the pin that could not red, moved to the crate that can see the
/// bundle.**
///
/// `btctax_core::tax::advisories::CTC_PER_CHILD_SS24H2` is the $2,000 upper bound
/// `ctc_provably_zero` multiplies Schedule 8812 line 8 by in order to conclude *"the credit is
/// provably zero"* — and, when it concludes that, **1040 line 19 prints a sworn `0`** (26 USC §6065).
/// OBBBA Pub. L. 119-21 §70104(a)(2) raises §24(h)(2) to **$2,200** for *"taxable years beginning
/// after December 31, 2024"* (§70104(f)), i.e. **TY2025 onward**. At the stale $2,000 ceiling the
/// proof concludes the credit is gone for households that still have it: measured, MFJ / 2 children /
/// AGI $482,000 flips from *"still has credit"* to *"provably zero"* on the constant alone.
///
/// ★★★ **Why it lives HERE and not beside the constant.** The pin that claimed this ran in
/// `btctax-core` and read `testonly::ty2024_params()` — a core-local fixture literal hardcoded to
/// `year: 2024`. `BundledFullReturnTables` lives in *this* crate and has zero occurrences in
/// `btctax-core`, so that test was structurally incapable of seeing a package land: bundling TY2026
/// (whose figure is already `dec!(2200)`) left it **green**. Its own doc called a hand-written year
/// list *"the `1..=38` trap in its usual costume"*; it was that trap.
///
/// ★ The year set is derived by [`shipped_param_years`] from the bundle's own `Debug` rendering,
///   cross-checked against its public lookup — never a list typed here. So
///   `by_year.insert(2025, ty2025_full_return())` in `tax_tables.rs` reds this the moment it lands.
///
/// ★★ **The fix when that red arrives is to thread the year's `FullReturnParams` into
///    `ctc_odc_line19`, NOT to raise the constant.** $2,000 is right for TY2024, and TY2024 is the
///    year btctax can actually file; raising it would move the wrongness onto the filed year.
#[test]
fn every_bundled_years_ctc_per_child_is_the_named_ceiling() {
    let bundle = BundledFullReturnTables::load();
    let years = shipped_param_years(&bundle);
    assert!(
        !years.is_empty(),
        "the shipped params year set derived to EMPTY, so this loop would pass vacuously — see \
         derive_shipped_years"
    );
    let named = btctax_core::tax::testonly::ctc_per_child_ss24h2();
    let mut wrong: Vec<String> = Vec::new();
    for year in &years {
        let p = bundle
            .full_return_for(*year)
            .expect("derived from the bundle's own year set");
        if p.child_tax_credit_per_child != named {
            wrong.push(format!(
                "TY{year}: the bundled package says §24(h)(2) is {} per child, \
                 advisories::CTC_PER_CHILD_SS24H2 says {named}",
                p.child_tax_credit_per_child
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{wrong:?}\n\nThe Schedule 8812 line-8 ceiling `advisories::ctc_provably_zero` multiplies by \
         is not this year's figure. At a ceiling that is too LOW the proof concludes the credit is \
         gone for a household that still has one, and 1040 line 19 prints a sworn `0` — \
         taxpayer-adverse and invisible on the page. Fix it by threading the year's FullReturnParams \
         into `ctc_odc_line19`, NOT by editing the constant: $2,000 is correct for TY2024."
    );
}
