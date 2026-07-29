# E2 review r2 — adversarial: is it safe to ship a fixture with no vector for three routings?

**Reviewer:** independent agent (opus tier — adversarial/judgment per the tiering rule).
**Subject:** commit `3199e92` on `feat/amt-e2-vector-population`.
**Brief:** one question — is it safe to ship a fixture with NO vector for the line-32 skip, line 39's
26% side, and the un-phased-out exemption, on the claim that they cannot occur with AMT owed? Attack
the root claim's parametrization, whether the test can pass vacuously, and the per-status corollaries.
Explicitly out of scope: the generator, the oracle harness, the docs, the witness census, style.

**Persisted VERBATIM before folding**, per `STANDARD_WORKFLOW.md` §2.

---

Restored the tree to HEAD (byte-identical, `git diff HEAD` empty, `tax::form6251` 7/7 green) after the probes and mutations below.

**ROOT CLAIM: HOLDS** — I enumerated every axis of btctax's real input surface that reaches `Form6251Inputs` and classified each by its effect on the "AMT owed with the exemption intact" corner; then exhaustively probed that whole corner on a $100 grid per status with the production `compute_6251` + `qdcgt_line16`, using the *largest* deduction and *largest* FTC btctax can grant. Max `line7 − line10` over the intact-exemption region is Single −$6,377 / MFJ −$2,193 / MFS −$1,098.50 / HoH −$2,785 — all strictly negative, so with the exemption intact the form cannot owe AMT and cannot even be *attached*.

**TEST IS LOAD-BEARING: YES** — adding `+ Decimal::from(50000)` (a fake line-2i ISO preference) to `line4` in `compute_6251` reds it with `Single itemized=true: AMT 68.76 owed with the exemption INTACT`; it also reds at +$30k/+$20k/+$10k, and `checked` is 5,945–13,136 per cell (not a trivial >500).

**COROLLARIES: SOUND for (b), UNSOUND-AS-WRITTEN for (a) on HoH** — `line32 ≤ line25` always, so `line12 > line25` kills the skip; that holds for Single ($523,650 > $518,900, margin only $4,750), MFS and MFJ, but **not** HoH, where min `line12` = $523,650 sits *below* HoH's $551,350 15%-band top.

---

**1. Important — "it reds if the input surface ever widens" is false below ~$10k, because the sweep cannot express the §63(f) standard deduction.**
`phaseout_precondition_sweep_for` (`/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/form6251.rs`) pins the non-itemized arm to `std_deduction(status)` — the *base* amount. btctax grants aged/blind boxes on top (`standard_deduction` in `/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_1040.rs:69`, `AgedBlindBoxes::count()` 0–4 in `/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/packet.rs:265`): up to **+$6,200 MFJ / +$3,900 Single-HoH / +$3,100 MFS** of extra *line 2a* add-back. Larger line 2a at fixed (ordinary, gain) raises AMTI while the regular tax is unchanged — the one direction that breaks the claim. It eats 44–56% of the margin (MFS: −$1,966.50 → −$1,098.50). Concrete false PASS: with a $5,000 AMTI widening the sweep stays **green**, while an MFS filer with `deduction = $17,700` owes AMT with the exemption intact at **7,895** grid points (best margin +$1.50, at ordinary $191,900 / gain $100,000). Fix: loop `deduction` over `[std_deduction(status), std_deduction(status) + max_boxes(status)]` — or just use the maximum, since larger line 2a strictly dominates at fixed (ordinary, gain).

**2. Important — the guard is `amt() > 0`, but Tier 2 attaches on `must_attach()`, and the sweep pins `schedule_3_line1: Usd::ZERO`, which makes the two identical by construction.**
With FTC = 0, `line10 = R` and `line11 > 0 ⟺ line7 > line10`, so the sweep can never distinguish the population it guards from the population Tier 2 files. They do differ in production — fixture V9 is `attach=True, AMT=0, ftc=600`. The routings are dead in the wider population too (verified: attach margin ≤ −$1,098.50 with FTC at the §904(j) ceiling), so nothing is wrong today; but under a widening there is a ~$2,300-wide band of preference size where a routing goes live on a *filed* form while this tripwire is silent. Fix: add an FTC arm at the ceiling ($300; $600 MFJ) and change the guard to `f.must_attach()`.

**3. Minor — the HoH leg of the (c) ⇒ (a) derivation is a false universal.**
The doc comment says "its line 12 exceeds the §1(h) 15%-band top, so a 20% tranche always survives lines 32/33 and the skip never fires." For HoH the phase-out-begun floor on line 12 is $609,350 − $85,700 = $523,650, which is **below** HoH's line-25 top of $551,350 — a $27,700 window the argument does not cover. The conclusion still holds (fine scan of that window: best `line7 − line10` = −$9,436, and the skip additionally needs ordinary ≤ $27,700, far from the margin's peak at ~$191,900), so this is a wrong *reason*, not a wrong result — but it is the "confident equivalence comment that was simply wrong" pattern `CLAUDE.md` codifies. Either name HoH as carried by the sweep rather than the argument, or state the correct bound (`line32 ≤ line25`, plus the ordinary-income constraint for HoH).

---

# E2 review r3 — narrow re-check of the r2 fold (commit `5a09b2d`)

