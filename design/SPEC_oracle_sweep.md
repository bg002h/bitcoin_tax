# SPEC — the ORACLE SWEEP (double-oracle differential testing, read from the filled PDF)

*Status: **r5 — GREEN (0C/0I)** — folds the r4 re-review (`design/oracle-sweep/reviews/SPEC-oracle-sweep-fable-r4.md`,
**0C/1I**/1M/1Nit): r4 confirmed r3-I1 resolved-as-scoped and r3-I2 resolved; the lone residual r4-I1 was
the **mirror image** of the OTS provenance predicate — the same lawful printed-vs-exact-operand residual
exists **taxcalc-side above the Table ceiling**. r5 makes the provenance class **per-oracle** O ∈ {OTS,
taxcalc}; the composition is clean by construction (below the ceiling the taxcalc conjunct-1 fails so it
cannot over-absorb; at/above, `Table_btctax` *is* the exact schedule so it absorbs exactly the lawful
residual — a real bug still failing conjunct-1), with a second §5.1 pinned liveness cell (§6.2b/§6.4/§5.1/§12).
Also folds r4-M1 (the predicate needs the oracle's exact **leaf** figures — L15 cents, L3a, net-LTCG — already
obtainable, not OTS worksheet internals) and declines r4-N1 with rationale. Earlier history: r1 2C/6I → r2
0C/2I → r3 0C/2I → r4 0C/1I → **r5 0C/0I**; the differential test is an **evolution of `golden_packet.rs`**;
per-household divergences became **classes**; MFS/AMT deferred. The r5 re-review
(`design/oracle-sweep/reviews/SPEC-oracle-sweep-fable-r5.md`) is **GREEN**; its own 1 Minor + 2 Nits (the
regime-crossing-straddle §10 note; two wording/citation precisions) are folded here. **Spec is green —
ready for an implementation plan.***
*Provenance: extends the shipped P7 harness — `crates/btctax-core/tests/golden_returns.rs` (numbers vs two
engines), **`crates/btctax-forms/tests/golden_packet.rs` (the paper already held vs OTS)**,
`scripts/oracle/{gen_goldens,ots_direct}.py`, `crates/btctax-core/tests/goldens/full_return_goldens.json`.*

---

## 1. The problem (corrected baseline — r1 M-7)

btctax already has a paper-vs-oracle differential test: `golden_packet.rs` fills the **real PDFs** for the
12 golden households, reads them back with `extract_lines`, and asserts the figures **on the paper** equal
the figures **OpenTaxSolver** computed — 1040 L11/L15/L16/L24, Schedule SE L12, and the Schedule A SALT-cap
lines (`golden_packet.rs:70-153, 161-185, 475-508`). So "hold the paper against an independent engine" is
not new; it is the codebase's own trajectory. Four gaps remain:

1. **Coverage is 12 hand-written households** — chosen for named features, with nothing systematically
   walking their *combinations* or the threshold *boundaries* between them.
2. **Only ONE oracle reaches the paper.** `golden_returns.rs` holds btctax's *compute structs* against
   **two** engines; `golden_packet.rs` holds the *paper* against **OTS only**. Tax-Calculator — the
   independent-lineage witness — never sees the paper.
3. **The on-paper line set is shallow** — four 1040 lines + SE L12 + the SALT lines. The deeper lines
   where btctax has real branching logic (the 8995 §199A cap, the deduction taken, the Schedule D flow)
   are checked against the compute struct, or not against an oracle at all.
4. **No discovery mechanism** — the matrix only ever contains bugs someone thought to hand-write.

This spec: **extend, unify, and scale** the existing paper-vs-oracle check — a second on-paper oracle, a
deeper line set, a generated corpus, and a live sweep — without pretending to build a harness from zero.

## 2. Scope

**IN (v1):**

- **A. Deepen + unify the on-paper comparison:** the full shared line set, read off the filled PDF, held
  against **both** oracles (today OTS-only on paper), with the printed-chain comparison rule of §6.
- **B. Scale the corpus** from 12 hand-written households to a deterministic **variable-strength covering
  array** (§5.1); the 12 stay as pinned anchors. This **evolves `golden_packet.rs`** (§3.4, §7).
- **C. A non-CI live sweep** (`scripts/oracle/sweep.py`): seeded, threshold-biased random scenarios diffed
  live against both oracles (§9).
- **D. A divergence lifecycle** with declared divergence **classes** (§6.4, §10).

**OUT / DEFERRED:**

