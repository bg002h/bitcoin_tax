# Whole-branch review — charitable/gift Chunk 2 (donee identifier + Form 8283 donee + per-donee Form 709 advisory)

**Reviewer:** independent (whole-branch, round 1)
**Range:** `3a405f0..80ebead` (3 commits: spec `c96f150`, task1 `24f0405`, tasks2+3 `80ebead`)
**HEAD verified:** `80ebead` == the diff package's HEAD (git rev-parse confirmed).
**Scope:** Tasks 2+3 tax logic AND the whole chunk (Task 1 back-compat re-confirmed at branch level).
**Method:** Read the current source (authoritative), re-derived every KAT by hand, and ran targeted
greps for the Critical-class failure modes (broken back-compat, wrong per-donee §2503(b), engine-B leak).

## Verdict: **GREEN — ready to merge. 0 Critical / 0 Important.**

Findings below are 2 Nits + 1 Minor observation, none blocking. The three Critical-class risks the
task flagged (broken back-compat, wrong per-donee rule, leak into engine B) are all **absent**.

---

## 1. Per-donee §2503(b) (Task 3) — VERIFIED CORRECT (highest priority)

`render_gift_advisory` (`crates/btctax-cli/src/render.rs`) groups `Removal{Gift}` by donee into a
`BTreeMap<String, Usd>`, applies the exclusion **per donee**, and triggers filing on strict
`total > excl` per donee. Core logic:

```rust
for (donee, &total) in &labeled {
    let applied = if total < excl { total } else { excl };
    let taxable = if total > excl { total - excl } else { Default::default() };
    ... print per-donee line ...
    if total > excl { filing_required_donees.push(donee.clone()); total_taxable += taxable; }
}
```

### Hand re-derivation — the KEY LOCK (Alice $15k + Bob $15k, TY2025 excl $19k)
- `labeled = {Alice: 15000, Bob: 15000}` (aggregate $30k, but each isolated).
- Alice: `15000 < 19000` → applied 15000; `15000 > 19000` false → taxable 0; not added to filing set.
- Bob: identical → taxable 0; not added.
- `filing_required_donees` empty → emits **"No Form 709 filing required based on per-donee totals
  (each ≤ $19,000.00 exclusion). Total taxable gifts: $0.00."**
- **Result: NO filing, $0 taxable.** The *old aggregate* rule ($30k > $19k) would have wrongly
  flagged this — the lock holds. KAT `per_donee_under_exclusion_two_donees_no_filing_required`
  asserts both `contains("No Form 709 filing required")` and `!contains("Form 709 filing required
  (donee(s):")` and `contains("Total taxable gifts: $0.00")`, so it genuinely fails under the old
  aggregate logic (not a vacuous test).

### Hand re-derivation — Alice $25,000 (TY2025 excl $19k)
- `labeled = {Alice: 25000}`. `25000 < 19000` false → applied 19000; `25000 > 19000` true →
  taxable `25000 − 19000 = 6000`; Alice pushed to filing set; `total_taxable = 6000`.
- Emits **"Form 709 filing required (donee(s): Alice). Total taxable gifts: $6000.00."** plus the
  per-donee line "Alice: total $25000.00, exclusion applied $19000.00, taxable $6000.00".
- **Result: filing required, taxable $6,000.** Matches KAT `one_donee_over_exclusion_filing_required`
  (asserts `"Form 709 filing required (donee(s): Alice)"` + `"6000.00"`). `6000.00` is not a
  substring of `25000.00`/`19000.00`, so the assertion is genuine.

### Other Task-3 cases re-checked
- **Unlabeled $30,000 (None donee):** labeled empty → no filing summary line; unlabeled caveat fires
  ("… no donee label … PER DONEE …") + conservative signal ($30,000 > $19,000). NOT silently dropped.
  Matches `unlabeled_bucket_caveat_with_conservative_aggregate`.
- **Mixed Alice $25k + unlabeled $5k:** Alice filing-required line AND the $5,000 unlabeled caveat both
  appear (conservative $5,000 ≤ $19,000 branch). Matches `mixed_labeled_over_and_unlabeled_shows_both`.
- **Donations excluded:** grouping and `any_gift` both filter `kind == RemovalKind::Gift`, so a
  `Removal{Donation}` (even FMV $50k, `donee: Some("Charity X")`) → `any_gift == false` → `None`.
  Matches `donations_excluded_from_form709_advisory`. §2503(b) advisory is Gifts-only; §170 Donations
  never enter it. ✔
- **Boundary (total == excl):** `applied = excl`, `taxable = 0`, no filing (strict `>`). Correct —
  a gift of exactly the exclusion is fully excluded and does not require filing.

## 2. Preserved safety branches (Task 3) — VERIFIED

