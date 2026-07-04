# SPEC — man pages + PDFs for the three btctax binaries (with inline file-format docs)

**Source baseline:** `main` @ `13b3b17` (branch `feat/binary-docs`). **Review status: DRAFT — awaiting R0
(2-round loop to 0C/0I).**
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
- **`btctax.1` [HYBRID]:** a new workspace generator (a `[[bin]]` `gen-docs` in `btctax-cli`, or `crates/xtask`)
  builds the clap `Command` (`Cli::command()`), renders it via **`clap_mangen`** (all ~33 subcommands + the
  file-format long-help), then STITCHES hand-authored roff sections — a top **DESCRIPTION** overview, a
  consolidated **FILES** section, and **EXAMPLES** — via `Man::render_*` hooks + appended roff.
- **`btctax-tui.1` / `btctax-tui-edit.1` [HAND-AUTHORED]:** interactive apps with a thin clap surface
  (`--vault`, passphrase prompt) — clap_mangen gives only a stub, so these are hand-authored roff/markdown:
  SYNOPSIS, the tab set (Holdings/Disposals/Income/Tax/Forms/Compliance), keybindings. The editor's keymap
  is the `?`-overlay content already in `draw_help_overlay` (reuse verbatim — single source with the overlay).
- **PDFs:** `groff -man -T pdf <x.1> > <x.pdf>` (verified `groff -T pdf` emits `%PDF-1.7`). Man FIRST, PDF after.
- **Locations:** generated `docs/man/{btctax,btctax-tui,btctax-tui-edit}.1` (committed — browsable/installable
  without a build), hand-authored roff/markdown supplements in `docs/man/src/`, `docs/pdf/*.pdf`, and the
  `gen-docs` bin. A `make docs` / `just docs` (or the bin itself) regenerates all.

## The inline file-format docs [requirement 3 — the enumerated set]
Extend the clap `///` long-help on each of these with **FORMAT + a text EXAMPLE** (flows to `--help` + `btctax.1`):
- **`init --key-backup <FILE>` and `backup-key --out <FILE>`** — an ASCII-armored, passphrase(S2K)-encrypted
  private key, owner-only (mode 0600). Example (structure, NOT a real key):
  ```
  -----BEGIN PGP PRIVATE KEY BLOCK-----
  ... base64 armor of the S2K-encrypted secret key ...
  -----END PGP PRIVATE KEY BLOCK-----
  ```
- **`export-snapshot --out <DIR>`** — a directory receiving the decrypted SQLite DB + projection CSVs
  (`disposals.csv`, `removals.csv`, `income_recognized.csv`; with `--tax-year`: `form8949.csv`,
  `schedule_d.csv`). Document each CSV's HEADER row (esp. the `event` column that reconcile commands consume).
  Example (e.g. `removals.csv`): `event,disposed_at,kind,btc,...` + one sample row. [R0/impl: read the exact
  headers from the export writer.]
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

## clap_mangen integration
- Add `clap_mangen = "0.2"` (compatible with clap 4.5) as a **build/dev dependency** of the generator only —
  NOT a runtime dep of `btctax` (keeps the shipped binary lean). The generator calls `Cli::command()` (derive
  already provides it) → `clap_mangen::Man::new(cmd)`.
- The generator is deterministic (no timestamps) so the committed `.1` files are reproducible (a KAT
  regenerates and asserts no diff).

## SemVer / lockstep
- **Docs-only + additive clap long-help.** No flag NAME/arg changes (only extended `///` help text) → no GUI
  `schema_mirror` concern, no behavior change, no new runtime dep on the shipped binaries. New `docs/` tree +
  a generator bin. PATCH-class.

## KATs
- **`help_documents_key_backup_format`** / `..export_snapshot` / `..import_selections` / `..classify_raw` /
  `..select_lots` — `Cli::command()`'s rendered long-help for each command CONTAINS its format token + example
  (e.g. the import-selections help contains `disposal_ref,origin_event_id,split_sequence,sat`). Pins
  requirement 3 for `--help`.
- **`manpage_covers_every_subcommand`** — the generated `btctax.1` contains every clap subcommand name
  (walk `Cli::command().get_subcommands()` recursively; assert each `.get_name()` appears). Guards drift as
  the CLI evolves (mirrors the help-overlay completeness KAT).
- **`gen_docs_is_deterministic`** — running the generator twice yields byte-identical `.1` (no timestamps);
  and the committed `docs/man/*.1` match a fresh generation (fails CI if stale).
- **`manpages_have_required_sections`** — each `.1` has `NAME`/`SYNOPSIS`/`DESCRIPTION`; `btctax.1` has `FILES`
  + `EXAMPLES`; the TUI pages document their tabs + keys (e.g. `btctax-tui-edit.1` lists `?`, `V`, `O`).
- **`file_format_examples_present_in_manpage`** — `btctax.1` contains each format example (the CSV header, the
  armor block, the JSON, the lot-pick spec).
- PDF: a smoke check that `groff -man -T pdf` produces a valid `%PDF` from each `.1` (the build step, not a unit test).

## Plan (TDD, phased — man pages FIRST per the user)
- **Task 1 — the inline file-format long-help (source of truth):** extend the clap `///` on the 5 file/format
  args with FORMAT + EXAMPLE (read exact formats from source: the export CSV headers, the classify-raw serde
  shape, the armor). KATs `help_documents_*`.
- **Task 2 — `gen-docs` generator + `btctax.1`:** clap_mangen render + hand-authored DESCRIPTION/FILES/EXAMPLES
  stitching; commit `docs/man/btctax.1`. KATs `manpage_covers_every_subcommand`, `file_format_examples_present`,
  `gen_docs_is_deterministic`.
- **Task 3 — hand-authored `btctax-tui.1` + `btctax-tui-edit.1`** (tabs + keys; editor reuses the `?` keymap);
  commit. KAT `manpages_have_required_sections`.
- **Task 4 — PDFs:** `groff -T pdf` for all three → `docs/pdf/*.pdf`; wire into the generator/`make docs`.
- **Task 5 — whole-diff review** + full workspace suite + FOLLOWUPS + a `docs/man/README` (how to regenerate + `man -l`).

## Gotchas
- **One source of truth** — file-format docs live in the clap long-help ONLY; the man page inherits them via
  clap_mangen. Do NOT hand-duplicate them in the roff (drift). The hand-authored roff is ONLY the
  DESCRIPTION/FILES-overview/EXAMPLES/TUI content.
- **Determinism** — the generator must not embed dates/versions that change per-run, or the committed `.1`
  churns and the `gen_docs_is_deterministic` KAT fails. (clap_mangen is deterministic; ensure the stitched
  roff is too — no `date`.)
- **Read the REAL formats from source** — the export CSV headers, the classify-raw serde JSON, and the
  key-armor header must match the actual code, not be guessed. Each example is verified against source at
  write time (STANDARD_WORKFLOW citation discipline).
- **clap_mangen is generator-only** — do not add it to the shipped `btctax` binary's runtime deps.
- **Man before PDF** (user ordering) — Tasks 1-3 land the man pages; Task 4 adds PDFs.
