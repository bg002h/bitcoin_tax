# Brief — residue sweep 1 (six small items, one agent, one verification)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore` files you did not
create. Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never `cargo test`, never
`--release`, never the whole workspace — the controller's gate runs `make check`); `cargo fmt --all`
and a clean `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee lands with a kill seen red once (plant, observe, revert via a `cp`
backup); the report says how, with the red text. Every pinned number moved: old → new with cause.
Process rule in force (owner S6, `STANDARD_WORKFLOW.md` §2): this build gets ONE seam verification
after you; make each item self-evidently correct and tested rather than leaving judgment to a reviewer.

## The six items (do all; each is independent — if one cannot be done, do the others and say why)

**1. FR-63 — the TUI commit modal's slice clause** (`FOLLOWUPS.md` FR-63; spec 1099-DA R6 T9).
On a params-less year the input form's commit says only that inputs are SAVED as a draft and to
finalize when tables publish (`crates/btctax-tui-edit/src/main.rs` ~1404-1408, the `CommitOutcome::
NoTables` status line — a no-wrap NOTICE kept ≤ ~104 chars, r1-M1, with a kill at ~10769-10777
asserting "no full-return tables", "SAVED as a draft" and "finalize" all render). Add the slice
clause WITHOUT breaking that invariant: a second NOTICE line (or a modal-body line — read how the
modal renders and pick the smaller change) saying the Form 1099-DA answers are held in the draft and
that `export-irs-pdf --tax-year <y>` prints the crypto slice from them. Show it only when the year's
answers are non-empty (the T9 accessor `input_form_store::broker_answers`) or the year's regime
reports basis. Kill: the rendered buffer contains the clause on a params-less basis year with a
seeded block, does not on TY2024, and the existing ≤104-char and three-needle kills still pass.

**2. The §7503 District of Columbia legal-holiday calendar** (`crates/btctax-forms/src/year_record.rs`
~185-200: the shifter models weekends only and records why). Add the holiday half: the federal
legal public holidays as observed (New Year's Day, Birthday of Martin Luther King Jr., Washington's
Birthday, Memorial Day, Juneteenth, Independence Day, Labor Day, Columbus Day, Veterans Day,
Thanksgiving, Christmas — with the Saturday→Friday / Sunday→Monday observance rule, 5 U.S.C. §6103),
Inauguration Day (January 20 every fourth year from 1965, when it falls on a weekday) and DC
Emancipation Day (April 16, observed Friday when Saturday, Monday when Sunday — the reason April 15
moves). Cite 26 U.S.C. §7503 and Treas. Reg. §301.7503-1(b) (a legal holiday in DC counts for every
filer) in the doc comment; do NOT fetch anything — the statute text already in the tree is enough,
and the holiday list is transcribed from 5 U.S.C. §6103 with the citation. Kills, each a DERIVED date
never typed as the expected constant's source: April 15, 2018 → 2018-04-17 (Sunday → Monday 16 =
Emancipation Day observed → Tuesday 17: TY2017's committed `return_due`, now DERIVED by the shifter
and asserted equal to the record); April 15, 2023 → 2023-04-18 (Saturday, Emancipation Day Monday 17);
April 15, 2026 → 2026-04-15 (Wednesday, unchanged); June 15, 2024 → 2024-06-17 (Saturday); a plain
weekday holiday (July 4, 2025 → July 7, 2025); a Sunday holiday observed Monday (Christmas 2022 →
Dec 26 → the 27th). Wire the shifter into `extension_due_date` (the 4868 warning) and wherever the
weekend-only fn was called; keep the weekend-only fn only if something still needs it.

**3. Cite-check fixtures for the four 4868 / 1040-V (form, year) pairs** (`crates/xtask/src/cite_check.rs`
`FORMS`, `AUTHORITY_NOT_YET_ARCHIVED`; the one existing pair is `f1040s1a/2025` with
`crates/btctax-core/src/tax/fixtures/schedule_1a_2025_{form,instructions}.txt`). Read how `FORMS` and
`extract` consume a pair (`instr_pages`, `extract_stem`), then add rows for `f4868/2024`, `f4868/2025`,
`f1040v/2024`, `f1040v/2025` with fixtures produced from the archived extracts
(`design/forms/extract/<stem>--<year>.txt`; the form IS its own instructions document — the
instructions fixture is the pages the map rows declare, [1,4] / [1,2]). Remove the four pairs from
`AUTHORITY_NOT_YET_ARCHIVED` (the list shrinks 40 → 36 excused; `authority_coverage_may_only_improve`
must stay green and the count pins move DOWN with the cause). Kill: the ratchet reds if a pair is
re-added to the excuse list while its fixtures exist (or whatever the existing planted-defect test
for that ratchet does — extend it).

**4. A named kill for the full return's Section-B unanswered-restriction row**
(`crates/btctax-core/src/tax/return_1040.rs` ~2683-2695: `donations_had_restrictions == None` on a year
filing a Section B 8283 → `DonationRestrictionsUnresolved`). Today only incidental tests red under a
plant (the R6 fold report says so). Add one test named for the row: unanswered + Section-B-sized
donation (claimed noncash > $500 AND a single donated item > $5,000) → refuses; unanswered +
Section-A-sized → does not; answered `false` + Section-B-sized → does not. Seen red by planting the
`None` arm away.

**5. The orphaned doc block.** In `crates/btctax-cli/src/cmd/admin.rs` the doc comment written for
`slice_broker_refusal` (~2208-2276) sits above `slice_map_gate` (:2277) while the fn is at :2364.
Move the block onto its function; give `slice_map_gate` and `first_unresolved_map` the one-line docs
they lack, if they lack them. No behaviour change; `cargo doc -p btctax-cli --no-deps` warnings-free
for that file is the check.

**6. Archive `f8995a--2025` as an authority** (evidence only — NO map, NO filler, NO bundled template).
The IRS serves it: `https://www.irs.gov/pub/irs-prior/f8995a--2025.pdf` (HTTP 200 measured at
dispatch). Follow the exact pattern of the 2026-09-06 archive of `f4868--2024` (commit `b60c600c`):
fetch to `design/forms/2025/f8995a--2025.pdf` (gitignored), write the `.pdf.txt` note with the
measured sha256 and byte count in the existing note convention, `pdftotext -layout` to
`design/forms/extract/f8995a--2025.txt`, `cargo run -q -p xtask -- extract-geometry f8995a--2025`,
`cargo run -q -p xtask -- authority-manifest --regen` then `authority-manifest` (must say OK). Verify
the year on the form's own text (the masthead says 2025) before committing to the note — the
archiver's rule is that the year is read off the document, never assumed. Then run
`cargo run -p xtask -- port-status 2025 2026-DRAFT` and, if the printed table changes (the f8995a row
gains a prior side), regenerate `design/TY2026_WORK_LIST.md`'s table from it and say what moved.

## Constraints
Do not touch the spec files or any `design/agent-reports/*.md` other than your report. Goldens and
man pages move only as a direct consequence (list any diff lines). Keep the census registers and
`max_unwitnessed` unmoved (item 3 moves only the cite-check excuse count).

## Report — your FINAL action
`design/agent-reports/2026-09-06-build-residue-sweep-1-implementation.md`: per item the change
(file:line), the kill and its red text, deviations with reasons, every pinned number moved, the exact
nextest commands with summary lines, anything not done and why. Return ONLY a 3-line summary.
