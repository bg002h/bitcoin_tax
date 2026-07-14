# FABLE Independent Review — `SPEC_full_return.md` — Round 1

**Reviewer:** Fable (independent; author = opus). **Date:** 2026-07-12.
**Target:** `design/SPEC_full_return.md` (v1 full return, Common W-2 household, TY2024, PDF-only, offline).
**Inputs read in full:** recon `00`–`05`, `deep/01`–`05` (04 in full; 03/05 for map/test claims), `fable/00`–`06`.
**Source spot-verified:** `crates/btctax-core/src/tax/{compute,se,tables,types,conventions}.rs`,
`crates/btctax-forms/src/{schedule_d,form1040,verify,lib}.rs`, `crates/btctax-forms/forms/{2024,2025}/`,
`crates/btctax-adapters/src/tax_tables.rs`, `crates/btctax-cli/src/tax_profile.rs`.

**VERDICT: NOT GREEN — 1 Critical / 8 Important / 12 Minor.** The spec is strong on everything the recon
corpus explicitly enumerated: the three F6 blockers are each addressed with real mechanisms, every Fable
correction I traced is folded faithfully, and all 12 IMPORTANT gaps are at least touched. The Critical
finding is a hole the corpus itself never surfaced: the absolute income assembly has **no line home for
crypto ordinary income** while the spec simultaneously keeps SE tax / ½-SE / 8959-Part-II in v1 scope —
as written it would print a 1040 that either understates total income or fails to cross-foot. Several
Important findings are places where the spec's normative text, taken literally, misbehaves (L20 IRA
refuse condition, Sch D L20 in loss years, dependents-vs-fail-closed contradiction) or where the data
model silently regressed below what deep/04 proved necessary (charitable classes/vintages, blind flag).

---

## 1. CRITICAL

### C1 — Crypto **ordinary income** has no home on the absolute return, but SE tax stays in scope (§1.1, §4.4, §5 stages 1–2/7)

**Location:** §5 stage 1 (lines 282–285), §4.4 table (lines 179–193), §1.1/§1.2 (lines 20–42), §11 phase 2.

**Problem.** The engine produces crypto **ordinary income** — `IncomeKind::{Mining, Staking, Interest,
Airdrop, Reward}`, hobby or business (`compute.rs:300-315`, `event.rs:33`) — and the spec keeps its tax
consequences in v1: §1.1 lists "SE tax (existing)"; §5 stage 2 wires ½-SE → Sch 1 L15; stage 7 wires
Sch SE L12 → Sch 2 L4 and `se.rs.addl` → 8959 Part II; stage 7's NII adds "+ crypto" (which per deep/02
§2.2 includes crypto-lending interest). But **§5 stage 1 gives that income no 1040 line**: total income
L9 = 1a + 2a/2b + 3a/3b + 7 + 8, where 2b = Σ(1099-INT box1+box3) only, 7 = Schedule D only, and 8 =
the §4.4 enumerated Schedule 1 surface — which contains **no line for crypto income** (L3 Sch C and L8x
are in the "no input; refuse" row). On a real return: business mining → **Schedule C → Sch 1 L3**
("Attach Schedule C" — a mandatory attachment, exactly the G2 closure class; Schedule SE Part I itself
reads Sch C net profit); hobby mining/staking/airdrops → **Sch 1 L8v** (2024 "Digital assets received as
ordinary income not reported elsewhere"); lending interest → 2b/Sch B. None is modeled; no Sch C filler
exists.

**Why it matters (wrong return, not a refusal).** Stage 2 pins "AGI (L11) = with-crypto AGI ★". The
with-crypto AGI *includes* `crypto_ord` (`compute.rs:364-366`), so either (a) the assembly injects
crypto_ord into AGI without any income line carrying it — the printed 1040 then fails its own arithmetic
(L11 ≠ L9 − L10), an immediate IRS cross-foot flag — or (b) the assembly builds AGI from the printed
lines and **silently understates income** by the entire crypto ordinary amount while still charging SE
tax on it. Both outcomes violate §3.4's own cardinal rule. The spec's test plan is self-inconsistent
with the hole: KAT-9's 768.69 figure **is** deep/02 Ex.2 — a household with $60,000 of Schedule C mining
income the spec's income model cannot represent — and the reduce-to-delta KAT regimes (deep/02 §4.3 /
F4 §2) include lending-interest and large-crypto-ordinary-income cases.

