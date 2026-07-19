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
- **#13 UX-P4-4 (Important) — COMPLETE + reviewed to GREEN (r1 0C/3I → r2 0C/1I → r3 0C/0I), PUSHED**
  (`af7f5cb..24f2d05`). All four sub-parts, each TDD + mutation-proven:
  - **(a)** the sign-policy table on BOTH surfaces. CLI: `eventref::parse_nonneg_usd_arg`/`parse_pos_sell_arg`
    (`4343543`,`674df3a`) + the `242a3d7` fmt-normalization (4a shipped without the fmt CI slice). TUI: new
    `form.rs::parse_nonneg_usd` on all 5 money validators (`13e1704`). Wiring witnessed for all **14** guard
    sites by `tests/value_guard_wiring.rs` (`3a7a3f0` + harvest rows `9647c7e`); `--income`/`--magi` stay
    allow-negative (NOL) with an accept-KAT that pins the effect.
  - **(b)** acquired-after-receipt guard on BOTH surfaces: CLI `classify_inbound` + `tz_label` (`6f2150e`);
    TUI `form.rs::check_acquired_not_after_receipt` (receipt = `InboundListItem.date`) (`13e1704`).
  - **(c)** Form 8283 TIN/EIN/PTIN shapes at the TRUE choke point `donation_details::set` (NOT the
    spec-cited reconcile.rs:1162 — the TUI bypasses it via `persist_donation_details`→`set`) (`fd40dc9`,
    `64b49c6`). bare-9 appraiser-tin accepted; donee-ein bare-9 normalized, SSN-shape refused.
  - **(d)** `--amount` doc + price-based sats-as-dollars stderr warn (`amount_fmv_advisory`, 100× event-date
    close; no-price NOTE; dust skip) (`a9e41c6`).
  - Reviews persisted: `reviews/ux-p4-4-impl-fable-review-r{1,2,3}.md`. Minors/Nits folded or filed
    (`002ee48`,`24f2d05`; FOLLOWUPS "UX-P4-4 impl review r1/r2 residue"). SPEC §3.3(c) amended to the as-built.

