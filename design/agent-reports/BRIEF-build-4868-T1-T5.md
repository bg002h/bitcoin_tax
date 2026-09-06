# Brief — Form 4868 / 1040-V build, tasks T1 + T5 (the rows and the reader walk)

You are the single implementer for this task in the shared main tree `/scratch/code/bitcoin_tax`
(branch `main`, HEAD `b60c600c`). Do NOT spawn subagents. Do NOT commit; do NOT push; do NOT run
`git checkout --`/`git restore` on anything you did not create. The controller commits. Work only
on the files this brief names plus the tests it asks for; if you find you need to touch something
else, say so in your report rather than doing it.

Run tests with `cargo nextest run --locked -p <crate> -E '<filter>'` (never `cargo test`; never the
whole workspace at once — `make check` is the controller's gate). Use `cargo fmt --all` before you
finish. Do not use `--release`.

## The spec
`design/SPEC_form_4868_1040v.md` (GREEN r6). Read **R1** (line 65 onward: two new stems, four new
rows, zero new hand-lists), the two provenance tables (lines 31–57), and the Plan items **T1** and
**T5** (lines 183–188 and 217–237). The spec is the contract; where this brief and the spec
disagree, the spec wins and you say so in the report.

## What is already done (machine-verified, do not redo)
- TY2024 and TY2025 revisions of both forms are archived: `design/forms/{2024,2025}/{f4868,f1040v}--<year>.pdf.txt`
  notes (with sha256), text layers `design/forms/extract/{f4868,f1040v}--{2024,2025}.txt`, geometry
  fixtures `design/forms/geometry/…json`, manifest entries (`xtask authority-manifest` says OK).
  The PDFs themselves are on disk (gitignored): `design/forms/<year>/<stem>--<year>.pdf`.
- Field inventories (`cargo run -q -p xtask -- dump-fields <pdf>`): f4868 = 17 fields, f1040v = 15
  fields, both years. Differences 2024 ↔ 2025 on the 4868: the Part I subform is
  `Part1_ReadOrder` in 2024 and `PartI_ReadOrder` in 2025; the page-3 confirmation cell is
  `Page3[0].Col3[0].f3_1[0]` in 2024 and `Page3[0].Col4[0].f3_1[0]` in 2025; the `VoucherHeader`
  cells and the `f1_11..f1_14` amount cells and `c1_1`/`c1_2` checkboxes have the same FQNs. The
  1040-V's 15 FQNs are identical across the two years. ALWAYS take FQNs from `dump-fields` on the
  archived PDF of THAT year, never from this brief.
- Neither form prints an "Attachment Sequence No." (the extracts contain none: measured 0).
- The label reader today joins the 2025 4868's boxes as: `f1_11`→"4", `f1_12`→"5", `f1_13`→"6",
  `f1_14`→"7", `c1_1`→"8", `c1_2`→"9", and MISLABELS Part I: `f1_4`→"5", `f1_5`→"6",
  `f1_6/f1_7/f1_8`→"8", `f1_9/f1_10`→"9" (Part I's labels 1/2/3 never form a candidate column).
  On `f1040v--2025` it refuses: "no numbered label column found". This is why R1 binds Part I and
  every 1040-V cell BY NAME and only `line4`…`line8` by number, and why T5 exists.
- `line_bindings` in `crates/xtask/src/label_reader.rs` extracts ONLY keys spelled `line<digits>…`;
  named cells are skipped by the walk. `map_and_census` in `crates/btctax-forms/tests/field_census.rs`
  counts EVERY quoted FQN (contains `[0]` and `.`) outside `[census]` as mapped and inside it as
  censused, so a map's 17 / 15 fields must each appear exactly once in one of the two.
- The full suite is green at HEAD (3091 passed).

## T1 — the rows (do all of this)
1. `crates/btctax-forms/src/bundled.rs`: `Stem` gains `F4868` and `F1040v` (doc comments in the
   style of the others), in `ALL`, `file_stem` (`"f4868"`, `"f1040v"`), `from_file_stem`, and the
   Display impl if it matches per variant. Then let the compiler list every exhaustive `match` on
   `Stem` and decide each (there should be very few — `file_stem`/`from_file_stem` and `line_set::schema`).
2. `crates/btctax-forms/src/line_set.rs`: `LineSet` gains `F4868_2024`, `F4868_2025`, `F1040v_2024`,
   `F1040v_2025` with `parse`/`as_str` strings `"f4868/2024"` etc.; `Schema` gains `Form4868Map` and
   `Form1040VMap`; `schema()` maps the four revisions to them. The test
   `the_unwired_set_is_exactly_the_two_ty2025_maps_step_5_could_not_wire` must stay true (these are wired).
3. `crates/btctax-forms/src/map.rs`: two new map structs in the style of `Form8959Map`
   (`#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields)]`, the row keys `form`, `year`,
   `irs_stem`, `versioning`, `template_sha256`, `authority`, `extract_override`, `instructions`,
   `instr_pages`, `line_set`, `attachment_sequence`, then the bindings as TOP-LEVEL `String` fields,
   then `census` as the other maps carry it — copy the exact pattern one existing small map uses,
   including `for_year(year)` / `ty2024()` / `ty2025()` / `field_names()` helpers if those exist on
   `Form8959Map`):
   - `Form4868Map` fields: `name_line`, `address_street`, `address_city`, `address_state`,
     `address_zip`, `taxpayer_ssn`, `spouse_ssn`, `line4`, `line5`, `line6`, `line7`, `line8`
     (line 8 is the `c1_1` checkbox; carry its on-state the way other maps carry a checkbox —
     look at how `f8949.map.toml`'s `box_on`/`boxes` or the 1040 map's checkboxes are declared, and
     use the simplest existing convention). Doc comment on EVERY binding = the printed line number
     and the caption text VERBATIM from `design/forms/extract/f4868--<year>.txt` (line 84–91 region
     for 2025; find the same region in 2024 — note the year in "Estimate of total tax liability for
     2025" changes).
   - `Form1040VMap` fields: `box1_ssn`, `box2_spouse_ssn`, `box3_amount`, `box4_first_name`,
     `box4_last_name`, `spouse_first_name`, `spouse_last_name`, `address_street`, `address_apt`,
     `address_city`, `address_state`, `address_zip`. Doc comments from `f1040v--<year>.txt` lines
     66–82 (2025) and the 2024 equivalent.
4. Four map files: `crates/btctax-forms/forms/{2024,2025}/{f4868,f1040v}.map.toml`, plus the
   bundled template PDFs `crates/btctax-forms/forms/{2024,2025}/{f4868,f1040v}.pdf` as byte-for-byte
   copies of the archived PDFs (`cp design/forms/<year>/<stem>--<year>.pdf crates/btctax-forms/forms/<year>/<stem>.pdf`;
   `build.rs` globs the directory, so a `.pdf` without a `.map.toml` beside it, or vice versa, fails
   the build — add both). Row header per map: `form`, `year`, `irs_stem = "<stem>"`,
   `versioning = "annual"`, `template_sha256 = <sha256 of the bundled PDF, measured with sha256sum>`,
   `instructions = "<stem>"` (the form is its own instructions document — the IRS publishes no
   i4868/i1040v), `instr_pages = [first, last]` = the instruction pages of the form itself (4868:
   pages 2–4; 1040-V: page 1's instruction half is on page 1 and page 2 — measure from the extract's
   form-feed page breaks and record what you measured), `line_set = "<stem>/<year>"`, NO
   `attachment_sequence` key, NO `authority` key. Bindings as top-level keys named exactly as the
   struct fields. `[census]` accounting for EVERY remaining field: 4868 → `VoucherHeader.f1_1/f1_2/f1_3`
   (line = "header", rule = "unmodeled", reason: btctax is calendar-year only), `c1_2` (line = "9",
   rule = "unmodeled", reason: btctax produces no Form 1040-NR), `Page3 … f3_1` (line = "page 3
   confirmation", rule = "unmodeled", reason: the electronic-payment confirmation is the filer's
   private record); 1040-V → `f1_14/f1_15/f1_16` (foreign country/province/postal code, rule =
   "unmodeled", reason as `forms/2024/f1040.map.toml:202-204` words it). Put a header comment block
   in each map in the style of `forms/2025/f8959.map.toml` (what was measured, from which PDF, the
   dump-fields count, that doc comments are transcribed from the extract).
5. Year records: `forms/2024/YEAR.toml` `forms_expected` gains `"f4868"` and `"f1040v"` (17 → 19);
   `forms/2025/YEAR.toml` likewise (15 → 17); `forms/2017/YEAR.toml` and `forms/2026/YEAR.toml`
   `[forms_absent]` gain both with a reason (2017: `"fillers begin at TY2024; no 2017 authority
   archived"`; 2026: `"TY2026 revision not released; January 2027 package"` — match the file's
   existing phrasing). `tests/year_record.rs` partitions `Stem::ALL` per year, so all four records
   must change together.
6. The three row-gate edits (R1 / spec I-3):
   - `crates/btctax-forms/tests/map_rows.rs` ~line 298: the assertion "attachment_sequence is
     absent iff form == f1040" becomes "absent exactly on the rows whose extract prints none" —
     kill 4 in the same file already compares the row against `printed_sequence(extract)`, so
     widen the assertion to `matches!(form, "f1040" | "f4868" | "f1040v")` with a comment saying
     the printed-sequence check is what actually holds it.
   - same file ~line 337: `instr_pages` is pinned to one row (`(2025, "f1040s1a", [101,110])`);
     make it a set that also holds the four new rows with the pages you measured.
   - `crates/xtask/src/cite_check.rs` ~line 1378 (`every_archived_rows_instructions_stem_is_a_manifest_entry`):
     `instructions = "f4868"` already resolves to `design/forms/<year>/f4868--<year>.pdf`, which IS a
     manifest entry, so verify that test passes as is; if some other cite-check test enumerates
     `instructions` stems or `iNNNN` names, teach it these two.
7. Also grep for every other pinned count that a new stem or map moves and update it with the reason
   in the message: `crates/btctax-forms/tests/field_census.rs` (the new maps carry a full `[census]`,
   so NO register entry — but `UNCENSUSED_ENTRIES`/`UNCENSUSED_FIELDS` must not change),
   `crates/btctax-forms/tests/map_rows.rs` row counts (37 → 41 maps), `crates/xtask/src/cite_check.rs`
   `rows()` counts, `crates/xtask/src/label_reader.rs` (see T5), `tests/kats.rs` /
   `tests/broker_boxes.rs`-style field-set tests if they enumerate all maps, `sp2.rs`/`sp3.rs`
   "map matches bundled PDF fieldset" tests — add the same test for the four new maps (every
   `field_names()` entry is an AcroForm field of the bundled PDF).

## T5 — the reader walk (do this in the same pass; T1's 1040-V maps cannot land without it)
In `crates/xtask/src/label_reader.rs`, test `every_mapped_line_lands_on_its_own_printed_label`
(~line 1577): today a map whose `label_join` fails is pushed to `unwitnessed`, and a committed
`f1040v` map would move the `max_unwitnessed` ratchets (2024's 1 is spent on f8283; 2025's is 0).
Per spec T5 (r4 R4-I1): insert the GRID branch **after the `m.stem` geometry join succeeds and
BEFORE `label_join`**: compute `keys = numbered_line_keys(text)` there; when `keys == 0` AND
`GRID_MAPS` names `(year, form)`, push `"<form> — grid: <reason>"` onto a NEW third bucket
`YearReach.grid: Vec<String>` (printed like the others), and `continue` — neither witnessed nor
unwitnessed. A declared grid that has LOST its geometry fixture still hits the `m.stem` Err branch
above and is counted unwitnessed (that is the kill: `the_per_year_audit_reds_on_…lost_fixture`
style — add a planted case if the existing planted-audit test cannot express it). Every undeclared
keyless map still reaches `map_reach_problem`'s `(false, 0, …)` red via the label_join path or the
ratchet. Add `("2024","f1040v","…")` and `("2025","f1040v","…")` to `GRID_MAPS` with the measured
reason (captions printed ~21pt above their cells and, for box 3, ~130pt left — not line labels
beside them). **Do NOT raise either `max_unwitnessed`.** Run the test; it prints per-year
`joins`; the 4868 contributes 5 numbered keys per year (`line4`…`line8`). Update `YEAR_FLOORS`
`min_joins` ONLY upward and only to what the run measured, and paste the measured numbers and the
cause into the floor's comment. The spurious `9a` heading the reader may see on the 4868 gets a
recorded reason if it surfaces.

## Kills you must add (each must be seen RED once on a planted defect — say how in the report)
- the census accounts for 17 + 15 boxes per year (the field_census gate will do this; confirm it is
  green and that deleting one `[census]` entry from a scratch copy reds — the gate has a planted-
  defect test already; cite it);
- a row with an `attachment_sequence` whose extract prints none reds (map_rows kill 4 — plant it on
  a tempdir copy the way `a_planted_wrong_sequence_number_is_reported` does, for the 4868);
- `instr_pages` holds every row that declares pages (the set assertion);
- `Form4868Map::ty2024()/ty2025()` and `Form1040VMap::…` field names are exactly the bundled PDFs'
  AcroForm names (sp2/sp3 style);
- the T5 grid bucket: a keyless map NOT in `GRID_MAPS` still reds; a declared grid with a deleted
  fixture still reds its year.

## Report — write it as your FINAL action
Write `design/agent-reports/2026-09-06-build-4868-T1-T5-implementation.md`: what you changed (file
by file, one line each), every pinned number you moved (old → new, cause), the measured
`instr_pages` and how you measured them, the label-walk numbers before/after (joins per year, grid
bucket contents), which kills were seen red and how, the exact nextest commands you ran with their
summary lines, and anything you could not do or had to decide. Then return ONLY a 3-line summary
(what landed, the suite result for the crates you touched, the report path).
