# CONTINUITY — post-v0.7.0 product cycle (resume doc)

**Purpose:** everything needed to resume the build after a context clear. Read this + `STANDARD_WORKFLOW.md`
first, then continue at "NEXT STEP" below.

**Branch:** `feat/post-v070-product-cycle` (pushed to origin; 22 commits ahead of `main`). `main` stays at
green **v0.7.0** — do NOT merge until the whole cycle is done + whole-branch reviewed (ship is the user's call).

## What this cycle is
Implementing the ~17 open usage-examples follow-ups (the bug-hunt payoff): UX-P4-1, UX-P4-3..12,
UX-P1-3/7/8/10, UX-P2-1, UX-P3-2, N-R1, M-1. Full list + status in the **system task list #10–23**
(TaskList). The design is settled + reviewed to green — do NOT re-litigate it.

## The contract (both Fable-GREEN — treat as fixed)
- `design/usage-examples/SPEC_post_v070_product_cycle.md` — the design of record. Reviewed 4 rounds / 2 Fable
  lenses → 0C/0I. Each item's exact behavior + KATs + §9 anchors are here. **§3.3 = UX-P4-4** (what's next).
- `design/usage-examples/IMPLEMENTATION_PLAN_post_v070_product_cycle.md` — phased TDD build order (green).
- `design/usage-examples/reviews/` — all reviews persisted verbatim (spec r1-r4, plan r1-r2, ux-p4-1-impl r1-r2).

## Standing rules (do not violate)
- **Workflow:** every artifact/step → independent review to 0C/0I; persist review verbatim before folding;
  re-review after every fold. **Fable is for REVIEWS ONLY now** (the 4-hr liberal grant elapsed) — do
  implementation + recon on general agents or yourself; use `model:"fable"` only for the review subagents.
- **Mutation-proof every fix** ([[untested-guard-pattern]]): the KAT must RED when the fix is reverted (I
  cp-backup the source, sed the guard to `if false`, run the KAT, confirm FAIL, restore). A fix isn't done
  until the mutation dies.
- **Green = the FULL CI surface**, not just `make check`: `make check` (nextest+clippy) PLUS
  `cargo fmt --all -- --check`, `cargo check --workspace --locked` @1.88 (msrv), `bash scripts/pii-scan-generic.sh`,
  `cargo run -p xtask -- check-isolation`. (These bit the release once — see [[fast-validation-gate]].)
- **§1 invariant:** no change may alter a computed tax figure for a correctly-specified return.
- Commit per sub-step; **push per item** once its review is green. Trailers required (Co-Authored-By +
  Claude-Session — copy from any recent commit).

## DONE (committed + pushed)
- **Ledger reconciled** (UX-P3-1 DISCHARGED, `8e14066`).
- **SPEC + PLAN** — green.
- **#12 UX-P4-1 (Important) — COMPLETE + reviewed 0C/0I.** Pseudo disclosure across all 4 number-bearing
  surfaces + the write-carryover gate: (1) CLI delta `render_tax_outcome` banner+suffix `dbea745`; (2) dual-
  report L24/Absolute suffix + (3) TUI Tax tab `render_tax_content` `3285a55`; (4) `write_back_carryover`
  refuse on pseudo OR NotComputable `3cd735f`. Impl-review fixups `46c9eae`. `render::PseudoDisclosure`
  {None,Synthetic,Placeholder} enum; predicate `pseudo_active() OR PseudoPlaceholder` from LIVE pseudo-ON state.
- **#13 UX-P4-4a (partial) — the full sign-policy table DONE + mutation-proven** (`4343543`, `674df3a`).
  `eventref::parse_nonneg_usd_arg` (refuse <0) + `parse_pos_sell_arg` (refuse ≤0), applied to every
  refuse-<0 flag (basis/fmv/fmv-at-gift/donor-basis/amount/fee/price/proceeds/carryforward-in + --sell).
  `--income`/`--magi`/tax-profile fields keep allow-negative (NOL legitimacy). Closes the `--basis=-N`
  clap `=`-bypass. Unit tests in `eventref.rs` + an integration KAT in `classify_inbound_self_transfer_cli.rs`.

## NEXT STEP — finish #13 (UX-P4-4), then it gets its Fable review
Per SPEC §3.3, three sub-parts remain (all record-time, at the same reconcile/donation surfaces):
- **(b) acquired-after-receipt guard [SPEC §3.3(b)]:** refuse `--acquired` / `--donor-acquired` strictly
  AFTER the receive/receipt date (impossible for a self-transfer-in / gift). The two dates come from
  different sources → the refusal message must print the receipt date + its tz basis; **same-day allowed**.
  Sites: `main.rs` self-transfer dispatch (`--acquired`, near the `--basis` block ~line 999) +
  classify-inbound-gift (`--donor-acquired` ~line 985). The receive/receipt date is on the TransferIn event
  in the projected state (resolve it to compare). KAT: `--acquired=<receipt+1d>` refused; same-day OK.
