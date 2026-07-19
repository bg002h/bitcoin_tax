# SPEC — post-v0.7.0 product cycle (usage-examples bug-hunt burndown)

**Status:** **GREEN** (r4) — the r4 re-review is **0C / 0I**
(`reviews/spec-post-v070-fable-review-r4.md`); the r4 Minors/Nits (R4-M1 banner flag spelling, R4-M3
non-revocable-void KAT arm, the two writer-count wording nits) are folded inline here — the reviewer
pre-approved them as one-clause fixes, no re-review needed. Review arc: r1 (0C/8I general, 2C/5I
tax-correctness) → r2 (0C/4I, 0C/1I) → r3 (0C/1I) → r4 (0C/0I). Fold provenance tagged inline:
`[G-*]`/`[T-*]` = r1; `[G2-*]`/`[T2-*]` = r2; `[R3-*]`/`[R4-*]` = r3/r4 findings.
**Branch:** `feat/post-v070-product-cycle`.

## 0. Why this cycle exists

Authoring J1–J6 was the bug-hunt half of the usage-examples project; it surfaced ~30 findings. 12 were
resolved earlier; UX-P3-1 was discharged by the P3 clock seam (`8e14066`). This spec covers the remaining
~17, all fence-barred from the docs cycle because they change **behavior or messages**. The fence is lifted.

## 1. Scope

In: UX-P4-1, UX-P4-3, UX-P4-4, UX-P4-5, UX-P4-6, UX-P4-7, UX-P4-8, UX-P4-9, UX-P4-10, UX-P4-11,
UX-P4-12(b–i), UX-P1-3, UX-P1-7, UX-P1-8, UX-P1-10, UX-P2-1, UX-P3-2, N-R1, M-1.

**UX-P4-1 is pseudo disclosure across every number-bearing surface:** (1) CLI delta report, (2) CLI
dual-report absolute totals, (3) the shared TUI Tax tab (viewer + editor), and (4) a fail-closed
persistence gate on `--write-carryover`. All phase-1 (correctness-cluster) owned.

Out: new tax law/schedules; MFS; the mixed-use-mortgage input (P8-owned); the "first real return"
schema-migration retirement (release-gate-owned).

**Non-negotiable invariant (§1).** No change may alter a *computed tax figure* for a correctly-specified
return. A false-refuse is the same harm delivered differently (it prevents the CORRECT ledger from being
built) `[G-§1]`, so every refusal below is pinned to a tax-law-justified invalid set with explicit carve-outs.

## 2. Severity map (what gates)

- **Critical (block):** the two answered-ness false-negative channels — `[T-C1]` `--write-carryover`
  pseudo-laundering into persisted year+1 inputs, and `[T-C2]` the silent TUI Tax tab. Both are clauses of
  UX-P4-1 (RESOLVED in r2; verified by both lenses).
- **Important (block):** UX-P4-1 (disclosure), UX-P4-4 (negative basis / bad TIN reach a filed form), the
  correctness of the record-time validation predicate (UX-P4-3).
- **Minor / Nit:** the rest.

## 3. Design decisions

### 3.1 UX-P4-1 — pseudo disclosure across all number-bearing surfaces (Important; C1+C2 folded)

**Problem.** Under `reconcile pseudo on`, an authoritative tax total renders with no `[PSEUDO]` flag on four
surfaces the export attest-gate does not cover — the CLI delta report, the CLI dual-report absolute totals,
the TUI Tax tab, and the `--write-carryover` *persistence* path.

**Predicate — "pseudo-contributed" `[G-I1, G-I2, T-endorse]`:**
`state.pseudo_active()` (`state.rs:282`) **OR** `provenance == Provenance::PseudoPlaceholder`.
- `pseudo_active()` catches every synthetic-decision (lot/FMV) channel — exhaustive for the row channel.
- The **`PseudoPlaceholder` profile channel is a real false-negative** `[G-I1]`: `resolve.rs:120-128`
  injects an all-$0 Single profile when `cfg.pseudo_reconcile` is on and nothing is stored (`count == 0`);
  the provenance is in scope at the hook (`tax.rs:282-296`). The predicate MUST OR it in.
- **Do NOT narrow per-year** `[G-I2]`: a pseudo lot in the pool displaces HIFO/FIFO selection of real lots,
  so a year can be pseudo-influenced with zero pseudo-flagged rows. Vault-wide is the safe over-fire
  direction. (Optional: exclude synthetics dated strictly after year-end.)
