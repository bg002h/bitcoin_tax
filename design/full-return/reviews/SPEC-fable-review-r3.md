# FABLE Independent Review — `SPEC_full_return.md` — Round 3 (re-review of the r2 fold + D-3/D-5/D-6 decision fold)

**Reviewer:** Fable (independent; author = opus). **Date:** 2026-07-12.
**Target:** `design/SPEC_full_return.md` **r3** (r3 changelog header; fold of
`design/full-return/reviews/SPEC-fable-review-r2.md` R2-I1..I4 + Minors, and user decisions D-3/D-5/D-6).
**Method:** every r2 finding re-verified against the r3 text; the D-6 fold (Schedule C + SE back into v1)
audited end-to-end for regressions; source and recon re-verified where load-bearing:
`crates/btctax-core/src/{event.rs,tax/compute.rs,tax/se.rs}`, `crates/btctax-forms/src/{schedule_se,
schedule_d,verify}.rs`, recon `deep/02` §§1–4, `deep/04` §§2d/5 (:175–:191), `fable/04` §§2/4,
`fable/06` G11/G12. Finding arithmetic machine-checked (8959 Part II divergence; the NIIT invariant
counterexample; KAT-9).

**VERDICT: NOT GREEN — 0 Critical / 4 Important / 10 Minor.**
The r2 fold is high-quality where it was prescribed exactly: R2-I1 (charitable 30%-class ceiling), R2-I2
(Schedule D exhaustive routing), and R2-I4 (SALT either/or) are **fully resolved**, and the D-3 (§904(j) FTC)
and D-5 (one spec/phased plan) folds are clean. The D-6 fold is structurally right — Schedule C + SE are
consistently *in* everywhere (no dangling "SE follow-on" text survives), the hobby/business split is keyed to
the real ledger flag, Sch 2 L4 is correctly unbundled, and the SS cap is per-owner. What keeps the gate
closed: one r2 residue (the Schedule B trigger enumeration is still incomplete) and **three defects the D-6
fold introduced** in §5 stage 7 and the reduce-to-delta contract — a mis-clamped 8959 Part II formula that
diverges on the spec's own golden fixture, a business-flagged-Interest seam that understates NIIT relative to
the frozen engine's own position, and a reduce-to-delta invariant that is now mathematically false in a
reachable SE regime. All four are local, normative-text fixes; no decision needs reopening.

---

## 1. r2 findings — resolution audit

