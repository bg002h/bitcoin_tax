# Phase review — design r2 steps 4–5 and the steps-2/3 fold (11a52555, b58dee9d, 162739e8)

Reviewer: Claude Opus 5 (1M), independent, read-only. Date: 2026-09-06. HEAD: `162739e8fc119f7a2fd1fed56465a56f403f31d5`
(during the review the branch gained `9e81a9b0`, a continuity-only commit, and an **uncommitted** 95-line
addition to `crates/btctax-cli/src/year_readiness.rs` — see R3).

**Read:** `git show --stat` for all three commits and the full diffs of `field_census.rs`, `packet.rs`, `map.rs`;
`forms/{2017,2024,2025}/YEAR.toml` in full; `src/year_record.rs`; `build.rs` (the YEAR.toml arm, lines 58–95 and
179–196); `tests/year_record.rs`; `tests/line_set_wiring.rs`; `src/line_set.rs`; `tests/field_census.rs` (the
`UNCENSUSED` register + `every_emittable_form_is_reached_by_the_gate_or_named_absent`); `tests/map_pdf_conformance.rs`;
`crates/btctax-cli/src/year_readiness.rs`; `src/packet.rs` (`stapled`, `attachment_sequence`, the sequence tests);
`src/bundled.rs::periodic_template`; `src/lib.rs::fill_form_8959`; `crates/xtask/src/label_reader.rs`
(`line_bindings`, `label_matches`, `every_map`, `every_mapped_line_lands_on_its_own_printed_label`, `YEAR_FLOORS`,
`audit_year_reach`); `crates/btctax-adapters/src/tax_tables.rs`; `crates/btctax-core/src/conventions.rs`,
`defensive/era.rs`, `project/transition.rs`, `project/resolve.rs:1650-1690`; `crates/btctax-tui/src/{app.rs,unlock.rs}`,
`crates/btctax-tui-edit/src/editor.rs`; `scripts/oracle/ots_direct.py`; design r2 §6/§7/§9/§10; `design/ROADMAP_STATUS.md` §0a.

**Ran:**
- `cargo nextest run -p btctax-forms -E 'binary(year_record) + binary(line_set_wiring)'` → **8 passed**.
- `cargo nextest run -p btctax-cli -E 'test(year_readiness)'` → **6 passed** (includes the 3 uncommitted tests).
- `cargo nextest run -p xtask -E 'test(every_mapped_line_lands_on_its_own_printed_label)' --no-capture` → PASS; printed
  `2017: 5 map(s), 0 join(s) checked, 5 unreachable` / `2024: 17 map(s), 99 join(s), 1 unreachable` /
  `2025: 15 map(s), 82 join(s), 0 unreachable` / `4 binding(s) landed on a box the reader could not label ('?')`.
- A python re-implementation of `label_reader::line_bindings` over every committed map, counting real
  `lineN = "<FQN>"` bindings vs those the parser accepts (numbers in R2).
- `python3` weekday computation for 2018-04-15/16/17, 2025-04-15/16, 2026-04-15/16.
- `.venv/bin/python -c "import taxcalc"` → 6.8.2; `Policy.JSON_START_YEAR` 2013, `LAST_KNOWN_YEAR` 2026.
- `ls ~/OpenTaxSolver*/bin/`; `tail -3 crates/btctax-adapters/data/btc_usd_daily_close.csv` → dataset ends 2026-06-03.

**Not re-derived (settled per the brief):** the design shape; steps 1–3 and their reviews; the owner rulings; that
TY2025 is paused; that `line_set` is one revision per file.

## Verdict: 0 Critical / 4 Important / 6 Minor (+2 Nit)

## Record-value audit (3 records × 9 fields)

