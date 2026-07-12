# Fable Recon F6 — Completeness / Adversarial Critic (final round-2 pass)

**Agent:** Fable F6, second pass (round 2) — the last gate before the spec. **Date:** 2026-07-11.
**Mandate:** not re-verifying settled math — finding what is STILL UNACCOUNTED FOR across the whole
recon corpus (opus `00`–`05` + `deep/01`–`05` + fable `01`–`05`) before the spec is written.
**Method:** walked the full TY2024 1040 line flow (`01-form-graph.md` §2/§6) against the input model
(`04-input-data-model.md`) and the locked deep-dive results; cross-checked assertions against current
source in `crates/btctax-forms/` and `crates/btctax-core/`; cited primary IRS sources on every
asserted gap. Corrections already surfaced this pass (F1 SALT-halve-last, F2 pref-cap + binding-min,
F3 rounding asymmetry/DOB/L13b-in-8995-L11, F5 `f1_57` collision + on-state reassignment) are
**factored in, not re-derived**.

**Verdict:** the corpus is spec-ready on the *computation core* (method, NIIT/Medicare, deductions,
field maps) but has **3 BLOCKER-class holes** — all in the "everything around the core" layer: the
Schedule 1 input surface is undefined, the form-set is not closed under the IRS's own "Attach…"
requirements, and no whole-return rounding/formatting convention exists outside QDCGT line 25.
Plus 12 IMPORTANT and ~10 MINOR items, each with a one-line recommended resolution.

---

## 0. Prioritized gap table (BLUF)

| # | Rank | Gap (one line) |
|---|---|---|
| G1 | **BLOCKER** | `Sch1Income`/`Sch1Adjustments` are named but never defined — no enumerated Schedule 1 line list, and every candidate line has an unmodeled limitation worksheet (state-refund §111, IRA phase-out, student-loan phase-out, HSA→8889) |
| G2 | **BLOCKER** | Form-set not closed under "Attach Form X": no 8959 / 8960 / 8995 PDF fillers planned, and the existing Schedule D filler **scopes out lines 17–22 incl. the §1211 line-21 loss limit** |
| G3 | **BLOCKER** | No whole-return rounding + negative-formatting convention — deep/01 §3.2 covers only QDCGT L25; every other form line (8959 L18 = 768.69, Sch A, AGI spine, Sch 2 L21) is undecided, and F2 F-E's ±$1 ambiguity generalizes to the whole return |
| G4 | IMPORTANT | Source-precedence is stated for `report` only; must be ONE resolver used by report/TUI/optimize/what-if/export, and mitigation (c) (`tax-profile set` refuses/warns when `ReturnInputs` exists) is still "consider", not decided |
| G5 | IMPORTANT | `W2` struct is missing fields the locked math requires: **box 6** (8959 Part V L19 — deep/02 Ex.2 consumes it; recon-04 marks it OUT and the struct has no field), box 4 (excess-SS), box 19 (local SALT), owner tag (known) |
| G6 | IMPORTANT | Multi-employer **excess-SS credit** (Sch 3 L11 → 1040 L31) named but never specced: per-person across employers, 2024 max $10,453.20, multi-employer-only rule |
| G7 | IMPORTANT | Which AGI feeds Schedule A? Charitable ceilings + medical floor key off **with-crypto** AGI in the absolute path, but `derive_tax_profile` (recon-04 §5.1) computes the deduction on **non-crypto** AGI — the two paths diverge and nothing documents it |
| G8 | IMPORTANT | Charitable carryover must be **reduced even in standard-deduction years** (Reg. §1.170A-10(a)(2); Pub 526 "Carryovers") — absent from the deep/04 engine spec; silently wrong multi-year state |
| G9 | IMPORTANT | Captured-but-inert inputs need an enumerated refuse-guard table (W-2 box 12 codes A/B/M/N/Z/W, box 8, box 10; INT box 9 / DIV box 13; DIV 2b–2d guard exists) — "fail closed" is asserted but never enumerated |
| G10 | IMPORTANT | Foreign tax paid (INT box 6 / DIV box 7) not even captured; the §904(j) no-1116 election (≤$300/$600 → Sch 3 L1) is the common index-fund case — decide compute-or-refuse |
| G11 | IMPORTANT | Schedule B **Part III (7a/7b/8)** answers are mandatory whenever Sch B is filed; no input field exists for them |
| G12 | IMPORTANT | Sch A line 5a composition undefined (Σ box 17 + box 19 + estimated state payments + prior-year balance vs the separate user-entered `salt_income_or_sales`) — double-count risk; 5a sales-tax election checkbox has no input |
| G13 | IMPORTANT | AMT screening: Sch 2 L1 is hard-0 with no "Worksheet To See if You Should Fill in Form 6251" screen and no LIMITATIONS entry — decide screen-refuse vs documented exclusion |
| G14 | IMPORTANT | Layered test plan (05 §2) predates every round-2/Fable correction — the new-KAT list (§5 below) must be folded into the spec's test plan |
| G15 | IMPORTANT | MFS-both-itemize: modeled as `bool`; must be tri-state (unknown ⇒ fail loud, per deep/04 §4) and drive BOTH directions (std = 0 *and* header box 12b checked on the PDF) |
| G16–G25 | MINOR | See §4: direct-deposit/L36/L38 decisions, IP PIN, designee, extension payment (Sch 3 L10), `other_withholding` gate, "Sch D not required" box policy, dependent-filer earned income, TI ≤ 0 refuse, Sch B always-file policy, 1040-SR/EIC LIMITATIONS notes |

