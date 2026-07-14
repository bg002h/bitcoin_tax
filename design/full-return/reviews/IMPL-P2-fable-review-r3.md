# IMPL-P2 — Fable independent code review, round 3 (re-review of the r2 fold)

**Scope:** full-return Phase 2, branch `full-return`, HEAD `eed852e`. Fold diff
reviewed: `5769dca..eed852e` (one commit, `eed852e` "fold Fable code review r2
(2I/2M) — P2 r3"; touches exactly 8 files: `resolve.rs`, `return_inputs.rs`,
`session.rs`, `tax_profile.rs`, `edit/persist.rs`, tui-edit `main.rs`,
`FOLLOWUPS.md`, and the verbatim r2 review). Whole-P2 context re-checked
(`059ec2a..eed852e`) where the fold touches it.
**Reviewer:** Fable (independent; author was a different model).
**Date:** 2026-07-12.

**Verdict: GREEN — 0 Critical / 0 Important / 4 Minor.**
Both r2 Important findings (N1, N2) are genuinely closed, N3 is fixed, N4 is
accurately recorded, and all seven r1 findings remain closed. The four new
Minors are test-gap/defensive-hardening items on the fold's own fixes — none
changes a number, none is fail-open on any reachable path.

---

## 1. Verification actually run (real output)

`cargo test --workspace` (full suite, exit 0):

```
ok-lines: 81   TOTAL passed: 1462   failed: 0
EXIT=0
$ cargo test --workspace 2>&1 | grep "test result:" | grep -v "^test result: ok" | wc -l
0        ← every one of the 81 `test result:` lines is `ok`
```

(1462 = r2's 1460 + the two new named tests below; KAT-F1 was reworked in
place, not added.)

`cargo clippy --workspace --all-targets` (tail; `grep -cE "^(warning|error)"` = **0**):

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.50s
EXIT=0
```

Frozen-engine byte-identity across the WHOLE phase (P1-GREEN → r3 head):

```
$ git diff 059ec2a..eed852e -- crates/btctax-core/src/tax/types.rs \
    crates/btctax-core/src/tax/compute.rs crates/btctax-core/src/tax/se.rs | wc -c
0
$ cargo test -p btctax-core --lib frozen_guard
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 143 filtered out; finished in 0.00s
```

(The fold itself touches no `btctax-core` file at all — `--stat` above.)

The fold's new/reworked KATs, run individually:

```
test resolve::tests::resolve_and_screen_gives_return_inputs_precedence_over_stored ... ok
test edit::persist::tests::persist_tax_profile_refuses_when_return_inputs_exist_d4 ... ok
test tests::kat_f1_p_opens_form_prepopulated_from_existing_profile ... ok
```

---

## 2. Per-r2-finding verification

| r2 | Claimed fix | Verified at | Status |
|---|---|---|---|
| **N1** (Important) tui-edit form shows/saves derived values, no D-4 guard | form reads raw stored profile via live `Session`; `persist_tax_profile` refuses when RI exist | §3 below | **CLOSED** (with two Minor test/robustness notes, §5.1–5.2) |
| **N2** (Important) precedence ladder duplicated; KAT pinned dead code | shared private `resolve_core`; both entry points delegate; new KAT on the LIVE `resolve_and_screen` path | §4 below | **CLOSED** (one Minor hardening note, §5.3) |
| **N3** (Minor) key enumeration deserializes every blob; corrupt row bricks viewer; per-year config/table reloads | `years()` helpers (`SELECT year` only); loads hoisted; per-year `Err` → per-year `Uncomputable` | §6 below | **CLOSED** (mapping itself untested — §5.4) |
| **N4** (Minor) pseudo-year viewer gap (pre-existing) | recorded to FOLLOWUPS → P4 | `FOLLOWUPS.md:31-41` (`p2-r2-n4-pseudo-year-viewer-gap`) | **RECORDED** — accurate description, correctly scoped to the P4 provenance-render work |

---

## 3. N1 verification (editor pre-population + D-4 guard)

**(a) Pre-population source.** `open_profile_form`
(`crates/btctax-tui-edit/src/main.rs:702-736`) now reads
`app.session.as_ref().and_then(|s| s.tax_profile(year).ok().flatten())` —
`Session::tax_profile` (`session.rs:445-447`) is `tax_profile::get`, the RAW
stored side-table row. `snapshot.profiles` is no longer read anywhere in the
form path (grep: the only remaining tui-edit reads of `.profiles` are two test
sites, one being KAT-F3 which correctly asserts the viewer-semantic map). The
r2 sub-scenarios both resolve correctly now: an RI+stored year shows the
STORED override (matching CLI `tax-profile --show`, `cmd/tax.rs:39-45` — the
two "show" surfaces agree again); a refused-RI year with a stored profile
shows the stored profile, not an empty form. `session` is always `Some` when
the form is reachable: both unlock paths (`editor.rs:360-383` `do_unlock`,
`:388-409` env fast-path) set `session` and `snapshot` together before
entering `Browse`.

**(b) D-4 guard fires before any write.** `persist_tax_profile`
(`edit/persist.rs:97-117`): the `return_inputs::exists` check (`:106`) is the
FIRST statement — before `session.snapshot()` (`:113`) and before
`tax_profile::set` (`:114`) — returning
`PersistError::NoChange(CliError::Usage(...))`. Nothing is written, no
rollback needed. `return_inputs::exists` (`return_inputs.rs:58-66`) is the
pre-existing `SELECT 1` probe the CLI guard uses.

**(c) Consistency with the CLI.** `cmd/tax.rs` `set_profile` (`:28-33`)
refuses the same write unless `--force`; the editor has no force and its
message points at both recovery paths (`income clear --year N` or CLI
`tax-profile set --force`). The editor is strictly more conservative — the
correct direction for the mandated escape-hatch protection.

**(d) No bypass path.** `edit/persist.rs` is the ONLY module allowed to touch
the mutation surface (module doc `:1-2`), mechanically enforced by the
pre-existing KAT-G1 token-scan (`persist.rs:1730-2011` — `tax_profile::set`
in non-test code of any other file is a test failure). `persist_tax_profile`
has exactly one production call site: the confirmation modal
(`main.rs:480`). The guard therefore covers every editor profile write.

**(e) Guard is user-visible.** The `Err` arm of the modal handler
(`main.rs:507-512`) routes to `on_persist_error` (`main.rs:627-645`), whose
`NoChange` arm renders `"Save error: {err} — no changes were recorded; safe
to retry."` with the full D-4 Usage text — the user sees WHY, with the
recovery commands.

