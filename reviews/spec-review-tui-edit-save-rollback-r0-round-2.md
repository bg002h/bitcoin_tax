# R0 spec review — `SPEC_tui_edit_save_rollback.md` (round 2, verification)

**Artifact:** `design/SPEC_tui_edit_save_rollback.md` (re-read in full). Baseline `main` @ `45b9332`.
**Scope:** verify the round-1 folds (I1/I2/M1/M2/N1) resolve their findings and introduce no new
drift/contradiction.

## Verdict: 0 Critical / 0 Important / 1 Minor / 2 Nit → **R0-GREEN (0C / 0I)**

All four blocking-tier findings from round 1 (2 Important + 2 Minor) are resolved and correctly
grounded in source. One Minor and two Nits were introduced by the folds; none block the gate.

---

## Round-1 findings — resolution audit

### [I1] RESOLVED — the false enforcement claim is gone; the latch write is now single-homed + tested

The three moving parts all landed and cohere:
- **The false claim is deleted.** D3 no longer says "the compiler forces every Enter arm to handle
  it." The new framing (D3 ¶1, Hard-Constraint 6 at lines 84-86) is accurate.
- **`PersistError: Display` is forbidden** (Hard Constraint, line 84) — `Debug`-only, so a lazy
  `format!("{e}")` on the enum won't compile.
- **The latch write is single-homed.** `rollback_failed = true` now appears in exactly ONE
  production site — `EditorApp::on_persist_error`'s `ResidueLive` arm (D4, line 279) — which the spec
  states explicitly (line 294). `on_persist_error` is a single `match e` that is **exhaustive** over
  the three variants with no catch-all, so once a consumer routes into it the compiler does force the
  `ResidueLive` arm. All 8 Enter arms delegate via `Err(e) => { app.<modal> = None;
  app.on_persist_error(e); }` (line 290).
- **The producer path is now tested.** D5 (lines 340-346) adds the load-bearing test: hand-build
  `PersistError::ResidueLive(<any CliError>)`, call `on_persist_error`, assert `rollback_failed ==
  true` + CRITICAL status + all surfaces closed; plus `NoChange`/`RolledBack` leave the latch false
  and keep the form open. The consumer/opener KAT is retained (lines 348-352).

**Does `on_persist_error` close the gap (no way to arm the latch except through it)?** Yes, for the
specified design. The only production writer of `rollback_failed` is `on_persist_error`; it is
exhaustive, `Display`-less, and unit-tested. The keep-open vs close-all behavior is consistent
(NoChange/RolledBack → status only, form stays open per the shipped pattern; ResidueLive →
`close_all_mutation_surfaces()`). The lone residual is ordinary implementation-fidelity risk — an
implementer could *deviate* from the specified arm shape and destructure the inner `CliError` inline
(which has `Display`), bypassing the handler. That is a spec deviation caught by the Task-4 whole-diff
review, not a design defect. (Downgraded to Nit N2 below, as a review checklist item.)

### [I2] RESOLVED — the contradicting tax-profile doc header is now in the rewrite list

D3's "Doc-comment rewrites (required)" now lists `persist_tax_profile:30-35` first (lines 219-221),
quotes the offending "do NOT roll back the side-table" line, tags it `[R0-I2]`, and specifies the
replacement ("reverted on a failed save via `save_or_rollback`"). This is consistent with
Hard-Constraint 4 (tax-profile INCLUDED, lines 78-81) and the universal invariant. No contradiction
between the new tax-profile rewrite and the rest of the design. (One stale *count* elsewhere — see
M-new.)

### [M1] RESOLVED — both `From` impls present; `From<CoreError>` is correct for `append_decision`

D3 now defines **both** `From<CliError>` and `From<CoreError> for PersistError` (lines 170-175), with
an explanatory comment (lines 167-169) that `append_decision`/`load_all` yield `CoreError` and Rust
won't chain. Verified against source:
- `append_decision` returns `Result<EventId, btctax_core::CoreError>` (`persistence.rs:238-262`) — so
  `From<CoreError>` is exactly the type the bare `?` needs. ✓
- `From<CoreError> for CliError` exists (`btctax-cli/src/lib.rs:22`, `Core(#[from] CoreError)`), so
  `NoChange(e.into())` (CoreError→CliError→NoChange) compiles. ✓
