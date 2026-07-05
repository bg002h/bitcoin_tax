# R0 — SPEC review, round 1: export-snapshot unresolved-Hard-blocker summary

- **Artifact:** `design/SPEC_export_blocker_summary.md`
- **Baseline:** branch `feat/export-blocker-summary` @ `34e64e3` (main == `e9a3690`).
- **Reviewer role:** independent architect. Read-only; no implementation.
- **Bar:** 0 Critical / 0 Important.

## Verdict: **0 Critical / 4 Important / 4 Minor / 2 Nit — NOT GREEN**

The mechanism is *implementable* and its runtime behavior is *correct* (files still
written, exit 0, clean ledgers unchanged, Hard-only counting). But four Important items
must fold before the plan/implementation gate: (I1) the `not_computable` field and its
`compute_tax_year` call are redundant and rest on a rationale that describes an
impossible state; (I2) the no-`--tax-year` warning understates the risk on the most
dangerous path; (I3) the stderr KATs + the ★ fault-inject are only observable at the
**binary** level, not from the library call the spec's other KATs use; (I4) the man page
is **generated**, so the named lockstep edit surface is wrong.

---

## Verified facts (the ones the prompt asked me to nail down)

**Exact `compute_tax_year` signature — `crates/btctax-core/src/tax/compute.rs:228-234`:**
```rust
pub fn compute_tax_year(
    events: &[LedgerEvent],
    state:  &LedgerState,
    year:   i32,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
) -> TaxOutcome
```
- Returns `TaxOutcome::NotComputable(Blocker{kind: TaxYearNotComputable, …})` when a Hard
  blocker gates the year (compute.rs:242-256), OR `TaxProfileMissing` (compute.rs:266-272),
  OR `TaxTableMissing` (compute.rs:258-264).
- `events` is **not consulted** — `let _ = events;` (compute.rs:235); every existing test
  passes `&[]`. So `export_snapshot` (which has no `events` slice in scope) can pass `&[]`.

**Availability in `export_snapshot` (`crates/btctax-cli/src/cmd/admin.rs:50-98`):** it already
has `state` (admin.rs:61) and, **inside the `Some(y)` arm only**, `tables =
BundledTaxTables::load()` (admin.rs:72) and `session.tax_profile(y)?` (admin.rs:73) — exactly
the two args `compute_tax_year` needs. Since `not_computable` is `Some` only when `tax_year ==
Some(y)`, the call is feasible in the arm where profile+tables already exist. So the spec's
`not_computable` computation is real — but see I1 for why it should not exist at all.

**The Hard-gate is projection-wide — `first_hard_blocker` (compute.rs:441-450):**
`state.blockers.iter().find(|b| b.kind.severity() == Severity::Hard)`. Its predicate is
**identical** to the spec's `unresolved_hard = count(severity()==Hard)` — good, consistent.
Crucially, "ANY unresolved Hard blocker ANYWHERE gates EVERY year" (compute.rs:237-241). This
invariant drives I1 and I2.

**`severity()` set — `state.rs:74-97`:** `FmvMissing … TaxYearNotComputable | TaxProfileMissing
| TaxTableMissing => Hard`; `PseudoReconcileActive` and the other notes are `Advisory`
(state.rs:90-95). `state.blockers: Vec<Blocker>` at state.rs:257; `Blocker.kind` at
state.rs:100-104. `TaxYearNotComputable` is a *synthesized outcome kind*, never
`add_blocker`'d into `state.blockers`, so `unresolved_hard` counts only the real underlying
blockers (FmvMissing etc.) — no double-count. Confirmed.

**Blast radius — `PathBuf → ExportReport` (prompt Q2):** the CLI function
`cmd::admin::export_snapshot` has **exactly one non-test caller: main.rs:325** (confirmed).
Nothing *silently* breaks — a return-type change turns every value-consuming site into a
**compile error**, never a silent wrong result. Enumerated call sites:

| Site | Use | Needs change? |
|---|---|---|
| `main.rs:325-326` | `let p = …?;` then `p.display()` | **yes → `report.path.display()`** |
| `tests/export.rs:26-27` | `let sqlite = …unwrap();` then `sqlite.exists()` | **yes → `.path`** |
| `tests/pseudo_reconcile_cli.rs:226-234` | `let sqlite = …expect(); sqlite.exists()` | **yes → `.path`** |
| `tests/pseudo_reconcile_cli.rs:158, 174` | `let ok = …; assert!(ok.is_ok()); "{ok:?}"` | needs `ExportReport: Debug` (see M3) |
| `tests/export.rs:134,143,267,388,539,582,639` | bare `…unwrap();` (value dropped) | no (compiles as-is) |
| `tests/tax_report.rs:392,962,1051` | bare `…unwrap();` (value dropped) | no |
| `tests/pseudo_reconcile_cli.rs:146,257,283` | `…unwrap_err()` (Err type unchanged) | no |
| `tests/pseudo_reconcile_cli.rs:311,318` | `…expect(…)` value dropped | no |

Net: **3 sites** rewire to `.path` (incl. main.rs), **2 sites** need `Debug` on the struct,
the rest compile untouched. See M2 for the store-function look-alike that must NOT be touched.

**Pseudo-active + Hard blockers (prompt Q3, ordering):** no issue. The attest gate
(`require_attestation`, admin.rs:62-64) runs **first, before any byte is written and before the
report is built**; if attestation is missing/wrong it returns `Err` and `?` in main.rs (line
325) short-circuits — the warning is never reached. If attestation is correct (or the ledger
isn't pseudo-active), files are written and the report's warning fires. `PseudoReconcileActive`
is Advisory (state.rs:71,95) so pseudo mode never inflates `unresolved_hard`. Both mechanisms
coexisting (attested pseudo draft that still carries a real Hard blocker → warn) is sensible.
**Affirmed, no finding.**

**Happy path (prompt Q4):** clean ledger → `unresolved_hard == 0` → no `eprintln!` → stdout is
byte-identical (`"Exported … + CSVs to …"`, main.rs:326). stderr is the right stream (the data
lives in files; the human-facing success line is stdout; a data-quality warning must not
corrupt a parsed/redirected stdout). Exit 0 is correct and user-approved. **Affirmed.** (One
caveat filed as N1.)

---

## Important

### I1 — `not_computable` (and its `compute_tax_year` call) is redundant; its rationale describes an impossible state
Because **any** Hard blocker gates **every** year (compute.rs:237-241; first_hard_blocker
compute.rs:441-450), then within the warning regime (`unresolved_hard > 0`):
`compute_tax_year(y,…)` **always** returns `NotComputable` ⇒ `not_computable == tax_year` for
every requested year. So `not_computable` carries no information the arm doesn't already have
from `tax_year.is_some()`.

Consequences of keeping it as specified (spec lines 24-29, 34):
- The spec's message-B rationale — *"Hard blockers present but the requested year still
  computes"* (line 34) — is **unreachable**: you cannot have `unresolved_hard > 0` and a
  computable requested year simultaneously. This wrong rationale will mislead the implementer
  and the KAT `export_not_computable_only_when_year_requested_and_blocked` (line 48).
- It drags a needless `compute_tax_year` call — and a `profile`/`tables` dependency — into the
  export path (and, if computed unconditionally, onto the clean happy path too).
- `compute_tax_year` also returns `NotComputable` for `TaxProfileMissing`/`TaxTableMissing`
  with **zero** Hard blockers (compute.rs:258-272); the spec's definition ("`compute_tax_year`
  is NotComputable") is therefore broader than "a Hard blocker gates the year." It's currently
  harmless only because `not_computable` is read exclusively under `unresolved_hard > 0` — a
  fragile coupling.

**Fix:** delete the `not_computable` field and the `compute_tax_year` call. Drive the message
purely from `unresolved_hard > 0` and `tax_year.is_some()`:
```
if unresolved_hard > 0 {
    if let Some(y) = tax_year { /* message A, naming y */ }
    else                     { /* message B */ }
}
```
`ExportReport` shrinks to `{ path, unresolved_hard }`. If the author prefers to keep a
`not_computable: Option<i32>` purely as an API convenience, then (a) correct the rationale
(delete "the requested year still computes"), (b) define it as `tax_year.filter(|_|
unresolved_hard > 0)` — no `compute_tax_year` call — and (c) keep passing `&[]` for `events`.

### I2 — the no-`--tax-year` (full-export) warning understates the risk (prompt Q3, answered)
Message B (spec line 35) is `"⚠ {n} unresolved Hard blocker(s) remain; some figures may be
incomplete. Run btctax verify."` For a **full export** (`tax_year == None`) the command writes
the all-years projection CSVs (cli.rs:94-100) and, under any Hard blocker, **every** year is
`NotComputable` — so every 8949/Schedule D that lands is empty/partial. "some figures may be
incomplete" is materially weaker than the year-scoped message's "the exported Form 8949 /
Schedule D are INFORMATIONAL, not final" — yet the danger is identical (indeed broader: all
years, not one). This is the exact silent-empty-forms failure mode the spec exists to fix (spec
lines 4-6), only re-clothed as a soft "incomplete."

