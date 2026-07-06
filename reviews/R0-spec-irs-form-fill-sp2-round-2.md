# R0 review — SPEC_irs_form_fill_sp2.md — round 2 (verification of the R0-round-1 fold)

- **Artifact:** `design/SPEC_irs_form_fill_sp2.md` @ `feat/irs-form-fill-sp2` `80ba633` (main == `99f26ca`).
  Spec's own last commit is `80ba633` ("fold SP2 R0 round 1 (4C/4I/3M/2N, Fable) — awaiting round 2").
- **Reviewer:** independent architect (R0, round 2; model: Opus 4.8 [1m]). Author ≠ reviewer.
- **Bar:** 0 Critical / 0 Important. Tax-critical (filling 3 more OFFICIAL IRS PDFs).
- **Charge:** confirm each round-1 fold (4C/4I/3M/2N) is captured correctly + internally consistent; scrutinize
  the C3 per-form geometric oracle hardest; find any NEW gap the rewrite introduced.
- **Method:** evidence-driven, re-verified against source at this commit. Re-read the folded spec; the SP1
  engine (`btctax-forms/src/{verify,schedule_d,map}.rs`); the core (`btctax-core/src/tax/se.rs`,
  `forms.rs`, `donation.rs`); the export site (`btctax-cli/src/cmd/admin.rs`). Spot-verified every
  load-bearing PDF fact against the bundled 2025 officials via the scratchpad pypdf 6.x venv
  (`irsforms/{f1040,fSE,f8283}-2025.pdf` field dumps + layout-mode page text — the Fable R0's saved
  `*_fields.txt`/`*_text.txt`, re-read here). Leaf counts re-confirmed: 1040 = 199, SE = 27, 8283 = 117.

## VERDICT: **1 Critical? no — 0 Critical / 1 Important / 3 Minor / 2 Nit → NOT YET R0-GREEN**

All four Critical folds (C1 line-12, C2 line-7a, C3 per-form oracle, C4 DA-conditionality) are captured
**correctly and are PDF-verified**, and the C3 oracle — the flagged residual-Critical risk — is **sound**
for all three forms (proof below). All four Important folds (I1–I4), all three Minor (M1–M3), and both Nit
(N1–N2) are folded faithfully. **The rewrite introduced one NEW residual Important (I★1): a self-consistency
gap on Form 1040 line 7a in the DA-yes-but-no-Schedule-D-activity year** (mine-and-hold / donate-and-hold),
where C2's literal "zero → `-0-`" stamps an unearned zero-capital-gains claim on the official 1040 while the
attached Schedule D line 16 is blank. One clarifying sentence clears it. Everything else is green.

---

## Per-fold verification (each fold: captured? consistent? evidence)

### C1 — SE line 12 = lines 10+11 (ss + medicare); line 13 = deductible_half; addl advisory — ✅ CORRECT
- Spec (lines 54–60, 118): `line 12 := ss + medicare` (NOT `SeTaxResult.total`); `line 13 := deductible_half`;
  loud advisory when `addl > 0`; KATs `schedule_se_line12_equals_ss_plus_medicare` +
  `schedule_se_line12_excludes_addl_medicare` (c1_lock: line 12 = 29,870.85 NOT 30,564.30).
- Code: `se.rs:160` `let total = ss + medicare + addl;` (INCLUDES addl) — so `total` is the wrong source. ✓
  `se.rs:42–45` doc: "Schedule SE line 13 = SS + regular Medicare only" (addl is a Form 8959 item). ✓
  c1_lock golden (`se.rs:258–265`): `ss = 21,836.40`, `medicare = 8,034.45` → `ss+medicare = 29,870.85`;
  `total = 30,564.30` (addl = 693.45). The spec's two numbers are exact. ✓
- PDF: `fSE-2025.pdf` line 12 text = **"Self-employment tax. Add lines 10 and 11."** (page-1 text) — addl
  Medicare appears nowhere on Schedule SE. Line 13 = "Multiply line 12 by 50% (0.50)". ✓
- Consistency: line 12 = ss+medicare and line 13 = deductible_half = round((ss+medicare)/2) are consistent
  by construction (line 13 == line 12 × 50%, no addl leak). ✓

