# Independent TAX review — Approach B sub-project 1 SPEC, round 2 (post r1-fold)

**Artifact:** `design/conservative-filing-approach-b/SPEC.md` (DRAFT, r1 two-lens fold applied).
**Lens:** US federal tax correctness / completeness / honesty. Adversarial; every load-bearing claim
re-verified against statute/reg/case law AND current source (`conservative.rs`, `project/{resolve,fold,
transition,pools}.rs`, `event.rs`, `void.rs`, `forms.rs`, `tax/{return_1040,charitable,printed,
return_refuse}.rs`, `btctax-cli/src/{cmd/tranche.rs,render.rs,session.rs}`, `btctax-forms/src/form8283.rs`).
**Provenance honored:** `reviews/DESIGN_PROVENANCE.md` + the arch r1 review read; no adjudicated point
re-litigated; no fold found to silently contradict an adjudicated ruling (checked: no-new-identity,
WindowLowClose-only, mandatory-8275, promote-over-disposed=allow+advisory, clamp+evaporation+
documented-unclamped, clean export). Parent-spec guarantees (amended Invariant KAT corners, D-7 re-scope,
D-8 mutual exclusion) checked — no violation.
**Reviewer:** Fable (independent; not the author of the fold). **Date:** 2026-07-21.

**Verdict: NOT green. 0 Critical / 3 Important / 2 Minor / 2 Nit.**

The fold is high quality: all 16 r1 findings are genuinely resolved as filed (§V below, each with the
code/law fact). The three new Importants are residue of the same §170(e)/decomposition hazard C-1 opened —
a second, independent §170(e) emitter in the full-return engine that the BG-D11 fix-site directive misses;
an estimate-money leak through the FIFO fee draw that falsifies the amended invariant; and a
no-profile/uncomputable-year blind spot that silently disables both load-bearing gates (BG-D6/BG-D9).

---

## V — Verified resolved (r1 → fold, each checked against source)

- **C-1 → RESOLVED as filed.** BG-D11 adopts fix direction 1 ("estimate never funds a deduction");
  the ST-donation deduction is limited to the documented component, the gift-`Removal` carryover ruled
  documented-only, the LT path confirmed basis-independent **at the fold site** (`fold.rs` Donate arm:
  LT legs → `leg.fmv_at_transfer`, basis uninvolved), the false `forms.rs::how_acquired_from` "$0"
  doc-sentence is §3 item 8, and §6 pins the ST-donation KAT. §6664(c)(3)'s removal of the
  reasonable-cause defense is correctly the stated motivation. Everything r1-C-1 asked for is present.
  *However*: the sweep r1 itself specified was incomplete — there is a SECOND §170(e) emitter and a
  printed donor-basis column neither r1 nor the fold enumerated → **NEW I-1 / M-1** below. The ruling
  closes the hazard; the fix-site census does not yet deliver the ruling.
- **I-1 → RESOLVED.** BG-D4 now derives the estimate share from the **stored `filed_basis`**
  (`filed_basis × leg.sat / tranche_sat`, keyed via `leg.lot_id.origin_event_id` → the promote set),
  documented = `usd_basis_share − estimate_share`, unclamped; §6 pins the relocated-with-fee-then-
  promoted-below-floor KAT. Verified against `rehome_onto_lot` (`fold.rs`, the TP8(c) merge into
  `lot.usd_basis`): the decomposition no longer mistakes the merged fee for estimate. (The REVERSE fee
  direction — estimate money riding the fee draw OUT of the tranche — is new, → **NEW I-2**.)
- **I-2 → RESOLVED.** BG-D6 defines the undisposed case: Σ of per-year clamped deltas over years with
  disposed tranche legs PLUS an explicit unrealized line ("saving and exposure accrue at disposal; at
  today's price … hypothetical, not a filed figure"), never a bare $0. (Year-set wording ambiguity →
  **NEW M-2**; the no-profile case where NO figure is computable → **NEW I-3**.)
- **I-3 → RESOLVED.** BG-D6 requires the figures re-fold through the CLAMPED promoted path (synthetic
  promote set threaded into the what-if — correctly diagnosed against `overpayment_delta_one`,
  `conservative.rs`, which swaps `usd_cost` post-`resolve` with no clamp); §3 item 2's funnel line must
  quote the clamped delta or state the below-window-low caveat; §6 pins the below-window-low consent KAT.
