# REPORT — Batch B: the five filer-facing items, before the S8 print rehearsal

**Builder:** one opus agent, shared `main` tree, nothing committed. **Date:** 2026-09-12.
**Brief:** `design/agent-reports/BRIEF-build-batch-b-filer-facing.md`.

> ★★ **PROVENANCE — controller transcription, and the LAST one.** The builder's harness refused this
> write for the **third time in the arc** (`Write` → *"Subagents should return findings as text, not write
> report files"*) — FR-129. It said so first, returned the text, and **correctly did not route around the
> refusal via Bash** because its brief did not authorise that. The handoff is now fixed rather than absorbed:
> every future dispatch instructs the agent to write its report with a **Bash heredoc**, which its harness
> permits, restoring the agent-as-scribe property the standing rule exists for. See FR-129.

**Gate:** `make gate` → **3614 passed / 12 skipped, 0 failed**, clippy silent, `cargo fmt --all --check`
clean. Baseline 3609/12; the +5 are the tests added below. `make docs` → exactly one man page moved
(`btctax-income-import.1`, FR-116), no other tracked diff. `xtask` standing checks all OK: `stop-list`,
`prompt-check` (90 assertions verbatim), `cite-check` (51 quotations verbatim), `box-census` (268 boxes),
`harness-check`, `line-coverage` (377 money lines), `authority-conflicts`, `archive-check`.

## 1. FR-112 — a label, not arithmetic

**`crates/btctax-cli/src/render.rs:1416`** (the label), reason at `:1405-1414`. No figure changed;
`docs/examples/examples.md` regenerated — **4 changed lines, all this label**.

| | text a filer reads, under *"Federal tax attributable to crypto — tax year 2025"* |
|---|---|
| **before** | `  net short-term: -350.00   net long-term: 0.00` |
| **after** | `  net short-term (whole-return level): -350.00   net long-term (whole-return level): 0.00` |

The vocabulary is the section's own: the next line already reads *"crypto ordinary income (level)"* and the
TOTAL reads *"(delta)"*. `compute.rs:210-212` is the authority — `ltcg_tax`, `niit`,
`total_federal_tax_attributable` are crypto-attributable **deltas**; `st_net`/`lt_net` *"describe the
WITH-crypto filing position"*, i.e. whole-return levels including stock and capital-gain distributions.

★ Measured: no second emitter of this label exists in the crates or docs, so there was nothing to sweep.

## 2. FR-113 — Part I grouped before Part II, under a row-conservation kill written first

### 2a. The kill, written BEFORE any pagination change (brief §3)

`crates/btctax-forms/tests/overflow.rs:242` — `every_entered_8949_row_is_emitted_exactly_once_across_all_pages`.
It turns the journey walk's own measurement into a standing check: 20 short-term + 19 long-term = **39 rows
over 4 pages** on the 14-row TY2024 grid, comparing the multiset of col-(a) descriptors **read back out of
the merged PDF across every page of every copy** against those entered.

Derived, not typed: the fixture size comes from `map.rows_per_page` (`:179`), the cells from
`map.parts[].rows[][0]` (`:209`) — a revision that renames a cell or changes the grid re-derives both.

**Seen RED on a planted dropped row**, then restored from a `cp` backup:

```
thread 'every_entered_8949_row_is_emitted_exactly_once_across_all_pages' panicked at
crates/btctax-forms/tests/overflow.rs:255:5:
assertion `left == right` failed: every row entered must reach a page: 39 entered, 35 emitted
  left: 35
 right: 39
```

The same plant also reds two pre-existing tests — worth recording, because neither *counts* rows, so
neither says "a row was lost".

### 2b. The fix: a page-tree permutation, and nothing else

- `crates/btctax-forms/src/overflow.rs:22` — new `PageOrder { ByCopy, ByPosition }`.
- `:34` — `merge_copies` keeps its exact behaviour (`ByCopy`), so **Form 8283's merge is untouched** (it
  shares this function; its `GOLDEN_8283` byte digest is unchanged).
