# r4 — verification of the r3-review fold (`3070a797`)

Reviewer: Claude Sonnet 5, independent, read-only. Date: 2026-09-06.
HEAD when this review began: `3070a797613fd9f7dd1a2682a02d9f22a8054107` (branch `main`), the fold
under review.

Scope: the ONE question — does `3070a797` resolve R1–R9 of
`design/agent-reports/2026-09-06-label-join-fold-review-r3.md` (0C/1I/5M/3N), and does each plant it
adds or changes actually DISCRIMINATE (would deleting the code it guards make the assertion fail)?

Taken as SETTLED, not re-derived: the r3 report's own findings and everything the r3 VERIFICATION
ledger (`…r3-VERIFICATION.md`, 10/10 TRUE) established.

### Concurrency note, recorded because it bounds the evidence

Another session is working this tree live, matching the pattern r3 itself recorded. Mid-review, HEAD
advanced from `3070a797` to `ddd0df68` — an unrelated spec fold (`spec(4868/1040-V): r2`, touching only
`FOLLOWUPS.md` and `design/SPEC_form_4868_1040v.md`; `git show ddd0df68 --stat` confirms zero touches
to `crates/xtask/`). `git merge-base --is-ancestor 3070a797 HEAD` → **YES**, a clean fast-forward. A
transient `M  FOLLOWUPS.md` (staged, empty diff) caught mid-write by that concurrent commit resolved to
byte-identical-with-HEAD within seconds. All source claims about the fold below are read from
`git show 3070a797:<path>` directly (not the moving working tree); the gate run and directory listings
were taken from the working tree after the race settled, and reflect `3070a797`'s `crates/xtask/`
content unchanged by `ddd0df68`.

## Gate numbers, reproduced

One run, captured once to `/tmp/claude-1000/-scratch-code-bitcoin-tax/lj-r4.txt` and grepped (never
twice):

```
cargo nextest run --locked -p xtask -E 'test(label) | test(work_list) | test(map_reach) |
  test(binding_shape) | test(planted) | test(checkbox)' --no-capture
Summary [0.408s] 19 tests run: 19 passed, 109 skipped
```

| expected | this run | verdict |
|---|---|---|
| 2024: 261 joins, 1 unreachable | `2024: 17 map(s), 261 join(s) checked, 1 unreachable` | **matches** |
| 2025: 215 joins, 0 unreachable | `2025: 15 map(s), 215 join(s) checked, 0 unreachable` | **matches** |
| 0 wrong | `0 binding(s) landed on a box the reader could not label (\`?\`)` | **matches** |
| work list 14/4/18 | `work list: 14 compared, 4 excused, 18 stems on the emitting surface` | **matches** |
| 19/19 | `19 tests run: 19 passed` | **matches** |

Identical to r3's own reproduction (261/215/0/19-19) — confirms the R6 widening (below) changed no
live join count, only added dormant coverage.

## Checklist — R1–R9

| item | severity (r3) | verdict | evidence |
|---|---|---|---|
| **R1** `GRID_MAPS` omits `2017/f8283` | Minor | **RESOLVED** | `label_reader.rs` `GRID_MAPS` now lists 6 entries; `2017/f8283` added with reason `"positional property rows; the Rev. 12-2014 map binds no Section B question line (unreachable: … r3 R1)"`. Measured: the map's `section_a`/`section_b` bind only `rows = […]` arrays and a checkbox (`k_digital_assets`); no `[lineN]` header anywhere — reason is TRUE. Six zero-key maps exist tree-wide (`2017/f8949`, `2017/f8283`, `2024/f8275`, `2024/f8949`, `2025/f8283`, `2025/f8949`), all six now listed — the ledger is total. |
| **R2** `claims_no_draft` conjunct unplanted | **Important** | **RESOLVED** | New plant on `f8995a` isolates the conjunct; see (a) below — confirmed to discriminate by hand-trace and an independent boolean-mirror script. Old `f6251` plant relabeled `"an excused row whose pair EXISTS"`, matching what it actually exercises (branch `(None, Ok(_))`, not the excuse arm). |
| **R3** excuse check read the wrong artifact | Minor | **RESOLVED** | `fixture` (geometry JSON) replaced with `archived = |stem| super::pdf_for(stem).is_some()` — literally the same function `compute → field_set → pdf_for` uses, not merely "the same paths." See (e). |
| **R4** TY2024 floor `why` was false | Minor | **RESOLVED** | New text: authority never archived at `design/forms/2024/f8283--2024.pdf` (confirmed absent — only `i8283--2024.pdf` there); bundled `crates/btctax-forms/forms/2024/f8283.pdf` (181,414 B, confirmed present) is not an extraction source (`pdf_rel_for_stem` only ever builds `design/forms/{year}/{stem}.pdf`, confirmed by reading `form_geometry.rs:125-128`); "3 lines / 6 section-bound bindings" is exact — `2024/f8283.map.toml` has `[line5a]`/`[line5b]`/`[line5c]` (3) binding 6 quoted FQNs (measured). |
| **R5** true-excuse control coupled to repo state | Minor | **RESOLVED** | Control switched from `f1040` (real, archivable stem) to synthetic `zzz-not-a-form`, which can never be archived. See (b). |
| **R6** inert `[[` guard; array-of-tables numbered lines invisible | Minor | **RESOLVED** | Both `numbered_line_keys` and `line_bindings` replace the inert `starts_with('[') → return/None` guard with `trim_start_matches('[').split(']').next()`, so `[[lineN…]]` now counts and yields FQNs while `[[part1_rows]]`-style positional headers (not "line"-prefixed) still correctly return nothing. See (c) and (f) — dormant today (0 occurrences), fail-closed either way. |
| **R7** scratch dir leaks on failure | Nit | **RESOLVED** | `every_binding_shape_is_parsed_and_counted` now wraps the temp dir in a `Scratch(PathBuf)` with a `Drop` impl that calls `remove_dir_all` — runs on unwind, not just the happy path. |
| **R8** `FOLLOWUPS.md` missing trailing newline | Nit | **RESOLVED** | `git show 3070a797:FOLLOWUPS.md \| tail -c 5 \| xxd` → `292e 2a2a 0a` (`).**\n`) — newline present at the reviewed commit. (The working tree showed the pre-fix byte pattern for a few seconds mid-review, from the unrelated concurrent commit in flight; resolved once that commit landed, see the concurrency note.) |
| **R9** doc comment silent on in-section attribution | Nit | **RESOLVED** | `line_bindings`'s doc comment now states: "inside a section EVERY quoted FQN on every non-comment line is attributed to the line … if one appears, the failure is a phantom binding → a false FAIL, never a false PASS." |

