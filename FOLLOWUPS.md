# FOLLOWUPS — bitcoin_tax (TaxApp)

Open/!resolved action items (STANDARD_WORKFLOW §4). Each: what · why · status · pointer.

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
  P0's CLI seam to cover it. Burn down in/before P3.
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
- **UX-P1-7 (Minor — docs coverage gap; from SPEC §15 r2 (a)).** Owning phase: **post-v0.7.0 docs**. The
  manual inbound-income pricing verb `reconcile classify-inbound-income <ref> --kind … --fmv …` is
  demonstrated **nowhere** in the worked examples (J4 uses auto-resolved River income; the missing-FMV auto
  path needs an unsupported year — §12 S4). Add a future journey that classifies an unclassified income
  Receive with a manual `--fmv`. Not a P1 blocker; recorded so the gap isn't silent.
- **UX-P1-8 (Minor — docs coverage gap; from SPEC §15 r2 (d)).** Owning phase: **post-v0.7.0 docs**. The
  matched-pair `reconcile match-self-transfers` workflow is undemonstrated (J3 uses the single-exchange
  `classify-inbound-self-transfer` path). Add a future two-exchange journey. Not a P1 blocker.
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
- **UX-P3-2 (Nit — P3, deferred).** Owning phase: post-v0.7.0 docs. The TUI PDF render (`make examples-tui`,
  `docs/examples-tui/tui-wrap.awk`) is MONOCHROME — it renders the goldens' glyph grid and drops the
  per-cell style overlay (colors), and maps box-drawing glyphs to ASCII (gropdf lacks them). SPEC §8 wants
  "color from the style map"; the `.txt` goldens carry the full fg/bg/modifier, so a colorized groff render
  (roff color escapes driven by the style runs) is a future enhancement. The GATED artifact (the `.txt`
  goldens) already captures color; only the convenience PDF is monochrome.
- **N-2 (P2 review) — RESOLVED in P3.** The TUI goldens (`docs/examples-tui/*.txt`) are staleness-gated
  in-process by the crates' `*_goldens_match_committed` tests (which the `test` job runs), so no git-diff
  widening of the CI `examples` job was needed; that job instead gained a `make examples-tui` PDF-build proof.

- **UX-P2-1 (Minor — P2 review M-2; future-drift, not a current bug).** Owning phase: **P4 residue**. The
  SOFT subcommand-coverage matcher `is_demonstrated` (`examples.rs`) is an in-order SUBSEQUENCE match, so a
  single-token leaf can be satisfied by a longer path that contains it — top-level `["import"]` matches the
  line `$ btctax income import …`. Today all 17 covered leaves are independently, genuinely demonstrated
  (P2 review verified line-by-line), so the count is honest; the risk is future drift (drop bare `import`,
  keep `income import`, and the report still claims top-level `import` covered). SOFT/non-blocking. Fix:
  require `path[0]` to be the first non-`-`-prefixed subcommand token (skipping the `--vault v.pgp` global
  flag + value). Deferred — the fix is non-trivial and the current report is correct.
- **P2 review nits (recorded).** **N-2 (owning phase P3):** `ci.yml`'s drift gate diffs only
  `docs/examples/examples.md`; SPEC §9 writes `git diff … docs/examples docs/examples-tui` — equivalent
  today (no TUI golden yet), but **P3 must widen the CI diff to `docs/examples-tui/` when the TUI golden
  lands**. **N-3 (no code):** plan Task 2.2 cites `crates/xtask/src/examples/mod.rs`; the code is at
  `crates/xtask/src/examples.rs` (P1's actual bin-only layout) — citation drift only. M-1 (silent 0/N on a
  missing golden) and N-1/M-3 (nested-build `--locked` + `Stdio::null()`) were FOLDED in the P2 gate commit,
  not deferred.
