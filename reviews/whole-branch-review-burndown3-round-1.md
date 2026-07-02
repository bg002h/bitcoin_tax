# Whole-branch review (Phase E), round 1 — feat/burndown3 (burndown-3, D1–D5)

- **Branch:** `feat/burndown3` @ `553cb8d` (worktree `/scratch/code/bitcoin_tax-burndown`), off `main` @ `4125db3` — HEAD/base verified.
- **Scope reviewed:** the full 6-commit diff `4125db3..553cb8d` against `design/SPEC_followups_burndown3.md`
  (R0 GREEN, 2 rounds) and the Task-1 report (`.superpowers/sdd/burndown3-report.md`).
- **Reviewer:** independent (author ≠ reviewer). Every claim below verified against the worktree source
  or by running the tools; nothing taken from the report on faith.
- **Date:** 2026-07-02
- **Verdict: READY TO MERGE — 0 Critical / 0 Important / 0 Minor / 4 Nit.**
  Merge-order note: this branch lands **SECOND**, after the export-from-TUI lane, and rebases its
  **single** `render.rs` hunk (see §2). No conflict is possible if the export lane honored its own pin:
  this branch's only render.rs change sits at 1227 inside `render_schedule_se` (1118–1290), far from the
  export lane's CSV-writer region (556–966) and render.rs test module (2411+) — the hunk will shift line
  numbers and apply cleanly.

---

## 0. Diff-package integrity

`.superpowers/sdd/review-4125db3..553cb8d.diff` regenerated and compared: **byte-identical** to
`git diff -U10 main..HEAD`. Worktree clean; commits match the package header. The review package is
faithful — everything below reviewed against it and against live source.

## 1. Item 1 — the bad-target backfill (D1) — highest priority

**Verdict: sound. No validation hole found; the re-target semantic is intact and genuinely locked.**

- **Per-type collection-time validation, all via the EFFECTIVE payload.** All three sites use the
  byte-identical idiom `by_id.get(target).map(|raw| applied.get(target).unwrap_or(&raw.payload))`
  (ManualFmv `resolve.rs:436–438`, ClassifyInbound `531–533`, ReclassifyOutflow `579–581` — same as
  ReclassifyIncome `636–638`). Three-arm match per site: `None` (by_id miss) → Hard
  `DecisionConflict` + EXCLUDED; expected type (`Income`/`TransferIn`/`TransferOut` respectively) →
  existing insert semantics; `Some(_)` wrong type → Hard `DecisionConflict` + EXCLUDED. The
  match is exhaustive — there is no arm through which a bad-target decision can reach the map.
  `DecisionConflict` maps unconditionally to `Severity::Hard` (`state.rs:68→77`, verified), and the
  ordering argument holds in the shipped code: `applied` is fully populated by pass 1b (343) + 1c
  (403) before 1d (423) and 1e (476) run.
- **ManualFmv LATEST-WINS re-target preserved.** The valid-Income arm is an unconditional
  `manual_fmv.insert(...)` — no duplicate blocker (`resolve.rs:453–457`). The KAT
  `manual_fmv_latest_seq_wins_no_duplicate_blocker` **genuinely locks it**: two sequential valid
  ManualFmv on one Income assert BOTH `count_decision_conflicts == 0` AND the later value
  ($2,000) projecting. A duplicate-blocker over-harmonization fails the count assertion; a
  first-wins over-harmonization fails the value assertion. Both failure modes are pinned.
  This satisfies the user-mandated correction-flow semantic.
- **Redirect messages present and accurate** (verified in source): ReclassifyOutflow wrong-type →
  "…for Income corrections use reclassify-income; void the decision to clear this blocker";
  ManualFmv wrong-type → "…for a TransferIn classified as income, set the FMV via
  classify-inbound-income (its own `fmv` field); void this decision". Both redirect targets are real
  clap subcommands (R0-verified at spec time; unchanged on this branch). All six messages advertise
  the void remedy.
