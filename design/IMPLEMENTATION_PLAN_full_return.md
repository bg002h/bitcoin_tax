# IMPLEMENTATION PLAN — Full Return v1 (Common W-2 Household, TY2024)

**Status:** DRAFT r3 (was GREEN r2; folded the whole-design Fable audit's plan-touching items →
`design/full-return/reviews/DESIGN-fable-audit-final.md`) → pending Fable confirmation → user review. Reviews:
`design/full-return/reviews/PLAN-fable-review-r{1,2}.md`.
**r3 changelog (audit fold):** added KAT-19 (Form 8615 kiddie-tax refuse, P1) + KAT-20 (box-12 allowlist, P1)
+ Schedule C net<0 refuse (P2); spec r5 dropped the orphan `qbi_override` (no plan task existed — closed).
**Implements:** `design/SPEC_full_return.md` (GREEN r4). **Governs:** `STANDARD_WORKFLOW.md`.
**Open FOLLOWUPS to fold as encountered:** `design/full-return/FOLLOWUPS.md`.

**r2 changelog (plan review r1 fold):** C1 KAT-3 assertion corrected to "edge ≡ 0 (mod $25)" + midpoint-edge
KAT (deep/01's no-interior-edge was TY2024-only); C2 added the delta-vs-absolute dual-reporting task (P4 t8);
I1 KAT-15→P2 + KAT-18 Sch B trigger task; I2 P1 resolver `ReturnInputs`-arm stubbed, full precedence→P2; I3
FROZEN guard = enumerated content-pin; I4 `derive_tax_profile` uses NON-crypto scalars (frozen contract), not
with-crypto AGI; I5 P0 mode-proof uses the half-even-discriminating cells (1,163/303), KAT-9 = cross-foot only.
Minors 1–7 folded; two spec errata (§8 KAT-3 wording; §4.8 L36) → FOLLOWUPS.

Reference the spec by section; this plan sequences the build and does not restate design. Every task is
**test-first (TDD)**: write the failing test (KAT / unit / golden), then the minimal implementation, then
refactor. Each phase ends at **green** = full workspace test suite passes **and** an independent Fable review
of that phase's diff is **0 Critical / 0 Important** (per §2 of the workflow).

## Global invariants (assert continuously, every phase)

- **FROZEN (I3):** never edit the **enumerated** frozen-path set — `crates/btctax-core/src/tax/{types.rs,
  compute.rs, se.rs}` and the delta-only helpers they call (`net_1222`, the NIIT closure, `preferential_tax`,
  `ordinary_tax_on`) — and never alter what-if/pseudo-reconcile or the existing crypto tests. **P0 task 0
  defines the guard concretely:** a CI test pins each frozen file's **content** (SHA-256, or
  `git diff --exit-code` vs a recorded baseline) — *content*, not just public surface, since the invariant is
  "never *edit*." A baseline update is its own separately-reviewed change (documented exception).
- **Additive:** all new code is new modules + new side-table + extended fillers (spec §2 module list).
- **Fail-closed:** the §4.10 refuse-guard table is authoritative; one KAT per row; a captured-but-unmodeled
  input that could change the return ⇒ `NotComputable`, never a silent value (spec §3.4).
- **Determinism:** golden-PDF SHA-256 per new form; `round_dollar` half-up distinct from `round_cents`.
- **CI cross-check:** vendor a TY2024 slice of CC0 PSL Tax-Calculator params; a CI test diffs the bundled
  `tax_tables.rs` standard-deduction / bracket / LTCG / NIIT / Add'l-Medicare values against it (spec §9/deep05).

## KAT ownership (spec §10 → phases)

KAT 1–3 → P0 · **KAT 9 → P0 (arithmetic + round-mode) + P4/P6 (printed cross-foot on real 8959 lines)** [I5] ·
KAT 5/5b,6,12,16 → P4 · KAT 13,17 → P3 · KAT 14 → P4 · **KAT 15 → P2** [I1] · **KAT 18 (Sch B trigger) → P2
(decision) + P6 (fill)** [I1] · KAT 7,8,10 → P6 · KAT 4 → (TY2025 follow-on) · KAT 11 → P3/P4 · refuse-row KATs
→ P1 (input-screenable rows) + P3/P4 (compute-dependent: TI≤0, excess-SS) + P6 (form-specific). Golden
end-to-end matrix → P7.

---

