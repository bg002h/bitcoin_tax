# Whole-slug review — P2-B: Form 8949 + Schedule D generation (round 1)

- **Slug diff:** `f9daf84..4ca28a9` (3 commits: spec R0-GREEN, Task 1 `acquired_at`+`wallet`, Task 2/3 forms).
- **Scope:** FINAL whole-diff / cross-cutting net over all 3 tasks. These are FILING ARTIFACTS a
  taxpayer submits — a wrong 8949 row, an unreconciled Schedule D, or any change to capital-gains
  math = Critical.
- **Reviewer role:** independent whole-diff reviewer (author ≠ reviewer).
- **Sources cross-checked at current tree:** `forms.rs` (full), `state.rs` (DisposalLeg),
  `fold.rs:154-204` (make_disposal_legs zone-coupling + single push site), `compute.rs:277-291`
  (crypto_st/lt loop — confirmed UNCHANGED), `render.rs` (CSVs + render_schedule_d + wallet_label /
  dispose_kind_tag), `tax.rs` (report_tax_year), `main.rs` (wiring), the KATs (kat_forms.rs,
  kat_tax.rs Task-1, tax_compute.rs reconciliation), export/tax_report integration tests.
- **Gate:** GREEN per prompt (487 tests, clippy -D clean, fmt, release, PII clean) — not re-run;
  reviewed the code/diff.

## Verdict: **READY TO MERGE — 0 Critical / 0 Important.** (2 Minor, 3 Nit — all non-blocking.)

The headline risk — a wrong 8949 row whose date-acquired contradicts its ST/LT term — is
**structurally prevented**: `acquired_at` is set from the *same* HP-start fed to `term_for` in every
one of the four fold branches, so it can never contradict `leg.term`. The loss-zone case (R0-C1) is
correctly implemented and locked by KAT (c). Capital-gains math is untouched (compute.rs not in the
diff; fold.rs only appends two read-only fields to the leg tuple). Schedule D reconciles with engine
B via genuinely independent code paths. No fabricated box is reachable (the `Form8949Box` enum has
only `C`/`F` variants).

---

## Cross-cutting dimension assessment

### 1. 8949 row correctness end-to-end — PASS
- **description** = `Decimal::from(sat) / Decimal::from(SATS_PER_BTC)` at `{:.8}` (`forms.rs:82-85`).
  Exact Decimal; no f64. Quotient is ≤ 8dp so the format is lossless. Unit tests + KAT confirm
  `0.53000000 BTC`, `0.00000001 BTC`, `0.12345678 BTC`.
- **date_acquired = zone-aware `leg.acquired_at`** — the crux. Verified branch-by-branch in
  `make_disposal_legs`: gain zone / NGNL / non-dual → `term_for(c.gain_hp_start,…)` and
  `acquired_at = c.gain_hp_start`; **loss zone → `term_for(c.loss_hp_start,…)` and
  `acquired_at = c.loss_hp_start`** (gift date; §1015 loss basis does NOT tack). `acquired_at` is
  set from the SAME branch tuple that produces `term`, at the SINGLE production push site
  (`fold.rs:200`). ⇒ **no date-acquired-vs-term contradiction can reach the 8949.** KATs (a) ordinary
  = purchase date/LT, (b) gain-zone gift = donor tacked date/LT, (c) loss-zone gift = gift date/ST
  (with an `assert_ne!` that it is NOT the donor date) all lock this.
- date_sold=`d.disposed_at`, proceeds/cost_basis/gain copied verbatim from the leg; part from term.
- **NGNL gift leg**: row emitted, `basis==proceeds` ⇒ gain 0, adjustment cols blank (fold sets
  basis=proceeds; forms copies verbatim — no recompute). KAT present.
- **adjustment cols**: `adjustment_code=""`, `adjustment_amount=Usd::ZERO` always (§1091 N/A to crypto).
- **per-leg granularity** (one row per DisposalLeg) — correct; avoids "VARIOUS" date-acquired.
- **deterministic ordering**: `sort_by` on (disposed_at, event id, lot_id); Rust's stable sort makes
  even equal-key legs order-deterministic. Total, deterministic (NFR4). KAT covers scrambled input.
- **year filter**: `.filter(|d| d.disposed_at.year()==year)`; KAT excludes prior AND future year.

