# R0 spec review — bulk-classify-inbound-self-transfer (round 2)

**Artifact:** `design/SPEC_bulk_classify_inbound_self_transfer.md` (post round-1 fold)
**Baseline:** branch `feat/bulk-classify-inbound-self-transfer`; main == `569a5ee`. All claims re-grounded.
**Reviewer:** independent architect (did NOT author). Round-1 review: `...round-1.md`.

## Verdict: **0 Critical / 0 Important / 2 Minor / 1 Nit — R0-GREEN**

The round-1 gate finding **[I1] is resolved and the fix is structurally complete.** Two non-blocking
Minors remain (both from incomplete propagation of the fold); neither gates. Cleared to implement.

---

## Round-1 folds — verification

### [I1] RESOLVED — candidate enumeration now excludes already-classified + wallet-less inbounds
D1 Selection (`SPEC:87-97`), grounding (`SPEC:47-61`), G1 (`SPEC:171-177`) now specify: enumerate
`UnknownBasisInbound`-flagged `TransferIn`s, then **exclude** (a) any `in_event` already targeted by a
non-voided `ClassifyInbound` (mirror `open_classify_inbound_flow` filter 3, `main.rs:2139-2171`) and
(b) any `wallet.is_none()` row.

**Completeness — confirmed airtight, and BETTER than the "three states" gloss implies.** The safety does
not rest on having enumerated every re-emission site; filter-3 is a *structural catch-all* (any TransferIn
with a live `ClassifyInbound` is dropped, regardless of which fold arm re-fired the blocker). I re-swept
ALL four `UnknownBasisInbound` emission sites in `fold.rs`:
- `fold.rs:818` `Op::UnknownInbound` (raw, unclassified TransferIn) → the intended target set. ✓ included
- `fold.rs:921` gift case-3 price-missing, `fold.rs:932` gift case-4 → classified TransferIn → filter-3 drops. ✓
- `fold.rs:973` wallet-less `SelfTransferMine` → classified TransferIn → filter-3 **and** wallet-less drop. ✓
- `fold.rs:233` **(a 4th site the spec doesn't list)** "removal consumes a basis-pending lot" — its
  `blocker.event` is the **DISPOSAL/removal event**, NOT a TransferIn, so **filter-2 (raw-TransferIn)
  excludes it**. Not a pollution vector; no spec change needed. Worth noting only to confirm the sweep is
  exhaustive.

The corrected line anchors all verify: gift-case-4 `fold.rs:931`, gift-case-3 `fold.rs:920`, wallet-less
`fold.rs:966`, income→`FmvMissing` `fold.rs:854`, matcher wallet-less skip `session.rs:493`, filter-3
`main.rs:2139-2171`. The income-safety spine (Income fires `FmvMissing`, never `UnknownBasisInbound`, so a
genuine income deposit is never a candidate) is stated correctly.

**VoidDecisionEvent accounting — the coordinator's focus question — is CORRECT as specified.** Traced
against the mirrored precedent (`main.rs:2126-2152`): `voided` is built from `VoidDecisionEvent.
target_event_id` (which is the *decision event's* `EventId` — how `void` targets a decision,
`event.rs:200` + `reconcile.rs:107-113`); `already_classified` then collects `ci.transfer_in_event` **only
for ClassifyInbound decisions whose own `e.id` is NOT in `voided`** (`!voided.contains(&e.id)`,
`main.rs:2144`). So voiding a `ClassifyInbound` drops it from `already_classified` → its `transfer_in_event`
re-enters the candidate set. Following the cited precedent, **a voided classification DOES re-expose the
inbound as a bulk candidate.** ✓ (See [M-r2-1] for a prose-precision wrinkle in the gloss — not a
behavioral defect given the precedent cite.)

### [M2] RESOLVED — wallet-less exclusion added (`SPEC:91-92`); `wallet` is now always `Some` for survivors, so the E2E "included → lots created" claim is sound. Mirrors `session.rs:493`. ✓
### [M1] RESOLVED — persist body shows the pre-snapshot empty guard (`SPEC:132`), mirroring `persist.rs:399-403`. ✓ (see Nit on the elided payload).
### [M3] RESOLVED — `BulkStiFilter.frame: crate::Frame` (`SPEC:74,93`) pins the exact bulk-link type. ✓
### [N1] ACKNOWLEDGED — `total_usd_fmv_floor` naming noted intentional (`SPEC:82`). ✓

---

## New/residual findings (non-blocking)

### [M-r2-1] MINOR — void-accounting prose is imprecise, and no KAT pins the void→re-candidate path
`SPEC:89-90` ("build the `already_classified` set from `ClassifyInbound` targets minus `VoidDecisionEvent`
targets") and KATs (`SPEC:200-203`).

The shorthand "ClassifyInbound **targets** minus VoidDecisionEvent **targets**" conflates two disjoint id
spaces: a `ClassifyInbound` target is a *TransferIn* id (`ci.transfer_in_event`); a `VoidDecisionEvent`
target is a *decision* id (`void.target_event_id`). A literal set-minus of the two is a no-op, and an
implementer who codes the gloss instead of tracing the precedent would OVER-exclude — a voided
classification's `transfer_in_event` would stay in `already_classified`, so the inbound could not be
re-bulk-classified after a void. This is the *conservative* (never double-classifies) direction, not
tax-unsafe, and the authoritative instruction ("mirror `open_classify_inbound_flow`'s filter 3,
`main.rs:2139-2171`") is correct — so the spec, read as intended, is right. But: (a) reword to the
precedent's actual operation ("for each ClassifyInbound **decision** whose own event-id is not the target
of a `VoidDecisionEvent`, collect its `transfer_in_event`"), and (b) the `bulk_sti_then_void` E2E
(`SPEC:202`) asserts only "re-exposing `UnknownInbound`" (a projection fact) — extend it to assert the
re-exposed inbound **reappears in a fresh `bulk_self_transfer_in_plan().included`** (the filter-3 fact).
That KAT is what actually locks the coordinator's question.

### [M-r2-2] MINOR — stale self-contradictory sentence in the Idempotence blurb
`SPEC:156-157`: *"candidates are only `UnknownBasisInbound` inbounds (already drops classified/matched
ones), so re-running never double-classifies."*

This is the exact framing [I1] corrected, left un-updated in the Atomicity section. It now contradicts the
corrected grounding/G1 (which establish that `UnknownBasisInbound` does NOT drop classified inbounds —
that is why filter-3 was added). The idempotence guarantee is real, but it holds **because of filter-3**
(a re-run finds the first batch's `ClassifyInbound` in `already_classified`), not because "candidates are
only `UnknownBasisInbound` inbounds". Reword to attribute idempotence to the filter-3 exclusion so the
spec isn't internally inconsistent.

### [N-r2-1] NIT — persist sketch shows `PersistError::NoChange` bare
`SPEC:132` vs `persist.rs:400` (`PersistError::NoChange(btctax_cli::CliError::Usage("…".into()))`). The
variant carries a `CliError` payload; the sketch elides it. Consistent with the block's short-name sketch
style and the mirror line is cited, so cosmetic only.

---

## Bottom line
[I1] (the gating finding) is fully and correctly resolved; the filter-2 + filter-3 + wallet-less design is
provably complete against every `UnknownBasisInbound` emission site, and the void accounting is correct as
specified. **0 Critical / 0 Important → R0-GREEN.** [M-r2-1]/[M-r2-2]/[N-r2-1] are polish — fold them
(especially the void→re-candidate KAT) opportunistically in Task 1/2; they do not block implementation.
Per §2, re-review after any fold.
