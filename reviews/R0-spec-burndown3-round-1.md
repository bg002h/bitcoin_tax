# R0 — architect review, round 1 — `design/SPEC_followups_burndown3.md`

- **Artifact:** `design/SPEC_followups_burndown3.md` (untracked, pre-implementation)
- **Baseline verified against:** `main` @ `4125db3` (matches the spec's declared baseline; HEAD confirmed)
- **Reviewer:** R0 (independent; author ≠ reviewer)
- **Date:** 2026-07-02
- **Verdict: NOT green — 0 Critical / 1 Important / 3 Minor / 4 Nit.** One spec fix (I-1, one
  sentence) plus the Minor text corrections, then re-review. The substantive design (D1, the
  mutating-TUI safety prerequisite) is sound as specified.

---

## 1. Verification log (independent, against current source and primary legal sources)

### 1.1 Item 1 — the bad-target backfill (D1) — highest priority

**Citations.** Every resolve.rs citation checked verbatim at `4125db3` — all exact:

| Spec claim | Verified |
|---|---|
| ReclassifyIncome pattern, pass 1e `resolve.rs:510–575`; divergence comment + FOLLOWUP at `515–523` | ✓ exact (FOLLOWUP line at 523: "backfill equivalent validation onto ReclassifyOutflow") |
| ReclassifyOutflow blind duplicate-only collector `499–509` | ✓ exact (duplicate → `DecisionConflict`, else blind insert) |
| ClassifyInbound blind duplicate-only collector `488–498` | ✓ exact |
| ManualFmv pass 1d `423–433`, latest-seq-wins documented at 424, NO duplicate blocker | ✓ exact ("latest decision_seq wins (ascending iteration = last write wins)") |
| `build_op` sole consumers: `outflow_class.get(id)` at 220 (TransferOut arm 201–250); `inbound_class.get(id)` at 255 (TransferIn arm 251–281); `manual_fmv.get(id)` at 184 (Income arm 183–200) | ✓ exact. **Sole-consumption independently confirmed by grep:** none of the three maps is read anywhere in `btctax-core/src` outside `resolve.rs`; within `resolve.rs` each is consulted only in the cited `build_op` arm. |
| Doc comment 158–159 ("ManualFmv on an `Income` replaces the FMV") | ✓ |
| `InboundClass::Income` carries its own `fmv` (`257–266`) — ManualFmv NOT consulted in the `IncomeInbound` path | ✓ verified: `Op::IncomeInbound { fmv: *fmv, .. }` takes the classification's fmv; `manual_fmv` is untouched in that arm. The spec's claim that a ManualFmv pointing at a TransferIn-classified-as-income row is inert today is **correct**, and the D1 redirect message ("set the FMV via classify-inbound-income (its own `fmv` field)") is **accurate**. Both redirect targets are real CLI subcommands (`main.rs:218 ClassifyInboundIncome`, `296 ReclassifyIncome` — clap kebab-case `classify-inbound-income` / `reclassify-income`). |
| Ordering: pass 1b `346–401` + pass 1c `403–421` fully populate `applied` before 1d (423) and 1e (435) | ✓ verified — straight-line code in `resolve()`; the effective-payload idiom works verbatim at all three sites. |
| `VoidDecisionEvent` remedy: ReclassifyOutflow/ClassifyInbound/ManualFmv all revocable (`301–306`) | ✓ verified (revocable-targets comment lists all three; `Some(_)` arm at 329–331 voids them). |

**Per-type expected-payload table** (ReclassifyOutflow→TransferOut, ClassifyInbound→TransferIn,
ManualFmv→Income): **correct and complete.** Each map has exactly one consumer arm; the expected
type is exactly the payload the consumer matches on; there is no second consumption path a
validation could miss.

**Effective-payload idiom** (`by_id.get(target).map(|raw| applied.get(target).unwrap_or(&raw.payload))`):
**correct for all three types.** The timeline build (`resolve.rs:590`) hands `build_op` the same
effective payload (`applied.get(&e.id).unwrap_or(&e.payload)`), so collection-time validation and
consumption-time dispatch judge the identical payload: a ClassifyRaw'd Unclassified→TransferOut row
is validly ReclassifyOutflow-able, a SupersedeImport'd Income→TransferIn row correctly rejects a
ManualFmv, and a `by_id` miss (including an `applied` entry whose target never existed) is bad at
both ends. No hole found.

**ManualFmv latest-wins preserved:** D1 rule 3 explicitly adds ONLY the missing/mismatched-target
arms in pass 1d, no duplicate blocker; the plan's KAT ("two sequential valid ManualFmv on the same
Income → the LATER value projects, NO DecisionConflict") locks it against over-harmonization with
the 1e first-wins maps. **Verified sound** — a valid re-target still overwrites; a bad-target
decision is excluded per-decision without disturbing an earlier valid one. This satisfies the
user-mandated correction-flow semantic.

**Documented non-goals** (TransferLink precedence over ReclassifyOutflow, `consumed_ins` over
ClassifyInbound): verified against `build_op` — links checked at 202–218 before `outflow_class`
at 220; `consumed_ins` at 252–254 before `inbound_class` at 255. Correctly characterized as
precedence questions, not bad targets; leaving them unblocked is the right call (blocking would
convert a legitimate layered decision into a false conflict).

**Compatibility posture:** honestly stated, remedy real (void, zero new code), no migration owed
(projection-only change over an append-only ledger), full-suite fixture audit mandated with
per-fixture triage. Spot-checks support "none are known": `reconcile.rs:280–348` (valid ManualFmv
on an FmvMissing Income — cited fixture verified), `properties.rs:319` (the only property-test
ReclassifyOutflow, targeting a real gift-out event). One understatement — see M-3. `DecisionConflict`
severity is Hard (`state.rs:62–77`) and `first_hard_blocker` gates `compute_tax_year`
(`compute.rs:242`, `445–450`) — the gating claim is correct.

**Mutating-TUI prerequisite:** satisfied as designed — after D1, no decision of these four types
can be appended against a wrong/missing target without a Hard, message-bearing, voidable blocker.

### 1.2 Item 2 — §6017 (web-verified against primary sources)

- **26 U.S.C. §6017** (law.cornell.edu, fetched this review): *"Every individual (other than a
  nonresident alien individual) having net earnings from self-employment of $400 or more for the
  taxable year shall make a return with respect to the self-employment tax imposed by chapter 2."*
  — **verbatim match** with the spec's quote. ✓