- **MFS (deferred — r1 I-2, a flagged decision):** the compute supports it, but the *harness* does not
  (`build_golden_household` panics on any status but Single/MFJ — `testonly.rs:483-487`; the fixture table
  has no MFS brackets — `testonly.rs:87`; `gen_goldens.py:222` maps any non-MFJ to taxcalc **Single**,
  silently answering a different question — MFS's NIIT/Add'l-Medicare thresholds are $125k not $200k, its
  loss cap $1,500 not $3,000). v1 domain is **{Single, MFJ}**, matching today's harness. MFS is future
  work: `testonly.rs` MFS brackets + builder arm, `gen_goldens.py` `MARS:3`, the OTS MFS status string,
  and D-1 coverage of `mfs_spouse_itemizes`.
- **AMT-triggering scenarios (OUT — r1 I-1):** v1 does not compute Form 6251; a return that needs it is
  **REFUSED** (`amt.rs:1-5`, `return_refuse.rs:161` `AmtScreenTriggered`). Such scenarios are outside the
  domain (§4).
- No change to the **compute engine** or the frozen files (`btctax-core/src/tax/{types,compute,se}.rs`).
- No change to the **fillers or map TOMLs** — this spec *reads* the filled PDFs and *fails* when they are
  wrong; fixing a fill bug it surfaces is separate work.
- **No third oracle engine**; no dependents/credits (CTC/ODC/EIC); no crypto lot/basis/§170(e) machinery
  in the differential path (only its consequences — a gain, a Schedule C profit — are varied).

## 3. Architecture

### 3.1 The pipeline under test

`golden_packet.rs` already runs it for OTS: `scenario → compute → assemble_printed_return → fill_full_return
→ extract_lines → diff vs oracle`. This spec scales the corpus feeding it, deepens the compared line set,
and adds Tax-Calculator as a second on-paper witness. A cell-mapping, overflow, transcription-rounding, or
blank-vs-zero bug is inside the test's reach — that part is already true today; we widen it.

### 3.2 The two read-back layers (r1 N-1, M-5)

- **`transcribe::extract_lines(pdf, map_toml) -> Result<BTreeMap<String, String>, FormsError>`** — reads a
  filled PDF back as `logical line → text on the paper`. Goes through the map, so it cannot by itself catch
  a mis-mapped cell; the oracle comparison is what gives it teeth.
- **`verify_flat` (geometric, map-independent)** — re-derives bands from the blank PDF's `/Rect`s and runs
  **inside every filler already** (`form1040_full.rs:241`, `schedule_a.rs:119`, `schedule23.rs`, `f8959/60/95`,
  `schedule_c/d/se/b`). It **narrows** the mis-map surface sharply (catches cross-column and descent-order
  mis-maps) but does **not** eliminate it — a mis-map onto a same-column widget preserving descent order
  evades it. We rely on `extract_lines` for the value-vs-oracle check and on `verify_flat` (already active)
  for placement; neither is claimed to make a mis-map impossible.

### 3.3 Hermeticity (verified)

`btctax-forms` depends on `lopdf` with `default-features = false`, nothing network-capable; all 14 blank
2024 PDFs are committed and `include_bytes!`-embedded (`pdf.rs:24-32`). Fill + `extract_lines` + `verify_flat`
are pure `lopdf`. The **gating** differential test fills and reads real PDFs entirely offline. Only the
*oracles* run out of band (§7); their answers are baked, as today.

### 3.4 Where it lives — evolve `golden_packet.rs` (r1 C-2, M-2)

The differential test is an **evolution of `crates/btctax-forms/tests/golden_packet.rs`**, not a new file
beside it (r1 C-2: a generated corpus breaks `golden_packet.rs` *first* — it loops `golden_households()` and
panics on any household missing from its hand-written form-set map, and asserts hard `checked == 3` counts).
The evolution: derive the form-set and SE/Sch-C expectations from inputs (§7), add the second on-paper
oracle and the deeper lines (§6), consume the generated corpus. The baked JSON **stays** at
`btctax-core/tests/goldens/full_return_goldens.json`, exposed as `testonly::GOLDEN_RETURNS_JSON` and read via
`golden_households()` — `include_str!` cannot cross the crate boundary without breaking `cargo package`
(`testonly.rs:358-364`), so r1's "goldens location" open decision is already settled in source (r1 M-2).

## 4. The comparability domain (what the generator may vary)

The generator emits **only** inputs all three engines model identically, so every disagreement is a real
bug, never an artifact.

**Varied:** filing status ∈ **{Single, MFJ}** (MFS deferred, §2); W-2 wages, taxable interest, ordinary +
qualified dividends, short/long-term capital gain/loss, Schedule C (SE) net profit; deductions standard or
itemized via the **components** (state income tax, real-estate tax, mortgage interest) so the §164(b)(5)
SALT cap can bind.

