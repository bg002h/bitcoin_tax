# REPORT — stage 2, **Tier B**: the TY2026 CLI journey on a throwaway branch

Run 2026-09-13. Brief: `design/agent-reports/BRIEF-stage2-run.md` @ `9d1e98cfc`.
Worktree `.claude/worktrees/agent-a32b7062870383e7c`, `CARGO_TARGET_DIR=…/target-b`. Nothing committed,
nothing pushed. Every command foreground. No subagents.

## 0. THIS RUN VALIDATES NOTHING

OpenTaxSolver 2026 does not exist until ~2027-01-27; `forms/2026/YEAR.toml` `[oracles]` says so itself
(*"none — OpenTaxSolver 2026 does not exist yet"*). Every template this run printed is a **byte copy of a
prior year's document**. Therefore **no figure below may be called correct, validated, or checked** —
not the $77,629 total tax, not the $54,000 itemized deduction, not the $19,721 refund. They are what the
code emits when fed a substituted year, and that is all. This is not S1: S1 has a signed return as its
answer key, and this has none. What this run produces is a list of **walls** and a list of **substitutions**.

Filer-input figures in the fixture (wages, interest, dividends, SALT, mortgage interest, a $5,000 cash
gift) are the household's own testimony, not tax authority. **No threshold, cap, bracket, phase-out or
published constant was typed anywhere in this run.** Where reaching the next wall would have required
one, I stopped and substituted a declaration instead — see §2, substitution 3.

## 1. The construction HELD. Both gates red, verbatim.

`by_year.insert(2026, ty2026_full_return())` reds **two** independent gates. Captured from
`cargo test`, quoted exactly:

`xtask::bin/xtask blockers::tests::no_bundled_params_year_has_unreadable_forms`
(panicked at `crates/xtask/src/blockers.rs:1543:9`):

```
TY2026: FullReturnParams are bundled but `f1040--2026` is NOT ARCHIVED — the year is declared computable while a document it must be transcribed from is absent
TY2026: FullReturnParams are bundled but `i1040gi--2026` is NOT ARCHIVED — the year is declared computable while a document it must be transcribed from is absent
TY2026: FullReturnParams are bundled but the §111(a) worksheet revision for 2026 is NOT TRANSCRIBED — archiving the document does not transcribe it (stage 1, plant B)
```

`btctax-adapters tax_tables::tests::ty2024_full_return_params_bundled`
(panicked at `crates/btctax-adapters/src/tax_tables.rs:1234:9`):

```
TY2026 full-return params appeared in `by_year`. The CONSTANTS exist and are pinned (`ty2026_full_return()`, FR-47) — that is not what this gate holds. It holds because the 2026 Form 6251 restructured Part I around Schedule 1-A and needs a re-transcription, and no OTS 2026 exists — see this function's doc comment before removing it.
```

And the whole suite, after all four substitutions:

```
Summary [ 108.567s] 3755 tests run: 3710 passed (1 slow), 45 failed, 12 skipped
```

