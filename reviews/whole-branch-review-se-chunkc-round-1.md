# Whole-branch review — SE-completion Chunk C: `ReclassifyIncome` decision (round 1)

**Artifacts:** diff `1b6dfe3..047bf69` (2 commits: spec `5eb878a`, impl `047bf69`);
`design/SPEC_se_chunkC_reclassify_income.md` (R0 GREEN); `reviews/R0-spec-se-chunkC-round-1.md`;
Task-1 report `.superpowers/sdd/se-chunkc-report.md`.
**Reviewer:** final whole-diff (Phase E) reviewer; author ≠ reviewer.
**Date:** 2026-07-01.
**Gate state accepted as given:** 667 tests pass, clippy `-D warnings` clean, fmt clean, PII clean,
`fold.rs` untouched (re-confirmed via the diff-stat: no `fold.rs` entry). Not re-run.
**Verification method:** every claim below checked against the CURRENT working tree at `047bf69`
(not the diff text alone); CLI parse behavior verified empirically against the built
`target/debug/btctax` binary; NIIT/SE values re-derived by hand from
`crates/btctax-core/src/tax/{compute.rs,se.rs,tables.rs}` and the actual fixture in
`crates/btctax-core/tests/reclassify_income.rs`.

## Verdict: **NOT ready to merge — 0 Critical / 2 Important / 2 Minor / 2 Nit.**

The engine work is correct end-to-end: the R0-I1 bad-target mechanism is implemented exactly as
specified and genuinely KAT'd, the build_op override is right, FIRST-WINS + void semantics hold,
back-compat is clean (new-variant-only, no reshaping), and all NIIT/SE interactions re-derive
correctly against the shipped KATs. Both Importants are small non-engine folds: (I-1) the CLI
`--business` argument shape diverges from the R0-GREEN spec and the newly shipped river.rs
comments document an invocation the binary rejects; (I-2) the spec-mandated FOLLOWUPS.md entries
have not landed on the branch. Fold both, re-review, and this is green.

---

## 1. Bad-target validation (R0-I1 mechanism) — ✅ CORRECT, genuinely KAT'd

**Code** (`crates/btctax-core/src/project/resolve.rs:510-575`, verified in source, not just diff):

- Collection-time (pass 1e) check against the **effective** payload:
  `by_id.get(target).map(|raw| applied.get(target).unwrap_or(&raw.payload))` (resolve.rs:525-527).
  This is exactly the spec's `applied.get(&target).unwrap_or(raw)` formula and the same idiom
  pass 2 uses at resolve.rs:590. `applied` is declared at resolve.rs:346 and fully populated in
  1b (supersede inserts, :391) and 1c (ClassifyRaw inserts, :418) **before** the 1e loop —
  ordering is sound. A ClassifyRaw'd/superseded row whose effective payload is Income stays
  reclassifiable; a ClassifyInbound-Income TransferIn is correctly rejected (its effective
  payload remains TransferIn).
- **Missing target** (`by_id` miss) → `None` arm → Hard `BlockerKind::DecisionConflict`
  (Hard per `state.rs` `severity()` — `DecisionConflict` is in the Hard arm) + decision NOT
  inserted (resolve.rs:529-542). Note: a `by_id` miss with a stray `applied` entry cannot
  produce a projected row anyway (pass 2 iterates ledger events), so by_id-first is the correct
  order of checks.
- **Non-Income effective target** → `Some(_)` arm → Hard `DecisionConflict` + excluded
  (resolve.rs:559-573).
- Both blocker details carry the N1 hint ("for TransferIn rows use classify-inbound-income")
  and the N2 correction flow ("void the prior decision first").
- The deliberate-divergence-from-ReclassifyOutflow comment and the TransferLink/LotSelection
  precedent citations are present (resolve.rs:511-523), per spec D2.

**KATs — both genuine** (`crates/btctax-core/tests/reclassify_income.rs`):

