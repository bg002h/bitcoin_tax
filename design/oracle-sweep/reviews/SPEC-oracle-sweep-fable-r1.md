# SPEC-oracle-sweep — independent Fable architect review, r1

*Persisted VERBATIM (STANDARD_WORKFLOW §2) before folding. Reviewer: Fable (independent architect pass).
Reviewed against `main`, HEAD `0603969` (the spec was committed at `b967340` on the same tree). Persisted
2026-07-15. NOT green (2C/6I) — fold to r2 follows.*

---

VERDICT: 2 Critical / 6 Important / 7 Minor / 3 Nit

# Fable architecture review — `SPEC_oracle_sweep.md` (r1)

Reviewed against source at `main` (clean tree, HEAD `0603969`). Every load-bearing claim was checked against the actual files; runtime claims were measured, not estimated. Verdict: the spec's architecture (paper as the comparison surface, hermetic baked corpus, non-CI sweep, divergence lifecycle) is sound in outline and most of its infrastructure citations are accurate — but two of its central claims are refuted by the source it cites, and both refutations land on the same spot: **the design as written bakes a corpus that cannot be green**, violating its own Invariant L-1 at birth.

---

## Strengths (brief)

- **The direction is right and already half-proven in-repo.** `crates/btctax-forms/tests/golden_packet.rs` already fills the packet for the 12 golden households and holds the paper against OTS (1040 L11/L15/L16/L24, Schedule SE L12, the Schedule A SALT-cap lines). The spec's paper-first philosophy is the codebase's own trajectory, not an invention.
- **Hermeticity claim fully verified.** `btctax-forms` depends on `lopdf` with `default-features = false` and nothing network-capable (`crates/btctax-forms/Cargo.toml`); all 14 blank 2024 PDFs are committed and `include_bytes!`-embedded (`crates/btctax-forms/src/pdf.rs:24-32`, `forms/2024/`). A gating CI test that fills and reads real PDFs offline is genuinely feasible.
- **The cross-crate reachability claim holds.** `pub mod testonly` (`crates/btctax-core/src/tax/mod.rs:24`) is deliberately a library module for downstream crates; `golden_packet.rs:32` already imports `build_golden_household`/`golden_households` from it. "Third consumer of the same builder" is accurate, not aspirational. No `pub(crate)` seam bug here.
- **Both read-back layers exist and are correctly characterized** — including the honest statement that `extract_lines` goes through the map and cannot catch a mis-map (`transcribe.rs:13-18`). Better than the spec claims, the geometric `verify_flat` oracle covers the *scalar* forms too and runs inside every filler automatically (`form1040_full.rs:241`, `schedule_a.rs:119`, `schedule23.rs:77/128/191`, `form8959.rs:105`, `form8960.rs:89`, `form8995.rs:270`, `schedule_c.rs:115`, `schedule_d_full.rs:304`, `schedule_se_full.rs:124`, `schedule_b.rs:195`).
- Keeping the sweep out of the gate, the two-oracle adjudication model, and the promotion lifecycle are all the right calls in principle.

---

## Critical

### C-1 — §6.2's "strictly more faithful, with no regression" claim is false: the printed chain cross-foots at every totaling line, and the spec's exact-match rule turns that lawful drift into un-declarable reds

