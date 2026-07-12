# FABLE CONFIRMATION REVIEW — audit fold (SPEC r5 + PLAN r3)

**Reviewer:** Fable, focused confirmation pass. **Date:** 2026-07-12.
**Mandate:** confirm the final whole-design audit's 1 Critical + 3 Important + 11 Minor
(`DESIGN-fable-audit-final.md`) are genuinely resolved in SPEC r5 + PLAN r3, with no regression.
Gate: 0 Critical / 0 Important ⇒ sound to begin implementation.

**VERDICT: NOT yet 0C/0I — 0 Critical / 3 Important / 6 Minor.** The four blocking audit items were
substantially folded, but the fold itself introduced one regression (F1: the M1 "fix" inverted the real
2024 Schedule 2 Part I layout — verified against the IRS PDF) and left two residues of the originals
(F2: the kiddie-tax unearned-income predicate is under-inclusive and misses the app's flagship income
type; F3: the box-12 allowlist admits over-402(g)-limit elective deferrals — a silent understatement in
a core multi-W-2 household). All three are small, localized text edits (spec §5/§4.10/§4.11/§9.2 + plan
P1 t4); none touches the architecture, the frozen seam, the pipeline DAG, or the phase structure.
**NOT sound-to-build until F1–F3 are folded and re-confirmed.**

---

## 1. Audit-item confirmations

### C1 (Form 8615 kiddie tax) — SUBSTANTIALLY RESOLVED; one residue → F2

Verified present and correctly stated at every required site:
- §1.2:71 — out-of-scope list: "Form 8615 kiddie tax (dependent filer + unearned income > $2,600 — refuse, C1)". ✓
- §4.10:310 — refuse row: "claimable-as-dependent filer + unearned income > $2,600 (TY2024, §1(g))",
  rationale names the understatement direction (`qdcgt_line16` at the child's rate vs parent's-rate
  required). ✓
- §8:449 — "§1(g) kiddie-tax unearned-income threshold $2,600 (TY2024, indexed — C1)" bundled in
  `TaxTable`. Threshold verified: Rev. Proc. 2023-34 §3.16 — TY2024 8615 trigger = $2,600 (2× the
  $1,300 dependent floor already bundled). ✓
- §10 KAT-19:493 ✓; PLAN P1 task 4:96-97 owns KAT-19 and cites the year-table threshold [Minor 1]. ✓
- Ordering: the §4.10 screen is the phase-1 input gate (§11:507; plan P1 t4 `fn screen(...)`), so the
  refusal structurally precedes §5 stage 4's unconditional `qdcgt_line16`. ✓ (with the F2 caveat that
  part of the predicate is ledger-dependent, not pure-input).

**Residue → F2 (Important, below):** the plan's operational predicate "(Σ int+div+capgain)" is
under-inclusive vs §1(g) and vs the audit's own fix formula ("2b + 3b + max(0, L7) + non-business
L8v").

### I1 (box-12 allowlist) — RESOLVED as specified; one residue → F3, one usability Minor

- §4.10:314 is now an inert **allowlist** `{D,E,F,G,H,S,AA,BB,EE,DD}`, refusing everything else —
  explicitly naming K (Sch 2 L17k excise), R (8853), T (8839), W (8889), A/B/M/N (Sch 2 L13), Z (L17h).
  Every refused code checked: all genuinely tax-affecting → correctly refused. ✓
- Allowlist inertness check (2024 W-2 instructions): D/E/F/G/S (pre-tax elective deferrals, excluded
  from box 1), AA/BB/EE (Roth, already in box 1), DD (informational) — inert ✓. H is included in box 1
  with an optional Sch 1 deduction — ignoring it only overstates tax (conservative; advisory nit, m2).
  The common 401(k) household's D is NOT wrongly refused. ✓ KAT-20 (§10:494; plan P1 t4). ✓
- **Residue → F3 (Important, below):** deferral codes are inert only *within the §402(g) limit*;
  the allowlist admits unlimited amounts.
- **Minor (m2):** very common, genuinely-inert codes C (group-term life > $50k) and V (NSO exercise —
  both already in box 1) are absent from the allowlist, so ordinary households get needlessly refused.
  Fail-closed-safe (refusal is a correct answer) but a scope regression vs the audit's suggested list
  (C, D, E, F, G, H, J, L, P, S, V, Y, AA, BB, DD, EE). Add C/V (and vet J/L/P/Y) once at spec time.

### I2 (Schedule C loss) — RESOLVED

- §4.10:311 refuse row ("Schedule C net < 0 … §465 at-risk + a negative Sch 1 L3 is unsubstantiated");
  §1.2:72. ✓ PLAN: P1 t4:98 routes it downstream as compute-dependent; P2 t2:112-115 owns the
  derivation-side refuse + "KAT: Sch C net<0 ⇒ refuse". ✓
