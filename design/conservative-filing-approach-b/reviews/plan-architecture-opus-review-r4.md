# Plan review — architecture lens (Opus), round 4

**Artifact:** `design/conservative-filing-approach-b/IMPLEMENTATION_PLAN.md` @ `5900abe` (branch `feat/conservative-filing-b`)
**Reviewer:** independent software-architecture lens (Opus 4.8), round 4. The r3 arch fold (four-site `FoldCtx`, optimizer KAT, N-1, N-2) re-derived from source at HEAD `5900abe`, not trusted from the prior verdict. Focus: buildability of the r3 fold + a fresh pass for any NEW defect it introduced.

## Verdict

**GREEN — 0 Critical / 0 Important / 0 Minor / 1 Nit.** The r3 blocker (I-1, the four-site `FoldCtx`) is genuinely and completely folded; M-1 (verify-drift `VerifyReport`-surface KAT), N-1 (`consume_fee`-param at :362), and N-2 (four-site framing) are resolved. Every load-bearing site verified against source: `FoldCtx` has exactly four construction sites, all four are now named in T4 and each is populated from a `Resolution` in scope (three sites) or the threaded param (the fourth), `optimize.rs` is untouched by signature, and the optimizer KAT exists. The plan is buildable exactly as written. One cosmetic Nit (a stale two-site commit message) — records, does not gate.

---

## Verified resolved (r3 findings — do not re-raise)

- **arch r3 I-1 (four `FoldCtx` sites) — RESOLVED, verified against source.** `grep "FoldCtx {"` returns exactly four sites: `fold` (fold.rs:413), `pools_before` (fold.rs:463), `state_as_of` (fold.rs:520), `universal_snapshot` (transition.rs:60). T4 (plan :411-428) now names all four:
  - The three fold.rs sites each take `mut res: Resolution` (`fold` :376, `pools_before` :450, `state_as_of` :505) — verified `res.promotes` is in scope at each; the plan populates them from `res.promotes`. Adding `promotes` to `FoldCtx` (fold.rs:21) forces all four via E0063, and the field addition stays inside fold.rs + transition.rs.
  - `universal_snapshot` (transition.rs:37) takes NO `Resolution` — it takes `elections`/`selections` as discrete params, so the plan correctly threads a **new** `promotes: &PromoteSet` param (not `res.promotes`). Its single call site is resolve.rs:1286, inside `resolve`'s `for (_seq,d) in &decisions` loop (:1260); `promotes` (built by T3 before the step-2 timeline loop at :1076) is in scope there. Called once per allocation → `&promotes` (immutable borrow) is correct.
  - **`optimize.rs` untouched by signature — CONFIRMED.** `available_lots_before` → `pools_before(res, prices, config, disposal)` (optimize.rs:316); `consult_sale` → `state_as_of(res, prices, config, at)` (optimize.rs:1249). Both pass `res` by value; the `promotes` threading is entirely inside `fold`/`pools_before`/`state_as_of`. No new file, no optimizer edit — as the plan claims.
  - **Optimizer KAT exists.** `the_optimizer_sees_the_clamped_promoted_basis_not_a_phantom` (plan :506-514) exercises `optimize::consult_sale` (→ `state_as_of`) over a promoted below-floor disposal; mutation-kill (:545) names `&PromoteSet::new()` at any `FoldCtx` site reds it + the snapshot KAT. `pools_before` correctly gets no behavioral KAT (compile-pinned only).
- **arch r3 M-1 (verify-drift `VerifyReport` surface) — RESOLVED.** T11 (plan :1103-1105) adds a CLI-surface test in `promote_cli.rs`: `verify` (with the threaded `PriceProvider`) returns a `VerifyReport` whose `drift` field is non-empty for a drifted promote — pinning the wire into `build_verify`/`verify`, not just the core fn.
- **arch r3 N-1 (site :362 param) — RESOLVED, verified.** `consume_fee` (fold.rs:323) takes `config: &ProjectionConfig`, NOT `ctx` — confirmed against source. The `make_disposal_legs` call at :362 is INSIDE `consume_fee`, so `&ctx.promotes` is out of scope; the plan (:416-418) correctly routes `consume_fee`'s forwarded `promotes` param (T5) there. The other seven builder sites are inside `fold_event` (:554-:1293, takes `ctx: &FoldCtx`) → `&ctx.promotes` is in scope.
- **arch r3 N-2 (four-site framing) — RESOLVED in prose.** Status header (:9-10), T4 Files (:411-413), and the r3-fold self-review (:1463-1473, "the four-site framing corrected here") all name the true four-site count. (One stray commit-message line survives → Nit-1.)
- **tax r3 I-1 (BG-D4 `clamp(net−documented)`) — buildable (arch check only).** `clamped_leg_basis` signature (plan :451-453: `Option<&PromoteEntry>, Sat, Usd, Usd -> Usd`) is unchanged from r3; only the body bound changed to `max(net − documented_share, 0)` (:525). The call site `clamped_leg_basis(promotes.get(&c.lot_id.origin_event_id), c.sat, c.gain_basis, net_share)` type-checks: `Consumed` has `lot_id`/`sat`/`gain_basis` (pools.rs:292-294), `promotes.get` yields `Option<&PromoteEntry>`. Buildable. (Formula correctness = tax lens.)

Re-verified against HEAD: the eight builder call sites at the exact plan lines — make_disposal_legs :362/:635, make_removal_legs :1118/:1195, consume_fee :641/:832/:1122/:1199 (grep-confirmed). `Resolution` (resolve.rs:201) has no `promotes` field yet (T3 adds it), field-set otherwise as cited. The `PromoteSet` ownership chain (T2 defines → T3 `Resolution.promotes` + `live_promotes` → T4 all four `FoldCtx` sites → T5/T6 builder params) is closed end-to-end with one owner per type.

---

## Nit

### N-1 (T4 Step 5) — the commit message still says the two-site count.

Plan :535 reads `…thread PromoteSet into BOTH FoldCtx sites (fold + universal_snapshot)` — the exact two-site framing r3 N-2 asked to sweep, surviving in a message an implementer pastes verbatim into git history. The Files/Steps (:411-428) and self-review (:1473) correctly thread all four, so **code will be correct** and buildability is unaffected; this is the last uncorrected instance of the two-site mental model. Fix: `…into ALL FOUR FoldCtx sites (fold + pools_before + state_as_of + universal_snapshot)`. Non-gating.

---

## Buildability statement

Verified buildable exactly as written at `5900abe`. `FoldCtx` gains `promotes`; all four construction sites are enumerated and populated (three from `res.promotes` in scope, the fourth from a threaded `&PromoteSet` param whose call site has it in scope); `optimize.rs` is signature-invariant (no new file); the eight builder call sites resolve `promotes` from `&ctx.promotes` (seven, inside `fold_event`) or the forwarded `consume_fee` param (:362); the optimizer and `VerifyReport`-surface KATs exist. No NEW buildability or decomposition defect introduced by the r3 fold. The single open item is a cosmetic Nit. **This lens is GREEN 0C/0I — the gate can close.**
