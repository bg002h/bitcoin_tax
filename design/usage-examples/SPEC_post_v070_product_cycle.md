# SPEC — post-v0.7.0 product cycle (usage-examples bug-hunt burndown)

**Status:** DRAFT (pre-review). Independent Fable review to 0C/0I required before the PLAN.
**Branch:** `feat/post-v070-product-cycle`.
**Design of record for:** the open UX-P4 / UX-P1 / UX-P2 / UX-P3 / M-1 follow-ups filed during the
usage-examples journeys cycle (see `FOLLOWUPS.md` → "USAGE-EXAMPLES cycle" + "P4 workaround-audit").

## 0. Why this cycle exists

Authoring the six worked-example journeys (J1–J6) was the bug-hunt half of the usage-examples project.
It surfaced ~30 findings. 12 were resolved in the P0 build or the pre-v0.7.0 wording cleanup; UX-P3-1 was
discharged by the P3 clock seam (ledger reconciled `8e14066`). This spec covers the **remaining ~17**, all
of which were `§3.1`-fence-barred from the docs cycle because they change **behavior or messages** — the
fence that deferred them is exactly the "first post-v0.7.0 product cycle" this document opens.

**The fence is lifted.** During the docs cycle we could not touch product messages/engine; this cycle's
entire purpose is to make those changes, reviewed, with goldens regenerated.

## 1. Scope

In: UX-P4-1, UX-P4-3, UX-P4-4, UX-P4-5, UX-P4-6, UX-P4-7, UX-P4-8, UX-P4-9, UX-P4-10, UX-P4-11,
UX-P4-12(b–i), UX-P1-3, UX-P1-7, UX-P1-8, UX-P1-10, UX-P2-1, UX-P3-2, N-R1, M-1.

Out: anything requiring new tax law/schedules; MFS; the mixed-use-mortgage input (P8-owned, separate);
the "first real return" schema-migration retirement (release-gate-owned, gated on real users).

**Non-negotiable invariant:** no change may alter a *computed tax figure* for a correctly-specified
return. Every item here is a message, an affordance, an input-guard, an exit code, or a display — never the
math. Any KAT that pins a dollar amount must show it UNCHANGED across the fix.

## 2. Severity map (what gates)

- **Important (block):** UX-P4-1 (silent authoritative pseudo number — answered-ness class);
  UX-P4-4 (negative basis flows into gain math — an input-contract hole with a wrong-number blast radius,
  even though the engine math is correct *given* the input).
- **Minor:** UX-P4-3, P4-5, P4-6, P4-7, P4-8, P4-9, P4-11, P1-3, P1-7, P1-8, P1-10, P2-1, M-1.
- **Nit:** UX-P4-10, P4-12(b–i), P3-2, N-R1.

## 3. Design decisions (items with a genuine choice — Fable review targets these)

### 3.1 UX-P4-1 — pseudo flag on `report --tax-year` (Important)

**Problem.** With `reconcile pseudo on` and a synthetic default contributing, `report --tax-year` prints a
clean, authoritative "TOTAL federal tax …" with no `[PSEUDO]` marker/banner, though the gain rides on a
deliberately-fictional $0-basis/LT-default lot. Bare `report` flags the rows `[PSEUDO]`; `verify` discloses
`[PseudoReconcileActive]`; export is attest-gated. The one silent surface is the primary number-bearing one.

**Decision (proposed — pending recon area 1d):**
1. Emit an **unconditional banner** at the top of `report --tax-year` output whenever the projection is
   pseudo-contributed:
   `⚠ [PSEUDO] This tax projection includes pseudo-reconciled (deliberately-synthetic) lots — it is an`
   `ESTIMATE, not filing-ready. Run 'btctax verify' for the [PSEUDO] rows; resolve them before filing.`
