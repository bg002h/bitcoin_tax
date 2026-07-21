All checks complete, tree verified clean after the cp-backup/restore mutation probes. Here is the re-review verbatim.

---

RE-REVIEW VERDICT: 0 Critical, 0 Important, 1 Minor, 1 Nit

# Focused re-review of fold `69fec06` — tax lens

Verified against current source on `feat/conservative-filing` (working tree clean). All 8 fold-touched tests run green. Two live mutations performed (cp-backup/restore, tree restored and re-greened, `git status --porcelain` empty).

## 1. Refusal-message split — CONFIRMED, with one cosmetic residual

Read both constants and both call sites in `/scratch/code/bitcoin_tax/crates/btctax-cli/src/cmd/tranche.rs:21-31, 74-99`.

- **Direction-correct:** `ALLOCATION_IS_FINAL_HINT` is used only by `guard_tranche_vs_allocation` (user recording a tranche, blocked by an allocation) and its finality caveat is about the *filed allocation* — correct artifact. `TRANCHE_IS_FINAL_HINT` is used only by `guard_allocation_vs_tranche` (user recording an allocation, blocked by a tranche) and its caveat is about the *tranche's filed $0 basis* — correct artifact. The r1 Nit's crossed reading is gone.
- **Provenance-neutral / hedged:** neither message claims legal authority or IRS mandate; "v1 makes the two mutually exclusive" is app-scoped; both terminate in "facts-and-circumstances matter for a professional". The cited remedy syntax `` `reconcile void <decision-ref>` `` is real verb syntax (identical phrasing at `crates/btctax-cli/src/cli.rs:539`).
- **No tax-understating instruction:** the "void the tranche first" remedy applies to the in-app, not-yet-filed case; the already-filed-$0-basis case is explicitly carved out to a professional, not to void-and-reallocate. Unwinding a filed conservative position is never instructed. The tranche-side hint likewise routes the filed-allocation case to a professional.
- **Tests:** no test asserts the old combined string. The direction tests assert substrings — "safe-harbor"/"allocation" (`declare_tranche_cli.rs:142-146`, tranche-side) and "tranche" (`:211`, `:482`, allocation-side) — each still matches and names the *right* collision object for its direction. The backstop blocker detail at `resolve.rs:1297-1301` is untouched and remains direction-appropriate.
- Cosmetic only (not counted): the allocation-side message now reads "…mutually exclusive. void the tranche first…" — lowercase after a period.

## 2. Test (d) ≥2025 non-poisoning assertions — CONFIRMED, mutation-proven, one residual pin gap (the Minor)

Traced `post2025_tranche_records_cleanly_beside_effective_allocation` (`declare_tranche_cli.rs:228-270`). The assertions are read-only over a projection; they cannot cause or mask an understatement. I ran three mutations:

- Snapshot date filter removed (`transition.rs:51` → include-all): test **survived** — masked by `pool_key`'s independent per-wallet routing (a leaked ≥2025 Eff never lands in `PoolKey::Universal`). Defense-in-depth, not a test weakness.
- Residue sum made pool-blind (`transition.rs:76` sum over all pools): test **survived** — masked by the date filter. Same reason.
- **The exact r1-feared regression** — `has_tranche_residue` at `resolve.rs:1290` re-keyed on window-blind payload *presence* — test (d) went **RED**. The primary vector tax-r1 Minor 2 named is genuinely killed.

**[Minor — residual, fold marks the item FIXED but the pin is partial]** No test anywhere asserts Path B directly in the alloc + ≥2025-tranche coexistence. `TransitionMode` is referenced by zero tests (`Resolution.transition` is public, `resolve.rs:201-203`), and test (d) asserts blocker-absence + tranche-lot coexistence but not the allocation's Path-B seed. A contrived regression that flips this filer to Path A *without emitting a blocker*, conditioned on tranche presence, passes test (d) (with `ActualPosition` the lot positions are identical either way). Never an understatement — Path A is the conservative direction — so non-blocking, but it is the exact Rev-Proc 2024-28 flexibility strip the r1 Minor wanted foreclosed, and `FOLLOWUPS.md` records the item as FIXED. One-line close, either: assert `state.lots.iter().any(|l| l.basis_source == BasisSource::SafeHarborAllocated)` in test (d), or a core KAT asserting `matches!(res.transition, TransitionMode::PathB{..})` with an effective allocation + ≥2025 tranche. Suggested owner: fold now (one line) or T16.

