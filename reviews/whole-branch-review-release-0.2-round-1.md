# Whole-diff review (Phase E) — feat/release-0.2 — round 1

**Verdict: 0 Critical / 0 Important — SHIP (then publish).** A mechanical version bump; rigor placed on the
diff-is-purely-version check + the `--workspace` publish dry-run.

## Diff
`main (9b10136)..HEAD` — 9 files, +30/−30: every crate's `version = "0.1.0" → "0.2.0"` (package versions +
internal `btctax-* { version }` pins) + Cargo.lock. Verified: the ONLY changes in the `crates/*/Cargo.toml`
files are `0.1.0→0.2.0` version lines (no dependency, feature, or metadata drift). All 8 workspace crates bumped
uniformly (incl. the `btctax` reserved stub + `xtask`), so no split-version skew.

## Publish readiness (dry-run)
`cargo publish --workspace --dry-run` packaged + VERIFIED every publishable crate cleanly, in dependency order:
`btctax-core → btctax-store → btctax-adapters → btctax-cli → btctax-tui → btctax-tui-edit` (+ the `btctax`
stub); `xtask` correctly excluded (`publish = false`). Each "Uploading …" aborted on the dry-run as expected;
no packaging errors. Because these are NEW VERSIONS of existing crates (not new crates), the new-crate 5-burst
rate limit does not apply.

## SemVer
0.2.0 (MINOR) is correct: the three shipped feature sets since 0.1.0 (per-exchange method election, pseudo-
reconcile mode, attestation export gate) + the store hardening are additive/behavior-compatible. No public-API
removals.

## Functional inertness
Version-string-only change; `cargo build --workspace` clean; the code is byte-identical to the 1146-green
`main`, so the suite result is unchanged (the bump touches no `.rs`).

**SHIP → publish from the merged, clean `main` with the token via `CARGO_REGISTRY_TOKEN` (never persisted).**
