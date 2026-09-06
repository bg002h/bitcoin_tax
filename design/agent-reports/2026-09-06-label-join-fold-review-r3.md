# r3 — review of the r2 fold (`fab4f2fa`)

Reviewer: Claude Opus 5 (1M), independent, read-only. Date: 2026-09-06.
HEAD when this review began: `fab4f2fa` (branch `main`), working tree clean apart from `CONTINUITY.md`.

Scope: the ONE question — does `fab4f2fa` resolve N1–N9 of
`design/agent-reports/2026-09-06-label-join-fold-review-r2.md` (0C/2I/4M/3N) without introducing a
new Critical or Important, and does every checker it adds actually discriminate?

Taken as SETTLED, not re-derived: everything r1/r2 and both VERIFICATION ledgers established (9/9
TRUE); the two-witness design; the owner rulings.

Read: `git show fab4f2fa` (all four files); `crates/xtask/src/label_reader.rs` at `fab4f2fa`
(`numbered_line_keys`, `GRID_MAPS`, `map_reach_problem`, `line_bindings`, `label_matches`,
`every_map`, `audit_year_reach`, `YEAR_FLOORS`, `every_mapped_line_lands_on_its_own_printed_label`,
the three plants); `crates/xtask/src/form_delta.rs` at `fab4f2fa` (`check_work_list`,
`parse_work_list_row`, `the_work_list_checker_reds_on_every_planted_row`, and the unchanged
`pdf_for`/`field_set`/`compute`); `crates/xtask/src/form_geometry.rs` (`extract`,
`pdf_rel_for_stem`); all 37 `crates/btctax-forms/forms/*/*.map.toml`; `design/TY2026_WORK_LIST.md`
at `fab4f2fa`; `design/ROADMAP_STATUS.md` §2; the FOLLOWUPS FR-56/FR-57 entries; the geometry and
archived-PDF inventories.

### ⚠ Concurrency note, recorded because it bounds the evidence

Another session is working this tree live. During this review HEAD advanced to `d04335e2`
(a report-persist commit touching no code), and the working tree acquired modifications to
`FOLLOWUPS.md`, `design/TY2026_WORK_LIST.md`, `design/forms/MANIFEST.json` and — after my gate run —
**`crates/xtask/src/form_delta.rs`** (a follow-on UNWITNESSED axis, FR-58), plus a new untracked
`design/forms/geometry/f1040s1--2025.json`.

- My gate capture is timestamped **04:46:54**; the first of those mutations is **04:48:57**. The run
  therefore measured `fab4f2fa` with a clean `crates/` and a clean `design/forms/`.
- `git diff fab4f2fa -- crates/xtask/src/label_reader.rs` is **empty**; every source claim below
  about `form_delta.rs` was read from `git show fab4f2fa`, not from the working tree.

## Gate numbers, reproduced

One run, captured once to `/tmp/…/lj-r3.txt` and grepped (never twice):

```
cargo nextest run --locked -p xtask -E 'test(label) | test(work_list) | test(map_reach) |
  test(binding_shape) | test(planted) | test(checkbox)' --no-capture
Summary [0.409s] 19 tests run: 19 passed, 109 skipped
```

| commit message says | this run says | verdict |
|---|---|---|
| 2024: 17 map(s), 261 join(s), 1 unreachable | `2024: 17 map(s), 261 join(s) checked, 1 unreachable` | **matches** |
| 2025: 15 map(s), 215 join(s), 0 unreachable | `2025: 15 map(s), 215 join(s) checked, 0 unreachable` | **matches** |
| 0 wrong | `wrong` assert green; `0 binding(s) landed on a box the reader could not label (\`?\`)` | **matches** |
| work list: 13 compared / 5 excused / 18 stems | `work list: 13 compared, 5 excused, 18 stems on the emitting surface` | **matches** |
| 19/19 tests | `19 tests run: 19 passed` | **matches** |

Also printed and checked: `2017: 5 map(s), 0 join(s), 5 unreachable`, with all six NOT WITNESSED
maps named (2017 × 5, plus `2024/f8283`).

## Checklist — N1–N9

