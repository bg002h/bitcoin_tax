# Approach-B sub-project-1 — FOLLOWUPS

Follow-ups for the basis-floor / PromoteTranche feature (`feat/conservative-filing-b`).
**Blocking findings (Critical/Important) are NEVER filed here — they are fixed before their gate
closes.** Each entry records its **owning phase** so reconciliation is a grep (STANDARD_WORKFLOW §
in-phase burndown). Only ownerless residue batches to the end.

## Phase-1b export-surface work — ✅ ALL FOLDED (kept for the audit trail)

Phase 1b was decomposed: **T16** = wire the 8275 PDF into the CLI export/full-return + un-hide + the
overflow-graceful consumer; **T17** = the HARD TUI-export gate. The whole-branch M1/M2 + the oracle KAT
were folded by the controller. **STATUS (all done, full CI green @ `6505c18`):** TUI-export HARD gate →
T17 (`fe8e9d4`); M2 all-years CSV co-emit → T16 (`e7d63b9`); overflow-graceful → T16 (`a3d3392`); M1
8275 no-loss suffix → `0e7a59a` (both emitters, guard KAT mutation-verified); sentinel oracle KAT →
`6505c18` (mutation-verified vs a map swap); provenance → resolved. Nothing open below.

- **[★ HARD BG-D8 requirement — owner T17 — NOT a deferrable cleanup] The TUI export path
  (`btctax_tui::export::do_export` → `write_form_csvs`) must come behind the BG-D8 gate.** (Raised T14;
  re-tagged HARD per the T14 Opus review + reaffirmed by the whole-branch arch review.) Task 14 put
  `promote_export_gate` + the `form_8275.txt` emit on the THREE CLI export fns (`export_snapshot`,
  `export_irs_pdf`, `export_full_return`) at the CLI layer, NOT in the shared `render.rs` writers. On
  the Phase-1a branch the TUI CANNOT bypass the gate — `btctax-tui` is source-gated against
  `export_snapshot`/`write_csv_exports` (the `e10_mechanized_source_gate` test), so there is no second
  export surface yet, and `promote` is CLI-only / unreleased. **★ In Phase 1b this becomes HARD: once
  `promote` reaches the released/TUI surface, a TUI-exported promoted packet would (a) never refuse an
  incomplete 8275 and (b) omit `form_8275.txt`/8275 PDF even for a COMPLETE promoted leg — a direct BG-D8
  (Reg §1.6662-4(f) inadequate-disclosure) violation. T17 MUST gate `write_form_csvs` (or route the TUI
  export through the gated CLI fns) AND carry its own refuse + emit KATs on the TUI surface; 1b cannot
  ship without it.**

- **[Minor → fold in T16] All-years CSV snapshot omits the `form_8275.txt` artifact** (whole-branch tax
  review M2; `cmd/admin.rs`, `export_snapshot`). `write_form_8275_txt` is written only under
  `if let Some(y) = tax_year`; the `tax_year: None` all-years dump exports the promoted disposal rows but
  no 8275 file rides alongside. The completeness GATE still fires for `None` (an incomplete Part II
  refuses even in the all-years path), so **no inadequately-disclosed position escapes** — this is a
  letter-of-BG-D8 surface-coverage gap, not a filed-number defect (low impact: a raw projection dump, not
  a per-year filing packet; filing happens per-year where the 8275 IS written). T16 wires the export
  surface — co-emit the complete 8275 alongside the all-years dump there.

- **[Minor → fold in T17] 8275 Part I "no-loss" suffix misses the mixed clamp+documented-fee corner**
  (whole-branch tax review M1; `tax/form8275.rs`). A promoted leg sold BELOW floor with a documented
  fee-sat carry re-homed onto it (`rehome_onto_disposal_leg` runs AFTER `make_disposal_legs`) has
  `leg.basis = proceeds + documented_fee ≠ proceeds`, so the `leg.basis == leg.proceeds` heuristic does
  NOT append `NO_LOSS_SUFFIX` even though the estimate WAS clamped. **The disclosed AMOUNT is exactly
  as-filed (matches 8949 col (e)) and the direction is taxpayer-ADVERSE (lower-than-method basis, no
  penalty exposure)** — a disclosure-narrative completeness gap, not a filed-number defect or an
  aggressive-position mismatch. Requires an exotic multi-lot same-disposal fee draw on a below-floor
  promoted sale. Fix in T17 (the 8275-content completeness pass): base the suffix on
  `leg.gain < 0 || leg.basis < pre-clamp floor share` rather than `== proceeds`, or document the corner.

