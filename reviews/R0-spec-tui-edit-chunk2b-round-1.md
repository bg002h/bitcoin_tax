# R0 architect review — SPEC_tui_edit_chunk2b (round 1)

**Artifact:** `design/SPEC_tui_edit_chunk2b.md` (reclassify-income + set-fmv + void in `btctax-tui-edit`)
**Baseline verified against:** working tree @ `fe726ff` (all cited files re-read at review time)
**Reviewer:** independent R0 (architect), per `STANDARD_WORKFLOW.md` §2
**Verdict:** **0 Critical / 4 Important / 5 Minor / 4 Nit — NOT green.** Implementation is gated
until the Important findings are folded and re-reviewed.

Void is the highest-risk flow in the program to date (it un-projects prior decisions). The
write-path design (D4) is correct and mirrors the CLI void verbatim, the revocable set is exact,
and the round-trip E2E is sound. The blocking findings are all on the *consequence-surfacing and
test-truth* side: an unstated dependent-decision cascade, a dishonest conflict-arm status string,
a NIIT claim in an E2E that is false against current source, and a spec-internal contradiction on
Esc semantics that the KATs then pin.

---

## 1. Source verification summary (spec claims vs HEAD `fe726ff`)

| Spec claim | Source | Verdict |
|---|---|---|
| Revocable set: TransferLink, ReclassifyOutflow, ClassifyInbound, ManualFmv, ClassifyRaw, MethodElection, LotSelection, ReclassifyIncome; SafeHarborAllocation conditional; SupersedeImport/RejectImport/VoidDecisionEvent non-revocable | resolve.rs:301–341 (comment 301–306; non-revocable arm 312–321; SafeHarbor arm 322–328; catch-all insert 329–331; unknown-target arm 332–338) | **EXACT** |
| Void-of-void → immediate DecisionConflict | resolve.rs:314 (match arm), 316–320 (blocker) | **CONFIRMED** |
| Re-void of already-voided decision: idempotent, no conflict | resolve.rs:330 `BTreeSet::insert` (spec cites 329 — 1-line drift) | **CONFIRMED** |
| SafeHarbor void adjudication: effective → DecisionConflict attributed to the VOID id; inert → no conflict | resolve.rs:924–934 (`v.void_id` at 930) | **CONFIRMED** |
| ReclassifyIncome: effective-payload validation + FIRST-WINS, conflict on 2nd non-voided | resolve.rs:635–694 (dup arm 665–673; insert 674–676; map decl 484) | **EXACT** |
| ManualFmv: latest-seq-wins, NO duplicate blocker; target-absent / non-Income → Hard DecisionConflict, EXCLUDED | resolve.rs:423–474 (note 427–428; None arm 440–452; Income insert 453–456; Some(_) 458–471) | **EXACT** |
| ClassifyInbound FIRST-WINS (round-trip step 5 cite) | resolve.rs:554–563 | **EXACT** |
| Voided decisions skipped in every collection pass (re-classify after void is clean) | resolve.rs:361, 407, 431, 487, 738, 781, 847 | **CONFIRMED** |
| FmvMissing fire paths (4 arms) | fold.rs:651–656, 672–677, 833–839, 854–859 | **EXACT** |
| CLI `--business` required-explicit | btctax-cli/main.rs:301–302 (`required = true, ArgAction::Set`) | **EXACT** |
| CLI void + LotSelection `optimize_attest::clear` atomic batch | reconcile.rs:108–147; `clear(conn, &EventId)` optimize_attest.rs:86; `pub mod optimize_attest` lib.rs:9 | **EXACT** (D4 mirrors it verbatim, incl. `load_all`) |
| `persist_only_tokens` incl. `"append_"` | tui-edit persist.rs:577 | **EXACT** |
| Old remedy strings (4 arms) | tui-edit main.rs:1346–1350, 1363–1371, 1382–1386, 1417–1421 | **EXACT** (all four match verbatim) |
| 2a substrate names (FieldBuffer, TargetList, list items, flow/modal structs, persist fns, `events_by_id`, dispatch layout 1–6) | form.rs:17/24, 208, 265, 282, 303, 332, 339, 466, 492, 499; persist.rs:36/60/99; main.rs:92–181, 1171–1176 | **CONFIRMED** |
| Browse key bindings free for `r`/`f`/`v` | main.rs:153–179 (Browse binds q/Esc, Tab/BackTab, k/j, PgUp/PgDn, g/G, ←/→, p/c/o). The existing `'r'` at main.rs:147 is the **Locked**-screen retry — different screen arm, no collision | **CONFIRMED** |
| 2a inbound-list pre-filter consults voided set (needed for round-trip step 4 "event back in `c` list") | main.rs:1186–1270 (`open_classify_inbound_flow`, voided set + `already_classified`) | **CONFIRMED** |
| `IncomeKind` cycle Mining→Staking→Interest→Airdrop→Reward | event.rs:29–35 | **EXACT** |
| Payload shapes (`ReclassifyIncome{income_event,business,kind:Option}` serde-default, `ManualFmv{event,usd_fmv}`, `VoidDecisionEvent{target_event_id}`) | event.rs:141–144, 184–186, 206–214 | **EXACT** |
| `append_decision(conn, payload, now, tz, None)` signature | persistence.rs:238–245 | **EXACT** |
| KAT-V-FMV-3 premise (whitespace-only → parse error, not "required") | form.rs:57–59, 182–188 ([R0-M4] pin) | **CONFIRMED** |
| KAT-C1 at "main.rs:972–1096" | **WRONG** — `kat_c1_*` at main.rs:2099–2172; 972–1096 holds the reclassify-outflow key handlers | drift (N1) |

