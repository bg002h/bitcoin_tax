# R0 — SPEC_crate_publishing.md — round 1

**Artifact:** `design/SPEC_crate_publishing.md`
**Baseline:** branch `feat/crate-publishing` @ `d9b60fe` (spec baseline states `main @ 2bd11ba`; `main` currently == `2bd11ba` ✓).
**Reviewer role:** independent architect (R0). Read-only; no Cargo.toml edits, no `cargo publish`, no branch switch.
**Toolchain in env:** `cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)`, `rustc 1.97.0-nightly (52b6e2c20 2026-04-27)`. Workspace MSRV = `1.88`.
**Bar:** 0 Critical / 0 Important.

## Verdict: 0 Critical / 1 Important / 2 Minor / 2 Nit

The #1 risk — a package leaking personal/tax data — is **CLEARED** (independently verified below). The publish order and the full set of path→version dep edges are correct and complete. One Important methodology error remains in the dry-run verification plan (Task 2/§Verification); it is factually wrong for per-crate downstream dry-runs and must be corrected before the plan relies on it. Not GREEN.

---

## Verified CORRECT (evidence)

**[★ safety] No sensitive data ships — CONFIRMED.** I ran `cargo package -p <c> --list --allow-dirty --no-verify` for all 6 crates. Only source (`src/**`, `tests/**`), `Cargo.toml`/`.orig`, `Cargo.lock`, `.cargo_vcs_info.json` ship. The sole data file is `crates/btctax-adapters/data/btc_usd_daily_close.csv`, present only in the adapters package. Its content is generic public BTC/USD closes:
```
date,usd_close
2024-01-15,42500.00
...
2025-06-15,67500.00     (7 lines total, 6 data rows)
```
That is public market/FMV data, **not** the user's holdings. No `vault.pgp`/`.key`/`.asc`/`.env`/credentials/export CSV/xlsx appears in ANY package.
- The real secrets (`vault.key`, `vault.pgp`, `vault.pgp.bak`) live at the **repo root**, outside every crate dir, and are gitignored (`.gitignore`: `*.pgp`, `*.asc`, `vault*`, `*.xlsx`, `*.sqlite`, `/data/`, `/samples/`). `git status --porcelain --ignored crates/` shows **no** ignored/untracked files inside any crate dir; `find crates -iname '*.pgp'|'*.key'|'*.asc'|'*.env'|'*.sqlite*'|'*.xlsx'` = empty; `git ls-files | grep -iE '\.(xlsx|pgp|key|asc)$'` = empty.
- Packaging is genuinely VCS-scoped: every package embeds `.cargo_vcs_info.json` (git-based), and there is **no** `include=`/`exclude=`/non-git source in any Cargo.toml (`grep` shows only `publish = false` on xtask). So gitignored root secrets cannot leak, and `cargo package` ships only tracked files. The spec's §Preconditions safety claim holds. **No Critical.**

**Publish ORDER is a valid topological sort — CONFIRMED.** Internal `btctax-*` deps per crate (from each `[dependencies]`): core = none; store = none; adapters → core; cli → core, store, adapters; tui → cli, store, core, adapters; tui-edit → tui, cli, core, store, adapters. Order core→store→adapters→cli→tui→tui-edit has every dep published before its dependent. `btctax-store` indeed has **zero** internal deps. `xtask → btctax-cli` exists but xtask is `publish = false`, so it is correctly excluded and never blocks a dry-run.

**Path→version edge set is COMPLETE — CONFIRMED (Task 4).** Every internal edge lives in `[dependencies]` (not dev/build). No crate has a `[build-dependencies]` table; **no** `[dev-dependencies]` references an internal `btctax-*` crate (core-dev = proptest/rust_decimal_macros; store-dev = tempfile; adapters-dev = rust_xlsxwriter/tempfile; cli-dev = tempfile/rust_decimal_macros/time; tui-dev = tempfile; tui-edit-dev = tempfile/rust_decimal_macros/serde_json). So the spec's enumeration (adapters→core; cli→core/store/adapters; tui→cli/store/core/adapters; tui-edit→tui/cli/core/store/adapters) misses nothing, and there are no internal dev-deps to version-ify. The `{ path, version = "0.1.0" }` form is correct (path for local build, `^0.1.0` req recorded for the registry); crates.io does hard-reject a path dep with no version ("all dependencies must have a version specified when publishing").

