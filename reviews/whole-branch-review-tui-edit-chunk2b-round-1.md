# Whole-branch review — mutating-TUI chunk 2b (round 1)

**Branch:** `feat/tui-edit-chunk2b` @ `e2fb481` (3 commits over `fe726ff`: spec `6d5c9d2`,
Task 1 `a623812`, Task 2 `e2fb481`).
**Scope:** reclassify-income + set-fmv + void flows + the D3.2 remedy-string rework.
**Spec:** `design/SPEC_tui_edit_chunk2b.md` (R0 GREEN, 2 rounds — round-2 disposition
persisted in `reviews/R0-spec-tui-edit-chunk2b-round-1.md`).
**Reviewer independence:** all implementer gate claims re-verified first-hand; targeted
tests run to completion by the reviewer (controller separately verified the full gate at
HEAD: 844/0, both clippys, fmt, both PII scans).

## Verdict

**NOT ready to merge as-is: 0 Critical / 2 Important / 2 Minor / 3 Nit.**
Both Importants are outstanding **Task-3 deliverables the spec itself mandates**
(a missing spec-required unit test and the missing FOLLOWUPS records) — small,
mechanical, and confined to test + docs surface. The shipped Task-1/Task-2 code and
KATs are conformant, honest, and strong; no product-code defect was found.
Fix I1 + I2, re-run the gate, re-review the fold → expect GREEN.

## Targeted test results (run by this reviewer)

```
cargo test -p btctax-tui-edit
test result: ok. 116 passed; 0 failed; 0 ignored; 0 measured; finished in 13.57s
```

Run as non-root, so the four `#[cfg(unix)]` chmod KATs (S1, S2, S2-RO, S2b) genuinely
executed (no root-skip). The 116 include every chunk-2b KAT named in the spec's D5
list plus the five strengthened 2a pins and KAT-G1. Matches the T2 report's claimed
per-crate count exactly — no false-green detected.

---

## Findings

### [I1] IMPORTANT — spec-mandated unit test for `persist_void`'s LotSelection →
`optimize_attest::clear` arm is missing; the arm has zero coverage in this crate

Spec Task 3 (cross-cutting checks): *"Void LotSelection side-effect: `persist_void`
calls `optimize_attest::clear` before save for LotSelection targets; **verified by grep +
unit test in persist.rs** (not a KAT-E2E, just a direct fn call on a seeded vault with a
LotSelection decision)."*

The code is present and correct (`crates/btctax-tui-edit/src/edit/persist.rs:197–217`),
but no test anywhere in `btctax-tui-edit` exercises a LotSelection target: the only
`LotSelection` references in the crate are the persist.rs doc comment/match arm and the
`summarize_void_payload` display arm (main.rs:2301). KAT-P2e and KAT-VOID-RETRY void a
MethodElection; the `disposal_to_clear` branch is never taken under test.

The equivalent CLI behavior IS pinned
(`crates/btctax-cli/tests/optimize_accept.rs:649`
`void_clears_attestation_row_prevents_mislabel_as_attested_recording`), which is why this
is Important rather than Critical — but `persist_void` is a **separate copy** of that
logic, and a regression in the copy (e.g. dropping the pre-save clear, or clearing after
save) would ship silently. This is a vault-mutating side-table write on the void flow —
the exact surface the program's void mandate protects.

**Fix:** the spec's own prescription — a persist.rs unit test that seeds a vault with an
import + a `LotSelection` decision + a populated `optimize_attestation` row, calls
`persist_void` directly, and asserts (a) the void row appended, (b) the attestation row
gone after drop+reopen (same atomic save), (c) a non-LotSelection void leaves an
unrelated attestation row untouched.

### [I2] IMPORTANT — chunk-2b FOLLOWUPS records not written (spec Task 3 deliverable)

