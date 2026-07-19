# UX-P4-3 (#14) implementation review — r3 (Fable, independent/adversarial, closing gate)

**Scope:** the r2 fold — commits `158b040` (persist r2, review file only, verified) + `2c0940d` (fold
r2-I1 + M1 + N2) — on the whole-#14 diff `666a868..HEAD` (verified byte-identical to the saved
`p14-r3.diff`). Task: verify the one r2 blocker (r2-I1: the 4th mandated KAT — `classify-raw` on an
accept-governed target refused `[R3-I1]` — absent, with a proven one-writer-short surviving mutant)
is closed with no drift, and rule GREEN or not. r1 established `would_conflict` is definitionally
correct; r2 closed I2 and three-quarters of I1; neither was re-derived, both were drift-checked.

**Method (evidence, not assertion):** full read of the fold diff (4 files: FOLLOWUPS +17, a 2-line
message alignment in `reconcile.rs`, +18 test lines, 1 detail line in `resolve.rs`) and of the
current `record_time_validation.rs` (15 KATs), `resolve.rs` passes 1a–1e, and all seven
`guard_decision_conflict` sites. Baseline `make check` run myself: **2021/2021 pass** + clippy
`-D warnings` + `cargo fmt --all --check` clean. **Mutation G** (delete the `classify_raw` guard,
`reconcile.rs:472`) → 13/15: `duplicate_classify_raw_is_refused` AND
`accept_governed_supersede_import_income_is_effective_income` both RED. **Mutation C rebuilt**
(r2's exact one-writer-short pass-1c: duplicate check against a ClassifyRaw-only `raw_classified`
set, ignoring `SupersedeImport`-written `applied` entries, ClassifyRaw overwriting the accept) →
14/15: the accept-governed KAT REDS at **`record_time_validation.rs:433`** — the folded assertion
precisely (`unwrap_err()` on `Ok`, i.e. the mutant silently ACCEPTED the classify-raw over the
user's permanent accept) — while `duplicate_classify_raw_is_refused` stays GREEN. Tree restored
byte-clean after each experiment via cp-backup/restore (`cmp` + `git status` clean; never
`git checkout`).

---

## Critical

None. Closing-gate sweep (all clean):

- **r2-I1 is CLOSED with teeth — the proven survivor is dead, killed by exactly the fold.** The
  accept-governed KAT (`record_time_validation.rs:376-445`) now ends with the mandated 4th case:
  `classify_raw` on the accept-governed `in_ref` → `unwrap_err()` contains "already classified"
  (`:434-439`) + `count(ClassifyRaw) == 0` fail-closed (`:440-444`). Under my faithful
  reconstruction of r2's Mutation C the ONLY red in the file is this KAT, failing at `:433` — the
  new assertion, not a side effect — and `duplicate_classify_raw_is_refused`
  (`record_time_validation.rs:340`, ClassifyRaw prior) stays green under the same mutant, so the
  two KATs now discriminate the two `applied` writers (`resolve.rs:524` SupersedeImport-accept vs
  `resolve.rs:573` ClassifyRaw): the SupersedeImport-written entry is pinned through pass-1c's
  duplicate check (`resolve.rs:562`), no longer only through the pass-1d/1e type-read path. Both
  refusal directions also pin the CLI wiring: Mutation G (guard deleted) reds the accept-governed
  KAT too. All four SPEC §3.2 accept-governed/`[G2-6]` mandated KATs now exist and each has a
  demonstrated kill.
- **The accept-governed KAT is genuinely non-vacuous** (re-confirmed by read, r2 verified
  empirically): a real `ImportConflict` is minted (same-id competing import, existence asserted
  with `expect`, `:402-412`) and really accepted (`:414`); `set-fmv` ACCEPTED on the target
  discriminates (the identical fixture's raw TransferIn REFUSES set-fmv in
  `set_fmv_on_non_income_is_refused`, `:232`); `classify-inbound` wrong-type-refuse reads the
  EFFECTIVE payload; and the new `count(ClassifyRaw)==0` is meaningful — no ClassifyRaw is ever
  recorded earlier in the test.
- **No logic drift.** The fold is strings + one test extension + FOLLOWUPS only: `resolve.rs`
  changed one `detail:` string expression inside pass-1c (`resolve.rs:568`) — blocker kind, blamed
  event, and control flow byte-identical; `reconcile.rs` changed only the already-voided message
  text (`reconcile.rs:284-288`); the test file gained 18 inserted lines at the end of one KAT, zero
  modified — no pre-existing KAT weakened (baseline 15/15 verified). `would_conflict` and all
  seven guard sites (`reconcile.rs:115/174/225/270/472/1300` + the fn at `:46`) untouched. The
  `resolve.rs` change is value-inert for `would_conflict` by the baseline-diff argument (the
  `(event, detail)` diff key is computed by the SAME resolver on both sides, so a text change
  affects both sides identically) — and empirically 2021/2021 with every golden byte-stable and
  `git status` clean. §1 read-only, fail-closed wiring, and NFR4 determinism stand as r1/r2
  verified them.
- **No stale or duplicated phrasing survives, and nothing string-matches the changed text.** The
  old pass-1c suffix ("void the prior decision to re-decide") now exists only on the four arms
  where it is unconditionally valid — `resolve.rs:718` (inbound_class), `:772` (outflow_class),
  `:831` (income_reclassify), `:895` (passthrough_skip) — each a map written ONLY by revocable
  deciders (pass-1a's catch-all `resolve.rs:462`; only SupersedeImport/RejectImport/Void are
  non-revocable). The old already-voided phrasing ("Run `btctax events list` to see event refs +
  their decision status.") is gone from source; the aligned message still satisfies the
  `void_already_voided_is_refused` matcher ("already voided", `record_time_validation.rs:277`).
  No test, golden, TUI string, or generated doc matches "if the prior decision is revocable" or
  the resolver's pass-1c detail (repo-wide grep; the TUI's "already classified" hits are its own
  comments/assertion labels).

## Important

None.

## Minor

None. (r2-M1 verified closed: `resolve.rs:568` now reads "if the prior decision is revocable, void
it to re-decide" — accurate on BOTH surfaces whether the occupant is a revocable ClassifyRaw or a
non-revocable SupersedeImport-accept, applied ONLY to the classify-raw duplicate arm — the sole arm
whose backing map (`applied`) has a non-revocable writer — with the other four duplicate arms'
unconditional hint correctly untouched. The new KAT pins the arm's text via its "already
classified" matcher. r2-N2 verified closed: `reconcile.rs:285-286` now mirrors the `CONFLICT_HINT`
wording.)

## Nit

**N1-r3 — The hint-policy comment above `CONFLICT_HINT` is now one arm inexact.**
`resolve.rs:414-415` still says `Duplicate details add "; void the prior decision to re-decide"` —
true for four of the five duplicate arms, but the pass-1c arm now adds the conditional variant
(the M1 fold). One-line comment touch-up; fold opportunistically.

**N2-r3 — Carry-forwards confirmed filed with owners** (no action here): r2-N1 (stale
`cli.rs` reclassify-income help/man describing the record-then-surface flow) is in `FOLLOWUPS.md`
with owning phase docs/#21; r2-N3 (non-§3.2 emitters' bare details) filed for a later cycle
alongside the UX-P4-11 events-list M1. Both consistent with the per-phase burndown rule.

## Verdict

**GREEN — 0 Critical / 0 Important.**

The r2-I1 fold closes the finding exactly and with proof: the 4th mandated KAT exists at the end of
the accept-governed test, and the precise one-writer-short pass-1c mutant that r2 proved survived
the entire 2021-test suite now dies at the folded assertion (`record_time_validation.rs:433`) while
the plain ClassifyRaw-prior duplicate KAT stays green — the two `applied` writers are separately
pinned, in both the resolver direction (Mutation C) and the CLI-wiring direction (Mutation G reds
the same KAT). M1 and N2 rode along correctly and touched nothing else; the whole fold is strings +
tests + FOLLOWUPS with zero logic change, no golden or matcher disturbed, and the full validation
surface is green under my own run (2021/2021 nextest + clippy `-D warnings` + fmt). r1's
definitional-correctness verdict on `would_conflict` and r2's I2 closure stand undisturbed. #14 is
done: two Nits recorded (one new one-line comment nit; two carry-forwards already filed with owning
phases), nothing gates.
