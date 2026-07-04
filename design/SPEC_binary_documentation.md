# SPEC — man pages + PDFs for the three btctax binaries (with inline file-format docs)

**Source baseline:** `main` @ `13b3b17` (branch `feat/binary-docs`). **Review status: R0 round 1 folded
(1C / 2I / 4M / 2N — all folded). Review: `reviews/R0-spec-binary-documentation-round-1.md`. Awaiting R0
round 2. Key fold: clap_mangen does NOT render subcommand args from a single root render → PER-SUBCOMMAND
man pages (git-style, recurse `get_subcommands()`); file list confirmed complete.**
**Lineage:** user request (2026-07-04): documentation for each binary — **man pages first, then PDFs**;
authoring = **HYBRID** (clap_mangen auto-gen + hand-authored). PLUS: wherever a command references a file
(or a structured format) OTHER than the vault or an exchange-transaction list, document its FORMAT with a
TEXT EXAMPLE **inline in the man page AND in the binary's `--help`**. User said "proceed autonomously."

## Goal
1. `man` pages for `btctax` (CLI), `btctax-tui` (viewer), `btctax-tui-edit` (editor).
2. `PDF`s of each (from the man pages; man first).
3. Inline **FILE-FORMAT** docs (format + a text example) for every file/structured-format argument that
   isn't the vault or an exchange-transaction import — appearing in BOTH `--help` and the man page.

## Architecture — single source of truth
The file-format docs live in the **clap `///` doc-comments (long-help)** on the file-taking args/subcommands
(`btctax-cli/src/main.rs`). clap shows the extended doc-comment as LONG help (`--help`), satisfying
requirement 3's help-text half; and **clap_mangen renders the same clap `Command` into the man page**,
satisfying the man-page half — **one placement, both outputs, zero drift** (this is why the hybrid was
chosen).
- **man-page layout [R0-C1 — PER-SUBCOMMAND, git-style]:** `clap_mangen::Man::new(cmd).render()` renders
  only ONE command's own args + a subcommand *name→about* list — it does **NOT** render subcommand ARGS from
  the root. So the six file-format args (all on subcommands) would be ABSENT from a single `btctax.1`.
  Generate one man page PER command by RECURSING `Command::get_subcommands()` — the standard multi-page
  convention (`git.1` + `git-commit.1` …): **`docs/man/btctax.1`** (root: NAME/SYNOPSIS/DESCRIPTION/FILES/
  EXAMPLES + the subcommand list) and **`docs/man/btctax-<sub>[-<subsub>].1`** per subcommand (e.g.
  `btctax-reconcile-import-selections.1` carries the CSV-format long-help; `btctax-init.1` the key-backup
  armor). The generator walks the tree, calls `clap_mangen::Man::new(subcmd)` per node, and names pages
  `btctax[-<path>].1`. The hand-authored **DESCRIPTION/FILES/EXAMPLES** are stitched (roff append) into the
  ROOT `btctax.1` only.
- **the generator [R0-M3]:** a NEW workspace member **`crates/xtask`** (NOT a `[[bin]]` in btctax-cli — a
  `[[bin]]` can't use a dev/build-only dep, and `clap_mangen` must NOT be a runtime dep of the shipped
  `btctax`). `xtask` depends on `btctax-cli` (for `Cli::command()` via `CommandFactory`) + `clap_mangen = "0.2"`;
  run via `cargo run -p xtask -- docs`. **No `#[command(version)]`** [R0-N1] and no embedded dates →
  deterministic, so the committed `.1` don't churn.
