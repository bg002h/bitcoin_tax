All verification complete. The tree is restored byte-identical to HEAD and the affected tests re-green. Here is my re-review.

---

RE-REVIEW VERDICT: 0 Critical, 0 Important, 0 Minor, 2 Nit

# Conservative-filing Phase-1 — focused re-review of fold `69fec06`

**Important resolved: YES** — verified in source and by killing the mutation myself.

**Verification actually performed (not assumed):** baseline `make check` green at `69fec06` (2117/2117 nextest + parallel clippy `-D warnings`, exit 0; 2113 at r1 + exactly the 4 new tests). I then ran all four mutations myself (cp-backup/restore to the scratchpad, never `git checkout --`; `git status` clean and `TREE-CLEAN` confirmed after restore, affected tests re-green):

1. **TUI arm removed** (revert to wildcard) → `summarize_void_payload_declare_tranche_is_human` **RED** (`tag` = `"?"` fails the `assert_eq!("DeclareTranche")`).
2. **CLI attest guard neutered** (`reconcile.rs:1199` call removed) → `attest_refused_under_a_pre2025_tranche` **RED**, failing with exactly the predicted fallback `"usage: no allocation to attest"` — proof the `.contains("tranche")` wording assert is the discriminator (both paths are `CliError::Usage`, so the `matches!` alone would NOT discriminate; the test is built correctly).
3. **TUI attest guard neutered** (`persist.rs:1105`) → `persist_safe_harbor_attest_refused_under_a_pre2025_tranche` **RED** via `unwrap_err()` on `Ok((Decision{seq:2}, Decision{seq:3}))` — without the guard the void + re-attest batch actually persists, so the test discriminates and additionally pins fail-closed (asserts zero `VoidDecisionEvent`/`SafeHarborAllocation` appended on refusal).
4. **Backstop neutered** (`resolve.rs` `estimated_conservative_remaining_sat > 0` → `> i64::MAX`) → exactly the two backstop KATs **RED** (`backstop_fires_when_the_allocation_is_recorded_before_the_tranche` + the original), all 13 other tranche KATs green — the new twin discriminates the backstop line specifically, not incidentally.

## 1. The Important

Resolved cleanly. The new arm (`crates/btctax-tui-edit/src/main.rs:3832-3843`) sits **before** the wildcard, renders `("DeclareTranche", "{sat} sat in {wallet} (window {start}..{end})", None, false)` — the exact CLI `bulk_void_payload_summary` sibling format (`btctax-cli/src/main.rs:2154`), split tag/summary in the established TUI shape. Both consumers (`open_void_flow` :3883, `open_bulk_void_flow` :8259) now get the human row. The 4th field (`is_safe_harbor`) is correctly `false`; note it is actually inert at both call sites (`_is_sha`) — the void modal derives the flag from `payload_tag == "SafeHarborAllocation"` (main.rs:3216), which also yields `false` for a tranche, so the two mechanisms agree. `target = None` is right (a tranche has no target event; matches `MethodElection`/`SafeHarborAllocation` precedent).

## 2. The folded Minors