**Recommended fix (a decision the spec must make, either way is buildable):**
1. **Narrow v1:** add a refuse-guard row "any `income_recognized` in the tax year ⇒ `NotComputable` for
   the full-return path" (crypto = capital gains + donations only in v1); delete "SE tax (existing)" from
   §1.1/§5 stages 2/7 and 8959 Part II's SE side (Part I wages-only remains); re-source KAT-9 and the
   reduce-to-delta KATs to income-free fixtures. The standalone SE report (delta path) is untouched.
2. **Or close the set:** enumerate Sch 1 **L8v** (derived from the ledger's hobby income — no user
   re-typing) and a **minimal Schedule C** (gross = business income, expenses = existing
   `schedule_c_expenses`, net → Sch 1 L3) + Sch C filler/map, keeping Sch SE/8959 wiring as specced.
   This is real new scope (one more form) and must be priced into §11/D-5.

Option 1 is smaller and honest; option 2 matches the product's crypto center of gravity. Undecided is
unshippable — this is precisely a G1/G2-class blocker that survived F6.

---

## 2. IMPORTANT

### I1 — §3.1 rounding: the composition rule for **filed-form internal totals** is still ambiguous (the G3 residue)

**Location:** §3.1 (lines 82–88); KAT-9 (§10 line 390).
"Every form-line value is `round_dollar`ed at the line" and "within a single IRS worksheet, carry cents
and round once at the line that lands on a form" conflict for **filed multi-line forms** (8959, 8960,
Sch A, Sch 2, the 1040 spine): every line of Form 8959 *is* a form line, so is L18 the sum of the
**printed (rounded)** L7/L13/L17, or `round_dollar` of the **cent** sum? The readings diverge by $1 in
reachable cases (e.g. L7 = x.50 and L13 = y.50: 271+499 = 770 vs round(769.00) = 769), and reading (ii)
can print a form that visibly fails to cross-foot (L7+L13 ≠ L18 on paper). F2 F-E blessed
carry-cents-once **only for the unprinted QDCGT worksheet**; extending it silently to printed forms is a
new, undecided choice. The chosen KAT (768.69 → 769) does not discriminate — both readings pass. Also:
quoting "8959 L18 (768.69)" imports deep/02's cent figures into a whole-dollar pipeline without saying
what the printed L13/L18 cells actually contain.
**Fix:** state one rule — recommend: *unprinted worksheets carry cents; printed form lines are rounded at
the line, and printed totals sum the printed line values* (cross-footing, the FFFF/commercial reading) —
and replace/augment KAT-9 with a discriminating fixture (two .50-cent components).

### I2 — Dependents/CTC: §1.2+§3.4 (refuse) contradicts §5 stage 6 (L19 = 0) — decide which

**Location:** §1.2 (lines 39–42), §3.4 (lines 104–107), §4.2 (line 166), §5 stage 6 (line 298), §4.10.
§1.2 says the fail-closed rule turns out-of-scope credits "into a refusal"; §3.4 says any
**captured-but-unmodeled input that would change the return** produces `NotComputable`. Dependents are
captured (§4.2) and a DOB-derived qualifying child changes the return (L19/L28) — so by the spec's own
normative rule, **every return with a dependent refuses** (ODC makes it *any* dependent). Yet §5 stage 6
prints "L19=0 (CTC out)", and §4.10 has no dependent row. For the flagship "Common W-2 household" this
is the difference between "v1 works for families (forfeits the credit, documented)" and "v1 refuses all
families." **Fix:** pick one: (a) a refuse-guard row for CTC/ODC-eligible dependents (harsh but pure), or
(b) an explicit carve-out in §3.4 for *conservative benefit omissions* (taxpayer-unfavorable only:
CTC/ODC, EIC — the G24 pattern), with an advisory + LIMITATIONS entry and a KAT pinning L19 = 0 +
advisory. (b) is recommended and matches the EIC precedent; either way §1.2's "turns each of these into
a refusal" must be corrected to match.

