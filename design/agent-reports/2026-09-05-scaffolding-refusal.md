# The refusal surface — what a user sees when they ask for a year btctax is not ready for

**Lens:** refusal surface. **Date:** 2026-09-05. **Method:** ran the prebuilt `./target/debug/btctax`
against a scratch vault; every exit code and message below is pasted, not described.

---

## Headline

**It is not uniform, and the non-uniformity is not a message problem — btctax has a refusal vocabulary
for "cannot compute" and none at all for "computes but cannot be filed."** Four different year-sets
govern four entry points, a filer can reach every one of them, and one of the four (`export-snapshot`)
has no year gate whatever: `--tax-year 2099` writes a file called `form8949.csv` and exits 0,
**byte-identical** to the TY2026 one. The worst case is not the silence, though — it is
`income import --year 2026`, which exits 0, destroys the only working number the year had, and then
tells the filer to delete their W-2s.

---

## 1. The measured matrix

Scratch vault, empty ledger, `tax-profile` set for every year shown. Exit codes are the process's, captured
individually (not through a pipe).

```
TY2017  report=0  export-irs-pdf=0  export-snapshot=0
TY2024  report=0  export-irs-pdf=0  export-snapshot=0
TY2025  report=0  export-irs-pdf=0  export-snapshot=0
TY2026  report=0  export-irs-pdf=2  export-snapshot=0
TY2027  report=1  export-irs-pdf=2  export-snapshot=0
TY2099  report=1  export-irs-pdf=2  export-snapshot=0
```

Four year-sets are in play, and no surface names more than one of them:

| level | gate | set | where |
|---|---|---|---|
| L0 | `BundledTaxTables::load()` | **{2017, 2024, 2025, 2026}** | `btctax-adapters/src/tax_tables.rs:76-81` |
| L1 | `btctax_forms::SUPPORTED_YEARS` (crypto-slice PDFs) | **{2017, 2024, 2025}** | `btctax-forms/src/lib.rs:68` |
| L2 | form maps actually on disk | 2017: **5**, 2024: **17**, 2025: **15**, 2026: **absent** | `btctax-forms/forms/*/` |
| L3 | `full_return_for` (the whole 1040) | **{2024}** | `btctax-adapters/src/tax_tables.rs:99-104` |
| — | `export-snapshot --tax-year` | **unbounded** | `btctax-cli/src/cmd/admin.rs:129` |

**The asymmetry, exactly.** `report --tax-year 2026` exits 0 and prints the complete crypto-delta block —
net ST/LT, ordinary-rate tax, LTCG tax, NIIT, §1211 deduction, carryforward out, marginal rates. Those
numbers are *real*: they come from the KAT-pinned Rev. Proc. 2025-32 table at L0. What is absent is
everything at L1–L3 — no Form 8949, no Schedule D, no 1040, no absolute return — and **nothing in the
output says so.** Compare TY2025's output and TY2026's: structurally identical, one filable, one not.

`report --tax-year 2027` by contrast:

```
Federal tax attributable to crypto — tax year 2027
  NOT COMPUTABLE [TaxTableMissing]: no bundled tax table for 2027
```

So the *only* readiness fact `report` can express is L0. There is no `BlockerKind` for the other three:
`btctax-core/src/state.rs:86,88` carries exactly `TaxProfileMissing` and `TaxTableMissing`, and
`compute.rs:262-275` is the only place either is raised.

---

## 2. Findings

### F1 — CRITICAL. `income import` on an unprepared year commits, destroys the year's number, and prescribes data loss.

Measured, in order, on TY2026:

```
$ btctax income import --year 2026 --file ri2026.toml
Imported full-return inputs for tax year 2026.          EXIT=0

$ btctax report --tax-year 2026
error: usage: tax year 2026 has full-return inputs, but full-return computation is not supported for
2026 in this version (v1 supports TY2024); run `income clear --year 2026` to remove them and use a raw
`tax-profile`                                            EXIT=2
```

The same command exited **0 with a full figure block** one step earlier. Importing a W-2 took the year
from *answerable* to *hard error*.

Three things are wrong here and they compound:

1. **The CLI bypasses a gate the TUI enforces.** `input_form_store::commit` refuses to write a committed
   row when the year has no tables — `btctax-cli/src/input_form_store.rs:309-310`:
   ```rust
   let (Some(table), Some(params)) = (table, params) else {
       return Ok(CommitOutcome::NoTables); // I-11: no tables for this year → write nothing
   };
   ```
   and its own doc comment at `:295` states the invariant verbatim: *"writes nothing, so a refused commit
   **never poisons the year at `resolve`** and the draft remains for the user to fix."*
   `cmd/tax.rs:199` — the `income import` handler — is `return_inputs::set(s.conn(), year, &ri)?;` with no
   table or params lookup anywhere in the function. It performs precisely the poisoning the invariant names.

2. **The two surfaces give opposite answers to the same act.** `btctax-tui-edit/src/main.rs:1359-1388`
   handles `NoTables` by flushing to a draft and reporting:
   `"{year} has no full-return tables yet (v1: TY2024) — inputs SAVED as a draft; finalize when tables
   publish."` The comment above it records the owner's ruling (2026-07-19): *"people author returns all
   year, before the IRS publishes that year's tables — the draft is how that work is stored."* The CLI's
   answer to the identical act is a broken year.

3. **The prescribed remedy deletes the filer's work.** `income clear` is `return_inputs::delete` with no
   draft fallback (`cmd/tax.rs:346-354`). The message tells a filer whose only fault was authoring their
   2026 return in January 2027 to throw away every W-2, 1099 and dependent they typed in.

Under the owner's TY2026-first ruling, authoring-before-forms-publish stops being an edge case and becomes
the **normal** entry path. This is the single highest-cost defect on the refusal surface.

### F2 — IMPORTANT. `export-snapshot --tax-year` has no year gate at all, and its artifacts do not record the year.

```
$ btctax export-snapshot --out snap_2099 --tax-year 2099
Exported .../snapshot.sqlite + CSVs to .../snap_2099     EXIT=0
$ ls snap_2099
disposals.csv form8283.csv form8949.csv income.csv lots.csv removals.csv schedule_d.csv snapshot.sqlite

$ diff snap_2026/form8949.csv snap_2099/form8949.csv && echo IDENTICAL
IDENTICAL
```

`export_snapshot` (`btctax-cli/src/cmd/admin.rs:129`) has two refuse-before-bytes gates — the BG-D8 Form
8275 completeness gate and the pseudo attestation gate — and no year gate. `export-irs-pdf` acquired one
(`admin.rs:678`, `if !btctax_forms::SUPPORTED_YEARS.contains(&tax_year)`); the CSV path never did.

This matters more than it looks, because the CSV path is the **preparer handoff**. For TY2026 today it is
the only year-scoped "Form 8949" btctax will produce, and a preparer receiving `form8949.csv` has no way
to tell which tax year it is for — the file carries no year column and no header. Under this repo's own
rule (*two blanks look identical on the printed page and are not the same thing*), two **years** are here
identical in the artifact.

### F3 — IMPORTANT. `report` has no way to say "this year computes but cannot be filed," so it says nothing.

`BlockerKind` (`btctax-core/src/state.rs:86,88`) has `TaxProfileMissing` and `TaxTableMissing`. Both are
`Severity::Hard` (`state.rs:143-144`) and both mean *no number*. There is no non-Hard reason meaning
*a number, but no form* — so L1/L2/L3 readiness has nowhere to be rendered, on any surface. Confirmed
across the whole poisoned-TY2026 surface:

```
verify                     → "Hard blockers (gate tax computation): 0"   (says nothing about 2026)
income show --year 2026    → prints the whole return, exit 0
export-snapshot 2026       → writes form8949.csv + schedule_d.csv, exit 0
report --tax-year 2026     → exit 2
```

`btctax --help` lists no command that reports year readiness, and there is no `--list-years`.

### F4 — IMPORTANT. Every user-facing supported-year list is a literal decoupled from the constant it describes, and no test couples them.

Four independent statements of the same fact, none derived:

| text | site | says |
|---|---|---|
| `"unsupported tax year {0}: this build bundles IRS forms for 2017, 2024 and 2025 only"` | `btctax-forms/src/error.rs:11` | L1, hardcoded |
| `"this build bundles TY2017, TY2024 and TY2025; other years are refused"` | `btctax-cli/src/cli.rs:208` (clap help) | L1, hardcoded |
| `"...not supported for {year} in this version (v1 supports TY2024)..."` | `btctax-cli/src/resolve.rs:222` | L3, hardcoded |
| `"no full-return tables for {year} — carryover write-back needs a supported tax year (TY2024)"` | `btctax-cli/src/cmd/tax.rs:822` | L3, hardcoded |
| `"Tax year supported: TY2024 only"` / `"Any tax year other than TY2024"` | `btctax-cli/LIMITATIONS.md:3` and `:304` | L3 — **and already stale**, since the crypto slice files 2017/2024/2025 |