**(f) Legitimate edits still work.** The new KAT
`persist_tax_profile_refuses_when_return_inputs_exist_d4`
(`persist.rs:1092-1128`) seeds RI for 2025 through the REAL CLI import path,
asserts the 2025 write is refused AND nothing was stored
(`session.tax_profile(2025) == None`), then asserts a 2024 write (no RI)
persists normally. The pre-existing persist round-trip KATs
(`persist.rs:1048,1081`) still pass — no false-positive refusals.

**(g) KAT-F1 is no longer vacuous.** The reworked
`kat_f1_p_opens_form_prepopulated_from_existing_profile`
(`main.rs:9908-9981`) builds a REAL vault, stores the profile via CLI
`set_profile`, unlocks through `do_unlock` (real session), presses `p`, and
asserts all 9 field buffers + filing status equal the stored values. It
exercises the live `open_profile_form` → `session.tax_profile` path
end-to-end. (It cannot DISTINGUISH raw-vs-resolved for a stored-only year —
see Minor §5.1.)

**N1 CLOSED.**

---

## 4. N2 verification (the structural change — scrutinized)

**(a) ONE ladder.** `resolve_core` (`resolve.rs:77-136`) is the only place
the §4.12 precedence order exists: RI (unsupported-year fail-closed →
`screen_inputs` → derive) → stored → pseudo → missing. `resolve_profile`
(`:143-151`) is a one-line delegation dropping the RI;
`resolve_and_screen` (`:170-202`) calls `resolve_core` once and contains NO
precedence logic of its own — no `return_inputs::get`, no
`tax_profile::get`, no pseudo branch (verified by reading the whole
function; the only remaining `return_inputs::get` call in the file is inside
`resolve_core` at `:85`). The module contract ("must resolve through ONE
function") is structurally true again.

**(b) Identical precedence decisions.** Both public entry points get their
`Resolved` from the same `resolve_core` call — they cannot disagree on
source selection by construction. `resolve_profile`'s output is
byte-for-byte the pre-fold `Resolved` in all four arms (checked arm-by-arm
against the pre-fold code in the diff: same `profile`/`provenance`/`refusal`
in every return).

**(c) M3 preserved — same bytes.** `resolve_core` returns
`(Resolved, Option<ReturnInputs>)`; the RI arm returns `Some(ri)` on ALL
THREE of its exits (unsupported `:93`, refused `:102`, derived `:109`), the
other arms `None`. `resolve_and_screen` runs `screen_compute_dependent` on
`ri.as_ref()` (`:190-191`) — the very struct that was screened and derived;
there is no second fetch anywhere in the function.

**(d) The new KAT exercises the LIVE path.**
`resolve_and_screen_gives_return_inputs_precedence_over_stored`
(`resolve.rs:313-343`) stores BOTH a raw MFJ/$120k profile and Single/$100k
`ReturnInputs` for 2024, then calls `resolve_and_screen` itself (the
function `Session::resolve_screened` wraps — `session.rs:461` — i.e. the
entry point of report/optimize/what-if/export/TUI) with real TY2024 tables
and `pseudo=true`, asserting `Provenance::ReturnInputs`, the DERIVED Single
profile, and `!= prof()`. This is exactly the pin r2 demanded: reordering
the ladder to consult the stored profile first now fails a test on the
production path. (It also implicitly pins RI > pseudo.) The old
`resolve_profile`-level KAT (`:285-307`) is retained and now ALSO pins live
code, since `resolve_profile` shares `resolve_core`.

**(e) No behavior change in the stored/pseudo/missing arms.** Post-`core`,
`resolve_and_screen` returns `Ready { profile, provenance }` for the three
non-RI arms — identical to the pre-fold fall-through. The
`is_return_inputs_uncomputable` gate (`resolve.rs:50-52`:
`provenance == ReturnInputs && profile.is_none()`) reproduces the two
pre-fold Uncomputable exits exactly: refusal `Some` → refusal detail;
refusal `None` → unsupported-year detail (`uncomputable_detail(year, None)`)
— it cannot fire for `Missing` (provenance differs). The pre-existing arm
KATs (missing / pseudo / stored-beats-pseudo / refused / unsupported-year)
all pass unchanged.

**(f) Consumers.** `resolve_profile` still has zero production callers
(grep: only doc-comment mentions in `cmd/tax.rs:18`, `persist.rs:103`, and a
test-file comment) — its behavior is unchanged anyway (same `Resolved`).
Every computing consumer still routes through `resolve_and_screen`
(`cmd/tax.rs:166`, `session.rs:461` + `:504`), unchanged.

**N2 CLOSED** (one hardening note, §5.3).

---

## 5. NEW findings introduced by this fold

### MINOR

**M-r3-1 — KAT-F1 cannot detect a regression of the N1 pre-population fix:
no test covers a BOTH-sources year at the form layer.**
For KAT-F1's stored-only year, the resolved map and the raw map hold the SAME
values (the stored arm returns the profile unchanged), so reverting
`open_profile_form` (`main.rs:717-720`) to read `snapshot.profiles` would
leave the entire 1462-test suite green. The distinguishing scenario — one
year with BOTH `ReturnInputs` and a stored raw override, form must show the
STORED values — exists only in the resolver KAT, not at the form. Residual
risk is bounded: the clobber half of r2-N1 is independently pinned (the D-4
persist KAT refuses the write regardless of what the form displays), so a
display regression is fail-closed user confusion, not data loss — hence
Minor, not Important. Fix: one KAT-F1 variant that seeds RI + a different
stored profile for the year and asserts the form buffers equal the STORED
values.

**M-r3-2 — `open_profile_form` swallows a vault read error as "no stored
profile".** `main.rs:717-720`: `s.tax_profile(year).ok().flatten()` maps a
`CliError` (e.g. a corrupt `tax_profile` blob — `bad_json_is_a_typed_error…`,
`tax_profile.rs:138`) to an EMPTY form, silently presenting a
present-but-unreadable profile as absent. No wrong number is possible (the
save path is guarded and atomic; the tabs independently show that year as
refused via the N3 mapping), and overwriting an unreadable row is arguably
recovery — but the editor is a mutation surface and a masked read error is
the wrong default under the house fail-closed rule. Fix: on `Err`, set
`app.status` ("could not read the stored profile for {year}: {e}") and open
the form empty (or refuse to open it).

**M-r3-3 — `resolve_and_screen`'s compute-dependent block degrades to a
silent skip if the `resolve_core` tuple invariant is ever broken.**
`resolve.rs:190`: `if let (Some(ri), Some(params)) = (ri.as_ref(), full_return)`
has no `else` — should a future edit make `resolve_core` return `ri: None`
from the RI arm (or a caller pass `full_return: None` with a derived
profile), the compute-dependent screen is skipped and `Ready` is returned:
fail-open by silent skip. Today the state is unreachable (all three RI-arm
exits return `Some(ri)`; a derived profile implies `params` was `Some`), and
it IS pinned end-to-end — breaking the threading fails
`report_tax_year_refuses_business_income_without_schedule_c` on the live
path (re-ran: passes) — so this is hardening, not a live defect. Fix: make
the broken state fail closed (an `else` returning `Uncomputable`, or
restructure `resolve_core`'s return so the RI arm carries the
`ReturnInputs` non-optionally, e.g. an enum arm `ReturnInputs { ri, resolved }`).

**M-r3-4 — the N3 per-year error mapping is itself untested.** Every LINK in
the chain has a test (corrupt blob → typed error: `return_inputs.rs:146-158`
and `tax_profile.rs` equivalent; `Uncomputable` → `refused` → rendered
refusal: the C1 viewer KATs), but no test injects a corrupt side-table row
and asserts `resolve_all_screened` yields a per-year `Uncomputable` while
OTHER years still resolve — the exact availability behavior N3 asked for
(`session.rs:501-517`). A regression (say, a refactor reintroducing `?` on
the per-year call) would surface only in a real corrupt vault. Cheap to pin:
in-memory conn, `INSERT … 'not json'` for one year, a valid profile for
another, assert one `Uncomputable` + one `Ready`.

---

## 6. N3 verification detail

`tax_profile::years` (`tax_profile.rs:65-70`) and `return_inputs::years`
(`return_inputs.rs:75-82`) are `SELECT year FROM … ORDER BY year` — no blob
column touched, so enumeration cannot fail on a corrupt row (both call
`init_table` first, so tableless older vaults stay OK).
`resolve_all_screened` (`session.rs:489-520`): `config()` +
`BundledFullReturnTables::load()` hoisted above the loop (`:496-497`,
eliminating the N redundant loads through `resolve_screened`); the per-year
`resolve_and_screen` `Err` is mapped to
`ProfileOutcome::Uncomputable { detail: "could not read the stored inputs for {year}: {e}" }`
(`:512-515`) — the viewer (`unlock.rs:183-195`) puts it in
`Snapshot.refused`, so the corrupt year renders its refusal and the vault
still opens; the CLI for the same year still hard-fails
(`resolve_screened_profile` ⇒ `Usage`) — both fail-closed, no number either
way. A systemic mid-loop error now renders as per-year refusals instead of an
unlock failure — fail-closed direction, acceptable for a read-only viewer.
Semantics vs the old code are otherwise identical (same `resolve_and_screen`,
same `pseudo`/tables values). **CLOSED** (test-gap noted as M-r3-4).

---

## 7. r1 regression check (must stay closed)

The fold touches no `btctax-core` file and none of the r1-fold sites
(`tables.rs`, `return_1040.rs`, `return_refuse.rs`, `tabs/tax.rs`,
`whatif_panel.rs`, `export.rs` — all absent from the fold `--stat`). Spot
re-runs, all green:

```
test tabs::tests::tax_tab_refused_full_return_year_renders_reason_not_a_number ... ok   (C1 viewer)
test tax::return_1040::tests::student_loan_phaseout_and_mfs_zero ... ok                 (C2 QSS)
test tax::return_1040::tests::schedule_b_part3_none_is_fail_loud_only_when_filing ... ok (I1)
test tax::return_refuse::tests::schedule_b_part3_unanswered_refuses ... ok               (I1)
test resolve::tests::return_inputs_beats_stored_profile_and_derives_a_profile ... ok     (precedence)
test report_tax_year_derives_and_computes_from_ty2024_return_inputs ... ok               (seam, live)
test report_tax_year_refuses_business_income_without_schedule_c ... ok                   (fail-closed, live)
test report_tax_year_with_return_inputs_for_unsupported_year_refuses_with_income_clear_hint ... ok
```

C1's viewer routing is strengthened, not weakened, by this fold (same
`resolve_and_screen` per year; errors now degrade per-year instead of
failing the snapshot). M1/M4 doc comments and the M2 dedup are untouched.
The r1 deferrals (D1 → P4, D2 → P4) and `p2-pref-over-ti-clamp` (→ P3)
records are intact in `FOLLOWUPS.md`. All seven r1 findings remain closed.

