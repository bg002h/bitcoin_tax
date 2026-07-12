# Fable Verify 02 — Adversarial confirmation of deep/01 (Tax Table · TCW · half-up rounding · QDCGT)

**Agent:** Fable recon, round 2, VERIFY-ONLY (adversarial).
**Target:** `../deep/01-tax-table-qdcgt-method.md` (the locked method) + `../02-computation-worksheets.md` §4/§6.
**Source:** re-downloaded `irs.gov/pub/irs-prior/i1040gi--2024.pdf` (verified: "2024 Instruction 1040", 113 pp) and
extracted the actual pages with `pdftotext -layout`; every cell below was read from that fresh copy, not from deep/01.
Engine cross-checks read from current source: `crates/btctax-core/src/tax/compute.rs`,
`crates/btctax-core/src/conventions.rs`, `crates/btctax-adapters/src/tax_tables.rs`.

---

## 0. VERDICT

**CONFIRMED — the locked method survives adversarial re-verification.**
`line16 = round_dollar(min(L23, L24))` with independent per-amount Table/TCW selection, HALF-UP
`round_dollar`, year-independent bin reconstruction, and the `PrefSplit` ↔ QDCGT L6–L21 mapping are all
correct against the primary source. All three worked examples re-derive to the cent.

Two genuine (non-method-breaking) findings escalate, plus three footnotes:

- **F-A (Important, one-line fix in the seam):** the doc's stated engine call
  `preferential_tax(bp, L5, L2+L3)` diverges from the worksheet when **L4 > L1** (pref > TI). The
  worksheet caps preferential at **L10 = min(L1, L4)**; the doc's call does not. `qdcgt_line16` must pass
  `pref = min(L1, L2+L3)`. (§5.1)
- **F-B (commentary correction + missing KAT):** deep/01's claim that `min(L23, L24)` is "non-binding for a
  common household" is **false** — when L1 and L5 land in the *same* $50 bin the min binds and changes the
  rounded line 16. The locked method already computes the min, so results stay correct; but the min is
  load-bearing, not "form fidelity," and deserves a same-bin KAT. (§5.2)
- Footnotes: rounding-rule citation is **p. 23, not p. 22** (§5.3); the $9.75 gap ceiling is valid but loose —
  the TY2024-achievable max is **$6.00**, hit exactly by example (c) (§5.4); the "round every line vs. carry
  cents" worksheet ambiguity is real (±$1) and the doc's convention is the defensible literal reading —
  document as decided (§5.5).

---

## 1. Falsification attempt: HALF-UP rounding — FAILED TO FALSIFY (it is half-up)

Mandate: find any IRS text or Tax-Table cell where half-up is wrong. Result: **none exists**; instead I found
**two additional discriminating cells** beyond deep/01's pair, plus a structural whole-region proof.

### 1.1 The rule text (read verbatim, printed/PDF p. 23 — NOT p. 22 as deep/01 cites)

> "You can round off cents to whole dollars on your return and schedules. If you do round to whole dollars,
> you must round all amounts. To round, drop amounts under 50 cents and increase amounts from 50 to 99 cents
> to the next dollar. For example, $1.39 becomes $1 and **$2.50 becomes $3**. If you have to add two or more
> amounts to figure the amount to enter on a line, include cents when adding the amounts and round off only
> the total."

