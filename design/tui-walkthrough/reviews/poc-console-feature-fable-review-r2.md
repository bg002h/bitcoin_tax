# Re-review (r2) — walkthrough exact-CLI-I/O feature fold — GREEN

_Reviewer: Fable (independent). Scope: the r1 fold (commit `febbbf7`) — verify I-1/M-1/M-2/N-1 resolved and
no new blocking defect. Persisted verbatim per STANDARD_WORKFLOW §2. GREEN, so no fold follows; the one
informational observation is recorded for Phase 2 (cfg pattern for the console generators)._

## VERDICT: GREEN — 0 Critical / 0 Important

The fold commit (`febbbf7`) is byte-identical to the diff under review (verified by `diff` against `git diff 144106e..febbbf7`), all four findings are genuinely resolved, and I found no new Critical/Important defect. Zero committed golden bytes changed (`git diff eedb445..HEAD` on `docs/examples-tui-walkthrough/j8/` and `docs/examples/examples.md` is empty).

## Per-finding

**I-1 — RESOLVED.** `plain_with_stderr` (`crates/xtask/src/examples.rs:213-220`, correctly `#[cfg(test)]`, as is its only caller `generate_j8_walkthrough_console` at :1025) sets `show_stderr: true` on all four setup steps (:1037, :1043, :1049, :1056). Traced `emit` (:158-165): stderr is appended verbatim in a labelled fenced block only when non-empty, and `capture` (:93-116) returns stdout+stderr from the **same single invocation under the pinned env** — same `env_remove("BTCTAX_NOW")`, and all four steps use `now: None`, so no banner. Proof the golden is byte-unchanged today: `walkthrough_console_golden_matches_committed` **PASSED** (regen-with-stderr == committed, which simultaneously proves stderr is empty today — the committed golden contains no `stderr:` block). A future nondeterministic stderr line would false-RED loudly rather than silently diverge — correct gate polarity, and exactly the r1-prescribed fix. No accidental `show_stderr` flip elsewhere: grep shows `true` only at pre-existing :612/:704/:853 (untouched), and `examples_golden_matches_committed` **PASSED**.

**M-1 — RESOLVED.** Empirically demonstrated both directions. Hostile fragment (`---`, `<!--`, `.TH fake`, `## not-a-heading`, plus an `emit`-shaped `stderr:` inter-fence label) through `awk -v fragment=1 -f docs/examples/man-wrap.awk`: every line renders verbatim, `.TH fake` is `\&`-escaped (so the `grep -v '^\.TH '` backstop can't eat it), `## ` is protected by `inpre==1`, no BEGIN `.TH` emitted. Contrast on the pre-fold awk: content after `---` is swallowed. Regression: new awk over the real `examples.md` is **byte-identical** to the pre-fold awk's output. The CONSOLE arm passes `-v fragment=1` (`assemble-walkthrough.sh:63`). No other document-level rule misfires in a fragment: `fm==1`/`incomment==1` states are unreachable when their setters are `!fragment`-guarded (man-wrap.awk:37, :43).

**M-2 — RESOLVED.** The doc comment (`examples.rs:1393-1407`) now states the three-directive grammar, the two bijections, the exactly-one-class partition (matching the unexpected-file panic at :1505), and the console regen gate. The Makefile header (`Makefile:85-92`) lists all three gated artifact classes with their real test names — each named test exists and passes. Nothing newly false.

**N-1 — RESOLVED.** The `"`/`\` assert is present for **both** CONSOLE (:1448-1452) and FRAME (:1469-1473). Mutation drills (cp-backup/restore, tree left clean): a `"` in the CONSOLE caption REDs; a `\` in a FRAME caption REDs. The committed manifest passes (no over-reach). `\` is rightly rejected for CONSOLE too — the caption lands inside `printf '.SH "%s"'` where roff would interpret it; PROSE remains the sanctioned home for roff markup.

## New findings

None blocking. One non-gating observation (informational, not a finding requiring a fold): `plain_with_stderr` and `generate_j8_walkthrough_console` are `#[cfg(test)]` but their only consumers live in the `#[cfg(test)] #[cfg(unix)]` tests module (:1306-1307), so a **Windows** test compile would raise a `dead_code` *warning*. It cannot RED anything: clippy `-D warnings` runs on ubuntu only (ci.yml:40,50), the OS matrix runs plain `cargo test --workspace --locked` with no RUSTFLAGS, and the pattern pre-exists from `eedb445`. [Author note: folded into Phase 2 — the console generators will use `#[cfg(all(test, unix))]`.]

## Whole-surface checks

Frame goldens both PASS; `make tui-walkthrough` renders end-to-end; manifest gate green; `git status --porcelain` empty at finish.

**GREEN.** The fold is sound and this feature is clear to close.
