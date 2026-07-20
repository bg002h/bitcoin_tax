# Re-review (r3) — TUI screen-walkthrough PoC fold — GREEN

_Reviewer: Fable (independent). Scope: the r2 fold (commit `92694dc`) — verify NEW-I-1/M-1/N-1/N-2
resolved and no new blocking defect. Persisted verbatim per STANDARD_WORKFLOW §2. This round is GREEN,
so no fold follows; r3-M-1 (below) is filed to FOLLOWUPS with owning phase Phase 2._

**VERDICT: GREEN — 0 Critical / 0 Important** (1 new Minor filed, non-blocking, Phase-2-owned).

## Per-finding verification

**NEW-I-1 — RESOLVED, empirically.**
- Baseline: all three gates pass on the committed tree, so the consts list the REAL emitted stems (`WALKTHROUGH_VIEWER_STEMS` at `crates/btctax-tui/src/tabs/tests.rs:953` = `j8/04-holdings-balanced`, matching the vec at :944; `WALKTHROUGH_EDITOR_STEMS` at `crates/btctax-tui-edit/src/main.rs:14367-14371` = 01/02/03, matching the vec at :14354-14358).
- Re-ran my r2 break (emptied the viewer capture vec) + an editor tuple drop (cp-backup/restore): **both RED at the stem `assert_eq!` before the byte-loop** (tests.rs:962 / main.rs:14380), each message naming its own const correctly (`left: {}` vs `right: {"j8/04-holdings-balanced"}`). The vacuous pass is dead.
- Honesty of "pins the whole artifact": under a **single-fault** model every link now reds somewhere — manifest line dropped → bijection; golden deleted → bijection + byte-loop; tuple dropped/renamed → stem assert; const entry dropped → stem assert. Adding a frame is pinned at every step too. My r2 fix-ask explicitly named "per-crate expected-stem consts asserted in each crate's gate" as a sufficient cheap fix; the fold implemented exactly that, so the claims (Makefile:88-90, SPEC §5:150-151, assemble-walkthrough.sh:5-8, examples.rs comment, ci.yml) are now honest at the standard r2 set.
- Residue found (demonstrated, filed as Minor below): the **pair-fault** — drop a tuple AND its const entry together, leaving the manifest ref + golden — passes all three gates (I proved it: viewer gate + xtask bijection both PASS with the const and vec emptied, `04-holdings-balanced.txt` orphaned). This is not Important: it requires two coordinated deliberate edits, and the red the dev sees after the first edit explicitly instructs "update WALKTHROUGH_VIEWER_STEMS **and the manifest** together" — following which the bijection then forces the golden's deletion. Single accidental faults (the Phase-2 rollout hazard that made r2's finding Important) all red.

**NEW-M-1 — RESOLVED.** SPEC §8 deliverable 4 (`SPEC_tui_walkthrough.md:196-203`) and 7 (:207-211) now carry explicit "_(As-built … supersedes …)_" markers; §9:221-223 says "hand-authored manifest + its xtask bijection gate (§5, As-built)"; §11:243-247 reworked (manifest hand-authored, regen = both emit tests, "(As-built, §5)"). Grepped for lingering unmarked struck-design mentions: SPEC's remaining ones are inside §5's original-contract text, superseded by the amendment directly beneath (accepted in r2); the PLAN's steps 3-4 wording is covered by its own As-built deviations section (:74-99, accepted in r2). §8 item 5's ".PP prose emitted directly, tui-wrap.awk on frames" matches the actual script. No contradictions introduced.

**NEW-N-1 — RESOLVED, empirically.** `examples.rs` now asserts `root.is_dir()` (~:1320) instead of returning; moved the dir aside → gate REDS with "docs/examples-tui-walkthrough is missing" (restored).

**NEW-N-2 — RESOLVED.** `44ffee7` ("persist … verbatim … before folding", 18:14:25) precedes fold `92694dc` (18:18:09), separate commits; the persisted review matches my r2 output verbatim.

## Fold-integrity and new-defect sweep
- The provided diff is **byte-identical** to fold commit `92694dc` (verified via `git diff 44ffee7 92694dc`); the commit touches only the four expected files.
- No lint/compile defects: consts are `#[cfg(unix)]` alongside the equally-cfg'd gates (no non-unix dead code); `BTreeSet`/`copied` are far pre-1.88. `make check` exit 0 (2070 passed, clippy `-D warnings` clean), `cargo fmt --check` clean.
- Non-finding observation: §11's mandated `make regen-walkthrough` target doesn't exist yet — unchanged in status from r2 (a future-directed "provide", tied to the Phase-2 golden count); not a fold regression.

## New finding (non-blocking)

**Minor (r3-M-1, owning phase: Phase 2 rollout).** The crate stem consts and the manifest's FRAME set are two independent authorities with no automated cross-check: `const ⊂ manifest∩disk` passes all gates (demonstrated — tuple+const removed together leaves an orphaned, never-re-verified golden in the PDF; the converse direction IS caught). Reachable only via a half-done deliberate removal that ignores the assert message's explicit instruction. Suggested Phase-2 remedy: when extending the consts per-journey, add a cross-check (e.g. the xtask gate parsing the two `WALKTHROUGH_*_STEMS` consts out of the crate sources and asserting their union equals the union of manifest FRAME refs), or record the removal protocol in the rollout checklist.

Working tree is byte-identical to HEAD — all mutations cp-backup/restored, verified via `git status` + `git diff HEAD --stat` (empty).

**GREEN.** The r2 fold is genuine: the disk⇄capture pin closes the vacuous-pass hole at exactly the single-fault standard r2 demanded, the doc residue is swept, and the fold introduced no new blocking defect.
