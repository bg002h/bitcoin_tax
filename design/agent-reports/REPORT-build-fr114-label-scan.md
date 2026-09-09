# REPORT — build FR-114: extend R15's ledger-word ban to `btctax-input-form` field LABELS

**Repo:** `/scratch/code/bitcoin_tax` · **Tree:** shared `main`, uncommitted · **HEAD when started:** `1e6095d7`

---

## ★★★ STOP — the brief's fact 1 is REFUTED by the built checker

> **Fact 1.** *"Zero labels red today. The extension lands green. Measured by replicating the checker's
> exact split … over every `label:`/`help:` string literal in `crates/btctax-input-form/src`."*

**One label reds.** The extension is built exactly as specified, and this is its output on the committed
tree:

```
$ cargo run -q -p xtask -- stop-list
xtask stop-list: R15/R9: a return-registry prompt or a form_spec field LABEL asks a LEDGER question — those belong to `reconcile`, and the interview never re-asks one:
  form_spec Sa1099s/Sa1099Box4Fmv: says "fmv"
```

The label is `"4 FMV on date of death"` — `crates/btctax-input-form/src/spec/sections.rs:3329`, built by
`doc_money!(FieldId::Sa1099Box4Fmv, sa_1099, "4 FMV on date of death", …)`.

**It is a verbatim transcription of the box's printed caption**, checked against the archived primary
source rather than asserted:

```
$ grep -n -i "FMV" design/forms/extract/f1099sa--2025.txt | head -2
12:RECIPIENT’S name          3 Distribution code     4 FMV on date of death         Internal Revenue
39:RECIPIENT’S name          3 Distribution code        4 FMV on date of death                   Recipient
```

So this is a genuine, load-bearing collision between two standing rules, not a defect in either side:

| rule | says |
|---|---|
| `CLAUDE.md` — *Transcribe IRS forms, never paraphrase them* | the label must say **exactly** what box 4 says |
| R15/R9 — the ledger-word ban | no interview question may say `transfer` / `lot` / `fmv` |

Both cannot hold on `Sa1099Box4Fmv` as things stand. **I stopped rather than work around it.** I did not
add an allow list (the brief rejects one), did not reword the label (the transcription rule protects it),
and did not weaken the ban.

### ★ Why fact 1's measurement missed it — and why that matters more than the one label

The measurement scanned `label:` string **literals**. Most labels are not literals: they are macro
arguments (`doc_money!`, `doc_text!`, …). Measured against the derived walk:

| | |
|---|---|
| labels in `form_spec()` | **279** |
| distinct `label: "` literals in `crates/btctax-input-form/src` | **63** |
| walked labels a `label:` literal scan can see | **70** |
| walked labels **invisible** to it | **209** (75%) |
| is the reddening label in the blind 209? | **yes** |

That is this repo's dominant instrument failure — *green because it never ran over the region that
mattered* — and it is the reason the brief's own premise, and the FR-114 premise it retracted, both
landed wrong. Fact 3 was right to retract *"Covered lots"*; the real instance was simply somewhere the
literal scan could not look.

Facts **2**, **3** and **4** all verified and hold (see `## Counts`).

---

## What changed

One file: `crates/xtask/src/r15_stop_list.rs`. Nothing else is edited. **Nothing is committed.**

1. **`const FORM_SPEC_LABEL: &str = "form_spec "`** — the one place the finding label and the floor
   guard's prefix filter agree, so they cannot drift.

2. **`fn section_labels(&[Section]) -> Vec<(String, String)>`** — the walk, taking the sections as a
   parameter so a planted defect can reach it (`form_spec()` is `&'static` and cannot be mutated).
   Derived from the **type**: `Section.fields` → `Field.label`, labelled `form_spec <SectionId>/<FieldId>`
   so a finding names the question rather than a line number. No hand list of sections, no regex over
   source text.

   Its doc comment carries the **boundary statement** the brief requires (FR-99 option 3): `Field.help`
   is **not** scanned, why (the label *is* the question; `help` explains it in the filer's documents' own
   words), and the residue named — *a ledger question phrased inside help text is not caught.*

3. **`fn form_spec_labels()`** — `section_labels(btctax_input_form::form_spec())`.

4. **`run()`** — walks the labels, extends the scanned set, and feeds it through the **existing**
   `ledger_words_in_registry_prompts`. No second copy of the ban. Two guards, in the house style:
   - a **loose floor** (`sections_walked < 20 || expected_labels < 100`) so a walk that finds nothing
     cannot pass by finding nothing;
   - an **equality** — `scanned_labels != expected_labels`, both derived from `form_spec()` — in the same
     shape as the existing `scanned_rendered != rendered` guard, so dropping the extend or narrowing the
     walk reds instead of going quietly blind.

   The comment states what the equality **cannot** catch (a section leaving `SECTIONS` moves both sides)
   and names, from measurement, what does catch that.

5. **The success string** now reports registry prompts, field labels and sections walked — it reports
   what it actually scanned.

6. **Tests** — one existing kill extended, two new (below).

---

## The kill