**Domain invariants:**

- **D-1 (no dependents):** every scenario claims none (CTC/ODC/EIC omitted, app §3.4), so `total_tax` is
  directly comparable. The builder already stamps `can_be_claimed_as_dependent_* = Some(false)`
  (`testonly.rs:498-509`).
- **D-2 (refusal-free — r1 I-1):** the corpus contains **only** scenarios btctax assembles without
  refusal. Enforcement is at generation time **via the §9 test-only harness binary**, invoked per candidate
  — generation is Python and assembly is Rust, so the check crosses that boundary through the harness, not a
  Python re-implementation of the AMT screen that could drift (r2-M4). A candidate is admitted only if the
  harness assembles it AND both oracles report **zero AMT (1040 L17)** and **zero credits (1040 L21)** — the
  two lines the L24 cross-foot precondition actually checks (`golden_packet.rs:104-119`; L18 is the L16+L17
  sum, not a credit — r3-N2). EIC is payments-side and
  touches no compared line, so childless EIC (which taxcalc computes automatically below ~$18.6k Single /
  ~$25.5k MFJ) is **not** a disqualifier, and the "low" W-2 band is floored above it so the covering array
  stays satisfiable (r2-M5). These guards are oracle-side *admission predicates*, not paper reads (no Form
  6251 in the repo; Schedule 2 Part I never prints — r1 I-1, N-2). A refusal that slips through anyway
  panics loudly at bake time (`return_refuse.rs:161`), before commit.
- **D-3 (comparability of the itemize election — r1 I-3):** itemized scenarios are constrained so
  itemizing **wins** (itemized total > standard). This removes the OTS `A18: Yes` forcing vs btctax's
  take-the-larger election as a confound (`ots_direct.py:261-268`).

**Never varied:** dependents/credits, HoH/QSS (need a qualifying person), the crypto lot/§170(e)
machinery. Only consequences (a gain, a Schedule C profit) are varied.

## 5. Generation strategy

### 5.1 The baked corpus — a **variable-strength**, constrained covering array (r1 I-3)