## Phase 0 — Conventions & tax method (spec §3.1, §4.11-adjacent, §5 stage 4, §8)

**Goal:** the year-independent absolute-tax kernel, provably matching the IRS Tax Table/TCW/QDCGT.
**New:** `conventions::round_dollar` (MidpointAwayFromZero); `btctax-core/src/tax/method.rs` (`round_dollar`
compose helper, `tax_table(sched, ti)`, `tax_on_amount(sched, x)` [Table if <100k else TCW], `qdcgt_line16`).
**Tasks (test-first):**
0. Define the **FROZEN CI guard** (enumerated frozen-path list + per-file content pin; Global invariants I3).
1. **Rounding MODE [I5]:** red test = deep/01's half-even-**discriminating** printed cells — MFJ [11,600,11,650)
   = **1,163**, Single [3,000,3,050) = **303** (half-even yields 1,162/302) — with a fault-inject check that
   `round_cents` (half-even) FAILS them → implement `round_dollar` (MidpointAwayFromZero). (KAT-9's cross-foot
   role is separate — task 6.)
2. KAT-1 (pref cap `min(TI, qd+ltcg)`; TI 35,400/QD 50,000 ⇒ line16 $0) → `qdcgt_line16` with the cap.
3. KAT-2 (binding-min same-bin; L5 58,000/QD 10 ⇒ 7,819 via L24) → `min(L23,L24)` load-bearing.
4. **KAT-3 (corrected, C1): every bracket edge < $100k is a multiple of $25** (a $50-bin boundary OR its exact
   midpoint — a midpoint edge still reproduces the printed cell because IRS taxes at the midpoint and TCW is
   continuous at edges; the mod-12.5 form in the $25-bin region is vacuous today) → assertion over every bundled
   `TaxTable` year, PLUS a pinned **midpoint-edge cell KAT** (TY2025 Single [11,900,11,950), edge 11,925 — deep/01's
   "no interior edge" was TY2024-only). The 3 deep/01 worked examples as regression KATs. *(Spec §8/KAT-3 wording
   erratum → FOLLOWUPS.)*
5. Bundle per-year **indexed** data in `TaxTable` (standard-deduction basic + §63(f) aged/blind + dependent
   floor; add to `ty2024()`, and as `Option`/defaulted so `ty2017/25/26` still compile). Keep **statutory
   constants** (SALT cap $10k/$5k; excess-SS MAX = 6.2%×`ss_wage_base`; FTC ceiling $300/$600) in core
   `tables.rs`, NOT `TaxTable` (`tax_tables.rs:9-11` convention, Minor 3). Data only; no compute.
