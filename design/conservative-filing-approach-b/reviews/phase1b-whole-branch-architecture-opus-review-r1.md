# Phase-1b whole-branch review — SOFTWARE ARCHITECTURE lens (Opus, r1)

**Scope:** the Phase-1b delta `88a0980..6505c18` (official Form 8275 fillable PDF in
`btctax-forms` + wiring into `export-irs-pdf` / full-return packet / DRAFT gate; BG-D8 gate
re-pointed at the PDF; `promote` verb un-hidden; TUI export surface gated). Reviewed the diff,
opened current source under `crates/`, and exercised the delta against the test suite.

## Verdict

**GREEN** — **0 Critical / 0 Important / 2 Minor / 1 Nit**

---

## Critical

None.

## Important

None.

## The eight rubric concerns — findings

### 1. ★ `encode_pdf_text` change to the shared `pdf.rs::apply_writes` — SOUND

- **ASCII path is byte-identical.** Merge-base `apply_writes` wrote
  `Object::String(s.clone().into_bytes(), Literal)`. New code writes
  `Object::String(encode_pdf_text(s), Literal)` where `encode_pdf_text` returns
  `s.as_bytes().to_vec()` for `s.is_ascii()` (`pdf.rs:401-412`). For any ASCII string these are
  the identical byte vector, so no existing form's `/V` bytes move and no golden SHA shifts.
- **The "first non-ASCII fill string" claim holds.** Grepped every `btctax-forms` fixture/fill
  input: all non-ASCII bytes are in *comments* (em-dashes, arrows, ★), never a `Text` field
  value. `full_return_form_fills_are_byte_deterministic` + the SE/1040/8283/8949 golden-SHA KATs
  all PASS post-change — proof the ASCII path preserves the existing goldens exactly.
