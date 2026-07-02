# R0 architect review — SPEC_se_chunkB_expenses.md — round 1

**Artifact:** `design/SPEC_se_chunkB_expenses.md`
**Baseline verified:** HEAD `60f33c0` (merge of SE Chunk C) — matches the spec's stated baseline.
**Reviewer:** independent R0 (architect), 2026-07-01.
**Verdict: NOT GREEN — 0 Critical / 1 Important / 3 Minor / 2 Nit.** One fold + re-review required.

---

## 1. Citation verification against HEAD `60f33c0`

| Spec claim | Current source | Status |
|---|---|---|
| `se_net_income(state, year)` sums gross business non-Interest income | `crates/btctax-core/src/tax/se.rs:53-60` | ✓ exact |
| `compute_se_tax(state, year, status, table, w2_ss, w2_medicare)` post-Chunk-A | `se.rs:89-96` | ✓ exact |
| `SeTaxResult.net_se` doc: "no Schedule C expenses modeled — FOLLOWUP" | `se.rs:28` | ✓ exact |
| Render caveat "your actual SE tax is lower" at "~1122-1127 pre-Chunk-A — re-verify" | `render.rs:1126-1136` (caveat at 1133-1136) | drift, self-hedged — see N2 |
| Chunk-A §164(f) advisory (quantified, no OTI prescription) | `render.rs:1167-1180` | ✓ exact (mechanism text matches the spec's I3 framing verbatim) |
| `TaxProfile` post-Chunk-A W-2 fields, `#[serde(default)]` | `types.rs:54-60` | ✓ exact |
| Real-path negative validation + `--show` for W-2 fields | `main.rs:725-741` (Usage on negative), `main.rs:666-676` (`--show`) | ✓ exact |
| THREE call sites | `cmd/tax.rs:83-91`, `cmd/admin.rs:63-71`, `btctax-tui/src/tabs/tax.rs:97` | ✓ all three, all currently pass the two W-2 args |
| `crypto_ord` gross, kind/business-agnostic | `compute.rs:296-301` (sum over ALL `income_recognized` for the year, no `business`/`kind` filter beyond year) | ✓ exact |
| FOLLOWUPS pre-negotiation: Chunk B advisory-only, engine-B deferred (high blast radius); next queue item TY2024 tables | `FOLLOWUPS.md:24-26, 47-49, 70` | ✓ exact |

No un-hedged drift. Baseline claim (`60f33c0` = post A + C) confirmed via `git log`.

---

## 2. §1402(a) net-SE formula and ordering (spec Q1) — CORRECT

§1402(a): net earnings from self-employment = gross income from the trade or business **less
the deductions attributable to it**, i.e. Schedule C net profit (Schedule C line 31 →
Schedule SE line 2/3); the §1402(a)(12) deemed deduction (the ×92.35% factor, Schedule SE
line 4a) applies **to that net**. So the spec's order — `net_se = max(0, gross − expenses)`
FIRST, then `base = round_cents(net_se × 0.9235)` — is the statutory/Schedule-SE order.
Subtracting expenses after the factor would understate the expense benefit by 7.65% of
expenses; the spec gets it right.

The `max(0, ·)` floor is correct: a Schedule C loss produces no SE tax and never a negative
one, and with a single aggregate SE activity there is no cross-activity netting to preserve.
`net_se == 0 → None` (fully expensed → no SE tax) is the correct **computation** outcome —
but see I1 for what the current render does with that `None`.

## 3. Advisory mechanism (spec Q2, the I3 lesson) — CORRECT framing

Verified against `compute.rs:335-338`:

- `bottom_with = OTI + crypto_ord + with.ordinary_gain − with.loss_deduction`
- `bottom_without = OTI + without.ordinary_gain − without.loss_deduction`

`crypto_ord` enters the WITH leg only. A user edit "reduce `ordinary_taxable_income` by
$expenses" subtracts from **both** legs, so the reported delta
(`total_federal_tax_attributable`) changes only by the bracket-differential across the
shifted range — it cannot express "crypto ordinary income is $expenses lower," which is the
actual correction (a net-of-expenses `crypto_ord` in the WITH leg). The spec's refusal to
prescribe the OTI edit is correct, and it is exactly the mechanism framing already shipped
in the Chunk-A §164(f) advisory (`render.rs:1167-1180`). Quantify-without-prescribe
(first-order overstatement = marginal ordinary rate × $expenses, "coordinate on your actual
return", engine-B coordination deferred to FOLLOWUPS) is the right advisory. The "to first
order" hedge properly covers bracket-spanning and the second-order MAGI/NIIT effect of a
lower AGI. D3's advisory text contains no prescription. **Approved as specified.**

## 4. Golden re-derivations (spec Q3) — ALL THREE CONFIRMED EXACTLY

TY2025 Single, wage base $176,100, addl threshold $200,000, mining $100,000 unless noted.

**(a) Headline — expenses $20,000, no W-2:**
- net_se = 100,000 − 20,000 = **80,000** ✓
- base = round_cents(80,000 × 0.9235) = **73,880.00** (exact, no rounding) ✓
- ss = 0.124 × min(73,880, 176,100) = 0.124 × 73,880 = **9,161.12** (exact) ✓
- medicare = 0.029 × 73,880 = **2,142.52** (exact) ✓
- addl = 0.009 × max(0, 73,880 − 200,000) = **0.00** ✓
- total = 9,161.12 + 2,142.52 = **11,303.64** ✓
- deductible_half = round_cents(11,303.64 / 2) = **5,651.82** (exact, no HALF_EVEN tie) ✓

**(b) Fully expensed — mining $10,000, expenses $15,000:**
- net_se = max(0, 10,000 − 15,000) = 0 → **`None`** ✓ (correct: Schedule C loss → no SE tax).
  The **render** consequence is where the spec has a gap — see I1.

**(c) Composed — expenses $20,000 + w2_ss $150,000 + w2_medicare $150,000:**
- net_se = 80,000; base = **73,880.00** ✓
- ss_cap = max(0, 176,100 − 150,000) = 26,100; ss = 0.124 × min(73,880, 26,100) = 0.124 × 26,100 = **3,236.40** ✓
- medicare = **2,142.52** ✓
- addl_threshold = max(0, 200,000 − 150,000) = 50,000; over = 73,880 − 50,000 = 23,880;
  addl = 0.009 × 23,880 = **214.92** ✓
- total = 3,236.40 + 2,142.52 + 214.92 = **5,593.84** ✓
- deductible_half = (3,236.40 + 2,142.52)/2 = 5,378.92/2 = **2,689.46** (exact; correctly
  EXCLUDES addl — the C1 rule survives composition) ✓

All intermediate products are exact at ≤2dp; none of these goldens depends on a rounding
tie, so `round_cents` HALF_EVEN cannot shift any figure. All spec figures confirmed.

## 5. Composition ordering (spec Q4) — CORRECT, nothing missed

Schedule SE order: line 2/3 = Schedule C net (gross − expenses) → line 4a = ×92.35% → line
8a/9 W-2 SS wages reduce the cap → line 10 = 12.4% × min(line 6, cap). So the SS cap mins
against the **expensed** base — exactly what falls out of the spec's D2 (expenses upstream of
the unchanged cap logic at `se.rs:106-115`). Likewise Form 8959 Part II: the 0.9% threshold
is reduced (not below 0) by Medicare wages and applied to the expensed SE base
(`se.rs:122-138` unchanged downstream). Golden (c) exercises both. No ordering subtlety
missed.

## 6. Regression + engine-B invariance (spec Q5) — SPECIFIED CORRECTLY

- Expenses $0 (serde default): `max(0, gross − 0) = gross` — algebraically byte-identical to
  today's path; the spec requires all P2-D + Chunk-A golden sets byte-identical. ✓
- Engine-B invariance: `compute_tax_year` never reads the new field (`crypto_ord` at
  `compute.rs:296-301` untouched); the spec's explicit invariance test (identical figures at
  expenses 0 vs 20,000) locks against future accidental coupling — same lock pattern as
  `tests/tax_compute.rs:252-279` (SE-not-in-engine-B). ✓
