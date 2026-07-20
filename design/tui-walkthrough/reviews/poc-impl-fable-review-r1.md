# Independent adversarial review — TUI screen-walkthrough PoC (J8)

_Reviewer: Fable (independent, author ≠ reviewer). Scope: PoC implementation commits `8cb0ca1`
(Phase 0), `f9f82b7` (Phase 1a), `9b197ba` (Phase 1b) — diff `1551886..9b197ba` restricted to impl
files. Spec/plan (already reviewed r1→r2 GREEN) used as the contract, not re-reviewed. Persisted
verbatim before folding, per STANDARD_WORKFLOW §2._

**VERDICT: NOT GREEN — 1 Critical, 1 Important** (frames/captures/convergence/e10/Phase-0 all verified sound; the manifest/PDF half of the pipeline is ungated and in-tree comments claim otherwise).

## What I verified as delivered (the claims that hold)

1. **Frames are real.** Gate and emit share one capture fn (`btctax_tui_edit_walkthrough_frames()` at `crates/btctax-tui-edit/src/main.rs:14316`; `btctax_tui_walkthrough_frames()` at `crates/btctax-tui/src/tabs/tests.rs:921`) driving the real `handle_key`/`draw` into `to_golden`. The gate re-captures fresh each run and byte-compares. All 4 gates + e10 + `examples_golden_matches_committed` **pass on this machine** (fresh tempdirs, different wall-clock than the author — empirical determinism proof). Mutating `j8/04-holdings-balanced.txt` reds the gate (demonstrated via cp-backup/restore; tree restored clean).
2. **Determinism.** Pinned `Clock::Pinned(2025-04-01)`, vault path overridden before every capture (frames show `/edit/vault.pgp` / `/vault.pgp`), 120x40 confirmed by the goldens' geometry, `BTCTAX_PRICE_CACHE` pinned nonexistent before any unlock in both drivers. No tempdir/time/price leak found in any committed frame; single-row lists/tables leave no HashMap-order surface.
3. **Convergence (§4.2) is provable, not stitched.** Frame `03-match-confirm.txt:19-20` literally displays the exact refs `import|coinbase|in|cb-recv` and `import|river|out|1741608000000|withdrawal|10000000#0` (ms-epoch verified = 2025-03-10T12:00Z) with action RELOCATE — the same refs `seed_j8_relocated` hardcodes (`crates/btctax-cli/src/testonly.rs:209-216`). Modal-Enter routes Relocate → `EventPayload::TransferLink{out, InEvent(in)}` (`main.rs:8873-8881`), byte-same payload as `cmd::reconcile::link_transfer` (`reconcile.rs:883-887`). The editor-side write is held by the existing e2e `kat_e2e_match_self_transfers_relocate_lands_coins_in_dest`. Caveat: the walkthrough editor test itself stops pre-persist (see M-3).
4. **e10 compliance.** The gate scans the whole of `tabs/tests.rs` for `cmd::` (whole-file test modules get no test-region carve-out for everywhere-tokens, `export.rs:881-893`); the viewer driver uses only `testonly::seed_j8_relocated` + `Session::open` + `build_snapshot` + render. Gate passes.
5. **Phase 0.** Const bytes moved verbatim (only doc-comments trimmed — golden-irrelevant); J6 TOML now a same-crate `include_str!` of the same file; `examples_golden_matches_committed` regenerates byte-identical; pii-scan and fmt pass; `testonly` pulls no new deps into the published crate.
6. **J8 tax story.** Frame 04 shows coinbase, acquired 2025-01-05, 0.10000000 BTC, basis 4000.00, source "transferred" — basis + holding period carried, non-taxable (matches the mandated self-transfer policy); prose says "no gain, no loss" and correctly discloses the TOTAL 40000.00 as weighted-avg cost/BTC. The PDF being ungated/git-ignored is by design (SPEC §2, `.gitignore:53`) — not filed.

## Critical