### C2 — 1040 line 7a; fill iff line 16 ≥ 0; loss ⇒ blank + notice; 7b unchecked; notice says "line 7a" — ✅ CORRECT (but see I★1 for the zero/no-activity sub-case)
- Spec (lines 74–81, 119–120): amount field `f1_70`; fill 7a only when Schedule D line 16 ≥ 0; net loss ⇒
  7a BLANK + notice (§1211 line-21 cap is the filer's — consistent with SP1's 17–22 scope-out,
  `schedule_d.rs:5–6`); 7b UNCHECKED; notice text "line 7a".
- PDF: `f1040-2025.pdf` dump — `f1_70` = `[504,90,576,102]` (line 7a amount) ✓; 7b pair =
  `c1_43` `[153,80]` ("Schedule D not required") + `c1_44` `[261,80]` ("Includes child's capital gain or
  (loss)") ✓; page-1 text grep hits "7a" (×3), "Capital gain", "Schedule D not required",
  "child's capital gain". There is no bare "line 7" on the 2025 form. ✓
- Consistency: SP1 scopes out Schedule D lines 17–22 incl. the §1211 line-21 cap (`schedule_d.rs:5–6`,
  verified), so auto-filling an uncapped loss onto 7a would contradict the attached Schedule D — the
  blank-on-loss rule is the honest choice. ✓

### C3 — the per-form geometric oracle (THE flagged residual-Critical risk) — ✅ CLAIM TRUE + ORACLE SOUND
**(a) The claim "verify.rs's Row{n} grid model does NOT fit these forms" is TRUE — verified against source + PDFs:**
- `verify.rs:56–61` `row_num` does `rest[..end].parse::<u32>()` on the chars after `.Row`. 8283 rows are
  **`Row1A`/`Row1B`/`Row1C`/`Row1D`** (dump lines 9–48) → `"1A".parse::<u32>()` = `None` → every row
  dropped → `derive_bands` returns `Structure("no data-grid widgets")` (`verify.rs:83–87`). ✓ The 8283 grid
  is additionally split across **two** subforms (`Table_Line1_ColsA-C` + `Table_Line1_ColsD-I`) with
  heterogeneous per-row widget counts (ColsA-C nests `ColB[0].c1_2` checkbox + `f1_6` VIN) → the
  consistent-`ncols` assertion (`verify.rs:100–105`) would also break. ✓
- Schedule SE has **no `Table_` subform** — 27 flat leaves (dump confirms; only `Line5a_ReadOrder` /
  `Line8a_ReadOrder` wrappers). `derive_bands`/`column_x_bands` need a `table_token` (`schedule_d.rs:18`
  `Table_PartI`); SE has none. ✓
- 1040: only grid is `Table_Dependents` (irrelevant); `f1_70` is flat and the DA checkbox `c1_10` is
  `Geo::Check`, which `verify.rs:169` exempts from geometry — so the SP1 verifier as built would check
  *nothing* on the two filled 1040 cells beyond no-unmapped. ✓
  → The spec's decision to **own** a new per-form oracle (not "free-reuse the verifier") is correct and necessary.

**(b) Is the proposed oracle (column-x membership + ordinal-y descent + same-y pair + spacer/page pins) SOUND
and map-independent for these scattered layouts? — YES. I looked hard for a hole; the covering argument holds:**
- *Not circular.* Ordinal-y descent assigns the ordinal INDEX from the fixed logical sequence (line 2 is
  always position 0, …) — a map-independent fact — and looks up the FIELD via the map. When a map swap
  moves a value, the field-at-ordinal-i changes and its `cy` no longer descends → RED. (Same shape as SP1's
  `derive_bands`, where row order is geometry-derived; here the order is the fixed line sequence.)
- *SE is y-monotonic in the declared sequence* (2,3,4a,4c,6,8a,8d,9,10,11,12). Measured `cy` (dump):
  534,522,510,462,414,354,318,306,294,282,258 — strictly descending. ✓ (8a is the mid-column field at
  cy 354, correctly between 6@414 and 8d@318 — mixing columns is fine for a pure `cy` check.)
- *Coverage of ALL pairwise map swaps on SE* — the two limbs partition it:
  - cross-column swap (e.g. 12 amount ↔ 13 mid): caught by **column-x** — SE amount cluster x≈[504,576],
    mid cluster x≈[410,482] (measured; non-overlapping, 482 < 504). Every written SE field sits cleanly in
    exactly one cluster (12=f1_21 @504 amount; 13=f1_22 @410 mid; 8a=f1_14 @410 mid; rest @504). ✓
  - same-column swap (e.g. 10 ↔ 11, both amount): caught by **ordinal-y** — any two DISTINCT-`cy` fields in
    the sequence break strict descent on at least one side (adjacent → the pair inverts; non-adjacent →
    inversion at both ends). No two written SE fields share a `cy`. The ONLY written-but-not-in-sequence
    field is line 13 (single element), so there is no pair of written fields that is *both* same-column
    *and* both-outside-the-y-sequence — hence every swap trips column-x or ordinal-y. ✓ (Verified by
    enumeration: 8a↔13 (both mid) is caught because 8a's `cy` moving to 222 inverts 8a→8d.)
- *Same-y-pair predicate for the DA question is UNIQUE on the 2025 1040 — verified from the dump.* Scanning
  every page-1 `/Btn`: the only same-`y` pair whose on-states are exactly {`/1`,`/2`} at the TOP of the
  page is `c1_10[0]`=/1 @[518,497] + `c1_10[1]`=/2 @[554,497]. Competitors are excluded: the Presidential
  pair `c1_6`/`c1_7` @y=590 are BOTH `/1` (not {/1,/2}); the filing-status `c1_8` same-y rows are {/1,/4}
  and {/2,/5} (not {/1,/2}); `c1_9` @y=526 is a lone `/1`; the first dependents {/1,/2} pair
  (`c1_28`) is @y=380 — below 497, so "top-most" correctly skips it. "Yes = left member" holds (Yes/`c1_10[0]`
  @x=518 < No/`c1_10[1]` @x=554). ✓ Sound and map-independent.
- *8283 rows A→D descend* (ColsD-I `cy`: A=390 > B=378 > C=366 > D=354; ColsA-C: 516>492>468>444; Section-B
  3A>3B>3C: 84>72>60). ✓ (Impl caveat → M1.)

**(c) Is `no_unmapped_filled` genuinely reusable? — YES.** `verify.rs:225–242` is fully form-agnostic
(iterates `collect_fields`, buttons via `checkbox_on`, text via non-empty `text_value`, fails on any filled
field outside the placement set). No grid/table dependency. Correctly kept per-form. The one gap it does NOT
cover — a mis-map ONTO an *authorized* target — is exactly why M1 (spacer exclusion) + the geometric limbs
exist; the spec keeps all three. ✓

→ **C3 is correctly folded and the oracle is sound. No residual Critical here.** Two non-blocking impl notes
land as M1/M2.

### C4 — DA question YES iff evidenced activity; else blank BOTH + skip 1040; never fill No — ✅ CORRECT + implementable
- Spec (lines 82–88, 123): YES iff any `form_8949` row ∨ any `income_recognized` ∨ any Gift/Donate removal;
  else leave BOTH blank + skip the whole 1040 + note; never fill No.
- PDF: DA question text verified exact — "At any time during 2025, did you: (a) receive (as a reward, award,
  or payment for property or services); or (b) sell, exchange, or otherwise dispose of a digital asset…?"
  (`f1040_p1_text.txt`). Yes = `c1_10[0]` `/1` @[518,497]. ✓
- Wiring is reachable: `form_8949(state, year)` (disposals), `state.income_recognized` (clause (a) receipts),
  and `state.removals` filtered `RemovalKind::{Gift,Donation}`. **Important correctness point that the spec
  gets right:** `form_8283()` (`forms.rs:333–336`) emits **NO rows for Gifts** and donations flow through
  8283 (not `form_8949`), so the gift/donation disposal signal MUST come from `state.removals`, not from
  `form_8949`/`form_8283` — the spec's phrasing "any Gift/Donate removal" (a removals signal, distinct from
  the 8949/income signals) is the coherent choice. ✓

