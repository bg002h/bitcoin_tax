# Phase review — design r2 step 1 (6267b6b1)

Reviewer: claude-opus-5[1m], independent, read-only. Date: 2026-09-05. HEAD: `f2a4b8387a588b8b2994957412c5e92100edc02d`.

**Tree caveat, stated up front.** HEAD is `f2a4b838` (the docs commit on top of `6267b6b1`) and the
working tree carries **uncommitted step-2 WIP**: `M crates/btctax-forms/src/{lib,map,pdf}.rs`,
`M crates/xtask/src/main.rs`, and untracked `crates/btctax-forms/build.rs`,
`crates/btctax-forms/src/bundled.rs`, `crates/xtask/src/package_check.rs`. I confirmed the artifacts
under review are unaffected: `git diff 6267b6b1 -- crates/btctax-forms/tests/map_rows.rs
crates/xtask/src/cite_check.rs crates/btctax-forms/forms | wc -l` → **0**. The WIP additions to
`map.rs` are a `#[cfg(test)] pub mod testonly_old_consts` only. Test runs below therefore exercise the
step-1 code as committed, with the step-2 `build.rs` present in the package.

Read: `git show 6267b6b1 --stat` and the full diffs for `crates/btctax-forms/src/map.rs`,
`crates/btctax-forms/src/packet.rs`, `crates/btctax-forms/src/lib.rs`,
`crates/btctax-forms/tests/map_rows.rs`, `crates/xtask/src/cite_check.rs`;
`design/FORM_AUTHORITY_TABLE_DESIGN.md` §4, §5, §8, §9, §10, §11;
`crates/btctax-forms/forms/{2017/f8283,2025/f1040s1a,2024/f8275}.map.toml` plus the header block of
all 37; `crates/xtask/src/authority_manifest.rs` (`is_draft`/`is_authority`);
`crates/xtask/src/cite_check.rs` (`STEM_ALIASES`, `irs_basename`, `emitted_form_years`, `FORMS`);
`crates/btctax-forms/tests/full_return_forms.rs` (the two ordering tests);
`crates/btctax-cli/src/cmd/admin.rs` (the `{seq}_{name}.pdf` prefix + `manifest.txt`);
`crates/btctax-adapters/src/tax_tables.rs` (`BundledFullReturnTables::load`).

Ran:
- `sha256sum` over all 37 `forms/<year>/<stem>.pdf` vs each row's `template_sha256`.
- `grep -oE 'Sequence No\. *[0-9]+[A-Z]?'` over `design/forms/extract/<irs_stem>--<year>.txt` for the
  32 rows that have an extract; `pdftotext -layout -f 1 -l 1` over the five TY2017 bundled PDFs for
  the rows that do not.
- `pdftotext -layout -f 1 -l 1 | grep -oiE '\(Rev\.[^)]*\)'` over **all 37** bundled PDFs, to find
  which templates print a revision date at all (not just the four the rows claim).
- `.venv/bin/python` join of the 37 template hashes against `design/forms/MANIFEST.json`, filtering
  drafts by `path contains "-DRAFT" || url contains "/irs-dft/"`.
- Independent Python re-derivation of `{(irs_stem, year)}` from the map glob vs `{(irs_basename(stem),
  year)}` from the PDF glob.
- Existence check of `design/forms/<year>/<instructions>--<year>.pdf` and
  `design/forms/extract/<instructions>--<year>.txt` for all 37 rows.
- `cargo nextest run -p btctax-forms -E 'binary(map_rows)'` → **7 passed**.
- `cargo nextest run -p xtask -E 'test(map_row_tests)'` → **3 passed**.
- `cargo nextest run -p btctax-forms` → **318 passed, 4 skipped**.
- `grep -c` field/struct/test counts in `map.rs`, `packet.rs`, `map_rows.rs`, `cite_check.rs`.

Not re-derived (settled, per the brief): the design itself (glob + header + `build.rs`; `line_set` =
revision; the six excused rows; `line_set = "<stem>/<year>"` at step 1); the owner rulings; that
steps 2/3 are later work.

## Verdict: 0 Critical / 2 Important / 6 Minor

## Row-value audit (37 rows)

Every field machine-checked against its own source; nothing eyeballed. **No wrong value found — 37/37
rows are correct on all nine fields.**