Four observations, each restoring the tree by file copy afterwards (never `git checkout`).

### K1 — the planted ledger question in a field LABEL reds, and names the field

New test `the_label_scan_reds_on_a_ledger_question_in_a_field_label_and_names_the_field`. It builds a real
`Section` carrying two real `Field`s — one labelled `"Covered lots — bought on this venue on/after
2026-01-01"`, one labelled `"Which lot did you sell?"` — walks it with `section_labels` and judges it with
`ledger_words_in_registry_prompts`. **It calls both instruments; it re-implements neither.** The two sit in
one section so the assertion is that the scan *discriminates between them*, not merely that it fires when
handed a bad label alone.

**Observed RED** by mutating the walk to read `f.help` instead of `f.label`:

```
thread 'r15_stop_list::tests::the_label_scan_reds_on_a_ledger_question_in_a_field_label_and_names_the_field' panicked at crates/xtask/src/r15_stop_list.rs:864:9:
assertion `left == right` failed: the ledger question must be the ONLY finding, and it must name the section and field that asked it — a line number does not tell a reader which question a filer is shown
  left: []
 right: ["form_spec BrokerReporting/BrokerNoncovered: says \"lot\""]
```

**Green after restoring:** `test r15_stop_list::tests::the_label_scan_reds_on_a_ledger_question_in_a_field_label_and_names_the_field ... ok`

### K2 — the floor guard reds when the extension is dropped

Mutation: delete `scanned.extend(form_spec_labels());` from `run()`.

```
xtask stop-list: 0 of 279 form_spec field labels were scanned — the LABEL is the question a filer is asked, and a registry-only scan reports success over a region it cannot see
```

Restored identical (`diff` clean).

### K3 — the plural/singular pin reds on the naive widening

Added to the existing kill `each_r15_grep_reds_on_a_planted_line_and_not_on_its_near_miss` (extended, not
duplicated — the `plot`/`allot`/`slot`/`transferable` near-misses were already there): `"Covered lots …"`
must stay green while `"Which lot did you sell?"` must red.

Mutation: `const BANNED: &[&str] = &["transfer", "lot", "lots", "fmv"];` — the naive widening that HEAD's
own commit message warns would break correct IRS vocabulary.

```
thread 'r15_stop_list::tests::each_r15_grep_reds_on_a_planted_line_and_not_on_its_near_miss' panicked at crates/xtask/src/r15_stop_list.rs:797:9:
`lots` is the broker's own §6045 cohort word and is NOT the ledger's `lot`
```

A coarser mutation (word-boundary split → `contains`) also reds, but on the pre-existing near-miss
assertion first, so K3 uses the widening — it isolates the new pin.

### K4 — B1a: the fixture is half the checker

New test `the_real_form_spec_labels_are_the_scanned_set_and_are_clean`. K1's fixture is hand-built by
necessity; this holds the other half. It asserts the real walk equals the count derived from the type, is
non-trivially large, and — the point — reads the discriminating near-miss **out of the shipped registry**
(`form_spec BrokerReporting/BrokerCovered`) rather than from a copy typed into the test. If
`BROKER_FIELDS` ever moves to the singular, this reds.

**This test is currently RED**, on its last assertion (`no shipped field label asks a ledger question`) —
that is the fact-1 refutation above, not a defect in the test.

---

## Counts

All measured, none hand-counted.

| quantity | value | how |
|---|---|---|
| `form_spec()` sections walked | **30** | derived walk, printed and counted |
| `form_spec()` field labels walked | **279** | derived walk; independently equals the coverage KAT's pin `coverage.rs:836` (`assert_eq!(field_count, 279)`) |
| labels that red | **1** | `form_spec Sa1099s/Sa1099Box4Fmv: says "fmv"` |
| `SectionId` variants | **30** | `awk` over the enum body |
| entries in `SECTIONS` (`spec/mod.rs:25`) | **30** | grep over the const body |
| `Section` consts defined | **30** | 26 in `sections.rs` + 4 in `registries.rs` |
| `label:` literals in the crate src | **63** distinct; **70** walked labels match one | independent Python replication of the checker's split |
| walked labels invisible to a literal scan | **209** | set difference |

**Fact 2 verified independently** — replicating the checker's exact split over every `label:`/`help:`
literal: 0 of 63 label literals red; exactly 3 help literals red, at `sections.rs:2483` (`lot`), `:2498`
(`transfer`), `:2937` (`lot`). All three untouched, per the brief.

**Fact 3 verified** — `"Covered lots — …"` → `[]`, `"Which lot did you sell?"` → `["lot"]`. Now pinned by K3.

**Fact 4 verified** — `btctax-input-form` at `crates/xtask/Cargo.toml:24`; `form_spec() -> &'static
[Section]` at `spec/mod.rs:24`; `Section.fields: &'static [Field]`, `Field.label: &'static str`
(`seam.rs:686`, `:704`).

**Subcommand name confirmed from source**, not assumed: it is **`stop-list`**, not `r15-stop-list`
(`crates/xtask/src/main.rs:273`).

### Gate