| field | 2017 | 2024 | 2025 | source checked | verdict |
|---|---|---|---|---|---|
| `year` | 2017 | 2024 | 2025 | directory name; `tests/year_record.rs:20` | ✅ all three |
| `status` | `slice` | `filable` | `preparing` | `tax_tables.rs:73-80` (TaxTable 2017/2024/2025/2026 all registered), `:99-104` (**params: TY2024 only**) | ✅ all three |
| `return_due` | 2018-04-17 | 2025-04-15 | 2026-04-15 | calendar: 2018-04-15 **Sunday**, 2018-04-16 **Monday** = DC Emancipation Day ⇒ Tue 04-17. 2025-04-15 Tue (Emanc. Wed 04-16, no shift). 2026-04-15 Wed (Emanc. Thu 04-16, no shift). 2025 also `== conventions::TY2025_RETURN_DUE` (`conventions.rs:19`), the constant `resolve.rs:1668` reads | ✅ all three |
| `tables` | Rev. Proc. 2016-55 §3.01/§3.35/§3.37 + SSA 2016 COLA | Rev. Proc. 2023-34 + SSA 2023 COLA | Rev. Proc. 2024-40 (+ "Pub. L. 119-21 for the OBBBA figures") | `tax_tables.rs:394-405` (2017: §3.01 rates, §3.37 $14,000, §3.35 $5,490,000, SSA 2016-10-18 $127,200), `:15-21` + `:112-114` (2024: §3.01/§3.03/§3.15, SSA 2023-10-12), `:23-34` (2025) | ✅ 2017, ✅ 2024, ⚠️ 2025 — **R10** |
| `prices_through` | 2017-12-31 | 2024-12-31 | 2025-12-31 | `btc_usd_daily_close.csv` ends **2026-06-03** ≥ all three; enforced only for `filable` (`year_readiness.rs:78-85`) | ✅ all three |
| `forms_expected` | 5 | 17 | 15 | `ls forms/<year>` + `glob_problems` + `every_bundled_year_has_a_record…` (ran, green) | ✅ all three |
| `forms_absent` (reasons) | 13 | 1 | 3 | each reason checked below | ✅ 2017, ✅ 2024, ⚠️ 2025 — **R5** |
| `oracles` | `ots="none — OTS 2017 is not archived"`, `taxcalc="none"` | `ots="OpenTaxSolver 2024 (OpenTaxSolver2024_22.07_linux64)"`, `taxcalc=">= 6.8.2"` | `ots="OTS_2025 — archived? no"`, `taxcalc=">= 6.8.2"` | `ls ~/OpenTaxSolver*`; `ots_direct.py:75-79,660-666`; `.venv` taxcalc 6.8.2 | ⚠️ 2017 — **R9**; ✅ 2024; ❌ 2025 — **R1** |
| `information_returns.f1099da` | `false/false` | `false/false` | `proceeds=true/basis=false` | TD 10000: gross proceeds for sales on/after **2025-01-01**, basis for covered assets acquired on/after **2026-01-01** | ✅ all three |

**Every `forms_absent` reason, checked.** TY2017: `f1040s1` "pre-TCJA Form 1040 carried no Schedule 1 (lines 7–37 on
its face)" ✅ (Schedules 1–6 are the TY2018 redesign); `f1040s1a` "Schedule 1-A is TY2025+ (OBBBA)" ✅ (Pub. L. 119-21);
`f8995`/`f8995a` "§199A did not exist for TY2017 (TCJA, TY2018+)" ✅; `f8275` "periodic (Rev. 10-2024) — served by hash
from forms/2024/" ✅ (`bundled::periodic_template` serves any BUNDLED year lacking its own file from the newest
`periodic` row); the seven "crypto slice only" rows ✅ (consistent with `status = slice` and the five files on disk).
TY2024: `f1040s1a` ✅. TY2025: `f1040s1` "no TY2025 map yet" ✅ (no `f1040s1.map.toml` in `forms/2025/`); `f8995a` "not
ported" ✅; `f8275` ✅ on substance, ⚠️ on its pointer (**R5**).

## Kill audit

