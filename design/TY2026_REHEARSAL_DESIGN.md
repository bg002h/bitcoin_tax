# Stage 2 — the throwaway TY2026 rehearsal. Design.

**Owner, 2026-09-13:** *"do a throw-away test run where we make up what is knowable yet and see what
problems arise"*, then *"design it the way you suggested"* — i.e. **a throwaway by CONSTRUCTION, not by
intention**, with invented values structurally incapable of reaching a bundled year.

**Stage 1** (the derived blocker command) is a separate brief. This document is stage 2 and is written
against what the tree already contains.

---

## 1. ★★ The first finding is that we barely need to invent anything

Measured before designing:

| | state |
|---|---|
| `by_year.insert(2026, ty2026())` — the **TaxTable** | **already bundled** (`tax_tables.rs:79`) |
| `ty2026_full_return()` — the **FullReturnParams** | **exists** (`:246`), **not inserted** |
| its figures | **transcribed from the statute and verified** — FR-212 confirmed the AMT phase-out at `:311`/`:312` |
| TY2026 form templates | nine schedule **DRAFTs** archived; ★ **no `f1040--2026`, no `i1040gi--2026`** (FR-181) |
| the price dataset | ends **2026-06-03** |

So the TY2026 params are **not unknowable — they are unwired.** The rehearsal's core is **one line**:
`by_year.insert(2026, ty2026_full_return())`. That is a very different risk profile from inventing
figures, and it means the contamination surface is one insert rather than a table of fabrications.

★ What genuinely must be faked is smaller and sharply bounded: the **1040 template** (unarchived), the
**price dataset's** tail, and whatever stage 1's command names. Each fake is a **document we do not have**,
never a **figure we made up** — and that distinction is the whole safety argument.

## 2. The guarantee: a CONDITION-BEARING gate, not a flag and not a promise

A feature flag can be flipped; a branch can be merged; an intention can be forgotten. The repo already
owns the right idiom twice over — `ty2025_full_return_must_stay_fail_closed_until_complete`
(`tax_tables.rs:1113`), and the new `state_local_refund` module which *"refuses TY2026 until the January
`i1040gi` is archived, with a test that reds the day it lands."*

**So the gate is:**

> **TY2026 full-return params may be bundled only when the TY2026 1040 AND its instructions are archived.**

- **Today:** `f1040--2026` and `i1040gi--2026` are absent ⇒ the params must not be bundled ⇒ **the rehearsal
  branch reds this test**, so it cannot merge silently.
- **In January:** the documents land ⇒ the gate permits the bundling.

★★★ **CORRECTED 2026-09-13, and by a PLANT rather than a review.** This section originally said the gate
*"stops blocking on its own"* once the documents land. **That is false, and stage 1's plant B proved it:**
copying the 2025 extract into a 2026 filename cleared the archive gap **and left the §111(a) gate shut**,
because `slr::revision_for` reads a **TRANSCRIPTION** while the directory only decides what the *suite*
demands. **Archiving is necessary and not sufficient — someone must transcribe.** So the gate's condition is
`TRANSCRIBED`, not `archived`, and January creates a *new* state rather than clearing one:
**"ARCHIVED but not TRANSCRIBED"**, which `xtask blockers` now derives as its own row and correctly moves
from `January` to `we must build`. ★ The lesson is the one this whole day has been about: *a document
arriving is not the same as a document being read*, and I had written the optimistic version.

★★ That is better than a tripwire in two ways: **nothing has to be deleted later** (a gate you must remember
to remove is a gate that gets removed early), and it encodes *why* the year is not filable rather than
merely *that* it is not. ★ It is also the third instance of this shape in the tree, which is the argument
for using it rather than inventing a fourth mechanism.

**Land the gate on `main` FIRST**, before the rehearsal branch exists, and watch it pass. Then the branch
that flips the insert reds it, visibly, in CI.

## 3. Two tiers, because the walls live in different places

**Tier A — library, unbundled, and it may STAY in the repo.**
Drive compute → printed chain → emitter with `ty2026_full_return()` injected **at the call site**, never
through `by_year`. `testonly::ty2026_table()` is the existing precedent for an unbundled year artifact.
Nothing can leak because nothing enters the bundle. This reaches the compute and emit surfaces — where
FR-218 lived — and is worth keeping as a permanent rehearsal harness.

**Tier B — the CLI journey, bundled, branch-only, never merged.**
`init → import → income import → export-irs-pdf`, the real binary, because that is where the *filer-facing*
walls live: the five from this morning, and the refusals whose exits do not work. This needs the insert, so
it lives on a branch the §2 gate reds. ★ Tier B is what found FR-102 and FR-218's class; Tier A cannot
reach it.

## 4. The deliverable is the GAP, not the packet

Stage 1's command predicts a set of blockers. The run produces a set of walls. **The product of stage 2 is
the difference:**

| | meaning |
|---|---|
| predicted **and** hit | the command works; no news |
| predicted, **not** hit | ★ the command over-reports, or the blocker is already cleared — both worth knowing |
| ★★ **hit, not predicted** | **the entire point.** An unknown unknown |

FR-102 (an HSA filer who could not print a page), FR-218 (a duplicate blank page found only on paper), and
this morning's five walls were all in that third row. A run that only confirms known blockers has taught
nothing and should be reported as having taught nothing.

## 5. What this CANNOT do — stated so no reader mistakes the output

- **It validates no figure.** OTS-2026 does not exist until ~2027-01-27, and a faked template carries no
  authority. Two oracles cannot be satisfied, so *nothing* here may be called validated.
- **It is not S1.** S1's value is a **signed** return as an answer key. This run has none. They are
  complements: this one finds *walls*, S1 finds *wrong figures*.
- **No invented value may be cited later as transcribed.** Every faked document is marked at its source,
  and the §2 gate is what makes that enforceable rather than aspirational.

## 6. Order of work

1. Land the §2 condition-bearing gate on `main`; watch it pass.
2. Stage 1's blocker command lands; capture its output as the **prediction**.
3. Tier A, in-repo.
4. Tier B, on a branch, and record the gap of §4.
5. File every §4-row-3 wall as a follow-up. Delete the branch; keep the report and the gate.