`cargo nextest run --workspace --no-fail-fast`, captured once to `run-b/nextest.log`. **45 distinct
failures** — the branch could not merge silently under any reading. (Per the brief, fmt and clippy were
Tier A's obligation; Tier B ran nextest only.)

## 2. ★★ THE SUBSTITUTION SET — a blocker list derived by necessity

Four substitutions, in the order necessity produced them. Diff footprint: **3 tracked files, +40/−24
lines, plus 40 untracked byte-copied form files.** Nothing else.

| # | what was substituted | from what | authority read | in prediction? |
|---|---|---|---|---|
| 1 | `FullReturnParams` for TY2026 inserted into `BundledFullReturnTables::by_year` (`tax_tables.rs:103`) | `ty2026_full_return()`, already transcribed in-tree from Rev. Proc. 2025-32 + Pub. L. 119-21 | none invented | **yes** (2 rows) |
| 2 | `form6251_line1_rule(2026)` to `Form6251Line1Rule::Y2025` (`return_1040.rs:3025`) | TY2025's Form 6251 Part I rule. No TY2026 6251 is transcribed; only a DRAFT is archived, and `xtask blockers` reports **8 changed meanings** on it | none invented | **no** — see §4 F1 |
| 3 | `forms/2026/YEAR.toml` `prices_through` 2026-12-31 to **2026-06-03** | the last real close in `btctax-adapters/data/btc_usd_daily_close.csv` (ends `2026-06-03,64813.38`) | none invented — **see below** | **yes** (2 rows) |
| 4 | all 20 fillable stems' `.map.toml` + `.pdf` copied into `forms/2026/`, `year =` restamped to 2026 | `forms/2024/` for 19 stems; `forms/2025/` for `f1040s1a` only (Schedule 1-A does not exist in TY2024) | no TY2026 revision read | **yes** for the 21 per-form rows; **no** for *which prior year is usable* — see §4 F2 |

★ **Substitution 3 is where I refused the design's own instruction.** `TY2026_REHEARSAL_DESIGN.md` §1
lists *"the **price dataset's** tail"* among the things that "genuinely must be faked". **A daily close is
a figure.** Extending the dataset to 2026-12-31 means writing 211 prices nobody published, which the
brief's inviolable rule forbids without qualification. I therefore shortened the year record's
*declaration* instead — a statement about coverage, not a price — and the consequence is exactly what
that gate exists to prevent: **the packet in §5 is computed from a partial year.** (For this household it
changes no figure: its only 2026 disposition is dated 2026-03-10, inside coverage. That is luck about the
fixture, not a property of the substitution.) **The design and the brief conflict here, and the brief
wins.** Recorded as premise conflict P2 in §6.

★★ Substitution 4 needed **two attempts**, and the failed one is a finding (§4 F2): copying `forms/2025/`
forward — the obvious choice, one revision back — cannot fill a full return at all.

## 3. The journey, wall by wall

`init` to `import` to `verify` to `income import` to `report` to `export-irs-pdf` to `extension`, real
binary (`target-b/debug/btctax`), synthetic vault, MFJ household: W-2 $400,000 · 1099-INT $2,400 ·
1099-DIV $3,800 / $2,900 qualified · Form 1098 $18,000 · SALT $31,000 · $5,000 cash gift · itemized ·
2.0 BTC bought 2021-03-15, 0.5 BTC sold 2026-03-10. No Schedule C, no retirement.

| # | step | wall | named exit | worked? |
|---|---|---|---|---|
| 1 | `income import --year 2026` | `unknown variant Cash60, expected one of cash60, …` | the message lists the accepted spellings | yes |
| 2 | `income import --year 2026` | `[SharedMortgageInterestUnanswered]` — a Form 1098 row must say whether another borrower paid | *"in the tax-inputs editor, or in the Form 1098 table of your import file"* | yes |
| 3 | `export-irs-pdf --tax-year 2026` | `cannot export TY2026: this build has no usable f8949 map for the year … A PARTIALLY ported year must write nothing` | *"use --forms to select only the forms this build can fill"* | **NO — F3** |
| 4 | `export-irs-pdf --forms full-return` | `[BrokerAnswerUnread { provider: "coinbase", cohort: Covered, year: 2026 }] … a declaration about nothing is fabricated` | *"Remove the answer."* | yes |
| 5 | `report --tax-year 2026` | full-return computation unavailable — `full_return_for(2026)` is None | `income clear --year 2026` | not exercised (destructive) |
| | | to **substitution 1** | | |
| 6 | `report --tax-year 2026` | mortgage-interest-credit (Form 8396) unanswered; cites `i1040sca--2025.txt:1091-1096` **on a TY2026 return** | `btctax income answer` | yes, and see **F5** |
| 7 | `report --tax-year 2026` | **PANIC**, not a refusal: `Form 6251 Part I has never been transcribed for TY2026 …` (`return_1040.rs:3078`) | *"add the arm to form6251_line1_rule"* — a developer instruction | dev-only · **F1** |
| | | to **substitution 2** | | |
| 8 | `report --tax-year 2026` | `[CharitableCwaUnresolved]` | `Run btctax income answer` | **NO — F4, the headline** |
| 9 | `export-irs-pdf --tax-year 2026` | `the bundled price dataset ends 2026-06-03, before TY2026's prices_through 2026-12-31 … every figure on the packet would be computed from a PARTIAL year` | *"Update the bundled daily-close dataset (scripts/ — the price updater appends closes)"* | **NO — F6** |
| | | to **substitution 3** | | |
| 10 | `export-irs-pdf --tax-year 2026` | `IRS form fill: unsupported tax year 2026: this build bundles IRS forms for 2024 and 2025 only` | none on this arm | — |
| | | to **substitution 4a (from forms/2025/)** | | |
| 11 | `export-irs-pdf --tax-year 2026` | `geometric read-back FAILED (mis-mapped cell): the TY2026 1040 map has no [header] block — a full return cannot file an unnamed 1040` | none | **F2** |
| | | to **substitution 4b (from forms/2024/)** | | |
| 12 | `export-irs-pdf --tax-year 2026` | **exit 0** — 8 PDFs + `manifest.txt` written | — | **F8** |
| 13 | `extension --year 2026 --out ext` | **exit 0** — `f4868.pdf` written | — | **F8** |

Steps 5 and 10 wrote no byte and created no output directory: `ls irs/` returned
`No such file or directory` after each.

## 4. THE GAP

### 4.1 HIT, NOT PREDICTED

#### F4 — `CharitableCwaUnresolved`: the named exit does not reach the question, and the interview reports the opposite of what the return does. Year-independent, stock code, fully bundled year.

Reproduced on **TY2024** — params bundled, 20 forms bundled, prices covered — so it owes nothing to any
substitution in §2 (all four are 2026-keyed; the `return_1040.rs` diff touches only the
`form6251_line1_rule` match arm). With `charitable_cwa_obtained = false` and a $5,000 cash gift:

```
$ btctax income answer --year 2024
interview: complete · return: computable
Answered the full-return questions for tax year 2024.     <- exit 0; the CWA question is never asked

$ btctax report --tax-year 2024
NOT COMPUTABLE [CharitableCwaUnresolved]: … Then answer yes and re-run. If a charity will not
provide one, remove that gift from the deduction

$ btctax export-irs-pdf --out irs2024 --tax-year 2024
error: usage: the 2024 return is not computable [CharitableCwaUnresolved] … no forms were written
```

Four defects in one place:

1. **`return: computable` is false.** Two instruments in one binary, over one stored input, disagree about
   whether the return computes — the "instrument reporting something other than what it measured" class
   named in `CLAUDE.md`.
2. **The named exit is a dead end.** `income answer` skips a question whose answer is already recorded
   (`Some(false)`), so the exit the refusal names asks nothing and exits 0. Measured: `--re-answer` asks
   the CWA prompt **once**; the default asks it **zero** times. **The refusal never mentions `--re-answer`.**
3. **The panel puts it in the wrong bucket.** It prints under
   `FORGOING (5) — lawful to skip; each one costs YOU, not the Treasury`, with `BLOCKING` empty. It is not
   lawful to skip in any sense the panel means: `None` refuses and `Some(false)` refuses too
   (`return_1040.rs:3436`, `:3469`). The panel's `BLOCKING` bucket is *"commit waits on these"*; there is
   no bucket for *"the return will not COMPUTE until you answer, and answering no does not clear it."*
4. **The second cure has no actuator.** *"remove that gift from the deduction"* — no CLI verb removes a
   charitable gift; the filer must hand-edit the TOML and re-import.

Fails **closed** (no wrong form written), so not a wrong-result defect. It is a false status claim plus a
dead exit, which the repo's severity rule keeps blocking. **Proposed: Important.**

#### F8 — a TY2026 packet printed 2024 on every face, at exit 0, and nothing in the tree compares a bundled template's printed revision to the year it is bundled under.

`export-irs-pdf --tax-year 2026` wrote `00_f1040.pdf 02_f1040s2.pdf 07_f1040sa.pdf 08_f1040sb.pdf
12_schedule_d.pdf 12A_f8949.pdf 71_f8959.pdf 72_f8960.pdf manifest.txt`, exit 0, with the manifest's
stapling order, sign-by-hand list and mailing guidance. `pdftotext -layout` of what it wrote:

- `Form 1040 U.S. Individual Income Tax Return 2024` / `For the year Jan. 1–Dec. 31, 2024`
- `At any time during 2024, did you: (a) receive … a digital asset` — checked Yes, for a 2026 disposition
- `Were born before January 2, 1960` — the TY2024 §63(f) age test
- margin standard deductions `$14,600 / $29,200 / $21,900` — TY2024's
- `Schedule A (Form 1040) 2024`, `Schedule 2 (Form 1040) 2024`, `SCHEDULE B … 2024`,
  `Schedule D (Form 1040) 2024`, `Form 8949 (2024)`
- the itemized total on **line 17**, where TY2026 moves it to 18.

★★ And the sharpest cell on the page:

```
e Enter the smaller of line 5d or $10,000 ($5,000 if married filing
  separately) . . .                                        5e        31000
```

A box whose own printed instruction caps it at **$10,000** carries **$31,000**. The compute applied a
post-OBBBA SALT cap; the template prints the pre-OBBBA text. The geometric read-back passed, because
read-back verifies **placement** and never the caption's arithmetic claim about the value. **A signed
return that contradicts itself on its own face, at exit 0.**

`extension --year 2026` is worse, because the whole document is a year statement:

```
For calendar year 2024, or other tax year beginning        , 2024, and ending        , 20    .
5 Total 2024 payments . . . . .    97350
… is April 15, 2025, for most people …   6 months (October 15, 2025, for most calendar year taxpayers)
```

The string `2026` does not appear anywhere in the emitted `f4868.pdf`. A filer who signed and mailed it
would be requesting an extension **for 2024**, carrying 2026 figures. This matters more than the rest:
the owner's TY2026 plan is to file on the extension.

**Why this is a finding and not merely a consequence of my faking.** The tree today reds loudly (45
tests), but **every one of those catches is keyed to the absence of `design/forms/extract/<stem>--2026.txt`**,
never to the template's identity:

```
crates/btctax-forms/forms/2026/f1040.map.toml: bundled for 2026 but …/design/forms/extract/f1040--2026.txt
cannot be read — its captions are unverifiable, and a caption nobody can check is exactly what a port
carries forward                                    (line_coverage_check, 21 problems)

f4868.pdf is bundled for 2026 but its committed text layer is missing …: btctax POINTS THE FILER at a
table on that page, and nothing else checks the table is there.       (service_center_check)
```

**In January those guards stop firing,** because the 2026 extracts arrive. What then covers a
copy-forward port is only the *caption* comparison — and that catches a wrong-year face **only where the
captions happen to spell the year.** `f4868`'s do (`"Estimate of total tax liability for 2024"`), so it
would be caught. `schedule_d`'s do not — and `xtask port-status` reports `schedule_d` for TY2026 as
*"the FORM = unchanged, field map = transfers (0 respelled, 0 added, 0 removed, 0 lines moved, 0 retired,
0 introduced, 0 meanings changed)"*, which is precisely the row that invites a porter to copy the
directory forward. **There is no assertion anywhere that a bundled template's printed revision year
equals its directory year.**

