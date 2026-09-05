# OWNER DECISIONS — 2026-09-04, ruled by the owner's delegate

**Standing:** the owner said *"Proceed autonomously … Consult fable if needed"* and *"Fable may answer
for me."* This document answers `design/LONG_RANGE_PLAN_filing.md` §8 (D-A … D-H) as the owner would,
on the evidence in the repo. Written on `feat/schedule-1a-ty2025` @ `ece7e668`; the plan was written
at `3d83e601`, four commits earlier, and two of its "in flight" rows have since landed (§3.5 below).

**Withheld, and not ruled here:** tagging a release, publishing to crates.io, and anything about the
unrevoked v0.17.0 crates.io token (`CONTINUITY.md:67`). Where a decision touches one of those it is
marked **WAITS FOR THE OWNER**.

**What I machine-checked rather than took from the plan** (commands run in this session):
`full_return_for(2025)` gate and its four clauses (`crates/btctax-adapters/src/tax_tables.rs:788-822`);
`grep tips_deduction|overtime_deduction|car_loan_interest crates/` → **0**; `schedule_1a.rs` absent;
`RefuseReason` variants → **62**; the commit-per-day histogram since T1 (§3.2); the filing-readiness
branch span (`3fc88497..3d01b5e3`: 65 commits, **2026-08-21 → 2026-08-23**); Part IV's phase-out
constants (`crates/btctax-core/src/tax/tables.rs:1112-1117`); the archived-statute list
(`legal/primary-sources/statute-irc/`, 16 files, no §164, no §6651); the FR-27/FR-28 renumbering
(`FOLLOWUPS.md:5671`, `:5696`); the label-reader merge (`ece7e668`) and its two B1 tests.

---

## 0. The one sentence to read first

**The plan's sequence stands — TY2025, profile P0, Phases 1→2→3 — but its "file late with btctax"
(A) versus "file by other means" (B) table is a false choice: build TY2025 with btctax AND file your
own TY2025 return by 2026-10-15 by other means unless you already paid in full under an extension;
nothing else in §8 needs you, and tag/publish/token were left untouched.**

---

## 1. Rulings

### ★ D-A — which year, and is TY2025 already handled by other means?

**RULING: TY2025 is the target year and Phases 1→3 build it; the owner's own TY2025 return is filed by
2026-10-15 by other means (a preparer or FFFF), and btctax's TY2025 packet is the comparison, not the
filing of record.**

**WHY.** (1) The build target is not actually in question: the fail-closed gate names TY2025's exact
unblock condition (`tax_tables.rs:801-810`), the owner's own August direction already made
Milestone 1 *"the owner files their own real TY2025 return"* (`design/direction/DIRECTION-2026-08-01.md:159`),
and Schedule 1-A, the SALT worksheet and the 1a/1b Form 6251 are TY2026 requirements too, so no
work on TY2025 is wasted (plan `:472-475`, correct). (2) TY2025's forms are **final and archived
today** and both oracles exist for it; TY2026's finals do not exist until ~January 2027 and TY2026 is
fail-closed with no oracle (`design/ty2025/SPEC.md:140-142`; `recon-open-work.md:37`) — so "skip to
TY2026" idles Phase 2 for four months, which is the decisive argument against option C and the plan
does not make it. (3) The personal filing is a different question with an asymmetric payoff: option B
wrong costs a preparer's fee and a day (plan `:468`); option A wrong costs 5% per month of *unpaid* tax
up to 25% (26 U.S.C. §6651(a)(1), base reduced by timely payments under §6651(b)(1) — **not archived
in-repo**, see §2.6) on a return the trial shows owing $382,710 with no withholding assumed
(`FILING-TRIAL-2026-08-02.md:19-21`, `:409`). The plan's own velocity verdict (§6.2) says the build
will not beat October 15; even if that verdict is overstated (§2.2), the owner should not bet a
five-figure penalty on it.

**WHAT WOULD CHANGE MY MIND:** the owner confirms a Form 4868 was filed *and* the TY2025 liability was
paid in full by 2026-04-15 — then a late btctax filing costs nothing under §6651(b)(1) and option B's
fee is wasted; or the owner confirms **no extension exists**, in which case October 15 is not their
deadline at all (the return has been late since April 15) and filing *now* by any means dominates.

### ★ D-B — is P0 still the owner's actual profile?

