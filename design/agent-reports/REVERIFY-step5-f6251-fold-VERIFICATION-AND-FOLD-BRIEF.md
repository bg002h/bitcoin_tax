# Controller ledger + fold brief — the step-5 re-verification's one Important

**Report:** `design/agent-reports/REVERIFY-step5-f6251-fold.md` (persisted verbatim `78d77754`, 172 lines,
**0C/1I/0M/0N**). Both halves are pre-fold controller artifacts, so they share a commit; the fold itself
gets its own, as always.

---

# Part 1 — the ledger. The finding is CONFIRMED.

## What the verifier found

`SeniorDeductionSubtotal` is unforgeable as claimed — no `Deserialize`, no `Default`, no `From`, one
construction site, one production caller. **But the emitter's actual join input is one hop downstream**:
`Form6251Line1::Y2025`'s `schedule_1a_line: u32`, a bare field on a public enum variant, which any crate
can set to anything. So the guarantee the join needs end-to-end is narrower than the guarantee the type
provides.

## Controller check — the mechanism, and why the obvious defence does not hold

`crates/btctax-core/src/tax/form6251.rs:46` carries `#[non_exhaustive]` **on the enum**, not on the
variant:

```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Form6251Line1 {
    Y2024 { line1: Usd },
    Y2025 { line1a: Usd, line1b: Usd, schedule_1a_line: u32 },
}
```

★ **`#[non_exhaustive]` on an enum forces a wildcard arm in external `match`es; it does not make existing
variants non-constructible.** Only `#[non_exhaustive]` on the *variant* does that. So the defence that
looks present is not the defence that is needed — which is the same shape as the round's original finding,
one level down.

**Proven in the tree**, four construction sites outside `btctax-core/src`:

| site | crate |
|---|---|
| `crates/xtask/src/line_coverage_check.rs:1534` | xtask |
| `crates/btctax-forms/tests/f6251_fill.rs:170` | btctax-forms |
| `crates/btctax-forms/tests/f6251_obbba.rs:759` | btctax-forms |
| `crates/btctax-forms/tests/f6251_obbba.rs:965` | btctax-forms |

So a `Form6251Line1::Y2025` can be built with any `schedule_1a_line`, and the emitter's comparison then
checks a number the caller chose against the form's sentence. The provenance the new type establishes is
**lost at the hop from `Form6251Line1Rule` to `Form6251Line1`.**

## Severity — Important, not Critical: the verifier's call is right

Every production construction is inside `btctax-core` and goes through the rule that consumes the
unforgeable subtotal; the four external sites are tests and one instrument. No filer's data can reach a
forged value today, and TY2026 still has no arm, no `LineSet`, and no bundled params. So this is **a
guarantee that is not structurally held**, not a live wrong result — Important. It becomes Critical the
day a non-core surface constructs this variant on a production path.

**An Important is open, so step 5 is not green and does not close on this round.**

---

# Part 2 — the fold brief

You are ONE opus implementer in the **shared main tree**. Nothing is committed while you work. Scope is
**this one finding**. Do not widen it.

## 0. Stop-and-report

Seven briefs in this arc have been refuted by measurement; three were the coordinator's. **Stop and report
rather than build on a premise here you can disprove** — including Part 1's reading of `#[non_exhaustive]`.
A refuted premise is a first-class finding.

## 1. The requirement

**Make `Form6251Line1::Y2025` impossible to construct outside `btctax-core` with an unvouched
`schedule_1a_line`** — so the line number the emitter joins against can only have come from the schedule
revision that printed it. The provenance must survive the hop from the rule to the computed value.

`#[non_exhaustive]` on the **variant** is the idiomatic mechanism, paired with a core-side constructor that
accepts the `SeniorDeductionSubtotal` (or whatever carries the vouched line) rather than a bare `u32`.
Carrying the subtotal itself in the variant is also defensible. **You choose; state why in the report.**

## 2. ★★ The constraint that makes this non-trivial — do not break the kill

The round's central kill, `a_revision_citing_line_43_refuses_a_figure_read_off_line_37`, **must be able to
construct a MISMATCHED value** — that is its whole job. So the fix must leave a deliberate route for a test
to forge one, while making production forging impossible.

Two failure modes to avoid, and they are opposite:

- **Sealing it so hard the kill cannot be written** — then the guarantee is unfalsifiable and B1 is
  violated. The kills that exist today must still exist and still red.
- **Leaving a public forge with an innocent name** — then nothing changed, because the next surface uses
  it. If you add a test-only route it must be **unmistakable at the call site** (a `testonly` module, a
  `#[doc(hidden)]` name that says it forges, or `#[cfg(test)]`-gating where the caller is in the same
  crate). Name it so that its appearance in production code reads as a defect on sight.

The four existing external construction sites (Part 1's table) all have to be updated. Three are tests and
one is `xtask`'s instrument — check what each actually needs: a *legitimate* vouched value, or a
deliberately forged one. Do not hand a legitimate caller the forge.

## 3. Kills — B1, each seen red first

1. **Production forging is impossible**: plant an attempt to construct `Y2025` with a chosen
   `schedule_1a_line` from a non-core crate on a non-test path → paste the compile error.
2. **The existing central kill still reds** when the join is neutralised — re-run it and paste the output,
   so the fix is shown not to have disarmed the thing it protects.
3. **The forge route, if you add one, cannot be mistaken for production**: say in the report exactly what
   stops that — a lint, a module boundary, a `cfg`, or the name.

## 4. Mechanics

**Main tree. Do NOT commit, push, `git stash`, `git checkout` or revert anything** — a builder in this arc
ran `git stash` against its brief. Revert a mutation with a **cp backup**. **No subagents.** No
`--no-verify`. Never hand-count what a tool can count. Baseline: `make gate` **3594 passed / 12 skipped**,
fmt clean, `xtask line-coverage` 377 / f6251:43 with ratchets unmoved. Report numbers as numbers.

Do not bundle params, do not add a `2026` arm, do not touch the fail-closed gates, and do not revisit the
six items of the previous fold — they are re-verified clean.

## 5. Your report

Write `design/agent-reports/FOLD-step5-f6251-provenance.md`: the mechanism chosen and **why**; what
happened to each of the four external construction sites; kills with pasted red-then-green; what stops the
forge route being used in production; refuted premises; residue with owning phases; the literal gate
numbers. Return only a short summary plus the path.