**Anchors:** spec §6.2–§6.3; `crates/btctax-core/src/tax/printed.rs:5-8` (module header: each printed total "sums the already-rounded lines above it… deliberately NOT `round_dollar(exact_total)`"), `printed.rs:233` (Sch SE L12 = printed L10 + printed L11), `printed.rs:292` (Sch 2 L21), `printed.rs:540` (1040 L9), `printed.rs:607-610` (1040 L16 = Tax Table applied to the **printed** L15 — "differ by a whole bin step (up to ~$18.50)"), `printed.rs:627` (L24); `golden_returns.rs:287-297` (the current test's careful absolute-vs-printed split).

The spec asserts: *"under the §3.1 round-all-amounts election a printed component line **is** `round_dollar` of its exact value, so the six component lines read off the paper equal today's numbers by construction."* The source says the opposite, in its own header. Concretely, of the eight headline lines the spec would now read off the paper:

- **SE tax** (Sch 2 L4 ← Sch SE L12) is `round(L10) + round(L11)` (`printed.rs:233`), not `round(exact SE tax)` — the two differ by $1 whenever the two parts' cents patterns conspire (`printed.rs:1488` says so verbatim).
- **Add'l Medicare** (8959 L18) is `line7 + line13` over printed parts (`other_taxes.rs:94`).
- **AGI** (L11) and **taxable income** (L15) sit atop `line9 = Σ printed components` (`printed.rs:540`).
- **Tax** (L16) is `Table(printed L15)` with printed QDCGT operands — the source itself documents divergence from `Table(exact TI)` "by a whole bin step (up to ~$18.50)" (`printed.rs:607-610`, `printed.rs:1588`).

The oracles report exact cents. So moving the comparison to the paper introduces a **new, lawful, systematic disagreement class on the very lines the current test matches exactly** — precisely the class the current test quarantines to L24 alone with a 250-word declared divergence (`golden_returns.rs:157-191`). The current 12 households pass `golden_packet.rs` only because whole-dollar inputs keep cents out of most lines; cents enter through every multiplication (×92.35%, ×12.4%, ×2.9%, ×0.9%, ×3.8%, ×20%), i.e. exactly the SE/8959/NIIT/QBI paths the covering array multiplies, and §5.2's threshold-biased amounts make it worse.

**Concrete failure:** a generated SE household whose exact OASDI part is \$X.50 and Medicare part \$Y.50 → paper Sch 2 L4 = exact + 1 → disagrees with `round_dollar(exact)` from **both** oracles → under §6.3 (exact match, no tolerance) an undeclared red; under §6.4 the only escape is a per-`(household, line)` `Divergence` whose `agrees_with` must say `"neither"` (`golden_returns.rs:358-372`) with statutory justification, hand-written in Rust, per household, with baked `dec!` figures, each policed by the dead-entry liveness assertion (`golden_returns.rs:388-401`). Multiply by ~100 households × ~15 lines. Invariant L-1 is unsatisfiable as designed.

**Fix direction:** the repo already contains the correct pattern — `golden_packet.rs:81-123` compares the paper L24 to the **oracle's components pushed through the same §3.1 printed-chain arithmetic** (Σround of the oracle's lines), needing no divergence entry at all. §6.2/§6.3 must be rewritten to specify, per compared line, the *function of oracle outputs* the paper is held against (round-at-the-line for leaf lines; Σround-of-oracle-components for cross-footed totals; `Table(printed L15)` semantics for L16) — or, less good, a principled ±$1 residual rule on Σ-lines plus a divergence-*class* mechanism. What it cannot keep is "exact after `round_dollar`, no tolerance" against exact-cents oracle figures.

### C-2 — "Growing the corpus from 12 to ~100 needs **no Rust change**" (§7) is false three ways, and the spec never disposes of the one existing test that breaks first

**Anchors:** spec §7, §8, §13.2; `golden_packet.rs:353-357` (panic: *"a household was added and its packet went unchecked"*), `golden_packet.rs:181-184`, `:564-567`, `:616-619` (hard `checked == 3` counts), `:16-22` (its OTS-only, no-divergence adjudication premise); `golden_returns.rs:94-213` (static `DECLARED_DIVERGENCES` with baked `dec!` figures); `golden_returns.rs:102-104` (the Tax-Table divergence is *systematic below $100k*, not household-specific).