Axes (illustrative; fixed in the plan): filing status {Single, MFJ}; deduction {standard, itemized};
W-2 band {none, low, mid ~$100k, high >$250k}; interest {none, <$1,500 no-Sch-B, >$1,500}; dividends
{none, ordinary+qualified}; capital-gain shape {none, LT, ST, capped loss (−, §1211(b)), both slices};
SE profit {none, present, over the $250k Add'l-Medicare threshold}; SALT position (itemized only)
{under cap, over cap}.

Pairwise (t=2) is **insufficient** for §12's load-bearing requirement — the 8995-L12 qualified-dividend
term is a **3-way** interaction (SE × LTCG × qualified dividends, all at once; `gen_goldens.py:177-192`),
and pairwise never guarantees the triple co-occurs. So:

- **Variable strength:** t=3 over the named dangerous **triples** — {SE, LTCG, qualified-dividends},
  {itemized, SALT-over-cap, high-income} — and t=2 elsewhere.
- **Constraints layer** (pairwise/t-wise *with constraints*, standard): SALT-position implies itemized;
  itemized implies itemizing-wins (D-3); exclude the degenerate all-none (zero-income) row.
- **Explicit pinned cells** for every §12 load-bearing obligation, plus the current 12 anchors (their
  `why` prose preserved) — **including two bake-time-steered liveness cells** (r3-I2b, r4-I1): one puts an
  L16 worksheet operand onto a $50 Tax-Table bin edge (holds the **OTS** provenance class live), the other
  is a high-TI, above-ceiling cents household steered so its rate×δ residual flips a rounded dollar (holds
  the **taxcalc** provenance class live). Both are deterministic and checked — the generator has both
  engines' exact figures offline.
- **Deterministic amounts** per axis-value (no RNG in the baked path). **Target ~80–120 scenarios**,
  subject to the runtime budget (§8).

### 5.2 The live sweep — seeded, threshold-biased random

`sweep.py --seed N --count K` samples each axis from bounded distributions, **threshold-biased** toward the
boundaries a grid steps over: the $1,500 Sch B trigger, the $10,000 SALT cap, the $200k/$250k
NIIT/Add'l-Medicare thresholds, the OASDI wage base, the standard-deduction crossover. It also honors D-2/D-3
(rejects refusing or itemize-losing draws). Reproducible from the seed.

## 6. The comparison surface — read from the PDF

### 6.1 The full shared line set

For each scenario the evolved test fills the triggered packet and, via `extract_lines`, reads the value on
the paper for each line, held against both oracles per §6.2/§6.4. btctax-side is the parsed on-paper number.

**Headline lines** (already on paper vs OTS; add taxcalc + the printed-chain rule): AGI (1040 L11), taxable
income (L15), tax before credits (L16), SE tax (Sch 2 L4 ← Sch SE L12), NIIT (8960 L17 → Sch 2),
Add'l Medicare (8959 L18 → Sch 2), QBI deduction (8995 L15 → 1040 L13), **total tax (L24)**.

**Deeper lines** (new on-paper checks; each btctax-side is a filled cell): deduction *taken* (1040 L12),
SALT total after the cap (Sch A L5e), Schedule D net gain to 1040 L7, **Form 8995 line 12** (the §199A cap).
The exact oracle source for each is resolved in the plan against each engine's exposed outputs; a line an
oracle cannot express is **single-witness** against the other (§6.4). **No AMT/credits paper line** — those
are admission predicates (D-2).

### 6.2 The comparison rule — reproduce btctax's §3.1 printing on the ORACLE's figures (r1 C-1)

r1 C-1 refuted the r1 "no regression" claim: the printed chain **cross-foots** — each printed total sums
the already-rounded lines above it, deliberately **not** `round_dollar(exact_total)` (`printed.rs:5-8`;
Sch SE L12 = round(L10)+round(L11) at `:233`; L9 = Σ printed components at `:540`; L16 = Tax Table applied
to the **printed** L15 at `:607-610`). So comparing the paper to `round_dollar(exact-oracle-line)` invents a
lawful $1-class disagreement on the very lines today's test matches.

The rule, already proven for L24 at `golden_packet.rs:81-131`: **push the oracle's figures through btctax's
own §3.1 printing, then compare to the paper — exact, no tolerance.** Each compared line's reproduction is
derived line-by-line from `printed.rs`/`other_taxes.rs` in the plan; the patterns below are **illustrative,
not an exhaustive taxonomy** (r2-M2):

| Pattern | btctax prints | Held against | Example |
|---|---|---|---|
| **Leaf** | `round_dollar(exact_line)` | `round_dollar(oracle_line)` | Sch SE L10 (SS), L11 (Medicare); 8960 L13 = round(exact AGI) |
| **Cross-footed total** | `Σ round_dollar(component)` | `Σ round_dollar(oracle_component)` | L24; AGI (L11); taxable income (L15); Sch SE L12; 8959 L18 |
| **Rate-on-printed-operand** | `round_dollar(rate × printed_operand)` | `round_dollar(rate × reproduced printed_operand)` | 8959 L7/L13 (0.9%); 8960 L17 (3.8%) |
| **Tax-table** | `Table(printed L15)` / QDCGT on printed operands | *two-part rule below* | L16 |

**Cross-footed and rate-on-printed lines** require the oracle drivers to expose the **component/operand**
lines they consume (deepening the extraction — exactly §2.A; `ots_direct.py:164-171` already parses every
`Lxx` OTS prints, and taxcalc exposes the component arrays). Where an oracle exposes only the total, that
line is single-witness against the oracle that exposes the parts. The **rejected** alternative (a ±$k
residual tolerance on Σ-lines) is weaker — it would mask a genuine off-by-a-few-dollars fill bug.

**The Tax-table family (L16 and every total it flows into) is TWO-PART (r2-I1)** — either part alone is
either a check that cannot fail or a source of undeclared reds:

- **(a) Structural, exact:** paper L16 `==` `Table_btctax(reproduced printed TI)` — with the QDCGT variant
  computed on the **reproduced printed operands** on *both* sides where it applies (r2-N1). `Table_btctax`
  is btctax's own `qdcgt_line16` + `ty2024_table()` (reachable cross-crate: `method.rs:74`, `testonly`).
  This catches every fill/transcription/printed-chain bug, but — both sides using btctax's own lookup — it
  is **blind to a Tax-Table *semantics* bug** (wrong bin, schedule-below-$100k, a QDCGT worksheet error).
- **(b) Witness, exact:** hold `Table_btctax(reproduced printed operands)` against `round_dollar(O L16)`.
  Their disagreement is **classed as lawful only when it is fully explained by operand provenance**
  (printed-chain rounding), never by lookup semantics — the declared class (§6.4) is a **provenance
  predicate**, not a geometric bin test (r3-I1), declared **per oracle** O ∈ {OTS, taxcalc} (r4-I1):
  `Table_btctax(O's OWN exact operands) == round_dollar(O L16)` **AND** `Table_btctax(reproduced printed
  operands) ≠ round_dollar(O L16)`. The first equality confirms O's L16 is reproduced by btctax's own lookup
  on O's operands, so a genuine Table-*semantics* bug (wrong bin, schedule-below-ceiling, a QDCGT worksheet
  error) **fails it**, is absorbed by no class, and stays red — part (b) keeps its teeth. It subsumes
  bin-straddle, **ordinary-remainder** straddle, 15/20%-slice rounding, and TCW-cents — every way
  printed-vs-exact operands flip a rounded dollar (`method.rs:84-90`). `Table_btctax` is `qdcgt_line16`,
  which takes the three **leaf** figures (TI/L15 at cents, qualified dividends/L3a, the Sch-D net-LTCG term)
  and derives the remainder and slices internally (`method.rs:74-91`) — so the drivers expose those **exact
  leaves**: L15 at cents (baked), L3a (a driver input), and the **QD-*exclusive*** §1(h) net-LTCG subterm
  `max(0, min(ltcg, ltcg+stcg))` — note `ots_direct.py:292-294` computes the QD-*inclusive* 8995-L12 variant,
  so the driver exposes the subterm alone, never that value, else QD double-counts (r5-N2) — **not**
  worksheet internals OTS's output may never print (r4-M1). Fail-closed bonus: if
  an oracle's internal worksheet ever diverges from what its leaves imply (an oracle worksheet bug), conjunct
  1 fails, no class absorbs, and §10 triage catches it — the correct behavior. Part (b) restores the oracle's
  opinion of the *tax* that part (a) alone drops (`golden_packet.rs:129`); it is a **compute-level**
  comparison — where the compute-side test earns its keep (§7).
