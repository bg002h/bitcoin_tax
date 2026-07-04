# Whole-diff review (Phase E) — feat/crate-publishing — round 1 (prep only; publish is gated)

**Verdict: 0 Critical / 0 Important / 0 Minor / 0 Nit — READY (metadata prep). The actual `cargo publish` is
GATED on explicit user go-ahead and is NOT part of this review's "ship".**

Independent Phase-E review of the PREP diff `main (2bd11ba)..HEAD` — Cargo.toml metadata + version-ified path
deps + the spec/reviews. Contract: `design/SPEC_crate_publishing.md` (R0-GREEN, 2 rounds). No source/behavior
change.

## What changed
- **`[workspace.package]`** gained `repository` + `homepage` (`github.com/bg002h/bitcoin_tax`) +
  `keywords = [bitcoin, tax, cryptocurrency, accounting, ledger]` (exactly 5).
- **Per crate:** `description` (the crates.io-required field, was the only one missing), the three
  `.workspace = true` refs, and `categories` set LITERALLY per crate — libs (core/store/adapters) `["finance"]`,
  bins (cli/tui/tui-edit) `["command-line-utilities","finance"]` (R0-M1; both are real crates.io slugs).
- **14 internal path deps version-ified** to `{ path = "..", version = "0.1.0" }` (adapters 1, cli 3, tui 4,
  tui-edit 5, xtask 1) — crates.io strips `path` and resolves by version; `path` still drives the local build.

## Verification
- **[★] Coordinated workspace dry-run PASSED** — `cargo publish --dry-run --workspace --allow-dirty` exited 0:
  all 6 **Packaged** and **build-Verified** in topological order (core → adapters → store → cli → tui →
  tui-edit) against the just-packaged siblings, offline, no upload. This is the correct last gate (a per-crate
  dry-run would have failed — R0-I1).
- **[★ safety, re-audited] no personal/tax data ships** — `cargo package --list` per crate: no
  `*.pgp`/`*.key`/`*.asc`/export CSV/`*.sqlite`/credential in any package. The single "vault"-matching entry is
  `btctax-store/src/vault.rs` (source CODE). The only DATA file is `btctax-adapters/data/btc_usd_daily_close.csv`
  (public BTC/USD closes — the bundled FMV dataset, intended to ship). `cargo package` is VCS-scoped and no
  `include=`/`exclude=` exists, so gitignored `vault.pgp` cannot leak.
- **Manifests parse** (`cargo metadata`); `description` present on all 6; `license = MIT OR Unlicense` (valid
  SPDX) workspace-shared; publish order is a valid toposort; `xtask` stays `publish = false`.
- No behavior change (metadata + version-on-path-deps are inert to the local build/tests, which pass at this SHA).

## The gate (NOT executed here)
The irreversible + public `cargo publish --workspace` runs ONLY after the user's explicit go-ahead, from a
clean committed tree (no `--allow-dirty`). The user must be told: names permanent, v0.1.0 permanently burned,
source becomes public (regardless of repo privacy), MIT-OR-Unlicense makes it freely reusable, and the
new-crate 5-burst rate limit means the 6th (`btctax-tui-edit`) will 429 and needs a ~10-min retry (safe,
resumable). Open question for the user: also reserve the bare `btctax` crate name?

**READY — merge the metadata; then present the go-ahead gate.**
