# REPORT — Phase 4's last two items: WHERE TO FILE, and record retention

**Built 2026-09-11** against `BRIEF-build-phase4-last-mile.md`, in the shared main tree from HEAD
`abe1ed24`. Nothing committed, pushed, stashed or checked out. Mutations reverted with `cp` backups.

**Outcome: both items built, Phase 4's exit gate is MET, `make gate` 3609 passed / 12 skipped / exit 0,
`cargo fmt --all --check` clean, `make docs` no tracked diff.** Baseline was 3598 passed / 12 skipped;
the eleven new tests are listed in §4.

---

## 1. What changed, and where

### The shared constants — one wording, three surfaces

`crates/btctax-cli/src/lib.rs`

| item | line | what |
|---|---|---|
| `WHERE_TO_FILE_STATE_FACT` | `:381` | fact 1 — which state you live in |
| `WHERE_TO_FILE_PAYMENT_FACT` | `:390` | fact 2 — whether a payment is in the envelope |
| `WHERE_TO_FILE_1040_SOURCE` | `:402` | the RETURN's table pointer |
| `WHERE_TO_FILE_4868_SOURCE` | `:419` | the EXTENSION's table pointer — a different table |
| `WHY_NO_ADDRESS_TABLE` | `:432` | why btctax prints a pointer, quoting the instructions |
| `RECORD_RETENTION_GUIDANCE` | `:505` | item (ii), in full |
| `missing_where_to_file_facts()` | `:456` | kill #1's instrument — names which part is absent |
| `normalize_guidance()` | `:478` | strips the surfaces' chrome (`#`, `•`, whitespace) so the checker compares sentences, not layout |

The facts are **constants, not literals per surface**, for the reason `DIGITAL_ASSET_HAND_MARK` is: the
packet manifest (October) and `btctax extension` (April) describe the same decision, and a filer must
not be told two different things about it. Only the *pointer* differs, because the tables differ.

### The surfaces

| surface | site | why there |
|---|---|---|
| packet `manifest.txt` — `WHERE TO POST IT` | `crates/btctax-cli/src/cmd/admin.rs:710` (`where_to_post_block`), appended at `:2248` | the manifest is *"the artifact the filer follows while assembling paper"*; this is the moment the envelope gets addressed |
| packet `manifest.txt` — `KEEPING YOUR RECORDS` | `admin.rs:788` (`record_retention_block`) | the plan says **DOCUMENT in the manifest** |
| `btctax extension` report | `crates/btctax-cli/src/render.rs:5721` | Form 4868 is mailed alone, weeks earlier — the packet manifest cannot reach that filer |
| `btctax limitations` / `LIMITATIONS.md` | `crates/btctax-cli/LIMITATIONS.md:477`, *"Two things btctax deliberately does not print"* | `include_str!`'d into the binary (`main.rs:582`) and printed verbatim, so it is **product surface**; it is also the only surface reachable **without exporting anything**, and the only place the *reason* for the absence belongs |
| the manifest's own label | `crates/btctax-cli/src/main.rs:1014` | see §5 |

**Block order inside the manifest** is the order the filer acts in: stapling list → 1040-V loose →
undated rows → `COMPLETE BY HAND` → `FORGONE` → **`WHERE TO POST IT` → `KEEPING YOUR RECORDS`**. Within
the where-to-post block the actionable half comes first and `WHY_NO_ADDRESS_TABLE` last; the first draft
had the rationale on top and it buried the instruction (I read the emitted text before deciding).

**`where_to_post_block` takes `payment_enclosed`**, read from `form_1040v_path.is_some()` — the voucher
actually written — not from `voucher.pay_by_check`, so the column named and the page in the envelope
cannot disagree. Both arms name **opposite** columns, so the line is not a constant that always says the
same thing.

---

## 2. Where the URLs came from — transcription, and one stated gap