- `bad_target_missing_event_yields_hard_blocker`: bogus import id; asserts exactly 1
  `DecisionConflict` **and** the unchanged projection (`kind == Reward`, `business == false`).
  Not silently inert; not a panic.
- `bad_target_non_income_event_yields_hard_blocker`: targets an Acquire; asserts exactly 1
  `DecisionConflict` **and** `income_recognized.is_empty()` (projection unchanged — no income
  fabricated). Not silently inert; not a panic.

Both assert the blocker AND the projected values. The R0-I1 hazard (silent no-op on an
SE-relevant correction) is closed in code and pinned by tests. **Pass.**

## 2. Override correctness — ✅

- **build_op-only:** the override lands solely in the `EventPayload::Income(x)` branch of
  `build_op` — `(o.business, o.kind.unwrap_or(x.kind))` when `income_reclassify.get(id)` hits,
  else `(x.business, x.kind)`. Exactly the spec formula. `fold.rs` untouched (diff-stat: no
  entry). Composes orthogonally with `manual_fmv` (FMV vs kind/business — disjoint fields).
- **FIRST-WINS:** `decisions` is sorted ascending by seq (resolve.rs:349-356); the 1e branch
  inserts only when `!contains_key` and pushes the `DecisionConflict` blocker for the second
  decision (resolve.rs:546-557). KAT `duplicate_reclassify_income_conflict_first_wins` asserts
  exactly 1 conflict AND that decision #1's values (Mining/true) govern — disambiguated from
  LotSelection's neither-applies semantic per R0-M2.
- **Void:** generic pass-1a `Some(_)` catch-all makes the new variant revocable with zero code;
  the revocability doc-comment now lists ReclassifyIncome (resolve.rs pass-1a header). KAT
  `void_reverts_to_original` asserts no blockers + original Reward/false restored.
- Keying is consistent: the map is keyed by target event id; build_op consults it by the event's
  own id, so overrides apply to raw AND ClassifyRaw'd/superseded-to-Income rows alike.

**Pass.**

## 3. NIIT/SE interactions — ✅ code and KATs correct; the REPORT's hand-derivation is wrong (see M-1)

**Independent re-derivation from the actual fixture** (`niit_profile()`: Single,
`magi_excluding_crypto` = $205,000, `ordinary_taxable_income` = 0, qd = 0, no disposals, no
carryforwards, no W-2; income FMV = $10,000; `niit_threshold(Single)` = $200,000 per
tables.rs:285; `NIIT_RATE` = 0.038 per tables.rs:133):

- **Reward (or Mining) arm:** `interest_nii` = 0 (kind filter, compute.rs:306-311) →
  `nii_with` = 0, `nii_without` = 0 → `niit_with` = `niit_without` = 0 → **r.niit = $0.00**.
- **Interest arm:** `interest_nii` = $10,000 → `nii_with` = $10,000; `nii_without` = $0.
  `crypto_agi` = `crypto_ord` = $10,000 (compute.rs:357-359) →
  **`magi_with` = $205,000 + $10,000 = $215,000** (MAGI includes the income itself —
  compute.rs:361); `magi_without` = $205,000.
  `niit_with` = round_cents(0.038 × min($10,000, $215,000 − $200,000 = $15,000))
  = 0.038 × $10,000 = **$380.00** (the MAGI cap does NOT bind).
  `niit_without` = 0.038 × max(0, min($0, $5,000)) = $0.
  **r.niit = $380.00.**
- **Deltas:** Reward→Interest: **+$380.00** (0 → 380.00). Interest→Mining: **−$380.00**
  (380.00 → 0). Non-vacuous in both directions (MAGI > threshold, R0-I2 satisfied); the KAT
  asserts these exact values and the suite is green — the engine agrees with this derivation.
- **The report claims +$190.00** (min($10,000, $205,000−$200,000) = $5,000 → 3.8% × $5,000).
  That derivation uses `magi_without` where the engine (correctly — Form 8960 MAGI includes the
  NII) uses `magi_with` = $215k. **The report is wrong; the shipped code and KAT are right.**
  Filed as M-1 (report fix only — no code change).
