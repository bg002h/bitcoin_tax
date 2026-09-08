# Fold — interview T8 seam review (0C / 3I / 2M / 1N), main tree at `53dbe680`

Every finding folded. Nothing committed, nothing pushed, no subagents, no `git stash`, no
`git checkout --` over uncommitted work — every plant was made in the buffer from a `cp` backup in the
session scratchpad and restored from it. `CARGO_TARGET_DIR` unset (default `target/`).

**Suite:** 3449 (HEAD) → **3455 tests run: 3455 passed, 12 skipped**, `make check` exit **0**.
Six new tests, each watched RED on a planted defect first. (Line numbers in the pasted panics are
omitted where a later `cargo fmt` moved them; the assertion each names is quoted with it.)

## Commands

```
cargo build --locked --workspace --all-targets                   # green
make check                                                       # Summary [26.404s] 3455 tests run: 3455 passed, 12 skipped   (exit 0)
cargo run -q -p xtask -- line-coverage                           # 373 money lines, 18 forms, 31 exceptions (ratchet 31), 0 unverifiable, 17 not line-bound
cargo run -q -p xtask -- census-join                             # 290 unmodeled entries across 13 maps, every one placed
cargo run -q -p xtask -- stop-list                               # 8 + 4 sources, 86 registry prompts; no forbidden shape
cargo run -q -p xtask -- prompt-check                            # OK — 88 assertions, all verbatim   (86 at HEAD; +2 is mine, see M-2)
cargo run -q -p xtask -- cite-check / box-census / harness-check / archive-check / check-isolation   # all exit 0
cargo test -p btctax-cli --test fullreturn_oracle -- --ignored emit_fullreturn_fixture               # fixture regenerated (rename)
cargo run -q -p xtask -- examples > docs/examples/examples.md                                        # golden regenerated (rename)
make docs                                                        # no tracked man page or PDF changed
cargo fmt --all; cargo clippy --workspace --all-targets --all-features   # clean
```

---

## I-1 — the grid is projected from the WALK, not from the raw leaf

**What changed.** `ReturnHeader::build` takes `let walk = walk_dependent(ri, row);` once per row and
projects **all four** row-(5)/(6) bools as `walk.demands(gate) && leaf == Some(true)`. `credit` now
reads `walk.verdict.credit_column()` off that same walk instead of calling `credit_column(ri, row)`,
which walked a second time — same value by definition (`credit_column` *is*
`walk_dependent(..).verdict.credit_column()`), one walk instead of two.

All four gates are gated even though only `LivedWithYouInUs` is conditional today: `dependent_gates.rs`'s
Step 1 block demands `LivedWithYouOverHalfYear`, `FullTimeStudent` and `PermanentlyAndTotallyDisabled`
unconditionally, and only `if w.yes(G::LivedWithYouOverHalfYear) { w.demand(G::LivedWithYouInUs); }` is
conditioned. **Decided and stated in the code:** the guarantee is *not-demanded ⇒ blank*, so a gate that
becomes conditional later must not silently re-open the hole.

**Where.** `crates/btctax-core/src/tax/packet.rs` (the projection, and the `DependentGridRow` doc, which
now says the guarantee is held *by construction* and names what it looked like before).

**The kills, and their observed red.**

*(a) The one the existing test skipped* — `the_credit_column_reaches_the_row_the_emitter_prints`
(`dependent_gates.rs`). The old block set `lived_with_you_in_us = None` **by hand**, so it tested
`None ⇒ false` and never *not-demanded ⇒ blank*. It now runs the real hazard: the fixture answers
(5)(a) *Yes* + (5)(b) *Yes*, the filer flips (5)(a) to *No*, `answer_all_live_declarations` answers
whatever Step 4 then demands, and the test asserts — before touching the packet — that the stale leaf
**survives** (`Some(true)`), that `walk.demands(LivedWithYouInUs)` is `false`, that the row is still
claimable, and that `screen_param_free` is `None`. Only then does it read `ReturnHeader::build`'s output.

