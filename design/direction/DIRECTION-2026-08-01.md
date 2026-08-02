# ⚠️ AMENDED (2) 2026-08-01 — TY2025 IS AN ORACLE, NOT A DEADLINE

> **"I am filer number one. We will first complete 2025 so I can compare btctax output to my real 2025
> tax return. We can also check prior years if needed. Ideally first file would be 2026 or 2027, but
> tax tables aren't out yet."**

This resolves the last fork and **reframes TY2025 in a way no reviewer considered.**

## What it settles

**Filer #1 is the owner.** So intake is effectively done — the adapters were built against these very
exports — and `data-in`'s "locked door" is a Milestone-2 problem, not a Milestone-1 one. Form/fact
coverage outranks venue breadth.

**TY2025 is not a filing target. It is ORACLE ACQUISITION.** The owner holds a real, prepared,
known-correct TY2025 return. That is a **third independent witness** — and a better one than either
existing oracle, because:

- OpenTaxSolver and Tax-Calculator validate **synthetic corpora**; the real return validates the
  **actual data path** — intake, reconciliation, interview, screens, emission — on one real household.
- It covers **the fact patterns the owner actually has**, replacing the demographic guessing the
  synthesis itself flagged (*"the frequency claims … are demographic reasoning, not measurement"*).
  For filer #1, the real return **is** the measurement.
- ★★ **It is a direct test of §G-11's thesis.** A prepared return carries blanks exactly where the form
  says blank. Every fabricated `0` btctax prints shows up as a diff, on real paper. No synthetic
  fixture can produce that evidence.

**First FILING is TY2026 or TY2027** — so there is real runway, and no statutory clock on anything
before it. ★ Note TY2026 is *also* fail-closed, and partly for reasons outside our control: the 2026
Form 6251 instructions were unpublished as of 2026-07-29 and **no oracle covers 2026**. That external
dependency should be tracked, because it gates the first real filing.

## The metric to optimize: TIME-TO-FIRST-DIFF

Everything downstream re-ranks on what the comparison finds. Building form coverage *before* the diff
is guessing; building it *after* is reading a list. **Get to the diff on the shortest honest path.**

★ "Honest" is doing work here. The TY2025 gate documents its own four exit conditions and warns that
**a partial landing is not a smaller version of TY2025 — it is a silently wrong return.** A diff
against plausible-wrong numbers is worse than no diff: it either falsely validates, or sends us chasing
phantoms. So all four conditions land before the comparison means anything.

## The revised critical path

**0. Ship what exists.** Unchanged, still first, still hours of owner action: push, `.pii-patterns`,
revoke the token, publish v0.15.0. 99 unpushed commits is loss-on-disaster, not caution.

**1. Complete TY2025 — all four gate conditions** (`tax_tables.rs:789-812`): every `FullReturnParams`
field with a per-field citation (★ `std_deduction` and `salt_cap` are OBBBA, *not* Rev. Proc. 2024-40);
the §164(b) SALT **worksheet**, not a scalar; Form 6251 Part I re-transcribed for 2025 (1a/1b, line 2a
citing 1040 line 12e, line 4 combining **1b** through 3) plus the 1040's 11a/11b/12e/13a/13b reshape;
and Schedule 1-A, all six parts. This is the B3 track. ★ **Its plan r3 was never independently
reviewed** — `design/ty2025/reviews/` holds only r1 — and the gate's own doc is the argument for fixing
that first.

**2. Build the real-return comparison harness.** Same shape as the existing oracle diff: a per-line
expected vector, diffed against btctax's computed and *printed* output.
★★ **The vector holds the owner's real financial data and MUST NOT be committed.** It lives untracked
(same posture as `.pii-patterns`); the harness reads it when present and skips cleanly when absent, so
CI is unaffected. Only the derived findings — which lines diverged and why — get committed.

**3. Run the diff, and let it re-rank.** Its output becomes the breadth backlog, replacing the
reviewers' demographic guesses with this filer's actual facts.

**4. §G-11 P0b/P1.** Independent of the above and can run in parallel or after; the "free before users"
window is open and now uncontested. ★ The diff may also *price* it — every fabricated zero appears
there as a real divergence.

**5. TY2026** for the actual first filing, tracking the external blockers (6251 instructions, oracle
coverage).

**6. Milestone 2 — a stranger files.** Intake breadth, the TUI's two missing ends, more statuses.

## What this demotes

