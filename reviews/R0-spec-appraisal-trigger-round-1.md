# R0 architect review — SPEC minimal qualified-appraisal trigger (round 1)

- **Artifact:** `design/SPEC_appraisal_trigger_minimal.md`
- **Baseline:** `origin/main` @ HEAD `eae88df` (verified: `git rev-parse HEAD` == `eae88dfd…`, branch `main`, == `origin/main`).
- **Reviewer role:** independent architect (author ≠ reviewer). Gate: proceed to implementation only at 0 Critical / 0 Important.
- **Verdict:** **NOT green.** 0 Critical, **1 Important**, 3 Minor, 4 Nit. One blocking finding (I1) must be folded + re-reviewed before implementation.

---

## Recon-citation verification against current source (HEAD eae88df)

| Spec claim | Source | Result |
|---|---|---|
| `RemovalLeg { basis, fmv_at_transfer, term, … }` all populated | `state.rs:134-142` | **CONFIRMED.** Fields present. |
| `make_removal_legs` builds legs w/ basis/fmv/term | `fold.rs:201-238` | **CONFIRMED.** term via `term_for` (233), basis=`c.gain_basis` (231), fmv pro-rata w/ remainder-takes-rest so Σfmv==`*fmv` exact (220-226). *(Spec cites `201-237`; fn closes at 238 — Nit N4.)* |
| Donate folds to `Removal{kind:Donation}` via `consume_principal`+`make_removal_legs` | `fold.rs:1004-1075` | **CONFIRMED.** legs at 1041-1042; `Removal` push at 1067-1074. |
| `appraisal_required: bool` recorded, consumed by nothing | `state.rs:149`; set from `*appraisal_required` at `fold.rs:1072` | **CONFIRMED.** No reader anywhere (`grep`); it is a raw user CLI bool. |
| Advisory blockers emitted via `st.add_blocker(kind, Some(ev), detail)` | `state.rs:200-211` | **CONFIRMED.** `detail: impl Into<String>`, `pub(crate)` (fold is same crate). |
| BlockerKind + `severity()` Hard/Advisory arms | `state.rs:23-76` | **CONFIRMED.** Advisory arm = `SafeHarborTimebar \| UnmatchedOutflows \| Pre2025MethodNote` (73). New variant slots in cleanly. |
| Statutory-constant convention (`Usd = dec!(…)` + cite + "not indexed" + "never in a TaxTable") | `tables.rs:107-143` | **CONFIRMED.** `pub const NIIT_RATE: Usd = dec!(0.038);` pattern; `QUALIFIED_APPRAISAL_THRESHOLD: Usd = dec!(5000)` matches exactly. |
| Advisory render already handles new kinds (no render change) | `render.rs:982-990` | **CONFIRMED.** Loops `r.advisory`, prints `[{:?}] {evt} :: {detail}` — new variant auto-renders via Debug. |
| `Term` in scope in fold.rs (`Term::LongTerm`) | `fold.rs:12` (`use crate::state::{…, Term}`) | **CONFIRMED.** |
| Advisory never gates `compute_tax_year` | `compute.rs:239, 419-423` | **CONFIRMED.** Gate = `first_hard_blocker` (`b.kind.severity() == Severity::Hard`). Advisory categorically cannot gate. |
| `note_pre2025_once` is a single-fire guard to AVOID | `fold.rs:82-114` | **CONFIRMED.** Guards on `!blockers.iter().any(|b| b.kind == Pre2025MethodNote)`. Spec correctly says do NOT reuse it (per-event required). |

No blocking drift. Mechanical hooks are all accurate. Symbol `QualifiedAppraisalNote` / `QUALIFIED_APPRAISAL_THRESHOLD` do not yet exist (clean addition). Task-3 target `crates/btctax-cli/tests/verify_report.rs` exists.

---

## Independent web-verification of the legal cites

