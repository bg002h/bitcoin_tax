# Phase 7 (#21) adversarial review — N-R1 de-stick + UX-P3-2 colorized TUI PDF

Reviewed at HEAD `34bf318` (`41a771a` + `34bf318` over `099f16b`); everything below re-derived from current source and executed locally. Working tree left clean (verified `git status` after the mutation experiment).

## What I verified clean (evidence)

**N-R1 (`41a771a`) — sound, non-vacuous, mutation-proven:**
- Both copies of `production_now_utc_lines` (`/scratch/code/bitcoin_tax/crates/btctax-tui/src/export.rs` ~1008, `/scratch/code/bitcoin_tax/crates/btctax-tui-edit/src/main.rs` ~14101) are logically identical: skip from `#[cfg(test)]` to the next column-0 `}`, then resume and re-enter on any later `#[cfg(test)]`. A production `now_utc()` after a test module is now caught.
- KATs pass: `cargo test -p btctax-tui --lib now_utc_scan_desticks` and `cargo test -p btctax-tui-edit --bin btctax-tui-edit now_utc_scan_desticks` both green; both real `no_direct_now_utc_in_production` scans green.
- **Mutation:** reverted the export.rs helper to sticky `in_test` (dropped the dedented-close reset) → `now_utc_scan_desticks_past_a_test_module` REDS. Restored via cp-backup; tree clean.
- **Fixture self-safety:** every fixture line is an indented string literal — `trim_start()` sees a leading `"`, so neither `#[cfg(test)]` nor the column-0-`}` heuristic can fire on it; the `now_utc(`-bearing fixture lines sit inside the real `mod tests` span. Verified structurally.
- **Soundness sweep of the column-0-`}` heuristic across every walked file** (both crates' `src/**.rs`): each top-level `#[cfg(test)]` annotates a braced `mod tests` whose ONLY column-0 `}` after it is the file's final line (audited all 17 files with such a marker). So no interior column-0 `}` (raw string, macro body) exists today; and note an early exit would produce a false POSITIVE (test code scanned as production — loud), not a silent false negative. The only silent-false-negative geometry is a `#[cfg(test)]` on a non-braced item — see Minor 1.
- **Latency claim confirmed:** every braced test module runs to EOF, so no file currently has production code after a test module — the old bug was latent for the real tree, and the KAT is the sole holder of the new behavior. It holds.
- `tabs/tests.rs` (whole-file test module, no `#[cfg(test)]` line of its own) is scanned as all-production — conservative direction; its `now_utc` mentions are doc-comments only.

**UX-P3-2 (`34bf318`) — mostly right, two real defects (below):**
- Glyph map covers the complete enumerated inventory across all four goldens (`grep -oP '[^\x00-\x7f]'` over glyph sections): `─ │ ┌ ┐ └ ┘ ▲ ← ↑ → ↓ —` — all mapped; `├┤┬┴┼` mapped-but-unused. No unmapped glyph, so no map-gap alignment shift (under a UTF-8 gawk).
- Colors are only `Cyan/Red/Yellow` → `tolower` → groff-predefined `cyan/red/yellow` (the `fg=Reset`/`bg=Reset` grep hits are the styles-section header text, not runs). Modifiers present: `BOLD`, `BOLD|REVERSED`, `BOLD|UNDERLINED`.
- Escapes are balanced: each transition closes bold (`\f[CR]`) then color (`\m[]`) before opening the new pair; row end closes both; `\&` protects a leading `.`/`'`; no golden contains a backslash or a leading-control row (checked).
- Prefix strips align between sections (both `  N│`; glyph position 1 = screen x0 in both).
- `make examples-tui` → 4-page `%PDF`, and the decompressed content streams contain real color operators (`0 1 1 rg`, `1 0 0 rg`, `1 1 0 rg`). The `▲`→`^` cell IS inside the cyan run.
- Makefile `\m[` guard is effective against a monochrome regression (grep -qF before groff, exit 1). CI's `examples` job runs `make examples-tui` (`.github/workflows/ci.yml` ~line 133) — commit claim true.
- Gated artifact untouched: diff = exactly `Makefile`, `main.rs`, `export.rs`, `tui-wrap.awk`; no `.txt` golden.
- Suite claims accurate: `make check` = **2065 passed / 8 skipped**; `cargo fmt --check` clean; `scripts/pii-scan-generic.sh` exit 0; `xtask check-isolation` OK.

## CRITICAL

None. The N-R1 scan cannot be fooled into a false negative in the current tree, and the PDF is the non-gated convenience artifact.

## IMPORTANT

**I-1 — UX-P3-2 misreads the run format: every style run bleeds one cell LEFT (`tui-wrap.awk:52`).**
The golden's `start..end` is 0-based, end-EXCLUSIVE — `capture.rs` writes `{start}..{x}` with `x` one past the run's last cell, and its own unit test pins a 2-cell "Hi" at x0–x1 as `0..2` (`/scratch/code/bitcoin_tax/crates/btctax-tui/src/capture.rs:43–53,132`). The awk paints `for (c = start; c <= end; c++)` against 1-based `substr` positions; correct is `start+1..end`. The two off-by-ones cancel at the right edge only, so every run with `start > 0` paints one extra cell to its left (and at adjacent-run seams the left run's final cell is overwritten by the right run's style). Demonstrated on real goldens:
- browse row 1, run `2..10 fg=Red mod=BOLD|REVERSED` (truth: exactly "Holdings", x2–x9) renders `|\m[red]\f[CB] Holdings\f[CR]\m[]` — the space at x1 is red/bold;
- holdings row 1, run `32..49 fg=Cyan…` (truth starts at 'A' of "Acquired", x32) opens cyan at the space x31; row 1's `1..31 mod=BOLD` bolds the left border cell x0.
Fix is one token: `for (c = start + 1; c <= end; c++)`.

**I-2 — the multibyte character class breaks under CI's actual awk (`tui-wrap.awk:67`).**
`gsub(/[┌┐└┘├┤┬┴┼]/, "+", line)` is a bracket class of multibyte glyphs. In a byte-oriented awk it degrades to a class of the constituent BYTES {E2, 94, 8C, 90, 98, 9C, A4, AC, B4, BC}: each corner becomes `+++` (row width 124 vs 120 — demonstrated with byte-mode awk: row 0 renders `+++ Holdings…`), and because this gsub runs before the `←↑→↓▲—` rules and shares bytes `E2`/`94` with them, those glyphs are half-eaten into raw high bytes the later gsubs can no longer match. The CI `examples` job runs `make examples-tui` on `ubuntu-latest`, whose `/usr/bin/awk` is **mawk** (byte-oriented; `gawk` is not in the runner image's installed-apt list — I fetched the ubuntu-24.04 manifest — and Ubuntu's base provides mawk). The `\m[` guard and the `%PDF` check both still pass, so CI silently "proves" a garbled render; the header's "cell-accurate in any awk" claim is unsound for the map itself (the single-glyph gsubs like `gsub(/─/,"-")` ARE byte-sequence-exact everywhere — only the class is not). Fix: replace the class with nine single-glyph gsubs in the style already used for `─ │ ← ↑ → ↓ ▲ ▼ —` (or force gawk + a UTF-8 locale in the recipe).

## MINOR

**M-1 —** `/scratch/code/bitcoin_tax/crates/btctax-tui/src/tabs/mod.rs:14–15` is `#[cfg(test)]` on an UNBRACED `mod tests;`, contradicting the helper doc's "Assumes `#[cfg(test)]` annotates a braced `mod` (true here)". Harmless today (it is the file's last item), but this geometry is the one remaining silent-false-negative class: `in_test` sticks until a column-0 `}` that may only arrive after swallowing a subsequently-added production item. Correct the doc (drop "(true here)") or special-case a following `mod X;` line as a zero-span skip.

