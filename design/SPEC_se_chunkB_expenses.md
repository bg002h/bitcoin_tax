# SPEC — SE completion Chunk B: Schedule C expenses (advisory-only)

**Source baseline:** `main` @ `60f33c0` (post Chunks A + C). The FINAL SE-cluster chunk (queue item 2).
**Goal:** A user-supplied per-year **Schedule C deductible-expenses** figure that reduces **net SE
earnings** (`net_se = max(0, gross − expenses)`) — replacing the "no business expenses modeled; your
actual SE tax is lower" caveat with real math. **ADVISORY-ONLY for engine B:** `crypto_ord` (the ordinary-
income stack) stays GROSS — the engine-B coordination is explicitly deferred (high blast radius; recon-
verified) and DISCLOSED with the correct mechanism framing.

**SemVer:** additive `TaxProfile` field + a `compute_se_tax` param + a CLI flag ⇒ **MINOR** (pre-1.0).

## Legal grounding (R0 to spot-check)
- **§1402(a):** net earnings from self-employment = gross income from the trade/business LESS the
  deductions attributable to it (Schedule C net) × 92.35%. Expenses reduce the SE base — real math, not
  an approximation.
- The SAME expenses also reduce Schedule C net income for the ORDINARY income tax — the engine's
  `crypto_ord` is gross, so the income-tax side is overstated when expenses exist. **[The R0-I3 lesson
  from Chunk A applies verbatim]:** do NOT prescribe "reduce your `ordinary_taxable_income` by $X" —
  `crypto_ord` enters the WITH leg only, so an OTI edit shifts BOTH legs of the crypto-attributable delta
  and corrects only the bracket differential, not the level. The honest advisory: quantify the first-order
  overstatement (marginal ordinary rate × expenses), state the profile cannot express it, defer the
  engine-B (`crypto_ord`) coordination.

## Current-state (recon @ 33b7f26, re-verify @ 60f33c0)
- `se.rs`: `se_net_income(state, year)` sums gross (business, non-Interest); `compute_se_tax(state, year,
  status, table, w2_ss, w2_medicare)` post-Chunk-A; the `SeTaxResult.net_se` doc says "no Schedule C
  expenses modeled — FOLLOWUP"; the render caveat ("your actual SE tax is lower") at `render.rs` (~1122-
  1127 pre-Chunk-A — re-verify).
- `TaxProfile` (post-Chunk-A): the `#[serde(default)]` pattern + real-path negative validation + `--show`
  established for the W-2 fields — mirror exactly.
- THREE call sites (report `cmd/tax.rs`, export `cmd/admin.rs`, TUI `tabs/tax.rs`) — all must pass the
  new figure (the Chunk-A parity lesson + the asymmetric-fixture guard pattern).
- `crypto_ord` (`compute.rs:296-301`): GROSS, kind/business-agnostic — UNTOUCHED by this chunk.

## Design

### D1 — `TaxProfile.schedule_c_expenses` + CLI
`#[serde(default)] pub schedule_c_expenses: Usd`; `--schedule-c-expenses` (optional, default $0; negative
→ `CliError::Usage` on the REAL path; help: "Schedule C deductible business expenses for the year —
reduces net SE earnings; the income-tax stack above is NOT adjusted (see the advisory)"); shown by
`--show`.

### D2 — `compute_se_tax` expenses param
`compute_se_tax(state, year, status, table, w2_ss, w2_medicare, schedule_c_expenses: Usd)` (explicit, like
the W-2 params; ≥0 doc precondition). Inside: `gross = se_net_income(state, year)` (unchanged helper);
`net_se = max(0, gross − schedule_c_expenses)`; `net_se == 0` → `None` (mirrors the no-business-income
path — a fully-expensed business owes no SE tax). Everything downstream (×0.9235, caps, addl, deductible)
operates on the expensed net. `SeTaxResult` gains NO new field (net_se now IS the expensed net; update its
doc — the render breakout surfaces the gross, per D3 [R0-N3]). ALL THREE call sites pass
`p.schedule_c_expenses` (default $0 when no profile — TUI).

### D3 — render: breakout + the correct-mechanism advisory + the None three-way split [R0-I1]
- **Pass `schedule_c_expenses` into `render_schedule_se`** (for the breakout) **plus [R0-M4] the two
  None-disambiguation signals the renderer cannot derive itself: `gross_se: Usd` (from `se_net_income` at
  the caller — subsumes `business_income_present` as `gross_se > 0`) and `table_present: bool`** (the
  caller already has both; alternatively the caller does the three-way dispatch — either, but specify one
  data path in code). The gross for display = `net_se + expenses` when `Some` [R0-M1]; for the
  fully-expensed None case = the passed `gross_se`. (No-table AND fully-expensed → case 2, deterministic.)
- **[R0-I1] Three-way `None` split** (the current two-state contract breaks — a fully-expensed year would
  falsely print the "SS wage base unavailable" note): (1) no business income → NO section (unchanged);
  (2) business income + NO bundled table → the wage-base-unavailable note (unchanged); (3) business income
  + table PRESENT + `expenses ≥ gross` → a NEW line: "fully expensed: gross ${gross} − Schedule C expenses
  ${expenses} ≤ $0 → no §1401 SE tax for {year}." — the liability status is "no tax owed", NOT "couldn't
  compute". Render-level golden: the fully-expensed report shows the new line AND (negative assertion) NOT
  the wage-base note.
- The net-SE line becomes a breakout when expenses > 0: "gross business income ${gross} − Schedule C
  expenses ${expenses} = net SE earnings ${net_se}". When $0: keep a short "no Schedule C expenses
  supplied (--schedule-c-expenses)" note (the old "not modeled" caveat REPLACED — now at
  `render.rs:~1126-1136` [R0-N2]).
