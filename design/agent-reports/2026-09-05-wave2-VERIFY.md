# Wave 2 — adversarial verification

**VERDICT: all five fixes HOLD. 0 Critical / 1 Important / 6 Minor.** Every claimed test exists,
every load-bearing plant I re-ran went RED, no guarantee was weakened, and the real TY2026 year
transition fails CLOSED across all three crates. The one Important is a *claim*, not a fix: the F5
report's mitigation sentence for the `regen` sha fallback is false, and the underlying hole is wider
than the 15 entries the report scopes it to.

## Method (everything below was run, not read)

Scoped runs only; no `make check`, no workspace run, no `git`. Restores by `cp` from
`$SCRATCH/backup/`, verified by `md5sum` — **all eight owned files and the three data files I
planted into are byte-identical to the state I found them in**, and the tree is green after restore:
`cargo nextest run -p btctax-forms` 307/307, `-p xtask` 110/110 (1 pre-existing `#[ignore]`),
`cargo fmt --check` clean, `cargo clippy -p btctax-forms -p xtask --tests` clean,
`.venv/bin/python scripts/archive_drafts.py --self-test` PASSED.

Plants re-run by me (each RED, then reverted):

| # | plant | test that went red |
|---|---|---|
| 1 | `Form8275Map::for_year` regains `2017 \| 2024 \| 2025 \| 2026 =>` | `sp4::for_year_answers_exactly_what_the_bundled_asset_registry_answers` |
| 2 | `#[serde(deny_unknown_fields)]` removed from `Form8275Map` | `sp4::an_unknown_key_in_a_map_is_a_parse_error_not_a_silent_drop` |
| 3 | `adjudicate_coverage` re-keyed on the form, year discarded (#13 itself) | `cite_check::a_prior_year_archive_does_not_discharge_a_new_year_obligation` (and, as the agent honestly reported, `authority_coverage_may_only_improve` still PASSED) |
| 4 | `emitted_form_years()` skips `f8995a` (R16 itself) | `the_emitting_surface_is_derived_and_carries_the_year` **and** `authority_coverage_may_only_improve` |
| 5 | LIVE: one hex character flipped in `design/forms/geometry/f1040s1a--2026-DRAFT.json` | `form_geometry::pdf_sha_tests::every_committed_geometry_fixture_matches_the_manifest` |
| 6 | LIVE DATA: `"extract"` blanked on the `f6251--2026-DRAFT` manifest row | 3 red: `no_manifest_entry_hides_a_text_layer_that_exists_on_disk`, `every_archived_draft_records_a_text_layer_that_still_says_draft`, `every_manifest_entry_resolves_and_hashes_true` |
| 7 | `extract_path()` returns the note path (the shipped F5 defect) | `archive_drafts.py --self-test`: *"record refused to write — the provenance note and the text layer resolve to ONE file"* |
| 8 | LIVE DATA: `[census]` truncated out of `forms/2024/f1040.map.toml` | `supported_years_cross_product::the_cross_product_matrix_matches_the_recorded_gaps` |

Plants the agents did **not** run, which I added because they are the actual event these fixes exist for:

| # | plant | result |
|---|---|---|
| 9 | a real `crates/btctax-forms/forms/2026/` directory appears (f1040 pdf + map) | `cite_check::authority_coverage_may_only_improve` RED (*"btctax can print [f1040--2026] with no archived primary source for THAT YEAR"*), `the_template_years_are_exactly_the_supported_years` RED, `supported_years_and_bundled_year_directories_agree` RED |
| 10 | …and then `SUPPORTED_YEARS` gains 2026 (the full port move) | `the_cross_product_matrix_matches_the_recorded_gaps` RED, `the_unsupported_year_refusal_names_exactly_the_supported_years` RED, `sp4::map_year_matches_bundled_pdf_fieldset_for_every_supported_year` RED |

Independent re-measurements (Python, off the tree, not from any report): 37 `(form, year)` template
pairs / 18 stems ✓; `AUTHORITY_NOT_YET_ARCHIVED` = 36 pairs, +1 archived = 37 ✓; 47 geometry
fixtures, 47/47 matching MANIFEST, 0 unbacked, 3 `kind:form` entries with no fixture
(`f8275r--2025`, `f8615--2025`, `Form_1099-DA`) ✓; MANIFEST 128 entries / 106 with an extract / 78
note-storage, 79 extract files, all 15 TY2026 note sha256 == manifest sha256 ✓; 27 of 37 maps carry
`[census]` ✓; 38 `Deserialize` items in `map.rs`, exactly one without `deny_unknown_fields` (the
`untagged` `MoneyCell`) ✓ and **no** `Deserialize` type anywhere else in `btctax-forms/src` ✓.

## Critical

**None.** No fix emits a wrong number, and no claimed guard was found structurally unable to fail.

## Important

### I-1 — `crates/xtask/src/authority_manifest.rs:871` (and `:885`): `regen`'s "trust the note" fallback can read **none** of the 78 committed notes, and the F5 report says the hole is moot

`regen` falls back to the note when the gitignored binary is absent:

```rust
let note_sha = note_text.lines()
    .find_map(|l| l.split_once("sha256:").map(|(_, s)| s.trim().to_string()))
    .or_else(|| note_text.lines()
        .find(|l| l.trim().len() == 64 && l.trim().chars().all(|c| c.is_ascii_hexdigit()))
        .map(|l| l.trim().to_string()))
    .unwrap_or_default();
let (sha256, bytes) = match sha256_of(&abs) { Ok(v) => v, Err(_) => (note_sha, 0) };
```

Every committed note — including the 15 this fix newly wrote, via its own `note_text()` — spells the
digest `# sha256  <hex>`: **no colon, and the line trims to 74 characters, not 64.** Neither branch
matches. Measured over the whole archive: **0 of 78 `*.pdf.txt` notes are readable by this
fallback** (0 contain the substring `sha256:`; 0 contain a bare 64-hex line). So a
`xtask authority-manifest --regen` on any tree without the binaries — a fresh clone, a container, CI
— writes `sha256: ""` and `bytes: 0` for all 78 note-storage entries.

Nothing in `verify()` fails on an empty `sha256`: the `Storage::Note` arm hashes the binary only
`if abs.is_file()`, `NoteIsNotAProvenanceNote` passes (the notes *are* valid notes), and
`regen_would_drop` is keyed on paths and is, in this file's own words, "structurally blind to an
entry losing a FIELD". The geosha fix catches 47 of the 78 as `Drifted`; the remaining **31 go
silently unpinned**. On a dev tree with the binaries present the damage is caught by
`every_manifest_entry_resolves_and_hashes_true` — but that is the tree where the fallback never
fires, which is exactly the shape this wave was convened to remove.

The unbacked claim: the F5 report states *"Moot for TY2026 now the notes carry digests; the guard is
still absent."* Half of that is wrong. The notes carry digests in a format this reader cannot parse,
so it is not moot, and it was never confined to the 15 TY2026 rows. The evidence that this was
reachable in-session: **the same file, 530 lines earlier, already contains a digest detector that
reads the real note format** — `verify()`'s new `NoteIsNotAProvenanceNote` guard at `:341`,
`text.split(|c| !c.is_ascii_hexdigit()).any(|t| t.len() == 64)`.

**Minimal fix.** Two lines. Replace the `note_sha` extraction with the `:341` hex-run scan, and add
a `Problem` for a manifest entry whose `sha256` is not 64 hex characters, so a blanked pin cannot
survive `verify()` on any tree. (A `sha_coverage` ratchet of the `url_coverage_may_only_improve`
shape would be the belt-and-braces, but the two lines close the reachable hole.)

## Minor

### M-1 — `crates/xtask/src/form_geometry.rs:645`: the fixture walk has no floor
`every_committed_geometry_fixture_matches_the_manifest` guards vacuity with `!paths.is_empty()`
only. Its sibling at `:473` asserts `checked > 30` for the same directory. A geometry tree reduced to
one fixture passes. **Fix:** `assert!(checked > 40, …)`, the same shape as `:473`.

### M-2 — `crates/btctax-forms/src/map.rs:1098`: the doc comment names a test that does not exist
`alias_is_licensed_by`'s rationale points a reader at
`alias_refuses_a_year_whose_bundled_8275_is_a_different_document`. The test is
`sp4::the_8275_alias_is_licensed_by_the_asset_not_by_the_calendar`. A stale cross-reference in the
one comment whose job is to say "here is where you can watch this fail". **Fix:** rename the
reference.

### M-3 — `crates/btctax-forms/src/pdf.rs:207`: the surviving Form 8275 year list is still a silent one-token widening
`2017 | 2024 | 2025 => Ok(F8275_PDF_2024)`. Adding `| 2026` there **alone** reds nothing: the
cross-product matrix iterates `SUPPORTED_YEARS`, so a 2026 cell never enters it, and
`Form8275Map::for_year(2026)` would then return the Rev. 10-2024 map stamped 2026. It is caught the
moment 2026 also joins `SUPPORTED_YEARS` (plant 10 above, via `KNOWN_ALIASES`), so the exposure needs
two edits and no filer path reaches it today. Disclosed by the maps agent; recorded here as the
residual. **Fix:** have `f8275_pdf` ask `SUPPORTED_YEARS`, or add an unsupported-year cell probe to
the cross-product.

### M-4 — `scripts/archive_drafts.py:736`: F5's headline B1 kill is run by no gate
`_note_extract_collision_kill` is wired into `--self-test`, and `--self-test` appears in neither
`Makefile` nor `.github/workflows/ci.yml`. It reds correctly when planted (row 7 above) — it just
never runs unless a human types it. Pre-existing shape, inherited rather than introduced. **Fix:**
add `.venv/bin/python scripts/archive_drafts.py --self-test` to the `check` target, or port the
collision assertion into `authority_manifest.rs` where the suite already reaches it.

### M-5 — `crates/xtask/src/authority_manifest.rs:1034`: `every_problem_class_is_caught` is a plant list, not an exhaustive match
The name promises "every class"; the body is seven hand-written plants. Adding a `Problem` variant
reds nothing. **Fix:** end it with an exhaustive `match` over `Problem` whose arms are the plants
already written, so a new variant fails to compile.

### M-6 — `design/TY2026_PORT_REPORT.md:124` and §2: two counts in the port report are now known-wrong
The report says 48 geometry fixtures (47 on disk, independently re-counted here) and 16 TY2026
manifest entries carrying `extract: ""` (15 — `f1040`, `f8275`, `f8283` were correctly refused by the
archiver). Both were found and disclosed by the wave-2 agents and neither was corrected, so the next
reader inherits two numbers that do not match the tree. **Fix:** correct the two rows.

## Ownership (brief item 4)

- **No wave-2 edit found in either fleet-owned file.** `crates/btctax-core/src/tax/testonly.rs` and
  `crates/btctax-adapters/tests/shipped_tables_are_the_validated_tables.rs` are both modified in the
  working tree, but neither contains a single wave-2 marker (`SUPPORTED_YEARS`,
  `deny_unknown_fields`, `CensusDecision`, `alias_is_licensed_by`, `emitted_form_years`,
  `pdf_sha256` — 0 hits in each), no wave-2 agent claimed them, and their content is validated-table
  work. `git` was off-limits, so this is content evidence rather than a diff.
- Also modified but claimed by no wave-2 agent, listed for the controller to attribute:
  `crates/xtask/src/archive_check.rs`, `design/ROADMAP_STATUS.md`,
  `legal/_scripts/fetch_inflation_revprocs.sh`, and the four
  `legal/primary-sources/federal-register/SSA_COLA_Determinations_*.pdf` / `legal/text/…` additions.

## What I could not weaken

- `SUPPORTED_YEARS` is `[2017, 2024, 2025]`, unchanged; `lib.rs` gained doc lines only.
- No ratchet was widened to buy green: `AUTHORITY_NOT_YET_ARCHIVED` went from form-keyed (which
  excused *every* year of a form) to `(form, year)` with every year typed out — strictly narrower —
  and `KNOWN_GAPS` matches its measurement exactly in both directions (a gap that *closes* fails too;
  I confirmed with plant 8 and with the agent's own `the_gate_reds_on_every_planted_defect`).
- `map_year_matches_bundled_pdf_fieldset_for_every_supported_year` still compares the same bytes
  three times **today**, because Form 8275 genuinely aliases one Rev. 10-2024 asset to all three
  years — but it now asks `bundled_pdf(year)` and asserts it covered every `SUPPORTED_YEAR`, so it
  stops agreeing the moment a year ships a different revision. That is the honest version of the
  fix, not a residual defect.