| item | verdict | evidence |
|---|---|---|
| **N1** `[lineN]` sections parsed AND counted | **RESOLVED** | Both parsers independently re-implemented and run over all 37 committed maps: they agree with the test's own printed per-map line on **every** map. The new parser is a strict **superset** — `LOST = 0` on every map — and captures **exactly** the section FQNs and nothing else (`EXTRA_beyond_sections = 0` on every map). See (a). |
| **N2** per-(year, form) grid reasons | **RESOLVED** (see R1, Minor) | `GRID_MAPS` is `&[(&str, &str, &str)]`; `2024/f8283` is gone and planted (`p("2024","f8283",0,0,0).is_some()`); each listed reason is true of that map. One measured zero-key map is unlisted — R1. |
| **N3** ROADMAP floors | **RESOLVED** | §2 no longer prints numbers ("floors held by `YEAR_FLOORS` … the numbers live in the test, not here"). `grep -E '\b(235\|193\|249\|201)\b' design/ROADMAP_STATUS.md` → no hits. |
| **N4** both ratchet comments | **RESOLVED** | 2024: `99 → 235 … 235 → 249 (L4) … 249 → the value below (r2 N1)` above `min_joins: 261`; 2025: `82 → 193 … 193 → 201 (L4) … 201 → the value below (r2 N1)` above `215`. Every leg matches a measured delta (L4 = +14/+8; N1 = +12/+14). |
| **N5** partial-drop arm | **RESOLVED** | `(false, k, e, _) if e < k` present and planted (`p("2025","f1040sa",19,1,1).is_some()`), with the healthy inline-table counter-case `p("2025","schedule_d",5,11,11).is_none()`. No committed map can trip it — see (c). |
| **N6** work-list checker planted for real | **PARTIAL** | The test is now a function over a document and four of five plants run the real loop through a distinct branch. But the *newly added* `claims_no_draft` conjunct has **no plant that reds on it**, and the commit message names one that does not reach it — **R2 (Important)**. Two further weaknesses: R3 (wrong artifact) and R5 (repo-state coupling). |
| **N7** quote-split assumption documented | **RESOLVED** | `line_bindings`' doc comment states the alternation assumption and that the failure mode is a phantom binding → false FAIL. Measured: zero `\"` in any `lineN` value, zero `lineN = { … } #` lines. |
| **N8** → follow-up | **RESOLVED as filed** | `FOLLUPS.md` FR-56, with the structural statement and an owning phase ("the TY2026 package"). |
| **N9** → follow-up | **RESOLVED as filed** | FR-57, with an owning phase ("the next `packet.rs` change"). |

## Verdicts (a)–(h)

### (a) N1 — section tracking and header counting. **Exact, superset-safe, and correctly bounded.**

I re-implemented `numbered_line_keys` and both the pre-fold and post-fold `line_bindings` and ran
them over every committed map. The per-map `keys`/`extracted` pairs agree with the test's printed
lines on all 31 reachable maps. Every map with a numbered section:

| map | pre-fold extracted | post-fold | delta | section FQNs | LOST | extra beyond sections |
|---|---|---|---|---|---|---|
| `2017/schedule_d` | 11 | 13 | +2 | 2 | 0 | 0 |
| `2024/f1040sb` | 4 | 10 | +6 | 6 | 0 | 0 |
| `2024/f8283` | 0 | 6 | +6 | 6 | 0 | 0 |
| `2024/schedule_d` | 23 | 29 | +6 | 6 | 0 | 0 |
| `2025/f1040s1a` | 46 | 52 | +6 | 6 | 0 | 0 |
| `2025/f1040sb` | 4 | 10 | +6 | 6 | 0 | 0 |
| `2025/schedule_d` | 11 | 13 | +2 | 2 | 0 | 0 |

- **Every `[lineN]` section's FQN now appears in the bindings**: the per-map delta equals the section
  FQN count exactly, on every map, in both directions.
