# Conservative-filing — Phase-2 whole-branch TAX review — Fable r1 (2026-07-20)

**Reviewer:** independent Fable, TAX-correctness lens. **Scope:** the Phase-2 delta (Tasks 8–15) +
T16 follow-up fixes, i.e. `9a3f163..HEAD` (`45c6882`), plus Phase-1 surfaces only where Phase 2
depends on them. Contract: `SPEC.md` (G-1..G-4, D-1..D-10) + `IMPLEMENTATION_PLAN.md` (Tasks 8–16).
**Validation state observed:** `make check` green (2155 passed / 11 skipped, clippy clean).

**Method note:** every finding below was verified against the CURRENT source at review time; the two
lead findings were additionally **reproduced empirically** with a scratch harness (path-dep on
`btctax-core`, no repo changes) — outputs quoted inline.

**Verdict: 0 Critical / 3 Important / 4 Minor / 4 Nit.** No Critical found: I found no path on which
a WRONG tax figure is FILED, no understatement, no assumed character, and no `>$0` estimate reaching
a filed 8949. All three Importants are real defects that block the gate.

---

## IMPORTANT

### I-1 — The feature's own designed void-then-declare flow permanently hard-gates every tax year

- **Where:** `crates/btctax-core/src/project/resolve.rs:1296-1317` (the new D-8
  `has_tranche_residue` backstop) interacting with `resolve.rs:477-483` (allocation voids are
  collected into `allocation_voids`, **never** inserted into `voided`, so `resolve.rs:1261` never
  skips a voided allocation — it is re-evaluated on every rebuild) and
  `crates/btctax-cli/src/cmd/tranche.rs:55-77` (`in_force_allocation_exists`, which — correctly —
  admits the tranche in exactly this state).
- **Defect:** after a filer voids an **inert** `SafeHarborAllocation` (product-allowed: inert
  allocations stay voidable — `void.rs:44-50`, pinned by
  `tests/transition.rs:417 void_of_inert_allocation_applies_no_conflict`) and then declares a
  pre-2025 tranche (the record-time guard rightly admits it: `non_voided = false`, `effective =
  false`), every subsequent projection re-evaluates the **voided** allocation, finds
  `snap.estimated_conservative_remaining_sat > 0`, and pushes `SafeHarborUnconservable` — **Hard**
  (`state.rs:89`) — on the voided allocation's id. `compute_tax_year` refuses on ANY Hard blocker
  (`tax/compute.rs:203`), so **every tax year becomes NotComputable, permanently**: the allocation
  is already voided (nothing left to void), and the only clearing move is voiding the tranche —
  abandoning the feature. The tranche-side refusal hint (*"revisit the in-app safe-harbor
  allocation"*) **directs the filer into this exact flow**.
- **Reproduced** (scratch harness): documented pre-2025 buy → conserving-but-timebarred (ProRata,
  unattested) allocation → `VoidDecisionEvent` → `DeclareTranche(window_end 2018-12-31)`:
  - before the tranche: blockers = `[SafeHarborTimebar (Advisory)]`, **hard = 0**;
  - after: `SafeHarborUnconservable sev=Hard ev=Decision{seq:1}: "a conservative-filing tranche
    ($0 EstimatedConservative) remains in the pre-2025 residue …"`, **hard = 1**.
- **Why it isn't Critical:** fail-loud — no wrong figure is ever computed or filed. But it is a
  hard dead-end in the feature's primary designed path for exactly its target audience (the
  poor-records filer who once tried Path B), and no test covers void-allocation-then-declare.
- **Contract violated:** D-8's own architecture — the backstop exists to *deny effectiveness*; an
  already-voided inert allocation has no effectiveness to deny (a voided allocation can only remain
  in force via the §7.4 effective-conflict arm, which the blocker-absence check already covers), so
  the Hard blocker on it is pure poison. Also SPEC §1's purpose (produce a filing-ready return).
