# R0 review — `design/SPEC_readme.md` (round 1)

**Artifact:** `design/SPEC_readme.md` (DRAFT — end-user README: install + init→import→verify→reconcile→report→export tutorial).
**Baseline:** branch `feat/readme` @ `c95373f`; source verified against the working tree at review time (`main` == `f7408f3`).
**Reviewer role:** independent architect, read-only. **Bar:** 0 Critical / 0 Important.

## Verdict

**0 Critical / 5 Important / 1 Minor / 1 Nit** — **NOT GREEN.**

No cited command or flag is nonexistent, and nothing hard-breaks the vault, so there is no Critical. But the tutorial as specced has five accuracy/completeness gaps that would each produce a wrong or unrunnable README step for a first-time user — the exact failure mode the spec's own §1 goal ("REAL, verified commands") exists to prevent. All five are concrete and mechanically fixable.

What is CORRECT (verified, no finding):

- Global `--vault` is `#[arg(long, global = true, default_value = "vault.pgp")]` (`cli.rs:20`); the spec's pre-subcommand placement `btctax --vault … init` works (and, being global, also works after the subcommand).
- `init --key-backup <PATH>` is a required arg (`cli.rs:38-39`); `import [FILES]...` positional variadic (`cli.rs:45`); `verify` no flags (`cli.rs:47`); `report` has BOTH `--year` and `--tax-year` (`cli.rs:57-58`); `export-snapshot --out <OUT> --tax-year <Y>` — the flag IS `--tax-year`, not `--year` (`cli.rs:99,103`); `reconcile classify-inbound-self-transfer <IN_REF>` positional (`cli.rs:267-272`). All match.
- `BTCTAX_PASSPHRASE` env var name exact, with an interactive `rpassword` prompt fallback (confirm on `init`) (`main.rs:47-59`).
- `verify` exits 1 **iff** open Hard blockers (`main.rs:85-90`; `has_hard_blockers`, `render.rs:413`). Spec claim accurate.
- Install facts: MSRV `rust-version = "1.88"` and license `MIT OR Unlicense` (workspace `Cargo.toml`); `rusqlite { features=["bundled"] }` ⇒ C-toolchain note correct (`btctax-cli/Cargo.toml`); the three bins are `btctax` (`crates/btctax-cli`), `btctax-tui`, `btctax-tui-edit` — `cargo install --path crates/btctax-cli` installs `btctax` (`[[bin]] name="btctax"`), and xtask is correctly excluded.
- `docs/man/btctax.1` + `btctax-init.1` exist; `Makefile` has `docs` and `bundles` targets; `STANDARD_WORKFLOW.md` exists — all referenced links resolve.
- TY2025 tax table IS bundled (`tax_tables.rs:65`), so `report --tax-year 2025` will not hit `TaxTableMissing` (but see I2 for `TaxProfileMissing`).

---

### [I1] IMPORTANT — tutorial step 1 writes the key backup ONTO the auto-created sidecar key (`--key-backup ./vault.key`)

**Evidence.** `init` creates TWO key artifacts, then a third:
1. `Session::create` → `Vault::create` writes the sidecar key at `suffixed_key(vault)`, i.e. `vault.pgp` → **`vault.key`** (`paths.rs:21-30`), as the **binary**-serialized TSK via `atomic_write(&kp, &tsk)` (`vault.rs:88-93`). The init test asserts `vault.key` is written (`cmd/init.rs:45`).
2. `init` then calls `backup_key(key_backup_path)` (`cmd/init.rs:26-29`), which writes the **ASCII-armored** TSK via `write_owner_only`, and `write_owner_only` truncates/clobbers (`vault.rs:176-192`; `fsperms.rs:44-48`).

Spec step 1 (`design/SPEC_readme.md:36-37`) is `btctax --vault ./vault.pgp init --key-backup ./vault.key`. Because `suffixed_key(./vault.pgp) == ./vault.key`, the forced backup is written to **the exact path of the live sidecar key**, overwriting the just-created binary sidecar with the armored copy.

