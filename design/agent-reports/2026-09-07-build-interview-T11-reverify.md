# Re-verification — interview T11 build + fold (sonnet, worktree, every kill planted)

Independent verifier, own git worktree `/scratch/code/bitcoin_tax/.claude/worktrees/agent-a5b3a9b046105b86b`,
at `ebe5bbec` (the T11 fold commit). Every plant below was made in this worktree from a `cp` backup,
reverted, and `touch`ed (FR-90); after every plant touching `crates/`, the affected binaries were
rebuilt explicitly (`target-review` for `cargo nextest`, the worktree's own default `target/debug` for
`btctax`/`btctax-oracle-harness`, since `scripts/oracle/check_return.py:110` hardcodes
`parents[2]/target/debug` and ignores `CARGO_TARGET_DIR`). No commits, no pushes, no subagents.
`export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` was set before every scoped
`cargo nextest`/`cargo build -p ... ` invocation used for the Rust-level kills; the two harness/CLI
builds that `check_return.py` needed were built with `CARGO_TARGET_DIR` unset (into the worktree's own
`target/`), which is the only way that script can find the binary at all — noted as an environment fact,
not a deviation from the brief's intent (both are scoped `-p` builds, never `--workspace`).

Both oracles: OpenTaxSolver 2024 (`OTS_DIR=/home/bcg/OpenTaxSolver2024_22.07_linux64`) and PSL
Tax-Calculator 6.8.2 (`/scratch/code/bitcoin_tax/.venv/bin/python`, pandas 3.0.3 — this worktree has no
`.venv` of its own; git worktrees do not share it, so the main tree's interpreter was invoked against
this worktree's own scripts, which is safe since it is just an interpreter path).

Baseline confirmed clean before any plant: `cargo nextest run -p btctax-core --test oracle_projection
--test golden_returns --test kat_tax` → **99/99 passed**; `-p btctax-cli --test tax_profile` →
**13/13 passed**; `check_return.py --selftest` → exit 0, the controller's exact string.

---

## Build-report kills (K1–K7)

| kill | guarantee | plant | command | RED text (verbatim, trimmed) | verdict |
|---|---|---|---|---|---|
| K1 | `ORACLE_INVISIBLE` complete for a new unlisted `Usd` leaf | equivalent live plants: dropped `w2s[].box17/19` and `form_1098[].box6_points` from the projection (see "Four latent defects" below — same shape, same test) | `cargo nextest run -p btctax-core --test oracle_projection` | `every_money_leaf_either_projects_or_is_named_in_oracle_invisible` — *"these `Usd` leaves neither reach the oracle row nor appear in ORACLE_INVISIBLE: w2s[].box17_state_tax_withheld, w2s[].box19_local_tax"* / `form_1098[].box6_points` | **PASS (confirmed)** |
| K2a | no stale `ORACLE_INVISIBLE` entry | prepended `OracleInvisibleLeaf{prefix:"schedule_a.a_field_that_no_longer_exists", …}` | `cargo nextest run -p btctax-core --test oracle_projection -E 'test(no_oracle_invisible_entry_is_stale)'` | *"these ORACLE_INVISIBLE entries claim no leaf at all — the field they named is gone: [\"schedule_a.a_field_that_no_longer_exists\"]"* | **PASS (confirmed)** |
| K2b | no exemption over a live figure | prepended `OracleInvisibleLeaf{prefix:"w2s[].box1_wages", …}` | `cargo nextest run -p btctax-core --test oracle_projection -E 'test(every_money_leaf_...)'` | *"these leaves DO move the projection and are still listed as invisible — a stale exemption hides a real figure: w2s[].box1_wages"* | **PASS (confirmed)** |
| K3 | the dependents block round-trips | `dependents: Vec::new()` in `project_to_golden` | `cargo nextest run -p btctax-core --test oracle_projection` | 3 red: `the_projection_inverts_a_household_with_two_dependents`, `deleting_the_dependents_block_from_the_projection_reds_the_inverse`, `the_projected_row_carries_no_identity` (dependents `left:0 right:2`) | **PASS (confirmed, exactly 3 reds)** |
| K4 | the ledger census reports line 8v | `LedgerSink::Schedule1Line8vReported => {}` in `unprojected_ledger_lines` (post-fold code moved to a derived `ledger_income_sink` match; equivalent plant) | `cargo nextest run -p btctax-core --test oracle_projection -E 'test(the_ledger_lines_...)'` | `assertion left==right failed: []` `left: 0 right: 1` | **PASS (confirmed)** |
| K5 | line-19 excuse refuses any other size, offline | `actual_gap = expected_gap - asserted + 1` in `credit_line_verdict` | `.venv/bin/python scripts/oracle/check_return.py --selftest` | `AssertionError: (gap, expected) == (1,2500)` → got `(2,2500)` | **PASS (confirmed)** |
| K6 | line-19 excuse refuses any other size, live, both oracles | `lines["1040.line19"] = "1"` injected right after the harness read-back | `check_return.py --file row.json` (MFJ $420k household, real CTC child) | `CTC/ODC (L19) [taxcalc]  1  550  DIVERGES gap=549 expected=550`; exit 1 | **PASS (confirmed)** |
| K7 | the projected row carries no identity | assertion-side; measured on TWO real vaults (identity: Pat/Doe/ACME and Robin/Sam/Kim Hale/NORTHWIND/BIG BANK, real SSNs) | `grep -c` the identity tokens over live `income project` output | `0` hits in every case | **PASS (confirmed)** |

## Fold kills — the ROOT (C-2 + I-1)

| kill | plant | command | RED text | verdict |
|---|---|---|---|---|
| Reviewer's original Plant B — `charitable_gift_projects` made to always return `true` (this LEFT ALL 9 T11 KATs GREEN before the fold, the review's headline evidence) | same plant, replanted at HEAD | `cargo nextest run -p btctax-core --test oracle_projection --test golden_returns --test kat_tax` | `a_non_cash_gift_is_dropped_by_the_projection_and_reported_as_such` — `assertion left==right failed: left:6000.0 right:1000.0`; **98/99 passed, 1 failed** (byte-identical to the fold report's own reproduction) | **PASS — now reds, confirmed the root fix closed the gap the reviewer found green** |
| Fold's own Plant A — the `schedule_a.charitable[].class` `ORACLE_INVISIBLE` entry deleted | deleted the entry block | same command | `every_routing_fact_either_reaches_the_row_or_is_named_in_oracle_invisible` — *"schedule_a.charitable[].class = \"cap_gain_prop30\" (the description stops reproducing the return)"* / `"ordinary_prop50"`; **96/99 passed, 3 failed** | **PASS (confirmed, exact message match)** |
| `standard_or_itemized` never read (`GoldenDeduction::Standard` unconditional) | Rust KAT | `cargo nextest run -p btctax-core --test oracle_projection -E 'test(the_projection_inverts_build_golden_return_...)'` | `the projection is not the inverse of build_golden_return on mfj_itemized_over_100k — left: … Standard … right: … Itemized …` | **PASS (confirmed)** |
| `standard_or_itemized` never read — **live, both oracles**, on a real itemizing MFJ household built from the repo's own `ANSWERED_MFJ_RETURN` fixture (W-2 $420,000, SALT $1,000+$6,000 box 17, mortgage $30,000, one CTC dependent) | same plant, harness+CLI rebuilt | clean: `check_return.py --file row.json` → 13/13 lines OK, exit 0. planted: `check_return.py --file row_planted.json` | clean run: all OK, exit 0. planted run: `taxable income (L15) 387000 399800/387000 DIVERGES`; `deduction (L12) [OTS] 42000 29200 DIVERGES`; `1040sa.line5e … NO MECHANISM ON FILE`; **exit 1** — reproduces the review's exact C-2 evidence AND shows I-3's fix (no false default string) firing on the same run | **PASS (confirmed live, both engines)** |

## C-1

| kill | plant | command | RED / real output | verdict |
|---|---|---|---|---|
| `income_project_reports_the_screen_that_refuses_the_filers_own_return` | `"refused": Option::<serde_json::Value>::None` hardcoded in `project_return_inputs` | `cargo nextest run -p btctax-cli --test tax_profile -E 'test(income_project_reports_the_screen_...)'` | `panicked at tax_profile.rs:324: this fixture must be a return btctax refuses, else the test proves nothing` | **PASS (confirmed)** |
| Live journey, real vault: `btctax report` refuses (unanswered FBAR gate), `income project` still emits a full row + `"refused"` block, `check_return.py` exits 2 | none (real behaviour) | `btctax report --tax-year 2024` → exit 2; `income project --year 2024` → exit 0, row + `refused:{screen:"ScheduleBPart3Unanswered", …}`; `check_return.py --file row.json` | *"btctax REFUSES this return — the ScheduleBPart3Unanswered screen fired… EXIT=2"* | **PASS (confirmed live)** |
| Same row with the `"refused"` block hand-deleted (`row["refused"]=None`) | data plant | `check_return.py --file row_planted.json` | full 10-line comparison prints, *"Every compared line reconciles."*, **EXIT=0** — the exact pre-fold defect reproduced on demand | **PASS (confirmed — exit-2 contract exists AND its absence reproduces the original defect)** |

## I-2

| kill | plant | command | RED output | verdict |
|---|---|---|---|---|
| `--selftest` regression: pre-fold collapse (`asserted = 0 if printed is None else int(printed)`; `ok ⟺ btctax_value==0`) | replaced `credit_line_verdict` body with the pre-fold formula | `check_return.py --selftest` | `AssertionError: a btctax that printed the correct full credit was called a divergence` | **PASS (confirmed)** |
| FR-85 live plant: `CTC_PER_CHILD_SS24H2 = dec!(1000)` (understated ceiling), MFJ $420k/1 CTC-child household, both oracles live | plant + harness rebuild | `check_return.py --file row.json` | `CTC/ODC (L19) [taxcalc] 0 550 DIVERGES gap=550 expected=550`; *"btctax SWORE 0, taxcalc computed 550…"*; **exit 1** (pre-fold this reconciled at `OK (excused)`, exit 0) | **PASS (confirmed live)** |

## I-3

| kill | plant | command | RED output | verdict |
|---|---|---|---|---|
| `--selftest` case 9 (key-set pin) | deleted the `"1040.line24"` entry from `SINGLE_WITNESS_REASON` | `check_return.py --selftest` | `AssertionError: dict_keys(['1040.line19','1040.line27','8995.line12'])` | **PASS (confirmed)** |
| Live: same plant, real household | `check_return.py --file row.json` | | *"the witness census has no mechanism on file for 1040.line24 (witness: OTS)…"*; exit 1 | **PASS (confirmed live)** |

## I-4

| kill | plant | command | RED output | verdict |
|---|---|---|---|---|
| `every_filing_status_round_trips_through_the_oracle_row` | `FilingStatus::Mfs => "Mfs"` (pre-fold `Debug`-name fallthrough) | `cargo nextest run -p btctax-core --test oracle_projection -E 'test(every_filing_status_...)'` | `assertion left==right failed: left:{…,"Mfs",…} right:{…,"Married/Sep",…}` | **PASS (confirmed)** |

## M-1, M-2, M-3, N-1

| finding | check | command | result | verdict |
|---|---|---|---|---|
| M-1 (ledger sink derivation) | same plant as K4 (`LedgerSink::Schedule1Line8vReported => {}`) reds the identical test that also holds M-1's "every sink reached" guarantee | see K4 row above | reds as shown | **PASS (confirmed — same instrument)** |
| M-2 (`--year` overriding `tax_year`) | live: `check_return.py --file row.json --year 2025` | (real behaviour, not a plant — the guard itself) | *"--year 2025 contradicts the projection's own tax_year 2024… EXIT=2"* | **PASS (confirmed live)** |
| M-3 (§G-9 doc statement) | `grep -n "NOTHING BELOW IS VALIDATED BY ORACLE AGREEMENT"` | `grep` | found at `testonly.rs:2100` | **PASS (confirmed present)** |
| N-1 (`[taxcalc]` rows print `agree-taxcalc`, not `agree-ots`) | observed directly in every live `check_return.py` run above | (repeated observation) | e.g. `deduction (L12) [taxcalc] 42000 42000 OK agree-taxcalc`, `SALT (Sch A L5e) [taxcalc] … agree-taxcalc` | **PASS (confirmed live, repeatedly)** |

---

## The four latent projection defects (the routing-partition side-effect)

Each is a money leaf that was projected to the **wrong figure** (not merely uncensused) — silently
understating a real filer's description to both oracles. Confirmed via `git diff 46d4b2d6 ebe5bbec --
crates/btctax-core/src/tax/testonly.rs`, which shows the exact pre-fold buggy helpers
(`schedule_a_line5a`, `schedule_a_line8` at `46d4b2d6:testonly.rs:2382,2396`), and independently
re-planted at HEAD:

| defect | plant | RED (Rust KAT) | live figure change |
|---|---|---|---|
| Schedule A 5a omitted W-2 box17 (state)/box19 (local) tax withheld | reverted `state_income_tax` to the pre-fold formula (`salt_state_estimated_payments + salt_prior_year_balance_paid`, no W-2 term) | `every_money_leaf_either_projects_or_is_named_in_oracle_invisible` — *"w2s[].box17_state_tax_withheld, w2s[].box19_local_tax"* not carried, not listed | Live, real vault (fixture: $1,000 est. payments + $6,000 box 17): `state_income_tax` **7000.0 → 1000.0** — $6,000 silently dropped |
| Schedule A 8a omitted Form 1098 box 6 points, and did not zero for a §163(h)(3)(F) mixed-use mortgage | reverted `mortgage_interest` to the pre-fold formula (`Σbox1_interest` only, no points, no mixed-use zeroing) | TWO tests red simultaneously: `every_money_leaf_...` (`form_1098[].box6_points` uncarried) AND `every_routing_fact_...` (`schedule_a.mortgage_all_used_to_buy_build_improve = false` — *"moves the filed return, moves nothing in the row"*) | not separately re-run live (the test vault's mortgage is not mixed-use / has no points row); Rust-level kill is direct evidence over the actual production code path |
| `hsa_deduction` ignored `sch1.hsa_activity`, so a stale Form 8889 line-2 figure could describe a §223 deduction btctax does not take | made `hsa_deduction` unconditional (`f(ri.hsa.line2_contributions_you_made)`, no `hsa_activity` gate) | `every_routing_fact_either_reaches_the_row_or_is_named_in_oracle_invisible` — *"sch1.hsa_activity = false (moves the filed return, moves nothing in the row)"* | not separately re-run live (no fixture with a stale HSA figure + `hsa_activity=false` was built); Rust-level kill is direct evidence over the actual production code path |

All three plants reverted and re-confirmed green (`99/99`) before moving on. **Verdict: all four latent
defects real, all four fixed, all four hold a red-on-removal test.**

---

## Negative-claim checks

| claim | check | result |
|---|---|---|
| Every golden household round-trips | `.venv/bin/python scripts/oracle/check_determinism.py` (fresh regen off BOTH engines vs. committed, byte-for-byte, 107 households) | `PASS: fresh-regen and committed are byte-identical over 107 households … §12 determinism holds.` |
| `ORACLE_INVISIBLE`'s doc states the §G-9 limit, generalized | `grep -n "NOTHING BELOW IS VALIDATED BY ORACLE AGREEMENT"` | found, `testonly.rs:2100` |
| No excuse keyed by a vector/household name | `grep -nE '"(V[0-9]\|single_\|mfj_\|mfs_\|hoh_\|qss_)'` over `check_return.py`, `sweep.py`, `oracle_projection.rs` | one hit, `sweep.py:304`, which is a **build-freshness probe** (`_verify_harness_freshness`, looks up one named fixture to check the harness binary emits `reproduction_ok`), not a divergence excuse keyed by name — not a violation |
| The existing sweep reconciles | `sweep.py --seed 1 --count 8` (both oracles live) | `8 admitted, 0 skipped … 0 undeclared divergences` |
| Every new shaped identifier in fixtures is allowed | `bash scripts/pii-scan-generic.sh HEAD` | `pii-scan: clean (HEAD).` exit 0 |

---

## Final scoped confirmation (clean tree, full recompile)

`git status --porcelain` → empty; `git diff --stat` → empty (no residual plants).

```
$ find crates -name '*.rs' -exec touch {} +
$ cargo nextest run --locked -p btctax-core --test oracle_projection --test golden_returns \
    --test kat_tax -p btctax-cli --test tax_profile -p btctax-forms --test attestation
     Summary [ 0.410s] 122 tests run: 122 passed, 0 skipped
$ cargo nextest run --locked -p btctax-oracle-harness
     Summary [ 3.268s] 5 tests run: 5 passed, 1 skipped   (the skip is the environment's
                                                            PDF-less / stale-hook noise the
                                                            brief names — not a finding)
$ .venv/bin/python scripts/oracle/check_return.py --selftest
check_return: blank-vs-sworn, refusal, year and witness checks discriminate (B1 kill OK)   exit 0
```

## Disposition

Every kill named in the T11 build report (K1–K7), every kill in the fold report (the ROOT's two plants,
C-1, C-2, I-2, I-3, I-4), M-1/M-2/M-3/N-1, and all four latent projection defects the root fix
uncovered — **reproduced red under a fresh plant, or confirmed live, or both.** No kill failed to red.
No finding lacks a holding test. No negative claim was contradicted by the tree.

**Counts: C=0 I=0 M=0 N=0**
