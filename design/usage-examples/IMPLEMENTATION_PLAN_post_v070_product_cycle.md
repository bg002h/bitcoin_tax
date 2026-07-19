# IMPLEMENTATION PLAN — post-v0.7.0 product cycle

**Status:** **GREEN** (r3). r1 (0C/3I) → r2 (0C/1I: one stale cross-reference) → r3 fixes that one sentence.
The r2 re-review verified I-1/I-2/I-3 + both Minors genuinely folded against source; r2's sole residual
(the "Sequencing" note still said the helper precedes 1b) is corrected here, aligning all three sites — a
pure consistency fix (grep-verified across preamble/step-1b/sequencing), no new content, so green without a
further full review round (workflow §8 ceremony-scales-down). Reviews: `reviews/plan-post-v070-fable-review-r1.md`,
`…-r2.md`. Against the GREEN SPEC.
**Spec of record:** `SPEC_post_v070_product_cycle.md` (GREEN). **Branch:** `feat/post-v070-product-cycle`.

This plan translates the spec's §7 phasing into concrete TDD steps. It does not re-argue design — the spec
(§3–§6, reviewed across r1–r4) is the contract; this is the build order, the file/function hooks (from spec
§9), and the KAT-per-step map.

## 0. Cross-cutting discipline (applies to every step)

- **TDD, mutation-proven.** For each fix: write the KAT(s) the spec names, watch them RED without the fix,
  implement, watch GREEN, then flip the guard (revert the fix in a scratch copy or comment the emit) and
  confirm the KAT RE-REDS. A fix with a non-red guard is not done (`[[untested-guard-pattern]]`).
- **§1 dollar-invariant.** Any step touching a render/validation path near the tax math must include the
  spec's §1 guard: an existing golden/KAT's dollar figures are byte-identical across the fix (only
  message/banner/suffix lines move).
- **Green per step = the FULL CI surface**, not just `make check`: `make check` (nextest + clippy) **plus**
  `cargo fmt --all -- --check`, `cargo check --workspace --locked` @ 1.88 (msrv), `bash
  scripts/pii-scan-generic.sh`, `cargo run -p xtask -- check-isolation`. (Ref: the make-check≠CI gap that
  bit the release.)
- **Goldens.** Any captured-output change regenerates: `cargo run --locked -p xtask -- examples >
  docs/examples/examples.md`; TUI goldens are in-process staleness-gated (the `*_goldens_match_committed`
  tests); man pages via `make docs` when clap doc-comments change.
- **Per-step gate.** Independent Fable review to 0C/0I on the step's diff, persisted verbatim under
  `reviews/`, folded, re-reviewed. Commit per step; **push per phase** once its review is green.
- **Per-phase burndown.** On entering a phase, reconcile open follow-ups it owns; do not carry a
  phase-owned item past its gate.

## Phase 1 — Correctness cluster (gates everything else)

**Shared prerequisite — a pseudo-OFF shadow projection helper (for UX-P4-3 ONLY) `[PLAN-I1]`.** UX-P4-3's
*validator* needs "what the resolver sees with `pseudo_reconcile` forced OFF." **UX-P4-1's predicate does
NOT** — it reads the LIVE pseudo-ON state/provenance (`pseudo_active() OR PseudoPlaceholder`); a pseudo-OFF
view has `count == 0` and can never yield `PseudoPlaceholder` (`cli/resolve.rs:121-128` is pseudo-on-gated),
so wiring 1b to the helper would make the banner structurally silent and **reinstate the [T-C1]/[T-C2]
Criticals**. Therefore: **1b reads the live projected state; the helper is a 1c-only dependency.** Build ONE
helper that reuses the resolver's own conflict/classify-pass construction (NOT a hand-rebuilt subset — spec
§3.2 `[R3-I1]`) with `pseudo_reconcile` forced off, exposing the effective `applied` map + projected state.
**NOTE `[PLAN-min]`:** `applied` is currently local to core `resolve()` — this is a **scoped new core API**
(expose the map, or a `validate_target(ref) -> Effect` shim), not merely a CLI change.
**Equivalence KAT — must be pseudo-DIVERGENT, not vacuous `[PLAN-I2]`:** on a fixture whose ON vs OFF
`applied` genuinely DIFFER (a pseudo default that accept-first `resolve.rs:521-522` / Phase-A `:949` adds
under ON), assert the helper's OFF `applied` OMITS those pseudo-gated writes and equals the resolver's own
pseudo-OFF `applied`. A mutation that wrongly reads the stored pseudo cfg (via `session.project()`, the path
§3.2 forbids) must RED it — else the KAT witnesses nothing.