- **26 U.S.C. §1402(b)(2)** (fetched this review — the cite the drafter flagged for me): *"the net
  earnings from self-employment, if such net earnings for the taxable year are less than $400."*
  — **the companion cite is CORRECT**: sub-$400 net earnings are excluded from "self-employment
  income", and §1401 imposes tax only on self-employment income, so no §1401 tax below the floor.
  The note's framing and the cite number both check out. ✓
- **The `r.base < 400` condition is the right operand:** the $400 test in both §6017 and
  §1402(b)(2) is on "net earnings from self-employment" per §1402(a), which includes the
  §1402(a)(12) 7.65% reduction — i.e. the ×0.9235 figure. `r.base = round_cents(net_se × 92.35%)`
  (`se.rs:32–33`, `123`) is exactly that quantity (Schedule SE line 4c applies the test post-factor).
  Condition confirmed correct; strict `<` matches "$400 or more". ✓ (But see M-1: the spec's
  worked example mis-rounds, and M-2: the flush language after §1402(b)(2) creates a church-employee
  exception the note's "only" ignores.)
- **Text-only / export-lane pin:** D2 confines the edit to the `Some(r)` arm of
  `render_schedule_se` (verified region `render.rs:1118–1275`; `Some(r)` arm 1128–1238; W-2
  disclosure 1214–1229; standalone note 1231–1236 — all cited line ranges exact). No writer, no
  TUI, no `compute_se_tax` change. ✓ Existing SE render fixtures all have base ≥ $400 (92,350.00
  and 73,880.00 — `render.rs:2402–2476`), so the "goldens byte-unchanged" claim in KAT (c) holds
  at HEAD. ✓

### 1.3 Items 3–5

- **D3 (negative-flag binary tests):** guards verified at `main.rs:738–740` (`--w2-ss-wages`) and
  `746–750` (`--w2-medicare-wages`) — exact. House pattern verified at `tax_report.rs:1038–1073`
  and `690–716` — exact, and genuinely binary-level (`CARGO_BIN_EXE_btctax`, non-zero exit).
  FOLLOWUPS carries the open half at line 116. Design correct. ✓
- **D4 (mode-assertion KAT):** `scripts/pre-push` and `scripts/pii-scan-generic.sh` verified
  100755 in the index AND on disk. **CI compatibility verified:** `.github/workflows/ci.yml` runs
  `actions/checkout` + `cargo test --workspace --locked`, so the test executes inside a real git
  checkout; `git ls-files -s` reads index modes populated from the committed tree, so 100755 is
  asserted correctly in CI (and in worktrees). Two `parent()` hops from `btctax-cli`'s manifest
  dir reach the repo root. Fail-closed posture (no skip arm) is right — the regression this locks
  was itself a fail-open. ✓ (See N-2: say `current_dir(repo_root)` explicitly.)
- **D5 (TY2024 full-schedule KATs):** `ty2024()` at `tax_tables.rs:96–215`, KATs at 583–763,
  A1 583–593 (3 of 7 Single edges), A2 598–617, A3 622–662 (all four LTCG pairs by struct
  equality + Qss≡MFJ) — all cited ranges exact. **Transcription source verified:** all 28
  ordinary edges in `SPEC_ty2024_tables.md` §3.01 Tables 1–4 match the builder digit-for-digit
  (independently compared this review), and I spot-checked four values against Rev. Proc. 2023-34:
  Single 22% start $47,150; MFJ 37% start $731,200; HoH 35% start $243,700 (the deliberate
  divergence from Single/MFS $243,725); MFS 37% start $365,600 — all correct. The
  delta-cancellation rationale (A6a–A6d assert marginal deltas that a lower-edge transposition
  can cancel) is valid; the full-schedule equality test closes it. FOLLOWUPS TY2024 M1 verified
  open at line 46. ✓ (See N-3 on the Qss storage assertion.)

### 1.4 Coordination pin

Writer region confirmed: `write_csv_exports` at `render.rs:567` (doc from 556) through
`write_schedule_d_csv` ending ~960; `render_schedule_se` at exactly 1118–1275; btctax-tui
untouched by every task in the plan. The pin is enforceable via per-commit diff-hunk inspection
**except for one self-contradiction the spec itself introduces — see I-1.** Also N-4 (region
description imprecision, harmless for this lane).

### 1.5 Item 6 — record items

- **CI M-1:** `reviews/whole-branch-review-ci-round-1.md:188–196` verified — the spec's
  transcription (MSRV-gated lint, fires only after the 1.88 bump, causality *stronger* not
  weaker) is faithful. FOLLOWUPS carry at line 30 verified. ✓
- **CI M-2:** `reviews/whole-branch-review-gift-chunk3b-round-1.md:30` verified — the two
  real-hyphen synthetic tokens are on exactly that line; the CI review's mandate (M-2, lines
  198–207: "dot-notate that old review or amend the spec's 'NOWHERE else' sentence… A decision,
  not silence") is quoted accurately. **R0 concurs with the recommendation (dot-notate + bracketed
  audit note):** the parent reviewer explicitly authorized this option; the tree-wide grep
  invariant is worth more than hyphen fidelity in an archived review; the meaning-preserving
  re-encoding with an in-line editorial marker does not alter reviewer substance, so the
  verbatim-persistence rule is respected in spirit and the change is auditable. No finding.

