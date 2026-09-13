# REPORT — A2: FR-141 (the per-`(stem, year)` cost) and FR-146 (the misattached doc comments)

**Agent:** A2, opus, isolated worktree `376d1141`, `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-a2`.
**Date:** 2026-09-12. **Nothing committed, nothing pushed.** Three files changed, all owned:
`crates/btctax-forms/build.rs`, `crates/btctax-forms/src/line_set.rs`,
`crates/btctax-forms/src/f6251_revision.rs` — `+498 / -278`.

## 0. The headline

| question | answer |
|---|---|
| **Hand-edits in my three files per new `(stem, year)`** | **7 -> 1.** The one that remains is the `line_set::schema` arm, and it is still an `E0004`. |
| **Of the 7, how many were forced by a red or a compile error** | **5.** Two — the `LineSet::ALL` entry and the `ALL.len()` count beside it — were forced by **nothing**: measured, omitting both leaves the whole `btctax-forms` suite with the *identical* failure set. |
| **So FR-141's second half is not a keystroke finding** | A variant missing from `ALL` is a revision **every gate that walks the revision set skips in silence** (`line_set_wiring`, `f6251_obbba`, `supported_years_cross_product` all iterate `ALL`). Deriving it closes a hole. |
| **Did the build error survive generation** | **Yes, and it was watched.** A planted `(stem, year)` with no `schema()` arm: `error[E0004]: non-exhaustive patterns: LineSet::F8995a_2025 not covered --> crates/btctax-forms/src/line_set.rs:141:11`, and **nothing in `f6251_revision.rs`**. |
| **Does a missing Form 6251 revision still stop the build** | **Yes** — `error[E0080]: evaluation panicked` from the new `const _: ()` walk, pasted in section 4. |
| **FR-146** | Closed **structurally** (the doc comment and the variant are emitted from one string) **and** machine-checked, with the generator's own kill watched red. |

## 1. What changed, and where

### `build.rs` — it now generates `LineSet`

It already globbed `forms/<year>/` and emitted `bundled.rs`. It now also emits
`$OUT_DIR/line_set_generated.rs`: the `LineSet` enum (one variant per **distinct** `line_set` row, with a
doc comment naming the row and the map file(s) that carry it), `LineSet::parse`, `LineSet::as_str` and
`LineSet::ALL`.

- The `line_set` value is read by `line_set_of()` — a deliberate **line scan**, not a TOML parse, because
  this crate has no `[build-dependencies]` and `Cargo.toml` is not mine to edit. The strictness is what
  makes a scan safe: **exactly one** `line_set = "…"` assignment, **in the header** (before the first
  `[section]`), a quoted value, and the shape `<stem>/<four-digit year>` — anything else is a build error.
  All four refusals were watched red (section 4.4).
- `line_set_variant()` derives the variant name with the file's existing one rule (`variant_name` +
  `_<year>`), and two rows that would spell the **same** variant from **different** strings is a build
  error naming both rows.
- Revisions are **deduped in glob order**, so the design's future many-to-one collapse (a constants-only
  year reusing its predecessor's revision, design r2 section 4) is one variant carried by two files, not
  two variants — and the doc comment then names both files.

**The generated variant set is identical to the hand-written one it replaced:** 38 = 38, a clean `diff`
of `sort -u` over both. Declaration order differs in one place only — `f8889` sorts before `f8949`, where
the hand-written list had it after (the same slip that produced FR-146). Nothing in the workspace depends
on `LineSet`'s `Ord` (grepped).

### `src/line_set.rs` — only the judgment is typed

The enum, `parse`, `as_str` and `ALL` became
`include!(concat!(env!("OUT_DIR"), "/line_set_generated.rs"))`. What stayed hand-written is `Schema`,
its docs, and **`schema()` — still an `_`-free match over every variant**, now `pub const fn` so
`f6251_revision` can hold its own totality at compile time.

The per-variant prose that the generated docs cannot carry (the eight "wired at step 5" notes, the
`f8889/2025` same-line-set measurement, the `f6251/2025` and `f1040s1a/2025` notes) was **relocated into
the `schema()` arms** — beside the decision it is about. Nothing was dropped.

