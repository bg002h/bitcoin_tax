# R0 spec review — bulk self-transfer (`bulk-link-transfer`) — round 2

**Artifact:** `design/SPEC_bulk_link_transfer.md` (round-1 folds applied)
**Baseline:** branch `feat/bulk-link-transfer`; source re-verified at HEAD.
**Reviewer:** independent adversarial architect (did NOT author the spec).
**Prior:** `reviews/R0-spec-bulk-link-transfer-round-1.md` (0C / 2I / 4M / 2N).

## Verdict: **0 Critical / 0 Important / 0 Minor / 1 Nit — R0-GREEN ✅**

All of round 1 folded correctly with no new drift. The two Important findings are resolved and I re-derived
each against current source. One non-blocking clarity Nit remains (documentation only). The spec passes the
R0 gate; implementation may proceed.

---

## Re-verification of each fold

### [I1] — mid-batch append rollback — **RESOLVED (fix is correct)**
`SPEC:189-204`, atomicity bullet `:217-221`, gotcha #5 `:247-248`, KAT `:259-260`.

The coordinator asked specifically whether the rollback fix is correct given `append_decision`'s per-call
commit and `rollback`'s signature. It is — verified end to end:

- **Per-call commit is real.** `append_decision` opens `conn.unchecked_transaction()` and `tx.commit()`s
  each call (`crates/btctax-core/src/persistence.rs:245-261`), so appends `1..k-1` are live in the in-memory
  conn when append *k* fails. Reverting the whole batch is therefore *required*, not optional. ✔
- **`rollback` signature matches.** `fn rollback(session: &mut Session, pre: &[u8], err: CliError) ->
  PersistError` (`crates/btctax-tui-edit/src/edit/persist.rs:62-71`). The spec calls
  `rollback(session, &pre, e.into())`:
  - `&pre` — `pre: Vec<u8>` coerces `&Vec<u8> → &[u8]`. ✔
  - `e.into()` — `e` is the `append_decision` error, `btctax_core::CoreError`; the target `CliError` has
    `Core(#[from] btctax_core::CoreError)` (`crates/btctax-cli/src/lib.rs:22`), which derives
    `From<CoreError> for CliError`, so `.into()` resolves to `CliError`. ✔
  - Result is `PersistError::RolledBack(err)` on clean revert or `ResidueLive(err)` if the revert itself
    fails — **never a bare `NoChange` over live phantoms**, which was the exact defect. ✔
- **Borrow/move discipline is sound.** The `session.conn()` immutable borrow ends when `append_decision`
  returns (the owned `CoreError` holds no connection borrow), so the subsequent `&mut session` for
  `rollback` is legal under NLL. `&pre` is used only on the early-return path; `pre` is moved into
  `save_or_rollback(session, pre)` only on the fall-through path — no move-then-use conflict. ✔
- **Both callable helpers are in-module.** `rollback` and `save_or_rollback` live in `edit/persist.rs`
  alongside `persist_bulk_link_transfer`, so no visibility problem. ✔

Atomicity bullet, gotcha #5, and the new `kat_persist_bulk_link_reverts_mid_batch_append_failure` (inject a
failing append at row k>1 → `Err`, log byte-unchanged, no residue, retry clean) are all consistent with the
code. Round-1 [I1] is closed.

### [I2] — honest USD floor — **RESOLVED (design holds)**
`SPEC:88-90`, rule `:108-110`, D2 footer `:141-142`, gotcha #3 `:241-242`.

