//! Tag helpers for human-readable labels in the viewer tabs.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

use btctax_core::event::{BasisSource, DisposeKind, IncomeKind};
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

#[allow(dead_code)] // used in Task 4 (disposals kind column)
pub(super) fn dispose_kind_tag(kind: DisposeKind) -> &'static str {
    match kind {
        DisposeKind::Sell => "sell",
        DisposeKind::Spend => "spend",
    }
}