Three tests where there were two:

- `every_variant_round_trips_through_its_string` — kept, minus `assert_eq!(LineSet::ALL.len(), 38)`.
- `every_bundled_map_names_the_revision_the_generator_attributed_to_it` — **new, and the replacement for
  that count.** For every bundled `(stem, year)`: `MapRow::read` (the real TOML parse) names a revision,
  `LineSet::parse` must know it, and the **generator's own doc line must name that same map file**. Set
  equality both ways on top. Two independent readers of one string, joined per map.
- `every_generated_doc_comment_names_its_own_row` — **new**, FR-146's machine check (section 3).

### `src/f6251_revision.rs` — the trap kept, its blast radius removed

`revision()` was an `_`-free match over **every** `LineSet`, so porting Form 8995-A produced an `E0004`
in a Form 6251 module. It is now `LineSet::F6251_2025 => Some(&F6251_2025), _ => None` — and the `_` is
**guarded, not swept**:

```rust
const _: () = {
    let mut i = 0;
    while i < LineSet::ALL.len() {
        let ls = LineSet::ALL[i];
        if matches!(crate::line_set::schema(ls), crate::line_set::Schema::Form6251ObbbaMap) {
            assert!(revision(ls).is_some(), "…");
        }
        i += 1;
    }
};
```

`schema()` is `const fn` and `revision()` is now `const fn`, so the walk runs **during compilation**:
a revision joining this field map without its own cells is `E0080`. It is derived twice over — `ALL`
comes from the rows, the schema from the one hand-written match — so there is no list here to go stale.
The const is anonymous because a *named* unused const is a `dead_code` warning and `-D warnings` would
have made the guard itself break the build; `tests::every_obbba_revision_has_cells` stays, because a
const panic cannot format *which* revision and that test can.

**No product behaviour changed.** No emitted figure, no map, no PDF, no refusal text a filer can see.

## 2. The measurement — 7 before, 1 after

Method, the same shape as the rehearsal's: plant the data half of a real port
(`forms/2025/f8995a.{pdf,map.toml}`, the 2024 map with `year` and `line_set` rewritten — the PDF is a
byte copy so `template_sha256` still holds), then count the edits **in my three files** that the compiler
or the suite forces, applying them one at a time. Plant removed afterwards; the working tree is clean but
for the three files.

### BEFORE (pristine `376d1141`) — 7 edits, of which 5 were forced

| # | edit | what forced it |
|---|---|---|
| 1 | `LineSet::F8995a_2025` variant | `supported_years_cross_product`: *"(2025, f8995a) is missing {"dispatch"} and NOTHING records it"*, and `field_census` |
| 2 | `parse` arm | the same two reds — without it the revision does not resolve |
| 3 | `as_str` arm | **E0004** |
| 4 | `schema()` arm | **E0004** |
| 5 | `f6251_revision::revision` "not this schema" arm | **E0004** — in a module about Form 6251 |
| 6 | `ALL` entry | **NOTHING** (measured below) |
| 7 | `ALL.len()` 38 -> 39 in the unit test | **NOTHING** — it only fires once #6 is done |

Adding the variant alone produced exactly three E0004s:

```
error[E0004]: non-exhaustive patterns: `LineSet::F8995a_2025` not covered
error[E0004]: non-exhaustive patterns: `LineSet::F8995a_2025` not covered
error[E0004]: non-exhaustive patterns: `LineSet::F8995a_2025` not covered
error: could not compile `btctax-forms` (lib) due to 3 previous errors
```

With edits 1-5 applied and **6 and 7 deliberately omitted**, the package suite reported
`399 tests run: 394 passed, 5 failed` — and the 5 are `field_census`, `map_pdf_conformance`,
`year_record` x2 and `supported_years_cross_product`, i.e. **exactly the failures the finished port still
has to fix in files I do not own**. Not one red mentions `ALL`. The old round-trip test passes because it
iterates `ALL` itself and the hardcoded `38` still matched the list it was counting.