| r2 | Status in r3 | Evidence |
|---|---|---|
| **R2-I1** charitable 30%-class cap | **RESOLVED** | §4.6: `min(30%·AGI, 50%·AGI − (allowed 60%/50%-tier contributions this year, INCLUDING allowed ordinary-income-property amounts))` — exactly deep/04 :189–:191 ("cash + ordinary-income amounts already allowed"). No residual "− cash only" anywhere. KAT-17 added and it discriminates (same-year ST+LT crypto donations exercise the dropped term; r2's own $100k/$10k/$35k/$25k example now yields $5k, not $25k, under the specced formula). |
| **R2-I2** Sch D routing | **RESOLVED** | §7.2 enumerates **all four** paths and each matches the 2024 Schedule D: both-gains (L17=Yes; L18=L19=0 behind the DIV 2b/2c/2d refuse-guard; L20=Yes → QDCGT; 21/22 not completed); **ST-gain/LT-loss** (L16>0 ∧ L15≤0 → L17=**No** → skip 18–21 → L22, Yes iff 3a>0); loss (skip 17–20; L21 = −min(|L16|, 3000/1500-MFS), sign policy §3.2; L22); **zero** (L16=0 → 1040 L7=0, skip 17–21, L22). The partition {L16>0∧L15>0, L16>0∧L15≤0, L16<0, L16=0} is exhaustive; no path prints L17=Yes on a skipped line. KAT-10 covers all four. |
| **R2-I3** Sch B force-file / Form 3520 | **PARTIAL → R3-I2** | The substance landed: `foreign_trust == Some(true)` ⇒ refuse is a §4.10 row (per-row KAT) + §1.2; §5 stage 1 now reads "[Sch B if >$1,500 **OR foreign**, §7]" so a below-threshold foreign account forces Sch B. The residue: the trigger enumeration is still not the prescribed normative line — see R3-I2. |
| **R2-I4** SALT 5a either/or | **RESOLVED** | §4.6: election **on** → `5a = salt_sales_tax_amount` **only** (income-tax withholding excluded); **off** → `5a = Σbox17 + Σbox19 + estimated + prior-year` — "Never both." Field renamed (`salt_sales_tax_amount`, "used IFF"); no path sums both. Two crumbs → R3-M9. |
| R2-M1 KAT-9 | **RESOLVED** | §10 KAT-9: 271.50 + 499.50 → printed 272 + 500 = **772** "(not round(771.00))" — arithmetic now correct (machine-checked) and **in-envelope** (8959 Part II is filled in v1 per D-6); genuinely discriminates printed-line rounding + cross-foot (772 vs 771). |
| R2-M2 SE fill-set contradiction | **RESOLVED (mooted by D-6)** | Schedule SE is now genuinely in the fill set (§1.1/§2/§7.1) and the existing `schedule_se.rs` **is** a real PDF filler with geometric read-back (verified: `fill_schedule_se_with_map` → PDF bytes) — "existing" is accurate. (Its $400 floor is unhandled in §5 → R3-M3.) |
| R2-M3 FTC omission-vs-refuse | **RESOLVED (mooted by D-3)** | §3.4 carve-out now lists only CTC/ODC, EIC; FTC is implemented (§4.7a) with a refuse row above the cap. Aligned. |
| R2-M4 phase double-assignment | **NOT FOLDED** → R3-M7 | §11 still books Sch 1 into phase 2 (stages 1–2) *and* phase 4 ("Schedule 1 (incl. L8v) + Schedule C net"); phase 3 emits L13–L16 but QBI/8995 is phase 4 (needs the 0-stub statement). |
| R2-M5 G20 checkbox | **RESOLVED** | §5 stage 9: "'Sch D not required' box (1040 L7) always unchecked (btctax always files Sch D, M4/G20)". |
| R2-M6 donee-class advisory | **NOT FOLDED** → R3-M4. |
| R2-M7 8960 printed line for crypto interest | **NOT FOLDED** → R3-M5 (now doubly needed — see below). |
| R2-M8 write-back precedence | **NOT FOLDED** → R3-M6. |
| R2-M9 dead SE term in §4.7 | **RESOLVED (mooted by D-6)** | "Σ box1 + Schedule C net − ½SE (G21)" is now live and substantively correct (box-7 nit → R3-M2). |
| R2-M10 8962/Sch 2 L2 + recon erratum | **HALF-FOLDED** | The erratum note is in (§10: recon-01's swapped Sch 2 L1/L2 recorded, spec's §4.11 confirmed correct). The 8962/excess-APTC out-of-scope enumeration is still absent from §1.2/§9.2 → merged into R3-M8. |

## 2. Decision-fold audit (D-3 / D-5 / D-6)

- **D-3 §904(j) — CLEAN.** §4.7a: `ftc_raw = Σ(int.box6 + div.box7)`; direct Sch 3 L1 (no Form 1116) iff
  passive + 1099-reported + ≤ $300/$600; refuse above (§4.10 row + §1.2 + D-3 all agree); Sch 3 L8 → 1040 L20
  → L21/L22 nonrefundable mechanics (§5 stage 6); ceiling in the §8 table; KAT-16 (≤cap credit + >cap refuse);
  §12 D-3 marked resolved. The "1099-sourced foreign tax assumed passive" advisory is stated.
- **D-5 — CLEAN.** One spec; §11 is the single phased plan ("one plan; D-5"); §12 D-5 resolved.
- **D-6 — structurally consistent, three defects in the seams (below).** Verified consistent across
  §1.1/§1.2 (SE in; Forms list has C new + SE existing), §2 (se.rs reused as-is; SE in the filler line;
  `schedule_c.rs` new), §4.4 (L3/L15 derived), §4.4a (one Sch C; owner; ≥2 SE earners → refuse = §4.10 row 1,
  correctly cited to fable/04 hardening (b)), §4.7 (dependent earned income now includes Sch C net − ½SE),
  §5 stages 1/2/7 (L3 in L8-chain; ½-SE → Sch 1 L26; SE tax → Sch 2 L4 **unbundled** — `ss + medicare` only,
  0.9% routed to 8959 Part II, matching `se.rs` where `total` bundles `addl` and `deductible_half` excludes
  it), §7.1/§7.3 (C filler new; SE existing — verified real), §10 (KAT-5 re-includes SE regimes; KAT-6, KAT-15;
  golden = deep/02 Ex.2 $60k mining), §11 (Sch C/SE in phases 2/4/6), §12 (D-6 resolved). **No dangling "SE
  out of v1" / "Part I only" / "SE follow-on" text survives** (checked every SE/8959/Schedule-C mention).
  The hobby-vs-business split is keyed to `Income.business` (**verified `event.rs:61`**), and Sch C gross ∪
  L8v partitions `crypto_ord` (compute.rs:300–305) exactly — every crypto ordinary dollar has one printed home.
  The SS cap is per-owner (§4.4a/§5 stage 7 "owner's own box 3"; deep/02 C4) while 8959 uses household Σbox5. ✓

---

## 3. IMPORTANT

### R3-I1 — §5 stage 7: the 8959 Part II formula drops the inner threshold clamp (and the 8960 closure drops both floors) — fold-introduced

**Location:** §5 stage 7: "Part II 0.9%·max(0,SE−(thr−Σbox5))" and "L17=3.8%·min(NII,MAGI−thr)".
**Problem (a).** Form 8959 L11 is `max(0, thr − Σbox5)` — the reduced threshold **floors at zero**. The
spec's inline formula omits the inner clamp, so whenever household Medicare wages exceed the threshold
(`Σbox5 > thr` — the *normal* case for a return that owes Part I), `(thr − Σbox5)` goes negative and the
formula **adds the wage excess to the SE base**: on the spec's own named golden (deep/02 Ex.2, §10 golden
matrix: Σbox5 = 280,000, SE base = 55,410, thr = 250,000) it yields L13 = 0.9%·85,410 = **768.69** where the
correct L13 = **498.69** (verified `se.rs` = Form 8959 L11–L13 exactly; machine-checked). L18 then
double-counts Part I (1,038.69 vs 768.69) — tax overstated by exactly the Part I amount.
**Problem (b).** Same block: "L17=3.8%·min(NII,MAGI−thr)" omits both statutory floors (8960 L12 "if ≤0,
-0-"; L15 "if ≤0, -0-"). MAGI < threshold is the *common* case; a literal implementation produces a
**negative NIIT** on Sch 2 L12 — tax understated (the cardinal sin). The engine's own closure has both
clamps (`compute.rs:369-380`, the D2 floor comment), as does deep/02 §2.4.
**Why.** Stage 7 was rewritten for D-6; the shorthand lost the clamps that deep/02 §§2.4/3.1 print
explicitly. The spec is the normative text a plan inherits; KAT-6 as specced would be *built from* the wrong
formula.
**Fix.** "Part II = 0.9%·max(0, SE − **max(0,** thr − Σbox5**)**) (≡ `se.rs.addl` ≡ 8959 L11–L13)" and
"L17 = 3.8%·**max(0,** min(**max(0,** NII**)**, **max(0,** MAGI − thr**)**)**)** (≡ the `compute.rs:369`
closure / deep/02 §2.4)". Point KAT-6 at the Σbox5 > thr fixture (deep/02 Ex.2 already is one).