- **Attest-guard tests:** both discriminating, mutation-proven above. The TUI test's dummy-`prior_alloc` approach is **legitimate**: the guard sits before any use of `prior_id`/`prior_alloc` in `persist_safe_harbor_attest`, which is the exact wired product function, and the test doubles as a fail-closed/no-half-applied-batch pin. (Incidentally the dummy `prior_id = decision(1)` names the tranche itself, which is what makes the neutered path complete and the `unwrap_err` discriminate.)
- **Inert-then-declare backstop KAT:** exact seq-swap twin (alloc seq 1, tranche seq 2 vs. the original's 1/2 reversed), same would-conserve fixture (100M sat/$0 alloc vs. 100M/$0 tranche residue) — so totals match the snapshot and `has_tranche_residue` is the **only** possible trigger; the SPEC-D-8-named ordering is now pinned and the mutation run confirms it discriminates.
- **Extended test (d):** correct. The no-`SafeHarborUnconservable`/`Timebar` assert catches the tax-r1-named regression (re-keying the backstop on payload presence would blocker the effective allocation → RED), and the coexistence assert (`EstimatedConservative` lot, `remaining_sat == 50_000_000`) can't conflate with the 20M-sat documented buy. It asserts blocker-absence rather than `TransitionMode::PathB` directly — a slightly weaker proxy than tax r1 suggested, but sufficient for the named failure mode since every allocation-inerting path in `resolve.rs` emits one of those two kinds.
- **Nits (hint split, lib.rs comment):** both direction-correct — `TRANCHE_IS_FINAL_HINT` (allocation-side, caveat about the tranche's filed $0 basis) and `ALLOCATION_IS_FINAL_HINT` (tranche-side, caveat about the filed allocation) each parse in their own direction; `IRREVOCABILITY_HINT` fully removed, no stale references or wording-coupled tests (grepped). The lib.rs re-export comment states the pure-predicate rationale and warns future additions — the review offered "move or comment"; comment is an acceptable resolution.

## 3. New issues introduced by the fold

Two Nits, nothing blocking:

- **[Nit] Message assembly starts a sentence lowercase** — `crates/btctax-cli/src/cmd/tranche.rs:76-81`: the allocation-side refusal now reads "…mutually exclusive. **v**oid the tranche first…" (period + lowercase hint start). The tranche-side sibling composes with a semicolon and reads fine. Cosmetic only.
- **[Nit] FOLLOWUPS.md is incomplete against the commit message's claim "the rest are filed"** — `design/conservative-filing/FOLLOWUPS.md` omits tax-r1 **Nit 2** (no test asserts `row.date_acquired == window_end` on an 8949 tranche row), **Nit 3** (no Σ-conservation assert over a projection containing a tranche), and the UTC-"today" residual from Nit 4. They were neither fixed inline nor filed with an owning phase; they survive only in the persisted review file, which is not the burndown-grep surface the workflow prescribes. Natural homes exist (Nit 3 belongs beside the P9/T15 invariant item; Nit 2 is a one-assert addition). Bookkeeping only — none gates — but the ledger should be corrected before the phase closes so they aren't silently lost.

## 4. The four deferrals — all legitimately non-blocking (verified, not taken on faith)

- **`build_op` id-guard + engine `sat≤0` → P9/T15:** correctly Minor. I verified the product chokepoint: `classify_raw` refuses any non-`is_imported()` payload (`reconcile.rs`, the `!as_.is_imported()` → Usage refusal), so both smuggling vectors require hand-crafting the vault file itself — a hand-crafter can already forge a worse arbitrary-basis `Acquire`. A smuggled pre-2025 tagged lot still trips the (mutation-verified-today) backstop; a ≥2025 one is a $0-basis lot, never an understatement. P9/T15 (engine integrity/never-understate task) is the right owner.
- **`in_force` dangling-void divergence → T16:** correctly Minor. Verified both engine halves in source: allocation voids go to the separate `allocation_voids` bucket, and a void of an *effective* allocation is adjudicated as a Hard `DecisionConflict` with the allocation staying in force (`resolve.rs` pass-1a + step 5). So the divergence is permissive-only, reachable solely through a state that already carries a Hard blocker (fail-loud), and the T5 backstop then denies the allocation on projection — tranche survives, Path A governs, nothing silent, never an understatement. A missed *friendly* refusal, exactly as filed. T16 (whole-branch, pre-merge) is appropriate.
- **`safe_harbor_residue` disposal-present skew → T16:** correctly Minor. Display-only in the TUI allocate opener; both record paths refuse anything authored under a pre-2025 tranche (CLI guard at `reconcile.rs:956` fires before residue matters; TUI persist guard at `persist.rs:1032`), so the skewed display feeds a flow that terminates in refusal. Fail-closed; no filed result can be wrong.
- **`--wallet` validation → P8/T14:** correctly Nit. The lot is $0-basis wherever it lands, so a typo is tax-neutral in the gain direction (a $0 lot that never gets consumed can only *overstate* gains elsewhere or produce a loud shortfall blocker — never understate). Warn-don't-refuse at the advisory task is the right disposition.

**No mis-filed blocking deferral found.**

## Bottom line

The fold is clean: the Important is genuinely resolved with a discriminating test, all four new tests kill their mutations (each verified RED by me, not taken from the author's report), the extended assertions are sound, the suite is fully green (2117 + clippy), and all four deferrals are correctly non-blocking with their source claims verified. The two new Nits (lowercase sentence assembly; the FOLLOWUPS ledger missing tax-r1 Nits 2/3 + the UTC-today residual) do not hold the gate — but the ledger omission should be corrected (one FOLLOWUPS edit or two cheap asserts) so the burndown grep stays trustworthy. **Phase-1 gate: 0 Critical / 0 Important — clear to close.**
