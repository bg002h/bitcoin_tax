# R0 Architect Review — SPEC P2-B: Form 8949 + Schedule D (round 1)

- **Artifact:** `design/SPEC_p2b_form8949_scheduled.md`
- **Baseline verified against:** HEAD `f9daf84` (matches spec's stated baseline).
- **Reviewer role:** independent architect (author ≠ reviewer).
- **Verdict:** **NOT 0C/0I — BLOCKED.** 1 Critical, 1 Important, 5 Minor, 3 Nit.
- **Web-independent verification:** performed (8949 boxes, 1099-DA phase-in, §1015 no-gain-no-loss,
  §1223/Pub 551 gift loss-zone holding period). Citations at the end.

---

## Recon citation check vs current source (f9daf84)

All spec citations verified accurate except as noted:

| Spec claim | Source | Verdict |
|---|---|---|
| `DisposalLeg{lot_id,sat,proceeds,basis,gain,term,basis_source,gift_zone}` | `state.rs:116-125` | ✅ exact |
| `Disposal{event,kind,disposed_at,legs,fee_mini_disposition}` | `state.rs:127-135` | ✅ exact |
| `Consumed{…gain_hp_start(294)…acquired_at(300)…wallet(299)}` | `pools.rs:287-302` | ✅ exact |
| `make_disposal_legs` discards `gain_hp_start` after computing `term` | `fold.rs:120-196` | ✅ exact |
| crypto_st/crypto_lt = Σ leg.gain by term, `disposed_at.year()==year` | `compute.rs:278-291` | ✅ exact |
| `write_csv_exports` (csv crate + `open_owner_only` 0o600) | `render.rs:522-653` | ✅ exact |
| `basis_source` has `ExchangeProvided`; `WalletId::Exchange/SelfCustody` | `event.rs:17`, `identity.rs:110-112` | ✅ (WalletId lives in `identity.rs`, not `event.rs`; harmless) |
| `is_broker` predicate exists | `optimize.rs:444` | ⚠️ exists but **private** — see M2 |
| `DisposalLeg` has no serde derives → no serde migration | `state.rs:115` (`Debug,Clone,PartialEq,Eq`) | ✅ confirmed |
| "update **the two** production leg-construction sites" | grep | ❌ **one** production site (`fold.rs:184`); see M4 |

---

## Findings

### C1 (CRITICAL) — `acquired_at = gain_hp_start` is WRONG for the dual-basis LOSS zone; contradicts the leg's own `term`

D1 (spec L47-51) sets `acquired_at` from `gain_hp_start` **unconditionally** and claims it is
"the SAME value already used for `term`, so it's tacking-consistent." **That claim is false for one
of the four zones.** In `make_disposal_legs`:

- Gain zone / NoGainNoLoss / non-dual → `term_for(c.gain_hp_start, disposed)` (`fold.rs:158,176,181`).
- **Loss zone → `term_for(c.loss_hp_start, disposed)` (`fold.rs:167`)** — the gift date, NOT the donor's date.

The loss zone uses `loss_hp_start` because its basis is FMV-at-gift (§1015(a) loss rule), and when
basis is FMV-at-gift the holding period **does not tack the donor's period — it begins the day after
the gift** (Pub 551, web-confirmed below; the engine already models this correctly). So for a
`GiftZone::Loss` leg, D1 as written yields `date_acquired = donor's date` (often years earlier)
while `leg.term` was computed from the gift date. Concretely: donor acq 2019, gift 2024-06, sold
2025-01 at a loss → engine `term = ShortTerm` (from gift date), but `acquired_at = 2019` → the 8949
row lands in **Part I (ST)** showing `date_acquired 2019 / date_sold 2025` — an internal
contradiction and a **materially wrong filing artifact** (a preparer/IRS reads that as long-term).

This is reachable: the app implements TP11 dual-basis gifts and has `GiftZone::Loss` live.

**FIX (blocking):** set `acquired_at` to the **same HP-start passed to `term_for` for that leg's
zone** — `loss_hp_start` in the loss zone, `gain_hp_start` in the gain / NGNL / non-dual zones. The
single push site (`fold.rs:184`) sits under a zone `match`; have each zone branch also yield its
`hp_start` (it already yields `term`) and thread it into `acquired_at`. Then `date_acquired` always
matches `term`/Part, tacked or not. **Add a Task-1 KAT for the loss zone** (`acquired_at == gift date
== loss_hp_start`, ST when < 1yr from gift) — the current Task-1 KATs only cover the gain/tacked
case, so this bug would ship green.

### I1 (IMPORTANT, blocking) — `box_needs_review` needs the disposing wallet, which is NOT reachable from `state.disposals` as designed (the flag is a phantom under D1)

D4 keys `box_needs_review` on "the disposing wallet is `WalletId::Exchange`," and question 3b asks
to confirm the wallet is reachable "via the lot / the dispose event." **It is not:**

- **Via the dispose event:** `LedgerState` retains **no events** (`state.rs:200-209`: lots,
  holdings, disposals, removals, income, pending, blockers, stats — no `events`). And the `Dispose`
  payload has no wallet (`event.rs:60-65`). So `Disposal.event → wallet` is impossible from state.
- **Via the lot:** `state.lots` **prunes fully-consumed lots** (`pools.rs:173,200`:
  `lots.retain(|l| l.remaining_sat > 0)`) — a disposal that fully consumes a lot deletes it, so
  `leg.lot_id` lookup often finds nothing. SelfTransfer also relocates residue under **new
  lot_ids/wallet** (`fold.rs:719-788`, `pools.rs:34`), so even a surviving lot's `wallet` need not
  equal the wallet the leg was disposed from. Unavailable and unreliable.
- **The only sound source** is `Consumed.wallet` (`pools.rs:299`), available at
  `make_disposal_legs` time — exactly where `acquired_at`/`gain_hp_start` come from.

D2 also states the builder is "pure over `state.disposals`," which cannot hold for a wallet-derived
flag unless the wallet is on the leg.

**FIX (blocking):** add a **second** additive field `pub wallet: WalletId` to `DisposalLeg`, set
from `c.wallet` in `make_disposal_legs` (same one-push-site edit as C1). Update D1 from "**the one**
core-model change" to two additive fields (`acquired_at` + `wallet`); both are serde-free, so still
no migration. This makes `box_needs_review` a pure function of `state.disposals` and lets the
`form8949.csv` `wallet` column (already promised in D2, L64) be populated. Add/keep the Task-2 KATs
(exchange leg → flag set + C/F; self-custody → not flagged).

### M1 (MINOR) — NGNL-zone 8949 row is consistent, but the spec should state *why* (basis = proceeds)

"Report the row with gain 0" (L24-26, L62) is correct AND internally consistent **because the engine
already reports `basis = proceeds` in the NGNL zone** (`fold.rs:177`: `(proceeds, Usd::ZERO, …)`), so
the row copies leg fields verbatim → proceeds − basis = 0. Web-confirmed: an NGNL §1015 disposition
is still reported on 8949/Sch D with zero gain, **no special adjustment code** (column f blank).
**Fix:** state explicitly that the 8949 row copies `leg.proceeds`/`leg.basis`/`leg.gain` **verbatim**
and must NOT recompute basis from the donor carryover (that would fabricate a nonzero gain). Prevents
a well-meaning implementer/reviewer from "correcting" it.

### M2 (MINOR) — `is_broker` is private in `optimize.rs`

D4 (L84) calls it "the existing `is_broker` predicate," but `fn is_broker` (`optimize.rs:444`) is
module-private. Either promote to `pub(crate)`/`pub` (it's a one-liner
`matches!(w, WalletId::Exchange{..})`) or inline it in the 8949 builder. Note in the plan so it's not
assumed reachable across modules.

### M3 (MINOR) — reconciliation KAT tautology risk

The D3 KAT (Sch D ST/LT Σgain == B's `crypto_st`/`crypto_lt`) is a **sound cross-check only if the
two sides run on independent code paths** (8949→Schedule-D aggregation vs B's own loop in
`compute.rs:280-291`). The spec's alternative ("expose a small helper" both call) would make it a
tautology and defeat the drift-detection purpose. **Fix:** mandate that the reconciliation compares
the Schedule-D-from-8949 total against B's independent sum (or `compute_tax_year`'s own path), not a
single shared helper.

### M4 (MINOR) — "two production leg-construction sites" is a miscount (drift)

There is exactly **one** production `DisposalLeg` literal: `fold.rs:184` (in `make_disposal_legs`).
`state.rs:116` is the struct definition; the other 3 literals are tests
(`tax_compute.rs`, `kat_rate_engine.rs` ×2). The spec's own "(grep)" instruction is the right hedge;
correct the count so the plan doesn't chase a nonexistent second site (and remember both new fields —
C1/I1 — get set at that one push).

