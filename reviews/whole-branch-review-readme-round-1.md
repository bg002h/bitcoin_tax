# Whole-diff review (Phase E) — feat/readme — round 1

**Verdict: 0 Critical / 0 Important / 0 Minor / 0 Nit — SHIP.**

Independent Phase-E review. Diff `main (f7408f3)..HEAD` — new `README.md` (docs-only) + the R0-GREEN spec +
reviews. Contract: `design/SPEC_readme.md` (R0-GREEN, 2 rounds).

## The KAT for a tutorial: it runs verbatim
Executed the ENTIRE README tutorial (steps 1–6) verbatim against the built `btctax` binary on a throwaway vault
— every command works with the exact outputs/exit codes the README promises:
- **Step 1** — `init --key-backup ./vault-key-backup.asc` created `vault.pgp` (57 KB) + the sidecar `vault.key`
  (907 B) + the DISTINCT armored backup `vault-key-backup.asc` (1400 B). No clobber (I1).
- **Step 2** — the pinned Coinbase CSV imported: `parsed 1 rows -> 1 BTC events` (I4 — the exact header +
  `Receive` row + `Asset=BTC` parse cleanly).
- **Step 3** — `verify` printed `[UnknownBasisInbound] import|coinbase|in|RCV-1 :: unclassified TransferIn —
  basis unknown` and exited **1** (M1 — framed as expected; I5 — the ref matches the README byte-for-byte).
- **Step 4** — `reconcile classify-inbound-self-transfer 'import|coinbase|in|RCV-1'` (single-quoted) →
  `Recorded decision decision|1`; re-`verify` exited **0**.
- **Step 5** — `tax-profile --year 2025 --filing-status single …` (all four flags) saved; `report --tax-year
  2025` printed a real "Federal tax attributable to crypto — tax year 2025" result, NOT "not computable" (I2).
- **Step 6** — `export-snapshot --out ./export --tax-year 2025` wrote `snapshot.sqlite` + `disposals/form8283/
  form8949/income/lots/removals/schedule_d.csv`. `git check-ignore` confirms these CSVs are **NOT** ignored →
  the README's "export outside the repo" warning (I3) is correct and load-bearing.

## Accuracy / framing
- Install: MSRV 1.88, C-toolchain (bundled SQLite), `cargo install --path crates/{btctax-cli,btctax-tui,
  btctax-tui-edit}` — correct; no premature `cargo install btctax` (crates.io deferred to #35).
- All relative links resolve (`STANDARD_WORKFLOW.md`, `docs/man/*.1`, `Makefile` with `docs`/`bundles`).
- License `MIT OR Unlicense` matches `Cargo.toml`. "not tax advice / US-only / BTC-only" stated up front.
- No real-data path referenced anywhere; synthetic example only.

## Scope / SemVer
Docs-only (new `README.md`). No code change. PATCH-class.

**SHIP.**
