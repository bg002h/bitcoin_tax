# PLAN review r1 — `IMPLEMENTATION_PLAN_full_return.md` (Fable, independent, 2026-07-12)

**Target:** `design/IMPLEMENTATION_PLAN_full_return.md` (opus-authored DRAFT 2026-07-12).
**Implements:** `design/SPEC_full_return.md` (GREEN r4). **Gate:** STANDARD_WORKFLOW §2 — advances only
at 0 Critical / 0 Important.

**Verdict: NOT GREEN — 2 Critical / 5 Important / 7 Minor.**

Everything below was verified against current source (`types.rs`, `compute.rs:364/369`, `se.rs`,
`schedule_d.rs:5-6`, `schedule_se.rs`, `form1040.rs`, `verify.rs`, `tax_profile.rs`,
`tax_tables.rs`) and the recon corpus (`deep/01`–`05`, `fable/05`, `05-prior-art-verification.md`).

---

## Critical

### C1 — P0's bin/bracket-edge assertion (KAT-3) is provably unsatisfiable on the current bundled tables

**Location:** P0 task 4 + P0 acceptance ("bin assertion passes for 2017/2024/2025/2026").
**Problem:** the assertion as stated — "no bracket edge <$100k in a $50 bin" — is **false for two of
the four bundled years**. Checked every `br(dec!(...))` edge in
`crates/btctax-adapters/src/tax_tables.rs` under $100k:

| Edge | Year/status | mod 50 | mod 25 |
|---|---|---|---|
| 9,325 | TY2017 Single/MFS | **25** | 0 |
| 11,925 | TY2025 Single/MFS | **25** | 0 |
| 48,475 | TY2025 Single/MFS | **25** | 0 |

