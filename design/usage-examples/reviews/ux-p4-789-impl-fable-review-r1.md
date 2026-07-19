# Phase 2 ("Legibility") independent review — UX-P4-7 / UX-P4-8 / UX-P4-9

Reviewed commits `34e9945` (2c), `66b4bad` (2b), `fa4badc` (2a) on `feat/post-v070-product-cycle`, diff verified against current source at HEAD. Validation: `make check` green — 2033 passed / 8 skipped, clippy `-D warnings` clean. (Per project memory: this is nextest+clippy only, not the CI-only fmt/msrv/pii-scan/net-isolation jobs.)

## CRITICAL

None.

## IMPORTANT

**I1 — UX-P4-8 regressed the TUI unlock screen's missing-vault message and left the curated handler as dead code (viewer AND editor).**
`crates/btctax-cli/src/session.rs:392-393` now converts every `StoreError::Io` out of `Vault::open` into `CliError::PathIo`. But `map_open_error` — `crates/btctax-tui/src/unlock.rs:233-252`, fed *exclusively* from `Session::open` at `unlock.rs:121-123` — still matches `CliError::Store(StoreError::Io(io_err)) if io_err.kind() == NotFound` (`unlock.rs:243-247`) to produce the curated `no vault at <path>`. That arm is now unreachable; it knows nothing of `PathIo`.
Failure scenario: launch `btctax-tui --vault /home/u/Documents/BitcoinTax/vault.pgp` where the vault does not exist. Pre-change the unlock error line read `✗ no vault at /home/u/Documents/BitcoinTax/vault.pgp`. Post-change it falls to the `_ =>` arm (`unlock.rs:250`) and reads `✗ vault error: io /home/u/…/vault.pgg: No such file or directory (os error 2) (check the --vault path, or run `btctax init` to create a new vault)` — and `draw_unlock_screen` (`crates/btctax-tui/src/draw.rs:36-79`) renders it in a `Paragraph` with no `.wrap()`, so on an 80-column terminal the line is clipped mid-clause and the hint is never visible. The path is also redundant with the screen's own `Vault:` line. The editor is equally affected (`crates/btctax-tui-edit/src/editor.rs:371-393` routes through the same `open_session`/`map_open_error`). No test pins the missing-vault unlock message, so the suite stayed green — the "Legibility" phase made a legibility surface worse. Fold: teach `map_open_error` a `CliError::PathIo { source, .. } if source.kind() == NotFound` arm preserving `no vault at <path>` (or match `PathIo` generally), and pin it.

**I2 — UX-P4-8 misses sibling `--out` sites; `export-irs-pdf --out` still reproduces the exact original symptom.**
The item's contract term is class-level: SPEC §4 "attach path + one-clause hint at vault-open … and `--out`"; PLAN 2b same; the FOLLOWUP (`FOLLOWUPS.md:2231-2236`) says "the vault-open and **export-out call sites**". Only `export-snapshot`'s two sites were enriched. Unenriched `--out` sites, verified in current source:
- `export_irs_pdf` — `crates/btctax-cli/src/cmd/admin.rs:259` `fsperms::mkdir_owner_only(out_dir)?` → bare `CliError::Io`. Failure scenario: `btctax export-irs-pdf --out <existing-file>` prints `error: io: File exists (os error 17)` — byte-for-byte the symptom UX-P4-8 exists to kill, on the flagship export.
- `export_full_return` — `admin.rs:494`, same bare `mkdir_owner_only(out_dir)?`.
- `backup_key` — `admin.rs:411-414`, `.backup_key(out_path)?` propagates pathless `StoreError::Io`.

The spec's parenthetical (`admin.rs:82`) is an anchor of the instance known at spec time (this project's own rule: citations decay; the class term governs). Fold is mechanical: the same `map_err(store_io_with_path/cli_io_with_path …, EXPORT_OUT_HINT)` at those sites + a KAT. If the author establishes the design intent really was export-snapshot-only, the alternative resolution is an explicit spec amendment plus a filed follow-up with an owning phase — but as the artifacts read today, the code does not meet the item's stated class.

## MINOR

**M1 — Overclaiming comment at the CSV-export wrap.** `crates/btctax-cli/src/cmd/admin.rs:113-115` claims "same path context for **any** CSV write that fails after the snapshot (e.g. a mid-write I/O error) — the CSV writers `?` on pathless `io::Error`." False for the writes themselves: `write_record`/`flush` in `write_csv_exports` (`crates/btctax-cli/src/render.rs:684-706` etc.) fail as `csv::Error` → `CliError::Csv`, which `cli_io_with_path` (`crates/btctax-cli/src/lib.rs:177-186`) deliberately passes through — pathless. Only the `mkdir_owner_only`/`open_owner_only` `io::Error` paths get the path. Behavior meets the spec KAT (collision names the path); fix the comment, or also enrich `Csv` errors whose inner kind is Io.

