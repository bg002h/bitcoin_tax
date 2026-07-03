# SPEC — btctax-tui-edit chunk 4b: resolve-conflict + optimize-accept

**Source baseline:** `main` @ `dc0859d` (post chunk-4a; all anchors verified at write time).
**Review status: R0-GREEN (2 rounds; 0 Critical / 0 Important). Reviews:
`reviews/R0-spec-tui-edit-chunk4b-round-{1,2}.md` (round 1: 0C/1I/3M/1N — the per-disposal-Δtax
data-model catch; round 2: 0C/0I).**
**Design lineage:** chunk-4 architect design (4b half), citations refreshed against post-4a `main`.
Second half of chunk 4 (4a = link-transfer + classify-raw, shipped).

**Goal.** Two new decision flows — both "accept a PROPOSED change":
1. **resolve-conflict** (`i`) — accept or reject a flagged import conflict → `SupersedeImport` (accept)
   or `RejectImport` (reject) decision. **Non-revocable** (prominent warning; NOT typed-word).
2. **optimize-accept** (`z`) — recompute the optimizer and persist a proposed `LotSelection` (plus, for
   already-executed disposals, an `optimize_attestation` side-table row). The heaviest flow (optimizer
   recompute + dual-write).

**SemVer.** New `pub fn`s `persist_resolve_conflict`, `persist_optimize_accept` in `edit/persist.rs`;
new flow/modal structs; keys `i`/`z`. **One new KAT-G1 token: `optimize_attest::set`** (like chunk 3's
`donation_details::set`). Optional additive `Session::optimize_proposal` read helper in `btctax-cli`
(flag-free → still no lockstep). No `btctax-core` change (all types/`optimize_year` already `pub`).
**MINOR** (pre-1.0; additive). **No lockstep** (TUI-only; no clap flags).

---

## Substrate inherited (verified at `dc0859d`)

- Openers call `residue_latch_status()` first; `save_or_rollback`/`PersistError`/`on_persist_error`
  baseline (save-rollback cycle); `#8` quit-first status convention; `TargetList<T>`; blocker-derived
  status. Free keys `i` and `z` confirmed (Browse binds `p c o r f v s d a l u` + nav).
- KAT-G1 `persist_only_tokens` at `persist.rs:1119` (`conn( / save( / tax_profile::set / append_ /
  donation_details::set / restore(`). `append_` covers the resolve-conflict appends; optimize-accept
  adds `optimize_attest::set`.

---

## D3 — resolve-conflict (accept + reject, unified) (`i`)

Mirrors TWO CLI verbs: `accept_conflict` → `SupersedeImport{conflict_event}` (`reconcile.rs:178`) and
`reject_conflict` → `RejectImport{conflict_event}` (`reconcile.rs:194`). Identical eligible set, differ
only in the appended variant → **one flow with an accept/reject branch**.
`SupersedeImport`/`RejectImport` at `event.rs:176/180`; `ImportConflict{target,new_payload,
new_fingerprint}` at `event.rs:86`.

