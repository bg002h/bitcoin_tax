//! ★★★ THE VALIDATED ARTIFACT MUST BE THE SHIPPED ARTIFACT.
//!
//! btctax has **two** TY2024 tax tables:
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

use btctax_adapters::tax_tables::{BundledFullReturnTables, BundledTaxTables};
use btctax_core::tax::tables::{FullReturnTables, TaxTables};
use btctax_core::tax::testonly::{ty2024_params, ty2024_table};
use btctax_core::tax::types::FilingStatus;

const STATUSES: [FilingStatus; 5] = [
    FilingStatus::Single,
    FilingStatus::Mfj,
    FilingStatus::Mfs,
    FilingStatus::HoH,
    FilingStatus::Qss,
];

/// The table the tests validate IS the table the binary ships, field by field.
///
/// ★ Compared structurally rather than with one `assert_eq!` on the whole value, so a failure names
/// the bracket that moved instead of dumping two tables and leaving a human to diff them. The lesson
/// is `CLAUDE.md`'s: an instrument that cannot say *what* changed gets ignored the first time it reds
/// on something innocuous.
#[test]
fn the_shipped_ty2024_table_equals_the_one_every_test_validates() {
    let shipped_all = BundledTaxTables::load();
    let shipped = shipped_all
        .table_for(2024)
        .expect("the binary bundles a TY2024 table");
    let validated = ty2024_table();

    assert_eq!(
        shipped.year, validated.year,
        "the two TY2024 tables disagree about their own year"
    );

    // ★★★ FOUND ON THIS TEST'S FIRST RUN: the validated table is a strict SUBSET of the shipped one.
    // `testonly::ty2024_table()` carries schedules for only some filing statuses, while the binary
    // ships all five — so those statuses' brackets reach a filer having been compared to nothing.
    // Reported as a named, counted gap rather than papered over: comparing only the intersection and
    // saying nothing would be the "instrument points slightly to one side" failure again.
    let mut unvalidated: Vec<String> = Vec::new();

    for st in STATUSES {
        // ── Ordinary-income brackets ────────────────────────────────────────────────────────────
        // ★ Symmetric: NEITHER side is required to carry every status. QSS, for one, takes the MFJ
        //   schedule by law (§1(a)/§2(a)), so a table may legitimately omit it and normalise at
        //   lookup. What must never happen is the two DISAGREEING where both speak.
        // ★★ The two schedules are checked INDEPENDENTLY. They used to share a `continue`: a status
        //    missing an ordinary schedule skipped the §1(h) comparison entirely, so its breakpoint gap
        //    was neither compared NOR counted — the instrument's own failure class, one level in. MFS
        //    and HoH were in exactly that state.
        let ordinary =
            if !shipped.ordinary.contains_key(&st) || !validated.ordinary.contains_key(&st) {
                unvalidated.push(format!(
                    "{st:?}/ordinary({})",
                    match (
                        shipped.ordinary.contains_key(&st),
                        validated.ordinary.contains_key(&st)
                    ) {
                        (true, false) => "shipped only — UNWITNESSED",
                        (false, true) => "validated only",
                        _ => "neither",
                    }
                ));
                None
            } else {
                Some((
                    shipped.ordinary.get(&st).unwrap(),
                    validated.ordinary.get(&st).unwrap(),
                ))
            };
        if let Some((s, v)) = ordinary {
            assert_eq!(
                s.brackets.len(),
                v.brackets.len(),
                "{st:?}: bracket COUNT differs — shipped {} vs validated {}",
                s.brackets.len(),
                v.brackets.len()
            );
            for (i, (sb, vb)) in s.brackets.iter().zip(v.brackets.iter()).enumerate() {
                assert_eq!(
                (sb.lower, sb.rate),
                (vb.lower, vb.rate),
                "{st:?} ordinary bracket {i}: shipped ({}, {}) vs validated ({}, {}) — the binary \
                 would tax a filer differently from every test that says it is correct",
                sb.lower,
                sb.rate,
                vb.lower,
                vb.rate
            );
            }
        }

        // ── §1(h) long-term capital-gain breakpoints ────────────────────────────────────────────
        let (Some(s), Some(v)) = (shipped.ltcg.get(&st), validated.ltcg.get(&st)) else {
            unvalidated.push(format!(
                "{st:?}/ltcg({})",
                match (
                    shipped.ltcg.contains_key(&st),
                    validated.ltcg.contains_key(&st)
                ) {
                    (true, false) => "shipped only — UNWITNESSED",
                    (false, true) => "validated only",
                    _ => "neither",
                }
            ));
            continue;
        };
        assert_eq!(
            (s.max_zero, s.max_fifteen),
            (v.max_zero, v.max_fifteen),
            "{st:?} §1(h) breakpoints: shipped (0% ≤ {}, 15% ≤ {}) vs validated (0% ≤ {}, 15% ≤ {}) \
             — this moves the rate on every crypto disposal",
            s.max_zero,
            s.max_fifteen,
            v.max_zero,
            v.max_fifteen
        );
    }

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
    //
    // ★★★ THE GATE IS THE MECHANISM, NOT A COUNT. A count cannot tell "the binary ships brackets
    // nobody compared" from "neither side speaks, by law" — and only the first is a defect. So an
    // `UNWITNESSED` entry fails outright, however few there are, and however the count is doing.
    // `CLAUDE.md`: state the mechanism, let it decide, never enumerate the outcomes you happened to
    // see. ★ This also stops the ratchet being satisfiable by DELETING a validated schedule, which
    // would move an entry from "shipped only" to "neither" and lower the number.
    let unwitnessed: Vec<&String> = unvalidated
        .iter()
        .filter(|u| u.contains("UNWITNESSED"))
        .collect();
    assert!(
        unwitnessed.is_empty(),
        "{} schedule(s) are SHIPPED to filers with no validated counterpart: {:?}. Each is brackets \
         the binary applies to a real return that no test has ever compared. Close it by \
         transcribing the status from Rev. Proc. 2023-34 — never by copying the shipped table.",
        unwitnessed.len(),
        unwitnessed
    );

    // The remainder — statuses NEITHER side carries, which is lawful normalisation, not a gap. Still
    // ratcheted, because a growing set of them means the two artifacts are drifting apart in scope.
    // Only ever goes DOWN. (2: Qss/ordinary and Qss/ltcg, both "neither".)
    const MAX_UNVALIDATED: usize = 2;
    assert!(
        unvalidated.len() <= MAX_UNVALIDATED,
        "{} schedule(s) have no validated counterpart (ratchet {MAX_UNVALIDATED}): {}.",
        unvalidated.len(),
        unvalidated.join(", ")
    );

    // ── The scalars every return touches ────────────────────────────────────────────────────────
    assert_eq!(
        shipped.ss_wage_base, validated.ss_wage_base,
        "the §3121(a)(1) Social Security wage base differs — this moves Schedule SE on every \
         self-employed return"
    );
}

