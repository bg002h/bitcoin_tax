# PLAN review r2 — `IMPLEMENTATION_PLAN_full_return.md` (Fable, independent re-review, 2026-07-12)

**Target:** `design/IMPLEMENTATION_PLAN_full_return.md` (DRAFT r2 — r1 fold).
**Prior:** `reviews/PLAN-fable-review-r1.md` (2 Critical / 5 Important / 7 Minor).
**Implements:** `design/SPEC_full_return.md` (GREEN r4). **Gate:** STANDARD_WORKFLOW §2 — advances only
at 0 Critical / 0 Important.

**Verdict: GREEN — 0 Critical / 0 Important / 4 Minor (new, wording-level; none blocking).**

All 14 r1 findings verified resolved against the r2 text and, where load-bearing, against primary
source (`crates/btctax-adapters/src/tax_tables.rs`, `crates/btctax-core/src/tax/{types,compute,se}.rs`,
`design/full-return/recon/deep/01-tax-table-qdcgt-method.md`, `design/full-return/FOLLOWUPS.md`).

---

## r1 findings — resolution verification

### C1 (bracket-edge assertion) — RESOLVED, and the corrected claim independently re-verified

P0 task 4 now asserts **"every bracket edge < $100k is a multiple of $25"** over every bundled
`TaxTable` year, plus a pinned midpoint-edge cell KAT (TY2025 Single [11,900, 11,950), edge 11,925),
plus the three deep/01 worked examples; acceptance says "the mod-25 edge assertion passes for
2017/2024/2025/2026"; the spec §8/KAT-3 wording erratum is recorded (`FOLLOWUPS.md`
`spec-s8-kat3-mod25`, with the correct offending edges 9,325 / 11,925 / 48,475 ≡ 25 mod 50).

**Mechanical re-check of the corrected claim** (script over every `br(dec!(…))` edge in
`tax_tables.rs`):

| Year | sub-$100k ordinary edges | all ≡ 0 (mod 25)? |
|---|---|---|
| 2017 | 0, 9,325, 13,350, 18,650, 37,950, 50,800, 75,900, 76,550, 91,900 | ✅ |
| 2024 | 0, 11,600, 16,550, 23,200, 47,150, 63,100, 94,300 | ✅ |
| 2025 | 0, 11,925, 17,000, 23,850, 48,475, 64,850, 96,950 | ✅ |
| 2026 | 0, 12,400, 17,700, 24,800, 50,400, 67,450 | ✅ |

Zero violations; 11,925 is the exact midpoint of [11,900, 11,950); TCW continuity at an edge (rate
step contributes 0 at the boundary) makes the midpoint-edge cell reproducible, exactly as the task's
rationale states. The mod-12.5/$25-bin caveat is correctly noted vacuous (no edge < $3,000 exists in
any bundled year except 0). Written test-first, the assertion now goes green on frozen data —
the r1 mid-phase-design-decision hazard is gone.

### C2 (spec §6 dual reporting) — RESOLVED

P4 task 8 is the missing task: render absolute-liability lines + crypto delta side-by-side with the
§6 "different questions" labels, document the delta-deduction as approximate, never reconcile to the
dollar, with a presence/labeling KAT on a fixture where `absolute_with − absolute_without ≠ delta`
(AGI-sensitive deduction). P4 acceptance carries "the dual-report surface renders + labels
correctly". Matches spec §6 fully (labels, approximation doc, non-reconciliation).

### I1 (KAT ownership / KAT-18) — RESOLVED

Ownership line now reads KAT-15 → P2 and KAT-18 → P2 (decision) + P6 (fill), matching the phase
text: P2 task 2 implements KAT-15 (L8v-vs-L3 partition + cross-foot + R3-M10 fail-loud); P2 task 4
is the new Schedule B trigger task (interest>$1,500 OR ord-div>$1,500 OR
`foreign_accounts==Some(true)`; Part III 7a/8 tri-state fail-loud; both spec-§10 KAT-18 cases named);
P6 task 3 carries the fill half. Walked all of KAT 1–18 + 5b + refuse rows against the ownership
table and phase texts: every KAT has exactly one owner (the multi-phase entries — 9, 18, 11,
refuse-rows — are deliberate layered splits, each half named in its phase). No unowned, no
double-owned.

### I2 (resolver dependency inversion) — RESOLVED

P1 task 3 ships the resolver skeleton with the `ReturnInputs` arm explicitly stubbed
`NotComputable("derivation pending")`; the stored-`TaxProfile`/pseudo/missing arms + provenance land
in P1; P2 task 5 completes the arm and owns the full §4.12 precedence + two-sources-of-truth KAT.
P1 acceptance now claims only "resolver skeleton (non-`ReturnInputs` arms) + provenance tested"; P2
acceptance claims "full resolver precedence tested". P1 remains fully testable (the stub itself is
assertable; the other three arms need nothing from P2). The stub is fail-closed in the right
direction: a vault holding both `ReturnInputs` and a stored `TaxProfile` mid-build gets
`NotComputable`, never a silent fall-through to the second source of truth. (Wording nit → Minor m3.)