---

## 2. Findings

### Important

**[I1] The dependent-decision cascade is neither stated, KAT'd, nor excluded — void's
consequence surfacing is incomplete.** (D3.1 modal note; `derive_void_status`; §D5)

Verified at HEAD: pass-1d ManualFmv and pass-1e ReclassifyIncome validate the **effective**
payload (`applied.get(target).unwrap_or(&raw.payload)`, resolve.rs:436–438, 644–646). Voiding a
`ClassifyRaw` whose target's effective payload had become `Income` therefore **orphans** every
non-voided `ManualFmv` (resolve.rs:458–471) and `ReclassifyIncome` (resolve.rs:678–692) targeting
that event: on the next projection each orphan fires a **Hard `DecisionConflict` attributed to the
orphaned decision's id — not the void's** — and gates `compute_tax_year`. The same shape exists via
lots: voiding a `ClassifyInbound`/`ClassifyRaw` that created a lot picked by a `LotSelection` makes
the fold fire `LotSelectionInvalid`.

This is reachable from the TUI: `ClassifyRaw` is in the void list (Claim E), and a mixed CLI+TUI
vault can hold dependent `ManualFmv`/`ReclassifyIncome` decisions (the CLI creates them freely; the
TUI's own set-fmv/reclassify-income raw-`Income` filters only constrain TUI-created ones).

Consequences in the spec as written:
- The modal note says only "Prior blockers may return" — a returned blocker is materially
  different from a **new Hard conflict on a different, non-voided decision** whose remedy is a
  *second* void.
- `derive_void_status` checks only blockers attributed to `void_decision_id`, so the cascade case
  reports the **clean** string; the generic "check Compliance for any returned blockers" tail is
  the sole surfacing, and it names the wrong category.

**Fix (all three):** (a) extend the D3.1 consequence note: "…prior blockers may return, and
decisions that depended on this one (e.g. a ManualFmv or ReclassifyIncome on a ClassifyRaw'd
event, or a LotSelection picking its lots) may now fire DecisionConflict/LotSelectionInvalid —
void those too"; (b) either add **KAT-E2E-VOID-CASCADE** (seed Unclassified → ClassifyRaw→Income →
ManualFmv; TUI-void the ClassifyRaw; assert a Hard DecisionConflict attributed to the ManualFmv
decision id and that the ManualFmv now appears in the `v` list as the remedy) **or** exclude the
cascade explicitly in scope + FOLLOWUPS with the interim answer ("Compliance tab shows the orphan;
void it"); (c) state in D3.1 that the clean-arm status cannot detect cascade conflicts (they are
attributed elsewhere) so the reader of the spec knows the limit is deliberate.

**[I2] The void conflict-arm status string claims success when the void did NOT take effect.**
(D3.1, `derive_void_status`)