**Step 1a — UX-P4-11 `events list` (FIRST — UX-P4-3's refuse hint names it).**
- New read-only subcommand `events list` (clap def in `cli.rs`; dispatch in `main.rs`; render in
  `render.rs`). Rows: every decidable event {ref, kind, date, amount, decided-status}; decided rows carry
  their `decision|N` ref; pseudo-defaulted events list as **decidable** (spec §3.6). Stable order by event
  seq.
- Man page + `make docs` (single-sourced from the clap doc-comment).
- KAT: a listed ref pasted verbatim into `reclassify-*` is ACCEPTED (closes the UX-P4-11 trap); a
  pseudo-defaulted event lists as decidable. Golden: `events list` is not yet in a journey — add a small
  demonstration OR keep it out of the golden this step (decide in review).
- Commit.

**Step 1b — UX-P4-1 pseudo disclosure (four surfaces + write-carryover gate).**
- Compute `pseudo_contributed = state.pseudo_active() OR provenance == PseudoPlaceholder` at the report
  build (`tax.rs:429`, provenance from `tax.rs:282-296`). Name the field `pseudo_contributed` (spec `[T2-N3]`).
- Surface 1: thread into `render_tax_outcome` (`render.rs:1018`) — banner (channel-aware text, spec §3.1) +
  ` [PSEUDO]` suffix on the total (`:1056-1061`).
- Surface 2: thread into `render_dual_report` (`render.rs:1173`) — suffix L24 (`:1229`) + Absolute TOTAL
  TAX (`:1247`).
- Surface 3: thread `snap.state.pseudo_active()` into `render_tax_content` (`tabs/tax.rs:55-121`); banner +
  `[PSEUDO]` on total. (TUI narrower signal justified by the enumeration invariant — add the one-sentence
  code comment citing `session.rs:497-498` + the trip-wire.)
- Surface 4: `write_back_carryover` (`tax.rs:444-517`) refuses (nonzero, persist nothing, before
  `apply_carryover_writeback` `:507`) when `pseudo_contributed` (4a) OR the delta outcome is `NotComputable`
  (4b). Message names the blocker.
- KATs (spec §3.1): (a) two-clause golden guard; (b) placeholder-channel banner wording; (c) reconciled/off
  → none; (d) write-carryover pseudo → nonzero + year+1 byte-identical; (e) write-carryover non-pseudo
  NotComputable → nonzero + year+1 byte-identical; (f) TUI pseudo/no-profile → NOT COMPUTABLE. Mutation each.
- New test fixture: a pseudo-active vault (synthetic-lot-consuming sale) + a placeholder-channel vault.
- Commit.

**Step 1c — UX-P4-3 record-time validation (mirror the resolver, pseudo-safe).**
- Add record-time validation to the single-verb append fns (`reconcile.rs:41/62/85/110/301/1136`, incl.
  `classify_raw`): refuse iff the shadow-projection resolver would raise a NEW `DecisionConflict` vs live
  real decisions. Existence/type against the shadow `applied` (the 1a helper). `set-fmv` exempt from the
  DUPLICATE refusal only (still existence/type validated). `void` refuses non-revocable/already-voided.
  Bulk `apply_*` paths OUT (plan-generated refs — spec §3.2).
- Unify the `DecisionConflict` remedy hints → one phrasing naming `events list` + "void decision|N first".
- KATs (spec §3.2 both directions), incl. the accept-governed `SupersedeImport` accept + `classify-raw`
  refuse cases, and the ClassifyRaw'd-target accept. Mutation.
- Commit.

**Step 1d — UX-P4-4 + UX-P1-3 value validation.**
- Per-flag sign guards at the sites in the spec §3.3(a) table (guard per-flag, never in `parse_usd_arg`/
  `parse_sell_arg`); `--sell=-1` refused; tax-profile already-guarded fields left as-is.
- **Ad-hoc trio decision (SPEC §3.3(a) delegated "Decide in the PLAN") `[PLAN-I3]`:** `--carryforward-in`
  refuses < 0 (it is a carryforward loss *magnitude*); `--income` and `--magi` **ALLOW** negative — a
  negative AGI/MAGI is legitimate (NOL years), so a blanket refuse would be a §1 false-refuse. All three are
  planning-only (no filed-form contact). KAT: `--carryforward-in=-1` refused; `--income=-5000` / `--magi=-5000`
  accepted (and flow into the marginal computation unchanged).
- `--acquired`/`--donor-acquired` > receipt refused with a receipt-date+tz message (spec §3.3b).
- EIN/TIN shape at the `set_donation_details` choke point (`reconcile.rs:1162`) — appraiser-tin EIN|SSN,
  donee-ein EIN + hyphenless-normalize + refuse-SSN, ptin `P\d{8}` (spec §3.3c). Covers the TUI-edit form.
- `--amount` doc comment + price-based FMV warn (event-date close; skip on no-price) (spec §3.3d).
- KATs (spec §3.3). Regen the J2 donation golden if any donation message/echo changes. Mutation.
- Commit. **Push phase 1** after its review is green.

## Phase 2 — Legibility (UX-P4-7/8/9)

- **2a UX-P4-7:** one screen-only human formatter for decision payloads (cite the `[R0-I4]` screen-only
  rule); used by the CLI bulk-void preview + TUI (`tui-edit/src/main.rs:3742`). KAT: formatter output; TUI
  no mid-field truncation.
- **2b UX-P4-8:** add `{path}` context at vault-open (`session.rs:390-394`) + `--out`
  (`admin.rs:82`, `render.rs:586-618`), mirroring `AdapterError::Io{path,source}`. KAT: missing vault names
  the path + hint; `--out` collision names the path.
- **2c UX-P4-9:** `WhatIfError::NoLots { available, requested }` (`whatif.rs:137`), populate at `:234-236`,
  render at `cmd/whatif.rs:170-172`; harvest arms mechanical. KAT: insufficient vs zero messages.
- Review, commit, push.

## Phase 3 — Report surfaces (UX-P4-6/10)

- **3a UX-P4-6:** pending line in the holdings view (BTC unit) from `stats.sigma_pending`. KAT.
- **3b UX-P4-10:** exit 1 on `NotComputable` (`main.rs:140-182`); man-page contract; stale doc-comment
  `tax_report.rs:780`; the write-carryover interaction already handled by 1b clause 4b. KAT asserts the
  process exit code, **including the two deliberate exit-0 non-triggers (SPEC §3.5):** a dual-report whose
  absolute total is refused but whose delta computed → exit 0; a pseudo-active report WITHOUT
  `--write-carryover` → exit 0 (the banner is the signal).
- Review, commit, push.

## Phase 4 — Affordances (UX-P4-5, UX-P4-12 b–i)

- **4a UX-P4-5:** warn `--forms` ignored on a full-return year (packet unchanged). KAT.
- **4b UX-P4-12(b–i):** the message/affordance papercuts; (i) default-year gate placement (align to CLI
  store-then-gate — must not change which year's packet exports). KAT per output-changing sub-item.
- Review, commit, push.

## Phase 5 — Display (M-1)

- Enable `serde_json` `preserve_order`; run the blast-radius enumeration KAT (spec §4 M-1); regen J6 golden;
  verify net-isolation + msrv still green. Review, commit, push.

## Phase 6 — Docs (UX-P1-7/8/10, UX-P2-1)

- Three new journeys in `xtask/src/examples.rs` (manual income FMV; match-self-transfers; select-lots);
  regen `examples.md` + PDF. Harden the `is_demonstrated` matcher (UX-P2-1). Review, commit, push.

## Phase 7 — Polish (UX-P3-2, N-R1)

- Colorized TUI PDF (roff color from the style runs); de-stick the `no_direct_now_utc` scan in **both**
  copies (tui `export.rs:970` + tui-edit `main.rs:13975` `[PLAN-min]`) (KAT: a production `now_utc()` after
  a test module is caught, in each). Review, commit, push.

## Phase 8 — Close

- Whole-branch independent Fable review to 0C/0I.
- Full CI-surface validation locally; regen ALL goldens (examples.md, TUI, man).
- FOLLOWUPS.md burndown: close every landed item with its commit; confirm residue is parked on a later
  owner.
- Push; verify CI green (all 9 jobs).
- Morning report: what landed, what remains, any user decisions.

## Sequencing / dependency notes

- 1a (`events list`) precedes 1c (its refuse hint names the verb) — spec `[G-I8]`.
- The shared shadow-projection helper (phase-1 prerequisite) precedes **1c's validator ONLY — NOT 1b**:
  UX-P4-1's predicate reads the live pseudo-ON projected state, so a pseudo-OFF view would silence the
  banner and reinstate the [T-C1]/[T-C2] Criticals `[PLAN-I1]`.
- Phases 2–7 are independent of each other; order is by value. If time-boxed, land phase 1 fully (the
  Important/correctness items), then descend.
- Each phase pushes only after its own Fable review is green; `main` is untouched until phase 8 (merge is
  the user's call — do NOT auto-merge to main; this branch ships on request).
