# Phase 7 (#21) fold re-review r2 — colorization defects + scan soundness + PDF page-fit

Reviewed at HEAD `1d762fe` (fold over `34bf318`; r1 persisted verbatim in `2b7f11c` **before** the fold — process order correct). Everything below re-derived from current source and executed locally: awk renders under UTF-8 gawk AND `LC_ALL=C` (byte-mode mawk proxy — mawk itself is not in this host's Arch repos; `pacman -Si mawk` has no candidate), both `make` targets, groff→PDF, `pdfinfo`, full-rasterization ink-bbox measurement of all 24 pages, the KATs plus a live mutation, `make check`, and `cargo fmt --check`. Working tree verified clean after the mutation experiment (cp-backup restore, `git diff --stat` empty).

## Fold verification (r1 findings)

**I-1 (run-range off-by-one) — correctly folded, verified analytically and empirically.**
- `capture.rs` truth re-confirmed: `writeln!(… "{y:>3}│ {start}..{x} {sig}")` with `x` one past the run's last cell — 0-based, end-EXCLUSIVE (`/scratch/code/bitcoin_tax/crates/btctax-tui/src/capture.rs:43–56`). The awk now paints `for (c = start + 1; c <= end; c++)` with a comment stating exactly that mapping. Cell k → substr k+1, so [start,end) → substr [start+1,end]: correct.
- Ran the awk on every golden. Browse row 1 run `2..10 fg=Red mod=BOLD|REVERSED` renders `| \m[red]\f[CB]Holdings\f[CR]\m[] | Disposals…` — the escape opens on 'H' and closes after 's'; the space at x1 is unstyled (r1's bleed is gone). Holdings row 1 `32..49 fg=Cyan mod=BOLD|UNDERLINED` opens cyan exactly at 'A' of "Acquired" and spans 17 cells (`Acquired ^` + 7 pad); the flanking `1..31`/`50..67` bold runs land on their exact cells with the unstyled gap cells (x31, x49) plain.
- Run-seam check on the densest row (classify-modal row 12, adjacent runs `25..26 fg=Cyan` / `26..27 fg=Red` and `93..94 fg=Red` / `94..95 fg=Cyan`): renders `\m[cyan]+\m[]\m[red]|\m[]` at substr 26/27 and the mirror at 94/95 — close-then-reopen at the seam, no overwrite, no gap. Rasterized page 2 confirms the red modal border with cyan outer corners at the right cells.
- Width invariant: stripped all `\m[…]`/`\f[C?]`/`\&`/`\e` escapes from every rendered row and compared lengths against the goldens' cell counts — **all rows of all 4 goldens preserved exactly** (no widening, no truncation).

**I-2 (multibyte bracket class) — correctly folded, byte-mode proven.**
- The class is gone; nine single-glyph corner/tee/cross gsubs plus the existing single-glyph rules cover the complete glyph census of all four goldens (`─ │ ┌ ┐ └ ┘ — ← ↑ → ↓ ▲` present; `┼├┤┬┴ ▼` mapped-but-unused). No unmapped non-ASCII glyph remains.
- Whole pipeline under `LC_ALL=C awk` over every golden: output is **byte-identical** to the UTF-8 gawk render for all four files; zero high bytes leak (`grep -cP '[\x80-\xFF]'` = 0 everywhere); every row's cell width preserved (same stripped-length check as above — no `+++` widening); concatenated C-mode roff still carries `\m[` (guard satisfied) and `groff -k -man -T pdf -dpaper=letterl -P-pletterl -rLL=10i -rPO=0.4i` emits `%PDF` with an empty stderr.
- Lead-byte-ordering analysis: every pattern is a complete 3-byte UTF-8 sequence and every replacement is ASCII, so no gsub can create or expose a partial sequence for a later gsub; and in valid UTF-8 a 3-byte pattern can only match its own aligned glyph (0xE2 is lead-only, continuation bytes are 0x80–0xBF, ASCII is <0x80) — no cross-boundary or shared-lead false match is constructible in any order. The byte-identity result confirms this empirically.

**M-1 (unbraced test-mod sticks the scan) — correctly folded in both copies, mutation-proven, sound.**
- Both `production_now_utc_lines` copies (`/scratch/code/bitcoin_tax/crates/btctax-tui/src/export.rs:1010`, `/scratch/code/bitcoin_tax/crates/btctax-tui-edit/src/main.rs:14103`) now peek the next line and start a skip only when it is NOT an unbraced `mod `/`pub mod `/`use `/`pub use ` decl ending in `;`. Both `now_utc_scan_does_not_stick_on_an_unbraced_test_mod` KATs pass, alongside the r1 de-stick KATs and both real scans (6/6 green).
- **Mutation:** replaced the guard in the export.rs copy with bare `in_test = true;` → the KAT **REDS** (0 passed / 1 failed). Restored from cp-backup; tree clean.
- **Peek soundness, audited against every `#[cfg(test)]` in both scanned trees:** the only unbraced decl is `tabs/mod.rs:14–15` (`mod tests;` — matches the guard, no skip, correct); every other top-level `#[cfg(test)]` is followed by `mod X {` / `mod sort_tests {`, which ends in `{` not `;` → classified braced → skip starts (correct). A braced open line cannot be misread as unbraced (needs a trailing `;`), and an unbraced decl cannot be misread as braced. The in-fixture string literals (`"#[cfg(test)]",`) start with `"` after trim and sit inside the braced `mod tests` span — inert.
- **De-stuck-scan safety:** `tabs/tests.rs` (the separate-file body of the unbraced decl) is now effectively scanned as production — I grepped it: **zero `now_utc(` occurrences**, so no false positive; and that failure direction is loud (test fails), never a silent miss. Both real scans pass; `make check` confirms.

**N-1 (doc) — folded and accurate.** The header now says a modifier CONTAINING BOLD → `\f[CB]`, standalone UNDERLINED/REVERSED → plain, bg dropped — exactly what `bold = (mod ~ /BOLD/)` and the ignored `bg=` do. The "none in the current goldens" claim verified: the complete modifier census is `BOLD`, `BOLD|REVERSED`, `BOLD|UNDERLINED` — no standalone UNDERLINED/REVERSED run exists. (No ratatui modifier name contains "BOLD" as a substring of another, so the regex can't false-positive.)

## PDF page-fit verification

**Landscape geometry — correct, unrotated, unclipped.**
- Both recipes use `-dpaper=letterl -P-pletterl -rLL=10i -rPO=0.4i` (no `-P-l`). `pdfinfo`: both PDFs are **792 × 612 pts, Page rot: 0**. TUI PDF is **4 pages** — one 40-row screen per page at `.ps 9`/`.vs 10.5p` (40 rows ≈ 420pt < 612pt; 120 CR cols at 9pt = 648pt < 720pt LL); examples PDF is 20 pages.
- Rasterized **every page of both PDFs** (`pdftoppm -r 100`, 1100×850 px) and measured the ink bounding box: max extent across all 24 pages is 1012×752 at offset +40+49 → right edge ≤ 1052/1100, bottom edge ≤ 801/850 — **every page fits both dimensions** with ≥ 0.45in of clear margin. Visual spot-checks (browse page, modal page, examples p.9) show the red/cyan/yellow colorization and intact grids.

**man-wrap.awk wrapping — correct, non-mangling, escape-safe.**
- `emit_pre` only touches lines > WRAP(118); breaks at the last space in (40,118] with a 4-space hanging indent; hard-breaks only an unbroken token > that span (no such token exists in the golden — longest is ~29 chars). Loop provably terminates (length strictly decreases ≥37/iteration).
- Escape interaction: `\`→`\e`, `⚠`→`(!)`, and the `\&` prefix are all applied BEFORE `emit_pre`, so WRAP applies to the final emitted text; `\e`/`\&` overcount printed width (wraps earlier — safe direction), `(!)` is exact. Console lines in the golden contain **zero** backslashes, so no `\e` can be split today (latent case recorded as N-r2-1).
- Output audit: the only >118-char output line (line 606) is fill-mode body (`nf=0`) that groff refills; **zero `.nf` lines exceed 118**. The 43 over-WRAP console lines are all single-line advisories/notes/commands — I checked every consecutive over-118 run: none is a multi-row aligned data table, so no table alignment is mangled (all aligned tables are ≤118 and pass through untouched).
- Byte-mode (`LC_ALL=C`) man-wrap run: wraps a few characters earlier on multibyte-bearing lines (byte length ≥ char count — safe direction), always at spaces (a continuation byte 0x80–0xBF can never equal 0x20, so a glyph can't be split at a space break), output is **valid UTF-8** (`iconv` clean), zero over-WRAP `.nf` lines, and groff emits `%PDF`. This matters because CI runs **both** `make examples` (ci.yml:128) and `make examples-tui` (ci.yml:133) on ubuntu/mawk — both hold.
- Byte-gated golden untouched: `git diff 34bf318..HEAD -- docs/examples/examples.md docs/examples-tui/*.txt` is **empty**; the fold's diff touches only the two awks, the Makefile, the two Rust test files, and the persisted review.

**Colorization guard — intact.** The `grep -qF '\m['` runs before groff in the recipe; negative-tested (an escape-less roff fails it, exit 1); the `.ps 9`/`.vs 10.5p` additions are roff requests that don't intersect the guard or the escapes. Colored-ink confirmation in the final PDF: 290/2649/290/305 chromatic pixels on TUI pages 1–4.

**Suite / claims.** `make check` exit 0 — **2067/2067 passed, 8 skipped** (2065 + the two new KATs; count consistent with r1). `cargo fmt --check` clean. `git status` clean.

## CRITICAL

None.

## IMPORTANT

None. Both r1 Importants are folded exactly as specified and hold under adversarial re-execution, including full byte-mode pipeline reproduction; the page-fit change is geometrically verified on every page of both PDFs.

## MINOR

None new. (r1's M-2 — the string-blind `//`-strip — remains recorded, pre-existing and unchanged, as r1 classified it.)

## NIT

**N-r2-1 (latent, record-only)** — `man-wrap.awk` `emit_pre`'s hard-break path could split a `\e` escape pair if a future golden ever contains a >77-char unbroken console token with a backslash, leaving a bare trailing `\` (a roff line-continuation) that would merge two rendered lines. Zero backslashes exist in any console line of the current golden, and space-breaks can never split `\e`. One-line hardening if it ever arises (step `brk` back off a trailing `\`).

**N-r2-2 (latent, record-only)** — the `unbraced_decl` peek does not recognize visibility-qualified unbraced decls (`pub(crate) mod x;`, `pub(super) use …;`); such a `#[cfg(test)]` decl would re-open the M-1 stick (silent false negative). No such geometry exists in either scanned tree (audited every `#[cfg(test)]` occurrence), and test modules are conventionally private; extend the prefix list if one ever appears.

## VERDICT

GREEN — 0 Critical / 0 Important
