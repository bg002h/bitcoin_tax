# IMPL-P2 — Fable independent code review, round 4 (FINAL certification pass)

**Scope:** full-return Phase 2, branch `full-return`, HEAD `0c73bc9`. Fold diff
reviewed: `eed852e..0c73bc9` (one commit, `0c73bc9` "fold Fable review r3 Minors
(in-phase burndown) — P2 r4"; touches exactly 4 files: `resolve.rs` (+19/−11),
`tests/tax_profile.rs` (+49), tui-edit `main.rs` (+76/−?), and the verbatim r3
review, +335). Phase 2 was GREEN at r3 (0C/0I/4M); this pass verifies the
4-Minor burndown and certifies the phase.
**Reviewer:** Fable (independent; author was a different model).
**Date:** 2026-07-12.

**Verdict: GREEN — 0 Critical / 0 Important / 1 Minor (record-only).**
All four r3 Minors are genuinely closed — each verified in current source AND
by mutation testing (all three behavioral claims were falsified-then-restored:
each mutation kills exactly the test that claims to pin it). All r1/r2/r3
findings remain closed; the frozen engine is byte-identical across the whole
phase; suite and clippy are clean. The one new Minor is a test-gap-on-a-fix
note that cannot regress below the already-reviewed r3 state — record it, do
not re-gate.

---

## 1. Verification actually run (real output)

`cargo test --workspace` (full log captured, exit 0):

```
EXIT=0
result-lines: 81  non-ok: 0
TOTAL passed: 1464
TOTAL failed: 0
```

(1464 = r3's 1462 + the fold's two new KATs. All 81 `test result:` lines `ok`.)

`cargo clippy --workspace --all-targets` (tail; `grep -cE "^(warning|error)"` = **0**):

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.50s
EXIT=0
```

Frozen-engine byte-identity across the WHOLE phase (P1-GREEN → r4 head):

```
$ git diff 059ec2a..0c73bc9 -- crates/btctax-core/src/tax/types.rs \
    crates/btctax-core/src/tax/compute.rs crates/btctax-core/src/tax/se.rs | wc -c