The cheap-refusal conversions (Form 6251 attach, Schedule 8812/CTC, box-12, 8889, 1099-B) drop from
"do next" to **"do what the diff says"**. They may all be needed — or the owner's return may not have a
child, an HSA, or a brokerage account, and three of the five would be speculative work. **The diff
costs less than guessing wrong.** The two exceptions that stay above the diff, because they are cheap
and unconditional: **§G-24** (a sworn `$0` on the refund/owe lines, already found) and the
**bottom-of-the-1040 witnesses** (those lines have zero oracle coverage and have already shipped a
defect — and they are precisely the lines the diff will compare first).

---

# ⚠️ AMENDED 2026-08-01 by the owner — READ THIS FIRST

> **"The current season is irrelevant. We will support all seasons eventually."**

This fires **falsifier #1** in §6 below, which the synthesis itself anticipated:

> *"The owner wants a current-season filer soon. **Falsified by:** the owner declaring TY2024 a
> deliberate frozen proving ground … Then the Oct-15 urgency evaporates, the G-11 migration's 'free
> before users' window becomes the defensible next move, and Steps 1–2 lose their deadline (not their
> ordering)."*

## What this changes

**§1's opening sentence is wrong as a criticism.** "The only supported year is TY2024" is not a
defect — it is a **deliberate frozen proving ground**, and seasons are a planned expansion axis rather
than a wall. TY2024 remains filable (a late return is still a return), so it works as the proving
ground for Milestone 1.

**Step 1 (TY2025) drops out of the critical path** and becomes ordinary scheduled work, one season
among many. The Oct-15 statutory pressure — the single argument that ranked it above everything —
is gone.

**§G-11 is REINSTATED, on the synthesis's own reasoning.** Its "keep the invariant, stop the program"
verdict rested on one comparison: *"TY2025 has a statutory deadline and G-11 does not."* Remove the
deadline and the comparison inverts. The "free before users exist" window is real, does close, and now
has nothing competing with it. ★ The four sub-verdicts that were NOT about sequencing still stand and
are still right: keep the coverage checker in CI; keep the doctrine for new forms; **fix G-24 now**;
and **cancel the r3 re-review** — that one was about crossing this repo's own document-review stop-line,
not about deadlines.

**Everything else in §2–§5 survives unchanged.** The bottleneck is still breadth, not depth. The cheap
refusal conversions are still the highest filers-unblocked-per-line moves available, and they are
**year-agnostic form transcriptions** — Form 6251 (already computed and two-oracle validated; only a
`map.toml` is missing), Schedule 8812/CTC (dependents already collected; unpins a `$0` that costs
families up to $2,000/child), the W-2 box-12 allowlist, Form 8889, aggregate 1099-B. The bottom of the
1040 still has **zero oracle witnesses** and has already shipped a defect (§G-24). Web is still a
rendering project, and the three compounding couplings are still worth paying now.

## The revised order

0. **Ship what exists** — hours of owner action, zero engineering. Unchanged and still first.
1. **The cheap breadth wins** — year-agnostic form transcriptions, above.
2. **§G-24 + the bottom-of-the-1040 witnesses** — the refund/owe lines a filer actually reads.
3. **§G-11 P0b/P1** — reinstated; do it while it is still free.
4. **The owner files a real return** (TY2024, late — a proving ground filing is still a filing).
5. **Intake breadth · the TUI's two missing ends · more filing statuses.**
6. **Seasons** — TY2025 and beyond, scheduled rather than rushed.

## Still open, and it now decides more than it did

**Is filer #1 the owner, or a stranger?** This was the second gating question and the amendment does
not touch it. If a stranger, `data-in` was the reviewer who had it right and intake jumps above form
coverage — four venues, one frozen at its 2012–2019 export schema, is a locked door. If the owner,
intake is effectively done and the order above stands.

---

# DIRECTION — btctax, 2026-08-01

Synthesized from seven independent slice reviews (filing-path, tax-engine, tui, data-in, web-readiness, confidence, allocation — the last commissioned as the adversarial case). Where reviewers disagreed it is surfaced, not averaged.

---

## 1. WHERE WE ACTUALLY ARE