### I1 — full SE chain + thread `w2_ss_wages` — ✅ CORRECT (see M3 for a small completeness note on line 9)
- Spec (lines 61–66): fill 2,3,4a,4c,6,8a,8d,9,10,11,12,13 with the explicit BLANK set (A-checkbox,1a/1b,4b,
  5a/5b,8b/8c,Part II) and the 8a ≥ $176,100 skip. `w2_ss_wages` threaded via the fill signature.
- PDF line labels all confirm the chain: line 3 "Combine lines 1a, 1b, and 2"; 4a "×92.35%"; 6 "Add 4c and
  5b"; 8a "Total social security wages and tips (boxes 3 and 7 on Form(s) W-2)… If $176,100 or more, skip
  lines 8b through 10, and go to line 11"; 9 "Subtract line 8d from line 7… If zero or less, enter -0-";
  10 "Multiply the smaller of line 6 or line 9 by 12.4%"; 11 "×2.9%"; 12 "Add 10 and 11". ✓
- `w2_ss_wages` availability confirmed at the export site: `admin.rs:86–95` already reads
  `session.tax_profile(y)?.…p.w2_ss_wages` to feed `compute_se_tax` (in `export_snapshot`). Note the SE
  computation does **not yet exist in `export_irs_pdf`** (`admin.rs:151–199` fills only 8949 + Schedule D)
  — SP2 must add the same `tax_profile(y)` read there; the field is reachable exactly as in `export_snapshot`. ✓