- **L24's tax component is the part-(a) figure**, so the total inherits the two-part treatment instead of
  smuggling the raw oracle L16 back in as a component (which would re-open the bin-straddle red on L24).

### 6.3 Sign conventions and blanks (r1 I-4, M-6)

- **Sign table.** The paper is not uniformly signed: 1040 **L7 is the one signed cell** (leading minus,
  `-3000`), while Schedule D's own L6/L14/L21 are **parenthesized-magnitude** boxes (`3000` meaning
  −3,000) (`printed.rs:387-390`). §6 carries a per-line sign-convention column: which cells are signed,
  which are magnitude-in-parenthesized-box, and the normalization applied before comparison. A sign-blind
  parse would false-red a correct capped-loss return; a reflexive `abs()` would mask a real sign-flip. The
  **capped-loss anchor** (`single_capital_loss_capped`) is this table's KAT (v1 guarantees the case).
- **Blank regimes.** Two distinct kinds of "absent," not collapsed: (a) lines the filler writes as
  **present-and-zero** are asserted present-and-zero (dropped-line detection — `golden_packet.rs:104-119`
  depends on this; defaulting absent→0 would make that guard vacuous); (b) a line on a **form the return
  legitimately omits** reads as absent ⇒ 0. §6's line table tags each compared line with its regime.
- Parse discipline: on-paper strings parse to integers after sign normalization; an unparseable value is
  itself a failure, reported with the raw string.

### 6.4 Two-oracle adjudication + divergence **classes** (r1 C-2, M-4, I-5)

- **Symmetric pass rule (r1 M-4):** a line passes when *every oracle that has an opinion* agrees with the
  paper (per §6.2). Lines an oracle cannot express are compared single-oracle — which requires the
  `ExpectedOts`/`ExpectedTaxcalc` schema to carry `Option` for those (today both are all-required `f64`,
  `testonly.rs:397-421`); a schema change the plan makes.