**No archived authority in this repo prints a where-to-file URL.** Measured:
`grep -rin "irs\.gov/where|irs\.gov/filing|WhereToFile|Where-to-File"` over `design/forms/extract/`,
`legal/text/` and `crates/btctax-core/src/tax/fixtures/` returns **three** hits and none is a where-to-file
page (`IRS.gov/filing/e-file-information-returns` ×2, `IRS.gov/Filing` in Pub 550). The 1040 general
instructions point at their *own* section (*"Make sure to check Where Do You File? before mailing your
return"*), not at a URL. **I did not invent one.**

What is printed verbatim, and is therefore what I transcribed:

| URL | printed by | archived at |
|---|---|---|
| `www.irs.gov/Form1040` | Form 1040's own footer line, *"Go to www.irs.gov/Form1040 for instructions and the latest information."* | `design/forms/extract/f1040--2025.txt:168` (and `f1040--2024.txt:144`) |
| `www.irs.gov/Form4868` | Form 4868's own header line, *"Go to www.irs.gov/Form4868 for the latest information."* | `crates/btctax-core/src/tax/fixtures/f4868_2025_form.txt:18` |
| `IRS.gov/PDSStreetAddresses` | the 1040 instructions' private-delivery-service note | `design/forms/extract/i1040gi--2025.txt` (*"For the IRS mailing address to use if you're using a private delivery service, go to IRS.gov/PDSStreetAddresses"*) |

The guidance therefore points at a **document** (*"the 'Where Do You File?' table on the LAST PAGE of the
IRS Instructions for Form 1040 for the year you are filing"* — `i1040gi--2025.txt:46610`, page 126 of
126) and names the URL the form itself prints as the way to get it. That is a pointer that cannot rot in
the dangerous direction: a wrong document reference is visible, a wrong address is not.

The rot argument is now quoted from the authority rather than asserted. `i1040gi--2025.txt:40973-40978`:

> Make sure to check Where Do You File? before mailing your return. Over the next several years, the IRS
> will be reducing the number of paper tax return processing sites. Because of this, you may need to mail
> your return to a different address than you have in the past.

That is stronger than the brief's mid-2026 1040-ES correction: **the IRS says in the instructions that the
set is moving.**

---

## 3. The extension path — verified, not assumed, and the answer is yes

The brief asked me to check rather than assume. Measured, three ways:

1. **Form 4868 has its own where-to-file table, on the form itself.** `pdftotext -layout` of the bundled
   template: `crates/btctax-forms/forms/2025/f4868.pdf` is **4 pages** and page **4** carries *"Where To
   File a Paper Form 4868"* (also `crates/btctax-core/src/tax/fixtures/f4868_2025_instructions.txt:427`,
   and the 2024 template likewise). So the table **travels in the filer's hand** — the strongest possible
   pointer, and one that cannot go stale relative to the form it is printed on.
2. **Its addresses differ from the 1040's on every row.** With payment: Charlotte NC 28201-**1302**
   (4868) against 28201-**1214** (1040); Louisville KY 40293-**1300** against 40293-**1000**. Without
   payment: Austin TX 73301-**0045** against 73301-**0002**. A filer who reuses the return's address
   posts a cheque to a lockbox that is not expecting it.
3. **Both facts apply there too** — the 4868 table's columns are literally *"And you're making a
   payment"* / *"And you're not making a payment"*, keyed by state.

So `render_extension` gained the same two constants plus `WHERE_TO_FILE_4868_SOURCE`, which says outright
that it is *not* the address the return goes to. `missing_where_to_file_facts` is deliberately
**parameterised by the source**, and a test asserts the RETURN's pointer does **not** satisfy the
extension surface (and vice versa) — so the two tables cannot be conflated by a later edit.

### A second measured fact that changed the prose

The bundled **Form 1040-V** template prints a *"Mailing Address for Payments"* table on its own page 2
(`forms/2025/f1040v.pdf`, 3 last-line matches; `forms/2024/f1040v.pdf`, 4). That is the **with-payment**
column only. So a filer who has ever seen a voucher is one glance from posting a **refund** return to a
payment lockbox — which is exactly the mistake knowing only your state produces. The no-payment arm of
the block now says *"Do NOT use a table printed on a Form 1040-V"* explicitly, and the paying arm points
**at** it. I would not have written either sentence from the brief alone.

