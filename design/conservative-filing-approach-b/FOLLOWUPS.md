# Approach-B sub-project-1 — FOLLOWUPS

Follow-ups for the basis-floor / PromoteTranche feature (`feat/conservative-filing-b`).
**Blocking findings (Critical/Important) are NEVER filed here — they are fixed before their gate
closes.** Each entry records its **owning phase** so reconciliation is a grep (STANDARD_WORKFLOW §
in-phase burndown). Only ownerless residue batches to the end.

## Open — Phase 1b (T16) export-surface requirements

These three are the export-surface completeness items T16 owns. The Phase-1a completeness GATE
(`promote_export_gate`, refuse-before-bytes on the three CLI export fns) already prevents any
INADEQUATE-disclosure filing from escaping today; these close the remaining surface coverage when
`promote` reaches the released/TUI surface in Phase 1b. **The T16 brief MUST reference this section.**

- **[★ HARD BG-D8 requirement on T16 — NOT a deferrable cleanup] The TUI export path
  (`btctax_tui::export::do_export` → `write_form_csvs`) must come behind the BG-D8 gate.** (Raised T14;
  re-tagged HARD per the T14 Opus review + reaffirmed by the whole-branch arch review.) Task 14 put
  `promote_export_gate` + the `form_8275.txt` emit on the THREE CLI export fns (`export_snapshot`,
  `export_irs_pdf`, `export_full_return`) at the CLI layer, NOT in the shared `render.rs` writers. On
  the Phase-1a branch the TUI CANNOT bypass the gate — `btctax-tui` is source-gated against
  `export_snapshot`/`write_csv_exports` (the `e10_mechanized_source_gate` test), so there is no second
  export surface yet, and `promote` is CLI-only / unreleased. **★ In Phase 1b this becomes HARD: once
  `promote` reaches the released/TUI surface, a TUI-exported promoted packet would (a) never refuse an
  incomplete 8275 and (b) omit `form_8275.txt` even for a COMPLETE promoted leg — a direct BG-D8
  (Reg §1.6662-4(f) inadequate-disclosure) violation. T16 MUST gate `write_form_csvs` (or route the TUI
  export through the gated CLI fns) AND carry its own refuse + emit KATs on the TUI surface; it cannot
  ship 1b without it.**

- **[Minor → fold in T16] All-years CSV snapshot omits the `form_8275.txt` artifact** (whole-branch tax
  review M2; `cmd/admin.rs`, `export_snapshot`). `write_form_8275_txt` is written only under
  `if let Some(y) = tax_year`; the `tax_year: None` all-years dump exports the promoted disposal rows but
  no 8275 file rides alongside. The completeness GATE still fires for `None` (an incomplete Part II
  refuses even in the all-years path), so **no inadequately-disclosed position escapes** — this is a
  letter-of-BG-D8 surface-coverage gap, not a filed-number defect (low impact: a raw projection dump, not
  a per-year filing packet; filing happens per-year where the 8275 IS written). T16 owns the export
  surface — co-emit the complete 8275 alongside the all-years dump there.

- **[Minor → fold in T16] 8275 Part I "no-loss" suffix misses the mixed clamp+documented-fee corner**
  (whole-branch tax review M1; `tax/form8275.rs`). A promoted leg sold BELOW floor with a documented
  fee-sat carry re-homed onto it (`rehome_onto_disposal_leg` runs AFTER `make_disposal_legs`) has
  `leg.basis = proceeds + documented_fee ≠ proceeds`, so the `leg.basis == leg.proceeds` heuristic does
  NOT append `NO_LOSS_SUFFIX` even though the estimate WAS clamped. **The disclosed AMOUNT is exactly
  as-filed (matches 8949 col (e)) and the direction is taxpayer-ADVERSE (lower-than-method basis, no
  penalty exposure)** — a disclosure-narrative completeness gap, not a filed-number defect or an
  aggressive-position mismatch. Requires an exotic multi-lot same-disposal fee draw on a below-floor
  promoted sale. Fix in T16 (which re-touches 8275 Part I for the AcroForm): base the suffix on
  `leg.gain < 0 || leg.basis < pre-clamp floor share` rather than `== proceeds`, or document the corner.

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
