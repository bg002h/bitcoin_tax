# BRIEF — FR-174: the last 5 CI failures, both causes already diagnosed

**Tier:** sonnet. **Isolation:** your own worktree. **One agent.** No subagents.
**BLOCKING** — CI is red. **Budget: small.** Both root causes are found; you are implementing, not
investigating. Do not widen this.

## 0. State — FR-165 already fixed the big one

Run `34762577861` (`08fde5d26`): **`test (ubuntu-latest)` is GREEN.** macOS `216 passed; 1 failed`.
Windows `201 passed; 4 failed` (was 10). The 5 that remain are platform assumptions, red the whole 8
days, invisible behind the louder failure. FR-174 in `FOLLOWUPS.md` has the full measurement.

## 1. Cause A — a path key whose separator is platform-dependent (3 Windows failures)

`crates/xtask/src/line_coverage_check.rs`:
- `:1078` `CAPTION_PARAPHRASES` is keyed by hardcoded forward-slash strings, e.g.
  `"crates/btctax-forms/forms/2024/f1040sa.map.toml"`.
- `:1372` the scanned path is `root.join("crates/btctax-forms/forms")`, then joined per year — so on
  Windows it renders `crates/btctax-forms/forms\2024\f1040sa.map.toml`. The literal prefix keeps `/`; the
  joined segments get `\`.
- `:1455` the lookup therefore misses, and **both** directions fire: the caption reads as an unexcused
  paraphrase, and `:1474`'s staleness check reads the excuse row as describing nothing. That is the 4
  problems the run printed — 2 captions × 2 directions.

Failing: `the_committed_coverage_table_is_consistent_with_the_form_text`,
`the_obbba_line_1_rows_are_verbatim_and_a_moved_cross_reference_reds`, and
`each_rule_rejects_a_table_that_violates_it` — the last because its **control** table fails first, and its
own message says why that matters: *"the control table must PASS — otherwise every plant below passes for
the wrong reason."*

**Fix:** normalise separators on both sides of the comparison. **Do NOT** retype the keys with `\`, and do
not add a second Windows-only key list — that is the disease, not the cure. ★ Check whether any *other*
string-keyed lookup in this file compares a hardcoded `/` path against a `join`ed one; `:1613` builds
`format!("crates/btctax-forms/forms/{}/{}.map.toml", …)` and is worth a look. Fix what you find in **this
file**; if the same shape exists elsewhere, report it rather than chasing it.

## 2. Cause B — `/tmp` hardcoded as the fallback temp dir (1 macOS + 1 Windows failure)

`crates/xtask/src/main.rs`, `default_proof_path` + `the_label_proof_default_path_honours_tmpdir`
(`:690-716`): the function returns the literal `"/tmp/<stem>-label-proof.pdf"` when `TMPDIR` is unset, and
the test pins that literal.

- macOS temp dir is `/var/folders/…/T/` — the run reported left `/var/folders/36/…/T/f8995a--2025-label-proof.pdf`, right `/tmp/f8995a--2025-label-proof.pdf`.
- Windows temp dir is `C:\Users\…\Temp\` and **does not consult `TMPDIR`**, so on Windows the test's *first*
  assertion fails as well.

**Fix:** use `std::env::temp_dir()`. It honours `TMPDIR` on unix and `TMP`/`TEMP` on Windows, **and still
returns `/tmp` on Linux when `TMPDIR` is unset** — so the operator guarantee the test exists to protect
survives on the platform operators actually use. The test then compares against `std::env::temp_dir()`
rather than a literal; keeping an additional `#[cfg(target_os = "linux")]` assertion of the literal `/tmp`
is welcome, because that is the promise worth pinning.

★ Frame this correctly: writing a label proof to `/tmp` on Windows is **the tool being wrong**, not the test
being fussy. Fix the function, then the test.

## 3. Scope

**IN:** `crates/xtask/src/line_coverage_check.rs` and `crates/xtask/src/main.rs`. Nothing else.

**OUT:** `form_delta.rs` / `form_geometry.rs` (FR-165 just landed); `scripts/oracle/*` and
`golden_returns.rs` (FR-164); the `CAPTION_PARAPHRASES` *entries* themselves — both rows are legitimate and
stay; any sweep for other platform assumptions (report, don't chase).

## 4. B1 — the kill you can actually run on Linux

You cannot run Windows here, so prove it the way the defect works:
1. **Cause A:** write a test that builds the path the way Windows does — with a `\` separator — and asserts
   the excuse lookup **still matches**. That test must RED before your fix and GREEN after. Paste both.
   ★ This is the honest kill: it reproduces the *mechanism* on any platform rather than requiring the OS.
2. **Cause B:** set `TMPDIR` to a temp dir and assert the path lands there; unset it and assert the path is
   `std::env::temp_dir()`-derived. Then plant the old literal back and watch the new test red. Paste both.
3. Re-run `each_rule_rejects_a_table_that_violates_it` and confirm its control passes and its plants run.

## 5. Stop-and-report

**Six briefs in this arc carried a premise the implementer refuted, three written by this controller** — the
most recent two hours ago. If you can disprove anything above — that `:1455` is the missing lookup, that
`temp_dir()` returns `/tmp` on Linux with `TMPDIR` unset, that 3 of the 4 Windows failures share one cause —
**stop and report** rather than building on it.

## 6. Working rules

- Your own worktree. `CARGO_TARGET_DIR=<your-worktree>/target-fr174` (covered by `.gitignore`'s `target-*/`).
  Never a target dir in `/tmp` — 32 GB tmpfs, one filled it and killed a running test.
- `make check` for the gate; capture once to a file and grep it, never run a suite twice.
- ★ `make check` does **NOT** include `cargo fmt --all --check`. Run `cargo fmt --all` before finishing — the
  pre-commit hook blocked a commit on exactly that today.
- **Do not commit, do not push.** Leave the worktree dirty; name every file you changed.

## 7. Deliverable

Final action: write your report with a Bash heredoc (`cat > <path> <<'MARKER'` … `MARKER`), **not** the
`Write` tool, to:

    design/agent-reports/REPORT-build-fr174-ci-platform-assumptions.md

Return only a short summary plus that path. State: what changed; both kills, red and green, pasted; whether
any premise in §5 failed; and anything you found but deliberately left alone.