On the sub-question "should it name the affected years?" — **no.** Because any Hard blocker
taints *all* years, enumerating years adds cost without adding truth; the accurate, simpler
statement is that **all** exported forms are informational. 

**Fix:** put the load-bearing "the exported forms are INFORMATIONAL, not final — run `btctax
verify`" clause in **both** messages; the year-scoped variant additionally names the year as
NOT COMPUTABLE. E.g. message B → `"⚠ {n} unresolved Hard blocker(s) remain — every affected
year is NOT COMPUTABLE; the exported forms are INFORMATIONAL, not final. Run btctax verify."`
Update KAT `export_clean_ledger_no_warning` is unaffected; add coverage that the None-path
message also contains "INFORMATIONAL"/"NOT COMPUTABLE".

### I3 — the stderr KATs and the ★ fault-inject are only observable at the BINARY level
The `eprintln!` lives in the **main.rs arm** (spec lines 30-36), by design (keep the library
I/O-free). Therefore `cmd::admin::export_snapshot(...)` — the function every current export test
calls (export.rs, tax_report.rs, pseudo_reconcile_cli.rs) — **returns the struct and prints
nothing.** A library-level test can assert `report.unresolved_hard` / `report.path` but can
**never** observe `"NOT COMPUTABLE"` on stderr, and cannot detect the ★ fault (dropping the
`unresolved_hard > 0` eprintln, spec lines 50-51), because that code isn't exercised by a
library call. As written, the ★ fault-inject and the "stderr contains …" KATs
(`export_with_hard_blockers_warns_and_still_writes`, lines 41-43) are **not implementable via
the library harness the spec's other KATs imply.**

The infra to do it right already exists: drive the built binary via
`env!("CARGO_BIN_EXE_btctax")` + `std::process::Command`, capturing stderr and the exit status
— exactly the pattern in `tests/fr9_exit_code.rs:53` and `tests/tax_report.rs:728`.

**Fix:** split the test plan explicitly. (a) **Library KATs** assert `ExportReport` fields
(`unresolved_hard` counts Hard only; `path` correct) — `export_report_counts_only_hard`, etc.
(b) **A binary integration KAT** runs `btctax export-snapshot --out … --tax-year Y` against a
vault with an unresolved Hard blocker (reuse the fr9_exit_code.rs Hard-blocker fixture pattern),
asserts stderr contains "NOT COMPUTABLE" + the count + "verify", asserts the files exist, and
asserts exit 0. The ★ fault-inject is "delete the main.rs eprintln ⇒ this binary KAT goes RED."

