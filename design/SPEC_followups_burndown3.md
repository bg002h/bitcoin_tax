# SPEC — small-FOLLOWUPS burndown 3

**Source baseline:** `main` @ `4125db3` (post CI-infrastructure). The user-approved form-program
sequence item "small-FOLLOWUPS burndown" (FOLLOWUPS §"TY2024 tables backfill" queue note).
**Fold record:** R0 round 1 (`reviews/R0-spec-burndown3-round-1.md`, 0C/1I/3M/4N) folded in full —
I-1, M-1, M-2, M-3, N-1, N-2, N-3, N-4 tagged inline at their sites.

**Goal:** Close six small OPEN FOLLOWUPS in one gated cycle: (1) the ReclassifyOutflow /
ClassifyInbound / ManualFmv **bad-target validation backfill** (the Chunk-C I-2 deferral — the
substantive item, and the safety prerequisite for any future mutating-TUI work: a mutating surface
must never be able to append a decision that silently does nothing); (2) a **§6017 $400 SE
filing-floor note** (text-report-only); (3) **real-binary negative-flag tests** for
`--w2-ss-wages`/`--w2-medicare-wages` (Chunk-A M-1); (4) a **hook mode-assertion KAT** (CI N-2);
(5) **TY2024 full-schedule equality KATs** (TY2024 M1); (6) two **record-only corrections** (CI
M-1/M-2).

**SemVer:** no public API, struct, or event-schema change ⇒ **PATCH** (pre-1.0). Item 1 is a
deliberate **projection-behavior change**: decisions that were silently inert now surface Hard
blockers — see D1 §"Compatibility posture".

**Coordination pin (PARALLEL-LANE — hard constraint):** this burndown is implemented in a worktree
parallel to the export-from-TUI lane. That lane owns `crates/btctax-cli/src/render.rs`'s
**CSV-writer region (`render.rs:556–966`)** and **all of `crates/btctax-tui/`**. [R0-N4] The
556–966 range is the lane's *forbidden area* and is deliberately over-inclusive: it holds
`write_csv_exports` + all `write_*_csv` fns + the export lane's new `pub fn`, but also the
`render_donation_appraisal_advisory` text renderer (788–810) and doc-comment lines (962–966) —
the export lane's own spec should describe its ownership precisely; for THIS lane the whole range
is simply off-limits. This burndown may touch `render.rs` ONLY inside the `render_schedule_se`
text region (`render.rs:1118–1275`) for item 2 — **no CSV header comments, no writer-block edits,
no btctax-tui changes** (the TUI condensed-block disclosure-lines item has moved to the export
lane). The pin also covers TESTS: render.rs's own test module (2396+) is outside 1118–1275 and is
export-lane territory — this burndown adds no tests there ([R0-I1], see Task 1). Any merge
conflict in `render.rs` outside `render_schedule_se` is a process violation, not a rebase chore.

---

## Current state (recon @ `4125db3`)

### Item 1 — bad-target handling is silently inert for three decision types

- **The fixed pattern (ReclassifyIncome, the model to replicate):**
  `crates/btctax-core/src/project/resolve.rs:510–575` (pass 1e). Collection-time validation
  against the EFFECTIVE payload:
  `by_id.get(target).map(|raw| applied.get(target).unwrap_or(&raw.payload))`, then a 3-arm match —
  target absent → Hard `BlockerKind::DecisionConflict` + decision EXCLUDED from the map; effective
  payload is the expected type → duplicate-check + insert (FIRST-WINS); any other payload → Hard
  `DecisionConflict` + EXCLUDED. The comment block at `resolve.rs:515–523` explicitly names the
  divergence and carries the FOLLOWUP to backfill (this spec discharges it).
