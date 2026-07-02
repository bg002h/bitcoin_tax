# Whole-diff review — SE completion Chunk A: W-2 wage coordination (round 1)

**Artifact:** branch diff `33b7f26..6329c25` (2 commits: `ada1cbb` spec+R0 docs, `6329c25` implementation)
**Spec (contract):** `design/SPEC_se_chunkA_w2.md` (R0 GREEN @ round 2 — 0C/0I)
**Reviewer:** independent whole-diff reviewer (author ≠ reviewer)
**HEAD verified:** `6329c25` (matches the review package; diff re-checked against live source, not
just the packaged diff)
**Gate context (taken as given, not re-run):** 655 tests pass; clippy `-D warnings` clean; fmt clean;
PII clean.

**Verdict: GREEN — READY TO MERGE. 0 Critical / 0 Important / 1 Minor / 3 Nit.**

---

## 1. Formulas — re-derived and matched line-by-line against `se.rs` (source, not diff)

`crates/btctax-core/src/tax/se.rs:96-142` (`compute_se_tax`), rounding =
`round_cents` = `round_dp_with_strategy(2, MidpointNearestEven)` (`conventions.rs:13,22-24`) —
half-even at cents, exact Decimal, no float anywhere in the diff.

| Spec formula | Code | Match |
|---|---|---|
| `ss_cap = max(0, ss_wage_base − w2_ss_wages)` | se.rs:106-113 (`if c < ZERO { ZERO } else { c }`) — **floor present** | ✓ |
| `ss = 12.4% × min(base, ss_cap)` | se.rs:114-115 (`if base < ss_cap { base } else { ss_cap }`, `round_cents(SE_RATE_SS * ss_taxable)`) | ✓ |
| `medicare = 2.9% × base` (uncapped, unchanged) | se.rs:118 | ✓ |
| `addl_threshold = max(0, threshold(status) − w2_medicare_wages)` | se.rs:122-129 — **floor present** | ✓ |
| `over = max(0, base − addl_threshold)` | se.rs:130-137 — **floor present** | ✓ |
| `addl = 0.9% × over` | se.rs:138 | ✓ |
| `deductible_half = (ss + medicare)/2` — **EXCLUDES addl** (P2-D C1) | se.rs:141-142 (`(ss + medicare) / dec!(2)`) | ✓ |

Both `max(0,·)` floors demanded by §1402(b)(1) / §1401(b)(2)(B) are present. ≥0 preconditions
doc-commented on both params (R0-M5). Param names unambiguous; signature order
`(…, w2_ss_wages, w2_medicare_wages)`.

## 2. Goldens — independently re-derived by hand (TY2025: base $176,100; Single threshold $200,000; mining $100,000 → base 92,350.00)

1. **Headline (w2_ss 150,000 / w2_medicare 150,000)** — `w2_both_directions_headline_150k_ss_150k_medicare`:
   ss_cap = 26,100 → ss = 0.124 × 26,100 = **3,236.40** ✓; medicare **2,678.15** ✓;
   addl_threshold = 50,000 → over = 42,350 → addl = **381.15** ✓; total **6,295.70** ✓;
   deductible = (3,236.40+2,678.15)/2 = 2,957.275 → half-even → **2,957.28** ✓ (excludes 381.15).
   All six assertions match exactly.
2. **SS-above-base (180,000 / 0)** — `w2_ss_above_wage_base_180k`: ss_cap = max(0, −3,900) = 0 →
   **ss 0.00** ✓; addl 0 (threshold un-reduced; 92,350 < 200,000) ✓; total = **2,678.15** ✓;
   deductible = 1,339.075 → **1,339.08** ✓. Assertions match.
3. **Medicare-above-threshold (0 / 250,000)** — `w2_medicare_above_threshold_isolated_250k`:
   addl_threshold = max(0, −50,000) = 0 → addl = 0.009 × 92,350 = **831.15** ✓; ss **11,451.40** /
   medicare **2,678.15** unchanged ✓; total **14,960.70** ✓; deductible **7,064.78 UNCHANGED** ✓ —
   correctly pins that addl STILL never enters the §164(f) deductible.
4. **Asymmetric guard (150,000 / 0)** — `w2_asymmetric_transposition_guard_150k_ss_0_medicare`:
   ss **3,236.40** AND addl **0.00** ✓. A transposition flips both (11,451.40 / 381.15) — engine-level
   pin present, plus the render-level pin (`w2_asymmetric_render_transposition_guard`) and the CLI-path
   pin (§3).

## 3. Call-site parity — ALL THREE consumer surfaces source the PROFILE's W-2 fields (highest-priority check)

Full grep of `compute_se_tax(` across the workspace: exactly **three production call sites**, all
verified in live source, all passing `(w2_ss, w2_medicare)` in the correct order:

| Surface | Location | W-2 source | Verified |
|---|---|---|---|
| **Report** | `crates/btctax-cli/src/cmd/tax.rs:83-90` | `p.w2_ss_wages, p.w2_medicare_wages`; same pair also forwarded to `render_schedule_se` (tax.rs:92-98) | ✓ |
| **Export/CSV** | `crates/btctax-cli/src/cmd/admin.rs:63-70` | `p.w2_ss_wages, p.w2_medicare_wages` (full profile in the `and_then`) | ✓ |
| **TUI** | `crates/btctax-tui/src/tabs/tax.rs:93-97` | `profile.map(\|p\| p.w2_ss_wages).unwrap_or_default()` / same for medicare — **$0 only when no profile exists** (Decimal::default() == 0) | ✓ |

Every `compute_se_tax` call passing literal `Usd::ZERO` is inside `#[cfg(test)]`/`tests/`
(se.rs test module ×13; `tax_compute.rs:279` D5 standalone KAT) — **no production ZERO-passing call
exists.**

Transposition/divergence net:
- CLI-path KAT `chunk_a_asymmetric_w2_transposition_guard_cli_path` (tax_report.rs) exercises the REAL
  `cmd/tax.rs` site with the asymmetric profile; asserts `3236.40` present AND `11451.40` absent AND
  `381.15` absent (I checked: no other rendered figure in that SE section — 92350.00 / 2678.15 / 0.00 /
  5914.55 / 2957.28 / 150000.00 — can collide with the negative assertions).
- Export-parity KAT `chunk_a_export_parity_asymmetric_w2` uses the **ASYMMETRIC** profile
  (w2_ss 150,000 / w2_medicare 0 — the R0-M6 fixture, exactly as mandated), asserts the report shows
  3,236.40 and `schedule_se.csv` contains 3,236.40 and NOT 11,451.40 — catches both an admin.rs
  ZERO-default and an admin.rs-only transposition.

## 4. Regression net [R0-I2]

- **Five P2-D se.rs golden FIGURE-sets byte-identical** (calls now pass `Usd::ZERO, Usd::ZERO` —
  test-plumbing-only change): Golden-1 14,129.55/7,064.78; C1-lock 30,564.30/14,935.42 (the
  `assert_ne!(…, 15,282.15)` wrong-deductible pin retained); wage-base-cap 21,836.40; MFS 27,730.00
  (537.30 / 5,356.30 sub-figures intact); fractional 1,744.39 (1,413.75 / 330.64 / 872.20 intact).
  With W-2 = $0 the new formulas degenerate exactly to the P2-D code path, so identity is structural,
  not coincidental.
- **CLI KAT figures unchanged:** 11,451.40 / 2,678.15 / 14,129.55 / 7,064.78, plus the 21,885.50
  engine-B pin and the D5 `!it.contains("14129.55")` standalone pin — all retained.
- **Text assertions updated per the R0-N1 semantic pattern:** new phrasing asserted present
  ("$0 W-2 wages", "--w2-ss-wages", "NOT auto-coordinated", "coordinate it on your actual return",
  "W-2 coordination applied", "Form 8959 Part II") AND `!contains("OVERSTATED")` /
  `!contains("UNDERSTATED")` in both tax_report.rs and the render.rs schedule-se tests, in both modes
  (set / unset).
- **Old hedging text fully gone from render.rs:** workspace grep for OVERSTATED/UNDERSTATED hits only
  comments and the negative test assertions — the production string is deleted.

## 5. §164(f) advisory [R0-I3]

