# Phase review — design r2 steps 2–3 and the step-1 fold (68b86b8e, bc6dce35, 98e279c9)

Reviewer: Claude Opus 5, independent, read-only. Date: 2026-09-05. HEAD: `98e279c9beeec7a0cf5c9c615b9e68e2a88a5c3f`.

Read: `crates/btctax-forms/build.rs`, `src/bundled.rs`, `src/line_set.rs`, `src/error.rs`, `src/pdf.rs`,
`src/packet.rs`, `src/lib.rs`, `src/map.rs` (`Form6251Map::for_year`, `Form8275Map` block),
`src/form8959.rs`, `src/form6251.rs`; `tests/supported_years_cross_product.rs`, `tests/map_rows.rs`,
`tests/sp4.rs`, `tests/full_return_forms.rs` (packet block), `tests/census.rs`, `tests/common/mod.rs`;
`crates/xtask/src/package_check.rs`, `src/cite_check.rs`, `src/authority_manifest.rs`;
`crates/btctax-oracle-harness/src/main.rs`; `design/FORM_AUTHORITY_TABLE_DESIGN.md` §3–§11;
`git show 6267b6b1:crates/btctax-forms/{map,pdf}.rs` (the pre-switch arms); the 37 committed
`forms/*/*.map.toml`.

Ran:
- `git show --stat` ×3; `git diff 6267b6b1..98e279c9 -- <the seven files>`; `git diff 0fd80b27..98e279c9 -- crates/xtask/…`
- a Python extraction of **every old `for_year` arm** and **every old `*_pdf` arm** out of
  `6267b6b1`, to build the old year-set table mechanically rather than by eye
- a Python key-tree + leaf-type comparison of each of the ten `Unwired` TY2025 maps against its
  TY2024 counterpart (`.venv/bin/python`, `tomllib`)
- a Python replay of `fill_full_return`'s push order against `packet::attachment_sequence` for
  each bundled year, to decide where the new sort is a no-op
- grep sweeps for the 54 deleted const names, for `fill_full_return` call sites/years, for
  `LineSet::` uses outside `src/`, and for `include_bytes!`/`include_str!` survivors

**No cargo command was run, and this is not an omission I could avoid.** The working tree carries an
in-flight uncommitted step-4 change (`build.rs` +21 lines requiring `forms/<year>/YEAR.toml`, plus
untracked `YEAR.toml` ×3, `src/year_record.rs`, `crates/btctax-cli/src/year_readiness.rs`) that
panics `btctax-forms`'s build script — the crate does not compile in the tree as it stands, and
`target/` is 488 GB while `/tmp` has 29 GB free, so no isolated copy was viable either. Of the files
this review reasons about, only `src/lib.rs` differs from HEAD, by one line (`pub mod year_record;`);
`bundled.rs`, `line_set.rs`, `map.rs`, `pdf.rs`, `packet.rs`, `error.rs` and every test read are
byte-identical to `98e279c9`. Every claim below is therefore either a static derivation over HEAD's
source or a filesystem/TOML measurement, and each is labelled as such.

Not re-derived (settled by the brief): the design itself; step 1's 37 row values; the owner rulings;
that `line_set` is one revision per file today; that steps 4–5 are later.

## Verdict: 0 Critical / 2 Important / 4 Minor

## Behaviour-preservation audit

**Method.** Extract each old `for_year` / `*_pdf` accepted-year set mechanically from `6267b6b1`;
derive the new set from the glob on disk (`forms/<year>/`) crossed with `line_set::schema`. Compare
cell by cell. 17 `for_year` and 17 `*_pdf` today (`grep -c`), matching the commit message.

**`Map::for_year` — the Ok/Err split is preserved exactly, all 17 structs.**

| struct | old Ok years | new Ok years |
|---|---|---|
| Form1040Map, Form8949Map, Form8283Map, ScheduleDMap, ScheduleSeMap | 2017, 2024, 2025 | 2017, 2024, 2025 |
| Form6251Map, Form8959Map, Form8960Map, Form8995Map, Form8995AMap, Schedule1Map, Schedule2Map, Schedule3Map, ScheduleAMap, ScheduleBMap, ScheduleCMap | 2024 | 2024 |
| Form8275Map | 2017, 2024, 2025 (year list) | 2017, 2024, 2025 (periodic alias) |

