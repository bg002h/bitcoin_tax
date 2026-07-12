# Deep Recon 01 — Tax-Calculation Method LOCK (TY2024): Tax Table · TCW · Whole-Dollar Rounding · QDCGT ↔ Engine

**Agent:** Opus deep-dive round 2 (builds on `../02-computation-worksheets.md` §4/§6 and `../00-SYNTHESIS.md` §2 "Layer 0").
**Scope:** lock the *absolute* Form 1040 line-16 tax method to spec grade for **TY2024**, "Common W-2 household."
**All numbers below were read directly from the 2024 Form 1040 instructions PDF** (`irs.gov/pub/irs-prior/i1040gi--2024.pdf`, 113 pp): Tax Table pp. 64–75 (PDF pages), Tax Computation Worksheet p. 76, QDCGT Worksheet p. 36, line-16 "which method" p. 33, Rounding rule p. 22. Every cell/worksheet line quoted below is verbatim, and every algorithm was re-derived and checked against the quoted cells (see the verification block at the end of each section).

---

## 0. BLUF + corrections to the first pass

The first pass (02 §4/§6, SYNTHESIS §2) is **directionally correct**: `ordinary_tax_on` = the Tax Computation Worksheet to the cent; `preferential_tax`/`PrefSplit` = QDCGT lines 6–21; the absolute return needs the Tax Table (< $100k) + whole-dollar rounding; L22/L24 each independently cross $100k. All confirmed. Four sharpenings, two of which are **load-bearing CORRECTIONS**:

