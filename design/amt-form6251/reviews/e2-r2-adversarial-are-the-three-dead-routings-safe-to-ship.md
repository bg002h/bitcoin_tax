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
