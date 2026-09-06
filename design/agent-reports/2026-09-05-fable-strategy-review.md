# Fable strategy review — "a filed return, every year"

Reviewer: Fable 5.1, independent, strategy-level, read-only. Date: 2026-09-05. HEAD: `52298b6d`.

**Read:** `CONTINUITY.md` (top 130 lines); `design/ROADMAP_STATUS.md` (all); `design/LONG_RANGE_PLAN_filing.md`
§1, §3 P4–P6, §6, §8, §9; `design/TY2026_PORT_REPORT.md` §1, §3, §4, §6, §7, §8; `design/FORM_AUTHORITY_TABLE_DESIGN.md`
r2 (all); `design/agent-reports/2026-09-05-fable-plan-review.md` and `…VERIFICATION.md` (all — folded, not re-found;
no `.r2-fold-review.md` exists); `CLAUDE.md`; `STANDARD_WORKFLOW.md` §0–§1 + outline; `design/HARNESS.md` outline;
`FOLLOWUPS.md` FR-46..FR-53 and the two ★★★ CRITICAL audit headers (`:5762`, `:5831`); `design/TY2026_WORK_LIST.md`;
`design/OWNER_DECISIONS_2026-09-04.md` §0–D-E; `design/direction/FILING-TRIAL-2026-08-02.md:1-40`;
`design/direction/DIRECTION-2026-08-01.md:150-175`; `git log` (60 commits, plus counts by month and by subject
prefix since 2026-07-01); `scripts/oracle/` listing; `.venv` taxcalc version and its 2025/2026 AMT parameters;
`legal/text/irs-guidance/RevProc_2025-32.txt:729-732`; `crates/btctax-forms/forms/{2024,2025}` listings;
`crates/btctax-adapters/src/sources/`; `crates/btctax-cli/src/cli.rs` (`income import`); `admin.rs:365` (`hand_marks`);
`tax_tables.rs:944-951`. No file other than this one was written; no mutating command was run.

**Not re-derived:** the plan review's 1C/8I/4M and its ledger; the port report's 27 risks and 24-step runbook;
the six-registry measurement in design r2 §1; the owner rulings; the doctrine list in the brief.

---

## Verdict (≤10 lines): will this strategy file TY2026 in 2027, and repeat?

**Probably yes for the extended deadline (2027-10-15), probably not for April — and only if the pre-finals
window is spent on the filer, not on the machine.** The strategy is sound at the instrument layer and mis-ordered
at the project layer: it invests first in a year-transition machine designed from **zero completed transitions**
(port report §7 D4: TY2025 was never wired end to end), while the one thing that has never happened in fourteen
months — the owner's own data driven through the tool to a printed packet — is scheduled for February 2027,
inside the ten-week window that also holds the finals, the oracle, the 1099-DA and every transcription.
"Done" is defined against a stress scenario (P0: nine children, $375k medical, SE mining) that a delegate ruled on
in the owner's absence, not against the owner's return. Fix the order (S1–S3 below) and the calendar has six months
of slack; keep it and the most likely outcome is a TY2026 return prepared outside the project for the second year
running, with a beautifully instrumented year-package table nobody has ported a year through.

## The single most likely reason it does not file

**The lived journey is scheduled last.** `DIRECTION-2026-08-01.md:173` made "the owner files a real return … on real
data; write down everything that hurt" Step 4 in August; it was never done, the TY2025 return went outside the
project, and `ROADMAP_STATUS.md` §3 now puts the first real-data pass in the AFTER-OTS bucket — February 2027.
The specific place it lands: the owner's dispositions are on exchanges (`sources/{coinbase,gemini,river,swan}.rs`),
TY2026 is the first year those exchanges report **basis** on Form 1099-DA (FR-46), the forms arrive ~2027-02-16,
and btctax has no place to put them (FR-46 b/c is "the TY2026 input surface", unscheduled). Under the measured
cadence — one schedule took 37 days and five review rounds (`LONG_RANGE_PLAN_filing.md` §6.2) — a first-contact
defect class discovered in mid-February on the filer's own rows does not close by April, and the extension becomes
the plan by default rather than by design. Everything else on the critical path is known work; this one is unknown.

## Improvements, ranked by leverage

