# ADVICE — the year-over-year shape: what makes the SECOND and THIRD ports expensive

**Agent:** Fable, one consult, owner-authorised. **Date:** 2026-09-13. **Nothing edited, no subagents, no
line-by-line code review.** Every number below is either quoted from the five evidence documents the brief
named or measured with one grep this session; each is labelled which.

**Read:** `REPORT-rehearse-port-f8995a-2025.md` (whole), `TY2026_PORT_REPORT.md` §2.3–2.7, §3, §4,
`FORM_AUTHORITY_TABLE_DESIGN.md` §10–11, `LONG_RANGE_PLAN_filing.md` §3 banner, §5, §7, `FOLLOWUPS.md`
FR-134…FR-158, `ROADMAP_STATUS.md` §0a/§3/§4, `TY2026_WORK_LIST.md`, `HARNESS.md` scope bound.

---

## 0. The answer in six lines

1. **The forms crate is finished for the purpose of the second port.** 85 → 0 → 7 → 1 per `(stem, year)`,
   every step-23 gate walks the glob, the caption gate is year-generic, the kills plant by removal. Stop
   machining it; what is left there is the human steps (1, 5, 16, 17, 19, 20, 21, 24) and two typed counts.
2. **The per-port cost has moved OUT of the forms crate and nobody owns it there.** The port report's §2.3
   and §2.6 name a dozen year-literal sites in `btctax-core` worksheets/prompts and in `scripts/oracle/`;
   **none of them has a `FOLLOWUPS.md` entry** (grep: `prompt_check` 0, `DEFAULT_ROW_YEAR` 0, `OTS_YEAR` 0;
   the one `capital_loss_carryover` hit is unrelated). The rehearsal ported a form, so it could not see them.
3. **Before January, change one thing:** make the year a required argument through the oracle harness and
   the golden corpus. It is a deletion of defaults, not a refactor and not a new instrument; it cannot be
   learned from the port because it is the *evidence* side, not the *form* side; and January's census is the
   first moment a second year enters a harness whose every default says 2024.
4. **Before January, also do FR-156** — weighed, and it is light: two counts that red loudly, twenty minutes,
   the exact edit A2/A3/S1 made three times this week. Rank 4, not rank 2.
5. **After the first port, and only then:** a revision axis for the compute-side transcription structs, and a
   carry rule for the 323 `LineCoverage` sentences. Both are sized by numbers only the January port produces.
6. **"Change nothing" is the right answer for the forms crate, the params tables and the compute structs.**
   It is wrong for exactly one layer, and that layer is the one the sign-off rests on.

---

## 1. Where the per-port cost lives now — by layer, not by step

