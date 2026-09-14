//! ★★★ **THE TY2026 REHEARSAL HARNESS — stage 2, Tier A (`design/TY2026_REHEARSAL_DESIGN.md` §3).**
//!
//! **THIS HARNESS VALIDATES NOTHING.** OpenTaxSolver 2026 does not exist until ~2027-01-27 and no
//! `f1040--2026` / `i1040gi--2026` is archived, so not one figure it computes may be called correct.
//! Its only job is to find **walls** — places the compute → printed → emit chain either cannot reach
//! TY2026 at all, or reaches it while silently using a TY2025 shape.
//!
//! **UNBUNDLED BY CONSTRUCTION.** `ty2026_full_return()` is injected **at the call site**; nothing
//! here inserts it into `by_year`, so `full_return_for(2026)` stays `None` and no CLI path, no export
//! and no year-readiness probe can reach these params. That is the whole safety argument, and it is
//! structural rather than a promise: this is a `tests/` integration target, and the bundle it would
//! have to mutate is constructed fresh inside `btctax-adapters`.
//!
//! ★★ **WHY THIS FILE LIVES IN `btctax-cli/tests/` AND NOT `btctax-core/tests/`.** It needs the
//! SHIPPED `ty2026_full_return()` transcription rather than a second copy of it, and `btctax-core`
//! does not depend on `btctax-adapters`. Adding it as a **dev**-dependency works and the suite passes
//! — except `repo_hygiene::every_intra_workspace_dependency_pins_the_current_version`, which requires
//! every `btctax-*` path dep to carry a `version`; and a *versioned* dev-dep on a downstream crate
//! breaks the dependency-first publish order (publishing `btctax-core` would demand
//! `btctax-adapters 0.18.0` from crates.io before it exists). `btctax-cli` already has both crates in
//! `[dependencies]`, and `tests/slice_from_answers.rs` already probes `full_return_for(2026)` here, so
//! this is the seam that was already TY2026-aware.
//!
//! ★ Every TY2026 fact asserted below is TRANSCRIBED from a document the repo HOLDS —
//! `design/forms/extract/f1040sa--2026-DRAFT.txt`, `f6251--2026-DRAFT.txt`,
//! `f1040s1--2026-DRAFT.txt` — each marked *"DRAFT — evidence only, never transcribed as authority"*.
//! A draft is enough to prove a **line moved**; it is not enough to fill a form, which is why this
//! file hunts walls and asserts no tax.
//!
//! ## What this harness found, in one table (the report is `design/agent-reports/REPORT-stage2-A.md`)
//!
//! | probe | verdict |
//! |---|---|
//! | `assemble_absolute` at TY2026 | **PANICS** — `form6251_line1_rule` has no 2026 arm ([`the_ty2026_compute_chain_panics_on_form_6251_part_i`]) |
//! | Schedule A line 5e SALT cap / phase-out | **noticed** — it is a parameter, and `params.salt` carries it |
//! | Form 6251 line 5 exemption phase-out | **noticed** — it is a parameter, and `params.amt` carries it |
//! | the Schedule A line SET (13/14/15/16/17a–17z/18/19) | **not noticed, and cannot be** — the printed chain has no year |
//! | Schedule 1 line 14's widened label | **not noticed** — the prose is a hardcoded string |

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use btctax_adapters::tax_tables::ty2026_full_return;
use btctax_core::conventions::Usd;
use btctax_core::event::{BasisSource, DisposeKind};
use btctax_core::forms::InformationReturnRegime;
use btctax_core::identity::{EventId, LotId, WalletId};
use btctax_core::state::{Disposal, DisposalLeg, LedgerState, Term};
use btctax_core::tax::charitable::apply_170b;
use btctax_core::tax::form6251::{compute_6251, Form6251Inputs, Form6251Line1Rule};
use btctax_core::tax::return_1040::{assemble_absolute, schedule_a_parts, ScheduleAParts};
use btctax_core::tax::return_inputs::{
    CharitableClass, CharitableGift, Form1099Div, Form1099Int, HouseholdHeader, Owner, Person,
    ReturnInputs, ScheduleAInputs, W2,
};
use btctax_core::tax::schedule_1a::Schedule1A;
use btctax_core::tax::tables::FullReturnParams;
use btctax_core::tax::testonly::{
    answer_all_live_declarations, form_1098_with_interest, reconcile_digital_asset_activity,
    ty2026_table,
};
use btctax_core::tax::types::FilingStatus;
use rust_decimal_macros::dec;
use time::macros::date;

/// The TY2026 Form 1099-DA regime, as `crates/btctax-forms/forms/2026/YEAR.toml` declares it
/// (`[information_returns.f1099da] proceeds = true, basis = true`). Stated rather than defaulted: the
/// basis leg is what makes the broker-reporting question LIVE, and a rehearsal that quietly used
/// TY2024's `NONE` would skip the one screen TY2026 newly opens.
const TY2026_REGIME: InformationReturnRegime = InformationReturnRegime::PROCEEDS_AND_BASIS;

fn person(first: &str, last: &str, ssn: &str) -> Person {
    Person {
        first_name: first.into(),
        last_name: last.into(),
        ssn: ssn.into(),
        occupation: "Engineer".into(),
        ..Default::default()
    }
}

