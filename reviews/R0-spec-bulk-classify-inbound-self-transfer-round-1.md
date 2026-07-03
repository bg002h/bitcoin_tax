# R0 spec review — bulk-classify-inbound-self-transfer (round 1)

**Artifact:** `design/SPEC_bulk_classify_inbound_self_transfer.md`
**Baseline:** branch `feat/bulk-classify-inbound-self-transfer` @ `412d944` (main == `569a5ee`).
**Reviewer:** independent architect (did NOT author). All claims grounded against current source.
**Gate:** R0 — Critical/Important block implementation.

## Verdict: **0 Critical / 1 Important / 3 Minor / 1 Nit — BLOCKED**

The mirror is, in the main, faithful and well-grounded: every cited line anchor is accurate
(`bulk_link_transfer_plan` session.rs:333, `self_transfer_match_plan` session.rs:421-453,
`bulk_link_plan`/`apply_bulk_link_transfer` reconcile.rs:214/229, `persist_bulk_link_transfer`
persist.rs:393 incl. the [bulk-I1] mid-batch `rollback`, `persist_classify_inbound` persist.rs:122,
`InboundClass::SelfTransferMine` event.rs:140, `fmv_of` core/price.rs:13, `ClassifyInbound` event.rs:152).
Core is genuinely UNCHANGED — the fold arm `Op::SelfTransferInbound` (fold.rs:958-1013) already projects
each appended `ClassifyInbound{SelfTransferMine{None,None}}` to a **non-taxable $0-basis lot, `basis_pending:
false`**. The honest-floor design, the two-phase CLI, the atomic one-save + mid-batch rollback, key `B`
(unbound; only `A`/`G`/`b` bound — verified no `Char('B')` in main.rs), and void-reversibility all hold.

**But** the candidate enumeration copies the WRONG precedent for a decision-APPENDING flow, so the spec's
central structural-safety claim (G1) is factually wrong in a reachable case. That gates the gate.

---

### [I1] IMPORTANT — candidate enumeration omits the `already_classified` filter; sweeps already-classified inbounds that still carry `UnknownBasisInbound` → spurious HARD `DecisionConflict`
`design/SPEC_...md:46-51, 77-83, 156-158` (grounding + D1 + G1) vs. `main.rs:2139-2171`
(`open_classify_inbound_flow` filter 3) and `fold.rs:920-935, 966-978`.

**Defect.** The spec grounds the candidate set on *"`TransferIn` events still flagged
`UnknownBasisInbound` … the EXACT pattern `Session::self_transfer_match_plan` uses"* (session.rs:437-453),
and asserts (G1, line 156-158; grounding, line 50-51): *"An already-classified inbound (Income/Gift/
self-transfer-in) … is no longer `UnknownBasisInbound`, so it can NEVER be swept."*

That equivalence — *"still flagged `UnknownBasisInbound`" == "unclassified"* — is **false**. `UnknownBasisInbound`
is re-emitted from the fold for THREE classified states, each on a raw `TransferIn` that already owns a
non-voided `ClassifyInbound` decision:
- **Gift, case 4** — `GiftReceived{donor_basis:None, donor_acquired_at:None}` → `UnknownBasisInbound`
  (`fold.rs:931-935`). Reachable: `fmv_at_gift` is mandatory but both donor fields are `Option` and may be `None`.
- **Gift, case 3 price-missing** — `donor_acquired_at:Some(d)` with no price at `d` → `UnknownBasisInbound`
  (`fold.rs:920-924`).
- **Self-transfer-in, wallet-less** — `SelfTransferMine` on a wallet-less `TransferIn` → Hard
  `UnknownBasisInbound` (`fold.rs:966-978`).