### AFTER — 1 edit

Same plant, fixed tree. The build stops once:

```
error[E0004]: non-exhaustive patterns: `LineSet::F8995a_2025` not covered
   --> crates/btctax-forms/src/line_set.rs:141:11
    |           ^^ pattern `LineSet::F8995a_2025` not covered
   --> /scratch/.../out/line_set_generated.rs:10:10
error: could not compile `btctax-forms` (lib) due to 1 previous error
```

`line_set.rs:141` is `schema()`. **`f6251_revision.rs` does not appear.** Adding the one arm
(`LineSet::F8995a_2025 => Schema::Form8995AMap`) gives `401 tests run: 396 passed, 5 failed` — the same
5 non-mine failures as before, no more.

**Extrapolated to a 17-form year**, the rehearsal's own axis: 7 x 17 = 119 edits in these three files
-> **17**. And the rehearsal's 13-edit total for one `(stem, year)` becomes **7**.

## 3. FR-146 — closed, and given a real kill

The defect: the doc line for `"f8959/2024"` sat above `F8889_2024`, leaving `F8959_2024` undocumented;
the 2025 block repeated it. Generated now, from the same string as the variant:

```
    /// `"f8889/2024"` — the revision transcribed by `forms/2024/f8889.map.toml`.
    F8889_2024,
    /// `"f8959/2024"` — the revision transcribed by `forms/2024/f8959.map.toml`.
    F8959_2024,
    /// `"f8889/2025"` — the revision transcribed by `forms/2025/f8889.map.toml`.
    F8889_2025,
    /// `"f8959/2025"` — the revision transcribed by `forms/2025/f8959.map.toml`.
    F8959_2025,
```

**Is a misplaced doc comment machine-detectable? In hand-written source, no** — it is invisible to the
compiler and to every test, which is exactly why FR-146 survived to be found by a human reading the
file, and I am not going to dress a grep up as a checker. What *is* detectable is a **generator** that
emits the doc line and the variant out of step, and that check now exists and has been watched red
(section 4.1). The honest statement: generation eliminated the class; the test guards the generator that
eliminated it.

## 4. The kills — red pasted, then green

### 4.1 FR-146's kill — the generator emits the previous revision's doc line

Plant (in `build.rs`): `let (ls, maps) = { let k = if i == 0 { 0 } else { i - 1 }; … }`.

```
thread 'line_set::tests::every_generated_doc_comment_names_its_own_row' panicked at src/line_set.rs:331:13:
assertion `left == right` failed: the doc comment above `F1040s1_2024` describes `f1040/2024` — the FR-146 slip, in the generator this time
  left: "f1040/2024"
 right: "f1040s1/2024"
```

…and the attribution test reds beside it:

```
thread 'line_set::tests::every_bundled_map_names_the_revision_the_generator_attributed_to_it' panicked at src/line_set.rs:285:13:
`forms/2024/f1040s1.map.toml` names revision `f1040s1/2024`, but the generator recorded that revision as
"`forms/2024/f1040.map.toml`" — the line scan and the TOML parse are reading different files, or the same
file differently. Note the SET can still match: this is what a set comparison alone cannot see.
```

Plant removed: `4 tests run: 4 passed`.

### 4.2 The near miss that shows the attribution check is not a set comparison

Plant (in `line_set_of`): **swap** the two Form 8949 revisions — `2024/f8949.map.toml -> "f8949/2025"` and
`2025/f8949.map.toml -> "f8949/2024"`. The variant **set is unchanged**, so `schema()`'s exhaustive match
is silent, the round-trip test passes, and `every_generated_doc_comment_names_its_own_row` passes. Only
the per-map join reds:

```
Summary: 4 tests run: 3 passed, 1 failed
thread 'line_set::tests::every_bundled_map_names_the_revision_the_generator_attributed_to_it' panicked at src/line_set.rs:285:13:
`forms/2024/f8949.map.toml` names revision `f8949/2024`, but the generator recorded that revision as
"`forms/2025/f8949.map.toml`" — …
```

That is why the test joins per map rather than comparing sets: a set comparison here was green.

