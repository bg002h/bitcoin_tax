# Phase 2 — J6 (a complete Form 1040) review r1 — GREEN

_Fable, independent. Commit 84c335d._

VERDICT: GREEN — 0 Critical / 0 Important (2 Minor, 3 Nit).

Determinism (regen ×2 byte-stable, other 8 untouched; mining FMV pinned 68,759.09×0.05=3,437.95; no BTCTAX_NOW banner). Tax facts independently replayed via the CLI export-snapshot: disposals.csv lot=import|river|…income… basis 3437.95 proceeds 3130 gain −307.95 SHORT — the HIFO/mining-lot claim CONFIRMED; form8283 §B 0.10 BTC FMV 6000; schedule_se byte-equals the golden (2437.95/2251.45/0/65.29; SS uses only owner-taxpayer Box3=168,600→0; Addl-Medicare threshold reduced by JOINT Box5 290,000). Tax-tab deltas 825.11/−46.19/−11.70/767.22 re-derived; "NIIT applies:false" semantically correct (crypto reduced NIIT). Forms-tab crypto-only caveat correct. Coherence HOLDS (single seed_j6_full; commit pre-persist). Gates GREEN.

Minor-1: testonly.rs:56-57 J6_COINBASE_CSV doc "a small 2024 LONG-TERM sale (Schedule D Part II)" — FALSE (HIFO draws the mining lot ⇒ short-term, Part I). Dev-facing (the AMT-margin audience). Fix the comment.
Minor-2: j6/05-tax.txt "NIIT (attributable delta): -11.70" beside "NIIT applies: false" — correct per the marginal-flag semantics but misreads; suggest a label follow-up.
Nit-1: examples.md:388 J6 persona "Frank" vs the John/Jane Doe household (pre-existing, out of diff).
Nit-2: console imports coinbase,river while j6() imports river-first — no observable divergence (refs content-derived; HIFO decisive) but the §4.2 asymmetry.
Nit-3: manifest step-2 "the couple's wage income" while the frame shows one W-2 (Owner Taxpayer).