### 1.6 Scope / right-sizing / TDD genuineness

Scope is six genuinely small items, one substantive (D1), correctly sequenced ahead of the
mutating TUI. TDD is genuine where it can be: D1's six blocker KATs and the void-remedy KAT are
RED at HEAD (the silent-inert behavior produces no blocker today); D2(a) is RED; D4/D5 are
honestly labeled locks (green at HEAD), with D4's counterfactual red (the mode-644 commit)
stated. The D1 per-type missing/wrong-type/happy-path matrix (2×3 + 3 + latest-wins + void) is
the right adversarial shape, and the wrong-type pairings match the redirect messages. The core
KAT neighborhood the plan cites exists (`reclassify_income.rs:632–718`, missing-event +
non-Income bad-target KATs to replicate).

---

## 2. Findings

### Critical

None.

### Important

**I-1 — The D2 KAT-location option contradicts the spec's own per-commit render.rs pin (and
courts the exact merge conflict the pin exists to prevent).**
Task 1 says: *"D2 render KATs (unit, `render.rs` test mod or `tax_report.rs`)"* — but the same
task mandates *"Pin re-check at every commit: `git diff` in `render.rs` confined to 1118–1275"*,
and the coordination pin declares any render.rs conflict outside `render_schedule_se` "a process
violation". The existing `render_schedule_se` KATs live in **render.rs's own test module**
(`render.rs:2396+`, header: "P2-D Task 2 / Chunk A + Chunk B KATs — `render_schedule_se` +
`schedule_se.csv`"), i.e. far outside 1118–1275 — and that same submodule is where the export
lane will naturally add `schedule_se.csv`/writer KATs (`write_csv_exports` tests sit at
2806/2844/2939). An implementer taking the spec's first option violates the mandatory per-commit
check and risks a textual conflict with the parallel lane.
**Exact fix:** strike "`render.rs` test mod or" — mandate `tax_report.rs` for the D2 KATs.
Feasibility verified: `pub mod render` (`lib.rs:10`), `render_schedule_se` is `pub`, and every
`SeTaxResult` field is `pub` (`se.rs:24–46`), so the fixtures are constructible from the
integration test.

### Minor

**M-1 — The D2 worked example mis-rounds under the house ROUND_HALF_EVEN: $430 × 0.9235 =
$397.1050 → $397.10, not $397.11.**
Both occurrences (D2 legal-grounding paragraph "base $397.11 < $400" and Task-1 KAT (a)
"net_se $430 → base $397.11") hit an exact tie ($397.1050) that `round_cents` (ROUND_HALF_EVEN,
`se.rs:10–11`) resolves to the even cent: **$397.10**. The floor argument is unaffected
(397.10 < 400 either way), but the spec mandates "assert EXACT", and a hand-transcribed fixture
pinning 397.11 is internally inconsistent with `compute_se_tax` for net_se $430 (RED for the
wrong reason if any KAT derives the base from the engine).
**Exact fix:** change both occurrences to $397.10 (or pick a non-tie example, e.g. net_se $432 →
base $398.952 → $398.95).

**M-2 — The note's "a Schedule SE filing is required only when…" is overbroad: the
church-employee-income exception.**
Verified this review: the flush language after §1402(b)(2) reads *"In the case of church employee
income, the special rules of subsection (j)(2) shall apply for purposes of paragraph (2)"* —
§1402(j)(2) substitutes a lower floor for church employee income (Schedule SE line 4c carries the
matching exception: "If less than $400 and you had church employee income, enter -0- and
continue"). The app does not model church employee income, so the note as drafted can be wrong
for a filer who has it. One clause fixes it at house precision.
**Exact fix:** append to the note, e.g.: "…(§1402(b)(2)) — church employee income excepted
(§1402(j)(2), not modeled) — the figures above are shown for transparency." (or soften "required
only when" to "required on account of this income only when").

**M-3 — The compatibility posture understates the blast radius: the Hard gate is
projection-wide, all years.**
`compute.rs:237–242` (verified): "ANY unresolved severity()==Hard blocker ANYWHERE in the
projection gates EVERY year — deliberately conservative." A stale 2023-era bad-target decision
therefore refuses **every** `compute_tax_year`, not just reports touching the target. The spec's
"may refuse a previously-'computing' report" is true but weaker than the actual behavior. This
*strengthens* the case for the mandated fixture audit and for loud-over-silent, so the posture's
conclusion stands — but the gate statement should be plain.
**Exact fix:** in D1 §"Compatibility posture", after "(because Hard blockers gate
`compute_tax_year`)", add "— projection-wide: one stale decision gates EVERY year
(`compute.rs:237–242`)".

### Nit

**N-1 — "the projection is byte-equal to the same fixture without the decision" is literally
unsatisfiable (the blocker lists differ by construction).** The parenthetical
(disposals/removals/income/holdings unchanged) already carries the real assertion — reword to
"the projected state fields (disposals/removals/income/holdings) are equal", matching how
`reclassify_income.rs:632–718` asserts it.

**N-2 — D4 should state `Command::current_dir(repo_root)` explicitly.** `cargo test` runs
integration tests with cwd = the crate manifest dir; `git ls-files -s scripts/pre-push` resolves
pathspecs relative to cwd, so the two-hop root must be set as the child's working directory (the
spec implies it via "locates the repo root"; say it).

