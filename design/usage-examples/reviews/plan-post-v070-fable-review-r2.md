# PLAN review r2 — post-v0.7.0 product cycle (independent Fable re-review)

**Artifact:** `design/usage-examples/IMPLEMENTATION_PLAN_post_v070_product_cycle.md` (r2, commit `a961f75`).
**Prior review:** `plan-post-v070-fable-review-r1.md` (0C/3I/5M/4N). **Spec of record:**
`SPEC_post_v070_product_cycle.md` (GREEN r4). **Branch:** `feat/post-v070-product-cycle`.
**Reviewer:** Fable (independent; author ≠ reviewer; same reviewer as r1). **Date:** 2026-07-18.
**Scope:** verify the r1 folds are genuine + sweep for fold-introduced defects. SPEC not re-litigated.

## Method

- Re-read r2 in full against r1 and the SPEC (§3.1/§3.2/§3.3/§3.5, §9 anchors).
- Re-verified the fold-critical source claims on the branch: cli `resolve.rs` placeholder inject is inside
  `if pseudo_reconcile` (`:121-128` — a pseudo-OFF view can never yield `PseudoPlaceholder`, confirming the
  [PLAN-I1] rationale); core `resolve()` `:402`, `applied` local `:468`, accept-first insert gated on
  `pseudo_on && !applied.contains_key` `:521-522`, Phase-A placeholder insert `:949` (both pseudo-only,
  confirming the [PLAN-I2] divergent-fixture construction is realizable); `Resolution` (`resolve.rs:201-214`)
  exposes NO `applied` and no `validate_target`-shaped entry exists (confirming the [PLAN-min] "scoped new
  core API" note is factually required, not optional); ad-hoc trio semantics from `cli.rs:435-449` doc
  comments + `cmd/whatif.rs:26-45` (`carryforward_in` → `cf_long` → `carryforward.long`, a §1212(b)
  LONG-TERM loss magnitude; `--income` = ordinary taxable income excluding crypto, the stacking base;
  `--magi` feeds the §1411 NIIT threshold, defaults to `--income`); render total `render.rs:1056-1061`
  (r2's anchor is correct — better than r1's spot-read); `apply_carryover_writeback` call `tax.rs:508`;
  years enum `session.rs:498-499`.

## Resolution table (r1 findings)

| r1 # | Sev | Status in r2 | Evidence |
|---|---|---|---|
| I-1 | Important | **PARTIAL — residual Important (r2-I1 below)** | Preamble reworded correctly `[PLAN-I1]` ("for UX-P4-3 ONLY"; "1b reads the live projected state; the helper is a 1c-only dependency"; the structural-silence rationale is stated and source-true). Step 1b computes `pseudo_contributed = state.pseudo_active() OR provenance == PseudoPlaceholder` from the LIVE build (`tax.rs:429`/`:282-296`) — correct. **But the sequencing note — the second of the two fix sites r1 I-1 explicitly named — is unchanged.** |
| I-2 | Important | **RESOLVED** | Equivalence KAT `[PLAN-I2]` now (i) pins a pseudo-DIVERGENT fixture (ON vs OFF `applied` differ via the `:521-522` accept-first / `:949` Phase-A pseudo-gated writes — both verified pseudo-only in source), (ii) asserts the helper's OFF `applied` OMITS those writes and equals the resolver's own pseudo-OFF `applied`, and (iii) mandates that the stored-cfg mutation (`session.project()`, the §3.2-forbidden path) REDS it. A wrongly-pseudo-ON wiring makes the OMITS assertion fail — non-vacuous by construction. (Nit r2-N2: the stored cfg must be pseudo-ON for (iii) to be satisfiable; implied, self-enforcing, worth one clause.) |
| I-3 | Important | **RESOLVED** | The delegated decision is made `[PLAN-I3]` and is tax-sound: `--carryforward-in` refuses < 0 — verified it is a §1212(b) long-term capital-loss *magnitude* (`cli.rs:446-449`, `cmd/whatif.rs:30/:45`), so a negative has no legal meaning; `--income`/`--magi` ALLOW negatives — negative AGI/MAGI is legitimate (NOL-shaped years; a negative MAGI is simply below every §1411 threshold), so a refuse would be the §1 false-refuse the SPEC forbids. Matches the SPEC's own hints ("loss magnitude → refuse < 0"; "follow the tax-profile posture" = per-field, allow legitimate negatives). KAT arms named for both directions (`--carryforward-in=-1` refused; `--income=-5000`/`--magi=-5000` accepted + flow through) — red-able each way. |
| M-1 | Minor | **RESOLVED** | 3b now names both deliberate exit-0 non-triggers verbatim from SPEC §3.5 [T-M5]: absolute-refused-but-delta-computed → 0; pseudo-active WITHOUT `--write-carryover` → 0 (banner is the signal). An over-broad exit-1 implementation now reds. |
| M-2 | Minor | **RESOLVED** | Phase 7 names both scan copies (tui `export.rs:970` + tui-edit `main.rs:13975`) with a KAT "in each" `[PLAN-min]`. |
| M-3 | Minor | OPEN (non-gating; now compounding — see r2-I1) | The helper is still an unnumbered "Shared prerequisite"; 1c still calls it "the 1a helper"; the sequencing note still calls it "the 1a shared shadow-projection helper". |
| M-4 | Minor | OPEN (non-gating) | Phase 6 UX-P2-1 still has no named KAT for the `is_demonstrated` hardening. |
| M-5 | Minor | **RESOLVED** | The `[PLAN-min]` NOTE scopes the core API change (expose the map, or a `validate_target(ref) -> Effect` shim). Verified sound: `Resolution` carries no `applied`, and the only widening-free alternative — re-deriving effective payloads CLI-side from `Resolution.timeline` — is a hand-rebuilt view of exactly the kind SPEC §3.2 `[R3-I1]` forbids (the timeline is post-projection `Eff`s, not the per-target `applied.get(target).unwrap_or(&raw.payload)` view the validator must mirror). No cleaner alternative exists; the note's two seams are the right menu. |
| N-1..N-4 | Nit | OPEN (non-gating) | Anchors still `:507`/`:497-498` (confirmed `:508`/`:498-499` — spec-inherited); UX-P3-2 still check-free; 1a paste-KAT rearm note absent; 1a golden default still "decide in review". |

## Findings (r2)

| # | Sev | Where | Finding |
|---|---|---|---|
| r2-I1 | **Important** | "Sequencing / dependency notes", second bullet | **I-1 is only half-folded: the sequencing note still asserts the false dependency r1 I-1 named.** It reads, unchanged from r1: "The 1a shared shadow-projection helper precedes 1b's predicate and 1c's validator." r1 I-1's fix explicitly named two sites — "reword the shared-prerequisite paragraph **and the sequencing note**" — and only the first was folded. The plan now self-contradicts on the cycle's keystone: the preamble (correctly, in bold) says the helper is a 1c-ONLY dependency and that wiring 1b to it reinstates [T-C1]/[T-C2]; the dependency section says the helper "precedes 1b's predicate," i.e. that 1b's predicate consumes it. An executor working phase-by-phase who takes the sequencing section as the build-order authority still has a sanctioned path to the structurally-silent banner. It also perpetuates the M-3 mis-name ("1a … helper") that makes the wrong reading easier. One-line fix: "The shadow-projection helper (UX-P4-3 ONLY) precedes 1c's validator; 1b has NO dependency on it — its predicate reads the live projected state (`[PLAN-I1]`)." |
| r2-N1 | Nit | Header | The fold provenance says "the two Minors (exit-0 KATs, N-R1 two scan copies)" but three r1 Minors were folded — M-5's scoped-core-API NOTE is in the preamble too. Under-counts its own fold; cosmetic. |
| r2-N2 | Nit | Preamble, `[PLAN-I2]` KAT | For the stored-cfg mutation to be RED-able, the fixture's STORED pseudo cfg must be ON (a stored-OFF fixture makes `session.project()` coincide with forced-OFF and the mutation passes). This is implied by "adds under ON" and self-enforcing via the mutation-must-red clause, but one explicit clause ("fixture's stored cfg: pseudo ON") removes the only way to write the KAT wrong. |

## Sweep notes (checked, clean)

- No fold introduced a SPEC inconsistency: 1d's trio decision matches the §3.3(a) table's own hints; 3b's
  non-triggers match §3.5 [T-M5]/[G2-8] including the write-carryover cross-ref; the [PLAN-I2] KAT enforces
  exactly §3.2's validator-mirrors-resolver mandate and its `session.project()` prohibition; Phase 7 matches
  the FOLLOWUPS "scans" plural.
- The fold-adjusted anchors are source-true: `render.rs:1056-1061` (r2 corrected r1's reading),
  `tabs/tax.rs:55-121`/`:18-23`, `main.rs:140-182`, `session.rs:390-394`, `render.rs:586-618`,
  `whatif.rs:137/:234-236`, `reconcile.rs:1162`, `cli/resolve.rs:121-128`, core `:521-522`/`:949`.
- The other two SPEC-delegated decisions remain discharged (§3.3(b) strict refuse; UX-P4-12(i)
  store-then-gate), and the [PLAN-I2]/[PLAN-I3] KATs are red-able in both directions (mutation arms named).

## Verdict

**NOT GREEN — 0 Critical / 1 Important (r2-I1) / 0 new Minor / 2 Nit; r1 M-3/M-4 + N-1..N-4 remain open
(recorded, non-gating).** r2-I1 is the unfolded half of r1 I-1 — a one-sentence fix to the sequencing note.
I-2, I-3, M-1, M-2, M-5 are genuinely folded and verified against source. Re-review (r3) required after the
fold; given the fix is a single sentence with no new surface, r3 can be a targeted delta review.
