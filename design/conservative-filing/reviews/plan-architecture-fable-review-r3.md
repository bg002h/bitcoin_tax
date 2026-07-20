# Plan Architecture Review — Conservative-Filing IMPLEMENTATION_PLAN.md (r3, convergence check)

Verification basis: `resolve.rs` (full `resolve()` body :402-926, timeline builder :1051-1085, pass 1c :556-577, `sort_canonical` :1376-1383), `fold.rs` (`fold()` :376-390, `finalize` :1285-1305), `identity.rs` (full), `event.rs:204-207` (`ClassifyRaw`), `lib.rs`/`project/mod.rs` (visibility), existing test usage (`tests/optimize_mode2.rs:506`).

## Disposition of r2 findings

**NEW-I-1 (vacuous ordering KAT) — NOT-RESOLVED: the fold did exactly what my r2 prescription asked, and my prescription was itself incomplete. The re-pointed KAT is now RED-on-correct-code.**

What the fold got right (all verified):
- The split is honest and correct. `finalize` (`fold.rs:1296-1301`) re-sorts `st.lots` by `(wallet, acquired_at, lot_id)`, and `LotId` derives `Ord` comparing `origin_event_id` first (`identity.rs:116-119`) with `Decision{seq: u64}` numeric (`identity.rs:56,69`) — so the additivity KAT is correctly captioned as pinning additivity + the *observable* order via `lot_id`, NOT the sort fix.
- Both Step-5(c) mutations (Task 2, Task 3) now target the timeline KAT.
- `resolve()` is `pub` (`resolve.rs:402`), returns `Resolution { pub timeline: Vec<Eff>, … }` (`resolve.rs:201-202`), reachable from integration tests today (`tests/optimize_mode2.rs:506` already calls it).

The residual defect: **`resolve()` never applies `sort_canonical`.** A grep of `resolve.rs` finds zero invocations (only the definition at :1376); the canonical sort runs in the *caller* — `fold()` at `fold.rs:381` (`sort_canonical(&mut res.timeline)`), followed by the stable transition partition at :387. `resolve()` builds the timeline in raw input order (`for e in events`, `resolve.rs:1056` — and the Task-2 admit is placed inside that same loop) and returns it unsorted (`Resolution { timeline, … }` at :925-926, no intervening sort). The KAT feeds `&[b, a]` = [seq-10, seq-2], so on **correct** code `res.timeline` yields `[10, 2]` and `assert_eq!(seqs, vec![2, 10])` is RED. Consequences:
1. Task 3 Step 2-4 says "run to confirm GREEN. If any is RED, that finding wasn't folded — fix it" — the implementer is stranded with remediation text pointing the wrong way.
2. Both Step-5(c) "must go RED" claims become *vacuously* true — the KAT is RED before AND after the mutation, so the "mutation dies" gate reports success while certifying nothing. The exact defect class NEW-I-1 was filed for.
3. The KAT caption ("sort_canonical is only observable on the timeline itself") embeds the false architectural claim that `res.timeline` *is* sort_canonical's output.

This is not sketch-elision covered by the N-1 illustrative-helpers note — it is a wrong semantic assumption in a load-bearing test, its caption, and two normative mutation steps. I own that the r2 fix text contained the same omission; the author folded it faithfully. It still gates.

**Fix (one line + caption/mutation-note edits, no redesign):** the KAT composes the production pipeline the way `fold()` does:
```rust
let mut res = btctax_core::project::resolve::resolve(&[b, a], &prices, &config);
btctax_core::project::resolve::sort_canonical(&mut res.timeline);
// then the existing seqs extraction + assert_eq!(seqs, vec![2, 10])
```
`sort_canonical` is `pub fn` (`resolve.rs:1376`) and both `project` (`lib.rs:11`) and `resolve` (`project/mod.rs:6`) are pub modules — no export work needed. Post-fix discrimination re-verified against code, all three directions:
- Revert the constant `src_ref` → `format!("{seq}")`: src_refs "10" < "2" under String-Ord at key 3 → `[10, 2]` → RED.
- Remove `.then(a.id.cmp(&b.id))`: all three keys tie; `sort_by` is stable → preserves push order `[10, 2]` → RED.
- Correct code: id key compares `Decision{2} < Decision{10}` numerically → `[2, 10]` → GREEN.
Update the caption to "canonical order = `sort_canonical`, applied by the fold pipeline at `fold.rs:381`; the KAT composes it explicitly because `resolve()` returns the timeline unsorted", and re-word both Step-5(c) notes accordingly.

**NEW-N-1 (inert fixture unenumerated) — RESOLVED.** Task 6 Step 1 now enumerates "(a2) pre-2025 tranche refused under an **inert** allocation". Under mutation 5(b) (effective-only scope) the (a2) fixture records instead of refusing → RED. Discriminates.

**NEW-N-2 (admit reads `applied`) — RESOLVED.** The admit now destructures `(&e.id, &e.payload)` directly and `build_op` takes `&e.payload`. Verified: pass 1c (`resolve.rs:556-577`) inserts with no target-type validation and `ClassifyRaw.as_` is an unconstrained `Box<EventPayload>` — so bypassing `applied` closes both the suppress and forge doors on the decision admit. The legitimate flow is unbroken (a real DeclareTranche decision never appears in `applied`); `voided` still honored. Residual noted, NOT a finding: a hand-crafted ClassifyRaw targeting an Unclassified *import* can still smuggle a payload through the import arm — pre-existing, strictly broader than tranches, and a forged $0-basis tranche is self-adverse, never an understatement.

## New-issue sweep (fold-introduced)
- The `build_op(&e.id, &e.payload, /* … */)` elision of the map params is covered by the N-1 illustrative-sketch convention.
- Anchor drift trivial (builder loop at :1051-1085 vs cited :1055-1083).
- `src_priority: u8::MAX`, the voided-guard placement, and the Task-5 backstop placement unchanged and verified.
- Nothing else unsound found; apart from the one finding, the plan is architecturally complete and implementable task-by-task.

## Verdict

**NOT-GREEN — 0 Critical / 1 Important.** The single blocker is the incomplete NEW-I-1 fold: the timeline KAT must compose `sort_canonical(&mut res.timeline)` after `resolve()` (which returns the timeline unsorted — the canonical sort lives in `fold()` at `fold.rs:381`), plus the matching caption and Step-5(c) note corrections in Tasks 2 and 3. The defect traces to my own r2 fix prescription — the author folded it faithfully — but it would strand an implementer at a guaranteed-RED step with misleading remediation text and reduce both mutation gates to vacuous passes, so it gates. It is a one-line-plus-captions fix; with it folded, I see nothing else between this plan and GREEN.