**M2 — `EXPORT_OUT_HINT` is mutation-vulnerable.** No test pins the hint clause on the `--out` surface: `io_error_context.rs:544-553` asserts only the path (this matches the spec KAT literally, which asks the hint only of the vault case). A mutation emptying `EXPORT_OUT_HINT` (`crates/btctax-cli/src/lib.rs:153-154`) survives the whole suite. The vault hint is properly pinned (`io_error_context.rs:519-520`). Per the project's untested-guard standard, add one `contains` on the hint.

## NIT

**N1** — `store_io_with_path` at `Session::open` also enriches Io from `VaultLock::acquire`/`recover_target`, and the existing-vault-with-missing-`.key` case (`crates/btctax-store/src/vault.rs:135` `std::fs::read(&kp)?`): the message then pairs "No such file or directory" with a `--vault` path that exists and a "run `btctax init`" suggestion that would refuse (`AlreadyExists`). Strictly more information than the pre-change bare error; noting only.

**N2** — `crates/btctax-core/src/whatif.rs:16` module doc still reads "an empty as-of pool ⇒ `NoLots`"; the variant (now explicitly) also fires on mere insufficiency. One stale clause.

**N3** — Residue observations, outside this item's scope: `crates/btctax-cli/src/main.rs:2029` still Debug-prints `MethodElection {:?}` (single fieldless token, not the truncating-struct class UX-P4-7 targets); and `OptimizeError::NoLots` → `"no lots available to sell"` (`crates/btctax-cli/src/cmd/optimize.rs:82`) has the same false-"no" shape UX-P4-9 just fixed when no feasible selection covers the target while lots exist (`crates/btctax-core/src/optimize.rs:1187`). Candidate follow-ups.

## Verified clean (stated plainly)

- **§1 dollar-invariant holds.** The core diff touches only the error variant, its raise (`whatif.rs:238-249` — `available` is the identical sum hoisted above the identical comparison), and `of_refusal` (`whatif.rs:543`); all other edits are message-only. Golden packet byte-reproducibility, oracle smoke, and examples goldens all pass unchanged — no output contract moved.
- **Screen-only invariant holds.** `describe_inbound_class`/`describe_outflow_class` are reachable only from the CLI bulk-void summary (`main.rs:2017/2022`), the TUI-edit void summary (`tui-edit/src/main.rs:3743/3753`), and tests; `write_csv_exports`/`write_form_csvs`/btctax-forms still emit via the machine `*_tag` helpers. `no_lots_message` is reachable only from `map_whatif_err` and `whatif_panel::refusal_message`.
- **Fail-closed holds at the enrichment seam.** `store_io_with_path` (`lib.rs:160-173`) enriches only `StoreError::Io`; `WrongPassphrase`/`Locked`/`HalfCreatedVault`/`InvalidVaultPath`/`Crypto`/`Corrupt`/`Sqlite`/`UnsupportedSchema`/`AlreadyExists` pass through unchanged (existing pins: `session.rs:1374`, TUI `unlock.rs` wrong-passphrase/Locked tests, `init.rs:57/102`, tui `export.rs:674` — all still green). A wrong passphrase on an existing vault is never re-labeled (decode errors are not `Io`). The TUI export `do_export` (`btctax-tui/src/export.rs:131-137`) is untouched (auto-derived dir, not a user-named `--out`).
- **UX-P4-9 logic is correct.** The pool filter (`whatif.rs:232-237`) keeps only `remaining_sat > 0` lots in the wallet's as-of pool, so `available == 0` ⟺ genuinely no BTC — a non-empty pool cannot report zero. Units are sats end to end; `fmt_btc` (`render.rs:3661-3665`) is the pre-existing 8dp product formatter. Harvest never raises `WhatIfError::NoLots` (empty pool → `Ok` + `HarvestStatus::NoLots`, `whatif.rs:703-710`, wording honest there); `InvalidTarget → NoLots` unchanged.
- **Tests are mutation-honest on the load-bearing behavior** (I2/M2 aside): exact-substring pins with negative assertions (`no lots available` absent; `only` absent in the zero case; no `{`/`Some(` in formatter output) and field-level `assert_eq`s on the core error. A swapped available/requested, a Debug-dump formatter, or a lost zero-branch all go red. No stale test or doc pins the old wording (grep clean).

## VERDICT

**NOT GREEN — fold required:**
1. **I1**: add a `CliError::PathIo` arm to `map_open_error` (btctax-tui `unlock.rs`) preserving the concise `no vault at <path>` on the unlock screens, with a pin.
2. **I2**: enrich the remaining user-named `--out` sites (`export_irs_pdf` admin.rs:259, `export_full_return` admin.rs:494, `backup_key` admin.rs:411-414) with path + hint and a collision KAT — or amend the spec to scope them out explicitly and file the follow-up with an owning phase.

Minors M1/M2 and nits are non-gating; fix inline or file. Re-review after the fold.
