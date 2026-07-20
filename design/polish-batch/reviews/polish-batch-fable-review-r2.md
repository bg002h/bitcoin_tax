# Polish batch — independent review r2 — GREEN (0C/0I)

_Fable, independent. The two r1 Importants (ProRata note unrenderable; P3-e wiring untested) + Minor-1 +
3 Nits all resolved and mutation-held. Two new non-gating Nits (FOLLOWUPS wording; residual ≤63-col clip)._

---

VERDICT: GREEN

Suite verified fresh: `make check` exit 0 (2082/2082 nextest + clippy `-D warnings`), `cargo fmt --check` clean. Working tree matches the reviewed diff except the author's FOLLOWUPS.md log entry — same delta r1 tolerated. All mutation experiments used cp-backup/restore (never `git checkout`); `cmp` confirmed byte-identical restoration after each, and `make check` re-ran green on the restored tree.

**Important-1: RESOLVED** — `crates/btctax-tui-edit/src/draw_edit.rs:4153-4197` (fix), `:6099-6128` (KAT).
- Geometry verified: on 80×24, `centered_rect(96,24)` (draw_edit.rs:2676-2685, `.min(area.width)`) caps the modal to 80×24 → inner 78×22; `Length(6)`+`Min(0)`+`Length(4)` = 10 fixed rows ≤ 22, so the banner gets its full 6 rows. The 5 banner lines measure 31/32/60/62/63 chars — all ≤ 78, none wraps, none clips.
- Mutants run: (a) `Length(6)`→`Length(3)` → KAT FAILED ("attest it yourself" clipped) — killed. (b) Full r1-defect state (Length(3) + rejoined single long line + `.wrap()` removed) → KAT FAILED — killed. (c) Characterization: rejoined single line with Length(6)+wrap KEPT → KAT passed — correctly, since wrap renders the joined line in full on 3 of the 6 rows; the pinned property (note renders in full) genuinely holds there, so this is not a survivorship gap.
- Narrower terminals, probed empirically by re-running the KAT at widths 66/64/63/62: full clean render down to 66; at 64 the content still renders completely but "attest it yourself" word-wraps across two rows (my `contains` probe fails only because the phrase spans a row break); real clipping resumes at ≤63 cols (7 wrapped rows > 6). Filed as a Nit below — the modal's residue table alone needs ~84+ cols, the TUI has no min-size guard anywhere, and the pre-fix state clipped at ALL widths including 80.
- Wording still accurate to the engine: lots are computed ONCE via `Session::safe_harbor_residue` under `pre2025_method`; the toggle changes only the recorded tag (G3/G5 — form.rs:2054-2062); `AllocMethod::ProRata`'s only production semantics is the timebar/inert rule (`crates/btctax-core/src/project/resolve.rs:1216,1221`); no auto-pro-rata lot-splitting path exists anywhere (grepped all non-test uses).

**Important-2: RESOLVED** — `crates/btctax-tui-edit/src/edit/tax_inputs.rs:184` (wiring), `:1134-1156` (KAT).
- Mutant run: reverted line 184 `seed`→`set` → `begin_edit_seeds_a_long_stored_text_without_truncating_it` FAILED (buffer truncated to exactly 64 chars, "…Consultant, Cons") — the wiring is now mutation-held, closing the exact gap r1 found (where 352/352 stayed green).
- The test is meaningful end-to-end: `TpOccupation` is a live Text field (`live: |_| true`, sections.rs:198-210); `tax_inputs_apply_edit` routes through `parse` — Text parse is unbounded (`crates/btctax-input-form/src/parse.rs:24`, `raw.to_string()`) — bypassing the capped keystroke path, so the 96-char value genuinely reaches `ri.header.taxpayer.occupation`; `seed_string` (tax_inputs.rs:583-593) returns the full stored Text. The assertion compares against the real stored 96-char value.

**Minor-1: RESOLVED** — `crates/btctax-cli/src/input_form_store.rs:270-286` (`mutate_and_save` + its own doc, placed after `CommitOutcome`); commit's contract doc (NoTables/Refused/all-or-nothing, updated to reference `mutate_and_save`) now immediately precedes `pub fn commit` (~:288-317). No franken-doc.

**Nits: all RESOLVED.**
- Nit-1: seam.rs comment now says "SetField + ClearField" and explicitly notes the four structural variants are not re-exercised — accurate against the 6-variant `Edit` enum (seam.rs:217-241).
- Nit-2: `seed` doc (form.rs:80-83) documents the deliberate per-call cap reset and the FREETEXT_CAP shrink hazard; logic unchanged (reset kept — the no-inheritance property r1 blessed). Claim "no larger-cap caller today" verified: the only production `seed` caller is tax_inputs.rs:184 on `form.buf`, and all 4 `TaxInputsFormState` constructions use `FieldBuffer::new()` (FIELD_CAP); the `with_cap(FREETEXT_CAP)` buffers (optimize-accept attest text, donation free-text in main.rs) never call `seed`.
- Nit-3: `discard_parked_draft` doc (input_form_store.rs:409-412) now correctly says restore on ANY failure (delete OR save) via `mutate_and_save`.

**No regressions** — Banner growth (3→6 rows) leaves the `Min(0)` residue table 12 rows on a 24-row terminal; it is stateful/scrollable, and all pre-existing modal render tests pass within the 2082. No borrow/format fallout (clippy `-D warnings` + fmt clean). The three new tests pass in baseline.

## New findings

**Nit-A (FOLLOWUPS.md wording)** — the polish-batch entry still says Task-2(d) covers "both `Edit` variants" — the same overclaim Nit-1 fixed in the code comment (`Edit` has six variants; the entry means the two field-edit variants). Log-only, outside the reviewed diff file; fix opportunistically.

**Nit-B (residual narrow-terminal clip)** — draw_edit.rs:4156. At terminal widths ≤63 cols the five banner lines wrap to ≥7 rows and the `Length(6)` chunk clips the tail of the note again (verified empirically at 63/62). Degenerate width for this modal (its residue table needs ~84+ cols) and strictly better than the pre-fix state (which clipped at all widths); worth revisiting only if a TUI min-size guard is ever introduced. No owning phase — ownerless residue.

No Critical, no Important. Both r1 Importants are now genuinely mutation-held (Length(3) mutant killed; r1-state mutant killed; `seed`→`set` mutant killed). GREEN.