- **The threaded bool is named `pseudo_contributed` `[T2-N3]`** (NOT `pseudo_active`) — it carries the full
  OR-predicate; the disjunct name would invite an implementer to thread `state.pseudo_active()` and drop the OR.

**Banner text — CHANNEL-AWARE `[G2-2]`** (the single pinned text was false on the placeholder channel):
- **Synthetic channel** (`pseudo_active()`): `⚠ [PSEUDO] This vault has pseudo-reconciled`
  `(deliberately-synthetic) entries; figures shown are an ESTIMATE, not filing-ready. See '[PSEUDO]' rows in`
  `'btctax report' and the [PseudoReconcileActive] advisory in 'btctax verify'; resolve them before filing.`
  `[G-N1, T-N4]`
- **Placeholder channel** (`PseudoPlaceholder`, `count == 0`, no `[PSEUDO]` rows and no verify advisory to
  point at): `⚠ [PSEUDO] These figures are estimated on a synthetic $0 placeholder profile — no tax profile`
  `or full-return inputs are stored for this year. This is an ESTIMATE, not filing-ready. Set a tax profile`
  `('btctax tax-profile --year <Y> …' — setting is the default; '--show' inverts), import inputs`
  `('btctax income import'), or turn pseudo mode off ('btctax reconcile pseudo off').`
  Each clause is true for its channel; the remedy pointers are live for the channel that fires them.
- **Channel precedence `[R3-min]`:** the two channels are **mutually exclusive** — the synthetic channel
  requires `pseudo_synthetic_count > 0`, the placeholder channel requires `count == 0` — so no `count`
  reading yields a false or ambiguous banner, and no precedence rule is needed.

**The four surfaces (all phase-1 owned):**
1. **CLI delta report** — thread `pseudo_contributed` into `TaxYearReport` (`tax.rs:429`) →
   `render_tax_outcome` (`render.rs:1018`): top banner + ` [PSEUDO]` suffix on "TOTAL federal tax
   attributable" (`render.rs:1056-1061`). Leading space kept (a last-field scraper reads `[PSEUDO]`,
   failing loud `[T-N2]`).
2. **CLI dual-report absolute totals `[T-I5]`** — thread the same bool into `render_dual_report`
   (`render.rs:1173`); suffix "TOTAL TAX (L24)" (`:1229`) and "Absolute TOTAL TAX …" (`:1247`) — the
   filer-transcribed lines. (The dual block is provenance-gated to `ReturnInputs` (`tax.rs:306`), so the
   placeholder disjunct is inert there; the delta banner+suffix cover the placeholder path.)
3. **Shared TUI Tax tab `[T-C2, G-I3]`** — `render_tax_content` (`btctax-tui/src/tabs/tax.rs:55-121`, total
   `:93`, charitable `:116-120`); `tabs/tax.rs::render` is the App-free entry the **editor** crate also
   calls (`:18-23`), so one change covers both TUIs. Thread `snap.state.pseudo_active()`. **Narrower signal
   is sound only by an enumeration invariant `[G2-5, T2-M2]`:** `resolve_all_screened` enumerates only
   `tax_profile::years ∪ return_inputs::years` (`session.rs:497-498`), so a `PseudoPlaceholder` profile
   never reaches `snap.profiles` — a pseudo-on/count-0/profile-less year renders NOT COMPUTABLE, not a
   number. **Trip-wire:** if the TUI ever resolves a bare year on demand (parity with the CLI placeholder
   path), the full OR-predicate applies here. KAT (f) pins it.
