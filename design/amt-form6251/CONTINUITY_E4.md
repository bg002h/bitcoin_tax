# CONTINUITY — AMT / Form 6251, resuming at E4

**Written 2026-07-29** (supersedes the E2 edition; E2 and E3b are now done). Read this first; it is
written for a reader with **no prior context**. Confirm the tree with `git log --oneline -3` and
`git status`.

---

## 1. Where the product is

**Tier 1 shipped as v0.14.0** (10 crates on crates.io, GitHub release live). btctax transcribes
Form 6251 line by line (`crates/btctax-core/src/tax/form6251.rs`), **computes it for every return**,
and **refuses to file** when i6251's *Who Must File* condition 1 holds — line 7 > line 10 — because
v1 computes the form but cannot attach it.

**Tier 2 removes that refusal**: fill and attach Form 6251 for filers who actually owe AMT.
It is **NOT started**. The evidence gate is now met; the build criteria (E4/E5/E6) are not.

### ★ The gate, and why it is now satisfied

**The rule, stated so it can be checked:** *btctax may not ATTACH a Form 6251 for any filing status
that has no AMT-owing vector agreed by two oracles (or by one oracle plus a named disqualification).*

That rule is no longer prose. `scripts/oracle/verify_f6251.py` ends with a **witness census** that
computes it and **fails the run** if any filing status in the fixture loses its last two-oracle
AMT-owing vector. Current state — all four clear it:

| status | AMT-owing vectors agreed by BOTH oracles |
|---|---|
| single | 6 — V11 V12 V13 V14 V27 V29 |
| hoh | 4 — V15 V16 V17 V18 |
| mfj | 5 — V6 V19 V20 V21 V26 |
| mfs | 1 — V22 |

---

## 2. The gate: `FOLLOWUPS.md` §G-6b

| | criterion | status |
|---|---|---|
| **E1** | compare every non-echo Form 6251 line OTS prints | **DONE** — 730 lines now, 0 unexpected |
| **E3a** | Single/HoH tables in `f6251_reference.py` | **DONE** |
| **E3b** | Single/HoH **vectors** | **DONE** — V11–V18, V27, V29 |
| **E2** | a population of two-oracle AMT-owing vectors across every Part III routing | **DONE** — 22 of 30 vectors owe AMT; 3 routings proved dead |
| **E4** | read the filled `f6251.pdf` back field-by-field vs the struct | **OPEN ← RESUME HERE** |
| **E5** | lift `gen_goldens.py`'s D-2 for itemizing AMT households | open — Tier-2 build |
| **E6** | lines 2c–2t as real provenance-carrying fields | open — Tier-2 build |
| **G-6c** | report taxcalc's missing §55(d)(3) MFS AMTI add-back upstream | open — Tier-2, after checking the latest release |

**G-6 stays OPEN.** Do not close it on any of the merges above.

---

## 3. What E2 established (do not re-derive these)

**Three of the five routings E2 asked for cannot own AMT at all.** A ~450M-point scan found zero
AMT-owing returns taking the line-32 skip, or with line 39 on the 26% side, or with the exemption
un-phased-out. They are one fact: *AMT is owed only once the §55(d)(3) phase-out has begun.* It is
pinned by `amt_is_owed_only_once_the_exemption_phaseout_has_begun`, which sweeps all four statuses
against the production regular tax and reds if the input surface widens. **Full detail in
`FOLLOWUPS.md` §G-6b → "E2's findings".**

**One region has NO oracle.** For MFS, §55(d)(3) puts the zero-exemption threshold and the line-4
kicker start at the same $875,950, so "exemption gone" and "kicker live" are one condition — and both
engines fail there (OTS: stale 2023 constants; taxcalc: models the exemption cliff via `AMT_em_pe`,
not the AMTI add-back). V23/V24/V25 owe AMT with **zero** witnesses; the census prints them every run.
**Tier 2 must not treat those three as validated figures.**

---

## 4. RESUME HERE — E4

Read the FILLED `f6251.pdf` back field-by-field against the `Form6251` struct, plus the Σround/roundΣ
residual rules on the 6251 → Schedule 2 → 1040 L17 chain. A perfect computation still files a wrong
number through a transposed AcroForm field, and the oracle sweep already reads other forms off the
PDF — that machinery is the pattern to copy.

The struct is now the right shape for it: all **41** lines are under KAT for all **30** vectors, and
the line list is closed at both ends (a fixture line the test forgets to look at fails the test), so
E4's mapping has a complete, trustworthy source to map FROM.

---

## 5. Environment — non-obvious, costs real time

- **Python lives in the repo's `.venv`.** `.venv/bin/python` has taxcalc 6.7.2 + pandas + numpy.
  Bare `python3` has NONE of them and always will not.
- **OpenTaxSolver is installed** at `~/OpenTaxSolver2024_22.07_linux64`. Export `OTS_DIR` to it.
  Without it the probe prints a loud SKIP, and the witness census reports INCONCLUSIVE rather than
  passing the gate on one oracle.
- **OTS filing-status tokens are its own**: `Single`, `Married/Joint`, `Married/Sep`,
  `Head_of_House`, `Widow(er)`. A wrong token yields **all zeros rather than an error** — this looks
  exactly like a broken install.