**RULING: P0 stands as the definition of done. Assumed answers to the four sub-questions: no 1099-R /
SSA-1099 / pension; no rental or K-1; broker 1099-B *with basis reported* — already modelled; no spouse
IP PIN — and Phase 4 builds that field regardless.**

**WHY.** The repo supports P0 as the *envelope* the owner wrote and drove through the binary
(`FILING-TRIAL-2026-08-02.md:14-21`, "the owner's scenario", with the trial's own list of assumptions
it had to add), and the adapters were built against the owner's own exports
(`DIRECTION-2026-08-01.md:159`). The $2M gain was modelled through the broker path, so a 1099-B is
*inside* P0, not a new item (`FILING-TRIAL:138-148`); a broker lot whose basis is not reported already
**refuses** (`FILING-TRIAL:150-152`). The default is fail-closed in every direction: if the owner
does have retirement or rental income, the scope attestation now names those forms and refuses
(`1548462a`, `CONTINUITY.md:31-33`; `return_refuse.rs` destructures `ReturnInputs` exhaustively,
plan `:54`) — so a wrong D-B surfaces as a **refusal at Phase 3**, never as a wrong return.

**The assumption, in one sentence the owner can correct:** *every income TYPE on the owner's real
TY2025 return is a member of P0's set — self-employment income, long-term gains (ledger and/or a
basis-reported 1099-B), a large charitable gift, medical expenses, dependents, student-loan and
car-loan interest — and nothing else.*

**WHAT WOULD CHANGE MY MIND:** one income type on the real return that P0 lacks and btctax refuses —
any of a 1099-R, SSA-1099, Schedule E/K-1, a basis-not-reported 1099-B, or a second SE earner. Each
moves exactly one §7.5 item onto the critical path (retirement → D-C flips; the rest are P3-permanent
refusals and mean "preparer for that year", not "build it").

### ★ D-C — retirement income (P2): build in parallel, or park after its spec goes green?

**RULING: Park the spec at DRAFT r1, unreviewed. Its first review round is Phase 6's entry gate, not
now. No build.**

**WHY.** It is not on P0's path (plan `:84-88`) and the owner's own ordering already puts it after the
TY2025 form set (`CONTINUITY.md:62-63`). Review budget is the scarce resource on the critical path —
the r4-fold review gates T2 (`CONTINUITY.md:51-53`) — and a spec reviewed to green now would have to
be re-verified at build time anyway, because *"line numbers and 'as of' references decay every
commit"* (`STANDARD_WORKFLOW.md:189-191`). This departs from the plan's default ("finish the spec to
0C/0I") on cost, not on direction: nothing builds on the spec, so no gate is crossed
(`STANDARD_WORKFLOW.md:15-23`).

**WHAT WOULD CHANGE MY MIND:** D-B answered "yes" to 1099-R/SSA-1099 — then it joins Phase 1, spec
review first.

### ~~D-D~~ — FR-1 now or with the TY2025 lane?

**RULING: MOOT — confirmed built and merged** (`fix/fr1-ctc-line19` is an ancestor of HEAD;
`crates/btctax-core/src/tax/printed.rs:544-552` carries `line19: Option<Usd>`). Nothing to decide.

### ★ D-E — build the year-port machine (§5)?

**RULING: Yes, split exactly as the plan proposes — the printed-text diff gate (pass 2) lands in
Phase 2 with a planted-defect test; the copy/scaffold (passes 1 and 3) lands in Phase 5 — with one
correction to its payoff claim (§2.3).**

**WHY.** Pass 2 is the only instrument that sees Schedule C's 27a/27b swap and Schedule 2's "Reserved
for future use" (plan `:373-376`; `recon-year-port-delta.md:64`, `:60`), and both traps fire in
Phase 2. Per B1 (`CLAUDE.md:167`) it does not exist until watched red on a fixture that renames one
field and changes one line's text (plan `:381-383`) — the same discipline the label-reader fix just
followed (`design/agent-reports/2026-09-04-label-reader-1a-fix.md:93-98`). Passes 1 and 3 cannot pay
for themselves on a port that happens once (plan `:390`).

**WHAT WOULD CHANGE MY MIND:** TY2026 finals (January 2027) restructuring more than the two forms
already known to change (§2.3) — then Phase 5 shrinks to pass 1 only.

### ★ D-F — tag / publish / crates.io token?

**RULING: WAITS FOR THE OWNER on all three limbs. Nothing in Phases 1–4 depends on any of them.**

