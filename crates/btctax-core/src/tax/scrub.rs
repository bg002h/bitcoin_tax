//! Replace a return's IDENTITY with synthetic stand-ins, preserving every figure and every field the
//! computation reads — so a real return can be handed to someone else to reproduce a defect.
//!
//! ## The guarantee, and the test that holds it
//!
//! **Scrubbing must not change a single computed number.** That is the whole value of the thing: a
//! scrubbed return that files differently from the original is worse than useless, because it sends
//! the recipient after a bug that is not there. The invariant is asserted directly —
//! `assemble_absolute(original) == assemble_absolute(scrubbed)`, over the whole `AbsoluteReturn`, not
//! a sampled figure — in `scrub_preserves_every_computed_figure`.
//!
//! ## ★★★ WHY THIS DESTRUCTURES EXHAUSTIVELY
//!
//! Every struct touched here is taken apart with **no `..`** — `ReturnInputs` itself included. A
//! scrubber that silently passes a NEW PII field through is worse than no scrubber at all: it carries
//! the user's authorisation ("this file is safe to share") onto a file that is not. So adding a field
//! to `ReturnInputs`, `Person`, `Dependent` or `HouseholdHeader` is *pattern does not mention field*
//! here, and the author must decide which side it falls on. The same argument the P9 classifier makes
//! about answered-ness, applied to disclosure.
//!
//! ★★★ THE TOP LEVEL WAS THE ONE THAT MATTERED, AND IT WAS THE ONE THAT WAS MISSING. The first
//! version of this module made the sentence above true of the three nested structs and false of
//! `ReturnInputs`, which was a `clone()` plus targeted mutation — so `business_description` (free
//! text, where a filer writes their business NAME) and `foreign_country_names` (which countries this
//! person banks in) rode straight through into a file the tool called shareable. New input types are
//! added at the TOP level, so that is precisely where the compile-time guard was worth having. Caught
//! by a security review, not by the suite.
//!
//! ## What is REPLACED, and what must survive
//!
//! | replaced — read by nothing but the printer | preserved — the computation reads it |
//! |---|---|
//! | names, SSNs, occupation, street/city/state/ZIP, IP PIN | filing status, every dollar figure |
//! | employer / payer / bank names | dates of birth and death, blindness |
//! | Schedule C `business_description` (free text) | `can_be_claimed_*`, the spouse flags, presidential-fund boxes |
//! | foreign account COUNTRY NAMES (count kept) | the NUMBER of dependents and each `relationship` |
//! | | Schedule C `naics_code` — a federal industry code, not a person |
//!
//! ★★ **EINs are replaced but keep their SAMENESS.** §6413(c)'s excess-social-security credit turns on
//! having *more than one employer*, decided by comparing canonicalised W-2 EINs
//! (`return_1040.rs`) — a real understatement bug in v0.15.0 was exactly a comparison that got this
//! wrong. So each DISTINCT real EIN maps to a distinct synthetic one, and two W-2s that shared an
//! employer still share one afterwards. Replacing them independently would silently turn one employer
//! into two and manufacture a credit; replacing them all with one constant would do the reverse.
//!
//! ★ SSNs are emitted with a middle group of `00`, which the SSA never issues — so the output is safe
//! by construction and needs no allowlist entry if it is ever committed as a fixture.

use crate::state::LedgerState;
use crate::tax::return_inputs::{Dependent, HouseholdHeader, Person, ReturnInputs};
use std::collections::BTreeMap;