- **The 11 KATs (`crates/btctax-core/tests/decision_bad_target.rs`) are genuine.** All six blocker
  KATs assert exactly-one `DecisionConflict` **filtered by kind** (so the TransferIn fixtures'
  expected `UnknownBasisInbound` cannot mask a missing/extra conflict) AND the excluded-decision
  projection: disposals/removals empty, income count/FMV unchanged — matching the [R0-N1] reference
  bar (`reclassify_income.rs:632–718` asserts the same way: kind-filtered count + material state
  fields). Wrong-type pairings are the spec's adversarial ones (ReclassifyOutflow→Income,
  ClassifyInbound→TransferOut, ManualFmv→TransferIn). Happy paths ×3 assert zero conflicts + the
  correct projected artifact (removal / gift lot / FMV override with FmvMissing suppressed). The
  void-remedy KAT (bad ReclassifyOutflow + `VoidDecisionEvent`) asserts the blocker is GONE.
  RED-before verified by construction: at `main` the collectors insert blindly, so all six blocker
  KATs and the void KAT fail there (no blocker emitted).
- **ReclassifyIncome's existing validation untouched.** The diff's only +/- lines in that arm are
  comment lines (the stale divergence/FOLLOWUP comment rewritten as the spec's rule 4 mandates);
  independently confirmed by extracting the arm from `main:` and `HEAD:` and diffing with comments
  stripped — code-identical. The two D1 non-goal comments (TransferLink precedence over both maps)
  sit at the correct valid-target arms and their factual claims match `build_op` (links at 202–218
  before `outflow_class` at 220; `consumed_ins` before `inbound_class`).
- **Fixture audit (zero-reliance claim) — spot-checked, holds.** Grepped every construction of the
  three decision types across the workspace test suites (`kat_tax.rs` ×30, `reconcile.rs` ×8,
  `transition.rs`, `properties.rs`, `lot_selection.rs`, `fixtures.rs`, `fr9_exit_code.rs`,
  `verify_report.rs`): every target is a fixture-local ref ("OUT", "IN", "GIFT", "GOUT", "OD", …)
  whose event exists in the same fixture (spot-verified `kat_tax.rs:345` ClassifyInbound→"IN" and
  `kat_tax.rs:563` ReclassifyOutflow→"OUT" against their fixture events; `reconcile.rs:275–348` is
  the valid FmvMissing ManualFmv R0 already verified; `reconcile.rs:450` builds a ManualFmv only for
  serialization, never projected). **709/709 green with zero fixture modifications** confirms it in
  aggregate. No test anywhere constructed a bad-target decision expecting silence.
- **Compatibility posture:** as specced — silent-inert decisions now surface Hard blockers that gate
  every year's compute; remedy (void) exists with zero new code and is KAT-proven. The
  mutating-TUI safety prerequisite is discharged: none of the four decision types can be appended
  against a wrong/missing target without a Hard, message-bearing, voidable blocker.

## 2. Item 2 — §6017 note (D2) + the coordination pin (empirical)

- **Pin check (run in the worktree, not taken from the report):**
  - `git diff main -- crates/btctax-cli/src/render.rs` → **exactly ONE hunk**:
    `@@ -1227,6 +1227,21 @@ pub fn render_schedule_se(` — inside `render_schedule_se`
    (fn starts 1118; next fn `render_gift_advisory` at 1314 post-change). ✓
  - `git diff main -- crates/btctax-tui/` → **empty (0 bytes)**. ✓
  - render.rs test module (2411+) untouched (the single hunk proves it); no CSV-writer-region
    (556–966) change; no new `pub fn` in render.rs. ✓
  - Whole-branch file list is exactly the six sanctioned files. ✓
- **Text = the spec's final wording exactly**, including "required on account of this income only",
  the ×92.35% base/§1402(a) parenthetical, §6017, §1402(b)(2), the §1402(j)(2) church-employee
  carve-out with "not modeled" [R0-M2], **plus the R0 round-2 N-6 optional clause** ("other
  self-employment activities, if any, combine on your actual Schedule SE") — verbatim as R0
  suggested it. Placement: immediately before the standalone note, `Some(r)` arm only.
- **Condition:** `r.base < rust_decimal::Decimal::from(400)` — strict `<` on the ×0.9235 base
  (matches "$400 or more"); the boundary KAT asserts note ABSENT at exactly `base == 400.00`.
- **The $397.10 half-even example [R0-M1]:** present in the KAT doc comment AND asserted
  mechanically — `round_cents(dec!(430.00) * dec!(0.9235)) == dec!(397.10)` pins the exact-tie
  $397.1050 → even cent under `MONEY_ROUNDING = MidpointNearestEven` (`conventions.rs:13,22–24`,
  verified). The pre-factor-$430/post-factor-$397.10 subtlety is exactly the case the KAT drives.
- **KATs in `tax_report.rs` ONLY** [R0-I1] — nothing in render.rs's test module. Feasible as R0
  verified (pub `render_schedule_se`, pub `SeTaxResult` fields); both KATs pass.
- No `compute_se_tax` change, no CSV change, no TUI change. Existing SE goldens untouched (their
  fixtures are all base ≥ $400; the 59-test cli lib module green unmodified).

## 3. Items 3–5

- **D3 (negative-flag binary tests):** byte-for-byte the house
  `tax_profile_negative_schedule_c_expenses_rejected` pattern — real binary
  (`CARGO_BIN_EXE_btctax`), full mandatory profile flags + poison flag, non-zero exit.
  **Empirically verified this review** by running the built binary directly:
  `--w2-ss-wages=-5` → `error: usage: --w2-ss-wages must not be negative` (exit 2);
  `--w2-medicare-wages=-5` → `error: usage: --w2-medicare-wages must not be negative` (exit 2).
  The guards (main.rs `w2_ss.is_sign_negative()` / `w2_medicare.is_sign_negative()`) ARE the
  tripping checks — not a clap parse error and not a missing-flag error (the medicare run leaves ss
  at default 0, so the ss guard passes first, and the negative value parses fine through
  `parse_usd_arg`). Chunk-A M-1 fully discharged.
- **D4 (mode-assertion KAT):** `repo_hygiene.rs` matches D4 exactly — repo root two `parent()` hops
  from `CARGO_MANIFEST_DIR`, `Command::current_dir(repo_root)` set explicitly [R0-N2], asserts
  exactly 2 index lines both starting `100755`, fail-closed with the mode-644 rationale in the doc
  comment (no skip arm). **CI-checkout behavior confirmed:** `git ls-files -s` reads the index,
  which `actions/checkout` populates from the committed tree — 100755 is asserted correctly
  regardless of runner filesystem; worktrees have their own index (verified here:
  `git ls-files -s` in the worktree returns both scripts at 100755, and the test passed in the
  709-green run executed in this worktree). Green at HEAD; counterfactually RED at the mode-644
  commit (mode is the first field asserted).
- **D5 (TY2024 full-schedule equality):** I independently compared **all 28 (lower, rate) ordinary
  pairs and all 4 LTCG pairs** in the test's inline arrays against
  `design/SPEC_ty2024_tables.md` §3.01 Tables 1–4 / §3.03 — **all match verbatim**, including every
  discriminator: HoH 24% **$100,500** (vs Single/MFS $100,525), HoH 35% **$243,700** (vs $243,725),
  MFS 37% **$365,600**, MFJ 37% **$731,200**, 32% start $191,950 in all four (the IRB-vs-PDF
  erratum), and MFS LTCG max_fifteen **$291,850** (NOT $291,875). `brackets.len()==7` per status;
  rates 10/12/22/24/32/35/37 in order. The delta-cancellation hole (TY2024 M1) is closed by direct
  equality. [R0-N3] direct not-stored assertions on the pub maps
  (`!tt.ordinary.contains_key(&Qss)` / `!tt.ltcg.contains_key(&Qss)`) + the
  `ordinary_for(Qss) == ordinary_for(Mfj)` alias check (Qss→Mfj `key()` verified at
  `tables.rs:87–92`). A1–A7 untouched (diff appends only).

## 4. The clippy fold + cross-cutting

- **`553cb8d`:** exactly 2 lines changed — `tt.ordinary.get(&Qss).is_none()` →
  `!tt.ordinary.contains_key(&Qss)` (and ltcg). Substance identical; assertion messages
  byte-unchanged; [R0-N3]'s "direct assertions on the pub fields" preserved.
- **No golden weakened, no existing test modified:** the whole branch is append-only for tests; the
  only existing-line edits are the resolve.rs collector rewrites, the ReclassifyIncome comment, one
  `use` line in tax_report.rs (adds `SeTaxResult`), and the render.rs hunk.
- **Determinism / house rules:** BTreeMap semantics preserved at all touched sites (per-decision
  exclusion never reorders anything); fixed timestamps; exact `dec!` Decimals; synthetic fixtures
  only (`coinbase`/`gemini`/`swan` refs, synthetic amounts); no PII (scan clean, §5).
- **Review-loop hygiene:** round-1 and round-2 Task-1 reviews persisted verbatim in
  `.superpowers/sdd/burndown3-task1-review-round-{1,2}.md` before folding; folds (`cf03bd2`,
  `553cb8d`) re-reviewed (round 2: 0 findings). Author ≠ reviewer held. D6/FOLLOWUPS correctly NOT
  on this branch (main-tree record files, spec Task 2).

## 5. Validation suite (all re-run by this reviewer in the worktree @ 553cb8d)

| Gate | Result |
|---|---|
| `cargo test --workspace` | **709 passed / 0 failed / 0 ignored** ✓ (matches the report) |
| `cargo +stable clippy --workspace --all-targets --locked -- -D warnings` | exit 0, clean ✓ |
| `cargo +nightly clippy --workspace --all-targets --locked -- -D warnings` | exit 0, clean ✓ |
| `cargo fmt --all -- --check` | exit 0, clean ✓ |
| `bash scripts/pii-scan-generic.sh` | `pii-scan: clean (HEAD).` ✓ |
| Binary guard checks (manual) | both W-2 guards fire with their exact Usage messages ✓ |

## 6. Findings

### Critical — none.
### Important — none.
### Minor — none.

### Nit

**N-1 — The six blocker KATs don't pin the blocker `detail` text or `event` attribution.** The
redirect messages ("use reclassify-income", "classify-inbound-income (its own `fmv` field)") and
the `event: Some(decision_id)` attribution are verified in source but not test-locked; a regression
rewording them to something unhelpful would pass the suite. This matches the house bar — the
[R0-N1]-cited reference (`reclassify_income.rs:632–718`) asserts the same way (kind-filtered count
+ state fields, no detail assertion) — so it is consistency, not a defect. Optional hardening for a
future slug: one `detail.contains(...)` per redirect message.

**N-2 — Happy-path KAT uses `ReclassifyOutflow→GiftOut` where the spec's plan bullet suggested
`→Donate`.** Substantively equivalent (both valid TransferOut classes; the spec bullet itself
allows "or assert via existing suite", and a valid Donate reclassify is exercised by the existing
suite, e.g. `kat_tax.rs` "OD" fixture). No coverage lost.

**N-3 — Blocker KATs assert disposals/removals/income but not `holdings_by_wallet`.** The spec's
[R0-N1] list names four fields. The asserted fields are the ones each decision type can affect, and
exclusion-from-map makes `build_op` behave identically to the no-decision fixture, so holdings
cannot diverge without one of the asserted fields diverging first. House-bar-consistent; recording
for completeness only.

**N-4 — Per-commit gate hygiene: `b04080f` was not fmt-clean until `ec4a260`.** Disclosed in the
report; content of the fixup verified formatting-only (behavioral identity confirmed by the round-1
reviewer and the suite). Whole-branch state is clean; noting so the record is honest about the
intermediate commit.

None of the four is blocking; none requires a fold before merge. N-1/N-3 may be folded into a
FOLLOWUPS line at the Task-2 (D6) record step if desired.

---

## 7. Disposition

**READY TO MERGE — 0 Critical / 0 Important.** All five items match the R0-GREEN spec exactly, all
[R0-…] tags are honored in the shipped code, the coordination pin is empirically intact (one hunk
@1227 inside `render_schedule_se`; btctax-tui untouched), the D1 safety property (no silent
bad-target decision of any of the four types) is implemented without a hole and KAT-locked
including the ManualFmv latest-wins re-target semantic, and the full validation suite is
independently re-verified green.

**Landing order:** merge the export-from-TUI lane FIRST, then rebase this branch. Expected rebase
surface: the single render.rs hunk shifts by whatever line count the export lane added in 556–966;
no textual conflict is possible unless the export lane violated its own pin — if `git rebase`
reports a render.rs conflict, treat it as the process violation the spec names, not a chore.
Spec Task 2 (D6 record items + FOLLOWUPS closes) remains owed in the MAIN tree after merge.