All three sit **strictly inside** a $50 bin (at its exact midpoint). deep/01:59's "no bracket
boundary ever falls in a bin interior" is a **TY2024-only** fact ("every TY2024 bracket edge … is a
multiple of $50"); the spec generalized it per-year (§8/KAT-3) and the plan operationalized it over
"every bundled `TaxTable` year" with an acceptance naming 2017/2025 — which cannot pass. Written
test-first, KAT-3 goes red on frozen bundled data and forces an unreviewed mid-phase design decision
(exactly what plan review exists to prevent).
**Why the fix is determinate:** an edge at the exact midpoint is harmless — the IRS constructs every
Tax-Table cell as `round(TCW(bin midpoint))` and TCW is continuous at an edge, so a midpoint-edge
cell is still reproduced exactly. The dangerous case is an interior edge **off** the midpoint, which
in the $50-bin region means "not a multiple of $25". All bundled edges are ≡ 0 (mod 25).
**Fix:** restate the assertion as **"every bracket edge < $100k is a multiple of $25"** (i.e., a $50-bin
boundary or its exact midpoint; analogously mod 12.5 in the $25-bin region, vacuous today), plus one
pinned printed-cell KAT for a midpoint-edge bin (e.g. TY2025 Single [11,900, 11,950)) to prove the
midpoint convention holds when an edge lands on it — or scope the strict no-interior-edge form to
TY2024/2026 only. Record the spec §8/§10-KAT-3 wording erratum in `FOLLOWUPS.md` (spec-text fix,
same pattern as the spec's §10 erratum note).

### C2 — Spec §6 (delta-vs-absolute dual reporting) has no home in any phase

**Location:** whole plan; closest misses are P1 task 3 (resolver wiring), P4 (compute only), P5
(LIMITATIONS + CTC/ODC/EIC advisories).
**Problem:** spec §6 is normative user-facing behavior: **"Both numbers reported, labeled as
different questions"** (deep/02's $2,242 absolute vs $1,596 delta), and **"the report documents the
delta deduction as approximate and never reconciles the two to the dollar."** No plan task builds
this: P4 stops at computed L24/L33–37, P6 fills PDFs, and no task puts the absolute return figures
(or the labeled delta/absolute pair, or the approximate-deduction documentation) on the `report`/TUI
surface at all. KAT-5/5b (P4) pin the *invariant*, not the *reporting*. A spec section with zero
plan coverage is a completeness failure at this gate.
**Fix:** add an explicit task (natural home: end of P4, or a P5 sibling) — render the absolute
liability lines + the crypto delta side-by-side with the §6 labels, document the delta-deduction
approximation in the output, and KAT the presence/labeling on a fixture where
`absolute_with − absolute_without ≠ delta` (AGI-sensitive deduction).

---

## Important

### I1 — KAT-ownership table is internally inconsistent, and KAT-18's implementing task doesn't exist

**Location:** "KAT ownership" block ("KAT 15,18 → P4/P1"); P2 task 2; P6 task 3.
**Problem:** (a) the ownership line assigns **KAT-15 to P4/P1**, but the plan itself implements
KAT-15 in **P2 task 2** ("KAT-15: L8v-vs-L3 partition + cross-foot") — the normative mapping
contradicts the phase text. (b) **KAT-18** (Schedule B filing trigger, spec §7.1/R3-I2: files on
interest > $1,500 *or* ordinary dividends > $1,500 *or* `foreign_accounts == Some(true)`; Part III
7a/8 tri-state fail-loud when it files) has **no implementing task anywhere**: P2 task 1 assembles
2b/3b without the trigger, P4 never mentions it, and P6 task 3 covers only the Sch B *filler* +
overflow. "P4/P1" matches neither.
**Fix:** correct the table (KAT-15 → P2). Add the trigger + Part-III-tri-state logic as an explicit
task (compute-side decision in P2 or P4; fill-side in P6) and assign KAT-18 to it.

### I2 — P1 task 3 depends on a P2 output (dependency inversion at the resolver)

**Location:** P1 task 3 + P1 acceptance ("`resolve_profile` precedence + provenance tested").
**Problem:** the first arm of the §4.12 precedence order — `ReturnInputs` → `TaxProfile` — requires
`derive_tax_profile`, which is **P2's** deliverable. In P1 the resolver cannot return a profile from
`ReturnInputs`, so the stated P1 acceptance (precedence tested) is unsatisfiable; the
two-sources-of-truth KAT needs the same arm.
**Fix:** either (a) P1 ships the resolver skeleton with the `ReturnInputs` arm explicitly stubbed
(`NotComputable("derivation pending")` — safe, no vault can have `ReturnInputs` yet) and moves the
first-arm KAT + full precedence acceptance to P2, or (b) move the consumer wiring wholesale to P2.
The D-4 `tax-profile set` warn + `--force` piece is P1-landable either way.

### I3 — The FROZEN CI guard is under-specified and, as described, weaker than the invariant it enforces

**Location:** Global invariants bullet 1 ("A CI guard test asserts these files' public surface is
unchanged").
**Problem:** every phase's acceptance terminates in "FROZEN guard green", so the guard *is* the
enforcement mechanism for the plan's #1 invariant — but (a) "public surface unchanged" does **not**
detect a behavior-changing edit inside `compute.rs` that preserves the API, while the invariant is
"never **edit**"; (b) the guarded file set is vague ("the crypto delta path", "the ~80 existing
constructors", "existing crypto tests" — which files, exactly?). An implementer can satisfy the
letter of the guard while violating the freeze.
**Fix:** define the guard concretely in P0: an enumerated frozen-path list (at minimum
`crates/btctax-core/src/tax/{types,compute}.rs`; enumerate what "delta path"/"constructors"/"crypto
tests" resolve to) with a CI test pinning each file's content (SHA-256 of the file, or
`git diff --exit-code` against a recorded baseline), plus a documented exception process (baseline
update = its own reviewed change).

### I4 — P2 task 3's "magi=AGI" is wrong as written at the plan's own stated biggest-risk seam

**Location:** P2 task 3: "AGI L11 (= with-crypto AGI); `derive_tax_profile` returns the frozen-shape
`TaxProfile` (ordinary_taxable_income stripped of pref slice; **magi=AGI**; SE channels populated)".
**Problem:** `TaxProfile`'s fields are *excluding-crypto* by contract
(`magi_excluding_crypto`, `ordinary_taxable_income` "EXCLUDING all app-computed crypto items" —
`types.rs:34-38`), because the frozen engine **adds** the crypto AGI delta itself
(`compute.rs:364-368`). The bullet defines L11 as *with-crypto* AGI in its first clause, so
"magi=AGI" reads as feeding with-crypto AGI into `magi_excluding_crypto` — which double-counts
crypto in the delta path. The plan names this seam its "biggest risk" and then specifies it
ambiguously-to-wrongly; the planned KATs ("derived profile drives compute.rs unchanged",
"derived == hand-entered") don't force the exclusion semantics, since a hand-entered comparison
profile built under the same misreading passes.
**Fix:** rewrite the bullet: `derive_tax_profile` populates `magi_excluding_crypto` /
`ordinary_taxable_income` from the **non-crypto** line items only (ledger `crypto_ord`, crypto
gains, and the Schedule-C-driven ½-SE stay out — the ½-SE/MAGI wedge is exactly the pinned KAT-5b
divergence); require the P2 KAT to use a **crypto-income fixture** and assert the derived-profile
delta equals the delta from an independently hand-built exclusion profile.

### I5 — P0 task 1's KAT-9 discriminant is mathematically false; the rounding-mode proof is mis-sequenced

**Location:** P0 task 1: "KAT-9 fixture + `round_dollar` (271.50+499.50 → 772 printed; **half-even
would give 771**) → implement mode."
**Problem:** half-even does **not** give 771 on this fixture: 271.50 →ties-to-even→ **272** and
499.50 → **500**, so half-even also prints 272 + 500 = 772. 771 arises only from *sum-then-round*
(`round(771.00)`) — i.e., KAT-9 discriminates printed-line-rounding + cross-foot (exactly how the
spec frames it), **not** the half-up rounding mode the task claims to drive. As sequenced, task 1
implements `round_dollar`'s mode against a test that passes under the wrong mode; the mode is only
caught incidentally three tasks later (task 4's deep/01 example (c): 7,818.50 → 7,819, where
half-even gives 7,818). Deep/01:255 explicitly demands the two half-even-failing cells as
fault-injection KATs; the plan never names them.
**Fix:** make task 1's red test the deep/01 discriminating cells (MFJ [11,600, 11,650) = **1,163**;
Single [3,000, 3,050) = **303** — half-even yields 1,162/302), with the fault-inject-`round_cents`
check. Keep KAT-9 as the cross-foot/printed-line proof it actually is, and add the hook re-asserting
its printed-line half on the real 8959 lines at P4/P6 (P0 has no form lines to "print"), i.e., mark
KAT-9 ownership P0 (arithmetic) + P4/P6 (printed cross-foot).

---

## Minor

1. **P1 refuse-guard acceptance vs TI≤0.** P1 promises "one KAT per row" and acceptance "every
   refuse row has a red→green KAT", but the §4.10 `taxable income ≤ 0` row is computable only in P3
   (the task's own "TI≤0-deferred" concedes this), and the `fn screen(&ReturnInputs)` signature
   cannot evaluate the single-employer excess-SS row (needs the year's MAX = 6.2%×`ss_wage_base` →
   `&TaxTable` parameter). Reword the acceptance ("every *input-screenable* row in P1; TI≤0 in P3")
   and widen the signature.
2. **QBI carryforward write-back + R3-M6 KAT.** Spec §4.5 "Carryforward-out persists per §4" — P4
   never names the QBI REIT/PTP write-back (only charitable, P3). The R3-M6 precedence behavior
   (computed overwrites computed; **refuses** user-entered without `--force`) has no named KAT in
   any phase. Add both to P4/P1.
3. **P0 task 5 data gaps.** Omits the SALT-cap constant (spec §8) from the additions; silent on the
   other bundled-year constructors when `TaxTable` gains fields (`Option` vs required — `ty2017/25/26`
   must still compile); note the adapters-crate convention that *statutory* constants
   (SALT cap, FTC ceiling) live in core `tables.rs`, never in `TaxTable` (`tax_tables.rs:9-11`) —
   settle placement explicitly.
4. **1040 L36 (refund applied to next year).** Spec §5 stage 9 carries "−L36 applied-to-2025 (G16)"
   but §4.8 `Payments` has no such input and P4 task 6 ("settle L34–L37") is silent. Pin L36
   blank/0 in v1 or add the input; record the spec §4.8 gap in FOLLOWUPS.
5. **`fr-schb-user-forced` not folded.** The FOLLOWUP (add `force_schedule_b` or drop the
   "user-forced" clause) is "encountered" at P1's input-surface task; the plan doesn't carry it.
6. **P6 task 2 "leaf map from deep/03" over-claims.** deep/03 contains only the six existing roots
   (1040, Sch 1/2/3/A/B); Schedule C / 8959 / 8960 / 8995 maps are **new extractions** (spec §7.3:
   "a scheduled deep/03-**style** extraction"). Fix wording so P6's effort is scoped honestly (the
   preamble's "extract map" step covers it; the task line contradicts it).
7. **Small unassigned spec details** (each needs one line in a phase): P3's QBI-0-stub (spec R3-M7)
   not restated; the §3.4 conservative-omission "line pinned to 0 + advisory" KAT not explicitly
   owned (P4 L19=0 / P5 advisory split it); §7.1's form-set-closure invariant ("no line without its
   backing form on a non-DRAFT return") has no KAT; SSN `--stdin` + masking (spec §4.2
   security-review item) absent from P1 task 2; Sch A 5a election-checkbox fill (R3-M9, deep/03
   `c1_1`) + the fail-loud sales-amount-with-election-off mismatch not named in P3/P6.

---

## Checked and found CLEAN

- **Phase DAG.** P0 → P1 → P2 → P3 → P4 → (P5 ∥) → P6 → P7 is a sound order (apart from I2): fillers
  after compute, goldens last; P2's std-basic-only is self-consistent (AGI needs no deduction; the
  deduction appears only inside the derived delta profile, upgraded in P3); P3's L16-vs-`method.rs`
  acceptance is a legitimate wiring (not circularity) test; P4-internal ordering (QBI → FTC → AMT →
  SE → 8959/8960 → settle) respects data flow.
