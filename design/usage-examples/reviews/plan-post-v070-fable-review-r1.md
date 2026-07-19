# PLAN review r1 — post-v0.7.0 product cycle (independent Fable review)

**Artifact:** `design/usage-examples/IMPLEMENTATION_PLAN_post_v070_product_cycle.md` (commit `b87babf`)
**Spec of record:** `SPEC_post_v070_product_cycle.md` (GREEN r4). **Branch:** `feat/post-v070-product-cycle`.
**Reviewer:** Fable (independent; author ≠ reviewer). **Date:** 2026-07-18.
**Scope:** soundness/completeness/sequencing of the build order only — SPEC design decisions are settled
and were NOT re-litigated.

## Method

- Cross-walked every in-scope SPEC item (§1 list) to a PLAN step and the SPEC's KATs to that step.
- Verified the PLAN's file/function hooks against current source on the branch (~30 anchors spot-checked):
  `render_tax_outcome` `render.rs:1018` / total `:1052-1058`; `render_dual_report` `:1173` / L24 `:1229` /
  Absolute `:1247`; `cmd/tax.rs` report build `:429`, provenance `:282-296`, `write_back_carryover`
  `:444-517` (provenance gate `:478-483`, apply `:508`); `tabs/tax.rs` App-free `render` `:22`,
  `render_tax_content` `:55`, NOT-COMPUTABLE arms `:59-63/:68-70`; `session.rs` `open` `:390`, years-enum
  `:498-499`, `project()` stored-cfg `:556-562`; `reconcile.rs` `:41/62/85/110/301/1136/1162` (all exact);
  core `resolve.rs` `:513/:521-522/:543-560/:563+` (accept-governed + pseudo-gated writers as the SPEC
  states), `applied` local to `resolve()` `:402/:468`; cli `resolve.rs:121-128` placeholder inject
  (pseudo-on-gated), `Provenance::PseudoPlaceholder` `:30`; `whatif.rs` `NoLots` `:137`, raise `:235`,
  `parse_sell_arg` `:481` (sign passed through — bug confirmed live); `cmd/whatif.rs:170-172`;
  `eventref.rs:77-79` (`parse_usd_arg`, no sign guard — confirmed); `state.rs` `sigma_pending` `:260`,
  `pseudo_active` `:282`; `admin.rs:82`; `render.rs:586` `write_csv_exports`, `pseudo_tag` `:57-65`
  ([R0-I4] screen-only precedent); `tui-edit/main.rs:3742` payload-summary match; `tests/tax_report.rs:780`
  stale "exit 0" doc; `xtask/examples.rs:933` `is_demonstrated`; `no_direct_now_utc_in_production` at
  `btctax-tui/src/export.rs:970` **and** `btctax-tui-edit/src/main.rs:13975` (two copies).
- Golden-churn premises checked empirically: `grep -c pseudo docs/examples/examples.md` = 0 (SPEC KAT (a)
  clause (i) premise holds); no journey report renders NOT COMPUTABLE (so UX-P4-10's exit-1 does not churn
  the examples golden); the golden records `[exit N]` lines, so exit-code changes WOULD churn if a journey
  hit one — none does.
- `cli.rs` has no existing `events` subcommand — `events list` is collision-free/additive as claimed.

## Completeness cross-walk (all in scope → mapped)

| SPEC item | PLAN step | KATs mapped |
|---|---|---|
| UX-P4-1 (4 surfaces + gate) | 1b | (a)–(f) + mutation — full §3.1 set ✓ |
| UX-P4-3 | 1c | by-reference to §3.2 both-directions + R3-I1/R4-M3 arms named ✓ |
| UX-P4-4 + UX-P1-3 | 1d | §3.3 by reference; `--sell=-1` named ✓ (but see I-3) |
| UX-P4-5 | 4a | warn + packet-unchanged ✓ |
| UX-P4-6 | 3a | pending-line KAT ✓ |
| UX-P4-7 | 2a | formatter + TUI-no-truncate ✓ |
| UX-P4-8 | 2b | path-in-message KATs ✓ |
| UX-P4-9 | 2c | insufficient vs zero ✓ |
| UX-P4-10 | 3b | exit-code KAT ✓ (but see M-1) |
| UX-P4-11 | 1a | paste-accept + pseudo-decidable ✓ (see N-3) |
| UX-P4-12(b–i) | 4b | per output-changing sub-item; (i) decided (store-then-gate, per [G-N3] default) ✓ |
| M-1 | P5 | blast-radius enumeration + J6 regen + net-isolation/msrv ✓ |
| UX-P1-7/8/10 | P6 | golden byte-gate ✓ |
| UX-P2-1 | P6 | no named KAT (M-4) |
| UX-P3-2 | P7 | no acceptance check (N-2; SPEC is silent too) |
| N-R1 | P7 | KAT named ✓ (but see M-2 — two copies) |

