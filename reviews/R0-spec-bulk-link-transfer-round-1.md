# R0 spec review — bulk self-transfer (`bulk-link-transfer`) — round 1

**Artifact:** `design/SPEC_bulk_link_transfer.md`
**Baseline reviewed against:** branch `feat/bulk-link-transfer` @ `441d1b0` (spec DRAFT commit is docs-only over `main`==`a16ea00`).
**Reviewer:** independent adversarial architect (did NOT author the spec).
**Gate:** R0 — Critical/Important block implementation.

## Verdict: **0 Critical / 2 Important / 4 Minor / 2 Nit**

The core safety claim — the selection boundary — is **sound and verified**: a linked-out projects as
`Op::SelfTransfer` and is never pushed to `pending_reconciliation`, so bulk can never re-touch a decided
out. Atomicity, KAT-G1 confinement, revocability, the `b` key being free, the wallet-union destination
source, the `#7` void filter, and the "no lockstep" claim all check out against current source. Two
Important findings remain (a TUI mid-batch rollback hole and the headline USD-total representation), both
with cheap fixes. Fix them and re-review; then this is implementable.

---

### [I1] IMPORTANT — TUI `persist_bulk_link_transfer` does not roll back a mid-batch append failure

`design/SPEC_bulk_link_transfer.md:159-171` (the `persist_bulk_link_transfer` body); atomicity claim at
`:184-186` and gotcha #5 at `:210`.

**Defect.** The spec's loop is:

```rust
let pre = session.snapshot()?;
for out_event in &out_events {
    ...
    append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;   // no save yet
}
save_or_rollback(session, pre)?;   // ONE save; on failure the whole batch reverts
```

`append_decision` runs its own `unchecked_transaction()` + `tx.commit()` per call
(`crates/btctax-core/src/persistence.rs:238-262`), so each successful append is **already committed to the
in-memory conn**. If append *k* fails, the `?` returns `Err(PersistError::NoChange(..))` **without ever
restoring `pre`** — leaving appends `1..k-1` as live in-memory residue. The single-append persisters
(`persist_link_transfer`, `edit/persist.rs:335-350`, and siblings) are immune to this because a failed
lone append commits nothing; the batch is the first persister where "an append fails *after* prior appends
succeeded" is possible, and the shape the spec copied does not cover it.

Two things are then wrong at once: (a) the spec's own invariant — "**A mid-batch or save failure reverts
the ENTIRE batch … Never a partial apply**" (`:184-186`) — is false for the append-failure case; and (b)
the caller is told `NoChange` (its contract: "the vault is unchanged, nothing to latch") while the
in-memory DB in fact holds `k-1` phantom decisions. The opener will not latch; a later unrelated save
silently persists the phantoms. That is exactly the silent-residue class the `RolledBack`/`ResidueLive`
taxonomy exists to prevent.

**Why it gates.** It contradicts an explicit spec invariant and violates the `PersistError::NoChange`
contract on the fail-closed persist seam. Low trigger probability (an in-memory SQLite append failing) is
precisely the rationalization the persist layer was built to reject — the single-flow persisters go to
lengths to never leak residue; the batch must match them.

**Fix.** Restore `pre` on any append error before returning (mapping `CoreError → CliError` so `rollback`'s
`RolledBack`/`ResidueLive` classification applies):

```rust
let pre = session.snapshot()?;
for out_event in &out_events {
    let payload = ...;
    if let Err(e) = append_decision(session.conn(), payload, now, UtcOffset::UTC, None) {
        return Err(rollback(session, &pre, e.into())); // CoreError -> CliError -> restore(pre)
    }
}
save_or_rollback(session, pre)?;
Ok(out_events.len())
```

Add a KAT that injects an append failure at row *k>1* and asserts the re-projected log is unchanged (no
phantom links) — the batch analogue of `kat_persist_bulk_link_rolls_back_on_failed_save`. (The CLI surface
is *not* affected: it discards the whole local `Session` on error, so its `:186` claim holds as written.)

---

### [I2] IMPORTANT — `total_usd_value: Option<Usd>` cannot express the promised "≥ $X" floor

D1 struct at `design/SPEC_bulk_link_transfer.md:78`; selection rule at `:92-93`; gotcha #3 at `:206-207`;
CLI footer at `:120-122`; TUI footer at `:149`.

