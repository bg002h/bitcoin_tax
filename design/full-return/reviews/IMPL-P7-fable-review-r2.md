# P7 Independent Review — ROUND 2, on the r1 fold (Fable, r2)

*Persisted VERBATIM before folding, per `STANDARD_WORKFLOW.md` §2. Author = Opus; reviewer = Fable.
Nothing below has been edited, softened, or reordered.*

---

**Baseline:** fold diff (927 lines) + current source. Tree clean, `make check` green at 1705, frozen files (`types/compute/se.rs`) still byte-identical to `059ec2a`. ✓

## Verification of the r1 fold

I re-ran my own r1 mutations and the fold's claims rather than taking them on trust.

- **r1-I2 (untested refuse) — GENUINELY FIXED.** I re-injected my exact M3 mutation (`business_qbi` → `Usd::ZERO` in `screen_absolute`). It now dies: `FAIL … qbi_above_threshold_refuses_on_a_schedule_c_business_with_no_reit_dividends`. The two anti-vacuity assertions are real (the fixture would fail loudly if it stopped producing QBI or stopped clearing the threshold).
- **r1-I3 (`make check` lies) — GENUINELY FIXED.** Planted a failing test: **exit 2**. Removed it: **exit 0**. Correct in both directions.
- **r1-M4 (divergence liveness) — non-vacuous.** I injected a dead `Divergence` entry for a household/line that always agrees; the test fails with `1 DECLARED_DIVERGENCES entr(ies) never fired`. Good mechanism.
- **Goldens regenerate bit-identically.** Re-ran both oracles from scratch (12 households): committed JSON matches exactly, `_provenance` aside. Not hand-edited.
- **The N2 catch is real and it was a good one.** All three engines now agree on the §199A deduction for all three SE households (OTS 8232.23 / taxcalc 8232.227 / btctax 8232). Finding that OTS had been agreeing *by accident* because line 12 was always zero is exactly what this harness is for.
- **Geometry (attack #1, second half) — the claim checks out.** I dumped the real `/Rect` centers from the blank `f8995.pdf`. `row1_qbi` cx = **529.2**, and the ordinary AMOUNT cluster is [504, 576] (centers 540). So 529.2 genuinely sits *inside* the wide band, the tight `(525, 533)` band genuinely excludes 540, and the reasoning is not post-hoc band-fitting. `row1_business` 230.5 ∈ (226,235) ✓, `row1_tin` 439.2 ∈ (435,443) ✓. The `Copy` drop on `Form8995Lines` has no consequence (nothing containing it derives `Copy`; it compiles and the packet is still byte-reproducible).
- **The `L12` formula (attack #3) is algebraically correct §1222(11)** for every household in the matrix. I checked all four sign cases against `net_1222`: `max(0, min(ltcg, ltcg+stcg))` equals `CapNet::preferential_gain` whenever there is no carryforward-in and no box-2a distribution — true of all 12 goldens. The capital-loss household never reaches it (no SE income). It is *not* fed btctax's answer; it is an independent restatement of the same statute. (See M1 for the caveat that follows from that.)

The fold is substantially good work. But it introduced two new defects and rests one divergence on a mis-citation.

---

## Findings

### [IMPORTANT] Form 8995 row 1i(b) prints the WRONG TIN when the spouse owns the Schedule C
**Where:** `crates/btctax-forms/src/form8995.rs:~103` (`&header.taxpayer.ssn.hyphenated()`); contrast `crates/btctax-core/src/tax/packet.rs:265-268, 289-295` (`ReturnHeader::proprietor`), `schedule_c.rs:66-77`, `schedule_se_full.rs:73`.
**What:** The new row 1i(b) hardcodes **`header.taxpayer.ssn`**. But `ScheduleCInputs` has an `owner` field, and core already derives `ReturnHeader::proprietor` precisely because — in its own words — *"a spouse-owned business files under the SPOUSE's name and SSN even on a joint return."* Schedule C and Schedule SE both use `proprietor`. Form 8995 does not.
**Why it matters:** On an MFJ return where the spouse owns the mining business, the filed packet is **internally contradictory**: Schedule C and Schedule SE are filed under the spouse's SSN, while Form 8995 claims the §199A deduction for a business whose TIN it reports as the *taxpayer's*. A business TIN that matches no Schedule C in the return is the kind of mismatch IRS matching flags. The golden matrix cannot catch it — `build_golden_household` hardcodes `owner: Owner::Taxpayer`.
**Evidence:** I built the case and ran it (probe since removed):
```
Schedule C header SSN (proprietor)  = 222-22-2222   ← spouse
Form 8995 header SSN                = 111-11-1111
Form 8995 row 1i(b) BUSINESS TIN    = 111-11-1111   ← WRONG: the taxpayer
```
**Fix:** Use `header.proprietor` (fail closed if absent, as `schedule_c.rs` already does) instead of `header.taxpayer`. Add an MFJ spouse-owned golden or unit KAT.

### [IMPORTANT] An empty `business_description` silently re-creates the exact r1-I1 defect
**Where:** guard at `form8995.rs:~89` (`if !lines.business_name.is_empty()`); `qbi.rs:~215` (`business_name: if business_qbi > Usd::ZERO { … }`); `return_inputs.rs:191-192` (`#[serde(default)] pub business_description: String`); `return_refuse.rs:363-365` (destructured `business_description: _` — **explicitly not validated**).
**What:** The fold drives the row off the *business name* rather than off *business QBI*. But `business_description` defaults to `""` under serde and **nothing anywhere refuses an empty one**. So a Schedule C captured without a description produces a Form 8995 with a **non-zero line 2 over a completely blank column (c)** — precisely the defect r1-I1 said must not exist. The fix made the defect conditional on an unvalidated free-text field instead of eliminating it.
**Why it matters:** The filer files a form claiming an $8,232 deduction, totalling a column with no rows, naming no business. Facially incomplete — same class as P6's unnamed 8949, and the exact thing this fold was supposed to close.
**Evidence:** Probe on a default-description Schedule C with $60k mining:
```
EMPTY-DESC: 8995 line2=55761  row1(a)=<BLANK>  row1(c)=<BLANK>  line15(deduction)=8232
```
**Fix:** Refuse an empty `business_description` at screen time (Schedule C **line A** needs it too — `schedule_c.rs` currently also skips line A when empty, so that form is incomplete on its face as well). Then key the row off `business_qbi > 0`, not off the name, so the row and line 2 cannot diverge.

### [IMPORTANT] The cross-footing divergence is justified by a citation that says the opposite
**Where:** `crates/btctax-core/tests/golden_returns.rs:~145-158` (`why:` "…the IRS instructions put it at the line"); mirrored at `golden_packet.rs:~86-95`.
**What:** The new `TOTAL TAX (L24)` divergence asserts btctax's 16,083 over OTS's 16,082 on the ground that **"the IRS instructions put [the rounding] at the line."** They do not. The 2024 Form 1040 instructions, *Rounding Off to Whole Dollars* (p. 23), say verbatim:

> "**If you have to add two or more amounts to figure the amount to enter on a line, include cents when adding the amounts and round off only the total.**"

Line 24 *is* a line figured by adding two amounts (lines 22 and 23). Applied literally, the instruction produces 7,604.59 + 8,477.73 = 16,082.32 → **16,082** — OTS's figure, not btctax's.
**Why it matters:** This is the mechanism I was asked to watch degenerating. The stated rule for a divergence is that btctax must be right *and the statute that settles it named*. Here the named authority settles it the other way, so the entry currently reads as "btctax is right because the instructions say so," when the instructions say the reverse. The *outcome* (a $1 overstatement, filer overpays) is defensible — SPEC §3.1 elected round-all-amounts, P6 reviewed it, LIMITATIONS.md discloses it, and it does make the filed form cross-foot — but the divergence must be grounded on **that election, declared as a deliberate departure**, not on a misreading of the instruction it contradicts. A divergence whose citation doesn't check out is the escape hatch.
**Evidence:** `pdftotext` of `irs.gov/pub/irs-prior/i1040gi--2024.pdf`, quoted above.
**Fix:** Rewrite the `why` to say what is true: the IRS instruction directs cents-in/round-the-total; SPEC §3.1 deliberately elects round-at-each-line instead, because it is what makes the filed form's printed lines add up, at a cost bounded by ~$1 in the filer's disfavour. (Or change the election — but that is a SPEC decision, not a P7 one.) Also worth confirming this reading does not silently apply to Schedule D vs. the 8949 rows, where the same instruction bites.

### [MINOR] Oracle 1 no longer independently derives Form 8995 line 12 (or line 1i(c))
**Where:** `scripts/oracle/ots_direct.py:~268-290`.
**What:** The harness now hand-computes **two** of OTS's Form 8995 inputs — `L1_i_c` (the QBI base) and now `L12` (net capital gain) — from formulas that restate the same rules btctax implements. OTS still computes the 8995 chain and both 1040 passes from them, but it can no longer *independently catch* an error in either input: if btctax's notion of §1222(11) net capital gain were wrong, the same wrong number would be handed to OTS and it would agree.
**Why it matters:** Taxcalc is now the **only** fully independent witness on line 12 (it derives it from `p23250`/`p22250`/`e00650`). It does agree (8232.227), so the check survives — but with one witness, not two, and the r1 claim of "two independent engines" is thinner on the QBI path than elsewhere. Worth stating explicitly rather than leaving implied.
**Fix:** Record the caveat in the module docstring beside the existing licensing note; ideally derive OTS's `L12` from OTS's own Schedule D output rather than from the harness.

### [MINOR] The qualified-dividends half of Form 8995 line 12 is still oracle-unchecked
**Where:** the household matrix.
**What:** `net_capital_gain = qualified_dividends + net_ltcg` in both btctax and the new harness formula — but **no golden household has both self-employment income and qualified dividends**, so the `qualified_dividends` term is always zero on every household that has a Form 8995. It is exactly the gap `single_miner_qbi_limited_by_net_capital_gain` just closed for LTCG, still open for QD. Drop the QD term from line 12 and every test stays green.
**Fix:** Give one SE household qualified dividends (or add a thirteenth), and the term becomes oracle-checked for free.

### [MINOR] `a_household_with_no_business_files_no_form_8995_row` never inspects a row
**Where:** `golden_packet.rs:~471-484`.
**What:** The test's name and docstring describe the blank-row contract ("A REIT-only Form 8995 leaves Part I blank"), but the body only asserts the *whole form* is absent (`!pkt.iter().any(|f| f.name == "f8995")`). It never reads `row1_*`. Its own comment claims the contract is "pinned by the unit KATs in `full_return_forms.rs`" — but `grep -rn "row1" crates/btctax-forms/tests/` shows **no KAT references `row1_*` at all**. The "empty name ⇒ blank row" contract is untested.
**Fix:** Assert `row1_business`/`row1_tin`/`row1_qbi` are absent on a REIT-only 8995 in `full_return_forms.rs` (the fixtures already exist — they pass `""`).

### [MINOR] Two of the three new cells have no y-check
**Where:** `form8995.rs:~89-110` + `cells.rs:73-86` (`push_literal` → `FlatPlacement::col_only`).
**What:** `row1_business` and `row1_tin` get column-band placements with **no descent ordinal**; only `row1_qbi` is y-pinned (ordinal 0). From the real PDF, the (a) column x-center 230.5 and (b) column 439.2 are each shared by **all five** table rows (y = 564/540/516/492/468). So a map typo pointing `row1_business` at row 1ii's (a) cell passes geometry silently — the business name would print on a different row than its income.
**Fix:** Add a y-band assertion for the two literal cells (they share a known y ≈ 558–564).

### [NIT] `form8995.rs` doc contradicts its own code
The module comment says the row's three cells "share a y and **CANNOT** join the ordinal-y descent group" — but the code puts `row1_qbi` in that group as ordinal 0 (correctly, and the comment even says so nine lines later).

### [NIT] The cross-footing exception is duplicated as a string match in a second file
`golden_packet.rs` hardcodes `if h.name == "single_miner_qbi_limited_by_net_capital_gain" && cell == "line24"`, re-implementing the `DECLARED_DIVERGENCES` mechanism in a place that cannot see it. It is self-invalidating (it pins OTS's figure), so it will not rot silently — but a second household needing the same treatment will copy-paste it. Consider consuming the declared list.

---

## Verdict

The fold genuinely closed all three r1 Importants — I re-injected my own mutations and watched them die, and the gate now fails on red in both directions. The N2 catch (OTS silently dropping the §199A income cap) was real and valuable, and the goldens regenerate bit-identically across twelve households. But the row-1i fix that closed I1 introduced two new defects of the same class it was fixing — a wrong TIN on a spouse-owned business, and a re-opening of the blank-column defect through an unvalidated field — and the new cross-footing divergence cites an IRS instruction that says the opposite of what it is cited for.

**VERDICT: 0 Critical / 3 Important / 4 Minor / 2 Nit**
