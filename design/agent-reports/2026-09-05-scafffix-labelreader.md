# scaffolding fix — `label_reader.rs`

**Date:** 2026-09-05 · **Agent:** scafffix/label_reader · **File owned and touched:**
`crates/xtask/src/label_reader.rs` (only). No other file was edited; no git command was run.

Everything numeric below was measured by running something, not read off a doc comment.

---

## 1. What was wrong, measured

### D1 — `label-proof` could not open any archived draft (`:456`)

`proof()` parsed its own year with `stem.rsplit("--").next().unwrap_or("2025")` and **no**
`.trim_end_matches("-DRAFT")`, unlike its sibling `form_geometry::extract`. So `f6251--2026-DRAFT`
resolved to the directory `design/forms/2026-DRAFT/`, which has never existed.

Measured before the fix, over every committed `*-DRAFT` geometry fixture:

```
BEFORE: OK=0 FAILED=16
```

15 of the 16 failed with
`design/forms/2026-DRAFT/<stem>.pdf not present (gitignored; re-fetch from its .pdf.txt note)`
— i.e. the message sent the reader to re-download a file **that is on disk**. The 16th failed for an
unrelated reader reason (see §4, F1).

This is the human-in-the-loop instrument whose own banner reads *"a defect the machines could not
see"* — the one instrument no machine covers for, and it was dead for the entire target year.

### D2 — the dead `unwrap_or("2025")` (`:456`)

`rsplit` always yields at least one item (`"f1040".rsplit("--").next()` is `Some("f1040")`), so the
`"2025"` could never be reached. It read as a deliberate fallback-to-2025 policy and was not one, in
one of the two sites most likely to be copied when a new stem-consuming command is added.

### D3 — coverage of the line→label join was ONE GLOBAL COUNT (`:1061`, `:1071-1079`)

`assert!(checked >= 151)` summed every year at once, and the maps the check could not reach were
`eprintln!`'d and never asserted. A TY2026 whose geometry was never generated contributes **0
joins**, moves the global count by nothing, and the gate stays green on 2024+2025 coverage.

### D4 — Schedule D was unreachable, and nothing said so (`every_map()`, `:1013`)

`every_map()` built the geometry stem as `format!("{form}--{y}")` using the **map's** spelling while
the fixtures are committed under the **IRS's**: maps `schedule_d` / `schedule_se`, fixtures
`f1040sd--2024` / `f1040sse--2025`. Measured by planting the old join back (§3, plant C):

| year | joins with the name join | joins with the hash join | delta |
|---|---|---|---|
| 2024 | 84 | **99** | +15 |
| 2025 | 67 | **82** | +15 |

**30 line→label joins whose fixtures were on disk the whole time.** Schedule D is where btctax's
capital gain lands, and no committed test joined any of its lines to its printed label in any year.

**All 30 newly-reached joins PASS.** `wrong` is empty. Nothing was forced green.

---

## 2. What changed

All in `crates/xtask/src/label_reader.rs`.

1. **New `pub(crate) fn stem_year(stem) -> Result<&str, String>`** — strips `-DRAFT` for the
   directory (keeping it in the filename, which is what `authority_manifest::Entry::is_draft`
   reads), and **refuses** a stem that is not `<form>--<year>` instead of falling back. `proof()`
   calls it as its first statement, so a malformed stem gets the useful message rather than
   "geometry fixture missing".

   ```
   AFTER: OK=14 FAILED=1     (of the 15 draft fixtures now on disk; see §5)
   $ xtask label-proof f6251
   xtask label-proof: `f6251` is not a form stem: expected `<form>--<year>`, e.g. `f6251--2025` or `f6251--2026-DRAFT`
   ```

2. **`every_map()` now joins a map to its geometry by the sha256 of the bundled PDF**, never by
   name. This is not a workaround for the naming mismatch — it is a stronger join: a name match
   pairs a map with *a* fixture for that year, the hash pairs it with the fixture observed from
   **byte-identical bytes**, which is the claim each map's own header makes (*"a byte-for-byte copy
   of design/forms/2025/f8959--2025.pdf"*) and which nothing checked. Measured: of the 37 bundled
   map PDFs, **31 match exactly one fixture, 0 match two**, 6 match none (TY2017's five and
   `2024/f8283.pdf`) and are reported by name.

   A map with no fixture is carried as `Err(reason)` and gated — never dropped.