| kill | where | can it fail? | verdict |
|---|---|---|---|
| a year dir with no `YEAR.toml` | `build.rs:63-70` | yes — panics at build; commit records it observed red by moving 2025's aside | ✅ real, and it is what makes the census's "registered" check below redundant |
| third state (form neither expected nor absent) | `partition_problems` + `a_form_dropped_from_the_absent_list…` | yes — plants `f1040s1a`→`f1040s1a_gone`, asserts **both** messages | ✅ real |
| glob, both directions | `glob_problems` + `a_phantom_expected_form_and_an_undeclared_bundled_form…` | yes — plants a phantom `f8995a` in 2025's expected list and an extra `f1040s1` in `present` | ✅ real, but see **R6** on what it cannot catch |
| mistyped status / unknown key | `a_mistyped_record_is_refused` | yes — 3 mutations (`fileable`, `prices_thru`, `moon`), all `is_err()` | ✅ real (`deny_unknown_fields` + `toml_date` refusing a datetime) |
| `filable` without params / prices short / params on a `preparing` year | `year_readiness.rs:167-196` | yes — three field plants on a live `YearReadiness`, each asserting its own sentence | ✅ real; ⚠️ the `Slice`/`Preparing` arm never checks `table` (**R11**) |
| declared-vs-build for every bundled year | `every_bundled_years_declaration_agrees_with_the_build` | yes — would red on any of the above for 2017/2024/2025 | ✅ real (ran green) |
| `return_due` ≡ core constant | `the_declared_ty2025_due_date_is_the_core_constant` | yes — two independently-written literals held equal; reds if either moves | ✅ honest bridge. **Limit:** it binds TY2025 only; the generic check is `return_due.year() == year + 1`, which cannot discriminate a wrong *day* (2018-12-25 would pass). The 2017/2024 dates rest on this review — both verified against the calendar above |
| wired ⇔ parses over the ten | `tests/line_set_wiring.rs:64-69` + the pinned table | yes — two-way (`schema(ls) != Unwired` ⇔ `verdict == "PARSES"`); commit records un-wiring `f8959/2025` reddening both | ✅ real (ran green) |
| the Unwired set is exactly two | `line_set.rs:338-345` | yes — exact `assert_eq!` on the list, shrink-only | ✅ real |
| `[census]` coverage of the eight | `UNCENSUSED` register (`field_census.rs:70-86`) + `every_bundled_year()` (`years.len() >= 3`) | yes | ✅ **the step-5 `[census]` claim is TRUE**: the five uncensused TY2025 maps are `f1040`, `f8283`, `f8949`, `schedule_d`, `schedule_se` — **none of the eight** |
| map ⊆ PDF fields for the eight | `map_pdf_conformance.rs:21-31` derives `(year, stem, map, pdf)` from the filesystem across all years | yes | ✅ TRUE for 2025 |
| **line → printed label for the eight** | `label_reader.rs:1350-1422` | **partially — it examined 0 of Schedule A/2025's 19 bindings** | ❌ **R2** |
| census "complete = declared filable" | `field_census.rs:610-615` | it now reads a declaration, not a measurement | ⚠️ **R7** |
| census "every year on disk is registered" | `field_census.rs:585-595` | **no** — `YearRecord::for_year` is `Some` for every bundled year by construction (build.rs refuses otherwise) | ⚠️ tautological, but harmless: the real kill moved to `build.rs` and was observed red (**R7**, second half) |

## Findings

### R1 — IMPORTANT — TY2025's record declares no OpenTaxSolver oracle, and one is installed and already in use as that year's second witness

**Where:** `crates/btctax-forms/forms/2025/YEAR.toml:36` — `ots = "OTS_2025 — archived? no; expected as the TY2026 baseline's prior side"`.

**What is wrong:** the tree exists and runs. `~/OpenTaxSolver2025_23.06_linux64/bin/` contains `taxsolve_US_1040_2025`,
`taxsolve_US_1040_Sched_C_2025`, `taxsolve_US_1040_Sched_SE_2025`, `taxsolve_f8959_2025`, `taxsolve_f8960_2025`,
`taxsolve_f8995_2025`, `taxsolve_f2210_2025`, `taxsolve_f8812_2025`. The harness has a setting whose only purpose is
selecting it — `ots_direct.py:75-79`: *"Two installs coexist here (`OpenTaxSolver2024_22.07_linux64` and
`OpenTaxSolver2025_23.06_linux64`) … the tree and the year are two facts, so they are two settings."*
`design/amt-form6251/CONTINUITY_TY2025.md:121` — *"Installed at `~/OpenTaxSolver2025_23.06_linux64` (SourceForge
`OTS_2025/v23.06_linux`)"*. `design/agent-reports/2026-09-05-ty2026-port-oracles.md:53`, written the same day as this
record — *"Installed trees (the only two on this box)"*, naming it. `design/ty2025/reviews/r2-four-lenses-…md` cites
that tree's `taxsolve_US_1040_2025.c` line-by-line as the TY2025 witness for SALT and QSS.
The 2024 record's convention for this field is the install name (`"OpenTaxSolver 2024 (OpenTaxSolver2024_22.07_linux64)"`,
which is verbatim `ots_direct.version()`'s fallback). Under the same convention TY2025's entry is simply false, and
TY2017's `"OTS 2017 is not archived"` — which *is* true, no such tree exists — sets the reading of "archived".

