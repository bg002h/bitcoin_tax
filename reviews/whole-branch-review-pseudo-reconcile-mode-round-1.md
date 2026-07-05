# Whole-diff review (Phase E) — feat/pseudo-reconcile-mode — round 1

**Verdict: 0 Critical / 0 Important / 0 Minor / 0 Nit — SHIP.**

Independent Phase-E review. Diff `main (514875b)..HEAD` — 8 task commits (T1-T6), 38 files, +2537/−36.
Contract: `design/SPEC_pseudo_reconcile_mode.md` (R0-GREEN, 3 rounds). Sub-project 2 of auto-pseudo-reconcile.

## Fault-injection of the tax-critical invariants (both restored byte-for-byte)
- **[★ C1 basis-taint] CONFIRMED load-bearing.** `fold.rs:214` threads `pseudo: c.pseudo || ev_pseudo` — a
  disposal leg is `[PSEUDO]` if the consumed lot's BASIS is pseudo (`c.pseudo`) OR the event is synthetic
  (`ev_pseudo`), so a REAL Sell (`ev_pseudo=false`) on a pseudo $0-basis lot is flagged. **Fault-inject:**
  dropping the `c.pseudo` term (leg pseudo only from the event) drove
  `real_sell_on_pseudo_lot_flags_the_disposal_leg` RED (panic pseudo_reconcile.rs:171). The bit rides
  `Lot`(+relocated)→`Consumed`→`DisposalLeg`/`RemovalLeg`/`PendingLeg` (state.rs:124/157/192/227; fold.rs:214/263).
- **[★ I3 export refusal] CONFIRMED load-bearing.** `admin.rs:56-57`: `if state.pseudo_active() { return
  Err(PseudoActiveExport(count)) }` refuses `export_snapshot` while any synthetic contributes. **Fault-inject:**
  `if false` drove `export_snapshot_refused_while_pseudo_active` RED (panic pseudo_reconcile_cli.rs:144). No
  fictional 8949/Schedule D leaves the machine unguarded until sub-3's typed-attest gate replaces this.

## The other invariants (verified by inspection + named KATs)
- **on-screen-yes / output-no** — `pseudo_marker_on_screen_but_absent_from_every_export_file`: renders `[PSEUDO]`
  (incl. the C1 basis-taint leg) then greps every written CSV/form for `pseudo`/`synthetic` → asserts NONE. The
  marker is a dedicated `pseudo` bool the writers OMIT (NOT a `BasisSource::Pseudo` variant — the R0-I4 leak trap).
- **not persisted** — `pseudo_projection_persists_no_events` (`load_all` count unchanged across repeated pseudo
  projections; only `approve` writes). Projection is a pure read (R0-verified session.rs:446-466).
- **real supersedes** — `real_decision_supersedes_no_synthetic_injected`; **mode-off byte-identical** —
  `mode_off_is_byte_identical_blockers_intact`; **determinism** — `pseudo_projection_is_deterministic`.
- **I2-precise** — `tax_total_computes_when_pseudo_clears_all_hard_blockers` +
  `no_tax_total_while_a_non_classification_hard_blocker_remains`. Injection is map/`Eff`-layer (resolve.rs) — no
  `EventId::Decision{seq}` minting (R0-I1). Bulk-approve uses the btctax-cli `apply_bulk_pseudo_approve` OWN-LOOP
  (R0-M4, no tui-edit dep cycle).

## The reviewer-flagged judgment call (R0-M2) — SOUND
Determinable `Unclassified` inbounds (has-wallet) default via a synthetic `ClassifyRaw` → a **$0/0-sat**
`Op::Acquire` placeholder (resolve.rs:921-930): it clears the Hard `Unclassified` blocker WITHOUT fabricating
holdings (0 sat) — the honest reading given `Unclassified{raw:String}` carries no structured amount; the user
supplies the real classification+amount on correction. Wallet-less `Unclassified` is left surfaced. Correct.

## Full suite
`cargo test --workspace --locked` (implementer: 1132 passed / 0 failed; re-run clean — see merge) + clippy -D +
fmt. The two fault-injections above required a compile + run of the affected crates (both KATs are real + bite).

**SHIP.**