3. **Per-year coverage.** The global `checked >= 151` is gone. The test now builds a `YearReach`
   census (`year`, `maps`, `unwitnessed: Vec<String>`, `joins`) and hands it to a pure
   `audit_year_reach(observed, floors)` that runs in **both directions**:

   - a year with maps on disk and **no recorded floor** fails ("a new tax year must not arrive
     silently") — this is the TY2026 case;
   - a **recorded floor whose year has vanished** from disk fails (the direction a one-way list is
     blind to, the defect shape CLAUDE.md records as having shipped here);
   - joins below the year's floor fail;
   - unreachable maps above the year's allowance fail, **by name**;
   - a floor that tolerates zero joins or an unreachable map and records **no reason** fails.

   The **years are not listed** — they are read off the maps directory. Only the ratchet numbers are
   recorded, each with the reason for any permissive value.

   Measured census, printed on every run (names now printed always, not only on failure):

   ```
   2017: 5 map(s), 0 join(s) checked, 5 unreachable
         NOT WITNESSED f1040 / f8283 / f8949 / schedule_d / schedule_se — no geometry fixture was
                       observed from crates/btctax-forms/forms/2017/<form>.pdf
   2024: 17 map(s), 99 join(s) checked, 1 unreachable
         NOT WITNESSED f8283 — no geometry fixture was observed from .../2024/f8283.pdf
   2025: 15 map(s), 82 join(s) checked, 0 unreachable
   4 binding(s) landed on a box the reader could not label (`?`)
   ```

   Floors recorded: 2017 `min_joins 0 / max_unwitnessed 5` (with the reason: no TY2017 form is
   archived under `design/forms/`, so no fixture can exist, and nothing emits a TY2017 return);
   2024 `99 / 1` (reason: `design/forms/` holds `i8283--2024`, the instructions, but not the form);
   2025 `82 / 0`.

4. **Two new tests** (§3) plus one rewritten. `cargo clippy -p xtask --all-targets` → 0 warnings;
   `rustfmt --check` clean; `cargo nextest run -p xtask -E 'test(label_reader)'` → **12/12 pass**.

---

## 3. Which test reds for which planted defect

Every plant below was applied to the working tree, run, observed red, and restored from a
scratchpad copy (no `git checkout`).

| # | planted defect | test that went RED | failure text |
|---|---|---|---|
| A | delete `.trim_end_matches("-DRAFT")` from `stem_year` | `tests::every_committed_geometry_stem_resolves_to_a_year_directory_that_exists` | ``resolved to year directory `2026-DRAFT` — `-DRAFT` belongs to the FILENAME`` |
| B | restore the shipped shape `rsplit("--").next().unwrap_or("2025")` (no refusal) | same test | ``a stem with no `--` must REFUSE`` |
| C | restore the map-name stem join in `every_map()` | `map_label_join_tests::every_mapped_line_lands_on_its_own_printed_label` **and** `map_label_join_tests::schedule_d_joins_through_the_pdf_hash_because_its_map_name_is_not_its_irs_stem` | `2025: 2 map(s) this check cannot reach, allowance is 0: schedule_d …, schedule_se …` / `2024/schedule_d must join to a fixture: no geometry fixture schedule_d--2024` |
| D | make `audit_year_reach` tolerate a year with no recorded floor (the old global-count behaviour) | `map_label_join_tests::the_per_year_audit_reds_on_a_new_year_a_lost_year_lost_coverage_and_a_lost_fixture` | `a year with maps and no recorded floor must FAIL` |
| E | drop the reverse direction (recorded year absent from disk) | same test | `a vanished year must FAIL` |

The per-year kill test plants its four failures **synthetically**, against the pure
`audit_year_reach`, precisely so it does not have to mutate a tree seven agents share — and so the
kill runs on every future run rather than once, by hand, today. It also covers coverage falling
below a floor, a fixture going missing, and a permissive floor with no recorded reason.

`the_join_check_reds_on_a_wrong_year_map_that_an_existence_check_would_pass` (the pre-existing B1
kill, TY2024's 6251 map against the TY2025 form) still passes unchanged.

---

## 4. Found, NOT fixed — each with its measurement

**F1 — `witness_text` cannot read `f1040s1--2026-DRAFT` (the only remaining `label-proof` failure).**

```
$ xtask label-proof f1040s1--2026-DRAFT
bare sub-letter `a` at page 1 y=120.859 has no numeric parent — the state machine is out of step
and the label set cannot be trusted
```

This is a genuine reader limitation on the TY2026 draft Schedule 1, not a path bug: it refuses
loudly, which is correct behaviour, but it means **Schedule 1 has no label proof for the target
year**. Fixing it is a change to the label state machine, not to a stem. Owning phase: whoever
ports Schedule 1 to TY2026.

**F2 — `line_bindings()` mis-parses any binding that carries a trailing comment, silently. This is
the largest single blind spot left in this file, and it is bigger than the one I was sent to fix.**

The parser takes the whole right-hand side, strips one leading and one trailing `"`, and keeps the
result if it contains `[` and `.`. For

```toml
line1  = "topmostSubform[0].Page1[0].f1_3[0]"   # "Medicare wages and tips from Form W-2, box 5"
```

it yields the FQN ``topmostSubform[0].Page1[0].f1_3[0]"   # "Medicare wages and tips from Form W-2, box 5``,
which is in no geometry, so `join.get(&fqn)` is `None` and the binding is **skipped by the
`else { continue }`** — no count, no name, no complaint. A comment not ending in `"` drops the
binding outright, equally silently.

Measured (a Python model of the parser reproduced the current in-test figure of 185 resolved
bindings **exactly**, so the counterfactual is calibrated): taking the first quoted span instead
would raise resolved bindings from **185 to 432** — the parser currently hides **247 bindings**,
across 17 of the 32 maps that have any. Worst cases: `2025/f1040s1a` 1 → 46, `2024/f1040` 1 → 35,
`2024/f1040sa` 0 → 19, `2025/f1040sa` 0 → 19, `2024/f8959` 0 → 17.

I did **not** fix it. It is a 2.3× expansion of what this gate checks; any wrong binding it exposes
would have to be fixed in `crates/btctax-forms/forms/**` — files I do not own — and it deserves its
own task with its own red/green record rather than being smuggled into a scaffolding fix. **Nothing
in my change depends on it**: the per-year floors are ratchets on what is actually checked, so they
only go up when it lands.

**F3 — `form_geometry.rs:194` still carries the dead `unwrap_or("2025")`.** I do not own that file.
The fix is one line: call `crate::label_reader::stem_year(stem)?`, which is `pub(crate)` for exactly
this. Its `-DRAFT` strip is already correct; only the phantom fallback needs removing.

**F4 — the six unreachable maps could probably be reached, from PDFs that are already committed.**
`2017/{f1040,f8283,f8949,schedule_d,schedule_se}.pdf` and `2024/f8283.pdf` all exist in
`crates/btctax-forms/forms/`, committed. `xtask extract-geometry` only looks in
`design/forms/<year>/<stem>.pdf`, which is gitignored and lacks them. Teaching `extract-geometry`
to accept a path (or to fall back to the bundled copy) would close all six and let the 2017 and
2024 allowances drop to 0. Not attempted — it is `form_geometry.rs`.

**F5 — `line_bindings()` is section-blind.** It reads `line2 = …` under `[part2_col_a]` as though it
were a top-level binding, and the result is a `BTreeMap` keyed on the line label, so two sections
binding the same line number would silently overwrite each other and halve the check. Measured
across all 32 maps: **no map currently has a colliding top-level `line` key**, so nothing is lost
today. It is a trap for the first multi-column map (Form 8995-A already has the shape; only
`_col_a` exists so far).

**F6 — observed, not caused by me:** during this session another agent removed
`design/forms/geometry/f1040--2026-DRAFT.json` and the `design/forms/2026/f1040--2026-DRAFT.pdf.txt`
provenance note (fixtures went 48 → 47, drafts 16 → 15). That currently reds
`xtask::authority_manifest::tests::every_manifest_entry_resolves_and_hashes_true`
(*"gitignored, and its `.txt` provenance note is missing"*). Not my file and not my change — flagged
so the controller does not attribute it here. It is also why the draft test asserts **non-vacuity**
(`drafts >= 1`, i.e. the `-DRAFT` strip is exercised by a real fixture) rather than a fixture count:
a hard count would red on a legitimate re-extraction instead of on the defect.

---

## 5. Verification run, verbatim

```
$ cargo clippy -p xtask --all-targets        # 0 warnings, 0 errors
$ rustfmt --edition 2021 --check crates/xtask/src/label_reader.rs   # clean
$ cargo nextest run -p xtask -E 'test(label_reader)'
     Summary [   0.024s] 12 tests run: 12 passed, 87 skipped
$ ./target/debug/xtask label-proof over every committed *-DRAFT fixture
     FINAL: OK=14 FAILED=1        (was OK=0 FAILED=16)
```

`cargo nextest run -p xtask --no-fail-fast` (the whole crate) is
`98 tests run: 97 passed, 1 failed, 1 skipped`; the single failure is F6 above, in
`authority_manifest`, and is not mine.
