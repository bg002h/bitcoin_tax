# SPEC-oracle-sweep — independent Fable architect re-review, r2

*Persisted VERBATIM (STANDARD_WORKFLOW §2) before folding. Reviewer: Fable (independent architect pass, r2
— re-review after the r1 fold). Reviewed against `main`, clean tree. Persisted 2026-07-15. NOT green
(0C/2I) — fold to r3 follows.*

---

VERDICT: 0 Critical / 2 Important / 6 Minor / 1 Nit

# SPEC-oracle-sweep — independent Fable architect re-review, r2

*Reviewer: Fable (independent architect pass, r2 — re-review after the r1 fold). Reviewed against `main`, clean tree. Every disposition below was checked against current source, not against the spec's own account of it. The r1 findings were 2C/6I/7M/3Nit; this pass verifies each fix and then hunts the fold's own seams.*

**Bottom line: the fold is real.** Both Criticals were rewritten around the correct in-repo patterns and the source now supports the spec's load-bearing claims — the printed-chain classification in §6.2 is accurate against `printed.rs`, the `golden_packet.rs` evolution in §3.4/§7 is the right disposition, and the deferrals (MFS, AMT) are honestly flagged. But the fold is **not yet green**: the C-1 fix leaves the comparison rule ambiguous on exactly one line-family (the Tax-Table tax and every total it flows into), where its two normative statements contradict each other; and the C-2 fix disposes one of the two same-JSON consumers and is silent on the other, which goes red at bake time exactly the way r1 described. Both are paragraph-sized spec fixes, not design rewrites.

---

## Disposition of r1 findings