/// ★★ **The B1 kill, and it is the whole reason this file exists.**
///
/// A checker never watched go red does not exist. This plants the exact divergence the test above
/// guards — one bracket threshold moved by a dollar — and asserts the comparison rejects it.
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

/// ★★★ The SECOND artifact, and the one that carries more of a return.
///
/// `FullReturnParams` holds the standard deduction, the §63(f) aged/blind additions, the SALT
/// limitation, the §199A thresholds and every AMT parameter. A divergence here moves line 12 — and
/// therefore taxable income — on **every** return, while the whole validated corpus stays green,
/// because the corpus is computed against `ty2024_params()` and the binary loads the bundled one.
///
/// ★ Compared field by field for the same reason as the table: a failure must name the parameter.
#[test]
fn the_shipped_ty2024_params_equal_the_ones_every_test_validates() {
    let shipped_all = BundledFullReturnTables::load();
    let s = shipped_all
        .full_return_for(2024)
        .expect("the binary bundles TY2024 full-return params");
    let v = ty2024_params();

    assert_eq!(
        s.year, v.year,
        "the two TY2024 params disagree about their year"
    );
    for st in STATUSES {
        assert_eq!(
            s.std_deduction.get(&st),
            v.std_deduction.get(&st),
            "{st:?}: STANDARD DEDUCTION differs — this moves 1040 line 12, and therefore taxable \
             income, on every return of that status"
        );
    }
    assert_eq!(
        (s.std_aged_blind_married, s.std_aged_blind_unmarried),
        (v.std_aged_blind_married, v.std_aged_blind_unmarried),
        "the §63(f) aged/blind additions differ"
    );
    assert_eq!(
        (s.dependent_std_floor, s.dependent_std_earned_addon),
        (v.dependent_std_floor, v.dependent_std_earned_addon),
        "the §63(c)(5) dependent standard-deduction floor/add-on differ"
    );
    assert_eq!(s.salt, v.salt, "the §164(b) SALT limitation differs");
    assert_eq!(
        s.kiddie_unearned_threshold, v.kiddie_unearned_threshold,
        "the §1(g) kiddie-tax unearned threshold differs"
    );
    assert_eq!(
        (s.qbi_ti_threshold_unmarried, s.qbi_ti_threshold_married),
        (v.qbi_ti_threshold_unmarried, v.qbi_ti_threshold_married),
        "the §199A(e)(2) thresholds differ — this decides whether a return REFUSES for 8995-A"
    );
    assert_eq!(
        (
            s.student_loan_phaseout_unmarried,
            s.student_loan_phaseout_married
        ),
        (
            v.student_loan_phaseout_unmarried,
            v.student_loan_phaseout_married
        ),
        "the §221 student-loan phase-out bands differ"
    );
    assert_eq!(
        s.elective_deferral_limit, v.elective_deferral_limit,
        "the §402(g) elective-deferral limit differs"
    );
    assert_eq!(s.ftc_ceiling, v.ftc_ceiling, "the FTC ceiling differs");
    assert_eq!(
        s.amt, v.amt,
        "the AMT parameters differ — exemption, phase-out and the 28% breakpoint all move Form 6251"
    );
}
