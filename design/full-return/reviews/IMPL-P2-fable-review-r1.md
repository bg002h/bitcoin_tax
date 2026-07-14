# IMPL-P2 — Fable independent code review, round 1

**Scope:** full-return Phase 2 (the frozen-seam phase), branch `full-return`,
diff `059ec2a..8eb7118` (P1-GREEN → P2 head). Code commits reviewed: `0e62190`
(derive_tax_profile), `aeb1ec4` (exclusion KATs), `8191d37` (Schedule B),
`232579c` (resolver arm), `97f3cbd` (compute-dependent refuse rows), `1e62541`
(consumer sweep + negscreen destructure), `8eb7118` (provenance deferral).
Process commits `74c488f` / `9bbeddf` checked only for the deferral records.
**Reviewer:** Fable (independent; author was a different model).
**Date:** 2026-07-12.

**Verdict: NOT GREEN — 2 Critical / 1 Important / 4 Minor.**

---

## 1. Verification actually run (real output)

`cargo test --workspace` (full suite, exit 0):

```
EXIT=0
81            ← count of "test result: ok" lines (every test binary green)
TOTAL passed: 1458  failed: 0
```

`cargo clippy --workspace --all-targets` (tail):

```
    Checking btctax-cli v0.5.0 (/scratch/code/bitcoin_tax/crates/btctax-cli)
    Checking btctax-tui v0.5.0 (/scratch/code/bitcoin_tax/crates/btctax-tui)
    Checking xtask v0.5.0 (/scratch/code/bitcoin_tax/crates/xtask)
    Checking btctax-tui-edit v0.5.0 (/scratch/code/bitcoin_tax/crates/btctax-tui-edit)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.99s
```

Frozen-engine byte-identity:

```
$ git diff 059ec2a..8eb7118 -- crates/btctax-core/src/tax/types.rs \
    crates/btctax-core/src/tax/compute.rs crates/btctax-core/src/tax/se.rs | wc -c
0
$ cargo test -p btctax-core --lib frozen_guard
test tax::frozen_guard::tests::frozen_engine_files_are_unchanged ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 142 filtered out
```

The three pinned SHA-256 fingerprints in `frozen_guard.rs` are unchanged and the
guard test passes. The frozen engine is byte-identical. ✓

---

## 2. Frozen-seam verification (deep/02 §1 Worked Example 1, re-derived by hand)

`derive_tax_profile` (`crates/btctax-core/src/tax/return_1040.rs:217-289`),
checked line-by-line against deep/02 §1.2–1.3 (MFJ, TY2024):

| Step | Hand re-derivation | Code path | ✓ |
|---|---|---|---|
| wages | 180,000 + 90,000 = 270,000 | `sum_wages` (Σ box 1) | ✓ |
| taxable int (2b) | 4,000 (box 1) + 0 (box 3) | `sum_taxable_interest` box1+box3 (box 3 is NOT ⊂ box 1 — correct) | ✓ |
| ord div (3b) | 10,000 = Σ box 1a (⊇ 1b) | `sum_ordinary_dividends` box 1a only | ✓ |
| qual div (3a) | 8,000 = Σ box 1b, split ONLY | `sum_qualified_dividends`, never added to income | ✓ |
| cap-gain distr | 3,000 box 2a, in AGI once via L7 | added to `income_total` AND to `other_net_capital_gain` | ✓ |
| AGI (L11) | 270,000+4,000+10,000+3,000 = **287,000** | `income_total − adjustments` | ✓ |
| deduction | std MFJ 29,200 (basic only, P2) | `std_deduction_for` | ✓ |
| TI (L15) | 287,000 − 29,200 = **257,800** | `(agi − deduction).max(0)` | ✓ |
| ord TI | 257,800 − 8,000 − 3,000 = **246,800** | strip qd + box2a EXACTLY once (`:254`) | ✓ |
| `w2_ss_wages` | **168,600** — SE-earner's OWN box 3 (Taxpayer default, no Sch C), NOT the 258,600 household sum | owner-filtered Σ box3+box7 (`:259-269`) | ✓ |
| `w2_medicare_wages` | **270,000** — household Σ box 5 | unfiltered Σ box5 (`:271`) | ✓ |
| `magi_excluding_crypto` | = AGI = 287,000 (deep/02 C1: no §911/CFC/PFIC in the model; muni interest is NOT a §1411 add-back) | `magi_excluding_crypto: agi` | ✓ |

