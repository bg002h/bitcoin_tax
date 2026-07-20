# Phase 2 — J5 (lot-selection optimization) review r1 — GREEN

_Fable, independent. Commit 729e403._

VERDICT: GREEN — 0 Critical / 0 Important (1 Minor, 3 Nits).

Determinism HOLDS (all 3 emits byte-stable twice; optimizer/what-if bundled-dataset only, no network, no HashMap; now threaded; made-date divergence inert — no frame renders a made-date). Tax facts ALL CORRECT (FIFO +$20k LT → $3,000; optimized −$30k ST → §1211 $3,000 @22% = −$660, §1212 $27,000 short carryforward; Δ −$3,660; pick opt-buy-st#0:100000000; Contemporaneous). Coherence HOLDS by construction. tui-wrap Δ→D correct+necessary. Gates GREEN (make check 2071).

Minor M-1: the `≤` (U+2264, 3 bytes) in the "(≤ 0)" banner (j5/01,02 row 10) is unmapped in tui-wrap's 1:1 ASCII glyph map — shifts the bold run 2 cells early under CI's mawk (`awk -b` reproduced: trailing "0)" un-bold in the CI PDF). Fix: `gsub(/≤/,"<")` beside the Δ map.
Nit N-1: examples.rs:1328-1331 doc "advisory pre-2025-method note" — the gated transcript shows "Advisory blockers: 0"; the note is informational.
Nit N-2: manifest.txt:7 caption "(year tax -3660)" can read as the year's tax not the delta; "(year delta -3660)" is exact.
Nit N-3: j5/00-setup.console.md:22 pre-election verify shows the sale :: non_compliant; the walkthrough never revisits compliance post-accept.