- `btctax_core::CoreError` is public at crate root (`btctax-core/src/lib.rs:43`). ✓
- Semantics correct: an `append_decision`/`load_all` failure means the inner transaction never
  committed → no residue → `NoChange` is the right variant. Neither `From` targets `ResidueLive`. ✓
- Coverage complete: the persist fns `?`-propagate only `CliError` (session wrappers) and `CoreError`
  (core appenders); no raw `StoreError` path, so no third `From` is needed. The inline comment on the
  append line is fixed to "CoreError ⇒ NoChange" (line 199). ✓

### [M2] RESOLVED — `restore(` token specified as runtime-constructed, with the false-positive note

Hard-Constraint 2 (lines 66-74) now specifies runtime construction (`format!("{}(", "restore")`,
mirroring the shipped `tok_save`/`tok_conn`), records that `restore_terminal` is `restore_` (not a
`restore(` match), and confirms `snapshot(` stays **ungated** because `build_snapshot(` at
`main.rs:3691` contains that substring. All three points match the round-1 findings and current
source.

### [N1] RESOLVED — the four silent headers get an optional one-line note

D3 lines 225-228 add the optional `[R0-N1]` note for `persist_classify_inbound:46-59`,
`reclassify_income:115-125`, `set_fmv:142-152`, `donation_details:253-257`. Correct and marked
optional.

---

## New observations introduced by the folds

### [M-new] MINOR — Plan Task 2 still says "rewrite the **3** stale doc comments"; D3 now requires **4**

`design/SPEC_tui_edit_save_rollback.md:365` ("rewrite the 3 stale doc comments") is stale: the I2 fold
added `persist_tax_profile` as a 4th **required** rewrite in D3 (lines 219-224 list four:
tax_profile, reclassify_outflow, select_lots, void). An implementer executing the Plan literally could
rewrite only three and miss `persist_tax_profile:30-35` — re-introducing exactly I2. Non-blocking
because D3 is the authoritative design section and flags the 4th with an explicit `[R0-I2]` callout,
but it must be reconciled. **Fix:** change "the 3 stale doc comments" → "the 4 stale doc comments" at
line 365.

### [N1-new] NIT — the review-status line is stale

Line 5 still reads "Review status: DRAFT — awaiting R0 round 1." After the round-1 fold this should
read round-2 (e.g., "awaiting R0 round 2" / "R0 round-1 findings folded"). Doc hygiene only.

### [N2-new] NIT — Hard-Constraint 6 wording slightly overstates; add a whole-diff checklist item

"forcing every consumer through `on_persist_error`" (line 85) is the design *intent*, not a hard
compiler guarantee — the no-`Display` rule only blocks the lazy `format!("{e}")`; an implementer could
still destructure the inner `CliError` inline and bypass the handler. The mitigations (single tested
handler, exhaustive match, specified delegation shape) are sound. Recommend Task 4 explicitly verify
**all 8 Enter arms delegate to `on_persist_error`** (none destructure `PersistError` inline). Not a
design defect.

---

## Spot-checks that stayed sound (no regression from the folds)

- The two CRITICAL residue strings (`residue_latch_status`, lines 249-251, vs `on_persist_error`
  ResidueLive, lines 280-283) differ only by the `({err})` detail — both contain "CRITICAL" / "Quit
  the editor NOW"; the shipped ERRLATCH pattern uses `.contains(...)` substring asserts, so both the
  producer test and consumer KAT pass. No contradiction.
- The attest string in `residue_latch_status` (lines 245-247) remains byte-verbatim to the 9 shipped
  opener guards, so `kat_e2e_attest_errlatch_chmod` stays green (unchanged from round 1).
- `From<CliError>` and `From<CoreError>` are non-overlapping (distinct source types), both → NoChange;
  no coherence conflict.
- The "rewrite exactly 4 residue KATs" set (D5, line 332) is unchanged and still complete (round-1
  verification holds: KAT-S1 side-table, REPOINT/VOID-RETRY/ATTEST-HAPPY are legitimate
  double-success, not residue).

---

**Gate: R0-GREEN. 0 Critical / 0 Important.** The 1 Minor (Plan "3"→"4") and the 2 Nits (stale status
line; whole-diff delegation checklist) are non-blocking; fold at convenience — the "3"→"4" edit is a
one-word change worth making before implementation so the Plan can't silently undo the I2 fix.
