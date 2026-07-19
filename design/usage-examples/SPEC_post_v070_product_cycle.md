# SPEC — post-v0.7.0 product cycle (usage-examples bug-hunt burndown)

**Status:** **r2** — folds the two independent Fable r1 reviews (general 0C/8I
`reviews/spec-post-v070-fable-review-r1.md`; tax-correctness 2C/5I
`reviews/spec-post-v070-fable-taxcorrectness-r1.md`). Re-review pending before the PLAN.
Fold provenance is tagged inline: `[G-*]` = general review finding, `[T-*]` = tax-correctness finding.
**Branch:** `feat/post-v070-product-cycle`.
**Design of record for:** the open UX-P4 / UX-P1 / UX-P2 / UX-P3 / M-1 follow-ups (see `FOLLOWUPS.md`).

## 0. Why this cycle exists

Authoring the six worked-example journeys (J1–J6) was the bug-hunt half of the usage-examples project;
it surfaced ~30 findings. 12 were resolved in the P0 build or the pre-v0.7.0 wording cleanup; UX-P3-1 was
discharged by the P3 clock seam (ledger reconciled `8e14066`). This spec covers the remaining ~17, all
`§3.1`-fence-barred from the docs cycle because they change **behavior or messages**. **The fence is
lifted** — this cycle exists to make those changes, reviewed, with goldens regenerated.

## 1. Scope

In: UX-P4-1, UX-P4-3, UX-P4-4, UX-P4-5, UX-P4-6, UX-P4-7, UX-P4-8, UX-P4-9, UX-P4-10, UX-P4-11,
UX-P4-12(b–i), UX-P1-3, UX-P1-7, UX-P1-8, UX-P1-10, UX-P2-1, UX-P3-2, N-R1, M-1.

**UX-P4-1 grew under review** — it is no longer "a banner on `report --tax-year`" but *pseudo disclosure
across every number-bearing surface*: (1) the CLI delta report, (2) the CLI dual-report absolute totals
`[T-I5/G-M4]`, (3) the **viewing TUI Tax tab** `[T-C2/G-I3]`, and (4) a fail-closed **`--write-carryover`
gate** `[T-C1]`. All four are phase-1 (correctness-cluster) owned; none rides past that phase.

Out: new tax law/schedules; MFS; the mixed-use-mortgage input (P8-owned); the "first real return"
schema-migration retirement (release-gate-owned, gated on real users).

**Non-negotiable invariant (§1).** No change may alter a *computed tax figure* for a correctly-specified
return. Both r1 reviews traced every change and confirmed the invariant holds *provided the refusal rules
are provably scoped to the invalid set* (a false-refuse prevents the CORRECT ledger from being built —
same harm, delivered differently `[G-§1]`). Every refusal rule below is therefore pinned to a
tax-law-justified invalid set with an explicit carve-out list.

## 2. Severity map (what gates)

- **Critical (block):** the two answered-ness false-negative channels folded from the tax-correctness
  review — `[T-C1]` `--write-carryover` pseudo-laundering into persisted year+1 inputs, and `[T-C2]` the
  silent TUI Tax tab. Both are now clauses of UX-P4-1.
- **Important (block):** UX-P4-1 (the disclosure item), UX-P4-4 (negative basis / bad TIN reach a filed
  form), and the correctness of the record-time validation predicate (UX-P4-3).
- **Minor / Nit:** the rest (per §2 of r1; unchanged).

## 3. Design decisions (folded; the re-review targets these)

### 3.1 UX-P4-1 — pseudo disclosure across all number-bearing surfaces (Important; C1+C2 folded)

**Problem.** Under `reconcile pseudo on`, an authoritative tax total is rendered with no `[PSEUDO]`
marker/banner on **four** surfaces the export attest-gate does not cover — the CLI delta report, the CLI
dual-report absolute totals, the viewing TUI Tax tab, and (worse) the `--write-carryover` *persistence*
path. The one attest-gated surface (export) is not enough; this is the answered-ness class.

