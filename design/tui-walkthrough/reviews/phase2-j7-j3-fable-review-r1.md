# Phase 2 — J7 + J3 batch walkthrough review r1 — NOT GREEN (J3 Critical)

_Reviewer: Fable (independent). Scope: the J7+J3 batch (commit `be7f9cb`). Persisted verbatim before
folding, per STANDARD_WORKFLOW §2._

**VERDICT: NOT GREEN — J7: 0C/0I (green); J3: 1 Critical (tax-fact error in shipped prose).**

## Critical

**J3-C1 — the walkthrough tells the filer a 9-month-old lot is "long-term".**
`docs/examples-tui-walkthrough/j3/manifest.txt:9` — "the 0.20 BTC lands (acquired 2024\-11\-01, basis $19,000, **long\-term**)". The receipt is 2025-08-01 (`crates/btctax-cli/src/testonly.rs:32`) and the depicted session's clock is 2025-08-02 (`crates/btctax-tui/src/tabs/tests.rs`, `j3_viewer_frames`; editor pinned the same). 2024-11-01 → 2025-08 is ~9 months; §1222 long-term requires holding **more than one year** (long-term only for a disposal on/after 2025-11-02). The lot is short-term at the depicted time, and no disposal exists to make any term claim. Failure scenario: a filer taught that carried-date self-transfer coins are "long-term" reports a short-term sale as LTCG. Note the codebase itself knows the rule: the `SelfTransferMine::acquired_at` default is deliberately "1yr+1day before receipt (long-term)" (`crates/btctax-core/src/event.rs:146`) — a property the hand-supplied 2024-11-01 does *not* have; and the shipped `docs/examples/examples.md:230-231` words J3 correctly ("carrying the original basis and acquisition date (for the holding period)") with no term claim. Same error echoed in the seeder doc: `crates/btctax-cli/src/testonly.rs:335` "(2024-11-01, long-term)". **Fix:** replace "long\-term" with the true teaching point, e.g. "…basis $19,000 \(en the holding period runs from the original acquisition, not the transfer)", and fix the testonly.rs doc comment. Manifest-only + comment change; no goldens move.

## Important

None. Everything else attacked held up:
- **Determinism:** all three regens (editor, viewer, console emits) left `git status --porcelain` empty; `make check` green (2071 passed) — its match gates are an independent second capture compared byte-for-byte, so J1/J4/J8 are proven untouched and both new journeys byte-stable. J3's two Holdings rows sort deterministically by the Acquired ▲ column (distinct dates). PDF (`make tui-walkthrough`) renders warning-free.
- **Editor↔viewer coherence:** picker starts on `Income` (`btctax-tui-edit/src/main.rs:1698-1699`), Tab cycles Income→GiftReceived→SelfTransferMine (`main.rs:1721-1723`) — J7's Enter-only frame shows `> Income`, J3's Enter+Tab×2 shows `> SelfTransferMine`, both matching their seeds. The editor confirm modal appends the identical `EventPayload::ClassifyInbound{transfer_in_event, as_}` (`main.rs:1568-1571`) that `cmd::reconcile::classify_inbound` appends (`reconcile.rs:114-119`), with matching acquired-not-after-receipt validation on both paths.
- **J7 tax facts:** verified independently — 2024 single 22%/24% boundary at $100,525 taxable; $525×0.22 + $2,775×0.24 = **$781.50** ✓; marginal 0.24, LTCG 0.15, NIIT false (MAGI $103,300 < $200k) all correct; "no auto-valuation, FMV by hand" matches the product (FmvMissing KAT + examples.md J7); frames show 3300.00/781.50 exactly as the manifest claims.
- **Gates:** manifest bijection gate is per-directory generic and passed for j7/j3; both STEMS consts list exactly the emitted stems with set-equality asserts; captions clean of `"`/`\`.

## Minor

1. **J3 — Holdings TOTAL row unexplained where it's most confusing.** `j3/02-holdings.txt:37` shows TOTAL USD-Basis **95071.43** while the two visible rows sum to 66550.00 (it's the weighted-average cost/BTC: 66550/0.7, `btctax-tui/src/tabs/holdings.rs:119-125`). J1 and J8 manifests both carry an explanatory clause after their reviews ("shows the average cost per BTC, not a sum"); `j3/manifest.txt:9` omits it, and J3 is the first frame with two rows making the mismatch arithmetically visible. Add the J1-style clause.
2. **J7 — persona contradicts the shipped J7 example.** `j7/manifest.txt:5` "She creates the vault…" but `docs/examples/examples.md:650` J7 is **Frank**. J1/J4/J8 walkthroughs name their examples.md personas (Alice/Erin/Grace); J3's "she" matches Carol. Name the persona (Frank → "He…") or rename consistently.
3. **J7 — the staking kind is silently elided.** The income form defaults to `IncomeKind::Mining` (`main.rs:1738`); the manifest narrates only the FMV ("the income form then takes the hand-entered FMV, $3,300", `j7/manifest.txt:7`), never the kind selection, yet the viewer frame shows *staking*. A filer replaying the depicted keys verbatim records mining income. One clause ("…and the kind, staking") closes the gap.

## Nit

1. **J7 walkthrough diverges numerically from examples.md J7** ($100k base/$781.50 straddle vs $90k/$726 flat-22%) — internally consistent and the straddle is the better frame, but a reader of both docs sees different numbers for the same journey; consider a note or future alignment.
2. **Neither new journey names its persona** while J1/J4/J8 do — style drift.
3. Pre-existing (not this diff): stale doc comment `btctax-tui-edit/src/main.rs:1714` "Income ↔ GiftReceived via Tab" — the cycle has three variants.

Tree left clean (`git status --porcelain` empty).
