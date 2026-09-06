# Brief — BUILD spec 1099-DA rule R6 (FR-62): the crypto slice files a live year from the stored answers

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore` files you did not
create. Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never `cargo test`, never
`--release`, never the whole workspace — the controller's gate runs `make check`); `cargo fmt --all`
and a clean `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee lands with a kill seen red once (plant, observe, revert via a `cp`
backup); the report says how, with the red text.

## The contract
`design/SPEC_1099da_broker_reporting.md`, the section **R6** (from "**R6 — the crypto slice files a
LIVE year…" to "## Current state"). It went through four review rounds
(`design/agent-reports/2026-09-06-spec-1099da-R6-review*.md` and their ledgers); the folded text is
the contract — build it AS WRITTEN. Where R6 names a function, signature, file or test by name, use
that name. Where the text and the tree disagree, the tree's real names win and you say so in the
report. Read R1, R2 and the T-plan of the same spec for the vocabulary (`screen_broker_reporting`,
`route_8949_boxes`, `broker_key_census`, `Form8949Box`, `InformationReturnRegime`).

## What R6 asks for (its own words are the spec; this is the map)
- **THE DISPATCH** in `crates/btctax-cli/src/cmd/admin.rs::export_irs_pdf_from_session` becomes three-way
  with the predicate `BundledFullReturnTables::load().full_return_for(year).is_none()` and the T9
  accessor; the `exists` branch keeps its early `return` only when params are bundled; arm (2) is
  reached by not returning early; the fourth cell (answers stored, params bundled, no committed row)
  is arm (3) with its own exit sentence.
- **T9**: `input_form_store::working_return(conn, year) -> Result<(Option<ReturnInputs>, Option<StaleNote>), CliError>`
  as a thin wrapper over `input_form_store::load` (draft shadows committed; stale split unchanged;
  `Loaded::Draft { parked: true, .. }` → `None`); `broker_answers(conn, year)` its projection; every
  arm-(2) gate reads that resolution; `Session::broker_reporting_answers` (`session.rs:594`) and
  `btctax export --csv` (`admin.rs:209`) resolve through it too; the StaleNote printed before the
  file list. No row is ever created; `resolve.rs` untouched; I-11 untouched.
- **Arm (2)'s gates, in order, before any byte**: the promote gate; the form-level gate over every
  map the SELECTED forms can reach (naming the first missing stem, `out_dir` never created); the
  pseudo-attestation gate; `screen_broker_reporting` (the slice's OWN refusal sentence carrying the
  same reason/detail); the Form 8283 restriction row in its slice form; the export-time price check
  `year_readiness::price_coverage_or_refuse(year)` (both arms); the re-worded `--pay-by-check` and
  `--forms full-return` refusals.
- **T8**: `btctax_core::forms::schedule_d_by_box(rows) -> BTreeMap<Form8949Box, ScheduleDPart>` from the
  ROUTED rows with the box→line table (1b = A|G, 2 = B|H, 3 = C|I, 8b = D|J, 9 = E|K, 10 = F|L);
  `fill_schedule_d_totals(totals, by_box, map)` / `fill_schedule_d(&totals, &by_box, year)`; the
  unbound-row refusal (`need`); the fifteen call sites (twelve `fill_schedule_d` + three
  `fill_schedule_d_totals`, named in R6 — the three B1 kills updated as transcription, none relaxed);
  `schedule_d.csv` with a `box` column and one row per (part, box); the three-artifact cross-check in
  `btctax-cli` reading the written CSV from a tempdir; the forms-side KAT re-scoped per box and renamed.
- **The surfaces**: the arm-(2) report note (the attachment-set wording); the two exit sentences;
  `uncomputable_sentence` and `import_note` clauses; `report` in states (2a)/(2b) with the flag to
  `render_tax_outcome` and the answers block reading the T9 resolution; the TUI export
  (`crates/btctax-tui/src/export.rs`) screening + routing BEFORE `mkdir_owner_only_exclusive`.
- **Every kill R6 lists** (the "Kills" paragraphs under T8, T9 and at the end of R6), run where they
  are observable: TY2025's templates with the LIVE regime injected (the pattern in
  `crates/btctax-core/tests/kat_broker_reporting.rs` and `crates/btctax-cli/tests/export_irs_pdf.rs`);
  TY2024 for "inputs + params → the full return still". The vault-building harness lives in
  `crates/btctax-cli/tests/export_irs_pdf.rs` / `tests/extension.rs`; the draft table is written via
  `input_form_store::save_draft` (see `crates/btctax-cli/src/input_form_store.rs`).

## Constraints
- Do not touch the spec or any `design/agent-reports/*.md` other than your report.
- `docs/examples/examples.md` and the man pages move only as a direct consequence (the readiness
  sentences, the new notes); regenerate (`cargo run -q -p xtask -- examples > docs/examples/examples.md`;
  `cargo run -q -p xtask -- docs`) and list every diff line in the report.
- Pinned counts you move: old → new with the cause. The census registers and `max_unwitnessed` do
  not move.
- If a sentence of R6 cannot be built as written, build the nearest thing that keeps the guarantee,
  and put the sentence, what you did instead and why in a "Deviations" section — never silently.

## Report — your FINAL action
`design/agent-reports/2026-09-06-build-1099da-R6-implementation.md`: per R6 item the change
(file:line), the kill and how it was seen red (red text), deviations, golden diff lines, pinned
numbers moved, the exact nextest commands with summary lines, anything you could not do. Return
ONLY a 3-line summary (what landed, suite results for the crates you touched, the report path).