- **The three inert collectors:**
  - `ReclassifyOutflow` — `resolve.rs:499–509` (pass 1e): duplicate-only check, blind insert into
    `outflow_class`. Consulted ONLY in `build_op`'s `EventPayload::TransferOut` arm
    (`resolve.rs:201–250`, `outflow_class.get(id)` at 220). A decision targeting a missing event
    or a non-TransferOut event is never read again — silently inert.
  - `ClassifyInbound` — `resolve.rs:488–498` (pass 1e): duplicate-only check, blind insert into
    `inbound_class`. Consulted ONLY in the `EventPayload::TransferIn` arm (`resolve.rs:251–281`,
    `inbound_class.get(id)` at 255). Same silent-inert failure mode.
  - `ManualFmv` — `resolve.rs:423–433` (pass **1d**, its own loop): blind insert into
    `manual_fmv`, **latest-seq-wins** (documented at 424; NO duplicate blocker — deliberately
    different from the 1e first-wins maps). Consulted ONLY in the `EventPayload::Income` arm
    (`resolve.rs:183–200`, `manual_fmv.get(id)` at 184; doc comment 158–159: "ManualFmv on an
    `Income` replaces the FMV"). **The ManualFmv expected target is an effective `Income`
    payload** — note it is NOT consulted in the `IncomeInbound` path (`InboundClass::Income`
    carries its own `fmv` field, `resolve.rs:257–266`), so a ManualFmv pointing at a
    TransferIn-classified-as-income row is also inert today.
- **Ordering is already safe for the backfill:** `applied` (ImportConflict resolutions, pass 1b
  `resolve.rs:346–401` + ClassifyRaw, pass 1c `resolve.rs:403–421`) is fully populated before both
  pass 1d and pass 1e run — the effective-payload lookup used by ReclassifyIncome works verbatim
  at all three sites.
- All three decision types are revocable via `VoidDecisionEvent` (`resolve.rs:301–306`) — the
  remedy path for a newly-surfaced blocker already exists and needs no code.

### Item 2 — no §6017 note

- `render_schedule_se` (`crates/btctax-cli/src/render.rs:1118–1275`): the `Some(r)` arm
  (1128–1238) renders net_se → base (`r.base` = round_cents(net_se × 92.35%), `se.rs:32–33`) →
  components → total → §164(f) → W-2 disclosure (1214–1229) → the standalone note (1231–1236).
  There is no mention of the §6017 $400 filing floor anywhere; the engine computes SE tax for any
  `base > 0` (`compute_se_tax`, `crates/btctax-core/src/tax/se.rs:107–`). FOLLOWUPS carries
  "§6017 $400 SE filing floor (not modeled; salient with expenses)" (Chunk-B deferred block).

### Item 3 — no binary-level negative-flag tests for the W-2 flags

- Guards exist and fire on the real path: `crates/btctax-cli/src/main.rs:738–740`
  (`--w2-ss-wages must not be negative`) and `main.rs:746–750` (`--w2-medicare-wages`).
- The house pattern to copy: `crates/btctax-cli/tests/tax_report.rs:1038–1073`
  (`tax_profile_negative_schedule_c_expenses_rejected` — real binary via
  `env!("CARGO_BIN_EXE_btctax")`, `--schedule-c-expenses=-5`, assert non-zero exit) and
  `tax_report.rs:690–716` (`report_negative_prior_taxable_gifts_rejected_without_tax_year`).
  Nothing drives `--w2-ss-wages=-…`/`--w2-medicare-wages=-…` through the binary (Chunk-A M-1).

### Item 4 — nothing asserts the tracked executable bit

- `scripts/pre-push` and `scripts/pii-scan-generic.sh` are 100755 on disk and in the index; the
  CI review (`reviews/whole-branch-review-ci-round-1.md:224–227`, N-2) records that the
  original mode-644 fail-open was caught only empirically and recommends a KAT asserting
  `git ls-files -s scripts/pre-push` starts with `100755`. No such test exists in the workspace
  (`git ls-files`/`100755` appear in no `crates/**/*.rs` test).

### Item 5 — TY2024 edges are spot-checked, not fully pinned

- `crates/btctax-adapters/src/tax_tables.rs`: `ty2024()` at 96–215; KATs at 583–763.
  KAT-A1 (583–593) pins only 3 of Single's 7 edges; KAT-A2 (598–617) pins the MFS/MFJ 37% starts;
  A6a–A6d are compute *deltas* (which can cancel a lower-edge transposition — the M1 concern,
  FOLLOWUPS TY2024 deferred block); **KAT-A3 (622–662) already pins all four LTCG pairs by direct
  struct equality** — the LTCG half of M1 is already satisfied; the 28 ordinary bracket edges
  (4 statuses × 7) are the open gap. `design/SPEC_ty2024_tables.md` §3.01 Tables 1–4 holds the
  triple-verified values (author + 2 independent reviewers against Rev. Proc. 2023-34 / IRB
  2023-48) — **transcribe, don't re-derive**.

### Item 6 — the two CI record items

- **M-1** (`reviews/whole-branch-review-ci-round-1.md:188–196`): the untracked ship report claims
  the `unnecessary_map_or` clippy lint "was pre-existing on `a1d5e26`" — reviewer-tested FALSE
  (the lint is MSRV-gated on `rust-version`; it fires only after the 1.88 bump). FOLLOWUPS's CI
  section (line ~30) carries it as "record-only".
- **M-2** (`reviews/whole-branch-review-ci-round-1.md:198–207`):
  `reviews/whole-branch-review-gift-chunk3b-round-1.md:30` contains the synthetic tokens
  `987·65·4321` and `12·3456789` with REAL hyphens, contradicting the CI spec's Notation rule
  ("the real hyphen exists in `scripts/pii-scan-generic.sh` and NOWHERE else"). No gate
  consequence (token-level exclusion is file-agnostic; tree scans clean). The review's mandate:
  "dot-notate that old review or amend the spec's 'NOWHERE else' sentence… A decision, not
  silence."

---

## Design

### D1 — bad-target validation backfill (ReclassifyOutflow / ClassifyInbound / ManualFmv)

Replicate the ReclassifyIncome pass-1e pattern (`resolve.rs:510–575`) at all three collection
sites, validating against the **EFFECTIVE** payload
(`by_id.get(target).map(|raw| applied.get(target).unwrap_or(&raw.payload))` — so a ClassifyRaw'd
or SupersedeImport-overridden row is judged by what it effectively IS, and a `by_id` miss counts
as bad).

**Per-type expected target payload** (each grounded in the sole `build_op` arm that consults the
map — the payload type the decision is inert without):

| Decision type      | Site (collection)     | Expected effective payload | Consulting `build_op` arm |
|--------------------|-----------------------|----------------------------|---------------------------|
| `ReclassifyOutflow`| pass 1e, 499–509      | `TransferOut`              | `resolve.rs:201–250` (220)|
| `ClassifyInbound`  | pass 1e, 488–498      | `TransferIn`               | `resolve.rs:251–281` (255)|
| `ManualFmv`        | pass 1d, 423–433      | `Income`                   | `resolve.rs:183–200` (184)|

**Rules, per site:**

1. **Target absent** (`by_id` miss) → Hard `BlockerKind::DecisionConflict` on the decision's id +
   decision EXCLUDED from the map. Detail message names the decision type, the target's
   `canonical()`, and the remedy ("void the decision" — all three are revocable, no new code).
2. **Effective payload ≠ expected type** → Hard `DecisionConflict` + EXCLUDED. Messages should
   redirect where a better tool exists, mirroring ReclassifyIncome's style:
   - ReclassifyOutflow on a non-TransferOut → "…targets non-TransferOut event … — for Income
     corrections use reclassify-income; void the decision to clear this blocker".
   - ClassifyInbound on a non-TransferIn → "…targets non-TransferIn event …".
   - ManualFmv on a non-Income → "…targets non-Income event … — for a TransferIn classified as
     income, set the FMV via classify-inbound-income (its own `fmv` field); void this decision".
3. **Duplicate semantics are NOT touched:**
   - ReclassifyOutflow/ClassifyInbound keep their existing duplicate → `DecisionConflict` +
     first-wins behavior (only the new arms are added around the existing insert).
   - **ManualFmv keeps latest-seq-wins with NO duplicate blocker** (its documented, deliberate
     semantic — re-pointing an FMV is a correction flow, not a conflict). The backfill adds ONLY
     the missing/mismatched-target arms in pass 1d; a valid re-target still overwrites.
4. **Comment maintenance:** rewrite the `resolve.rs:515–523` divergence comment — the divergence
   no longer exists; the FOLLOWUP line there is discharged by this spec. Add a short shared
   comment at 1d/1e noting all four decision types now validate the effective target payload at
   collection time.

**What this deliberately does NOT validate (out of scope, document in a comment):**
- A ReclassifyOutflow on a TransferOut that is ALSO TransferLink'd (link wins in `build_op`,
  201–218 before 220) passes type-validation but stays overridden by the link — that is a
  *precedence* question, not a bad target; unchanged.
