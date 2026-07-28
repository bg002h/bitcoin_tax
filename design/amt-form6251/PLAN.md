# Form 6251 (AMT) — staged implementation plan

**Status:** DRAFT, pre-review. Not build-ready until it passes the §2 independent review loop to
0 Critical / 0 Important, per `STANDARD_WORKFLOW.md`.

**Goal (one sentence):** stop refusing returns over the AMT screen — first by computing Form 6251
internally and proceeding when AMT is $0 (**Tier 1**), then by filling and attaching the real form when
AMT is genuinely owed (**Tier 2**).

**Base:** `main` after `fix/amt-screen-line2` merges. **Lineage:** `FOLLOWUPS.md` §G-4.

---

## 1. Why, and why two tiers

btctax v1 does not compute AMT. It runs the official 2024 *"Worksheet To See if You Should Fill in Form
6251"* and, when that worksheet says a 6251 is required, **refuses the entire return and writes no forms
at all**. The worksheet answers *"must you fill in Form 6251?"*, never *"do you owe AMT?"* — so every
filer above a low threshold is turned away regardless of whether any AMT exists.

Two distinct populations sit behind that one refusal, and they need different work:

| | Who | What they need | Printed output |
|---|---|---|---|
| **Tier 1** | AMT computes to **$0** | Stop refusing | **Byte-identical to today** — Sch 2 L2→L3→1040 L17 are already $0 |
| **Tier 2** | AMT is genuinely **owed** | A filled, attached Form 6251 | New form in the packet; Sch 2 L2 → L3 → 1040 L17 |

**Tier 1 is cheap precisely because Form 6251 need not be attached when AMT is $0** — the "Who Must
File" test is not met. No PDF asset, no AcroForm map, no emitter. The change is a computation plus a
refusal-condition edit.

**Tier 2 is not optional, and Tier 1 must not be mistaken for closing G-4.** Mapping the
(wages × gain × donation) space produced this rule:

> AMT is owed when the exemption is **fully phased out** (AMTI ≥ $1,751,900 MFJ) **and** ordinary
> taxable income is below **$769,139**.

The gain phases out the exemption; the wages decide the outcome. Below the crossover the graduated
regular brackets are cheaper than AMT's flat 26/28%, so TMT wins. **A salaried engineer who sells a large
Bitcoin position is squarely inside that region** — at $250,000 of wages and a $2M gain the AMT is about
$28,000. That is btctax's archetypal user, and Tier 1 alone still refuses them.

Exposure is bounded: once the exemption is gone, further gain cancels (§55(b)(3) taxes it at 20% in both
systems), so AMT plateaus and peaks near **$24,615** at ~$384,000 of ordinary taxable income.

---

## 2. Scope boundary

**In scope:** Form 6251 Parts I–III for the inputs v1 already accepts; the refusal split; Schedule 2
line 2 plumbing (Tier 2); the PDF asset, map and emitter (Tier 2).

**Explicitly OUT of scope** — each is either refused upstream today or an input v1 never captures, and
each must **stay** refused/absent so the AMTI derivation in §3.1 remains exhaustive:
§57(a)(5) private-activity-bond interest (refused via 1099-INT box 9 / 1099-DIV box 13) · §56(b)(3) ISO
exercise · §57(a)(7) §1202 exclusion · §163(d)/§4952 investment interest · §56(a)(1) post-1986
depreciation · NOL/ATNOL · estate & trust K-1 adjustments · §56(a)(6) disposition basis differences ·
§57(a)(1) depletion · §57(a)(2) IDC · §56(a)(3) long-term contracts · pre-1987 installment sales.

**Form 8801 (prior-year minimum tax credit) is out of scope, and §3.4 argues no obligation to it
arises.** That argument is load-bearing and the review must confirm it.

---

## 3. The computation

### 3.1 AMTI — already exact

After `fix/amt-screen-line2`, worksheet line 3 **is** AMTI for every input v1 accepts:

```
AMTI = taxable_income_L15 + amt_worksheet_line2(itemized?, standard_deduction, schedule_a_line7)
```