### 4.3 The exhaustiveness property, both halves

**(a) A new revision with no `schema()` arm still fails to compile** — section 2 AFTER, `E0004` at
`line_set.rs:141`, and nothing in `f6251_revision.rs`. Both halves of the brief's kill in one run.

**(b) A Form 6251 revision on this schema with no cells still fails to compile.** Plant:
`LineSet::F6251_2024 => Schema::Form6251ObbbaMap`.

```
error[E0080]: evaluation panicked: a revision is wired to Form6251ObbbaMap and states none of its own
year-varying cells: it would inherit another revision's Schedule 1-A cross-reference, and the AMT base
would be wrong with every other instrument green. Add its arm to `f6251_revision::revision` —
`tests::every_obbba_revision_has_cells` names which one.
   --> crates/btctax-forms/src/f6251_revision.rs:325:13
    | |_____________^ evaluation of `f6251_revision::_` failed here
error: could not compile `btctax-forms` (lib) due to 1 previous error
```

Plant removed, build green.

### 4.4 `build.rs`'s four new refusals, each watched red

Planted on a temporary `forms/2025/f8275.map.toml` + `.pdf` pair (both removed afterwards):

| plant | build error |
|---|---|
| no `line_set` in the header | `build.rs:103: .../2025/f8275.map.toml: the header carries no line_set = "<stem>/<year>". Every map names the LINE-SET REVISION it transcribes (design r2 §4); without it that revision is absent from LineSet, and every gate that walks LineSet::ALL skips this map in silence.` |
| `line_set` twice | `build.rs:109: ... line_set is assigned 2 times in the header — one map transcribes one revision, and two assignments make the generated variant whichever the scan saw last` |
| `line_set = "f8275-2025"` | `build.rs:123: ... line_set = "f8275-2025" is not <stem>/<year>` |
| two rows spelling one variant (`"f_8949/2025"` beside `"f8949/2025"`) | `build.rs:248: line_set = "f_8949/2025" (forms/2025/f8275.map.toml) and line_set = "f8949/2025" (forms/2025/f8949.map.toml) both spell the variant F8949_2025 — two revisions the generated enum cannot tell apart` |

## 5. Premises refuted

1. **FR-141 / rehearsal F10 says "6 hand-edits per `(stem, year)`, 5 in `line_set.rs`, 1 in
   `f6251_revision.rs`" and "from 6 to 1". Measured: it is 7 in these files**, and the seventh is the
   rehearsal's own edit #7 (`LineSet::ALL.len()` 38 -> 39), which it counted in a different class
   ("hardcoded counts") although it recurs on every port and lives in this file. 7 -> 1, not 6 -> 1.
2. **The rehearsal's edit table attributes edit 2 (`parse`) and edit 4 (`ALL`) to "`map_rows` red".
   `tests/map_rows.rs` never mentions `LineSet`** (grepped: zero references) and never resolves a row's
   revision. What actually reds is
   `supported_years_cross_product::the_cross_product_matrix_matches_the_recorded_gaps`
   (*"missing {"dispatch"}"*) and
   `field_census::every_emittable_form_is_reached_by_the_gate_or_named_absent`.
3. **And `ALL` was forced by nothing at all** (section 2). This is the part of FR-141 that was under-read:
   not a redundant keystroke but a **silent coverage hole**, since `ALL` is the iterand of every gate over
   the revision set. It is the `CLAUDE.md` "typed list beside a set that grows" shape *inside the
   instrument that was supposed to catch it*.
4. **My own first stem choice was refuted by measurement.** I began with `f8275/2025` and it forced
   *nothing* in `line_set.rs`: Form 8275 is periodic and aliased, so its TY2025 dispatch never reads a
   TY2025 row. The BEFORE count is therefore stem-dependent, and I re-measured with the rehearsal's own
   `f8995a/2025` for comparability. A port of an aliased stem is cheaper than 7, and the cost is not
   uniform across stems.

## 6. What I could not do, for want of a file I do not own

