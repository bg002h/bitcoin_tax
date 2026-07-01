# SPEC — P2-B: Form 8949 + Schedule D generation (Phase-2, sub-project 2)

**Source baseline:** `origin/main` @ `f9daf84` (post P2-A §170(e) deduction).
**Goal:** Generate the taxpayer's **IRS Form 8949** (per-disposition rows, ST Part I / LT Part II) and
**Schedule D** (aggregated ST/LT proceeds/basis/gain totals) for a tax year, as CSV exports (+ a text
summary), reconciling with engine B. The headline "produce the capital-gains filing artifacts"
deliverable. Federal-only; offline; CSV (no PDF dependency).

**SemVer:** additive `DisposalLeg.acquired_at` field (projection struct) + new CSV exports + a report
section ⇒ **MINOR** (pre-1.0). No change to capital-gains tax math.

## Legal grounding (IRS Form 8949 + Schedule D instructions)
- **Form 8949** reports each sale/disposition of a capital asset: (a) description, (b) date acquired,
  (c) date sold, (d) proceeds, (e) cost basis, (f) adjustment code, (g) adjustment amount, (h) gain/loss.
  **Part I = short-term** (held ≤ 1 yr), **Part II = long-term** (held > 1 yr).
- **Box** per part: **A/D** = transactions reported on a 1099-B WITH basis reported to the IRS; **B/E** =
  1099-B but basis NOT reported; **C/F** = NOT reported on a 1099-B. (A/B/C are ST; D/E/F are LT.)
- **Schedule D** aggregates the 8949 part totals: Part I → line 1b/2/3 (proceeds, basis, gain); Part II →
  line 8b/9/10; then nets ST vs LT (lines 7/15/16). The **netting + carryforward** (§1222/§1211/§1212)
  is engine B's job (`compute_tax_year`) — P2-B produces the RAW part totals (pre-netting); the netted
  figures live in the tax report (`report --tax-year`).
- **Wash sale (§1091):** N/A to crypto (shipped C.5) → the adjustment columns (f/g) are always blank; no
  adjustment codes are produced by this model.
- **Dual-basis received-gift lot (§1015):** a disposition in the **NoGainNoLoss** middle zone produces
  gain = 0; it is still a reported disposition (row present, gain 0). The `leg.gift_zone` field
  distinguishes it.

## Current-state (recon @ f9daf84)
- `Disposal{event, kind, disposed_at, legs, fee_mini_disposition}` (`state.rs:127-135`);
  `DisposalLeg{lot_id, sat, proceeds, basis, gain, term, basis_source, gift_zone}` (`state.rs:116-125`).
- 8949 columns present on the leg: proceeds, basis, gain, term; **date sold** = `Disposal.disposed_at`;
  **description** synthesizable from `leg.sat`. **GAP — date acquired:** `Consumed{acquired_at,
  gain_hp_start}` (`pools.rs:294,300`) carries the acquisition/HP-start date, but `make_disposal_legs`
  (`fold.rs:120-196`) uses the ZONE-APPROPRIATE HP-start (loss zone → `loss_hp_start` at `fold.rs:167`;
  gain/NGNL/non-dual → `gain_hp_start`) ONLY to compute `term` and then discards it — it is not on
  `DisposalLeg`. The 8949 "date acquired" MUST be that SAME zone-appropriate HP-start so it matches the
  leg's `term` (see D1) — NOT unconditionally `gain_hp_start`, which would contradict `term` in the loss
  zone [R0-C1].
- **GAP — box:** no 1099-B / broker-reported signal exists. `basis_source` (ExchangeProvided/…) and
  `WalletId::Exchange` vs `SelfCustody` exist but neither asserts a 1099-B was issued or basis reported.
- CSV export pattern: `write_csv_exports` (`render.rs:522-653`) writes `disposals.csv` (one row per leg,
  via `csv` crate + `fsperms::open_owner_only` 0o600). No 8949/Schedule D output exists.
- Engine B's `compute_tax_year` (`compute.rs:279-291`) already sums `crypto_st`/`crypto_lt` = Σ `leg.gain`
  by `term` for `disposed_at.year()==year` — the Schedule D part gain totals reconcile with these
  (before B's carryforward/other-LT netting).

## Design

### D1 — add `acquired_at` + `wallet` to `DisposalLeg` (two additive fields, ONE edit site)
Add TWO fields to `DisposalLeg`, both from the `Consumed` already in scope at the **single** leg
construction site in `make_disposal_legs` (`fold.rs:184` — there is ONE production site, not two [R0-M4]):
- `pub acquired_at: TaxDate` — **[R0-C1] the HP-start actually passed to `term_for`, ZONE-AWARE.** In the
  §1015 dual-basis **loss zone** the term is computed from `loss_hp_start` (the gift date — a §1015
  loss-basis gift does NOT tack, Pub 551), so `acquired_at` MUST be `loss_hp_start` there; in every other
  zone it is `gain_hp_start` (the tacked donor date for a gift; the acquisition date otherwise). Rule:
  `acquired_at` = the exact date whose holding period produced this leg's `term` — it must NEVER
  contradict the leg's ST/LT classification. (Set it from the same branch that selects the HP-start for
  `term_for` at `fold.rs:167` (loss zone) vs elsewhere.)