The instrument best placed to hold it demonstrably does not. `label_reader`'s
`every_mapped_line_lands_on_its_own_printed_label` resolves each year's template **by sha256** against the
geometry fixtures (`every_map()`, `by_hash`, `label_reader.rs:1749`) — so it *knows by content* that
`forms/2026/f1040.pdf` is the TY2024 document, and printed the stem it chose:

```
2026: 20 map(s), 364 join(s) checked, 2 unreachable, 0 declared grid(s)
      NOT WITNESSED f1040v — f1040v--2024: no numbered label column found …
```

364 joins checked for year 2026, **zero wrong labels** — because it compared TY2024's map against
TY2024's labels. `disposition(year, form, stem, keys)` (`label_reader.rs:1515`) never compares
`stem_year(stem)` to `year`. The only 2026 failure it raised was `GRID_MAPS` bookkeeping
(`2026/f8949: NO numbered line key`).

Remedy, cheap and B1-shaped: assert `stem_year(resolved_stem) == directory_year` for every bundled
template, and plant this very substitution as the kill. Corollary of the same shape: my restamp changed
`year = 2024` to `2026` but left `line_set = "f4868/2024"` in the same file, and nothing compared them.
**Proposed: Important (the class), with the f4868 instance as its evidence.**

#### F1 — a year-keyed fail-closed written as `panic!` is invisible to the prediction instrument.