### M5 (MINOR) — `sat/1e8` description must be exact Decimal, not the f64 literal

The plan says "exact Decimal, 8dp — sats/1e8; no float" (good), but `1e8` is an f64 literal in Rust.
Implement as `Decimal::from(sat) / dec!(100_000_000)` (or set scale on `Decimal::from(sat)` with a
1e-8 scale) so no `f64` ever touches the description. Add to the no-float assertion in Task 4.

### N1 (NIT) — `box_needs_review` flags ALL exchange dispositions regardless of year

Pre-2025 crypto exchange sales generally had no 1099-B/1099-DA, so flagging them is slightly
over-inclusive (harmless/conservative — it only prompts a manual check). Optional: gate on
`disposed_at.year() >= 2025` to reduce false prompts. Not blocking.

### N2 (NIT) — Schedule D line mapping for the eventual filled form

CSV part-totals (`part,proceeds,cost_basis,gain`) commit to no line numbers — fine. But note for the
deferred filled-Schedule-D FOLLOWUP: box **C → line 3**, box **F → line 10** (not 1b/8b, which are
box A/D). The spec's "line 1b/2/3 // 8b/9/10" phrasing describes the block, not the C/F rows.

### N3 (NIT) — date format

`TaxDate::to_string()` emits ISO `YYYY-MM-DD`; fine and unambiguous for CSV. The 8949 form itself
wants MM/DD/YYYY — a presentation concern to defer with the filled-PDF FOLLOWUP.

