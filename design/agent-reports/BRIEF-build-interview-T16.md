# Brief — interview build T16: Form 8889 (Health Savings Accounts), transcribed — owner ruling FR-76

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore`/`git stash` (revert a
plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never
`cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and a clean
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee lands with a kill seen red once that RUNS the instrument; quote
the red. Every pinned number moved: old → new with cause. `/tmp` is a 32 GB tmpfs. Network is
available for the archive fetches (`curl -A btctax-archive`). Synthetic identifiers: SSNs from the
never-issued space (area 000/666, group 00, serial 0000); EINs only from
`scripts/pii-scan-generic.sh`'s `ALLOWED_EIN`. The Python stack is `.venv/bin/python`. Process in
force (owner S6): ONE seam review and ONE re-verification after you.

★ **The standing rules that matter most here.** TRANSCRIBE the form — one field per numbered line,
named for the line, in the form's own numbering, the official instruction text verbatim as the doc
comment, read from the TEXT LAYER (`pdftotext -layout`), never the rendered page; if the form asks
something our input surface cannot answer, COLLECT it. A derived or closed form is allowed only
with a written equivalence proof naming the branch where it breaks and a KAT pinning that branch.
Every line has a determinate PROVENANCE; blank because the inputs say so is correct. No decision
keys on a list you typed beside derived data. A kill CALLS the instrument it protects. Two oracles,
never one. The answer is in the manual (`i8889`).

## The contract
`design/SPEC_interview.md` r2 **§7 row T16** (added 2026-09-07 on the owner's ruling), **R4** (a
document screen's boxes verbatim from the archived extract, each reaching a named line, the
`[boxes]` census per edition), **R2** (`line-coverage` on every printed line; a `Collected` line
names its `DocBox` / `FilerRecords`), **R3** (the census rows: the HSA information returns join
`DocumentCensus`), **R10** (`LEAF_SOURCE`; `record_answer` the only writer), **R14**; `CLAUDE.md`
"Transcribe IRS forms — never paraphrase them" and "Two oracles". `FOLLOWUPS.md` FR-76 (ruled).
Build AS WRITTEN from the FORM; the tree's real names win; deviations recorded.

## Settled facts (controller-measured at `2363a44c`)
- Reach lines, both years: Schedule 1 line 13 *Health savings account deduction. Attach Form 8889*
  (`design/forms/extract/f1040s1--2024.txt:62`, `--2025.txt:64`); Schedule 1 line 8f *Income from
  Form 8889* (`:30` / `:31`); Schedule 2 line 17c *Additional tax on HSA distributions. Attach Form
  8889* and 17d (the last-month-rule / eligibility additional tax, *Attach Form 8889*)
  (`f1040s2--2024.txt:84,87`, `--2025.txt:84,87`). W-2 box 12 code W = employer contributions to an
  HSA (`iw2w3--2024.txt:1527`); T5's W-2 section already collects box 12 codes.
- Archive availability on `irs-prior` (HTTP 200): `f8889--2024`, `f8889--2025`, `i8889--2024`,
  `i8889--2025`, `f5498sa--2024`, `f5498sa--2025`, `i1099sa--2024`, `i1099sa--2025` (the combined
  1099-SA/5498-SA instructions), `f1099sa--2025`; `f1099sa--2024` is 404 — the 1099-SA is a periodic
  revision; find the revision in force for TY2024 the way T2's fold did for Form 1098 (`irs-prior`
  picklist; the Wayback capture of the moving URL as a last resort, corroborated by digest) and
  record it in the note. Nothing of these is archived today.
- Today: `ReturnInputs.hsa_activity: Option<bool>` (`return_inputs.rs:906-911`) is a class-(A)
  declaration, live always; `Some(true)` refuses `RefuseReason::HsaActivityUnsupported`
  (`return_refuse.rs:276,2378`); the `HsaActivity` `FormQuestion` at `questions.rs:610`. The maps
  under `crates/btctax-forms/forms/2024|2025/` have no `f8889`; the year packages' `YEAR.toml`
  lists the forms a year bundles. The two oracles take no HSA input today
  (`scripts/oracle/gen_goldens.py`, `ots_direct.py` — 0 hits for `e03290`/`HSA`).
- The pattern to follow for a new form: T2's archive steps (`design/forms/README.md`), T5's
  document sections + `[boxes]` census with `Collected(FieldId)` joined by `xtask box-census`, the
  label reader for map cells, `line-coverage` productions per printed line, the transcription-struct
  rule (`Form6251Map`, `Schedule1A` are the models in-tree).

## What T16 delivers
1. **Archive** `f8889` / `i8889` (TY2024, TY2025), `f5498sa` (2024, 2025), `f1099sa` (the TY2024
   revision in force + 2025), `i1099sa` (2024, 2025) as authorities — notes with measured sha256 and
   bytes, extracts, geometry for the forms, `authority-manifest --regen` → OK; `revision_in_force`
   extended for the periodic ones.
