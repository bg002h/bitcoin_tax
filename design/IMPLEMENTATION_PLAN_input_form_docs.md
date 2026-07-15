# Input-Form Docs (plan 4 of 4) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Document the new "tax inputs" authoring path (spec §11.9): the `btctax-tui-edit` "tax inputs" mode gets a man-page section, and `LIMITATIONS.md` records that the form is now the primary authoring path for a full-return `ReturnInputs` and what it cannot see at entry — while `income template`/`income import` remain the TOML import/export path.

**Architecture:** Docs-only. Edit the HAND-AUTHORED roff man page `docs/man/btctax-tui-edit.1` (the TUI pages are hand-authored + committed, NOT clap-generated — `crates/xtask/src/docs.rs:98`), keep the docs KATs green (`manpage_covers_every_subcommand`, `manpages_have_required_sections`, `gen_docs_is_deterministic` in `xtask/src/docs.rs`), regenerate the PDFs, and append a section to `crates/btctax-cli/LIMITATIONS.md`. No production code changes (the one allowed exception: extending the `manpages_have_required_sections` KAT to also require the new `T` key, so the man page can't drift).

**Tech Stack:** roff man pages, `crates/xtask` docs generator (`make docs`), Markdown.

## Global Constraints

- **`docs/man/btctax-tui-edit.1` is hand-authored roff** — edit it directly (it is NOT generated from clap; `make docs` regenerates the CLI pages + all PDFs, but the TUI `.1` files are static committed sources). Keep roff valid (the existing `.SH`/`.SS`/`.TP`/`.B`/`.BR` idiom; escape `-` as `\-`, `§` survives via `groff -k` preconv).
- **The docs KATs gate** (`cargo test -p xtask`, run by `make check`): `manpages_have_required_sections` asserts the TUI page lists certain action keys (currently `?`/`V`/`O`) and has the required `.SH` sections; `gen_docs_is_deterministic` asserts two generation runs match; `manpage_covers_every_subcommand` covers the CLI (unchanged — the tax-inputs mode adds NO `btctax-cli` subcommand). Do not break them.
- **No new `btctax-cli` subcommand** — the "tax inputs" mode is a KEY (`T`) inside `btctax-tui-edit`, launched directly (`btctax-tui-edit [--vault PATH]`); it is NOT a `btctax income ...` subcommand. Docs reflect that.
- **`income template` / `income import` remain** — the form does NOT remove them; it is the primary INTERACTIVE authoring path, complementary to the TOML import/export. Say so; do not imply they are deprecated.
- **`LIMITATIONS.md` lives at `crates/btctax-cli/LIMITATIONS.md`** — append/extend; keep its existing structure + tone (fail-closed, conservative-overstate, the screen-vs-report distinction).
- **No presupposed installed base** ([[no-users-yet]]) — btctax has no users; docs describe the feature, not a migration for existing users.
- **Gate per task:** `make check` (the docs KATs run here). If `make docs`/`groff`/`gs` PDF regeneration isn't available in the environment, edit the `.1` + note the PDF must be regenerated (the `.1` is the source of truth; the KATs validate the `.1`, not the PDF). Fish shell: quote globs; heredoc for `git commit -F -`.

## File Structure

- **Modify** `docs/man/btctax-tui-edit.1` — add the "tax inputs" mode (a `.SH` section or a `.SS` under KEY BINDINGS) documenting the `T` entry key + the mode's keys/flow.
- **Modify** `crates/xtask/src/docs.rs` (tests only) — extend `manpages_have_required_sections` to require the `T` key in `btctax-tui-edit.1` (drift guard).
- **Regenerate** `docs/pdf/btctax-tui-edit.pdf` (+ the combined manual) via `make docs`/`make bundles` if the tooling is present.
- **Modify** `crates/btctax-cli/LIMITATIONS.md` — a section on the form as the primary authoring path + what it can't see at entry.

---

### Task 1: Document the "tax inputs" mode in `btctax-tui-edit.1` + pin it with the docs KAT

**Files:**
- Modify: `docs/man/btctax-tui-edit.1`
- Modify: `crates/xtask/src/docs.rs` (the `manpages_have_required_sections` test)

**Interfaces:** none (docs). Produces: a man-page section describing the mode + a KAT that requires the `T` key.

- [ ] **Step 1: Write the failing test first** — in `crates/xtask/src/docs.rs`'s `manpages_have_required_sections`, add `T` to the set of action keys required in `btctax-tui-edit.1` (alongside `?`/`V`/`O`). Run `cargo test -p xtask manpages_have_required_sections` → it FAILS (the man page doesn't yet document `T`). This is the drift guard: the man page must list the tax-inputs entry key.

- [ ] **Step 2: Confirm it fails** — `cargo test -p xtask manpages_have_required_sections` → FAIL naming the missing `T`.