Two cautions from the same instructions page were included because both bite this packet specifically: a
full return is always more than five pages (postage), and every with-payment address is a P.O. Box (so a
courier cannot deliver it).

---

## 4. How §2's inform-without-prescribing tension was resolved

The two sources are **both right and do not actually conflict**, and the resolution is a transcription
rather than a compromise. The IRS instruction itself declines to name a window for property records
(`i1040gi--2025.txt:41182-41198`, *"How Long Should Records Be Kept?"*):

> Keep a copy of your tax return … until the statute of limitations runs out for that return. Usually,
> this is 3 years … **You should keep some records longer. For example, keep property records (including
> those on your home) as long as they are needed to figure the basis of the original or replacement
> property.**

So the guidance:

- **quotes both sentences** and says the **second** is the one that governs a crypto ledger — which is
  the plan's *holding period + 3 years* stated as a **mechanism** (every lot you still hold is a property
  record; its acquisition date and basis are what Form 8949 reports in the year you dispose of it, so the
  clock starts at the disposal, not at this filing);
- **names no date, deadline or default** — which is FIELD_PROVENANCE:464's *"do not pick a window"*
  respected as an **output**;
- **builds no retention mechanism** — it points at `export-snapshot`, which already writes the artifact
  unprompted;
- and says btctax *"will not shred anything, sets no deletion date, and does not decide how long you keep
  this"*, which is FIELD_PROVENANCE:265's never-auto-shred made filer-facing.

The no-prescription half is **held by a test**, not by care:
`the_manifest_informs_about_record_retention_without_prescribing_a_window`
(`crates/btctax-cli/tests/export_irs_pdf.rs:3325`) reds on a year (`19xx`/`20xx`), an ISO date, a month
name, or any of six prescriptive phrasings inside the block. ★ Its first version failed for the right
reason and the wrong rule: a "no four-digit run" check reported `["1040", "1099", "8949"]`. A checker that
cannot tell a form number from a year would have forced the authority's own quote out of the text — so the
rule is year-shaped, not digit-shaped.

---

## 5. The kills — both watched red on planted defects

### Kill #1 — the guidance is present and complete, and reds on either fact alone

**Instrument:** `missing_where_to_file_facts(text, source)` (`lib.rs:456`) returns which of
`{the STATE-dependence, the PAYMENT-dependence, the address source}` is absent. A raw `contains` was not
usable and that is not incidental: the manifest prefixes `#` and wraps under a bullet while
`btctax extension` wraps without it, so a raw check would pass on one surface and fail on the other for
reasons unrelated to the facts. `normalize_guidance` removes exactly those three markers and nothing
else — `normalizing_removes_chrome_but_not_words` (`lib.rs:645`) pins that a missing *clause* still
differs after normalizing, or the whole checker would be vacuous.

**(a) PRE-FIX RED — the four surface tests, against the tree before the blocks existed:**

```
        FAIL [   0.172s] (1/4) btctax-cli::extension the_extension_report_says_where_to_post_form_4868_and_that_it_differs_from_the_return
    assertion `left == right` failed: both facts and the 4868's OWN table pointer must be present: …
      left: ["the STATE-dependence", "the PAYMENT-dependence", "the address source (the year's own table)"]
     right: []
        FAIL [   0.199s] (2/4) btctax-cli::export_irs_pdf the_manifest_informs_about_record_retention_without_prescribing_a_window
    the manifest must carry a retention block: # btctax full-return packet — staple in this order …
        FAIL [   0.199s] (3/4) btctax-cli::export_irs_pdf the_manifest_tells_a_refund_filer_where_to_post_it_and_on_what_it_depends
    the manifest must carry a where-to-post block: # btctax full-return packet — staple in this order …
        FAIL [   0.316s] (4/4) btctax-cli::export_irs_pdf the_manifest_tells_a_paying_filer_the_with_payment_column_is_theirs
      left: ["the STATE-dependence", "the PAYMENT-dependence", "the address source (the year's own table)"]
     right: []
     Summary [   0.316s] 4 tests run: 0 passed, 4 failed, 56 skipped
```