The KAT `derive_matches_deep02_example1_to_the_cent`
(`return_1040.rs:465-501`) asserts exactly these values, every field, plus the
round-trip identity `246,800 + 8,000 + 3,000 = 257,800`; it passes in the green
suite above. Cent-exact. ✓

**Strip-once invariant.** Holds exactly when `TI ≥ qd + cap_gain_distr`. The
`.max(Usd::ZERO)` at `return_1040.rs:254` breaks the identity when
`TI < qd + cap_gain_distr` (ordinary base clamps to 0 while the full pref slice
is still handed to the engine) — this is audit-final **M2**, already recorded,
assessed further as finding M1 below.

**Structural crypto exclusion.** Verified: `ReturnInputs` (return_inputs.rs)
carries only W-2/1099/Sch-1/Sch-A/payments fields — no ledger figure is
reachable from `derive_tax_profile`, so crypto cannot leak into or double-count
in the derived profile. The engine adds crypto exactly once (`compute.rs:339-342`
`bottom_with = profile.ordinary_taxable_income + crypto_ord + …`). The two
seam KATs (`derived_profile_composes_with_the_frozen_crypto_engine`,
`forgetting_to_strip_changes_the_engine_result`) prove zero-delta on an empty
ledger and that the strip is load-bearing through the engine. ✓

**SE-earner channel.** `se_owner = schedule_c.owner` (Taxpayer when none);
`se_owner_selects_ss_wages_channel` KAT pins spouse-owned-Sch-C →
spouse's box 3 with household box 5 — matches deep/02 §3.4/C4. Box 7 SS tips
correctly join box 3 (Sch SE L8a = box 3 + box 7). ✓

**Documented delta divergence.** The student-loan deduction is fixed at the
non-crypto MAGI (`:239-245`); with-crypto MAGI could phase it lower. This is the
deliberate, SPEC-§6-documented delta approximation, stated in the fn doc
(`:180-182`), bounded by $2,500 × marginal rate. Settled design; verified as
documented, not a finding.

---

## 3. Fail-closed screening — traced

`resolve_and_screen` (`crates/btctax-cli/src/resolve.rs:146-175`):
ReturnInputs present → (a) no TY tables ⇒ `Uncomputable` (no number); (b)
`screen_inputs` refusal ⇒ `Uncomputable` with reason; (c) derived profile then
`screen_compute_dependent` (business Interest / business-without-Sch-C /
Sch-C loss / kiddie) ⇒ `Uncomputable` on any hit. I traced every refuse row to
an early `return` before any `compute_tax_year` call; within this entry point
no refuse-worthy `ReturnInputs` can produce a number.

CLI consumer sweep — each verified to route through
`Session::resolve_screened{,_profile}` → `resolve_and_screen`:

- `report_tax_year` (`cmd/tax.rs:164-178`) — `Uncomputable ⇒ CliError::Usage` (hard fail). ✓
- `optimize run/consult/accept` (`cmd/optimize.rs:46,110,180`) — hard fail. ✓
- `what-if sell/harvest` fallback (`cmd/whatif.rs:84,142`) — hard fail; ad-hoc arg path stays ad-hoc (per plan). ✓
- TUI `optimize_proposal` (`session.rs:587`). ✓
- `export_snapshot` / `export_irs_pdf` (`cmd/admin.rs:92,252`) — Uncomputable ⇒ SE figure omitted, export proceeds (a data snapshot never emits a wrong number — acceptable mapping). ✓
- prior-year M4 advisory (`cmd/tax.rs:227-231`) — Uncomputable ⇒ skip (non-gating). ✓

Integration KATs confirm behavior end-to-end
(`report_tax_year_derives_and_computes_from_ty2024_return_inputs`,
`report_tax_year_refuses_business_income_without_schedule_c`,
`…unsupported_year_refuses_with_income_clear_hint` — all green).

**But the sweep is incomplete — the TUI's own snapshot consumers were missed.
See Critical C1.**