- Serde back-compat, real-path negative → `CliError::Usage`, `--show`, three-surface parity,
  asymmetric-style guard: all mirror the verified Chunk-A patterns (`types.rs:142-152`,
  `main.rs:725-741`). ✓

## 7. Findings

### I1 (Important) — Fully-expensed `None` breaks `render_schedule_se`'s None-disambiguation and emits a FALSE "wage base unavailable" note

`render_schedule_se(year, result, business_income_present, …)` disambiguates `None` with a
single boolean (`render.rs:1113-1118` + the None arm at ~1207+): `None` +
`business_income_present` → the "**SS wage base unavailable for {year}** … §1401 tax NOT
computed" note. That is sound today because `compute_se_tax` returns `None` **iff** gross SE
income is zero, so `None` + business-income-present can only mean "no bundled table"
(`se_net_income`'s own doc at `se.rs:50-52` states this two-state contract).

Chunk B **breaks that invariant**: golden (b) (gross $10,000, expenses $15,000, table
present) yields `None` with `business_income_present == true` — and `cmd/tax.rs:81-98` as
composed would render the false "wage base unavailable" note, telling the user their §1401
tax *could not be computed* when in truth **no SE tax is owed**. That misstates the user's
liability status on a real path. The spec's D2 says "(+ the render shows the no-SE state,
not a zero-row)" — it names the desired outcome but specifies no mechanism, and the
mechanism it inherits produces the wrong output.

