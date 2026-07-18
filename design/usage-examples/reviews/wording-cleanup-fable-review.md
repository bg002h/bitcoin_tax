# Independent review — pre-v0.7.0 product-wording cleanup (`128b36b`)

Reviewer: Fable (independent; author ≠ reviewer). Scope: the user-authorized product-message
cleanup commit `128b36b` (UX-P4-2, UX-P1-2/N3, UX-P1-4, UX-P1-5, UX-P1-6 + Section-A/multi-lot
extension, UX-P1-9, UX-P4-12(a)) and its regenerated goldens/man page. Verified against LIVE
source on `feat/usage-examples` at `128b36b` (2026-07-18).

## Verdict

**GREEN — 0 Critical / 0 Important** (1 Minor, 2 Nits, none gating). Every fixed string was
verified against the behavior it describes, at the source of truth: the TUI modal's new
acquired-date default matches the engine (`long_term_default_acquired` = `replace_year(y−1)` then
`previous_day()` — exactly 1 yr + 1 day before receipt, with the long-term proof comment,
`crates/btctax-core/src/conventions.rs:84-99`, applied at `fold.rs:1024`) and the CLI help
(`cli.rs:525-528`); the export-irs-pdf help's dispatch claim matches the runtime (the P5-C1
refusal is gone — `cmd/admin.rs:226-229` dispatches full-return years to `export_full_return`,
which takes no `forms` argument, so "`--forms` is ignored on that path" is literally true, and the
non-overlapping-filenames claim is pinned at the dispatch comment); the `--forms` value names were
verified EMPIRICALLY against clap (`invalid value` error lists `f8949, schedule-d, schedule-se,
form8283, form1040` — the fix is right, the old `form-8283`/`form-1040` were wrong); the multi-lot
Form 8283 advice matches the structural flag (`core/forms.rs:426-430`: every non-carrier row is
`needs_review: true` unconditionally, the carrier row by `is_review_complete(section)` — so the
two-part advice is accurate in all four detail×lot-count combinations and breaks the UX-P1-6-ext
advice loop); the DOB render is calendar-correct for the leap-year fixture (`[2012, 106]` →
`04/15/2012`, independently recomputed) and display-only (the stored-serialization oracle
`fullreturn_fixture_is_the_kitchen_sink_oracle` passes; nothing anywhere parses `income show`
output; DOB is the *only* `time::Date` field class in `ReturnInputs`, so the wart class is fully
covered); and `parse_income_kind`'s error lists exactly the five `IncomeKind` variants
(`eventref.rs:123-135` vs `core/event.rs:33-39`). Regen integrity: an order-insensitive
value-level JSON comparison of the J6 `income show` block shows **zero figure/value drift**
(only DOB format changed); the J6 empty "Filled IRS forms →" header is gone while J1/J2 slice
headers survive (3 → 2 occurrences); the TUI modal golden changes only the acquired_at line (which
now wraps, absorbing one blank line inside the fixed-height box); the man page regenerates
**byte-identical** from current source (`cargo run -p xtask -- docs` → clean tree). Scope fence
holds: the diff is strings, one display-only JSON transform, and a println gate — `written` is
computed identically and the forms are already written in `admin.rs` before the header decision,
so UX-P1-4 changes only whether a header prints, never which forms exist. **`make check` is green:
1963/1963 passed, 8 skipped (standing), ~14s** — including `examples_golden_matches_committed` and
`btctax_tui_edit_goldens_match_committed`, which independently prove both regenerated goldens
faithful to the live binaries.

## Findings

### Minor

- **M-1 — undisclosed whole-tree key re-ordering in `income show` output.**
  `crates/btctax-cli/src/cmd/tax.rs:186-196`: the UX-P1-5 fix replaced direct
  `to_string_pretty(&mask_pii(&ri))` with a round-trip through `serde_json::to_value` (to host the
  DOB transform). `serde_json::Value::Object` is BTreeMap-backed (no `preserve_order` feature in
  this workspace), so **every object's keys are now alphabetical instead of the struct's declared
  order** — the real reason the J6 golden hunk is ~160 lines for a ~5-line intended change.
  Failure scenario: none functional — verified value-identical (order-insensitive JSON compare),
  deterministic, display-only, and never parsed (the only consuming test,
  `tests/tax_profile.rs:178-208`, asserts substrings). But it is an unintended, undisclosed output
  change inside a cycle whose premise is controlled string edits: the commit message and FOLLOWUPS
  fold note both characterize UX-P1-5 as "renders DOB as MM/DD/YYYY … display-only" without naming
  the re-ordering, and the churn obscured golden review. Also a mild readability trade: the
  struct's curated grouping (identity → address → income blocks) becomes an alphabetical scatter
  (`capital_loss_carryforward_in` now leads). Non-gating; disposition for the author: either
  disclose it as accepted (one line in the FOLLOWUPS fold note) or restore field order (e.g.
  `serde_json` `preserve_order`, weighing the transitive `indexmap` cost) in a post-v0.7.0 cycle.

### Nit

- **N-1 — decaying line-number citation in a comment.**
  `crates/btctax-tui-edit/src/draw_edit.rs:927-929`: the new comment cites "cli.rs:526-527".
  Accurate today, but the project's own cycle-prep doctrine notes line citations decay every
  merge; a symbol citation (`ClassifyInboundSelfTransfer` doc-comment) would not.
- **N-2 — tail asymmetry between the two reworded 8283 advisories.** The full-return message
  (`main.rs:682-688`) ends "NOT filing-ready as written."; the slice message (`main.rs:775-780`)
  has no such tail. Pre-existing structure, merely more visible now that both were reworded to the
  same two-part shape in one commit. No filer is misled (the slice message still says "needs
  REVIEW").

## Gate

**0 Critical / 0 Important — GREEN.** `make check` 1963/1963 passed (8 standing skips); clippy
clean per the gate. The wording cleanup is fit to ship in v0.7.0 as reviewed. M-1/N-1/N-2 are
recorded, non-gating; M-1 deserves a one-line disclosure or a post-v0.7.0 owner per the burndown
rule.
