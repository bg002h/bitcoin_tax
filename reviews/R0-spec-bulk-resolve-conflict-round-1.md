# R0 Spec Review — `design/SPEC_bulk_resolve_conflict.md` (round 1)

**Reviewer:** independent adversarial architect (did NOT author this spec).
**Branch:** `feat/bulk-resolve-conflict` @ `5256c7e` (main == `719e9fe`).
**Artifact:** `design/SPEC_bulk_resolve_conflict.md` (2 tasks: extract `persist_bulk_decisions`; bulk-resolve-conflict `C`).
**Bar:** 0 Critical / 0 Important. Critical/Important block implementation.

## Verdict: 0 Critical / 1 Important / 2 Minor / 1 Nit

The spine is sound. Task 1's extraction is genuinely zero-behavior (the two shipped fns are byte-identical except the per-row payload build and the empty-guard string), the mid-batch-rollback safety invariant is faithfully preserved and is pinned for BOTH callers, the candidate set + both-side-summary join is correct against `open_resolve_conflict_flow`, the non-revocable Tier-B ceremony is well-calibrated and grounded in the shipped single-item modal, key `C` is free, and core is genuinely unchanged. All grounding anchors verified accurate at `5256c7e`.

The one blocking issue: the CLI `apply_bulk_resolve_conflict` signature commits to `kind: ResolveKind`, a type that lives ONLY in `btctax-tui-edit` and is unreachable from `btctax-cli` (would require a dependency cycle). The spec must define the CLI's accept/reject representation. Two Minors + a Nit sit around the CLI plan layer and the "zero behavior" claim.

---

### [I1] IMPORTANT — CLI `apply_bulk_resolve_conflict` signature references `ResolveKind`, which `btctax-cli` cannot see (dependency cycle / build break)

`design/SPEC_bulk_resolve_conflict.md` §D2:
> `apply_bulk_resolve_conflict(vault, pp, conflict_events, kind: ResolveKind, now)`
> `cmd::reconcile::{bulk_resolve_conflict_plan, apply_bulk_resolve_conflict}` … (SemVer line: "btctax-cli: MINOR/additive")

**Defect.** `ResolveKind` is defined **only** at `crates/btctax-tui-edit/src/edit/form.rs:1599`. The dependency direction is `btctax-tui-edit → btctax-cli` (`crates/btctax-tui-edit/Cargo.toml:14`); `btctax-cli` does **not** (and cannot) depend on `btctax-tui-edit` (verified: zero `ResolveKind` occurrences in `crates/btctax-cli/`). So a `cmd::reconcile::apply_bulk_resolve_conflict(..., kind: ResolveKind, ...)` in the CLI crate does not compile — the type is not in scope, and adding the dep would form a cycle Cargo rejects.

The shipped single-item CLI deliberately AVOIDS a shared kind: `accept_conflict` (`reconcile.rs:179`) and `reject_conflict` (`:195`) are two separate fns, each hard-coding its payload. The spec neither reuses that pattern nor defines a CLI-local representation.

**Why it gates.** The spec's committed API cannot be implemented as written. It also leaves the CLI accept/reject *typing* undefined, which in turn under-specifies the clap `--accept | --reject` wiring and the subject of KAT `bulk_resolve_cli_requires_accept_xor_reject` (bool? `ArgGroup`? CLI-local enum?). An implementer must invent a design decision the spec was supposed to make.

