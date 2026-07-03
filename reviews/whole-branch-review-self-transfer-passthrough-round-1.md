# Whole-diff review (Phase E) — `feat/self-transfer-passthrough` (round 1)

**Scope:** `git diff main..HEAD` (main == `1dcacad`; spec `7d594bc`, Task 1 `6f9e0d0`, Task 2 `d63af07`,
Task 3 `28aa4ad`). 14 files, ~2646 ins / 27 del. Contract: `design/SPEC_self_transfer_passthrough.md`
(R0-GREEN, 2 rounds). Reviewer role: independent adversarial, btctax-core-touching, tax-safety-critical.
**Bar: 0 Critical / 0 Important.**

## Verdict: **0 Critical / 0 Important / 0 Minor / 2 Nit → SHIP**

The implementation faithfully realizes the R0-GREEN spec. The load-bearing tax-safety guard [I1] is
present, correctly placed (a separate loop AFTER pass-1e collection), its four sets are exhaustive, and
it is order-independent. I fault-injected the three starred paths (I1 both directions + DROP), each went
RED against the shipped KATs, and I restored the tree byte-for-byte (`git status` clean). The CLI
force-apply path (item 2) cannot hide a taxable event — it is fully guarded at projection. No Critical or
Important findings; two Nits, both non-blocking.

---

## Fault-injection results (mandated)

All performed on `crates/btctax-core/src/project/resolve.rs`, one KAT run each, tree restored byte-clean
after every probe (`git status --porcelain` empty; final re-run of all 9 `passthrough` KATs green).

| # | Injection | KAT | Result |
|---|-----------|-----|--------|
| I1(a) | `out_overlaps = false` (neuter out-leg guard) | `passthrough_then_dispose_on_out_leg_conflict_disposal_recognized` | **RED** — "the passthrough must lose to a Hard DecisionConflict"; a real `Dispose` on the out-leg is silently `Op::Skip`'d. Guard is load-bearing. |
| I1(b) | `in_overlaps = false` (neuter in-leg guard) | `passthrough_then_income_on_in_leg_conflict_income_recognized` | **RED** — income on the in-leg silently skipped. |
| DROP | skip only the in-leg (omit `passthrough_skip.insert(out_target)`) | `passthrough_drop_skips_both_legs_non_taxable` + `passthrough_out_leg_never_pending_and_never_misconsumes` (+ `passthrough_duplicate_leg_first_wins`) | **RED (3)** — orphaned out lands in `pending_reconciliation` AND mis-consumes the co-located BUY lot (holdings drop 100k→0). Proves G-BOTH-ATOMIC. |

After restore: `cargo test -p btctax-core --test kat_tax passthrough` → **9 passed**; `git diff` empty.

---

## Per-item verification

### 1. [I1 ★★] cross-type overlap guard — VERIFIED CORRECT & COMPLETE
- **Structure (`resolve.rs:773-802`):** a SEPARATE `for (dec_id, in_ev, out_ev) in &passthroughs` loop
  that runs AFTER the pass-1e collection loop (`:514-772`) — not inside the collector arm. So a
  passthrough appended BEFORE the conflicting `ReclassifyOutflow`/`ClassifyInbound` is still caught: the
  guard reads the fully-populated `links`/`consumed_ins`/`inbound_class`/`outflow_class` maps regardless
  of `decision_seq` order. **Order-independent — confirmed by code reading and by fault-injection.**
- **Four sets exhaustive (verified against current source):**
  - out-leg reconciled iff `outflow_class.contains_key(out)` (`:644`, → `Op::Dispose/GiftOut/Donate`) OR
    `links.contains_key(out)` (`:553`, → `Op::SelfTransfer`). No other decision reconciles a raw
    `TransferOut` — `LotSelection` only targets an already-honoring op, never a raw out. **Complete.**
  - in-leg reconciled iff `inbound_class.contains_key(in)` (`:591`, → `Op::IncomeInbound/GiftReceived/
    SelfTransferInbound`) OR `consumed_ins.contains(in)` (`:549`, → link-consumed `Op::Skip`).
    `ManualFmv`/`ReclassifyIncome` validate an `Income` target, never a `TransferIn`. **Complete.**
- **Bad-target validation uses the EFFECTIVE payload** (`applied.get().unwrap_or(raw)`, `:732-737`), so a
  `SupersedeImport`/`ClassifyRaw` that turned the in-leg into an Acquire → `in_ok = false` → whole
  decision excluded (bad target), never a silent skip. Superseded/reclassified legs are covered by the
  combination of effective-payload type-check + the I1 guard.