/// **THE OBLIGATION.** Scrub may emit only when the ledger contributes **nothing** to any figure,
/// refusal, gate, watermark or advisory btctax produces for `year`. This function owns that
/// sentence; `true` means the year is NOT scrubbable and the command must refuse.
///
/// ★★★ IT IS SCRUB-OWNED, AND IT IS **NOT** THE 1040 DIGITAL-ASSET BOX. The first draft reused
/// [`crate::tax::return_1040::digital_asset_activity`] alone, arguing the refusal was "exactly as
/// accurate as a box btctax already prints". Both halves of that were wrong, and a spec review
/// killed it (SPEC_income_scrub.md §2.2):
///
/// 1. **Year-scope is not artifact-scope.** [`LedgerState::pseudo_active`] and the Hard-blocker gate
///    are **projection-wide and year-blind**. A vault
///    whose only activity is an unclassified inbound has no in-year digital-asset activity at all,
///    yet the FILER's export is DRAFT-watermarked and attestation-gated while the recipient's copy
///    would be clean and ungated. A 2022 `ImportConflict` makes the filer's 2024 delta
///    `NotComputable` and the recipient's computable. **That is precisely the filer most likely to
///    run `income scrub`** — "btctax won't export my forms" — handed the one file on which the
///    problem vanishes.
/// 2. **It leaned on a don't-know.** `digital_asset_activity`'s own doc says `false` leaves the box
///    **unchecked, NOT answered "No"**. Reading that deliberate silence as "the ledger is empty" is
///    widening an exemption, which is never the safe edit.
///
/// Every disjunct names the mechanism that put it there:
///
/// - `digital_asset_activity(state, year)` — disposals / income / removals dated in-year.
/// - `digital_asset_activity(state, year - 1)` — ★ **DELIBERATE OVER-REFUSAL that secures no live
///   read.** Two rounds of naming a mechanism for it were both wrong: it is *not* the M4
///   carryforward-consistency advisory (which needs the prior year's stored *profile*, so it does
///   not reproduce from a one-year file whatever this decides), and it is *not* "the prior year's
///   ledger feeds this year's carryforward-in" ([`scrub_pii`] preserves
///   `capital_loss_carryforward_in` and its provenance verbatim, so those figures travel in the
///   TOML and reproduce with no ledger at all). It is kept because scrub declines to prove the
///   negative that no path reads the prior year's ledger live. The error direction is over-refusal,
///   which is the safe one. **Do not "fix" it by inventing a third mechanism.**
/// - [`LedgerState::pseudo_active`] — the DRAFT watermark and the attestation gate.
/// - An unresolved `Severity::Hard` blocker — `NotComputable`, projection-wide.
///
/// ★★★ **THE HARD-BLOCKER DISJUNCT DOES NOT CALL `compute::first_hard_blocker`, AND THAT IS A
/// CORRECTION TO THE SPEC.** SPEC_income_scrub.md §2.2 directed widening that helper to
/// `pub(crate)`. It cannot be: `tax/compute.rs` is a **frozen engine file**, content-pinned by
/// [`crate::tax::frozen_guard`] (SPEC_full_return §2), and even an edit preserving the public API
/// trips `frozen_engine_files_are_unchanged` — which is exactly what the pin is for. Its exception
/// process demands a separately-reviewed commit, and "a sharing feature wanted one more caller" is
/// not the exceedingly-rare case that process exists for.
///
/// So this uses the predicate every other non-frozen caller already uses —
/// `state.blockers.iter().any(|b| b.kind.severity() == Severity::Hard)`, as at `admin.rs:234`,
/// `:712` and `:972`. `first_hard_blocker` has no caller outside `compute.rs`; it returns the FIRST
/// such blocker for a deterministic `TaxYearNotComputable` detail, and this only needs existence.
/// ★ Five spec-review rounds mandated an edit to a frozen file and none of them noticed. It was a
/// `cargo nextest` away, and the build found it in one run.
///
/// ★ The caller must also **project the ledger** before asking, and **refuse if projection fails**
/// — never fall back to "assume empty".
#[must_use]
pub fn ledger_contributes(state: &LedgerState, year: i32) -> bool {
    ledger_contribution(state, year).is_some()
}

/// Which disjunct of [`ledger_contributes`] fires, so a refusal can name its own mechanism.
///
/// ★★ The CAUSE is the primitive and the bool is derived from it — `ledger_contributes` is literally
/// `self.is_some()`. The alternative (a `bool` here plus a separate cause-finder for the message, or
/// the CLI re-deriving the cause from public state) would be **two sources of truth for one
/// obligation**, and they would drift on the first new disjunct. The refusal message is not
/// decoration: a filer who cannot see which mechanism stopped them cannot act on it.
///
/// Order is the disjunction's own and matters only to the message, never to the verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerContribution {
    /// Disposals / recognized income / removals dated in the scrub year itself.
    InYearActivity,
    /// The same, in `year - 1`. ★ Deliberate over-refusal — see [`ledger_contributes`]; this
    /// disjunct secures no live read and must not be re-justified with a fresh mechanism.
    PriorYearActivity,
    /// A synthetic default contributes to the projection: the filer's export is DRAFT-watermarked
    /// and attestation-gated, and the recipient's copy would be neither. Projection-wide.
    PseudoActive,
    /// An unresolved `Severity::Hard` blocker anywhere in the projection makes the filer's year
    /// `NotComputable` while the recipient's computes. Projection-wide and year-blind.
    HardBlocker,
}

