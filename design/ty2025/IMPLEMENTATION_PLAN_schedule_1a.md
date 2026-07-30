# Schedule 1-A (TY2025) — IMPLEMENTATION PLAN

**Status: DRAFT r1.** Implements `design/ty2025/SPEC_schedule_1a.md` **r3** (0 Critical / 0 Important),
which is branch **B3** of `design/ty2025/SPEC.md` §8a. Parent decisions D-1 … D-11 bind.

**Branch:** `feat/amt-e2-vector-population` (current; B1 + B2 already on it).

---

## 0. What this plan is, and what it deliberately is not

**It is a sequencing document.** The transcription itself lives in the code, per this project's standing
rule: *one field per numbered line, in the form's own numbering, carrying the official instruction text
verbatim as its doc comment.* This plan does **not** restate the 38 lines — restating them here would
create a second, unexecutable copy that can drift from both the form and the struct. The extracted text
layer is the source: `f1040s1a--2025.pdf` (`64f97b38`) and `i1040gi--2025.pdf` (`482e9c48`) pp. 101-110,
both archived in `design/amt-form6251/`.

**Consequently the review gate on this work is mechanical** (CLAUDE.md): *is every line present, and
does each doc comment match the instruction text?* That is a test, and T2 writes it.

**★ This plan does NOT delete `ty2025_full_return_must_stay_fail_closed_until_complete`.** B3 satisfies
the gate's **condition 4** only. Conditions 2 and 3 landed in B2; condition 1 (the `FullReturnParams`
themselves) is the LAST thing to land, after B4. Until then Schedule 1-A is fully built and fully tested
against synthetic params and reaches no filed return.

---

## 1. Exit criteria (the definition of green for B3)

1. The **five gates**: `make check` · `cargo fmt --all --check` · `cargo +1.88 check --workspace
   --locked` · `cargo run -p xtask -- check-isolation` · `bash scripts/pii-scan-generic.sh`.
2. **All 38 numbered lines present**, asserted by a closed-at-both-ends KAT (T2), plus all four
   worksheets.
3. **Every part's phase-out tested at its own knee in its own direction** (S-1), including the recon's
   worked examples (b) $2,300 and (c) $5,000 — figures that differ under the wrong rounding.
4. **TY2024 provably unmoved**: golden matrix md5 `c4e1853ed82d113ca5cd97ffd8abbf47`, both oracles
   exit 0.
5. **Mutation-verified**, per guard. A guard whose mutation survives is not a guard.
6. **TY2029+ fails closed**, mutation-verified.

---

## 2. Task sequence

Each task is test-first and lands green. Tasks T1-T2 are the chokepoints — T3 onward all depend on the
shapes they fix, so they are done first and reviewed before the surface work fans out.

### T1 — the per-year table, with rounding as a parameter

`crates/btctax-core/src/tax/tables.rs`.

A `Schedule1aParams` carried per year for **2025-2028 only** (S-7: nothing is indexed, all four
provisions expire after TY2028, so a Rev. Proc. lookup is not merely unnecessary but wrong).

**The three things that must not be shared:**

