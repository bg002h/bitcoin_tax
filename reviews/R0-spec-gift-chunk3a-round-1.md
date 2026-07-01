# R0 architect review — SPEC gift Chunk 3a (§2505 advisory-level lifetime exemption) — Round 1

- **Artifact:** `design/SPEC_gift_chunk3a_lifetime_exemption.md`
- **Baseline:** `origin/main` @ `6a694fa` (verified: `git rev-parse HEAD` = `6a694fa8c311320ea19f810c75a29a9b7743dcc3`).
- **Reviewer role:** independent architect (author ≠ reviewer).
- **Gate:** R0 spec gate — 0 Critical / 0 Important required before implementation.
- **Verdict:** **NOT GREEN — 0 Critical, 2 Important, 4 Minor, 2 Nit.** Two blocking Importants; both fixed by spec-text additions (no design rework).

---

## Independent web / primary-source verification

### 1. Basic exclusion amount TY2025 — CONFIRMED $13,990,000 (§2.41)
Pulled the **primary source** (Rev. Proc. 2024-40 PDF, `irs.gov/pub/irs-drop/rp-24-40.pdf`), extracted verbatim:

> **.41 Unified Credit Against Estate Tax.** For an estate of any decedent dying in calendar year 2025, the **basic exclusion amount is $13,990,000** for determining the amount of the unified credit against estate tax under § 2010.

> **.43 Annual Exclusion for Gifts.** … For calendar year 2025, the first **$19,000** of gifts to any person … under § 2503 …

So the spec's exact figure (**$13,990,000**) and exact citation (**Rev. Proc. 2024-40 §2.41**, §2010(c)(3)) are **both correct against the primary source**. The existing `gift_annual_exclusion` ($19,000, §2.43) is likewise confirmed. Cross-checked against IRS/KPMG/Barley Snyder secondary sources — all agree ($13.99M for 2025; $13.61M for 2024, matching the spec's TY2024 note). **Year-indexed → belongs in `TaxTable`, not a statutory constant.** Correct: it moves annually (2024 $13.61M → 2025 $13.99M → 2026 rises again under OBBBA), exactly the `gift_annual_exclusion`/`ss_wage_base` precedent, and unlike the truly-fixed §1211/§1411/§170(f)(11)(C) statutory constants in `tables.rs`.

Nuance (does not change the verdict): §2.41 is phrased in **estate**-tax terms (decedent dying in 2025). §2505(a)(1) imports the **same** §2010(c) applicable/basic exclusion amount for the **gift** credit ("the applicable credit amount … which would apply if the donor died as of the end of the calendar year"). So sourcing the gift-tax BEA from §2.41 is legally sound.

### 2. §2505 / §2502 cumulative "no tax due until exhausted" model — CONFIRMED
Verified against 26 U.S.C. §2505 (Cornell LII / House OLRC) and §2502:
- §2505 gives a **unified credit** equal to the applicable credit amount under §2010(c) (the exemption-equivalent = the basic exclusion amount), **reduced by credit allowed for all preceding calendar periods** → the exclusion is consumed **cumulatively** across the donor's lifetime.
- §2502 computes gift tax on **cumulative** taxable gifts (current-period taxable gifts stacked on all prior-period taxable gifts) at the rate schedule, less the credit.
- Net effect: **no gift tax is actually DUE until cumulative lifetime taxable gifts EXCEED the basic exclusion amount**; past that point the excess is taxed (40% top rate — the BEA sits far above the $1M point where the 40% bracket begins).

The spec's model — `used = cumulative taxable gifts`, `remaining = BEA − cumulative`, "no tax due until exhausted; then tax may be due on the excess" — is an **accurate** advisory rendering of §2505/§2502.

### 3. Advisory-scope soundness — DEFENSIBLE (honest, not misleading), with one wording nit
Reporting consumption/remaining + "no tax due until exhausted" **without** computing the §2502 rate-schedule liability is honest **because below the BEA the tax genuinely DUE is $0** (the unified credit fully offsets). Above the BEA the spec does **not** assert a number — it says gift tax "may be due **on** $X; consult a professional," framing $X as the taxable **base**, not the tax. That is the correct honesty boundary (the actual tax on $41,000 excess ≈ $16,400 at 40%, which the advisory deliberately does not claim). See **N1** for a wording tightening so "on $X" is not misread as "$X of tax."

