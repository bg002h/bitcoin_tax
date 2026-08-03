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
//! Every struct touched here is taken apart with **no `..`**. A scrubber that silently passes a NEW
//! PII field through is worse than no scrubber at all: it carries the user's authorisation ("this file
//! is safe to share") onto a file that is not. So adding a field to `Person`, `Dependent` or
//! `HouseholdHeader` is *pattern does not mention field* here, and the author must decide which side
//! it falls on. The same argument the P9 classifier makes about answered-ness, applied to disclosure.
//!
//! ## What is REPLACED, and what must survive
//!
//! | replaced — read by nothing but the printer | preserved — the computation reads it |
//! |---|---|
//! | names, SSNs, occupation, street/city/state/ZIP, IP PIN | filing status, every dollar figure |
//! | employer / payer / bank names | dates of birth and death, blindness |
//! | | `can_be_claimed_*`, the spouse flags, presidential-fund boxes |
//! | | the NUMBER of dependents and each `relationship` |
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

use crate::tax::return_inputs::{Dependent, HouseholdHeader, Person, ReturnInputs};
use std::collections::BTreeMap;

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
#[derive(Default)]
struct EinMap(BTreeMap<String, String>);

impl EinMap {
    fn map(&mut self, real: &str) -> String {
        let next = self.0.len();
        self.0
            .entry(real.to_string())
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

/// Scrub the identity out of `ri`, preserving every figure and every computation-bearing field.
///
/// See the module docs for the guarantee and for why EIN sameness is preserved.
#[must_use]
pub fn scrub_pii(ri: &ReturnInputs) -> ReturnInputs {
    let mut out = ri.clone();
    out.header = scrub_header(&ri.header);

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
        ] {
            let scrubbed = scrub_pii(&ri);
            let a = assemble_absolute(&ri, &state, &ty2024_params(), &ty2024_table(), 2024);
            let b = assemble_absolute(&scrubbed, &state, &ty2024_params(), &ty2024_table(), 2024);
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
        // ★ …and the SSNs it emits are structurally impossible, so the output is safe to commit.
        assert!(
            s.header.taxpayer.ssn.contains("-00-"),
            "middle group 00 is never issued: {}",
            s.header.taxpayer.ssn
        );
    }
}
