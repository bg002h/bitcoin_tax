# R0 review — `design/SPEC_readme.md` (round 2)

**Artifact:** `design/SPEC_readme.md` (folded — end-user README: install + init→import→verify→reconcile→report→export tutorial).
**Baseline:** branch `feat/readme` @ `1750979` (`main` == `f7408f3`); source verified against the working tree at review time.
**Reviewer role:** independent architect, read-only. **Bar:** 0 Critical / 0 Important.
**Method:** verified every folded finding against current source AND executed the ENTIRE tutorial verbatim on a throwaway vault with the release-built binary (the spec's own KAT). Every step's exit code and output is quoted below.

## Verdict

**0 Critical / 0 Important / 0 Minor / 0 Nit — R0-GREEN.**

All five round-1 Important items and the Minor + Nit are resolved, and — the thing that actually matters — the tutorial run **verbatim, end-to-end, works**. I re-ran the exact commands from `SPEC_readme.md:41-72` in order; each produced the promised result.

### End-to-end tutorial transcript (verbatim commands, throwaway dir, `BTCTAX_PASSPHRASE` set)

| Step | Command (verbatim from spec) | Result |
|---|---|---|
| 1 | `init --key-backup ./vault-key-backup.asc` | exit 0; wrote **three** distinct files: `vault.pgp`, sidecar **`vault.key`** (907 B binary), `vault-key-backup.asc` (1400 B armored) |
| 2 | `import ./coinbase.csv` (the pinned CSV) | exit 0; `parsed 1 rows -> 1 BTC events (0 dropped no-BTC, 0 unclassified)` |
| 3 | `verify` | **exit 1**; `[UnknownBasisInbound] import\|coinbase\|in\|RCV-1 :: unclassified TransferIn — basis unknown` |
| 4 | `reconcile classify-inbound-self-transfer 'import\|coinbase\|in\|RCV-1'` | exit 0; `Recorded decision decision\|1` |
| 4b | `verify` (re-run) | **exit 0**; Hard blockers 0 (one *Advisory* SelfTransferInboundZeroBasis, non-gating) |
| 5a | `tax-profile --year 2025 --filing-status single --ordinary-taxable-income 80000 --magi-excluding-crypto 80000 --qualified-dividends 0` | exit 0; `Tax profile for 2025 saved.` |
| 5b | `report --tax-year 2025` | exit 0; prints a **real** `Federal tax attributable to crypto — tax year 2025` + `Schedule D` block (NOT "not computable") |
| 6 | `export-snapshot --out ./export --tax-year 2025` | exit 0; wrote `snapshot.sqlite` + 7 CSVs (`lots/disposals/removals/income/form8949/schedule_d/form8283`) |

Plain `report` and `report --year 2025` (the step-5 asides) also both exit 0 and show the reconciled lot as a holding.

---

### [I1] RESOLVED — distinct key-backup path

Spec `:41-44` now uses `init --key-backup ./vault-key-backup.asc` and adds the one-line warning that `init` ALSO auto-writes the sidecar `./vault.key`, so the backup path must be distinct (never `./vault.key`).

Verified: `paths.rs:24 suffixed_key` maps `vault.pgp → vault.key` (with a `debug_assert` guard against a `.key`-suffixed vault). At runtime, `init --key-backup ./vault-key-backup.asc` produced `vault.key` (907 B binary sidecar) AND `vault-key-backup.asc` (1400 B armored) as **separate** files — no clobber. The distinct-path guidance is correct and load-bearing.

### [I2] RESOLVED — mandatory `tax-profile` step precedes `report --tax-year`

Spec `:64-68` inserts the `tax-profile` step before the report. Confirmed against `crates/btctax-cli/src/cli.rs:121-181` + `main.rs:315-404`:

- **All four flags exist and are mandatory when setting a profile.** They are declared `Option<..>` at the clap layer but enforced as required in `main.rs:317-343` (`--filing-status` / `--ordinary-taxable-income` / `--magi-excluding-crypto` / `--qualified-dividends` each `ok_or_else(Usage(... "is required when setting a profile"))`). The other eight profile flags legitimately default to 0, so the spec's exact 5-token invocation (year + 4) is sufficient — confirmed at runtime (`Tax profile for 2025 saved.`).
- **`single` is a valid `--filing-status` value.** `tax-profile --help` → `[possible values: single, mfj, mfs, hoh, qss]` (`FilingStatusArg`, cli.rs:586-593).
- **With a profile set, `report --tax-year 2025` computes.** Runtime output is a real TaxResult (`net short-term/long-term`, `TOTAL federal tax attributable`, Schedule D part totals), not the `NotComputable(TaxProfileMissing)` message from `tax/compute.rs`. I2's failure mode is gone.

### [I3] RESOLVED — export CSVs are NOT git-ignored; export outside the repo

Spec `:69-72` + `:76-78` now state plainly that the CSVs are NOT git-ignored (only `snapshot.sqlite` matches) and instruct exporting to a directory OUTSIDE any git repo. Verified with `git check-ignore -v` against every emitted name:

- **IGNORED:** `snapshot.sqlite` only (`.gitignore:18 *.sqlite`).
- **NOT ignored:** `export/`, `lots.csv`, `disposals.csv`, `removals.csv`, `income.csv`, `form8949.csv`, `schedule_d.csv`, `form8283.csv`, `schedule_se.csv`.

The spec's claim ("only `snapshot.sqlite` matches a rule; the CSVs do not") is exactly right, and the "write `--out` outside any git repo" guidance is the correct fix.

### [I4] RESOLVED — pinned CSV actually imports, producing exactly one pending inbound

The pinned CSV (spec `:50-53`) imported cleanly at runtime: `parsed 1 rows -> 1 BTC events`, which `verify` reports as `unknown-basis inbounds: 1`. Cross-checked against `btctax-adapters/src/sources/coinbase.rs` + `parse.rs` + `read.rs`:

- **Detection** (`coinbase.rs:82-84`): header line contains `Transaction Type` + `Quantity Transacted` + `Subtotal` (within the 4096-byte peek) → recognized as Coinbase.
- **Header signature** (`coinbase.rs:41`, `read.rs:77-84`): the two preamble lines (`Transactions`, `User,00000…`) contain neither `Transaction Type` nor `Quantity Transacted`; the real header (line 3) AND-matches all of `ID`/`Transaction Type`/`Quantity Transacted` and is selected.
- **Asset gate** (`coinbase.rs:120-124`): `Asset=BTC` → kept.
- **Timestamp** (`parse.rs:118-122`): `2025-03-01 12:00:00 UTC` hits the dedicated trailing-` UTC` arm (there is even a unit test at `parse.rs:276-282` asserting exactly this string). No parse error.
- **Empty `Subtotal`/`Total`/`Fees`/`Recipient` cells** are harmless: `RawRow::opt` returns `None` for blank cells (`read.rs:39-44`), so `subtotal`/`fees` default to `Usd::ZERO` — no `parse_usd("")` failure. (`parse_usd` also handles empty as 0 anyway, `parse.rs:38-40`.)
- **`Receive` → `TransferIn`** (`coinbase.rs:176-183`) → a Hard `UnknownBasisInbound` — the exact target step 4 reconciles.

No header-column mismatch; the 13-column data row lines up with the 13-column header. The importer accepts it. The spec correctly cites the adapter as source of truth.

### [I5] RESOLVED — `import|coinbase|in|RCV-1` ref shape + shell quoting

Spec `:59-63` shows the ref coming from `verify`'s Hard-blocker line and reconciles it single-quoted. Verified:

- **Ref shape** matches `normalize.rs:51-52` (`native(In,"RCV-1")` → `SourceRef("in|RCV-1")`) + `identity.rs:86-90` (`canonical()` → `import|coinbase|in|RCV-1`).
- **`verify` surfaces it** on the Hard-blocker line exactly as `[UnknownBasisInbound] import|coinbase|in|RCV-1 :: unclassified TransferIn — basis unknown` (observed at runtime; render path `render.rs`).
- **Single-quoting is correct** — the unquoted ref would be three shell pipes; the single-quoted `'import|coinbase|in|RCV-1'` was accepted verbatim and recorded `decision|1`. Re-running `verify` then cleared to exit 0.

### [M1] RESOLVED — fresh-import exit-1 framed as expected

Spec `:56-58` frames the post-import Hard blocker + `verify` exit 1 as the EXPECTED next-step gate, not a failure, and tells the user to re-run `verify` after reconciling. Runtime confirms the exact sequence: exit 1 before reconcile, exit 0 after. Accurate.

### [N1] RESOLVED — reconcile subcommand count

Spec `:63` says "24 reconcile subcommands". `btctax reconcile --help` lists exactly **24** (excluding the auto-generated `help`), matching the 24 `enum Reconcile` variants in `cli.rs:235-576` (LinkTransfer … MatchSelfTransfers). Correct.

---

## No new drift

- **License / MSRV:** `Cargo.toml` → `license = "MIT OR Unlicense"`, `rust-version = "1.88"` — both match spec `:35,80`.
- **Binaries / install:** three bins — `btctax` (`crates/btctax-cli`, `[[bin]] name="btctax"`), `btctax-tui`, `btctax-tui-edit`; `cargo install --path crates/btctax-cli` installs `btctax`; the tui crate paths in the spec's `(+ btctax-tui, btctax-tui-edit)` resolve. xtask correctly excluded.
- **C-toolchain note:** `rusqlite { features=["bundled"] }` in both `btctax-store` and `btctax-cli` Cargo.toml → the vendored-SQLite / C-toolchain prereq is accurate.
- **`report` flags:** `--year` (calendar filter) and `--tax-year` (compute) both exist and are independent — the step-5 aside is correct.
- **Links resolve:** `docs/man/btctax.1`, `docs/man/btctax-init.1`, `STANDARD_WORKFLOW.md`, and `Makefile` (with both `docs:` and `bundles:` targets) all exist.

## Bottom line

Would the tutorial, run verbatim, actually work end-to-end? **Yes — verified by executing it.** Every command succeeds, every exit code matches the framing, every referenced flag/value/link is real, and the one place a naive README would silently leak tax data (the non-ignored export CSVs) is now called out correctly.

**R0-GREEN.** Proceed to Plan.
