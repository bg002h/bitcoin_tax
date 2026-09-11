# BRIEF — folding the B3 whole-branch review

**Round:** B3, the last review before an irreversible action. **Report:**
`design/agent-reports/2026-09-11-b3-whole-branch-review.md` (persisted verbatim, `587d7a9c`).
**Controller ledger:** `design/agent-reports/2026-09-11-b3-whole-branch-review-VERIFICATION.md` (`22ed5e2a`)
— every citation in the report was re-derived from source before this brief was written; the report stands
at **1C/2I/0M/1N**.

You are ONE opus implementer working in the **shared main tree**. Read the report and the ledger in full
first. The ledger tells you which claims are already machine-verified, so you do not re-derive them — spend
your budget on the fix and its kills.

---

## 0. The instruction that outranks the rest

**Stop and report rather than build on any premise in this brief you can disprove.** Four briefs in this arc
have been refuted by measurement — two written by this coordinator — and every time, the implementer
stopping was what kept the wrong thing out of the product. The fix *directions* in §2 below are the
coordinator's reading, not findings; if measurement says a direction is wrong, say so and stop rather than
implementing it. A refuted premise is a successful outcome for this dispatch.

★ In particular, §2's C-1 direction rests on a claim you should test before building on it: that stamping
only at the commit gate would leave consequences 2–5 open **and** leave a year-0 sentence in the answer log.
Measure it.

## 1. Mechanics — not negotiable

- **Main tree, `/scratch/code/bitcoin_tax`.** Do **not** commit, do **not** push, do **not** `git checkout`
  or `git stash` anything. The coordinator commits your work after machine-checking it.
- **Do not spawn subagents.** A subagent's edits have raced this tree before.
- Validation: `make check` (~6s + suite) and `cargo fmt --all --check`. Baseline at `22ed5e2a` is
  **3561 passed / 12 skipped**, fmt clean. Report your final numbers as numbers, not as "green".
- If a doc/man surface changes, `make docs` must show no diff (regenerate if it does).
- Never hand-count what a tool can count. Paste measured values.

## 2. What to fold — three blocking findings, one nit

### C-1 (Critical) — the form-engine surface never stamps `ReturnInputs.tax_year`

**Requirement: every reader of `ri.tax_year` on the editor surface must see the filer's real year, from the
moment the working return exists.** The ledger confirms zero production writes across the eight editor-chain
files, `input_form_store.rs:666` screening before `:670` stamps, and the sibling writer already fixed the
same way at `cmd/tax.rs:280-281` for FR-103.

A gate-only stamp is **not** sufficient, and this is the part to verify before you build: the answer log is
written *during* editing (`apply.rs:116-121`), so a year-0 sentence is already hashed by the time the filer
presses `s`. Stamping at the gate alone would make the gate refuse a correct answer rather than pass a
false one — fail-closed, but still wrong, and consequences 2–5 (the per-frame completeness panel, the §152
walk, the debt-ceiling warning, the Form 8615 liveness) all run *before* the gate and stay broken.

So the stamp belongs at the surfaces that **know the year and create or load the working return** — the
renderer carries it already (`edit/form.rs:177`, `TaxInputsFormState::year`), which is why the FR-97 fixture
has to set it by hand (`apply.rs:1623-1628`). Cover, at minimum:

1. a freshly materialized working return (`Loaded::Fresh` → `apply::materialize`);
2. the draft read boundary (`input_form_store::get_draft_row`) and the draft write path, per the report's
   table — a draft-only year (TY2025/26, where `input_form_store.rs:348-352` says the draft is the *primary*
   store for months) must not re-arm the trap;
3. the commit gate, in the FR-103 shape — `stamp_year` **then** screen. `set`'s disagreement rule makes a
   mismatch refuse rather than launder, so this is a belt as well as a brace.

