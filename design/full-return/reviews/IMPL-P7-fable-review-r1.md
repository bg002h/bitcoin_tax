# P7 Independent Review — golden returns, §199A QBI, packet round-trip (Fable, r1)

*Persisted VERBATIM before folding, per `STANDARD_WORKFLOW.md` §2. Author = Opus; reviewer = Fable.
Nothing below has been edited, softened, or reordered.*

---

**Reviewer:** independent (did not author any P7 code). **Baseline:** `c55584d..HEAD`, suite green at 1702/1702 before and after review. **Frozen files:** `git diff 059ec2a..HEAD -- tax/{types,compute,se}.rs` is empty — untouched. ✓

## What I did

- Read the full 3,366-line diff and the current source of every named artifact.
- Re-ran **both oracles from scratch** (`OTS_DIR=… gen_goldens.py`): output is **bit-identical** to the committed `full_return_goldens.json` (modulo `_provenance`). Not hand-edited, not stale.
- Pulled the **actual 2024 IRS Tax Table** (`irs.gov/pub/irs-prior/i1040tt--2024.pdf`) and verified every declared divergence against the printed table.
- Cross-checked the three input translations field-by-field.
- Ran **8 fault injections** against the live suite (`make check` ≈ 6s each).
- Verified `debug_assertions` and overflow checks are still ON under `opt-level = 1` with a canary test (added, ran, deleted).

## Verification of P7's claims (what survived attack)