Nothing tests this field: `tests/year_record.rs` checks `status`, the two dates, the regime, the partition and the
glob. `tables` and `oracles` are unchecked prose, and one of the two is already wrong on its first day. The harm is
the year record being the artefact a later reader trusts for *"do we have two witnesses for TY2025?"*, against the
standing rule *"Never file an upstream defect, or call a figure validated, on one oracle."*

**Minimal change:** `ots = "OpenTaxSolver 2025 (OpenTaxSolver2025_23.06_linux64) — OTS_YEAR=2025"`. Optionally give
`oracles` a kill: assert the named OTS tree resolves, or that the string is one of a closed set.

### R2 — IMPORTANT — the line→printed-label join, which step 5 names as its criterion, examined ZERO of Schedule A/2025's nineteen line bindings

**Where:** `crates/xtask/src/label_reader.rs:1047-1072` (`line_bindings`); the claims it licenses are
`crates/btctax-forms/src/line_set.rs:78-99` (eight doc comments: *"map ⊆ PDF fields, the label join and `[census]`
all green"*), commit `162739e8`'s message, and design §10 step 5 (*"the line→printed-label join, derived over every map"*).

**What is wrong:** `line_bindings` accepts a binding only when the **entire trimmed RHS** is a quoted string:

```rust
if let Some(v) = rhs.trim().strip_prefix('"').and_then(|s| s.strip_suffix('"'))
```

Every map line carrying a trailing `# comment` that does not itself end in `"` is dropped — silently, with no counter
and no report. Measured over the committed maps (real = `lineN = "<FQN>"`; seen = accepted by that parser):

| TY2025 map | real | seen | | TY2024 map | real | seen |
|---|---|---|---|---|---|---|
| **f1040sa** | **19** | **0** | | f1040 | 35 | 1 |
| f1040sc | 9 | 2 | | f1040s1 | 10 | 0 |
| f8960 | 15 | 11 | | f1040sa | 19 | 0 |
| f8995 | 16 | 12 | | f1040s2 | 6 | 2 |
| f1040s1a | 46 | 1 | | f1040s3 | 5 | 3 |
| f1040s2 / s3 / sb | 6/5/4 | 6/5/4 | | schedule_d | 9 | 3 |
| f6251 / f8959 / schedule_se | 42/17/12 | 42/17/12 | | f8960 / f8995 | 15/16 | 12/12 |
| **TOTAL 2025** | **195** | **116** | | **TOTAL 2024** | **237** | **152** |

`f1040sa/2025` is one of the eight wired at step 5, and the instrument the wiring cites looked at **none** of its lines.
The year floor cannot see this: `YEAR_FLOORS` records `2025 { min_joins: 82, max_unwitnessed: 0 }` and the run prints
`2025: 15 map(s), 82 join(s) checked, 0 unreachable` — Schedule A *is* witnessed (its geometry fixture exists), it just
contributes nothing, and the floor only aggregates. A second silent path compounds it: `let Some(got) = join.get(&fqn)
else { continue };` is uncounted, so of the 116+152 = 268 parsed bindings only 82+99 = 181 actually joined, and the run
reports just **4** of the 87 lost as `?`.

The residual risk is exactly what the join exists for: `map_pdf_conformance` proves the FQN *exists* in the PDF; only
the label join proves it is the box the form prints *that line number* beside. That is the Form 6251 line-33/line-22
class named in `CLAUDE.md`. (Mitigating: `f1040sa--2025.map.toml`'s own header records a manual
`xtask label-boxes f1040sa--2025` corroboration — *"The two agree on all 33"* — so the map is probably right. What is
missing is the instrument, not necessarily the correctness.)

**Minimal change:** take the value as the **first quoted token** on the line (or parse the map as TOML) and re-measure
`YEAR_FLOORS`. ★ The fix must skip non-numeric `line_*` keys — Schedule C's `line_a_business` / `line_b_naics` would
otherwise produce false failures, because `label_matches("_a_business", …)` takes `split('_').next()` = `""` and
returns false. Then add the thing the aggregate floor cannot express: a **per-map** join count, so a map contributing
zero is loud. Until that runs green for `f1040sa/2025`, the eight are wired on seven maps' evidence, not eight.

### R3 — IMPORTANT — `YearReadiness` had no non-test reader in the reviewed range, while its own module doc states, in the present tense, that it is read everywhere

**Where:** `crates/btctax-cli/src/year_readiness.rs:1-6` — *"this is where 'is TY2026 ready?' gets one answer,
**rendered on every number-bearing surface and used to build every refusal string**, instead of five literals that
drift"*; design §6 says the same.

**What is wrong:** `grep -rn "YearReadiness\|year_readiness\|default_year" crates/` over the reviewed tree returns, as
non-test readers, exactly two lines — `btctax-tui/src/app.rs:195` and `btctax-tui-edit/src/editor.rs:300`, both calling
`default_year()`. `problems()` and `sentence()` are called only from the module's own `#[cfg(test)] mod tests`. No
refusal string, no status surface, no number-bearing surface reads the type. Design §10 step 4's checklist ("`YEAR.toml`
… with `YearReadiness` and its kills; move the four literals in") does **not** promise the surfaces, so this is an
overclaiming doc comment rather than a missed deliverable — but a present-tense false statement about wiring is the
class this repo reds on, and it is the sentence a future reader will trust instead of grepping.

**★ Observed mid-review:** the working tree carries an **uncommitted** 95-line addition to this exact file —
`YearReadiness::bundled`, `uncomputable_sentence`, `import_note`, `export_stamp` and three tests (FR-48 / port report
§2.5) — i.e. the readers are being written now. This finding closes when that lands; it is recorded because the
committed range asserts the wiring today.

**Minimal change:** land those readers, or make the doc say *"step 4 introduces the type and its kills; the surfaces
read it at step N."*

### R4 — IMPORTANT — the year literal that actually reaches a user survived step 4: `latest_year()` falls back to a hardcoded `2025`

**Where:** `crates/btctax-tui/src/unlock.rs:223-230`

```rust
pub fn latest_year(state: &LedgerState) -> i32 {
    …
    from_disposals.chain(from_income).max().unwrap_or(2025)
}
```

**What is wrong:** both TUIs' constructors now read `default_year()` (`app.rs:195`, `editor.rs:300`) — but both
**overwrite** it on every successful open with this function's result (`app.rs:231` via `OpenOutcome::Success(_, year)`;
`editor.rs:384` and `:412` via `SessionOpenOutcome::Success { year }`, both sourced from `build_snapshot`'s
`let year = latest_year(&state)` at `unlock.rs:206`). So the constructor literals that were retired are the ones a user
never sees, and the literal that decides the year for a vault with no disposals and no income — a brand-new filer's
first open, and the one case where a default is the whole answer — is still typed `2025`.