2. **Suffix** the headline total line(s) with ` [PSEUDO]` so a scraped single line still carries the flag.
3. **Predicate — "pseudo-contributed":** `LedgerState.pseudo_active()` (`state.rs:282` → `pseudo_synthetic_count > 0`)
   — the SAME signal that raises the `PseudoReconcileActive` verify advisory (`fold.rs:396`) and backs the
   per-row `.pseudo` markers. RECON RESOLVED (§9.1): the signal is in scope at `tax.rs:429` where
   `TaxYearReport` is built but currently dropped. Fix: add `pseudo_active: bool` to `TaxYearReport`
   (`tax.rs:429`, from `state.pseudo_active()`), thread it into `render_tax_outcome` (`render.rs:1018`,
   which today takes no state signal). Reuse — do NOT re-derive.

**Acceptance KAT:** a vault with `pseudo on` + a synthetic-lot-consuming sale → `report --tax-year` stdout
contains the banner AND a `[PSEUDO]`-suffixed total; the identical vault with the lot properly reconciled
(non-pseudo) → NO banner, NO suffix, and the **dollar figures are byte-identical** between the two only in
the fields not affected by the basis change (i.e., the guard proves the flag toggles on pseudo-ness, not on
the numbers). Mutation check: removing the banner emit reds the KAT.

### 3.2 UX-P4-3 — record-time ref / type / duplicate validation (Minor)

**Problem.** classify/reclassify accept a typo'd ref, wrong-type ref, or duplicate re-decide with
`Recorded decision …` (exit 0); the error surfaces only on the next `verify` as a `DecisionConflict` hard
blocker. `void decision|99` (nonexistent) "succeeds" then hard-blocks as "void targets unknown event",
cleared only by voiding the void. Hints are inconsistent. `set-donation-details` already validates at record
time (feasibility proof).

**Decision:** validate at **record time**, fail-closed (**refuse**, nonzero exit, record nothing):
- unknown target ref → refuse: `no event 'X' — run 'btctax report'/'events list' to see valid refs`;
- wrong-type target (e.g., reclassify-income on an outflow) → refuse naming the actual kind;
- exact-duplicate re-decide (same target, same op) → refuse: `already decided — void decision|N first`;
- `void <nonexistent>` → refuse (do not create a self-blocking void).
Unify every `DecisionConflict`-adjacent remedy hint to the same phrasing.
Rationale: conservative fail-closed matches the app's posture; the precedent already exists; the cost today
is a false success + a void round-trip. RECON RESOLVED (§9.3): the precedent is `set_donation_details`
(`reconcile.rs:1162-1188`, project→find ref in `state.removals`→type-check→write); the single verbs
(`classify_inbound` `reconcile.rs:41`, `reclassify_outflow` `:62`, `set_fmv` `:85`, `reclassify_income`
`:1136`, `void` `:110`) parse the ref but never `session.project()` before `append_and_save` (`:28`), so a
project-then-validate must be ADDED to each (or a shared helper). `void` currently appends regardless.

**Acceptance KAT:** each bad input above exits nonzero with the specified message and leaves the decision
log unchanged (assert `verify` is clean afterward — no new DecisionConflict); a *valid* decision still
records (exit 0).

### 3.3 UX-P4-4 + UX-P1-3 — value validation at record time (Important / Minor)

**Problem.** (a) NEGATIVE basis accepted — CLI `--basis=-5000.00` (the `=` form bypasses clap's
`-`-prefix guard) and the TUI form — and flows into gain math (`basis -5000 → gain > proceeds`). (b)
`--acquired` AFTER the receive date accepted (impossible for a self-transfer-in; hides the lot from
what-if before that date). (c) `set-donation-details --donee-ein banana --appraiser-tin fruit` saved →
lands on Form 8283. (P1-3) `reclassify-outflow --as-kind donate --amount` has no doc comment + ambiguous
unit; passing sats (`200000000`) yields a **$100,002,000** §170(e) deduction silently.

**Decision:**
- **Refuse** (record-time, fail-closed): negative USD basis (both CLI `=` and space forms, and the TUI
  form); `--acquired` strictly after the receive date for a self-transfer-in; a `--donee-ein` /
  `--appraiser-tin` that is not EIN/TIN-shaped (reuse the existing shape validator if one exists — recon
  area 3d — else add `\d{2}-\d{7}` EIN / `\d{3}-\d{2}-\d{4}` SSN-shape checks).