`report --tax-year 2026` (after substitution 1) did not refuse — it **panicked**, printing a Rust panic
and `note: run with RUST_BACKTRACE=1` to a filer:

```
thread 'btctax-main' panicked at crates/btctax-core/src/tax/return_1040.rs:3078:17:
Form 6251 Part I has never been transcribed for TY2026, so there is no line-1 rule to apply. REFUSING
rather than filing TY2024's Part I under a TY2026 heading … Fix: read Form 6251 for TY2026 from its text
layer and add the arm to `form6251_line1_rule` (see design/TY2026_PORT_REPORT.md §7 D3).
```

The stage-1 command's own census reads: *"RefuseReason has 133 variants. 5 distinct variants are raised at
a site that READS the tax year (11 sites)"*. A `panic!` is not a `RefuseReason`, so **this blocker cannot
appear in the prediction at all** — the instrument enumerates the wrong universe. Sibling found by grep:
`tables.rs:1286`, `schedule_1a_params(year).unwrap_or_else(|| panic!("TY{year} needs params"))`.

**Generalised, and this is the more valuable half: the prediction is blind to the class "a typed per-year
list inside a shipped crate with no TY2026 arm".** That is this repo's own highest-yield rule
(`CLAUDE.md`, *"Derive the list, or make the compiler hold it"*). Each row below appears in the 45
failures and **none** appears in the 58 prediction rows:

| year-keyed list with no TY2026 arm | where | reds as |
|---|---|---|
| `form6251_line1_rule` | `return_1040.rs:3023` | a **panic** in the filer's face |
| `f1040_clusters` amount band | `btctax-forms form1040` | `form1040::cluster_year_guard::geometry_is_recorded_for_exactly_the_supported_years` |
| `sec_clusters` (Form 8283) | `btctax-forms form8283` | `form8283::geometry_year_tests::…` |
| `se_clusters` (Schedule SE) | `btctax-forms schedule_se` | `schedule_se::cluster_year_guard::…` |
| `era::ALL_PRESETS` | `btctax-core/src/defensive/era.rs:94` | `census::the_newest_era_preset_reaches_the_newest_filable_tax_year` |
| `GRID_MAPS` | `xtask/src/label_reader.rs` | `2026/f8949: NO numbered line key` |
| `BUNDLED_BUT_NOT_SUPPORTED` | `btctax-forms` tests | `supported_years_and_bundled_year_directories_agree` |
| the validated-tables corpus | `btctax-adapters` tests | `every_shipped_year_has_a_validated_counterpart`: `params: [2026]` |
| the `mfs_kicker` doc-comment year list | `tax_tables.rs:1200` | `the years this test actually executes have changed: [2024, 2026]` |

The era preset deserves quoting on its own:

```
the newest era preset ends 2025-12-31, but btctax can file TY2026: a filer with a TY2026 shortfall has
no reachable acquisition-era preset. Extend the newest bucket in crates/btctax-core/src/defensive/era.rs
(and update its defensive_era.rs KATs).
```