1. **`golden_packet.rs` is a same-JSON consumer the spec acknowledges (§8: "alongside the existing `golden_packet.rs`") but never reconciles.** It loops `golden_households()` and panics on any household absent from its hand-written per-name form-set map; three of its tests assert *exactly 3* SE/Schedule-C households. Swap in a generated corpus and `make check` is red before the new test exists. §13.2 debates the fate of `golden_returns.rs` (core) and is silent on the forms test that actually breaks — the closer relative, whose role the new test duplicates under a different adjudication model.
2. **The taxcalc Tax-Table divergence does not scale as per-household entries.** taxcalc lands "a few dollars away on precisely the households where the Table is mandatory" — i.e., on essentially **every** generated household with taxable income (or QDCGT ordinary remainder) under $100,000. Today that is 6 hand-written Rust entries; over a ~100-scenario covering array with "low/mid" W-2 bands it is plausibly 40–60, each with exact `dec!` figures that must be re-derived at every regeneration, each policed by the liveness guard. The divergence model needs a *class/predicate* form ("taxcalc, L16, when TI < $100k, Table-vs-schedule") before the corpus can grow — a Rust design change §6.4 explicitly disclaims ("unchanged, extended per line").
3. **The MFS axis requires Rust changes in the cited fixtures** (detailed as I-2).

