# Whole-diff review (Phase E) — feat/binary-docs — round 1

**Verdict: 0 Critical / 0 Important / 0 Minor / 0 Nit — SHIP.**

Independent Phase-E review (reviewer ≠ author). Diff `b597a1a..HEAD` (Task 1 `3e493fd` inline long-help;
Task 2 `160fc08` xtask + man tree; Task 3 `328d164` TUI pages; Task 4 `07cdc95` PDFs). Contract:
`design/SPEC_binary_documentation.md` (R0-GREEN, 2 rounds). 50 files, +2649/−580.

## Verification + fault-injection

**1. [★ requirement 3 — the file-format docs] single source → both outputs — CONFIRMED.**
- The 6 file/format args carry FORMAT + a text EXAMPLE in their clap doc-comments (`cli.rs`), each read from
  SOURCE not the (misnaming) comments: export CSV headers from the writer `render.rs`
  (`income.csv` NOT `income_recognized.csv`; + `lots.csv`/`form8283.csv`/`schedule_se.csv`), classify-raw
  serde shape, key armor from `vault.rs`.
- **LIVE `--help` half:** `btctax reconcile import-selections --help` prints the `FORMAT (header + one sample
  row):` block + `disposal_ref,origin_event_id,split_sequence,sat` + the disposals.csv/lots.csv cross-links;
  `btctax init --help` prints the ASCII-armored-key description. (Ran the actual binary.)
- **LIVE man half:** each example landed on ITS git-style subcommand page — `btctax-reconcile-import-selections.1`
  (CSV header), `btctax-init.1` (armor), `btctax-export-snapshot.1` (`income.csv`),
  `btctax-reconcile-classify-raw.1` (JSON), `btctax-reconcile-select-lots.1` (`#0:25000`). `groff -man` renders them.
- **[★ fault-inject] the `help_documents_*` KAT is LOAD-BEARING.** Redacting ONLY the production doc-comment
  (lines before `mod tests`, leaving the assertion literal intact) + forced rebuild → `Compiling btctax-cli …`
  → `help_documents_import_selections_format` **FAILED** at cli.rs:699 ("must document the required CSV
  header"). (A first probe that `sed`-replaced the token file-wide passed tautologically because it also
  mutated the assertion's own expected string — a probe artifact, not a weak test; the isolated redaction
  proves the guard bites. Restored byte-for-byte.)

**2. [★ the mechanism / C1 lesson] per-subcommand pages — CONFIRMED.** 40 pages: root `btctax.1` + 37
`btctax-<path>.1` (recursing `get_subcommands()`) + hand-authored `btctax-tui.1`/`btctax-tui-edit.1`.
`manpage_covers_every_subcommand` recurses and asserts each has a committed page (drift guard);
`file_format_examples_present_in_manpage` targets the subcommand page; `manpage_multiline_examples_survive`
pins `verbatim_doc_comment`.

**3. Determinism — CONFIRMED.** `gen_docs_is_deterministic` passes (two runs byte-identical; committed `.1`
match a fresh generation — `cargo test -p xtask` fails if stale). No `#[command(version)]`, no dates.

**4. [★ no bloat] the `Cli` extraction is behavior-preserving.** Task 1 moved the clap `Cli`/`Command`/
`Reconcile`/`Optimize` from `main.rs` into `btctax_cli::cli` (723 lines) so xtask can reach `Cli::command()`
via `CommandFactory`; `main.rs` stays thin dispatch. All pre-existing CLI KATs stay green (suite 1095, up from
1084 = +11 doc KATs) → no parse/behavior change. `clap_mangen`/`roff` are reachable ONLY from `xtask`
(implementer's `cargo tree`); `btctax-cli` gained neither — the shipped binary is unchanged. `Cargo.lock`
pins clap_mangen 0.2.33 committed under `--locked`.

**5. TUI pages — CONFIRMED.** Hand-authored (the apps have no clap surface); the editor page mirrors the `?`
overlay keymap; `manpages_have_required_sections` pins root FILES+EXAMPLES + the tui-edit keys.

**6. PDFs [requirement 2].** `groff -man -T pdf` per `.1` (git-ignored — gropdf embeds a timestamp, so the
deterministic `.1` are the committed source of truth; `.gitignore:48-50` documents this). Added a `make
bundles` target producing ONE combined PDF per binary — `btctax-manual.pdf` (41 pp: root + every subcommand,
14 format-token hits) + `btctax-tui.pdf` / `btctax-tui-edit.pdf` — the user-facing deliverable.

## Full suite
`cargo test --workspace --locked` **1095 passed / 0 failed**; `clippy --all-targets -D warnings` 0;
`fmt --check` clean.

**SHIP.**
