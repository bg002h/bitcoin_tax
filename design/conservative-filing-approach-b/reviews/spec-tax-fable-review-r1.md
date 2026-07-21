# Independent TAX review — Approach B sub-project 1 SPEC, round 1

**Artifact:** `design/conservative-filing-approach-b/SPEC.md` (DRAFT, author-written)
**Lens:** US federal tax correctness / completeness / honesty. Adversarial; every load-bearing claim
verified against statute/reg/case law AND against current source (`crates/btctax-core/src/conservative.rs`,
`project/{resolve,fold,transition,pools}.rs`, `forms.rs`, `btctax-cli/src/cmd/tranche.rs`, `btctax-forms/`).
**Provenance honored:** `reviews/DESIGN_PROVENANCE.md` read; folded Opus/adjudication points were
*re-verified*, not re-raised. Where the adjudication's claims check out in source, that is recorded in §V
below so the gate has a positive record, not just defects.
**Reviewer:** Fable (independent; not the spec author of record for this round).
**Date:** 2026-07-21.

**Verdict: NOT green. 1 Critical / 4 Important / 7 Minor / 4 Nit.**

---

## C — CRITICAL

### C-1 (BG-D1 / BG-D7 / BG-D8 / §3 sweep) — The promoted floor silently reaches the FILED Form 8283 / Schedule A surface through §170(e), ungated, unconsented, undisclosed, and under a penalty regime the copy misstates.