**Fix direction:** the spec must enumerate the fate of every `golden_households()` consumer (extend/supersede/refactor `golden_packet.rs` with *derived*, not hand-written, form-set expectations — e.g., computed from the household's inputs against the trigger thresholds) and replace per-household divergence entries with a declared divergence-class mechanism. "Corpus-size-agnostic" must be made true before it is claimed.

---

## Important

### I-1 — The domain (§4) ignores btctax's refusal surface; the "AMT guard line" cannot be read off any paper and mis-models the actual failure mode

**Anchors:** spec §4, §6.1 ("Guard lines… AMT (Form 6251 / 1040 L16 alt)"); `crates/btctax-core/src/tax/amt.rs:1-5` (v1 "does not compute Form 6251; when the worksheet concludes the taxpayer must fill it in, the return is **REFUSED**"); `return_refuse.rs:161` (`AmtScreenTriggered`); `printed.rs` Schedule2Lines doc ("Schedule 2 Part I is blank in v1… nothing in Part I is printed"); `forms/2024/` (no `f6251.pdf` exists).

A generated scenario that trips the AMT screen produces **no packet at all** — `assemble_printed_return` refuses — while both oracles happily compute the return. That is neither a divergence nor a fill bug; it is a scenario outside btctax's v1 domain, and the generator has no stated constraint keeping the covering array inside the refusal-free region (high-income × itemized × preferential-slice cells approach the screen). Separately, the guard line as specced is unreadable: there is no Form 6251 in the repo and Schedule 2 Part I never prints, so "AMT read off the paper" has no cell to read.

**Concrete failure:** a covering-array cell (MFJ, high band, itemized, both preferential slices) trips `AmtScreenTriggered` → the differential test panics on packet assembly → the baked corpus is red with nothing to declare.

**Fix direction:** make refusal-freedom a generation-time invariant (run each candidate scenario through btctax's screen during `gen_goldens.py`-side generation, reject/adjust trippers), and restate the AMT/credits guards as *oracle-side triviality assertions* (both oracles must report 0 for the scenario to be admissible) rather than paper read-backs.

### I-2 — The MFS axis is unsupported by every piece of cited infrastructure, and one oracle would silently answer a different question

**Anchors:** spec §4, §5.1; `testonly.rs:485` (`build_golden_household` panics on any status but `"Single"`/`"Married/Joint"`), `testonly.rs:87` (fixture table: "MFS/HoH pricing is not exercised" — no MFS ordinary/LTCG schedules at `:88-142`); `gen_goldens.py:222` (`"MARS": 2 if … "Married/Joint" else 1` — **MFS maps to taxcalc Single**); `return_1040.rs:384` (`mfs_spouse_itemizes` is a live answered-ness input).

btctax's compute genuinely supports MFS (`tables.rs:180/204/216` — $125k NIIT/Add'l-Medicare thresholds, $1,500 loss cap; `return_1040.rs:323` — $5k SALT cap), but the *harness* does not: the builder panics, the fixture table lacks MFS brackets, and the taxcalc driver would run MFS households as Single — whose standard deduction coincides but whose NIIT/Add'l-Medicare thresholds ($200k vs $125k) and loss cap ($3,000 vs $1,500) do not. That is exactly the apples-to-oranges artifact §4's own comparability principle exists to forbid, and it would surface as false "divergences" attributed to btctax. MFS also activates `mfs_spouse_itemizes`, an answered-ness question Invariant D-1 does not mention.

**Fix direction:** the spec must list the required extensions as in-scope work — `testonly.rs` MFS brackets/breakpoints and builder arm (Rust), `MARS: 3` and the OTS MFS status string (Python) — and extend D-1 to cover the MFS-specific questions.

### I-3 — §5.1's pairwise strength cannot deliver §12's own validation requirement, and no constraint model is specified

**Anchors:** spec §5.1, §12; `gen_goldens.py:177-192` (the 8995-L12 household needs SE profit × LTCG × qualified dividends **simultaneously**).

§12 requires every deeper compared line to be load-bearing in at least one corpus scenario. The flagship example — 8995 L12's qualified-dividend term — is a **3-way** interaction: pairwise guarantees SE×LTCG, SE×QD, and LTCG×QD each occur *somewhere*, possibly in three different scenarios, never together. Today it is held only by the pinned anchor. The same holds for the other interactions the spec itself names as dangerous (itemized × SALT-over-cap × high-income). Separately, the axes carry unstated constraints: "SALT position" is conditional on itemized; "itemized" × low-W-2 cells produce scenarios where itemizing loses (and OTS's `A18: Yes` forcing vs btctax's take-the-larger election becomes its own comparability question, `ots_direct.py:261-268`); the all-none row is a degenerate zero-income scenario.

**Fix direction:** specify a variable-strength array (t=3 over named axis triples, t=2 elsewhere) or explicit seeded cells for every §12 load-bearing obligation, plus a constraints layer (pairwise-with-constraints is standard) — in the spec, since §12's guarantee depends on it, not in the plan.

### I-4 — Read-back sign semantics are unspecified where the paper carries magnitudes in parenthesized boxes — and the §1211(b) axis guarantees the case in v1

**Anchors:** spec §6.1 ("Schedule D net gain reaching 1040 L7"), §6.3 ("parsed to integers"); `printed.rs:387-390` (Form1040Lines doc: "**Line 7 is the one signed cell**, signed with a LEADING MINUS… unlike Schedule D's own lines 6/14/21, which are parenthesized boxes carrying **magnitudes**").

1040 L7 prints `-3000`; Schedule D line 21 prints `3000` inside a pre-printed parenthesized box, *meaning* −3,000. A sign-blind integer parse compares +3,000 to an oracle's −3,000 → spurious red on a correct return; the reflexive `abs()` fix would then mask a genuine sign-flip fill bug. The spec's parse discipline ("a value that fails to parse is itself a failure") does not help — `"3000"` parses fine.

**Fix direction:** a per-line sign-convention table in §6.3 (which cells are signed, which are magnitude-in-parenthesized-box, and the normalization applied before comparison), with the capped-loss household as its KAT.

### I-5 — The 8995-L12 deeper line has no independent witness as designed: against OTS it passes by construction

**Anchors:** spec §6.1 (L12 "and its inputs"); `ots_direct.py:19-33` ("★ A LIMIT on this oracle's independence — say it out loud"), `ots_direct.py:283-304` (the harness **hand-computes** L12 from the household inputs and feeds it to OTS).

OTS's Form 8995 cannot infer net capital gain; our driver computes it (in Python, from the same §1222(11) formula) and hands it over. Comparing the paper's L12 to "OTS's L12" therefore compares btctax to a restatement of the same formula — it cannot fail on a wrong-formula bug. Under §6.4's pass rule ("passes when OTS agrees AND (taxcalc agrees OR reports no comparable figure)"), if taxcalc exposes no L12-granular output (its `qbided` is L15), the line passes *always*: a guard that cannot fail — the exact anti-pattern this very test file fixed once already (`golden_returns.rs:305-311`).

**Fix direction:** mark L12 single-witness in the spec's line table, resolve whether taxcalc exposes an L12-comparable variable, and/or adopt `ots_direct.py`'s own proposed fix (derive OTS's L12 from OTS's Schedule D output, closing the loop).

### I-6 — CI runtime: measured ~150–250 ms per packet fill; the design plausibly turns the ~6 s sacred fast gate into ~30–40 s, and the spec sets no budget

**Anchors:** spec §5.1 (~80–120 scenarios), §7; `Makefile:6-15` (`make check` ≈ 6 s warm, `--workspace` via nextest); measured: `cargo nextest run -p btctax-forms --test golden_packet` → `every_golden_household_prints…` 2.7 s (12 fills), `the_whole_packet_is_byte_reproducible` 4.1 s (24 fills).

A single serial `#[test]` looping ~100–120 households (the shape of `golden_returns.rs`, which the spec says it preserves) costs ~20–30 s by itself — nextest parallelizes across tests, not within one. Worse, the *existing* whole-corpus loops scale with it: byte-reproducibility goes ~4 s → ~35 s, the identity sweep ~3 s → ~27 s. The project treats the 6-second gate as sacred (it is documented in the Makefile and in project memory); the spec never mentions runtime at all.

**Fix direction:** state an explicit wall-clock budget in §12; shard households across `#[test]`s (or parallelize the loop internally); reconsider whether every existing whole-corpus loop must run over the full generated array or only the named anchors.

---

## Minor

- **M-1 — §12's "byte-identical regeneration" is false as stated:** `gen_goldens.py:306` writes `"generated": date.today().isoformat()` into the JSON. Regenerating on another day differs. Pin or exclude the field from the determinism claim.
- **M-2 — §13.1 flags as open a decision the codebase already made:** `GOLDEN_RETURNS_JSON` is exported from `testonly.rs:363-364` precisely because "`include_str!` cannot reach across a crate boundary without breaking `cargo package`" (`testonly.rs:358-359`). One of the two options the spec offers the plan is documented in-source as broken; the forms test should simply call `btctax_core::tax::testonly::golden_households()` as `golden_packet.rs` already does.
- **M-3 — No engine-version drift policy.** Provenance pins `OpenTaxSolver 2024 22.07` / `taxcalc 6.7.2`, but nothing says what happens to the baked corpus and the `dec!`-literal divergence entries when a version moves and answers shift wholesale. With ~100 households the churn is 8× today's. State the policy (regeneration is version-gated; a version bump is its own reviewed event).
- **M-4 — §6.4's pass rule is asymmetric (oracle-1 must agree) but §13.5 promises lines "an engine cannot express are compared single-oracle."** A line only taxcalc exposes cannot pass the current rule, and `ExpectedOts` is all-required `f64` (`testonly.rs:397-409`) — supporting OTS-absent lines is a schema and logic change the spec doesn't note.
- **M-5 — §3.2 oversells the geometric guard** ("a mis-mapped cell cannot hide behind a right-looking number"). `verify_flat` catches cross-column and descent-order mis-maps; a mis-map onto a same-column widget that preserves descent ordering evades it. "Narrows sharply" is defensible; "cannot" is not.
- **M-6 — §6.3's blank-as-zero conflates two regimes.** The 1040 filler writes explicit zeros for mapped computed lines — `golden_packet.rs:104-119` *depends* on present-and-zero and says defaulting absent→0 "would silently make this guard vacuous." The spec should state which lines are asserted present-and-zero (dropped-line detection) versus legitimately absent-form ⇒ 0.
- **M-7 — §1 gap 2 and the §3.1 "Before" diagram misstate the baseline.** "The comparison stops at the compute engine" is false: `golden_packet.rs` already holds the paper against OTS on the 1040 headline lines, Schedule SE L12, and the Schedule A SALT-cap lines. The claimed-new catch classes ("none of them did before", §3.1) are partially caught today. Recast the motivation as *extend/unify/scale* — it also changes the sizing.

## Nit

- **N-1** — §3.2 quotes `extract_lines(pdf, map_toml) -> BTreeMap<String, String>`; the real signature returns `Result<…, FormsError>` (`transcribe.rs:40`).
- **N-2** — §6.1's "AMT (Form 6251 / 1040 L16 alt)" names a form that exists nowhere in the repo (no PDF, no filler, no compute); covered substantively by I-1, but the citation itself should go.
- **N-3** — §9/§13.3: the `export-irs-pdf` CLI exists (`btctax-cli/src/cli.rs:899`), but `build_golden_household` fabricates `LedgerState` directly, bypassing vault reconciliation — the "CLI over a synthetic vault" branch means authoring a vault that reconciles to the same ledger, which is far from "thin." The constraint ("same on-paper values") effectively forces the harness-binary option; the spec could say so.

---

## Feasibility of the load-bearing claims

| # | Claim (spec §) | Verified against | Held? |
|---|---|---|---|
| 1 | Current harness: 8 compared lines, two-oracle pass rule, `Divergence` model keyed `(household, line)`, anti-"btctax against the world" guard, components-vs-absolute / total-vs-printed split | `golden_returns.rs:249-298, 311-314, 317-321, 358-372` | **Yes** — described accurately, including the L24 printed-chain nuance |
| 2 | `extract_lines` semantics + "cannot catch a mis-mapped cell" limitation | `transcribe.rs:13-18, 40` | **Yes** (signature nit N-1) |
| 3 | `verify` geometric guards available for the scalar 1040/schedule lines, not just 8949/Sch-D grids | `verify.rs:337` (`verify_flat`) + call sites in all 12 fillers | **Yes — stronger than claimed**: runs inside every filler already (M-5 caveat on "cannot hide") |
| 4 | Committed blank-PDF set (1040, S1/2/3, A/B/C, SE, D, 8949, 8959, 8960, 8995, 8283) | `forms/2024/` listing | **Yes** — all present with maps |
| 5 | Hermetic: fill + extract + verify are pure `lopdf`, no network | `btctax-forms/Cargo.toml`, `pdf.rs:24-32` | **Yes** |
| 6 | Frozen files untouched by the design | spec §2 OUT; no compute change needed for the read-back | **Yes** (but note I-1: the *refusal* layer constrains the corpus even untouched) |
| 7 | Forms test can reach the household builder ("third consumer") | `tax/mod.rs:24`, `golden_packet.rs:32` | **Yes** — precedent already exists |
| 8 | §6.2: printed component line = `round_dollar(exact)`; paper comparison has "no regression" | `printed.rs:5-8, 233, 292, 540, 607-610, 627, 1488, 1588` | **No — refuted by the cited source itself** (C-1) |
| 9 | §7: corpus growth 12→~100 "needs no Rust change" | `golden_packet.rs:181-184, 353-357, 564-567, 616-619`; `golden_returns.rs:94-213, 388-401` | **No** (C-2) |
| 10 | §4/§5.1: MFS is in the comparability domain as-is | `testonly.rs:87, 485`; `gen_goldens.py:222` | **No** (I-2) |
| 11 | §12: regeneration is byte-identical | `gen_goldens.py:306` | **No** — the `generated` date field (M-1) |
| 12 | §13.1: goldens location is an open decision | `testonly.rs:358-364` | **Already decided in-source**; one offered option documented as broken (M-2) |
| 13 | Guard lines (AMT, credits) readable off the paper | `amt.rs:1-5`, `forms/2024/` (no f6251), printed Sch 2 Part I never prints | **No** (I-1, N-2) |

**Bottom line:** the architecture is worth building, and most of its foundations are real and verified — but the spec is not green-able as written. C-1 requires rewriting §6.2/§6.3 around the printed-chain arithmetic (the correct pattern already exists at `golden_packet.rs:81-123`); C-2 requires a scaling story for the divergence model and an explicit disposition of `golden_packet.rs`. The six Importants are all resolvable at spec level; none undermines the core idea.