**Defect.** D1 defines the headline safety number as a single `total_usd_value: Option<Usd>` and specifies
(`:92-93`) that it is **`None` if ANY included row is missing a price**. But gotcha #3 and both footers
promise the total render as an honest **floor**: "**≥ $X (some prices unavailable)**". A `None` carries no
`X` — the floor sum is unrecoverable from that field. Implemented literally against D1, the CLI/TUI would
print `—`/`unavailable` with **no** floor, defeating the stated "never a false total, but always show what
we do know" safety goal. This is the feature's marquee number ("total USD reclassified non-taxable"), and
for a bulk run over recent transfers a missing daily-close price is a plausible-to-common case (the bundled
CSV, `crates/btctax-adapters/src/price.rs:9-49`, has no gap-fill: a date not present → `None`), so the
degenerate `None` total is not a rare corner.

**Why it gates.** The design's own data model contradicts its own specified output on the safety-relevant
figure. It must be resolved before implementation, not discovered when the footer renders blank.

**Fix.** Represent the total so the floor always survives: e.g.
`pub total_usd_value_floor: Usd` (Σ of the `Some` per-row values) **plus** `pub priced_row_count: usize` /
`pub missing_price_count: usize` (or a `prices_complete: bool`). Render exact `$X` when
`missing_price_count == 0`, else `≥ $X (N prices unavailable)`. (The per-row `usd_value: Option<Usd>`
fields already carry enough to derive this, so if you prefer, keep `total_usd_value: Option<Usd>` as the
*exact-or-None* convenience but state explicitly that the footer floor is summed from rows — the current
spec says neither.)

---

### [M1] MINOR — use core's `fmv_of`, not a bespoke `principal_as_btc × usd_per_btc`; and `usd_per_btc` is a trait method

`design/SPEC_bulk_link_transfer.md:43-44` and `:71`/`:85` (`usd_value = principal × usd_per_btc(date)`).

The price call **is reachable** (so this is not the infeasibility the task flagged as "likely"): `usd_per_btc`
is a `PriceProvider` **trait** method (`crates/btctax-core/src/price.rs:5-8`, impl for `BundledPrices` at
`:46-49`), and `PriceProvider` is re-exported at the crate root
(`crates/btctax-core/src/lib.rs:27 pub use price::PriceProvider`), so `prices.usd_per_btc(date)` works with
`use btctax_core::PriceProvider` in scope. But the spec's citation frames it as an inherent
`BundledPrices::usd_per_btc`, and its hand-rolled `principal_sat_as_btc × price` reinvents the vetted,
tested helper **`btctax_core::price::fmv_of(prices: &dyn PriceProvider, date, sat) -> Option<Usd>`**
(`price.rs:13-18`) — which does the sat→BTC divide with **checked** Decimal ops + `round_cents`, and
returns `None` on overflow (the exact "missing FMV → `—`" semantics the spec wants). The bespoke path skips
`round_cents`, so the preview would show un-rounded values like `$1234.56789012`.

**Fix.** Compute `usd_value = fmv_of(&prices, date, principal_sat)` (and derive the floor total by summing
the `Some`s). Same call the price tests already exercise (`price.rs:67-73`). Update the D1/grounding
citation to name `price::fmv_of` and note `PriceProvider` must be imported.

---

### [M2] MINOR — no CLI interactive-prompt precedent; factor the apply so the `y/N` stays a thin, testable shell

`design/SPEC_bulk_link_transfer.md:120-129` (interactive `y/N`), KATs at `:215-218`.

The CLI has **no** existing `stdin`/`read_line` confirmation path — the only prompt is the passphrase via
`rpassword::prompt_password` (`crates/btctax-cli/src/main.rs:421-427`). The `y/N` (default No) is a new
pattern; feasible, but the interactive branch is awkward to unit-test and the spec's CLI KATs cover only
`--dry-run` and `--yes` (`:216-218`), not the prompt. Keep the risk out of the tested core: put
plan-build + preview-render + the N-append/one-save apply in a helper that takes an already-resolved
`confirmed: bool`, and let `dispatch_reconcile` own the thin `stdin` read. This mirrors how `import_selections`
(`crates/btctax-cli/src/cmd/reconcile.rs:338-410`) already does one-session/N-append/one-save — the correct
precedent for the atomic apply.

---

### [M3] MINOR — `basis_usd`/`total_basis_usd` sum over principal **+ fee** legs while `usd_value` is principal-only

D1 at `design/SPEC_bulk_link_transfer.md:72-73`, `:78`, `:86`.

`PendingTransfer.legs` are the lots consumed for `total_sat = principal + fee_sat`
(`crates/btctax-core/src/project/fold.rs:710-734` — `consume_fifo(&key, total_sat)`), so
`basis_usd = Σ leg.usd_basis` includes the **fee-sat** basis. But `usd_value` uses `principal_sat` only.
The two advisory numbers therefore measure different sat quantities. Under the mandated TP8-(c) treatment
(non-taxable, basis carries) including the fee basis in "basis carried" is defensible, but the mismatch
should be called out (or aligned) so the preview isn't read as "value vs basis of the same coins." Also
note the leg field is `usd_basis`, not `basis` (state.rs:190-195) — minor naming in `:35`/`:86`.

