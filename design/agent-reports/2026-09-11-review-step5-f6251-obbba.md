# REVIEW — seam review of `d8d023af` (`f6251/2025` wired, the OBBBA-era Form 6251)

**Reviewer:** one opus reviewer, isolated worktree `2673df24`
(`/scratch/code/bitcoin_tax/.claude/worktrees/agent-a7c1491036b03618c`).
**Brief:** `design/agent-reports/BRIEF-review-step5-f6251-obbba.md`. **Nothing built, nothing committed,
no file edited but this one. No subagents.**

**Counts: 0 Critical / 2 Important / 3 Minor / 1 Nit.**

---

## Verdict

The forms-side trap is real and I could not evade it by reading. The `_`-free match, the derived
revision enumerators, the two rewritten instruments and the extract-derived label partition all
discriminate, and the emitter's three refusals all refuse. Seam 1's answer is the round's finding, and
it is worse than the build, the brief, or the source's own prose says — not because a live wrong-result
path exists (three independent gates block it, each verified below) but because the collision at
Schedule 1-A **line 37** is not a renumber. **Line 37 exists on BOTH schedules and denotes different
quantities of different orders of magnitude:** the TY2025 senior-deduction subtotal (capped near
$6,000) versus, on the TY2026 draft, *"Enter the amount from line 3"* — modified AGI. Every description
of this hazard in the repo calls it "37 → 43, a renumber". The second Important is a B3-shaped gap: the
new emitter's Part III — 29 of its 42 money cells, including the line-33/36 pair — is executed by no
test, and the accessor built for the sweep that would have covered it (`money_cells()`) has no reader,
while the TY2024 twin nine files away has exactly that sweep.

Both Importants are in the diff's own blast radius, neither is reachable in production today, and
neither is a defect in what the build *claims* — the claims are narrower than the prose around them,
which is Finding I-1's second half.

---

## Findings

### ★★★ I-1 — Important. The per-revision Schedule 1-A line number and the figure that lands in line 1a are never joined, in the one function that holds both; and the two lines COLLIDE rather than renumber.

**Mechanism, in three hops.**

1. `crates/btctax-forms/src/f6251_revision.rs:23-37` is headed **"How it is closed"** and names the
   defect it closes as: *"Schedule 1-A line 37 is the senior deduction subtotal and line 43 is its
   TY2026 renumber. **Read the wrong one and the AMT base is wrong** while every instrument stays
   green."* What is actually closed is narrower: a revision cannot be **wired** to this schema without
   stating its own printed cells (`E0004`, then `every_obbba_revision_has_cells`). Nothing joins the
   stated cell to the money.
2. The money is computed in core from a **field name**:
   `crates/btctax-core/src/tax/form6251.rs:399` declares `schedule_1a_l37: Usd`, and
   `crates/btctax-core/src/tax/return_1040.rs:2988-2990` fills it from
   `schedule_1a.and_then(|s| s.part5.line37)`. `form6251.rs:471` then computes
   `line1a = form_1040_l14 - schedule_1a_l37`. The cross-reference exists only as an **identifier** —
   the one form of a cell that no test, no `_`-free match and no extract comparison can read.
3. `crates/btctax-forms/src/form6251.rs:296` — the emitter, which has **both halves in scope** —
   resolves `revision.schedule_1a_line()` and **discards the value**:
   `revision.schedule_1a_line().ok_or_else(|| …)?;`. It is used as a liveness guard on the sentence,
   never as a comparison against the line the figure actually came off.

**The collision, measured — and this is the part nothing in the repo says.**

| schedule | line 37 | line 43 | source |
|---|---|---|---|
| TY2025 final | "Enhanced deduction for seniors. Add lines 36a and 36b" | — (schedule ends at 38) | `design/forms/extract/f1040s1a--2025.txt:108,110` |
| TY2026 draft | **"Enter the amount from line 3"** | "Enhanced deduction for seniors. Add lines 42a and 42b" | `design/forms/extract/f1040s1a--2026-DRAFT.txt:213,222` |