- **I-4 → RESOLVED.** BG-D9's advisory now triggers on "any year `< current` whose computed `tax_total`
  changes between the pre- and post-promote fold" — verified this catches the default-HIFO retroactivity
  (`applicable_method` → `unwrap_or(LotMethod::Hifo)`, `fold.rs`; `hifo_cmp`'s `usd_basis == ZERO`
  sorts-last special-case, `pools.rs` — a promoted lot exits it and outranks cheaper documented lots).
  Partially-disposed ambiguity resolved; copy conditional ("if Y was already filed") + §6511; §6 KAT
  pins the undisposed-promote HIFO-reorder case in BOTH directions. (Predicate blind spot when
  `tax_total` is `None` in both folds → **NEW I-3**.)
- **M-1 → RESOLVED.** §3 item 5 enumerates all five copy sites; each verified in current source:
  `TRANCHE_IS_FINAL_HINT` ("filed the tranche's $0 basis", `cmd/tranche.rs`), the allocation-guard
  refusal ("($0 EstimatedConservative) is on file", `cmd/tranche.rs::guard_allocation_vs_tranche`),
  the phantom-wallet warning ("still files at $0", `cmd/tranche.rs::declare_tranche`), the
  `SafeHarborUnconservable` blocker detail (`resolve.rs`), the TUI opener (`session.rs`).
- **M-2 → RESOLVED.** BG-D4's formula is now `clamp(net_proceeds_share, $0, estimate_share)` =
  `min(estimate_share, max(net_share, $0))`; both corners (`fee_usd > proceeds` netting, `fold.rs`
  `net = round_cents(proceeds − fee_usd)`; the `make_disposal_legs` last-leg cent-scale negative
  remainder) are named in BG-D4 and pinned in §6. Estimate basis ≥ $0; the cent-negative stays in gain
  attributed to documented rounding — correct.
- **M-3 → RESOLVED.** BG-D10 names the base: "20% ordinary / 40% worst-case **of the resulting
  additional tax** (the underpayment attributable to the misstatement)"; the >$5k threshold correctly
  stated as underpayment-measured (Reg §1.6662-5(b)). §6 copy KAT pins it.
