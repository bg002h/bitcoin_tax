# P3 Fable RE-REVIEW — fold of p3-fable-review.md (commit a503868)

**Reviewer:** Fable (independent; author ≠ reviewer). **Scope:** the a503868 fold of the RED P3 review
(0C/3I/2M/3N), verified against LIVE source at HEAD (a503868). **Date:** 2026-07-18.

## Verdict

All eight findings are RESOLVED, the fold introduced no new Critical or Important defect, and the suite
is green (1963/1963 + clippy via `make check`, ~14.5s). **Gate: 0C/0I — P3 GREEN.**

## Validation evidence (run by this reviewer)

- `make check` → **GREEN**: 1963 passed, 8 skipped. Delta vs. pre-fold (1956) reconciles exactly: +8 new
  tests (3 `from_env` units, 2 structural scans, 3 subprocess KATs) − 1 replaced (the old parse-only
  `from_env_rejects_malformed_…` test, whose assertions the new `from_env` tests subsume).
- **Mutation B re-run (the prior review's proven gap):** reverted `main.rs:8811`
  (`handle_match_self_transfers_modal_key`) to `now_utc()` → `no_direct_now_utc_in_production` **FAILED,
  naming `main.rs:8811`**. Restored; tree verified byte-identical to HEAD.
- **Viewer-side mutation (unprompted):** reverted `lib.rs:248` (the what-if routing site the prior review
  called unguarded) → btctax-tui's `no_direct_now_utc_in_production` **FAILED**. Restored; tree clean.
- `cargo test -p btctax-tui --lib clock::` → 5/5 pass (incl. the 3 new `from_env` units).
- Both subprocess KATs pass: `btctax-tui` (1) and `btctax-tui-edit` (2) — exit 2, stderr names
  `BTCTAX_NOW`, piped stdio (no TTY), proving the exit fires before raw mode.
- Both `*_goldens_match_committed` gates pass; the editor gate run 3× consecutively (fresh tempdir vault
  + seed + drive each run) — byte-stable, no flake.

## Per-finding status

- **I-1 (reconcile-flow golden) — RESOLVED.** `edit-classify-confirm-modal` is a genuine reconcile-flow
  frame: the classify-inbound confirm modal showing the target event (`import|river|test-ti-1`,
  TransferIn), date 2025-05-23, sat 500000, the `SelfTransferMine` classification, the $0-basis /
  receipt-date defaults, the non-gating advisory, and the append-only/atomic-write disclosure — exactly
  the §8 bug-hunt surface class, with a full style overlay. Deterministic: seeded from a fixed unix
  timestamp (1748000000), fixed `SourceRef("test-ti-1")`, fixed wallet, pinned clock, and the tempdir
  vault path overridden to `/edit/vault.pgp` before capture (the title is the only path surface; the
  frame shows no other path or wall-clock text). Committed (`git ls-files` confirms), iterated by
  `btctax_tui_edit_goldens_match_committed`, and inside the same `#[cfg(unix)]` gate the prior review
  verified on both unix CI test legs. The drive stops pre-persist (modal open asserted, no final Enter),
  so no vault write races the capture.
- **I-2 (env seam untested) — RESOLVED.** `clock.rs` now has `from_env_unset_is_wall`,
  `from_env_valid_rfc3339_is_pinned`, `from_env_malformed_and_empty_are_err` — the full CLI-contract
  matrix minus non-UTF-8 (covered structurally by the `to_str().ok_or_else` arm; acceptable).
  Env mutation is serialized under `ENV_LOCK` with save/restore (poisoning handled via `into_inner`);
  edition is 2021 workspace-wide, so the "`set_var` is safe" comment is accurate. No hidden NEW race:
  within the lib test binary only the `clock::` tests touch `BTCTAX_NOW`, and the theoretical
  libc-level concurrent-setenv hazard (vs. `unlock.rs`'s `BTCTAX_PASSPHRASE` tests under a different
  mutex) is the repo's pre-existing, documented pattern — not introduced by this fold. The subprocess
  KATs pin the exit-2/banner wiring per binary, and live source confirms `from_env` → exit(2) → banner
  all precede `enable_raw_mode` in both `main.rs` (~9778) and `lib.rs` (~678).
- **I-3 (structural guard + false claims) — RESOLVED.** Both crates carry
  `no_direct_now_utc_in_production`; btctax-tui's skips `clock.rs` (the seam's floor), the editor's has
  no exemption (its src has no clock.rs — stricter). I independently re-proved the exact gap: the
  Mutation B site (`main.rs:8811`) now REDS the editor scan naming the line, and the viewer's what-if
  site (`lib.rs:248`) REDS the viewer scan. Scan soundness verified against the whole tree: every
  production clock site (editor 1551–9746 < first marker 9809; viewer 248/257/644 < 711; editor.rs has
  no marker) precedes its file's first `#[cfg(test)]`, and an awk sweep found ZERO top-level production
  items after any file's first marker (every first marker opens the trailing `mod tests {`) — so the
  never-resetting `in_test` flag has no blind production line today. The doc comment at the per-site
  guard is corrected and now accurate (classify site end-to-end; whole-invariant held structurally by
  the scan).
