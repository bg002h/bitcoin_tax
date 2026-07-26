//! Approach-B / Task 10 — the `promote-tranche` CLI verb: BG-D5 (record-time purchase-provenance
//! attestation), BG-D6 (two-sided informed-consent recording), and BG-D7 (Form 8275 Part II present-by-
//! construction).
//!
//! Defensive Filing Wizard Task 1: the actual plan/confirm/apply PIPELINE now lives in
//! `crate::chokepoint` (`plan_promote`/`render_consent`/`apply_promote`) — a reusable chokepoint a future
//! TUI can drive identically. This module is a THIN DRIVER over it: `Session::open` → build args →
//! `plan_promote` (mapping a `Refusal` to a `CliError`) → print the consent screen → prompt/collect the
//! acknowledgment → `apply_promote`. It also still owns the copy constants (`PROMOTE_ACK_PHRASE`,
//! `PROVENANCE_TEXT`/`PROVENANCE_VERSION`, `CONSENT_INTRO`) and the shipped `render_consent(terms,
//! gift_only_years)` two-arg renderer (kept `pub` — `tests/promote_cli.rs` calls it directly, and
//! `crate::chokepoint::render_consent(&plan)` calls it internally to reproduce the shipped byte order).
use crate::{CliError, Session};
use btctax_core::event::ConsentTerm;
use btctax_core::Usd;
use btctax_store::Passphrase;
use std::collections::BTreeSet;
use std::path::Path;
use time::OffsetDateTime;

use crate::eventref::parse_event_id;

/// The units' real acquisition provenance (BG-D5). A CLOSED enumeration: only `Purchase` clears the
/// promote gate — every other value has a documented FMV-at-receipt/carryover basis from the return
/// (Notice 2014-21; Rev. Rul. 2019-24) and is refused, pointed at modeling the real acquisition instead.
/// CLI-facing only (not part of the persisted schema — only `provenance_attested: bool` plus the fixed
/// `PROVENANCE_TEXT`/`PROVENANCE_VERSION` are stored on the event, per T1's `Acknowledgment`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ProvenanceKind {
    Purchase,
    Gift,
    Inheritance,
    Mining,
    Earned,
    Airdrop,
    Fork,
}

impl ProvenanceKind {
    /// The CLOSED enumeration, in `--provenance` declaration order — the SINGLE list a driver offers the
    /// filer to choose from. Added for the TUI Promote flow's BG-D5 provenance step (P-C gate tax I-2):
    /// the wizard must make the filer SELECT their acquisition provenance exactly as `--provenance` makes
    /// the CLI filer select it, and `btctax-tui-edit` does not depend on `clap`, so it cannot reach
    /// `ValueEnum::value_variants()`. Kept in lock-step with the enum by
    /// `tests::provenance_all_covers_every_clap_value_variant`.
    ///
    /// ★ A **slice**, not a fixed-length array (whole-branch arch Nit-N1, narrowed before the first
    /// publish): `[ProvenanceKind; 7]` bakes the variant COUNT into the public type, so adding a
    /// provenance kind would be a breaking change for every downstream matcher. As a slice that addition
    /// is purely additive.
    pub const ALL: &'static [ProvenanceKind] = &[
        ProvenanceKind::Purchase,
        ProvenanceKind::Gift,
        ProvenanceKind::Inheritance,
        ProvenanceKind::Mining,
        ProvenanceKind::Earned,
        ProvenanceKind::Airdrop,
        ProvenanceKind::Fork,
    ];

    /// The filer-facing label for one variant. `pub` (widened from `pub(crate)` for the TUI Promote
    /// flow's BG-D5 step): the chokepoint (`crate::chokepoint::refuse_non_purchase`) builds its refusal
    /// copy from this, so a driver offering the enumeration renders the SAME words the refusal names —
    /// no second copy of the enumeration's wording.
    pub fn label(self) -> &'static str {
        match self {
            ProvenanceKind::Purchase => "purchase",
            ProvenanceKind::Gift => "gift",
            ProvenanceKind::Inheritance => "inheritance",
            ProvenanceKind::Mining => "mining",
            ProvenanceKind::Earned => "staking/earning",
            ProvenanceKind::Airdrop => "airdrop",
            ProvenanceKind::Fork => "fork",
        }
    }
}