**(b) PLANTED-DEFECT RED — one fact deleted from the PRODUCTION block, not from a synthetic string.**
Plant: delete `push(&mut s, crate::WHERE_TO_FILE_PAYMENT_FACT);` from `where_to_post_block`:

```
        FAIL [   0.199s] (1/2) btctax-cli::export_irs_pdf the_manifest_tells_a_refund_filer_where_to_post_it_and_on_what_it_depends
      left: ["the PAYMENT-dependence"]
     right: []
        FAIL [   0.317s] (2/2) btctax-cli::export_irs_pdf the_manifest_tells_a_paying_filer_the_with_payment_column_is_theirs
      left: ["the PAYMENT-dependence"]
     right: []
     Summary [   0.317s] 2 tests run: 0 passed, 2 failed, 46 skipped
```

Plant: delete `push(&mut s, crate::WHERE_TO_FILE_STATE_FACT);`:

```
        FAIL [   0.201s] (1/1) btctax-cli::export_irs_pdf the_manifest_tells_a_refund_filer_where_to_post_it_and_on_what_it_depends
      left: ["the STATE-dependence"]
     right: []
```

Both restored by `cp` and verified byte-identical. The unit kill
(`a_guidance_text_missing_either_fact_or_the_pointer_is_named_and_a_complete_one_is_not`, `lib.rs:572`)
plants all four deletions plus the wrong-table near miss, and asserts the checker names **that one and
only that one**.

**(c) GREEN after the fix:** `4 tests run: 4 passed` (and 5 including the `LIMITATIONS.md` test).

### Kill #2 — the "no bundled table" decision, held structurally

**`crates/xtask/src/service_center_check.rs`** (647 lines), registered `#[cfg(test)]` in
`crates/xtask/src/main.rs` beside `forge_reach_check`, whose shape it follows.

| item | line | claim |
|---|---|---|
| `usps_last_line()` | `:181` | a USPS last line — `…, <ST> <ZIP5+>` where `<ST>` is a real USPS code |
| `post_office_box()` | `:236` | `P.O. Box <number>` in any spelling — the **number is required** |
| `service_center_addresses()` | `:274` | every hit in shipped text, as `label:line: text` |
| `USPS_STATES` | `:96` | the closed ANSI/USPS set, **pinned against the authority** (below) |

**Scope:** every `.rs` under `crates/` reduced to its production half by
`r15_stop_list::production_source` (comments stripped, `#[cfg(test)]` items skipped), **plus**
`crates/btctax-cli/LIMITATIONS.md` scanned whole — it has no test half and `btctax limitations` prints
every line of it. `tests/` directories are **not** excluded, deliberately: a table pasted into a fixture
is one copy-paste from the manifest, and excluding a directory would be a boundary drawn by path rather
than by shape. Anti-vacuity floor of 300 files against **355** measured
(`find crates -name "*.rs" -not -path "*/target*" | wc -l`), plus an assertion that the shipped doc is in
the scan and non-empty.

**PLANTED-DEFECT RED.** Plants: a two-row address table in `where_to_post_block` (`admin.rs`), and one
address sentence in `LIMITATIONS.md`:

```
        FAIL [   0.130s] (1/1) xtask::bin/xtask service_center_check::tests::no_shipped_text_carries_an_irs_service_center_address
    an IRS service-center postal address is in shipped text. … : [
      "crates/btctax-cli/src/cmd/admin.rs:731: Revenue Service, Austin, TX 73301-0002; with payment: Internal Revenue Service, P.O. Box \\",
      "crates/btctax-cli/src/cmd/admin.rs:732: 1214, Charlotte, NC 28201-1214\",",
      "crates/btctax-cli/src/cmd/admin.rs:734: Department of the Treasury, Internal Revenue Service, Ogden, UT 84201-0002\",",
      "crates/btctax-cli/LIMITATIONS.md:477: If you owe, post the return to Internal Revenue Service, P.O. Box 931000, Louisville, KY 40293-1000."]
     Summary [   0.131s] 1 test run: 0 passed, 1 failed, 185 skipped
```