- A ClassifyInbound on a TransferIn consumed by a TransferLink (`consumed_ins`, 252–254) likewise
  passes type-validation; the link consumes first. Unchanged.

**Compatibility posture (state it plainly):** this converts previously-SILENT bad decisions into
Hard blockers. An existing vault holding a stale ReclassifyOutflow/ClassifyInbound/ManualFmv whose
target was deleted-by-supersede or was always wrong will NOW show a Hard `DecisionConflict` and
(because Hard blockers gate `compute_tax_year` — **projection-wide: one stale decision gates
EVERY year's compute, not just reports touching the target (`compute.rs:237–242`, "ANY unresolved
Hard blocker ANYWHERE in the projection gates EVERY year — deliberately conservative") [R0-M3]**)
will refuse every previously-"computing" report until voided. **That is the point** — such a
vault was computing numbers the user believed were corrected but weren't; the all-years blast
radius makes surfacing the breakage loudly strictly safer than silence (a silently-inert
correction poisons the same years invisibly), and the remedy (void the stale decision) is one
command. This is also the mutating-TUI safety prerequisite: an interactive
surface must get a Hard error, not silence, when it appends a decision against a wrong target.
Task 1 includes an explicit audit: run the FULL workspace suite; any existing KAT/fixture that
relied on the silent behavior (none are known — the CLI tests target valid events, e.g.
`reconcile.rs:280–348`) must be either re-pointed at a valid target or flipped to assert the new
blocker, each with a one-line justification in the task record. No migration is owed (append-only
ledger; the fix changes projection, not stored events).

### D2 — §6017 $400 filing-floor note (text-only)

In `render_schedule_se`'s `Some(r)` arm ONLY (inside `render.rs:1128–1238`; the pin forbids
touching anything else in the file): when `r.base < dec!(400)`, append one note line immediately
before the standalone note (1231–1236):

