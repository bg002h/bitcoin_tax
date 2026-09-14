# REPORT — FR-200(b): Form 8283 Section A built; the $600 bag of clothes now files

**Status: BUILT, gates run, NOT COMMITTED.** The work is in the worktree
`/scratch/code/bitcoin_tax/.claude/worktrees/agent-a25d89c5d0a516238`, uncommitted, 27 files modified
and 3 added. No subagents were spawned. No gate was backgrounded.

**One line.** A non-crypto noncash charitable gift no longer refuses the packet: Form 8283 Section A is
transcribed line-1-column-by-column from the text layer, the columns it asks for are collected on
`CharitableGift.noncash`, and `RefuseReason::NonCryptoNoncashGift` now carries **which** condition
refused instead of being the whole class.

---

## 1. The gate — run SERIALLY, in the foreground

    $ cargo nextest run --workspace --no-fail-fast
         Summary [  19.721s] 3853 tests run: 3852 passed, 1 failed, 12 skipped
            FAIL [   0.125s] (3833/3853) xtask::bin/xtask
              line_coverage_check::tests::the_committed_coverage_table_is_consistent_with_the_form_text

    $ cargo clippy --workspace --all-targets -- -D warnings
        Finished `dev` profile [optimized + debuginfo] target(s) in 3.78s      # clean

    $ cargo fmt --all --check
        (no output; exit 0)

Baseline for comparison: the same three commands were run on a `git stash`ed tree and all passed, so the
one failure is mine, and it is **three xtask ratchets that only the controller may move.** Verbatim:

    line-coverage FAILED (3 problem(s)):
      - 13 bundled (form, year) pair(s) are covered by this table at a DIFFERENT year only, so their
        quotes are verified against another booklet (ratchet 12): … f8283--2025 (quoted: 2024) …
      - 33 exceptions, ratchet is 31 — every one is a line that fits no production. Raising the
        ratchet is a decision, taken in a diff, with a reason.
      - 19 row(s) name a line that cannot be located in the form text, so their quote is bound to
        nothing (ratchet 17): … f8283:1(h) (SectionARow.col_h_fair_market_value),
        f8283:1(g) (SectionAColumnsEfg.col_g_cost_or_adjusted_basis)

### What the controller has to decide, with the numbers

| `crates/xtask/src/line_coverage_check.rs` | now | needs | why |
|---|---|---|---|
| `MAX_EXCEPTIONS` | 31 | **33** | the two Form 8283 Section A money columns, (g) and (h) |
| `MAX_UNLOCATABLE` | 17 | **19** | the same two rows: `pdftotext -layout` transposes Section A's header block, so a column-suffixed label (`1(g)`, `1(h)`) cannot bind to line 1's own text — the documented `f8949:I-1(d)` class, already carrying six units for exactly this reason |
| `MAX_UNQUOTED_BUNDLED_YEARS` | 12 | **13** | `f8283--2025` joins the list: the table now covers `f8283` and quotes it at 2024. Twelve other bundled 2025 pairs are already on it |

★ **And a one-line fix that would retire the first two, which I recommend instead of the ratchet bump.**
Both columns are `Production::Exception` *only* because `booklet_for("f8283")` has no arm, so
`Production::filer_records` — the correct classification, and the one that would bind them — reports
*"knows no instructions document"*. The booklet **is archived**
(`design/forms/extract/i8283--2024.txt`, `--2025.txt`), and the README's own rule is the identically-
numbered convention. Adding `"f8283" => Some("i8283")` to that join lets both rows become
`filer_records`, quoting i8283's own sentences:

* (g) — *"For items over $500, enter your cost or adjusted basis."* (`i8283--2024.txt:926`)
* (h) — *"Enter the FMV of the property on the date you donated it."* (`i8283--2024.txt:936-937`)

The reason is written into each row's `reason` string in `line_coverage.rs`, so whoever reads the diff
finds it there too. `MAX_UNQUOTED_BUNDLED_YEARS` still needs 12 → 13 either way.

### The `RefuseReason` census did NOT move

