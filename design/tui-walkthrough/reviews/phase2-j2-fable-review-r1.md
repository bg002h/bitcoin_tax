# Phase 2 — J2 (§170(e) donation) review r1 — GREEN

_Fable, independent. Commit b385769._

VERDICT: GREEN — 0 Critical / 0 Important (3 Minor, 1 Nit).

Determinism: all 3 emits byte-stable, other 8 untouched; FMV doubly pinned ($217,992.34 = 2×108,996.17 bundled 2025-09-01 close). Tax facts CORRECT: deduction $110,996.17 = $108,996.17 (LT→FMV) + $2,000 (ST→min(FMV,basis), §170(e)(1)(A)); no gain recognised (Sch D 0); "before §170(b) AGI limits" present; investor+public-charity assumptions carried by the frame advisory; verify exit 0 held by the gated transcript. Coherence HOLDS (kind cycle Sell→Donate via Tab×3, hard-asserted; d lists only Donation removals; details satisfy is_review_complete(B)). Gates PASS.

Minor-1: manifest.txt:13 "the requirement it has satisfied" — "it"=btctax reads as the software discharged §170(f)(11)(C); it records Bob's appraiser metadata. Fix "…Bob's recorded appraisal satisfies".
Minor-2: manifest.txt:4/7 "gift" collides with the app's nondeductible Gift kind (beside Donate in frame 01). Fix "donation".
Minor-3 (out-of-diff): tabs/forms.rs:161 footnote "Section A/B is per-donation" stale — core picks the section from the §170(f)(11)(F) year-aggregate. File a TUI-footnote follow-up.
Nit-1: tests.rs:1118 doc "carrier leg at FMV" — the carrier row shows the §170(e) claimed deduction, not FMV.