- Nit (m1): the audit's disposition named §4.4a as a co-fix-site; §4.4a itself does not restate the
  refuse. Non-gating — §4.10 is the normative table ("one KAT per row") and §1.2 lists it.

### I3 (`qbi_deduction_override`) — FULLY RESOLVED

- §4.5:236-239 documents the drop with rationale; §7.1:411 is now "box5>0 forces the 8995 map" (no
  override). Whole-file grep of spec + plan: zero dangling references (the §2:100 "raw-override escape
  hatch" is the pre-existing `TaxProfile` hatch, a different mechanism, correctly retained).
- QBI compute story remains whole without it: Σ `div.box5` → 8995 simplified ≤ $191,950/$383,900,
  refuse above, refuse QBI-on-Sch-C (§4.5, §4.10:313); plan P4 t1 implements exactly that. ✓
- Plan r3 changelog:7 records the closure ("no plan task existed — closed"). ✓

### Key Minors — M1 INVERTED (→ F1); M9 and KAT-3 correctly folded

- **M1 (Sch 2 L1/L2): the fold went the WRONG way — see F1.** r5 §5 stage 5:358 now reads "L1 = AMT /
  L2 = excess-APTC"; the actual 2024 form is the opposite.
- **M9 (TI≤0):** §4.10:323 narrowed to "taxable income ≤ 0 **with a capital-loss carryforward**",
  refund-only filers served. ✓ Safety re-derived: a TI≤0 filer *without* a carryforward-in files a
  correct current-year return (Sch D L21 = −min(|L16|, 3,000) is right regardless); the carryover-out
  worksheet effect only touches next year's figure, which stays manual + advisory (audit M3, accepted
  asymmetry), and the naive manual carryover errs in the conservative (smaller-loss) direction. ✓
- **KAT-3 mod-25:** §8:451-453 now states "every bracket edge < $100k is a multiple of $25 …
  corrected per plan-review C1", matching plan P0 t4 (already implemented mod-25 + midpoint-edge KAT).
  Closes `spec-s8-kat3-mod25`. ✓ (FOLLOWUPS.md still lists it open — bookkeeping, m6.)

---

## 2. FINDINGS

### F1 — IMPORTANT (fold regression). The M1 fix inverted the real 2024 Schedule 2 Part I; the spec now contradicts the IRS form and both project recon extractions

**Primary source (fetched 2026-07-12, `irs.gov/pub/irs-prior/f1040s2--2024.pdf`, FINAL):**
2024 Schedule 2 Part I is:
- **1 "Additions to tax": 1a = Excess advance premium tax credit repayment (Form 8962)**; 1b/1c
  clean-vehicle credit-transfer repayments (8936 Sch A); 1d–1f Form 4255 items; 1y other; **1z = add
  1a–1y**;
- **2 = Alternative minimum tax (Form 6251)**;
- 3 = **1z + 2** → 1040 L17.

This matches the project's own extractions exactly — deep/03:222 ("L1z=`f1_11`, **L2 AMT**=`f1_12`,
L3→1040 L17=`f1_13`") and recon-01:46 ("Part I (AMT **L2** / excess APTC repayment **L1**)") — and the
audit M1's statement ("APTC repayment lives in the L1a–1z block"; fix = correct the §9.2 cite).

**What r5 did instead:** flipped the normative pipeline to match the one wrong cite. Now ALL of:
- §5 stage 5:358 — "**L1 = AMT** (screen=0); **L2 = excess-APTC** (refuse); L3 = L1+L2" — wrong on
  both lines, and the sum formula (real form: L3 = L1z + 2);
- §4.11:327 — "Sch 2 **L1** = 0, not silently" (AMT's explicit screen-zero cell is **L2**);
- §4.10:312 — excess-APTC row rationale "Sch 2 **L2** repayment" (it is **1a**);
- §9.2:462 — "excess-APTC/Form 8962 (Sch 2 **L2**)" (the original erratum, still uncorrected);
- §10:500-501 — the erratum note now asserts "recon-01 §2 shows Sch 2 L1/L2 **swapped vs the 2024
  form**" — recon-01 was RIGHT; the note falsifies the project record.

**Why Important, not Minor/Critical:** no in-scope tax dollar changes (AMT is screened, APTC refused —
Part I carries 0 either way), so not Critical. But the audit ranked the ORIGINAL as Minor because only
one stray §9.2 cite was wrong while the normative pipeline was right; r5 made the normative pipeline,
the refuse table, the AMT-screen section, and the erratum record all wrong. §4.11 mandates *printing*
the screen's 0 ("not silently"), so P6 would map that explicit 0 to the wrong cell (1a — visually
asserting a Form-8962 reconciliation of $0 with no 8962 attached); the P6 fresh-extraction task will
collide head-on with the spec text (churn at best, spec-following mis-map at worst); and the shipped
LIMITATIONS doc would misdirect users. A normative spec statement that contradicts its primary source,
introduced by the fold, is exactly what this gate exists to stop.