- **Declared divergence CLASSES (r1 C-2), not per-household entries.** A genuine engine-methodology
  difference is declared once as a **predicate** `(oracle, line-family, condition) → statute/why`, covering
  every matching household; per-household `dec!` figures do not scale (6 today → 40–60 over a ~100 corpus).
  The two L16-family classes are conditioned on the **worksheet operands the lookup consumes, not the
  headline TI** (r3-I1) — the QDCGT worksheet looks its **ordinary remainder** up in the Table independently
  of TI (`method.rs:47-56`, `golden_returns.rs:116-126`):
    - **`(taxcalc, L16-family, btctax's lookup consulted the Tax Table for ANY worksheet operand)`** — the
      **methodology** class: btctax **and OTS** use the Table's $50 bins (mandatory per the 1040 instructions
      when an operand is below `TAX_TABLE_CEILING`); taxcalc uses the exact rate schedule. Fires on
      `single_qdcgt_both_slices` (TI = 112,400, but its ordinary remainder is below the ceiling — the anchor
      the old "TI < $100k" gloss wrongly excluded); **refutable** when every operand is at/above the ceiling.
      It is condition-only, no value check (**declining r4-N1**): a strict value check would *under*-absorb a
      **mixed** household whose diff is methodology **plus** provenance, and the class is already backstopped
      by the OTS provenance conjunct under stacking — a taxcalc-only disagreement means btctax matched OTS's
      independent figure exactly, which witnesses btctax's Table semantics.
    - **`(O, L16-family, the §6.2(b) provenance predicate holds)` for O ∈ {OTS, taxcalc} (r4-I1)** — the
      printed-vs-exact-operand rounding residual, **per oracle**. Composition is clean by construction:
      **below** the ceiling (all operands) the taxcalc conjunct 1 *fails* (`Table_btctax` bins, taxcalc uses
      the schedule), so the methodology class is the taxcalc absorber there; in a **mixed** household a
      below-ceiling remainder near its bin midpoint may satisfy conjunct 1, but then the absorbed diff is
      identically a pure operand-provenance residual — lawful, not over-absorption (r5-N1). **At/above** the
      ceiling `Table_btctax` *is* the exact schedule (`method.rs:52-55`), so the taxcalc conjunct 1 holds
      exactly and the class absorbs precisely the lawful cents residual the §5.1 high-income SE cells produce
      — while a real btctax TCW/schedule bug still fails conjunct 1 and stays red. The OTS variant applies
      throughout (OTS uses the Table like btctax). **The one out-of-class residual** — a Table↔TCW
      regime-crossing straddle (exact TI a few dollars below the ceiling, the printed chain crossing it) — is
      measure-epsilon, cannot occur in the deterministic baked corpus, and falls to §10 triage if the sweep
      surfaces one; it is **not** fixed by widening the methodology condition to printed operands (that would
      also absorb a real btctax TCW bug — r5-M1).
- **The guard's class-form (r3-I2a).** The anti-"btctax against the world" guard stays, stated for classes: a
  line where btctax disagrees with **both** oracles passes **only when EACH oracle's diff independently
  matches its own declared, condition-bearing predicate** — the class analogue of the old per-household
  `agrees_with:"neither"` + `outlier_alt` stack (`golden_returns.rs:41-53, 358-372`), and *stronger*: a
  btctax Table bug matches neither predicate and stays red. Without this class-stacking rule a straddle
  household — *necessarily* a both-oracle disagreement (btctax taxes the bin midpoint, taxcalc the exact
  schedule at the edge) — could never legally fire its class. The **one** sanctioned exception is an
  explicitly-declared **known-defect divergence** (§10) — a both-oracle disagreement where btctax is the one
  that is wrong, pinned with an open follow-up id: the deliberate, loud escape hatch, never silent, and
  structurally distinct from the lawful classes.
- **Class liveness (r2-M6, r3-I2b, r4-I1):** every declared class fires for ≥1 corpus household **OR** carries
  a §5.1 pinned-cell obligation. The taxcalc **methodology** class is live via `single_qdcgt_both_slices` (+
  four more Table anchors, `golden_returns.rs:94-144`). The two **provenance** classes are occasional and the
  baked corpus is deterministic, so each is held live by its own **§5.1 bake-time-steered cell**: the OTS one
  by a household with an L16 operand on a $50 bin edge; the taxcalc one by a high-TI, above-ceiling cents
  household steered so its rate×δ residual flips a rounded dollar (the generator has both engines' exact
  figures offline — steering is deterministic and checked). The predicate analogue of the dead-entry guard
  (`golden_returns.rs:388-401`): an explanation that never applies fails loudly.
- **L12 single-witness closure (r1 I-5):** OTS cannot infer net capital gain — our driver **hand-computes**
  8995 L12 and feeds it to OTS (`ots_direct.py:19-33, 283-304`), so "paper L12 vs OTS L12" is
  self-referential and cannot fail on a wrong-formula bug. The plan closes the loop per `ots_direct.py`'s
  own proposal — **derive OTS's L12 from OTS's Schedule D output** — and/or resolves whether taxcalc
  exposes an L12-granular variable. Until closed, L12 is marked single-witness/weak in the line table, not
  advertised as an independent check.

### 6.5 Failure localization (three-way)

The test computes the internal chain anyway (to fill the PDF), so a mismatch reports **oracle /
btctax-internal / btctax-on-paper**: internal matches the oracle but paper does not ⇒ a fill/transcription
bug; both btctax values differ ⇒ a compute bug.

## 7. The baked corpus and the evolved test (r1 C-2, I-6)

