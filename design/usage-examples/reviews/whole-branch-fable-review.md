# WHOLE-BRANCH Fable review — usage-examples (P4 Task 4.2, final gate)

*Reviewer: Fable (independent). Date: 2026-07-18. Scope: `feat/usage-examples` vs `main`
(merge-base = local `main`, fully contained in HEAD — clean merge), HEAD `402c2e7`, plus the two
uncommitted P4 deliverables (`reviews/tutorial-workaround-audit.md`, the FOLLOWUPS.md P4 filings),
which are branch deliverables and were reviewed as such. Every phase (P0–P3) already holds its own
Fable 0C/0I; this pass is holistic — cross-phase coherence, branch-authored truth, fence, and
release-readiness. All findings verified against live source; all figures re-derived, not trusted.*

## Verdict

The branch is in excellent shape: the full suite is green (**1963/1963 passed, 8 skipped, + clippy
via `make check`, exit 0**), the §3.1 fence held across the whole diff (the only product-crate
`src/` changes are the two sanctioned env-injection clock seams and the test-module-only e10
source-gate fix — `btctax-core`/`-store`/`-adapters`/`-forms` src/`-input-form`/`-update-prices`
are byte-untouched), and **every number and claim authored by the project that I re-derived is
true**: J2's "2 × the $108,996.17 close = $217,992.34" matches the bundled dataset's 2025-09-01
row exactly; J4's 7,450.67 income = 0.05 × 85,484.60 + 0.03 × 105,881.32 and its 1,721.16 tax is
the correct 22%→24% bracket-straddle at the 2025 $103,350 edge; J5's 3,000.00/-660.00/-3,660.00
optimize triple and the what-if's 1,932.71 marginal (= 1,272.71 with-sale − (−660) baseline) all
reconcile; J6's conservation identity (0.35 in = 0.05 disposed + 0.10 removed + 0.20 held) and its
14-form packet are exact; the committed TOML fixture is pinned `== kitchen_sink_household().0` by
`fullreturn_oracle.rs`; and the census 14-key literal, the SPEC §6.1 list, and J6's emitted
manifest stems agree three ways. **One Important remains**: a fifth journey-content deviation
(J2/J1 vs SPEC §4.1/§5) that the §15 amendment mechanism — created by this branch precisely to
record such deviations, and claiming completeness at four — silently omits. That is a
design-artifact fold (no code, no golden change), then re-review. The gate is not green until it
lands.

## Findings

### Critical — none.

### Important

- **I-1 — SPEC §4.1/§5 vs the shipped J1/J2: a fifth (and sixth) journey-content deviation,
  unamended; §15's completeness claim is false.**
  `design/usage-examples/SPEC_usage_examples.md:189-191` (§4.1: `coinbase_buy_sell_send` "reuse
  as-is … **J1**" incl. the Send→pending-TransferOut leg; `coinbase_two_lot_donation` "reuse
  as-is … **J2**") and `:246` (§5 J2: "§170(e) donation **+ lot-selection**", commands
  `… → reconcile select-lots <disp> --from … → report --tax-year 2025 → export-irs-pdf …`)
  versus `docs/examples/examples.md:55-197` (shipped J1 has no Send leg; shipped J2 is
  init → import → reclassify-outflow → set-donation-details → verify → export-irs-pdf — **no
  `select-lots`, no `report`**, and both journeys run on freshly authored CSV corpora
  (`crates/xtask/src/examples.rs:191-201`), not the §4.1 builders). `select-lots` — the J2 row's
  own titled demonstrandum ("lot-selection") — appears **nowhere** in the golden (confirmed by
  grep and by the SOFT coverage report, which honestly lists it among 29 undemonstrated leaves).
  The §15 preamble (`SPEC_usage_examples.md:468-470`) states "**four** §4.2/§5/§6.1
  journey-content mandates could not be delivered as spec'd; **each** is amended here" — so the
  green SPEC now both contradicts the shipped golden on J2's content and misstates its own
  deviation census. No phase review ruled on this (grep: `select-lots` appears in no review file);
  the P1 review's I-2 caught (a)–(d) and missed this sibling.
  *Why Important, not Minor:* this is precisely the defect class the P1 review graded Important
  (I-2, "the shipped doc no longer contradicts a green, unamended spec") — the severity ladder
  must stay consistent within the branch; and the project's own recorded failure mode
  ([[dont-defer-spec-mandate-with-false-citation]]) treats silently undelivered spec mandates as
  serious. The discovered-reality rationales are real and §15-shaped: (i) the §4.1 builders are
  library `Vec<LedgerEvent>` constructors a CLI journey cannot import — the identical root cause
  as §15(c)'s "no CLI path to inject a LedgerState" — so every journey re-authors a CSV; (ii) a
  full-balance 2-BTC donation consumes both lots, making `select-lots` degenerate there (no
  choice to demonstrate; J5's `optimize accept` is the branch's actual lot-selection
  demonstration); (iii) J1's Send leg was dropped for a clean happy path.
  **Failure scenario:** a maintainer (or a regen after drift) works from the green SPEC's J2
  script, expects `select-lots` in the golden, and reconciles against a gate that never captured
  it; the spec's false "each is amended here" then hides that the divergence was unreviewed
  rather than blessed.
  **Fix (design artifact only — do not touch code or goldens):** add §15 amendment(s) (e)
  recording the J2 (`select-lots`/`report` dropped; degenerate-choice rationale; corpus
  re-authored) and J1 (Send leg dropped; corpus re-authored) deviations with the rationale above,
  correct the §15 preamble's count, and file the `select-lots` docs-coverage gap as a FOLLOWUP in
  the UX-P1-7/8 pattern (owner: post-v0.7.0 docs). Then re-review the fold.

