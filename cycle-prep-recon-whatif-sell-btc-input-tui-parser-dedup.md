# cycle-prep recon — 2026-07-06 — whatif-sell-btc-input, whatif-tui-parser-dedup

**Origin/main SHA at recon time:** `2e89911` (this repo's default branch is `main`, not `master`)
**Local branch:** `main`
**Sync state:** up-to-date (0 ahead / 0 behind origin/main)
**Untracked:** none

Slug(s) verified: `whatif-sell-btc-input`, `whatif-tui-parser-dedup`. Both are freshly-filed (this session, at the
same SHA), so **zero drift expected** — confirmed.

---

## Per-slug verification

### whatif-sell-btc-input
- **WHAT (from FOLLOWUPS.md):** `what-if sell --sell` takes an integer SAT amount; users think in BTC (0.05 BTC
  = 5,000,000 sat — unintuitive). FIX: accept a BTC decimal (parse→sat, reject ambiguous precision) or add a
  `--btc` alias.
- **Citations:**
  - `crates/btctax-cli/src/cli.rs:334` `Sell {` (the `WhatIf::Sell` clap subcommand; `--sell <SELL>` field) —
    **ACCURATE** (subcommand present at that line).
  - `crates/btctax-cli/src/cmd/whatif.rs:68` `sell_sat: i64` — **ACCURATE**: the handler's sale amount IS a raw
    i64 sat, and the clap arg deserializes straight to it (empirically: `--sell 0.05` → "expected an integer sat
    amount"). So the sat-only claim is confirmed at the source.
  - Note: the **TUI panel already accepts a BTC decimal** (P3, whatif_panel.rs BTC→sat parse) — so this fix
    aligns the CLI `--sell` with the TUI, resolving an inconsistency, not inventing a convention.
- **Action for brainstorm spec:** accept a decimal-BTC on `--sell` (if the arg contains `.`, parse BTC→sat with
  8-dp max, reject over-precision; bare integer stays sat = NON-breaking), OR add an explicit `--btc <DECIMAL>`
  (additive). Prefer the former (one flag, no ambiguity, matches the TUI). Cite source SHA `2e89911`.

### whatif-tui-parser-dedup
- **WHAT (from FOLLOWUPS.md):** the TUI what-if panel re-implements the harvest-target parser
  (`zero-ltcg|fifteen-ltcg|gain=$X|tax=$X`) + the refusal formatter locally, to avoid importing `cmd::` (which
  KAT-E10's source-gate forbids in btctax-tui). FIX: move the target parser to `btctax-core::whatif` (a `FromStr`
  on `HarvestTarget`) so both `cmd::` and the panel share one source.
- **Citations:**
  - `crates/btctax-core/src/whatif.rs:371` `pub enum HarvestTarget` — **ACCURATE**; and it has **NO `FromStr`
    impl** (grep found only the enum def) — so "add `FromStr` on `HarvestTarget`" is valid + non-conflicting.
  - `crates/btctax-cli/src/cmd/whatif.rs:110-111` `pub fn parse_harvest_target(s: &str) -> Result<HarvestTarget,
    CliError>` — **ACCURATE**: the CLI's parser (the canonical one to fold into the shared `FromStr`).
  - `crates/btctax-tui/src/whatif_panel.rs:15` imports `HarvestTarget` + carries a `target_buf` and the
    `zero-ltcg | fifteen-ltcg | gain=$X | tax=$X` handling (lines 39/60/99/344) — **ACCURATE**: the panel owns
    a second, local parse (to keep KAT-E10 green by not touching `cmd::`).
- **Action for brainstorm spec:** add `impl FromStr for HarvestTarget` in `btctax-core::whatif` (returning a
  core error type, NOT `CliError`); rewrite `cmd::whatif::parse_harvest_target` + the panel to call it. The
  cmd/panel keep only their error-mapping. Removes the duplication AND keeps KAT-E10 satisfied (the panel
  depends on `btctax_core`, not `cmd::`). Cite source SHA `2e89911`.

---

## Cross-cutting observations
1. **No drift, no structural errors** — both slugs were filed this session at `2e89911` and verified against the
   same bytes; every citation is ACCURATE.
2. **Related surface:** both touch the what-if CLI/panel area (`cmd/whatif.rs`, `whatif_panel.rs`, core
   `whatif.rs`). They compose cleanly and share no conflicting edit region (slug-1 = the `sell` arg; slug-2 =
   the `harvest` target parser).
3. **Lockstep (repo-specific):** this repo has NO GUI `schema_mirror` and NO `docs/manual/src/40-cli-reference/`
   (those are generic to the skill). The real CLI-surface mirror here is the **clap doc-comments → `docs/man/*.1`
   (regenerate via `cargo run -p xtask -- docs`) + the README** — update `btctax-what-if-sell.1` if slug-1
   changes the `--sell` help; slug-2 is internal (no man/README change).
4. **Both are cosmetic/UX + refactor** — neither touches tax math; the verified `whatif::{sell,harvest}` core is
   unchanged, so no Fable-level review is warranted (a standard Opus R0 suffices).

---

## Recommended brainstorm-session scope
**One combined cleanup cycle** (both slugs; ~60–90 LOC total, both in the whatif area):
- **SemVer:** slug-1 (decimal `--sell`, backward-compatible parse) = **PATCH**; slug-2 (internal `FromStr`
  refactor) = **PATCH**. Combined → a **0.4.1 PATCH** (no new public surface; `--btc` alias would make it MINOR
  — prefer the non-flag decimal parse to stay PATCH).
- **Ordering:** independent; do slug-2 first (it's a pure refactor that also removes the KAT-E10 workaround),
  then slug-1 (the `--sell` decimal parse; reuse the TUI's existing BTC→sat helper — lift it to a shared spot).
- **Lockstep:** regenerate `docs/man/btctax-what-if-sell.1` + README on slug-1; KAT that `--sell 0.05` == `--sell
  5000000` and over-precision (`0.000000001`) is rejected; KAT that the shared `HarvestTarget::FromStr` matches
  both prior parsers. Standard Opus R0 gate before implementation (no code until 0C/0I).