impl LedgerContribution {
    /// A short phrase naming the mechanism, for the refusal message.
    #[must_use]
    pub fn cause(self) -> &'static str {
        match self {
            Self::InYearActivity => "the ledger records digital-asset activity in that year",
            Self::PriorYearActivity => {
                "the ledger records digital-asset activity in the PRIOR year, which this command \
                 refuses on deliberately rather than prove that nothing reads it"
            }
            Self::PseudoActive => {
                "the ledger is pseudo-reconciled — a synthetic default contributes to the \
                 projection, so your own export carries a DRAFT watermark and an attestation gate \
                 that a scrubbed copy would not reproduce"
            }
            Self::HardBlocker => {
                "the projection carries an unresolved hard blocker, so your year is not computable \
                 while a scrubbed copy of these inputs would compute cleanly"
            }
        }
    }
}

/// See [`LedgerContribution`]. Returns the first firing disjunct, or `None` when the year is
/// scrubbable.
#[must_use]
pub fn ledger_contribution(state: &LedgerState, year: i32) -> Option<LedgerContribution> {
    use crate::tax::return_1040::digital_asset_activity;

    if digital_asset_activity(state, year) {
        Some(LedgerContribution::InYearActivity)
    } else if digital_asset_activity(state, year - 1) {
        Some(LedgerContribution::PriorYearActivity)
    } else if state.pseudo_active() {
        Some(LedgerContribution::PseudoActive)
    } else if state
        .blockers
        .iter()
        .any(|b| b.kind.severity() == crate::state::Severity::Hard)
    {
        Some(LedgerContribution::HardBlocker)
    } else {
        None
    }
}

/// A synthetic SSN that the SSA can never have issued (middle group `00`), distinct per `n`.
fn synthetic_ssn(n: usize) -> String {
    format!("1{:02}-00-{:04}", n % 100, (n % 9999) + 1)
}

/// A synthetic EIN, distinct per `n`. There is no "impossible EIN" — the IRS has issued prefixes
/// across nearly the whole 2-digit space — so this is merely synthetic, not provably unissued.
fn synthetic_ein(n: usize) -> String {
    format!("9{}-{:07}", n % 10, n + 1)
}

/// Remaps EINs so that DISTINCTNESS is preserved and nothing else is.
///
/// ★★★ **KEYED ON `canonical_ein`, NOT ON THE RAW SPELLING — this is r1's CRITICAL.** §3.1 of
/// SPEC_income_scrub.md: where code compares identifiers, scrub must preserve the partition **that
/// comparison** induces, never string equality. §6413(c)'s excess-Social-Security credit turns on
/// having *more than one employer*, and `excess_social_security` decides that by comparing
/// **canonicalized** EINs — so `"11-1111111"` and `"111111111"` are ONE employer.
///
/// Keying the raw string split them into two synthetic EINs that canonicalize apart, manufacturing a
/// **$1,546.80** credit on the repo's own `two_spellings` vector, raising `total_payments` and the
/// refund by the same. An understatement on a §6065 return, inside a file the command called safe to
/// share. Held by `two_spellings_of_one_employer_survive_as_one_employer`.
///
/// ★ A value `canonical_ein` REJECTS (letters, wrong digit count) has no canonical form, so it falls
/// back to the raw string as its own key: two malformed EINs that differ as text stay distinct, and
/// nothing is merged on the strength of a comparison the program never makes. Preserving the
/// malformed *class* into the output is §3.2's job, not this map's.
#[derive(Default)]
struct EinMap(BTreeMap<String, String>);