- **The +26 matches the five live maps exactly.** Per-year deltas are 2017 **+2**, 2024 **+18**,
  2025 **+14**. 2024's `f8283` (+6) is unreachable and 2017 is entirely unreachable, so the *joined*
  movement is **+12 (2024) / +14 (2025) = +26** — precisely r2's count of 26 reachable FQNs on
  `2024/f1040sb`, `2024/schedule_d`, `2025/f1040sb`, `2025/schedule_d`, `2025/f1040s1a`.
- **Nothing else is captured.** `EXTRA_beyond_sections = 0` everywhere: no non-FQN quoted string
  inside a section survives the `contains('[') && contains('.')` filter, and no comment contributes
  (`#` lines are skipped before the section branch).
- **`[[lineN_rows]]`-style headers contribute nothing**, correctly (measured: no `[[line…]]` header
  exists in the tree; the residual shape is discussed in R6).
- **The section ends correctly at `[[part1_rows]]`.** `2024/f1040sb` and `2025/f1040sb` both put
  `[line8]` immediately above 14 `[[part1_rows]]` entries carrying 40+ FQNs, and the measured
  extracted count is 10, not 50. It also ends correctly at an ordinary header: `2025/f1040s1a`'s
  `[line22b]` is closed by `[census]` at line 231, and the map's 231 lines of census prose
  contribute nothing.
- **`[line_i]` / `[line_j]` (Schedule C) are correctly ignored** — `numbered()` requires a digit.
- **`[line7a_fbar]`** yields the binding line `7a_fbar`, which `label_matches` reduces to `7a` via
  its `split('_')` — pre-existing behaviour, and the run reports 0 wrong.

**The kill discriminates.** `every_binding_shape_is_parsed_and_counted` asserts the exact vector
`["1","3","3","7a","7a"]` and `numbered_line_keys(text) == 3`. Remove section tracking and the
vector becomes `["1","3","3"]` and the count `2` — both asserts red. B1 satisfied.

### (b) N2 — `GRID_MAPS` per (year, form, reason). **Each listed reason is true; one measured blank is unlisted.**

Measured over all 37 maps, the maps with **zero** numbered line keys are exactly six:

`2017/f8283`, `2017/f8949`, `2024/f8275`, `2024/f8949`, `2025/f8283`, `2025/f8949`.

`GRID_MAPS` lists five: `2017/f8949`, `2024/f8949`, `2025/f8949`, `2024/f8275`, `2025/f8283`.

- **Each stated reason is TRUE of that map**, read from the map: the three `f8949` maps bind only
  `[[rows]]` arrays; `2024/f8275` binds `part_ii_continuation`/`part_iv_continuation` arrays;
  `2025/f8283` binds `rows = [...]` arrays and holds no `[line…]` section (its 2024 sibling's
  `[line5a]/[line5b]/[line5c]` are absent from the Rev. 12-2025 map).
- **`2017/f8275` does not exist** — `forms/2017/` holds only `f1040`, `f8283`, `f8949`,
  `schedule_d`, `schedule_se`. So it is not an omission.
- **`2017/f8283` does exist, has 0 numbered keys, and is not listed** — while its equally
  unreachable sibling `2017/f8949` is. See **R1**.
- **`2024/f8283` is correctly out**: it binds three sections / six FQNs, and it is skipped before the
  reach line because it is unwitnessed (`every_map()` returns `Err` for it; the loop `continue`s
  before `map_reach_problem`). The plant `p("2024","f8283",0,0,0).is_some()` pins that it would red
  the moment it became reachable with zero keys.

### (c) N5 — could a legitimate map have `extracted < keys`? **Not today, and not by any shape in the tree.**

- Measured over all 37 maps: **zero** have `extracted < keys`. The only strict inequalities run the
  other way (inline tables and sections extract more than they key).
- **No numbered key has a non-FQN RHS.** A regex scan for `^line[0-9]\w* =` whose right-hand side
  contains no `[` returns nothing across all 37 maps — there is no `line5 = "n/a"` anywhere.
- **No `[lineN]` section holds only non-FQN values.** Every numbered section body is
  `yes/no = { field = "<FQN>", on = "…" }` or `col_* = "<FQN>"`; the minimum FQN yield per section
  is 2.