| # | change | why (evidence) | cost | risk | when |
|---|---|---|---|---|---|
| S1 | **TY2025 dress rehearsal on the owner's real data, diffed against the return filed outside** — never mailed | The owner's filed TY2025 return is a signed gold standard for THIS filer; D-A itself said "a comparison against your filed return … is strictly better evidence than an oracle" (`OWNER_DECISIONS_2026-09-04.md` D-A). The TY2025 build IS the TY2026 build (`LONG_RANGE_PLAN` §6.3). Design r2 §10 step 5 already wires the ten orphaned TY2025 maps | `FullReturnParams` TY2025, three authority fetches (`ROADMAP_STATUS` P2), delete the TY2025 gate, one real-data pass; ~2 weeks | re-opens paused TY2025 assets; mitigated: nothing is filed, and every artifact is reused for TY2026 | Oct 2026 |
| S2 | **Define done against the owner's actual return** — two owner answers this week: income types/venues for 2026, and state of residence | D-B was answered "as the owner would" by a delegate (`OWNER_DECISIONS` header, D-B); P0 is `FILING-TRIAL-2026-08-02.md:14-20`, a stress scenario. The brief's own profile (W-2, exchange dispositions, card rewards, charitable BTC, AMT, car loan) needs no Sch C/SE/8995/8995-A and no Sch 1-A Parts II/III | one owner message | none | this week |
| S3 | **1099-DA on the input surface NOW, plus the owner's broker specific-ID instruction in 2026** | FR-46(b)(c) is filed to "the TY2026 input surface" with no date; the data arrives inside the window. `§1.1012-1(j)` timeliness is already one predicate in the engine (`fb578528`); the owner-side act — a standing specific-ID instruction to each exchange BEFORE selling — is a 2026 calendar item, else every 1099-DA row needs a code-B adjustment | one input field per disposition source, box routing G/H/I–J/K/L, one kill; owner: four emails | HIFO vs broker-FIFO divergence on every row if the instruction is not given | Sep 2026 |
| S4 | **Bound the machinery: design r2 steps 1–5 by 2026-10-31, hard stop; port TY2026 by hand with `forms delta`/`wire --check`/`port-status`; extract `forms port` afterwards** | Zero completed transitions to design from (port report §7 D4); the one measured machine output on drafts was wrong by a cover sheet (`TY2026_WORK_LIST.md` header); the generator is item 8 of 8 in the port report's own build order (§4) | steps 1–5 ≈ the r2 estimate; the generator is deferred, not cancelled | the 85 hand-edits happen once more, by a person, with the diff tools watching | Sep–Oct 2026; generator May–Aug 2027 |
| S5 | **Use the tool on real 2026 data for the 2026-09-15 and 2027-01-15 estimated-payment checks** | `report --tax-year 2026` computes today (`TY2026_PORT_REPORT` §1 ¶2); it exercises tables, the four adapters, card rewards and the price dataset on the year that will be filed, four months before finals | hours | none; "advise only" for 1040-ES stands (`LONG_RANGE_PLAN` P4) | Sep 2026, Jan 2027 |
| S6 | **Right-size ceremony for a one-person annual update** — one review round per document then execute; executed audits stay; the single Fable B3 review is the pre-mail one; fan-out ≤ 6 lenses; no instrument without a consumer on the owner's path | 1,284 commits in July; review/docs commits ≈ build commits 514:525 since 07-01; 535 design `.md` = 12.6 MB against 260k LOC; 13 review files for one schedule. The two 2026-09 Criticals (FR-29, FR-31) came from audits that RAN the tool, not from document rounds | a written rule change in `CLAUDE.md`/`STANDARD_WORKFLOW.md` §8 | a document defect reaches the build — where a test or the compiler catches it (`CLAUDE.md` "tests for conformance") | now |
| S7 | **Pre-rule the oracle fallback in September**: what evidence suffices to sign by 2027-10-15 if OTS 2026 slips or is wrong | Two-oracle rule is absolute; OTS 2026 ~2027-01-27 and has known AMT defects; taxcalc 6.7.2 installed vs 6.8.2 latest with two known-wrong TY2026 cells (`…port-oracles.md:115,172,190`) | one ruling, recorded in `ROADMAP_STATUS` §3 | a ruling made under April pressure instead | Sep 2026 |
| S8 | **Last mile as a physical rehearsal**: print, assemble by attachment sequence, sign block, 1040-V/4868 (FR-49), mailing link, certified mail — done once on the S1 packet | No evidence anywhere in `design/` that a packet has ever been printed on paper; `hand_marks` (`admin.rs:365`) exists but has never met a printer | a day, plus FR-49's two AcroForms | none | Oct 2026 (S1), Mar 2027 |
| S9 | **Rule on TY2017: drop it** | five wired forms, zero validated counterpart/notes/geometry/census (`ROADMAP_STATUS` §0 ¶3); it costs a row in every glob-walking gate forever | one owner sentence; a directory delete | none | now |