> "(§6017 filing floor) Net earnings from self-employment ({base}) are below $400: a Schedule SE
> filing is required on account of this income only when net earnings from self-employment (the
> ×92.35% base, §1402(a)) are $400 or more (§6017), and below that floor no §1401 SE tax is
> imposed (§1402(b)(2); church employee income excepted — §1402(j)(2), not modeled) — the figures
> above are shown for transparency."

[R0-M2] The "on account of this income only" softening + the church-employee clause are both
load-bearing: the flush language after §1402(b)(2) ("In the case of church employee income, the
special rules of subsection (j)(2) shall apply for purposes of paragraph (2)") substitutes a
lower floor for church employee income (Schedule SE line 4c carries the matching exception), and
the app does not model it — an unqualified "required only when ≥ $400" would overclaim for a
filer who has church employee income.

**Legal grounding (web-verified at spec time; R0 to re-verify):** 26 U.S.C. §6017 — "Every
individual (other than a nonresident alien individual) having **net earnings from self-employment
of $400 or more** for the taxable year shall make a return with respect to the self-employment tax
imposed by chapter 2" (verbatim from uscode.house.gov; 26 CFR §1.6017-1 matches, citing §1402).
The $400 test is on **"net earnings from self-employment"** as defined in §1402(a) — which
INCLUDES the §1402(a)(12) 7.65% reduction, i.e. the **×0.9235 base** (`r.base`), not the
pre-factor `net_se`. Schedule SE's own line 4c applies the "less than $400 → do not file" test to
the post-×0.9235 line. So gross profit of e.g. $430 → base **$397.10** < $400 ([R0-M1] the raw
product $397.1050 is an exact tie, which the house `round_cents` ROUND_HALF_EVEN (`se.rs:10–11`)
resolves to the even cent — $397.10, not $397.11) → below the floor even though the pre-factor
profit exceeds $400 — the exact subtlety the condition `r.base < 400` encodes correctly. The
§1402(b)(2) companion (below $400 → not "self-employment income", so no §1401 tax either) is
cited in the note; **verified by R0 round 1 against the statute text** ("the net earnings from
self-employment, if such net earnings for the taxable year are less than $400" — cite number and
framing both confirmed), including the church-employee flush language that motivates [R0-M2].

**Not done here (deliberate):** no change to `compute_se_tax` (the engine still computes and
displays the sub-$400 figures — conservative transparency; suppressing the computation is an
engine-behavior change out of scope), no CSV changes of any kind (**the pin**: schedule_se.csv
and every other writer belong to the export lane), no TUI change.

### D3 — negative-flag binary tests

Two tests in `crates/btctax-cli/tests/tax_report.rs`, byte-for-byte the
`tax_profile_negative_schedule_c_expenses_rejected` pattern (1038–1073): minimal init'd vault,
real binary via `env!("CARGO_BIN_EXE_btctax")`, full mandatory profile flags plus the poison flag
(`--w2-ss-wages=-5` in one, `--w2-medicare-wages=-5` in the other), assert
`status.code() != 0` with a message naming the guard (`main.rs:738–740` / `746–750`). This
discharges Chunk-A M-1 (the `--prior-taxable-gifts` half of that item already shipped at 690–716).

### D4 — hook mode-assertion KAT

A new integration test (suggested: `crates/btctax-cli/tests/repo_hygiene.rs`) that locates the
repo root from `env!("CARGO_MANIFEST_DIR")` (two `parent()` hops), runs
`git ls-files -s scripts/pre-push scripts/pii-scan-generic.sh` **with
`Command::current_dir(repo_root)` set explicitly [R0-N2]** (cargo runs integration tests with
cwd = the crate manifest dir, and `git ls-files` resolves pathspecs relative to cwd — without the
explicit cwd the pathspecs miss), and asserts BOTH lines are present and start with `100755`. **Fail-closed:** if `git` is unavailable, the command errors, or either
file is missing from the index, the test FAILS (no skip-if-not-git arm — the mode-644 regression
this locks was itself a fail-open; the workspace gate always runs in a git checkout locally and in
CI, and a source-tarball test run failing loudly here is acceptable and documented in the test's
doc comment). This closes CI N-2 and locks the I-1 mode-644 regression permanently.

### D5 — TY2024 full-schedule equality KATs

One table-driven test in `tax_tables.rs`'s existing `mod tests` (under the `── TY2024 KATs ──`
header): for each of the four stored statuses (Single/Mfj/HoH/Mfs), assert the COMPLETE
`OrdinarySchedule` — `brackets.len() == 7` and every `(lower, rate)` pair by direct equality —
against inline expected arrays **transcribed verbatim from `design/SPEC_ty2024_tables.md` §3.01
Tables 1–4** (triple-verified; do NOT re-derive, do NOT read from the builder). That pins all
28 edges + all 28 rates. Include the four LTCG pairs in the same test for a single self-contained
full-schedule assertion (KAT-A3 at 622–662 already pins them directly — the re-assertion is
harmless and makes this one test the complete TY2024 schedule lock). Also assert `Qss` is NOT a
stored key for either map: [R0-N3] `TaxTable::ordinary` and `ltcg` are `pub` fields
(`tables.rs:58, 61`), so mandate the DIRECT assertions
`tt.ordinary.get(&FilingStatus::Qss).is_none()` + `tt.ltcg.get(&FilingStatus::Qss).is_none()`,
plus the alias check `ordinary_for(Qss) == ordinary_for(Mfj)` (no "if reachable" hedge — it is
reachable). Existing A1–A7 KATs stay untouched (they remain the readable spot-checks; this test
is the exhaustive lock).