**(a) §170(f)(11)(C) — $5,000 claimed-deduction appraisal threshold — CONFIRMED.**
"For contributions of property where a deduction of **more than $5,000** is claimed, the [taxpayer] must obtain a qualified appraisal … and attach [Form 8283 info]." Form 8283 Section A = "$5,000 or less"; **Section B = "more than $5,000"** → exactly $5,000 needs NO appraisal. Sources: Cornell LII 26 U.S.C. §170; Bloomberg Tax §170; IRS Instructions for Form 8283 (12/2025). **The spec's DIRECTION (appraisal when claimed deduction > $5k) and the STRICT `>` (exactly $5,000 not flagged) are both correct.**

**(b) §170(e)(1)(A) — FMV reduction — CONFIRMED.**
Deduction = FMV reduced by "the gain which would **not** have been long-term capital gain if the property … had been sold at its fair market value." Consequences (verified): LT capital-gain property → **FMV** (no reduction); property that would yield ordinary income or short-term gain if sold — "inventory, art created by the donor, and property held one year or less" — → **basis** (lower of basis/FMV). Sources: Cornell LII 26 CFR §1.170A-1; Bloomberg Tax §170; The Tax Adviser (Jan 2026). **Spec's LT→FMV / ST-or-ordinary→basis mapping is correct.** (See I1 for the mis-example.)

**(c) CCA 202302012 — crypto > $5k needs a qualified appraisal; readily-valued exception does NOT apply — CONFIRMED.**
IRS Office of Chief Counsel memo (released Jan 13, 2023): a qualified appraisal under §170(f)(11)(C) is required for a crypto charitable deduction **> $5,000**; "cryptocurrency is not cash, a publicly traded security or any other listed type of readily valued property," so the **readily-valued exception does not apply**; an exchange-reported value does **not** substitute, and the reasonable-cause exception does not save it. Sources: irs.gov/pub/irs-wd/202302012.pdf; Journal of Accountancy (Jun 2023); McDermott Will & Emery; CBIZ. **Spec's crypto-specific grounding is accurate.**

All three cites are correctly stated and correctly directed. No cite inverts the conclusion.

---

## Tax-correctness of the term-aware proxy (highest priority) — DIRECTION CORRECT

Rule: flag Donate iff `Σ(leg.term==LongTerm ? leg.fmv_at_transfer : leg.basis) > $5,000`.

- **LT capital-gain property (the textbook case):** actual §170 deduction = FMV; proxy contributes `fmv_at_transfer` = FMV. **Exact — never missed.** Donate-appreciated-BTC (FMV $60k / basis $5k / >1yr) → proxy $60k > $5k → flagged. The rejected "FMV>$5k ∧ basis>$5k" AND-rule would have MISSED it (basis $5k not > $5k). The proxy choice is the correct fix.
- **ST property (appreciated):** deduction = basis; proxy = basis. **Exact.**
- **ST property (depreciated, FMV<basis):** deduction = FMV (§170(e) reduction = 0 on a loss); proxy = basis > FMV → **over-flags (safe).**
- **LT ordinary-income property (true inventory/dealer):** deduction = basis; proxy = FMV > basis → **over-flags (safe)** — the disclosed caveat direction.
- **LT depreciated capital-gain property:** deduction = FMV; proxy = FMV. **Exact.**

Conclusion: the proxy **never misses a required appraisal for capital-gain property** and **over-flags only in the safe ("verify") direction.** Strict `>` matches the "more than $5,000" boundary (Form 8283 Section A/B split). **This is correct and is the right minimal design.** The proxy is Decimal/exact (Σfmv is conserved by remainder-takes-rest); no float. The only tax defect is in the *explanation* of the over-flag, not its behavior — see I1.

---

## Findings

### Critical — none.

### Important

**I1 — The over-flag caveat's worked example ("e.g. mining") is tax-incorrect and ships to users.**
Locations: (1) Legal-grounding §170(e)(1)(A) line — "basis for … ordinary-income property (e.g. mining-income lots)"; (2) "Deferred imprecision" paragraph — "LT-held **ordinary-income** property (e.g. mining held >1yr) is deducted at basis"; (3) Task 2 emitted **detail text** — "long-term-held ordinary-income property (e.g. mining) is deducted at basis under §170(e)."