- **Document + WARN** (not refuse) for P1-3: add a `--amount` doc comment naming the unit (USD FMV); WARN
  (stderr, non-fatal) when the FMV wildly exceeds `sats/1e8 × recent-close` (a legitimately large FMV must
  still be allowed). A guard threshold: FMV > 100× the lot's cost-basis-implied value ⇒ warn. (Refuse would
  be wrong — high-appreciation gifts are real.)

**Acceptance KAT:** negative basis refused on BOTH surfaces (CLI `=` form included) with no lot written;
acquired>receipt refused; `--donee-ein banana` refused; a sats-as-USD `--amount` emits the warning but a
legitimate large FMV does not. Mutation: dropping any guard reds its KAT. **Dollar-figure invariant:** an
existing valid donation KAT's deduction is unchanged.

### 3.4 UX-P4-9 — insufficient-balance message (Minor)

**Problem.** `what-if sell --sell 0.6` with 0.5 BTC held → `no lots available to sell from that wallet as
of that date` — "no" is false; the available balance isn't shown; genuine-zero and insufficient collapse.

**Decision:** distinguish the two and show the number:
- zero available → `no BTC available in <wallet> as of <date>`;
- insufficient → `only <X> BTC available in <wallet> as of <date> (requested <Y>)`.
RECON RESOLVED (§9.4): both `lots.iter().map(|l| l.remaining_sat).sum()` and `req.sell_sat` are in scope at
the raise site (`whatif.rs:234-236`) but discarded into the data-less `WhatIfError::NoLots` (`whatif.rs:137`).
Fix: carry them — `NoLots { available: Sat, requested: Sat }` — and render at `whatif.rs:170-172`. (Mirror
the sibling `HarvestStatus::NoLots` at `whatif.rs:694` if cheap; not required.)

**Acceptance KAT:** 0.5 held, sell 0.6 → the "only 0.5 … (requested 0.6)" message; 0 held → the "no BTC"
message. Distinct strings.

### 3.5 UX-P4-10 — `report` exit-code contract (Nit)

**Problem.** `report --tax-year` exits 0 on NOT COMPUTABLE — loud in text, invisible to scripts; `verify`
exits 1 on hard blockers, `report` never does.

