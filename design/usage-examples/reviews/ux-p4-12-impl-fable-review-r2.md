# Independent adversarial review — Phase 4 "Affordances" fold, round 2 (r1 findings I-1..I-4, M-1, M-2)

Reviewed against current source at `/scratch/code/bitcoin_tax` (HEAD `5fe230c`, fold diff `b101fef..HEAD`). Every fold was verified against source, not the diff or the author's summary; the underlying behaviors were independently re-derived, four were reproduced with the built binary (`target/debug/btctax`), and two mutants (I-2, I-4) were applied live to confirm the new KATs kill them (both red; source restored, tree clean, `make check` re-run green: 2056/2056).

## CRITICAL — GATES
None.

## IMPORTANT — GATES

**I-A — (I-1 fold, incomplete) The corrected `--fmv` help replaces the false fallback claim with a false remedy claim: "re-classify with `--fmv` to clear it" is a command the CLI refuses.**
`crates/btctax-cli/src/cli.rs:535-539` (and the regenerated man page): *"omitting `--fmv` records a Hard "FMV missing" blocker (re-classify with `--fmv` to clear it)."* The first claim is now accurate. But the parenthetical remedy is refuted by the engine's own first-wins rule: duplicate `ClassifyInbound` -> `DecisionConflict` (`resolve.rs:708-727`), enforced at record time by `guard_decision_conflict` (`reconcile.rs:46-61`). Repro: classify without --fmv (Hard FmvMissing), then classify again with --fmv -> `error: usage: cannot record this decision — duplicate ClassifyInbound`, exit 2, blocker NOT cleared. Working sequence: `reconcile void <decision-ref>` then re-classify. The same fold's own sibling advisory (`fold.rs:1043-1046`) states the void-first rule. Fix: "(void the classification, then re-classify with `--fmv` — classify-inbound is first-wins)"; regen man; add "void" to the help_units pin.

**I-B — (I-2 fold, new defect introduced) With two or more in-force GLOBAL standing orders, `config` names exactly one as "the vault-wide standing order" — chosen by raw `decision_seq`, not the projection's key — and silently hides the other in-force global order(s), which the engine will honor.**
`main.rs:565-576`: the vault-wide line is `orders.iter().rfind(|e| e.wallet.is_none() && e.note == "in force")` — max `decision_seq`. The engine's resolver (`resolve.rs:175-199` `resolve_election`, used by `fold::applicable_method` + compliance) resolves the global tier per date by `max_by (effective_from, decision_seq)` among orders with `effective_from <= date`. Keys disagree when an earlier-seq order has a later effective_from; multi-order vaults are reachable (`set_forward_method` appends, never voids priors). Repro: set hifo effective 2030, then lifo effective 2027, then `config` prints only `forward_method: LIFO (vault-wide standing order, effective 2027-01-01)` while `verify` lists BOTH `[in force]` and `resolve_election` gives HIFO for every disposal on/after 2030. Fix: use the projection's key / the shared resolver; when >1 in-force global order exists, disclose them; add a multi-order KAT.

## MINOR

**M-a — (I-4 residue)** `export_irs_pdf.rs:586-590` still asserts `rep.full_return_paths.len() == rep2.full_return_paths.len()` under the message "the packet is byte-for-byte the same set" — r1-I4 said compare sets instead of counts. The new process-level KAT does the genuine set comparison (substance covered), but the misleadingly-labeled count assert persists.

## NIT
**N-a —** The new help_units pin is an OR; the exact-old-text negative assert kills the r1-I1 revert, which is what matters.
**N-b —** The process-level KAT message says "byte-for-byte identical" while comparing sorted file NAMES; no `!files.is_empty()` guard (non-vacuity rests on the sibling KAT).
**N-c —** r1's N-1..N-3 remain open.

## STATUS
- I-1 — PARTIALLY RESOLVED (false daily-close gone + pinned, but replacement remedy refuted by first-wins guard = I-A).
- I-2 — PARTIALLY RESOLVED (scoped misattribution fixed + KAT'd + mutation-proven, but single-winner-by-decision_seq hides co-existing in-force global orders = I-B).
- I-3 — RESOLVED (valid grammar, verified vs clap + live binary; pinned).
- I-4 — RESOLVED (process KAT non-vacuous, matches emission, mutant dies; Minor residue M-a).
- M-1 — RESOLVED (both sites fixed + pinned; sweep clean).
- M-2 — RESOLVED (second advisory pinned both directions).

## VERDICT
**NOT GREEN — 0 Critical / 2 Important to fold:** I-A (--fmv help teaches a command the record-time guard refuses; void-first required), I-B (config hides in-force global orders beyond max-decision_seq and selects by a key the projection does not use). Both display/help-layer only — computation, verify, §1 untouched. Re-review after folding.