---

## Assessment of the six review dimensions

1. **8949 column correctness.** Mapping is right EXCEPT the `date_acquired`/`term` consistency in the
   loss zone (**C1**). Web-confirmed: Part I = ST, Part II = LT; A/D = 1099-B basis reported, B/E =
   1099-B basis not reported, C/F = not on 1099-B; wash-sale is the only common adjustment (N/A for
   crypto → column f/g blank). Gain-zone tacked gift → donor's date is the correct `date_acquired`
   (matches its LT/ST term). ✅ except C1.
2. **NGNL dual-basis on 8949.** "Row present, gain 0" is correct and internally consistent because
   the engine sets basis = proceeds; no adjustment code needed (**M1** = make it explicit). ✅
3. **Box default + disclosure honesty.** C/F + `box_needs_review` is the honest call: it never
   fabricates an unsubstantiated A/B/D/E, and proceeds/basis/gain are reported correctly regardless
   of box, so no tax is misstated. 1099-DA phase-in (web-confirmed: gross proceeds from 2025 sales,
   basis only for covered assets acquired ≥ 2026) means 2025+ exchange sales MAY sit on a 1099-DA →
   the flag correctly prompts reclassification to B/E (or A/D for 2026+ covered). Spec never
   auto-assigns A/B/D/E ✅. Deferring per-disposition 1099-B input to FOLLOWUPS is acceptable ✅.
   **3b:** wallet reachability fails as designed → **I1**.
4. **Schedule D ↔ engine B reconciliation.** Sound cross-check (same disposals, same year-filter,
   same term split) — subject to **M3** (keep paths independent). Raw-part-totals-here vs
   netted-in-B distinction is correct: `crypto_st`/`crypto_lt` (`compute.rs:278-291`) are PRE
   §1222/§1211/§1212 loss-limit/carryforward, which is exactly what Schedule D part totals are;
   reconciling against those (not `st_net`/`lt_net`) is right. No change to B's math. ✅
   (Note: B's loop includes `fee_mini_disposition` legs; D2's "one row per leg" also includes them →
   consistent; state this so reconciliation is understood to cover fee-mini rows.)
5. **Determinism / no-float / CSV / additive field.** Deterministic ordering specified; no-float
   (**M5** nuance); CSV via existing `open_owner_only` 0o600 + stable columns + no PDF dep ✅;
   `acquired_at` additive with no serde migration (confirmed no serde derives) ✅ — but now **two**
   additive fields (I1).
6. **Scope / right-sizing / TDD.** 4 tasks, independently testable, D1 isolated — good shape. Gaps:
   Task-1 KATs miss the loss-zone `acquired_at` case (**C1**); D1 understates the core-model change
   (**I1**); `is_broker` reuse (**M2**); reconciliation tautology (**M3**); site miscount (**M4**).
   No missing task otherwise; out-of-scope list (filled-PDF, 1099-DA import, netting) is appropriate.

---

## Required to reach green (0C/0I)

1. **C1:** make `acquired_at` zone-aware (= the HP-start used for that leg's `term`; `loss_hp_start`
   in the loss zone) + add a loss-zone Task-1 KAT.
2. **I1:** add `wallet: WalletId` to `DisposalLeg` from `Consumed.wallet`; reword D1 to two additive
   fields; make `box_needs_review` pure over `state.disposals`.
3. Fold M1–M5 (explicit NGNL basis=proceeds; `is_broker` visibility; independent reconciliation
   paths; correct site count; Decimal description). N1–N3 optional.

