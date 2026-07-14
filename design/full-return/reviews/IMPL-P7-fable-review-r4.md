# P7 Independent Review — ROUND 4, on the r3 fold (Fable, r4) — **GREEN**

*Persisted VERBATIM, per `STANDARD_WORKFLOW.md` §2. Author = Opus; reviewer = Fable.
Nothing below has been edited, softened, or reordered.*

---

**Baseline:** r3 fold diff (399 lines) + current source. Tree clean, `make check` green at 1710, goldens regenerate bit-identically, frozen files byte-identical to `059ec2a`. ✓

## Verification of the r3 fold

I attacked every fix, and I checked for clobbering first.

- **No clobbering from the backup restore.** `git diff 297ec2d 66768de --stat` touches exactly 6 files, all intentional. All three r2 KATs survive in `extract_lines.rs` (`form_8995_row_1i_carries_the_proprietors_tin_not_the_taxpayers`, `..._refuses_to_file_a_qbi_total_for_an_unnamed_business`, `..._with_only_reit_dividends_leaves_part_i_blank`). Nothing was silently reverted.
- **r3-I1 — GENUINELY FIXED, both halves.** I re-ran my own mutation (`if false && c.business_description.trim().is_empty()`): it now dies on `a_schedule_c_with_no_business_description_refuses`. The negative leg is real (a screen that refused every Schedule C would fail it). And the deeper point landed: blanking Schedule C line A is now caught by **two** tests, including the new `every_filed_schedule_c_names_its_business_on_line_a` — I verified with `--no-fail-fast` that the new one genuinely fires, not just the pre-existing unit KAT.

- **Attack #1 — the cross-footed TOTAL TAX is NOT self-referential, and I can prove it.** The formula's components are OTS's own figures, and the structure is independently corroborated: `Σround(components)` reproduces **OTS's own separately-reported `total_tax` exactly on 11 of 12 households**, differing by exactly $1 only on `single_miner_qbi_limited_by_net_capital_gain` — which is precisely the declared §3.1 cross-footing effect and nothing else:

  | household | Σround(OTS components) | OTS total_tax | Δ |
  |---|---:|---:|---:|
  | (10 others) | … | … | **0** |
  | single_miner_qbi_limited_by_net_capital_gain | 16,833 | 16,832 | **1** |
  | mfj_se_over_the_addl_medicare_threshold | 49,569 | 49,569 | 0 |

  Crucially, **`golden_returns.rs:275` still holds btctax's printed line 24 against OTS's real `total_tax`** with the declared divergence intact — so the independent total check was *added to*, not replaced. And the formula is load-bearing: mutating btctax's `line24 = line22 + line23` → `line22` (dropping the whole Schedule 2 leg) fails the packet test on three households with correct diagnostics. It is not asserting btctax against itself.

- **Attack #2 — the `matches_2` / `no_second_opinion` rework is correct, and the guard now actually fires.** I traced all four branch combinations by hand and fault-injected two:
  - Relabelling the line-24 divergence `agrees_with: "PSL Tax-Calculator"` now **FAILS** (under the old `is_none_or` it passed). The anti-"btctax against the world" guard is no longer decoration.
  - The divergence-liveness check survived the rework (a dead entry still fails).
  - An undeclared disagreement on a two-oracle line (AGI + $1) still fails.
  The "both dissent" branch is now correctly gated on `!no_second_opinion`; it is currently unreachable (no household has both oracles dissenting), but it is correct, and `outlier_alt` still pins both when it is.

- **Attack #3 — the PDF-load hoist is safe.** `blank_fields` is collected from the pristine document *before* `drop_xfa_and_set_needappearances` and `apply_writes` mutate it — exactly as before; only the statement order moved. `pdf::index(&blank_fields)` is still built from the same pristine field list. No ordering hazard. `render_ssn(&proprietor.ssn, tin_max_len)` reads the cell's real `/MaxLen` (I confirmed it is 11), so the guard is now genuinely wired rather than asserted in a comment.