- **M-4 → RESOLVED.** BG-D7 adds the clamped-leg narrative sentence ("limited so as not to report a
  loss from the estimate"); §6 copy KAT.
- **M-5 → RESOLVED.** §6511 (3 yrs from filing / 2 from payment) in the BG-D9 copy; the VOID direction
  explicitly gets the same any-year-diff advisory (amend-to-pay, symmetric).
- **M-6 → RESOLVED.** BG-D5's negative enumeration now includes mining/staking-earning/airdrop/fork and
  closes with "or any acquisition other than purchase" (kills the *expressio unius* misreading); the
  FMV-at-receipt income-basis pointer (Notice 2014-21; Rev. Rul. 2019-24) is correct.
- **M-7 → RESOLVED.** §6's BG-D1 KAT pins term-invariance: `acquired_at`/term byte-identical pre/post
  promote; the date-nudging mutation must go red.
- **N-1 → RESOLVED.** §1 now says December-2017 for the ~$12k window-min and states the Q4-2017 min
  daily close ≈$4.2k (early-Oct 2017 — correct). (A different §1 figure is off → **NEW N-2**.)
- **N-2 → RESOLVED.** The `Acknowledgment` struct records the consent phrase + the shown figures + the
  attested provenance text/version — the §6664(c) artifact is complete.
- **N-3 → RESOLVED.** BG-D3's drift advisory is direction-aware and the direction is right: stored
  floor above the recomputed reference (overstated basis) on an unfiled position → void + re-promote
  hint; filed year stays advisory-only.
- **N-4 → RESOLVED.** §1 limit (ii): an on-chain receipt bounds only `window_end`; `window_start` must
  honestly widen. Correct and correctly framed as a G-4 guardrail.

**Also verified true in the fold (new claims):** `void.rs::is_revocable_payload` lists `DeclareTranche`
unconditionally (the BG-D9-iii premise); `session.rs::safe_harbor_residue` filters `DeclareTranche` but
would keep a promote (§3 item 10); `resolve.rs` `build_op` `_ => Op::Skip` + the `Some(_)` void
classification (§3 item 11); `would_conflict` (`project/mod.rs`, §3 item 14); `Coverage` has no serde
derive (`conservative.rs`, §3 item 15); `render.rs` writes `basis_methodology.txt` unconditionally
(BG-D8's "cannot mirror that gate" premise) and the pseudo export-block string exists (`fold.rs`);
`consume_fifo` is acquisition-date FIFO pinned independent of the elected method (`pools.rs` — the fact
powering NEW I-2).

---

## NEW findings

### I-1 (BG-D11 / §3 item 8 / §6) — The §170(e) ST-donation deduction has a SECOND, independent emitter the fix-site directive misses: the full-return Schedule A engine still funds the deduction from the floor.

**Defect:** §3 item 8 locates the BG-D11 fix at "the §170(e)(1)(A) `min(FMV, leg.basis)` site
(`fold.rs`)" — the `claimed_deduction` computation in the Donate arm — but the **full-return engine
re-derives the same §170(e) reduction directly from removal legs**:
`crypto_charitable_gifts` (`crates/btctax-core/src/tax/return_1040.rs`) does
`Term::ShortTerm => short_basis += leg.fmv_at_transfer.min(leg.basis)` per leg and feeds `apply_170b`
(`tax/charitable.rs`) → Schedule A line 12 → taxable income. It never reads `Removal.claimed_deduction`
(its own doc says the two "reconcile" — a fold-site-only fix breaks that documented invariant too).

**Failure scenario:** the C-1 filer (1 BTC promoted to a ~$60k floor, donated ST) on the full-return
path: the fold's `claimed_deduction` is fixed to $0/documented, the Form 8283 KAT goes green — and the
computed 1040's Schedule A still deducts ~$60k of estimated basis, silently understating tax on the
exact surface where Reg §1.6662-5(g) deems the claim gross and §6664(c) strips the defense. The r1-C-1
harm, shipped through the second emitter.

**Authority/code fact:** §170(e)(1)(A); Reg §1.6662-5(g); §6664(c)(2)–(3);
`return_1040.rs::crypto_charitable_gifts` (the `min(FMV, leg.basis)` at the ST arm);
`tax/return_refuse.rs` has NO tranche refusal — full return + tranche coexist by design (parent D-5:
filing-ready).

**Additional mis-aim, same root:** BG-D11 cites `rehome_onto_removal_leg` as the gift-carryover path,
but that function carries only the FEE cents — the **principal** §1015 carry is `make_removal_legs`
(`fold.rs`, `basis: c.gain_basis`). An implementer aiming at the cited function alone misses the
principal carryover entirely.

**Fix direction (in-spec, one mechanism closes everything):** rule the decomposition at the
**removal-leg builder** — a removal leg drawn from a promoted lot carries the **documented-only** basis
(estimate share, decomposed exactly as BG-D4 from the stored `filed_basis`, evaporates; the pool-side
debit still conserves Σbasis — the §1015 NoGainNoLoss reported≠consumed precedent, `fold.rs`). Then the
fold's `claimed_deduction`, `crypto_charitable_gifts`, the Form 8283 `cost_basis` column, the printed
packet, `removals.csv`, and the gift carryover ALL inherit BG-D11 by construction. At minimum: add
`crypto_charitable_gifts` and `make_removal_legs` to §3's census, and make the §6 ST-donation KAT
assert BOTH surfaces (the fold's `claimed_deduction`/Form 8283 AND the full-return Schedule A line 12 /
computed tax).

### I-2 (BG-D4 / BG-D11 / parent Invariant) — Estimate money escapes through the FIFO fee draw: fee-sats consumed FROM a promoted tranche carry floor-derived basis that re-homes, unkeyed and unclamped, onto surviving lots/legs — falsifying the amended invariant.

**Defect:** `consume_fee` draws fee-sats **FIFO** (`pools.rs::consume_fifo`, acquisition-date order,
independent of the elected method); a promoted tranche is typically the OLDEST lot in its wallet (that
is the feature's audience), so fee draws hit it first. `take_from` gives the fee fragment its pro-rata
`usd_basis` — post-promote, **floor money** — and the `FeeCarry` re-homes it undecomposed onto the
surviving lot (`rehome_onto_lot`), the last disposal leg (`rehome_onto_disposal_leg`), or the last
removal leg (`rehome_onto_removal_leg`). The receiving leg is either not promote-keyed at all (a
documented lot/leg) or the cents land in BG-D4's "documented = `usd_basis_share − estimate_share`"
subtraction residue — either way the estimate escapes the clamp, the deduction ban, and the carryover
ban.

**Failure scenario (the product's own recommended flow):** filer promotes a 1-BTC tranche to a $12k
floor, follows P8 and self-transfers it Exchange → SelfCustody paying a 10,000-sat on-chain fee drawn
from the tranche itself. Relocated lot: 99,990,000 sat, `usd_basis` = $11,998.80 (floor of moved sats)
+ $1.20 (floor of the burned fee-sats, re-homed). Later sale below the window low at net $10,000:
estimate share = 12,000 × 0.9999 = $11,998.80 → clamped to $10,000; "documented" share = $1.20,
UNCLAMPED → the leg files a **$1.20 loss that is 100% estimate money**. Cross-lot variant: a
self-transfer/disposal of documented coins in the same wallet draws its fee from the (older) tranche →
estimate dollars merge into a documented lot's basis, later enlarging a claimed loss or an ST-donation
deduction with no key, no clamp, no 8275.

**Authority/guarantee violated:** BG-1's absolute ("may never manufacture a loss off the estimate");
BG-D11 ("the estimate NEVER funds a deduction or an outbound basis carry"); the amended invariant's
attribution claim ("any negative gain remains attributable solely to documented fee/rounding, never the
estimate") is FALSE in this corner as specced. Cent-to-dollar scale, but the guarantee is claimed "by
construction" and the §6 attribution KATs would be written against an identity this corner breaks.

**Fix direction (in-spec):** decompose the fee draw the same way BG-D4 decomposes legs — the estimate
component of consumed fee-sats (fragment `origin_event_id` ∈ promote set, per-sat floor × fee-sat)
**EVAPORATES** (BG-D4's own evaporation rule; basis forfeiture is always conservative); only documented
fee basis re-homes. State it in BG-D4 (or a BG-D4 sub-bullet), add `consume_fee`/`FeeCarry` to §3's
census, and pin a KAT (tranche-fee-draw self-transfer then below-floor sale files $0 estimate loss).

### I-3 (BG-D6 / BG-D9) — Both load-bearing gates silently no-op when the tax engine cannot compute: no `TaxProfile` (a permitted state) or a Hard-blocked year yields `tax_total = None` in both folds — no consent figures, and NO prior-year advisory while the prior year's 8949 still silently rewrites.

**Defect:** every figure BG-D6 shows and the BG-D9 trigger ("computed `tax_total` changes") both run
through `compute_tax_year`, which is `NotComputable` without a profile (`conservative.rs::tax_total`;
`overpayment_nudge_lines` returns empty on `profile.is_none()`). A profile is optional: forms export
(8949 CSV/PDF) needs none, so the profile-less filer who files from the exports is a real, permitted
flow. For that filer: the consent screen has NO defined behavior (the spec never says what it shows
when nothing is computable), and a HIFO-reordering promote rewrites a prior filed year's 8949 rows with
`None == None` on every year — **no advisory fires**, resurrecting the exact r1-I-4 silent-rewrite harm
for a permitted configuration. Same blind spot for a year Hard-blocked in both folds.

**Failure scenario:** profile-less filer, documented 2016 lot + 2017-window tranche, filed 2025 from
the exported 8949; 2026: promotes the undisposed tranche. Consent screen: unspecified (blank figures —
the "consents to numbers false in both directions" defect r1 blocked, in absent-figure form). The 2025
HIFO draw reorders; no year's `tax_total` "changes" (all `None`); no advisory; the 2026 export
double-counts the documented basis across filed years — silently.

**Fix direction (in-spec):** (a) define the no-profile consent behavior — refuse `promote-tranche`
without a profile, OR an explicit "cannot quantify saving/exposure without a tax profile" consent copy
(recorded in the acknowledgment snapshot as unquantified, so the §6664(c) artifact stays honest);
(b) give the BG-D9 trigger a profile-free fallback: diff each prior year's **8949 rows/legs**
(`form_8949` is profile-free machinery) and fire on any difference, quoting the $Δ only when
computable. Pin both with KATs (a no-profile promote never silently passes the consent screen; the
prior-year advisory fires with no profile).

### M-1 (BG-D11 / §3 census) — The Form 8283 "Donor's cost or adjusted basis" column prints the floor (LT and ST), all the way to the official filled PDF; with a deduction-site-only fix the filed form shows a $0 deduction beside a $60k printed basis.

`Form8283Row.cost_basis = leg.basis` (`crates/btctax-core/src/forms.rs`) → whole-dollar in the printed
packet (`tax/printed.rs::form_8283_printed`) → written into the official AcroForm
(`btctax-forms/src/form8283.rs`, the `row.cost_basis` pushes) → also `removals.csv`'s `basis` column
(`btctax-cli/src/render.rs`). BG-D11's own sentence says the estimate "does not flow to … Form 8283",
but no §3 site delivers that for the column, and §3 item 8's "verified NO-change forms sites" is silent
on it. An ST donation post-fix would file deduction = $0 next to printed basis = floor — an internal
mismatch on a signed form (the same examiner-mismatch class the fold's own M-4 fix treats as a defect);
an LT donation prints an unsubstantiated estimate as "cost" with no 8275 anywhere (a donation-only year
has no promoted 8949 leg, so BG-D8 never gates the packet). Fully resolved by I-1's removal-leg ruling
(the column then prints the documented component); otherwise the spec must decide the column explicitly.
Either way, add the site chain to §3.

### M-2 (BG-D6) — The consent Σ's year set is ambiguous ("years that already have disposed tranche legs"): read pre-promote, it omits the prior year a HIFO-reordering promote changes, understating the quoted saving AND exposure.

The with-scenario is the post-promote fold (a reordered prior year then HAS tranche legs), but "already"
invites the pre-promote reading; under it, the promote that retroactively draws the tranche in 2025
contributes a 2025 delta the consent Σ omits — the acknowledgment then snapshots an understated
exposure, the exact artifact-integrity failure BG-D6 exists to prevent. One-sentence fix: the year set
is the **post-promote projection's** (equivalently: every year the BG-D9 diff identifies, plus the
current year) — the machinery is already shared.

### N-1 (BG-D11) — §6664(c) subsection precision: the gross-case removal of the reasonable-cause defense for charitable-deduction property is §6664(c)(2); (c)(3) is the carve-back that restores it ONLY for the substantial case with a qualified appraisal + good-faith investigation.

The spec (and r1) cite "§6664(c)(3) removes the reasonable-cause defense entirely". Substance and
consequence are unchanged (for the deemed-gross Reg §1.6662-5(g) case there is no defense, period), but
the cite should be corrected before it seeds filer-facing copy: current numbering — (c)(2) exception,
(c)(3) special rule, (c)(4) definitions.

### N-2 (§1) — "a 2013–2017 min daily close is ~$65/BTC" is wrong for a window covering early 2013: BTC's daily close in January 2013 was ≈$13; ~$65 is the post-April-2013-crash (mid-2013) low.

The anti-overstatement point (wide window ⇒ trivial floor) survives — is in fact stronger at $13 — but
this copy seeds product/wizard text (same class as r1 N-1); fix the figure or re-window the example
("mid-2013–2017").

---

## Disposition

Every r1 finding is genuinely resolved; the adjudicated architecture and the fold's new rulings (BG-D11
included) are the right tax law. The three Importants are all completeness residue of the same lesson
the project's own memory codifies (whole-surface sweep on a taxonomy/behavior change): a second §170(e)
emitter (full-return Schedule A), the fee-draw back-channel for estimate basis, and the
uncomputable-year blind spot in the two gates. All three are repairable inside the existing decision
structure — one removal-leg-level ruling (I-1, which also closes M-1), one FeeCarry decomposition
sentence (I-2), and one defined-fallback paragraph (I-3). Re-review after fold.

| Severity | Count |
|---|---|
| Critical | 0 |
| Important | 3 |
| Minor | 2 |
| Nit | 2 |
