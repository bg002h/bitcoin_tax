# BRIEF — build FR-114: extend R15's ledger-word ban to `btctax-input-form` field LABELS

**Repo:** `/scratch/code/bitcoin_tax` · **Tree:** shared `main` · **Role:** ONE opus builder.

## The one task

`ledger_words_in_registry_prompts` (`crates/xtask/src/r15_stop_list.rs:233`) bans the ledger words
`transfer` / `lot` / `fmv` on word boundaries — but it is fed **only** `btctax-core`'s question
registries (`registry_prompts()`). `btctax-input-form`'s `Field.label` — text a filer reads while
typing — is never scanned. Extend the scan to those labels.

## DECIDED — do not re-litigate, do not widen

- ✅ **Scan `Field.label` ONLY. Do NOT scan `Field.help`.** (Owner, 2026-09-09.) R15's target is the
  interview *re-asking* a ledger question; the label **is** the question, `help` explains.
- ✅ **State the boundary in the source**, per `CLAUDE.md`'s FR-99 option 3: say that `Field.help` is
  not scanned, why, and name the residue — *a ledger question phrased inside help text is not caught*.
  An honest boundary is reviewable; a silent one is the defect.
- ❌ **Rejected: a per-site allow list.** It would be the FR-99 disease reproduced inside the fix for
  the eighth FR-99 instance. If you find yourself typing a list of exempt sites, stop and report.

## Facts ALREADY MACHINE-CHECKED — build on these, do not re-derive

1. **Zero labels red today.** The extension lands green. Measured by replicating the checker's exact
   split (`lower.split(|c| !c.is_ascii_alphanumeric()).filter(non-empty)`, then `BANNED.contains(&w)`)
   over every `label:`/`help:` string literal in `crates/btctax-input-form/src`.
2. **The three strings that red are all in `help`, none in a `label`**: `sections.rs:2483`
   (*"a per-lot import"*), `:2498` (*"arrived by transfer"*), `:2937` (*"its own crypto lot engine"*).
   All three are out of scope by the decision above. `:2937` is filed separately as **FR-115**.
3. **FR-114's own stated premise is RETRACTED.** It claimed `BROKER_FIELDS`' *"Covered lots"* /
   *"Noncovered lots"* would red and were correct IRS §6045 vocabulary. **They do not red** — the
   checker matches WHOLE words and those labels say `lots`; `"lots" != "lot"`. Verified:
   `"Covered lots — …" → []`, `"Which lot did you sell?" → ["lot"]`. Do not reinstate that reasoning.
4. **`xtask` already depends on `btctax-input-form`** (`crates/xtask/Cargo.toml:24`), and
   `btctax_input_form::spec::form_spec() -> &'static [Section]` (`spec/mod.rs:24`) is the public
   registry. `Section.fields: &[Field]`, `Field.label: &str`.

## What to build

- Walk **`form_spec()`** → `Section.fields` → `Field.label`. **Derive from the type — never a hand
  list of sections or fields, and never a regex over source text.** The other three R15 checks scan
  source strings because they hunt *shapes*; this one has a typed registry, so use it.
- Feed those labels through the **existing** `ledger_words_in_registry_prompts` — do not fork a second
  copy of the ban. Label them so a finding names the section and field (e.g.
  `"form_spec BrokerReporting/BrokerCovered"`), not just a line number.
- Add a **floor guard** in the same style as the existing ones (`form.len() < 5`,
  `prompts.len() < 50`, the `scanned_rendered != rendered` equality): *a walk that finds nothing must
  not pass by finding nothing.* Prefer an **equality** against a count derived from `form_spec()` over
  a magic minimum, so a section dropping out of the registry reds.
- Fold the label count into `run()`'s success string, which already reports what it actually scanned.

## The B1 gate — seen-red-once

The checker does not exist until it has been **observed RED on a planted defect**, and the kill must
**call the instrument** (not re-implement its logic inline):

- Plant a label containing a real ledger question — `"Which lot did you sell?"` — and assert the
  extension reds **naming the field**.
- Assert the near-misses stay green (`plot`, `allot`, `slot`, `transferable`) — the existing kill at
  `r15_stop_list.rs:691` already covers the word-boundary half; extend, do not duplicate.
- ★ Add the discriminating case this task discovered: assert **`"Covered lots"` does NOT red** while
  **`"Which lot did you sell?"` does**. That pins the plural/singular boundary that the retracted
  premise got wrong, so nobody re-derives it incorrectly later.

## Check before you finish

`SECTIONS` (`spec/mod.rs:25`) is itself a hand-typed const list. **Determine whether an existing guard
proves it exhaustive over `SectionId`** (an `_`-free match, a count assertion, a coverage KAT). If one
exists, name it in your report. If none exists, **report it as a finding — do NOT fix it here**; it is
pre-existing, out of this task's scope, and belongs in a follow-up.

## Constraints

- **Stop and report rather than build on a premise you can disprove.** Two controller briefs were
  refuted by measurement this session, and one of my own was refuted an hour ago. If any numbered fact
  above is wrong, say so and stop — do not work around it.
- Never hand-count what a tool can count. Run it, paste the value.
- Scope: this checker only. No unrelated cleanups, no touching the three `help` strings, no reformatting.
- **Commit nothing.** The controller commits. Leave the tree dirty with your edits.
- Gate before you report: `make gate` (touch-then-check), then `cargo fmt --all --check` — note
  `make gate` does NOT run `cargo fmt` and the pre-commit hook blocks on it.

## Output

As your **final action**, write your report to exactly:

`design/agent-reports/REPORT-build-fr114-label-scan.md`

Sections: `## What changed` (files + what each edit does) · `## The kill` (the planted defect, the
command, and the VERBATIM red output, then the green after) · `## Counts` (labels scanned, sections
walked, gate output) · `## The SECTIONS exhaustiveness question` (your finding) · `## Anything I could
not do` (empty if none).

Return only a 5-line summary plus that path. Do not paste the report inline.