**Same reviewer, resumed with its r2 context.** Brief: three questions only — (A) is each r2 finding
actually closed, (B) does the rewritten corollary paragraph state anything still false, (C) did the
fold introduce a new way for the test to pass vacuously. No fresh audit, no file modification.

**Persisted VERBATIM before folding.**

---

`git diff HEAD` is empty; all probing was done read-only plus a Python replication in /tmp.

**A: CLOSED — all three.**
- #1: the `deduction` loop is present and the amounts are right — $6,200 MFJ/QSS, $3,100 MFS, $3,900 Single/HoH — matching `std_aged_blind_married/unmarried` (`tax_tables.rs:124-125`), `AgedBlindBoxes::count()`'s 0–4 range, and `standard_deduction`'s MFS-declines-the-spouse-box rule.
- #2: guard is `f.must_attach()` and `schedule_3_line1` now loops `[0, ftc_ceiling]`; the ceilings ($600 MFJ/QSS, $300 otherwise) match `return_refuse.rs:241`. I re-derived the sweep grid independently in Python and reproduce the Rust `checked` counts exactly at ftc=0 (Single 7,175 / MFJ 13,136 / MFS 11,283 / HoH 8,181), so the replication is faithful.
- #3: HoH is now named as carried by the sweep, and both the $27,700 window and the `line27 ≤ line25 − line12` side condition are correct — but the replacement inequality introduces a new false statement (see B).

**B: UNSOUND — the new chain drops the `max(0, …)` clamp on line 29, and 7 of the fixture's own 30 vectors falsify it.** The four per-status numbers, both sets, are correct: floors $523,650 / $542,700 / $1,085,400 (all clearing $232,600 / $116,300, so (b) is sound); Single's $4,750; HoH's $27,700; and the margins −$6,377 / −$2,193 / −$1,098.50 / −$2,785 are my measured values. The ~$4,400 sensitivity also independently reproduces on my own replication of the grid — green at $4,200, red at $4,400 (20 intact-and-attachable points) on MFS, deduction $17,700, FTC $300 — and the 26–28% arithmetic checks ($1,098.50 / 0.28 = $3,923). The defect is only in the middle term.

**C: NO NEW VACUITY.** `amt() = max(0, line7 − line8 − line10)` with `line8 ≥ 0`, so `amt() > 0 ⇒ line7 > line10 = must_attach()` for every FTC value — the new guard is a superset by construction, not by parametrization, and a nonzero FTC only lowers line 10, which can only add points. Measured per cell, the guarded population rises monotonically along both new axes and never falls (Single 7,175→7,591, MFJ 13,136→13,627, MFS 11,283→11,701, HoH 8,181→8,613), all far above the 500 floor, and the ftc=0 arm is retained so nothing is dropped. I also checked the one way the widening could have *falsely* fired: a `must_attach` point with preferential income = 0, where `line12 = line32 = 0` would make `line32 != line12` a spurious red — zero such points across all 16 (status, deduction, ftc) cells.

---

**1. Minor — `line32 = line23 + line30 ≤ line21 + (line25 − line21 − line27) = line25 − line27 ≤ line25` is false whenever line 29 clamps.** Production clamps it: `f.line29 = (f.line25 - f.line28).max(z)`. When the regular-side ordinary bottom exceeds the 15%-band top (`line27 > line25`), line29 = 0, line21 = 0, so line32 = 0 while `line25 − line27` is negative — the stated bound is violated. V1, V3, V4, V6, V7, V8 and V9 are all in that state (V1: line27 = $915,000, line25 = $583,750, line32 = 0, bound = −$331,250). The conclusion `line32 ≤ line25` survives, and so does the HoH side condition, but via the clamped form: `line32 ≤ line21 + max(0, line25 − line21 − line27) = max(line21, line25 − line27) ≤ line25`, using `line21 ≤ line19 < line25` on all four statuses. Substituting that one expression restores the paragraph — and it still gives `line12 > line19` ⇒ the skip needs `line27 ≤ line25 − line12`, so nothing downstream changes.

**2. Nit — "the best `line 7 − line 10` is still negative on every status" is 0, not negative.** The unrestricted maximum over the intact region is exactly $0, at the degenerate corner where line 7 = line 10 = 0 (line 6 ≤ 0, regular tax ≤ FTC). The four quoted figures are the maxima over points with **line 7 > 0**, which is the meaningful set. Since `must_attach` is a strict `>`, $0 still fails to attach and the conclusion is untouched; "never positive, and at most −$1,098.50 wherever line 7 > 0" would be exact.

---

## Fold of r3 (this session)

Both accepted. The chain now carries line 29's clamp — `line32 ≤ line21 + max(0, line25 − line21 −
line27) = max(line21, line25 − line27) ≤ line25` — and the margin sentence says "never positive, and
at most −$1,098.50 wherever line 7 > 0", with the $0 degenerate corner named.

**And the bound stopped being prose.** Having been written wrongly twice, it is now two assertions
inside the sweep, evaluated at every point rather than only the attachable ones. Mutation-verified:
removing `max(z)` from `f.line29` reds four tests. That is the r2/r3 lesson applied rather than
merely recorded — a claim that survives two prose reviews and dies instantly to one assertion did not
need a third reviewer, it needed a test.