| field | how checked | result |
|---|---|---|
| `form`, `year` | vs the file's stem and its directory | 37/37 (also asserted in-test) |
| `template_sha256` | `sha256sum` of the `.pdf` beside each map | **37/37 exact** |
| `attachment_sequence` | 32 rows vs `Sequence No. …` in `design/forms/extract/<irs_stem>--<year>.txt`; 5 TY2017 rows vs `pdftotext` of the bundled template itself | **36/36 present values exact**; absent on exactly the three `f1040` rows, and those three sources print no sequence number |
| `versioning` | `pdftotext` over **all 37** templates for `(Rev. …)` | exactly 4 print one — 2017/f8283 `December 2014`, 2024/f8275 `October 2024`, 2024/f8283 `December 2023`, 2025/f8283 `December 2025` — and exactly those 4 rows carry `periodic`, with the matching short form. The other 33 are `"annual"` |
| `authority` | content join of all 37 hashes vs `MANIFEST.json` (120 authority / 15 draft entries of 135) | **exactly 6** templates fail the join — 2017/{f1040,f8283,f8949,schedule_d,schedule_se} and 2024/f8283, all *absent entirely* (none matches a draft entry) — and exactly those 6 rows carry `authority`, each prefixed `not-yet-archived: ` |
| `instructions` | `design/forms/<year>/<ins>--<year>.pdf` in `MANIFEST.json` + `extract/<ins>--<year>.txt` on disk | all 32 archived-year rows resolve to a real archived instructions PDF **and** extract; the 5 TY2017 rows resolve to nothing, consistent with their `authority` excuse. Aliases as briefed: `f1040sa→i1040sca`, `f1040`/`f1040s1`/`f1040s1a`/`f1040s2`/`f1040s3`→`i1040gi`, `schedule_d→i1040sd`, `schedule_se→i1040sse` |
| `instr_pages` | vs `cite_check::FORMS` | present on 2025/f1040s1a only, `[101, 110]` == `FORMS[0].instr_pages == Some((101,110))` |
| `extract_override` | file existence + design §9's stated size | present on 2025/f1040s1a only; `crates/btctax-core/src/tax/fixtures/schedule_1a_2025_form.txt` exists, 11,443 B — the figure §9 cites |
| `line_set` | vs `<form>/<year>` | 37/37 (also asserted in-test) |
| `irs_stem` | independent alias re-derivation | 37/37; `{(irs_stem, year)}` == `{(irs_basename(stem), year)}` from the PDF glob, **both directions, 37 == 37, no asymmetry** |

Structural checks: `map.rs` carries the nine row fields on **18** definitions (17 top-level `*Map`
structs + `MapRow`; the 19 `pub struct …Map` hits minus `PartMap` and `ScheduleBRowMap`), and **none of
the five required fields carries `#[serde(default)]` anywhere** (`grep -B1` over the required-field
declarations → 0 `serde(default)` hits), so a missing key is a hard refusal on all 18.

## Kill audit (4 + the two-way test)

| # | kill | can it fail? | evidence |
|---|---|---|---|
| 1 | missing required key → refusal | **YES** | `a_map_missing_a_required_row_key_is_refused` strips the single `line_set` line and asserts both `Form8959Map::parse` and `MapRow::read` err; `line_set` is required on all 18 structs (verified above). Also asserts a `line_sett` typo is refused by `deny_unknown_fields` |
| 2 | `template_sha256` ≠ the PDF | **YES** | `a_planted_hash_mismatch_is_reported` calls the *same* `check_rows` the green test calls, on a tempdir copy with the hash zeroed, and asserts the specific `HashMismatch` variant. Deleting the check reds it |
| 3 | manifest join, excused only by `authority` | **YES** | `a_planted_missing_excuse_is_reported` strips `authority` from 2024/f8283 and asserts `NotInManifest`. I independently confirmed that row's hash really is absent from the manifest, so the plant is load-bearing rather than incidental; stripping `authority` also un-skips kill 4 there and produces no second problem (155 == the extract), so the assertion is unambiguous |
| 4 | `attachment_sequence` ≠ the printed number | **YES on 31 of 37 rows, NO on 6** | `a_planted_wrong_sequence_number_is_reported` plants 155 on 2025/f8283 and asserts the exact `SequenceMismatch`. But `check_rows` guards the whole kill with `if row.authority.is_none()`, so it never runs on the six excused rows — see **P1** |
| — | packet ↔ row (`packet_sequences_agree_with_every_map_row`) | **YES** | it is what found the 8283 155→36 defect; it walks the glob, so a new bundled year cannot skip it |
| — | two-way row-set vs emitting surface (xtask) | **the assertion holds; its "kill" does not witness it** | `a_row_pointing_at_no_template_is_caught` re-derives the set difference inline rather than invoking the checked assertion — see **P4** |

