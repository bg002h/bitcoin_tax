# P7 Independent Review — ROUND 5, on the r4 fold (Fable, r5) — **GREEN, CLOSE THE PHASE**

*Persisted VERBATIM, per `STANDARD_WORKFLOW.md` §2. Author = Opus; reviewer = Fable.
Nothing below has been edited, softened, or reordered.*

---

**Baseline:** 95-line fold diff + current source. Tree clean, `make check` green at 1710, frozen files intact.

## Verification

The fold touches three files (`Makefile`, `golden_packet.rs`, plus the persisted r4 review). Nothing else. I attacked both things you asked about.

**1. The precondition assertion — non-vacuous, and it fires on the real scenario.**

Your `unwrap_or("0")` concern was the right one to raise, so I checked it empirically rather than reasoning about it. Probing the transcript for all twelve households:

```
single_w2_only_standard      line17=Some("0") line21=Some("0")
...  (all 12 identical) ...
mfj_se_over_the_addl_medicare_threshold   line17=Some("0") line21=Some("0")
```

Both cells are **present and written as `"0"`** on every household — the 1040 filler does write every mapped line, as you believed. `unwrap_or("0")` never fires today, so the guard is reading a real value, not a default.

More importantly, I tested the scenario the guard *exists* for, which your line-16 fault injection did not: I gave the printed 1040 an actual $500 credit on line 21, and separately a $300 AMT on line 17. Both fire, with exactly the intended message:

```
single_w2_only_standard: the cross-foot formula for line 24 assumes NO credits (1040 line21),
and this household has some. Extend the formula — do not weaken the assertion.
```

So the thirteenth-household hazard is now caught at the formula, pointing at the formula. That closes the Minor properly.

One honest residual, and it is genuinely a Nit: *if* the 1040 filler ever stopped writing those two cells, `unwrap_or("0")` would silently make the guard vacuous. That is a real (if remote) coupling — the guard depends on a property of a different module that nothing local pins. It does not gate: the packet's own `no_unmapped_filled`/geometry machinery and the r1-era "every mapped cell is written" property both stand behind it, and a household with credits would still fail the line-24 comparison itself, just with a worse message. Worth a one-line note, not a change.

**2. `--no-fail-fast` — exit status intact, and it does what it was for.**

- Two planted failures → **exit 2**, and *both* are reported (4 `FAIL` lines), plus `make check: FAILED`. The under-reporting that cost me time in r3 is gone.
- Green → **exit 0**.
- **Clippy-only red** (tests green, one `unused_variable`) → **exit 2**. This is the leg most easily broken by touching that recipe, and it still works: the `wait $$t || st=1; wait $$c || st=1` OR-ing was untouched, and I confirmed it end-to-end rather than by reading.

**3. The `printed()` helper.** Moved above its caller, one spelling, still used by `the_se_households_print_the_oracles_se_tax_onto_schedule_se`. Cosmetic, correct.

## Findings

### [NIT] The precondition guard's `unwrap_or("0")` couples it to the 1040 filler's write-every-cell property
**Where:** `crates/btctax-forms/tests/golden_packet.rs` (`got.get(cell).map(String::as_str).unwrap_or("0")`).
**What:** Absent ⇒ `"0"` ⇒ passes. Verified today that both cells are always present, so it is not vacuous — but the guard's soundness rests on a property of `fill_form_1040_full`, not on anything in the test.
**Fix:** Either assert presence (`Some("0")` rather than defaulting) or drop a comment recording that the 1040 writes every mapped line, including explicit zeros. Trivial either way.

---

## Verdict

**Clean. The last fold broke nothing.** The residual Minor is closed correctly — the precondition is now asserted where the formula is built, it is non-vacuous (both cells are really written), and it fires on the actual credits/AMT scenario rather than only on the synthetic one you tested. The `--no-fail-fast` change did not disturb exit-status propagation on any of the three legs (tests-red, clippy-red, green), which I verified end-to-end rather than by inspection.

For the record across the whole gate: the phase is now backed by twelve households against two genuinely independent engines whose answers I regenerated from scratch and diffed bit-for-bit; six Tax-Table divergences I verified line-by-line against the printed 2024 IRS table; a §199A implementation I checked against the statute and broke in four places to confirm the tests hold it; and a paper round-trip that catches a real line-24 assembly bug when I inject one. Every load-bearing guard added across the five rounds now dies under mutation — I re-ran each of my own mutations against the final tree.

Close the phase.

**VERDICT: 0 Critical / 0 Important / 0 Minor / 1 Nit**
