# BRIEF — exercise runbook steps 16 & 17 on PRIOR years, where the answer is already known

**Tier:** opus. **Isolation:** worktree (you will add archive files). No subagents.
**Owner ask:** *"testing our port apparatus for tax year 2023, 2022, etc and testing against released tax
sample data"* — reshaped to the cheap form, below.

## 0. The one question

> Runbook steps **16** (adjudicate a witness disagreement) and **17** (decide whether a moved line is the
> same line) are called load-bearing and have been exercised **exactly once** — `f6251/2025`'s 37→43
> collision, which took two review rounds to hold by a type. **What does the port machine do when handed a
> real moved line, a real retired line, and a real reused line number?**

## 1. Why prior years, and why this is nearly free

- **`form-delta` takes arbitrary stems**, not bundled years: `cargo run -p xtask -- form-delta <old> <new>`.
- The archive already fetches prior-year documents — `design/forms/2019/f1099sa--2019.pdf.txt` cites
  `https://www.irs.gov/pub/irs-prior/f1099sa--2019.pdf`. **That URL pattern is the whole enabler.**
- So this needs **fetch → extract → extract-geometry → run the tool**. No params, no `YEAR.toml`, no
  bundling, no `Stem` variant, no compute.
- **Prior years have what TY2026 does not: FINAL documents and working oracles.** TY2026 is drafts and
  OTS-2026 will not exist until ~January (that is S7's risk). OTS **2024** and **2025** trees are installed
  at `/home/bcg/OpenTaxSolver{2024_22.07,2025_23.06}_linux64`; taxcalc covers back years natively.

★★ **DO NOT BUNDLE a prior year.** FR-163 and the S9 precedent (TY2017 deleted by owner ruling) say every
old year becomes a row in every glob-walking gate forever. This is a **rehearsal**: archive what you need,
measure, and leave the bundle untouched. Your report is the artifact, not a new year package.

## 2. Method — let the tool choose its own test case

Do **not** pick a pair from memory of line numbers, and do not trust mine. Instead:

1. Archive several prior-year revisions of a handful of high-traffic forms (the 1040 itself, Schedule 1,
   Schedule 3, Schedule 8812, Schedule D, Form 8949 are the obvious candidates) across roughly **2020–2023**
   — the window that contains the pandemic-era lines that appeared and then vanished.
2. Run `form-delta` over consecutive pairs.
3. **Pick the pairs whose verdict is non-trivial**, one of each shape if you can find them:
   - a field that **moved** to a different line (step 17's real question),
   - a line that was **retired** (present in the old revision, gone in the new),
   - ★ a line number **reused for a different quantity** — the 37→43 shape, the dangerous one, because
     reusing the old cross-reference substitutes one quantity for another and is taxpayer-adverse.
4. For each, answer: **what does the tool say, and is it right?** Read the two extracts and adjudicate
   against the documents, not against the tool's output.

## 3. What to report — the number Fable's §6 asked for

- Per pair: the tool's verdict, whether it was correct, and **what a human had to do** that the tool could
  not. That is the cost of steps 16/17, which nothing has measured.
- Every place the tool was **silent** where it should have spoken, or **spoke wrongly**. A moved line
  reported as a rename, or a reuse reported as a renumber, is the FR-114-class defect at the port layer.
- Whether `extract-geometry` resolves prior-year PDFs at all (they are older AcroForms; some may have none —
  say so plainly rather than working around it).
- ★ If a prior-year form has **no AcroForm at all**, the label axis is unavailable and `form-delta`'s
  `labels_available` should be false. **Verify that it says so rather than silently comparing nothing** —
  `no_archived_pair_reports_a_clean_verdict_from_zero_comparisons` exists for exactly this fear.

## 4. Out of scope

No bundling. No params. No `Stem` additions. No compute changes. Do not touch `crates/` **except** if you
find a genuine defect in `form_delta.rs`/`form_geometry.rs`, in which case **report it, do not fix it** —
FR-165 landed in those files hours ago and a second edit on top of it is not this task.
Do not run the oracles here; this probe is about **documents**, not figures.

## 5. Stop-and-report

Six briefs in this arc were refuted by their implementer, three of them mine, all today. If you can
disprove anything above — that `irs-prior` serves the revisions you need, that `form-delta` accepts
arbitrary stems, that prior-year forms carry AcroForms — **stop and report it.**

## 6. Working rules

- Own worktree; `CARGO_TARGET_DIR=<your-worktree>/target-probe` (covered by `.gitignore`'s `target-*/`).
  Never a target dir in `/tmp` — 32 GB tmpfs, one filled it and killed a running test.
- **Run the gate in the FOREGROUND** (`make check`, ~20s). ★ FR-175: an agent that backgrounds its gate and
  ends its turn never writes its report. Do not do that. Capture output once to a file and grep it.
- `make check` does **not** include `cargo fmt --all --check`; run `cargo fmt --all` if you touch Rust.
- **Do not commit, do not push.** Leave the worktree dirty and list every file you added or changed.
- Verify every fetched PDF's sha256 into its `.pdf.txt` note, exactly as the existing notes do. **A changed
  hash means the IRS revised the document — that is a change to the authority, never silently absorbed.**

## 7. Deliverable

Final action: Bash heredoc (not the `Write` tool) to:

    design/agent-reports/RECON-prior-year-delta-probe.md

Return a short summary plus that path.
