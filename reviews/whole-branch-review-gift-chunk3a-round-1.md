# Whole-branch review — charitable/gift Chunk 3a (§2505 advisory-level lifetime exemption) — Round 1

- **Role:** independent whole-branch reviewer = task review + final merge gate.
- **Branch:** `feat/gift-chunk3a` @ `a3370d0` (base `6a694fa`, spec commit `220a452`).
- **Artifacts read:** SPEC (`design/SPEC_gift_chunk3a_lifetime_exemption.md`), report
  (`.superpowers/sdd/gift-chunk3a-report.md`), diff (`review-6a694fa..a3370d0.diff`), and the
  **current source** at `crates/btctax-core/src/tax/tables.rs`,
  `crates/btctax-adapters/src/tax_tables.rs`,
  `crates/btctax-cli/src/{render.rs,main.rs,cmd/tax.rs,eventref.rs}`.
- **Gate posture:** the validation gate is reported GREEN (611 tests, clippy `-D warnings` clean,
  fmt clean, PII clean, `compute.rs` untouched). Not re-run per instruction; I independently
  re-derived the arithmetic and re-verified the I1/I2 disclosure integrity and the standalone
  posture against live source.
- **Verdict:** **GREEN — 0 Critical / 0 Important. Chunk 3a is READY TO MERGE.**
  Residual: 0 Minor of substance + 4 Nits (test-robustness / UX polish), none blocking.

---

## Re-derivation of the §2505 consumption arithmetic (highest priority)

TY2025 constants: annual exclusion **$19,000**, lifetime (basic) exclusion **$13,990,000**.
Alice $100,000 gift → per-donee taxable = `max(0, 100,000 − 19,000)` = **$81,000** (labeled).

Code under test (`render.rs:1335-1362`):
`cumulative = prior + total_taxable`; `remaining = if cumulative >= excl { 0 } else { excl − cumulative }`;
exceeded iff `cumulative > excl` (strict); `excess = cumulative − excl`; block gated on `cumulative > 0`.

| Case | prior | cumulative | remaining | exceeded? | excess |
|------|-------|-----------|-----------|-----------|--------|
| KAT-U under | $0 | 81,000 | 13,909,000 | no | — |
| KAT-P accumulate | 13,900,000 | 13,981,000 | 9,000 | no | — |
| **KAT-E exceeded** | **13,950,000** | **14,031,000** | **0** | **YES** | **41,000** |
| **KAT-B exact boundary** | **13,909,000** | **13,990,000** | **0** | **NO** | — |
| KAT-P4 prior-only | 5,000,000 (Alice $10k, taxable $0) | 5,000,000 | 8,990,000 | no | — |
| KAT-N no block | $0 (Alice $10k, taxable $0) | 0 | (block suppressed) | — | — |

**Exceeded-$41,000 (KAT-E), hand-derived:** `13,950,000 + 81,000 = 14,031,000`. Since
`14,031,000 > 13,990,000`, EXCEEDED fires; `excess = 14,031,000 − 13,990,000 = 41,000`. Matches the
`41000.00` assertion. ✔

**Exact-boundary-$13,990,000 (KAT-B), hand-derived:** `13,909,000 + 81,000 = 13,990,000`.
`remaining`: `cumulative >= excl` is `13,990,000 >= 13,990,000` = true → `remaining = 0`.
`exceeded`: `cumulative > excl` is `13,990,000 > 13,990,000` = **false** → NOT exceeded. So remaining
$0 and no EXCEEDED line — exactly the `>` (not `>=`) contract. ✔ The `remaining` branch's `>=` and the
exceeded branch's `>` are mutually consistent: at the boundary, remaining=0 AND not-exceeded.

**`cumulative_taxable > 0` render gate — confirmed** (`render.rs:1339`): KAT-N (no labeled taxable,
prior $0 → cumulative 0) suppresses the block; KAT-P4 (prior $5M, current taxable $0) still renders
because prior alone makes cumulative > 0. Both correct.

---

## Item-by-item

**1. §2505 arithmetic — CORRECT (see table above).** `cumulative = prior + current_labeled_taxable`;
`remaining` floored at 0; strict-`>` exceeded; `excess = cumulative − excl`. All six branches
re-derived exact. The block sits inside the `Some(t)` table path (after the no-table early return),
so it never runs without a table.

**2. $13,990,000 year-indexed field — CORRECT.** `TaxTable.gift_lifetime_exclusion: Usd`
(`tables.rs:81`) with §2010(c)(3)/Rev. Proc. 2024-40 §2.41 doc-cite; `ty2025()` sets
`dec!(13_990_000)` (`tax_tables.rs:193`) and the `source` string appends `+ §2.41`. All **13** literal
construction sites carry `gift_lifetime_exclusion: dec!(13_990_000)` (grep confirmed: synthetic_table,
ty2025, render.rs test helper, + 10 test fixtures across core/cli tests). The green build proves the
fan-out is complete (a missed literal fails to compile).

**3. I1 (stale caveat) — CURED.** The "§2505 lifetime exemption is a later chunk (Chunk 3)" clause is
GONE (`render.rs:1378-1384` footer now reads §2513 single-filer + future-interest + §2505
advisory-only/no-portability/prior-user-supplied). No self-contradiction can ship. The §2513 and
future-interest caveats are preserved (not over-removed). Absence KATs present
(`section_2505_stale_chunk3_caveat_is_absent`, and the `!contains("later chunk (Chunk 3)")` assertion
inside KAT-U and KAT-D0). No test pins the stale string, so the KAT is the only regression lock —
and it is present.