Both surfaces red, `.rs` and markdown. Both restored by `cp`, verified byte-identical, then green.

#### ★★ The near misses, pinned — every one of them a shape really in this repo

`a_planted_service_center_address_reds_and_its_near_misses_do_not` (`:331`) asserts **6 plants red** and
**8 near-miss families do not**:

| near miss | real occurrence | why it is not an address |
|---|---|---|
| `TD 10000` (Treasury Decision) | `printed.rs:83`, `year_record.rs:43`, `:45`, `tests/year_record.rs:107` | `TD` is not a USPS code |
| `89 FR 85279` (Federal Register) | `testonly.rs:2657`, `:2801`, `:2960`, `:2970` ×2, `:2983`, `:3153`, `:3159` | `FR` is not a USPS code |
| **a comma right before a citation** — `"(TY2025+, TD 10000)"` | `year_record.rs:43` verbatim | the closest shape there is; only the state set separates it |
| EINs — `99-9999999`, `12-3456789`, `00-0000000`, and `pii-scan-generic.sh`'s `ALLOWED_EIN` list | `document_census.rs`, `scrub_axis.rs`, `scripts/pii-scan-generic.sh:153,166` | 2-7 digits, no state |
| SSNs from the never-issued space — `987-65-4321` | `packet.rs:1330` etc. | 3-2-4 digits, no state |
| grouped dollar amounts — `$1,234,567`, `MFS 875950` | throughout | a comma beside five digits, no state code |
| `box N` field references | **1,180** occurrences of `box <digits>` in `crates/**/*.rs` | the number is not preceded by `po box` |
| P.O. boxes **discussed but never named** | all **6** `p.o. box`-shaped mentions in `.rs` are this feature's own guidance | the detector requires a digit |
| comments, and `#[cfg(test)]` items | this module's header prints two real addresses | stripped / skipped |

★★ **The first draft of the guard reported one of these as a finding.** The near-miss test failed on
`"see the regulation, TD 10000"` — because comma + two capitals + five digits *is* an address's shape, and
`TD 10000` *is* that shape. Shape alone cannot separate a citation from an address. That is why
`USPS_STATES` exists, and why it is not merely a typed list: **the members that are load-bearing are
DERIVED from the authority.** `the_state_set_covers_every_state_the_archived_tables_actually_use` (`:519`)
scans the three archived tables (`i1040gi--2025.txt`, `f4868_2025_instructions.txt`,
`f1040v_2025_form.txt`) with the **raw** two-capital shape — not with `usps_last_line`, which would be
circular — collects `{DC, KY, MI, MO, NC, TX, UT}`, and asserts every one is in the set, with a floor of 5
so a scan that reads nothing pins nothing. A center opening in a state the list lacks reds on the next
authority refresh.

#### ★ The guard's first real act was to red on my own test

`make gate` failed on `crates/btctax-cli/tests/limitations.rs:270`, a line I had written:

```
    an IRS service-center postal address is in shipped text. …:
      ["crates/btctax-cli/tests/limitations.rs:270: !norm.contains(\"P.O. Box 1214\") && !norm.contains(\"Austin, TX 73301\"),"]
```

That is the guard working, not a false positive — and the right fix was to **delete the assertion**, not to
excuse the file. A two-entry hand-typed negative list beside a six-address table is exactly the shape
`CLAUDE.md`'s *"derive the list, or make the compiler hold it"* forbids: it would pass on the four it never
named. The structural guard holds that half by shape, over that document and every `.rs` in the workspace.
The reasoning is recorded at the deletion site.

#### The complement — the pointer's target must exist

Declining to print the table is only safe if what btctax points **at** really carries it.
`every_bundled_year_of_the_two_forms_we_point_at_really_carries_its_address_table` (`:602`) **derives** the
year set from the bundled templates (`forms/<year>/f4868.pdf`, `f1040v.pdf` — currently `{2024, 2025}`;
`forms/2026/` holds only `YEAR.toml`, so it is correctly out of scope until its PDFs land) and asserts each
year's committed text layer carries *"Where To File a Paper Form 4868"* / *"Mailing Address for Payments"*.
Read from `design/forms/extract/`, not by shelling out to `pdftotext`, so it runs in CI's net-isolated job.