- **Removal is atomic & non-interfering** (`:799-800`): removes BOTH ids; each leg belongs to ≤1 accepted
  passthrough (duplicate detection `:752-764`), so removing one passthrough's ids never disturbs another.
- **Excluding the passthrough genuinely surfaces the taxable event** (traced + KAT-proven): out-leg
  removed → `build_op` skips the (now-absent) `passthrough_skip` check → `outflow_class.get(out)` →
  `Op::Dispose`, gain computed (`disposals[0].legs[0].gain == 140.00` in the KAT). Symmetric for income
  (`income_recognized[0].usd_fmv == 500.00`). The non-overlapping sibling leg reverts to
  `UnknownInbound`/`PendingOut` (safe return-to-unreconciled, never a silent drop).

### 2. [CLI unproposed-pair force-apply] — CANNOT bypass I1. NOT Critical.
`cmd/reconcile::apply_self_transfer_passthrough` (`reconcile.rs:281-303`) appends the decision for ANY
pair via `append_and_save`, which does NOT re-project (`:25-33`) — so append always succeeds. The entire
defense is at projection, and it holds:
- **Worst case A — force-apply on a pair whose out is ALREADY a `Dispose`:** the `ReclassifyOutflow`
  already populated `outflow_class`; when the (later-seq) passthrough is collected it enters
  `passthroughs`, then the I1 guard fires (`outflow_class.contains_key(out)`) → excluded → the disposal is
  recognized. Because the guard runs after all maps are built, the append ORDER is irrelevant (this is the
  reverse of the KAT's order, and the guard treats both identically).
- **Worst case B — force-apply, THEN append a `Dispose` on the out later (no void):** exactly the shipped
  KAT `passthrough_then_dispose_on_out_leg_conflict_disposal_recognized` — conflict raised, disposal
  recognized.
- **Bad target (out is not a `TransferOut`, e.g. a typo'd ref):** collection bad-target arm (`:740-751`)
  raises `DecisionConflict`, whole decision excluded, neither leg skipped.
- The only "collapse" a force-apply achieves on its own is dropping TWO genuinely-**unreconciled** legs
  (a pending out + an unknown-basis in) — which is precisely the user-confirmed determination the design
  authorizes (neither leg is a recognized taxable event at that point). No hidden taxable event.
**Assessment: fully guarded. Not a finding.**

### 3. [DROP ★] both legs `Op::Skip` — VERIFIED
`build_op` returns `Op::Skip` at the top for any id in `passthrough_skip` (`resolve.rs:187-189`), before
the `TransferOut`→`PendingOut` fallthrough. `Op::Skip => {}` is a true no-op in the fold (`fold.rs:1220`).
KAT `passthrough_drop_skips_both_legs_non_taxable`: no lot, holdings empty, income/disposals/removals
empty, `pending_reconciliation` empty, no `UnknownBasisInbound`/`UnmatchedOutflows` blocker,
`conservation_report.balanced`, `sigma_held == 0`. Skip-only-in fault → RED (see table).

### 4. False-match safety (structural) — VERIFIED
`self_transfer_match_plan` (`session.rs:413-...`): candidate ins enumerated ONLY from
`BlockerKind::UnknownBasisInbound` joined to the raw `TransferIn` via the event index (`:433-457`);
candidate outs ONLY from `state.pending_reconciliation` (`:461-...`). An already-classified income /
self-transfer-in carries no `UnknownBasisInbound`, and a reclassified/linked out is not pending — both
structurally excluded (CLI KAT `self_transfer_match_excludes_reconciled_legs`: classify-in as Income +
reclassify-out as Dispose ⇒ 0 proposals). Also: a previously-dropped leg projects to `Op::Skip`, so it
carries no blocker and is not pending → never re-proposed (no double-drop). Ambiguity (a leg in >1 pair)
is flagged, never auto-picked (`:...` `in_count`/`out_count`). Persists nothing — CLI preview asserts
`event_count` unchanged, and the TUI cancel KAT asserts the vault bytes are **byte-identical** after
`open_app` + `m` (matcher run) + cancel. Invariants 3 & 4 pinned.

### 5. Void surface (I2) — VERIFIED
`is_revocable_payload` gains `EventPayload::SelfTransferPassthrough(_)` (`form.rs:908`);
`summarize_void_payload` gains a real arm ("SelfTransferPassthrough", "drop in … + out …")
(`main.rs:2665-2674`) — no "?" render. TUI KAT `kat_e2e_match_self_transfers_drop_then_void_re_exposes_both`
green: the DROP appears in the void list with the real tag, voids to re-expose out→pending and
in→`UnknownBasisInbound`, and leaves NO `DecisionConflict`.

### 6. RELOCATE reuse — VERIFIED
Both CLI (`main.rs:1180-1197`, `link_transfer(out, InEvent(in))`) and TUI
(`handle_match_self_transfers_modal_key` Relocate arm → `persist_link_transfer` with
`TransferLink{out, InEvent(in)}`) route to the EXISTING path — no RELOCATE core reinvented. E2E
`kat_e2e_match_self_transfers_relocate_lands_coins_in_dest`: destination wallet B holds 100_000, zero
disposals.

### 7. Matcher criteria — VERIFIED, no foot-gun
`tol = max(out.fee_sat.unwrap_or(0), ceil(0.005·principal))` with correct integer ceil
`(p+199)/200` (`session.rs`); `txid_match` (both `Some` & equal) relaxes the amount check but not
ambiguity; window is a *directional* `0..=2` whole-days (DROP: out on/after in; RELOCATE: in on/after
out) — faithful to the spec's determination rule and deliberately conservative (a clock-skewed pair is
under-proposed → manual reconcile, never a silent mis-collapse). All arithmetic is bounded i64 (sat
amounts ≪ i64::MAX; `.abs()` on the signed diff) — no overflow/panic. `from_unix_timestamp(0).unwrap()`
is infallible. Deterministic sort by (out_date, out_event, in_event).

### 8. SemVer / serde — VERIFIED
Additive `EventPayload::SelfTransferPassthrough` variant with the forward-only old-binary-fails-loud doc
(mirrors `ReclassifyIncome`). `fingerprint == None` via the `_ => return None` catch-all
(`persistence.rs:96`) — KAT `self_transfer_passthrough_decision_has_no_fingerprint`. Serde round-trip KAT
green. No other exhaustive `EventPayload` match mishandles the variant: the code compiles (so every
non-catch-all match was updated), `persistence::fingerprint`'s catch-all correctly yields None, and the
remaining TUI catch-alls (`classify_raw_variant_label:4699`, `summarize_imported_payload:6072`) only ever
receive imported/built payloads, never a decision variant — no stray "?".

### 9. No regression — VERIFIED
`cargo clippy -p btctax-core -p btctax-cli -p btctax-tui-edit --tests` clean (no warnings). Targeted
suites green: core `passthrough` (9), CLI `match_self_transfers_cli` (3) + `reconcile self_transfer` (4),
TUI `match_self_transfers` E2E (4) + `kat_p2_stp` persist strict-prefix (1). Fault-injections all
restored byte-for-byte.

### 10. Dead code / doc — clean; no `TODO`/`FIXME`; the new `m` Browse binding is a fresh key (no
collision — clippy would flag an unreachable arm).