**Fix (spec-level):** specify the disambiguation. Recommended: pass `schedule_c_expenses`
(and keep `business_income_present`) into `render_schedule_se`, and distinguish the three
`None` states at the call/render seam — (i) no business income → no section (unchanged);
(ii) business income + **no table** → the wage-base note (unchanged; the caller knows
`tables.table_for(year).is_none()`); (iii) business income + table present + expenses ≥
gross → a new "fully expensed" line: gross ${gross} − expenses ${expenses} ≤ $0 → no §1401
SE tax for {year}. Add a **render-level golden for the fully-expensed report text** (the
spec currently goldens only the compute-level `None`), including a negative assertion that
the wage-base note does NOT appear.

### M1 (Minor) — D3's breakout inputs are unspecified (`render_schedule_se` signature)

D3 renders "${gross} − ${expenses} = ${net_se}" and the advisory quotes ${expenses}, but
`SeTaxResult` gains no field and the current signature receives neither. "Optionally surface
gross in the render breakout" leaves the data path ambiguous. Specify: `render_schedule_se`
gains a `schedule_c_expenses: Usd` param (gross is reconstructible as `net_se + expenses`
whenever `result` is `Some`, since `Some` implies no clamping). This is the same signature
change I1's fix needs — fold together.

### M2 (Minor) — TUI and CSV fully-expensed behavior: state it, don't inherit it silently

- TUI (`tabs/tax.rs:95-121`): on `None` the SE block silently disappears — so a
  fully-expensed business loses its SE block with no explanation. This is consistent with
  the TUI's pre-existing thinness (it already silently drops the no-table case, and the
  Chunk-A N-1 deferral already covers the TUI's condensed block omitting disclosure text),
  but the spec should say explicitly whether the fully-expensed TUI drop is accepted
  (recommended: accept + note under the existing N-1 deferral) or handled.
- CSV (`render.rs:721-723`): fully expensed → no `schedule_se.csv` written. Acceptable
  (no Schedule SE to file), but the spec should state it, and the "nothing to file" comment
  at `render.rs:721` should be touched to cover the new reason.

### M3 (Minor) — `schedule_se.csv` shape for expensed years unspecified

`write_schedule_se_csv` (`render.rs:732-750`) writes a `net_se_earnings` column carrying
`se.net_se`. Post-Chunk-B that column silently becomes the **expensed** net with no
gross/expenses column alongside. The spec asserts $0-byte-compat (good) but does not say
whether the >0 case keeps the shape (re-documented) or gains `gross`/`schedule_c_expenses`
columns. Pick one explicitly; either is defensible (recommended: keep the shape, update the
column doc — the report carries the breakout).

### N1 (Nit) — §6017/$400 floor, pre-existing, now more salient