Its B1 pair, both planted and both restored:

```
# heading renamed in design/forms/extract/f4868--2025.txt
    the 2025 f4868 no longer prints its "Where To File a Paper Form 4868" table, and the shipped
    guidance sends the filer to it. …
# design/forms/extract/f1040v--2024.txt moved away
    f1040v.pdf is bundled for 2024 but its committed text layer is missing (…/f1040v--2024.txt):
    btctax POINTS THE FILER at a table on that page, and nothing else checks the table is there. …
```

#### The guard's blind spots, stated in its own header (§"Where it is blind")

1. **The bundled official IRS templates** carry the real addresses via `include_bytes!` — measured
   `forms/2024/f1040v.pdf` 4, `forms/2024/f4868.pdf` 18, `forms/2025/f1040v.pdf` 3,
   `forms/2025/f4868.pdf` 14 last-line matches. Correct and must not be "fixed": they are the IRS's own
   pages, they are per-year, and the 1040-V's table is what a paying filer is pointed at.
2. **The archived text-layer extracts** carry the tables verbatim and are not scanned (not `.rs`,
   `# GENERATED`).
3. **Comments and `#[cfg(test)]` items** — which is what lets the module print real addresses in its own
   header without excusing itself by path.
4. ★ **An address assembled at runtime** (`format!("{city}, {st} {zip}")`) is invisible to a source
   scanner. Nothing does this today — there is no city/state/ZIP field anywhere on the export path — and
   closing it would need data-flow analysis. Named rather than implied.
5. Shipped text that is neither `.rs` nor `SHIPPED_DOC` → filed as **FR-131**.

---

## 6. The one change outside the two items

`main.rs:1014`: the manifest's terminal label was `"← your stapling order"`. After these two blocks that
**understates the file**, and it is the filer's only signpost at it — a filer who has already stapled has
no reason to reopen "your stapling order", which would leave the gate met only in principle. Now
`"← stapling order, where to post it, what to keep"`. `docs/examples/examples.md` regenerated with
`cargo run --locked -p xtask -- examples`; the diff is **exactly one line** (`:870`), and
`examples_golden_matches_committed` + `walkthrough_console_golden_matches_committed` pass.

Also amended: `design/LONG_RANGE_PLAN_filing.md`'s superseded-banner Phase 4 row, which still asserted
*"zero irs.gov where-to-file references … the phase's own exit gate is therefore unmet"*. Both items are
marked CLOSED with their evidence; **the original finding text is preserved inline**, and the banner's
"original text preserved below unchanged" region — including Phase 4's own table — was **not** touched.

---

## 7. Premises checked — one brief statement needs a qualifier, nothing refuted

| premise | verdict |
|---|---|
| *"`grep -rn "irs.gov" crates/` finds no where-to-file or service-center reference anywhere"* | **True as stated, but the grep is not empty:** it returns **90** hits, all inside archived IRS form/instruction text under `crates/btctax-core/src/tax/fixtures/`. None is a where-to-file or service-center reference, and **zero** were on any surface a filer reads. The gap was real; the grep needs the qualifier. |
| Form 4868's where-to-file differs from the 1040's | **Confirmed by measurement** (§3) |
| `btctax extension` mails the 4868 on its own | **Confirmed** — `admin.rs` refuses to write into a directory holding a `manifest.txt`, citing the form's *"Don't attach a copy of Form 4868 to your return."* |
| the IRS corrected the 1040-ES addresses mid-2026 | **Not independently verifiable here** (no 1040-ES artifact in the repo). It is not load-bearing: the 1040 instructions carry a stronger warning in their own words, and that is what the shipped text quotes. |
| `hand_marks` is the natural home | **Declined, deliberately** — `hand_marks` is *"marks btctax deliberately did NOT make"* on **forms**, and the file already refuses to fold the §170(f)(11)(D) appraisal into it because *"it is not a mark"*. Addressing an envelope is not a mark on a form. Separate blocks, so the `COMPLETE BY HAND` block keeps meaning what it says. |
| `LIMITATIONS.md` is `include_str!`'d at `main.rs:582` | **Confirmed**, and it is byte-gated by `limitations_prints_the_shipped_doc_verbatim` |
| baseline `make gate` 3598 / 12 | **Confirmed** — 3609 / 12 after, i.e. exactly the eleven new tests |

