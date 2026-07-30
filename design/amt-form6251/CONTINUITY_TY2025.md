# CONTINUITY — AMT / Form 6251: E4 is PARKED, the next move is TY2025

**Written 2026-07-29** (supersedes the E2 and E4 editions). Read this first; it is written for a
reader with **no prior context**. Confirm the tree with `git log --oneline -5` and `git status`.

> **The one-line version.** Tier 2 (attach Form 6251) is gated on E4/E5/E6. We are **not** doing E4
> next. We are adding **TY2025** first, because that is what makes the last unwitnessed region of the
> AMT computation witnessable at all. **TY2026 fails closed, by decision, with a test holding it.**
>
> ## ★ PROGRESS SINCE THE PIVOT (read this before starting anything)
>
> Branch `feat/amt-e2-vector-population`, all five gates green, TY2024 provably unchanged throughout
> (2435 tests, golden-matrix md5 `c4e1853ed82d113ca5cd97ffd8abbf47` unmoved, both oracles exit 0 on every
> commit — the sweep at seed 7 / 40 households reports 0 undeclared divergences).
>
> ★★ **A LIVE DEFECT WAS FOUND AND FIXED MID-BRANCH: `FOLLOWUPS.md` §G-9** (HEAD `dd3d42f`). The
> §63(f) age-65 box on 1040 line 12a was decided from the date of BIRTH alone, but i1040gi carves out a
> person who died in-year before reaching 65 — so a spouse who died at 64 was granted a $1,550 addition
> they were not entitled to, **understating tax**, in shipped v0.14.0. **Neither oracle can catch it**
> (OTS takes a filer-answered `"You_65+Over?"`; taxcalc has only `age_spouse`), so every gate stayed
> green. Fixed as two class-(A) gates on `HouseholdHeader` plus two class-(B) dates on `Person`, five
> mutations killed. **How it was found is the transferable part:** it came out of a SPEC review of
> TY2025 Part V, and checking whether the same rule touched TY2024 took one `grep` of an already-
> archived PDF. Residue filed as **§G-9a** (the blind boxes, same owning phase).
>
> | branch | state |
> |---|---|
> | **B1** harness year seams | **DONE** — 7 seams. `OTS_YEAR` split from `OTS_DIR`; `_ots_amt_disqualified` year-scoped (★ the one that mattered: left year-blind it would have gated TY2025's MFS vectors against a solver that handles them CORRECTLY, killing the pivot silently); `gen_goldens` MARS map made total; `exact=1` TY2025-only; `corpus.py`'s SALT axis year-keyed with a straddle guard; the OTS `S1A_2a` skip guard; per-year standard deduction. **4 of the 7 would have failed silently with everything green.** |
> | **B2** numbers + form shapes | **DONE** except TY2025 `FullReturnParams` itself. The TY2025 fail-closed gate landed FIRST (§5.0). `salt_cap: Usd` → `salt: SaltLimitation` enum (`FlatCap` / `Worksheet2025`). MAGI add-backs collected through the whole input stack. Form 6251 Part I is a per-year TYPE (`Form6251Line1::Y2024/Y2025`). 1040 `L13b` threaded into L14 and Form 8995 line 11. |
> | **B3** Schedule 1-A | **SPEC r3 GREEN + PLAN r1 written, NOT built** — `SPEC_schedule_1a.md` (0C/0I; §7's last two open questions CLOSED against the instructions, §7a folds five extracted facts F-1…F-5) and `IMPLEMENTATION_PLAN_schedule_1a.md` (7 tasks, T1-T2 chokepoints). Plan is in its independent review round. |
> | **B4** filing assets + corpus | not started. 12 PDFs/maps + 2 partial 2025 maps rebuilt. |
>
> ★ **TY2025 `FullReturnParams` MUST NOT land until B3 is built.** The gate
> `ty2025_full_return_must_stay_fail_closed_until_complete` names four conditions and Schedule 1-A is
> one of them; bundling params early does not refuse, it emits plausible wrong numbers.
>
> ★ **Every constant TY2025 needs already has a value and a citation** (parent SPEC §5.1). Nothing is
> outstanding there — three of the four fields once listed as "still a lookup" were printed in
> `i1040gi--2025.pdf`, a document already committed to this repo. **Grep the archive before deferring.**

---

## 1. Where the product is

**Tier 1 shipped as v0.14.0** (10 crates on crates.io, GitHub release live). btctax transcribes
Form 6251 line by line (`crates/btctax-core/src/tax/form6251.rs`), **computes it for every return**,
and **refuses to file** when i6251's *Who Must File* condition 1 holds — line 7 > line 10 — because
v1 computes the form but cannot attach it. **Tier 2 removes that refusal. It is NOT started.**

btctax supports **TY2024 only** for the full return. `TaxTable` (brackets, §1(h) breakpoints, gift
exclusion, SS wage base) covers 2017/2024/2025/2026, but `FullReturnParams` — the struct carrying
`AmtParams` — exists for 2024 alone, and `full_return_for(year)` returns `Option`, so every other
year fails closed at the CLI.

---

## 2. The gate: `FOLLOWUPS.md` §G-6b

| | criterion | status |
|---|---|---|
| **E1** | compare every non-echo Form 6251 line OTS prints | **DONE** — 730 lines, 0 unexpected |
| **E2** | a population of two-oracle AMT-owing vectors across every Part III routing | **DONE** — 22 of 30 vectors owe AMT; 3 routings proved dead |
| **E3a/E3b** | Single/HoH reference tables, then vectors | **DONE** — V11–V18, V27, V29 |
| **E4** | read the filled `f6251.pdf` back field-by-field vs the struct | **open — PARKED behind TY2025** |
| **E5** | lift `gen_goldens.py`'s D-2 for itemizing AMT households | open — Tier-2 build |
| **E6** | lines 2c–2t as real provenance-carrying fields | open — Tier-2 build |
| **G-6c** | report taxcalc's missing §55(d)(3) MFS AMTI add-back upstream | open — after checking the latest release |
| **G-6d** | no fixture vector has Schedule A line 7 > 0 | open — Tier-2 · E4 |

**G-6 stays OPEN.** Do not close it on any merge described here.

---

## 3. Why TY2025 comes before E4

E2 closed with one region genuinely unwitnessed: **MFS with the exemption phased to zero.** For MFS,
§55(d)(3) puts the zero-exemption threshold and the Form 6251 line-4 kicker start at the *same*
amount — this is definitional, not a coincidence of one year's constants, because the statute's
flush sentence defines the kicker start as "the minimum amount of such income for which the exemption
amount under paragraph (1)(C) is zero". And that point is exactly where **both** TY2024 oracles fail:

| engine | failure at the MFS kicker, TY2024 |
|---|---|
| OpenTaxSolver 2024 | implements the add-back with the **stale 2023** constants (831,150 / 1,084,150 / 63,250) — `taxsolve_US_1040_2024.c:270-275` |
| Tax-Calculator 6.7.2 | does not model the add-back at all; its `c62100` block has no MFS branch. It *does* model the other limb, the exemption cliff (`AMT_em_pe`, `calcfunctions.py:2590`) — state this narrowly, we have published an over-broad version of it before |

Vectors V23/V24/V25 therefore owe AMT with **zero** witnesses, and `verify_f6251.py`'s witness census
prints them as such every run.

**The fix is not a better TY2024 oracle — it is TY2025.** OTS 2024's defect is a stale-constant bug
in one year's solver, not blindness to the rule. **OTS 2025 implements it correctly**, so building
the same MFS vectors at TY2025 gets the rule witnessed. The TY2024 *constants* stay anchored where
they belong: on the 2024 form and i6251's own worked example.

---

## 4. TY2025 — established from primary sources, do NOT re-derive

Archived here: **`f6251--2025.pdf`** and **`i6251--2025.pdf`** (IRS **final**, fetched 2026-07-29).
Every figure below was read from those text layers with `pdftotext -layout`, and independently
matches OpenTaxSolver 2025's source.

| | single/hoh | mfj/qss | mfs |
|---|---|---|---|
| exemption (line 5) | 88,100 | 137,000 | 68,500 |
| phase-out start | 626,350 | 1,252,700 | 626,350 |
| zero-exemption | 978,750 | 1,800,700 | **900,350** |
| 26/28% breakpoint (lines 7/18/39) | 239,100 | 239,100 | 119,550 |
| 28% subtrahend | 4,782 | 4,782 | 2,391 |
| line 19 (0%-band top) | 48,350 / hoh 64,750 | 96,700 | 48,350 |
| line 25 (15%-band top) | single 533,400 / hoh 566,700 | 600,050 | 300,000 |

- **Phase-out rate is still 25%** in 2025 (every row satisfies `zero-exemption = start + 4 × exemption`).
- **MFS kicker:** starts at **900,350**, flat **+68,500** at or above **1,174,350**, else 25% of the
  excess. Note 900,350 is *also* the MFS zero-exemption threshold — the collision, in 2025 too.
- **A fresh KAT is written for us** by i6251 2025 p.9, verbatim: *"if the amount on line 4 is
  $920,350, enter $925,350 instead—the additional $5,000 is 25% of $20,000."*

### OpenTaxSolver 2025 — installed and PROVEN on the dark region

Installed at **`~/OpenTaxSolver2025_23.06_linux64`** (SourceForge `OTS_2025/v23.06_linux`,
sha256 `0cc7e540…5e6b`). Binaries ship prebuilt; `bin/taxsolve_US_1040_2025` runs. Its constants are
genuinely 2025 and internally consistent, and it was smoke-tested end to end on an MFS household
deliberately above the kicker start:

```
MFS, wages 260,000, LTCG 700,000, cash gift 25,000  ->  taxable income 935,000
OTS 2025 line 4 = 943,662.50  ==  935,000 + 0.25 x (935,000 - 900,350)     ✔ kicker applied
OTS 2025 line 5 = 0           (exemption zeroed above 900,350)             ✔
OTS 2025 line 11 = 13,571.50, and 1040 L17 = 13,571.50                     ✔ 30 6251 lines printed
```

**That is the proof the pivot rests on.** The region with no oracle in TY2024 has a working one in
TY2025. Tax-Calculator also covers 2025 — with both of its AMT defects intact, so it stays a
*named-disqualification* witness there, which our own gate rule accepts.

---

## 5. TY2026 — FAILS CLOSED, by decision

**Decision (2026-07-29): do not implement TY2026 full-return support.** It is held by
`ty2026_full_return_must_stay_fail_closed` in `crates/btctax-adapters/src/tax_tables.rs`, which is
mutation-verified (bundling a 2026 table reds it). **Adding TY2025 will delete the `2025` assertion
beside it; 2026 must survive that.** Three independent reasons, each sufficient:

1. **The 2026 instructions do not exist.** The draft form's line 4 says *"more than $640,200, see
   instructions"* — and `irs.gov/pub/irs-dft/i6251--dft.pdf` is still the **2025** instructions. So
   the phase-out rate, the zero-exemption thresholds, and the kicker's rate and cap are unpublished.
   They can be *inferred* — 500,000 + 2 × 70,100 = 640,200 implies the rate moved to **50%** — but an
   inferred constant is precisely what `CLAUDE.md` forbids encoding.
2. **The form was restructured, not re-parameterized.** 2026 splits line 1 into `1a`/`1b`, where 1a
   subtracts **Schedule 1-A (Form 1040), line 43** — a new OBBBA schedule btctax has no surface for —
   and line 4 reads "Combine lines **1b** through 3". `Form6251` transcribes the 2024 layout.
   ★ **This is NOT a 2026-only reason — see §6a. TY2025 is restructured too.** It is listed here
   because it compounds with (1) and (3): for 2026 we would be re-transcribing a form whose
   instructions do not exist, against no oracle.
3. **No oracle covers 2026.** OpenTaxSolver's newest release is `OTS_2025`; there is no OTS 2026.
   Tax-Calculator reaches 2036 but carries both AMT defects, so a 2026 figure would rest on one
   engine we already know is wrong there.

Archived as **`f6251--2026-DRAFT.pdf`** (sha256 `a547fc9d…b5ac`) — drafts get replaced silently, and
this one is the evidence for the decision. **It is a draft: "NOT FOR FILING". Do not transcribe
constants from it into `AmtParams`.**

---

## 6a. ★★ TY2025 IS ALSO A RE-TRANSCRIPTION — OBBBA restructured the 2025 return

**Correction to an earlier draft of this document, which implied TY2025 was constants-plus-plumbing.
It is not.** Established 2026-07-29 from the archived IRS finals:

**The 2025 Form 1040 changed shape.** Line **11b** is the AGI line taxable income is measured from,
line **12e** is the deduction line, and line **13b** is *"Additional deductions from Schedule 1-A,
line 38"* — a new OBBBA schedule.

**The 2025 Form 6251 Part I changed with it**, exactly as the 2026 draft does:

| | 2024 (what `Form6251` transcribes) | 2025 |
|---|---|---|
| line 1 | "…from Form 1040 line **15**…" | **1a** "Subtract Schedule 1-A line **37** from Form 1040 line **14**" |
| | | **1b** "Subtract line 1a from Form 1040 line **11b**" |
| line 2a | "…otherwise, the amount from line **12**" | "…otherwise, the amount from line **12e**" |

★ **Read line 1a carefully — it subtracts Schedule 1-A line 37, which is the *senior* deduction
alone (Part V, "Enhanced Deduction for Seniors"), not line 38, the total.** So for the AMT, the
enhanced senior deduction is **added back** and the tips / overtime / car-loan deductions are
**allowed**. That is a substantive tax fact, and it is the kind of thing that must be transcribed
rather than inferred.

**Schedule 1-A (2025) has five parts**, each with its own cap and phase-out:

| part | deduction | cap | phase-out |
|---|---|---|---|
| II | No Tax on Tips | 25,000 | $100 per $1,000 of MAGI over 150,000 / 300,000 |
| III | No Tax on Overtime | 12,500 / 25,000 | $100 per $1,000 over 150,000 / 300,000 |
| IV | No Tax on Car Loan Interest | 10,000 | $200 per $1,000 over 100,000 / 200,000 (needs VINs) |
| V | Enhanced Deduction for Seniors | 6,000 per person | 6% of MAGI over 75,000 / 150,000 |

**This is the central open question for the TY2025 spec**, and it is an answered-ness question, not a
computation one: btctax cannot see tips, overtime or car-loan interest, so it cannot know Schedule 1-A
is zero — it can only *assume* it, which is the defect class this project keeps paying for. Three
options, to be settled in the spec:

- **(a) implement Schedule 1-A in full** — five parts, four phase-outs, VIN collection. Disproportionate.
- **(b) existence questions, any "yes" refuses** — cheapest, fail-closed, in character. ★ Note the
  senior deduction is *different*: btctax already collects `date_of_birth` for the §63(f) aged/blind
  boxes, so a 65+ filer is identifiable and their deduction is computable from data already held.
  Refusing every 65+ filer would be a large and avoidable population.
- **(c) collect Schedule 1-A lines 37 and 38 as `ReturnInputs` fields** — the filer computes the
  schedule elsewhere, mirroring how btctax already takes other non-crypto schedule totals.

**Recommendation: (b) for tips/overtime/car-loan + compute Part V**, with (c) as a later widening.

★ **None of this blocks the pivot's purpose.** The AMT-witnessing vectors are wages + LTCG + a cash
gift, so Schedule 1-A is genuinely zero on them: line 1a = line 14, line 1b = taxable income, and
Part I reduces to the 2024 shape. The MFS kicker can be witnessed against OTS 2025 regardless of how
Schedule 1-A is resolved.

---

## 6. ★ A latent defect found during this recon — fix it before TY2025

`AmtParams.phaseout_rate` is used for **two statutorily distinct rates**:

- `form6251.rs:285` — the §55(d)(3) exemption phase-out
- `form6251.rs:279` — the MFS line-4 kicker (`(amt.phaseout_rate * excess).min(amt.mfs_kicker_max)`)

Both are 25% in 2024 **and** 2025, so the conflation is invisible and every test passes. The 2026
draft's arithmetic implies the phase-out moves to 50% while nothing says the kicker's rate follows.
One field standing for two form rules is the compression pattern `CLAUDE.md` names — and here it
would print a wrong number on a **signed** form with nothing reding.

**Split into `exemption_phaseout_rate` and `mfs_kicker_rate`, both `0.25` today.** A no-op change
that turns a future divergence into a config question instead of a silent defect. Do this first: it
is small, and TY2025 adds a second `AmtParams` literal that would otherwise duplicate the conflation.

---

## 7. What TY2025 actually requires

Roughly, and to be specced properly per `STANDARD_WORKFLOW.md` before any code:

0. **Settle the Schedule 1-A question (§6a) first** — it decides whether TY2025 is a tax year or a
   feature. Everything below assumes Schedule 1-A is resolved one way or another.
1. **`FullReturnParams` for 2025**, including `AmtParams` — transcribed from the archived 2025 form
   and instructions plus Rev. Proc. 2024-40. Every field: `std_deduction` (4 statuses),
   `std_aged_blind_married`/`_unmarried`, `dependent_std_floor`/`_earned_addon`, `salt_cap`
   (★ OBBBA raised this — read it, do not assume 10,000), `kiddie_unearned_threshold`,
   `elective_deferral_limit`, `ftc_ceiling`, `qbi_ti_threshold_unmarried`/`_married`,
   `student_loan_phaseout_unmarried`/`_married`, and `amt`.
1b. **Re-transcribe Form 6251 Part I for 2025** — lines 1a/1b replacing line 1, and line 2a's
   citation moving from 1040 line 12 to line 12e (§6a). `Form6251`'s `line1` field is a 2024 shape.
   Whether that means a per-year struct, a versioned Part I, or a widened field is a spec decision.
2. **A year seam in the OTS driver.** `ots_direct._bin` hardcodes `taxsolve_{form}_2024` (line 76)
   and `_template` looks for `{name}_2024_template.txt` (line 84). Both need the year threaded
   through, and `OTS_DIR` becomes per-year (two installs now coexist).
3. **A `year` field in `form6251_vectors.json`**, which is implicitly TY2024 today, plus per-year
   `params()`/`bps()` in the Rust KAT and per-year params in `verify_f6251.py`.
4. **TY2025 MFS kicker vectors** — the whole point. Build them at TY2025 constants and let OTS 2025
   witness them.
5. **Re-verify the witness census** passes per year, not globally.

---

## 8. Environment — non-obvious, costs real time

- **Python lives in the repo's `.venv`.** `.venv/bin/python` has taxcalc 6.7.2 + pandas + numpy;
  bare `python3` has NONE of them and always will not.
- **TWO OTS installs now coexist:** `~/OpenTaxSolver2024_22.07_linux64` and
  `~/OpenTaxSolver2025_23.06_linux64`. `OTS_DIR` selects one; the driver's year is *separately*
  hardcoded, so pointing `OTS_DIR` at 2025 without patching `_bin` fails with a confusing
  "solver not found".
- **OTS filing-status tokens are its own**: `Single`, `Married/Joint`, `Married/Sep`,
  `Head_of_House`, `Widow(er)`. A wrong token yields **all zeros rather than an error**.
- **`__pycache__` will lie to you.** Restoring `f6251_reference.py` from a backup left a mutated
  `.pyc` in play and a mutation test "passed" against code that was no longer there. `rm -rf
  design/amt-form6251/__pycache__` after any restore.
- **The shell is fish.** `for x in "cmd --flag"; $x; end` gives `rc=127`. Run gates individually.
  Also: a `cd` inside one `Bash` call does not persist, so relative paths like `.venv/bin/python`
  break in the next command — use absolute paths.
- **Never read a gate's exit code through a pipe.** `make check | grep …` gives you *grep's* status.
  `make check >/dev/null 2>&1; echo $?`. nextest and clippy run concurrently, so a green test
  summary proves nothing about clippy — that is how this branch first reported 2422/2422 passing
  while clippy was failing.
- **Five gates, not one:** `make check` · `cargo fmt --all --check` ·
  `cargo +1.88 check --workspace --locked` · `cargo run -p xtask -- check-isolation` ·
  `bash scripts/pii-scan-generic.sh` (this one scans **HEAD** — commit first).

---

## 9. Oracle facts — from source, not assumed

| oracle | valid for | defective for |
|---|---|---|
| **OTS 2024** (v22.07) | everything else, incl. all of Part III | MFS above the stale 2023 threshold; cash gifts above the §170(b) 60%-of-AGI ceiling |
| **OTS 2025** (v23.06) | verified correct on §55(d)(3), incl. the MFS kicker | not yet swept beyond the smoke test |
| **Tax-Calculator 6.7.2** | itemizing AMT filers — exact agreement | standard-deduction filers (line 2a add-back, #3108); MFS above the kicker start (line 4 add-back, unmodelled) |

Both OTS 2024 defects are fixed in the 2025 line — do **not** report them upstream, and do **not**
patch OTS locally (observe-only posture).

**Neither excuse list is maintained by hand.** OTS's is `ots_direct._ots_amt_disqualified`
(fail-closed); taxcalc's is `verify_f6251._taxcalc_expected_gaps`, which names each omission's exact
SIZE so a divergence of the wrong shape is unexpected even on a vector expected to diverge. Both
were rewritten because their hand-kept predecessors went stale the moment E2 added vectors:
`{V8, V2b}` missed V23–V25, and `{("V10","line10")}` missed six lines. **A name list of excused
vectors is a liability in this harness.**

★ **A one-oracle claim has cost us before.** #3108 was filed on one oracle against our own rule and
contained a wrong claim. Before filing G-6c: grep their source, run the second oracle, check the
LATEST release.

---

## 10. Two findings that still change how Tier 2 must be BUILT

**(a) Equivalence questions do not survive into attach mode.**
`QuestionId::AmtDepreciationSameAsRegular` asks the filer to affirm an AMT-technical *equivalence*
they cannot evaluate; they will guess "yes" and btctax will print a signed 0 resting on the guess.
Re-phrase to existence before attach: *"does your Schedule C include any depreciation or §179
deduction?"* → yes ⇒ refuse. Same for any sibling declaration.

**(b) The most likely wrong filed number is an ISO exercise (line 2i) printed as 0.**
Invisible to the entire validation apparatus — no oracle can witness an input btctax never collects,
no vector can encode it, no transcription test reds. Cheapest catch: an existence interview over
i6251's Who-Must-File Exception items (ISO, §1202, §4952, NOL, Form 8801, accelerated depreciation —
PAB is already refused), any "yes" refusing, plus a test asserting every 2c–2t field is non-silent
before emit. **This is also what would make E2's three dead routings live** — see §11.

---

## 11. Coverage, measured (TY2024)

- The fixture holds **30** vectors; **22** owe AMT. All four filing statuses present, all clearing
  the Tier-2 attach gate on two oracles: single 6, hoh 4, mfj 5, mfs 1.
- All **41** Form 6251 lines are under KAT for all 30 vectors, with the line list closed at both
  ends (a fixture line the test forgets to check fails the test).
- **Three Part III routings are unreachable with the form attachable** — the line-32 skip, line 39's
  26% side, and an un-phased-out exemption. One fact: *attachable only once the §55(d)(3) phase-out
  has begun.* Pinned by `amt_is_owed_only_once_the_exemption_phaseout_has_begun`, which sweeps all
  four statuses, both deduction extremes and both FTC extremes against the production regular tax.
  ★ Its sensitivity is **~$4,400 of AMTI**, measured by bisection — *not* "any widening". A ΔAMTI
  moves the tentative tax by only 26–28% of itself. It narrows E6; it does not discharge it.
- **No independent witness anywhere:** lines 3, 14, 35, 36, 37.
- **Modelled by neither oracle:** lines 2c–2t, all 18. That is E6.
- Golden corpus: **0 of 104** admitted households have nonzero AMT on either oracle. That is E5.
- Gaps beyond status: no qualified-dividends-distinct-from-LTCG vector, no QBI vector, and no vector
  with Schedule A line 7 > 0 (G-6d).