| layer | per-port cost today | machine-held? | who owns it |
|---|---|---|---|
| **forms crate** — templates, maps, bindings, `line_set`/`Schema`, gates | 1 edit per `(stem, year)` in `schema()` (FR-141) + 2 typed counts (FR-156) + the H steps | yes: `build.rs`, glob-walking gates, year-generic caption gate (FR-134), plant-by-removal kills (FR-136) | the port machine; done |
| **params** — `FullReturnParams` (23 fields + `AmtParams`/`Schedule1aParams`/`HsaParams`), `TaxTable` | one hand-transcribed Rust fn per year (`tax_tables.rs:116` TY2024, `:246` TY2026) + one `by_year.insert` | yes: **E0063** on a missing field; `YearReadiness` reds a declared-filable year with no params | NOW bucket; TY2026 written (FR-47) |
| **compute-side transcription structs** — `printed.rs` 129 line-numbered fields / 10 structs, zero `year` (report §2.3 #21); `Schedule1A` with TY2025 line numbers in field names + 52 label literals (#16); `qbi.rs::Form8995Lines` (#19); `ScheduleALines.line17` (#20) | **unknown until January.** TY2026 rebuilds four forms (`f1040s1a`, `f1040sa`, `f1040sc`, `f8995`) and moves 23 lines on `f1040s2` (work list) | no revision axis at all — the maps know their `line_set`; these structs do not | nobody; design §11: *"It does not touch compute"* |
| **compute-side year-relative TEXT** — `line_coverage.rs:70 DEFAULT_ROW_YEAR = "2024"` (273 of 323 rows fall through, #23; FR-135 ratchets 12 mis-pointed pairs); `prompt_check.rs` 18 extract literals pinned `--2025` (#24); `capital_loss_carryover.rs` *"your 2024 Form 1040"* ×4 + `SOURCE_EXTRACT` pinned `i1040sd--2025` (#22) | re-point ~323 rows + 18 literals + 4 prompt strings, by hand, every year | partly: FR-135's ratchet reds when a bundled year has no rows quoting it — **so the cheap January discharge is widening the ratchet** | FR-135 remainder → "the port machine"; the other two sites unfiled |
| **oracle harness + goldens** — `gen_goldens.py` 4× `year: int = 2024` (:262/:308/:337/:423) + `"tax_year": 2024` (:618); `ots_direct.py:79` `OTS_YEAR` default 2024, `version()` blind to it (R21); `verify_f6251.py:54` `DEFAULT_FIXTURE_YEAR = 2024`, *every committed vector relies on it*; `sweep.py` module-level 2024 constants; `golden_returns.rs` imports `ty2024_params`/`ty2024_table` (:39, :87, :522, :596), `_provenance.tax_year` unread; one goldens file (227,699 B), one year; `ty2024_params`/`ty2024_table` **354** refs vs **9** for 2026 (grep) | edit six defaults per year — and TY2024's validation then either silently stops or silently runs against the wrong year | **no.** The single-year assurance surface the file `shipped_tables_are_the_validated_tables.rs` was written to close is still one year (`validated_table_for` matches `2024 =>` only, :224/:234) | nobody |

The compute layer's *arithmetic* is already keyed by data: **11** non-test `20xx =>` match arms across 7
files in `crates/` (grep), and the report's `dec!(≥1000)` scan found no indexed value outside a per-year
table. The disease is not in the arithmetic. It is in the **text** the arithmetic is verified against, and
in the **harness** the arithmetic is validated by. Those are the two layers the year-package design
declared out of scope, correctly for the first port and expensively for the second.

---

## 2. The ranked shortlist

### 1. Make the year a REQUIRED argument through the oracle harness and the golden corpus — BEFORE January

**What.** Delete the six `= 2024` defaults (`gen_goldens.py` ×4 + the `"tax_year"` literal, `ots_direct.py`,
`verify_f6251.py`, `sweep.py`'s module-level constants — the model is two files away, `corpus.py::salt_for(year)`,
which refuses an unknown year). Name the goldens file by year and have `golden_returns.rs` read the
`_provenance.tax_year` it already carries and refuse when it disagrees with the params it is fed. Make
`ots_direct.py::version()` report the tree it actually ran (R21 — SPEC §11 gates regeneration on that string).

**This is not a new instrument.** It is removing defaults from instruments that exist, plus one existing test
reading one field it already stores. B1 applies and is a one-liner: feed the 2024 corpus to 2026 params → red.

**Cost of doing:** roughly one agent-day, in the Sep–Dec NOW bucket, off the critical path. It touches no form,
no map, no worksheet.

**Cost of not:** the TY2026 census (~2027-01-27, on the critical path) is the first time a second year enters
this harness. Every default is a place the TY2024 answer gets reused by *omitting an argument* — the
port report's own phrase — and under April pressure the fix that gets made is `= 2026`, which is the
typed-list move applied to the sign-off evidence: TY2024's validation stops running and nothing reds.
§G-9's limit is the sharp form: an oracle fed the wrong year agrees with nothing, and two of them agreeing
is then diagnostic of nothing. Ports 2 and 3 repeat the six edits, and each repetition is a chance to
disagree about which year is "current".

**Why it cannot wait.** Nothing about it is learned from the port: the form side teaches you which lines
moved; it teaches you nothing about whether your oracle ran on the year you think it did. And the census
sits *after* the port on the calendar, so "after the first port teaches us" means "after the moment it
mattered".

**Falsifier — the case that would make this wrong:** if the TY2026 census will be driven by a *committed
runner* that passes the year explicitly on every path, and `verify_f6251.py`'s 30 vectors all carry a `year`
key, then the defaults are dead code and this is a doc fix. Evidence against: the report says every
committed vector relies on `DEFAULT_FIXTURE_YEAR`, and `verify_schedule_1a.py` already spans TY2025–2028
in one script — the pattern here is one script, many years, which is exactly when a default bites.
Second falsifier: if OTS-2026 needs a different driver entirely (a new tree `ots_direct.py` cannot wrap), oracle
1's defaults are moot — but taxcalc's four are not.

### 2. A carry rule for compute-side year-relative TEXT — AFTER the first port, with one January rule now

**What.** `LineCoverage` rows (323), `prompt_check` clauses (18), the carryover worksheet's prose (4) all
quote a sentence from a specific year's booklet. The `forms port` design already has the right mechanism —
P3, *carry only if the sentence is byte-identical in the new extract, else delete the quote and mark
needs-transcription*. Applied to these three sites it turns ~345 H re-verifications into a diff plus a
short H residue.

**Why after.** The number that sizes it does not exist yet: **how many of the 323 sentences actually change
2025→2026.** If it is the thresholds only (the f8995a rehearsal saw 2 of 44), hand re-pointing is cheaper
than the tool for every future port too, and the tool is never worth building. If OBBBA's booklet rewrites
hundreds, build it in February from real data. Building it in October designs it against zero rebuilt years.

**The one thing to do NOW is a sentence in the January brief, not code:** *the FR-135 ratchet may not be
widened; a bundled year gets rows quoting its own booklet, or the form is `Unwired`.* The ratchet's whole
value is that the cheap discharge is loud; the brief has to make the loud path the only path. Same sentence
for `prompt_check`'s 18 literals: they are the strings shown immediately before an answer becomes sworn
testimony, and a TY2026 filer shown *"your 2024 Form 1040"* is FR-134's class on the input side. The
carryover prose is on the January path anyway (`i1040sd--2026` lands with the package); do it as a
two-year `format!` in that edit, not before.

**Cost of not:** ports 2 and 3 each re-verify ~345 sentences by hand, or the ratchet grows by a year each
time and the compute side's instruction text drifts one booklet behind — which is the Form 6251 line-33
class, on a slower clock.

### 3. A revision axis for compute-side transcription structs — DO NOT build before January; apply the rule, measure, decide after

**What the temptation is.** The forms crate got `line_set` + `Schema` + a many-to-one `schema()`; `printed.rs`'s
129 line-numbered fields, `Schedule1A`, `Form8995Lines` and `ScheduleALines` did not, and the port report
flags `printed.rs` as OPEN pending exactly that. TY2026 rebuilds four forms. The symmetric move — give
every compute struct a `line_set` — is the obvious refactor, and it is the one this repo's evidence says
not to make on theory.

**Why not before.** (a) The decision procedure already exists on paper (report §3: *arithmetic changed →
variant on the params bundle; only the printed number changed → label row; collection surface changed →
per-PART struct*), and it has been exercised **once**, on the map side — `f6251/2025`, where step 17 in
anger produced the 37→43 collision and took two review rounds to hold by a type. That is the ground truth
about what a rebuild costs, and it is one data point. (b) Of the four TY2026 rebuilds, S2 may turn two into
refusals (no Schedule C, no business ⇒ `f1040sc`, `f8995`), which changes the count the axis would serve.
(c) A refactor of `printed.rs` competes head-on with the January critical path and would itself need the
seam review → fold → re-verify loop, during the one window that cannot afford it.

**What to do instead, and it is free:** the January briefs for each rebuilt form state the rule and one
prohibition — *a rebuilt PART gets a new per-revision struct; the shared struct is never edited in place;
the selection lives on the year's params bundle (the `SaltLimitation` exemplar, `tables.rs:323`).* Then the
first port writes three or four such structs, and February can see whether they want a common axis or
whether `printed.rs`'s 1040 lines simply never moved — in which case 129 fields with no year was the right
design and the OPEN flag closes as "not a transcription struct after all".

**Cost of not doing it after:** port 2 inherits whatever shape January improvised under pressure; if that
shape is four ad-hoc structs with four selection sites, port 2 pays a refactor and a port in the same
window. Cost of doing it before: a structural move on a layer with zero rebuilt-year evidence, in the
window the plan says must go to the owner's real data.

### 4. FR-156 and the count residue — BEFORE January, as hygiene, sized honestly

`tests/supported_years_cross_product.rs::BUNDLED_FORMS_PER_YEAR` and `tests/map_pdf_conformance.rs`'s
8995-A refusal loop. **Weighed: light.** Both red *loudly* on the port (a wrong count cannot produce a wrong
return), both are twenty-minute derive-from-`YEAR.toml` edits of the shape A3 made for the `38` and S1 for
the fill years, and the second is a test asserting the *absence* of a port — the FR-136 plant shape, one
level up. Do them now because January has enough genuine H work without two speed bumps that look like
H and are not. Do **not** let this become a sweep for "every hardcoded count in the tree" — that is a fresh
audit, and the port machine's own list (FR-152's 30 absolute `extract_line` anchors, FR-155) is the same
class and correctly parked.

### 5. Old years accumulate as a row in every gate forever — a DECISION after the first port, not a mechanism

S9's reasoning applied forward: by port 3 the bundle is 2024/2025/2026/2027/2028, every glob-walking gate
walks all of them, every tightening of an instrument must pass on every year's maps, and FR-153 shows old
maps carry pinned transcription defects that each tightening must step around. TY2025 (18 maps) is needed
through the TY2026 port as the *prior side* of every `form-delta`; the day that port closes it is the
prior side of nothing, since TY2027's prior is TY2026. TY2024 stays exactly as long as the interview
simulation and the filed-return reference use it.

**Do not build a "frozen year" mechanism for this.** `HARNESS.md`'s scope bound requires a mechanism be
earned by an observed failure; the observed cure is S9 — a deletion, taken in one ruling, 3168 tests
green. Put a line in the port runbook: *on closing a port, ask which prior years are still read by
anything, and rule.* The answer for TY2025 is probably "drop, keep the `TaxTable`", the S9 shape exactly.

---

## 3. The before / after January split

| item | before January | after the first port | why the split falls there |
|---|---|---|---|
| **1** oracle harness year required | **yes** — deletions + one field read | — | it is the evidence layer; the port teaches nothing about it; the census is after the port on the calendar |
| **2** text carry rule | one sentence in the brief (no ratchet widening) + the carryover prose as a January edit | build P3-for-`LineCoverage` only if the measured change count says so | the sizing number does not exist until a booklet has been diffed |
| **3** compute revision axis | one rule in the rebuild briefs; no code | decide from the 3–4 structs January wrote | one data point today (`f6251/2025`); S2 may halve the population; competes with the critical path |
| **4** FR-156 | **yes** — 20 minutes | — | cheap, certain, loud today; removes false H from January |
| **5** old-year residue | — | rule on TY2025 the week the TY2026 port closes | TY2025 is load-bearing (prior side) until then |

Items 1 and 4 together are under two agent-days and touch no form, no map, no worksheet, no params, no
review loop on prose. Everything else is a sentence in a brief. That is the whole "before" budget.

---

## 4. The "change nothing" option, argued honestly

**The case for it.** The first port is the only source of ground truth, the rehearsal was on the cleanest
possible pair and still produced 18 findings, and the four rebuilt forms plus f1040s2's 23 moved lines
are the first real exercise of steps 16/17 at scale. Every structural move in this repo's history that was
made ahead of evidence bought a defect: the 37→43 "renumber" comments, the 31/28 moved-line counts that
were a cover-sheet artifact, the one-shape caption parser that would have read 134 of 429. Ten agents
just burned 18 findings down in a day; the forms crate is at 1 edit per `(stem, year)`. The strong prior
is: port TY2026 with what exists, write everything down, and design ports 2 and 3 from the diff.

**Where it is right.** For the forms crate — completely. For the params tables — completely; `dec!` in a
Rust fn with E0063 totality is the correct shape and a TOML would lose the compiler. For the compute
structs — completely, per item 3. For the port machine's remaining generators (`forms port`, Layer-4
codegen) — completely, and S4's 2026-10-31 hard stop already says so.

