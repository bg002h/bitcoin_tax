# Fold review r3 — of de1fa8a3

Reviewer: Claude Opus 5 (1M), independent, read-only. Date: 2026-09-05. HEAD: `de1fa8a3`.

**Read:** `design/agent-reports/2026-09-05-fable-plan-review.r2-fold-review-r2.md` (all 192 lines) and its
`…VERIFICATION.md`; `git show de1fa8a3` in full (message + all three file diffs);
`design/FORM_AUTHORITY_TABLE_DESIGN.md` §4 (`:62-103`), §5 (`:105-137`), §9 (`:196-211`), §10 (`:213-238`)
in full, plus §7 (`:168-186`) and §8 (`:188-194`) as the sections the fold's new text cross-refers to;
`design/TY2026_PORT_REPORT.md:304` and its siblings `:136`/`:419`; `design/ROADMAP_STATUS.md:177,184-187`;
and, for the four claims below that needed a machine check, `crates/btctax-forms/src/map.rs`
(`for_year` arms per struct, `fn ty2025` × 5), `crates/btctax-forms/src/lib.rs:70-86`,
`crates/btctax-forms/tests/supported_years_cross_product.rs:56-128,335-360`,
`crates/btctax-core/src/tax/printed.rs`, `crates/btctax-core/src/tax/schedule_1a.rs`,
`crates/btctax-core/src/tax/qbi.rs`, `crates/xtask/src/authority_manifest.rs:129,135,195`,
and the `forms/{2017,2024,2025}` directory listings.