**Fix (spec-only; plan never states an L1/L2 orientation):** restore the true layout at all five
sites — stage 5: "Sch 2 Part I: L1a = excess-APTC (refuse if any, §4.10; rest of 1a–1y out of scope
⇒ 1z=0); **L2 = AMT** (screen=0, §4.11); L3 = L1z + L2 → 1040 L17"; §4.11 "Sch 2 **L2** = 0"; §4.10
row cite "Sch 2 **L1a**"; §9.2(ii) cite likewise (fold with `fr-8962-taxonomy` as the audit directed);
rewrite the §10 erratum note (recon-01/deep-03 were correct; r4's §9.2 cite was the sole error).

### F2 — IMPORTANT (incomplete C1 fold). The kiddie-tax unearned-income predicate misses in-scope unearned income — including the app's flagship type — so a sliver of C1's silent understatement survives

PLAN P1 t4:97 operationalizes the trigger as "unearned income **(Σ int+div+capgain)** > $2,600".
Under §1(g)(4) (unearned = AGI not attributable to §911(d)(2) earned income; Form 8615 instructions
enumerate interest, dividends, capital gains, **unemployment compensation**, etc.), this omits, all
in-scope:
- **Sch 1 L8v non-business crypto ordinary income** (hobby mining/staking/airdrop/reward + non-business
  lending interest, §4.4a) — present in the audit's own fix formula ("2b + 3b + max(0, L7) +
  **non-business L8v**") and dropped by the fold. `IncomeKind`/`Income.business` verified at
  `crates/btctax-core/src/event.rs:33-39,61` — none of these hobby kinds is §911(d)(2) compensation.
- **Sch 1 L7 unemployment** (1099-G, in scope §4.3) — explicitly unearned per the 8615 instructions.
- Sch 1 L1 taxable state refunds (attest input).

A claimable 17-year-old with $3,000 of hobby staking income (or unemployment) still files silently at
the child's rate — the exact C1 understatement class, in the very income type this app exists for.

**Secondary defect (same fix):** the plan lists the row under "one KAT per **input-screenable** row"
with signature `screen(&ReturnInputs, &TaxTable)` — but `capgain` (ledger Sch D → 1040 L7) and L8v
(ledger `crypto_ord`) are not in `ReturnInputs`. As typed, the screen cannot evaluate its own
predicate; if implemented ReturnInputs-only, a claimable dependent with $5k of custodial-account
crypto LTCG also leaks. Either pass the ledger-derived figures into the screen or split the row like
the other compute-dependent rows (TI≤0 → P3, Sch C loss → P2).

**Fix (one line each, spec + plan):** define unearned income by complement, reusing §4.7's G21
derivation — `unearned = in-scope total income − earned income (Σ box1 + Sch C net − ½SE)`, i.e.
`2b + 3b + max(0, 1040 L7) + Sch 1 (L1 + L7 + L8v)` — pin it in the §4.10 row (the spec currently
defers to bare "§1(g)"), and correct the P1 placement/signature. KAT-19 gains a hobby-income variant.

### F3 — IMPORTANT (I1 residue). Allowlisted elective-deferral codes are inert only within the §402(g) limit; a core multi-W-2 household exceeds it and the return silently understates

Box-12 codes D/E/F/G/S (and H) are inert *because* the deferral is already excluded from box 1 — but
only up to the §402(g)/plan limit (TY2024: $23,000 elective deferral, +$7,500 age-50 catch-up; SIMPLE
$16,000). **Excess deferrals are includible in wages (1040 line 1h)** — a line v1 does not compute
(1a = Σ box1 only, §5 stage 1). Single-plan excess is administrator-prevented, but the excess arises
naturally in exactly v1's headline household — **multi-employer** (job-change year, both 401(k)s
funded: two × $13k = $26k of code D > $23,000). The $3,000 excess is captured (box12 Vec), unmodeled,
and tax-increasing — precisely the class §3.4 forbids and the same class as the audit's I1 (K/R/T),
which it ranked Important. The allowlist admits it unconditionally.

**Fix (one §4.10 condition + KAT):** make the deferral rows amount-conditional — per person, refuse
when Σ(box-12 D+E+F+G+H+S amounts) > the year's §402(g) elective-deferral limit + (DOB ≥ 50 ?
catch-up : 0) (limits → §8 `TaxTable`, indexed; a blunter refuse-over-$23,000-flat with an advisory is
also acceptable fail-closed, at the cost of refusing legitimate 50+ maxers). G's separate 457(b) limit
and SIMPLE's lower limit can be folded into the same row conservatively.

