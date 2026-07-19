# Independent spec review — SPEC_post_v070_product_cycle.md (r1)

**Reviewer:** Fable (independent — did not author the spec).
**Date:** 2026-07-18. **Branch:** `feat/post-v070-product-cycle`.
**Artifact:** `design/usage-examples/SPEC_post_v070_product_cycle.md` (DRAFT).
**Inputs read:** the spec (all sections); `FOLLOWUPS.md` "USAGE-EXAMPLES cycle" + "P4 workaround-audit"
+ "Pre-v0.7.0 product-wording cleanup" sections (the original problem statements); source at every §9
anchor (all verified against the working tree, see "Anchor verification" below); `STANDARD_WORKFLOW.md`.

**Review focus (per charter):** (1) the §1 tax-figure invariant; (2) the §3.1 pseudo predicate;
(3) §3.2/§3.3 refuse-vs-warn soundness and completeness; (4) the §3.5 exit-code contract;
(5) §3.6 `events list`; (6) KAT genuineness / hidden dependencies; (7) scope and phasing.

---

## Findings table

| ID | Severity | Location | Problem (one line) | Fix (one line) |
|----|----------|----------|--------------------|----------------|
| I-1 | **Important** | §3.1 (spec:54–59) | Predicate false-NEGATIVE: the `PseudoPlaceholder` profile channel (pseudo mode ON, zero synthetics, no stored profile) computes a tax total on a fictional $0-income Single profile with `pseudo_active() == false` → no banner, and nothing else on the delta-only path discloses it | Banner/suffix iff `pseudo_active() OR provenance == Provenance::PseudoPlaceholder`; add a placeholder-channel KAT |
| I-2 | **Important** | §3.1 (spec:49–52, 61–65) | Predicate/banner-text mismatch: `pseudo_active()` is projection-wide, so a future-year-only synthetic banners an earlier clean year while the pinned text asserts "This tax projection includes pseudo-reconciled lots" — a false statement on the primary surface; conversely a row-level per-year narrowing would be UNSOUND (pool-presence selection displacement) | Keep the vault-wide signal (optionally date-bounded ≤ year-end) but make the banner text a true vault-level statement; add cross-year + "synthetic FMV not lot" wording to the KAT |
| I-3 | **Important** | §3.1 Problem (spec:43–46) | "The one silent surface is the primary number-bearing one" is factually wrong: the TUI viewer's tax tab (`btctax-tui/src/tabs/tax.rs`) renders the same "TOTAL federal tax" with NO pseudo banner or row markers anywhere in the viewer tabs — a second silent number-bearing surface, left unowned by the spec | Extend §3.1 to the viewer tax tab, or file an explicitly-owned follow-up for it in this spec's scope table |
| I-4 | **Important** | §3.2 (spec:75–90) | The validation predicate is too narrow and under-specified: "exact-duplicate (same target, same op)" misses same-target-different-payload and cross-op second decisions (resolver adjudicates ALL as first-wins `DecisionConflict` — `resolve.rs:706/759/818`), doesn't exclude voided priors (void-then-re-decide is the documented sanctioned update path) or pseudo synthetics (a first REAL classify on a pseudo-defaulted target must not read as duplicate), and `void` validation misses non-revocable / already-voided targets (`resolve.rs:427/:436`) | Pin the predicate: refuse any append the resolver would adjudicate as a NEW `DecisionConflict`, judged against live REAL (non-voided, non-synthetic) decisions; enumerate each verb's valid-ref universe; mandate validator-mirrors-resolver (shared helper or shadow-projection) with a KAT in both directions |
| I-5 | **Important** | §3.3 (spec:102–105) | Negative-guard incompleteness: `parse_usd_arg` (`eventref.rs:77–79`, no sign guard) feeds ~10 more math-bearing flags with the same `=`-form bypass — `classify-inbound-income --fmv` (main.rs:965), `classify-inbound-gift --donor-basis/--fmv-at-gift` (:982/:988), `reclassify-outflow --amount/--fee` (:1026/:1027), `set-fmv --fmv` (:1031), `what-if --price` (:324/:400), `optimize --proceeds` (:247), `tax-profile` money fields (:852–:885) — the spec guards only `--basis`; a blanket guard would ALSO be wrong (`--other-net-capital-gain` is legitimately negative) | Add a per-flag sign-policy table to §3.3 (refuse-negative for basis/FMV/proceeds/fee/price; explicitly ALLOW legitimate negatives e.g. other-net-capital-gain, decide MAGI); include `--donor-acquired` > receive date (same impossibility class as `--acquired`) |
| I-6 | **Important** | §3.3 (spec:104–105) | The SSN-only shape for `--appraiser-tin` contradicts the field's own documented contract: `cli.rs` help says "Appraiser TIN/SSN/**EIN** … satisfies the TIN-or-PTIN requirement" and a sibling `--appraiser-ptin` exists — refusing an EIN-shaped appraiser TIN fail-closes a legitimate documented input with no override | Accept SSN-shape OR EIN-shape for `--appraiser-tin`; give `--appraiser-ptin` its own shape check (`P\d{8}`) or explicitly exclude it; keep EIN-shape-only for `--donee-ein` (ITIN passes SSN-shape) |
| I-7 | **Important** | §3.3 (spec:106–109) | The P1-3 warn threshold "FMV > 100× the lot's cost-basis-implied value" is wrong and self-contradicting: every zero/low-basis lot (the common long-held-BTC donation — 2011 basis, 100,000× appreciation) trips it, so the spec's own KAT clause "a legitimate large FMV does not [warn]" fails under its own formula; funding-lot basis is also unknowable at record time without folding lot selection | Use the FOLLOWUPS UX-P1-3 formula: warn when FMV wildly exceeds `outflow_sats/1e8 × recent dataset close` (sats are on the TransferOut event; prices in `session.prices()`); define the no-price fallback (skip the warn) |
| I-8 | **Important** | §3.2 + §7 (spec:76, 161, 207–210) | Hidden ordering dependency the phasing contradicts: phase 1 (UX-P4-3) pins the refusal hint "run 'btctax report'/'events list' to see valid refs", but `events list` ships in phase 4 (UX-P4-11) — at the phase-1 gate the product instructs users to run a nonexistent verb (and bare `report` shows NO income refs, per UX-P4-11 itself), and the phase-1 KAT pins a hint that churns again in phase 4 | Resequence UX-P4-11 into/before phase 1, or spec a phase-local hint with the phase-4 rewording called out |
| M-1 | Minor | §3.1 KAT (spec:61–65) | The KAT clause "dollar figures are byte-identical between the two only in the fields not affected by the basis change" is muddled/near-untestable (the two vaults legitimately differ) and does not implement §1's "UNCHANGED across the fix" check | Rewrite: (a) pseudo vault → banner + suffix + dollar figures identical to the PRE-fix values; (b) reconciled vault (with a real profile or mode off — see I-1 interplay) → no banner; (c) mutation reds |
| M-2 | Minor | §3.5 (spec:137–148) | The stated code map "2 = usage/uncomputable-profile" is inaccurate — `run_to_exit` (main.rs:38–45) maps EVERY `Err` to 2 (io, lock, store, refusals), so the man-page contract must say "2 = any error"; the exit-1 return placement relative to `--write-carryover` is unpinned (before it would silently skip a requested write-back); doc-comment `tax_report.rs:780` ("exit 0") goes stale | Document 1 = ran-but-NOT-COMPUTABLE, 2 = command failed (any error); place the return AFTER the write-carryover block; update the stale comment. (Verified: no in-repo consumer asserts `report` exits 0 — xtask `emit()` tolerates via `[exit N]`, committed golden has no NOT COMPUTABLE report run) |
| M-3 | Minor | §3.6 (spec:157–164) | `events list` row universe is under-specified: decided vs undecided events, whether `decision\|N` refs appear (the §3.2 remedy "void decision\|N first" needs them discoverable), ordering/stability, pseudo rows; the new subcommand also needs its own man page + docs regen (binary-docs infra) — unstated | Define the row set + columns (incl. decided-status and, if listed, decision refs), pin ordering, and add the man-page/docs-regen deliverable |
| M-4 | Minor | §3.1 decision 2 (spec:53) | "Suffix the headline total line(s)" is ambiguous: delta total only, or also the §6 dual-report ABSOLUTE 1040 total (a ReturnInputs year with pseudo rows would otherwise show an unflagged absolute total)? CLI/TUI tests asserting exact TOTAL-line text will need touching | Enumerate exactly which lines get the suffix, including the dual-report absolute block's treatment |
| M-5 | Minor | §4 M-1 (spec:183–185) | `preserve_order` is a workspace-feature-unification switch: flipping it changes `serde_json::Value` map ordering for every crate in the graph that uses it (core, cli, input-form, tui-edit, oracle-harness, update-prices), not just `income show`; the spec checks only MSRV/net-isolation | Add a sweep of Value-emitting surfaces to the KAT scope + note the J6 golden regen; note `btctax-forms` is deliberately serde_json-free (unaffected) |
| M-6 | Minor | §9.4 (spec:249–251) | Anchor imprecision: `WhatIfError::NoLots` (:131/:137), the raise site (:234–236) and `HarvestStatus::NoLots` (:516/:694) are in `btctax-core/src/whatif.rs`, while the ":170–172" CLI map is `btctax-cli/src/cmd/whatif.rs` — the spec cites both as "whatif.rs"; also `harvest` maps `InvalidTarget → NoLots` (core :534) and has no `sell_sat`, so `NoLots { available, requested }` needs an explicit story at that mapping | Qualify the crate paths; state how the harvest-side mapping populates/ignores the new fields |
| N-1 | Nit | §3.1 banner text (spec:50–52) | "Run 'btctax verify' for the [PSEUDO] rows" — the flagged ROWS are in bare `report`; `verify` carries the advisory. Also "lots" under-describes `PseudoFmv` income synthetics (counted by the same signal) | Reword: point at `report` for rows / `verify` for the advisory; "synthetic lots/values" |
| N-2 | Nit | §4 UX-P4-6 (spec:171–173) | `Pending: <N> sat` — pick the unit consistent with the holdings view's display convention (BTC elsewhere) | State the unit choice |
| N-3 | Nit | §4 UX-P4-12(i) (spec:180–182) | An unresolved either/or ("align to the CLI's store-then-gate-at-export, **or** gate the default earlier") leaves a design decision to the implementer un-reviewed | Pick one in the spec (or delegate explicitly to the PLAN with criteria) |
| N-4 | Nit | §4 UX-P4-5 (spec:169–170) | WARN-and-write-anyway sits in tension with the fail-closed posture §3.2/§3.3 argue from; defensible (the packet is a correct, complete superset) but the rationale should be stated to pre-empt the next reviewer | One sentence of rationale in §4 |

