# REPORT — stage 2, Tier A. The TY2026 throwaway rehearsal, library half.

**Agent:** Tier A (stage-2 brief `design/agent-reports/BRIEF-stage2-run.md`, commit `9d1e98cfc`).
**Deliverable:** the GAP against `design/agent-reports/TY2026-BLOCKERS-PREDICTION-2026-09-13.txt`.
**Nothing was committed and nothing was pushed.** Two new test files, no source file modified.

---

## 0. THIS RUN VALIDATES NOTHING

OpenTaxSolver 2026 does not exist until ~2027-01-27; taxcalc has no validated TY2026 parameters
(`forms/2026/YEAR.toml [oracles]` says so in both rows); no `f1040--2026` and no `i1040gi--2026` is
archived. **Not one figure this run produced may be called correct**, including every figure quoted
below. Every TY2026 fact asserted comes from a document marked *"DRAFT — evidence only, never
transcribed as authority"*. This is a wall-hunt. S1, with a signed return as its answer key, is the
thing that validates figures; this is its complement.

---

## 1. THE GAP, in the brief's three rows

### 1a. ★★★ HIT, NOT PREDICTED — the entire point

| # | finding | severity |
|---|---|---|
| **A-1** | **`assemble_absolute` at TY2026 ABORTS THE PROCESS with a `panic!`, on the most ordinary household there is** — and the CLI's **default year is 2026**, with the params bundle as the *only* guard in front of it. | **Critical** |
| **A-2** | **`xtask blockers` cannot see A-1, structurally**: its year-keyed gate set is a hand-written `vec![…]` of **7** gates in `blockers.rs::gates()`, and its refusal census walks `RefuseReason` variants. A year-keyed `panic!` is in neither set, and `blockers.rs` contains no scan for one. | **Important** |
| **A-3** | **The design doc's §2 gate is insufficient as written.** Its condition is the TY2026 1040 + instructions archived *and transcribed*. That says nothing about Form 6251 Part I or Schedule 1-A, so the gate as specified would **permit** the bundling that produces A-1. | **Important** |
| **A-4** | `Advisory::UnmodeledDeductionsOmitted` is a hardcoded prose string with no year, and it is the surface that tells a filer which deductions btctax did not compute. It names Schedule A *"line 15, Form 4684"* and *"line 16"* (TY2026: **16** and **17a–17z**) and *"moving expenses for the Armed Forces"* (TY2026: *"and the intelligence community"*). `UnmodeledDeductionsOmitted` appears **0 times** in `FOLLOWUPS.md`. | **Minor** |
| **A-5** | `bundled_years()` **contains 2026** (the glob of `forms/<year>/`, and `forms/2026/` holds a `YEAR.toml`), so the map layer is *not* uniformly blocked for TY2026: **2 of 21** `Stem::ALL` forms — `f8275` and `f8283` — resolve a template through `periodic_template`, because that function's guard is `BUNDLED_YEARS.contains(&year)`. The thing preventing an incoherent two-page TY2026 packet is the CLI export gate, not the map layer. | **Minor** |
| **A-6** | `repo_hygiene::every_intra_workspace_dependency_pins_the_current_version` is a **line scan with no section awareness**, so it applies *"a path dep with NO version cannot be published at all"* to `[dev-dependencies]` — where it is false (path-only dev-deps are stripped from the packaged manifest) and where the version it demands would **break** the dependency-first publish order in a dev-dep cycle. Found by tripping it. | **Minor** |
| **A-7** | Methodological trap for any future derived checker: a line-id set derived from `pdftotext -layout` **over-reports** the 2025→2026 Schedule A delta. The 2025 layout puts sub-line letters in their own column (`a State and local income taxes`), so `5a`–`5e` and `8a`–`8e` read as *new in 2026* although both revisions print them. Measured: a naive set difference returned 23 "new" ids of which 10 are layout artifacts. | **Nit (recorded)** |

**A-1, in full.** `return_1040.rs::form6251_line1_rule` is `match year { 2024 => …, 2025 => …, _ => None }`,
and `form6251_inputs_from_parts` turns that `None` into a `panic!`:

> `Form 6251 Part I has never been transcribed for TY2026, so there is no line-1 rule to apply. REFUSING rather than filing TY2024's Part I under a TY2026 heading — line 1 (2024) and line 1b (2025) are different quantities, and the 2024 rule understates AMTI by Schedule 1-A line 37. Fix: read Form 6251 for TY2026 from its text layer and add the arm to `form6251_line1_rule` (see design/TY2026_PORT_REPORT.md §7 D3).`

Form 6251 is computed on **every** return since v0.14.0, so every TY2026 return reaches it. The fixture
that hit it is a married couple with W-2 wages, two Bitcoin sales and an itemized Schedule A.

★★ **The gate's reasoning is right and its direction is right** — TY2025 line 1a subtracts *Schedule 1-A
line 37*, the TY2026 draft subtracts *line 43*, and 37 is not vacated (on the 2026 schedule it is
modified AGI), so substituting one for the other overstates the AMT base by roughly MAGI. Failing
closed is correct. **What is wrong is the channel and the reachability:**

1. a `panic!` is not a `RefuseReason` — it carries no *exit*, so a filer meets a backtrace instead of a
   sentence and a next step (the FR-102 family);
2. the *only* thing between this and a default-year invocation is `full_return_for(year).is_some()`.
   `cmd/tax.rs:558` and `cmd/admin.rs:1861` both destructure it and return `CliError::Usage` on `None`.
   There is no second guard;
3. `year_readiness::default_year()` is `max(bundled_years())` = **2026**, and `year_readiness.rs`'s own
   `the_default_year_is_the_newest_bundled_one` asserts exactly that. So a no-`--year` invocation is a
   TY2026 invocation.

★ **What is already known, stated so the gap is not overclaimed.** `design/TY2026_PORT_REPORT.md` §7 D3
already rules that the `Y2025` line-1 *shape* is reusable for TY2026 and treats the work as a rename;
the prediction file carries `f6251` as a form-map row. **Neither says the intermediate state is a
process abort on the default year**, and `form6251_line1_rule` appears **0 times** in both the
prediction file and `FOLLOWUPS.md`. The composition — *one insert converts a clean refusal into an
abort, on the year a bare command selects* — is the new fact, and Tier B is about to perform that
insert.

### 1b. PREDICTED AND HIT — no news

* `FullReturnParams are bundled (THE compute gate)` shut for 2026 — confirmed, and it is the single
  guard named above.
* `f1040sa`, `f6251`, `f1040s1`, `f8949`, `schedule_d` and the rest not bundled for TY2026 — confirmed
  derived over `Stem::ALL`: **19 of 21** forms resolve no template at all for 2026.
* `no TY2026 revision of i1040sca / i1040gi is archived` — confirmed, and this is what blocks the two
  new Schedule A worksheets (below).
* `TY2026 declares status = "preparing"` — confirmed.

### 1c. PREDICTED, NOT HIT — the command over-reports for a Tier-A run, correctly

| predicted row | why Tier A did not hit it |
|---|---|
| the §111(a) state-and-local-refund worksheet gate | never fires: the fixture has no state tax refund. It is a **conditional** blocker, and the prediction states it unconditionally. |
| `Schedule1aNotOnThisYearsReturn` | never fires: the fixture has no tips, overtime, car-loan interest or senior deduction, so Schedule 1-A is legitimately empty. |
| the export-time price gate (dataset ends 2026-06-03) | not reached — Tier A never exports. **Tier B's row.** |
| `slice_can_print` (8949 + Schedule D maps) | same — Tier B's row. |
| the 12 `UNMEASURED` pinned-literal rows | none is reachable from a compute run; they are instrument-decay rows, not walls. |

★ That is not a criticism of the command. It is the correct reading of *"predicted, not hit"*: **four of
the seven year-keyed gates are conditional on facts a particular household supplies**, and the
prediction has no slot for a condition.

---

## 2. THE SUBSTITUTION LIST — derived by necessity, which makes it a blocker list

**The rule honoured throughout: substitute a document we HAVE for one we LACK; never invent a figure.**
Nothing below is a fabricated number.

