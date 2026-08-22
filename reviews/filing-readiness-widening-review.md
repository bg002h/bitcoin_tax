# Review — the un-reviewed range `99628341..HEAD` (the folds + the two widenings)

_Fable, read-only. 17 commits, 41 files, +4712/−381. The persisted whole-branch review covered only
the first 34 commits; everything here landed after it — nine review-response folds (authorship that
re-earns the gate) and eight implementing two owner-authorised widenings whose CODE had had **no
independent review at all**. **Persisted VERBATIM. NOT FOLDED — the owner asked to pause here.**_

★ The agent could not write this file itself (the harness blocks subagents from writing report
files), so the controller is the scribe; text copied verbatim, in its own commit, before any fold.

---

VERDICT: needs-changes

**Scope reviewed:** `99628341..HEAD` on `feat/filing-readiness`. I read the full worksheet module, the (A) lift and its KATs, the entire write-back/import/M4/render surface of (B), the T1 question/refusal/anchor/classifier wiring, the xtask checker's fourth check, the LIMITATIONS diff, and the code of all nine fold commits. I honored the standing constraints: no oracle/corpus findings, no re-litigation of the widenings themselves.

## WIDENING FINDINGS (A)/(B)

**(A) — SOUND.** What the admitted household prints, per line: 1040 1a/1z and 25a–d are collected (W-2); line 7 is computed from a Schedule D whose line 6/14 carryover entry is collected testimony (User) or btctax's own rolled figure (Computed, gated — see below); 9/11/12/14/15 computed, with line 15's floor carried by the form's own instruction in `printed.rs` (independently of the assembly-side floor — K2's doc records that double-floor honestly); tax lines computed zeros over real figures; lines 27–30 are **structurally absent** from `Form1040Lines` (`printed.rs:570` — "lines 27–30 are blank"), a lawful forgone claim with `EicOmitted` firing (`advisories.rs:941-943`, earned income $5,000 / AGI $3,000 is inside the ceiling) and LIMITATIONS naming it. I re-derived the worksheet for H1 (line 1 = −11,600 → line 4 = 0 → carry {2000,0}) and H9 (AGI 13,728.34 → line 1 = −871.66 → line 13 = 42,871.66) by hand from the extract; both match the KATs. The transcription of all 13 lines and both interstitials is verbatim against `i1040sd--2025.txt:1810-1870`. No fabricated zero found; the one blank-vs-zero hazard (worksheet skips) is held in the type system and by K4.

**(B) — TWO FINDINGS, one reachable today.**

