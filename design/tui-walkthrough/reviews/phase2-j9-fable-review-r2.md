# Phase 2 — J9 (select-lots) re-review r2 — GREEN

_Fable, independent. Commit 3b297ba (the J9 fold + J2/J5/J6 Minors)._

VERDICT: GREEN — 0 Critical / 0 Important.

I1 — RESOLVED. Repo-wide grep ("less than either / smaller than either / either combined") over docs/, crates/, design/tui-walkthrough (excl. reviews): zero hits. Replacement true (0.50 < 1.00): manifest.txt:4, testonly.rs:53,361, examples.rs:1248,1506. The surviving CLI-side "less than her holdings … a real choice" (examples.rs:1517 → examples.md:801) is the phrasing r1 endorsed, true in the CLI context (CLI picks are replay-time feasible, lot-b included).

I2 — RESOLVED, frame honest. Regen (emit exit 0) → git status --porcelain empty: committed j9/01-select-lots.txt matches the driver. Frame now shows `Pick Sat 50000000` / `Picked: 50000000 / 50000000 sat` (a completed identification). Prose describes the single offered lot ("lists the lot she can identify it against — the long-term lot-a") and assigning the whole 0.50 to it; no multi-lot-form claim, no new overclaim; the recording genuinely changes the draw vs default HIFO (35,000 LT vs 21,000 mixed). Driver types the pick + asserts matches!(SelectLotsStep::LotsForm{..}) — r1's N1 fixed. App-limit filed FOLLOWUPS.md; J9-M1/J2-M3/J6-M2/J5-N3 present.

Tax facts hold (j9/02-disposals: 0.50 from lot-a, 47,500/12,500/35,000, long; 03-compliance: contemporaneous). The Holdings-footer TOTAL 25000.00 beside the row 12500.00 is the shipped weighted-avg cost $/BTC footer (pre-existing, no manifest claim) — not a finding.

Minor folds correct: J2 "donation" not "gift" (remaining "gift" hits are the app's own kind-picker/advisory verbatims); J6 corpus comment now SHORT-term/Part I (confirmed by j6/04-forms); W-2 prose matches the single-W-2 frame; J5 caption "year tax delta -3660" accurate. make check green (2071); make tui-walkthrough warning-free; tree clean.

New finding: none gating. Nit (record-only): j6/manifest.txt:4 "a charitable gift" — same wording class as the J2 fold; harmless (away from any kind-picker), a one-word consistency polish. [Author: folded to "donation" in the closeout commit.]