- **Engine-B invariance (business-only flip):** KAT compares `ordinary_from_crypto`, `niit`,
  `ltcg_tax`, `total_federal_tax_attributable` before/after — all identical, as they must be:
  `crypto_ord` is year-filtered only (compute.rs:296-301), `interest_nii` kind-filtered only,
  and the fixture's Reward kind keeps `interest_nii` = 0 on both sides. Correct and non-trivial
  (the fixture MAGI is above threshold, so a kind-sensitivity bug WOULD move niit).
- **Headline SE flip (P2-D math), re-derived from se.rs** (92.35% factor; ss 12.4% capped at
  `ss_wage_base − w2_ss_wages` = $176,100; medicare 2.9%; addl 0.9% over $200k(Single);
  `round_cents` = HALF_EVEN, end-of-component):
  net_se = $10,000.00; base = $9,235.00; ss = 0.124 × 9,235 = $1,145.14 (exact);
  medicare = 0.029 × 9,235 = $267.815 → HALF_EVEN → **$267.82**; addl = $0 (9,235 < 200,000);
  total = **$1,412.96**; deductible_half = 1,412.96 / 2 = **$706.48**. All match the KAT
  assertions exactly. Before the decision: se_net_income = 0 → `compute_se_tax` None — pinned.
- `--business true --kind interest` would be SE-none + NII-in (se.rs excludes Interest
  regardless of business) — the correct §1402(a)(2)/§1411(c)(1)(A)(i) split; unchanged by this
  diff and already pinned by the existing se.rs M2 test.

**Engine pass.**

## 4. Back-compat — ✅ (with one doc-wording Minor)

- **New variant only:** the event.rs diff strictly ADDS (`ReclassifyIncome` struct at
  event.rs:205, variant at event.rs:275). No existing variant reshaped — no GiftOut-style trap.
  Externally-tagged serde: adding a variant cannot change the decode of existing variants.
- **Old-vault KAT:** `old_vault_without_variant_loads_unchanged` pins a variant-free stream
  projecting unchanged.
- **Serde round-trips both kind arms:** pinned twice — in the
  `every_variant_serde_round_trips` inventory (event.rs:476-489, both `Some(Mining)` and `None`
  arms — R0-M3 satisfied) and in
  `reclassify_income_serde_round_trips_both_kind_arms`. `#[serde(default)]` on
  `kind: Option<IncomeKind>` is correct for field omission.
- **fingerprint() → None:** pinned twice (`event.rs::reclassify_income_decision_has_no_fingerprint`,
  both arms; `tests/reclassify_income.rs::reclassify_income_fingerprint_is_none`). R0-M1
  satisfied. `append_decision` is payload-generic; zero schema surface.
- **Old-binary limitation documented** in both the struct and variant doc-comments — but the
  struct doc's serde characterization is wrong in one phrase (see M-2): it says "serde
  unknown-variant handling is silent-skip for future variants in old vaults"; the actual
  behavior (R0-verified) is a LOUD unknown-variant error → `CoreError::Persistence` hard load
  failure. The conclusion (old vaults are safe because they contain no such rows) is right; the
  stated mechanism is not.

**Pass, modulo the M-2 wording.**

## 5. CLI + docs — ❌ one Important (I-1); rest pass

- **Wiring:** `Reconcile::ReclassifyIncome` variant (main.rs:291-301) + dispatch arm
  (main.rs:880-889) → `cmd::reconcile::reclassify_income` → `parse_event_id` on the ref,
  `parse_income_kind` on `--kind` (via `.transpose()`), `append_and_save`. Correct pattern;
  mutating decision, not a side-table. ✅
- **N1/N2 folded:** blocker details hint classify-inbound-income + void-first; the subcommand
  help and `reclassify_income` doc state "DecisionConflict is Hard — void the prior decision
  first, then re-issue". ✅