I expected it to and it did not, because `NonCryptoNoncashGift` kept its **name** and gained a payload
(`NonCryptoNoncashGift(NoncashGiftRefusal)`) rather than being split into twelve variants. `xtask
blockers` enumerates variant names, so the count is unchanged and no census pin needs raising. That was
deliberate: twelve new names would have bought nothing the payload does not, and the payload keeps the
`_`-free match in `btctax-input-form::attribute` a one-line change.

---

## 2. What files now, and what still refuses

**Files (Form 8283 Section A, one row per gift):** a noncash gift whose property block is present, whose
class is clothing-or-household-item **in good used condition or better** or `OtherProperty`, claiming
`$5,000` or less, with columns (a), (c), (i) non-blank, column (h) equal to the claim, and — only when the
claim is over `$500` — columns (e), (f) and (g) all collected.

**Still refuses, each naming its own condition** (`NoncashGiftRefusal`, 14 variants, `_`-free `detail()`):

| condition | authority |
|---|---|
| `DetailsNotCollected` | the FR-200(b) state: nothing the columns ask for was recorded |
| `DoneeNameAndAddressEmpty` / `DescriptionAndConditionEmpty` / `FmvMethodEmpty` | Reg. §1.170A-16(c)(3)(v) requires Section A to be *fully* completed; i8283 forbids *"available upon request"* |
| `ColumnsEfgRequiredButNotCollected` | over $500: *"For items over $500, enter your cost or adjusted basis."* btctax produces no reasonable-cause attachment, and that determination is the filer's |
| `FmvReducedNeedsAttachedStatement` | (h) > claim ⇒ *"You must attach a statement if you were required to reduce the FMV"* |
| `ClaimExceedsFairMarketValue` | (h) < claim ⇒ a claim larger than the value contributed; §170(b) ceilings only reduce, and apply to the return's total |
| `Vehicle` | §170(f)(12) / Form 1098-C; column (b)'s box and VIN are **not in the field map** at all |
| `IntellectualProperty` | §170(e)(1)(B)(iii); Form 8899, deduction accretes over later years |
| `InventoryOrHeldForSale` | §1221(a)(1); §170(e)(1)(A) basis limit, interacts with COGS |
| `OverFiveThousandNeedsSectionB` | *"more than $5,000 per item or group of similar items"* ⇒ Section B ⇒ a qualified appraiser btctax does not hold for non-ledger property |
| `NotGoodUsedConditionNeedsSectionBAppraisal` | §170(f)(16)(A): not in good used condition, over $500 ⇒ qualified appraisal **attached** + Section B |
| `NotGoodUsedConditionNotDeductible` | §170(f)(16)(A): not in good used condition, $500 or less ⇒ **no deduction at all**. Pub. 561: *"You cannot take an income tax charitable contribution deduction for an item of clothing unless it is in good used condition or better."* |
| `PubliclyTradedSecurity` | **a deliberate, recorded boundary — see §3** |

★ Nothing widens into silence: the `match` on `NoncashPropertyKind` is `_`-free, so a new property class
is a build error at the decision point.

---

## 3. ★ I NARROWED the brief on one point, deliberately: publicly traded securities refuse