- **Frozen-engine preservability** (modulo I3's guard definition): `derive_tax_profile` constructs
  `TaxProfile` via public fields — no `types.rs`/`compute.rs` edit needed; the absolute 8959 Part II
  reuses `se.rs::compute_se_tax`'s `addl` (verified ≡ 8959 L11–L13 incl. the inner clamp,
  `se.rs:140-158`, deep/02 §3.3 cent-exact); the §6017 $400 floor is implementable in the new
  wrapper without touching `se.rs` (the existing `schedule_se.rs` filler already skips below $400,
  so the two layers agree); 8960's floors match `compute.rs:369-380`'s closure shape without reuse
  of frozen internals.
- **KAT-ownership completeness** (modulo I1/I5's corrections): KATs 1–3, 4 (labeled TY2025
  follow-on, matching spec), 5/5b, 6–14, 16, 17 and the refuse rows all land in coherent phases;
  KAT-11's declared P3/P4 split matches its compound content; refuse rows P1-owned with end-to-end
  re-assertion in P4 is layered, not double-owned.
- **KAT-5b framing.** The reduce-to-delta inequality is correctly carried as a *pinned expectation*
  in both the P4 task and "Sequencing & risk" ("must be a pinned expectation, not a bug to fix"),
  with both forbidden mis-fixes named — faithful to spec §5-tail/R3-I4.