---

## Nits (non-blocking)

### [N1] NIT — CLI "writes-nothing" asserts event count, not bytes
`crates/btctax-cli/tests/match_self_transfers_cli.rs:124-152` proves the preview/`--dry-run` write
nothing via `load_all(...).len()` unchanged, whereas Invariant 3 is phrased "byte-identical." Not a gap:
the helper opens a read-only `Session` and never saves, and the TUI cancel KAT already pins the
byte-exact property (`std::fs::read` before/after, covering the matcher run). Consider a byte-exact
assertion in the CLI test too for symmetry; purely test-strength.

### [N2] NIT — Phase-2 confirm of an `[AMBIGUOUS]` proposed pair does not re-echo the flag
`main.rs` `MatchSelfTransfers` Phase 2 uses the proposal's suggested action silently when a proposed pair
is named with `--in/--out`, even if that proposal was flagged `ambiguous` in Phase 1. This is spec-
compliant (explicit `in`+`out` refs ARE the disambiguation), and the user must have seen the flag in the
preview — but echoing "(was ambiguous)" on confirm would harden the surface against a copy-paste of the
wrong ref. Cosmetic.

---

## Confirmations (pressure-tested, passed)
- I1 guard: separate post-collection loop, four sets exhaustive, order-independent, atomic removal,
  taxable-event-survives — proven by both fault-injection directions.
- DROP = `Op::Skip` both legs; skip-only-in is a real hazard (pending + mis-consume) caught by KATs.
- CLI force-apply cannot hide a taxable event (projection guards are append-order-independent).
- False-match candidate sets exclude all reconciled legs; matcher persists nothing (byte-exact in TUI).
- Void surface complete (both TUI arms + KAT). RELOCATE routes to the existing `link_transfer`.
- Matcher criteria computable & conservative; SemVer additive/forward-only; fingerprint None; no stray
  exhaustive-match mishandling.

**Ship gate: GREEN (0 Critical / 0 Important). Cleared to ship.**