### R3-I2 — Schedule B trigger enumeration still incomplete: the dividends > $1,500 term is missing, "foreign" is undefined, and the pointer dangles (R2-I3 residue)

**Location:** §5 stage 1: "2b=Σ(int.box1+box3) [Sch B if >$1,500 OR foreign, §7]" — the spec's **only**
trigger text (grep-verified; §7 contains no trigger line, only the §7.4 overflow note).
**Problem.** fable/06 G11 quotes the 2024 Part III header verbatim: required if "(a) had over $1,500 of
taxable interest **or ordinary dividends**; (b) had a foreign account; or (c) received a distribution from …
a foreign trust." The r3 trigger (i) hangs off the *interest* line only and never states the **ordinary-
dividends > $1,500** trigger — a household with $2,000 of index-fund dividends and $100 of interest (squarely
the spec's own golden-matrix population, which includes REIT-dividend fixtures) files with **no Schedule B at
all**: Part II and the mandatory Part III answers silently omitted — the same incomplete-filed-return class
R2-I3 was ranked Important for; (ii) never defines "foreign" (the intended reading — `foreign_accounts ==
Some(true)`; trust refuses first — is discernible but nowhere normative) and cites "§7", which defines
nothing; (iii) the below-threshold foreign-account KAT prescribed in r2 was not added (§10 has no Sch B KAT;
the per-row rule covers only the trust *refusal*).
**What DID land:** the foreign-account force-file itself ("OR foreign"), the `foreign_trust == Some(true)` ⇒
refuse row (Form 3520), the tri-state fail-loud, and the §9 FinCEN advisory — the r2 harm's core is fixed.
**Fix.** The one normative line, verbatim from r2: "Sch B files when taxable interest > $1,500 **or**
ordinary dividends > $1,500 **or** `foreign_accounts == Some(true)` (Part III trigger (b)) or user-forced;
`foreign_trust == Some(true)` refuses before filing (trigger (c) → Form 3520)." Put it in §7.1 (where stage 1
points), and add the ≤$1,500-with-foreign-account KAT.