- **Recon/feasibility anchors all real:** deep/02 Ex.1 (MFJ derivation, §1 "verified to the cent")
  and Ex.2 ($60k mining, both-sides 8959 worked to the cent, §3.2) exist as claimed; deep/04 §3's
  $70k-LT-crypto → $60k allowed + $10k 30%-class carryover and the :190 two-term 30%-cap formula
  verified; ATS Scenario 2 ("Sean & Joan Jackson", MFJ, TY2024) fetched/parsed per recon-05:214-220
  with the M6 out-of-scope-forms caveat the plan honors via partial-line diff; CC0 Tax-Calculator
  `policy_current_law.json` vendoring is license-verified (deep/05 §1) and the CI diff is concretely
  actionable; tenforty/PolicyEngine correctly observe-only.
- **Code claims accurate:** `method.rs`/`charitable.rs`/`return_1040.rs`/`other_taxes.rs`/
  `return_inputs.rs` correctly marked NEW (none exist); `schedule_d.rs:5-6` scope-out comment real —
  L17–22 extension genuinely needed; `form1040.rs` today fills only 7a + digital-asset (full-1040 is
  honest new work); `verify.rs` really has only the Yes/No-pair oracle (`topmost_yes_no_pair`) — the
  5-way + negative-cell extensions are needed exactly as planned; `f1_57` L12(2024)/L1z(2025)
  collision and `c1_8`/`c1_1` FQNs confirmed in deep/03 + fable/05; `tax_profile.rs` side-table
  pattern is a faithful mirror-template for P1.
- **FOLLOWUPS folding:** `fr-schedc-27a` folded (P6 "line 27a/48 + 28"); `fr-se-sscap-clamp` /
  `fr-profile-diagram-nit` are spec-text-only (no plan action due); `fr-8962-taxonomy` lands with P5.
  (`fr-schb-user-forced` — Minor 5.)
- **Ship mechanics:** phased merges to `full-return`, one whole-diff review before `main`, version
  bump — present (Sequencing & risk).

---

**Gate result: NOT GREEN.** 2 Critical / 5 Important / 7 Minor. Re-review required after fold
(including of the final fold, per workflow §2).