- **river.rs comments updated** (both "income" and "interest" arms, river.rs:149-154/167-172) —
  but they document `--business true`, which the shipped binary REJECTS (see I-1). ⚠
- **resolve.rs revocability doc** updated to include ReclassifyIncome (and the previously
  missing MethodElection/LotSelection). ✅
- **Determinism:** `BTreeMap<EventId, ReclassifyIncome>`, decisions iterated in ascending seq;
  no float anywhere in the diff (Decimal/`dec!` only, incl. the whole test file). ✅

---

## Findings

### I-1 — IMPORTANT: `--business` shipped as a bare presence flag; the spec-D3 syntax `--business <true|false>` — repeated in the spec's headline KAT, the Task-1 report, and BOTH newly-shipped river.rs comments — is rejected by the binary

`main.rs:295-296` is `#[arg(long)] business: bool` under clap 4.5 derive → `ArgAction::SetTrue`.
Empirically verified against `target/debug/btctax`:

- `reconcile reclassify-income REF --business true` → `error: unexpected argument 'true' found`
- `reconcile reclassify-income REF --business=true` (and `=false`) → `error: unexpected value 'true' for '--business' found; no more were expected`
- `--business` (bare) → true; **omitted** → silently `business = false`.

Consequences:

1. The invocation shipped in the river.rs comments (`--business true [--kind mining|…]`,
   river.rs:151 and :169) — the exact breadcrumb a River business-miner is pointed at — fails
   at parse time. Loud, not silent, but the shipped documentation is wrong on day one.
2. Divergence from the R0-GREEN spec (D3: `--business <true|false>`; headline KAT:
   `reclassify-income --business true --kind mining`) with no recorded deviation. The Task-1
   report also claims the `<true|false>` form shipped — it did not.
3. The flag form makes `business=false` expressible only by omission: `reclassify-income REF
   --kind mining` parses cleanly and appends a decision with `business: false` — an implicit
   value on the SE-relevant field the spec required to be explicit. (Today all adapter income
   is `business: false` — verified: no adapter emits `business: true` — so the trap is latent,
   not live; it still contradicts the contract.)