## 3. New backstop KAT — CONFIRMED discriminating

`backstop_fires_when_the_allocation_is_recorded_before_the_tranche` (`kat_tranche.rs:570-601`) is a genuine seq-swap twin (`[a seq 1, t seq 2]` vs the original's `[t seq 1, a seq 2]`). The fixture conserves on totals (100M sat/$0 vs 100M/$0 residue), so the `SafeHarborUnconservable` blocker can arise *only* from the tranche-residue denial — the assert discriminates on the D-8 mechanism, not incidental non-conservation. Under an evaluate-at-declaration-time regression, the original KAT stays green (tranche pre-exists the allocation there) while the twin goes RED — it uniquely pins the r3-New-1 ordering. The lot-survival assert (`EstimatedConservative`, `remaining_sat > 0`) genuinely detects silent discard: a Path-B seed tags lots `SafeHarborAllocated` and `seed_transition`'s PathB arm discards the Universal remainder (`transition.rs:104+`), so a discarded tranche leaves no `EstimatedConservative` lot and the `expect` fails. The conservation-break / coins-vanish hazard is pinned under alloc-first ordering.

## 4. Fold is purely additive to tax-critical paths — CONFIRMED

- `cmd/tranche.rs`: message strings only; both guard predicates (`in_force_allocation_exists`, `pre2025_tranche_exists`) and the `window_end < TRANSITION_DATE` scoping byte-identical.
- `lib.rs`: comment only.
- `btctax-tui-edit/src/main.rs`: one display-only match arm in `summarize_void_payload`; both consumers (`open_void_flow` :3883, `open_bulk_void_flow` :8259) bind the 4th tuple element as `_is_sha` (ignored) and use tag/summary for row display only; `target: None` matches the CLI sibling.
- Everything else is tests + `FOLLOWUPS.md`. No fold arm, no guard predicate, no export path, no computed figure touched. D-7 ($0-only), G-4, D-2 (window_end homing), D-5 (clean export) KATs all untouched and green.

## 5. Deferred items — NONE can produce a wrong filed tax result; classifications upheld

- **build_op smuggle (P9/T15):** independently re-verified, not taken from r1 — CLI `classify_raw` refuses non-`is_imported()` payloads *before* opening the vault (`reconcile.rs:467-470`), `DeclareTranche.is_imported()` is `false` by documented design (`event.rs:352`), and the TUI constructs only `Acquire`/`Income` `as_` payloads (`form.rs:1738-1747`, `persist.rs:1833-1836`). Hand-crafted-vault only; P9/T15 ownership (pre-ship) is right.
- **Dangling-void divergence (T16):** worst case is a tranche coexisting with a still-effective allocation until the next projection — and *every* projection (including export) re-runs the T5 backstop → Hard `SafeHarborUnconservable`, Path A. Loud and conservative; a friendly-refusal miss only.
- **Residue display skew (T16):** the shadow projection feeds only the TUI allocate opener; all four record sites refuse while a pre-2025 tranche exists, and no form reads it. The skew direction *understates* documented remainders — over-conservative even if hand-transcribed.
- **`--wallet` unknown (P8/T14):** pre-2025 figures are unaffected (Universal pool is wallet-blind). Post-2025, a per-wallet FIFO draw can shift lot-assignment timing — but that is the generic wrong-user-data class (identical to a typo'd wallet on any import), the engine computes correctly from the recorded ledger, drift is lifecycle-offset and shortfall-blockered at exhaustion. Warn-not-refuse in P8 is the right remedy. Not re-classified.

## Nit (new, fold-introduced doc drift)

**[Nit]** `design/conservative-filing/SPEC.md:104` and `IMPLEMENTATION_PLAN.md:556` quote the pre-split hedge as *the* wording for a refusal that "fires in BOTH directions"; after the split, the allocation-side message no longer contains the quoted sentence (the tranche side retains it verbatim, and both directions still satisfy the normative "hedges real-world irrevocability" requirement). Doc-consistency only — align the SPEC quote to note the two direction-specific variants. Owner: T16 doc-consistency sweep.

---

**Conclusion:** the fold is clean from the tax lens — no new Critical/Important, nothing weakened, the tax-critical additions do what the commit claims (with the (d)-assertion's primary regression vector now *proven* discriminating by live mutation). The 1 Minor is a partial-fix bookkeeping residue with a one-line close; the 4 deferrals stay non-blocking as filed.