`amt_worksheet_line2` (`tax/amt.rs`) returns Schedule A **line 7** (capped SALT, §56(b)(1)(A)(ii)) for an
itemizer, or the whole standard deduction (§56(b)(1)(E)) otherwise. Those are the only two §56(b)(1)
add-backs reachable in v1; §2's exclusion list is why. The §199A deduction is **allowed** for AMT
(§199A(f)(2)), so it stays subtracted.

### 3.2 Exemption and taxable excess

`AmtParams` (`tax/tables.rs`) already carries, per year and per status, `exemption_*`, `phaseout_start_*`
and `breakpoint_28pct*`. The phaseout arithmetic already exists in `amt_should_file_6251` and moves to the
new module:

```
exemption      = max(0, base_exemption − 0.25 × max(0, AMTI − phaseout_start))
taxable_excess = AMTI − exemption                                   // Form 6251 line 6
```

### 3.3 Part III — ★ THE ONE GENUINELY UNRESOLVED PIECE

Part III taxes the capital gain at §1(h) rates and the remainder at 26/28%. The unresolved question is
**where the gain's rate bands are positioned**.

During the analysis that produced this plan I computed two different answers for the same taxpayer
($1M wages / $10M gain / $1M donation): **$75,812.50** stacking the gain at its *regular-return* ordinary
position, and **$55,897.50** stacking it on the *AMT* ordinary slice. That is a $19,915 spread on one
return. Reading the 2024 form, Part III line 20 pulls **"the amount from line 5 of the Qualified Dividends
and Capital Gain Tax Worksheet"** — a figure from the **regular** computation — which indicates the bands
are positioned by the regular bottom, making $75,812.50 the correct one. **This is stated from a reading
of the form layout, not from a verified implementation, and Task 1 must settle it against the actual
2024 Form 6251 and its instructions before any other task proceeds.**

Consequence if we get it wrong: TMT is misstated by up to five figures, which flips the AMT decision for
anyone near the crossover — and §1's rule says a large population sits near it.

An earlier idea of using the regular-position stack as a cheap *upper bound* (clear the bound ⇒ AMT is
certainly $0, skip exact Part III) **must not be adopted**: the bound is only valid while the add-back is
smaller than the exemption, and it fails exactly where the margin is thinnest — at $1M wages / $10M gain
the exemption is $0 while the add-back is $29,200. Compute Part III exactly.

The §1(h) primitive already exists: `compute.rs:57 preferential_tax(bp, bottom, pref) -> PrefSplit`.

### 3.4 Why no Form 8801 obligation arises

Paying AMT normally creates a §53 minimum tax credit carryforward. It does **not** here: §53(d)(1)(B)
excludes AMT attributable to **exclusion** preferences from the credit, and v1's only AMT adjustment — the
§56(b)(1) taxes / standard-deduction add-back — is an exclusion item. Deferral items (ISO, depreciation,
§56(a)(6)) are all in §2's out-of-scope list. So AMT computed by btctax generates a **$0** credit and no
Form 8801 is ever required. **Review must confirm this**; if it is wrong, Tier 2 silently creates a
next-year obligation btctax cannot discharge, and Tier 2 must then refuse instead of filing.

---

## 4. File map

| File | Tier | Change |
|---|---|---|
| `crates/btctax-core/src/tax/form6251.rs` | 1 | **new** — the computation (§3) |
| `crates/btctax-core/src/tax/amt.rs` | 1 | keep the screen as a cheap pre-filter; `amt_worksheet_line2` is reused verbatim |
| `crates/btctax-core/src/tax/return_refuse.rs` | 1 | split `AmtScreenTriggered` → `AmtOwed` |
| `crates/btctax-core/src/tax/return_1040.rs` | 1 | `screen_absolute` calls the computation; `AbsoluteReturn` gains `amt: Amt6251` |
| `crates/btctax-core/src/tax/printed.rs` | 2 | `Schedule2Lines.line2`, `line3`; `Form6251Lines` |
| `crates/btctax-forms/forms/2024/f6251.pdf` + `.map.toml` | 2 | **new** asset + AcroForm map |
| `crates/btctax-forms/src/form6251.rs` | 2 | **new** emitter |
| `crates/btctax-cli/src/cmd/admin.rs`, `cli.rs` | 2 | packet includes `form_6251.pdf`; `--forms form6251` |
| `crates/btctax-oracle-harness/src/main.rs` | 1 | stop returning `None` on AMT (`main.rs:705`) — expands the sweep domain |
| `docs/man/*`, `docs/examples/examples.md` | 2 | regenerate |