### 2. Box honesty — PASS
- Default **C (ST) / F (LT)** = "not reported on a 1099-B". `box_needs_review = matches!(leg.wallet,
  WalletId::Exchange{..})` — the DIRECT match required by [R0-M2], not the private
  `optimize.rs::is_broker`. KAT: exchange → flagged + C; self-custody → not flagged + C.
- **Never auto-assigns A/B/D/E**: the `Form8949Box` enum only *has* `C` and `F`, so a substantiated
  box is structurally unrepresentable. No path fabricates one.

### 3. Schedule D + reconciliation with engine B — PASS
- `schedule_d(state,year)` sums raw ST/LT Σproceeds/Σbasis/Σgain per part (signed gain). Hand-derived
  golden KAT (incl. a signed LT loss leg + out-of-year exclusion) passes; empty-year → all zero.
- **Reconciliation is a real cross-check, not a tautology.** `schedule_d` (forms.rs) and
  `compute_tax_year` (compute.rs) are separate functions that each independently re-sum
  `state.disposals` by term/year. I diffed the two loops: `compute.rs:280-291` iterates
  `.filter(|d| d.disposed_at.year()==year)` over all legs summing `leg.gain` by `Term` — **byte-for-
  byte the same leg set as `schedule_d`** (both include `fee_mini_disposition` legs identically, so
  no divergence). On the all-gains / zero-carryforward / zero-other fixture, `net_1222` is a no-op
  (`st_net=crypto_st`, `lt_net=crypto_lt`), so the KAT's `sd.st.gain==r.st_net` / `sd.lt.gain==r.lt_net`
  is a sound equality; a `dec!(20000)`/`dec!(50000)` sanity assert backstops it.
- Raw-here vs netted-in-B distinction disclosed: `render_schedule_d` prints the
  "§1222/§1211/§1212 netting + carryforward … applied in the tax computation … raw pre-netting"
  note. B's tax math is UNCHANGED (compute.rs absent from the diff's files-changed list).

### 4. No unintended change / API deviations — PASS
- No capital-gains / removal / disposal-basis math changed. `fold.rs` only extends the zone `match`
  tuple from 4→5 elements to thread `acquired_at`, and adds `acquired_at`/`wallet` to the one
  DisposalLeg literal; `(basis, gain, term, gift_zone)` are byte-identical to before.
- Signature changes are sound and all callers updated (green build proves completeness):
  `write_csv_exports(+tax_year: Option<i32>)`, `export_snapshot(+tax_year)`,
  `report_tax_year(→ …, ScheduleDTotals)`. Test callers pass `None` / destructure the 3-tuple.
- The two filing CSVs are year-scoped (written only when `tax_year` is `Some`) — correct, forms are
  per-year. All-years CSVs (lots/disposals/removals/income) still written unconditionally;
  disposals.csv gained `acquired_at`+`wallet` columns (additive, appended, stable snake_case).

### 5. NFR4/NFR5, CSV perms/columns, privacy, backward-compat — PASS
- Determinism: total + stable ordering. No float anywhere (all money is `Usd`=Decimal; description is
  exact Decimal). form8949.csv + schedule_d.csv both opened via `fsperms::open_owner_only` (0o600);
  export tests assert 0o600 on the pre-existing-dir path. Stable snake_case headers, asserted verbatim
  in `export_writes_year_scoped_form8949_and_schedule_d`. `wallet_label`/`dispose_kind_tag`/part/box
  tags are stable strings (not Debug). Privacy: KATs are synthetic-only (kat_forms.rs states it; the
  direct-state fixtures read no user file). Backward-compat: `DisposalLeg` derives only
  `Debug,Clone,PartialEq,Eq` (serde-free) ⇒ two additive fields are migration-free.

### 6. Cross-task consistency / dead code / spec drift — PASS (minor notes below)
- No dead code (clippy -D would fail otherwise); the new `form_8949`/`schedule_d`/enums/tags are all
  used. Only one production DisposalLeg literal (`fold.rs:200`); all other literals are tests. Both
  new fields set at that single site.

---

## Findings

### Minor