### R3-I3 — Business-flagged Interest income breaks three D-6 seams: Sch C/Sch SE disagree, and the absolute NII exclusion understates NIIT (fold-introduced)

**Location:** §4.4a ("Gross business income = Σ ledger `crypto_ord` where `Income.business == true` …
net profit → Sch 1 L3; and feeds Schedule SE Part I"); §5 stage 7 (8960: "EXCLUDES Schedule C business
income (§1411 active-business exclusion)"; NII includes only "**non-business** crypto investment income
(lending interest)").
**Problem.** `business = true, kind = Interest` is a first-class ledger state (`InboundClass::Income{kind:
Interest, business: true}`, `ReclassifyIncome`; `se.rs` KATs the combination —
`m2_business_interest_excluded_mining_included`). For that state:
1. **Sch C vs Sch SE contradiction.** §4.4a sweeps it into Schedule C gross (all-kinds filter) and says the
   net "feeds Schedule SE Part I" — but the spec simultaneously mandates the frozen `se.rs`, which **excludes
   Interest from the SE base** (§1402(a)(2); `se.rs:59`). The printed Schedule SE L2 ("net profit from
   Schedule C") then cannot equal Schedule C L31 and the spec doesn't say which governs: implement the form
   literally and SE tax is charged on interest `se.rs` correctly exempts; implement `se.rs` and the two
   printed forms visibly disagree. (Also `se.rs` charges the *whole* `schedule_c_expenses` against the
   non-interest gross while Sch C nets them against the interest-inclusive gross — a second silent wedge.)
2. **NIIT understated.** The blanket "EXCLUDES Schedule C business income" sweeps business interest out of
   Form 8960 NII. But §1411(c)(6) shelters only items **taken into account for SECA** — and §1402(a)(2) keeps
   interest *out* of SECA, so business-flagged interest escapes **both** the 0.9%/SECA side and the 3.8% side.
   The ordinary-course exception (§1411(c)(1)(A)(i)) requires a non-passive, non-§1411(c)(2)-trading business
   determination the model doesn't make. The frozen engine deliberately takes the conservative position —
   `interest_nii` includes **all** lending interest with no business filter (`compute.rs:310-315`) — so the
   absolute return would report *less* NIIT than the product's own delta engine computes for the same ledger:
   tax understated, §3.4's cardinal rule, and the "absolute ≥ delta" invariant (deep/02 §4.3.2) breaks.
3. Tertiary: §4.7's dependent earned income ("Schedule C net − ½SE") also over-counts interest as earned.
**Fix (pick one, state it).** (a) **Refuse-guard row**: business-flagged `Interest` income ⇒ `NotComputable`
on the full-return path (cleanest v1; the standalone SE report is untouched); or (b) define the seams
kind-wise: Sch C gross = business `crypto_ord` of SE-eligible kinds only… **but then business interest needs
a stated printed home** (else the C1 income-hole returns), NII includes **all** lending interest (business or
not — matching `interest_nii`), and Sch SE L2 explicitly = Sch C net (no interest present by construction).
(a) is one row + one KAT.

### R3-I4 — The reduce-to-delta invariant (§5 tail, KAT-5) is false for SE regimes: the absolute MAGI sees ½-SE and Schedule C expenses the frozen engine cannot (fold-introduced)

**Location:** §5 after the pipeline: "absolute 8960/8959 collapse to the engine's crypto-delta when
non-crypto inputs are 0 — KAT'd across the 4 regimes incl. SE/business-income…"; §10 KAT-5.
**Problem.** Under D-6 the absolute AGI/MAGI subtracts the ½-SE deduction (stage 2) and uses Schedule C
**net** on L3, but the engine's `magi_with` adds `crypto_ord` **gross** (`compute.rs:364-366`; `se.rs`
doc: "`crypto_ord` in engine B remains GROSS"). deep/02 §4.3.1's "absolute MAGI = engine `magi_with`" was
proven **before** ½-SE/Sch-C entered the absolute model. Whenever the MAGI arm of 8960 binds, equality fails
and even "absolute ≥ delta" inverts. Machine-checked counterexample (Single, $200k business mining, $20k
non-business lending interest, expenses 0, no non-crypto inputs): ½-SE = 13,131.35 → absolute MAGI =
206,868.65 → absolute NIIT = 3.8%·6,868.65 = **261.01**; engine delta = 3.8%·min(20,000, 220,000−200,000) =
**760.00**. Absolute ≠ delta and absolute < delta — with all non-crypto inputs zero, exactly the condition
under which the spec asserts equality. (The 8959 half of the invariant survives: Part II reads the same
`se.rs` base on both sides, expenses included; Part I = 0 without wages.) fable/04's four probed regimes all
happened to be NII-binding, which is why this never surfaced. The absolute side is the *correct* Form 8960
(L13 = AGI); it is the **invariant statement** that is wrong — and its natural mis-fixes at plan time
(strip ½-SE from the absolute MAGI, or teach the frozen engine about expenses) are respectively tax-wrong and
freeze-breaking.
**Fix.** Scope the invariant: "8959 collapses exactly; 8960 collapses exactly for regimes without SE income,
and for SE regimes **when the NII arm binds** — with SE income the absolute MAGI additionally reflects ½-SE
and Schedule C expenses, which the frozen delta cannot see (documented divergence, cf. §6)." Pick KAT-5's
SE fixture NII-binding (deep/02 Ex.2 qualifies), and add one KAT pinning the *documented inequality* for a
MAGI-binding SE fixture (e.g. the counterexample above).

---

## 4. MINOR

- **R3-M1 (§4.4a/§7.3):** the Schedule C fill is underspecified: the single `expenses` scalar has no named
  Part II landing line (line 27a requires a Part V itemization on page 2), and the form's mandatory header
  enumerations — line B principal-business code (6-digit NAICS; 999999 or a stated default), line F
  accounting method — aren't in `ScheduleCInputs`. Name the line and the two defaults so the map extraction
  has targets.
- **R3-M2 (§4.4a/§5 stage 7):** Schedule SE L8a is W-2 **box 3 + box 7 tips** (`se.rs` doc; §4.1 captures
  `box7_ss_tips`), but both SS-cap sentences say "box 3" only. Also stage 7 routes "Part V L22 → 1040 25c"
  while §4.8 (correctly) uses **L24** (= L22 + L23); equal only because RRTA = 0 — cite L24.
- **R3-M3 (§5 stage 7):** the §6017 **$400 floor** is unstated: `compute_se_tax` has no floor, while the
  reused filler skips the form below it (`schedule_se.rs` `SE_FLOOR`, returns `Ok(None)`); a literal assembly
  charges Sch 2 L4 on a sub-$400 base with **no Schedule SE attached** — tripping §7.1's unbacked-line rule
  (spurious refusal) or filing inconsistently. State: base < $400 ⇒ SE tax = 0, no Sch SE, no ½-SE, 8959 L8 = 0.
- **R3-M4 (R2-M6 survives):** the public-charity-donee assumption behind the ledger's auto-classing
  (`CapGainProp30`/`OrdinaryProp50`) is still not surfaced as an advisory + §9.2 LIMITATIONS line
  (deep/04 :218–:219: private-foundation donee ⇒ 20%-class at basis).
- **R3-M5 (R2-M7 survives):** the printed Form 8960 line carrying crypto lending interest is still unnamed —
  now doubly load-bearing: with crypto interest riding Sch 1 L8v, printed 8960 **L1 ≠ 1040 2b** unless the
  spec picks L1-with-a-note or L7. Name it.
- **R3-M6 (R2-M8 survives):** carryover write-back still mixes two mechanisms ("staging field" vs "written to
  year Y+1's row") and is silent on precedence when Y+1's `carryover_in` was user-entered (overwrite/refuse/
  provenance).
- **R3-M7 (R2-M4 survives):** §11 phase double-assignment — Schedule 1 is computed in phase 2 (stages 1–2)
  yet re-listed in phase 4; phase 3 emits L13–L16 but QBI/8995 is phase 4. State "phase 2 computes, phase 4
  completes; QBI = 0-stub until phase 4" or reorder.
- **R3-M8 (R2-M10 residue + taxonomy):** Sch 2 L2 excess-APTC / Form 8962 is still absent from the §1.2/§9.2
  out-of-scope enumerations. While there: §9.2's "conservative-omission list" mixes true favorable-only
  omissions (CTC/ODC, EIC) with screened items (AMT — a §4.11 *refuse*-trigger) and unrepresentable-scope
  items (1099-R/SSA — no input exists; §3.4 would *refuse* if captured); §1.2's newly added credits
  (education/dependent-care/saver's/energy/adoption) are missing from §9.2's list. Align the three lists with
  §3.4's omission-vs-refuse split.
- **R3-M9 (§4.6):** two SALT crumbs: the 5a **sales-tax election checkbox** fill (checked iff
  `salt_use_sales_tax`; deep/03 `c1_1`) is no longer stated; and a nonzero `salt_sales_tax_amount` with the
  election **off** is silently ignored where r2's fix prescribed fail-loud (refuse) — silent-ignore is
  tax-safe but hides an input error.
- **R3-M10 (§4/§4.4a):** behavior when the ledger has business income but `schedule_c == None` is unstated —
  the owner (per-earner SS cap on MFJ) and business description are unknowable; per the G15 pattern this must
  fail loud, not default. One sentence. (Nit, same area: §2's "TaxProfile (2 scalars)" undersells the
  ~9-field profile deep/02 §1.3 enumerates.)

---

## 5. Re-checked and found CLEAN (beyond §§1–2)

- **§4.6 charitable engine** end-to-end against deep/04: 6-class enum, statutory ordering, oldest-vintage-
  first, 5-yr expiry, G8 aging in std years, ledger §170(e) supply by holding period (LT→30% FMV, ST→50%
  basis), the restored two-term 30%-class cap (worked example deep/04 §3 reproduces), persistence home.
- **§7.2** against the real 2024 Schedule D flow (L16 zero/loss sentences, L17 both-gains question, L20
  yes-branch "don't complete 21/22", L21 §1211 amounts, L22 QD question) — all four specced paths correct.
- **SE unbundling** against source: Sch 2 L4 = `ss + medicare` (never `se.rs.total`, which bundles `addl`);
  ½-SE = `deductible_half` (excludes `addl`); 8959 Part II ≡ `se.rs.addl`; the existing Schedule SE PDF
  filler is real (geometric read-back, line 12 = ss+medicare pinned in `schedule_se.rs` R0-C1).
- **KAT-9** arithmetic + envelope + discrimination (771.00 cent-sum vs 772 printed; Part II in-scope via D-6).
- **§4.4a buildability:** `Income.business` at `event.rs:61`; `crypto_ord` partition (Sch C ∪ L8v) exact;
  `ReclassifyIncome` exists for flag correction; one-Sch-C structurally enforced (`Option`, not `Vec`);
  ≥2-SE-earners refuse row cites fable/04 hardening (b) accurately.
- **8960 business exclusion for SE-eligible kinds** (mining/staking/airdrop/reward, business=true): correct
  under §1411(c)(6) and consistent with the engine, which already excludes them from NII (the defect is
  Interest only — R3-I3).
- **§4.7a/§4.8/§4.9/§4.10/§4.11/§4.12, §6, §8, §9** re-read post-fold: no D-6 collateral damage found;
  refuse-guard table gained the two D-6 rows + 3520 row, each KAT'd; §8 gains nothing SE-specific it needs
  (wage base already present); §12 D-1..D-6 all marked resolved and consistent with the body.
- **§10** layer plan: KAT set 1–17 + per-row coherent with D-6 (KAT-5 regimes, KAT-6, KAT-15, golden
  matrix incl. deep/02 Ex.2); ATS partial-diff caveat retained; loss-year negative-cell KAT retained.

## 6. Disposition

**Gate does not pass: 0 Critical / 4 Important / 10 Minor.** The r2 Importants are 3-of-4 fully resolved
(R2-I1, R2-I2, R2-I4) with R2-I3 substantively landed but its prescribed enumeration only half-folded
(→ R3-I2). The D-6 fold is architecturally sound — the three new Importants are seam defects, not structure:
one formula clamp (R3-I1), one input-state whose cleanest fix is a refuse row (R3-I3), and one invariant
scope-statement (R3-I4). All four fixes are local normative text; none reopens a user decision. Re-review
(r4) after the fold per `STANDARD_WORKFLOW.md` §2; given locality, r4 should be fast.
