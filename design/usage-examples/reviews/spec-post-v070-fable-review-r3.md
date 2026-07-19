# Independent spec re-review — SPEC_post_v070_product_cycle.md (r3, FINAL-candidate)

**Reviewer:** Fable (independent; reviewed r1 and r2 across both lenses; did not author the spec or any
fold). **Charters merged this pass:** (general) design soundness, completeness, KAT genuineness, phasing;
(tax-correctness) the §1 math-path invariant, answered-ness false-negatives on the pseudo surfaces,
valid-return-blocking refusals.
**Date:** 2026-07-18. **Branch:** `feat/post-v070-product-cycle`.
**Artifact:** `design/usage-examples/SPEC_post_v070_product_cycle.md` @ r3 (`d49dea4`).
**Inputs:** the r3 spec (all sections, all `[G2-*]`/`[T2-*]` fold tags); both r2 re-reviews
(`spec-post-v070-fable-review-r2.md` NEW-1..NEW-8; `spec-post-v070-fable-taxcorrectness-r2.md`
R2-I1, R2-M1..3, R2-N1..3); source re-verified at every anchor an r3 fold leans on:
`btctax-cli/src/main.rs` (exit map :38-45, verify :112-118, Report arm :140-182, write-carryover call
:179-181, optimize :220-250, what-if sell :300-355, harvest :396-430, tax-profile :850-912, classify
arms :960-1035), `cli.rs` (TaxProfile :237-300, `--sell` args :335/:412), `cmd/tax.rs` (:60-102,
:276-310, :425-443, :444-517), `cmd/reconcile.rs` (fn map, :284-330, :1130-1190), `render.rs` (:55-64,
:1014-1062, :1170-1250), `resolve.rs` (ladder :85-135, `placeholder_tax_profile` :58-70,
`Provenance` :25-33), `session.rs` (:488-520, :554-564), `eventref.rs` (:75-80),
core `project/resolve.rs` (PseudoKind :215-230, void revocability :420-442, conflict resolution
:495-542, pass 1c :543-560, pass 1d :562-600, passes 1e/ClassifyInbound/ReclassifyOutflow/
ReclassifyIncome :690-825, pseudo Phases A/B :930-980, `applied` writers via grep), core `state.rs`
(:275-290), core `whatif.rs` (:128-145, :228-246, :478-490, :512-536, :690-698), core
`project/fold.rs` (:394-409 advisory gate), `btctax-tui/src/tabs/tax.rs` (:14-30, :55-122).

---

## Part 1 — Resolution of the r2 blocking findings

