# P1 re-review 2 — focused gate on the NEW-1 fold (independent Fable re-review)

*Reviewer: Fable (independent; author = the build agent). Scope: NARROW per STANDARD_WORKFLOW §2 —
confirm the fold commit `565a959` resolves NEW-1 (the false "stderr is never dropped" front-matter
claim) without introducing a new false statement, spot-check the two nit folds, confirm nothing else
regressed. Date: 2026-07-18.*

## Verdict

**NEW-1 is RESOLVED on the merits — every sentence of the reworded stderr clause verified true against
the generator, the golden, and a full 42-step live-binary replay of all six journeys — the two nit folds
are accurate, the suite is green (1944/1944), and no regression was found. 0 Critical / 0 Important —
P1 GREEN.**

## NEW-1 — RESOLVED

The reworded clause (`front_matter()`, `crates/xtask/src/examples.rs:165-170`; shipped at
`docs/examples/examples.md:16-21`) makes three claims. Each was verified independently — not against the
commit message but against source, the golden, and the real binary:

1. **"captured SELECTIVELY: where it carries substantive output — an advisory, the not-authorised
   filing notice, a Form 8283 caveat — it is shown in a separately labelled `stderr:` block, never
   merged into stdout." TRUE, with no counterexample.** The golden has exactly three `stderr:` blocks
   (J1 `examples.md:122`, J2 `:189`, J6 `:642`) — the only three `show_stderr: true` steps
   (`examples.rs:418/456/521`, all `export-irs-pdf`). Their contents carry precisely the clause's three
   named examples: the `[I5]` 1099-DA advisory + the Schedule D 17-22 note (J1), the NOT-AUTHORISED
   notice (all three), and the Form 8283 Section-B / needs-REVIEW caveats (J2/J6). `emit()`
   (`examples.rs:129-146`) never appends stderr to the stdout fence — "never merged" holds structurally.
2. **The over-claim hunt (does any non-shown step emit substantive stderr that is silently dropped?):
   NO — verified empirically, not just by source-reading.** I replayed **every step of all six journeys
   plus `--help` (42 commands)** against the freshly built debug binary under `capture()`'s exact pinned
   env (`BTCTAX_PASSPHRASE`/`BTCTAX_PRICE_CACHE`/`HOME`/`TZ`/`LC_ALL`/`LANG`; `BTCTAX_NOW` removed,
   re-set only on the four pinned steps). Result: all 35 non-shown unpinned steps emit **zero bytes** of
   stderr; the four pinned steps emit **exactly** the one banner line and nothing else; the three shown
   steps emit exactly what the golden's blocks show. Corner cases specifically ruled out: J3's
   `[exit 1]` `verify` prints its blocker report to **stdout** (the `error:` stderr path,
   `main.rs:43`, is the exit-2 `Err` arm no golden step hits); J5's `optimize run`/`accept` did **not**
   fire the APPROXIMATE warning (`cmd/optimize.rs:62`); J6's `income import` did **not** fire the
   carryover note (`cmd/tax.rs:92`); J1's non-shown `export-snapshot` has 0 unresolved hard blockers so
   `main.rs:577/584` are silent; the env-conditional store warnings (memlock/vault) fired nowhere
   (also evidenced by their absence from the three *shown* blocks).
3. **"What is NOT shown is the fixed integrity banner that a pinned `BTCTAX_NOW` prints to stderr on
   the clock-pinned steps … deliberately elided (disclosed here so the omission is never silent)."
   TRUE.** (a) All four pinned steps — J3 `classify-inbound-self-transfer` (`examples.rs:361-368`,
   pinned 2025-08-02) and J5 `config`/`optimize run`/`optimize accept` (`examples.rs:326/329/335`,
   pinned 2025-01-01) — emit `warning: BTCTAX_NOW override active — decision timestamps are simulated`
   via `resolve_now()` (`main.rs:83`, unconditional, before dispatch) — confirmed live, banner-only.
   (b) `emit()` drops stderr whenever `show_stderr` is false (`examples.rs:139`). (c) The banner text
   appears **nowhere** in the golden (`grep 'override active'` = 0 hits). The SPEC §3.3 "declared out
   of the verbatim-stdout capture" / §13(d) "disclosed, not silently dropped" contract is now genuinely
   served: the elision is real, scoped exactly as stated, and disclosed.

## Nit folds — spot-checked, both accurate

- **N-A (SPEC §15(a) year switch): CORRECT.** §5:254 does say (verbatim) "J4 uses 2024 for kitchen-sink
  oracle-consistency"; delivered J4 is all-2025 (corpus dates 2025-04-15 / 2025-05-20,
  `report --tax-year 2025` — `examples.rs:213-215/296`); "J4 no longer shares the kitchen-sink oracle
  (only J6 does)" is true (only J6 imports `fullreturn_inputs.toml`); and the sentence claims only that
  the 2024-specific alignment rationale evaporated — it does not falsely claim 2024 (a supported,
  on-dataset year) would not work.
- **N-B (UX-P1-5 de-anchor): CORRECT.** The FOLLOWUPS entry now cites "in J6's `income show` block of
  the golden" (no line number); the `"date_of_birth": [2012, 106]` tuple sits at `examples.md:481-484`
  inside that block (the old `474-477` anchor was indeed stale). Decay-proof as intended.

## New findings

- **N-C (Nit, non-gating — record with the ownerless wording residue).** The clause's aside "it is
  determinism scaffolding, **not btctax output**" is loosely worded: the btctax binary does literally
  print the banner (the same sentence concedes it — "prints to stderr on the clock-pinned steps"). The
  intended sense — not part of the demonstrated journey's own output, but the harness clock-pin's
  disclosure — is unambiguous in context, and no replaying reader's inference goes wrong (they are told
  exactly which extra line appears on pinned steps and why the captures omit it). Not false in effect;
  a future wording pass could say "not part of the journey's own output". Does not gate.

## Validation surface

- `make check`: **green** — 1944 run / 1944 passed / 6 skipped (12.7s), including all three examples
  gates (`examples_golden_matches_committed`, `examples_generate_is_hermetic_across_ambient_env`,
  `generate_is_deterministic_and_captures_help`).
- Working tree clean; `crates/xtask/tests/` does not exist (no fixture-move residue).
- No other file in the fold commit beyond the five expected (front-matter reword + regen + SPEC §15(a)
  sentence + FOLLOWUPS de-anchor + the persisted re-review verbatim).

## Gate

**0 Critical / 0 Important / 0 Minor / 1 Nit (N-C, non-gating) — P1 GREEN.**