`manifest_authority_hashes` reads the right shape: `MANIFEST.json` is a **top-level array of 135
objects** with keys `bytes, extract, kind, path, sha256, storage, url`; the reader's
`v.get("entries")` misses, `.or_else(|| v.as_array())` hits, and the draft filter reproduces
`Entry::is_draft()` exactly. My independent Python join over that shape returns the same 120 authority
hashes and the same 6 misses.

## Findings

### P1 — IMPORTANT — kill 4 is disabled on 6 of 37 rows, because the *manifest* excuse is being used as the *extract* excuse

**Where:** `crates/btctax-forms/tests/map_rows.rs:184-201` (inside `check_rows`).

```rust
// kill 4 — the printed sequence number, from the archived extract.
let extract = extract_root.join(format!("{}--{}.txt", row.irs_stem, year));
if row.authority.is_none() {
    let printed = std::fs::read_to_string(&extract).ok().and_then(|t| printed_sequence(&t));
    if printed != row.attachment_sequence { problems.push(RowProblem::SequenceMismatch { … }); }
}
```

**What is wrong:** design r2 §4 and §10 step 1 give kill 4 exactly one exception — *"except on a form
the IRS prints no sequence number for (the 1040)"* (F8). The implementation instead exempts every row
carrying `authority`, which is a different predicate for a different kill (the manifest join). The two
sets coincide only by accident. Consequences:

- The five TY2017 sequence values (`f8283` 155, `f8949` 12A, `schedule_d` 12, `schedule_se` 17) are
  checked against nothing — yet their bundled templates **do** carry text layers that print the
  number, which is how I verified them (`pdftotext -layout -f 1 -l 1`). The mechanism the excuse is
  standing in for is "no extract exists", and it is not the same as "no manifest entry exists".
- Worse, `forms/2024/f8283` is excused even though `design/forms/extract/f8283--2024.txt` exists and
  prints `Sequence No. 155`. Its value is unchecked against the form. What holds it today is only
  `packet_sequences_agree_with_every_map_row` — i.e. a hand-written row agreeing with a hand-written
  literal in `packet.rs`. That is precisely the shape that produced the TY2025 8283 defect this commit
  exists to fix, still standing on the same form one year earlier.
- This is the repo's own recorded anti-pattern: *"an excuse list keyed by VECTOR NAME is a liability
  … state the mechanism, let it decide, never enumerate the outcomes you happened to see."* Here the
  excuse is keyed by a **different check's** excuse field.

The blindness is invisible: all 37 rows pass, and `a_planted_wrong_sequence_number_is_reported` plants
on 2025/f8283, an **unexcused** row, so it cannot expose the hole. Planting 999 on
`forms/2024/f8283.map.toml` produces no `SequenceMismatch`.

**Minimal change:** key the kill to its own mechanism instead of `authority`. Read the printed number
from the extract when one exists, else from the bundled template's own text layer, and refuse only
when neither yields anything:

```rust
let printed = std::fs::read_to_string(&extract).ok().and_then(|t| printed_sequence(&t))
    .or_else(|| printed_sequence_from_pdf(&map.with_extension("").with_extension("pdf")));
if printed != row.attachment_sequence { problems.push(SequenceMismatch { … }); }
```

If shelling out to `pdftotext` in a unit test is unwanted, the smaller correct fix is to gate on
*extract existence* rather than on `authority` — that alone restores the kill on 2024/f8283 and
records the five TY2017 rows as an explicit, separately-named gap instead of laundering them through
the manifest excuse. Either way, the plant must move to an excused row so the exemption itself is
witnessed.

### P2 — IMPORTANT — the 8283 renumber fixed the *number* but not the *push position*; the TY2025 packet is emitted out of sequence order and nothing can red

**Where:** `crates/btctax-forms/src/packet.rs:98-274` (`fill_full_return`'s fixed push order; `f8283`
is pushed last, after `f8275`), read together with `crates/btctax-cli/src/cmd/admin.rs:1105-1123` and
`crates/btctax-forms/tests/full_return_forms.rs:3159-3204, 3214-3246`.

**What is wrong:** `fill_full_return` has no sort — the packet's order **is** the literal push order,
which is hard-coded ascending for TY2024: `None, 01, 02, 03, 07, 08, 09, 12, 12A, 17, 32, 55, 55A, 71,
72, 92, 155`. Making 8283 year-aware changes its number to **36** for year ≥ 2025 but leaves it pushed
last, so a TY2025 packet comes back `… f8275(92), f8283(36)` — descending at the tail.
`admin.rs` then writes `manifest.txt` in packet order and labels it the filer's stapling order, while
naming the file `36_f8283.pdf`. The manifest would contradict the prefixes it prints.