**Kiddie arithmetic** (`return_1040.rs:156-172`): `unearned = 2b + 3b + L7 +
state refund + unemployment + non-business crypto ordinary`; earned (wages +
Sch C net) excluded by construction. Checked the specific worry that a capital
LOSS lowers `unearned` below $2,600 and under-refuses: per the Form 8615
instructions the child's unearned income is AGI − earned income, and AGI
includes the §1211-limited loss (1040 L7 can be −3,000) — so a net loss
*correctly* reduces unearned; this is not an under-refuse.
`capital_gain_line7` (`:92-104`) feeds `net_1222(sd.st.gain, sd.lt.gain,
box2a, cf.short, cf.long, loss_limit)` — the same signature/character
conventions the frozen engine uses at `compute.rs:319-335` (box 2a is
LT-character `other_lt`; carryforwards are same-character loss magnitudes), and
`ord + pref − loss_deduction` is exactly L7 in both gain and loss years. ✓
Pseudo income rows are counted by both the screen and the frozen engine
(neither filters `pseudo`) — consistent. The screen's component-sum omits the
Sch-1 adjustments that AGI−earned would net out, so it can only *over*-state
unearned ⇒ over-refuse (conservative; noted as M4).

**Negative-screen destructure** (`return_refuse.rs:110-276`): walked every
field of every destructured struct. All `Usd` fields are checked; every `_` is
genuinely non-money (`header` = names/SSNs/DOBs/flags only — verified against
`Person`/`Dependent`/Header in return_inputs.rs; `naics_code`,
`accounting_method`, `salt_use_sales_tax`, `hsa_present`, tri-states, strings).
No `..` anywhere, so a new `Usd` field is a compile error. Fold of R3-M1 is
correct and complete. ✓

**Consumer-sweep pseudo change:** optimize/what-if/export previously read the
stored profile only; they now inherit the pseudo placeholder when the mode is
on. This makes them consistent with `report` (the single-resolver objective) —
a deliberate fix, not a regression; existing pseudo/optimize/what-if suites
pass unchanged.

---

## 4. Findings

### CRITICAL

**C1 — The TUI's snapshot consumers bypass the single resolver: two
liabilities for one year, and refused years still show a number.**
`crates/btctax-tui/src/tabs/tax.rs:56` (`render_tax_content` calls
`compute_tax_year(…, snap.profiles.get(&year), …)`), `tabs/tax.rs:121` (SE
section), `whatif_panel.rs:243` (stored-or-placeholder), `export.rs:89` and
`export.rs:141-150` (SE CSV) all read `Snapshot.profiles` =
`session.all_tax_profiles()` (`unlock.rs:174`) — the RAW stored-profile map.
None of `derive_tax_profile`, `screen_inputs`, or `screen_compute_dependent`
runs on these paths.

Concrete failure scenario (directly reachable, no `--force` needed): store a
raw 2024 `tax-profile` (pre-existing), later run `income import --year 2024`
with full-return inputs (nothing guards this order — only the *reverse* is
guarded in `cmd/tax.rs:24-32`). Now CLI `report --tax-year 2024` derives from
`ReturnInputs` (liability X) while the TUI Tax tab computes with the stale
stored profile (liability Y ≠ X) — the exact "two different liabilities for
one year" that `resolve.rs:1-6` names the cardinal sin. Worse, fail-open: make
the imported inputs refuse-worthy (e.g. `hsa_present = true`, or business
income without a Schedule C) — the CLI refuses with a reason; the TUI Tax tab,
what-if panel, and export happily print/export numbers from the stale profile.
Even without a stale profile the TUI shows `TaxProfileMissing` for a year the
CLI computes — a contradiction on every ReturnInputs vault.