This does not brick the vault (`open` reads the sidecar with `Cert::from_bytes`, which auto-detects armor — `vault.rs:132`), so it is not Critical. But it is wrong, self-contradicting guidance: `init --key-backup` help says the backup is *"the only way to recover the vault if you lose `vault.key`… Store it offline"* (`cli.rs:30-37`). A backup living AT `vault.key` provides zero recovery value, and the tutorial's very first command teaches users to defeat the backup and clobber their working sidecar.

**Fix.** Use a distinct, clearly-offline path for `--key-backup`, e.g. `btctax --vault ./vault.pgp init --key-backup ./vault-key-backup.asc`, and have the README say one line: this file is the offline recovery copy — move it off the machine; it is separate from the auto-created `vault.key` sidecar. (Note `.gitignore` `vault*` already ignores `vault.key`; pick a backup name you are content to keep out of the repo too — `.asc` is ignored.)

---

### [I2] IMPORTANT — the tutorial's `report --tax-year 2025` will NOT print a TaxResult; it prints "not computable: no tax_profile set" (missing prerequisite step)

**Evidence.** `report_tax_year` reads `s.tax_profile(year)?` (may be `None`) and passes `profile.as_ref()` to `compute_tax_year` (`cmd/tax.rs:64-68`). With `profile = None`, `compute_tax_year` returns `TaxOutcome::NotComputable(Blocker { kind: TaxProfileMissing, detail: "no tax_profile set for {year}" })` (`tax/compute.rs:265-272`). The `report --tax-year` arm never sets a non-zero exit code (`main.rs:111-133`; falls through to `Ok(ExitCode::SUCCESS)` at `:408`), so the command "succeeds" (exit 0) while rendering a not-computable message — NOT the "per-year TaxResult + Schedule D" the spec promises (`design/SPEC_readme.md:44`).

Setting a profile is a separate command with FOUR mandatory flags: `tax-profile --year Y --filing-status … --ordinary-taxable-income … --magi-excluding-crypto … --qualified-dividends …` (enforced in `main.rs:317-343`). The tutorial has no `tax-profile` step at all.

**Fix.** Insert a `tax-profile` step before step 5, e.g.:
`btctax --vault ./vault.pgp tax-profile --year 2025 --filing-status single --ordinary-taxable-income 80000 --magi-excluding-crypto 80000 --qualified-dividends 0`
Then `report --tax-year 2025` yields a real TaxResult. (Alternatively, lead the walkthrough with plain `report` for holdings/realized and present `report --tax-year` as "after you set a profile" — but the spec currently leads with the tax-year form as the payoff, so the profile step is the cleaner fix.)

---

### [I3] IMPORTANT — false privacy claim: `.gitignore` does NOT guard the export-snapshot CSVs (only the `.sqlite`)

**Evidence.** Spec §7 and step 6 assert *".gitignore already guards them [exports]"* (`design/SPEC_readme.md:47-48, 51-52`). `export-snapshot --out ./export` writes, into the out dir: `snapshot.sqlite` (`vault.rs:172`) plus `lots.csv`, `disposals.csv`, `removals.csv`, `income.csv` always, and `form8949.csv`, `schedule_d.csv`, `form8283.csv`, `schedule_se.csv` with `--tax-year` (`render.rs write_csv_exports`: `lots.csv` @ +10, `disposals.csv` @ +35, `removals.csv` @ +76, `income.csv` @ +127, form8949/schedule_d/form8283/schedule_se @ +151-161).