- **M-1 (§14 gap 7 unrecorded) — RESOLVED.** capture.rs module docs record the decision (drops
  `underline_color`/`skip`, rationale, re-open trigger) and SPEC §14 gap 7 is ticked "DECIDED in P3"
  pointing at capture.rs/M-1.
- **M-2 (monochrome PDF vs §8's color mandate) — RESOLVED.** SPEC §8 carries a dated r2 amendment: the
  render is monochrome, the gated `.txt` goldens carry full style, the colorized render is explicitly
  deferred to FOLLOWUPS UX-P3-2 (owning phase: post-v0.7.0 docs). As the independent reviewer I grade
  the deviation ACCEPTED: the gated artifact loses nothing, the PDF is a git-ignored convenience with
  no consumers, and the deferral now has spec cover instead of contradicting it.
- **N-1 (filename-based tests.rs exemption) — RESOLVED.** The exemption now additionally requires the
  sibling `mod.rs` to contain `#[cfg(test)]` and `mod tests` (`export.rs:872-879`); `tabs/mod.rs:14-15`
  satisfies it, and the e10 gate stayed green in `make check`. A hypothetical production `pub mod
  tests;` whose mod.rs lacks any `#[cfg(test)]` is refused. (Residual imprecision: the two `contains`
  checks are not adjacency-linked, so a mod.rs with an unrelated `#[cfg(test)]` item plus a production
  `mod tests` would still slip — strictly stronger than the reviewed state and within the filed Nit's
  ask; noted, not re-filed.)
- **N-2 (untracked recon) — RESOLVED.** `design/usage-examples/RECON_P3_TUI.md` is committed and tracked.
- **N-3 (color_str Debug dependence) — RESOLVED.** capture.rs docs note the locked-0.29 stability and
  the loud regen-time failure mode.

## New findings

No new Critical or Important findings. One new Nit, recorded for the ownerless residue:

- **N-R1 (Nit — residue, no owning phase).** The structural scans' `in_test` flag never resets, so any
  future PRODUCTION code added after a file's trailing test module (or between test modules) would be
  invisible to the scan. Verified harmless today — no file has top-level production items after its
  first `#[cfg(test)]`, and the same heuristic shape is the established e10 `scan_non_test` pattern —
  but worth a one-line comment or a reset-on-module-close if the codebase ever interleaves. Do not let
  this reopen the gate; it is a robustness note on a guard that is empirically proven at every current
  site.

## Explicit gate

**0C/0I — P3 GREEN.** (0 Critical / 0 Important / 0 Minor / 1 Nit — N-R1, residue.) The fold delivered
the reconcile-flow golden (I-1), pinned the env seam with units + per-binary KATs (I-2), made the
25-site routing invariant structural with mutation-proven scans in both crates and corrected the false
claims (I-3), and discharged M-1/M-2/N-1/N-2/N-3 as filed. `make check` green at 1963/1963. P3 may
close.