impl EinMap {
    fn map(&mut self, real: &str) -> String {
        let key = crate::tax::return_1040::canonical_ein(real).unwrap_or_else(|| real.to_string());
        let next = self.0.len();
        self.0
            .entry(key)
            .or_insert_with(|| synthetic_ein(next))
            .clone()
    }
}

fn scrub_person(p: &Person, who: &str, n: usize) -> Person {
    // ★ No `..` — a new `Person` field must be classified here before this compiles.
    let Person {
        first_name: _,
        last_name: _,
        ssn: _,
        date_of_birth,
        date_of_death,
        blind,
        occupation: _,
    } = p;
    Person {
        first_name: who.to_string(),
        last_name: "Example".into(),
        ssn: synthetic_ssn(n),
        // ★★ KEPT. §63(f)'s age-65 and blindness additions read these, and i1040gi's carve-out for
        //    someone who died in-year before reaching 65 reads the death date. Scrubbing them would
        //    move the deduction and break the invariant this module exists to guarantee.
        date_of_birth: *date_of_birth,
        date_of_death: *date_of_death,
        blind: *blind,
        occupation: "Occupation".into(),
    }
}

fn scrub_dependent(d: &Dependent, n: usize) -> Dependent {
    let Dependent {
        name: _,
        ssn: _,
        relationship,
        date_of_birth,
    } = d;
    Dependent {
        name: format!("Dependent{n}"),
        ssn: synthetic_ssn(100 + n),
        // ★ KEPT: relationship decides child-vs-other-dependent, and the DOB decides qualifying-child
        //   age. Both are read; only the name and SSN are not.
        relationship: relationship.clone(),
        date_of_birth: *date_of_birth,
    }
}

fn scrub_header(h: &HouseholdHeader) -> HouseholdHeader {
    let HouseholdHeader {
        taxpayer,
        spouse,
        address_street: _,
        address_city: _,
        address_state: _,
        address_zip: _,
        dependents,
        can_be_claimed_as_dependent_taxpayer,
        can_be_claimed_as_dependent_spouse,
        spouse_had_no_income,
        spouse_not_filing_a_return,
        presidential_fund_taxpayer,
        presidential_fund_spouse,
        taxpayer_died_during_year,
        spouse_died_during_year,
        ip_pin: _,
    } = h;
    HouseholdHeader {
        taxpayer: scrub_person(taxpayer, "Taxpayer", 1),
        spouse: spouse.as_ref().map(|s| scrub_person(s, "Spouse", 2)),
        address_street: "1 Example St".into(),
        address_city: "Springfield".into(),
        // ★ The STATE is replaced too. btctax computes no state tax, so nothing reads it — but a
        //   state plus a filing status plus an income is a long way toward identifying a household.
        address_state: "IL".into(),
        address_zip: "62704".into(),
        dependents: dependents
            .iter()
            .enumerate()
            .map(|(i, d)| scrub_dependent(d, i + 1))
            .collect(),
        // ★★ All KEPT: every one of these is a fail-loud declaration that moves the return.
        can_be_claimed_as_dependent_taxpayer: *can_be_claimed_as_dependent_taxpayer,
        can_be_claimed_as_dependent_spouse: *can_be_claimed_as_dependent_spouse,
        spouse_had_no_income: *spouse_had_no_income,
        spouse_not_filing_a_return: *spouse_not_filing_a_return,
        presidential_fund_taxpayer: *presidential_fund_taxpayer,
        presidential_fund_spouse: *presidential_fund_spouse,
        taxpayer_died_during_year: *taxpayer_died_during_year,
        spouse_died_during_year: *spouse_died_during_year,
        // ★★★ DROPPED, never replaced. The IP PIN is a live anti-fraud credential issued by the IRS;
        //     a synthetic one would be a plausible-looking secret in a file marked safe to share.
        //     Nothing computes from it, so dropping it cannot move a figure.
        ip_pin: None,
    }
}

