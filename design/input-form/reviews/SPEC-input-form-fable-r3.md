# SPEC review — `design/SPEC_input_form.md` (r4 fold), independent Fable pass r3

*Persisted VERBATIM (STANDARD_WORKFLOW §2). Reviewer: Fable (same independent reviewer). Persisted
2026-07-14 against HEAD `b2c5f2f`. Final confirming pass on the NI-1/NI-2 fold. Folded in spec r5.*

---

## VERDICT: 0 Critical / 0 Important

NI-1, NI-2, and M-a..d are all genuinely closed, and the `Working = Option<ReturnInputs>` change did not introduce a new correctness seam. Four residual **Minor** doc-consistency items (not gating) are listed at the end for optional cleanup.

## The two Importants — confirmed closed

**NI-1 (parked survives edits): CLOSED.** `Loaded::Draft{ ri, parked }` round-trips the flag (§5.7); `save_draft` read-modify-writes to preserve `parked` (§5.7, §6); §9 states a parked draft stays `parked=1` through edits until a re-commit consumes it. The write graph is single-sourced and closed: the *only* setter of `parked=1` is `park_to_profile`; `save_draft` preserves; re-commit and 'discard parked draft' are the only deleters. The r3 downgrade window (edit a parked return → autosave resets `parked=0` → external write deletes it) is gone.

**NI-2 (filing_status by construction): CLOSED, and clean.** `Working = Option<ReturnInputs>`: `None` until chosen; the only accepted `Edit` on `None` is `SetField{filing_status}`, which materializes `Some(ReturnInputs{filing_status, ..default})`; any other `Edit` on `None` is `ApplyError`; `commit`/`save_draft` take a materialized `&ReturnInputs`, so the impossible in-`commit` check is correctly dropped (§6.1). "Chosen ≡ RI exists" is now a type-level invariant, not a renderer bool — it satisfies the project's answered-ness-by-construction doctrine, and both renderers inherit it.

### Probe results — the NI-2 change is seam-free

**(a) Liveness / KATs / load all interact cleanly.** `live: fn(&ReturnInputs)` is only evaluable once `Some`; §9A special-cases `None` to present *only* the filing-status choice, so no `live`/`get` fn is called without an RI. Round-trip / coverage KATs are engine tests over a materialized `&ReturnInputs` via the accessors — the `Working` wrapper lives at the apply/renderer layer, not the accessor layer, so the KATs are untouched (accessor signatures were *not* changed to `Working`). `load` maps `Committed(ri)`/`Draft{ri,..}` → `Some(ri)`, `Fresh` → `None`, one-to-one.

**(b) No `Edit` can leave `Working` in a bad state.** The transition is strictly one-way: `None` → (first `SetField{filing_status}`) → `Some`, nothing returns it to `None`. `filing_status` is a frozen non-`Option` `Enum`, so `ClearField{filing_status}` is a `SetError`, and `ReturnOptions` is a `Singleton` (no `DeleteSection`). A non-`filing_status` first edit is rejected. `DeleteSection(ScheduleA)` (with the I-10 `itemize_election`→`Auto` reset) and `DeleteSection(Spouse)` operate on `Some` and yield valid — possibly screen-refusable, never *malformed* — RIs. No reachable bad state.

**(c) One stale assumption of `apply(&mut ReturnInputs)` remains — only in the §3 overview box (Minor, M-1).** Everywhere authoritative (§5.7, §6, §9A, §10) uses `Working`/"working copy" consistently.

### M-a..d — confirmed
M-a `Bool` in §5.2's list ✓. M-b Payments in the §9A order ✓. M-c `NonCryptoNoncashGift` → own row `Section(ScheduleACharitable)` — re-counted the map: **37 variants, each placed exactly once** ✓. M-d refuse message names 'use full return' / 'discard parked draft' ✓ (but see M-2).

## Residual MINOR items (not gating)

- **M-1. §3 architecture box stale vs §5.7** — shows `apply(&mut ReturnInputs, Edit)` and pre-`Working`/`sess` store signatures. §5.7 is canonical, so no implementer is misled; sync the overview.
- **M-2. M-d names a 'discard parked draft' form action with no §9A affordance** — the primary exit ('use full return') is wired, so not stranded, but the named remedy has no key. Add it to §9A.
- **M-3. §10 should pin the NI-2 guard with an explicit test** — `apply(None, non-filing_status Edit) → ApplyError`, and `None + SetField{filing_status} → Some(RI)` with that status and no other. (untested-guard memory warns exactly here.)
- **M-4 (Nit). §10 "a Fresh/unchosen `filing_status` blocks commit" now reads as an in-`commit` check** — post-NI-2 it's by construction; reword to match §6.1.

None change a computed result, lose data, or leave a gate unmet. The spec is internally consistent on every load-bearing axis (§5.7 ↔ §5.8 ↔ §6 ↔ §7 ↔ §9/§9A ↔ §10), the §7 match is exhaustively and uniquely 37/37, and the `parked` and `Working` invariants both hold by construction. This fold is 0C/0I — clean to take to an implementation plan.