## Per-improvement detail

### S1 — the TY2025 dress rehearsal (never mailed)
**What changes.** Un-pause exactly the slice of TY2025 that is a rehearsal: `FullReturnParams` TY2025 (encoded beside
FR-47's TY2026 params — same shape, one sitting), archive `f1040s1--2025`, `f8275`, `f8995a--2025`
(`ROADMAP_STATUS` P2 — the fetch is a NOW item under FR-50 anyway), wire the ten orphaned maps (design r2 §10 step 5,
already planned), delete `ty2025_full_return_must_stay_fail_closed`, then drive the owner's real 2025 exports and W-2
through CLI + TUI to a packet and **diff every printed line against the return actually filed**.
**Why.** It is the only oracle that is (a) specific to this filer's profile, (b) already signed, and (c) available
before January. It walks the whole journey — import, reconcile, interview, refusals, print — a year before the
critical window, and it converts the unknown ("what does the owner's data do to this tool?") into a follow-up list in
October. The August direction said this in one sentence: *"every priority above and below this line is a guess that
one lived filing will re-rank better than any census"* (`DIRECTION-2026-08-01.md:173`).
**Cost.** ~2 weeks of the pre-finals slack, most of it already scheduled under other names.
**Risk.** Scope creep back into "finish TY2025". Guard: the exit criterion is a diff report, not a green packet; every
divergence becomes a FOLLOWUP with an owning phase; the packet is shredded.
**How to know it worked.** A committed `design/ty2025/REHEARSAL.md` listing every line that disagreed with the filed
return and its disposition; the count of *engine* disagreements (as opposed to input gaps) is the number to watch.

### S2 — done is the owner's return, not P0
**What changes.** Ask the owner two questions and record the answers in `ROADMAP_STATUS` §0: (1) every income type,
venue and deduction on the real 2026 return (W-2s? any Sch C? interest/dividends over $1,500? charitable BTC and its
size — Form 8283 Section B and a qualified appraisal above $5,000 are owner actions with a before-filing deadline,
`SPEC_appraisal_trigger_minimal.md`; car-loan interest; tips/overtime: none?); (2) state of residence — a state return
is DO-NOT-BUILD (`LONG_RANGE_PLAN` §7.2) but it is external work on the same calendar, and its SALT figure feeds
Schedule A line 5 under the TY2026 $40,400 cap and phase-out.
**Why.** The plan's P0 anchor is a scenario driven through the binary on 2026-08-02; D-B ("is P0 still your profile?")
was ruled by the delegate on repo evidence, and the delegate's D-A ("TY2025 is the target") was overruled by the owner
a day later — the delegate mechanism drifts. If the owner is a W-2 filer, Schedule C, SE, 8995 and 8995-A leave the
TY2026 critical path, and Schedule 1-A Parts II/III (the only REBUILT parts, `TY2026_PORT_REPORT` §3) become a
refusal ("tips/overtime inputs are not supported for TY2026") instead of a transcription.
**Cost.** One message. **Risk.** None. **How to know it worked.** `YEAR.toml forms_expected` for 2026 lists the
owner's set, and every form outside it is either `forms_absent` with a reason or a refusal with a kill.

### S3 — 1099-DA now, and the broker instruction in 2026
**What changes.** Build FR-46(b)(c) in September: `broker_reported: none | proceeds | basis` per disposition source,
box routing, and when `basis` disagrees with btctax's basis, (e) = reported, (f) = B, (g) = adjustment, exactly as
the 8949 Note says. Separately, an **owner action** this month: give each exchange a standing specific-identification
instruction (HIFO or the method the engine files) before further 2026 sales, and check what each of Coinbase, Gemini,
River and Swan will actually put in box 1e.
**Why.** This is the one TY2026 change that hits the owner's own rows, the data arrives ~2027-02-16, and the
divergence is understatement-direction in a rising year (plan review C1 — folded for the advisory, not for the input).
**Cost.** A field, a routing function, a kill-test; four emails. **Risk.** The exchanges may not honour an
instruction; then every row carries a code-B adjustment and the input surface must accept the broker's basis per lot
— which is a 1099-DA import, a larger item; knowing that in September is the point.
**How to know it worked.** A TY2026 8949 with an exchange row and no `broker_reported` answer REFUSES (the §6 rule
FR-46(c) names), and a row with `basis` prints Box A/D with (f)=B when bases differ.

### S4 — bound the machine; port by hand once; extract the generator from the diff
**What changes.** Design r2 §10 steps 1–5 land by 2026-10-31 and then the machinery stops: no `forms port` P0–P5
generator, no Layer-4 struct/filler codegen, no `forms fetch` refactor beyond promoting `archive_drafts.py`. The
TY2026 port is done by a person per form as finals land, with `forms delta`, `wire --check`, `port-status` and the
existing gates watching. After the return is filed, `git log` of the hand port is the specification of the generator.
**Why.** Steps 1–5 remove a measured, recurring class (85 hand-edits, unwired maps) and are cheap; the generator is
designed from zero observed ports, and the one time the machine measured drafts it was wrong on the two forms that
matter most (31/28 → 0/1). A generator built before one transition has been watched is the B1 failure at design
level — an instrument never seen discriminating.
**Cost.** One more year of hand edits, by a person with a diff tool. **Risk.** The owner's stated priority is the
machinery; this defers the visible half of it by ~8 months. The invisible half (globbed registry, YEAR.toml,
refusals) ships now and is what actually retires the per-year edits.
**How to know it worked.** `forms port-status 2026` reports every stem `carried` or `needs-human` with a count, and
the hand port's total human steps per form are recorded — that number is the generator's acceptance test for 2027.

### S5 — real 2026 data, twice, before finals
Run the owner's 2026-to-date exports through `report --tax-year 2026` for the 2026-09-15 estimate and again for
2027-01-15. Refresh `BundledPrices` (ends 2026-06-03; FR-53). Each run is a free intake test on the filed year. Any
adapter, price or card-reward gap found here is found five months early. Success: two runs, each with a FOLLOWUP
list, and FR-53's `prices_through` satisfied by 2027-01-02.

### S6 — ceremony sized to a one-person autumn
Keep: every compute/emitter/provenance gate; B1 kills (they are tests); the two-oracle census; executed audits on
funds-safety code (they found FR-29 and FR-31); the single whole-branch B3 review before mailing. Cut: review rounds
on documents beyond one (the repo's own rule — `CLAUDE.md` "Stop reviewing a document once findings become section X
disagrees with section Y"); persist+fold ceremony on prose; fan-outs above six lenses (a 2.4M-token run is not
something one person repeats every October); any new instrument without a named consumer on the owner's path. Write
the annual update as a runbook of commands with a measured step count, and treat the step count as the metric.
Success: the TY2027 port in autumn 2027 is executed from the runbook by one session in under a month, with the
number of human steps lower than the TY2026 count.

### S7 — the oracle fallback, ruled in September
Upgrade taxcalc to 6.8.2 now (port report §6 rule 12). Record the two known-wrong TY2026 cells as mechanism-computed
excuses (`AMT_em_pe` 639,200 vs Rev. Proc. 2025-32 §2.10's $640,200 at `RevProc_2025-32.txt:732`; `PT_qbid_taxinc_thd`
MFS). Then rule, in `ROADMAP_STATUS` §3: *if OTS 2026 is absent or disqualified on a line by 2027-03-15, the return
goes on extension; by 2027-09-15 the evidence for signing without it is taxcalc 6.8.2 + the S1 rehearsal diff + the
owner's hand-worked Form 6251 and Schedule 1-A on the official worksheets.* The form is designed to be worked by hand
(`CLAUDE.md`); for one filer's own return that is a genuine third witness even though the two-oracle rule does not
count it for "validated". Success: the ruling exists before the season, and nobody argues it in April.

### S8 — the physical last mile, rehearsed once
On the S1 packet: print it, assemble in attachment-sequence order with W-2 on the front, check every field lands
inside its box on paper, sign in the signature block, address the envelope from the IRS page (D-H: a link, not a
table), 1040-V or Direct Pay, certified mail receipt into the vault's record set. Build FR-49 (Form 4868 + 1040-V) in
the NOW window — both are year-agnostic AcroForms. Success: a one-page checklist in `design/` with the date it was
walked, and a second walk in March 2027 on the real packet.

### S9 — TY2017
Drop it. It is the only "supported" year with no evidence behind it, and under design r2 it becomes a row in every
gate. If the owner wants the 2017 crypto slice preserved, keep the tax table and delete the five forms.

## Lens answers

**1. Calendar realism.** There is a credible path only through the extension: inputs arrive W-2 ~2027-02-01,
1099-DA ~02-16; OTS 2026 prelim ~01-27; `i6251` Jan–Feb. The earliest both-oracle validation is ~mid-March, leaving
~5 weeks to April 15 for transcriptions, censuses, the pre-mail review and the packet read — one measured
review cycle (37 days/schedule). The slack is (a) the six-month extension, which the fold correctly made the default,
and (b) the entire Sep–Dec window, which is currently allocated to machinery and should carry S1/S3/S5/S8. Most likely
reason not to file: stated above — the lived journey is scheduled last, and 1099-DA is where it bites.

**2. Process cost.** Measured: 1,284 commits in July 2026, 187 in August, 122 in five September days; review/docs
vs build subjects 514:525 since 07-01; 266 review `.md` files; 12.6 MB of design prose against 260k LOC and 3,024
tests; 13 review documents for Schedule 1-A. The year-package design took one round (right-sized). The ceremony
earns its keep where it executes — FR-29 and FR-31 were found by agents running the tool, and B1 kills are cheap
tests. It has become the product where documents are reviewed to convergence (five rounds on one plan) and where
fan-outs of 20+ agents are routine. Cut those two; never cut the compute gates, the refusals, the two-oracle census,
or the B3 pre-mail review. The test of "sized to the risk" is whether one person can repeat it in autumn 2027.

**3. Scope of forms.** Yes, and the milestone should be named. For the brief's profile the after-finals
transcription set is about six forms: Sch 1 (crypto ordinary income; NO PRIOR SIDE today), Sch 1-A Parts I/IV/VI
(+6 renumber; II/III refused), Sch 2 (27 lines moved), Sch 3 (only with a 4868 payment), Sch A + 8283 (only if
itemizing — depends on S2), 6251 (one citation). `f1040`, `sb`, `sd`, `8949`, `8959`, `8960` are `unchanged` in the
work list and carry. Sch C/SE/8995/8995-A/8275 go behind the milestone unless S2 says otherwise. Everything else on
the 17-form list is a refusal for TY2026, which is the doctrine's own definition of done (`LONG_RANGE_PLAN` §1.1).

**4. The annual-update machine.** Hybrid, per S4: the binding half now (steps 1–5 — cheap, measured, removes the
85-edit class and wires TY2025 for the rehearsal), the generating half after one hand port. Data is a good home for a
year only once you have watched one year move; the repo has watched none end to end. The bet's size is otherwise
right — build.rs plus headers is small — the error is only in ordering the generator before the first port.

**5. The oracle dependency.** Before OTS 2026: taxcalc has TY2026 policy today (measured in `.venv`: `AMT_em`
90,100/140,200/70,100, `AMT_em_ps` 500k/1M, `AMT_prt` 0.5), so FR-47's KATs and a one-oracle TY2026 AMT census can
run this month; the S1 rehearsal is a filer-specific witness for the engine; a parameter-swap invariance test (TY2026
engine with TY2025 params ≡ TY2025 engine, which OTS 2025 + taxcalc already witness) validates the machinery
independently of the constants. Honest fallback: S7 — extension, then a pre-ruled evidence set; never a relaxed
rule, and never a single-oracle "validated". OTS's AMT defects are already handled by mechanism-computed excuses
(port report §6 rule 13); keep that shape for 2026.

**6. Missing pieces.** (a) The broker specific-ID instruction is a 2026 act, not 2027 code (S3). (b) No physical
print is evidenced anywhere (S8). (c) Charitable BTC above $5,000 needs a qualified appraisal signed before filing —
an owner action with a deadline that the plan does not calendar. (d) The state return is correctly unbuilt but
absent from the calendar; its SALT figure is a Schedule A input. (e) Estimated payments: advise-only is right, but
the two 2026/27 dates are free real-data exercises (S5). (f) The price dataset through 12-31 (FR-53) is a January-2
task with no owner. (g) A mid-season re-issue is covered by `template_sha256` (FR-52c) — documentation only.
(h) Intake: four venues; if the owner used any other in 2026, intake is a locked door — ask in S2.

**7. What to STOP doing.** See the section below.

## What to stop

- **The `forms port` generator and Layer-4 codegen before the first hand port** (S4). Defer to May–Aug 2027.
- **Sch C / SE / 8995 / 8995-A TY2026 ports and Schedule 1-A Parts II/III transcription** unless S2 puts them on
  the owner's return; refuse instead.
- **FR-51's "widen `LineCoverage` until step 24's residual shrinks to nothing"** — unbounded; do it for the owner's
  forms only.
- **Multi-round document reviews and 20+-agent fan-outs as routine** (S6). One round, then execute.
- **TY2017** (S9).
- **Release/publish work during the season** (D-F: no bearing on filing; no users). Revoke the token, then nothing.
- **The retirement-income spec** — already parked; keep it parked (D-C).
- **330 TY2025 census readings** — D6 already says no; the rehearsal does not need them.
- **Re-deriving the calendar.** It is written four times (`LONG_RANGE_PLAN` §6, `ROADMAP_STATUS` §3, the port
  report §1, the plan review I5). One tracker, one calendar.

## A proposed calendar, Sep 2026 → Oct 2027

| month | work | slack |
|---|---|---|
| 2026-09 | S2 owner answers; S3 1099-DA input + broker instructions; FR-47 `AmtParams` TY2026 + statute archive; taxcalc 6.8.2; S7 ruling; design r2 steps 1–3; S5 run for the 09-15 estimate | — |
| 2026-10 | design r2 steps 4–5, **machinery hard stop 10-31**; `FullReturnParams` TY2025; three authorities; delete the TY2025 gate; **S1 rehearsal + S8 physical print**; Rev. Proc. 2026-xx (TY2027) lands — ignore (rule 16) | whole month is pre-finals slack — spend it on S1 |
| 2026-11 | first finals (1040, Sch 1/2/3/B/D, 8949 typically): fetch, hash, carry, census per form as they land; FR-49 4868 + 1040-V; fold the rehearsal's follow-ups | — |
| 2026-12 | remaining finals (Sch A, 1-A, 6251): the two Sch A worksheets, the 1-A renumber, 6251 citations; refusals for out-of-set forms with kills | — |
| 2027-01 | `i1040gi`/`i6251` finals; S5 run for the 01-15 estimate; prices through 12-31 (01-02); season opens ~01-26; OTS 2026 prelim ~01-27; W-2 ~02-01 | — |
| 2027-02 | 1099-DA/INT/DIV ~02-16; import; OTS-2026 two-oracle census; first TY2026 packet as DRAFT; owner hand-works 6251 and 1-A | — |
| 2027-03 | the one B3 Fable pre-mail review; fold; packet read; decide April vs 4868 by 03-31 | — |
| 2027-04 | 04-15: mail signed return + payment, or 4868 + payment (default) | **the extension: six months** |
| 2027-05 | if extended: finish and file; else retrospective — count the hand port's human steps | slack |
| 2027-06 | extract `forms port` from the hand-port diff; runbook as commands | slack |
| 2027-07 | TY2027 drafts begin; `forms port-status 2027` on drafts (evidence only) | slack |
| 2027-08 | generator dry-run on TY2027 drafts; B1 kills | slack |
| 2027-09 | 09-15 estimate on real 2027 data; nothing else scheduled | slack |
| 2027-10 | 10-15 extended deadline — last slack; Rev. Proc. for TY2027 late in the month | **last slack** |

## What this review did NOT examine

Code line by line; design r2's internals (folded, settled); the crypto engine and its September fixes; the
correctness of any tax position; the oracle scripts' internals; whether the owner's four exchanges accept a
specific-ID instruction (S3 assumes it must be asked); the TUI; the older `design/direction/` documents beyond the
cited lines; anything under `/tmp` or any transcript.