---

## 8. The eleven new tests

| test | file:line |
|---|---|
| `a_guidance_text_missing_either_fact_or_the_pointer_is_named_and_a_complete_one_is_not` | `btctax-cli/src/lib.rs:572` |
| `normalizing_removes_chrome_but_not_words` | `btctax-cli/src/lib.rs:645` |
| `the_manifest_tells_a_refund_filer_where_to_post_it_and_on_what_it_depends` | `btctax-cli/tests/export_irs_pdf.rs:3221` |
| `the_manifest_tells_a_paying_filer_the_with_payment_column_is_theirs` | `…:3272` |
| `the_manifest_informs_about_record_retention_without_prescribing_a_window` | `…:3325` |
| `the_extension_report_says_where_to_post_form_4868_and_that_it_differs_from_the_return` | `btctax-cli/tests/extension.rs:705` |
| `limitations_carries_the_where_to_file_facts_and_the_retention_guidance` | `btctax-cli/tests/limitations.rs:245` |
| `no_shipped_text_carries_an_irs_service_center_address` | `xtask/src/service_center_check.rs:297` |
| `a_planted_service_center_address_reds_and_its_near_misses_do_not` | `…:331` |
| `the_state_set_covers_every_state_the_archived_tables_actually_use` | `…:519` |
| `every_bundled_year_of_the_two_forms_we_point_at_really_carries_its_address_table` | `…:602` |

Each of the four "does the surface carry it" tests names its mutation in its doc comment.

---

## 9. Residue, with owning phases

Filed in `FOLLOWUPS.md` (new section, *"From Phase 4's last two items"*):

- **FR-131** — the guard's scan covers `crates/**/*.rs` + `LIMITATIONS.md` only; a future filer-facing
  document that is neither would not be scanned. **Nit. Owning phase: ownerless residue.** Man pages are
  generated from clap doc comments, which *are* scanned as `.rs`, so the live gap is narrower than it
  sounds. Not derived today because *"shipped filer-facing text"* has no machine-readable definition here
  — an `include_str!` walk would pull in the archived extracts, which legitimately carry the tables.
- **FR-132** — a **refund or pay-online** return's packet contains no address table at all (the paying
  return holds the 1040-V's, the extension holds the 4868's). **Minor. Owning phase: the TY2026 packet
  read, before FILED.** The remedy is NOT a table — D-H forbids it — but a decision about whether the
  packet should carry the year's instructions page as an *authority artifact* with a refresh story. ★ The
  asymmetry runs the right way: the filer who risks a lost **payment** is the one already holding a table.

**Not residue, by decision:** the runtime-assembly blind spot (§5.4 — named in the source, no action
possible without data-flow analysis, and nothing in the tree does it); Phase 4's item **(iii)**, S8's
physical print rehearsal, which is the owner's to perform and not buildable.

**No secret-handling defect was found**, and nothing in this work touches secret material.

---

## 10. Gate numbers, as numbers

| gate | result |
|---|---|
| `make gate` | **3609 passed, 12 skipped, 0 failed** — exit 0 (baseline 3598 / 12) |
| clippy (`-D warnings`, inside `make gate`) | **0 warnings, 0 errors** |
| `cargo fmt --all --check` | clean, exit 0 |
| `make docs` | exit 0, **no tracked diff** |
| `xtask examples` golden | regenerated, **1 line** changed, both golden tests pass |
| git state | nothing committed, pushed, stashed or checked out; every mutation restored by `cp` and verified byte-identical |