Impact today is nil (`default_year()` is 2025 too). The moment `forms/2026/` lands, the derived default moves to 2026
and this one does not: two defaults, silently disagreeing, in the same binary. §9's row counted two `selected_year:
2025` literals; this is the third, and the survivor.

**Minimal change:** `.unwrap_or_else(btctax_cli::year_readiness::default_year)` — and consider whether the right answer
for an empty vault is the newest *bundled* year or the newest *filable* one (see the Minor below).

### R5 — MINOR — TY2025's `f8275` absence cites a function that `b58dee9d` took off that path eight minutes later

**Where:** `forms/2025/YEAR.toml:32` — *"served by hash from forms/2024/, licensed in `Form8275Map::alias_is_licensed_by`"*.
`b58dee9d` removed that call (`map.rs:1376-1400`): the licence is now `bundled::periodic_template`'s pairing of the bytes
with the year whose row says `periodic`, stated by a `debug_assert_eq!`; `alias_is_licensed_by` *"remains as the
documented statement of the hash rule … but is no longer on this path."* The substance of the reason is still correct
(`periodic_template` does serve 2017 and 2025 from 2024 — verified at `bundled.rs:151-170`); only the pointer is stale.
TY2017's f8275 reason carries no such pointer and is fine.
**Minimal change:** drop the clause, or point it at `bundled::periodic_template`.

### R6 — MINOR — `glob_problems` cannot catch a record regenerated to match a broken directory, and the pins that can live outside the record — and only for TY2024

**Where:** `year_record.rs:156-177`; `tests/year_record.rs:29-34`.
Both sides derive from the same glob (`forms_expected` was computed from disk at authoring; `present_for` reads
`bundled::BUNDLED`). Delete `forms/2017/f8949.*` and move `f8949` into `[forms_absent]` with a sentence, and
`glob_problems`, `partition_problems`, `field_census`'s `measured == recorded`, and `YearReadiness`'s
`forms_expected.len() != forms_bundled` all still pass. What actually reds is outside the record:
`the_declared_statuses_are_the_measured_ones` pins TY2024's absent set to exactly `{f1040s1a}` (2024 only, by hand),
plus `LineSet::ALL.len() == 37`, `BUNDLED.len() >= 37` and the `UNCENSUSED` register. TY2017's five and TY2025's fifteen
have no per-year count pin. That is not a defect — a declaration cannot audit itself — but the module doc should say it,
because "expected == on disk, both directions" reads as a stronger guarantee than it is.
**Minimal change:** one sentence in the module doc naming the 37-count as the anti-shrink pin, or an expected-count pin
per year (`2017 => 5, 2024 => 17, 2025 => 15`) in `tests/year_record.rs`.

### R7 — MINOR — the census's liveness line now reads a declaration where it used to measure, and its "registered" check became a tautology

**Where:** `tests/field_census.rs:585-595` and `:610-620`.
(a) `complete_years` was `if measured.is_empty()` (every emittable form present) and is now
`if record.status == YearStatus::Filable`, so `assert!(!complete_years.is_empty(), "…there is no year the packet can
actually be filled for")` is satisfied by a self-declaration. The change was **necessary** (no year can bundle all 18 —
`f1040s1a` is TY2025+), and TY2024's absences are pinned in `tests/year_record.rs`; but the pin is in another crate's
test file and names 2024 by hand, so a future `filable` year with twelve absences would pass this gate. (b) `registered`
is now `years.filter(|y| YearRecord::for_year(*y).is_some())`, which cannot fail — `build.rs` panics on a year directory
without a `YEAR.toml`. Harmless (that build error was observed red, and is the stronger kill), but the assertion no
longer asserts anything.
**Minimal change:** for a `filable` year assert its `measured` absences against a structural rule (TY2025+ schedules
and `periodic` forms only), and say in a comment that the registration kill now lives in `build.rs`.