- **Fix direction (not prescriptive):** suppress the D-8 blocker push (or the whole
  timebar/unconservable evaluation) for an allocation that is void-targeted AND not in `effective`
  — i.e. let the applied void actually retire the decision from the D-8 arm; keep the §7.4
  effective-despite-void conflict path untouched. Add the void-inert-then-declare KAT (year stays
  computable) alongside the existing dangling-void-of-an-EFFECTIVE-allocation test.

### I-2 — P6 overpayment nudge treats the per-BTC window reference as the TOTAL lot cost (wrong "~$X" for any tranche ≠ 1 BTC)

- **Where:** `crates/btctax-core/src/conservative.rs:379` passes `wr.min` — **USD per WHOLE BTC**
  (`price.rs:6 usd_per_btc`) — as `reference`; `conservative.rs:291` writes it verbatim into
  `Acquire.usd_cost`, which the fold treats as the **whole-lot** basis for `sat` sats
  (`fold.rs:588 usd_basis = usd_cost + fee_usd`). No `× sat / SATS_PER_BTC` scaling (contrast
  `price.rs:13-18 fmv_of`). The correct reconstructed basis is `wr.min × sat / 100_000_000`.
- **Reproduced** (min daily close $20,000/BTC, Single, $60k ordinary, synthetic table, ST):
  - 1 BTC sold $50k → nudge "~$4400" — **correct only by coincidence** (basis $20k, 22% × $20k);
  - **2 BTC sold $100k → nudge "~$4400"; TRUE ≈ $8,800** (basis $40k) — 2× understated;
  - **0.1 BTC sold $5k → nudge "~$1760"; TRUE ≈ $440** (basis $2k) — 4× overstated, and the
    internal what-if put a fabricated **$15k loss** on the tranche leg (basis $20k vs $5k
    proceeds). The what-if state is discarded (never filed — no D-7 breach), but the quoted
    figure derives from it.
- **Why the suite is green:** every P6 fixture in `kat_conservative.rs` uses exactly
  `100_000_000` sat (1 BTC), the one size at which price == lot cost.
- **Contract violated:** SPEC P6 (`tax($0) − tax(window-reference)` — the reference is a *price*;
  the swap needs the lot-scaled basis) and **G-3** — the quantified lever is the point of the
  feature's fairness curve, and it misinforms the choice in both directions (forgo a real $8.8k
  reconstruction, or chase a phantom $1.7k one). Informational-only (never filed) → Important,
  not Critical.
