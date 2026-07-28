# Form 6251 (AMT) — staged implementation plan

**Status:** r1-FOLDED, awaiting re-review. Not build-ready until the §2 review loop reaches
0 Critical / 0 Important, per `STANDARD_WORKFLOW.md`.

**Goal (one sentence):** stop refusing returns over the AMT screen — first by computing Form 6251
internally and proceeding when no attachment is required (**Tier 1**), then by filling and attaching the
real form when it is (**Tier 2**).

**Base:** `main` after `fix/amt-screen-line2`. **Lineage:** `FOLLOWUPS.md` §G-4.
**Review history:** r1 = Fable × 4 tax lenses + Opus × 1 plan lens, adjudicated by Fable, against the
fetched 2024 `f6251`/`i6251`/`i1040gi` PDFs → **5 Critical, 12 Important**, all folded here. Reviews are
persisted verbatim in `design/amt-form6251/reviews/`.

---

## 1. Why, and why two tiers

btctax v1 does not compute AMT. It runs the official *"Worksheet To See if You Should Fill in Form
6251"* and, when that worksheet says a 6251 is required, **refuses the entire return and writes no forms
at all**. The worksheet answers *"must you fill in Form 6251?"*, never *"do you owe AMT?"*

| | Who | What they need |
|---|---|---|
| **Tier 1** | Form 6251 line 7 ≤ line 10 ⇒ **no attachment required** | Stop refusing |
| **Tier 2** | Line 7 > line 10 ⇒ **Form 6251 must be attached** | A filled, attached form |

Tier 1 is cheap because when no attachment is required, Schedule 2 L2 → L3 → 1040 L17 are all $0 and the
printed packet is what a correct engine would already emit. **This is asserted in T4 against a
hand-built expected packet, not by comparing to today's output — today writes no packet at all.**

**★ Tier 2 is not optional, and Tier 1 must not be read as closing G-4.** The trigger rule:

> AMT is owed when the exemption is **fully phased out** (AMTI ≥ $1,751,900 MFJ) **and** ordinary
> taxable income is below the crossover.

**The crossover is add-back dependent** (r1 I-3 — an earlier draft quoted one figure as if universal):
≈ **$769,139** with a zero add-back, ≈ **$800,250** for a SALT-capped itemizer, ≈ **$859,983** for a
standard-deduction filer. Below it the graduated brackets are cheaper than AMT's flat 26/28%; above it
the 37% bracket wins. **Exposure peaks are also add-back dependent**: $24,619 at ordinary TI $383,900
with a zero add-back, rising by 0.28 × add-back to **$32,795** for a standard-deduction filer.

A salaried engineer selling a large Bitcoin position sits squarely inside that region — at $250,000 of
wages and a $2M gain the AMT is **$26,271.00** (r1 C-2: an earlier draft said "≈$28,000", which is within
rounding of $27,731 — the figure the *wrong* Part III reading produces. Never approximate a vector).

---

## 2. Scope boundary

**In scope:** Form 6251 Parts I–III including lines 8–10, for the inputs v1 accepts; the refusal split;
Schedule 2 line 2 (Tier 2); the PDF asset, map and emitter (Tier 2).

**Out of scope — refused upstream or uncapturable, and each must STAY so** for §3.1 to be exhaustive:
§57(a)(5) PAB interest · §56(b)(3) ISO · §57(a)(7) §1202 · §163(d)/§4952 investment interest ·
§56(a)(1) depreciation · NOL/ATNOL · estate & trust K-1 · §56(a)(6) dispositions · §57(a)(1) depletion ·
§57(a)(2) IDC · §56(a)(3) long-term contracts · pre-1987 installment sales.

**★ Two items are NOT covered by that dichotomy and need their own handling** (r1 I-5, I-6):

- **Line 2k — an AMT-divergent capital-loss carryover.** `capital_loss_carryforward_in` is a declared,
  externally-originated input: neither refused nor uncapturable. btctax cannot know whether the filer's
  AMT carryforward differs. Requires an explicit **declaration** (equal-for-AMT? unknown ⇒ refuse),
  mirroring the existing unanswered-question pattern. Direction if ignored: **understates**.