### R8 — MINOR — `FiledPacket`'s "only constructor" and "removing the sort is a compile error" are both weaker than stated

**Where:** `packet.rs:52-56` (`pub struct FiledPacket { pub forms, pub statements }`, `#[derive(… Default)]`), `:297-305`,
and `b58dee9d`'s Q2 paragraph.
Nothing outside `packet.rs` constructs one today (grep finds only the `pub use` at `lib.rs:454`), but `FiledPacket {
forms, statements }` and `FiledPacket::default()` + `push` are legal from any crate, in-workspace or downstream. And
removing `sort_by_attachment_sequence` from `stapled` **compiles** — it leaves an unused `mut` (a warning; the crate
sets no `deny(warnings)`). What actually reds is `the_only_packet_constructor_staples_in_sequence_order`
(`packet.rs:390-401`), a test — which is adequate, and is the honest claim.
**Minimal change:** `#[non_exhaustive]` on the struct (blocks the external literal, keeps the fields readable) or private
fields with accessors; and correct the two sentences to "held by test", per *"a guarantee without a test that reds when
it is removed does not exist"* — the test exists, the compile-error claim does not.

### R9 — MINOR — TY2017's `oracles.taxcalc = "none"` states an impossibility where the truth is "not wired"

**Where:** `forms/2017/YEAR.toml:37`. `.venv/bin/python`: taxcalc 6.8.2, `Policy.JSON_START_YEAR = 2013`,
`LAST_KNOWN_YEAR = 2026` — TY2017 is inside its range. The field is documented as *"which independent engines can
witness this year's figures"* / *"Tax-Calculator version floor"*, so `"none"` reads as *no second witness is possible*,
when what is true is *no harness path exists for the TY2017 slice*. That is exactly the distinction §G-6's witness
census turns on. (The `ots` entry for 2017 gets this right: it states the reason.)
**Minimal change:** `taxcalc = "none wired — 6.8.2 covers TY2017, but no slice harness path exists (strategy review S9)"`.