The only delta is the refusal **class** on nine cells — TY2025 × {f6251, f8959, f8960, f8995,
f1040s2, f1040s3, f1040sa, f1040sb, f1040sc} — which went `UnsupportedYear` → `UnwiredLineSet`.
That is the intended step-3 change (note design §5's own text says `Unwired` "returns
`UnsupportedYear(year)`"; the implementation's more precise variant is a deliberate improvement, and
the three test expectations were updated with it). `f1040s1a/2025` has no struct at all and is
recorded in `STEMS_WITH_NO_MAP_TYPE`.

**`pdf::*_pdf` — nine getters widened, by design.** `f6251_pdf`, `f8959_pdf`, `f8960_pdf`,
`f8995_pdf`, `schedule_2_pdf`, `schedule_3_pdf`, `schedule_a_pdf`, `schedule_b_pdf`,
`schedule_c_pdf` went `{2024}` → `{2024, 2025}`, because those files exist under `forms/2025/`.
Every other getter is unchanged (`f8995a_pdf` and `schedule_1_pdf` stay `{2024}` — TY2025 bundles
neither `f8995a` nor `f1040s1`). This is design §9 exactly (`*_pdf(year) × 18` → `template(stem,
year)`) and it is **not reachable as a wrong number**: every filler loads `pdf::X_pdf(map.year)`
where `map` came from `Map::for_year(year)`, and `map_rows.rs` asserts `row.year == <directory
year>` for all 37 rows, so `map.year` cannot disagree with the map's own package. `pdf` is a private
module; only `f6251_pdf`/`f8995a_pdf` escape, via `testonly`, called with literal 2024.

**The periodic alias.** `f8283` has its own file in all three bundled years, so it never aliases;
`periodic_template(F8283, 2026)` and a hypothetical `2030` both return `None` (`BUNDLED_YEARS`
guard, `bundled.rs:156`) → `UnsupportedYear`. Correct. `f8275` exists only under `forms/2024/`, so
2017 and 2025 alias to it — identical to the old `2017 | 2024 | 2025 => Ok(F8275_PDF_2024)`, and
`KNOWN_ALIASES` still records exactly `{(2017, f8275), (2025, f8275)}`. A year outside
`BUNDLED_YEARS` refuses on every path (`template`/`map_text` `None`; `periodic_template` `None`),
and `sp4::for_year_answers_exactly_what_the_bundled_asset_registry_answers` probes 2010..=2035 and
sees both verdicts. **The "newest bundled revision" rule is where the seam is** — see Q1.

**Deleted-and-still-needed:** nothing in live code. The 54 consts and `testonly_old_consts` survive
only in `design/agent-reports/*` and `reviews/*` archives (correctly untouched) and in one **live**
spec — see Q4. The oracle harness's `map_for` was moved onto `bundled::map_text` verbatim, form for
form (14 arms, checked one by one against the old const names).

## Kill audit