The engine computes SE tax on any positive base; the real Schedule SE owes nothing when net
earnings (line 4c) < $400. Pre-existing conservative overstatement, out of Chunk-B scope —
but expenses make near-zero nets much more reachable (e.g. gross 10,000 − expenses 9,700 →
base 277.05 → engine says ~$42 SE tax, actual $0). Worth a FOLLOWUPS line in Task 2.

### N2 (Nit) — stale line citation

The render-caveat citation "~1122-1127 pre-Chunk-A" is now `render.rs:1126-1136` at HEAD.
The spec self-hedges with "re-verify" — update the numbers at plan time.

## 8. Scope / right-sizing / TDD genuineness (spec Q6) — SOUND

- **Scope** matches the FOLLOWUPS pre-negotiation exactly (advisory-only; engine-B
  gross-vs-net `crypto_ord` coordination deferred as high blast radius; one annual figure;
  §164(f) auto-coordination stays deferred). The out-of-scope list is right. SemVer MINOR
  claim is correct (additive field with serde default, new param on a pre-1.0 API, new
  optional flag).
- **TDD genuineness:** the expensed goldens (a)/(c) cannot pass pre-change (new param —
  red via signature), the fully-expensed `None` is a genuine new branch, the regression set
  is a real byte-identity check, and the engine-B invariance test is an honest future-lock
  in the established pattern. The I1 fix must add the fully-expensed **render** golden to
  keep the red-first discipline on the one branch the current spec under-tests.
- **Task 2** (whole-diff review + FOLLOWUPS: cluster complete A+C+B; next = TY2024 tables)
  matches `FOLLOWUPS.md:70`.

## 9. Verdict

**0 Critical / 1 Important (I1) / 3 Minor (M1-M3) / 2 Nit (N1-N2) — NOT GREEN.**

The math core is fully verified: §1402(a) ordering correct, all three goldens confirmed
exact, W-2 composition ordering correct, the I3 advisory mechanism correctly reasoned
against `compute.rs` and correctly non-prescriptive. The single blocking issue is the
fully-expensed `None` colliding with `render_schedule_se`'s two-state None-contract,
producing a false "wage base unavailable" note (I1); its fix also resolves M1 and should
sweep M2/M3 statements into D3. Fold and re-review.

---

# Round 2 — re-review (post-fold)

**Artifact:** `design/SPEC_se_chunkB_expenses.md` (revised).
**Baseline re-verified:** HEAD `60f33c0` (unchanged).
**Reviewer:** independent R0 (architect), 2026-07-01. Scope: I1 closure, M1–M3/N1–N2 folds,
new-finding sweep. Round-1 confirmations (goldens, §1402(a) ordering, W-2 composition,
advisory mechanism) stand and were not re-litigated.

## I1 — CLOSED (behavioral level)

D3 now passes `schedule_c_expenses` into `render_schedule_se` and specifies the three-way
`None` split with the correct conditions and the correct liability framing ("no tax owed",
NOT "couldn't compute"). Verified against the actual `None` sources: `compute_se_tax`
returns `None` only at `se.rs:98-99` (`net_se.is_zero()` — which post-Chunk-B covers both
no-business and fully-expensed), and the no-table case arises at the caller's
`tables.table_for(year).and_then(…)` (`cmd/tax.rs:82`). The spec's three cases are
therefore **mutually exclusive and exhaustive** over the `None` states (business + table +
expenses < gross ⇒ `Some`, so no fourth state exists). The overlap edge (no table AND
fully expensed) deterministically lands in case 2 (wage-base note) — defined and truthful
("NOT computed" is literally accurate there), if slightly conservative; acceptable,
observation only.

The Task-1 golden set includes the render-level fully-expensed golden with BOTH the
positive assertion (the new "fully expensed … no §1401 SE tax" line) and the negative
assertion (NOT the wage-base note). Both are genuinely red pre-fix: the new line does not
exist, and the current `None` arm (`render.rs:1207+`) WOULD emit the false wage-base note.
Red-first discipline is intact on exactly the branch round 1 flagged as under-tested.

## M1/M2/M3/N1/N2 — folded, verified