Plant: revert the projection for that one gate to `d.lived_with_you_in_us == Some(true)`.

```
thread 'tax::dependent_gates::tests::the_credit_column_reaches_the_row_the_emitter_prints' panicked at
crates/btctax-core/src/tax/dependent_gates.rs:
  row (5)(b) is blank under an unchecked (5)(a): the walk never demanded it, so the stale Some(true) leaf must not reach the page
Summary [0.005s] 1 test run: 0 passed, 1 failed, 1332 skipped
```

*(b) The printed page* — new
`full_return_forms::a_stale_row_5b_answer_is_not_printed_under_an_unchecked_row_5a`. A shadow test is
one that re-implements the predicate; this one drives the production `push_dependents_grid` + `pdf::`
path into the **real TY2025 template** through `fill_ty2025_grid_only` and reads the checkbox back off
the saved bytes, exactly as the grid's other KATs do. It also asserts the row **is** on the page (the
ODC box is checked), so a "fix" that dropped the dependent would not pass. Same plant:

```
thread 'a_stale_row_5b_answer_is_not_printed_under_an_unchecked_row_5a' panicked at
crates/btctax-forms/tests/full_return_forms.rs:
  assertion `left == right` failed: dependent 1, lived_with_you_in_us: (5)(b) may not print under an unchecked (5)(a)
    left: Some("1")
   right: None
```

**Decided differently:** nothing. The doc comment at `packet.rs` was kept (it is now true) and extended
to say *how* it is held, per the brief.

---

## I-2 — the CTC ceiling pin moved to the crate that can see the bundle

**What changed.**

- **New pin, in `btctax-adapters`:**
  `shipped_tables_are_the_validated_tables::every_bundled_years_ctc_per_child_is_the_named_ceiling`.
  It derives the year set with the file's existing `shipped_param_years(&bundle)` — which reads the
  bundle's own `Debug` rendering and cross-checks it against the artifact's public lookup, so it is
  never a hand-written year list — and asserts each year's `child_tax_credit_per_child` equals the
  named constant. It refuses to pass on an empty year set.
- **New accessor:** `btctax_core::tax::testonly::ctc_per_child_ss24h2()`, because
  `CTC_PER_CHILD_SS24H2` is `pub(crate)` and the pin must live downstream. Its doc says why it exists.
- **Core keeps only the positive control**, renamed
  `the_named_ceiling_is_the_figure_the_proof_multiplies_by` (from
  `the_named_ceiling_is_the_years_own_figure`), with its doc rewritten to say what it does and does not
  hold, and one assertion added so the accessor the downstream pin reads cannot drift from the constant.
- **`CTC_PER_CHILD_SS24H2` is unchanged at 2000**, per the brief. Its doc now names the adapters pin,
  states that the fix when the red arrives is to thread the year's `FullReturnParams` into
  `ctc_odc_line19` and never to raise the constant ($2,000 is right for TY2024, the only bundled year),
  and no longer claims a core test reds on a bundled package.
- **`FOLLOWUPS.md` FR-85** records that the pin was blind and is now derived, citing the controller's
  ledger for both measurements. The item stays **OPEN** (its owning phase is still the TY2025 package).

**Where.** `crates/btctax-adapters/tests/shipped_tables_are_the_validated_tables.rs` (new §5),
`crates/btctax-core/src/tax/advisories.rs`, `crates/btctax-core/src/tax/testonly.rs`, `FOLLOWUPS.md`.

**The kills, and their observed red.**

Plant A — bundle TY2026 (`by_year.insert(2026, ty2026_full_return());` in `tax_tables.rs`, whose figure
is already `dec!(2200)`):