### I3 — §4.4 L20 IRA refuse condition, as written, refuses every 401(k) household

**Location:** §4.4 table, L20 row (line 191); §4.1 box13 comment (line 154); D-2 (line 419–420).
"Refuse when `box13_retirement_plan` is set OR `ira_contribution > 0`" — box 13 is checked for anyone
with a workplace retirement plan (the majority of the target market). Read literally, box13-set alone
(no IRA contribution at all) makes the return `NotComputable`. That is certainly not intended (box 13
with no IRA deduction claimed changes nothing on the return), but the spec is the normative text and a
plan will inherit it. **Fix:** "refuse iff `ira_contribution > 0`" (the phase-out worksheet is unmodeled,
so *any* claimed IRA deduction refuses in v1); note box 13 becomes load-bearing only when the worksheet
is implemented in a follow-on. D-2's wording ("refuse L20") should match.

### I4 — Charitable inputs/carryover regressed below deep/04's proven-necessary model

**Location:** §4 `charitable_carryover_in: [Usd; 4]` (line 132); §4.6 `charitable_cash` /
`charitable_noncash_non_crypto` (lines 214–216).
deep/04 CORRECTION 3 + §5 established that flat charitable fields are **insufficient**: non-cash spans
two different ceilings (LT cap-gain = 30% FMV vs ordinary/basis property = 50%), the classes consume AGI
room in a fixed order, and carryover must retain **class + vintage** (5-year expiry, §170(d)(1);
deep/04 recommends `Vec<CarryoverItem { class, amount, origin_year }>`). The spec regresses to exactly
the flat shape deep/04 corrected: `charitable_noncash_non_crypto` has no class (the engine cannot place
it at 30% vs 50%), `charitable_cash` is annotated "60%/50% class" (cash to a public charity is 60%-only;
cash to a non-50%-org is 30% — there is no 50% cash class), and `[Usd; 4]` (a) never says which 4 of
deep/04's 5 classes it holds and (b) has no origin-year, so the 5-year expiry and oldest-first
consumption are unimplementable and the G8 "age even in std years" output cannot be persisted correctly.
G8's second half — **where carryover-out persists** (F6 recommends a field on the stored `ReturnInputs`
row, written back at report time) — is also unaddressed (same question applies to the QBI L16/L17
carryforwards, §4.5). **Fix:** adopt deep/04 §5's classified inputs (or collapse with an explicitly
conservative class default, stated) and a class+vintage carryover type; name the persistence home.

### I5 — Missing header inputs: `blind` and `can_be_claimed_as_dependent` (the spec's own §4.7 math needs them)

**Location:** §4.2 `Person` (lines 163–165); §4.7 (lines 227–230).
§4.7 computes "basic + §63(f) aged/blind (DOB-derived) + dependent floor" — but **blindness is not
derivable from a DOB**, and no `blind: bool` exists anywhere in the model; likewise the §63(c)(5)
dependent-filer path §4.7 specs (with the G21 earned-income derivation) has no "someone can claim
you/spouse as a dependent" flag. Both are required inputs per deep/04 §1.2 and both drive 1040 header
checkboxes the filler must set. As written the blind bump is unclaimable and its checkbox unfillable.
**Fix:** add `blind: bool` to `Person` (taxpayer + spouse) and `can_be_claimed_as_dependent` flags to the
header; wire to the §4.7 formula and the 2024 header checkbox map rows (deep/03: `c1_6/c1_7/c1_9–c1_12`).

### I6 — Unemployment modeled as a bare scalar loses 1099-G box 4 withholding

