# REPORT — S4: making TY2026_PORT_REPORT.md and FORM_AUTHORITY_TABLE_DESIGN.md true

**Agent:** S4 (sonnet), isolated worktree `agent-a5cf5c481c9161a02`. Findings owned: **FR-137, FR-145,
FR-149, FR-151**, plus two forward-looking corrections handed down from agents A2 and A3. Files touched
(exclusive ownership per `design/agent-reports/PLAN-fr134-151-burndown.md`): `design/TY2026_PORT_REPORT.md`,
`design/FORM_AUTHORITY_TABLE_DESIGN.md`. **Nothing else touched** — no code, no `FOLLOWUPS.md`, no commit,
no push.

**No kill exists for this work, and none is claimed.** These are prose design documents; there is no test
to run red-then-green. What follows instead is: (a) the source-of-truth citation for every claim added,
and (b) which claims I re-measured myself in this tree rather than trusting the rehearsal report's account
of them, per the brief's instruction that the rehearsal itself could contain a sixth stale "today" claim.

---

## FR-137 (rehearsal F1) — step 12 cannot follow step 11; the dependency order deadlocks

**File:** `design/TY2026_PORT_REPORT.md`, §3 runbook table (step 12 row) + new correction paragraph
after the table.

**Before:** `| 12 | Dump the AcroForm field inventory | **M** | \`xtask dump-fields\` |` — no mention of
the deadlock.

**After:** the cell now points to a new paragraph stating the mechanism and the fix (dump fields from
`design/forms/<year>/<stem>--<year>.pdf` instead, or reorder step 12 ahead of step 11).

**Verified myself, not trusted from the rehearsal:**
- `crates/xtask/Cargo.toml:19` — `btctax-forms = { path = "../btctax-forms", ... }` — confirmed `xtask`
  depends on `btctax-forms` (grep, read in full context, not head/tail).
- `crates/btctax-forms/build.rs:104-109` — read the panic block in full:
  `"build.rs: forms/{year}/{stem} has {} — a template without a map (or a map without a template) is a
  form the crate would list and could not fill. Location is status: both files, or neither."` This is
  the exact live message today, byte-checked against my quote.
- Did not re-run the actual `cargo run -p xtask -- dump-fields` reproduction (that would require
  copying an unpaired PDF into `crates/btctax-forms/forms/`, which is outside my exclusive files and
  would leave a stray tracked-directory mutation) — the mechanism (dependency + panic condition) is
  fully confirmed by source; the exit-101 reproduction itself is the rehearsal's own measurement and I
  did not need to repeat it to trust the mechanism.

## FR-149 (rehearsal F15) — step 1's real output, `YEAR.toml`, is missing from the runbook

**File:** `design/TY2026_PORT_REPORT.md`, §3 (step 1 row) + new correction paragraph.

**Before:** step 1's "today" column described only the IRS-page lookup; no mention of `YEAR.toml`.

**After:** cites `forms/2025/YEAR.toml`'s actual `forms_expected`/`[forms_absent]` structure and states
the fix (move the stem from `forms_absent` to `forms_expected`) or the two tests that catch its
omission.

**Verified myself:**
- `cat crates/btctax-forms/forms/2025/YEAR.toml` — confirmed `f8995a` is listed today under
  `[forms_absent]` with reason `"not ported to TY2025 (Form 8995-A Rev. 2025 map not yet
  transcribed)..."` — quoted verbatim from the live file, not the rehearsal's paraphrase.
- `crates/btctax-forms/src/year_record.rs:170-173`, read in full: the exact format string
  `"{}: {p} is bundled but not expected (and {})"` with the two possible second clauses
  (`"declared ABSENT — a contradiction"` / `"not declared absent either"`) — confirms the message I
  quoted is the live one, not a rehearsal paraphrase.
- `crates/btctax-forms/tests/field_census.rs::every_emittable_form_is_reached_by_the_gate_or_named_absent`,
  read in full: confirmed it independently compares measured-absent (glob) against
  `record.forms_absent.keys()` and reds with `"{year}: recorded absent {...}, measured absent {...}"`
  on a mismatch — this is a *second*, independently-verified red the original FOLLOWUPS entry did not
  cite by name, so I named both.

## FR-145 (rehearsal F11) — the port of one form is one atomic commit

**File:** `design/TY2026_PORT_REPORT.md`, new correction paragraph after the runbook table, plus inline
pointers on the step 11 and step 12 rows.

**Content:** states that from step 11 (template copied) to step 21 (map finished) the workspace does
not build — the identical `build.rs` panic as FR-137, the mirror case (`.pdf` present, `.map.toml`
absent) — so a mid-sequence commit leaves the tree unbuildable, and `forms port` (§4, unbuilt) is meant
to close this gap by writing both files together.

**One number I deliberately hedged rather than round to a single value:** the rehearsal's own report
gives two different upper bounds for the atomic window — its F11 finding text says "steps 11 to 22" and
the FOLLOWUPS.md FR-145 entry's body says "between step 11 and step 21." I did not silently pick one;
the doc now says "steps 11 through 21 (through 22 for a genuinely new `line_set`/`schema()` arm)" and
explains why the boundary can extend by one step (a brand-new line-set revision needs `line_set.rs`
edits before the crate compiles; a constants-only port like f8995a/2025 does not). This is a citation
discrepancy in the *source material*, not something I introduced — I did not invent a resolution beyond
naming the mechanism that explains both numbers.

## FR-151 (rehearsal F18) — the design contradicts itself on `line_set` for a constants-only year

**File:** `design/FORM_AUTHORITY_TABLE_DESIGN.md`, §4, new correction paragraph after "Deciding the
other way — one `line_set` per struct — would erase the renumber the field exists to name."