- `gen_goldens.py`'s hand-written `HOUSEHOLDS` becomes the generated covering array (§5.1); the 12 anchors
  are emitted with their `why`. Both oracles run offline; `ots_direct.py` is extended for the deeper +
  component lines; the JSON schema gains the deeper-line keys (and `Option` per §6.4).
- **Making "corpus-size-agnostic" true (not assumed):** the evolved `golden_packet.rs` replaces its
  hand-written per-name form-set map and hard `checked == N` counts with **derived** expectations — the
  expected form set is computed from the household's inputs against the documented trigger thresholds (Sch B
  $1,500; 8959 $200k/$250k; 8995 with QBI; Sch D with gains; Sch A when itemized), and SE/Sch-C counts are
  derived from the inputs. Only then does adding a household need no Rust edit. **The 12 anchors keep their
  hand-written form sets** (`golden_packet.rs:300-350`) as pinned data, and §12 obligates the derivation to
  reproduce all twelve — so the trigger-derivation (which re-implements core's assembly triggers) is itself
  anchored by twelve known-answer sets, and a systematically-wrong derivation is caught rather than silently
  agreeing with a matching filler bug (r2-M3).
- The existing **whole-corpus determinism loops** — byte-reproducibility, the identity sweep — run over the
  **12 anchors only**, not the full generated array (they test packet-assembly determinism, which the
  anchors already exercise; running them over ~100 households is what blows the budget — §8). The
  **attachment-sequence-order** check (`golden_packet.rs:383-414`) is the exception: new form combinations
  yield new orderings, so it is genuinely valuable on generated households and **rides the sharded
  differential loop's existing fills** rather than running anchors-only (r3-N1).
- **`golden_returns.rs` (the second same-JSON consumer) is disposed, not left to break (r2-I2):** it
  **stays** and runs the **full generated corpus at compute level** — btctax's compute structs vs both
  oracles, cheap (no PDF fills) — adopting the same declared-class mechanism (§6.4). This is deliberate: it
  is the **compute-side witness** §6.2(b) needs — btctax's Tax-Table lookup held against the oracle's L16 —
  and the layer that catches a Table-*semantics* bug the paper's structural check (§6.2(a)) cannot. Its
  per-household `DECLARED_DIVERGENCES` become classes; its dead-entry liveness guard becomes the
  class-liveness rule (§6.4). Without this, regenerating the JSON turns `make check` red in `btctax-core`
  before the evolved forms test exists.

## 8. Runtime budget (r1 I-6)

`make check` is the gate: `cargo nextest run --workspace` + clippy, concurrently, **~6s warm**
(`Makefile:26-31`); the project treats this as sacred ([[fast-validation-gate]]). Measured ~150–250 ms per
packet fill; a single `#[test]` looping ~100 households is ~20–30 s serial (nextest runs each `#[test]` in
its own process, parallel across the whole run; the serial cost is *within* a single `#[test]`, which is
exactly why sharding the loop across many `#[test]`s parallelizes it — r2-M1). Budget and mitigations, all
in-spec:

- **The differential loop is sharded** across multiple `#[test]` functions (by household-hash into N
  shards), so nextest runs them in parallel and no single test dominates the gate.
- The determinism/identity loops stay on the **12 anchors** (§7).
- **Target:** the evolved test suite adds no more than a small constant to the warm `make check` (order a
  few seconds), verified by measurement in the plan. If the full ~100-corpus differential cannot fit the
  fast gate even sharded, the split is: `make check` runs the anchors + a fixed deterministic sample; the
  **full** generated corpus runs in a still-hermetic slower test that CI runs on every push (not a
  network/oracle dependency — just more fills). The plan measures and picks.

## 9. The live sweep (discovery, non-CI — r1 N-3)

`scripts/oracle/sweep.py`: generate K threshold-biased scenarios (§5.2); for each, drive **btctax** to fill
+ read back the packet, run **both oracles live**, diff the full line set (§6); emit a **divergence report**
(scenario as a ready-to-paste household, the disagreeing line, oracle-1 / oracle-2 / btctax-on-paper, seed +
index to reproduce). The sweep's btctax entry point is a **test-only harness binary**, not the
`export-irs-pdf` CLI: `build_golden_household` fabricates `LedgerState` directly, so the CLI path would
require authoring a vault that *reconciles* to the same ledger — far from "thin" — and the harness must read
the **same on-paper values** the CI test reads (r1 N-3). Never in the gating suite (needs the oracle
binaries/venv; non-deterministic across seeds).

## 10. Divergence lifecycle (r1 L-1; caught-bug policy user-mandated 2026-07-16)

Every divergence (baked corpus or sweep) is triaged into exactly one of:

- **btctax is wrong** (compute or fill; the three-way report says which) → **file a `FOLLOWUPS.md` entry**
  (with a severity and an owning phase, per STANDARD_WORKFLOW §4). **Fixing it is out of *this* project's
  scope** — the harness reads-and-fails; a compute/fill fix is separate work (**user-mandated 2026-07-16:
  caught bugs file follow-ups**). To keep the corpus green while the follow-up is open, the scenario is
  pinned as a **declared known-defect divergence**: btctax's *current* (wrong) value is asserted, labelled
  `KNOWN DEFECT → <FU-id>`, with the oracles' correct figures recorded beside it. A known-defect divergence
  is a **separate, loudly-named category — never one of the §6.4 lawful classes** (so a bug can never
  masquerade as a lawful difference), and it carries the follow-up id so it is burned down, not rotted. When
  the fix lands (separate work), the scenario converts to an ordinary green agreement.
- **btctax right, an oracle differs** → add or extend a declared divergence **class** (§6.4) and promote the
  scenario;
- **an oracle wrong** → record + exclude the line for that engine with a statute cite.

**Invariant L-1:** the baked corpus is always fully green — every difference in it is covered by a declared
lawful class, a promoted reconciled scenario, or a declared **known-defect** divergence carrying an open
follow-up. Nothing is ever silently tolerated; the class / known-defect mechanisms (not per-household `dec!`
entries) keep L-1 satisfiable as the corpus grows.

## 11. Engine-version drift policy (r1 M-3)

The baked answers depend on external engine versions (OTS 2024 22.07; Tax-Calculator 6.7.2), pinned in the
JSON `_provenance`. Regeneration is **version-gated**: a version bump is its **own reviewed event** — the
whole corpus is regenerated, the diff inspected line-by-line, and any shifted divergence class re-justified
before commit. Routine corpus edits (adding a scenario) do not bump versions; a version bump does not
silently ride in on a scenario edit.

## 12. Validation — how we know this works (r1 M-1)

- **Deeper lines have teeth:** each new compared line is load-bearing in ≥1 corpus scenario (the §5.1
  t=3 triples guarantee the 8995-L12 case).
- **Derived form-sets reproduce the anchors (r3-M1):** the §7 trigger-derivation reproduces all 12
  hand-written anchor form sets (`golden_packet.rs:300-350`) — a KAT, so a systematically-wrong derivation
  is caught rather than silently agreeing with a matching filler bug.
- **Every declared divergence class is live (r3-I2b, r4-I1):** each class fires for ≥1 corpus household or is
  held by its §5.1 pinned cell — the OTS and taxcalc provenance classes each carry their own pinned cell; a
  class matching nothing fails.
- **Read-back has teeth:** a fault-injection fixture (a perturbed on-paper value, or a temporary map swap
  under `#[should_panic]`) proves the test reads the PDF, not the struct.
- **Hermeticity:** the evolved test runs under the network-free `make check` with no venv/OTS binary.
- **Determinism:** regenerating the corpus twice yields identical `households` payload — the claim
  **excludes** the `_provenance.generated` date field (`gen_goldens.py:306`; r1 M-1), which is pinned or
  ignored by the determinism check.
- **Runtime:** the §8 budget is met, by measurement.
- **Green** = `make check` passes AND the differential test is 0 undeclared divergences AND the review loop
  is 0 Critical / 0 Important.

## 13. Non-goals

No new oracle engine; no MFS/HoH/QSS, no dependents/credits, no crypto lot machinery, no AMT scenarios in
v1; no change to compute, fillers, or map TOMLs; no automated/scheduled sweep in v1.

## 14. Open decisions for the plan (trimmed)

1. **MFS deferral** (§2) — **user-confirmed low priority** (2026-07-15: "definitely low priority to support
   mfs … maybe we will do this later when we understand why it exists"); v1 is {Single, MFJ}. Not revisited
   without an explicit ask; doing MFS means the harness work listed in §2.
2. **The L12 closure** (§6.4) — derive OTS L12 from OTS Schedule D vs resolve a taxcalc L12 variable vs
   ship it single-witness/weak.
3. **Full-corpus runtime split** (§8) — whether the full ~100 corpus fits the fast gate sharded, or splits
   into anchors-in-`make check` + full-corpus-in-CI. Decided by measurement.
4. **Covering-array construction** (§5.1) — a small vetted variable-strength-with-constraints generator vs
   hand-rolled (a dev/offline Python dep is acceptable; no new *runtime* dependency).
5. **Exact deeper-line oracle mappings** (§6.1) — resolved against each engine's exposed variables/lines;
   OTS-absent lines carried single-oracle via the §6.4 `Option` schema.
