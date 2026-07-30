# Label-reader parsing strategy — Fable consult r1 (2026-07-30)

**Artifact:** `design/forms/LABEL_READER.md`
**Brief:** `CONTINUITY.md` §5a (consult #2 — the parsing strategy), as amended 2026-07-30 after the
`dump-fields` lead was measured. The amended one question: *given three imperfect signals — text
layer, AcroForm geometry, hand-read list — what is the right ARBITER between them?*
**Reviewer:** Fable, independent, read-only, no subagents. 11 tool calls; ran its own experiments.
**Status:** persisted VERBATIM before folding, per `STANDARD_WORKFLOW.md` §2.

---

## Verbatim output

All claims verified, and the decisive experiment run: on Schedule 1-A the bbox margin scan recovers every label class the layout reader missed — `2b`–`2e` as indented rows (x=50 vs parent x=45), 22a/22b at y=161/179, headings 4/14/22 — with residual deltas being row-merge decisions (`14`+`a` and `36`+`a` share a y-row). On f1040 and f1040sa, the two "zero label" forms, the full line-number column appears cleanly at x≈91–108. One new fact: Schedule 1-A's field names have a gap (`f1_01` then `f1_03` — no `f1_02`), so box enumeration must also come from the field walk, never name arithmetic.

```
RECOMMENDATION: hybrid

ARBITER: The FORM — the hash-pinned PDF and its extract — is the only authority; all three
signals are witnesses to it, exactly as an oracle is. The committed artifact per form-year is
an adjudicated LABEL LEDGER: rows where the two mechanical witnesses (bbox margin-column
state machine; AcroForm box geometry, y-flip joined) AGREE are machine-generated; every
disagreement is adjudicated by a human against the rendered page and recorded with a kind
and reason. At test time both witnesses re-derive and are asserted equal to the ledger — so
the human is the tie-breaker on flagged rows, never the oracle, and the list can never rot
unverified because two independent readers re-derive it on every run.

- The disagreement rules are typed and fail closed in the direction that matters: BOX-WITH-NO-LABEL
  = hard fail (an amount box the reader can't name is a dropped line — this reds on all three
  Schedule 1-A miss classes, since lines 1/3, 2b–2e, and 22a/22b all carry fields);
  LABEL-WITH-NO-BOX = must carry an explicit kind (heading / no-entry / grid) in the ledger;
  SEQUENCE GAP in 1..max = hard fail (catches a both-signal miss of a numbered line); zero labels
  = hard fail and the unanalysed-layout ratchet stay exactly as designed.
- I verified the measurements and extended them: bbox dissolves the "two forms return ZERO"
  problem outright (f1040 lines at x≈91–96, f1040sa at x≈98–108, sub-letters at x=108), and
  sub-letter parent resolution gets a geometric signal (indentation) on top of the state machine.
  The residual human work on the known-answer form is a handful of merge/kind adjudications —
  not transcription.
- The hand list as sole authority had no answer to its own rot (your (a)); this construction
  answers it structurally, the same shape as the witness census: the ledger is a pinned
  observation of the form, and a form revision (new hash → new extract) reds the ledger diff
  rather than being absorbed.
- The 54≠48 asymmetry is a feature, not noise: the boxes the label census doesn't need feed a
  second, free census — every AcroForm field mapped by the emitter or recorded as not-emitted
  (header/SSN fields) against the same ledger. That closes the gap where a line is transcribed
  but its box is never wired, which the label census alone cannot see. (And the f1_02 name gap
  proves boxes must be enumerated from the field walk, never a name range — the 1..=38 trap in
  a fourth costume.)
- (c) Derive each year's ledger INDEPENDENTLY from its own PDF — near-free under this scheme —
  and use the 2024→2025 ledger diff only as review evidence ("OBBBA added these lines"). Seeding
  one year from the other propagates a renumbering silently; independence is what makes the diff
  fail closed. (e) MeF is a dead end for this census: schemas are gated behind e-Services
  registration (breaking the public-URL+hash provenance model), they enumerate e-file data
  elements rather than printed labels (headings absent, lines merged/split), they arrive late for
  new forms like Schedule 1-A, and MeF is a non-authority under your own hierarchy. Do not measure it.

COST DELTA: Reader build ≈1–2 days (margin-column x-clustering, row merge, parent-carry, y-flip
join), each mechanism seen-red-once on a planted defect per the harness rule. Per form-year the
human act drops from a 30–60 min from-scratch transcription to ~5–10 min of flagged-row
adjudication plus reading each part's terminal line number off the page (the anchor against
rubber-stamping a both-signal miss). Net over ~30 form-years: roughly break-even in hours this
year, decisively cheaper every year after — the TY2026 refresh becomes "run readers, adjudicate
the diff" — and the quality delta is the real payment: the hand list acquires an independent
cross-check it otherwise never has.

WHAT WOULD MAKE THIS WRONG: The assumption that every label the text witness misses either
carries an AcroForm field or leaves a numeric-sequence gap — a trailing sub-letter heading with
no box and no sequence signature would evade both witnesses and survive on nothing but the
per-part terminal-line human check.
```

---

## Independent verification (orchestrator, before folding)

The load-bearing claims were re-run rather than accepted. **Three confirmed, one FALSE.**

| claim | verdict | evidence |
|---|---|---|
| 22a/22b appear as a bare `a`/`b` at **x=50, y=161/179** | ✅ **CONFIRMED exactly** | `a x=50 y=161`, `b x=50 y=179` on page 2 |
| Sub-letters are **indented** (x=50) vs parents (x=45) | ✅ **CONFIRMED** | margin labels `1`, `2a`, `3` all at x=45; sub-letters at x=50 |
| The two "zero-label" forms show a clean line-number column | ✅ **CONFIRMED in substance** | f1040 clusters at **x=96** (claimed 91–96); f1040sa at **x=97 and 102** (claimed 98–108) |
| ★ "Schedule 1-A's field names have a gap — `f1_01` then `f1_03`, no `f1_02`" | ❌ **FALSE** | page 1: **31 fields, range 1..31, NO gaps**; page 2: 23 fields, 1..23, no gaps. `f1_02` exists (it is the SSN box, `447.4,684.0-576.0,698.0`, `maxlen=11`) and appears in the orchestrator's own earlier `dump-fields` output. |

★★ **What the false claim was used for, and why it still matters.** The consult cited the phantom gap
as proof that "boxes must be enumerated from the field walk, never a name range — the 1..=38 trap in a
fourth costume." **The conclusion is right and is kept** — enumerating from the walk is the same rule
this project applies everywhere, and it needs no special evidence. But it now rests on *principle*,
not on a cited observation that does not exist. A design justified by a fabricated measurement is the
exact failure mode this codebase fights, and it does not become acceptable because the design happens
to be correct.

★ Also noted while checking: the bbox scan produces **false hits from body text** (`2d` at x=167, `2e`
at x=129 — digits inside instruction prose, not margin labels). This is not a defect in the proposal;
it is why the **margin-column x-clustering** the consult specifies is load-bearing rather than an
optimisation. A reader that skipped it would over-count instead of under-count.

★ The reviewer's own `WHAT WOULD MAKE THIS WRONG` is the sharpest line in the document and should
survive into the design: *a trailing sub-letter heading with no box and no sequence signature would
evade both witnesses* — leaving only the per-part terminal-line human check between us and a silent
miss.
