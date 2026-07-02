# R0 — Architect review: SPEC_tui_edit_chunk2a.md (round 1)

**Artifact:** `design/SPEC_tui_edit_chunk2a.md` (untracked working-tree draft)
**Baseline verified against:** HEAD `096a07b` ("Merge mutating-TUI chunk 1") — matches the spec's
claimed baseline. Every citation below re-checked by the reviewer against current source, not
taken from the spec.
**Reviewer stance:** independent — payload shapes, fire sites, resolve precedence, CLI arms,
gate allowlist, and chunk-1 substrate all re-read from source.
**Date:** 2026-07-02

**Verdict: NOT GREEN — 0 Critical, 7 Important, 8 Minor, 5 Nit.**

The two pre-filters — the highest-stakes items — are **sound in the dangerous direction**: the
compound inbound filter can never offer an already-decided target (no DecisionConflict-by-
construction), and Claim B (pending_reconciliation is inherently post-filtered) is confirmed
against `build_op`. The payload construction is exact against `event.rs`, the KAT-G1 token claim
is verified (no gate change needed), and the test plan's strict-prefix form is structurally right.
The blocking findings are: a factually wrong failed-save/retry story (first-wins inverted; the
Hard DecisionConflict consequence omitted; KAT-S2's growth assertion wrong), a self-contradictory
EditorApp state layout that risks the exact R0-M4 Esc-fall-through class of bug, two internal
spec contradictions (spend amount label; gift-modal donee omission), two "honest status" messages
that recommend remedies the engine rejects with Hard blockers, a silent-clean-status hole for the
gift price-unavailable re-fire case, and a wrong `decision_seq` expectation formula in KAT-P2a/P2b.

---

## 1. Pre-filter verification (the calling mandate's highest priority)

### 1.1 Claim A — the compound inbound filter vs the FOUR fire sources: CONFIRMED

All `UnknownBasisInbound` fire sites in current `fold.rs` re-located and re-read:

| # | Fire site (verified) | Blocker `event` | Spec cite | Filter outcome | Correct? |
|---|---|---|---|---|---|
| 1 | `Op::UnknownInbound`, fold.rs:815–822 | the TransferIn's `EventId` (`eff.id`) | 815–822 ✓ | passes all three filters → **listed** | ✓ actionable (no `ClassifyInbound` exists, else it would not have resolved to `UnknownInbound`) |
| 2 | `Op::GiftReceived` case 4 (both donor fields `None`), fold.rs:929–937 | the TransferIn's `EventId` | 929–935 ✓ | filter 3 **excludes** (non-voided `ClassifyInbound` targets it) | ✓ — a second `ClassifyInbound` would fire `DecisionConflict` (resolve.rs:554–561, FIRST-WINS, verbatim as spec quotes) |
| 3 | `Op::GiftReceived` case 3 (`donor_acquired_at=Some(d)`, price unavailable at `d`), fold.rs:913–927 | the TransferIn's `EventId` | 918–923 ✓ | filter 3 **excludes** | ✓ same reason |
| 4 | removal consumes a basis-pending lot, fold.rs:230–237 | the **removal** event (a raw `TransferOut` reclassified GiftOut/Donate) — never a TransferIn | 232–236 ✓ | filter 2 **excludes** (raw payload is not `TransferIn`) | ✓ |

Additional adversarial checks (all pass):

- **Voided-classification row re-listed:** a TransferIn whose only `ClassifyInbound` is voided
  resolves to `Op::UnknownInbound` (resolve skips voided decisions at resolve.rs:487); the spec's
  `voided` set removes that classify from filter 3's scan → the row **stays listed** ✓, and a new
  `ClassifyInbound` after a void is accepted cleanly by resolve ✓.
- **Void-of-void:** resolve treats `VoidDecisionEvent` as a NON-revocable target (resolve.rs:301–306
  — void of a void is inert, conflict-flagged). The spec's simpler "collect all void targets" set
  agrees with resolve *for the `ClassifyInbound`-membership question* (a void of a ClassifyInbound
  is always honored; voids of other things never affect this filter). No over-inclusion vector ✓.
- **Consumed-by-TransferLink TransferIn:** resolves to `Op::Skip` → no blocker fires → never in the
  list ✓.
- **No over-inclusion path found.** The filter cannot offer a target whose `ClassifyInbound` would
  conflict. The exclusion of the incomplete-gift cases (2 and 3) is exactly right — those need a
  VOID first, and the spec **does** state that remedy (Pre-filter §, D2 post-effect note, and a
  FOLLOWUPS entry). See M4 for the UX-gap rating and I4/I5 for defects in the *wording* of the
  remedy and the case-3 status.
- **One under-inclusion divergence found** (raw vs. effective payload) — see M2. Safe direction.

### 1.2 Claim B — outflow list: CONFIRMED

`build_op` for `TransferOut` (resolve.rs:201–250; spec cites 200–249 ✓): `links` → `Op::SelfTransfer`;
else `outflow_class` → `Op::GiftOut`/`Op::Donate`/`Op::Dispose`; only the residual falls to
`Op::PendingOut`, which is the sole site pushing `pending_reconciliation` (fold.rs:729–734).
`pending_reconciliation` therefore contains ONLY unreclassified, unlinked TransferOuts. No
client-side filter needed ✓. The advisory `UnmatchedOutflows` fires only in the same arm
(fold.rs:736–740), so the D3 "blocker clears after reclassify" claim is correct ✓.

### 1.3 Stale-list race — PARTIALLY WRONG mechanism claim → see M1

The duplicate arm is real and detectable (duplicate `ReclassifyOutflow` → `DecisionConflict`
attributed to the NEW decision's id, resolve.rs:606–614 — the D4 step-2 check by returned `id`
works). The "**or the target is now SelfTransfer**" arm is contradicted by source: resolve.rs:600–605
explicitly treats a link+reclassify overlap as *precedence, not conflict* — no blocker fires, the
link silently wins in `build_op`. In that arm the spec's honesty check would report plain success
while the decision is inert. Unreachable in chunk 2a (VaultLock held for the editor's lifetime,
persist.rs:8–11; no link flow exists), hence Minor, but the claim must be corrected before chunk 3
inherits it.

---

## 2. Verification log (what was checked and found clean)

- **Payload shapes** — exact against event.rs:104–139: `InboundClass::Income{kind,fmv,business}`,
  `InboundClass::GiftReceived{donor_basis,donor_acquired_at,fmv_at_gift}` (fmv_at_gift non-optional ✓),
  `ClassifyInbound{transfer_in_event,as_}`, `ReclassifyOutflow{transfer_out_event,as_,
  principal_proceeds_or_fmv,fee_usd,donee}`, `OutflowClass::Dispose{kind}/GiftOut/Donate{appraisal_required}` ✓.
  D3's payload-build match arms are exactly the CLI's (main.rs:848–862) ✓.
- **IncomeKind cycle** — Mining→Staking→Interest→Airdrop→Reward matches the enum declaration order
  (event.rs:29–35) and `parse_income_kind` (eventref.rs:122–131) ✓.
- **`now` injection** — matches the CLI discipline (reconcile.rs:4, `now` parameter throughout);
  `UtcOffset::UTC`, `wallet: None` match `append_and_save` (reconcile.rs:26–34; spec cites 26–33 ✓).
- **Persist fns** — mirror `classify_inbound` (reconcile.rs:39–53 ✓ exact) and `reclassify_outflow`
  (reconcile.rs:59–80; spec cites 60–80 ✓). `append_decision` at persistence.rs:238–262 ✓ exact.
  `CliError: From<CoreError>` conversion exists (same `?` pattern as reconcile.rs:31) ✓.
- **Gate token claim — VERIFIED:** `"append_"` is in `persist_only_tokens` at
  crates/btctax-tui-edit/src/edit/persist.rs:235, alongside `conn(`/`save(`; edit/persist.rs is the
  allowlisted file (persist.rs:324–332). KAT-G1 needs no change ✓. Note the gate also bans `cmd::`
  in NON-test regions everywhere (persist.rs:204–213) — the KATs' use of `cmd::import::run`/
  `cmd::inspect::*` is test-region-only and permitted (KAT-P1 already uses `cmd::init::run` in test
  code, persist.rs:93) ✓.
- **Validation parity vs CLI arms** (main.rs:807–863): Income `kind` required / `fmv` optional /
  `business` plain flag **default false** (main.rs:224–225) — the spec's default-false toggle is
  correct parity; the required-explicit `--business` is chunk-C's `ReclassifyIncome` only
  (main.rs:301) and does NOT apply here ✓. Gift `fmv_at_gift` required, donor fields optional ✓.
  Outflow `amount` required / `fee` optional / `appraisal` flag / `donee` optional ✓.
  `parse_usd_arg`/`parse_date_arg` cited at eventref.rs:76–83 ✓ exact.
- **Chunk-1 substrate citations:** dispatch order main.rs:79–88 ✓; `handle_modal_key` 146–208 ✓;
  Enter-arm Ok/Err semantics (main.rs:168–198) match the spec's inherited description ✓;
  `FieldBuffer`/`FIELD_CAP=64` (form.rs:13–57) ✓; `MutationModalState` (form.rs:114) ✓;
  `draw_mutation_modal` (draw_edit.rs:236, spec's 232–293 ✓) and `centered_rect` (296–305 ✓);
  `EditorApp.profile_form`/`mutation_modal` (editor.rs:79/84) ✓; `Snapshot{events,state,…}`
  (btctax-tui/src/app.rs:104–111) ✓; `build_snapshot` is `pub`, returns `(Snapshot, i32)`
  (unlock.rs:170) — consistent with existing `Ok((snap, _))` usage ✓; `load_all_ordered`/`RawEventRow`
  with `ordinal` (persistence.rs:335–381; spec's 354–380 ✓); `append_import_batch`
  (persistence.rs:172) ✓; `cmd::inspect::report`/`verify` (inspect.rs:11/29) ✓;
  KAT-S1 chmod-0o500 pattern exists (tui-edit main.rs:1100–1123, incl. a root-skip guard the KAT-S2
  spec should inherit) ✓.
- **FmvMissing fire site for Income-classified inbound:** fold.rs:853–859 (`Op::IncomeInbound`,
  fmv=None; lot created with `basis_pending=true`) — spec citation exact ✓.
- **Scope/right-sizing:** Task 1 (shared list infra + classify-inbound) / Task 2 (reclassify-outflow)
  / Task 3 (whole-diff) is a sane split; 2b/3+ exclusions explicit; no core/CLI changes needed
  (all types + `append_decision` pre-exist) ✓; SemVer MINOR claim correct (additive, existing crate) ✓.

---

## 3. Findings

### Important

**[I1] The failed-save/retry story is factually wrong: first-wins is inverted and the Hard
DecisionConflict consequence is omitted; KAT-S2's growth assertion contradicts it.**
Spec, Hard constraints (lines 51–56): *"retry is safe (… the old orphaned event stays in the log
but **projects cleanly**, and the retry's new decision **takes precedence by higher
decision_seq**)"* — and D4 `persist_classify_inbound` doc: *"The stale first decision is
effectively inert."* Source says the opposite: `ClassifyInbound`/`ReclassifyOutflow` duplicates are
**FIRST-WINS** (resolve.rs:549–564 / 600–617) — the FIRST (failed-save) decision stays **in force**,
the retry's second decision is the excluded one, and the duplicate **fires a Hard
`DecisionConflict`** (severity Hard, state.rs:65–77), which gates `compute_tax_year` via
`TaxYearNotComputable`. Because both decisions carry identical payloads the *classification*
projects the same either way, but the projection is NOT clean — the user is left with a Hard
blocker clearable only by voiding the duplicate (CLI `reconcile void decision|N`; the TUI void flow
is chunk 2b). D4's ReclassifyOutflow comment gets this right; the classify-inbound comment and the
Hard-constraints paragraph contradict it — and each other. Consequently KAT-S2's *"retry → success,
**log grows by 1 row**"* is wrong: after the retry the on-disk log is `pre + 2` decisions (the
in-memory conn already committed decision N+1 before `save()` failed; the retry appends N+2).
**Fix:** rewrite both retry notes to the true semantics (retry appends a duplicate → FIRST-WINS →
Hard `DecisionConflict` on the retry decision; classification content unchanged; clear by voiding
the duplicate — name the CLI as the 2a-interim path). KAT-S2 must assert the TRUE outcome: post-retry
log == `pre + 2`, both payloads round-trip, `DecisionConflict` present in the re-projection, and the
post-persist status surfaces it (extend the D4 step-2 check to this case).

**[I2] EditorApp state layout is self-contradictory; the dispatch guard as named risks the R0-M4
Esc-fall-through bug for the picker/field-form steps.**
D1 normatively adds `pub classify_inbound_list: Option<ListState<InboundListItem>>` and
`pub reclassify_outflow_list: Option<ListState<OutflowListItem>>` to `EditorApp` — while D2/D3
add `classify_inbound_flow`/`reclassify_outflow_flow` whose structs each CONTAIN `list:
ListState<…>`. Two homes for the same state; Task 1's file list names only the flow fields. Worse,
the dispatch layer is specified as a "list" layer guarded by `any_list_open`: if an implementer
keys it to the D1 list fields (or to `step == List`), then in the VariantPicker/IncomeForm/GiftForm/
KindPicker/FieldForm steps NO layer claims the key: `profile_form` is `None`, so keys fall to
Browse screen dispatch, where **`q`/`Esc` quit the app mid-flow** (main.rs:118) — precisely the
"R0-M4 lesson" the substrate exists to prevent (main.rs:60–62). The C2a/C2b Esc-walk scripts would
catch this at test time, but the spec must not ship an ambiguous state design to TDD against.
**Fix:** delete D1's two standalone `Option<ListState<…>>` EditorApp fields; the flows own their
lists. Rename the dispatch layer to the **flow** layer, guarded by
`classify_inbound_flow.is_some() || reclassify_outflow_flow.is_some()`, handling ALL steps of an
open flow (List/picker/forms), with `q` swallowed at every step. Dispatch order: modal → flow →
form → screen. State "at most one flow and at most one modal is `Some`" once, in D1.

**[I3] Spend amount-label: the D3 field table and KAT-V-RO-9 contradict each other.**
D3 table: *"gross proceeds (USD)" for sell; "FMV (USD)" for **spend**/gift/donate*. KAT-V-RO-9:
*"gross proceeds (USD)" when kind=sell/**spend**; "FMV (USD)" when kind=gift/donate*. Both cannot
hold. Source semantics: `Dispose.usd_proceeds` is GROSS proceeds for both Sell and Spend
(event.rs:62; reconcile.rs:55–57 — "`principal` is the gross proceeds (Dispose) or FMV-at-transfer
(Gift/Donate)"). **Fix:** align the D3 table to KAT-V-RO-9 (sell/spend → "gross proceeds (USD)";
gift/donate → "FMV (USD)").

**[I4] Two "honest status" strings recommend remedies the engine rejects with Hard blockers.**
(a) D2 Income-fmv-None status: *"use set-fmv (chunk 2b) or re-classify with an FMV to clear the
FmvMissing blocker"*. `set-fmv` appends `ManualFmv`, whose target is validated at collection time
against the EFFECTIVE payload and must be an **Income event** — a TransferIn target fires
*"ManualFmv targets non-Income event"* → Hard `DecisionConflict`, decision excluded
(resolve.rs:423–470); and `build_op`'s TransferIn arm never consults `manual_fmv` (resolve.rs:251–281),
so it could not work even if accepted. "Re-classify with an FMV" without voiding first is a
duplicate `ClassifyInbound` → Hard `DecisionConflict` (resolve.rs:554–561). Both suggested actions,
as literally worded, CREATE Hard blockers.
(b) D2 GiftReceived-both-None status: *"void and re-classify with donor info, **or use set-fmv
(chunk 2b)**"* — same set-fmv defect.
The E2E KATs (FMV-MISSING, GIFT-UNKNOWN) then lock these strings in as expected values.
**Fix:** the only valid remedy for both cases today is **void the ClassifyInbound, then re-classify**
(CLI `reconcile void` until the 2b void flow ships). Reword both statuses and the KAT expectations;
drop set-fmv from them (or spec 2b's set-fmv flow as Income-event-targeted only).

**[I5] Gift case 3 (donor date given, price unavailable) re-fires `UnknownBasisInbound` but gets
the clean success status — an honesty hole.**
D2's post-effect statuses are keyed off the *payload shape* (fmv=None; both-donor-None). Case 3
(fold.rs:913–927: `donor_basis=None, donor_acquired_at=Some(d)`, no BTC price at `d`) is
data-dependent, re-fires `UnknownBasisInbound` for the same TransferIn, and under D2 as written
receives *"Classified inbound as GiftReceived"* — a silent-failure status for a Hard blocker the
spec's own pre-filter analysis enumerates as path 3. **Fix:** derive the post-persist status from
the RE-PROJECTED state, not the payload: after `build_snapshot`, if `blockers` contains
`UnknownBasisInbound` (or `FmvMissing`) with `event == target`, emit the corresponding honest
message — this handles cases 3 and 4 uniformly and replaces the shape-based dispatch. Extend D4
step 2 (currently only UncoveredDisposal/DecisionConflict) accordingly, and add a KAT: gift with
`donor_acquired_at` outside the bundled price dataset → blocker re-fires → status says so.

**[I6] KAT-P2a/P2b `decision_seq` expectation formula is wrong versus `append_decision`'s MAX
semantics — and the spec'd seed order makes it fail.**
Spec: `post[pre.len()].decision_seq == pre.last().map_or(1, |r| r.decision_seq.unwrap_or(0) + 1)`.
`append_decision` allocates `COALESCE(MAX(decision_seq),0)+1` over ALL decision rows
(persistence.rs:246–250). The KAT-P2a seed is: two MethodElections (seq 1,2), THEN the imported
TransferIn (decision_seq NULL) — so `pre.last()` is the import row and the formula expects
`0+1 == 1` while the engine allocates `3`. The test as spec'd fails for the wrong reason, and the
tempting "fix" (reorder the seed) leaves a fragile, semantically wrong assertion in the safety net.
**Fix:** `expected = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0) + 1`, mirroring
the allocator. Same fix in KAT-P2b.

**[I7] The gift (GiftOut) confirmation modal under-shows the payload: `donee` is displayed only
for Donate.**
The D3 field form collects `donee` for **gift and donate**; the built payload carries it for both;
`Op::GiftOut` consumes it (resolve.rs:222–228) and it lands in removals.csv / Form 8283 surfaces.
But D3's modal rendering shows the donee line only under *"Donate variant additionally shows"* —
a GiftOut with `donee=Some(…)` would persist a field the user never saw at the write gate,
violating the payload-showing guarantee this crate is built on. Task 3's checklist ("donee where
applicable") already contradicts the D3 rendering. **Fix:** show `donee:` (or `donee: (none)`) in
the modal for BOTH gift and donate; assert it in KAT-E2E-DONATE and add the assertion to a gift-path
modal check.

### Minor

**[M1] Stale-race mechanism half-wrong (D3):** the "target is now SelfTransfer" arm does NOT
produce `DecisionConflict` — resolve.rs:600–605 explicitly treats link+reclassify overlap as
precedence (link wins silently in `build_op`). Only the duplicate-ReclassifyOutflow arm fires
(resolve.rs:606–614, not 580–600 as cited; 587–598 is the unknown-target arm). Unreachable in 2a
(editor holds the VaultLock for its lifetime — persist.rs:8–11 — and no link flow exists), so
Minor; but correct the text before chunk 3 (link-transfer) inherits a false safety claim. An
alternative robust check: after re-projection, assert the target now appears in
`disposals`/`removals` (or report otherwise).

**[M2] The inbound pre-filter checks the RAW payload where resolve validates the EFFECTIVE
payload.** Filter 2 matches `e.payload == TransferIn` in `snap.events`; resolve validates
`applied.get(target).unwrap_or(&raw.payload)` (resolve.rs:531–533) — so a row whose effective
payload became TransferIn via `ClassifyRaw` (or an accepted ImportConflict) fires
`Op::UnknownInbound` and is CLI-classifiable, but is invisible in the TUI list. Under-inclusion —
the safe direction (never a conflict), and rare; but it is drift from the engine's own target
rules. **Fix (cheap):** also treat as TransferIn any event targeted by a non-voided
`ClassifyRaw{as_: TransferIn(_)}` in `snap.events`; or document the limitation + CLI path in the
spec's pre-filter section and FOLLOWUPS.

**[M3] KAT-E2E-CI asserts "staking" in the modal but never cycles the kind picker, and the initial
`IncomeKind` selection is unstated.** D2 states initial variant (Income) and D3 initial kind (sell),
but the IncomeKind row's initial value is only implied (Mining, by cycle order). **Fix:** state
initial = Mining; add the `Tab` press to the E2E script (or assert "mining").

**[M4] The pre-filtered incomplete-gift rows: remedy stated, but no in-TUI path in 2a — rate:
acceptable, with one wording gap.** The rows remain visible as blockers in the Compliance tab
(honest), the spec names the remedy (void + re-classify), records the FOLLOWUP, and the CLI path
exists today (`reconcile void`, main.rs:260/867 + `classify-inbound-gift`). The gap: neither the
status string nor the FOLLOWUP says *how* to void in 2a. **Fix:** name the CLI as the interim path
in the D2 status note and the FOLLOWUPS entry (dovetails with I4's rewording).

**[M5] KAT-C2a cites "chunk-1 KAT-C1 (unlock.rs:470–526 pattern)" — wrong file.** KAT-C1 lives at
`crates/btctax-tui-edit/src/main.rs:972–1096`; the editor crate has no `unlock.rs`. Fix the citation.

**[M6] D3's UncoveredDisposal citation points at the wrong fire site, and the blocker may
pre-exist.** fold.rs:712–718 is the `Op::PendingOut` shortfall — the arm that STOPS executing once
the outflow is reclassified. The post-reclassify fire sites are the Dispose/GiftOut/Donate consume
paths (fold.rs ~575–630 and ~965–1095). Also note: an uncovered PendingOut fires UncoveredDisposal
*before* reclassification too, so the D4 step-2 status may surface a pre-existing shortfall — fine
for honesty, but KAT-E2E-UNCOVERED should assert the pre-state (blocker already present) so the
test documents the transition truthfully.

**[M7] No E2E for the donor-provided gift happy path.** The gift flow's only confirmed-write E2E is
the both-None case. Add one KAT: GiftReceived with `donor_basis` (and ideally `fmv_at_gift <
donor_basis` to pin the dual-basis lot: `usd_basis`, `dual_loss_basis`, `donor_acquired_at`
carry-through per fold.rs:903–954) — or record as an explicit FOLLOWUP.

**[M8] D2/D1 empty-list behavior is specified twice, inconsistently.** D2/D3: an empty filtered
list never opens (status + return to Browse). D1: an open empty list renders a placeholder row and
swallows Enter — dead behavior if empty lists never open. Keep the D2/D3 rule; delete the
placeholder, or keep it only as a defensive render.

### Nit

**[N1]** `ListState<T>` collides with `ratatui::widgets::ListState` (TableState is already imported
in editor.rs); rename (e.g. `TargetList<T>`) to avoid import friction.
**[N2]** D2's Enter-arm says `Ok(())` but the persist fns return `Ok(EventId)` (D4 uses `Ok(id)`).
**[N3]** `parse_usd_arg` accepts negative decimals; neither the CLI nor the spec'd forms validate
sign — exact parity, but a `-500` FMV writes silently. Optional followup (both surfaces at once).
**[N4]** TUI trims `donee` and caps it at FIELD_CAP=64; the CLI passes it untrimmed/unbounded —
harmless divergence, worth one sentence in the spec.
**[N5]** KAT-S2 should inherit KAT-S1's root-skip guard (main.rs:1120–1124: chmod 0o500 does not
deny writes to root) rather than only the R0-M3 pre-recorded fallback.

---

## 4. Per-mandate evaluation summary

1. **Pre-filters:** Inbound compound filter — CONFIRMED correct against all four fire sources; no
   over-inclusion; remedy for excluded rows stated (wording defect → I4/M4; effective-payload
   drift → M2). Outbound — Claim B CONFIRMED; no extra filter needed. Stale-race — duplicate arm
   handled; SelfTransfer arm mis-claimed (M1).
2. **Payload construction:** exact vs event.rs; `now` injected at Enter; persist fns confined to
   edit/persist.rs; `append_` token allowlist claim VERIFIED at persist.rs:235 — no gate change ✓.
   Retry-semantics doc wrong (I1).
3. **Validation parity:** required/optional/parse fns match the CLI arms exactly, including
   `business` default-false (plain flag, main.rs:224–225 — the chunk-C required-explicit rule
   applies only to `ReclassifyIncome`) ✓. Spend label self-contradiction (I3).
4. **Post-effect honesty:** FmvMissing and both-None-gift cases surfaced and KAT'd ✓; case-3 gift
   re-fire NOT surfaced (I5); two statuses recommend blocker-creating remedies (I4); gift modal
   omits donee (I7); Income/Gift modals otherwise enumerate every payload field + target ✓;
   both-None warning line present and conditioned correctly ✓.
5. **Safety tests:** strict prefix form structurally right (len+1, full-row prefix equality incl.
   ordinal, kind, payload round-trip, non-empty seed) with one wrong expectation formula (I6);
   per-flow E2E incl. blocker transitions ✓ (one gap, M7); per-flow cancel-bytes walks every step ✓;
   validation matrix covers the field tables incl. the R0-M4 whitespace pins ✓; KAT-S2 growth
   assertion wrong (I1).
6. **Scope:** Task split sane; 2b/3+ excluded explicitly; viewer untouched; SemVer MINOR correct ✓.

**Gate decision: BLOCKED — re-spec and return for R0 round 2. 0 Critical / 7 Important /
8 Minor / 5 Nit.** All seven Importants have mechanical, spec-text-level fixes; none invalidates
the architecture (list → picker → form → payload-showing modal → persist.rs writer → re-project),
which is verified sound against HEAD 096a07b.

---

# Round 2 — re-review (post-fold)

**Artifact re-read in full:** `design/SPEC_tui_edit_chunk2a.md` (1103 lines, all `[R0-…]` tags
inspected). **Baseline:** still HEAD `096a07b`; all fold-introduced citations re-checked against
the same source reads used in round 1 (no source drift since).
**Date:** 2026-07-02

**Verdict: 0 Critical / 0 Important / 0 Minor / 3 record-only Nits — R0 GREEN. Clear to
implement Tasks 1–3.**

## Closure verification — the seven Importants

| Finding | Closed? | Evidence in the folded spec (verified against source) |
|---|---|---|
| **I1** retry story | **YES** | Hard constraints §: FIRST-WINS stated correctly for both types (resolve.rs:549–564 / 600–617 — spans re-verified); retry = identical-payload duplicate → **Hard `DecisionConflict` on the retry's id** (resolve.rs:606–614, `event: Some(d.id)` confirmed) gating via `TaxYearNotComputable`; on-disk log grows by **2**; CLI-void remedy named (`btctax reconcile void decision|<N+2>`); "no rollback / no dedup-on-retry" pinned so the behavior isn't "fixed" away. Both D4 doc comments now agree with the Hard-constraints text (the round-1 self-contradiction is gone). KAT-S2 asserts the TRUE outcome: `pre + 2`, both payload round-trips identical, the conflict attributed to the retry id in the re-projection, and the status surfacing it — plus KAT-S1's root-skip guard [N5]. |
| **I2** flow layer | **YES** | D1: NO standalone list fields on `EditorApp` (flows own their `TargetList`); state invariant stated once; dispatch order **modal → flow → form → screen** with the guard on the flow `Option`, explicitly NOT the step — every step of an open flow is claimed, `q` swallowed and `Esc` steps back at every step. KAT-C2a/C2b now assert `q`-swallow (`!should_quit`, flow open) at EVERY step. Task 1/Task 3 wording consistent (`handle_classify_inbound_flow_key` dispatches on step; "no step can leak keys to Browse"). |
| **I3** spend label | **YES** | D3 table: "gross proceeds (USD)" for sell AND spend, with the correct source anchors (event.rs:62 GROSS; reconcile.rs:55–57). KAT-V-RO-9 agrees; KAT-E2E-RO asserts the "gross proceeds" label for sell; KAT-E2E-DONATE asserts "FMV". No remaining contradiction. |
| **I4/M4** remedies | **YES** | Pre-filter § and both D2 statuses name ONLY void-then-re-classify, with the CLI path (`btctax reconcile void decision|{seq}`) explicit; the set-fmv and bare-re-classify failure modes are documented with the correct citations (ManualFmv→non-Income Hard conflict, resolve.rs:423–470; TransferIn arm never reads `manual_fmv`, resolve.rs:251–281; duplicate classify, resolve.rs:554–561). KAT-E2E-FMV-MISSING pins "FmvMissing" + "void" and explicitly bans the "set-fmv" string; GIFT-UNKNOWN pins "basis unknown" + "void". FOLLOWUPS constrain a future 2b set-fmv flow to Income targets. |
| **I5** blocker-derived status | **YES** | D2/D3 post-effect statuses are normatively derived from the RE-PROJECTED `snap.state.blockers` (D4 step 2 defines the uniform check: conflict-by-returned-id; `FmvMissing`/`UnknownBasisInbound`-by-target for inbound; `UncoveredDisposal`-by-target + optional disposals/removals presence for outflow) — never payload-shape-keyed. New **KAT-E2E-GIFT-PRICE-GAP** (donor date outside the price dataset, fold.rs:913–927) proves the derivation: a shape-keyed status would falsely report success there. Cases 3 and 4 share one honest status — correct, since after re-projection any `UnknownBasisInbound` on the target IS the re-fire (the original `Op::UnknownInbound` can no longer execute once classified). |
| **I6** decision_seq formula | **YES** | KAT-P2a/P2b: `Some(pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0) + 1)` — mirrors `append_decision`'s MAX-over-all-decision-rows allocator (persistence.rs:246–250), correct under the spec'd seed order (2 elections + trailing import → expected 3), and now type-correct (`Option<i64>`). The Current-state § documents the allocator semantics beside the citation. |
| **I7** gift-modal donee | **YES** | D3 modal shows the `donee` line for BOTH gift and donate (with a "(none)" fallback — the field is displayed even when empty); gift mock shows `appraisal_required` absent. KAT-E2E-DONATE asserts the donee value in the modal; new **KAT-E2E-GIFTOUT-DONEE** asserts donee shown + appraisal NOT shown for gift, and the persisted `Removal{kind=Gift, donee=Some}`. Task 3 checklist updated. |

## Minor/Nit closures — all confirmed

- **M1:** D3 now states the link+reclassify overlap is *precedence, not conflict*
  (resolve.rs:600–605), corrects the duplicate-conflict citation to 606–614, marks the arm
  unreachable in 2a (VaultLock, persist.rs:8–11), and hands chunk 3 the robust
  disposals/removals-presence detector (Task 3 + FOLLOWUPS). ✓
- **M2:** raw-vs-effective under-inclusion documented in the pre-filter § with the
  resolve.rs:531–533 citation, CLI interim path, and the cheap `ClassifyRaw{as_: TransferIn}`
  fix recorded in FOLLOWUPS. ✓
- **M3:** initial `IncomeKind` = Mining stated (D2 table, flow struct, KAT-V-CI-9); KAT-E2E-CI
  adds the Tab press to reach "staking" — the assert is now consistent with the script. ✓
- **M5:** KAT-C2a cites KAT-C1 at `crates/btctax-tui-edit/src/main.rs:972–1096` (verified);
  the phantom `unlock.rs` citation is gone; Current-state § adds the KAT-C1/KAT-S1 anchors. ✓
- **M6:** UncoveredDisposal fire sites corrected (Dispose/GiftOut/Donate consume paths,
  fold.rs ~575–630 / ~965–1095 — matches the grep of all fire sites); pre-existing-shortfall
  note added; KAT-E2E-UNCOVERED asserts the pre-state first. ✓
- **M7:** new **KAT-E2E-GIFT-DUAL** pins the §1015 dual-basis lot (`usd_basis == donor_basis`,
  `dual_loss_basis == Some(fmv_at_gift)`, `donor_acquired_at` carried) — verified achievable
  against fold.rs:903–954 case 2 + lot construction. ✓
- **M8:** single normative empty-list rule (flow never opens empty); the widget placeholder is
  explicitly defensive/unreachable/un-KAT'd. ✓
- **N1** `TargetList<T>` rename ✓; **N2** `Ok(id)` ✓; **N3** negative-sign parity FOLLOWUP
  (both surfaces together) ✓; **N4** donee trim/cap divergence documented as deliberate ✓;
  **N5** KAT-S2 inherits the root-skip guard ✓.

## New-findings sweep over the fold-introduced text

Checked: the step-2 check's blocker attributions (DecisionConflict → decision id;
FmvMissing/UnknownBasisInbound → TransferIn target; UncoveredDisposal → TransferOut target —
all match the `add_blocker` call sites); no pre-existing inbound blocker can masquerade as a
re-fire; unrelated pre-existing DecisionConflicts are excluded by the returned-id scoping; the
`decision|{seq}` status template matches `EventId::canonical()`; flow/profile-form mutual
exclusion holds structurally (flows open only from Browse; the form layer consumes Chars);
1990-01-01 is safely outside any BTC price dataset for PRICE-GAP; Task 1/2 KAT lists include
all five new/renamed KATs; the chunk-1 substrate citations (draw_edit.rs:232–293/296–305,
main.rs:79–88/146–208/168–198, form.rs:13–57, persistence.rs:238–262/334–380, app.rs:104–111,
eventref.rs:76–83, persist.rs:235) all re-verified accurate. **No new Critical, Important, or
Minor findings.**

### Residual nits (record-only — none blocks GREEN)

- **[R2-N1]** KAT-C2a's per-step `q`-swallow assertion: at TEXT-INPUT steps `q` is a printable
  char and is INSERTED into the focused `FieldBuffer` (that insertion IS the correct non-quit
  behavior). The test must account for the stray char (Backspace it, or assert `q` at the
  non-text steps and char-insertion at text steps) or the subsequent `Enter` validation will
  fail on a corrupted buffer. Implementation detail; the spec'd assertion (`!should_quit`,
  flow open) remains correct as written.
- **[R2-N2]** Hard constraints: "Quitting without any successful save loses **both** in-memory
  decisions" — "both" only applies post-retry; it is ONE decision if the user quits after the
  first failed save. The claim is right in either case; wording only.
- **[R2-N3]** KAT-S2 exercises the save-error/retry path for `persist_classify_inbound` only;
  the reclassify-outflow persist fn is structurally identical and its retry claims are stated
  in its doc comment. Same posture round 1 accepted — record here so Task 3's whole-diff review
  can add the parallel test if it proves cheap.

## Gate decision

**R0 GREEN — 0 Critical / 0 Important.** All 20 round-1 findings are folded faithfully; the
fold introduced no regressions; internal consistency holds (dispatch order ↔ KAT-C2a/C2b;
label table ↔ KAT-V-RO-9/E2E asserts; retry text ↔ KAT-S2; status strings ↔ E2E string pins).
Implementation of Tasks 1–3 may proceed under the standard TDD-red-first discipline, with
KAT-G1 green throughout and the three R2 nits available to the Task-3 whole-diff reviewer.