**Location:** §4.4 L7 row (line 188); §4.8 (lines 231–234).
F6 G1's recommended resolution was "L7 via a 1099-G struct." The spec captures `unemployment: Usd` only.
Federal withholding on unemployment (1099-G box 4, the 10% voluntary withholding — common) then has no
25b path: §4.8 defines 25b = Σ 1099-**INT/DIV** box 4 only, so the user must either drop real
withholding or stuff it into `other_withholding` → **25c**, misplacing a Form-1099 withholding amount
that belongs on 25b. **Fix:** `Form1099G { payer, box1_unemployment, box4_fed_withheld }`; 25b = Σ INT
box4 + DIV box4 + G box4.

### I7 — Schedule B Part III: defaulted `bool`s silently answer the foreign-account question "No"

**Location:** §4 (lines 133–135); §5 stage 1 (line 283).
`foreign_accounts: bool` / `foreign_trust: bool` with `#[serde(default)]` means a user who never
answered gets **"No" auto-checked on 7a/8** whenever Sch B files — a silent assertion on the FBAR
question, whose wrongness carries its own penalty regime. This is the exact unknowable-≠-false defect
G15 fixed for MFS, applied to G11; F6 G11 also required the FinCEN-2020-2 crypto-account **advisory**
(crypto-only foreign exchange accounts currently outside FBAR "reportable accounts"), which the spec
drops. **Fix:** tri-state (`Option<bool>`, required whenever Sch B is produced — fail loud when `None`);
add the crypto/FBAR advisory text to §4/§9.2.

### I8 — §7.2 Sch D L17–22 normative text is wrong for loss years ("L20 = Yes" is unconditional)