All three are `Op::GiftReceived`/`Op::SelfTransferInbound` in `build_op` (so `inbound_class` DOES contain
them, resolve.rs:271-300) yet **still carry `UnknownBasisInbound`**. The spec's enumeration keys ONLY on
(blocker == `UnknownBasisInbound`) + (payload is `TransferIn`) — it matches `self_transfer_match_plan`,
which is a *matcher* that never appends a `ClassifyInbound`, so it does not need the guard. The **bulk-STI
flow DOES append a `ClassifyInbound` per row**, so its correct precedent is the single-item opener
`open_classify_inbound_flow`, which adds **filter 3**: exclude any `TransferIn` already targeted by a
non-voided `ClassifyInbound` (main.rs:2139-2152, 2168-2171 — the comment there states the exact reason:
*"adding a second would fire DecisionConflict; FIRST-WINS"*).

**Consequence (why it gates).** Sweep such a deposit and the bulk appends a duplicate
`ClassifyInbound(SelfTransferMine)`. Decisions sort ascending by `decision_seq` (resolve.rs:379,514) →
the pre-existing gift/self-transfer wins (first-wins, resolve.rs:582-592), so **the tax number is
preserved** (no under-report — this is why it's Important, not Critical). But the duplicate fires a
**`DecisionConflict`, which is `Severity::Hard`** (state.rs:74,83) → it **blocks `compute_tax_year` for
the whole return**. The user is told *"Classified N inbound deposits…"* (false success, over-count) while
their return silently stops computing until they hunt down and `v`-void the phantom. A bulk op advertised
as touching only unclassified deposits converts a known/recoverable gift-basis-unknown state into a
return-blocking hard conflict. The KAT `bulk_sti_plan_selects_unknown_inbounds_in_frame` as described
("classified/matched inbounds NOT selected") would MISS this if its fixture only uses an Income-classified
inbound (Income re-fires `FmvMissing`, not `UnknownBasisInbound`, fold.rs:854 — correctly excluded), so
the false claim would ship green.

**Note — the income headline IS safe.** A genuine income deposit can *never* be a candidate: classified
Income emits `FmvMissing` (fold.rs:854), never `UnknownBasisInbound`, and even a hypothetical sweep is
first-wins-protected. The tax-safety spine (can't zero-basis an income deposit) holds. The gap is
narrowly: already-classified **gift-basis-unknown** and **wallet-less self-transfer** inbounds.

**Fix.** Mirror `open_classify_inbound_flow` (filter 3), NOT `self_transfer_match_plan`, in
`bulk_self_transfer_in_plan`: build the `already_classified` set (non-voided `ClassifyInbound` targets,
accounting for `VoidDecisionEvent`) and exclude those `in_event`s from `included`. Correct G1 and the
grounding bullet to say the set must exclude *both* not-yet-`UnknownBasisInbound` AND
already-`ClassifyInbound`'d inbounds. Add a KAT whose fixture includes a gift-case-4 inbound (and a
wallet-less one) asserting they are NOT in `included`.

---

### [M1] MINOR — the shown `persist_bulk_self_transfer_in` body drops the empty-guard the mirror places BEFORE the snapshot
`design/SPEC_...md:116-129` vs. `persist.rs:399-403`.

The mirror `persist_bulk_link_transfer` refuses an empty batch *before* `snapshot()`:
`if out_events.is_empty() { return Err(PersistError::NoChange(..)) }` (persist.rs:399-403). The spec's
code block goes straight to `let pre = session.snapshot()?;` and only mentions the guard in prose
(line 131). As written it would `snapshot → loop-nothing → save_or_rollback` (a no-op save), not the
`NoChange` the mirror + the `persist_bulk_sti_refuses_empty` KAT require. Benign (idempotent save) and the
KAT forces the fix, but the shown body contradicts the mirror. **Fix:** show the pre-snapshot empty guard
verbatim from persist.rs:399-403.

### [M2] MINOR — wallet-less candidates are swept but create no lot + re-fire `UnknownBasisInbound`; contradicts the E2E's "included → lots created" claim
`design/SPEC_...md:77-83, 177-179` vs. `fold.rs:966-978`; cf. `self_transfer_match_plan` skips wallet-less
ins (session.rs:493).

A wallet-less raw `TransferIn` reaches `Op::UnknownInbound` (fold.rs:815 — no wallet check) → is a
candidate. Under the "Any" wallet filter it is included; once swept, `Op::SelfTransferInbound` hits the
wallet-missing corner (fold.rs:966-978) → **no lot, re-fires Hard `UnknownBasisInbound`**. So the E2E
`bulk_sti_then_lots_created` invariant ("included inbounds create non-taxable $0-basis lots + clear
`UnknownBasisInbound`") is false for such a row, and the bulk over-counts. This limitation is inherited
from the single-item flow (which also lists wallet-less ones), so it's Minor. **Fix:** either skip
`wallet.is_none()` rows in the enumeration (as the matcher does, session.rs:493) or state explicitly that
wallet-less rows won't create a lot and keep them out of the "created N lots" assertions.

### [M3] MINOR — the frame-filter helper's `Frame` provenance should be pinned
`design/SPEC_...md:64` ("`Frame` reused from bulk-link").

`Frame` (`All/Year/Range`) is defined in the bulk-link D1 and lives in `btctax-cli` (used in
`bulk_link_transfer_plan`, session.rs:368-372). The spec reuses it but doesn't pin its module path; make
`BulkStiFilter` reuse the exact `crate::Frame` (not a re-declared twin) so the `in_frame` closure is
byte-identical to session.rs:368-372. Trivial, but worth stating to avoid a divergent copy.

### [N1] NIT — field-name drift `total_usd_fmv_floor` vs mirror's `total_usd_value_floor`
`design/SPEC_...md:72` vs `session.rs:396` (`BulkLinkPlan.total_usd_value_floor`).

Intentional (FMV-given-$0-basis vs value-reclassified), and the inversion is the right honest number. No
change needed; flagged only so a reviewer doesn't read it as an accidental rename of the mirror field.

---

## Confirmed sound (no finding)
- **Core UNCHANGED / reuse faithful:** `ClassifyInbound{SelfTransferMine{None,None}}` per row → non-taxable
  $0-basis lot, `basis_pending:false`, no `IncomeRecord` (fold.rs:958-1013). Cycle A semantics preserved.
- **Enumeration feasible:** `self_transfer_match_plan` (session.rs:421-453) already joins the
  `UnknownBasisInbound` blocker set (`b.event`) to the raw `TransferIn` via the event index — the helper
  exists and the blocker carries the in-event id. (The join is right; only filter 3 is missing — [I1].)
- **fmv_of + honest floor:** `fmv_of(&prices, date, sat) -> Option<Usd>` (core/price.rs:13); floor = Σ of
  `Some` + `missing_price_count` → exact `$X` or `≥ $X (N unavailable)`. Correct API, no false-exact/blank.
- **[bulk-I1] mid-batch rollback:** the spec's `if let Err(e) = append_decision(..) { return
  Err(rollback(session,&pre,e.into())) }` (NOT `?`) matches persist.rs:412-420 exactly; CLI `apply` uses
  `?`/session-discard, matching reconcile.rs:237-247. One save each. Correct atomicity split.
- **Reversibility:** each bulk `ClassifyInbound` is voidable; void drops it from `inbound_class` →
  `Op::UnknownInbound` → re-exposes `UnknownBasisInbound` (build_op resolve.rs:301-303). Confirmed.
- **Key/clap surface:** `B` free; `BulkLinkTransfer` (main.rs:362) + `ClassifyInboundSelfTransfer`
  (main.rs:240) exist; a new `BulkClassifyInboundSelfTransfer` kebab variant won't collide.
- **SemVer/lockstep:** btctax-core UNCHANGED is correct (no new variant); no `docs/manual/`, no GUI crate.

## Required to reach R0-GREEN
Fold **[I1]** (add the `already_classified` filter + correct G1/grounding + the gift-case-4/wallet-less
KAT). [M1]-[M3] are strongly recommended in the same pass. Re-review after the fold (§2 loop) — including
that the tightened enumeration KAT actually exercises a gift-basis-unknown inbound.