1. **A compile-fail harness for the `E0080` guard** (`trybuild`-style) needs `[dev-dependencies]` in
   `crates/btctax-forms/Cargo.toml` — **not mine**. So the const guard's automated standing instruments
   are the two runtime tests (`f6251_revision::tests::every_obbba_revision_has_cells` and
   `tests/f6251_obbba.rs`), and the build error itself was observed manually (section 4.3b) — the same
   posture the `_`-free match had before this change (design section 10 step 5: *"`E0004` observed on a
   planted stub 2026 revision"*), not a weakening of it. **Candidate follow-up.**
2. **The remaining per-`(stem, year)` hand-edits are all outside my three files**, and two are outside the
   whole partition: `tests/year_record.rs::expected_count` (A5), `crates/xtask/src/cite_check.rs`'s
   map-row count and `AUTHORITY_NOT_YET_ARCHIVED` (A3), and — **unowned by any agent in
   `PLAN-fr134-151-burndown.md`** — `tests/supported_years_cross_product.rs::BUNDLED_FORMS_PER_YEAR` and
   `tests/map_pdf_conformance.rs`'s 8995-A refusal loop. Every one of them is the same shape as what I
   removed here (a count or an absence-assertion beside a glob), and they are what stops the port from
   being *only* the `schema()` arm.
3. **`design/FORM_AUTHORITY_TABLE_DESIGN.md` section 9 ("What this hand-list becomes") and section 10
   step 6 still describe `LineSet` as hand-written.** That file is **S4's**. One sentence there wants
   updating.

## 7. Gate

Run on the final tree, `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-a2` (clippy in
`target-a2-clippy`), after `find crates -name '*.rs' -exec touch {} +`:

| gate | result |
|---|---|
| `cargo nextest run --workspace --no-fail-fast` | **3616 run: 3609 passed, 7 failed, 12 skipped** |
| `cargo nextest run -p btctax-forms --no-fail-fast` | **401 run: 401 passed, 4 skipped** (399 before; +2 new tests) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **exit 0** |
| `cargo fmt --all -- --check` | **clean** |

**The 7 workspace failures are not mine, measured rather than asserted.** I restored all three files from
their pristine copies and re-ran exactly those tests: **the same 7 fail**, identical names
(`form_delta::tests::{every_common_field_is_either_compared_or_named_as_unwitnessed,
no_archived_pair_reports_a_clean_verdict_from_zero_comparisons, port_status_prints_the_committed_work_list,
the_committed_work_list_matches_form_delta_at_head, the_ty2026_drafts_are_ready_to_be_diffed_against_their_finals,
the_work_list_checker_reds_on_every_planted_row}` and
`harness_check::tests::the_write_hook_denies_new_archives_and_asks_once_per_new_directory`). Mechanisms,
from the failures' own text:

- 6 x `form_delta`: *"no PDF found for f6251--2026-DRAFT"* — `design/forms/**/*.pdf` is ignored by version
  control (`.gitignore:63`); an isolated worktree holds the `.pdf.txt` notes and none of the PDFs. This is
  the burndown plan's worktree caveat, and the same class as **FR-147**.
- 1 x `harness_check`: *"cargo build -p xtask succeeded but left no binary where on-write.sh looks — if
  the target dir moved, the HOOK's lookup needs updating too"* — **FR-147** verbatim (S3 owns it).

One post-gate edit exists: a stale sentence in `line_set.rs`'s module header (it still called the
`f6251_revision` trap an `_`-free match) was rewritten. Comment-only; `cargo fmt --all -- --check` clean
and `-p btctax-forms` re-run green afterwards (401/401).

## 8. Scope notes for the coordinator

- **FR-141 and FR-146 are both closed**, with kills, in three files, no product behaviour change.
- The integration ask is the diff of those three files plus one re-plant of section 4.3a (add
  `forms/2025/f8995a.{pdf,map.toml}` from the 2024 pair with `year`/`line_set` rewritten, build, expect
  the single `E0004` at `line_set.rs:141` and nothing in `f6251_revision.rs`, then delete both files).
- `FOLLOWUPS.md` untouched, per the plan.