---

### [M4] MINOR — opener framing: the non-empty check + wallet union read `snap` directly (like `l`), not via the Session helper

`design/SPEC_bulk_link_transfer.md:135-136` and `:208-209` say the TUI opener does its selection "ONLY
through the Session plan helper." In practice the dest pick-list and the "pending non-empty" guard read
`snap.state.pending_reconciliation` / `snap.events` **directly**, exactly as `open_link_transfer_flow`
does (`crates/btctax-tui-edit/src/main.rs:3720-3750` and the wallet union at `:3776-3786`) — which is
already KAT-G1-clean (reading snapshot state is not a forbidden token). Only the USD-enriched plan
(step 2→3) needs `session.bulk_link_transfer_plan(...)`. The claim isn't wrong about KAT-G1, but the
"selection only via the helper" wording overstates it; state the split (snap-read for the guard/wallet
union; Session helper for the priced plan) so the implementer doesn't route the wallet union through the
helper unnecessarily.

---

### [N1] NIT — citation line-drift

Line numbers decay (the spec acknowledges this), but for round-2 accuracy: `is_revocable_payload` is at
`crates/btctax-tui-edit/src/edit/form.rs:853` (spec `:55` cites `form.rs:841`; 841 is a struct field
doc-comment) — `TransferLink` is confirmed in the `matches!` at `form.rs:857`. The `#7` effective-allocation
exclusion is at `crates/btctax-tui-edit/src/main.rs:2559-2583` (spec `:57` cites `main.rs:2544`; 2544 is
the snapshot guard inside `open_void_flow`, which starts at `:2536`). Substance of both claims verified.

### [N2] NIT — `source_wallet` is always `Some` for pending outs (the same-wallet `None` case is dead)

A wallet-less `TransferOut` never reaches `pending_reconciliation` — the `Op::PendingOut` fold arm adds an
`UncoveredDisposal` blocker and `return`s *before* pushing (`crates/btctax-core/src/project/fold.rs:698-708`).
So `BulkLinkRow.source_wallet: Option<WalletId>` (`:70`) will never be `None`, the same-wallet guard's
`source_wallet == Some(dest)` (`:89`) never has to consider `None`, and the fold's "self transfer without
source wallet" blocker (`fold.rs:742-752`) is unreachable for bulk rows — corroborating the "no blocker arm
normally reachable" claim (`:176-178`). The defensive `Option` is fine; just note it can't be `None` here.

---

## Verified sound (the pressure-test items)

1. **Selection boundary (core safety).** VERIFIED. A `TransferOut` in `links` projects as `Op::SelfTransfer`
   (`crates/btctax-core/src/project/resolve.rs:201-216`), and only `Op::PendingOut` pushes to
   `pending_reconciliation` (`fold.rs:729`; `Op::SelfTransfer` at `:742` does not). `PendingTransfer {
   event, principal_sat, fee_sat, legs }` and `PendingLeg { lot_id, sat, usd_basis, acquired_at }` confirmed
   (`state.rs:190-202`); legs carry per-lot basis. Bulk can never re-touch a decided/linked out. **No Critical.**
3. **Atomicity — CLI.** SOUND. One `Session`, N `append_decision` (each `decision_seq = MAX+1`,
   `persistence.rs:246-250`), one `save()`; a mid-batch failure drops the local session → in-memory conn
   discarded, disk untouched. `import_selections` is the exact precedent (`cmd/reconcile.rs:338-410`).
   **TUI** append-failure hole is [I1]; the save-failure path via `save_or_rollback` (`edit/persist.rs:75-80`)
   is correct.
4. **KAT-G1.** SOUND. `persist_only_tokens` = `conn(`/`save(`/`append_`/`restore(`/… (`edit/persist.rs:1270-1278`);
   putting the batch append in `edit/persist.rs` and the opener/plan behind `session.bulk_link_transfer_plan`
   keeps main.rs token-free — same pattern as `optimize_proposal`/`safe_harbor_residue` (`session.rs:158-221`).
5. **Same-wallet skip / empty guards / revocability.** SOUND (modulo N2). `TransferLink` ∈
   `is_revocable_payload` (`form.rs:853-864`) → per-row `v` works, justifying explicit-confirm over
   typed-word. Empty-plan / all-unchecked refusals specified at every gate.