- **[Minor → owner T16 (graceful) + post-1b (pagination)] Form 8275 refuses beyond 6 Part I rows**
  (T15 review Minor-1; `form8275.rs:153-159`). `fill_form_8275` fail-closes with `FormsError::Overflow`
  past 6 Part I item rows (no silent truncation; consistent with the accepted Schedule-B "refuse-not-
  paginate" precedent) — but Form 8283 paginates via `merge_copies`. **T16 MUST ensure the export
  consumer handles `Err(Overflow)` GRACEFULLY** (a clear user-facing error naming the year + remedy, not
  a panic or a half-written packet) — this is the T15-review ⚠️ item. Actual pagination (a `merge_copies`
  equivalent for >6 promoted disposal legs in one year) is a **post-1b future enhancement** (ownerless).

- **[Minor → fold in T17] The 8275 free-text fault-injection oracle is weak by design** (T15 review
  Minor-2). Because Part I is pure free-text (`col:None, descent:None`), `verify_flat` only checks
  page + `/MaxLen` + no-unmapped; the existing fault KAT reds only because it targets the `/MaxLen 3`
  Line-No. comb cell. A same-page swap between two WIDE cells (item↔desc, or a row reorder) would
  silently mislabel and NOT fail closed. Inherent to the free-text design, not an impl defect — but pin
  it: T17 adds a per-field **sentinel** fill→readback KAT (distinct sentinel value per Part-I/II/identity
  field; read back BY FIELD NAME; assert each sentinel lands in its own field) so a map swap between wide
  cells is caught ([[untested-guard-pattern]]).

- **[RESOLVED — provenance] `f8275.pdf` provenance confirmed.** The T15 review could not confirm the
  bundled blob is the genuine official IRS asset (binary). Resolved by the controller: fetched directly
  from `https://www.irs.gov/pub/irs-pdf/f8275.pdf` (Rev. October 2024), sha256
  `9b4b82e3d0dd4eceac81eec700573481be91cfafc4d7f7e9796fd4dcec5fa164`, unmodified. Public-domain IRS form;
  consistent with the existing bundled f8283/f1040 blanks ([[licensing-notice-posture]] unaffected).

## Ownerless residue (cosmetic/doc — batch, no owning phase)

- **[Nit] `rehome_onto_*` per-fn docs still say "full basis carries"** (T5) — could add a one-line BG-D4
  estimate-withholding cross-ref; the `FeeCarry` struct doc already names the withholding.
- **[Nit] CLI `verify` prints "drift advisories: 0" while the TUI hides the line when empty** (T11) —
  pick one convention.
- **[Nit] Methodology header names "documented on-chain fee basis" even on a promote-only return** (T11)
  — narrow the wording.
- **[Nit] Interactive-TTY consent screen prints twice** (T10; a discarded ack=None preview + the real
  call) — restructure to compute figures once, then prompt. Correctness unaffected.
- **[Nit] `with_synthetic_promote` duplicated** across `conservative_promote.rs` (private) and
  `cmd/promote.rs` (whole-branch arch Nit-2) — both yield the same `promotes` set; consider `pub` + reuse.
- **[optional] Partial-promoted-removal cent-residue characterization KAT** (T6) — nice-to-have; the
  whole-tranche KATs + the `clamped_leg_basis` floor-at-$0 logic already cover the residue.
- **[Nit] `push_free` duplicated** across `form8275.rs` and `form8283.rs` (T15 review) — byte-for-byte;
  candidate to hoist into `cells.rs`. Also the inert `let writes = w; let placements = p;` rebinding in
  `form8275.rs` (inherited from form8283) can go.

## Post-release residue (cosmetic Minors/Nits from the Phase-1b whole-branch review; v0.9.0 shipped with these)

Both Phase-1b whole-branch lenses were GREEN 0C/0I; these are the non-gating residue, recorded for a
future patch cycle. None affects a filed/disclosed NUMBER.

- **[Minor] Whole-dollar (official 8275 PDF) vs exact-cents (crypto-slice 8949 + `form_8275.txt`)
  rounding inconsistency** (tax whole-branch M2) — sub-dollar; the PDF rounds Part I amounts to whole
  dollars (IRS convention) while the txt/8949 carry cents. Pick one presentation or document the split.
- **[Minor] TUI modal `compute_files` under-lists `form_8275.txt`** (tax whole-branch M3) — the export
  confirm modal's file preview omits the co-emitted `form_8275.txt`; the file IS written (T17), only the
  preview list is short. Add it to the modal's enumerated files.
- **[Minor] Overflow-refusal message literal duplicated** across the crypto-slice + full-return pre-checks
  (arch whole-branch) — extract the shared format string.
- **[Minor] Promoted-year collection duplicated** (arch whole-branch) — the set of years-with-a-promoted-
  leg is gathered in more than one place; hoist to one helper.