- Self-consistency (no dangling line) matches SP1's Sch D 7/15/16 doctrine. ✓

### I2 — $400 floor skip — ✅ CORRECT
- Spec (lines 67–69, 124): SP2 skips Schedule SE when `base < $400` + note + KAT; FOLLOWUP for a core change.
- PDF line 4c text: **"Combine lines 4a and 4b. If less than $400, stop; you don't owe self-employment
  tax."** ✓ `compute_se_tax` has no $400 gate (`se.rs:118` returns `None` only when `net_se == 0`) — so the
  SP2-boundary skip is the right layer, and filing a FOLLOWUP for the core-semantics question is correct. ✓
  (Note the threshold is on **`base`** = round(net_se × 0.9235), i.e. line 4a/4c, not net_se — the spec
  says `base < $400`, which matches the form.) ✓

### I3 — 8283 Rev. 12-2025 specifics (k Digital assets /11, parts III/IV/V, 4+3 rows, mo/yr dates) — ✅ CORRECT, PDF-exact
- PDF: header "Form 8283 (Rev. December 2025)" / footer "Form 8283 (Rev. 12-2025)". ✓
- **"k Digital assets" checkbox = `Lines2i-l[0].c1_6[2]`, on-state `/11`** — the dump line is exactly
  `Form8283[0].Page1[0].Lines2i-l[0].c1_6[2] states=['/11']` @[425,230] and the page text lists option
  **"k Digital assets"** (with f = Securities /6, l = Other /12 flanking) → the "do NOT check f/l" caution is
  right. ✓ Property-type states map a→/1 … l→/12 (b(1) NPS = separate `c1_7`), so k = /11 is exact.
- Parts renumbered: Part **III** = "Taxpayer (Donor) Statement", Part **IV** = "Declaration of Appraiser",
  Part **V** = "Donee Acknowledgment" (page-2 text). ✓ Section A = 4 rows A–D; Section B Part I line 3 =
  3 rows A–C (text + dump). ✓ Dates "(mo., yr.)" (col (e) Section A; col (d) Section B). ✓
- The spec's claim that the **core** `DonationDetails` doc-comments use the OLD numbering is TRUE and the
  spec correctly does NOT inherit it: `donation.rs:18` "Donee organization name (Part IV…)" but the 2025
  donee is Part **V**; `donation.rs:26` "Qualified appraiser name (Part III…)" but appraiser is Part **IV**.
  The spec's fill scope (line 43–44) correctly assigns donee→Part V, appraiser→Part IV. ✓

