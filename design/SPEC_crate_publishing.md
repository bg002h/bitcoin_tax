# SPEC — publish the btctax crates to crates.io

**Source baseline:** `main` @ `2bd11ba` (branch `feat/crate-publishing`). **Review status: DRAFT — awaiting R0.**
**Lineage:** user request (2026-07-04): "crate publishing (you might find credentials in shibboleth project
folder)." Follows the README (#34). **The `cargo publish` step is IRREVERSIBLE + PUBLIC — it happens ONLY
after an explicit user go-ahead (§Go-ahead gate); all prep + dry-runs are done first.**

## Preconditions (verified 2026-07-04)
- **Network:** reachable (`cargo search serde` returns results; the earlier `curl` 403 was UA filtering).
- **Token:** `~/.cargo/credentials.toml` has `[registry] token = …` (the user has published before — shibboleth
  crates). No token handling needed beyond what cargo already has; NEVER print the token.
- **Names available:** `cargo search btctax` returns EMPTY — `btctax-core/-store/-adapters/-cli/-tui/-tui-edit`
  are all free. (Names are claimed permanently once published — even a yank doesn't free them.)
- **[★ safety] package contents audited** — `cargo package --list` per crate shows NO `vault.pgp`/`.key`/`.asc`/
  export CSVs/credentials. The only data file is `crates/btctax-adapters/data/btc_usd_daily_close.csv` (public
  BTC/USD market prices — the bundled FMV dataset, intended to ship). `.gitignore` already excludes vault/exports
  and `cargo package` only ships VCS-tracked files, so no personal data leaks.

## What to publish + ORDER (dependency-topological)
Six crates; **`xtask` is `publish = false` (skip)**. Publish order (each depends only on already-published ones):
1. **btctax-core** (no internal deps)
2. **btctax-store** (no internal deps)
3. **btctax-adapters** (→ core)
4. **btctax-cli** (→ core, store, adapters; ships the `btctax` binary)
5. **btctax-tui** (→ cli, store, core, adapters; ships `btctax-tui`)
6. **btctax-tui-edit** (→ tui, cli, core, store, adapters; ships `btctax-tui-edit`)
All at **v0.1.0** (each crate's own `version`, not workspace-inherited).

## Changes required before publishing
1. **Add `description`** (crates.io REQUIRES it) to each of the 6 crates. One line each, e.g.:
   - core: "Offline US Bitcoin tax engine — per-lot basis, gains, and IRS-form projection (part of btctax)."
   - store: "Encrypted, atomic, single-user vault storage for the btctax Bitcoin tax ledger."
   - adapters: "Exchange-CSV parsers and price data for the btctax Bitcoin tax ledger."
   - cli: "btctax — an offline, single-user US Bitcoin tax ledger (CLI)."
   - tui: "Read-only terminal viewer for the btctax Bitcoin tax ledger."
   - tui-edit: "Interactive terminal editor for reconciling the btctax Bitcoin tax ledger."
2. **Add shared publish metadata to `[workspace.package]`** (inherited via `field.workspace = true`):
   `repository = "https://github.com/bg002h/bitcoin_tax"`, `homepage` (same), `keywords`/`categories` where
   sensible (crates.io caps: ≤5 keywords, categories from the fixed slug list — use `finance`,
   `command-line-utilities` for the bins; keywords `bitcoin`, `tax`, `cryptocurrency`, `accounting`, `ledger`).
   Reference them per-crate with `repository.workspace = true` etc. (license/edition/rust-version already shared.)
3. **[★] Path deps need a `version`.** crates.io strips `path` and resolves the published dep by VERSION, so
   convert every internal dep to BOTH: `btctax-core = { path = "../btctax-core", version = "0.1.0" }` (all of:
   adapters→core; cli→core/store/adapters; tui→cli/store/core/adapters; tui-edit→tui/cli/core/store/adapters).
   The `path` is still used for the local workspace build; the `version` is what crates.io records.
4. **README on crates.io:** the workspace `README.md` is at the repo root (outside each crate dir), so a
   per-crate `readme` is DEFERRED (cargo can't package a file outside the crate). Description + the repository
   link suffice for 0.1.0. (A future option: a short per-crate README.)

## Verification (before the go-ahead gate)
- `cargo publish --dry-run --allow-dirty -p <crate>` for EACH crate, in order (`--dry-run` packages + verifies
  the build against the would-be-published deps WITHOUT uploading). For crates 3–6 whose deps aren't on
  crates.io yet, dry-run resolves the internal deps from the local `path` (works offline for verify) — confirm
  each packages + builds. Fix any missing-metadata / dirty-file / dependency error.
- Re-run the `cargo package --list` safety grep after the Cargo.toml edits (no new sensitive file slipped in).
- Full workspace suite still green (the Cargo.toml edits must not change resolution/behavior).

## Go-ahead gate [MANDATORY — irreversible + public]
After all dry-runs pass, STOP and present to the user for explicit confirmation, stating plainly:
- The 6 crate names will be **permanently claimed** (yank ≠ release).
- The **source becomes public** on crates.io (flag: confirm the GitHub repo's intended visibility — publishing
  exposes the source regardless of repo privacy).
- v0.1.0, the bundled public price CSV ships, and this is the point of no return.
Only on an explicit "yes, publish" do the real `cargo publish -p <crate>` (in order) run. Publishing waits for
each crate to appear in the index before the next (recent cargo does this automatically).

## Scope / SemVer
Additive Cargo.toml metadata + the publish action. No source/behavior change. The metadata commit is PATCH-class;
the publish itself is an external action (no repo state change beyond the committed metadata).

## Plan
- **Task 1** — add `description` (per crate) + shared `repository`/`homepage`/`keywords`/`categories`
  (`[workspace.package]` + `.workspace = true`) + version-ify all internal path deps.
- **Task 2** — `cargo publish --dry-run` each crate in order; re-audit `cargo package --list`; full suite green;
  whole-diff review.
- **Task 3 [gated]** — present the go-ahead summary; on explicit approval, `cargo publish` in dependency order.

## Gotchas
- **Irreversible + public** — never publish without the explicit go-ahead; names + source are permanent.
- **Never print the crates.io token.**
- **Path deps MUST carry a version** or crates.io rejects the upload (unresolved dep).
- **Publish order is strict** — a later crate can't publish until its deps are on the index.
- **`--allow-dirty`** only for the dry-run if the tree has untracked files (vault.pgp etc.); the real publish
  should run from a clean, committed tree (the metadata commit merged first).