**1. Input-translation independence — HOLDS.** Field-by-field: `w2_income` → btctax W-2 box 1 = box 3 = box 5 (owner Taxpayer) / OTS `L1a` + Sch SE `L8a` + 8959 `L1` / taxcalc `e00200`=`e00200p` — all three say "one filer earned everything," including the MFJ-SE household, and the docstrings disclose it. `ordinary_dividends` is inclusive of qualified in all three (OTS `L3b` is the form's own line 3b; confirmed empirically — an additive treatment would have moved AGI by $8,000 on `single_qdcgt_both_slices` and it didn't). SALT reaches all three engines as separate 5a/5b components, so the cap is genuinely exercised. Capital gains go to OTS as dated 8949-style transactions (term decided by OTS from dates, matching btctax's ledger terms). The lump `itemized_deductions` lands on different Schedule A *lines* per engine (btctax 8a, OTS A16, taxcalc e19200) but identical uncappable totals — sound for figure comparison.

**2. DECLARED_DIVERGENCES — all six verified against the printed 2024 Tax Table.** Rows extracted from the IRS PDF: 47,400–47,450 Single = **5,487**; 52,400–52,450 = **6,587** (two households); 70,000–70,050 = **10,459**; MFJ 3,700–3,750 = **373**. The Tax Table is mandatory below $100,000 (1040 line-16 instructions), and taxcalc's exact-schedule figures differ by precisely the bin-midpoint delta in every case. `single_qdcgt_both_slices` (TI $112,400) is correctly declared: the QDCGT worksheet looks up the *ordinary remainder* ($79,400) in the Table — IRS row 79,400–79,450 = **12,527**, + 15% × 33,000 = **17,477**, exactly btctax/OTS. **No divergence is a btctax bug being explained away.** The test logic also cannot be used to mask a btctax regression: a moved btctax value trips `assert_eq!(ours, d.btctax)` even if it lands on taxcalc's figure (verified by reading all branches).

**3. §199A arithmetic — correct against the statute.** QBI = Sch C net profit − §164(f) half-SE (per Form 8995 instructions; **independently derived by taxcalc's own `qbided` = 11,152.227 = 20% × (60,000 − 4,238.865)** — the one engine that computes the rule rather than being handed it). Income limitation = lesser of the component and 20% × (TI-before-QBI − net capital gain) per §199A(a); line 12's net capital gain = qualified dividends + `preferential_gain`, and I verified `net_1222.preferential_gain` is the §1222(11) quantity (LT gain surviving ST-loss cross-netting, ≥ 0 — equals min(Sch D 15, 16)⁺ in all sign cases). Threshold refuse compares TI-*before*-QBI to $191,950/$383,900 ✓. A Schedule C loss refuses upstream (`ScheduleCLoss`, return_1040.rs:602) so negative QBI is unreachable ✓. One independence caveat, disclosed in the harness: OTS's 8995 is *fed* `round(profit − half_se)` by the harness, so OTS validates only the downstream arithmetic of the QBI rule; taxcalc alone independently confirms the rule itself. Acceptable, but worth knowing.

**4. Packet round-trip reasoning — sound, and it has teeth.** The printed-vs-absolute chain argument for TOTAL TAX is correct (components are `round_dollar` of exacts by construction; only the total sums pre-rounded lines). Fault injection M6b (21% instead of 20% on the printed 8995's line 5) was caught by `every_golden_household_prints_the_oracles_figures_onto_the_1040` with exactly the right diagnostics (paper 69,451 vs OpenTaxSolver 70,009 on line 15, cascading to 16 and 24).

**5. `extract_lines` silent-drop attack — defused.** `pdf::apply_writes` fails closed (`MapFieldMissing`) on any write to a nonexistent field, so a map typo on a written cell breaks the *fill*, loudly, before the transcriber could silently drop it. Absence of an asserted key produces `<BLANK>` ≠ expected → failure. The residual exposure (a typo on a mapped-but-never-written cell) has no assertion riding on it.

**6. Fault-injection results:** M1 (drop half-SE from QBI) — caught by both golden tests. M2 (drop net-capital-gain from the income limit) — caught by 2 unit KATs. M4 (emit OFF checkboxes) — caught. M5 (tamper a golden JSON value) — caught. M6 (starve printed 8995 of business QBI) — caught (form-set test). M6b — caught (paper round-trip). M7 (uncap printed SALT 5e) — caught. **M3 — survived. See Important-2.**

**7. `opt-level` change:** `debug_assertions` and overflow checks confirmed ON by canary under the new profile (cargo's dev profile sets both explicitly; rustc's opt-level-tied default doesn't apply). Nothing masked.

## Findings

### [IMPORTANT] Form 8995 files with a non-zero line 2 over a BLANK Part I table
**Where:** `crates/btctax-forms/src/form8995.rs:13-15, 72-88`; `crates/btctax-core/src/tax/qbi.rs:180`; map `crates/btctax-forms/forms/2024/f8995.map.toml:18` (comment only — rows unmapped).
**What:** P7 makes `Form8995Lines.line2` the Schedule C QBI (e.g. **$55,761** for `single_crypto_business_se`), but the filler never writes Part I rows 1i–1v — no trade/business name (1i-a), no TIN (1i-b), no per-business QBI (1i-c). `grep -rn "Ln1" crates/btctax-forms/src/` finds nothing; the row widgets exist in the PDF (the map's own comment names them) but are not even mapped. The form's line 2 text — quoted in the map itself — is "Total qualified business income or (loss). **Combine lines 1i through 1v, column (c)**": the filed form totals a column that is empty.
**Why it matters:** Every SE household now files an internally inconsistent Form 8995 — a total with no rows behind it and no identification of the business the deduction is claimed for. The deduction amount is right (not a wrong tax), but this is the same class as P6's unnamed Form 8949, which was treated as blocking. The module doc still asserts the table is "never touched: v1's only QBI is §199A REIT dividends" — false since 0d2347f.
**Evidence:** Code paths above; the golden packet's identity sweep checks only the form *header*, not row 1i, so it passes.
**Fix:** Map `Ln1A/B/C_Row1` widgets; plumb the Schedule C business description (already in `ScheduleCHeader`) and the filer's SSN into row 1i with 1i(c) = line 2; assert it in `golden_packet.rs` for the SE households.

### [IMPORTANT] The business-QBI leg of the §199A(e)(2) over-threshold refuse is untested — the mutation survives the entire suite
**Where:** `crates/btctax-core/src/tax/return_1040.rs:1354` (the guarded call); only test: `qbi_above_threshold_refuses` (return_1040.rs:2611), which exercises the REIT path exclusively.
**What:** Replacing `ar.printed_inputs.business_qbi` with `Usd::ZERO` in `screen_absolute` — i.e., deleting the entire new refuse behavior for Schedule C filers — passes all 1702 tests (**M3: `Summary … 1702 passed`**). No unit test, no golden household (both SE households sit below the threshold), no screen test covers a Schedule C business above $191,950/$383,900.
**Why it matters:** This is the load-bearing guarantee that LIMITATIONS.md and the amended SPEC §4.5 now advertise ("btctax **refuses** rather than guess at the wage-and-property limits"). If it regresses, an above-threshold miner is handed a simplified-8995 deduction where the law requires Form 8995-A's W-2-wage/UBIA limits — which can be **smaller**, i.e. a filed return with an overstated deduction and understated tax. The phase's claim to have "fault-injected every load-bearing KAT" is falsified by this mutation.
**Evidence:** M3 run above; test file grep shows `qbi_above_threshold_refuses` uses `box5_section_199a` only.
**Fix:** Extend `qbi_above_threshold_refuses` (or add a KAT) with a Schedule C household whose TI-before-QBI exceeds the threshold with **no** REIT dividends; assert `RefuseReason::QbiAboveThreshold`. Consider a golden household above the threshold asserting the refusal.

### [IMPORTANT] `make check` — the validation gate — exits 0 when the suite fails
**Where:** `Makefile:16-21` (commit 4429fdd).
**What:** The recipe backgrounds nextest and clippy and ends with bare `wait`. POSIX `wait` with no operands always returns 0. Empirically: I planted a deliberately failing test, ran `make check` → **`make-check-exit=0`** with 4 FAIL lines in the log.
**Why it matters:** The phase renamed this target "the validation gate" and the project's process (`STANDARD_WORKFLOW.md`, MEMORY) directs all validation through it. Any scripted use — git hooks, CI migration, an agent checking `$?`, `make check && git commit` — reports green on a red suite. That is an unmet guarantee of the gate itself.
**Evidence:** `sh -c '(exit 1) & (exit 0) & wait; echo $?'` → 0; the canary run above.
**Fix:** Capture PIDs and propagate: `@cargo nextest run --workspace & t=$$!; CARGO_TARGET_DIR=target-clippy cargo clippy … & c=$$!; st=0; wait $$t || st=1; wait $$c || st=1; exit $$st` (or make `check: test lint` and run with `-j2`).

### [MINOR] Stale documentation contradicts the new QBI behavior throughout qbi.rs / form8995.rs
**Where:** `qbi.rs:1-10` (module doc: "Crypto Schedule C business income is **not** §199A QBI in v1"), `qbi.rs:44-46` ("Lines 1–5 … are 0 in v1"), `qbi.rs:110-113` ("The Part I table … is BLANK: v1's only QBI is §199A REIT dividends"), `qbi.rs:121-126` (field docs "Always 0 in v1"), `form8995.rs:13-15`.
**What:** Five separate doc sites still describe the pre-P7 REIT-only model. The code beneath them does the opposite.
**Fix:** Rewrite alongside the Important-1 fix.

### [MINOR] advisories.rs test comments now mislabel the `agi` argument as "Schedule C net profit"
**Where:** `advisories.rs:341, 363, 396, 409, 420, 501, 516` (commit 0d2347f); plus a mangled "$66, 819" at line 471.
**What:** The `advisories` signature is unchanged (`agi: Usd`, doc: "`agi` = 1040 L11"), but the P7 commit rewrote the argument comments in seven tests to call that value "Schedule C net profit" — apparently residue of an abandoned refactor. The comments are simply wrong and will mislead the next maintainer of the EIC gate (which really does compare AGI).
**Fix:** Revert the comment edits; fix the "$66, 819" typo.

### [MINOR] The committed `qbi_deduction` oracle figures are never asserted
**Where:** `testonly.rs:371-393` (`ExpectedOts`/`ExpectedTaxcalc` omit the field; serde silently ignores it), `golden_returns.rs` line table.
**What:** Both oracles' `qbi_deduction` is committed in the JSON for every household but deserialization drops it. The cross-check pins AGI and TI, which constrains only the *sum* (deduction + QBI); asserting `ar.qbi_deduction` directly is free and closes the (far-fetched) compensating-error class.
**Fix:** Add the field to both structs and an eighth line to the comparison table.

### [MINOR] DECLARED_DIVERGENCES entries have no liveness check
**Where:** `golden_returns.rs:94, 254-258`.
**What:** Entries are consulted only when a mismatch occurs. If a future taxcalc release adopts Tax-Table semantics (or a household is renamed), an entry goes permanently dead with no signal — the mechanism's hygiene depends on it staying grep-clean.
**Fix:** Track which entries fired during the sweep and assert all fired.

### [NIT] Fixture realism labels
**Where:** `gen_goldens.py` / `testonly.rs` builders.
**What:** `mfj_two_w2_standard`'s `why` says "multi-W-2" but the builder makes ONE W-2; the MFJ-SE household's single W-2 carries box 3 = $220,000, above the $168,600 per-employer maximum (fine as a per-person *sum*, impossible on one W-2). Arithmetic is unaffected — all three engines were told the same thing.

### [NIT] Matrix gap: no golden household combines QBI with net capital gain
**Where:** the household matrix.
**What:** The 8995 line-13 net-capital-gain subtraction is validated only by unit KATs (which do catch M2), never by an oracle — the two SE households have no gains, the gain households no QBI. A `single` miner with an LTCG would close it next time the generator runs.

## Verdict

The core P7 claims survive adversarial scrutiny: the oracles are genuinely independent on everything except the OTS QBI *base* (which taxcalc independently confirms), the goldens regenerate bit-identically, all six divergences are confirmed against the printed 2024 IRS Tax Table, the QBI arithmetic is right against §199A, and the paper round-trip demonstrably catches wrong printed values. But the phase cannot close as-is: the filed Form 8995 is facially incomplete for exactly the households this phase added, its headline refuse guarantee is enforced by zero tests (a full-suite-surviving mutation), and the validation gate itself reports green on red.

**VERDICT: 0 Critical / 3 Important / 4 Minor / 2 Nit**