This is the identical defect shape the suite already documents as ★★★ at
`full_return_forms.rs:3204-3210` — *"the emitter shipped with the `f6251` block written after the 8995
blocks … Every ordering test passed; the filer would have been handed a `manifest.txt` labelled '←
your stapling order' that was not in order."* The commit message states the defect is "Fixed"; half of
it is.

**Nothing would catch it.** Both ordering tests are TY2024-only and compare against hand-written
name lists / hand-picked positions (`the_packet_emits_every_required_form_in_attachment_sequence_order`
asserts a literal `vec![…]`; `the_amt_packet_staples_form_6251_at_sequence_32_before_form_8995a`
compares three positions). There is no derived monotonicity assertion anywhere — `grep` for
`sort`/`is_sorted`/`windows(2)` in `packet.rs` and `admin.rs` returns only a doc comment.

**Latent, not live.** `BundledFullReturnTables::load()` inserts **2024 only**
(`crates/btctax-adapters/src/tax_tables.rs:101`), so `full_return_for(2025)` is `None` and a TY2025
packet is not producible today; ten TY2025 maps are also still unwired. That is why this is Important
and not Critical — but it becomes wrong output on the same day TY2025 params land, which is the
project's declared next blocker.

**Minimal change:** derive the order instead of hand-maintaining it — stable-sort `out` by
`attachment_sequence` before returning (`None` first). For TY2024 this is a **no-op** (the push order
is already ascending), so the goldens do not move. Two details the comparator must settle explicitly,
because a naive string sort gets both wrong: `12` vs `12A` and `55` vs `55A` (fine on `str`), and
`1A` (Schedule 1-A) which string-sorts *after* `12`, not after `01`. If sorting is judged out of scope
for step 1, then at minimum add the derived check — assert the emitted sequences are non-decreasing —
and run it for **every bundled year**, not 2024 alone; that test reds on this defect today.

### P3 — MINOR — `manifest_authority_hashes` reimplements `Entry::is_authority()` instead of calling it

**Where:** `crates/btctax-forms/tests/map_rows.rs:79-97` vs
`crates/xtask/src/authority_manifest.rs:129-137`.

**What is wrong:** the draft predicate is duplicated as a string filter. It is byte-correct today (I
verified both produce the same 120 hashes), and `btctax-forms` cannot depend on `xtask`, so the
duplication is largely forced. But `is_draft` is documented as *"derived from BOTH signals … because
either alone has been wrong"* — if a third signal is added, this copy silently keeps the old
definition and the join quietly loosens.

**Minimal change:** an assertion in `xtask` that `Entry::is_authority()` and the string filter agree
over every manifest entry — one test, and it reds the day the predicate grows.

### P4 — MINOR — the xtask two-way "kill" re-derives the check rather than exercising it (B1 shape)

**Where:** `crates/xtask/src/cite_check.rs:1341-1357`
(`a_row_pointing_at_no_template_is_caught`).

**What is wrong:** the test rebuilds `from_rows`, inserts `("f9999", 2024)`, and asserts
`from_rows.difference(&emitted).count() == 1`. It never calls the assertion it is supposed to witness.
Delete or weaken the real check in
`the_row_set_equals_the_emitting_surface_both_ways_through_irs_stem` and this test still passes — the
B1 question *"which test reds when this checker is removed?"* has no answer here. `map_rows.rs` got
this right by factoring `check_rows` and having every plant call it; this module did not.

**Minimal change:** factor the two-way comparison into a `fn two_way_diff(rows, emitted) -> (Vec, Vec)`
that both the green test and the plant call, the same way `check_rows` is shared.

### P5 — MINOR — `AnnualTag`'s stated guarantee has no test

**Where:** `crates/btctax-forms/src/map.rs` — *"A separate enum so that a typo (`"anual"`) is a parse
refusal rather than a silently-accepted free string."*

**What is wrong:** the claim is true by construction (untagged `Versioning`: `"anual"` matches neither
`AnnualTag` nor the `Periodic` struct shape), but `grep -rn 'anual'` over `crates/` finds no test, and
the suite's only `Versioning` assertions are the positive ones in `map_rows.rs:270-280`. A guarantee
with no test that reds when it is removed does not exist. Related: the untagged `Periodic` variant does
not `deny_unknown_fields`, so `{ periodic = "…", typo = 1 }` parses.

**Minimal change:** two one-line asserts —
`MapRow::read(&text.replace("\"annual\"", "\"anual\"")).is_err()` and the same for a mistyped
`periodic` key.

### P6 — MINOR — the row's `instructions` / `instr_pages` now duplicate `cite_check::FORMS` with nothing tying them together, and `instructions` has no kill at all