- `pub wallet: WalletId` — **[R0-I1]** from `Consumed.wallet` (`pools.rs:299`), the ONLY sound source of
  the disposing wallet. Do NOT try to derive it from `Disposal.event`/`leg.lot_id`: `LedgerState` retains
  no events, `Dispose` carries no wallet, and `state.lots` prunes fully-consumed lots + SelfTransfer
  reassigns lot_ids/wallet, so any lookup is absent/unreliable. Used for the box_needs_review check (D4).
Update any `DisposalLeg` literals (grep). Add `acquired_at` (and `wallet`) columns to `disposals.csv`.
`DisposalLeg` is a projection struct (no serde) → both additions are migration-free. No gain/basis/term
math change (`WalletId` is `identity.rs`, not event.rs — R0 note).

### D2 — Form 8949 generation (core fn + CSV)
A core fn produces the 8949 rows for a tax year: **one row per `DisposalLeg`** whose
`Disposal.disposed_at.year() == year`, with:
- **description** = the BTC amount, e.g. `"0.53000000 BTC"` — **[R0-M5] computed as exact Decimal**:
  `Decimal::from(leg.sat) / dec!(100_000_000)` formatted to 8dp; NEVER an `f64` (`sat as f64 / 1e8`);
- **date_acquired** = `leg.acquired_at` (the zone-aware HP-start from D1); **date_sold** = `disposal.disposed_at`;
- **proceeds** = `leg.proceeds`; **cost_basis** = `leg.basis`; **gain** = `leg.gain`;
- **part** = ST (Part I) if `leg.term==ShortTerm` else LT (Part II);
- **box** = the conservative default **C (ST) / F (LT)** ("not reported on a 1099-B") — see D4;
- **adjustment code / amount** = blank / 0 (no §1091, no other adjustments);
- rows are emitted for ALL legs — **[R0-M1] incl. NoGainNoLoss dual-basis gift-zone legs: the engine
  already sets `basis == proceeds` for that zone (`fold.rs:177`), so the row's `gain` is `0` and is
  internally consistent (proceeds, basis, gain=0) with no special 8949 code needed** (web-confirmed: a
  §1015 NGNL disposition is reported with zero gain, no adjustment code).
Export as `form8949.csv` (stable snake_case columns: `part, box, description, date_acquired, date_sold,
proceeds, cost_basis, adjustment_code, adjustment_amount, gain, wallet, disposition_kind`) via the
existing `write_csv_exports` pattern. Deterministic ordering (by disposed_at, then event, then lot_id).