/// ★★ **THE OWNER'S REAL SHAPE** (stage-2 Tier A brief): W-2 wages + Bitcoin dispositions +
/// **itemized** Schedule A + Schedule B over the $1,500 thresholds. **No Schedule C, no retirement.**
///
/// Itemizing is load-bearing and not a flourish: `choose_deduction` is `max(standard, itemized)`, so
/// only an itemizing household reaches Schedule A at all — and therefore the §164(b)(7) SALT
/// worksheet, the charitable ceilings, and the itemized total whose LINE NUMBER moves in 2026.
///
/// `wages` and `salt_withheld` are the two dials, so one fixture can be driven into three regimes:
/// the ordinary one, the §164(b)(7)(B) SALT phase-out band, and the §55(d) AMT exemption phase-out
/// band. Everything else about the household is identical across the three, which is what makes the
/// cells that move attributable to the dial rather than to the fixture.
fn owner_shape_household(
    status: FilingStatus,
    wages: Usd,
    salt_withheld: Usd,
) -> (ReturnInputs, LedgerState) {
    let spouse = if status == FilingStatus::Mfj {
        Some(person("Robin", "Vale", "987-65-4321"))
    } else {
        None
    };
    let mut ri = ReturnInputs {
        tax_year: 2026,
        filing_status: status,
        header: HouseholdHeader {
            taxpayer: person("Alex", "Vale", "123-45-6789"),
            spouse,
            address_street: "4 Larch Ln".into(),
            address_city: "Springfield".into(),
            address_state: "IL".into(),
            address_zip: "62704".into(),
            ..Default::default()
        },
        w2s: vec![W2 {
            owner: Owner::Taxpayer,
            employer: "SOFTWARE CO".into(),
            box1_wages: wages,
            box2_fed_withheld: wages * dec!(0.2),
            box3_ss_wages: wages,
            box4_ss_withheld: Usd::ZERO,
            box5_medicare_wages: wages,
            box6_medicare_withheld: wages * dec!(0.0145),
            box17_state_tax_withheld: salt_withheld,
            ..Default::default()
        }],
        // Schedule B files: over $1,500 of interest AND over $1,500 of ordinary dividends.
        int_1099: vec![Form1099Int {
            payer: "CREDIT UNION".into(),
            box1_interest: dec!(4200),
            ..Default::default()
        }],
        div_1099: vec![Form1099Div {
            payer: "INDEX FUND".into(),
            box1a_ordinary: dec!(9000),
            box1b_qualified: dec!(7500),
            ..Default::default()
        }],
        // Schedule B Part III must be ANSWERED whenever Schedule B files.
        foreign_accounts: Some(false),
        foreign_trust: Some(false),
        form_1098: vec![form_1098_with_interest(dec!(19000))],
        schedule_a: Some(ScheduleAInputs {
            medical: dec!(3000),
            salt_real_estate: dec!(9500),
            charitable: vec![CharitableGift {
                class: CharitableClass::Cash60,
                amount: dec!(6000),
            }],
            ..Default::default()
        }),
        // §911/931/933: no exclusion, ANSWERED — the §164(b)(7)(B)(iv) modified-AGI worksheet reads
        // it and `line_5e` fails closed on `None`, which is itself a TY2026-only reachability fact
        // (TY2024's `FlatCap` never asks).
        has_income_exclusion: Some(false),
        other_out_of_scope_income: Some(false),
        charitable_cwa_obtained: Some(true),
        donations_had_restrictions: Some(false),
        filing_form_4952: Some(false),
        claiming_mortgage_interest_credit: Some(false),
        ..Default::default()
    };
    // Bitcoin dispositions — one long-term and one short-term, self-custody, disposed inside 2026.
    let state = LedgerState {
        disposals: vec![
            Disposal {
                event: EventId::decision(2),
                kind: DisposeKind::Sell,
                disposed_at: date!(2026 - 03 - 10),
                legs: vec![DisposalLeg {
                    lot_id: LotId {
                        origin_event_id: EventId::decision(3),
                        split_sequence: 0,
                    },
                    sat: 50_000_000,
                    proceeds: dec!(60000),
                    basis: dec!(22000),
                    gain: dec!(38000),
                    term: Term::LongTerm,
                    basis_source: BasisSource::ExchangeProvided,
                    gift_zone: None,
                    acquired_at: date!(2021 - 02 - 01),
                    lot_acquired_at: date!(2021 - 02 - 01),
                    wallet: WalletId::SelfCustody {
                        label: "cold".into(),
                    },
                    pseudo: false,
                }],
                fee_mini_disposition: false,
            },
            Disposal {
                event: EventId::decision(4),
                kind: DisposeKind::Sell,
                disposed_at: date!(2026 - 09 - 02),
                legs: vec![DisposalLeg {
                    lot_id: LotId {
                        origin_event_id: EventId::decision(5),
                        split_sequence: 0,
                    },
                    sat: 10_000_000,
                    proceeds: dec!(9000),
                    basis: dec!(7000),
                    gain: dec!(2000),
                    term: Term::ShortTerm,
                    basis_source: BasisSource::ExchangeProvided,
                    gift_zone: None,
                    acquired_at: date!(2026 - 01 - 05),
                    lot_acquired_at: date!(2026 - 01 - 05),
                    wallet: WalletId::SelfCustody {
                        label: "cold".into(),
                    },
                    pseudo: false,
                }],
                fee_mini_disposition: false,
            },
        ],
        ..Default::default()
    };
    answer_all_live_declarations(&mut ri);
    reconcile_digital_asset_activity(&mut ri, &state, 2026);
    (ri, state)
}

