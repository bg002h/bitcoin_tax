# P9 IMPLEMENTATION RE-REVIEW — the r1 fold (`8b46b2b`), r2

**Reviewer:** Fable (independent — not the author). **Scope:** the fold of `reviews/P9-IMPL-fable-r1.md` only (I-1, I-2, M-1, Nit-1, Nit-2; Nit-3 recorded-only). **Gate:** `make check`. **Method:** full read of the fold diff and the touched source; both r1 mutations re-run live against the full suite; one additional empirical probe of the Nit-2 fixture; every experiment restored, tree verified clean and green after each.

**Verdict.** The fold genuinely closes both Importants with tests that are registry-derived and behavioural, not vacuous or hardcoded: each of r1's surviving mutations now dies to exactly one named test, added by this commit. The fold is purely additive on the test surface (nothing deleted from `tax_report.rs` or `answer.rs`), the one existing test it touched (`resolve.rs`) was strengthened, not weakened — I probed it and the refusal is now genuinely `HsaActivityUnsupported`, so its rewritten comment is true — and the M-1/Nit-1 text edits accurately match the code they describe. Gate green at HEAD: **1740 tests run, 1740 passed, 1 skipped**, working tree clean. **GREEN (0C/0I) — 0C / 0I / 0M / 0Nit.**

---

### I-1 — CLOSED (mutation-verified)

**The guard is now genuinely held.** Re-applied r1's exact mutation — `&& q.id != QuestionId::MortgageAllUsedToBuyBuildImprove` on the filter in `live_questions` (`/scratch/code/bitcoin_tax/crates/btctax-cli/src/cmd/answer.rs:95`) — and ran `make check`: **1739 passed, 1 failed — the sole failure is the new `cmd::answer::tests::income_answer_asks_every_live_declaration`** (in r1, this mutation passed 1736/1736). Restored; suite green; tree clean.

**Registry-derived, not a hardcoded list.** The test iterates `FORM_QUESTIONS` itself (`answer.rs:349-366`) and asserts, per entry, that `live_questions` asks it — exactly r1's prescribed fix. The `scenario_for` helper's `_ => {}` default is honest about future questions: a step-6–12 question whose liveness `single()` does not satisfy trips the test's *first* assertion (`"must be live in its own scenario (test bug otherwise)"`) — a loud named failure that forces the scenario to be extended, never a silent skip; an always-live addition is covered with zero edits. No drift channel remains.

### I-2 — CLOSED (mutation-verified)

**The guard is now genuinely held.** Re-applied r1's exact mutation — `return_inputs::get(s.conn(), year)?` → `.ok().flatten()` in `import_return_inputs` (`/scratch/code/bitcoin_tax/crates/btctax-cli/src/cmd/tax.rs:63`) — and ran `make check`: **1739 passed, 1 failed — the sole failure is the new `tax_report::import_over_a_stale_row_refuses`** (in r1, this mutation passed 1736/1736). Restored; suite green; tree clean.

**The three tests test what they claim; none is vacuous.** (`/scratch/code/bitcoin_tax/crates/btctax-cli/tests/tax_report.rs:1856-2063`)
- Stale means stale: `stale_the_row` sets `schema_version = 1` against `SCHEMA_VERSION = 2` (`return_inputs.rs:24`), and asserts the `UPDATE` hit exactly one row — the fixture cannot silently miss and leave the test asserting against a fresh row.
- **(a)** matches the error precisely (`StaleReturnInputs { year: 2024, .. }`), through the command — not the `Display` string.
- **(b)** is behavioural at both ends: `clear` must return `Ok(true)` on the stale row (so if `clear` ever grows a deserializing read, this fails), and the recovered row's version is read back by direct SQL, not through the very gate under test.
- **(c)** cannot pass as empty==empty: the precondition assert requires `carry_before` to contain a `Computed`-provenance carryover before staling, so the final `assert_eq!(carry_after, carry_before)` demands genuine reconstruction — and it compares full items including provenance, so a `User`-stamped rebuild would also fail. The chain exercises what matters: clear-on-stale must succeed, import-after-clear must succeed, and the write-back must restore the carryover `clear` discarded. The re-store of 2024's real inputs via `return_inputs::set` (rather than a second, fuller TOML) is a documented scaffolding shortcut (the in-test NOTE explains the filer's equivalent step) and does not hollow the property — every refusable step in the remedy still runs against the stale rows.

### The fold introduced nothing new

- **Additive only.** `git diff 8b46b2b~1..8b46b2b` deletes no test lines in `tax_report.rs` or `answer.rs`; no existing assertion was weakened anywhere.
- **Nit-2 fix verified empirically, not just read.** I temporarily strengthened `return_inputs_refused_by_guard_is_uncomputable_with_reason` to assert the refusal reason and ran it: it is `HsaActivityUnsupported` — the registry loop no longer fires first, so the rewritten comment ("the value-refusal refuses") is true. (Probe removed; the empty-SSN fixture is safe because `first_malformed_ssn` skips uncaptured SSNs by design, `return_refuse.rs:181-183`.) The fixture change makes the test *stronger* — the HSA refusal is now the sole defect.
- **M-1 comment matches the mechanism.** The corrected anchor comment (`questions.rs:201-207`) claims exactly what holds: exhaustive match ⇒ compile-forced human edit beside the hardcoded `len() == 8` tripwires; the index round-trip catches a mis-ordered `ALL`; and it now states the honest limit (arm-added-but-`ALL`-and-count-forgotten slips through) instead of overclaiming.
- **Nit-1 done and complete.** Both user-facing `unanswered_detail` strings dropped "(§2.9)"; the one remaining "§2.9" in a detail-adjacent string (`return_refuse.rs:931`) is a test assertion message — internal, correctly outside Nit-1's scope. No test asserted the old detail text.
- **Gate state:** `make check` green at HEAD `8b46b2b` — 1740 run, 1740 passed, 1 skipped, ~7s — matching the commit's claim; `git status` clean after all experiments.

No new findings. **GREEN (0C/0I).** Steps 1–5 pass this gate; Nit-3's step-8 expiry discipline (the interim understatement window) remains recorded and must stay open until step 8 lands.