| r2 ID | Verdict | Basis |
|-------|---------|-------|
| NEW-1 (sign-table site completeness) | **RESOLVED** | The §3.3(a) table now carries all three demanded extensions, each anchor-accurate against source. (i) **`--sell` row** (`main.rs:305` what-if sell, `:224` optimize consult — both verified as `parse_sell_arg` call sites) with **refuse ≤ 0** and the downstream-fiction rationale (pool check `whatif.rs:234` cannot fire against a negative; proceeds go negative `:242` — re-verified; `parse_sell_arg`'s own doc-comment at `whatif.rs:475-478` confirms "sign PASSED THROUGH … NO sat-side sign check" on the integer path). (ii) **Ad-hoc trio row** (`--income`/`--magi`/`--carryforward-in`, `main.rs:347-353` and `:421-427` — verified: the six `parse_usd_arg` sites building `AdhocProfile`) with `--carryforward-in` refuse < 0 (correct — it is a loss magnitude) and `--income`/`--magi` deferred to the PLAN per the tax-profile posture, the same per-field deferral r2 accepted for the tax-profile row. (iii) The tax-profile row's range extended to `:852-907` with the **already-guarded** trio cited for exclusion from re-work — verified: `w2_ss.is_sign_negative()` guard at `main.rs:890`, `w2_medicare` at `:898`, `sce` at `:908`, exactly as cited. The harvest `Gain/Tax` negative target correctly needs no row (`InvalidTarget`, re-confirmed `whatif.rs:140-142`). One residual on the KAT's `--sell -1` spelling — new **R3-N1** (Nit), not a gap in the table itself. |
| NEW-2 (channel-aware banner text) | **RESOLVED** | §3.1 now pins two texts. The **placeholder variant** is true clause-by-clause on its channel: "synthetic $0 placeholder profile" — verified, `placeholder_tax_profile()` is all-$0 Single (`btctax-cli/src/resolve.rs:58-70`); "no tax profile or full-return inputs are stored for this year" — exactly the ladder condition for `Provenance::PseudoPlaceholder` (arms 1–2 empty, `resolve.rs:110-128`); the `income import` remedy is a real subcommand and "turn pseudo mode off" is real (`pseudo_set_mode`, `reconcile.rs:168`). The **synthetic variant** keeps the r2-verified text, and its pointers are live on that channel (rows via `pseudo_tag`, `render.rs:57-64`; the count-gated `[PseudoReconcileActive]` advisory, `fold.rs:396-407`). **KAT (b) now asserts the placeholder-variant wording** (spec:107-109) — the demanded correct-text clause is present and reds if an implementer ships the synthetic text on the placeholder channel. Two residuals, both new Minors: the `tax-profile --set` pointer names a nonexistent flag (**R3-M2**) and the channel-selection rule in the overlap state is implicit (**R3-M1**). Neither can produce a *false* banner (see Part 2), so NEW-2's own bar — every clause true for the channel that fires it — is met. |
| NEW-3 (set-fmv exempt from DUPLICATE only) | **RESOLVED** | §3.2 now pins exactly the demanded sentence: "`set-fmv` is exempt from the **DUPLICATE** refusal ONLY … still gets existence/type validation like every verb," with the correct engine rationale on both halves — last-wins-no-conflict re-verified at `resolve.rs:566-568` ("NO duplicate blocker … a correction flow, not a conflict") and the refuse half re-verified at pass 1d: an unknown target → Hard `DecisionConflict`, EXCLUDED (`:575-590`); a wrong-type target → Hard `DecisionConflict`, EXCLUDED (`:592-600`) — so record-time refusal of `set-fmv <bad-ref>` is exactly mirror-consistent. **The KAT's refuse direction is updated**: `set-fmv <bad-ref>` is now an enumerated refuse case (spec:158-159), and the accept direction keeps "second `set-fmv` on a valid target." The original UX-P4-3 trap no longer survives on the verb that feeds ordinary income. |
| NEW-4 (write-carryover refuses on NotComputable) | **RESOLVED** | §3.1 clause 4 is now a dual gate: (4a) `pseudo_contributed`, (4b) **delta outcome `NotComputable`** — refuse fail-closed (nonzero, persist nothing) *before* `apply_carryover_writeback` (site verified: `tax.rs:507-510`; persistence only at `s.save()`, an `Err` discards the in-memory mutation). The factual predicate for 4b re-verified end-to-end: `write_back_carryover` (`tax.rs:444-517`) has no delta-outcome or `state.blockers` gate today — its screens are the resolver's input/compute-dependent screens plus `screen_absolute` (QBI/AMT/TI≤0), none of which consult hard ledger blockers — so the clause closes a real hole. **KAT (e) is present** (hard-blocked **non-pseudo** vault + `--write-carryover` → nonzero AND year+1 `ReturnInputs` byte-identical), correctly isolating 4b from 4a, and the mutation clause covers "either write-carryover refuse branch." §3.5's ordering question genuinely dissolves: in the Report arm (`main.rs:140-182`) the write-carryover block precedes any exit-1 return, and a clause-4 refusal propagates as `Err` → exit 2 (`run_to_exit`, `main.rs:38-45`) before the `ExitCode::from(1)` placement is ever reached — "nonzero, persists nothing, before any exit-1 return" is accurate. |
| R2-I1 (effective-payload view, not raw-log) | **RESOLVED as prescribed** — but the prescription itself was one channel short; see **R3-I1**. | §3.2 now pins existence/type validation against "the EFFECTIVE payload under live real decisions — the raw event log folded with void-folded real `ClassifyRaw` rewrites (synthetics excluded), NOT the raw log alone," names the resolver's keying (`applied.get(target).unwrap_or(&raw.payload)` — re-verified at `:577-578`, `:729-730`, `:790-791`), pins the ordering fact (pass 1c real `ClassifyRaw` populates `applied` *before* pseudo Phase A — re-verified, `:543-560` vs `:934+`), and mandates the shadow-projection (pseudo forced OFF), never the tainted `session.project()` (stored-cfg taint re-verified, `session.rs:554-564`). **The ClassifyRaw'd-target success KAT is present and mirrors the resolver**: accept `set-fmv`/`reclassify-income` on a target whose Income type comes from a live real `ClassifyRaw`; the same target with that `ClassifyRaw` *voided* → refused wrong-type (spec:160-161) — both adjudications confirmed against passes 1c/1d/1e. The sanctioned post-`pseudo approve` correction (`ClassifyRaw` zero-value placeholder shape confirmed, `PseudoKind::RawInbound` doc, core `resolve.rs:222-224`) is no longer falsely refused. What the r2 fix text did not know: `applied` has a **third real writer** — accepted conflicts (`SupersedeImport`) at `resolve.rs:513` — so the pinned parenthetical definition still admits one divergence channel. That is a new third-order finding (R3-I1), not a failure to fold R2-I1, whose demanded text and KAT were adopted verbatim. |

**Folded Minors/Nits — spot-check (all verified in place):**
- `[G2-6, T2-M1]` **ClassifyRaw verb**: added to the first-wins list with the pass-1c cite and the CLI
  name (`reconcile classify-raw` — arm confirmed in `main.rs`); the bulk-paths scoping sentence is
  present with the intra-batch-adjudication rationale. Residual: the choke-point line-list omits the
  `classify_raw` append fn — **R3-M3**.
- `[G2-5, T2-M2]` **TUI enumeration invariant**: stated in clause 3 with the trip-wire sentence, and
  **KAT (f)** pins NOT COMPUTABLE. Re-verified: `resolve_all_screened` enumerates
  `tax_profile::years ∪ return_inputs::years` only (`session.rs:498-499`), so a `PseudoPlaceholder`
  profile cannot reach `snap.profiles`; the tab renders a reason, never a number
  (`tabs/tax.rs:59-70`).
- `[T2-N3]` **bool rename**: the threaded bool is pinned as `pseudo_contributed` with the
  drop-the-OR rationale — the name can no longer argue with the predicate.
- `[T2-N1]` **event-date close**: §3.3(d) now says "use the event-date close, not a 'recent' close,"
  with the 26 CFR §1.170A-1(c)(2) contribution-date grounding. Correct law, correct direction.
- `[T2-N2]` hyphenless-EIN ambiguity note present ("do NOT 'harden' it into a false refuse");
  `[G2-7]` KAT (a) restated as two clauses (committed golden byte-identical + the NEW pseudo-fixture
  banner-only-insertion clause); `[G2-8]` §3.5 cross-references clause 4a so the exit-0 non-trigger
  and KAT (d) cannot read as conflicting. All folded faithfully.

## Part 2 — Final adversarial sweep (third-order hunt)

The charter's five named probes, answered:

**(1) Can `pseudo_active()` AND `PseudoPlaceholder` co-occur, and which text wins?** Yes — the overlap
is real and is in fact the most common novice pseudo state: pseudo on, unreconciled rows (synthetics →
`pseudo_active()`), nothing stored for the year (ladder arm 3 → `PseudoPlaceholder`). The spec's
channels are disjoint only via the placeholder channel's "`count == 0`" conjunct, and "count" is used
ambiguously (see R3-M1). Decisive for severity: **no selectable reading renders a FALSE banner.** If
the synthetic text fires in the overlap, every clause is true (rows exist; the count-gated advisory is
present, `fold.rs:396-407`); if the placeholder text fires, every clause is also true ("no tax profile
or full-return inputs are stored" holds — that is what put it on arm 3; the $0 placeholder is in use).
Each variant is merely *incomplete* about the other channel's remedy, and the residue fail-closes:
following either remedy path ends at the other banner or at a loud NOT COMPUTABLE. Minor, not
blocking (R3-M1 pins the precedence sentence).

**(2) Does the write-carryover dual gate have a gap or a double-refuse oddity?** No gap found. Channel
audit re-run on the r3 shape: non-`ReturnInputs` provenance → pre-existing gate (`tax.rs:478-483`);
pseudo taint → 4a (and at this site `pseudo_active()` is exactly the taint feeding
`assemble_absolute(&ri, &state, …)`, `tax.rs:486`); hard-blocked ledger → 4b; absolute-side refusals →
`screen_absolute` (pre-existing); the derivative `income import` preserve channel (`tax.rs:66-100`,
re-verified) stays transitively closed — it can only preserve a `Computed` carryover the gated
write-back produced. Double-fire (pseudo-active AND hard-blocked — reachable, e.g. a
`DecisionConflict` pseudo never clears) is benign: both branches refuse fail-closed with a true
message; which fires first is a PLAN detail with no soundness content. 4b is implementable at the
site (`state` is in scope from `load_events_and_project`, `tax.rs:456`).

**(3) Does "refuse ≤ 0 for `--sell`" wrongly block any legitimate use?** No. The surface is
planning-only (`what-if`/`optimize` never persist, never touch a filed form — no §1 contact); a
zero-sat sell is a degenerate no-op with no answer value (baseline comes from `report`); the
BTC-decimal path *already* refuses negatives (`parse_sell_arg` doc, `whatif.rs:475-480`), so the
policy merely unifies the integer-sat path with existing behavior; and the harvest `Gain(X)/Tax(X)`
negative targets remain separately (correctly) guarded via `InvalidTarget`. No legitimate
zero/negative use exists to block.

**(4) Does the effective-payload view definition still admit resolver divergence?** **Yes — one
channel: real accepted conflicts.** This is R3-I1 below.

**(5) Is any KAT still non-red under mutation?** One spelling defect: the §3.3 KAT's literal
"`--sell -1`" is rejected by clap *before* the new guard exists (no `allow_hyphen_values`/
`allow_negative_numbers` anywhere in `cli.rs`; the arg is a plain `String` option), so a KAT invoking
that exact form exits nonzero with or without the fix — non-red under mutation. R3-N1. (The
NEW-1-demanded `=`-form is the reachable bypass and must be the KAT's spelling.) All other KATs
re-checked red-capable: (a)(i)/(a)(ii) split correctly separates the empty-diff guard from the
banner-insertion witness; (b) reds a wrong-variant or dropped-OR mutation; (d)/(e) red on either
refuse branch's removal (byte-identical year+1 file check); (f) reds a future placeholder-estimate
path in the TUI; the §3.2 KAT's voided-ClassifyRaw→refuse case reds a validator that ignores void
folding.

**Checked and explicitly clean** (so a further pass can skip them): the §3.5 exit-map story against
`run_to_exit` (`main.rs:38-45`) and the `verify` precedent (`:112-118`); the dual-report
placeholder-inert parenthetical (`tax.rs:306` provenance gate, re-verified); clause 4a's
structural-inertness claim (`tax.rs:478-483`); the §3.4 message fields all in scope at
`whatif.rs:234-236` with `HarvestStatus::of_refusal` mapping `InvalidTarget → NoLots` (`:528-536`);
the ad-hoc trio's `--carryforward-in` is genuinely a loss magnitude (positive-in convention, matching
the tax-profile carryforward fields); §9's r3-extended anchors spot-verified accurate
(`:277/:282` state, `:507-509` persist, `:584-650` `screen_compute_dependent` named correctly as
blocker-blind, `:543-560/:575-577` passes, `:997-1000` Phase B effective-payload keying,
already-guarded trio `:890/:898/:908`); phasing (§7) unchanged and consistent with §1 ownership —
KATs (e)/(f) land inside phase-1-owned items, adding no new phase weight; §5/§6 unchanged from the
r2-clean state. One out-of-enumerated-scope observation, recorded without a finding: `accept-conflict`
on a target governed by a live real `ClassifyRaw` retroactively conflicts that earlier decision
(conflict application at `resolve.rs:513` runs before pass 1c and inserts unconditionally) — the
record-then-conflict shape on a verb §3.2 never claimed to validate. Edge-of-edge, arguably
informative rather than wrong (the accept is the user's explicit instruction and the resulting
blocker is loud); the PLAN may note it, no spec change required.

## Part 3 — New findings

### R3-I1 (Important, class C + UX-P4-3 trap): the effective-payload view omits the accepted-conflict (`SupersedeImport`) channel of the resolver's `applied` map

§3.2 pins the view as "the raw event log **folded with void-folded real `ClassifyRaw` rewrites**
(synthetics excluded)." But the resolver's `applied` map — the exact thing
`applied.get(target).unwrap_or(&raw.payload)` consults — has **three** writers, and two of them are
real (verified, core `project/resolve.rs`):

1. `:513` — a **real accepted conflict**: `SupersedeImport` resolution inserts the conflict's
   `new_payload` onto the target, unconditionally, *before* pass 1c;
2. `:522` — pseudo accept-first (synthetic; correctly excluded under the pseudo-OFF view);
3. `:558` — pass 1c real `ClassifyRaw` (the only real channel the spec's definition names).

Nothing constrains an `ImportConflict`'s `new_payload` to preserve the original payload's *type* — a
type-changing re-import (e.g. an adapter update that decodes a formerly-`Unclassified`/`TransferIn`
row as `Income`) is the documented reason conflicts exist, and `reconcile accept-conflict`
(`reconcile.rs:328`) is a first-class verb. Two divergences follow from the pinned definition:

- **False refuse (the R2-I1 class, one channel over):** `set-fmv` / `reclassify-income` on a target
  whose Income type comes from a **real accepted conflict** is honored by the resolver (passes 1d/1e
  see `applied[target] = Income`) but refused wrong-type by a validator built to the pinned
  definition (raw + ClassifyRaw only sees the pre-supersede payload). A valid-return-blocking false
  refuse — the harm class §1's `[G-§1]` amendment elevates to invariant-harm.
- **Missed refuse (the original UX-P4-3 trap, on the verb r3 just added):** `classify-raw` on a
  target already governed by a real accepted conflict IS adjudicated by the resolver as a NEW
  `DecisionConflict` (pass 1c keys its duplicate check on `applied.contains_key`, `:551` — which
  includes the `:513` entries), but a record-time duplicate check that consults only prior
  `ClassifyRaw` *decisions* accepts it exit-0, detonating at the next `verify`.

The master predicate ("refuse iff the resolver would adjudicate the append as a NEW
`DecisionConflict`") and the shadow-projection mechanism are both correct and would capture all of
this — but the mandate equally blesses a "shared helper," a helper built to the pinned parenthetical
diverges, and **no KAT exercises either accept-conflict case**, so nothing reds. This is precisely
the internal-inconsistency shape R2-I1 was graded Important for: one bullet mandates mirroring, the
definitional bullet does not mirror, and the KAT set cannot arbitrate.

**Required fix (textual + KAT):** redefine the view as the resolver's `applied` under pseudo-OFF —
"the raw event log folded with **all live real payload-overriding decisions**: void-folded real
`ClassifyRaw` rewrites (`resolve.rs:543-560`) AND accepted-conflict `SupersedeImport` payload
applications (`resolve.rs:508-513`); synthetics excluded" — and extend the §3.2 KAT: **accept**
`set-fmv`/`reclassify-income` on a target whose Income type comes from a real accepted conflict;
**refuse** `classify-raw` on a target a real accepted conflict already governs (duplicate). (No
voided-variant case exists here: `SupersedeImport` is non-revocable, `resolve.rs:423-440`.)
Mutation reds.

### R3-M1 (Minor): the banner channel-selection rule is implicit and "count" is used with two meanings

The two banner channels overlap (see Part 2 probe 1). Disjointness hangs on the placeholder channel's
"`count == 0`" conjunct, but §3.1 uses "count" both for the ladder's stored-profile emptiness
("injects … when `cfg.pseudo_reconcile` is on and nothing is stored (`count == 0`)" — factually
loose: the inject at `resolve.rs:120-128` is provenance-ladder-only, with no count condition) and,
in the channel/KAT (b) usage, necessarily for `pseudo_synthetic_count` (`state.rs:277`) — the only
reading under which the channels are disjoint and KAT (b) pins the pure-placeholder state. No reading
yields a false banner, so Minor: pin `count` = `pseudo_synthetic_count`, state the precedence in one
sentence ("the synthetic text wins when both channels hold; the placeholder variant is the
`count == 0` else-arm"), and tighten the inject-description clause.

### R3-M2 (Minor): the placeholder-variant remedy pointer names a nonexistent flag

The pinned wording says "Set a tax profile (`btctax tax-profile --set …`)" — but the `TaxProfile`
subcommand has **no `--set` flag**: setting is the default action and `--show` inverts it (verified,
`cli.rs:237-300`; the flag set is `--year`, the money/status fields, `--show`, `--force`). The spec's
own claim that "the remedy pointers are live for the channel that fires them" is false for this
pointer, and KAT (b) — which asserts the placeholder-variant wording — would pin the dead spelling
into the shipped banner, on exactly the surface a no-profile novice reads. One-token fix (e.g.
"`btctax tax-profile --year <Y> …`"). Consequence is self-correcting (clap errors with usage), hence
Minor, but it must not survive the fold.

### R3-M3 (Minor): the §3.2 choke-point enumeration omits `classify_raw`

The pinned choke list "`reconcile.rs:41/62/85/110/1136`" resolves to `classify_inbound` /
`reclassify_outflow` / `set_fmv` / `void` / `reclassify_income` (verified against the fn map) — it
**omits `classify_raw` (`reconcile.rs:301`)**, the append fn of the very verb the same fold added to
the first-wins refusal list. The KAT's refuse direction says only the generic "first-wins duplicate"
(no per-verb enumeration), so a PLAN that wires validation into the five listed fns and KATs one
duplicate verb goes green with `classify-raw` unvalidated — the coverage gap `[G2-6]` was filed to
close, reopened by a line-list. Fix: add `:301` to the choke list and name `classify-raw` in the
KAT's duplicate case (or require a per-verb duplicate KAT).

### R3-N1 (Nit): the `--sell` negative KAT must use the `=`-form and assert the message

§3.3's KAT writes "`--sell -1` refused." With no `allow_hyphen_values`/`allow_negative_numbers` on
the arg (verified: none in `cli.rs`), clap rejects the space form as an unknown flag *before* the
guard exists — the KAT as spelled is green without the fix and non-red under mutation (the
untested-guard failure mode). Spell it `--sell=-1` (the bypass NEW-1 itself identified) and assert
the specified refusal message, exactly as the negative-basis clause already does for its `=` form.

## Part 4 — Verdict

All five r2 blocking findings are genuinely folded: NEW-1, NEW-2, NEW-3, and NEW-4 **RESOLVED**;
R2-I1 **RESOLVED as prescribed**, with the discovery that the prescription itself (adopted verbatim)
was one real channel short of the resolver's `applied` construction. The spine — four-surface
disclosure with a dual fail-closed persistence gate, resolver-mirroring record-time validation, the
completed sign table, and the resequenced discovery verb — is sound and fully anchored. The one new
blocker is a two-sentence-plus-KAT re-pin of the effective-payload view to include accepted-conflict
payload applications; the three Minors and the Nit are one-line fixes in its blast radius.

**VERDICT: 0 Critical / 1 Important (R3-I1) / 3 Minor (R3-M1, R3-M2, R3-M3) / 1 Nit (R3-N1) — NOT
GREEN. Fold R3-I1 (effective view = the resolver's pseudo-OFF `applied`: real `ClassifyRaw` AND real
`SupersedeImport` applications; + the two accept-conflict KAT cases) and re-review; none of this
threatens the r3 architecture.**