**Defect (one sentence):** The fold computes the §170(e)(1)(A) short-term charitable-donation deduction
as `min(FMV, leg.basis)` (`fold.rs:1231-1240`), so a promoted tranche's estimated floor — today $0 by the
parent P1 guarantee (`forms.rs:266-268`: "an ST-held tranche donation → deduction limited to basis =
**$0**") — becomes a `>$0` charitable deduction derived from the estimate, on a filed surface that none of
BG-D6 (consent), BG-D7 (the 8275 is 8949-scoped: "item = Form 8949 col (e)"), or BG-D8 (the export gate
keys on "a promoted **leg**", i.e. a *disposal* leg — a donation is a `Removal`) covers.

**Failure scenario (filer facts → wrong outcome):** Cash P2P buyer acquires 1 BTC in a tight 2025 window,
no records; declares the tranche, promotes to a ~$60k window-min floor (consent screen quantifies only the
8949 gain reduction); within a year of `window_end` donates the BTC (ST) to a public charity. The engine
files Form 8283 / Schedule A with a claimed deduction of `min(FMV, floor)` ≈ $60k — an estimated-basis
deduction — with **no** Form 8275 describing it, **no** consent line quantifying it, and **no** export-time
gap. On exam, the correct basis is determined $0 (*Vanicek* — no evidentiary predicate accepted):
Reg §1.6662-5(g) makes any positive claimed basis an automatic **gross** valuation misstatement, and
because this is **charitable deduction property**, §6664(c)(3) removes the reasonable-cause defense for the
gross case entirely (and conditions the substantial case on a qualified appraisal + good-faith
investigation) — so the spec's BG-3/BG-D10 promise that "the 8275 and good-faith methodology mitigate" is
**false on this surface**. The filer takes a 40% strict-exposure position they never knowingly chose —
BG-1 ("never understate — as a KNOWING choice, enforced structurally") is unmet.

**Authority violated:** §170(e)(1)(A); §6662(e)(1)(A)/(h); Reg §1.6662-5(g); **§6664(c)(3)** (no
reasonable-cause for gross valuation overstatement w.r.t. charitable deduction property); BG-1/G-4 (the
spec's own structural guarantee).

**Note on reachability:** the window is narrow (ST requires donation within 1 year of `window_end`, so a
recently-windowed tranche) but fully reachable and completely silent. The LT donation path is clean
(deduction = FMV, basis uninvolved). Gift removals also carry the floor outward to a donee's carryover
records (`rehome_onto_removal_leg`, `fold.rs:303-307`) — lesser, note in the fix.

**Fix directions (any one closes it; pick in the spec, not the plan):**
1. **Estimate never funds a deduction** (cleanest, matches the parent Invariant's spirit): the §170(e)
   ST-donation deduction for a promoted-tranche leg stays limited to the *documented* component
   (i.e. $0 absent a fee carry) — with copy explaining why; or
2. Hard-refuse / loudly gate donating promoted units while ST; or
3. Extend BG-D6/BG-D7/BG-D8 to removal legs (consent quantifies the deduction, the 8275 Part I covers the
   Schedule A item, the packet gate counts removal legs) **and** add the §6664(c)(3) strict-penalty copy.
Whichever way: fix `forms.rs:266-268`'s "$0" sentence and add the donation case to §3's sweep and §6's KATs.

---

## I — IMPORTANT

### I-1 (BG-D4) — The clamp's decomposition operand is wrong: "the lot's `usd_basis` share" is NOT the estimate component once a TP8(c) self-transfer fee carry has landed IN the lot.

**Defect:** BG-D4 says the leg builder "decomposes basis into the estimate component (the lot's
`usd_basis` share) vs. documented components (fee carry applied after)" — but "applied after" is true only
for the *disposal-time* carry (`rehome_onto_disposal_leg`, `fold.rs:653-654`); the **self-transfer** fee
carry re-homes documented fee basis *into the surviving lot's `usd_basis`* (`rehome_onto_lot`,
`fold.rs:844-845` / `fold.rs:291-301`), where it is indistinguishable from the estimate.

**Failure scenario:** Filer promotes a tranche to a $46k floor, then follows the P8 nudge and self-transfers
it Exchange → SelfCustody paying an on-chain sat fee; the relocated tranche keeps its tag
(`fold.rs:816-820`) and its `usd_basis` becomes `floor + documented_fee_carry`. On later disposal below the
window low, the clamp as written (`min(usd_basis_share, net)`) clamps the **documented** fee component too —
suppressing exactly the documented-loss corner the spec's own amended invariant preserves ("documented
components stay UNCLAMPED... any negative gain remains attributable solely to documented fee/rounding"),
and the §6 KAT "the documented fee corners still reach negative (attribution intact)" goes red or gets
written vacuously. The P7 disclosure sentence rewrite (§3 item 1) also mis-attributes.

**Not an understatement** (clamping documented basis is conservative), but the design of record contradicts
its own invariant in a corner the product itself steers filers into; an implementer can ship either side.

**Fix:** the estimate component of a leg = the **stored `filed_basis`** pro-rated over the tranche's sats
(`filed_basis × leg.sat / tranche_sat`), keyed via `leg.lot_id.origin_event_id` → the promote set (the spec
already threads the promote set into the builder); documented = `usd_basis_share − estimate_share`,
unclamped. State it in BG-D4; pin it with a relocated-with-fee-then-promoted-sale KAT.

### I-2 (BG-D6) — The two-sided consent quantification is UNDEFINED for the promote-before-disposal flow and, as specced, displays ~$0 saving AND ~$0 exposure for an unrealized floor.

**Defect:** BG-D6 quantifies both sides via `overpayment_delta_one` "with reference = the floor", but that
seam is **year-scoped to realized disposals** (`conservative.rs:284-322`: it re-folds and diffs
`compute_tax_year` for one `year`); for a tranche not yet disposed — the planning flow the spec itself
sells (promote, then hold/sell) — `tax($0) − tax(floor) = $0` in every year, and the spec never says which
year(s) the consent screen runs over (a tranche disposed across multiple years needs a Σ, not one year).

**Failure scenario:** Filer declares a 2017-window tranche still held, runs `promote-tranche`; the consent
screen — the load-bearing BG-1 informed-consent artifact — quantifies "saving ~$0 / at-risk tax ~$0" for a
position whose eventual exposure is five figures. The filer "consents" to numbers that are false in both
directions; the recorded acknowledgment then *documents* that the product understated the risk it disclosed.

**Authority/guarantee violated:** BG-1's "explicit two-sided informed-consent acknowledgment that states
both the saving and the penalty exposure" — unmet for the undisposed case; §6664(c) good-faith posture is
undermined by a recorded consent quoting wrong figures.

**Fix:** define the consent quantification in the spec: Σ of per-year deltas over all years with disposed
tranche legs, PLUS for undisposed sats either (a) an explicit "unrealized — saving and exposure accrue at
disposal; at today's price the floor would reduce reported gain by ~$X (hypothetical, not a filed figure)"
line, or (b) a hard statement that no dollar figure is shown because none is realized. Never a bare $0.

### I-3 (BG-D6 / §3-item-2 vs BG-D4) — The reused what-if seam is UNCLAMPED, so the consent screen (and the promote-funnel nudge) quote a saving the promote cannot deliver whenever the clamp binds.

**Defect:** `overpayment_delta_one` swaps `usd_cost` and re-folds with **no loss clamp** (there is no
promote event in the what-if, so the promote-set-keyed clamp will not engage): the with-floor scenario can
claim a loss the filed promote is structurally forbidden to claim (parent §3: "never claim a loss off an
estimated basis"), understating `tax(floor)` and **overstating the displayed saving** — and, since BG-D6
sets exposure = the same delta, overstating the quantified at-risk tax too.

**Failure scenario:** Tranche window-min $12k; the filer sold the BTC at $8k. True promoted outcome: gain
clamped to $0 (basis = proceeds), saving = tax on $8k of gain. Consent screen (unclamped swap): with-floor
scenario files a $4k **loss**, so the displayed saving includes tax relief the promote will never file. The
filer consents to, and the funnel line (§3 item 2: existing reconstruction nudge + "`reconcile
promote-tranche …`") advertises, a number only *true reconstruction* (documented records, loss legitimately
claimable — which is why v1's P6 is NOT wrong today) could deliver.

**Fix:** BG-D6's saving/exposure MUST re-fold through the promoted path *including the BG-D4 clamp* (thread
a synthetic promote set into the what-if); §3 item 2's unpromoted-tranche funnel line must either quote the
clamped promote delta alongside the reconstruction delta or state that the promote's saving can be lower
(sale below the window low). Add a KAT: consent delta on a below-window-low sale equals the clamped saving.

### I-4 (BG-D9) — The prior-year-delta advisory triggers on the wrong predicate: under HIFO (the DEFAULT method), promoting an UNDISPOSED tranche can silently rewrite already-filed years.

**Defect:** BG-D9 fires the 1040-X advisory only for "a promote over an already-DISPOSED tranche", but the
promote re-folds full history with the lot at floor basis from inception, and `hifo_cmp` orders by per-sat
`usd_basis` (`pools.rs:272-276`) — so a promoted floor-per-sat above a documented lot's per-sat basis makes
**prior years' disposals retroactively draw the tranche instead of the documented lot**, changing filed
years with no advisory at all. HIFO is the forward default (`project/mod.rs:55,197`), so this is the
default configuration, not an exotic one.

**Failure scenario:** HIFO wallet. 2025 sale drew a documented lot bought at $25k/BTC; the $0 tranche
(2021 window) sat unsold and sorted last. 2026: filer promotes the still-undisposed tranche to its ~$46k
window-min floor → the re-fold's 2025 HIFO draw now picks the tranche ($46k/sat > $25k/sat); the books'
2025 legs no longer match the filed 2025 8949, **no advisory fires** (target was never "disposed" at
promote time under the old fold), and the 2026 filing then draws the documented $25k lot the filed 2025
return already reported sold — **documented basis double-counted across filed years, a real later-year
understatement**, reached silently.

**Authority violated:** G-4 / BG-1 (silent understatement path); §6662(d) exposure created without the
knowing-choice machinery.

**Fix:** trigger the advisory on the correct predicate — "any year `< current` whose computed tax changes
under the promote" (diff `tax_total` per year pre/post promote; the machinery exists) — which also covers
partially-disposed tranches ("already-DISPOSED" should read "any tranche with disposed legs"); and make the
1040-X copy conditional ("if year Y was already filed…" — the engine has no filed-year concept, so the copy
must not assert an amendment is required for a not-yet-filed current year). Pin with a HIFO-reorder KAT.

---

## M — MINOR

### M-1 (§3) — The enumerated semantic sweep misses user-visible "$0"-assuming copy sites, while claiming "all verified against current source".
Missed: `btctax-cli/src/cmd/tranche.rs:30` (`TRANCHE_IS_FINAL_HINT`: "if you have already filed the
tranche's **$0 basis**" — post-promote the filed basis is the floor), `cmd/tranche.rs:95` ("($0
EstimatedConservative) is on file"), `cmd/tranche.rs:157-158` (phantom-wallet warning: "it still files at
**$0**" — false for a promoted tranche), `resolve.rs:1312` (`SafeHarborUnconservable` blocker detail: "a
conservative-filing tranche (**$0** EstimatedConservative) remains…"), and `forms.rs:266-268` (subsumed by
C-1). The whole-surface-sweep rule (project memory: taxonomy changes took 4 review rounds when swept
piecemeal) requires these enumerated in §3 now, not found at implementation time.

### M-2 (BG-D4) — `min(floor_share, net_proceeds_share)` files a NEGATIVE basis when the last-leg cent-scale remainder is negative.
The parent Invariant KAT (c) documents that pro-rata remainder rounding can make a last leg's
`net_proceeds_share` negative at cent scale (`make_disposal_legs`, `fold.rs:134-135`); `min(floor, −$0.01)`
= `−$0.01` → a negative amount in the 8949 col (e) position (vanishes at whole-dollar rounding, but the
internal/CSV surface shows it, and the rounding-negative silently migrates from "documented rounding gain"
into "negative estimate basis", breaking the attribution story). Formulate as
`clamp(net_proceeds_share, $0, floor_share)` — i.e. `min(floor_share, max(net_share, $0))` — so basis
stays `≥ $0` and the cent-scale negative remains in gain, attributed to documented rounding.

### M-3 (BG-D7/BG-D10) — Penalty-base copy imprecision: "20% / 40% on the disallowed portion" reads as a penalty on the disallowed BASIS.
§6662(a)/(e)(2): the penalty is 20%/40% of the portion of the **underpayment** (the additional tax)
attributable to the misstatement, and the $5k threshold is likewise an underpayment-attributable test
(Reg §1.6662-5(b)). A filer reading "40% on the disallowed portion" against a $60k disallowed basis fears
$24k; the true worst case is 40% of the ~tax-delta. The consent/8275 copy must name the base ("of the
resulting additional tax"). Honesty cuts both ways — overstating the penalty is also a copy defect.

### M-4 (BG-D7) — The 8275 position framing is inaccurate for a CLAMPED leg.
The disclosure says "basis estimated at the minimum daily closing price over the attested acquisition
window", but on a below-window-low sale the FILED basis is the lower clamped amount (= net proceeds).
Reg §1.6662-4(f) adequacy wants the relevant facts in sufficient detail; a filed amount that differs from
the disclosed method invites an examiner mismatch. Add one sentence to the generated narrative when the
clamp applied: "limited so as not to report a loss from the estimate."

### M-5 (BG-D9) — The 1040-X advisory omits §6511, and the VOID direction gets no advisory at all.
(a) A promote over a 2019-disposed tranche invites a refund claim that §6511(a) (3 years from filing / 2
from payment) has likely time-barred — the advisory should say so. (b) Symmetric gap: **voiding** a promote
over a disposed tranche (e.g. the filer realizes the attestation was wrong) reverts the books to $0 while
the filed return still claims the floor — an amend-to-PAY situation that needs the same loud prior-year
advisory; as specced only the promote direction warns. (Subsumed by I-4's any-year-delta trigger if that
fix is taken for voids too — say so explicitly in BG-D9.)

### M-6 (BG-D5) — The attestation's negative enumeration omits mined / earned / airdrop / fork provenance.
The affirmative clause ("acquired by purchase within the declared window") is the operative control and
does exclude them, but the enumerated negatives ("not by gift, inheritance, or as unreported income")
invite expressio-unius misreading: a miner who reported the income truthfully finds none of the three
negatives applicable and may self-certify "purchase-ish" (electricity/effort). Basis for mined/earned/
airdrop/fork coins is FMV-at-receipt income basis (Notice 2014-21; Rev. Rul. 2019-24), documented from the
return — the real-acquisition path. Extend the enumeration ("…not mined, earned, or received via
airdrop/fork — if acquired other than by purchase, model the real acquisition") or close with "not acquired
in any other way".

### M-7 (§6) — No KAT pins term/holding-period invariance under promote.
The whole BG-D9 character claim ("no silent LT/ST flip") rests on the promote rewriting ONLY `usd_cost`
(acquired_at stays `window_end` via the Eff date). That is true by construction today, but nothing in the
§6 KAT list pins `acquired_at`/`term` invariance across a promote — the exact mutation (a promote that
also nudges the date or term) would survive the listed KATs. One assertion added to the BG-D1 promote-fold
KAT closes it (cf. the untested-guard memory: a guarantee isn't held until the mutation dies).

---

## N — NIT

- **N-1 (§1):** "a 'Q4-2017, ~$12k window-min' tranche" is internally inconsistent — the min daily close
  over Q4-2017 is ≈$4.2k (Oct 1, 2017); ~$12k fits a *December*-2017 window. Fix before the narrative seeds
  product/wizard copy (the five-figure-delta point survives either way).
- **N-2 (BG-D5):** `provenance_attested: bool` records that an attestation happened but not WHAT was
  attested; store the attestation text/version alongside (BG-D6 already records typed consent — mirror it).
  The recorded artifact is the §6664(c) good-faith evidence the design leans on.
- **N-3 (BG-D3):** make the verify drift advisory direction-aware: stored floor ABOVE the recomputed
  reference on a not-yet-filed year deserves a "void + re-promote to the corrected number" hint; drift on a
  filed year is where "advisory only" is right.
- **N-4 (§1):** "on-chain receipt timestamps are permanent evidence" — a receipt bounds only the window
  **end** (an exchange purchase precedes the withdrawal); the copy (and later the wizard) should say the
  filer must set `window_start` early enough to cover the actual purchase, which honestly widens the window
  and lowers the floor.

---

## V — Verified correct (checked adversarially; record for the gate)

- **§6662(d) return-wide claim (BG-D7):** correct — §6662(d)(1)(A) measures the understatement against the
  tax required to be shown on the return; the ≥10%/$5k threshold is unknowable at promote time. Mandatory
  8275 is the right ruling; the Opus threshold objection was correctly overruled.
- **Form 8275, not 8275-R (BG-D7):** correct — a *Cohan* basis estimate contradicts no regulation;
  8275-R is for reg-contrary positions (Reg §1.6662-4(f)(1)).
- **"Never say safe harbor" copy rule (BG-3):** tax-correct and appropriately conservative. The real
  statutory mechanism (§6662(d)(2)(B)(ii): adequately disclosed items with a reasonable basis are excluded
  from the understatement) is *contingent* on reasonable basis being sustained — exactly what an exam
  contests — so refusing to promise it as a "safe harbor" is honest, and §6664(c) + Reg §1.6662-3(b)(1)
  (a reasonable-basis position is not negligence) are correctly identified as the dependable footing.
- **BG-D10 §6662(e)/(h) statement:** correct in every particular checked — Reg §1.6662-5(g) (correct
  basis $0 ⇒ any positive claim is deemed a gross misstatement), *United States v. Woods*, 571 U.S. 31
  (2013) (penalty applies to total basis disallowance; also why disclosure is no shield), the >$5,000
  attributable-underpayment limitation (Reg §1.6662-5(b)), and Reg §1.6662-5(a) (no disclosure exception
  for (b)(3)). C-1's charitable carve-out (§6664(c)(3)) is the one surface where this framing breaks.
- **Cohan/Vanicek footing (BG-3/§1):** citations correct (*Cohan*, 39 F.2d 540 (2d Cir. 1930); *Vanicek*,
  85 T.C. 731, 742-43 (1985)); a window-min-close for an attested-purchase filer is a genuine
  bearing-heavily estimate with an evidentiary predicate, and the honest limit (wide window → trivial
  floor → file $0) is stated. The close-vs-intraday-low caveat is correctly carried into the 8275 copy.
- **BG-D2 drops:** §1014 OUT is right (statutory DoD-FMV basis + §1223(9) auto-LT contradict the
  estimate/`window_end` frame; routing it here would falsify the attestation and break BG-2);
  PartialRecords OUT is right (substantiated basis = the documented import, no 8275 needed).
- **BG-D3:** computed-not-chosen + stored-at-record-time is sound as-filed hygiene; `Coverage::Full`
  HARD-required is correct (a Partial covered-part min can exceed the true window min → overstate basis,
  G-4); refusing `Partial`/`None` rather than caveating is the right call for a *filed* number.
- **BG-D1 by-construction claims — verified TRUE in source:** the D-8 backstop keys on the TAG, not $0
  (`transition.rs:76-80`), so a promoted pre-2025 tranche still denies safe-harbor effectiveness; the
  Path-A seed exemption is tag-keyed (`transition.rs:97`); the relocation carve is tag-keyed
  (`fold.rs:816-820`); `hifo_cmp`'s `usd_basis == 0` special-case (`pools.rs:275-276`) means a promoted
  lot sorts by real per-sat basis — HIFO-on-as-filed-basis, correctly characterized (but see I-4 for the
  retroactive consequence the spec missed). The adjudication's no-new-identity ruling is the right
  architecture for tax-guarantee preservation.
- **BG-D4 core direction:** `min(floor_share, net_proceeds_share)` with evaporation is the correct
  never-understate shape (basis forfeiture is always conservative; per-leg clamping is at least as
  conservative as per-disposal); the reported-basis≠pool-basis precedent already exists in the §1015
  NoGainNoLoss zone (`fold.rs:181-190`), so the mechanism is implementable as claimed. Defects are the
  operand (I-1) and the negative-remainder corner (M-2), not the concept.
- **BG-D9 character/holding:** the promote rewrites `usd_cost` only; `acquired_at`/term derive from the
  unchanged Eff date (`resolve.rs:1101-1113`) — no LT/ST flip (pin per M-7). §1091 wash sales are
  inapplicable (BTC is not stock or securities under current law, and the clamp forbids estimate losses
  anyway). The 1040-X refund path is legitimate (subject to M-5's §6511 note); an 8275 attached to the
  amended return taking the position is adequate disclosure for that position.
- **G-4 / parent D-7 re-scope:** sound — "nothing >$0 ever filed" re-scoped to UNPROMOTED tranches keeps
  the invariant meaningful, and the promote path is the sanctioned, gated, disclosed >$0 route.
- **Phasing (§4):** correct on the law — Reg §1.6662-4(f)(1) makes a properly completed Form 8275 (or the
  annual Rev. Proc. return-disclosure) the only adequate-disclosure vehicles; a plain-paper memo has no
  §6662(d) effect, so refusing a text-only standalone MVP is right (Opus correctly overruled).
- **Clean export / no watermark:** consistent with the standing full-return DRAFT-gate policy (user
  mandate: attestation/DRAFT stays pseudo-only).

## Disposition

The single Critical is a genuine unmet-guarantee hole (a >$0 estimate reaching a filed surface with no
gate), not a copy quibble — it needs a BG-level decision (estimate-never-funds-a-deduction is the cleanest)
plus §3/§6 coverage. The four Importants are each fixable inside the existing decision structure (an
operand, two quantification definitions, a trigger predicate). Nothing found undermines the adjudicated
architecture (BG-D1); on the contrary, its by-construction claims all verified. Re-review after fold.