```
thread 'every_bundled_years_ctc_per_child_is_the_named_ceiling' panicked at
crates/btctax-adapters/tests/shipped_tables_are_the_validated_tables.rs (the `wrong.is_empty()` assert):
["TY2026: the bundled package says §24(h)(2) is 2200 per child, advisories::CTC_PER_CHILD_SS24H2 says 2000"]

The Schedule 8812 line-8 ceiling `advisories::ctc_provably_zero` multiplies by is not this year's figure. […]
Fix it by threading the year's FullReturnParams into `ctc_odc_line19`, NOT by editing the constant: $2,000 is correct for TY2024.
Summary [0.003s] 1 test run: 0 passed, 1 failed, 103 skipped
```

(The same plant left the old core test **PASS** — that is the finding, and the controller's ledger
records it.)

Plant B — move the shipped TY2024 figure (`tax_tables.rs`, `2000` → `2200`):

```
["TY2024: the bundled package says §24(h)(2) is 2200 per child, advisories::CTC_PER_CHILD_SS24H2 says 2000"]
Summary [0.003s] 1 test run: 0 passed, 1 failed, 103 skipped
```

Both plants reverted from `cp` backups; `git diff --stat crates/btctax-adapters/src/tax_tables.rs` is
empty.

**Decided differently:** the brief offered "(a) a `pub(crate)` core helper taking params, driven from an
adapters test, or (b) a cleaner shape." I took (b): the assertion lives wholly in the adapters test —
which already owns the shipped-vs-validated comparison and the year derivation — and core exports only
the constant's value. That puts one fact in one place instead of a helper in each crate.

---

## I-3 — the qualifying child's name reaches the page, on HoH **and** QSS

**What changed.**

- **Renamed** `HouseholdHeader::hoh_qualifying_child_name` → `qualifying_child_name` and
  `FieldId::HohQualifyingChildName` → `FieldId::QualifyingChildName`; the compiler carried it through
  `classifier.rs`, `scrub.rs`, `scrub_axis.rs` (fixture sentinel and matrix row), `seam.rs`,
  `spec/sections.rs`, `spec/coverage.rs` and the CLI fixture. The `hoh_` prefix was part of the mistake.
- **One predicate, two readers.** `FilingStatus::wants_qualifying_child_name()` (HoH | Qss) and
  `wants_spouse_name_in_the_shared_entry_space()` (Mfs) — the seam's `live`/`get`/`set` and the emitter
  both read them, so the two ends cannot drift again.
- **`ReturnHeader` carries `qualifying_child_name`**, and `form1040_full.rs` writes it into
  `cells.mfs_spouse_name` when the status wants it and the string is non-empty — **outside** the
  `if let Some(sp) = &header.spouse` block, which is what made the cell unreachable on HoH/QSS. An empty
  name writes nothing: a lawful blank is the filer's silence, not an empty string.
- **The exclusivity is asserted, not assumed:**
  `packet.rs::shared_entry_space_tests::the_shared_entry_space_has_exactly_one_claimant_per_status`
  is total over `FilingStatus::ALL` and pins the claimant set to exactly `[Mfs, HoH, Qss]` — the form's
  own sentence.
- **Stale prose corrected:** the emitter comment (*"which v1 does not capture, so it stays blank"*), the
  `seam.rs` `FieldId` doc, the leaf's doc, the field's `label` and `help` (which now names QSS, as the
  instruction it quotes already did), and `design/SPEC_interview.md` §5.4.

**Where.** `crates/btctax-core/src/tax/{packet,return_inputs,classifier,scrub,scrub_axis}.rs`,
`crates/btctax-forms/src/form1040_full.rs`, `crates/btctax-input-form/src/{seam.rs,spec/sections.rs,spec/coverage.rs}`,
`crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml`, `design/SPEC_interview.md`.

**The kill, and its observed red.** New
`full_return_forms::the_shared_entry_space_prints_the_qualifying_childs_name_on_hoh_and_qss`: a TY2024
return (the only year `full_return_for` answers, i.e. the filable one) is filled through
`fill_form_1040_full` and `topmostSubform[0].Page1[0].f1_18[0]` is read back off the PDF. It loops
**`FilingStatus::ALL`** — HoH and QSS must print the child's name, MFS must print the spouse's, Single
and MFJ must print nothing — and then asserts that an unentered name leaves the cell blank rather than
writing `""`.