### I3 (FROZEN guard) — RESOLVED

Global invariants bullet 1 now enumerates the frozen-path set —
`crates/btctax-core/src/tax/{types.rs, compute.rs, se.rs}` plus the named delta helpers — and P0
task 0 defines the guard as a CI test pinning each frozen file's **content** (SHA-256 or
`git diff --exit-code` vs baseline), with baseline updates as separately-reviewed changes.
Verified the helper enumeration is closed under the file set: `ordinary_tax_on` (compute.rs:24),
`preferential_tax` (compute.rs:57), `net_1222` (compute.rs:137), and the NIIT closure
(compute.rs:369-380) all live **inside** `compute.rs`, so the three-file content pin covers every
named helper. `se.rs` exists at the enumerated path. Content-pin (not public-surface) matches the
"never *edit*" invariant. (Residual scope question on what-if/tests → Minor m4.)

### I4 (the frozen seam) — RESOLVED, verified against the frozen contract

P2 task 3 now draws the two-AGI distinction exactly right: (a) `return_1040` computes the absolute
**with-crypto** AGI (L11 = L9−L10) for the filed return; (b) `derive_tax_profile` populates
`magi_excluding_crypto` / `ordinary_taxable_income` from **non-crypto line items only** — ledger
`crypto_ord`, crypto gains, and the Schedule-C-driven ½-SE stay out. Checked against current source:
`types.rs:34-38` defines both scalars as "EXCLUDING all app-computed crypto items" / "Modified AGI
excluding crypto", and `compute.rs:364-368` computes `crypto_agi` and sets
`magi_with = magi_excluding_crypto + crypto_agi` — the engine adds the crypto delta itself, so the
plan's exclusion rule is the only reading that doesn't double-count. Keeping ½-SE out is likewise
correct: it is Schedule-C-(crypto-)driven, invisible to the frozen delta, and is exactly the pinned
KAT-5b MAGI wedge (spec §5 tail, R3-I4). The KAT is now discriminating as demanded: crypto-income
fixture, derived-profile delta vs an **independently hand-built** exclusion profile, with the
explicit requirement that a same-misreading comparison profile must NOT pass.

### I5 (rounding-mode proof mis-sequenced) — RESOLVED, arithmetic re-verified

P0 task 1's red test is now the deep/01 half-even-**discriminating** cells — MFJ [11,600, 11,650) =
1,163 and Single [3,000, 3,050) = 303 — with the fault-inject check that `round_cents` FAILS them.
Re-verified: midpoint taxes are exactly $1,162.50 / $302.50; half-up → 1,163/303, half-even →
1,162/302 (discriminating ✅); deep/01:13/68-69 print exactly these cells and deep/01:255 demands
exactly this fault-injection. KAT-9 (task 6) is correctly re-scoped to printed-line rounding +
cross-foot only ("NOT the mode; that's task 1") — and indeed 271.50/499.50 round to 272/500 under
*both* modes (272 and 500 are even), so KAT-9 could never have proven the mode; 771 arises only from
sum-then-round. KAT-9 re-assertion on real 8959 lines at P4/P6 is carried in the ownership line and
task 6. (Ownership-line annotation nit → Minor m2.)

### Minors 1–7 — ALL FOLDED

1. **M1** — P1 task 4 signature is `fn screen(&ReturnInputs, &TaxTable) -> Option<Blocker>`; "one
   KAT per **input-screenable** row" with the six rows named; compute-dependent rows explicitly
   deferred ("TI≤0 → P3"); acceptance says "every **input-screenable** refuse row". ✅
2. **M2** — P4 task 1 adds the QBI REIT/PTP carryforward-out write-back + R3-M6 precedence KAT;
   P3 task 3 carries the charitable R3-M6 KAT; P1 task 5 is correctly storage-only. ✅
3. **M3** — P0 task 5: indexed data added as `Option`/defaulted so `ty2017/25/26` still compile;
   SALT cap $10k/$5k, excess-SS MAX, FTC ceiling placed in core `tables.rs` NOT `TaxTable`.
   Verified the cited convention is real: `tax_tables.rs:9-11` says statutory constants are "never
   placed in a `TaxTable`". ✅
4. **M4** — P4 task 6 pins L36 = 0/blank in v1; spec §4.8 gap recorded (`FOLLOWUPS.md`
   `spec-s48-l36`). ✅
5. **M5** — P2 task 4 drops the unused "user-forced" clause, resolving `fr-schb-user-forced`. ✅
6. **M6** — P6 task 2 now says "deep/03-**style FRESH extraction**" and names Schedule C /
   8959 / 8960 / 8995 as NEW (deep/03 holds only the six existing roots). ✅
