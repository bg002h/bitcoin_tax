# App-side polish batch (walkthrough residue) review r1 — NOT GREEN (select-lots C1)

_Fable, independent. Commit e59768c. Fixes 1/2/3/5 GREEN; #4 (select-lots at-disposal availability) has
1 Critical + 4 Important._

VERDICT: NOT GREEN — 1 Critical, 4 Important (suite green: make check 2071; regens byte-stable).

## Critical
C1 — the never-over-offers claim bounds AMOUNTS, not MEMBERSHIP. The seed loop (main.rs:4127-4140) filters
snap.state.lots by WALLET only (no time condition), so it offers (a) a lot acquired AFTER the disposal, and
(b) a fragment relocated into the wallet by a LATER self-transfer (fresh lot_id via bump_split fold.rs:800;
acquired_at keeps the original date fold.rs:808, so no acquired_at filter detects it). For such a row the
true at-disposal pool is ZERO; validate_select_lots has no per-row cap / existence check (form.rs:1318-1355,
Σ==principal only); the pick is persisted before re-projection (main.rs:3959-4011) → replay fails
selection_feasible "does not exist" (pools.rs:118-136) → hard LotSelectionInvalid → tax NotComputable until
voided. PRE-EXISTING (old residue build had it too) BUT the batch's comment (main.rs:4122-4123), commit
message, and FOLLOWUPS DONE entry assert the guarantee — false. FIX: core ships pools_before (fold.rs:450-481,
boundary-seed-correct both transition paths); optimize.rs:266 available_lots_before wraps it. Expose a helper,
read PoolKey::Wallet(item.wallet). Kills C1 + I2's self-transfer half + I3 in one move.

## Important
I1 — Treatment-B fee mini-disposition shadows the main Disposal in `find` (same EventId, pushed at
fold.rs:641 BEFORE the main at :663). The List filters minis out (main.rs:4782) but the availability lookup
(main.rs:4141-4145) does not → `find` returns the fee record → only fee sats added back, principal legs never.
For any post-2025 Sell/Spend with fee_sat under config (b), the J9 defect persists. FIX: `.find(|d| d.event ==
item.disposal_event && !d.fee_mini_disposition)` + test.
I2 — Gift/Donate/SelfTransfer items get NO add-back (scan is only snap.state.disposals; removal legs live in
state.removals fold.rs:1136; self-transfers produce no record fold.rs:797). So for those kinds the form is
residue-only — the exact defect FOLLOWUPS marks DONE. FIX: scan state.removals for removal items; self-transfers
need pools_before.
I3 — "offers the AT-DISPOSAL availability" (FOLLOWUPS) overstates: later-disposal consumption is not restorable
(under-offers in multi-disposal; a fully-later-consumed or later-relocated lot vanishes). Under-offers only (safe)
but the DONE text + displayed Remaining are wrong in general. Solved by pools_before.
I4 — the safety property is UNTESTED. J9's rows.len()==2 + the golden pin one case (one disposal, no fee,
Sell). Mutations that add back ALL disposals' legs, drop the same-event guard, or drop the leg wallet filter
survive the suite. Per this project's standard (a fix isn't done until the mutation dies), add a
two-disposals-one-lot test + a Treatment-B fee test.

## Minor
M1 — reconstructed "Basis USD" can be wrong: leg.basis is zone-dependent (dual-basis gift loss zone) and under
Treatment (c)+fee the last leg carries the re-homed fee-sat basis → r.usd_basis inflated. Display-only.
M2 — reconstructed acquired_at is the zone-aware HP start (tacked donor date for gain-zone gift legs), not the
lot's date; the same lot shows/sorts differently by path. Display-only.

## Nit
N1 — "marginal rates:" prefix now mislabels the NIIT clause (no longer a rate); pre-existing, slightly worsened.

## Confirmed good
NIIT label exact (niit_applies = niit_with>niit_without, compute.rs:397; both surfaces; examples.md diff = 3 NIIT
lines). Forms footnote accurate (year-aggregate forms.rs:199-213). Basis header correct (modulo M1). J9 frame math
(lot-a 60000000/15000, lot-b 40000000/24000, pick on lot-a). J5 note accurate. Determinism (BTreeMap<LotId> stable).
Pre-2025 path unchanged; no test broke.

Recommended: C1+I3(+I2 self-transfer) via one pools_before candidate build; I1 the find filter; I2 removal half via
state.removals; I4 tests — then re-review.
