# Conservative-filing SPEC v2 — tax-correctness lens review r2 — GREEN (0C/0I)

_Fable, independent, tax-correctness lens, round 2. Verified v2 against the live IRS i8949 (2025 digital-asset
boxes G-L + the "Do not use C/F" prohibition + VARIOUS split + basis-explanation sentence), §§1001/1011/1222/
1223/6662(d), §170(e)(1)(A), §1014, Rev. Proc. 2024-28, Notices 2025-7/2026-20, and the engine source._

VERDICT: GREEN

r1-C1 (always-LT → understated tax): RESOLVED — G-4 + D-6 derive Part from the leg's computed term (ST → Part I with I/H; LT → Part II with L/K); D-2 is purely the date convention, right direction; P1 test pins "LT iff window_end >1yr before disposal" (matches §1223 day-after, conventions.rs:80 / fold.rs:72 `term_for`); P7 term-correct with a no-hard-coded-"long-term" test; §6 adds the term-split KAT. No residual LT assumption anywhere in v2.

r1-C2 (stale 8949 boxes; false code claim): RESOLVED — D-6's TY2025+ scheme (no-1099-DA → L/I; 1099-DA-without-basis → K/H; pre-2025 keeps C/F) matches the live i8949 exactly. False code-grounding claim gone, replaced by an honest thrice-stated prerequisite on the shipped-box fix (sound; the "unbuildable until the fix" state is called out for the plan to sequence). Boxes condition on 1099-DA existence, not custody (avoids the fabricated-1099 trap). Dropping VARIOUS for a single window-end date is i8949-compliant for a one-row tranche (VARIOUS is a permission for combined multi-date lots, not a requirement).

r1-I3 (close-min isn't a floor): RESOLVED — P5 renamed "window reference-price," "NOT a true floor" caveat stated, never filed (D-7 $0-only); P6's "~$X … at the cost of a documented basis an examiner can question" honestly bounded. (Residual: a stale cross-reference — Nit-1.)

r1-I4 (overbroad no-8275): RESOLVED — D-4 scopes no-8275 to the $0 position (a $0 basis can't understate gain); P7 stays mandatory on the i8949 basis-explanation ground; D-10 states §6662(d)(2)(B) mechanics correctly and mandates Form 8275 for Approach B's floor path.

r1-min-5: RESOLVED — D-1 sets `acquired_at = window_end`, D-2 pins it (never midpoint) — the operative pin since DeclareTranche gets its own fold arm.
r1-min-6: RESOLVED — P1 corrects "how acquired" to the Form 8283 donor field (forms.rs:237), new variant → `Review`, §170(e) interplay correct (LT → FMV; ST-held → basis $0, §170(e)(1)(A)).
r1-min-7: RESOLVED — no-loss Invariant KAT (gain = proceeds − $0 ≥ 0; no §1211/§1212/§1091) + the B-path no-loss-off-estimate warning.
r1-min-8: RESOLVED (with note) — §1014 date-of-death nudge (a) + provenance-neutral P3/P7 (c) folded; INHERITED-in-col-(b) (b) not folded but reasonably subsumed (a tranche is provenance-unknown; the nudge routes known-inherited out to the legal reconstruction). Non-gating.
r1-min-9: RESOLVED — D-4 paraphrases the i8949 ask contextually, not a clipped quote.
r1-min-10: RESOLVED (substantially) — D-1 declares into a specific wallet; D-8 works the Rev. Proc. 2024-28 interaction; the B-note is a B-spec concern.

## New findings
No Critical. No Important.

**N-1. [Nit] P5 stale cross-reference:** "caveated in D-2/P6 copy" — in v2's renumbering D-2 is the holding-period-date decision and carries no reference-price content; the caveat's homes are P5 and P6. Fix the pointer to "D-7/P6" or "P6".

**N-2. [Nit] D-9's "gain-maximizing inversion" is not always tax-maximizing — but no understatement arises either way.** FIFO consumes an old $0 tranche first and maximizes gain; character can invert the tax comparison (an LT $0 tranche at ≤20% can be less tax than a HIFO ST documented lot at ordinary rates). This is method choice between two lawful filings, not an understatement — the tranche's LT character is conservatively derived (window-end ⇒ computed-LT implies true-LT), so correct application of the in-force method files the correct tax for that method. G-4 not threatened. Copy tweak only: keep "gain-maximizing," don't imply always tax-maximizing.

**N-3. [Nit] D-8 Path-B refusal copy vs Rev. Proc. 2024-28 irrevocability.** The refusal is tax-safe (nothing filed/understated/inerted). But a real safe-harbor allocation is irrevocable once made; "amend the allocation first" is cleanly available only at the in-app-record level before the real allocation is final. Hedge the message (e.g., "revisit the in-app allocation; if your safe-harbor allocation is already final, unallocated pre-2025 units are a facts-and-circumstances problem for a professional"). Copy-level only.

## Verified sound (new in v2)
- **D-8 transition exemption is tax-clean and compliance-POSITIVE.** In `seed_transition`, Path A keeps basis + `acquired_at`, overwriting only `basis_source`; exempting `EstimatedConservative` changes *only the tag* — the per-wallet position is identical ($0 basis, consistent with the safe harbor's no-allocation-for-unsubstantiated-units), and a later disposal's character derives from the preserved `acquired_at`. Without the exemption the tag is lost and P7's mandatory disclosure silently misses every pre-2025 tranche — so it is REQUIRED for the disclosure guarantee. The KAT pins the right thing.
- **D-9 v1 direction is safe** (N-2): no path files less than the correct tax for the in-force method; the advisory fires whenever a tranche is consumed while a documented lot remains; the FIFO-inversion KAT pins the dependence.
- **D-5 clean export** re-verified (`pseudo_active()` keys on `pseudo_synthetic_count`; a real DeclareTranche never increments it).
- **D-3/P4** re-verified (`is_broker`, `ForbiddenBroker2027`, relief-ends-2026).
- **D-6 "no adjustment code"** correct for I/L and H/K (supplying a missing basis takes no code; code B is for correcting a shown basis).
- **G-2's narrowed claim** (basis unassailable; character/proceeds assailable → G-4) exactly right.

Bottom line: v2 genuinely resolves all four blocking r1 tax findings; the rewrite introduced no tax defect; D-8 exemption and D-9 advisory both err safe. Three copy-level Nits, none gating. GREEN.

Sources: IRS i8949 (live, 2026-07-20); §§1001/1011/1222/1223/6662(d); §170(e)(1)(A); §1014; Rev. Proc. 2024-28; Notices 2025-7/2026-20.