- **🚩 CORRECTION 1 — the rounding mode is HALF-UP, not the engine's HALF-EVEN.** The IRS whole-dollar rule is *"drop amounts under 50 cents and increase amounts from 50 to 99 cents to the next dollar… **$2.50 becomes $3**"* (p. 22) — i.e. **round-half-away-from-zero**. The engine's only rounding is `round_cents` = `MidpointNearestEven` (half-even, banker's). **Reusing `round_cents` mis-rounds real table cells.** Proof against the actual table: MFJ bin **[11,600, 11,650)** prints **1,163**; the midpoint tax is exactly **$1,162.50** → half-up = 1,163 ✅, half-even = 1,162 ❌. Same at Single **[3,000, 3,050)** = **303** (midpoint tax $302.50 → half-even would give 302). A new `round_dollar` (`MidpointAwayFromZero`) is **required**; it is not a stylistic choice.

- **🚩 REFINEMENT 2 — NO per-year "Tax Table bins" need to be bundled.** SYNTHESIS §5 phase-0 and 02 §6 imply new per-year bundled bin data. Not needed: the bin structure (special sub-$25 bins, $25 bins to $3,000, $50 bins to $100,000) is a **fixed, year-independent construction rule**; the table value is reconstructed from the *existing* per-year `OrdinarySchedule` (already in `TaxTable`) via `round_dollar(ordinary_tax_on(schedule, bin_midpoint))`. New code = one fixed bin function + `round_dollar`. New *data* = none.

- **REFINEMENT 3 — the table-vs-formula gap, precisely.** 02 §6 said "up to ≈ $25 × top-rate ≈ $8–9." Exact bound: within a $50 bin the exact-formula value can differ from the table value by up to (½·bin·marginal) + ½¢ whole-dollar rounding = **≤ $25 × 0.37 + $0.50 ≈ $9.75**; typical is $3–$6 (worked example (a) = **$3** at 12%; (c) = **$6** at 22%). Confirmed the magnitude, tightened the ceiling.

- **CONFIRM 4 — MFS column.** The Tax Table prints **four** distinct columns: *Single*, *Married filing jointly \** (the `*` footnote: *"This column must also be used by a qualifying surviving spouse"*), *Married filing separately*, *Head of household*. MFS does **not** share Single's column, but its values are **numerically identical to Single in every row of the 2024 table**, because TY2024 Single and MFS ordinary schedules coincide below $100,525 and the table stops at $100,000. (QSS ≡ MFJ; already handled by `TaxTable::key`.)

**LOCKED METHOD (one line):** `line16 = round_dollar( min( L23, L24 ) )`, where each of L22 (tax on L5) and L24 (tax on L1) independently = **Tax Table** (`round_dollar(ordinary_tax_on(sched, bin_midpoint(amt)))` if `amt < 100_000`) **or TCW** (`ordinary_tax_on(sched, amt)`, cents kept) if `amt ≥ 100_000`; L6–L21 = one `preferential_tax(bp, L5, L2+L3)` call; cents are carried through the worksheet and rounded **once** at L25.

---

## 1. IRS Tax Table (TY2024) — exact construction & algorithm

### 1.1 Column & filing-status layout (verbatim header, p. 64)
```
If line 15 (taxable income) is—        And you are—
At least | But less than | Single | Married filing jointly * | Married filing separately | Head of household
                                       Your tax is—
* This column must also be used by a qualifying surviving spouse.
```
- **"At least / But less than"** = a half-open bin **[lo, hi)** — you use the row where `lo ≤ line15 < hi`. Line 15 is always whole dollars, so lookup is exact.
- Four columns, one per status; **MFS is its own column** (§0 CORRECTION-4). QSS reads the MFJ column.
- Each column's value = that status's own rate schedule applied to the **bin midpoint**, rounded to a whole dollar.

### 1.2 Bin structure (verbatim edges, p. 64) — YEAR-INDEPENDENT
| Range | Bin width | Bins (examples read from the table) |
|---|---|---|
| $0 – $25 | irregular: **[0,5), [5,15), [15,25)** | tax = **0, 1, 2** |
| $25 – $3,000 | **$25** | [25,50)=4, [50,75)=6, [75,100)=9, [100,125)=11, [125,150)=14 … |
| $3,000 – $100,000 | **$50** | [3,000,3,050)=303, [3,050,3,100)=308 … [11,650,11,700)=1,169 … |
| ≥ $100,000 | (no table) | → Tax Computation Worksheet (§2) |

Midpoint of **[lo, hi)** = `(lo + hi)/2`: 2.5, 10, 20 for the three special bins; `lo + 12.5` for $25 bins; `lo + 25` for $50 bins.

### 1.3 THE ALGORITHM (reproduces the official cell from taxable income `ti`, whole dollars, `ti < 100_000`)
```
bin_midpoint(ti):                      # year-independent
    if ti <   5: return 2.5
    if ti <  15: return 10
    if ti <  25: return 20
    if ti < 3000: return (ti // 25)*25 + 12.5     # $25 bins
    return (ti // 50)*50 + 25                      # $50 bins
tax_table(schedule, ti) = round_dollar( ordinary_tax_on(schedule, bin_midpoint(ti)) )
    where round_dollar = round to integer, HALF-AWAY-FROM-ZERO   # "$2.50 → $3"
```
`ordinary_tax_on` already yields the cent-exact marginal-formula value at the midpoint; since every TY2024 bracket edge (11,600 / 47,150 / 23,200 / 94,300 / 16,550 / 63,100 …) is a multiple of $50, **no bracket boundary ever falls in a bin interior**, so the marginal rate is constant across each bin and the midpoint value is exact to ≤ 2 dp — `round_dollar` then reproduces the printed cell.

### 1.4 VERIFICATION — cells quoted from the actual 2024 table, all reproduced
| Cell (row × column) | Quoted official value | midpoint | exact | `round_dollar` (half-up) | half-even (engine) |
|---|---|---|---|---|---|
| MFJ **[25,300, 25,350)** (the p.64 worked example) | **2,575** | 25,325 | 2,575.00 | **2,575** ✅ | 2,575 |
| Single **[25,300, 25,350)** | **2,807** | 25,325 | 2,807.00 | **2,807** ✅ | 2,807 |
| HoH **[25,300, 25,350)** | **2,708** | 25,325 | 2,708.00 | **2,708** ✅ | 2,708 |
| Single **[11,650, 11,700)** | **1,169** | 11,675 | 1,169.00 | **1,169** ✅ | 1,169 |
| **MFJ [11,600, 11,650)** | **1,163** | 11,625 | **1,162.50** | **1,163** ✅ | **1,162 ❌** |
| **Single [3,000, 3,050)** | **303** | 3,025 | **302.50** | **303** ✅ | **302 ❌** |
| Single low bins [0,5)/[5,15)/[15,25)/[25,50)/[50,75)/[75,100) | **0/1/2/4/6/9** | 2.5/10/20/37.5/62.5/87.5 | .25/1/2/3.75/6.25/8.75 | **0/1/2/4/6/9** ✅ | 0/1/2/4/6/9 |

The two **❌** rows are the whole proof that the table rounds **half-up** and that the engine's `round_cents` must **not** be reused for this path.

---

## 2. The $100,000 boundary and the Tax Computation Worksheet (TCW)

### 2.1 The rule (verbatim, p. 33, "Line 16 → Tax Table or Tax Computation Worksheet")
> *"If your taxable income is **less than $100,000, you must use the Tax Table** … If your taxable income is **$100,000 or more, use the Tax Computation Worksheet** right after the Tax Table."*

Boundary is **inclusive at $100,000** for the TCW (`< 100_000` → Table; `≥ 100_000` → TCW). Mandatory ("must use the Tax Table"), not optional — a filed return that prints a formula value where the table applies risks an IRS notice.

### 2.2 TCW = `taxable × rate − subtraction` = `ordinary_tax_on` to the cent (verbatim rows, p. 76)
Section A (Single) rows, columns `(a)=line15`, `(b)=rate`, `(d)=subtraction`, Tax = `(a)×(b) − (d)`:
```
At least $100,000 but not over $100,525   × 22% (0.22)   − $4,947.00
Over $100,525 but not over $191,950       × 24% (0.24)   − $6,957.50
Over $191,950 but not over $243,725       × 32% (0.32)   − $22,313.50
Over $243,725 but not over $609,350       × 35% (0.35)   − $29,625.25
Over $609,350                             × 37% (0.37)   − $41,812.25
```
(Sections B/C/D = MFJ+QSS / MFS / HoH with their own subtraction constants, e.g. MFJ 22%−$9,894.00, HoH 22%−$6,641.00 — read verbatim.)

**Identity check — TCW vs `ordinary_tax_on` (Single, one row per bracket):**
| line15 | TCW `(a)×(b)−(d)` | `ordinary_tax_on` exact | match |
|---|---|---|---|
| 100,000 | 22,000 − 4,947.00 = **17,053.00** | 17,053.00 | ✅ |
| 150,000 | 36,000 − 6,957.50 = **29,042.50** | 29,042.50 | ✅ |
| 200,000 | 64,000 − 22,313.50 = **41,686.50** | 41,686.50 | ✅ |
| 300,000 | 105,000 − 29,625.25 = **75,374.75** | 75,374.75 | ✅ |
| 700,000 | 259,000 − 41,812.25 = **217,187.75** | 217,187.75 | ✅ |

→ **At ≥ $100k the engine already matches the IRS to the cent.** The TCW's own "Tax" result, when entered on 1040 line 16 directly, is rounded to whole dollars (§3); when the TCW feeds a *worksheet* line (QDCGT L22/L24) the cents are kept (§4).

---

## 3. Whole-dollar rounding — exact spec

### 3.1 The rule (verbatim, p. 22, "Rounding Off to Whole Dollars")
> *"You can round off cents to whole dollars … If you do round to whole dollars, you must **round all amounts**. To round, **drop amounts under 50 cents and increase amounts from 50 to 99 cents to the next dollar**. For example, **$1.39 becomes $1 and $2.50 becomes $3**. If you have to add two or more amounts to figure the amount to enter on a line, **include cents when adding the amounts and round off only the total**."*

Two operative facts: (1) **half-up** away from zero (`$2.50→$3`), tax is always ≥ 0 so "away from zero" = "up"; (2) **round only the total** of a summed line — do not pre-round the addends.

### 3.2 Deterministic rounding spec for the absolute-return path
- **New helper** `round_dollar(v) = v.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)`. Distinct from `round_cents` (`MidpointNearestEven`). Live it next to `round_cents` in `conventions.rs` with the p. 22 cite.
- **Composition with the Tax Table:** the table already returns whole dollars (`round_dollar` is applied *inside* `tax_table`, §1.3); `round_dollar` is idempotent on a whole-dollar input, so re-applying it at a line entry is harmless.
- **Composition inside the QDCGT worksheet ("round only the total"):** carry **exact `Decimal` (cents)** on every worksheet line. L22/L24 are whole (table) or cents (TCW); L18/L21 carry cents; **round exactly once at L25** before it lands on 1040 line 16. Never pre-round L18/L21/L22/L23/L24. (This is the p. 22 "add with cents, round the total" rule applied to the worksheet whose only form-entered output is L25.)
- **Pure-ordinary returns** (no QD, no LTCG → QDCGT worksheet not required): line 16 = `round_dollar(tax_on_amount(sched, taxable))`, which is the table (whole) or `round_dollar(TCW cents)`.

---

## 4. QDCGT ↔ engine mapping — LOCKED (verbatim 25 lines, p. 36) + 3 cent-exact worked examples

### 4.1 The 25 lines (verbatim) with the engine equivalent
Breakpoints read verbatim from the worksheet and **matched to the bundled table**: L6 = $47,025 S/MFS · $94,050 MFJ/QSS · $63,000 HoH (= `LtcgBreakpoints.max_zero`); L13 = $518,900 S · $291,850 MFS · $583,750 MFJ/QSS · $551,350 HoH (= `max_fifteen`, incl. the MFS $291,850 that is *not* ½ of MFJ).

| L | Verbatim operation | Engine equivalent |
|---|---|---|
| 1 | line 15 taxable income (TI) | `taxable_income` (assembly) |
| 2 | line 3a **qualified dividends** | 1099-DIV box 1b |
| 3 | Sch D: `min(15,16)`, else -0- (**blank/loss → -0-**) | `net_1222(...).preferential_gain` |
| 4 | L2 + L3 | `qd + preferential_gain` (= `pref`) |
| 5 | L1 − L4, if ≤0 → -0- | `bottom` (= B) |
| 6 | 0% breakpoint | `bp.max_zero` |
| 7 | min(L1, L6) | — |
| 8 | min(L5, L7) | — |
| 9 | L7 − L8 = **taxed at 0%** | **`PrefSplit.at_0`** |
| 10 | min(L1, L4) | `pref` |
| 11 | = L9 | — |
| 12 | L10 − L11 | — |
| 13 | 15% breakpoint | `bp.max_fifteen` |
| 14 | min(L1, L13) | — |
| 15 | L5 + L9 | — |
| 16 | max(0, L14 − L15) | — |
| 17 | min(L12, L16) = **taxed at 15%** | **`PrefSplit.at_15`** |
| 18 | **L17 × 15%** | part of `PrefSplit.tax` |
| 19 | L9 + L17 | — |
| 20 | L10 − L19 = **taxed at 20%** | **`PrefSplit.at_20`** |
| 21 | **L20 × 20%** | part of `PrefSplit.tax` |
| 22 | *"tax on L5. If L5 < $100,000 use the Tax Table; if $100,000 or more use the TCW"* | **`tax_on_amount(sched, L5)`** (Table **or** TCW — its **own** $100k test) |
| 23 | L18 + L21 + L22 | `PrefSplit.tax + tax_on_amount(sched, L5)` |
| 24 | *"tax on L1. If L1 < $100,000 use the Tax Table; if $100,000 or more use the TCW"* | **`tax_on_amount(sched, L1)`** (its **own** $100k test) |
| 25 | **min(L23, L24)** → 1040 line 16 | `round_dollar(min(L23, L24))` |

**Confirmed:** `preferential_tax(bp, bottom=L5, pref=L2+L3)` returns exactly `{at_0=L9, at_15=L17, at_20=L20, tax=L18+L21}` (verified against `compute.rs` unit tests). **L22 and L24 each carry their own "< $100,000" test** (verbatim on the worksheet) — so within one worksheet L22 can be Table-based while L24 is TCW-based (whenever L5 < 100k ≤ L1). The `min(L23,L24)` is **non-binding for a common household** (L23 ≤ L24 always, since every preferential rate ≤ the ordinary rate on that slice) but is computed for form fidelity; the `capital-gain-excess`/Form 2555 second-worksheet interaction that can make it bind is **out of scope** (matches 02 §4b; Bitcoin never produces §1(h)(4)/(5) 28% or §1250 amounts — 02 §5 guard stands).

**⚠️ The core absolute-vs-delta divergence:** in the *delta* engine L22 = `ordinary_tax_on(sched, B)` (cent formula, no table). In the *absolute* return L22 must be `tax_on_amount(sched, L5)`, which for `L5 < 100k` is the **Tax Table**, not `ordinary_tax_on`. Examples (a)/(c) below show the resulting $3/$6 gaps — this is the entire reason Layer 0 exists.

### 4.2 Example (a) — MFJ, TI $85,000 incl. $6,000 qualified dividends (**Table path; min binds; QD at 0%**)
| L | value | | L | value |
|---|---|---|---|---|
| 1 | 85,000 | | 14 | 85,000 |
| 2 | 6,000 (QD) | | 15 | 85,000 |
| 3 | 0 | | 16 | 0 |
| 4 | 6,000 | | 17 | 0 |
| 5 | 79,000 | | 18 | 0.00 |
| 6 | 94,050 | | 19 | 6,000 |
| 7 | 85,000 | | 20 | 0 |
| 8 | 79,000 | | 21 | 0.00 |
| 9 | **6,000 @ 0%** | | 22 | **9,019** (Table, L5=79,000<100k; midpoint 79,025 → 9,019.00) |
| 10 | 6,000 | | 23 | 9,019.00 |
| 11 | 6,000 | | 24 | **9,739** (Table, L1=85,000<100k; midpoint 85,025 → 9,739.00) |
| 12 | 0 | | 25 | **min = 9,019 → line 16 = $9,019** |
| 13 | 583,750 | | | |

All $6,000 QD lands in the 0% band (TI $85,000 < MFJ 0% breakpoint $94,050); the $720 spread (9,739 − 9,019 = 6,000 × 12%) is the ordinary tax avoided. **Layer-0 gap:** delta-engine `ordinary_tax_on(MFJ, 79,000) = 9,016.00`; the **table** value on L22 = **9,019** → **+$3**. Engine wiring: `preferential_tax(bpMFJ, 79000, 6000) = {at_0:6000, at_15:0, at_20:0, tax:0.00}` ✅.

### 4.3 Example (b) — Single, TI $120,000 incl. $20,000 net LTCG (**TCW path on BOTH lines; $100k boundary inclusive**)
| L | value | | L | value |
|---|---|---|---|---|
| 1 | 120,000 | | 14 | 120,000 |
| 2 | 0 | | 15 | 100,000 |
| 3 | 20,000 (Sch D `min(15,16)`) | | 16 | 20,000 |
| 4 | 20,000 | | 17 | 20,000 |
| 5 | **100,000** (= exactly the $100k edge → TCW) | | 18 | **3,000.00** (20,000 × 15%) |
| 6 | 47,025 | | 19 | 20,000 |
| 7 | 47,025 | | 20 | 0 |
| 8 | 47,025 | | 21 | 0.00 |
| 9 | 0 | | 22 | **17,053.00** (TCW, L5=100,000≥100k: 100,000×22%−4,947) |
| 10 | 20,000 | | 23 | 20,053.00 |
| 11 | 0 | | 24 | **21,842.50** (TCW, L1=120,000: 120,000×24%−6,957.50) |
| 12 | 20,000 | | 25 | **min = 20,053.00 → round_dollar → line 16 = $20,053** |
| 13 | 518,900 | | | |

All $20,000 LTCG in the 15% band → $3,000. Demonstrates: **L5 = $100,000 exactly triggers the TCW** ("$100,000 or more"); **TCW carries cents** (L24 = 21,842.50) — had the min bound to L24 it would `round_dollar(21,842.50) = 21,843` (half-up). Engine wiring: `preferential_tax(bpSingle, 100000, 20000) = {at_0:0, at_15:20000, at_20:0, tax:3000.00}` ✅; L23 = `TCW(100000) + 3000 = 17,053 + 3,000 = 20,053`.

### 4.4 Example (c) — Single, TI $60,000 incl. $2,000 QD, **net-capital-loss year (L3 = 0, L2 > 0)**
Setup: a net capital **loss** of $8,000 → §1211(b) allows $3,000 against ordinary income (already inside TI = 60,000 via 1040 line 7 = −3,000), $5,000 carries forward; Sch D line 16 is a loss → **L3 = -0-** per the worksheet; QD = $2,000 still flows preferential on L2.
| L | value | | L | value |
|---|---|---|---|---|
| 1 | 60,000 | | 14 | 60,000 |
| 2 | 2,000 (QD) | | 15 | 58,000 |
| 3 | **0** (Sch D 16 is a loss → -0-) | | 16 | 2,000 |
| 4 | 2,000 | | 17 | 2,000 |
| 5 | 58,000 | | 18 | **300.00** (2,000 × 15%) |
| 6 | 47,025 | | 19 | 2,000 |
| 7 | 47,025 | | 20 | 0 |
| 8 | 47,025 | | 21 | 0.00 |
| 9 | 0 | | 22 | **7,819** (Table, L5=58,000<100k; midpoint 58,025 → **7,818.50 → 7,819**) |
| 10 | 2,000 | | 23 | 8,119.00 |
| 11 | 0 | | 24 | **8,259** (Table, L1=60,000<100k; midpoint 60,025 → 8,258.50 → 8,259) |
| 12 | 2,000 | | 25 | **min = 8,119 → line 16 = $8,119** |
| 13 | 518,900 | | | |

QD $2,000 taxed at 15% ($300) vs. 22% ordinary ($440) → $140 saved (8,259 − 8,119). Demonstrates: **L3 = 0 in a loss year while L2 flows preferential** (matches `preferential_tax` clamp + `net_1222.preferential_gain = 0`), **and the table's half-up rounding twice** (7,818.50 → 7,819; 8,258.50 → 8,259). **Layer-0 gap:** delta-engine `ordinary_tax_on(Single, 58,000) = 7,813.00`; table L22 = **7,819** → **+$6**. Engine wiring: the §1211 $3,000 sits in the upstream assembly (reduces TI before L1); `preferential_tax(bpSingle, 58000, 2000) = {at_0:0, at_15:2000, at_20:0, tax:300.00}` ✅.

---

## 5. Implementation seam (concrete)

**Reuse verbatim, do NOT touch (cent-precision crypto-delta path):** `ordinary_tax_on`, `preferential_tax`/`PrefSplit`, `net_1222`, `compute_tax_year`, `round_cents` — all in `crates/btctax-core/src/tax/compute.rs` + `conventions.rs`. The delta path stays exactly as is (its `ordinary_tax_on` cent formula is correct for a *difference*).

**Net-new (additive):**

1. **`crates/btctax-core/src/conventions.rs`** — add next to `round_cents`:
   ```rust
   /// IRS whole-dollar rounding (2024 i1040 p.22: "drop <50¢, 50–99¢ to next dollar; $2.50→$3").
   /// HALF-AWAY-FROM-ZERO — distinct from `round_cents` (half-even). Absolute-return path only.
   pub const DOLLAR_ROUNDING: RoundingStrategy = RoundingStrategy::MidpointAwayFromZero;
   pub fn round_dollar(v: Usd) -> Usd { v.round_dp_with_strategy(0, DOLLAR_ROUNDING) }
   ```

2. **NEW `crates/btctax-core/src/tax/method.rs`** (year-independent; the "tax method"):
   ```rust
   /// Year-independent IRS Tax Table bin midpoint (2024 i1040 pp.64–75 structure).
   fn bin_midpoint(ti: Usd) -> Usd;                       // §1.3 above
   /// Tax Table value for ti < 100_000 (asserts). round_dollar(ordinary_tax_on(sched, midpoint)).
   pub fn tax_table(sched: &OrdinarySchedule, ti: Usd) -> Usd;
   /// The line-16 "tax method": <100k → tax_table (whole); ≥100k → ordinary_tax_on (TCW, cents kept).
   /// Used for QDCGT L22/L24 (each with its OWN amount → its own $100k test).
   pub fn tax_on_amount(sched: &OrdinarySchedule, amt: Usd) -> Usd;
   /// Full QDCGT Worksheet (§4). Reuses preferential_tax for L6–L21; tax_on_amount for L22/L24;
   /// carries exact Decimal; round_dollar ONCE at L25. Refuse-guard: caller ensures no 28%/§1250.
   pub fn qdcgt_line16(table: &TaxTable, status: FilingStatus,
                       ti: Usd, qual_div: Usd, net_ltcg: Usd) -> Usd;   // returns whole-dollar line 16
   /// Pure-ordinary line 16 (no QD/LTCG): round_dollar(tax_on_amount(sched, taxable)).
   pub fn ordinary_line16(sched: &OrdinarySchedule, taxable: Usd) -> Usd;
   ```
   Register `pub mod method;` in `tax/mod.rs`; re-export from `lib.rs`.

3. **No change to `TaxTable` / `tax_tables.rs`.** The bins are a fixed rule (REFINEMENT-2); the existing per-year `OrdinarySchedule` + `LtcgBreakpoints` are the only data `method.rs` needs. `qdcgt_line16` pulls `sched = table.ordinary_for(status)` and `bp = table.ltcg_for(status)` (QSS→MFJ already handled).

**Why this seam:** every rate primitive is reused; the only genuinely new logic is (i) `round_dollar` (half-up), (ii) `bin_midpoint` (fixed), and (iii) the L22/L24 Table-vs-TCW selector + the L25 min/round — a thin assembly. The delta engine is untouched, so the two paths can't drift on the shared primitives.

**Test layering (for the plan):** Layer-1 KATs assert `tax_table`/`qdcgt_line16` against the **verbatim cells and worked examples above** (esp. the two half-even-fails: MFJ [11,600,11,650)=1,163, Single [3,000,3,050)=303 — fault-inject `round_cents` in place of `round_dollar` must turn them RED), the p.64 example (MFJ 25,300 → 2,575), the TCW identity rows (§2.2), and examples (a)/(b)/(c) line-by-line.

---

## 6. Primary sources (all read directly, TY2024)
- **2024 Form 1040 Instructions** (`irs.gov/pub/irs-prior/i1040gi--2024.pdf`): Tax Table pp. 64–75 (cells + bin edges + the p.64 MFJ $25,300→$2,575 example + column header/QSS footnote); **Tax Computation Worksheet p. 76** (Sections A–D, `(a)×(b)−(d)`); line-16 "Tax Table or Tax Computation Worksheet" **$100k rule p. 33**; **QDCGT Worksheet—Line 16 p. 36** (verbatim 25 lines incl. the L22/L24 independent $100k tests); **"Rounding Off to Whole Dollars" p. 22** (half-up, "$2.50 becomes $3", "round only the total").
- **Engine cross-checks:** `crates/btctax-core/src/tax/compute.rs` (`ordinary_tax_on`, `preferential_tax`→`PrefSplit`), `crates/btctax-core/src/conventions.rs` (`round_cents`=`MidpointNearestEven`), `crates/btctax-adapters/src/tax_tables.rs` `ty2024()` (breakpoints/brackets matched to QDCGT L6/L13 & TCW rows).
- **IRC:** §1(h) (0/15/20 stacking), §1(j)(2) (rate schedules), §1211(b)/§1222 (loss/netting), §1(h)(4)–(5)/§1(h)(1)(E) (28%/§1250 — the Schedule-D-Tax-Worksheet trigger, out of scope).