- **The trim is applied once, in the right place.** `ScheduleCHeader` (return_1040.rs:1297) trims, and both consumers — Schedule C line A (`printed.rs:1001`) and Form 8995 row 1i(a) (`packet.rs:452`) — read from that header. They cannot now disagree. Nothing else reads the raw `ri.schedule_c.business_description` except the screen (which trims).

- **The measured-vs-reasoned doc claims are corrected**, and correctly: the docstring now says "LOWER half" (matching the real rects y[551.97,563.97] inside y[551.97,575.97]) and now states plainly that a consistently-moved row is *not* rejected and should not be, since line 2 combines rows 1i–1v.

This is the first fold that introduced no defect of its own.

---

## Findings

### [MINOR] The cross-foot formula silently encodes "no credits, no AMT" and nothing asserts that precondition
**Where:** `crates/btctax-forms/tests/golden_packet.rs:~90-95` (`oracle_line24`).
**What:** 1040 line 24 = line 22 + line 23, where line 22 = line 18 − **line 21 (credits)** and line 18 = line 16 + **line 17 (AMT / excess APTC)**. The formula `round(tax) + round(SE) + round(NIIT) + round(addl_medicare)` is line 24 **only because** every golden household has zero credits and zero AMT (no dependents, no foreign tax credit, AMT screened out). That precondition is true today, holds across all twelve, and is corroborated by the 11/12 agreement with OTS's own total — but it is nowhere stated or asserted.
**Why it matters:** The thirteenth household is the hazard. Add one with a foreign tax credit and the formula over-states line 24 by the credit. It fails *loudly* (the paper would read lower than `oracle_line24`), not silently — so this is not a correctness hole — but it fails with a confusing message that points at the printed 1040 rather than at the formula, and the next author will debug the wrong thing. Given that this test's whole job is to be the last line of defence, its own assumptions should be visible.
**Fix:** Assert the precondition where the formula is built — e.g. that the transcribed 1040's line 17 and line 21 are absent/zero for the household — or note it in the comment beside `oracle_line24` so the failure mode is self-explaining.

### [NIT] `make check` is fail-fast, so a red gate shows only the first failure
`cargo nextest run --workspace` defaults to `fail-fast`, so the gate stops at the first failing test. Exit status still propagates correctly (verified in r3), so the gate is sound — but on red it under-reports. This bit me during this review: my Schedule C line-A mutation appeared to be caught by only one test until I re-ran with `--no-fail-fast`, which revealed the new golden-packet test catching it too. During a fold, this turns "fix all the breakage" into a serial rediscovery loop.
**Fix:** `cargo nextest run --workspace --no-fail-fast` in the `check` target (the suite is 7 seconds; there is no time to save).

### [NIT] The `printed()` helper is now nearly vestigial in `golden_packet.rs`
After the cross-foot rework, `printed(v: f64)` survives for exactly one caller (`the_se_households_print_the_oracles_se_tax_onto_schedule_se`), while the 1040 checks moved to `Usd` + `round_dollar` + `.to_string()`. Two spellings of "as it is printed" now coexist in one file. Harmless; worth collapsing when someone is next in there.

---

## Verdict

Three folds in a row introduced defects; this one did not. Every r3 finding is closed, and closed *properly* — the refusal is now tested three ways with a real negative leg, the line-A hole I pointed at (the case the 8995's fail-closed structurally cannot reach) is now pinned on the paper, the `/MaxLen` guard is wired rather than asserted, and the trim happens once at the source.

The two changes I flagged as risky in the ask both survive attack. The cross-footed TOTAL TAX is the stronger mechanism it claims to be — it reproduces OpenTaxSolver's own independently-computed total on 11 of 12 households, isolates the $1 §3.1 departure on exactly the household where it belongs, still leaves the true-total comparison standing in `golden_returns.rs`, and catches a real line-24 assembly bug when I inject one. The divergence-engine rework fixed the vacuous guard rather than papering over it: the guard I called decoration in r3 now genuinely fails when the divergence lies about which oracle it agrees with.

Nothing open blocks the gate.

**VERDICT: 0 Critical / 0 Important / 1 Minor / 2 Nit**