"$2.50 becomes $3" is half-away-from-zero. (Note the rule is *elective* but all-or-nothing — "You **can**
round … you **must** round all amounts." The app's choice to round everything is the standard reading; the
Tax Table's whole-dollar bins presuppose it.)

### 1.2 Discriminating cells — every one read from the printed 2024 table, arithmetic shown

Midpoint tax = `ordinary_tax_on(sched, midpoint)` using the engine's TY2024 schedules (verified identical to
the TCW boundaries, §3). "HE" = half-even (the engine's `round_cents` mode, `conventions.rs:13`).

| Printed cell (row × col) | Printed value | Midpoint | Exact midpoint tax | Half-up | Half-even |
|---|---|---|---|---|---|
| all cols **[3,000, 3,050)** | **303** | 3,025 | 10% × 3,025 = **302.50** | **303** ✅ | 302 ❌ |
| MFJ **[11,600, 11,650)** | **1,163** | 11,625 | 10% × 11,625 = **1,162.50** | **1,163** ✅ | 1,162 ❌ |
| Single **[58,000, 58,050)** | **7,819** | 58,025 | 1,160 + 4,266 + 22%×10,875 = **7,818.50** | **7,819** ✅ | 7,818 ❌ |
| Single **[60,000, 60,050)** | **8,259** | 60,025 | 1,160 + 4,266 + 22%×12,875 = **8,258.50** | **8,259** ✅ | 8,258 ❌ |
| **NEW** — MFJ **[99,950, 100,000)** (the table's LAST row) | **12,101** | 99,975 | 2,320 + 8,532 + 22%×5,675 = **12,100.50** | **12,101** ✅ | 12,100 ❌ |

Rows 3–4 are the two cells examples (c) uses — deep/01's examples are themselves discriminators. The new
last-row cell means the half-up property holds at both ends of the table.

### 1.3 Structural proof (stronger than any single cell)

Every $50 bin inside a 10% bracket has midpoint tax `lo/10 + 2.50` — **a .50 tie in every single row**. The
printed table steps uniformly **+5** across that whole region (303, 308 … 603, 608 … 903, 908 …). Under
half-even the values would alternate ties-to-even (302, 308, 312, 318 — steps +6/+4). The uniform +5
staircase proves half-up across hundreds of consecutive rows, not sampled cells. **CORRECTION 1 of deep/01
is confirmed beyond doubt; `round_dollar` (MidpointAwayFromZero) is mandatory.**

---

## 2. Bin structure & midpoint rule — CONFIRMED (verbatim, incl. every low bin)

Read from the table's first page (printed p. 64: heading, sample-table example, QSS footnote all verbatim as
deep/01 quotes them) through its last (p. 75, ending at "$100,000" → TCW):

- **Sub-$25 special bins exactly [0,5)=0, [5,15)=1, [15,25)=2.** Midpoints 2.5/10/20 → 10% = 0.25/1.00/2.00 →
  0/1/2 ✅ (0.25 rounds *down* under half-up — not a tie).
- **$25 bins from 25 to 3,000:** [25,50)=4, [50,75)=6, [75,100)=9, [100,125)=11, [125,150)=14 …
  [975,1,000)=99 … [2,975,3,000)=299 — every one = `round_dollar(10% × (lo+12.5))` ✅.
- **$50 bins from 3,000 to 100,000:** [3,000,3,050)=303 through [99,950,100,000)=17,048/12,101/17,048/15,354 ✅.
- **Bracket-edge alignment:** the Single 22% edge (47,150) and MFJ 22% edge (94,300) land ON bin edges;
  the straddling rows verify the midpoint formula right across the kink:
  Single [47,100,47,150)=5,423 (= 1,160+12%×35,525 exactly) then [47,150,47,200)=5,432
  (= 5,426 + 22%×25 = 5,431.50 → half-up) ✅; MFJ [94,250,94,300)=10,849 then [94,300,94,350)=10,858
  (10,857.50 → half-up) ✅.
- **deep/01 §1.3 algorithm edge-tested:** `bin_midpoint` at ti = 0, 4, 5, 24, 25, 2,999, 3,000, 99,999 all
  reproduce the printed rows (e.g. ti=3,000 → mid 3,025 → 303; ti=2,999 → mid 2,987.5 → 299).

**REFINEMENT 2 (no per-year table data) — CONFIRMED, with one caveat to carry into the spec:** the
reconstruction works because every TY2024 bracket edge **below $100k** is a multiple of $50. That is a
**per-year contingent fact**, not structural: §1(f)(7) rounds some edges to $25-only (TY2024's 100,525 and
243,725 are ×25, not ×50 — safely above the table's domain this year). **Recommend a per-year assertion/test
in `method.rs`: no bracket edge < $100,000 may fall in a $50-bin interior** (and none < $3,000 in a $25-bin
interior). Cheap, and it converts the caveat into a checked invariant for every future year table.

Also verified: `ordinary_tax_on`'s internal `round_cents` (half-even) is a **no-op on this path** — all
midpoint products are exact at ≤ 2 dp for TY2024 (fractional midpoints only ever meet the 10% rate), so no
half-even contamination occurs before `round_dollar`.

---

## 3. $100k boundary, TCW, and MFS column — CONFIRMED

- **p. 33 verbatim** ✅: "If your taxable income is less than $100,000, you **must** use the Tax Table … If
  your taxable income is $100,000 or more, use the Tax Computation Worksheet right after the Tax Table."
  Boundary inclusive at 100,000 for the TCW ✅.
- **TCW p. 76** ✅: all Section A–D `(b)`/`(d)` constants match deep/01 verbatim (Single 22%−4,947.00 /
  24%−6,957.50 / 32%−22,313.50 / 35%−29,625.25 / 37%−41,812.25; MFJ 22%−9,894.00; HoH 22%−6,641.00; plus MFS
  Section C ending 37%−36,937.25 over 365,600). All five §2.2 identity rows independently recomputed from the
  engine's bracket schedule: 17,053.00 / 29,042.50 / 41,686.50 / 75,374.75 / 217,187.75 — **`ordinary_tax_on`
  = TCW to the cent** ✅. The TCW's header **Note** explicitly instructs entering "the amount from that form or
  worksheet" (naming the QDCGT worksheet) in column (a) — primary-source support for the L22/L24 usage.
- **MFS column** ✅: four distinct printed columns; QSS footnote verbatim. MFS ≡ Single numerically in every
  sampled row including the extremes ([11,650,11,700): 1,169/1,168/**1,169**/1,168; last row
  17,048/12,101/**17,048**/15,354) — structurally forced since Single and MFS schedules coincide below
  100,525 > 100,000. CONFIRM 4 stands (implement per-status; identity is emergent, not assumed).
- **QDCGT worksheet p. 36** ✅: all 25 lines verbatim as deep/01 tabulates, including L3's
  "If either line 15 or line 16 is blank or a loss, enter -0-", L6 (47,025 / 94,050 / 63,000) and L13
  (518,900 / **291,850 MFS** / 583,750 / 551,350) matching the engine's `LtcgBreakpoints` exactly
  (`tax_tables.rs` — incl. MFS 291,850 ≠ 583,750/2 = 291,875), the **independent** L22/L24 $100k tests
  (each names its own line's amount), and L25 = "smaller of line 23 or line 24" → line 16.

---

## 4. The three worked examples — INDEPENDENTLY RE-DERIVED, all match to the cent

All four Tax-Table cells the examples consume were verified against the printed table (not recomputed only):
MFJ [79,000,79,050) = **9,019**, MFJ [85,000,85,050) = **9,739**, Single [58,000,58,050) = **7,819**,
Single [60,000,60,050) = **8,259**.

- **(a) MFJ TI 85,000, QD 6,000:** L5=79,000; L9=6,000 @0% (85,000 < 94,050); L18=L21=0;
  L22=table(79,000)=9,019 (midpoint 79,025 → 2,320+12%×55,825 = 9,019.00); L23=9,019.00;
  L24=table(85,000)=9,739 (midpoint 85,025 → 9,739.00); **line 16 = 9,019** ✅.
  Delta gap: `ordinary_tax_on(MFJ, 79,000)` = 9,016.00 → **+$3** ✅.
  `preferential_tax(bpMFJ, 79000, 6000)` = {6,000, 0, 0, 0.00} ✅.
- **(b) Single TI 120,000, LTCG 20,000:** L5=100,000 → **TCW (inclusive edge)**: 22,000−4,947 = 17,053.00;
  L9=0; L17=20,000; L18=3,000.00; L23=20,053.00; L24=TCW(120,000)=28,800−6,957.50=**21,842.50** (cents
  carried); **line 16 = 20,053** ✅. round_dollar(21,842.50)=21,843 half-up as stated ✅.
  `preferential_tax(bpS, 100000, 20000)` = {0, 20,000, 0, 3,000.00} ✅.
- **(c) Single TI 60,000, QD 2,000, net-loss year:** L3=0 per the verbatim blank/loss rule; L5=58,000;
  L9=0 (58,000 > 47,025); L17=2,000; L18=300.00; L22=table(58,000)=**7,819** (7,818.50 half-up);
  L23=8,119.00; L24=table(60,000)=**8,259** (8,258.50 half-up); **line 16 = 8,119** ✅.
  Delta gap: `ordinary_tax_on(S, 58,000)` = 7,813.00 → **+$6** ✅ (this is the maximal TY2024 gap, §5.4).
  Both cells are half-even discriminators (would print 7,818/8,258) — example (c) doubles as a rounding KAT.

Additionally proved (case analysis over the worksheet algebra): for **L4 ≤ L1**, `preferential_tax(bp, L5,
L4)` ≡ QDCGT L6–L21 exactly — {at_0 = L9, at_15 = L17, at_20 = L20, tax = L18+L21} in every branch (B vs
max_zero/max_fifteen orderings, clamps included). The identity's one failure mode is F-A below.

---

## 5. Findings

### 5.1 F-A — pref > TI corner: the stated engine call diverges from the worksheet (Important; one-line fix)

deep/01 §4.1 asserts "`preferential_tax(bp, bottom=L5, pref=L2+L3)` returns exactly {at_0=L9, …}". **False
when L4 > L1** (possible when deductions exceed ordinary income — e.g. low wages + large QD). The worksheet
caps the preferential amount at **L10 = min(L1, L4)**; the stated call passes the *uncapped* L2+L3.

Counterexample: TI (L1) = 35,400, QD = 50,000, LTCG = 0 → L5 = 0. Worksheet: L9 = 35,400 @0%, L12 = 0,
L17 = L20 = 0, L22 = tax(0) = 0 → **L23 = 0 → line 16 = $0**. As-mapped engine:
`preferential_tax(bp, 0, 50000)` = {at_0: 47,025, at_15: 2,975, tax: **446.25**} → line 16 = **$446**
(silently overstated). Fix in the `qdcgt_line16` seam: `pref_ws = min(L1, qd + net_ltcg)` (with
`bottom = L5 = clamp(L1 − (qd+net_ltcg))`; note `L5 + pref_ws = min(L1, L1) = L1` holds in both regimes).
Rare for a "Common W-2 household" but reachable, silent, and the fix is one clamp. Add a KAT.
(The delta engine is unaffected — its `bottom`/`pref` arguments have no TI-cap semantics; `compute.rs:348`.)

### 5.2 F-B — the min(L23, L24) DOES bind for common households (commentary correction + KAT)

deep/01 §4.1: "The `min(L23,L24)` is **non-binding for a common household** (L23 ≤ L24 always …) but is
computed for form fidelity." **The parenthetical is false.** When L1 and L5 fall in the **same $50 bin**
(pref ≤ ~49 whole dollars), L22 = L24 (identical table cell) while L23 = L24 + 15%·pref > L24 — the min
selects L24, and for pref ≥ $4 this changes the *rounded* line 16. Example: L5 = 58,000, QD = 10, L1 = 58,010
(same bin): L23 = 7,819 + 1.50 = 7,820.50 → 7,821 unrounded-min-less; L24 = 7,819; correct line 16 =
round_dollar(min) = **7,819**. The rate-inequality argument only holds for exact-formula tax; bin
quantization breaks it. **The LOCKED METHOD is unaffected** (it computes the min — which this proves is
load-bearing). Correct the commentary; add a same-bin KAT so no future "optimization" drops the min.
(Also verified: round-after-min ≡ min-after-round by monotonicity, so the locked ordering is safe.)

### 5.3 F-C — citation: the rounding rule is on p. 23, not p. 22

Verified by page footer: "Rounding Off to Whole Dollars" prints on **p. 23** of the 2024 i1040 (both printed
and PDF numbering; the PDF's printed page = PDF page throughout). deep/01 cites p. 22 in §0/§3.1/§5/§6.
All other page cites verified correct: line-16 rule p. 33 ✅, QDCGT p. 36 ✅, Tax Table pp. 64–75 ✅, TCW p. 76 ✅.
Text quoted is verbatim-correct; only the page number is off.

### 5.4 F-D — the table-vs-formula gap ceiling: $9.75 is valid but loose; TY2024 max is $6.00

deep/01 REFINEMENT 3 bounds the gap by ½·$50·37% + $0.50 = $9.75 using the top statutory rate. The table's
domain (< $100k) never sees a marginal rate above **22%** in TY2024 (every status's 24% bracket starts
≥ $100,500), so the achievable ceiling is ½·$50·22% + $0.50 = **$6.00** — attained exactly by example (c)
(ti at bin bottom: 7,819 − 7,813.00). Harmless overstatement; tighten if the number is ever load-bearing
(e.g. in a tolerance assertion).

### 5.5 F-E — worksheet-internal rounding ambiguity: the doc's convention is defensible; document it as decided

The p. 23 rule supports two readings inside the QDCGT worksheet: (i) round every entered line ("you must
round all amounts" — what hand-filers and much commercial software do), or (ii) carry cents and round once at
L25 (deep/01 §3.2, reading "include cents when adding … round off only the total" as governing L23/L25).
These genuinely diverge by up to ~$1 in reachable cases (e.g. L18 = 300.60, L21 = 200.60: (i) 301+201 vs
(ii) 501.20 → differ by $1 on line 16). There is no more-specific IRS authority; a $1 line-16 delta is within
IRS math tolerance. **No correction** — but the spec should state convention (ii) explicitly as a decided
interpretation so a future reviewer doesn't flip it silently.

---

## 6. What was checked and found CLEAN (explicit no-findings list)

- Half-up rounding: could not falsify (5 printed discriminator cells + whole-region structural proof, §1).
- Bin edges: exact sub-$25 bins ([0,5)/[5,15)/[15,25)), $25-to-$3,000, $50-to-$100,000 — all verbatim; the
  $3,000 and $25 transitions land where deep/01 says; midpoint rule reproduces ~25 sampled cells including
  both ends of the table and both sub-$100k bracket-edge straddles.
- $100k rule verbatim, inclusive at 100,000; L22/L24 independent tests verbatim; mixed Table/TCW within one
  worksheet is primary-source supported (TCW Note + worksheet text).
- TCW Sections A–D constants and the five-point `ordinary_tax_on` ≡ TCW identity.
- QDCGT all 25 lines verbatim; L6/L13 ↔ `LtcgBreakpoints` (incl. MFS 291,850); L3 loss rule; L25 → line 16.
- MFS-vs-Single column identity in every sampled row incl. the last; QSS ≡ MFJ footnote.
- All three worked examples, line-by-line, to the cent, with their table cells read from the printed table.
- Engine: `round_cents` = MidpointNearestEven (`conventions.rs:13`); `ordinary_tax_on` no-float exact-Decimal
  marginal formula with no half-even hazard on the midpoint path; `preferential_tax` ≡ L6–L21 for L4 ≤ L1;
  TY2024 schedules ≡ TCW boundaries for all four statuses.