0
$ cargo test -p btctax-core --lib frozen_guard
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 143 filtered out; finished in 0.00s
```

(The fold touches no `btctax-core` file — `--stat` confirms.)

The fold's new KATs, from the suite log:

```
test tests::kat_f1_both_sources_form_shows_stored_not_derived ... ok
test resolve_all_screened_maps_a_corrupt_year_to_a_refusal_not_a_brick ... ok
```

Working tree clean at `0c73bc9` before and after review (mutation probes in §3
were each reverted; `git status --short` / `git diff HEAD` empty at the end).

---

## 2. Per-r3-Minor verification

| r3 | Claimed fix | Verified at | Status |
|---|---|---|---|
| **M-r3-1** no BOTH-sources form-layer KAT | `kat_f1_both_sources_form_shows_stored_not_derived` | tui-edit `main.rs:9988-10049`; §2.3 + mutation A (§3) | **CLOSED — non-vacuous (mutation-killed)** |
| **M-r3-2** `open_profile_form` swallows a read error | error → `app.status`, form opens empty | tui-edit `main.rs:713-723`; §2.2 | **CLOSED** (one record-only note, §5) |
| **M-r3-3** compute-dependent block silent-skips on broken invariant | `let (Some(ri), Some(params)) … else { Uncomputable }` | `resolve.rs:189-198`; §2.1 + mutation C (§3) | **CLOSED — fails closed (mutation-demonstrated)** |
| **M-r3-4** N3 per-year mapping untested | `resolve_all_screened_maps_a_corrupt_year_to_a_refusal_not_a_brick` | `tests/tax_profile.rs:204-251`; §2.4 + mutation B (§3) | **CLOSED — non-vacuous (mutation-killed)** |

### 2.1 M-r3-3 — the let-else fails closed; the normal path is unchanged

`resolve.rs:194-198`: the former no-else `if let` is now
`let (Some(ri), Some(params)) = (ri.as_ref(), full_return) else { return Ok(ProfileOutcome::Uncomputable { detail: uncomputable_detail(year, None) }) }`.
Reachability of the else-arm, re-derived from current source: the block is
entered only with `provenance == ReturnInputs` AND `profile: Some` (the
`is_return_inputs_uncomputable` early-return at `:183-187` catches
`profile: None`); `resolve_core` produces that combination ONLY at its derived
exit (`:104-109`), which requires `full_return` was `Some` and returns
`Some(ri)`. So the else-arm is unreachable today and fires only on a broken
invariant — returning `Uncomputable`, never a number. The happy path,
`screen_compute_dependent(ri, state, year, params)` → refusal → `Uncomputable`
(`:199-203`), is byte-identical to the pre-fold code (same bindings, same
calls — checked against the diff). `resolve_core` itself and `resolve_profile`
are untouched by the fold. **CLOSED.**

### 2.2 M-r3-2 — the read error is surfaced; the borrow is sound

`main.rs:716-723`: the `.ok().flatten()` is replaced by a match on
`app.session.as_ref().map(|s| s.tax_profile(year))`. `Session::tax_profile`
(`session.rs:445-447`) takes `&self` and returns an OWNED
`Result<Option<TaxProfile>, CliError>`, so the match scrutinee is an owned
temporary — no borrow of `app` is held into the arms, and
`app.status = Some(…)` in the `Some(Err(e))` arm is sound (borrow ends at the
`map`; confirmed by compilation + clippy clean). The error text
("could not read the stored profile for {year}: {e}") lands in `app.status`,
which renders in the footer (`draw_edit.rs:191`), and the form still opens
empty — one of the two remedies r3 offered. `None` (no session) and `Some(Ok)`
arms reproduce the old behavior exactly. **CLOSED.**

### 2.3 M-r3-1 — the BOTH-sources KAT is distinguishing

`kat_f1_both_sources_form_shows_stored_not_derived` (tui-edit
`main.rs:9993-10048`): builds a real vault; stores a DISTINCTIVE raw profile
(MFJ/$999,999) via the real CLI `set_profile` (no `--force` needed — stored
BEFORE the import; `import_return_inputs`, `cmd/tax.rs:49-60`, has no
stored-profile guard, so the seeding order is legitimate); imports RI that
derive a DIFFERENT (Single/$50k-W-2) profile; unlocks through the real
`do_unlock` (`editor.rs:360-383` — session + snapshot set together via the
shared `open_session`/`build_snapshot` path); presses `p`; asserts
`form.filing_status == Mfj` and `fields[0].buf == "999999"`. The resolved
`snapshot.profiles` for that year holds the DERIVED Single profile (RI wins
the ladder; `btctax-tui/src/unlock.rs:181-195` splits Ready→`profiles`,
Uncomputable→`refused`), so a regression of `open_profile_form` to the
resolved map cannot pass — confirmed empirically in §3 (mutation A).
**CLOSED, non-vacuous.**

### 2.4 M-r3-4 — the corrupt-year KAT pins the availability mapping

`resolve_all_screened_maps_a_corrupt_year_to_a_refusal_not_a_brick`
(`tests/tax_profile.rs:208-251`): real vault; valid `ReturnInputs` for 2024
via `return_inputs::set` (which also creates the table, so the subsequent raw
`INSERT INTO return_inputs(year, inputs_json) VALUES (2023, 'not json')`
targets an existing table); reopens the vault, calls the real
`Session::resolve_all_screened` with the real projection + bundled tables;
asserts 2023 → `Uncomputable { .. }` AND 2024 → `Ready { .. }`. The pin is
the `.unwrap()` at `:240`: if the per-year `Err` mapping
(`session.rs:504-516`) regressed to `?`, the whole call returns `Err` and the
test panics — confirmed empirically in §3 (mutation B). The `Ready` assert on
2024 additionally pins that the mapping doesn't over-refuse healthy years.
**CLOSED, non-vacuous.**

---

## 3. Mutation testing (each claim falsified, then restored)

Each mutation was applied to a clean tree, the claiming test run, and the
mutation reverted; final `git status --short` / `git diff HEAD` — empty.

**Mutation A** (M-r3-1): reverted `open_profile_form` to
`app.snapshot…profiles.get(&year).cloned()` (the pre-N1 bug):

```
test tests::kat_f1_both_sources_form_shows_stored_not_derived ... FAILED
panicked at crates/btctax-tui-edit/src/main.rs:10034:9:
assertion `left == right` failed: form must show the STORED filing status, not the derived Single
```

**Mutation B** (M-r3-4): replaced the per-year `match` in
`resolve_all_screened` with `?`:

```
test resolve_all_screened_maps_a_corrupt_year_to_a_refusal_not_a_brick ... FAILED
panicked at crates/btctax-cli/tests/tax_profile.rs:240:60:
called `Result::unwrap()` on an `Err` value: BadConfigValue { key: "return_inputs[2023]",
value: "invalid JSON: expected ident at line 1 column 2" }
```

**Mutation C** (M-r3-3): broke the `resolve_core` invariant (derived exit
returns `ri: None`):

```
test resolve::tests::resolve_and_screen_gives_return_inputs_precedence_over_stored ... FAILED
panicked at crates/btctax-cli/src/resolve.rs:348:56:   ← the `Uncomputable` arm:
    ProfileOutcome::Uncomputable { detail } => panic!("expected Ready, got: {detail}")
