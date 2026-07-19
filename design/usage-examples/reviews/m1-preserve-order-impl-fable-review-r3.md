# Independent re-review — M-1 fold (r2-I1), round 3: un-stick the enumeration-scan cfg(test) skip

Reviewed commit `c8e0d8a`; source read at HEAD `bd73968`. Mutation run on HEAD, tree restored clean.

## Verified
1. The sticky `#[cfg(test)]` skip is GONE (every line scanned; only line-comment stripping remains). The exact r2-I1 case re-proven: a `json!` injected into `render.rs render_events_list` (post-2475 production, after its test mods) REDS the scan (render.rs:3708/9) with the remediation message — silent under the old flag, dead now. Restored, green.
2. No false positive introduced: independent grep of `serde_json::to_value|json!(` over crates/*/src hits ONLY the three ALLOWED files (tax.rs:195; oracle-harness main.rs ×8; coverage.rs:71); zero in any non-allowed cfg(test) region. make check 2059/2059 green.
3. Doc-comment matches (no "skipping cfg(test)" claim; the no-skip rationale is documented).
4. Test+doc only; prior M-1 KATs (flip + income-show order) intact; walk/is_src_descendant byte-untouched (r2 cross-crate proof stands).

## FINDINGS
CRITICAL — none.
IMPORTANT — none.
MINOR
- M1-r3 — the `use serde_json::to_value` disjunct is DEAD (superstring of `serde_json::to_value`); detection power unchanged; the commit/doc overstate "hardened." Optional tidy: drop it, or replace with `use serde_json::{` (which closes the braced-import hole but would false-positive on innocent braced imports). Non-gating.
NIT
- N1-r3 — `split("//")` false-negatives a pattern after a `//` inside a same-line string literal (contrived; pre-existing).
- r2 N1/N2 stand.

## VERDICT
GREEN — 0 Critical / 0 Important. r2-I1 correctly + minimally folded (skip deleted — the safe direction; flagged case now reds, re-proven; no false positive; doc truthful; 2059 green). The Minor (dead disjunct) does not gate.
