# Fable review — filing-blocker strategy + adversarial review of the B9/B10/R2 fixes

**Scope:** `git diff c8c3704~1..HEAD -- crates` on `main` @ `8a5097b`. Primary sources quoted from
`design/forms/extract/` and from IRS Pub 505 (fetched: 2024 rev., and the 2018 rev. — the last that
prints the excess-SS worksheet). Suite green at HEAD before review (`make check`: 2553 passed).
All mutations below were made under `cp` backup/restore; **the tree is clean** (only the pre-existing
untracked `reviews/BRIEF-fable-blockers.md` and this report).

---

## STRATEGY

### The ordering — mostly right, one artifact, one missing head

**B2-before-B1 is an artifact of framing.** The claimed dependency (B2 builds the continuation-statement
asset later items need) is real for B2's *successors* — any grid-overflow surface — but B1 consumes none
of it: B1 is two or three collected inputs plus a 40-line transcription (`f8995a--2024.txt` is archived;
line 4 "Allocable share of W-2 wages", line 7 "unadjusted basis immediately after acquisition (UBIA)").
B1 and B2 are independent; run them in either order or in parallel. The orderings that ARE load-bearing:

- **B1 before B3** — correct as argued: dollar-stated gross receipts push every real SE filer above
  $191,950/$383,900 into `QbiAboveThreshold`, so B3 without B1 grows the refusal population.
- **B1's scope must include the SSTB declaration**, and this is why B1-before-B3 is not just about
  volume. Today the code's own reasoning holds every Schedule C to be crypto mining, "not an SSTB"
  (`return_1040.rs:1340`). B3 makes the business arbitrary; above the threshold an unasked SSTB
  question is an *understatement* hole (a consultant's SSTB deduction is $0, and computing the
  wage-limit-only answer overstates the deduction). The checkbox is on Form 8995-A itself, so
  collecting it is transcription, and it is class-(A): unanswered must refuse.