Cross-checking `.gitignore`: `snapshot.sqlite` is covered (by `snapshot.*` and `*.sqlite`). **None of the eight CSV names match any `.gitignore` rule** (patterns are `*.pgp/*.gpg/*.asc/vault*/*.vault/*-snapshot.*/snapshot.*/*.sqlite/*.sqlite3/*.db/…`; there is no rule for `lots.csv`, `disposals.csv`, `income.csv`, `form8949.csv`, etc.), and a dir literally named `./export` is not ignored. A user who runs `export-snapshot --out ./export` inside the repo and `git add .` would stage their **real tax data** (basis, disposals, income, Form 8949 / Schedule D).

**Fix.** Drop the "gitignore already guards exports" claim (it is only true for the SQLite). Instruct users to export **outside** the repo (matching `.gitignore`'s own header: *"Real exchange exports live OUTSIDE this repo"*), e.g. `--out ~/Documents/BitcoinTax/export-2025`, and keep the strong "contains your data" warning without leaning on gitignore. (Extending `.gitignore` to cover the CSVs is a code change, out of this docs-only scope; if desired, file a FOLLOWUP.)

---

### [I4] IMPORTANT — synthetic import CSV is under-specified; the README could easily produce a shape the Coinbase adapter rejects, and must contain a `Receive` row to feed the reconcile step

**Evidence.** Spec step 2 says only *"a tiny synthetic Coinbase-shape example inline"* (`design/SPEC_readme.md:38-39`) — no header, no transaction-type vocabulary. The adapter is strict:

- **Detection** requires the file text to contain `Transaction Type` AND `Quantity Transacted` AND `Subtotal` (`coinbase.rs:82-84`); otherwise the file isn't recognized as Coinbase.
- **Header row** is found by AND-matching the tokens `ID`, `Transaction Type`, `Quantity Transacted` (`coinbase.rs:41`; `read.rs:77-84`). Real column names read by `normalize`: `ID`, `Timestamp`, `Transaction Type`, `Asset`, `Quantity Transacted`, `Subtotal`, `Fees and/or Spread`, `Sender Address`, `Recipient Address` (`coinbase.rs:22-35, 118-145`).
- `Asset` must be `BTC` or the row is dropped (FR2) (`coinbase.rs:120-124`).
- Transaction-type values that produce real events: `Buy`, `Sell`, `Send`/`Withdrawal`, `Receive`; `Order` and the Pro-move types become `Unclassified` (`coinbase.rs:148-217`). `Timestamp` accepts RFC3339 (`2023-01-05T14:00:00Z`), `YYYY-MM-DD HH:MM:SS`, `MM/DD/YYYY HH:MM:SS`, or bare date (`parse.rs:113-142`).

Critically, the tutorial's reconcile step (step 4) needs a **pending inbound** to reconcile. Only a **`Receive`** row yields a `TransferIn` (`coinbase.rs:176-183`) → `Op::UnknownInbound` → a Hard `UnknownBasisInbound` blocker (`project/fold.rs:815-821`) that `classify-inbound-self-transfer` then clears. If the spec's example omits a `Receive` row, there is nothing to reconcile and the tutorial's step 4 has no target. (A `Buy` row is also wanted so `report` shows a holding.)

**Fix.** Pin the exact example in the spec so the README author cannot drift. Minimum viable, importable CSV (header line is auto-detected; a preamble is optional):
```
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Subtotal,Fees and/or Spread,Sender Address,Recipient Address
BUY-1,2023-01-05T14:00:00Z,Buy,BTC,0.05,1000.00,5.00,,
RCV-1,2024-06-01T10:00:00Z,Receive,BTC,0.01,,,bc1qexamplesender,
```
State that `Asset` must be `BTC`, that the `Receive` row is what produces the pending inbound reconciled in step 4, and cite `crates/btctax-adapters/src/sources/coinbase.rs` as the source of truth. The KAT already commits to running the whole tutorial end-to-end, which will catch any residual drift.

---

### [I5] IMPORTANT — no guidance on where `<IN_REF>` comes from, and the ref contains `|` (must be shell-quoted)

**Evidence.** Spec step 4 shows `reconcile classify-inbound-self-transfer <IN_REF>` (`design/SPEC_readme.md:41-43`) but never says how a user obtains `<IN_REF>`. The ref is the inbound event's canonical id. For a Coinbase `Receive` with `ID = RCV-1`: `mint.native(Direction::In, "RCV-1")` → `SourceRef("in|RCV-1")` (`normalize.rs:51-53`), and `EventId::canonical` renders `import|coinbase|<source_ref>` (`identity.rs:86-90`) ⇒ the ref is **`import|coinbase|in|RCV-1`** — three `|` characters.

Where it surfaces: `verify` prints each Hard blocker as `[{kind}] {event_canonical} :: {detail}` (`render.rs:1806-1813`), so the reconcile target appears as
`[UnknownBasisInbound] import|coinbase|in|RCV-1 :: unclassified TransferIn — basis unknown`.
It is NOT in any export CSV — `export-snapshot` writes only lots/disposals/removals/income (the `event` columns there are for disposals/removals/income, not pending inbounds; `cli.rs:88-93`), and `report` shows holdings/realized, not pending-inbound refs. So `verify`'s Hard-blocker line is the acquisition path.

Two problems the README must handle: (a) tell the user to copy the ref from the `verify` Hard-blocker line (or, since the synthetic `ID` is fixed, the README can hard-code `import|coinbase|in|RCV-1`); and (b) the ref MUST be **single-quoted** in the shell — unquoted `import|coinbase|in|RCV-1` is parsed as three piped commands. Every tutorial command that pastes an event ref has this hazard.

**Fix.** In step 4, show sample `verify` output containing the `[UnknownBasisInbound] import|coinbase|in|RCV-1 …` line, then the reconcile command with the ref **quoted**:
`btctax --vault ./vault.pgp reconcile classify-inbound-self-transfer 'import|coinbase|in|RCV-1'`
Add a one-line callout that event refs contain `|` and must be quoted. (Optionally mention `--basis` to suppress the `SelfTransferInboundZeroBasis` $0-basis advisory; `cli.rs:264-273`.)

---

### [M1] MINOR — a fresh import legitimately makes `verify` exit 1; the README should frame that as expected, not an error

**Evidence.** After importing the CSV in I4, the `Receive` row yields a Hard `UnknownBasisInbound` blocker (`fold.rs:815-821`), so `verify` prints it and exits 1 (`main.rs:85-90`). A first-time user reading step 3 ("exit 1 iff Hard blockers — the gate") may read a non-zero exit as a failure of their setup.

**Fix.** One sentence: seeing a Hard blocker (and exit 1) right after import is normal — it is exactly what step 4 reconciles; re-run `verify` after reconciling and it should exit 0.

---

### [N1] NIT — "the 25 reconcile subcommands" is off by one (there are 24)

**Evidence.** `enum Reconcile` has 24 variants (`cli.rs:234-576`; counted: LinkTransfer … MatchSelfTransfers = 24). Spec §Structure step 4 says "the 25 reconcile subcommands" (`design/SPEC_readme.md:43`).

**Fix.** Say "two dozen" / "the full set of reconcile subcommands" rather than a brittle exact count, or use 24.

---

## Summary for the author

Fix all five Important items before proceeding to Plan:
- **I1** — `--key-backup` must not point at `./vault.key` (the live sidecar); use a distinct offline path.
- **I2** — add the mandatory `tax-profile --year 2025 …` step before `report --tax-year`, or `report --tax-year` prints "not computable".
- **I3** — the export CSVs are NOT gitignored; drop that claim and export outside the repo.
- **I4** — pin the exact Coinbase CSV (header + `BTC` asset + a `Receive` row); cite the adapter.
- **I5** — show where `<IN_REF>` comes from (the `verify` Hard-blocker line, `import|coinbase|in|…`) and quote it in the shell.

Then re-review at R0 round 2.