- **`__pycache__` will lie to you.** Restoring `f6251_reference.py` from a backup left a mutated
  `.pyc` in play, and a mutation test "passed" against code that was no longer there. `rm -rf
  design/amt-form6251/__pycache__` after any restore.
- **The shell is fish.** `for x in "cmd --flag"; $x; end` gives `rc=127` — fish does not word-split
  variables. Run gates individually.
- **Never read a gate's exit code through a pipe.** `make check | grep …` gives you *grep's* status.
  Run `make check >/dev/null 2>&1; echo $?`. `make check` runs nextest and clippy concurrently, so a
  green test summary proves nothing about clippy — that is exactly how this branch's first run
  reported 2422/2422 passing while clippy was failing.
- **Five gates, not one:** `make check` · `cargo fmt --all --check` ·
  `cargo +1.88 check --workspace --locked` · `cargo run -p xtask -- check-isolation` ·
  `bash scripts/pii-scan-generic.sh` (this one scans **HEAD**, not the worktree — commit first).

---

## 6. Oracle facts — established by reading source, not assumed

| oracle | valid for | defective for |
|---|---|---|
| **OpenTaxSolver 2024** (v22.07) | everything else, incl. all of Part III | **MFS with line 4 above the stale 2023 threshold** (`taxsolve_US_1040_2024.c:270-275`); **cash gifts above the §170(b) 60%-of-AGI ceiling** |
| **Tax-Calculator 6.7.2** | **itemizing** AMT filers — exact agreement | **standard-deduction** filers: AMTI omits line 2a's add-back (#3108, open); **MFS above $875,950**: omits line 4's §55(d)(3) AMTI add-back |

Both OTS defects are **already fixed in OTS 2025 v23.06** — do **not** report them upstream, and do
**not** patch OTS locally (observe-only posture; the form's own worked example is the better anchor).

**Neither excuse list is maintained by hand any more.** OTS's is
`ots_direct._ots_amt_disqualified` (fail-closed); taxcalc's is `_taxcalc_expected_gaps`, which names
each omission's exact SIZE, so a divergence of the wrong shape is reported as unexpected even on a
vector expected to diverge. Both changed because their hand-kept predecessors went stale the moment
E2 added vectors: OTS's `{V8, V2b}` missed V23–V25, and the Tax-Table list `{("V10","line10")}`
missed six lines across V12/V16/V19. **A name list of excused vectors is a liability in this file.**

★ A ONE-ORACLE CLAIM HAS COST US BEFORE. #3108 was filed on one oracle against our own rule and
contained a claim that was wrong. Before filing G-6c: grep their source, run the second oracle, check
the LATEST release.

---

## 7. Two findings that change how Tier 2 must be BUILT

**(a) Equivalence questions do not survive into attach mode.**
`QuestionId::AmtDepreciationSameAsRegular` asks the filer to affirm an AMT-technical *equivalence*
they cannot evaluate; they will guess "yes" and btctax will print a signed 0 resting on the guess.
**Before attach, re-phrase to existence**: *"does your Schedule C include any depreciation or §179
deduction?"* → yes ⇒ refuse. Same for any sibling declaration.

**(b) The most likely wrong filed number is an ISO exercise (line 2i) printed as 0 on a signed form.**
Invisible to the entire validation apparatus — no oracle can witness an input btctax never collects,
no vector can encode it, no transcription test reds. Cheapest catch: an existence interview over
i6251's Who-Must-File Exception items (ISO, §1202, §4952, NOL, Form 8801, accelerated depreciation —
PAB is already refused), any "yes" refusing, plus a test asserting every 2c–2t field is non-silent
before emit. **This is also what would make E2's three dead routings live** — see §3.

---

## 8. Coverage map, measured

- The fixture holds **30** vectors; **22** owe AMT (was 3). All four filing statuses present.
- btctax emits **41** Form 6251 lines; **all 41 are under KAT for all 30 vectors**, with the line
  list closed at both ends.
- OTS witnessed **730** lines this run, 0 unexpected, 7 methodology (Tax-Table quantization),
  5 vectors disqualified.
- **No independent witness anywhere:** `line3`, `line14`, `line35`, `line36`, `line37`.
- **Modelled by neither** (btctax stubs to 0; OTS never prints): **lines 2c–2t**, all 18. This is E6.
- Corpus: **0 of 104** admitted households have nonzero AMT on either oracle. This is E5.
- Fixture gaps beyond status: no qualified-dividends-distinct-from-LTCG vector, no QBI vector, and
  **no vector with Schedule A line 7 (taxes) > 0** — every itemizer here deducts a cash gift only, so
  the fixture exercises line 2a's itemizer limb only at zero. That limb is *not* untested: the
  original shipped bug's regression KAT,
  `amt::tests::itemizer_addback_is_schedule_a_line7_not_the_itemized_total`, drives it with a nonzero
  SALT, and `return_1040.rs:1439` wires `salt_5e` into it. What is missing is a line-7-live vector
  carried end to end through both oracles. E4 is the natural place.