**Fix.** Specify the CLI's accept/reject representation without `ResolveKind`. Cleanest options, in order of fit with the codebase:
- mirror the shipped single-item pattern — two apply fns (`apply_bulk_accept_conflicts` / `apply_bulk_reject_conflicts`), no shared enum; or
- a `bool accept` param (clap `ArgGroup(required, multiple=false)` → bool); or
- a CLI-local `enum` in `btctax-cli` (the TUI's `persist_bulk_resolve_conflict` keeps using `crate::edit::form::ResolveKind`, which is fine — that fn is in `btctax-tui-edit`).
Note: the TUI-side wrapper `persist_bulk_resolve_conflict(..., kind: ResolveKind, ...)` (§D3, in `edit/persist.rs`) is fine as written — `persist.rs` already imports `ResolveKind` for `persist_resolve_conflict`. Only the CLI signature is broken.

---

### [M1] MINOR — CLI `BulkResolveRow{current_summary, new_summary}` needs a summary builder that does not exist in `btctax-cli`

§D1 places `BulkResolvePlan`/`BulkResolveRow` (with `current_summary: String`, `new_summary: String`) on `Session::bulk_resolve_conflict_plan` (i.e., in `crates/btctax-cli/src/session.rs`), and says the rows are built "exactly as `open_resolve_conflict_flow` does."

**Defect.** `open_resolve_conflict_flow` builds those strings via `import_payload_summary` (`crates/btctax-tui-edit/src/main.rs:6488`) — a private fn in the TUI **binary** crate, unreachable from `btctax-cli`. The sibling CLI plans (`BulkStiRow` at `session.rs:92`, `BulkLinkRow` at `:44`) carry **structured** data (ids/dates/sats), not pre-rendered summary strings — the front-ends format. Putting `String` summaries on a `btctax-cli` row both breaks the established pattern and requires a summary fn the crate lacks.

**Why it's Minor (not blocking).** Display text only; no data-integrity or safety-path impact. But it's a real feasibility gap that will surface mid-implementation if unaddressed.

**Fix.** Either (a) have `BulkResolveRow` carry structured data (`conflict_event`, `date`, `target`, and the raw current/new `EventPayload` or the pieces needed) and let each front-end render its own summary (CLI table formatter; TUI reuses `import_payload_summary`); or (b) hoist a summary helper into `btctax-cli`/`btctax-core` and have both front-ends call it. Prefer (a) for symmetry with `BulkStiRow`/`BulkLinkRow`.

---

### [M2] MINOR — the unified empty-guard message is a (real, if unobservable) deviation from the "ZERO behavior change" / "byte-for-byte" claim

§Task 1 pseudocode: `return Err(/* same NoChange(CliError::Usage(..)) shape as the shipped fns */)` — a single message.

**Defect.** The two shipped fns return **different** `CliError::Usage` strings: `"bulk link: nothing selected"` (`persist.rs:401`) vs `"bulk classify-inbound-self-transfer: nothing selected"` (`:449`). Re-pointing both at one `persist_bulk_decisions` collapses them to one string — a behavior change to the returned error, contradicting the spec's loudly-stated "ZERO behavior change" / "byte-for-byte" invariant. The empty-refuse KATs (`kat_persist_bulk_link_refuses_empty:3718`, `kat_persist_bulk_sti_refuses_empty:3939`) assert only `matches!(_, Err(NoChange(_)))`, so they stay green — the delta is genuinely unobservable (the persist fns are TUI-only via KAT-G1; the UI gates empty earlier; the CLI has its own non-guarded apply path). But the spec's own strong claim invites the correction.

**Fix.** State explicitly that the empty-guard message is unified (acceptable — unobservable), OR thread a caller-supplied label into `persist_bulk_decisions` to preserve per-caller strings. Either is fine; the spec should make it a conscious choice rather than gloss it under "byte-for-byte."

---

### [N1] NIT — `BulkResolveRow` struct omits `new_fingerprint`, but the §D1 prose lists it as built per row

§D1 prose: "build, per row: `conflict_event`, `date`, `target`, `current_summary` and `new_summary` + `new_fingerprint` — exactly as `open_resolve_conflict_flow` does." The adjacent struct definition has no `new_fingerprint` field. `open_resolve_conflict_flow` does compute an 8-char `new_fingerprint` on `ConflictItem` (`main.rs:6556-6566`) for disambiguation.

**Fix.** Decide whether the bulk preview disambiguates rows with a fingerprint (add the field) or not (drop it from the prose). Trivial; resolve at plan time so the row shape is settled.

---

## Pressure-test results (affirmations)

1. **Task 1 extraction faithfulness — SOUND.** `persist_bulk_link_transfer` (`:393`) and `persist_bulk_self_transfer_in` (`:442`) are structurally identical: empty-guard `NoChange` → `snapshot()?` → `for … { if let Err(e) = append_decision(conn, payload, now, UTC, None) { return Err(rollback(session,&pre,e.into())) } }` → `save_or_rollback(session, pre)?` → `Ok(len)`. The ONLY per-row difference is the payload built (`TransferLink` vs `ClassifyInbound{SelfTransferMine{None,None}}`) — no fee handling, no side-effect, no error-mapping divergence. Building `Vec<EventPayload>` up-front then consuming in the helper is order- and value-identical (payloads are pure data). The `NoChange` shape, the mid-batch `rollback(...)` (NOT `?`), and the single `save_or_rollback` all transfer exactly. Only deviation = the empty-guard string (see M2).

2. **Mid-batch rollback is pinned for BOTH — SOUND.** `kat_persist_bulk_link_reverts_mid_batch_append_failure` (`:3811`) and `kat_persist_bulk_sti_reverts_mid_batch_append_failure` (`:3968`) both inject a BEFORE-INSERT trigger that aborts the SECOND append and assert `RolledBack` + event-log byte-unchanged (append #1's phantom reverted) + clean retry. A regression in the extracted helper's whole-batch revert would be caught for both callers. Empty-refuse and strict-prefix are likewise pinned for both; the shared `save_or_rollback` failure path is pinned by `kat_persist_bulk_link_rolls_back_on_failed_save` (`:3745`) plus every single-append rollback KAT. Coverage is adequate; the spec's "existing KATs are the pins" holds.

3. **Candidate set + both-side join — CORRECT.** Filtering `snap.state.blockers` to `kind == BlockerKind::ImportConflict` (`state.rs:26`) is the right set: the blocker fires ONLY while unresolved (per `open_resolve_conflict_flow` doc, resolve.rs:386-401), so an accepted/rejected conflict drops out → structural idempotence, no client-side exclusion needed. `Blocker.event` (`state.rs:95`) carries the `ImportConflict` EventId (`conflict_id = b.event.as_ref()?`, `main.rs:6540`); `current_summary` is resolved from `conflict.target`'s payload and `new_summary` from `conflict.new_payload` (`ImportConflict{target, new_payload, new_fingerprint}`, `event.rs:90-94`), with `target != conflict_event` — exactly as `open_resolve_conflict_flow` (`main.rs:6550-6568`) and §G4 state. Target is always an imported event, never a decision, so "a conflict whose target is a decision" cannot arise.

4. **Non-revocable ceremony — CORRECTLY CALIBRATED.** `SupersedeImport`/`RejectImport` are excluded from `is_revocable_payload` (`form.rs:896-912`; pinned by `kat_rc_supersede_reject_are_non_revocable:3106`). The shipped single-item modal `draw_resolve_conflict_modal` (`draw_edit.rs:2849`) is already Tier-B ("This decision CANNOT be voided (non-revocable)", red border, Enter/Esc — NOT typed-word). The bulk flow reusing that framing pluralized (+ per-row preview + checked count) is strictly ≥ the single-item ceremony and < the reserved `ATTEST` typed-word (`draw_edit.rs:2681`, for the §7.4 unrecoverable batch). §G2's warning to NOT reuse the bulk-link "each link voidable" reassurance (`draw_edit.rs:3512`) is correct. Proportionate and grounded.

5. **Payload/apply + idempotence — CORRECT.** `persist_bulk_resolve_conflict` building `SupersedeImport{conflict_event}`/`RejectImport{conflict_event}` per row matches `persist_resolve_conflict` (`:533-540`) and the CLI verbs (`reconcile.rs:189/205`). Each candidate blocker is a distinct `conflict_event`, so no intra-batch duplicate → no `DecisionConflict`; the candidate set is snapshot-once, and resolving conflict A does not invalidate conflict B's event id. Re-running excludes resolved conflicts. Empty selection → the shared empty guard refuses. The batch-toggle + per-row-exclude model is coherent, and the "resolve some differently" case is explicitly handled out-of-band (exclude + single-item `i`).

6. **Key `C` free / SemVer / grounding — CONFIRMED.** No `KeyCode::Char('C')` binding exists in the Browse keymap (`main.rs:340-356`; `b`/`B`/`m`/`i` bound, `C` absent); uppercase bindings already work (`A`/`B`/`G`). Core is genuinely unchanged (reuses `SupersedeImport`/`RejectImport`; no new decision variant → no forward-only serde break). All cited anchors verified accurate: `persist.rs` 393/442/525, `event.rs` 90/192/196, `form.rs:896`, `reconcile.rs` 179/195/216/231, `draw_edit.rs` resolve modal + non-revocable wording, `BlockerKind::ImportConflict` `state.rs:26`.

---

## Gate

**I1 (Important) blocks implementation.** Resolve the CLI accept/reject representation before Task 2. M1/M2/N1 are non-blocking but should be folded (M1 especially, to avoid a mid-implementation cross-crate surprise on the plan row shape). Re-review after the fold.
