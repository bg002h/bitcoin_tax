# Independent re-review — M-1 fold (r1-I1): enumeration-scan tripwire + spec §9.6 reconcile

Reviewed `git diff 7284d6f..HEAD` against a fresh independent audit. Mutations run on HEAD, tree restored clean.

## Verified clean
- Fold shape: 3 files (test, spec, r1 review persist); no production code / golden change. Both prior M-1 KATs intact; flip KAT doc folds r1-N1 ("from ALL serde_json deps").
- Cross-crate reach PROVEN (CARGO_MANIFEST_DIR.parent()=crates/; a to_value probe in btctax-tui/src/export.rs reds; tests/ excluded; no build.rs).
- ALLOWED enumeration MATCHES an independent grep exactly (tax.rs:195; oracle-harness main.rs json! sites; coverage.rs:71). update-prices parse-only; btctax-forms Value is toml::Value; forms/xtask serde_json-free.
- Spec §9.6 accurate (typed-persistence citation persistence.rs:164-165 exact; oracle-harness omission honestly recorded).
- Non-vacuity above the trip point PROVEN (json! in render.rs top reds).
- make check green 2059/2059.

## FINDINGS
CRITICAL — none.

IMPORTANT
- I1-r2 — The `in_test` flag is STICKY (never resets): once a line matches `#[cfg(test)]`, every subsequent line of that file is skipped, leaving real production blind regions. render.rs has mid-file test mods (line 2475) with production AFTER them (`render_events_list` ~3708) — a `json!` there PASSES the scan (demonstrated false negative; the author's mutation only probed pre-2475). spec/mod.rs is blind from line 7 to EOF. AND: an unfiltered grep shows ZERO to_value/json! inside any cfg(test) region today (coverage.rs is gated at its parent mod, invisible to file-level skipping, already ALLOWED) — so the skip excludes nothing and only manufactures blind spots. Fold: delete the `in_test` logic (test passes identically; a future test-mod hit in a non-allowed file reds loudly — the safe direction); re-prove the mutation INSIDE render_events_list; fix the doc comment.

MINOR
- M1-r2 — pattern gap: matches only `serde_json::to_value`/`json!(`; would miss a `use serde_json::to_value` bare call, hand-built Map/Value::Object output, or parse-then-reserialize. None exist today; typed-serde invariant independently held. Cheaply hardened by also matching `use serde_json::to_value`.

NIT
- N1-r2 — a string/comment containing the patterns false-positives (loud/safe direction).
- N2-r2 — allowlist `contains` is file-granularity suffix-loose (consistent with the audit).

## VERDICT
1 Important (I1-r2) to fold — un-stick the cfg(test) skip (deleting is minimal+correct; excludes zero hits today) and re-prove in render.rs post-2475. Else green.