- **#18 UX-P4-11 `events list` — COMPLETE + reviewed to GREEN (r1 0C/2I → r2 0C/0I), PUSHED**
  (`8ddeb46..c23c8ee`). New read-only `events list`: the decidable universe (transfer-in/out,
  unclassified, import-conflict, income) with {ref, kind, date, amount, decided-status}, event-sequence
  order. `cmd::inspect::events_list` (persisted-decision reverse-map incl. TransferLink BOTH legs [r1-I1];
  pseudo-decidable by construction) + `render::{EventRow,render_events_list}` (ref-first, `[decidable]` /
  `[decided: decision|N]`) + clap `Command::Events` + man pages. Reviews r{1,2}. Residue filed (M1 owned by
  #14/Step-1c; M2 SPEC §3.6 amended for the universe scope; M3/N*). Mutation-proven KATs incl. the
  link-both-legs + void→re-decide loops.

- **#14 UX-P4-3 — COMPLETE + reviewed to GREEN (r1 0C/2I → r2 0C/1I → r3 0C/0I), PUSHED**
  (`990f786..4bfd382`). Record-time decision validation that MIRRORS the resolver DEFINITIONALLY: new core
  **`btctax_core::would_conflict`** (`project/mod.rs`) runs the REAL projection on `events` + the candidate
  (next decision seq, pseudo forced OFF) and diffs the DecisionConflict set — so every per-verb rule
  (first-wins dup incl. classify-raw; set-fmv duplicate-EXEMPT-but-type-checked; wrong-type/unknown against
  the EFFECTIVE `applied`; void non-revocable/unknown + explicit already-voided) falls out for free. Wired
  via `guard_decision_conflict` at all 6 single-verb appends (fail-closed; bulk `apply_*` OUT). §3.2 "unify
  hints" done at the source: one surface-neutral `CONFLICT_HINT` const in `resolve.rs` naming `events list`.
  16 KATs in `tests/record_time_validation.rs` (both directions mutation-proven, incl. accept-governed
  SupersedeImport [R3-I1] + classify-raw-refuse arms; the two `applied` writers separately pinned). r1/r2/r3
  verified `would_conflict` definitionally correct. Reviews r{1,2,3}. Residue N1 (docs/#21), N3 (later cycle).

- **#15 UX-P4-7/8/9 (Phase 2 Legibility) — COMPLETE + reviewed to GREEN (r1 0C/2I → r2 0C/1I → r3
  0C/0I), PUSHED** (`34e9945..288d669`). Three sub-steps, each TDD + mutation-proven:
  - **2c UX-P4-9** (`34e9945`): `WhatIfError::NoLots{wallet,at,available,requested}` + one shared
    `render::no_lots_message` (CLI `map_whatif_err` + TUI `whatif_panel::refusal_message`) naming the
    available balance/wallet/date/requested, empty-pool "no BTC" vs insufficiency. Reuses 8dp `fmt_btc`.
  - **2b UX-P4-8** (`66b4bad`): `CliError::PathIo{path,hint,source}` + `store_io_with_path`/
    `cli_io_with_path` at vault-open (`Session::open`) + export `--out` (`export_snapshot`). Folded r1-I2
    (missed siblings → `admin::mkdir_out` choke point for `export-irs-pdf`/`export-full-return` +
    `backup_key` wrap) and r2-I1 (`cli_io_with_path` now enriches `Store(Io)` subpath collisions too).
  - **2a UX-P4-7** (`fa4badc`): shared `render::describe_inbound_class`/`describe_outflow_class` replace
    `{:?}` in CLI `bulk_void_payload_summary` + TUI `summarize_void_payload`. Screen-only [R0-I4].
  - Folded r1-I1 (my `Session::open` PathIo change had orphaned the TUI unlock screen's "no vault at"
    message — restored `map_open_error` arm). Reviews r{1,2,3}. Residue = one "pathless user-path I/O"
    class (r1-N3 optimize NoLots, r2-N3 init --key-backup, r3-M1 PDF subpath, r3-N1/N2) filed for a later
    legibility-polish cycle. Full CI surface green (2039 nextest+clippy, fmt, pii, isolation, msrv@1.88).

- **#16 UX-P4-6/10 (Phase 3 Report surfaces) — COMPLETE + reviewed to GREEN (r1 0C/1I → r2 0C/0I),
  PUSHED** (`20b2a58..<r2-cert>`). Two sub-steps, each TDD + mutation-proven:
  - **3a UX-P4-6** (`20b2a58`): BTC-unit `Pending: <btc> BTC (N unreconciled transfer(s) — see verify)`
    line in `render_report` holdings when `state.stats.sigma_pending > 0`; hidden when reconciled.
  - **3b UX-P4-10** (`027a89d`): `report --tax-year` exits 1 on `TaxOutcome::NotComputable`, 0 otherwise
    (main.rs Report arm, keyed on the delta `outcome`, after printing). clap doc → --help + man page
    (0/1/2, key on NON-ZERO); stale `tax_report.rs` exit-0 doc-comments corrected.
  - Folded r1-I1 (the plan-mandated dual-report absolute-refused-but-delta-computed exit-0 KAT via
    `screen_absolute` case (c)) + M1 (hard-blocker exit-1 KAT). `tests/report_exit_code.rs` = 5 KATs;
    r2 independently re-ran the exact regression mutation (predicate ∨ "NOT COMPUTABLE" → reds only the
    dual KAT). Reviews r{1,2}. Residue: N1 (no vault-level pending test) + N2 (assert refusal reason) —
    optional Nits filed. Full CI surface green (2046 nextest+clippy, fmt, pii, isolation, msrv@1.88).

- **#17 (Phase 4 Affordances) — COMPLETE + reviewed to GREEN (r1 0C/4I → r2 0C/2I → r3 0C/0I), PUSHED**
  (`b101fef..8204375`). UX-P4-5 (`--forms`-ignored stderr warn) + UX-P4-12(b–h): (b) units/kind on
  classify-inbound-income/set-fmv, (c) config forward-method read-back via the SHARED
  `project::in_force_methods` resolver [big r2/r3 rework — HIFO default, engine key], (d) tax-profile
  set-error names --show, (e) config human labels + 2 missed Hifo sites, (f) surface-neutral void remedy,
  (g) valid reclassify-outflow --as-kind donate hint, (h) dropped "q: swallowed" footers. Each TDD +
  mutation-proven. Reviews r{1,2,3}. Residue = pathless-io + legibility Nits filed.
  **(i) DONE (user-decided 2026-07-19) + reviewed to GREEN (r1 0C/1I → r2 GREEN), UNPUSHED**
  (`bd73968`, r1 fold `c2597ad`). Investigation found the premise incomplete: the TUI ALREADY stores a
  table-less-year return as a resolve-invisible DRAFT (all-year authoring works), so the reviewed I-11
  finalize-guard was CORRECT to keep. USER DECISION: keep I-11, fix only the blunt refusal message.
  `NoTables` arm now persists the draft + shows a reassuring, un-truncated status ("… inputs SAVED as a
  draft; finalize when tables publish."). r1 folded I-1 (reconciled the memory record — no live mandate
  to reverse I-11 survives), M-1 (message shortened, render-visible, mutation-proven), N-1 (dirty-clear
  tested). See [[full-return-store-before-tables-policy]] + `reviews/ux-p4-12i-impl-fable-review-r{1,2}.md`.

## NEXT STEP — #19, then #20–23 (per the PLAN's phase order)
- **#19 (Phase 5 Display: M-1)** — enable `serde_json` `preserve_order` for `income show`; workspace-global
  flip (spec §4 M-1 — audit verified safe: fingerprints hand-rolled bytes; typed serde field-ordered;
  `Value` sites = income-show display + input-form coverage tooling + update-prices parse; btctax-forms
  serde_json-free). Pin the blast-radius enumeration in a KAT; regen J6 golden; verify net-isolation + msrv.
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
> `STANDARD_WORKFLOW.md`, then continue at "NEXT STEP". UX-P4-4, #18 `events list`, and #14 UX-P4-3 are all
> COMPLETE + reviewed to GREEN + pushed. Next is #15 (P4-7/8/9 legible-error cluster), then #16–23 per the
> PLAN phase order. Each item: TDD + mutation-proven → independent Fable review to 0C/0I → push. Fable for
> reviews only.

(The memory note [[post-v070-product-cycle]] auto-loads and points here.)
