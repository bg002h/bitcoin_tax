# WHOLE-BRANCH FINAL REVIEW — rounds 3 & 4 (Opus) — persisted verbatim (verdicts + findings)

## ARCH r3 (base eb840ed → 3f8dd9a) — **GREEN — 0C / 0I / 1 Minor / 4 Nit** — "Merge and publish."
All four r2 recommendations closed at source. Verified the public-API narrowing CALL SITE BY CALL SITE (20
`ALL_PRESETS` sites, 9 `ProvenanceKind::ALL`; three idioms — `.iter().enumerate()`, `.get(idx)`/`[i]`, `.len()` —
all valid on a slice); both drift guards (clap `value_variants()`, census) still assert what they claim; ran the
refuting `kat_tranche.rs:331` itself. `next_preset` deleted with its KATs (tombstone records the false
justification, which is what stops it coming back). `flow.clearance` doc fixed.
- Verified the `live_declare_ids` liveness fix is **monotone**: `promoted ⟹ in force` (the resolver refuses a
  promote whose target is not a live `DeclareTranche`), so the union can only add years — the safe direction.
- **WITHDREW its own r2 Nit-N2 (`#[non_exhaustive]`)** after finding it would force `_ =>` arms in
  `btctax-tui-edit` and destroy the exhaustiveness check that guarantees the dashboard renders every advisory
  variant. "That check is worth more than the additive-variant freedom."
- **CONFIRMED the controller's defer of r2 M-6** (two public `render_consent`s) with a stronger argument: the
  narrow one shipped at v0.9.0 (removing it is the breaking change); the crate root has NO collision
  (`btctax_cli::render_consent` resolves unambiguously to the correct advisory-carrying one); and it has NO
  production caller — only four test assertions. Not reachable by any ergonomic path.
- **★ PUBLISH PREREQUISITE (not a branch defect):** all 10 publishable crates sit at `version = "0.9.0"` AND pin
  siblings by version alongside path. The bump must move BOTH in lockstep — a stale `version = "0.9.0"`
  requirement resolves against the ALREADY-PUBLISHED 0.9.0 rather than the new local code. Silent; visible only
  downstream. (`xtask` + `btctax-oracle-harness` are correctly `publish = false`.)
- New Minor M-1: `era.rs`'s MODULE doc still said a lot "lands in exactly ONE pooling era" — docs.rs surface, in
  tension with the corrected text below it. Nits: duplicated assertion block; "permanently forfeits" overstated;
  lifetime-spelling inconsistency; SPEC accreting changelog.

## TAX r3 (same base) — **NOT GREEN — 0C / 1 Important / 3 Minor / 3 Nit**
Both r2 Importants RESOLVED (the false pooling claim retracted at all four sites — a whole-repo grep found no
un-retracted instance; the README "never exceed documented basis" clause gone, replacement verified against
`clamped_leg_basis`). Both code fixes correct and mutation-pinned.
- **I-1 (BLOCKER):** the fix for r2 M-3(4) introduced a NEW false sentence — *"The full consent text — every
  advisory and every term — is recorded verbatim with the decision"*. FALSE: `Acknowledgment` stores
  `shown_terms` (figure terms only) + phrase + provenance; `advisory_lines` and `post_consent_note` are SHOWN at
  consent but have no field on `PromoteTranche`. Same class as r2 I-2, on the one artifact the promote gate
  exists to build. Supplied exact replacement wording.
- Minors: README displacement overclaim (`WouldDisplaceIfPromoted` fires only at `covered_sat == 0`);
  "permanently forfeits"/"only way" overstated vs `!voided` + the shipped `TRANCHE_IS_FINAL_HINT`; partial
  FOLLOWUPS re-ownership sweep.

## TAX r4 (base 3f8dd9a → 6bad981, the doc fold) — **GREEN — 0C / 0I / 2 Minor / 4 Nit** — "Merge, and publish."
- **I-1 RESOLVED and the third version is EXACTLY true** — verified clause by clause against
  `chokepoint/mod.rs:413-416`, `event.rs:363`, `require_promote_ack:302-313`, `wide_window_note:203-215`.
  ★ Key: the new sentence asserts only `shown ⊆ recorded`, which the open no-scrolling defect CANNOT falsify,
  and does NOT re-assert the r2-deleted `recorded == screen`. Correct direction for a §6664(c) claim.
- M-1/M-2 resolved **with substance, not softening**; "the only ONE-KEYSTROKE way" verified true against the
  sharpest counter-example (the alternative route costs two keys minimum, since nudges are inert before a pick).
- M-3 resolved: swept all 16 `[open]` items — nothing points at a phase closing at this gate.
- **The inverted nit (iv) was inverted CORRECTLY:** `&'static [T]` on a free-standing const trips
  `clippy::redundant_static_lifetimes` (a hard failure under `-D warnings`); the associated const at
  `tax/questions.rs:56` escapes it only because the lint's scope is free-standing items.
- New Minors (post-merge): the straddle invariant's stated CONSEQUENCE is a non-sequitur (pool assignment is a
  total function of `window_end` alone, so a straddling window would be determinate, not ambiguous) — the real,
  tax-relevant reason is that a straddling window would let a filer attest a pre-2025 acquisition while
  `pre2025_tranche_exists` reads only `window_end`, silently un-blocking a Rev. Proc. 2024-28 allocation; and two
  test doc comments still carry the pre-fold phrasing (the repo's recurring whole-surface-sweep miss).
- Nits: "~1,461 presses" ignores that `nudge_window_start` clamps at `window_end`; `Named` is tranche-scoped not
  year-scoped; "re-derived" can read as a reconstructability guarantee; `SPEC.md:70`/`DESIGN.md:46` restate a
  scope claim that was struck from the README (internal docs only).
