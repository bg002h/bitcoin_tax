# Approach B sub-project 1 — design provenance (brainstorm → Opus critique → Fable adjudication)

The SPEC's design of record was NOT the first draft. This records how it got there, so a future reader sees
which decisions were contested and why the final ruling went the way it did.

## Brainstorm decisions (2026-07-21, with the user)
- Decompose Approach B: **1 = floor engine + Form 8275** (first), 2 = wizard, 3 = VARIOUS. (approved)
- Floor source = **both** window-reference + filer-supplied, per-tranche choice. (approved — later REFINED by
  the adjudication to WindowLowClose-only; see below)
- Recording = a **`PromoteTranche` decision layered on the $0 tranche** (not extend `DeclareTranche`). (approved)
- Form 8275 = a **filled official IRS PDF** with auto-generated disclosure. (approved — later PHASED: content
  in 1a, the PDF in 1b)
- §6662 guardrail = **any filer-justified floor, gated** (8275 + attest + method). (approved — later REFINED)

## Opus critique (independent) — 4 blocking findings + scope split; verdict "a core decision needs rethinking"
1. **Drop §1014** — statutory basis, not an estimate; breaks provenance-neutrality; contradictory term rule.
2. **§6662 protection overstated** — "reasonable basis" is authority-keyed; honest protection is §6664(c) +
   negligence-avoidance, not a clean §6662(d) safe harbor; ≥10%/$5k threshold makes "8275 for every promote"
   over-broad; PartialRecords is substantiated (no 8275).
3. **Missing loss-clamp** — a floor on a below-window sale files a loss off an estimate (SPEC §3 forbids it).
4. **`EstimatedFloor` tag silently exits the D-8 backstop + ~7 advisory predicates** — needs a whole-surface
   `is_conservative_tranche()` sweep.
   Lesser: P6 rewrite; event-source the method; Op-construction not lot mutation; forbid `--basis` on non-Full
   coverage; auto-scaffold + filer facts; whole-tranche only; re-promote=DecisionConflict; promote-over-disposed;
   attest composition. Scope: split 1a (text disclosure MVP) / 1b (PDF).

## Fable-architect adjudication (the design decision) — verdict **BUILD-REDUCED**
Grounded in law + code; overruled parts of BOTH the initial design and Opus:
- **§1014 OUT, and go further — drop `acquired_at` entirely** (a date that moves term without moving the window
  is the incoherence G-4 forbids; substantiate a date → void + re-declare a tighter window).
- **§6662 — Opus overstated the weakness.** *Cohan* (39 F.2d 540) + *Vanicek* (85 T.C. 731): the window-min
  close is the "bearing-heavily" number and, for a **purchase-provenance** filer, a genuine reasonable-basis
  position. **Keep the 8275 mandatory** — Opus's threshold argument is WRONG (§6662(d) is measured RETURN-WIDE,
  unknowable at promote time). Fix the COPY, not the gate.
- **★ Both MISSED §6662(e)/(h)** — a 40% gross-valuation-misstatement penalty if basis is disallowed to $0
  (*Woods*, 571 U.S. 31), and adequate disclosure does NOT protect against §6662(b)(3). Must be in the risk copy.
- **Loss clamp — AGREE, it is already a parent-SPEC mandate.** `estimate-basis = min(floor_share,
  proceeds_share)`; unclaimed floor evaporates; documented components unclamped.
- **★ The key architecture ruling — NO new `BasisSource`.** A promoted tranche stays `EstimatedConservative`;
  `PromoteTranche` resolves as pass-2 Op-construction (rewrite the target's `Op::Acquire.usd_cost`). Every v1
  guarantee (D-8 backstop above all) keys on that tag → holds BY CONSTRUCTION, killing the invisibility class
  (which Fable found was ~14 sites incl. a `!=` that would invert method-inversion) instead of sweeping it.
- **★ Both MISSED: WindowLowClose itself asserts PURCHASE provenance** — fabrication for gift/income coins. →
  require a recorded purchase-provenance attestation; refuse otherwise (BG-2's load-bearing half).
- **FloorMethod = WindowLowClose ONLY** (§1014 out; PartialRecords redundant → real documented import). No
  `--basis` — the floor is COMPUTED (`Coverage::Full` required) + STORED (a later price update mustn't move a
  filed position; verify flags drift).
- **Gates:** one command + recorded consent + export-time 8275 hard-gap; clean export, no watermark (DRAFT-gate
  policy). Promote-over-disposed = ALLOW + 1040-X advisory (Opus's hard-refuse was wrong).
- **Scope split:** 1a (engine + content) / 1b (official PDF) are PHASES of ONE sub-project, NOT independently
  shippable — Opus's "plain-paper MVP" is wrong on the law (Reg §1.6662-4(f) requires a completed Form 8275).
- **Single most important thing to get right:** the promote must not mint a new identity.

(Full agent transcripts are in the session task outputs; this is the load-bearing summary.)
