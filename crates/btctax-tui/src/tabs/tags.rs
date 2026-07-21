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
/// - `contemporaneous`       — `LotSelection` recorded on or before the day of sale.
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
