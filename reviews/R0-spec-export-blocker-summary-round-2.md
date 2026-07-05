# R0 — SPEC review, round 2: export-snapshot unresolved-Hard-blocker summary

- **Artifact:** `design/SPEC_export_blocker_summary.md`
- **Baseline:** branch `feat/export-blocker-summary` @ `80a02f5` (main == `e9a3690`).
- **Reviewer role:** independent architect. Read-only; no implementation.
- **Bar:** 0 Critical / 0 Important.
- **Round-1 result folded here:** 0C / 4I / 4M / 2N (`reviews/R0-spec-export-blocker-summary-round-1.md`).

## Verdict: **0 Critical / 0 Important / 1 Minor / 1 Nit — R0-GREEN (cleared to implement)**

Every round-1 finding (I1–I4, M1–M4, N1–N2) is folded and each fold verifies against current
source. The mechanism is self-consistent, has no open questions, and the test-site rewiring
list is complete and correct. One Minor (user-facing copy imprecision on the no-`--tax-year`
path) and one Nit (the binary KAT must use `Command::output()`, not `.status()`) ride along —
neither blocks. Cleared to implement.

---

## Fold-by-fold verification (each confirmed against source)

### I1 — drop `not_computable` / `compute_tax_year`: **FOLDED, confirmed**
- `compute_tax_year` (compute.rs:228-234) checks `first_hard_blocker(state)` **first** at
  **compute.rs:242**, returning `NotComputable(TaxYearNotComputable)` **before** the table
  (compute.rs:258) and profile (compute.rs:266) checks. `first_hard_blocker`
  (**compute.rs:445-450**) is `state.blockers.iter().find(|b| b.kind.severity() ==
  Severity::Hard)`. So `first_hard_blocker(state).is_some()` ⟺ `unresolved_hard > 0` ⟺
  `compute_tax_year` returns `NotComputable` for *every* year. The per-year call therefore adds
  **zero** information over `unresolved_hard > 0`. The redundancy claim (spec lines 15-21) is
  **true**.
- The spec now derives the warning purely from `unresolved_hard > 0` + `tax_year.is_some()`
  (spec lines 21, 29-36, 73), and `ExportReport = { path: PathBuf, unresolved_hard: usize }`
  (spec lines 24-26) — **no `compute_tax_year` call, no profile/tables dependency**. This also
  eliminates the round-1 hazard that a `compute_tax_year`-based check would mis-fire on
  `TaxProfileMissing`/`TaxTableMissing` with 0 Hard blockers (those arms are only *reachable*
  after the Hard-gate short-circuit is passed).
- `unresolved_hard`'s predicate (spec line 26, `b.kind.severity() == Severity::Hard`) is
  **byte-identical** to `first_hard_blocker`'s predicate (compute.rs:449) — consistent, no
  double-count (`TaxYearNotComputable` is a synthesized outcome kind, never pushed into
  `state.blockers`; the real underlying blockers — `FmvMissing`, `UnknownBasisInbound`, … —
  are the Hard set at state.rs:77-89).
- **Residual-reference check:** `grep not_computable|compute_tax_year design/SPEC…` returns
  only explanatory lines (16, 18-19, 28, 73) that state the field/call are **DROPPED**. No
  surviving field usage anywhere. Confirmed.

### I2 — both messages informational: **FOLDED, confirmed (one Minor, below)**
- Message A (`--tax-year {y}`, spec lines 31-32) and Message B (no `--tax-year`, spec lines
  33-35) **both** now carry "…are INFORMATIONAL, not final. Run `btctax verify`". The
  load-bearing disclosure is present in both. Confirmed.
- Full-export-writes-all-years justification: with `tax_year == None`, `write_csv_exports`
  (render.rs:584-616+) writes the all-years projection CSVs (`lots.csv`, `disposals.csv`,
  `removals.csv`, `income.csv`) from `state.lots`/`state.disposals`/… (cli.rs:94-100
  doc-comment). Any Hard blocker gates every year (I1), so the entire all-years projection
  rests on an unsound basis foundation — the "every affected year is NOT COMPUTABLE … BROADER
  risk" framing (spec line 33-35) is **substantively justified**. See Minor M-A for the one
  wording nuance ("forms" vs projection CSVs).

### I3 — binary test split: **FOLDED, confirmed; sound + implementable**
- The `env!("CARGO_BIN_EXE_btctax")` + `std::process::Command` pattern exists exactly where
  cited: **fr9_exit_code.rs:53** (`let bin = env!("CARGO_BIN_EXE_btctax");` then
  `Command::new(bin)…`) and **tax_report.rs:727-728** (same). `CARGO_BIN_EXE_btctax` is the
  correct bin name (Cargo sets `CARGO_BIN_EXE_<name>`; the binary is `btctax`).