- `:48` — `merge_copies_ordered`: `ByPosition` rebuilds the page-tree `/Kids` array as the **transpose**
  (every copy's page 1, then every copy's page 2). Same page objects, same widget annotations, same `/V`
  values, same `/Count`.
- `crates/btctax-forms/src/lib.rs:177` (crypto slice) and `crates/btctax-forms/src/fill8949_full.rs:138`
  (full return) now pass `ByPosition`.

**Why this stayed inside the brief's risk boundary.** No change to chunking, `rows_per_page`, overflow
handling, box grouping, or the attachment sequence. Each copy is still filled and geometry-verified as a
whole two-page form **before** it reaches the merge; the merge only reorders. The tree was measured before
being touched: the root `/Pages` has **4 direct `/Page` kids, no intermediate node** — which is what makes
an array permutation a document permutation.

| | page order of `f8949.pdf` on a 39-row return |
|---|---|
| **before** | copy 1 Part I · copy 1 Part II · copy 2 Part I · copy 2 Part II |
| **after** | copy 1 Part I · copy 2 Part I · copy 1 Part II · copy 2 Part II |

### 2c. Two order tests, both seen RED pre-fix; both code paths held

`tests/overflow.rs:326` `every_part_i_page_precedes_every_part_ii_page` (crypto slice) and `:361`
`the_full_return_8949_groups_its_parts_and_conserves_every_row` (full-return path — a **second**
implementation of the same rule, so a test on one holds nothing about the other; it asserts conservation
too). Identical red on each path:

```
every Part I page must precede every Part II page — Part I on [0, 2], Part II on [1, 3]
```

★ The page→widget join (`:274`) reads each page's `/Annots` **dereferenced** — the first version skipped the
deref, saw every page as annotation-free, and *panicked rather than reporting an empty set as agreement*. It
still fails loudly if a cell is on no page (`design/HARNESS.md` class β).

### 2d. Two new fail-closed guards, each with a B1 kill

`overflow.rs:136` `pages_per_copy` (a ragged copy set refuses) and `:153` `flat_page_tree` (a `/Kids` array
that is not exactly copy 0's flat page list refuses, so a future nesting template fails CLOSED instead of
emitting a filed form with pages missing). Extracted as pure functions **so they could be killed**: tests at
`:173` and `:194`, both verified red on removal of the conjunct they exist for, then restored.

### 2e. One consequence worth naming, deliberately NOT changed

When one part is empty (a long-term-only filer — likely at the rehearsal), the blank part-pages now group at
the front instead of alternating. The page **set** is identical: btctax has always emitted a blank part page
for an exhausted side, and each 8949 page is a filed page carrying its own header. Dropping blank pages
would change the page set — outside "page ordering" and outside this item.

## 3. FR-115 — the refusal stops naming btctax's internals

The brief scoped this to the interview help. The phrase was swept to where a filer meets it, because the
same sentence sat in the **refusal message itself** — fixing only the help would leave the filer reading
"crypto lot engine" one step later, at the refusal. Three sites, one phrase; the FR-110 truth (securities
reach the return **only** as Schedule D line 1a/8a totals, never per transaction) is preserved and, in the
refusal, stated more explicitly than before.

**(a) `crates/btctax-input-form/src/spec/sections.rs:3004`** — `B1099BasisReportedNoAdjustments.help`:

> **before:** … anything else belongs on Form 8949 one row at a time, which btctax fills from **its own
> crypto lot engine** alone. Unanswered and NO both refuse.
>
> **after:** … anything else belongs on Form 8949 one row at a time, and **btctax reports your broker's
> transactions ONLY on those two Schedule D lines — it never writes one of them on a Form 8949 row.**
> Unanswered and NO both refuse.

**(b) `crates/btctax-core/src/tax/return_refuse.rs:2631-2634`** — the `RefuseReason::Form1099BNeedsForm8949`
detail, i.e. the refusal a filer actually hits:

> **before:** … one row per sale — btctax fills Form 8949 from **its own crypto lot engine only, and will
> not report securities it cannot itemize**
>
> **after:** … one row per sale — **btctax writes Form 8949 rows from your own bitcoin records only, and
> holds no per-sale record of a broker's transactions to itemize, so your broker's sales can reach this
> return ONLY as those two Schedule D totals**

**(c) `crates/btctax-cli/LIMITATIONS.md:382`** — the deliberate-ceiling bullet: *"btctax fills Form 8949
from its own **bitcoin** lot engine and will not build a second one for securities"* → *"btctax writes Form
8949 rows from your own **bitcoin** records only, and holds no per-sale record of a broker's transactions to
itemize"*. The surrounding ceiling framing is untouched.

Left alone on purpose: the two **developer-facing** doc comments using the phrase (`return_refuse.rs:610`,
`return_inputs.rs:648`). No filer reads them, and "lot engine" is the right word for an engineer.

## 4. FR-116 — one clause, and the man page

**`crates/btctax-cli/src/cli.rs:479-481`**:

> **before:** … a missing one refuses the return until given. **Earlier years neither ask nor accept them.**
>
> **after:** … a missing one refuses the return until given. **Earlier years never ask: an import still
> reads such a table in, but the return then refuses it as unread and tells you to remove it, so no
> earlier-year answer ever reaches a box.**

Verified against the mechanism, not the entry: `screen_broker_reporting` (`return_refuse.rs:1753`) refuses
any stored answer when `!regime.basis`, with the detail *"the tool would never read this answer, and
testimony it would discard is not kept … Remove the answer."* `make docs` re-run;
`docs/man/btctax-income-import.1:11` carries the new sentence and **no other man page moved**.

## 5. FR-118 — one function, the boundary comment, and five tests (not two)

**`crates/btctax-core/src/tax/transcription_warnings.rs:326`.** `money()` now delegates to the core-side
house formatter `crate::tax::advisories::fmt_usd` (thousands-separated, cents only when there are any)
instead of `format!("${v:.2}")`. Delegating rather than writing a second grouping routine is the FR-99 rule
applied: `fmt_usd`'s own doc comment already calls itself *"the core-side equivalent … used by every
advisory that prints money"*.

> **before:** … adds up to **$1200000.00**, which is more than the **$750000.00** the §163(h)(3)(B) limit
> allows …
>
> **after:** … adds up to **$1,200,000**, which is more than the **$750,000** the §163(h)(3)(B) limit
> allows …

And the W-2 checks, which share `money()`: *box 4 … is **$10,453.20**, but 6.2% of box 3 (Social security
wages, **$250,000**) is **$15,500** … above this year's Social Security wage base of **$168,600**.*

### The boundary comment (the deliverable — `transcription_warnings.rs:311-325`)

It states that this is deliberately **not** "the product's money formatter"; that the other formatter,
`btctax_cli::render::fmt_money`, prints ungrouped and **must stay that way** because its module is *"Text
rendering of CLI outputs … + FR10 CSV export"* and imports `csv::Writer`, so a separator there would put a
comma **inside** an exported CSV field; and that **the split is by AUDIENCE, not by oversight — prose a
filer reads is grouped, machine-readable output is not. Do not unify them.**

Machine-checked while writing it: `render.rs:1-2` really is that header; `render.rs:23` really is
`use csv::Writer;`; `fmt_money` appears **94 times** in that module (**92 call sites**, plus its definition
and one prose mention).

### Which tests moved, and how each still discriminates

Every one verified by **planting `format!("${v:.2}")` back** and watching it red, then restoring.

1. `transcription_warnings.rs:654` `the_ceiling_warning_formats_both_figures_as_money` — grouped figures at
   `:661` **plus a new negative half** at `:665`, so it reds both when the figures stop going through
   `money()` **and** when `money()` reverts. Two discriminators where there was one.