### I4 — 8283 fill/blank scope + notice + overflow — ✅ CORRECT
- Fillable-from-data confirmed: `donation.rs:17–47` carries `donee_name/donee_ein/donee_address`,
  `appraiser_name/address/tin/ptin`, `appraisal_date` → Part IV appraiser identity + Part V donee identity
  can be prefilled honestly. ✓ Blank set (Part III taxpayer sig; Part IV appraiser SIGNATURE/date; Part V
  donee ACKNOWLEDGMENT — receipt date `f2_18`, unrelated-use `c2_4`, authorized sig/title/date; Part II
  `c2_1..c2_3`) is another-party's-declaration and correctly stays blank. ✓
- `needs_review` escalation: `forms.rs:426–430` exists (the spec's "forms.rs:426" cite is right). ✓
- Overflow: one row per `RemovalLeg` (`forms.rs:333`, verified) → a multi-lot donation overflows 4 Section-A
  / 3 Section-B rows immediately; header text "Attach one or more Forms 8283" sanctions `merge_copies`. ✓
- (Heads-up for T2 only — see N2: the round-1 appendix's tentative Part V donee cells need the label-anchored
  pass; the spec correctly does not pin them.)

### M1 — SE spacer exclusion — ✅ CORRECT. Line 7 `f1_13`=[575,384,576,396] (1 pt wide) and line 14
`f2_1`=[575,696,576,708] confirmed in the dump; both must be excluded from map targets and asserted
non-spacer in `map_2025_matches_bundled_pdf_fieldset` (rect width < ~2 pt). ✓

### M2 — SE None-reason discriminator (`se_net_income`) — ✅ CORRECT. `admin.rs:86–98` collapses
profile-absent and income-absent into a single `None` via the `.and_then` chain; `se.rs:55` `se_net_income`
is the exposed discriminator. Spec (lines 71–72) uses it so a miner WITH SE income but no stored profile gets
a NOTE, not a silent skip. ✓

### M3 — no `/TU`, label-anchored extraction + fieldset counts — ✅ FOLDED (lightly). The spec routes this
through `map_2025_matches_bundled_pdf_fieldset` per form + the C3 pins; leaf counts (199/27/117) reconfirmed.
Adequate. (The label-anchored extraction method is named for 8283/1040 identification.) ✓

### N1 / N2 — KAT renames + revision-dating — ✅ CORRECT. KATs renamed throughout (C1/C2/C4);
notice says "line 7a". Revision string "Rev. 12-2025" recorded (line 128) with the SP3 note that 8283 is
revision- not year-editioned. ✓

---

## NEW finding introduced/left by the rewrite

## Important

### I★1 — Form 1040 line 7a is not self-consistent with the attached Schedule D in the DA-yes / no-Schedule-D-activity year: C2's literal "zero → `-0-`" stamps an unearned zero-capital-gains claim on the official 1040 for a mine-and-hold (or donate-and-hold) filer, while the attached Schedule D line 16 is blank.

**The gap.** C4 makes the 1040 *present* whenever there is any reportable activity, where activity =
`form_8949` ∨ `income_recognized` ∨ Gift/Donate removal. So an **income-only** year (a miner who receives
mining rewards and holds — a first-class user of this app) or a **donation-only** year is DA = YES and
1040-present with **no capital-gains disposals**. For that year:
- `schedule_d(state, year)` returns `ScheduleDTotals::default()` — all-zero — because it is **non-`Option`**
  (`forms.rs:172`), so the 1040 code computes `line 16 = st.gain + lt.gain = 0`.
- C2 as written (spec lines 76–77): "Fill 7a ONLY when Schedule D line 16 ≥ 0 (gain → line 16; **zero →
  '-0-'**)". Read literally with `line 16 = 0`, this **fills 7a = `-0-`**.
- But `schedule_d.rs`'s own fill (`fill_schedule_d_totals`, `active()` gate at lines 25–27, 62–110) writes
  Schedule D **line 16 only when `st_active || lt_active`** — so on the attached Schedule D, **line 16 is
  BLANK** for this year.