**C-1. The manifest/PDF half doesn't implement the spec's gating architecture, and in-tree comments claim gating that doesn't exist.** SPEC §5 mandates: an xtask `tui-walkthrough` subcommand emitting a committed, **byte-gated** manifest whose captions live as Rust literals ("not hand-edited" — §8 deliverables 4+7, M-2 single-owner decision); "a `regen == committed` test on the manifest … together pin the whole artifact"; and a CI `%PDF` proof (PLAN Phase 1 steps 3+5, SPEC §7). The impl has none of it:
- No xtask subcommand (`crates/xtask/src/main.rs:15-52` — no `tui-walkthrough` arm).
- `docs/examples-tui-walkthrough/j8/manifest.txt` is hand-authored prose+captions with **no gate of any kind**: nothing in `make check` or CI parses it or checks its `FRAME` references. Delete/typo a `FRAME` line, or rename a golden stem in the emit tests, and the full validation surface stays green — the walkthrough silently loses a screen until a human runs `make tui-walkthrough` locally.
- No CI proof: `.github/workflows/ci.yml` examples job stops at `make examples` + `make examples-tui` (lines 127-129); `make tui-walkthrough` is exercised by nothing in CI, so even the "renders a %PDF" backstop is absent.
- **False in-tree claim:** `Makefile:86-88` states "the `.txt` frames + manifests are the gated artifacts, held by each crate's `*_walkthrough_goldens_match_committed` test" — those tests (`tabs/tests.rs:949`, `main.rs:14372`) read only `j8/NN-*.txt` frames, never a manifest.

Fix: implement §5 as written, or formally amend the spec to bless the bash+hand-manifest design AND add a real gate (a test that parses each manifest, validates grammar, and asserts every `FRAME` reference names a golden some crate byte-gates) plus the CI `%PDF` step; correct the Makefile comment either way.

## Important

**I-1. `Makefile:94-97` — the multi-manifest loop fails OPEN.** Demonstrated in a sandbox: with manifests A (missing frame) + B (good), the `{ echo; for …; done; } > roff` compound exits with the **last** iteration's status (0), the roff silently omits journey A, and the `\m[`/`%PDF` checks pass. Latent today (one manifest), but line 89's own comment promises Phase 2 adds more — this reintroduces, one level up, exactly the "silently drop a screen" failure `assemble-walkthrough.sh` fails closed against (bad line and missing frame both exit 1 — verified). Fix: `bash … || exit 1;` in the loop body.

## Minor

- **M-1.** `manifest.txt:10` — "Enter opens the confirmation modal — the informed-consent point, and the moment the vault is written" misstates the write point: opening the modal writes nothing (frame 03's own footer: "[Esc] Cancel — writes nothing"; the capture is pre-persist). Reword: confirming the modal is the write moment.
- **M-2.** `manifest.txt:13` caption "viewer: Holdings, BALANCED" — "BALANCED" is a literal token of the **Compliance** tab (`tabs/compliance.rs:70`), and frame 04 (Holdings) doesn't display it. Add a Compliance frame (which would genuinely read BALANCED) or reword.
- **M-3.** PLAN Phase 1 step 1's fourth editor frame ("post-relocate Browse") was silently dropped (`main.rs:14363-14367` captures 01-03 only), so the editor half never shows the post-confirm state. Defensible under SPEC §10's granularity latitude and functionally covered by the existing e2e, but the deviation is unrecorded — record it (or capture the frame) so the owner's PoC gate sees a choice, not an omission.
- **M-4.** `assemble-walkthrough.sh:32-34` — a `FRAME <file>` line with no caption doesn't fail closed: `caption="${rest#* }"` degrades to the filename as the `.SH` caption. Enforce a non-empty caption.

## Nit

- **N-1.** `assemble-walkthrough.sh:4` (and the Phase 1b commit message) cite "SPEC §4.4 / I-4" — the spec has no §4.4; the interleave contract is §5. Fix the citation.
- **N-2.** `main.rs:14299-14312` `seed_j8_vault` duplicates `btctax_cli::testonly::seed_journey` line-for-line — call the shared seeder (nothing in the editor crate forbids it; tightens the one-source-of-truth story).
- **N-3.** `main.rs:14318` comment claims the price-cache pin protects "the displayed market value ($8137.26…)" — no committed J8 frame contains that value (the match-list row clips before the USD column). The pin is still right; fix the comment.

**Also noted (not a finding):** the impl replays RELOCATE via `link_transfer` rather than the plan's cited `apply_self_transfer_passthrough` — the plan's citation was wrong (that fn is the DROP action); the impl deviated **toward** correctness per G-RELOCATE-REUSE.