and TY2026 draft line 3 is *"Add lines 1 and 2e"* (row 59) where line 1 is *"Enter the amount from Form
1040, 1040-SR, or 1040-NR, line 11b"* (row 53) — i.e. **modified AGI**. TY2025's line 37 is bounded:
the worksheet above it is `$6,000 − 6% × (line 3 − $75,000)`, floored at zero
(`f1040s1a--2025.txt:99-103`).

So "line 37" on the TY2026 schedule is not a neighbouring deduction subtotal one line off. It is a
six-figure income magnitude sitting where a four-figure deduction is expected.

**Concrete failure scenario (inputs/state → wrong output).** TY2026 finals arrive. Someone bundles
`forms/2026/f6251.map.toml`, adds `LineSet::F6251_2026 => Schema::Form6251ObbbaMap`, states its cells
in `f6251_revision` (both forced, both observed), transcribes the TY2026 Schedule 1-A into core (which
then has **both** a `line37` and a `line43`), bundles `FullReturnParams`, and follows the porting
instruction written in the source at `return_1040.rs:2968-2972`: *"TY2026 is expected to REUSE the
`Y2025` shape rather than gain a variant … only the cited Schedule 1-A line moves, 37 → 43. Adding the
`2026 =>` arm is therefore a **one-line edit**."* The one-line edit is
`2026 => Some(Form6251Line1Rule::Y2025 { form_1040_l11b, form_1040_l14, schedule_1a_l37: …part5.line37 })`
— which compiles, because the TY2026 schedule really does print a line 37. A single filer with $300,000
modified AGI then gets `line1a = L14 − ≈300,000` (deeply negative) and
`line1b = L11b − line1a ≈ 600,000` — AMTI **overstated by roughly the whole AGI**, in the opposite
direction from the TY2025 understatement the existing kills pin. Meanwhile
`f6251_revision::revision(F6251_2026).schedule_1a_line() == Some(43)`, the emitter resolves it,
discards it, and prints the 37-derived figure into 1a's box. Read-back passes (the value lands in the
box it was sent to), the field map cannot see it (62 fields, 0 renamed), and both oracles take the
figure as input.

**Is there a live wrong-result path today? No — three gates, each verified in this worktree.**

1. `LineSet::F6251_2026` does not exist. `LineSet::ALL.len() == 38` (`line_set.rs:377`) and
   `f6251_revision::revision`'s `_`-free match lists 38 variants, one `Some` and 37 `None`.
2. `full_return_for(2026)` is a tested `None` (`crates/btctax-adapters/src/tax_tables.rs:1233`), and it
   is the only production entry to `assemble_absolute`.
3. `form6251_line1_rule`'s `_ => None` (`return_1040.rs:2992`) panics at the caller for any
   unenumerated year, pinned by a **derived** sweep over `1990..=2060` asserting `vec![2024, 2025]`
   (`return_1040.rs:12032-12040`).

So: **armed, not live → Important**, exactly as the brief's rule prescribes.

**Is the naming itself the compression the transcription rule forbids? Yes — but not for the reason
the follow-up gives.** A type named `Y2025` serving TY2026 is a lie about the revision and should be
renamed; that alone is a Nit. The violation is that the *year-varying cross-reference is held as a Rust
identifier*, so it can never be compared against the form. The forms crate solved precisely this by
holding the **sentence** and parsing the number out of it (`ObbbaRevision::schedule_1a_line`), with the
stated reason *"never a second field: a number typed beside the sentence could disagree with it"*. Core
holds neither the sentence nor a number — it holds a name. The `_`-free match cannot reach it because
there is nothing of the right kind to reach.

**What the existing gate does and does not buy.** Adding the `2026 =>` arm reds
`form6251_part_i_refuses_every_year_it_has_not_transcribed`. That red is discharged by editing the
expected vector to `vec![2024, 2025, 2026]`; the test asserts nothing about **which** Schedule 1-A line
the new arm reads. One stop sign, and no check on the thing that matters.

### ★★ I-2 — Important. The OBBBA emitter's Part III is executed by no test — 29 of its 42 money cells, including the line-33/36 pair — and `Form6251ObbbaMap::money_cells()` has no reader, though its own doc comment says the read-back sweeps iterate it.