**A real person cannot file a return with btctax today, and the first thing that stops them is the calendar: the only supported year is TY2024, whose deadline passed sixteen months ago, while the season that exists — TY2025, extensions open until 2026-10-15 — is fail-closed at `btctax-adapters/src/tax_tables.rs:782`.** Behind that gate, the machine is genuinely real: a verified CSV→reconcile→interview→screens→15-form filled AcroForm packet, two-oracle-validated, byte-reproducible, fail-closed, with the best refusal UX any reviewer had seen — one reviewer ran J6 end-to-end and got a signable MFJ packet. But it serves roughly **1 in 4 plausible TY2024 crypto filers and 0% of the current season**. Even inside TY2024: a filer with a child overpays up to $2,000/child (1040 L19 pinned $0), an employer HSA contribution (W-2 box 12 W) refuses the whole return, box-12 code C — informational, on tens of millions of W-2s — hard-refuses with no remedy (verified live), a single share of stock is unrepresentable (no 1099-B input exists), and the intake admits exactly four venues, one frozen at its 2012–2019 export schema. The TUI can reconcile beautifully but cannot state a 1099-INT, never shows line 37, and cannot emit the PDF. Meanwhile **main is ~99 commits ahead of origin, v0.15.0 is reviewed and unpublished, and 378 commits in 12 days have reached zero humans** — the last month bought 0C/0I-grade rigour on a year nobody can timely file.

## 2. THE BOTTLENECK

Six of seven reviewers named the same constraint wearing different clothes: **breadth, not depth**. The validated corridor — 13 lines of the 1040, Single/MFJ, credit-free, AMT-free, withholding-free, four venues, one tax year — is triple-instrumented and superb, and every recent unit of effort deepened it further, while the walls that stop actual filers (year, child, HSA, brokerage, intake, TUI ends) sat still. The adapters crate has not grown since Phase 1 (4k lines) while 46k lines of TUI-edit and the forms constellation accreted behind it.

Within breadth, **one term dominates and carries a statutory deadline: the year gate.** TY2025 support is worth more than every other move combined, because without it the answer to "who can file?" is *no one*, regardless of refusal calibration.

The one genuine disagreement is `data-in` versus the rest, and it is really a disagreement about **who filer #1 is**. The adapters were built against the owner's own exports; if filer #1 is the owner, intake is *done* and the critical path is TY2025 + fact-pattern coverage. If filer #1 is a stranger with a Kraken CSV, intake is a locked door and nothing else matters until it opens. **This plan resolves the fork deliberately: Milestone 1 is "the owner files their own real TY2025 return"; Milestone 2 is "a stranger does."** That ordering makes every reviewer's bottleneck true in sequence instead of contradictory — and it front-loads the one milestone with a deadline.

## 3. THE CRITICAL PATH

Ordered; each step gates the next. Sizes are honest estimates at current capacity.

**Step 0 — Ship what exists.** Push main, write `.pii-patterns`, revoke the temp crates.io token, publish v0.15.0. *Why first:* it is minutes of owner action, zero engineering, and every unpushed commit raises loss-on-disaster (the backlog was nearly lost to /tmp once already). **Hours.**

**Step 1 — TY2025 becomes filable.** Finish track B3 (Schedule 1-A T2–T7, spec green, T1 built), bundle TY2025 `FullReturnParams` (constants already researched at `tax_tables.rs:793-801`), OBBBA SALT worksheet, Form 6251 re-transcription, 1040 reshape. Run the missing independent review of the plan (r3 has only r1 in `design/ty2025/reviews/`), then build. *Why it precedes everything:* every other improvement lands on a year no one can file; the extension window closes Oct 15, ~10 weeks out. **3–6 weeks. If the plan breakdown says >8, the window is already lost — see §6.**

**Step 2 — Convert the cheap refusals, against the TY2025 surface.** In rough value-per-line order: (a) box-12 allowlist calibration — add C, V, Y, P with citations and tests (**hours**; largest unblocked-filer count per line changed); (b) attach Form 6251 — it is already computed and two-oracle-validated for every return; only a `map.toml` is missing (**days**); (c) Schedule 8812 / CTC — dependents are already fully collected; unpins L19 and converts every family from "overpays $2k/child" to filable (**days**); (d) Form 8889 common case, un-refusing box-12 W (**days–week**); (e) aggregate 1099-B input feeding Sch D 1a/8a per Exception 1 (**days**). Transcribe each new form with G-11-typed entries from day one — the doctrine at zero marginal cost. *Why after Step 1:* these are form transcriptions and must land on the year people will file. Partial overlap with Step 1 is fine. **~2–3 weeks total.**

