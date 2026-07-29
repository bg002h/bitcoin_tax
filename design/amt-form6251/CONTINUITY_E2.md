# CONTINUITY — AMT / Form 6251, resuming at E2

**Written 2026-07-29.** Read this first; it is written for a reader with **no prior context**.
`main` is green and pushed; this document is itself the most recent commit on it. Nothing is in
flight and no branch is open — confirm with `git log --oneline -3` and `git status`.

---

## 1. Where the product is

**Tier 1 shipped as v0.14.0** (10 crates on crates.io, GitHub release live). btctax transcribes
Form 6251 line by line (`crates/btctax-core/src/tax/form6251.rs`), **computes it for every return**,
and **refuses to file** when i6251's *Who Must File* condition 1 holds — line 7 > line 10 — because
v1 computes the form but cannot attach it.

**Tier 2 removes that refusal**: fill and attach Form 6251 for filers who actually owe AMT.
It is **NOT started**, and must not start until the entry criteria below are met.

### ★ Why the gate is real, in one concrete sentence

Right now the fixture has **exactly one** vector with AMT > 0 witnessed by both oracles (V6), and
**no Single or HoH vector at all**. Tier 2's whole job is to produce a Form 6251 the filer **signs
under §6065 penalties of perjury**. So starting Tier 2 before E2/E3b means attaching a signed form for
a filing status with **zero** AMT-owing test coverage. That is the gate — not process ceremony.

**The rule, stated so it can be checked:** *btctax may not ATTACH a Form 6251 for any filing status
that has no AMT-owing vector agreed by two oracles (or by one oracle plus a named disqualification).*
If a status cannot reach that bar, restrict the shipped claim for that status rather than attaching
anyway.

---

## 2. The gate: `FOLLOWUPS.md` §G-6b

A Fable consult (2026-07-29) set six falsifiable entry criteria. Current status:

| | criterion | status |
|---|---|---|
| **E1** | compare every non-echo Form 6251 line OTS prints | **DONE** — 247 lines, 0 unexpected |
| **E3a** | Single/HoH tables in `f6251_reference.py` | **DONE**, regression-clean |
| **E3b** | Single/HoH **vectors** | **OPEN** |
| **E2** | a population of two-oracle AMT-owing vectors across every Part III routing | **OPEN ← RESUME HERE** |
| **E4** | read the filled `f6251.pdf` back field-by-field vs the struct | open — Tier-2 build |
| **E5** | lift `gen_goldens.py`'s D-2 for itemizing AMT households | open — Tier-2 build |
| **E6** | lines 2c–2t as real provenance-carrying fields | open — Tier-2 build |

**G-6 stays OPEN.** Do not close it on any of the merges above.

---

## 3. RESUME HERE — what E2 actually requires

Today the fixture has **exactly one** vector (V6) with AMT > 0 witnessed by **both** oracles.
E2 needs a *population*, not a point. Add itemizing AMT-owing vectors covering each live Part III
routing, per filing status:

- line 23 > 0 vs line 23 = 0
- the line-32 skip taken vs not taken
- the 26% side vs the 28% side of §55(b)(1)
- exemption phase-out active vs not
- line 8 live (a foreign tax credit) **with line 7 > line 10**

**Prefer ITEMIZING vectors.** That is not cosmetic: Tax-Calculator is a fully valid second oracle for
itemizing AMT filers, and is defective only for standard-deduction ones (see §5). An itemizing vector
gets you two oracles; a standard-deduction one gets you one plus a form citation.

### The workflow that works (verified today)

1. Construct inputs; compute expected values with `design/amt-form6251/f6251_reference.py`
   (`form6251(...)`; note `qdcgt()` returns a **tuple** `(tax, line5)` — unpack it).
2. Cross-check against **both** oracles before committing the vector. A worked example for a Single
   itemizing AMT-owing case is in commit `761dbf4`'s message — all three engines agreed to the dollar.
3. Add to `crates/btctax-core/src/tax/fixtures/form6251_vectors.json`.
4. Run `OTS_DIR=… .venv/bin/python scripts/oracle/verify_f6251.py` — must stay 0 unexpected.
5. `make check` must stay green (the vector-count assertion in `form6251.rs` may need bumping).

---

## 4. Environment — non-obvious, cost real time today

- **Python lives in the repo's `.venv`.** `.venv/bin/python` has taxcalc 6.7.2 + pandas.
  Bare `python3` has NEITHER and always will not.
- **OpenTaxSolver is installed** at `~/OpenTaxSolver2024_22.07_linux64`. Export `OTS_DIR` to it.
  Without `OTS_DIR` the probe prints a loud SKIP and validates against one oracle only.
- **OTS filing-status tokens are its own**: `Single`, `Married/Joint`, `Married/Sep`,
  `Head_of_House`, `Widow(er)`. A wrong token yields **all zeros rather than an error** — this looks
  exactly like a broken install.
