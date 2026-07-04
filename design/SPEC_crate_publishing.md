# SPEC — publish the btctax crates to crates.io

**Source baseline:** `main` @ `2bd11ba` (branch `feat/crate-publishing`). **Review status: R0 round 1 folded
(0C / 1I / 2M / 2N — all folded; the safety check [no data leaks] was independently CLEARED). Review:
`reviews/R0-spec-crate-publishing-round-1.md`. Awaiting R0 round 2. Key folds: coordinated
`cargo publish --dry-run --workspace` (per-crate dry-run would fail — path stripped, deps not yet on registry)
[I1]; `categories` per-crate not workspace-shared [M1]; v0.1.0-permanently-burned in the go-ahead gate [M2];
bare-`btctax`-name + `cargo publish --workspace` notes [N1/N2].**
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
   `repository = "https://github.com/bg002h/bitcoin_tax"`, `homepage` (same), and
   `keywords = ["bitcoin","tax","cryptocurrency","accounting","ledger"]` (≤5). Reference per-crate with
   `repository.workspace = true` / `keywords.workspace = true` (license/edition/rust-version already shared).
   **[R0-M1] `categories` are NOT workspace-shared** (inheriting would wrongly tag the 3 LIBRARY crates as
   `command-line-utilities`) — set literally PER CRATE: libs (core/store/adapters) `categories = ["finance"]`;
   bins (cli/tui/tui-edit) `categories = ["command-line-utilities","finance"]`. (Slugs verified against the
   crates.io category list at implementation.)
3. **[★] Path deps need a `version`.** crates.io strips `path` and resolves the published dep by VERSION, so
   convert every internal dep to BOTH: `btctax-core = { path = "../btctax-core", version = "0.1.0" }` (all of:
   adapters→core; cli→core/store/adapters; tui→cli/store/core/adapters; tui-edit→tui/cli/core/store/adapters).
   The `path` is still used for the local workspace build; the `version` is what crates.io records.
4. **README on crates.io:** the workspace `README.md` is at the repo root (outside each crate dir), so a
   per-crate `readme` is DEFERRED (cargo can't package a file outside the crate). Description + the repository
   link suffice for 0.1.0. (A future option: a short per-crate README.)

## Verification (before the go-ahead gate)
- **[R0-I1] `cargo publish --dry-run --workspace --allow-dirty`** — the COORDINATED workspace dry-run
  (cargo ≥1.90; this env is 1.97). Per-crate `--dry-run -p <downstream>` would FAIL: the verify build extracts
  the packaged crate with `path` STRIPPED and resolves siblings from the REGISTRY (not yet published) →
  `no matching package named 'btctax-core' found`. `--workspace` packages ALL members then verifies each
  against the just-packaged siblings OFFLINE, in topological order, WITHOUT uploading — the correct last gate.
  (Fallback if needed: per-crate `--dry-run --no-verify`, which packages but skips the build-verify.) Fix any
  missing-metadata / dirty-file / dependency error before the go-ahead.
- Re-run the `cargo package --list` safety grep after the Cargo.toml edits (no new sensitive file slipped in).
- Full workspace suite still green (the Cargo.toml edits must not change resolution/behavior).

## Go-ahead gate [MANDATORY — irreversible + public]
After the workspace dry-run passes, STOP and present to the user for explicit confirmation, stating plainly:
- The 6 crate names will be **permanently claimed** (yank ≠ release; a name is never freed).
- **[R0-M2] v0.1.0 is permanently burned** — even after a yank, that exact version can NEVER be re-published;
  any fix must ship as 0.1.1. So the tree + metadata must be right before publishing.
- The **source becomes public** on crates.io — confirm the intended GitHub-repo visibility (publishing exposes
  the source regardless of whether the repo is private).
- The bundled public price CSV ships; this is the point of no return.
- **[R0-N1]** the CLI installs the `btctax` BINARY from the `btctax-cli` CRATE; the bare `btctax` crate name is
  NOT among the six — ask whether the user also wants to reserve/publish `btctax` (optional).
Only on an explicit "yes, publish" does the real publish run: **[R0-N2] `cargo publish --workspace`** (from a
clean committed tree, NO `--allow-dirty`) — packages, verifies, uploads in dependency order, and waits for each
crate to hit the index before the next. (Fallback: `cargo publish -p <crate>` per crate in topological order.)

## Scope / SemVer
Additive Cargo.toml metadata + the publish action. No source/behavior change. The metadata commit is PATCH-class;
the publish itself is an external action (no repo state change beyond the committed metadata).

## Plan
- **Task 1** — add `description` (per crate) + shared `repository`/`homepage`/`keywords`/`categories`
  (`[workspace.package]` + `.workspace = true`) + version-ify all internal path deps.
- **Task 2** — `cargo publish --dry-run --workspace --allow-dirty` (coordinated verify — R0-I1); re-audit
  `cargo package --list`; full suite green; whole-diff review.
- **Task 3 [gated]** — present the go-ahead summary; on explicit approval, `cargo publish --workspace` from a
  clean committed tree (R0-N2), dependency-ordered with index waits.

## Gotchas
- **Irreversible + public** — never publish without the explicit go-ahead; names + source are permanent.
- **Never print the crates.io token.**
- **Path deps MUST carry a version** or crates.io rejects the upload (unresolved dep).
- **Publish order is strict** — a later crate can't publish until its deps are on the index.
- **`--allow-dirty`** only for the dry-run if the tree has untracked files (vault.pgp etc.); the real publish
  should run from a clean, committed tree (the metadata commit merged first).
