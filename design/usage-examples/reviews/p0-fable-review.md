# Fable independent review — P0 `BTCTAX_NOW` seam (persisted verbatim)

*Persisted 2026-07-16 verbatim, per STANDARD_WORKFLOW §2. Reviewer: Fable (independent). Verdict: GREEN —
0 Critical / 0 Important. P0 ready; golden-recording unblocked. 2 Minor + 1 Nit folded/recorded below.*

---

# P0 Independent Review — `BTCTAX_NOW` clock-injection seam (Fable, 2026-07-16)

**Scope reviewed:** `git diff b45c878..HEAD` (3 commits: 909ded7, e5a182f, 27b43f7) against SPEC §3.2 (R-P0.1..6, T-P0.1..6) + the §3.1 (i)/(ii)/(iii) fence. All claims below verified against source **and** by execution.

## VERDICT: GREEN — 0 Critical / 0 Important. P0 ready; golden-recording unblocked.

(2 Minor + 1 Nit recorded below; none holds the gate.)

## Verified correct (terse)

- **Fence (i)/(ii)/(iii).** Unset arm of `resolve_now()` (`crates/btctax-cli/src/main.rs:70-72`) is `now_utc()`, no banner, no other side effect; the sole run-path change is `main.rs:92` (`let now = resolve_now()?;`). `git diff b45c878..HEAD --stat` over core/tui/tui-edit/update-prices/store is **empty** — CLI-only (R-P0.5). Injects an input; transforms no output. T-P0.1 pins the inactive path.
- **Sole clock read.** `grep now_utc` over `btctax-cli/src` finds exactly one hit — inside `resolve_now()`. No bypass.
- **R-P0.1..4 by execution.** Strict `well_known::Rfc3339` parse; `CliError::Usage` → exit 2 via `run_to_exit` (`main.rs:40-48`). Probed live: `"not-a-date"`, `""`, `" "` (whitespace), `"2025-07-01"` (date-only) → all exit 2 naming `BTCTAX_NOW` + expected format. Non-UTF-8 handled in code. Banner is byte-exact (`warning: BTCTAX_NOW override active — decision timestamps are simulated`), on stderr, emitted with stderr piped (proves not TTY-gated). R-P0.2: `BTCTAX_NOW` appears nowhere in `cli.rs` and `--help | grep -c BTCTAX_NOW` = 0.
- **Weird-but-valid offsets are sane.** `+05:30` parses, banner fires, and downstream honors the instant: `set_forward_method` derives the date via `now.to_offset(UtcOffset::UTC).date()` (`cmd/reconcile.rs`), and `tax_date()` converts via `to_offset(tz)`. Lowercase `t`/`z` accepted — RFC3339-conformant.
- **Tests: 6/6 green** (`cargo test -p btctax-cli --test btctax_now_seam`). Setup exit codes all asserted in both `vault_with_election` and `accept_under` — including the final `accept` code, which the plan draft lacked; the committed test is stronger than the plan. T-P0.2/T-P0.1 fail without the seam: `verify` prints `recorded 2025-05-01 effective … -> METHOD` (confirmed live); without the seam the made-date would be 2026-07-16 and `contains("2025-05-01")`/`contains("recorded 2025-05-01")` go red.
- **T-P0.6 non-vacuity independently proven** (no file mutation needed): I replicated the exact KAT journey in a scratch dir. Backdated arm: `Optimize accept — 1 persisted, 0 skipped.` + `PERSISTED … [Contemporaneous]`; postdated arm: `0 persisted, 1 skipped` + `skipped …: already executed — re-run … --attest`. The **entire** diff between arms is the persistability outcome (persisted vs attest-gated skip) — so `assert_ne!` → `assert_eq!` goes RED, and the KAT exercises persistability itself, not incidental output. The 2025-06-01 sale keeps `ForbiddenBroker2027` dead as claimed (`optimize.rs:476-480`).
- **Man pages.** `ROOT_ENVIRONMENT` wired into `render_root` between SUBCOMMANDS and FILES (man-pages(7) conventional order); `\(sc` renders as `§ 1.1012-1(j)` (confirmed via `man -l`); zero new groff warnings (the two `-ww` warnings at `btctax.1:79,82` are pre-existing em-dashes at b45c878, silenced by `man`'s preconv). Text accurate: RFC3339 + example, exit 2 on malformed/empty, stderr banner, §1.1012-1(j) caveat matching the memory'd NOTICE posture (disclaims, never restricts).
- **909ded7 drift fix correct and complete.** `btctax-update-prices.1` is inside `render_generated_pages()`, so the drift (page says v0.6.0, crate at 0.6.1 since 57e468c) made `gen_docs_is_deterministic` red on main; regen fixes it, and the now-green determinism test (`cargo test -p xtask`: 5/5) proves **all** pages match a fresh generation. FOLLOWUPS UX-P0-3 + the plan's version-bump-step amendment accurately record the process fix.
- **No regression.** `make check`: **1940 passed, 5 skipped** — matches expectation.
- **Prompt Q6 (report-byte-identity test):** judged YAGNI. Fence (i) is "seam-inactive == pre-seam", which T-P0.1 pins; "`report` output is independent of `now`" is a property of `report`, not the seam, and may legitimately change (as-of rendering). Not a gap.

## Findings

### Critical — none.

### Important — none.

### Minor

1. **T-P0.6 pins difference, not direction.** `crates/btctax-cli/tests/btctax_now_seam.rs:169-175` asserts only `back != post`; SPEC T-P0.6 states "backdated ≤ sale ⇒ `persistability` yields `ContemporaneousNow`". A direction-flip mutation in `optimize.rs`'s `made ≤ sale` lever survives this KAT (arms swap but still differ). It **does not gate** because the direction is pinned elsewhere on the same validation surface — `btctax-core/tests/optimize_compliance.rs:237-263` (library) and `btctax-cli/tests/optimize_accept.rs` (CLI render, library-injected `now`) — and the KAT's unique job (the seam reaches the persistability gate through the real binary) is genuinely pinned. Fix (2 lines, next touch of the file or fold into P1): add `assert!(back.contains("[Contemporaneous]"), …)` and `assert!(post.contains("0 persisted"), …)` before the `assert_ne!`.
2. **Banner pinned by substring, not the full R-P0.4 line.** `btctax_now_seam.rs:96` asserts `contains("BTCTAX_NOW override active")`; the integrity-disclosure tail "— decision timestamps are simulated" is unpinned, so a wording-drift mutation survives, and R-P0.6(b) reads "the R-P0.4 banner, pinned by test". Presence/stderr/unconditionality — the gate-relevant behaviors — are pinned. Fix (1 line): assert the full exact line `warning: BTCTAX_NOW override active — decision timestamps are simulated` in T-P0.4.

### Nit

3. **Non-UTF-8 arm untested.** `main.rs:75-76` handles it, but no test exercises it (spec T-P0.3 demands only malformed/empty, so this is conformant). A unix-gated test via `Command::env("BTCTAX_NOW", OsStr::from_bytes(&[0xFF]))` would pin it if ever wanted.

---

**The single most important thing:** T-P0.6's *directionality* (backdated ⇒ Contemporaneous, not merely ≠ postdated) currently rests on the core/CLI library-level tests, not the binary-level KAT itself — the two `contains` asserts in Minor #1 are the cheapest way to make the KAT self-sufficient before P1 starts recording goldens that lean on it.
