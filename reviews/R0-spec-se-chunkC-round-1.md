# R0 architect review — SPEC_se_chunkC_reclassify_income.md (round 1)

**Artifact:** `design/SPEC_se_chunkC_reclassify_income.md`
**Baseline verified against:** HEAD `1b6dfe3` (post Chunk A)
**Reviewer:** R0 (independent architect review; author ≠ reviewer)
**Date:** 2026-07-01

**Verdict: NOT green — 0 Critical / 2 Important / 3 Minor / 2 Nit.** Both Importants are
spec-text fixes (no design change); the architecture itself is sound.

---

## Recon verification (spec citations vs current source @ 1b6dfe3)

| Spec claim | Verified? | Current source |
|---|---|---|
| `river.rs:~145-180` — `"income"`→Reward, `"interest"`→Interest, both `business: false` hard-coded with IMMUTABLE comments | ✅ | `crates/btctax-adapters/src/sources/river.rs:145-181` (comments at 149-154, 168-172) |
| `ClassifyRaw` at `event.rs:~188-191`, resolve handling `~400-410` | ✅ | `event.rs:187-191`; `resolve.rs:393-411` (1c) |
| ReclassifyOutflow collection + duplicate→`DecisionConflict` at `resolve.rs:~487-496` | ✅ | `resolve.rs:487-497` — first-in wins, second gets the blocker |
| `build_op` Income branch `~180-191` → `Op::Income { …, business: x.business }` | ✅ | `resolve.rs:180-191`; `manual_fmv` applied there too (composes orthogonally with the new override) |
| Fold `~642-697` pushes `IncomeRecord { …, kind, business }` from the Op — no fold change needed | ✅ | `fold.rs:642-697` reads only `Op::Income` fields |
| `se_net_income`: `business && kind != Interest && year` | ✅ | `se.rs:53-58` (`.filter(\|i\| i.business && i.kind != IncomeKind::Interest && …)`); M2 test at `se.rs:287-313` pins business-Interest exclusion |
| `crypto_ord` kind/business-agnostic | ✅ | `compute.rs:296-301` — filters on `recognized_at.year()` only |
| `interest_nii` filters on kind only | ✅ | `compute.rs:306-311`; feeds `nii_with` only (352-353), NIIT delta at 398 |
| `compute_tax_year` contains no SE component | ✅ | `TaxResult` (compute.rs:393-406) has no SE field; SE is a separate `se.rs` entry point — the engine-B-invariance KAT is well-defined as worded |
| Kind strings match the existing parser | ✅ | `eventref.rs:122-130` `parse_income_kind`: mining/staking/interest/airdrop/reward, exactly |
| `append_and_save` + decision plumbing | ✅ | `cmd/reconcile.rs:26` (generic over payload); `main.rs:203-…` Reconcile subcommand tree + `dispatch_reconcile` |
| `fingerprint()` → None for decisions | ✅ | `persistence.rs:56-97` — decisions fall through the `_ => return None` catch-all; zero code needed |
| Void "genuinely free" via `VoidDecisionEvent` | ✅ | `resolve.rs:299-331` pass 1a: non-revocable list is Supersede/Reject/Void; SafeHarborAllocation deferred; **everything else** (`Some(_)` arm, line 319-321) is revocable → a new decision variant is voidable with zero code |
| **"mirror how ReclassifyOutflow handles a missing/mismatched target — follow the existing convention exactly"** | ❌ **DRIFT — see I1** | There is no such convention. ReclassifyOutflow does **zero** target validation |

---

## Findings

### I1 — IMPORTANT: the bad-target "ReclassifyOutflow convention" the spec defers to does not exist; D2 and its own KAT are contradictory as written

**The actual convention, verified:** the ReclassifyOutflow collection (`resolve.rs:487-497`)
inserts into `outflow_class` keyed by `ro.transfer_out_event` with **no check that the target
exists or is a TransferOut**. The map is consulted **only** inside `build_op`'s
`EventPayload::TransferOut` branch (`resolve.rs:211`). A ReclassifyOutflow pointing at a
nonexistent event, or at an Acquire/Income/Dispose, is **silently inert** — no blocker, no
error, no panic. `ClassifyInbound` and `ManualFmv` behave identically (blind map inserts at
`resolve.rs:484`, `resolve.rs:421`).