**Where it is wrong, and it is one place.** The oracle harness and golden corpus. "Change nothing" there
means the TY2026 census runs on scripts whose year is an omitted argument, and the two-oracle rule —
the thing that makes "validated" mean anything — is applied through a surface that cannot tell which
year it validated. That is not a form-side fact the port will teach; it is the measuring instrument, and
the port is the measurement. You cannot learn from the reading that the meter was on the wrong range.
Fixing it is a deletion. So: **change nothing, except delete the defaults.**

If the owner rules "genuinely nothing", the residual risk is bounded by S7's other evidence for signing
(taxcalc 6.8.2 + the S1 diff + the hand-worked 6251/1-A) — but S7 names those as what suffices *when OTS
is absent*, not as a substitute for knowing which year OTS ran.

---

## 5. What NOT to touch

- **`FullReturnParams` / `TaxTable` as per-year Rust fns** (`tax_tables.rs`). E0063 is the totality check;
  `YearReadiness` is the insertion check. Do not move them to data files; do not generate them.
- **The forms crate's Layers 0–4 and every glob-walking gate.** Done; reviewed; kills planted by removal.
  No Layer-4 codegen and no `forms port` generator before one real port (S4).
- **`frozen_guard`'s three files** (`types.rs`, `compute.rs`, `se.rs`). Not year-related; named so nobody
  "tidies" them while threading a year through the harness.