6. **KAT-9 (arithmetic + cross-foot):** two `.50` components (271.50 + 499.50) → `round_dollar` each → printed
   272 + 500 = **772**, vs the wrong sum-then-round **771** — proves printed-line rounding + cross-foot (NOT the
   mode; that's task 1). Re-asserted on real 8959 lines at P4/P6.
**Acceptance:** `method.rs` reproduces sampled IRS Tax-Table/TCW cells to the cent (incl. one midpoint-edge cell);
the **mod-25** edge assertion passes for 2017/2024/2025/2026; CI param cross-check green. FROZEN guard green.

## Phase 1 — `ReturnInputs` model + side-table + CLI/TOML + resolver + refuse-guards (spec §4, §4.12)

**Goal:** the offline input surface; nothing computes liability yet.
**New:** `btctax-core/src/tax/return_inputs.rs` (all §4 structs incl. `W2`/`Person`(+blind, DOB, can-be-claimed)/
`ScheduleCInputs`/`ScheduleAInputs`(classified charitable)/`Form1099{Int,Div,G}`/`Payments`/`QbiInputs`/
`CharitableCarryItem`); `btctax-cli/src/return_inputs.rs` (side-table, mirror `tax_profile.rs`);
`resolve_profile(year) -> (TaxProfile, Provenance)`.
**Tasks (test-first):**
1. Structs + serde round-trip tests (`#[serde(default)]` back-compat).
2. Side-table init/get/set/all + `income add-w2/add-1099-int/-div/-g`, `schedule-c set`, `deductions set`,
   `dependents add`, `household set`, `payments set`, `income import <toml>` / `income show --toml` (spec recon-04 §6).
3. `resolve_profile` + `Provenance` **skeleton [I2]:** the `ReturnInputs` arm is **stubbed
   `NotComputable("derivation pending")`** here (no vault can hold `ReturnInputs` yet); the stored-`TaxProfile`
   / pseudo / missing arms + provenance land now — **full precedence + the first-arm KAT move to P2.** Wire the
   single resolver into report/TUI/optimize/what-if/export; `tax-profile set` warn + `--force` when
   `ReturnInputs` exists (D-4).
4. **Refuse-guard table (§4.10)** as `fn screen(&ReturnInputs, &TaxTable) -> Option<Blocker>` [Minor 1: needs
   the year table for the excess-SS MAX + the §1(g) kiddie threshold]; **one KAT per input-screenable row**
   (business-Interest R3-I3; foreign-trust; **box-12 inert-allowlist — KAT-20, code K present ⇒ refuse (audit
   I1)**; box-8/10; foreign-tax>cap; ≥2 SE earners; single-employer-excess-SS; **Form 8615 kiddie-tax — KAT-19,
   claimable dependent + unearned income (Σ int+div+capgain) > $2,600 (audit C1)**; excess-APTC/8962).
   **Compute-dependent rows are owned downstream** (TI≤0 → P3; **Schedule C net<0 → P2, audit I2**).
5. Carryover write-back plumbing (spec §4 R3-M6: **provenance** computed-vs-user; computed overwrites computed
   but **refuses user-entered without `--force`**) — storage only; the R3-M6 KAT lands in P3/P4. SSN `--stdin`
   entry + masked rendering (spec §4.2 security-review item).
**Acceptance:** full CRUD + TOML round-trips; resolver skeleton (non-`ReturnInputs` arms) + provenance tested;
every **input-screenable** refuse row has a red→green KAT; SSN masking verified; FROZEN guard green.

## Phase 2 — `derive_tax_profile` + income→AGI assembly (spec §5 stages 0–2, §4.4/§4.4a)

**Goal:** build AGI from line items (crypto ordinary income included), standard-deduction-basic only here.
**New:** `btctax-core/src/tax/return_1040.rs` (income assembly); `ReturnInputs::derive_tax_profile`.
**Tasks (test-first):**
1. Income lines 1a/2a/2b/3a/3b (KAT: 1a⊃… no double-count; box1a⊇1b strip-once) + Sch D L7 reuse.
2. Sch 1: L1 (attest), L7 (Σ G), **L8v (Σ non-business `crypto_ord`)**, **L3 (Schedule C net = Σ SE-eligible
   business `crypto_ord` − expenses; **refuse if net < 0 — Schedule C loss, audit I2**)**, L18 (Σ INT box2),
   L21 (student-loan worksheet, MFS=$0), L15 (½-SE from Sch C, wired in P4 but derivation stub here) → L10.
   **KAT-15: L8v-vs-L3 partition + cross-foot (L9 carries crypto_ord); KAT: business-income-without-Sch-C ⇒
   fail-loud (R3-M10); KAT: Sch C net<0 ⇒ refuse.**
3. **Two distinct AGI notions [I4 — the frozen seam]:** (a) `return_1040` computes the **absolute, WITH-crypto**
   AGI (real 1040 L11 = L9−L10, crypto included) for the filed return; (b) `derive_tax_profile` populates the
   frozen `TaxProfile` scalars `magi_excluding_crypto` / `ordinary_taxable_income` from **NON-crypto line items
   only** — ledger `crypto_ord`, crypto gains, and the Schedule-C-driven ½-SE stay OUT, because the frozen
   engine ADDS the crypto AGI delta itself (`compute.rs:364-368`; `types.rs:34-38` "EXCLUDING app-computed
   crypto"). **KAT (crypto-income fixture):** the derived-profile delta from `compute.rs` == the delta from an
   *independently hand-built* exclusion profile (forces the exclusion semantics — a same-misreading comparison
   profile must NOT pass).
4. **Schedule B filing trigger + Part III [I1/KAT-18]:** filing = interest>$1,500 OR ord-div>$1,500 OR
   `foreign_accounts==Some(true)` (`foreign_trust==Some(true)` already refuses, P1); drop the unused
   "user-forced" clause (`fr-schb-user-forced`). When filing, Part III 7a/8 tri-state ⇒ fail-loud if `None`.
   **KAT-18** (the $2,000-dividend and the ≤$1,500-with-foreign-account cases).
5. Complete the P1 resolver's `ReturnInputs → derive_tax_profile` arm (stubbed in P1); the full §4.12
   precedence + two-sources-of-truth KAT land here [I2].
**Acceptance:** deep/02 Ex.1 income side to the cent; the **exclusion-semantics** KAT + the with-crypto
`L11 = L9−L10` cross-foot KAT both green; full resolver precedence tested; FROZEN guard green.

## Phase 3 — Deductions: standard (full) + Schedule A + charitable engine (spec §4.6, §5 stage 3)

**Goal:** taxable income (L15) and regular tax (L16).
**New:** `btctax-core/src/tax/charitable.rs` (§170(b) class ceilings + vintage carryover).
**Tasks (test-first):**
1. Standard deduction full (basic + §63(f) aged[DOB]/blind[flag] + dependent floor[derived earned income]).
2. Schedule A: medical 7.5%; **SALT §164(b)(5) either/or (R2-I4)** + the 5a sales-tax election intent
   (checkbox fill in P6) + **fail-loud when `salt_sales_tax_amount`>0 with the election off (R3-M9)**; mortgage
   8a; **charitable via `charitable.rs`** — 6-class ceilings, statutory order, **30%-class two-term cap
   (R2-I1)**, oldest-vintage-first, 5-yr expiry, **G8 aging in std-deduction years**, ledger §170(e) supply by
   holding period. **KAT-17** (same-year ST+LT crypto donation ceiling), **KAT-13** (std-year-between-two-
   itemized carryover).
3. std-vs-itemized `max` + MFS coupling (tri-state; both math and header box); charitable carryover write-back
   with the **R3-M6 precedence KAT** (computed overwrites computed; refuses user-entered without `--force`).
4. L16 = `method.rs::qdcgt_line16` on **with-crypto AGI** (G7). KAT: Schedule A reads with-crypto AGI. (QBI is
   a **0-stub** here — L13=0, L14=L12; the real 8995 lands in P4, spec R3-M7.)
**Acceptance:** charitable worked example (deep/04 §3, $70k LT crypto → $60k + $10k carryover) to the cent;
carryover write-back round-trips across two years; L16 golden vs `method.rs`.

## Phase 4 — Credits + other taxes (spec §4.5/§4.7a/§4.9/§4.11, §5 stages 5–8)

**Goal:** total tax (L24) and payments/refund-owed (L33–L37).
**New:** `btctax-core/src/tax/other_taxes.rs` (absolute 8960 + 8959 Part I/II/V); QBI/8995; FTC; excess-SS.
**Tasks (test-first):**
1. QBI/8995 simplified (REIT box5; TI-before-QBI limit; refuse above threshold / non-REIT QBI) + **REIT/PTP
   carryforward-out write-back** (spec §4.5) with the R3-M6 precedence KAT (Minor 2).
2. §904(j) FTC (§4.7a; ≤$300/$600 passive → Sch 3 L1; refuse above) — **KAT-16**.
3. AMT screen (§4.11 6251 worksheet as refuse-trigger) — **KAT-14**.
4. SE tax (Schedule SE reuse; **§6017 $400 floor**; Sch 2 L4 = ss+medicare only — **KAT-6 unbundle**);
   ½-SE → Sch 1 L15 (completes P2 stub).
5. Absolute **8960** (NII rebuilt from line items incl. crypto lending interest on L7; floors; MAGI=AGI
   fail-closed) + **8959 Part I+II+V** (inner clamp; Part V→25c). **KAT-5** (reduce-to-delta, 4 regimes,
   NII-binding SE) + **KAT-5b** (documented absolute<delta MAGI-binding SE) + **KAT-12** (25c composition).
6. Excess-SS (§4.9 per-employer clamp, per-person, ≥2 employers) — **KAT-11**; Sch 3 L15 → L31; settle L34–L37
   (**L36 apply-to-next pinned 0/blank in v1**, Minor 4 — spec §4.8 L36 gap → FOLLOWUPS).
7. **CTC/ODC conservative omission (§3.4):** L19=0 + loud advisory; **KAT** pinning the line to 0 and the
   advisory present (Minor 7).
8. **Delta-vs-absolute dual reporting (C2 — spec §6):** render the absolute-liability lines and the crypto
   *delta* side-by-side with the §6 labels ("different questions"), document the delta-deduction as approximate,
   never reconcile them to the dollar. **KAT:** presence + labeling on a fixture where
   `absolute_with − absolute_without ≠ delta` (AGI-sensitive deduction).
**Acceptance:** deep/02 Ex.2 ($60k mining) end-to-end other-taxes block to the cent; reduce-to-delta KATs
(KAT-5/5b) pass; the dual-report surface renders + labels correctly; every credit/other-tax has a golden;
refuse rows (QBI/FTC-over-cap/single-employer-SS) KAT'd.

## Phase 5 — LIMITATIONS doc + advisories (spec §9.2)

**Tasks:** author the versioned LIMITATIONS/supported-forms doc (man page + `--help` + shipped file) with the
three aligned lists (omission / refuse / unrepresentable), the FBAR + charitable-donee advisories, DRAFT/
attestation posture. Wire the conservative-omission advisories (CTC/ODC/EIC) into `report`/output.
**Acceptance:** doc builds into man/`--help` (binary-docs infra); advisories surfaced on the relevant fixtures.

## Phase 6 — PDF fillers (spec §7, §3.2)

**Goal:** fill the official TY2024 PDFs from the computed values.
**Tasks (test-first, per form: extract map → fill → geometric read-back → golden SHA):**
1. Extend `schedule_d.rs` to L17–22 — **all four routing paths (R2-I2)**; **KAT-10** (gain/ST-gain-LT-loss/
   loss/zero) + loss-year negative-cell (1040 L7 = −3,000) read-back; extend the read-back oracle to negative
   cells (§3.2) and the **5-way filing-status checkbox group** (§7.4).
2. New maps + fillers: **Schedule C** (Part I income + line 27a/48 + 28; header B/F defaults), **8959, 8960,
   8995** (1-page). Per-(form,year) root FQN + leaf map via a deep/03-**style FRESH extraction** — Schedule C /
   8959 / 8960 / 8995 are NEW (deep/03 holds only the six existing roots; Minor 6); **KAT-7** (cross-year
   `f1_57` collision ⇒ read-back FAILS) + **KAT-8** (filing-status on-state per year).
3. Full `form1040.rs` + Sch 1/2/3/A/B fillers (income/deduction/tax/payment lines) incl. the **Sch A 5a
   sales-tax election checkbox** (`c1_1` iff `salt_use_sales_tax`, R3-M9) and the **KAT-18 Sch B filing / Part-III**
   fill; Schedule SE (existing). DRAFT watermark + attestation gate forced; Sch B overflow (>14/>15 payers)
   reuse 8949 continuation.
**Acceptance:** every in-scope line fills + reads back; golden-PDF SHA per form; mis-mapped cell writes zero
(fail-closed); **form-set-closure KAT** — a non-DRAFT return with any line lacking its backing form is refused
(§7.1, Minor 7); DRAFT/attest gate enforced.

## Phase 7 — End-to-end golden returns + fixtures (spec §10 L2/L3)

**Tasks:** build the synthetic-household golden matrix (single/MFJ; std/itemized; ±QD+LTCG; under/over $100k;
multi-W-2; REIT box5; crypto hobby income; **crypto Schedule-C business + SE**). Diff every 1040/schedule line
vs an independent oracle (hand-worked + `method.rs` cross-check + tenforty/PolicyEngine observe-only). Ingest
**IRS ATS Scenario 2** with a **partial-line diff** (M6 caveat) or a v1-envelope synthetic golden.
**Acceptance:** all goldens green; `export-irs-pdf` produces a complete DRAFT return for each golden household.

---

## Sequencing & risk

- **Critical path:** P0 → P2 → P3 → P4 (compute) then P6 (fillers) then P7 (goldens). P1 unblocks P2; P5 is
  parallelizable after P4. Fillers (P6) need the computed values, so they follow the compute phases.
- **Biggest risk = the frozen-engine seam** (P2): `derive_tax_profile` must produce a `TaxProfile` that drives
  `compute.rs` byte-identically for the crypto path — the FROZEN CI guard + a "derived == hand-entered for an
  equivalent profile" KAT protect it.
- **Second risk = SE/NIIT interaction** (P4): the documented reduce-to-delta inequality (spec §5 tail / KAT-5b)
  must be a *pinned expectation*, not a bug to "fix."
- **Estimation:** ~7 phased cycles; each is spec-scoped and independently reviewable. Ship as phased merges to
  `full-return`, then one whole-diff review before merging to `main` (per workflow), then a version bump.
