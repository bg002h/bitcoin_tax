# Brief — independent SEAM review of the Form 4868 / 1040-V build (T1–T5 landed, T2–T4 landed, T6 landed)

You are an independent, adversarial BUILD REVIEWER for the Rust workspace `bitcoin_tax` (btctax),
in your own git worktree at the commit the controller names at dispatch. Read-only with respect to
the repo: do NOT edit source, do NOT commit, do NOT spawn subagents. You MAY run cargo/nextest —
first `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` (a warm cache on disk; never
build under /tmp), then `cargo nextest run --locked -p <crate> -E '<filter>'` for scoped runs. Never
run the whole workspace; it is green at the commit under review (the controller states the count).

## The one question
Does the Form 4868 / 1040-V build hold together ACROSS ITS SEAMS — map ↔ filler ↔ command ↔ export
hook ↔ manifest ↔ the surfaces — such that a filer who runs `btctax extension` and `export-irs-pdf
--pay-by-check` on a TY2024 return gets exactly the two documents the spec describes, and nothing
the spec forbids (a stapled voucher, a copy of the 4868 in the return's envelope, a line 5 that
includes the extension payment, a due-date warning computed from `return_due + 2 months`)? And
does every guarantee the build claims have a test that reds when the guarantee is removed?

## The contract
`design/SPEC_form_4868_1040v.md` (GREEN r6): R1 (rows and the naming deviation), R2 (the command,
its refusals in order, the warning rule), R3 (the payment already on the return), R4 (the voucher
rides loose), R5 (TY2024 is the year that computes), the provenance tables (lines 31–57) and the
journey table (lines 162–179 — every row is a behaviour to check). The three implementation
reports are the builders' own accounts: `design/agent-reports/2026-09-06-build-4868-T1-T5-implementation.md`,
`…-T2-T4-implementation.md`, and (if present) `…-T6-implementation.md`. Treat their claims as claims.

## Seams to examine, in order
1. **Map ↔ filler.** The 4868 map binds Part I by NAME and lines 4–8 by number; the voucher binds
   everything by name. Does each filler write every mapped cell it should and NO cell it should
   not (`c1_2`, the fiscal-year header, the page-3 confirmation cell, the voucher's three foreign
   cells)? Verify by READING BACK a filled PDF, not by reading the filler. The T1 report found that
   `f1_11`–`f1_14` collide across the two forms — confirm the fill of one form cannot land on the
   other's template (the year/stem selects the template; check `template()` dispatch).
2. **Filler ↔ the printed return.** Line 4 = Form 1040 line 24; line 5 = line 33 − Schedule 3 line
   10 (0 when `sch_3` is None); line 6 = max(0, 4−5), printed `0` when zero; line 7 default = the
   printed Schedule 3 line 10 when > 0 else line 6; box 3 of the voucher = line 37 (or `--pay`,
   never above line 37). Build a `PrintedReturn` from the kitchen-sink fixture
   (`btctax_core::tax::testonly`) and check the arithmetic against what the PDF prints.
3. **Command ↔ refusals.** R2's order: no params → no inputs → screen refuses → manifest.txt in
   `--out` → bad `--pay` → pseudo gate. Does each refusal write ZERO bytes (the directory must not
   exist or be empty afterwards)? Is the pseudo watermark on EVERY page of a 4868 (four pages)?
4. **The date rule.** `return_due` from the year record; `--out-of-country` ⇒ June 15 of the
   following year shifted by §7503. Check the shifter on a Saturday, a Sunday, a weekday; check the
   TY2017 pin (2018-06-15, not 06-17). Check that `BTCTAX_NOW` is the clock everywhere (no
   `now_utc()` in the new code).
5. **Export hook ↔ manifest.** `--pay-by-check`: voucher beside the packet, footer block SEPARATE
   from the stapling list, never in `FiledPacket::stapled`; no flag ⇒ nothing written + the note;
   line 37 = 0 ⇒ note, no voucher; slice year ⇒ refusal with the spec's sentence. Does the manifest
   line for the voucher say "ENCLOSE LOOSE — do not staple"? Does anything sort the voucher into the
   stapling order by accident (`attachment_sequence` now has explicit `None` arms — confirm)?
6. **Surfaces (T6).** `report` names the extension and voucher paths; help text names both `--pay`
   flags with their different ceilings and defaults (4868: above line 6 allowed; voucher: never
   above line 37); `YearReadiness::sentence` unchanged. `income import` doc unchanged.
7. **The kills.** For every test the reports name as a kill, ask: would it go red if the guarantee
   were removed? Plant at least these in your worktree and observe: remove the `c1_1` write (the
   out-of-country box); make line 5 include the extension payment; push the voucher into
   `stapled`; drop the §7503 shift. Revert each plant (`git checkout -- <file>` is fine in YOUR worktree).

## Already machine-verified (do not re-establish)
The suite is green at the commit under review; `make check` (nextest + clippy) passed in the
pre-commit hook of every commit in the range; the goldens moved only where the reports list.

## Severity (STANDARD_WORKFLOW.md)
Critical = wrong result / data loss / an unmet guarantee / a gate that cannot fail. Important = a
real defect, missing case, unsound assumption. Minor / Nit recorded, non-blocking. Secret-handling
never blocks. A red test you cause by running a scoped command IS a finding.

## Output — write the report as your FINAL action
`design/agent-reports/2026-09-06-build-4868-T1-T6-review.md` in your worktree: header (reviewer,
date, commit, tree state, commands run with their summary lines); per seam (1–7) what you checked,
verdict, findings; findings as `### <ID> (<Severity>) — <claim>` with **Where** (file:line), **What
is wrong** (the input/state that produces the wrong output), **Evidence**, **Minimal change**;
`Counts: C=<n> I=<n> M=<n> N=<n>`; a "What I did not examine" section. Return ONLY a 3-line
summary (counts, the single most important finding, the report path).