**N-3 — D5's "if storage is not reachable from tests" hedge is moot: `TaxTable::ordinary` and
`ltcg` are `pub` fields (`tables.rs:58, 61`).** Drop the fallback; mandate the direct
`ordinary.get(&FilingStatus::Qss).is_none()` + `ltcg.get(&FilingStatus::Qss).is_none()`
assertions (plus the existing `ordinary_for(Qss) == ordinary_for(Mfj)` alias check).

**N-4 — The pinned "CSV-writer region (556–966)" also contains a non-writer:
`render_donation_appraisal_advisory` (pub text renderer, `render.rs:788–810`), and lines 962–966
are `render_tax_outcome`'s doc comment.** Harmless for THIS lane (over-inclusive forbidden area
is safe), but the description "write_csv_exports + all write_*_csv fns + a new pub fn" is not
what the range holds — worth one clarifying clause so the export lane's own spec doesn't inherit
the imprecision.

---

## 3. Disposition

Gate: **blocked** on I-1 (one-sentence fix) — plus M-1/M-2/M-3 text corrections (each has an
exact fix above). No design change is required anywhere: D1's architecture, the per-type table,
the effective-payload idiom, the ManualFmv latest-wins preservation, the compatibility remedy,
the §6017/§1402(b)(2) legal grounding (both verified against the statute text this review), and
the D3/D4/D5 test designs are all verified sound. After the folds, re-review per §2 (round 2
should be fast — the fixes are mechanical and each carries its exact wording here).