- **`any_gift`/no-gifts → `None` guard** survives (returns `None` before any table lookup;
  Donations don't count). KAT `no_gifts_is_none` intact.
- **gifts-present-but-no-table → `Some(note)` branch** survives, returns *before* per-donee grouping,
  sums all gift legs for the year so nothing is dropped, format string unchanged. KAT
  `gifts_present_but_no_table_emits_note_not_none` passes unmodified.
- The stale **"donee identity is not modeled" / "total-exposure signal"** test
  (`over_exclusion_emits_advisory_with_total_and_caveat`) and `under_exclusion_is_none` are replaced
  with real per-donee assertions (`labeled_donee_over_exclusion_emits_advisory` explicitly asserts
  `!contains("donee identity is not modeled")`). Genuine replacement, not deletion.

## 3. Form 8283 donee column (Task 2) — VERIFIED

`forms.rs` `form_8283` (line ~397): `donee: if is_first { r.donee.clone().unwrap_or_default() }
else { String::new() }` — carrier (smallest-`lot_id`) row populated from `Removal.donee`,
non-carrier legs empty (matches the `section`/`claimed_deduction`/`fmv_method` first-leg convention).
`None → ""`. `form8283.csv` auto-populates via the pre-existing `write_form8283_csv` path (writes
`row.donee` directly). Field + struct-level docs updated (donee removed from the "always EMPTY"
unmodeled list; Section-B structured name/address/EIN/appraiser correctly deferred to Chunk 3).

## 4. Back-compat (Task 1) — RE-CONFIRMED at branch level

Two-part lock, both verified in current source:
- `event.rs:119-120`: `#[serde(default)] pub donee: Option<String>` on the **`ReclassifyOutflow`
  struct** (required — serde errors on a missing `Option` field without `#[serde(default)]`).
- `event.rs:107`: `OutflowClass::GiftOut` stays a **UNIT variant** (bare `"GiftOut"` string), NOT
  converted to a struct variant.
- KAT `reclassify_outflow_legacy_json_back_compat_donee_defaults_to_none` (event.rs:518) pins **both**
  exact legacy JSON strings — bare `"as_": "GiftOut"` and legacy `Donate` map, neither containing
  `donee` — and asserts each `serde_json::from_str::<EventPayload>` returns `Ok` with `donee: None`.
  This is the "existing vault opens" guarantee. Airtight.

## 5. Standalone / no regression — VERIFIED (highest priority)

- `git diff --stat 3a405f0..HEAD -- crates/btctax-core/src/tax` → **empty**. `tax/` (compute.rs,
  mod.rs, se.rs, tables.rs, types.rs) is byte-untouched.
- `grep '\.donee'` across `btctax-core/src/` returns only: the back-compat test (event.rs), the
  resolve.rs `Op` mapping (218/226), and the forms.rs carrier-row read (398). **No `compute_tax_year`
  / engine-B path reads `removals.donee`.** Gifts/donations recognize zero gain (TP10), so the donee
  is pure carried data.
- `render_gift_advisory` is invoked only at `cmd/tax.rs:68` for the display report, computed
  *separately* from `compute_tax_year` (line 64) over `&state`; it never re-enters engine B or
  `state.advisory`. Standalone, matching the Chunk-1 advisory pattern.

## 6. Exact Decimal / determinism / fixture sites — VERIFIED

- `Usd = Decimal` (`conventions.rs:8`). No `f64`/`f32`/`as f`/float in the advisory region
  (grep of render.rs 1170-1420 → NONE). All arithmetic (`total - excl`, `+=`, comparisons) is exact
  Decimal; `Default::default()` for `Usd` is Decimal zero.
- `fmt_money = format!("{d:.2}")` — no thousands separators, so KAT substring assertions
  (`"6000.00"`, `"$0.00"`, `"15000.00"`) are genuine, not partial matches of larger numbers.
- Determinism: donee grouping via `BTreeMap<String, Usd>` (sorted by label); `filing_required_donees`
  built in that order, `.join(", ")` deterministic.
- `donee: None` fixture sites: 32 total; the only two in non-`tests/` files (event.rs:361,
  render.rs:1706) both sit inside `#[cfg(test)]` modules (starting at event.rs:278 / render.rs:1671).
  All test-only. ✔

---

## Findings

| # | Severity | Finding |
|---|----------|---------|
| 1 | Nit | The unlabeled "conservative aggregate" deliberately sums **all** `None`-donee gifts against **one** exclusion, so it can over-warn when unlabeled gifts actually went to distinct donees (e.g. 3×$10k → "aggregate $30k > $19k"). This is spec-mandated (D3), disclosed inline ("Shown as a single conservative aggregate" / "may span multiple donees"), and errs in the safe (over-warn) direction with a clear remedy (`--donee`). No action. |
| 2 | Minor (obs.) | "Total taxable gifts" reflects labeled donees only; unlabeled gifts that *might* (once labeled) push a donee over aren't in that figure. Correct — unattributable without a label — and covered by the unlabeled caveat. No action. |
| 3 | Nit | `applied`/`taxable` use two separate `if total </> excl` comparisons per donee; a single `match total.cmp(&excl)` would be marginally clearer. Current form is correct and readable. No action. |

## Bottom line

Chunk 2 is **ready to merge**. The per-donee §2503(b) rule is correct (the Alice+Bob-$15k → no-filing
lock and the Alice-$25k → taxable-$6,000 case both re-derive exactly and are guarded by genuine,
non-vacuous KATs), the safety branches survive the refactor, Form 8283's donee column is wired
correctly, back-compat is airtight (unit-variant + `#[serde(default)]`, pinned by a legacy-JSON KAT),
and the change is fully standalone — `tax/` is untouched and no engine-B path reads the donee.
**0 Critical / 0 Important.**
