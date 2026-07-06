# Whole-diff review (Phase E) — feat/irs-form-fill-sp2 — round 1

**Verdict: 0 Critical / 0 Important — SHIP.**

Independent Phase-E review. Diff `main (99f26ca)..HEAD` — 3 task commits (T0–T3 engine `cdaaa0f` + CLI `0206fef`
+ docs `d10ca52`). Contract: `design/SPEC_irs_form_fill_sp2.md` (R0-GREEN, 3 rounds; round 1 on Fable caught 4
Criticals — the SE line-12/8959 split, the 1040 7a renumber, the non-reusable read-back, the perjury-risk
DA-YES, the 8283 digital-assets checkbox). SP2 of task #45 — the full IRS packet (8283 + Schedule SE + 1040
cap-gains), TY2025.

## Fault-injection of the ★ per-form oracle (map restored byte-for-byte)
- **[★ the C3 concern — the geometric read-back had to be REDESIGNED for scattered-field forms] CONFIRMED
  fail-closed, both legs.** My independent fault-inject: swapping SE **line 10 ↔ line 11** (BOTH in the amount
  column — a same-column swap that column-x membership CANNOT see) drove **8 SE KATs RED** via the ordinal-y
  descent leg (fill returns Err, no bytes). The complementary cross-column leg (SE 12↔13) is covered by the
  implementer's `fault_injected_se_cross_column_swap_12_13_is_red` (column-x), and the DA Yes/No pair by
  `fault_injected_1040_da_yes_no_swap_is_red` (same-y `/Btn` predicate). All three oracle legs bite; a
  mis-mapped cell on any of the three scattered-field forms cannot escape. `no_unmapped_filled` reused per form.

## Verified by KAT + inspection (my own runs)
- **[C1] Schedule SE line 12 = SS + Medicare only.** `schedule_se_line12_equals_ss_plus_medicare` +
  `schedule_se_line12_excludes_addl_medicare` pass — the $300k golden fills **29,870.85**, NOT the
  `SeTaxResult.total` 30,564.30 (the 0.9% Additional Medicare Tax is a Form 8959 item, advisory printed). Line
  13 = deductible_half; the full chain 2/3/4a/4c/6/8a/8d/9/10/11/12/13 is self-consistent; $400-floor skip;
  W-2≥ss_wage_base skips 8b–10.
- **[C2 + I★1] 1040 line 7a** = `f1_70`: filled iff Schedule D ACTIVE ∧ line 16 ≥ 0; **blank when Schedule D
  inactive** (income/donation-only year, DA=YES) — no unearned `-0-`; net loss → blank + §1211 notice; 7b
  untouched.
- **[C3/8283] "k Digital assets" box** = `Lines2i-l[0].c1_6[2]` on-state `/11` (map-confirmed) — not "l
  Other"/"f Securities"; Parts III/IV/V pinned by their own `/Rect`; "(mo., yr.)" dates; identity filled,
  other-party declarations blank; overflow via `merge_copies`.
- **[C4] DA question = YES iff reportable activity** (disposal ∨ ANY income_recognized ∨ gift/donate removal
  from `state.removals`); else BOTH boxes blank + the whole 1040 skipped. `c1_10[0]`=/1 (Yes, LEFT of the
  top-most same-y pair, oracle-verified map-independently).
- **Determinism:** golden sha256 per new form; the implementer's independent pypdf on genuinely-exported PDFs
  confirmed **XFA=0** + NeedAppearances on schedule_se / form_8283 / form_1040_capgains + the DRAFT-watermarked
  pseudo variant. Pseudo ⇒ attestation + watermark reused for every new form.
- **Conditional presence:** 8283 only with donations; Schedule SE only with SE income ≥ $400 net-earnings
  floor (se_net_income discriminator NOTEs a missing-profile case); 1040 only with reportable activity.

## Engine KATs + suite + isolation
btctax-forms: **46 KATs pass** (27 new SP2 across sp2.rs) + the SP1 battery. Full workspace `cargo test
--locked` = **1273 passed / 0 failed** (implementer; my independent close-out re-running to confirm). clippy
-D + fmt clean; `check-isolation` OK — btctax-forms still links no `ureq`/rustls (all-pure-Rust; the tax
binaries cannot open a socket). MINOR (+3 fill fns, +3 maps, +3 bundled public-domain PDFs, +the per-form
oracle) → next release bump.

## FOLLOWUP (non-blocking, filed)
- **Multi-donee 8283:** a year donating to multiple DISTINCT donees fills only the first carrier row's donee
  identity on page 2 (flagged by the partial-scope + needs-review notices). Single/multi-LOT donations to one
  donee are fully covered. Worth a FOLLOWUP if multi-donee packets become common.

**SHIP — SP2 completes the IRS packet (fail-closed per-form oracle; SE line 12 correct; 1040 7a honest; 8283
digital-assets box; DA-iff-activity). SP3 (2017 + 2024 maps, per-year box taxonomy) remains.**