**Pre-filter:** events carrying `BlockerKind::ImportConflict` (Hard; `state.rs:26`; fires only while
UNRESOLVED — `resolve.rs:386-401`), so no extra exclusion is needed (inherently post-filtered, like
reclassify-outflow's pending list). The blocker's `.event` is the conflict EventId; the
`ImportConflict` payload lives at that id in `snap.events`.

**Flow (no free text).** Step 1 pick a conflict (Table: Date | Target | New-fingerprint | conflict
EventId, title `" Resolve Import Conflict — select a conflict "`). Step 2 = an accept/reject **choice**
(an in-flow left/right toggle — do NOT use Browse-level `a`; use `←/→` or `h/l`-style within the flow).
`Enter` → `resolve_conflict_modal`. Esc steps back (choice → conflict-list → close). `q` swallowed.

**Flow/modal state:**
```rust
pub enum ResolveKind { Accept, Reject }
pub enum ResolveConflictStep { List, Choose { conflict: ConflictItem, kind: ResolveKind } }
pub struct ResolveConflictFlowState { pub list: TargetList<ConflictItem>, pub step: ResolveConflictStep }
pub struct ResolveConflictModalState { pub conflict_event: EventId, pub target: EventId,
    pub kind: ResolveKind, pub old_summary: String, pub new_summary: String }
```

**Modal** — must show BOTH sides + the NON-REVOCABLE warning. Resolve `new_payload` (in the conflict
event) against the target's CURRENT payload (a separate event, `conflict_event != target`):
```
╔═ Confirm: ACCEPT conflict — WRITES THE VAULT ═════════════╗
║  conflict: conflict|river|row-88|a1b2                      ║
║  target:   import|river|row-88                             ║
║  current:  Income 100000 sat @ $6,800                      ║
║  →new:     Income 100000 sat @ $7,050   (ACCEPT adopts new)║
║  !! This decision CANNOT be voided (non-revocable).        ║
║  [Enter] Confirm & save   [Esc] Cancel — writes nothing    ║
╚════════════════════════════════════════════════════════════╝
```
Reject variant: `"→new: … (REJECT keeps current, discards new)"`. **NOT a typed-word gate** — that is
reserved for the §7.4 unrecoverable-batch attest; a single non-revocable append with a prominent
warning + `save_or_rollback` (clean retry) is the correct ceremony. (`SupersedeImport`/`RejectImport`
are excluded from `is_revocable_payload`; a later void fires `DecisionConflict`, `resolve.rs:312-313`.)

**persist fn (one fn, kind param):**
```rust
pub fn persist_resolve_conflict(session: &mut Session, conflict_event: EventId, kind: ResolveKind,
    now: OffsetDateTime) -> Result<EventId, PersistError> {
    let payload = match kind {
        ResolveKind::Accept => EventPayload::SupersedeImport(SupersedeImport { conflict_event }),
        ResolveKind::Reject => EventPayload::RejectImport(RejectImport { conflict_event }),
    };
    let pre = session.snapshot()?;
    let id = append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    save_or_rollback(session, pre)?;
    Ok(id)
}
```

**Post-save status:** the `ImportConflict` blocker for the target clears on success →
`"Conflict {conflict.canonical()} {accepted/rejected}; import-conflict resolved."` (No `DecisionConflict`
retry arm is reachable: the pre-filter removes resolved conflicts; a failed save rolls back clean.)

---

## D4 — optimize-accept (`z`)

Mirrors `cmd::optimize::accept` (`cmd/optimize.rs:142`). The heaviest flow, and the only non-plain-append
one: (a) RECOMPUTES the optimizer (never trusts a stale proposal, NFR4); (b) appends a `LotSelection`
decision; (c) for already-executed disposals, ALSO upserts the `optimize_attestation` side-table
(`optimize_attest::set(conn, disposal, attestation, attested_at)`, `optimize_attest.rs:26`).

**Opener = read-only optimizer recompute (KAT-G1-clean).** Compute an `OptimizeProposal`
(`optimize.rs:86`; `per_disposal: Vec<DisposalProposal>`, `optimize.rs:49`, each
`{disposal, current_selection, proposed_selection, persistable}`) via core `optimize_year`
(`optimize.rs:713`) — do NOT call `cmd::optimize::accept` (it opens its own `Session` → deadlock on the
held VaultLock; `cmd::` is KAT-G1-forbidden).

**PRIMARY approach [R0-M2/N1]:** add a read-only `Session::optimize_proposal(year, now) ->
Result<OptimizeProposal, CliError>` in `btctax-cli`. It assembles `optimize_year`'s inputs the way
`cmd::optimize::run`/`accept` do — `events`,`config` from its own conn, `prices = BundledPrices::load()`,
`tables` from the bundled tables, `profile = self.tax_profile(year)?` (read FRESH, not from `snap`),
`attested = self.optimize_attested_set()?`, `proposal_made = tax_date(now, UtcOffset::UTC)` [R0-M1: the
2-arg form] — and applies `map_opt_err` INTERNALLY (returning `CliError`), since `map_opt_err` is
`pub(crate)` and NOT TUI-reachable [R0-M2]. Additive, flag-free → no lockstep; keeps all optimizer
plumbing out of `main.rs`. The opener calls `session.optimize_proposal(year, now)` and on `Err(e)` shows
`"{e} — quit the editor and run: btctax optimize consult"`, no-open. (`optimize_year` returns only
`PreTransitionYear`/`YearNotComputable`/`NoDisposals` — `optimize.rs:723-750`; `NoLots`/`Evaluate` are
consult-only, so the free-standing fallback would map just those three.)

**Pre-filter of the proposal list:** keep only `per_disposal` rows where `proposed_selection !=
current_selection` (no-change = the CLI "already optimal" skip) AND `persistable !=
ForbiddenBroker2027` (2027+ broker lots NEVER persist — no attestation cures them, `optimize.rs:454-477`)
AND the disposal has NO live (non-voided) `LotSelection` (else the append is a duplicate ⇒
`DecisionConflict` NEITHER-applies, `resolve.rs:787-800` — mirrors select-lots' already-selected
exclusion; the KEY optimize-accept pre-filter subtlety). **Empty filtered list → status "No persistable
optimizer improvements available" + NO open [R0-M3]** (the void R0-M8 discipline).

**Flow:** step 1 pick a proposed disposal (Table: **Date | Wallet | Persistability | disposal EventId**
— NO per-disposal Δtax column [R0-I1]: `DisposalProposal` carries no per-disposal delta, and
`OptimizeProposal.delta` is the WHOLE-YEAR optimized−baseline figure (`optimize.rs:90`); a per-row
dollar value would be a misleading fabrication — the CLI's `render_optimize_proposal` likewise shows
per-disposal `disposal·date·wallet·status·picks` only). Show a **flow-level banner** with the year
figure: `"Expected year Δtax if the FULL proposal is accepted: {delta} (≤ 0)"`, plus the "APPROXIMATE —
not a guaranteed global minimum" caveat when `proposal.approximate` (`optimize.rs:86-102`). Step 2
branches on `persistable`:
- `ContemporaneousNow` (`optimize.rs:39`) → straight to the modal (basis `"Contemporaneous"`).
- `NeedsAttestation` (`optimize.rs:42`) → an **attestation-text step** (the user types the contemporaneous-
  ID statement = the `--attest "<text>"` value; non-empty required) → modal.

**Modal** shows disposal id, the proposed `LotSelection` picks (elide past 8 like select-lots), the
basis label (`Contemporaneous`/`AttestedRecording`) [no per-disposal Δtax — R0-I1], and for the
attested case the attestation text +
`"an attestation row is written alongside the LotSelection; voiding the LotSelection clears it."`

**persist fn (dual-write; adds the KAT-G1 token; INVERSE of `persist_void`):**
```rust
pub fn persist_optimize_accept(session: &mut Session, disposal: EventId, picks: Vec<LotPick>,
    attestation: Option<String>, made: TaxDate, now: OffsetDateTime) -> Result<EventId, PersistError> {
    let pre = session.snapshot()?;
    let payload = EventPayload::LotSelection(LotSelection { disposal_event: disposal.clone(), lots: picks });
    let id = append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    if let Some(att) = attestation {
        if let Err(e) = btctax_cli::optimize_attest::set(session.conn(), &disposal, &att, &made.to_string()) {
            return Err(rollback(session, &pre, e));   // symmetric with persist_void's clear-then-rollback
        }
    }
    save_or_rollback(session, pre)?;   // whole-DB restore reverts BOTH the append AND the side-table set
    Ok(id)
}
```

**KAT-G1 change (REQUIRED):** add `"optimize_attest::set"` to `persist_only_tokens` (`persist.rs:1119`)
+ the plant-a-token self-check, exactly as chunk 3 added `donation_details::set`.
(`optimize_attest::clear` is already used by `persist_void`; `set` is the new mutation-surface token.)

**Post-save status (keyed to `decision_id`):** (1) `DecisionConflict` on `decision_id` (duplicate
LotSelection — only via failed-save race) → NEITHER-applies/method-order wording (reuse
`derive_select_lots_status` arm 1). (2) `LotSelectionInvalid` for the disposal → "saved but invalid —
see Compliance; void ('v') and retry." (3) Clean → `"Optimizer selection recorded for
{disposal.canonical()} — {N} lot(s); {basis}."` (+ "; attestation recorded" when attested).

---

## Interactions (chunk-4 architect)

- **optimize-accept ↔ `persist_void`'s `optimize_attest::clear` (save-rollback cycle) — positive,
  closed-loop, NO new work.** optimize-accept (attested path) is the PRODUCER of the `optimize_attestation`
  row keyed by `disposal_event`; the shipped `persist_void` detects a `LotSelection` void target and
  calls `optimize_attest::clear(conn, ls.disposal_event)` in its rolled-back batch. So voiding an
  optimize-accepted `LotSelection` via `v` automatically clears its attestation — exactly the CLI's
  behavior. Closes the loop with zero `persist_void` changes; pinned by re-using `kat_p2f`'s pattern.
- **optimize-accept ↔ void-list #7 filter (hardening cycle) — NONE.** #7 excludes effective
  `SafeHarborAllocation`s; optimize-accept writes `LotSelection`, so its decisions remain normally
  voidable and appear in the `v` list.

## KATs (chunk-3 skeleton)

Per flow: strict-prefix persist KAT; cancel-path bytes-unchanged KAT (`q` swallowed, Esc steps back);
save-error chmod KAT (`save_or_rollback` reverts, retry clean, `on_persist_error`); validation KATs;
E2E. Plus:
- **resolve-conflict:** E2E accept (target adopts `new_payload`, `ImportConflict` cleared) AND reject
  (original stands, blocker cleared); **non-revocable KAT** — after accept, the `v` void list either
  omits it or a confirmed void yields the `DecisionConflict`-not-"Voided" status (mirrors chunk-3
  KAT-E2E-ATTEST-VOID).
- **optimize-accept:** E2E attested → `post.len()+1` (LotSelection) AND `optimize_attest::get(conn,
  disposal) == Some(text)`; then **void round-trip** → `optimize_attest::get == None` (reuses the shipped
  `persist_void` clear); `ForbiddenBroker2027` excluded from list; duplicate-LotSelection pre-filter (a
  disposal with a live LotSelection is not offered); `NeedsAttestation` requires non-empty text;
  `ContemporaneousNow` skips the text step.
- **KAT-G1** stays green with `optimize_attest::set` added to `persist_only_tokens` + the self-check
  plant; no forbidden token in `main.rs`/`form.rs`/`draw_edit.rs`.

## Plan (TDD, phased — each: KATs red → implement green → review to 0C/0I)

- **Task 1 — resolve-conflict** (form.rs structs; editor.rs flow+modal; main.rs `i` dispatch + opener +
  handlers + status; draw_edit.rs; persist.rs `persist_resolve_conflict`).
- **Task 2 — optimize-accept** (the `Session::optimize_proposal` read helper + the recompute opener;
  the attestation-text step; persist.rs `persist_optimize_accept` + KAT-G1 `optimize_attest::set` token).
- **Task 3 — whole-diff review (Phase E) + FOLLOWUPS.**

## Gotchas (chunk-4 architect)

1. optimize-accept must NOT call `cmd::optimize::accept` (own `Session` → deadlock; `cmd::` forbidden).
2. Add `optimize_attest::set` to `persist_only_tokens` AND the self-check plant.
3. resolve-conflict non-revocability: the modal MUST warn "cannot be voided"; do NOT add
   `SupersedeImport`/`RejectImport` to `is_revocable_payload`.
4. optimize-accept duplicate-LotSelection guard (exclude disposals with a live `LotSelection`) is
   MANDATORY — else accept silently worsens the outcome (NEITHER applies, method-order fallback).
5. `ImportConflict` display resolves `new_payload` (conflict event) vs the TARGET's payload (separate
   event); accept adopts `new_payload` onto the original id, reject keeps the original.
6. optimize-accept `approximate` banner when `OptimizeProposal.approximate` — never present the
   optimized tax as "the optimum."

## Out of scope
- (The additive read helper `Session::optimize_proposal` is the PRIMARY opener path [R0-M2/N1] — in
  scope, flag-free, no lockstep; the free-standing assembly is a KAT-G1-clean fallback.)
- Chunk 5 (safe-harbor-allocate) — the next cycle.
- Any `btctax-core`/`btctax-cli` public API beyond the optional read helper; viewer changes.