- [ ] **Step 3: Add the man-page section.** In `docs/man/btctax-tui-edit.1`, add (mirror the existing roff idiom; place after the reconciliation KEY BINDINGS, e.g. a new `.SH "TAX INPUTS MODE"` or a `.SS "Tax inputs (full-return authoring)"`):
  - The entry: `.B T` on the Browse screen opens the "tax inputs" editor for the selected year — the interactive way to author a full-return `ReturnInputs` (header/PII, W-2s incl. box 12, Schedule A incl. charitable, dependents, declarations, skippables) WITHOUT hand-editing TOML.
  - The layout: a section list (left), a per-field pane (right) with live validation, a status line (active source + screen status + key legend).
  - The keys (from the in-app legend): a filing status must be chosen first (the return materializes on that choice); `Up/Down` field, `Left/Right`/`Tab` section, `Enter` edit a field, `a`/`d` add/remove a repeating row, `c`/`x` create/delete an optional section (Schedule A / spouse), a drill-in for nested box-12 / charitable entries, `s` commit (screens the return; a refusal jumps to the offending field), `t` toggle the active source (full return ↔ tax-profile estimate), `X` discard a parked draft, `q`/`Esc` close (the draft is autosaved). SSN / IP-PIN are entered no-echo and shown masked; the whole return autosaves to an encrypted draft.
  - A pointer: it commits only a `screen_inputs`-clean return; `income template` / `income import` remain for TOML export/import.
  Keep it factual and within v1 scope (do NOT describe deferred sections as available).

- [ ] **Step 4: Run green** — `cargo test -p xtask` (all docs KATs incl. the extended one + `gen_docs_is_deterministic` pass), then `make check` green. If `make docs` is available, run it to regenerate `docs/pdf/btctax-tui-edit.pdf` + `make bundles`; otherwise note in the report that the `.1` is updated and the PDF needs a `make docs` on a groff-equipped box (the KATs validate the `.1`).

- [ ] **Step 5: Commit** — `git commit -m "docs(input-form): document the tax-inputs mode in btctax-tui-edit.1 + pin the T key (plan 4 task 1)"`

---

### Task 2: `LIMITATIONS.md` — the form is the primary authoring path + what it can't see at entry

**Files:**
- Modify: `crates/btctax-cli/LIMITATIONS.md`

**Interfaces:** none. Produces: a section recording the authoring path + the entry-time blind spots.

- [ ] **Step 1: Read the current `LIMITATIONS.md`** structure (the fail-closed rule, Supported, OMISSIONS, REFUSALS, UNREPRESENTABLE, the screen-vs-report material). Find the right home for the new section (near "Supported" / the authoring/input material, or a new top-level `## Authoring a full return` section).

- [ ] **Step 2: Add the section.** Content (match the file's tone — factual, conservative, fail-closed):
  - **The form is the primary authoring path.** The `btctax-tui-edit` "tax inputs" mode (press `T`) is the primary way to author a full-return `ReturnInputs` — interactive, with live per-field validation, autosave to an encrypted draft, and a commit that writes ONLY a `screen_inputs`-clean return. `income template` (export a TOML skeleton) and `income import` (import a filled TOML) remain as the file-based import/export path; they are complementary, not deprecated.
  - **What the form cannot see at entry.** The commit-screen (`screen_inputs`) is the SAME gate the resolver uses, so a committed return is never worse than what `report` computes — BUT: (a) the v1 form covers only the common subset (header/PII, W-2s, Schedule A, dependents, declarations, skippables); the **deferred sections** — Schedule C, QBI, 1099-INT/DIV/G, capital-loss / charitable / QBI carryforwards — are NOT authorable in the form yet (enter them via TOML import if needed; the tree already expresses them for a later phase). (b) Some checks fire only at **`report`**, not at the commit-screen — the advisories (e.g. the mixed-use-mortgage allocation, the §164(b)(5) sales-tax election, the §170 qualified-appraisal trigger, blindness/DOB forfeiture prompts) surface when you run `report`, so a form-clean return can still have report-time advisories to act on. The form commits a return that is SCREEN-clean, not necessarily advisory-free.
  - Keep it honest: the form removes the fiddly-TOML pain for the common case; it does not expand what v1 computes.

- [ ] **Step 3: Verify** — `make check` (LIMITATIONS.md isn't gated by a test, but confirm nothing references a now-stale claim; if there is a doc-consistency KAT that reads LIMITATIONS.md, keep it green). Grep the repo for any doc that says "hand-edit the TOML" as the ONLY path and reconcile (the form is now primary).

- [ ] **Step 4: Commit** — `git commit -m "docs(input-form): LIMITATIONS — the form is the primary authoring path + entry-time blind spots (plan 4 task 2)"`

---

## Self-Review notes (controller)

- **Spec coverage (§11.9):** man pages (T1) · `income template`/`import` remain (T1 pointer + T2) · `LIMITATIONS.md` form-is-primary + can't-see-at-entry (T2). Fully covered.
- **No production behavior change** — docs + a docs KAT only; the engine/store/TUI are frozen at plan-3's green state.
- **Ceremony (docs):** each task ends with an independently-reviewable deliverable; the final review is a doc-accuracy/consistency pass (Opus), not a Fable code-integration whole-branch (there is no code integration seam in a docs plan).
- **Deferred:** the README / any tutorial expansion (out of §11.9 scope); the deferred-section docs (they don't exist as a feature yet).
