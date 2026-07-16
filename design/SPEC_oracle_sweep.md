# SPEC — the ORACLE SWEEP (double-oracle differential testing, read from the filled PDF)

*Status: **DRAFT r1** — brainstormed 2026-07-15, approved for spec. Pending an independent Fable
architect review (`design/oracle-sweep/reviews/SPEC-oracle-sweep-fable-r1.md`). NOT green until that
loop reaches 0 Critical / 0 Important per `STANDARD_WORKFLOW.md`.*
*Provenance: extends the shipped **P7 golden-return harness** (`crates/btctax-core/tests/golden_returns.rs`,
`scripts/oracle/gen_goldens.py`, `scripts/oracle/ots_direct.py`,
`crates/btctax-core/tests/goldens/full_return_goldens.json`), which already diffs btctax against TWO
independent engines (OpenTaxSolver 2024, driven directly; PSL Tax-Calculator 6.7.2). This spec does not
build a harness from zero — it (a) scales the scenario matrix, (b) deepens the per-scenario comparison
to the full shared line set, (c) **moves the btctax side of the comparison to the values read back off
the filled IRS PDF**, and (d) adds a non-CI live "sweep" for unknown-unknown discovery.*

---

## 1. The problem

The P7 harness is btctax's only non-self-referential test: it holds btctax's computed figures against
two engines of separate lineage, so an *internally-consistent wrong number* (a return where every form
cross-foots and the tax is simply wrong) has somewhere to be caught. Two gaps remain:

1. **Coverage is 12 hand-written households.** They were chosen to hit specific features (QDCGT, SALT
   cap, §199A line 12, NIIT + Add'l Medicare, Schedule C→SE). Nothing systematically walks the
   *combinations* or the threshold *boundaries* between them.
2. **The comparison stops at the compute engine.** The btctax side is `assemble_absolute` /
   `assemble_printed_forms` — internal structs. The artifact a filer actually submits is the **filled
   IRS PDF**, produced by a whole additional layer (the map, the fillers, transcription rounding,
   overflow-row handling, blank-vs-zero). A bug in that layer — a value written to the wrong cell, a
   line rounded at the wrong step, a figure lost to an overflow page — is invisible to a struct-level
   comparison. The oracle should be held against **what is on the paper**, because that is what the IRS
   receives.

This spec closes both.

## 2. Scope

**IN (v1):**

- **A. Deepen the comparison to the full shared line set**, read back **off the filled PDF** via
  `btctax_forms::transcribe::extract_lines`, with `btctax_forms::verify`'s geometric guards active.
- **B. Scale the baked corpus** from 12 hand-written households to a deterministic **covering array**
  over orthogonal input axes (current 12 preserved as named anchors). CI stays hermetic and gating.
- **C. A non-CI live sweep** (`scripts/oracle/sweep.py`): seeded, threshold-biased random scenarios
  diffed **live** against both oracles, emitting a divergence report for triage.
- **D. A divergence lifecycle** that promotes every triaged sweep finding into the baked corpus as a
  permanent regression case.

**OUT (not this spec):**

- Any change to the compute engine or the frozen files (`crates/btctax-core/src/tax/{types,compute,se}.rs`).
- Any change to the **fillers or the map TOMLs** — this spec *reads* the filled PDFs and *fails* when
  they are wrong; fixing a fill bug it surfaces is separate work.
- A third oracle engine. Two of separate lineage already give adjudication (§6.4); a third is not
  justified by this spec.
- Dependents and credits (CTC/ODC/EIC) — btctax conservatively omits them (§3.4 of the app); the
  domain forbids them (§4) so `total_tax` stays directly comparable.
- The crypto-specific machinery (§170(e) reduction, 8949 lot/basis selection) — it has no counterpart
  in any general engine and stays on btctax's own hand-worked KATs. The sweep varies only its
  *consequences* (a capital gain, a Schedule C profit).

**DEFERRED (post-v1, if wanted):** withholding/estimated-payment axes (refund/owe lines); AMT-triggering
scenarios; a scheduled/automated run of the live sweep (v1 is run-by-hand).

## 3. Architecture

### 3.1 The pipeline under test — now end-to-end

Before: `scenario inputs → compute → (internal structs) → compare to oracles`.
After:

```
scenario inputs → compute → packet assembly → FILL the official IRS PDFs
                                                        │
                                                        ▼
                              READ the values back off the paper (extract_lines)
                                                        │
                                                        ▼
                       diff the full line set against oracle-1 AND oracle-2
```

A cell-mapping bug, an overflow-row bug, a transcription-rounding bug, and a blank-vs-zero bug now all
fall inside the test's reach. None of them did before.

### 3.2 Two read-back layers, both already in the codebase

btctax already ships exactly the two complementary read-backs this needs, and their own doc comments
draw the distinction:

- **`transcribe::extract_lines(pdf, map_toml) -> BTreeMap<String, String>`** — reads a filled PDF back
  as `logical line → the text actually on the paper` (keys `line11`, `line15`, dotted group keys,
  indexed rows). *"This says the right VALUE is in it."* It goes through the map, so it cannot by
  itself catch a mis-mapped cell — but the **oracle comparison is what gives it teeth**: a wrong value
  on the paper disagrees with two engines.
- **`verify` (geometric, map-INDEPENDENT)** — re-derives column/row bands from the blank PDF's own
  `/Rect`s and never consults the map. *"Geometry says the value landed in the right box."* Its
  `verify_8949` / `column_x_bands` cover the 8949/Schedule-D grids; its `no_unmapped_filled` asserts no
  field outside the authorized set carries a value, on any form.

The differential test uses **both**: `extract_lines` for the value comparison against the oracles, and
`verify`'s geometric + `no_unmapped_filled` guards so a mis-mapped cell cannot hide behind a
right-looking number.

### 3.3 Hermeticity is preserved

The blank fillable IRS PDFs are **committed** (`crates/btctax-forms/forms/2024/*.pdf` — the full set:
`f1040`, `f1040s1/s2/s3`, `f1040sa/sb/sc`, `schedule_se`, `schedule_d`, `f8949`, `f8959`, `f8960`,
`f8995`, `f8283`). Fill + `extract_lines` + `verify` are pure `lopdf`, no network. So the **gating,
baked-corpus test fills and reads real PDFs entirely offline**. Only the *oracles* need to run out of
band (§5), and their answers are baked exactly as today (`why_baked`, SPEC §10). CI stays network-free.

### 3.4 Where the pieces live

| Piece | Location | Language | Runs in CI? |
|---|---|---|---|
| Oracle drivers (extended lines) | `scripts/oracle/ots_direct.py`, `gen_goldens.py` | Python | no (offline, by hand) |
| Covering-array corpus generator | `scripts/oracle/gen_goldens.py` | Python | no |
| Baked oracle answers | `crates/btctax-core/tests/goldens/full_return_goldens.json` | JSON | consumed by CI |
| **Differential test (fill + read-back + diff)** | **`crates/btctax-forms/tests/golden_returns_pdf.rs`** (new home) | Rust | **yes, gating, hermetic** |
| Live sweep | `scripts/oracle/sweep.py` | Python | no (discovery) |

**Test relocation (§8):** the differential test moves from `btctax-core/tests` (which cannot fill PDFs)
to `btctax-forms/tests`, where the fillers, the blank PDFs, `extract_lines`, and `verify` all live
(alongside the existing `golden_packet.rs` / `full_return_forms.rs`).

## 4. The comparability domain (what the generator may vary)

The generator emits **only** inputs that all three engines model identically, so every disagreement is
a real bug and never an apples-to-oranges artifact.

**Varied:**

- **Filing status** ∈ {Single, Married/Joint, Married/Separate}. **NOT HoH / QSS** — both require a
  qualifying person, and the domain forbids dependents.
- **Income:** W-2 wages (the filer's own box-1/box-3), taxable interest, ordinary + qualified
  dividends, short-term capital gain/loss, long-term capital gain/loss, Schedule C (SE) net profit.
- **Deductions:** standard, OR itemized via the **components** — state income tax, real-estate tax,
  mortgage interest — so the §164(b)(5) SALT cap can actually bind (a lump-sum itemized total cannot
  exercise it).

**Never varied (would break comparability):**

- Dependents and credits (CTC/ODC/EIC) — btctax omits them (app §3.4); every scenario has none, so
  `total_tax` is directly comparable.
- The crypto lot/basis/§170(e) machinery — no oracle counterpart. Only its consequences are varied.

**Invariant D-1:** every generated scenario must state (as the current households do) that no dependent
is claimed, and the coverage of the domain is asserted by construction (the generator cannot emit a
field outside this list).

## 5. Generation strategy

Split to match the baked-gate + live-sweep architecture.

### 5.1 The baked corpus — a deterministic covering array

Define orthogonal **axes** (illustrative; final list fixed in the plan):

| Axis | Values |
|---|---|
| filing status | Single · MFJ · MFS |
| deduction | standard · itemized |
| W-2 band | none · low · mid (~$100k) · high (>$250k threshold) |
| interest | none · below $1,500 (no Sch B) · above $1,500 (Sch B files) |
| dividends | none · ordinary+qualified |
| capital gain shape | none · LT only · ST only · capped loss (−, §1211(b)) · both preferential slices |
| SE profit | none · present · over the $250k Add'l-Medicare threshold |
| SALT position (when itemized) | under the $10k cap · over the cap |

The full cross-product explodes; the corpus is a **pairwise covering array** (every pair of axis-values
co-occurs in at least one scenario) **plus the current 12 households pinned as named boundary anchors**.
Deterministic and legible — each generated scenario carries a `why` naming the cell(s) it fills.
**Target size: ~80–120 scenarios** (tunable; the Rust test is corpus-size-agnostic — see §7).

**Amounts are chosen deterministically** from each axis-value (no RNG in the baked path), so
regenerating the corpus is reproducible to the cent.

### 5.2 The live sweep — seeded, threshold-biased random

`sweep.py` samples each axis from bounded distributions using a **fixed seed passed on the command
line** (so any run is reproducible), **threshold-biased**: it deliberately draws amounts *near* the
boundaries a grid can step over — the $1,500 Schedule B trigger, the $10,000 SALT cap, the
NIIT/Add'l-Medicare thresholds ($200k/$250k), the OASDI wage base, the standard-deduction crossover.
Boundary bugs (off-by-one at a threshold, a `>=` that should be `>`) are exactly what a covering array
of round numbers misses and a threshold-biased fuzzer finds.

## 6. The comparison surface — read from the PDF

### 6.1 The full shared line set

For each scenario the test fills the **full triggered packet** and, via `extract_lines`, reads back the
value **on the paper** for each line below. Each oracle figure maps to a `(form, line)`; the btctax side
is the parsed on-paper number (not a compute struct).

**Headline lines (extend the current 8):**

| Line | btctax source (on paper) | OTS | Tax-Calculator |
|---|---|---|---|
| AGI | 1040 L11 | ✓ | `c00100` |
| Taxable income | 1040 L15 | ✓ | `c04800` |
| QBI deduction | 8995 L15 (→1040 L13) | ✓ | `qbided` |
| Tax before credits | 1040 L16 | ✓ | `taxbc` |
| SE tax | Sch 2 L4 (from Sch SE L12) | ✓ | `setax` |
| NIIT | 8960 L17 (→Sch 2) | ✓ | `niit` |
| Add'l Medicare | 8959 L18 (→Sch 2) | ✓ | `ptax_amc` |
| **Total tax** | **1040 L24** | ✓ | (none — bundles payroll) |

**Deeper lines (new; each btctax source is a filled cell; the exact oracle variable/line for each is
resolved in the plan against the engines' exposed outputs, and any line an engine does not expose is
simply not compared for that engine):**

- Deduction *taken* (1040 L12) — distinguishes standard vs itemized on the paper.
- SALT total **after the cap** (Schedule A L5e) — the line where §164(b)(5) actually binds.
- Schedule D net gain reaching **1040 L7**.
- Form 8995 **line 12** (the §199A cap = 20% × (taxable income − net capital gain, increased by
  qualified dividends)) and its inputs — the line the `single_miner_qbi_limited_by_net_capital_gain`
  household exists to hold.
- **Guard lines** (expected trivial for this domain, still compared so a regression that makes them
  non-trivial goes red): AMT (Form 6251 / 1040 L16 alt), total credits.

### 6.2 Chain semantics — why reading off the PDF is *more* faithful, with no regression

The current test compares **component** lines against `assemble_absolute` (`round_dollar` of the exact
value) but the **total** against the **printed** chain (`printed.f1040.line24`), with a careful comment:
the filed total is the sum of already-rounded lines (cross-footing, Σround ≠ roundΣ), and *"it is the
filed figure the oracle must be held against, because it is the filed figure the IRS receives."*

Reading every line off the PDF **generalizes that argument to every line**: the PDF carries the
**printed/filed** figure for all of them. And there is no regression on components — under the §3.1
round-all-amounts election a printed component line *is* `round_dollar` of its exact value, so the six
component lines read off the paper equal today's numbers by construction, while the **total** is now the
genuinely cross-footed filed figure automatically (it is literally the number in the box). The
refinement is strictly more faithful: **every line compared is the filed figure the IRS receives.**

### 6.3 Matching, tolerance, and blanks

- **Exact after `round_dollar`.** No fuzzy tolerance. The paper is already in whole dollars (§3.1); the
  oracles report cents and are `round_dollar`'d before comparison, exactly as today.
- **Blank/absent cell ⇒ "none" ⇒ compared as 0.** `extract_lines` returns only cells the fill actually
  wrote; an absent key is a blank line on the form (a statement of "none"), read as 0. A household with
  no SE tax has no Schedule SE and no `Sch 2 L4` key — that is 0, and must match an oracle SE tax of 0.
- **Parse discipline.** On-paper strings are parsed to integers; a value that fails to parse (unexpected
  formatting) is itself a failure, reported with the raw string.

### 6.4 The two-oracle adjudication model (unchanged, extended per line)

The existing logic is preserved and applied to every line: a line **passes** when OTS agrees AND
(Tax-Calculator agrees OR reports no comparable figure). A disagreement is either a **declared
`Divergence`** (which oracle btctax follows, and the statute that makes it right — with the dissenting
oracle's figure recorded, and BOTH pinned when both dissent) or a **red failure**. The anti-"btctax
against the world" guard stays: a line where btctax disagrees with *both* oracles is never silently
declared. The `DECLARED_DIVERGENCES` table gains a `line` granularity that already exists (it keys on
`(household, line)`).

### 6.5 Failure localization (three-way report)

The test still computes the internal chain (`assemble_absolute` / `assemble_printed_forms`) — it must,
to fill the PDF — so a mismatch reports all three figures: **oracle / btctax-internal / btctax-on-paper**.

- internal matches the oracle, paper does not ⇒ a **fill/transcription/map bug**.
- both btctax values differ from the oracle ⇒ a **compute bug**.

The test names which the instant it goes red, instead of leaving a human to bisect the pipeline.

## 7. The baked corpus (gates CI, hermetic)

- `gen_goldens.py`'s hand-written `HOUSEHOLDS` list is replaced by the **generated covering array**
  (§5.1); the current 12 are emitted as pinned anchors so their `why` prose survives.
- Both oracles run offline over the corpus; `ots_direct.py` is extended to emit the deeper lines (§6.1);
  `gen_goldens.py` records them per household. The JSON schema gains the deeper-line keys under
  `expected_ots` / `expected_taxcalc` and keeps the `_provenance` block (oracle versions, licensing,
  `not_covered`, `why_baked`).
- The Rust differential test **loops the `households` array** and is therefore **corpus-size-agnostic**:
  growing the corpus from 12 to ~100 needs **no Rust change**. Only *deeper lines* need Rust changes
  (more comparison entries) and oracle-side extraction.
- Regeneration recipe (unchanged shape, documented in the file header):
  `OTS_DIR=… .venv/bin/python scripts/oracle/gen_goldens.py > …/full_return_goldens.json`.

## 8. Test placement & module layout

- **New:** `crates/btctax-forms/tests/golden_returns_pdf.rs` — the fill + read-back + diff test. It
  owns the `Divergence` model, the per-line comparison, `extract_lines`-based read-back, `verify`
  geometric guards, and the three-way localization.
- **The baked JSON** stays at `crates/btctax-core/tests/goldens/full_return_goldens.json` (the single
  source of oracle truth). The forms test reads it; **‹decision for the plan›** either `include_str!`
  via a relative path or relocate the goldens to a workspace-shared `tests/goldens/` — the plan picks
  one and states why.
- **The existing `btctax-core/tests/golden_returns.rs`:** **‹proposed›** keep a thinned compute-level
  check (it is what localizes compute-vs-fill and needs no PDFs), with the **authoritative** oracle
  comparison now the on-paper one in the forms crate. The plan may instead fully supersede it — but not
  silently: if it is removed, the localization role moves into the forms test's three-way report.
- Households are built in btctax-core's `testonly` (`build_golden_household`, `golden_households`);
  the forms test consumes them the same way, so the "one builder, two consumers" discipline
  (`testonly.rs` note) is preserved — the forms test is a **third** consumer of the same builder.

## 9. The live sweep (discovery, non-CI)

`scripts/oracle/sweep.py`:

1. Given `--seed N --count K`, generate K threshold-biased random scenarios (§5.2).
2. For each: drive **btctax** to fill the packet and read it back (via a thin, existing btctax entry
   point — **‹plan decides›** the `export-irs-pdf` CLI path over a synthetic vault, or a small
   test-only harness binary; the sweep must read the SAME on-paper values the CI test does), run **both
   oracles live**, diff the full line set.
3. Emit a **divergence report**: the scenario inputs (as a ready-to-paste household dict), the
   disagreeing line, and all three figures (oracle-1 / oracle-2 / btctax-on-paper), plus the seed and
   index so it reproduces.

The sweep is **never** in the gating suite (it needs the oracle binaries/venv and is non-deterministic
across seeds). It is run by hand or periodically. A clean sweep prints "N scenarios, 0 undeclared
divergences"; a dirty one prints the report for triage.

## 10. Divergence lifecycle (how a sweep finding becomes durable)

Every sweep divergence is triaged into exactly one of:

1. **btctax is wrong** (compute or fill) → file the bug, fix it, and **freeze the scenario into the
   baked corpus** as a permanent regression case (with its `why`). The three-way report says whether
   the fix belongs in compute or in the fill layer.
2. **btctax is right, an oracle differs** (a statutory position, or an oracle quirk) → add a declared
   `Divergence` (statute + which oracle) and **promote the scenario to the baked corpus** so the
   difference is pinned and re-opens if either engine's answer moves.
3. **An oracle is wrong** → record it (a note beside the divergence) and exclude the line for that
   engine, citing the statute.

**Invariant L-1:** the baked corpus is *always* fully green — every difference in it is declared and
explained. New/undeclared divergences live only in the sweep, which is where the unknowns surface. A
sweep finding is not "handled" until it is either fixed (→ regression case) or declared (→ promoted).

## 11. Non-goals

- No new oracle engine.
- No dependents/credits, no HoH/QSS, no crypto lot machinery in the differential path.
- No change to the compute engine, the fillers, or the map TOMLs (this spec *reads* and *fails*; it does
  not fix the fill layer).
- No automated/scheduled sweep in v1.

## 12. Validation — how we know this works

- **The deeper lines have teeth:** for each new compared line, at least one corpus scenario makes it
  load-bearing (a scenario where dropping that line's logic changes a number), mirroring the discipline
  the current `why` prose already applies (e.g. 8995 L12 is held by
  `single_miner_qbi_limited_by_net_capital_gain`).
- **The read-back has teeth:** a deliberate fault-injection check — e.g. a scenario whose on-paper total
  is perturbed (or a temporary map swap in a `#[should_panic]` fixture) — proves the test reads the PDF,
  not the struct. (Design detail in the plan; the point is the read-back path is itself tested.)
- **Hermeticity holds:** the forms test runs under the existing network-free CI with no venv and no OTS
  binary; only committed PDFs + baked JSON are touched.
- **Determinism:** regenerating the baked corpus twice yields byte-identical JSON (the covering array
  and its amounts are deterministic); `gen_docs`-style determinism KAT optional.
- **Green** = `make check` passes AND the differential test is 0 undeclared divergences AND the spec/plan
  review loop is 0 Critical / 0 Important.

## 13. Open decisions for the plan (flagged, not deferred)

1. **Goldens location** (§8): `include_str!` relative path vs relocate to a shared `tests/goldens/`.
2. **Fate of the core-side test** (§8): thinned compute check vs full supersession.
3. **Sweep's btctax entry point** (§9): `export-irs-pdf` CLI over a synthetic vault vs a test-only
   harness binary — constrained by "must read the same on-paper values the CI test reads."
4. **Covering-array construction** (§5.1): a small vetted pairwise generator vs a hand-rolled one (no
   new runtime dependency; a dev/offline Python dep is acceptable).
5. **Exact deeper-line oracle mappings** (§6.1): resolved against each engine's exposed
   variables/lines; lines an engine cannot express are compared single-oracle (still adjudicated by the
   other).
