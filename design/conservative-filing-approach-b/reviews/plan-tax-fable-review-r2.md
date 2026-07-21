# Plan review — US-federal-tax-correctness lens (Fable) — ROUND 2

**Artifact:** `design/conservative-filing-approach-b/IMPLEMENTATION_PLAN.md` @ `50d7f3e` (branch `feat/conservative-filing-b`)
**Against:** r1 (`plan-tax-fable-review-r1.md`, 0C/8I/7M/3N) + SPEC (GREEN) + current source. All fold-added
citations re-verified against the tree: `consume_fee` TreatmentC summation fold.rs:348-357 (the exact T5
subtraction point) / `FeeCarry` :273-277 / net-share split :133-140 / §1015 NoGainNoLoss reported≠consumed
:181-190; `build_op` `_ => Op::Skip` resolve.rs:405-413; forms.rs §170(e) "$0" doc sentence :264-268;
`basis_methodology` "never the estimate" sentence conservative.rs:~159-161 (line-wrapped in source — the
rendered string does contain it, so T11's negative assert is well-grounded) + advisory ranges :25/:57/:96/:125/:366;
admin.rs checked-first slots :80/:281-284/:535-538 + `write_basis_methodology_txt` :304/:555, render.rs:871/911;
session.rs:692-696/:713-716; `pre2025_tranche_exists` shared by BOTH record-time guards (tranche.rs:70/:92,
session.rs:692); `ATTEST_PHRASE` lib.rs:197; `would_conflict` mod.rs:107; `PrintedForms` packet.rs:421 /
`assemble_printed_return` :461; forms packet.rs:38-58 no-`..` destructure; `SUPPORTED_YEARS = [2017, 2024, 2025]`
(forms lib.rs:61); `Printed8283Rows` printed.rs:135; `verify` = cmd/inspect.rs:146. All accurate.

## Verdict

**NOT green. 0 Critical / 1 Important / 3 Minor / 2 Nit.**

The engine core, the decomposition chain, and 7 of the 8 r1 Importants are genuinely and correctly folded. The
one blocker: the **BG-D3 verify-drift advisory (r1 I-2 / arch I-4) is claimed folded ("→ T11") in the Status
line and Self-Review but does not exist in T11's body** — no step, no KAT, no `verify` touchpoint. The fold
record asserts a task that was never written.

---

## Verified resolved (r1 → status)

- **I-1 → RESOLVED.** T3 Step 1 now carries all three by-construction KATs:
  `snapshot_timing_the_floor_is_visible_to_pass1_conservation` (★ pins the inside-resolve timing against the
  `overpayment_delta_one` post-resolve precedent; mutation named), `relocated_promoted_tranche_keeps_tag_and_floor`,
  `a_promoted_tranche_still_refuses_a_safe_harbor_allocation_at_record_time`. The single refusal KAT covers both
  cited sites by fact: tranche.rs:92 and session.rs:692 both call the one tag-keyed `pre2025_tranche_exists`
  predicate, so exercising it with a promote on file pins both.
- **I-2 → NOT RESOLVED.** See Important-1 below.
- **I-3 → RESOLVED.** `Direction::Promote` wired in T10 Step 3 (printed before the consent prompt, declared the
  only promote-direction call site); `Direction::Void` wired in T8 Step 3b (`cmd/reconcile.rs` + bulk-void, for
  a `PromoteTranche` void AND a promoted-target void) with a CLI void-direction test and a named mutation. The
  file-map `compute.rs`/`cmd/tax.rs` claim is reconciled as a recorded no-change decision in T8's Reference —
  which SPEC §3 item 8c expressly permits ("The plan decides whether their copy becomes promote-aware"). Residual
  map-listing inconsistency → Nit-2.
- **I-4 → RESOLVED (type + routing).** T1 `ComputedTax` gained `deduction_delta_usd: Option<Usd>` with the
  engine-B-excludes-crypto-donations comment; T9 Step 3 routes by computability only and fills `Some(Σ diff)` on
  a removal-leg diff; KAT `a_computing_removal_flagged_year_carries_the_deduction_delta` (2024 table-ships +
  profile + donation reorder → `ComputedTax{deduction_delta: Some(≠0)}`, not `Uncomputable`) holds the routing.
  Residuals: the survived "never emit `ComputedTax{delta:0}`" sentence → Minor-3; the unpinned exclusion copy → Minor-2.
- **I-5 → RESOLVED (flavor + fallback).** T1 `Unrealized { sat, hypothetical_reduction: Option<Usd>, as_of }`;
  T9 emits it for undisposed sats with the two-stage no-current-price fallback; KATs
  `fully_undisposed_promote_records_an_unrealized_term_not_empty` + `no_current_price_falls_back_to_the_floor_as_max_reduction`
  pin the term and the fallback branch at the type level. Residual: the "hypothetical, not a filed figure"
  render label unpinned → Minor-2.
- **I-6 → RESOLVED, and the fold is tax-correct.** T13 Part I is 8949-disposal-scoped, **amount = `leg.basis` AS
  FILED** (clamped where the clamp bound; KAT asserts `== filed_8949_col_e_basis` and `!= dec!(12_000)`);
  removal legs excluded (`removal_donation_legs_are_absent_from_part_i`). Fresh completeness check on the
  re-scope: no estimated position now goes undisclosed — the estimate reaches a filed amount ONLY through
  promoted 8949 disposal legs (TreatmentB fee mini-dispositions route through `make_disposal_legs`, so they ARE
  disposal legs and are included; a relocated tranche keeps `origin_event_id`, so its later disposals still key
  into the `PromoteSet`; removal legs file documented-only per BG-D11 = no estimated position; TreatmentC fee
  estimate evaporates, so the survivor's basis carries none; a clamped-to-$0 leg is still itemized, with the
  no-loss sentence). A promoted removal-only year correctly yields `None` — the return takes no estimated position.
- **I-7 → RESOLVED.** Gift KAT asserts `!contains("1040-X")`; `a_both_deltas_zero_flagged_year_names_the_changed_content_not_a_bare_zero`
  added + the copy rule in Step 3; `a_donation_reorder_names_the_170d_charitable_carryover_direction` added;
  `the_void_direction_fires_amend_to_pay` added (and the void copy it pins is the tax-correct direction).
- **I-8 → RESOLVED.** Refusal KAT constructed via the specified raw-vault bypass (`raw_vault_promote_with_empty_part_ii`);
  the `incomplete` predicate is now a real field on `Disclosure8275` (T13: `part_ii.trim().is_empty()`); the
  success KAT asserts `form_8275.txt` by its own name, no `basis_methodology.txt` disjunction (whose
  always-written character I re-confirmed at render.rs:871/911).
- **M-1 → RESOLVED.** T8/T9 baselines both "exclude the `PromoteTranche` EVENT itself, NOT its `DeclareTranche`
  target". Fresh check: this is the correct diff — baseline = tranche at `$0` with all disposals/removals
  intact; the per-year leg-set diff then flags exactly the years whose filed content the promote changes
  (basis/gain + HIFO reorders), no `UncoveredDisposal` garbage.
- **M-2 → RESOLVED.** T7 adjudicates deferred tranche-voids immediately after pass-1a, BEFORE step 2 (correct —
  a post-step-3 `voided.insert` is a no-op), and `void_of_promote_alone_reverts_to_zero_tag_intact` pins the
  plain §6 revert.
- **M-3 → RESOLVED.** T6 LT KAT asserts `form_8283_cost_basis == 0` (documented-only column, LT too).
- **M-4 → RESOLVED.** T10 iterates all six non-Purchase `ProvenanceKind`s, asserts fail-closed + the
  closed-enumeration/"real acquisition" copy.
- **M-5 → RESOLVED.** T10 consent-copy KAT (underpayment base, "plus interest", never "safe harbor"); T11
  funnel KAT asserts the quoted saving equals the CLAMPED promote delta.
- **M-6 → RESOLVED.** T11 owns §3 items 6/7 (parent D-7 re-scope, event.rs/forms.rs:264-268 doc fixes, with the
  explicit NB that T6's no-patch rule covers basis consumers, not doc comments); T3 adds the explicit `build_op`
  arm (item 11, insertion point verified at resolve.rs:405-413); T7 Step 3(b) is the explicit void-classification
  arm; T4 amends the parent Invariant KAT wording.
- **M-7 → RESOLVED.** T15 mandates aliasing the single 8275 revision to every `SUPPORTED_YEAR` with per-year
  KATs (2017/2025) + T16's end-to-end 2025 gate-green KAT. Verified complete against source: forms
  `SUPPORTED_YEARS = [2017, 2024, 2025]` is the crate-wide PDF-export universe (a 2026 packet is refused on
  every form today), so the aliasing covers every year a promoted leg can PDF-export.
- **N-1 → PARTIALLY RESOLVED.** Distinct `PROMOTE_ACK_PHRASE` const mandated ("NOT the pseudo-attest phrase")
  and `Acknowledgment{phrase: PROMOTE_ACK_PHRASE}` recorded — but T10's KAT snippets still pass
  `Some(ATTEST_PHRASE)` → Nit-1.
- **N-2 → RESOLVED.** T10 Step 3: figures still printed on the `--i-acknowledge` path; "plus interest" pinned
  in the consent-copy KAT.
- **N-3 → RESOLVED.** T14: `year: None` = "any year with a promoted filed leg in the exported range".

---

## Important

### I-1 (r2) — r1 I-2 (BG-D3 verify-drift advisory) is claimed folded but was never written: the Status line and Self-Review say "→ T11"; T11 contains no drift step, no drift KAT, no `verify` touchpoint.

- **Defect:** the fold record asserts "the verify-drift task (T11)" (Status, line 10) and "**verify-drift
  advisory → T11 (added, plan-r1 I-2/I-4)**" (Self-Review). Grep the plan: "drift" appears ONLY in those two
  claims and the Global Constraints quote. T11's Files (conservative.rs advisories, tranche.rs/session.rs/
  resolve.rs copy), its three KATs (basis_methodology / dip+self-custody / funnel), and its Step 3 contain zero
  drift work — no recompute-vs-stored comparison, no direction-aware advisory, no mutated-price fixture, no
  fold-still-uses-stored pin, and no `verify` surface (`cmd/inspect.rs::verify` → `VerifyReport`, whose current
  signature takes only vault+passphrase and would need prices threaded — precisely why this needs owned steps,
  not a gesture). Both lenses flagged this gap in r1 (tax I-2, arch I-4); it is now doubly defective: the
  surface is still unbuilt AND the plan's own fold record is false.
- **Tax failure (unchanged from r1):** a bundled-price correction that lowers the window min leaves an unfiled
  position silently carrying an overstated floor — an understatement the filer is never told to correct. The G-4
  anti-overstatement guardrail the spec traded for dropping the v1 warn-if-above check (SPEC :106-108, §6 :495)
  is simply absent from the plan.
- **Fix:** write the actual T11 step (or a new task): a `promote_drift_advisory` in `conservative_promote.rs` —
  per live promote, recompute `filed_basis_for` against current prices vs the stored `filed_basis`; stored >
  recomputed on a not-yet-filed position → the "void + re-promote to the corrected lower number" hint;
  already-filed → advisory only; surface it through `cmd/inspect.rs::verify` (thread a `PriceProvider`/report
  field). KATs: a mutated-price fixture pinning BOTH directions AND that the fold still consumes the STORED
  number. Then correct the Self-Review to cite the step that exists.

## Minor

### M-1 (r2, T8 Step 3, fold-introduced) — the closing sentence inverts the amend directions: "amend-to-**pay** (promote direction) / amend-to-refund (void)".

The correct mapping is the reverse: the PROMOTE direction claims a higher basis on a filed year → amend-to-
**refund** (which is why §6511's refund limitation is the cited authority); the VOID direction reverts filed
floor-basis to `$0` → amend-to-**pay** (BG-D9's verbatim mandate, and what T8-3b and the
`the_void_direction_fires_amend_to_pay` KAT correctly say — that KAT would red a literal implementation of the
sentence, r1-M-1 precedent for severity). Fix the sentence; since a promote-direction HIFO reorder can also
raise a flagged year's tax, the honest rule is: copy follows the SIGN of the year's Δ, naming §6511 on a refund
claim and "additional tax, plus interest" on a pay. Cheap extra pin: assert the promote-direction advisory on a
pure basis-increase fixture does NOT say "additional tax".

### M-2 (r2, T9/T10) — two BG-D6/D9 consent-copy mandates are unpinned: the tax-Δ-excludes-deduction sentence and the "hypothetical, not a filed figure" label.

BG-D9: "The tax-Δ figure must NOT be implied to capture the deduction effect" lives only in T1's type comment;
T10's `render_consent` copy spec and its copy KAT never mention it — a consent screen showing "2024: tax Δ $0,
deduction Δ $12,000" without the exclusion sentence implies the tax-Δ is the whole effect. Likewise BG-D6's
"(hypothetical, not a filed figure)" label for the `Unrealized` line is pinned only as a term flavor, not as
rendered copy (r1 I-5's fix asked for the label assert). Fix: add both sentences to T10 Step 3's copy list and
extend `the_consent_copy_names_the_underpayment_base_and_never_says_safe_harbor` (or a sibling) to assert them
on a fixture with `deduction_delta: Some` and an `Unrealized` term.

### M-3 (r2, T9 Step 3) — "NEVER emit `ComputedTax{delta:0}` for a real change" survived the I-4 fold and now literally forbids the correct term for a donation-only computing year.

A donation-only reorder in a computing year yields engine-B tax-Δ = exactly `$0` with `deduction_delta_usd:
Some(D)` — the correct, honest term is `ComputedTax { delta_usd: 0, deduction_delta_usd: Some(D) }`, which the
sentence forbids; the only literal-compliance escapes are the I-4 mislabel (`Uncomputable` for a computing
year — a false statement in the §6664(c) artifact) or dropping the year. The KAT wins in practice, but only if
`promote_reorders_2024_donation_with_profile` is donation-ONLY (so `delta_usd == 0` is the exercised path) —
the fixture name suggests it, the plan never says it. Fix: reword to "never a bare `$0` — `delta_usd: 0` is
permitted only when `deduction_delta_usd` is `Some(≠0)` and the copy names the excluded deduction effect", and
state the fixture is donation-only.

## Nit

### N-1 (r2, T10) — the KAT snippets still pass `Some(ATTEST_PHRASE)` (three sites) while Step 3/Reference mandate the distinct `PROMOTE_ACK_PHRASE`; an implementer making the snippets pass verbatim must accept the pseudo-attest phrase as promote consent — the exact confusion r1 N-1 flagged. Swap the snippets to `Some(PROMOTE_ACK_PHRASE)`.

### N-2 (r2, plan-consistency) — the File-Structure Map still lists `tax/compute.rs` + `cmd/tax.rs` under **Modified** "carryover-cascade naming hooks (T8)" while T8 records a (spec-blessed) no-change decision; and T14's mutation line names `export_with_a_promoted_leg_but_no_8275_content_refuses_before_bytes` vs the KAT's actual name `..._but_incomplete_8275_...`. Align both.

---

*Reviewer note: fresh-pass items checked and NOT filed: the T13 8949-scope completeness census (clean — see
I-6 entry); T9's clamped-saving KAT arithmetic (floor $12k / proceeds $8k → saving = tax on $8k gain, correct);
T16's Attachment Sequence "92" for Form 8275 (correct; 8275-R is 92A); the 1a-merge/1b-release split vs Reg
§1.6662-4(f) (sound — no released binary carries `promote` without the completed form, and there are no users
of unreleased `main`); T8-3b firing the advisory on an inert tranche-void (diff-driven, non-gating,
conservative); T12's `safe_harbor_residue` promote-drop (prevents a dangling-target phantom in the pre-2025
residue — correct). The T5 evaporation point (`consume_fee` TreatmentC summation :350) and the non-dual-arm
completeness were re-confirmed against source.*
