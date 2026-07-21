//! Conservative-filing advisories (Phase 3 / D-9). PURE builders over already-projected state — no
//! folding, no I/O. They are provenance-neutral (never assert "purchase"/"bought"; a tranche is
//! undocumented BTC filed at its AS-FILED basis) and never instruct a tax-understating action.

use crate::conventions::TaxDate;
use crate::event::LedgerEvent;
use crate::optimize::{persistability, Persistability};
use crate::price::PriceProvider;
use crate::project::{in_force_methods, ProjectionConfig};
use crate::state::{Disposal, LedgerState};
use crate::{BasisSource, LotMethod, Usd, WalletId};
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

/// P5 coverage caveat (arch M-6): whether `window_reference`'s `min` spans EVERY day in the queried
/// window (`Full`) or only the subset with bundled data (`Partial`). A `Partial` covered-part min can
/// EXCEED the true window min, so P6 MUST surface this in user-visible copy (tax r1 N-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
    Full,
    Partial,
}

/// P5 window reference-price result: the min daily close (`min`) plus its `coverage` caveat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowRef {
    pub min: Usd,
    pub coverage: Coverage,
}

/// P5 window reference-price: the MIN daily CLOSE over `[start, end]` from `prices`. INFORMATIONAL ONLY
/// and NEVER filed (D-7) — it feeds only P6's overpayment-delta nudge. NOT a true floor: an intraday low
/// can be below any daily close (tax I-3), so the result CARRIES a `Coverage` caveat (arch M-6) rather
/// than pretending to be a floor. `Coverage::Partial` means some days in the window had no bundled close
/// (the covered-part min can then EXCEED the true window min — P6 surfaces the caveat). `None` when NO
/// day in the window has a close (no overlap — never fabricate a floor over a data gap); `start > end`
/// (an empty window — already refused at the declare-tranche record guard) is likewise `None`.
pub fn window_reference(
    prices: &dyn PriceProvider,
    start: TaxDate,
    end: TaxDate,
) -> Option<WindowRef> {
    let mut min: Option<Usd> = None;
    let mut covered: u64 = 0;
    let mut total: u64 = 0;
    let mut day = start;
    while day <= end {
        total += 1;
        if let Some(px) = prices.usd_per_btc(day) {
            covered += 1;
            min = Some(min.map_or(px, |m| if px < m { px } else { m }));
        }
        match day.next_day() {
            Some(d) => day = d,
            None => break, // time::Date::MAX — end already processed above
        }
    }
    min.map(|m| WindowRef {
        min: m,
        coverage: if covered == total {
            Coverage::Full
        } else {
            Coverage::Partial
        },
    })
}

/// P4 / D-3 custody-aware compliance warning: `Some` iff specifically identifying an undocumented
/// (tranche) unit held at an EXCHANGE (broker) for a disposal on `sale_date` falls inside the 2027+
/// broker envelope, where own-books specific identification is INSUFFICIENT — the broker must
/// communicate the specific identification by the time of sale, or the sale defaults to FIFO. This is
/// pure REUSE of the optimizer's `persistability` gate (D-3, verified TRUE by both lenses): the warning
/// fires exactly when that gate returns `ForbiddenBroker2027` (a broker wallet with a `year >= 2027`
/// sale). `SelfCustody` (own-books, never expires) and `≤2026` sales (the Notices 2025-7/2026-20
/// own-books transitional relief, in force through 2026-12-31) return `None`. No transfer-statement
/// modeling in v1 (D-3). `selection_made` is threaded to `persistability` faithfully — the
/// `ForbiddenBroker2027` branch ignores it (the broker envelope precedes the contemporaneous lever),
/// but threading it means this advisory inherits any future change to that gate's semantics rather than
/// re-deriving the predicate. Provenance-neutral (never asserts "purchase"/"bought").
pub fn tranche_broker_specific_id_advisory(
    wallet: &WalletId,
    sale_date: TaxDate,
    selection_made: TaxDate,
) -> Option<String> {
    match persistability(wallet, sale_date, selection_made) {
        Persistability::ForbiddenBroker2027 => Some(format!(
            "Broker specific-ID warning — this {year} disposal draws undocumented BTC held at an \
             exchange (broker). From 2027, own-books specific identification is INSUFFICIENT at a \
             broker (the Notices 2025-7/2026-20 own-books transitional relief ended 2026-12-31): the \
             broker must communicate the specific identification by the time of sale, or the sale \
             defaults to FIFO. To keep own-books specific-ID for no-records units, hold them in \
             self-custody.",
            year = sale_date.year(),
        )),
        _ => None,
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

    // Dip advisories — one per disposal made in `year` that consumed a tranche leg — plus the P4/D-3
    // custody-aware compliance warning for a disposal that draws a tranche lot held at a broker in the
    // 2027+ envelope. Disposals are single-wallet, so the first tranche leg's wallet is representative.
    // Scope decision (D-3, deliberate): the warning is DISPOSAL-scoped, NOT gated on an explicit
    // `LotSelection` — it is prospective/conditional ("to specifically identify these units at a 2027+
    // broker you need a broker-communicated selection..."), so it also reaches the filer who has not yet
    // recorded a specific-ID but whose undocumented units sit at a broker (exactly the P8 self-custody
    // audience). Informational, never gates; it errs toward SURFACING the 2027 limitation rather than
    // risk under-warning (cf. SPEC D-8's accepted knowingly-over-broad friendly warning).
    for d in state
        .disposals
        .iter()
        .filter(|d| d.disposed_at.year() == year)
    {
        if let Some(a) = tranche_dip_advisory(d) {
            lines.push(a);
        }
        if let Some(w) = d
            .legs
            .iter()
            .find(|l| l.basis_source == BasisSource::EstimatedConservative)
            .map(|l| &l.wallet)
        {
            // The disposal date doubles as `selection_made` — irrelevant to the ForbiddenBroker2027
            // branch (the broker envelope precedes the contemporaneous lever) — see the builder doc.
            if let Some(a) = tranche_broker_specific_id_advisory(w, d.disposed_at, d.disposed_at) {
                lines.push(a);
            }
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
