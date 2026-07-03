# R0 spec review — tui-edit chunk 4b, round 2 (fold verification)

**Verdict: R0-GREEN — 0 Critical / 0 Important.** All four round-1 findings folded correctly, no drift.

- **[I1] RESOLVED** — per-disposal Δtax column + modal field removed; whole-year `OptimizeProposal.delta`
  (`optimize.rs:90`) shown once as a flow-level banner ("Expected year Δtax if the FULL proposal is
  accepted: {delta} (≤ 0)"). Honest + CLI-matching (`render.rs:1602-1637` shows per-disposal
  date·wallet·status·picks only). The right fix.
- **[M2/N1] RESOLVED** — `Session::optimize_proposal(year, now) -> Result<OptimizeProposal, CliError>`
  made the PRIMARY opener; cleaner for KAT-G1 (all optimizer plumbing — `optimize_year`, `map_opt_err`,
  `tax_date`, tables — stays in btctax-cli; TUI only types the one call). Assembly methods are all
  `&self` on the held session (`tax_profile` session.rs:95, `optimize_attested_set` :108,
  `load_events_and_project` :136) → no new Session::open, no deadlock. Profile read FRESH (N1);
  `map_opt_err` applied internally (M2). Error arms match `optimize.rs:723-750`.
- **[M1] RESOLVED** — `tax_date(now, UtcOffset::UTC)` (2-arg).
- **[M3] RESOLVED** — empty post-filter list → status + no-open (void R0-M8 discipline).

Non-blocking note: the banner delta is the full-proposal figure and can exceed what this flow realizes
when some rows are ForbiddenBroker2027/filtered — but that IS the shipped CLI's behavior; the "if the
FULL proposal is accepted" wording + the approximate caveat disclose it. Parity with green CLI =
acceptable.

Unchanged parts from round 1 still hold (pre-filter + resolve.rs:787-800; persist_optimize_accept
dual-write inverse; optimize_attest::set KAT-G1 token). **Cleared to proceed to implementation.**
