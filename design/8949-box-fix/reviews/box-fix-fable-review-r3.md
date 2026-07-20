# Round-3 re-review — year-aware Form 8949 box fix

Reviewer: independent Fable (did not author). Round: r3 (re-review after the r2 fold).

## R2 finding dispositions

**NEW-IMPORTANT-1 (year-blind [I5] export advisory) — RESOLVED.**
- `crates/btctax-cli/src/cmd/admin.rs:212-227`: `broker_reporting_advisory(tax_year, rows)` is pure, `None` on zero rows, and year-gated on the same `DIGITAL_ASSET_8949_FIRST_YEAR` constant the core box assignment uses. Both eras are tax-correct: pre-2025 → 1099-B / "Box A/B (ST) / D/E (LT)" separate / "files EVERY Bitcoin row under Box C/F" (matching what the 2024/2017 maps actually check, read-back-pinned by sp3/sp3b); TY2025+ → 1099-DA / Box G/H/J/K / Box I/L.
- **No behavior change for a 2025 export**: the assembled 2025 string is byte-identical to the old inline literal — confirmed against the test-enforced golden `docs/examples/examples.md:129`, which still matches (suite green).
- **Pinned**: the two era tests each assert presence of their own era's strings AND absence of the other era's ("1099-B" is not a substring of "1099-DA" and vice versa — checked). I mutation-tested the gate (`>=` → `>`): `broker_advisory_ty2025_cites_1099da_and_digital_asset_boxes` fails; the mirror revert fails the pre-2025 test by construction. Tree restored, clean.
- **Call site** (`crates/btctax-cli/src/main.rs:792-799`): passes `report.tax_year` (set from the fn's `tax_year` param, the same value that feeds `form_8949(&state, tax_year)` at admin.rs:287, so year and rows can't diverge) and `report.broker_reported_rows`. No 2025-only or pre-2025-only string leaks: the repo-wide grep finds no other [I5] emission. Residual note, not filed (same class r2 declined): a contrived mutant hardcoding `2025` *at the call site* survives — the 2025 wiring is end-to-end-pinned by the examples golden, the pre-2025 wiring only by the unit tests plus reading.

**NEW-MINOR-1 (doc residue) — RESOLVED.** All five cited sites verified accurate for both eras in current source, no new misstatement: `crates/btctax-tui/src/tabs/forms.rs:30-32` (fn doc now four-letter, year-aware), `crates/btctax-cli/tests/export.rs:119-122` (header now says box I, matching the assertion), `crates/btctax-core/tests/kat_forms.rs:77-80` (2025-scoped, correct i8949 quotes), `crates/btctax-core/src/tax/printed.rs:785-791` (era-qualified Schedule D citation), `crates/btctax-forms/src/fill8949_full.rs:3-5` (same). The two r2-named precursors (`admin.rs:155-160` report doc, `btctax-forms/src/lib.rs:354-358`) are also correctly rewritten.

## The completeness question

**The sweep is NOT yet exhausted. One more year-blind emitter exists — and it is user-facing.** See NEW-IMPORTANT-1 (r3) below. The r1→r2 pattern repeats a third time: each fold fixed the found emitter and swept the sites the reviewer named, but did not independently re-sweep the whole surface.

Surfaces checked (the audit trail for this claim):