4. **`--write-carryover` gate `[T-C1] + [G2-4]` (the Critical persistence channel)** —
   `write_back_carryover` (`main.rs:179-181` → `tax.rs:444-517`) persists §170(b)/(d) charitable carryover +
   §199A(c)(2) REIT/PTP carryforward into year+1's stored `ReturnInputs` (`tax.rs:507-509`), which carry no
   taint field. **REFUSE fail-closed** (nonzero exit, persist nothing), *before* `apply_carryover_writeback`
   (`tax.rs:507`), when **EITHER**:
   (4a) `pseudo_contributed` holds `[T-C1]` — at this gate the `PseudoPlaceholder` disjunct is structurally
   inert (`write_back_carryover` already refuses non-`ReturnInputs` provenance, `tax.rs:478-483`), so the
   operative half is `pseudo_active()` = exactly the taint feeding `assemble_absolute` (`tax.rs:486`); OR
   (4b) the year's **delta outcome is `NotComputable`** `[G2-4]` — a hard-blocked ledger. `write_back_
   carryover` has no blocker gate (`tax.rs:444-517`; `screen_compute_dependent` checks QBI/AMT/TI≤0, not
   `state.blockers`), so today a hard-blocked vault persists a carryover derived from a ledger the engine
   refuses to answer for. Refuse (message names the blocker). This dissolves the §3.5 ordering question.

**Acceptance KATs `[G-M1, T-N1, G2-7]`:**
- (a) §1-invariant guard, **two clauses `[G2-7]`:** (i) the committed examples golden is byte-identical
  across the fix (no journey is pseudo-active — verified: zero `pseudo` in `examples.md`); (ii) a NEW
  pseudo-active fixture KAT asserts its dollar figures byte-identical pre/post-fix with ONLY banner/suffix
  lines inserted, on surfaces 1/2/3.
- (b) `PseudoPlaceholder` channel (pseudo on, `count==0`, no stored profile) → banner fires with the
  **placeholder-variant wording** `[G2-2]` (the false-negative + correct-text KAT).
- (c) reconciled vault / mode off → no banner, no suffix, on all surfaces.
- (d) `--write-carryover` on a pseudo-active vault → nonzero exit AND year+1 `ReturnInputs` byte-identical.
- (e) `--write-carryover` on a hard-blocked **non-pseudo** `NotComputable` vault `[G2-4]` → nonzero exit AND
  year+1 `ReturnInputs` byte-identical.
- (f) TUI Tax tab: pseudo on + no stored profile/inputs for the year → NOT COMPUTABLE, never a number
  `[T2-M2]`.
- Mutation: removing any surface's emit, or either write-carryover refuse branch, reds its KAT.

### 3.2 UX-P4-3 — record-time validation that mirrors the resolver, pseudo-safe

**Problem.** classify/reclassify/void accept a typo'd/wrong-type/duplicate target with `Recorded decision`
(exit 0); the error surfaces only at the next `verify`. Precedent: `set_donation_details` validates at record
time (`reconcile.rs:1162-1188`).

**Decision — refuse at record time, predicate MIRRORS the resolver:**
- **Refuse iff the resolver would adjudicate the append as a NEW `DecisionConflict`** `[G-I4]`, judged
  against the **live real (non-voided, non-synthetic) persisted** decisions — so **void-then-re-decide** and
  **first real classify of a pseudo-defaulted target** keep working `[G-I4, T-I2]`.
- **Existence/type validated against the resolver's own EFFECTIVE `applied` map, pseudo forced OFF
  `[T2-I1, R3-I1]`.** Do NOT hand-rebuild a subset of `applied` — reuse the resolver's pass-1c/1d/1e
  construction in a **shadow projection** with `pseudo_reconcile` forced off, so the view is
  *definitionally* whatever the resolver sees (`applied.get(target).unwrap_or(&raw.payload)`,
  `resolve.rs:728-730/789-791`; ManualFmv pass 1d `:575-577`). Under pseudo-OFF `applied` has **two real
  writers**, both before pseudo Phase A: accepted-conflict `SupersedeImport` payloads (`resolve.rs:513`) and
  live real `ClassifyRaw` rewrites (pass 1c `:543-560`); for a target neither rewrote, existence/type fall
  back to the raw event log via `.unwrap_or(&raw.payload)`. (The `:522` accept-first and `:949` Phase-A
  inserts are both `pseudo_on`-gated → absent under pseudo-OFF.) An
  enumerate-the-writers view is fragile — the r3 draft named only `ClassifyRaw` and was one channel short of
  `SupersedeImport` `[R3-I1]`, which (i) false-refused `set-fmv`/`reclassify-income` on an accept-governed
  Income target the resolver honors, and (ii) missed refusing `classify-raw` on an accept-governed target
  the resolver adjudicates a NEW conflict (`applied.contains_key`, `:551`). Mirroring the resolver's
  `applied` closes the whole class. It also covers the sanctioned **post-`pseudo approve` correction**
  (approve persists real zero-value `ClassifyRaw` placeholders, `resolve.rs:223`; `set-fmv` supplies the true
  FMV a correct income figure needs). Never the tainted projection (`session.project()` uses stored pseudo
  cfg, `session.rs:556-562`).
- **`set-fmv` is exempt from the DUPLICATE refusal ONLY `[G2-3, T-I1]`.** `ManualFmv` is deliberately
  last-wins with no conflict (`resolve.rs:564-568/593-597`) — re-pointing an FMV is a sanctioned correction.
  But `set-fmv` **still gets existence/type validation** like every verb (a `set-fmv <unknown/wrong-type ref>`
  IS a new Hard `DecisionConflict` the resolver excludes, so record-time must refuse it — else the original
  UX-P4-3 trap survives on the verb that feeds ordinary income).
- **First-wins verbs (duplicate refused):** `ClassifyInbound` (`resolve.rs:694-709`), `ReclassifyOutflow`
  (`:746-762`), `ReclassifyIncome` (`:807-821`), **and `ClassifyRaw`** (pass 1c first-wins,
  `resolve.rs:543-560`; CLI `reconcile classify-raw`) `[G2-6, T2-M1]`.
- **`void`** additionally refuses non-revocable (`SupersedeImport`/`RejectImport`/`VoidDecisionEvent`,
  `resolve.rs:423-440`) / already-voided targets.
- **Choke point `[T2-M1, R3-M]`:** the single-verb append fns (`reconcile.rs:41/62/85/110/301/1136` — incl.
  `classify_raw` `:301`). The bulk `apply_*`
  paths (`reconcile.rs:286/395/438`) append via their own loops with **plan-generated (not user-typed) refs**
  — deliberately OUT of scope for record-time validation (a validate-batch-then-append-batch shape risks
  intra-batch adjudication diverging from the resolver's ascending-seq first-wins); state this so the PLAN
  neither silently skips nor reinvents it.
- **Unify** the `DecisionConflict` remedy hints to one phrasing pointing at `events list` (3.6) + "void
  decision|N first".
- **Mandate: validator-mirrors-resolver** — shared helper or shadow-projection; if they disagree, the
  record-time layer is wrong `[T-I1]`.

**Acceptance KAT (both directions):** refuse {unknown ref, wrong-type ref (raw-log, ClassifyRaw'd, AND
accept-governed `SupersedeImport` cases `[R3-I1]`), **`set-fmv <bad-ref>`** `[G2-3]`, first-wins duplicate,
**`classify-raw` on an accept-governed target** `[R3-I1]`, void-nonexistent, void-already-voided}; accept
{valid new decision, void-then-re-decide, first real classify over a pseudo default, second `set-fmv` on a
valid target, **`set-fmv`/`reclassify-income` on a target whose Income type comes from a live real
`ClassifyRaw`** `[T2-I1]` **or from an accepted `SupersedeImport` conflict** `[R3-I1]`; the same target with
that decision voided → refused wrong-type (voided arm is the **`ClassifyRaw` antecedent only** — a
`SupersedeImport` is non-revocable, `resolve.rs:423-440`, so it has no voided counterpart) `[R4-M3]`}.
Mutation reds.

### 3.3 UX-P4-4 + UX-P1-3 — value validation at record time (Important / Minor)

**(a) Negative numeric inputs — per-flag sign policy `[G-I5, T-M1, G2-1, T2-M3]`.** `parse_usd_arg`
(`eventref.rs:77-79`) and `parse_sell_arg` (`whatif.rs:481-488`) have no sign guard. Guard **per-flag**
(precedent `main.rs:126-139`), never in the shared parser. Table:

| Flag(s) | Site | Policy |
|---|---|---|
| `classify-inbound-self-transfer --basis` | `main.rs:998` | refuse < 0 (zero allowed — the app's default) |
| `classify-inbound-income --fmv`, `classify-inbound-gift --fmv-at-gift`, `set-fmv --fmv` | `main.rs:965/988/1031` | refuse < 0 |
| `classify-inbound-gift --donor-basis` | `main.rs:982` | refuse < 0 |
| `reclassify-outflow --amount` (USD FMV), `--fee` | `main.rs:1026/1027` | refuse < 0 |
| `what-if … --price`, `optimize --proceeds` | `main.rs:324/400/247` | refuse < 0 |
| **`what-if sell/optimize --sell` (sats, `parse_sell_arg`)** `[G2-1]` | `main.rs:305/224` | **refuse ≤ 0** (a negative sell survives the pool check `whatif.rs:234` and renders a fictional LOSS, `:242`) |
| **what-if/harvest ad-hoc `--income`, `--magi`, `--carryforward-in`** `[G2-1, T2-M3]` | `main.rs:347-353`, `:421-427` | per-field (planning-only, no filed-form contact); `--carryforward-in` is a loss magnitude → refuse < 0; `--income`/`--magi` follow the tax-profile posture. Decide in the PLAN |
| tax-profile money fields | `main.rs:852-907` | per-field; **allow** legitimate negatives (`--other-net-capital-gain`). NOTE `--w2-ss-wages` (`:887`), `--w2-medicare-wages` (`:895`), `--schedule-c-expenses` (`:905`) are **ALREADY guarded** (`:890/:898/:908`) — cite as the in-repo precedent, exclude from re-work `[G2-1]` |

Tax rationale: no legitimate negative cost basis exists (§1012; §1016 floors adjustments at zero;
§301(c)(2)–(3)/§733 excess-of-basis is *gain*, never negative basis). Zero stays allowed.

**(b) Acquired-after-receipt `[T-M2]`.** Refuse `--acquired` / `--donor-acquired` `[G-I5]` strictly after the
receive/receipt date (impossible). The two dates come from different sources and may skew by a day (tz); the
refusal message must **print the receipt date and its tz basis**. Same-day allowed. (PLAN may substitute
allow-plus-one-day-with-advisory — reviewer's call.)

**(c) EIN/TIN shapes `[G-I6, T-I3, T-M3]`.** Validate at the **shared side-table write choke point**
`donation_details::set` — the single point BOTH the CLI (`set_donation_details`) and the TUI-edit form
(`persist_donation_details`) converge on. *(As-built correction, review r1: the earlier cite
`reconcile.rs:1162` (`set_donation_details`) is CLI-only — the TUI persists via `persist_donation_details`
→ `donation_details::set`, bypassing it; `set` is the true choke point that covers both surfaces.)*
- `--appraiser-tin`: accept **EIN-shape OR SSN-shape OR a bare 9-digit** (26 CFR 301.6109-1(a)(1)(i): a TIN
  is SSN/ITIN/ATIN/**EIN**; `cli.rs:653`). ITIN `9xx-xx-xxxx` passes SSN-shape; masked `***-**-1234` refused.
  *(As-built, review r1 M1: bare 9-digit is accepted for the same `[T2-N2]` anti-hardening reason as
  `--donee-ein` below — refusing an unformatted real TIN would be a false-refuse; the in-repo fixtures use
  bare-9 appraiser TINs. Accepts strictly more, refuses nothing the literal rule accepts.)*
- `--donee-ein`: EIN-shape; **normalize** hyphenless 9-digit (`123456789` → valid EIN); refuse SSN-shape
  (an individual is not a §170(c) donee); optional (`cli.rs:646`) → refuse message says "omit `--donee-ein`
  if the donee has none". **Note the inherent ambiguity `[T2-N2]`:** a hyphenless 9-digit necessarily also
  accepts an unhyphenated SSN (no shape check distinguishes them) — this is correct (refusing hyphenless
  would refuse real unformatted EINs); do NOT "harden" it into a false refuse.
- `--appraiser-ptin`: own shape `P\d{8}` (or explicit exclusion).

**(d) `--amount` unit + FMV warn `[G-I7, T-I4, T2-N1]`.** Add a `--amount` doc comment (unit = USD FMV). WARN
(stderr, non-fatal) when `FMV > 100 × (outflow_sats / 1e8) × close-at-the-outflow-date` — **price-based**
(the FMV of donated BTC is value **at the contribution date**, 26 CFR §1.170A-1(c)(1); use the event-date
close, not a "recent" close `[T2-N1]`). A $0/low-basis long-held-BTC gift is the *common* case, so a
cost-basis threshold would false-warn every time — repudiated. **No-price fallback: skip the warn** (state
explicitly — silent death of the guard is the failure mode). Refuse would be wrong.

**Acceptance KATs:** negative basis refused on BOTH surfaces incl. the CLI `=` form; **`--sell=-1`** refused
with a message assert `[R3-nit]` (the `=` form — the space form `--sell -1` is clap-rejected pre-fix, so it
cannot witness the guard under mutation); each other refusal fires with the specified message; EIN-shaped
`--appraiser-tin` accepted; hyphenless donee
EIN accepted; sats-as-USD `--amount` warns but a legitimate high-appreciation FMV does not; no-price path
does not warn. Dollar invariant: an existing valid donation KAT's deduction unchanged. Mutation reds each.

### 3.4 UX-P4-9 — insufficient-balance message (Minor)

zero → `no BTC available in <wallet> as of <date>`; insufficient → `only <X> BTC available in <wallet> as
of <date> (requested <Y>)`. Both values in scope at `whatif.rs:234-236`; carry them on `WhatIfError::NoLots`
(`whatif.rs:137`); harvest arms (`whatif.rs:530/534`, `InvalidTarget → NoLots`) populate/ignore mechanically
`[T-N3, G-M6]`. KAT: 0.5 held / sell 0.6 → "only 0.5 … (requested 0.6)"; 0 held → "no BTC".

### 3.5 UX-P4-10 — `report` exit-code contract (Nit)

`report --tax-year` returns **exit 1** on `TaxOutcome::NotComputable` (mirrors `verify`); exit 0 for a
rendered report. Folds `[G-M2, T-M5]`:
- Man page: **1 = ran but NO filing-ready number; 2 = command failed (ANY error)** (`run_to_exit` maps every
  `Err` to 2, `main.rs:38-45`). Key on **non-zero**.
- Place `return Ok(ExitCode::from(1))` after printing. The `--write-carryover`-on-`NotComputable` case that
  earlier made the placement ambiguous is now **refused fail-closed by 3.1 clause 4b** `[G2-4]` (nonzero,
  persists nothing) before any exit-1 return — so the ordering question dissolves.
- Two deliberate **exit-0 non-triggers** `[T-M5]`: a dual-report whose absolute total is refused but whose
  delta computed; a pseudo-active report (the banner is the signal) — **UNLESS `--write-carryover` is passed,
  which 3.1 clause 4a refuses** `[G2-8]` (cross-ref so the two clauses cannot read as conflicting).
- Update the stale `tax_report.rs:780` doc-comment ("exit 0").

### 3.6 UX-P4-11 — event-ref discoverability (Minor) — RESEQUENCED to phase 1 `[G-I8]`

Add `btctax events list` (additive, low golden-churn); it moves into/before phase 1 because UX-P4-3's
unified refuse-hint (3.2) names it `[G-I8]`. **Row universe `[G-M3]`:** every decidable event with columns
{ref, kind, date, amount, decided-status}. *(As-built, review r1 M2: the universe is the reconciliation-
CLASSIFICATION surface — `TransferIn` / `TransferOut` / `Unclassified` / `ImportConflict` / `Income`, the
verbs 3.2's refuse-hint names. A `Dispose` is EXCLUDED: its only decision is specific-ID `select-lots`, a
distinct flow whose refs come from the `disposals.csv` `event` column, not this surface. An `Acquire` is a
determined import no verb retargets.)* **Pseudo-defaulted events MUST list as decidable** `[T-I2 rider]`
(no persisted decision). If a `decision|N` ref is shown (the "void decision|N first" remedy needs it),
include decided rows with their decision ref. Stable ordering (by event sequence). Add the man page +
`make docs` regen (single-sourced from clap doc-comments — mechanical). No per-row refs in `report` this
cycle. **KAT:** a listed ref pasted verbatim into `reclassify-*` is ACCEPTED; a pseudo-defaulted event lists
as decidable.

## 4. Mechanical fixes (TDD only)

- **UX-P4-5** — WARN (stderr) that `--forms` is ignored on a full-return year; packet still writes.
  Rationale `[G-N4]`: honoring a slice of a jointly-computed 14-form packet is tax-unsound. KAT: warning
  emitted; packet bytes unchanged.
- **UX-P4-6** — pending line in the holdings view when pending > 0, from `stats.sigma_pending`
  (`state.rs:257-261`). **Unit: BTC** `[G-N2]`. KAT: fully-pending vault shows it; reconciled does not.
- **UX-P4-7** — one shared **screen-only** human summary formatter for decision payloads (CLI + TUI
  bulk-void); must NOT be reused by any CSV/form writer (cite the `[R0-I4]` screen-only precedent,
  `render.rs:57-62`). KAT: formatter output; TUI no longer truncates mid-field.
- **UX-P4-8** — attach path + one-clause hint at vault-open (`session.rs:390-394`) and `--out`
  (`admin.rs:82`, `render.rs:586-618`), mirroring `AdapterError::Io { path, source }` (`adapters/lib.rs:23`).
  KAT: missing vault names the path + suggests `init`/`--vault`; `--out` collision names the path.
- **UX-P4-12(b–i)** — message/affordance papercuts (see FOLLOWUPS). (i) whichever default-year gate
  placement is chosen must **not change which year's packet is exported** `[T-U-P4-12]`. Pick it in the PLAN
  `[G-N3]` (default: align to the CLI's store-then-gate-at-export). KAT per output-changing sub-item.
- **M-1** — enable `serde_json` `preserve_order` for `income show`. Workspace-global flip `[G-M5, T-M4]`:
  audit — verified safe (fingerprints hand-rolled bytes `persistence.rs:25-55`; typed serde field-ordered;
  `Value` sites = `income show` display + input-form coverage tooling + update-prices API parse;
  `btctax-forms` serde_json-free). Pin that enumeration in the KAT; regen J6 golden. Low priority.

## 5. Docs items (new worked-example journeys)

- **UX-P1-7** manual `classify-inbound-income --fmv`; **UX-P1-8** two-exchange `match-self-transfers`;
  **UX-P1-10** genuine per-disposal `select-lots`. Each extends `xtask/src/examples.rs`, regens `examples.md`
  + PDF, byte-gated by `examples_golden_matches_committed`.
- **UX-P2-1** harden the SOFT `is_demonstrated` subsequence matcher: `path[0]` must be the first
  non-`-`-prefixed subcommand token (skip `--vault v.pgp`).

## 6. Polish (lowest priority)

- **UX-P3-2** colorized TUI PDF from the `.txt` style runs. **N-R1** de-stick the
  `no_direct_now_utc_in_production` scan (scan only the test module's brace span). KAT: a production
  `now_utc()` after a test module is caught.

## 7. Phasing (feeds the PLAN)

1. **Correctness cluster (gates first):** UX-P4-1 (four surfaces incl. write-carryover pseudo+NotComputable
   gate + TUI tab), **UX-P4-11 `events list`** (moved up `[G-I8]`), UX-P4-3 (validator-mirrors-resolver,
   effective-payload, pseudo-safe), UX-P4-4/UX-P1-3 (sign table + TIN shapes + price-based FMV warn).
2. **Legibility:** UX-P4-7, UX-P4-8, UX-P4-9.
3. **Report surfaces:** UX-P4-6, UX-P4-10.
4. **Affordances:** UX-P4-5, UX-P4-12(b–i).
5. **Display:** M-1.
6. **Docs:** UX-P1-7/8/10, UX-P2-1.
7. **Polish:** UX-P3-2, N-R1.
8. **Close:** whole-branch review, full CI-surface validation, regen all goldens, FOLLOWUPS burndown, push.

Each phase: TDD (guard reds without the fix), independent Fable review to 0C/0I, goldens regenerated,
commit, push. Per-phase burndown by ownership.

## 8. Open questions — RESOLVED by the reviews

- 3.1: `pseudo_contributed = pseudo_active() OR PseudoPlaceholder`, vault-wide, channel-aware banner text,
  banner+suffix on all four surfaces, write-carryover refuses on pseudo OR NotComputable.
- 3.2: refuse for first-wins verbs (incl. ClassifyRaw); `set-fmv` exempt from the DUPLICATE refusal only;
  effective-payload pseudo-OFF view; validator-mirrors-resolver.
- 3.5: exit 1 = no filing-ready number; 2 = any error.
- 3.6: `events list` alone (pseudo-aware).

## 9. Verified recon anchors (2026-07-18; extended for r3)

**§9.1 UX-P4-1.** `pseudo_tag()` `render.rs:62`; `.pseudo` `state.rs:131/164/199/231`;
`pseudo_synthetic_count` `state.rs:277` + `.pseudo_active()` `:282`; advisory (count-gated) `fold.rs:396-407`.
Delta `render_tax_outcome` `render.rs:1018` (total `:1056-1061`); dual `render_dual_report` `:1173` (L24
`:1229`, Absolute `:1247`, provenance-gated `tax.rs:306`); TUI `render_tax_content` `tabs/tax.rs:55-121`
(App-free entry `:18-23`; NOT-COMPUTABLE arms `:59-63/68-70`); TUI snapshot `unlock.rs:171-219` via
`resolve_all_screened` enum `session.rs:497-498`; export modal reads pseudo `tui/lib.rs:263-311`.
`TaxYearReport` `tax.rs:429` (Debug-only); `provenance` `tax.rs:282-296`; placeholder inject
`resolve.rs:120-128` (`Provenance::PseudoPlaceholder` `:30`). write-carryover `main.rs:179-181` →
`write_back_carryover` `tax.rs:444-517` (provenance gate `:478-483`, project `:455`, `assemble_absolute`
`:486`, persist `:507-509`; `screen_compute_dependent` no-blocker-gate `return_1040.rs:584-650`); derivative
preserve `tax.rs:66-100`; other `return_inputs::set` writers `answer.rs:212`/`input_form_store.rs:299`
(user-typed).

**§9.2 UX-P4-10.** `verify` exit1 `main.rs:112-118`; Report arm `:140-182` → terminal SUCCESS `:933`;
`NotComputable` render `render.rs:1027-1029`; every `Err`→2 `main.rs:38-45`; resolver-uncomputable `tax.rs:290`;
stale doc `tax_report.rs:780`.

**§9.3 UX-P4-3/4.** Precedent `set_donation_details` `reconcile.rs:1162-1188`; single verbs `:41/62/85/110/1136`;
append `:28`; bulk `apply_*` `:286/395/438`. Resolver: `ClassifyInbound` `resolve.rs:694-709`,
`ReclassifyOutflow` `:746-762`, `ReclassifyIncome` `:807-821`, **`ClassifyRaw` pass 1c first-wins `:543-560`**;
effective payload `applied.get(target).unwrap_or(&raw.payload)` `:728-730/789-791`, ManualFmv pass 1d
`:575-577`; **`ManualFmv` last-wins no-conflict `:564-568/593-597`**; `pseudo approve` ClassifyRaw shape `:223`;
void revocability `:423-440`; pseudo fill Phase A/B/C `:934-953/966-976/1008-1021`; real-before-synthetic
`:563-780/:939-941`; `session.project()` stored pseudo cfg `session.rs:556-562`. Sign sites: `parse_usd_arg`
`eventref.rs:77-79`, `parse_sell_arg` `whatif.rs:481-488` (both no guard); negative-reject precedent
`main.rs:126-139`; flag sites `main.rs:224/247/305/324/347-353/400/421-427/852-907/965/982/988/1026/1027/1031/998`;
already-guarded `main.rs:887/890/895/898/905/908`. EIN/TIN verbatim `main.rs:1095-1106`; help `cli.rs:646/653/
656-658`; TUI-edit form `tui-edit/src/edit/form.rs:1328-1420`.

**§9.4 UX-P4-9.** core `whatif.rs`: `NoLots` `:131/137`, raise `:234-236`, `HarvestStatus::NoLots` `:516/694`,
`InvalidTarget→NoLots` `:530/534`; cli map `cmd/whatif.rs:170-172`.

**§9.5 UX-P4-8.** `CliError::Io` `cli/lib.rs:44-45`, `StoreError::Io` `store/lib.rs:19-20`; vault-open
`session.rs:390-394` → `Vault::open` `vault.rs:117/129`; export-out `admin.rs:82/113`, `export_snapshot`
`vault.rs:263-271`, `write_csv_exports` `render.rs:586/593/595/605-618`; precedent
`AdapterError::Io{path,source}` `adapters/lib.rs:23-28`/`read.rs:63-66`.

**§9.6 M-1.** fingerprints hand-rolled bytes `persistence.rs:25-55`; persisted event bytes = TYPED serde
(`persistence.rs:164-165` `to_string(&WalletId)`/`to_string(&EventPayload)`, field-ordered regardless).
`Value`-OUTPUT sites (the true blast radius, corrected per impl-review r1 — the original list omitted the
oracle-harness) = **income-show display** (`btctax-cli/src/cmd/tax.rs`, never parsed) + **oracle-harness
`json!`→stdout** (`btctax-oracle-harness/src/main.rs`, displayed/re-parsed, never stored/hashed) +
**input-form coverage tooling** (`btctax-input-form/src/spec/coverage.rs`, `#[cfg(test)]`-gated);
update-prices is PARSE-only (`from_str`, constructs no output `Value`); `btctax-forms`/`xtask`
serde_json-free. Pinned by the `m1_preserve_order_value_output_sites_are_enumerated` scan tripwire.