/// The exact phrase a filer must affirm to RECORD a `promote-tranche` decision (BG-D6). Distinct from the
/// pseudo-export `ATTEST_PHRASE` (`lib.rs`) — promoting is an estimated-basis FILING CHOICE, not an
/// attestation that a fictional draft is being exported on purpose; conflating the two phrases would let a
/// scripted pseudo-export attest ALSO silently satisfy this gate (N-1). Compared TRIMMED, case-SENSITIVE
/// (mirrors `require_attestation`'s compare, `lib.rs:208`).
pub const PROMOTE_ACK_PHRASE: &str = "I understand and accept this estimated-basis risk";

/// BG-D5 attested statement (verbatim, stored on `Acknowledgment.provenance_text`). The affirmative clause
/// is operative; the negative enumeration is CLOSED so it cannot be misread `expressio unius` (tax r1
/// M-6).
pub const PROVENANCE_TEXT: &str = "these units were acquired by purchase within the declared window — \
     not by gift, inheritance, mining, staking/earning, airdrop, fork, or any acquisition other than \
     purchase";
/// Attestation-text version (BG-D5) — bump if `PROVENANCE_TEXT`'s wording ever changes.
pub const PROVENANCE_VERSION: &str = "v1";

/// The consent-screen intro (BG-D6/D10): the penalty base, "plus interest", and the mitigation framing —
/// NEVER "safe harbor" (SPEC BG-D7/D10 — the copy must not use that phrase even to deny it).
const CONSENT_INTRO: &str = "Promoting this tranche is a KNOWING choice to file a >$0 basis floor \
     (the minimum daily closing price over the attested acquisition window) instead of the IRS-fallback \
     $0. If an exam determines the correct basis is $0, the penalty is 20% ordinary / 40% worst-case of \
     the resulting additional tax (the underpayment attributable to the misstatement), plus interest; \
     the Form 8275 disclosure and the good-faith window-low-close methodology mitigate this exposure, \
     but do not eliminate it and do not guarantee immunity from penalty.";

/// Render one `ConsentTerm` line for the consent screen (BG-D6/D10 copy).
fn render_term(term: &ConsentTerm, gift_only_years: &BTreeSet<i32>) -> String {
    match term {
        ConsentTerm::ComputedTax {
            year,
            delta_usd,
            deduction_delta_usd,
        } => {
            let mut line = if *delta_usd >= Usd::ZERO {
                format!(
                    "Year {year}: promoting this tranche SAVES ~${} in computed federal tax.",
                    delta_usd.round_dp(2)
                )
            } else {
                format!(
                    "Year {year}: promoting this tranche INCREASES computed federal tax by ~${}.",
                    (-*delta_usd).round_dp(2)
                )
            };
            if let Some(d) = deduction_delta_usd {
                line.push_str(&format!(
                    " The computed tax figure does NOT capture this charitable-deduction change (priced \
                     only on the full return); its own Δ is ~${}.",
                    d.round_dp(2)
                ));
                if gift_only_years.contains(year) {
                    line.push_str(
                        " That Δ is a donee-basis (§1015) documentation change; the donor's 1040 is \
                         unaffected — NOT a Schedule-A deduction.",
                    );
                }
            }
            line
        }
        ConsentTerm::Uncomputable {
            year,
            gain_delta_usd,
            deduction_delta_usd,
        } => {
            let mut line = format!(
                "Year {year}: tax not computable here (no table/profile/blocked) — promoting changes the \
                 reported gain by ~${} and the deduction/basis by ~${}.",
                gain_delta_usd.round_dp(2),
                deduction_delta_usd.round_dp(2)
            );
            if *deduction_delta_usd != Usd::ZERO && gift_only_years.contains(year) {
                line.push_str(
                    " The deduction/basis figure is a donee-basis (§1015) documentation change; the \
                     donor's 1040 is unaffected — NOT a Schedule-A deduction.",
                );
            }
            line
        }
        ConsentTerm::CascadeNamed { year } => format!(
            "Year {year}: this promote's cross-year effects may also shift that year's §1212(b)/§170(d) \
             carryover-in (named here, not separately quantified)."
        ),
        ConsentTerm::Unrealized {
            sat,
            hypothetical_reduction,
            as_of,
        } => match (hypothetical_reduction, as_of) {
            (Some(r), Some(d)) => format!(
                "{sat} sat remain undisposed: at the {d} close, promoting would reduce a future sale's \
                 reported gain by up to ~${} (hypothetical, not a filed figure) — saving and exposure \
                 accrue only at disposal.",
                r.round_dp(2)
            ),
            _ => format!(
                "{sat} sat remain undisposed: no current bundled price is available — the filed floor \
                 itself is the maximum possible gain reduction on a future sale (hypothetical, not a \
                 filed figure) — saving and exposure accrue only at disposal."
            ),
        },
    }
}