### D3 — Schedule D aggregation (totals + reconcile + text summary)
Aggregate the year's 8949 rows into Schedule D part totals: **Part I (ST)** Σproceeds/Σbasis/Σgain and
**Part II (LT)** Σproceeds/Σbasis/Σgain. Export `schedule_d.csv` (`part, proceeds, cost_basis, gain`) +
add a **text Schedule D summary** to the tax report (mirroring `render_tax_outcome`) showing the two
part totals and noting that §1222/§1211/§1212 netting + carryforward is applied in the tax computation
(`report --tax-year`), not here. **Reconciliation KAT:** the Schedule D ST/LT Σgain equals engine B's
internal `crypto_st`/`crypto_lt` for the same year (before B's carryforward/other-LT netting) — assert
this cross-check so the forms and the tax engine can never silently diverge. (B's `crypto_st`/`crypto_lt`
are local to `compute_tax_year`; the reconciliation is done by computing both from the same
`state.disposals` year-filter and asserting equality, OR by exposing a small helper — decide at plan;
do NOT change B's tax math.)

### D4 — box classification (conservative default + honest disclosure)
No 1099-B signal exists, so box is the conservative **C/F ("not reported on a 1099-B")** default for
every disposition. **Because exchange-custodied dispositions (2025+ especially, with 1099-DA broker
reporting phasing in — gross proceeds from 2025, basis for covered assets from 2026) MAY have been
reported on a 1099-B/1099-DA**, add an honest signal: for a disposition whose `leg.wallet` (D1) is an
**Exchange** — check `matches!(leg.wallet, WalletId::Exchange { .. })` **directly** ([R0-M2] do NOT
depend on `optimize.rs::is_broker`, which is private) — set a `box_needs_review` flag/column and a
one-line note in the text summary: "box defaulted to C/F (not reported on 1099-B); this disposition was
from an exchange wallet — if you received a 1099-B/1099-DA, reclassify to box A/B (ST) or D/E (LT) per
whether basis was reported." Do NOT auto-assign A/B/D/E (we cannot know 1099-B issuance/basis-reporting
from the model). The C/F default never fabricates a substantiated box, and the leg's `gain` is correct
regardless of box. A per-disposition 1099-B/box user input is **deferred to FOLLOWUPS**.

### Decisions
- **Per-year, per-leg, CSV-first.** Forms are per-tax-year (take a `year` param like `report
  --tax-year`); per-leg granularity (legs differ in acquired-date/term); CSV (csv crate; no PDF dep) +
  a text summary. Filled-PDF 8949/Schedule D is deferred (no PDF dependency in-tree).
- **Raw part totals here; netting in B.** P2-B produces pre-netting 8949/Schedule D part totals; the
  §1222/§1211/§1212 netted result stays in engine B / the tax report. The reconciliation KAT ties them.
- **Box is a conservative default, not a guess.** C/F + exchange-verify note; never auto-assign a
  1099-B box we can't substantiate.

## Plan (TDD)

### Task 1 — `DisposalLeg.acquired_at` (zone-aware) + `wallet` + disposals.csv columns
- **Files:** `crates/btctax-core/src/state.rs` (two fields + literals), `crates/btctax-core/src/project/fold.rs` (`make_disposal_legs`, the single site `fold.rs:184` — set `acquired_at` from the SAME zone branch that feeds `term_for` (loss zone → `loss_hp_start`; else `gain_hp_start`) and `wallet` from `Consumed.wallet`), `crates/btctax-cli/src/render.rs` (disposals.csv columns).
- KATs: (a) an ordinary disposal leg's `acquired_at == gain_hp_start`; (b) a **§1223 tacked gift (gain-zone)** leg's `acquired_at == the donor's tacked date` (not the gift date) — matches its `term`; (c) **[R0-C1] a §1015 dual-basis LOSS-zone gift leg's `acquired_at == the GIFT date (`loss_hp_start`), NOT the donor's date** — and it is consistent with the leg's `term` (loss basis does not tack); (d) `wallet` matches the consumed lot's wallet (exchange vs self-custody); disposals.csv shows the columns. No gain/basis/term math change (assert existing disposal KATs unchanged).

### Task 2 — Form 8949 rows + `form8949.csv`
- **Files:** `crates/btctax-core/src/...` (an `Form8949Row` type + a `form_8949(state, year)` builder — decide core vs cli placement; the builder is pure over `state.disposals`), `crates/btctax-cli/src/render.rs` (`form8949.csv` in `write_csv_exports` + wire a `year`).
- KATs: ST leg → Part I, box C; LT leg → Part II, box F; description = "{amount} BTC" exact; date_acquired/date_sold correct; proceeds/basis/gain match the leg; a multi-leg disposal spanning ST+LT → two rows in the correct parts; a NoGainNoLoss gift-zone leg → row present with gain 0; adjustment columns blank; year-filter (prior-year disposal excluded); deterministic ordering; **exchange-wallet disposition → `box_needs_review` set + C/F**; self-custody → not flagged.

### Task 3 — Schedule D totals + `schedule_d.csv` + text summary + B reconciliation
- **Files:** `crates/btctax-core/src/...` (`schedule_d(state, year)` → ST/LT part totals), `crates/btctax-cli/src/render.rs` (`schedule_d.csv` + a `render_schedule_d` text section in the tax report).
- KATs: ST/LT Σproceeds/Σbasis/Σgain correct for a mixed year (hand-derived golden); **[R0-M3]
  reconciliation via INDEPENDENT code paths (not a tautology):** on an **all-gains** fixture with a
  `TaxProfile` carrying zero carryforward-in and zero `other_net_capital_gain` (so §1222 does no
  cross-netting → net == raw), assert P2-B's `schedule_d(state, year)` ST Σgain == B's
  `compute_tax_year(...).TaxResult.st_net` and LT Σgain == `TaxResult.lt_net` — P2-B's aggregator and B's
  `compute_tax_year` are separate functions, so equality is a real cross-check. (Do NOT reconcile against
  a shared helper.) The text summary notes §1222/§1211/§1212 netting + carryforward is applied in `report
  --tax-year` (so with losses/carryforward the netted figures legitimately differ from these raw part
  totals); year-filter correct (prior-year disposal excluded); empty year → zero totals.

### Task 4 — whole-diff review (Phase E) + FOLLOWUPS
- Cross-cutting: `acquired_at` tacking-consistent (matches `term`); 8949 rows exhaustive + per-leg; box default honest (C/F + exchange-verify, never a false A/D); Schedule D reconciles with B (the KAT); NO capital-gains tax-math change; CSV perms 0o600 + stable columns; determinism; no float; privacy (synthetic-only).
- FOLLOWUPS: per-disposition **1099-B / box (A/B/D/E) user input** (reclassify from the C/F default);
  **filled-PDF** 8949/Schedule D generation; 1099-DA reconciliation (2025+ broker reporting); any
  §1256/other form scope. All deferred.

## Out of scope
- Filled-PDF forms (no PDF dependency in-tree); a per-disposition 1099-B/box input (default C/F +
  exchange-verify note only); 1099-DA import/reconciliation; §1222/§1211/§1212 netting (engine B owns it —
  P2-B is raw part totals); §170/Schedule A (P2-A); 2026/2027 tax tables.