This was a MANDATORY P2 obligation: FOLLOWUPS `p1-consumer-sweep-P2` — "P2
MUST route **every** profile consumer through `resolve_profile` … or a year
with `ReturnInputs` silently gives those paths a stale/absent profile" — and
SPEC §4.12 names the TUI as a consumer. The P2 record
(`FOLLOWUPS.md:15-21`, "every computing consumer now goes through the shared,
fail-closed `Session::resolve_screened`") claims completion but only
`optimize_proposal` (which lives in `Session`) was swept; the TUI's four
snapshot read-sites were not, and no deferral is recorded. Fix direction:
resolve per year at `build_snapshot` time (it holds the `Session`) into the
snapshot, or replace `Snapshot.profiles` reads with a resolved map; a
refused/uncomputable year must render its refusal in the TUI, not a number.

**C2 — §221 student-loan phase-out: QSS is mapped to the married range —
understates tax for a Qualifying Surviving Spouse.**
`crates/btctax-core/src/tax/tables.rs:263`:
`FilingStatus::Mfj | FilingStatus::Qss => Some(self.student_loan_phaseout_married)`.
§221(b)(2)(B) doubles the phase-out floor only "in the case of a **joint
return**"; a QSS return uses MFJ *rate schedules* but is not a joint return.
The TY2024 authorities group QSS with the unmarried range: Pub 970 ch. 4 /
the Schedule 1 worksheet — "single, head of household, or **qualifying
surviving spouse**: $80,000–$95,000; married filing jointly:
$165,000–$195,000" — and Rev. Proc. 2023-34 §3.22 (the very cite on the
constant at `tax_tables.rs:132-133`) says "$165,000 **for joint returns**".
This codebase already implements exactly this QSS-is-not-a-joint-return
distinction for §904(j) (`return_refuse.rs:95-103`, review I2, KAT
`foreign_tax_over_ceiling_refuses` pins QSS at $300) — the two sites now
contradict each other.

Concrete failure: QSS filer, $2,500 interest paid, MAGI $120,000 → code grants
the full $2,500 deduction (below $165k); correct is **$0** (≥ $95k). AGI
understated by $2,500 ⇒ taxable income and tax silently understated (~$550 at
22%, up to ~$925). Reachable: `filing_status = "Qss"` imports fine, nothing
refuses QSS, `std_deduction_for` maps Qss→Mfj (which IS correct for §63(c)(2)
— "surviving spouse" is statutorily in the joint bucket there, unlike §221).
Fix: `Qss` → `student_loan_phaseout_unmarried`, + a KAT pinning QSS at the
$80k–$95k range (mirror the §904(j) QSS KAT). One-line fix, but it is a wrong
tax number in the understatement direction — Critical per house rules.

### IMPORTANT