**Metadata requirements — CONFIRMED.** `license.workspace = true` is present in all 6 crates and `[workspace.package].license = "MIT OR Unlicense"` is a valid SPDX expression (both `MIT` and `Unlicense` are valid SPDX ids). `description` is genuinely the only missing *required* field — every `cargo package --list` prints `warning: manifest has no description, documentation, homepage or repository`; of those, only `description` is a hard crates.io requirement (documentation/homepage optional). All of `description`, `repository`, `homepage`, `keywords`, `categories` are workspace-inheritable `[workspace.package]` fields (stabilized in Rust 1.64; fine on 1.88/1.97), so `field.workspace = true` works. Keywords `bitcoin, tax, cryptocurrency, accounting, ledger` = exactly 5 (≤5 cap), each ≤20 chars/alphanumeric — OK. `finance` and `command-line-utilities` are real crates.io slugs (see M1 for the caveat).

**Repository URL correct.** Spec's `https://github.com/bg002h/bitcoin_tax` matches `git remote -v` (`git@github.com:bg002h/bitcoin_tax.git`).

**Irreversibility handling — mostly present.** Go-ahead gate is MANDATORY, states permanent name claim (yank ≠ release), source-becomes-public, flags GitHub repo visibility, and defers the real publish to explicit "yes." `--allow-dirty` is correctly confined to the dry-run; the real publish runs from the clean committed tree (root vault files are gitignored and outside crate dirs, so they do not trip cargo's per-crate dirty check). See M2 for the one gap.

---

## Findings

### [I1] Important — per-crate `cargo publish --dry-run` of downstream crates (3–6) will FAIL the verify build; the spec's "resolves from local path" claim is wrong

`§Verification` (spec lines 50–54) says:
> "For crates 3–6 whose deps aren't on crates.io yet, dry-run resolves the internal deps from the local `path` (works offline for verify) — confirm each packages + builds."

This is inaccurate for **per-crate** dry-runs, which is exactly what `Task 2` / `§Verification` prescribe (`cargo publish --dry-run --allow-dirty -p <crate>` … "each crate, in order"). A dry-run packages the crate **then verifies by building the extracted `.crate`** in a temp dir whose `Cargo.toml` has `path` **stripped** — the dep is now only `btctax-core = "0.1.0"`, resolved from the **registry**, not the workspace path. With core/store/adapters not yet on crates.io, the verify build of adapters/cli/tui/tui-edit fails with `no matching package named 'btctax-core' found; location searched: registry crates-io`. (The path is not consulted during verification precisely because it has been removed to simulate the published artifact. The whole reason Rust 1.90 added coordinated workspace publishing was that this per-crate case did not work.)

Why it matters: the dry-run gate is the **last safety check before an irreversible action**. A methodology that silently doesn't build-verify the downstream crates (or that the team "fixes" by dropping `--no-verify` reasoning or skipping the step) undermines the pre-publish safety story.

I could not empirically confirm on this env (constraints forbid `cargo publish`, and the current Cargo.tomls still lack the `version` on path deps so any dry-run aborts earlier on "does not specify a version"), so the finding rests on cargo's documented packaging/verify mechanism.

**Fix (works on this 1.97 toolchain):** replace the per-crate dry-run with the **coordinated** form —
- `cargo publish --dry-run --workspace --allow-dirty` (or `cargo package --workspace --allow-dirty`). Rust 1.90+ packages **all** members and verifies each against a temporary local registry of the just-packaged siblings, so inter-member deps resolve **without** anything being on crates.io — genuinely offline, genuinely build-verified.
- If a per-crate dry-run is still wanted for crates 3–6, it must use `--no-verify` (packages + checks metadata only; **no** build verification), and the spec should state that this trades away the build check until the real in-order publish. Either way, correct the "resolves from local `path`" sentence — it does not.

Re the real publish: the in-order `cargo publish -p <crate>` plan is fine *because* each upstream is actually on the index before its dependent's verify build (spec already notes cargo waits for the index). No `--no-verify` is needed on the real publish. Only the **dry-run** claim is broken. (See N2 for an optional `--workspace` real-publish improvement.)

### [M1] Minor — `categories` via `[workspace.package]` inheritance applies `command-line-utilities` to the three LIBRARY crates; internally inconsistent with "for the bins"

`§Changes 2` (spec line 40) says to put shared metadata in `[workspace.package]` and "use `finance`, `command-line-utilities` **for the bins**," then "Reference them per-crate with `repository.workspace = true` etc." A single `[workspace.package].categories` inherited via `categories.workspace = true` is applied **identically to all 6 crates** — you cannot both inherit one shared value *and* vary it "for the bins." The result would tag the libraries `btctax-core`/`-store`/`-adapters` as `command-line-utilities`, which is wrong for libraries. Not a publish blocker (crates.io accepts it), but it's a metadata-quality error and the spec's wording is self-contradictory about the mechanism.

**Fix:** either (a) put only `finance` (valid for all 6) in `[workspace.package].categories` + `categories.workspace = true`, and add a **literal** per-crate `categories = ["command-line-utilities", "finance"]` in the 3 bin crates (cli/tui/tui-edit) — i.e., do **not** use `.workspace = true` for categories there; or (b) drop `categories` from the workspace table and set it per-crate. Also confirm slugs against the live list at implement time: `finance` and `command-line-utilities` are valid today, but crates.io flags unknown category slugs on upload — verify before the irreversible run.

### [M2] Minor — go-ahead gate omits that version `0.1.0` is permanently burned (not just the names)

`§Go-ahead gate` (lines 58–65) surfaces permanent **name** claim, public source, and the CSV — but not that a specific **version** is unrepublishable. Once `0.1.0` is published, it can never be re-uploaded, even after a `yank`; any correction (bad metadata, an accidentally-included file, a wrong description) requires bumping to `0.1.1`. Given the whole point of this gate is informed consent before the point of no return, the user should be told a mistake in `0.1.0` is fixable only by yank + version bump, never by overwrite.

**Fix:** add one bullet to the go-ahead summary: "v0.1.0 is permanent per crate — a yank hides it but never frees the version; any fix ships as 0.1.1."

### [N1] Nit — install ergonomics: `cargo install btctax-cli` produces the `btctax` binary, not `cargo install btctax`

The publishable crate is `btctax-cli` (bin `btctax`), so users install with `cargo install btctax-cli`. The bare name `btctax` is free (spec verified `cargo search btctax` empty). Optional: reserve/publish a thin `btctax` crate (or rename) so `cargo install btctax` works as users will guess. Product call, not a blocker; worth a sentence in the spec.

### [N2] Nit — consider `cargo publish --workspace` for the REAL publish too

Same 1.90+ capability behind the I1 fix: `cargo publish --workspace` publishes all members in dependency order and waits for the index between them automatically, removing the manual "6 separate `-p` invocations in exact order" step (a step where a human mis-order is possible). Optional hardening of `Task 3`; the current in-order plan is correct, just more manual.

---

## Bottom line
0 Critical (safety/no-leak fully verified), publish order + version-ify edge set correct and complete, crates.io required-field analysis correct. **1 Important:** fix the dry-run verification methodology (I1) — the per-crate downstream dry-run does not build-verify offline as claimed; use the coordinated `--workspace` dry-run available on this toolchain. Address M1/M2 while there. Re-review after the fold. **Not R0-GREEN.**