The brief says *"anything over $5,000 that is not publicly traded securities must still refuse"*, which
implies a security over $5,000 may file in Section A. **It does not, in this build, and the reason is
recorded in the source** (`NoncashPropertyKind::PubliclyTradedSecurity`'s doc comment) rather than left
as a silent gap. Three of Section A's columns ask a security something btctax has never collected:

* **(g)** — i8283 (`i8283--2024.txt:926-929`): *"Do not complete this column for publicly traded
  securities held more than 12 months, **unless you elect** to limit your deduction cost basis. See
  section 170(b)(1)(C)(iii)."* Whether (g) prints **at all** turns on an election btctax does not model,
  and printing a basis the form says not to complete is the Section B column (i) defect again — an entry
  the form did not ask for, sworn to under §6065.
* **(c)** — for securities the instructions want the company name, the number of shares, the kind of
  security, whether it is a mutual fund share and whether it is regularly traded: five sub-fields, not
  one free-text line.
* **(e)** — *"For publicly traded securities, enter only if you held the securities for more than 12
  months."*

Admitting a security would file a row whose (c), (e) and (g) are each a guess. That is the understatement
direction on the highest-scrutiny form in the packet, so it fails closed and says which class it is.
**Recommended follow-up (own its own task):** collect the five (c) sub-fields, the >12-month holding
answer and the §170(b)(1)(C)(iii) election, then delete the refusal.

A consequence worth naming: with securities, vehicles, IP and inventory all refusing for their own
reasons, the over-$5,000 rule reaches `section_a_row` with **no exception** — every class Section A
carries above that line has already returned. That is why the code reads a plain `claimed >
SECTION_B_PER_ITEM_THRESHOLD`, not a conjunction.

---

## 4. The TWO $500 thresholds — how each is decided, and the proof they are distinct

| constant | question | measured over | read by |
|---|---|---|---|
| `printed::FORM_8283_THRESHOLD` | is a Form 8283 filed **at all**? | the **TOTAL** — every non-crypto noncash gift **plus** the ledger's crypto donations | `return_1040::noncash_section_a`, which turns it into `NoncashGate::{FormIsFiled, AllowabilityOnly}`; and `packet.rs`'s presence test |
| `form8283_section_a::COLUMNS_EFG_PER_ITEM_THRESHOLD` | must columns **(e)(f)(g)** be completed? | **ONE item** or group of similar items (`CharitableGift.amount`) | `columns_efg_required`, inside `section_a_row` |

Authorities: the filing one is the form's own header (`f8283--2024.txt:12-13`), *"Attach one or more Forms
8283 to your tax return if you claimed a total deduction of over $500 for all contributed property."* The
per-item one is the note printed between the two halves of line 1 (`f8283--2024.txt:39`), *"Note: If the
amount you claimed as a deduction for an item is $500 or less, you do not have to complete columns (e),
(f), and (g)."*

★ A third constant, `SECTION_B_PER_ITEM_THRESHOLD` ($5,000), is *defined as*
`tables::QUALIFIED_APPRAISAL_THRESHOLD` rather than retyped, so the two cannot drift while both claim to
be §170(f)(11)(C)'s *"more than $5,000"*.

### ★★ The proof they are distinguished — DISJOINT kill sets, measured

On today's form both read `$500`, so a swap reds nothing and no single vector can tell them apart. Two
plants, run separately:

| plant | reds | passes |
|---|---|---|
| `COLUMNS_EFG_PER_ITEM_THRESHOLD` → `dec!(5000)` | `the_six_hundred_dollar_bag_of_clothes_files_section_a_with_columns_efg`, `the_per_item_carve_out_cannot_be_moved_by_the_years_total`, `over_five_hundred_each_missing_efg_column_refuses_on_its_own`, `clothing_not_in_good_used_condition_routes_three_ways`, `every_refusing_property_class_names_its_own_condition` (5 of 22) | **`four_two_hundred_dollar_bags_file_the_form_and_omit_columns_efg`** |
| `FORM_8283_THRESHOLD` → `dec!(5000)` | `four_two_hundred_dollar_bags_file_the_form_and_omit_columns_efg`, `the_per_item_carve_out_cannot_be_moved_by_the_years_total` (2 of 22) | **`the_six_hundred_dollar_bag_of_clothes_files_section_a_with_columns_efg`** |

The two headline vectors are each other's controls: $600 one item (files, needs e/f/g) versus four × $200
totalling $800 (files, needs none of them).

★ **And the FILING measure is deliberately PRE-ceiling**, because i8283 says so
(`i8283--2024.txt:51-53`): *"For this purpose, 'amount of your deduction' means your deduction before
applying any income limits that could"* reduce it. So a year whose §170(b) ceilings push printed Schedule
A line 12 under $500 is still asked for the details — an over-ask in the conservative direction, never an
under-report. **This surfaces a PRE-EXISTING gap, recorded below as follow-up F-1.**

---

## 5. Validation — from i8283 and Pub. 561, never from my own implementation

No oracle was consulted and none could be: both consume Schedule A line 12 as an **input** (§G-9). Every
expected value below is quoted from a document with its line cited, and is in the source beside the
assertion.

| KAT | expectation and its authority |
|---|---|
| `the_six_hundred_dollar_bag_of_clothes_files_section_a_with_columns_efg` | Section A row; (e)(f)(g) **completed** because `$600 > $500` (`f8283--2024.txt:39`; `i8283--2024.txt:926`); (i) = `Thrift shop value`, i8283's own example for clothing and household items (`i8283--2024.txt:946-947`), backed by Pub. 561 (`Pub561…txt:310-318`): *"The price that buyers of used items actually pay in used clothing stores, such as consignment or thrift shops, is an indication of the value."* |
| `four_two_hundred_dollar_bags_file_the_form_and_omit_columns_efg` | form filed (total $800 > $500, the header sentence); all four rows `NotRequiredDeductionAtOrUnderFiveHundred` (the line-1 note) |
| `exactly_five_thousand_is_section_a_and_a_cent_more_is_section_b` | strict `>`: Section A's heading, *"an item (or a group of similar items) for which you claimed a deduction of $5,000 or less"* |
| `clothing_not_in_good_used_condition_routes_three_ways` | the three §170(f)(16)(A) outcomes, quoted from `i8283--2024.txt:735-743` |
| `a_poor_condition_item_under_the_filing_threshold_still_refuses` | allowability is not a question about an attachment: a $50 poor-condition item refuses on a year that files no 8283 |
| `below_the_filing_threshold_the_forms_own_completeness_is_not_demanded` | its mirror: an empty column (i) on a $300 gift refuses only on a filing year |
| `column_h_disagreeing_with_the_claim_refuses_in_both_directions` | (h) > claim ⇒ the attached-statement sentence (`i8283--2024.txt:936-939`); (h) < claim ⇒ §170(a) |
| `the_published_toml_spelling_parses_including_the_iso_date` (btctax-cli) | the spelling `docs/income-import-schema.md` publishes, ISO date quoted as that document instructs |

**Conformance is a test, not a review** (`crates/btctax-forms/tests/f8283_section_a.rs`): the nine
`(letter, heading)` pairs are **derived from `design/forms/extract/f8283--<year>.txt`** by reassembling
each heading down its own `pdftotext -layout` x-span, for every year in `SUPPORTED_YEARS`, and compared
to `SECTION_A_COLUMNS` in **both directions**. The four-row capacity is derived too — from the form's own
`A`/`B`/`C`/`D` row labels — and compared with what the year's map enumerates.

---

## 6. B1 — "which test reds when this is reverted?", one sentence each, with pasted output

Ten plants were run. Each is a mutation of the **transcription, the emitter or the fixture**, never of a
checker's own vocabulary (FR-235). Each was applied, observed, and reverted from a file backup.

**1. Swap the `(e)` and `(f)` headings in `SECTION_A_COLUMNS`** ->
`every_section_a_column_of_the_form_is_accounted_for` reds:

    TY2024 column (e): the table says "(f) How acquired by donor" and the form prints
    "(e) Date acquired by donor (mo., yr.)". A real sentence attached to the WRONG column is the
    Form 6251 line-33 class.

**2. Delete the `'b'` (vehicle) entry from `SECTION_A_COLUMNS`** -> same test reds:

    TY2024: the FORM prints Section A column(s) the table does not account for: ['b']. Every column
    needs a determinate provenance — collected, carve-out, or a named refusal.

**3. Set the Section A copy count to 1** — `section_a.len().div_ceil(cap_a)` becomes `1`, the plausible
"one page is enough" edit -> `a_fifth_noncash_gift_reaches_a_second_copy_and_is_not_dropped` reds, and
the message enumerates exactly the four that reached paper:

    gift 5 (BAGMARKER5) reached no page of the emitted Form 8283. Section A prints four rows per copy;
    the fifth must continue onto another copy, never vanish. Values on paper: [… "Goodwill 1, …",
    "BAGMARKER1 …", "Goodwill 2, …", "BAGMARKER2 …", "Goodwill 3, …", "BAGMARKER3 …",
    "Goodwill 4, …", "BAGMARKER4 …", …]

**4. Give the carve-out arm a zero** — the `NotRequiredDeductionAtOrUnderFiveHundred` arm gains a
`push_money(…, Usd::ZERO, …)`, the "just print a zero" edit ->
`an_omitted_column_efg_writes_nothing_rather_than_a_zero` reds:

    column (g) printed a zero for an item the form does not ask it of. "you do not have to complete
    columns (e), (f), and (g)" is not "the basis is zero", and a hardcoded zero is indistinguishable
    from a computed one on the page. Values: [… "11/30/2024", "0", "200", "Thrift shop value", …]

**5. THE FIRST TRY AT A MIXED-SECTION PLANT WAS BLIND, and that is recorded rather than quietly
replaced.** Restoring the old single-section read — one section for the whole form, read off the first
carrier — **while leaving the separate Section A row list in place** left all nine forms tests green. It
was not a kill, because in that version the noncash rows never travelled through the ledger's partition.
The plant that DOES red reproduces the old code's actual consequence: make the noncash rows conditional
on the ledger's single section, by wrapping the `section_a.extend(...)` call in `if ledger_b.is_empty()`.
Then `a_section_b_crypto_year_with_a_noncash_gift_prints_both_sections` reds:

    the Section A gift's description must be on paper: ["John Doe & Jane Doe", "123-45-6789",
    "BTCMARKER 1.00000000 BTC", "60000", "03/2021", "Purchased", "1200", …]