**The contradiction:** D2 says *"mirror how ReclassifyOutflow handles a missing/mismatched
target — follow the existing convention exactly"* while requiring *"a blocker/decision-error"*;
the KAT says *"the blocker/error (per the ReclassifyOutflow convention), not a panic/silent
ignore"*. Mirroring exactly produces silent ignore, which the KAT forbids. The spec cannot be
implemented as written, and a silent no-op on an SE-relevant correction (user believes the
business flip took; SE never moves) is precisely the hazard class this gate exists to catch.

**Fix (spec text, no design change):**
1. Delete every "per/mirror the ReclassifyOutflow convention" reference for bad targets and
   bind concrete behavior: **at pass-1e collection time** (after `applied` is built in 1b/1c,
   so ordering works), an entry whose target's **effective payload** is not
   `EventPayload::Income` — where effective = `applied.get(&t).unwrap_or(&by_id[t].payload)`,
   and a `by_id` miss with no `applied` entry counts as bad — emits a **Hard
   `BlockerKind::DecisionConflict`** (`event: the decision's id`, detail e.g. `"ReclassifyIncome
   targets a non-Income event"`) and is **not inserted** into the map.
2. Cite the real in-repo precedents for validated targeting instead: `TransferLink`'s
   unresolvable-in-event check (`resolve.rs:456-466`, I-1) and `LotSelection`'s
   non-honoring-target check (`resolve.rs:604-611`) — both blocker-on-bad-target.
3. State explicitly that this **diverges from** (improves on) ReclassifyOutflow /
   ClassifyInbound / ManualFmv, all silently inert on bad targets today, and add a FOLLOWUPS
   entry to consider backfilling the same validation there.
4. Why effective-payload (not raw) matters: an `Unclassified` row ClassifyRaw'd into an Income,
   or a superseded import whose accepted payload is an Income, **must be reclassifiable**;
   conversely an Income superseded into a non-Income must be flagged. Raw-payload validation
   gets both wrong. Note also that a TransferIn classified via `ClassifyInbound::Income`
   (`Op::IncomeInbound`) is **correctly rejected** by this rule (its effective payload is still
   TransferIn) — consistent with the spec's out-of-scope line — see N1 for the error-message
   hint.

### I2 — IMPORTANT: the kind-flip NIIT KAT can pass vacuously — pin the fixture above the §1411 threshold

`TaxResult.niit` is a **delta** (`niit_with - niit_without`, compute.rs:398) gated by
`min(NII, MAGI − threshold)` (compute.rs:362-373). With a minimal fixture profile
(`magi_excluding_crypto` at/near zero — the natural choice in existing KATs), `niit` is `0`
before **and** after a `Reward → Interest` flip: the KAT "NIIT moves" would assert `0 == 0`
and enforce nothing. The spec's KAT list demands exact assertions but never constrains the
fixture. **Fix:** one sentence in the KAT bullet — the fixture profile's MAGI must exceed the
filing-status §1411 threshold so `interest_nii` produces a **nonzero** `niit` delta, and the
KAT must assert the exact before/after `niit` values (plus `se_net_income` dropping the flipped
FMV). The reverse direction (`Interest → Mining` leaves NII) should likewise assert the exact
nonzero-to-lower delta, not merely "changed".

### M1 — MINOR: missing no-fingerprint KAT for the new variant

D1 states `fingerprint() → None` but the KAT list omits the test. Repo convention pins each
new decision variant (`method_election_decision_has_no_fingerprint`,
`lot_selection_decision_has_no_fingerprint`, `event.rs:459-482`). It's the `_ => return None`
catch-all — zero code — but pin it like its predecessors.

### M2 — MINOR: duplicate-conflict KAT should also assert which decision remains in force

The mirrored ReclassifyOutflow semantic is **first-wins, second-conflicts**
(`resolve.rs:487-497`) — distinct from LotSelection's *neither-applies* (`resolve.rs:589-601`).
The KAT as written ("two non-voided reclassifies → DecisionConflict") passes under either
semantic. Assert additionally that the projection reflects decision #1's business/kind.