**Findings not raised** (checked and clean — recorded so the re-review can skip them): the §3.5
1-vs-2 split itself is coherent (1 = command ran, the ANSWER is a refusal — mirrors `verify`;
2 = the command failed); `session.project()` in the single verbs introduces no new lock (Session::open
already holds it; `set_donation_details` precedent projects) and O(#events) per record is fine for a
CLI; duplicate-re-decide REFUSE (not warn-and-proceed) is correct given first-wins semantics — a
warn-and-proceed would record a decision the resolver then silently discards; `events list` as a new
verb (vs restructuring `report`) is the right call for golden-churn and answered-ness reasons, and the
snapshot-CSV help pointers being superseded is handled by the spec's "help and refusal hints point to
it" clause; severity map is sound (UX-P4-4 is genuinely Important — an input-contract hole with a
Form-8283/gain blast radius; UX-P4-1 Important — the answered-ness class); scope reconciles 1:1 against
the FOLLOWUPS "RE-OWNED to post-v0.7.0" + docs/polish lists with no orphan; the Out list is right.

---

## The §1 tax-figure invariant — traced (charter item 1)

No proposed change touches the math path directly:

- **§3.1** adds `pseudo_active: bool` to `TaxYearReport` (built at `tax.rs:429` AFTER
  `compute_tax_year`) and threads it into `render_tax_outcome` (`render.rs:1018`) — render-only.