**Step 3 — Put the bottom of the 1040 under witness.** Add withholding/payments as a corpus + sweep axis (L22/L24/L25d/L33/L34/L37 — both oracles model withholding), and fix G-24 (L34/L37 print a sworn $0 where the form says blank) directly, as two line-level emitter gates on existing precedent — no type migration required. *Why:* the refund/owe number is the line a filer actually reads, it has zero witnesses today, and it has already shipped a defect. Also: one equality test binding `testonly::ty2024_table()` to the bundled adapters tables (**hours** — closes the "validated artifact ≠ shipped artifact" class). **~1 week.**

**Step 4 — The owner files a real return.** TY2025, via CLI + TUI, on real data. Write down everything that hurt. *Why it gates Milestone 2:* btctax has never had a user; every priority above and below this line is a guess that one lived filing will re-rank better than any census. **Days, calendar.**

— *Milestone 1 lands ~6–9 weeks from today: inside the Oct-15 window only if Step 1 starts now.* —

**Step 5 — Open the front door (Milestone 2: a stranger files).** (a) Generic btctax-native CSV schema + `Source` variant — turns "impossible" into "spreadsheet work" for every unsupported venue and on-chain wallet (**days**); (b) modern Coinbase vocabulary (Convert, staking, Advanced Trade) against real fixtures (**days**); (c) ingest skips-and-reports unrecognized files instead of aborting the batch (**hours**); (d) guard pseudo-approve's zero-sat `RawInbound` placeholder over trade-direction Unclassifieds — the one silent-understatement trap found (**hours–days**).

**Step 6 — Close the TUI's two missing ends.** (a) 1099-INT/DIV/G sections in the interview (the W2S repeating-section pattern is the template; delete the EXEMPT entries so the coverage KAT enforces it) — this is the interview's only silent-understatement path, condemned by the repo's own §G-22 criterion; (b) render the absolute return / line 37 in the Tax tab; (c) IRS PDF export behind the TUI `e` key, reusing `export_full_return` verbatim. **~2–3 weeks.**

**Step 7 — Widen the validated corridor.** HoH to corpus and sweep, then MFS (3 of 5 fileable statuses have zero 1040-level oracle coverage); 1099-R/SSA-1099 inputs + lines 4a–6b with the SS worksheet (**1–2 weeks**); Pub 550 / 8949-instruction worked-example KATs for the lot engine plus one CLI end-to-end fixture test (the tenforty wrapper-class seam).

**Total honest distance: a stranger completes CSV→signature through the TUI in roughly 4–6 months. The owner files in ~2.** That is the shortest honest path, and it is reachable — but only if the allocation flips from depth to breadth now.

## 4. WHAT TO STOP OR DEFER

**§G-11 — the straight answer: keep the invariant, stop the program.** Four reviewers converged independently. Concretely: **(1) Keep** the coverage checker in CI forever — it is the spec, it found G-24, it is the program's banked value. **(2) Keep** the doctrine for all *new* forms (Step 2 applies it at zero marginal cost). **(3) Fix G-24 now** as two direct emitter gates (days), banking the program's one real paper defect. **(4) Cancel the queued r3 re-review** — the r2→r3 delta was "the number reviewers spent two ~250k-token rounds estimating is now computed by CI"; that is the repo's own documented stop-line, crossed. **(5) Defer the 179-line P0b/P1 migration until TY2025 ships**, then do it — the "free before users exist" argument is correct and the window genuinely closes, but TY2025 has a statutory deadline and G-11 does not. The program's own survey says ~62 of 64 fabrication sites print a zero that is arithmetically true; the one member of the class with real money (ISO exercise on 6251 line 2i) is a *collection* gap in parked Tier-2 AMT that the migration does not touch. The ledger to date is 10.5:1 prose-to-code. Sequencing, not existence, is the error.

**Stop, in-flight, named:** the form-authority pipeline's label reader (§5) and further harness/B-series meta-instruments — the census is complete (15/15, zero unaccounted); work the gap list, not the instrument. New refusing questions on the return surface (the owner already issued this correction). New hand-rolled key-dispatch flows in `tui-edit/main.rs` — Cycle 5 was marked "the LAST"; honor it until the FULL-SEAM/STAGED decision (§5 below) is made. New features in the standalone viewer (the parked column-totals footer stays parked). New walkthrough journeys/goldens until the funnel's ends exist — more goldens today document more middle. Further ceremony on the six bulk reconcile verbs. G-13/G-17 emission-side census extensions. Any additional assurance layer on 1040 L11–L24 Single/MFJ — that surface has two engines at two levels, fault localization, geometric read-back, byte-reproducibility, and a coverage grammar; the next unit there buys strictly less than the *first* witness on a withholding line. And stop presenting "104 households reconcile against two engines" without its qualifier — Single/MFJ, credit-free, AMT-free, aggregate-gain-input.

