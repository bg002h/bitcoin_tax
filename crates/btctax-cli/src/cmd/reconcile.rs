//! reconcile decision emitters (FR6/FR7/FR8, §7.2). Each fn builds exactly ONE `EventPayload` decision
//! variant and appends it via `append_decision` (monotonic `decision_seq`), then saves. Decisions are
//! append-only and re-projectable; the engine resolves precedence (latest-`decision_seq`, Void-first).
//! `now` is the injected decision creation-time / safe-harbor made-date (§6.2) — deterministic in tests.
use crate::{CliError, Session};
use btctax_core::persistence::append_decision;
use btctax_core::{
    ClassifyInbound, EventId, EventPayload, InboundClass, OutflowClass, ReclassifyOutflow,
    TransferLink, TransferTarget, Usd,
};
use btctax_store::Passphrase;
use std::path::Path;
use time::{OffsetDateTime, UtcOffset};

use crate::eventref::parse_event_id;

/// Append one decision (creation tz = UTC; decisions are not wallet-scoped) and persist.
fn append_and_save(
    session: &mut Session,
    payload: EventPayload,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let id = append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    session.save()?;
    Ok(id)
}

/// FR6: classify an externally-sourced inbound `TransferIn` as Income or a received Gift. For Income
/// this supplies the FMV basis; for Gift it supplies donor basis/date + fmv_at_gift (TP11 dual-basis).
/// This is the re-supply path for the §9.1 Swan `deposit` basis GAP.
pub fn classify_inbound(
    vault_path: &Path,
    pp: &Passphrase,
    in_ref: &str,
    class: InboundClass,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let transfer_in_event = parse_event_id(in_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let payload = EventPayload::ClassifyInbound(ClassifyInbound {
        transfer_in_event,
        as_: class,
    });
    append_and_save(&mut session, payload, now)
}

/// FR6: reclassify a pending `TransferOut` as a Sell/Spend disposition, a Gift out, or a Donation.
/// `principal` is the gross proceeds (Dispose) or FMV-at-transfer (Gift/Donate); `fee_usd` is the
/// optional disposition fee (TP8 / TP2). The engine applies the configured TP8 (c)/(b) fee treatment.
pub fn reclassify_outflow(
    vault_path: &Path,
    pp: &Passphrase,
    out_ref: &str,
    class: OutflowClass,
    principal: Usd,
    fee_usd: Option<Usd>,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let transfer_out_event = parse_event_id(out_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let payload = EventPayload::ReclassifyOutflow(ReclassifyOutflow {
        transfer_out_event,
        as_: class,
        principal_proceeds_or_fmv: principal,
        fee_usd,
    });
    append_and_save(&mut session, payload, now)
}

/// FR6/TP7: confirm a self-transfer. `target` is a destination `TransferIn` event (`--to-event`) or a
/// known wallet (`--to-wallet`); the engine relocates the lots carrying basis + acquired_at.
pub fn link_transfer(
    vault_path: &Path,
    pp: &Passphrase,
    out_ref: &str,
    target: TransferTarget,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let out_event = parse_event_id(out_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let payload = EventPayload::TransferLink(TransferLink {
        out_event,
        in_event_or_wallet: target,
    });
    append_and_save(&mut session, payload, now)
}