- **The shell is fish.** `for x in "cmd --flag"; $x; end` gives `rc=127` — fish does not word-split
  variables. Run gates individually.
- **Never read a gate's exit code through a pipe.** `make check | grep …` gives you *grep's* status.
  Run `make check >/dev/null 2>&1; echo $?`. `make check` runs nextest and clippy concurrently, so a
  green test summary proves nothing about clippy.
- **Five gates, not one:** `make check` · `cargo fmt --all --check` ·
  `cargo +1.88 check --workspace --locked` · `cargo run -p xtask -- check-isolation` ·
  `bash scripts/pii-scan-generic.sh` (this one scans **HEAD**, not the worktree — commit first).

---

## 5. Oracle facts — established by reading source, not assumed

**Both oracles work on AMT. An earlier claim that OTS "computes no Form 6251 at all" was FALSE** and
was asserted without installing it. `taxsolve_US_1040_2024.c:222` is
`form6251_AlternativeMinimumTax()`; it prints the whole form as `AMT_Form_6251_L*` and sets `L[17]`.

| oracle | valid for | defective for |
|---|---|---|
| **OpenTaxSolver 2024** (v22.07) | everything else, incl. all of Part III | **MFS with line 4 above the stale 2023 threshold** (`:270-275` carries 831,150/1,084,150/63,250); **cash gifts above the §170(b) 60%-of-AGI ceiling** (2024 Schedule A applies no cap) |
| **Tax-Calculator 6.7.2** | **itemizing** AMT filers — exact agreement | **standard-deduction** filers: AMTI omits Form 6251 line 2a's add-back (PSLmodels/Tax-Calculator#3108, open) |

Both OTS defects are **already fixed in OTS 2025 v23.06** — do **not** report them upstream, and do
**not** patch OTS locally (observe-only posture; the form's own worked example is the better anchor).
`ots_direct.py` gates OTS to `None` ("not witnessed") on households its defects reach.

**#3108** was filed on one oracle, against our own two-oracle rule. It has since been corroborated by
OTS and a follow-up comment posted:
<https://github.com/PSLmodels/Tax-Calculator/issues/3108#issuecomment-5119300474>. That comment also
**corrects an error we published** — we wrote §55(d)(3) "appears not to be modelled"; taxcalc does
model it (`AMT_em_pe` = 875,950, guard at `calcfunctions.py:2590`). One grep would have caught it.

---

## 6. Two findings that change how Tier 2 must be BUILT

**(a) Equivalence questions do not survive into attach mode.** Refusal-by-declaration is sound only
when the question is a fact the filer can verify — an item's *existence*.
`QuestionId::AmtDepreciationSameAsRegular` currently asks the filer to affirm an AMT-technical
*equivalence* they cannot evaluate; they will guess "yes" and btctax will print a signed 0 resting on
the guess. **Before attach, re-phrase to existence**: *"does your Schedule C include any depreciation
or §179 deduction?"* → yes ⇒ refuse. Same for any sibling declaration.

**(b) The most likely wrong filed number is an ISO exercise (line 2m) printed as 0 on a signed form.**
It is invisible to the entire validation apparatus — no oracle can witness an input btctax never
collects, no vector can encode it, no transcription test reds — and post-TCJA it is the dominant
real-world reason an individual owes AMT, in exactly the population Tier 2 is enriched for.
"Unreachable at the input surface" quietly conflates *btctax cannot see it* with *the filer does not
have it*. Cheapest catch: an existence interview over i6251's Who-Must-File Exception items (ISO,
§1202, §4952, NOL, Form 8801, accelerated depreciation — PAB is already refused), any "yes" refusing,
plus a test asserting every 2c–2t field is non-silent before emit.

---

## 7. Framing corrections — do not re-adopt the pessimistic version

1. **"Both oracles are blind in Tier 2's region"** — overstated. The doubly-dark region is roughly
   standard-deduction MFS-kicker filers, which is narrow. Two oracles are *achievable* for the
   itemizing slice.
2. **"The corpus structurally cannot contain an AMT filer"** — true today, **contingent tomorrow**.
   Both rejection legs are ours: one *is* the Tier-2 change, and D-2 is a predicate we wrote. Corpus
   admission is a Tier-2 exit criterion (E5), not an impossibility.
3. **The fixture is a bridge, not the permanent instrument.** Its Part III interior was circular until
   E1; the corpus (E5) is the ongoing guarantee.

---

## 8. Coverage map, measured

- btctax emits **41** Form 6251 lines; OTS witnesses **37**; **36** comparable.
- **No independent witness anywhere:** `line3`, `line14`, `line35`, `line36`, `line37`.
- **Modelled by neither** (btctax stubs to 0; OTS never prints): **lines 2c–2t**, all 18. This is E6.
- Corpus: **0 of 104** admitted households have nonzero AMT on either oracle.
- Fixture gaps beyond status: no qualified-dividends-distinct-from-LTCG vector, no QBI vector.