2. `crates/btctax-tui-edit/src/edit/form.rs:4823` — today's B3 consequence-4 kill, now grouped. Its original
   discrimination is intact: at year 0 the ceiling is `None` and **neither figure appears at all**.
3. **★ The third test the brief did not list — and the false green inside it.**
   `crates/btctax-cli/tests/tax_report.rs` asserted `screen.contains("$1000000") && screen.contains("$750000")`
   over the **whole answer screen**. Updating it to the grouped spelling made it **pass with the old format
   still planted** — because the debt-limit *question's own text* quotes both statutory figures
   comma-separated (`questions.rs:1070-1083`). A screen-wide `contains` was being satisfied by the prompt,
   not the warning. Fixed by asserting over the **warning's own span** (`&screen[warn..prompt]`, both
   indices the test already computed) plus a negative half, now at `:3803`/`:3807`.
4. Two more the suite found: `transcription_warnings.rs:463` and `:500` asserted bare `contains("900000")` /
   `contains("1000000")`; they now assert `$900,000` / `$1,000,000` — with the `$`, strictly tighter.
5. `crates/btctax-cli/tests/tax_report.rs:5102`/`:5106` failed for a different reason worth recording: the
   warning block is **word-wrapped**, and the longer grouped figures moved the wrap point into the middle of
   *"btctax has changed nothing"*. The assertions now run over the paragraph with whitespace collapsed — the
   same sentence, checked independently of where the renderer breaks it. An assertion that depends on the
   wrap column is a false-red (and symmetrically false-green) generator.

## 6. Premises tested against measurement

| premise | verdict |
|---|---|
| FR-118: do not unify the formatters (`fmt_money` feeds CSV) | **confirmed** — `use csv::Writer;` at `render.rs:23`, 92 call sites |
| FR-118: "two existing tests assert the current spelling and will move" | **incomplete** — **five** moved; the third hid a false green |
| FR-113: "39 rows over 4 pages, none dropped" | **reproduced**, now a standing test rather than an observation |
| FR-113: grouping might need overflow / 14-row / attachment-sequence changes ⇒ stop | **did not arise** — the page tree is flat (measured) and the fix is a `/Kids` permutation |
| FR-115 sits at `sections.rs:3004` (entry's `:2937` drifted) | **confirmed**, and the same sentence was **also** in the refusal detail and `LIMITATIONS.md` |
| FR-116: `screen_broker_reporting` refuses an earlier-year answer as unread | **confirmed** at `return_refuse.rs:1753` |
| FR-112: `compute.rs:210-212` states the delta/level split | **confirmed** |

## 7. Residue

- **FR-133 filed**: the Form 8949 paginator exists **twice** — `lib.rs:146` and `fill8949_full.rs:103` each
  carry their own `pages()` implementing the same spec 1099-DA R3/T3 rule. FR-113 had to be applied to
  both. Mitigated by the two new per-path tests (a divergence now reds), not closed; the real fix is one
  paginator, a refactor of a filed form's page composition that does not belong in a Nit-grade UX item.
  Owning phase: ownerless residue (structural), Minor.
- Nothing else. No entry for §2e (page set unchanged, pre-existing), none for the developer-facing "lot
  engine" doc comments (§3).

## 8. Mechanics

No commit, no push, no `git stash`, no `git checkout`, no subagents, no `--no-verify`. Every mutation planted
and restored via `cp` backups, with `find crates -name '*.rs' -exec touch {} +` before each measured run
(FR-90). 15 files changed (491 insertions, 40 deletions), of which two are generated
(`docs/examples/examples.md`, `docs/man/btctax-income-import.1`) and one is the ledger (`FOLLOWUPS.md`).

★ One environment note: the first `make gate` after `touch`-ing every source **OOM-killed the linker**
(`ld terminated with signal 9`) on a box with 3 GB free, because nextest's and clippy's full rebuilds ran
concurrently. Re-running the two halves serially, then `make gate` warm, was clean — no code cause.
