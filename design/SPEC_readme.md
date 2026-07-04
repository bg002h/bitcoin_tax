# SPEC — project README (install instructions + tutorial)

**Source baseline:** `main` @ `f7408f3` (branch `feat/readme`). **Review status: R0-GREEN (2 rounds; 0C/0I).
Reviews: `reviews/R0-spec-readme-round-{1,2}.md`. Round 2 EXECUTED the full tutorial verbatim on a throwaway
vault — every step works end-to-end with the promised exit codes/outputs. Cleared to implement.**
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
5. **Quickstart tutorial** — the canonical flow; each step a verified command + a one-line "what it does".
   Uses `export BTCTAX_PASSPHRASE=...` up front (else each command prompts).
   1. **[R0-I1]** `btctax --vault ./vault.pgp init --key-backup ./vault-key-backup.asc` — creates the
      encrypted vault. `init` ALSO auto-writes the sidecar `./vault.key` (needed to open the vault), so the
      `--key-backup` path MUST be distinct (an offline backup) — do NOT point it at `./vault.key` (that
      clobbers the live sidecar). Link `btctax-init(1)` for the armor format.
   2. **[R0-I4]** `btctax --vault ./vault.pgp import ./coinbase.csv` — ingest exchange CSVs. Pin the EXACT
      importable synthetic Coinbase example (verified against `btctax-adapters/src/coinbase.rs`; header needs
      `ID`/`Transaction Type`/`Asset`/`Quantity Transacted`/`Subtotal`, `Asset=BTC`, and a `Receive` row so
      step 4 has something to reconcile):
      ```
      Transactions
      User,00000000-0000-0000-0000-000000000000
      ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address
      RCV-1,2025-03-01 12:00:00 UTC,Receive,BTC,0.05000000,USD,84000.00,,,,,bc1qsender,
      ```
      Note the ReadOnly-files convention (keep real exchange data out of the repo).
   3. **[R0-M1]** `btctax --vault ./vault.pgp verify` — shows blockers/advisories. A fresh import of an inbound
      Receive legitimately produces a Hard `UnknownBasisInbound` blocker → `verify` exits 1. Frame this as the
      EXPECTED next-step signal, not an error; it's the gate that tax computation waits on.
   4. **[R0-I5]** Reconcile the blocker. `verify`'s Hard-blocker line prints the event ref, e.g.
      `[UnknownBasisInbound] import|coinbase|in|RCV-1 :: ...`. Feed it back (SINGLE-QUOTED — the ref contains
      `|`, a shell pipe): `btctax --vault ./vault.pgp reconcile classify-inbound-self-transfer
      'import|coinbase|in|RCV-1'`. Then point to `btctax-tui-edit` for the guided, full-surface flow (the 24
      reconcile subcommands + bulk ops are man-page/`--help` territory, not the README).
   5. **[R0-I2]** Set a tax profile (required before per-year tax — else `report --tax-year` prints "not
      computable"): `btctax --vault ./vault.pgp tax-profile --year 2025 --filing-status single
      --ordinary-taxable-income 80000 --magi-excluding-crypto 80000 --qualified-dividends 0`. Then
      `btctax --vault ./vault.pgp report --tax-year 2025` (per-year TaxResult + Schedule D). Plain `report`
      (or `report --year 2025`) shows holdings/realized without a profile.
   6. **[R0-I3]** `btctax --vault ./vault.pgp export-snapshot --out ./export --tax-year 2025` — writes the
      decrypted SQLite + the Form 8949 / Schedule D CSVs (the NFR2 plaintext exception). **⚠ These files
      contain your tax data and are NOT git-ignored** (only `snapshot.sqlite` matches a rule; the CSVs do
      not) — write `--out` to a directory OUTSIDE any git repo.
6. **Getting help** — `btctax <cmd> --help` (rich, with inline file-format docs), `man -l docs/man/btctax.1`
   (or the PDFs via `make bundles`), the `?` overlay in the editor.
7. **Data & privacy** — offline; the vault is passphrase-encrypted at rest. NEVER commit `vault.pgp` (the
   `.gitignore` guards `vault*`/`*.pgp`/`*.asc`) — but **[R0-I3] it does NOT guard the `export-snapshot` CSVs**
   (only `snapshot.sqlite` matches a rule), so write exports to a directory OUTSIDE any git repo. This is
   software, not tax advice; US-only; BTC-only.
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