---

## 5. Global constraints

- **Gate:** `make check` **and** `cargo fmt --all -- --check`, both, from the first commit. Green =
  suite passes **and** 0 Critical / 0 Important.
- **Fail-closed is preserved at every step.** No task may make a return computable that was previously
  refused *unless* the AMT for it is proven $0 or filed. When in doubt, refuse.
- **Never understate.** If Part III is uncertain for an input, refuse rather than guess low.
- Whole-dollar rounding per SPEC §3.1; the printed chain rounds at the line and re-adds rounded lines.
- No new external dependency. No network.
- Per-year figures come from `AmtParams`; **no literal AMT constant may appear outside the tax tables.**

---

## 6. Tier 1 — tasks

### T1 — Part III, settled against the form (BLOCKING; do this first)

- [ ] Read the 2024 Form 6251 and i6251 Part III lines 12–40. Write `design/amt-form6251/PART_III.md`
      recording, line by line, which figures come from the **regular** QDCG/Schedule-D worksheet and which
      from the AMT base. Resolve the $75,812.50 vs $55,897.50 question in §3.3 with a citation.
- [ ] Encode the resolution as three KATs against the worked vectors in §8 before writing any code, and
      watch them fail.

**Deliverable:** the ambiguity is closed in writing, with a failing test per band.

### T2 — `form6251.rs`: the computation

- [ ] `pub struct Amt6251 { amti, exemption, taxable_excess, tentative_minimum_tax, amt: Usd }`.
- [ ] `pub fn compute_6251(status, taxable_income, line2_addback, net_ltcg, qual_div, regular_tax,
      params) -> Amt6251`, composing §3.1–3.3 and reusing `preferential_tax`.
- [ ] `amt = max(0, TMT − regular_tax)`.
- [ ] KATs: the §8 vectors, plus the exemption-phaseout boundary ($1,218,700 and $1,751,900 MFJ) and the
      26/28 breakpoint ($232,600).

### T3 — refusal split

- [ ] `RefuseReason::AmtOwed` alongside the existing screen reason; message names the computed dollar
      amount and says v1 cannot yet fill Form 6251.
- [ ] `screen_absolute`: cheap screen clears ⇒ AMT $0 (unchanged). Screen trips ⇒ compute; `amt == 0` ⇒
      **proceed**; `amt > 0` ⇒ refuse with `AmtOwed`.
- [ ] KAT: the §8 zero-AMT vector now **computes** where it previously refused; the §8 AMT-owed vector
      refuses with `AmtOwed`, not the old reason.
- [ ] ★ Mutation: revert `screen_absolute` to the blanket refusal — the first KAT must red. (The
      `fix/amt-screen-line2` experience: two pre-existing AMT tests both used the standard deduction and
      a caller-level revert passed the whole suite.)

### T4 — printed-output invariance

- [ ] Assert that for a zero-AMT return the printed 1040/Schedule 2 are **identical** to a
      hand-constructed expected packet — L17 = 0, Sch 2 L3 = 0, no Form 6251 in the packet.
- [ ] Regenerate `docs/examples/examples.md` and the TUI goldens; a zero-AMT journey must show no diff
      beyond the newly-computable return itself.

### T5 — oracle domain

- [ ] Remove the `return None` at `oracle-harness/src/main.rs:705` for the AMT case; let AMT-screened
      returns into the differential sweep.
- [ ] Confirm the sweep still reconciles; record how many previously-skipped households it now covers.

**Tier 1 gate:** full suite green, 0C/0I, and a zero-AMT return exports a complete packet.

---

## 7. Tier 2 — tasks

### T6 — the asset and its map

- [ ] Bundle the official IRS `f6251.pdf` (2024 revision) and write `f6251.map.toml`, following the
      `f8275`/`f8949` precedent. Verify every mapped field exists in the AcroForm (the census test).