**B-1 (Important). The write-back summary claims authorship of a write the `grounded` gate deliberately skipped.**
- WHAT IS ASSERTED: `write_back_carryover`'s summary — *"carryover written back to {Y+1}: …; capital-loss carryover short $X / long $Y"* — is "DERIVED from what was assigned" (T5 commit); LIMITATIONS says "If this year was never asked about a carryover and produced none of its own, **nothing is stamped**."
- WHAT IS ACTUALLY TRUE: the four `wrote.push` lines in `crates/btctax-cli/src/cmd/tax.rs` are **unconditional** and read the *row field*, not the assignment. When `grounded` is false (`return_1040.rs`, `if grounded { … }` — and `force` is not consulted there), the capital-loss write is skipped, yet the summary still lists it under "written back".
- THE FAILING CASE, reachable in v1 today: year Y with a charitable/QBI carryover, no capital-loss activity, never asked (carryforward-in {0,0} User, worksheet returns None). The roll succeeds and prints *"capital-loss carryover short $0.00 / long $0.00"* as written back — while nothing was stamped, and Y+1's `BenefitCarryoversNotStated` stays live for exactly that carryover. The filer holds a success message the next advisory contradicts. This is the dual of the defect K11 exists for (named two / wrote three → now names four / wrote three); K11 checks observed ⊆ summary but not summary ⊆ observed, and its fixture assigns all four, so it cannot see this. On a **re-roll** after year Y's grounding is edited away, the same line prints the *stale prior figure* under "written back" (the shape visible in K9(a)'s own red output).
- SEVERITY: Important. No filed figure moves; but it is a false filer-facing statement on the one surface T9 itself says the filer trusts to know what happened.

**B-2 (Important, owning phase = TY2025 full-return support). A Computed capital-loss stamp has no retraction path, and LIMITATIONS' cure sentence fails on exactly that branch.**
- WHAT IS ASSERTED: LIMITATIONS — *"edit the prior year and re-run `--write-carryover`"* cures a stale stamp; the import doc — *"a carryover the TOML does supply is the user's and wins (as `User`)"*.
- WHAT IS ACTUALLY TRUE: if the prior-year edit removes the roll's entire grounding (carryover-in {0,0} User and worksheet-out {0,0}), the re-roll **retains** the stale Computed figure (grounded=false skips; `--force` does not reach it); `income import` with an explicit zero is **resurrected** by the T6 arm (keyed on default+User→existing-Computed, and TOML cannot distinguish explicit zero from absent); so no command retracts a figure btctax itself stamped. The only technical escape is hand-writing `capital_loss_carryforward_in_provenance = "Computed"` in the TOML — which also means the import surface can *forge* a Computed stamp (silencing M4 via `m4_authority`'s `(None, Computed) → None` and the benefit advisory), making the quoted import comment false whenever the TOML supplies the provenance key.
- THE FAILING CASE: roll 2024→2025 with a $47,000 loss; discover the 1099-B loss was erroneous; re-import 2024 without it; re-run → summary "written back … long $47000.00", 2025 still carries the stale Computed $47,000; import 2025 with zeros → "preserved: … $47000.00". M4 catches it only while 2024 still re-derives cleanly.
- SEVERITY: Important with owning phase TY2025 — the row is unreadable for filing in v1 (the branch's own FR-8 residue acceptance), which is what keeps this out of Critical. Suggest: on a grounded=false roll over an existing Computed value, clear-to-User or warn; normalize provenance to User at import parse.

**Otherwise (B) is sound**: the grounded predicate's three disjuncts are each a supportable claim of knowledge; the rounding decision ties the stored figure to the sworn page and both M4 sides are rounded; the r3 I-4 excluded case stays closed and K6's double plant genuinely covers both halves; the atomicity reasoning (charitable refusal blocks the capital-loss roll because the deduction moves worksheet line 1) is correct tax mechanics.

## FOLD FINDINGS

None. I verified the code of all nine against their claims: the 8960 predicate matches `i8960--2024.txt:63-64` verbatim and line 12's floor is the form's own; the mortgage-gate conjunct calls the single `mixed_use_mortgage_forgone` derivation that `schedule_a_parts` keys the box and zero on (`return_1040.rs:453,492,2351`), so scope cannot drift from disclosure; the deferral-donor triple fix (warn/refuse/one-predicate) coheres and its repointed sibling test now asserts *which* gate refused; the KAT-8 fix makes the election the binding conjunct with both premises asserted; the repointed exit-code fixture tests the same refusal-agnostic contract.

## REFUSAL-SURFACE COHERENCE

**Yes.** The four added variants each carry a shared liveness predicate (`carryforward_in_present`, deliberately not TI-gated), an `Anchor::Field`, and either a cure or an honest dead-end; the deleted variant leaves zero trace (K19, with positive control). The filer I tested hardest: **the canceled-debt YES filer.** Their dead-end is substantively correct — a §108 exclusion year needs Form 982, which btctax cannot produce — but the refusal detail (*"Work the reduction out by hand (Pub. 4681, Form 982) and enter the reduced carryover"*) implies a cure the gate does not admit: the truthful answer stays YES for the exclusion year and the return refuses again after the filer does the work. That sentence is accurate only for the *following* year's return. **Minor** — reword to say btctax cannot file the exclusion year itself. Also tested: the joint-sourcing filer (can truthfully flip to NO after the hand split — real exit); the CWA-`Some(false)` deferral donor (whole write-back refused forever, but hand entry of the other three is available and the refusal text points there — disclosed, not a brick); the §108-in-Y+1 timing (the form asks the condition on the receiving year's worksheet, and btctax asks it on the receiving row — the frame matches the form exactly, so no gap for the current-year-loss household).

## WHAT WOULD MAKE THIS REVIEW WRONG

If `wrote`/`grounded` interact somewhere I did not see — e.g. a caller that suppresses the summary or re-stamps on the grounded=false path — then B-1/B-2 evaporate and the verdict is `sound`, since everything else I attacked (the worksheet arithmetic by hand, the fail-closed question surface, the fold commits' code against their claims) held.