| checker | can it fail? | how I know |
|---|---|---|
| `bundled::the_generated_bindings_are_the_files_on_disk_both_ways` | **yes** | `on_disk()` is an independent `std::fs::read_dir` walk of `forms/<year>/*.map.toml`; it is compared to `BUNDLED`, and each pair's bytes to `std::fs::read`. It does **not** derive "disk" from `BUNDLED`. `assert!(disk.len() >= 37)` guards a broken walk; two explicit `None` probes guard stale bindings. |
| `bundled::bundled_years_are_the_year_directories` | **yes** | Two derivations plus a `&[2017, 2024, 2025]` literal pin. This literal — not `BUNDLED_BUT_NOT_SUPPORTED` — is now what makes `forms/2026/` loud (Q5). |
| `bundled::every_stem_round_trips_through_its_file_stem` | yes | Round-trip + a duplicate-stem check; the `match` covers the other direction at compile time. |
| `line_set::every_variant_round_trips_through_its_string` | yes | Round-trip, `parse("f6251/1999") == None`, `ALL.len() == 37`. |
| `line_set::the_unwired_set_is_exactly_the_ten_ty2025_maps` | yes | Shrink-only pin on the exact ten strings. Together with the 27 dispatched cells in `supported_years_cross_product`, all 37 variants are held to a real row: a swapped variant reds in one place or the other. |
| the `line_set` ⇔ row join | **only transitively** | No test calls `LineSet::parse` on the committed rows. `map_rows.rs` asserts `row.line_set == "<stem>/<year>"` for all 37, and `LineSet::ALL.len() == 37`; the enum-to-rows equality is then closed by dispatch (27) + the unwired pin (10). `line_set.rs`'s module doc citing `tests/map_rows.rs` for "an unknown string is a parse refusal" is imprecise — map_rows tests a *missing* `line_set` and the string's *shape*. |
| `build.rs` non-year directory | **yes, observed** | Commit message: `forms/draft/` planted, rc=101. |
| `build.rs` `.pdf` with no `.map.toml` | **yes, observed** | Commit message: `f9999.pdf` planted, rc=101. |
| `build.rs` `.map.toml` with no `.pdf` | **never observed** | Same `if !(*has_pdf && *has_map)` branch, different message arm. The *condition* is covered by `supported_years_cross_product::every_committed_map_has_its_bundled_pdf`; the *build refusal* is not. Q6. |
| `package_check::every_bundled_form_file_and_the_build_script_are_in_the_published_tarball` | yes | Runs `cargo package --list` for real; `disk.len() >= 74` guards the walk; `listed.contains("build.rs")` is a separate assertion. |
| `package_check::a_file_the_tarball_does_not_list_is_caught` | yes | Plants `forms/2026/f9999.pdf` into the disk set and requires it to be the sole difference. |
| `packet::sequence_key_orders_letter_suffixes_after_their_number_and_1a_after_01` | **yes** | Twelve real sequence numbers including every case a string sort gets wrong (`1A`, `12A`, `55A`, `155`, `36`). This is the comparator's real kill. |
| `packet::a_shuffled_packet_sorts_into_stapling_order_for_every_bundled_year` | **partly** | Its ordering assertion is a tautology — it sorts by `sequence_key` and then checks the result is non-decreasing *in `sequence_key`*, which holds for any definition of that function. Its real content is `TapCheck`: every packet literal is re-asserted against the map row. It exercises the **function**, not the `fill_full_return` call site. |
| `packet::ty2025_form_8283_staples_after_6251_and_before_8995` | yes | Real, and the only test that pins the 36-vs-155 renumber's effect — but again on the function. |
| `map_rows` kill 4 (re-keyed) | **yes, observed both ways** | Plant on `2024/f8283` (a manifest-excused row *with* an extract) → `SequenceMismatch{155}`; plant on `2017/f8949` (no extract) → `SequenceUnverifiable`, pinned to exactly five TY2017 rows, shrink-only. The P1 fold is correct and the excuse is now keyed to its own mechanism. |
| `map_rows::a_mistyped_versioning_is_refused` | yes | `"annual"`→`"anual"` and `periodic =`→`periodc =`, both required `Err`. |
| `cite_check::a_row_pointing_at_no_template_is_caught_in_both_directions` | yes | P4's `two_way_diff` is now the *same function* the green test runs, planted in each direction. |
| `authority_manifest::is_authority_agrees_with_the_string_filter_over_every_entry` | yes | ≥100 entries, both predicates compared per entry. |
| `supported_years_cross_product`'s `wired` obligation | **no — and design §5 says so** | It is now `template(stem, y).is_some() && map_text(stem, y).is_some()` over a stem set derived from the same glob build.rs walks. Design §5: "tautological under this layer — that is what a witness is for." Honestly labelled in the doc comment. |
| `supported_years_cross_product::supported_years_and_bundled_year_directories_agree` | **almost never** | Q5. |

## Findings

### Q1 — IMPORTANT — the Form 8275 hash licence cannot fail as called, and refuses the one case it should allow

**Where:** `crates/btctax-forms/src/map.rs:1385-1396` (`Form8275Map::for_year`), `:1334-1373`
(`alias_is_licensed_by` and its doc), `crates/btctax-forms/src/bundled.rs:150-169`
(`periodic_template`).