- The one way to trip the arm legitimately would be an empty placeholder section (`[line12]` with a
  TODO comment under it) — which is a line that genuinely is not bound, so the red is correct.
  Fail-closed either way.

### (d) N6 — `check_work_list` and its plants. **The refactor is right; one branch is unplanted and mis-described.**

Branches of `check_work_list`, and which plant reaches each:

| branch | reached by | discriminates? |
|---|---|---|
| A `(Some, Ok)`, cell mismatch | `\| \`f6251\` \| 62 \| 0 \| 0 \| 1 \| port \|` | **yes** — computed is `(62,0,0,0)` |
| B `(Some, Err)` — numeric row, no pair | `\| \`f1040\` \| 199 \| 0 \| 0 \| 0 \| unchanged \|` | **yes** — no `f1040--2026-DRAFT.pdf` |
| C `(None, Ok)` — excused but a pair exists | `\| \`f6251\` \| yes \| \`f6251--2025\` \| **NO DRAFT** — planted \| **NO DRAFT** \|` | **yes**, but see below |
| D `(None, Err)` + `claims_no_prior` && prior on disk | `\| \`f1040\` \| yes \| **NO PRIOR SIDE** — planted \| no draft either \| — \|` | **yes** |
| E `(None, Err)` + neither claimed | `\| \`f1040\` \| yes \| \`f1040--2025\` \| nothing claimed \| — \|` | **yes** |
| F `(None, Err)` + `claims_no_draft` && draft on disk | **nothing** | **NO** — R2 |

So four of the five plants do exercise different branches, and D and E are the ones that reach the
newly written excuse logic. **Plant C does not.** Both `f6251--2025.pdf` and `f6251--2026-DRAFT.pdf`
are archived, so `compute` returns `Ok`, the row falls into the pre-existing `(None, Ok(_))` arm and
reds as *"excused as having no pair, but form-delta computes one"* — never entering the
`(None, Err(_))` arm the plant's message describes. **R2 (Important)**.

**The cells are read correctly.** `parse_work_list_row` returns `cells[3]` (prior side) and
`cells[4]` (TY2026 side). Against the committed table:

| row | `cells[3]` | `cells[4]` | claim checked | on disk |
|---|---|---|---|---|
| `f1040` | `` `f1040--2025` `` | `**NO DRAFT** — …` | no `f1040--2026-DRAFT.json` | absent ✔ |
| `f1040s1` | `**NO PRIOR SIDE** — …` | `draft archived (…)` | no `f1040s1--2025.json` | absent at `fab4f2fa` ✔ |
| `f8283` | `` `f8283--2025` (Rev. 12-2025…) `` | `**NO DRAFT** — …` | no `f8283--2026-DRAFT.json` | absent ✔ |
| `f8275` | `` `f8275--2024` (Rev. 10-2024…) `` | `**NO DRAFT** — …` | no `f8275--2026-DRAFT.json` | absent ✔ |
| `f8995a` | `**NO PRIOR SIDE** — …` | `draft archived (…)` | no `f8995a--2025.json` | absent ✔ |

Column 3 is read for the two real NO PRIOR SIDE rows and column 4 for the three NO DRAFT rows, as
the brief asked. Confirmed.

**The "true excuse" plant does depend on repo state** — `excused == ["f1040"]` holds only while
`f1040--2026-DRAFT.json` (and its PDF) are absent. That is real coupling, and it moves: see **R5**.

**The check tests the wrong artifact.** The arm is entered because `compute` could not find a *PDF*
(`pdf_for`: bundled `crates/…/forms/{year}/{form}.pdf`, else `design/forms/{year}/{stem}.pdf`), but
the excuse is validated against a *geometry fixture* (`design/forms/geometry/{stem}.json`). See
**R3**.

### (e) `every_binding_shape_is_parsed_and_counted` and `std::env::temp_dir()`