**Location:** §7.2 (lines 335–339).
On the 2024 Schedule D: **if line 16 is a loss, lines 17–20 are skipped** (L17 "are 15 and 16 both
gains?" = No/blank; L18/L19 blank; L20 not answered), L21 takes the §1211-limited loss, and **L22**
("do you have qualified dividends?") selects the QDCGT path. The spec's "L18/L19 = 0; L20 = Yes"
describes only the gain-year path but is stated unconditionally, and L17/L22 semantics are never given
despite "lines 17–22" being the scope. A literal implementation checks "Yes" on a skipped line in every
loss year — a facially wrong filed schedule, on the exact line-block whose omission was blocker G2.
**Fix:** spell out both paths (gain year: L17 = Yes, L18 = L19 = 0, L20 = Yes; loss year: 17–20
blank/skipped, L21 = min(|L16|, 3000/1500), L22 = Yes iff 3a > 0) and point the existing gain/loss KATs
at them.

---

## 3. MINOR

- **M1 (§4.9/§4.10):** excess-SS needs the per-employer clamp — credit = max(0, Σ min(box4ᵢ, $10,453.20)
  − $10,453.20); as written, one employer over-withholding *plus* a second employer over-claims the
  credit. Disambiguate the refuse row to "any single employer's box 4 > $10,453.20 ⇒ refuse" (the Pub 505
  worksheet's per-employer cap), which also covers the mixed case.
- **M2 (§5 stage 7):** 8960 NII component "Sch D net" should read "the §1211-limited figure that reaches
  1040 L7" (8960 L5a ← 1040 L7, deep/02 §2.2); "Sch D net" invites using L16 (full loss) in a loss year.
  Also 8959 Part V: state the floor — L22 = **max(0,** L19 − 1.45%·Σbox5**)** — the spec's formula can go
  negative on an under-withheld box 6.
- **M3 (§4.7):** "the filler checks 1040 box **12b (2024)**" — 12b is the **2025** designation (F1 §3.1
  split); 2024 is the combined header checkbox (`c1_8`, deep/03). Fix the year label so the map row is
  keyed correctly.
- **M4 (§5 stages 8–9):** L31 reads **Sch 3 L15** (which includes the §4.8 extension payment on L10), not
  L11 alone; stage 9 omits L35a (refund = L34 − L36) and the G16 direct-deposit decision (recommend:
  omit 35b–d, paper check, LIMITATIONS entry); G20's "Sch D not required box always unchecked" policy is
  nowhere stated.
- **M5 (§11):** phase 2 spans "§5 stages 1–4" incl. the deduction (stage 3) but the deduction machinery is
  phase 3; also the refuse-guard table lands at phase 5, after phases 2–4 can already compute — state
  phase 2 uses std-deduction-basic only (or reorder), and consider pulling the guards into phase 1's
  input layer so intermediate phases can't silently mis-compute.
- **M6 (§10 L3):** ATS Scenario 2 contains out-of-scope forms (Sch 8812, 8863, 4972, 8867 — recon-05
  §2.3's own caveat), so v1 cannot reproduce it end-to-end; the spec must carry the "partial-line diff"
  caveat or pick a scenario inside the v1 envelope.
- **M7 (§10):** two KATs the corpus asked for are unnamed: the G8 std-year-between-two-itemized-years
  carryover fixture (F6 §3 G8) and an AMT-screen refuse-trigger KAT (§4.11 adds a computed screen; only
  the box-9/box-13 rows get KATs via §4.10).
- **M8 (§4.6):** single `mortgage_interest` labeled "8a/8b" — 8b (not on 1098) requires payer
  name/address write-ins the model doesn't carry; recommend v1 = 8a-only (matches §1.1 "user-entered from
  Form 1098") + refuse/advise for 8b amounts.
- **M9 (§4.12/§2):** the derived profile's `w2_ss_wages` must be the **SE-earner's own** box 3 (deep/02
  C4) — the spec never says how the app knows which spouse owns the crypto ledger/SE activity (input or
  convention needed). Evaporates if C1 resolves by refusing SE income in v1.
- **M10 (§4.4/§4.10):** if D-3 = implement, the §904(j) election also requires all foreign tax be
  passive-category and 1099-reported — state the conditions and refuse above $300/$600 (F6 G10 wording),
  so the guard table row has testable semantics.
- **M11 (§5 stage 7 / §9.2):** record F4's hardening (c) — Form 8960 Part II = 0 is a deliberate,
  conservative simplification (can only overstate NIIT) — in the spec/LIMITATIONS text.
- **M12 (§4):** G23 deviation is silent — the spec files Sch B threshold-based ("if >$1,500 or forced")
  where F6 recommended always-file-when-any-interest/dividends; fine either way, but state the choice
  (it interacts with I7's "required when Sch B is produced" trigger). Sch B >14/>15-payer grid overflow
  (deep/03 watch-item 3) also deserves one line.

---

## 4. Checked and found CLEAN (explicit no-findings list)

- **G3 core (§3.1/§3.2):** `round_dollar` = `MidpointAwayFromZero`, distinct from `round_cents`
  (`conventions.rs:13` verified half-even; no `round_dollar` exists yet ✓); p. 23 citation (F2 F-C
  folded ✓); global round-all-amounts election; per-(form,line) `neg: minus|parens|magnitude` sign
  policy + negative-cell read-back extension + loss-year 1040 L7 KAT. Sound apart from I1's
  filed-form-total residue.
- **G1 structure (§4.4):** closed enumerated struct (F4 hardening (a) honored — no signed catch-all);
  L1 attest+advisory, L15/L18 derived, L21 full worksheet with correct TY2024 numbers ($2,500;
  80–95k/165–195k; MFS = $0 per §221(e)(2); AGI-without-this-deduction circularity note) — all match
  F6/G1's recommended minimum apart from I3/I6.
- **G2 closure (§7):** 8959/8960/8995 fillers scheduled (verified absent today: no `f8959/f8960/f8995`
  in `btctax-forms/src` or `forms/2024`); "no non-DRAFT return with an unbacked line" refuse; QBI
  override forces the 8995 map. `schedule_d.rs:5-6` scope-out claim verified verbatim.
- **Correction fidelity:** F2 F-A pref cap (`pref_ws = min(TI, qd+ltcg)`, §5 stage 4) ✓; F2 F-B
  binding-min kept + KAT-2 ✓; deep/02 C2 NII rebuilt from line items (verified `nii_with` at
  `compute.rs:359` omits household interest/non-qualified dividends) ✓; C4 owner tag + per-earner box 3
  (§4.1) ✓; C5 Sch 2 L4 = ss+medicare only (verified `se.rs:160` `total` bundles `addl`;
  `deductible_half` excludes it) + KAT-6 ✓; G5 box 4/6/19 present (box 6 → Part V → 25c) ✓; G7
  with-crypto AGI feeds Sch A + §6's three-part delta/absolute statement ✓; G8 aging-in-std-years stated
  ✓ (persistence = I4); F1 SALT-halve-last correctly **parked** as TY2025 follow-on (TY2024 flat cap —
  right call) with KAT-4 labeled follow-on ✓; F3 DOB-not-booleans + ssn-validity ✓; F5 map hazards
  (f1_57 L12↔L1z, on-state reassignment, root flips, 5-way checkbox oracle — verified `verify.rs` today
  models only Yes/No pairs via `topmost_yes_no_pair`) ✓.
- **Gap walk G4–G15:** G4 one-resolver + provenance + D-4 guard ✓; G6 per-person/≥2-employer/$10,453.20
  (= 6.2%×168,600 ✓) → Sch 3 L11 ✓ (M1 clamp aside); G9 guard table row-for-row matches F6 incl. Z→Sch 2
  L17h, box 8→4137, box 10→2441, TI≤0 (G22) ✓; G10 surfaced as D-3 with boxes captured ✓; G11 fields
  exist (I7 tri-state aside); G12 5a composition + sales-tax election box, no double-count ✓; G13 the
  6251 screening worksheet as refuse-trigger ✓; G14: all 12 F6 KATs present in §10, one KAT per refuse
  row, golden branch matrix ✓; G15 `Option<bool>` + both directions ✓ (M3 label aside). Minors: G16–G25
  mostly covered (M4/M12 residue).
- **Math/law spot-checks:** TY2024 std-deduction set ($14,600/29,200/21,900; 1,550/1,950; 1,300/+450) ✓
  deep/04; 8995 thresholds 191,950/383,900 ✓ F3; QDCGT stage-4 mapping and the DAG claim (Sch A reads
  AGI, 8995 reads TI-before-QBI) ✓ recon-01 §6; reduce-to-delta invariant wording ✓ deep/02 §4.3/F4 §2;
  AMT screen scope ✓; §8 per-year bin/bracket-edge assertion = F2 §2 caveat converted to an invariant ✓;
  "no per-year Tax-Table data" ✓ F2 REFINEMENT 2.
- **Code claims:** `types.rs:114` cardinal-sin cite ✓; `tax_profile.rs` `year INTEGER PRIMARY KEY`
  side-table mirror ✓; `ty2024()` breakpoints/wage base in `tax_tables.rs` ✓; `fmt_money` = raw Decimal
  Display (`lib.rs:56`) ✓; frozen-engine feasibility: the derived `TaxProfile` feeds `compute.rs`
  unchanged — the additive seam is real (subject to C1's income-side decision).
- **§9 legal posture:** DRAFT+attestation forced, PTIN blank, LIMITATIONS doc, permissive+distributable
  with CC0 Tax-Calculator CI cross-check and observe-only oracles — faithful to 00-SYNTHESIS §6 and
  deep/05. **§12 decisions:** D-1..D-5 are the right forks to surface; recommendations defensible
  (D-3's tension with "no credits" is honestly flagged; D-2 needs I3's rewording).

---

## 5. Disposition

Gate does not pass: **1 Critical / 8 Important / 12 Minor.** C1 requires an author decision
(narrow-or-close) that reshapes §1.1/§4.4/§5/§10/§11 consistently; I1–I8 are each locally fixable in the
spec text. Re-review required after the fold per `STANDARD_WORKFLOW.md` §2.
