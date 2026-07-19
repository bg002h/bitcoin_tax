# Adversarial spec re-review — SPEC_tui_walkthrough.md (r2)

Re-reviewed against current source on `feat/post-v070-product-cycle` (clean tree, post-`0d26691`). Scope: confirm the four r1 Importants are folded correctly, re-derive the folds' new claims from source, and gate the spec's readiness for a PLAN. The build hold for owner approval (§9) is noted and respected — this verdict authorizes planning, not building.

## Fold verification (each r1 Important re-derived from source)

**I-1 (shared fixtures) — FOLDED CORRECTLY.** §4.1 no longer claims shared fixtures exist; it owns the hoist to `btctax-cli::testonly` as the first deliverable (§8 item 1). Feasibility re-verified: all three consumers already depend on `btctax-cli` (`crates/xtask/Cargo.toml:13`, `crates/btctax-tui/Cargo.toml:22`, `crates/btctax-tui-edit/Cargo.toml:19`), so no dependency-graph change is needed; the `btctax-forms` precedent is a **plain `pub mod testonly`** in a published crate (`crates/btctax-forms/src/lib.rs:365`) — not feature-gated, so the TUI crates' `#[cfg(test)]` emit tests can use it with zero Cargo plumbing. `btctax_cli::cmd::import::run` is `pub` (`crates/btctax-cli/src/cmd/import.rs:10`), and writing CRLF consts to a tempdir at test runtime bypasses the `.gitattributes` force-LF hazard. The "xtask's golden must stay byte-identical" claim is achievable: the consts are pure string data; re-importing identical bytes from a new home cannot change `generate()`'s output. (But the *count* is wrong — see M-1.)

**I-2 (cross-crate capture) — FOLDED CORRECTLY, AND THE DECISION IS SOUND.** §4.2 explicitly decides: keep the `pub(crate)` encapsulation (matching the documented intent at `crates/btctax-tui/src/app.rs:136-139`), split capture by owning crate, converge on one shared per-journey fixture, viewer half replays via btctax-cli, and **strikes** the xtask-capture alternative. Every editor mutation has a `pub` btctax-cli twin the viewer's test module can call: import `cmd::import::run` (import.rs:10); J2 `set_donation_details` (`cmd/reconcile.rs:1312`); J3/J7 `classify_inbound` (reconcile.rs:68); J4 `reclassify_income` (reconcile.rs:1285); J5 `set_profile` (`cmd/tax.rs:20`) + `set_forward_method` (reconcile.rs:1117) + `optimize::accept` (`cmd/optimize.rs:143/168`); J6 `import_return_inputs` (tax.rs:49); J8 `self_transfer_match_plan` + `apply_self_transfer_passthrough` (reconcile.rs:888/899); J9 `select_lots` (reconcile.rs:998). The viewer capture surface is complete in-crate: `tabs/tests.rs:126` (`render_viewer`) drives the full `crate::draw::draw`, which renders the export modal AND the what-if overlay (`crates/btctax-tui/src/draw.rs:158-164`) — full chrome/modal/overlay capturable inside `btctax-tui`'s own test module without any visibility change. No remaining infeasibility.

**I-3 (price-cache determinism) — FOLDED CORRECTLY.** §7 pins `BTCTAX_PRICE_CACHE` to a nonexistent file inside each emit/gate test, names the exact leak path (`LayeredPrices::load_with_cache(btctax_cli::price_cache::default_cache_path()...)` in `build_snapshot`; `default_cache_path()` honors the env var first — `crates/btctax-cli/src/price_cache.rs:19-24`), cites the examples-generator precedent, and documents the nextest-process-per-test / plain-`cargo test` caveat. Line-number drift only (N-1).

**I-4 (render pipeline glue) — FOLDED CORRECTLY.** §5 states the pipelines are disjoint (`man-wrap.awk` prose at Makefile:58-64, `tui-wrap.awk` per-`.txt` loop at Makefile:70-83), specifies the committed artifact as an xtask-emitted ordering MANIFEST (byte-gated via `regen == committed` — precedent `examples_golden_matches_committed`, `crates/xtask/src/examples.rs:1369`), puts captions/narration solely in the manifest (r1 M-2 resolved), and specifies `make tui-walkthrough` as NEW manifest-walking glue. `docs/examples-tui-walkthrough/` doesn't collide with the `docs/examples-tui/*.txt` glob. The advisory-job `%PDF` proof matches the existing CI pattern (`.github/workflows/ci.yml:106-133`).

**r1 Minors:** all three landed — J5 firmed to "surfaced" (`KeyCode::Char('z') => open_optimize_accept_flow`, `crates/btctax-tui-edit/src/main.rs:452`, fn at 9248), the journey table claims J2's `d` flow (main.rs:438, fn 4835) and J5's `p`/`e` surfaces, and §11 mandates the single `make regen-walkthrough` target.

## Findings

### CRITICAL / IMPORTANT
None.

### MINOR

**M-1. §4.1/§8's "nine corpus consts (`J1_CSV … J9_CSV`)" undercounts the fixture inventory.** The actual inventory at `crates/xtask/src/examples.rs:201-291` is **eleven** CSV consts — J6 and J8 each split into two per-exchange corpora (`J6_RIVER_CSV`/`J6_COINBASE_CSV`, `J8_RIVER_CSV`/`J8_COINBASE_CSV`) — **plus `J6_FULLRETURN_TOML`**, an `include_str!` of `btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml`. The hoist's architecture is unaffected (and the TOML gets *simpler*: the fixture already lives in btctax-cli, so `testonly` can include it same-crate, retiring the cross-crate `include_str!` that examples.rs flags as the M-5 exception). Fix the enumeration in §4.1 and §8 item 1 so the plan scopes the full inventory.

### NIT
**N-1.** §7's `unlock.rs:171` is now `~:203-204`; the `examples.rs:96` pin is `~:90`. Refresh at plan-write time per the verify-citations rule.
**N-2.** "Captions authored in the manifest" (§8 item 7) is loosely worded: the authoring source of truth is Rust literals in the xtask assembler (like examples.md's prose in examples.rs); the committed `walkthrough.md` is the gated emission. One clarifying clause prevents a planner reading it as "hand-edit the committed .md".

## VERDICT

**GREEN — 0 Critical / 0 Important.** All four r1 Importants are folded with decisions, not hedges, and every new load-bearing claim survives re-derivation from source: the three dependency edges exist, the `testonly` precedent is a plain pub module, every journey's viewer-replay twin is a `pub` btctax-cli fn, the full-frame draw captures modal+overlay in-crate, the price-cache pin matches the generator's, and the manifest/interleave design is honest about new versus reused. The spec is ready to proceed to an implementation PLAN (the build remains held for owner approval per §9). M-1 and the nits are plan-time fixes.