Confirmed: `std::fs::remove_dir_all(&dir).ok()` is the last statement of the test, so a failed
assertion unwinds past it and leaks `/tmp/btctax-shapes-<pid>/shapes.map.toml`. On this box `/tmp`
is a 32 GB tmpfs and the artifact is ~400 bytes, on the failure path only, in a directory keyed by
pid (so no cross-run collision and no cross-test interference — this is the only test using it).
**Acceptable for a test; recorded as R7 (Nit).** A `tempfile::tempdir()` guard, or writing under
`target/`, would clean up on unwind and would also make a failed run reproducible without a stale
file on disk.

### (f) N3/N4 — the roadmap line and both ratchet comments. **Accurate at HEAD.**

- `design/ROADMAP_STATUS.md:184-185` now reads *"floors held by `YEAR_FLOORS` (the numbers live in
  the test, not here — fold review r2 N3)"*. This is a wider fix than r2's minimal change (which was
  to correct the numbers), and a better one: it removes the only floor claim in the tree that no
  test held. A grep of `ROADMAP_STATUS.md` for `235|193|249|201` returns nothing.
- Both `YEAR_FLOORS` comments now carry every move with its cause, and every leg reconciles against
  a measurement: L4 moved 2024 by +14 and 2025 by +8 (r2, settled); N1 moved them by +12 and +14
  (measured here). `235 + 14 = 249`, `249 + 12 = 261`; `193 + 8 = 201`, `201 + 14 = 215`. ✔
- The 2025 comment's collateral claim *"Thirteen of fifteen TY2025 maps contribute"* is true at HEAD
  (15 maps printed, `f8949` and `f8283` at 0).
- The 2024 entry's `why` — untouched by the fold — is **not** accurate. See **R4**.

### (g) The floors 261/215. **Measured, and a regression to 249/201 reds both years.**