`BulkLinkPlan` now carries `total_usd_value_floor: Usd` (Σ of the priced rows' `Some` values) +
`missing_price_count: usize`, replacing the un-expressible `Option<Usd>`. Because per-row `usd_value` is now
`fmv_of(...) -> Option<Usd>` (cent-rounded), the floor is always a real, correctly-rounded number, and the
renderer shows exact `$X` when `missing_price_count == 0`, else `≥ $X (N unavailable)`. The marquee number
can no longer collapse to a blank. The floor design is expressible and internally consistent across D1, D2,
and gotcha #3. Closed.

### [M1] — `fmv_of` not a hand-rolled multiply — **RESOLVED**
`SPEC:47-51`, `:80`, `:98-99`. Signature confirmed: `fmv_of(prices: &dyn PriceProvider, date: TaxDate, sat:
Sat) -> Option<Usd>` (`crates/btctax-core/src/price.rs:13`). `&prices` (`&BundledPrices`) coerces to
`&dyn PriceProvider`; overflow→`None` + `round_cents` are exactly the "missing → `—`" semantics wanted. The
note that `usd_per_btc` is the `PriceProvider` trait method (`price.rs:46`, reachable via
`use btctax_core::PriceProvider`) is accurate. Closed.

### [M2] — CLI apply as a `confirmed: bool` helper — **RESOLVED (see Nit)**
`SPEC:136-152`. The apply is a pure helper taking `confirmed`, with the `y/N` read in dispatch (no
`stdin`-confirm precedent beyond `rpassword`; `import_selections` `cmd/reconcile.rs:338` is the correct
one-session/N-append/one-save precedent). Coherent under a two-call reading (build+render+prompt, then apply)
— see the Nit for a one-line clarification.

### [M3] / [M4] / [N1] / [N2] — **all RESOLVED**
- [M3] basis note added: `basis_usd = Σ leg usd_basis` over principal+fee sats vs principal-only `usd_value`;
  field is `usd_basis` (`SPEC:82`, `:100-102`). ✔
- [M4] opener framing split: non-empty guard + wallet union read `snap` directly (KAT-G1-clean, like
  `open_link_transfer_flow`); only the priced plan uses the Session helper (`SPEC:243-246`). ✔
- [N1] citations corrected and re-verified: `is_revocable_payload` at `form.rs:853` (contains
  `EventPayload::TransferLink`); `#7` filter at `main.rs:2559-2583` (`SPEC:61`, `:63`). ✔
- [N2] `source_wallet` always-`Some` for pending outs documented (`SPEC:77-78`). ✔

### [Fork B EXPAND] — typed TUI destination — **ADOPTED cleanly, no new drift**
`SPEC:10-12`, `:165-171`, out-of-scope `:281-283`, KAT `:264-265`, FOLLOWUP `:277-278`.

The headline case is now reachable in the TUI. Verified feasible and non-drifting:
- **Parser reuse:** `eventref::parse_wallet_id("self:cold-wallet")` → `SelfCustody { label: "cold-wallet" }`
  (grammar `exchange:P:A` | `self:LABEL`, `crates/btctax-cli/src/eventref.rs:57-76`) — the same call
  `--to-wallet` uses. ✔
- **Engine accepts a never-seen dest:** `build_op` maps `TransferTarget::Wallet(w) => Op::SelfTransfer {
  dest: w }` with **no** requirement that `w` appear in any event (`crates/btctax-core/src/project/resolve.rs:
  204-215`); the fold relocates principal into that wallet's pool regardless. So a cold wallet in zero events
  links correctly. ✔
- **KAT-G1 clean:** the typed-dest affordance sits in `main.rs` and uses `parse_wallet_id` — none of the
  forbidden tokens (`conn(`/`save(`/`append_`/`restore(`). ✔
- **Same-wallet skip unaffected:** a never-seen dest equals no source wallet → nothing spuriously skipped. ✔
- Footer "use the CLII" nudge dropped; `kat_bulk_typed_dest_cold_wallet` added; backport-to-`l` recorded as a
  Task-3 FOLLOWUP. Non-goals/out-of-scope updated consistently (never-seen dest moved OUT of non-goals for
  the TUI; only the free-text date RANGE remains CLI-only). ✔

---

## Remaining item (non-blocking)

### [N-r2] NIT — spell out that the CLI helper is invoked two-phase for the interactive path
`SPEC:136-152`. `cmd::reconcile::bulk_link_transfer(..., confirmed: bool, ...)` both "returns the plan for
the caller to render" and "applies only when `confirmed`". For the interactive path the preview must be shown
*before* the `y/N` is read, so `confirmed` can't be known on a single call that also builds the plan. The
coherent (and intended) shape is a two-call sequence — call once with `confirmed = false` to obtain+render
the plan (and stop for `--dry-run`), read `y/N`/`--yes`, then call again with `confirmed = true` to apply
(fresh session, N appends, one save — atomic, no TOCTOU under the single-instance vault lock). Both KAT'd
paths (`--yes`, `--dry-run`) are unaffected either way. Recommend one sentence stating the two-phase call
(or splitting into `plan()` + `apply(confirmed)`). Documentation only — does not gate.

---

## New-drift sweep — clean
No fold introduced a regression. The I1 rollback is type-correct and revert-complete; the I2 floor is fully
expressible; M1's `fmv_of` compiles against the real signature; Fork B's typed dest is engine- and
KAT-G1-safe. The core safety boundary (selection over `pending_reconciliation`, which excludes linked outs)
is unchanged and remains sound.

**R0 disposition: GREEN — 0 Critical / 0 Important.** Cleared to implement (Task 1 → Task 2 → Task 3, each
review-to-green per the standard workflow). The single Nit is an optional one-line clarification for the
implementer, not a gate.