**Predicate — "pseudo-contributed" `[G-I1, G-I2, T-endorse]`:**
`state.pseudo_active()` **OR** `provenance == Provenance::PseudoPlaceholder`.
- `pseudo_active()` (`state.rs:282`, `= pseudo_synthetic_count > 0`) catches every *synthetic-decision*
  (lot/FMV) channel — proven exhaustive for the row channel (`state.rs:131/164/199/231`).
- The **`PseudoPlaceholder` profile channel is a real false-negative** `[G-I1/T (fold.rs advisory is
  count-gated)]`: `resolve.rs:120-128` injects an all-$0 Single profile whenever `cfg.pseudo_reconcile`
  is on and nothing is stored — with `count == 0`. The provenance is in scope at the hook
  (`resolve_and_screen` returns it, `tax.rs:282-296`). The predicate MUST OR it in.
- **Do NOT narrow per-year** `[G-I2, T-§8]`: a pseudo lot merely present in the pool displaces
  HIFO/FIFO selection of *real* lots, so a year can be pseudo-influenced with zero pseudo-flagged rows.
  Vault-wide is the safe (over-fire) direction for a disclosure. (Optional, not required: exclude
  synthetics dated strictly after year-end.)
- Because the signal is vault-wide, the **banner text must be a TRUE vault-level statement** `[G-I2]`,
  e.g.: `⚠ [PSEUDO] This vault has pseudo-reconciled (deliberately-synthetic) entries; figures shown are`
  `an ESTIMATE, not filing-ready. See '[PSEUDO]' rows in 'btctax report' and the [PseudoReconcileActive]`
  `advisory in 'btctax verify'; resolve them before filing.` `[G-N1, T-N4]` (points rows→report,
  advisory→verify; "entries" covers both synthetic lots and `PseudoFmv` income).

**The four surfaces (all phase-1 owned):**
1. **CLI delta report** — thread `pseudo_active` into `TaxYearReport` (`tax.rs:429`) → `render_tax_outcome`
   (`render.rs:1018`): top banner + ` [PSEUDO]` suffix on the "TOTAL federal tax attributable" line
   (`render.rs:1056-1061`). Leading space kept — a last-field scraper reads `[PSEUDO]`, failing loud
   `[T-N2]`.
2. **CLI dual-report absolute totals `[T-I5, G-M4]`** — thread the same bool into `render_dual_report`
   (`render.rs:1173`); suffix "TOTAL TAX (L24)" (`render.rs:1229`) and "Absolute TOTAL TAX …"
   (`render.rs:1247`) — the lines a filer actually transcribes. (The placeholder-profile estimate path
   `tax.rs:272-279` renders through the delta block; banner+suffix cover it.)
3. **Viewing TUI Tax tab `[T-C2, G-I3]`** — `render_tax_content` (`btctax-tui/src/tabs/tax.rs:55-121`,
   total `:93`, charitable `:116-120`) has NO pseudo handling. Thread `snap.state.pseudo_active()` (same
   signal the export modal already uses) → banner line + `[PSEUDO]` on the total. TUI golden updated.
4. **`--write-carryover` gate `[T-C1]` (the Critical persistence channel)** — `write_back_carryover`
   (`main.rs:179-181` → `tax.rs:444-517`) persists §170(b)/(d) charitable carryover + §199A(c)(2) REIT/PTP
   carryforward into year+1's stored `ReturnInputs` (`tax.rs:507-509`), which carry no taint field —
   derived from pseudo-tainted state, invisible to next year's banner (`count==0` then). **REFUSE
   fail-closed** (nonzero exit, persist nothing) when the predicate holds, *before* `apply_carryover_
   writeback` (`tax.rs:507`), consistent with the export attest-gate. Warn-and-write is unacceptable for a
   persisted input.

**Acceptance KATs `[G-M1, T-N1]`:**
- (a) pseudo-active vault → each of surfaces 1/2/3 shows the banner + `[PSEUDO]` suffix; the operational
  §1-invariant guard: the existing tax-report golden's byte-diff across the fix shows **only** banner/suffix
  line insertions — every dollar figure byte-identical to pre-fix.