| r1 | Disposition | Where / evidence |
|---|---|---|
| C-1 (printed chain cross-foots; exact-match rule manufactures reds) | **Resolved — but introduced a new seam** (→ r2-I1) | §6.2 rewritten around the `golden_packet.rs:81-131` reproduction pattern; classification verified correct against `printed.rs` (see below) |
| C-2 (corpus growth breaks consumers; divergences don't scale) | **Partially resolved** (→ r2-I2) | `golden_packet.rs` disposed (§3.4, §7 derived expectations); class mechanism specified (§6.4); `golden_returns.rs` — the second same-JSON consumer — never disposed |
| I-1 (AMT refusals; unreadable guard line) | **Resolved** | D-2 refusal-freedom + oracle-side admission predicates (§4); no paper AMT line (§6.1); sound — see checkability note below |
| I-2 (MFS unsupported; taxcalc silently answers Single) | **Resolved** | Deferred with the harness work enumerated (§2) and flagged for the user (§14.1); citations `testonly.rs:481-486`, `:87`, `gen_goldens.py:222` all verified |
| I-3 (pairwise can't deliver §12; no constraint model) | **Resolved** | §5.1: variable strength (t=3 named triples) + constraints layer + explicit pinned cells; tooling feasible (PICT/ACTS both do mixed-strength-with-constraints; the pinned cells carry the §12 obligation even hand-rolled) |
| I-4 (sign semantics; parenthesized magnitudes) | **Resolved** | §6.3 sign table + capped-loss KAT; verified against `printed.rs:387-390` (L7 leading minus; Sch D L6/14/21 magnitudes) |
| I-5 (8995 L12 passes by construction vs OTS) | **Resolved** | §6.4 marks it single-witness/weak with closure options; §14.2; matches `ots_direct.py:19-33, 283-304` |
| I-6 (runtime blows the 6s gate) | **Resolved** | §8: budget, sharding, anchors-only determinism loops, measured fallback (one factual misstatement → r2-M1) |
| M-1 (generated-date breaks determinism claim) | Resolved — §12 excludes `_provenance.generated` (`gen_goldens.py:306` verified) |
| M-2 (goldens location already decided) | Resolved — §3.4 states the in-source decision (`testonly.rs` `include_str!` comment verified) |
| M-3 (no version-drift policy) | Resolved — §11 (version-gated, reviewed regeneration) |
| M-4 (asymmetric pass rule; all-required schema) | Resolved — §6.4 symmetric rule + `Option` schema change (`testonly.rs:394-421` verified all-required `f64` today; serde `Option` is the right change, not `#[serde(default)]`) |
| M-5 (verify_flat oversold) | Resolved — §3.2 "narrows… does not eliminate" |
| M-6 (blank regimes conflated) | Resolved — §6.3 two regimes, per-line tag |
| M-7 (baseline misstated) | Resolved — §1 recast as extend/unify/scale with accurate citations |
| N-1 (extract_lines signature) | Resolved — §3.2 shows `Result<…, FormsError>` |
| N-2 (F6251 citation) | Resolved — removed; §6.1 "No AMT/credits paper line" |
| N-3 (CLI vs harness binary) | Resolved — §9 test-only harness binary with the vault-reconciliation reasoning |

**C-1 classification spot-check (requested):** verified line-by-line against source. *Leaf:* Sch SE L10/L11 are `round_dollar(se.ss)` / `round_dollar(se.medicare)` (`printed.rs`, `schedule_se_lines`). *Cross-footed:* SE L12 = printed L10 + L11 (`printed.rs:233`); 1040 L9 = Σ printed 1z/2b/3b/7/8 (`printed.rs:541`), L11 = L9 − L10, L15 = (L11 − L14).max(0), L24 = L22 + L23 (`printed.rs:627`); 8959 L18 = line7 + line13 (`other_taxes.rs:178`). *Table:* L16 = `qdcgt_line16(table, printed L15, L3a, printed Sch-D figure)` (`printed.rs:~600-612`). All as the spec's table states. The claimed component-extraction burden is real but **not unspecified-impossible**: `ots_direct.py:164-171` already parses *every* `Lxx = value` line of each OTS output file into a dict (the driver merely doesn't return them), and taxcalc exposes component arrays (`c02900`, `c04470`, `standard`, `c23650`, …) — "deepening the extraction" (§6.2) is an honest description, and the §6.2 fallback ("where an oracle exposes only the total, that line is single-witness against the oracle that exposes the components") correctly covers taxcalc's lump-sum lines (`ptax_amc`, `niit`). Cross-crate feasibility of "reproduce the printing" also holds: `qdcgt_line16` is `pub` in `pub mod method` (`tax/mod.rs:10`, `method.rs:74`) and `ty2024_table()` is exported via `testonly`.

**D-2 checkability (requested):** sound. btctax's AMT screen is deliberately *over*-estimating (`amt.rs:1-5`: "clearing it means no AMT"), so "btctax assembles" already proves zero AMT; the oracle-side zero-AMT/zero-credit predicates are a redundant belt, checkable at minimum via taxcalc (`c09600`, credit arrays) and via the same OTS output files the driver already parses. A refusal that slips through anyway panics loudly at bake time (`return_refuse.rs:161`), before commit. Two Minor wrinkles below (r2-M4, r2-M5).

---

## Important

### r2-I1 — §6.2's Table row is ambiguous where it matters most: as written it either reintroduces the manufactured reds on L24 or silently removes the oracle's opinion of the *tax* from the whole corpus

**Anchors:** spec §6.2 (the one-line rule; the class table's Cross-footed and Tax-table rows); `crates/btctax-forms/tests/golden_packet.rs:120-131` (today's L24 cross-foot uses `round_dollar(e.income_tax_before_credits)` — the **oracle's own L16** — as a component, and L16 itself is held directly against `round_dollar(oracle L16)` at `:129`); `crates/btctax-core/src/tax/printed.rs:~600-612` (L16 = Table/QDCGT on the **printed** L15).

The spec's two normative statements diverge on the tax component:

- The one-line rule — "push the oracle's figures through btctax's own §3.1 printing" — read faithfully, means the L16 expectation is `Table(reproduced printed TI)` and the L24 expectation uses **that** figure as its tax component. This is internally consistent and red-free. But `Table` here can only be **btctax's own lookup** (`qdcgt_line16` + `ty2024_table`), applied to a TI reconstructed from oracle leaves. The oracle's *own* L16 figure is then **used nowhere**: a Tax-Table semantics bug in btctax (wrong bin midpoint; rate-schedule-below-$100k; a QDCGT worksheet error on the ordinary remainder) applies identically to both sides of the comparison and passes the entire generated corpus. That is the `golden_returns.rs:305-311` anti-pattern — a check that cannot fail — landing on the single most consequential line, and it is a **regression against today's on-paper test**, which holds L16 against the oracle's figure exactly (`golden_packet.rs:129`).
- The class table's Cross-footed row — "held against `Σ round_dollar(oracle_component)`", with L24 as the example, matching today's `golden_packet.rs:120-123` — keeps the oracle's L16 as a component. Then the C-1 residual returns in a new coat: OTS applies the Table to *its* exact-cents TI; the paper's L16 is Table(printed L15). When the two TIs straddle a $50 bin edge, they differ by a whole bin step (up to ~$18.50, `printed.rs` L16 comment) — impossible on today's whole-dollar anchors, but the generated corpus's SE/QBI households carry cents into TI (×92.35%, half-SE at $X.865, …), making a straddle a ~1–2%-per-household event. Over ~100 baked households plus the sweep, that is an expected ≥1 **undeclared red** with no declared class to receive it — Invariant L-1 violated again, at low frequency instead of systematically.

**Concrete failure (horn 1):** btctax's Table lookup taxes each bin at its lower edge instead of its midpoint. Every generated household's paper L16 equals `Table_btctax(reproduced TI)` by construction → the whole corpus is green → a wrong tax ships. **Concrete failure (horn 2):** a generated SE household's reproduced printed TI is $47,150 while OTS's exact TI is $47,149.87 (bin edge $47,150) → OTS's L16 is one bin lower → paper L24 ≠ Σround(oracle components) → undeclared red.

**Fix direction:** make the rule two-part, per line-family, in §6.2: (a) **structural** — paper L16 = `Table_btctax(reproduced printed TI)`, exact, no tolerance (catches fill/transcription and chain bugs); (b) **witness** — `Table_btctax(reproduced printed TI)` vs `round_dollar(oracle L16)`, exact, with one declared divergence **class** `(any oracle, L16-family, reproduced-TI and oracle-TI straddle a $50 bin boundary)` — the predicate is computable from the two TIs, which the deepened extraction already carries. State explicitly that L24's tax component is the part-(a) figure, so the total inherits the same two-part treatment. This restores the oracle's opinion of the tax while keeping the corpus red-free, using only machinery §6.4 already defines.

### r2-I2 — `golden_returns.rs` is the second consumer of the same grown JSON, and the spec still never says what happens to it; as written, bake day is red in `btctax-core` before the evolved test exists

**Anchors:** spec §3.4 ("the baked JSON **stays** at `btctax-core/tests/goldens/full_return_goldens.json`… read via `golden_households()`"), §7 (`gen_goldens.py`'s `HOUSEHOLDS` becomes the generated array — same output file, `gen_goldens.py:50-51`); `crates/btctax-core/tests/golden_returns.rs:221-238` (loops `golden_households()` — all of them), `:94-213` (per-household `DECLARED_DIVERGENCES` with baked `dec!` figures — six entries, five of them the taxcalc Tax-Table L16 divergence), `:388-401` (dead-entry liveness guard), `:297` (its own L24-vs-OTS single-witness comparison, which carries the Σround-vs-roundΣ per-household entry at `:157-191`).

r1 C-2's fix direction was "enumerate the fate of **every** `golden_households()` consumer." The live tree has exactly two test consumers: `golden_packet.rs` (disposed thoroughly — §3.4, §7) and `golden_returns.rs` (mentioned only as provenance in §1 and as a citation source in §6.4). §7's heading claims "Making 'corpus-size-agnostic' true (not assumed)" and concludes "Only then does adding a household need no Rust edit" — but that is only true of the forms test. Grow the JSON to ~100 households and `every_golden_household_matches_the_independent_oracles` runs over all of them at compute level: every generated household with taxable income (or QDCGT ordinary remainder) under $100,000 diverges from taxcalc on L16 → an **undeclared** per-household diff → red; and every SE household whose component cents conspire re-creates the L24 Σround-vs-roundΣ entry per household. The §6.4 class mechanism is exactly the cure — but §6.4 specifies it for the evolved on-paper test, and nothing in the spec applies it to, restricts, or retires the compute-level test.

**Concrete failure:** implement the spec as written (evolve `golden_packet.rs`, regenerate the JSON, touch nothing in `btctax-core/tests/`) → `make check` fails in `golden_returns.rs` with dozens of undeclared taxcalc L16 diffs, and the implementer must invent the missing disposition mid-implementation — the precise situation the spec gate exists to prevent.

**Fix direction:** one paragraph in §7 (or §3.4) choosing and stating `golden_returns.rs`'s fate: (i) it adopts the same declared-class mechanism and runs the full corpus at compute level (cheap — no PDF fills — and it is the layer that witnesses btctax's Table semantics against OTS, which r2-I1 needs); or (ii) it runs anchors-only; or (iii) it is absorbed into the evolved test's three-way internal-vs-oracle comparison (§6.5) and retired. Whichever is chosen, reconcile its dead-entry liveness guard with the class form (see r2-M6).

---

## Minor

- **r2-M1 — §8 misstates nextest's execution model, in the direction that would defeat its own mitigation.** "nextest parallelizes *across* test binaries, not within one" (echoing `Makefile:15/33`'s shorthand) is false: nextest runs each **test** in its own process, parallel across the whole run regardless of binary. If the spec's statement were true, sharding the loop across `#[test]` functions *within one binary* would buy nothing. The sharding design is sound precisely because the stated premise is wrong. Correct it to r1's wording: parallel across tests, serial within a single `#[test]`.
- **r2-M2 — the §6.2 three-class taxonomy is not exhaustive, and one example is misfiled.** `printed.rs`/`other_taxes.rs` contain a fourth pattern — *rate-on-printed-operand* — e.g. 8959 L7/L13 (`round(0.9% × printed L6/L12)`, `other_taxes.rs:167,173`) and 8960 L17 (`round(3.8% × line16)`, `other_taxes.rs:321`); and 8960 L13 is `round_dollar(exact AGI)` (`other_taxes.rs:317`), *not* the 1040's cross-footed L11. The general rule ("push oracle figures through btctax's §3.1 printing") subsumes all of these, but the plan must derive each compared line's reproduction from `printed.rs`/`other_taxes.rs` line-by-line rather than treating the three classes as a complete enumeration. Also "8960 base" as a *Leaf* example is imprecise (line 8 is itself a Σ of rounded leaves).
- **r2-M3 — derived form-set expectations (§7) lose the hand-pinned anchor sets without a stated compensating KAT.** The derivation re-implements core's assembly triggers; a systematically wrong derivation plus a matching filler bug would pass silently (dropped forms are a named historical bug class — `golden_packet.rs:283-297`). Cheap fix: the 12 anchors' hand-written form sets (`golden_packet.rs:300-350`) stay as pinned data and §12 gains the obligation that the derivation function reproduces them — the derivation then covers only generated households, anchored by twelve known-answer sets.
- **r2-M4 — D-2's "admitted only if btctax assembles it" is enforced *at generation time*, but generation is Python and assembly is Rust; the mechanism is unstated.** The natural vehicle is the §9 test-only harness binary (invoke it per candidate), and the loud-panic backstop makes the failure mode safe either way — but the spec should say which, so the plan doesn't reinvent a Python re-implementation of the AMT screen that can drift.
- **r2-M5 — "zero credits" (D-2) needs a scope, or it silently constrains the covering array's low-income cells.** taxcalc computes childless EIC automatically below ~$18.6k (Single) / ~$25.5k (MFJ); an unqualified zero-credits predicate rejects every low-W-2-only cell, and the rejection surfaces as an unsatisfiable array cell at generation time. Either scope "credits" to the L18–L21 band (what the L24 cross-foot precondition — `golden_packet.rs:104-119` — actually requires; EIC is payments-side and touches no compared line) or fix the "low" W-2 band above the childless-EIC domain in the plan.
- **r2-M6 — the class mechanism drops the liveness guard the per-household model had.** `golden_returns.rs:388-401` fails any declared entry that never fires, so explanations cannot rot. §6.4/§10 carry no analogue for classes — a taxcalc release adopting Table semantics would leave the L16 class as an unread claim forever. Require each declared class to fire for ≥1 corpus household, same principle, predicate form.

## Nit

- **r2-N1 — §6.2's Tax-table row:** the "btctax prints" column carries the QDCGT variant but the "held against" column says only `Table(reproduced printed TI)` — the QDCGT-on-reproduced-operands variant should appear on both sides. (Subsumed by the r2-I1 rewrite; noted so it isn't lost if r2-I1 is fixed narrowly.)

---

## Strengths (brief)

The fold engaged every r1 finding on the merits — nothing was waved through, the two refuted claims were rewritten around the in-repo patterns r1 pointed at, and the new sections (§8 budget, §11 drift policy, D-2/D-3 invariants, the §6.3 sign table) are verified accurate against source, including the honest single-witness labeling of 8995 L12 and the correct settlement of the goldens-location question. The remaining defects are concentrated in two paragraphs of §6/§7, and both are resolvable with machinery the spec already defines.

**Not green: 0 Critical / 2 Important blocks the gate. Fold r2-I1 and r2-I2 (and the Minors as convenient) and re-review.**
