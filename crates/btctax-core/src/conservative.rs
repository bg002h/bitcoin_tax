//! Conservative-filing advisories (Phase 3 / D-9). PURE builders over already-projected state — no
//! folding, no I/O. They are provenance-neutral (never assert "purchase"/"bought"; a tranche is
//! undocumented BTC filed at its AS-FILED basis) and never instruct a tax-understating action.

use crate::event::LedgerEvent;
use crate::price::PriceProvider;
use crate::project::{in_force_methods, ProjectionConfig};
use crate::state::{Disposal, LedgerState};
use crate::{BasisSource, LotMethod, WalletId};
use std::collections::BTreeSet;

/// D-9 dip advisory: `Some` iff the disposal consumed at least one conservative-filing tranche leg
/// (`EstimatedConservative`). One line per such leg, naming the estimated acquisition (`acquired_at` =
/// the tranche's `window_end`), the basis **AS FILED** — `leg.basis` printed directly, so a `$0` filing
/// says `$0` and a TP8(c) fee-sat carry that landed on the tranche leg says the documented fee basis
/// (tax r1 I-1) — and the resulting gain. Provenance-neutral: never "purchase"/"bought" (a tranche is
/// undocumented BTC, not a known buy), and it points to *substantiating* a higher basis (which only ever
/// LOWERS the reported gain toward the documented amount — never understates below it).
pub fn tranche_dip_advisory(disposal: &Disposal) -> Option<String> {
    let lines: Vec<String> = disposal
        .legs
        .iter()
        .filter(|l| l.basis_source == BasisSource::EstimatedConservative)
        .map(|l| {
            format!(
                "Conservative-filing dip — {sat} sat of undocumented BTC (estimated acquired by {acq}) \
                 disposed on {date} at ${basis} basis as filed, reporting ${gain} gain. If you can \
                 substantiate a higher basis for these units, recording it lowers the reported gain \
                 (never below the amount you can document).",
                sat = l.sat,
                acq = l.acquired_at,
                date = disposal.disposed_at,
                basis = l.basis,
                gain = l.gain,
            )
        })
        .collect();
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// D-9 method-inversion advisory: `Some` iff the in-force `method` for `wallet` is NON-HIFO **and** the
/// wallet still holds BOTH a conservative-filing tranche lot ($0 `EstimatedConservative`, remaining) and
/// a documented lot (remaining). Under a non-HIFO method a future disposal can draw the $0 tranche lot
/// before the documented higher-basis units — the gain-maximizing inversion of P2's emergent HIFO
/// steering. The advisory recommends a HIFO election. HIFO itself never inverts (it sorts $0 lots last),
/// and with no documented lot present there is nothing to draw first, so both cases return `None`.
pub fn method_inversion_advisory(
    state: &LedgerState,
    wallet: &WalletId,
    method: LotMethod,
) -> Option<String> {
    if method == LotMethod::Hifo {
        return None;
    }
    let has_tranche = state.lots.iter().any(|l| {
        l.wallet == *wallet
            && l.remaining_sat > 0
            && l.basis_source == BasisSource::EstimatedConservative
    });
    let has_documented = state.lots.iter().any(|l| {
        l.wallet == *wallet
            && l.remaining_sat > 0
            && l.basis_source != BasisSource::EstimatedConservative
    });
    if has_tranche && has_documented {
        Some(format!(
            "Method-inversion warning — the in-force lot method for this wallet is {method:?} (not HIFO). \
             Under it a disposal can draw a $0-basis conservative-filing unit before your documented \
             higher-basis units, maximizing the reported gain. Electing HIFO would draw the documented \
             units first: `btctax config --set-forward-method hifo`."
        ))
    } else {
        None
    }
}

/// D-9 report-time assembly (surfaced by `report --tax-year` + the TUI Tax tab): the combined
/// conservative-filing advisory for `year` — every dip advisory for a tranche disposal made in `year`,
/// followed by a method-inversion warning for each wallet still holding a tranche lot whose in-force
/// method (at `year`-end) is non-HIFO with a documented lot also present. `None` when there is nothing
/// to say. Pure over the projected `state` + `events` + `prices` + `config`; both frontends share it so
/// the CLI and TUI can never drift.
pub fn tranche_report_advisory(
    state: &LedgerState,
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    year: i32,
) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();

    // Dip advisories — one per disposal made in `year` that consumed a tranche leg.
    for d in state
        .disposals
        .iter()
        .filter(|d| d.disposed_at.year() == year)
    {
        if let Some(a) = tranche_dip_advisory(d) {
            lines.push(a);
        }
    }

    // Method-inversion warnings — per wallet still holding a tranche lot, keyed on the in-force method
    // at `year`-end (pre-2025 → config.pre2025_method; else the forward election / HIFO default).
    let tranche_wallets: Vec<WalletId> = state
        .lots
        .iter()
        .filter(|l| l.remaining_sat > 0 && l.basis_source == BasisSource::EstimatedConservative)
        .map(|l| l.wallet.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if !tranche_wallets.is_empty() {
        let as_of = time::Date::from_calendar_date(year, time::Month::December, 31)
            .expect("Dec 31 is always a valid date");
        let methods = in_force_methods(events, prices, config, as_of, &tranche_wallets);
        for (w, m) in tranche_wallets.iter().zip(methods) {
            if let Some(a) = method_inversion_advisory(state, w, m.method) {
                lines.push(a);
            }
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}