### Minors (m1–m6; none gates individually)

- **m1** — §4.4a does not restate the Sch C net<0 refuse (audit disposition named §4.4a/§4.10 as
  co-sites). One sentence in §4.4a; §4.10 already normative.
- **m2** — allowlist over-refusal of common inert codes C and V (see I1 confirmation above); H's
  foregone Sch 1 deduction is a favorable omission without the §3.4 advisory apparatus — add C/V after
  one-time verification + an H advisory, or accept and note in LIMITATIONS.
- **m3** — the audit's disposition ("M1–M11 fold into FOLLOWUPS.md") was not executed: **M2–M8, M10,
  M11 appear nowhere in FOLLOWUPS.md** (M1/M9 went into the spec instead — M9 correctly, M1 per F1).
  Sanity-check performed: **M2 (derived-profile pref>TI clamp) is confirmed Minor** — the missing
  L10=min(L1,L4) cap degrades only the delta/planning number, which §6 already labels approximate; the
  filed return uses `qdcgt_line16` with the F-A cap. M5–M8/M10/M11 re-checked: none is secretly ≥
  Important (M8's 1099-INT box-10 market discount is *uncaptured* — the "unrepresentable/documented"
  class, needing the P5 exhaustive LIMITATIONS list, not a §3.4 refuse). Transcribe all nine into
  FOLLOWUPS.md so they aren't lost.
- **m4** — plan P0 t5's `TaxTable` bundling enumeration omits the §1(g) threshold (and would omit the
  F3 §402(g) limits) that P1 t4 consumes; TDD self-heals (the KAT-19 red test forces it) but the
  enumeration should name them.
- **m5** — plan:8 "**Implements:** `design/SPEC_full_return.md` (GREEN r4)" is stale → r5.
- **m6** — FOLLOWUPS.md `spec-s8-kat3-mod25` is resolved by spec r5 §8; mark closed.

---

## 3. Regression scan — clean except F1

- **New refuse rows vs §3.4 carve-out:** all three new rows guard tax-*increasing* configurations
  (8615 parent-rate tax; box-12 codes triggering Sch 2/8853/8839; an unsubstantiated Sch C loss
  *claim*), so none belongs in the favorable-omission carve-out; the carve-out remains
  favorable-omissions-only (CTC/ODC/EIC, §3.4/§9.2(i)). No interaction. ✓
- **Kiddie row vs §4.7 dependent support:** claimable filers with earned income remain fully served
  below the threshold; the dependent std-deduction floor and G21 derivation are untouched. Consistent
  (and G21 is the natural earned-income half of the F2 fix). ✓
- **Dropping qbi_override:** box5 → 8995 story whole (§4.5/§7.1/plan P4 t1); over-threshold and
  QBI-on-Sch-C still refuse; the raw `TaxProfile` hatch (§2) remains the global escape. No orphan
  references (whole-corpus grep). ✓
- **TI≤0 narrowing:** conservative in both remaining branches (see M9 confirmation). ✓
- **Fail-closed surface:** with F2+F3 folded, the audit's three leak classes are closed; the r5 rows
  do not weaken any previously-verified refuse/omit path (all r4 rows retained verbatim in §4.10). ✓
- **Path nit:** the audit/prompt cite `crates/btctax-core/src/tax/event.rs`; the file is
  `crates/btctax-core/src/event.rs`. Content claims verified (IncomeKind exactly 5 variants;
  `Income.business`). No design impact.

---

## 4. Disposition

| # | Finding | Rank | Fix site |
|---|---|---|---|
| F1 | M1 fold inverted vs the real 2024 Sch 2 (L1a=APTC/1z; **L2=AMT**; L3=L1z+L2) | **Important** | SPEC §5 stage 5, §4.10 APTC row, §4.11, §9.2(ii), §10 erratum note |
| F2 | Kiddie unearned-income predicate under-inclusive (L8v/L7/L1 missing) + P1 screen can't see ledger figures | **Important** | SPEC §4.10 row (pin the formula via G21 complement); PLAN P1 t4 |
| F3 | Box-12 deferral codes inert only ≤ §402(g); multi-W-2 excess-deferral understatement | **Important** | SPEC §4.10 row condition + §8 limits; PLAN P1 t4 KAT |
| m1–m6 | as listed | Minor | spec/plan text + FOLLOWUPS.md |

**Gate: NOT met — 0 Critical / 3 Important.** C1, I2, I3 are confirmed resolved; I1 is resolved as
designed with one amount-conditional residue (F3); the M1 Minor was folded backwards (F1); the C1 fold
left the predicate under-inclusive (F2). All three Importants are localized text edits with no
architectural impact. **The design is NOT yet sound to begin implementation; fold F1–F3 (+ transcribe
the m3 Minors) and re-confirm — that pass should be trivial.**