### M3 — MINOR: two stale-doc touch points missing from the file list

(a) The pass-1a revocability comment (`resolve.rs:292-296`) enumerates revocable targets
explicitly — add `ReclassifyIncome` or it's stale the day this ships. (b) The
`every_variant_serde_round_trips` inventory (`event.rs:299-456`) should gain the new variant
(state that the spec's round-trip KAT lands there, both `kind: Some(_)` and `None` arms).

### N1 — NIT: bad-target blocker detail should hint the IncomeInbound correction path

A user whose income came in as a TransferIn + `ClassifyInbound::Income` will plausibly aim
`reclassify-income` at it and hit the I1 blocker. Detail text should hint: "for income
classified via classify-inbound, void the ClassifyInbound and re-classify with the corrected
business/kind."

### N2 — NIT: document the correction flow in CLI help

`DecisionConflict` is **Hard** (`state.rs:62-77`) → a duplicate or bad-target ReclassifyIncome
hard-blocks the whole tax year (`TaxYearNotComputable`) until voided. CLI help for the new
subcommand should say: to change a prior reclassify, `reconcile void` it first, then re-issue.

---

## Mandated evaluations

### 1. Vault back-compat — SOUND (highest-priority check passes)

- **Old vault, new binary:** `load_all` (`persistence.rs:264-…`) deserializes `payload_json`
  via serde's externally-tagged `EventPayload`; old vaults contain no `ReclassifyIncome` rows,
  and adding a variant never changes the decode of existing variants. Loads unchanged. The
  spec's "trivially true — pin it" KAT is honest.
- **New vault, old binary:** first `ReclassifyIncome` row → serde "unknown variant" →
  `CoreError::Persistence` → hard load failure (loud, not silent misreading). Same accepted
  trade-off as `MethodElection`/`LotSelection`; spec documents it. Consistent.
- **No Chunk-2-GiftOut-style trap:** that trap was reshaping an *existing* variant's
  serialization (unit variant + new field). Here it's a brand-new struct variant; no existing
  bytes are reinterpreted. `#[serde(default)]` on `kind: Option<IncomeKind>` is correct
  (permits field omission; `IncomeKind` unit variants serialize as plain strings under
  `Option` without incident).
- **No persistence-layer change needed:** the SQL `kind` column is only
  import/decision/conflict; `append_decision` (`persistence.rs:238-262`) is payload-generic and
  stores `fingerprint = NULL`; `fingerprint()` catch-all returns None; `is_imported()`
  untouched. Zero schema/migration surface — the spec's MINOR SemVer call is right.

### 2. Resolve/override correctness — sound, modulo I1

Collection into a `BTreeMap<EventId, ReclassifyIncome>` with first-wins duplicate conflict
mirrors `outflow_class` exactly (deterministic: decisions iterated in seq order, BTreeMap).
The build_op override (`business = o.business; kind = o.kind.unwrap_or(x.kind)`) lands in the
one place `Op::Income` is built, composes orthogonally with `manual_fmv` (fmv vs kind/business),
and requires no fold change (verified). It also correctly applies to ClassifyRaw'd/superseded
Income payloads, since build_op receives the *effective* payload keyed by the target id. Void
is genuinely free via the pass-1a `Some(_)` catch-all. The only hole was the bad-target clause
(I1) — resolvable purely in spec text.

### 3. Tax interactions — analysis complete and verified

- **Business-only flip:** moves `se_net_income` only. `crypto_ord` is year-filtered only
  (compute.rs:296-301); `interest_nii` is kind-filtered only (306-311); no other computational
  consumer of `IncomeRecord.business` exists (full-workspace sweep: se.rs, fingerprint (imported
  Income only), `render.rs:301/709` and TUI `tabs/income.rs:44` + `tabs/tags.rs` — all
  display/CSV-only). Engine-B invariance is a true invariant and the KAT is correctly scoped to
  `compute_tax_year` figures (which contain no SE term).
- **Kind flip to/from Interest:** moves `interest_nii` (→ NIIT, subject to I2's fixture
  requirement) and the SE filter. Interest is excluded from SE **regardless of business**
  (se.rs:57; pinned by the existing M2 test) — so `--business true --kind interest` yields
  SE-none + NII-in, which is the correct §1402(a)(2)/§1411(c)(1)(A)(i) split. `crypto_ord` is
  unmoved by any kind flip (all IncomeKinds ordinary). No missed consumer.

### 4. Decision-vs-side-table — correct

This changes projected state (`IncomeRecord.business/kind` → SE and potentially NIIT), so it
needs the event-sourced decision machinery: audit trail, `DecisionConflict`, `VoidDecisionEvent`
revert, deterministic replay. Consistent with — and correctly distinguished from — the 3b
side-table rationale (pure form metadata, no projected-state effect).

### 5. Scope / right-sizing / TDD — good

Single implementation task + whole-diff review task fits a one-variant decision that reuses
`parse_income_kind`, `append_and_save`, and the void machinery wholesale. KAT set covers the
headline flip, both invariances, the conflict, void-revert, bad-target, and back-compat — all
genuine synthetic-fixture tests (subject to I1's rewording and I2's fixture pin, plus M1/M2
additions). Out-of-scope list (Chunk B, FMV/amount edits, non-Income targets, old-binary
migration, batch, future-year tables) is sane.

---

## Required to reach green

| # | Severity | Fix |
|---|---|---|
| I1 | Important | Replace the phantom "ReclassifyOutflow bad-target convention" with concrete behavior: pass-1e effective-payload validation → Hard `DecisionConflict`, decision excluded; cite TransferLink/LotSelection precedents; note the deliberate divergence + FOLLOWUPS backfill item |
| I2 | Important | Pin the kind-flip NIIT KAT fixture: MAGI above the §1411 threshold; assert exact nonzero `niit` deltas both directions |
| M1 | Minor | Add the `reclassify_income_decision_has_no_fingerprint` KAT (repo convention) |
| M2 | Minor | Duplicate KAT: also assert the FIRST decision remains in force |
| M3 | Minor | Add `resolve.rs:292-296` revocability comment + `every_variant_serde_round_trips` inventory to the touch list |
| N1 | Nit | Bad-target blocker detail: hint the ClassifyInbound void+re-classify path |
| N2 | Nit | CLI help: correction flow = void + re-issue (DecisionConflict is Hard) |

---

# Round 2 — re-review (post-fold)

**Artifact:** `design/SPEC_se_chunkC_reclassify_income.md` (revised)
**Baseline re-verified against:** HEAD `1b6dfe3` (confirmed via `git rev-parse` at review time)
**Reviewer:** R0 (independent architect re-review; author ≠ reviewer)
**Date:** 2026-07-01
**Scope:** confirm the I1/I2/M1–M3 folds; scan for new findings and internal consistency. The
round-1 confirmations (back-compat soundness, resolve/override correctness, tax-interaction
analysis, decision-vs-side-table, right-sizing) are not re-litigated.

## Verdict: **GREEN — I1 and I2 closed, all three Minors folded, 0 Critical / 0 Important / 0 new Minor. Ready to implement.**

## Fold verification

### I1 — CLOSED ✅ (verified against source, not just spec text)

D2 now binds fully concrete bad-target behavior with no reference to the phantom
"ReclassifyOutflow convention":

- **Pass-1e collection-time validation** — verified implementable: `resolve.rs:425` is the
  `1e. Classification decisions` pass header, and `applied` is fully built beforehand in 1b
  (`resolve.rs:336` declaration, `:381` supersede inserts) and 1c (`:408` ClassifyRaw inserts).
  Ordering works exactly as the spec claims.
- **Effective-payload formula** — `applied.get(&target).unwrap_or(raw)`, with a `by_id` miss
  counting as bad. This is the *same idiom pass 2 already uses* at `resolve.rs:512`
  (`applied.get(&e.id).unwrap_or(&e.payload)`), so the rule is native to the codebase, covers
  ClassifyRaw'd-to-Income rows (reclassifiable) **and** superseded imports (the `applied` map
  holds 1b supersede results too — the formula subsumes the round-1 fix's supersede clause even
  though the spec's parenthetical only names ClassifyRaw), and correctly rejects
  ClassifyInbound-Income TransferIns (effective payload still TransferIn).
- **Consequence** — Hard `BlockerKind::DecisionConflict` + the decision **excluded** from the
  override map. Matches the round-1 prescription verbatim.
- **Precedents** — re-verified live: `resolve.rs:456-466` (TransferLink: Hard
  `DecisionConflict` blocker on an unresolvable in-event, decision not honored) and
  `resolve.rs:604-611` (LotSelection: blocker + `retain`-based exclusion on a non-honoring
  target). Both citations accurate at HEAD.
- **Divergence + backfill** — the deliberate divergence from ReclassifyOutflow is called out
  with a required code comment, and the FOLLOWUPS item to backfill validation onto
  ReclassifyOutflow is specified (out of scope here — correct).
- **KAT** — covers BOTH bad shapes (missing event; existing non-Income event), each asserting
  the Hard blocker **and** unchanged projection, "NOT a panic and NOT silently inert". The D2
  rule and the KAT now agree; the round-1 contradiction is gone.

Well-defined and implementable. **Closed.**

### I2 — CLOSED ✅

The kind-flip NIIT KAT now (a) **mandates** the fixture profile's MAGI exceed the §1411
filing-status threshold, and (b) requires **exact nonzero `niit` deltas in both directions**
(`Reward → Interest`: niit rises by the derived amount, SE still excludes; `Interest → Mining`:
niit falls to the derived value, enters SE). Non-vacuity is now guaranteed *by construction*:
because the KAT itself asserts nonzero deltas, a sub-threshold fixture would make the KAT
**fail**, not pass vacuously — the failure mode round 1 flagged is structurally unreachable.
The SE-side assertions in each direction are retained. **Closed.**

### Minors — all folded ✅

- **M1** — `ReclassifyIncome.fingerprint() == None` KAT present (repo convention, matching
  the MethodElection/LotSelection predecessors). ✅
- **M2** — the duplicate-conflict KAT now asserts **FIRST-WINS for the projected value**,
  disambiguating from LotSelection's neither-applies semantic. ✅
- **M3** — both stale-doc touch points present: the new variant added to the
  `every_variant_serde_round_trips` inventory (both `kind` arms via the back-compat bullet's
  "with and without `kind`") and the `resolve.rs:~292-296` revocability doc-comment fix. They
  live in the KAT bullet rather than the Task-1 file list, but both files (`event.rs`,
  `resolve.rs`) are already in that list — substance satisfied. ✅

## New-finding scan — nothing at Critical/Important/Minor

- **Internal consistency:** D2's validation rule ↔ the Task-1 bad-target KAT now agree
  (blocker kind, Hard severity, exclusion, both shapes). The duplicate rule (first-wins →
  `DecisionConflict`) and the bad-target rule compose without ambiguity the KATs would leave
  untested.
- **Still no fold change:** the override remains build_op-only (spec lines on
  `resolve.rs:~180-191` / `fold.rs:~642-697` unchanged and still accurate).
- **Engine-B invariance and back-compat KATs intact**, unmodified from the round-1-approved
  form; the SemVer/old-binary trade-off text unchanged.
- **Right-sizing unchanged:** one implementation task + whole-diff review remains correct for
  a single-variant decision reusing `parse_income_kind`, `append_and_save`, and the void
  machinery.
- Interleaving of duplicate-vs-bad-target checks within 1e (e.g., decision #2 both duplicate
  and bad-target) is observably immaterial — both paths yield a Hard `DecisionConflict` on the
  second decision and exclude it. Not a finding.

## Residual (non-blocking)

- **N1, N2 (round-1 Nits) remain unfolded** — the bad-target blocker-detail hint for the
  ClassifyInbound void+re-classify path, and the CLI-help correction-flow note (void +
  re-issue, since `DecisionConflict` is Hard). Both are one-line implementation-time niceties;
  neither gates green. Recommend picking them up during Task 1 or logging with the FOLLOWUPS
  backfill item.

## Gate decision

**0 Critical / 0 Important. The spec is R0 GREEN — proceed to implementation
(IMPLEMENTATION_PLAN / Task 1 TDD).**