Plant 1, the brief's: drop the `Qss` arm from `wants_qualifying_child_name`.

```
thread 'the_shared_entry_space_prints_the_qualifying_childs_name_on_hoh_and_qss' panicked at
crates/btctax-forms/tests/full_return_forms.rs (the per-status read-back):
  assertion `left == right` failed: Qss: the shared entry space beside the filing-status boxes
    left: None
   right: Some("Robin Roe")
```
…and the core claimant test reds beside it: `left: [Mfs, HoH] / right: [Mfs, HoH, Qss]`.

Plant 2, the defect as shipped: delete the emitter's HoH/QSS write entirely.

```
  assertion `left == right` failed: HoH: the shared entry space beside the filing-status boxes
    left: None
   right: Some("Robin Roe")
```

**The TY2025 template's cell is NOT the same FQN — and that is a finding.** TY2024 prints one sentence
and one widget for three statuses. **TY2025 split it in two:** the MFS half reads *"Married filing
separately (MFS). Enter spouse's SSN above and full name here:"* and the HOH/QSS half is its own
sentence in the right-hand column (`f1040--2025.txt:29-32`). Measured with `xtask dump-fields` on the
bundled template: `Checkbox_ReadOrder[0].f1_28[0]` at (180.0, 540.0)–(324.0, 552.0) is the MFS
spouse-name cell, and **`f1_29[0]`** at (362.0, 534.0)–(576.0, 546.0) is the qualifying-child cell — the
HOH checkbox `c1_8[0] on="4"` sits at x 349.6, the same column. So a TY2025 `[header]` map will need
**two** keys where `Form1040Map` has one (`mfs_spouse_name`).

**Census: nothing is needed today, and nothing changed.** TY2025's map declares no `[header]`, so both
cells remain on the `UNCENSUSED` register with the rest of the identity block, and `census-join` is
unchanged at 290/13. The requirement is recorded under **FR-84**, whose owning task (the TY2025 identity
block) already owns that block.

**Decided differently:** the predicate lives in `packet.rs`, not beside the enum — see Deviations.

---

## M-1 — the test is renamed to what it asserts

`the_ty2024_1040_is_byte_identical_with_dependents_whose_credit_column_is_computed` →
**`the_ty2024_1040_ignores_the_computed_grid_and_fills_deterministically`**, with the doc rewritten and
the two halves labelled (determinism; and the discriminating half, the TY2024 credit boxes staying
`None`). The fixture assertion is strengthened: rows (5)(a), (5)(b) and (6) are asserted **set** on the
fixture, so "TY2024 ignores the grid" is asserted against a grid with something in every slot rather
than a default-shaped one.

**I chose rename over a committed digest, and the reason is not taste:** a *pre-T8* digest is
unobtainable for this test. Its fixture is `ty2025_two_dependents()`, which T8 itself introduced, and
`DependentRow` had no `grid` field before T8 to put in it — so any digest committed now would pin
today's bytes while the name claimed a pin against yesterday's. Byte-stability for TY2024 is really held
by `full_return_form_fills_are_byte_deterministic` and the committed golden packet, both of which ran
green through this fold.

---

## M-2 — the prompt's operative clause is now the checked span

**What changed.** `HohPaidOverHalfCostOfKeepingUpHome`'s prompt asked *"did you pay over half the cost of
keeping up a home"* — a paraphrase — beside a quote of the real test, so an edit to the lead-in alone
changed the question with `prompt-check` green (the controller reproduced it). The prompt now reads:

> Head of household — for this tax year, is this true: **"You paid over half the cost of keeping up a
> home"**? (Both Test 1 and Test 2 begin with it. See Cost of keeping up a home …)

There is no longer a second, unchecked statement of the test to drift from.