**Mechanism.** `part_iii_completed` appears exactly **once** in `crates/btctax-forms/tests/f6251_obbba.rs`
— at line 774, set to `false` — and `part_i_only_2025()` is the fixture every fill test uses. So
`fill_form_6251_obbba_with_map`'s `if f.part_iii_completed { … }` branch
(`crates/btctax-forms/src/form6251.rs`, the 29-element `p3` array) is **never taken**. The
whole-dollar sweep has no OBBBA counterpart either (no `contains('.')` assertion anywhere in the file).

`Form6251ObbbaMap::money_cells()` is at `crates/btctax-forms/src/map.rs:811`, with the doc comment
*"the sweeps that read the FILLED page back (whole-dollar, paren-magnitude) iterate this list: a cell
missing from it is a cell no read-back ever checks, which is the quietest way for a line to stop being
verified while every test stays green."* Measured: `grep -rn money_cells crates --include=*.rs` yields
two definitions (`map.rs:412`, `map.rs:811`) and two consumers
(`tests/f6251_fill.rs:212`, `tests/attestation.rs:954`) — **both on `Form6251Map`**. The accessor
written for the sweep has no sweep.

**The B3 shape, and it is exact.** The guarantee already exists for the sibling revision:
`crates/btctax-forms/tests/f6251_fill.rs:190-227` routes Part III (`part_iii_completed = true`),
iterates `map.money_cells()`, and asserts `checked == 41` with the message *"a count that drifts means
the sweep stopped seeing cells and started passing vacuously."* That single assertion is
simultaneously a whole-dollar check **and** an every-modelled-line-is-written check for TY2024. It was
not carried across to the revision that is TY2026's actual emitter. `CLAUDE.md` §B3: *"the fix already
existed in the branch … nobody carried it back, because no reviewer ever held both commits at once."*

**Concrete failure scenario.** During the TY2026 port someone edits the `p3` array — a reorder, a
rebase conflict, a line dropped while adding a widget. `money_cells()` still returns 42 (exhaustive
destructure, so the struct change would have been a build error, but the *fill list* is a plain array
literal with no totality check). `every_mapped_field_exists_and_the_three_sets_partition_the_acroform`
still passes: the map ↔ AcroForm partition is unchanged. `verify_flat` still passes: it checks descent
across the placements it was **given**, and an omitted placement does not break the descent of the
rest. Result: a numbered box blank on a filed Form 6251 — the exact harm the emitter's own doc comment
invokes (*"filling this one would leave a numbered box blank on a filed form"*) — with nothing red.
Symmetrically, a Part III cell pointing at the wrong *existing* widget (lines 19/20 transposed, the
mirror of the build's own kill 7.4, which is demonstrated only on the Part I/II pair 9/10) is caught on
TY2024 by the routed fill and on this revision by nothing.

### M-1 — Minor. `cover_form6251line1` emits no rows for the OBBBA variant, and the trigger its source comment names has now fired.

`crates/btctax-core/src/tax/line_coverage.rs:682-698`. The body is
`if let Form6251Line1::Y2024 { line1 } = p { … }`, so a TY2025/TY2026 chain yields an **empty**
`Coverage`. The comment above it reads: *"TY2025's 1a/1b are a DIFFERENT form revision with 60 boxes;
**they get their own rows when that year's map lands**."* That year's map landed in this commit, and no
rows were added; nothing reds, because an `if let` that matches nothing is silent. Consequence:
`xtask line-coverage` prints `f6251:41` and `Form6251Line1::Y2025`'s doc comment — the one that
hardcodes *"line 37"* and is the artifact I-1 turns on — is verified verbatim by **nothing**.
Independently confirmed: `cite-check`'s `AUTHORITY_NOT_YET_ARCHIVED` carries
`("f6251", &[2024, 2025])` (`crates/xtask/src/cite_check.rs:958`), so cite-check reads no f6251
extract at all, and the whole `cover_form6251*` table is `Coverage::quoting("2024")`.

The builder disclosed this in its report §8 as out of scope, correctly. Recorded here because the
condition the *source* named as its own trigger is now satisfied, which converts a deferral into a
stale conditional. It is Minor rather than Important because TY2025 is unfilable and the forms-side
verbatim check does cover 1a/1b through the map TOML's comments.

### M-2 — Minor. The reason given for not transcribing line 4's figure is measurably false for the one revision this module holds.

`crates/btctax-forms/src/f6251_revision.rs:41-46`: *"those figures are already per-year in
`btctax_core::tax::tables::AmtParams` and the year's `TaxTable` (line 4's is
`AmtParams::mfs_kicker_start` — `dec!(875950)` for TY2024, `dec!(640200)` for TY2026). Re-typing one
here would create a **second authority**."* Measured:
`grep -rn "900350" crates --include="*.rs"` → **one** hit, `f6251_revision.rs:232`, the module's own
test assertion. No `AmtParams` anywhere carries TY2025's printed **$900,350**. For `f6251/2025` — the
only revision this module holds — the printed threshold has no first authority, so the
second-authority argument does not apply to it. Harmless today; recorded because the sentence is the
stated justification for a figure not being transcribed, and it is wrong about the revision it
justifies.