`cargo fmt --all --check` — **clean**.
`CARGO_TARGET_DIR=target-clippy cargo clippy -p xtask --all-targets --all-features -- -D warnings` —
**clean**.

`make gate`:

```
     Summary [  22.358s] 3560 tests run: 3558 passed, 2 failed, 12 skipped
        FAIL [   0.003s] xtask::bin/xtask r15_stop_list::tests::the_real_form_spec_labels_are_the_scanned_set_and_are_clean
        FAIL [   0.017s] xtask::bin/xtask r15_stop_list::tests::the_r15_stop_list_holds_on_the_committed_tree
make check: FAILED
```

**Both failures are the same finding**: `form_spec Sa1099s/Sa1099Box4Fmv: says "fmv"`. No other test moved.
Clippy ran clean inside the same gate.

**⚠ Do not commit as-is.** The tree is red, deliberately, because the red *is* the finding. The
resolutions I can see are all owner/controller decisions I was not authorized to make:

1. **Move the ban, mechanism-derived.** A label that is a transcribed document-box caption is not the
   interview *re-asking* anything. There is a real shape available — the `doc_*!` macros produce exactly
   these labels, and every one begins with its box number (`"4 FMV on date of death"`, `"13 Bartering"`,
   `"2 Earnings on excess cont."`). Deriving the exemption from *how the label is produced* is the
   `CLAUDE.md`-shaped fix, and is categorically different from the per-site allow list the brief rejects.
   But it changes what R15 means across 209 labels, which is a design decision.
2. **Scope the ban to authored question labels only** and say so — i.e. walk the sections but exclude
   document-transcription fields by the same derived mechanism.
3. **Change the label** — rejected on sight: it would violate the transcription rule and the primary
   source above.

---

## The SECTIONS exhaustiveness question

**Answer: no single guard proves `SECTIONS` exhaustive over `SectionId` — but the gap is much narrower
than "nothing", and three separate mechanisms cover the plausible failures. I measured all three rather
than reading names.**

| the mistake | what reds | measured? |
|---|---|---|
| a new `SectionId` variant is added | **`E0004` non-exhaustive patterns** at `crates/btctax-input-form/src/apply.rs:248` — `row_depth`, an `_`-free match whose own comment says *"Exhaustive so a new `SectionId` is a compile error here"* | ✅ planted `SectionId::OrphanVariant`; compile error |
| a `Section` const is written but not listed in `SECTIONS` | **`dead_code`** — the `sections` module is private, so an unlisted `pub const` is unused, and `make check`'s clippy runs `-D warnings` | ✅ planted `ORPHAN_SECTION`; `error: constant ORPHAN_SECTION is never used` |
| a section is **dropped** from `SECTIONS` | **three `btctax-input-form` tests**, incl. `spec::coverage::every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt` (`coverage.rs:797`), `spec::tests::declarations_section_delegates_every_decl_and_the_question_map_is_total`, and `apply::tests::the_fields_that_record_an_answer_are_exactly_the_three_registries_images` | ✅ deleted `sections::HOME_SALE,` from `SECTIONS`; 3 failed / 77 passed |

**The residual hole, stated precisely.** A `SectionId` variant that already exists, given a `Section` that
is listed nowhere, is caught only by `dead_code` — which depends on the `sections`/`registries` modules
staying private. Make either module `pub`, or `pub use` the consts, and that guard evaporates silently.
And a new section wired to leaves that are *already covered or already `EXEMPT`* clears the coverage KAT
too. There is no `SectionId::ALL`, no `strum`, and no direct assertion that
`SECTIONS.len() == <number of SectionId variants>` — the totality is assembled from three side effects
rather than stated once.

**Reported, not fixed** — pre-existing and out of this task's scope, per the brief. Suggested follow-up
wording: *derive `SECTIONS` from an `ALL` const guarded by an `_`-free match on `SectionId`, or assert
`form_spec()` covers every variant, so the three side effects become one stated invariant.*

★ One correction of my own: my first draft of the `run()` comment asserted *"every match on `SectionId`
has a `_` arm."* That was false — `apply.rs:248` has none. I found it by measurement and rewrote the
comment before finishing. It would have been a false sentence in source, which is the class this repo reds
on hardest.

---

## Anything I could not do

- **The task cannot land green** without a decision I was not authorized to make. See the three options
  under **Gate**. Everything else the brief asked for is built, formatted, clippy-clean and kill-tested.
- **Two files changed in the shared tree that I did not touch.** `git status` was clean at
  `1e6095d7` when I started; it now also shows `M FOLLOWUPS.md` and `M crates/btctax-cli/LIMITATIONS.md`,
  carrying an FR-111 fold (Form 8949 box re-derivation). Another writer in the shared tree, presumably the
  controller. I left both strictly alone — flagging it only so the controller does not mistake them for
  mine when staging.
- No subagents were spawned. Nothing was committed, added, pushed, checked out, or branched. Every
  mutation was restored by file copy from a scratchpad backup, and the final `r15_stop_list.rs` was
  `diff`-verified identical to the intended state after the plant/measure/restore loop.