- **The missing head of the queue: the understatement closures.** B11, R1, and this review's Critical
  are each days of work, change the *shipped* product's failure direction, and are the only open items
  whose failure direction is understatement or active misdirection. They go before any input-surface
  widening. R1 concretely: the CTC advisory's "up to $2,000 per qualifying child" figure should pass
  through the §24(b) phase-out arithmetic it already has all the inputs for, and go quiet (or say "$0
  — fully phased out") when AGI provably kills the credit.

So: **(0)** B11 + R1 + the Critical/Important below → **(1)** shared instruments once (the
`Option<Usd>` writer is §G-24 made structural — build it before B1/B2 emit new lines) → **(2)** B2 and
B1, either order, parallel if staffed → **(3)** B3, then B4 → **(4)** B7/TY2025 → **(5)** the owner's
TY2025 comparison run as the acceptance milestone.

### B11 — agree it is underweighted, and the minimal mechanism already exists in this repo

B11 is not a feature; it is the answered-ness invariant at the scope boundary, and every piece of the
fix is already built. The mechanism: **one always-live class-(A) declaration** in the existing
registry (`questions.rs` → `screen_inputs` → `income answer` → `attribute`), a completeness
attestation, not a questionnaire:

> "Other than what you entered above, did you receive any income this year — rental or royalty, farm,
> a partnership/S-corp/trust (Schedule K-1), or anything else this tool did not ask about?"

`None` refuses (unanswered live declaration — the D-8 machinery verbatim); `Some(true)` refuses
(out of scope, exactly like `hsa_activity` and `dual_status_alien`); `Some(false)` is testimony and
files. One enum variant, one question, one `RefuseReason`. Why a refusal and not an advisory: the
failure direction is omitted §61 income — the direction the product promises never to go — and an
advisory lets the packet emit anyway. Why one union question and not per-schedule: the boundary moves
every tax year; the union is stable and the refusal message can enumerate. This is precisely the sharp
test from the testimony doctrine: today the filer's silence *asserts* "no rental income" on a signed
return; after the question, silence *forgoes* filing. It is also what every human preparer asks.

Scope the gate to **income only**. Out-of-scope deductions and credits fail conservative and are
already advised; RRTA (see review below) is a forgone credit and belongs in `LIMITATIONS.md` prose,
not a gate. The gate is for understatement-direction silence exclusively — widening it to every
out-of-scope item is how it becomes the questionnaire nobody finishes.

### What should NOT be built

- **B5 — do not build, three independent reasons.** (1) The standing argument holds: a filer-stated
  USD for a ledger asset is testimony about value and creates a second authority for a number the
  bundled FMV already answers — the §G-9 oracle-input problem in input form. (2) B5's actual pain in
  the trial was *harness* pain (reproducing published answer keys); solve it in `xtask` — a
  satoshi-for-dollars solver or a test-only FMV override — never in the product. (3) B3+B4 dissolve
  the demand: dollar-denominated income enters in dollars, and what remains BTC-denominated is exactly
  what should be dataset-valued.
- **Lot-level securities (the maximal B4).** B4 should be Schedule-D-shaped: per-category totals
  (proceeds/basis/adjustments per box, lines 1a–3/8a–10), which is what the form itself carries and
  what every gold standard states. No wash-sale engine, no second lot ledger. The Bitcoin ledger stays
  the only lot-level engine in the product.
- **RRTA (tier 1 or 2), Schedule E/F/K-1 computation** — B11 is the boundary mechanism for all of it.
- **Per-schedule scope questionnaires** beyond the single B11 attestation.

### Product identity

B3/B4 at the totals level do not make btctax a general preparer in any sense that threatens its
identity — they make the input surface match the *forms'* input surface, which is the "transcribe,
don't paraphrase" rule applied to inputs. A W-2 is typed from a W-2; a 1099-B enters as the Schedule D
category totals the form itself prints. The identity worth defending is: **the only engine that does
lot-level accounting is the Bitcoin ledger; everything else enters exactly as the 1040 constellation
states it.** Under that framing, wide-vs-loud is a false choice: go wide where the form is (B3/B4
totals), and loud where it is not (B11). The decisive fact is that the owner is filer #1 and the
owner's own scenario is inexpressible without B3/B4 — a filing tool its only user cannot use has
answered the identity question in the wrong direction.

### Sequencing against reality

The highest-value act available to this project is the owner's TY2025 comparison — one complete real
return with a known answer key, worth more than any synthetic corpus. Everything is ordered by what
that run needs: B7 for the year, B1/B3/B4 to express the owner's facts, B2 if the household exceeds
four dependents. With the first real filing at TY2026, there is no calendar pressure to ship the wide
surface — but every interim output is already shown to a real person, which is why the understatement
closures (B11, R1, the EIN Critical) are step 0, not residue.

---

## REVIEW

**VERDICT: 1 Critical / 1 Important** (plus 2 Minor, 3 Nit). The core §6413(c) arithmetic, the
per-employer-cap construction, the refusal placement, and the B9 guard are all correct and are held by
tests I observed red under planted defects. The Critical is the one seam the fix left open, and it is
the same seam the fix was shipped to close.

---

SEVERITY: Critical
WHERE: crates/btctax-core/src/tax/return_1040.rs:665-693 (compute); crates/btctax-core/src/tax/return_refuse.rs:791-816 (screen); crates/btctax-input-form/src/spec/sections.rs:649-663 (capture)
CLAIM: The EIN is compared as trimmed free text, so one employer spelled two standard ways counts as "more than one employer" — restoring the exact §6413(c) understatement this commit exists to kill.
FAILURE: Single filer, one employer, two W-2s (a W-2 plus a W-2c, the fix's own motivating example), box 4 = $6,200 each; box b typed `11-1111111` from the paper form on one and `111111111` from a payroll-portal export on the other. Both spellings are the same EIN; `BTreeSet<&str>` holds two entries; the "more than one employer" test passes; **Schedule 3 line 11 = $1,946.80 on a filed return for a filer entitled to $0**. On the trial's MFJ shape the identical collision restores the full $3,894. **Verified live**: a temporary probe test with exactly those inputs PASSED — `excess_social_security` returned `1946.80` (probe added, observed, removed; tree restored).
EVIDENCE: `eins.insert(e)` over `w.ein.as_deref().map(str::trim)` (return_1040.rs:667-669) — set membership of trimmed spellings, with no format validation anywhere: the TUI `set` stores raw text (`w.ein = Some(s).filter(|t| !t.trim().is_empty())`, sections.rs:660) and the TOML path accepts any string. The new field's own doc comment states the governing rule while the code violates it: *"`employer` is free text and is NOT a substitute — nothing reads it, and **two spellings of one employer are two employers to a string compare**"* (return_inputs.rs:55-56) — and §6413(c) is then decided by a string compare of the new field. The house pattern exists and is proven: `Ssn::canonical` (packet.rs:58-73) strips hyphens/whitespace, requires exactly nine digits, and fails loud (`SsnError`); `IpPin::canonical` repeats it. Fix: `Ein::canonical` on that model; canonicalize at both read sites (screen + compute); refuse a malformed EIN at the screen whenever the EIN is demanded (over-cap); and the B1 kill test — same employer, two spellings, asserts `Usd::ZERO`.

---

SEVERITY: Important
WHERE: crates/btctax-core/src/tax/return_refuse.rs:141-144
CLAIM: The doc comment justifying the removed refusal claims "this now yields $0 on Schedule 3 line 11 **and an advisory**" — no such advisory exists anywhere, so the filer is never told their money exists.
FAILURE: One employer erroneously withholds $11,000 of Social Security. Old behavior: a hard refusal whose message said "recover it from the employer". New behavior: the return files (correctly — the credit is genuinely $0) and **no output anywhere tells the filer that ~$547 of their money is recoverable from the employer, or that Form 843 exists if the employer will not adjust**. No wrong number on the return; a known, computed, forgone recovery is silently dropped in a codebase whose own rule is that a conservative omission is permitted *only if the filer is told* (see e.g. `Mfs63fSpouseBoxesForgone`: "§3.4 permits it only if the filer is TOLD").
EVIDENCE: The `Advisory` enum (advisories.rs:43+) has no excess-SS variant; grep for the remedy text finds only code comments. The instruction's remedy sentences — `i1040gi--2024.txt` (the Line 11 block, ~43325-43342): *"The employer should adjust the tax for you. **If the employer doesn't adjust the overcollection, you can file a claim for refund using Form 843.** Figure this amount separately for you and your spouse."* — are exactly where the code's block-quote transcription stops early (`return_1040.rs` doc ends the quote at "…adjust the tax for you."), and the Form 843 sentence is conveyed nowhere on the non-refusing path. Fix: an `ExcessSsSingleEmployerNotCreditable { amount }` advisory carrying the per-person over-cap amount and the Form 843 remedy (the fix consistent with the codebase's own doctrine); merely correcting the false comment is the lesser repair.

---

SEVERITY: Minor
WHERE: crates/btctax-cli/LIMITATIONS.md:194
CLAIM: The shipped `btctax limitations` output still lists the removed refusal as live and does not document the new one.
FAILURE: A filer reads "**A single employer over-withholding Social Security** (not creditable — recover it from the employer)" under "(ii) REFUSALS — v1 stops rather than guess" and expects a refusal that no longer happens; the actual new refusal (`ExcessSsEmployerUnknown` — over the cap with a missing EIN) is undocumented. The B10 taxonomy change missed this emitter (the whole-surface-sweep rule).
EVIDENCE: LIMITATIONS.md:194 vs. return_refuse.rs:141 ("★★ NO LONGER RAISED"). When fixing, this is also the right place to name the RRTA silence (below).

---

SEVERITY: Minor
WHERE: crates/btctax-core/src/tax/line_coverage.rs:973-975
CLAIM: The Schedule 3 line 11 coverage Exception still records the PRE-FIX formula as the line's provenance.
FAILURE: The census rationale reads "(per person, max(0, Σ W-2 box 4 − 6.2% × wage base))" — the exact naive computation commit 18c9980 replaced, with no mention of the per-employer cap or the EIN gate. The coverage table — the instrument that exists to make provenance greppable — now documents the defective arithmetic as current.
EVIDENCE: line_coverage.rs:975 vs. return_1040.rs:683-694 (`per_employer.min(max)` summed, then `(creditable - max).max(ZERO)`).

---

SEVERITY: Nit
WHERE: crates/btctax-core/src/tax/return_refuse.rs:772
CLAIM: The block-header comment still lists "single-employer excess SS" among the checks that follow; that guard was removed in this diff.

SEVERITY: Nit
WHERE: crates/btctax-core/src/tax/return_1040.rs:677-680
CLAIM: `if eins.len() < 2 { return Usd::ZERO; }` is semantically dead — provably subsumed by the per-employer cap (Σ over one employer of `min(w, max)` ≤ `max`, so the capped aggregate minus `max` is already ≤ 0) — and its comment ("the first condition, and the one that was missing") credits the defect kill to a line that decides nothing. Verified: removing it leaves **all 954 btctax-core tests green** (an equivalent mutant, not a test gap — the one-employer→$0 guarantee is held by the capping line and observed red under its mutation). Harmless as executable documentation; the comment should say the cap subsumes it so a future refactor deletes the right line.

SEVERITY: Nit
WHERE: crates/btctax-cli/src/cmd/admin.rs:429
CLAIM: For a mixed `--forms full-return,f8949` on a crypto-only year, the refusal's remedy ("drop --forms to export the crypto slice") overshoots — the precise remedy is dropping only `full-return`. Refusing is the right behavior; `.contains` fires regardless of the other members.

---

### Adjudications and verifications (the brief's attack list, answered)

**The per-employer-cap reading is CORRECT — adjudicated against the IRS's own worksheet.** The 2024
Pub 505 no longer prints the worksheet (fetched and checked), so I fetched the last revision that does
(2018), Worksheet 3-1, "Excess Social Security—Nonrailroad Employees": *"1. Add all social security
tax withheld **(but not more than $7,886.40 for each employer)** … 4. Social security limit …
5. Excess. Subtract line 4 from line 3."* — mechanically the implemented construction, and it makes
the single-employer case $0 by arithmetic. The alternative reading ("any one employer over the cap ⇒
no credit at all") contradicts the worksheet and the statute's structure: an employer's own
overcollection is §6413(a)/(b) territory (employer adjustment, else Form 843), while §6413(c)'s
multi-employer credit is granted as of right and survives it. Note also Worksheet 3-1 line 2
(uncollected SS on tips/GTLI, box 12 codes A/M) is unreachable here: non-inert box-12 codes refuse
(`INERT_BOX12_CODES`, return_refuse.rs:26), so the implemented two-term form is the *complete*
worksheet for every return btctax admits.

**RRTA:** not modeled anywhere — no box 14 input, no railroad question. Schedule 3 line 11's own
caption covers "tier 1 RRTA", so a railroad filer's excess tier-1 credit is silently forgone. Same
*class* as B11 (unasked scope), but the direction is overstatement (a forgone credit), so prose in
LIMITATIONS.md suffices; no gate, no build.

**The `None → Usd::ZERO` fallback is not reachable in production.** Every path to
`assemble_absolute` runs `screen_inputs` first: resolve.rs:96, input_form_store.rs:317,
cmd/admin.rs:794, cmd/answer.rs:344-363, oracle-harness main.rs:746; the packet adds
`ReturnHeader::build` as a second boundary for declarations. If a future caller bypasses the screen,
ZERO *overstates* — the tolerated direction — though it would silently deny a legitimate credit.

**MFS / no MFJ assumption.** `per_person` runs per owner; a spouse-tagged W-2 on a non-joint return
refuses upstream (`SpouseOwnerWithoutJointReturn`, return_refuse.rs:751). The extract's last sentence
— "Figure this amount separately for you and your spouse" — is implemented (never pooled).

**Does the new refusal strand anyone?** Over the cap with an unobtainable EIN refuses the return. Box
b is on every W-2 by construction, so the practically-stranded set is empty, and both guesses are
worse (one fabricates a credit, the other silently denies one). One over-refusal edge exists: an
EIN-less W-2 with box 4 = $0 trips the screen although its identity cannot move the credit — harmless,
remedy stated. Right trade.

**The tests red on the defects they name — observed, not assumed** (all under cp backup/restore):
- M1, the shipped naive credit reintroduced → `excess_social_security_per_person_not_pooled` FAILED at
  the `same_employer` assertion: `left: 1546.800, right: 0`.
- M2, `per_employer.min(max)` removed → same test FAILED at `one_over_cap`: `left: 7546.800, right: 6000`.
- M3, the screen block deleted → `excess_ss_refuses_only_when_employer_identity_is_unknown` FAILED:
  `left: None, right: Some(ExcessSsEmployerUnknown)`.
- M4, `eins.len() < 2` removed → survives all 954 core tests (equivalent mutant; see Nit above).
- The old test's blindness (a "two employers" comment over fixtures carrying no identity) is genuinely
  closed: the fixtures now state EINs, and the same-EIN counter-case exists and was observed killing M1.

**B9.** Guard placement verified: after the full-return dispatch (so a full-return year *honors* the
selection; `forms_ignored_full_return = !forms.is_empty() && any(≠ FullReturn)` is honest in all three
shapes — alone/none/mixed) and before any crypto-slice byte (before `promote_export_gate`, the
attestation gate, and `mkdir_out`). All entry points route through `export_irs_pdf_from_session`
(CLI main.rs:718; the chokepoint has no PDF export path). Mixed `full-return,f8949` on a crypto-only
year refuses via `.contains` irrespective of other members. The committed test's kill is structurally
forced: without the guard, `wants()` matches nothing, the export returns `Ok` having written nothing,
and both `expect_err` and `wrote_nothing()` fail.

**Tree state:** both mutated files restored byte-identically from backups; post-restore targeted run
6/6 green; `git status` shows only the pre-existing untracked brief and this report. No commits made.

---

### WHAT WOULD MAKE THIS REVIEW WRONG:

- **The worksheet adjudication rests on a retired document.** The 2024 i1040gi prints no worksheet and
  the 2024/2026 Pub 505 revisions have dropped the chapter; I adjudicated the per-employer cap on the
  2018 Worksheet 3-1 plus §6413's structure. If a current-year IRS worksheet exists somewhere I did not
  look (e.g., a Pub 17 revision) with different mechanics, that adjudication must be re-run against it.
- **The Critical presumes a filer can plausibly render one EIN two ways.** If capture were normalized
  (it is not, today — verified in both the TUI setter and the TOML path), the finding collapses to a
  missing-validation Nit. I judged the hyphen/bare split plausible because the paper W-2 prints the
  hyphenated form and payroll exports commonly do not.
- **I did not drive the TUI end-to-end for `W2Ein`** (spec/coverage/seam wiring and unit tests only);
  a TUI-level storage defect would be invisible here.
- **Fallback reachability was established by enumerating callers at this commit**; a future
  screen-skipping caller changes that answer (in the conservative direction).
- **The strategy half leans on the trial's factual claims** (verified only where they intersected the
  diff — e.g., `QbiInputs` really lacks wage/UBIA fields, the 1040 really has the 4-row box and the
  i1040gi statement remedy at :1452). If the owner's real TY2025 facts differ materially from Scenario
  A, the B3/B4 priority — though not their design — should be re-weighed.