The string `"Voided decision|{seq} — but DecisionConflict fired on the void: see Compliance"`
fires exactly when the void was **rejected**: void-of-effective-SafeHarborAllocation
(resolve.rs:926–933 — "irrevocable, §7.4"; the allocation **stays in force**) and
void-targets-unknown (resolve.rs:332–338). Leading with "Voided" asserts the decision was revoked;
it was not. Additionally `{seq}` is the *target's* seq while the conflict is attributed to the
*void* decision — mixed identities in one line. On the highest-risk flow, the failure-mode status
must be unambiguous.

**Fix:** reword the conflict arm to e.g. `"Void saved, but DecisionConflict fired — the target
decision remains in force (see Compliance)"`, and add a cheap **unit KAT for this arm** with a
synthetic snapshot carrying a `DecisionConflict` blocker attributed to the void id (no Path-B
vault fixture needed — the FOLLOWUPS deferral of the effective-allocation *E2E* can stand; the
*string arm* itself is trivially testable now and currently has zero coverage).

**[I3] KAT-E2E-RI-SE's NIIT premise is false against current source — the NIIT half of the E2E
is vacuous as specified.** (D5)

Verified at HEAD: NII = capital-gain components + `interest_nii`, where `interest_nii` filters
`kind == IncomeKind::Interest` **only** (compute.rs:306–309, 352–354; the `business` flag is not
consulted). SE eligibility = `business && kind != Interest` (se.rs:56–62). Therefore the specified
fixture — `Reward, business=false` reclassified to `Mining, business=true` — moves SE from `None`
to `Some(nonzero)` (that half is sound and non-vacuous) but **cannot move NIIT at all**: neither
Reward nor Mining is ever NII, and MAGI is unchanged by the flip. The spec's step 1 ("NIIT
computed on the FMV") and step 2 ("NIIT on Mining income moves … NIIT component may decrease")
are wrong, and "may decrease" is not an assertable predicate in any case.

**Fix (either):** (a) rescope the KAT to SE-only and **pin the non-effect** — assert the NIIT
delta is *unchanged* by the Reward→Mining flip (that is the true, KAT-worthy fact); or (b) add a
second fixture where the kind flip involves `Interest` (e.g. `Interest, business=*` →
`Mining, business=true`): Interest leaves NII and Mining enters SE, so **both** figures move —
assert exact before/after values (the engine is deterministic; "exact figures belong in core KATs"
does not excuse a directionally-false claim here). The modal's generic "SE/NIIT exposure may
change" note survives either way (Interest-involving flips do move NIIT) but D5's fixture text
must be corrected.

**[I4] D1/D2 Esc semantics contradict the spec's own inherited hard constraint, and KAT-C2c/C2d
pin the deviation.** (Hard constraints §; D1 step 2; D2 step 2; KAT-C2c/C2d)

The hard-constraints section inherits the 2a substrate "verbatim," including **Esc-steps-back**.
Verified 2a behavior at HEAD: every step steps back exactly one step per press — IncomeForm →
VariantPicker (main.rs:975–989), KindPicker → List (1013–1017), FieldForm → KindPicker
(1068–1075), List → close (510–513, 974–977). D1 and D2 instead specify FieldForm `Esc` → **close
the whole flow**, with the rationale "no picker step to step back to" — false: the **List step
exists** and is the one-step-back target. KAT-C2c/C2d then pin the deviating sequence ("`Esc` →
modal closes (form still open); `Esc` → flow closes"). The two sections cannot both be
implemented; cancel semantics on mutating flows are safety-adjacent and must be unambiguous
before TDD-red.

**Fix:** FieldForm `Esc` → back to `List` (matching 2a), and update KAT-C2c/C2d to the
three-stage sequence (modal-Esc → form; form-Esc → list; list-Esc → close; bytes unchanged
throughout). If the author instead wants direct-close, the hard-constraints paragraph must carve
an explicit, justified exception — but consistency across six flows favors step-back.

### Minor

**[M1] Void's save-error UI state is unspecified, and the void retry contract has zero test
coverage.** D1's Err-arm ("close modal, keep FieldForm open") cannot apply — void has no
FieldForm. Specify: on `Err(e)`, close `void_modal`, flow remains at `List`, status
`"Save error: {e}"`; retry = re-select → modal → Enter. The KAT-S2b sampling justification is
accepted for the chmod pattern, but D4's documented void-retry contract (+2 inert rows, target
still voided, no conflict) is currently pinned nowhere — add a cheap unit test in persist.rs
(call `persist_void` twice; assert `pre+2` rows, both `VoidDecisionEvent`, re-projection shows the
target still excluded and no new blocker). No chmod machinery needed.

