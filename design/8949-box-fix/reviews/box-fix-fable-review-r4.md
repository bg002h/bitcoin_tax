# Round-4 re-review — year-aware Form 8949 box fix (convergence check)

Reviewer: independent Fable (did not author). Round: r4 (re-review after the r3 fold).

## R3 finding dispositions

**NEW-IMPORTANT-1 (r3) — year-blind export-irs-pdf help/man sentence — RESOLVED.**
- `crates/btctax-cli/src/cli.rs:176-178`: the sentence is now era-spanning — "Rows on an exchange that MAY carry broker reporting (a 1099-DA from TY2025; a 1099-B before) are flagged on stderr — btctax files every row under the year's not-reported box (I/L from TY2025, C/F before) and says so." Tax-correct for all three shipped years: the 1099-DA attribution starts at TY2025 (confirmed against `legal/text/irs-guidance/Notice_2026-20.txt:253` — gross-proceeds reporting begins 2025); "C/F before" matches what the 2024/2017 maps actually check (read-back-pinned by sp3/sp3b); "MAY carry a 1099-B" is a correct possibility statement pre-2025 and Boxes A/B/D/E exist on both pre-2025 revisions. It also now correctly describes the r2-fixed stderr advisory in both eras.
- **Internally consistent**: the same help block's other two box mentions (cli.rs:150-154 "Never the wrong pair for the year"; the `--tax-year` arg doc at cli.rs:200-202 "Bitcoin is filed under Box C/F (not Box I/L)") agree with the new sentence. No contradiction remains anywhere in `btctax help export-irs-pdf`.
- **Man page verified by regeneration, not by reading**: I re-ran the generator (`cargo run -p xtask -- docs`) and `git status docs/man/` came back empty — the committed `docs/man/btctax-export-irs-pdf.1:13` is byte-identical to a fresh regeneration from current cli.rs, and no *other* man page drifted. The single-source pipeline is intact.
- No new misstatement in the sentence.

**NEW-MINOR-1 (r3) — doc residue — RESOLVED, all five sites, no new error.**
- `crates/btctax-forms/src/lib.rs:77-81` (`fill_form_8949`): now era-aware, and the new pagination claim "11 rows per part on the 2025 form, 14 on 2024/2017" is **verified against the maps** (`rows_per_page = 11` in `forms/2025/f8949.map.toml:3`, `14` in 2024/2017).
- `crates/btctax-forms/src/map.rs:104-106` (`box_field`): now correctly says which box the field is depends on the loaded year's map.
- `crates/btctax-forms/src/schedule_d.rs:8-11`: era-qualified, and the added "fills all three revisions (2017/2024/2025)" is true.
- `crates/btctax-core/src/tax/printed.rs:1892-1894` (test doc): era-qualified, correct 2025 line text.
- `README.md:229`: era-spanning, matches the cli.rs sentence.

**NIT-1 (r3) — RESOLVED.** `FOLLOWUPS.md:52-55` now reads `Form8949Box = {C,F,I,L}` year-aware, and the future reported-box feature entry itself correctly states the year-aware reported pairs (A/B/D/E ↔ 1099-B pre-TY2025; G/H/J/K ↔ 1099-DA from TY2025).

**R3 observation (full-return [I5] suppression) — FILED.** `FOLLOWUPS.md:56-60`: accurate description (`admin.rs` hardcodes `broker_reported_rows: 0` on the full-return path), era-independent, with a stated owning phase ("a future export-parity pass"). The phase name is loose but the entry is greppable and correct; not gating.

## The completeness certification (independent sweep, enumerated)

I swept the entire repo myself — patterns `1099-DA`/`1099-B` (incl. roff-escaped `1099\-DA`/`1099\-B`), `Box [A-L]`, `box [a-l]`, `C/F`, `I/L`, `G/H`, `J/K`, `A/B/D/E` — and audited every hit for era-correctness:

1. **Core** (`btctax-core/src/forms.rs`) — the single year-aware predicate; enum {C,F,I,L}, no 1099-reported box constructible; docs era-correct.
2. **Letter emitters** — exactly two exhaustive `form8949_box_tag` twins (`btctax-cli/src/render.rs:185-192`, `btctax-tui/src/tabs/forms.rs:30-40`), both rendering the row's year-aware box; the map-driven PDF fill (per-year TOMLs, read-back-pinned). No serde/Debug leak path.
3. **CLI stderr [I5]** — pure `broker_reporting_advisory` (`admin.rs:222-238`), year-gated on the shared constant; both eras unit-pinned; the 2025 string byte-matches the golden-enforced `docs/examples/examples.md:129`.
4. **CLI help / man** — cli.rs export block: three box mentions, all mutually consistent and era-correct. All 58 `docs/man/*.1` grepped (both escaped and unescaped forms): only `btctax-export-irs-pdf.1` carries box/1099-broker text, all of it era-correct; committed pages proven identical to regeneration.
5. **TUI + tui-edit** — the year-gated footnote (`forms.rs:175-186`) mirrors the core constant; tui-edit renders through the same fn; F0/F1 non-vacuous. No box text in the help overlay or any other tab (crate-wide grep: zero hits outside `tabs/forms.rs` + `tabs/tests.rs`).
6. **Walkthrough goldens** — j2 ("Forms — 2025" → I/L note) and j6 ("Form 8949 — 2024" → C/F note, 1099-B only), both test-enforced; no other journey renders a forms pane.
7. **README.md** — lines 187-199 (per-year Box I/L vs C/F, 11 vs 14 rows) and 229, all era-correct.
8. **docs/examples/examples.md** — [I5] appears once, on the TY2025 export; the TY2024 full-return block (line ~632) emits no [I5] (the filed suppression follow-up, not misguidance). W-2/1099-DIV "box" mentions unrelated. `docs/architecture/`, `docs/examples-tui/`: zero hits.
9. **Per-year TOMLs** — 2024/2017 f8949 map comments (Box C/F, on-state /3), 2025 (I/L, /6), 2017 schedule_d map ("Box C totals on LINE 3") — all era-correct for their own revision.
10. **Test/fixture comments** — `kats.rs` (2025→I/L), `sp3.rs`/`sp3b.rs` (2024/2017→C/F, A/B off, plus the 2025 I/L regression), `export_irs_pdf.rs` (per-year correct), `kat_forms.rs` (2025/2024/2026 KATs), `common/mod.rs` (inert-fixture comment, accurate) — all era-scoped correctly.
11. **Remaining Rust** — `return_1040.rs:151,161` ("Box C or **Box I**" / "Box F or **Box L**" — era-spanning, accurate); `optimize.rs:14` (1099-DA box 1i wash-sale statement — factual, about the form itself); `pdf.rs:225` ("/6 for Box I" — factual example); `xtask/src/examples.rs:704` (1099-DA caveat comment attached to a `--tax-year 2025` invocation). `btctax-store`/`btctax-adapters`/`btctax-tui-edit`: zero hits.
12. **legal/** — verbatim primary-source IRS text (and `Form_8949.txt:26` re-confirms the 2025 Box C "other than digital asset transactions" language). Internal artifacts (reviews/, design/, CONTINUITY, FOLLOWUPS) — not filer-facing; FOLLOWUPS now accurate anyway.
13. **docs/pdf/** — gitignored build output, not a repo surface. (Release-time reminder from r3 stands: rebuild the PDF manuals/walkthrough before any bundling; the committed man pages themselves are current.)

**Certification: no year-blind or era-wrong 8949-box or 1099-broker-form guidance remains on any repo surface.** The r1→r3 pattern (one more emitter each round) terminates: this round's independent sweep found zero.

## New findings from the r3 fold

None. The fold is docs-plus-regeneration only (no behavior change; suite green at 2088), the regenerated man page is provably in sync with its source, and every new factual claim it introduced (year boundaries, box pairs, grid sizes, revision line texts) checks out against the maps, the read-back tests, and the bundled IRS primary sources.

## Verdict

**GREEN — 0 Critical / 0 Important** (0 Minor, 0 Nit). The year-aware box fix has converged: core predicate, both letter emitters, the stderr advisory, the help/man surface, the TUI note, goldens, and every documentation surface are era-correct and pinned.