`SUPPORTED_YEARS` is defined at `btctax-forms/src/lib.rs:68`, **57 lines above** the message that
restates it by hand. Grepping the whole workspace for the message text returns exactly one hit — the
definition itself:

```
$ grep -rn "this build bundles IRS forms\|unsupported tax year" --include=*.rs .
btctax-forms/src/error.rs:11:    #[error("unsupported tax year {0}: this build bundles IRS forms for 2017, 2024 and 2025 only")]
```

**No test reds when `SUPPORTED_YEARS` gains 2026.** Under B1 (*seen-red-once*) there is no checker here
at all, only a sentence. The one-sentence answer to *"which test reds when this is wrong?"* is "none."

The repo already contains the right shape, one crate away:
`btctax-forms/tests/census.rs::the_newest_era_preset_reaches_the_newest_filable_tax_year` (`:419-434`)
couples `era.rs`'s hardcoded `2025-12-31` to `*SUPPORTED_YEARS.iter().max()` and reds the moment the
constant moves. Its doc comment says the reason out loud: *"Nothing in the type system couples the two…
This test IS the coupling."* That test exists for one literal and not for the four above.

### F5 — IMPORTANT. The TUI Forms tab renders "Form 8949 — 2026" with no year gate.

`btctax-tui/src/tabs/forms.rs:61-76,123` renders `Form 8949 — {year}` and Schedule D / 8283 tables for
whatever `app.selected_year` holds; `SUPPORTED_YEARS` appears nowhere in `btctax-tui/`. On a tab literally
named **Forms**, a year with no bundled forms shows a populated form. The TUI export
(`btctax-tui/src/export.rs:128,169`) likewise writes the year's CSVs with no gate — same class as F2.

### F6 — MINOR. Unbumped year literals in the TUI's own defaults.

`btctax-tui/src/app.rs:193` — `selected_year: 2025`; `btctax-tui/src/unlock.rs:229` —
`.max().unwrap_or(2025)` ("or 2025 when the ledger is empty"). A new user opening an empty vault in 2027
lands on TY2025. Neither is coupled to anything; both are hand-bumps a port would have to remember.

### F7 — MINOR. The year-acceptance surface at the input layer is unbounded.

```
$ btctax tax-profile --year 1899 --filing-status single ...
Tax profile for 1899 saved.                              EXIT=0
```

`report --tax-year 1899` then correctly refuses with `TaxTableMissing`, so no wrong number escapes — but
the vault accumulates rows for years no gate ever looked at, and the TUI enumerates its year list from
exactly that set (`unlock.rs:183`, `resolve_all_screened` over `tax_profile ∪ return_inputs`).

---

## 3. The design question — should a table-only year refuse earlier?

**No. Compute is correct; the silence is the defect. But `income import` should refuse, and does not.**

Argued from this repo's own doctrine, not from taste:

**(a) `report` output is not testimony, so refusing it protects nothing.** *An entry is testimony* applies
to a line on a signed return. `report --tax-year 2026` is decision support delivered to the filer, nothing
is signed, and the figure is backed by a KAT-pinned Rev. Proc. 2025-32 table. Refusing it would withhold
information btctax has independently validated and push the filer to estimate by hand — the same shape as
the `era.rs` finding, where *friction toward an untruthful answer was itself the defect*.

**(b) The missing readiness line is the doctrine's own defect class, exactly.** *"Blank because the inputs
say so"* vs *"blank because nothing ever populated it — never collected, never asked, never modelled."*
TY2026's report has no readiness line not because a filer's inputs made it empty, but because **no such
line exists on any surface**. That is the second row of the table, and the remedy the doctrine prescribes
is not refusal — it is a **determinate provenance** for readiness, carried as a value, on every
number-bearing surface.

**(c) The operator's ruling makes the pre-forms window the main path.** With TY2026 the first filed year
and TY2027 timing acceptable, "the tables are out, the forms are not" is where the product now *lives*,
not an edge. The TUI already ratified this (owner decision 2026-07-19, quoted at
`btctax-tui-edit/src/main.rs:1364-1366`). Refusing at compute would make btctax unusable in precisely the
window it now targets.