**Where:** `crates/xtask/src/cite_check.rs:740-746` (`FORMS`, `instructions: "i1040gi"`,
`instr_pages: Some((101,110))`) vs `crates/btctax-forms/forms/2025/f1040s1a.map.toml`.

**What is wrong:** two copies of the same fact until step 3 retires `FORMS` (design §9). They agree
today (verified). Separately, none of the 37 `instructions` values is checked by anything — step 1's
kill list does not include one, so the field is inert data. All 32 archived-year values are correct (I
resolved each against `MANIFEST.json` and the extract dir), which also means the check is free.

**Minimal change:** in the xtask module, assert that for every row with an archived year,
`design/forms/<year>/<instructions>--<year>.pdf` is a manifest entry; and that the `f1040s1a/2025`
row's `instructions`/`instr_pages` equal `FORMS[0]`'s, so the duplicate cannot drift before it is
deleted.

### P7 — MINOR — design §4's `instructions = ""` example is stale, and the branch is now unexercised

**Where:** `design/FORM_AUTHORITY_TABLE_DESIGN.md` §4 — *`instructions = "i6251" # "" only for a
self-instructing form (f8275)`* — vs `forms/2024/f8275.map.toml`, which carries `instructions =
"i8275"`.

**What is wrong:** the row is **right** and the design comment is wrong: the IRS does publish separate
Instructions for Form 8275, and this repo archives them
(`design/forms/2024/i8275--2024.pdf` is a manifest entry, `design/forms/extract/i8275--2024.txt`
exists). `FormAuthority`'s own doc carries the same stale claim
(`cite_check.rs:729-731`, *"Form 8275 is one"*). Net effect: no row uses `""`, so that branch is now
dead until some genuinely self-instructing form appears.

**Minimal change:** correct the two doc comments; if `""` is to stay legal, say which form is expected
to use it, otherwise drop the branch when `FORMS` retires at step 3.

### P8 — NIT — the commit message's test count is hand-written and wrong

`6267b6b1`'s message says *"13/13"*. `grep -c '^#\[test\]'` gives **7** in `tests/map_rows.rs` and
**3** in `cite_check.rs`'s `map_row_tests` — 10, which is exactly what nextest ran (7 + 3, both green).
Never hand-count what a tool can count.

## Consumption check (step 1 must consume nothing)

**Clean.** No production path reads a row field. `grep` for
`.irs_stem|.line_set|.template_sha256|.versioning|.instr_pages|.extract_override|MapRow|Versioning::|AnnualTag`
across `crates/*/src/` returns only the `lib.rs` re-export line and `cite_check.rs`'s `#[cfg(test)]
mod map_row_tests`. (`cite_check.rs:586`'s `f.instr_pages` is the pre-existing `FormAuthority` const,
not `MapRow`.) The 17 `*Map` structs now *require* the keys to parse, which is design §4's intended
refusal, not consumption. `packet::attachment_sequence` is production and is called by production, but
it is a literal table held to the rows by a test — it reads no row.

`packet.rs`: **17 push sites, 17 calls to `attachment_sequence` (18 occurrences = 17 + the
definition), and zero remaining `Some("…")` literals outside the function body.** The function also
carries an `f1040s1a => "1A"` arm with no push site yet, which is what the 2025 row needs.

**TY2026 8283 (brief Q3):** `year >= 2025 ⇒ "36"` is correct for the bundled TY2025 template (`Form
8283 (Rev. 12-2025)`, printed `Sequence No. 36` — verified against both the PDF and
`extract/f8283--2025.txt`). A TY2026 8283 fails **closed**: dropping `forms/2026/` files creates rows,
`emitted_form_years()` refuses a PDF without a map, and `packet_sequences_agree_with_every_map_row`
walks the glob — so if TY2026 ships Rev. 12-2025 again the literal stays right, and if a new revision
renumbers, the row (read off the new extract) disagrees with `"36"` and reds. The residual is only
that a *year* threshold encodes a *revision* property, which the function's own doc admits and design
§9 retires at step 3. The `_ => None` fallback likewise fails closed for a bundled stem (its row
carries `Some`, the packet says `None`, the test reds).

## Step 2 may proceed?

**NO — but only just.** Neither Important touches step 2's surface (`build.rs`, `template`/`map_text`,
the `cargo package --list` gate), so nothing here invalidates work already done; the gate is 0C/0I and
both are small, local folds. P1 is ~5 lines in `check_rows` plus moving one plant to an excused row;
P2 is a stable sort (a no-op for TY2024, goldens unmoved) or a derived non-decreasing assertion run
per bundled year. Fold both, re-run `-E 'binary(map_rows)'` and the forms suite, and step 1 is green.
