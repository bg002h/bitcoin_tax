# FABLE Independent Review — `SPEC_full_return.md` — Round 4 (re-review of the r3 fold)

**Reviewer:** Fable (independent; author = opus). **Date:** 2026-07-12.
**Target:** `design/SPEC_full_return.md` **r4** (r4 changelog header; fold of
`design/full-return/reviews/SPEC-fable-review-r3.md` R3-I1..I4 + R3-M1..M10).
**Method:** every r3 finding re-verified against the r4 text; every load-bearing citation re-verified
against primary source: `crates/btctax-core/src/tax/se.rs` (the `addl` clamp :142–158, the `ss_cap`
clamp :126–133, the Interest exclusion :55–62, the `m2_business_interest_excluded_mining_included`
KAT), `crates/btctax-core/src/tax/compute.rs` (`interest_nii` :310–315, `crypto_agi`/`magi_with`
:364–368, the NIIT closure :369–380), `crates/btctax-core/src/event.rs` (the **complete** `IncomeKind`
enum :33–38; `business` :61), `crates/btctax-forms/src/schedule_se.rs` (`SE_FLOOR`/`Ok(None)` :44–61;
line-9 floor + 8a≥base skip :63–90), `crates/btctax-forms/src/schedule_d.rs` (:5–6 scope-out), recon
`deep/02` §§2.4/3.2/3.3 and `deep/04` :177–178/:219. Arithmetic machine-checked: the 8959 Part II
golden (Ex.2), the 8960 clamp equivalence, and the R3-I4 MAGI-binding counterexample.

**VERDICT: GREEN — 0 Critical / 0 Important / 5 Minor.**
All four r3 Importants are **fully resolved**, each with the prescribed normative text, source-anchored
(`≡ se.rs.addl`, `≡ compute.rs:369`), and KAT'd (KAT-5b, KAT-18). The r3 Minors are 8-of-10 fully
folded; the two part-folds (M1's Part V, M8's §1.2/list-taxonomy) survive only as Minors below,
together with three small new/carryover observations. **No fold-introduced regression of Critical or
Important severity was found.** Under `STANDARD_WORKFLOW.md` §2 this spec passes the gate.

---

## 1. r3 IMPORTANT findings — resolution audit