- Note: the public `overpayment_delta(refs: &[(EventId, Usd)])` inherits the ambiguity — its doc
  says "reference price" while the implementation needs a whole-lot USD amount. Fix the scaling at
  the nudge call (or scale inside `overpayment_delta_one` off the tranche's `sat`) and pin with a
  non-1-BTC KAT.

### I-3 — The MANDATORY D-4 methodology disclosure is not produced by the `export-irs-pdf` filing surface

- **Where:** `crates/btctax-cli/src/cmd/admin.rs:247-445` (`export_irs_pdf`, the crypto-slice PDF
  packet) and `admin.rs:482+` (`export_full_return`) write the 8949/Schedule-D/SE/8283/1040 PDFs —
  including the $0-basis tranche rows — but never `basis_methodology.txt`. Only the CSV paths write
  it (`render.rs:871` `write_csv_exports`, `render.rs:911` `write_form_csvs` → TUI export).
- **Contract violated:** D-4/P7 — the disclosure is "REQUIRED … not opt-in", "MANDATORY when a
  tranche is filed" (the i8949 asks for a basis explanation whenever actual cost isn't used), and
  the plan's Task 13 scopes the export to *"the `export-irs-pdf` / CSV export path
  (`cmd/admin.rs`)"* by name. P7's own test rubric calls a filed-tranche year without the
  disclosure "a hard gap". A filer who files from the PDF packet — the flagship filing-ready
  artifact — gets a $0-basis 8949 with no methodology artifact anywhere in the output directory.
- **Failure scenario:** tranche filed in 2026 → `btctax export-irs-pdf --out d/ --tax-year 2026`
  → `d/` contains `f8949.pdf` (Box-I/L row, $0 basis) and no `basis_methodology.txt`; the filer
  mails the packet without the i8949-requested explanation the SPEC promised to force.
- **Fix direction:** call the shared `basis_methodology` writer from both `export_irs_pdf` and
  `export_full_return` (same "Some ⇒ write" gate), + a presence KAT per surface.

---

## MINOR

### M-1 — Disclosure header unconditionally asserts "$0 … was used as filed" while an enumerated leg can be filed >$0

`conservative.rs:152-159`: the header says *"A conservative $0 basis … was used as filed"* for ALL
units, but in the TP8(c) fee-carry corner (pinned by
`kat_conservative.rs:1219 tp8c_fee_sat_basis_can_land_on_the_last_tranche_leg_corner_b`) the
enumerated bullet correctly reads "filed at $30 basis" — the compliance document contradicts itself
in exactly the corner tax r1 I-1 was about. The per-item lines are right; condition the header
("$0, plus any documented fee-sat basis re-homed by the fee flow") or scope it to the $0 legs.

### M-2 — Disclosure omits `window_start` (SPEC P7: "enumerates each tranche's window")

`conservative.rs:139-146`: each bullet carries only `leg.acquired_at` (= `window_end`) as
"estimated acquired by {date}". The declared window's start never appears anywhere in the filed
disclosure, and two tranches with different `window_start` but the same `window_end` are
indistinguishable. Honest and conservative (the upper bound is the binding date for term), but a
SPEC-copy deviation in a REQUIRED artifact. (The P6 nudge does print `{ws}–{we}`.)

### M-3 — `safe_harbor_residue` refusal drops the D-8 normative finality hedge

`session.rs:692-698`: the T16 opener refusal ends "Void the tranche first to allocate." — without
the *"if you have already filed the tranche's $0 basis, unallocated pre-2025 units are a
facts-and-circumstances matter for a professional"* hedge that SPEC D-8 makes normative for the
refusal surfaces and that `TRANCHE_IS_FINAL_HINT` carries (`cmd/tranche.rs:31-33`). On the TUI
allocate opener this Err **is** the whole user-visible message, and it invites voiding a tranche
the filer may have already filed, unhedged.

### M-4 — Method-inversion advisory recommends the wrong lever for a pre-2025-keyed method

`conservative.rs:76-81` + `conservative.rs:486-488`: for a report year < 2025,
`in_force_methods(as_of Dec-31)` returns `config.pre2025_method`; the advisory then recommends
`btctax config --set-forward-method hifo` — a FORWARD `MethodElection`, which by construction
cannot change any pre-2025-dated disposal (late-imported back-records consume under
`config.pre2025_method`). Advisory-only and never understates, but the named remedy does not
address the warned mechanism in that staging; mention the pre-2025 method config when the keyed
method is the pre-2025 one.

---

## NIT

- **N-1** `conservative.rs:270/301-303`: doc claims the per-tranche delta is "Never negative" but
  `baseline - with_tax` is returned unclamped and summed in the public `overpayment_delta`; the
  nudge path filters `delta <= 0` per-line, the pub fn does not. Today's engine components
  (LTCG stack, NIIT, ordinary) are monotone in gain, so unreachable — clamp or drop the claim.
- **N-2** `conservative.rs:486-487`: `.expect` on `Date::from_calendar_date(year, December, 31)`
  panics for a year outside `time::Date`'s range — reachable only via an absurd `--tax-year`.
- **N-3** P4 copy (`conservative.rs:238-240`) says the Notices 2025-7/2026-20 relief "ended
  2026-12-31" — correct from the ≥2027 sale's vantage (the only branch that fires), but the warning
  can render in 2026 for a future-dated sale; "ends"/"ended by then" phrasing would be exact.
- **N-4** Dip advisory copy renders the corner-(a) fee-driven negative as e.g. "$-30 gain"
  (`conservative.rs:31-41`) — the number is honest (§1001(b) net), the copy awkward.

---

## Checked and CLEAN (no finding at any severity)

- **G-4 (never understate / derived character):** every term string in the new surface derives
  from `leg.term` (dip `conservative.rs:37`, disclosure `:135-138`); no hard-coded "long-term"
  (ST-fixture KAT pins it); Part/box untouched by this delta and inherited from the shipped box
  fix (Phase-1 KATs: Box L/I/F + the strict-`>` one-year boundary).
- **G-2/D-7 ($0 the only filed basis):** nothing in the delta writes a `>$0` estimate to any filed
  surface. `window_reference`/`overpayment_delta` feed only advisory strings; the basis-replacement
  what-if state is local to `overpayment_delta_one` and discarded; `basis_methodology.txt` is
  prose. The only `>$0` on a tranche row is the TP8(c) documented fee-sat carry — real §1011 basis,
  exactly as the amended SPEC §6 scopes.
- **No-loss-from-the-estimate invariant (T15):** correctly scoped and pinned — fee-free core ≥ 0
  ($0-proceeds exactly 0); corner (a) documented `fee_usd > proceeds` drives gain < 0 with the
  estimate intact at $0 (§1001(b)); corner (b) staged reachably (specific-ID naming the full
  tranche with a documented lot remaining for the FIFO fee draw — per plan-tax r2 NEW-1 the pure-
  HIFO staging is correctly NOT claimed); Σ-conservation + `sat ≤ 0`/id-guard pins present.
- **D-8 mutual exclusion (apart from I-1):** record-time refusal fires both directions, effective
  OR inert, all four allocation append sites through the one chokepoint; the ≥2025-window tranche
  coexists by design (`universal_snapshot` filters `e.date() < TRANSITION_DATE`, so a ≥2025
  `window_end` never enters the residue — verified); the projection backstop keys on
  `remaining_sat > 0` (fully-consumed tranche exempt, arch r4 Nit-2); the hand-crafted
  dangling-void-of-an-EFFECTIVE-allocation still blocks (T16 test).
- **D-3/P4:** pure `persistability` reuse — fires iff broker wallet ∧ sale year ≥ 2027; silent for
  SelfCustody and ≤2026 (three-way KAT + builder KAT); the disposal-scoped (not selection-gated)
  firing is documented as deliberate and errs toward warning.
- **D-9/P2:** HIFO-draws-documented-first and the FIFO inversion pinned as characterization in the
  pre-2025 pool (correct staging per arch M-2); the inversion is correctly described as
  gain-maximizing, never an understatement.
- **P6 §1014 note:** legally correct and provenance-neutral — §1014(a) date-of-death FMV basis
  with no cost records; §1223(9) automatic long-term. Partial-coverage caveat surfaces (tax r1
  N-3). Per-tranche references summed (multi-window KAT).
- **Provenance neutrality:** genuine across all five builders — "undocumented BTC", "estimated
  acquired by"; no "purchase"/"bought" anywhere (KAT-pinned on dip, nudge, disclosure).
- **Surfacing:** one shared assembler (`tranche_report_advisory`) feeds both `report --tax-year`
  (`cmd/tax.rs:429-438` → `main.rs` print) and the TUI Tax tab — no drift; `in_force_methods` zip
  is input-order aligned; TUI `compute_files` and `write_form_csvs` share the same
  `basis_methodology` gate.
- **Engine id-guard (T15):** `build_op` DeclareTranche arm guarded on `EventId::Decision` ∧
  `sat > 0` → `Op::Skip` for both malformed shapes; `ClassifyRaw`-routed and `sat ≤ 0` KATs pin
  it; Σ-conservation holds.

## Counts

**0 Critical / 3 Important / 4 Minor / 4 Nit.**