---

## Findings

### Critical — none.

### Important

**I1 — Stale, self-contradicting caveat not updated by the spec. (blocking)**
`render.rs:1332-1334` hardcodes the Chunk-2 caveat footer:
> "Caveats: §2513 gift-splitting (MFJ) not modeled; future-interest gifts … not detectable; **§2505 lifetime exemption is a later chunk (Chunk 3).**"
After Chunk 3a ships, the same advisory will **emit a §2505 consumption block AND say §2505 is "a later chunk (Chunk 3)"** — a direct self-contradiction. Grep confirms **no test locks this caveat string** (`grep "later chunk"` → only the source literal), so it will **ship silently** as contradictory output; the green build will NOT catch it. The spec (D3/Task 1) never mentions editing this line.
**Fix:** Spec Task 1 must mandate replacing the "…§2505 lifetime exemption is a later chunk (Chunk 3)" clause with the real §2505 caveats (single-filer; no §2513 gift-splitting; no §2010(c)(4) portability/DSUE; prior cumulative user-supplied, default $0). Add a KAT asserting the "later chunk" phrase is **absent**.

**I2 — Unlabeled-gift taxable is omitted from the §2505 cumulative, and the omission is undisclosed in the block. (blocking)**
`current_year_taxable` = the Chunk-2 `total_taxable`, which sums **only labeled-donee over-exclusion taxable** (`render.rs:1264-1288`); the unlabeled bucket is excluded from that total by construction. The §2505 block therefore **understates consumption** whenever unlabeled gifts exist → **overstates `remaining`** → the headline "no gift tax is DUE until exhausted / $Y remaining" becomes **falsely reassuring** in exactly the direction that under-warns the user. The spec (D5/Task 2) flags the question but resolves it only as "consistent with Chunk 2's total" — consistency of the *base* does not discharge the *disclosure* duty on a reassurance claim. The existing unlabeled NOTE tells the user to label, but nowhere states that the §2505 consumption figure excludes those gifts.
**Fix:** When `unlabeled_count > 0`, the §2505 block must disclose it reflects **labeled-donee taxable only** (e.g., "This consumption excludes N unlabeled gift(s) shown above — label them for an accurate lifetime figure"). Add a KAT: labeled-over-exclusion + unlabeled gift → §2505 block present **and** the exclusion-disclosure line present.

### Minor