**The contradiction, precisely:** the §4 header-example comment reads *"constants-only year ⇒ same
line_set; renumber ⇒ new one"* — read literally, this says a constants-only year should REUSE the
prior year's `line_set` STRING. Two paragraphs later the same section says the opposite for the
`Form1040Map`-style structs: *"those 15 maps ... get per-year `line_set`s ... that all resolve to one
struct."*

**I resolved this rather than leaving it flagged for the owner**, because the brief's own escape hatch
("if you cannot resolve it from the primary sources") did not apply — the primary sources (the rest of
§4, in the same document, and the built tree) already answer it unanimously and I did not need to invent
a new policy, only correct one sentence to match the surrounding text and the code:

- **`grep '^line_set' crates/btctax-forms/forms/*/*.map.toml` (run myself, full list, not head/tail):
  38 rows, 38 distinct values, 0 duplicates.** No map, constants-only or not, ever reuses a prior
  year's `line_set` string.
- **`LineSet::ALL.len() == 38`** (`crates/btctax-forms/src/line_set.rs:376`, read in full) — matches
  the file count exactly. (Note: the rehearsal's own report says "39" — that count included the
  rehearsal's own throwaway `f8995a/2025` addition, since discarded per its "nothing committed" rule.
  **38 is the correct count in this tree today**; I did not carry the rehearsal's stale 39 forward.)
- **I wrote a small script (not by hand) to parse every `LineSet::* => Schema::*` arm** in
  `crates/btctax-forms/src/line_set.rs::schema()` and diff each `_2025` arm against its `_2024`
  counterpart by stem. Output, pasted directly, not hand-counted:
  ```
  NEW-2025-ONLY: F1040s1a Unwired
  DIFF: F6251 Form6251Map -> Form6251ObbbaMap
  total 2025 arms: 18
  same struct as 2024: 16
  different struct: 1
  no 2024 counterpart: 1
  ```
  16 of 18 TY2025 line-sets are constants-only by the document's own definition (same schema arm as
  2024) and every one of those 16 still mints its own `<stem>/2025` string. This is the exact
  counterexample the header comment's literal reading cannot survive.

**Resolution written into the doc:** `line_set` is minted per year, always; "constants-only" versus
"renumber" decides only which `schema()` arm the freshly-minted `line_set` resolves to (same struct vs.
new struct) — not whether the string itself is reused. The header comment's *"same line_set"* clause
should read *"same `schema()` arm"*, filed as a one-line follow-on edit alongside whatever process next
touches §4's header fields.

---

## Forward-looking corrections (A2, A3) — not yet true, and I said so rather than guessing

Per the brief, I could not read either agent's diff (both are in separate isolated worktrees; neither
worktree nor a `design/agent-reports/REPORT-A2-*.md` / `REPORT-A3-*.md` exists in this tree as of
2026-09-12). I did not invent replacement wording for either. Both notes are written as **pending**,
addressed to the coordinator at integration, and explicitly say what I could and could not verify.

- **A3 (FR-138/FR-150, `cite_check.rs` + `archive_check.rs`)** — inline pointer added at
  `design/FORM_AUTHORITY_TABLE_DESIGN.md`'s `AUTHORITY_NOT_YET_ARCHIVED` row (line 212 in the
  pre-edit file — the exact line A3's own report names), plus a full note identifying which two
  sentences ("This is a DIFFERENT 'archived' ... is the other notion") are the ones that will need
  collapsing to one, once A3's rewrite lands. I did not guess the unified wording.

- **A2 (FR-141/FR-146, `build.rs`/`line_set.rs`/`f6251_revision.rs`)** — A2's own report names "one
  stale sentence in §9/§10" without saying which. I named my best-guess candidate (§10 step 3's
  "LineSet (37) / Schema (17 + Unwired) / the exhaustive schema() match", which frames `LineSet` as a
  hand-maintained enum — the same framing F10's fix removes) and a second candidate (§9's hand-lists
  table has no row for `LineSet` itself), and explicitly said I am not asserting either is the one A2
  means. This is described as an ambiguity, not resolved as a guess.

**Coordinator action required at integration:** re-read both notes against A2's and A3's actual merged
diffs and either confirm + remove the note, or correct the note if my candidate guess was wrong.

---

## What I left flagged for the owner

**Nothing.** All four of my findings (FR-137, FR-145, FR-149, FR-151) were resolvable from primary
sources already in the tree, and I resolved all four with citations rather than leaving any as an open
owner question. FR-151 was the one the dispatch brief explicitly anticipated might need to go to the
owner; I judged — and showed my work above — that it did not, because the contradiction is internal to
the document (resolved by its own §4 prose two paragraphs later) and by 100% of the committed maps, not
a genuine design ambiguity requiring a judgment call.

## Refuted premises

None of my four findings' underlying premises were refuted by my own re-verification — all four are
still true in this tree as measured directly (build.rs panic mechanism, YEAR.toml content and its two
red tests, and the line_set per-year-always practice across all 38 maps). The one correction I made to
the rehearsal's own numbers was **not** a refutation of a finding, but a stale count: the rehearsal's
report says "all 39 maps," and the correct figure in this tree today is **38** (the rehearsal's own
`f8995a/2025` addition, which inflated the count to 39, was discarded per its "nothing committed" rule).
I used 38 throughout, cited against `LineSet::ALL.len()` and a fresh `find`/`grep`, not copied from the
rehearsal's report.

## Markdown integrity

Both files re-checked after editing: `**` and backtick counts remain even in both files (no unclosed
emphasis or code-span introduced); the runbook table's column count is unchanged in every edited row
(verified by re-reading the rendered table, not just the diff).