**WHY.** The owner withheld these explicitly (`CONTINUITY.md:67`, `:79-81`) and a delegate answering
questions does not lift a prohibition. The plan is right that none bears on filing (`:571`). I note,
without authorising it, that revoking the stale token was "Step 0 … hours … owner action" on
2026-08-01 (`DIRECTION-2026-08-01.md:165`) and is still open five weeks later.

**WHAT WOULD CHANGE MY MIND:** nothing I can rule on. The owner types it.

### ★ D-G — EITC/ACTC (FR-16)?

**RULING: Stays deferred past the first filed return. This is a *sequencing* ruling; owner decision 11
("IN SCOPE", `design/direction/FILING-READINESS-PLAN.md:566-569`) is not reversed.**

**WHY.** P0 cannot claim EITC in any P0-shaped year — the $2M gain exceeds the §32(i) investment-income
limit many times over (plan `:572`), and even a no-gain P0 year at $85,000 SE income is above the
MFJ/3+-children completed-phase-out (a tax-law fact **not in-repo**; verify before relying on it). FR-16
is a project with two unmapped schedules and a booby-trapped oracle (`FOLLOWUPS.md:5580-5584`;
`design/direction/ORACLE-TRAP-credit-takeup.md`). Follow-ups burn down by owning phase, and its owning
phase is P6 (`/scratch/code/CLAUDE.md:155`; `STANDARD_WORKFLOW.md:193-213`).

**One distinction the plan blurs:** ACTC (Schedule 8812) is not EITC. On a P0 year *without* the
outsized gain, nine children make line 19 a dollars item (plan `:330-331`), and FR-1 leaves that line
**blank for the filer to fill** (`printed.rs:547-552`) — an overstatement, loudly advised, not a
refusal. Schedule 8812 therefore moves onto the critical path the first time the owner files a
no-gain year, which is a P6 trigger the plan should name.

**WHAT WOULD CHANGE MY MIND:** D-B reveals the real return has no large gain and earned income under
the EITC ceiling — then Schedule 8812 (not Schedule EIC) enters Phase 6 first.

### ★ D-H — mailing-address help as a link plus two rules, not a bundled table?