- **Rounding direction is an explicit argument, never baked in** (S-1). Parts II/III **floor** (lines 11,
  19: *"decrease the result to the next lower whole number"*); Part IV **ceils** (line 28: *"increase the
  result to the next higher whole number"*). A `phase_out(excess, per_step, step)` helper with one
  direction is silently wrong on one side, by exactly $100 and $200 — which is precisely what worked
  examples (b) and (c) measure.
- **Three distinct threshold pairs** (spec F-4): $150,000/$300,000 (lines 9, 17), $100,000/$200,000
  (line 26), $75,000/$150,000 (line 32). No `threshold_for(status)`.
- **Three distinct caps**: $25,000 tips (line 7, per-return regardless of status — S-3), $12,500/$25,000
  MFJ overtime (line 15), $10,000 QPVLI (line 24).

**Tests.** Each constant against the extracted line text. `TY2029+` returns `None` and a mutation that
extends the table reds. The statutory identity worth asserting here: every part reaches $0 exactly at
`threshold + cap/per_step × step` — for Part V, `threshold + $100,000` (S-4), which is a closed-form
check on threshold, rate and cap together rather than three separate literals.

### T2 — the struct: 38 lines + four worksheets, and the conformance KAT

`crates/btctax-core/src/tax/schedule_1a.rs` (new).

One field per numbered line, named for it (`line4a`, `line36b`, …), instruction text verbatim as the
doc comment. Sub-structs per part keep the names short without renumbering.

**The four worksheets are their own transcribed types**, not `min()` calls in the emitter (spec F-2, and
OQ-2's closure): *Qualified Tips From More Than One Employer*, *Multiple Trades or Businesses*,
*Qualified Overtime Compensation From More Than One **Employer***, and *… From More Than One **Payor***.
The last two are distinct forms of the same idea (W-2 side vs 1099 side) and the r1 branch list
collapsed them.

★ **The *Multiple Trades or Businesses* worksheet is where OQ-2's real content lives.** The per-business
ceiling is *not* net profit: it is net profit (Schedule C line 31 / the total of Schedule E lines 28(g)
through 28(k) / Schedule F line 34) **minus** the deductible part of self-employment tax, the deduction
for contributions to self-employed SEP/SIMPLE/qualified plans, and the self-employed health insurance
deduction, **floored at zero**, and expressly **not** reduced by the qualified-tips deduction itself
(which is what keeps it acyclic).

**The conformance KAT** — the mechanical gate, executed:

- every one of the 38 numbered lines is a field, with the list **closed at both ends** via a
  `BTreeSet` comparison so neither a missing line nor an unexpected extra passes (the pattern the Form
  6251 KAT settled on);
- each field's doc comment contains the line's own instruction text, checked against a committed
  extract of the text layer, so a paraphrase reds.

### T3 — the input surface, landed whole

~25 leaves plus **six declarations**, through the whole stack in one pass (the G-9 walk, since the user
has directed that the input surface not lag the core): `return_inputs.rs` → `classifier.rs` →
`questions.rs` → `return_refuse.rs` → `input-form` `seam/registries/coverage/sections` → CLI `answer.rs`
→ TUI. The exhaustive matches and the coverage KAT force every site; nothing here is found by grep.

**The declarations, and why each is a declaration rather than a derived value:**

| declaration | why btctax cannot answer it |
|---|---|
| valid SSN, per person | spec F-3 — *"valid for employment and … issued by the SSA before the due date"*. Neither property is visible to us. |
| SSTB tips | §224(d)(3), **as relaxed by Notice 2025-69** (OQ-3). The prompt must carry the relief or it refuses filers the statute allows. |
| qualified tips ⊆ W-2 box 7 | spec **F-1**, the largest input-surface consequence in the form: the 2025 W-2/1099s *"were not updated to separately identify tips that may qualify"*. Box 7 is a starting point, not the figure. |
| overtime is the FLSA **premium half** | §4.1 trap 1. Invisible on a W-2 and easy to answer wrongly — not double-time's second half, not holiday/weekend premiums absent >40 hours. |
| overtime excludes qualified tips | §4.1 trap 2. No double-dip between Parts II and III; the surface must refuse the same dollars twice, not silently allow it. |
| state-law-only overtime does not qualify | §4.1 trap 3. The entitlement must arise under FLSA §7. |

★ **Prompt wording is the deliverable here, not plumbing** (R-2). A wrong prompt is a wrong return that
every test passes. Each prompt states the condition that permits a *yes* and defaults to the answer that
cannot overstate the deduction — the structural lesson from
`widening-an-exemption-is-never-the-safe-edit`: enumerate the YES-conditions so every omission fails
closed.

Death of a taxpayer/spouse needs **no new collection**: §G-9 landed
`HouseholdHeader::{taxpayer,spouse}_died_during_year` and `Person::date_of_death`, with the
day-before-the-65th-birthday convention in `reaches_65_on`. Part V reuses them at $6,000 per person.

### T4 — compute, transcribed line by line

**The skip branches are the risk, not the arithmetic** (spec F-5):

- Lines **10, 18, 27**: *"If zero or less, enter the amount from line 7 [15, 24] on line 13 [21, 30]"* —
  a jump **past** the phase-out, not a zero.
- Line **33**: *"If zero or less, **enter $6,000 on line 35**"* — a jump that writes a **nonzero
  constant** into a later line. Transcribing this as `-0-` yields **$0 instead of $6,000**: the whole
  senior deduction lost for every filer under the threshold, which is most of them. It happens to agree
  with `max(0, …)` only because 6% × 0 = 0, so a `max(0, …)` transcription passes for the wrong reason
  and breaks if the rate ever moves. Pin the branch itself.

Filing-status bars are transcribed, not inferred (S-5): Parts II, III and V print *"If married, you must
file jointly to claim this deduction"* ⇒ **zero for MFS**; Part IV prints no such caution ⇒ **allowed for
MFS**, which is adjudicated against the form over OTS 2025 (which bars all four — a witness, not the
authority).

### T5 — wiring, and the below-the-line invariant

`L38 → 1040 L13b` (the `AbsoluteReturn::schedule_1a_additional` seam B2 already threaded);
`L37 → Form 6251 line 1a` — the **senior subtotal**, not the total (parent D-3). File Schedule 1-A only
when `L38 > 0`.

★ **Spec §5.6b is the cheapest high-value guard in the whole branch and it is written here, not left to
T6.** These deductions sit **below** the AGI line, so every AGI-keyed quantity must be **byte-identical**
with and without a Schedule 1-A deduction: Form 8960's NIIT MAGI, Schedule A's 7.5% medical floor, the
§164(b) SALT phase-down MAGI, and the IRA/student-loan phase-outs. Assert it directly — it is the exact
guard against wiring a below-the-line deduction into an above-the-line consumer, and the compiler cannot
catch that class at all.

### T6 — tests

Beyond each task's own: the recon's four worked examples ((b) $2,300, (c) $5,000, (d) two-senior MFJ at
MAGI $200,000 ⇒ L37 $6,000 / $3,000 each, proving S-4's 12¢-per-$1 aggregate slope); all five filing
statuses (S-3's per-return caps and S-5's MFS bar are status-dependent); `L38 > 0` gates filing; `L37`
(not `L38`) reaches Form 6251 line 1a.

**Mutation-verify every guard.** Two of the previous session's guards were vacuous until a mutation said
so, both because every term in them was zero — so for each guard, ask first *which real input axis it
cannot express* (the parametrization lesson from the AMT sweep, where pinning the base standard deduction
hid the §63(f) add-back direction entirely).

### T7 — the two-oracle census, per part

Disqualifications **computed and sized**, never a name list (the shape `verify_f6251.py` converged on).
Known going in: **OTS 2025's Part IV is defective three ways** and **taxcalc has the wrong QSS
threshold** (parent D-8), so **QSS Part IV inside the phase-out band ships zero-oracle** — and the census
must *say so per vector* rather than printing two independent "OK" lines. That failure mode is on the
record: for MFS AMT, two separately-disqualified oracles agreed and left three vectors with no witness at
all, and only a per-vector witness count found it.

Where the oracles cannot adjudicate, the **form** does, and the citation goes in the code (the standing
policy: we are literally told what to do on the form or in its instructions).

---

## 3. Out of scope for B3

- **The PDF and AcroForm map**, including the VIN's per-character comb boxes — parent **B4** owns form
  assets (`recon/fable/05-ty2025-field-maps.md` has the extracted field names). R-4 stands: the generic
  PII scanner does not cover VINs, so B4's emitter tests assert no VIN-shaped literal in fixtures.
- **`FullReturnParams` for TY2025** — lands after B4, and only then does the fail-closed gate come out.
- **Schedule 8812**, which shares the MAGI and the ceil but is its own work (§3).
- **Optimizer changes.** The phase-out bands add hidden marginal-rate adders, so per-$1 what-ifs show $0
  then a cliff. **Document it; do not "fix" it** — the step function is the law (§3).

---

## 4. Risks carried from the spec

R-1 the rounding asymmetry (T1 answers it); R-2 input definitions with no source document (T3, and the
reason prompt wording is the deliverable); R-3 Part IV's weak oracle coverage (T7 states it rather than
papering over it); R-4 the VIN as a new class of filed data (B4); R-5 expiry after TY2028 (T1).
