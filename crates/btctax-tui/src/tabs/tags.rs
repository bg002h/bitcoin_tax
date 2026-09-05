//! Tag helpers for human-readable labels in the viewer tabs.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

use btctax_core::event::{BasisSource, IncomeKind};
use btctax_core::project::ComplianceStatus;
use btctax_core::state::Term;

pub(super) fn term_tag(term: Term) -> &'static str {
    match term {
        Term::ShortTerm => "short",
        Term::LongTerm => "long",
    }
}

// ── Sort ranks — map an enum to its declaration-order index so a column can be sorted by
//    "enum order" (short<long, and each enum by the order its variants are declared). No RNG;
//    these are the ONLY orderings sort_views relies on for the enum columns.

/// Term sort rank: short (0) < long (1).
pub(super) fn term_rank(term: Term) -> u8 {
    match term {
        Term::ShortTerm => 0,
        Term::LongTerm => 1,
    }
}

/// `BasisSource` sort rank = its declaration order in `event.rs`.
pub(super) fn basis_source_rank(src: BasisSource) -> u8 {
    match src {
        BasisSource::ExchangeProvided => 0,
        BasisSource::ComputedFromCost => 1,
        BasisSource::FmvAtIncome => 2,
        BasisSource::CarriedFromTransfer => 3,
        BasisSource::GiftCarryover => 4,
        BasisSource::GiftFmvFallback => 5,
        BasisSource::SafeHarborAllocated => 6,
        BasisSource::ReconstructedPerWallet => 7,
        BasisSource::SelfTransferInbound => 8,
        BasisSource::EstimatedConservative => 9,
        // FR-45 — declaration order, so it sorts last as the newest variant.
        BasisSource::CardRewardRebate => 10,
    }
}

/// `IncomeKind` sort rank = its declaration order in `event.rs`.
pub(super) fn income_kind_rank(kind: IncomeKind) -> u8 {
    match kind {
        IncomeKind::Mining => 0,
        IncomeKind::Staking => 1,
        IncomeKind::Interest => 2,
        IncomeKind::Airdrop => 3,
        IncomeKind::Reward => 4,
    }
}

pub(super) fn basis_source_tag(src: BasisSource) -> &'static str {
    match src {
        BasisSource::ExchangeProvided => "exchange",
        BasisSource::ComputedFromCost => "cost",
        BasisSource::FmvAtIncome => "income_fmv",
        BasisSource::CarriedFromTransfer => "transferred",
        BasisSource::GiftCarryover => "gift_carryover",
        BasisSource::GiftFmvFallback => "gift_fmv_fallback",
        BasisSource::SafeHarborAllocated => "safe_harbor",
        BasisSource::ReconstructedPerWallet => "reconstructed",
        BasisSource::SelfTransferInbound => "self_transfer_in",
        BasisSource::EstimatedConservative => "estimated_conservative",
        // FR-45 — must MATCH the CLI tag byte-for-byte; a divergence here is a UI/CSV split.
        BasisSource::CardRewardRebate => "card_reward_rebate",
    }
}

pub(super) fn income_kind_tag(kind: IncomeKind) -> &'static str {
    match kind {
        IncomeKind::Mining => "mining",
        IncomeKind::Staking => "staking",
        IncomeKind::Interest => "interest",
        IncomeKind::Airdrop => "airdrop",
        IncomeKind::Reward => "reward",
    }
}

/// Stable per-disposal compliance status string (re-implemented locally — btctax-cli's version is
/// private). Matches the CLI's `compliance_status_tag` output exactly.
///
/// - `standing_order:<date>` — in-force standing order effective from `<date>`.
/// - `contemporaneous`       — `LotSelection` recorded no later than the date AND TIME of the sale
///   (§1.1012-1(j)(2)). ★ This line said "on or before the DAY of sale" until I-1; the code
///   applied that standard at only one of the rule's three sites, and a selection made at 17:00
///   on the day of a 10:00 sale is not one — the fold rejects it.
/// - `attested_recording`    — Mode-1-persisted selection backed by contemporaneous-ID attestation.
/// - `non_compliant`         — no adequate identification.
pub(super) fn compliance_status_tag(cs: &ComplianceStatus) -> String {
    match cs {
        ComplianceStatus::StandingOrder { effective_from } => {
            format!("standing_order:{effective_from}")
        }
        ComplianceStatus::Contemporaneous => "contemporaneous".into(),
        ComplianceStatus::AttestedRecording => "attested_recording".into(),
        ComplianceStatus::NonCompliant => "non_compliant".into(),
    }
}

#[cfg(test)]
mod fr45_parity_tests {
    use super::basis_source_tag;
    use btctax_core::BasisSource;

    /// ★ **FR-45 M-3.** `basis_source_tag` in this module carried a comment claiming byte parity
    /// with the CLI's tag — a guarantee no test held, which by this repo's own rule means it did
    /// not exist. The CLI's function IS the CSV contract; this one is display. A silent divergence
    /// is a UI/CSV split, where the screen and the exported file disagree about what a lot is.
    ///
    /// The list is derived from an exhaustive `match`, so a new variant cannot be omitted: adding
    /// one reds this test with `E0004` rather than passing while unchecked.
    #[test]
    fn every_basis_source_tag_matches_the_cli_csv_contract_byte_for_byte() {
        const ALL: [BasisSource; 11] = [
            BasisSource::ExchangeProvided,
            BasisSource::ComputedFromCost,
            BasisSource::FmvAtIncome,
            BasisSource::CarriedFromTransfer,
            BasisSource::GiftCarryover,
            BasisSource::GiftFmvFallback,
            BasisSource::SafeHarborAllocated,
            BasisSource::ReconstructedPerWallet,
            BasisSource::SelfTransferInbound,
            BasisSource::EstimatedConservative,
            BasisSource::CardRewardRebate,
        ];
        // Exhaustive by construction: this match cannot compile while ignoring a new variant, so
        // ALL above cannot silently fall behind the enum.
        fn assert_covered(bs: BasisSource) {
            match bs {
                BasisSource::ExchangeProvided
                | BasisSource::ComputedFromCost
                | BasisSource::FmvAtIncome
                | BasisSource::CarriedFromTransfer
                | BasisSource::GiftCarryover
                | BasisSource::GiftFmvFallback
                | BasisSource::SafeHarborAllocated
                | BasisSource::ReconstructedPerWallet
                | BasisSource::SelfTransferInbound
                | BasisSource::EstimatedConservative
                | BasisSource::CardRewardRebate => {}
            }
        }
        for bs in ALL {
            assert_covered(bs);
            assert_eq!(
                basis_source_tag(bs),
                btctax_cli::render::basis_source_tag(bs),
                "TUI display tag and CLI CSV tag diverged for {bs:?} — the screen and the exported \
                 file would disagree about what this lot is"
            );
        }
    }
}