**Not re-derived** (settled per the brief): everything in both ledgers; the glob + header + `build.rs`
shape; `line_set` = revision, many-to-one; the six manifest-join misses; the ten unwired TY2025 maps
(re-measured anyway as a by-product of checking the fold's new enumeration — they match exactly); the
owner rulings; that the fixtures do not move (G3); `emitted_form_years()` keying on `irs_basename`
(`cite_check.rs:853`, verified in r2); the `deny_unknown_fields` count of 55.

No file other than this one was written. No mutating command was run.

## Verdict: **0 Critical / 0 Important / 3 Minor**

The fold closes G1–G6 faithfully. Where the r2 reviewer supplied a "minimal change" text, the fold used
it — the two §4 header lines are byte-identical to the reviewer's block (modulo column alignment), the
ten-map enumeration is the reviewer's list in the reviewer's order, the G6 sentence and the two §4 kill
exceptions are the reviewer's wording verbatim, and G4's "15" is exact (measured: 2017 = {f1040, f8283,
f8949, schedule_d, schedule_se}, and those same five appear in 2024 and 2025 → 5 × 3 = 15, with
`fn ty2025` = 5 and TY2025 = 15 files, so 15 shared + 10 unwired + 12 own-struct 2024 = 37 and 5 + 12 =
17 top-level structs — the whole arithmetic closes). Two closures were widened beyond their minimal
change, both in the direction the review was pushing: G2's `Unwired` was propagated into §4's fifth kill
(which is what actually ends the §4-vs-§10 divergence G1 was about), and G3 carries the measured sizes.
No closure was argued away, and no new contradiction rises to blocking.

**§10 step 1 is executable as written now.** The G1 blocker — an excuse field named only in §10, in a
document whose parse gate is `deny_unknown_fields` — is gone, and every other artifact step 1 names
resolves: `MANIFEST.json` is at `design/forms/MANIFEST.json`, `Entry::is_authority()` and `is_draft()`
are at `authority_manifest.rs:135` and `:129`, and `emitted_form_years()` is where §10 says it is.

The three Minors are recorded because each is a one-line edit at a site this fold touched; none blocks
step 1, 2 or 3, and none should hold a gate.

## Per-finding trace

| G | closed where (file:line) | closed? | note |
|---|---|---|---|
| **G1** (`authority`/`extract_override` undefined; §4 kills carry no exceptions) | `FORM_AUTHORITY_TABLE_DESIGN.md:70-71` (block) and `:95-103` (kill list) | **yes** | reviewer's step-1 code block used verbatim; both kill exceptions use the reviewer's own words. §4 and §10 now state the manifest and sequence kills identically. The added ratchet clause ("the count may only shrink") is new but consistent with §9's shrink-only row → no contradiction |
| **G2** (relocated `line_set` kill had no expected-red set or excuse slot) | `:217` (step 1, the ten named), `:231-234` (step 3, the `Unwired` arm), `:100-101` (§4's fifth kill restated) | **yes** | the ten names and their order are the reviewer's; measured against disk they are exactly TY2025 minus the five `fn ty2025` structs. The `Unwired` state is already what `supported_years_cross_product.rs:117-128` records as the shrink-only `wired`/`dispatch` gap for those same ten, so §8's witness accommodates it → **M3** on the runtime value |
| **G3** (§9's "move" would clobber the booklet extract and create escaping `include_str!`s) | `:209` (§9 row, Decision replaced) | **yes** | retraction is explicit and correct; the src/-vs-tests/ distinction the argument rests on holds (`tables.rs` is library code, so it would break `cargo package --verify`, unlike the existing test-only escapes at `f6251_map.rs:16`). Residual gap → **M2** |
| **G4** ("~13" is 15) | `:84` and `:217` | **yes** | both sites; 15 verified by directory listing. Zero `~13` residue in `design/*.md` outside agent reports |
| **G5** (withdrawal widened past `schedule_1a.rs`) | `TY2026_PORT_REPORT.md:304` | **yes** | withdrawal narrowed to `schedule_1a.rs` (56 `lineNN` fields — measured, matches); `printed.rs` and `qbi.rs::Form8995Lines` (16 — measured, matches) returned to the bad column as OPEN. §7 `:183-186` already named only `schedule_1a.rs`, so no new §7 conflict. The bad cell's new citation is the wrong witness → **M1** |
| **G6** (4868 both "NOW" and "join P4") | `ROADMAP_STATUS.md:187` | **yes** | reviewer's replacement sentence used verbatim; `:177`'s NOW row agrees; no other "join P4" for 4868 remains |

## Findings

### M1 — MINOR — G5's new bad-cell citation names the two structs in `printed.rs` that are *not* line-named, and carries a stale count

**Where:** `design/TY2026_PORT_REPORT.md:304` (the cell the fold rewrote).

The cell reads *"**OPEN:** `printed.rs` (107; `Printed8949Row`, `Printed8949Totals` … carry NO year field
and serve 2017/2024/2025 alike) … — line-named structs shared across three revisions"*. Measured:

- `Printed8949Row` fields are `description`, `date_acquired`, `date_sold`, `proceeds_d`, `cost_e`,
  `gain_h`; `Printed8949Totals` is `proceeds_d`, `cost_e`, `gain_h`. **Zero `lineNN` fields in either** —
  they are the file's two semantic-named structs, i.e. the shape the good column is for. The "no year
  field" half of the claim is true (the file has none anywhere), but as witnesses for *line-named* they
  are the wrong two.
- The count: `^\s*pub line[0-9]` in `printed.rs` = **129**, across **10** structs (`Form1040Lines` 34,
  `ScheduleALines` 22, `ScheduleDLines` 18, `ScheduleSeLines` 12, `Form1040Income` 11, `Schedule1Lines`
  10, `ScheduleCLines` 7, `Schedule2Lines` 6, `Schedule3Lines` 5, `ScheduleBLines` 4), not "107 …
  across 7". The 107/7 figure is pre-existing at `:136` and `:419`; the fold carried it into the new
  parenthetical, where it now sits beside two counts (`56`, `16`) that I measured and that are exact.

G5 asked for "a real citation in the bad cell"; this one does not exhibit the property. Whoever later
decides `printed.rs`'s fate would open `Printed8949Row`, find semantic names, and close the OPEN
question the wrong way.

**Minimal change:** in `:304`, replace the two struct names with the three largest line-named ones —
*"(`Form1040Lines` 34, `ScheduleALines` 22, `ScheduleDLines` 18 `lineNN` fields; **129** across 10
structs, none carrying a `year`)"* — and either correct `:136`/`:419` to 129/10 in the same pass or drop
the bare number from `:304` so the document does not hold two counts of one thing.

### M2 — MINOR — §9's regeneration recipe covers only one of the two fixtures it retires

**Where:** `design/FORM_AUTHORITY_TABLE_DESIGN.md:209`.

The corrected Decision says *"The fixtures are regenerated at test time from the booklet extract using
the header's `instr_pages`, then deleted"* — but only `schedule_1a_2025_instructions.txt` is a booklet
slice. `schedule_1a_2025_form.txt` (11,443 B) is a second extraction of `f1040s1a--2025.txt` (11,153 B):
no booklet, no `instr_pages`, and 290 bytes of difference that are precisely why `extract_stem` exists.
So the recipe as written retires one fixture and leaves the other's disposal unstated — and "delete it
and read the convention file instead" is only safe if every `FORMS` quotation still resolves against the
11,153 B text, which nothing in §9 asserts. The r2 reviewer's own minimal change has the same shape, so
this is carried rather than introduced, and it is scheduled nowhere in §10 steps 1–6 (the
`extract_override` field holds it open) — hence Minor.

**Minimal change:** *"…the instructions fixture is regenerated from the booklet extract using
`instr_pages`; the form fixture is replaced by `f1040s1a--2025.txt` **once a test asserts every `FORMS`
quotation still resolves against it** (the two extractions differ by 290 B). Both are then deleted."*

### M3 — MINOR — three sites still state the fifth kill in its pre-`Unwired` form, and `Unwired`'s return value is unstated

**Where:** `design/FORM_AUTHORITY_TABLE_DESIGN.md:225-226` (step 1's forward reference: *"`line_set`
naming no schema → compile error"*), `:118-119` (§5: *"parse into the struct the header's `line_set`
names"*), `:192-193` (§8: *"`for_year(y)` Ok ⇔ file on disk (tautological under §5)"*) versus `:100-101`
and `:231-234`.

The fold correctly propagated `Unwired` into §4's kill, which is what closes the §4/§10 divergence. Three
older sentences describe the same kill without it: after step 3, ten `line_set`s name no struct, so §5's
"parse into the struct the header's `line_set` names" and §8's "⇔" are both false for exactly those ten.
This is not a builder trip — step 3's own text is explicit and correct, and the real witness
(`supported_years_cross_product.rs:117-128`) already records `["wired","dispatch"]` as a shrink-only gap
for those ten, so nothing reds — but it is the same class of §-to-§ drift G1 was raised about.

Related and worth one clause: the design never says what the `Unwired` arm *returns*. Today
`Form6251Map::for_year(2025)` is `Err(UnsupportedYear(2025))` (`map.rs:222-227`), and under §5
`map_text(Stem::F6251, 2025)` will succeed once `build.rs` globs the file, so the `.ok_or(UnsupportedYear)`
that produces today's refusal stops firing and the arm has to. Every plausible implementation fails
closed (there is no struct for `Unwired` to produce), so this is a wording gap, not a defect — but the
refusal string changes if the builder falls through to a toml parse error instead.

**Minimal change:** step 1 `:225-226` → *"`line_set` naming neither a schema nor `Unwired`"*; §5 `:119` →
*"…into the struct the header's `line_set` names (or `Unwired`, which returns
`UnsupportedYear(year)` — §10 step 3)"*; §8 `:192` → *"`for_year(y)` Ok ⇔ file on disk **and** its
`line_set` wired".*

## §10 step 1 — executable as written NOW? **YES**

I walked it as a builder. Every field step 1 tells me to write is defined in §4 with its optionality
(`authority` and `extract_override` included, which is the G1 fix); the four kills it plants each resolve
to something that exists — `design/forms/MANIFEST.json`, `Entry::is_authority()`/`is_draft()`
(`authority_manifest.rs:135`/`:129`), the bundled PDFs, `design/forms/extract/`, and
`emitted_form_years()` keyed on `irs_basename`; the manifest kill's expected red is a named set of six
with `authority` as the stated excuse; the sequence kill tolerates the 1040; and the fifth kill is
explicitly deferred to step 3 with a named arm. The one thing I would have called a trip — "the required
fields AND the two optional ones above" (`:96`), when `instr_pages` and `attachment_sequence` are
conditional too — is disambiguated by their own comments three lines up and by step 1's own
`attachment_sequence` parenthetical, so it self-corrects at the first `cargo test` rather than leaving a
gate that cannot be greened. Nothing in step 1 now points at an undefined thing.
