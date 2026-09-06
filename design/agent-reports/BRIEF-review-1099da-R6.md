# Brief — one-round DESIGN review of spec 1099-DA rule R6 (S10 reversed)

You are an independent, adversarial DESIGN reviewer in your own worktree (read-only: no source
edits, no commits, no subagents; `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`
before any cargo command; scoped `cargo nextest run --locked -p <crate> -E '<filter>'` only).

## The one question
Is **R6** in `design/SPEC_1099da_broker_reporting.md` (the section headed "R6 — the crypto slice
files a LIVE year from the stored answers…") a SOUND and COMPLETE rule for the owner's ruling —
*"we will need to have option to file 2026 tax year with crypto sales"* — such that an implementer
building it exactly as written would give the filer a correct, fail-closed TY2026 crypto-slice
filing path, and would not break anything R1–R5 or the landed build guarantee? Judge the RULE, not
the prose style. One round: the controller folds your Criticals/Importants and builds.

## Read, in this order
1. `design/ROADMAP_STATUS.md` §0a — the S10 entry and its RULED paragraph (the owner's words).
2. `design/SPEC_1099da_broker_reporting.md` — R1 (liveness, the per-key answer, "what the declaration
   cannot see"), R2 (routing), R6, and the superseded N-3 bullet under R1.
3. The code R6 names: `crates/btctax-cli/src/cmd/admin.rs` — `export_irs_pdf_from_session` ("THE
   DISPATCH"), `export_full_return` and its `screen_full_return` prelude, `slice_broker_refusal`, the
   slice arm's routing; `crates/btctax-core/src/tax/return_refuse.rs::screen_broker_reporting`;
   `crates/btctax-core/src/forms.rs::route_8949_boxes`; `crates/btctax-cli/src/render.rs::write_form_csvs`
   / `routed_8949_rows`; `crates/btctax-tui/src/export.rs`; `crates/btctax-cli/src/year_readiness.rs::
   uncomputable_sentence` / `import_note`; `crates/btctax-cli/src/resolve.rs` (how `report` treats a
   year with inputs and no params).

## Lenses (answer each; a lens with nothing to report says so)
- **Journey.** Walk the owner's real path in the 2027 season with a TY2026 vault holding exchange
  dispositions: (a) the TY2026 8949/Schedule D finals are bundled, `FullReturnParams` TY2026 is not;
  (b) they open the TUI input form for 2026, answer the seeded Form 1099-DA block, commit; (c)
  `export-irs-pdf --tax-year 2026`. At each step: what exactly do they have, what does the tool do,
  what ELSE might they reasonably do (run `report` first; import a TOML with only the answers; have a
  self-custody-only vault; have a venue outside the four adapters; have a `mixed` venue; run the TUI
  export instead)? Classify each divergence: refusal / warning / default / not our concern / doc only.
- **Fail-closed.** Can arm (2) ever print a box the answers did not choose? Can it print when the
  full return would have refused for a reason UNRELATED to the broker answers (e.g. a screen the full
  return runs that the slice does not — `screen_inputs`, the promote gate, pseudo attestation)? Should
  any of those screens run on arm (2)? Which, and why?
- **State machine.** Enumerate the (inputs stored?, params bundled?, templates bundled?, regime,
  exchange rows?) product and state R6's outcome for every cell; find any cell R6 leaves undefined
  or where two rules disagree (R1's liveness, R6's arms, `SUPPORTED_YEARS`).
- **`report` and the readiness surfaces.** With inputs and no params, `report` refuses (uncomputable,
  inputs kept). Is that the right outcome once the slice can print? Does `import_note`'s sentence
  ("`report --tax-year N` will refuse (keeping them) until …") still tell the truth? What should the
  readiness sentence say?
- **The TUI export.** The viewer's Snapshot carries the stored answers (fold `c25f7489`); does the TUI
  export have everything arm (2) needs, and does its ordering (refusal before the exclusive mkdir) hold?
- **The finals gate.** R6 says TY2026 prints nothing until its 8949/Schedule D finals are bundled. Is
  that the only gate, or does the slice ALSO need a TY2026 `TaxTable` (Schedule SE?) or price data
  through 12-31 (`YearReadiness` reds a filable year whose dataset ends early — does that bind the
  slice export or only the `filable` status)?
- **Kills.** Are the listed kills sufficient to discriminate the rule from its nearest wrong
  neighbours (e.g. routing from the answers but skipping the unread check; printing the slice on a
  year WITH params)? Name any missing kill.

## Severity (STANDARD_WORKFLOW.md): Critical = a wrong result, a fail-open, an unmet guarantee;
Important = a real defect, missing case, unsound assumption; Minor/Nit non-blocking.

## Output — write the report as your FINAL action
`design/agent-reports/2026-09-06-spec-1099da-R6-review.md` in your worktree: header; per lens what
you checked and the verdict; findings as `### <ID> (<Severity>) — <claim>` with **Where** (spec
line / file:line), **What is wrong**, **Evidence**, **Minimal change** (spec wording, or a build
task); `Counts: C=<n> I=<n> M=<n> N=<n>`. Return ONLY a 3-line summary (counts, the single most
important finding, the report path).
