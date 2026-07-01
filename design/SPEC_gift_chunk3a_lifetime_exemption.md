# SPEC — Charitable/gift Chunk 3a: §2505 advisory-level lifetime exemption

**Source baseline:** `origin/main` @ `6a694fa` (post Chunk 2).
**Goal:** Extend the per-donee Form 709 gift advisory (Chunk 2) with an **advisory-level §2505 lifetime
(basic) exclusion** consumption tracker: a year-indexed `gift_lifetime_exclusion` TaxTable field + a
user-supplied prior-cumulative-taxable-gifts input → a running "you have consumed $X of your $Y lifetime
exclusion; gift tax is due only after it is exhausted" advisory. **Advisory-level, single-filer** — NO
portability/DSUE, NO §2513 gift-splitting, NO multi-year auto-tracking (per the user-approved scope).

First half of Chunk 3 (the cluster's final chunk); Chunk 3b = Form 8283 Section-B appraiser/structured-
donee struct. **Standalone** — does NOT feed `compute_tax_year` / engine B (the gift advisory is display
only).

**SemVer:** additive `TaxTable.gift_lifetime_exclusion` field + a new CLI flag + an extended advisory ⇒
**MINOR** (pre-1.0). Additive only.

## Legal grounding (R0 to web-verify)
- **§2505(a) / §2010(c):** the gift tax uses a **unified credit** equivalent to the **basic exclusion
  amount** — no gift tax is actually DUE until the donor's cumulative taxable gifts exceed the basic
  exclusion amount. §2502: gift tax is computed on **cumulative** lifetime taxable gifts (current-year
  taxable gifts stack on prior-year taxable gifts), less the unified credit.
- **Basic exclusion amount (year-indexed, §2010(c)(3), inflation-adjusted):** **TY2025 = $13,990,000**
  (Rev. Proc. 2024-40 §2.41). (TY2024 = $13,610,000.) Belongs in the year-indexed `TaxTable` (like
  `gift_annual_exclusion`), NOT a fixed constant.
- **Advisory-level scope (user-approved):** single-filer; NO §2513 gift-splitting (which would double the
  effective exclusion for MFJ); NO §2010(c)(4) portability / DSUE (deceased-spouse unused exclusion); NO
  automatic prior-year cumulative tracking (the model sees only the current vault/year — prior cumulative
  taxable gifts are USER-SUPPLIED). All disclosed as caveats.

## Current-state (recon @ 6a694fa — re-verify at write time)
- **`render_gift_advisory`** (`render.rs`, per-donee after Chunk 2): groups `Removal{Gift}` by donee,
  applies `gift_annual_exclusion` PER DONEE, computes `taxable_to_donee = max(0, donee_total − exclusion)`,
  a filing-required trigger, and a "Total taxable gifts: $X" line (Σ labeled-donee taxable). The §2505
  consumption extends THIS current-year total-taxable figure.
- **`TaxTable.gift_annual_exclusion: Usd`** (`tables.rs`; TY2025 $19k in `tax_tables.rs`) — the exact
  precedent for adding `gift_lifetime_exclusion`. Adding a non-`Default` field to `TaxTable` forces every
  `TaxTable { .. }` literal to update (grep — like P2-D's `ss_wage_base`).
- CLI `report --tax-year` (`main.rs` + `cmd/tax.rs`) computes the advisory; a new `--prior-taxable-gifts`
  flag feeds the §2505 input.
- Standalone confirmed (the gift advisory is render-time; does NOT enter `state.advisory`/engine B).

## Design

### D1 — `gift_lifetime_exclusion` TaxTable field (year-indexed)
Add `pub gift_lifetime_exclusion: Usd` to `TaxTable` (`tables.rs`), cite §2010(c)(3)/Rev. Proc. 2024-40
§2.41; set TY2025 = `dec!(13_990_000)` in `BundledTaxTables::ty2025()` (`tax_tables.rs`). Update
`synthetic_table` + **every `TaxTable { .. }` literal** (grep `TaxTable {` — enumerate ALL, like P2-D's
`ss_wage_base`; the green build proves completeness).

### D2 — prior-cumulative-taxable-gifts input (`--prior-taxable-gifts`)
The §2505 consumption is `prior_cumulative_taxable_gifts + current_year_taxable_gifts` vs the lifetime
exclusion. The model cannot know prior-year gifts (single-vault, current-year) → a **user-supplied CLI
flag `--prior-taxable-gifts <USD>`** on the tax-report path (default `$0`), threaded into
`render_gift_advisory`. Default $0 is disclosed ("assumes $0 prior lifetime taxable gifts; if you have
filed Form 709 in prior years, supply your cumulative prior taxable gifts via `--prior-taxable-gifts`").
Stateless (no vault/config write) — matches the standalone advisory pattern. Validate non-negative.

### D3 — §2505 consumption in the gift advisory
Extend `render_gift_advisory` (after the per-donee current-year total taxable is computed):
- `current_year_taxable` = the Σ labeled-donee taxable already computed (Chunk 2).
- `cumulative_taxable = prior_taxable_gifts + current_year_taxable`.
- `lifetime_used = cumulative_taxable`; `lifetime_remaining = max(0, gift_lifetime_exclusion −
  cumulative_taxable)`.
- Emit ONLY when `cumulative_taxable > 0` (there are taxable gifts to apply the exclusion against):
  "§2505 lifetime (basic) exclusion: you have used ${cumulative_taxable} of your ${gift_lifetime_exclusion}
  ({year}) lifetime exclusion ($ {lifetime_remaining} remaining). No gift tax is DUE until cumulative
  taxable gifts exceed the lifetime exclusion." If `cumulative_taxable > gift_lifetime_exclusion` →
  "lifetime exclusion EXCEEDED — gift tax may be due on ${cumulative_taxable − gift_lifetime_exclusion};
  consult a professional."
- Caveats: single-filer (no §2513 gift-splitting); no portability/DSUE (§2010(c)(4)); prior cumulative is
  user-supplied (default $0 if not given).
- **[R0-I1] Replace the stale "§2505 lifetime exemption is a later chunk (Chunk 3)" caveat**
  (`render.rs:~1332-1334`) — post-3a the advisory emits a real §2505 block, so that line self-contradicts.
  Remove it; add an absence KAT asserting the "later chunk (Chunk 3)"/"§2505 … later" string no longer
  appears.
- **[R0-I2] Disclose the unlabeled-gift omission.** `current_year_taxable` = Σ LABELED-donee taxable ONLY
  (Chunk 2 excludes the unlabeled bucket). So the §2505 consumption UNDERSTATES when unlabeled gifts exist
  — the under-warning direction ("remaining / no tax due" is falsely reassuring). When any unlabeled gifts
  exist, emit a disclosure line: "§2505 consumption reflects LABELED-donee taxable gifts only; N unlabeled
  gift(s) totalling ${X} are NOT included — label them via `--donee` for a complete figure; consumption
  may be understated / remaining overstated." Add a mixed KAT (labeled taxable + unlabeled present → the
  disclosure line present).
- **[carry Chunk-2 safety] Preserve** the `any_gift → None` guard + the gifts-but-no-table → note branch;
  the §2505 block only renders inside the Computed-advisory path. Still STANDALONE (no engine B).

### Decisions
- **Advisory-level only** — report consumption/remaining + "no tax due until exhausted"; do NOT compute
  the actual §2502 gift-tax-rate-schedule liability (that needs the full cumulative-bracket recomputation +
  the credit — deferred; and gift tax is genuinely due only past $13.99M, a corner case for this user).
- **Prior cumulative = user-supplied CLI flag, default $0 + disclosed** (stateless; no vault write).
- **Year-indexed lifetime exclusion** in TaxTable (mirrors `gift_annual_exclusion`).

## Plan (TDD)

### Task 1 — `gift_lifetime_exclusion` + `--prior-taxable-gifts` + §2505 consumption + goldens
- **Files:** `crates/btctax-core/src/tax/tables.rs` (field + synthetic_table), `crates/btctax-adapters/
  src/tax_tables.rs` (TY2025 $13,990,000), `crates/btctax-cli/src/{main.rs,cmd/tax.rs,render.rs}` (the
  flag + threading + the §2505 block). **[R0-M1] TWO fan-outs to update completely:** (a) the ~**13**
  `TaxTable { .. }` literal construction sites — grep `ss_wage_base:` as the proxy (NOT `TaxTable {`,
  which false-positives on `se.rs`'s fn signature); (b) the **9** `render_gift_advisory` CALL sites whose
  arg list grows by the new `prior_taxable_gifts` param. The green build proves both are complete.
- **[R0-M3] `--prior-taxable-gifts` flag hygiene:** on the `report --tax-year` path only; parse as exact
  `Usd`/Decimal (no float); reject negative (error, not silent clamp); help text says "cumulative prior-
  year TAXABLE gifts (post-annual-exclusion Form 709 amounts), not gross gifts".
- Hand-verified KATs (synthetic; TY2025 exclusion $13,990,000, annual $19,000; assert EXACT):
  - **Under lifetime, one donee over annual:** "Alice" $100,000 gift, prior $0 → current-year taxable
    $81,000 (100,000 − 19,000); §2505: used $81,000 of $13,990,000, remaining $13,909,000; NO tax due.
  - **Prior gifts accumulate:** "Alice" $100,000, `--prior-taxable-gifts 13,900,000` → cumulative
    $13,981,000; remaining $9,000; no tax due (still ≤ exclusion).
  - **Exceeds lifetime:** "Alice" $100,000, `--prior-taxable-gifts 13,950,000` → cumulative $14,031,000 >
    $13,990,000 → "EXCEEDED — gift tax may be due on $41,000".
  - **No taxable gifts → no §2505 block:** all donees under the annual exclusion (Chunk-2 no-filing case)
    → current-year taxable $0, prior $0 → NO §2505 line (nothing to apply).
  - **Default $0 prior:** no `--prior-taxable-gifts` → prior $0 + the disclosure caveat present.
  - **[R0-M2] Exact-boundary:** cumulative EXACTLY $13,990,000 → remaining $0, NOT "exceeded" (tests
    `>` vs `>=` on the exceeded branch — at exactly the exclusion, no tax is due).
  - **[R0-M4] Prior-only edge:** `--prior-taxable-gifts 5,000,000` with all current-year donees UNDER the
    annual exclusion (current taxable $0) → §2505 block STILL shows (cumulative $5,000,000 > 0, from prior).
  - **[R0-I1] Absence:** the "§2505 … later chunk (Chunk 3)" stale caveat string is GONE from the output.
  - **[R0-I2] Mixed/unlabeled:** labeled "Alice" $100,000 (taxable $81k) + an unlabeled $50,000 gift →
    the §2505 block shows used $81,000 AND the unlabeled-omission disclosure line (consumption understated).
  - Preserve: no-gifts → `None`; gifts-but-no-table → the note.

### Task 2 — whole-diff review (Phase E) + FOLLOWUPS
- Cross-cutting: §2505 consumption arithmetic correct (cumulative = prior + current; remaining floored at
  0; exceeded case); the $13,990,000 year-indexed field + all TaxTable literals updated; the CLI flag
  default $0 disclosed + non-negative; STANDALONE (engine B / tax identity untouched — assert a golden
  unmoved); the Chunk-2 per-donee + safety branches intact; exact Decimal; determinism.
- FOLLOWUPS: Chunk 3b (Section-B appraiser/structured-donee); the actual §2502 gift-tax-rate-schedule
  liability computation (deferred — advisory only); §2513 splitting + portability/DSUE (out of scope);
  year-indexed lifetime exclusion for TY2024/2026+.

## Out of scope
- The §2502 gift-tax-rate-schedule liability (advisory only); §2513 gift-splitting; §2010(c)(4)
  portability / DSUE; automatic prior-year cumulative tracking; Form 8283 Section-B appraiser struct
  (Chunk 3b); feeding any of this into engine B / `compute_tax_year`; 2026/2027 tables.