### Minor

- **M-1 — RULING (asked of this review): the committed TUI golden captures the UX-P4-2 product
  bug verbatim — acceptable as a branch deliverable, binding at the release gate.**
  `docs/examples-tui/btctax-tui-edit-classify-confirm-modal.txt:21` (grid row 19) shows
  `acquired_at: (empty = default = receipt date, short-term)` — the product's confirm-modal
  string (`btctax-tui-edit/src/draw_edit.rs:927`), which states the rate-determining default
  **backwards** (the engine persists `long_term_default_acquired`, 1 year + 1 day before receipt
  → LONG-TERM; verified by the P4 audit end-to-end). **Ruling:** a faithful capture of a filed
  product bug is *acceptable* in a regen-gated golden — that is the bug-hunt design working
  (identical in kind to UX-P1-4/5/6 captured verbatim in `examples.md`), the fix is fence-barred
  from this cycle, and the golden regenerates (gate reds until it does) the moment the string is
  fixed. It is NOT a branch Important. **Condition that keeps it Minor:** UX-P4-2's owning phase
  is the **pre-v0.7.0 product-wording cleanup** (FOLLOWUPS, P4 filings) — under the per-phase
  burndown rule the v0.7.0 tag cannot ship while it is open. Two things distinguish it from the
  examples.md warts and would ESCALATE it to Important if that owner slips past the release: the
  TUI artifact has **no prose layer** (nothing in `docs/examples-tui/` discloses the wrongness,
  unlike J2/J6's framing sentences), and the wrong statement is a rate-determining fact at the
  point of informed consent. Ship v0.7.0 only after the wording batch lands + TUI goldens regen.

### Nit

- **N-1 — `crates/btctax-cli/tests/fullreturn_oracle.rs:4`** says the committed fixture lives at
  `crates/xtask/tests/fixtures/examples/fullreturn_inputs.toml`; it actually lives at
  `crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml` — as line 20 of the same
  file, the `include_str!` at line 23, the emitter path at lines 64-67, and the FOLLOWUPS
  plan-conformance record all correctly state. Stale path from the plan's original layout; fix
  the one word at the next touch.
- **N-2 — SPEC footer stale** (`SPEC_usage_examples.md:516-517`): "§15 added at r2 … — pending
  the P1 re-review." The P1 re-review-2 has since closed GREEN (0C/0I). Update the footer when
  folding I-1's amendment (same edit session).
- **N-3 — observation, product-side (captured output, NOT a branch defect):** J5's `what-if sell`
  block (`docs/examples/examples.md:377`) prints `§1212: -3000 offsets ordinary income this
  year, -27000.00 carried to next year` adjacent to `status: net gain`. The delta math proves the
  §1212 line describes the *baseline* (no-hypothetical) year while the surrounding figures are
  with-sale — internally correct but ambiguous which state the line describes. Candidate for the
  post-v0.7.0 wording batch (UX-P4-12 class); recorded here so it isn't lost.

## Release-readiness checklist (report, not performed)

**Version-embedding committed artifacts — the 0.6.1→0.7.0 bump MUST regen these, and both are
in-tree-gated so forgetting reds `make check` (the UX-P0-3 failure cannot recur silently):**

1. `docs/examples/examples.md` — front matter `btctax-version: 0.6.1` (:3). Regen:
   `cargo run -p xtask -- examples > docs/examples/examples.md`. Gates:
   `examples_golden_matches_committed` (in-tree) + the CI `examples` job diff. **UX-P1-9's
   front-matter rewording folds here** (its designated owner is this regen).
2. `docs/man/btctax-update-prices.1` — `.SH VERSION v0.6.1` (:48). Regen:
   `cargo run -p xtask -- docs`. Gate: `gen_docs_is_deterministic`. It is the **only** man page
   embedding a version (`btctax`/TUI pages carry none — `#[command(version)]` is deliberately
   omitted per SPEC §7; grep-verified across `docs/man/`).
3. `Cargo.lock` — mechanical with the bump.

**Version-free and safe across the bump:** all four `docs/examples-tui/*.txt` goldens
(grep-verified: no version string) — the bump itself forces no TUI regen. The `btctax --help`
capture in `examples.md` is also version-free (no `--version` flag exists). No other committed
artifact embeds the version (README grep-verified). **Nothing goes stale on the bump outside the
regen list above.**

**Pre-tag sequence implied by the branch's own FOLLOWUPS ownership:**

- [ ] Fold I-1 (§15 amendment + FOLLOWUP) and re-review to green; fix N-1/N-2 opportunistically.
- [ ] **Commit the currently uncommitted P4 deliverables** before merge:
      `design/usage-examples/reviews/tutorial-workaround-audit.md` (untracked), the FOLLOWUPS.md
      P4 filings (modified), and this review.
- [ ] Merge `feat/usage-examples` (main is fully contained in HEAD — clean). Note: local `main`
      is 2 commits ahead of `origin/main` (the SPEC-green + brainstorm commits) — push both.
- [ ] **Pre-v0.7.0 product-wording cleanup** as its OWN reviewed change (it is fence-barred from
      this branch): UX-P4-2 (the backwards modal string — M-1's release condition), UX-P1-2/N3
      (stale `export-irs-pdf` help + `--forms` value naming), UX-P1-4 (empty "Filled IRS forms →"
      header), UX-P1-5 (DOB `[2012,106]`), UX-P1-6 (+ Section-A extension, needs-REVIEW wording).
      Each changes captured output ⇒ the CLI golden / TUI goldens / man pages regen as part of
      that change (the gates red until they do — by design).
- [ ] Bump all 10 crates → 0.7.0; regen list items 1–3 (fold UX-P1-9); `make check` green.
- [ ] Tag, release, publish (heed [[crate-publishing-state]]: `cargo publish --workspace` can
      internal-error at the tail — resume with `-p <crate>`; verify the index with `grep -c`).
- [ ] USER action (not in-tree, SPEC §9/§14 gap 1): promote the CI `examples` job from advisory
      to a required check in GitHub branch protection once stable.
- [ ] Post-v0.7.0 (does NOT gate the release, per ownership): UX-P4-1 (pseudo-mode `report
      --tax-year` flag gap — top priority of the next product cycle), UX-P4-3..12, UX-P1-7/8,
      UX-P2-1, UX-P3-2, N-R1, N-3 above.

## Fence audit (whole branch)

Product-crate `src/` changes, exhaustively: `btctax-cli/src/main.rs` — the `resolve_now()` seam
only (R-P0.1–R-P0.4 exactly; the only removed production line is `now_utc()`); `btctax-tui` —
new `clock.rs`/`capture.rs` modules, the `Clock` field + 2 routed reads (`lib.rs:247,256`), the
`run_viewer` seam init (banner before raw mode, exit 2 on malformed), and the `export.rs` e10
source-gate change **confined to `mod tests`**; `btctax-tui-edit` — the `Clock` field
(`editor.rs`) + 23 routed reads in `main.rs` (verified: the diff's only removed production lines
are the 23 `now_utc()` reads + the `run()` signature — no message, engine, or schema edit
anywhere). `btctax-core`, `btctax-store`, `btctax-adapters`, `btctax-forms/src`,
`btctax-input-form`, `btctax-update-prices`: **zero diff**. The structural
`no_direct_now_utc_in_production` scans in both TUI crates hold the invariant against reversion.
**Fence: PASS.**

## Cross-phase coherence audit (what was checked and agreed)

- Census three-way agreement: SPEC §6.1 literal == `census.rs::CENSUS_KEYS` (14) == J6's manifest
  stems (`00_f1040 … 155_f8283`), with both census tests asserting set equality both directions.
- `fullreturn_inputs.toml` == `kitchen_sink_household().0` pinned by `fullreturn_oracle.rs`
  (exact `PartialEq`), banner-idempotent regen helper.
- §15 (a)–(d) amendments each verified against the shipped golden (J4 auto-resolved 2025 income;
  J5 single-branch + accurate prose; J6 small synthetic ledger, no false oracle caveat printed;
  J3 single-exchange path) — all agree. The residual is I-1.
- FOLLOWUPS owning-phase reconciliation vs shipped reality: UX-P1-1 discharged as claimed
  (verified `[exit N]`/relative-path/front-matter in the artifact); UX-P0-3's stale man page
  fixed in-branch; P2 N-2 resolved in P3 as recorded (TUI goldens in-process-gated, CI gained the
  PDF-build proof instead of a diff widening); UX-P2-1's "17/46 honest today" re-verified against
  the live coverage report (17/46, `select-lots` correctly listed undemonstrated).
- The J2 prose framing of the needs-REVIEW stderr and the J6 "no such note" claim both verified
  against the captured stderr blocks and the UX-P1-6 (+ extension) filings — consistent.
- All four phase reviews re-checked: each is a genuine persisted 0C/0I verdict.

## Gate

**0 Critical / 1 Important (I-1) / 1 Minor / 3 Nits — NOT GREEN.** The Important is a
design-artifact fold (SPEC §15 amendment + one FOLLOWUP entry; no code, no golden, no regen),
then re-review per §2. Everything else about the branch — suite, fence, authored-truth,
census, version-regen posture — is release-shaped.