/// The consent screen (BG-D6/D10): the penalty-base/interest/mitigation intro, then one line per
/// `ConsentTerm`. `gift_only_years` (T9 handoff) relabels a gift-only flagged year's deduction/basis-Δ as
/// a §1015 donee-basis change rather than a Schedule-A deduction. Kept `pub` (not `pub(crate)`): external
/// KATs in `tests/promote_cli.rs` call this two-arg renderer directly, independent of the full
/// `chokepoint::PromotePlan` pipeline; `chokepoint::render_consent(&plan)` also calls this internally to
/// reproduce the shipped `promote.rs:443-455` byte order (I-1).
pub fn render_consent(terms: &[ConsentTerm], gift_only_years: &BTreeSet<i32>) -> String {
    let mut out = String::new();
    out.push_str(CONSENT_INTRO);
    for term in terms {
        out.push('\n');
        out.push_str(&render_term(term, gift_only_years));
    }
    out
}

/// Append a `PromoteTranche` decision (BG-D1) promoting `target_ref`'s `$0` `DeclareTranche` to a filed
/// `>$0` basis floor, behind the BG-D5 provenance gate, the BG-D7 Part II narrative gate, and the BG-D6
/// informed-consent acknowledgment. `now` is the injected decision creation-time (deterministic in
/// tests) — it ALSO doubles as the clock-free "current tax year" the T8 prior-year advisory is filtered
/// against (years `>= current` are still being authored, not yet filed, so no 1040-X pointer is owed).
///
/// A thin driver over `crate::chokepoint`: `Session::open` → build args → `plan_promote` → print the
/// consent screen → `apply_promote`. No pipeline logic remains here (Task 1).
pub fn promote_tranche(
    vault_path: &Path,
    pp: &Passphrase,
    target_ref: &str,
    provenance: ProvenanceKind,
    part_ii: String,
    acknowledge: Option<&str>,
    now: OffsetDateTime,
) -> Result<btctax_core::EventId, CliError> {
    let target_event_id = parse_event_id(target_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let events = btctax_core::persistence::load_all(session.conn())?;
    let cfg = session.config()?.to_projection();

    let plan = crate::chokepoint::plan_promote(
        &events,
        session.prices(),
        &cfg,
        &target_event_id,
        provenance,
        &part_ii,
        now,
    )
    .map_err(CliError::from)?;

    println!("{}", crate::chokepoint::render_consent(&plan));

    crate::chokepoint::apply_promote(&mut session, plan, acknowledge, now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::ValueEnum;

    /// Drift guard for `ProvenanceKind::ALL` (added for the TUI's BG-D5 provenance step): the array a
    /// driver offers the filer MUST stay the SAME closed enumeration `--provenance` accepts. Adding a
    /// variant without extending `ALL` would silently make it un-offerable in the wizard while the CLI
    /// still accepts it.
    #[test]
    fn provenance_all_covers_every_clap_value_variant() {
        let variants = ProvenanceKind::value_variants();
        assert_eq!(
            ProvenanceKind::ALL.len(),
            variants.len(),
            "ProvenanceKind::ALL must list EVERY variant (the closed BG-D5 enumeration): {:?} vs {:?}",
            ProvenanceKind::ALL,
            variants
        );
        for v in variants {
            assert!(
                ProvenanceKind::ALL.contains(v),
                "ProvenanceKind::ALL is missing {v:?} — a driver could never offer it to the filer"
            );
        }
        assert_eq!(
            ProvenanceKind::ALL[0],
            ProvenanceKind::Purchase,
            "Purchase (the only gate-passing value) stays first so a driver's own ordering is stable"
        );
    }

    /// Every variant has a distinct, non-empty filer-facing label (the refusal copy names it).
    #[test]
    fn every_provenance_label_is_distinct_and_non_empty() {
        let mut seen = std::collections::BTreeSet::new();
        for k in ProvenanceKind::ALL {
            let l = k.label();
            assert!(!l.trim().is_empty(), "{k:?} has an empty label");
            assert!(seen.insert(l), "duplicate provenance label: {l}");
        }
    }
}