**M1 — Two fan-outs; spec should name both (green build proves completeness, per P2-D precedent).**
- *TaxTable literal sites (new non-`Default` field):* `grep "ss_wage_base:"` (the exact per-literal proxy) = **14 hits = 1 struct def + 13 literal constructions**: `tables.rs:249` (synthetic_table), `tax_tables.rs:188` (ty2025 → set `$13,990,000`), `render.rs:1733` (CLI test helper), and tests `optimize_wash_sale:74`, `optimize_mode1:69`, `optimize_mode2:82`, `optimize_score:73`, `optimize_compliance:76`, `tax_compute:68` & `:128`, `method_election:471`, `kat_tax:1936`, `optimize_accept:125`. (Note `se.rs:165 fn tbl() -> TaxTable` is a *signature*, not a literal — it does not set `ss_wage_base`, so it is not a site; the spec's "grep `TaxTable {`" over-counts it. Use `ss_wage_base:` as the true proxy.)
- *`render_gift_advisory` signature fan-out:* adding `prior_taxable_gifts: Usd` breaks all **9 existing test call sites** in `render.rs` (compiler-caught; they use substring `.contains`, so the *added* §2505 content does not break their assertions — only the arg-count does). Spec should note both fan-outs explicitly.

**M2 — Boundary KAT missing.** Add cumulative **exactly == $13,990,000** → `remaining $0`, **NOT** "EXCEEDED" (guards `>` vs `≥` in the exceeded branch). The three provided arithmetic KATs re-derive correctly (see below) but none pins the boundary.

**M3 — `--prior-taxable-gifts` flag hygiene under-specified.** Spec should state: (i) the flag is consumed **only on the `--tax-year` path** (no-op/ignored otherwise — it slots into the `Report { year, tax_year }` variant at `main.rs:383`, whose report body is guarded by `if let Some(y) = tax_year`); (ii) **non-negative validation** with a clear rejection message; (iii) parsed as **exact `Usd`/Decimal** like other money inputs (never float, NFR5); (iv) the disclosure should say "cumulative prior-period **taxable** gifts (from your prior Forms 709), not gross gifts" to pre-empt the common gross-vs-taxable user error.

**M4 — Uncovered edge: prior>0 & current-year-taxable=0.** A gift present but under the annual exclusion (so `any_gift` is true and the advisory is not `None`) with `--prior-taxable-gifts > 0` yields `cumulative_taxable > 0` → the §2505 block renders **from prior alone**. This is reasonable behavior, but it is neither a KAT nor a stated decision. Add a KAT or a one-line "intended" note.

### Nit

**N1 — "gift tax may be due on ${excess}" wording.** Tighten to make `$X` unambiguously the **base**, not the tax: e.g., "gift tax may be due on the **$X of cumulative gifts exceeding the exclusion** (liability not computed — advisory only)."

**N2 — Spec prose typo.** Line 66: "($ {lifetime_remaining} remaining)" has a stray space after `$`; cosmetic (the real format is fixed by the exact-string KAT).

---

## Item-by-item evaluation (task checklist)

**4. §2505 consumption arithmetic (D3) — all three KATs re-derived, EXACT:**
- Alice $100k, prior $0: current taxable `100,000 − 19,000 = 81,000`; cumulative `0 + 81,000 = 81,000`; remaining `13,990,000 − 81,000 = 13,909,000`. ✔ (matches spec)
- Alice $100k, prior $13,900,000: cumulative `13,900,000 + 81,000 = 13,981,000`; remaining `13,990,000 − 13,981,000 = 9,000`; ≤ BEA → no tax due. ✔
- Alice $100k, prior $13,950,000: cumulative `13,950,000 + 81,000 = 14,031,000` > `13,990,000` → EXCEEDED by `14,031,000 − 13,990,000 = 41,000`. ✔
The `remaining = max(0, …)` floor and the `cumulative > exclusion` exceeded predicate are correct. (Boundary case → M2.)

**5. `current_year_taxable` basis — labeled-donee-only confirmed** (`render.rs:1264-1288`); no double-count (each labeled donee's taxable added once, only when `total > excl`). The omission risk is the unlabeled bucket → **I2**.

**6. `--prior-taxable-gifts` mechanism — SOUND for advisory scope; rate: keep as a CLI flag, do NOT persist.** A stateless, disclosed, default-$0 flag matches the existing standalone-advisory pattern (render-time, no vault write). Persisting to the profile would add wrong-year-attribution and staleness risk for no advisory benefit; the value is inherently a user-maintained external running total (the prior Form 709 line). Hygiene gaps → **M3**.

**7. `cumulative_taxable > 0` gate — correct** (no taxable gifts → nothing to consume → no block; consistent with the Chunk-2 "Total taxable gifts: $0.00" no-filing case). **Chunk-2 safety branches preserved & correctly located:** `any_gift → None` (`render.rs:1216`) and gifts-but-no-table → note (`:1221-1235`) are untouched; the §2505 block must sit inside the `Some(t)` path (it needs `t.gift_lifetime_exclusion`) and after `total_taxable` (declared at `:1264`, in scope). ✔ (Edge → M4.)

**8. Standalone (no engine B change) — CONFIRMED.** `compute_tax_year` reads `TaxTable` only via `ordinary_for`/`ltcg_for`; `compute_se_tax` reads `ss_wage_base`. Neither reads the new field, so adding `gift_lifetime_exclusion` is inert to engine B; the gift advisory is assembled render-side in `report_tax_year` (`cmd/tax.rs:64-68`) and never enters `state.advisory`/the blocker set. **No §2502 rate-schedule computation, no leak.** The spec's "assert a golden unmoved" is the right guard (note: `TaxTable` derives `PartialEq/Eq`, so whole-table equality changes — but no engine-B KAT compares whole tables; each constructs its own).

**9. Scope / right-sizing / TDD — appropriate.** Advisory-level, single-filer, additive MINOR (new field + new flag + extended advisory) is correctly right-sized; deferrals (§2502 liability, §2513 splitting, DSUE/portability, auto prior-year tracking, Chunk 3b Section-B, 2026 tables) are explicit and land in FOLLOWUPS. The KAT set (under / accumulate / exceeds / no-taxable-no-block / default-$0) is genuine and branch-covering; gaps are additive, not structural (M2 boundary, M4 prior-only, I2 unlabeled-disclosure KAT).

---

## Required to reach GREEN (Round 2)
1. **I1** — mandate replacing the stale "§2505 … later chunk (Chunk 3)" caveat; add an absence KAT.
2. **I2** — mandate an unlabeled-exclusion disclosure line in the §2505 block when `unlabeled_count > 0`; add a mixed labeled-over + unlabeled KAT asserting it.

Recommended (fold if cheap): M1 (name both fan-outs; use `ss_wage_base:` as the literal proxy — 13 sites), M2 (boundary KAT), M3 (flag hygiene), M4 (prior-only edge), N1 (excess-base wording), N2 (typo).

**Nothing in the legal core is wrong:** $13,990,000 / §2.41 / §2010(c)(3), the §2505/§2502 cumulative "no-tax-due-until-exhausted" model, the year-indexed placement, the consumption arithmetic, and the standalone (no-engine-B) posture are all verified correct. The two blockers are output-integrity/disclosure gaps, not model errors.

---

# Round 2 — re-review

- **Artifact (revised):** `design/SPEC_gift_chunk3a_lifetime_exemption.md`.
- **Baseline re-verified:** `git rev-parse HEAD` = `6a694fa8c311320ea19f810c75a29a9b7743dcc3` (matches spec's stated baseline). Citations re-checked against live source this round (below).
- **Scope:** confirm the R1 folds only. Legal core (BEA $13,990,000, §2.41/§2010(c)(3), the §2505 cumulative model, the arithmetic) was web-confirmed in R1 and is **not** re-litigated.
- **Verdict:** **GREEN — 0 Critical / 0 Important.** I1 + I2 closed; M1–M4 folded and sound; no new C/I introduced. Residual: 2 non-blocking Nits (N1, N2, both carried unaddressed) + 2 micro-observations below.

## Live-source re-verification (this round)
- **I1 target still present:** `render.rs:1334` = `later chunk (Chunk 3).` inside the caveat footer (`:1333` `§2505 lifetime exemption is a \`). `grep -rn "later chunk" crates/` returns **only** that one source literal — no test pins it → it *would* ship silently as a self-contradiction absent the fold. Confirms the I1 hazard was real and the absence-KAT is the correct catch.
- **`ss_wage_base:` proxy = 14 hits** (1 struct def + **13** literal constructions) → the spec's "~13 literal sites" fan-out count is exact; `TaxTable {` over-count (se.rs signature) correctly avoided.
- **I2 basis still holds:** `total_taxable` (`render.rs:1264`) accumulates **labeled** donees only; `unlabeled_count`/`unlabeled_total` are already tracked (`:1242-1243`, `:1256-1257`) → the disclosure line's `N` and `$X` are computable from in-scope values, and a `mixed_labeled_over_and_unlabeled_shows_both` KAT already exists (`:1916`) as the pattern for the new mixed I2 KAT.

## Fold-by-fold

1. **I1 — CLOSED.** D3 bullet `[R0-I1]` mandates *removing* the stale "§2505 … later chunk (Chunk 3)" clause at `render.rs:~1332-1334`, and Task 1 adds an **absence KAT** asserting the "§2505 … later chunk (Chunk 3)" string no longer appears. Both halves present (mandate + regression lock). The removal is correctly *targeted* — only the §2505 "later chunk" clause is struck; the §2513 / future-interest caveats in the same footer are untouched (the absence KAT keys on "later chunk" only), so no over-removal. **No self-contradiction can ship. Adequate.**

2. **I2 — CLOSED.** D3 bullet `[R0-I2]` requires, `when any unlabeled gifts exist`, a disclosure line stating the §2505 consumption reflects **LABELED-donee taxable only**, names `N` unlabeled gift(s) totalling `$X`, and states "consumption may be understated / remaining overstated" — i.e., it corrects the *under-warning* direction R1 flagged. Task 1 adds the mixed KAT (labeled $81k taxable + unlabeled $50k → block shows used $81,000 **and** the omission line). The `$X` is the unlabeled **gross** (the only honest figure available, since unlabeled gifts can't be per-donee-exclusion-reduced); the wording claims it's "NOT included," not that it's taxable, so no new mis-statement. The block-render precondition (`cumulative_taxable > 0`) means the disclosure only fires where a reassurance is actually made — correct scoping. **The silent under-warning is cured.**

3. **Minors — all four present and sound.**
   - **M1** — Task 1 names *both* fan-outs: (a) ~13 `TaxTable { .. }` literals via `ss_wage_base:` proxy (verified = 13), with the explicit "NOT `TaxTable {` (se.rs false-positive)" note; (b) the 9 `render_gift_advisory` test call sites growing by `prior_taxable_gifts`. "Green build proves completeness." Sound. (See micro-obs O1 on the call-site count.)
   - **M2** — Exact-boundary KAT: cumulative **== $13,990,000 → remaining $0, NOT exceeded**; explicitly pins `>` vs `>=` on the exceeded branch. Matches D3's strict-`>` exceeded predicate. Sound.
   - **M3** — Flag hygiene: report-`--tax-year`-path only; exact `Usd`/Decimal (no float); reject negative as an error (not silent clamp); help text "cumulative prior-year TAXABLE gifts (post-annual-exclusion Form 709 amounts), not gross gifts." All four R1 sub-points covered. Sound.
   - **M4** — Prior-only edge KAT: `--prior-taxable-gifts 5,000,000` with all current-year donees under the annual exclusion (current taxable $0) → §2505 block **still** shows (cumulative $5M > 0 from prior). Sound; matches the `cumulative_taxable > 0` gate.

4. **No new Critical/Important.** The folds are additive spec text (one removal mandate + disclosure line + KATs + hygiene). The §2505 arithmetic, the `cumulative_taxable > 0` emit gate, the strict-`>` exceeded predicate, the Chunk-2 safety branches (`any_gift → None`; gifts-but-no-table → note), and the STANDALONE (no engine B / no §2502 rate-schedule) posture are all intact and internally consistent. Right-sized (additive MINOR), TDD-complete (every branch has a KAT, incl. the two new disclosure KATs), no engine B introduced. No self-contradiction ships.

## Residual (non-blocking)
- **N1 (Nit, unaddressed):** D3's "gift tax may be due on ${excess}" was *not* tightened to frame `$X` as the taxable **base** ("liability not computed — advisory only"). Still defensible (R1 §3 honesty analysis holds), but the tightening was recommended. Carry to implementation wording / FOLLOWUPS.
- **N2 (Nit, unaddressed):** Spec line 66 still shows the cosmetic "($ {lifetime_remaining} remaining)" stray-space-after-`$` in the illustrative string; the real output format is pinned by the exact-string KATs, so this is spec-prose only.
- **O1 (micro-observation, not a finding):** M1(b) says "the **9** `render_gift_advisory` CALL sites." Live grep shows **9 test call sites + 1 production call site** (`cmd/tax.rs:68`) whose arg lists grow. The production site is the intended *threading target* (listed separately in Task 1 Files / D2), and the green build catches any miss regardless, so the "9" faithfully mirrors R1's own "9 test call sites" framing and cannot cause an escaped defect. No action required.
- **O2 (micro-observation):** The "no §2513 gift-splitting" caveat now appears both in the new §2505 block (D3 caveats) and in the pre-existing footer. Mild redundancy with different framing (exclusion-doubling vs. general advisory); harmless. Optional dedupe at implementation.

**Bottom line: I1 + I2 closed, M1–M4 folded, 0 new Critical/Important → the spec is R0 GREEN and ready to implement.** Two Nits (N1, N2) may be swept during implementation but do not gate.