Mined BTC, once received, is a **capital asset**; the mining reward is ordinary income *at receipt* (Notice 2014-21) but that does not taint the coin's later character. A mined coin **held > 1 year** and then donated produces **long-term capital gain** if sold, so under §170(e)(1)(A) there is **no reduction** — the deduction is **FMV**, and flagging it when FMV > $5k is **correct behavior, not an over-flag.** Two compounding errors: (a) mining-then-held-LT is not §170(e) ordinary-income property; (b) it is used as the example of *over*-flagging when it is actually a *correct* flag. The genuine §170(e) ordinary-income category is property that would yield ordinary income / short-term gain if sold — **inventory / crypto held for sale by a dealer or in a trade or business (§1221(a)(1)), self-created property, ≤1-yr property** (per §170(e)(1)(A), independently web-verified). Because item (3) is **user-facing** in a tax app, it affirmatively misguides: a taxpayer could read it as "deduct my long-held mined BTC at basis" and **under-claim** a legitimate FMV deduction.

Why Important (not Critical): it does not invert the trigger or cause a missed appraisal — the proxy still behaves safely. Why not Minor: it is wrong tax content in emitted output, under a tax-correctness-priority gate.

**Fix:** In all three locations, drop "mining" as the ordinary-income-property example and replace with a correct one — e.g. "property that would produce ordinary income or short-term capital gain if sold (e.g. crypto held as inventory / for sale in a trade or business, or ≤1-yr lots)." Note that inventory character is independent of holding period (so the "LT-held" framing must be about asset *character*, not just >1yr). The detail text should say the proxy may over-flag such property, precise §170(e) determination deferred — without the mining claim.

### Minor

**M1 — Pin the proxy computation point to AFTER the fee re-home (determinism / persisted-legs match).**
Task 2 says compute "after `make_removal_legs`." But in the Donate arm, `consume_fee` (fold.rs:1046-1056) then `carry.rehome_onto_removal_leg(last)` (1057-1058) run *after* `make_removal_legs` and **mutate the last leg's `basis`** (`leg.basis += self.gain_basis`, fold.rs:274-276). For a **short-term last leg** (proxy uses `basis`), computing the proxy before the re-home would exclude the re-homed fee basis, while the **persisted** `Removal.legs` (pushed at 1067) are post-re-home. Magnitude is fee-cents and **LT legs are unaffected** (re-home touches only `basis`; LT uses `fmv_at_transfer`), hence Minor — but pin it for determinism. **Fix:** specify the proxy is computed from the **final `legs`** immediately before `st.removals.push(Removal{…})` (after `rehome_onto_removal_leg`). Optionally add a KAT: a fee'd ST donation whose proxy includes the re-homed fee basis.

**M2 — Add a decoupling KAT for the manual `appraisal_required` bool.**
Full decoupling from the user's manual bool is an explicit (and correct) design decision, and it is a documented risk that a later reviewer may "helpfully" couple them. Lock it in: KAT that (i) proxy > $5k with `appraisal_required=false` **still emits**, and (ii) proxy ≤ $5k with `appraisal_required=true` does **not** emit. Folds into Task 2.

**M3 — Add a §170(f)(11)(F) aggregation caveat to the emitted detail text.**
Deferring cross-donation aggregation is acceptable for a "minimal" trigger and is documented (Legal grounding / Out-of-scope / Task-3 FOLLOWUPS). But the emitted advisory is silent on it. Add one line to the detail: "checks each donation individually; similar-item donations aggregated across the tax year (§170(f)(11)(F)) are not considered — your yearly total of similar donations may still require an appraisal." Note the inherent limit the caveat cannot cure: the pure-small-donations false-negative (e.g. two $3k LT donations = $6k aggregate) emits **nothing at all** and cannot be caught per-event — that residual is an accepted minimal-trigger limitation and is adequately recorded in FOLLOWUPS.

### Nit

**N1 — FOLLOWUPS.md drift.** The "Standing roadmap" entry (FOLLOWUPS.md ~line 10) still describes this slug as "minimal appraisal-trigger **FMV>$5k∧basis>$5k**" — the AND-rule the spec explicitly rejects (it under-flags the LT-appreciated case). Reconcile to the term-aware proxy when the slug ships.

