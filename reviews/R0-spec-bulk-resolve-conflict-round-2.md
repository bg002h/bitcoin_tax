# R0 Spec Review — `design/SPEC_bulk_resolve_conflict.md` (round 2)

**Reviewer:** independent adversarial architect (did NOT author this spec).
**Branch:** `feat/bulk-resolve-conflict` @ `5256c7e` (main == `719e9fe`).
**Scope:** verify the round-1 folds (I1/M1/M2/N1) resolved with no new drift.
**Prior:** `reviews/R0-spec-bulk-resolve-conflict-round-1.md` (0C / 1I / 2M / 1N).

## Verdict: 0 Critical / 0 Important / 0 Minor / 2 Nit — **R0-GREEN** (implementation may proceed)

All four round-1 findings are resolved. The blocker (I1) is genuinely gone: no `ResolveKind` reference survives in any `btctax-cli` signature, and the two-apply-fn design is clean and idiomatic. Two cosmetic naming Nits remain (a stale fn name in the SemVer summary line + one loose "ResolveKind" prose mention in the CLI clap description); neither reintroduces the cross-crate problem nor gates implementation.

---

## Fold-by-fold verification

### [I1] RESOLVED — CLI no longer references `ResolveKind`
§D2 (lines 104-109) now specifies **two** apply fns `apply_bulk_accept_conflicts(vault, pp, conflict_events, now)` / `apply_bulk_reject_conflicts(...)`, each hard-coding its payload variant, with the clap `--accept`/`--reject` bool selecting the fn in dispatch. This mirrors the shipped single-item split `accept_conflict` (`reconcile.rs:179`) / `reject_conflict` (`:195`) exactly — a pattern the codebase already uses precisely to avoid a shared kind enum in the CLI. Verified: `grep -rn ResolveKind crates/btctax-cli/` returns **zero** hits. Both `SupersedeImport`/`RejectImport` are `btctax_core` types already built in `reconcile.rs:189/205`, so the two bulk fns are a trivial loop-extension of the shipped ones with `btctax-cli` CLI-atomicity (drop-session-on-`?`). The TUI-side wrapper `persist_bulk_resolve_conflict(..., kind: ResolveKind, ...)` (§D3, lines 137-144) correctly retains `ResolveKind` — it lives in `edit/persist.rs`, which already imports it. Design is clean and free of the dependency-cycle problem.

### [M1] RESOLVED — `BulkResolveRow` carries structured data, not pre-rendered strings
§D1 (lines 92-97) now defines `BulkResolveRow { conflict_event: EventId, date, target: EventId, current_payload: EventPayload, new_payload: EventPayload, new_fingerprint: String }` — no `current_summary`/`new_summary` String fields. Rationale note (lines 85-88) correctly states `import_payload_summary` is a private TUI-binary fn unreachable from `btctax-cli` and that each front-end renders its own summary, symmetric with `BulkStiRow`/`BulkLinkRow`. Feasibility confirmed: `EventPayload` is already imported in `session.rs:14`, and the CLI plan has the event index + `conflict.target`/`conflict.new_payload`/`conflict.new_fingerprint` (`event.rs:90-94`) to populate every field. D3 step 2 "current → new" (line 123) is now correctly the *rendered* display over the structured payloads. No lingering String-summary-row reference anywhere.

### [M2] RESOLVED — empty-label threaded; re-point is byte-for-byte
§Task 1 (lines 51-56, 68-69) now gives `persist_bulk_decisions(session, payloads, now, empty_label: &str)`, with the two shipped fns passing their EXACT existing strings. Verified against source: `persist.rs:401` = `"bulk link: nothing selected"`, `:449` = `"bulk classify-inbound-self-transfer: nothing selected"` — both reproduced verbatim in the spec comment (lines 53-55). The wrapper passes its own `"bulk resolve-conflict: nothing selected"` (line 143). The re-point is now truly zero-behavior (the one round-1 message-collapse deviation is eliminated); the empty-refuse KATs stay green unchanged.

### [N1] RESOLVED — `new_fingerprint` is a struct field
§D1 line 96: `pub new_fingerprint: String` — the 8-char disambiguator (`event.rs:93` truncated, as `open_resolve_conflict_flow` builds at `main.rs:6556-6566`). Struct and prose are now consistent.

---

## Residual drift (2 Nits — cosmetic; do NOT gate)

### [N-r2-1] NIT — SemVer bullet still names the OLD singular apply fn
Line 22: `cmd::reconcile::{bulk_resolve_conflict_plan, apply_bulk_resolve_conflict}` still lists `apply_bulk_resolve_conflict` (the pre-fold single fn), inconsistent with §D2's two fns `apply_bulk_accept_conflicts` / `apply_bulk_reject_conflicts`. This is a stale summary-line name only — NOT a `ResolveKind` reference, so I1 is unaffected. **Fix:** update line 22 to list the two fn names.

### [N-r2-2] NIT — CLI clap prose calls `--accept`/`--reject` "the batch-wide `ResolveKind`"
Line 113: "`--accept`/`--reject` is REQUIRED (the batch-wide `ResolveKind`; mutually exclusive)." In the CLI there is no `ResolveKind` (it's a bool selecting one of the two fns per lines 108-109); this is loose conceptual prose in the CLI section. The actual CLI signatures (line 104) are correct and `ResolveKind`-free. (Line 120's identical phrase is in the **TUI** `C`-flow step, where `ResolveKind` legitimately exists in `form.rs` — that one is correct.) **Fix:** reword line 113 to "the batch-wide accept/reject choice" to avoid implying a CLI type.

---

## Gate

**R0-GREEN: 0 Critical / 0 Important.** Implementation may proceed. The two Nits are optional cleanup best done in-line when the plan is written; they do not require another review round.