### D6 — record items (Task 2, no code)

- **CI M-1:** append a one-line correction to FOLLOWUPS's CI section: the `unnecessary_map_or`
  lint was NOT pre-existing at `a1d5e26`; it is MSRV-gated and fired only after the
  `rust-version = "1.88"` bump — the Constraint-9 causality is the bump itself (stronger, not
  weaker, per the reviewer's mechanism test). Mark the M-1 deferral closed.
- **CI M-2 — recommendation: dot-notate the old review file.** Edit
  `reviews/whole-branch-review-gift-chunk3b-round-1.md:30`, replacing the two real-hyphen
  synthetic tokens with the `·`-notation, plus a bracketed editorial note at the line
  ("[notation normalized at burndown-3 — CI M-2]"). Rationale: (a) the Notation rule's value is a
  tree-wide grep invariant ("real hyphen NOWHERE else") — a scoped exception in prose is a trap
  for every future scan/review; (b) the verbatim-persistence rule protects reviewer *substance*,
  and a marked, meaning-preserving token re-encoding with an audit note does not alter substance;
  (c) it is two tokens on one line. The alternative (amend the CI spec's "NOWHERE else" scope)
  weakens a shipped invariant to preserve two characters of hyphenation. **R0 round 1 CONCURRED
  with the dot-notate recommendation** (§1.5: the parent CI reviewer explicitly authorized the
  option; the meaning-preserving re-encoding with an in-line editorial marker respects the
  verbatim-persistence rule in spirit and is auditable) — the decision is settled, no fallback
  needed. Record the decision + edit in FOLLOWUPS; mark the M-2 deferral closed.

---

## Plan (TDD)

### Task 1 — items 1–5 + KATs

**Files:** `crates/btctax-core/src/project/resolve.rs` (D1);
`crates/btctax-cli/src/render.rs` — `render_schedule_se` region ONLY (D2; production line only —
NO tests in render.rs's test module, [R0-I1]);
`crates/btctax-cli/tests/tax_report.rs` (D2 render KATs + D3);
new `crates/btctax-cli/tests/repo_hygiene.rs` (D4);
`crates/btctax-adapters/src/tax_tables.rs` tests (D5);
core KATs for D1 (suggested: extend `crates/btctax-core/tests/reclassify_income.rs`'s bad-target
neighborhood or a new `tests/decision_bad_target.rs`).

KATs (synthetic; assert EXACT; RED before the fix where applicable):

- **D1, per decision type — six blocker KATs (2 × 3 types):**
  - *Missing target:* a `ReclassifyOutflow` (resp. `ClassifyInbound`, `ManualFmv`) pointing at an
    EventId absent from the ledger → project → assert exactly one Hard `DecisionConflict`
    naming the decision, AND the projected state fields (disposals/removals/income/holdings) are
    equal to the same fixture without the decision ([R0-N1] the blocker lists differ by
    construction, so "byte-equal projection" is unsatisfiable — assert the state fields, matching
    how `reclassify_income.rs:632–718` asserts it) — blocked, not applied, not a panic.
  - *Wrong-type target:* the decision pointing at (respectively) an `Income` event, a
    `TransferOut` event, and a `TransferIn` event → same assertions. (These type pairings are
    chosen adversarially: each is the type a confused user would most plausibly hit.)
- **D1 happy paths unmoved — three KATs (or assert via existing suite):** a valid
  ReclassifyOutflow→Donate, a valid ClassifyInbound→GiftReceived, and a valid ManualFmv on an
  FmvMissing Income (the `reconcile.rs:281` fixture pattern) still project identically to today
  — zero new blockers. The FULL workspace suite green is the aggregate happy-path regression
  gate; any fixture that trips a new blocker is triaged per D1's compatibility-audit rule.
- **D1 ManualFmv semantics preserved:** two sequential valid ManualFmv decisions on the same
  Income → the LATER value projects, NO DecisionConflict (locks latest-seq-wins against an
  over-eager "harmonize with 1e" refactor).
- **D1 void remedy:** bad-target decision + `VoidDecisionEvent` → blocker GONE, projection clean
  (one KAT, any one type — the remedy path the blocker messages advertise).
- **D2 render KATs — in `tax_report.rs` ONLY [R0-I1]:** NOT in render.rs's own test module —
  that module (2396+, where the existing `render_schedule_se` + `schedule_se.csv` KATs live) is
  outside the pinned 1118–1275 region and is exactly where the export lane will add its writer
  KATs; adding there violates the mandatory per-commit pin check and courts the merge conflict
  the pin exists to prevent. Feasible from the integration test: `pub mod render` (`lib.rs:10`),
  `render_schedule_se` is `pub`, and every `SeTaxResult` field is `pub` (`se.rs:24–46`). KATs:
  (a) a computed `SeTaxResult` with `base < 400` (e.g. net_se $430 → base **$397.10** — [R0-M1]
  the exact tie $397.1050 resolves to the even cent under ROUND_HALF_EVEN `round_cents`,
  `se.rs:10–11`; NOT $397.11) → the note present, citing §6017, and showing it despite
  pre-factor profit ≥ $400 — the ×0.9235 subtlety pinned; (b) `base ≥ 400` → note ABSENT; (c)
  the existing Chunk-A/B goldens byte-unchanged for their (all ≥ $400) fixtures.
- **D3:** the two binary Usage-error tests (non-zero exit each).
- **D4:** the mode-assertion test (green at HEAD; would have been RED at the mode-644 commit).
- **D5:** the full-schedule equality test (green at HEAD — this is a lock, not a bug-fix; its
  value is regression + the delta-cancellation hole M1 names).

Determinism (BTreeMap iteration everywhere touched); exact Decimal; synthetic fixtures only; no
PII. **Pin re-check at every commit:** `git diff` in `render.rs` confined to 1118–1275.

### Task 2 — whole-diff review (Phase E) + record items + FOLLOWUPS

- Cross-cutting for R0: the three D1 sites match the ReclassifyIncome pattern exactly (effective
  payload, Hard blocker, exclusion, message quality); ManualFmv's latest-wins untouched;
  pass-1b/1c → 1d/1e ordering argument re-verified against the code; the compatibility audit
  record present (every suite fixture triaged); §6017/§1402(b)(2) cites re-verified against the
  statute; the ×0.9235 floor condition on `r.base` (not `net_se`) confirmed; the render.rs diff
  confined to the pinned region and btctax-tui untouched (the lane pin); D5 values verbatim vs
  `SPEC_ty2024_tables.md`; D4 fail-closed.
- Execute D6: the FOLLOWUPS M-1 correction line; the M-2 dot-notation edit (or the R0-chosen
  fallback) + its FOLLOWUPS record.
- FOLLOWUPS: close the Chunk-C I-2 backfill item, the Chunk-B §6017 item, the Chunk-A M-1 item,
  CI N-2/M-1/M-2, and TY2024 M1. Note the export-from-TUI lane as the next queue item (already
  in flight, parallel).

---

## Out of scope

- Anything in `render.rs:556–966` (CSV writers), any new `pub fn` in render.rs, and ALL of
  `crates/btctax-tui/` — **export-lane property** (the coordination pin).
- Suppressing/zeroing SE tax below the $400 floor in `compute_se_tax` (engine change; the note is
  advisory-only, matching the house §164(f)/W-2 advisory posture).
- Link-vs-reclassify and link-vs-classify-inbound *precedence* conflicts (D1's documented
  non-goals); duplicate-semantics changes for ManualFmv.
- The engine-B gross-vs-net `crypto_ord` coordination; Form 8960; TY2026/2027 tables; Windows/
  macOS CI runners; cargo-audit/deny (all remain OPEN in FOLLOWUPS).
- Any migration/rewrite of stored events (append-only; D1 changes projection only).