- (b) `PseudoPlaceholder` channel (pseudo on, `count==0`, no stored profile) → banner fires (the
  false-negative KAT).
- (c) reconciled vault / mode off → no banner, no suffix, on all surfaces.
- (d) `--write-carryover` on a pseudo-active vault → nonzero exit AND year+1 `ReturnInputs` unchanged
  (assert the stored inputs file is byte-identical before/after).
- Mutation: removing any surface's emit, or the write-carryover refuse, reds its KAT.

### 3.2 UX-P4-3 — record-time validation that mirrors the resolver, pseudo-safe (Minor→gating predicate)

**Problem.** classify/reclassify/void accept a typo'd/wrong-type/duplicate target with `Recorded decision`
(exit 0); the error surfaces only on the next `verify` as a `DecisionConflict` hard blocker; `void
<nonexistent>` self-blocks. Precedent that validates at record time: `set_donation_details`
(`reconcile.rs:1162-1188`).

**Decision — validate at record time, fail-closed (refuse), predicate pinned to MIRROR the resolver:**
- **Refuse iff the resolver would adjudicate the append as a NEW `DecisionConflict`** `[G-I4]`, judged
  against the **live, real (non-voided, non-synthetic) persisted** decision log — NOT a naive raw-log scan.
  This keeps the sanctioned flows working: **void-then-re-decide** (voided priors excluded) and **first
  real classify of a pseudo-defaulted target** (synthetics excluded) `[G-I4, T-I2]`.
- **EXEMPT `set-fmv` `[T-I1]`.** `ManualFmv` is *deliberately last-wins with no conflict*
  (`resolve.rs:564-568/593-597`) — re-pointing an FMV is a sanctioned correction the resolver honors with
  no void. Refusing it would break a correction a *correct income figure needs*. The refusal applies only
  to the **first-wins verbs**: `ClassifyInbound` (`resolve.rs:694-709`), `ReclassifyOutflow` (`:746-762`),
  `ReclassifyIncome` (`:807-821`).
- **`void`** additionally refuses non-revocable / already-voided targets (`resolve.rs:427/436`).
- **Pseudo-safe projection `[T-I2, T-C-class]`.** `session.project()` uses the stored config *including*
  `pseudo_reconcile` (`session.rs:556-562`), so the tainted projection makes real correcting targets look
  already-decided. Record-time validation MUST consult a **pseudo-OFF** view: the raw event log for
  existence/type + the persisted (void-folded) decision log for duplicates. Never the tainted projection.
- **Unify** every `DecisionConflict`-adjacent remedy hint to one phrasing that points at `events list`
  (see 3.6) + "void decision|N first".
- **Mandate: validator-mirrors-resolver** — implement as a shared helper or a shadow-projection so the
  record-time predicate cannot drift from the resolver's adjudication; if they ever disagree, the
  record-time layer is the wrong one `[T-I1]`.

**Acceptance KAT (both directions) `[G-I4]`:** each of {unknown ref, wrong-type ref, first-wins duplicate,
void-nonexistent, void-already-voided} refuses (nonzero, nothing recorded, `verify` clean after); AND each
of {valid new decision, void-then-re-decide, first real classify over a pseudo default, second `set-fmv`}
still succeeds. Mutation reds.

### 3.3 UX-P4-4 + UX-P1-3 — value validation at record time (Important / Minor)

**(a) Negative numeric inputs — per-flag sign policy `[G-I5, T-M1]`.** `parse_usd_arg` (`eventref.rs:77-79`,
no sign guard) feeds **~25 sites**; a guard inside it is WRONG (some negatives are legitimate). Guard
**per-flag** (precedent `main.rs:126-139`), never in the shared parser. Sign-policy table:

