# BRIEF — give `form-delta` the three axes it lacks. This is the biggest measured risk.

**Tier:** opus. **Worktree.** No subagents. **Owner: 2026-09-13, "fix biggest measured risk".**

## 0. Why this is the biggest risk, measured

Three independent forms showed the same shape **in one day**: a **surviving line number whose meaning
changed**. Schedule 1-A 37→43 (2026-09-11, two review rounds to hold by a type), Schedule A's six-line
cascade (FR-185), Schedule 3's nine of thirteen (FR-192). It is taxpayer-adverse in whichever direction the
substituted quantity runs, and **`form-delta` cannot see it.**

## 1. The three axes, and what each must do

**Axis A — FR-190, the label axis is keyed to the FULL AcroForm FQN.**
Controller-reproduced: `f1040s3--2021` root subform is `form1[0]`, `--2022` is `topmostSubform[0]`; nothing
else differs. **Full-FQN intersection 0; root-stripped intersection 40 of 41.** The tool prints
`0 common / 40 added / 41 removed` and its loudest banner for the calmest transition in the corpus. Across 11
pairs, **71 of 159 real line moves (45%)** never reached the output.
★★ The blindness is **correlated**: a container is renamed *because* lines were renumbered, so the key drops
boxes preferentially where the axis matters most.
*Fix:* compare on a container-insensitive key; keep the full FQN for reporting only.

**Axis B — FR-191, no line-SET axis.** 62 retired line numbers reported as zero; Schedule 8812 TY2021→TY2022
killed 34 printed lines and the output says *"54 removed"* names, none labelled. `label-census` can already
answer this — nothing joins the two instruments.

**Axis C — FR-192, no line-MEANING axis. THIS IS THE ONE.** For every line number present in **both**
revisions, compare its **printed caption** from the text layer. A materially changed caption under an
unchanged number is the collision. ★ You already have three verified positives to test against, and that is
unusual luck: use them as fixtures.

## 2. B1 — the kills write themselves here

- **Axis C must red on all three known collisions**: Schedule A 13/14/15/16/17/18 (2025→2026-DRAFT),
  Schedule 3 line 7 and line 8 (2020→2021), and Schedule 1-A 37→43 if archived. Paste the output.
- **Axis C must NOT red on a pure renumber** where the caption travels with the number — find one in the
  corpus and show it clean, or the axis is a noise generator.
- **Axis A**: plant a root-subform rename on an otherwise identical pair; the axis must still compare.
- ★ FR-187 is in scope for Axis C and is the subtle case: TY2025 *"from a **federally declared** disaster"*
  → TY2026 *"from a **federally or state-declared** disaster"*, on a line that ALSO moved 15→16. A caption
  diff must catch an eligibility change even when the number moved, so **do not skip a line just because it
  moved** — report both facts.

## 3. Scope

**IN:** `crates/xtask/src/form_delta.rs`, `form_geometry.rs`, and whatever joins `label-census`.
**OUT:** everything under `crates/btctax-core/`, `crates/btctax-forms/forms/**` (no map edits), `scripts/`.
Do not bundle a year. Do not re-archive documents — 143 extracts and 87 geometry fixtures are committed, and
prior-year pairs for 2020–2023 are already there.
★ "Materially changed" needs a definition you can defend: state it in the source, and make whitespace and
the dotted leader lines irrelevant. A caption check that reds on reflow is worthless.

## 4. Working rules

Own worktree; `CARGO_TARGET_DIR=<worktree>/target-axes`. **FOREGROUND every command** (FR-175). `make check`
(~20s) and `cargo fmt --all` — `make check` excludes fmt. Capture once, grep. **Do not commit or push.**

## 5. Deliverable

Bash heredoc (not `Write`) to `design/agent-reports/REPORT-build-form-delta-axes.md`. Short summary + path.
