# Architecture review — Defensive Filing SPEC (r2, Opus lens)

**Artifact reviewed:** `design/defensive-filing-wizard/SPEC.md` (binding decisions DFW-D1..D12),
commit `0502de4` on `feat/defensive-filing-wizard`.
**Stance:** independent SPEC-level architecture review, re-derived from the current tree. I am a
different model from r1 and from the two brainstorm critiques; every load-bearing claim below was
re-checked against real source, not carried forward. My job: (a) audit whether each r1 architecture
finding is genuinely resolved by the fold, and (b) find any NEW spec-level architectural defect the
fold introduced.

---

## Verdict

**NOT GREEN** — **1 Critical / 0 Important / 2 Minor / 0 Nit**.

The r1 fold is faithful: all four r1 Importants (I-1..I-4) and all five r1 Minors/Nits (m-1..m-3,
n-1..n-2) are genuinely resolved at the SPEC level, verified against source (audit below). The
three-seam architecture holds, the DFW-D2 gate ordering still matches the shipped pipeline exactly
(`cmd/promote.rs:378-485`), and `apply(&mut Session, Plan, acknowledge)` is implementable — `Session`
owns the vault lock + prices (`session.rs:331-420`), the TUI already holds one, and the one-flow
invariant exists as comment-law (`editor.rs:116`). But formalizing DFW-D5.3's over-coverage refusal
introduced ONE new Critical: the phantom-refusal predicate is **over-constrained** and, placed on the
shared promote chokepoint, **refuses a shipped, CLI-tested, BG-D6-guaranteed operation** (promoting a
fully-undisposed tranche). It is the exact "moved/weakened guarantee at the extraction seam" class
this review exists to catch — and, like r1's I-2, it lacks the CLI-vs-dashboard carve that would
contain it.

---

## r1-resolution audit (verified against source)

| r1 finding | status | evidence |
|---|---|---|
| **I-1** triage not total; §2 prose⊂def | **RESOLVED** | §2 reworded to "BTC that **left the filer's records with no acquisition record**" and names self-transfer/fee/gift/donate shorts. DFW-D4 keys on **structural `short_sat` presence**. Re-derived `fold.rs`: exactly **6 sat-carrying** emitters (`:388,712,833,878,1198,1276`) → coverable; **9 non-sat** (5 without-wallet `:691,819,864,1177,1255` + 4 degenerate `:742,935,1225,1303`) → data-fix by absence. Total=15, partition is total by construction. Pending-out short co-emits `UnmatchedOutflows` on the same event (`:831` + `:854`) — the DFW-D4.1 "route through it first" exception is real. `pool_key(date,wallet)` + `date ≤ short-op date` well-defined (`pools.rs:15`). §5 KATs extended to gift/donate-without-wallet (ZERO candidates). |
| **I-2** clearance vs behavior-preserving declare | **RESOLVED (declare)** | DFW-D5.2 `plan(…, target_shortfall: Option<EventId>)`; dashboard→`Some` (clearance), CLI→`None` (shipped gate set byte-for-byte). `declare_tranche` gates only on `sat>0`/window-order/`guard_tranche_vs_allocation` (`tranche.rs:135-154`) — preserved. Clearance forces `pseudo_reconcile=false`. **But DFW-D5.3 reopens the identical contradiction for PROMOTE — see C-1.** |
| **I-3** ack residency unspecified | **RESOLVED** | DFW-D2 pins `apply(&mut Session, Plan, acknowledge: Option<&str>)`, `require_promote_ack` **inside** `apply`, fail-closed (`None` refuses; distinct `None`-vs-`Some(wrong)` preserved — `promote.rs:346-357`), before `would_conflict`→append; drivers only COLLECT. Declare/export members explicitly have no ack. Feasible: `apply(&mut Session)` mutates via `session.conn()`/`save()`. |
| **I-4** didn't-cover not derivable | **RESOLVED** | DFW-D5.3 pins the pool-level predicate: row enters "still short" iff a live `DeclareTranche` with `pool_key(window_end,wallet)` = shortfall pool AND `window_end ≤ short-op date`; ONE pool state, no per-tranche attribution, no persisted state. Fully derivable. |
| **m-1** `Coverage::None` absent | **RESOLVED** | DFW-D9 names `Coverage::Partial` + `NoCoverage`/`PartialCoverage` **refusal outcomes**; "the enum is `{Full,Partial}` — no `None` variant." Matches `conservative.rs:217-221`, `conservative_promote.rs:56-68`. |
| **m-2** single-source year enum | **RESOLVED** | DFW-D11 pins extracting `promoted_filing_years(state)` for the gate `None`-arm (`admin.rs:84-98`) + 8275-completeness callers; explicitly **NOT** the export set. Clean two-set split. |
| **m-3** `shown_terms` structural not bytes | **RESOLVED** | DFW-D2: rendered copy + advisory/refusal **byte-identical**; `shown_terms` (`Vec<ConsentTerm>`, `event.rs`) equal by **structural `Eq`**. |
| **n-1** parity KAT altitude | **RESOLVED** | DFW-D2 + §5: drive **both full driver paths** (CLI verb fn AND TUI persist), compare recorded artifacts + captured output; mutation = a driver re-wrapping/bypassing the chokepoint. |
| **n-2** export trio degenerate | **RESOLVED** | DFW-D2: export `plan`=gates over `&Session`/state, `apply`=write files, no consent/ack/`Plan`. |