**Then close the class, not the instance** (this is B3's whole point, and `CLAUDE.md`'s *"derive the list, or
make the compiler hold it"*): after the fix, a future surface must not be able to hand an unstamped
`ReturnInputs` to a year-scoped rule. Prefer a structural mechanism over a typed list of call sites. ★ Note
that `return_refuse.rs:2459-2461` **declines** to refuse `tax_year == 0` on the stated premise that *"a
yearless `ReturnInputs` is a test convenience"* — C-1 refutes that premise for this surface, so revisiting
that decision is in scope if you judge it the right mechanism. Whatever you choose, the four doc comments
the ledger quotes (`return_inputs.rs:2104`, `questions.rs:920`, `return_refuse.rs:2459`, `coverage.rs:696`)
must end up **true**, or corrected.

**Kills (B1 — no checker exists until it has been observed RED on a planted defect):**

- the blocking scenario, end to end: author a TY2024 return through the form seam, commit, then screen the
  committed row — it must refuse **today** with `DigitalAssetActivityUnanswered`/`WORDING_CHANGED_DETAIL`
  and pass after the fix. Run it against the unfixed code and paste the red.
- one kill per consequence you close (the completeness panel saying `complete`; the §152 age test at year 0;
  the ceiling warning silent on TY2024; the Form 8615 conjuncts live for a 60-year-old). Where a consequence
  is already covered by the first kill's mechanism, say so instead of duplicating it.
- if you add a structural guard, it lands with its own planted-defect test.

### I-1 (Important) — the registry draws the static prompt and hashes the rendered one

Five questions (`RENDERED_PROMPTS`) draw `FORM_QUESTIONS[i].prompt` via `registries.rs:43` → `f.label` →
`draw_edit.rs:2949`, while `apply.rs:116-121` hashes `prompt_text(ri)`. The ledger confirms both sides and
confirms that **none** of the nine `QuestionId::ALL` walks in the workspace asserts drawn == hashed, while
the sibling registry's pin (`apply.rs:1856-1885`, walking `DependentGate::ALL`) does exactly that and says
why: *"a yes/no gate that draws one sentence and hashes another is C-1 again: the filer's answer would read
as given under words they never saw."*

**Direction: what is drawn must become the rendered sentence** — the year-quoting form is the design intent
(`questions.rs:1902` calls the static string *"The STATIC fallback"*, and `questions.rs:104-122` gives the
reason each prompt quotes a value: a question the filer cannot check against their own papers is not a
question). `Field.label` is `&'static str`, so this needs a mechanism rather than an edit; choose it, and say
in your report why.

★ Two interactions to check rather than assume: **R15's label scan** (`xtask/src/r15_stop_list.rs`, the
FR-114 widening) reads `Field.label` through `form_spec()` and splits on `label_source` — a label that stops
being a static must still be scanned, or the instrument goes quietly blind, which is the exact failure
FR-114 existed to fix. And **`box_census`'s caption join** (`box_census.rs:1391`) builds
`format!("{} {}", f.label, f.help)`.

**Kill:** the missing pin — derived over `QuestionId::ALL` (walk the set; do not type a list of five), red
today on all five, and mutation-verified after the fix. `FR-100`'s single documented exception belongs to the
gate registry; if this registry needs one, name it and file it.

### I-2 (Important) — shipped filer-facing text names a command that prints nothing

`LIMITATIONS.md:425-427` (added in this range at `52b348c2`) tells the filer *"`report` prints an advisory
naming how many of your dispositions occurred on an exchange"*. The ledger confirms
`broker_reporting_advisory` has exactly one production caller, `main.rs:1089`, inside
`Command::ExportIrsPdf` (`:850`), and that `report`'s only broker block gates on `regime.basis`
(`render.rs:1960`), false for TY2024.

**Default fix: correct the sentence to name the command that actually prints it.** That is the minimal
correct change to a shipped filer-facing document, and the substance of the paragraph is already right.
Wiring the advisory into `report` instead is allowed **only** if you can show it is behaviour-safe and
golden-clean, and you state why the filer is better served — it changes `report`'s output, which the
examples and walkthrough goldens pin. `LIMITATIONS.md` is `include_str!`'d at `main.rs:582` and
single-sourced into the man page, so run `make docs`.

### N-1 (Nit) — two broken intra-doc links, pre-existing and outside the range

`return_1040.rs:2016` and `return_refuse.rs:289` link `Advisory::ExcessSsSingleEmployerNotCreditable`; the
variant is `ExcessSsNotCreditable` (`advisories.rs:153`). Fix both inline — it is a rename in two doc
comments — and say so in your report. Do not chase other doc links.

## 3. Severity rules that bind this fold

- A **blank** is the normal, correct case on a tax return; the invariant is **provenance**, never
  non-blankness. Never make a surface answer for the filer to close a finding. A hardcoded `0` on an
  unasked line fabricates sworn testimony.
- **Secret-handling defects never gate.** If you find one, file it in `FOLLOWUPS.md` with a reproduction and
  an owning phase; do not let it hold this fold.
- **Do not widen scope.** Three findings and a nit. If you see something else, file it in `FOLLOWUPS.md`
  with an owning phase and move on. An exemption widened to make a check pass is never the safe edit.

## 4. Your report — final action

Write `design/agent-reports/FOLD-b3-whole-branch-review.md` as your **final action**, then return only a
short summary plus its path. It must contain:

- **Per finding:** what you changed, the file:line, and *why that mechanism*.
- **Kills:** for each, the command, the **pasted red output before the fix**, and the green after. A kill
  that was never seen red is not a kill.
- **Refuted premises:** anything in this brief you measured and disproved, with the measurement.
- **Residue:** anything you filed rather than fixed, with its owning phase.
- **Gate:** the literal `make check` counts and `cargo fmt --all --check` result, plus `make docs` if a doc
  surface moved.