- **M1** ✓ — D3: "gross for display = `net_se + expenses` when `Some`" (correct: `Some` ⇒
  no clamp, so the reconstruction is exact). The `None`-case gross is the M4 residual below.
- **M2** ✓ — TUI silent-omit of the fully-expensed line explicitly accepted under the
  Chunk-A N-1 deferral + FOLLOWUPS note; the no-CSV-output-on-fully-expensed behavior
  explicitly accepted and noted. (Wording nit N4 below.)
- **M3** ✓ — `schedule_se.csv` keeps its shape; `net_se_earnings` re-documented as the
  EXPENSED net; the report breakout carries the gross. Matches the round-1 recommendation.
- **N1** ✓ — Task-2 FOLLOWUPS gains the §6017 $400 filing-floor line.
- **N2** ✓ — citation updated to `render.rs:~1126-1136`; re-verified at HEAD (net-SE line
  1127-1131, caveat 1132-1135). Accurate.

## New/residual findings

### M4 (Minor) — `None`-arm renderer inputs under-specified (residual of the I1/M1 fold)

D3's parenthetical says `schedule_c_expenses` is needed "for the breakout AND the None
disambiguation" — but expenses alone cannot disambiguate case 2 from case 3, and the
fully-expensed line's `${gross}` is not reconstructible from a `None` result (the
`net_se + expenses` rule is explicitly `Some`-only). `render_schedule_se`
(`render.rs:1113-1119`) has neither `state` nor table access, so both signals must come
from the caller — where both already sit in hand (`cmd/tax.rs:81` computes
`se_net_income(&state, year)` for the existing bool; `:82` has `table_for(year)`).

**Why Minor, not Important:** the *behavior* is fully pinned — the three-way conditions
are stated in terms of gross/table-presence, and the render-level golden (positive +
negative assertions) makes the false note unimplementable-past. Only the parameter
plumbing is unnamed, and the caller-side facts force an obvious carrier. Same severity
calibration as round-1 M1. **Fix (one sentence in D3, plan-time):** the caller passes
`gross_se: Usd` (= `se_net_income(state, year)`; subsumes `business_income_present` as
`gross_se > 0`) and `table_present: bool` (= `tables.table_for(year).is_some()`); case 3's
`${gross}` is the passed gross — or equivalently, the caller performs the three-way
dispatch itself.

### N3 (Nit) — stale hedge in D2

D2's "optionally surface gross in the render breakout" predates the fold; D3 now MANDATES
the breakout when expenses > 0. Strike "optionally" (D3 governs; no behavioral ambiguity).

### N4 (Nit) — CSV wording: no FILE, not "no row"; and the stale `render.rs:721` comment

Fully expensed ⇒ `se_result` is `None` ⇒ `write_schedule_se_csv` is never called
(`render.rs:722-724`), so the whole per-year one-data-row `schedule_se.csv` is not
written — the spec's "no `schedule_se.csv` row is written" should say no *file* (same
outcome; tighten to avoid a header-only-file reading). The round-1 M2 sub-item — touch the
"nothing to file" comment at `render.rs:720-721` to add the fully-expensed reason — should
ride the D3 "note it" clause explicitly.

## Consistency / TDD sweep

Internally consistent apart from N3 (D2/D3 now agree that `SeTaxResult` gains no field,
`net_se` is the expensed net, and the render carries the gross). TDD remains genuine: the
expensed goldens are red via the new `compute_se_tax` signature, the render-level
fully-expensed golden is red on both assertions, the $0-regression is a real
byte-identity check, and the engine-B invariance test locks `compute.rs:296-301` against
future coupling. Scope, SemVer-MINOR, and the Task-2 FOLLOWUPS list (cluster complete
A+C+B; engine-B coordination deferred; §6017; TUI N-1 family; next = TY2024 tables) all
match the pre-negotiated queue.

## Round-2 verdict

**I1 CLOSED. 0 Critical / 0 Important / 1 Minor (M4) / 2 Nit (N3, N4) — R0 GREEN.**
Ready to implement. M4/N3/N4 are non-blocking plan-time touch-ups; if folded into the
spec, a light round-3 confirmation of just those lines suffices (no full re-review — no
behavioral surface moves).