Re-review required after the fold (including a re-check of the C1 zone logic and the I1 field
threading), per the standard workflow's re-review-after-every-fold rule.

---

## Web verification log (independent)

- **8949 boxes / parts:** Part I = short-term (≤1yr), Part II = long-term (>1yr). Box A/D = on
  1099-B, basis reported to IRS; B/E = on 1099-B, basis NOT reported; C/F = not on a 1099-B. Only one
  box per page. Wash sale ("W") is the common adjustment code; N/A to crypto. — IRS Instructions for
  Form 8949 (2025) https://www.irs.gov/instructions/i8949 ; form
  https://www.irs.gov/pub/irs-pdf/f8949.pdf
- **1099-DA phase-in:** brokers report **gross proceeds for sales on/after 2025-01-01**; **basis
  reporting starts 2026-01-01** and only for "covered" digital assets acquired ≥ 2026 and held at the
  broker. 2025 is proceeds-only with good-faith penalty relief. — IRS final digital-asset broker
  regs https://www.irs.gov/newsroom/final-regulations-and-related-irs-guidance-for-reporting-by-brokers-on-sales-and-exchanges-of-digital-assets
  ; Instructions for Form 1099-DA (2026) https://www.irs.gov/instructions/i1099da
- **§1015 no-gain-no-loss zone:** when proceeds fall between FMV-at-gift and donor's carryover basis,
  gain computed on carryover is a loss and loss computed on FMV is a gain → they cancel → **zero**;
  still reported on 8949/Schedule D with zero gain/loss; no special code. — 26 U.S.C. §1015
  https://www.law.cornell.edu/uscode/text/26/1015 ; IRS Instructions for Form 8949 (2025)
- **§1223 / Pub 551 gift holding period (basis for C1):** if the donee's basis is the donor's
  carryover basis, the holding period **tacks** the donor's; **but if the basis used is FMV at the
  date of gift (the loss case), the holding period begins the day after the gift — no tacking.** —
  IRS Publication 551 (12/2025) https://www.irs.gov/publications/p551

---

# Round 2 — re-review (post-fold)

- **Artifact:** `design/SPEC_p2b_form8949_scheduled.md` (revised).
- **Reviewer role:** independent architect (re-review after the fold, per §2 loop).
- **Scope:** confirm C1 + I1 closed and M1–M5 folded with no new defect. Web-confirmations
  (8949 boxes, 1099-DA timing, NGNL) + the Schedule-D-vs-B raw/netted distinction were validated
  in round 1 and are NOT re-litigated.
- **Source spot-checked @ current tree:** `fold.rs:154-196` (zone match + single push at 184),
  `pools.rs:287-302` (`Consumed.wallet` at 299), `identity.rs:110-113` (`WalletId::Exchange`),
  `compute.rs:133-194` (`net_1222`) + `compute.rs:277-380` (`crypto_st/lt` → `st_net/lt_net`),
  `state.rs:115-125` (`DisposalLeg` derives/fields).
- **Verdict:** **C1 CLOSED, I1 CLOSED, M1–M5 folded, 0 new Critical/Important → R0 GREEN, ready to
  implement.** One non-blocking Minor residual (recon prose, R2-m1) noted below.

## C1 — CLOSED (verified against source)

D1 (spec L50-56) now makes `acquired_at` **zone-aware = the exact HP-start passed to `term_for`**:
loss zone → `loss_hp_start` (gift date, no §1015 tacking); every other zone → `gain_hp_start`.
Cross-checked against `fold.rs`:
- Loss zone (`fold.rs:165-173`) computes `term_for(c.loss_hp_start, disposed)` (L167) → `acquired_at`
  = `loss_hp_start`. ✅
- Gain (L156-164), NGNL (L174-178), non-dual (L179-183) all compute `term_for(c.gain_hp_start, …)`
  → `acquired_at` = `gain_hp_start`. ✅

So the spec's rule "loss zone → `loss_hp_start`; else `gain_hp_start`" maps **exactly** onto the four
branches; `acquired_at` is set from the SAME branch that feeds `term_for`, at the SINGLE push site
`fold.rs:184` (extend the branch tuple to also yield `hp_start`). It can therefore never contradict
the leg's `term`/Part. Task-1 KATs (L121) cover all three required cases: (a) ordinary =
`gain_hp_start`; (b) §1223 tacked gift gain-zone = donor's date; **(c) §1015 dual-basis LOSS-zone =
the GIFT date (`loss_hp_start`), NOT the donor's date, consistent with `term`.** The false round-1
"always `gain_hp_start` / same value as term" claim is gone from D1. ✅

## I1 — CLOSED (verified against source)