**M1 — `render_schedule_d` is printed unconditionally, even when the tax outcome is NotComputable.**
`main.rs` prints `render_schedule_d(y, &sched_d)` right after `render_tax_outcome`, with no gate on
the outcome. `report_tax_year` computes `sched_d` regardless of blockers. For **TaxProfileMissing**
this is fine and even useful (the gains are known; only the profile is absent). For a **hard blocker
(TaxYearNotComputable)** the projected legs may be incomplete/placeholder, yet the user still sees
concrete "Schedule D … part totals" beneath a "NOT COMPUTABLE" line. The numbers are the arithmetically
correct sum of whatever legs projected, and the "raw pre-netting" note is present, so this is a
clarity concern, not a wrong-number defect. *Recommendation:* when the outcome is NotComputable due to
a hard blocker, either suppress the Schedule D text or add a one-line caveat ("data-integrity blockers
present; these raw totals are provisional"). Non-blocking. (The CSV filing artifacts are produced only
via `export-snapshot --tax-year`, a separate raw-dump path, so this affects only the advisory text.)

**M2 — Task-4 FOLLOWUPS deferrals are not recorded in `FOLLOWUPS.md`.** The slug leaves `FOLLOWUPS.md`
untouched (git diff over the 3 commits is empty for that file). Of the three deferrals the SPEC Task 4
mandates be recorded, only "filled IRS 8949 + Schedule D PDFs" pre-exists (line 362); **per-disposition
1099-B / box A/B/D/E user input** and **1099-DA reconciliation** are absent (no `1099-DA` hit in the
file). Per CLAUDE.md, `FOLLOWUPS.md` is a required workflow artifact. This does not affect filing-artifact
correctness (0C/0I stands), but completing Task 4 requires appending the two missing deferrals (see
triage below). Fix before ship as part of closing the slug.

### Nit

**N1 — `box_needs_review` flags ALL exchange dispositions regardless of year.** Pre-2025 exchange
sales generally carried no 1099-B/1099-DA, so flagging them is slightly over-inclusive. It is
conservative and harmless (only prompts a manual check) and matches R0's accepted N1. Optional: gate
on `disposed_at.year() >= 2025`.

**N2 — CSV dates are ISO `YYYY-MM-DD`; Form 8949 itself wants `MM/DD/YYYY`.** Unambiguous and correct
for a CSV interchange artifact; a presentation concern to fold into the deferred filled-PDF work (R0 N3).

**N3 — Spec internal inconsistency (spec, not code): D2's column enumeration omits `box_needs_review`,
which D4 mandates as a "flag/column".** The implementation correctly includes `box_needs_review` as
CSV column 3 (asserted by the export test). No code defect; the SPEC's D2 list is merely less complete
than D4. Opportunistic spec tidy.

---

## Deferred-FOLLOWUPS triage (BLOCK / DEFER)

1. **Per-disposition 1099-B / box A/B/D/E user input (reclassify from the C/F default) — DEFER.**
   The shipped default is honest and never overstates: C/F is the conservative "not reported on a
   1099-B" position, `box_needs_review` surfaces exchange dispositions for manual reclassification, and
   proceeds/basis/gain are correct **regardless of box**, so no tax figure is misstated. A reclassify
   input is an additive enhancement, not a correctness blocker. (Must be appended to FOLLOWUPS.md — M2.)

2. **Filled-PDF 8949 / Schedule D generation — DEFER.** Out of scope by design (no PDF dependency
   in-tree); CSV + text summary is the delivered form. Already recorded in FOLLOWUPS.md (line 362).

3. **1099-DA reconciliation (2025+ broker reporting) — DEFER.** Requires importing broker 1099-DA data;
   explicitly out of scope. The `box_needs_review` flag already prompts the user to reconcile manually.
   (Must be appended to FOLLOWUPS.md — M2.)

No deferral is a BLOCK: each is additive, and none is required for the produced 8949/Schedule D to be
correct and honest.

---

## Bottom line

**P2-B is ready to merge: 0 Critical / 0 Important.** The filing-artifact invariants that matter —
date-acquired always consistent with term (incl. the §1015 loss zone), no fabricated box, per-leg
exhaustive rows, deterministic ordering, exact-Decimal amounts, unchanged capital-gains math, and a
genuine (non-tautological) Schedule-D↔engine-B reconciliation — all hold. Address M2 (append the two
missing FOLLOWUPS deferrals) as part of closing the slug; M1/N1–N3 are optional polish.