→ The 1040 would show `7a = -0-` while its own attached Schedule D shows line 16 blank: **self-inconsistent**
(this is exactly the "line 16 filled while its feeders are blank" defect SP1's I1/I4 established the fill layer
must not create), **and** `-0-` on the aggregate 7a is an affirmative "your total capital gain is zero" claim
btctax cannot know (the filer may have non-crypto capital gains) — the same honesty problem C4 solved for the
"No" box. It is on one of exactly **two** cells btctax fills and vouches for, so the partial-scope-notice
"everything else is the filer's" does not cover it.

**Symptom of under-specification, not a hard contradiction, but the artifact-as-written is wrong here:** the
Schedule-D instruction the spec quotes ("if line 16 is zero, enter -0-") is written for the case where
Schedule D *is filed with transactions that net to zero* — not "no capital activity". The Plan T3 phrase
"gain-only 7a" hints the authors may already intend blank-on-no-activity, but C2's body says "zero → -0-";
the two must be reconciled explicitly.

**Fix (one sentence).** Fill 7a **iff Schedule D is active (`st_active || lt_active`) AND line 16 ≥ 0**
(gain → the amount; active-and-netted-to-zero → `-0-`, mirroring the *filled* Schedule D line 16); otherwise
leave 7a **BLANK** even when the DA question is YES from income/gift/donation. Reserve `-0-` for the
active-netted-to-zero case so 7a always mirrors the attached Schedule D line 16. Add a KAT
`form_1040_line7a_blank_when_no_schedule_d_activity` (income-only fixture: DA = YES, 7a blank).

---

## Minor

### M1 — 8283 ordinal-y descent must be applied PER COLUMN, because Section A's logical rows A–D are split across two subforms at different y-bands.
Section A `Table_Line1_ColsA-C` rows sit at y≈432–528 and `Table_Line1_ColsD-I` at y≈348–396 (dump). A naive
"cy strictly descending over all Section-A writes in logical order" is **not** monotonic by row (all ColsA-C
widgets are above all ColsD-I widgets). The oracle must assert descent **within each written column's cluster**
(row A→D per column), not across columns. Same shape as SE (per-column implicitly). Regular geometry, fully
derivable — an impl note for T0/T2, not a design change. Say it in the spec so T2 doesn't build a cross-column
descent that fails on a correct map.