**The sweep found one more of the same shape.** `HohQualifyingPerson` quoted Test 2's whole condition and
both of Test 1's *names*, but stated Test 1's own condition — the half that says what the filer must
have **paid** — as an unchecked paraphrase between them. It is now quoted, and a `QuestionClause` for
`"You paid over half the cost of keeping up a home that was the main home"` was added. The span stops
one word short of the instruction's year (*"the main home for all of 2025 of your parent"*), which is
the table's own standing rule.

Swept and left alone, with reasons: `NraSpouseResidentElection` (its lead-in asks the filer's own fact;
the rule sentence is already quoted verbatim in the prompt), QSS conditions 2–5 (each quotes its
operative clause), and QSS condition 1 (deliberately year-free static fallback — the *rendered* variant
is what the table checks).

**Observed red, both plants** (each reverted):

```
$ cargo run -q -p xtask -- prompt-check          # plant: "over half" → "most" in the HoH cost prompt
xtask prompt-check: a prompt no longer matches the form:
  question clause 5 (HohPaidOverHalfCostOfKeepingUpHome) is NOT in the string the filer reads: "You paid over half the cost of keeping up a home"

$ cargo run -q -p xtask -- prompt-check          # plant: the same paraphrase in Test 1's operative span
xtask prompt-check: a prompt no longer matches the form:
  question clause 1 (HohQualifyingPerson) is NOT in the string the filer reads: "You paid over half the cost of keeping up a home that was the main home"
```

That first line is the exact edit that was green at HEAD.

---

## N-1 — the caption check now asserts ORDER

**What changed.** New `dependents_grid::tests::the_two_box_rows_are_captioned_in_the_forms_own_order`,
beside the existing presence check. For every (row, band) that `SLOT_CAPTIONS` gives two positions —
derived from the table itself, never listed — it finds the single line of `f1040--2025.txt` carrying that
parenthesised label and asserts position 0's caption head appears before position 1's.

Two things had to be got right, and both are recorded in the test:

- **Whole-token comparison, not `contains`.** *"(7) Credits"* contains the substring *"credit"*, so a
  substring search finds position 1's caption *before* position 0's and reports a pass on a genuinely
  swapped table. Measured while writing it.
- **The line must be unique** (asserted), so the check is a reading of the form rather than of a guess,
  and `checked >= 2` refuses to pass vacuously.

**Observed red** — plant: swap `FullTimeStudent`'s and `PermanentlyAndTotallyDisabled`'s captions in
`SLOT_CAPTIONS`:

```
FAIL  dependents_grid::tests::the_two_box_rows_are_captioned_in_the_forms_own_order
  row (6) prints FullTimeStudent before PermanentlyAndTotallyDisabled, but SLOT_CAPTIONS orders them the
  other way — a swapped caption checks the wrong box for a filer while every FQN in the map stays correct.
  The form's line: "(6) Check if             Full-time        Permanently       Full-time        Permanently …"
PASS  dependents_grid::tests::every_slot_caption_is_the_forms_own_words
Summary 2 tests run: 1 passed, 1 failed
```

The second line is the point: the old check passes on the exact defect the new one catches.

---

## Deviations

1. **`FilingStatus`'s two predicates are in `packet.rs`, not `types.rs`.** `tax/types.rs` is
   **content-pinned** by `frozen_guard::FROZEN_TYPES_SHA256` (SPEC_full_return §2, additive-only). I
   wrote them there first and `frozen_engine_files_are_unchanged` reds — correctly:
   `left: e1719323… / right: 51b912cc…`. Bumping that pin is a documented exception process needing its
   own reviewed commit, which is not what a review fold should quietly do for a printed-header
   predicate. `types.rs` was restored byte-for-byte from `HEAD` (its diff is empty, the guard is green)
   and the inherent `impl FilingStatus` block lives in `packet.rs` — same API at every call site, with
   the reason written at the block.