- **`btctax-tui.1` / `btctax-tui-edit.1` [HAND-AUTHORED]:** interactive apps with **NO clap surface** (they
  parse `env::args` manually, main.rs:69-88) [R0-M1] — so they're hand-authored roff/markdown regardless:
  SYNOPSIS, the tab set (Holdings/Disposals/Income/Tax/Forms/Compliance), keybindings. The editor's keymap
  is a hand-authored COPY of the `?`-overlay content (`draw_help_overlay` is styled ratatui `Line`s, not a
  reusable const [R0-M2]) — a KAT pins the man page lists the current action keys (same guard as the overlay's).
- **PDFs:** `groff -man -T pdf <x.1> > <x.pdf>` (verified `groff -T pdf` emits `%PDF-1.7`). Man FIRST, PDF after.
- **Locations:** generated `docs/man/{btctax,btctax-tui,btctax-tui-edit}.1` (committed — browsable/installable
  without a build), hand-authored roff/markdown supplements in `docs/man/src/`, `docs/pdf/*.pdf`, and the
  `gen-docs` bin. A `make docs` / `just docs` (or the bin itself) regenerates all.

## The inline file-format docs [requirement 3 — the enumerated set]
Extend the clap `///` long-help on each of these with **FORMAT + a text EXAMPLE** (flows to `--help` + the
subcommand's man page `btctax-<sub>.1`). **[R0-I1] each carries `#[arg(verbatim_doc_comment)]` (or
`#[command(verbatim_doc_comment)]`)** so the multi-line examples (armor block, 2-line CSV, JSON) are NOT
reflowed by clap — the codebase already uses this workaround at main.rs:111-119; the verbatim text flows to
clap_mangen too.
- **`init --key-backup <FILE>` and `backup-key --out <FILE>`** — an ASCII-armored, passphrase(S2K)-encrypted
  private key, owner-only (mode 0600). Example (structure, NOT a real key):
  ```
  -----BEGIN PGP PRIVATE KEY BLOCK-----
  ... base64 armor of the S2K-encrypted secret key ...
  -----END PGP PRIVATE KEY BLOCK-----
  ```
- **`export-snapshot --out <DIR>`** — a directory receiving the decrypted SQLite DB + projection CSVs. The
  ACTUAL set (from the WRITER `render.rs`, NOT the code's own doc-comments which misname it [R0-I2]):
  `disposals.csv`, `removals.csv`, **`income.csv`** (NOT `income_recognized.csv`), `lots.csv`; with
  `--tax-year`: `form8949.csv`, `schedule_d.csv`, `form8283.csv`, `schedule_se.csv`. Document each CSV's
  HEADER row (esp. the `event` column that reconcile commands consume). Example (e.g. `removals.csv`):
  `event,disposed_at,kind,btc,...` + one sample row. [impl: read the EXACT headers from `render.rs`
  (~577/694/720/727), never from the doc-comments.]
- **`reconcile import-selections <CSV>`** — header `disposal_ref,origin_event_id,split_sequence,sat`
  (validated loudly). Example:
  ```
  disposal_ref,origin_event_id,split_sequence,sat
  import|gemini|trade|T-2.O-2,import|coinbase|X,0,1000000
  ```
- **`reconcile classify-raw --payload-json <JSON>`** — a JSON imported `EventPayload`
  (Acquire/Income/Dispose/TransferOut/TransferIn/Unclassified). Example:
  `{"Acquire":{"sat":2000000,"usd_cost":"1680.00","fee_usd":"5.00","basis_source":"ExchangeProvided"}}`
  [impl: verify the exact serde shape.]
- **`reconcile select-lots --from <PICK>...`** — each PICK is `origin_event_id#split_sequence:sat`
  (`parse_lot_pick`, eventref.rs:111). Example: `--from import|coinbase|X#0:25000 --from import|river|Y#1:5000`.
- (Not a file, but referenced: event-refs come from the export CSVs' `event` column — the FILES section
  cross-links this so a user knows where to get an eventref.)

## clap_mangen integration [R0-M3]
- `clap_mangen = "0.2"` (clap 4.5-compatible; both derive from clap's `Command`) is a **normal dependency of
  the `crates/xtask` generator crate** — NOT a dep (runtime/build/dev) of the shipped `btctax`. `xtask`
  depends on `btctax-cli` for `Cli::command()` (`CommandFactory`) and RECURSES `get_subcommands()`, calling
  `clap_mangen::Man::new(subcmd)` per node (one `.1` per command).
- Deterministic (no timestamps, no `#[command(version)]` — R0-N1) so committed `.1` are reproducible.

## SemVer / lockstep
- **Docs-only + additive clap long-help.** No flag NAME/arg changes (only extended `///` help text) → no GUI
  `schema_mirror` concern, no behavior change, no new runtime dep on the shipped binaries. New `docs/` tree +
  a generator bin. PATCH-class.

## KATs
- **`help_documents_<X>_format`** (key_backup / export_snapshot / import_selections / classify_raw /
  select_lots) [R0-M4] — NAVIGATE to the SUBCOMMAND's `Command` (`Cli::command().find_subcommand(...)`,
  recursing into `reconcile`), render ITS long-help, assert it CONTAINS the format token + example (e.g.
  import-selections's help contains `disposal_ref,origin_event_id,split_sequence,sat`). Pins requirement 3
  for `--help`. (The root `Cli::command()` long-help does NOT carry subcommand args — the C1 lesson.)
- **`manpage_covers_every_subcommand`** [C1-aware] — walk `Cli::command().get_subcommands()` RECURSIVELY;
  assert each produces a committed page `docs/man/btctax[-<path>].1`. Guards drift (a new subcommand without
  a page fails CI).
- **`file_format_examples_present_in_manpage`** [C1] — each format example appears in ITS subcommand's page
  (e.g. `btctax-reconcile-import-selections.1` contains the CSV header; `btctax-init.1` the armor block;
  `btctax-reconcile-classify-raw.1` the JSON; `btctax-reconcile-select-lots.1` the `#split:sat` spec).
- **`gen_docs_is_deterministic`** — running the generator twice yields byte-identical `.1` (no timestamps /
  no `#[command(version)]`); the committed `docs/man/*.1` match a fresh generation (fails if stale).
- **`manpages_have_required_sections`** — each page has `NAME`/`SYNOPSIS`; the ROOT `btctax.1` has
  `DESCRIPTION`/`FILES`/`EXAMPLES`; the hand-authored TUI pages document their tabs + keys (e.g.
  `btctax-tui-edit.1` lists `?`, `V`, `O`).
- PDF: a smoke check that `groff -man -T pdf` produces a valid `%PDF` from each `.1` (the build step, not a unit test).

## Plan (TDD, phased — man pages FIRST per the user)
- **Task 1 — the inline file-format long-help (source of truth):** extend the clap `///` on the 6 file/format
  args (init `--key-backup`, backup-key `--out`, export-snapshot `--out`, import-selections `<csv>`,
  classify-raw `--payload-json`, select-lots `--from`) with FORMAT + EXAMPLE, each with
  **`#[arg(verbatim_doc_comment)]`** [I1] so multi-line examples survive. Read EXACT formats from source (the
  export CSV headers from `render.rs`, the classify-raw serde shape, the armor). KATs `help_documents_*`
  (navigating to the subcommand `Command`).
- **Task 2 — `crates/xtask` generator + the man tree:** recurse `get_subcommands()`, render one `.1` per
  command (`btctax.1` + `btctax-<path>.1`, git-style); stitch hand-authored DESCRIPTION/FILES/EXAMPLES into
  the ROOT `btctax.1`; commit `docs/man/*.1`. KATs `manpage_covers_every_subcommand`,
  `file_format_examples_present`, `gen_docs_is_deterministic`.
- **Task 3 — hand-authored `btctax-tui.1` + `btctax-tui-edit.1`** (tabs + keys; editor reuses the `?` keymap);
  commit. KAT `manpages_have_required_sections`.
- **Task 4 — PDFs:** `groff -T pdf` for all three → `docs/pdf/*.pdf`; wire into the generator/`make docs`.
- **Task 5 — whole-diff review** + full workspace suite + FOLLOWUPS + a `docs/man/README` (how to regenerate + `man -l`).

## Gotchas
- **[C1] clap_mangen does NOT render subcommand args from the root** — a single `Man::new(Cli::command())`
  yields a subcommand NAME list only. RECURSE `get_subcommands()` and render one page per command
  (`btctax-<path>.1`); the file-format long-help lands on ITS subcommand page. A single-`btctax.1` design is
  wrong.
- **[I1] `verbatim_doc_comment`** on the file-format args — else clap reflows the multi-line armor/CSV/JSON
  examples into one paragraph (both `--help` and the man page). Mirror the existing use at main.rs:111-119.
- **One source of truth** — file-format docs live in the clap long-help ONLY; the subcommand man page inherits
  them via clap_mangen. Do NOT hand-duplicate them in the roff (drift). The hand-authored roff is ONLY the
  root DESCRIPTION/FILES-overview/EXAMPLES + the TUI content.
- **Determinism** — the generator must not embed dates/versions that change per-run, or the committed `.1`
  churns and the `gen_docs_is_deterministic` KAT fails. (clap_mangen is deterministic; ensure the stitched
  roff is too — no `date`.)
- **Read the REAL formats from source** — the export CSV headers, the classify-raw serde JSON, and the
  key-armor header must match the actual code, not be guessed. Each example is verified against source at
  write time (STANDARD_WORKFLOW citation discipline).
- **clap_mangen is generator-only** — do not add it to the shipped `btctax` binary's runtime deps.
- **Man before PDF** (user ordering) — Tasks 1-3 land the man pages; Task 4 adds PDFs.