/// AGI for the fixture, by hand from its own leaves — the ONE figure this file computes without the
/// engine, and only because the engine cannot be reached (see the panic probe below). Wages +
/// interest + ordinary dividends + net capital gain; there are no adjustments on this household, so
/// AGI = total income.
///
/// ★ It is derived from the same `ReturnInputs` the probes pass in, so it cannot drift from the
/// fixture. It is NOT a claim that the engine would agree — and it is used only to *position* the
/// fixture inside a phase-out band, never as an expected value.
fn agi_by_hand(ri: &ReturnInputs) -> Usd {
    let wages: Usd = ri.w2s.iter().map(|w| w.box1_wages).sum();
    let interest: Usd = ri.int_1099.iter().map(|i| i.box1_interest).sum();
    let dividends: Usd = ri.div_1099.iter().map(|d| d.box1a_ordinary).sum();
    wages + interest + dividends + dec!(40000) // the two disposals' net gain, 38,000 + 2,000
}

/// The Schedule A parts for the fixture, with the §170(b) ceilings applied at TY2026 AGI. This is the
/// deepest the chain can be driven at TY2026 — `schedule_a_parts` takes `params` and needs no
/// `AbsoluteReturn`, so it is *upstream* of the Form 6251 wall.
fn ty2026_schedule_a(ri: &ReturnInputs, params: &FullReturnParams) -> ScheduleAParts {
    let agi = agi_by_hand(ri);
    let gifts = ri
        .schedule_a
        .as_ref()
        .map(|a| a.charitable.clone())
        .unwrap_or_default();
    let charitable = apply_170b(agi, &gifts, &[], 2026);
    schedule_a_parts(ri, agi, &charitable, params).expect("the fixture itemizes")
}

