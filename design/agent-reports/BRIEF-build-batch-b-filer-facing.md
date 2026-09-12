# BRIEF — Batch B: the five filer-facing items, before the S8 print rehearsal

**Owner approved 2026-09-12.** These are done **before** the owner's physical print rehearsal (strategy
review S8) on purpose: three of the five came out of the journey walks, and the rehearsal is exactly when a
filer meets them. Fixing them first makes the rehearsal about the *process* rather than about cosmetic noise.

You are ONE opus agent in the **shared main tree**. Nothing is committed while you work. Five items, each
small. Do not widen scope.

---

## 0. Stop-and-report

**Stop and report rather than build on any premise here you can disprove.** Eleven briefs in this arc have
been refuted by measurement, six of them the coordinator's — and one of those was a follow-up entry whose
central claim was false because nobody ran the instrument. **Each entry below was re-verified live by the
coordinator today** (results in §1), but verify anything you rely on.

## 1. The five, all confirmed still live today

| item | site, re-verified 2026-09-12 | shape of fix |
|---|---|---|
| **FR-112** | `render.rs:1404-1410` prints `"net short-term: {} net long-term: {}"` from `r.st_net`/`r.lt_net` under the *"Federal tax attributable to crypto"* heading | a **label**, not arithmetic |
| **FR-113** | Form 8949 pages interleave Part I and Part II | page **composition** — the only non-trivial one, see §3 |
| **FR-115** | `spec/sections.rs:3004` (the entry's `:2937` has drifted — the FR-114 fold moved it) ends *"btctax fills from its own crypto lot engine alone"* | **one sentence** |
| **FR-116** | `cli.rs:479` *"Earlier years neither ask nor accept them"* | **one clause** |
| **FR-118** | `transcription_warnings.rs:312-314` — `money()` is `format!("${v:.2}")`, so a filer reads `$1200000.00` vs `$750000.00` | **one function**, see §2 |

## 2. ★★ FR-118 — the entry left a question open, and the coordinator has measured the answer

The entry says *"changing a filer-facing number format deserves its own decision about which formatter the
product uses everywhere."* **Do not unify the formatters. The answer is measured and it is no.**

There are two, and the split is load-bearing:

- `transcription_warnings.rs::money()` → `format!("${v:.2}")`. Feeds **prose warnings a filer reads.**
  Digit grouping earns its keep here — the whole content of the §163(h)(3)(B) warning is a comparison of two
  seven-figure numbers.
- `render.rs::fmt_money()` → `format!("{d:.2}")`, used **94 times** in a module whose own header reads
  *"Text rendering of CLI outputs … + FR10 CSV export"* and which imports `csv::Writer`. **Adding thousands
  separators there would inject commas into CSV fields** and corrupt the export (or silently force quoting,
  changing a contract the module says is stable).

So: change `money()` only, and **state that boundary in its doc comment** with the reason, so the next reader
does not "tidy up" the two into one. That comment is the deliverable as much as the format is.

★ Two existing tests assert the current spelling and will move:
`transcription_warnings.rs`'s `the_ceiling_warning_formats_both_figures_as_money`, and the
`btctax-tui-edit` consequence-4 kill from today's B3 round (it asserts the literal strings). **Update them to
assert the NEW format — do not loosen them to match anything.** A test relaxed to `contains("750")` would be
this repo's golden-regenerated-to-match-a-change failure.

## 3. ★★★ FR-113 is the one with real risk — row conservation

It reads as cosmetic (*"the IRS does not require grouping"*), and the fix is page composition. But
repagination is arithmetic on a **filed form**, and the entry's own measurement is the thing to protect:
*"with 39 rows over 4 pages all rows paginated correctly and none was dropped — verified by counting emitted
rows against those entered."*

**So the kill is row conservation, and it is mandatory:** a test that counts rows *entered* against rows
*emitted across all pages* and reds if they differ. Plant a dropped row → red. If no such test exists today,
that is the first thing you write, **before** touching pagination — then repaginate under it.

★ If grouping Part I and Part II turns out to require more than page ordering — a change to overflow
handling, the 14-row page limit, or the attachment sequence — **stop and report instead.** A dropped or
double-counted 8949 row is a wrong return, and this item is a Nit-grade UX improvement. It is not worth any
risk to the row set. Saying "this is bigger than it looks, here is why" is a successful outcome.

## 4. FR-115 and FR-116 — the wording items

- **FR-115**: replace the "lot engine" sentence. It is explaining a **refusal**, which is the moment wording
  has to land, and it must keep saying the true thing FR-110 settled: **btctax reports securities only as
  Schedule D line 1a/8a totals, never per transaction.** Do not weaken that claim while simplifying the
  prose.
- **FR-116**: `"accept"` is false read as *ingest* — `parse_return_inputs_toml` has no year gate and
  `cmd/tax.rs:260-293` stores the row unconditionally; only `screen_broker_reporting` later refuses it. Fix
  the clause (the entry suggests *"act on"*, or *"stored but refused as unread"*). ★ This doc comment
  **generates `docs/man/btctax-income-import.1`**, so `make docs` must be re-run and its diff committed.

## 5. FR-112 — follow the precedent that is already there

The entry notes the section **already** distinguishes: the very next line is labelled *"crypto ordinary
income (level)"*. `compute.rs:210-212` states which fields are crypto-attributable **deltas**
(`ltcg_tax`, `niit`, `total_federal_tax_attributable`) and which *"describe the WITH-crypto filing
position"*. Use that distinction in the label rather than inventing a new vocabulary. No arithmetic changes.

## 6. Scope, severity, mechanics

Five items. Anything else → `FOLLOWUPS.md` with an owning phase. Do not bundle params, do not touch the
fail-closed gates, do not start FR-120/123/128 (owned by the TY2026 port), and do not revisit FR-132 (closed
by owner ruling). Filer-facing prose must stay true: a **blank** is the normal case, and a sentence that
overstates what btctax does is worse than a clumsy one.

**Main tree. Do NOT commit, push, `git stash`, `git checkout` or revert anything** — a builder in this arc ran
`git stash` against its brief. Revert a mutation with a **cp backup**. **No subagents.** No `--no-verify`.
Never hand-count what a tool can count, and never quote a number or list from a `head`/`tail` view.

Baseline: `make gate` **3609 passed / 12 skipped**, fmt clean. `make docs` must end with no *unexpected* diff
(FR-116 will legitimately move a man page — commit that). Report numbers as numbers.

## 7. Your report — final action

Write `design/agent-reports/REPORT-build-batch-b-filer-facing.md`: per item, what changed and where
(`file:line`) and the **before/after filer-visible text**; FR-113's row-conservation kill with its pasted
red; FR-118's boundary comment quoted; which tests moved and how they still discriminate; refuted premises;
residue with owning phases; the literal gate numbers.

★ **Two agents' harnesses have now refused that write (FR-129, twice).** If yours does, say so **first** and
return the text — do not silently skip it.
