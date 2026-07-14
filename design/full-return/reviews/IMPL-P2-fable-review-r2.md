# IMPL-P2 — Fable independent code review, round 2 (re-review of the r1 fold)

**Scope:** full-return Phase 2, branch `full-return`, HEAD `5769dca`. Fold diff
reviewed: `8eb7118..5769dca` (one code commit, `5769dca` "fold Fable code
review r1"). Whole-P2 context re-checked where the fold touches it
(`059ec2a..5769dca`).
**Reviewer:** Fable (independent; author was a different model).
**Date:** 2026-07-12.

**Verdict: NOT GREEN — 0 Critical / 2 Important / 2 Minor.**
All seven r1 findings are genuinely closed (table in §2), but the fold itself
introduced two new Important defects: a missed consumer of the re-semanticized
`Snapshot.profiles` (the tui-edit profile form now pre-populates DERIVED values
into a raw-profile editor and can silently clobber the stored escape-hatch
profile), and the M3 restructure de-linked the SPEC §4.12 precedence KAT from
the live resolver path (the only stored-vs-ReturnInputs precedence test now
pins production-dead code).

---

## 1. Verification actually run (real output)

`cargo test --workspace` (full suite, exit 0):

```
ok-lines: 88   TOTAL passed: 1460   failed: 0
EXIT=0
```

(88 `test result:` lines across all test binaries incl. doc-tests, every one
`ok`; 1460 = r1's 1458 + the two new named tests below.)

`cargo clippy --workspace --all-targets` (tail; grep -c "warning|error" = 0):

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.55s
EXIT=0
```

Frozen-engine byte-identity across the WHOLE phase (P1-GREEN → r2 head):

```
$ git diff 059ec2a..5769dca -- crates/btctax-core/src/tax/types.rs \
    crates/btctax-core/src/tax/compute.rs crates/btctax-core/src/tax/se.rs | wc -c
0
$ cargo test -p btctax-core --lib frozen_guard
test tax::frozen_guard::tests::frozen_engine_files_are_unchanged ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 143 filtered out
```

Targeted new/changed KATs, run individually:

```
test tabs::tests::tax_tab_refused_full_return_year_renders_reason_not_a_number ... ok
test tax::return_1040::tests::student_loan_phaseout_and_mfs_zero ... ok
test tax::return_1040::tests::schedule_b_part3_none_is_fail_loud_only_when_filing ... ok
test tax::return_refuse::tests::schedule_b_part3_unanswered_refuses ... ok
```

---

## 2. Per-r1-finding verification

| r1 | Claimed fix | Verified at | Status |
|---|---|---|---|
| **C1** TUI resolver bypass | `Snapshot.profiles` = resolved map + `Snapshot.refused`; all viewer read-sites guarded | §3 below | **CLOSED** (viewer). The fold missed the EDITOR read-site → new finding **N1** |
| **C2** QSS §221 range | `Qss` → unmarried $80k–$95k | `tables.rs:264-271`; KAT `return_1040.rs:607-617` | **CLOSED** (re-derived, §4) |
| **I1** Sch B Part III orphan predicate | wired into `screen_inputs` as `ScheduleBPart3Unanswered`, now covers 7a AND 8 | `return_refuse.rs:50,302-310`; KATs both layers | **CLOSED** (§5) |
| **M1** pref>TI clamp | documented at strip site + P3 FOLLOWUP | `return_1040.rs:260-265`; `FOLLOWUPS.md:33-41` (`p2-pref-over-ti-clamp`) | **CLOSED** (the "document" remedy r1 offered; exact fix scheduled to P3, its owning phase) |
| **M2** Sch-B sum duplication | reuses `sum_taxable_interest`/`sum_ordinary_dividends` | `return_1040.rs:310-315` | **CLOSED** |
| **M3** double RI fetch | `resolve_and_screen` fetches once | `resolve.rs:154-177` | **CLOSED as written** — but the restructure creates new finding **N2** |
| **M4** kiddie over-refuse direction | direction comment at the screen | `return_1040.rs:156-163` ("can only OVER-refuse … Do NOT 'fix' by subtracting…") | **CLOSED** |

Also verified: `FOLLOWUPS.md` correctly reassigns `p1-r1-m3-dob-option-pin`
P2→P3 with an accurate "nothing in P2 reads `dob`" justification (checked:
`derive_tax_profile` uses basic std only; the kiddie screen keys on
`can_be_claimed_as_dependent_taxpayer`, a bool).

---

## 3. C1 deep verification (the biggest change)

**(a) Union coverage.** `Session::resolve_all_screened`
(`session.rs:489-501`) enumerates `tax_profile::all(…).into_keys() ∪
return_inputs::all(…).into_keys()` and resolves each through
`resolve_screened` → `resolve_and_screen` — the SAME entry point `report`,
`optimize`, `what-if` and CLI `export` use. Errors propagate via `?` (nothing
swallowed). ✓

**(b) Two-liabilities scenario.** A year with BOTH a stored profile and
`ReturnInputs` enters the union once; `resolve_and_screen` hits the
`ReturnInputs` arm first (`resolve.rs:156`), so `Snapshot.profiles[year]` is
the DERIVED profile — the Tax tab (`tabs/tax.rs:62-63`), what-if panel
(`whatif_panel.rs:252`) and export (`export.rs:89,141`) all now compute from
the same figure the CLI derives. Verified by construction; **not pinned by any
test through the live path** — see N2. ✓ (code) / ✗ (KAT)

**(c) Refused year fail-closed in all three consumers.**
- Tax tab: `render_tax_content` early-returns
  `NOT COMPUTABLE (full-return inputs): {reason}` (`tabs/tax.rs:59-61`)
  BEFORE `compute_tax_year` AND before the SE section (`:127`) and advisories —
  the early return covers the whole body. KAT
  `tax_tab_refused_full_return_year_renders_reason_not_a_number`
  (`tabs/tests.rs:1700-1718`) pins: reason shown, no "TOTAL federal tax
  attributable", no "Schedule SE", other years unaffected. ✓
- What-if panel: refused check (`whatif_panel.rs:244-249`) fires BEFORE the
  placeholder substitution (`:252-261`), so a refused year gets the reason,
  never a placeholder-based number — matches the CLI fallback path
  (`resolve_screened_profile` ⇒ `Usage` error). ✓
- Export: a refused year is absent from `profiles` ⇒ `se_result_for`
  (`export.rs:89`) and `do_export` (`:141-157`) emit no `schedule_se.csv`; the
  other three CSVs are ledger-only. Identical to the CLI `export` mapping r1
  already blessed (`cmd/admin.rs:92,252`). ✓
- Year navigation: `[`/`]` step `selected_year` freely (`lib.rs:236-240`), so a
  refused year IS reachable and renders its reason. ✓

**(d) Exhaustive consumer sweep for remaining raw/unresolved reads.**
Grepped `all_tax_profiles`, `.profiles`, `snap.profiles`, `compute_tax_year`,
`compute_se_tax` across `btctax-tui` and `btctax-tui-edit`:
- `all_tax_profiles()` now has ZERO production callers (definition
  `session.rs:504` + doc mention only). ✓
- All viewer engine calls are the three guarded sites above. ✓
- **One missed consumer remains:** `btctax-tui-edit/src/main.rs:713`
  (`open_profile_form`) reads `snap.profiles.get(&year)` to pre-populate the
  raw-profile EDIT form — and tui-edit builds its snapshot with the very same
  `btctax_tui::unlock::build_snapshot` (`editor.rs:91`). The fold updated
  tui-edit's test constructors (the mechanical `refused:` field additions) but
  never looked at this production read-site, whose contract is the opposite of
  the viewer's: it must show the STORED raw profile (it is the edit form for
  `tax_profile::set`), not the resolved one. → **N1**.

---

## 4. C2 re-derivation (§221 QSS)

`FullReturnParams::student_loan_phaseout` (`tables.rs:264-271`): `Mfs → None`,
`Mfj → married ($165k–$195k)`, `Single | HoH | Qss → unmarried ($80k–$95k)`
(constants verified at `adapters/tax_tables.rs:134-135`). Hand re-derivation
through `student_loan_deduction` (`return_1040.rs:192-215`): QSS, $2,500 paid,
MAGI $120,000 → range (80,000, 95,000), `magi >= hi` ⇒ **$0** (was $2,500
pre-fold — the understatement is gone). At $60,000 ⇒ full $2,500. Both pinned
in `student_loan_phaseout_and_mfs_zero` (`return_1040.rs:607-617`), green.

**No §63(c)(2) regression:** `std_deduction_for` (`tables.rs:253-256`) still
routes through `TaxTable::key`, which maps `Qss → Mfj` (`tables.rs:85-89`) —
correct, since "surviving spouse" IS statutorily in the joint bucket for the
standard deduction, unlike §221. The doc comment at `tables.rs:259-263` now
records exactly this distinction and cross-cites the §904(j) precedent. ✓

---

## 5. I1 verification (Schedule B Part III)

`schedule_b_part3_unanswered` (`return_1040.rs:321-323`) now checks
`foreign_accounts.is_none() || foreign_trust.is_none()` (7a AND 8), gated on
`schedule_b_files`. It is **wired**: `screen_inputs` calls it at
`return_refuse.rs:304-310` as `RefuseReason::ScheduleBPart3Unanswered`
(`:50`), ordered AFTER the `foreign_trust == Some(true)` → `ForeignTrust`
refuse (`:295-300`) so an affirmative trust still gets its specific reason, and
before the accumulating rows. Trace: `screen_inputs` runs inside BOTH
`resolve_profile` (`resolve.rs:92`) and `resolve_and_screen` (`resolve.rs:164`),
so every consumer refuses. No over-refuse: fires only when Schedule B files
(interest/dividends > $1,500 or `foreign_accounts == Some(true)`); a modest
return with `None` tri-states stays clean (pinned by the `not_filing` case in
`schedule_b_part3_none_is_fail_loud_only_when_filing`), and a fully-answered
Part III passes (`schedule_b_part3_unanswered_refuses`,
`return_refuse.rs:656-671`; `clean_return_is_not_refused` updated at
`:488-493` to answer Part III once its dividends cross the threshold — the
right fix, not a test weakening). ✓ Closed.

---

## 6. NEW findings introduced by the fold

### IMPORTANT

**N1 — The tui-edit profile form is a missed consumer of the re-semanticized
`Snapshot.profiles`: it pre-populates a raw-profile EDITOR with DERIVED values
and can silently clobber the stored escape-hatch profile.**
`crates/btctax-tui-edit/src/main.rs:701-731` (`open_profile_form`) pre-fills
the form from `snap.profiles.get(&year)` (`:713`), documented as "the `--show`
equivalent" (`:697`). Pre-fold that map was the raw stored map, so open→save
was value-preserving. Post-fold the editor's snapshot comes from the same
`build_snapshot` (`editor.rs:91`) whose `profiles` is now the RESOLVED map:

- **ReturnInputs year + stored raw profile** (reachable: profile stored before
  `income import` — nothing guards that order — or via `--force`): the form
  displays the DERIVED profile as if it were the stored one; Enter→Enter runs
  `persist_tax_profile` (`edit/persist.rs:97-106`), silently overwriting the
  raw escape-hatch profile with machine-derived values. The editor has **no
  D-4 guard** — zero `return_inputs` awareness anywhere in `btctax-tui-edit`
  (grep-verified) — while the CLI refuses exactly this write without `--force`
  (`cmd/tax.rs:28-33`), and CLI `tax-profile --show` (`cmd/tax.rs:39-45`)
  still shows the STORED profile, so the two "show" surfaces now disagree.
- **Refused ReturnInputs year + stored raw profile:** the year is in `refused`,
  absent from `profiles` → the form opens EMPTY, displaying "no profile" while
  one is stored.

Concrete failure: 2024 has RI plus a deliberately different stored raw
override; user opens the editor form (`p`), confirms the (derived-looking)
payload, the override is gone; later `income clear --year 2024` makes the
stale derived copy the live liability source — values the user never entered,
frozen at an old snapshot of the RI. No liability is wrong WHILE the RI
precedence shields it, so this is not Critical — but it is a fold-introduced
data-integrity regression on the vault-mutation surface, unrecorded and
untested. Fix: pre-populate from the STORED profile (the editor holds a live
`Session` — `session.tax_profile(year)`), and give the editor save path the
D-4 refuse (or an explicit RI-exists warning) to match `cmd/tax.rs`.

**N2 — The M3 restructure de-linked the SPEC §4.12 precedence invariant from
its KAT: the only stored-vs-ReturnInputs precedence test now pins
production-dead code, and the precedence ladder exists in two copies.**
`resolve_and_screen` (`resolve.rs:156-177`) now carries its OWN inlined copy of
the ReturnInputs arm (fetch → tables-else-uncomputable → `screen_inputs` →
`screen_compute_dependent` → derive) and falls through to `resolve_profile`
only when NO `ReturnInputs` exist (`:180`). Every production consumer routes
through `Session::resolve_screened` → `resolve_and_screen` (grep-verified:
`resolve_profile` has no other production caller), so `resolve_profile`'s RI
arm (`resolve.rs:82-104`) is unreachable outside unit tests — yet the ONLY
test of RI-beats-stored precedence,
`return_inputs_beats_stored_profile_and_derives_a_profile`
(`resolve.rs:268-290`), exercises exactly that dead arm. Nothing in the
workspace stores a profile AND ReturnInputs for the same year through the live
path (grep-verified across `tests/tax_report.rs`, `tests/tax_profile.rs`, all
`return_inputs::set` call sites). Concrete failure scenario: a refactor
reorders `resolve_and_screen` to consult the stored profile before
`ReturnInputs` (or botches a "simplification" of the now-duplicated ladder) —
the ENTIRE 1460-test suite stays green while `report`/TUI/optimize silently
regress to the stale-profile cardinal sin C1 was about; the dead-copy KAT
keeps giving false assurance. The module's own contract ("must resolve through
ONE function", `resolve.rs:3-4`) is now structurally false — two pub functions
each implement the precedence ladder. Fix (either): re-unify (e.g. an internal
`resolve_with(ri: Option<ReturnInputs>, …)` both fns share, preserving the M3
single-fetch), and/or add a precedence KAT on `resolve_and_screen` plus an
integration KAT (`report` with both sources for one year ⇒ derived figure).
Cheap; but without it the phase's central invariant is unpinned.

### MINOR

**N3 — `resolve_all_screened` deserializes every `ReturnInputs` blob just to
enumerate keys, making one corrupt row brick the whole viewer.**
`session.rs:495` calls `return_inputs::all` (which JSON-parses every year's
blob, `return_inputs.rs:76-92`) only for `.into_keys()`; a single malformed
blob (`BadConfigValue`) now fails `build_snapshot` entirely — the viewer AND
editor cannot open the vault at all, where pre-fold they opened and only that
year failed on demand. Fail-closed direction (no wrong number), but a
whole-vault availability regression from one bad side-table row. Also
per-year, `resolve_screened` re-reads the config and re-loads
`BundledFullReturnTables` (`session.rs:459-460`) inside the loop — N redundant
loads. Fix: a `years()` helper (`SELECT year FROM …`) for both side-tables;
hoist config/tables out of the loop; optionally map a per-year load error into
`refused` instead of failing the snapshot.

**N4 — (pre-existing, recorded for completeness, non-blocking) Pseudo-mode
years remain un-resolved in the viewer.** In pseudo mode the CLI computes a
placeholder for ANY year (`resolve.rs:114-120`), but `resolve_all_screened`
enumerates only stored∪ReturnInputs years, so the Tax tab still shows
`TaxProfileMissing` for a pseudo-only year the CLI `report` computes. This
predates P2 (the pre-fold snapshot had the same gap) and the what-if panel's
own single/$0 placeholder (`whatif_panel.rs:252-261`) happens to match the
pseudo placeholder, so no two NUMBERS diverge — but it is a number-vs-refusal
divergence between two consumers of the single resolver. Not a fold defect;
worth a FOLLOWUPS note so the P4 provenance-rendering work owns it
deliberately.

---

## 7. Fold hygiene checked

- The r1 review is persisted verbatim (`IMPL-P2-fable-review-r1.md`, in the
  fold commit) before folding — §2-compliant. ✓
- No frozen-file drift anywhere in the fold (`git diff 8eb7118..5769dca` stat:
  no `types.rs`/`compute.rs`/`se.rs`). ✓
- `Snapshot.refused` is documented at the struct (`app.rs:105-115`) and every
  test constructor updated exhaustively (compile-enforced by the new field). ✓
- Deferrals D1/D2 untouched by the fold; not re-litigated. ✓

---

## Verdict: NOT GREEN

**0 Critical / 2 Important / 2 Minor.** All seven r1 findings are genuinely
closed — the resolver now reaches the viewer, QSS §221 is correct to the
dollar, and the Part III fail-loud actually fires — but the fold introduced
two Important defects of its own: the tui-edit profile form now shows/saves
DERIVED values where its contract is the raw stored profile (N1, a
vault-mutation data-clobber path), and the M3 restructure left the SPEC §4.12
precedence invariant pinned only on production-dead code (N2). Both are small,
targeted fixes; re-review required after the fold.