- The `eprintln!` lives in the **main.rs arm** (main.rs:325-326), so the stderr assertion and
  the ★ fault-inject are only observable at the binary level — the spec's split is correct:
  library KATs assert struct fields (`unresolved_hard`, `path`) via
  `cmd::admin::export_snapshot(...)` directly (spec lines 47-49); binary KATs drive the built
  binary and capture stderr/exit (spec lines 50-55); the ★ fault-inject targets the binary KAT
  (spec lines 56-57). Sound and implementable. (See Nit N-A: the cited sites use
  `.status()`+`Stdio::null()`, which drops stderr; the new stderr-asserting KAT must use
  `Command::output()` — trivial, obvious variation.)

### I4 — docs (generated man page): **FOLDED, confirmed**
- `docs/man/btctax-export-snapshot.1` **exists and is generated** (clap_mangen via xtask).
  `gen_docs_is_deterministic` at **docs.rs:342-357** asserts the committed `docs/man/*.1` match
  a fresh generation ("committed docs/man/{name} is STALE; regenerate with `cargo run -p xtask
  -- docs`"). So hand-editing the `.1`, or editing the doc-comment without regenerating, both
  fail the drift test — exactly as the spec states (lines 60-64, 77).
- The `ExportSnapshot` variant doc-comment is at **cli.rs:92** (`/// FR10: export decrypted
  SQLite + CSV (the NFR2 plaintext exception).`). Adding the "warns (does not refuse)" note
  there → `--help` and the `.1` update together via `cargo run -p xtask -- docs`. Correct
  single-source edit surface.
- The `.contains` help test won't break: `help_documents_export_snapshot_format`
  (**cli.rs:751**; spec cites cli.rs:750 = its `#[test]` line, fine) asserts
  `h.contains("event,kind,removed_at,lot,sat,basis,fmv_at_transfer")` — an added sentence
  leaves that substring intact. Confirmed.

### M2 / M3 / N2 — store-vs-CLI split, derives, faithful fixture: **FOLDED, confirmed**
- **M2 (change only the CLI fn):** two `export_snapshot` functions exist. The store method
  **vault.rs:263** (`pub fn export_snapshot(&self, out_dir: &Path) -> Result<PathBuf,
  StoreError>`) stays `PathBuf`; it is called at admin.rs:65 (`session.vault().export_snapshot`)
  and integration.rs:282 (`v.export_snapshot`) — both untouched. Only the CLI wrapper
  **admin.rs:50** (`Result<PathBuf, CliError>` → `ExportReport`) changes. Spec states this
  explicitly (lines 24, 44, 80). `state` is in scope at admin.rs:61, so `unresolved_hard` is
  computable inside the wrapper. Confirmed.
- **M3 (`ExportReport: Debug + Clone`):** spec line 26 specifies `#[derive(Debug, Clone)]`.
  Required by the `{ok:?}` sites at **pseudo_reconcile_cli.rs:167 and 177** (the `assert!(…,
  "…{ok:?}")` where `ok: Result<ExportReport, CliError>`; `Result: Debug` needs `ExportReport:
  Debug`). Confirmed. (`Blocker` at state.rs:99 already derives Debug/Clone; `PathBuf`+`usize`
  are trivially both.)
- **N2 (real Advisory fixture):** the counts-only-Hard KAT uses `SelfTransferInboundZeroBasis`
  (spec lines 48-49), which is a genuine **Advisory** at **state.rs:65 / :94** (`severity()`
  Advisory arm). Truly exercises "Advisory present, Hard count 0 → no warning," not an empty
  blocker list. Confirmed.

### Exact test-site rewiring list — re-grepped, **COMPLETE**
`grep -rn 'export_snapshot(' crates/` enumerated every caller; each falls into the spec's
categorical buckets (spec lines 41-44) with none unaccounted-for:

| Site | Current use | Change |
|---|---|---|
| main.rs:325-326 | `let p = …?;` → `p.display()` | **→ `report.path.display()`** (+ add the arm's `eprintln!`) |
| export.rs:26-27 | `let sqlite = …unwrap();` → `.exists()` | **→ `.path`** |
| pseudo_reconcile_cli.rs:226-235 | `let sqlite = …expect();` → `.exists()` | **→ `.path`** |
| pseudo_reconcile_cli.rs:158-168, 174-178 | `let ok = …;` → `"{ok:?}"` in assert msg | **needs `Debug`** (M3) |
| export.rs:134,143,267,388,539,582,639 | bare `…unwrap();` (value dropped) | none |
| tax_report.rs:392,962,1051 | bare `…unwrap();` (value dropped) | none |
| pseudo_reconcile_cli.rs:146,257,283 | `…unwrap_err()` (Err type unchanged) | none |
| pseudo_reconcile_cli.rs:311,318 | `…expect(…)` value dropped (asserts on tempdir path, not return) | none |
| **vault.rs:263** (store) / integration.rs:282 | store method → `PathBuf` | **untouched** |

Net: 3 sites → `.path` (incl. main.rs), 2 sites rely on new `Debug`, rest compile untouched,
store path untouched. Matches the spec's categorical description exactly. Because the return
type changes, every value-reading miss would be a **compile error**, never a silent wrong
result — so the list cannot hide a runtime break.

### No unaccounted-for test breakage
`grep '"export-snapshot"'` shows **no existing test drives the binary with the
`export-snapshot` CLI arg** — every current export test calls the library fn
`cmd::admin::export_snapshot(...)` directly. The new stderr `eprintln!` is therefore exercised
only by the NEW binary KATs; no existing test asserts on export-snapshot binary stdout/stderr,
so nothing regresses on the runtime side. The attest-gate ordering (require_attestation at
admin.rs:62-64 runs before any byte; a refused pseudo export `?`-returns Err at main.rs:325 and
never reaches the warning) is unchanged and correctly described (spec lines 38-40).

---

## Minor

### M-A — no-`--tax-year` path writes projection CSVs, not tax "forms"; Message B's word "forms" is loose
When `tax_year == None`, `export_snapshot` writes only the all-years projection CSVs
(`lots.csv`, `disposals.csv`, `removals.csv`, `income.csv`; render.rs:593-616+ and the
cli.rs:94-100 doc-comment). `form8949.csv` / `schedule_d.csv` are written **only** with
`--tax-year` (admin.rs:70-96; doc-comment cli.rs:96-97). So Message B's "the exported **forms**
are INFORMATIONAL, not final" (spec line 35) is slightly imprecise — no tax *forms* land on
that path; the informational artifacts are the projection CSVs. The disclosure's **substance**
is correct and errs toward caution (the projection foundation is unsound under any Hard
blocker), and the spec's own parenthetical (spec line 33, "full export writes ALL years") is
accurate — only the user-facing string overstates by naming "forms." Non-blocking; consider
"the exported figures/projection CSVs are INFORMATIONAL, not final" for Message B, reserving
"Form 8949 / Schedule D" for Message A (where they are actually produced). Not Important: the
copy tells the user to run `btctax verify` and not to file — the safe direction.

## Nit

### N-A — the stderr-asserting binary KAT must use `Command::output()`, not `.status()`
The two cited patterns (fr9_exit_code.rs:54-67, tax_report.rs:728-739) call `.status()` with
`stderr(Stdio::null())` — they capture the **exit code only** and discard stderr. To assert
`stderr contains "NOT COMPUTABLE"` (spec lines 52-55), the new KAT must call
`Command::output()` (which returns `Output { stdout, stderr, status }`) and read
`String::from_utf8(out.stderr)`. Trivial, well-known, and the spec's phrasing ("capturing
stderr/exit", line 50) already signals the intent; the cited sites correctly establish the
`CARGO_BIN_EXE_btctax` + `Command` machinery. Worth one word in T1 so the implementer doesn't
copy the `.status()`-with-null-stderr shape verbatim.

---

## Self-consistency + open-question sweep
- No residual contradiction after the rewrite. The `ExportReport { path, unresolved_hard }`
  shape, the two-message logic (`unresolved_hard > 0` × `tax_year.is_some()`), the
  library/binary KAT split, the ★ fault-inject target, and the cli.rs:92 → regenerate lockstep
  are mutually consistent and each maps to verified source.
- Implementable as written with **0 open questions**: every arg the wrapper needs (`state` at
  admin.rs:61) is in scope; no new dependency (`compute_tax_year`/profile/tables) is dragged in;
  the man-page regeneration path is the single-source one; the test rewiring is compile-forced
  and enumerable.
- No test breaks unaccounted-for (compile-forced rewires listed above; no runtime stdout/stderr
  regression on existing tests).

## What rides along to implementation
M-A (sharpen Message B copy) and N-A (`Command::output()` for the stderr KAT) are cheap and
should be picked up in T1, but neither gates.

**R0-GREEN — cleared to implement.**