2. **Form 8889 as a transcription struct** (`Form8889` per year revision if the line set differs —
   check both extracts): every numbered line of Parts I–III, named for the line, the instruction's
   text as its doc comment (`i8889`), the arithmetic lines COMPUTED from the lines the form names
   ("enter the smaller of", "subtract line N from line M"), the input lines COLLECTED from the
   documents or the filer's records with `FilerRecords { instruction_line }` — never a closed form.
   Part I: the coverage type (self-only / family) and months of eligibility, the year's contribution
   limit from the year's params (`FullReturnParams` gains the §223(b) figures with cites — the
   instruction's line-3 table, the additional contribution at 55+), employer contributions (W-2 box
   12 code W, read from the W-2 rows), qualified HSA funding distribution, the deduction on line 13
   → Schedule 1 line 13. Part II: distributions (1099-SA box 1), unreimbursed qualified medical
   expenses (filer's records), the taxable amount → Schedule 1 line 8f, the additional tax →
   Schedule 2 line 17c (the exceptions checkbox on line 17a as the form prints it). Part III: the
   last-month rule / testing period failure → Schedule 1 line 8f and Schedule 2 line 17d.
3. **The document screens** (R4): `Form1099Sa` (boxes per the archived edition: 1 gross
   distribution, 2 earnings on excess contributions, 3 distribution code, 4 FMV on date of death, 5
   HSA/Archer/MA MSA checkbox) and `Form5498Sa` (boxes 1–6) as repeating sections with the `[boxes]`
   census complete per edition; `DocumentCensus` rows `sa_1099`, `sa_5498` (supported) with the
   three rules and `requires_transcription` decided; the `hsa` census presence joins
   `hsa_activity`: `Some(true)` now OPENS the section instead of refusing; `Some(false)` with any
   HSA row refuses the contradiction; the W-2 code-W amount is read, never re-asked.
4. **The map and emitter** for `f8889` (TY2024 and TY2025 through the label reader, every cell
   censused; `YEAR.toml` gains the form; the packet's attachment sequence places it as the
   instructions' attachment sequence number says), Schedule 1 lines 8f/13 and Schedule 2 lines
   17c/17d mapped and filled (they are `unmodeled` today — their census entries retire, with the
   direction tables' blocks unchanged; the `covered_by` join's counts fall by exactly those lines).
5. **The oracles**: `GoldenInputs` gains the HSA deduction (Tax-Calculator `e03290`; find OTS's
   HSA line in its template); a golden household with an HSA reconciles on both; `project_to_golden`
   (T11 — if T11 is not yet built, add the leaf to `ORACLE_INVISIBLE`'s predecessor list with a
   note, and say so); the sweep's excuse mechanism if either oracle omits a Form 8889 line.
6. **Refusals only where the form sends the filer elsewhere**: Archer MSA / Medicare Advantage MSA
   (Form 8853) — refuse naming the form; a line the instructions route to a worksheet the build
   does not carry — refuse naming the worksheet; nothing silent.

## Kills (each seen red once)
`line-coverage` on every Form 8889 line (a deleted production or a one-character quote drift reds);
the `[boxes]` censuses for 1099-SA and 5498-SA complete per edition (a deleted entry reds; a caption
the extract does not carry reds; `Collected` naming a field outside the section reds — the T5
instrument); the year's contribution limit from params (plant a contribution over the limit → the
excess line non-zero and the additional-tax path or the refusal the form prescribes, cite it);
`hsa_activity = Some(true)` with no HSA row refuses `DocumentDeclaredNotTranscribed`-style, with one
1099-SA row passes and fills Part II; `Some(false)` with a row refuses; a distribution not for
qualified expenses reaches Schedule 1 line 8f AND Schedule 2 line 17c on a fixture (both oracles
agree, or the excuse names the mechanism); the golden corpus's HSA household reconciles on both
oracles; the TY2024 map cells read back from the filled PDF; `census-join` counts fall by exactly
the lines now modelled; the R15 greps clean; the opener seeds an HSA trustee as a payer identity
with every box blank.

## Constraints
- Never paraphrase the form; if a line "feels hard", stop and read `i8889` again — every line is an
  instruction someone can follow.
- The owner's filed TY2024 Form 8889 is the third witness — compared by the owner LOCALLY; nothing
  from it enters the repo. Your fixtures are synthetic.
- If context runs short: leave the tree compiling, fmt-clean and green, and say which numbered item
  and which form is unfinished — the controller dispatches a continuation.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T16-implementation.md`: per numbered item what
landed, the archived documents (stem, edition, revision read off the document, sha256 short,
bytes), the line table of Form 8889 as transcribed (line → production → reach), the two box census
tables, the params figures with cites, the oracle inputs added, every deviation, every kill with
its red text, every pinned number moved, suite lines per crate. Return only a 4-line summary plus
the path.