Spec Task 3 mandates recording in `FOLLOWUPS.md`: the WB-I4(a) raw-vs-effective
under-inclusion now extended to the reclassify-income and set-fmv filters; the chunk-3+
flow list; the deferred SafeHarbor effective-allocation E2E; the [M3]
rejected-SafeHarbor-void list gap; the [I1] generic cascade-detection deferral; and the
[R0-N3] negative-sign parity carryforward. None are present (`grep -n "2b" FOLLOWUPS.md`
shows only the 2a-era forward references). Program precedent (chunk 2a: `5806e75`
"docs: FOLLOWUPS — chunk 2a shipped" landed on-branch before the merge commit) puts
these records **pre-merge**. The T2 report honestly flags Task 3 as remaining; it is
still a merge-blocking artifact per the spec and `STANDARD_WORKFLOW.md`.

### [M1] MINOR — KAT-E2E-VOID-RECLASSIFY-INCOME omits the spec'd IncomeRecord
projection asserts

Spec D5 steps 2/3/5 pin the projected `IncomeRecord` at each stage ("business=true" →
void → "business=false (original restored)" → re-reclassify → "{business:true,
kind:Mining}"). The shipped test (main.rs:7018–7114) pins only the r-list membership
round-trip + no-conflict status. List membership derives from the **event log**
(pre-filter), not the projection — so this KAT alone would not catch a resolve-layer
failure to un-apply ReclassifyIncome on void. Mitigation (why Minor, not Important):
the restoration IS pinned at the core layer
(`crates/btctax-core/tests/reclassify_income.rs:583` `void_reverts_to_original`:
kind reverts to Reward, `!rec.business`), and KAT-E2E-RI pins the forward apply through
the TUI. Suggest adding the three `income_recognized` asserts when touching the file.

### [M2] MINOR — KAT-E2E-VOID-CASCADE closes on negative-form asserts

Spec step 4 asks that the void's own status "was the CLEAN (or returned-blocker)
string"; the test asserts only `!starts_with("Void saved, but DecisionConflict")`
(main.rs:7262). Spec step 5 asks that "the original Unclassified blocker for the raw
event is back — the honest baseline"; the test asserts only that no `DecisionConflict`
remains. Both negative forms are sound (a save failure would have tripped the step-4
orphan-conflict assert against the un-re-projected snapshot), but positive pins
(`starts_with("Voided")`; the returned raw-event blocker) would be strictly stronger.

### [N1] NIT — KAT-VOID-CONFLICT-ARM uses `contains` for the first assert

R0 round-2 [R2-N2] preferred `starts_with("Void saved, but DecisionConflict fired")`;
the shipped test (main.rs:7330) uses `contains`. The honesty pin
(`!starts_with("Voided")` — and "Void saved," is verifiably not a "Voided" prefix) is
intact, and R2-N2 was explicitly non-blocking.

### [N2] NIT — duplicate set-fmv list rows possible if multiple FmvMissing blockers
attribute to one event (T1 report already records this; same property as 2a's
blocker-sourced list; cosmetic).

### [N3] NIT — `summarize_void_payload`'s `_ => ("?", "?", None, false)` arm is
unreachable behind the `is_revocable_payload` filter; harmless defensive dead arm.

---

## Verification evidence (per review mandate)

### 1. The void flow (highest priority) — CONFORMANT

- **Revocable set exact.** `is_revocable_payload` (form.rs:822–836) lists precisely the
  spec's nine: TransferLink, ReclassifyOutflow, ClassifyInbound, ManualFmv, ClassifyRaw,
  MethodElection, LotSelection, ReclassifyIncome, SafeHarborAllocation — verified
  against resolve.rs:301–341 read first-hand (`Some(_)` insert arm at :330;
  SafeHarborAllocation diverted to `allocation_voids` at :322–328; SupersedeImport/
  RejectImport/VoidDecisionEvent → immediate conflict at :312–321; unknown target →
  conflict at :332–338). `open_void_flow` (main.rs:2334–2388) applies exactly Claim E's
  three filters (Decision id, not-in-voided-set, revocable), sorted by seq
  (deterministic). **KAT-VOID-EXCLUSIONS** (main.rs:7433) seeds all three non-revocables
  + an already-voided ClassifyInbound + RO + ME and asserts `len == 2` with per-tag
  presence/absence — exact. **KAT-VOID-SHA-WARNING** (draw_edit.rs:1726) pins the
  SafeHarbor Path-B warning INCLUDING the [M3] permanence line, its absence for
  non-SafeHarbor, and the always-present cascade note — render-level, both directions.