- **[R0-M2]** The TUI's condensed SE block silently omits the fully-expensed line (accepted under the
  Chunk-A N-1 deferral — note in FOLLOWUPS); the whole `schedule_se.csv` FILE is skipped for a fully-expensed year [R0-N4 — update the "nothing to
  file" comment at `render.rs:~720-724` to mention this case]
  (the CSV writer sees `None` — same as no-business today; acceptable, note it). **[R0-M3]** Document that
  `schedule_se.csv`'s `net_se_earnings` column now carries the EXPENSED net (header unchanged; the report
  breakout carries the gross).
- **The advisory [I3-mechanism]:** when expenses > 0: "Schedule C expenses also reduce your ORDINARY
  taxable income, but the income-tax total above uses GROSS crypto income — to first order it OVERSTATES
  your tax by your marginal ordinary rate applied to ${expenses}. The tax profile cannot express this
  (an `ordinary_taxable_income` edit would shift both legs of the crypto-attributable delta); the
  engine-side coordination is deferred — coordinate on your actual return." NO OTI-edit prescription.

## Plan (TDD)

### Task 1 — field + param + math + render + goldens
- **Files:** `crates/btctax-core/src/tax/{types.rs,se.rs}`, `crates/btctax-cli/src/{main.rs,cmd/tax.rs,
  cmd/admin.rs,render.rs}`, `crates/btctax-tui/src/tabs/tax.rs`.
- Hand-verified goldens (TY2025; Single; mining $100,000; assert EXACT; expensed goldens FAIL red pre-fix):
  - **Headline:** expenses $20,000, no W-2 → net_se $80,000; base $73,880.00; ss = 12.4% × min(73,880,
    176,100) = **$9,161.12**; medicare = 2.9% × 73,880 = **$2,142.52**; addl $0; total **$11,303.64**;
    deductible_half **$5,651.82**.
  - **Fully expensed → None [R0-I1]:** mining $10,000, expenses $15,000 → net_se $0 → `compute_se_tax` →
    `None`; the RENDER-level golden: the report shows the "fully expensed … no §1401 SE tax" line and
    (negative) NOT the "wage base unavailable" note.
  - **Expenses × W-2 combined:** expenses $20,000 + w2_ss $150,000 + w2_medicare $150,000 → base $73,880;
    ss = 12.4% × min(73,880, 26,100) = **$3,236.40**; medicare **$2,142.52**; addl = 0.9% × (73,880 −
    50,000) = **$214.92**; total **$5,593.84**; deductible_half = (3,236.40 + 2,142.52)/2 = **$2,689.46**
    (excludes addl — the C1 rule survives composition).
  - **Regression:** expenses $0 (serde default) → ALL P2-D + Chunk-A golden figure-sets byte-identical.
  - **Engine-B invariance:** `compute_tax_year` figures IDENTICAL with expenses 0 vs $20,000 (crypto_ord
    gross, untouched) — the advisory, not the engine, carries the difference.
  - **CLI/render:** negative flag → Usage (real path); the breakout line when > 0; the $0 note otherwise;
    the I3-mechanism advisory text present (and NO "reduce your ordinary_taxable_income" prescription);
    `--show` displays it; serde back-compat (old profile JSON → $0); export/TUI parity (the asymmetric-
    style guard: an expensed profile renders the same figures in report + CSV).

### Task 2 — whole-diff review (Phase E) + FOLLOWUPS
- Cross-cutting: the max(0,·) floor; None-on-fully-expensed; composition with W-2 correct; the deductible
  still excludes addl; regression nets byte-identical; engine B untouched (assert); the advisory mechanism
  correct (no OTI prescription); all three surfaces source the profile; exact Decimal; determinism.
- FOLLOWUPS: the SE cluster is COMPLETE (A + C + B); the engine-B gross-vs-net `crypto_ord` coordination
  (the real fix for the ordinary-income overstatement — deferred, high blast radius); **[R0-N1] the §6017
  $400 filing floor** (SE tax filing is required only when net SE earnings ≥ $400 — not modeled; more
  salient now that expenses can bring net near zero); the TUI fully-expensed line (Chunk-A N-1 family);
  next queue item = TY2024 tables backfill (the queue's last item).

## Out of scope
- Engine-B `crypto_ord` net-of-expenses coordination (deferred — the advisory discloses); per-activity
  expense allocation (one annual figure); §164(f) auto-coordination (already deferred); depreciation/
  §179/home-office sub-schedules (the user supplies the net deductible total); 2026/2027 tables.