**(d) Where refusal *is* right, it is missing.** Writing a **committed** full-return row for a year with
no `FullReturnParams` is not authoring — it is the poisoning `input_form_store.rs:295` says must never
happen. `income import` should do what the TUI does: refuse the commit, keep the work as a draft, and
never take the year's crypto figure away. That is F1.

**So the shape of the fix is one type, not five messages.** A `YearReadiness` computed once from the four
sets (table / crypto-slice forms / full-return maps / full-return params), rendered on every
number-bearing surface, and *used to build* every refusal string — so the strings cannot go stale and
adding TY2026 is a data change. Per B1 it lands paired with a planted-defect test: add 2026 to one set
only, and a test reds.

---

## MECHANICAL — a machine can do these

1. **Couple every supported-year sentence to its constant.** Format `FormsError::UnsupportedYear`
   (`btctax-forms/src/error.rs:11`) from `SUPPORTED_YEARS`; build the clap `long_help` at
   `btctax-cli/src/cli.rs:208` from it; build `resolve.rs:222` and `cmd/tax.rs:822` from
   `BundledFullReturnTables`' key set. Ship each with the kill-test B1 requires, on the model of
   `census.rs:419`.
2. **Add the year gate `export-snapshot` never got** — mirror `admin.rs:678` into `export_snapshot`
   (`admin.rs:129`), refusing before any byte is written. Kill-test: `--tax-year 2099` must write zero files.
3. **Add a `tax_year` column (or header row) to `form8949.csv` / `schedule_d.csv`.** Kill-test: the
   TY2026 and TY2099 files must not compare equal.
4. **Move the `income import` write behind the `commit` gate.** `cmd/tax.rs:199` should route through
   `input_form_store::commit` (or replicate its `NoTables` arm), flushing to a draft instead of the
   committed row. Kill-test: `income import --year 2026` then `report --tax-year 2026` must still exit 0
   with a figure.
5. **Assert `SUPPORTED_YEARS` matches the maps on disk.** `map.rs`/`pdf.rs` are hand-listed
   `include_str!`/`include_bytes!` constants; a test can enumerate `forms/*/` and assert the year
   directories, the `for_year` arms and `SUPPORTED_YEARS` are the same set.
6. **Grep-and-fix the unbumped literals**: `btctax-tui/src/app.rs:193`, `btctax-tui/src/unlock.rs:229`,
   `btctax-cli/LIMITATIONS.md:3` and `:304` (the last is already wrong today).
7. **Enumerate `full_return_for`'s production call sites once** — `resolve.rs:264`, `session.rs:526,569`,
   `cmd/tax.rs:475,512,660,819`, `cmd/admin.rs:965`, `btctax-tui-edit/src/main.rs:1310`,
   `btctax-input-form/src/spec/mod.rs:531` — and check each renders readiness rather than swallowing `None`.

## HUMAN — someone must read a form, or decide

1. **Decide what a table-only year is allowed to emit.** Report figures: yes (argued above). A CSV named
   `form8949.csv`: unresolved — it is a filing artifact in everything but format. This is a product ruling,
   not a code question.
2. **Write the readiness sentence.** One line, on `report`, the TUI Tax and Forms tabs, and
   `export-snapshot`'s stderr, that distinguishes L0/L1/L2/L3 in a filer's words. Getting this wrong in the
   understating direction is worse than saying nothing, so it wants the operator's own phrasing.
3. **Rule on the L2 gap inside a "supported" year.** TY2025 is in `SUPPORTED_YEARS` with **15** maps against
   TY2024's **17** (missing `f1040s1`, `f8275`, `f8995a`; gained `f1040s1a`). A filer asking for a TY2025
   full return today is refused by L3 — but once TY2025 `FullReturnParams` land, L2's gap becomes reachable
   and only a human comparing the two form sets can say which absences are correct for the year and which
   are unported.
4. **Decide whether `income clear`'s destructiveness stays.** If `income import` keeps a draft (MECHANICAL
   4), the clear-and-retype advice disappears with it; if not, `income clear` needs a draft fallback before
   any message keeps recommending it.
5. **Compare the TY2026 draft forms for the L1 port.** Adding 2026 to `SUPPORTED_YEARS` requires the map +
   PDF + geometry-cluster arms that `f1040_clusters` and `se_clusters` now panic on
   (`btctax-forms/src/form1040.rs:45-54`, `schedule_se.rs:48-57`) — that panic is a human's cue to open the
   form, and is the correct behaviour, not a defect.