- **Cascade note verbatim.** `draw_void_modal` (draw_edit.rs:1406–1423) carries the
  D3.1 consequence block word-for-word (both consequence categories [I1]) and the
  SafeHarbor warning word-for-word including "A rejected void permanently removes this
  allocation from this list (CLI void remains available)".
- **E2E-VOID-ROUNDTRIP** (main.rs:6882): UBI fires → classify via `c` → excluded from
  `c` list → void via `v` (decision found by id) → UBI RETURNS → re-classify with no
  conflict → voided decision absent from `v` list. Full remedy loop closed in-editor.
- **E2E-VOID-CASCADE** (main.rs:7121): Unclassified+wallet → ClassifyRaw(as Income,
  fmv None) → ManualFmv clears FmvMissing → pre-state clean → TUI-void the ClassifyRaw →
  Hard DecisionConflict attributed to the **ManualFmv's id** (not the void's) → the
  orphan IS in the re-opened `v` list → void it → conflict gone. Pins the orphan
  attribution, the D3.1 surfacing limit, and the in-list remedy. (See [M2] for the
  negative-form closing asserts.)
- **VOID-CONFLICT-ARM** (main.rs:7317): synthetic snapshot, conflict attributed to the
  void's id → status contains "Void saved, but DecisionConflict fired" + "the target
  decision remains in force", `!starts_with("Voided")`. `derive_void_status` arm 1
  (main.rs:2410–2417) has no `{seq}` interpolation — the [I2] identity-mixing fix holds.
- **VOID-RETRY** (main.rs:7349): `persist_void` twice on the same MethodElection →
  pre+2 rows, both round-trip as `VoidDecisionEvent` targeting the ORIGINAL decision,
  re-projected `blockers.is_empty()`. Mechanism reasoning verified in code and against
  resolve.rs: the retry's target is the original decision (the modal carries
  `target_event_id` = the decision id, never the first void's id), so the
  non-revocable void-of-void arm (:312–321) cannot fire; `BTreeSet::insert` (:330) is
  idempotent. This is correctly the OPPOSITE of 2a's FIRST-WINS-conflict retry, and
  the seeded election being back-dated makes `blockers.is_empty()` double as an
  un-projection proof (an in-force back-dated election would fire
  MethodElectionBackdated). Save-error UI [M1-spec]: `handle_void_modal_key` Err arm
  (main.rs:1680–1684) closes only the modal; the flow stays at List — as spec'd.
- **`optimize_attest::clear` ruling: spec-required, correct, NOT scope creep.**
  Mandated twice by the spec (Hard constraints bullet 3; D4 `persist_void` reference
  implementation — the shipped fn is line-for-line the D4 sketch) and it mirrors the
  CLI `cmd::reconcile::void` verbatim (reconcile.rs: load_all → find LotSelection →
  append → clear → single `session.save()`, same atomic batch; the CLI doc explains the
  stale-AttestedRecording mislabel edge it closes). Gating: the call lives only in
  `edit/persist.rs`; it cannot be invoked elsewhere without `session.conn()`, whose
  `conn(` token KAT-G1 forbids outside persist.rs non-test code; `persist_void`'s sole
  non-test call site is the void-modal Enter arm (main.rs:1649). The ONLY gap is the
  missing unit test — finding [I1].

### 2. reclassify-income + set-fmv (Task 1) — CONFORMANT

