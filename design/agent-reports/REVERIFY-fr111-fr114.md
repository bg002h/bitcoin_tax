# Re-verification: FR-111 fold (`52b348c2`) + FR-114 fold (`52558813`)

Scope: `git show 52b348c2` and `git show 52558813` only. No fresh audit. Report-only — nothing
in the working tree was left changed (see §A3 restore verification).

## Verdict

**1 Minor finding, 0 Critical, 0 Important.** Both commits do what their messages claim. The
`make gate` run surfaced 7 test failures, but all 7 are attributable to this re-verification
agent's own isolated-worktree environment (missing gitignored PDF fixtures + the
task-mandated `CARGO_TARGET_DIR` override), not to either fold — neither touched file
(`form_delta.rs`, `harness_check.rs`) is in either commit's diff, and `git diff --stat` between
the two commits confirms it. Clippy: 0 warnings.

## A — FR-114 blast radius

**A1. Every `Field` literal in the workspace sets `label_source`.** Grepped `Field {` across the
whole workspace (not just `btctax-input-form`). Two other `Field` types exist and are unrelated:
`btctax-forms::pdf::Field` (a PDF AcroForm widget descriptor, `crates/btctax-forms/src/pdf.rs:130`)
and `Edit::SetField`/`ClearField` (an enum variant, not this struct) used throughout
`btctax-input-form/src/apply.rs` and `btctax-tui-edit`. The only real
`btctax_input_form::seam::Field` literal-construction sites are:
- `crates/btctax-input-form/src/spec/registries.rs` — 6 sites (all via 4 macros: `decl_tristate!`,
  `skippable_tristate!`, `skippable_date!`, `skippable_choice!`, plus one hand-written
  `FOREIGN_COUNTRY_NAMES`, plus `census_tristate!`) — all set `label_source: LabelSource::Authored`.
- `crates/btctax-input-form/src/spec/sections.rs` — ~85 sites (hand-written literals + 9 macros:
  `w2_money!`, `scha_money!`, `ret_money!`, `dep_gate_tristate!`, `doc_money!`, `doc_text!`,
  `doc_transcribed_on!`, `sa8b_text!`, `hsa_money!`) — every site sets `label_source`.
- `crates/xtask/src/r15_stop_list.rs` — 3 sites, all inside `#[cfg(test)]`, used to plant/verify
  the checker itself — all set `label_source`.

The compiler enforces this: `LabelSource` derives `Debug, Clone, Copy, PartialEq, Eq, Hash` — no
`Default` — and `Field` is a plain struct with no `..Default::default()` anywhere in the diff or
the surrounding file, so an omitted `label_source` is `E0063` (missing struct field), not a silent
default. `cargo build`/`make gate` (§C) compiled the whole workspace clean, independently
confirming no site was missed.

**A2. `Authored`/`DocumentCaption` correctness — spot-checked 15+ sites.**
- Every `doc_money!`/`doc_text!` call (70 total, confirmed by count) sets `DocumentCaption`;
  `doc_transcribed_on!` sets `Authored`, matching the commit's claim exactly.
- Verified the load-bearing case against the extracted text: `Sa1099Box4Fmv`'s label
  `"4 FMV on date of death"` is byte-identical to `design/forms/extract/f1099sa--2025.txt:12`
  (`"3 Distribution code     4 FMV on date of death"`). `Div1099Box2bUnrecap1250`'s
  `"2b Unrecap. Sec. 1250 gain"` likewise matches `f1099div--2024.txt:29` verbatim.
