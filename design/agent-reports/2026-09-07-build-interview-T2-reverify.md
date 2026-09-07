# Re-verification — interview T2 build fold (kills A–I, review findings, negative claims)

Independent VERIFIER, sonnet, own worktree. Read-only for the record: every plant below was made in
this worktree with the Edit tool, observed, and reverted with `git checkout --` (confirmed clean —
`git status --short` empty — after every plant and at the end). No commits, no subagents.

**Worktree note.** The dispatch named the worktree HEAD as `b6609751` and the fold under verification
as `0c942ac2`, but this worktree's checked-out HEAD was actually `2bd04d45` (`fold(FR-45)`, 244 commits
behind `main` — an unrelated, later lineage; `main..2bd04d45` and `main..0c942ac2` do not intersect
until `main` itself). `0c942ac2` and `b6609751` exist in the shared object store, so `git checkout
--detach 0c942ac2` reached the correct commit with no fetch needed. Everything below runs at `0c942ac2`,
working tree clean before the first plant.

Environment reconfirmed before planting anything (matches the brief's settled facts exactly):
`box-census` → 246 printed boxes / 15 editions / 7 information returns / 123 entries; `authority-manifest`
→ 170 entries (form 71, guidance 22, instructions 46, publication 6, regulation 6, statute 19);
`line-coverage` → 341 / 24 / 0 / 12; `cargo nextest run --locked -p xtask --no-fail-fast` → 144 run: 137
passed, **7 failed** (6 `form_delta` + 1 `harness_check`, all environment per the brief, reproduced
identically below).

## Kills A–I (fold report §9) — each re-planted, run, reverted

| # | plant made | command | RED text | verdict |
|---|---|---|---|---|
| A | `revision_in_force`'s filter replaced with `\|(_, _rev, _cadence)\| true` (ignores the tax year) | `cargo nextest run -p xtask -E 'test(revision_in_force_pins_the_whole_table)'` | `assertion `left == right` failed: fw2 @ TY2024`<br>`  left: Some("2026")`<br>` right: Some("2024")` | **HOLDS** — exact match to fold §9 row A |
| B | the `f1098e--2025` `DocumentAuthority` entry deleted (only that edition) | `cargo run -p xtask -- box-census` | `the censused document set is not the archive:`<br>`  f1098e--2025 is archived as a form but nothing censuses it — an archived information return outside the census is invisible to every box assertion` | **HOLDS** — exact match |
| C | the `FilerRecords` arm gutted to `if normalize(&text).contains(&wanted) { return Vec::new(); }` (block/line binding removed) | `cargo nextest run -p xtask -E 'test(each_rule_rejects_a_table_that_violates_it)'` | `planting a FilerRecords sentence from another LINE of the right booklet must RED, and it did not` | **HOLDS** — exact match. (The (7d) AMTFTC plant still reds correctly first, for the right reason, confirming the booklet-to-form binding survived the plant and only the line binding was gutted.) |
| D | the `DocBox` edition resolution replaced with `DOCUMENTS.iter().filter(\|d\| d.stem==stem).map(\|d\| d.edition).max()` (newest archived, ignoring the row's year) | `cargo nextest run -p xtask -E 'test(each_rule_rejects_a_table_that_violates_it)'` | `planting a DocBox naming a box only a LATER edition prints must RED, and it did not` | **HOLDS** — exact match |
| E | `authority_refresh::compare`'s verdict branch replaced with `Ok(_bytes) => Verdict::Same` unconditionally | `cargo nextest run -p xtask -E 'test(a_revised_document_reds_and_an_unchanged_one_does_not)'` | `a revised document must RED, and it did not: Same` | **HOLDS** — exact match |
| F | `fw2--2026`'s `preamble_end: PREAMBLE_W2_2026` replaced with `PREAMBLE_1141` | `cargo nextest run -p xtask -E 'test(the_enumerator_markers_are_present_in_every_extract)'` AND `cargo run -p xtask -- box-census` | test: ``assertion `left == right` failed: fw2--2026: its recorded preamble marker "1141, 1167, and 1179" does not occur exactly once; the face block is being guessed`` (`left: 0, right: 1`); instrument: `fw2--2026: the preamble marker "1141, 1167, and 1179" occurs 0 times, expected exactly 1 — the face block cannot be bounded…` | **HOLDS** — both exact match |
| G | the Rule-5 year-stub skip (`if previous.contains(YEAR_STUB) && runs.len()==1 { continue; }`) deleted | `cargo run -p xtask -- box-census` | `f1098--2022: the box numbers this enumerator found are not contiguous: [12, 13, 14, 15, 16, 17, 18, 19] missing from 1..=20. A form does not skip a box number, so this is the READER dropping boxes, not the form omitting them` | **HOLDS** — exact match |
| H | the Rule-6 lettered-run digit check (`if !run.bytes().any(char::is_ascii_digit) { continue; }`) deleted | `cargo run -p xtask -- box-census` | `fw2--2024: the lettered boxes are not contiguous from 'a': found {'a', 'b', 'c', 'd', 'e', 'f', 'o'}, expected {'a', 'b', ..., 'o'} — the reader is dropping boxes` | **HOLDS** — same defect, same document, same found-set as fold §9 row H (fold's quote abbreviates the expected-set listing as `{'a'..'o'}`; the tool prints it enumerated — a presentation difference only) |
| I | the `fw2` box-1 `BoxEntry` split: `editions: &["2024"]` carries the caption with the trailing `n` dropped (`"…compensatio"`), `editions: &["2025","2026"]` keeps the true caption | `cargo run -p xtask -- line-coverage` | 2 problems, both `fw2--2024`: `f1040:1a (Form1040Lines.line1a) is Collected from fw2--2024 box 1, whose census entry quotes "1 Wages, tips, other compensatio", but the extract prints "1 Wages, tips, other compensation" — the box caption is the document's text, never ours (instructions: iw2w3)` (+ the same for `Form1040Income.line1a`) | **HOLDS** — exact match, and confirmed only the two TY2024 rows red; no TY2025 row (Schedule 1-A's box-1 citation) appeared in the 2-problem list, confirming it stayed green |

## Review findings — test that holds each, review's own evidence re-planted

| finding | holding test / instrument | evidence re-planted | result |
|---|---|---|---|
| **C1** (the 1099-G box-10 gap / stale archive) | `box_census::tests::revision_in_force_pins_the_whole_table` (pins `f1099g` → `"2026"` at TY2026) + the `box-census` verdict check (every printed box must carry a `BoxDecision`) | (a) confirmed `f1099g--2026` box 10 is now `BoxDecision::RefuseIfNonzero("T5: paid family leave benefits…")` — `f1099g` box 10, `f1099g--2026 (i1099g instructions): 13 boxes — 2 collected, 1 refuse-if-nonzero, 10 not read`. (b) deleted that one `BoxEntry` and reran `box-census`: `xtask box-census: the box census failed for 1 document(s): f1099g--2026: box 10 ("10 Family leave benefits") is printed on the form and NOTHING decides it — we forgot this box, which is invisible on the page and to every value assertion`. (c) ran the live network instrument once (`authority-refresh --check`): `115 note(s) re-fetched — 115 unchanged, 0 DRIFTED, 0 unreachable`; `probed 14 information-return stem(s) for a newer edition — 0 found`; `OK` | **HOLDS** — the TY2026 editions are archived, box 10 is decided (fails closed, handed to T5 by name as the fold claims), and the archive is confirmed current against live irs.gov today |
| **I1** (`FilerRecords` unbound to line) | `line_coverage_check::tests::each_rule_rejects_a_table_that_violates_it`, source_plant (7d) | the review's literal plant — Schedule A 5b re-pointed at `CollectedFrom::FilerRecords { instruction_line: "The AMTFTC is a credit that you can claim against the AMT." }` — is now **in-suite** and reds unmodified: `f1040sa:5b (line5b) cites "The AMTFTC is a credit that you can claim against the AMT." — it is neither inside a `Line 5b` block of i1040sca--2024 nor a sentence that NAMES line 5b…` (confirmed via kill C's run above, where this plant is (7d) and reds first, for the right reason, before the (7e) plant is reached) | **HOLDS** |
| **I2** (document set is a hand-list) | `box_census::tests::documents_equal_the_archived_information_returns` + `check_document_set` (the `box-census` operator command) | the review's literal plant — the whole `f1098e` `DocumentAuthority` (all 3 editions) **and** its 2 `BoxEntry` rows deleted — reran `cargo nextest run -p xtask -E 'test(box_census)'`: **9 tests run: 7 passed, 2 failed** (`documents_equal_the_archived_information_returns` and, incidentally, `revision_in_force_pins_the_whole_table`), vs. the review's original **6 tests run: 6 passed, 0 failed** on the identical filter before the fold. `box-census` itself: `the censused document set is not the archive: f1098e--2024/2025/2026 is archived as a form but nothing censuses it — an archived information return outside the census is invisible to every box assertion` | **HOLDS** — the exact plant that passed 6/6 before the fold now fails 2/9 |
| **I3** (no round-trip reader) | `authority_refresh::tests::a_revised_document_reds_and_an_unchanged_one_does_not` (the in-suite kill, = fold §9 kill E above) + `xtask authority-refresh --check` (live, network-gated, run once) | kill E above **is** the "mutated note sha256" plant: the pure `compare()` function is given a fetch that returns bytes differing from the recorded sha256/byte-count, and the test asserts `Verdict::Drift` (or panics `"a revised document must RED, and it did not"` if not). Additionally ran the real command against irs.gov: `authority-refresh: 115 note(s) re-fetched — 115 unchanged, 0 DRIFTED, 0 unreachable`; `probed 14 information-return stem(s) for a newer edition — 0 found`; `OK — every note still matches irs.gov, and no newer edition is served` | **HOLDS** — matches the fold's §1 round-trip claim exactly, reproduced independently today |
| **M1** (doc-comment miscount) | doc-only | grepped `crates/xtask/src/box_census.rs` for `"Six of the seven"` — **0 hits**. The `DocumentAuthority` struct doc (lines 160–178) carries no document-count claim at all now; the miscounting field/comment is gone, as the fold states | **HOLDS**, Minor/doc-only as filed |
| **M2** (Form 8995 line 12 retag) | doc-only (no dedicated test; rule (4) still does not gate `Combine` vs `Carry` either way, as both the review and fold state) | confirmed at `crates/btctax-core/src/tax/line_coverage.rs:1100-1106`: `Production::Carry` for f8995 line 12, `"Enter your net capital gain, if any, increased by any qualified dividends"`, with a comment (lines 1095-1099) explaining `Combine`'s blankness rule has no operands here | **HOLDS** as filed (Minor, ungated either way — unchanged risk, correctly not overclaimed) |
| **M3** (unlabelled-box limit + `a..=f`→`a..=z`) | `is_label` widened; safety comes from Rule 6 (kill H's mechanism) | confirmed `crates/xtask/src/box_census.rs:793`: `1 => b[0].is_ascii_digit() \|\| b[0].is_ascii_lowercase()` — full alphabet, not `a..=f`. Confirmed module doc (lines 106-116) now declares **both** limits: the wrapped-caption limit and the FATCA/2nd-TIN unlabelled-box limit by name | **HOLDS** |
| **M4** (per-edition preamble bound) | `box_census::tests::the_enumerator_markers_are_present_in_every_extract` (= fold §9 kill F above) | already re-planted and confirmed RED as kill F | **HOLDS** |

## Negative claims

| claim | check run | result |
|---|---|---|
| nothing pre-existing in `design/forms/` changed | `git diff 1c8a7301..0c942ac2 --stat -- design/forms \| grep -v '^ design/forms/20\(22\|24\|26\)\|extract\|geometry'` | Only `MANIFEST.json` (153 insertions) and `README.md` (22 changed lines) survive the filter. Read the full `README.md` diff directly: every hunk is either a new bullet/paragraph (the new directory-listing lines, the `authority-refresh` blurb, the Form-1098-Wayback-sourcing paragraph) or the single count-line edit `**98**`→`**115**` with its measurement date bumped `2026-09-06`→`2026-09-07`. No pre-existing sentence was removed. **Confirmed as claimed.** |
| the ratchet constants are untouched | `git diff 1c8a7301..0c942ac2 -- . \| grep -n "^[+-].*\(EXCEPTION_RATCHET\|MAX_UNLOCATABLE\|MAX_UNVERIFIABLE\|DUPLICATE_SOURCE_GROUPS\|max_unwitnessed\|AUTHORITY_NOT_YET_ARCHIVED\)"` | The only hit is the fold report's own prose (`design/agent-reports/2026-09-07-build-interview-T2-fold.md`) *naming* the constants as untouched — no code hunk in the diff touches a definition. **Confirmed as claimed.** |
| `revision_in_force` returns the table the KAT pins | `cargo nextest run -p xtask -E 'test(revision_in_force_pins_the_whole_table)'` on the clean tree | `1 test run: 1 passed`. **Note on the brief's "21 (stem, year) pairs":** the test's `expected` table has **14** stems × 3 tax years = 42 assertions in the loop, plus 3 explicit `None`-case assertions = **45** total `assert_eq!` calls measured (not 21) — a discrepancy in the *reverify brief's* paraphrase, not in the fold or the test. Recorded as a Nit below, not held against the fold. |
| no template added under `crates/btctax-forms/forms/`; spec untouched (bonus checks, cheap, not in the brief's explicit list but load-bearing for "nothing folded outside scope") | `git diff 1c8a7301..0c942ac2 --stat -- crates/btctax-forms/forms/ design/SPEC_interview.md` | Both empty. **Confirmed.** |

## Suite state after every revert

`git status --short` empty. `cargo nextest run --locked -p xtask --no-fail-fast` → `144 tests run: 137
passed, 7 failed, 1 skipped` (the identical 6 `form_delta` + 1 `harness_check` environment failures, same
test names, reproduced above before any plant). `cargo nextest run --locked -p btctax-core -E
'test(line_coverage)'` → `5 tests run: 5 passed`. `box-census` / `line-coverage` / `authority-manifest`
all print the pinned numbers unchanged from the brief's settled facts.

## Verdict

Every one of the nine fold kills (A–I) reds exactly as claimed when re-planted independently, with
messages matching the fold report verbatim (kill H's message is the same defect presented in a fuller
listing — not a discrepancy). Every review finding (C1, I1, I2, I3) now has a test or instrument that
catches the review's own original evidence, re-planted and confirmed red (or, for I3's live claim,
independently reproduced against irs.gov). M1–M4 are each confirmed as described, doc-only or
already-acknowledged-ungated where the fold says so. All three of the brief's negative claims hold
against the current tree. No wrong result, no unheld guarantee, and no claim the tree contradicts was
found.

The one deviation recorded is a **process** fact, not a code finding: the dispatched worktree's actual
HEAD did not match the brief's stated `b6609751`/`0c942ac2` and had to be re-pointed by commit hash
before any verification could begin (see "Worktree note" above) — filed here as a Nit for whoever wires
worktree dispatch for the next re-verify round.

Counts: C=0 I=0 M=0 N=2