2. **M-1 resolved by renaming, not by a committed digest.** Reasoned above; the brief left the choice
   open and asked that it be stated.
3. **`prompt-check` is 88 assertions, not 86.** One clause added by M-2's sweep (Test 1's operative
   condition), counted twice as the checker counts every clause — once against the extract, once against
   the prompt. No other instrument moved.
4. **Two goldens regenerated, both forced by the I-3 rename and both verified to contain only it.**
   `crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml` (TOML keys sort alphabetically, so
   the key moved) via its own documented `--ignored emit_fullreturn_fixture` command, and
   `docs/examples/examples.md` (one JSON key in the J6 dump) via `xtask examples`. Diffs are one line
   each.
5. **Beyond the brief, deliberately small:** `FilingStatus::ALL` came with a totality anchor,
   `every_filing_status_is_in_all`, in the shape `provenance.rs` already uses for `DependentGate::ALL` —
   because the claimant test *loops* `ALL`, so a status missing from it would simply never be checked.
   Planted (dropped `Qss` from `ALL`): `left: 4 / right: 5`, and the claimant test reds too.
6. **Checked and deliberately NOT changed:** `scrub_axis`'s matrix row for the renamed leaf keeps
   `NoSuchState(NO_READER)` in the *malformed* column. That column's discriminator is *"does any
   predicate read a **validity class** off the field"* — the three are `Ssn::canonical`,
   `IpPin::canonical`, `canonical_ein` — and an emitter that prints a name is not one, exactly as
   `header.taxpayer.first_name` is printed and carries the same verdict. The `empty` column still
   exercises the emptiness the liveness reads.
7. **The build report is annotated, not rewritten.** `2026-09-07-build-interview-T8-implementation.md`
   keeps its original sentences and carries three dated **CORRECTION** blocks (§1's *"lawfully blank"*,
   §5's *"the moment TY2025's or TY2026's package is bundled the test REDS"*, §4's
   `hoh_qualifying_child_name` row) plus the renamed test in its kills table, so `git diff` shows what
   the review falsified rather than hiding it.

## Instruments (all four, after the fold)

```
line-coverage  line-coverage OK: 373 money lines across 18 form(s) [f1040:45 f1040s1:12 f1040s1a:50 f1040s2:9
               f1040s3:5 f1040sa:19 f1040sb:5 f1040sc:7 f1040sd:31 f1040sse:22 f6251:41 f8889:27 f8949:12
               f8959:17 f8960:15 f8995:16 f8995a:39 i1040gi:1], 31 exception(s) (ratchet 31),
               0 unverifiable (ratchet 0), 17 not line-bound (ratchet 17)
census-join    census join: 290 unmodeled entries across 13 maps, every one placed by a direction block
               asserted against the form's extract, graded by one of DIRECTION_OF_CAPTION's 22 readings
               (each cited to a sentence the form prints) and covered by an existing variant
stop-list      R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources and 86 registry prompts
               scanned; no forbidden shape
prompt-check   xtask prompt-check: OK — 88 assertions, all verbatim
```

`line-coverage`, `census-join` and `stop-list` are byte-identical to HEAD's; `prompt-check` moved 86 → 88
for the reason in Deviations 3.

## Closing gate

```
$ make check
     Summary [  26.404s] 3455 tests run: 3455 passed, 12 skipped
MAKE_CHECK_EXIT=0
```

3449 at HEAD → 3455: the six tests added by this fold
(`a_stale_row_5b_answer_is_not_printed_under_an_unchecked_row_5a`,
`the_shared_entry_space_prints_the_qualifying_childs_name_on_hoh_and_qss`,
`the_shared_entry_space_has_exactly_one_claimant_per_status`, `every_filing_status_is_in_all`,
`every_bundled_years_ctc_per_child_is_the_named_ceiling`,
`the_two_box_rows_are_captioned_in_the_forms_own_order`). `git status --porcelain` lists 22 modified
files plus this report as the only untracked one; nothing is committed.