| r3 | Status in r4 | Evidence |
|---|---|---|
| **R3-I1** 8959/8960 clamps | **RESOLVED** | §5 stage 7 now reads `Part II = 0.9%·max(0, SE − max(0, thr − Σbox5))` with the `≡ se.rs.addl ≡ 8959 L11–L13` anchor — **machine-checked on the spec's own golden** (deep/02 Ex.2: Σbox5 = 280,000, thr = 250,000, SE base = 55,410 → L11 = max(0, −30,000) = 0 → L13 = 0.9%×55,410 = **498.69**, matching deep/02 :236/:255 and `se.rs:142–158` exactly; the r3 unclamped text gave 768.69). And `L17 = 3.8%·max(0, min(max(0, NII), max(0, MAGI − thr)))` — both statutory floors present; verified ≡ `compute.rs:369–380` (the extra inner `max(0,NII)` is redundant but exactly equivalent — checked both branches: for NII < 0 both forms give 0; for NII ≥ 0 identical) and ≡ deep/02 §2.4's transcription. No negative NIIT is derivable. KAT-6's "8959 Part I+II" requirement *forces* a Σbox5 > thr fixture (Part I > 0 ⟺ Σbox5 > thr), so the discriminating-fixture prescription is satisfied by construction. |
| **R3-I2** Schedule B trigger | **RESOLVED** | §7.1 now carries the **single normative site**, verbatim-equivalent to the prescription: taxable interest > $1,500 **or ordinary dividends > $1,500** or `foreign_accounts == Some(true)` (trigger (b)) or user-forced; `foreign_trust == Some(true)` refuses first (trigger (c) → Form 3520, §4.10). §5 stage 1 is now a pure pointer ("[Sch B per §7.1 trigger]" — grep confirms no second trigger text). Part III 7a/8 must be answered, tri-state fail-loud, FinCEN advisory retained. **KAT-18 added with both prescribed cases:** the $2,000-dividends/$100-interest household **files Schedule B Part II+III** (yes — the dividends term now fires), and the ≤$1,500 household with a foreign account files Part III. The r3 harm class (silently omitted mandatory Part III) is closed. |
| **R3-I3** business-flagged Interest | **RESOLVED** (option (a), refuse) | Three seams re-verified coherent: (1) §4.4a Sch C gross is now **kind-restricted** — `business == true AND kind ∈ {Mining, Staking, Airdrop, Reward}` — exactly `se.rs:55–62`'s SE-eligibility predicate, so printed Sch SE L2 = Sch C L31 **by construction** (same gross, same single `expenses` scalar both sides; the r3 "second silent wedge" is gone). (2) Business-flagged `Interest` ⇒ refuse: §4.4a sentence + §4.10 row (with the §1402(a)(2)-vs-NIIT rationale) + §9.2(ii) listing — per-row KAT applies. (3) §5 stage 7 8960 now says **"ALL lending interest is in NII (business-flagged Interest is refused)"** — consistent with `interest_nii` (`compute.rs:310–315`, no business filter); the r3 NIIT understatement is unreachable. **Gap check (new-defect scan): the partition is exhaustive** — `event.rs:33–38` shows `IncomeKind` = exactly {Mining, Staking, Interest, Airdrop, Reward}, so business∧SE-kinds → Sch C, business∧Interest → refuse, non-business → L8v covers every ledger state; every crypto ordinary dollar has one printed home or the return refuses. §4.7's dependent earned income ("Sch C net − ½SE") is also fixed by construction (no interest can reach Sch C net). |
| **R3-I4** reduce-to-delta scope | **RESOLVED** | §5 tail now scopes the invariant exactly as prescribed: 8959 collapses **exactly** (same `se.rs` base both sides, expenses included; Part I = 0 without wages — verified against `compute_se_tax`'s signature); 8960 collapses exactly **only** for no-SE regimes or NII-binding SE regimes; MAGI-binding SE ⇒ documented `absolute NIIT < delta` (½-SE + Sch C expenses visible only to the absolute side; `compute.rs:364` gross-`crypto_ord` cite verified). **Counterexample re-machine-checked:** Single, $200k business mining, $20k non-business interest → ½SE = 13,131.35, absolute MAGI = 206,868.65, absolute NIIT = **261.01** < delta **760.00** ✓. The inequality direction is provably correct in general (absolute MAGI ≤ engine MAGI ⇒ absolute ≤ delta whenever the absolute MAGI arm binds, NII equal on both sides post-refuse). KAT-5 pinned NII-binding; **KAT-5b added** pinning the inequality; both plan-time mis-fixes (strip ½-SE / unfreeze the engine) explicitly prohibited. |

## 2. r3 MINOR findings — resolution audit

| r3 | Status | Evidence |
|---|---|---|
| M1 Sch C fill fields | **RESOLVED (part)** | `naics_code` (default "999999", line B) + `accounting_method` (default Cash, line F) named in `ScheduleCInputs`; `expenses` → **line 27a → line 28** named. Residual: "Part V itemization left blank" → finding N1 below. |
| M2 box3+box7 / L24 | **RESOLVED** | §4.4a + §5 stage 7 both say "owner's own (box3+box7 tips)" (≡ `se.rs` doc "Box 3 + Box 7 tips; Schedule SE line 8a"); Part V now cites **L24** = L22 (+L23 RRTA=0) → 25c, matching §4.8's "25c = 8959 L24 + other_withholding". |
| M3 §6017 $400 floor | **RESOLVED** | §5 stage 7: base < $400 ⇒ SE tax = 0, NO Schedule SE filed, NO ½-SE, 8959 L8 = 0 — exactly the reused filler's behavior (`SE_FLOOR = 400`, `Ok(None)`, `schedule_se.rs:44–61`); no unbacked Sch 2 L4 is derivable. |
| M4 donee advisory | **RESOLVED** | §9.2 charitable-donee advisory (public-charity assumption; private-foundation ⇒ 20%/basis) — verified against deep/04 :177–178/:219. |
| M5 8960 printed line | **RESOLVED** | §5 stage 7 names **L7** (other modifications) for crypto lending interest "since it is NOT on 1040 2b" — and the double-count check passes: L1 = 2b (1099-INT only), L7 = L8v interest only; disjoint by construction. |
| M6 write-back | **RESOLVED** | §4: single mechanism (write to Y+1's `*_carryover_in`, no staging field); precedence stated (computed overwrites computed; user-entered ⇒ warn + `--force`); provenance on every carryover-in. |
| M7 phase de-dup | **RESOLVED** | §11 phase 4 parenthetical: income lines computed in phase 2; QBI carried as 0-stub by phase 3, completed in phase 4. No double-booked deliverable remains. |
| M8 8962 + list taxonomy | **RESOLVED (part)** | §9.2 restructured into the three §3.4-aligned lists; 8962/excess-APTC and the newly added credits now enumerated; AMT correctly in REFUSALS; 1099-R/SSA in unrepresentable. Residuals (list-anchor imprecision + §1.2) → N4. |
| M9 SALT crumbs | **RESOLVED** | §4.6: filler checks the 5a election box (deep/03 `c1_1`) iff `salt_use_sales_tax`; nonzero `salt_sales_tax_amount` with election off ⇒ **fail-loud refuse** (r2's prescription restored). |
| M10 no-Sch-C fail-loud | **RESOLVED (part)** | §4.4a: business income with `schedule_c == None` ⇒ fail loud (G15 pattern). Residual nit ("TaxProfile (2 scalars)") → N5. |

## 3. New-defect scan (fold regressions)

Checked and **clean**: the 8960 L7 wording vs L1/L2 (no double-count — L1 = 2b excludes L8v interest;
box1b ⊂ box1a counted once in L2; box2a reaches NII only via L5a); the kind-filter vs Interest-refuse
partition (exhaustive over the verified 5-kind enum — no silent income hole); KAT set 1–18 + per-row
(KAT-6's Part I+II implies the Σbox5>thr fixture; KAT-5/5b consistent with the scoped invariant;
KAT-18 matches §7.1; no numbering collision); §4.8 ↔ stage 7/8 payment wiring (L24/25c, L31 = Sch 3
L15) now agrees at every site; the §6017 floor vs §7.1's unbacked-line rule (no Sch SE ⇒ no Sch 2 L4
⇒ no 8959 L8 — coherent); the 8960 formula's redundant `max(0,NII)` (equivalent, not a behavior
change); §1.1/§7.1 forms lists; the r4 changelog against the actual edits (slightly overclaims
"M1–M10 folded" — the part-folds are captured below). Five Minors, none blocking:

## 4. MINOR

- **N1 (§4.4a/§7.3; residual of R3-M1):** filling Sch C **line 27a with Part V left blank** breaks the
  printed form's own arithmetic (27a is captioned "from line 48") — contra §3.1's "every filed form
  cross-foots". Fix: emit one Part V row ("Other expenses" + amount) and line 48 = `expenses` → 27a.
  Tax-unchanged; presentation only.
- **N2 (§5 stage 7; r3-carryover, not fold-introduced):** the Sch SE paraphrase `L10 = 12.4%×min(base,
  SS_base − owner's own (box3+box7 tips))` omits the `max(0,·)` floor on the cap (and the form's
  "8a ≥ wage base ⇒ skip 8b–10"). Reachable regime: a multi-employer owner with Σ(box3+box7) >
  $168,600 (§4.9's own population) + Sch C ⇒ literal formula yields a **negative** L10. Not Important
  because the implementation is the mandated frozen pair — `se.rs:126–133` clamps and
  `schedule_se.rs:63–90` floors line 9 and implements the skip — so a KAT written from the bad
  paraphrase would fail loud against the reused code, not file wrong. Fix is one token + one anchor:
  `min(base, max(0, SS_base − owner's own (box3+box7)))` "(≡ `se.rs` ss_cap; form: 8a ≥ base ⇒ skip
  8b–10)".
- **N3 (§7.1):** the "user-forced" Schedule B trigger has no named mechanism — no `ReturnInputs` field
  and no CLI flag anywhere in §4. Name one (e.g. `force_schedule_b: bool` or a fill-time flag).
- **N4 (§9.2/§1.2; residual of R3-M8):** excess-APTC/Form 8962 is filed under list (ii) REFUSALS
  "(§4.10)" but **no §4.10 row exists** and no 1095-A input exists to refuse on — by the spec's own
  taxonomy it belongs in list (iii) "unrepresentable (no input; would refuse if captured)". Same
  anchor imprecision for "AMT-screen trigger" (§4.11, not a §4.10 row). And §1.2 still doesn't
  mention 8962 (the r3 fix named §1.2 *and* §9.2).
- **N5 (§2; residual of R3-M10's nit):** the diagram still says "TaxProfile (2 scalars)" vs the
  ~9-field profile deep/02 §1.3 enumerates (and the changelog claims M10 fully folded).

## 5. Re-checked and found CLEAN (beyond §§1–3)

- **All r1/r2 resolutions spot-held** through the r4 edits: charitable 30%-class two-term ceiling
  (§4.6, deep/04 formula intact), Sch D four-path routing (§7.2 untouched), SALT either/or, §904(j)
  FTC block, Sch 2 L4 unbundling (`ss + medicare`, never `total`), ½-SE = `deductible_half`.
- **§5 stage 7 worked end-to-end** against deep/02 §§2.4/3.2/3.3: Part I (0.9%×30,000 = 270.00 on
  Ex.2), Part II (498.69, above), Part V L22 clamp (`max(0, Σbox6 − 1.45%·Σbox5)` = the form's
  "if zero or less, -0-"), NII assembly, MAGI = AGI fail-closed.
- **§5 tail invariant statement is now mathematically true** in both directions (equality cases and
  the documented inequality — proven, not just exampled).
- **§4.10** (15 rows incl. the new business-Interest row, each KAT'd), §4.11, §4.12, §6, §8, §11
  phases, §12 D-1..D-6: no collateral damage from the r4 edits.

## 6. Disposition

**GATE PASSES: 0 Critical / 0 Important / 5 Minor — the spec is GREEN for this gate.** All four r3
Importants are resolved with source-anchored normative text and discriminating KATs; the fold
introduced no Critical/Important regression. The five Minors are one-sentence-class polish (N1–N5)
that can be folded opportunistically at plan time or carried in `FOLLOWUPS.md`; none blocks
proceeding to `IMPLEMENTATION_PLAN_full_return` per `STANDARD_WORKFLOW.md`. Per §2, any future
substantive edit to the spec re-enters the review loop.