Nothing dropped or diluted. C-1 below is new, surfaced by formalizing DFW-D5.3.

Also verified HOLDING: `would_conflict` forces `pseudo_reconcile=false` (`project/mod.rs:119`); the
DFW-D6 latent gap is real — `promote_tranche` folds the *stored* `cfg.pseudo_reconcile`
(`config.rs:43`) into `consent_terms`/advisory/`gift_only` (`promote.rs:410-449`), so the
chokepoint-wide pseudo-off fix is well-founded; Phase-B synthesizes `SelfTransferMine{basis:None}`
for every unresolved `TransferIn` (`resolve.rs:~1156`), grounding "pseudo is not shortfall-stable";
decision sort `src_priority:u8::MAX` at `window_end.midnight()` (`resolve.rs:1310-1312`) grounds
DFW-D5.1; `promote_prior_year_advisory` diffs per-year disposal ∪ removal (donation+gift) leg sets
(`conservative.rs:664,741-758`), grounding DFW-D11's over-disposal-AND-removal export set.

---

## Critical

### C-1 (DFW-D5.3) — The over-coverage / "phantom" promote-refusal is over-constrained; on the shared promote chokepoint it refuses a shipped, BG-D6-guaranteed operation (promoting a fully-undisposed tranche)

DFW-D5.3 adds to **the promote chokepoint** a "refusal-grade check — **a target tranche that
currently covers no shortfall is refused**," with the predicate "a live `EstimatedConservative`
tranche whose **removal leaves NO `UncoveredDisposal` that it was clearing**" is a phantom. Per
DFW-D2 the promote chokepoint is the single trio shared by the CLI `promote-tranche` verb AND the
dashboard, so this refusal lands on both.

**The predicate cannot distinguish two materially different states, both of which satisfy it:**

1. **Genuine phantom (the intended target):** a tranche that WAS clearing a disposal, but a later
   real import/classify now supplies those coins. Removing it introduces no new `UncoveredDisposal`
   (the import covers the disposal) — but a double-count now exists, its `>$0` floor exits
   `hifo_cmp`'s sort-last case and is drawn first → understated gain. A real hazard. Refuse: correct.
2. **Fully-undisposed / forward promote (legitimate, shipped):** a tranche whose coins simply have
   **not been sold yet**. No disposal draws it, so removing it also introduces no new
   `UncoveredDisposal`. It "currently covers no shortfall" — and the predicate **refuses it too**.

But promoting a fully-undisposed tranche is an **explicitly shipped, tested, BG-D6-guaranteed**
operation: `promote_tranche` renders an `Unrealized{sat, hypothetical_reduction, as_of}` consent term
for undisposed sats (`promote.rs:310-322`, `conservative_promote.rs:246-258`) and records; the core
KAT `fully_undisposed_promote_records_an_unrealized_term_not_empty` (`kat_promote.rs:2117`) pins that
BG-D6 mandates the UNREALIZED line "never a bare nothing," and the CLI exercises the same
`ConsentTerm::Unrealized` path (`promote_cli.rs:360`). `promote_tranche` today has **no**
covers-a-shortfall guard (`grep` confirms). So DFW-D5.3 as written:

- **weakens a shipped guarantee** — the CLI `promote-tranche` verb would refuse a fully-undisposed
  promote *before* consent, making BG-D6's UNREALIZED disclosure path unreachable via the chokepoint;