| Flag(s) | Site | Policy |
|---|---|---|
| `classify-inbound-self-transfer --basis` | `main.rs:998` | refuse < 0 (zero allowed — the app's own default) |
| `classify-inbound-income --fmv`, `classify-inbound-gift --fmv-at-gift`, `set-fmv --fmv` | `main.rs:965/988/1031` | refuse < 0 |
| `classify-inbound-gift --donor-basis` | `main.rs:982` | refuse < 0 |
| `reclassify-outflow --amount` (USD FMV), `--fee` | `main.rs:1026/1027` | refuse < 0 |
| `what-if … --price`, `optimize --proceeds` | `main.rs:324/400/247` | refuse < 0 |
| tax-profile money fields (MAGI, wages, etc.) | `main.rs:852-885` | per-field; **allow** legitimate negatives (`--other-net-capital-gain`) — decide each in the PLAN, default allow unless it lands on a form as a non-negative |

Tax rationale for the refusals: no legitimate negative cost basis exists (§1012; §1016 adjustments floor at
zero; §301(c)(2)–(3)/§733 excess-of-basis is *gain*, never negative basis) `[T-M1]`. Zero stays allowed.

**(b) Acquired-after-receipt `[T-M2]`.** Refuse `--acquired` (and `--donor-acquired` `[G-I5]`) strictly
after the receive/receipt date for a self-transfer-in / gift (impossible). BUT the two dates come from
different sources and may skew by a day (tz); the refusal message must **print the receipt date and its tz
basis** so the user can enter a consistent date. Same-day equality allowed. (PLAN may add
allow-plus-one-day-with-advisory instead of hard refuse — reviewer's call.)

**(c) EIN/TIN shapes `[G-I6, T-I3, T-M3]`.** Validate at the `set_donation_details` **choke point**
(`reconcile.rs:1162`), so the **TUI-edit form path** (`form.rs:1364-1420`) is covered, not just CLI parsing.
- `--appraiser-tin`: accept **EIN-shape OR SSN-shape** (26 CFR 301.6109-1(a)(1)(i): a TIN is SSN/ITIN/ATIN/
  **EIN**; `cli.rs:653` help says so). ITIN `9xx-xx-xxxx` passes SSN-shape (correct); masked `***-**-1234`
  refused (correct).
- `--donee-ein`: EIN-shape; **normalize** hyphenless 9-digit (`123456789` is a valid EIN); refuse SSN-shape
  (an individual is not a §170(c) qualified donee); it is **optional** (`cli.rs:646`) so the refuse message
  says "omit `--donee-ein` if the donee has none" (covers treaty charities without EINs).
- `--appraiser-ptin`: its own shape `P\d{8}` (or explicitly exclude from this check).

**(d) `--amount` unit + FMV warn `[G-I7, T-I4]`.** Add a `--amount` doc comment naming the unit (USD FMV).
WARN (stderr, non-fatal) when `FMV > 100 × (outflow_sats / 1e8) × recent-dataset-close` — **price-based,
NOT cost-basis** (a $0/low-basis long-held-BTC gift is the *common* case and would false-warn every time).
sats are on the `TransferOut` event; prices in `session.prices()`. **No-price fallback: skip the warn**
(state this explicitly — silent death of the guard is the failure mode). Refuse would be wrong
(high-appreciation gifts are real).

**Acceptance KATs:** negative basis refused on BOTH surfaces incl. the CLI `=` form; each other refusal
fires with the specified message; an EIN-shaped `--appraiser-tin` is ACCEPTED; a hyphenless donee EIN is
accepted; a sats-as-USD `--amount` warns but a legitimate high-appreciation FMV does NOT; the no-price path
does not warn. Dollar-figure invariant: an existing valid donation KAT's deduction is unchanged. Mutation
reds each guard.

### 3.4 UX-P4-9 — insufficient-balance message (Minor)

Distinguish zero-available from insufficient and show the number: zero → `no BTC available in <wallet> as
of <date>`; insufficient → `only <X> BTC available in <wallet> as of <date> (requested <Y>)`. RECON
RESOLVED (§9.4): both values in scope at `whatif.rs:234-236`; carry them on `WhatIfError::NoLots`
(`whatif.rs:137`); the harvest arms (`whatif.rs:530/534`, incl. `InvalidTarget → NoLots`) populate/ignore
the new fields mechanically `[T-N3, G-M6]`. Optional: name the excluded pending-transfer sats `[T-N3]`.
KAT: 0.5 held / sell 0.6 → the "only 0.5 … (requested 0.6)"; 0 held → the "no BTC".

### 3.5 UX-P4-10 — `report` exit-code contract (Nit)

`report --tax-year` returns **exit 1** when the requested computation is `TaxOutcome::NotComputable`
(mirrors `verify`); exit 0 for a rendered report. RECON RESOLVED (§9.2). Folds `[G-M2, T-M5]`:
- The man page documents **1 = ran but NO filing-ready number; 2 = command failed (ANY error)** —
  `run_to_exit` (`main.rs:38-45`) maps every `Err` (io/lock/store/refusal/uncomputable-profile) to 2, so
  "2 = usage" was inaccurate. Key on **non-zero**.
- Place the `return Ok(ExitCode::from(1))` **AFTER** the `--write-carryover` block `[G-M2]` (before it would
  silently skip a requested write-back — though note 3.1 clause 4 already refuses write-carryover under
  pseudo; the ordering still matters for the non-pseudo NOT-COMPUTABLE case).
- Two deliberate **exit-0 non-triggers** to document `[T-M5]`: a dual-report whose absolute total is refused
  but whose delta computed stays 0; a pseudo-active report stays 0 (the banner is the signal — a nonzero
  would break the estimate workflow).
- Update the stale `tax_report.rs:780` doc-comment ("exit 0").

### 3.6 UX-P4-11 — event-ref discoverability (Minor) — RESEQUENCED to phase 1 `[G-I8]`

Add `btctax events list` (additive, low golden-churn). **It moves into/before phase 1** because UX-P4-3's
unified refuse-hint (3.2) points at it — shipping the hint in phase 1 while the verb waits for phase 4 would
pin a message naming a nonexistent verb `[G-I8]`.

**Row universe `[G-M3]`:** every decidable event (income Receives, transfer legs, disposals) with columns
{ref, kind, date, amount, decided-status}. **Pseudo-defaulted events MUST list as decidable** `[T-I2
rider]` (they carry no persisted decision — else the banner's remedy path loses its discovery verb). If a
`decision|N` ref is shown (the "void decision|N first" remedy needs it discoverable), include decided rows
with their decision ref. Stable ordering (by event sequence). Add the man page + `make docs` regen (binary
-docs infra). Do NOT add per-row refs to `report` this cycle (golden churn without a correctness payoff)
`[G-M3/T-§8]`.

**KAT:** `events list` prints a decidable line whose ref, pasted verbatim into `reclassify-*`, is ACCEPTED
(closes the UX-P4-11 trap end-to-end); a pseudo-defaulted event appears as decidable.

## 4. Mechanical fixes (TDD only)

- **UX-P4-5** — WARN (stderr) that `--forms` is ignored on a full-return year; packet still writes. Rationale
  `[G-N4]`: honoring a slice of a *jointly-computed* 14-form packet is tax-unsound (a lone 8949 from a
  coordinated return would misstate) — hence warn-and-write-the-correct-superset, not refuse. KAT: warning
  emitted; packet bytes unchanged.
- **UX-P4-6** — add a pending line to the holdings view when pending > 0, from `stats.sigma_pending`
  (`state.rs:257-261`). **Unit: BTC** to match the holdings view convention `[G-N2]`. KAT: fully-pending
  vault shows it; reconciled vault does not.
- **UX-P4-7** — one shared human summary formatter for decision payloads (CLI + TUI bulk-void). It is
  **screen-only** — must NOT be reused by any CSV/form writer (cite the `[R0-I4]` screen-only precedent,
  `render.rs:57-62`) `[T-U-P4-7]`. KAT: formatter output; TUI no longer truncates mid-field.
- **UX-P4-8** — attach path + one-clause hint at vault-open (`session.rs:390-394`) and `--out`
  (`admin.rs:82`, `render.rs:586-618`), mirroring `AdapterError::Io { path, source }` (`adapters/lib.rs:23`).
  KAT: missing vault names the path + suggests `init`/`--vault`; `--out` collision names the path.
- **UX-P4-12(b–i)** — message/affordance papercuts (see FOLLOWUPS). (i) whichever default-year gate
  placement is chosen must **not change which year's packet is exported** — it moves the check, never the
  stored value `[T-U-P4-12]`. Pick the placement in the PLAN `[G-N3]` (default: align to the CLI's
  store-then-gate-at-export). KAT per sub-item that changes output.
- **M-1** — enable `serde_json` `preserve_order` for `income show` field order. It is a **workspace-global
  feature flip** `[G-M5, T-M4]`: audit the blast radius — verified safe today (conflict fingerprints are
  hand-rolled bytes `persistence.rs:25-55`; typed-struct serde is field-order-declared; the only
  `serde_json::Value` sites are `income show` display, input-form coverage tooling, update-prices API parse;
  `btctax-forms` is serde_json-free). Pin that enumeration into the KAT so no future Value-map iteration
  feeding persistence or a byte-compared artifact sneaks in. Regen the J6 golden. Low priority.

## 5. Docs items (new worked-example journeys)

- **UX-P1-7** manual `classify-inbound-income --fmv`; **UX-P1-8** two-exchange `match-self-transfers`;
  **UX-P1-10** genuine per-disposal `select-lots`. Each extends `xtask/src/examples.rs`, regens
  `examples.md` + PDF, byte-gated by `examples_golden_matches_committed`.
- **UX-P2-1** harden the SOFT `is_demonstrated` subsequence matcher: `path[0]` must be the first
  non-`-`-prefixed subcommand token (skip `--vault v.pgp`).

## 6. Polish (lowest priority)

- **UX-P3-2** colorized TUI PDF from the `.txt` style runs. **N-R1** de-stick the
  `no_direct_now_utc_in_production` scan (scan only the test module's brace span). KAT: a production
  `now_utc()` after a test module is caught.

## 7. Phasing (feeds the PLAN)

1. **Correctness cluster (gates first):** UX-P4-1 (all four surfaces incl. write-carryover gate + TUI tab),
   **UX-P4-11 `events list`** (moved up `[G-I8]` — the refuse-hint dependency), UX-P4-3 (validator-mirrors-
   resolver, pseudo-safe), UX-P4-4/UX-P1-3 (per-flag sign table + TIN shapes + price-based FMV warn).
2. **Legibility:** UX-P4-7, UX-P4-8, UX-P4-9.
3. **Report surfaces:** UX-P4-6, UX-P4-10.
4. **Affordances:** UX-P4-5, UX-P4-12(b–i).
5. **Display:** M-1.
6. **Docs:** UX-P1-7/8/10, UX-P2-1.
7. **Polish:** UX-P3-2, N-R1.
8. **Close:** whole-branch review, full CI-surface validation, regen all goldens, FOLLOWUPS burndown, push.

Each phase: TDD (guard reds without the fix), independent Fable review to 0C/0I, goldens regenerated,
commit, push. Per-phase burndown by ownership.

## 8. Open questions — RESOLVED by the r1 reviews

- 3.1 predicate: `pseudo_active() OR PseudoPlaceholder`, vault-wide, truthful wording, banner+suffix on all
  four surfaces (both reviews endorse vault-wide; narrowing is unsound).
- 3.2/3.3: fail-closed refuse is correct for the **first-wins verbs**; `set-fmv` exempt (last-wins);
  predicates pseudo-OFF + void-aware; shape checks match documented contracts; per-flag sign table.
- 3.5: exit 1 (not a distinct code); "non-zero ⇒ no filing-ready number"; 2 = any error.
- 3.6: `events list` alone (pseudo-aware); no in-report ref hint this cycle.

## 9. Verified recon anchors (2026-07-18; extended for r2)

**§9.1 UX-P4-1 pseudo signal.** `pseudo_tag()` `render.rs:62`; `.pseudo` `state.rs:131/164/199/231`;
`pseudo_synthetic_count` `state.rs:277` + `.pseudo_active()` `:282`; advisory `fold.rs:396-407` (count-gated
— hence the placeholder false-negative). Delta surface `render_tax_outcome` `render.rs:1018` (total
`:1056-1061`); **dual-report** `render_dual_report` `render.rs:1173` (L24 `:1229`, Absolute TOTAL TAX
`:1247`); **TUI tab** `render_tax_content` `btctax-tui/src/tabs/tax.rs:55-121` (total `:93`, charitable
`:116-120`); TUI export modal already reads `pseudo_active` `tui/src/lib.rs:263-311`. `TaxYearReport` built
`tax.rs:429` (Debug-only, no serde); `provenance` in scope `tax.rs:282-296`; placeholder inject
`resolve.rs:120-128`. **write-carryover** `main.rs:179-181` → `write_back_carryover` `tax.rs:444-517`
(project `:455`, `assemble_absolute` `:486`, `apply_carryover_writeback` persists `:507-509`).

**§9.2 UX-P4-10 exit.** `verify` `ExitCode::from(1)` `main.rs:112-118`; Report arm `main.rs:140-182` →
terminal `Ok(SUCCESS)` `main.rs:933`; `NotComputable` render `render.rs:1027-1029`; every `Err`→2
`main.rs:38-45`; resolver-uncomputable `Err` `tax.rs:290`; stale doc `tax_report.rs:780`.

**§9.3 UX-P4-3/4 validation.** Precedent `set_donation_details` `reconcile.rs:1162-1188` (projects); single
verbs `classify_inbound` `:41`, `reclassify_outflow` `:62`, `set_fmv` `:85`, `reclassify_income` `:1136`,
`void` `:110`; append `:28`. Resolver conflict/first-wins: `ClassifyInbound` `resolve.rs:694-709`,
`ReclassifyOutflow` `:746-762`, `ReclassifyIncome` `:807-821`; **`ManualFmv` last-wins no-conflict**
`resolve.rs:564-568/593-597`; void revocability `resolve.rs:427/436`; pseudo fill Phase A/B/C
`resolve.rs:943-949/966-976/1008-1021`, real-before-synthetic `:563-780`/`:939-941`; `session.project()`
uses stored pseudo cfg `session.rs:556-562`. `--basis` etc. via `parse_usd_arg` `eventref.rs:77-79` (no sign
guard); negative-reject precedent `main.rs:126-139`; numeric flag sites `main.rs:247/324/400/852-885/965/
982/988/1026/1027/1031/998`. EIN/TIN built verbatim `main.rs:1095-1106`; help contract `cli.rs:646/653/
656-658`; TUI-edit donation form `tui-edit/src/edit/form.rs:1364-1420`.

**§9.4 UX-P4-9 balance.** core `btctax-core/src/whatif.rs`: `NoLots` `:131/137`, raise `:234-236`,
`HarvestStatus::NoLots` `:516/694`, `InvalidTarget→NoLots` `:530/534`; cli map `btctax-cli/src/cmd/
whatif.rs:170-172`.

**§9.5 UX-P4-8 io.** `CliError::Io` `cli/lib.rs:44-45`, `StoreError::Io` `store/lib.rs:19-20`; vault-open
`session.rs:390-394` → `Vault::open` `vault.rs:117/129`; export-out `admin.rs:82/113`, `export_snapshot`
`vault.rs:263-271`, `write_csv_exports` `render.rs:586/593/595/605-618`; precedent
`AdapterError::Io{path,source}` `adapters/lib.rs:23-28` / `read.rs:63-66`.

**§9.6 M-1 preserve_order blast radius.** fingerprints hand-rolled bytes `persistence.rs:25-55`; `Value`
sites = `income show` display + input-form coverage tooling + update-prices API parse; `btctax-forms`
serde_json-free.