### M-3 — Minor. Two copies of TY2025's line-4 sentence; the checked one and the unchecked one are in different files.

`crates/btctax-forms/src/map.rs:628-631` (`Form6251ObbbaMap::line4`'s doc comment) quotes the TY2025
sentence including `$900,350`, inside a struct the design says serves TY2026 as well. The checked copy
is `f6251_revision.rs:155`, asserted verbatim per revision by
`the_year_varying_cells_are_verbatim_in_that_revisions_own_extract`.
`every_quoted_instruction_is_verbatim_on_its_own_revisions_form` does **not** reach the `map.rs` copy:
it parses the **map TOML's** `# <label> "…"` comment lines (`tests/f6251_obbba.rs:563-612`), not Rust
doc comments, and cite-check excuses the pair (M-1). Mitigated — and only partly — by the `★★★` note
immediately beneath it stating the figure is per revision and per year.

### N-1 — Nit. The three cell accessors take the first occurrence of their anchor, and only the zero-occurrence case fails closed.

`f6251_revision.rs:114-144`. `schedule_1a_line`, `mfs_threshold_printed` and
`form_1040_capital_gain_line` each `split_once` on an anchor phrase and read forward. Correct on both
archived revisions (verified: `Some(37)`, `Some(900350)`, `Some("7")` against
`design/forms/extract/f6251--2025.txt:18-19,44-45,65`). A future revision whose clause repeats its
anchor before the delimiter would silently take the first. The `None` path is tested
(`a_sentence_without_the_cross_reference_yields_none_rather_than_a_default`); the ambiguous path is not
expressible.

---

## Refuted premises

**R1 — my own brief's §3 mis-measures the diff it scopes.** It says *"11 files, +919/−110"*. Measured:
`git show d8d023af --numstat` → **12 files, +2588 / −110**; the `crates/*/src/` subset is
**5 files, +972 / −19**. Nothing downstream depended on the number (the commit body states none), so
this is a coordinator arithmetic slip rather than a design error — but it is the seventh refuted brief
premise in this arc and the §0 instruction says to report it with the measurement.

**R2 — the repo's own characterisation of the line-1a hazard as a "renumber" understates it, in two
places.** `f6251_revision.rs:23` (*"line 43 is its TY2026 renumber"*) and `return_1040.rs:2970`
(*"only the cited Schedule 1-A line moves, 37 → 43"*) are both true about 43 and both silent about the
fact that **37 survives on the TY2026 schedule meaning something else**: *"Enter the amount from line
3"*, i.e. modified AGI (`f1040s1a--2026-DRAFT.txt:213`, with rows 59 and 53 resolving line 3 and line
1). This is the difference between "off by the gap between two deduction subtotals" and "a six-figure
income magnitude in a four-figure deduction's slot", and it is why I-1 is a finding rather than a
tidiness note. My brief's Seam 1 got the framing right (*"a line that means something else on that
year's schedule"*); the source does not.

