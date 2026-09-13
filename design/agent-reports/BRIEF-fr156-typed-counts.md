# BRIEF — FR-156: the two per-port typed counts, and whether deriving them is actually right

**Tier:** sonnet. **Isolation:** your own worktree. **One agent.** Do not spawn subagents.
**Owning phase:** NOW, before January. Budget: small. This is hygiene, **not** a sweep.

## 0. The one question — and it is NOT "go derive them"

> Two test constants are re-typed every port. Should they be **derived** from the year record — or does
> deriving them destroy the very thing they were written to catch?

**Read that twice before editing anything.** FR-156 was filed assuming "derive them", and the
controller's own check found a reason that may be wrong. Your job is to **answer the question**, then do
whatever the answer says — including "leave one of them alone, here is why."

## 1. The two sites, measured — do not re-derive

| site | today |
|---|---|
| `crates/btctax-forms/tests/supported_years_cross_product.rs:147` | `const BUNDLED_FORMS_PER_YEAR: &[(i32, usize)] = &[(2024, 20), (2025, 18)];` |
| `crates/btctax-forms/tests/map_pdf_conformance.rs:207` | the `Form8995AMap::for_year(unmapped).is_err()` refusal loop |

Also measured: `forms/2025/YEAR.toml`'s `forms_expected` has **exactly 18** entries, matching the pinned
18. There are **3** `YEAR.toml` files (2024, 2025, 2026).

## 2. ★ The counter-argument you must engage

`BUNDLED_FORMS_PER_YEAR`'s own doc comment says why it exists:

> *"Pinned so that **deleting** an asset is as loud as adding one — a cell with no recorded gap vanishing
> from the matrix would otherwise be silent."*

And `tests/year_record.rs` already holds `forms_expected` to the stems **on disk**.

So trace this through before you touch it:
- Delete an asset, leave `YEAR.toml` alone → `year_record.rs` reds. Good either way.
- Delete an asset **and** its `YEAR.toml` line → a count derived from `YEAR.toml` now agrees with disk,
  and **nothing reds**. The hand-typed pin would have red.

If that trace holds, deriving from `YEAR.toml` is a **downgrade dressed as a derivation** — the exact
shape `CLAUDE.md` warns about, one level up. The honest outcomes are then: keep the pin; or derive *and*
add a check that the two sources agree; or something better you can argue. **Any of those is a fine
answer. Silently deriving is not.**

Do the same trace for site 2 before changing it: that loop asserts the **absence** of a port, which is
the FR-136 plant shape one level up, and absence-assertions are easy to weaken by accident.

## 3. Scope

**IN:** the two files named in §1, plus whatever minimal edit your §2 answer requires.

**OUT:**
- **Do NOT sweep for other hardcoded counts.** FR-152 (30 absolute `extract_line` anchors), FR-155 and
  FR-159 are the same class and are **correctly parked**. A sweep is a fresh audit and is not this task.
- Anything under `scripts/oracle/` or `crates/btctax-core/tests/golden_returns.rs` — another agent owns
  those files right now. Do not read-modify them.
- Do not change `year_record.rs`, `bundled.rs`, or any `YEAR.toml`.

## 4. B1 — whatever you land must be seen RED

If you change a checker, plant the exact defect it exists to catch, watch it red, unplant, watch it
green, and **paste both outputs**. Specifically: plant the *deletion* case from §2 (remove an asset and
its `YEAR.toml` entry together) and report honestly whether your version catches it. If it does not,
that is the finding, and it is worth more than the edit.

★ If you conclude "change nothing", you still owe the trace and the planted-defect evidence showing the
current pin *does* catch what its comment claims. A "leave it" with no kill run is not an answer.

## 5. Stop-and-report

If you can disprove anything in this brief — including §2's trace, or the claim that `forms_expected` is
18 — stop and report it rather than building on it. Four briefs in this arc carried a refuted premise;
every time, stopping was the right call.

## 6. Working rules

- Work in **your own worktree**. `CARGO_TARGET_DIR=<your-worktree>/target-fr156` — covered by
  `.gitignore:61`'s `target-*/`. Never a target dir in `/tmp` (32 GB tmpfs) or outside the ignore glob.
- `make check`, not `cargo test --workspace`. Capture once, grep twice.
- **Do not commit, do not push.** Leave the worktree dirty; name the files you changed.

## 7. Deliverable

As your **final action**, write your report with a Bash heredoc (`cat > <path> <<'MARKER'` … `MARKER`) —
**not** the `Write` tool — to:

    design/agent-reports/REPORT-build-fr156-typed-counts.md

Return only a short summary plus that path. The report states: your §2 trace and its verdict; what you
changed (or deliberately did not); the planted-defect output, red and green, pasted; and any follow-up
worth filing with a proposed owning phase.