- [ ] `Form6251Map::for_year` — note 6251 is **tax-year**-versioned, unlike 8275.

### T7 — the emitter

- [ ] `btctax-forms/src/form6251.rs`, filling Parts I–III from `Amt6251` + `ScheduleAParts`.
- [ ] Read-back verification via `verify_flat`, per the existing forms convention.
- [ ] Golden-packet KAT: byte-reproducible fill for the §8 AMT-owed vector.

### T8 — Schedule 2 and the 1040

- [ ] `Schedule2Lines.line2` = AMT, `line3` = Part I total → 1040 **L17**; L18 = L16 + L17.
- [ ] KAT: 1040 L24 total tax now includes AMT; the §8 AMT-owed vector's balance due matches §8.

### T9 — packet and CLI

- [ ] `export-irs-pdf` emits `form_6251.pdf` when `amt > 0`; `--forms form6251` accepted; the form is
      **skipped** when AMT is $0 (Who Must File).
- [ ] Remove `RefuseReason::AmtOwed`. Regenerate man pages and examples.
- [ ] Update `btctax limitations` — the "AMT screen trips" bullet is retired.

**Tier 2 gate:** full suite green, 0C/0I, the §8 AMT-owed vector exports a packet whose Form 6251 and
1040 L17 agree to the dollar.

---

## 8. Worked vectors (derived during the G-4 analysis; MFJ, TY2024)

Use verbatim as KATs. Each was computed independently of btctax.

| # | Wages | LTCG | Donation | Deduction | Taxable income | Regular tax | TMT | **AMT** |
|---|---:|---:|---:|---|---:|---:|---:|---:|
| V1 | 1,000,000 | 500,000 | 85,000 | itemized | 1,415,000 | 364,675.50 | 327,965.00 | **0** |
| V2 | 1,000,000 | 500,000 | 750,000 | itemized | 750,000 | 129,397.50 | see T1 | **0** |
| V3 | 1,000,000 | 10,000,000 | 0 | standard | 10,970,800 | 2,285,321.50 | 2,275,348.00 | **0** (margin 9,973.50) |
| V4 | 700,000 | 10,000,000 | 0 | standard | 10,670,800 | — | — | **15,818.50** |
| V5 | 250,000 | 2,000,000 | 0 | standard | 2,220,800 | — | — | **≈28,000** |
| V6 | 1,000,000 | 10,000,000 | 250,000 | itemized | 10,750,000 | 2,203,625.50 | 2,205,348.00 | **1,722.50** |

V3 is the sensitivity canary — a 0.44% margin. V6 is the donation-triggers-AMT case. V1 also pins the
non-AMT figures the same return must still produce: NIIT $19,000, Additional Medicare $6,750, balance
due $83,225.50.

**V2's TMT is deliberately left blank**: it is the vector whose value depends on the §3.3 resolution
($75,812.50 vs $55,897.50). T1 fills it in. Do not guess it.

---

## 9. Risks

| Risk | Early warning | Mitigation |
|---|---|---|
| Part III positioned wrong (§3.3) | V2/V3 KATs disagree with a hand-check | T1 blocks everything; V3's 0.44% margin is the canary |
| An out-of-scope preference stops being refused, silently breaking §3.1's exhaustiveness | a new 1099/W-2 input lands without an AMT review | a source-scan guard asserting §2's list is still refused |
| §3.4 is wrong ⇒ Tier 2 creates a Form 8801 obligation | review disputes the exclusion-item argument | Tier 2 refuses instead of filing until 8801 exists |
| Tier 1 read as closing G-4 | Tier 2 slips indefinitely | this plan; G-4 states both tiers; §1's grid is the evidence |
| Oracle can't validate AMT | T5 finds the sweep has no AMT coverage | treat §8 as the oracle for Tier 1; extend the harness in Tier 2 |

---

## 10. SemVer

**Tier 1: MINOR** — new public `Amt6251` / `compute_6251` on `btctax-core`, new `RefuseReason` variant,
and a behaviour change (returns that refused now compute). **Tier 2: MINOR** — new public emitter, new
bundled asset, new packet member. Ship each tier as its own release; do not batch.