/// Replace each name in a delimited free-text list, preserving how many there are.
///
/// ★ Schedule B line 7b prints the COUNTRIES a filer holds foreign accounts in — which is free text
/// and is identifying (it is a fact about a specific person's offshore banking). The count and the
/// delimiter shape are kept because line 7b prints them and a refusal fires when the list is empty
/// while `foreign_accounts` is `Some(true)`; the names themselves carry nothing a reproducer needs.
fn scrub_name_list(s: &str) -> String {
    if s.trim().is_empty() {
        return String::new();
    }
    s.split(',')
        .enumerate()
        .map(|(i, _)| format!("Country{}", i + 1))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Scrub the identity out of `ri`, preserving every figure and every computation-bearing field.
///
/// See the module docs for the guarantee and for why EIN sameness is preserved.
#[must_use]
pub fn scrub_pii(ri: &ReturnInputs) -> ReturnInputs {
    // ★★★ THE COMPLETENESS WITNESS. No `..`, so a new `ReturnInputs` field is *pattern does not
    //     mention field* HERE and someone must decide whether it can carry identity before this
    //     compiles.
    //
    //     This was missing in the first version, and a background security review caught it. The
    //     module doc claimed "every struct touched here is taken apart with no `..`" — true of
    //     `Person`, `Dependent` and `HouseholdHeader`, and FALSE of the top level, which was a
    //     `clone()` plus targeted mutation. So `schedule_c.business_description` (free text: a real
    //     filer writes their business NAME there) and `foreign_country_names` (which countries this
    //     person banks in) both rode straight through into a file the tool told the user was safe to
    //     share. The guarantee is worthless exactly where new input types get added, which is the
    //     top level. Same lesson as `a-figure-with-no-reader`: the doc asserted a property nothing
    //     enforced.
    //
    //     `_` = carries no identity: a figure, a flag, a provenance tag, or an enum.
    let ReturnInputs {
        tax_year: _,
        filing_status: _,
        header,
        w2s: _,
        int_1099: _,
        div_1099: _,
        g_1099: _,
        b_1099: _,
        schedule_c,    // ★ business_description is FREE TEXT — scrubbed below
        schedule_a: _, // money only
        itemize_election: _,
        mfs_spouse_itemizes: _,
        sch1: _, // money only
        payments: _,
        capital_loss_carryforward_in: _,
        capital_loss_carryforward_in_provenance: _,
        amt_carryover_same_as_regular: _,
        amt_depreciation_same_as_regular: _,
        charitable_carryover_in: _, // money + year only; no donee identity here
        charitable_carryover_in_provenance: _,
        qbi: _, // money only
        foreign_accounts: _,
        foreign_trust: _,
        foreign_country_names, // ★ FREE TEXT and identifying — scrubbed below
        fbar_filing_required: _,
        donations_had_restrictions: _,
        dual_status_alien: _,
        has_income_exclusion: _,
        other_out_of_scope_income: _,
        excluded_puerto_rico_income: _,
        form_2555_line45: _,
        form_2555_line50: _,
        form_4563_line15: _,
    } = ri;

    let mut out = ri.clone();
    out.header = scrub_header(header);
    out.foreign_country_names = scrub_name_list(foreign_country_names);

    if let (Some(sc_out), Some(_sc)) = (out.schedule_c.as_mut(), schedule_c.as_ref()) {
        // ★★ FREE TEXT, and the one field here a filer fills in their own words. "Smith Family
        //    Dental" is a perfectly ordinary answer. It must stay NON-EMPTY: an empty description
        //    refuses the return (`ScheduleCNoBusinessDescription`), so scrubbing it to "" would make
        //    the scrubbed copy refuse where the original filed — breaking the reproduce-it guarantee
        //    in the other direction.
        sc_out.business_description = "Example business".into();
        // ★ `naics_code` is KEPT: a six-digit federal industry taxonomy shared by thousands of
        //   businesses is not a personal identifier, and it is printed on Schedule C line B, so a
        //   reproducer wants the real one. Recorded as a decision rather than an oversight.
    }

    // ★★ W-2 employers: the NAME is free to replace, the EIN is not — only its sameness matters.
    let mut eins = EinMap::default();
    for (i, w) in out.w2s.iter_mut().enumerate() {
        w.employer = format!("Employer{}", i + 1);
        if let Some(e) = w.ein.as_ref().filter(|e| !e.is_empty()) {
            w.ein = Some(eins.map(e));
        }
    }
    for (i, f) in out.int_1099.iter_mut().enumerate() {
        f.payer = format!("Payer{}", i + 1);
    }
    for (i, f) in out.div_1099.iter_mut().enumerate() {
        f.payer = format!("Payer{}", i + 1);
    }
    for (i, f) in out.b_1099.iter_mut().enumerate() {
        f.payer = format!("Broker{}", i + 1);
    }
    for (i, f) in out.g_1099.iter_mut().enumerate() {
        f.payer = format!("Agency{}", i + 1);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::return_1040::assemble_absolute;
    use crate::tax::testonly::{
        kitchen_sink_household, ty2024_params, ty2024_table, w2_only_household,
    };

    /// ★★★ r1's CRITICAL, PINNED: **one employer spelled two ways must stay ONE employer.**
    ///
    /// §6413(c)'s excess-social-security credit turns on having *more than one employer*, and
    /// `excess_social_security` decides identity by comparing **canonicalized** EINs
    /// (`return_1040.rs::canonical_ein`). A W-2 and its W-2c — box b typed off the paper form on one
    /// and off a payroll-portal export on the other — are `"11-1111111"` and `"111111111"`: the SAME
    /// employer, and the repo pins that at `return_1040.rs`'s `two_spellings` vector, where the
    /// correct credit is **$0**.
    ///
    /// The held scrubber keyed `EinMap` on the RAW STRING, so the two spellings became two distinct
    /// synthetic EINs, which canonicalize apart — **two employers** — and manufacture a
    /// **$1,546.80** credit the filer is not entitled to. `total_payments` rises by it and so does
    /// the refund. That is an UNDERSTATEMENT on a §6065 return, reached through a file the command
    /// told the filer was safe to share, and it is the exact failure the module's own EIN paragraph
    /// says it exists to prevent.
    ///
    /// ★ Neither existing test could see it: the three fixtures in
    /// `scrub_preserves_every_computed_figure` carry no EIN at all, and
    /// `ein_distinctness_is_preserved_exactly` uses two IDENTICAL spellings.
    /// One employer, two standard spellings of its EIN, each W-2 withholding over half the §3101(a)
    /// cap. The correct §6413(c) credit is **$0** and the over-withholding is *stranded* — it appears
    /// in `AbsoluteReturn::excess_ss_not_creditable`, which is what makes this vector exercise BOTH
    /// halves of §8 step 3.
    fn two_spellings_household() -> (ReturnInputs, crate::state::LedgerState) {
        use crate::tax::return_inputs::W2;
        let (base, state) = w2_only_household();
        let w = base.w2s[0].clone();
        let ri = ReturnInputs {
            w2s: vec![
                W2 {
                    ein: Some("11-1111111".into()),
                    box3_ss_wages: rust_decimal_macros::dec!(6000),
                    box4_ss_withheld: rust_decimal_macros::dec!(6000),
                    ..w.clone()
                },
                W2 {
                    ein: Some("111111111".into()), // the SAME employer, the other standard spelling
                    box3_ss_wages: rust_decimal_macros::dec!(6000),
                    box4_ss_withheld: rust_decimal_macros::dec!(6000),
                    ..w
                },
            ],
            ..base
        };
        (ri, state)
    }

    #[test]
    fn two_spellings_of_one_employer_survive_as_one_employer() {
        let (ri, state) = two_spellings_household();
        let scrubbed = scrub_pii(&ri);

        // The partition the PROGRAM'S OWN comparison induces must survive — never string equality.
        let canon = |r: &ReturnInputs| -> Vec<Option<String>> {
            r.w2s
                .iter()
                .map(|w| {
                    w.ein
                        .as_deref()
                        .and_then(crate::tax::return_1040::canonical_ein)
                })
                .collect()
        };
        let c = canon(&scrubbed);
        assert_eq!(
            c[0], c[1],
            "two spellings of ONE employer must canonicalize to one employer after scrubbing, or \
             §6413(c) manufactures an excess-Social-Security credit"
        );
        for (o, n) in ri.w2s.iter().zip(&scrubbed.w2s) {
            assert_ne!(o.ein, n.ein, "the real EIN must not survive");
        }

        // …and the figure itself, which is what the filer would actually be handed.
        let a = assemble_absolute(&ri, &state, &ty2024_params(), &ty2024_table(), 2024);
        let b = assemble_absolute(&scrubbed, &state, &ty2024_params(), &ty2024_table(), 2024);
        assert_eq!(
            a.excess_social_security, b.excess_social_security,
            "scrubbing moved the §6413(c) credit"
        );
        assert_eq!(
            a.total_payments, b.total_payments,
            "scrubbing moved total payments"
        );
    }

    /// ★★★ THE GUARANTEE: scrubbing changes NO computed figure, on any household.
    ///
    /// Asserted over the whole `AbsoluteReturn` rather than a sampled line, because the failure this
    /// guards is a field nobody thought to check. A scrubbed return that computes differently sends
    /// its recipient after a bug that is not there — worse than not sharing it at all.
    #[test]
    fn scrub_preserves_every_computed_figure() {
        for (name, (ri, state)) in [
            ("kitchen sink", kitchen_sink_household()),
            ("W-2 only", w2_only_household()),
            ("AMT-owing", crate::tax::testonly::amt_owing_household()),
            // ★ §8 step 3: the ONLY vector here carrying an EIN at all, and the only one that
            //   populates `excess_ss_not_creditable` — so it is what makes the `ein` normalization
            //   below load-bearing instead of speculative.
            ("two spellings, one employer", two_spellings_household()),
        ] {
            let scrubbed = scrub_pii(&ri);
            let mut a = assemble_absolute(&ri, &state, &ty2024_params(), &ty2024_table(), 2024);
            let mut b =
                assemble_absolute(&scrubbed, &state, &ty2024_params(), &ty2024_table(), 2024);

            // ★★★ ONE normalisation, and it is named rather than blanket. `business_description` is
            //     the ONLY identity-bearing string that survives into `AbsoluteReturn` (Schedule C
            //     line A, carried for printing; every other field there is a figure, a flag or an
            //     enum). Scrubbing it is the POINT — a filer writes their business name in it — so it
            //     is blanked on BOTH sides and everything else must still be equal.
            //
            //     ★★ It lands in TWO places, which is why this is a helper and not one line: Schedule
            //        C line A, and Form 8995-A line 1(a)'s trade-or-business name. An identity string
            //        PROPAGATES, so the set of places to normalise grows as the forms grow — and the
            //        growth is safe precisely because forgetting one REDS THIS TEST rather than
            //        silently weakening it.
            //
            //     ★ This is deliberately not a `..`-style exclusion list. If a future field makes the
            //       two sides differ, this test reds and the author must decide whether that field is
            //       identity (blank it here, and scrub it) or a figure (a real bug). The opposite
            //       direction — a string that should have been scrubbed but was not — is held by
            //       `the_identity_does_not_survive`.
            //     ★★★ THE THIRD MEMBER IS `excess_ss_not_creditable[].ein`, and without it this test
            //         REDS ON A CORRECT SCRUB (§3.3). That advisory names the employer to go and ask,
            //         canonicalized — so it carries a scrubbed identity string into `AbsoluteReturn`,
            //         and the two sides differ by `"111111111"` vs `"900000001"` while every figure
            //         matches. The **re-sort by `(owner, amount)`** goes with it: once the eins are
            //         blanked, two entries that differed only by employer are otherwise equal, and
            //         `non_creditable_ss` buckets by canonical EIN — so their ORDER is a function of
            //         identity strings that scrubbing legitimately changes. Comparing unsorted would
            //         make this test fail on employer count, not on any figure.
            //
            //     ★ The normalization set is stated HERE, in full, and never inherited: Schedule C
            //       line A, Form 8995-A line 1(a), and this. Three members, each with a reason.
            let blank_identity = |r: &mut crate::tax::return_1040::AbsoluteReturn| {
                r.printed_inputs.schedule_c_header.business_description = String::new();
                if let Some(p) = r.f8995a_parts_i_to_iii.as_mut() {
                    p.part_i.col_a_name = String::new();
                }
                for nc in &mut r.excess_ss_not_creditable {
                    nc.ein = String::new();
                }
                // ★ Keyed on the Debug rendering of `owner` rather than deriving `Ord` on it: this is
                //   a test-local normalization and it must not reach back into a production type to
                //   get one.
                r.excess_ss_not_creditable.sort_by(|x, y| {
                    (format!("{:?}", x.owner), x.amount).cmp(&(format!("{:?}", y.owner), y.amount))
                });
            };
            blank_identity(&mut a);
            blank_identity(&mut b);
            assert_eq!(a, b, "{name}: scrubbing moved a computed figure");
        }
    }

    /// ★★★ EIN SAMENESS SURVIVES — the §6413(c) excess-SS credit turns on "more than one employer",
    /// so collapsing two employers into one (or splitting one into two) manufactures or destroys a
    /// credit. This is the property the invariant above cannot see on a single-W-2 fixture.
    #[test]
    fn ein_distinctness_is_preserved_exactly() {
        let (mut ri, _) = kitchen_sink_household();
        let w = ri.w2s[0].clone();
        ri.w2s = vec![
            crate::tax::return_inputs::W2 {
                ein: Some("11-1111111".into()),
                ..w.clone()
            },
            crate::tax::return_inputs::W2 {
                ein: Some("11-1111111".into()), // SAME employer
                ..w.clone()
            },
            crate::tax::return_inputs::W2 {
                ein: Some("22-2222222".into()), // a DIFFERENT one
                ..w
            },
        ];
        let s = scrub_pii(&ri);
        let e: Vec<Option<String>> = s.w2s.iter().map(|w| w.ein.clone()).collect();
        assert_eq!(
            e[0], e[1],
            "two W-2s from ONE employer must stay one employer"
        );
        assert_ne!(e[1], e[2], "a second employer must stay a second employer");
        for (orig, new) in ri.w2s.iter().zip(&s.w2s) {
            assert_ne!(orig.ein, new.ein, "the real EIN must not survive");
        }
    }

    /// ★★ The identity is actually gone — a scrubber that quietly kept a field would still pass the
    /// invariant above, because names move no figure. Both halves are needed.
    #[test]
    fn the_identity_does_not_survive() {
        let (mut ri, _) = kitchen_sink_household();
        ri.header.ip_pin = Some("123456".into());
        let s = scrub_pii(&ri);
        assert_ne!(s.header.taxpayer.ssn, ri.header.taxpayer.ssn);
        assert_ne!(s.header.taxpayer.last_name, ri.header.taxpayer.last_name);
        assert_ne!(s.header.address_street, ri.header.address_street);
        assert_eq!(
            s.header.ip_pin, None,
            "an IP PIN is a live credential: DROPPED"
        );
        for (o, n) in ri.header.dependents.iter().zip(&s.header.dependents) {
            assert_ne!(o.ssn, n.ssn);
            assert_eq!(
                o.relationship, n.relationship,
                "relationship is computational"
            );
        }
        // ★★ …and the two fields a security review found riding through the FIRST version of this
        //    module: free text where a filer writes a business name, and the countries they bank in.
        let mut ri2 = ri.clone();
        if let Some(c) = ri2.schedule_c.as_mut() {
            c.business_description = "Smith Family Dental".into();
        }
        ri2.foreign_accounts = Some(true);
        ri2.foreign_country_names = "Switzerland, Liechtenstein".into();
        let s2 = scrub_pii(&ri2);
        assert_eq!(
            s2.schedule_c
                .as_ref()
                .map(|c| c.business_description.as_str()),
            Some("Example business"),
            "a business NAME is free text and must not survive"
        );
        assert!(
            !s2.foreign_country_names.contains("Switzerland")
                && !s2.foreign_country_names.contains("Liechtenstein"),
            "which countries someone banks in is identifying: {}",
            s2.foreign_country_names
        );
        assert_eq!(
            s2.foreign_country_names.split(',').count(),
            2,
            "the COUNT is kept — Schedule B line 7b prints the list and an empty one refuses"
        );

        // ★ …and the SSNs it emits are structurally impossible, so the output is safe to commit.
        assert!(
            s.header.taxpayer.ssn.contains("-00-"),
            "middle group 00 is never issued: {}",
            s.header.taxpayer.ssn
        );
    }
}
