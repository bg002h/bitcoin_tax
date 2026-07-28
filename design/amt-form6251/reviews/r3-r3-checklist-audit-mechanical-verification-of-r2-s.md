# r3 review — r3 checklist audit — mechanical verification of r2's seven required closures, plus citation spot-check

**Headline:** CLOSED: 1, 2, 3, 4, 5, 7 (all six confirmed against plan text + repo). PARTIAL: 6 — the §5 discharge KAT is fully closed and owned, but the §6(ii) input-surface guard cites a mechanism (coverage.rs's mutate-and-diff) that, verified in-tree, structurally cannot prove what it's asked to prove. One unrelated Nit-level citation drift found while spot-checking line numbers.

## [Important] §6 (Global constraints, exhaustiveness bullet ii) / T2's KAT-list bullet ("the two §6 exhaustiveness guards") (prior r2 I-6)

**Problem:** §6 states the input-surface guard as: "(ii) input-surface — the eleven uncapturable items still have no ReturnInputs leaf, via spec/coverage.rs's existing mutate-and-diff mechanism." Verified against crates/btctax-input-form/src/spec/coverage.rs: the mutate-and-diff mechanism (doc comment lines 1-12; `leaf_map` at 68-76; the KAT body) works by serializing a maximally-populated `ReturnInputs` to JSON, walking every EXISTING leaf, and checking each is covered by exactly one `Field`'s `set` or is in the literal `EXEMPT_LEAVES`/`EXEMPT_PREFIXES` lists. A concept that was never given a struct field (ISO, §1202, depletion, etc.) never appears in `leaf_map`'s output — there is nothing to observe or diff, so the mechanism cannot assert non-existence of a leaf, only catch drift among leaves that already exist. It's also foreclosed as a workaround: the KAT's own "keep the EXEMPT lists LIVE" check (coverage.rs:262-279, confirmed) asserts every `EXEMPT_LEAVES`/`EXEMPT_PREFIXES` entry matches a REAL fixture leaf, panicking on "stale exemption" — so you cannot pin an absent concept by adding it to EXEMPT either. This is the same species of error r2 caught in C-A (an existing mechanism cited for a guarantee it cannot structurally carry), and r2's own I-6 fix text flagged this exact ambiguity ("say whether this extends spec/coverage.rs's mutate-and-diff KAT... or stands alone") without the plan answering it.

**Fix:** Point (ii) at a mechanism that can actually observe absence: e.g. a KAT that takes `coverage.rs`'s own `leaf_map(&ReturnInputs::default())` (or the maximal fixture) and asserts no key matches a literal blocklist of the eleven items' field-name patterns — reusing `leaf_map`'s infrastructure but not the mutate-and-diff Field-coverage algorithm. Or state plainly that (ii) is a documentation/code-review guarantee, not an automated KAT, and drop the coverage.rs citation. Either way, give it its own explicit T2 sub-bullet distinct from "the two §6 exhaustiveness guards" so the mechanism is named, not just gestured at.

---

## [Nit] §3 ("★ DECISION (r2 C-B)" — the exemplar cross-reference)

**Problem:** The plan cites `return_refuse.rs:1006` for "asserts reason == None for Some(false), commented 'No brick: the screen does not refuse a truthfully-answered mixed-use return.'" Verified in-tree: the comment is at line 1004 and the `assert_eq!(reason(&r), None, "{election:?}: must not refuse");` it explains is at line 1005; line 1006 is a blank line. Off by one — content is real and present, just one line up from where cited.

**Fix:** Retarget the citation to `return_refuse.rs:1004-1005`.

---