- **`Schedule1aParams`'s 2025..=2028 identity guard** (`tables.rs:1147`). Its provisions are unindexed by
  statute; for ports 2 and 3 the guard is *right*. The one moving datum (the Part V birth cutoff) is
  computed by `born_early_enough(dob, year)`, not stored, so the guard's blind spot is not a port cost.
- **Do not add** `ty2026()` convenience constructors to the map structs, or a `ty2026_params()` to
  `testonly.rs`. The 354-reference single-year assurance surface is what `ty2024_params` became; new years
  enter tests through `bundle.full_return_for(y)` and `covered_years()` (FR-142's shape) or not at all.
- **Do not build a frozen-year mechanism** (item 5) or a compute-side `line_set` (item 3) on principle.
- **Do not promote any of this** to `~/.claude/CLAUDE.md` or `/scratch/code/CLAUDE.md`. Scope bound.
- **§7 stands:** no e-file, no state, no second §24/§32, no P2-profile items, no smoothing the 1-A cliffs.

---

## 6. What the first port must be asked to MEASURE, so the "after" items are decided by numbers

These go into the January briefs as required outputs, each one line:

1. Of the 323 `LineCoverage` sentences and 18 `prompt_check` clauses, **how many changed text** 2025→2026,
   by form. (Decides item 2: tool or hand.)
2. For each rebuilt form, **which bucket** of the §3 decision procedure it landed in — variant / label row /
   per-part struct — and how many selection sites the per-part structs needed. (Decides item 3.)
3. Whether any TY2026 change touched `printed.rs`'s 1040 lines at all. If not, the 129-field OPEN flag
   closes as "quantity struct, correctly year-free".
4. **Steps 16 and 17, costed:** per form, how many witness disagreements and how many "same line?"
   rulings, and how many review-round findings they produced. `f6251/2025` is the prior: one collision,
   two rounds. This is the number that tells you whether the H ceiling is 8 of 24 steps or 8 of 24
   *hours*.
5. Which TY2025 artifacts were read by anything after the port closed. (Decides item 5.)

---

## 7. The falsifier for the top recommendation, stated once more

Item 1 is wrong if the January census is run by a committed driver that names the year on every path and
every vector already carries one — then the defaults are unreachable and the item is a comment. The
evidence in hand says the opposite (`verify_f6251.py`'s own doc; one goldens file with `tax_year` unread;
354 references to a 2024 fixture and 9 to 2026). If someone can produce that driver, downgrade item 1
to "delete the dead defaults anyway" and the before-January budget is FR-156 alone.
