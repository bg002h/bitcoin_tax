# BRIEF — wave 3. Two parcels. **Owner: "Plan for both."**

The owner was asked which SALT election their prior-year Schedule A made — income taxes, or the §164(b)(5)
general-sales-tax election — because it decides whether FR-196's wall is real for them. They answered
**"Plan for both."** ★ That is the right call and it is also the better engineering answer: **a product that
ASKS the filer beats one that needs the developer to know the filer's facts.** So neither path is a special
case; both are supported, and nothing in this wave may branch on who the owner happens to be.

Standing rules for both parcels: own worktree, `CARGO_TARGET_DIR=<worktree>/target-<letter>`, **never
`/tmp`** (32 GB tmpfs). **FOREGROUND every command** — FR-175: two agents have stalled by backgrounding a
gate, the second *with the prohibition in its brief*, and the report is exactly what a stall destroys.
`make check` excludes `cargo fmt --all --check`; run both. Capture once, grep. **Do not commit, do not
push.** No subagents. ★★ **Nine briefs in this arc were refuted by their implementer, six of them mine — if
you can disprove a premise, STOP and report it.** It has paid every time.
★ Two DIFFERENT concurrency failures exist and must not be conflated (FR-176, FR-223): an
`ld.lld: undefined hidden symbol … from a stale .rlib` is a corrupted incremental cache, cured by
`cargo clean -p <crate>`; a `signal: 9, SIGKILL` is **memory** from nextest and clippy running concurrently
beside sibling agents, cured by running the two halves serially.

---

## J — the state-refund path, end to end. FR-221, FR-196's refusal removal, FR-222, FR-196a.

**OWNS:** `crates/btctax-core/src/tax/{return_refuse.rs, return_inputs.rs, state_local_refund.rs,
questions.rs, advisories.rs}`, `crates/btctax-cli/src/cmd/tax.rs`, `crates/xtask/src/toml_schema.rs`.

### FR-221 first — the exit that is never offered

`design/forms/extract/i1040gi--2025.txt:41882-41887`, verbatim:
> *"**None of your refund is taxable** TIP if, in the year you paid the tax, you either **(a)** didn't
> itemize deductions, or **(b) elected to deduct state and local general sales taxes instead of state and
> local income taxes**."*

`RefuseReason::StateAndLocalRefundWorksheetNotComputed` offers only limb (a). **Ask limb (b).** A filer who
elected sales taxes in the prior year gets a **blank** Schedule 1 line 1 — blank, not zero — and no refusal.
★ `schedule_a.salt_use_sales_tax` already exists **for the current year**; the TIP turns on the **prior**
year's election, so this is a new fact, not a reuse. Name it so the two cannot be confused.
★★ **Plan for both** means: whichever limb the filer answers, the return completes — one via the worksheet,
one via the exit. Neither may require the other's inputs.

### Then the refusal removal, per `REPORT-wave2-E.md` §7 — follow it, it is precise

Keep the variant, **narrowed to three states**; replace the *condition* with a call to
`state_local_refund::figure(...)` and refuse only on its own refusals. Its five carried obligations, in its
stated order of importance: (1) a **new** `Pub525ItemizedDeductionRecovery(Pub525Exception)` reason quoting
that revision's own sentence — *"the form FORBIDS this worksheet for you"* is a different fact with a
different remedy from *"btctax does not compute it"*; (2) leave the reachability fixture at `:10459` alone
but **check** it still refuses, via `FactsNotCollected`; (3) **two testimonies about one line must REFUSE** —
a present `state_local_refund` block and a hand-attested `sch1.state_refund_taxable` are two answers to
Schedule 1 line 1; (4)–(5) per the report. ★ Preserve the `Option` all the way into the printed line: blank
must never collapse to zero.

### FR-222 — a box with no reader

`g_1099[].box2_state_refund` feeds nothing. Wire it into the worksheet's input, **or** state in the source
why a filer-entered figure is preferred over the reported box. ★ *"An unread computed value is not thereby
correct."* Also: taxcalc's `e00700` is verified present, so note in the source that this output is
oracle-checkable once `GoldenInputs` gains the axis — do **not** build that axis here.

### FR-196a — a derived document was RIGHT and the code was WRONG

`cmd::tax` forces `CarryProvenance` to `user` at **five hand-listed sites**; FR-196's field was a silent
sixth, while `xtask toml-schema` **derives** the *"forced to `user`"* annotation from the field path — so the
regenerated schema documented a normalisation the code did not perform. ★★ Derive the forcing from the same
path predicate the generator uses, **or** assert the two agree. A test joining them is the deliverable; the
sixth site is already closed.

**B1 for J:** the sales-tax-election filer completes with a **blank** line 1 (not zero); the income-tax
filer with facts completes via the worksheet; a Pub. 525 exception refuses under the **new** reason quoting
the form; two testimonies refuse; and a planted sixth un-forced field reds the FR-196a test.

---

## K — FR-218: the 8949 emits a blank Part I page per copy. **Owner-found, on paper.**

**OWNS:** `crates/btctax-forms/src/fill8949*.rs`, `crates/btctax-forms/src/overflow.rs`, and the 8949 tests.

The owner printed a packet and reported *"the first page of 8949 is printed twice."* Confirmed: pages 1 and
2 of a 4-page 8949 have **identical text layers**. Cause, `fill8949_full.rs:122`:
`let n_copies = st_pages.len().max(lt_pages.len()).max(1);` — the copy count is the **max** of the two
parts' page counts and each copy then fills **both** parts via `st_pages.get(k).unwrap_or(&empty)`, so a part
with **zero rows still emits a page**, once per copy. A long-term-only filer needing two Part II pages gets
**two blank Part I pages**, each carrying their name and SSN.

**The authority** (`design/forms/extract/i8949--2024.txt:422-424`):
> *"You don't need to complete and file an entire copy of Form 8949 (Parts I and II) if you can check a
> single box to describe all your transactions. In that case, complete and file **either Part I or II** and
> check the box that describes the transactions."*

**Fix:** paginate the two parts **independently** — `lt_pages.len()` Part II pages and `st_pages.len()`
Part I pages — not `max()` copies of both. The output already groups by part (`:139`, FR-113), so only the
page manufacture is wrong.
★ **Preserve what is already correct**, verified by the controller: per-copy totals are right (the form's
line 2 says *"Enter each total here"*), and the two Part II pages' subtotals roll up to Schedule D exactly
(352655+468635 = 821290; 86361+102520 = 188881; 266294+366115 = 632409). Your change must not disturb that
roll-up — assert it.
★★ **B1:** a long-term-only filer's 8949 contains **zero** Part I pages; a short-term-only filer **zero**
Part II pages; a mixed filer both. Plant a zero-row part and watch a page still appear. ★ And nothing this
project owns caught this — the golden is byte-stable, every field reads back, both oracles agree, the
roll-up closes. **It was a defect only when someone held the paper**, so the kill has to be structural
rather than visual.