`render.rs:1167-1179`: the quantified first-order-overstatement text is present and matches spec D3
verbatim (deductible-half printed twice via `fmt_money`; "overstates your combined tax by your marginal
ordinary rate applied to $X"). The only `ordinary_taxable_income` mention is the D3-mandated
anti-prescription rationale ("The tax profile cannot express this deduction directly (reducing
`ordinary_taxable_income` would shift BOTH legs … only correct the bracket differential, not the
level)"). Workspace grep confirms **no "reduce your ordinary_taxable_income" prescription exists
anywhere** — the wrong-mechanism instruction R0-I3 killed did not resurface.

W-2 disclosure (D3 both modes, `render.rs:1180-1197`): mode switch is `w2_ss > 0 || w2_medicare > 0`
(matches the spec's exhaustive set/unset definition); coordinated mode prints
"SS cap = max(0, wage base − $X)" — the floored **expression**, so a negative cap value is never
rendered (R0-M3(a) satisfied); $0 mode prints the short set-the-flags note with no directional hedging.

## 6. CLI hygiene + global constraints

- **Negative rejection on the REAL path, BOTH flags:** `main.rs:710-721` — `is_sign_negative()` →
  `CliError::Usage` for `--w2-ss-wages` AND `--w2-medicare-wages`, inside the real `Command::TaxProfile`
  dispatch (identical placement to the `--prior-taxable-gifts` precedent at main.rs:454). Zero accepted.
  R0-N4's substance (both flags) confirmed satisfied. (Test coverage residue → M-1 below.)
- **`--show` displays both fields:** main.rs:268-279 prints `w2_ss_wages:` / `w2_medicare_wages:` lines;
  the set-then-show round-trip test now carries the fields through the struct equality.
- **Help text:** "Form W-2 Social Security wages (Box 3 + Box 7 tips; Schedule SE line 8a)…" /
  "Medicare wages (Box 5; Form 8959 line 1)…" — exactly the R0-M2 folded wording, with the coordination
  formula and non-negative requirement in the doc text.
- **Serde back-compat:** both fields `#[serde(default)]` (types.rs); `optional_profile_fields_default_to_zero`
  KAT extended to assert both new fields deserialize to $0 from old/minimal JSON; the round-trip KAT
  updated (compile-forced). All other `TaxProfile` struct literals across 15 test files updated with
  explicit `dec!(0)`/ZERO — mechanical, no figure moved.
- **Engine B untouched:** the diff's file list contains no `compute.rs`/`tables.rs`; `compute_tax_year`
  never reads the new fields; the D5 standalone KAT (`se_tax_is_standalone_not_in_total_federal_tax_attributable`)
  and the 21,885.50 CLI pin are unchanged.
- **Exact Decimal / determinism:** all new arithmetic is Decimal with `round_cents` (half-even);
  `fmt_money` = `{:.2}` on Decimal (no thousands separators — the `contains("3236.40")` assertions are
  sound); TUI `{:.2}` is Decimal Display, not float. Pure functions, no random state.
- **Workflow artifacts:** spec + R0 review persisted verbatim in commit `ada1cbb` before implementation;
  R0 ran two rounds to 0C/0I as required.

## 7. Findings

### Critical — none

### Important — none

### Minor

**M-1 — No automated test pins the negative-W-2 `CliError::Usage` rejection (either flag).**
The Task-1 CLI bullet lists "negative `--w2-ss-wages` → Usage error on the real path" in the test net,
but no test asserts it (the guards themselves are verified correct on the real dispatch path,
main.rs:710-721, both flags). Mitigations that keep this Minor rather than Important: (a) the specified
*behavior* exists and was verified by direct source reading — this is test-strength, not a wrong
behavior (the same calibration R0 applied to its own M6); (b) the precedent flag
(`--prior-taxable-gifts`) has the identical untested status — repo grep finds no "must not be negative"
test anywhere — so this matches existing convention; (c) the D1 phrasing ("validated on the REAL path,
not a test-side copy") reads primarily as a placement mandate, which is satisfied. A cheap fix exists:
`tests/config_dispatch.rs` already has a binary-invocation exit-code harness
(`config_binary_attest_without_set_pre2025_method_is_an_error`) that could be mirrored for
`tax-profile --w2-ss-wages -1` / `--w2-medicare-wages -1`. **Recommend a FOLLOWUPS entry; non-blocking.**

### Nit

**N-1 — TUI SE block shows coordinated figures without the W-2/§164(f) disclosures.**
`tui/tabs/tax.rs:100-117` renders components + the standalone note only. The figures are now correctly
profile-coordinated (the important part); the disclosure text lives only in the CLI report. Spec D3's
scope was `render.rs`, and the TUI block is deliberately condensed (it already omitted the expenses
caveat), so this is in-scope-faithful — but a user comparing TUI vs report sees the same figures with
less explanation. Optional FOLLOWUPS.

**N-2 — FOLLOWUPS.md not yet updated for the chunk split.** The P2-D "Deferred (OPEN → later)" block
(~lines 147-151) still lists `w2_ss_wages`/`w2_medicare_wages` as open, and the spec's Task 2 calls for
queuing Chunk C (ReclassifyIncome) then Chunk B (expenses, advisory-only) with the §164(f)
auto-coordination deferral rationale. Not part of the reviewed code diff — ship-phase bookkeeping to do
before/at merge.

**N-3 — Export-parity KAT asserts shared substrings rather than parsed figure equality.** It pins the
two named failure modes exactly (ZERO-default and admin-only transposition both produce 11,451.40,
which is asserted absent), so it discharges I1/M6 as specified; a parsed `ss_component ==` report-figure
comparison would be marginally stronger. No action required.

## 8. Verdict

**READY TO MERGE: 0 Critical / 0 Important / 1 Minor (M-1 negative-flag test coverage) /
3 Nit (N-1 TUI disclosure, N-2 FOLLOWUPS bookkeeping, N-3 parity-assertion form).**

The two coordination formulas are implemented exactly as specified with both `max(0,·)` floors; all
four W-2 goldens re-derive by hand to the cent (half-even ties included) and the assertions match; all
three consumer surfaces — CLI report (`cmd/tax.rs`), CSV export (`cmd/admin.rs`), and TUI
(`tabs/tax.rs`) — source the profile's `w2_ss_wages`/`w2_medicare_wages` with no production ZERO-passing
call; the asymmetric transposition + export-parity pins close the R0-I4/M6 wiring risk at engine,
render, CLI, and CSV levels; the five P2-D golden figure-sets and the CLI KAT figures are byte-identical
with the text assertions correctly migrated to the R0-N1 semantic pattern; the §164(f) advisory is the
quantified-overstatement form with no OTI-edit prescription; engine B is untouched. Recommend folding
M-1 (one exit-code test) via FOLLOWUPS and completing the N-2 FOLLOWUPS update at ship.