- **Line 3 — mortgage interest on a non-qualified dwelling.** §56(b)(1)(C) adds back interest on a
  dwelling that is not a principal or qualified second residence (houseboat, RV, transient use). The
  `mortgage_interest_1098` input carries no dwelling question. Requires a **declaration** (None ⇒
  refuse), mirroring `mortgage_all_used_to_buy_build_improve`. Direction if ignored: **understates**.

**Form 8801 is out of scope and §3.5 argues no NEW obligation arises.** (A pre-btctax 8801 carryforward
is a separate, already-unsupported input.)

---

## 3. The computation

### 3.1 AMTI (Form 6251 lines 1–4)

```
line1 = taxable_income_L15                                  // "if zero or less, enter -0-"
line2a = amt_worksheet_line2(itemized?, standard_deduction, schedule_a_line7)   // §56(b)(1)(A)(ii)/(E)
line2b = − state_refund_taxable                             // ★ r1 C-3: NEGATIVE. i6251 p.5.
AMTI  = line1 + line2a + line2b  ( + the MFS kicker, §3.2 )
```

**★ r1 C-3.** An earlier draft wrote `AMTI = taxable_income + add-back` and dropped line 2b — it
reproduced screening-worksheet lines 1–3 and lost lines 4–5. The input is live:
`ri.sch1.state_refund_taxable` (`return_inputs.rs:317`), used at `return_1040.rs:668, 736, 1112`, and the
existing screen already subtracts it. Omitting it **overstates** AMTI ⇒ over-refuses in Tier 1 and files
an overstated Schedule 2 L2 in Tier 2. Note line 2b sits **outside** Who-Must-File condition 4's "lines
2c through 3", so it never by itself forces an attachment.

`amt_worksheet_line2` (`tax/amt.rs`) is reused verbatim. §199A stays subtracted (§199A(f)(2); line 1
starts net of QBI and there is no add-back line) — r1 CONFIRMED.

### 3.2 Exemption and taxable excess (lines 4–6)

```
// ★ r1 C-4 — the MFS line-4 kicker. i6251 p.9, verbatim:
//   "If your filing status is married filing separately and line 4 is more than $875,950, you must
//    include an additional amount on line 4. If line 4 is $1,142,550 or more, include an additional
//    $66,650. Otherwise, include 25% of the excess of the amount on line 4 over $875,950."  (§55(d)(3))
if status == Mfs && line4 > mfs_kicker_start {
    line4 += min(0.25 * (line4 − mfs_kicker_start), mfs_kicker_max)
}
exemption      = max(0, base − 0.25 * max(0, line4 − phaseout_start))
taxable_excess = max(0, line4 − exemption)          // line 6; ≤ 0 ⇒ enter 0 on 7, 9 and 11
```

MFS is a live status (`types.rs:13`). Omitting the kicker understates AMTI by up to $66,650 ⇒ TMT by up
to ~$18,662 ⇒ **understated filed tax** — the only finding in this plan whose error direction is
understatement. `mfs_kicker_start` ($875,950) and `mfs_kicker_max` ($66,650) join `AmtParams`.
**The existing screen lacks the kicker too and is fixed in the same task.**

### 3.3 Part III — SETTLED (r1 C-1)

An earlier draft posed this as "$75,812.50 vs $55,897.50". **That was a false dichotomy — both are
wrong.** The form does two independent things:

**(a) Bands are positioned by the REGULAR return.** Line 20: *"Enter the amount from line 5 of the
Qualified Dividends and Capital Gain Tax Worksheet … **(as figured for the regular tax)**."* Line 27
repeats it for the 20% band. Contrast line 13's *"(as refigured for the AMT, if necessary)"* — the gain's
**amount** is AMT-side; its band **positions** are regular-side.