---

## 1. The 1040 (TY2024) line-walk — what the planned assembly can and cannot produce

Walking `01-form-graph.md` §2 against `04-input-data-model.md` + deep/02/04 + F3:

**Producible today from the modeled inputs (✓):** 1a/1z, 2a, 2b, 3a, 3b, 7 (crypto Sch D + DIV 2a),
9, 11, 12 (deep/04), 14, 15, 16 (deep/01 + F2 fixes), 18, 21, 22, 23 (Sch 2 L4/L11/L12/L21 per
deep/02), 24, 25a, 25b, 25c (8959 L24 + `other_withholding`), 25d, 26, 32, 33, 34–37. Header:
filing status, digital asset, dependents (with F3's DOB/SSN-validity upgrade), std-deduction
checkboxes, presidential fund, occupation.

**NOT producible or undefined (the gaps):**

| 1040 line | Problem | Gap # |
|---|---|---|
| **L8 / L10** (Sch 1 L10/L26) | `Sch1Income`/`Sch1Adjustments` never enumerated; every in-scope line has an unmodeled worksheet | **G1** |
| **L13** (QBI) | Math locked (F3) but claiming it requires "Attach Form 8995" (1040 L13 caption) — no 8995 filler exists or is planned | **G2** |
| L17 (Sch 2 L3) | Hard-0 with no AMT screen or documented limitation | G13 |
| L20 (Sch 3 L8) | Foreign tax paid not captured; §904(j) simple path unresolved | G10 |
| **L31** (Sch 3 L15) | Excess-SS credit (L11) unspecced; extension payment (L10) unmodeled | G6, G24 |
| L35b–d / L36 / L38 | Direct-deposit, apply-to-next-year, penalty: all undecided (inputs absent) | G16 |
| L1b–1h, 4a–6b, 19, 27–29 | Structurally 0/blank — correct **only if** the LIMITATIONS doc + refuse-guards say so explicitly (e.g. a 1099-R the user holds has no entry path; the doc must own that) | G9 / LIMITATIONS |

Lines 4a–6b (IRA/pension/SS income) are out of scope *by design*; the fail-closed posture is
structural (no input exists). That is the FFFF pattern and fine — but the LIMITATIONS doc must list
them per-line, and the spec must say the absence is deliberate.

---

## 2. BLOCKERS (spec cannot be written without resolving)

### G1 — The Schedule 1 input surface is undefined [BLOCKER]

`ReturnInputs` (recon-04 §5.1) contains `sch1_income: Sch1Income` and `sch1_adjustments:
Sch1Adjustments` — **neither struct is defined anywhere in the corpus.** `01-form-graph.md` §1F
names the "common live lines" (Part I L1 state refund, L7 unemployment; Part II L13 HSA, L15 ½-SE
(exists), L18 early-withdrawal (modeled via INT box 2), L20 IRA, L21 student loan) but every one of
those except L15/L18 carries a limitation the corpus never models:

- **L1 state refund** — taxable only per the tax-benefit rule (IRC §111(a)); the 2024 Schedule 1
  instructions' *State and Local Income Tax Refund Worksheet* needs prior-year itemized/SALT-cap
  facts. A raw user-entered box silently overtaxes anyone whose prior-year SALT was capped at $10k
  (the common case — refund then usually nontaxable).
- **L20 IRA deduction** — active-participant phase-out (W-2 box 13 "Retirement plan", which the
  model captures but ignores): TY2024 MAGI $77k–$87k S / $123k–$143k MFJ / $230k–$240k
  spouse-not-active / **$0–$10k MFS** (Notice 2023-75; Sch 1 L20 *IRA Deduction Worksheet*).
- **L21 student-loan interest** — $2,500 cap, MAGI phase-out $80k–$95k S / $165k–$195k MFJ, and
  **MFS cannot claim it at all** (IRC §221(e)(2); Sch 1 L21 worksheet). The MAGI here is itself
  special (AGI computed *without* this deduction — the worksheet resolves the circularity).
- **L13 HSA** — requires Form 8889 (limits by coverage type); see also G9 (box 12 code W).
- **L7 unemployment** — needs a 1099-G capture (no struct exists).

F4's hardening note (a) already rules out a free-form signed grab-bag. **Resolution:** the spec must
publish the exact enumerated Sch 1 line list for v1 with a per-line policy from {full worksheet |
user-attests-already-limited + advisory | refuse}. Recommended v1 minimum: L1 (user enters the
*taxable* portion + a worksheet advisory), L7 via a 1099-G struct, L15/L18 (already derived), L21
with the full worksheet (small, closed-form), L20 either full worksheet or refuse-when-box-13-set,
L13 refuse (couples to 8889). Everything else: no input, listed in LIMITATIONS.

### G2 — Form-set closure under "Attach Form X" [BLOCKER]

The IRS's own line captions make attachments mandatory: Schedule 2 L11 **"Attach Form 8959"**,
L12 **"Attach Form 8960"**, 1040 L13 **"Attach Form 8995 or Form 8995-A"**, Sch 2 L4 "Attach
Schedule SE". The recon computes absolute 8959/8960 (deep/02) and captures QBI (F3), but the PDF
plan (recon-03 + deep/03 + F5) covers **only** 1040 + Sch 1/2/3/A/B. Current
`crates/btctax-forms/src/` has fillers for 8949, Schedule D, Schedule SE, 8283, and partial 1040 —
**no f8959.rs / f8960.rs / f8995.rs, no bundled PDFs, no field maps, no feasibility check.**
A paper return showing $646 on Sch 2 L12 with no Form 8960 behind it is an incomplete filing.

Compounding it: the existing Schedule D filler **explicitly scopes out lines 17–22**
(`crates/btctax-forms/src/schedule_d.rs:5-6`: "lines 17-22 … are NOT filled — the CLI prints a
notice. Line 21 (the §1211 loss limit) lives inside that scoped-out block"). For an absolute return
those lines are not optional: **L21 is the mandatory loss line in a loss year** (Sch D instructions,
line 21: "enter the smaller of the loss on line 16 or $3,000 ($1,500 MFS)"), and L17–L20 are the
printed gate that selects the QDCGT worksheet (the very path deep/01 locked).

**Resolution:** spec must (a) extend the Sch D filler to lines 17–22 (L18/L19 = 0 by the existing
28%/§1250 refuse-guard; L20 = Yes), (b) add 8959 + 8960 to the fill set (both are 1-page,
fixed-line, same XFA-hybrid family — extraction is a small F5-style pass, but it must be *scheduled*),
and (c) decide QBI: either add the trivial 8995 map or hard-refuse when DIV box 5 > 0 and no
override — an override amount **without** an attached 8995 must not be printable on a
non-DRAFT return.

### G3 — Whole-return rounding + negative-formatting convention [BLOCKER]

deep/01 §3.2 fixes rounding for exactly one line (QDCGT → 1040 L16). F2 F-E documents the ±$1
worksheet-internal ambiguity and blesses carry-cents-round-once *for that worksheet*. Nothing in the
corpus decides the rest of the return, and the p. 23 rule (2024 i1040: "If you do round to whole
dollars, you must round **all** amounts … include cents when adding and round off only the total")
is an all-or-nothing election across the whole return:

- Does 8959 L18 = 768.69 (deep/02 Ex.2) print as **769** on Sch 2 L11? Does Sch 2 L21 then sum
  rounded line values or cents? Same question for 8960 L17, Sch A lines 3/4, every AGI-spine sum,
  and the withholding lines (W-2 box 2 arrives in cents).
- Without one global rule, Layer-2/3 golden diffs are ±$1 noise on *every* line — the exact failure
  mode that made Layer 0 a blocker in the first place (00-SYNTHESIS §2).

**Resolution (recommend):** adopt the round-all-amounts election globally — *every form-line entry
is `round_dollar`ed at the line*; within any single IRS worksheet, carry cents and round once at the
line that lands on a form (deep/01's convention, generalized); inputs are accepted in cents and
rounded at first form-line use. State it in `conventions.rs` docs and KAT the 8959→Sch2→1040
composition.

**Negative formatting (the other half):** `fmt_money` today is raw `Decimal` Display →
`-3000` (leading minus, no grouping). Some cells pre-print parentheses and must receive the
**magnitude** (Sch 1 L8a NOL, per deep/03 "negative `( )` field"); 1040 L7 does not. FFFF's
precedent is a leading minus (recon-05 §1.4). The per-(form, line) sign policy — minus vs parens vs
magnitude-into-parens-cell — is undecided, and the geometric read-back has never verified a negative
cell. **Resolution:** per-line sign-policy in the map schema (`neg: minus|parens|magnitude`), KAT a
loss-year 1040 L7 = −3,000 fill + read-back.

---

## 3. IMPORTANT (spec must address)

### G4 — Precedence must be one resolver, everywhere
Recon-04 §5.2 defines the 4-step order **for `report --tax-year`** and leaves mitigation (c)
("make `tax-profile set` refuse when `ReturnInputs` exists") as *consider*. But the profile is also
consumed by `optimize`, `whatif` (defaults), the TUI edit/export panels, and pseudo-reconcile. If
`report` derives from `ReturnInputs` while `optimize` reads the stale hand-entered `TaxProfile`, the
app shows two different liabilities for one year — the cardinal-sin surface (`types.rs:114`).
**Resolution:** one `resolve_profile(year) -> (TaxProfile, Provenance)` function used by every
consumer; provenance printed on every output (report/TUI/PDF footer); `tax-profile set` hard-warns
+ requires `--force` when `ReturnInputs` exists (decide refuse vs force in spec, not later).

### G5 — `W2` struct vs the locked math (internal contradiction)
The recon-04 §2 `W2` struct predates deep/02 and is now inconsistent with it:
- **box 6 (Medicare tax withheld) has no field and is marked OUT (recon-04 §1.1) — yet deep/02's
  locked 8959 Part V (L19 = Σ box 6 → L24 → 1040 L25c) consumes it** and worked Ex.2 depends on it
  (3,370/870 → $180 on 25c). Without box 6 the return under-claims withholding.
- box 4 (SS tax withheld) — needed for G6.
- box 19/20 (local income tax) — SALT 5a input marked "CALC (minor)" but no field.
- owner tag (taxpayer/spouse) — deep/02 C4, known.
**Resolution:** spec the struct with box2/3/4/5/6/7/12/13/17/19 + `owner`; regenerate the TOML
example.

### G6 — Excess-SS credit (the named multi-W-2 edge)
Two employers × one earner over the $168,600 wage base → excess = Σ that person's box 4 −
**$10,453.20** (6.2% × 168,600, TY2024) claimed on **Sch 3 L11 → 1040 L31** (2024 Schedule 3 L11
instructions; Pub 505). Rules the spec must carry: computed **per person** (each spouse separately
on MFJ — never household-pooled); only when **two or more employers** — a single employer
over-withholding is *not* creditable (recover from the employer; i1040 Sch 3 L11); RRTA out of
scope. Inputs exist once G5 lands (box 4 + owner). **Resolution:** wire the computation + Sch 3
L11 + a single-employer-overwithheld refuse-guard; KAT the two-employer case.

### G7 — Which AGI feeds Schedule A (crypto coupling the corpus never states)
The absolute assembly's Schedule A must read **final, with-crypto AGI**: the 7.5% medical floor
(§213(a)), and — decisive for this product — the §170(b) charitable ceilings (30%/50%/60% × AGI,
deep/04 §2d) move dollar-for-dollar with crypto gains; a big LT-gain year *raises* the allowed
crypto-donation deduction. But `derive_tax_profile` (recon-04 §5.1 steps 7–8) computes
`deduction = max(std, schedule_a_total(AGI_noncrypto))` to feed the frozen delta engine, which
cannot re-branch std-vs-itemized per scenario. So the delta path's implied deduction and the
absolute path's actual deduction **diverge** whenever medical/charitable/(2025-SALT-phase-down) is
AGI-sensitive, and `absolute_with − absolute_without ≠ engine delta`. Nothing in the corpus flags
this (deep/02 §4.4 covers only the NIIT analog). **Resolution:** spec states (a) absolute path:
Sch A on with-crypto AGI, full stop; (b) delta path: deduction fixed at derivation time, documented
as approximate; (c) the report labels the two numbers as answering different questions (same pattern
as the $2,242-vs-$1,596 NIIT example).

### G8 — Charitable carryover is consumed even in standard-deduction years
Reg. **§1.170A-10(a)(2)** / Pub 526 "Carryovers": if you take the standard deduction in a carryover
year, you must still **reduce the carryover by the amount that would have been deductible had you
itemized**. deep/04's engine emits carryover-out only from the itemized path; a std-deduction year
that leaves carryover untouched overstates next year's line 13 — a silent multi-year dollar error.
**Resolution:** run the §170(b) ceiling engine every year (even when std wins) solely to age/reduce
the carryover; KAT a std-year-between-two-itemized-years fixture. (Also: spec where carryover-out
persists — deep/04 asks for per-year storage but no home is named; recommend a field on the stored
`ReturnInputs`/side-table row, written back at report time.)

### G9 — Enumerated refuse-guard table for captured-but-inert inputs
"Fail closed on any unmodeled in-scope line" (recon-05 §3.2) is asserted, but no artifact enumerates
the guards. Inputs the model *captures then ignores*, each needing {compute | advisory | refuse}:
- **W-2 box 12 code W** (HSA, incl. employer) → **Form 8889 filing is mandatory** even with no
  deduction (2024 i8889, *Who Must File*). Refuse (or add 8889 later).
- **W-2 box 12 codes A/B/M/N** (uncollected SS/Medicare) → Sch 2 L13; **code Z** (§409A) → Sch 2
  L17h. Additional tax — refuse.
- **W-2 box 10** (dependent-care benefits) → Form 2441 Part III mandatory, else the benefit is
  taxable wages (2024 i2441). Refuse.
- **W-2 box 8** (allocated tips) → income via Form 4137 → 1040 L1c + Sch 2 L5. Refuse.
- **INT box 9 / DIV box 13** (private-activity-bond AMT preference) → couples to G13. Advisory/refuse.
- DIV boxes 2b–2d → the existing Sch-D-Tax-Worksheet refuse-guard (02 §5) — already planned; keep.
**Resolution:** a normative guard table in the spec, one row per captured field, tested by one
KAT per refusing row (input present ⇒ `NotComputable`, never a silent drop).

### G10 — Foreign tax credit, the §904(j) simple path
1099-INT box 6 / 1099-DIV box 7 are common on brokerage statements (international index funds).
§904(j) + Sch 3 L1 instructions: creditable **without Form 1116** when all foreign tax is passive,
1099-reported, and ≤ **$300 ($600 MFJ)**. The model currently doesn't even capture the boxes
(recon-04 marks OUT), silently forfeiting the taxpayer's money. **Resolution:** capture boxes 6–7;
v1 either implements the ≤$300/$600 direct credit (trivial: Σ boxes → Sch 3 L1, refuse above the
ceiling) or refuses when nonzero. Recommend implement — it is a pure sum with a cap.

### G11 — Schedule B Part III has no inputs
2024 Schedule B, Part III header (verbatim): "You must complete this part if you (a) had over
$1,500 of taxable interest or ordinary dividends; (b) had a foreign account; or (c) received a
distribution from … a foreign trust." So *any* return where Sch B is required must answer 7a
(and 7a-ii/7b/8 as applicable) — deep/03 mapped the checkboxes, but `ReturnInputs` has no
`foreign_accounts`/`foreign_trust` fields, so the filler cannot answer them and a blank Part III is
an incomplete schedule. Crypto nuance: FinCEN's current position leaves crypto-only foreign
exchange accounts outside FBAR "reportable accounts" (FinCEN Notice 2020-2 proposed changing this)
— surface as an advisory, don't auto-answer. **Resolution:** add the two booleans (+ country list
for 7b), require them whenever Sch B triggers, advisory on the crypto question.

### G12 — Sch A line 5a composition (double-count trap)
Recon-04 defines `ScheduleA.salt_income_or_sales` as user-entered *and* annotates it "Σ W-2 box17 +
est." while the W-2s separately carry box 17 — two sources for one number. If derivation ever adds
box 17 to the user-entered field, SALT double-counts. Also missing: the 5a **sales-tax election
checkbox** input (Sch A 5a box, mapped in deep/03 as `c1_1`), and box 19 local tax (G5).
**Resolution:** define 5a := Σ W-2 box 17 + Σ W-2 box 19 + `estimated_state_payments` +
`prior_year_state_balance_paid` (new explicit fields), delete the ambiguous catch-all; add
`salt_use_sales_tax: bool`.

### G13 — AMT screening decision
Sch 2 L1 will be hard-zero. The 2024 Schedule 2 instructions provide the "Worksheet To See if You
Should Fill in Form 6251"; commercial tools always run it. Post-TCJA, a capped-SALT W-2 household
essentially never owes AMT, but "essentially never" is not fail-closed. **Resolution:** either
implement the screening worksheet as a refuse-trigger (small, closed-form) or put an explicit
"AMT not evaluated — out of scope" line in LIMITATIONS + a refuse on the known AMT-preference
inputs we *can* see (INT box 9 / DIV box 13, per G9). Decide in spec; don't leave implicit.

### G14 — Fold the round-2/Fable findings into the layered test plan
Recon-05 §2's plan predates every correction. The spec's test plan must add, at minimum:
1. **QDCGT `pref` cap KAT** (F2 F-A: `pref_ws = min(L1, qd+ltcg)`; TI 35,400 / QD 50,000 ⇒ line 16
   = $0, not $446).
2. **Binding-min same-bin KAT** (F2 F-B: L5 = 58,000, QD = 10 ⇒ line 16 = 7,819 via L24).
3. **Per-year bin/bracket-edge alignment assertion** in `method.rs` (F2 §2 caveat: no bracket edge
   < $100k may fall inside a $50 bin; < $3,000 inside a $25 bin) — run for every bundled year.
4. **SALT MFS halve-LAST worksheet KAT** (F1 §2.2: MFS, MAGI 300k, 5d 30k ⇒ 5e = $12,500).
5. **Reduce-to-delta invariant KATs** (deep/02 §4.3; F4 probed 4 regimes — pin all 4).
6. **Sch 2 L4 unbundling KAT** (L4 = ss+medicare only; 0.9% appears once, on L11).
7. **Cross-year map-collision negative test**: fill the 2024 map against the 2025 PDF and assert
   read-back FAILS (the `f1_57` L12↔L1z collision, F5 hazard 1) — proves the safety net actually
   trips.
8. **Filing-status on-state KATs per year** (F5 hazard 2: MFJ `/3`→`/2` etc.) + extend the
   read-back oracle to the 5-way checkbox group (deep/03 watch-item 5 — the current oracle only
   models Yes/No pairs).
9. **Rounding-composition KAT** across forms (8959 L18 cents → Sch 2 L11 → L21 → 1040 L23, per G3).
10. **Loss-year formatting KAT** (1040 L7 = −3,000; Sch 1 8a magnitude-in-parens cell) + Sch D
    L17–L21 fill KATs (gain year and loss year).
11. **Excess-SS KAT** (G6) and **MFS-both-itemize golden** (std = 0 path + box 12b/`c1_8` checked).
12. **L25c composition KAT** (8959 L24 + `other_withholding`).

### G15 — MFS coupling: tri-state + both directions
deep/04 §4 requires fail-loud when the MFS spouse-itemizes fact is unknown, but the recon-04 model
would encode it as a defaulted `bool` (unknowable ≠ false). Also the flag must drive the PDF header
checkbox (2024 combined box `c1_8`; 2025 split 12b/12c per F1 §3.1), not just the std = 0 math.
**Resolution:** `Option<bool>` (or an enum) required for every MFS return; filler asserts
flag ⇒ checkbox.

---

## 4. MINOR (note for the plan)

- **G16 Direct deposit (35b–d), L36, L38.** Decide: omit direct-deposit (paper check) or capture
  routing/account in the vault; add `applied_to_next_year` input (L36 splits L34); leave L38 blank
  (i1040 permits IRS to figure the penalty) — all three are one-line spec decisions + LIMITATIONS
  entries.
- **G17 IP PIN.** The 1040 signature block has an Identity Protection PIN field; an IP-PIN holder's
  paper return is delayed without it. Add optional `ip_pin` (vault-stored, masked). Third-party
  designee: default "No" (no inputs). Phone/email: optional pass-through.
- **G18 Extension payment.** `Payments` lacks Sch 3 L10 (amount paid with Form 4868) → 1040 L31.
  One field.
- **G19 `other_withholding` gate.** Recon-04's escape hatch cites "e.g. W-2G, 1099-R" — income
  sources the return cannot carry. Withholding without its income line is an inconsistent return;
  keep the field but warn loudly (or restrict its docs to genuinely in-scope cases).
- **G20 "Sch D not required" checkbox (1040 L7).** Policy: btctax always files Sch D (crypto engine
  always produces it); box always unchecked; a box-2a-only no-crypto year still files Sch D
  (simpler, valid).
- **G21 Dependent-filer earned income.** deep/04's §63(c)(5) algorithm consumes
  `taxpayer.earned_income` — derive it (Σ W-2 box 1 + net SE − ½SE) rather than adding an input.
- **G22 TI ≤ 0 / carryover edge.** The Capital Loss Carryover Worksheet (Sch D instr.) adjusts
  carryover-out when taxable income goes negative; out of scope for a W-2 household — add a refuse
  (or documented limitation) for TI ≤ 0 rather than trusting the delta-engine carryforward there.
- **G23 Sch B always-file policy.** The >$1,500 threshold is a *may-omit*, not a must-omit; spec
  "always produce Sch B when any interest/dividends exist" — simpler, harmless, and sidesteps
  threshold-edge logic (Part III answers still required, G11).
- **G24 LIMITATIONS entries to pre-draft:** EIC (benefit omitted ⇒ conservative; households likely
  above income limits in v1 scope anyway), 1040-SR (we fill the standard 1040 — permitted at any
  age), no state returns, no 1099-R/SSA income, AMT per G13.
- **G25 Vault/PII.** SSN masking/`--stdin`, PDF-output warnings already flagged (recon-04 §3);
  keep as the implement-phase security-review item — no new gap found.

---

## 5. Checked and CLEAN (probed, no gap found)

- **Computation order** (recon-01 §6 + F3 §4.7 extension): re-walked with Sch 1-A and 8995-L11;
  still a DAG; ACTC's Stage-7 placement (F3) is the only ordering amendment needed for follow-ons.
- **NIIT/Medicare absolute layer** (deep/02 + F4): nothing further — G5's box-6 field is the only
  input-side hole.
- **Method lock** (deep/01 + F2): nothing beyond folding F2's F-A/F-B/F-C/F-E into spec text.
- **TY2025 deltas** (F1/F5): complete for the six-form set; Schedule 1-A PDF extraction is already
  flagged as the known seventh-map TODO (F5), and the 8959/8960 gap (G2) applies to 2025 equally.
- **Precedence design** (recon-04 §5.2/5.3): sound; G4 is about *enforcement breadth*, not the rule.
- **License/legal posture** (05/deep-05): closed; no new exposure found. The Paid-Preparer-blank,
  DRAFT-watermark, attestation-gate decisions all still hold for every new form added under G2.
- **Charitable class model** (deep/04 §5): sufficient once G8's std-year aging is added; the
  ST-crypto-50% correction is consistently propagated everywhere it appears.

---

## 6. Primary sources for the asserted gaps (read/verified this pass)

- 2024 Schedule 1 instructions — *State and Local Income Tax Refund Worksheet* (L1); *IRA Deduction
  Worksheet* (L20); *Student Loan Interest Deduction Worksheet* (L21). IRC §111(a); §221(e)(2);
  Notice 2023-75 (2024 IRA phase-outs).
- 2024 Instructions for Form 8889, *Who Must File* (code W ⇒ 8889 mandatory). 2024 i2441 (box 10 ⇒
  Part III mandatory). Form 4137 / 2024 i1040 line 1c (allocated tips).
- 2024 Schedule 2, lines 11/12 captions ("Attach Form 8959"/"Attach Form 8960"); 2024 Form 1040
  line 13 caption ("Attach Form 8995 or Form 8995-A"); 2024 Schedule D instructions line 21
  ($3,000/$1,500-MFS loss line); Schedule 2 instructions, *Worksheet To See if You Should Fill in
  Form 6251*.
- 2024 Schedule 3 line 11 instructions + Pub 505 (excess SS: per-person, two-or-more employers;
  $10,453.20 = 6.2% × $168,600); IRC §31(b)/§6413(c). §904(j) + Sch 3 L1 instructions (no-1116
  election, $300/$600).
- 2024 Schedule B, Part III header (must-complete condition); FinCEN Notice 2020-2 (crypto/FBAR
  advisory posture).
- Reg. §1.170A-10(a)(2) + Pub 526 *Carryovers* (carryover reduced in standard-deduction years) —
  wording to be re-quoted verbatim at spec time.
- 2024 i1040 p. 23 (round-all-amounts election; add with cents, round the total) — per F2 F-C.
- In-repo: `crates/btctax-forms/src/schedule_d.rs:5-6` (lines 17–22 scope-out), `src/lib.rs:56-58`
  (`fmt_money` = raw Decimal Display), forms dir (no f8959/f8960/f8995 assets),
  `design/full-return/recon/04-input-data-model.md` §1.1/§2 (box 4/6 OUT; undefined Sch1 structs).