---

# Round 2 — fold confirmation (2026-07-02)

Re-read the folded spec in full against the round-1 findings. All eight folds verified at their
sites:

- **I-1 — CONFIRMED at all three sites.** The pin now covers tests explicitly ("render.rs's own
  test module (2396+) … is export-lane territory — this burndown adds no tests there [R0-I1]");
  the Task-1 Files list restricts render.rs to "production line only — NO tests in render.rs's
  test module"; and the D2 KAT bullet mandates "in `tax_report.rs` ONLY", with the feasibility
  citations (`lib.rs:10`, pub `render_schedule_se`, pub `SeTaxResult` fields) carried in. The
  "render.rs test mod or" option is gone; the per-commit 1118–1275 diff check is now internally
  consistent. Closed.
- **M-1 — CONFIRMED, both occurrences.** D2's legal-grounding paragraph and the Task-1 KAT (a)
  both now read $430 → $397.1050 exact tie → **$397.10** under ROUND_HALF_EVEN (`se.rs:10–11`),
  with the tie mechanism stated. Arithmetic re-checked this round: correct. Closed.
- **M-2 — CONFIRMED, statutorily accurate.** The final note text reads: *"…a Schedule SE filing
  is required on account of this income only when net earnings from self-employment (the ×92.35%
  base, §1402(a)) are $400 or more (§6017), and below that floor no §1401 SE tax is imposed
  (§1402(b)(2); church employee income excepted — §1402(j)(2), not modeled)…"*. The carve-out is
  placed exactly where the statute puts it — the flush language after §1402(b)(2) applies the
  §1402(j)(2) special rules "for purposes of paragraph (2)", i.e. it modifies the (b)(2)
  exclusion the parenthetical wraps — and "not modeled" prevents any overclaim for a filer who
  has church employee income. The §6017 sentence remains verbatim-correct. Closed.
- **M-3 — CONFIRMED.** The compatibility posture now states the projection-wide gate plainly
  ("one stale decision gates EVERY year's compute, not just reports touching the target,
  `compute.rs:237–242`" — quote verified) and correctly folds the all-years blast radius into
  the loud-over-silent argument. Closed.
- **N-1..N-4 — CONFIRMED.** N-1: the missing-target KAT now asserts the projected state fields
  (disposals/removals/income/holdings), matching `reclassify_income.rs:632–718`. N-2: D4 states
  `Command::current_dir(repo_root)` explicitly with the cwd rationale. N-3: D5 mandates the
  direct `tt.ordinary.get(&Qss).is_none()` + `tt.ltcg.get(&Qss).is_none()` assertions on the pub
  fields; the "if reachable" hedge is gone. N-4: the pin now describes the 556–966 range honestly
  as a deliberately over-inclusive forbidden area and delegates precise ownership description to
  the export lane's spec. All closed.
- **D6 hedge — resolved to R0's round-1 concurrence** ("the decision is settled, no fallback
  needed"), correctly quoting §1.5. Closed.

**New findings this round: two Nits, both non-blocking (green = 0C/0I):**

**N-5 (Nit) — stale fallback parenthetical in Task 2.** Task 2 still says "the M-2 dot-notation
edit **(or the R0-chosen fallback)**" while D6 now records the decision as settled with "no
fallback needed". Two-word fix: strike "(or the R0-chosen fallback)". Editorial; may be folded
at implementation and swept by the Task-2 whole-diff review — no re-gate required.

**N-6 (Nit, observation) — mixed-SE-income scope.** The note's `{base}` is crypto-only; a filer
with OTHER (non-crypto) self-employment income combines all activities for the statutory $400
test, so their true net earnings can exceed $400 while `r.base < 400`. The note as worded stays
statutorily accurate (it states the rule conditionally and never asserts "you need not file"),
and this is the report's established, pervasive model boundary (same posture as the §164(f)/W-2
advisories: "coordinate it on your actual return"). Optional hardening if desired at
implementation: append "…transparency (other self-employment activities, if any, combine on your
actual Schedule SE)." Not a blocker; recorded for the implementer's discretion.

## Round-2 disposition

**0 Critical / 0 Important — R0 GREEN.** The spec is internally consistent, all round-1 findings
are folded faithfully with inline [R0-…] tags and a fold record, and no new blocking findings
emerged. Ready to implement in the parallel worktree lane under the coordination pin as written.
