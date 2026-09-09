# VERIFICATION — controller's independent machine-check of `RECON-fr111-8949-box-truth.md`

Run by the controller before folding anything, per `CLAUDE.md` *"independently machine-checks every
measurable claim before acting on it."* Read-only; no source edited.

## Claims checked

| # | report's claim | result | evidence |
|---|---|---|---|
| 1 | regime table: 2024 `(proceeds=false, basis=false)`, 2025 `(true, false)`, 2026 `(true, true)` | ✅ **exact** | `grep -A3 f1099da crates/btctax-forms/forms/{2024,2025,2026}/YEAR.toml` → `false/false`, `true/false`, `true/true` |
| 2 | only three bundled year records | ✅ | `ls -d crates/btctax-forms/forms/*/` → `2024/ 2025/ 2026/` |
| 3 | TY2024 crypto rows are Box C/F, pinned by a KAT | ✅ | `kat_forms.rs:152-153` — `assert_eq!(st_row.box_, Form8949Box::C); assert_eq!(lt_row.box_, Form8949Box::F);` |
| 4 | a live year routes to G/H/J/K from stored answers | ✅ | `kat_broker_reporting.rs:337-341` — the table literally enumerates `(NotReported, I, L)`, `(ProceedsOnly, H, K)`, `(BasisMatches, G, J)`, driven by `sold_in(2026, …)` |
| 5 | `LIMITATIONS.md` claims TY2024 only | ✅ | `LIMITATIONS.md:3` — *"**Tax year supported: TY2024 only**"* |

## ★ Two findings the report did not draw out, both sharpening Q3-1

**A. "TY2024 only" is NOT itself stale — which makes the box paragraph worse, not better.**
`BundledFullReturnTables::load()` (`crates/btctax-adapters/src/tax_tables.rs:100-105`) inserts
exactly one year: `by_year.insert(2024, ty2024_full_return());`. So `full_return_for(2025)` is
`None` and the header's scope claim is accurate.

The consequence: `LIMITATIONS.md:415-417` sits in a document whose supported year is **TY2024**, and
it describes **I/L** — the TY2025+ boxes — and then explicitly denies **C/F**, which is exactly and
unconditionally what TY2024 uses. The paragraph does not merely contain a false clause; **it names
the wrong year's boxes and then rules out the right ones.** That is the sharpest available statement
of the defect and the one the fold should lead with.

**B. `SUPPORTED_YEARS` is derived, and it is a different set from the filing scope.**
`crates/btctax-forms/src/lib.rs:105` — `pub const SUPPORTED_YEARS: &[i32] = bundled::TEMPLATE_YEARS;`
*"★ DERIVED from the glob by build.rs: the years with at least one bundled TEMPLATE (a `preparing`
year with only its record — TY2026 since spec 1099-DA T0 — is bundled but not supported for
filling)."* So three distinct year sets are in play — bundled records (2024/25/26), bundled
templates, and full-return params (2024 alone) — and the fold must not collapse them into one
sentence. This is why the report's "TY2026 can reach G/H/J/K via `report`/CSV/TUI but cannot print a
PDF" is consistent rather than contradictory.

## Corrections the report made to the controller's own brief — both accepted

1. **Q2 — the export advisory is NOT stale.** My brief implied it might print *"files EVERY Bitcoin
   row under Box I/L"* on a return routed to G/H/J/K. It cannot: that wording is branch 3, gated on
   `regime.basis == false`, while reaching G/H/J/K requires `regime.basis == true`
   (`forms.rs:127-129`). Mutually exclusive on the same `regime`. Accepted — no defect there.
2. **`Form1099B` existing does not reopen the Form 8949 box question.** `form_8949()` reads only
   `state.disposals`; a `[[b_1099]]` row never becomes an 8949 row of any box, by IRS design
   (Schedule D Exception 1/2 aggregate reporting). The mechanism that reopens the box question is
   `BrokerReporting` (1099-DA), not `Form1099B`. **`FOLLOWUPS.md`'s FR-111 entry carries the
   conflated framing and must be corrected in the fold** — it says *"if a 1099-B with basis reported
   can be entered, the Form 8949 box choice is live again and Box C/F is the 'reported to you'
   case."* That is wrong on the mechanism.

## Verdict

Report is accurate on every measurable claim checked. Safe to fold. Nothing folded yet — this file
and the report are both pre-fold artifacts.