- **Claim-C pre-filter verbatim.** `open_reclassify_income_flow` (main.rs:2054–2137):
  `voided` and `already_reclassified` sets hoisted once before the closure [N3], raw
  `EventPayload::Income` filter, exclusion of non-voided-ReclassifyIncome targets,
  income_recognized FMV enrichment (None → "(pending)"), date-sorted, R0-M8 empty-list
  status. Verified against resolve.rs pass-1e read first-hand: FIRST-WINS with the
  duplicate conflict attributed to the SECOND decision's id (matches the derive fn's
  decision-attributed check).
- **Required-explicit business.** `validate_reclassify_income` (form.rs:710–726) rejects
  `None` with the exact spec string; `cycle_business_optional` is the spec 3-state;
  KAT-RI-REQUIRED-BUSINESS is key-driven (render pin for `---`/`[required]`, Enter
  blocked, Tab→true→Enter opens modal) + KAT-V-RI-1..4 / KAT-V-FMV-1..3 (whitespace-only
  → parse error, the R0-M4 pin) + both cycle KATs.
- **KAT-E2E-RI-SE exact figures.** Before: `niit == dec!(380.00)` exact,
  `compute_se_tax == None` (Interest SE-excluded). Drive Interest→Mining,
  business=true via keys; modal render pins "business: true", "(was false)",
  "mining (was interest)". After: `niit == 0`, delta `== dec!(380.00)`; SE
  base 9235.00 / ss 1145.14 / medicare 267.82 / total 1412.96 / deductible_half 706.48 —
  every figure cross-checked against the core KAT
  (`crates/btctax-core/tests/reclassify_income.rs:168–173, 261–266, 445–512`;
  `magi_excluding_crypto = 205000` matches `niit_profile()` at :134). Status pinned with
  `assert_eq!` to the exact clean string — proves no tax figure leaks into status.
- **REPOINT.** `kat_e2e_fmv_repoint_second_set_fmv_no_conflict` (main.rs:6320):
  `persist_set_fmv` twice → pre+2 rows, no DecisionConflict, income_recognized reflects
  the SECOND FMV — latest-wins pinned at the unit level exactly as the spec prescribes
  (the event leaves the `f` list after the first set, so E2E re-point is impossible —
  correctly reasoned). resolve.rs pass-1d read first-hand: latest-seq-wins insert with
  the explicit no-duplicate-blocker comment (:427–428, :453–456).
- **Fixture MethodElection fix: legitimate.** The T1 deviation-4 fix (back-dated
  election fired the Hard MethodElectionBackdated blocker, refusing the exact-figure
  NIIT assert) removed/date-fixed the seed rather than weakening any assert — the
  correct direction. The remaining back-dated seeds (P2c/P2d) assert log shape only
  (no projection); KAT-VOID-RETRY's back-dated seed is voided before projection.

### 3. The remedy-string rework — CONFORMANT, NOTHING DELETED