**9/9 RESOLVED**, matching the fold commit message's own claims for each item.

## Verdicts (a)–(f)

### (a) R2's new plant — `f8995a`, NO PRIOR SIDE / NO DRAFT — planted

Directory listings (working tree, unaffected by the concurrent commit):

- `design/forms/2025/` — no `f8995a*` entry (only `f6251--2025.pdf`/`.txt` among forms near it).
- `crates/btctax-forms/forms/2025/` — no `f8995a.pdf`/`.map.toml` (full listing checked: f1040,
  f1040s1a, f1040s2, f1040s3, f1040sa, f1040sb, f1040sc, f6251, f8283, f8949, f8959, f8960, f8995,
  schedule_d, schedule_se — no f8995a).
- ⇒ `super::pdf_for("f8995a--2025")` is **None** (both the bundled and archived checks miss).
- `design/forms/2026/` — `f8995a--2026-DRAFT.pdf` **present**.
- `crates/btctax-forms/forms/2026/` does not exist as a directory.
- ⇒ `super::pdf_for("f8995a--2026-DRAFT")` is **Some** (the archived fallback hits).

So `field_set("f8995a--2025")` errs, `compute` propagates `Err` via `?`, and `check_work_list` enters
`(None, Err(_))`. `parse_work_list_row` on `"yes"` in the numeric-cell position fails `.parse::<usize>()`
→ `cells: None`, confirming the branch dispatch. Inside the arm: `prior = archived("f8995a--2025") =
false`, `draft = archived("f8995a--2026-DRAFT") = true`, `claims_no_prior = true` (cell contains "NO
PRIOR SIDE"), `claims_no_draft = true` (cell contains "NO DRAFT").

Current: `ok = (T∨T) ∧ (¬T∨¬F) ∧ (¬T∨¬T) = T ∧ T ∧ F = false` → `wrong.len() == 1`. ✔ matches the
assertion.

**Deleting `&& (!claims_no_draft || !draft)`:** `ok = (T∨T) ∧ (¬T∨¬F) = T ∧ T = true` → the row is
`excused` instead of `wrong`, `wrong.len() == 0`, and `assert_eq!(wrong.len(), 1, …)` **fails**.
Verified two ways: by hand and by an independent Python mirror of the exact boolean expression against
the measured on-disk facts (`ok_current = False`, `ok_without_conjunct = True` — the assertion flips).
Also confirmed the `claims_no_prior`/`prior` term is a tautology for this plant (`prior = False`,
`claims_no_prior = True` ⇒ `¬claims_no_prior ∨ ¬prior` is always `True`), so the plant is a clean
isolation of the `claims_no_draft` conjunct alone, not a conflation of both. **Discriminates.**

### (b) R5's synthetic control — `zzz-not-a-form`

No stem `zzz-not-a-form` exists anywhere in `crates/btctax-forms/forms/*/` or `design/forms/*/`
(synthetic by construction). `pdf_for("zzz-not-a-form--2025")` and
`pdf_for("zzz-not-a-form--2026-DRAFT")` are both **None** ⇒ `compute` returns **Err** ⇒ the `(None,
Err(_))` arm is entered. `prior = draft = false`; `claims_no_prior = claims_no_draft = true` (row
claims both). `ok = (T∨T) ∧ (¬T∨¬F) ∧ (¬T∨¬F) = T ∧ T ∧ T = true` → `excused == ["zzz-not-a-form"]`,
`wrong` empty. Matches the assertion, and unlike the old `f1040` control this stem can never be
archived by a future session, so the control tests the *predicate* rather than current inventory state.

### (c) R6 — `[[line5_rows]]`

Traced `line_bindings` on the test's fixture text: for the header `[[line5_rows]]`,
`l.strip_prefix('[')` → `"[line5_rows]]"`; `.trim_start_matches('[')` → `"line5_rows]]"`;
`.split(']').next()` → `"line5_rows"`; `numbered("line5_rows")` strips `"line"` → `"5_rows"`, first
char `'5'` is a digit → `section = Some("5_rows")`. The next line `cell = "a[0].b[0].f1_30[0]"` is
attributed to that section, yielding `("5_rows", "a[0].b[0].f1_30[0]")` — matches the test's expected
`["1","3","3","7a","7a","5_rows"]`.

`label_matches("5_rows", "5")`: `base = "5_rows".split('_').next() = "5"`; `base == printed` (`"5" ==
"5"`) → **true**. So a real `[[lineN_rows]]` binding **would** join correctly against a printed label
`"5"` — not a false FAIL. (The immediately following `[[part1_rows]]` header in the same fixture
correctly does **not** open a numbered section: `numbered("part1_rows")` fails `strip_prefix("line")`
since `"part1_rows"` doesn't start with `"line"`, so `section = None` — the grid-row exclusion still
holds.)

### (d) R1 — `2017/f8283` under the new counter

```
grep -cE '^\s*(line[0-9]|\[\[?line[0-9])' crates/btctax-forms/forms/2017/f8283.map.toml
→ 0
```

Confirmed by reading the full file: it contains only `form`/`year`/metadata scalars, `[section_a]` and
`[section_b]` headers (neither numbered), and `rows = […]` positional arrays. No `line…` key and no
`[line…]`/`[[line…]]` header anywhere. **0 numbered keys**, matching the reason text and GRID_MAPS
entry.

### (e) R3 — does `pdf_for` resolve the same paths `compute` uses?

`compute(old, new)` (`form_delta.rs:277`) calls `field_set(old)?` and `field_set(new)?`;
`field_set(stem)` (`form_delta.rs:63`) calls `pdf_for(stem).ok_or_else(...)?` before doing anything
else. So the excuse check's `archived = |stem| super::pdf_for(stem).is_some()` calls the **identical
function** `compute`'s `Err` originates from — not merely "the same paths," the same code. This is
airtight by construction: as long as `field_set`'s first line stays `pdf_for(stem)`, the excuse check
and `compute`'s failure mode cannot diverge.

### (f) Any `[[line…]]` header anywhere in the 37 committed maps?

```
find crates/btctax-forms/forms -name "*.map.toml" | wc -l → 37
grep -rnE '^\s*\[\[line[0-9]' crates/btctax-forms/forms/*/*.map.toml → (no matches, exit 1)
grep -rnE '\[\[line' crates/btctax-forms/forms/*/*.map.toml → (no matches, exit 1)
```

**Zero.** The R6 widening is purely additive/dormant — confirmed by the gate numbers above being
byte-identical to r3's own pre-fold reproduction (261/215/0/19-19). No real map's join count changed.

## New findings

**None.** No Critical, Important, Minor, or Nit findings beyond what R1–R9 already named and this
review confirms resolved. Specifically checked and found clean:

- The `f6251` branch-A plant (line 486, `"62 | 0 | 0 | 1 | port"`) is untouched by this fold's hunks
  and still exercises branch A independently of the R2/R5 changes.
- The D/E plants (`f1040` NO PRIOR SIDE / neither-claimed) are unmodified by this fold and were
  already confirmed discriminating in r3 — not re-derived here, out of this fold's diff.
- No regression in ordinary (non-array) section handling: `numbered()` still correctly returns `None`
  for any non-`"line"`-prefixed header (`[census]`, `[section_a]`, `[[part1_rows]]`), so section
  boundaries for the 5 live `[lineN]`-bearing maps are unchanged — consistent with the gate reproducing
  identical 261/215 counts.
- `GRID_MAPS`'s own invariant ("a map here with a numbered key, or a map NOT here without one, is a
  red") now holds without exception: exactly 6 zero-key maps measured, exactly 6 listed.

## Counts

**0 Critical / 0 Important / 0 Minor / 0 Nit** (new). All 9 of r3's findings (1 Important, 5 Minor, 3
Nit) are RESOLVED, each plant traced to discriminate on the exact conjunct/shape it names, and the gate
numbers reproduce exactly.

## May the instrument be trusted to gate a new map?

**YES**, unchanged from r3's verdict, now with R2's excuse-check gap closed: the work-list checker's
`(None, Err(_))` arm is fully planted on both conjuncts, the excuse check reads the artifact `compute`
actually fails on, `GRID_MAPS`'s zero-key ledger is total, and `[[lineN…]]` array-of-tables bindings —
though absent from every committed map today — would now join and label-match correctly rather than
vanish silently.
