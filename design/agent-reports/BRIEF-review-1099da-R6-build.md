# Brief — independent SEAM review of the R6 build (the crypto slice files a live year from the stored answers)

Independent, adversarial BUILD REVIEWER in your own worktree at the commit the controller names.
Read-only: no source edits, no commits, no subagents. `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`
before any cargo command; scoped `cargo nextest run --locked -p <crate> -E '<filter>'` only; never the
whole workspace (green at the commit under review — the controller states the count).

## The one question
Does the R6 build make the crypto slice a CORRECT, FAIL-CLOSED filing path for a live year from the
stored answers — such that a filer with a TY2026-shaped vault (exchange dispositions, a draft with
the Form 1099-DA block answered, no full-return parameters) gets Form 8949 page-sets boxed from THEIR
answers, a Schedule D whose per-box lines agree with those page-sets and with the CSV, and a refusal
before any byte whenever a key is unsettled, a map is missing, a draft is stale, or a restricted
donation would print — and that no other surface (the viewer's Box column, the TUI export, `export
--csv`, `report`) can show or file a different answer set than the one the export used?

## The contract
`design/SPEC_1099da_broker_reporting.md` **R6** (five rounds; the r4 ledger says why the spec loop
ended and the build is the gate). The implementer's account is
`design/agent-reports/2026-09-06-build-1099da-R6-implementation.md` — its claims are claims.

## Seams to examine, in order
1. **The dispatch.** Write the decision table (answers stored? · committed row? · params bundled? ·
   templates bundled? · regime · exchange rows?) from the code, and check every cell against R6's
   arms 1–3 and the fourth cell. Plant: force `full_return_for` to `Some` on a slice year and confirm
   arm (1) takes over; force `is_none` on TY2024 and confirm arm (2) runs with routed boxes.
2. **T9 and the readers.** `input_form_store::working_return` / `broker_answers` over `load`: draft
   shadows committed; stale WIP discarded + `StaleNote` printed; stale parked refuses; parked yields
   `None`. Then EVERY reader: the CLI export, `export --csv`, the TUI export, `Session::broker_reporting_answers`
   → the viewer's Box column, `report`'s answers block. Construct the two-way vector (committed
   `basis_matches` + draft `proceeds_only`) and read each surface — they must all say I. Plant a
   `return_inputs::get` in one reader and confirm its kill reds.
3. **Arm (2)'s gates, before any byte.** For each gate R6 lists, produce the state that trips it and
   confirm `out_dir` does not exist afterwards (the promote gate; the form-level gate over the
   selected forms with a partially ported fixture year; the pseudo gate; the broker screen with its
   OWN sentence; the 8283 restriction row on a draft-only vault; the price-coverage check; the two
   re-worded refusals).
4. **T8.** Read back a filled Schedule D on a G+I fixture: line 1b = the G page-set total, line 3 = the
   I total, `1b + 3` = the Part I total; a TY2024 slice keeps line 3 whole and 1b blank; the
   unbound-row refusal; `schedule_d.csv`'s partition; the cli-side three-artifact check and the
   forms-side half — plant a CSV writer revert and a moved PDF line.
5. **`report` in states (2a)/(2b)** and the readiness sentences (`uncomputable_sentence`,
   `import_note`): exit codes, the answers block present, the clause's condition.
6. **The kills.** Every test the report names as a kill: would it go red if the guarantee were
   removed? Plant at least: the routing call on arm (2); the draft-over-committed precedence; the
   parked → None; the 8283 gate reading `return_inputs::get`; the form-level gate; the CSV partition.

## Already machine-verified (do not re-establish)
The suite is green at the commit under review; `make check` passed in the pre-commit hook; the
goldens moved only where the report lists.

## Severity (STANDARD_WORKFLOW.md): Critical = wrong result / fail-open / unmet guarantee / a gate
that cannot fail; Important = real defect, missing case, unsound assumption; Minor/Nit non-blocking.
A red test you cause by a scoped command IS a finding.

## Output — your FINAL action
`design/agent-reports/2026-09-06-build-1099da-R6-review.md` in your worktree: header (commands + summary
lines); per seam (1–6) what you checked, verdict, findings; findings as `### <ID> (<Severity>) — <claim>`
with Where / What is wrong / Evidence / Minimal change; `Counts: C=<n> I=<n> M=<n> N=<n>`; a "What I
did not examine" section. Return ONLY a 3-line summary (counts, the single most important finding,
the report path).