**Confirmed, not refuted** — every §2 settled row I traversed held:
the Unwired pin assertion really is at `line_set.rs:391` with the value `["f1040s1a/2025"]`; core's
`Schedule1A` really ends at `line38` (`crates/btctax-core/src/tax/schedule_1a.rs:392`); TY2025's Form
6251 line 7 really cites a bare "line 7" that the TY2025 Form 1040 does not print; and
`packet.rs`'s refusal is real — `packet.rs:238` still calls `Form6251Map::for_year(year)`, whose
schema match sends `f6251/2025` to the `other` arm (`map.rs:396`) returning
`Structure("…parses into Form6251ObbbaMap, not Form6251Map")`, pinned by
`map_pdf_conformance.rs:190-205`.

---

## Seams traversed

**Seam 1 — the computation-side twin.** Followed the chain the brief named, in the direction the money
moves, and then one hop further than the brief did. `f6251_revision::revision` → the emitter's revision
gate (`forms/src/form6251.rs:290-306`) → where `line1a` actually comes from
(`f.line1`, i.e. core's `Form6251Line1::Y2025`) → its constructor
(`core/src/tax/form6251.rs:466-476`) → its input (`Form6251Line1Rule::Y2025 { schedule_1a_l37 }`,
`form6251.rs:393-400`) → who fills that (`return_1040.rs:2974-2993`) → what gates *that*
(`_ => None` → panic; the derived `1990..=2060` sweep at `return_1040.rs:12032`;
`full_return_for(2026).is_none()` at `tax_tables.rs:1233`). Established the gate really blocks it, so
the severity is Important not Critical. Then went to the **primary source** rather than the design
doc for the 37/43 claim, because that is the one thing a reviewer can do that the forms table cannot:
read `f1040s1a--2026-DRAFT.txt`'s label column for 35-44 and `f1040s1a--2025.txt`'s for 31-38. That is
where the renumber turned out to be a collision, and where the direction of the error flipped from
understatement to overstatement. Also answered the brief's second question by comparing the two crates'
*representations* of the same cell — a parsed sentence on one side, an identifier on the other — which
is what makes the forms-side match unable to reach it.

**Seam 2 — the three cells versus the five constant lines.** Checked each of the five in the code, not
the table. `compute_6251` (`core/src/tax/form6251.rs:451-600`) reads line 5 from
`amt.exemption(st)`/`amt.phaseout_start(st)`/`amt.exemption_phaseout_rate`; lines 7/18/39 from
`amt.breakpoint_28pct(st)` and the two `rate_28_subtrahend*`; line 19 from `bp.max_zero`; line 25 from
`bp.max_fifteen`. All five are parameter-driven, none is a literal. Then read every doc comment on the
five `MoneyCell`s in the new struct (`map.rs:638-732`): none quotes a figure — each names `AmtParams`
or the year's `TaxTable` as the authority instead. The **only** dollar literal in the new struct is
line 4's, inside a verbatim quote, with the per-revision caveat stated beneath it — which is how M-2
and M-3 surfaced, from the justification rather than from the figure. Verified the three cells are the
right three by confirming that the sixth candidate — line 7's `7 → 7a` — has **no** money
consequence: core routes Part III on `preferential = net_capital_gain + qualified_dividends > 0`
(`form6251.rs:589-591`), never on a 1040 line number, so that cell is a documentation cross-reference
and 1a is the only money one.

**Seam 3 — does anything read this.** Grepped every mention of `Form6251ObbbaMap` and
`fill_form_6251_obbba_with_map` outside their own module and test file: the exports are
`lib.rs:552` and `lib.rs:569`, both inside `pub mod testonly`, and the only caller is
`tests/f6251_obbba.rs`. Confirmed the refusal by reading the path rather than the claim — `packet.rs:238`
→ `Form6251Map::for_year` → `line_set::schema` → the `other` arm → `Structure`, with the message
naming the struct, and the boundary stated in three source locations (the struct's doc, the filler's
doc, the test module's doc). The refusal refuses. This is what made I-2 visible: I went looking for
what *does* exercise the 42 cells and found that 29 of them are exercised by nothing, which is a
sharper version of "a figure with no reader" than the build's own honest disclosure.

**Seam 4 — the instruments.** Read `parses_into_its_schema` (`map.rs:4500-4546`) in full: `_`-free over
`Schema`, `Unwired => return None`, one arm per struct, so a new struct is `E0004`. Read the rewritten
`line_set_wiring.rs` end to end and checked its liveness floors (`wired_and_parses >= 37`, and
`wired_and_parses + no_struct.len() == LineSet::ALL.len()`, so a dispatch returning `None` for
everything cannot satisfy it). Read the new `the_per_stem_dispatch_agrees_with_the_derived_dispatch`
and confirmed it is a genuine cross-check of a hand table against a derived one in **both** directions
(`Some(Ok)` ⇒ must dispatch; `None` ⇒ must not), with its own `checked >= 37` floor. Traced the
conformance partition by reading `printed_labels`, `mapped`, `censused_labels` and the two-direction
comparison, and confirmed the `printed.len() >= 59` guard against a blind label reader. On the brief's
question about the 18 censused labels: they are lines **2c–2t** exactly (18 letters, counted off
`f6251--2025.txt:24-42`), the same set as TY2024's, carried on the map file that predates this commit,
recorded as `rule = "unmodeled"` with the `[direction]`/`subtracts` machinery and `xtask census-join`
behind them. They are real lines encoding real decisions — 2i is the ISO exercise `CLAUDE.md` names as
the standing example — but they are declared with a mechanism, not laundered, and none of it is
authored by this diff. Not a finding here; it belongs to the `FOLLOWUPS.md` §G-5 audit.

**Seam 5 — transcription faithfulness.** Compared the new struct's Part I doc comments against
`design/forms/extract/f6251--2025.txt` rows 17-45 by eye, line by line: 1b, 2a (*"line 12e"*, correct
for this revision and measured identical in the 2026 draft), 2b, 3 and 4 are verbatim including the
wrap. Confirmed the 33/36 pair reads *"from line 22"* and *"from line 12"* respectively, against
extract rows 136 and 140 — the original $200,000 defect, correct here and additionally pinned by a
test. Noted that `line1a`'s doc comment carries **no** instruction text at all and instead points at
`f6251_revision`; that is defensible under the transcription rule's own per-revision scoping (the
sentence differs per revision, so a shared struct cannot hold it) and it is stated rather than
implied, so it is not a finding. Then asked which instrument holds these quotes and found the answer is
**none** for the Rust doc comments — `every_quoted_instruction_is_verbatim_on_its_own_revisions_form`
reads the map TOML's comments, cite-check excuses `("f6251", &[2024, 2025])`, and line-coverage quotes
against the 2024 extract with the OBBBA variant skipped. That is M-1 and M-3.

---

## Out of scope / already filed

* The owner's TY2025 ruling; bundling any params; the TY2025/TY2026 fail-closed gates; the Schedule 1-A
  emitter; `f1040s1a/2025` staying `Unwired`; the B3 work; the rest of the branch. None touched.
* **The suite was not run**, per the brief: the controller's `make gate` on the main tree
  (3589 passed / 12 skipped, exit 0, fmt clean) stands, and no finding above rests on a red. Every
  claim here is either a read of committed source at a cited line or a `grep`/extract measurement
  quoted inline.
* **Already filed by the builder, and confirmed accurate as far as it goes:** F3 (core's
  `Form6251Line1::Y2025` doc hardcodes "line 37"). I-1 is that follow-up plus the three things it does
  not say — that the cross-reference lives in a *field name* and so is unreachable by any check; that
  the emitter resolves the right number and discards it; and that 37 and 43 are a collision of unlike
  quantities rather than a renumber. F4 (nothing joins `mfs_threshold_printed()` to
  `AmtParams::mfs_kicker_start`) is correct about the crate boundary — but note that the **1a** join
  has no such excuse: `btctax-forms` depends on `btctax_core` and the emitter already holds both sides.
* F5 (the `gap`/`unmodeled` prose contradiction) reproduced and pre-existing; untouched by this diff.
* F6 (`ty2026_full_return_must_stay_fail_closed`'s first blocker partially retired) confirmed: the gate
  at `tax_tables.rs:1233` is untouched and still closed.
