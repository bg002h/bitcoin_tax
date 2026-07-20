# Plan Architecture Review — Conservative-Filing IMPLEMENTATION_PLAN.md (r4, convergence check)

Verification basis: fold-commit diff `a96791d~1..HEAD`; live `resolve.rs` (`resolve()` :402-406, `Resolution` :201-202 + construction :1326-1333, timeline builder :1051-1084, `sort_canonical` :1376-1383, `voided` set :439), `fold.rs` (`sort_canonical(&mut res.timeline)` :381; `finalize` lot re-sort :1296-1301), `identity.rs` (`EventId` derived `Ord` :55, `Decision { seq: u64 }` :69, `SourceRef::new(impl Into<String>)` :38), `lib.rs:11` + `project/mod.rs:6` (visibility), `tests/optimize_mode2.rs:495-510` (existing `resolve(&events, &prices, &config)` call shape).

## Disposition of NEW-I-1 — **RESOLVED** (all four checks pass against real code)

1. **Callable + mutable.** `pub fn resolve(events, prices, config) -> Resolution` (`resolve.rs:402-406`); `Resolution { pub timeline: Vec<Eff>, … }` (`:201-202`). The KAT's `let mut res = btctax_core::project::resolve::resolve(&[b, a], &prices(), &config())` matches the exact call shape already used in-repo (`optimize_mode2.rs:506`); `mut res` + pub field make `&mut res.timeline` legal.
2. **`sort_canonical` reachable.** `pub fn sort_canonical(timeline: &mut [Eff])` (`resolve.rs:1376`); `pub mod project` (`lib.rs:11`), `pub mod resolve` (`project/mod.rs:6`). No export work needed.
3. **Discrimination, all three directions, re-verified.** `resolve()` builds the timeline in raw input order (`for e in events`, `:1056`) and returns it unsorted (`:1326`; the only `sort_canonical` call sites are `fold.rs:381/457/511` and `transition.rs:49`). Input `[b, a]` = [seq 10, seq 2]:
   - **Correct code:** keys 1-3 tie; the new `.then(a.id.cmp(&b.id))` compares `Decision{2} < Decision{10}` numerically → `[2, 10]` → **GREEN** (the [10, 2] input order means the sort must actually reorder — load-bearing).
   - **Revert constant `src_ref` → `format!("{seq}")`:** key 3 decides — `"10" < "2"` String-Ord → `[10, 2]` → **RED**.
   - **Remove `.then(a.id.cmp(&b.id))`:** all keys tie; `sort_by` stable → `[10, 2]` → **RED**.
   Both mutation gates non-vacuous.
4. **Captions and Step-5(c) notes accurate.** All three caption claims verified; Task-2/Task-3 Step-5(c) point at the timeline KAT with the correct `st.lots` contrast; Task-3 Step 2-4 remediation now points the right way. `voided: BTreeSet<EventId>` in scope; `SourceRef::new("")` compiles.

## New-issue sweep

The fold delta is exactly: the KAT rewrite + discrimination comment, the Task-15 TP8c fixture footnote (tax r3 N-6 — sound), and the SPEC §3 cent-scale rounding tightening (tax r3 N-5). Nothing else moved.

**NEW-N-3 (Nit, non-gating, Task 15):** the M-5 parenthetical still says "`≤$0.01` pro-rata rounding remainder" while the amended SPEC now bounds it as "cent scale (≤ ½¢ per prior leg)" — which can exceed $0.01 for >2 legs; and its "stays documented in SPEC §6" pointer is off by a section (the characterization lives in SPEC §3's no-loss scoping). Two micro-instances of wording drift; cannot strand an implementer. Fix when convenient.

Final completeness pass: every load-bearing mechanism in all 16 tasks has a specified location, mechanism, test, and non-vacuous mutation. A fresh engineer can implement task-by-task without hitting an unspecified decision.

## Verdict

**GREEN — 0 Critical / 0 Important** (1 Nit recorded: NEW-N-3, doc-consistency). The r3 blocker is fully and correctly folded — verified against the live pipeline — and the plan is architecturally sound and complete.