**[M2] The remedy-string KAT-update mandate is inaccurate in one direction and incomplete in
another.** (a) Verified at HEAD: **none of the existing pins break** under the new strings —
KAT-S2 (main.rs:2896–2910) and KAT-S2-RO (4400–4415) pin `"DecisionConflict"` +
`"void {canonical}"`; kat_e2e_fmv_missing (3135), gift_unknown (3202), gift_price_gap (3300) pin
`contains("void")` — every pinned substring survives because the CLI path is retained. The spec
should say the updates are *strengthenings*, not break-fixes. (b) The update list omits
`kat_e2e_gift_price_gap_donor_date_outside_price_dataset` (main.rs:3256), which pins arm 3's
string just like GIFT-UNKNOWN — include it. (c) "These four tests **replace** the current KAT
assertions" (KAT-REMEDY-STRINGS) contradicts D3.2's "must also be **updated**": clarify that the
in-test E2E assertions are strengthened in place (pin both `"'v'"` and `"btctax reconcile void"`)
AND the four derive-fn unit tests are added — nothing is deleted.

**[M3] The already-voided pre-filter silently hides an in-force effective SafeHarborAllocation.**
A rejected void attempt (conflict; allocation stays in force, resolve.rs:926–933) still puts the
allocation's id in the TUI's voided set → it vanishes from the void list while NOT inert. The
Claim-E justification ("no point offering already-inert decisions") is wrong for exactly this
case. The behavior is acceptable (re-voiding only re-fires the conflict) but the spec must state
the exception, and the D3.1 SafeHarbor warning should note that a rejected void permanently
removes the allocation from this list (CLI remains available).

**[M4] Claim D's fire-path paragraph self-contradicts mid-sentence** ("the `ManualFmv` decision
will still fire a conflict at pass-1d … so no DecisionConflict"). The conclusion is correct (no
conflict; `FmvMissing` re-fires from the wallet arm, fold.rs:651–656, and the status arm surfaces
`b.detail` honestly) — rewrite the paragraph to state only the verified behavior.

**[M5] `derive_void_status(snap, &target_event_id, &void_decision_id)` never uses
`target_event_id`** in either specified arm. Either drop the parameter or use it (e.g. also check
for blockers attributed to the *voided decision's own target* to make the "returned blocker" case
concrete instead of generic). Ties to [I1]'s surfacing limit; at minimum make the signature honest.

### Nit

**[N1] Citation drift** (spec claims write-time verification; these are the misses):
`resolve.rs:329` → 330 (voided insert); `resolve.rs:429` → 427–428 (latest-wins note);
SafeHarbor arm "325–330" → 322–328; "KAT-C1 (main.rs:972–1096)" → `kat_c1_*` at 2099–2172
(972–1096 holds `handle_ro_kind_picker_key`/`handle_ro_field_form_key`); "KAT-S2 … ~2840–2910" →
fn spans 2734–2911. All others checked exact.

