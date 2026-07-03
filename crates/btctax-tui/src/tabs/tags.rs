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