### R10 — MINOR — TY2025's `tables` cites Pub. L. 119-21 for figures this build does not carry

**Where:** `forms/2025/YEAR.toml:9`. `tax_tables.rs:30-34` states plainly that OBBBA *"did **not** change the **TY2025**
bracket thresholds or the §1(h) breakpoints"* and that *"the TY2025 indexed values are exactly Rev. Proc. 2024-40"*;
and the same record line says `FullReturnParams TY2025 NOT bundled`. So "Pub. L. 119-21 for the OBBBA figures" is a
citation of record for nothing in this build. Harmless today; the risk is a later reader taking it as evidence that the
OBBBA figures *are* in (std. deduction $15,750/$23,625, Schedule 1-A) when the year is deliberately paused.
**Minimal change:** move it into the parenthetical: *"…NOT bundled; when they are, Pub. L. 119-21 is the authority for
the OBBBA figures."*

### R11 — NIT — `YearReadiness`'s `Slice`/`Preparing` arm never checks `table`

`year_readiness.rs:87-94` checks only "params bundled for a non-filable year". A `slice` year — which by definition
computes the crypto slice, and therefore needs a `TaxTable` — would declare `slice` with no table and pass. TY2017 has
one (`tax_tables.rs:75`), so nothing is wrong today, and design §6's B1 list only names the `filable` kills, so the
implementation is faithful to the design. One line: check `self.table` for `Slice` as well.

### R12 — NIT — `BundledTaxTables`' own doc omits the year it registers first

`tax_tables.rs:60-61`: *"Currently contains **TY2024**, **TY2025**, and **TY2026**"* — while `load()` at `:75` inserts
2017 before all three. Pre-existing, found while verifying that `status = "slice"` has a table behind it.

## What is right, recorded so a later round does not re-derive it

- **The step-4 design amendment on `TRANSITION_DATE` is correct.** `conventions.rs:17` — *"The per-wallet basis snapshot
  date (§7.4)"*, `2025-01-01`; `defensive/era.rs:23-29` — *"the pooling cutover … no bucket STRADDLES"*; `era.rs:65` —
  *"a pre-2025 declare forfeits **Rev. Proc. 2024-28** safe-harbor eligibility"*; `project/transition.rs:3` — *"an
  effective Rev. Proc. 2024-28 safe-harbor"*. Measured 89 references across 20 files (the amendment says "~90"). It is a
  one-time regulatory date, not a per-year fact, and keeping it in `conventions` while **listing the omission** in §9 is
  the right call.
- **The `return_due` bridge is honest.** `resolve.rs:1668` (`let due = TY2025_RETURN_DUE;`) is the single core reader,
  in a crate that cannot see `btctax-forms`; the test holds two independently-written literals equal and reds if either
  moves. Its limit is stated in the kill table above.
- **The step-5 `[census]` and `map ⊆ PDF` claims are TRUE for all eight.** The `UNCENSUSED` register names exactly five
  TY2025 maps without a census and none is among the eight; `map_pdf_conformance` derives its list from the filesystem
  across every year.
- **`fill_form_8959` now asks `lines.must_file()` before `Form8959Map::for_year(year)`** (`lib.rs:198-205`) — the fold's
  Q2 claim is exactly what the code does.
- **`default_year()` changes no behaviour today**: `bundled_years()` is `[2017, 2024, 2025]`, so it returns the same
  `2025` the literals held, and in both TUIs it is overwritten on a successful open (R4). Its forward risk is that it is
  the newest **bundled** year, not the newest **filable** one — when `forms/2026/` lands as `preparing`, a fresh surface
  will open on a year with no params, and the type that could answer "newest filable" (`YearReadiness`) is not consulted.

## The next build step may proceed?

**NO** — not as "the eight are verified". Fold R1 and R5 (one-line record edits) and R4 (one token), then fix R2's
parser and re-run the label join: if `f1040sa/2025`'s nineteen bindings land on their printed labels, step 5's
justification stands and step 6 is clear. R3 closes with the `year_readiness.rs` work already in the working tree.