- Verified a hand-written `Authored` site is correctly NOT `DocumentCaption`:
  `FOREIGN_COUNTRY_NAMES`'s label `"Schedule B line 7b — foreign country name(s)"` is btctax's own
  composed shorthand — the real Schedule B line 7b caption
  (`f1040sb--2024.txt:73`, *"list the name(s) of the foreign country(-ies) where the account is
  located"*) reads nothing like it. Correctly `Authored`.
- Spot-checked 10+ ordinary hand-written `Authored` literals (`"Filing status"`, `"First name"`,
  `"Routing number (line 35b)"`, `"Dependent name"`, `"Gift amount"`, etc.) — all are btctax's own
  composed prompts, correctly `Authored`.

**★ One thing worth flagging, Minor severity.** Two groups of `doc_money!`/`doc_text!` labels are
not *purely* verbatim captions — the macro wraps authored framing around a real box reference:
`"PAYER'S name (the broker)"` (B1099Payer — the parenthetical is authored) and the four B1099
total labels `"Short-term total: 1d Proceeds"` / `"Short-term total: 1e Cost or other basis"` /
`"Long-term total: 1d Proceeds"` / `"Long-term total: 1e Cost or other basis"` (the
"Short-term total:"/"Long-term total:" prefix is authored; "1d Proceeds"/"1e Cost or other basis"
is the real caption). None of the six currently contain a banned R15 word
(`transfer`/`lot`/`fmv`), so this produces **no current false negative** — I confirmed this by
reading every `doc_money!`/`doc_text!` label in the file (70 total) for the three banned words.
It is also not a *silent* gap: `r15_stop_list.rs`'s own doc comment on `section_labels`
(lines ~424-436) explicitly states the residue — *"a ledger question typed into a transcribed
label is caught by neither [R15 nor box-census]"* — so this is a stated, reviewed boundary
(satisfies `CLAUDE.md`'s FR-99 option 3), not an undisclosed one. Recording it here because the
brief asked for it to be flagged; it does not rise to Important given it's documented and
currently inert.

**A3. The three-way equality guard actually reds.** Planted a mutation in `section_labels`
(`crates/xtask/src/r15_stop_list.rs`) that drops `Sa1099Box4Fmv` from both the `authored` and
`transcribed` vectors (an `if f.id == FieldId::Sa1099Box4Fmv { continue; }` before the match).
Ran `cargo run -p xtask -- stop-list` (via `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`):

```
xtask stop-list: 209 scanned + 69 exempt != 279 form_spec field labels — a label must be one or
the other, never neither. ...
```

Red as expected (69 exempt vs. the correct 70). Restored the file by `cp` from a pre-mutation
backup (never `git checkout --`); `git diff --stat` and `git status --short` on
`crates/xtask/src/r15_stop_list.rs` both came back empty, confirming an exact restore.

Note for a future reader: because `section_labels`'s match is `_`-free and exhaustive over exactly
the two `LabelSource` variants, the equality is close to tautological under normal edits (any
`Field`'s label lands in exactly one of the two vectors by construction) — the guard's real target,
per its own doc comment, is a *walk-shaped* regression (a section dropped from the scan, or the
scanned/exempt counts computed from different sources), which the planted mutation above
reproduces faithfully.

## B — FR-111 prose

**B1. TY2024 prints Box C/F unconditionally for every crypto 8949 row — confirmed.**
`crates/btctax-core/src/forms.rs::form_8949()` iterates only `state.disposals` (never anything
1099-B-derived) and computes `da = year >= DIGITAL_ASSET_8949_FIRST_YEAR`, box = `C`/`F` when
`!da`, `I`/`L` when `da`. `crates/btctax-core/tests/kat_forms.rs:152-153` pins exactly this for
`year = 2024` (`Form8949Box::C` ST, `Form8949Box::F` LT). `BundledFullReturnTables::load()`
(`crates/btctax-adapters/src/tax_tables.rs:~101`) inserts only `2024` into `by_year`, so
`full_return_for` fails closed on every other year — matches the "TY2024 only" scope claim.

**B2. The REFUSALS bullet is accurate.**
`crates/btctax-core/src/tax/return_refuse.rs:2574-2589`:
```rust
if carries_totals && b.basis_reported_and_no_adjustments != Some(true) {
    return refuse(RefuseReason::Form1099BNeedsForm8949, ...);
}
```
`!= Some(true)` covers both `None` (unanswered) and `Some(false)` — both refuse, exactly as
LIMITATIONS.md states. `Form1099B` (`return_inputs.rs:652-699`) carries no per-box (1f/1g/5/7)
fields — only the single combined `basis_reported_and_no_adjustments: Option<bool>` — so the
prose's box-1f/1g/5/7 enumeration is explanatory gloss on what "adjustments" means (matching the
field's own help text word-for-word), not a claim the tool tracks those boxes individually. Not
misleading.

**B3. All three quotes verified verbatim against source:**
- `"Box A/B (ST) / D/E (LT)"` — exact substring of `docs/examples/examples.md:983` (the `[I5]`
  export advisory line).
- `"Do not use box C to report digital asset transactions. Use box I"` —
  `crates/btctax-core/src/forms.rs:34-35`, and that text is itself verbatim from
  `design/forms/extract/i8949--2025.txt:416-417`.
- Schedule D line 1a text — LIMITATIONS.md's excerpt `"transactions reported on Form 1099-B for
  which basis was reported to the IRS and for which you have no adjustments."` is an exact
  substring of `return_inputs.rs:641-643`'s quote, which is itself verbatim against
  `design/forms/extract/f1040sd--2024.txt:29-30,54-55`.

**B4. Nothing in the new prose was found to be false.** Checked in addition to the above: the
`[I5]` advisory mutual-exclusivity claim (`"gated on regime.basis == false... G/H/J/K needs
regime.basis == true"`) is confirmed exactly in
`crates/btctax-cli/src/cmd/admin.rs::broker_reporting_advisory` (`if regime.basis { G/H/J/K
wording } else { ... }` — a real `if`/`else`, mutually exclusive by construction); the `report`
prints-an-advisory claim is confirmed live in `crates/btctax-cli/src/main.rs:1086-1092`; and the
TY2026 regime figures cited by the FOLLOWUPS.md entry (`basis: true` only from TY2026) match
`crates/btctax-forms/forms/{2024,2025,2026}/YEAR.toml` exactly. FR-116 residue is correctly filed
at `FOLLOWUPS.md:7034`. One judgment call, not a defect: the "From TY2026 this stops being a scope
answer and becomes yours" paragraph describes `route_8949_boxes`/regime-routing behavior that is
real and tested in `forms.rs`, but full-return TY2026 filing itself is not yet bundled
(`YEAR.toml` status "preparing"); the document's own opening line ("Tax year supported: TY2024
only") and the adjacent "Box I/L is a later year's answer, not this one's" bullet make the
future-tense framing clear in context — not flagged as false.

## C — suite

`make gate` (touches every `.rs` file, then `cargo nextest run --workspace --no-fail-fast` +
`cargo clippy --workspace --all-targets --all-features -- -D warnings` in parallel), run once,
captured to a file:

```
Summary [39.608s] 3561 tests run: 3554 passed, 7 failed, 12 skipped
```

Clippy: 0 warnings/errors anywhere in the captured log (`grep -c "warning:"` → 0).

All 7 failures are `xtask`-crate tests in two files, **neither touched by either commit**
(`git diff --stat 52b348c2^..52558813 -- crates/xtask/src/form_delta.rs
crates/xtask/src/harness_check.rs` is empty):

- **6× `form_delta::tests::*`** — all fail with `"no PDF found for f6251--2026-DRAFT"` (or
  downstream consequences of it). `design/forms/**/*.pdf` is gitignored
  (`.gitignore:63`) by design (commit `408dc6a0`, "drop the IRS PDFs from the repo; keep URL notes
  + the text layer"). The main checkout (`/scratch/code/bitcoin_tax/design/forms/2026/`) has the
  actual gitignored PDF cached locally; this agent's isolated worktree
  (`.claude/worktrees/agent-ae43ee72485767b57/design/forms/2026/`) does not — git worktrees don't
  share untracked/gitignored files. Confirmed directly: `find` on the worktree path returns only
  `.pdf.txt`/`.json`/extract `.txt` siblings, no `.pdf`; the same `find` against the main checkout
  path returns the `.pdf` too.
- **1× `harness_check::tests::the_write_hook_denies_new_archives_and_asks_once_per_new_directory`**
  — fails with `"cargo build -p xtask succeeded but left no binary where on-write.sh looks — if
  the target dir moved, the HOOK's lookup needs updating too"`. Directly caused by this task's
  mandated `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` override: the hook script
  looks for the xtask binary at the default (unset-env) target path.

Both causes are artifacts of this re-verification agent's environment (a fresh isolated worktree
+ the task-mandated target-dir override), not regressions from either fold. Every test in
`r15_stop_list.rs` — including the three FR-114 planted-defect tests
(`the_label_scan_reds_on_a_ledger_question_in_a_field_label_and_names_the_field`,
`the_real_form_spec_labels_are_the_scanned_set_and_are_clean`,
`the_exemption_covers_a_transcribed_caption_and_never_an_authored_ledger_question`) and
`the_r15_stop_list_holds_on_the_committed_tree` — passed. The "12 tests skipped" figure matches
the FR-114 commit's own claimed gate output exactly.

## Commands run

```
git show --stat 52b348c2
git show --stat 52558813
git show 52b348c2 -- crates/btctax-cli/LIMITATIONS.md
git show 52558813 -- crates/btctax-input-form/src/seam.rs
git show 52558813 -- crates/btctax-input-form/src/spec/registries.rs
git show 52558813 -- crates/btctax-input-form/src/spec/sections.rs
git show 52558813 -- crates/xtask/src/r15_stop_list.rs
grep -rn "Field {" --include="*.rs" .            # whole-workspace construction-site sweep
grep -n "doc_money!\|doc_text!" crates/btctax-input-form/src/spec/sections.rs
diff/grep against design/forms/extract/{f1099sa--2025,f1099div--2024,f1040sb--2024,i8949--2025,f1040sd--2024}.txt
# A3 mutation-plant-restore (r15_stop_list.rs::section_labels):
cp crates/xtask/src/r15_stop_list.rs <backup>
<Edit: planted `if f.id == FieldId::Sa1099Box4Fmv { continue; }`>
CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review cargo run -p xtask --bin xtask -- stop-list
cp <backup> crates/xtask/src/r15_stop_list.rs   # restore
git diff --stat crates/xtask/src/r15_stop_list.rs; git status --short   # both empty, confirmed
# B — source checks
sed -n crates/btctax-core/src/forms.rs (form_8949, box enum doc comment)
sed -n crates/btctax-core/src/tax/return_refuse.rs:2555-2600
sed -n crates/btctax-core/src/tax/return_inputs.rs:640-700
sed -n crates/btctax-cli/src/cmd/admin.rs:688-735 (broker_reporting_advisory)
sed -n crates/btctax-cli/src/main.rs:1070-1095
cat crates/btctax-forms/forms/{2024,2025,2026}/YEAR.toml
# C — suite
CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review make gate  > gate-output.log 2>&1   # once
grep -E "Summary|FAIL|warning:" gate-output.log
find /scratch/code/bitcoin_tax/design/forms -iname "*f6251*2026*"
find <worktree>/design/forms -iname "*f6251*2026*"
```