**Fix (pick one; either is a one-to-two-line fold + doc alignment):**
(a) match the spec: `#[arg(long, action = clap::ArgAction::Set, value_name = "true|false")]
business: bool` — makes `--business true|false` work exactly as documented everywhere
(note: intentionally diverges from `ClassifyInboundIncome`'s flag convention; say so); or
(b) keep the flag form and amend river.rs comments, spec D3, and the report to the flag syntax,
recording the deviation. Option (a) is recommended — three written artifacts and the help
text's own wording ("true → SE-tax eligible") all promise a value.

### I-2 — IMPORTANT: the spec-mandated FOLLOWUPS.md entries have not landed on the branch

Spec D2 (binding, R0-round-2-verified): "add a FOLLOWUPS item to backfill the same validation
onto ReclassifyOutflow (out of scope here)"; spec Task 2: "FOLLOWUPS: Chunk B (expenses
advisory) next". As of `047bf69`, `FOLLOWUPS.md` is untouched by the branch (diff-stat) and
contains no backfill item (grep: no "backfill", no ReclassifyOutflow-bad-target-validation
entry); only the resolve.rs code comment (resolve.rs:523) records the intent. The known
silently-inert hazard on ReclassifyOutflow/ClassifyInbound/ManualFmv — the exact hazard class
R0-I1 escalated — would go untracked the moment this merges. Task-2 placement means this is the
natural moment to land it, but the merge gate cannot pass while it is absent from the branch.

**Fix:** add the FOLLOWUPS.md entry (ReclassifyOutflow/ClassifyInbound/ManualFmv bad-target
validation backfill, citing resolve.rs:487-497 blind insert + the Chunk-C precedent at
resolve.rs:510-575) and refresh the cluster status line (Chunk C shipped → Chunk B remaining).

### M-1 — MINOR: the Task-1 report's NIIT hand-derivation is wrong ($190.00); the shipped code and KAT ($380.00) are right

`.superpowers/sdd/se-chunkc-report.md` ("Hand-derived NIIT delta values") computes
3.8% × min($10,000, $205,000 − $200,000) = $190.00 — using MAGI **excluding** the income.
The engine (compute.rs:357-361) adds `crypto_agi` (+$10,000) to MAGI, so over-threshold =
$15,000, the cap does not bind, and the delta is 3.8% × $10,000 = **$380.00** — which the KAT
asserts and the green suite confirms. No code change; correct the report so a future reader
doesn't "fix" the correct KAT toward the wrong number.

### M-2 — MINOR: event.rs back-compat doc-comment misstates the serde failure mode

`ReclassifyIncome` struct doc (event.rs:~199-203): "serde unknown-variant handling is
silent-skip for future variants in old vaults — no data at all, nothing to skip". Serde's
externally-tagged unknown-variant handling is a hard ERROR (→ `CoreError::Persistence`, loud
load failure) — R0 §1 pinned exactly this ("loud, not silent misreading"). The comment's
conclusion is right for the wrong reason; in the back-compat area this gate ranks highest,
the mechanism text must not claim "silent-skip". Reword: old vaults are safe because they
contain no `ReclassifyIncome` rows; an old binary opening a NEW vault that contains one fails
loudly with an unknown-variant error.

### N-1 — NIT: engine-B invariance KAT compares four figures, not the whole `TaxResult`

`ordinary_from_crypto`, `niit`, `ltcg_tax`, `total_federal_tax_attributable` — sufficient for
the spec's "ordinary/total" wording and the material invariant. If `TaxResult` ever gains
`PartialEq`, a whole-struct compare would future-proof it.

### N-2 — NIT: bad-target KATs pin blocker kind/count/projection but not the blocker's `event` id or detail text

Asserting `b.event == Some(decision_id)` and a detail substring (e.g. "classify-inbound-income")
would additionally pin the N1 hint against regression. Adequate as shipped.

---

## Cross-cutting confirmations (no findings)

- **fold.rs untouched** — confirmed via diff-stat; the override is build_op-only as specified.
- **Determinism** — BTreeMap collections; seq-sorted decision iteration; blocker order stable.
- **No float** — Decimal/`dec!` throughout the diff, tests included.
- **Voided bad-target decisions** produce no blocker (skipped in 1e) — void clears the Hard
  block, consistent with the N2 correction flow.
- **Duplicate + bad-target interleaving** — both paths exclude decision #2 with a Hard
  `DecisionConflict`; observably immaterial, as R0 round 2 noted.
- **Synthetic-only fixtures; PII clean** — the test file fabricates all ids/values.

## Required to reach green

| # | Severity | Fix |
|---|---|---|
| I-1 | Important | Make the CLI match the contract: `--business <true|false>` via `ArgAction::Set` (recommended), or amend spec D3 + river.rs comments + report to the flag form with a recorded deviation |
| I-2 | Important | Land the FOLLOWUPS.md entries: ReclassifyOutflow (+ ClassifyInbound/ManualFmv) bad-target-validation backfill; Chunk C shipped / Chunk B next |
| M-1 | Minor | Correct the report's NIIT derivation to $380.00 (magi_with includes the income) |
| M-2 | Minor | Reword the event.rs "silent-skip" back-compat sentence to the actual loud unknown-variant failure |
| N-1 | Nit | (Optional) whole-struct engine-B compare if TaxResult gains PartialEq |
| N-2 | Nit | (Optional) pin blocker `event` id + detail substring in the bad-target KATs |

## Gate decision

**NOT ready to merge: 0 Critical / 2 Important.** The engine, the R0-I1 mechanism, the tax
math, and back-compat are all verified correct — both Importants are perimeter folds (one clap
attribute + doc alignment; a FOLLOWUPS.md entry). Fold, re-run the suite, re-review.