---

## 8. Fold hygiene

- The r2 review is persisted verbatim in the fold commit
  (`IMPL-P2-fable-review-r2.md`, 289 lines, byte-matches what I wrote) —
  same §2 pattern accepted at r2. ✓
- One fold commit (`eed852e`), accurate message (names N1/N2/N3 fixes and
  the N4 deferral). ✓
- `FOLLOWUPS.md` N4 record is faithful to the r2 finding (pre-existing,
  non-fail-open, scoped to P4 provenance rendering, with two concrete fix
  directions). ✓
- No frozen-file drift in the fold or across the phase (§1). ✓

---

## Verdict: GREEN

**0 Critical / 0 Important / 4 Minor.** N1 is closed on both halves (the form
reads the raw stored profile through the live `Session`; the D-4 guard
refuses the write before anything touches the vault, matches the CLI, covers
the editor's only write path, and is KAT-pinned including the
must-still-work no-RI arm). N2 is closed structurally and evidentially (ONE
ladder in `resolve_core`, both entry points delegate, the same fetched
`ReturnInputs` feed screen and derive, and the §4.12 precedence invariant is
now pinned on the exact function every production consumer calls). N3's
helpers, hoisting, and per-year degradation are as claimed; N4 is recorded.
The four Minors (a distinguishing-KAT gap for the form fix, an `.ok()` error
swallow, a silent-skip hardening in `resolve_and_screen`, and the untested
N3 mapping) are non-blocking — fold opportunistically or record; none
requires another gate round.
