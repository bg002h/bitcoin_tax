# Controller's machine-check ledger — `…r2-fold-review-r2.md` (0C/3I/3M), checked at `ccdc53eb`

| finding | claim | check | verdict |
|---|---|---|---|
| G1 | every top-level map struct is `deny_unknown_fields` (55 in `map.rs`), so an `authority` header key REFUSES rather than excuses | `grep -c deny_unknown_fields` = **55** | HOLDS |
| G1 | §4's header block has no `authority` / `extract_override`; §4's kill bullets carry no exceptions | r2 `:64-74`, `:93-97` read | HOLDS |
| G2 | ten TY2025 maps have no struct until step 5 | `fn ty2025()` arms in `map.rs` = **5**; TY2025 `.map.toml` files = **15** → 10 unwired | HOLDS |
| G3 | the two core fixtures and the two extract-tree files differ (11,443 vs 11,153; 52,672 vs **616,274** bytes) and `tables.rs:1351,1365` are in-crate `include_str!`s | sizes and sha256 heads exactly as reported (`d361c281`/`ae0e3d50`/`0ba915a2`/`6d4d8c1e`); both lines are `include_str!("fixtures/…")` | HOLDS — the §9 "move" sentence would clobber the booklet extract and create an escaping `include_str!` |
| G4 | the five shared structs serve **15** maps, not ~13 | `ls forms/*/{f1040,f8949,f8283,schedule_d,schedule_se}.map.toml` = **15** | HOLDS |
| G5 | `printed.rs` structs carry no year field | `grep -c 'pub year' printed.rs` = **0** | HOLDS — the widened withdrawal blessed a cross-year struct |
| G6 | `ROADMAP_STATUS.md` says 4868 "join P4" one paragraph after the NOW row | `:187` | HOLDS (introduced by 3de5c272, not the fold) |
| step 1 "NO" | follows from G1 | — | ACCEPTED |

All six hold. Folded in the commit after this one.