**RULING: Yes. Link plus the two facts (filer's state; payment enclosed or not), the attachment order,
and Form 1040-V — nothing that can rot between releases.**

**WHY.** The IRS corrected the related addresses mid-2026 (`recon-efile-procedure.md:99`); a bundled
table is a figure with no reader and a stale one at that. Fetching at export time is impossible by
design — `check-isolation` is one of the five gates (plan `:22-24`). Form 1040-V is the one item that
must actually be *built* because P0 owes (`FILING-TRIAL:409`) and the voucher is not even named as an
exclusion today (`recon-last-mile.md:141-144`).

**WHAT WOULD CHANGE MY MIND:** nothing foreseeable.

---

## 2. What the plan gets wrong

Its central reframing — **October 15 is a penalty deadline, not a product deadline** — is correct as a
statement about the *product* and is the right frame for the build. Where it goes wrong is in
presenting the product decision and the owner's personal filing as one choice, and in three factual
claims.

### 2.1 Options A and B are not alternatives — they are the build and the hedge

The §6.3 table (`:465-470`) lists A ("build properly, file late with btctax") against B ("file by other
means, use btctax as a check"). But B's sacrifice column says *"nothing in this plan"* — B is A plus a
preparer. The only thing A adds over B is *not* paying a preparer, and its downside is the §6651
penalty on any unpaid balance. A plan that recommends A over B is recommending the owner save a fee by
carrying a five-figure tail risk on a schedule the same plan calls unreachable. **Take both.** D-A rules
so.

### 2.2 The velocity argument measures allocation, not throughput

§6.2 (`:430-433`) says T1 landed 2026-07-29 and *"37 days later, the TY2025 track has produced no
further implementation commit … August was consumed by the filing-readiness branch."* Measured:

```
commits per day since 2026-07-29 (git log --format=%ad --date=short | sort | uniq -c)
  07-30: 74   07-31: 29   08-01: 11   08-02: 45   08-03: 25   08-05: 2   08-09: 23
  08-10: 10   08-20: 3    08-21: 30   08-22: 31   08-23: 7    09-04: 33
  → 323 commits on 13 active days out of 38; idle 08-11→08-19 and 08-24→09-03
filing-readiness branch: 65 commits, 2026-08-21 → 2026-08-23   (three days, not a month)
```

The 37 days contain **ten active days, every one spent elsewhere by choice**. When the repo is active it
lands ~25 commits a day and a four-phase, seven-review, 92-file branch in three days. So "TY2025 by
2026-10-15 is NOT reachable" (`:426`) is stated with more certainty than the evidence carries: the
constraint is which work the next ~13 active days are pointed at, not how fast work goes. The
conclusion survives on other grounds — the 0C/0I loop rate (`:434-437`), the gate's own prohibition on
deleting early (`tax_tables.rs:812-813`), and the penalty asymmetry — and D-A rests on those, not on
§6.2. The plan should say "should not be relied on", not "not reachable".

### 2.3 "TY2026 is then a cheap port" is contradicted by the repo's own recon

§5.3 (`:398-402`): *"TY2026 is unlikely to restructure like TY2025 did."* The repo already knows
otherwise — `design/full-return/recon/fable/01-ty2025-finals-obbba.md:47-56` lists three provisions
that are statutorily **TY2026**: the 0.5%-of-AGI charitable floor for individuals (§70425, new
§170(b)(1)(I)), the §68 replacement that reduces itemized deductions by 2/37 for 37%-bracket filers
(§70111), and the 90% gambling-loss limit; `:82` adds the SALT cap stepping to $40,400; and `:251-252`
says *"park them as the first TY2026 delta list."* So Schedule A gets **new tax logic again** in TY2026
— a floor and a limitation worksheet — and Form 1040 line 12e with it. Phase 5's "ten verified copies
plus two judgment calls" (`:410-411`) is still the right shape, but the two judgment calls are the same
two forms and they are real work, and "first *on-time* return: TY2026, made cheap by Phase 5" (`:485-486`)
undersells that. This matters for P0 specifically: an $85,000 gift on a $2M-gain return is exactly the
filer the 0.5% floor and the 2/37 haircut were written for.

### 2.4 P0 is a weak witness for TY2025's structural delta — and the plan implies otherwise

§1.2 (`:72`) justifies P0's TY2025 dependence by *"$2,500 car-loan interest, which is a TY2025 Schedule
1-A item"* and §6.3 says under option C it *"is hand-computed."* It is **$0**. Part IV phases out at
$200 per $1,000 of MAGI above $200,000 MFJ (`tables.rs:1112-1117`; `SPEC_schedule_1a.md:259`); P0's AGI
is $2,085,000 (`FILING-TRIAL:400-402`), $1.885M into the band. P0 also has no SALT
(`FILING-TRIAL:407`) so the §164(b) worksheet prints its floor, and AMT is zero so the 1a/1b split
changes nothing. **Every TY2025-specific line on P0's return is provably zero.** The honest reason
Schedule 1-A is on P0's critical path is the gate's doctrine — a TY2025 that computes without it is
silently wrong for *other* filers (`tax_tables.rs:812-813`) — and that reason is sufficient; the plan
should give it rather than the car loan. The useful inverse: P0 is precisely the r4 fold's C-I2 filer
(*"a car-loan-only filer computes 15 phase-out lines the form says to skip and line 35 prints $6,000
for a non-senior"*, `IMPLEMENTATION_PLAN_schedule_1a.md:15`) — so P0 is the right witness for T2's
completion predicates, and Phase 3's packet read must check that Schedule 1-A is **not attached** when
line 38 is zero.

### 2.5 F-7 went stale within hours, in the safe direction

`:146-157` lists the label-reader fix as in flight and Phase 2's R7 prerequisite. It landed at
`0642401e` and merged at `ece7e668` (*"the census was blind on most forms, not one"* — 86 more labels
recovered), with **two tests written before the fix and observed red**
(`2026-09-04-label-reader-1a-fix.md:93-98`), so R7 is closed under B1. `SPEC_form6251_ty2025.md`
also landed (DRAFT r1, `:1-12`). Still not landed: the r4-fold review and the understatement audit.
The plan's r2 should move those rows.

### 2.6 "A fact the plan cannot cite" can be cited, and the plan asks the wrong half of it

§6.3 (`:477-481`) rests option A on *"the failure-to-file penalty is computed on unpaid tax"* and calls
it uncitable. It is 26 U.S.C. §6651(a)(1) (5% per month, 25% cap), §6651(a)(2) (failure to pay, 0.5%
per month), and §6651(b)(1) (the (a)(1) base is reduced by tax paid on or before the due date) —
statute, which is the only authority this repo treats as law. It is **not archived**:
`legal/primary-sources/statute-irc/` holds 16 sections and §6651 is not one (nor is §164, per
`SPEC_schedule_a_ty2025.md:107-109`). Archive it in Phase 4 — the penalty mechanism is part of
"everything the envelope needs" (`:47`). And the plan never asks the prior question: **does an
extension exist at all?** Without a Form 4868, "extensions open until 2026-10-15" (`:422`) is not the
owner's deadline; the return has been late since April 15 and the (a)(1) penalty saturates at 25%
around mid-September. D-A's "what would change my mind" carries both halves.

### 2.7 Minor

- §8's Phase 0 exit is *"D-A and D-B recorded in `CONTINUITY.md`"* (`:185`). This delegation was
  limited to one file; the controller records them (§3, item 5).
- `:49` counts 62 refusal variants; confirmed (awk over `return_refuse.rs`, this session).

---

## 3. The next 8 hours of autonomous work

**Fleet, as of `ece7e668`:** the Schedule 1-A r4-fold review — in flight; the label-reader fix —
**landed and merged**; SPEC Form 6251 TY2025 — **landed, DRAFT r1, unreviewed**; the understatement
audit — in flight.

**Do, in this order:**

1. **Persist → fold → re-review the r4-fold report the moment it lands**, and build T2 only at 0C/0I
   (`CONTINUITY.md:51-53`; `/scratch/code/CLAUDE.md:22`). Do not front-run it. T2 is the chokepoint
   and its type decisions cannot be repaired at T4 (plan `:200-202`); C-I3/C-I4 are
   understatement-direction and outrank everything else in Phase 1 (`:204-206`).
2. **Land SA25-2 and SA25-3 now, in one worktree, as their own reviewed change** — the
   `field_census.rs:105` year loop (KAT-2) and the `verify_flat` absolute-row leg (M-3)
   (`SPEC_schedule_a_ty2025.md:446`, `:452`, `:509-510`). Both are "before the map" prerequisites,
   independent of Schedule 1-A, and each is a B1 kill-test: plant a bogus FQN in
   `forms/2025/f1040.map.toml` and **confirm the current census stays green first** — that green is
   the defect; then point `line9` at line 11's box and confirm the verifier passes it — that pass is
   the second defect. The spec's §4.3 (`:289-326`) is a measured hole in a guarantee the crate
   advertises (`schedule_a.rs:33-34`): under `STANDARD_WORKFLOW.md:238` that is Critical-class on
   the day the first TY2025 map is authored, and today is the cheapest day to close it.
3. **Dispatch ONE independent review each of `SPEC_form6251_ty2025.md` r1 and
   `SPEC_schedule_a_ty2025.md`** (opus; one question each; forbid fresh-audit scope; say what is
   already machine-verified). They are Phase 2's two structural forms and can be green before Phase 1
   closes, so Phase 2's entry is not gated on review latency.
4. **Phase 4's two one-liners, only if an implementer is otherwise idle:** the spouse IP PIN field
   (*"one optional field on `ReturnInputs`"*, `f1040.map.toml:274`) and `broker_reported_rows: 0`
   (`admin.rs:1200`). Both are off the arithmetic critical path and touch no Schedule 1-A file, so
   they cannot race T2. Worktree, own review.
5. **Record D-A…D-H in `CONTINUITY.md` ④ as Phase 0's exit, and update ③** (label-reader landed;
   6251 spec landed). Fold §2.3–2.6 above into the plan as its r2 — report and fold in separate
   commits, per `/scratch/code/CLAUDE.md:22`.

**Do not:**

- Start any TY2025 map port (Phase 2) before T2 exists — Phase 2's entry gate is Phase 1 green
  (plan `:213`), and the three unarchived PDFs (`:230-232`) are the only Phase 2 commit that may
  land early.
- Build the port machine's copy/scaffold (D-E: Phase 5).
- Spend a review round on `SPEC_retirement_income.md` (D-C).
- Run more recon. Five reports and a 613-line plan is the synthesis; the marginal recon is now
  negative and the understatement audit is the last one owed.
- Touch tag, publish, or the token (D-F).

**One thing to watch in the understatement audit when it lands:** if it re-ranks §4 (plan `:156`), the
only re-ranking that changes this document is a new understatement path on P0's own line set — that
would enter Phase 1 ahead of T2, the way C-I3/C-I4 did.