7. **M7** — all five sub-items landed: P3 task 4 QBI 0-stub (L13=0, L14=L12, R3-M7); P4 task 7
   CTC/ODC L19=0 + advisory KAT; P6 acceptance form-set-closure KAT; P1 task 5 SSN `--stdin` +
   masked rendering (+ acceptance); P3 task 2 5a election fail-loud (R3-M9) + P6 task 3 `c1_1`
   checkbox fill. ✅

---

## New-defect scan of the fold (r2 deltas)

- **P2 five-task ordering:** sound — task 5 (complete the resolver arm) depends on task 3
  (`derive_tax_profile`), which precedes it; task 4 (Sch B trigger) needs only task 1's income items.
- **P1 testability after the precedence move:** intact (see I2 above).
- **Mod-25 assertion edge cases:** the $0 edge trivially passes (no false red); edges just above
  $100k (100,500 / 100,525 / 100,800 / 103,350 / 105,700) are correctly excluded by the "< $100k"
  scope and irrelevant to the Table (which ends at $100k); in the $25-bin region the mod-25 form is
  strictly conservative (a hypothetical mod-12.5 midpoint edge would go red → forced review), and no
  such edge can exist in Rev.-Proc. whole-dollar data. No unsound acceptance.
- **KAT-9's P4/P6 re-assertion, KAT-5b pinned-expectation framing, phase DAG, ship mechanics:** all
  unchanged from their r1-CLEAN state; the fold introduced no regressions in them.
- **r2 changelog accuracy:** every claim in the changelog corresponds to a real edit verified above;
  header status/pending-review updated correctly.

---

## Minor (new; none blocking)

1. **m1 — ownership line mislabels excess-SS as compute-dependent.** KAT-ownership block:
   "refuse-row KATs → … P3/P4 (compute-dependent: TI≤0, **excess-SS**)". The single-employer
   excess-SS refuse row is **input-screenable** — P1 task 4 lists it in the input-screenable set
   (that is the entire point of the M1 `&TaxTable` signature widening; box4 vs MAX needs no
   computed value). No KAT is orphaned (P1 owns it; P4 acceptance re-asserts end-to-end), so this is
   a label contradiction only. Fix: drop "excess-SS" from the compute-dependent parenthetical (or say
   "P4 re-asserts end-to-end"). While there: have P3 name the TI≤0 refuse KAT on a task line (it is
   currently phase-assigned via P1 task 4's pointer + the ownership line, task-unnamed).
2. **m2 — ownership line's "KAT 9 → P0 (arithmetic + round-mode)" re-blurs what task 6 separates.**
   Task 6 states KAT-9 proves cross-foot "NOT the mode; that's task 1", but the ownership annotation
   attributes "round-mode" to KAT-9 again. Tasks are correct and normative; fix the annotation to
   "P0 (arithmetic/cross-foot; mode = task 1's cells)".
3. **m3 — P1 task 3 parenthetical "(no vault can hold `ReturnInputs` yet)" is false at phase end:**
   P1 task 2 lands the side-table CRUD before task 3. Harmless — the stub is fail-closed (see I2),
   which is the real safety argument; reword the justification to "fail-closed: refuses rather than
   falling through to a second source of truth". (Inherited from r1's own suggested fix wording.)
4. **m4 — FROZEN pin scope for the non-engine surfaces left implicit.** The content-pin is concrete
   for `{types,compute,se}.rs`, but the invariant also says "never alter what-if/pseudo-reconcile or
   the existing crypto tests" (and spec §2 lists "~80 constructors") without stating whether those
   files join the content-pin set or are review-enforced only (note: content-pinning live test files
   would false-red on legitimate additive tests). P0 task 0 should state the choice explicitly;
   its own phase review gates the final enumeration, so this does not block the plan gate.

---

## Re-checked and found now-CLEAN

- C1's corrected assertion holds mechanically on all four bundled years (table above); the
  midpoint-edge KAT cell is genuinely a midpoint edge; deep/01 citations (:13, :43, :59, :68-69,
  :255) all real and quoted accurately.
- The I4 seam wording matches the frozen contract verbatim (`types.rs:34-38`, `compute.rs:364-368`);
  the ½-SE exclusion is consistent with spec §5-tail/KAT-5b.
- I3's helper enumeration is closed under the three pinned files (all four helpers defined in
  `compute.rs`).
- KAT ownership walk: 18 KATs + 5b + refuse rows, all singly-owned or deliberately layered.
- FOLLOWUPS: both spec errata present with accurate content; `fr-schb-user-forced` resolution
  (drop-the-clause) matches the FOLLOWUP's stated options.
- Everything on r1's CLEAN list that the fold touched (phase DAG, frozen-engine preservability,
  KAT-5b framing, recon anchors, ship mechanics) — no regressions.

---

**Gate result: GREEN.** 0 Critical / 0 Important / 4 Minor. Per STANDARD_WORKFLOW §2 the plan passes
this gate; the 4 Minors are wording/traceability fixes foldable without re-opening the gate (fold →
they are one-line edits; if folded, a confirmation pass on those lines suffices). → pending user
review before implementation begins.