— the Section B crypto leg printed, and the $600 bag of clothes printed **nowhere at all**.

★ **What that measurement also says, and it is now stated in the source:** no input `form_8283()` can
produce exercises `partition_by_section`'s *mixed-ledger* branch, because the ledger's section is decided
once from the year aggregate and is uniform across the year. The branch is defensive; both its doc comment
and this report say so. What IS killed is the property that matters today — a non-crypto Section A gift
prints in Section A whatever section the LEDGER is in.

**6. Delete one row entry from the 2024 map's Section A row list** ->
`the_section_a_row_capacity_matches_the_forms_own_row_labels` reds:

    TY2024: the FORM prints 4 Section A row(s) ({'A', 'B', 'C', 'D'}) but the map enumerates 3 — the
    overflow paginates against the map, so a disagreement silently drops or invents a row

**7 and 8. The two $500 thresholds** — disjoint kill sets, tabulated in §4 above.

**9. Remove `method_used_to_determine_fmv` from the worked TOML** ->
`every_published_noncash_key_is_read_by_the_deserializer` reds:

    `schedule_a.charitable[].noncash.method_used_to_determine_fmv` is published in
    docs/income-import-schema.md but the worked TOML in this test does not exercise it — either the
    document publishes a key nothing reads, or this test has stopped covering the surface it claims to.