### M2 — the SE/1040 column-x oracle's logical-line→cluster assignment is a hand-pinned table (weaker independence than SP1's grid-derived ordinal columns); the fault-inject KAT should prove BOTH oracle limbs.
Unlike `derive_bands` (which derives column ORDER purely by x-sorting a known grid), the scattered-field
oracle must carry an a-priori "line 12 → amount cluster, line 13 → mid cluster" table in the verify fn (like
`schedule_d.rs`'s `SD_COL_D=0`). Its correctness rests on the one-time golden / manual check — the same trust
level as SP1, acceptable — but the spec's "swap-two-map-entries ⇒ RED" KAT should **explicitly exercise both**
(i) a cross-column swap (SE 12 ↔ 13 → **column-x** RED) and (ii) a same-column swap (SE 10 ↔ 11 → **ordinal-y**
RED), so both limbs are proven and a future edit can't silently disable one. Also state that ordinal-y runs
over the **actually-written subset in logical order** (conditional omissions: the 8a ≥ $176,100 case blanks
9/10, so the written subsequence is 2,3,4a,4c,6,8a,8d,11,12 and must still be checked descending).

### M3 — SE line 9 (= line 7 $176,100 − line 8d) needs the year's `ss_wage_base` constant, not only `w2_ss_wages`.
I1 threads `w2_ss_wages`, but line 9 = 176,100 − 8d also consumes the preprinted line-7 wage base
(`f1_13` spacer, not filled). The `year` param in the I1 signature can source it (from the bundled `TaxTable`
`ss_wage_base` — already loaded at the export site, `admin.rs:85` — or as a per-year map constant), so this is
resolvable, but the spec's I1 fill-set should note line 9's second input so the implementer supplies 176,100
rather than trying to recover it from `SeTaxResult` (where the min/round has already destroyed `ss_cap`).

---

## Nit

### N1 — C4's parenthetical "(reward/mining/staking/airdrop = clause (a))" reads like an exhaustive kind-filter; the operative predicate is "any `income_recognized`".
Clarify that the DA trigger is **any** `income_recognized` entry regardless of `IncomeKind` — including
`Interest` (crypto received as interest is a digital-asset receipt → clause (a)/(b) YES). The four listed
kinds are examples, not a whitelist; a narrowed filter would under-answer the perjury question.

### N2 — the round-1 appendix's tentative Part V donee cells (f2_15 / f2_16) actually fall in the Part IV appraiser business-address block; the spec correctly defers exact cells, so this is only a heads-up for T2.
Page-2 text places the Part IV appraiser "Business address … Identifying number / City…ZIP" at the
f2_15/f2_16/f2_17 region (y≈186–222) and the Part V "Name of charitable organization (donee) / EIN /
Address / Authorized signature" lower, in the f2_19–f2_22 region (y≈36–96). The spec itself does NOT pin
these cells (it defers to the label-anchored pass at T2) — no spec defect — but T2 must run that pass rather
than trust the appendix's `f2_15=donee` guess. Defended anyway by `map_2025_matches_bundled_pdf_fieldset` +
the 8283 oracle + `no_unmapped_filled`.

---

## Answers to the round-2 charter

- **C1:** ✅ correct + PDF-verified (line 12 "Add lines 10 and 11"; `total` includes addl; c1_lock 29,870.85
  vs 30,564.30 exact).
- **C2:** ✅ correct (7a = `f1_70`; 7b pair `c1_43/c1_44`; "line 7a" text; loss-blank right) — with the
  zero/no-activity sub-case pulled out as **I★1**.
- **C3:** ✅ **the flagged residual-Critical does NOT materialize.** (a) the "Row{n} doesn't fit" claim is
  TRUE (8283 `Row1A` fails `parse::<u32>`, split tables; SE has no table; 1040 cell is `Geo::Check`-exempt);
  (b) the oracle is **sound** — ordinal-y descent is non-circular and, together with column-x membership,
  covers every pairwise map swap on SE (proven by enumeration; the only written-but-unsequenced field, line
  13, cannot form an uncatchable same-column pair), and the DA same-y {/1,/2} pair is uniquely the top-most
  such pair on the 2025 1040 (all competitors excluded by state-set or y); 8283 rows descend A→D; (c)
  `no_unmapped_filled` is genuinely form-agnostic and reusable. Two impl notes → M1/M2.
- **C4:** ✅ coherent + implementable; the gift/donation signal correctly comes from `state.removals`
  (`form_8283` emits no gift rows), not from `form_8949`.
- **I1–I4:** ✅ all folded; `w2_ss_wages` reachable at the export site (must be ADDED to `export_irs_pdf`,
  which today fills only 8949+SchedD); 8283 k-box `/11`, parts III/IV/V, 4+3 rows, mo/yr dates all
  PDF-exact; fill/blank scope + overflow + `needs_review` escalation correct. (Small completeness note M3.)
- **M1–M3 + N1–N2:** ✅ folded (spacer exclusion PDF-confirmed; None-reason discriminator; revision-dating).
- **Self-consistency / NEW gaps:** one NEW residual (**I★1**, 7a in the no-activity DA-yes year). The Plan
  (T0 engine oracle → T1 SE → T2 8283 → T3 1040) is implementable with a clean prerequisite chain — T0 is
  unit-testable against the blank PDFs before any fill, and the T1–T3 fault-inject KATs correctly depend on
  the T0 oracle. No blocking open question **other than I★1**.

## Bottom line
`80ba633` is a faithful, PDF-verified fold of the round-1 4C/4I/3M/2N — the C3 oracle (the one place a
residual Critical could have hidden) is sound. **One clarifying sentence on line 7a (I★1) stands between this
and R0-GREEN.** Fold I★1 (and, cheaply, M1/M2/M3), then re-review (the fold must itself pass a round-3 pass on
the changed lines). **Verdict: 0 Critical / 1 Important / 3 Minor / 2 Nit — NOT YET R0-GREEN.**
