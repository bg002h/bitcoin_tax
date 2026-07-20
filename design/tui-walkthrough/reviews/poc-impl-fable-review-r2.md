# Re-review (r2) — TUI screen-walkthrough PoC fold

_Reviewer: Fable (independent). Scope: the r1 fold (commit `216f1e3`) — verify each r1 finding is
genuinely resolved and that the fold introduced no new blocking defect. Persisted verbatim before
folding r2, per STANDARD_WORKFLOW §2._

**VERDICT: NOT GREEN — 0 Critical / 1 Important** (all nine r1 findings are genuinely resolved; the fold's C-1 gate has one residual hole vs. the r1 fix-ask's explicit predicate, and the fold's new in-tree claims over-state it — demonstrated empirically below).

### Per-finding confirmation

| r1 | Status | Evidence |
|---|---|---|
| **C-1** | **RESOLVED** (all four enumerated harms; residue filed as new I-1 below) | Gate is real and executes: `xtask::bin/xtask examples::tests::walkthrough_manifests_valid_and_complete` (`crates/xtask/src/examples.rs:1303-1372`) runs under `cargo nextest run --workspace` (= `make check`; confirmed via `nextest list`) AND in CI's 3-OS `test` job (`cargo test --workspace` runs bin-crate unit tests; CRLF-safe on Windows — it parses via `str::lines()`). Demonstrated red on: dropped `FRAME 04` line (orphaned golden — the exact r1 hole; full bijection assert fired), typo'd reference, caption-less FRAME, duplicate refs/grammar (cp-backup/restore; tree clean). CI `%PDF` step added to the advisory `examples` job (`.github/workflows/ci.yml:134-140`). False Makefile comment corrected (`Makefile:85-89`). SPEC §5 As-built amendment (`SPEC_tui_walkthrough.md:131-146`) + PLAN "As-built deviations" (`IMPLEMENTATION_PLAN_tui_walkthrough.md:74-99`) are honest, reasoned, and map correctly onto plan steps 1/2/3-4/5. |
| **I-1** | **RESOLVED** | `Makefile:97` `|| exit 1` is inside the `{ … } > file` brace group, which runs in the recipe shell (not a subshell) — proven empirically: planted a bad `j0` journey sorting *before* `j8`; `make tui-walkthrough` → `make: *** [Makefile:94] Error 1` (pre-fold the last iteration masked it). Baseline run still succeeds; `\m[` + `%PDF` guards intact. |
| **M-1** | **RESOLVED** | `j8/manifest.txt:9` now says opening changes nothing / "the vault is written only when you confirm here" — matches frame 03's own footer "[Enter] Apply — writes the vault [Esc] Cancel — writes nothing". |
| **M-2** | **RESOLVED** | Caption now "Holdings, the 0.10 BTC has landed at Coinbase" — frame 04 shows exactly that (coinbase row, 0.10000000, basis 4000.00, "transferred"); no BALANCED token. TOTAL 40000.00 backs the weighted-avg prose. |
| **M-3** | **RESOLVED** | PLAN:80-84 records three-not-four editor frames as a deliberate choice with rationale (viewer frame + existing e2e cover; SPEC §10 latitude). |
| **M-4** | **RESOLVED** | `assemble-walkthrough.sh:35-42` fails closed — demonstrated for both `FRAME file` (no caption) and `FRAME file␣␣␣` (whitespace-only): exit 1 both. xtask gate mirrors it. |
| **N-1** | **RESOLVED** | Script header cites SPEC §5; zero `§4.4` hits remain. |
| **N-2** | **RESOLVED** | `seed_j8_vault` (`btctax-tui-edit/src/main.rs:14300-14304`) delegates to `testonly::seed_journey`, whose body (`testonly.rs:181-197`) is the old inline code verbatim (same `vault.pgp`/`key.asc` in `dir`, same corpus-order import) — equivalence **by construction**, and **empirically**: the editor gate re-captures fresh through the new path each run and byte-matches the unchanged committed frames (ran it: PASS). Not luck — the gate is a fresh capture+compare, so a state divergence would red it. |
| **N-3** | **RESOLVED** | Comment now "any market-value cell a frame might show"; no `8137` in crates (the CSV hit is the raw daily-close dataset). |

### New findings

**Important (NEW-I-1). The bijection pins manifest⇄disk, but nothing pins disk⇄capture-fns — the r1 C-1 fix-ask's predicate ("every FRAME reference names a golden *some crate byte-gates*") was narrowed to "names a golden *on disk*", and the fold's new comments over-claim the difference.** Demonstrated empirically: emptied the viewer capture fn (`tabs/tests.rs:944` is the crate's *only* frame tuple) → **all three gates PASS** — the crate gate loop (`tabs/tests.rs:949`, same shape at `main.rs:14363`) iterates only over captured frames with no floor/expected-stem assertion (vacuous pass), and the bijection still holds — while the orphaned `04-holdings-balanced.txt` keeps rendering into the PDF, never re-verified against the TUI (restored; tree clean). An accidental tuple loss during Phase 2's ~10× mechanical rollout ships a silently stale screen forever. Meanwhile the fold *added* the claim "together they pin the whole artifact" in four places (`Makefile:87-88`, SPEC §5 As-built ~:144, `assemble-walkthrough.sh:5-7`, `examples.rs:1315`; echoed in `ci.yml:135-137`) — false for exactly this case, the false-gating-claim pattern that elevated r1's C-1. Cheap fix: per-crate expected-stem consts (or per-journey count floors) asserted in each crate's gate, or each crate asserting its captured set covers its slice of the on-disk stems; and/or soften the claims.

**Minor (NEW-M-1).** The SPEC §5 amendment doesn't reach the struck design's other mentions: §8 deliverables 4+7 (`SPEC:198-206`) still mandate the xtask subcommand + `walkthrough.md` + Rust-literal captions with no As-built marker; §9:214 and §11:239 still reference "the xtask manifest". A Phase-2 executor reading §8's checklist could rebuild the struck design. (Residue of C-1(d); doc-consistency.)

**Nit (NEW-N-1).** `examples.rs:1320` `if !root.is_dir() { return; }` bypasses the `journeys_checked >= 1` floor — deleting the whole `docs/examples-tui-walkthrough/` dir passes the xtask gate vacuously (today backed by the crate gates panicking on missing committed frames, so not independently reachable; but let the floor fail instead of returning).

**Nit (NEW-N-2, process).** The r1 review was persisted in the *same* commit as the fold (`216f1e3`), not a separate prior persist commit (repo habit: separate `docs(reviews): persist … verbatim`). Content is verbatim; noting for the record.

### Suite state
`make check` green (2070 passed, clippy clean), `cargo fmt --check` clean, `pii-scan-generic.sh` exit 0, baseline `make tui-walkthrough` writes a valid PDF. Working tree byte-identical to the fold commit; all demo mutations cp-backup/restored.