1. **Every Rust string/doc in `crates/` + `xtask/`** — pattern grep for `Box A–L`, lowercase `box <letter>`, `C/F`, `I/L`, `G/H/J/K`, `A/B/D/E`, plural "boxes", "box letter", `1099-B`, `1099-DA`; all 134 hits individually audited. The only letter-emission paths remain the two exhaustive `form8949_box_tag` twins (`btctax-cli/src/render.rs`, `btctax-tui/src/tabs/forms.rs` — both render the row's year-aware box), the year-gated TUI note, the year-gated [I5] advisory, and the map-driven PDF fill (per-year TOMLs — comments verified correct: 2017/2024 C/F on `/3`, 2025 I/L on `/6`).
2. **CLI stdout/stderr** — full read of main.rs export arm; no other subcommand prints a box letter.
3. **TUI + btctax-tui-edit** — only `tabs/forms.rs`; the editor renders through the same fn. F0/F1 pin the mapping.
4. **Man pages** — all 58 tracked `docs/man/*.1` grepped; only `btctax-export-irs-pdf.1` carries box text: line 11 (packet bullet) and line 27 (`--tax-year` arg) are year-correct both eras; **line 13 is the finding**.
5. **README.md** — 185-227 year-correct ("never the wrong pair"); line 229 is the Minor tail of the finding.
6. **docs/examples/examples.md** — [I5] appears once, on the TY2025 slice export (correct, golden-enforced). The TY2024 export shown (line 621) is the full-return path, which emits no [I5] at all — suppression, not misguidance.
7. **Walkthrough goldens** — j2 (2025 → I/L note) and j6 (2024 → C/F note) correct and golden-test-enforced; j1/j3-j5/j7-j9 have no forms pane; `docs/examples-tui` and `docs/architecture` have no box content.
8. **Generated artifacts** — `docs/pdf/` is **gitignored** (not a repo surface); `docs/man/` is tracked (finding above). Form map TOMLs per-year correct. No CHANGELOG, no per-crate READMEs.
9. **Error messages, `limitations`, help units, comments** — no box letters outside the audited sites. No serde/Debug leak path for `Form8949Box` was added by the fold.
10. **Internal artifacts** (FOLLOWUPS, design/) — not filer-facing; one stale citation noted as Nit.

After the finding below is folded, to my knowledge **no other year-blind box emitter remains** on any repo surface.

## New findings

### NEW-IMPORTANT-1 (r3) — the export-irs-pdf **help text / man page** [I5] description is still year-blind
`crates/btctax-cli/src/cli.rs:176-177` → generated into the **tracked** man page `docs/man/btctax-export-irs-pdf.1:13` (and the local PDF manual):

> "Rows on an exchange that MAY carry **1099-DA** broker reporting are flagged on stderr (btctax files them all under **Box I/L** and says so)."

Unconditional, in the long help of a command whose own doc block supports TY2017/TY2024. For those years: (1) no 1099-DA existed — and after the r2 fold the stderr flag it describes correctly cites the 1099-B, so the help now *misdescribes the very advisory r2 fixed*; (2) "files them all under Box I/L" is a false statement about the written PDFs — 2024/2017 file under C/F, and boxes I/L do not exist on those revisions (r1 verified from the bundled forms). The same doc block is year-aware 25 lines earlier (cli.rs:150-153: "Never the wrong pair for the year") and in the `--tax-year` arg doc (198-199), so `btctax help export-irs-pdf` currently contradicts itself. This sentence was outside the ranges r2 audited (152-160, 198-201) — an r2 sweep miss, gated regardless of provenance per the whole-surface rule both prior rounds applied. Same severity class as r1-I1/r2-NI1: era-wrong box guidance on a user-facing surface.
**Fix:** make the sentence era-spanning in cli.rs (e.g. "Rows on an exchange that MAY carry broker reporting (a 1099-DA from TY2025; a 1099-B before) are flagged on stderr — btctax files every row under the year's not-reported box (I/L from 2025, C/F before) and says so."), then regenerate `docs/man` (single-source: clap doc-comments → --help + man).

### NEW-MINOR-1 (r3) — year-blind internal-doc residue the sweeps have not yet reached (aggregate)
- `crates/btctax-forms/src/lib.rs:77-78` — `fill_form_8949` public API doc: "Bitcoin is filed under Box I/L" — the fn takes `year` and files C/F for 2024/2017; also "> 11 rows paginate" (rows_per_page is 14 on 2024/2017, map data). Published-crate docs.rs surface.
- `crates/btctax-forms/src/map.rs:104` — `PartMap::box_field` doc: "The digital-asset box checkbox field — Box I (ST) / Box L (LT), NOT C/F" — the same struct carries the 2024/2017 maps whose `box_field` IS the C/F checkbox (map.rs:155 states it correctly).
- `crates/btctax-forms/src/schedule_d.rs:8-9` — module doc quotes the 2025-only Schedule D line text unconditionally; the module fills all three revisions.
- `crates/btctax-core/src/tax/printed.rs:1892-1893` — test doc quotes "with Box C checked" era-unconditionally (the class the r1-MINOR-2 sweep qualified elsewhere).
- `README.md:229` — "may carry 1099-DA broker reporting are flagged on stderr" — era-blind 1099-DA attribution (no box claim; fix together with the Important).

### NIT-1 (r3) — `FOLLOWUPS.md:52` still says "enum is `{C,F}` only (`forms.rs`)"; it is `{C,F,I,L}` now, and the future 1099-reconciliation feature it files must be year-aware (A/B/D/E vs G/H/J/K).

### Observations (non-gating)
- The full-return path hardcodes `broker_reported_rows: 0` (`admin.rs:589`), so a full-return export never emits [I5] even with exchange disposals. Pre-existing, era-independent suppression (emits nothing, misguides no era) — outside this fix's scope, but worth a follow-up entry: the omission is undocumented at the construction site.
- The **local, gitignored** `docs/pdf/btctax-tui-walkthrough.pdf` on this machine still embeds the pre-r1 note ("Review box C/F … A/B/D/E (1099-B/1099-DA)") on the 2025 J2 pane — stale build output, not a repo defect. Re-run `make tui-walkthrough` (and the manual PDFs after fixing cli.rs) before any release bundling.

## Verdict

Both r2 findings are genuinely resolved — the advisory seam is well-built, era-correct, byte-stable for 2025, and mutation-pinned — and the fold introduced no new defect. But the whole-surface sweep this fix's scope demands turns up one more year-blind, user-facing emitter: the export command's own help/man text, which now misdescribes the advisory the r2 fold just fixed.

**NOT-GREEN — 0 Critical / 1 Important** (cli.rs:176-177 / man page), plus 1 Minor aggregate and 1 Nit. Suite green at 2088; the Important is the gate.