**4. I2 (unlabeled under-warning) — CURED.** When `unlabeled_count > 0` **and** the §2505 reassurance
is actually made (`cumulative > 0`), the block discloses that consumption reflects LABELED-donee
taxable only, names N unlabeled gift(s) and $X gross, and states "consumption may be understated /
remaining overstated" (`render.rs:1365-1373`). Mixed KAT present
(`section_2505_mixed_shows_omission_disclosure_for_unlabeled`: Alice $81k taxable + unlabeled $50k →
block shows $81,000 AND the omission line). The false-reassurance direction is corrected.

**5. `--prior-taxable-gifts` hygiene — CORRECT.** report-`--tax-year`-path only (consumed inside
`if let Some(y) = tax_year`, `main.rs:393-408`); parsed via `eventref::parse_usd_arg` =
`Decimal::from_str` → exact Decimal, no float (`eventref.rs:76-78`); NEGATIVE rejected with a
`CliError::Usage` (error, not clamp — `main.rs:402-406`); help text says "cumulative prior-year
TAXABLE gifts (post-annual-exclusion Form 709 amounts), not gross gifts" (`main.rs`); default $0 via
`unwrap_or_default()` and disclosed in the footer caveat.

**6. Standalone / no regression — CONFIRMED (highest priority).** `git diff --stat 6a694fa..a3370d0`
shows **`compute.rs` is not in the diff**; `grep` shows the sole production read of
`gift_lifetime_exclusion` is `render.rs:1335` (the §2505 block). `compute_tax_year` / engine B never
reads the new field → the tax identity and all goldens are unmoved; `tax_report.rs` goldens still
assert `1747.50` unchanged. The Chunk-2 safety branches are intact and correctly located:
`any_gift → None` (`render.rs:1217`), gifts-but-no-table → note (`render.rs:1231`), per-donee §2503(b)
grouping (`render.rs:1263-1304`). No §2502 rate-schedule computation; no leak into engine B.

**7. Exact Decimal / determinism — CONFIRMED.** No `f32/f64/as f64` in any changed file; `Usd =
Decimal`; `fmt_money = format!("{d:.2}")` (exact 2dp, no separators — the KAT exact strings hold).
Donee grouping uses `BTreeMap` (deterministic order); the block appends deterministically.

---

## Findings

### Critical — none.
### Important — none.
### Minor — none of substance.

### Nit
- **N-a (test robustness, KAT-B).** `section_2505_exact_boundary_...` asserts `msg.contains("0.00")`
  for remaining $0, but `"0.00"` is a substring of `"13990000.00"`, so that assertion is trivially
  satisfied and does not actually pin `remaining == 0`. This is harmless because the
  `!contains("EXCEEDED")` assertion in the same test **does** correctly lock the `>` vs `>=`
  boundary (a `>=` regression would emit EXCEEDED and fail), and the arithmetic is correct. Consider
  asserting the exact substring `"(0.00 remaining)"` for a tighter lock. Non-blocking.
- **N-b (flag UX).** `--prior-taxable-gifts` is parsed and negative-validated **only** inside the
  `--tax-year` branch; if a user passes it with `--year` alone (no `--tax-year`), it is silently
  ignored (and a negative value is not even rejected, since the guard never runs). No incorrect
  output results (the value is unused on that path), but a "flag ignored without --tax-year" hint
  would be friendlier. Spec only required "tax-year-path only," which is met. Non-blocking.
- **N-c (defensible scoping, not a defect).** In an all-unlabeled year with `prior = $0`,
  `cumulative = 0` → the entire §2505 block (and its omission disclosure) is suppressed. This is the
  **safe** choice — no false "$13.99M remaining / no tax due" reassurance is emitted — and the general
  unlabeled NOTE still warns and directs the user to `--donee`. The R0 round-2 review explicitly
  blessed gating the omission disclosure on `cumulative > 0` ("the disclosure only fires where a
  reassurance is actually made"). Recording it here only so the gate is documented; no action.
- **N-d (cosmetic redundancy).** The footer caveat naming `--prior-taxable-gifts (default $0)` renders
  even in the no-§2505-block case (cumulative 0). Harmless; optional to gate.

Note: R0's carried Nit **N1** ("gift tax may be due on $X" could be misread as $X-of-tax) is
effectively **resolved** in the implementation — the EXCEEDED line reads "gift tax may be due on ${excess}
(the excess base past the unified credit, not a computed tax); consult a professional," which frames
$X as the base, not the liability. R0's **O2** (duplicate §2513 caveat) is also resolved — §2513
appears only once (footer); the block does not repeat it.

---

## Bottom line

The legal core (BEA **$13,990,000**, §2.41 / §2010(c)(3), the §2505/§2502 cumulative
"no-tax-due-until-exhausted" model, year-indexed placement), the consumption arithmetic (all six
branches re-derived exact, incl. **exceeded $41,000** and **exact-boundary $13,990,000 → remaining $0,
not exceeded**), the I1 self-contradiction removal, the I2 under-warning cure, the flag hygiene
(exact Decimal, negative-rejected, taxable-not-gross help, default $0 disclosed), and the STANDALONE
posture (engine B / `compute_tax_year` never reads the new field; `compute.rs` untouched; goldens
unmoved) are all verified correct against live source.

**0 Critical / 0 Important → Chunk 3a is ready to merge.** The 4 Nits are optional polish (tighten
the KAT-B remaining assertion; the ignored-flag hint) and may be swept into FOLLOWUPS or during
Chunk 3b; none gate the ship.