| # | what was substituted | substituted FROM | why it is legitimate | the blocker it names |
|---|---|---|---|---|
| S-1 | TY2026 Form 6251 Part I | `Form6251Line1Rule::Y2025` | the 2026 **draft's** 1a/1b arithmetic is identical to 2025's — only the cited Schedule 1-A line moves, 37 → 43 — **and this household's senior-deduction subtotal is $0**, so the 37/43 collision cannot move a figure on it | **A-1.** A household *with* a senior deduction is exactly where the collision bites, which is why the production gate must stay shut. |
| S-2 | the TY2026 1040's line set | the TY2026 Schedule A and Form 6251 **drafts**, which cite it (line 11b, line 12e, lines 13a/13b, line 14, line 7a) | evidence about a document we lack, taken from documents we hold, with no figure invented | FR-181 (no `f1040--2026`). ★ Note the citations are **TY2025's** numbering too — `f1040sa--2025.txt` already cites *"line 11b"* and *"line 12e"*. The 1040 renumber is a **TY2025** event, not a TY2026 one. |
| S-3 | AGI, computed by hand in the harness | the fixture's own leaves (wages + interest + ordinary dividends + net gain) | used **only to position the fixture inside a phase-out band**, never as an expected value; the engine's own AGI is unreachable behind A-1 | A-1 again. |
| — | **NOT substituted, and this is the wall** | | | |
| S-4 | the **Charitable Contribution Limitation Worksheet** (Schedule A 2026 line 13) | *nothing* | it lives in `i1040sca--2026` / `i1040gi--2026`, neither archived; **no archived document anywhere contains it** (checked by reading the whole extract directory) | **the honest stop.** Line 13 cannot be computed. |
| S-5 | the **Itemized Deductions Worksheet** (Schedule A 2026 line 18, the `$384,350` gate) | *nothing* | same | **the honest stop.** The total cannot be limited. |

★★ **The substitution list is two items long and both are single-line reuses of the adjacent year.** The
design doc's §1 claim — *"we barely need to invent anything"* — **holds**, and is now confirmed rather
than assumed. What does not hold is §3's claim that Tier A *"reaches the compute and emit surfaces"*: it
reaches them only past S-1, and S-1 is a `panic!` a test cannot bypass without editing shipped source.

---

## 3. THE ANSWER TO THE BRIEF'S QUESTION: does the compute chain NOTICE?

**It splits cleanly, and the split is the finding.** Everything TY2026 changed that is a **number** is
noticed, because it is a `FullReturnParams` field and the chain reads params. Everything TY2026 changed
that is a **line set** is not noticed, and *cannot* be, because the printed chain has no year:
`printed::schedule_a_lines(ar, line11_1040)` takes neither a year nor the params, and
`packet::assemble_printed_forms(…)` is not passed the params at all.

| TY2026 change | noticed? | mechanism, measured |
|---|---|---|
| Schedule A 5e SALT cap $40,000 → **$40,400** (FR-219) | **YES** | `params.salt.line_5e` to `ScheduleAParts::salt_5e`. Measured: 5d $69,500 gives 5e **$40,400**, not $40,000 and not $10,000. |
| SALT phase-out $500,000 → **$505,000** at 30% | **YES** | measured at MAGI $553,200: 5e = **$25,940** = 40,400 − 0.30 × 48,200. The TY2025 pair would give $24,440 — a $1,500 gap at every MAGI in the band. Floor: at MAGI $1,253,200, 5e = **$10,000**. |
| Form 6251 line 5 phase-out start MFJ $1,252,700 → **$1,000,000**, rate 25% → **50%** (FR-212) | **YES** | `params.amt`. Measured at AMTI $1,123,200: line 5 = **$78,600** = 140,200 − 0.50 × 123,200. Each older half refuted separately, because both moves point the same way and a mixed pair would still look plausible. |
| Schedule A's charity block renumbering (13/14/15) | **NO** | the chain computes `line14 = 11 + 12 + 13`, TY2025's *"Add lines 11 through 13"*. On the 2026 revision **13** is the Charitable Contribution Limitation Worksheet's line 6 and **14** is the carryover. Measured on the ordinary vector: 11=$6,000, 12=$0, 13=$0, 14=$6,000 — so the **cash-gift total lands on the carryover line** and the worksheet line prints $0. |
| the itemized total moving **17 → 18** behind the `$384,350` gate | **NO** | `ScheduleAParts::total_17` is `medical + salt + 8a + 8b + 8c + investment interest + charitable`, and 1040 line 12 takes it. On the 2026 revision **17 is "Other itemized deductions"**, so a TY2026 field map fed this chain prints the whole itemized total in the *Other* box and leaves the real total blank. |
| the §68-style limitation behind that gate | **NO — and this is the money direction** | nothing in `crates/` computes it. Not limiting the deduction **overstates it and understates the tax**. Measured: the ordinary vector (1040 L11 $273,200) does **not** trip the gate; the high vector (L11 $1,113,200) **does** — *"more than $384,350"* — and the chain prints the unlimited $35,000 with **no refusal and no advisory**. The size cannot be stated: it needs S-5. |
| Schedule A **8d** *"Reserved for future use"* to *"Mortgage insurance premiums"*, and 8e *"through 8c"* to *"through 8d"* | **NO** | `line8e = 8a + 8b + 8c`. `Form1098::box5_mortgage_insurance` is already collected, so the figure exists and no line adds it — a **forgone deduction**, the safe direction, but silent. |
| Schedule 1 line 14 *"Armed Forces"* to *"… and the intelligence community"* (FR-220) | **NO** | no figure moves (btctax models no Schedule 1 Part II adjustment). The **sentence** a filer reads does — see A-4. |