fn extract(name: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../design/forms/extract")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// Every line id an extract prints, derived from the text layer: an `N`/`Na` token followed by
/// whitespace and then a capital letter or an opening paren (the form's own label column).
///
/// ★★ **WHAT THIS DERIVATION CANNOT SEE, stated because an honest boundary is reviewable and a
/// silent one is the defect** (`CLAUDE.md`, *derive the list* rule 3): `pdftotext -layout` renders the
/// 2025 Schedule A's sub-lines as a bare letter in its own column — `a State and local income
/// taxes` — so `5a`–`5e` and `8a`–`8e` appear in the 2026 id set and NOT in the 2025 one although
/// both revisions print them. A raw set difference therefore **over-reports**, and no assertion below
/// keys on one. What the sets are used for is narrower and layout-independent: the highest line number
/// each revision reaches, and the presence of an id with a letter suffix the other revision does not
/// use at all (`17z`).
fn line_ids(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for raw in text.lines() {
        let bytes: Vec<char> = raw.chars().collect();
        let mut i = 0;
        while i < bytes.len() {
            // The token must start at the line start or after whitespace.
            if i > 0 && !bytes[i - 1].is_whitespace() {
                i += 1;
                continue;
            }
            let start = i;
            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let digits = j - start;
            if digits == 0 || digits > 2 {
                i = if j > start { j } else { i + 1 };
                continue;
            }
            let mut end = j;
            if end < bytes.len() && bytes[end].is_ascii_lowercase() {
                end += 1;
                // Exactly one suffix letter, and the next char must not continue a word.
                if end < bytes.len() && bytes[end].is_ascii_alphanumeric() {
                    i = end;
                    continue;
                }
            }
            let mut k = end;
            let mut spaces = 0;
            while k < bytes.len() && bytes[k] == ' ' {
                k += 1;
                spaces += 1;
            }
            if spaces >= 1 && k < bytes.len() && (bytes[k].is_ascii_uppercase() || bytes[k] == '(')
            {
                out.insert(bytes[start..end].iter().collect::<String>());
            }
            i = end.max(start + 1);
        }
    }
    out
}

/// The highest *whole* line number an id set reaches.
fn max_line(ids: &BTreeSet<String>) -> u32 {
    ids.iter()
        .filter_map(|s| {
            s.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse::<u32>()
                .ok()
        })
        .max()
        .expect("a form has at least one numbered line")
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// §1 — THE FIRST WALL, AND IT IS A `panic!` RATHER THAN A REFUSAL.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **THE HEADLINE WALL: `assemble_absolute` at TY2026 ABORTS THE PROCESS.**
///
/// `form6251_line1_rule` (`return_1040.rs`) is `match year { 2024 => …, 2025 => …, _ => None }`, and
/// `form6251_inputs_from_parts` turns that `None` into a `panic!`. Form 6251 is computed on **every**
/// return since v0.14.0, so *every* TY2026 return reaches it — this fixture is a wage earner with two
/// Bitcoin sales, not an exotic filer.
///
/// ★★ **The gate itself is right and its reasoning is right**: TY2025 line 1a subtracts *Schedule 1-A
/// line 37* and the TY2026 draft subtracts *line 43*, and line 37 is not vacated by the move — on the
/// 2026 schedule it is modified AGI. Substituting one for the other overstates the AMT base by
/// roughly MAGI. Failing closed is correct.
///
/// ★★★ **What is wrong is the CHANNEL.** A `panic!` is not a [`btctax_core::tax::RefuseReason`]:
/// * it cannot carry the *exit* a refusal carries, so a filer meets a backtrace rather than a
///   sentence and a next step;
/// * `xtask blockers`' refusal census enumerates `RefuseReason` variants and year-keyed *gate
///   functions*, and a `panic!` is neither — so this wall is **structurally invisible to the
///   prediction instrument**, which is why it is an unpredicted row rather than a confirmed one.
///
/// This test pins the wall with `catch_unwind`, and REDS the day the 2026 arm lands — which is
/// exactly when someone must come back and re-run the rest of this file end to end.
#[test]
fn the_ty2026_compute_chain_panics_on_form_6251_part_i() {
    let (ri, state) = owner_shape_household(FilingStatus::Mfj, dec!(220000), dec!(14000));
    let prior = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = std::panic::catch_unwind(|| {
        assemble_absolute(&ri, &state, &ty2026_full_return(), &ty2026_table(), 2026)
    });
    std::panic::set_hook(prior);

    let Err(err) = outcome else {
        panic!("TY2026 must still be unreachable; if this reds, the 2026 Part I arm landed")
    };
    let msg = err
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_else(|| String::from("<non-String panic payload>"));
    assert!(
        msg.contains("Form 6251 Part I has never been transcribed for TY2026"),
        "the wall must still be the Form 6251 Part I one; got: {msg}"
    );
    assert!(
        msg.contains("form6251_line1_rule"),
        "the panic must still name the fix site; got: {msg}"
    );
}

/// ★ The one screen that runs *before* the wall also refuses TY2026 — but for a different reason, and
/// it is worth separating: the 1099-DA regime's basis leg is live in 2026, so a TY2026 return with
/// exchange dispositions needs the broker-reporting answers. This fixture is entirely self-custody,
/// so that screen is silent and the panic is the FIRST thing a TY2026 filer meets.
#[test]
fn the_ty2026_regime_is_the_basis_regime_and_self_custody_does_not_trip_it() {
    // The regime the year record declares, read back off the const so a change to it lands here.
    assert_eq!(
        (TY2026_REGIME.proceeds, TY2026_REGIME.basis),
        (true, true),
        "forms/2026/YEAR.toml declares proceeds AND basis"
    );
    let (_, state) = owner_shape_household(FilingStatus::Mfj, dec!(220000), dec!(14000));
    let rows = btctax_core::forms::form_8949(&state, 2026);
    assert_eq!(rows.len(), 2, "two disposals, two Form 8949 rows");
    assert!(
        !btctax_core::forms::broker_question_is_live(&rows, TY2026_REGIME),
        "self-custody rows are Noncovered, so the 1099-DA question is not live on this fixture"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// §2 — WHAT THE CHAIN *DOES* NOTICE: the PARAMETERIZED TY2026 changes, probed upstream of the wall.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// **FR-219 — Schedule A line 5e's SALT cap, $40,000 → $40,400.**
///
/// The 2026 draft prints, verbatim: *"Enter the smaller of line 5d or $40,400 ($20,200 if married
/// filing separately). If Form 1040 or 1040-SR, line 11b, is more than $505,000 ($252,500 if married
/// filing separately), or if you completed Form 2555, Form 4563, or excluded income from Puerto Rico,
/// see instructions"*. TY2025's own line 5e printed *$40,000 / $20,000* and *$500,000 / $250,000*.
///
/// The literals here are the DRAFT FORM'S OWN. `params.salt.line_5e` is the instrument and
/// `ScheduleAParts::salt_5e` is what the printed form reads, so this asks whether the worksheet's
/// result actually reaches the cell.
#[test]
fn the_chain_notices_the_ty2026_salt_cap() {
    // $23,500 of SALT — above TY2025's $10,000-era flat cap, below the 2026 limitation, MAGI far
    // below the threshold ⇒ 5e must be the whole 5d.
    let (ri, _) = owner_shape_household(FilingStatus::Mfj, dec!(220000), dec!(14000));
    let p = ty2026_schedule_a(&ri, &ty2026_full_return());
    assert_eq!(p.salt_5d, dec!(23500), "5a $14,000 + 5b $9,500");
    assert_eq!(p.salt_5e, dec!(23500), "under the limitation, 5e = 5d");

    // Over the limitation, still under the threshold.
    let (ri, _) = owner_shape_household(FilingStatus::Mfj, dec!(300000), dec!(60000));
    let p = ty2026_schedule_a(&ri, &ty2026_full_return());
    assert_eq!(
        p.salt_5e,
        dec!(40400),
        "the 2026 applicable limitation amount the draft form prints"
    );
    assert_ne!(p.salt_5e, dec!(40000), "TY2025's amount");
    assert_ne!(p.salt_5e, dec!(10000), "the pre-OBBBA flat cap");
}

/// **FR-219 — the §164(b)(7)(B) phase-out threshold $500,000 → $505,000, at 30%.**
///
/// §164(b)(7)(B) (Pub. L. 119-21 §70120) reduces the limitation by 30% of modified AGI over the
/// threshold, never below $10,000. The arithmetic below is the STATUTE'S with the draft's threshold: a
/// $505,000 threshold and a $500,000 one differ by $1,500 of allowed SALT anywhere in the band, so the
/// probe would notice a TY2025 instrument even though both years phase out at the same rate.
#[test]
fn the_chain_notices_the_ty2026_salt_phaseout_threshold_and_rate() {
    let (ri, _) = owner_shape_household(FilingStatus::Mfj, dec!(500000), dec!(60000));
    let params = ty2026_full_return();
    let magi = agi_by_hand(&ri);
    // The $10,000 floor binds from MAGI = 505,000 + (40,400 − 10,000)/0.30 = $606,333.33 up, so the
    // window where the RATE is observable is (505,000, 606,333.33). Both bounds are computed from the
    // draft form's own figures, never typed as a remembered number.
    let floor_binds_at = dec!(505000) + (dec!(40400) - dec!(10000)) / dec!(0.30);
    assert!(
        magi > dec!(505000) && magi < floor_binds_at,
        "the fixture must sit inside the band and clear of the floor ({floor_binds_at}); MAGI was {magi}"
    );
    let p = ty2026_schedule_a(&ri, &params);
    let expected = (dec!(40400) - dec!(0.30) * (magi - dec!(505000))).max(dec!(10000));
    assert_eq!(p.salt_5e, expected, "§164(b)(7)(B) at 30% over $505,000");
    let ty2025_shape = (dec!(40000) - dec!(0.30) * (magi - dec!(500000))).max(dec!(10000));
    assert_ne!(
        p.salt_5e, ty2025_shape,
        "TY2025's $40,000/$500,000 pair must not be what ran"
    );

    // Deep in the band the $10,000 floor binds — the one place both years agree, asserted separately
    // so the floor never stands in for the band.
    let (ri, _) = owner_shape_household(FilingStatus::Mfj, dec!(1200000), dec!(60000));
    let p = ty2026_schedule_a(&ri, &params);
    assert_eq!(p.salt_5e, dec!(10000), "§164(b)(7)(B)'s floor");
}

/// **FR-212 — Form 6251 line 5's exemption phase-out threshold, and §70107(c)'s doubled rate.**
///
/// The 2026 draft's own line-5 table: *"Single or head of household … $ 500,000 … $ 90,100 / Married
/// filing jointly or qualifying surviving spouse 1,000,000 … 140,200 / Married filing separately …
/// 500,000 … 70,100"*. TY2024's pair was $626,350 / $1,252,700, and Pub. L. 119-21 §70107(c) replaces
/// *"25 percent"* with *"50 percent"* for years beginning after 2025-12-31.
///
/// ★★ **THE SUBSTITUTION THIS PROBE MAKES, DECLARED.** The wall above means `assemble_absolute`
/// cannot build the form, so the inputs are handed to the public `compute_6251` directly with
/// `Form6251Line1Rule::Y2025`. That is legitimate and bounded:
/// * the 2026 draft's line 1a/1b arithmetic is IDENTICAL to 2025's — *"Subtract Schedule 1-A (Form
///   1040), line 43, from … line 14"* / *"Subtract line 1a from … line 11b"*; only the Schedule 1-A
///   line cited moves, 37 → 43;
/// * this household has **no** senior deduction, so the subtracted subtotal is `$0` under either
///   revision and the 37/43 collision cannot move a figure here. It is a document substitution, not
///   an invented figure — and it is precisely why the production gate must stay shut, because a
///   household WITH a senior deduction is where the collision bites.
///
/// ★ Both TY2026 moves point the same way (a smaller exemption, sooner), so a chain that mixed
/// TY2025's threshold with TY2026's rate would still produce a plausible AMT. Each half is refuted
/// separately.
#[test]
fn the_chain_notices_the_ty2026_amt_exemption_phaseout() {
    let params = ty2026_full_return();
    // AMTI in the band: on this household AMTI is AGI (no preferences), so dial wages until AGI is
    // between $1,000,000 and $1,280,400 — where the 2026 exemption is partly, not wholly, gone.
    let (ri, _) = owner_shape_household(FilingStatus::Mfj, dec!(1060000), dec!(60000));
    let agi = agi_by_hand(&ri);
    let p = ty2026_schedule_a(&ri, &params);
    let itemized = p.total_17;
    let taxable = (agi - itemized).max(Usd::ZERO);

    let f = compute_6251(
        Form6251Inputs {
            status: FilingStatus::Mfj,
            taxable_income_l15: taxable,
            agi_l11: agi,
            line1_rule: Form6251Line1Rule::Y2025 {
                form_1040_l11b: agi,
                form_1040_l14: itemized,
                senior_deduction: Schedule1A::senior_deduction_subtotal(None),
            },
            deduction_l12: itemized,
            deduction_l14: itemized,
            schedule_a_line7: p.salt_5e,
            itemized: true,
            state_refund_sch1_l1: Usd::ZERO,
            net_capital_gain: dec!(38000),
            qualified_dividends: dec!(7500),
            qdcgt_line5_regular: taxable - dec!(45500),
            regular_tax_l16: Usd::ZERO,
            schedule_2_line1z: Usd::ZERO,
            schedule_3_line1: Usd::ZERO,
        },
        &params.amt,
        ty2026_table().ltcg_for(FilingStatus::Mfj),
    );
    let amti = f.line4;
    assert!(
        amti > dec!(1000000) && amti < dec!(1280400),
        "the fixture must land inside the 2026 phase-out band; AMTI was {amti}"
    );
    let ty2026 = (dec!(140200) - dec!(0.50) * (amti - dec!(1000000))).max(Usd::ZERO);
    assert_eq!(f.line5, ty2026, "§55(d)(4) at 50% over $1,000,000");

    // Refute each older half on its own.
    let old_threshold = (dec!(140200) - dec!(0.50) * (amti - dec!(1252700))).max(Usd::ZERO);
    let old_rate = (dec!(140200) - dec!(0.25) * (amti - dec!(1000000))).max(Usd::ZERO);
    assert_ne!(f.line5, old_threshold, "TY2024's $1,252,700 threshold");
    assert_ne!(f.line5, old_rate, "the pre-§70107(c) 25% rate");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// §3 — WHAT THE CHAIN DOES **NOT** NOTICE: the STRUCTURAL changes.
//
// ★★★ Everything in §2 is a NUMBER the params carry, and the chain reads params, so it notices. The
//     probes below are LINE SETS, which no parameter can carry: `ScheduleALines` is a single struct
//     with TY2025's numbering in its field names, `schedule_a_lines(ar, line11)` takes neither a year
//     nor the params, and `assemble_printed_forms` is not passed the params either. So the printed
//     chain cannot notice, and these tests pin that it does not — with the document's own sentences
//     as the statement of what a TY2026 port must build.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **SIX SCHEDULE A CELLS CHANGE MEANING IN 2026, AND THE PRINTED CHAIN HAS ONE SHAPE.**
///
/// Transcribed from the two text layers (the sentences are asserted below, so this table is checked
/// rather than claimed):
///
/// | line | TY2025 | TY2026 draft |
/// |---|---|---|
/// | 8d | *"Reserved for future use"* | *"Mortgage insurance premiums"* (§70108(a)(1)(D) revives it) |
/// | 8e | *"Add lines 8a through 8c"* | *"Add lines 8a through 8d"* |
/// | 13 | *"Carryover from prior year"* | *"Enter the amount from line 6 of the Charitable Contribution Limitation Worksheet"* |
/// | 14 | *"Add lines 11 through 13"* | *"Carryover from prior year"* |
/// | 15 | Casualty and theft loss(es) | *"Add lines 13 and 14"* |
/// | 16 | *"Other—from list in instructions"* | Casualty and theft loss(es) |
/// | 17 | **the total**, → 1040 line 12e | *"Other itemized deductions"*, with new 17a–17k and 17z |
/// | 18 | the §63(e) elect-smaller box | **the total**, behind a `$384,350` gate |
/// | 19 | — | the §63(e) elect-smaller box |
///
/// So a TY2026 field map fed the current printed chain would put the **itemized total** in the *Other
/// itemized deductions* box and the **charitable total** on the *Carryover from prior year* line.
///
/// ★★ And `ScheduleAParts::total_17` is `medical + salt + mortgage + investment interest +
/// charitable` — TY2025's *"add lines 4 through 16"* with 15 and 16 legitimately blank. The 2026 total
/// is line 18, which is gated and may be **limited**; nothing in the chain computes that limitation,
/// and not computing it OVERSTATES the deduction, i.e. understates the tax.
#[test]
fn the_printed_schedule_a_is_the_ty2025_line_set() {
    let sa25 = extract("f1040sa--2025.txt");
    let sa26 = extract("f1040sa--2026-DRAFT.txt");

    // The sentences that move. Each is the form's own, so a re-extract that changes one reds here
    // rather than silently invalidating the table above.
    for needle in [
        "Reserved for future use",
        "Add lines 8a through 8c",
        "Carryover from prior year",
        "Add lines 11 through 13",
        "Other—from list in instructions",
        "line 12e",
    ] {
        assert!(
            sa25.contains(needle),
            "TY2025 Schedule A must print {needle:?}"
        );
    }
    for needle in [
        "Mortgage insurance premiums",
        "Add lines 8a through 8d",
        "Limitation Worksheet",
        "Add lines 13 and 14",
        "Other itemized deductions",
        "Deductible gambling losses",
        "Add lines 17a through 17k",
        "$384,350",
        "Itemized Deductions Worksheet",
    ] {
        assert!(
            sa26.contains(needle),
            "the 2026 Schedule A draft must print {needle:?}"
        );
        assert!(
            !sa25.contains(needle),
            "{needle:?} must be NEW in 2026 for the collision table to hold"
        );
    }

    // Derived: the tail shifted by exactly one line, and 2026 opened a `z` sub-line 2025 never used.
    let ids25 = line_ids(&sa25);
    let ids26 = line_ids(&sa26);
    assert_eq!(max_line(&ids25), 18, "TY2025 Schedule A ends at line 18");
    assert_eq!(max_line(&ids26), 19, "the 2026 draft ends at line 19");
    assert!(ids26.contains("17z") && !ids25.contains("17z"));

    // Code side: the chain's total is the TY2025 sum, over the TY2025 blocks.
    let (ri, _) = owner_shape_household(FilingStatus::Mfj, dec!(220000), dec!(14000));
    let p = ty2026_schedule_a(&ri, &ty2026_full_return());
    assert_eq!(
        p.total_17,
        p.medical_allowed
            + p.salt_5e
            + p.mortgage_8a
            + p.mortgage_8b
            + p.mortgage_8c
            + p.investment_interest_9
            + p.charitable_14,
        "TY2025's 'add lines 4 through 16' with 15 and 16 blank — no 8d term, no §68 limitation, and \
         the total is called 17"
    );
}

/// **The two TY2026 worksheets Schedule A now cites exist in NO document the repo holds.**
///
/// Line 13 cites a *Charitable Contribution Limitation Worksheet*; line 18 cites an *Itemized
/// Deductions Worksheet*, both *"in the instructions"*. The Schedule A instructions for 2026
/// (`i1040sca--2026`) are not archived, and neither is `i1040gi--2026`. This is the honest statement
/// of a blocker: the form we hold names two worksheets whose line sets we cannot read, so they cannot
/// be transcribed, so neither line can be computed.
#[test]
fn the_two_new_ty2026_worksheets_are_in_no_archived_document() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../design/forms/extract");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("the extract archive")
        .map(|e| {
            e.expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    for missing in ["i1040sca--2026.txt", "i1040gi--2026.txt"] {
        assert!(
            !names.contains(&missing.to_string()),
            "{missing} is now archived — the worksheets can be transcribed; update the report"
        );
    }
    // And no archived document anywhere carries either worksheet's own line set.
    let mut carriers = Vec::new();
    for n in &names {
        let text = std::fs::read_to_string(dir.join(n)).unwrap_or_default();
        if text.contains("Charitable Contribution Limitation Worksheet") {
            carriers.push(format!("{n} (charitable limitation)"));
        }
    }
    assert!(
        carriers.iter().all(|c| c.starts_with("f1040sa--2026-DRAFT")),
        "only the 2026 Schedule A draft CITES the charitable worksheet; a document that CONTAINS it \
         would be a new fact: {carriers:?}"
    );
}

/// **FR-220 — Schedule 1 line 14's eligibility widened, and the prose we ship is TY2025's.**
///
/// 2026 draft: *"Moving expenses for members of the Armed Forces **and the intelligence community**.
/// Attach Form 3903."* TY2025: *"Moving expenses for members of the Armed Forces."*
///
/// btctax models none of the Schedule 1 Part II adjustments, so **no figure moves**. What moves is the
/// sentence a filer reads: `Advisory::UnmodeledDeductionsOmitted` lists the deductions btctax did not
/// compute so the filer can claim them, and an intelligence-community filer reading *"moving expenses
/// for the Armed Forces"* is being told the omission is not theirs. The advisory is a hardcoded string
/// with no year discriminant.
///
/// ★ The same advisory also names Schedule A *"line 15, Form 4684"* and *"line 16"* — which on the
/// 2026 revision are lines **16** and **17**. Pinned here too, because the fix is one edit and the two
/// halves would otherwise be found a round apart.
#[test]
fn schedule_1_line_14_widened_and_our_prose_did_not() {
    let s26 = extract("f1040s1--2026-DRAFT.txt");
    let s25 = extract("f1040s1--2025.txt");
    assert!(
        s26.contains("Armed Forces and the intelligence community"),
        "the 2026 draft's own line-14 label"
    );
    assert!(
        !s25.contains("Armed Forces and the intelligence community"),
        "TY2025's label must be the narrower one for this to be a CHANGE"
    );
    let adv = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../btctax-core/src/tax/advisories.rs"),
    )
    .expect("advisories.rs");
    assert!(
        adv.contains("moving expenses for the Armed Forces"),
        "if this reds the prose was widened — update the report, not the assertion"
    );
    assert!(
        !adv.contains("intelligence community"),
        "the advisory has no year and still names only the Armed Forces"
    );
    assert!(
        adv.contains("(line 15, Form 4684)"),
        "the advisory cites Schedule A's TY2025 casualty line; TY2026's is line 16"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// §4 — THE FIXTURE ITSELF STILL COMPUTES AT A BUNDLED YEAR.
//
// ★ Without this, every probe above could be passing because the fixture is malformed rather than
//   because TY2026 is unreachable — a green-and-blind instrument. Driving the SAME household through
//   the SAME entry point at TY2024 shows the chain is otherwise fine and the year is the variable.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// The identical household at a year whose Form 6251 Part I *is* transcribed: no panic, a full
/// `AbsoluteReturn`, and an itemized deduction. **No figure here is validated either** — the point is
/// only that the fixture is computable and the wall is about the YEAR.
#[test]
fn the_same_household_computes_at_a_bundled_year() {
    let (mut ri, state) = owner_shape_household(FilingStatus::Mfj, dec!(220000), dec!(14000));
    ri.tax_year = 2024;
    let params = btctax_core::tax::testonly::ty2024_params();
    let table = btctax_core::tax::testonly::ty2024_table();
    let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
    assert!(
        ar.deduction_is_itemized,
        "the fixture itemizes at TY2024 too"
    );
    assert!(ar.agi > Usd::ZERO);
    let printed = btctax_core::tax::packet::assemble_printed_forms(
        &ri,
        &state,
        &BTreeMap::new(),
        &ar,
        &table,
        2024,
        &[],
        InformationReturnRegime::NONE,
    );
    assert!(printed.sch_a.is_some(), "Schedule A files at TY2024");
    assert!(printed.sch_b.is_some(), "Schedule B files at TY2024");
    assert!(
        printed.sch_c.is_none(),
        "no Schedule C on the owner's shape"
    );
    // ★ Form 8949 is correctly ABSENT here: both disposals are dated in 2026, so a TY2024 run has no
    //   Form 8949 rows. That is the year filter working, and it is asserted rather than left implicit
    //   so nobody reads this test as "the packet is complete at 2024".
    assert!(
        printed.f8949.is_none(),
        "the disposals are dated 2026; a TY2024 run must have no Form 8949"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// §5 — THE ONE GUARD, AND THE YEAR IT GUARDS IS THE DEFAULT.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **THE §2 GATE IS NOT SUFFICIENT, AND THIS IS WHY.**
///
/// `design/TY2026_REHEARSAL_DESIGN.md` §2 makes the bundling of `ty2026_full_return()` conditional on
/// the TY2026 **1040 and its instructions** being archived *and transcribed*. That condition says
/// nothing about Form 6251 Part I or about Schedule 1-A — and this test shows what that omission costs:
///
/// 1. `default_year()` is **2026** (it is `max(bundled_years())`, and `forms/2026/` exists), so a
///    no-`--year` invocation is a TY2026 invocation. `year_readiness`'s own
///    `the_default_year_is_the_newest_bundled_one` asserts exactly this.
/// 2. The *only* thing standing between that default path and `assemble_absolute` is
///    `full_return_for(year).is_some()` — `cmd::tax` and `cmd::admin` both destructure it and return
///    `CliError::Usage` on `None`. There is no second guard.
/// 3. And `assemble_absolute` at TY2026 **panics** ([`the_ty2026_compute_chain_panics_on_form_6251_part_i`]).
///
/// So the single insert the design calls *"the rehearsal's core"* converts the default year's default
/// path from a clean refusal into a process abort. **The gate must also require the TY2026 Form 6251
/// Part I arm** — which in turn requires the TY2026 Schedule 1-A, because line 1a cites its line 43 and
/// `SeniorDeductionSubtotal` has no lawful way to vouch for a line the schedule has not printed.
#[test]
fn the_params_bundle_is_the_only_guard_in_front_of_the_panic() {
    assert_eq!(
        btctax_cli::year_readiness::default_year(),
        2026,
        "the default year is the newest BUNDLED year, and forms/2026/ exists"
    );
    let fr = btctax_adapters::BundledFullReturnTables::load();
    assert!(
        btctax_core::tax::tables::FullReturnTables::full_return_for(&fr, 2026).is_none(),
        "the params must stay unbundled; if this reds, the panic above is live on the default year"
    );
    // The guard's own sentence, so a reader can see it is a Usage refusal and not a second gate.
    let sentence = btctax_cli::year_readiness::uncomputable_sentence(2026, false);
    assert!(
        !sentence.is_empty(),
        "the refusal that stands in for the missing params must say something"
    );
}

// ══ THE GATE MY §2 DESIGN MISSED ═══════════════════════════════════════════════════════════════════
//
// ★★★ `blockers::no_bundled_params_year_has_unreadable_forms` checks that a bundled year's FORMS can be
//     READ — the 1040 and its instructions archived, the §111(a) revision transcribed. That is necessary
//     and it is NOT SUFFICIENT, as this file's own panic probe proves: it names neither Form 6251 Part I
//     nor Schedule 1-A, so it would happily permit `by_year.insert(2026, ty2026_full_return())` — the one
//     line the stage-2 design calls "the rehearsal's core" — and that insert converts the DEFAULT year's
//     DEFAULT path from a clean refusal into an ABORT, because `default_year()` is 2026.
//
// ★★ The fix is deliberately NOT a second list of year-keyed rules. Enumerating them would be the very
//    disease (`blockers.rs`'s own gate set is a hand-written `vec![…]` of 7, which is why it could not see
//    `form6251_line1_rule`). Instead this asserts the OBSERVABLE PROPERTY, derived over the bundled years:
//    a year this build declares computable must COMPUTE OR REFUSE — never abort. It does not care which
//    rule lacks an arm, which is exactly why a new one cannot slip past it.

/// ★★★ **No year whose `FullReturnParams` are bundled may ABORT on the ordinary path.** A `panic!` carries
/// no [`btctax_core::tax::RefuseReason`], so it carries no exit and no remedy — a filer meets a backtrace.
///
/// Today only TY2024 is bundled and it passes. The moment TY2026's params are inserted this reds, naming
/// the year and the panic message, because [`the_ty2026_compute_chain_panics_on_form_6251_part_i`] above
/// measures exactly that abort. **That is the gate the stage-2 design should have had.**
#[test]
fn no_bundled_params_year_aborts_the_ordinary_path() {
    let years: Vec<i32> = btctax_forms::bundled::bundled_years()
        .iter()
        .copied()
        .filter(|y| {
            use btctax_core::tax::tables::FullReturnTables;
            btctax_adapters::tax_tables::BundledFullReturnTables::load()
                .full_return_for(*y)
                .is_some()
        })
        .collect();
    assert!(
        !years.is_empty(),
        "no year has FullReturnParams bundled, so this gate would pass vacuously — which is itself the bug"
    );

    let mut aborted = Vec::new();
    for y in years {
        use btctax_core::tax::tables::{FullReturnTables, TaxTables};
        let ft = btctax_adapters::tax_tables::BundledFullReturnTables::load();
        let params = ft.full_return_for(y).expect("filtered above").clone();
        let tt = btctax_adapters::tax_tables::BundledTaxTables::load();
        let table = tt
            .table_for(y)
            .expect("a bundled-params year must have a TaxTable too")
            .clone();
        let (ri, state) = owner_shape_household(FilingStatus::Mfj, dec!(220000), dec!(14000));
        let mut ri = ri;
        ri.tax_year = y;

        let prior = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let outcome =
            std::panic::catch_unwind(|| assemble_absolute(&ri, &state, &params, &table, y));
        std::panic::set_hook(prior);

        if let Err(e) = outcome {
            let msg = e
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_else(|| String::from("<non-String panic payload>"));
            aborted.push(format!("TY{y}: ABORTED rather than refused — {msg}"));
        }
    }
    assert!(
        aborted.is_empty(),
        "a year is declared computable whose ordinary path aborts. A panic carries no RefuseReason, so \
         it carries no exit: the filer meets a backtrace instead of a remedy. Give the year its arm, or \
         do not bundle its params.\n{}",
        aborted.join("\n")
    );
}