**N2 — Add a just-over-boundary KAT.** Complement the "exactly $5,000 → not flagged" KAT with "$5,000.01 → flagged" to nail the strict `>` from both sides.

**N3 — (optional) conflict-aware cross-check.** Full decoupling is fine and keeps this "minimal." A future enhancement: when proxy > $5k **and** the manual `appraisal_required == false`, note the disagreement (the most useful case). Non-blocking; do not add coupling now.

**N4 — Citation off-by-one.** Spec recon cites `make_removal_legs` at `201-237`; the fn spans `201-238` (closing brace at 238). Trivial.

---

## Answers to the charge questions

1. **Term-aware proxy direction:** Correct. Never misses capital-gain property (LT contributes FMV = the actual deduction); over-flags only safe (LT ordinary-income/inventory, depreciated ST). Strict `>` correct ("more than $5,000"). Only defect is the mis-example (I1), not the behavior.
2. **Legal cites:** All three independently web-confirmed and correctly directed — §170(f)(11)(C) (>$5k), §170(e)(1)(A) (LT→FMV, ST/ordinary→basis), CCA 202302012 (crypto >$5k needs appraisal, readily-valued exception inapplicable). No cite inverts the conclusion. The §170(e) *example* (mining) is wrong → I1.
3. **§170(f)(11)(F) deferral:** Acceptable for "minimal," structurally disclosed. The emitted detail should also carry a one-line aggregation caveat (M3); the pure-small-donations FN is an inherent, documented limitation.
4. **Advisory semantics:** Correct. Advisory-only (compute_tax_year gates only on Hard — verified); per-event (correctly avoids the `note_pre2025_once` single-fire guard); decoupled from the manual bool (right call — an independent computed cross-check; a manual-`false`+proxy>$5k is exactly the case to surface). Add M2 to lock decoupling.
5. **Hook/placement:** Correct — legs carry term/fmv/basis after `make_removal_legs`; constant belongs in `tables.rs`; new BlockerKind in the Advisory arm; no render change. One refinement: compute the proxy after the fee re-home so it matches the persisted legs (M1).
6. **Over-flag caveat disclosure:** The disclosure *mechanism* (detail text) is adequate, but its *content* is wrong (I1). Fix the example.
7. **Scope / TDD:** Right-sized at 3 tasks, independently testable. KAT set is genuine and mostly sufficient (LT-flagged, ST-not-flagged, mixed both ways, exact-$5k boundary, Advisory/never-gates, two-donations per-event, GiftOut-never-emits). Add: decoupling KAT (M2), just-over boundary (N2), and a fee'd-ST KAT if M1 is folded. No missing *task*.

**Gate status: BLOCKED on I1.** Fold I1 (and, recommended, M1–M3 + nits), persist this review verbatim, then re-review before implementation.

---

# Round 2 — re-review (fold verification)

- **Artifact re-reviewed:** `design/SPEC_appraisal_trigger_minimal.md` (revised).
- **Scope:** confirm the round-1 fold closed I1 + M1–M3 + nits, introduced no new tax error, and left the spec internally consistent. Proxy direction + legal cites were web-confirmed in round 1 — not re-litigated.
- **Verdict:** **NOT green.** 0 Critical, **1 Important (I1 residual — I1 only *partially* closed)**, 0 new C/I. M1, M2, M3 and all nits are CLOSED. One blocking line remains.

## I1 — PARTIALLY closed. Two of three flagged locations fixed; the third (Legal grounding) still ships the tax error.

Round-1 I1 named **three** locations. The fold fixed the two the round-2 charge enumerated but left the first untouched:

- **Location (2), "Deferred imprecision" bullet (spec lines 36–45): CLOSED / tax-correct.** Now character-framed — "**Ordinary-income property** — crypto held as **inventory / for sale in a trade or business (§1221(a)(1))**, self-created property, or other property whose sale would yield ordinary income — is deducted at **basis** … even when held >1yr" — and explicitly states the counter-case: "**Do NOT describe this as 'long-term-held property':** investment-held mined BTC held >1yr is a CAPITAL asset → LT capital-gain property → correctly deducted at FMV and correctly flagged (NOT an over-flag)." Meets charge 1(a) + 1(b). No under-claim-inducing "mining held >1yr = basis" language.
- **Location (3), Task-2 emitted detail text (spec lines 84–89): CLOSED / tax-correct.** "…crypto held as inventory/for sale in a trade or business (§1221(a)(1)) or other ordinary-income property is deducted at basis under §170(e) **REGARDLESS of holding period** — the precise determination is deferred; verify." Character-framed; carries the required "regardless of holding period" phrasing (charge 1(c)). The under-claim-inducing text is gone from **shipped output** — the highest-harm vector is remediated.
- **Location (1), Legal grounding §170(e)(1)(A) line (spec lines 15–17): NOT fixed — residual tax error.** Still reads: "…the claimed deduction is **FMV for long-term capital-gain property** and **basis for short-term OR ordinary-income property** *(e.g. mining-income lots)*." This is the exact I1 defect: a **mining-income lot is not categorically ordinary-income property.** Once received, mined BTC is a **capital asset** (the mining reward is ordinary income *at receipt* per Notice 2014-21; that does not taint the coin's later character). Held >1yr → LT capital-gain property → deducted at **FMV**, not basis. So "(e.g. mining-income lots)" as the exemplar of the "deduct at basis" category is tax-incorrect — the very conflation (acquired-as-ordinary-income ≠ ordinary-income-property-for-§170(e)) that I1 was raised to eliminate.

**Why this blocks (Important, not Minor):**
1. It is a **genuine tax error** surviving in the spec's **Legal grounding** — the definitional anchor the implementer relies on — under a tax-correctness-priority gate.
2. It **self-contradicts** the corrected Deferred-imprecision bullet in the *same document*: line 17 calls mined lots an example of "deduct at basis / ordinary-income property," while lines 40–42 explicitly state mined BTC held >1yr is a capital asset correctly deducted at FMV. A spec internally contradictory on the exact flagged point is not "ready to implement."
3. Round-1's fix instruction was unambiguous — "In **all three** locations, drop 'mining'." One location was missed; the "I1 closed everywhere" confirmation the charge asks for therefore fails.

**Why not Critical:** it does not invert the trigger or cause a missed appraisal (proxy behavior unchanged and still safe), and the **user-facing emitted text is fixed**, so no taxpayer under-claim flows from this residual line directly. Real-world filing harm is low; gate compliance is not met.

**Fix (trivial, one line):** in spec lines 15–17, drop "(e.g. mining-income lots)" or replace with a correct exemplar consistent with the corrected bullet — e.g. "(e.g. crypto held as inventory / for sale in a trade or business (§1221(a)(1)), or ≤1-yr lots)". No other change needed; re-review can be confined to this one line.

## M1 — CLOSED.
Task 2 (spec lines 75–80) now pins the proxy to the **final persisted legs, AFTER `make_removal_legs` AND `carry.rehome_onto_removal_leg` (`fold.rs:274-276`), immediately before `st.removals.push(...)`**, explicitly noting "a re-homed ST fee-cent basis is then included." Matches the persisted `Removal.legs`. Determinism concern resolved. (The round-1 fee'd-ST KAT was flagged "optional"; its absence is non-blocking.)

## M2 — CLOSED.
KAT (h) (spec lines 98–100): proxy>$5k with `appraisal_required=false` STILL emits; proxy≤$5k with `appraisal_required=true` does NOT emit. Both directions lock the independent-cross-check decoupling.

## M3 — CLOSED.
Emitted detail (spec lines 86–89) now includes the §170(f)(11)(F) cross-donation-aggregation caveat: "this flags a single donation; the $5,000 test also aggregates similar donated items across the tax year — cross-donation aggregation is not considered here." Cite is correct (§170(f)(11)(F) = aggregation of similar items of property).

## Nits — all CLOSED.
- **N2:** boundary KAT (d) now covers **$5,000.00 → NOT flagged AND $5,000.01 → flagged** (strict `>` from both sides).
- **N4:** `make_removal_legs` cite corrected to **`fold.rs:201-238`** (line 48).
- **N1:** Task 3 (lines 111–113) now instructs reconciling the FOLLOWUPS "Standing roadmap" AND-rule line → term-aware deduction proxy.

## No NEW Critical/Important introduced.
The fold's new text is tax-correct: "self-created property" and "inventory / for sale in a trade or business (§1221(a)(1))" are proper §170(e)(1)(A) ordinary-income categories; "deducted at basis … regardless of holding period" is correct; the §170(f)(11)(C)/(F) and CCA 202302012 cites are correctly stated and directed. Spec remains right-sized (3 tasks) and TDD-complete (KATs (a)–(h) + Task-1 + Task-3 verify KAT). The only defect is the *residual* (not new) I1 line.

## Verdict
**I1 is NOT fully closed** — the Legal-grounding line (spec lines 15–17) still exemplifies "ordinary-income property / deduct at basis" with "mining-income lots," which is tax-incorrect and self-contradicts the corrected caveat elsewhere. M1/M2/M3/nits are closed; no new C/I. **Not R0 GREEN.** One trivial single-line fold (drop/replace the mining exemplar), then a line-scoped re-review clears the gate.

---

# Round 3 — re-review (line-scoped, I1 residual only)

- **Artifact re-reviewed:** `design/SPEC_appraisal_trigger_minimal.md` (revised post round-2 fold).
- **Scope:** line-scoped to the §170(e)(1)(A) Legal-grounding line; confirm I1 closed, no new issue. All other findings were 0C/0I in round 2 and are not re-litigated.
- **Verdict:** **R0 GREEN. 0 Critical, 0 Important.** I1 is fully closed. Spec is ready to implement.

## I1 — CLOSED.

The §170(e)(1)(A) Legal-grounding line (spec lines 15–19) now reads:

> "the claimed deduction is **FMV for long-term capital-gain property** and **basis for short-term OR ordinary-income property** (ordinary-income = property by CHARACTER: crypto held as **inventory/for sale in a trade or business (§1221(a)(1))**, self-created property, or ≤1-yr lots — NOT investment-held mined BTC held >1yr, which is a capital asset deducted at FMV)."

All four charge items confirmed:

1. **"(e.g. mining-income lots)" is gone.** The line now describes ordinary-income property by CHARACTER — inventory/for-sale (§1221(a)(1)), self-created property, ≤1-yr lots — and explicitly states the counter-case: investment-held mined BTC held >1yr is a capital asset deducted at FMV. No mining lot is exemplified as "deduct at basis" property. Tax-correct.

2. **Internal consistency: all three locations now align.** (a) Legal-grounding §170(e)(1)(A) line: character-framed, mined BTC >1yr explicitly called a capital asset at FMV. (b) "Deferred imprecision" bullet (spec lines 38–47): same character-framing ("crypto held as inventory / for sale … even when held >1yr"), same explicit counter-case ("mined BTC held >1yr is a CAPITAL asset → … correctly deducted at FMV"). (c) Task-2 emitted detail text (spec lines 86–89): "crypto held as inventory/for sale in a trade or business (§1221(a)(1)) or other ordinary-income property is deducted at basis … REGARDLESS of holding period." All three are character-framed; none contains "mining held >1yr = basis." The self-contradiction round-2 identified is eliminated.

3. **No remaining tax-incorrect statement.** "Mined BTC" appears in the spec only once (Legal-grounding line, now) and only to *correctly* identify it as a capital asset deducted at FMV when held >1yr. No holding-period-vs-character conflation survives. No statement would induce a taxpayer to under-claim a legitimate FMV deduction on long-held mined BTC.

4. **No new issue introduced.** The edit is additive/clarifying: it replaced one misidentified exemplar with a correct character-based description plus an explicit carve-out for the mined-BTC case. It does not alter the proxy formula, the threshold, the KATs, the hook placement, the advisory semantics, or any cite. No new tax claim is introduced. The legal-grounding line's core mapping (LT capital-gain property → FMV; ST-or-ordinary → basis) was already correct; only the exemplar was wrong and is now fixed.

## Gate status: **R0 GREEN.**

0 Critical / 0 Important. I1 fully closed; M1/M2/M3/nits were already closed in round 2. The spec is internally consistent, tax-correct, and ready to proceed to implementation.
