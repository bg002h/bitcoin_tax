# FABLE Independent Review — `SPEC_full_return.md` — Round 2 (re-review of the r1 fold)

**Reviewer:** Fable (independent; author = opus). **Date:** 2026-07-12.
**Target:** `design/SPEC_full_return.md` **r2** (r2 changelog header; fold of
`design/full-return/reviews/SPEC-fable-review-r1.md` — C1, I1–I8, M1–M12).
**Method:** every r1 finding re-verified against the r2 text; fold-introduced regressions hunted; source and
recon spot-verified where the fold is load-bearing: `crates/btctax-core/src/{event.rs,tax/compute.rs,tax/se.rs}`,
`crates/btctax-forms/src/schedule_se.rs`, recon `deep/02` §§3–4, `deep/03` (Sch 1 field grid), `deep/04`
§§2d–2e/3, `fable/06` G11/G12/G23, `01-form-graph.md`.

**VERDICT: NOT GREEN — 0 Critical / 4 Important / 10 Minor.**
The Critical is genuinely resolved: crypto ordinary income now has a real, printed, cross-footing home
(Sch 1 **L8v** → L9 → L10 → 1040 L8 → L9), the hobby-vs-SE split is a classification the ledger actually
makes today (`Income.business: bool`, `event.rs:61`), and SE tax is consistently and completely out of v1.
Seven of the eight Importants are cleanly resolved. What keeps the gate closed: one **fold regression**
(the restored charitable model dropped a term from deep/04's load-bearing 30%-class cap), two **residues**
of r1 findings whose fixes were folded exactly as prescribed but remain incomplete (Sch D path enumeration;
Sch B Part III trigger), and one defect that **survived r1** (the SALT 5a sales-tax election branch). None
is architectural; all four are local normative-text fixes.

---

## 1. CRITICAL

None.

---

## 2. IMPORTANT

### R2-I1 — §4.6 charitable: the 30%-class interaction cap dropped the ordinary-property term (fold regression on r1-I4)

**Location:** §4.6, `tax/charitable.rs` paragraph: "the 30%-capgain class also capped at `50%·AGI − cash`".
**Problem.** deep/04 (lines 189–191), flagged verbatim as "**must be in the spec**": the 30% capital-gain
class is capped at the lesser of (a) 30%·AGI or (b) **50%·AGI − (cash + ordinary-income amounts already
allowed this year)**. The r2 transcription subtracts **cash only**. The dropped term is not exotic in this
product: `OrdinaryProp50` is the **ledger-driven ST-crypto donation class** (§4.6), so any year with both an
ST-crypto and an LT-crypto donation exercises it. Example: AGI $100k, cash $10k, ST-crypto (basis) $35k,
LT-crypto FMV $25k → correct 30%-class room = min(30k, 50k−(10k+35k)) = **$5k**; spec formula = min(30k,
50k−10k) = **$25k** — deduction overstated by $20k → **tax understated**, violating §3.4's own cardinal rule.
**Why.** r1-I4 demanded restoration of deep/04's model; classes, vintages, ordering, expiry, aging, and the
persistence home were all restored faithfully — but this one formula was mis-transcribed in the fold, and it
is exactly the "load-bearing interaction" deep/04 called out.
**Fix.** `30%-class cap = min(30%·AGI, 50%·AGI − (cash + ordinary-income-property amounts allowed this
year))`, and point KAT-13 (or a sibling) at a fixture where the (b) branch binds (the example above
discriminates).

### R2-I2 — §7.2 Schedule D paths are still non-exhaustive: L16 ≥ 0 with L15 < 0 (and L16 = 0) are undefined (r1-I8 residue)

**Location:** §7.2 ("both paths — I8").
**Problem.** The two enumerated branches are "gain year (L16 ≥ 0 **and** L15 ≥ 0)" and "loss year (L16 < 0)".
Two reachable cases match neither: **(a) L16 > 0 ∧ L15 < 0** (net ST gain, net LT loss — a common crypto
year): per the 2024 Sch D, L17 = **No**, lines 18–21 **skipped**, go to **L22** (= Yes iff 3a > 0);
**(b) L16 = 0**: skip 17–21, enter 0 on 1040 L7, go to L22. A literal implementation of §7.2 either has an
uncovered branch or (worse) applies the gain-year arm and prints **L17 = Yes** when line 15 is a loss — a
facially wrong answer on the exact block whose mis-fill was r1-I8.
**Why.** The author folded r1-I8's prescription verbatim; the prescription itself under-enumerated. The gate
is on the artifact, not on fold fidelity, and this block is mandatory on every return the product files.
**Fix.** Spell all four paths: gain (L16>0 ∧ L15≥0 → 17=Yes, 18=19=0, 20=Yes, 21/22 blank); mixed
(L16>0 ∧ L15<0 → 17=No, 18–21 blank, 22=Yes iff 3a>0); zero (L16=0 → 17–21 blank, 1040 L7=0, 22 as above);
loss (as specced). Extend KAT-10 with the mixed-path fixture (ST gain + LT loss).

### R2-I3 — Schedule B is never *forced* by a foreign account/trust: mandatory Part III can be silently omitted (r1-I7 residue)

**Location:** §4 (`foreign_accounts` comment: "required if Sch B files"); §5 stage 1 ("[Sch B if >$1,500 or
forced]"); §9 (advisory).
**Problem.** F6 G11 quotes the 2024 Sch B Part III header verbatim: you must complete Part III if "(a) had
over $1,500 …; **(b) had a foreign account; or (c) received a distribution from … a foreign trust**." The r2
causality runs the wrong way only: the tri-state is required *when Sch B files*, but nothing makes
`foreign_accounts == Some(true)` (or a foreign-trust answer) **force Sch B to file**. A taxpayer with a
foreign account and ≤ $1,500 of interest+dividends gets a return with **no Schedule B at all** — the
mandatory 7a answer silently omitted, on the question with its own penalty regime; that is the same
unknowable-≠-unasked failure r1-I7 targeted, one trigger to the left. (The word "forced" in stage 1 is
undefined; nothing binds it to Part III triggers (b)/(c).) Secondary: `foreign_trust = Some(true)` prints
line 8 = Yes, which points at **Form 3520** — a mandatory attachment v1 cannot produce; §3.4/§7.1 imply
refuse-or-advise, but the spec is silent.
**Why.** Reachable population for this product (accounts on non-US crypto exchanges is exactly the FinCEN
2020-2 advisory audience the spec itself added); produces an incomplete filed return, not a refusal.
**Fix.** One normative line: "Sch B files when interest > $1,500 **or** dividends > $1,500 **or**
`foreign_accounts == Some(true)` **or** `foreign_trust == Some(true)` (Part III triggers (b)/(c)) or
user-forced." Plus: `foreign_trust == Some(true)` with a distribution ⇒ refuse (Form 3520 unbuilt) — add the
§4.10 row; otherwise advisory. KAT the below-threshold foreign-account case.

### R2-I4 — §4.6 SALT 5a composition ignores the §164(b)(5) either/or election (survived r1)

**Location:** §4.6: `salt_use_sales_tax` + "SALT 5a := Σ w2.box17 + Σ w2.box19 + salt_income_or_sales_manual
+ salt_state_estimated_payments + salt_prior_year_balance_paid".
**Problem.** Sch A 5a is **either** state/local income taxes **or** general sales taxes, never both
(§164(b)(5); the checkbox marks the election). The r2 formula is unconditional: with
`salt_use_sales_tax = true`, 5a = sales-table amount **plus** W-2 box 17/19 income-tax withholding **plus**
estimated/prior-year income taxes — box 17 is auto-summed from W-2s, so the user cannot even zero it. 5a is
overstated → itemized deduction overstated → **tax understated** whenever the mixed total is under the $10k
cap (the election population is precisely taxpayers with modest income-tax withholding and a
large-purchase year; in no-income-tax states the formula degenerates correctly, which hides the bug). The
field name (`income_or_sales`) also contradicts its own comment ("sales-tax-table amount ONLY").
**Why.** r1 checked G12 for the box-17 double-count trap and passed it ("no double-count ✓") — the election
branch was never examined; the defect predates the fold but violates the cardinal rule and must not pass the
gate. F6 G12's own resolution formula had the same gap.
**Fix.** Branch the composition: election **off** → 5a = Σbox17 + Σbox19 + estimated + prior-year-balance
(manual sales field must be 0, else refuse); election **on** → 5a = `salt_sales_tax_manual` only (rename the
field), income-tax components excluded, box unchecked/checked accordingly. KAT both.

---

## 3. MINOR

- **R2-M1 (§3.1/KAT-9):** the discriminating rounding fixture is defective twice: (a) arithmetic —
  271.50 + 499.50 = **771.00**, not the quoted "round(770.00)" (expected printed total 772 vs 771 still
  discriminates, but the wrong figure will be copy-pasted into a KAT); (b) envelope — it uses **8959 L13
  (Part II, SE)**, which v1 never fills after the C1 fold. Re-point in-envelope (e.g. two `.50` components
  landing on Sch 3 L10 + L11 → L15, or 25a/25b → 25d).
- **R2-M2 (§1.1 vs §7.1):** §1.1 "Forms filled" lists "Schedule SE (existing, delta-only)" while §7.1 says
  "Schedule SE is **not** filled in v1." Both are true (`btctax-forms/src/schedule_se.rs` is the standalone
  delta report), but the v1-scope list should say "standalone delta report only — not part of the
  full-return fill set" to kill the collision C1 was about.
- **R2-M3 (§3.4 vs §4.10/D-3):** the carve-out lists "foreign-tax **if D-3=refuse**" as a conservative
  *omission*, while §4.10's row and D-3 option 1 define box 6/box 7 presence as a `NotComputable` *refusal*.
  Contradictory semantics for the same decision value (both readings are tax-safe; pick one — omission
  matches the carve-out's logic, refusal matches the table — and align all three sites).
- **R2-M4 (§11):** fold-introduced phase double-assignment: phase 2 builds "§5 stages 1–2" whose stage 1/2
  *include* Sch 1 L1/L7/L8v/L18/L21, yet "Schedule 1 minimal incl. L8v" is phase 4; likewise phase 3 needs
  L13 = QBI for L14/L15→L16 (stage 3c) but QBI/8995 is phase 4. State that phase 2 carries Sch 1 Part I/II
  computation (fillers later) and QBI = 0-stub until phase 4, or reorder (same class as folded r1-M5).
- **R2-M5 (r1-M4 residue):** the G20 policy — 1040 L7 "Schedule D not required" checkbox **always
  unchecked** (v1 always produces Sch D) — is still stated nowhere.
- **R2-M6 (§4.6):** the ledger auto-classes crypto donations as 50%-org (`CapGainProp30`/`OrdinaryProp50`);
  deep/04 (lines 218–219) carries the explicit caveat that a private-foundation donee would be 20%-class at
  basis. Now that the class picks the *ceiling* (not just the 8283 amount), restate the public-charity-donee
  assumption as an advisory + §9.2 LIMITATIONS line.
- **R2-M7 (§5 stage 7):** the NII total is pinned but the **printed 8960 line** carrying non-SE crypto
  lending interest is unassigned (L1 "taxable interest" would then ≠ 1040 2b; L7 "other modifications" is
  the alternative). Name the line so the §7.3 map extraction has a target.
- **R2-M8 (§4 persistence):** the write-back sentence mixes two mechanisms ("staging field on the side-table
  row" *and* "writes year Y's out into year Y+1's in") and is silent on precedence when Y+1's
  `carryover_in` was user-entered (overwrite? refuse? provenance flag?). r1-I4's "name the home" is
  satisfied; pin the overwrite/provenance semantics (one sentence) so the plan doesn't guess.
- **R2-M9 (§4.7):** dependent earned income "Σ box1 + **net SE − ½SE**" retains a dead SE term post-C1 (no
  SE income can exist on a computable v1 return). Harmless (evaluates 0) but it is the last dangling SE
  reference in v1 normative math — mark it "0 in v1 / SE follow-on."
- **R2-M10 (§1.2/§9.2):** Schedule 2 **L2 excess-APTC repayment (Form 8962)** is absent from both
  out-of-scope enumerations. It is undetectable from modeled inputs (no 1095-A capture) and prints as 0 —
  the attestation/LIMITATIONS shield is the design for this class, so it must be *on the enumerated list*
  (recon-01 scoped 8962 out explicitly). While there: recon-01 (lines 46/91/266) has Sch 2 **L1/L2 swapped**
  (AMT is L1, APTC is L2 on the real 2024 form); the spec's "Sch 2 L1 = 0" in §4.11 is **correct** — record
  the recon erratum so nobody "fixes" the spec backwards.

---

## 4. r1 findings — resolution audit (finding by finding)

| r1 | Status in r2 | Evidence |
|---|---|---|
| **C1** crypto ordinary income | **RESOLVED** (residuals → R2-M1/M2/M9) | L8v home real & printable (2024 Sch 1 8v = `f1_33`, deep/03:174); income cross-foots (§5 stage 1: L8v → Sch 1 L9→L10 → 1040 L8; L9 = 1a+2b+3b+7+8); refuse-guard row is §4.10 row 1; SE moved out consistently (§1.1 8959-Part-I-only, §1.2, §5 stage 2 "(½-SE … follow-on)", stage 7 "(SE tax / Sch 2 L4 / 8959 Part II = SE follow-on)", §7.1, §11, D-6); KAT-5 re-sourced (income-free/hobby), KAT-15 added, golden matrix has "crypto hobby income". **Buildability verified in source:** per-event `business: bool` (`event.rs:61`) is the ledger classification; `crypto_ord` (`compute.rs:300-305`) filterable by it; SE eligibility = `business && kind != Interest` (`se.rs:59`); lending-interest NII isolated (`compute.rs:310-315`) so "non-SE crypto NII" is exactly the engine's `interest_nii` — no double-count with 2b (INT box1/box3 only) or Sch D (FMV becomes basis). D-6(a)/(b)/(c) correctly frames the user decision. |
| **I1** rounding composition | **RESOLVED** (fixture defect → R2-M1) | §3.1 now states one rule: worksheets carry cents; printed lines rounded at the line; **printed totals sum the printed (rounded) values** — the cross-footing reading, exactly as recommended. |
| **I2** dependents refuse-vs-omit | **RESOLVED** (D-3 example wrinkle → R2-M3) | §3.4 carve-out (taxpayer-favorable-only, advisory + LIMITATIONS + KAT); §1.2 corrected ("omitted with an advisory, not refused"); §5 stage 6 L19 = 0 + advisory; §4.2 dependents captured-not-computed. Internally consistent: carve-out explicitly overrides the base rule. |
| **I3** L20 IRA condition | **RESOLVED** | §4.4 "refuse **iff** `ira_contribution > 0`"; §4.1 box-13 comment ("load-bearing only when the IRA worksheet ships"); §4.10 row "L20-with-deduction"; D-2 "(I3 rewording applied)". |
| **I4** charitable model | **PARTIAL → R2-I1** | 6-class enum matches deep/04's corrected table exactly (Cash60/Cash30/CapGainProp30/CapGainProp20/OrdinaryProp50/OrdinaryProp30 — no phantom 50% cash class); class+vintage `CharitableCarryItem` with `origin_year`; oldest-first, 5-yr expiry, G8 aging in std years; ledger §170(e) supply by holding period matches deep/04 §2e; persistence home named (§4). The one dropped term in the 30%-class cap is R2-I1. |
| **I5** blind / can-be-claimed | **RESOLVED** | `Person.blind` explicit ("not DOB-derivable"); `can_be_claimed_as_dependent_{taxpayer,spouse}`; wired into §4.7 math and the deep/03 header-checkbox rows. |
| **I6** 1099-G | **RESOLVED** | `Form1099G { box1, box4 }` (§4.3); L7 derived (§4.4); 25b = Σ(INT+DIV+**G** box4) (§4.8, §5 stage 8). |
| **I7** Sch B Part III | **PARTIAL → R2-I3** | Tri-state `Option<bool>` + fail-loud ✓; `foreign_country_names` for 7b ✓; FinCEN 2020-2 advisory restored (§9) ✓; the force-file trigger is the residue. |
| **I8** Sch D L17–22 | **PARTIAL → R2-I2** | Gain and loss paths now correct as written (L17=Yes/18=19=0/20=Yes; loss: 17–20 skipped, L21 = −min(|L16|, 3000/1500), L22 = Yes iff 3a>0) and KAT'd; enumeration non-exhaustive is the residue. |
| **M1** | folded ✓ | per-employer clamp formula + "any single employer's box4 > MAX ⇒ refuse" (§4.9, §4.10, KAT-11). |
| **M2** | folded ✓ | 8960 L5a = "the §1211-limited figure that reaches 1040 L7"; 8959 Part V floor `max(0, Σbox6 − 1.45%·Σbox5)` (§5 stage 7). ("Part V" verified correct — deep/02:210-211: Part IV = L18, Part V = reconciliation.) |
| **M3** | folded ✓ | §4.7: 2024 = combined `c1_8`; 12b/12c noted as TY2025. |
| **M4** | mostly folded | L31 = Sch 3 L15 ✓ (§4.8, stage 8); L35a/L36 ✓; direct-deposit omitted, paper check, LIMITATIONS ✓ (stage 9); **G20 checkbox policy still missing → R2-M5**. |
| **M5** | folded ✓ (new residue → R2-M4) | phase 2 = std-basic only; refuse-guards pulled into phase 1. |
| **M6** | folded ✓ | ATS Scenario-2 partial-line-diff caveat (§10). |
| **M7** | folded ✓ | KAT-13 (G8 std-year fixture) + KAT-14 (AMT-screen trigger). |
| **M8** | folded ✓ | `mortgage_interest_1098`, 8a-only, 8b refuse-or-advise + $750k/$1M advisory. |
| **M9** | moot ✓ | SE out of v1; owner comment "(SE cap moot in v1)". |
| **M10** | folded ✓ | D-3 states §904(j) conditions (passive + 1099-reported, refuse above $300/$600); §4.10 row references them. |
| **M11** | folded ✓ | 8960-Part-II-=-0 conservatism recorded in §9.2. |
| **M12** | folded ✓ | Sch B trigger stated ("> $1,500 or forced"); >14/>15-payer overflow via 8949 continuation pattern (§7.4). |

## 5. Re-checked and found CLEAN (beyond the table)

- **No dangling SE wiring in v1** except the dead §4.7 term (R2-M9): stage 7's explicit exclusion note, 8959
  Part-I-only everywhere, Schedule SE out of the §7.1 fill set, phases SE-free, reduce-to-delta re-based on
  income-free/hobby fixtures — and the invariant still holds (hobby income ⇒ 8959 delta 0 = absolute Part I
  with no wages; `interest_nii` appears identically on both sides).
- **§5 line plumbing re-verified against the real 2024 forms:** Sch 1 L9/L10 → 1040 L8; Sch 1 L26 → 1040
  L10; Sch 3 L8 → L20; Sch 3 L15 → L31; Sch 2 L3 → L17; Sch 2 L21 → L23; L34/L35a/L36/L37/L38. All correct
  (including surviving recon-01's swapped Sch 2 L1/L2 — see R2-M10 erratum note).
- **§3.1/§3.2/§3.3** unchanged where r1 passed them; §4.5 QBI (box5 ⊂ 3b, no income double-count, threshold
  refuse); §4.9 constants ($10,453.20 = 6.2%×168,600); §8 TY2024 std-deduction set and bin-alignment
  assertion; §4.11 AMT screen; §4.12 resolver/provenance/D-4; §6 delta-vs-absolute three-part statement;
  §9 legal posture incl. restored FBAR advisory; §10 layer plan + KAT set (1–15 + per-row) coherent with the
  C1 re-sourcing; §12 D-1..D-6 coherent (D-6 correctly framed as the user's call, rec (a) matches the body).

## 6. Disposition

**Gate does not pass: 0 Critical / 4 Important / 10 Minor.** The r1 fold was high-fidelity — C1 and
I1/I2/I3/I5/I6 fully land, and the C1 resolution is verified buildable against current source. The four
Importants are all single-site normative-text fixes (one dropped formula term, two missing enumeration
branches/triggers, one missing either/or election branch); no decision reshaping is required. Re-review (r3)
after the fold per `STANDARD_WORKFLOW.md` §2 — given the findings' locality, r3 should be quick.