**[N2] KAT-C2e wording lists three Esc presses for two transitions** ("Esc → modal closes; Esc →
list; Esc → flow closes") — modal-Esc already lands on the list. Two presses after the modal.

**[N3] Claim C's snippet rebuilds the voided `BTreeSet` inside the per-event filter closure**
(O(n²)); the 2a precedent (`open_classify_inbound_flow`, main.rs:1194–1220) hoists it once. Mark
the snippet illustrative or hoist it, so the implementer copies the right shape.

**[N4] "extended to six layers" heads a nine-item list** (6 modals + flow + form + screen).
Re-label ("six modal layers" or "nine-layer dispatch").

---

## 3. Explicit answers to the gate questions

- **Revocable set exact?** Yes (see table). The resolve catch-all is nominally broader (a void of
  a raw import id also inserts into `voided`, inertly), but the spec's Decision-id filter makes
  that unreachable from the TUI and says so.
- **Already-voided pre-filter justified?** Yes — insert idempotence verified (resolve.rs:330), no
  conflict on re-void of a revocable target; the void-of-void conflict independently justifies the
  `VoidDecisionEvent` exclusion. One carve-out mis-justified: [M3].
- **Round-trip E2E sound?** Yes. Voided decisions are skipped in every collection pass
  (resolve.rs:487–489 for pass 1e), so the re-classify is the first non-voided decision — no
  FIRST-WINS conflict (resolve.rs:554–563). Step 4's "back in the `c` list" holds because the 2a
  filter consults the voided set (main.rs:1194–1220).
- **Orphaned-ManualFmv cascade?** It does **not** go inert: pass-1d re-validates the effective
  payload every projection, so a ManualFmv whose target reverts to non-Income fires a **Hard
  DecisionConflict attributed to the ManualFmv decision** (excluded from `manual_fmv`), gating
  `compute_tax_year`. Same for ReclassifyIncome. The spec must state + KAT or exclude — [I1].
- **set-fmv latest-wins / re-point?** Verified; no already-decided pre-filter is correct, and
  KAT-E2E-FMV-REPOINT pins the right thing at the right level (unit, since the list empties).
- **Remedy rework?** All four arms identified, OLD strings verbatim-exact at HEAD, new strings
  name the in-editor flow first. The KAT-update story needs [M2]'s corrections; nothing actually
  breaks.
- **Sampling justification (KAT-S2b)?** Accepted for the chmod pattern; void retry needs the
  cheap unit pin instead — [M1].
- **Scope?** Chunk 3+ exclusions and viewer/core/CLI freezes are clean; SemVer call (MINOR,
  additive) is right.

## 4. Disposition

**Blocked at the R0 gate: 4 Important.** Fold [I1]–[I4] (and ideally [M1]–[M5]) into the spec,
persist this review verbatim, and submit for round 2. No implementation (including TDD-red
scaffolding) may start until 0C/0I.

---

# Round 2 — re-review (post-fold)

**Artifact:** `design/SPEC_tui_edit_chunk2b.md` @ 1339 lines, round-1 findings folded with inline
`[I…]/[M…]/[N…]` tags.
**Verified against:** working tree @ `fe726ff`, all NEW citations re-checked at review time.
**Verdict:** **0 Critical / 0 Important / 0 Minor / 2 Nit (non-blocking) — R0 GREEN.**
Ready to implement (Tasks 1–3, TDD-red first).

## Closure verification

**[I1] CLOSED.** The void-modal consequence note now states both categories (returned blockers
AND dependent-decision conflicts, "void those too" — spec lines 676–688). The D3.1 mechanics
paragraph is source-accurate: orphan conflicts fire on the ORPHAN's own id (pass-1d/1e
effective-payload re-validation, resolve.rs:436–438/644–646, arms 458–471/678–692 — re-verified),
and the clean-arm surfacing limit is stated as deliberate with the three compensating surfaces
(modal note pre-write, Compliance tab, the orphan's own `v`-list entry). KAT-E2E-VOID-CASCADE
pins the full loop; I independently verified each step is provable at HEAD: `build_op` lets
ManualFmv win over the event FMV (resolve.rs:184–187) so step 1's FmvMissing→clear sequence
works; the post-void `Unclassified` blocker (fold.rs:1157–1163) fires on `cr.target` ==
`inner_target`, which is why step 4's "CLEAN (or returned-blocker)" parenthetical is exactly
right. The generic blockers-diff FOLLOWUP is recorded.

**[I2] CLOSED.** The conflict-arm string is now `"Void saved, but DecisionConflict fired — the
target decision remains in force (see Compliance)"` — never leads with "Voided", no `{seq}`
interpolation (the mixed-identity problem is called out inline). KAT-VOID-CONFLICT-ARM (synthetic
snapshot, no Path-B fixture) pins the wording including the NOT-starts-with-"Voided" honesty
assert; the effective-allocation E2E deferral stands in FOLLOWUPS with the string-arm now covered.

**[I3] CLOSED.** The fixture is now Interest → Mining and the exact figures are lifted from a
real, shipped core KAT — verified against
`crates/btctax-core/tests/reclassify_income.rs` at HEAD: `niit_profile()` (Single,
`magi_excluding_crypto = $205,000`, lines ~130–145) matches; the ±$380 derivation
(`kind_flip_niit_non_vacuous_…`, lines ~369–388) matches exactly (MAGI_with = $215,000 → over
$15,000 → capped $10,000 → $380.00; Interest→Mining: $380 → $0); the SE hand-derivation matches
exactly (base $9,235.00, ss $1,145.14, medicare $267.82 HALF_EVEN, addl $0, total **$1,412.96**,
deductible_half **$706.48**). The SE predicate cite (se.rs:59) is line-exact. The status stays
blocker-derived — figures asserted on the computed `TaxResult` only. The round-1 vacuity is gone:
both figures now move, in assertable exact amounts.

**[I4] CLOSED.** D1 and D2 FieldForm `Esc` → back to **List** (one step per press), matching the
2a source behavior; the false "no picker step" rationale is retracted inline. KAT-C2c/C2d now pin
the three-stage sequence (modal-Esc → form; form-Esc → list; list-Esc → close) and KAT-C2e is
reworded to exactly two Esc presses after the modal [N2]. The hard-constraints "Esc-steps-back"
inheritance and the design sections no longer contradict.

**[M1] CLOSED — void retry story independently re-verified against resolve.rs.** The retry
appends a second `VoidDecisionEvent` whose `target_event_id` is the ORIGINAL decision (the modal
re-opens from the intact List row; the payload never names the first void), so pass-1a matches on
the TARGET's payload — e.g. `MethodElection` → the `Some(_)` catch-all (resolve.rs:329–331), NOT
the non-revocable arm (312–321), which fires only when the target's payload IS a
`VoidDecisionEvent`. `BTreeSet::insert` at 330 is idempotent → +2 inert rows, NO conflict, clean
status. The spec states this precisely and correctly distinguishes it from 2a's conflict-producing
FIRST-WINS retry. Save-error UI is now specified (modal closes, flow REMAINS at List — list
intact because no re-projection happens on `Err` — status `"Save error: {e}"`; retry =
re-select → modal → Enter). KAT-VOID-RETRY pins the +2-rows/no-conflict/target-still-excluded
contract at unit level (the voided-skip at resolve.rs:738 makes the still-excluded assert
provable); the KAT-S2b sampling justification now correctly points at it.

**[M2] CLOSED.** The spec now states the KAT updates are strengthenings (verified claim — every
existing pin survives), enumerates all FIVE existing tests including the previously-omitted
`kat_e2e_gift_price_gap_donor_date_outside_price_dataset` (main.rs:3256), and resolves the
replace-vs-update ambiguity: in-place strengthening + four new unit tests, nothing deleted.

**[M3] CLOSED.** The rejected-SafeHarbor-void list gap is documented in Claim E with an explicit
decision (record, don't refine — no remedial power lost; refinement would duplicate step-3
adjudication), the modal warning carries the permanence line, and FOLLOWUPS records it.

**[M4] CLOSED.** The Claim-D fire-path paragraph is rewritten with no self-contradiction and is
source-accurate: ManualFmv on a wallet-less Income is VALID at pass-1d (no conflict), the fold's
wallet check (fold.rs:648–657) runs before the FMV match, FmvMissing re-fires from the wallet arm,
and the status surfaces `b.detail` ("income without wallet").

**[M5] CLOSED.** `VoidListItem`/`VoidModalState` carry `inner_target` (per-payload inner event,
`None` for MethodElection/SafeHarborAllocation); `derive_void_status` gains a concrete
returned-blocker arm keyed on it; `target_event_id` is now load-bearing (the `{seq}` source).

**[N1]–[N4] CLOSED.** KAT-C1 → main.rs:2099–2172 and KAT-S2 → 2734–2911 corrected; resolve.rs
329→330, 429→427–428, 325–330→322–328 corrected; KAT-C2e Esc count fixed; Claim-C snippet hoists
both sets before the closure (2a precedent cited); "six MODAL layers in a nine-layer dispatch".

## New findings

None blocking. Two non-blocking nits, recorded here for the implementer (no re-review required):

- **[R2-N1]** The `derive_void_status` signature sketch omits `payload_tag`/`seq` used by its
  clean/returned-blocker strings (`seq` is derivable from `target_event_id`; `payload_tag` must be
  passed or carried on the modal state). Implementation detail; the arms themselves are fully
  specified.
- **[R2-N2]** KAT-VOID-CONFLICT-ARM's "starts with (or contains)" is loose; the third bullet's
  NOT-starts-with-"Voided" assert carries the honesty pin regardless. Prefer `starts_with`.

## Disposition

**R0 GREEN — 0 Critical / 0 Important.** The gate is cleared for implementation of Tasks 1–3
under the standard TDD discipline (KATs red first; full validation suite green at every step;
whole-diff review at Task 3). Persist this round verbatim per §2.