**Keep parked:** Tier-2 AMT (Tier 1 refuses for exactly Tier 2's population — fails closed), noting honestly that its resumption, not G-11, is what closes the ISO/2i canonical defect.

## 5. WEB UI: THE SEAM

Unanimous across reviewers: **web is a rendering project, not a re-architecture** — `btctax-core` is pure, `btctax-input-form` is deliberately UI-agnostic with a serde wire (`seam.rs`), forms are bytes-in/bytes-out, plans are structured with "front-ends render their own summaries" as the stated contract. Do not build any server now. But three couplings compound with every commit, and those are worth paying for **now**:

1. **Structure the remedy channel** (days). Refusal and advisory remedies are CLI prose baked into core — "run `btctax income answer`" (`return_refuse.rs:652,680`, `CliError` Display). Make the remedy a typed field (QuestionId / field-path / verb); each front end phrases it. Every refusal added before this deepens the debt — and Step 2 adds several.
2. **No new derived arithmetic in `render.rs`, no new CLI prose in core error strings** (a rule, free). The §2505 lifetime-exclusion block at `render.rs:1999-2029` is tax logic in presentation; new display figures go in core view-models (`printed.rs` is the precedent). Lift the existing four sites opportunistically.
3. **Return warnings as values** (hours). The library fns that print directly (`reconcile.rs:345,902`, `tax.rs:92`, `vault.rs:153`) silently lose consent-relevant output on any non-terminal front end.

**Decide now, build later:** the FOLLOWUPS FULL-SEAM vs STAGED question for tui-edit — the analysis and costing already exist; the decision is free and it gates whether any further flow code may be written. **Defer to web-start:** the `btctax-session` crate extraction (2–4 days, mechanical, deletes the KAT re-export wall), plan-staleness/ledger-generation tokens, session lifetime. **Confirm one assumption with the owner now** because a wrong answer here is the expensive one: this plan assumes web = **localhost, single-user** (same trust model as the TUI; axum + one held Session + `spawn_blocking` saves). If "web" means hosted multi-tenant, the vault/key/lock model is unfit and the distance is months of key-management design, not days of extraction.

## 6. WHAT WOULD CHANGE THIS PLAN

1. **The owner wants a current-season filer soon.** *Falsified by:* the owner declaring TY2024 a deliberate frozen proving ground with spring 2027 (TY2026) as the real target. Then the Oct-15 urgency evaporates, the G-11 migration's "free before users" window becomes the defensible next move, and Steps 1–2 lose their deadline (not their ordering).
2. **Filer #1 is the owner.** *Falsified by:* a real prospective user with non-four-venue data. Then Step 5 jumps above Step 2, and `data-in` was the reviewer who had it right.
3. **B3 T2–T7 is weeks, not months.** *Falsified by:* the Schedule 1-A plan's own task breakdown (no reviewer read it in full). If months, the Oct-15 window is already lost — retarget spring 2027, which promotes coverage/intake moves and demotes calendar pressure; the plan reorders but does not change membership.
4. **The frequency claims** (HSA/CTC/brokerage prevalence, ~1-in-4 served, 15–40h modern-Coinbase estimate) are demographic reasoning, not measurement, and the modern-Coinbase export shape was tested synthetically, not against a real 2024–25 export. *Falsified by:* one real export parsing cleanly (shrinks Step 5b) or Step 4's lived filing re-ranking the walls.
5. **Web = localhost single-user.** *Falsified by:* the owner meaning hosted multi-tenant — §5's answer changes from "days of extraction, later" to "a key-management design project, planned explicitly."
6. **The TUI is meant to be the filing surface.** *Falsified by:* the owner affirming the recorded v1 design (TOML as the reviewable artifact of record). Then Step 6a becomes TOML authoring docs + validation instead of interview sections; 6b/6c survive unchanged.

**The one-sentence answer to the brief's one question:** the machine is converging and the filer is not — the current course, unchanged, polishes a return no one can file; flip the allocation from depth to breadth now and the owner files in about two months, a stranger in four to six.