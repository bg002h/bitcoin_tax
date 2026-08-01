# §G-11 SPEC r2 — RE-REVIEW BRIEF

**Artifact:** `design/no-testimony/SPEC.md` (revision r2, commit `104197d`).
**r1 reviews, already folded:** `reviews/SPEC-doctrine-opus-r1.md` (2C/3I),
`reviews/SPEC-engineering-opus-r1.md` (0C/4I).

## The ONE question

**Did the r2 fold actually close its findings — and did it break anything in the process?**

This is a re-review, not a fresh audit. r1 was thorough; its ALSO-CHECKED-SOUND sections are settled.
**Do not re-derive what r1 established.** Spend your budget on the deltas and on what the fold could
plausibly have broken.

## What changed in r2 (the delta you are reviewing)

| # | r1 finding | r2's fix |
|---|---|---|
| C-1 | 4 productions, ≥25 fields fit none (threshold was 5) | **8 productions**; coverage table promoted from P0 *aspiration* to P0 *deliverable* |
| C-2 | `computed` keyed on the VERB while the prose said FLOOR; 17 fields forced to print a sworn zero | split into `combine` (no floor ⇒ blank iff all operands blank) and `floored` (has the clause ⇒ always entered) |
| I (doctrine) | `total_of`'s `iff` missed the SKIP class (Sch SE 8d) | production 3 takes an optional skip predicate |
| I (doctrine) | Schedule 1 has no extract; `*.txt` grep sweeps instruction files | grep restricted to `f*--YYYY.txt`; P0 must add `f1040s1--2024.txt`; grep produces a *candidate* set, each adjudicated once |
| I (eng) | scope named `printed.rs` only — excluded Form 6251, its own headline example | scope is four files: `printed.rs`, `other_taxes.rs`, `qbi.rs`, `form6251.rs`, `forms.rs` |
| I (eng) | no phase performed the uniform retyping §2.2 mandates (~60 of 168 scheduled) | **new P0b** retypes everything in scope |
| I (eng) | containment was a grep; missed inline `match`, banned the honest accessor where production needs it | **`LineEntry` is now OPAQUE** (newtype over a private enum) + `clippy.toml` + grep as backstop |
| I (eng) | both plants missed a filer-STATED zero being deleted | **third plant + stated-zero fixture** |
| M×several | E0063→E0308, counts, `Printed8949Totals` Default, map enumerator predicate, `golden_packet.rs:321` | folded |

Also new in r2: §1 names a **fourth lawful move** (btctax asserting its own arithmetic over lines
already on the page) that r1 left unnamed and which therefore became its residual bucket.

## Where a fold like this plausibly breaks

1. **The new productions may be judgment buckets wearing production costumes.** `scaled`, `bounded`,
   `constant`, `conditional` were added under time pressure to absorb r1's misfits. Are they each keyed
   to a real, quotable phrase family — or did I just name the leftovers? **This is the same failure as
   C-1, one level up**, and it is the most likely way r2 is still wrong.
2. **The count may still exceed the threshold.** r1 measured ≈25 misfits against 4 productions.
   **Re-run it against 8.** If it is still ≥5, r2 fails its own criterion exactly as r1 did.
3. **Opacity may cost more than it was priced at.** `LineEntry` must plausibly be `Copy`, `PartialEq`,
   `Eq`, `Debug`, and usable in the ~25 production sites r1 inventoried. Does the published-exit set in
   §2.2/§6 actually cover them, or does something still need a `match` it cannot have?
4. **`combine`'s skip predicate may be under-specified** — where does the predicate come from, and is it
   itself a transcription or a judgment?
5. **P0b's byte-identity claim** ("byte-unchanged, all entered") over the *four-file* scope, not just
   `printed.rs`. The crypto-slice `ScheduleDTotals` path never calls `push_money` at all.

## Settled — do NOT re-derive

The survey counts (64/168, seven mechanisms, 24 instructed zeros, layer split). The architecture (two
types, emitter-first) — an accepted consult; do not propose a different one. The four owner decisions
(§8). r1's ALSO-CHECKED-SOUND findings, specifically: P0 byte-identity traces cleanly; suppression is
geometrically safe (`verify.rs` requires strict descent, not contiguity); `extract_lines`' absent-key
premise holds; `§4`'s escape hunt is otherwise clean; `no_unmapped_filled` half-closes map coverage;
`PresentZero` and `repo_hygiene.rs` exist as precedents. Tax-figure correctness.

## Output

`VERDICT: green` or `VERDICT: <n> Critical / <n> Important`

Findings in the r1 format (`SEVERITY / WHERE / CLAIM / CONSEQUENCE / EVIDENCE`), most severe first.

**Critical** = implementing r2 as written still leaves fabricated zeros printing, or the gate cannot
detect them. **Important** = a real gap that would surface during the build.

**Green is the expected outcome and a short report is a good one.** r2 folded nine findings from two
independent lenses; if it closed them, say so and stop. Do not manufacture a finding to justify the
round — but do say plainly if a fix is cosmetic rather than real.

End with `WHAT R2 CLOSED:` (so the next round knows) and `WHAT WOULD MAKE THIS REVIEW WRONG:`.

**Constraints:** READ-ONLY. No edits, no commits, no subagents. You have a shell — read the form text
and the code rather than reasoning from the spec's description of them.