**(b) The preferential slice is CAPPED at the taxable excess.** Line 16: *"Enter the **smaller** of line
12 or line 15."* Line 22: *"Enter the **smaller** of line 12 or line 13."* Line 17 = L12 − L16 floors the
26/28% slice at 0. And when line 32 = line 12: *"skip lines 33 through 37"* — the 20% tranche never
engages. Line 40 takes *"the **smaller** of line 38 or line 39."*

```
excess      = line6
pref        = min(excess, net_capital_gain + qual_div)        // L13/L15/L16, L22
ord_slice   = max(0, excess − pref)                           // L17
line38      = tax26_28(ord_slice) + preferential_tax(REGULAR bottoms, pref)
line39      = tax26_28(excess)
TMT         = min(line38, line39)                             // L40
```

`preferential_tax(bp, bottom, pref)` already exists (`compute.rs:57`). The excess-<-gain case needs **no
special code** — L16/L17/L22 and the L32 skip handle it structurally — but it **must** gain a KAT (V2b).

**The upper-bound shortcut stays rejected, for a corrected reason (r1 I-2).** The regular-position stack
on the *full* gain IS an unconditional upper bound on line 40 (positions are regular-side; preferential
tax is monotone in the amount and L16/L22 only shrink it; L40's min only lowers further). The earlier
draft's rationale — "only valid while the add-back is smaller than the exemption" — is **false**, and
that misconception is exactly what produced $75,812.50. Reject it instead because Tier 2 must fill lines
12–40 exactly anyway, T3's refusal message names an exact dollar, and a second approximate path adds
risk for nothing.

### 3.4 Lines 8–10 and the attach test (r1 I-1)

**The Tier 1 / Tier 2 boundary is Who-Must-File condition 1, not `AMT > 0`.** i6251 p.1: *"Attach Form
6251 to your return if any of the following statements are true. 1. Form 6251, line 7, is greater than
line 10."* Only condition 1 is reachable in v1.

```
line7  = TMT (from §3.3)
line8  = AMT foreign tax credit; for the §904(j) elector this EQUALS Schedule 3 line 1  (i6251 p.10)
line9  = line7 − line8
line10 = 1040 L16 + Schedule 2 L1z − Schedule 3 L1     // ★ NOT 1040 L24
line11 = max(0, line9 − line10)  → Schedule 2 line 2
```

Three consequences the earlier draft missed:
1. **`regular_tax` was undefined.** It is line 10. Passing 1040 L24 would overstate AMT by NIIT +
   Additional Medicare — $25,750 on V1 alone.
2. Net AMT is invariant to the FTC (line 8 and line 10's Sch-3-L1 subtraction cancel), but **printed
   lines 8/9/10 are each wrong by the FTC** if line 11 is filled as `max(0, TMT − regular)`.
3. There is a window — `1040 L16 − FTC < line7 ≤ 1040 L16` — where **AMT is $0 yet the form must still be
   attached**. "Proceed with no form" (T3) and "skip when $0" (T9) are both wrong there.

Also per i6251 p.10: *"If the amount on line 10 is greater than or equal to the amount on line 7 … Leave
line 8 blank and enter -0- on line 11."*

### 3.5 Why no NEW Form 8801 obligation arises — r1 CONFIRMED IN FULL

§53(d)(1)(B)(ii)(I) specifies **all** of §56(b)(1) — the taxes add-back and the standard-deduction
add-back — as **exclusion** items, which are excluded from the minimum tax credit. Form 8801 Part I line
15 = the entire AMT; lines 18 and 21 = $0; per i8801 *Who Should File* the filer is not even directed to
complete the form. The §904(j) FTC cancels symmetrically (i8801 Line 12). Every **deferral** item is in
§2's out-of-scope list.

**Conditional on** §2's two new declarations (I-5, I-6) and §5's exhaustiveness guard. Carry a
`debug_assert` that every AMT adjustment applied is a §56(b)(1) exclusion item, plus an 8801-recompute
KAT asserting $0.

---

## 4. File map

| File | Tier | Change |
|---|---|---|
| `btctax-core/src/tax/form6251.rs` | 1 | **new** — §3.1–3.4; emits the full **line vector**, not five scalars (r1 I-8) |
| `btctax-core/src/tax/amt.rs` | 1 | keep as cheap pre-filter; **add the MFS kicker here too**; reuse `amt_worksheet_line2` |
| `btctax-core/src/tax/tables.rs` | 1 | `AmtParams` += `mfs_kicker_start`, `mfs_kicker_max` |
| `btctax-core/src/tax/return_refuse.rs` | 1 | `AmtOwed` (**kept in Tier 2**, trigger narrowed — r1 I-9) + the two §2 declarations |
| `btctax-core/src/tax/return_1040.rs` | 1 | `screen_absolute` calls the computation; `AbsoluteReturn.amt` |
| `btctax-input-form/src/attribute.rs` | 1 | ★ r1 I-11 — exhaustively anchors every `RefuseReason` (hand-enumerated test at `:348`) |
| `btctax-core/src/tax/printed.rs` | 2 | `Schedule2Lines.line2/line3`; `Form6251Lines` |
| `btctax-forms/forms/2024/f6251.pdf` + `.map.toml` | 2 | **new** asset + map (6251 is **tax-year**-versioned, unlike 8275) |
| `btctax-forms/src/form6251.rs` | 2 | **new** emitter |
| `btctax-cli/src/cmd/admin.rs`, `cli.rs` | 2 | packet member; `--forms form6251` |
| `btctax-cli/src/cmd/…` (report) | 1 | print AMTI / exemption / TMT / AMT so the filer sees the number that un-refused them |
| `oracle-harness/src/main.rs:704` | 1 | ★ **narrow** to the AMT reason only — it is a combined check over three refusal classes |
| `scripts/oracle/gen_goldens.py:257`, `scripts/oracle/corpus.py` | 1 | ★ the **binding** AMT exclusions (`c09600 != 0`) + domain caps |
| `docs/man/*`, `docs/examples/examples.md`, `btctax limitations` | 2 | regenerate; retire the "AMT screen trips" bullet |

---

## 5. Global constraints

- **Gate:** `make check` **and** `cargo fmt --all -- --check`, from the first commit. Green = suite passes
  **and** 0C/0I.
- **Fail-closed at every commit.** No task may make a previously-refused return computable unless its
  Form 6251 line 7 ≤ line 10 is proven, or the form is filed.
- **Never understate.** C-4 (MFS) and §2's two declarations are the understatement risks; each refuses
  when unknown.
- **Every guarantee ships with a test that reds when the guarantee is removed** (T3-style mutation). This
  project's recorded failure: a correct fix landed and a caller-level revert passed the entire suite.
- Whole-dollar rounding per SPEC §3.1, per line.
- **No literal AMT constant outside `AmtParams`** — enforced by a source-scan test in T2 (r1 I-12).
- **§3.1's exhaustiveness is guarded executably** by a source-scan test asserting §2's out-of-scope list
  is still refused (r1 I-12).

---

## 6. Tier 1 — tasks

### T1 — pin the oracle BEFORE writing code (BLOCKING)

★ r1 C-5: the earlier gate could not detect a wrong Part III. V1/V3/V4/V6 all have regular ordinary
bottoms above $583,750, so both readings give identical TMTs — they are **mathematically insensitive**
and cannot be the canary. And §9's "oracle can't validate AMT" was **false**:
`scripts/oracle/gen_goldens.py:215` already runs Tax-Calculator and reads `c09600` — an independent Form
6251 including Part III — which line 257 then discards.

- [ ] Write `design/amt-form6251/PART_III.md`: lines 12–40 line by line, each tagged AMT-side or
      regular-side, quoting lines 20/27 *"(as figured for the regular tax)"* against line 13
      *"(as refigured for the AMT)"*, plus the L16/L22 caps and the L32 skip.
- [ ] **Derive every §8 vector's TMT from `c09600`**, not by hand, and record them *before* any code
      exists. Hand figures are the cross-check, not the source.
- [ ] KATs asserting `tentative_minimum_tax` (**not** `amt` — V2's AMT is $0 under both readings, only
      its TMT discriminates) on V2, V2b and V5.
- [ ] Watch them fail.

### T2 — `form6251.rs`

- [ ] The full line vector for Parts I–III (r1 I-8: Part III alone is ~30 printed boxes, and §5 requires
      per-line rounding). Mark `#[non_exhaustive]` so Tier 2 is not a breaking change.
- [ ] §3.1 incl. the negative line 2b; §3.2 incl. the MFS kicker; §3.3; §3.4 lines 8–11.
- [ ] `AmtParams` += the two MFS constants; **fix the existing screen's missing kicker in this task**.
- [ ] KATs: every §8 vector; the phase-out boundaries ($1,218,700 / $1,751,900); the 26/28 breakpoint
      ($232,600); the MFS kicker boundaries ($875,950 / $1,142,550); line 6 ≤ 0; line 1 ≤ 0.
- [ ] The two source-scan guards from §5.

### T3 — the refusal split

- [ ] Keep `RefuseReason::AmtOwed`; narrow its trigger to **line 7 > line 10** (r1 I-1, I-9). Message
      names the exact dollar and says v1 cannot yet fill the form.
- [ ] Add the §2 declarations (AMT capital-loss twin; qualified dwelling) as unanswered ⇒ refuse.
- [ ] Screen clears ⇒ AMT $0. Screen trips ⇒ compute; line 7 ≤ line 10 ⇒ **proceed**; else refuse.
- [ ] ★ r1 I-11 — enumerate every surface un-gated by clearing this Hard blocker (report / harvest /
      what-if / conservative-promote / TUI) and update `attribute.rs` + their goldens.
- [ ] ★ Mutation: revert to the blanket refusal — the V1 KAT must red.

### T4 — printed-output correctness

- [ ] Assert a no-attachment return's printed 1040/Schedule 2 against a **hand-built expected packet**
      (r1 non-blocking: "byte-identical to today" is uncheckable — today writes nothing). L17 = 0,
      Sch 2 L3 = 0, no Form 6251.
- [ ] ★ r1 I-10 — add a **screen-tripping, no-attachment** journey to the bundled examples. Every current
      journey is deliberately sized under the screen (`testonly.rs:48–51, 58–59`), so regeneration alone
      proves nothing.

### T5 — oracle domain

- [ ] Narrow `main.rs:704` to the **AMT reason only** (r1 I-7 — it currently covers three refusal classes;
      a blanket delete would admit QBI-over-threshold and TI≤0 returns).
- [ ] Lift `gen_goldens.py:257`'s `c09600 != 0` rejection and widen `corpus.py`'s caps — these are the
      binding exclusions.
- [ ] Assert a **numeric floor** on newly-covered households (r1 non-blocking: "record how many" is not
      an assertion).

**Tier 1 gate:** suite green, 0C/0I, a no-attachment return exports a complete packet, and §8 reconciles
against `c09600`.

---

## 7. Tier 2 — tasks

### T6 — asset and map
- [ ] Bundle official `f6251.pdf` (2024) + `f6251.map.toml`; every mapped field verified present.

### T7 — emitter
- [ ] `btctax-forms/src/form6251.rs` filling Parts I–III from the T2 line vector, **including lines
      8/9/10** (r1 I-1). Read-back via `verify_flat`; byte-reproducible golden for V5.

### T8 — Schedule 2 and 1040 — r1 CONFIRMED CORRECT
- [ ] 6251 L11 → *"Enter here and on Schedule 2 (Form 1040), line 2"*; Sch 2 L3 = L1z + L2 → 1040 **L17**.
- [ ] KAT: V5's balance due with AMT included.

### T9 — packet and CLI
- [ ] Attach iff **line 7 > line 10** (not `AMT > 0`). ★ Tier-2 **skip** KAT with mutation discipline
      (r1 I-10: "no 6251 in the packet" is vacuous before the emitter exists).
- [ ] Keep `AmtOwed` for the still-unreachable Who-Must-File conditions; do not delete the variant
      (r1 I-9 — and deleting a public variant contradicts §10's MINOR).
- [ ] Regenerate docs; retire the `limitations` bullet.

---

## 8. Worked vectors — MFJ, TY2024

**★ r1 C-5: T1 derives every TMT from `c09600` before code exists. The figures below are the
cross-check.** Those marked ✅ were independently recomputed and confirmed in r1.

| # | Wages | LTCG | Gift | Ded | Taxable income | Regular tax | TMT | AMT | Why it exists |
|---|---:|---:|---:|---|---:|---:|---:|---:|---|
| V1 ✅ | 1,000,000 | 500,000 | 85,000 | item | 1,415,000 | 364,675.50 | 327,965.00 | 0 | the baseline |
| V2 ✅ | 1,000,000 | 500,000 | 750,000 | item | 750,000 | 129,397.50 | **113,654.50** | 0 | TMT discriminates the readings |
| **V2b** ✅ | 1,000,000 | 500,000 | 1,000,000→900,000 | item | 600,000 | 87,918.50 | **70,005.00** | 0 | ★ **excess < gain**: L16 caps, L17 = 0, L32 skip |
| V3 ✅ | 1,000,000 | 10,000,000 | 0 | std | 10,970,800 | 2,285,321.50 | 2,275,348.00 | 0 | 0.44% margin — but **insensitive** to Part III |
| V4 ✅ | 700,000 | 10,000,000 | 0 | std | 10,670,800 | 2,175,529.50 | 2,191,348.00 | **15,818.50** | AMT owed, no donation |
| V5 ✅ | 250,000 | 2,000,000 | 0 | std | 2,220,800 | 420,929.50 | 447,200.50 | **26,271.00** | the archetypal user |
| V6 ✅ | 1,000,000 | 10,000,000 | 250,000 | item | 10,750,000 | 2,203,625.50 | 2,205,348.00 | **1,722.50** | a donation *creates* AMT |
| **V7** | — | — | — | — | — | — | — | — | ★ **state refund > 0** (line 2b) — derive in T1 |
| **V8** | — | — | — | — | — | — | — | — | ★ **MFS** at $875,950 and $1,142,550 — derive in T1 |
| **V9** | — | — | — | — | — | — | — | — | ★ **FTC > 0**, incl. the attach-with-$0-AMT window — derive in T1 |
| **V10** | — | — | — | — | — | — | — | — | ★ regular ordinary < $94,050 so the **0% band** engages — derive in T1 |

V1's non-AMT figures the same return must still produce: NIIT $19,000; Additional Medicare $6,750;
**balance due $83,225.50 against payments of $307,200 = $300,000 W-2 box 2 + $7,200 mandatory
Additional-Medicare withholding** (Form 8959 line 24 → 1040 line 25c) — r1 I-4: the figure is ambiguous
by exactly $7,200 without that composition.

---

## 9. Risks

| Risk | Early warning | Mitigation |
|---|---|---|
| Part III encoded wrong | V2/V2b/V5 disagree with `c09600` | T1 blocks; **V2b** is the discriminating canary (V3 is insensitive) |
| MFS kicker omitted ⇒ **understated tax** | MFS boundary KATs absent | T2 pins $875,950 / $1,142,550 in both the screen and the computation |
| An out-of-scope preference stops being refused | a new input lands without an AMT review | §5's source-scan guard (T2) |
| §2's two declarations skipped ⇒ understatement | a 1098 or carryforward vector with no declaration | T3 refuses when unanswered |
| §3.5 wrong ⇒ a Form 8801 obligation | review disputes the exclusion-item argument | r1 confirmed it; `debug_assert` + 8801-recompute KAT |
| Tier 1 read as closing G-4 | Tier 2 slips | §1's grid; G-4 states both tiers |
| KATs certify a misreading | four of six vectors are insensitive | T1 derives from `c09600` first; V2b/V5/V7–V10 added |

---

## 10. SemVer

**Tier 1: MINOR** — new public `form6251` API (`#[non_exhaustive]` so Tier 2 is additive), new
`RefuseReason` variants, and a behaviour change (previously-refused returns now compute).
**Tier 2: MINOR** — new emitter, bundled asset, packet member. Ship each tier separately.
