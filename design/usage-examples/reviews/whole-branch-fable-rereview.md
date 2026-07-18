# Whole-branch Fable RE-REVIEW — fold verification of `1f6bb38` (usage-examples, final gate)

*Reviewer: Fable (independent; author ≠ reviewer). Date: 2026-07-18. Scope: confirm the
`1f6bb38` fold resolved whole-branch I-1 + N-1 + N-2 without introducing new issues, verified
against live source (SPEC, golden, fixtures.rs, xtask, FOLLOWUPS, coverage report) + a full
`make check`. Per instruction, nothing was fixed by this review.*

**VERDICT: the fold resolves I-1, N-1, and N-2; one new Minor (a false mechanism claim inside
the §15(e)/(f) rationale, inherited verbatim from the original review's own I-1 text) and one
self-resolving Nit; the branch gate is 0 Critical / 0 Important — WHOLE BRANCH GREEN.**

---

## Per-finding status

### I-1 — RESOLVED (spec/golden contradiction closed; deviation census now complete and true)

Verified against live source, not the diff:

- **§15 now carries (e) and (f)**, and the preamble reads "SIX §4.1/§4.2/§5/§6.1
  journey-content mandates" with (a)–(d) attributed to the P1 fold and (e)–(f) to the
  whole-branch fold — the false "four … each amended here" census is gone. (a)–(f) counts to
  six; §4.1 was correctly added to the section list (the (e)/(f) deviations are §4.1/§5 ones).
- **§15(e) is factually true against the shipped golden** on every load-bearing claim:
  - Shipped J2 (`docs/examples/examples.md:130-197`) is exactly
    init → import → reclassify-outflow → set-donation-details → verify → export-irs-pdf —
    **no `select-lots`, no `report`** (grep over `docs/examples/`: zero hits for
    `select-lots` anywhere in the golden).
  - J2 really donates the **full 2-BTC balance across both lots**: `J2_CSV`
    (`crates/xtask/src/examples.rs:196-201`) is two 1-BTC buys (LT 2023-06-01 basis $5,000;
    ST 2025-03-01 basis $2,000) + one 2-BTC Send, and the golden's conservation line proves it
    (`in 200000000 = disposed 0 + removed 200000000 + held 0`). With both lots consumed
    entirely, lot ordering cannot change the §170(e) result — `select-lots` is genuinely
    **degenerate** there. The deduction figure re-derives: LT→FMV $108,996.17 +
    ST→min(FMV, $2,000) = **$110,996.17**, matching the golden's QualifiedAppraisalNote.
  - **J5 is genuinely the branch's lot-selection demonstration**: it sets a FIFO baseline
    (`config --set-forward-method fifo`), `optimize run` proposes the high-basis ST lot
    (`opt-buy-st#0:100000000` — the HIFO pick) over FIFO's LT lot — a real changed selection —
    and `optimize accept` persists it as a `LotSelection … [Contemporaneous]`.
  - The SOFT coverage claim verifies live: `xtask subcommand-coverage` reports **17/46**
    leaves demonstrated (= **29 undemonstrated**) and lists `btctax reconcile select-lots`
    among them.
- **UX-P1-10 is filed** (`FOLLOWUPS.md:2100`): Minor, owner **post-v0.7.0 docs**, correctly
  states the degeneracy rationale, names J5 as the actual demo, and matches the fix
  instruction's UX-P1-7/8 pattern.
- **§15(f) is true**: the §4.1 builder `coinbase_buy_sell_send`
  (`crates/btctax-cli/tests/fixtures.rs:8-21`) does carry a Send leg (`cb-send` →
  TransferOut→pending, per its own doc-comment), and shipped J1 (`examples.md:55-129`) has
  none — 2 rows (buy + sell), `pending 0`, a clean happy path. J1's additional shipped steps
  (verify, tax-profile, export-snapshot) are additive, not contradictions; the Send-leg drop
  was the only remaining §4.1-vs-shipped contradiction and is now recorded.

The §15(e)/(f) amendment is a **complete and accurate record of WHAT deviated** (steps
dropped, degeneracy, corpus re-authored, coverage gap filed). One statement of **WHY** is
factually wrong — see M-R1 below; it does not reopen I-1, because the spec no longer
contradicts any shipped artifact and the deviation census is complete.

### N-1 — RESOLVED

`crates/btctax-cli/tests/fullreturn_oracle.rs:4` now names
`crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml`, and the fixture verifiably
lives there (file present on disk; consistent with the `include_str!` and the FOLLOWUPS
plan-conformance record).

### N-2 — RESOLVED (with one observation, N-R2 below)

The SPEC footer no longer dangles on "pending the P1 re-review"; it records §15's r2 origin +
the (e)/(f) whole-branch extension and the P1 re-review-2 GREEN. No stray text.

### M-1 (ruling) / N-3 — no action was required; both stand as recorded. Not re-adjudicated.

---

## New findings

- **M-R1 (Minor) — §15(e)'s corpus-re-authoring mechanism claim is false against live source
  (and internally contradicts §4.1); §15(f) inherits it.** §15(e) states "the §4.1 builders
  are library `Vec<LedgerEvent>` constructors a CLI journey cannot import (the SAME root
  cause as (c)'s 'no CLI path to inject a LedgerState')". In fact `coinbase_buy_sell_send`
  (`fixtures.rs:8`) and `coinbase_two_lot_donation` (`fixtures.rs:50`) are **tempdir CSV
  writers returning `PathBuf`** — real Coinbase-format CSVs the CLI *could* import if they
  were reachable; §4.1 itself (`SPEC:192-193`) marks only `income_fmv_missing_batch(n)` as
  "returns `Vec<LedgerEvent>` (NOT a CSV) … Library-level only". The TRUE mechanism: the
  builders are btctax-cli **integration-test-tree helpers xtask cannot link against** (no lib
  target exports them; the CSVs are written at test runtime, never committed), and the
  fixture's content did not fit the journey anyway (its Send is dated 2026-03-01 with FMV
  $100k — outside J2's 2025 tax year). The *conclusion* (J2/J1 embed freshly-authored
  CRLF-const CSVs) and every shipped artifact, figure, and guarantee are unaffected; the
  load-bearing rationale for dropping `select-lots` (degeneracy) is true and untouched.
  Provenance: this wording was transcribed faithfully from the original whole-branch review's
  own I-1 rationale (i) — the error is the reviewer's, folded as instructed. *Why Minor, not
  Important:* unlike I-1, nothing shipped is contradicted and no consumer builds on the claim;
  it is a design-history side-rationale, fixable by a two-line reword. **Owning phase:
  post-v0.7.0 docs batch (file alongside UX-P1-8/10; or fix at the next SPEC touch).** Not
  fixed here per this review's instruction.
- **N-R2 (Nit, self-resolving) — the footer's whole-branch re-review GREEN was written
  prospectively.** The footer (updated in `1f6bb38`) states the whole-branch re-review "has
  since closed GREEN (0C/0I)" — i.e. it asserted THIS review's verdict before the review ran.
  Because this review does close GREEN, the statement is true the moment this artifact lands;
  recorded only so the sequencing is honest in the history. No action.

Nothing else was broken by the fold: it touched only the SPEC, one doc-comment line, FOLLOWUPS,
and the two (new) review artifacts — no code, no goldens, no regen — and the working tree is
clean at `1f6bb38` (the earlier untracked `crates/xtask/tests/` is gone).

## Validation

`make check`: **1963/1963 passed, 8 skipped — GREEN**, including the examples golden gate
(`examples_golden_matches_committed`), determinism, and hermeticity tests.

---

## FINAL GATE

**0 Critical / 0 Important — WHOLE BRANCH GREEN, release-ready.**
I-1, N-1, N-2: RESOLVED. New: 1 Minor (M-R1, filed to the post-v0.7.0 docs batch) + 1 Nit
(N-R2, self-resolving) — neither holds the gate. Standing release condition (unchanged, from
M-1's ruling, outside this gate): the v0.7.0 **tag** still requires the pre-v0.7.0 UX-P4-2
wording fix + TUI-golden regen before shipping.
