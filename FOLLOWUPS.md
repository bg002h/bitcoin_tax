# FOLLOWUPS — bitcoin_tax (TaxApp)

Open/!resolved action items (STANDARD_WORKFLOW §4). Each: what · why · status · pointer.

---

## ★ STALE-SWEEP AUDIT (2026-07-20, verified against HEAD `de5e4a1`)

A full audit checked every "OPEN" marker in this file against **current source** (not the
follow-up text). The large majority were stale bookkeeping — implemented, superseded, or moot,
never swept back after the feature landed. This block is the authoritative reconciliation: where a
cluster below is listed **RECONCILED-DONE**, the scattered `OPEN` tags in its original section are
**superseded by this block** (kept for record, not live work). The **GENUINELY-OPEN INDEX** is the
live residue — grep it first.

### ✅ SHIPPED-BUG FIXED (2026-07-20) — TY2025+ Form 8949 digital-asset boxes (GREEN, merge-pending)

Found during the conservative-filing SPEC tax review: the shipped product (v0.7.0) emitted the
**securities** boxes C (ST) / F (LT) for **every** year, including TY2025+, where the 1099-DA revision
requires the **digital-asset** boxes I (ST) / L (LT) and forbids C/F. Fixed year-aware on branch
`fix/8949-digital-asset-boxes` (off main): `Form8949Box` = `{C,F,I,L}`, `DIGITAL_ASSET_8949_FIRST_YEAR
= 2025`; both letter emitters (CSV + TUI), the `[I5]` broker-reporting advisory, and the export
help/man text all made year-aware. The filled-PDF layer was already correct (map-driven). Four
independent Fable tax-lens rounds to **GREEN 0C/0I** (reviews in `design/8949-box-fix/reviews/`);
year boundary + advisory gate + TUI mapping all mutation-verified. Suite 2088 green + all CI-only jobs.
**NOT yet merged to main — owner's call.** Prerequisite for conservative-filing D-6.

### RECONCILED-DONE (verified implemented / superseded — original OPEN tags below are stale)

- **Phase-2 tax deferrals → DONE** by the full-return (P0–P7) + `btctax-forms` crate: §170(f)(11)(F)
  year-aggregation (`forms.rs` `year_donation_deduction`), §170(b) AGI limits + carryover
  (`tax/charitable.rs`), the SE cluster (`w2_ss_wages`/`w2_medicare_wages`, `ReclassifyIncome`,
  `schedule_c_expenses`, §6017 $400 floor, §164(f) ½-SE), NII + **Form 8960** (`form8960.rs`), and
  the filled **8949 / Schedule D / 8283** PDFs (`fill8949*.rs`, `schedule_d*.rs`, `form8283.rs`).
- **"NEXT-cycle" forward-pointers → DONE** by later sections: Cycle-A `SelfTransferPassthrough` +
  the confirmed self-transfer matcher (`MatchSelfTransfers`), the whole **bulk-*** family
  (void/resolve-conflict/classify-inbound-income/self-transfer/reclassify-outflow/link-transfer),
  chunk-3 follow-ups #1–#8 (tui-edit-hardening), and tui-edit-hardening FU#1 (select-lots
  at-disposal pool, `main.rs:4102`, commits `8fd0517`/`de5e4a1`).
- **The two 🟡 "AWAITING WHOLE-DIFF REVIEW / not merged" sections → DONE**: pseudo-reconcile mode
  (`reconcile pseudo`) and the attest-export gate (`require_attestation`) are both live in source and
  shipped in v0.7.0.
- **price-data #41 → DONE** (check-isolation in CI, the real 5,800-row dataset, `btctax-update-prices`
  crate) and **crate publishing → DONE** (v0.7.0 fully published; `btctax` name reserved).
- **Sub-project A/B/C review nits → DONE** (recommendations adopted): Sub-A M1/Task-4/7/8/9/R0-C1 +
  N2/N3; Sub-B F1 + `MarginalRates.niit_applies` doc; Sub-C Task-3 (`available_lots_before`),
  Task-4, M-1, M-2.
- **Oldest whole-branch fixes → DONE**: core I-1/I-2, adapters I-1/I-2 + `AdapterError.adapter`
  rename + calamine Data-variant audit, store M-1/M-2 + OS-crash `repair()` + save-path zeroize +
  crypto-rust backend, CLI M-1/M-2/N-1 + attest advisory + `*_tag()` renderers + AllocLot dual-basis.
- **Input-form Task-2 (a/b/c) + banner (d–k) + P2-a → DONE**; **UX-P1/P2/P3/P4-\* → DONE**
  (post-v0.7.0 cycle; no self-marked-DONE item was found still-open).
- **MOOT (superseded)**: nettle-4.0 incompatibility + Sequoia/S2K pin (→ crypto-rust switch);
  core Task-0 version pins + adapters version pins (→ caret ranges + `Cargo.lock` reproducibility).

### ★ GENUINELY-OPEN INDEX (the live residue — verified absent in current source)

**A — Tax-scope features (deferred by design; larger work):**
- **AMT (Form 6251) compute** — only a fail-closed *screen* exists (`tax/amt.rs`; refuses if AMT
  might apply). Computing an actual AMT liability is out of scope.
- **NIIT model is minimal** (B-M1, disclosed) — NIIT *is* computed (`form8960.rs`) but under-states
  for complex non-crypto NII; §1411(c)(2) active-trade lending exception not modeled.
- **engine-B gross-vs-net `crypto_ord`** ordinary-income coordination (high blast radius) — `compute.rs`.
- **Form 8949 broker-reported-box input + 1099-B / 1099-DA reconciliation** — the not-reported boxes
  are shipped year-aware (`Form8949Box` = `{C,F,I,L}`, `forms.rs`); this feature adds the *reported*
  boxes, which are also year-aware — A/B (ST) / D/E (LT) from a 1099-B pre-TY2025, G/H (ST) / J/K (LT)
  from a 1099-DA from TY2025.
- **Full-return export never emits the [I5] broker-reporting advisory** — `admin.rs` hardcodes
  `broker_reported_rows: 0` on the full-return packet path, so a full return with exchange disposals
  gets no [I5] flag (the crypto-slice path does). Pre-existing, era-independent; owning phase: a
  future export-parity pass. Wire `rows_possibly_broker_reported` through the full-return path (and
  emit the year-aware `broker_reporting_advisory`) or document the omission at the construction site.
- **§170(e) dealer/inventory CHARACTER detection** (investor case shipped; dealer over-flag) — `fold.rs:1247`.
- **Donee-type §170(e)(1)(B)** private-foundation modeling — caveat text only.
- **Gift-tax liability**: §2502 rate schedule, §2513 splitting, DSUE/portability, **Form 709 PDF** — no `form709.rs`.
- **`TransferIn` carries no basis/date hint** side-table (cross-crate gap) — `event.rs`.
- **Durable Path-A `Pre2025MethodDeclaration` ledger event** (attestation lives only in mutable config; R0-I2).
- TP8 m1/m2 de-minimis holding/loss-basis edges (accepted); §1015(d), non-BTC scope (intentional won't-do).

**B — Input-form store/UI polish (P2/P3 + macro/test nits):** P3-a dead `shadows`
(`form.rs:259`) · P3-b idle-tick flush no backoff (`main.rs:9848`) · P3-c unconditional "parked full
return" label (`main.rs:1427`) · P3-d `value_is_answered` treats `Money(0)`/`Bool(false)` as
unanswered (`draw_edit.rs:2590`) · P3-e seed truncation through `FIELD_CAP=64` (`form.rs:19`) · P2-b
`draft_exists` swallows DB errors (`.is_ok()`, `input_form_store.rs:99`) · P2-c `save_draft` skips
`schema_version` on the parked path · P2-d no `restore` on post-snapshot/pre-save `Err` · P2-Nits
(park gate `== Some(false)`; restore drops original error) · macro `(h)` near-dup
`decl_tristate!`/`skippable_tristate!` (`registries.rs:33`) · `(l)` coverage KAT doesn't assert EXEMPT
literals live (`coverage.rs:239`) · `(m)` NI-2 first-edit arm skips `guard_arity` (`apply.rs:31`) ·
Task-2(d) serde-wire round-trip breadth is Money-only (`seam.rs:280`).

**C — TUI/CLI Minors/Nits:** classify-raw 6-variant builder (TUI builds only Acquire+Income,
`form.rs:1848`) · read-only viewer never got `r`-refresh / `?`-help · ProRata cross-wallet
redistribution (core O4, `resolve.rs:1216`) · typed/sealed write-token (`export.rs`, `persist.rs:243`) ·
`fmt_btc`/`sat_to_btc` cross-crate dedup · `try_env_passphrase` duplication · bulk-link empty-plan
cosmetic (M1) + TUI date-RANGE filter + backport typed-dest to `l` · bulk-reclassify-outflow
Gift/Donate targets (`eventref.rs:166`) · chunk-4a link-to-never-seen-wallet + TargetPick empty-list
hint · chunk-4b optimize-accept `made`-date threading · chunk-5 allocate-E2E skip-guard +
`AllocLotRow`→`TargetList` · tui-edit-hardening FU#2 (SelfTransfer reconstruction as `pub fn`) ·
save-rollback FU (retire `attest_save_failed`) · Cycle-A `--acquired` future-date warning · bulk-sti
colon mis-align + price-data cache-provenance marker · `export_snapshot` timestamped filenames · fs2→fd-lock
swap · CI `cargo-audit` / `cargo-deny`.

**D — Oracle-sweep test-hardening (ownerless post-ship residue):** OS-WB-1 (taxcalc-leaf perturbation) ·
OS-WB-2 (readback parametrization) · OS-WB-3 (`compare` top-level catch-all) · OS-WB-4 (`Sign::Unsigned`
leading-minus assert) · OS-WB-5 (doc cosmetics) · OS-WB-6 (harness `paper` closure) · OS-14.2 (8995-L12
single-witness WEAK).

**E — WATCH / externally-blocked (not actionable now):** §1091 crypto wash-sale enactment (C.5, Congress) ·
TY2027 tables (IRS/SSA, fall 2026) · FmvMissing beyond the bundled price range (offline-reproducibility limit).

**F — NEEDS-OWNER (a user decision, not a code fix):** P9(b) retire refuse-and-reimport (release gate;
no real return entered yet) · Swan `Transaction ID` stability / `Total`-USD cost semantics · Coinbase
internal-move default · store Windows world-readable CI assertion.

**G — Approach-B (defensive filing): pending architecture decision + the surviving items.**
See the dedicated section below — it supersedes the three per-cycle registries
(`design/defensive-filing-wizard/`, `design/stale-snapshot-latch/`,
`design/f8275-part-ii-overflow/FOLLOWUPS.md`), which are now historical except for the items
restated there.

---

## ★ APPROACH-B ARCHITECTURE DECISION + POST-WIZARD RECONCILIATION (2026-07-27)

### G-0 — THE OPEN DECISION (owner's call; nothing is being built until it is made)

Approach B (declare a $0 tranche of undocumented BTC → promote it to a >$0 **floor** → mandatory
Form 8275 disclosure) shipped its engine in v0.9.0 and a ratatui wizard in v0.10.0. The wizard's
construction went badly enough to warrant a disclosure in `NOTICE`.

**★ RESOLVED 2026-07-27 — the owner chose the cut, and it is MERGED to `main`.**
`arch/engine-keep-wizard-cut` was merged (no-ff) after the reconciliation below was written: the
engine stays, the TUI wizard and the stale-snapshot latch are gone, `btctax defensive status`
replaces the dashboard as a read-only surface. Merged result verified green — 2,407 tests pass,
`cargo fmt --all -- --check` clean, `clippy -D warnings` clean. **No version bump, tag, or crates.io
publish accompanied this merge**; `main` still reads v0.12.0 in every manifest while the shipped
0.12.0 crates contain the wizard. That divergence is deliberate and must be closed by the next
release — see G-3.

| Branch | Status |
|---|---|
| `main` | **engine kept, wizard cut** (post-merge); manifests still say v0.12.0 |
| `arch/engine-keep-wizard-cut` | merged to `main`; kept as a ref, never pushed |
| `backout/pre-approach-b` | the v0.8.0 tree — before any of this existed; **pushed**, retained as the escape hatch |

The choice was made on the basis that the re-layering in G-0's paths must delete exactly what this
branch deletes and reuse exactly what it keeps, so building anywhere else meant porting the latch
subsystem through a crate split it does not survive, then deleting it anyway.

Analysis of record, both now on `main`:
`design/arch-engine-keep-wizard-cut/COMPARISON.md` (measured three-way comparison) and
`design/arch-engine-keep-wizard-cut/UI_READD_SKETCH.md` (independent architect's sketch for
re-adding a swappable UI). **Load-bearing finding, verified directly:** the wizard's state machines
were *already* UI-agnostic — `declare_flow.rs` and `promote_flow.rs` contain zero ratatui/crossterm
code (their only grep hits are comments saying so), and `defensive_dashboard.rs` has exactly one
crossterm import. The 3,901 lines restore near-verbatim from git history. What was actually bad was
the ~1,200 lines of wizard key dispatch in `main.rs` and the latch subsystem built to compensate.

**The decision to make — two costed paths, both building on `arch/engine-keep-wizard-cut`:**

- **(i) FULL SEAM** — carve a UI-free `btctax-edit` crate that generalizes `btctax-input-form`'s
  proven pattern to the whole editor (`ActionId` where it has `FieldId`, a serde `Cmd` where it has
  `Edit`, a `ViewModel` where it has `Pane`, with `btctax_input_form::Edit` embedded verbatim as one
  `Cmd` variant so there is never a second field wire); add the missing read-only
  `availability(ActionId) -> Ready{candidates} | Empty{note} | Refused{reason}` query; restore the
  wizard behind it; prove it with a headless JSON driver that links no ratatui.
  **≈ 8,000–12,200 new production lines + 6–10k test lines; 18–30 review-gated tasks; 2–5 weeks.**
  Spread driven mainly by the `handle_*_key` → `FlowCmd` compression ratio (the softest number) and
  whether `draw_edit.rs` re-points cheaply.
- **(ii) STAGED (P1 + P2 + gen-protocol)** — do only the crate split, the availability query, and
  the `SnapshotGen` commit protocol; restore the wizard's UI-free flow files behind that thinner
  seam; **defer** the `Cmd`/`ViewModel` envelope until a second front end is actually committed.
  **≈ 8–12 tasks.** Banks the three assets any web project certainly reuses. Cost: general-editor
  flow *transitions* stay keypress-shaped, so the editor is swappable-for-defensive-filing but not
  wholesale.

**The question that picks between them:** is a web UI committed within roughly a year? If yes, (i);
if it stays speculative, (ii) — the envelope built without a real second consumer risks being shaped
wrong either way. The sketch's phase order is arranged so stopping after (ii) is a coherent landing
point, not an abandonment.

**Common to both, and the reason either is worth doing:** `SnapshotGen` pins every plan to the
projection generation it was computed from and refuses to commit across a generation bump. That
makes the v0.10.0 Critical — *a confirm tail computing a filed number from a projection older than
the confirming keystroke* — **unrepresentable by type**, replacing the entire stale-snapshot latch
subsystem and its four mechanized source-scanning guards with one comparison.

**Owner ruling already recorded (2026-07-27):** amending several prior years at once is **not a real
workflow**, so the composed multi-year export (`plan_export`/`apply_export`) is not to be designed
for. It is dead code on `arch/engine-keep-wizard-cut` (zero callers from any shipped surface) and
wizard-only on `main`; delete it on whichever branch survives.

### G-1 — SURVIVING items (live on every branch; NOT pruned)

These are in code that outlives all three outcomes — `btctax-core`, `btctax-forms`, `btctax-cli`
(including its published API), and the general ledger editor. Restated here because the per-cycle
registries they came from are now historical.

**Engine / filing-adjacent:**
- **tax-M-3 — displacement-caveat hole, and it is now WORSE than when filed.**
  `btctax-core/src/defensive/mod.rs:659-688`: `WouldDisplaceIfPromoted` fires only when
  `covered_sat == 0`; when `covered_sat > 0 && t.sat == covered_sat` neither it nor `OverCovered`
  fires, yet a HIFO reorder across multi-year disposals still shifts gain between years — so that
  row's per-year delta is a reorder artifact shown as an unqualified saving. **Escalation:** this was
  filed as a wizard-dashboard item, but `arch/engine-keep-wizard-cut` added
  `render_defensive_saving` (`btctax-cli/src/render.rs:4175`), which prints "would save an estimated
  $X in federal tax" off the *same* `journey_view` advisories — so the new CLI surface inherits the
  identical hole. Fix at the core: fire on `!promoted && displaces_documented_basis(..)`, suppressing
  only where `OverCovered` already carries displacement copy.
- **tax-M-4 (generalized) — a bare gain-Δ must never be printed without its displacement caveat,
  *wherever* it is printed.** Originally scoped to `declare_flow.rs:293-307`; the standalone basis is
  surface-independent and now binds `render_defensive_saving` too.
- **arch M-3 — two public `render_consent` functions on `btctax-cli`'s published API**
  (`cmd/promote.rs:186`, `chokepoint/mod.rs:438`). Published surface, unaffected by the UI decision.
- **Era/straddle doc precision** (`btctax-core`): the straddle invariant's stated consequence is a
  non-sequitur (`era.rs:25`); the arch-M-1 phrasing sweep is incomplete (`defensive_era.rs:74-76`,
  `era.rs:151-155`); "~1,461 presses … alone" overstates (`era.rs:62-63`, `SPEC.md:247-249`).
- **All seven `design/f8275-part-ii-overflow/FOLLOWUPS.md` items stay live** — they are entirely
  `btctax-forms`/`btctax-cli` (per-item Part II numbering, a record-time length bound in
  `plan_promote`, the inflated unbreakable-token row count, a fill-layer emptiness re-check, the
  ~28-line/~3,700-char ceiling wanting documentation, `field_names()` delegation). No UI dependency.

**General ledger editor (predates Approach B, serves every flow):**
- `flush_tax_inputs_draft`'s residue refusal returns `None` with no status and without clearing
  `dirty` (`main.rs:1264-1278`), so `q`/`Esc` can look like a dead key under a latch whose own
  message says "Quit the editor NOW"; also update its "Returns" contract doc.
- `handle_tax_inputs_key` can panic via `session.as_mut().unwrap()` (`main.rs:1278`) if
  `tax_inputs_form` is `Some` while `session` is `None` — the "convention, not construction" class.
- `approve_all_pseudo_defaults_then_fail_reprojection` (`main.rs:31384`) is a shared fixture that
  never asserts, on its own, that the approval write landed before the induced failure.
- **`BTCTAX_PRICE_CACHE` cross-test race** — 8 unsynchronized `std::env::set_var` sites
  (`main.rs:17969-18284`); a warning note about this was deleted in `8f84326` with no replacement entry.
- `corrupt_cli_config` (`main.rs:30998-31009`): doc wrong on two counts, and its bare `INSERT` will
  panic if a future fixture pre-sets the key — needs `ON CONFLICT` or the sibling's `UPDATE` pattern.
- Browse status band **measures** `Line::from(String)` (`draw_edit.rs:334`) but **renders**
  `Paragraph::new(String)` (`:418`) — newline-blind vs newline-splitting. Unreachable today (no
  status embeds `\n`); fix by sharing one `Vec<Line>` between the measure and render passes.

### G-2 — PRUNED as superseded by the G-0 decision (~20 items, not lost — see the cycle registries)

Two whole classes are struck because their surface is deleted on `arch/engine-keep-wizard-cut` **and**
rewritten under either path in G-0. They are not defects being ignored; they are polish on code that
is not going to survive in its current form:

- **The stale-snapshot latch subsystem and its four mechanized guards** (~9 items in
  `design/stale-snapshot-latch/FOLLOWUPS.md`): guard (b)'s function-scope presence test,
  `NESTED_EXEMPT_OPENERS`'s unasserted parent tuple, the column-anchored `#[cfg(test)]` detection,
  the unconfined `stale_after_write` clear sites, `ALL_25_OPENER_KEYS`/`KEYMAP-SYNC` desync, the
  write-tail-prefix citation drifts, the park-ordering substring assertion, and the T7 armed-dashboard
  clipping residual. **Superseded by `SnapshotGen`**, which removes the subsystem outright.
- **Wizard TUI chrome** (~11 items in `design/defensive-filing-wizard/FOLLOWUPS.md`): Browse footer
  `w`, flow-render scrolling, `Esc` at the PartII step, the ~230-char NOTICE clipping below 77
  columns, dashboard-only render KATs, the "[optional, SUPPRESSED]" copy, Debug-format rows, the
  Provenance screen's answer-before-asking wording, free-text date/sat entry, and plan-doc drift.
  **Superseded by** the `Cmd`/`ViewModel` rewrite (path i) or the thin-seam restore (path ii); the
  flow files come back from git history and get re-reviewed as new either way.

One cross-cutting lesson is worth keeping out of that pile: **stop hand-citing self-referential line
numbers in doc comments.** The drift entries above are the third recurrence; name the
function/const instead where the citation is not load-bearing, or add a merge-time doc-lint that
flags stale `` `:\d+` `` citations.

### G-3 — CLOSED at the 0.13.0 release (2026-07-27)

Filed when the wizard cut landed on `main` with no version bump, leaving `main` divergent from the
published 0.12.0 crates. All three items discharged in the 0.13.0 release:

- **[done] Version bump.** All 12 crates 0.12.0 → 0.13.0. Breaking (a shipped interactive surface
  removed), and pre-1.0 cargo SemVer makes a breaking change MINOR.
- **[done] `plan_export`/`apply_export` deleted** per the G-0 owner ruling — the composed multi-year
  export trio (`ExportPlan`, `ExportOutcome`, `ExportOutcomes` and both fns), its crate-root
  re-export, and its 5 dedicated tests. Two `flagged_years` tests that *also* asserted the composed
  plan were trimmed rather than deleted, keeping their live half.
  **Two things that deliberately survived the cut, both verified:** `conservative::flagged_years`
  (a separate symbol — `btctax defensive status` reports its year set,
  `btctax-core/src/defensive/mod.rs:681`), and `chokepoint::promoted_filing_years`, a live
  `pub(crate)` helper for `cmd/admin.rs`'s `promote_export_gate` that merely *sat inside* the Task-3
  region. The first cut removed it; its own surviving unit test caught that immediately.
- **[no change needed — the filed claim was WRONG] `NOTICE`'s experimental section.** This item
  asserted the section "tells filers to check things on screens that no longer exist" and that its
  point-of-use claim was "now only half true". **Both were false, verified against the cut tree:**
  all three of its performable checks are PDF/figure checks (Part II renders in full; Form 8949
  column (e) equals the consented floor; tranche quantity and window match the 8275) — none names a
  screen. And the notice is still surfaced in *both* places it claims: the TUI
  (`btctax-tui/src/draw.rs:117,167`, `btctax-tui-edit/src/draw_edit.rs:93` — `uses_approach_b` stays
  wired into Browse's banner) and the CLI's stderr (`cmd/defensive.rs`, `cmd/admin.rs:242`).
  `NOTICE` was left untouched. Recorded because acting on the claim would have edited a filer-facing
  disclosure to fix a defect it did not have.

### G-4 — findings from the 2026-07-27 return simulation (a $1M-wage / $500k-LTCG MFJ TY2024 return)

A four-lens adversarial review hand-computed a complete return against btctax's output and checked every
figure. **btctax's tax arithmetic was correct throughout** — the $100,000 §1(h) LTCG tax, the $19,000
§1411 NIIT, and the $119,000 crypto-attributable total all reproduce exactly under independent
with/without recomputation, and it correctly did **not** net the charitable gift against net investment
income (a common preparer error that would have understated NIIT by $3,230). Form 8949 Box F, four rows,
the $25,000-per-half-BTC basis split, and the all-long-term classification are all right. What follows is
the residue.

- **[done] AMT screen worksheet line 2** — fixed in `fix/amt-screen-line2` (`731228c`). See that commit;
  the defect was reading Schedule A line 7 as the itemized total.
- **[TIER 1 DONE — shipped v0.14.0, 2026-07-29 / TIER 2 open] Compute Form 6251 instead of refusing —
  TWO TIERS, see `design/amt-form6251/PLAN.md`.** *The* headline finding, and it grew after further
  analysis on 2026-07-27. **Tier 1 shipped**: the form is transcribed line by line, computed for every
  return, and the gate is now i6251 *Who Must File* condition 1 (line 7 > line 10) rather than the
  screening worksheet. **Tier 2 — filling and ATTACHING the form when AMT is owed — remains open and is
  blocked on G-6** (no oracle can validate an AMT figure).

  **Tier 1 — the zero-AMT case.** The originally-simulated taxpayer ($1M wages / $500k LTCG / MFJ) owes
  **$0 AMT** — TMT $327,965 against regular tax $364,675.50, a **$36,710.50** margin — yet btctax refused
  the whole return and wrote no forms at all. The refusal threshold is low: worksheet line 11 > $232,600,
  i.e. (MFJ, no QBI) whenever AGI less non-SALT itemized exceeds about **$365,900**. Because Form 6251
  need not be *attached* when AMT is $0 ("Who Must File" is not met) and Schedule 2 L2 → L3 → 1040 L17 are
  all already $0 in v1, **the printed forms for a zero-AMT filer are byte-identical to today's** — the
  whole change is to stop refusing. No PDF asset, no map, no emitter.

  **★ Tier 2 — AMT is genuinely owed by a large slice of the target audience.** Do NOT ship Tier 1
  believing it closes this. Mapping the (wages × gain × donation) space produced a clean rule:

  > AMT is owed when the exemption is FULLY phased out (AMTI ≥ **$1,751,900** MFJ) **and** ordinary
  > taxable income is below **$769,139**.

  The gain phases out the exemption; the wages decide the outcome. Below the crossover the graduated
  regular brackets are cheaper than AMT's flat 26/28%, so TMT wins. Worked grid (MFJ, standard deduction,
  no donation) — AMT owed, in dollars:

  | wages ＼ gain | $1M | $2M | $5M | $10M | $25M |
  |---|---:|---:|---:|---:|---:|
  | $100,000 | — | 22,916 | 22,916 | 22,916 | 22,916 |
  | $300,000 | — | 29,731 | 29,731 | 29,731 | 29,731 |
  | $500,000 | 9,997 | 30,779 | 30,779 | 30,779 | 30,779 |
  | $700,000 | 12,186 | 15,818 | 15,818 | 15,818 | 15,818 |
  | $900,000+ | — | — | — | — | — |

  **This is btctax's archetypal user**: a salaried engineer who sells a large Bitcoin position. At
  $250,000 of wages and a $2M gain the AMT is about **$28,000** — mandatory, and Tier 1 alone would still
  refuse them. Exposure is bounded (it plateaus once the exemption is gone, because §55(b)(3) taxes the
  gain at 20% in both systems so further gain cancels) and peaks near **$24,615** at ~$384,000 of ordinary
  taxable income, but it must be filed. Tier 2 therefore needs the real thing: PDF asset, AcroForm map,
  emitter, and Schedule 2 L2 wired through to 1040 L17.

  **Donations are non-monotonic** and defeat any rule of thumb: at $1M wages / $10M gain a gift above
  ~$230,861 *creates* an AMT liability (each charitable dollar cuts regular tax 37¢ but TMT only 28¢, so
  9¢ is clawed back), while in the low-wage/high-gain cells a large enough gift *removes* AMT by shrinking
  AMTI enough to restore part of the exemption.

  **Retracted:** an earlier note in this entry claimed AMT was "structurally unreachable" and that the
  margin "never closes". That was true only for the $1.5M-AGI scenario it was derived from. It does not
  generalize, and the grid above is the counterexample. Superseded by the rule stated here.
- **[open] Schedule D lines 17 and 20 are determinable, not "out of scope".** The crypto-slice export
  leaves them blank and tells the filer to complete them by hand, but both are Yes/No routing boxes fully
  determined by data already in the packet: L17 = Yes (both L15 and L16 are gains), L20 = Yes → use the
  Qualified Dividends & Capital Gain Tax Worksheet. L20 is the single determination that picks which of
  two worksheets computes the tax. btctax's own full-return engine already derives exactly this
  (`printed.rs`, `ScheduleDRouting`); the crypto-slice path never reaches it. L18/L19 (28%-rate,
  unrecaptured §1250) genuinely are out of scope and correctly blank.
- **[open] `report --tax-year` prints a misleading LTCG marginal rate.** It reports "LTCG 0.20" when the
  all-in marginal rate on the next dollar of long-term crypto gain is **23.8%** (§1(h) 20% + §1411 3.8%)
  — which is the very $119,000 ÷ $500,000 it prints two lines earlier. A filer sizing a sale off the
  20% figure under-reserves by 3.8 points.
- **[open] The crypto-slice `form_1040_capgains.pdf` should be watermarked as a partial worksheet.** It
  renders as "Form 1040" showing $500,000 on line 7 and a blank line 1a — for a taxpayer with $1,500,000
  of income. The stderr note says only two fields were filled, but the artifact outlives the note. A
  document that looks filable and understates income by $1,000,000 if mistaken for one.
### G-6 — the oracle cross-check RAN, and it found a Tax-Calculator defect (2026-07-28)

**★ CORRECTION.** An earlier version of this entry said `taxcalc`/`pandas` were "not installed" and
filed the cross-check as blocked. **That was wrong** — I checked bare `python3` instead of the repo's
`.venv`, which carries **taxcalc 6.7.2 / pandas 3.0.3**. The probe ran.

**Result: 9 of 11 vectors agree with an independent engine**, and the two that differ are a defect in
**Tax-Calculator**, not in btctax. Run it with
`.venv/bin/python scripts/oracle/verify_f6251.py` (0 unexpected divergences).

**The defect.** `taxcalc/calcfunctions.py`, the AMTI block:

```python
if standard > 0.0:
    c62100 = c00100 - e00700 - qbided - standard     # subtracts it, never adds it back
```

Its **itemizer** branch correctly adds Schedule A line 7 back (`+ c18300`) — which is why every
itemizing vector agrees to the cent. Its **standard-deduction** branch stops at Form 6251 line 1 and
never applies line 2a's else-clause: *"If filing Schedule A (Form 1040), enter the taxes from Schedule
A, line 7; **otherwise, enter the amount from Form 1040 or 1040-SR, line 12**."* i6251 p.2 repeats it,
§56(b)(1)(D) mandates it, and i6251's own TIP depends on it (*"the standard deduction isn't allowed for
the AMT"*). Measured: every standard-deduction vector shows ΔAMTI = exactly the standard deduction,
every itemizer 0, and V8's Δ19,600 = $14,600 MFS standard + the $5,000 MFS kicker taxcalc also omits.
**Direction: Tax-Calculator UNDERSTATES AMT for standard-deduction filers.**

**★ Why this matters beyond AMT.** Had the plan's original instruction been followed — *"derive every
vector's TMT from `c09600`"* — the oracle would have "corrected" a faithful transcription into a wrong
one on exactly the two vectors that owe AMT. This is the concrete instance of
`CLAUDE.md`'s rule that a domain fact can make a green test meaningless: the form is the authority, and
a taxcalc disagreement is adjudicated against the PDF, never encoded.

**⚠ RETRACTED: do not file upstream yet — this is a ONE-oracle result.** An earlier version of this
entry said to report it. That skips this repo's own standard, which is **two** oracles. Corrected
below, and the reason it could not be met is itself the more important finding.

**★★★ CORRECTED 2026-07-29 — THE BLIND SPOT WAS (b) ONLY, AND OTS ARBITRATES AMT DECISIVELY.**

I claimed, here and in the shipped v0.14.0 release notes, that *"OpenTaxSolver computes no Form 6251 at
all."* **That is false.** `taxsolve_US_1040_2024.c:222` defines
`form6251_AlternativeMinimumTax(int itemized)`, "Updated for 2024", and the solver prints the WHOLE form
line by line (`AMT_Form_6251_L1` … `L40`) into its 1040 output, then `L[17] = Sched2[3]`. I asserted the
absence without ever installing the tree or reading the source — the same error as claiming taxcalc was
uninstalled after checking the wrong Python.

**Installed and RUN 2026-07-29** (`~/OpenTaxSolver2024_22.07_linux64`, v22.07). On our upstream repro —
MFJ, $250,000 wages + $2,000,000 LTCG, standard deduction — OTS gives:

| line | OTS | btctax | Tax-Calculator |
|---|---:|---:|---:|
| 6251 L2a (std-deduction add-back) | **29,200** | 29,200 | *omitted* |
| 6251 L11 = 1040 L17 (the AMT) | **26,271** | 26,271 | 18,331 |

Critically, `taxsolve_US_1040_2024.c` codes line 2a as `if (itemized) amtws2a = SchedA[7]; else
amtws2a = L[12];` — byte-for-byte the branch btctax implements and the one Tax-Calculator gets wrong.
**So PSLmodels/Tax-Calculator#3108 is now corroborated by a genuinely independent second oracle**, which
is what this project's two-oracle standard demands and what we filed without.

**[open] What actually remains:** the harness never asks OTS for it — `scripts/oracle/ots_direct.py` extracts
`income_tax_before_credits = L16` and `total_tax = L24 + niit`, and never reads the 1040's **line 17**,
which is exactly where AMT lands (its only `L17` is Form 8960's NIIT). So the differential sweep has no
second opinion on AMT and never has.

**This is a Tier-2 blocker, not a nicety.** Tier 2 exists to serve filers who genuinely OWE AMT. For
that population the model degrades from two oracles to one — and that one is the engine we now suspect
of understating. Widening the corpus (below) does not fix it; it produces households only one oracle
can score. **Before Tier 2 ships, either teach `ots_direct.py` to read the 1040 L17 and install OTS, or
state plainly that AMT-bearing returns are validated by the form and hand-derived vectors alone.**

**[DONE 2026-07-28/29] Reported upstream as PSLmodels/Tax-Calculator#3108** (open), and as of
2026-07-29 **corroborated by OTS** per the correction above — so the two-oracle bar is now met
retroactively. Original text (this project has form: tenforty issue #278 / PR #279). Minimal repro to confirm against a second engine first: MFJ, $250,000 wages +
$2,000,000 long-term gain, standard deduction → taxcalc `c09600` = $18,331; the form gives **$26,271**.
Current evidence, all consistent but all one-sided: the form's own line 2a text, i6251 p.2 and its TIP,
§56(b)(1)(D), taxcalc's own source, and r1's independent hand-derivation (which pre-dates the code and
matches btctax).

### G-6b — ★★★ TIER-2 ENTRY CRITERIA (Fable consult, 2026-07-29). Tier 2 does NOT ship until these are green.

The consult was called once, before merging `feat/amt-oracle-comparison`. **Merge: YES**; G-6 stays
OPEN. What follows is its ruling, and it corrects our framing in three places.

**★ Our framing was wrong in three ways — recorded so the despair version does not get re-adopted:**
1. *"Both oracles are blind or defective in exactly the region Tier 2 operates in"* — **overstated.**
   taxcalc is defective only for **standard-deduction** AMT filers (#3108); for **itemizing** AMT
   filers it is a fully valid second oracle (V6 agrees exactly). OTS is disqualified on two named
   shapes. The doubly-dark region is roughly standard-deduction MFS-kicker filers — narrow. The
   two-oracle standard is ACHIEVABLE for the itemizing slice. Despair framing invites accepting weaker
   evidence than is actually available.
2. *"The corpus structurally cannot contain an AMT filer"* — **true today, contingent tomorrow.** Both
   rejection legs are OURS: one IS the Tier-2 change, and D-2 is a predicate we wrote. Corpus admission
   is a Tier-2 exit criterion, not an impossibility.
3. *"65 lines compared, 0 unexpected"* — both under- and over-sold. Fixed in `04ead65` (now 247).

**Entry criteria, each falsifiable:**
- **[DONE 2026-07-29 — E1]** Compare every non-echo line OTS prints. 247 comparisons, 0 unexpected.
- **[DONE 2026-07-29 — E2] A POPULATION of two-oracle AMT-owing agreement, not a point.** Was one
  vector (V6); now **22 AMT-owing vectors of 30**, and every filing status clears the attach gate with
  two oracles agreeing to the cent: single 6, hoh 4, mfj 5, mfs 1. Emitted by
  `design/amt-form6251/gen_e2_vectors.py`, which asserts each vector's intended routing at generation
  time. **Three of the five listed routings turned out to be UNREACHABLE with AMT owed**, and that is
  a result, not a gap — see E2's findings below.
- **[DONE 2026-07-29 — E3] Single and HoH.** Reference tables landed in `761dbf4` (E3a); the vectors
  are V11–V18 and V27/V29 (E3b). Both harnesses were silently mapping every non-MFS status onto MFJ —
  `form6251.rs`'s `bps()` (`Mfs => …, _ => …`) and `compute_vector` (`_ => Mfj`), and
  `verify_f6251.py`'s `MARS: 3 if mfs else 2`. Three wildcards, one defect class, all found by adding
  the first Single vector.
- **[open → Tier 2 · E4] Read the FILLED f6251.pdf back field-by-field** against the struct, plus the
  Σround/roundΣ residual rules on the 6251 → Sch 2 → L17 chain. A perfect computation still files a
  wrong number through a transposed AcroForm field. The sweep already reads other forms off the PDF.
- **[open → Tier 2 · E5] Lift D-2 for itemizing AMT households** once the refusal is gone; carry
  taxcalc as a known-defect class for standard-deduction ones (the existing L16 `KnownDefect` machinery
  is the pattern) with OTS live. The fixture is the bridge; the corpus is the ongoing guarantee.
- **[open → Tier 2 · E6] Lines 2c–2t become REAL FIELDS carrying provenance** — computed /
  declared-absent(QuestionId) / unreachable-with-proof — and attach requires no silent line. Adding the
  fields makes the compiler enforce totality at every constructor.

**★★ EQUIVALENCE QUESTIONS DO NOT SURVIVE INTO ATTACH MODE.** Refusal-by-declaration is sound only when
the question is a FACT THE FILER CAN VERIFY (an item's *existence*). `AmtDepreciationSameAsRegular` — the
declaration built this very session — asks the filer to affirm an AMT-technical *equivalence* most
filers cannot evaluate. They will guess "yes", and btctax will print a signed 0 resting on the guess.
For attach mode it must be re-phrased to existence: *"does your Schedule C include any depreciation or
§179 deduction?"* → yes ⇒ refuse. Same for any sibling. Owning phase: **Tier 2, before attach.**

**★★ THE MOST LIKELY WRONG FILED NUMBER: an ISO exercise (line 2m) printed as 0 on a signed form.**
It is invisible to the ENTIRE validation apparatus — no oracle can witness an input btctax never
collects, no vector can encode it, no transcription test reds — and post-TCJA it is the dominant
real-world reason an individual owes AMT, in exactly the high-income equity-comp population Tier 2 is
enriched for. "Unreachable at the input surface" conflates *btctax cannot see it* with *the filer does
not have it*; for a refusing v1 that was survivable, on an attached form it is the answered-ness defect
reappearing on 18 lines at once. Cheapest catch: the existence-question interview over i6251's
Who-Must-File Exception items (ISO, §1202, §4952, NOL, Form 8801, accelerated depreciation — PAB is
already refused), any "yes" refusing, plus a test asserting every 2c–2t field is non-silent before emit.

**Explicitly NOT to be done:** ship Tier 2 on the 11-vector fixture as-is; let any equivalence-phrased
declaration into attach mode; encode an oracle defect as truth; patch OTS locally (the form's own worked
example anchors the MFS kicker better than a modified oracle, and the observe-only posture stands); let
the fixture ossify into the permanent instrument; or close G-6 on this merge.

---

#### E2's findings (2026-07-29) — three routings are DEAD, one region has NO oracle

**★ Three of the five routings E2 asked for cannot own AMT at all**, and they are one fact, not three.
A ~450M-point scan over (wages, gain, gift, refund, FTC) at $500/$1,000 resolution, across all four
filing statuses and both deduction modes, found **zero** AMT-owing returns with:

- the **line-32 skip** taken, · **line 39 on the 26% side**, · the exemption **not yet phased out**.

The root is the third: *in btctax's input class, AMT is owed only once the §55(d)(3) phase-out has
begun* — the exemption is worth more than the flat 26/28% rate's excess over the graduated schedule
everywhere it survives intact. The other two follow: the phase-out starts at $609,350 (MFJ
$1,218,700), so an AMT-owing line 12 clears the $232,600 breakpoint (MFS $116,300) outright and also
clears the §1(h) 15%-band top, leaving a 20% tranche that keeps line 32 ≠ line 12.

That claim is **executable**, not prose: `amt_is_owed_only_once_the_exemption_phaseout_has_begun`
sweeps all four statuses — both deduction extremes (base and the §63(f) aged/blind maximum) and both
FTC extremes (zero and the §904(j) ceiling) — against the PRODUCTION regular tax (`qdcgt_line16`,
Tax-Table quantization included), guarded on `must_attach()` because that is what Tier 2 files on.
It reds when the input surface widens — an ISO exercise at line 2i, §1202 at 2h, §1250 gain reaching
line 14 — because a preference item then lifts AMTI without moving the regular tax, and **three
routings would go live with no vector and no oracle**.

★ **Its sensitivity is $4,400 of AMTI, measured by bisection ($4,200 green, $4,400 red) — not "any
widening".** That floor is arithmetic: a ΔAMTI moves the tentative minimum tax by only 26–28% of
itself, so closing the tightest margin (MFS, $1,098.50) needs ~$4,000. **A smaller preference goes
unnoticed**, so this tripwire narrows E6 but does not discharge it. Review r2 found the floor had
been far worse — the sweep pinned the BASE standard deduction and a zero FTC, and stayed green at
$5,000 while an MFS filer at the §63(f) maximum was already attachable with the exemption intact.

**★★ ONE REGION HAS NO ORACLE AT ALL — worse than the consult's "doubly-dark" framing, and narrower.**
The consult put the dark region at "standard-deduction MFS-kicker filers". Measured, it is **every**
MFS return with the exemption phased to zero, standard or itemized — because §55(d)(3) puts the MFS
zero-exemption threshold and the Form 6251 line-4 kicker start at the *identical* $875,950
(609,350 + 4 × 66,650). A search found zero MFS returns with a zeroed exemption below the kicker
start: for MFS the two conditions are one. And the kicker is exactly where both engines fail —

| engine | what it does with the §55(d)(3) MFS rule |
|---|---|
| OTS 2024 | implements the line-4 add-back, with the **stale 2023** constants (831,150/1,084,150/63,250) |
| Tax-Calculator 6.7.2 | implements the **exemption cliff** (`AMT_em_pe`, `calcfunctions.py:2590`) and **not** the line-4 AMTI add-back — its `c62100` block has no MFS branch at all |

So V23/V24/V25 owe AMT and **nothing witnesses them**. `verify_f6251.py` now prints a witness census
that says so every run, because two sections each printing "OK" concealed it. V22 — the phase-out
ramp, below the kicker start — is the two-oracle MFS vector the gate actually requires, and it exists.

★ **State that taxcalc claim precisely, because we have been imprecise here before.** Our upstream
comment on #3108 said §55(d)(3) "appears not to be modelled" and was corrected — `AMT_em_pe` does
model it. The accurate, grep-surviving claim is narrower: **the exemption cliff is modelled, the AMTI
add-back is not.** Verified by reading `calcfunctions.py` in the pinned 6.7.2, not from memory.

**→ NEW: G-6c** — report the missing §55(d)(3) MFS AMTI add-back upstream to Tax-Calculator. Owning
phase: **Tier 2**, and *not before* checking the latest release and re-running both oracles — the
#3108 lesson. Two witnesses already exist (the form's line-4 parenthetical with i6251 p.9's worked
example, and OTS implementing the add-back), so this clears the one-oracle bar that #3108 did not.

**→ NEW: G-6e — ★★ THE PIVOT: TY2025 comes before E4, and TY2026 fails closed (decided 2026-07-29).**
The MFS kicker region is unwitnessed in TY2024 *because of a stale-constant bug in one year's
solver*, not because the rule is unwitnessable. **OpenTaxSolver 2025 implements §55(d)(3) correctly**
— installed at `~/OpenTaxSolver2025_23.06_linux64`, verified against the IRS **final** 2025 form, and
smoke-tested on an MFS household above the kicker start: line 4 = 943,662.50 = 935,000 + 25% ×
(935,000 − 900,350), exemption 0, AMT 13,571.50. So building the same vectors at TY2025 witnesses the
rule, while the TY2024 constants stay anchored on the 2024 form. Adding TY2025 is also product work
we owe regardless — btctax computes only TY2024 in mid-2026.

**TY2026 does NOT follow**, for three independent reasons, each sufficient: (1) the 2026 instructions
are unpublished, so the phase-out rate, zero-exemption thresholds and kicker rate/cap are unknown —
inferable (500,000 + 2 × 70,100 = 640,200 implies 50%) but inference is what we forbid encoding;
(2) the form restructured Part I into 1a/1b around a new **Schedule 1-A**, so it needs
re-transcription, not new constants; (3) **no OTS 2026 exists**, leaving only taxcalc, which we know
is wrong on AMT. Held by `ty2026_full_return_must_stay_fail_closed`
(`btctax-adapters/src/tax_tables.rs`), mutation-verified — **adding TY2025 deletes the `2025`
assertion beside it, and 2026 must survive that.** Primary sources archived under
`design/amt-form6251/`: `f6251--2025.pdf`, `i6251--2025.pdf`, `f6251--2026-DRAFT.pdf`.
Full detail: `design/amt-form6251/CONTINUITY_TY2025.md`.

**→ G-6f — DONE 2026-07-29. `AmtParams.phaseout_rate` did double duty for two distinct statutory
rates**: the §55(d)(3) exemption phase-out (`form6251.rs:285`) and the MFS line-4 kicker (`:279`).
Both are 25% in 2024 *and* 2025, so the conflation was invisible and green; the 2026 draft implies the
phase-out moves to 50% with nothing saying the kicker follows, at which point we would print a wrong
number on a **signed** form with nothing reding. Now `exemption_phaseout_rate` + `mfs_kicker_rate`.

★ **The split needed a test, and finding that out took a mutation.** Swapping the two at their use
sites left the ENTIRE suite green — they are numerically equal today, so no fixture vector, sweep or
oracle can distinguish them. A split whose only evidence is a comment is not a split.
`the_exemption_phaseout_and_the_mfs_kicker_use_their_own_rates` gives them *different* values and
checks each rule took its own; both swap directions now red.

★ **Two §55(d)(3) IDENTITIES are now asserted too**, over every bundled year (so a year added later
is covered without anyone remembering): the add-back **cap is the MFS exemption** (clause (ii)) and
the add-back **threshold is the zero-exemption point** = `phase-out start + exemption / phase-out
rate` (clause (i)). Both are definitional and verified across three regimes — TY2024, TY2025, and the
TY2026 draft at its implied 50%. They turn five separately-typed MFS constants into a system that
catches its own transcription slips, which is what makes typing TY2025 in safe. Identity (i) is also
*why* the MFS region has no TY2024 oracle: "exemption gone" and "kicker live" are one condition.

**→ NEW: G-6d — no fixture vector has Schedule A line 7 > 0.** Every itemizing vector deducts a cash
gift only, so the fixture drives line 2a's *itemizer* limb at zero throughout. The limb is not
untested — the original shipped bug's regression KAT
(`amt::tests::itemizer_addback_is_schedule_a_line7_not_the_itemized_total`) exercises it with a
nonzero SALT, and `return_1040.rs:1439` wires `salt_5e` in — but nothing carries a line-7-live
household end to end through both oracles. Adding one means a SALT input in the vector surface plus
`A5a/A5b` (OTS) and `e18400/e18500` (taxcalc). Owning phase: **Tier 2 · E4**.

### G-9 — ★★★ ~~LIVE DEFECT IN SHIPPED CODE~~ **FIXED 2026-07-29**: the §63(f) aged box ignored the death carve-out

**Found 2026-07-29** by the Schedule 1-A spec review, which raised it about TY2025's Part V; checking
the TY2024 instructions showed the same rule governs a box **btctax already files**.

**The rule, verbatim from `i1040gi--2024.pdf` (Standard Deduction, line 12a):**

> **Death of spouse in 2024.** If your spouse was born before January 2, 1960, but died in 2024 before
> reaching age 65, **don't check the box** that says "Spouse was born before January 2, 1960."
>
> A person is considered to reach age 65 on the day before the person's 65th birthday.

The TY2025 instructions carry it identically, and Part V of Schedule 1-A repeats it with the IRS's own
boundary pair: *"born on February 14, 1960, and died on February 13, 2025"* qualifies; **February 12
does not**.

**What btctax does.** `is_aged` (`return_1040.rs:42`) decides the box from the date of birth alone:

```rust
pub(crate) fn is_aged(dob: Option<Date>, year: i32) -> bool {
    let Some(d) = dob else { return false };
    Date::from_calendar_date(year - 64, Month::January, 1).is_ok_and(|cutoff| d <= cutoff)
}
```

It is scrupulous about an *absent* DOB — `None` fails closed, deliberately, "never grant an
unsubstantiated deduction". It has **no death branch at all**, and `grep date_of_death crates/` is
empty: btctax does not collect one, so it cannot know.

**Consequence.** A spouse born before the cutoff who died in-year before reaching 65 gets a §63(f) box
they are not entitled to: **+$1,550 of standard deduction (TY2024 married rate), understating tax.**
That is the dangerous direction on a signed return. It is the answered-ness invariant again — btctax
silently answers *"was your spouse 65 when they died?"* for the filer.

**★ Neither oracle can catch it, so the two-oracle sweep reconciles on the wrong figure.** OTS takes a
filer-answered boolean (`taxsolve_US_1040_2025.c:2044`, `"You_65+Over?"`) and models no death date;
taxcalc has only `age_head`/`age_spouse`. Both are fed the same wrong premise, and every gate stays
green — which is why this survived to v0.14.0.

**Scope, stated precisely.** Confirmed for the **spouse** aged box. The taxpayer's own box is a
final-return question with its own rules and is NOT part of this finding. The blind boxes are
unexamined. TY2025 multiplies the stake: the same predicate gates Schedule 1-A Part V at **$6,000 per
person**, four times the §63(f) amount.

**Fix — LANDED 2026-07-29** on `feat/amt-e2-vector-population`, whole input stack, 2432 tests green.

- `is_aged(dob, died_during_year, date_of_death, year)` now applies the carve-out, with the
  day-before-the-65th-birthday convention in its own `reaches_65_on` — which also handles the **Feb 29**
  birth that has no 65th birthday (attained Mar 1 ⇒ reaches 65 on Feb 28). Without that fallback such a
  filer could never qualify at all.
- **Two gates, not one.** The scope note above was too narrow: the TY2025 instructions state the rule
  twice — *"Death of a **taxpayer** in 2025 … the taxpayer doesn't qualify"* as well as *"Death of
  spouse"* — so `HouseholdHeader::{taxpayer,spouse}_died_during_year` are separate declarations
  (`QuestionId::{Taxpayer,Spouse}DiedDuringYear`, `RefuseReason::{Taxpayer,Spouse}DeathUnanswered`).
  The taxpayer gate is always live; the spouse gate is live iff a spouse `Person` is on the return.
- **The gates sit on `HouseholdHeader`, the dates on `Person`.** Not a style call: `Person`'s name and
  SSN are serde-REQUIRED, so a gate on `Person` forces a complete `[header.taxpayer]` table into every
  inputs TOML that wants to answer it (five fixtures failed to parse before the move). The header is
  also where the other per-person declarations already live (`can_be_claimed_as_dependent_*`,
  `presidential_fund_*`). `Person::date_of_death` sits next to `date_of_birth`, where dates belong.
- **The date is class (B), the gate is class (A).** `SkippableId::Dod{Taxpayer,Spouse}` — skipping the
  date leaves the person unable to be *shown* to have reached 65, so the addition is FORGONE, never
  granted. Only the gate refuses. Both fail-closed arms are pinned, including the unreachable
  `(None, None)` one, so a future caller that bypasses `screen_inputs` cannot leak the defect back.
- **KATs, all mutation-verified** (`return_1040.rs`): the IRS's boundary pair for TY2024 (born
  1959-02-14, died 2024-02-13 qualifies / 02-12 does not) asserted both as a predicate and in dollars
  through the production `standard_deduction` ($16,550 vs $14,600 — *"this used to be 16,550"*); the two
  fail-closed arms; the leap-day case. Five mutations killed: reverting to the pre-fix DOB-only
  behaviour (6 tests red), `>=`→`>` on the boundary (4), granting on a dateless death (2), granting on
  an unanswered gate (2), dropping the leap-day fallback (2).
- **TY2024 figures provably unmoved:** every fixture answers "did not die", the golden matrix is
  byte-identical, and the only golden diffs are the four new keys appearing.

**Residue.** The **blind** boxes remain unexamined for a death interaction, and a TY2024 patch release
carrying this fix is still worth considering on its own merits (owner's call). Filed as G-9a below.

### G-16 — ~~LIVE: `delete_draft` is not a deletion~~ — **FIXED 2026-07-30**

**Fixed** by `sqlite_io::harden()`: `PRAGMA secure_delete=ON` on **both** connection paths
(`open_in_memory` and, critically, `db_from_bytes` **after** the deserialize, or an existing vault
reopened would keep leaking). Set at the connection rather than the call site, so it cannot be
forgotten by the next `DELETE`.

**The defect, for the record.** `delete_draft` was a plain `DELETE`; SQLite freed the row's pages
without overwriting them; `db_to_bytes` serializes **every page including free ones**; `save()`
encrypted that. So the draft's SSNs, DOBs and every superseded income figure the filer typed and
discarded rode into **every subsequent vault generation, indefinitely** — retention that was
unbounded and invisible, strictly worse than `.bak`'s bounded single generation, while the code read
as a deletion.

**Two tests, both written FIRST and observed RED, both mutation-verified** (flipping the pragma to
`OFF` reds both):

- `sqlite_io::deleted_content_does_not_survive_into_the_serialized_image` — the general property, on
  both the fresh and deserialized connection paths.
- `input_form_store::delete_draft_leaves_no_trace_of_the_discarded_values_in_the_image` — the actual
  artifact, with an identity-shaped sentinel.

★ **Both assert against the SERIALIZED IMAGE as raw bytes.** A row-level query reports the draft gone
and proves nothing — which is exactly how this survived review.

★ **Honest bound:** `secure_delete` governs *future* deletes. Free-page residue already present in a
vault predating this fix is not scrubbed by it; a one-off `VACUUM` would be needed. Moot today (no
users), stated so nobody assumes otherwise.

★★ **This was §G-14's prerequisite.** The shred's tombstone is only real if the destroyed content
leaves no bytes behind — that now holds, so §G-14 no longer has to carry the `VACUUM`/`secure_delete`
requirement itself.

### G-18 — ★★★ Form 1040 line 7: btctax neither attaches Schedule D nor checks "not required"

**Verified on an emitted return, 2026-07-30** (`btctax-forms/tests/field_census_slice.rs`). Found
while measuring §G-17 gap 2.

The form's own instruction, verbatim from the extract:

> **7  Capital gain or (loss). Attach Schedule D if required. If not required, check here** ☐

For `w2_only_household` — the simplest filer, no capital transactions — the emitted packet is
**one form (`f1040`)**, and:

| | measured |
|---|---|
| Schedule D attached | **no** |
| line 7 amount | **`"0"`** — a printed zero, not a blank |
| "if not required" checkbox (`c1_23`) | **present in the PDF, UNCHECKED** |
| `c1_23` in the map | **absent** (0 occurrences) |

**The form offers two lawful states — attach, or check the box — and the emitted return is in
neither.** The IRS supplied exactly the provenance marker §G-13 wants for "blank because not
applicable", and btctax does not use it.

★ Interacts with **§G-11**: line 7 printing `0` is defensible as a *computed* zero (no transactions ⇒
no gain), but the unchecked box is the form's instruction unfollowed, and together they render a
return whose Schedule D status is unstated.

★ **Practitioner judgement flagged:** whether an unchecked box on a return with no Schedule D is a
filing defect or a cosmetic omission is a preparer's call. What is not in doubt is that the form
states an instruction we do not follow.

### G-17 — ★★ MULTI-FORM: the census needs form-level liveness, FQN normalisation, and cross-form provenance

**Owner question, 2026-07-30.** Detail: `design/forms/FIELD_PROVENANCE.md` §6g. Binds **§G-13**.

The census is keyed per-(form, year), so it generalises in shape. Already solid: `PrintedForms` holds
one `Option` per form and `fill_full_return` destructures it with **no `..`**, so the form set is
compiler-enforced and per-return inclusion is `Option::is_some`. ★ The census's form list must DERIVE
from `PrintedForms`, never a hand-list.

**Three gaps:**

1. **Overflow renames FQNs.** `overflow::merge_copies` uniquifies every field name on copies 1.. (the
   ISO 32000 same-name trap). Form 8949 paginates at 11 rows/part, so 2+ copies is routine. Key the
   census on the TEMPLATE, but **normalise the copy prefix in any emitted-document check** — else
   every overflowing 8949 reds with ~200 phantom unaccounted fields and the checker gets muted.
2. **Cross-form provenance is inexpressible.** Schedule 1-A line 38 → 1040 line 13b; 8949 → Sch D →
   1040. **Is Schedule D line 16 blank because there were no gains, or because 8949 was never
   emitted?** Same blank, different provenance — spanning two forms, with no vocabulary for it.
3. **★ An absent form floods the RESOLUTION** (mis-framed on filing; corrected same day). `sch_c:
   None` ⇒ Schedule C's 105 fields are not-applicable *at the return level*, and r1's
   *not-applicable* is field-level.
   ★★ **CORRECTION: the form-set half already exists.** `btctax-forms/tests/census.rs` is a HARD gate
   asserting `fill_full_return` emits **exactly 15** keys, and it already records the trap this gap
   half-restated: *"SPEC §6.2 forbids reading a household's packet as the authority — kitchen_sink
   emits 13/15 … which would silently under-gate"*, so its fixture injects the missing arms.
   **The static census must therefore key on all 15 forms** — the full decision surface, never what a
   household happens to emit. What is genuinely missing is only the **per-return RESOLUTION gate**
   (`Option::is_some` before asking about a field), without which ~800 fields from absent forms
   would be reported unresolved.

**⇒ The per-return resolution needs TWO levels:** is this form in the return, and only then is this
field accounted for. The static census is unaffected.

### G-15 — ~~The question registry has NO YEAR DIMENSION~~ — **SUBSTANCE DONE 2026-07-30**

**⑦ verdict:** `reviews/shred-and-year-fable-r2.md`. **G-15 leads the whole sequence** — it is a small
compiler-driven change, every TY2025 census `asked()` entry needs year-correct liveness, and it must
land **before a third always-live workaround ships**.

**✅ BUILT.** `ReturnInputs.tax_year` (`0` = not stated); the row key stamps it at the ONE read
boundary and `set` refuses a disagreement, so the in-memory year and its row can never diverge.
`HasIncomeExclusion` retired from always-live to `ri.tax_year >= 2025` — **the workaround is gone**,
and an unstated year fails closed (not ≥ 2025 ⇒ not live). Six fixtures had to state their year, which
is the Declarations invariant working. `Durability::{PerYear, Durable}` added per entry, compiler-
forced on all 21; exactly the two DOBs are `Durable`, and **no declaration may ever be** (each asserts
about a tax year). All mutation-verified.

★ **Residual (not a gate):** the *confirmation UX* — showing a `Durable` prior and requiring an
explicit keystroke — has no consumer yet. The architect called it "ergonomics, never a gate"; it
lands with whatever surface first re-asks across a year boundary. And `answered_on` + prompt-hash
still want a home before answers exist in the wild (they cannot be back-filled).

**Original shape:** put `tax_year` **on `ReturnInputs`** as the authoritative copy — `set` keys the row from
`ri.tax_year`, `get(year)` refuses on mismatch, **no `Default`** (constructor-required, so the
compiler drives every fixture edit), schema bump under the existing refuse-and-reimport policy (free:
no users). ★★ **`live` KEEPS its one-argument signature** and reads `ri.tax_year` internally — P9's
"only one copy of each predicate" invariant survives exactly, whereas `live(year, &ri)` *"invites
every caller to pass a year that disagrees with the row's origin — a second copy in flight."*
Year gates transcribe the statute (`(2025..=2028).contains(&ri.tax_year)`); `HasIncomeExclusion`'s
always-live workaround retires to `ri.tax_year >= 2025`.

**Durability:** per-entry `PerYear | Durable`, **defaulting to `PerYear`** (fail toward re-asking).
★ `PerYear` yes/no re-asks **BLANK — never shows the prior**: for a one-keystroke answer the prior
*"buys nothing but anchoring."* `Durable` facts (DOB) **do** show the prior but require the same
explicit keystroke — never Enter-to-accept, never pre-filled — because a forced retype invites typos,
and for a DOB that is the worse failure. Either way it is a NEW answer bearing this year's date.

**Cost of doing nothing:** every future TY2025+ question ships always-live with a bespoke neutrality
proof — and **Schedule 1-A Part IV has none**; a "no" would swear to a predicate with no TY2024 legal
existence, the census would then cite fabricated testimony as provenance, and the interview bloats
O(years × questions).



**Owner question, 2026-07-30.** Detail: `design/forms/FIELD_PROVENANCE.md` §6d. Interlocks with
**§G-13** (the ⑥ consult assumed a *"year-stable"* registry — it is not) and **§G-14**.

**Handled:** per-year ANSWERS. `return_inputs::get/set(conn, tax_year)` keys by year; nothing carries
forward silently.

**NOT handled:** per-year QUESTION SETS. `questions.rs:403`, verbatim — *"`live` receives only
`&ReturnInputs`, which carries no tax year, so it CANNOT be scoped"*. `HasIncomeExclusion` (a TY2025
MAGI question) was therefore made **always live**, asking TY2024 filers a TY2025 question, justified
because `Some(false)` is provably neutral for TY2024.

★★★ **That justification does not generalise.** Schedule 1-A Part IV asks about a deduction that **did
not exist in TY2024**; a "no" there is not neutral, it answers a question with no TY2024 meaning.
The registry needs a year dimension — a tax year on `ReturnInputs`, or `live(year, &ri)` — and that
touches P9's central invariant (the liveness predicate is the only copy), so it is a spec item, not a
patch.

**Also missing — DURABILITY.** Every question is re-asked every year. Right for facts that change,
wasteful for date of birth. ★ A prior-year answer must NEVER silently satisfy this year's provenance
(the answered-ness invariant across a year boundary); the lawful shape is a **confirmation** — *"Last
year you said no. Still true for 2025?"* — which is a NEW answer with this year's date, not a carry.

### G-14 — ★★★ SHRED = tombstoned DELETE, **not** envelope encryption (⑦ consult: DO NOT BUILD IT)

**⑦ verdict, verbatim:** `reviews/shred-and-year-fable-r2.md`. **Envelope encryption is REJECTED** —
*"it cannot deliver what its name promises inside this vault, and a false crypto guarantee is worse
than none."*

★★ **Verified fact that decides it:** `atomic.rs:18-22` copies the pre-write ciphertext to `.bak`
before renaming. So a wrapped per-item key would sit in the **same plaintext DB as the answers it
protects**, one generation back, under the same never-rotated cert. Per-item keys protect nothing a
plain `DELETE` does not. (`export_snapshot` also writes **plaintext** `snapshot.sqlite`.)

**Build instead — tombstoned deletion, no new cryptography:**
`Answered(bool)` / `Shredded { answered_on, shredded_on, prompt_hash }` / `None`; rewrite the row with
content destroyed; **`VACUUM` or `secure_delete=ON` before serialize** (else rows survive in free
pages — *the most mutation-testable part: plant, serialize, grep the image*); **save TWICE** so `.bak`
also ages past the shred. Granularity per-(year, question) — free, no keys to sprawl.

★★ **A shredded answer must NEVER collapse to `None`.** `Shredded` is a determinate `asked()`
resolution — "answered, content destroyed on date D" — so a lawfully-blank field stays ACCOUNTED. It
is not the second store round 1 forbade: that forbade a *duplicate that can drift*; this is the
primary record, redacted.

**Gate + honest bounds:** offer shred only once the return is emitted + archived and carryover
propagated. ★ **An emitted return is NOT reproducible after a shred** (answers feed computation) — the
confirmation must say so. ★ **Never auto-shred**: a retention timer is software destroying the filer's
records on its own initiative, the same defect class as a manufactured `0`. ★ State the bound:
backups, `snapshot.sqlite` exports and filesystem snapshots are beyond btctax's reach.

★ Correction to the consult: it says "schema v3"; `SCHEMA_VERSION` is **1** today, so the next is v2.



**Owner requirement, 2026-07-30.** Interview answers ("were you asked about foreign accounts? said
no") must be **vault-encrypted** (they are) **and cryptographically deletable after filing** (they are
not, usefully). Detail: `design/forms/FIELD_PROVENANCE.md` §6b. Interlocks with **§G-13**.

**Already true:** the whole vault is one encrypted SQLite image under a passphrase-protected OpenPGP
cert; `SecretBuf` mlocks + scrubs plaintext on the save path. Privacy at rest is DONE.

**The gap:** there is exactly **ONE cert per vault** — no envelope encryption, no per-item data keys.
"Destroy the passphrase" therefore destroys everything, and **basis carries forward across tax
years**: shredding the vault after filing loses the cost basis of every unsold lot. That trades a
privacy win for a permanent tax disaster, so it is not a usable post-filing action.

**What is needed:** per-item envelope encryption — answers under their own data key, wrapped by the
vault key; destroy the wrapped key ⇒ answers unrecoverable, **lot ledger intact**.

★ **A shred must not turn a lawfully-blank field into an UNACCOUNTED one** (§G-13 category 6). The
census likely has to record the decision's EXISTENCE separately from its CONTENT — *"declined
2026-04-15, detail shredded"* is still a determinate provenance.

★ **Never auto-shred.** Destroying evidence of diligence must be an explicit, informed act by the
filer — never a default or a background job.

### G-13 — ★★★ FIELD PROVENANCE — **COMPLETE: 15 of 15 forms censused (2026-07-30); 16 GAP fields**

**Owning phase: NOT B3.** Whole-surface, and it interlocks with **§G-11** (which blocks its honest
form). Filed 2026-07-30. **Full design note: [`design/forms/FIELD_PROVENANCE.md`](design/forms/FIELD_PROVENANCE.md)** — read that first; this entry is the register hook.

★★★ **Censusing form 2 found what form 1 could not: a field the taxonomy cannot hold.** Schedule SE
line **A** is the Form 4361 minister declaration — a class-(A) statement the filer is *required* to
make, which btctax **never asks**. It is not a forgone benefit (`unmodeled`); a minister with $400+
of other self-employment earnings would file an **incomplete Schedule SE** and nothing would say so.
Recorded as **`rule = "gap"`** — a countable, shrink-only DEFECT, pinned by
`recorded_gaps_may_only_shrink` so it cannot be added quietly or reclassified away. It closes by
adding a `QuestionId`, never by deleting the record.

★★★ **Schedule B found a WORSE one, and the form states the stakes itself.** Part III's 7a carries an
unnumbered sub-question — *"If 'Yes,' are you required to file FinCEN Form 114, Report of Foreign
Bank and Financial Accounts (FBAR)…?"* btctax collects 7a (`foreign_accounts`) and 7b
(`foreign_country_names`) but **never asks this one**, so it can only be left blank. The form's own
Caution, verbatim: *"If required, failure to file FinCEN Form 114 may result in **substantial
penalties**."*

★★★ **Form 8995 line 3 is the first gap in the UNDERSTATEMENT direction — the worst one.**
*"Qualified business net (loss) carryforward from the prior year"*, printed in parentheses because it
is NEGATIVE, and line 4 combines lines 2 and 3. So omitting it **inflates the QBI deduction and
understates the tax**. `qbi.rs:64` records the v1 choice in prose ("lines 3 and 16 stay blank"), but a
filer WITH a prior-year QBI loss is never asked, so nothing stops the overstatement.
★ This is exactly why `unmodeled` and `gap` must stay distinct: `unmodeled` is a benefit the filer
FORGOES (safe — it can only overstate tax); a missing REDUCTION is the reverse.

**Current gap count: 16 fields / 8 items**, pinned by `recorded_gaps_may_only_shrink` (`GAPS = 16`).
★ It was 18/9 for about an hour: **Schedule C line G was wrongly recorded as a gap and is now
`unmodeled`** — see the correction note below, which is the most useful thing in this section.
Counted per FIELD because that is the mechanical unit — a Yes/No is two widgets for one question.

| # | form | line | what is never asked | direction |
|---|---|---|---|---|
| 1 | Schedule SE | A | Form 4361 minister declaration | incomplete return |
| 2 | Schedule B | 7a-FBAR | *"are you required to file FinCEN Form 114?"* (2 fields) | penalties |
| 3 | Form 8995 | 3 | prior-year QBI loss carryforward | **UNDERSTATES** |
| 4 | Form 8283 | 5a | restriction on the donee's right to use or dispose (2 fields) | **UNDERSTATES** |
| 5 | Form 8283 | 5b | retained right to income/possession/voting (2 fields) | **UNDERSTATES** |
| 6 | Form 8283 | 5c | restriction limiting the property to a particular use (2 fields) | **UNDERSTATES** |
| 7 | Form 8283 | p2 header | name + TIN btctax HOLDS and never writes (2 fields) | administrative |
| 8 | Schedule C | I / J | Form 1099 filing declarations (4 fields) | penalties — **but see the open question below** |

★★★ **THE CORRECTION — Schedule C line G was wrongly a gap, and its prescribed fix was dangerous.**
The original entry claimed a "No" makes the income passive §1411 NII that btctax fails to tax, and told
an implementer to route a "No" into Form 8960 line 4a. **Building that would have added 3.8% NIIT on top
of the SE tax already charged on the same dollars — the double tax §1411(c)(6) exists to forbid.** Three
independent confirmations that the answer cannot change btctax's tax:

1. **§1411(c)(6)**, i8960 verbatim: *"NII doesn't include any item taken into account in determining
   self-employment income subject to tax under section 1401(b) for the tax year."* btctax's Schedule C
   income is not filer-typed — it is DERIVED as exactly the SE base (`return_1040.rs:527`
   `business_se_gross: se_net_income(state, year)`; `:1172` nets expenses off it; `:1201` calls it the
   *"SAME figure that feeds Schedule 1 line 3 and Schedule SE"*). So it is §1401(b) income by
   construction and is excluded from NII **whether or not** the filer materially participates.
2. **i8960's line 4b BODY** names this exact back-out: *"Net income or loss from a section 1411 trade
   or business that's taken into account in determining self-employment income."* A passive sole
   proprietorship enters 4a and leaves on 4b, netting 4c = 0.
3. **The repo already held the answer** — `design/full-return/FOLLOWUPS.md:481-483`: *"G especially: a
   mining/staking trade-or-business with SE tax is materially participated by construction."*

★★ **The root cause is worth more than the fix: I read Form 8960 line 4b's printed CAPTION
("non-section-1411 trade or business") instead of its instruction BODY, and concluded it was "dead".**
That is the paraphrase-instead-of-transcribe failure this project has a standing rule against —
committed *inside the census built to prevent it*, and it then propagated into an f8960 entry I
"corrected" on the same wrong reading. **A form's line title is not its instruction.** Both entries now
carry the correction inline.

★ **Honest residue, not rounded away:** below `SE_6017_FLOOR = $400` (`return_1040.rs:868`) no §1401(b)
tax is due, so §1411(c)(6) shelters nothing. A passive proprietor under $400 of net earnings who is also
over the NIIT threshold has at most **3.8% × $432 ≈ $16** of untaxed NII. Recorded, not counted.

★★ **OPEN QUESTION this raised, for the owner — it moves the count again.** `btctax limitations`
(`crates/btctax-cli/LIMITATIONS.md:231-233`) already tells the filer: *"Schedule C lines G, H, I, J —
left blank (deferred to your pen) … Fill them in yourself."* If a **disclosed** deferral counts as
honest provenance — and it is hard to argue it does not — then **lines I and J are not gaps either**,
and the count drops to 12/6. The census criterion never distinguished "silently blank" from "blank and
disclosed to the filer". Deciding that is a judgement call, not a mechanical one, so it is left open
rather than settled unilaterally.

★★ **Item 7 is a different species from the rest and is why `gap` is not simply "unmodelled".** Every
other gap is a question never asked; that one is a datum btctax **has** (it writes the name and TIN to
page 1) with no map cell on page 2 to write it to. "We have it and never write it" is not an honest
account, so it is a gap even though no figure changes.

★★★ **Recorded but deliberately NOT counted — the Schedule C aggregate-expense problem.**
`ScheduleCInputs` carries one `expenses: Usd` with no category breakdown, so btctax writes line 28
(*"Add lines 8 through 27b"*) while all of lines 8–27b are blank: **a total whose addends are empty**,
the only place on the return where a mapped line and its censused inputs do not cross-foot. Two
consequences follow mechanically — line 9 stays blank so Part IV's own gate (*"only if you are
claiming car or truck expenses on line 9"*) never opens, and line 24b has no place to apply the
§274(n) 50% meals limit. Each cell IS honestly accountable individually ("no breakdown is collected"),
which is why they stay `unmodeled`; the weakness is recorded once rather than smeared across twenty
reasons. **Owning phase: not B3.**

★ **Two near-misses worth keeping visible, both `unmodeled` by the criterion and both one field from
being closed:** Form 1040's **spouse IP PIN** (the taxpayer's *is* captured — a joint return omitting
an issued spouse PIN is rejected), and Schedule 3 **line 6b**, the Form 8801 prior-year minimum-tax
credit, which becomes a gap the moment a prior-year AMT input is collected.

**✅ The gate exists AND the ratchet is CLOSED.** `btctax-forms/tests/field_census.rs` asserts
`(map ∪ [census]) == the PDF's AcroForm field set`, exactly, for **all 15 forms**.
`CENSUS_NOT_YET_WRITTEN` ran 15 → 0 and its bound is now `is_empty()`, not a slack allowance — a 16th
form cannot arrive uncensused, because `the_two_lists_partition_every_form` forces it onto one of the
two lists, the emptiness assertion rejects the uncensused one, and `census_accounts_for_every_field`
rejects the other unless its fields are genuinely accounted for. Both routes fail closed.
Mutation-verified in both directions: dropping a census entry reds with *"in NEITHER the map nor the
[census]"*, censusing an already-mapped field reds as a contradiction, and restoring a form to
`CENSUS_NOT_YET_WRITTEN` reds two independent tests.

**Three `artifact` classifications recur and are worth naming**, because they are the cases where a
blank is not merely permitted but *required*: IRS **"Reserved for future use"** cells (Schedule 3
line 6e, Form 1040 line 30, Schedule 1 line 22); the **Paid Preparer** block on Form 1040 (7 fields —
a preparer attests under §6695 to their own PTIN and firm, and btctax is not one); and **Form 8283
Part V**, the donee acknowledgment, where writing the charity's own sworn statement would forge a
third party's testimony. Leaving these blank is the correct positive act.

★★ **The prototype's real lesson: this is knowledge made ENFORCEABLE, not new knowledge.**
`Form8959Map`'s doc comment already said it in prose — *"Lines 2/3 (Form 4137 / Form 8919) and all of
Part III plus line 23 (RRTA) are unmodeled and are deliberately absent"* — and `f1040.map.toml` says
the same of the spouse's IP PIN. The facts were already written down where nothing could check them.
That is the fourth time in one session the repo already held what was about to be built.

★ Format settled by the prototype: the `[census]` section lives **inside the `.map.toml`**, so the
map and the census are one edit and `no_unmapped_filled` reads the same universe (consult r1's
recommendation, now exercised). Each entry carries `line`, `rule`, and a `reason` that must name what
is forgone.

**The original finding.** `verify.rs::no_unmapped_filled` asserts every field carrying a value is authorised —
it stops us writing where we should not. **Nothing asserts the other direction:** that every field is
either mapped, or deliberately not ours. One direction stops stray writes; the missing one stops
silent omissions, and omission is the direction that costs a filer money.

**Measured (TY2024, the year whose maps are complete): 1158 AcroForm fields, 662 mapped, 496 with no
recorded decision.** Not 496 defects — 496 boxes where nobody wrote down whether we fill them, the
filer does, or they do not apply.

**The owner's taxonomy** — every field must resolve to exactly one, and only the last is a bug:
filled · **declined (asked, said no)** · not applicable · not ours · refused · **nothing ever
decided**.

★★ **A recorded "no" is provenance for US, not testimony on the return.** The line stays blank and we
must never print `0` to show the filer answered. That is why §G-11 gates the honest version.

★★ **Invert it and the forms derive the interview:** an unaccounted field is either a missing question
or a missing map entry. One y/n question can account for many lines (car-loan interest → Schedule 1-A
lines 22–30, nine lines, all lawfully blank). Much of P9's registry
(`btctax-core/src/tax/questions.rs`) already exists and must be built on, not duplicated — including
both doctrine classes and an `Option<bool>` answer that already distinguishes *unanswered* from *no*.

**Sub-item G-13a — question state is not visible at scale.** `screen_inputs` returns
`Option<Refusal>`, i.e. the FIRST unanswered live declaration only. N unanswered ⇒ N round-trips, and
the filer never sees how much is left. `live_questions` already enumerates; nothing aggregates.

### G-12 — btctax emits Form 8275 but NOT Form 8275-R, so it cannot disclose a position contrary to a REGULATION

**Owning phase: unassigned** — a product decision before it is a build. Filed 2026-07-30.

**The distinction, and it is not cosmetic.** Only **26 USC** is law. A Treasury regulation is the
executive's *interpretation* of the statute: binding in practice, and **capable of being wrong** — regs
are regularly held invalid for exceeding or contradicting the statute, the more so since *Loper Bright*
ended deference. **If the statute disagrees with a regulation, the filer has a duty to push back**, and
the system supplies the instrument for it:

| form | discloses a position contrary to | btctax |
|---|---|---|
| **8275** | rules **other than** regulations | ✅ emitted (`form8275.rs`) |
| **8275-R** | a **regulation** | ❌ **not modelled** (`f8275r.pdf` exists at the IRS) |

**Consequence.** btctax can take positions that agree with the regulations, or take a contrary position
**undisclosed** — which is the one outcome with penalty exposure. It has no way to take a statute-based
position *properly disclosed*. So the duty is not merely neglected; it is **unrepresentable**.

★ **This is not a call to become adversarial.** The honest observation is that the duty is routinely
neglected because challenging is expensive, and that is a legitimate choice a filer makes — but it must
be *a choice*, not an absence in the software. Cf. §G-11: the theme is identical, that btctax should not
silently decide something on the filer's behalf.

**What this is NOT:** an invitation to have btctax *identify* statute/reg conflicts on its own. That is
legal judgement and is out of scope in the same way intent is (§G-11's scope bound). The narrow question
is whether the filer, having taken such a position, can file it correctly.

### G-11 — ★★★ ARCHITECTURAL: the emitter cannot express "no testimony" — `Usd::ZERO` prints `0`

**Owning phase: NOT B3.** This is a whole-surface finding, larger than Schedule 1-A, and it must not be
smuggled into a task that cannot discharge it. B3 must (a) not make it worse, and (b) carry the
distinction in its own types so it is ready when this is fixed. Filed 2026-07-29.

**The framing that produced it** (user, and it is the sharpest statement of this project's core hazard
yet): *"Every entry on a line is effectively testimony from the filer against the filer in a court of law.
A blank line means no testimony provided and could be lawful and appropriate or an outright financial
crime; the distinction is intent and that's not the domain of tax software."*

**So a blank and a printed `0` are not two renderings of one value. They are different speech acts:**

| output | what it says under §6065 |
|---|---|
| blank | **nothing.** No testimony on this line. |
| `0` | **an affirmative sworn statement** that the amount IS zero. |

When btctax writes `0` on a line the filer was never asked about, it does not merely guess a number — it
**fabricates testimony and files it under someone else's signature.** That is a different and worse
failure than a wrong figure, and it is the [answered-ness invariant] at the emission layer.

**The defect, verified 2026-07-29.** `btctax-forms/src/lib.rs:77` is the whole money path:

```rust
pub(crate) fn fmt_money(d: Usd) -> String { d.to_string() }   // Usd::ZERO -> "0"
```

Every money field on every emitted form is `Usd`, never `Option<Usd>`. **There is no representation for
"no testimony", so no line can choose it.** Zero-suppression exists only ad hoc and only for whole ROWS
(`schedule_d.rs:26`, `fill8949.rs:44`), never for line-level blank-vs-zero. Invisible to every test and
both oracles, because `0` is the correct *value* in the overwhelming majority of cases — the defect is in
the *act*, not the arithmetic.

**★ Why this also EXPLAINS the class (A)/(B) split, and gives a sharper test than "fail closed".**

| class | silence | therefore |
|---|---|---|
| (A) declaration | would **ASSERT** something | must be answered, or the return REFUSES |
| (B) benefit claim | **FORGOES** something (New Colonial Ice: the burden to claim is the filer's) | silence is lawful |

So the test for any unknown is **"does the silence assert, or forgo?"** — which is why §G-9's fix is
legitimate (a dateless death forgoing the §63(f) box declines a benefit; it swears to nothing) and why a
printed `0` on an unasked line is categorically different.

**Bounds on the fix — from the same sentence, and easy to overshoot.** Intent is not ours to adjudicate,
so btctax has exactly three lawful moves: **collect** the testimony, **refuse** to produce the return, or
leave **genuinely blank**. It must never silently choose silence and present it as the filer's choice.
★ And it must equally never build the opposite thing — a heuristic that flags an omission as suspicious,
or any feature that opines on whether a blank is lawful. **Both directions are software deciding intent.**
This is a hard scope boundary, not a preference.

**Sketch, not a plan** (a real one needs its own spec): the emitter's money type grows a "not stated"
state that survives to the AcroForm write, computations may not manufacture a stated zero from unstated
inputs, and each line records which of the three lawful moves it takes and why. The per-line decision is
then a reviewable fact instead of an accident of `Decimal::default()`.

### G-10 — ~~`xtask cite-check` verifies a quotation EXISTS in the manual, not that it is at the cited line~~ **LARGELY CLOSED same day**

**Owning phase: B3 T2** (the conformance KAT is being written there anyway, against the same extracts).
Filed 2026-07-29 with the tool itself.

`cargo run -p xtask -- cite-check` checks every quoted span in the Schedule 1-A spec and plan verbatim
against the committed IRS text-layer extracts (34/34 today). ★ **Its residual gap was found by mutation,
not by reasoning:** changing S-1's line-11 quotation from *"decrease … to the next lower whole number"* to
*"increase … to the next higher whole number"* — inverting the rounding direction, the most dangerous
single fact in this form — **survives**, because line 28 genuinely says that. The checker asks "is this
the form's words?", never "are these THIS line's words?".

**★★ CLOSED for the class that matters, and NOT the way this entry proposed.** The filed fix was "parse
the attribution and require the span to appear in that line's region of the extract" — more citation
checking. The better answer, from the user: **floor and ceiling are a DECISION the code makes, so derive
it from the printed line instead of checking prose about it.**

`tables.rs::schedule_1a_conformance` now reads each phase-out's direction *off the form* — a
`printed_line(label)` reader over the in-crate extract, parsing "decrease … to the next lower whole
number" ⇒ `Floor` and "increase … to the next higher" ⇒ `Ceil` (and panicking rather than defaulting if it
can read neither) — and asserts it equals what `schedule_1a_params` assigned. **Neither side can drift
alone:** editing the params reds it, and so does editing the extract.

★ **The same reader closes a second, larger class for free: the CROSS-REFERENCE.** Each divide line is
asserted to divide *its own* excess line ("Divide line 27 by $1,000" for Part IV). That is precisely the
Form 6251 line-33 defect — *"Subtract line 32 from line 12"* where the form said **line 22**, which
inflated a tentative minimum tax by $200,000 and which CLAUDE.md records as uncatchable by review. It is
now a test.

**Mutation-verified from both directions**, including the exact mutation that SURVIVED `cite-check`:
moving line 28's sentence onto line 11 (2 red), setting the tips params to `Ceil` (8), `printed_line`
silently returning empty so everything would pass vacuously (6), and line 28 divide-referencing line 25
(2).

**★★ WHAT REMAINS IS BIGGER THAN THIS ENTRY FIRST SAID, AND IT IS ON THE CRITICAL PATH.** I closed the
misattribution class and then wrote that the residue was "narrow and off the critical path: a line whose
text encodes no decision the code makes … nothing computes from it, so it can't produce a wrong number."
**That is wrong three ways**, and the user caught it:

1. **"No decision the code makes" is time-indexed.** T2-T4 do not exist yet, so *most* of the form
   currently encodes no decision — including the Part II occupation Caution and the Part IV loan
   conditions. The claim is true only because the code is absent, and decays as the code grows.
2. ★★ **"Encodes no decision the code makes" is the SIGNATURE OF AN OMITTED REQUIREMENT.** Both r1
   Criticals were exactly that: Part IV's eligibility conditions encoded no decision in the plan, and
   *that was the bug*. CLAUDE.md states the same thing historically — every defect in the AMT sequence
   was a line that was never typed in. The category I called harmless is where the defects live.
3. **The design document is an INPUT to the code, so "nothing computes from it" is false.** A Caution
   misquoted in a spec becomes a wrong prompt in T3, a wrong eligibility gate, a wrong number — and
   SPEC §R-2 says a wrong prompt is a wrong return that *every test passes*, because the filer's answer
   is taken as truth. See the answered-ness invariant.

**And a measured hole in the fix itself.** `each_phase_out_rounds_the_way_its_own_printed_line_says_to`
iterates a **hand-written** list of three parts, so removing Part III from that list reds **nothing**, and
only **6 of the form's 48 labels** are examined by any conformance test. Same failure shape as an
oracle excuse list keyed by vector name: state the mechanism, let it enumerate, never hand-list the cases
you happened to think of.

**Therefore the real requirement, and it belongs to T2's KAT:** enumerate all 48 labels **from the
extract** and require every one to be ACCOUNTED FOR — either mapped to a field/decision, or explicitly
recorded as carrying no decision **with a reason**. Unaccounted-for must fail. A reader that cannot
distinguish *"this line encodes no decision"* from *"we forgot this line"* is not a conformance check.
Owning phase: **B3 T2** (unchanged).

### G-9a — do the §63(f) BLIND boxes have a death interaction?

**Owning phase: before TY2025 Part V** (the same gate G-9 was owned by). G-9 examined and fixed the
**aged** boxes. i1040gi's blind instruction reads *"blind at the end of 2024"* — which on its face a
person who died mid-year cannot satisfy, yet `Person::blind` is a plain tri-state with no death branch.
Adjudicate against the instruction text, not by analogy to G-9: the aged carve-out is stated
explicitly, and the absence of a matching sentence for blindness may itself be the answer (a decedent's
final return is generally filed as though the year ended at death). The machinery G-9 built —
`{taxpayer,spouse}_died_during_year` and `Person::date_of_death` — is already in place, so if the rule
does bite, the fix is a predicate change with no new input collection.

### G-19 — the crypto-slice-trio review residue (2026-07-30). Four Minors, two lenses, 0C/0I.

Two independent reviewers (tax-correctness + instrument-integrity) over commits `1a757f0..65270db`.
Verdict **0 Critical / 0 Important**; every one of 13 planted defects was killed by a test. Two Minors
were fixed inline (a stale `ScheduleBLines` doc comment that still promised "unanswered refuses" for a
box whose refusal this branch deleted; and the §1411 threshold boundary, documented but unpinned —
flipping `>` to `>=` had left the whole suite green). The four below are filed, not fixed.

**(G-19a) ★★ `niit_at_margin` reads the model's PARTIAL NII, and the false answer is the
UNDER-RESERVE one. Owning phase: whenever `TaxProfile` next gains an NII field — or sooner, on the
owner's call.** `compute.rs`'s `nii_with = qd + ST + LT − loss_deduction + crypto interest` omits
rents, royalties, taxable interest and non-qualified dividends, while `magi_excluding_crypto` **is**
the filer's complete non-crypto MAGI (its own doc says so). So modelled NII is a **lower bound**:
`nii_with >= 0` soundly implies real NII ≥ 0, but `nii_with < 0` does **not** imply real NII < 0.
Concrete miss: Single, OTI 250,000, MAGI 300,000 including $60,000 of rental income, net short-term
crypto loss $80,000 ⇒ modelled NII = −3,000 ⇒ `niit_at_margin = false` ⇒ the report prints
`§1411 0` when the filer's next long-term dollar really does owe 3.8%.
★ The pre-existing display made no forward §1411 claim at all; the new one prints an affirmative zero,
so this is a NEW affirmative statement, not an inherited silence. Display-only — no tax figure moves
(verified: the only readers of `MarginalRates` are `render.rs` and the TUI Tax tab; `optimize.rs`
carries the struct without reading it, `whatif.rs` computes its own NIIT delta).
★★ **The fix is a product judgment, not a bug fix, which is why it is filed and not applied.** The
narrow case is "MAGI over the threshold AND modelled NII < 0", where the honest answer is *unknown*.
Failing safe (always include §1411 once MAGI clears the threshold) over-states by 3.8 points on the
common crypto **loss** year with high income; a third display state is accurate but wordier. Either
edit touches `tax/types.rs` + `tax/compute.rs`, so it costs a second `frozen_guard` pin exception.

**(G-19b) ✅ FIXED 2026-07-30 — the class is gone, not just this instance.** `pdf::apply_writes` now
rejects any checkbox `on` value absent from that widget's own `/AP` `/N` keys, so the defect below
fails closed at the ONE chokepoint every checkbox on every form passes through — Schedule D line 17,
the QOF question, the 1040 digital-asset pair, Schedule B Part III, Schedule C's I/J, all of it.
Paired with the B1 planted-defect test `a_swapped_yes_no_map_fails_closed_instead_of_rendering_a_
blank_box` (kats.rs), which swaps the field names on all three revisions and asserts the refusal —
and asserts the UNSWAPPED map still fills, since a guard that rejects everything would pass the first
half. Observed RED with the checker disabled before landing. Original finding, kept for the record:

**★★ Schedule D line 17's Yes/No pair had NO map-independent oracle — a swapped map was invisible.** Every assertion reads the widget through
`pair.yes.field` / `pair.no.field`, so the map is both the thing under test and the test's own index.
Swap only the two `field =` values in `forms/2025/schedule_d.map.toml` `[line17]` and every filed 2025
Schedule D renders line 17 **blank** — `apply_writes` sets `/AS = /1` on a widget whose only on-state
is `/2`, so there is no `/AP /N /1` to draw — while `checkbox_on` reads the raw `/AS` back as
`Some("1")` and the KAT stays green. That is CLAUDE.md's second provenance row (*"nothing ever
populated it"*) laundered as a blank. ★ **The current maps are verified CORRECT** (`xtask dump-fields`:
2025 `c2_1[0]` on `"1"` at y 578–586 above `c2_1[1]` on `"2"` at 566–574; 2017 `"Yes"`/`"No"` likewise),
so this is a latent instrument gap, not a live defect. **The repo already solved exactly this for the
1040's digital-asset question** — `verify.rs::topmost_yes_no_pair`, *"derived from the blank PDF's
widget geometry + appearance states, never the map."* Schedule D line 17 wants the same, and the map
comment already records the geometry (*"Yes is the UPPER widget"*) that would supply it.
★ A cheaper partial: have `apply_writes` reject an `on` value absent from the widget's own
`button_on_states`. That kills the whole class, not just this instance.

**(G-19c) the crypto slice's line 17 inherits its missing lines 6/13/14. Owning phase: ownerless
residue — it is a scope boundary, not a defect.** The slice's lines 15/16 omit capital-loss carryovers
(6/14) and 1099-DIV capital-gain distributions (13), which the man page discloses. A filer with a
$20,000 long-term carryover and $5,000 of long-term BTC gain now gets line 17 = **Yes**, where their
real return has line 15 = −15,000 ⇒ line 16 a loss ⇒ *"skip lines 17 through 20."* Minor because the
slice's line 15/16 amounts and 1040 line 7a were already wrong for that filer — line 17 adds a cell
that inherits the error, not a new class of it — and because a filer with a carryover is outside the
slice's stated "crypto-only year" premise. Fixing it properly means the slice collecting the
carryover, i.e. becoming the full return.

**(G-19d) the FBAR skip advisory only reaches `report --tax-year`. Owning phase: ownerless residue.**
`advisories_for` has exactly one production caller (`cmd/tax.rs`), so `export-irs-pdf` writes
`schedule_b.pdf` with 7a = Yes and the FBAR pair blank while printing no FBAR notice on that
invocation. ★ The three load-bearing legs of the class-(B) reversal are all independently verified true
(no figure reads the box; a blank is no testimony; the Caution's penalty attaches to not filing FinCEN
114), so the non-refusal itself is justified — only the disclosure's REACH is narrower than
`LIMITATIONS.md` implies. The pre-existing `Advisory::FbarFinCen` has exactly the same reach, so this
is a whole-advisory-surface question, not an FBAR one.

### G-6a — TWO OTS DEFECTS, both ADJUDICATED 2026-07-29, neither a btctax defect

Found by the new line-by-line Form 6251 comparison. Recorded with the method used, because the two
needed *different* methods and that is the reusable part.

**1. OTS's 2024 solver carries the 2023 §55(d)(3) MFS constants. ADJUDICATED AGAINST THE FORM.**
`taxsolve_US_1040_2024.c:270-275` uses `831150.0` / `1084150.0` / `63250.0`; i6251 (2024) p.9 gives
`875,950` / `1,142,550` / `66,650`. No second oracle was needed or possible — **the authority answers
our exact input directly**: *"if the amount on line 4 is $895,950, enter $900,950 instead — the
additional $5,000 is 25% of $20,000."* That is vector V8 verbatim. btctax returns 900,950; OTS returns
912,150. Both engines agree on line 1 (881,350) and line 2a (14,600), so the base is identical and only
the kicker differs — ruling out our driver. Internally corroborated by OTS's own file: the *same
function* uses correct 2024 values for every exemption constant (`609350`, `952150`, `1218700`,
`1751900`, `232600`), so one block was updated for 2024 and the adjacent one was not.
Tax-Calculator cannot arbitrate this — it does not model the MFS kicker at all.

**2. OTS does not apply the §170(b)(1)(G) 60%-of-AGI ceiling on cash gifts. ADJUDICATED BY THE SECOND
ORACLE.** V2b: MFJ, $1,500,000 AGI, $1,000,000 cash gift to a public charity. btctax allows $900,000
(60% × AGI); OTS allows the full $1,000,000. Here a second oracle IS available and was used —
Tax-Calculator returns `c19700 = 900,000` and `c04800 = 600,000`, matching btctax to the dollar. So
btctax + taxcalc + the statute against OTS alone. This is a Schedule A defect, not an AMT one; it
reaches Form 6251 only through line 1.

**★ CHECKED AGAINST THE LATEST RELEASE — DO NOT REPORT EITHER. Both are already fixed upstream.**
Downloaded `OpenTaxSolver2025_23.06` (the current release) and read the same two sites in
`taxsolve_US_1040_2025.c`:

| defect | 2024 solver (v22.07, final for TY2024) | 2025 solver (v23.06, current) |
|---|---|---|
| §55(d)(3) MFS kicker | `831150 / 1084150 / 63250` — the **2023** triple | `900350 / 1174350 / 68500` — correct, inflation-adjusted |
| §170(b) 60% cash ceiling | `SchedA[11] = charityCC;` — **no ceiling at all** | `if (charityCC > 0.60*L[11]) {warn} charityCC = 0.60*L[11];` |

So both were real, both are fixed, and **filing them would waste a maintainer's time on a release line
that is closed** (v22.07 is the last TY2024 build). This is exactly the check that PSLmodels#3108 taught
us to run BEFORE filing rather than after.

**What it means for us instead:** btctax is corroborated on both points by OTS's own later release, and
our TY2024 oracle has two KNOWN DEFECT CLASSES. Any sweep that compares against OTS on TY2024 must pin
them as known-defect (the harness already has that concept — `known_defect` in `verdict_l16`) rather
than treat an OTS disagreement there as a btctax failure. Owning phase: **Tier 2**, with the corpus
widening below.

**[open] Widen the corpus.** `scripts/oracle/corpus.py` caps W-2 at $270,000 and LTCG at $20,000, so
the richest household reaches ~$410,000 of AMTI — it trips the screen but sits four times below the
$1,218,700 phase-out where AMT bites, so the differential sweep has never exercised a return that OWES
AMT. Owning phase: **Tier 2** with T9, paired with relaxing `gen_goldens.py:259`'s `c09600` half — and
now with the knowledge that the AMT half of that gate cannot be trusted for standard-deduction filers.

### G-5 — ★★★ CONSTELLATION AUDIT: transcribe IRS forms, never paraphrase them (2026-07-27)

**The rule is now normative in `CLAUDE.md`.** One field per numbered line, in the form's own numbering,
with the official instruction text verbatim as the doc comment. A derived/closed form is allowed only
with a written equivalence proof naming the branch where it breaks **and** a KAT pinning that branch.

**Why this is a whole-codebase concern, not an AMT concern.** Every defect in the 2026-07-27 AMT
sequence — one shipped in v0.6.0–v0.13.0, five more caught in review — was a line never typed in, never
a hard tax question: the screening worksheet compressed to `AGI − QBI` (and Schedule A line **7**
conflated with line **17**); Form 6251 line 2b dropped; the MFS line-4 kicker dropped; three review
rounds spent on a Part III question line 20 answers in one sentence; "attach when AMT > 0" instead of
Who Must File condition 1. **Compression always looks like good engineering.** It is where the bugs
live, because the dropped term is invisible once the lines are gone — and two of these compressions
carried confident equivalence comments that were simply wrong.

**Audit method.** For every module implementing an IRS form, schedule, or **worksheet**, classify:
`TRANSCRIBED` (line-per-line, instruction text present) · `COMPRESSED-PROVEN` (closed form + written
equivalence proof + a KAT on the breaking branch) · `COMPRESSED-UNPROVEN` (**a finding**). Worksheets
count — the AMT screen and the QDCGT worksheet are both worksheets and both were compressed.

**Prime suspects, from a first pass (line-reference density vs. compression language):**

- **`tax/se.rs` — strongest signal, verified.** Implements Schedule SE with **zero** line references,
  written entirely in statute terms (§1402(a), §1401, the 92.35% factor). May well be arithmetically
  right; it is **unauditable against the form**, which is the risk. Highest priority.
- **`tax/method.rs::qdcgt_line16`** — the Qualified Dividends & Capital Gain Tax Worksheet as a closed
  form (`l23 = worksheet_tax(bottom) + split.tax  // L22 + (L18 + L21)`), whose own doc admits "two
  locked subtleties". This worksheet is *also* what Form 6251 Part III lines 20/27 read from, so an
  error here propagates into AMT.
- **`tax/amt.rs`** — the known case. Partially fixed in `fix/amt-screen-line2`; the screening worksheet
  is still evaluated as a reduction rather than transcribed, and still lacks the MFS line-4 kicker.
- **`tax/compute.rs::preferential_tax`** — the §1(h) band primitive shared by the regular return and
  (soon) 6251 Part III. Rounds once to cents where the forms round per line.
- **`tax/qbi.rs`** — first pass flagged it, spot-check **cleared** it: the "simplif" hits are Form 8995's
  own name ("the simplified §199A deduction"), not compression language.
- **`tax/other_taxes.rs`** (Forms 8959/8960), **`tax/charitable.rs`** (§170 limits + carryover
  worksheets), **`tax/return_1040.rs`** — check.

**The exemplar to follow:** `tax/printed.rs` — 184 line references, zero compression language. The
printed chain already does exactly what this rule asks; the *computation* chain is where the
paraphrasing lives.

**Owning phase:** its own cycle, after the Form 6251 work (`design/amt-form6251/PLAN.md`) lands — that
build is the rule's first application and will establish the pattern. Do not batch this into another
feature; a wrong finding here is a wrong filed number.

- **[open] Full-return computation is TY2024-only.** `report --tax-year 2025` refuses with "full-return
  computation is not supported for 2025 in this version (v1 supports TY2024)" even though `SUPPORTED_YEARS`
  bundles 2025 forms and the crypto-slice path fills them. Worth a roadmap decision, not just a refusal.

### G-8 — v0.14.0 release residue (2026-07-29, shipped)

Filed from the pre-release verification, which returned **NO-GO on five blocking items** — all folded
before the tag (see `chore(release): v0.14.0`). What remains:

- **[open → Tier 2] Rename `RefuseReason::AmtScreenTriggered`** (e.g. `AmtFormMustBeAttached`). The name
  is now a misnomer: the trigger is Form 6251 line 7 > line 10, not the screening worksheet. Deferred
  deliberately — renaming a public enum variant reopens the cross-crate exhaustive-match blast radius
  that four review rounds closed, and Tier 2 is already breaking, so the rename is free there.
  `return_refuse.rs`; call sites `return_1040.rs`, `btctax-input-form/src/attribute.rs`.
- **[DONE 2026-07-29]** Lockstep guard added —
  `repo_hygiene::every_intra_workspace_dependency_pins_the_current_version`. ★ The rationale NARROWED
  on contact: cargo itself rejects a *lowered* exact pin, so the guard's real value is catching a LOOSE
  requirement (`>=0.13`), which resolves locally and publishes wrong. Original text: A test asserting every
  inter-crate dependency `version` equals `CARGO_PKG_VERSION`. **A missed pin does NOT fail the
  publish** — crates.io would accept `btctax-cli 0.14.0 → btctax-core 0.13.0`, silently re-shipping the
  bug the release fixes. This release's bump was verified by hand (36 literals, all btctax).
- **[PARTLY DONE 2026-07-29]** The specific class is guarded by
  `repo_hygiene::no_published_crate_includes_a_file_outside_its_own_root`. The general form (unpack each
  `.crate` and `cargo test --no-run`) remains OPEN. Original text: Nothing in CI or the Makefile
  runs it, which is exactly why the escaping-`include_str!` reached a release candidate. The permanent
  guard added this release (`repo_hygiene::no_published_crate_includes_a_file_outside_its_own_root`)
  catches that specific class; unpacking each `.crate` and running `cargo test --no-run` would catch
  the general one.
- **[DONE 2026-07-29]** `make architecture` added; `bundles` now depends on `docs architecture
  examples`, so a stale PDF cannot reach a release. Verified by deleting all three and rebuilding.
  Original text:
  They are gitignored, built ad-hoc, and `make bundles` concatenates whatever is on disk — so v0.14.0's
  first bundle silently baked in a 10-day-old architecture PDF carrying text the release had just
  fixed. Give them a Makefile target so `make bundles` cannot ship stale content.
- **[open → ownerless residue] `cargo update -p spin`** off the release path, then re-green. `spin
  0.9.8` is yanked upstream, so `cargo package` warns and `cargo install --locked` resolves a yanked
  transitive dep. Deliberately NOT done during release prep — perturbing the dependency graph the green
  suite was measured against is not a release-day edit.
- **[CLOSED — owner ruling 2026-07-29] The crates.io token stays.** The owner declined revocation: the
  token is time-limited and expires on its own. Do NOT re-file this or raise it at the next release.

### G-7 — Tier-1 whole-branch review residue (2026-07-29, branch `fix/amt-screen-line2`)

The whole-branch review ruled **2 Critical / 1 Important**; all three, plus every Minor that was cheap
in-file, were folded before merge. What follows is the residue, each with an **owning phase**.

**Folded, recorded here only so the reasoning is greppable:**
- **C-1** every Qualifying-Surviving-Spouse full return panicked (`table.ltcg.get(&status).expect(…)`
  bypassed `ltcg_for`, which exists solely to map `Qss → Mfj`; the map has no `Qss` key and a live test
  elsewhere asserts it does not). Regression I introduced on this branch. Fixed + KAT + mutation-verified.
- **C-2** Form 6251 **line 2l (depreciation)** rode inside `ScheduleCInputs::expenses`, a flat total that
  by construction contains Schedule C Part II line 13. `amt.rs` listed depreciation among "inputs v1 never
  captures", which was **factually wrong** and was the sole cover for a live understatement channel.
  Closed with a third class-(A) declaration (`AmtDepreciationSameAsRegular`) on the line-2k pattern.
  **Scope note (verified against i6251 p.4-5 after the fold):** the reviewer's worked example — a miner
  who elected out of §168(k) bonus on 5-year rigs — does NOT itself carry a line-2l adjustment if the
  rigs were placed in service after 2015 ("It isn't subject to an AMT adjustment for depreciation if it
  was placed in service after 2015"). The channel is real but narrower than the example: chiefly
  200%-DB MACRS property placed in service from 1999-2015 with a recovery period long enough to still be
  running in 2024, **plus any tangible property placed in service before 1999** — i6251's must-refigure
  list carries "Tangible property placed in service after 1986 and before 1999" with NO method
  qualifier, so a pre-1999 asset refigures even when depreciated straight-line. The declaration stays;
  its PROMPT took THREE tries and the failures alternated direction: v1 refused every filer who owns
  equipment (fail-closed, merely bricking); v2 "fixed" that by asserting an UNCONDITIONAL straight-line
  exemption, letting a filer with a 1990s building answer "yes" truthfully and omit a required add-back;
  v3 still dropped i6251's parenthetical "(other than section 1250 property)" from the 150%-DB
  exemption, so post-1998 land improvements — on the MUST-refigure list — were also steered to "yes".
  Both v2 and v3 were UNDERSTATEMENTS, the direction never permitted.

  **The fix was structural, not verbal.** Enumerating NO-triggers with a broad "otherwise yes" fallback
  makes every omission an understatement; enumerating YES-conditions with an "otherwise no" fallback
  makes every omission an over-refusal. The prompt now does the latter and says "if you are unsure,
  answer NO" outright, and the doc comment on `amt_depreciation_question_live` records all three
  failures with the instruction text grounding each permitted YES. **Adding a missing exemption later is
  a safe edit; widening the fallback is not.**
- **I-1** the Who-Must-File gate was nested inside the screening worksheet, so the branch's own line-2 fix
  was a net *safety reduction*, and its headline KAT was a **false pass**. Gate hoisted; the worksheet is
  now off every production path and survives as a swept cross-check.

**Open residue:**
- **[open → Tier 2]** `Form6251` has no fields for lines **2c–2t** (18 numbered adjustment lines), so the
  module's "every field is one numbered line" invariant holds in one direction only and `must_attach`'s
  condition-4 argument is prose resting on an input-surface audit. Tier 2 files the form and must give
  them real fields. Recorded in the module doc.
- **[open → Tier 2]** `AmtParams`'s seven Tier-1 fields are hand-copied literals in **8 local test
  constructors**; none routes through `ty2024_full_return()`, so adapter drift between the bundled table
  and the test fixtures is untested. A single shared fixture would close it.
- **[open → Tier 2]** `f6251.pdf` is the only bundled form with no `.map.toml`, no `include_bytes!` and no
  consumer — ~101 KiB shipped in a published crate for a form Tier 1 cannot file. Either wire it in Tier 2
  or move it under `design/` until then.
- **[open → ownerless residue]** The §1(h) 15/20/25% rates are inline literals in `form6251.rs` while
  26/28% moved into `AmtParams`, and `amt.rs`'s screen still hardcodes `0.25`/`0.26` that `compute_6251`
  reads from params — two sources for the same statutory rates. `PART_III.md`'s claim that "the only
  production literal is `tax_tables.rs:141`" is false as written.
- **[DONE 2026-07-29]** Both fixed and mutation-verified: `v9_must_attach_while_the_amt_is_zero` now
  calls `compute_6251`/`must_attach()` via a new `compute_vector` helper instead of reading the fixture
  back, and `line40_min_is_a_proved_no_op_for_this_input_class` sweeps the MFS §55(b)(1) schedule as
  well as the general one. Original text: Two vector tests assert less than their names suggest:
  `v9_must_attach_while_the_amt_is_zero` never calls `compute_6251`/`must_attach` (it checks the
  *fixture's* self-consistency; the real coverage is the vector loop), and
  `line40_min_is_a_proved_no_op_for_this_input_class` sweeps only the general 26/28% schedule, never the
  MFS one this branch added.
- **[DONE 2026-07-29]** `oracle_diff::table_l16` now uses `ordinary_for`/`ltcg_for` with a Qss
  regression test; no raw index remains in the workspace. Original text: `oracle_diff.rs:60-67`
  raw-indexes `table.ordinary` / `table.ltcg`
  instead of the `_for` accessors, so it panics for `Qss`/`HoH`/`Mfs` — the same shape as C-1, but
  pre-existing and confined to the oracle harness and golden tests (never a filing path). Fix it before
  any four-status corpus reaches `table_l16`.
- **[open → ownerless residue]** `a_cleared_screen_never_hides_a_must_attach_return` sweeps **Single and
  Mfj only**, because `return_1040.rs`'s test-local `real_2024_table()` carries schedules for exactly
  those two statuses (the test asserts that, so widening the table is a loud reminder). MFS/HoH rest on
  the `form6251.rs` vector KATs. A four-status fixture would let the sweep cover the §55(d)(3) MFS kicker
  end-to-end.

---

## ⚠★ SHIPPED BUG — Form 8949 uses pre-2025 boxes (C/F) for TY2025 digital assets (found 2026-07-20)

**Surfaced by the conservative-filing SPEC tax review; affects the RELEASED product (v0.7.0 on `main`).** The
2025 Form 8949 added **digital-asset-specific boxes** — Part I **G** (1099-DA, basis reported) / **H**
(1099-DA, basis not reported) / **I** (no 1099-DA); Part II **J/K/L** analogues — and the 2025 i8949 says
expressly *"Do not use box F to report long-term digital asset transactions"* (use J/K/L; G/H/I for
short-term). btctax's `Form8949Box` enum has only **C and F** (`crates/btctax-core/src/forms.rs:34-40`) and
emits them for ALL digital-asset rows → **non-compliant for TY2025 filings** (the forms a user files for
2025). **Fix (its own project, owner deferred it behind the conservative-filing SPEC 2026-07-20):**
year-aware box selection — C/F stay correct for **pre-2025** tax years; **G–L from TY2025**, derived from
(term, whether a 1099-DA was received + basis-reported, custody). KEEP the existing `box_needs_review`
"never fabricate a 1099 claim" semantics (don't auto-assert a received-1099-DA from custody alone — a
foreign/non-filing-exchange disposal marked K would fabricate a received-form claim). TDD + tax-lens review.
The conservative-filing feature's compliant 8949 output DEPENDS on this fix. — OPEN, **high priority**.
Verify against the 2025 i8949 primary source before implementing. — owner-scheduled security/compliance work.

---

## ★ POLISH BATCH — DONE (2026-07-20; the "5 that matter + cheap hardening" from the OPEN INDEX §B/§C)

Each TDD + mutation-verified (cp-backup/restore experiments, never `git checkout`):
- **P2-d (data integrity) — DONE.** `input_form_store.rs` commit/park/discard route through a new
  `mutate_and_save` helper that restores the pre-write snapshot on ANY error (mutate OR save) — previously
  only a `save()` failure restored, leaving the long-lived in-memory Session partially mutated on a mid-write
  `set`/`delete` error. KAT `mutate_and_save_rolls_back_the_in_memory_write_on_a_mutate_error` (RED without
  the restore).
- **P2-b — DONE.** `draft_exists` distinguishes `QueryReturnedNoRows` (→ false) from a real DB error (→
  propagate); `.is_ok()` swallowed the latter (parity with `parked_flag`).
- **P3-e (data loss) — DONE.** `FieldBuffer::seed` (grows cap to `FIELD_CAP.max(len)`) preserves an
  already-accepted stored value; `begin_edit` uses it instead of `set`, which capped at 64 and silently
  truncated a CLI/imported long Text value on TUI re-edit. KAT
  `field_buffer_seed_preserves_a_value_longer_than_field_cap`.
- **(m) — DONE.** `apply.rs` NI-2 first-edit arm now calls `guard_arity` (parity with `apply_to`; a depth-0
  `set` otherwise ignored a junk addr and materialized). KAT `m_first_edit_rejects_a_malformed_arity_addr`.
- **(l) — DONE.** The coverage KAT now asserts every `EXEMPT_LEAVES`/`EXEMPT_PREFIXES` entry is LIVE (matches
  ≥1 fixture leaf) — a stale entry silently over-exempts. Mutation-verified (fires on a dead prefix).
- **Task-2(d) — DONE.** `edit_roundtrips_through_json` broadened from Money-only to all 8 `FieldValue` kinds
  across `SetField` + `ClearField` (the four structural `Edit` variants are trivial derives, not re-exercised).
- **ProRata note — DONE (minimum-honest).** The safe-harbor-allocate preview states plainly that ProRata is
  NOT auto-computed (you attest the same per-wallet actuals; the tag sets only the timebar rule). Decision:
  attest-only — the auto pro-rata (Rev. Proc. 2024-28 "global allocation") split is a separate feature, and
  specific-unit (`ActualPosition`) dominates for tax minimization anyway.
- **cargo-deny (supply-chain CI) — PULLED from this batch (owner-directed 2026-07-20); re-filed as a
  standalone security project (§ below).** It was wired (`deny.toml` + a `supply-chain` CI job) but couldn't
  run in the sandbox; its FIRST real CI run RED'd on real, **pre-existing** findings (LGPL license + 6 RUSTSEC
  advisories — recorded below). Rather than half-ship a gate that reds `main`, the job + `deny.toml` were
  reverted; cargo-deny lands later with proper per-advisory triage + owner sign-off. **The guardrail worked:
  main was NOT merged red.**

**P3-d — DEFERRED (NOT a cheap fix).** `value_is_answered` treating `Money(0)`/`Bool(false)` as unanswered
(`draw_edit.rs:2590`, section-completeness glyph) is a facet of the answered-ness invariant: plain-value
fields (`payments.*`, `presidential_fund`) default to 0/false and their `get` ALWAYS returns `Some`, so a
naive "any set value = answered" would mark UNSET payments/checkboxes complete (worse). The correct fix is
structural (Option-ize those leaves, or a field-aware answered predicate) — answered-ness-refactor territory,
tracked with the answered-ness invariant. Not force-fixed.

**Review r1→r2 (`design/polish-batch/reviews/`):** r1 NOT-GREEN found the ProRata note UNRENDERABLE (a
225-char line clipped mid-word in a `Length(3)`/96-col modal) + the P3-e `begin_edit` wiring UNTESTED (the
untested-guard pattern again) — both folded (note split into short lines + `Length(6)` + wrap + an 80-col
render KAT; a `begin_edit` integration KAT, mutation-verified). r2 GREEN 0C/0I. **Nit-B (ownerless residue):**
the ProRata modal note still clips at terminal widths **≤63 cols** — a degenerate width where the modal's
~84-col residue table is unusable anyway, and strictly better than the pre-fix all-widths clip; revisit only
if a TUI min-size guard is ever added.

---

## ★ supply-chain security gate (cargo-deny) — DEFERRED as its own project (findings recorded 2026-07-20)

Wiring cargo-deny into CI red'd on its first real GitHub run — surfacing **pre-existing** dependency issues
(none introduced by any recent work). Owner directed pulling it out of the polish-batch merge; it returns as
a scoped security project with per-advisory triage + owner sign-off. Starting point (verified in CI run
`29770240057`, `advisories FAILED, licenses FAILED, bans ok, sources ok`):

**License:** `LGPL-2.0-or-later` — `buffered-reader` + `sequoia-openpgp` (the vault's PGP crypto, a deliberate
dep). Resolution: **allow LGPL** (copyleft-but-linkable; project is source-available). No code change.

**Advisories (6), with in-context triage (btctax = local CLI, trusted bundled PDF templates, passphrase/S2K
vault — NOT RSA, parses the user's own files):**
- **RUSTSEC-2026-0187** — `lopdf` stack-overflow parsing deeply-nested PDFs. **Low** (fills bundled TRUSTED
  AcroForms; no untrusted-PDF parse). Check for a patched lopdf.
- **RUSTSEC-2026-0194 / -0195** — `quick-xml` (via `calamine`/xlsx) XML DoS. **Low–moderate** (a malicious
  `.xlsx` import could resource-exhaust the LOCAL CLI; self-inflicted DoS, no RCE). Check calamine/quick-xml bump.
- **RUSTSEC-2023-0071** — `rsa` (via sequoia) Marvin timing key-recovery. **Low** (vault is passphrase/S2K,
  no RSA-decryption oracle exposed). No upstream fix — likely `ignore`-with-justification.
- **RUSTSEC-2025-0136** — AES key-unwrap underflow (PGP stack). **Low** (same posture); assess the exact crate.
- **RUSTSEC-2024-0436** — `paste` unmaintained (proc-macro). **No runtime risk**; transitive/unremovable →
  `ignore`-with-justification.

**Plan when resumed:** (1) allow LGPL; (2) update deps where a patched version exists (lopdf, quick-xml via
calamine); (3) `ignore`-with-justification the rest (rsa-Marvin not-applicable, paste unmaintained, AES-kw
assessed); (4) re-add the `supply-chain` job + `deny.toml`; (5) blocking once green. — OPEN, owned by a
**dedicated security-hardening project** (owner-scheduled).

---

## oracle-sweep — deferred hardening (2026-07-16)

- **(OS-14.2) Derive OTS's Form 8995 line 12 from OTS's OWN Schedule-D output — Minor, owned by
  post-oracle-sweep / future hardening.** `scripts/oracle/ots_direct.py::evaluate` **hand-feeds** Form 8995
  line 12 (`qbi_cap_l12 = round(net_capital_gain)`, the driver's own §1(h) figure — NOT derived by OTS)
  because OTS's 8995 solver reads a 1040 *output* file that carries a taxable income, not a net capital
  gain. Consequence: on the QBI-limited-by-net-capital-gain path OTS **cannot independently catch an error**
  in line 12 — if btctax's notion of net capital gain were wrong, the same wrong number is handed to OTS and
  it would agree. PSL Tax-Calculator (which derives line 12 from `p23250`/`p22250`/`e00650`) is the only
  fully independent witness there, so it is ONE witness, not two — the two-oracle claim is thinner on the
  QBI path than everywhere else. `qbi_cap_l12` is therefore emitted (T8) and asserted (T5/T6) as
  **single-witness/WEAK**, not advertised as an independent check. **The close:** derive OTS's line 12 from
  OTS's own Schedule-D solver output (the D-line proceeds/cost → §1(h) net capital gain) rather than the
  driver's hand-computed figure, restoring OTS as a genuinely independent second witness. Out of the
  oracle-sweep plan's scope (the plan ships the WEAK leaf as-is). — OPEN, owned by **post-oracle-sweep /
  future hardening**. — `scripts/oracle/ots_direct.py` `evaluate` (8995 L12 feed);
  `SPEC_oracle_sweep.md` §6.4 "L12 single-witness closure (r1 I-5)"; plan §6.1 table "8995 L12" row (§14.2
  closure).

---

## input-form PLAN 3 (TUI) — whole-branch review Minors (2026-07-15)

The Fable plan-3 whole-branch review (`design/input-form/reviews/WHOLE-BRANCH-P3-fable-r1.md`, 0C/4I) — the
4 Importants (I-1 snapshot re-projection, I-2 status invisible, I-3 close-on-failed-flush, I-4 `!` glyph +
screen-status) are folded in fix r1. Deferred Minors/Nits (ownerless polish unless noted):

- **(P3-a)** `TaxInputsModalState.shadows` is production-dead (only tests read it; the summary embeds the
  warning) — drop the field or read it. — OPEN, ownerless.
- **(P3-b)** on persistent flush failure the idle tick retries a full vault re-encrypt every ~100ms — add a
  backoff / stop-retrying-until-next-edit. (Related to fix-r1 I-3.) — OPEN, ownerless.
- **(P3-c)** `reinstate_parked_full_return` labels any `Loaded::Draft` "the parked full return" even if
  `parked=false` (unreachable in-session under the exclusive lock) — tighten the label. — OPEN, ownerless.
- **(P3-d)** `value_is_answered` treats `Money(0)`/`Bool(false)` as unanswered, pinning the section glyph at
  `…` for a deliberately-zero field (cosmetic). — OPEN, ownerless.
- **(P3-e)** `seed_string` through the 64-byte `FieldBuffer` cap would silently truncate a longer
  externally-imported Text value on re-commit (v1 fields are short in practice). — OPEN, ownerless.

---

## input-form PLAN 2 (persistence) — whole-branch review carry-forwards (2026-07-15)

The Fable plan-2 whole-branch review (`design/input-form/reviews/WHOLE-BRANCH-P2-fable-r1.md`, 0C/3I) —
the 3 Importants (I-1 load StaleNote, I-2 delete_draft pub(crate), I-3 commit per-year I-11 gate) are folded
in fix r1. Deferred Minors/Nits, each with owner:

- **(P2-a) stale-PARKED remedy chain is two-hop; discard must be reachable when `load` errors — owned by
  PLAN 3.** A stale parked draft surfaces `ParkedDraftBlocksWrite` from a committed-row writer, but both its
  named exits are unexecutable for the stale case ('use full return' → `load` refuses `StaleParkedDraft`
  first; 'discard' lives inside a form that may not open). PLAN 3 MUST make the 'X' discard-parked affordance
  reachable when `load` returns `StaleParkedDraft` (else a stale parked draft is undiscardable in-app).
  Optionally, `coherence_clear_or_refuse` could check `schema_version` and emit `StaleParkedDraft` directly.
  — OPEN, owned by **PLAN 3**. — input_form_store.rs load/coherence.
- **(P2-b) `draft_exists` swallows real DB errors — ownerless cleanup.** `.is_ok()` maps a genuine rusqlite
  failure to `false` (a hidden affordance) instead of `Err`; fix to `.optional()?` like `parked_flag` /
  `return_inputs::exists`. — OPEN, ownerless. — input_form_store.rs `draft_exists`.
- **(P2-c) `save_draft` silently overwrites/heals a STALE parked draft — ownerless hardening.** `parked_flag`
  ignores `schema_version`; a version check on the parked path would hold §6.3 by construction. Unreachable
  via the intended flow (`load` refuses first), so caller-convention-held today. — OPEN, ownerless.
- **(P2-d) post-snapshot/pre-save errors don't `restore` in commit/park/discard — ownerless hardening.**
  Disk is safe (save never ran), but restoring on ANY post-snapshot `Err` makes the fns fully transactional
  (double-fault territory otherwise). — OPEN, ownerless. — input_form_store.rs commit/park/discard.
- **(P2-Nits) ownerless polish:** park clean-state gate `== Some(false)` → `.is_some()` (closes the
  parked-overwrite corner for free); latch both errors when `restore` itself fails; `discard_parked_draft`
  refuse message is slightly off for the WIP case; a one-line comment on why `save_draft` omits snapshot/
  restore (plan-blessed, behaviorally right). — OPEN, ownerless.

---

## ✅ input-form engine (plan 1) — follow-up reconciliation after whole-branch review (2026-07-15)

The final Fable whole-branch review (`design/input-form/reviews/WHOLE-BRANCH-fable-r1.md`, 0C/7I) triaged the
deferred Minors below and flagged a per-phase-burndown violation (I-7). Reconciled:

- **RESOLVED in whole-branch fix r1 (commit `3bebaf8`):** **(e)** ClearField→None un-answer path — now a
  `Field.clear` closure per §5.7 M-6 (was the false-"§10" deferral = review I-1); **(b)** SecretView guarded
  `set_masked` constructor (review I-3); **(d)** coverage KAT get→set round-trip breadth (review I-6);
  **(k)** mask short-input full-mask (subsumed by I-3); **(f)** KAT `Some` seeds (verified done); **(g)**
  same-kind-`None`/clear boundary now pinned; **(i)** RowAddr arity guard (verified done, was already burned
  in Task 7); **(j)** Bool/Date kinds exercised by the KAT (verified done).
- **RESOLVED (manifest, commit follows):** **(c)** `btctax-input-form/Cargo.toml` now self-declares
  `rust_decimal serde-str` + `time serde-well-known` (no longer relies on feature unification).
- **STILL OPEN — genuinely ownerless, legitimately parked (not merge-blockers):** **(h)** near-duplicate
  `decl_tristate!`/`skippable_tristate!` macros; **(l)** coverage KAT does not assert its `EXEMPT` literals
  are live (cosmetic dead-literal hygiene); **(m)** the NI-2 first-edit arm (`apply`, `None` → the initial
  `SetField{FilingStatus}`) does not `guard_arity` the addr, so an over-long addr on the very first edit is
  accepted while the identical post-materialization edit is refused (re-review r2 Nit — no panic, no wrong
  value, just an inconsistency). Batch to a later cleanup pass.

The individual item entries below are retained for history; their status is superseded by this banner.

---

## input-form engine (plan 1) — Task-2 review Minors, filed with owning task (2026-07-14)

Task-2 (seam types) review was GREEN after the one Important — the `salt_use_sales_tax` duplicate
`FieldId` — was folded (dropped `FieldId::SalesTaxElection`, kept `SaSaltUseSalesTax`; per Fable-blessed
Option A; spec §5.8 amended with the "shown in ScheduleA above" dedup, mirroring `MortgageAllUsed`). Three
Minors deferred, each to its owning task:

- **(a) coverage-KAT assertion shape — owned by Task 5 (accessors + KAT).** When the coverage KAT is
  written, assert *"every `SkippableId` maps to exactly one `FieldId` somewhere in the form"*, NOT *"the
  SALT skippable appears in the Skippables section"* — the SALT election's FieldId is Schedule-A-owned
  (`SaSaltUseSalesTax`), so the Skippables section is blind ×2 + DOB ×2. — OPEN, owned by **Task 5**. —
  seam.rs `FieldId`; spec §5.6/§5.8.
- **(b) `SecretView::Set{masked}` has no type-level "never digits" guard — owned by Task 5 (Secret
  getters).** Today the masking invariant is convention-held (no constructor exists yet). When Task 5
  writes the `Secret` getters, give it a stronger guarantee (e.g. a private-constructor newtype) so a
  future caller cannot stuff raw digits into `masked`. Matches the answered-ness-by-convention pattern this
  codebase otherwise avoids. — OPEN, owned by **Task 5**. — seam.rs `SecretView`.
- **(c) `btctax-input-form/Cargo.toml` doesn't self-declare the serde features its wire types need — owned
  by Task 5 (or opportunistic).** `FieldValue::Money(Usd)`/`Date` derive `Serialize`/`Deserialize` but the
  manifest requests `rust_decimal = ["std"]` / `time = ["macros","parsing","formatting"]` — it compiles
  only because `btctax-core` enables `serde-str`/`serde-well-known` and Cargo unifies features across the
  shared graph (a real transitive guarantee, since the dep is unconditional). Declare them directly for
  manifest hygiene. Low risk. — OPEN, owned by **Task 5** (or any Cargo.toml touch). — Cargo.toml:14-15.
- **(d) `Edit`/`FieldValue` serde round-trip test covers only `Money` — owned by Task 5.** Broaden the
  round-trip KAT to `Text`/`Bool`/`TriState`/`Date`/`Choice`/`Secret`/`SecretEntry` and `SectionId`/
  `RowAddr` before the web renderer relies on the wire contract. Matches the brief's Step-1 test exactly,
  so not a Task-2 failure. — OPEN, owned by **Task 5**. — seam.rs `tests::edit_roundtrips_through_json`.

### Task-4 review carry-forwards (2026-07-15)

- **(e) `ClearField`→`None` clear path for registry-delegating TriState/Date fields — owned by Task 7.**
  Declarations/Skippables `Field.set` delegates to the core registry, whose setter is `fn(&mut RI, bool)`
  / `fn(&mut RI, Date)` and CANNOT express a clear — so `SetField{TriState(None)}`/`Date(None)` are
  (correctly) rejected `WrongKind`. Spec §5.8 M-6 requires `ClearField` on a `TriState`/`Date` to yield
  `None` (the answered-ness "true unasked" path). Task-4 review ruled this lands on **Task 7's `apply` +
  a DISTINCT clear path**, not on `Field.set` (routing clear through `set` is architecturally impossible
  for a delegating field). Recommended design: add a `clear: fn(&mut ReturnInputs, &RowAddr) ->
  Result<(),SetError>` to the `Field` struct (seam.rs), populated by every section builder
  (registries.rs + Task 5's tree); registry-delegating fields' `clear` writes `None` to the underlying
  `Option` leaf directly; plain fields clear to their M-6 empty; `apply` routes `ClearField` → `Field.clear`
  (Enum → `Immutable`). — OPEN, owned by **Task 7**. — registries.rs setters; spec §5.8 M-6; seam.rs `Field`.
- **(f) Task-6 round-trip KAT must seed `Some` for registry-delegating TriState/Date fields — owned by
  Task 6.** Because `None` can't be set through these delegating setters (see (e)), any get→set→get
  round-trip over a Declarations/Skippables field must use a `Some(bool)`/`Some(Date)` seed, not `None`. —
  OPEN, owned by **Task 6**. — the coverage/round-trip KAT.
- **(g) same-kind-`None` rejection is unpinned by a test — owned by Task 7.** The wrong-kind tests use
  CROSS-kind values (Text on a Decl, `Date(None)` on a YesNo field); the exact behavior (e) relies on —
  `TriState(None)` rejected on a `TriState` field, `Date(None)` on a `Date` field — has no assert. It is
  correct-by-construction (refutable `let … Some(b) = v else`), but an untested guard ([[untested-guard-pattern]]).
  When Task 7 builds the clear path, pin this boundary so a later refactor can't silently no-op same-kind
  `None`. — OPEN, owned by **Task 7**. — registries.rs:287,325.
- **(h) `decl_tristate!`/`skippable_tristate!` near-duplicate macros — ownerless polish, batch to end.**
  Differ only in registry path + accessor names (`get`/`set` vs `get_bool`/`set_bool`). Collapsing adds
  macro complexity across two registries; justifiable as written. Non-gating. — OPEN, ownerless. —
  registries.rs:275-312.

### Task-5 review carry-forwards (2026-07-15)

- **(i) malformed-arity `RowAddr` panics a row accessor — owned by Task 7 (apply layer). IMPORTANT there.**
  Row accessors index `a.0[0]`/`a.0[1]` directly; the row-beyond-length case fails safe (`.get()`→`None`/
  `NoSuchRow`), but an EMPTY or too-short `RowAddr` panics (index out of bounds). `Edit`/`RowAddr` are
  serde-deserialized from an untrusted web renderer (spec §4/§13 day-one seam consumer), so a malformed
  addr is wire-reachable → a panic-on-untrusted-input. Task 5's `a.0[0]` matches the brief's prescribed
  pattern and arity is an apply-layer contract, so it's not a Task-5 defect — but Task 7's `apply` MUST
  fail closed on malformed-arity addrs (validate arity per section depth → `ApplyError`, or have accessors
  read `a.0.get(n)`), NEVER panic. — OPEN, owned by **Task 7**; IMPORTANT at the apply layer. —
  sections.rs:113,116,508,588-600; seam.rs:10 (`RowAddr(pub Vec<usize>)`).
- **(j) Bool + Date field kinds are not round-trip-tested in Task-5 spot checks — owned by Task 6.** They
  rely on macro/pattern uniformity only (`TpPresidentialFund`/`SpPresidentialFund` Bool; `DepDob` Date).
  Task 6's exhaustive coverage KAT must exercise EVERY kind incl. Bool/Date. — OPEN, owned by **Task 6**. —
  sections.rs (presidential-fund, DepDob).
- **(k) `mask_secret` reveals the full value for a ≤4-char secret (takes last 4) — Nit, owned by Task 8.**
  Unreachable in practice (SSN=9 digits, IP PIN=6), and Task 8's `parse` enforces canonical length before
  a `SecretEntry` is built. Defensive full-mask on short input is cheap. Non-gating. — OPEN, owned by
  **Task 8** (or ownerless). — sections.rs:96-99 (`mask_secret`).

### Task-6 review carry-forward (2026-07-15)

- **(l) coverage KAT does not assert its `EXEMPT` literals are live — ownerless polish, batch to end.**
  `is_exempt` is a predicate; nothing asserts each `EXEMPT_LEAVES`/`EXEMPT_PREFIXES` entry matches ≥1
  realized fixture leaf, so a renamed/removed `sch1` leaf would leave a harmless DEAD literal. NOT a bite
  hole — the dangerous "deferred struct becomes in-scope ⇒ covered AND exempt" case is caught by the
  `covered_and_exempt` guard; only the cosmetic dead-literal case slips. Cheap fix:
  `assert!(EXEMPT_LEAVES.iter().all(|e| before.contains_key(*e)))` + each prefix matches ≥1 key. — OPEN,
  ownerless. — coverage.rs:201-211. *(The other two Task-6 Minors — fail-loud `addr_for`, theoretical
  array-collapse with no in-scope `Vec<scalar>` trigger — are accepted as-is; no action.)*

---

## P9 (form question registry) — deferred work, filed per `SPEC_form_questions.md` §5 step 12 (2026-07-14)

Two items P9 deliberately did not do, each filed with its OWNING PHASE per the per-phase follow-up
burndown rule (`/scratch/code/CLAUDE.md`, `STANDARD_WORKFLOW.md`) — burn down on that phase's schedule,
not "all at the end."

- **(a) `mortgage_interest_deductible` input — owned by P8 (input surface).** Capture the Pub. 936
  worksheet result as an input so a mixed-use-mortgage filer who HAS done the worksheet can enter its
  result and have Schedule A line 8a take it. §2.7 zeroes line 8a for a mixed-use mortgage (closing the
  false-statement — an unaffirmed box beside a full deduction); P8 recovers the money. Until then, a
  mixed-use filer forfeits the whole mortgage-interest deduction. — OPEN, owned by **P8**. — spec §2.7,
  §5 step 12(a).
- **(b) retire refuse-and-reimport — owned by the RELEASE GATE.** The §2.6 "refuse-and-reimport" policy
  for pre-P9 stored rows (`StaleReturnInputs`) is lawful ONLY while every stored row is test data. The
  moment a real return is entered, "re-import everything" stops being free — prior-year carryforwards
  (capital-loss and charitable carryforwards, the QBI REIT/PTP carryforward) are exactly what a real
  filer cannot reconstruct. The first real return must RETIRE refuse-and-reimport and require real schema
  migrations. — OPEN, owned by **the release gate**, not "later". — spec §2.6, §5 step 12(b).

---

## ✅ reconcile defaults: HIFO global default + long-term self-transfer-in — IMPLEMENTED on `feat/reconcile-defaults` (2026-07-05) — the auto-reconcile estimate is less punitive

Two user-mandated tax-policy default changes (revises [[self-transfer-completion-policy]]), per
`design/SPEC_reconcile_defaults.md` (R0-GREEN, 2 rounds):

- **Default lot method FIFO → HIFO** — GLOBAL (real + auto-reconcile), the fallback when no per-account/
  global `MethodElection` is on file. Four explicit `LotMethod::Fifo` default literals flipped to `Hifo`
  (`project/fold.rs:41` — the fold's only method-resolution default; `cli/config.rs` `CliConfig::default`;
  `project/mod.rs` `ProjectionConfig::default` + `in_force_methods` None arm). Stays `attested: false`
  (user still affirms HIFO per exchange). The enum `#[default]` (event.rs) is UNCHANGED — flipping it would
  silently rewrite pre-A.7 irrevocable `SafeHarborAllocation`s (`#[serde(default)] pre2025_method`); the
  FIFO relocation/fee mechanic (`pools.rs consume_fifo`) is also unchanged.
- **Self-transfer-in acquisition defaults to LONG-TERM** — when no `--acquired` is supplied,
  `fold.rs:1019` dates the acquisition **1 year + 1 day before receipt** (leap-safe
  `conventions::long_term_default_acquired`), so any later sale is long-term. Basis still defaults to $0.
  A new `SelfTransferInboundDefaultedAcquired` advisory discloses the assumption INDEPENDENT of basis
  (so `--basis` with no `--acquired` no longer silently backdates).

Both REDUCE the estimate (HIFO minimizes gain; long-term lowers the rate); basis stays $0 (conservative on
the amount). Test blast radius: 42 migrated (optimizer clusters pinned to explicit FIFO elections; the
inverted short-term KAT replaced) + new KATs; whole-suite GREEN. README "Realistic reconcile defaults" note
added; man pages regenerated.

---

## ✅ comprehensive price data + pseudo income-FMV + online updater (#41) — IMPLEMENTED on `feat/price-data-fmv` (2026-07-05) — real-vault income `FmvMissing` now RESOLVABLE

The bundled `btc_usd_daily_close.csv` was a 6-row STUB; the real-vault income events with no export FMV
therefore projected to Hard `FmvMissing` with no way to fill them offline. Three parts, per
`design/SPEC_price_data_and_pseudo_fmv.md` (R0-GREEN, 3 rounds):

- **A — real dataset.** Bundled the real daily closes (5,801 rows, 2010-07-17 → 2026-06-03; ISO,
  `Decimal` 2dp; sorted/deduped). NO attribution/NOTICE file — the prices are public market FACTS
  (Binance/CoinGecko-sourced), not copyrightable (user correction 2026-07-05); a one-line provenance note
  lives in the README. The stub-swap broke ~50 assertions across 3 crates; migrated via a **Session-level
  injectable price provider**
  (`Session::set_prices`) — plan KATs inject the old stub (zero recompute), free-function KATs recompute
  to real / move unpriced sentinels beyond the dataset.
- **B — pseudo income-FMV.** `IncomeRecord.pseudo` (set at both fold push sites) + a new
  `PseudoKind::PseudoFmv`: in pseudo mode an unresolved native `Income{Missing}` on a **priced** date is
  filled from the daily close (`ManualFmv` default, `[PSEUDO]`-flagged, approve→real, export-gated). NO
  price ⇒ stays `FmvMissing`. **This REVERSES `SPEC_pseudo_reconcile_mode.md`'s leave-uncleared decision**
  (contract comment updated). So the real-vault "27 income `FmvMissing`" are now clearable under pseudo
  mode wherever the bundled dataset covers the date — the M5 fixture (`income_fmv_missing_batch`) + the
  `income_fmv_27_clear_under_pseudo` KAT pin it.
- **C — separate `btctax-update-prices` binary.** `LayeredPrices` (cache-over-bundled, no `dirs`/network)
  in adapters; the NEW `crates/btctax-update-prices` is the ONLY crate linking an HTTP client (`ureq`
  rustls-tls + `dirs`). An xtask gate (`cargo run -p xtask -- check-isolation`) asserts ureq/rustls are
  ABSENT from every tax crate. **Add this step to CI** (alongside `cargo test`) so a stray network dep
  fails the build.

**OPEN (residual):** income `FmvMissing` on a date BEYOND the bundled range still needs the user to run
`btctax-update-prices` first (populating the cache) before pseudo mode can fill it — the "no price → run
`btctax-update-prices`" hint surfaces this. A per-value **cache-provenance marker** on auto-FMV disposal
proceeds (a cache-sourced price feeds the real proceeds path unflagged) is deferred — the bundled-only
projection remains the reproducible baseline. Whole-diff review + ship still pending.

## ✅ export-snapshot unresolved-Hard-blocker summary — SHIPPED (2026-07-05) — real-vault "silent empty forms" finding RESOLVED

The real-vault finding — `export-snapshot` silently wrote an EMPTY Form 8949 / zero Schedule D (and
empty projection CSVs) when unresolved Hard blockers made every year `NotComputable`, with NO warning
— is FIXED. `cmd::admin::export_snapshot` (`crates/btctax-cli/src/cmd/admin.rs`) now returns
`ExportReport { path, unresolved_hard }` (`#[derive(Debug, Clone)]`) instead of a bare `PathBuf`;
`unresolved_hard = blockers.filter(severity()==Hard).count()` (NO `compute_tax_year` call — any Hard
blocker gates every year, so the count alone drives the disclosure). The `ExportSnapshot` main.rs arm
(`main.rs:325`) prints `Exported …` as before, THEN — only when `unresolved_hard > 0` — an
`eprintln!` to STDERR: a `--tax-year` export names the year NOT COMPUTABLE + "Form 8949 / Schedule D
are INFORMATIONAL, not final"; a full export says "every affected year is NOT COMPUTABLE" + "the
exported figures are INFORMATIONAL, not final" ("figures", since no `--tax-year` writes projection
CSVs not the forms). Both say "Run `btctax verify`". It WARNS, does not refuse — export still writes
the files and exits 0; a clean (0 Hard) ledger prints no warning (stdout byte-identical to before).
The store method (`vault.rs:263`) stays `PathBuf`. Clap doc-comment note added at `cli.rs:92` +
`btctax-export-snapshot.1` regenerated (`cargo run -p xtask -- docs`). R0-GREEN spec (2 rounds, 0C/0I):
`design/SPEC_export_blocker_summary.md`. New KATs (`tests/export.rs`): lib `export_report_counts_only_hard`
/ `export_report_path_points_at_snapshot` / `export_still_writes_files_with_blockers`; binary
`export_with_hard_blockers_warns_on_stderr` (★ fault-inject verified RED) /
`export_full_no_year_warns_informational` / `export_clean_ledger_no_warning`. Automation that must GATE
on unresolved blockers checks `btctax verify` (exits non-zero), since export stays exit 0.

## ✅ TY2026 tax-table backfill — SHIPPED (2026-07-05) — 2027 DEFERRED (unpublished)

`ty2026()` in `BundledTaxTables` (`btctax-adapters/src/tax_tables.rs`), wired via
`by_year.insert(2026, ty2026())` in `load()`. Figures transcribed VERBATIM from the primary sources
(Rev. Proc. 2025-32 §4.01/§4.03/§4.42; OBBBA Pub. L. 119-21 §70106; SSA Fed. Reg. 2025-11-03) per the
R0-GREEN spec `design/SPEC_tax_tables_2026.md` (2 rounds, 0C/0I): 28 ordinary edges (4 statuses × 7 —
incl. the **HoH 32%/35% = $201,750/$256,200 trap**, distinct from Single's $201,775/$256,225; MFS 37% =
$384,350 = ½ MFJ), 4 LTCG pairs, gift annual $19,000, SS wage base $184,500, lifetime exclusion flat
$15,000,000 (OBBBA, not §1(f)). KATs per-status + fault-inject HoH + `ty2024_and_2025_tables_unchanged`.
**ARMS TY2026**: `table_for(2026)` flips `None → Some`, so a 2026 compute is now `Computed`, not
`NotComputable(TaxTableMissing)`. Owned regressions: `tax_report.rs carryforward_mismatch_advisory_rendered`
re-pointed 2026→2027 (the whole scenario, since the loss lives in the prior-year CSV); `optimize.rs`
timing-insight doc reworded (the real guard is the same-year check, not "2026 hits a missing table").

**Deferred (OPEN): TY2027 tables** — IRS/SSA publish those figures in fall 2026, after our data horizon;
do NOT fabricate. Backfill when published (mirror this cycle: verify vs the TY2027 Rev. Proc. + SSA).

---

## 🟡 pseudo-reconcile mode (auto-pseudo-reconcile sub-project 2) — IMPLEMENTED on `feat/pseudo-reconcile-mode`, AWAITING WHOLE-DIFF REVIEW (2026-07-04)

A reversible **mode** that fills DELIBERATELY-FICTIONAL default decisions at PROJECTION time (NEVER
persisted) to clear the Hard **classification** blockers, producing a loudly-flagged `[PSEUDO]` on-screen
estimate the user corrects toward truth. R0-GREEN spec `design/SPEC_pseudo_reconcile_mode.md` (3 rounds,
0C/0I). Tasks **T1–T6 all implemented + committed** on branch `feat/pseudo-reconcile-mode` (base `main`
`514875b`); left for the human whole-diff review + merge (NOT merged).

- **Defaults (only where no real decision):** `UnknownBasisInbound`→`ClassifyInbound(SelfTransferMine $0)`;
  `Unclassified` (determinable-inbound)→`ClassifyRaw` zero-value placeholder (the row carries no structured
  amount, so pseudo fabricates no holdings; wallet-less Unclassified LEFT SURFACED); `TransferOut`→left as
  `PendingOut` (already non-taxable); `ImportConflict`→accept-first `SupersedeImport`. `DecisionConflict`,
  `UncoveredDisposal`, native-Income `FmvMissing`, `TaxTableMissing` are NOT cleared (stay surfaced).
  CLI placeholder tax profile at `report_tax_year` clears `TaxProfileMissing` ONLY. A tax TOTAL computes
  only at 0 Hard blockers of ANY kind (pseudo `$0`-basis Sells make it HIGH, not zero).
- **Tax-safety (all fault-inject KAT'd):** synthetics NEVER persisted by projection (only `approve` writes);
  real supersedes pseudo; the ★ headline guard — `[PSEUDO]` is on-screen (incl. the C1 basis-taint case: a
  REAL Sell on a pseudo `$0` lot is flagged) and PROVABLY ABSENT from every export CSV/form (a dedicated
  `pseudo` bool the writers OMIT, never a `BasisSource` variant); mode-off byte-identical; determinism.
- **Surfaces:** `reconcile pseudo on|off|approve` (own-loop bulk-approve, `--kind/--wallet/--year` filter);
  `[PSEUDO]` on report/TUI rows + a `PseudoReconcileActive` advisory in `verify`; `export-snapshot` while
  pseudo-active is **REPLACED by sub-3 (attestation gate — both the CLI and the btctax-tui viewer)** — the
  interim [I3] blanket refusal is gone; btctax-tui-edit loud banner + `P` approve flow. Man pages regenerated
  (`make docs`).

**sub-project 3 — attestation export gate: IMPLEMENTED on `feat/attest-export-gate` (base `main` `afb0807`),
AWAITING WHOLE-DIFF REVIEW (not merged).** Producing `export-snapshot` / any form/data file while the ledger
is pseudo-active requires the exact phrase **`I attest this is true`** (trimmed, case-sensitive) — a
fully-real ledger exports with no prompt. Both form-writing paths are gated [R0-C1]: the CLI
(`cmd::admin::export_snapshot` + `--attest`/TTY prompt) AND the btctax-tui viewer `e` export (typed-word
modal). Pure `btctax_cli::require_attestation` exact-compare helper + `ATTEST_PHRASE` const (both `pub`,
shared by the viewer); errors `AttestationRequired`/`AttestationFailed` name the phrase. Output stays clean
(no markers added). R0-GREEN spec `design/SPEC_attest_export_gate.md` (2 rounds, 0C/0I). **This closes the
auto-pseudo-reconcile program.**

---

## ✅ crate publishing — PUBLISHED to crates.io + repo made PUBLIC (2026-07-04)

**All 7 crates are LIVE on crates.io at v0.1.0** — `btctax` (name-reservation crate → `btctax-cli`),
`btctax-core`, `btctax-store`, `btctax-adapters`, `btctax-cli`, `btctax-tui`, `btctax-tui-edit`
(`xtask` stays `publish=false`). `cargo install btctax-cli` works. The **GitHub repo `bg002h/bitcoin_tax` is now
PUBLIC** (full git-history audited clean first — no keys/tokens/vault/tax data ever committed; `main` pushed to
origin `5662c3c`). Published with a user-supplied temporary `publish-new`-scoped token via `CARGO_REGISTRY_TOKEN`
(not persisted; the stored `~/.cargo` token lacked publish perms). Hit the new-crate 5-burst limit → the 7th
(`btctax-tui-edit`) 429'd and was retried after the ~10-min window. **v0.1.0 is permanently burned — future
releases are 0.1.1+.** See memory [[crate-publishing-state]].

_(historical prep record below.)_

Publish-ready, merged to main
(`3492023`): crates.io metadata (description per crate, shared repository/homepage/keywords in
`[workspace.package]`, per-crate categories — libs `finance`, bins `command-line-utilities`+`finance`) +
`version = "0.1.0"` on all 14 internal path deps. **Coordinated `cargo publish --dry-run --workspace` PASSES**
(6 crates packaged + build-verified in topo order: core→store→adapters→cli→tui→tui-edit; `xtask` is
`publish=false`). Safety audited twice — no vault/key/tax data ships (only the public `btc_usd_daily_close.csv`).
R0-GREEN 2 rounds + whole-diff 0C/0I. Reviews: `reviews/{R0-spec-crate-publishing-round-{1,2},
whole-branch-review-crate-publishing-round-1}.md`.

**TO PUBLISH (when the user says go):** from a CLEAN committed `main` (no `--allow-dirty`, token already in
`~/.cargo/credentials.toml`), run `cargo publish --workspace`. Expect a **429 on the 6th crate**
(`btctax-tui-edit`) — crates.io's new-crate 5-burst limit — wait ~10 min and re-run (`cargo publish
--workspace` or `-p btctax-tui-edit`); safe + resumable.
**USER DECISION — reserve the bare `btctax` name:** the user said YES. When publishing, ALSO publish a minimal
`btctax` v0.1.0 name-reservation crate (design: a lib-only placeholder whose description/doc points to
`btctax-cli`, `cargo install btctax-cli`; no internal deps so it can publish independently). This makes 7 new
crates → the rate-limit retry applies to the last 2. **Irreversibility reminders for the go:** names + v0.1.0
permanent; source becomes public (regardless of repo privacy); MIT-OR-Unlicense = freely reusable.

---

## ✅ README (install + verified tutorial) — SHIPPED (2026-07-04)

Greenfield end-user `README.md`: what btctax is, install-from-source (`cargo install --path crates/*`; crates.io
deferred to the publishing task), and a hands-on tutorial (init → import → verify → reconcile → tax-profile →
report → export-snapshot) with a synthetic Coinbase CSV. R0-GREEN 2 rounds (round 2 EXECUTED the tutorial
verbatim); whole-diff re-ran all 6 steps against the built binary — every command works with the promised
outputs/exit codes. Notable review catches: `report --tax-year` needs a `tax-profile` step first; the
`export-snapshot` CSVs are NOT git-ignored (warn: export outside the repo); the reconcile event-ref contains
`|` and must be single-quoted. Merge `926b51a`. Reviews:
`reviews/{R0-spec-readme-round-{1,2},whole-branch-review-readme-round-1}.md`.

---

## ✅ cross-platform CI (macOS + Windows test matrix, NFR8) — SHIPPED (2026-07-04)

Matrixed the CI `test` job over ubuntu/macos/windows (`fail-fast:false`) + `.gitattributes` (`* text=auto
eol=lf`) so the store's `cfg`-gated OS primitives (fs2 locks, mlock/VirtualLock, atomic rename, owner-only
perms) are EXECUTED on every OS, not just compile-checked. The three `test (<os>)` legs are the required
checks (user sets branch protection). Merge `b0b5676`; all 3 legs green (run 28707743830); Linux suite 1095/0.
**Resolves the "Cross-platform validation … executed under per-OS CI (set up later) — OPEN (CI)" items below
(NFR8 / crypto-rust) and exercises the M-3 owner-only-perms sinks on Windows.**
The matrix immediately caught **3 real bugs** invisible on any single dev machine, each root-caused +
Linux-reproduced + CI-verified:
1. `.gitignore` `*-snapshot.*` silently un-committed `docs/man/btctax-export-snapshot.1` (xtask docs KATs fail
   on a clean checkout) → `!docs/man/*.1` negation. **This was a latent binary-docs bug.**
2. `btctax` `STATUS_STACK_OVERFLOW` on Windows (1 MiB main stack) in classify-inbound-self-transfer → run the
   CLI on a 64 MiB worker thread (`crates/btctax-cli/src/main.rs`).
3. Windows `ERROR_LOCK_VIOLATION(33)` not recognized as lock contention (std doesn't normalize it to
   `WouldBlock` — the old `lock.rs` comment's assumption was wrong) → `is_contention()` matches raw codes
   32/33 under `cfg(windows)`. **The `fs2`→`fd-lock` swap note below is now moot for correctness** (contention
   is handled explicitly); fd-lock remains a maintenance-only consideration.
Reviews: `reviews/{R0-spec-cross-platform-ci-round-1,whole-branch-review-cross-platform-ci-round-1}.md`.

---

## ✅ binary documentation (man pages + PDFs + inline file-format docs) — SHIPPED (2026-07-04)

Man pages for all three binaries + PDFs + inline FILE-FORMAT docs. **Single source of truth:** the file-format
docs (format + text example) live in the clap doc-comments in `crates/btctax-cli/src/cli.rs` (the `Cli` was
extracted from `main.rs` to a lib module so the generator can reach `Cli::command()`), each with
`#[arg(verbatim_doc_comment)]` — they flow to BOTH `--help` AND the man page (via `clap_mangen`), so no drift.
**Layout:** git-style per-subcommand pages (`docs/man/btctax.1` + `btctax-<path>.1`, 40 total) — because
`clap_mangen` renders only ONE command's args per call, NOT subcommand args from a single root render.
**Generator:** `crates/xtask` (clap_mangen is generator-only — the shipped `btctax` gained no runtime dep).
**Documented formats** (not vault / not exchange-import): key-backup armor, export-snapshot CSV set
(`income.csv` etc., headers read from the `render.rs` writer), import-selections CSV, classify-raw JSON,
select-lots picks. **Regenerate:** `make docs` (man+PDF, deterministic `.1`); `make bundles` → one combined
PDF per binary (`docs/pdf/btctax-manual.pdf` + the 2 TUI manuals; PDFs git-ignored — gropdf embeds a
timestamp). R0-GREEN 2 rounds (r1 caught the clap_mangen single-root limitation); whole-diff 0C/0I (help KAT
fault-injection-confirmed load-bearing). **1095 tests.** Merge `04d27ce`. Reviews:
`reviews/{R0-spec-binary-documentation-round-{1,2},whole-branch-review-binary-documentation-round-1}.md`.

---

## ✅ frozen column totals (btctax-tui) — SHIPPED (2026-07-03) — PARKED ITEM 2 DONE → QUEUE CLEAR

Column totals as a FROZEN `Table::footer()` on the output tabs. **Disposals**: freeze the existing scrolling
TOTAL row + add Σ BTC (basis stays SUMMED — `Σ gain = Σ proceeds − Σ basis`). **Holdings**: Σ BTC +
**weighted-average cost $/BTC** (`round_cents((Σbasis×1e8)/Σsat)`, multiply-first ROUND_HALF_EVEN; `Σsat==0
→ —`). **Income**: Σ BTC + Σ FMV. **Height gate** (user req): shown only when the tab area ≥ 10 rows
(`MIN_ROWS_FOR_TOTALS`), else omitted so data keeps the space. **Forms deferred** (its ST/LT totals are
already the Schedule D summary — a footer would duplicate). `btctax-tui` only; the editor inherits via the
shared renderers; no core change. R0 GREEN (2 rounds; r1 caught the weighted-avg change breaking an existing
Holdings KAT + 2 more test-side issues); whole-diff 0C/0I (weighted-avg + height-gate fault-injections).
**1084 tests.** Reviews: `reviews/R0-spec-column-totals-round-{1,2}.md`,
`reviews/whole-branch-review-column-totals-round-1.md`.

**★★ QUEUE CLEAR (2026-07-03):** the 5-cycle bulk-reconcile program (extract → resolve-conflict → void →
inbound-income → outflow-reclassify) + both parked TUI-polish items (`?` help overlay, column totals) — ALL
shipped to `main`. No outstanding user-directed work.

---

## ✅ `?` help overlay (btctax-tui-edit) — SHIPPED (2026-07-03) — PARKED ITEM 1 DONE

A `?` shortcut opens a **full-keymap help overlay** in the Browse screen — same on every tab (the reconcile
action keys are global). `EditorApp.help_open` + a top-level modal gate in `handle_key` (`?`/`Esc`/`q`
close, all else swallowed, pre-empts the Browse quit arm) + `draw_help_overlay` (centered modal, grouped
Navigation/Reconcile/App, fits 80×24) + the footer now advertises `?: help` (R0-I1: the entry point must be
discoverable). Value: the ~20 action keys (incl. bulk `C/V/I/O`) had no on-screen hint. R0 GREEN (2 rounds);
whole-diff 0C/0I (modal-gate fault-injection; the `help_modal_swallows` KAT was strengthened to use `Tab`
after a fault-injection showed a snapshot-less `v` probe wasn't load-bearing). 6 KATs. **1078 tests.**
Reviews: `reviews/R0-spec-help-overlay-round-{1,2}.md`, `reviews/whole-branch-review-help-overlay-round-1.md`.
**Next parked item: 2 — frozen column totals.**

User-reported bug: `btctax import .../ReadOnly/*` → `gemini row 2: fractional satoshi in BTC amount
"0.0010216163"`. Gemini exports 10-dp internal-ledger artifacts (fee splits / interest / averaged fills —
8 of 825 BTC-Amount cells in the user's file are finer than a satoshi); `parse_btc_to_sat` REJECTED them
(`AdapterError::FractionalSat`), aborting the whole multi-file import on the first data row. **Fix
(user-approved): round BTC amounts to the NEAREST satoshi** (`Decimal::round()` = `MidpointNearestEven`,
matching `round_cents`) — normalizing an un-representable BTC QUANTITY to the satoshi grid (< 1 sat ≈
<$0.001 error). USD/tax VALUES are still parsed exactly (NFR5 intact); this is BTC quantity only. Removed
the now-unused `FractionalSat`; corrected the `parse.rs`/`read.rs` docs (the xlsx `Data::Float →
format!("{f}") → parse_btc_to_sat` read path is in scope; its ≤8-dp bound was wrong). `btctax-adapters`
only. R0 GREEN (2 rounds; round 1 caught the xlsx numeric-cell read-path gap); whole-diff review 0C/0I
(`.round()`→`.trunc()` fault-injection drove both the unit + the numeric-xlsx integration KATs RED).
**1006 workspace tests.** Reviews: `reviews/R0-spec-gemini-subsatoshi-round-round-{1,2}.md`,
`reviews/whole-branch-review-gemini-subsatoshi-round-round-1.md`. **The user's Gemini disposals (~42
sells) now import.**

---

## ✅✅ bulk-reclassify-outflow — SHIPPED (2026-07-03) — QUEUE ITEM 3, CYCLE 5 DONE → **PROGRAM COMPLETE**

The LAST cycle. Bulk reclassify pending outflows → `Dispose{Sell,Spend}` with auto-FMV as **ESTIMATED
proceeds** (TUI `O` / CLI `reconcile bulk-reclassify-outflow --kind sell|spend`). **Primary driver:** Spend
on goods/services — no price exists, so the FMV of the BTC that left is the correct+only valuation. The
estimate is flagged **persistently** via a `btctax-cli`-only `bulk_estimated_proceeds` side-table (keyed by
`transfer_out_event` == `Disposal.event`; **core UNCHANGED**) and shown as an **`[est]`** marker on the
Disposals tab + a Compliance advisory count. Tax-safety: #a `fmv_of==None` excluded (silent-fabricated-proceeds
defense); `estimated_gain = fmv − Σ fold-computed leg basis` (not double-counted); **clear-on-void** wired
into BOTH the TUI (`persist_void`/`persist_bulk_void`) AND CLI (`void`/`apply_bulk_void`) paths. Sell/Spend
only (Gift/Donate deferred — donee not uniform; §170 appraisal). R0 GREEN (2 rounds; r1 caught clear-on-void);
whole-diff 0C/0I — the CLI-void-clear parity gap folded + 4 tax-critical fault-injections. **1072 tests.**
Reviews: `reviews/R0-spec-bulk-reclassify-outflow-round-{1,2}.md`,
`reviews/whole-branch-review-bulk-reclassify-outflow-round-1.md`.

**★ QUEUE ITEM 3 — the 5-cycle bulk-reconcile-other-types program — is COMPLETE** (extract →
bulk-resolve-conflict → bulk-void → bulk-classify-inbound-income → bulk-reclassify-outflow). Next: the two
parked TUI-polish items (`?` help overlay, then column totals) — user-authorized 2026-07-03.

---

## ✅ bulk-classify-inbound-income — SHIPPED (2026-07-03) — QUEUE ITEM 3, CYCLE 4 DONE

Bulk classify many pending unknown-basis inbounds → `Income` (uniform `IncomeKind` {Mining/Staking/Interest/
Airdrop/Reward} + `business`, per-row auto-FMV) — TUI `I` / CLI `reconcile bulk-classify-inbound-income`.
Near-clone of the shipped bulk-sti (`B`) with the ONE tax-safety twist [#a]: **EXCLUDE `fmv_of == None`
rows** (missing daily-close price). A persisted `Income{fmv:None}` raises a Hard `FmvMissing` that gates the
year AND is unrecoverable without void+reclassify (a `ManualFmv` on a classified inbound is itself Hard
`DecisionConflict`); bulk-sti INCLUDES those rows ($0-basis needs no FMV), bulk-income must NOT. `plan.included`
carries a resolved `fmv: Usd`; the CLI apply uses its OWN append-loop (NOT the tui-edit `persist_bulk_decisions`
— dependency cycle, the Cycle-2 trap; R0-I1) with a defensive `let Some(fmv)=fmv_of(..) else continue` so
`Income{fmv:None}` is STRUCTURALLY unreachable. R0 GREEN (2 rounds; r1 caught the persist cycle); whole-diff
0C/0I (#a exclusion fault-injected + the defense-in-depth fold). **1044 workspace tests.** Reviews:
`reviews/R0-spec-bulk-classify-inbound-income-round-{1,2}.md`,
`reviews/whole-branch-review-bulk-classify-inbound-income-round-1.md`.
**Remaining: Cycle 5 bulk-reclassify-outflow (the last — highest value, estimated-proceeds Sells).**

---

## ✅ bulk-void — SHIPPED (2026-07-03) — QUEUE ITEM 3, CYCLE 3 DONE (the dangerous one)

Sweep-void many reconcile decisions at once (TUI `V` / CLI `reconcile bulk-void`). **Task 1** extracted the
voidable-candidate predicate to `btctax-core::voidable_decisions` (+ moved `is_revocable_payload` to
`btctax-core/src/void.rs`) so bulk == single-void on the **#7 tax-safety exclusion** — voiding an EFFECTIVE
`SafeHarborAllocation` fires a Hard `DecisionConflict` that gates the whole year; `!effective_alloc`
(SafeHarborAllocation with no timebar/unconservable blocker) is the sole defense, now one shared predicate
(no drift). `open_void_flow` re-pointed (zero-behavior; stale `resolve.rs:865-921` cite fixed). **Task 2**:
`Session::bulk_void_plan` + bespoke atomic `persist_bulk_void` (N `VoidDecisionEvent` appends + per-`LotSelection`
`optimize_attest::clear` inside ONE envelope, mid-batch rollback) + CLI dispatch derives targets from
`bulk_void_plan().rows` (NEVER raw `--ref` ids — the CLI-layer #7 defense) + TUI Tier-B blast-radius confirm
(non-revocable, NOT typed-word). Core relocation-only (no new variant, no serde break). R0 GREEN (2 rounds);
whole-diff review 0C/0I — **three tax-critical fault-injections** (drop #7 filter → 2 KATs RED; bypass
save_or_rollback → revert KAT RED; drop attestation clear → clear KAT RED). **1032 workspace tests.** Reviews:
`reviews/R0-spec-bulk-void-round-{1,2}.md`, `reviews/whole-branch-review-bulk-void-round-1.md`.
**Remaining queue-item-3 cycles: Cycle 4 bulk-classify-inbound-income · Cycle 5 bulk-reclassify-outflow.**

---

## ✅ bulk-resolve-conflict — SHIPPED (2026-07-03) — QUEUE ITEM 3, CYCLE 2 DONE

Bulk `C` flow to accept/reject many `ImportConflict` blockers at once, + **Task 1**: extract the shared
`persist_bulk_decisions` helper (empty-guard + mid-batch rollback + single save) and re-point
bulk-link-transfer & bulk-self-transfer-in through it (zero-behavior). CLI: two apply fns
(`apply_bulk_accept_conflicts` → `SupersedeImport` / `apply_bulk_reject_conflicts` → `RejectImport`) behind
a clap ArgGroup — **NO `ResolveKind` in btctax-cli** (R0-I1: it lives only in tui-edit; referencing it from
cli = dependency cycle). Structured `BulkResolveRow` (current/new payloads); Tier-B non-revocable confirm
(not typed-word); candidate = live `ImportConflict` blockers only; not added to `is_revocable_payload`.
R0 GREEN (2 rounds; r1 caught the `ResolveKind` cycle); whole-diff review 0C/0I — two fault-injections
(mid-batch rollback removed → 3 KATs RED incl. both re-pointed callers; accept→`RejectImport` →
`accept_adopts_new` RED). **1016 workspace tests.** Reviews:
`reviews/R0-spec-bulk-resolve-conflict-round-{1,2}.md`,
`reviews/whole-branch-review-bulk-resolve-conflict-round-1.md`.
**Remaining queue-item-3 cycles: Cycle 3 bulk-void · Cycle 4 bulk-classify-inbound-income · Cycle 5
bulk-reclassify-outflow.**

---

## ✅ bulk-classify-inbound-self-transfer — SHIPPED (2026-07-03) — QUEUE ITEM 2 DONE

The inbound mirror of `bulk-link-transfer` applied to Cycle A's `InboundClass::SelfTransferMine`: sweep
many pending unknown-basis inbound deposits → self-transfer-in ($0 conservative basis, non-taxable) in one
filtered, per-row-excludable, confirmed, atomic batch. Preview surfaces the **total USD being given $0
basis** (over-tax exposure, honest floor). CLI `reconcile bulk-classify-inbound-self-transfer` (two-phase,
`--dry-run`/`--yes`) + TUI `B` flow. **Core-read-only** (reuses `ClassifyInbound`; `btctax-core` untouched).
The R0 catch (I1): the candidate set must exclude inbounds already targeted by a non-voided `ClassifyInbound`
(mirror `open_classify_inbound_flow` filter-3, NOT the matcher) + wallet-less ones — because
`UnknownBasisInbound` is re-emitted for gift-basis-unknown / wallet-less states; sweeping one would append a
duplicate → return-blocking Hard `DecisionConflict` (first-wins keeps the tax number). Income stays safe
(fires `FmvMissing`, never `UnknownBasisInbound`). Spec R0 GREEN (2 rounds); whole-diff review 0C/0I/0M/1N
(3 fault-injection probes RED-then-restored; additive-only, 0 tests removed). **1005 workspace tests.**
Governed by [[self-transfer-completion-policy]]. Reviews:
`reviews/R0-spec-bulk-classify-inbound-self-transfer-round-{1,2}.md`,
`reviews/whole-branch-review-bulk-classify-inbound-self-transfer-round-1.md`.

**Nit (non-blocking):** [WD-N1] `draw_bulk_sti_modal` — the "Σ USD → $0 basis :" label colon doesn't
column-align with the two lines above. Cosmetic. — OPEN (nit).

**NEXT (the LAST approved queue item): bulk reconcile for the OTHER decision types** — void ·
resolve-conflict · outflow→Sell/Spend/Gift/Donate (FMV auto as estimated proceeds for Sell) ·
inbound→Income. Its own [[standard-workflow]] cycle(s); likely split across a couple of cycles.

---

## ✅ self-transfer completion, Cycle B — matched in/out pairs — SHIPPED (2026-07-03) — PROGRAM COMPLETE

Identify + CONFIRM that an inbound leg + an outbound leg are two sides of one self-transfer. Two
representations: **RELOCATE** (cross-wallet, dest tracked) reuses the existing `TransferLink` out→in (basis
carries to the destination); **DROP** (passthrough — coins in+out of a tracked waypoint to external) = a
NEW `EventPayload::SelfTransferPassthrough` decision mapping BOTH legs to `Op::Skip` (net zero, no lot, no
tax). A read-only **matcher** (`Session::self_transfer_match_plan`) PROPOSES pairs (candidate ins =
`UnknownBasisInbound`, outs = `pending_reconciliation`; amount-within-fee-tolerance + ±2-day directional
window + one-in/one-out ambiguity + txid corroboration; DROP/RELOCATE suggested by wallet topology) — but
NEVER auto: the user confirms every pair (CLI `reconcile match-self-transfers` two-phase / TUI
proposal-list). **False-match safety is structural** (only unreconciled legs are candidates). The
load-bearing **[I1] cross-type overlap guard** (a separate post-collection loop) raises a Hard
`DecisionConflict` if a passthrough leg also carries a taxable classification → the taxable event ALWAYS
wins (never silently skipped). Spec R0 GREEN (2 rounds; round 1 caught I1 + the void surface); whole-diff
review 0C/0I/0M/2N (fault-injected I1 both directions + DROP; the CLI force-apply verified unable to hide a
taxable event). **992 workspace tests.** Governed by [[self-transfer-completion-policy]]. Reviews:
`reviews/R0-spec-self-transfer-passthrough-round-{1,2}.md`,
`reviews/whole-branch-review-self-transfer-passthrough-round-1.md`.

**The self-transfer completion program (Cycle A inbound + Cycle B matched pairs) is COMPLETE.**

**NEXT (user-approved queue, 2026-07-03):** (1) **bulk-classify-inbound-self-transfer** — the inbound
mirror of bulk-link (sweep leftover unmatched `UnknownInbound` deposits → self-transfer-in, $0 basis,
filtered/per-row-excludable/confirmed/atomic; surface the total USD given $0 basis); then (2) **bulk
reconcile for the OTHER decision types** (void, resolve-conflict, outflow→Sell/Spend/Gift/Donate,
inbound→Income). Each its own [[standard-workflow]] cycle.

**Nits (non-blocking):** [WD-N1] the CLI "writes-nothing" test asserts event-count not bytes (byte-exact
coverage already exists via the TUI cancel KAT); [WD-N2] Phase-2 confirm of an ambiguous proposed pair
doesn't re-echo the ambiguity flag (spec-compliant). — OPEN (nits).

---

## ✅ self-transfer completion, Cycle A — inbound self-transfer-in — SHIPPED (2026-07-03)

New `btctax-core` capability (the first core change in a long TUI-only series): classify a pending
inbound `TransferIn` as **"my own coins" (`InboundClass::SelfTransferMine`)** — the missing 4th path (an
unmatched inbound was `Op::UnknownInbound`, hard-gated, no lot). Creates a **non-taxable** origin lot:
basis defaults to **$0** (conservative; optionally `--basis`), acquired_at defaults to the **receipt
date** (short-term; optionally `--acquired`), `basis_pending: false` (a $0 basis is computable → NEVER
gates the return), `BasisSource::SelfTransferInbound`, `sigma_in += sat` (FR9), and an **Advisory**
`SelfTransferInboundZeroBasis` flag only when basis was defaulted. Outside FIFO/HIFO/LIFO by construction.
`forms.rs how_acquired_from → Review` (provenance lost — honest). CLI `reconcile
classify-inbound-self-transfer` + TUI classify-inbound extension. Rides the EXISTING `ClassifyInbound`
decision (reuses collection/first-wins/persist). Brainstorm→architect design→spec R0 GREEN (2 rounds) →
whole-diff review 0C/0I/1M/1N (4 fault-injection probes: G1 never-gates, G2 non-taxable, G6 outside-FIFO,
G4 attested-zero-silent — all RED-then-restored). **970 workspace tests.** Governed by
[[self-transfer-completion-policy]]. Reviews: `reviews/R0-spec-self-transfer-inbound-round-{1,2}.md`,
`reviews/whole-branch-review-self-transfer-inbound-round-1.md`.

**Folded [WD-M1]:** the zero-basis advisory message now says to VOID-then-reclassify (classify-inbound is
first-wins, so re-running `--basis` would conflict, not update) — matching the Income path.

**NEXT — Cycle B (matched in/out pairs):**
- **`SelfTransferPassthrough` drop primitive** — a new `EventPayload` decision mapping BOTH legs of a
  passthrough (coins in + out of a tracked waypoint, leaving to external) to `Op::Skip` (net zero, no
  tax, no lot). The RELOCATE half (cross-wallet, destination tracked) already exists as `TransferLink`
  out→in. — OPEN (feature; the next cycle).
- **the confirmed matcher** — a read-only proposal pairing UNRECONCILED legs (amount within a fee
  tolerance, time window, txid corroboration), user-confirmed per pair, NEVER automatic (a coincidental
  income-in + sale-out must not be auto-collapsed). — OPEN.
- **bulk-classify-inbound-self-transfer** — a bulk version of Cycle A (after single-item ships). — OPEN.
- **[WD-N1 nit]** the optional `--acquired > receipt date` future-typo warning (spec G7) — not
  implemented (a future date only makes the lot short-term = conservative). — OPEN (nit).

---

## ✅ bulk-link-transfer (`b` / `reconcile bulk-link-transfer`) — SHIPPED (2026-07-03)

Bulk self-transfer: apply `TransferLink`→`Op::SelfTransfer` to many pending outbound transfers at once,
filtered by time frame + optional source wallet, each linked to ONE destination wallet, atomically +
reversibly, behind a USD-value preview. Both surfaces — CLI `bulk-link-transfer` (two-phase:
`bulk_link_plan` read + `apply_bulk_link_transfer` write; `--dry-run`/`--yes`) + TUI `b` flow (dest
pick-or-**type** → filter → per-row-exclude checklist → confirm → atomic apply). Selection =
`pending_reconciliation` (already excludes decided/linked outs); a mid-batch append failure reverts the
WHOLE batch [I1]; honest USD floor `≥ $X (N unavailable)` [I2]; typed cold-wallet destination [Fork B].
`btctax-core` untouched. First feature born from the full brainstorm→spec pipeline: R0 GREEN (2 rounds;
caught the mid-batch-rollback + USD-floor) → whole-diff review GREEN (0C/0I/2M/3N; 3 fault-injection
probes RED-then-restored). **946 workspace tests.** Reviews:
`reviews/R0-spec-bulk-link-transfer-round-{1,2}.md`, `reviews/whole-branch-review-bulk-link-transfer-round-1.md`.

Scope was **self-transfer-only, out→wallet, one destination per batch**. CONSCIOUSLY DEFERRED
(tracked-open backlog, USER-DIRECTED — do not auto-start):

- **out→in auto-matching.** v1 links each selected outflow to ONE chosen *wallet* (`TransferTarget::Wallet`);
  it does NOT fuzzy-match outs to specific inbound TransferIn events. A future pass could pair outs with
  candidate `TransferIn`s by amount/date proximity. — OPEN (feature).
- **other reconcile decision types.** Bulk applies ONLY `TransferLink` (self-transfer). Bulk
  reclassify-outflow (Sell/Spend/Gift/Donate), bulk classify-inbound, etc. are not in scope — each needs
  per-decision required inputs (proceeds/FMV/donee) that resist a single-confirm batch. — OPEN (feature).
- **TUI free-text `--from/--to` date RANGE.** The TUI filter offers All + each distinct year (a picker,
  no free-text date entry); an arbitrary date range is CLI-only (`--from`/`--to`, `Frame::Range`). The
  year picker + per-row exclude covers the TUI case (R0 Fork-A: KEEP CLI-only). — OPEN (feature).
- **backport the typed destination [Fork B] to the single `l` link-transfer flow.** The bulk `b` flow
  accepts a TYPED destination (`parse_wallet_id` → a never-seen `self:cold-wallet` is reachable). The
  single `l` flow is still pick-list-only (its R0-I2 limitation: destinations sourced from `snap.events`).
  The typed-dest affordance built here should be backported to `l`. — OPEN (small; `open_link_transfer_flow`
  `main.rs`, `handle_lt_target_pick_key`).
- **[M1 whole-diff] CLI empty-plan cosmetic.** On an empty plan the CLI renders a header-only preview
  table before the "no pending outbound transfers match" line (harmless redundancy; output still correct).
  Move the empty check above `render_bulk_link_preview`. — OPEN (nit).

---

## ✅ Terminal chunk-5 burndown — DISPOSITION (2026-07-03) — AUTONOMOUS RUN COMPLETE

The post-chunk-3 autonomous run (mandate 2026-07-02: save-rollback + hardening → chunk 4 → chunk 5 →
burndown; STOP after the chunk-5 burndown) is **COMPLETE**. Shipped to `main`: A `tui-edit-save-rollback`
(`8c8b924`), B `tui-edit-hardening` 6 items (`755e47c`), C chunk 4 = 4a+4b (`f31c1d6`), D chunk 5
(`396a728`). The mutating-TUI editor is **feature-complete** (chunks 1/2a/2b/3/4/5). **931 workspace tests.**

**Terminal-burndown triage (architect-decided).** Every open chunk-4/chunk-5 review followup was triaged.
The decisive finding: **not one item is simultaneously cheap AND worth a code change** — the valuable
items are feature/engine-scoped; every cheap item is already-adequate, no-practical-impact, or
never-triggering. So this burndown is a **documentation-only closing pass** (no code TDD cycle; §8
scaled-down ceremony): one code-comment correction + this disposition record. Disposition:

- **FIXED (comment):** **[C5-3a]** the `open_safe_harbor_allocate_flow` doc comment (`main.rs:4967`) mis-cited
  `load_all`/`project` as KAT-G1-gated — only `conn(` is a persist-only token; reads aren't gated. Reworded.
  (Zero runtime risk — the gate strips comments; no KAT needed.)
- **CONSCIOUSLY DEFERRED — tracked-open (rationale per architect triage):**
  - **[4a-1]** classify-raw 6-variant builder — a feature; CLI `classify-raw --payload-json` covers the rest.
  - **[4a-2]** link-transfer to a never-seen wallet — needs a wallet registry (none exists); the pick-list is
    sourced from `snap.events` by design (R0-I2); CLI `--to-wallet` is the escape.
  - **[4a-3]** TargetPick empty-lists UX — already adequate (per-mode empty hints render at
    `draw_edit.rs:2148/2170`); residual is cosmetic.
  - **[4b-N1]** optimize-accept `made` open- vs enter-time — no practical impact (midnight boundary only,
    R0-round-2-blessed); the "fix" adds churn to the rollback path for zero gain.
  - **[C5-1]** ProRata cross-wallet redistribution — a `btctax-core` feature (open question O4); the TUI is
    already faithful to core (G3).
  - **[C5-2]** allocate-E2E date skip-guard — a `now < 2026-04-15` guard can never fire (window closed;
    run terminating) → would add permanently-dead code. Left as-is (monotonically safe; production
    date-correct; date-independent arm-3 coverage exists).
  - **[C5-3b]** `AllocLotRow`→`TargetList<AllocLot>` — zero-value cosmetic refactor with nonzero risk.
  - **[C5-3c]** `fmt_btc`/`sat_to_btc` — cross-crate, different return types + sign semantics; not a
    mechanical dedup.

  These remain OPEN in their chunk sections below as tracked backlog — the next work is USER-DIRECTED
  (the autonomous mandate is discharged; do NOT auto-start).

---

## ✅ tui-edit chunk 5 (safe-harbor-allocate `A`) — SHIPPED (2026-07-03) — MUTATING-TUI PROGRAM FEATURE-COMPLETE

Cycle D (chunk 5), the FINAL feature cycle. **safe-harbor-allocate (`A`)** — CREATES a
`SafeHarborAllocation` (the §7.4 pre-2025 Universal-residue snapshot @ 2025-01-01). Recompute the residue
via a new additive `Session::safe_harbor_residue` (returns lots + the `LotMethod` used; KAT-G1-clean; the
CLI command refactored to share it, DRY); Preview (method toggle — residue is method-INDEPENDENT) →
REVOCABLE modal (not typed-word; creation is voidable while inert) → single-append
`persist_safe_harbor_allocate` (save_or_rollback, no side-table, no latch). Completes the
create(`A`)→attest(`a`)→void(`v`) loop. Voidability tracks EFFECTIVENESS not attestation (#7 encodes it);
at the current date every fresh allocation is timebarred/inert/voidable. `btctax-core` unchanged. Spec R0
2 rounds → 0C/0I (verified the 3 residue gotchas: voidability / timebar-at-current-date / ProRata);
whole-diff review → 0C/0I/1M/3N (3 fault-injection probes; the E2E date-dependence assessed
monotonically-safe + production date-correct; btctax-core untouched). **931 workspace tests.** Reviews:
`reviews/R0-spec-tui-edit-chunk5-round-{1,2}.md`, `reviews/whole-branch-review-tui-edit-chunk5-round-1.md`.

**FOLLOWUPS recorded:**
- **[C5-2 M-DATE] the two allocate E2E tests embed an implicit "today > 2026-04-15" assumption** (a fresh
  allocation is timebarred only past `TY2025_RETURN_DUE`). Monotonically safe (passes now and forever
  forward; production uses `now_utc()` and is date-correct at any date; date-independent arm-3 coverage
  exists via a ProRata-unattested seed). Optional: add a `now < 2026-04-15` skip-guard for pre-deadline
  determinism. — OPEN (non-blocking, test hygiene).
- **[C5-3 nits] cosmetic:** the opener doc comment over-lists `load_all`/`project` as KAT-G1-gated (they
  aren't; intent correct); `AllocLotRow` duplicates `AllocLot` (a `TargetList<AllocLot>` would suffice);
  `draw_edit::fmt_btc` mildly duplicates `btctax-tui`'s `sat_to_btc`. All harmless. — OPEN (non-blocking).
- **[C5-1] ProRata `AllocMethod` records the tag but does NOT redistribute basis cross-wallet (matches
  core open question O4).** Both `ActualPosition` and `ProRata` seed the safe-harbor allocation from the
  SAME per-wallet actuals (`crates/btctax-cli/src/cmd/reconcile.rs` I-1 note + O4; `Session::safe_harbor_residue`
  in `crates/btctax-cli/src/session.rs`); the recorded `method` changes ONLY the engine's
  timebar/effectiveness rule (`ProRata ⇒ always-timebarred-unless-attested`), never the displayed lots. The
  chunk-5 TUI allocate flow (`A`) records the elected method tag and shows the actuals; its Preview/modal are
  worded so ProRata does NOT imply cross-wallet redistribution (G3). A true cross-wallet pro-rata
  redistribution is unimplemented in the engine (core O4) — out of scope here; the TUI is faithful to core.
  *Recommend* implementing ProRata redistribution in `btctax-core` transition seeding, then surfacing it in
  both the CLI command and the TUI preview. — OPEN (non-blocking; tracks the core O4 gap).

---

## ✅ tui-edit chunk 4b (resolve-conflict + optimize-accept) — SHIPPED (2026-07-03) — CHUNK 4 COMPLETE

Cycle C (chunk 4), second half. **resolve-conflict (`i`)** — accept/reject a flagged `ImportConflict`
→ `SupersedeImport`/`RejectImport` (NON-revocable: prominent warning, both-sides modal, not typed-word).
**optimize-accept (`z`)** — the heaviest flow: recompute the optimizer via a new additive
`Session::optimize_proposal` (KAT-G1-clean — all optimizer plumbing stays in btctax-cli), pre-filter
(changed & not `ForbiddenBroker2027` & no live LotSelection), pick → (NeedsAttestation: text step) →
persist a `LotSelection` + the `optimize_attestation` side-table (the INVERSE of `persist_void`'s
attest-clear; whole-DB rollback reverts both; KAT-G1 gains `optimize_attest::set`). No per-disposal Δtax
(the R0 catch: the data model has only a whole-year `delta`, shown once as a flow banner). Positive
closed-loop with `persist_void` (voiding an optimize-accepted LotSelection clears its attest row).
`btctax-core` untouched. Spec R0 2 rounds → 0C/0I (round 1 caught the per-disposal-Δtax data-model gap +
the `map_opt_err`/`tax_date` reachability); whole-diff review → 0C/0I/0M/1N (3 fault-injection probes;
diff clean, 36 deletions a rehunk artifact). **921 workspace tests.** Reviews:
`reviews/R0-spec-tui-edit-chunk4b-round-{1,2}.md`, `reviews/whole-branch-review-tui-edit-chunk4b-round-1.md`.

**Chunk 4 (import-level decisions) is COMPLETE:** 4a (link-transfer, classify-raw) + 4b
(resolve-conflict, optimize-accept). All 5 CLI reconcile/optimize verbs now have TUI decision flows.

**FOLLOWUP recorded:**
1. **[WB4b-N1 nit] optimize-accept `made` date** — the `Persistability` verdict is fixed at open-time
   (`proposal_made`) while the attestation's `attested_at` is computed at Enter-time; they could differ
   by one day at a midnight boundary (no practical impact; matches the CLI's single-`made` intent).
   Optional tighten: thread the opener's `proposal_made` through to the persist call.

**NEXT: chunk 5 — safe-harbor-allocate** (the CREATION side of SafeHarborAllocation; pre-2025 residue
math; LARGE/COMPLEX) per the roadmap, then the terminal chunk-5 burndown.

---

## ✅ tui-edit chunk 4a (link-transfer + classify-raw) — SHIPPED (2026-07-03)

Cycle C (chunk 4) of the autonomous run, first half (architect split 4a/4b). Two new TUI decision
flows on the shipped substrate: **link-transfer (`l`)** — link a pending TransferOut to a TransferIn
or a wallet → `TransferLink` → non-taxable self-transfer (wallet-list unions ALL distinct event
wallets, not just `holdings_by_wallet` — an R0 catch); **classify-raw (`u`)** — classify an
`Unclassified` raw import → `ClassifyRaw` with a struct-accurate Acquire/Income builder (the two
dominant variants). Both single-append via `save_or_rollback`; both revocable. Spec R0 2 rounds →
0C/0I (round 1 caught wrong builder struct-fields + the wallet-source narrowing); whole-diff review →
0C/0I/1M/2N (3 fault-injection probes verified the KATs load-bearing; numstat churn verified a benign
diff-artifact — only 8 import lines removed). `btctax-core`/`btctax-cli` untouched. **906 workspace
tests.** Reviews: `reviews/R0-spec-tui-edit-chunk4a-round-{1,2}.md`,
`reviews/whole-branch-review-tui-edit-chunk4a-round-1.md`.

**FOLLOWUPS recorded:**
1. **classify-raw remaining-variant parity** — the TUI builder covers Income + Acquire; the CLI
   `classify-raw --payload-json` also accepts Dispose/TransferOut/TransferIn/Unclassified. Deferred
   (a full 6-variant structured builder + the FIELD_CAP=64 free-text limit); CLI remains for the rest.
2. **link-transfer to a never-seen wallet** — the Wallet-target pick-list offers only wallets that
   appear in `snap.events` (no wallet registry exists); a brand-new destination wallet isn't offerable
   → the CLI `reconcile link-transfer --to-wallet <id>` remains. [R0-I2]
3. **[WB4a-3 nit] link-transfer TargetPick empty-lists UX** — if a pending TransferOut has no wallet
   and no other event carries one, both target lists are empty at TargetPick (Enter is a graceful
   no-op, Esc exits) with no status hint. Minor polish: show "no link targets available".

**NEXT: chunk 4b** — resolve-conflict (accept/reject) + optimize-accept (re-derive its design against
post-4a HEAD).

---

## ✅ tui-edit-hardening (chunk-3 follow-ups #1/2/3/6/7/8) — SHIPPED (2026-07-03)

Cycle B of the autonomous run (roadmap `design/ROADMAP_autonomous_run.md`). The six select-lots +
safety/UX hardening fixes: **#1** SelfTransfer disposals are now selectable in select-lots (in-TUI
reconstruction from non-voided `TransferLink`s, engine-faithful — sorted by `decision_seq`, FIRST-WINS,
`consumed_ins` dedup); **#2** pre-2025 disposals offer Universal-pool cross-wallet candidate lots via a
feasibility-honest gate (`l.acquired_at < TRANSITION_DATE && basis_source != SafeHarborAllocated` — the
R0 review caught that the naive gate would offer §7.4 Path-B seed lots that fail `selection_feasible`);
**#3** under-covered (`UncoveredDisposal`) disposals are pre-filtered out of select-lots (no doomed
selection); **#6** free-text donation fields accept 512 chars (per-instance `FieldBuffer` cap; money/ID
fields keep 64); **#7** the void list pre-filters EFFECTIVE `SafeHarborAllocation`s (neither timebar nor
unconservable) — closing the permanent §7.4 doomed-void trap that KAT-E2E-ATTEST-VOID used to pin (that
KAT rewritten to assert the empty list; the §7.4 engine guard stays pinned by
`crates/btctax-core/tests/transition.rs:365`); **#8** the CLI-void remedy in 6 status arms names "quit
the editor first" (VaultLock audit). `btctax-core` untouched. Spec R0 2 rounds → 0C/0I; whole-branch
review + M1 fold (the reachable inert-alloc `is_safe_harbor` E2E assertion) → GREEN, 3 fault-injection
probes verified the KATs load-bearing. **workspace tests green.** Reviews:
`reviews/R0-spec-tui-edit-hardening-round-{1,2}.md`, `reviews/whole-branch-review-tui-edit-hardening-round-1.md`.

**Chunk-3 follow-up status:** #1/2/3/6/7/8 RESOLVED (this cycle) + #9 RESOLVED (save-rollback cycle). Of
the original chunk-3 followups, only **#4 (safe-harbor-allocate) = chunk 5** and **#5 (WB-I4a) =
informational** remain — both accounted for in the roadmap.

**FOLLOWUPS recorded (new, small):**
1. **select-lots final-state vs fold-time lot residual** — the TUI offers CURRENTLY-projected lots, not
   the pool AT the disposal's fold position; a lot created by a LATER split (`bump_split`, e.g. a
   pre-2025 self-transfer fragment) can be offered for an EARLIER pre-2025 disposal where it was
   infeasible at fold time. Fails SAFE — the engine raises `LotSelectionInvalid`, which GATES
   `compute_tax_year` (never a silent wrong number), and `derive_select_lots_status` arm 2 surfaces it.
   The irreducible "final-state ≠ fold-time" gap; the CLI (re-projects at fold position) is exact.
2. **#1 SelfTransfer in-TUI reconstruction drift** — the TUI re-derives the SelfTransfer set from
   `snap.events` rather than a core API; if the engine's link logic evolves, the TUI copy could drift
   (backstopped by `LotSelectionInvalid`). A `pub fn` in `resolve.rs` exposing the honoring set would be
   zero-drift (additive-MINOR to core) — deferred.

**NEXT: cycle C — chunk 4 (import-level decisions)** per the roadmap.

---

## ✅ tui-edit-save-rollback (mutating-TUI hardening #9) — SHIPPED (2026-07-03)

Cycle A of the autonomous post-chunk-3 run (roadmap: `design/ROADMAP_autonomous_run.md`, order
A→B→C→D→E). A failed `session.save()` in any of the 8 editor persist fns now reverts the in-memory
DB byte-identically (`Vault::snapshot`/`restore` over `sqlite_io`, `Session` wrappers,
`save_or_rollback`) — so a confirmed-but-unsaved decision can NEVER piggy-back a later save. Replaces
the old "failed save → residue → retry = N+2 rows + DecisionConflict" with "failed save → clean no-op;
retry is clean (same `decision_seq`)". `PersistError{NoChange,RolledBack,ResidueLive}` (no `Display`);
`on_persist_error` is the sole site arming the new `rollback_failed` latch on `ResidueLive`; the 9
opener guards folded into `residue_latch_status` (attest wording verbatim). Whole-DB restore reverts
`persist_void`'s `optimize_attest` side-table clear for free (incl. a post-append `clear`-failure —
WB-M1 fold). `persist_tax_profile` INCLUDED for a uniform invariant. **Attest left latched** (its
double-batch is unrecoverable; unification filed below). Spec R0 2 rounds → 0C/0I; whole-branch review
+ M1 fold → GREEN. **876 workspace tests.** Reviews: `reviews/spec-review-tui-edit-save-rollback-r0-round-{1,2}.md`,
`reviews/whole-branch-review-tui-edit-save-rollback-round-1.md`.

**FOLLOWUP recorded:**
1. **Attest adopts snapshot/restore → retire `attest_save_failed`** — once the rollback mechanism has
   soaked, `persist_safe_harbor_attest` can use `save_or_rollback` too (a clean rollback of its
   two-decision batch makes the unrecoverable double-batch impossible and even permits safe in-editor
   retry), retiring the separate C1 latch and folding `residue_latch_status` down to one branch.
   Deliberately deferred this cycle (do not wire a brand-new mechanism into the catastrophic path
   until it soaks). [N1 nit: the 3 remaining "silent" persist headers could gain the one-line
   "reverted on failed save" note — the module header already documents the invariant; no action.]

**NEXT: cycle B — `tui-edit-hardening`** (the 6 items: #1/2/3 select-lots + #7/8/6 safety/UX), per the
roadmap. Re-recon B against post-A HEAD first (A churned the opener heads + persist layer).

---

## ✅ Mutating-TUI chunk 3 — select-lots + set-donation-details + safe-harbor-attest — SHIPPED (2026-07-02)

The remaining decision flows: `s` select-lots (specific-ID lot assignment; disposals + BOTH gift/donation
removals, fee-mini + already-selected pre-filtered; wallet from the raw `LedgerEvent`; Σpick == principal
conserved in-TUI; duplicate ⇒ `DecisionConflict` on the 2nd id, NEITHER applies, method-order fallback until
one is voided), `d` set-donation-details (Form 8283 §B appraiser/donee side-table upsert, last-write-wins,
pre-populated on re-edit from `snap.donation_details`), `a` safe-harbor-attest (IRREVOCABLE §7.4; typed-word
`ATTEST`; two-decision atomic Void+re-attest batch; the C1 residue latch — `attest_save_failed` blocks all 9
mutating openers after a failed save so no unrelated save can piggy-back the in-memory batch; close-on-Err,
no retry path). Spec R0 2 rounds → 0C/0I; whole-branch review (3 independent lenses — safety, engine-semantics,
test-fidelity) round 1 → 0C/2I (both on the test/docs surface; no product-code defect), folded + re-reviewed
→ GREEN. **868 workspace tests.** Review: `reviews/whole-branch-review-tui-edit-chunk3-round-1.md`.

**Whole-branch review folds (round 1):** [I1] KAT-V-DD-4 was coverage theatre (re-implemented the
List→FieldForm pre-population mapping IN the test body — a dropped optional-field pre-population passed
uncaught, risking a last-write-wins upsert of `None` over a stored field) → rewritten to drive the real
`d`→List→Enter→FieldForm path, assert all 10 buffers, then Enter→modal for the validator round-trip
(fault-injection-verified: dropping a production pre-population line now fails the test). [TF-M1]
KAT-E2E-ATTEST-ERRLATCH now loops the latch refusal over ALL 9 openers, not just a/f/p. [SAFE-M1] dead code
in the select-lots "no lots"/modal-Enter arms removed. [SAFE-N1 nit] declined — reusing `parse_date_arg`
would leak `CliError`'s "usage:" prefix into a TUI field error; the inline parse is format-identical and
KAT-V-DD-3-pinned.

**FOLLOWUPS recorded for chunk 3:**

1. **SelfTransfer select-lots under-inclusion** — linked TransferOut events that project to `Op::SelfTransfer`
   are method-honoring (`honoring_principal` → `Some`) but are absent from the TUI select-lots list (not in
   `state.disposals`/`state.removals`). Under-inclusion only (safe direction; the CLI `select-lots` remains
   available). Fix = scan `snap.events` for a TransferOut with a non-voided TransferLink (the SelfTransfer
   case) and include it in the disposal list.
2. **Lot-display at disposal date** — the TUI shows currently-projected lots, not the pool available AT the
   disposal date; the engine validates accurately (fires `LotSelectionInvalid` on re-projection), so the
   display is a best-effort guide. **[ENG-m1] narrows this:** for a disposal DATED before `TRANSITION_DATE`
   the engine consumes from `PoolKey::Universal` (un-partitioned by wallet), but the TUI candidate-lot filter
   (`l.wallet == item.wallet`, main.rs) offers only the disposal-wallet's lots — so a valid cross-wallet
   pre-2025 selection can be un-presentable. Under-inclusion only. Fix = drop the wallet filter when
   `item.date < TRANSITION_DATE`.
3. **[ENG-m2] Shortfall-disposal principal target** — for an under-covered disposal (`UncoveredDisposal`),
   `Σ legs.sat < op.sat`, so `validate_select_lots` conserves against a smaller number than the engine's
   `honoring_principal`; a TUI-passing selection is then engine-rejected as `LotSelectionInvalid`. Degenerate
   (the disposal already carries a Hard `UncoveredDisposal`) and surfaced by `derive_select_lots_status`
   Arm 2 — no silent loss. One-line guard candidate.
4. **Safe-harbor-allocate TUI flow** — `reconcile safe-harbor-allocate` (the CREATION side of the allocation)
   is out of scope for chunk 3 (attest-only cure path). The user creates the allocation via CLI, then attests
   via the TUI. Deferred to chunk 5.
5. **WB-I4(a) carryforward** — the raw-vs-effective under-inclusion (2b FOLLOWUP) does NOT affect chunk 3
   (select-lots uses already-projected disposals/removals; donation-details targets removals by `RemovalKind`;
   attest targets `SafeHarborAllocation` by voided-set scan).
6. **FIELD_CAP=64 CLI-parity limit** — the free-text donation fields (addresses, `appraiser_qualifications`)
   truncate at 64 chars in the TUI (form.rs); the CLI accepts arbitrary length. Candidate fix = a larger cap
   for designated free-text fields.
7. **Void-list pre-filter for effective allocations [R0-I6]** — the 2b void flow still LISTS an effective
   (attested) allocation, and a confirmed void is a permanently-damaging no-op (§7.4 doomed-void Hard
   `DecisionConflict`; KAT-E2E-ATTEST-VOID pins today's behavior). Effectiveness is derivable from blockers —
   pre-filter effective allocations out of the void list in a later chunk so the trap is unreachable.
8. **[SAFE-M2] Pre-existing 2a/2b void-remedy statuses omit "quit the editor first"** —
   `derive_classify_inbound_status` / `derive_reclassify_income_status` / `derive_set_fmv_status` name
   `"CLI: btctax reconcile void {}"` without the quit-first clause the R0-C1 lock audit mandates (the editor
   holds the exclusive VaultLock for its lifetime). Present verbatim at `main` (NOT a chunk-3 regression) and
   each names the in-editor `press 'v'` remedy first, so not a safety hole. Apply the quit-first fold to these
   strings in a follow-up.
9. **In-memory residue after failed saves (2a/2b flows)** — the C1 piggy-back mechanics exist for the benign
   single appends of the shipped flows too (keep-form-open retry). Benign there (re-confirm is the intended
   remedy; the payloads are revocable), but consider generalizing the `attest_save_failed` latch into a
   session-dirty latch for all failed saves.

**NEXT: chunk 4** — import-level decisions (link-transfer, classify-raw, accept/reject-conflict,
optimize-accept). Chunk 5 = safe-harbor-allocate (the creation side). The chunk-3 spec/pattern carries over.

---

## ✅ Mutating-TUI chunk 2b — reclassify-income + set-fmv + VOID — SHIPPED (2026-07-02) — THE RECONCILE FAMILY IS COMPLETE IN THE GUI

The correction family: `r` reclassify-income (required-explicit business; kind-optional; the Interest→
Mining E2E pins exact NIIT −$380.00 / SE $1,412.96 effects), `f` set-fmv (latest-wins re-point — no
conflict), `v` VOID (the exact nine-variant revocable set; SafeHarborAllocation with the mandatory Path-B
+ permanence warning; the DEPENDENT-DECISION CASCADE stated in the modal + KAT'd end-to-end — orphans fire
conflicts on their own ids, "void those too"; the honest void-REJECTED status; the void retry verified
OPPOSITE to classify's — idempotent, +2 inert rows, no conflict; the LotSelection void clears
optimize_attest, unit-locked). The four 2a remedy arms now name the in-editor Void flow first (all pins
strengthened in place — a mechanized diff analysis found ZERO deleted asserts). Spec R0 2 rounds → 0C/0I;
whole-branch 2 rounds → 0C/0I. **845 workspace tests.**

**[I2 records]:** (a) WB-I4(a) raw-vs-effective under-inclusion now spans the 2b lists too (deferred,
same remedy); (b) [M3] a REJECTED SafeHarbor void permanently hides the in-force allocation from the v
list (documented in the modal; refine-later); (c) cascade conflicts are invisible to the immediate status
when attributed to orphans (the Compliance tab carries them; a generic blockers-diff status is a deferred
enhancement); (d) [R0-N3] hoisted-set staleness across re-projections (the 2a precedent, benign);
(e) possible duplicate f-list rows under duplicate FmvMissing blockers (not observed; dedupe later).

**NEXT: chunk 3** — select-lots, set-donation-details, safe-harbor attest (the remaining decision flows)
→ chunk 4 import → chunk 5 optimize. The 2a/2b specs are the pattern; the chunk-2 recon lineage maps most
of chunk 3's surface.

---

## ✅ Mutating-TUI chunk 2a — classify-inbound + reclassify-outflow — SHIPPED (2026-07-02)

The first decision-APPENDING GUI flows: filterable target pick-lists from the projected state (the
compound inbound pre-filter — UnknownBasisInbound + resolves-to-TransferIn + no non-voided classify —
ADVERSARIALLY VERIFIED: no listable target can produce a DecisionConflict; outflows via
pending_reconciliation, post-filtered by construction); per-variant forms (Income/GiftReceived;
sell/spend/gift/donate — spend = GROSS proceeds) with CLI-parity validation; payload-showing modals
(donee for gift AND donate; the both-donor-None warning); statuses derived from the RE-PROJECTED blockers
(honest FmvMissing / gift-refire / price-gap / UncoveredDisposal surfacing; the only remedy ever named =
void-then-re-classify — the double-prefixed remedy ref caught empirically and fixed red-then-green +
mutation-tested); the STRICT append-only prefix tests; per-flow cancel-bytes + chmod save-failure KATs.
Spec R0 2 rounds → 0C/0I (7 Importants incl. the FIRST-WINS retry story); whole-branch 2 rounds → 0C/0I.
**810 workspace tests.** Process note: the Task-1 implementer's "all green" report was FALSE (5 E2E
failures at its commit, fixture-side, fixed test-only) — caught by the next agent's honest report + a
first-hand check; reviewer trust-notes now standard.

**[WB-I4 records, spec-mandated]:** (a) the inbound pre-filter checks RAW payloads, not effective —
UNDER-inclusion only (a ClassifyRaw'd-to-TransferIn row won't list; remedy = CLI; harden later);
(b) donee trim/cap divergence: the TUI caps the buffer, the CLI accepts unbounded — unify later;
(c) negative-sign parity: fmv/amount fields accept negatives on BOTH surfaces today (CLI parity
preserved) — tighten both together later; (d) KAT-C2a q-swallow at text steps documented (q types);
(e) the retry-duplicate escape hatch depends on CLI void until **chunk 2b** ships the void flow.

**NEXT: chunk 2b** — reclassify-income + set-fmv + void (the correction family; 1-3 fields each; the
void flow closes the in-editor remedy loop). Then chunk 3 (select-lots/donation-details/attest),
chunk 4 (import), chunk 5 (optimize).

---

## ✅ Mutating-TUI chunk 1 — btctax-tui-edit (tax-profile editing) — SHIPPED (2026-07-02) — THE KEY GOAL's first chunk

The first vault-writing GUI binary, under the two-guarantee structure: the VIEWER went lib+bin (pure
visibility — its write-free guarantee, E10 gate, and 76-test suite byte-untouched); the EDITOR
(`btctax-tui-edit`) holds a live `mut Session` (VaultLock-exclusive, documented), writes ONLY via
`edit/persist.rs` (its own mechanized gate incl. the four vault-CREATING constructor tokens — the R0-I1
hole), every mutation behind a payload-showing confirmation modal (Enter → typed setter → `save()`'s
atomic tmp/.bak/rename path → live re-projection; Esc → bytes-identical; failed-save semantics pinned +
KAT-S1 chmod-forced, green un-ignored). Chunk-1 flow: `p` → the 10-field tax-profile form (pre-populated;
CLI-parity validation incl. whitespace pin) → confirm → the Tax tab recomputes. Safety: the append-only
prefix test (full-row+ordinal `load_all_ordered`, new in core), the cancel-bytes test, E2E CLI-readback.
Spec R0 2 rounds → 0C/0I; whole-branch review 0C/0I (M1 modal-values asserts folded). **777 workspace
tests.**

Deferred (OPEN): a sealed write-token (type-level modal gating); per-mutation bundled-data reload;
try_env_passphrase duplication; the t1-report surface-listing drift (record-only); tightening negative
validation on BOTH surfaces (CLI+editor) together. **NEXT: chunk 2 — the reconcile-decision family**
(classify-inbound, reclassify-outflow/income, set-fmv, void — the append_decision flows on the same
skeleton; the prefix test's strict form activates).

---

## ✅ Export-from-TUI + FOLLOWUPS burndown 3 — SHIPPED IN PARALLEL (2026-07-02)

Two lanes, isolated (main tree + worktree), user-approved parallelization; landed export-first, burndown
rebased cleanly (the coordination pin held — 6/6, zero conflicts). Combined: **725 workspace tests**.

**Export-from-TUI:** the viewer's first write capability under the re-scoped guarantee ("never the vault
or any decrypted image; only the four named form CSVs on explicit confirmation"): `e` → a confirmation
modal → a fresh exclusive 0o700 timestamped dir (the new `fsperms::mkdir_owner_only_exclusive` — closes
the mkdir-p clobber/symlink vector) → `write_form_csvs` (exactly form8949/schedule_d/form8283/schedule_se,
0o600). The E10 mechanized source-scan gate (comment-stripping, mutation-tested); profile-gated SE parity
by calling the pub `render_schedule_se` (the TUI hand-rolled SE block is gone — disclosure drift dead);
swap-catching hard-coded parity goldens + the donee-passthrough e2e. R0 2 rounds + whole-diff → 0C/0I.

**Burndown 3:** the **bad-target backfill** (ReclassifyOutflow/ClassifyInbound/ManualFmv now validate at
collection time against the effective payload → Hard `DecisionConflict` + exclusion; ManualFmv latest-wins
preserved; zero fixtures relied on the old silence) — **the mutating-TUI safety prerequisite is DONE**;
the §6017 $400 floor note (text-only, §1402(j)(2) carve-out, the $397.10 half-even tie); negative-W-2-flag
binary tests; the hook mode-assertion KAT; TY2024 full-schedule equality locks (all 32 pairs). R0 2 rounds
+ whole-diff → 0C/0I/0M. Task-2 records: the CI report's clippy-baseline misstatement noted (record-only);
the old gift-chunk3b review's synthetics converted to ·-notation (M-2, this commit).

Deferred (OPEN): E10 scanner string-literal false-negative hardening (M-1); export.rs test-region
everywhere-token exemption (M-2-export); a typed/sealed write-token (the ExportConfirmState FOLLOWUP);
the nine stale-but-true STRICTLY-READ-ONLY lines in sibling tab modules; `do_export`'s se_result_for
duplication; blocker detail/attribution test-pinning (N-1); E11 asserting AlreadyExists-kind (done in
4f02b7a — CLOSED).

**NEXT: the mutating-TUI program (THE KEY GOAL — user 2026-07-02)** — prerequisite (this backfill) +
substrate (the export modal + write discipline) both in place. Separate `btctax-tui-edit` crate; 4-6
chunks; recon → chunk-1 spec next. Then 5a FDF / 5b filled-PDF (Jan–Feb 2027) behind it.

---

## ✅ CI infrastructure — SHIPPED (2026-07-02) — form program item 1

GitHub Actions CI (`.github/workflows/ci.yml`): test / clippy `-D warnings` / fmt / **MSRV 1.88** /
generic-shape PII scan — all `--locked`, `permissions: contents: read`, the 3 actions SHA-pinned
(independently re-resolved at review). Plus a **fail-closed range-scanning pre-push hook**
(`scripts/pre-push`, 100755 — the review caught the mode-644 fail-open + the `--not --all` scan-nothing
arm empirically): owner patterns from an untracked `scripts/.pii-patterns` (missing OR empty → exit 1;
`BTCTAX_PII_BYPASS=1` scoped to that check only — the generic scan always runs); scans EVERY rev in
`remote..local` (new refs via `--not --remotes`); `:(exclude)LICENSE` the sole allowlist entry. 18 hook
KATs (temp-workspace copies). R0 3 rounds + whole-diff + confirmation → 0C/0I. 692 tests.

**[M5 AMENDED — the user's own recorded decisions]:** the old "cargo +1.74 MSRV gate" item is superseded.
(1) **MSRV → 1.88** (the empirical floor: lockfile v4 + the time/instability/darling families bind at
1.88): the USER selected "Raise MSRV to the true floor" in the 2026-07-02 in-session structured question
(vs downgrading deps). (2) **LICENSE carve-out** for the owner-name scan: per the USER's standing rule
("…only LICENSE author name allowed"). Corollary ratified: `render.rs` `map_or(true,…)`→`is_none_or`
(the lint is MSRV-gated; behavior-identical).

**Operator setup (required for the hook to be active locally):** `git config core.hooksPath scripts` +
create `scripts/.pii-patterns` (one regex per line; untracked) — see `scripts/README-pii-setup.md`.
**Post-merge acceptance:** the first green CI run on GitHub (recorded at ship). **Branch-protection
ruleset:** the documented `gh api` command is in the spec — pending the operator's go-ahead.

Deferred (OPEN): a mode-assertion KAT (N-2); the report's clippy-baseline misstatement (M-1, record-only);
pre-existing real-hyphen synthetics in an older review file vs the Notation rule (M-2); Windows/macOS
runners; cargo-audit/deny.

---

## ✅ TY2024 tables backfill — SHIPPED (2026-07-01) — THE CONFIRMED QUEUE IS COMPLETE

Queue item 3 (last). `ty2024()` in BundledTaxTables: all 28 ordinary bracket edges (Rev. Proc. 2023-34
§3.01 — incl. HoH 35%@243,700, MFS 37%@365,600), the four LTCG pairs (§3.03 — MFS max_fifteen 291,850,
NOT the naive half), gift $18,000 (§3.43), lifetime $13,610,000 (§3.41), SS wage base $168,600 (SSA/88 FR).
Every digit verified by the author AND two independent reviewers against the primary sources (the
whole-diff reviewer re-fetched IRB 2023-48 + FR 2023-23317). KATs A6a-d/A7 (the R0 caught the
ST-gains-ARE-NII omission: MFS $396.00 incl. $38.00 NIIT) + structural + report-path + TY2025 byte-identical
regression. `report --tax-year 2024` now computes. R0 2 rounds → 0C/0I; whole-diff 0C/0I. 692 tests.

Deferred (OPEN): full-schedule equality KATs per status (M1 — the A6 delta KATs can cancel lower-edge
errors; pin all 28 edges directly). **TY2026 tables SHIPPED 2026-07-05** (Rev. Proc. 2025-32 — see the
top-of-file entry); **TY2027 stays BLOCKED on IRS/SSA publication (fall 2026).**

**Queue COMPLETE (NII slice → SE cluster → TY2024). Next: the user-approved form-program sequence** —
CI infrastructure → small-FOLLOWUPS burndown → export-from-TUI → 5a FDF/XFDF → the mutating-TUI program
(position 6, fall 2026) → 5b filled-PDF (Jan–Feb 2027).

---

## ✅ SE completion Chunk B — Schedule C expenses (advisory-only) — SHIPPED (2026-07-01) — SE CLUSTER COMPLETE

Final SE chunk (queue item 2 done: A W-2 coordination + C ReclassifyIncome + B expenses).
`TaxProfile.schedule_c_expenses` → `compute_se_tax(…, expenses)`: net_se = max(0, gross − expenses) before
×0.9235 (§1402(a)); fully-expensed → None with a THREE-WAY render split (no false "wage base unavailable"
note — liability status is "no tax owed"); composes with the W-2 caps (goldens $11,303.64 / None /
$5,593.84); engine-B `crypto_ord` stays GROSS with a quantify-don't-prescribe advisory (the I3 mechanism —
no OTI-edit prescription); all three surfaces (report/CSV/TUI) source the profile. R0 2 rounds → 0C/0I;
whole-diff 0C/0I after a test-only fold (engine-B invariance KAT, report↔CSV parity, fully-expensed
integration, real-binary negative-flag — the review caught them missing). 682 tests.

Deferred (OPEN): engine-B gross-vs-net `crypto_ord` coordination (the real ordinary-income fix — high
blast radius); §6017 $400 SE filing floor (not modeled; salient with expenses); the TUI condensed-block
disclosure lines (Chunk-A N-1 family).

**Next (queue + the architect-sequenced form program, user-approved 2026-07-01, no TY2025 extension):**
TY2024 tables backfill → CI infrastructure (MSRV 1.74 gate + PII scan — BEFORE the new write surface/dep)
→ small-FOLLOWUPS burndown → export-from-TUI (form CSVs only; never export_snapshot/the vault image;
scoped export.rs + confirmation modal + extended bytes test) → 5a FDF/XFDF form-data output (zero deps, no
template redistribution; builds the per-(form, revision-year) field-mapping architecture) → 5b filled-PDF
(Jan–Feb 2027, when the IRS publishes the TY2026 revisions; lopdf MSRV-verify at pin time; Form 8949 may
stay an attached statement per Exception 2). Mutating-TUI placement: architect consult in flight.

---

## ✅ SE completion Chunk C — ReclassifyIncome decision (business flip) — SHIPPED (2026-07-01)

Queue item 2, chunk 2 of 3. New event-sourced `ReclassifyIncome{income_event, business, kind:
Option<IncomeKind>}` decision + `reconcile reclassify-income <ref> --business <true|false> [--kind …]`
(explicit-value, required, binary-verified) — closes the River `business:false` immutability (river.rs
comments updated). Collection-time bad-target validation against the EFFECTIVE payload → Hard
`DecisionConflict` + exclusion (a DELIBERATE divergence from ReclassifyOutflow's silently-inert behavior);
FIRST-WINS dedup; void via VoidDecisionEvent; build_op-only override (fold untouched). KATs: the headline
flip enables compute_se_tax; engine-B invariance under business-only flips; NON-VACUOUS kind-flip NIIT
deltas ±$380.00 (the reviewer corrected the implementer's ±$190 derivation — the code/KAT were right);
back-compat (old vaults load; old binaries fail LOUD — documented). R0 2 rounds → 0C/0I; whole-diff
0C/0I after folds (the --business SetTrue parse bug caught empirically against the binary). 670 tests.

**Deferred (OPEN) — [I-2 backfill]: `ReclassifyOutflow` (and `ClassifyInbound`/`ManualFmv`) bad-target
handling is SILENTLY INERT** (blind collection, consulted only in the matching build_op branch) — backfill
the same collection-time effective-payload validation → Hard blocker that ReclassifyIncome now has.

**Cluster remaining: Chunk B** — Schedule C expenses (ADVISORY-ONLY: `TaxProfile.schedule_c_expenses` →
net_se = max(0, gross − expenses); engine-B gross-vs-net coordination explicitly deferred — high blast
radius; precise advisory text per the recon).

---

## ✅ SE completion Chunk A — W-2 wage coordination — SHIPPED (2026-07-01)

Queue item 2, chunk 1 of 3. `TaxProfile.w2_ss_wages`/`w2_medicare_wages` (`#[serde(default)]`; CLI flags,
negative-rejected on the real path, `--show`) → `compute_se_tax(…, w2_ss, w2_medicare)`: SS cap =
max(0, wage_base − w2_ss) (§1402(b)(1)/Sch SE 8a-9) + Additional-Medicare threshold = max(0, threshold −
w2_medicare) (§1401(b)(2)(B)/Form 8959 Part II). ALL THREE surfaces (report/CSV/TUI) source the profile;
asymmetric transposition + export-parity KATs. Goldens $6,295.70 (both directions) / ss-$0 above-base /
addl-$831.15 threshold-zeroed (deductible $7,064.78 unchanged — addl still excluded). The dual-direction
"$0 assumed" hedging REPLACED with accurate coordinated/unset text; the §164(f) advisory now QUANTIFIES the
first-order overstatement (no OTI-edit prescription — wrong mechanism, R0-I3). P2-D figure-sets
byte-identical. R0 2 rounds → 0C/0I (formulas verified against the actual Sch SE + Form 8959); whole-diff
0C/0I. 655 tests.

Deferred (OPEN): a binary-level test pinning the negative-flag Usage errors (M-1; the config_dispatch.rs
harness makes it cheap — pair with the same gap on --prior-taxable-gifts); the TUI's condensed SE block
omits the coordination disclosure text (N-1). **Cluster remaining: Chunk C** — ReclassifyIncome decision
(River business:false flip; new EventPayload variant + resolve collection + build_op override + CLI;
old-vaults-read-fine back-compat) → **Chunk B** — Schedule C expenses (ADVISORY-ONLY: reduces net_se,
floored at 0; engine-B gross-vs-net coordination explicitly deferred — high blast radius). Full §164(f)
auto-coordination remains deferred (circular + breaks the identity).

---

## ✅ NII interest slice — crypto-lending interest → §1411 NII — SHIPPED (2026-07-01)

Queue item 1 (user-confirmed order). **RESOLVES the B-M1 "per-IncomeKind NII" deferral** — the known
residual NIIT understatement. `IncomeKind::Interest` income now enters `nii_with` (WITH-scenario ONLY, per
the crypto_ord attribution convention — a both-scenario insertion would cancel out of the `r.niit` delta);
mining/staking/airdrops/rewards remain excluded (§1411(c)(6) SE / non-NII other income); MAGI unchanged
(interest already in crypto_agi — no double-count); `nii_without`/the identity/SE untouched. Disclosure
"cannot yet isolate" language replaced at all 3 sites; the pinned KAT re-pointed semantically. Goldens
(TDD red→green): $570.00 headline (min-cap over-bound; absolute total $4,970.00 = ord_delta $4,400 + NIIT
$570) + $380.00 mixed Mining+Interest boundary lock (wrong-inclusion → $1,520). The 5-golden B-M1
regression net byte-identical. R0 GREEN round 1; whole-diff 0C/0I (both goldens + the bracket math
independently re-derived). 647 tests.

Deferred (OPEN, disclosed): the §1411(c)(2) active-trade-or-business lending exception (business-agnostic
inclusion is conservative for the atypical active-lender case); Form 8960 generation. Nits (cosmetic, sweep
opportunistically): the render footer names the excluded kinds twice; an optional §1411(c)(2) code comment.

**Next (queue):** SE-tax completion → TY2024 tables.

---

## ✅ Charitable/gift cluster — Chunk 1: §170(f)(11)(F) aggregation + Form 8283 FMV-method — SHIPPED (2026-07-01)

First of three chunks in the user-directed charitable/gift completion cluster (deferred Phase-2/3). Form
8283 Section A/B now decided on the YEAR aggregate claimed-deduction for similar property (all BTC =
similar; §170(f)(11)(F)), not per-donation; a year-aggregate qualified-appraisal advisory fires when the
aggregate > $5k even if no single donation does (CCA 202302012 — no readily-valued exception for crypto).
`fmv_method` = honest section-derived label (Section B → "qualified appraisal"; Section A → empty — no
fabrication). Shared core `year_donation_deduction` helper (form + advisory + CSV can't diverge).
STANDALONE (forms.rs + render.rs; engine B/fold/event-schema/state untouched). R0 3 rounds → 0C/0I;
whole-branch review 0C/0I. 590 tests.

---

## ✅ Charitable/gift cluster — Chunk 2: donee identifier + per-donee Form 709 — SHIPPED (2026-07-01)

Second chunk. `donee: Option<String>` on the `ReclassifyOutflow` STRUCT (`#[serde(default)]` — back-compat
safe; `GiftOut` stays a unit variant so legacy vaults still open) → `Op::GiftOut`/`Donate` → `Removal.donee`
→ removals.csv + Form 8283 donee column + CLI `reclassify-outflow --donee`. Form 709 gift advisory
refactored to PER-DONEE §2503(b) exclusion (TY2025 $19k) — the key correctness fix (two donees at $15k each
= $0 taxable, no filing, vs the old aggregate rule that wrongly flagged $30k) + filing-required trigger +
an unlabeled-bucket conservative caveat. STANDALONE (donee is data; `tax/`/engine B untouched — asserted).
R0 2 rounds → 0C/0I (C1 = the unit-vs-struct-variant vault back-compat trap, empirically caught);
whole-branch review 0C/0I. 602 tests.

---

## ✅ Charitable/gift cluster — Chunk 3a: §2505 advisory-level lifetime exemption — SHIPPED (2026-07-01)

Chunk 3 split into 3a (§2505 advisory) + 3b (Section-B appraiser) for shippability. 3a: year-indexed
`TaxTable.gift_lifetime_exclusion` (TY2025 $13,990,000, Rev. Proc. 2024-40 §2.41) + a `--prior-taxable-gifts`
CLI flag → the per-donee gift advisory now shows §2505 consumption (cumulative = prior + current labeled
taxable; remaining floored at 0; "no gift tax due until the lifetime exclusion is exhausted; then the
excess base" — strict `>`, $13.99M boundary → remaining $0 not exceeded). Advisory-level, single-filer (no
§2513/portability/DSUE/§2502 rate liability); discloses the labeled-only omission when unlabeled gifts
exist. STANDALONE (compute.rs untouched; goldens unmoved). R0 2 rounds → 0C/0I (legal core web-verified);
whole-branch review 0C/0I. 611 tests.

(3a's nits were swept in 3b: the KAT-B assertion now pins `"($0.00 remaining)"`; the
`--prior-taxable-gifts` negative-validation is always-on, locked by a real binary-level test.)

---

## ✅ Charitable/gift cluster — Chunk 3b: Form 8283 Section-B appraiser/donee details — SHIPPED (2026-07-01) — CLUSTER COMPLETE

Final piece. `DonationDetails` type in core (`donation.rs`) with section-aware
`is_review_complete(Form8283Section)` (Section B requires the full §6695A block — appraiser name +
TIN-or-PTIN + appraisal date + qualifications + donee EIN; Section A complete-on-presence); a
`donation_details` SIDE-TABLE in cli keyed by `EventId::canonical()` (mirrors `optimize_attestation` —
idempotent DDL, defensive init, old-vault back-compat); `reconcile set/show-donation-details` (validates
against the projected removals; Donation-only, Gift-arm error tested). `form_8283(state, year, details)`
populates structured donee/appraiser, `fmv_method_override` (resolves the Chunk-1 Section-A deferral,
user-supplied — honest), and the SECTION-AWARE `needs_review` flip (skeletal Section-B stays true — the
honest-gap lock); 6 new form8283.csv columns; TUI `Snapshot.donation_details` (read-only guarantee
compile-intact, vault-bytes-unchanged passing). STANDALONE (tax//project//state.rs untouched). R0 2 rounds
→ 0C/0I; whole-branch review 0C/0I; the final Minors folded (real binary-level negative-guard test; e2e
side-table→form_8283 seam test). 645 tests.

**The charitable/gift completion cluster is COMPLETE** (Chunks 1, 2, 3a, 3b all shipped). Deferred (OPEN):
filled-PDF Form 8283 (CSV only); a donee registry (re-use across donations); the §2502 gift-tax rate-
schedule liability (advisory-only §2505 today); an event-sourced/decision variant of donation details
(side-table chosen); real FMV provenance on RemovalLeg (the override covers the form need); §2513
gift-splitting + portability/DSUE.

**Next (user-confirmed queue):** NII interest slice (spec in flight) → SE-tax completion → TY2024 tables.

---

## ✅ GUI sub-project 1: btctax-tui ratatui read-only viewer — SHIPPED (2026-07-01)

First GUI work (user-directed: "work on gui first"). New `btctax-tui` crate — a ratatui terminal UI,
strictly READ-ONLY: unlock the PGP vault → tabs for Holdings/Disposals/Income/Tax/Forms/Compliance, all
from the pure read-only builders (`Session::open` + `load_events_and_project` + `compute_tax_year`/
`compute_se_tax`/`form_8949`/`schedule_d`/`form_8283`/`disposal_compliance`/`build_verify`). Read-only
enforced at COMPILE level (immutable `Session` binding → `save()` won't compile; `conn()` forbidden) +
review grep + a byte-identical-vault test. Passphrase moved (`mem::take`, capped, never cloned/rendered);
offline (only ratatui 0.29 + crossterm 0.28; MSRV 1.74; Cargo.lock committed); terminal restored on
exit/Err/panic (`TerminalGuard` + panic hook); VaultLock `Locked` handled; `q` typeable in the passphrase.
Figure parity with the CLI by construction (same builders). Additive only — core/cli/store/adapters
untouched. Spec R0 2 rounds → 0C/0I; 5 SDD tasks each independently reviewed; whole-branch review 0C/0I.
584 workspace tests.

Deferred (OPEN → later): **export-from-TUI** (CSV/snapshot); the **mutating flows** (import, reconcile/
classify, config, tax-profile set, optimize run/accept/consult, safe-harbor attest) — a future interactive
TUI or the egui/graphical GUI; **`r` refresh (re-project)** + **`?` help overlay** (trimmed from the footer
until implemented); charts/visualizations; mouse support; concurrent read-only vault open (vs the exclusive
VaultLock); **CI infra** (no `.github/workflows` exists — add one, incl. the `cargo +1.74` MSRV gate [M5]
and the PII scan). Next GUI step (when user-directed): either the egui graphical viewer or the
interactive/mutating TUI.

---

## Standing roadmap — next program (user-approved 2026-06-30; auto-pick-up after slugs ship)

The Phase-1 burndown (below) + both slugs (pre-2025 filed-method reconciliation mechanism; minimal
appraisal-trigger — a **term-aware claimed-deduction proxy** Σ(LT-legs FMV + ST-legs basis) > $5k, NOT
the originally-proposed FMV>$5k∧basis>$5k AND-rule which under-flagged the LT-appreciated case) have all
SHIPPED. **Automatically pick up Phase 2: Forms & §170(e) deduction computation** — no re-ask. Sequence: §170(e) charitable-deduction computation
(FMV-vs-basis, ST/LT reduction) → upgrade the minimal appraisal-trigger to the precise
>$5k-claimed-deduction trigger (§170(f)(11)(C)); Form 8949 + Schedule D generation; Form 8283 + Form
709 routing; SE-tax routing (business mining → Schedule SE); slot in **B-M1** (NIIT loss-year
understatement). Lower/triggered: adapter refinements (TransferIn basis gap, Gemini BTC-pair FMV,
owner-confirms), hardening + Windows/macOS CI, 2026/2027 tax tables (arms the 2027+ broker gate),
§1091 wash-sale enactment, multi-year horizon optimization, non-BTC scope. (Mirror of memory
`phase2-standing-roadmap`.)

## ✅ Phase-2 P2-D: self-employment tax routing — SHIPPED (2026-07-01) — Phase-2 program COMPLETE

Fourth + final Phase-2 sub-project. Branch `feat/p2d-se`; R0 spec 3 rounds to 0C/0I (independent
web-verification caught: deductible must EXCLUDE §1401(b)(2) Additional Medicare per §164(f)(1);
W-2 disclosure direction — SS overstated but Additional-Medicare UNDERstated; Interest §1402(a)(2)
carve-out); whole-slug review 0C/0I ($14,935.42 C1-lock re-derived; banker's rounding load-bearing).
`tax/se.rs::compute_se_tax(state, year, status, table) -> Option<SeTaxResult>`: net_se = Σ(business,
non-Interest income) × 92.35% (§1402(a)); SS 12.4% capped at `TaxTable.ss_wage_base` (year-indexed,
TY2025 $176,100 SSA); Medicare 2.9%; Additional-Medicare 0.9% over §1401(b)(2) threshold; deductible_half
= (ss+medicare)/2 EXCLUDING addl. `render_schedule_se` + `schedule_se.csv` (year-scoped) wired into the
tax-report; dual-direction W-2 disclosure + "no business expenses modeled" caveat + standalone note.
**STANDALONE — NOT folded into `total_federal_tax_attributable`** (§164(f) coordination + preserves the
`total==ord_delta+ltcg+niit` identity; D5 KAT asserts the total is unchanged). 525 tests.

Deferred (OPEN → later): `TaxProfile.w2_ss_wages`/`w2_medicare_wages` field (W-2 coordination for employed
miners — disclosed via the correct-direction note); a `ReclassifyIncome`/business-flip decision (the River
`business:false` immutability — a River business-miner must re-import with a patched adapter); Schedule C
deductible mining EXPENSES (net SE = gross income; conservative/overstates — caveat rendered); §164(f)
½-SE-deduction auto-coordination into the income-tax total; SS wage base for TY2024/2026+.

---

## ✅ Phase-2 Forms & §170(e) program — COMPLETE (2026-06-30 → 07-01)

The user-approved standing-roadmap program is done: P2-A (§170(e) charitable-deduction) → P2-B (Form
8949 + Schedule D) → B-M1 (NIIT loss-year correctness fix) → P2-C (Form 8283 + Form 709 advisory) → P2-D
(SE tax). All shipped to `main`, each spec→R0-to-green→implement→whole-diff→ship at 0C/0I, with
primary-source tax verification catching multiple directional errors (appraisal AND-rule; B-M1
over-vs-under; §2.42→§2.43 citation; SE §164(f) deductible; SE W-2 disclosure direction). Remaining
Phase-2/3 work is all deferred FOLLOWUPS (donee identifier/full Form 709, Section-B appraiser struct,
§170(f)(11)(F) aggregation, per-IncomeKind NII interest slice, w2-wages/expenses/ReclassifyIncome,
year-indexed tables for other years) + the standing lower/triggered items (adapter refinements, CI/
hardening, 2026/2027 income-tax tables, §1091 wash-sale monitor, multi-year optimization, non-BTC).

## ✅ Burndown pass 2 (2026-06-30) — A/B/C deferrals resolved

Branch `chore/followups-burndown-2`, three groups each independently reviewed to 0 Critical / 0
Important; workspace gate green (433 tests). Closed:

- **A (lot-id):** A-M1 (`disposal_compliance` SelfTransfer scope — documented intentional exclusion,
  code doc + SPEC §A.5); A-Task-7-M2 (extracted shared `method_election_is_forward` predicate, DRY,
  De-Morgan-verified behavior-preserving); A-Task-8a (`compliance_status_tag` stable, both renderers
  off `{:?}`); A-Task-8b (selection_count guard — moot, documented); A-Task-9b (no-op identity KAT
  `evaluate_disposal(existing,no-selection)==project()`); A-M3 (binary-level `Command::Config`
  dispatch tests); A-Task-4 plan doc `90.00`→`90.25`.
- **A-N2 / A-N3 — RESOLVED:** N2 (evaluate_disposal `lots_after` shape for C) — C shipped and Mode-2
  `consult_sale` consumes `evaluate_disposal` successfully. N3 (B/C per-year Hard-blocker gate) — B's
  `compute_tax_year` `first_hard_blocker` gate + C's `PreTransitionYear`/`YearNotComputable` refusal
  both shipped. No code owed.
- **B (rate engine):** B-F1 (`fmt_money` 2dp on all tax-report money fields, display-only — no tax
  figure changed); B-Minor (`niit_applies` doc aligned to code semantic); B-nits (redundant
  rust_decimal_macros dev-dep removed; `filing_status_tag` stable in tax-profile --show; `events`
  param kept+documented; advisory-only→Computed KAT; §4.3 stale doc line).
- **C (optimizer):** C-M1 (exhaustive_min eviction strict-only → baseline wins exact ties, no
  delta==0 divergent pick; oracle-exactness + delta≤0 + determinism preserved; regression KAT
  `tie_exact_baseline_kept_when_lex_smaller_is_not_baseline`); C-M2 (`ConsultReport.approximate` from
  the heuristic flag + ⚠ note in render_consult); C-M3 (proposal scope-boundary footer).

---

## ✅ Phase-2 P2-C: Form 8283 + Form 709 gift advisory — SHIPPED (2026-07-01)

Branch `feat/p2c-8283`; R0 spec 2 rounds to 0C/0I; comprehensive whole-slug review 0C/0I after folding
an Important (a wrong statutory citation — the deeper review fetched the IRS PDF and caught §2.42→§2.43,
propagated from the round-1 R0; the $19,000 value was correct). `RemovalLeg.acquired_at` (= gain_hp_start,
matches term — no loss zone for removals). `forms.rs::form_8283(state, year)`: per-leg Form 8283 rows,
Section A (≤$5k) / B (>$5k) by `claimed_deduction`; how_acquired from basis_source
(Purchased/Gift/Other/Review); donee/appraiser/fmv_method BLANK + `needs_review` (honest user-input
flags, never fabricated); `form8283.csv` (0o600) with a standing §170(f)(11)(F) aggregation caveat + a
≤$500 note as `#` header comments. `TaxTable.gift_annual_exclusion` (TY2025 $19,000, Rev. Proc. 2024-40
**§2.43**); `render_gift_advisory` thin Form 709 over-annual-exclusion signal (donee not modeled →
total-exposure only; emits a note when a year has gifts but no table). Standalone (no engine-B change).
509 tests.

Deferred (OPEN → later): **§170(f)(11)(F) similar-item YEAR-aggregation** for the Section A/B split
(disclosed via the standing caveat; aggregate-of-small-donations case not computed); **donee identifier**
on Donate/GiftOut → full Form 709 (per-donee exclusion + lifetime exemption) + Form 8283 donee/FMV-method
fields; **Section B appraiser-info struct**; gift-exclusion tables for TY2024/2026+ (year-dependent);
how_acquired origin-loss for CarriedFromTransfer/SafeHarborAllocated lots; future-interest/non-citizen-
spouse gift cases.

## ✅ Phase-2 B-M1: §1411 NIIT net-capital-loss fix — SHIPPED (2026-06-30)

Branch `feat/p2-bm1-niit`; R0 spec 0C/0I with INDEPENDENT primary-source web-verification; comprehensive
review 0C/0I (headline golden re-derived). **CORRECTS the earlier B-M1 note, which was directionally
WRONG:** the minimal NII model did not subtract the §1211-allowed capital loss, so in net-capital-loss
years it **OVERSTATED** NIIT (not understated). Verified vs §1.1411-4(d)(2)+(d)(3)(ii) Example 1 +
Form 8960 line 5a: all dispositions net together; a net capital loss reduces NII by only the §1211(b)
loss (≤ $3k/$1.5k). Fix (`compute.rs`): `nii_{with,without} -= loss_deduction`; NIIT base floored at
`max(0, min(nii, over))`. Golden: Single, crypto ST −$80k + other_lt +$15k → `r.niit` −684.00 (was
−570.00); NII-negative floor → 0.00; MFS → −57.00. No gain-year regression (loss_deduction==0 → no-op).
Disclosure corrected (removed "can only ever understate"). 491 tests.

crypto ordinary income confirmed CORRECTLY excluded from NII (mining/staking/airdrops = SE-excluded
§1411(c)(6) or non-NII "other income"). Deferred (OPEN):
- **Per-`IncomeKind` NII classification:** add crypto-LENDING **interest** to NII (§1411(c)(1)(A)(i)) —
  the only residual understatement slice; the model can't yet distinguish it from other `crypto_ord`.
- **Minor coverage:** a golden pinning the delta path where the no-crypto baseline itself has a §1211
  loss AND `magi_without > threshold` (fix is symmetric/correct there; untested by an asserting golden).

## ✅ Phase-2 P2-B: Form 8949 + Schedule D generation — SHIPPED (2026-06-30)

Second Phase-2 sub-project. Branch `feat/p2b-form8949`; R0 spec 2 rounds to 0C/0I; 2 impl passes each
0C/0I; whole-slug review 0C/0I. New core `forms.rs`: `form_8949(state, year)` → per-leg 8949 rows (ST
Part I / LT Part II; exact-Decimal BTC description; C/F box default + `box_needs_review` for exchange
wallets; NoGainNoLoss gift legs → gain 0; adjustment cols blank per §1091-exempt; deterministic order;
year-filtered) + `schedule_d(state, year)` → raw ST/LT part totals. Two additive `DisposalLeg` fields:
`acquired_at` (ZONE-AWARE = loss_hp_start in the §1015 loss zone, else gain_hp_start — structurally
coupled to `term_for`, can never contradict the row's ST/LT term) + `wallet` (from `Consumed.wallet`).
CLI: `form8949.csv` + `schedule_d.csv` (0o600, year-scoped) + a `render_schedule_d` text section (with a
NotComputable caveat). Reconciles with engine B (schedule_d ST/LT gain == TaxResult.st_net/lt_net on
all-gains/zero-carryforward, independent paths). No capital-gains / basis math change. 487 tests.

Deferred (OPEN → later Phase-2):
- **Per-disposition 1099-B / box (A/B/D/E) user input** — reclassify from the conservative C/F default
  when a 1099-B/1099-DA was issued (`box_needs_review` flags exchange dispositions today). `Form8949Box`
  is currently `{C, F}` only — A/B/D/E structurally unrepresentable until this lands.
- **1099-DA reconciliation** (broker digital-asset reporting: gross proceeds 2025+, basis 2026+) — needs
  broker-data import; the exchange flag prompts manual reconcile meanwhile.
- **Filled-PDF Form 8949 / Schedule D** — no PDF dependency in-tree; CSV + text only for now.
- **Nits:** exchange box flag not year-gated (conservative); ISO vs MM/DD/YYYY dates (defer with PDF);
  SPEC D2 column list omits `box_needs_review` (doc only — code includes it).

## ✅ Phase-2 P2-A: §170(e) charitable-deduction computation — SHIPPED (2026-06-30)

First Phase-2 (Forms & §170(e)) sub-project. Branch `feat/p2a-170e-deduction`; R0 spec 2 rounds to
0C/0I; impl + comprehensive whole-slug review 0C/0I. `Removal.claimed_deduction: Option<Usd>` = exact
§170(e)(1)(A) deduction per donation: **LT→FMV, ST→min(FMV,basis)** (depreciated ST deducts at FMV, not
basis — R0-C1). Drives the appraisal trigger off the exact amount (retired the "proxy"). Surfaced:
donation header, `removals.csv` `claimed_deduction` column (emitted on the FIRST leg only — no multi-leg
SUM double-count), per-year charitable-deduction total labeled "BEFORE §170(b) AGI limits / carryover".
STANDALONE — does NOT feed engine B (Schedule-A figure; `TaxProfile.ordinary_taxable_income` is already
post-deduction). 468 tests.

Deferred (OPEN → later Phase-2 sub-projects):
- **Ordinary-income CHARACTER detection** (dealer/inventory §1221(a)(1), self-created) → those deduct at
  basis even LT; unmodeled (capital-asset investor assumed); disclosed via the retained dealer caveat.
- **Donee-type modeling (§170(e)(1)(B))** — public charity (LT→FMV) vs non-operating private foundation
  (appreciated LT crypto → basis; crypto ≠ qualified appreciated stock); unmodeled; retained donee caveat.
- **§170(b) AGI percentage limits (30%/20%/60%) + 5-yr carryover + OBBBA-2026 0.5% floor / 35% cap** —
  the surfaced figure is BEFORE these; computing the limited/allowed amount is deferred.
- **§170(f)(11)(F) cross-donation aggregation** (from the appraisal trigger) — per-event only.
- **Double-count trap (note):** the §170 deduction is standalone; if a FUTURE sub-project auto-reduces
  `ordinary_taxable_income` by itemized deductions, it must NOT also expect the user's profile income to
  be post-deduction — that would be a separate, careful change.
- **Nit:** legacy "proxy" wording lingers in a few pre-existing test names/comments (cosmetic).

## ✅ Slug: minimal qualified-appraisal trigger — SHIPPED (2026-06-30)

Branch `feat/appraisal-trigger`; R0 spec 3 rounds to 0C/0I (round-1 corrected the AND-rule →
term-aware proxy; round-2/3 fixed a mining-mischaracterized-as-ordinary-income tax error); impl +
comprehensive whole-slug review 0C/0I. Emits Advisory `QualifiedAppraisalNote` on a donation whose
term-aware deduction proxy Σ(LT legs' `fmv_at_transfer` + ST legs' `basis`) > `QUALIFIED_APPRAISAL_THRESHOLD`
($5,000, §170(f)(11)(C), tables.rs) — a conservative upper bound that never under-flags a single donation;
per-donation-event; never gates `compute_tax_year`; decoupled from the manual `appraisal_required` bool.
Detail cites §170(f)(11)(C) + CCA 202302012 (crypto >$5k needs a qualified appraisal, no readily-valued
exception) + character-framed over-flag caveat (§1221(a)(1) inventory/ordinary-income deducts at basis
regardless of holding period) + §170(f)(11)(F) aggregation caveat. 454 tests.

Deferred (→ Phase-2 forms & §170(e) program):
- **Precise §170(e) claimed-deduction** (character-based ordinary-income-property detection) — upgrades
  the proxy from "all LT legs at FMV" to the exact deduction; removes the safe over-flag on LT-held
  dealer/inventory crypto. — OPEN.
- **§170(f)(11)(F) cross-donation aggregation** — the $5k test aggregates similar donated items across a
  tax year; this slug flags per-donation-event only (can miss an aggregate of sub-$5k donations). — OPEN.

## ✅ Slug: pre-2025 filed-method reconciliation mechanism — SHIPPED (2026-06-30)

Branch `feat/pre2025-reconciliation`; R0 spec 2 rounds to 0C/0I; 2 impl passes each reviewed 0C/0I;
whole-slug review 0C/0I. Gave the pre-2025 method declaration engine teeth: `ProjectionConfig`
gains `pre2025_method_attested` (plumbed via `to_projection`); `note_pre2025_once` advisory is
attestation-aware (unattested "have NOT declared" + guidance / attested "DECLARED + ATTESTED", still
Advisory — never gates `compute_tax_year`); `safe-harbor-allocate` REFUSES under an undeclared method
(appends nothing; reads the config flag, not `timely_allocation_attested`). Basis-adjustment math
unchanged. 441 tests.

Deferred from this slug (OPEN):
- **Durable Path-A `Pre2025MethodDeclaration` ledger event (R0-I2).** For a Path-A (no-allocation)
  taxpayer the attested method lives only in mutable `cli_config` (not source-of-truth per NFR6) — no
  audit trail. Add an append-only, supersede-tracked declaration event so the attestation is auditable
  in the ledger. Deferred because it changes NO number for Path A (basis recomputes live under the set
  method; the advisory updates with it) — audit-trail enhancement, not a correctness gap. — OPEN.
- **N-1 (Nit) — `safe_harbor_allocate` reads `session.config()?` twice** (gate + `to_projection`);
  collapse to one read. Cleanup, no correctness impact. — OPEN.
- **N-2 (Nit) — no separate non-FIFO attested-allocate success KAT.** The gate is method-agnostic
  (`if !attested { refuse }`) and KAT (c) proves attested-FIFO allocate records the method; a
  LIFO/HIFO-attested allocate test would round out coverage. — OPEN.

---

## C.5 — Monitor §1091 crypto wash-sale enactment (OPEN)

**What.** §1091 currently disallows losses only on "stock or securities"; crypto is property
(Notice 2014-21) and is **exempt**. The optimizer therefore selects loss lots freely — there is
no 30-day disallowance rule in the current code.

**Why monitor.** Recurring Greenbook proposals and legislative bills (e.g. various "Build Back
Better"-era and subsequent drafts) have proposed extending §1091 to digital assets. None have
been enacted as of this writing (2026-06-30).

**If enacted:** add a 30-day look-back disallowance guard to loss-lot selection in
`crates/btctax-core/src/optimize.rs` (the C.5 doc note identifies the attachment point) AND
update the `## §1091 wash sale (C.5)` module doc note in lockstep. The regression KAT
`tests/optimize_wash_sale.rs::loss_lot_freely_selectable_no_wash_sale_bar` must also be
revised to assert the guard (not the current free-selection behavior).

**Pointer.** `optimize.rs` module doc `## §1091 wash sale (C.5)`; KAT
`tests/optimize_wash_sale.rs`.

---

## Sub-project C (optimizer) — Task-3 review IMPORTANT resolved (2026-06-30)

- **RESOLVED — `available_lots_before` returned the wrong pre-disposal pool for the FIRST 2025 disposal
  under safe-harbor Path B (FIXED).** The old truncate-then-refold never crossed `TRANSITION_DATE` when the
  target disposal was the chronologically-first 2025 timeline event, so the re-fold never fired the §7.4
  transition seed and surfaced the UN-seeded Universal residue — harmless under Path A (residue relocates by
  wallet; lot_ids/basis preserved) but WRONG under Path B (the seed DISCARDS the residue and installs
  `SafeHarborAllocation` seed lots with different lot_ids/basis). Fix: new
  `pub fn fold::pools_before(res, prices, config, target) -> PoolSet` (fold.rs) folds the canonical timeline
  up to (but not including) the target and fires the real `transition::seed_transition` at the correct
  boundary (the seed check runs before the target short-circuit, so it fires even when the target is the
  first ≥2025 event); `available_lots_before` now delegates to it (no duplicated seed logic). KATs added:
  `available_lots_before_path_b_first_2025_disposal_returns_seeded_lots` (fails without the fix) +
  `available_lots_before_path_a_first_2025_disposal_relocates_residue`. R0-I1 canonical ordering preserved
  inside `pools_before`. — RESOLVED (2026-06-30). — optimize.rs / fold.rs; plan §TASK 3 updated.

---

## ✅ Burndown pass (2026-06-29) — actionable Phase-1 items resolved

Branch `chore/followups-burndown`, each fix independently reviewed to 0 Critical / 0 Important;
workspace gate green. What was closed:

**btctax-cli (commits f6880e6, 39e09e0, 282ae20, 4a78727):**
- **RESOLVED — `safe_harbor_status` goes dark when all Path-B lots consumed.** Now ORs in
  `state.disposals[*].legs[*].basis_source` + `removals[*].legs[*].basis_source == SafeHarborAllocated`
  (legs are not filtered by `remaining_sat`), so an effective Path B reports "effective" even after every
  allocated lot is disposed. Test added (all-consumed + stale advisory → still "effective"). Reviewer
  confirmed it cannot mask a genuine time-bar or unconservable state (those never seed SafeHarborAllocated lots).
- **RESOLVED — `verify` double-loads events (recon M-1 / eng M1).** Added
  `Session::load_events_and_project() -> (Vec<LedgerEvent>, LedgerState, ProjectionConfig)`; `verify` and
  `safe_harbor_attest` routed through it. Behavior-preserving; unit-tested.
- **RESOLVED — `{:?}` Debug enums in CSV (eng-M2).** Six stable snake_case `*_tag()` fns
  (`term`→`short`/`long`, `dispose_kind`→`sell`/`spend`, `basis_source`→`exchange`/`cost`/`safe_harbor`/…,
  etc.); all four CSV writers + text renderers switched off `{:?}`. CSV columns are now a stable contract.
  Export test asserts column values. (Exhaustive matches — no `_` fallback masking a real variant.)
- **RESOLVED — weak `set-fmv` test (recon N-1).** Repointed to an FMV-missing `Income` target; asserts the
  `FmvMissing` hard blocker present BEFORE and cleared AFTER `set-fmv` (+ income recognized at the manual FMV).
- **RESOLVED — attest leaves a stale `safe_harbor_timebar` advisory (Plan-4 fold I-2 follow-on).** Subsumed by
  the `safe_harbor_status` fix above (status now keyed on the effective-Path-B signal, not the advisory).

**btctax-adapters (commit 614d43a):**
- **RESOLVED — Swan zero-sat withdrawal counted under `dropped_no_btc` (tax Nit).** Added a distinct
  `skipped_zero_sat` counter to `GroupOutput`/`FileReport` (+ `merge`/`ingest` threading); the Swan arm now
  increments it instead of `dropped_no_btc`. Bucket-neutral (`parsed_rows = rows.len()` counted once), so the
  FR2 identity `parsed_rows = events + dropped_no_btc + unclassified + skipped_zero_sat` holds exactly. Test added.
  CLI import render reads named fields → no CLI change needed.
- **RESOLVED — River `business: false` immutability (tax M2).** Doc note added at both `Income` construction
  sites: `business: false` is hard-coded + immutable post-ingest (Income is not `ClassifyRaw`-able); SE-tax
  exposure requires confirming/changing the mapping at the adapter layer.

**btctax-core (verified by read-only survey — NO code change needed):**
- **VERIFIED already-handled — tax m1 (loss-basis cross-lot edge).** The `loss_basis` drop on a non-dual
  survivor is deliberate + taxpayer-conservative (promoting `None→Some` would misclassify a later sale as a
  §1015(a) dual-basis disposition — a far larger error). KAT `self_transfer_fee_c_cross_lot_normal_survivor_stays_non_dual` (kat_tax.rs:1204).
- **VERIFIED already-handled — tax m3 (principal==0 fee'd transfer).** All four fee arms raise an
  `UncoveredDisposal` blocker (not a silent drop) when there's no surviving leg/lot (fold.rs:569/394/770/836);
  fee-sats still consumed so conservation holds.
- **VERIFIED already-handled — 2025-transition timezone straddle.** Timeline partitioned at the **tax-date**
  boundary (`fold.rs:281` stable sort on `e.date() >= TRANSITION_DATE`); `universal_snapshot` + `pool_key` use
  the same tax-date predicate, so the pre-seed residue matches. KAT `reversed_offset_straddle_seeds_on_tax_date_not_utc_order` (transition.rs:546).
- **VERIFIED already-handled — `allocation_voids`.** Properly declared (resolve.rs:270), populated (286-289),
  consumed in the pass-3 irrevocability check (591-599) — the void-of-allocation behavior the CLI attest relies on.
- **ACCEPTED de-minimis tradeoff — tax m2 (exact-boundary fee holding-period attribution).** When principal
  drains exactly to a lot boundary, the fee-cents basis (from the next lot) rides the earlier lot's holding
  period. Total basis is conserved; only the HP anchor of a few cents shifts, only in the exact-boundary case.
  Fixing it requires splitting fee basis into a separate micro-leg/lot in the conservation-critical fold —
  not worth the complexity/risk for a cents-scale effect. WONTFIX (Phase-1); revisit only if shown material.

---

## ✅ Cycle-prep slug burndown (2026-06-29) — second pass

Ran `cycle-prep` recon (`reviews/cycle-prep-recon-2026-06-29.md`) on four slugs, then burned down one at a time
(cycle-prep → spec → opus R0 review-to-green → implement (SDD) → whole-slug review → ship). Each shipped at
0 Critical / 0 Important; PII-clean; workspace gate green throughout.

- **`vault-half-created-autorepair` — SHIPPED** (merge `db9f074`). `StoreError::HalfCreatedVault` + explicit
  `init --repair` that clears ONLY an orphan key (lock-first `AlreadyExists` guard provably never deletes a
  real/recoverable key); R0 caught the `init::run` arity blast-radius (fixed via wrapper); safety review 0C/0I.
- **`reconcile-allocation-dual-loss-basis` — SHIPPED** (merge `dd990f9`). `AllocLot` gains
  `dual_loss_basis`+`donor_acquired_at` (serde-default); Path-B seed + CLI allocate preserve the §1015(a) dual
  basis + §1223(2) tacking. R0 caught 3 inverted §1015(a) labels pre-implementation (gain=donor carryover,
  loss=FMV-at-gift); conservation unchanged.
- **`pre2025-filed-method-reconciliation` — Phase-1 part SHIPPED** (merge `c881967`). The advisory
  `Pre2025MethodNote` already existed + is surfaced in `verify`; made its message actionable (FIFO-assumed +
  reconcile-against-filings). **The runtime reconciliation MECHANISM (declare filed method → adjust
  reconstructed basis) remains OPEN — Phase-2 feature, deferred.**
- **`appraisal-trigger-precision` — NO-OP** (cycle-prep found the follow-up structurally wrong: no Phase-1
  FMV>$5k auto-flag exists; `appraisal_required` is a user CLI bool). Corrected the citation; Phase-2 only.

## Sub-project A (lot-id substrate) — items folded from the R0-plan review round 1 (2026-06-29)

- **Acquisition-date FIFO corrects a latent §1012 foundation deviation for relocated/seeded lots (R0-plan C1).**
  The shipped foundation's `consume_fifo` walks **insertion (push) order** (`pools.rs:58-100`); Sub-project A's plan
  makes FIFO **acquisition-date order** (`acquired_at` asc, tie `lot_id` asc) at all six consume sites. For
  **relocated** (self-transfer, `fold.rs:537-553,580-583`) and **Path-B-seeded** (`resolve.rs:566-586` →
  `transition.rs:67-73`) lots — which carry an `acquired_at` older than their push position — this is a **material
  behavior change**, not a no-op: it changes reported basis/term on the affected disposals **and** the safe-harbor
  conservation residue `snap.basis` (`transition.rs:25-51`; guard `resolve.rs:546-547`). It is the **legally-correct**
  rule (§1.1012-1(j)(3)(i): earliest *acquisition*; a self-transfer is not a new acquisition, `fold.rs:545`). Landed
  deliberately in A's plan (Task 2 deliberate-change statement + mandatory fixture-re-verification; RED→GREEN divergence
  KATs in Tasks 3 and 6), conservation-re-verified across existing self-transfer / Path-B / safe-harbor fixtures.
  **No real users exist yet (foundation just shipped), so no migration/restatement is owed.** Spec §A.3 reframed
  (deliberate-correctness note) + the spec M2 fold-record line updated. — RESOLVED-in-design (lands when A is
  implemented). — R0-plan C1, `reviews/R0-plan-lot-id-substrate-round-1.md`.

- **N3 (verified N/A) — `inspect::verify` "reads config twice."** `Session::load_events_and_project()` returns a
  **`ProjectionConfig`** as its third tuple element (burndown 2026-06-29, commit 39e09e0), *not* a `CliConfig`. `verify`
  needs the `CliConfig` (declared `pre2025_method` + `pre2025_method_attested`) for its new surfacing, so the separate
  `session.config()?` read is **required**, not redundant. No change. — R0-plan N3.

## ✅ Sub-project A (lot-id substrate) — whole-branch review round 1 deferrals — ALL RESOLVED (verified in source 2026-07-04)

The blocking Important (post-hoc selection + in-force election mis-labeled `StandingOrder`) and in-area Minors
**M2** (`evaluate_disposal` existing-event principal) + **M3** (`config --set-forward-method` apply-all) were FIXED
on `feat/lot-id-substrate` (Task-10 fold). Source: `reviews/whole-branch-review-lot-id-substrate-round-1.md`.

**★ 2026-07-04 verification (all remaining items below were addressed by later cycles but never struck):**
- **M1 (SelfTransfer compliance coverage) — RESOLVED (documented).** `project/compliance.rs:71-83` carries a
  "Scope boundary — `SelfTransfer` is intentionally excluded" doc-comment with the §1.1012-1(j) rationale (a
  self-transfer is non-taxable → no identification obligation attaches; §A.3 method-honoring is about the
  selection mechanism, not compliance-flagging). This is exactly the "if intentionally excluded, document it"
  disposition.
- **Task-4 (`90.00`→`90.25` plan doc) — RESOLVED.** No `90.00`/`90.25` figure remains in
  `IMPLEMENTATION_PLAN_lot_id_substrate.md`.
- **Task-7-M2 (shared election-collector DRY) — RESOLVED.** `project/compliance.rs::collect_elections`
  (lines 47-67) uses the shared `resolve::method_election_is_forward` predicate — no duplicated guard.
- **Task-8 nits — RESOLVED.** (a) `render.rs:133-149 compliance_status_tag` is the stable display
  (`standing_order`/`contemporaneous`/`attested_recording`/`non_compliant`), used at render.rs:1625 — no
  `{:?}`. (b) `render.rs:531-533` documents the intentionally-omitted `Decision`-id guard on `selection_count`.
- **Task-9 nits — RESOLVED.** (a) the `u64::MAX` sentinel is documented at `optimize.rs:1227` ("unreachable
  for real sequences, never persisted"). (b) the no-op identity KAT exists:
  `tests/evaluate.rs:267 evaluate_disposal_existing_no_selection_is_no_op_identity` (asserts legs + st/lt gain
  match `project()`).

_(original deferral text kept below for record.)_

- **M1 (Minor coverage gap) — `disposal_compliance` omits method-honoring SelfTransfers.** SelfTransfers produce no
  Disposal/Removal record, so they never get a compliance row (`compliance.rs` iterates only `state.disposals` /
  `state.removals`). A.3 lists SelfTransfer as method-honoring (a §1.1012-1(j) "transfer" that pre-positions lots
  for future HIFO/gains), so a post-hoc `select-lots` on a self-transfer is never compliance-flagged. Decide
  explicitly whether transfers belong in the projection; if intentionally excluded, document it. — OPEN. — whole-branch M1.

- **Task-4 plan-text `dec!(90.00)` → `90.25` (doc only).** A KAT-text figure in the Task-4 plan reads `90.00` where
  the implemented (correct) TP8(c) fee re-home yields `90.25`. Implementation is correct; only the plan doc text is
  stale. Reconcile the plan text. — OPEN (doc). — whole-branch Task-4 triage.

- **Task-7-M2 — shared election-collector DRY.** `compliance.rs::collect_elections` duplicates resolve's
  `MethodElectionBackdated` guard (both kept in sync by the shared spec rule). Extract a single shared collector to
  reduce drift risk (would also have de-risked the M1 classifier fix). DRY only — no behavior change. — OPEN. — whole-branch Task-7-M2.

- **Task-8 nits.** (a) `ComplianceStatus` is rendered with `{:?}` in `render_verify` — compliance-facing output should
  use a stable `compliance_status_display` (mirrors the burndown `*_tag()` work). (b) `selection_count` lacks a
  `Decision`-guard; moot in practice (a `LotSelection` payload only ever rides a `Decision` event). — OPEN. — whole-branch N1 / Task-8.

- **Task-9 nits.** (a) `evaluate_disposal`'s synthetic event id uses a `u64::MAX` sentinel — documented and
  unreachable by real sequences; revisit only if a typed sentinel is preferred. (b) Add a pinning KAT asserting
  `evaluate_disposal(existing-disposal, no selection) == project()` for that disposal (no-op identity). — OPEN. — whole-branch Task-9.

## ✅ RESOLVED earlier (kept for record)

## btctax-core whole-branch fixes (2026-06-29) — both Important findings resolved

- **I-1 — `ReclassifyOutflow → Dispose` on-chain `fee_sat` silently dropped (FIXED).**
  Added `fee_sat: Option<Sat>` to `Op::Dispose`; `OutflowClass::Dispose` arm now passes
  `t.fee_sat`; native `EventPayload::Dispose` passes `None`. Fold arm calls `consume_fee`
  after principal and re-homes carry onto last disposal leg via `rehome_onto_disposal_leg`.
  Fee-sats are consumed; holdings no longer overstated; conservation is honest.
  KATs: `reclassify_dispose_fee_sat_treatment_c_conservation_honest` and
  `reclassify_dispose_fee_sat_treatment_b_mini_disposition` — both pass.

- **I-2 — Path-B seeded-lot `LotId` collision after post-2025 `SelfTransfer` (FIXED).**
  Added `PoolSet::init_split_counter(origin, next)` and called it in `seed_transition`'s
  Path-B arm after pushing seed lots, setting `next_split[allocation_id] = seed.len()`.
  Later `bump_split(allocation_id)` returns `seed_len` (not 0), so relocated fragments get
  fresh unique split sequences.
  KAT: `path_b_seeded_lot_relocation_no_lotid_collision` — all LotIds unique, conservation
  balanced after partial relocation of a seeded lot.

- **Phase-2 refinement note:** The precise fee-sat disposition treatment when a
  `TransferOut` is reclassified as Dispose is a TP8-adjacent Phase-2 refinement (the Phase-1
  TP8 treatment is applied correctly per the existing TreatmentC/B config; any further
  guidance-specific nuance is deferred).

## btctax-adapters whole-branch fixes (2026-06-29) — both Important findings resolved

- **I-1 — Gemini Buy/Sell on BTC-quoted pairs (ETHBTC/BCHBTC) → Unclassified (FIXED).**
  Added `cols::SYMBOL` and gated `Buy/Sell → Acquire/Dispose` on `Symbol == "BTCUSD"` (case-insensitive)
  OR `USD Amount USD` present-and-non-empty. Any `Buy`/`Sell` row failing both checks emits `Unclassified`
  with `raw_of(row)` — never falls through to `usd_cost/proceeds = ZERO`, never guesses direction.
  KATs: `gemini_btcquoted_pair_buy_is_unclassified` (ETHBTC Buy → Unclassified, not Acquire, not zero-basis).
  §9.1 updated to state the rule.

- **I-2 — Gemini USD sign: magnitudes abs-normalized (FIXED).**
  Applied `.abs()` to `fee` at parse time in the Gemini parser and to `usd_abs` inside the Buy/Sell arm.
  `parse_usd` is unchanged (shared). A negative-encoded Buy no longer produces a negative `usd_cost`;
  a parenthesized Sell no longer produces a negative `usd_proceeds`. Applied only in `gemini.rs`.
  KATs: `gemini_negative_usd_normalized_to_positive` (negative USD Amount + parenthesized Fee → positive).

- **Phase-2 refinement note — full crypto↔BTC-pair FMV handling:** For a Gemini `ETHBTC` Buy/Sell the
  BTC leg IS a taxable disposition at FMV (or acquisition), but Phase 1 cannot auto-compute the BTC FMV
  for a non-BTCUSD pair without a second price lookup. These rows are conservatively emitted as
  `Unclassified` and require explicit user classification via reconciliation. Auto-recognizing the BTC
  disposition at FMV (e.g., by looking up the BTC/ETH rate from an exchange dataset) is a Phase-2
  refinement. — OPEN (Phase 2). — I-1 fix follow-on.

## btctax-adapters (Plan 3) — confirmed real schemas folded into §9.1 (2026-06-29)
- **CROSS-CRATE GAP — inbound `TransferIn` cannot carry cost-basis / acquisition-date (record clearly).**
  Swan `transfers` `deposit` rows carry **`USD Cost Basis` + `Acquisition Date`**, and Coinbase `Receive` /
  Gemini `Credit`(BTC) inbound rows may carry basis context, but core's
  `TransferIn { sat, src_addr?, txid? }` has **no field to hold a cost-basis or acquisition-date**. So at
  ingest every inbound on-chain row becomes a **plain `TransferIn`** and the exchange-supplied basis/date are
  **dropped from the event**. They must be **re-supplied by reconciliation (Plan 4)** — e.g. a
  `ClassifyInbound` decision (`GiftReceived{donor_basis, donor_acquired_at, …}`) or a future
  `ClassifyInbound`-style "external-acquisition" decision that records basis+date for an externally-sourced
  inbound. For a confirmed **self-transfer** the source lot is authoritative anyway (the Swan basis is only
  relevant for externally-sourced coins), so no data is lost there. **Candidate fix (Phase-2):** a
  reconciliation-hints side-table (or extra optional fields on `TransferIn`) so the adapter can persist the
  exchange-provided basis/date as a *hint* the reconciler can accept, instead of re-keying it by hand. —
  OPEN (Plan 4 reconciliation / Phase-2). — adapters §9.1 / plan FOUND GAP.
- **Swan withdrawals `source_ref` — native-vs-semantic owner question.** The confirmed withdrawals schema
  carries a `Transaction ID` column, but per the owner it is **not a stable per-row id** (the schema-only
  doc shows the column but not values; cf. Swan-trades' present-but-empty `Tag`). The adapter therefore
  treats withdrawals as **id-less** (synthesized `(source, direction, utc_ms, type, sat)` + occurrence_index,
  §6.2). If the withdrawals `Transaction ID` turns out to be stable/unique, switch to a native ref (one-line
  change). — OPEN (owner confirm). — adapters §9.1 / plan Schema-items.
- **Swan `Total/Transaction USD` purchase-cost semantics.** Swan transfers `purchase`→`Acquire` uses
  `Transaction USD` (principal) + `Fee USD` (fee), with `Total USD` as the basis cross-check (`Total ==
  Transaction + Fee`); confirm by fixture once real values are available. — OPEN (confirm). — adapters §9.1.
- **Coinbase internal-move default.** `Exchange/Pro Deposit/Withdrawal` (Coinbase↔Coinbase-Pro) are routed to
  `Unclassified` (likely self-transfers, but user-confirmed via reconciliation rather than auto-`TransferIn`/
  `TransferOut`). Confirm this conservative default is desired. — OPEN (owner confirm). — adapters §9.1.
- **XLSX-float→decimal precision bound; id-less `occurrence_index` file-order fragility** (River, Swan trades,
  Swan withdrawals, Gemini `Credit`/`Debit`) — both already noted; carry forward. **Pin** the resolved
  `csv`/`calamine`/`rust_xlsxwriter` versions + re-verify the `calamine::Data` variant list after first build.
  RESOLVED (versions pinned 2026-06-29): `csv` 1.4.0, `calamine` 0.26.1, `rust_xlsxwriter` 0.79.4.
  `calamine::Data` variant audit deferred to Task 2 (first build confirmed 0.26.1 resolves; no variant
  references in Task 0). — OPEN (Task 2 Data-variant audit). — plan Notes for Plan 4.
- **`AdapterError.source` field rename (thiserror compat, 2026-06-29).** The brief's `lib.rs` stub used
  `source: &'static str` (the adapter name) in `MissingColumn`/`Parse`/`FractionalSat` variants. Both
  thiserror 1.x and 2.x auto-treat any field named `source` as `Error::source()`, requiring `impl Error`.
  Field renamed to `adapter: &'static str`; format strings updated to `{adapter}`. Parse functions updated
  to construct with `adapter: source`. Display output unchanged. — RESOLVED (Task 0).

## Deferred to later phases (out of Phase-1 scope by design)
- **Forms generation (Phase 2):** filled IRS 8949 + Schedule D PDFs; §170(e) charitable-deduction computation (FMV vs basis); Form 8283 (>$5k qualified appraisal — §170(f)(11)(C), CCA 202302012); Form 709 routing for gifts. — *Phase 1 captures the metadata (FMV, ST/LT, appraisal-required, donor carryover) so Phase 2 can compute.* — OPEN (Phase 2). — tax-review N1/M-(donation), spec §16.
- **Rate/limit mechanics (Phase 2/3):** 0/15/20% (§1(h)), 3.8% NIIT (§1411), $3,000 loss limit + carryforward (§1211/§1212). — Confirmed safe to defer (downstream of per-lot basis/gain/ST-LT). — OPEN (Phase 2/3). — tax-review "Positions confirmed".
- **Self-employment tax routing (Phase 2):** business-vs-hobby mining → Schedule SE (Notice 2014-21 A-9). — *Phase-1 ledger tags `Income{Mining, business: bool}`; Phase 2 routes.* — OPEN. — tax-review N1.
- **Optimizer (Phase 3):** goal-driven specific-ID/HIFO/LIFO/loss-harvesting, bracket/NIIT-aware. — OPEN. — spec §16.
- **Non-BTC scope:** fork-coin income (e.g., 2017 BCH airdrop, RevRul 2019-24) and non-BTC dispositions are OUT of BTC-only scope and must be handled separately. — Acknowledged, not covered. — OPEN/won't-do-in-foundation. — tax-review M4.

## Deferred — precise Phase-2 tax refinements (Phase-1 over-approximates safely)
- **`appraisal-trigger-precision` — Qualified-appraisal trigger precision.** **[cycle-prep 2026-06-29 correction:** the earlier claim "Phase 1 flags `appraisal_required` on FMV>$5k (over-flag)" is FALSE — there is NO auto-computation; `appraisal_required` is a raw **user-supplied CLI boolean** on `reconcile reclassify-outflow … donate` (`main.rs` → `OutflowClass::Donate{appraisal_required}`). The earlier "§16" pointer is also wrong (§16 is the impl-order list).** The precise §170(f)(11)(C) trigger is a **claimed deduction > $5,000**, aggregating similar items in a year (§170(f)(11)(F)); for §170(e)-reduced property (≤1-yr / ordinary-income) the deduction is limited to **basis**, so a high-FMV short-term donation with basis ≤ $5k would not trigger an appraisal. Computing the exact trigger requires the *claimed-deduction* (= §170(e) deduction computation), which is itself Phase-2. **No Phase-1 action.** — OPEN (Phase 2; depends on deduction computation). — TP10, spec fold-record R3/TAX-N2.
- **§1015(d) gift-tax basis increase.** A donee's basis is bumped by gift tax paid attributable to net appreciation (§1015(d)). Rare for personal BTC gifts (mostly under the annual exclusion); omitted in Phase 1, noted for completeness. — OPEN (won't-do unless needed). — tax-review R3 N3; spec §15.

## btctax-store — whole-branch fix I-1 (owner-only perms) — deferred hardening
- **M-1: `open`/`recover_target` bak-on-corrupt.** `recover_target` restores from `.bak` only when the target is MISSING, not when it is present-but-corrupt. Consider retrying from `.bak` on decrypt/decode failure — but must NOT retry on `WrongPassphrase` (caller error, not corruption). Deferred hardening; overlaps the kill-mid-save fuzz-harness item. — **RESOLVED (SPEC_store_hardening T2, 2026-07-05).** `Vault::open` now retries from `.bak` on GENUINE corruption only (`Crypto`/`Corrupt`/deserialize-`Sqlite`) via a shared `decode_conn` helper; `WrongPassphrase` AND [R0-C1] `UnsupportedSchema` (a NEWER vault — recovering would DOWNGRADE) propagate untouched; recovery WARNs + does a `.bak`-PRESERVING crash-safe restore (`restore_from_bak`, never clobbers `.bak`). KATs: `open_recovers_from_bak_when_target_genuinely_corrupt`, `open_unsupported_schema_never_recovers_from_bak`, `open_wrong_passphrase_never_touches_bak`, `open_both_corrupt_propagates_and_bak_intact`, `restore_preserves_bak_and_is_crash_safe`. — I-1 fix follow-on.
- **M-2: save-path plaintext not zeroized.** The `db_to_bytes`/`encode_blob` `Vec`s produced during `save()` hold plaintext before encryption and are not zeroized on drop. Within the accepted R1 bound (SQLite heap already holds plaintext all session). Future: wrap in `SecretBuf`/zeroize after `encrypt_to`. — **RESOLVED (SPEC_store_hardening T1, 2026-07-05).** `save()` (image + `encode_blob` output), `export_snapshot` (image), and `backup_key` (armored key) now wrap their plaintext intermediates in `SecretBuf` (mlock + scrub on drop). Honest bound documented on `save()`: defense-in-depth (shrinks copy count/lifetime), NOT full at-rest secrecy — the live SQLite connection holds plaintext all session; the on-disk `.tmp`/`.bak` are ciphertext. `snapshot()` intentionally NOT wrapped (its only Vec is the caller-owned FR10 return). — I-1 fix follow-on.
- **M-3: Windows owner-only perms — verify under CI.** All four sinks (`vault.key`, `vault.pgp`, `export_snapshot`, `backup_key`) now use the non-Unix ACL-inheritance path (no explicit DACL). Verify under Windows CI that the written files are not world-readable. — OPEN (CI). — I-1 fix follow-on.

## btctax-store (Plan 1) — deferred hardening (non-blocking; plan is review-green)
- **Password zeroization (Task-3).** Resolved: `sequoia-openpgp::crypto::Password` wraps `Encrypted`, which stores the plaintext in a `Protected` buffer. The `Protected` type implements `Drop` with `memsec::memzero` — the ciphertext (encrypted plaintext) IS zeroized on drop. The `salt` field in `Encrypted` is NOT explicitly zeroized, but it is a key-derivation salt, not the actual secret. Confirmed — Password zeroizes (Protected buffer). — RESOLVED (2026-06-28). — Task-3.
- **OS-crash mid-first-create residual.** A `kill -9`/power-loss between the `vault.key` write and the first `vault.pgp` rename leaves `vault.key` present + `vault.pgp`/`.bak` absent → `create`→`AlreadyExists`, `open`→`Io(NotFound)`; manual key deletion needed (no committed data lost). In-process failures are cleaned up. Add an OS-level kill-mid-save fuzz harness and/or treat "key present, pgp+bak absent" as a half-created vault to auto-repair. — **RESOLVED (kill-mid-save harness) (SPEC_store_hardening T3, 2026-07-05).** The "key present, pgp+bak absent" half-created signature is now a typed `HalfCreatedVault` error (auto-`repair`able); and `kill_mid_save_state_enumeration_open_is_always_safe` deterministically enumerates `vault.pgp`×`.bak`×`.tmp` ∈ {absent,good,corrupt}³ (key present) and asserts `open` is always safe (valid vault OR a specific `StoreError`, never panic/silent-wrong) with a good `.bak` always recovering + surviving the C2 crash-window. A true OS-level `kill -9`/power-loss harness (real interrupt injection) remains a deferred FOLLOWUP. — plan-review R3 M2.
- **Lock file persists after a failed/`AlreadyExists` create** (lock-first; conventional flock pattern, lock files are never unlinked). Harmless. — WONTFIX/ack. — plan-review R3 N1.
- **Sequoia/S2K pin (R3) — CONFIRMED by Task-0 spike:** sequoia-openpgp `1.21` resolved to `1.22.0`; backend `crypto-nettle`. Spike confirmed secret-key S2K = `Iterated { hash: SHA256, hash_bytes: 65011712 }` (i.e. `0x3E00000`, max OpenPGP work factor, ~354 ms) — no Argon2 in 1.22, strongest available = high-work-factor iterated-salted SHA-256, satisfying spec §8. Both primary key and subkey carry this S2K. Revisit if a future Sequoia exposes Argon2 or a public S2K-work-factor setter. — RESOLVED/confirmed (2026-06-28). — plan-review R2/R3 + Task-0.
- **nettle 4.0 system incompatibility (CONCERN, non-blocking for now):** The dev machine has system nettle 4.0, but `nettle-sys-2.3.2` + `nettle-7.5.0` require nettle 3.x API (functions removed/renamed, SHA3 init symbols gone, digest callback arity changed). Build workaround: extracted cached `nettle-3.10.2-1.1-x86_64_v3.pkg.tar.zst` from pacman cache to `/tmp/nettle-3.10.2/`, set `PKG_CONFIG_PATH=/tmp/nettle-3.10.2/pkgconfig-custom LD_LIBRARY_PATH=/tmp/nettle-3.10.2/usr/lib` when running cargo. This workaround is session-local and NOT committed. Future task: either (a) wait for a new `nettle`/`nettle-sys` crate supporting nettle 4.0, (b) install nettle 3.x system-wide, or (c) switch to `crypto-rust` backend (pure Rust, no system lib dependency) for CI portability. Per task-0-brief, no silent backend switch; this is an explicit concern. — OPEN. — Task-0 report.
- **Two on-disk artifacts** (`vault.pgp` + `vault.key`) and the vault is **encrypted but not signed** — documented deviations from §8's single-artifact wording (NFR2 still holds; `vault.key` is S2K-encrypted). Sign-on-save is a future option. — ack. — plan-review R1 M2/M8.

## btctax-store — cross-platform + crypto-rust (user decisions 2026-06-28)
- **Target OS = Linux + macOS + Windows (NFR8).** Store crate abstracts OS primitives: single-instance lock via `fs2` (flock/LockFileEx); secret-memory lock via `rustix` mlock (Unix) / `windows-sys` VirtualLock (Windows); atomic save via `std::fs::rename` (POSIX atomic / Windows MoveFileEx-replace, with the fsync'd `.bak` as the safety net). Spec NFR8 + §8 + plan Tasks 0/4/5/6 updated. — RESOLVED (decision). — user OS choice.
- **Crypto backend = `crypto-rust` (pure Rust)** — supersedes the earlier `crypto-nettle` choice. Reasons: (a) the dev box's nettle 4.0 is incompatible with `nettle-sys` (the Task-0 hack is no longer needed/used); (b) NFR8 cross-platform (Windows can't use nettle). `crypto-rust` needs no system crypto lib on any OS. **Security trade-off accepted:** Sequoia labels RustCrypto variable-time / "not recommended for general use"; acceptable for local at-rest single-user encryption (no network/oracle exposure). `allow-variable-time-crypto` enabled. The Task-0 nettle-4.0 concern above is **SUPERSEDED** by this switch. — RESOLVED (decision). — user backend choice.
- **Cross-platform validation:** Linux is the dev/test OS; Windows/macOS code paths are `cfg`-gated and compile-checked but executed under per-OS CI (set up later). — OPEN (CI). — NFR8.
- **crypto-rust builds clean (no system crypto lib, nettle hack unused):** `cargo build -p btctax-store` + `cargo test --test smoke` pass with `features = ["crypto-rust", "allow-variable-time-crypto", "allow-experimental-crypto"]` and no `PKG_CONFIG_PATH`/`LD_LIBRARY_PATH` set; S2K = `Iterated{SHA256, hash_bytes=65011712}` confirmed unchanged under crypto-rust. `allow-experimental-crypto` is required (sequoia-openpgp build script gates RustCrypto behind it). — RESOLVED (2026-06-28). — Task-0 crypto-rust switch.
- **File-lock crate: `fs2` 0.4 (dormant ~2017) vs `fd-lock` (maintained).** We use `fs2::try_lock_exclusive`; on Windows it relies on Rust ≥1.64 mapping `ERROR_LOCK_VIOLATION(33)`→`WouldBlock` (MSRV 1.74 satisfies). `fd-lock 2.x` normalizes this explicitly and is maintained — preferred swap if Windows CI shows any mapping issue or if the dormant dep becomes a supply-chain concern. — OPEN (monitor; swap candidate). — plan-review delta M-1.

## btctax-core (Plan 2) — review-green; deferred Minors to address at implementation
- **TP8(c) loss-basis cross-lot edge (tax m1).** When a fee spans lots and `relocated.last()`/last removal-leg is non-dual-basis but the fee originates on a dual-basis received-gift lot, the carry's `loss_basis` fragment is dropped. Effect: future loss-zone basis understated by fee-cents (taxpayer-conservative); gain basis fully conserved. — OPEN (Task 11). — core tax-review R2 m1.
- **TP8 fee exact-boundary holding-period attribution (tax m2).** When principal consumes exactly to a lot boundary, the fee basis (from the next, later-acquired lot) rides the earlier relocated lot's holding period by a few cents. De-minimis; total basis conserved. — OPEN (Task 11). — core tax-review R2 m2.
- **Degenerate `principal==0` fee'd transfer (tax m3).** Carry is silently dropped (no relocated lot/leg) with no blocker — unreachable for real TransferLink/gift (principal>0). At implementation: assert principal>0 or raise `uncovered_disposal` instead of dropping. — OPEN (Task 11). — core tax-review R2 m3.
- **2025-transition seed timezone straddle (eng Minor).** The boundary seed fires on the UTC-sorted timeline while pool routing + `universal_snapshot` use the tax-date; a sub-day offset straddling 2025-01-01 (e.g. a +12:00 post-2025 event vs a −05:00 pre-2025 event) can fold a pre-2025-tax-date event after the seed → fails safe (`uncovered_disposal` or stranded lot), but `universal_snapshot` won't match the real fold's pre-seed residue. At implementation (Task 12): partition the timeline at the **tax-date** boundary (or seed lazily on first wallet route) + add a reversed-offset KAT. — OPEN (Task 12). — core eng-review R2 Minor.
- **`allocation_voids` declaration (eng Nit).** Referenced (pass-1 step 1a, deferred from Task 7) with `.target`/`.void_id` but its struct/collection isn't formally declared in the plan; declare it explicitly at implementation. — OPEN (Task 7/12). — core eng-review R2 Nit.

## Standing notes / decisions (informational)
- **PGP KDF tradeoff (user-mandated PGP retained).** Engineering review suggested age / XChaCha20-Poly1305+Argon2id as simpler with a stronger KDF; **declined — PGP is a hard user requirement.** Mitigation: protect the app-managed private key with the strongest S2K the chosen Sequoia version supports (Argon2 S2K if available, else high-work-factor iterated-salted S2K). — RESOLVED (decision) / monitor. — eng-review YAGNI, spec §8/§15.
- **TP8 self-transfer fee = treatment (c) default, config-switchable to (b) mini-disposition.** User-mandated default; do not flip. Contrary signal: §1.1012-1(h)(2)/(h)(4) (fees-in-crypto have disposition consequences in the *taxable-exchange* context; no on-point guidance for a pure self-transfer). — RESOLVED (decision). — spec TP8, memory `self-transfer-fee-treatment-c`.
- **Daily-close FMV is an approximation** of the "date and time of dominion & control" standard (RevRul 2023-14). Documented convention; revisit if higher precision is needed. — RESOLVED (decision) / monitor. — spec §9.2, tax-review M3.
- **`pre2025-filed-method-reconciliation` — Pre-2025 lot method = FIFO (legal default).** **[cycle-prep 2026-06-29 correction:** the advisory note ALREADY EXISTS — `BlockerKind::Pre2025MethodNote` (state.rs, Advisory severity) is emitted by `note_pre2025_once` (fold.rs) on any pre-2025 disposal, and `verify` already surfaces it. The earlier text implied it was unimplemented.** The Phase-1 advisory ("FIFO assumed; reconcile if your filed pre-2025 returns used a different method") is **DONE**. What is genuinely OPEN is a *runtime reconciliation mechanism* (taxpayer declares the filed method → engine adjusts the reconstructed carryforward basis), which does not exist and is a Phase-2 feature (needs a brainstorm to scope: method-declaration config + basis adjustment). — note DONE / reconciliation mechanism OPEN (Phase 2). — spec §7.4, eng-review I-2.
- **Source-priority tiebreak (Swan>Coinbase>Gemini>River)** is arbitrary-but-stable for same-instant cross-source FIFO ties; documented as such. — RESOLVED (decision). — spec §6.2, eng-review n-2.
- **Id-less-source `source_ref` fragility (River).** For sources without native ids, `source_ref = (source, direction, utc_ms, type, sat)` with a last-resort `occurrence_index` for exact duplicates in one import. Two known limitations: (a) `occurrence_index` shifts if a corrected re-export inserts an earlier row; (b) a re-export that edits a *constituent* field (e.g., `sat`) changes the `source_ref`, so it is NOT detected as a "same source_ref, changed content" conflict and cannot be auto-`SupersedeImport`-ed (old event orphans, new appears). — OPEN (documented limitation; prefer time-resolution / native ids where possible). — spec §6.2, eng-review round-2 m-2/m-5.
- **Daily-close FMV (labeled M3)** — see the "Daily-close FMV is an approximation" note above; flagged as the chosen convention vs the date-and-time dominion-and-control standard. — RESOLVED (decision). — tax-review M3.

## Resolved in SPEC v0.2 (folded round-1 reviews)
See the spec's "Fold record (v0.2)" section for the 1:1 mapping of each Critical/Important to its fix. Round-1 reviews: `reviews/spec-review-phase1-tax-round-1.md`, `reviews/spec-review-phase1-engineering-round-1.md`, `reviews/architecture-review-phase1-foundation-round-1.md`.

- **N-2 (export_snapshot silently overwrites snapshot.sqlite):** Current behaviour matches the brief (no mention of rotation); future improvement: timestamped filenames (e.g. `snapshot-20260628T120000Z.sqlite`) to avoid clobbering a previous export. **Windows owner-only perms** for both `export_snapshot` and `backup_key` rely on user-profile directory ACL inheritance (no explicit DACL set); verify under Windows CI that the written files are not world-readable.

## btctax-adapters plan — deferred Minors (review-green; 2026-06-29)

Non-blocking items raised during the round-1 review of `btctax-adapters` (IP-1 and all code-level Minors folded inline into the plan on 2026-06-29). These are deferred observations for implementation time or later phases.

- **River `Income`→`IncomeKind::Reward` documentation + `business: false` immutability (tax M1/M2).** River's `Income` tag maps to `IncomeKind::Reward` (non-business yield/reward); `business: false` is hard-coded at ingest. At implementation, add a module-doc note that `business: false` is immutable at the adapter layer — the Plan-4 reconciler cannot flip it without a re-import. If the owner's River income is business income (e.g., from professional mining operations), the `IncomeKind` / `business` mapping must be confirmed before implementing the River parser. — OPEN (confirm at River-parser implementation). — adapters tax-review M1/M2.
- **Swan zero-sat-withdrawal defensive counter (tax Nit).** The Swan withdrawals arm currently increments `dropped_no_btc` for a `sat == 0` row (defensive guard; Swan is BTC-only). At implementation, consider whether a zero-sat Swan withdrawal should be counted under a separate `skipped_zero_sat` field rather than the FR2 `dropped_no_btc` counter, since the two cases are semantically different. — OPEN (implementation note). — adapters tax-review Nit.
- **Coinbase internal-move = Unclassified decision (tax-review endorsed).** `Order` + `Exchange/Pro Deposit/Withdrawal` → `Unclassified` is the correct conservative default. The tax reviewer explicitly endorsed keeping this (over auto-routing to `TransferIn`/`TransferOut`), since these Coinbase↔Coinbase-Pro internal moves require user confirmation via reconciliation. No change to the plan; noted here so Plan-4 docs know the decision is reviewed and intentional. — RESOLVED (decision retained; no action needed). — adapters tax-review.
- **Swan withdrawals `Transaction ID` stability — treated id-less; confirm later.** The withdrawals file carries a `Transaction ID` column but the adapter treats it as non-stable (semantic `source_ref`). If confirmed stable/unique, switch to native ref (one-line change in `Swan::normalize` withdrawals arm). Cross-referenced with the existing schema-items entry above. — OPEN (owner confirm). — adapters plan Schema-items / tax-review Nit.

## btctax-core (Task 0) — dependency versions pinned for reproducibility
- btctax-core pinned `rust_decimal` 1.42.1 / `rust_decimal_macros` 1.40.0 (independent Cargo entries; `dec!` literals binary-compatible with the 1.42 `Decimal`) / `time` 0.3.51 — R3 pin record.

## btctax-cli plan (Plan 4) — deferred items from round-1 reviews (2026-06-29)

Non-blocking items raised in the round-1 reviews of `IMPLEMENTATION_PLAN_foundation_04_cli.md`
(`reviews/plan-foundation-04-cli-engineering-round-1.md`,
`reviews/plan-foundation-04-cli-reconciliation-round-1.md`). The blocking findings (C1, I-1, I-2/Eng-I1,
M3, N-2) were folded into the plan (see its "Fold record (round 1)"). These remain open.

- **M-2 (recon) — `AllocLot` carries no `dual_loss_basis` → a pre-2025 received-GIFT lot loses its
  §1015(a) dual basis under Path B.** A safe-harbor `SafeHarborAllocation.lots` entry is
  `{wallet, sat, usd_basis, acquired_at}` — single-basis. So when a pre-2025 gift lot (which under TP11
  carries a separate loss-basis = donor basis vs gain-basis = FMV-at-gift) is re-seeded via Path B, the
  loss-leg basis collapses to the single `usd_basis`. This is **spec-faithful** (the spec defines
  `AllocLot` without a dual-basis field), and Path A (the default) preserves the dual basis correctly, so
  the loss only arises when a taxpayer *elects* Path B over a gift lot. Effect: a future loss-zone
  disposition of that seeded lot could mis-state basis. **Phase-2 refinement:** extend `AllocLot` (and the
  Path-B seed in `transition::seed_transition`) to carry `dual_loss_basis` + `donor_acquired_at`. — OPEN
  (Phase 2; spec change required). — recon review M-2.

- **M-1 (recon) / M1 (eng) — `verify` double-loads events.** — **RESOLVED (burndown 2026-06-29, commit 39e09e0):**
  added `Session::load_events_and_project()`; `verify` + `safe_harbor_attest` routed through it. See the
  burndown section above.

- **eng-M2 — render + CSV use `{:?}` (Debug) for enums.** — **RESOLVED (burndown 2026-06-29, commit 282ae20):**
  six stable snake_case `*_tag()` fns; all CSV writers + text renderers switched off `{:?}`; export test
  asserts column values. CSV columns are now a committed contract. See the burndown section above.

- **N-1 (recon) — strengthen the `set-fmv` test.** — **RESOLVED (burndown 2026-06-29, commit 4a78727):**
  repointed to an FMV-missing `Income` target; asserts the `FmvMissing` blocker present before and cleared
  after `set-fmv` (+ income recognized at the manual FMV). See the burndown section above.

- **attest leaves a stale `safe_harbor_timebar` advisory (follow-on of the I-2 fold).** — **RESOLVED**
  (the CLI-I2 whole-branch fix made `safe_harbor_status` prefer the effective-Path-B signal over the advisory;
  the burndown fix (commit f6880e6) extended that signal to disposal/removal legs for the all-lots-consumed
  case). `verify` no longer mislabels an effective Path B as time-barred. See the burndown section above.

## Sub-project A (lot-id substrate) — whole-diff review deferrals (2026-06-29, round 2 residuals)
- **N2 — `evaluate_disposal` `lots_after` semantics for C.** Confirm the returned post-disposal lots/outcome shape is what Sub-project C (optimizer + Mode-2) needs before C consumes it. — OPEN (C planning).
- **N3 — B per-year hard-blocker gate.** B must refuse a TaxResult / C must refuse to optimize for a tax year with unresolved Hard blockers (basis-pending/uncovered/LotSelectionInvalid/etc.). — OPEN (B planning).
- **M3 binary-dispatch test.** The `config` multi-flag apply-all + attest-guard are tested at library level, not by driving the real clap `Command::Config` arm; add a binary-level dispatch test to fully retire the Task-5 note. — OPEN (B/C or a CLI test pass).

## Sub-project B (rate/NIIT/loss engine) — whole-diff review deferrals (2026-06-30)
- **F1 (Nit) — money "0" vs "0.00" display.** Load-bearing figures (ltcg_tax/niit/total) are round_cents-scaled and always print cents; descriptive level fields inherit source scale → cosmetic inconsistency. Add a `fmt_money` (`{:.2}`) render helper. — OPEN (polish).
- **Minor — `MarginalRates.niit_applies` doc vs code.** Doc says "MAGI exceeds threshold"; code computes "crypto increased NIIT" (niit_with>niit_without). Display-only, feeds no figure. Align doc or rename. — OPEN.
- **B-M1 (Phase-2) — minimal NII model can understate NIIT** in loss years (NII excludes crypto ordinary income + not reduced by §1211 loss). Disclosed in output. Phase-2 refinement. — OPEN.
- **Nits (DEFER):** unused `events` param in compute_tax_year; redundant rust_decimal_macros dev-dep (adapters); `{:?}` filing_status in tax-profile --show; advisory-only→Computed KAT; B-R2-N1 stale §4.3 doc line. — OPEN (cosmetic/doc).

## Sub-project C (optimizer) — Task-4 review Nit deferred (2026-06-30)

- **Nit — `proposed_compliance_status` / `persistability` asymmetry for divergent contemporaneous 2027+
  broker picks.** `proposed_compliance_status` returns `NonCompliant` for a selection that diverges from the
  current pick AND was made at/before the sale date (`made ≤ sale`, i.e. contemporaneous) when the wallet is a
  2027+ broker-held account. `persistability` returns `ContemporaneousNow` for the same inputs (made ≤ sale
  is the only criterion for `persistability`; the 2027+ broker check is only in `ForbiddenBroker2027`). This
  means the status says "NonCompliant" while the persistability gate says "persists freely" — an unusual
  combination that a caller would see only for a future-dated existing disposal to a 2027+ broker where the
  optimizer proposes a pick that differs from the current selection. In practice, the CLI's Task-10
  2027+ broker refusal prevents this path from being reached (the CLI refuses to persist any divergent pick
  for 2027+ brokers regardless of persistability). A one-line alignment (either widen
  `proposed_compliance_status` to return `NonCompliant` from `persistability == ForbiddenBroker2027` even
  for contemporaneous picks, OR add a `ForbiddenBroker2027` arm to `Persistability` and let the CLI check
  that instead of `persistability == ContemporaneousNow`) would remove the conceptual gap. — **RESOLVED
  (whole-diff-review fold, 2026-06-30):** `persistability` now tests the 2027+ broker envelope FIRST, ahead
  of the `made ≤ sale` contemporaneous branch, so a 2027+ broker lot is categorically `ForbiddenBroker2027`
  (never `ContemporaneousNow`) regardless of timing — matching `proposed_compliance_status` (which already
  returned `NonCompliant` ahead of the contemporaneous branch). Both core functions now agree, and `accept`'s
  gate categorically refuses these even when `made ≤ sale` (no own-books-insufficient 2027+ broker record can
  persist). Covered by `persistability_broker_2027_contemporaneous_is_forbidden`,
  `persistability_broker_pre_2027_contemporaneous` (regression), and the end-to-end
  `accept_refuses_2027_broker_contemporaneous_divergent_no_write` (synthetic TY2027 table; fails without the
  fix). `crates/btctax-core/src/optimize.rs` (`persistability`).

## Sub-project C (optimizer) — whole-branch review round 1 deferrals (2026-06-30)

Source: `reviews/whole-branch-review-optimizer-round-1.md` (VERDICT: READY TO MERGE — 0 Critical / 0
Important). The review's one MUST-FIX-before-TY2027-table item (the `persistability`/`proposed_compliance_status`
2027+ broker asymmetry) was folded this cycle (see the Task-4 nit above, now RESOLVED). The remaining three
new Minors are non-blocking and deferred here.

- **M-1 (Minor) — exact-tie tie-break can emit a `delta == 0` divergent pick.** In `exhaustive_min`
  (`crates/btctax-core/src/optimize.rs`, the `total == best_total && assign < best_assign` branch) a candidate
  that TIES the baseline total but is lexicographically smaller than `baseline_assignment` evicts the baseline
  incumbent (`best_total` stays `== base.total`). Result: `best != baseline_assignment` with `delta == 0`, so a
  disposal with two equal-basis/equal-term lots can yield `proposed != current` at zero tax benefit → `run`
  shows a "change … needs `--attest`" line for no benefit, and a future-dated (`made ≤ sale`) disposal would let
  a bare `accept` auto-persist a no-benefit divergent `LotSelection`. **No invariant is broken** (`delta = 0` is
  shown, the pick is gated/legally valid, the reported optimum is still a true minimum) — it is needless churn /
  a pointless attestation prompt. The lex-smallest tie-break is the spec'd §0 total order, so this is a quality
  choice, not a correctness bug. *Recommend* preferring the baseline on an exact tie (evict only on
  `total < best_total`). — OPEN (non-blocking polish).

- **M-2 (Minor) — Mode-2 `consult_sale` discards the `candidate_selections` heuristic flag.**
  `crates/btctax-core/src/optimize.rs` binds `let (cands, _heuristic) = candidate_selections(&lots, req.sell_sat)`.
  For a wallet pool > `LOT_ENUM_BOUND` (12) — common for weekly-DCA / active-trading wallets — the candidate set
  is a deterministic INCOMPLETE subset, so the proposed selection may not be the true tax-minimum, with NO
  disclosure (unlike Mode-1's `PoolHeuristic` banner). Mitigation: `ConsultReport` has no `approximate` field and
  the renderer hedges ("read-only what-if", "proposed selection", "federal tax attributable (estimated)") rather
  than claiming "the optimum" — so it is NOT a false-global claim (hence Minor). The plan scoped R2-C1's
  disclosure to Mode-1. *Recommend* a parallel "heuristic — searched a subset of a large pool" note in
  `render_consult` for symmetry. — OPEN (non-blocking; add a consult-level approximate disclosure later).

- **M-3 (Minor) — the optimizer's "global" excludes self-transfer lot-selection; scope undocumented.**
  `optimize_year` (`crates/btctax-core/src/optimize.rs`) targets only `baseline_state.disposals`; SelfTransfers
  produce no Disposal/Removal record, so a same-year self-transfer's lot routing is held at its baseline. Spec
  §A.3 lists SelfTransfer as method-honoring and says it "lets the optimizer pre-position lots," so a user could
  read "proven global minimum" (`approximate == false`) as including self-transfer re-routing. In practice the
  available-lots pools are still correct (the real fold, incl. self-transfers at baseline, is replayed), and
  self-transfers are non-taxable so they affect the single-year objective only indirectly via an uncommon
  intra-year move-then-sell pattern — hence Minor. The `approximate == false` "global" claim is global over
  taxable-disposal selections only. *Recommend* documenting the scope boundary in the proposal footer (mirroring
  the R0-M2 vertex-granularity caveat); relates to A's open `disposal_compliance`-omits-SelfTransfers item. —
  OPEN (non-blocking; document the scope boundary vs spec §A.3).

---

## ORACLE-SWEEP whole-branch residue (owning phase: ownerless residue; batch post-ship)

Filed 2026-07-16 at the `feat/oracle-sweep` whole-branch review (Fable, Ready-to-merge YES, 0C/0I). All Minor/Nit,
none weakens a comparison or hides a defect (whole-branch review verified: frozen files untouched, hermetic ~8s
gate, no caught-bug pins, three consumers non-drifted, corpus regenerates byte-identically, fresh-seed live sweep
clean). Burn down opportunistically; none holds any gate.

- **OS-WB-1 (Minor) — deeper-line teeth prove only the OTS witness.** All §12 teeth KATs perturb the OTS leaf;
  deleting the `if let Some(tc)` taxcalc block for deduction/SALT/Sch-D→L7 (`golden_packet.rs:496,513,524`;
  `golden_returns.rs:394-404`) reddens nothing. Every line keeps a proven-biting OTS witness, so no comparison
  goes blind — only the redundant second (taxcalc) witness is unproven. Fix: one taxcalc-leaf perturbation case
  per level.
- **OS-WB-2 (Minor) — read-back fault-injection re-implements the L16 compare inline** rather than driving
  `diff_household` over a swapped map (`golden_packet.rs` `readback_reads_the_pdf_not_the_struct`). Spec-blessed
  map-swap shape; a follow-up could parametrize `diff_household` over the 1040 map to close the residual.
- **OS-WB-3 (Minor) — `check_determinism.py::compare` has no top-level catch-all** — checks `households` +
  `_provenance` only; a future new top-level corpus key could drift unnoticed. One `set(na)==set(nb)` + equality
  line closes it.
- **OS-WB-4 (Nit) — `Sign::Unsigned` (`common/mod.rs:187`) accepts a leading minus** that `paper_money` rejects;
  an assert would mirror the parse discipline (currently only unit-test-exercised).
- **OS-WB-5 (Nit) — doc/comment cosmetics:** stale comment `golden_packet.rs:341` ("provenance leaves are None
  until T11" inside Part-2, contradicted by the correct "LIVE at T11" six lines below); `corpus.py:311` says
  "5 income axes" (injection list has 4); anchor-error attribution wording; `sweep.py::_verify_harness_freshness`
  attributes exit-2 solely to a stale binary (could also be schema drift).
- **OS-WB-6 (Nit) — harness generic `paper` closure (`main.rs:216`) parses a leading minus on any line;** the
  paper level is strict per the sign table. Not a masking risk (a wrong sign diverges from the oracle target).
  Add a one-line comment on the deliberate asymmetry when T6-m2's `on_paper_signed`-for-negative-AGI-L11 switch
  is revisited.

(Also open, filed T8, out of THIS plan's scope: **OS-14.2** — derive OTS's 8995-L12 from OTS's own Schedule-D
output to close the QBI-path single-witness/WEAK gap. Correctly labeled WEAK at every consumer.)

---

## USAGE-EXAMPLES cycle (owning phase per entry)

Filed 2026-07-16 during the usage-examples brainstorm (design of record: `design/usage-examples/
BRAINSTORM_usage_examples.md`). UX-P0-1 was surfaced by the determinism recon and ruled on by an
independent Fable architect (`design/usage-examples/reviews/fable-clock-seam-ruling.md`). Owning phases
are hard: a phase-owned item burns down in/before its owning phase, never batched to the end.

- **UX-P0-1 (Important — PHASE-GATING) — the CLI has no deterministic clock seam; wall-clock `now` leaks
  into stdout.** Owning phase: **P0** (gates all goldens). The single read at
  `crates/btctax-cli/src/main.rs:66` (`OffsetDateTime::now_utc()`) becomes each decision's stored
  `utc_timestamp`, which surfaces in the **clock-derived** read surfaces: `verify` (MethodElection
  `recorded` date, `render.rs:2258`), the `reconcile bulk-void` preview (`session.rs:1134` →
  `main.rs:2005`, over `voidable_decisions`), and the `config --set-forward-method` made-date
  (`cmd/reconcile.rs:968` ← `now` from `main.rs:470`) — all in `btctax-cli`, not `btctax-core`. **NOTE
  (corrected r0-review I4):** `reconcile bulk-resolve-conflict` (`session.rs:1097`) and
  `match-self-transfers` (`session.rs:1183`) are **CSV-derived / deterministic** and must NOT be used as
  seam-proof surfaces. This blocks golden-diff of any decision-bearing journey (exactly the
  bug-rich surface). Fix = a CLI-only `BTCTAX_NOW` (RFC3339) seam, fallback `now_utc()` when unset,
  malformed⇒exit 2, unconditional stderr banner, integrity KAT + man-page misuse language, gated by the
  (i)/(ii)/(iii) determinism-prerequisite fence. **Burn down in P0 before the first golden is recorded;
  NOT deferrable past P0.** — **RESOLVED** (2026-07-16): seam shipped (`e5a182f` Task 0.1 + `27b43f7`
  Task 0.2, integrity KAT + man-page misuse language); independent Fable P0 review GREEN 0C/0I
  (`reviews/p0-fable-review.md`); full suite green (1940).
- **UX-P3-1 (Important) — the TUI has ~30 wall-clock reads incl. an on-screen timestamped export-dir
  path.** Owning phase: **P3** (Artifact-2 / TUI-doc design). `btctax-tui/src/lib.rs:247,256` (`:256` →
  `export_dir_for` at `export.rs:30`, rendered on screen) + `btctax-tui-edit` ~28 reads. Blocks
  deterministic TUI text-capture; needs a shared clock helper — its own P3 prerequisite. Do NOT stretch
  P0's CLI seam to cover it. Burn down in/before P3. — **DISCHARGED** (2026-07-18, ledger reconciliation):
  P3 built the shared clock helper (`btctax_tui::clock::Clock {Wall, Pinned}`) + the style-aware capture
  harness (`capture.rs`), and routed the `btctax-tui` reads (`export_dir_for`/`lib.rs`) and the ~23
  `btctax-tui-edit` wall-clock sites through the injected clock. Held structurally by the
  `persisted_decision_made_date_is_the_injected_clock` guard + the `no_direct_now_utc_in_production`
  scans (export.rs, tui-edit/main.rs), with byte-gated TUI goldens (browse + classify-confirm-modal).
  Independent Fable P3 review GREEN 0C/0I (I-3 mutation-proven fold). Nothing left to fix.
- **UX-P1-1 (Minor) — capture-convention discipline for the CLI goldens.** Owning phase: **P1**. (a) Exit
  codes are output — `verify` returns 1 on hard blockers (`main.rs:89-91`); goldens + the twice-run
  hygiene test must assert exit codes, not just stdout. (b) `init`/`import` echo `vault.display()` /
  key-backup paths → fix a cwd + relative-path invocation convention. (c) Front-matter states the
  pinned-env convention (`BTCTAX_NOW`, `BTCTAX_PASSPHRASE`, `BTCTAX_PRICE_CACHE`→nonexistent) + one honest
  sentence that captures use `BTCTAX_PASSPHRASE` where a real user sees an interactive prompt.
- **UX-P1-2 (Minor — pre-existing product doc bug, surfaced by the SPEC r0 review; §3.1-fence class).**
  Owning phase: **P1**. `export-irs-pdf`'s clap/man help still says the command is "REFUSED for a tax year
  that has FULL-RETURN inputs … Transcribe the report's figures by hand until the full-return fillers
  ship" (`cli.rs` doc comment ~`:182`), but the runtime now dispatches to the full-return packet
  (`admin.rs:216-227`). J6 demonstrates the full-return export, so the shipped doc set would contain a man
  page contradicting a transcript. Wording fix only (fails the (i)/(ii)/(iii) fence → NOT an inline edit
  in the docs cycle; file + own in P1). Bundle with **N3**: `cli.rs:197-198`'s doc comment writes
  "form-8283"/"form-1040" while the actual `--forms` clap values are `form8283`/`form1040`.
- **UX-P0-3 (Minor — pre-existing product drift, surfaced by running `make check` during P0 execution;
  already FIXED).** Owning phase: **P0** (fixed in-branch). The v0.6.1 release (`57e468c`) bumped crate
  versions 0.6.0→0.6.1 but did **not** regenerate the man pages (last regenerated at v0.6.0, `4c9b1c2`),
  so committed `docs/man/btctax-update-prices.1` said `v0.6.0` while the crate is `0.6.1` →
  `gen_docs_is_deterministic` (`docs.rs:353`) is RED on `main`. Fixed by `cargo run -p xtask -- docs`
  (one line: `v0.6.0`→`v0.6.1`). **Process finding:** the release ritual must regenerate man pages on any
  version bump (the man pages embed `CARGO_PKG_VERSION`), same as the golden-regen ritual — folded into
  the plan's release "Version bump" step so the v0.7.0 bump can't repeat it.
- **UX-P1-3 (Minor — UX papercut, surfaced authoring J2 §170(e) donation; bug-hunt payoff).** Owning
  phase: **P1** (file; do not inline-fix — engine/UX). `reconcile reclassify-outflow … --as-kind donate
  --amount <X>`: `--amount` is the **USD proceeds/FMV** (→ `ro.principal_proceeds_or_fmv` →
  `Op::Donate.fmv`, `project/resolve.rs:332-338`), but the clap def is a bare `amount: String` with **no
  doc comment** (`cli.rs:539-540`) and the name is ambiguous (sats? BTC? USD?). Passing the satoshi count
  (`200000000`) is silently accepted and yields a **$100,002,000** §170(e) deduction (1 BTC's sat count
  read as USD) with **no warning** — a footgun for a figure that lands on Form 8283/Schedule A. Fix (P4/
  later): a `--amount` doc comment naming the unit (USD FMV) + a sanity guard (an FMV wildly exceeding
  `sat/1e8 × recent-close` warns). NOT a correctness bug in the engine (the math is right *given* the
  input); it is an input-contract/affordance gap.

- **UX-P1-4 (Minor — UX papercut, surfaced authoring J6 full-return export; bug-hunt payoff).** Owning
  phase: **P1** (file; do not inline-fix — engine/UX). On the **full-return** `export-irs-pdf` path the
  handler unconditionally prints the crypto-slice header `Filled IRS forms for tax year {y} →\n  {list}`,
  but the five slice paths (`f8949_path`, `schedule_d_path`, `schedule_se_path`, `form_8283_path`,
  `form_1040_path`) are all `None` there (the packet lands in `full_return_paths`), so the list is EMPTY —
  the output is a bare `Filled IRS forms for tax year 2024 →` followed by a blank indented line, THEN the
  real `Full-return packet — 14 form(s):` block (`main.rs:626-672`). Redundant/confusing noise before the
  authoritative listing. Fix (P1/later): gate the "Filled IRS forms →" header on `!written.is_empty()` (or
  merge the two headings) so the full-return path prints only the packet block. NOT a correctness bug (the
  packet + manifest are written correctly); it is a presentation wart, now captured verbatim in the J6
  golden.

- **UX-P1 reconciliation (2026-07-18, entering the P1 review gate).**
  - **UX-P1-1 — DISCHARGED by the P1 implementation.** (a) Exit codes ARE captured: `emit()` writes an
    `[exit N]` marker on any non-zero code (`examples.rs`), the whole golden is byte-gated by
    `examples_golden_matches_committed`, and the double-run hygiene test pins determinism — so an exit-code
    change reds the gate. (b) `init`/`import` run under a cwd + relative-path convention (relative `--vault
    v.pgp`/`--out irs`, `HOME=cwd`), so echoed paths are deterministic. (c) `front_matter()` states the
    pinned-env convention + the honest interactive-passphrase sentence. Nothing left to fix.
  - **UX-P1-2 / N3 and UX-P1-4 — CONFIRMED fence-barred; the FIXES are re-owned OUT of the docs cycle.**
    Per SPEC §3.1 the fence explicitly lists "rewording a message" as failing the trichotomy → routed to
    FOLLOWUPS, never an inline docs-cycle edit. P1's ownership was to SURFACE them (done; J6 concretizes the
    UX-P1-2 man-page-vs-reality contradiction and captures the UX-P1-4 empty header verbatim). The wording
    fixes are re-owned to a **pre-v0.7.0 product-wording cleanup** (a separate reviewed change, landing with
    the release's man-page regen so the shipped doc set is coherent) — flagged to the P1 Fable review as a
    release-gating consideration, NOT a P1 deliverable.
  - **UX-P1-3 — already re-owned** ("Fix (P4/later)"); a `--amount` guard is a behavior change (fence-barred
    from the docs cycle). Parked correctly.

- **UX-P1-5 (Minor — fence-barred; surfaced by the P1 Fable review M-2).** Owning phase: **P1** (file; do
  not inline-fix — product JSON/serde). `income show` renders a date of birth as the raw serde
  `(year, ordinal-day)` tuple: `"date_of_birth": [2012, 106]` (in J6's `income show` block of the golden) —
  a filer cannot read "day 106 of 2012" as 2012-04-15. Same class as UX-P1-4 (a presentation wart captured verbatim in the
  golden). The committed TOML fixture a user is invited to imitate carries the same `date_of_birth = [2012,
  106]`. Fix (pre-v0.7.0 wording/UX cleanup): render `time::Date` as `MM/DD/YYYY` in `income show` (and,
  optionally, accept that form on `income import`). Not a correctness bug; a display wart.
- **UX-P1-6 (Minor — fence-barred; surfaced folding the P1 review M-3 on J2).** Owning phase: **P1** (file;
  do not inline-fix — product message). For a Section B donation that spans **≥ 2 lots**, every non-first
  Form 8283 property row is UNCONDITIONALLY `needs_review` (`forms.rs:426` — subsequent rows carry no
  appraiser/donee identity block), so the export's stderr advice "at least one donation needs REVIEW … Run
  `btctax reconcile set-donation-details …` to complete it" is **misleading**: re-running set-donation-
  details (even fully, as J2 now does with `--appraiser-qualifications`) can never clear it — the extra row
  is completed on the paper form, not in the vault. J2 now frames this in prose; the tool's message should
  distinguish "your inputs are incomplete" (actionable) from "additional property rows need manual
  completion" (inherent). Fix: pre-v0.7.0 wording cleanup.
- **✅ UX-P1-7 DONE (`f043872`, #20 Phase 6; Fable-reviewed GREEN 0C/0I).** New journey **J7** classifies an
  unknown-basis 2024 Coinbase Receive as staking income via `reconcile classify-inbound-income <ref> --kind
  staking --fmv 3300.00` (the single-event path has no auto-valuation — the review reproduced that omitting
  `--fmv` leaves a Hard FmvMissing blocker even for a bundled-dataset date), verify clears, report shows the
  crypto ordinary income. Was: (Minor — docs coverage gap; SPEC §15 r2 (a)).
- **✅ UX-P1-8 DONE (`f043872`, #20 Phase 6; Fable-reviewed GREEN).** New journey **J8** matches a
  cross-exchange self-transfer: a River Withdrawal (out) + Coinbase Receive (in), no-arg `match-self-transfers`
  PREVIEW → confirm `--in/--out` → RELOCATE (basis + HP carry), ledger BALANCED. Was: (Minor — docs coverage
  gap; SPEC §15 r2 (d)).
- **✅ UX-P1-10 DONE (`f043872`, #20 Phase 6; Fable-reviewed GREEN).** New journey **J9** makes a genuine
  per-disposal `reconcile select-lots` — two lots + a 0.50 sale (< holdings) so the default splits both lots;
  select-lots identifies the whole 0.50 against the cheap LT `lot-a`, recorded as contemporaneous per-disposal
  compliance. Was: (Minor — docs coverage gap; SPEC §15 (e), whole-branch review I-1).
  - **↳ N1 (Nit, non-gating; Phase-6 Fable review) — J9's before/after lot split is never DISPLAYED.** Owning
    phase: **future docs cycle**. The refs live in `snapshot/disposals.csv` the reader opens; the generator by
    design only emits btctax commands, so showing a file excerpt needs new machinery. Prose is truthful and
    points at the file. If a future mechanism ever shows file excerpts, J9 is the first customer.
  - **↳ N3 (Nit, non-gating; Phase-6 Fable review; PRE-EXISTING, outside #20's diff) — FmvMissing hint
    wording.** Owning phase: **future legibility cycle**. The `[FmvMissing]` hint "no local price for this date
    — run `btctax-update-prices`" fires even when the BUNDLED dataset has a close for that date (the hint refers
    only to the cache). Reword to distinguish "no bundled/cached price" from "single-event path needs `--fmv`".
- **P1 plan-conformance drift record (P1 review M-6; no code change).** Recorded so the plan's Task shapes
  aren't silently contradicted: (a) the three gate tests live as `#[cfg(test)]` unit tests in
  `crates/xtask/src/examples.rs`, not the plan's `crates/xtask/tests/examples_golden.rs` — xtask is bin-only
  (no lib target); functionally equivalent, both run under `make check`. (b) the §4.2 CSV corpora are
  embedded CRLF `const`s, not committed `.csv` files — `.gitattributes` `* text=auto eol=lf` would LF-
  normalize a committed CSV and break the Coinbase parser (follows `fixtures.rs`). (c) Task 1.2's "a
  `cargo test` asserting each corpus imports without a hard blocker" is covered transitively by the golden
  gate (each journey's real import is captured), not a dedicated test. (d) `tempfile` is a regular (not dev)
  dependency of xtask because the non-test `run()`/`generate()` path needs it — fine, xtask is
  `publish = false`. The J6 fixture lives in `btctax-cli/tests/fixtures/` (the published crate, self-
  contained), with xtask holding the cross-crate `include_str!` (M-5 fold).
- **UX-P1-9 (Nit — non-gating; P1 re-review-2 N-C).** Owning phase: **fold at the release-bump golden
  regen** (the front matter is a docs-cycle artifact, not fence-barred). The front-matter stderr clause
  says the elided `BTCTAX_NOW` banner is "determinism scaffolding, not btctax output" — loose: the binary
  DOES print the banner (the sentence itself concedes "prints to stderr"). Reword to e.g. "the seam's own
  reproducibility notice, not part of a command's result." Meaning is unambiguous in context; recorded so
  it is tightened at the next mandatory golden regen (the v0.7.0 version-pin bump) rather than forcing an
  extra review round now.
- **✅ UX-P3-2 DONE (`34bf318` + fold `1d762fe`, #21 Phase 7; Fable-reviewed r1 0C/2I → r2 GREEN).**
  `tui-wrap.awk` now colorizes the TUI PDF from the goldens' style runs (per-cell `\m[fg]` + `\f[CB]` bold),
  mapping every multi-byte glyph 1:1 to ASCII for cell-accurate coloring. r1 folded I-1 (run range is 0-based
  end-exclusive → paint `start+1..end`, no left-bleed) and I-2 (the multi-byte bracket class broke under
  mawk = CI's awk → per-glyph gsubs, byte-mode-proven). Makefile `\m[` guard catches a monochrome
  regression. Also (user-reported): both `make examples`/`examples-tui` render LANDSCAPE + wrap long lines so
  nothing runs off the page (verified by rasterizing every page). Residue: N-r2-1 (emit_pre `\e`-split,
  latent), N-r2-2 (`unbraced_decl` peek misses `pub(crate)` vis-quals, latent) — both record-only.
- **N-2 (P2 review) — RESOLVED in P3.** The TUI goldens (`docs/examples-tui/*.txt`) are staleness-gated
  in-process by the crates' `*_goldens_match_committed` tests (which the `test` job runs), so no git-diff
  widening of the CI `examples` job was needed; that job instead gained a `make examples-tui` PDF-build proof.
- **✅ N-R1 DONE (`41a771a` + M-1 fold `1d762fe`, #21 Phase 7; Fable-reviewed GREEN).** Both
  `no_direct_now_utc_in_production` scans (btctax-tui + btctax-tui-edit) de-stuck: extracted a pure
  `production_now_utc_lines` helper that bounds each `#[cfg(test)]` module by its DEDENTED (column-0) close
  and resumes production scanning after (brace-counting was rejected — `{`/`}` in string/char literals
  corrupt a depth count). A production `now_utc()` after a test module is now caught. r2-M1 fold: an unbraced
  `#[cfg(test)] mod X;` no longer sticks the scan (peeks the next line). New de-stick + unbraced KATs in
  each crate, mutation-proven. Residue N-r2-2 (vis-qualified unbraced decls) record-only.

- **✅ UX-P2-1 DONE (`81d220b`, #20 Phase 6; Fable-reviewed GREEN 0C/0I).** `is_demonstrated` now skips
  leading global options (a `-`-prefixed flag + the value of value-taking `--vault`) and ANCHORS `path[0]`
  to the first subcommand token (exact match, not a free subsequence); sub-verbs still subsequence-match
  after the anchor. A subcommand named only as an argument (`reconcile void import`) or a flag value
  (`v.pgp`) no longer over-reports. 3 platform-agnostic `matcher_tests` pin it (the two over-report cases
  RED on the old matcher — the review re-killed that mutant). **↳ N4 residue (Nit, non-gating):** the
  post-anchor tail is still a free subsequence, so a hypothetical `reconcile void <arg-named-like-a-sibling
  -sub-verb>` line could over-report a sibling `reconcile <x>` leaf; no such collision exists in the current
  golden (review checked all 47 leaves) and the code comment states the tail behavior. Owning phase: future
  docs cycle if a collision ever arises. Was: (Minor — P2 review M-2; future-drift).
- **P2 review nits (recorded).** **N-2 (owning phase P3):** `ci.yml`'s drift gate diffs only
  `docs/examples/examples.md`; SPEC §9 writes `git diff … docs/examples docs/examples-tui` — equivalent
  today (no TUI golden yet), but **P3 must widen the CI diff to `docs/examples-tui/` when the TUI golden
  lands**. **N-3 (no code):** plan Task 2.2 cites `crates/xtask/src/examples/mod.rs`; the code is at
  `crates/xtask/src/examples.rs` (P1's actual bin-only layout) — citation drift only. M-1 (silent 0/N on a
  missing golden) and N-1/M-3 (nested-build `--locked` + `Stdio::null()`) were FOLDED in the P2 gate commit,
  not deferred.

- **UX-P1-6 extension (2026-07-18, P4 workaround-audit).** The unconditional non-first-property-row
  `needs_review` ALSO fires on the **Section A** path: a sub-$5,000 donation spanning ≥ 2 lots warns
  "needs REVIEW … Run `btctax reconcile set-donation-details …`" on every export even AFTER
  set-donation-details is fully completed (the advice loops) — and Section A requires no appraiser
  declaration at all, sharpening the misleadingness. A single-lot Section A donation with complete
  details clears clean (root cause pinned to the same rule). Fold into the UX-P1-6 wording fix.

## P4 workaround-audit findings (2026-07-18; design of record: `design/usage-examples/reviews/tutorial-workaround-audit.md`)

All fixes below are §3.1-fence-barred from the docs cycle (behavior/wording changes). Owning phases:
UX-P4-2 joins the **pre-v0.7.0 product-wording cleanup** batch (one stale string; ships with the
release's man-page regen); everything else is owned by the **first post-v0.7.0 product cycle**
(UX-P4-1 first among them). Any fix that changes captured output must regen the goldens (the gate
will red until it does).

**✅ BURNDOWN — Phase 8 close (2026-07-19): ALL of UX-P4-1 … UX-P4-12 below are DONE + Fable-reviewed
to 0C/0I on `feat/post-v070-product-cycle`**, across the cycle's phases (verbose per-item text
retained below for the record):
- UX-P4-1 (pseudo banner on `report --tax-year`) + UX-P4-4 (record-time value guards: negative basis /
  acquired>receipt / non-TIN EIN) — correctness cluster #14 (`cmd/tax.rs:251`; `b2a9f7d`/`13e1704`).
- UX-P4-2 (self-transfer modal long-term-default string) — folded (`draw_edit.rs:931-933`, comment
  records "was backwards").
- UX-P4-3 (record-time decision validation + unified conflict hints) — #14 (`990f786`, r1→r2).
- UX-P4-11 (`btctax events list`) — #18 (`f338e6b`).
- UX-P4-7/8/9 (legible payloads / path+hint IO errors / insufficient-balance) — #15 (`fa4badc`,
  `66b4bad`, `34e9945`; r1→r2).
- UX-P4-6/10 (holdings pending line / `report --tax-year` exit code) — #16 (`20b2a58`, `027a89d`;
  r1→r2).
- UX-P4-5 + UX-P4-12(b–i) (`--forms`-ignored warn + the grouped wording/affordance papercuts, incl.
  (i) the table-less-year reassurance) — #17 (`8544621` + `981f45d`/`10331e9`/`fd60f1d`/`f80426c` +
  `bd73968`→`c2597ad`; r1→r3).
- UX-P4-12(a) (`--kind` valid-values list) — DONE: the runtime error enumerates them
  (`eventref.rs:161` "bad income kind {s:?} — expected one of: mining, staking, interest, airdrop,
  reward"), fixed in the pre-v0.7.0 wording cycle; the help lists them too (`cli.rs:533`).
- M-1 / UX-P1-7/8/10 / UX-P2-1 / UX-P3-2 / N-R1 — #19/#20/#21 (already marked ✅ in their own entries).
Residue (non-gating, parked with later owners): the UX-P1-* wording Nits, UX-P1-9, and the pathless-io /
legibility-polish Nits below.

- **UX-P4-1 (Important — pseudo-mode loud-flag gap).** Owning phase: **post-v0.7.0 product cycle
  (top priority)**. With `reconcile pseudo on` and a synthetic default contributing, `report
  --tax-year` prints a clean, authoritative-looking tax summary with **no `[PSEUDO]` marker or
  banner** — repro: a vault whose sale consumes a pseudo-classified lot reports
  `TOTAL federal tax … 4041.50` where the entire LT gain rides on the deliberately-fictional
  $0-basis/LT-default lot. Bare `report` DOES flag the lot + disposal rows `[PSEUDO]`; `verify`
  discloses `[PseudoReconcileActive]`; export is blocked behind the attest phrase — the one silent
  surface is the primary number-bearing one, violating the mode's own "loudly-flagged on-screen
  estimate" contract (the answered-ness class). Fix: an unconditional pseudo banner (and/or
  `[PSEUDO]`-suffixed totals) on `report --tax-year` whenever the projection is pseudo-contributed.
- **UX-P4-2 (Important — TUI modal states a rate-determining default backwards).** Owning phase:
  **pre-v0.7.0 product-wording cleanup**. The classify-inbound self-transfer confirm modal renders
  `acquired_at: (empty = default = receipt date, short-term)` (`btctax-tui-edit/src/draw_edit.rs:927`),
  but the engine persists `long_term_default_acquired` = **1 year + 1 day before receipt → LONG-TERM**
  (`btctax-core/src/project/fold.rs:1024`), as the CLI help + `SelfTransferInboundDefaultedAcquired`
  advisory correctly disclose. Verified end-to-end: confirming on a 2025-05-23 receipt persists
  `Acquired 2024-05-22` (Holdings tab). The modal is the informed-consent point for the vault write;
  its statement of the holding-period default is the opposite of what persists. Fix: one string.
- **UX-P4-3 (Minor — record-then-conflict false success + inconsistent remedy hints).** Owning
  phase: post-v0.7.0. The classify/reclassify verbs accept a typo'd ref, a wrong-type ref, or a
  duplicate re-decide with `Recorded decision decision|N` (exit 0); the error surfaces only on the
  next `verify` as a `DecisionConflict` HARD blocker. `reconcile void decision|99` (nonexistent)
  also "succeeds" and becomes its own hard blocker (`void targets unknown event`) cleared only by
  voiding the void. Hint text is inconsistent across variants (some carry "void the decision to
  clear this blocker"; the void-of-unknown carries none; the unknown-event ReclassifyIncome hint
  suggests the wrong verb for a typo). `set-donation-details` already validates at record time —
  proving feasibility. Fix: validate the target ref/type/duplicate at record time (warn or refuse),
  and unify DecisionConflict remedy hints. Conservative posture is intact (verify gates) — the cost
  is a false success + a void round-trip, not a wrong number.
- **UX-P4-4 (Minor — value-validation gaps; extends UX-P1-3's input-contract class).** Owning
  phase: post-v0.7.0. (a) NEGATIVE basis accepted on both surfaces — CLI
  `classify-inbound-self-transfer --basis=-5000.00` (the `=` form bypasses clap's `-`-prefix error)
  and the TUI form (rejects `abc` as `bad USD`, permits `-5000`) — and it flows into gain math
  (`basis -5000.00 → gain 26799.23` > proceeds, verified via what-if). (b) `--acquired` AFTER the
  receive date accepted silently (factually impossible for a self-transfer-in; the lot is then also
  invisible to what-if sales before that date). (c) `set-donation-details --donee-ein banana
  --appraiser-tin fruit` → saved; lands on Form 8283. Fix: reject negative USD, acquired > receipt,
  and non-TIN-shaped EIN/TIN at record time.
- **UX-P4-5 (Minor — `--forms` silently ignored on a full-return year).** Owning phase:
  post-v0.7.0. With full-return inputs present, `export-irs-pdf --tax-year 2024 --forms f8949`
  writes the whole 14-form packet with no notice that the explicit slice request was disregarded.
  Distinct from UX-P1-2 (whose stale help describes a THIRD behavior — refusal) and UX-P1-4 (the
  empty header). Fix: honor `--forms` on a full-return year, or refuse/warn that it is ignored.
- **UX-P4-6 (Minor — bare `report` renders a fully-pending vault as empty).** Owning phase:
  post-v0.7.0. With the whole balance inside a pending outbound transfer, `report` prints
  `Holdings: none / Lots: none / Disposals: none` — indistinguishable from an empty vault, no
  pending line, no pointer — while `verify` shows `pending 200000000` / `Pending reconciliation: 1`.
  Fix: a `Pending: N sat (M unreconciled transfer(s) — see verify)` line in the holdings view.
- **UX-P4-7 (Minor — raw Debug dumps in decision summaries).** Owning phase: post-v0.7.0. The CLI
  `bulk-void` preview and the TUI void list + bulk-void preview render
  `… as SelfTransferMine { basis: Some(19000.00), acquired_at: Some(2026-01-01) }`
  (`btctax-tui-edit/src/main.rs:3742` formats the payload with `{:?}`; the TUI column truncates it
  mid-field: `{ basis: Non`). The repair surface — where every UX-P4-3 mistake sends the user — is
  the least legible one. Fix: a human summary formatter (`basis $19,000.00, acquired 2026-01-01`).
- **UX-P4-8 (Minor — bare io errors without path or hint).** Owning phase: post-v0.7.0. A missing
  or wrong `--vault` yields `error: io: No such file or directory (os error 2)` — no path, no
  "check --vault / run `btctax init`"; an `--out` colliding with an existing file yields
  `error: io: File exists (os error 17)` — no path. In-house precedent exists: `import` names the
  file (`io reading nope.csv: …`). Fix: attach the path + a one-clause hint at the vault-open and
  export-out call sites.
- **UX-P4-9 (Minor — false "no lots available" on mere insufficiency).** Owning phase:
  post-v0.7.0. `what-if sell --sell 0.6` with 0.5 BTC held → `no lots available to sell from that
  wallet as of that date` — "no" is false, the available balance is not shown, and genuine-zero vs
  insufficient collapse into one message. Fix: `only X BTC available in <wallet> as of <date>
  (requested Y)`.
- **UX-P4-10 (Nit — `report --tax-year` exits 0 on NOT COMPUTABLE).** Owning phase: post-v0.7.0.
  The refusal is loud in text but invisible to scripts; `verify` exits 1 on hard blockers, `report`
  never does. Decide + document an exit-code contract (nonzero on NOT COMPUTABLE, or an explicit
  "informational, always 0" statement in the man page).
- **UX-P4-11 (Minor — event-ref discoverability is a workaround, not an affordance).** Owning
  phase: post-v0.7.0. No `list`-refs verb exists; the sanctioned discovery path is export-snapshot
  CSV columns (`set-donation-details --help` and `select-lots --help` both say so) or stripping the
  trailing `#0` split-suffix from `report`'s lot ids; the Income section prints no refs at all
  (J4's refs embed a ms-timestamp a user cannot construct by hand). Concrete trap, reproduced:
  pasting the tool's own displayed lot id (`…#0#0`) into `reclassify-income` records a decision
  that then hard-blocks as "targets unknown event" (UX-P4-3 compounds it). Fix: print event refs
  beside income/disposal rows in `report` (or add `btctax events list`), and say "lot id = event
  ref + `#split`" wherever a lot id is shown.
- **UX-P4-12 (Nit — grouped wording/affordance papercuts).** Owning phase: post-v0.7.0 (fold
  opportunistically into any adjacent wording cleanup). Itemized: (a) bad `--kind` → `bad income
  kind "lemonade"` with no valid-values list (contrast `--as-kind`'s clap enum listing); (b)
  `reconcile classify-inbound-income` / `set-fmv` args have BLANK help and no units (contrast the
  exemplary `what-if sell --help`, which disambiguates sats/BTC); (c) `config` SETS the forward
  method but won't SHOW it — read-back only in `verify`'s Standing-orders block; (d) the
  `tax-profile --year` set-error never mentions `--show`; (e) internal enum names on screen:
  `TreatmentC`, `Hifo`, `:: non_compliant`; (f) "press 'v'" TUI keybinding language inside CLI
  `verify` advisory text; (g) `set-donation-details` before `reclassify-outflow` points at
  removals.csv (circular — the removal isn't there either) instead of the missing prior step; (h)
  TUI footer dev-speak `q: swallowed` (wraps mid-word at 120 cols); (i) the TUI editor defaults to
  year 2025, whose full-return commit then refuses ("2024 only") — a late gate on the default year,
  and the opposite gate placement from the CLI (which stores 2025 inputs and gates at export).

  **RESOLUTION (#17 Phase 4, 2026-07-19):** (a) DONE pre-v0.7.0. (b) DONE `4dd51e1` (units + kind
  values on classify-inbound-income/set-fmv). (c) DONE `10331e9` (`config` echoes the forward-method
  standing order). (d) DONE `981f45d` (tax-profile set-error names `--show`). (e) DONE `fd60f1d`
  (config human labels — TreatmentC/Hifo gone; the `non_compliant` tag is a documented vocabulary,
  left as-is). (f) DONE `981f45d` (surface-neutral void remedy). (g) DONE `981f45d` (points at the
  reclassify-outflow prior step). (h) DONE `f80426c` (dropped "q: swallowed").
  **(i) DONE (2026-07-19, user-decided — supersedes the SPEC §4(i) "align to CLI" default).** Investigation
  revealed the papercut's premise was incomplete: the TUI editor ALREADY stores an in-progress return
  for a table-less year — the `return_inputs_draft` side-table (INVISIBLE to `resolve.rs`, so no
  poisoning) is autosaved as the filer edits, flushed on quit, and reloaded next session (draft SHADOWS
  committed, §6.1). The reviewed **I-11 guard** blocks only the FINALIZE/commit (the engine-visible
  committed row), which is conceptually correct (you can't finalize/compute a return in a year whose
  tables don't exist). USER DECISION (see [[full-return-store-before-tables-policy]]): people author
  returns all year before tables publish, so authoring must not be blocked — and it isn't (drafts).
  The real defect was the blunt commit-refusal message reading as REJECTION. FIX (kept the I-11 guard,
  no poisoning risk): `commit_tax_inputs`'s `NoTables` arm now PERSISTS the working return to the draft
  and shows a reassuring status — "{year} has no full-return tables yet (v1: TY2024) — inputs SAVED as
  a draft; finalize when tables publish." (kept ≤ ~104 chars so the whole line — including the "finalize"
  clause — renders on the no-wrap NOTICE line; see (i) r1-M1 below). Committed `tax_inputs_commit_non_2024_...`
  KAT pins: committed row still absent (guard KEPT) + draft PERSISTS + status reassures (SAVED/draft/finalize)
  + a saved draft is not dirty; all mutation-proven. The SPEC §4(i) "default: align to the CLI" is
  superseded by this user decision (keep the guard; fix the message) — `[T-U-P4-12]` trivially held
  (no packet-export path touched).

  **(i) independent review trail (`reviews/ux-p4-12i-impl-fable-review-r{1,2}.md`, r1 0C/1I → r2 GREEN):**
  - **I-1** (r1, Important): the persisted user-decision record (memory doc + MEMORY.md index) recorded the
    OPPOSITE prescription (align-to-CLI / reverse I-11) vs the implemented decision — a live mandate to
    reverse a reviewed guard. FOLDED (`c2597ad`): memory doc + index rewritten to KEEP-I-11 with the
    align-to-CLI framing marked SUPERSEDED; r2 whole-dir grep confirms no reversal mandate survives.
  - **M-1** (r1, Minor): the 162-char reassurance truncated on the no-wrap NOTICE line. FOLDED: shortened
    to ~104 chars; render KAT strengthened to require BOTH "SAVED as a draft" AND the late "finalize"
    clause on-screen; r2 re-killed the long-message mutant (clips "finalize", reds the KAT).
  - **N-1** (r1, Nit): `form.dirty` left set after a clean draft-save. FOLDED: `saved`-gated dirty-clear +
    (r2-N1) a non-vacuous `!form.dirty` assert (dirty precondition) — mutation-proven.
  - r2 residue (non-gating): this stale-quote fix + the CONTINUITY "(i) DEFERRED" flip (done in this touch);
    N-2 memory-quote fragment aligned.

  **r1 review fold (2026-07-19, `reviews/ux-p4-12-impl-fable-review-r1.md`, 0C/4I → folded):**
  - **I-1** (b `--fmv` help falsely claimed a daily-close fallback — omitting `--fmv` actually fires a
    Hard FmvMissing blocker; there is NO auto-valuation on the single-event path): help corrected +
    man regenerated; `help_units.rs` now pins the real behavior + the absence of the false claim.
  - **I-2** (c a per-account SCOPED standing order was read back as vault-wide): `ElectionLine` gained
    the `wallet` scope; config now states the vault-wide method explicitly (global order else FIFO)
    then attributes scoped orders to their account; new scoped-order KAT.
  - **I-3** (g suggested invalid syntax): now `reclassify-outflow <out-ref> --as-kind donate --amount
    <usd> …`; KAT pins `--as-kind donate`.
  - **I-4** (UX-P4-5 warning emission unpinned): new process-level KAT spawns the binary and asserts
    the stderr warning fires with `--forms` / not without, and the written packet FILE-SET is identical.
  - **M-1** (e missed `Hifo` sites): fixed the `main.rs` scoped-set confirmation (`attests {:?}`→label)
    + the bulk-void MethodElection summary (`{:?}`→label); both pinned. The "(e) Hifo gone" claim is
    now accurate vault-wide.
  - **M-2** (f second advisory unpinned): the defaulted-acquired advisory's surface-neutral wording is
    now asserted too.
  - **NEW (Nit, later polish — from I-1's repro):** the `FmvMissing` blocker's remedy hint ("no local
    price for this date — run `btctax-update-prices`", `price_cache.rs:14` via `render.rs`) is attached
    even when a local price DOES exist for the date; misleading. Sweep with the legibility residue.

  **r2+r3 review fold (2026-07-19, `reviews/ux-p4-12-impl-fable-review-r{2,3}.md`, r2 0C/2I → r3 GREEN):**
  - **r2-I-A** (the r1-corrected `--fmv` help's remedy was ALSO wrong — a bare re-classify is refused by
    the first-wins record-time guard): now `reconcile void <decision-ref>` then re-classify; man regen;
    help_units pins `reconcile void` + `first-wins`. Reproduced working live by r3.
  - **r2-I-B** (config forward-method read-back picked the vault-wide order by max `decision_seq` and
    claimed a wrong "FIFO default"): REWRITTEN to use the shared `project::in_force_methods` resolver
    (same as `fold::applicable_method`; HIFO is the real default); new multi-order KAT + clock-controlled
    `run_config_at`; nothing hidden (full history deferred to `verify`).
  - **r3-M1 (Minor, FIXED here)** — the stale "FIFO default" doc-comments in core that CAUSED r2-I-B
    (`resolve.rs:160/205`, `session.rs:583`) corrected to HIFO (verified against `fold.rs:43`).
  - **r3-N1 (Nit, later polish)** — config's "N standing order(s) recorded" excludes voided while
    `verify`'s "Standing orders (MethodElection): M" includes them (labeled `[voided]`); reconcile later.
  - **r3-N2 (Nit)** — `config` uses `now.date()` (BTCTAX_NOW's own offset) where made-dates use
    `to_offset(UTC).date()`; a non-UTC BTCTAX_NOW could skew the displayed "as of" by a day. Unreachable
    in shipped docs/tests (all pin `Z`).
  - **r3-N3 (Nit)** — "1 standing order(s)" lacks plural inflection (cosmetic).

## Pre-v0.7.0 product-wording cleanup — FOLDED (2026-07-18, user-authorized before the release)

A deliberate, reviewed product-fix cycle (distinct from the fence-barred docs work; the user chose "fix
the wording batch first" over shipping v0.7.0 with the stale strings open). The gating + coherence + the
cheap error-message items are FIXED and their goldens/man pages regenerated; the rest — error-model /
affordance / feature changes, none of which appears in any shipped golden — are RE-OWNED to post-v0.7.0.

**FIXED in this cycle (goldens regenerated, `make check` green):**
- **UX-P4-2** (Important, M-1's release condition) — the TUI classify self-transfer modal now states the
  default acquired-date correctly ("1 yr + 1 day before receipt → long-term", was "receipt date,
  short-term"); `draw_edit.rs`. Classify-confirm-modal golden regenerated.
- **UX-P1-2 / N3** — the `export-irs-pdf` help now describes the full-return dispatch (was "REFUSED for
  full-return years … transcribe by hand"), and the `--forms` values are named correctly (`form8283`/
  `form1040`, not `form-8283`/`form-1040`); `cli.rs`. Man page regenerated.
- **UX-P1-4** — the empty "Filled IRS forms →" header is suppressed on the full-return path (gated on a
  non-empty slice list); `main.rs`. J6 golden regenerated.
- **UX-P1-5** — `income show` renders each date of birth as `MM/DD/YYYY` (was the raw `[year, ordinal]`
  serde array), display-only; `cmd/tax.rs`. J6 golden regenerated.
- **UX-P1-6** (+ Section-A/multi-lot extension) — the Form 8283 "needs REVIEW" advice now distinguishes a
  missing detail (`set-donation-details`) from a multi-lot gift's extra property rows (completed on the
  paper form); `main.rs` (both export paths). J2 golden regenerated.
- **UX-P1-9** — the front-matter stderr-elision clause reworded ("the seam's own reproducibility notice,
  not part of a command's result"); `examples.rs`. CLI golden regenerated.
- **UX-P4-12(a)** — `parse_income_kind`'s bad-kind error now lists the valid kinds; `eventref.rs`.

**RE-OWNED to post-v0.7.0** (behavior/error-model/affordance/feature — NOT pure wording, and none is in a
shipped golden, so deferring leaves no stale string in v0.7.0): UX-P4-8 (io errors need path context at the
vault-open/export-out call sites), UX-P4-9 (insufficient-balance needs the core to carry the available
amount — an error-model change), UX-P4-10 (exit-code contract), UX-P4-11 (a new `events list` verb),
UX-P4-12(b)–(i) (blank arg help/units, `config` read-back, enum-name display, TUI keybinding language in a
CLI advisory, the circular set-donation-details hint, TUI footer dev-speak, the TUI year-gate placement).

**Wording-cleanup review residue (2026-07-18, all NON-gating — the review was 0C/0I):**
- **M-1 (Minor) — DISCLOSED + accepted.** The UX-P1-5 fix routes `income show` through `serde_json::to_value`
  to host the DOB transform, which re-orders every object's keys ALPHABETICALLY (BTreeMap-backed `Value`;
  no `preserve_order`) instead of the struct's declared field order — the real cause of the large J6 golden
  hunk. Value-identical, deterministic, display-only, never parsed. Accepted (disclosed at `cmd/tax.rs` +
  here). Field-order restoration (`serde_json` `preserve_order`, weighing the `indexmap` transitive cost) is
  a **post-v0.7.0** candidate.
- **N-1 (Nit) — FIXED.** The `draw_edit.rs` UX-P4-2 comment's decaying `cli.rs:526-527` line citation was
  replaced with a symbol reference.
- **N-2 (Nit) — FIXED.** The slice-path 8283 advisory gained the "NOT filing-ready as written." tail for
  symmetry with the full-return advisory (J2 golden regenerated).

**UX-P4-4 impl review r1 residue (2026-07-19, `reviews/ux-p4-4-impl-fable-review-r1.md`):**
The 3 Important findings were FOLDED to green (I1: TUI negative-money guards; I2: TUI
acquired-after-receipt guard; I3: mandated `--sell=-1` + ad-hoc trio + per-flag wiring KATs).
Minors/Nits fixed inline: **M1** (SPEC amended — bare-9 `--appraiser-tin` acceptance + the
`donation_details::set` choke-point correction now recorded in §3.3(c)); **M2** (pin-cite
`1.170A-1(c)(2)`→`(c)(1)` in `cli.rs`/`reconcile.rs`/SPEC + man regen); **M3** (`tz_label` non-UTC
unit test added); **N3** ((d) warn line "USD FMV"→"USD proceeds/FMV"). Filed (owning phase =
post-release UX / ownerless residue — none gates, none is in a shipped golden):
- **M4 (Minor)** — a donation-detail TIN/EIN/PTIN refusal surfaced in the TUI edit form names the CLI
  flag (`--appraiser-tin …`), because the message comes from the shared `donation_details::set` choke
  point. Recoverable (the FieldForm stays open). Fix: thread a field-label context into
  `validate_and_normalize`, or accept the mismatch (the flag name still identifies the field).
- **N1 (Nit)** — `donation_details.rs`'s "§6695A PTIN" comment shorthand: §6695A is the appraiser
  *penalty* section; the PTIN authority is §6109(a)(4)/Reg. §1.6109-2. Pre-existing repo shorthand
  (`donation.rs`); comment-only — keep it out of user-facing text.
- **N2 (Nit)** — `is_ptin_shape` refuses a lowercase `p` (spec-literal `P\d{8}`); uppercasing before
  the check would be friendlier.
- **N4 (Nit)** — a seconds-only tz offset (e.g. `+00:00:30`) renders as `UTC` in the receipt-date
  message (documented minute-resolution behavior; pathological input; message-only).

**UX-P4-4 impl review r2 residue (2026-07-19, `reviews/ux-p4-4-impl-fable-review-r2.md`):**
The one Important (r2-I1: the two `what-if harvest` guard sites had no wiring rows) was FOLDED —
`value_guard_wiring.rs` now covers all **14** guarded dispatch sites (12 `parse_nonneg_usd_arg` + 2
`parse_pos_sell_arg`; the "16" in the r2 note + commit `9647c7e` was arithmetic drift from r2's own
count — corrected per review r3 N1), both harvest rows mutation-proven.
Minors/Nits fixed inline: **M2(r2)** (the trio accept KAT now pins the effect — a low-vs-high
`--income` run must yield a different plan, killing a parse-then-drop mutation); **N1(r2)** (the
stale `1.170A-1(c)(2)` pin-cite in `CONTINUITY_post_v070.md` corrected to `(c)(1)`). Filed:
- **M1(r2) (Minor)** — the TUI `classify-raw` forms (`validate_classify_raw_acquire` `usd_cost`/
  `fee_usd`, `validate_classify_raw_income` `usd_fmv`; `form.rs`) still parse with bare
  `Usd::from_str` and build `Acquire`/`Income` directly (deliberately NOT via `InboundClass`,
  R0-I1), so a negative-basis Acquire is recordable from the TUI raw path. OUT of the UX-P4-4 §3.3(a)
  table (classify-raw is on neither surface's table; the CLI counterpart is an equally-unguarded
  raw-JSON escape hatch — the surfaces are symmetric), so not a fold defect and non-gating — but it
  is the same "negative basis reaches a filed form" class on a sibling record path. Owning phase =
  post-release UX. If guarded later, guard BOTH the TUI raw forms and the CLI `classify-raw` payload
  symmetrically. **N2(r2) (Nit)** — the receipt threading at the two TUI confirm sites is
  compile-forced but not value-witnessed (item.date is the only TaxDate in scope; recorded only).

**UX-P4-11 (#18 `events list`) impl review r1 residue (2026-07-19, `reviews/ux-p4-11-impl-fable-review-r1.md`):**
The 2 Important findings were FOLDED to green (I1: `TransferLink --to-event` now decides BOTH legs in
the reverse-map — the in-leg it relocates onto — with a matched-pair KAT; I2: a void→re-decide KAT that
reds when the `voided` filter breaks). Recorded inline: **M2** (SPEC §3.6 amended — the row universe is
the reconciliation-CLASSIFICATION surface; `Dispose`/`select-lots` deliberately excluded, refs via
`disposals.csv`; the `inspect.rs` doc-comment softened accordingly); **N3** (the `List` help now states
rows are in ledger/import order, not by date). Filed (non-gating):
- **M1 (Minor, owning phase = Step-1c / #14)** — `events_list`'s voided-set honors EVERY
  `VoidDecisionEvent`, but the resolver keeps a `SupersedeImport`/`RejectImport` IN FORCE when a void
  targets it (the void is inert + raises the conflict, `resolve.rs:424-443`). So in a vault already
  hard-blocked by such a void, an accept-conflict row wrongly flips back to `[decidable]`. Only reachable
  in an already-blocked vault; Step-1c's record-time `void` refusal (refuse non-revocable/already-voided)
  makes it unreachable going forward. Burn down with 1c: mirror the resolver's revocability rule OR derive
  decided-status from resolver outputs.
  **✅ DISPOSITION (Phase 8 whole-branch review, 2026-07-19):** the DEFECT is discharged by
  unreachability — all three void surfaces (the guarded CLI `void`, `voidable_decisions`-filtered
  bulk-void, `is_revocable_payload`-filtered TUI) refuse a non-revocable target, so only a vault written
  by a ≤v0.7.0 binary could exhibit it (and there are no users yet). The cosmetic `events_list` mirror is
  RE-OWNED (non-gating) to the events-list-M3 / legibility sweep, clearing the passed-1c ownership.
- **M3 (Minor)** — the Income `amount` column shows the imported `usd_fmv` (else close price), ignoring a
  live `ManualFmv` override (the resolver's effective FMV, `resolve.rs:287-289`). So a just-`set-fmv`'d
  income row displays the pre-correction figure next to its `[decided: <ManualFmv>]`. The `~$` marker is
  explicitly indicative for every kind; prefer the live override when one is present.
- **N1 (Nit)** — `render::fmt_btc` drops the sign for sat ∈ (−1e8, 0) (`-0.5` → "0.50000000"); unreachable
  for persisted payloads (adapters `.abs()` at build time), display-robustness only.
- **N2 (Nit)** — in an already-blocked vault the `decided` map diverges from the resolver in the same
  class as M1: it is later-wins while the resolver is first-wins for ClassifyInbound; and it marks a
  target `[decided]` for a decision the resolver does NOT actually consume — the `TransferLink` in-leg
  when the link is inert (a duplicate in-event, or a no-wallet/type-invalid link the resolver excludes).
  The shown ref is usually the correct void target anyway. All of it dissolves if decided-status ever
  moves to resolver-derived (see M1); only reachable in an already-hard-blocked vault. (review r2 N1.)

**UX-P4-3 (#14) impl review r1 residue (2026-07-19, `reviews/ux-p4-3-impl-fable-review-r1.md`):**
The 2 Important findings were FOLDED to green (r1 verified the `would_conflict` construction is
definitionally the resolver — no false-accept/refuse). I1: added the [G2-6] classify-raw-duplicate KAT
(mutation-proves the previously-unwitnessed `classify_raw` guard — deleting it now reds) + the [R3-I1]
accept-governed `SupersedeImport` KAT (Income conflict minted via `append_import_batch`, accepted →
set-fmv accepted / classify-inbound refused wrong-type — the OTHER `applied` writer). I2: the
DecisionConflict remedy hints are UNIFIED at the source (`resolve.rs` — one `CONFLICT_HINT` const naming
`events list`, surface-neutral; duplicates add "void the prior decision to re-decide"; the record-time
wrapper no longer double-adds a contradictory hint). Nits (non-gating, filed):
- **N1 (Nit)** — `classify_inbound` loads all events twice (the UX-P4-4(b) receipt guard + the UX-P4-3
  conflict guard) and runs two `project()`s; immaterial at real ledger sizes (the whole suite projects
  constantly in ~3s). Only noted so a future BULK-wiring attempt doesn't copy the pattern into a loop.
- **N2 (Nit, no action)** — documented, spec-conforming residual: under stored-pseudo-ON with an
  UNRESOLVED ImportConflict, a real classify-raw on that target is accepted at record time (pseudo-OFF
  shadow: `applied` lacks it) but the next pseudo-ON `verify` shows "duplicate classify-raw". Exactly
  what §3.2 `[T2-I1,R3-I1]` mandates (pseudo-gated inserts absent under pseudo-OFF); the trap survives
  only inside pseudo mode's advisory fiction, which never gates. Examined + intended.
- **N3 (Nit, no action)** — `link_transfer` (duplicate-TransferLink) and `select_lots`
  (duplicate-LotSelection) also raise DecisionConflict at verify with user-typed refs but are OUTSIDE
  §3.2's six-fn choke list — correctly left unwired; a later cycle can decide whether they deserve the
  same record-time treatment.

**UX-P4-3 (#14) impl review r2 residue (2026-07-19, `reviews/ux-p4-3-impl-fable-review-r2.md`):**
r2 verified I2 fully closed. The one Important (r2-I1: the 4th mandated KAT — `classify-raw` on an
accept-governed target refused `[R3-I1]` — absent, with a proven one-writer-short survivor) was FOLDED:
the accept-governed KAT now also asserts classify-raw on the accept-governed target is REFUSED
("already classified", nothing appended), pinning pass-1c's duplicate check against the SupersedeImport
`applied` writer (resolve.rs:513). M1 folded (the classify-raw duplicate hint softened to "if the prior
decision is revocable, void it to re-decide" — only that arm can blame a non-revocable SupersedeImport
prior). N2 folded (the already-voided refusal phrasing aligned to the `CONFLICT_HINT` wording). Filed:
- **N1 (Nit, owning phase = docs/#21)** — `cli.rs` reclassify-income help + `btctax-reconcile-reclassify-income.1`
  still describe the record-then-surface-at-verify flow ("fires a Hard DecisionConflict blocker (decision
  excluded)… to re-decide, void the prior decision first"); true of the engine, but the CLI verb now
  refuses at record time. Not in §3.2's mandate; fold into the cycle's doc pass.
- **N3 (Nit, later cycle)** — the non-§3.2 DecisionConflict emitters keep bare details (TransferLink
  arms, the R0-I1 overlap "void the conflicting decision", LotSelection, allocation arms) — outside
  §3.2's six-verb subject (consistent with r1 N3); a later cycle can sweep their hints + record-time
  wiring together (see the events-list M1 already filed under UX-P4-11).

**UX-P4-7/8/9 (#15 Phase 2) impl review r1 residue (2026-07-19, `reviews/ux-p4-789-impl-fable-review-r1.md`):**
r1 raised 2 Important, both FOLDED (re-review r2 pending):
- **I1** (UX-P4-8 regressed the TUI unlock screen's missing-vault message — my `Session::open` PathIo
  change orphaned `map_open_error`'s `no vault at <path>` arm, viewer AND editor) — FOLDED: added a
  `CliError::PathIo`/NotFound arm to `unlock.rs::map_open_error` preserving the concise line, pinned by
  `missing_vault_maps_to_concise_no_vault_message` (mutation-proven).
- **I2** (UX-P4-8 missed the sibling `--out` sites) — FOLDED: new `admin::mkdir_out` choke point enriches
  `export-irs-pdf` + `export-full-return`; `backup_key` wrapped; KATs for all three + a `mkdir_out` unit
  test (all mutation-proven). M2 (unpinned `EXPORT_OUT_HINT`) folded by the same KATs; M1 (overclaiming
  CSV-wrap comment) + N2 (stale `whatif.rs:16` module doc) fixed inline.
- **N1 (Nit, later cycle)** — `store_io_with_path` at `Session::open` also enriches a NON-NotFound Io
  (e.g. an existing vault whose `.key` sidecar is missing → `vault.rs` `read(&kp)`): the message then
  pairs "No such file" with a `--vault` path that exists + a "run `btctax init`" hint that would refuse.
  Strictly more info than the prior bare error; a later pass could branch the hint on `io.kind()`.
- **N3 (Nit → new follow-up, owning phase = later polish cycle)** — `OptimizeError::NoLots` →
  `"no lots available to sell"` (`crates/btctax-cli/src/cmd/optimize.rs:82`; raised
  `crates/btctax-core/src/optimize.rs:1187`) has the SAME false-"no" shape UX-P4-9 just fixed for
  `what-if sell`, when no feasible lot selection covers the target while lots exist. NOT a mechanical
  reuse of `render::no_lots_message` (optimize's "no feasible selection" ≠ whatif's "insufficient
  balance"), so it needs its own small analysis. Also `main.rs:2029` still Debug-prints `MethodElection
  {:?}` — a single fieldless token, NOT the truncating-struct class UX-P4-7 targets; leave as-is.

**UX-P4-7/8/9 (#15 Phase 2) impl re-review r2 residue (2026-07-19, `reviews/ux-p4-789-impl-fable-review-r2.md`):**
r2 verified I1+I2+M2+N2 RESOLVED, but the M1 comment fix exposed a NEW Important — FOLDED:
- **r2-I1 (Important)** — the `write_csv_exports` `--out` wrap used `cli_io_with_path`, which matched
  only `CliError::Io`; but a SUBPATH collision (`out_dir/lots.csv` as a directory → `open_owner_only`
  fails) arrives as `CliError::Store(StoreError::Io)` and passed through PATHLESS (reviewer proved it
  live). FOLDED: `cli_io_with_path` now enriches BOTH `Io` and `Store(Io)`; pinned by
  `export_out_subpath_collision_names_path` (mutation-proven); the misleading `admin.rs` comment
  corrected. **r2-N2** (self-referential hint pin in the `mkdir_out` unit test) folded: the unit test now
  asserts the literal hint substring.
- **r2-N1 (Nit, later cycle)** — `map_open_error` (`unlock.rs`) keeps two now-dead NotFound arms
  (`Store(Io)` / `Io`), since `Session::open` maps every vault-open Io to `PathIo`. Harmless defensive
  residue (keeps the pin green if the enrichment were ever reverted); leave or collapse in a later pass.
- **r2-N3 (Nit → later polish cycle)** — `init --key-backup <path>` (`cli.rs`) writes a key to a
  user-named path and is pathless on collision — same UX shape but NOT an `--out` (outside UX-P4-8's
  class term). Sweep it with the N1/N3 residue when the later legibility polish cycle runs.

**UX-P4-7/8/9 (#15 Phase 2) impl re-review r3 — GREEN (0C/0I), r3 residue (2026-07-19, `reviews/ux-p4-789-impl-fable-review-r3.md`):**
r3 verified r2-I1 + r2-N2 RESOLVED and passed the gate (0C/0I). Non-gating residue, all the SAME
"pathless user-path I/O" class — **sweep together in a later legibility-polish cycle** (with the r1-N3
`OptimizeError::NoLots` + r2-N3 `init --key-backup`, one owning phase, one coherent pass):
- **r3-M1 (Minor, later polish cycle)** — the PDF `--out` exporters still surface a SUBPATH collision
  pathless: `admin.rs::write_bytes_owner_only` (`open_owner_only(path)?` → `CliError::Store(StoreError::Io)`)
  is unwrapped, so `export-irs-pdf --out out` with `out/f8949.pdf` an existing DIRECTORY yields
  `io: Is a directory (os error 21)` with no path. Outside UX-P4-8's mandated site list (spec §4/§9.5 =
  vault-open + export-snapshot's `admin.rs:82`/`render.rs:586-618`); NOT the advertised-but-dead-guard
  shape r2-I1 gated on. One-line fix when swept: enrich inside `write_bytes_owner_only`
  (`.map_err(|e| cli_io_with_path(e, path, EXPORT_OUT_HINT))`, names the EXACT colliding file) + a KAT.
- **r3-N1 (Nit)** — `Csv(csv::Error)` can carry an underlying mid-write `io::Error` (a large-CSV
  `ENOSPC`/`EIO`), which stays pathless (`csv: <io>`); the `cli_io_with_path` passthrough was blessed in
  r1/r2. Exotic; sweep with the class.
- **r3-N2 (Nit)** — `cli_io_with_path`'s doc should state the call-site PRECONDITION ("wrap ONLY
  expressions whose every Io is a write under `path`") so a future caller cannot mislabel a vault-side
  `Store(Io)` with the wrong path/hint. One-sentence doc note.
- **r3-N3 (process, done)** — the r2 review was persisted in the SAME commit as its fold; going forward
  persist the review in its own commit BEFORE folding (as the r3 GREEN persist does here).

**UX-P4-6/10 (#16 Phase 3) impl review r1→r2 — GREEN (0C/0I), residue (2026-07-19, `reviews/ux-p4-6-10-impl-fable-review-r{1,2}.md`):**
r1 (0C/1I) folded the plan-mandated dual-report exit-0 non-trigger KAT (I1) + hard-blocker exit-1 KAT
(M1); r2 GREEN, independently re-ran the exact regression mutation. Non-gating Nits (both optional, later
polish cycle):
- **N1 (Nit)** — no vault-level (projection-through-render) test pins the UX-P4-6 pending line; both KATs
  hand-build `LedgerState`. Held belt-and-suspenders by the `fold.rs` sigma_pending derivation invariant.
- **N2 (Nit)** — `report_dual_report_absolute_refused_delta_computed_exits_zero` could additionally assert
  the refusal reason substring (`TaxableIncomeNonPositiveWithCarryforward`) to pin the fixture to
  screen_absolute case (c). Documentation-strength only — the exit-0 non-trigger is refusal-agnostic.

**TUI screen-walkthrough PoC (design/tui-walkthrough) impl review r1→r2→r3 — GREEN (0C/0I) (2026-07-19,
`reviews/poc-impl-fable-review-r{1,2,3}.md`):** r1 (1C/1I/4M/3N) folded the ungated-manifest Critical
(new xtask `walkthrough_manifests_valid_and_complete` grammar + FRAME⇄golden bijection gate + CI %PDF +
corrected a false Makefile gating claim + SPEC §5 As-built amendment) and the fail-open Makefile loop
(I-1); r2 (0C/1I) folded NEW-I-1 (per-crate `WALKTHROUGH_{VIEWER,EDITOR}_STEMS` asserts pin disk⇄capture
so a dropped tuple can't pass vacuously); r3 GREEN. One non-gating residue:
- **r3-M-1 (Minor, owning phase: Phase 2 rollout) — DONE 2026-07-20 (cc8e885)** — closed the const⇄manifest
  drift gap: `walkthrough_manifests_valid_and_complete` now parses the two `WALKTHROUGH_*_STEMS` consts out
  of the crate sources and asserts their UNION == every frame golden on disk (a golden captured by no crate
  reds). Mutation-proven. The whole chain manifest⇄disk⇄capture⇄real-TUI is pinned.
- **r3 non-finding (Phase 2) — DONE 2026-07-20 (cc8e885)** — `make regen-walkthrough` added (regenerates all
  frame + console goldens in one command; manifests untouched).

**Phase-2 walkthrough journey reviews (J1-J9) — app-side / doc residue (2026-07-19,
`reviews/phase2-*.md`):** all 9 journeys reviewed to GREEN (J9 folded 2 Important — the false
"less than either holding" + the select-lots frame that depicted no identification; both fixed in the
walkthrough). Non-gating residue, owned by a later polish cycle (app-side surfaces the walkthrough only
SURFACED — not walkthrough bugs):
- **J9 app-limit (Minor) — DONE 2026-07-20** — the editor `select-lots` LotsForm now builds candidates from
  the TRUE at-disposal pool via the engine's own `Session::available_lots_before` →
  `optimize::available_lots_before` → `fold::pools_before` — the SAME pool `selection_feasible` validates a
  pick against. Correct in MEMBERSHIP (a lot that did not exist at the event, or was later relocated in, is
  excluded by construction — the fold stops before the target EventId) AND amounts, for disposals, removals,
  and self-transfers alike, so the form can never offer a pick the engine rejects. **BOTH transition arms**
  now delegate (single code path): `pool_key(item.date, wallet)` routes a POST-2025 disposal to its own
  wallet pool (§1.1012-1(j)) and a PRE-2025 disposal to the pre-boundary UNIVERSAL pool (cross-wallet; the
  §7.4 seed has not fired, so Path-B `SafeHarborAllocated` seed lots are absent by construction). Supersedes
  (i) the initial hand-rolled post-2025 add-back (bounded amounts, NOT membership — polish-batch review r1
  C1/I1–I3) AND (ii) the pre-2025 residue arm, which offered final-state lots and could hand a doomed pick
  against a later-acquired or later-relocated lot → `LotSelectionInvalid` → `NotComputable` (review **r2
  NEW-1**). Correctness inherited from the tested primitive (`optimize.rs` `available_lots_before_*` KATs:
  acquired-after excluded, cross-wallet excluded, path-A/B transition, deterministic — review r1 I4); TUI
  integration pinned by three drivers: the J9 editor driver's `rows.len() == 2` (post-2025), the new
  `kat_new1_pre2025_select_lots_offers_at_disposal_pool_not_residue` (pre-2025 membership + amount), and the
  rewritten `kat_pre2025_pathb_seedlots_excluded` (pre-2025 Path-B: original pre-seed lot offered, seed lots
  excluded, pick accepted CLEANLY end-to-end — no LotSelectionInvalid / no SafeHarborUnconservable).
- **J9-M1 (Minor) — DONE 2026-07-20** — the LotsForm column header "Basis/Sat" (read as per-sat) relabeled
  to "Basis USD" (it renders the row's total USD basis).
- **J2-M3 / J6 (Minor) — DONE 2026-07-20** — the Forms-tab footnote reworded: the Section (A/B) is set by
  the §170(f)(11)(F) year-aggregate, not per donation (`tabs/forms.rs`).
- **J6-M2 (Minor) — DONE 2026-07-20** — relabeled the misnamed flag on both surfaces: TUI "NIIT applies:
  {bool}" and CLI "NIIT {bool}" → "NIIT increased by crypto: yes/no" (`tabs/tax.rs`, `render.rs`). The
  underlying `MarginalRates.niit_applies` field remains misnamed (core `whatif.rs:119`) — a rename is a
  deeper, ownerless-residue refactor (13 non-test refs).
- **J5-N3 (Nit) — DONE 2026-07-20** — J5's confirm prose now notes the accepted identification clears the
  `non_compliant` flag the setup's verify showed. (J1's beat is already addressed by its own non_compliant
  clause pointing to select-lots.)

**Select-lots fold — Fable r2/r3 residue — ALL 4 DONE + reviewed (2026-07-20, HEAD `<this batch>`,
`reviews/polish-batch-selectlots-fable-review-r2.md`/`-r3.md`):**
- **(SL-r2-a) Pre-2025 Path-B specific-ID — DECIDED: LEAVE (engine-validated) + NF-2 status arm + multi-lot
  KAT. DONE 2026-07-20.** Unifying the select-lots arms lets the user specific-ID a PRE-2025 disposal under
  an in-force Path-B `SafeHarborAllocation`. **Decision = option (a) LEAVE** — it is architecturally clean
  (re-introducing a suppress-branch would undo the NEW-1 unification), permits a legitimate filer action,
  and is safe: a non-FIFO pick that changes the Universal residue's Σ-BASIS breaks conservation, and the
  engine surfaces that as a HARD, year-gating `SafeHarborUnconservable` (`universal_snapshot` is
  selection-aware, resolve.rs:1227-1244; state.rs:46-49/89) — never a silent wrong number. **NF-2 FOLDED:**
  `derive_select_lots_status` gained a 4th arm that fires only when THIS save NEWLY introduces
  `SafeHarborUnconservable` on an allocation, so the status warns "broke your safe-harbor allocation … the
  year will not compute" instead of the clean "recorded" message. The attribution is a **SET-DELTA** on the
  allocation ids already broken pre-save (`unconservable_before`), NOT a boolean — this (a) resolves the
  review-r1 **NEW-2** masking false-negative inline (a break on one allocation can't hide behind a
  pre-existing break on another), and (b) is fully mutation-held — the CONDITION by `kat_pre2025_pathb_
  multilot_…` (HIFO default → D draws expensive Z, alloc attests cheap X, specific-ID X flips the residue →
  break → NF-2 status; kills polarity/arm-off) + `kat_pre2025_pathb_preexisting_break_…` (wrong-basis alloc
  broken AT BASELINE → a feasible pick must NOT be blamed → kills guard-dropped) + the tightened `ST-SEL`
  clean-save assert (`contains("contemporaneity") && !contains("BUT it broke")` → kills the always-fire
  shape); and the CAPTURE-side kind filter (**review-r2 NEW-3, closed inline**) by `kat_pre2025_pathb_timebar_
  advisory_does_not_mask_a_new_break` (an unattested-ProRata TIMEBARRED-but-conserving alloc → a new break
  must still be attributed despite the pre-existing Advisory blocker → kills the drop-the-kind-filter
  mutant). All mutants verified RED under the mutation before landing. **NF-1 (composition drift) = out-of-engine-scope,
  documented, NOT code:** the conservation guard is TOTALS-only (Σsat, Σbasis); an equal-totals composition
  drift (same per-sat basis, different `acquired_at`) is not detected — but that is PRE-EXISTING semantics
  (the CLI has always accepted pre-2025 Path-B selections; the attestation's per-lot dates are the filer's
  claim, verified nowhere even with zero selections). A future decision on it would scope to
  `AllocMethod::ActualPosition` (the one mode that claims to mirror actual position). Recorded in the
  `derive_select_lots_status` doc-comment (arm 3 note). — **DONE (incl. NEW-2).**
- **(SL-r2-b, M-1) per-row cap in `validate_select_lots` — DONE 2026-07-20.** Enforces per-row
  `sat ≤ remaining_sat` at form validation (the form only OFFERS feasible lots, but the INPUT was uncapped —
  the last way to reach `LotSelectionInvalid` from this form). KAT `kat_v_sl_4_per_row_overdraw_is_rejected_
  even_when_sum_matches` (RED→GREEN: 80k on a 30k row while Σ==principal). — **DONE.**
- **(SL-r2-c, M-2) drop the double-projection in `Session::available_lots_before` — DONE 2026-07-20.** Now
  loads events + config directly (no discarded full projection); `optimize::available_lots_before` runs its
  own resolve+fold. Behavior held by the existing select-lots KATs (byte-identical rows). — **DONE.**
- **(SL-r2-d, M-3) distinguish load-error from empty pool — DONE 2026-07-20.** The row build now matches on
  `available_lots_before`'s `Result`: an `Err` surfaces "Couldn't read the vault to list lots …" and stays
  on List, no longer masquerading as "No lots available". KAT `kat_sl_r2d_load_error_is_distinct_from_empty_
  pool` (drops the `events` table after the flow opens to force the read failure). — **DONE.**