★ **Already filed, so not claimed as new:** FR-185 (the six Schedule A collisions), FR-186 (the
`$384,350` trap — the same numeral is already in the tree as the TY2026 **MFS 37% bracket start**),
FR-187 (the casualty line's widened eligibility), FR-219, FR-220. **None of them is in the prediction
file** — `384,350`, `12e`, `11b`, `intelligence`, `Limitation Worksheet`, `ScheduleALines` and
`Form1040Lines` all return **0** grep hits there. That is worth recording on its own: the derived
blocker command and `FOLLOWUPS.md` have **disjoint** coverage of the Schedule A port.

---

## 3a. ★★★ LATE ADDITIONS — A-8 and A-9, after the coordinator's mid-task correction

The coordinator corrected the brief mid-run on three points and passed on one Tier B finding. All four
are folded in here rather than by editing §1a, so the order of discovery stays visible.

### A-9 (the corrected §68 target) — *"no such modelling exists anywhere"* is the complete answer

**Measured, not read.** `grep -rniE "section.?68|itemized.deduction.limitation|pease" crates/ --include='*.rs'`
returns **0** hits outside this harness. There is **no §68-style itemized-deduction limitation modelling
anywhere in the workspace**, and `FullReturnParams` has no field for its threshold.

**What the extract actually prints**, verbatim and in full (`f1040sa--2026-DRAFT.txt:158-162`):

> *"Is the amount on Form 1040 or 1040-SR, line 11b, minus the amounts on lines 13a and 13b of that
> form, more than **$384,350**? … **No.** Your deductions are not limited. Add the amounts in far-right
> column for lines 4 through 17z. Also enter this amount on Form 1040 or 1040-SR, line 12e. … **Yes.**
> Your deductions **may be** limited. See the **Itemized Deductions Worksheet** in the instructions to
> figure the amount to enter"*

★★ **`$384,350` appears exactly once on the form and carries NO filing-status parenthetical** — unlike
line 5e two inches above it, which does (*"$40,400 ($20,200 if married filing separately)"*). So the
form's own gate is a **single screening threshold for every status**, and *"may be limited"* is doing the
work: the per-status arithmetic lives in the worksheet, which is in `i1040sca--2026` / `i1040gi--2026` and
is **not archived** (S-5).

★ **So there is no "MFJ threshold per the extract" to report — the extract prints one number for
everybody.** The coordinator's FR-186 trap is therefore about *implementation*, not about the form: a
coder who reads the threshold from the bracket table gets a **Single** filer their own 37% start
($640,600) and screens them **out** of a limitation the form screens them **in**. My §3 table's §68 row
should be read with that correction: the vector at 1040 L11 $1,113,200 trips *the form's printed
screening threshold*, and how much (if any) limitation follows is **unresolvable without S-5.** No figure
is claimed.

### A-8 — ★★★ nothing tied a bundled template to its directory year. Now something does.

Tier B measured a TY2026 packet writing at exit 0 with **2024 printed on every face**. I reproduced the
missing-guard half and **refined the claim**, because a bare file swap *is* caught and the reason it is
caught is incidental:

**Measured differential.** Planting the realistic defect — `forms/2025/f1040sa.pdf` replaced by the
genuine 2024 template *and* its map row's `template_sha256` regenerated to match, so the pair agrees
with itself — over the whole `btctax-forms` suite:

| check | verdict under the plant | why |
|---|---|---|
| `map_rows::every_committed_map_has_a_row_that_parses` (kills 2, 3, 4) | **PASSES** | kill 2 compares the PDF to the row *beside it*; kill 3's `manifest_authority_hashes` is a **flat, year-agnostic** set so the 2024 hash *is* an authority; kill 4 compares the printed *Attachment Sequence No.*, which is **07 in both years**. |
| `map_pdf_conformance::every_committed_map_field_exists_in_its_own_pdf` | **REDS** | the 2025 map names AcroForm fields the 2024 PDF does not have. |
| `field_census::census_accounts_for_every_field` | **REDS** | same cause. |
| `full_return_forms::line_8b_overflows…` | **REDS** | same cause. |
| the new `every_bundled_template_is_the_document_its_year_archived` | **REDS**, naming `2025/f1040sa` | the 2025 extract records a different source SHA-256. |

★★ **So the existing coverage is INCIDENTAL: it holds only because these two revisions happen to differ
in field spellings.** It is silent in exactly the case that matters for TY2026 — a **wholesale copy** of
`forms/2025/*` into `forms/2026/` with `year` bumped. The map's field names then came from the same PDF
that is filed beside it, so the conformance, census and overflow checks all agree; kill 2 agrees; kill 3
agrees; kill 4 agrees. **Every existing check passes and the packet prints 2025 on every face.**

**The guard added.** Every archived extract records the SHA-256 of the PDF its text layer came from
(`# sha256:…`, verified: `forms/2025/f1040sa.pdf` is `c14acf3478f4c33f…` and `f1040sa--2025.txt` records
`sha256:c14acf3478f4c33f…`). So the join is: *for every bundled `(stem, year)`, the template's SHA-256
must be the one `design/forms/extract/<irs_stem>--<year>.txt` records.* A template that is that
document has that year's revision **printed on it by construction**.

* **Coverage is total today: 38 of 38 bundled templates, 0 mismatched, 0 without an extract.** The
  `irs_stem` alias (`schedule_d` → `f1040sd`) is read from the map ROW, so nothing is typed here.
* **It does not weaken in January.** It is keyed to the *identity of the bundled bytes*, not to the
  absence of a 2026 extract. Today a 2026 template would land in `NoExtract` and red; when
  `f1040sa--2026.txt` lands, the join demands the bundled 2026 template be that file's source.
* **Both failure classes are NAMED**, never skipped: `IsNot` (mis-filed) and `NoExtract` (nothing holds
  the revision). A checker that cannot tell those apart is not a conformance check.
* **Seen red twice** — a pure kill on the two real documents (`filing_the_2024_schedule_a_under_2025_is_refused`,
  which also asserts the accept direction and refuses a vacuously short prefix), and the live plant above.

---

## 4. THE HARNESS — it may stay in the repo, and every probe was seen RED

* `crates/btctax-cli/tests/ty2026_rehearsal.rs` — **10 tests**, the compute half.
* `crates/btctax-forms/tests/ty2026_emitter_reach.rs` — **4 tests**, the emitter half plus A-8's join.

★★ **The compute half is in `btctax-cli/tests/`, not `btctax-core/tests/` as the brief said, and the
reason is A-6.** The harness must read the **shipped** `ty2026_full_return()` rather than a second copy of
it, and `btctax-core` does not depend on `btctax-adapters`. A dev-dependency works and the suite passes —
except `repo_hygiene::every_intra_workspace_dependency_pins_the_current_version`, and a *versioned*
dev-dep on a downstream crate breaks the dependency-first publish order (publishing `btctax-core` would
demand `btctax-adapters 0.18.0` from crates.io before it exists). `btctax-cli` already has both crates in
`[dependencies]` and `tests/slice_from_answers.rs` already probes `full_return_for(2026)` there, so this
is the seam that was already TY2026-aware. **Zero manifest changes; no source file modified.**

**Unbundled by construction:** `ty2026_full_return()` is injected at the call site in every probe.
`full_return_for(2026)` is still `None`, and one test asserts it.

### B1 — eight plants, each killing exactly its own test and nothing else

| plant | edit (all reverted; SHA-256 checked back to the original) | what reddened |
|---|---|---|
| 1 | `schedule_a_parts` ignores `params.salt`, uses a flat `$40,000` | both SALT tests, nothing else |
| 2 | `form6251.rs` line 5 uses `0.25` and `$1,252,700` | the AMT test only |
| 3 | `total_17` gets a §68-shaped `× 35/37` | the Schedule A line-set tripwire only |
| 4 | a TY2026 arm added to `form6251_line1_rule` (the wall removed) | the wall pin only |
| 5 | the advisory prose widened to *"and the intelligence community"* | the FR-220 test only |
| 6 | an `i1040sca--2026.txt` planted in the extract archive | the worksheet test only |
| 7 | the 2024 Schedule A template filed under `forms/2025/` | A-8's live half (+ 3 pre-existing tests) |
| 8 | …and its map row's `template_sha256` regenerated to agree | A-8's live half **only** among the content joins — see §3a |

★ Plant 4 is also how the measurements past the wall in §3 were taken. It was reverted and the throwaway
probe file deleted; `return_1040.rs`, `form6251.rs` and `advisories.rs` are all back to their original
SHA-256, the planted extract was removed, and the two planted `forms/2025/` files were restored from
byte copies (never a checkout, per the standing rule about reverting mutations).

### Gate — run in the FOREGROUND, and nextest/clippy SERIALLY, in `target-a` / `target-a-clippy` under the worktree (never `/tmp`)

```
cargo nextest run --workspace --no-fail-fast                          → 3769 passed, 12 skipped   (exit 0)
cargo clippy --workspace --all-targets --all-features -- -D warnings  → exit 0
cargo fmt --all -- --check                                            → exit 0
```

Baseline at `9d1e98cfc` is **3755**; the harness adds **14**. Two clippy findings in my own test code
(`err_expect`, `assertions_on_constants`) were fixed rather than allowed.

---

## 5. REFUTED PREMISES (the brief asked for these explicitly)

1. **Row count.** The brief says the prediction file has *"66 rows"*; the coordinator then corrected it to
   *"58 rows plus 15 UNMEASURED"*. **Measured: 58 data rows in total — 43 `BLOCKED` and 15
   `**UNMEASURED**`** — which is what the file's own title says (*"58 rows (15 UNMEASURED)"*). The 15 are
   a subset of the 58, not an addition to them. `grep -c '^|'` returns 62 because it sweeps in four header
   and separator lines.
2. **"`forms_bundled` is 0 for TY2026 (no maps at all)."** True of *maps*; but `bundled_years()` **does**
   contain 2026 and two periodic forms resolve templates for it (A-5). The crate already separates the two
   senses correctly — `years_sentence()` is built from `TEMPLATE_YEARS` for exactly this reason — but the
   brief's phrasing collapses them.
3. **The 1040 renumber is a TY2025 change, not a TY2026 one.** `f1040sa--2025.txt` already prints *"Enter
   amount from Form 1040 or 1040-SR, line 11b"* and *"enter this amount on Form 1040 or 1040-SR, line
   12e"*. I wrote a test asserting the opposite and it reddened, which is how this was found.
4. **Schedule A `8d` is not a new line id.** TY2025 prints `8d Reserved for future use`; the **meaning** is
   new, not the number. A delta reported as "new line" would be filed under the wrong class — and that
   distinction is the whole point of the port machine's *meanings changed* column.
5. **The design doc §3's claim that Tier A "reaches the compute and emit surfaces" is false for TY2026
   today** — it reaches them only past a `panic!` a test cannot bypass without editing shipped source
   (A-1). The harness therefore pins the wall rather than crossing it.
6. **The design doc §2's gate condition is insufficient** (A-3).
7. **The design doc's "the rehearsal's core is one line."** For Tier A the substitution set is **two**
   items (§2); the coordinator reports Tier B measured **four**. Either way *"one line"* is the
   understatement, and both counts are larger than the design's.
8. **Tier B's "nothing in the tree asserts printed revision = directory year" — refined, not refuted.**
   Three existing tests DO red on a 2024↔2025 Schedule A swap, but only because those two revisions
   happen to differ in AcroForm field spellings. They are silent on a wholesale year-directory copy, which
   is the TY2026 shape. §3a has the measured differential.

---

## 6. FOLLOW-UP CANDIDATES (for the controller to file; I filed none)

| id | item | owning phase |
|---|---|---|
| **A-1** | Form 6251 Part I's TY2026 `None` is a `panic!`, not a `RefuseReason`, and it sits behind the one gate Tier B is about to open — on the CLI's **default** year. **Convert it to a refusal with an exit, and add "the TY2026 Form 6251 Part I arm exists" to the §2 bundling gate.** | **before any params insert** |
| **A-2** | `blockers.rs::gates()` is a hand-written `vec![…]` of **7**; a new year-keyed refusal point reds nothing. Derive it, or add a scan for `panic!` sites whose message interpolates `year`. Then plant an eighth and watch it red (B1). | with stage 1 |
| **A-3** | the §2 gate's condition must name Form 6251 Part I **and** the TY2026 Schedule 1-A — line 1a cites its **line 43**, and `SeniorDeductionSubtotal` has no lawful way to vouch for a line the schedule has not printed. | before any params insert |
| **A-8** | ★★★ **the printed-revision join is now in the tree** (`ty2026_emitter_reach.rs`, 38/38). What remains is a decision: should it move beside `map_rows.rs`' other three content joins, and should `manifest_authority_hashes` become year-keyed as well? | the TY2026 port, **before** `forms/2026/` gains a template |
| **A-9** | no §68-style limitation modelling exists anywhere (**0** grep hits). It needs its own `FullReturnParams` field and **must never** be read from the bracket table (FR-186). Blocked on S-5 for the arithmetic. | the TY2026 port |
| **A-4** | `Advisory::UnmodeledDeductionsOmitted` is year-free prose citing Schedule A lines **15/16** and *"the Armed Forces"*. One edit fixes both halves; find them together or they arrive a round apart. | the TY2026 port |
| **A-5** | `periodic_template`'s guard is `BUNDLED_YEARS.contains(&year)`, so a `preparing` year with zero templates still serves two forms. Confirm the export gate is the only thing stopping a two-page TY2026 packet, and say so where the function is defined. | the TY2026 port |
| **A-6** | `repo_hygiene`'s pin rule is section-blind and its stated justification (*"cannot be published at all"*) is false for `[dev-dependencies]`, where a version would instead **break** the publish order. Make it section-aware, or state the boundary in the test. | opportunistic |
| **A-7** | any future extract-derived line-set checker must not diff raw id sets across revisions whose `pdftotext -layout` column shapes differ. The blind spot is documented in `line_ids`'s doc comment in the harness. | with FR-188 |

---

## 7. WHAT TIER B AND THE JANUARY PORT SHOULD KNOW

1. **The insert the design calls "the rehearsal's core" does not produce a wrong figure — it produces an
   abort.** Expect `by_year.insert(2026, ty2026_full_return())` to turn every TY2026 command into a
   `panic!` from `form6251_line1_rule`, on the default year, with no `--year` needed to reach it. The
   minimum honest substitution past it is S-1 (reuse `Form6251Line1Rule::Y2025`), and it is only honest
   on a household whose senior-deduction subtotal is `$0`.
2. **Before `forms/2026/` gains its first template, land A-8's join** (or its equivalent beside
   `map_rows.rs`). A wholesale copy of `forms/2025/*` into `forms/2026/` passes every content join in the
   tree today.
3. **The parameterized half of the port is already done and now demonstrated working.** SALT's cap and
   phase-out and the AMT exemption phase-out all read TY2026's figures correctly, on measured vectors,
   with each older shape refuted. The port's remaining risk is entirely in **line sets and prose**, not in
   constants — which is the opposite of where a reader of FR-47 might expect it.

---

## 8. AND ONCE MORE: THIS VALIDATED NOTHING

Every figure in §3 and §3a was produced by an engine reading unbundled parameters against a **draft**
form, with no oracle in existence. They are evidence that a *mechanism* reads a *field*. They are not
evidence that any number is right. The only thing this run establishes about correctness is negative: the
**direction** of two gaps (§68 unlimited ⇒ tax understated; Schedule A 8d dropped ⇒ deduction forgone,
tax overstated), and even those sizes are unquantified because the worksheets that would size them are in
documents the IRS has not posted.
