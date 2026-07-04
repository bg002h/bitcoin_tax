# SPEC — project README (install instructions + tutorial)

**Source baseline:** `main` @ `f7408f3` (branch `feat/readme`). **Review status: DRAFT — awaiting R0.**
**Lineage:** user request (2026-07-04): "a readme with install instructions and tutorial." Crate publishing is
the NEXT task (#35) — so install is source-based for now, with a note that crates.io install is coming.

## Goal
A top-level `README.md`: what btctax is, how to install it, and a hands-on tutorial that walks the canonical
workflow end-to-end with REAL, verified commands. End-user focused (a contributor/dev section is a short
pointer, not the bulk).

## Audience & decisions
- **End user** who wants to compute their US Bitcoin taxes offline. Not a contributor guide (link to
  `STANDARD_WORKFLOW.md` + `docs/man/` for depth).
- **Install = from source** (`cargo install --path …` or `git clone` + `cargo build --release`) — the crates
  are not published yet; a one-line "coming to crates.io" note (do NOT document a `cargo install btctax` that
  doesn't work yet; #35 will update this).
- **Synthetic example data** in the tutorial — a tiny inline CSV, NEVER the user's real `vault.pgp` / ReadOnly
  files. Show `BTCTAX_PASSPHRASE` for non-interactive steps + note the interactive prompt.
- **Every command verified** against the actual CLI (signatures confirmed at spec time; the whole-diff re-runs
  the whole tutorial end-to-end).

## Structure (top → bottom)
1. **Title + one-liner** — "btctax — an offline, single-user US Bitcoin tax ledger." Badges optional (CI).
2. **What it is / why** — offline (no network, FR: NFR2), encrypted local vault, event-sourced ledger, computes
   per-lot basis/gain, ST/LT, Form 8949 / Schedule D, income, gifts/donations, safe-harbor lot ID. Explicitly
   **not tax advice**; US-only; BTC-only (per scope).
3. **The three binaries** — `btctax` (the CLI engine), `btctax-tui` (read-only viewer), `btctax-tui-edit`
   (interactive reconcile editor; `?` for the keymap). One line each.
4. **Install** —
   - Prereqs: Rust ≥ **1.88** (MSRV), a C toolchain (rusqlite `bundled` builds vendored SQLite).
   - From a clone: `cargo install --path crates/btctax-cli` (+ `btctax-tui`, `btctax-tui-edit`), or
     `cargo build --release` → `target/release/`.
   - Platforms: Linux/macOS/Windows (CI-tested — link the matrix). "crates.io publish coming (#35)."
5. **Quickstart tutorial** — the canonical flow, each step a verified command + a one-line "what it does":
   1. `btctax --vault ./vault.pgp init --key-backup ./vault.key` (creates the encrypted vault + key backup;
      document the key-backup format link to `btctax-init(1)`). Set `BTCTAX_PASSPHRASE` or prompt.
   2. `btctax --vault ./vault.pgp import statement.csv …` (ingest exchange CSVs; a tiny synthetic Coinbase-shape
      example inline). Note the ReadOnly-files convention (keep real data out of the repo).
   3. `btctax --vault ./vault.pgp verify` (shows blockers/advisories; exit 1 iff Hard blockers — the gate).
   4. **Reconcile** the blockers — show ONE representative CLI example
      (`reconcile classify-inbound-self-transfer <IN_REF>`) AND point to `btctax-tui-edit` for the guided,
      full-surface flow (the 25 reconcile subcommands + bulk ops are man-page/`--help` territory, not the README).
   5. `btctax --vault ./vault.pgp report --tax-year 2025` (the per-year TaxResult + Schedule D). Also plain
      `report` for holdings/realized.
   6. `btctax --vault ./vault.pgp export-snapshot --out ./export --tax-year 2025` (decrypted SQLite + the
      Form 8949 / Schedule D CSVs — the NFR2 plaintext exception; **warn: contains your data, keep it out of
      any repo / git-ignored**).
6. **Getting help** — `btctax <cmd> --help` (rich, with inline file-format docs), `man -l docs/man/btctax.1`
   (or the PDFs via `make bundles`), the `?` overlay in the editor.
7. **Data & privacy** — offline; the vault is passphrase-encrypted at rest; NEVER commit `vault.pgp` / exports
   (`.gitignore` already guards them); this is software, not tax advice.
8. **Build/test/docs (contributors)** — `cargo test --workspace`, `make docs`; link `STANDARD_WORKFLOW.md`.
9. **License** — MIT OR Unlicense (matches `Cargo.toml`).

## Non-goals (YAGNI)
- No full command reference (that's the man pages — link, don't duplicate).
- No architecture deep-dive (link STANDARD_WORKFLOW / design/).
- No `cargo install btctax` (crates.io) until #35 ships.

## KAT / verification
No unit test (it's prose). **Whole-diff = run the ENTIRE tutorial end-to-end** on a throwaway vault (synthetic
CSV) and confirm each command succeeds + the described output appears; confirm every flag matches `--help`;
confirm no real-data path is referenced. Markdown lints clean (no broken relative links to `docs/man/`,
`STANDARD_WORKFLOW.md`).

## Scope / SemVer
Docs-only (new `README.md`). No code, no crate change. PATCH-class.

## Plan
- **Task 1** — write `README.md` per the structure; every command copied from verified `--help`.
- **Task 2** — whole-diff: execute the full tutorial on a synthetic vault (throwaway dir), fix any drift; verify
  links resolve; confirm privacy warnings present. Ship.