- All four 2a arms updated to the spec's exact NEW strings:
  `derive_classify_inbound_status` DecisionConflict (main.rs:1939–1943 — "Void flow
  (press 'v') or CLI"), FmvMissing (:1956–1959 — "Void flow: press 'v'; or CLI"),
  UnknownBasisInbound (:1975–1979), `derive_reclassify_outflow_status` DecisionConflict
  (:2011–2015). The WB-I1 single-prefix `canonical()` discipline is retained.
- **Deletion audit (mechanized).** A net-deletion analysis of the full
  `fe726ff..e2fb481` diff (deleted lines minus re-added lines, per file) found **zero
  net-deleted assert/contains lines** across all five source files. Total net deletions:
  import-list reshuffles + exactly the four OLD remedy strings. Nothing was weakened or
  removed from any test.
- **Five in-place strengthenings verified:** KAT-S2 (main.rs:3908), KAT-S2-RO (:5429),
  `kat_e2e_fmv_missing…` (:4141), `kat_e2e_gift_unknown…` (:4212),
  `kat_e2e_gift_price_gap…` (:4314) each gained `contains("'v'")` while retaining their
  original `DecisionConflict` / `void {canonical}` / `contains("void")` /
  `!contains("set-fmv")` pins.
- **KAT-RS-1..4** (main.rs:6535–6622): each pins BOTH `"'v'"` and
  `"btctax reconcile void"` on the correct arm via synthetic snapshots.

### 4. The safety net — CONFORMANT

- **KAT-P2e** (persist.rs:1098): strict prefix (`post[..pre.len()] == pre`), tail seq =
  max+1, returned id == tail id, payload round-trips as `VoidDecisionEvent` targeting
  the seeded MethodElection, drop+reopen identical. P2c/P2d follow the same skeleton
  with non-trivial prior decision_seq.
- **KAT-C2e** (main.rs:6627): `q` swallowed at List AND at modal (`!should_quit`
  asserted), Enter → modal DIRECTLY (no FieldForm), exactly two Esc presses after the
  modal (modal→List, List→close — the [N2] count), bytes-identical vault, plus the
  writes-on-confirm complement. C2c/C2d cover the two-step flows incl. the R2-N1
  text-field `q`-inserts behavior.
- **KAT-S2b** (main.rs:6738): root-skip guard, chmod 0o500, save fails → modal closed /
  FieldForm open / "Save error" / bytes unchanged → restore → retry → **pre+2 rows, both
  ManualFmv, FmvMissing GONE, NO DecisionConflict, clean status** — the LATEST-WINS
  retry contract, correctly the opposite of 2a's classify retry (mechanism: resolve
  pass-1d last-write-wins insert, no duplicate blocker — verified in source).
- **KAT-G1**: green in the targeted run. `persist_reclassify_income` / `persist_set_fmv`
  / `persist_void` all live in `edit/persist.rs`; `"append_"`, `conn(`, `save(` remain
  the gated tokens (persist.rs:632+, unchanged); grep confirms each persist fn has
  exactly ONE non-test call site — its own modal Enter arm (main.rs:1243, :1315, :1649);
  all other `append_decision`/`conn(` hits are in the `#[cfg(test)]` region;
  `optimize_attest` appears nowhere in tui-edit outside persist.rs.

### 5. Substrate, determinism, scope — CONFORMANT

- **2a/chunk-1 substrate untouched:** the net-deletion analysis shows the only
  substantive changes to pre-existing code are the four spec'd remedy strings and the
  dispatch-layer insertions (modal layers 4–6 before the flow layer; 9-layer order
  matches the [N4] diagram exactly, main.rs:99–168). `TargetList`, FieldBuffer,
  events_by_id, and all 2a handlers are reused, not modified.
- **Diff surface:** exactly 5 crate files (all `btctax-tui-edit`) + spec + R0 review.
  No `btctax-core`/`btctax-cli`/viewer changes — the SemVer MINOR claim holds; the E10
  viewer freeze is untouched.
- **Determinism:** all three lists sorted (date/date/seq); BTree containers throughout;
  `now` injected into persist fns at Enter-press; test seeds use fixed unix timestamps.
- **Synthetic-only:** all new fixtures use River/main exchange ids, `kat-*-pass`
  passphrases, round synthetic amounts; controller's PII scans clean at HEAD.
- **No scope creep:** every shipped item traces to a spec section; the one candidate
  (`optimize_attest::clear`) is affirmatively spec-required (see §1 ruling). Out-of-scope
  items (link-transfer, batch void, negative-sign tightening) correctly absent.

---

## Required actions before merge

1. **[I1]** Add the spec-mandated persist.rs unit test for the LotSelection →
   `optimize_attest::clear` arm of `persist_void` (direct fn call on a seeded vault;
   assert the attestation row cleared atomically; non-LotSelection no-op).
2. **[I2]** Write the chunk-2b FOLLOWUPS records (WB-I4(a) extension, chunk 3+,
   SafeHarbor E2E deferral, [M3] list gap, [I1] cascade-detection deferral, [R0-N3]).
3. Re-run the full gate; fold; re-review per §2 (this review, round 2).

Minors/Nits may be folded opportunistically or recorded; they do not block.