- **§3.5** returns `ExitCode::from(1)` after printing (`main.rs:140–182`) — post-render.
- **§3.2/§3.3** refuse APPENDS — they can never alter the projection of an existing ledger.
- **§3.6/§4/§5/§6** are additive verbs, messages, display, docs, and test tooling.

Two **invariant-adjacent hazards** exist, both in the refusal machinery, both covered by findings:

1. A record-time validator that FALSE-REFUSES a legitimate decision (I-4's drift risk, I-6's
   EIN-shaped appraiser TIN, I-5's blanket-negative risk on `--other-net-capital-gain`) does not change
   a computed figure for a given ledger — it prevents the CORRECT ledger from being constructed at all,
   which is the same harm delivered differently. This is why I-4/I-5/I-6 block: fail-closed is only
   safe when the closed set is provably the invalid set.
2. §3.3's warn threshold (I-7) is non-fatal, so no invariant breach — but the KAT as written would be
   red under the spec's own formula, i.e. the spec fails its own gate.

Conclusion: the invariant holds as designed **provided** I-4/I-5/I-6/I-7 are folded.

## The §3.1 predicate — full analysis (charter item 2)

`pseudo_active()` = `pseudo_synthetic_count > 0` = "any synthetic decision contributes to this
PROJECTION" (`fold.rs:391–393`; the count is `res.pseudo_decisions.len()`, vault-wide, not per-year).

- **False negative (the dangerous direction), lot channel: none.** A `.pseudo` leg/income row can only
  arise from a synthetic decision, which requires pseudo mode at fold time, which makes the count > 0.
  Verified via `state.rs:131/164/199/231` marker plumbing.
- **False negative, PROFILE channel: real (I-1).** `resolve.rs:121–127` injects the
  `PseudoPlaceholder` all-$0 Single profile whenever `cfg.pseudo_reconcile` is on and nothing is
  stored — independent of the synthetic count. With every classification properly resolved
  (`count == 0`) and no stored profile, `report --tax-year` prints an authoritative total computed on
  a deliberately-fictional profile; `provenance_label` prints ONLY on the full-return/dual paths
  (`render.rs:1186`, `tax.rs:318` — the latter gated `provenance == ReturnInputs`), and even the
  `PseudoReconcileActive` verify advisory is count-gated (`fold.rs:396`) — so the spec's fix leaves a
  silently-authoritative pseudo number standing. The signal to close it is already in scope at the
  §9.1 hook site (`resolve_and_screen` returns `provenance` at `tax.rs:282–296`).
- **False positive: real but must be resolved by WORDING, not row-level narrowing (I-2).** The
  tempting per-year predicate ("any `.pseudo` row among the year's legs/removals/income") is UNSOUND:
  a pseudo lot merely PRESENT in the pool at-or-before year-end displaces method selection
  (HIFO/FIFO pick different real lots than the properly-reconciled counterfactual would), so a year
  can be pseudo-INFLUENCED with zero pseudo-flagged rows. The vault-wide signal is therefore the
  right conservative choice — but then the banner must say something TRUE ("synthetics are present in
  this vault; this year's figures may be influenced"), not "this projection includes pseudo lots".
  The only strictly-safe narrowing is excluding synthetics dated strictly AFTER year-end, if wanted.
- **Banner + suffix (§8 q1): both, yes.** The banner is the human surface; the suffix survives a
  single-line scrape. Keep both; resolve M-4's line enumeration.

## §8 open questions — reviewer's answers (charter items 3–5)

- **3.1** — both banner and suffix; the predicate must be `pseudo_active() OR PseudoPlaceholder
  provenance`, with truthful vault-level wording (I-1, I-2).
- **3.2/3.3** — fail-closed REFUSE is correct for every listed case *including* duplicate-re-decide
  (first-wins semantics make warn-and-proceed a silent no-op recorder — worse than the current bug),
  PROVIDED the duplicate predicate is live-real-decision-scoped (I-4) so void-then-re-decide and
  pseudo-default correction keep working. The shape checks must match the fields' documented contracts
  (I-6); the negative guard must be a per-flag audited table, not a single-flag patch (I-5).
- **3.5** — exit 1 (not a distinct code) is right and mirrors `verify`; no in-repo script/test/CI
  asserts `report` exits 0 (verified: `xtask` `emit()` records `[exit N]` and both journey
  `report --tax-year 2025` runs are Computed in the committed golden; the only "exit 0" reference is a
  stale library-test doc comment, `tax_report.rs:780`). Fix the "2 = usage" mislabel (M-2).
- **3.6** — `events list` alone is right for now; do NOT add per-row refs to `report` in this cycle
  (golden churn + answered-ness of the format, as the spec argues). But resequence it (I-8) and pin
  its row universe (M-3).

## Anchor verification (§9) — charter item on feasibility

All §9 anchors were checked against the working tree and are accurate, with one imprecision (M-6):
`state.rs:277/282` (count + `pseudo_active`), `fold.rs:391–407` (count + advisory), `render.rs:62`
(`pseudo_tag`), `render.rs:1018/1027/1056–1061` (`render_tax_outcome`), `tax.rs:264/282–296/429`
(report build; `state` and `provenance` both in scope at the hook), `main.rs:38–45/112–118/140–182/933`
(exit paths), `tax.rs:290` resolver-uncomputable → `Err` → 2, `reconcile.rs:28/41/62/85/110/1136`
(append-without-project verbs) vs `reconcile.rs:1162–1188` (`set_donation_details` projects —
precedent confirmed), `eventref.rs:77–79` (no sign guard), `main.rs:134–139` (negative-reject
precedent), `main.rs:1095–1106` + `cli.rs` donation args (EIN/TIN built verbatim; NOTE the help text
that grounds I-6), core `whatif.rs:131/137/234–236/516/694` + cli `cmd/whatif.rs:170–172` (M-6),
`lib.rs:44–45` / `store/lib.rs:19–20` / `session.rs:390–394` / `vault.rs:117/129` (io context sites).
Feasibility of every §3 hook is confirmed.

---

## Verdict

**0 Critical / 8 Important / 6 Minor / 4 Nit — NOT GREEN.**

The spec is architecturally sound — the invariant discipline, the refuse-posture, the additive-verb
choice, and the phased burndown are all right — but it is not yet the contract it needs to be: the
§3.1 predicate misses one pseudo channel and over-claims on another, §3.2/§3.3's fail-closed sets are
not yet provably the invalid sets, and one phase-ordering contradiction would make phase 1 gate on a
false message. Fold I-1 through I-8 (M/N at author's discretion per workflow), then re-review.