`era.rs` documents that it is scheduled (*"the census drift guard actively schedules the next length
change for whenever a new filing year is bundled"*) — so the **code** predicted it and the **command** did
not. The word "era" does not occur in the prediction file; all 16 grep hits are substrings (`erage`,
`eral`, `erated`…). **Proposed: Important for the class; each instance is a January task.**

#### F2 — `forms/2025/` cannot fill a full return; TY2024 is the only complete map set in the build.

Substitution 4a copied `forms/2025/` forward — one revision back, the obvious port source. Export failed:

```
error: IRS form fill: geometric read-back FAILED (mis-mapped cell): the TY2026 1040 map has no [header]
block — a full return cannot file an unnamed 1040
```

`forms/2025/f1040.map.toml` is **98 lines** against TY2024's **406**, with no `[header]`,
`[filing_status]`, `[direct_deposit]`, `[[header.dependent_rows]]`, `[[direction]]` or `[census]` block —
a crypto-slice stub. Measured across the overlap:

```
f1040       2024=406  2025=98        f8283       2024=243  2025=53
schedule_d  2024=152  2025=35        schedule_se 2024=93   2025=32       f8949 2024=93 2025=60
```

The prediction's 21 per-form rows say *which* forms are unbundled for 2026; nothing in it says **which
prior year a port can start from**, and the answer is not the adjacent one. That is a real January
scheduling fact: `f1040`, `f8283`, `schedule_d`, `schedule_se` and `f8949` must be ported from TY2024,
across two revisions of field-name churn — and `f1040sa`'s own map warns *"TY2024's topmostSubform to
TY2025's form1. Not one FQN survives the year."* **Proposed: Important (scheduling).**

#### F3 — a refusal whose named exit cannot exist on a 0-forms year.

```
$ btctax export-irs-pdf --out irs --tax-year 2026
error: usage: cannot export TY2026: this build has no usable f8949 map for the year … No forms were
written; use --forms to select only the forms this build can fill, or wait for the year's package.

$ btctax export-irs-pdf --out irs --tax-year 2026 --forms f8949
error: usage: … (byte-identical message)
```

`forms_bundled` is 0 for TY2026, so **no value of --forms can succeed**, and passing one returns the same
sentence rather than saying so. The advice is sound on a *partially* ported year and false on a zero-form
year, the only state TY2026 has ever been in. **Proposed: Minor.**

#### F6 — the price gate's named exit addresses a developer, and the gate cannot see the one mechanism a filer has.

```
Update the bundled daily-close dataset (`scripts/` — the price updater appends closes) and re-export.
```

A filer installed a binary; they have no `scripts/`. The product *does* ship a filer-side mechanism —
`btctax-update-prices`, which fetches closes into a **local price cache** — and the gate cannot see it:
`price_coverage_or_refuse(year: i32)` takes no cache argument and reads
`YearReadiness::bundled(year).prices_max_date`, which is `BundledPrices::load()` — the `include_str!`'d
compiled-in dataset (`btctax-adapters/src/price.rs:10`). So a filer who fills the cache through
2026-12-31 is still refused. Consequence: **a TY2026 filer cannot export until a new btctax RELEASE ships
a longer dataset** — a reasonable policy, but not what the sentence says. The prediction carries this
blocker twice and says nothing about its exit. **Proposed: Minor (message), plus a scheduling note that
the January package includes a price-dataset release.**

#### F5 — a TY2026 refusal quotes the TY2025 instructions by filename without saying the year's own revision does not exist.

```
Schedule A's Line 8a Caution says "…" (i1040sca--2025.txt:1091-1096)
```

printed on a **TY2026** return. The archive gap is predicted (`i1040sca` row); a filer-visible citation
that silently falls back a year is not. **Proposed: Minor.**

#### Three smaller ones, filer-facing, none predicted

- **F9 — `income project --year 2026` exits 0 with `"refused": null`** and a complete household row, and
  says nothing about there being no engine to consume it. Its own `--help` says *"Run it before you
  export"*; `forms/2026/YEAR.toml` `[oracles]` already records `ots = "none — OpenTaxSolver 2026 does not
  exist yet"`, and the command does not read it. **Minor.**
- **F10 — `docs/income-import-schema.md` mis-describes its own worked example** as *"A married-filing-jointly
  household with wages, interest, dividends, an itemized Schedule A and Bitcoin dispositions."* The example
  has `int_1099 = false`, `div_1099 = false`, no `[[int_1099]]`, no `[[div_1099]]`, no `[schedule_a]`, and
  `itemize_election = "auto"` with no itemized input. **The owner's exact profile has no worked example in
  the schema doc** — I had to build one, which is where walls 1 and 2 came from. **Minor.**
- **F11 — `form_1098[].other_borrower_paid_interest` is listed as optional** (blank notes column) but a
  Form 1098 row cannot be stored without it. The doc's rule is *"required when omitting the key breaks the
  PARSE"*; this one breaks the **store**, so the table is literally true and practically misleading. **Nit.**

### 4.2 PREDICTED AND HIT — no news

- `full_return_for(2026)` is None, THE compute gate (2 rows) — wall 5.
- the export-time price gate (2 rows) — wall 9.
- the crypto slice cannot print: no `f8949` / `schedule_d` map (1 gate row + 21 per-form rows) — walls 3, 10.
- the year declares `status = "preparing"` — surfaced by
  `year_readiness::every_bundled_years_declaration_agrees_with_the_build`: *"TY2026: FullReturnParams are
  bundled but the year declares itself Preparing — declare it filable or unbundle them."*
- `f1040--2026` / `i1040gi--2026` unarchived — the §1 gate messages, verbatim.
- the §111(a) worksheet revision is not TRANSCRIBED — the §1 gate's third line. The design's ★★★ CORRECTED
  block is confirmed: archiving would not have cleared it.

### 4.3 PREDICTED, NOT HIT

Not "cleared" — **out of this journey's reach**, and I will not claim otherwise:

- the §111(a) `StateAndLocalRefundWorksheetNotComputed` refusal never fired, because this household
  received no state refund (`documents.g_1099 = false`, `state_refund_without_1099g = false`). A household
  with one would hit it; the gate row is correct.
- `Schedule1aNotOnThisYearsReturn` never fired: TY2026 *has* Schedule 1-A, and this household has no tips,
  overtime, car-loan interest or senior deduction, so the schedule is correctly absent from the packet.
- the 11 literal (family, revision) pins (`f6251--2024`, `f8949--1999`, `i1040sd--2025`, …) are silent by
  construction — that is the row's own claim (*"archiving X--2026 reds nothing"*), and this run could not
  test it without archiving a 2026 extract.
- the 3 owner decisions (S1/S2/S7) and the §170(f)(11)(C) appraisal: not code, not reachable.
- Schedule C / Schedule SE / Form 8995 / Form 8889 rows: no Schedule C and no HSA in this profile.

## 5. What the packet actually contained

For the record only — **not a validated figure among them.**

```
00_f1040.pdf  02_f1040s2.pdf  07_f1040sa.pdf  08_f1040sb.pdf
12_schedule_d.pdf  12A_f8949.pdf  71_f8959.pdf  72_f8960.pdf  manifest.txt
```

Total income 441,200 · AGI 441,200 · itemized deduction 54,000 · taxable income 387,200 · tax 74,713 ·
Form 8959 1,350 · Form 8960 1,566 · AMT 0 (AMTI 418,200, exemption 140,200, TMT 68,111 < regular 74,713) ·
total tax 77,629 · payments 97,350 · refund 19,721. Eight advisories printed. Schedule B Parts I and II
filled, Part III answered No. No Schedule 1 and no Schedule 1-A: this household has no line for either,
which is the correct blank.

Two things I checked and will **not** report as defects:

- **No §68-style itemized haircut was applied** despite AGI 441,200 > 384,350. On inspection that is
  correct for **MFJ**: 384,350 is the **MFS** 37% threshold (`ty2026_mfs_37_pct_starts_at_384350`), and
  MFJ's is roughly double, above this household. An MFJ fixture cannot test the new limitation; a Single
  or MFS one can. **Tier A should re-aim that probe** — see P4.
- **No FR-218-class duplicate or blank page.** `pdfinfo` page counts: `f1040` 2, `f1040s2` 2,
  `schedule_d` 2, the rest 1, `f4868` 4. The apparent trailing empty page in a naive `pdftotext` split is
  the form-feed terminator, not a page.

## 6. PREMISES I CAN DISPROVE

**P1 — the brief's row count is wrong.** It cites the prediction as "(66 rows)". The file's own header says
`58 rows (15 UNMEASURED)`, and both are correct: counting rows whose first cell is BLOCKED or UNMEASURED
gives **58**, of which **15** are UNMEASURED (41 + 13 + 3 + 1 by section). Nothing downstream depends on
it; recorded so the number does not propagate.

**P2 — the design instructs a fake the brief forbids.** `TY2026_REHEARSAL_DESIGN.md` §1: *"What genuinely
must be faked is … the **price dataset's** tail"*. The brief: *"Never invent a figure."* A daily close is a
figure; the two cannot both be followed. I followed the brief (§2, substitution 3), and the design's §1
list should lose that item — the honest move at that wall is to shorten the declaration, or to teach the
gate to read the local price cache, not to write prices.

**P3 — "the rehearsal's core is one line" understates it by the whole form package.** Design §1: *"The
rehearsal's core is **one line**: by_year.insert(2026, ty2026_full_return())."* Measured: that line moves
the journey from wall 5 to wall 7, which is a **panic**. Three further substitutions and 40 copied files
are needed before a single byte is written. §1 does acknowledge the 1040 template and the price tail two
paragraphs below, so the "one line" sentence contradicts its own section; the sentence is the wrong one.

**P4 — one Tier-A target is unreachable from the profile the brief specifies** (see §5): the §68-style
$384,350 gate is MFS's 37% threshold, so an MFJ household cannot trip it.

## 7. DELETE THE BRANCH

Per the design: *delete the branch; keep the report and the gate.* Nothing here is folded. The tree is left
dirty and uncommitted:

```
 M crates/btctax-adapters/src/tax_tables.rs        (+2)
 M crates/btctax-core/src/tax/return_1040.rs       (+4/-1)
 M crates/btctax-forms/forms/2026/YEAR.toml        (+34/-23)
?? crates/btctax-forms/forms/2026/*.map.toml       (20 byte copies)
?? crates/btctax-forms/forms/2026/*.pdf            (20 byte copies)
```

Each source edit carries an in-source `★★ TIER-B REHEARSAL SUBSTITUTION n — BRANCH ONLY, NEVER MERGED`
marker, and `forms/2026/YEAR.toml`'s `[forms_absent]` block records what each copy was copied **from**.
The suite reds 45 tests until they are reverted. The run's scratch artifacts (vault, fixture TOMLs, the
emitted packet, `nextest.log`) are under `run-b/`, untracked, and can be deleted with the branch.

## 8. What Tier B taught, in one line each

1. **F4** — a refusal on a live, bundled year whose named exit does nothing, beside an interview that says
   `return: computable` while the return refuses. The only finding here that is live in a shipped build.
2. **F8** — nothing compares a bundled template's printed year to its directory year, and the guards that
   currently mask it stop firing in January.
3. **F1** — the prediction's universe is `RefuseReason` variants and archives; the January blockers that
   are *typed per-year lists inside shipped crates* are a whole class it cannot see, and one of them greets
   the filer as a panic.
4. **F2** — the port source for the full-return forms is TY2024, not TY2025.
5. **F3 / F5 / F6 / F9 / F10 / F11** — six exits, citations and documents that mislead the filer rather
   than stop them.

And the negative result, stated plainly because the brief demands it: **§4.2 taught nothing.** Six
prediction rows were confirmed exactly as written. The command works. That part of this run is no news and
has not been padded into any.