- **(c) EIN/TIN shapes [SPEC §3.3(c)]:** validate at the `set_donation_details` CHOKE POINT
  (`crates/btctax-cli/src/cmd/reconcile.rs:~1162`) so the TUI-edit form (`tui-edit/src/edit/form.rs:1328-1420`)
  is covered too, not just CLI arg parsing. `--appraiser-tin` accepts EIN-shape (`\d{2}-\d{7}`) OR SSN-shape
  (`\d{3}-\d{2}-\d{4}`) — 26 CFR 301.6109-1(a)(1)(i); `--donee-ein` EIN-shape + NORMALIZE hyphenless 9-digit +
  refuse SSN-shape (§170(c)); optional so refuse msg says "omit if the donee has none"; `--appraiser-ptin`
  own shape `P\d{8}`. NOTE the hyphenless EIN/SSN ambiguity is inherent — do NOT "harden" it. KAT: EIN-shaped
  appraiser-tin ACCEPTED; `--donee-ein banana` refused; hyphenless donee EIN accepted.
- **(d) `--amount` doc + FMV warn [SPEC §3.3(d)]:** add a `--amount` clap doc-comment (unit = USD FMV; then
  `make docs`). WARN (stderr, non-fatal) when `FMV > 100 × (outflow_sats/1e8) × close-at-the-outflow-date` —
  price-based (26 CFR §1.170A-1(c)(2), event-date close), NOT cost-basis. No-price fallback: SKIP the warn
  (state it). sats on the TransferOut event; prices via `session.prices()`. KAT: sats-as-USD `--amount` warns;
  a legit high-appreciation FMV does NOT; no-price path silent.
- Then: **independent Fable review of the whole UX-P4-4** (all of a/b/c/d) → 0C/0I → mark #13 done → push.

## Then #14–23 (per the PLAN's phase order)
- **#18 `events list` (UX-P4-11)** must come BEFORE **#14 UX-P4-3** (P4-3's refuse-hint names the verb).
- **#14 UX-P4-3** needs a **pseudo-OFF shadow-projection helper that MIRRORS the resolver's own `applied`
  map** (reuse resolve.rs's pass-1c/1d/1e construction; `applied` has 2 real writers under pseudo-OFF:
  SupersedeImport `resolve.rs:513` + ClassifyRaw pass-1c `:543-560` — enumerate-the-writers is fragile, mirror
  the resolver). `set-fmv` is exempt from the DUPLICATE refusal ONLY (ManualFmv last-wins) but STILL gets
  existence/type validation. First-wins verbs incl. `classify-raw`. See SPEC §3.2 + `ux-p4-1`... (SPEC §3.2/§9.3).
- #15 (P4-7/8/9 legible errors), #16 (P4-6/10 report surfaces + exit code), #17 (P4-5 --forms warn),
  #19 (P4-12 papercuts), #20 (M-1 serde preserve_order), #21 (docs journeys + P2-1), #22 (P3-2/N-R1 polish),
  #23 (phase-8 whole-branch close: full-CI-surface green, regen ALL goldens, FOLLOWUPS burndown, whole-branch
  Fable review, then it's mergeable). Each item's spec is in SPEC §3–§6.

## Test-harness quick ref
- CLI lib fns callable directly in tests (`cmd::tax::report_tax_year`, `render::*`); or spawn the binary via
  `env!("CARGO_BIN_EXE_btctax")` + `BTCTAX_PASSPHRASE`. `make_vault`/`make_vault_with(csv)`; `pseudo_set_mode`;
  `write_buy_receive_2024` = a ready unknown-basis-Receive fixture (pseudo trigger ON / Hard-blocker OFF);
  `return_inputs::set` for TY2024/2025 rows; `build_snapshot(&session)` for TUI integration.

## Outstanding (user action)
⚠️ **Revoke the temporary crates.io token** in `~/.cargo/credentials.toml` (from the v0.7.0 release).

## HOW TO RESUME (command to issue after /clear)
> Resume the post-v0.7.0 product cycle: read `design/usage-examples/CONTINUITY_post_v070.md` and
> `STANDARD_WORKFLOW.md`, then continue at "NEXT STEP" — finish UX-P4-4 (b) acquired>receipt, (c) EIN/TIN
> shapes, (d) --amount doc + FMV warn, TDD + mutation-proven, then Fable-review UX-P4-4 to green. Fable for
> reviews only.

(The memory note [[post-v070-product-cycle]] auto-loads and points here.)