- **contradicts §5** ("the shipped BG-D1..D11 KATs remain green — the extraction is
  behavior-preserving") and reintroduces r1's **I-2** dilemma for PROMOTE with **no** CLI-vs-dashboard
  carve (DFW-D5.2 gave declare a `target_shortfall:None` escape; DFW-D5.3 gives promote none);
- is **unsound as a predicate**: "removal leaves no `UncoveredDisposal`" is broader than the stated
  intent "(a later import/classify supplied the real coins)" — a plan-writer implementing the
  predicate breaks undisposed; one implementing the intent needs a double-count detector the SPEC
  never defines. The distinction is a binding decision that belongs in this SPEC, not the plan.

Scoping the refusal dashboard-only does NOT save it: it would still wrongly refuse a dashboard-driven
promote of an undisposed *declared* tranche (DFW-D3's per-tranche fork offers promote on any live
tranche row), and it would violate DFW-D1 (second gating authority). The predicate is wrong on both
surfaces.

**Fix (pick one, make it binding):**
- **(a) Narrow the predicate to the actual hazard:** refuse only when the tranche's sats are
  **unconsumed AND a disposal in the same pool currently draws documented (non-estimated) coins the
  promoted floor would HIFO-reorder ahead of** — i.e. an over-count actually exists. A never-drawn
  (undisposed) tranche has no such disposal → not refused. Same shadow-projection machinery; still
  derived. OR
- **(b) Demote the promote-side check to a dashboard advisory** ("this tranche no longer covers
  anything real — void it") and rely on BG-1's existing hifo/clamp machinery for the filing outcome,
  leaving the CLI/chokepoint promote gate set unchanged (behavior-preserving).
- Either way: state the CLI-vs-dashboard residency explicitly (as DFW-D5.2 does for declare), and add
  a KAT that a **fully-undisposed** tranche still promotes and records the UNREALIZED term.

---

## Minor

### m-1 (DFW-D4 / DFW-D7) — `short_sat` per-record vs per-event is under-specified; a single event can carry two sat-carrying shortfalls

A partially-covered disposal emits BOTH a principal short (`fold.rs:708`) and, since `consumed` is
non-empty, a fee short from `consume_fee` (`:388`) — two `UncoveredDisposal` on **one** `EventId`,
differing only in the `Blocker.detail` string (blockers carry no identity beyond source+kind —
`state.rs`). DFW-D7's `{event, wallet, date, short_sat}` and the §5 KAT "exactly one candidate of
`short_sat`" read as one-per-record, but the clearance target is an `EventId` (DFW-D5.2), so the only
derivable clearance predicate is **event-level** ("no `UncoveredDisposal` remains on the target
event"); and DFW-D8's "excess above the prefilled `short_sat` = out-of-scope holdings" misfires if
`short_sat` is a single leg rather than the event aggregate. A competent plan-writer resolves this by
aggregating `short_sat` per event (prefill = event total; event-level clearance), but the SPEC should
say so — one sentence pinning `short_sat` as the per-event aggregate and clearance as
event-level closes it. (Edge case for the "sales imported, purchases gone" audience, which usually
shorts fully → single record; hence Minor.)

### m-2 (§5 wording) — "behavior-preserving" does not acknowledge DFW-D6's intended pseudo-off change

DFW-D6's chokepoint-wide `pseudo_reconcile=false` is a real behavior change to the shipped
`promote_tranche` on a pseudo-active vault (today it folds the stored flag — `promote.rs:410` over
`config.rs:43`). DFW-D6/§8 correctly frame it as a **fix** to a latent sub-project-1 defect, but §5's
blanket "the shipped BG-D1..D11 KATs remain green (behavior-preserving)" reads as if nothing changes.
Add a half-clause: behavior-preserving **except** the DFW-D6 pseudo-off correction (a bug fix; the
KATs it changes are the buggy ones, replaced by the latent-gap KAT). This keeps the C-1 contradiction
visibly distinct from the DFW-D6 one, which is reconciled.

---

## Lens answers (condensed)

**L1 (DFW-D2):** Contract is complete/consistent/implementable. `apply(&mut Session, Plan,
acknowledge)` lets the single-sourced `require_promote_ack` enforce fail-closed with no driver bypass
(I-3 resolved); export trio degenerate; parity at full-driver altitude; no ack-signature gap remains.
**L2 (DFW-D4):** Keying on structural `short_sat` is total over all 15 emitters (6 carry it, 9 do
not); `pool_key(date,wallet)`+`date ≤ short-op date` is a well-defined resolve-first predicate. One
under-spec (m-1: multi-shortfall-per-event).
**L3 (DFW-D5):** Derivable with no persisted state. Didn't-cover predicate (I-4) sound. **The
over-covered/phantom promote-refusal is NOT sound — C-1** (over-constrained; refuses undisposed).
**L4 (DFW-D11):** Two-set split clean: `promoted_filing_years` (disposal-legs-only, gate/8275) vs the
strictly-larger fold-diff export set (disposal∪removal, via the `promote_prior_year_advisory`
fold-pair, across all live promotes ∪ current year). Both derived from state.
**L5/L6 (consistency/new):** The only DFW-D# contradiction is C-1 (DFW-D5.3 vs DFW-D2/§5/DFW-D8,
mirroring the r1 I-2 shape). Every other shipped BG-D1..D11 enforcement point stays put:
BG-D5/D7/D3/D6 inside the extracted sequence, BG-D8 in the untouched `promote_export_gate`, the
allocation guard reused. Phasing coherent (C-1 is a P-C decision but must be pinned in the SPEC now).

---

*End r2 (SPEC, Opus). Verdict: **NOT GREEN — 1C/0I/2m/0n**. All r1 findings resolved; the single
Critical is the DFW-D5.3 phantom-refusal, over-constrained at the shared promote chokepoint —
resolvable by narrowing the predicate (or demoting it to a dashboard advisory) without reshaping the
architecture. Re-review required after fold.*