- **Non-ASCII path is valid PDF and round-trips.** UTF-16BE + `FEFF` BOM is Adobe's Unicode
  string convention and exactly what `decode_pdf_text` (`pdf.rs:311`) reverses. lopdf's literal
  writer escapes `(`/`)`/`\` for arbitrary bytes (pre-existing, exercised by every text fill).
  `form_8275_fills_part_i_part_ii_and_identity` round-trips the em-dash **and** balanced parens
  ("Part I — column (e)") encode→save→reload→decode and PASSES; `form_8275_is_byte_deterministic`
  pins a golden SHA. The `/MaxLen` leg counts **characters** (`verify.rs:410` `v.chars().count()`),
  so the 2-byte-per-char UTF-16 encoding does not falsely trip the cell-overflow guard.

### 2. `fill_form_8275_inner(filer: Option<&ReturnHeader>)` refactor — behavior-preserving
Mirrors `form8283::fill_form_8283_inner` exactly: full-return caller passes `Some(header)`
(identity written); crypto-slice `fill_form_8275_slice` passes `None` (identity skipped, as the
slice 8283 does). `_with_map` (testonly) keeps identity required. Clean.

### 3. `assemble_printed_return`/`assemble_printed_forms(&events)` signature change — all callers fixed
Every call site updated with the correct event set: `cmd/tax.rs` report (`&events`),
`export_full_return` (`events`), packet.rs internal, and the no-promote callers pass `&[]`
(golden_returns, oracle-harness, common/mod, full_return_forms) — legitimate, since those corpora
contain no promotes so `disclosure_8275 → None`. Compiles; the packet KATs PASS.

### 4. ★ BG-D8 gate architecture — refuse-before-bytes preserved at BOTH seams; e10 unweakened
- `promote_export_gate` (`admin.rs:78`) is a **pure** `(&LedgerState, &[LedgerEvent], Option<i32>)
  -> Result` predicate — no `Session`, lock, or I/O — re-deriving `disclosure_8275` and refusing on
  `disc.incomplete` (= `part_ii.trim().is_empty()`, catching whitespace-only, `form8275.rs:163`).
- Called **FIRST**, before any `mkdir_out`/byte, in all four export fns: `export_snapshot` (142 →
  mkdir-in-write_csv 187), `export_irs_pdf` (385 → mkdir 429), `export_full_return` (651 → mkdir
  725), TUI `do_export` (169 → mkdir 171). KATs assert `out_dir` untouched on refusal at every
  seam and PASS.
- TUI reaches the gate via the crate-**root** re-export `btctax_cli::promote_export_gate`
  (`lib.rs`), never `btctax_cli::cmd::…`. `e10`'s `everywhere_tokens` (incl. `cmd::`) and
  `write_class_tokens` are **byte-identical** to merge-base (diffed) — the token list was not
  weakened. `e10_mechanized_source_gate` PASSES.
- BG-D8 completeness holds even under `--forms`: `write_form_8275_txt` is emitted
  **unconditionally** post-mkdir on every export path (`admin.rs:437`/`730`, self-gating on
  `disclosure_8275`); only the official *PDF* is `--forms`-gated — mirroring `basis_methodology.txt`.

### 5. Census 14→15 compile-forced hook — intact; J6 wired
`packet.rs` destructures `PrintedForms` with the explicit `// ★ NO ..` comment and both `f8283` +
`f8275` arms — a new member without a filler is a compile error. `CENSUS_KEYS` bumped to 15,
Attachment Seq 92 asserted. `journey_j6` now declares → promotes (threading the printed
`declare-tranche` ref via `emit`'s new stdout return) → sells a tranche, and
`every_census_form_demonstrated_in_j6` requires all 15 keys in the J6 manifest. All PASS.

### 6. Year-alias map — sound, no dead branches
`f8275_pdf`/`Form8275Map::for_year` alias the single Rev. 10-2024 asset to `{2017,2024,2025}` and
re-stamp only the `year` tag (never written to the PDF). `form_8275_fills_for_every_supported_non_2024_year`
asserts byte-identity across all three years; `unsupported_year_rejected_for_form_8275` +
`map_year_matches_bundled_pdf_fieldset_for_every_supported_year` PASS.

### 7. SSOT / dead code — clean; the duplications are pinned (see Minors)
`write_form_8275_txt` → `write_form_8275_txt_named("form_8275.txt")` is a clean single-source
split; the all-years path uses `form_8275_{year}.txt` (no bare-name collision). Overflow capacity
is single-sourced from `Form8275Map::for_year().rows.len()` everywhere.

### 8. Test architecture — pins each seam, kills each named mutation
sp4 (fill / fault-injection / per-field sentinel-no-swap / byte-determinism / year-alias /
unsupported), census (15 / J6 / iff both directions), promote_cli (2025+2017 e2e / gate-absent /
overflow crypto-slice **and** full-return twins / M2 multi-year), TUI e12/e13. The crypto-slice
overflow twin explicitly documents that deleting only its block survives the full-return sibling
but reds itself — the two pre-checks are independently pinned. All 78 exercised KATs PASS.

---

## Minor (non-gating; recorded)

- **M-1 — duplicated overflow-refusal message literal.** The identical multi-line refusal string
  is written twice: the crypto-slice pre-check (`admin.rs` ~438-448) and the full-return pre-check
  (`admin.rs` ~344-354). The *cap* is single-sourced (`Form8275Map::rows.len()`) and both blocks
  are mutation-pinned, so only the human-readable copy can drift; both KATs assert the load-bearing
  substrings (`"void one of the promotes"`, the year, the leg count), so a meaningful drift reds.
  Consider hoisting the message to one `fn`. *Owning phase: Phase-1b residue / FOLLOWUPS.*
- **M-2 — duplicated promoted-year collection.** `promote_export_gate`'s `None` branch
  (`admin.rs:88-97`) and `export_snapshot`'s `None` branch (`admin.rs:163-172`) each independently
  iterate `state.disposals` × `promoted_origins` to build the promoted-year set. Same shape, could
  drift; the M2 KAT pins the writer set, the gate is a separate concern. Consider a shared helper.
  *Owning phase: Phase-1b residue / FOLLOWUPS.*

## Nit

- Branch HEAD at review time is `9c6e49d` (`docs(followups)…`), one commit past the reviewed range
  `6505c18`. Docs-only, outside the code scope — noted for completeness, not gating.

---

## Validation exercised
`cargo nextest -p btctax-forms --test sp4 --test census --test full_return_forms` → 62/62 PASS.
`cargo nextest -p btctax-cli --test promote_cli` (8275/gate/overflow/M2 filter) → 13/13 PASS.
`cargo nextest -p btctax-tui` (e12/e13/e10) → 3/3 PASS. No red in the delta surface.
