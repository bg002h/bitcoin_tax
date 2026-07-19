# Independent adversarial review — Phase 4 "Affordances" fold, round 3 (r2 findings I-A, I-B)

Reviewed against current source at HEAD `e78b8a2`, fold diff `5fe230c..HEAD`. Every claim re-derived from source; `make check` re-run green (2057/2057), man pages regenerated (zero drift), both folds reproduced end-to-end with the built binary, one live mutant applied (restored byte-exact, tree clean, tests re-green).

## I-A verification (--fmv help remedy)
Reproduced live: without --fmv -> Hard [FmvMissing]; bare re-classify with --fmv -> refused at record time (exit 2) via guard_decision_conflict/would_conflict first-wins (resolve.rs:714-724); `reconcile void decision|1` then re-classify -> both accepted, 0 hard blockers. Sound by construction: ClassifyInbound revocable (resolve.rs:434), voided skip precedes duplicate check, ManualFmv targets Income-only. cli.rs:536-540 correct; man drift-free; help_units pins `reconcile void` + `first-wins`.

## I-B verification (config forward-method read-back)
1. Same resolver: in_force_methods -> resolve_election, identical to fold::applicable_method (fold.rs:33-45), same None=>Hifo fall-through. Pre-existing shared helper.
2. Probe soundness: scoped elections Exchange-only at every write path (CLI reconcile.rs:1131-1159; TUI exchange_method_election_rows); resolve_election matches elections only, so SelfCustody{""} probe never matches tier-1 and cannot collide with a real wallet.
3. Clock: today=now.date() from BTCTAX_NOW; J5 config pinned 2025-01-01; golden `forward_method: FIFO (vault-wide, in force as of 2025-01-01)` engine-correct + deterministic.
4. Multi-order KAT genuine + mutation-proven live: HIFO(eff-2030,seq1)+LIFO(eff-2027,seq2) at 2031 -> HIFO by max_by(effective_from,decision_seq); resurrecting the last-recorded defect reds the KAT. Swept 2026/2028/2031 all match resolve_election.
5. HIFO default correct (fold.rs:43 unwrap_or(Hifo)); fresh-vault KAT asserts HIFO.
6. Edge sweep clean: future-dated scoped suppressed until effective; backdated engine-excluded+blocker but counted; voided excluded from resolution+count, still listed [voided] in verify.

## Standing checks
1. §1: git diff --stat touches no btctax-core/computation file; verify/tax math untouched; golden diff = config block only.
2. Mutation-honesty: I-B killed by live mutant; I-A honest by token analysis.
3. New defects: none (methods[0] panic-safe, zip alignment correct, sequential Session::open, run_config_at faithful).
4. (i) untouched.

## CRITICAL — None.
## IMPORTANT — None.

## MINOR
M-1 (pre-existing, NOT introduced by this fold) — stale "FIFO default" doc comments contradict the HIFO fall-through: resolve.rs:160, resolve.rs:205, session.rs:583. Code is HIFO (fold.rs:43, mod.rs:197); this is the doc-drift class that produced r2-I-B. Comments only, non-gating. File with an owning phase.

## NIT
N-1 — count vocabulary: config "N standing order(s) recorded" (voided excluded) vs verify "Standing orders (MethodElection): M" (voided included, labeled). Nothing hidden.
N-2 — main.rs:565 now.date() (BTCTAX_NOW offset) vs made-dates' to_offset(UTC).date(); a non-UTC BTCTAX_NOW can skew "as of" by a day. Unreachable in shipped docs/tests (all pin Z).
N-3 — "1 standing order(s)" no plural inflection (cosmetic).
N-4 — r2 N-b/N-c remain open (non-gating).

## STATUS
- I-A — RESOLVED (void-then-reclassify taught + reproduced; pinned by AND of both tokens).
- I-B — RESOLVED (engine's shared resolver; HIFO default; multi-order KAT kills last-recorded mutant; nothing hidden).

## VERDICT
GREEN — 0 Critical / 0 Important. The r2 fold is complete and correct; residue is one pre-existing doc-comment Minor (M-1) plus cosmetic Nits, none introduced by this fold, none gating.
