# SPEC — the ORACLE SWEEP (double-oracle differential testing, read from the filled PDF)

*Status: **DRAFT r2** — r1 (`design/oracle-sweep/reviews/SPEC-oracle-sweep-fable-r1.md`, 2C/6I/7M/3Nit) is
folded here. The two Criticals were both refuted-by-source claims: C-1 (the printed chain cross-foots at
every total, so "compare paper to `round(exact-oracle)`" manufactures reds) and C-2 ("corpus growth needs
no Rust change" — `golden_packet.rs` breaks first, and the divergence model doesn't scale as per-household
entries). r2 rewrites the comparison rule around the **printed-chain-on-oracle-figures** pattern that
already ships (`golden_packet.rs:81-131`), makes the differential test an **evolution of
`golden_packet.rs`** rather than a new file beside it, replaces per-household divergences with declared
divergence **classes**, defers **MFS**, adds a **refusal-freedom** domain invariant, a **per-line
sign/cross-foot** table, a **runtime budget**, and an **engine-version drift** policy. Pending re-review.
NOT green until the loop reaches 0C/0I.*
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
  refusal. AMT-screen trippers and any other refusal are excluded by construction — the generator keeps
  amounts inside the refusal-free region — and the harness treats a refused assembly as a **generator
  bug** (loud panic naming the scenario), never a divergence and never silent. Admissibility is enforced
  at generation time: a candidate scenario is admitted only if btctax assembles it AND both oracles report
  **zero AMT and zero credits** (the AMT/credits "guards" are oracle-side *admission predicates*, not
  paper reads — there is no Form 6251 in the repo and Schedule 2 Part I never prints; r1 I-1, N-2).
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
  `why` prose preserved).
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
own §3.1 printing, then compare to the paper — exact, no tolerance.** Per line, by class:

| Class | btctax prints | Held against | Example |
|---|---|---|---|
| **Leaf** | `round_dollar(exact_line)` | `round_dollar(oracle_line)` | Sch SE L10/L11 parts; 8960 base |
| **Cross-footed total** | `Σ round_dollar(component)` | `Σ round_dollar(oracle_component)` | L24; AGI (L11); taxable income (L15); Sch SE L12; 8959 L18 |
| **Tax-table** | `Table(printed L15)` (or QDCGT on printed operands) | `Table(reproduced printed TI)` | L16 |

Cross-footed lines therefore require the oracle drivers to expose the **component** lines they sum
(deepening the extraction — exactly what §2.A entails); where an oracle exposes only the total, that line is
single-witness against the oracle that exposes the components. The **rejected** alternative (a ±$k residual
tolerance on Σ-lines) is weaker — it would mask a genuine off-by-a-few-dollars fill bug — and is not used.

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
  difference is declared once as a predicate `(oracle, line-family, condition) → statute/why`, covering
  every matching household. The canonical class: **`(taxcalc, {L16 and lines derived from it}, Tax Table
  mandatory i.e. TI < $100,000)`** — btctax **and OTS** use the Tax Table's $50 bins (mandatory per the
  1040 instructions below $100k); taxcalc uses the exact rate schedule (`golden_returns.rs:16-22, 102-104`).
  Per-household `dec!` figures do not scale (6 today → 40–60 over a ~100 corpus, each re-derived every
  regeneration); a class scales to any corpus size. The anti-"btctax against the world" guard stays: a line
  where btctax disagrees with **both** oracles is never silently classed.
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
  derived from the inputs. Only then does adding a household need no Rust edit.
- The existing **whole-corpus determinism loops** — byte-reproducibility, the identity sweep — run over the
  **12 anchors only**, not the full generated array (they test packet-assembly determinism, which the
  anchors already exercise; running them over ~100 households is what blows the budget — §8).

## 8. Runtime budget (r1 I-6)

`make check` is the gate: `cargo nextest run --workspace` + clippy, concurrently, **~6s warm**
(`Makefile:26-31`); the project treats this as sacred ([[fast-validation-gate]]). Measured ~150–250 ms per
packet fill; a single `#[test]` looping ~100 households is ~20–30 s serial (nextest parallelizes *across*
test binaries, not within one). Budget and mitigations, all in-spec:

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

## 10. Divergence lifecycle (r1 L-1)

Every sweep divergence is triaged into exactly one of: **btctax wrong** (compute or fill; the three-way
report says which) → fix + freeze the scenario into the baked corpus as a regression; **btctax right, an
oracle differs** → add or extend a declared divergence **class** (§6.4) and promote the scenario; **an
oracle wrong** → record + exclude the line for that engine with a statute cite.

**Invariant L-1:** the baked corpus is always fully green — every difference in it is covered by a declared
class or is a promoted, reconciled scenario. The class mechanism (not per-household entries) is what keeps
L-1 satisfiable as the corpus grows.

## 11. Engine-version drift policy (r1 M-3)

The baked answers depend on external engine versions (OTS 2024 22.07; Tax-Calculator 6.7.2), pinned in the
JSON `_provenance`. Regeneration is **version-gated**: a version bump is its **own reviewed event** — the
whole corpus is regenerated, the diff inspected line-by-line, and any shifted divergence class re-justified
before commit. Routine corpus edits (adding a scenario) do not bump versions; a version bump does not
silently ride in on a scenario edit.

## 12. Validation — how we know this works (r1 M-1)

- **Deeper lines have teeth:** each new compared line is load-bearing in ≥1 corpus scenario (the §5.1
  t=3 triples guarantee the 8995-L12 case).
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

1. **MFS deferral** (§2) — flagged for the user; v1 is {Single, MFJ}. (Overridable: doing MFS means the
   harness work listed in §2.)
2. **The L12 closure** (§6.4) — derive OTS L12 from OTS Schedule D vs resolve a taxcalc L12 variable vs
   ship it single-witness/weak.
3. **Full-corpus runtime split** (§8) — whether the full ~100 corpus fits the fast gate sharded, or splits
   into anchors-in-`make check` + full-corpus-in-CI. Decided by measurement.
4. **Covering-array construction** (§5.1) — a small vetted variable-strength-with-constraints generator vs
   hand-rolled (a dev/offline Python dep is acceptable; no new *runtime* dependency).
5. **Exact deeper-line oracle mappings** (§6.1) — resolved against each engine's exposed variables/lines;
   OTS-absent lines carried single-oracle via the §6.4 `Option` schema.