### I4 — lockstep edit surface is wrong: the man page is GENERATED, not hand-authored
Spec §Scope (lines 56-57) says *"the `btctax-export-snapshot.1` man page + `--help` doc-comment
gain a one-line note"*, implying two hand-edits. In fact `docs/man/btctax-export-snapshot.1` is
**generated** from the clap doc-comment via clap_mangen (`crates/xtask/src/docs.rs`, header
lines 1-11: "The file-format long-help authored on the subcommand args … rides along
automatically — zero drift with `--help`"), and `gen_docs_is_deterministic`
(docs.rs:342-357) **asserts the committed `docs/man/*.1` match a fresh generation** ("committed
docs/man/{name} is STALE; regenerate with `cargo run -p xtask -- docs`"). So:
- Hand-editing the `.1` **without** touching the source will fail the drift test.
- Editing the doc-comment **without** regenerating will also fail it.

The real single-source edit surface is the `ExportSnapshot` clap doc-comment in
`crates/btctax-cli/src/cli.rs:92` (the `/// FR10: export decrypted SQLite + CSV …` variant doc,
and/or the `--out` long-help at cli.rs:94-104), **followed by** `cargo run -p xtask -- docs` to
regenerate `docs/man/btctax-export-snapshot.1`.

**Fix:** restate lockstep as "add the warn-not-refuse note to the `ExportSnapshot` doc-comment
in cli.rs:92, then regenerate the man page (`cargo run -p xtask -- docs`); `--help` and the `.1`
update together from that one source." Verify the existing help test
`help_documents_export_snapshot_format` (cli.rs:750-757) still passes — it's a `.contains(...)`
assertion on the CSV header, so an added sentence won't break it.

---

## Minor

### M1 — imprecise citations for `has_hard_blockers()` and severity lines
Spec line 20 cites *"`has_hard_blockers()` exists (inspect.rs:22)."* inspect.rs:22 is only a
**doc-comment mention**; the actual method is `VerifyReport::has_hard_blockers` at
`crates/btctax-cli/src/render.rs:426-431` (`!self.hard.is_empty()`) — a method on the *verify
report*, **not** on `LedgerState`, and it reads `report.hard`, not `state.blockers`. The spec's
own mechanism does not use it (it counts `state.blockers` directly), so this is a citation
hygiene fix, not a design flaw. Correct the reference (or drop it). (`TaxYearNotComputable` Hard
is at state.rs:49 decl / :87 match — spec's :49 is fine.)

### M2 — two distinct `export_snapshot` functions; change only the CLI one
`grep 'export_snapshot('` hits **two** functions: the store method
`btctax-store/src/vault.rs:263` (`-> Result<PathBuf, StoreError>`, called at admin.rs:65 and
`tests/integration.rs:282` via `.exists()`) and the CLI wrapper
`btctax-cli/src/cmd/admin.rs:50`. Only the **CLI** one changes to `ExportReport`; the store
method and `integration.rs:282` must stay `PathBuf`. The spec should state this explicitly so
the implementer doesn't over-reach.

### M3 — `ExportReport` must derive `Debug`
`tests/pseudo_reconcile_cli.rs:158,174` format the call result with `"{ok:?}"` where `ok:
Result<ExportReport, _>`, so `ExportReport` must be `Debug` to keep those tests compiling. The
spec's struct sketch (lines 24-25) lists no derives. Specify `#[derive(Debug, Clone)]` (Clone is
cheap insurance; PathBuf + usize + Option<i32> are all trivially Debug/Clone).

### M4 — name the `events` argument if `not_computable` survives I1
If the author keeps a real `compute_tax_year` call (rejecting I1's simplification), the spec
must state that `export_snapshot` has no `events` slice in scope and passes `&[]` — sound,
because `compute_tax_year` ignores `events` (compute.rs:235). Moot if I1 is adopted.

---

## Nit

### N1 — exit 0 hides "not final" from scripts (accepted, but note the escape hatch)
Warn-don't-refuse + exit 0 is explicitly user-approved and consistent with the project's
conservative-but-non-gating policy. But a CI/script that pipes `export-snapshot` and checks
`$?` will not detect that the forms are non-final (contrast `verify`, which exits 1 on
`has_hard_blockers`, main.rs:89). Consider one doc sentence directing automation to gate on
`btctax verify` rather than on the export exit code. Not blocking.

### N2 — make the Advisory fixture faithful in `export_report_counts_only_hard`
Ensure that KAT builds a ledger whose only blocker is a genuine **Advisory**
(e.g. `SelfTransferInboundZeroBasis` or `QualifiedAppraisalNote`, state.rs:59/65) so it truly
exercises "Advisory present, Hard count 0 → no warning," not merely an empty blocker list.

---

## What must change before GREEN
Fold I1–I4 (I3 and I4 are the implementability blockers; I1 and I2 are correctness/clarity of
the signal and the message). Re-review after the fold, including a re-check that the message
wording, the split library/binary test plan, and the cli.rs-doc-comment lockstep are all
reflected. M1–M4 and N1–N2 are cheap and should ride along.