**What is wrong:** two halves of one mistake — the licence is applied unconditionally, including
where no substitution happened.

*(a) It is tautological today.* `periodic_template(F8275, y)` returns either the year's own bytes
(only 2024 has an `f8275.pdf`) or `template(F8275, newest)` where `newest = max{y : (F8275, y) ∈
BUNDLED} = 2024`. Both branches are therefore byte-identical to `template(F8275, 2024)`, which is
exactly what `alias_is_licensed_by` compares against. **Deleting the call changes nothing on any
input.** The design names this hash as the replacement for the year list (§4: "a periodic form
aliases a prior revision BY HASH, never by year list"), and `map.rs:1345-1357` still presents it as
the live guard that makes `| 2026` unavailable — but the widening it is supposed to block now needs
*zero* tokens: drop `forms/2026/` with no `f8275` in it and `Form8275Map::for_year(2026)` returns
`Ok` with the licence approving, because it is comparing the 2024 file to itself.

*(b) It refuses a correctly-transcribed own-year file.* When a year does bundle its own newer Form
8275 — design §10 step 6, `forms/2026/f8275.pdf` at Rev. 12-2026 with its own transcribed
`f8275.map.toml` — `periodic_template` returns `(own_bytes, 2026)`, `for_year` compares `own_bytes`
to the hard-coded `template(F8275, 2024)`, they differ, and it refuses with a message instructing
the reader to *"transcribe `forms/2026/f8275.map.toml` against the revision this build actually
ships"* — a file that already exists and that `for_year` would have loaded on the next line
(`map_text(F8275, from_year)`). The symmetric case is worse: once 2026 is the newest bundled
revision, TY2017 and TY2025 alias to Rev. 12-2026 and would correctly use `map_text(F8275, 2026)` —
and the 2024-keyed licence refuses all of them, silently un-shipping Form 8275 for two years that
ship it today. `sp4::for_year_answers_exactly_what_the_bundled_asset_registry_answers` does red when
this triggers (it requires `for_year(y).is_ok() == bundled_pdf(y).is_ok()`), but with the wrong
diagnosis — *"the map has re-acquired a year list of its own"*. Fails closed, so no wrong number;
a wrongly-closed door at the design's own next step.

The invariant that actually matters is *"the map used was transcribed from the bytes served"*, and
step 3 already made it **structural**: `periodic_template` returns `(bytes, from_year)` with
`bytes == template(stem, from_year)` by construction, and `for_year` reads `map_text(stem,
from_year)`. There is nothing left for a runtime comparison to catch — which is why the one it does
is tautological.

**Minimal change:** in `Form8275Map::for_year`, drop the `alias_is_licensed_by(year, bundled)?` call
and state the invariant where it is now enforced (`periodic_template` pairs the bytes with the year
whose map is read), or keep it as `debug_assert_eq!(bundled, template(Stem::F8275, from_year))`,
which is an assertion of that invariant rather than a guard against a calendar. Keep
`alias_is_licensed_by` and its sp4 plants as the documented statement of the rule if wanted, but
stop calling it from the live path, and correct the two doc comments that still describe a year list
as the thing being defended against (`map.rs:1345-1357`,
`supported_years_cross_product.rs` `KNOWN_ALIASES`).

### Q2 — IMPORTANT — nothing reds if `sort_by_attachment_sequence` is deleted from `fill_full_return`

**Where:** `crates/btctax-forms/src/packet.rs:292` (the call), `:310-313` (the function),
`:317-397` (`mod sequence_order_tests`).

**What is wrong:** the fold's headline guarantee — the emitted packet *is* the stapling order — has
no test that fails when the guarantee is removed.

Measured, by replaying `fill_full_return`'s push order against `packet::attachment_sequence` for
each bundled year: the push order is **already** in stapling order for TY2017 and TY2024, and only
TY2025 differs (Form 8283 renumbered 155 → 36, so it must move from last to between 6251 and 8995).
All three new tests build a `Vec<NamedForm>` by hand and call `sort_by_attachment_sequence` /
`sequence_key` directly; **no test anywhere in the workspace calls `fill_full_return` with a year
other than 2024** (`full_return_forms.rs`, `census.rs`, `field_census_slice.rs`, `attestation.rs`,
`common/mod.rs`, `golden_packet.rs` via `full_return()` — all literal 2024; the CLI reaches it only
through `admin.rs` at runtime). So the sort line can be deleted and the suite stays green.

The mitigation is real and worth stating: TY2025 is the only year where the sort bites, and
`fill_full_return(_, 2025)` **cannot succeed at all today** — `crates/btctax-forms/src/lib.rs:195`
calls `Form8959Map::for_year(year)?` unconditionally, *before* `must_file()` is consulted inside the
filler, so every TY2025 packet aborts with `UnwiredLineSet{stem: "F8959"}` regardless of content.
That is pre-existing (the old code aborted with `UnsupportedYear`) and out of this range, but it is
why the end-to-end assertion is not writable yet, and it is worth knowing at step 5.

**Minimal change:** make the omission unexpressible rather than untested — build the result through
a constructor that sorts (`FiledPacket::stapled(forms, statements)`, sorting inside), so removing
the sort is a compile error, per `CLAUDE.md`'s "prefer designs in which an omission does not
compile". If that is too much for a fold, file a step-5-owned follow-up to add
`fill_full_return(pr, 2025)` → f8283 between f6251 and f8995 the moment TY2025 dispatches, and say
in `packet.rs:292`'s comment that the call site is currently unwitnessed.

### Q3 — MINOR (owning phase: step 5) — `Schema::Unwired`'s stated reason is false for eight of the ten

**Where:** `crates/btctax-forms/src/line_set.rs:9-11` (module doc), `:263-265` (`Schema::Unwired`
doc), `:66-89` (ten variant docs), `crates/btctax-forms/src/error.rs:19-27`
(`UnwiredLineSet`'s message).

**What is wrong:** all four sites say the ten TY2025 maps are `Unwired` because **no struct parses
them**. Measured with `tomllib` — full key tree plus leaf type, arrays included, `[census]` entries
excluded and their `CensusDecision` shapes separately confirmed to introduce no 2025-only variant —
eight of the ten are **structurally identical** to the TY2024 map their struct already parses:

| map | vs its 2024 counterpart |
|---|---|
| `f1040s2`, `f1040s3`, `f1040sa`, `f1040sb`, `f1040sc`, `f8959`, `f8960`, `f8995` | identical keys, identical leaf types — `<Struct>::parse` **would succeed** |
| `f6251/2025` | genuinely differs: `line1` → `line1a` + `line1b`, so `deny_unknown_fields` refuses. This is the one the design singles out ("this is where the TY2025 6251 map first parses") |
| `f1040s1a/2025` | no `Schedule1AMap` exists at all |

Refusing is still right and still fail-closed, and the behaviour is unchanged from before the switch
— so nothing computes wrongly. The cost is at step 5: if the recorded criterion is "does it parse?",
eight doors open in one commit on a check that was always going to pass, without anyone asking
whether the TY2025 *line semantics* match — which is precisely the wrong-number path
`Form6251Map::for_year`'s own doc comment (`map.rs:262-270`) exists to describe.

**Minimal change:** reword the four sites to what is true — the revision has not been **verified**
against a struct (the label/extract join is the criterion, not `parse()`) — and record the
measurement above beside the `Unwired` arm so step 5 starts from it.

### Q4 — MINOR — stale post-switch prose, including one live spec

**Where and what:**

- `crates/btctax-forms/src/lib.rs:83-85` — TY2025's forms are "never `include_bytes!`'d,
  `for_year(2025)` refusing". The first half is now false; `KNOWN_GAPS` was edited in the same
  commit to drop `wired` for exactly these ten cells.
- `crates/btctax-forms/src/lib.rs:80-81` and `tests/supported_years_cross_product.rs:14` — TY2017
  has "five `include_bytes!`, `for_year(2017)` arms in all five map types". Both are gone.
- `tests/supported_years_cross_product.rs`, `KNOWN_ALIASES` doc — "the alias guard is a **year
  list** — `| 2026` is a one-token edit". The year list was deleted at step 3; see Q1 for what
  replaced it and what that costs.
- `crates/btctax-forms/tests/sp4.rs:411` — a mechanical rename left a full
  `btctax_forms::bundled::template(btctax_forms::bundled::Stem::F8275, 2024).unwrap()` expression
  inside prose ("Written against `…` it could not"), which no longer reads as a sentence.
- **`design/ty2025/SPEC_schedule_a_ty2025.md:352-353`** — a *live* spec (last touched by the
  in-flight TY2025 cycle, not an archived report) whose "files to change" list instructs the
  implementer to add a `SCHEDULE_A_PDF_2025` const and a `2025 => Ok(..)` arm at `pdf.rs:185`, and a
  `SCHEDULE_A_MAP_2025` const at `map.rs:73`. All four referents were deleted at step 3. This is the
  spec step 5 executes.

This is the class `3d01b5e3` folded six of, and the SPEC entry is the one that can actually mislead
work. **Minimal change:** correct the five sites; for the spec, replace items 3–4 with "delete the
`Unwired` arm for `f1040sa/2025` in `line_set::schema`."

### Q5 — MINOR — the "a year must not arrive quietly" ratchet no longer fires the way its doc claims

**Where:** `crates/btctax-forms/tests/supported_years_cross_product.rs`,
`BUNDLED_BUT_NOT_SUPPORTED` and `year_verdict` / `supported_years_and_bundled_year_directories_agree`.

**What is wrong:** `SUPPORTED_YEARS` is now `bundled::BUNDLED_YEARS`, derived by `build.rs` from the
same `forms/<year>/` glob the test itself walks. So `year_verdict`'s first branch
(`claimed_without_assets`) is empty by construction and its third (`stale`) cannot fire while the
const is empty; only `unclaimed` survives, and only for a year directory that is *completely empty*
(build.rs enters a year in `found` on its first file). The const's doc still says "committing
`forms/2026/` makes this file red until the year is either listed as supported or recorded here" —
it does not; 2026 would simply become supported.

The ratchet does still hold, through three other places, and this should be written down rather than
left to be rediscovered: `bundled.rs::bundled_years_are_the_year_directories`'s `&[2017, 2024,
2025]` literal, `count_verdict`'s `BUNDLED_FORMS_PER_YEAR`, and `matrix_verdict`'s
"missing … and NOTHING records it" branch. Note this is **not** covered by design §5's blessing,
which is explicitly about the `wired`/`dispatch` obligations.

**Minimal change:** say in `BUNDLED_BUT_NOT_SUPPORTED`'s doc which test now makes a new year loud,
or key the ratchet to something not derived from the same glob.

### Q6 — MINOR — one build.rs refusal arm never observed red, and the cross-product's reader lost its liveness check

**Where:** `crates/btctax-forms/build.rs:92-99`; `tests/supported_years_cross_product.rs` (the
deleted `src_text`).

**What is wrong:** two small B1 residues.

- The unpaired-file panic has two message arms; only "a `.pdf` but no `.map.toml`" was planted
  (`f9999.pdf`, rc=101). The reverse — a `.map.toml` with no `.pdf` — shares the branch but its own
  arm was never witnessed, and the brief asked specifically. The *condition* is covered by
  `every_committed_map_has_its_bundled_pdf`, so nothing is unguarded; the *build-time* refusal is
  unwitnessed. Cheapest close: extend the existing plant to a second tempdir case.
- `src_text()`'s `assert!(blob.contains("include_bytes!"), "no includes found in src/")` went with
  the grep it guarded. It was the reader's own liveness check — the thing that stopped `wired` from
  reporting green because the reader had broken. Its replacement (`template`/`map_text` are `Some`)
  needs no such guard because it is tautological by design (§5), so nothing is lost in substance.
  Recorded because the brief asked, and because a tautological obligation with no liveness check is
  the shape worth naming out loud in a file whose whole point is an honest matrix.

## Step 4 may proceed?

NO — fold Q1 and Q2 first. Both are small and land in `map.rs` / `packet.rs`, which step 4 does not
touch, so they are independent of the `YEAR.toml` work and can go in parallel. Note step 4 is
**already in flight in the working tree** (`build.rs`, three `YEAR.toml`, `src/year_record.rs`,
`crates/btctax-cli/src/year_readiness.rs`), so these two folds will have to land on top of it, and
the crate does not currently build.