```

Mutation C is the load-bearing one: under the broken invariant the function
now returns **Uncomputable** (the KAT's fail-arm fired), where the pre-fold
`if let` would have silently skipped the compute-dependent screen and
returned `Ready` — i.e. this precedence KAT would have PASSED pre-fold. The
let-else demonstrably converts the fail-open silent skip into a refusal.

All three tests re-run green on the restored tree:

```
test resolve::tests::resolve_and_screen_gives_return_inputs_precedence_over_stored ... ok
test resolve_all_screened_maps_a_corrupt_year_to_a_refusal_not_a_brick ... ok
test tests::kat_f1_both_sources_form_shows_stored_not_derived ... ok
```

---

## 4. Regression check — r1 (2C/1I), r2 (2I), r3 closures all intact

The fold touches none of the r1 sites (`tables.rs`, `return_1040.rs`,
`return_refuse.rs`, `tabs/tax.rs`, `whatif_panel.rs`, `export.rs`) and none of
the r2 mechanisms (`resolve_core` body, `edit/persist.rs`, `session.rs`,
`return_inputs.rs` — the fold's `resolve.rs` hunk is entirely inside
`resolve_and_screen`'s compute-dependent block). Named pins, from this run's
full-suite log — all ok:

```
test tabs::tests::tax_tab_refused_full_return_year_renders_reason_not_a_number ... ok   (r1-C1)
test tax::return_1040::tests::student_loan_phaseout_and_mfs_zero ... ok                 (r1-C2)
test tax::return_1040::tests::schedule_b_part3_none_is_fail_loud_only_when_filing ... ok (r1-I1)
test tax::return_refuse::tests::schedule_b_part3_unanswered_refuses ... ok               (r1-I1)
test edit::persist::tests::persist_tax_profile_refuses_when_return_inputs_exist_d4 ... ok (r2-N1)
test resolve::tests::return_inputs_beats_stored_profile_and_derives_a_profile ... ok     (r2-N2)
test tests::kat_f1_p_opens_form_prepopulated_from_existing_profile ... ok                (r2-N1)
test report_tax_year_derives_and_computes_from_ty2024_return_inputs ... ok               (seam, live)
test report_tax_year_refuses_business_income_without_schedule_c ... ok                   (fail-closed, live)
test report_tax_year_with_return_inputs_for_unsupported_year_refuses_with_income_clear_hint ... ok
```

Single-resolver invariant unchanged: the one ladder is still `resolve_core`;
both public entry points still delegate; every computing consumer still routes
through `resolve_and_screen` (`session.rs:461` + `:504`, `cmd/tax.rs:166`).
FOLLOWUPS records (r2-N4 → P4, D1/D2 → P4, `p2-pref-over-ti-clamp` → P3) are
untouched by the fold, correctly — all four r3 Minors were FIXED in-phase, so
there was nothing to record.

---

## 5. NEW findings introduced by this fold

### MINOR

**M-r4-1 (record-only) — the M-r3-2 error-surface arm has no dedicated test.**
`open_profile_form`'s `Some(Err(e))` arm (tui-edit `main.rs:718-721`) is
exercised by no KAT: no test seeds a corrupt `tax_profile` blob and asserts
the status message appears and the form opens empty. Both KAT-F1s exercise
only the `Some(Ok)` arm. A silent regression here can at worst reintroduce the
r3 masked-as-empty behavior — itself already reviewed and ranked Minor, with
the save path independently D-4-guarded and atomic — so the regression floor
is a known, documented state, not a new hazard. Record to FOLLOWUPS (or fold
opportunistically alongside future tui-edit work); this does NOT warrant
another gate round, and certification does not wait on it.

### Observations (not findings)

- The M-r3-3 else-arm reuses `uncomputable_detail(year, None)`, whose text
  says the year "is not supported (v1 supports TY2024)" — self-contradictory
  if the broken-invariant state were ever reached for 2024. Unreachable by
  construction (§2.1), fail-closed, and the `income clear` recovery hint is
  still correct; any refactor that made it reachable already fails the
  precedence KAT (§3, mutation C). Not ranked.

---

## 6. Fold hygiene

- The r3 review is persisted verbatim in the fold commit
  (`IMPL-P2-fable-review-r3.md`, 335 lines) — same §2 pattern as r2/r3. ✓
- One fold commit (`0c73bc9`); the message accurately names all four Minors
  and their fixes, and correctly states the r3 GREEN baseline. ✓
- Diff is exactly the four claimed changes — no drive-by edits (`--stat`:
  4 files, 468/11). ✓
- No frozen-file drift in the fold or across the phase (§1). ✓
- Note for the record: mid-review, an UNCOMMITTED edit to `FOLLOWUPS.md`
  (marking `p1-consumer-sweep-P2` superseded) appeared in the shared worktree
  from outside this review session (the tree was verified clean at review
  start and after each mutation revert). It is not part of `0c73bc9` and does
  not affect this certification; flagged per the no-parallel-tasks-in-a-
  shared-worktree process rule.

---

## Verdict: GREEN

**0 Critical / 0 Important / 1 Minor (record-only).** The r3→r4 fold does
exactly what it claims and nothing else. All four r3 Minors are closed, and —
unusually well-evidenced for a burndown — every behavioral claim survived
mutation testing: the BOTH-sources KAT dies if the form reads the resolved
map, the corrupt-year KAT dies if the per-year mapping regresses to `?`, and
the let-else turns a broken `resolve_core` invariant into `Uncomputable`
where the old code silently skipped the screen. Suite 1464/0, clippy clean,
frozen engine byte-identical from `059ec2a` through `0c73bc9`, and all
r1/r2/r3 findings remain closed. The single new Minor (no dedicated test for
the M-r3-2 error arm) is record-only with a bounded regression floor.
**Phase 2 is certified GREEN at `0c73bc9`.**