**10. Flip `good_used_condition_or_better` to `false`** in the end-to-end fixture ->
`a_six_hundred_dollar_bag_of_clothes_exports_a_packet_with_its_form_8283` reds at the CLI boundary, which
measures two things at once (the routing reaches the CLI, and it reaches it **named**):

    the 2024 return is not computable [NonCryptoNoncashGift(NotGoodUsedConditionNeedsSectionBAppraisal)]:
    noncash charitable gift #1: a single article of clothing or a household item that is NOT in good used
    condition or better, with a claimed deduction over $500, is deductible only with a qualified appraisal
    attached to the return along with Form 8283 SECTION B (§170(f)(16)(A); i8283 "Clothing and household
    items not in good used condition") — btctax holds no appraisal. Complete Form 8283 Section B by hand,
    or remove the gift — no forms were written

### The fifth gift, stated plainly

It cannot disappear, and there are three independent guards. In core,
`five_gifts_produce_five_rows_and_none_is_dropped` asserts five gifts produce five rows **in input order**
(by description, not by count — a count is what a dropped row still satisfies). In forms, plant 3 above is
the kill on the emitted PDF's own fields. The third is structural: Section A's printable list is ONE
derived `Vec` (the ledger's Section A rows transcribed, then the noncash rows), and the overflow chunks
that one list — so there is no second list for a gift to fall out of.

---

## 7. What was built, and the two things the existing instruments forced

**New files:** `crates/btctax-core/src/tax/form8283_section_a.rs` (1,325 lines, 22 KATs) — the line-1
column transcription (`SectionARow`, one field per lettered column, the form's own heading as the doc
comment), the collected input (`NoncashGiftProperty`), the column-provenance table (`SECTION_A_COLUMNS`),
the two thresholds, `NoncashGiftRefusal`, and the single decision `section_a_row` plus the single walk
`screen_noncash_gifts`. Plus `crates/btctax-forms/tests/f8283_section_a.rs` (505) and
`crates/btctax-cli/tests/f8283_section_a_import.rs` (124).

**"Optional" is not "blank", structurally.** `SectionAColumnsEfg` has exactly **two** variants:
`Completed{…}` and `NotRequiredDeductionAtOrUnderFiveHundred`. *"Nobody ever collected it"* is **not
representable** — `section_a_row` turns it into `ColumnsEfgRequiredButNotCollected`, a refusal. The
carve-out arm in the emitter writes no cell, no placement and no zero.

★ This is also why the noncash rows could not be squeezed into `Form8283Row`: all three of its (e)(f)(g)
fields are non-optional, which is §G-11's *"the emitter cannot express blank"* exactly. `Printed8283Rows`
therefore carries a second list, and the emitter's Section A branch was rewritten to consume the
transcription for **both** sources (a ledger leg converts with `Completed`, because a lot always has an
acquisition date, a `BasisSource` and a basis — so btctax is never in the position the carve-out exists
for, and printing all three is what the form invites: *"you do not **have to** complete"*).

**ONE definition, two readers.** `return_1040::noncash_section_a` is read by `screen_compute_dependent`
(to refuse) and by `packet::assemble_printed_forms` (to print). The packet's call carries a loud backstop
panic naming `screen_compute_dependent` as the gate — the established `route_8949_boxes` pattern — rather
than silently emitting an under-reported property list.

**Two existing instruments forced real design changes, and both were right:**

1. `every_money_leaf_household` (derived from `maximal_sentinel`, setting every money leaf to a distinct
   value) **refused to be satisfiable** by my first design, because column (h) and the gift's `amount` must
   be equal. That is a genuine cross-leaf invariant the derived pass cannot know, so it is now stated
   beside the §223 one already there, as a second *"ORDERING CONSTRAINT, stated rather than discovered"* —
   including pinning the amount to `$4,999` (not congruent to the `1_000 + 137*i` series, so both leaves
   stay distinct from every other money leaf, which is what that household exists to guarantee).
2. `the_surviving_sentinels_are_exactly_the_deliberately_kept_fields` would have caught a donee's name and
   street address riding into a file the command stamps shareable. Four Section A free-text columns are now
   scrubbed — (a) as the §8b `recipient_name`/`recipient_address` identity class, (c)/(f)/(i) as the
   `schedule_1a.vehicles[].description` class — with trim-emptiness preserved so a scrubbed copy cannot
   refuse differently. Four §3.3 matrix rows were added for them, and `scrub.rs`'s destructure of the
   block is `..`-free, so a future column is a compile error there.

**`CharitableClass::is_cash()`** is new, an `_`-free `match`, replacing a typed-out
`!matches!(g.class, Cash60 | Cash30)` at the refusal site — so a seventh §170(b) class is a build error in
the one place that knows the answer.

**Ratchet raised INSIDE my own remit** (not xtask): `btctax-input-form`'s `EXEMPT_PREFIX_CEILING`, 5 -> 6,
for `schedule_a.charitable[0].noncash`. The reason is written beside it: the interview form has no fields
for the nine columns yet, exactly as `schedule_1a` has none, and it is held back for the same stated
reason — the property-KIND prompt decides §170(f)(16)(A) allowability and a careless YES there is an
overstatement no test would catch. The gap is fail-CLOSED (no block ⇒ the year refuses above $500), and
the TOML import surface carries every column.

**Generated docs regenerated** (both are derived, both were stale after the field was added):
`docs/income-import-schema.md` (via `cargo run -p xtask -- toml-schema`; the noncash keys are now
published, and required-key count 63 -> 72) and `docs/examples/examples.md` (one line: a serialized
`"noncash": null`).

---

## 8. Findings this work surfaced that are NOT mine to close — recommended follow-ups

**F-1 (Important, pre-existing).** The FILING threshold is measured in two places against two different
quantities, and i8283 says only one of them is right. `noncash_section_a` uses the **pre-ceiling** total
(the instruction's own measure: *"'amount of your deduction' means your deduction before applying any
income limits"*), while `packet.rs` decides whether to ATTACH the form from the **post-ceiling** printed
Schedule A line 12. They part company whenever a §170(b) percentage ceiling binds: gross noncash over
$500, printed line 12 at or under it, and i8283 requires a Form 8283 that btctax then does not attach.
This predates FR-200(b) — the same disagreement governs crypto donations today — and closing it means
changing a presence test that byte-goldens depend on, so I left it alone and made the screen the
conservative superset. Owning phase: whichever cycle next touches the packet's attachment decisions.

**F-2 (Minor).** `no_oracle_invisible_entry_is_stale` does not discriminate. I probed it: an
`ORACLE_INVISIBLE` entry for `filing_status` — a routing fact that demonstrably DOES reach the oracle row
(`every_filing_status_round_trips_through_the_oracle_row`) — leaves it green. So it checks that a prefix
matches some leaf path, not that the exemption is needed. That means an over-broad entry can be added and
nothing objects, which is the *"green and blind"* shape. I mention it because I used it to try to decide
whether `noncash.kind` needed an entry, and it could not answer; the derived
`every_routing_fact_either_reaches_the_row_or_is_named_in_oracle_invisible` said no, and that one IS
derived, so I took its verdict and added no entry for `kind`.

**F-3 (Minor).** `booklet_for("f8283")` — the one-line xtask join described in §1, which would let both
Section A money columns be `filer_records` instead of `Exception` and retire two of the three ratchet
bumps. The booklet is already archived.

**F-4 (Minor).** Publicly traded securities in Section A — the recorded boundary of §3. Collect the five
column (c) sub-fields, the >12-month holding answer and the §170(b)(1)(C)(iii) election, then delete
`NoncashGiftRefusal::PubliclyTradedSecurity`.

**F-5 (Minor).** The interview form has no fields for the nine Section A columns — the exemption in §7.
Its removal is what makes the coverage KAT police them.

**F-6 (Nit).** A ledger Section A donation's column (i) can still print blank, because `fmv_method` is
`""` for Section A unless the filer stored an override. That is a pre-existing honest gap (a noncash gift's
empty (i) now refuses, because that path has a filer to ask); Reg. §1.170A-16(c)(3)(v)'s fully-complete
requirement arguably reaches it.

---

## 9. Process notes the controller should know

* **A near-miss worth recording: I reverted a plant with git and it ate my own work.** Reverting one
  planted line in `crates/btctax-core/src/tax/printed.rs` through git discarded **every** uncommitted edit
  in that file — the `section_a_noncash` field, its accessor and the `form_8283_printed` signature — because
  the file was never committed. I noticed within a minute (the field was gone from a grep), rebuilt the
  three edits by hand, and confirmed with `cargo build --workspace --all-targets`. Every later plant was
  reverted from a `cp` backup instead. This is the `git-checkout-eats-uncommitted-mutation-reverts`
  learning, met in the wild and re-earned.
* **The `Write` tool was used for SOURCE files, not for this report.** Bash heredocs of ~10KB were refused
  by the worktree-isolation guard as "too complex to verify"; source files went through `Write`/`Edit`, and
  this report was written with heredocs in five appends, as the brief requires.
* **Nothing was committed, no subagents were spawned, no gate was backgrounded**, and
  `crates/btctax-oracle-harness/**`, `crates/xtask/**`, `design/forms/extract/**`, `legal/**`, the other
  `design/agent-reports/REPORT-*` files and `FOLLOWUPS.md` were not touched.