**I1 — Schedule B Part III "fail-loud if None" is an orphan predicate — the
plan's P2 task-4 behavior is not delivered and the gap is unrecorded.**
Plan P2 task 4 (`IMPLEMENTATION_PLAN_full_return.md:128-131`): "When filing,
Part III 7a/8 tri-state ⇒ **fail-loud if `None`**." SPEC §7.1
(`SPEC_full_return.md:423-424`) same. The implementation ships
`schedule_b_part3_unanswered` (`return_1040.rs:313-315`) whose rustdoc says
"the caller refuses rather than guess" — and it has **zero call sites**
outside its own unit tests (verified: repo-wide grep). A household with $2,000
of interest and `foreign_accounts: None` derives, screens clean, and computes
with no fail-loud anywhere. The computed *number* is not wrong (7a is a
disclosure answer, and 2b/3b are already summed), so this is not Critical —
but the plan put the fail-loud in P2, the commit (`8191d37` "…+ Part III
fail-loud (task 4)") claims it, and unlike the two recorded deferrals this gap
has no FOLLOWUPS entry. Fix: wire it into `screen_inputs` (it is
input-screenable — a `RefuseReason::ScheduleBPart3Unanswered` row), or record
an explicit FOLLOWUPS deferral to the phase that owns it (P6 fill at the
latest) with the fail-closed justification. Silent partial delivery of a gate
task is what this review exists to catch.

### MINOR

**M1 — Audit-M2 (pref > TI clamp) is now live in shipped code and was not
folded at its scheduled phase.** `return_1040.rs:254`'s `.max(Usd::ZERO)`
clamps the ordinary base to 0 when `TI < qd + cap_gain_distr` (low ordinary
income + large qualified dividends — a plausible retiree household) while the
full pref slice still reaches the engine, which stacks `qd + pref_gain` with
no min-against-TI cap (`compute.rs:348-350`) — the delta/planning number
degrades (overstates; it can never understate, since the reconstructed TI is
only ever ≥ the true TI). DESIGN-fable-audit-final M2 ranked this Minor twice
with two remedies ("clamp at derivation **or** add it to §6's documented
approximations"); FOLLOWUPS says "fold opportunistically during the relevant
phase". P2 built the derivation — the relevant phase — and did neither; even
the `derive_tax_profile` doc is silent. Keeping the settled Minor rank (not
re-litigating), but this should be folded no later than P3 (P4's dual report
will surface the discrepancy): clamp `qd`/`cap_gain_distr` down so
`qd + other ≤ TI` (reduce `other` first, mirroring the worksheet), or document.

**M2 — `schedule_b_files` re-implements the 2b/3b sums inline**
(`return_1040.rs:299-304`) instead of calling `sum_taxable_interest` /
`sum_ordinary_dividends` twenty lines up. Drift risk if the 2b/3b definition
ever grows a source; trivially deduplicable.

**M3 — `resolve_and_screen` re-reads `ReturnInputs` from the DB**
(`resolve.rs:163`) after `resolve_profile` already fetched them (`:82`) — the
profile was derived from the first read, the compute-dependent screen runs on
the second. Same connection, so a divergence is not practically reachable
today, but screening different bytes than were derived is the kind of seam a
future refactor trips over. Return the `ri` (or the derived pieces) from
`resolve_profile` instead.

**M4 — Kiddie `unearned` overstates by the Sch-1 adjustments** (early-
withdrawal penalty, student-loan deduction): Form 8615's worksheet is
AGI − earned income; the component sum at `return_1040.rs:159-164` omits the
adjustments, so `unearned` is high by up to their sum ⇒ can only over-refuse.
Conservative and acceptable fail-closed; worth a one-line comment so a future
"fix" doesn't flip it the other way without noticing the direction.

---

## 5. Deferral assessment

**D1 — absolute income-assembly struct (plan P2 task 1) → P4: ACCEPTABLE.**
The absolute WITH-crypto L1a..L11 struct has no P2 consumer (the delta report
is the only output surface until P4's dual report), and its L11 would be
knowingly wrong without ½-SE (a P4 stage). Building it now = stubbed dead code
that P4 rebuilds — the no-stub rationale is sound. Fail-closed impact: none —
an absent struct cannot emit a number; nothing in P2 fail-opens because of it.
The deviation is properly recorded (`FOLLOWUPS.md:7-14`) with the
`L11 = L9 − L10` cross-foot KAT and the four Schedule-D routing paths
explicitly re-owned by P4, and the P2 replacement (derivation side +
`crypto_income`/`capital_gain_line7`, which the P2 screens already consume, so
they are live code) is real. Condition for P4: the cross-foot KAT and routing
paths are gate-blocking there; P2's formal acceptance line is amended by this
record.

**D2 — provenance printing (§4.12) → P4: ACCEPTABLE.** The mechanism is done
(`Resolved.provenance`, `ProfileOutcome::Ready { provenance, .. }` carried
through every consumer); only rendering is deferred, and P2 still emits the
pre-existing crypto-delta report — there is genuinely no finished output line
for a provenance label to live on. Not fail-open: provenance is an audit
nicety; the number itself is single-sourced *within the CLI*. Caveat: C1 shows
the "printed on every output so a reviewer can audit" goal has a bigger hole
than printing — fixing C1 matters more than the label, and P4 should land both
together.

---

## 6. Settled items honored (not re-litigated)

P0/P1 findings and their folds were checked only for regression: the R3-M1
exhaustive destructure is correctly implemented (§3); the §904(j) QSS ceiling,
per-owner §402(g), negative-screen-first ordering, and spouse-owner guard all
retain their KATs and pass. The `p1-se-earners-and-business-interest-rows`
resolution (business Interest refuses; ≥2 SE earners structurally
unrepresentable) is correctly realized in `screen_compute_dependent` +
the single `schedule_c: Option<…>` model.

---

## Verdict: NOT GREEN

2 Critical (C1 TUI resolver bypass; C2 QSS §221 phase-out range) /
1 Important (I1 Schedule B Part III fail-loud unwired & unrecorded) /
4 Minor. The frozen seam itself is byte-frozen, cent-exact against deep/02
Ex. 1, and correctly fail-closed **within the CLI**; both recorded deferrals
are acceptable. Re-review required after the folds.