**M-2 (pre-existing, unchanged by N-R1) —** the `//`-strip is string-blind: `foo("https://x"); let t = now_utc();` on one line is silently missed (`split("//").next()` truncates at the URL). Same in the old scan; recorded for completeness, not a regression.

## NIT

**N-1 —** header comment (`tui-wrap.awk:16–17`) says UNDERLINED/REVERSED "are approximated as bold", but the code bolds only `mod ~ /BOLD/` — a standalone UNDERLINED/REVERSED run would render plain. Unobservable today (every such run carries BOLD), but the sentence overstates.
**N-2 —** if groff fails, `docs/pdf/.tui-screens.roff` is left behind (the `rm -f` is a later recipe line); gitignored, harmless. A mid-loop awk failure on a non-final golden is swallowed by the `for` (pre-existing recipe shape).
**N-3 —** the `\m[` guard would false-fail a legitimately colorless future golden set — loud, acceptable as designed.

## VERDICT

Fold before green: **I-1** (paint `start+1..end` in `tui-wrap.awk:52`) and **I-2** (replace the multibyte bracket class at `tui-wrap.awk:67` with per-glyph gsubs, or pin gawk+UTF-8 in the Makefile recipe). Both are small, both in the same file; N-R1 is clean as shipped. Not GREEN: 0 Critical / 2 Important.