6. **CLI clap + TUI wiring.** SOUND. `enum Reconcile` (`crates/btctax-cli/src/main.rs:207-216`), `--year`
   XOR `--from/--to` via `conflicts_with_all`/`requires` is coherent; `parse_wallet_id`
   (`eventref.rs:57-76`, grammar `exchange:P:A` | `self:LABEL`) reachable; dispatch arms may `return Ok(())`
   early (`main.rs:941-981`) so the preview/prompt/summary flow fits. `b` is free in Browse
   (`main.rs:307-320`: `p c o r f v s d l u a A i z` + nav). Dest = full `snap.events` wallet union
   (`main.rs:3776-3786`). Frame filters by `tax_date`; `TaxDate = time::Date` (`conventions.rs:10`) so
   `.year()` and `from ≤ date ≤ to` are valid.
7. **Lockstep NONE.** VERIFIED. Crates = adapters/cli/core/store/tui/tui-edit; **no** `docs/manual/`, **no**
   GUI crate, **no** `schema_mirror`. (The read-only `btctax-tui` viewer renders no reconcile-mutation
   surface, so the additive bulk feature needs no viewer mirror either.)
8. **Interactions.** SOUND. Single `l` shares `pending_reconciliation` + payload shape; `v`/`#7` list
   bulk links normally (`main.rs:2577-2599` keeps revocable non-effective-allocation decisions);
   TP8-(c) treatment untouched (bulk only applies the existing `TransferLink`).

---

## Scope adjudication

### Fork A — TUI date input → **KEEP AS SCOPED** ("All + pick-a-year"; free-text `--from/--to` CLI-only)

Arbitrary sub-year ranges add real cost (a validated two-field date-entry sub-step with error surfacing —
a new input mode over the `TargetList` pick substrate) for low marginal value: the dominant TUI use case is
"catch up on a tax year's self-transfers," which `Year(y)` covers. Crucially, **the step-3 per-row exclude
checklist is already the fine-grained escape hatch** — a user who selects a whole year and wants only part
of it simply unchecks the out-of-range rows, and every created link is individually voidable besides. So
sub-year precision is reachable in the TUI *without* a date widget, and the CLI covers true ranges. Keep;
the deferral is already recorded (`:236-237`, `:239-242`).

### Fork B — TUI destination → **EXPAND (strongly recommended)**: add a typed-wallet entry alongside the pick-list

I verified the sharp point the coordinator raised. The TUI destination list is built **solely** from
`snap.events[].wallet` values (`crates/btctax-tui-edit/src/main.rs:3776-3786`, the chunk-4a R0-I2
limitation). In the feature's **headline** scenario — exchange outflows → a cold wallet
(`WalletId::SelfCustody { label: "cold wallet" }`) — the destination cold wallet **frequently has no
imported events** (users import exchange data but not their cold wallet's on-chain receipts; that missing
`TransferIn` is *why* those outs are pending in the first place). So the pick-list would, in the common
case, **not contain the intended destination**, forcing the user to the CLI for exactly the use case the
feature exists to make easy — gutting the "both surfaces" value.

The expansion is cheap: the parser already exists and already produces cold wallets —
`eventref::parse_wallet_id("self:cold-wallet")` → `SelfCustody { label: "cold-wallet" }`
(`crates/btctax-cli/src/eventref.rs:57-76`), the same call `--to-wallet` uses; and the editor already has
free-text input infra (profile / set-fmv / classify forms). Value is high, cost is one text field + an
existing parser.

**Minimal spec adjustment (D3 step 1).** Offer, in addition to the known-wallet pick-list, a "type a new
destination wallet" affordance — e.g. an `n` key or a sentinel top row `+ type a destination wallet…` —
that opens a one-line input parsed by `eventref::parse_wallet_id` (full grammar for parity with
`--to-wallet`; at minimum `self:LABEL`). On parse error, set a status and stay on the step. Drop the D3
footer note that pushes users to the CLI for a new destination (`:142-143`). Add KAT
`kat_bulk_typed_dest_cold_wallet` (type `self:cold-wallet` → the included outs project as `SelfTransfer`
into that never-seen wallet). This lifts R0-I2 **for the bulk flow only**; record a FOLLOWUP to backport
the same typed-dest affordance to the single `l` flow (out of scope here). This is a scope change, not a
blocking finding — but I recommend taking it, since without it the TUI misses the feature's primary use case.

---

**R0 disposition:** BLOCKED on **I1** and **I2**. Both have small, well-scoped fixes; fold them (persist
verbatim first), adopt the Fork B expansion if the user agrees, and re-review to 0C/0I.