`audit_year_reach` reds on `o.joins < f.min_joins`, so losing the section joins gives 249 < 261 and
201 < 215 — **two** findings, each naming the year and the shortfall (*"Coverage for this year
FELL"*). The pure-function kill
`the_per_year_audit_reds_on_a_new_year_a_lost_year_lost_coverage_and_a_lost_fixture` already plants
case (3) at exactly that shape. Ratchet direction is sound.

### (h) Anything outside what N1–N9 asked for

**Nothing material.** All seven diff hunks land inside `mod tests` (`form_delta.rs`) or
`mod map_label_join_tests` (`label_reader.rs`) — **zero production code changed**, which bounds the
blast radius to the instrument itself. The only edits not directly named by a finding are: hoisting
`repo_root()` into a `root` local in the work-list test (cosmetic), and rewording the two
`map_reach_problem` messages to mention the year and "with its reason" (consistent with N2). The
FOLLOWUPS additions are N8/N9 filed, numbered FR-56/FR-57 with no collision, each with an owning
phase as the repo's follow-up rule requires. One byte-level slip: **R8**.

## New findings

### R2 — IMPORTANT — the `claims_no_draft` half of the new excuse check has no plant, and the commit message names one that does not reach it

**Where:** `crates/xtask/src/form_delta.rs` @ `fab4f2fa` — `check_work_list`'s `(None, Err(_))` arm,
the conjunct `&& (!claims_no_draft || !draft)`; and
`the_work_list_checker_reds_on_every_planted_row`'s third plant.

**What is wrong:** the plant labelled *"a NO DRAFT claim with the draft fixture on disk"* is

```
| `f6251` | yes | `f6251--2025` | **NO DRAFT** — planted | **NO DRAFT** |
```

Both `design/forms/2025/f6251--2025.pdf` and `design/forms/2026/f6251--2026-DRAFT.pdf` are archived,
so `super::compute` returns `Ok` and the row is caught by the **pre-existing** `(None, Ok(_))` arm —
*"excused as having no pair, but form-delta computes one"*. The excuse arm is never entered, and
`claims_no_draft` is never evaluated.

Deleting the conjunct `&& (!claims_no_draft || !draft)` leaves **every one of the six assertions
green**, and the real document green too:

| plant | `ok` without the conjunct | still red? |
|---|---|---|
| A cell off by one | n/a (branch A) | yes |
| B numeric, no pair | n/a (branch B) | yes |
| C "NO DRAFT" f6251 | n/a (branch C) | yes |
| D NO PRIOR SIDE, prior on disk | `false` via `(!claims_no_prior \|\| !prior)` | yes |
| E claims neither | `false` via `(claims_no_prior \|\| claims_no_draft)` | yes |
| F true excuse | `true` | passes, as intended |

So B1's reviewable one-sentence question — *"which test reds when this checker is removed?"* — has
the answer **none** for the newly added NO DRAFT validation, while the fold's commit message asserts
`N6 … the plants run the real loop (… NO DRAFT with the draft on disk …)`. That is the r2 N6 class
recurring inside the fold that closed it, and it is what makes this a finding against the fold
rather than a pre-existing gap: the live code is correct, the guarantee is unheld, and the message
claims otherwise.

The branch is genuinely reachable (so this is a missing kill, not dead code): a form whose
prior-side PDF is absent but whose draft **is** archived, on a row claiming NO DRAFT, enters the arm
with `draft = true`.

**Minimal change:** one plant that isolates the conjunct —

```rust
let (_, _, wrong) = check_work_list(
    "| `f8995a` | yes | **NO PRIOR SIDE** | **NO DRAFT** — planted | — |\n",
);
assert_eq!(wrong.len(), 1, "a NO DRAFT claim with the draft on disk: {wrong:?}");
```

`compute("f8995a--2025", …)` errs (no prior PDF) so the excuse arm *is* entered;
`claims_no_prior` is satisfied by the absent `f8995a--2025.json`, so the prior conjunct passes;
`f8995a--2026-DRAFT.json` **is** on disk, so only `(!claims_no_draft || !draft)` can red it. Remove
that conjunct and this assertion fails — which is the whole property. (Then reword the existing
f6251 plant to say what it actually tests: an excused row whose pair exists.)

### R1 — MINOR — `GRID_MAPS` omits `2017/f8283`, a measured zero-key map, while listing its equally unreachable sibling `2017/f8949`

**Where:** `crates/xtask/src/label_reader.rs`, the `GRID_MAPS` constant.

**What is wrong:** the constant's own invariant is *"A map here with a numbered key, or a map NOT
here without one, is a red."* Measured, six maps have zero numbered keys; five are listed.
`2017/f8283` is the sixth. The inclusion rule is therefore neither "every zero-key map" (it misses
one) nor "every *reachable* zero-key map" (it includes `2017/f8949`, which is unreachable and whose
entry can never be exercised — 2017 has no geometry at all, so `every_map()` returns `Err` for all
five of its maps and the loop `continue`s before `map_reach_problem`).

Exposure is **fail-closed**: if TY2017 geometry ever lands — an open owner decision per FOLLOWUPS —
`2017/f8283` reds with *"NO numbered line key — a grid map must be named in GRID_MAPS with its
reason"*, which is a correct refusal, not a false pass. But it is a recorded-blank ledger with a
hole in it, which is the same class N2 was.

**Minimal change:** add `("2017", "f8283", "positional property rows; the Rev. 12-2014 map binds no
Section B question line")` after measuring it, **or** state in the doc comment that the constant
covers reachable maps only and drop `2017/f8949`. Either makes the rule decidable.

### R3 — MINOR — the excuse check validates a geometry fixture, but the arm it guards is entered on a missing PDF

**Where:** `crates/xtask/src/form_delta.rs` @ `fab4f2fa` — `check_work_list`'s `fixture` closure
(`design/forms/geometry/{stem}.json`) versus `super::compute` → `field_set` → `pdf_for`
(`crates/btctax-forms/forms/{year}/{form}.pdf`, else `design/forms/{year}/{stem}.pdf`).

**What is wrong:** the row's claim is about the **archive** (*"the draft URL served a 2025 document;
archiver refused"*), the arm's entry condition is about the **PDF**, and the verification is about
the **geometry JSON**. Those are three artifacts and only two of them are the same thing. A row
claiming `NO DRAFT` for a form whose draft PDF **is** archived but whose geometry was never
extracted is accepted as a true excuse, in the case where the prior-side PDF is also missing. That
is exactly the direction the check exists to catch.

Not live today (every archived 2026 draft has a geometry fixture), so this is an unsound proxy
rather than a wrong result.

**Minimal change:** test the artifact the claim is about —
`super::pdf_for(&format!("{form}--2026-DRAFT")).is_some()` and the `--2025` equivalent — which is
also the artifact `compute`'s `Err` is reporting on, so the entry condition and the verification
finally speak about the same file. (`pdf_for` is private to the module; it is in the same crate.)

### R4 — MINOR — the TY2024 `YEAR_FLOORS` reason is contradicted by the run's own NOT WITNESSED line, and it excuses six live section-bound bindings

**Where:** `crates/xtask/src/label_reader.rs`, `YEAR_FLOORS[2024].why` — the struct literal whose
`min_joins` this fold edited two lines above.

**What is wrong:** the recorded reason is

> "f8283 — design/forms/ holds i8283--2024 (the instructions) but not the form, so **there is no PDF
> to extract geometry from**."

The same test run prints, four lines later:

> `NOT WITNESSED f8283 — no geometry fixture was observed from
> /scratch/code/bitcoin_tax/crates/btctax-forms/forms/2024/f8283.pdf (sha256:e94015ab2ca0…) —
> generate one with \`xtask extract-geometry <stem>\``

That PDF exists (181,414 bytes) and is the very file `every_map()` hashes to find a fixture, so a
fixture extracted from those bytes would join by hash. What is actually missing is the **archived
authority** at `design/forms/2024/f8283--2024.pdf` — the only path `form_geometry::extract` will
read (`pdf_rel_for_stem` → `design/forms/{year}/{stem}.pdf`), which means the run's own remediation
instruction errors out for this stem.

The cost is not cosmetic: `2024/f8283` binds `[line5a]/[line5b]/[line5c]` — the Section B
RESTRICTION questions, six FQNs — and those are precisely the shape N1 was written for. They sit
unchecked behind an excuse whose stated reason is false as written.

**Minimal change:** *"f8283 — `design/forms/2024/` holds only `i8283--2024` (the instructions); the
form authority was never archived, so `xtask extract-geometry f8283--2024` has no input. The bundled
`crates/btctax-forms/forms/2024/f8283.pdf` is not an extraction source. Archiving the authority
would join 3 lines / 6 bindings."*

### R5 — MINOR — the true-excuse control is coupled to repo state, and that state demonstrably moves

**Where:** `the_work_list_checker_reds_on_every_planted_row`, the final
`assert!(wrong.is_empty() && excused == ["f1040"])`.

**What is wrong:** the control passes only while `f1040--2026-DRAFT` has neither a PDF nor a
geometry fixture. Archiving that draft flips the plant from `excused` to `wrong` (via the
`(None, Ok(_))` arm), reddening a *plant* rather than the document it protects — so the failure will
be read as a broken test, not as a work-list row that needs rewriting.

This is not hypothetical. Within four minutes of `fab4f2fa`, a concurrent session archived
`f1040s1--2025` (PDF + extract + geometry) and rewrote its work-list row — the exact state
transition, on a sibling stem.

**Minimal change:** build the true-excuse control from a stem that cannot be archived — e.g. a
synthetic `zzz-not-a-form` — so the control tests the *predicate*, not the inventory. Keep the
inventory coupling where it belongs, in `the_committed_work_list_matches_form_delta_at_head`.

### R6 — MINOR — the `[[` guard cannot fire, and the shape it names is the residual blind spot

**Where:** `numbered_line_keys` (`if rest.starts_with('[') { return false; }`) and the mirror in
`line_bindings` (`section = if rest.starts_with('[') { None } else { … }`).

**What is wrong:** both are inert. For `[[part1_rows]]`, `rest.split(']').next()` yields
`"[part1_rows"`, whose `strip_prefix("line")` already returns `None` — so deleting the guard changes
nothing, and `every_binding_shape_is_parsed_and_counted`'s *"nothing from the grid row"* clause
stays green either way. It is a guard with no kill because it has no effect.

The shape it appears to address is the real point: a numbered line bound as an **array of tables**
(`[[line5_rows]]`) is invisible to both the counter and the parser — the N1 defect one level up. It
occurs nowhere today (measured: no `[[line…]]` header in any of the 37 maps, so this is not a live
gap), and a map binding *all* its lines that way reds fail-closed on `keys == 0`. But a map binding
*some* lines that way keeps `keys` and `extracted` balanced and those lines are silently unchecked —
the exact mechanism r2's N1 described.

**Minimal change:** make the guard mean something — treat `[[line<digit>…]]` as a numbered key and
extract its FQNs, or delete the two dead branches and say in the doc comment that an array-of-tables
binding of a numbered line is out of scope and reds on `keys == 0` only if it is the map's only
shape.

### R7 — NIT — the shape kill leaks its scratch file on the failure path

`every_binding_shape_is_parsed_and_counted` removes `std::env::temp_dir()/btctax-shapes-<pid>` only
as its last statement, so a failed assertion unwinds past the cleanup. ~400 bytes on a 32 GB tmpfs,
failure path only, pid-keyed so no collision. Acceptable; a `tempfile::tempdir()` guard (or writing
under `target/`) would clean up on unwind.

### R8 — NIT — `FOLLOWUPS.md` now ends without a trailing newline

`git show fab4f2fa:FOLLOWUPS.md | tail -c 4` → `).**` with no `\n`; the diff carries
`\ No newline at end of file`. The next append will land on FR-57's line.

### R9 — NIT — the section parser widens the quote-split exposure the N7 comment documents

Inside a `[lineN]` section, **every** quoted FQN on **every** non-comment line is attributed to that
line, so a trailing comment quoting a superseded FQN (`field = "a[0].b[0]"  # was "a[0].old[0]"`)
would produce a phantom binding. The doc comment's phrasing ("in a `lineN` value") predates the
section path. Measured zero occurrences today (`EXTRA_beyond_sections = 0` on all 37 maps) and the
failure mode is a false FAIL. One clause in the doc comment would cover it.

## Counts

**0 Critical / 1 Important / 5 Minor / 3 Nit.**

R2 is not a regression in behaviour — the code it concerns is correct — but it is an unheld
guarantee in a *newly added* checker, asserted as held in the fold's own commit message, which is
the standard this repo's B1 rule and the r2 report both applied. R1, R3, R5 and R6 are latent and
fail-closed; R4 is pre-existing (the fold did not introduce it) but sits two lines below an edit
this fold made, is contradicted by the run's own output, and hides six live section-bound bindings
of exactly the class N1 was about.

## May the instrument be trusted to GATE a new map?

**YES.** r2's split verdict ("YES for the surface it covers; NO for the `[lineN]` section surface")
is closed.

- **Covered: 476 joins** (2024 = 261, 2025 = 215) across **31 reachable maps** in two years, with
  **0 wrong** and **0 unlabelled** — every `lineN = "FQN"`, every `lineN = { … }` inline table, and
  now every `[lineN]` table section. The 26 FQNs r2 named as invisible are now parsed, counted,
  joined and label-matched, and all 26 land on the label the form prints.
- **All three shapes are held by a kill** that reds if either the parser or the counter loses the
  section shape, and `map_reach_problem` reds — each planted — on a total drop (`19/0/0`), a partial
  drop (`19/1/1`), a zero-key non-grid map, a grid map that grew a key, and a per-year map now
  listed for the wrong year (`2024/f8283`). A new map that binds a Yes/No pair or a sub-row table —
  how every checkbox line in this tree is written — is genuinely gated.
- **Bounded caveats to state in any gating brief:** (i) a numbered line bound as
  `[[lineN_rows]]` is still unseen by both halves and would be silently unchecked if mixed with
  other shapes (R6 — no such map exists today); (ii) `2024/f8283`'s six section FQNs remain
  unchecked behind an excuse whose recorded reason is wrong (R4), and archiving
  `design/forms/2024/f8283--2024.pdf` would close it; (iii) the work-list half of the suite carries
  one unplanted conjunct (R2).

None of those touches the line→label gate a new *map* passes through. Gate a new map on this
instrument.