Nothing in scope is dropped. Sequencing honors the SPEC's stated dependencies: 1a (`events list`) precedes
1c ([G-I8]) ✓; the shadow helper precedes 1c ✓; the write-carryover/exit-1 interaction is ordered 1b → 3b
and 3b correctly notes clause 4b already dissolved the placement question ✓. No phase-1 KAT needs a
later-phase artifact. Phase 5 (M-1 J6 regen) before Phase 6 (journeys) avoids golden conflicts ✓.

## Findings

| # | Sev | Where | Finding |
|---|---|---|---|
| I-1 | **Important** | Phase-1 preamble ("Shared prerequisite") + sequencing note | **False dependency claim: UX-P4-1's predicate does NOT consume the pseudo-OFF shadow projection.** The preamble says "Both UX-P4-1's predicate and UX-P4-3's validator need 'what the resolver sees with `pseudo_reconcile` forced OFF.'" The SPEC §3.1 predicate is `state.pseudo_active() OR provenance == PseudoPlaceholder`, both read from the **live (pseudo-ON)** projection/resolve (`tax.rs:282-296`, `state.rs:282`). A pseudo-OFF shadow view has `pseudo_synthetic_count == 0` by construction and can never yield `Provenance::PseudoPlaceholder` (`cli/resolve.rs:121-128` injects only when `pseudo_reconcile` is on) — so an implementer who wires 1b's predicate to the shadow helper gets a banner that is **structurally silent**, reintroducing exactly the [T-C1]/[T-C2] Critical channels this cycle exists to close. Step 1b's own text is correct (live state + provenance at `tax.rs:429`); the fix is one of scoping: reword the shared-prerequisite paragraph and the sequencing note to say the helper serves **UX-P4-3 only**. |
| I-2 | **Important** | Phase-1 preamble, helper KAT | **The shadow-equivalence KAT is under-specified — it can pass vacuously and would not red under the most likely wiring mutation.** "The helper's `applied` for a fixture equals the resolver's own for the same ledger with pseudo off" is trivially green on any fixture whose pseudo-ON and pseudo-OFF views coincide (any fully-real ledger) — including under the mutation the SPEC explicitly forbids: the helper reading the **stored** pseudo cfg (`session.project()`, `session.rs:556-562`). The KAT must pin a **pseudo-divergent** fixture — one where ON/OFF `applied` differ (an unresolved `ImportConflict` whose accept-first insert is `pseudo_on`-gated, `resolve.rs:521-522`, and/or a Phase-A synthetic `:949`) — and assert the pseudo-gated writes are ABSENT from the helper's view. Otherwise the plan's own §0 untested-guard rule is violated at the cycle's keystone. |
| I-3 | **Important** | Step 1d | **A SPEC-delegated decision is not made.** SPEC §3.3(a)'s ad-hoc row (`what-if`/`harvest` `--income`/`--magi`/`--carryforward-in`, `main.rs:347-353/:421-427`) ends "**Decide in the PLAN**" — the cell is a delegation, not a policy. Step 1d only says "per-flag sign guards at the sites in the spec §3.3(a) table," which never pins whether `--income`/`--magi` accept negatives. Negative AGI/MAGI is legitimate on a real return (NOL-shaped years), so an over-eager refuse here is a §1-invariant false-refuse on the planning surface. The PLAN must state the per-field policy (natural resolution consistent with the SPEC's own hints: `--carryforward-in` refuse < 0 as a loss magnitude; `--income`/`--magi` allow negatives, mirroring the tax-profile posture, e.g. `--other-net-capital-gain`) plus the KAT arms for it. Note the plan's other two delegated decisions ARE discharged (§3.3(b) strict refuse; UX-P4-12(i) store-then-gate) — this is the one miss. |
| M-1 | Minor | Step 3b | The SPEC's two **deliberate exit-0 non-triggers** ([T-M5]: dual-report absolute-refused-but-delta-computed; pseudo-active report without `--write-carryover`) have no KAT in 3b. Without them, an over-broad "exit 1 whenever refused/pseudo" implementation passes 3b's stated KAT. Add both non-trigger arms. |
| M-2 | Minor | Phase 7 | N-R1 names "the `no_direct_now_utc` scan" (singular) but there are **two copies**: `btctax-tui/src/export.rs:970` and `btctax-tui-edit/src/main.rs:13975` (the FOLLOWUPS entry says "scans"). Fix + KAT both, or the un-fixed copy keeps the sticky-skip bug. |
| M-3 | Minor | Phase-1 structure | Step-identity confusion compounding I-1: the helper is an unnumbered "Shared prerequisite," yet 1c calls it "the 1a helper" and the sequencing note says "the 1a shared shadow-projection helper" — conflating it with `events list` (which doesn't use it). Give the helper its own step ID (e.g. 1-pre) with its own commit/review slot. |
| M-4 | Minor | Phase 6 | UX-P2-1 (`is_demonstrated` hardening, `examples.rs:933`) has no named KAT. The red-able guard is cheap: a golden line whose command starts `--vault v.pgp …` must no longer satisfy a bare top-level leaf. Name it. |
| M-5 | Minor | Phase-1 preamble | The helper's "exposing the effective `applied` map" is a **core API change left unscoped**: `applied` is a local inside `pub fn resolve` (`btctax-core/src/project/resolve.rs:402/:468`), not exposed anywhere. Name the seam (extend `resolve()`'s return, or a core `resolve_with_applied`-style entry the CLI helper calls) so the implementer reuses — not duplicates — the construction, per [R3-I1]. |
| N-1 | Nit | 1b, 1b-S3 | Off-by-one anchors (spec-inherited, cosmetic): `apply_carryover_writeback` is `tax.rs:508` (plan/spec say `:507`); the years-enumeration is `session.rs:498-499` (plan/spec say `:497-498`). |
| N-2 | Nit | Phase 7 | UX-P3-2 (colorized TUI PDF) has no acceptance check at all (SPEC is silent too). Even a smoke assert (the generated roff contains color requests / the PDF's content stream contains non-black fill ops) beats nothing. |
| N-3 | Nit | Step 1a | The paste-accept KAT is weak until 1c lands: pre-1c, a *wrong-but-well-formed* listed ref would also be "ACCEPTED" (that is the UX-P4-3 bug), so a mutation that lists the wrong event's ref would not red at 1a-time. Either assert the listed ref equals the event's `canonical()` identity directly, or note the KAT re-arms under 1c. |
| N-4 | Nit | Step 1a | "Golden: … decide in review" is a legitimate step-gate question, but name the default (keep `events list` out of the golden this step; a journey demo is P6 material if ever) so the step isn't blocked on an unscoped decision. |

## Strengths (for the record)

- Hook accuracy is excellent: of ~30 anchors spot-checked, all resolve to the named constructs; the only
  drift found is two off-by-one line numbers (N-1).
- The completeness cross-walk closes: every §1 item has a step; every §3.1 KAT (a)–(f) is enumerated in 1b;
  the R3-I1/R4-M3 tricky arms are named in 1c rather than lost in the by-reference.
- Golden-churn reasoning is sound and its premises verify empirically (zero `pseudo` in `examples.md`; no
  NOT-COMPUTABLE report in any journey).
- The §0 discipline block correctly carries the full-CI-surface lesson, the per-phase burndown rule, and
  the no-auto-merge boundary.

## Verdict

**NOT GREEN — 0 Critical / 3 Important (I-1, I-2, I-3) / 5 Minor / 4 Nit.** The build order and hooks are
fundamentally sound; the three Importants are all confined to Phase 1's preamble and step 1d and are
one-paragraph folds. Re-review (r2) required after the fold.