D1 (L57-60) adds the second additive field `pub wallet: WalletId`, sourced from `Consumed.wallet` and
explaining why the alternatives fail (LedgerState retains no events; `Dispose` carries no wallet;
`state.lots` prunes consumed lots; SelfTransfer reassigns lot_ids/wallet). Confirmed `Consumed` carries
`pub wallet: WalletId` at `pools.rs:299`, in scope at `make_disposal_legs`. `wallet` is set at the same
single site `fold.rs:184` (`c.wallet.clone()`), so `box_needs_review` (D4) becomes a pure function of
`state.disposals` via `leg.wallet`. ✅

## Minors — all folded

- **M1 (NGNL row):** D2 copies `cost_basis = leg.basis` verbatim (L71) and states the row is
  consistent because the engine already sets `basis == proceeds` in the NGNL zone (L75-78). Confirmed
  `fold.rs:177` = `(proceeds, Usd::ZERO, t, Some(GiftZone::NoGainNoLoss))` → gain 0, no code. ✅
  (The verbatim-copy design removes any invitation to recompute basis from the donor carryover.)
- **M2 (`is_broker`):** D4 (L100) uses `matches!(leg.wallet, WalletId::Exchange { .. })` **directly**
  and explicitly rejects the private `optimize.rs::is_broker`. `WalletId::Exchange { provider, account }`
  confirmed at `identity.rs:111` → the `matches!` pattern is valid. ✅
- **M3 (independent reconciliation):** D3 + Task-3 (L129-137) reconcile P2-B's `schedule_d(state,year)`
  against B's `compute_tax_year(...).TaxResult.st_net/lt_net` — two separate functions, not a shared
  helper. ✅ (Premise verified below.)
- **M4 (one site):** Exactly one production `DisposalLeg` literal (`fold.rs:184`); the other literals
  are tests (`kat_rate_engine.rs`, `tax_compute.rs`). Spec says "update any `DisposalLeg` literals
  (grep)" — correct count + right hedge. ✅
- **M5 (exact Decimal):** D2 (L68-69) mandates `Decimal::from(leg.sat) / dec!(100_000_000)` at 8dp,
  never `f64`; added to the Task-4 no-float assertion. ✅

## M3 premise — verified (net == raw on the KAT fixture)

`net_1222` (`compute.rs:133-194`) with the KAT's inputs (all-gains: `crypto_st ≥ 0`, `crypto_lt ≥ 0`;
`other_lt = 0`; `cf_short = 0`; `cf_long = 0`):
- `st_net = crypto_st − 0 = crypto_st` (L143); `lt_net = crypto_lt + 0 − 0 = crypto_lt` (L145);
- cross-net match hits the `(true, true)` arm (L149) → `(st_net, lt_net)` unchanged.

`compute_tax_year` sets `TaxResult.st_net = with.st_net` (L368) and `lt_net = with.lt_net` (L369). So
on the specified fixture `TaxResult.st_net == crypto_st` and `TaxResult.lt_net == crypto_lt` with **no
cross-netting** — the reconciliation KAT is sound and non-tautological (aggregator path vs
`compute_tax_year` path both re-sum `state.disposals` by `term`/year independently). ✅

## No new Critical/Important

- Two additive projection fields on a **serde-free** struct (`DisposalLeg` derives
  `Debug, Clone, PartialEq, Eq` — `state.rs:115`) → migration-free, no wire/format change. ✅
- `acquired_at` is a new read-only field taken from the existing `hp_start` values; it does not enter
  the `(basis, gain, term, gift_zone)` tuple → capital-gains math (term/gain/basis) is untouched. ✅
- `wallet` is a pure additive read of `c.wallet`. ✅
- Right-sized: 4 tasks, each TDD with KATs; out-of-scope list intact. ✅

## Residual (non-blocking)

- **R2-m1 (MINOR, non-blocking — recon prose):** the current-state section still reads (L34-36) that
  `make_disposal_legs` "uses `gain_hp_start` ONLY to compute `term`" and that `gain_hp_start` "is
  exactly the 8949 'date acquired' that matches the leg's `term`." That is the precise C1
  overgeneralization and is **false in the loss zone** (which uses `loss_hp_start` at `fold.rs:167`).
  The operative design (D1) and the Task-1(c) KAT are correct, so no defect ships; but for internal
  consistency the recon paragraph should be reworded to note the loss-zone exception. Does not affect
  the 0C/0I green criterion — fix opportunistically during Task 1. N1–N3 from round 1 remain optional.

**Green** on the workflow criterion (full validation aside): **0 Critical / 0 Important.** The spec is
R0 GREEN and ready to implement.