**Decision:** `report --tax-year` returns **exit 1** when the requested tax computation is NOT COMPUTABLE
(mirrors `verify`'s hard-blocker convention); exit 0 for a rendered report (advisories notwithstanding).
Document the contract in the `btctax-report` man page. No-users ⇒ no back-compat concern. RECON RESOLVED
(§9.2): `TaxOutcome::NotComputable(_)` renders at `render.rs:1027` and falls through to the terminal
`Ok(ExitCode::SUCCESS)` (`main.rs:933`) = exit 0; a resolver-level uncomputable-PROFILE already `Err`s to
exit 2 (`tax.rs:290`, `main.rs:42`). Fix: in the Report arm (`main.rs:140-182`), after printing,
`if matches!(outcome, TaxOutcome::NotComputable(_)) { return Ok(ExitCode::from(1)); }`. NOTE the resulting
code map — 0 = rendered, 1 = NOT COMPUTABLE (this fix), 2 = usage/uncomputable-profile — flag for the
reviewer: is 1-vs-2 collision a concern, or is "1 = computed-but-blocked, 2 = bad-invocation" coherent?

**Acceptance KAT:** a vault whose tax year is NOT COMPUTABLE → `report --tax-year` exits 1; a computable
year → exits 0. (Integration test asserting the process exit code.)

### 3.6 UX-P4-11 — event-ref discoverability (Minor)

**Problem.** No `list`-refs verb; discovery is via export-snapshot CSV columns or stripping the `#0`
split-suffix from report lot ids; the Income section prints no refs (J4 refs embed a ms-timestamp a user
can't hand-construct). Repro trap: pasting the displayed lot id `…#0#0` into reclassify-income records a
decision that hard-blocks as "targets unknown event" (compounds UX-P4-3).

**Decision:** add a first-class **`btctax events list`** verb (additive, low golden-churn) that prints every
decidable event ref + kind + date + amount in a stable columnar form; and add one line wherever a lot id is
shown: `lot id = event ref + #split`. Do **not** restructure the existing `report` rows (avoids churning the
report golden and the answered-ness of the report format). `events list` becomes the sanctioned discovery
path the other verbs' `--help` and refusal hints point to (ties to UX-P4-3's refuse-message).

**Acceptance KAT:** `events list` prints a line per decidable event with a ref that, pasted verbatim into
`reclassify-*`, is ACCEPTED (round-trips — closes the UX-P4-11 trap end-to-end).

## 4. Mechanical fixes (TDD only — no design choice)

- **UX-P4-5** — full-return `export-irs-pdf --forms f8949` silently writes the whole packet. Fix: WARN on
  stderr that `--forms` is ignored on a full-return year (honoring a slice of a coordinated 14-form packet
  is unsound — the packet is computed as a whole). KAT: the warning is emitted; the packet still writes.
- **UX-P4-6** — bare `report` with the whole balance in a pending transfer prints all-`none`. Fix: add a
  `Pending: <N> sat (<M> unreconciled transfer(s) — see 'btctax verify')` line to the holdings view when
  pending > 0. KAT: fully-pending vault shows the line; a reconciled vault does not.
- **UX-P4-7** — decision-payload Debug dumps (`SelfTransferMine { basis: Some(19000.00), … }`) in bulk-void
  previews (CLI + TUI). Fix: a human summary formatter (`basis $19,000.00, acquired 2026-01-01`); one
  formatter shared by both surfaces. KAT: the formatter output; the TUI no longer truncates mid-field.
- **UX-P4-8** — bare `io: … (os error N)` at vault-open and `--out` collision. Fix: attach the path + a
  one-clause hint (precedent: `import` names the file). KAT: missing vault → message names the path +
  suggests `init`/`--vault`; `--out` collision → names the path.
- **UX-P4-12(b–i)** — see FOLLOWUPS itemization. Batch of message/affordance papercuts; TDD where behavior
  changes (esp. (i) the TUI default-year gate placement — align to the CLI's store-then-gate-at-export, or
  gate the default earlier with a clear message). KAT per sub-item that changes output.
- **M-1** — `income show` field order alphabetized by `serde_json::Value`. Fix: enable `serde_json`
  `preserve_order` (verify the `indexmap` transitive dep passes net-isolation + MSRV 1.88); KAT asserts the
  declared field order. Low priority.

## 5. Docs items (new worked-example journeys)

- **UX-P1-7** — a journey demonstrating manual `reconcile classify-inbound-income <ref> --kind --fmv`.
- **UX-P1-8** — a two-exchange `reconcile match-self-transfers` journey.
- **UX-P1-10** — a genuine per-disposal `reconcile select-lots` identification journey.
- **UX-P2-1** — harden the SOFT `is_demonstrated` subsequence matcher: require `path[0]` to be the first
  non-`-`-prefixed subcommand token (skip the `--vault v.pgp` global) so a longer path can't spuriously
  satisfy a bare-token leaf. (Test-tooling; do alongside the new journeys since they add coverage.)
Each new journey extends `xtask/src/examples.rs`, regenerates `docs/examples/examples.md` + the PDF, and is
byte-gated by the existing `examples_golden_matches_committed`.

## 6. Polish (lowest priority)

- **UX-P3-2** — colorized TUI PDF: drive roff color escapes from the `.txt` goldens' style runs.
- **N-R1** — the `no_direct_now_utc_in_production` scans set `in_test` STICKILY; harden to scan only the
  test module's brace span (or reset at module close). KAT: a synthetic production `now_utc()` placed AFTER
  a test module is caught.

## 7. Phasing (feeds the PLAN)

1. **Correctness cluster** (gates first): UX-P4-1, UX-P4-4/UX-P1-3, UX-P4-3.
2. **Legibility:** UX-P4-7, UX-P4-8, UX-P4-9.
3. **Report surfaces:** UX-P4-6, UX-P4-10.
4. **Affordances:** UX-P4-5, UX-P4-11, UX-P4-12(b–i).
5. **Display:** M-1.
6. **Docs:** UX-P1-7/8/10, UX-P2-1.
7. **Polish:** UX-P3-2, N-R1.
8. **Close:** whole-branch review, full CI-surface validation, regen all goldens, FOLLOWUPS burndown, push.

Each phase: TDD (guard reds without the fix), independent Fable review to 0C/0I, goldens regenerated,
commit, push. Per-phase burndown by ownership (no batching a phase-owned item past its gate).

## 8. Open questions for the Fable reviewer

- 3.1: is banner + suffix the right loudness, or is one sufficient? Is "pseudo-contributed" the correct
  predicate (vs "pseudo mode is on at all")?
- 3.2/3.3: refuse vs warn — is fail-closed correct for *every* listed case, or should duplicate-re-decide
  warn-and-proceed?
- 3.5: exit 1 vs a distinct code; does any in-repo script contract assume `report` is always 0?
- 3.6: `events list` only, or also a compact ref hint in `report`?

## 9. Verified recon anchors (2026-07-18; current source, not stale)

**§9.1 UX-P4-1 pseudo signal.** Row marker helper `pseudo_tag()` `render.rs:62`; per-row `.pseudo` on
`Lot`/`DisposalLeg`/`RemovalLeg`/`IncomeRecord` (`state.rs:131/164/199/231`); state-level
`LedgerState.pseudo_synthetic_count` `state.rs:277` + `.pseudo_active()` `state.rs:282`; advisory raised in
`fold.rs:396-407`. The SILENT surface: `render_tax_outcome` `render.rs:1018` ("TOTAL federal tax" line
`render.rs:1056-1061`), called from `main.rs:154`, signature takes no state signal. Report built by
`report_tax_year` `tax.rs:264`; `TaxYearReport` returned `tax.rs:429` with `state` in scope but pseudo flag
dropped. Hook: add `pseudo_active` to `TaxYearReport` + thread into `render_tax_outcome`.

**§9.2 UX-P4-10 exit code.** `verify` returns `ExitCode::from(1)` on hard blockers `main.rs:112-118`; Report
arm `main.rs:140-182` never sets a code → terminal `Ok(ExitCode::SUCCESS)` `main.rs:933`. `NotComputable`
render `render.rs:1027-1029`. Uncomputable-profile `Err` → exit 2 (`tax.rs:290-292` + `main.rs:42-44`).

**§9.3 UX-P4-3/4 validation.** Precedent `set_donation_details` `reconcile.rs:1162-1188` (projects, checks
`state.removals` for ref + `RemovalKind::Donation` type). Single verbs: `classify_inbound` `reconcile.rs:41`,
`reclassify_outflow` `:62`, `set_fmv` `:85`, `reclassify_income` `:1136`, `void` `:110`; common append
`append_and_save` `:28`. `--basis` → `parse_usd_arg` `eventref.rs:77-79` (NO sign guard); negative-reject
precedent `main.rs:135` (prior-taxable-gifts inline). Self-transfer CLI dispatch `main.rs:997-1003`.
EIN/TIN built verbatim `main.rs:1095-1106` from `Option<String>` (`cli.rs:646/655`); NO shape check.

**§9.4 UX-P4-9 balance.** `whatif::sell` insufficiency check `whatif.rs:234-236`; data-less
`WhatIfError::NoLots` `whatif.rs:137`; CLI map `whatif.rs:170-172`; sibling `HarvestStatus::NoLots`
`whatif.rs:694`.

**§9.5 UX-P4-8 io context.** Bare wrappers: `CliError::Io` `cli/lib.rs:44-45`, `StoreError::Io`
`store/lib.rs:19-20`. Vault-open `session.rs:390-394` → `Vault::open` `vault.rs:117` (path-aware special-case
only `HalfCreatedVault` `vault.rs:129`). Export-out: SQLite `admin.rs:82` → `export_snapshot` `vault.rs:263-271`;
CSVs `admin.rs:113` → `write_csv_exports` `render.rs:586` (`render.rs:593/595/605-618`). Precedent that names
the file: `btctax-adapters` `AdapterError::Io { path, source }` `adapters/lib.rs:23-28`, populated
`adapters/read.rs:63-66`.
