# LONG-RANGE PLAN — from here to a FILED federal return, and then to a cheap second year

**Status: r1** (written 2026-09-04, on `feat/schedule-1a-ty2025` @ `3d83e601`; `main` @ `2b34c13c`). **This is a sequencing document.**
It writes no spec and no code; it names which specs and plans get written, in what order, with what
entry and exit gate, and — the part the repo does not have anywhere else — **what "done" means** for a
product whose stated boundary is *"Federal only · Print-and-mail (no e-file)"*
(`crates/btctax-cli/LIMITATIONS.md:3`).

It synthesizes five recon reports and re-derives none of them:

| report | what it settled |
|---|---|
| `design/agent-reports/2026-09-04-recon-federal-completeness.md` | what a complete return needs that btctax cannot produce |
| `design/agent-reports/2026-09-04-recon-efile-procedure.md` | e-file is **closed** to this product as built |
| `design/agent-reports/2026-09-04-recon-last-mile.md` | the six areas between a correct PDF and a filed return |
| `design/agent-reports/2026-09-04-recon-open-work.md` | the de-duplicated open-work table, ranked |
| `design/ty2025/recon-year-port-delta.md` | the measured cost of a TY2024→TY2025 form port |

**Binding process.** `STANDARD_WORKFLOW.md` §1 (Brainstorm → Spec → Plan → Implement → whole-diff
review → Ship), §2 (independent review to 0 Critical / 0 Important, persist verbatim *before* folding),
§6 (severity). `CLAUDE.md`'s transcription rule, two-oracle rule, and harness B1/B2/B3. Every phase
below closes on the same **five gates** the TY2025 spec already fixes (`design/ty2025/SPEC.md:520`):
`make check` · `cargo fmt --all --check` · `cargo +1.88 check --workspace --locked` ·
`cargo run -p xtask -- check-isolation` · `bash scripts/pii-scan-generic.sh`.

**Measured this session, not estimated** (commands, not hand counts):

```
$ cargo nextest list --workspace | wc -l                     2767          tests   (@ 3d83e601)
$ cargo run -p xtask -- line-coverage
  line-coverage OK: 279 money lines across 16 form(s) … 15 exception(s) (ratchet 15),
  0 unverifiable (ratchet 0), 8 not line-bound (ratchet 8)
$ ls crates/btctax-forms/forms/{2017,2024,2025}/*.map.toml | …   5 / 17 / 5   map+PDF pairs
```

---

## 1. "DONE" — the definition this plan is written against

### 1.1 The three axes of done, and why "support everything" is not one of them

A return is **filed** when a human has signed a paper Form 1040 packet under §6065 and mailed it. Three
things must be true before that packet exists, and they are independent:

| axis | done means | authority |
|---|---|---|
| **channel** | a printed AcroForm packet + everything the envelope needs (voucher, address, attachment order) | `LIMITATIONS.md:3`; recon-efile §5 |
| **year** | `full_return_for(<year>)` returns `Some`, which is the **only** year gate on the full-return path | `crates/btctax-adapters/src/tax_tables.rs:107`; `design/ty2025/SPEC.md:302-311` |
| **profile** | every line the *named* filer's facts touch is computed correctly **or refused loudly** | `crates/btctax-core/src/tax/return_refuse.rs:36` (**62 refusal variants**, counted) |

★★ **The refusal surface is part of "done", not a gap in it.** This is the single most important
framing decision in this document, and it is the repo's own doctrine rather than a new claim: a return
that refuses is a return the filer knows to take elsewhere; a return that files a fabricated figure is
signed false testimony. `return_refuse.rs:378-403` destructures `ReturnInputs` exhaustively so a new
input field cannot be added without being classified. **Therefore "done" never requires Schedule E, or
Form 8962, or a second SE earner** — it requires that a filer who has one is *stopped*, which is
already true.

The corollary is the whole reason this plan can be finite: **completeness is measured against a named
filer, never against the tax code.**

### 1.2 The filer profiles — in scope and out

The owner's own scenario is on the record and is the anchor (`design/direction/FILING-TRIAL-2026-08-02.md:14-20`,
verbatim):

> $85,000 earned income self-employed · $85,000 donation to church · $2M capital gains · 9 dependent
> children · $375,000 medical expenses · $10,000 student loan interest · $2,500 car loan interest

| profile | who | in scope? | what it costs |
|---|---|---|---|
| **P0 — the owner's return** | self-employed (mining) Schedule C · large LTCG · 9 dependents · itemizing with a large charitable gift and a medical floor · **$2,500 car-loan interest, which is a TY2025 Schedule 1-A item** | **YES — the definition of done** | everything in §3 |
| **P1 — the core crypto filer** | W-2 and/or crypto Schedule C · disposals → 8949/Sch D · standard or itemized · interest/dividends · possible AMT, NIIT, §199A | **YES** | P0's work covers it; P1 ⊂ P0 except the dependent grid |
| **P2 — the ordinary household** | P1 **plus** retirement income (1040 lines 4a–6b) and/or a CTC/EIC the family is owed | **NOT on the critical path** — see §7, ★ D-C | its own spec + build per item |
| **P3 — out of scope, permanently for v1** | Schedule E/F · K-1 · second SE earner · Form 8962 (marketplace ACA) · non-passive foreign tax · Form 3520 · kiddie tax · clergy SE exemption · dual-status alien · **any state return** | **NO** | already refused; keep refusing |

★ **P0 is not hypothetical and not aspirational — it has already been driven through the shipped
binary.** The filing trial found eleven blockers; nine are closed. What still stops P0 is exactly two
things: **the year** (B7 in that document: *"TY2025 is not supported, so the car loan interest has
nowhere to go"*, `FILING-TRIAL-2026-08-02.md:190-197`) and — until it closed mid-session — provenance
on 1040 line 19 (§4, R2). ★★ **With FR-1 merged, the YEAR is the only thing left between P0 and a
filed return.** That is the whole justification for §3's sequencing.

★★ **A finding worth stating before it is mis-sequenced.** P0 has **no retirement income, no rental,
no K-1, and no marketplace coverage.** The retirement-income spec now in flight
(`design/ty2025/SPEC_retirement_income.md`, DRAFT r1) closes the **largest P2 gap** — it is genuinely
the right next completeness item — but it is **not on P0's critical path**, and putting it there costs
the first filed return. Finish it to green; sequence its *build* per ★ D-C.

### 1.3 What "done" explicitly does NOT include

Not e-file (§7.1). Not state (§7.2). Not direct deposit (a paper check is the disclosed consequence,
`LIMITATIONS.md:222`). Not Form 2210 (the i1040 instructions make the line optional and the IRS bills
the penalty — recon-last-mile §3). Not a second §24(b) implementation inside `btctax-forms`
(`FOLLOWUPS.md:5355-5359`, and the forms lane was right to refuse it).

---

## 2. THE FACTS THAT BIND — established, not re-litigated here

**F-1. TY2025 is the year being filed, and it is fail-closed.** `full_return_for(2025)` returns `None`
from a deliberate, tested, mutation-verified refusal (`tax_tables.rs:814-822`). Its doc comment states
the unblock condition in four numbered clauses (`tax_tables.rs:801-810`) — this is the definition of
Phase 1+2's exit gate and it is quoted in full in §3.

**F-2. Two of the four clauses have already landed.** Verified in source, not from a report:
`SaltLimitation::Worksheet2025` (`crates/btctax-core/src/tax/tables.rs:335-355`) is the §164(b)
worksheet, not a scalar — clause 2. `Form6251Line1::Y2025 { line1a, line1b }`
(`crates/btctax-core/src/tax/form6251.rs:60`, computed at `:471-474`) and the 1040's third term
(`crates/btctax-core/src/tax/printed.rs:737-739`, *"Add lines 12e, 13a, and 13b"*) are clause 3.
**Clause 4 (Schedule 1-A) and clause 1 (the `FullReturnParams` themselves) remain.**

**F-3. Schedule 1-A is 1 of 7 tasks built.** `Schedule1aParams` exists (`tables.rs:1017`, landed
`94fa025e`, 2026-07-29 — T1). `crates/btctax-core/src/tax/schedule_1a.rs` **does not exist**; the
symbols `tips_deduction`, `overtime_deduction` and `car_loan_interest` appear **zero times** in
`crates/` (grep, this session). T2–T7 are unbuilt.

**F-4. TY2025 ships 5 map/PDF pairs, and they are SLICE maps, not full-return maps.** Counted this
session — TY2025's existing maps are a small fraction of their TY2024 equivalents:

| stem | 2025 map | 2024 map |
|---|---|---|
| `f1040` | **10 lines** | 293 |
| `schedule_d` | 20 | 120 |
| `f8283` | 45 | 234 |
| `f8949` | 46 | 85 |
| `schedule_se` | 24 | 85 |

The fill layer refuses on exactly this: *"the TY{year} Form 1040 map has no `{what}` — the full-return
fill needs it. Full-return v1 is TY2024-only."* (`crates/btctax-forms/src/form1040_full.rs:53-59`).
★ **So the asset work is not "12 new maps." It is 13 new maps (12 ports + Schedule 1-A, which exists
for no year — `find crates -name "*1a*map.toml"` returns nothing) PLUS 5 rebuilds** = **18 map
artifacts**.

**F-5. E-file is closed to this product as built.** MeF transmission requires an EFIN/ETIN held by an
Authorized IRS e-file Provider; there is no personal-use tier; the Online Provider security profile
(EV-SSL, CAPTCHA, privacy-seal audit) presupposes a website; ATS re-testing recurs every tax year
(recon-efile §2, §4(d), §6). **Consequence, and it is a gain, not a loss:** prior-year AGI and the
Self-Select PIN are *correctly N/A* rather than gaps (recon-last-mile §4).

**F-6. The port is cheap for 8 of 12 forms and not for 2.** Form 8275's TY2025 PDF is **byte-identical**
to TY2024's (sha256 match, recon-delta §1). Forms 8959/8960 have zero field-geometry differences.
Schedule A (new SALT cap + a brand-new MAGI phase-out) and Form 6251 (Part I restructured around
Schedule 1-A) are structural, and both are named by `tax_tables.rs`'s own gate.

**F-7. Seven agents and one workflow are in flight** (`CONTINUITY.md` §③, 2026-09-04). Do not
duplicate any of them; this plan assumes each lands:

| in flight | lands at | this plan's dependency |
|---|---|---|
| review of the Schedule 1-A plan's **r4 fold** | `design/ty2025/reviews/PLAN_schedule_1a-r4fold-review.md` | **Phase 1 entry gate** |
| **SPEC Schedule A TY2025** | `design/ty2025/SPEC_schedule_a_ty2025.md` | Phase 2, "new tax logic" tier |
| **SPEC Form 6251 TY2025** | `design/ty2025/SPEC_form6251_ty2025.md` | Phase 2, "new tax logic" tier |
| **SPEC retirement income 4a–6b** ✅ landed | `design/ty2025/SPEC_retirement_income.md` | Phase 6 (★ D-C) |
| **label-reader fix** (drops Form 6251 line 1a) | branch `fix/label-reader-drops-1a` | **Phase 2 prerequisite** (R7) |
| **understatement audit** (6 lenses + adversarial verify) | `design/agent-reports/2026-09-04-understatement-audit.md` | may re-rank §4 |
| ~~**FR-1 build**~~ ✅ **LANDED while this plan was being written** | `5094bfc5`, merged `3d83e601` | R2 closes |

★★ **FR-1 is CLOSED and the fix is the one the follow-up specified** (`FOLLOWUPS.md:5351-5362`):
`Form1040Lines::line19` is now `Option<Usd>` (`crates/btctax-core/src/tax/printed.rs:552`), the §24(b)
predicate is exposed **once** as `advisories::ctc_odc_line19(ri, agi) -> Option<Usd>`, and
`btctax-forms` contains no §24(b) reasoning. ★ A **provable** phase-out still prints `0`, because
Schedule 8812 line 12-No instructs *"Enter -0-"* — the distinction this project's answered-ness
doctrine turns on.

---

## 3. THE CRITICAL PATH TO THE FIRST FILED RETURN

Six phases. **Phases 1, 2 and 3 are the critical path and nothing else is.** Each phase's exit gate is
this repo's `green` — the five gates pass **and** an independent review returns 0 Critical / 0 Important
(`STANDARD_WORKFLOW.md` §2).

```
  P1 Schedule 1-A ──▶ P2 filing assets ──▶ P3 gate deletion + packet trial ──▶ FILED
        (B3)              (B4)                    (delete the test)
                                                        │
  P4 last mile ─────────────────────────────────────────┤   (parallel, small, must precede FILED)
  P5 year-port machine ─────────────────────────────────┴──▶ TY2026 cheap
  P6 P2-profile completeness (retirement, CTC, EIC) ── off the critical path entirely
```

### Phase 0 — two owner answers (blocking, ~one pass)

**Entry:** this document. **Exit:** ★ D-A and ★ D-B (§8) recorded in `CONTINUITY.md`.

Everything below assumes P0 is the filer and TY2025 is the year. If either is wrong the sequence is
wrong, and it is wrong cheaply now and expensively in six weeks.

### Phase 1 — B3, Schedule 1-A end to end (CRITICAL PATH)

**Entry gate:** `design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md` at **0C/0I** — the r4 fold's review
is in flight (F-7) and this plan does not front-run it. **Exit gate:** the plan's own §1 exit criteria
(all 48 line labels present; every part's phase-out tested at its own knee in its own direction; TY2024
provably unmoved at golden md5 `c4e1853ed82d113ca5cd97ffd8abbf47`; mutation-verified guards; TY2029+
fails closed).

Tasks T2–T7 as written in that plan. **Do not re-plan them here.** Two things this document adds:

★ **T2 is the chokepoint and it is where the plan's own r4 findings live** — completion predicates
(C-I2), the line-5 net-income ceiling (C-1), line 22's arity, and the `Production` provenance half of
the conformance KAT (B-I1). A shape decided wrong at T2 cannot be repaired at T4.

★★ **Two of the r4 findings are understatement-direction eligibility gaps and therefore rank above
everything else in this phase** — the Part IV refinance balance cap (C-I3) and the Part II
illegal-service exclusion (C-I4). See §4, R3.

**Not in this phase:** the PDF and the AcroForm map (B4), `FullReturnParams` for TY2025 (last), and
Schedule 8812 (§7).

### Phase 2 — B4, the filing assets (CRITICAL PATH)

**Entry gate:** Phase 1 green. **Exit gate:** 18 map artifacts committed, each with (a) a `dump-fields`
field inventory that matches, (b) a per-line printed-text check, (c) a `[census]` disposition with a
written reason for every unmapped field, and (d) the field-provenance census re-run.

**The work, sized by the measured delta** (recon-delta §4), ordered cheapest-first so that the two hard
forms are attempted with the tooling already exercised:

| tier | forms | port cost |
|---|---|---|
| zero | `f8275` | byte-identical PDF; copy the map, re-verify the hash |
| mechanical | `f8959`, `f8960` | 0 field-geometry differences |
| small | `f8995`, `f8995a`, `f1040sb`, `f1040s3`, `f1040s1` | threshold constants + a few new checkboxes to census |
| **human decision** | `f1040s2`, `f1040sc` | Schedule 2's line 10 went *"Reserved for future use"*; **Schedule C's 27a/27b traded content while the field names stayed pinned** |
| **new tax logic** | `f1040sa`, `f6251` | SALT worksheet (built in Phase 1's predecessor B2) + a changed AcroForm root subform; 6251 Part I depends on Schedule 1-A |
| **new form** | `f1040s1a` | no prior year exists |
| **rebuild** | `f1040`, `schedule_d`, `f8283`, `f8949`, `schedule_se` | slice maps → full-return maps (F-4) |

★ **Three PDFs are not archived yet** — `f1040s1--2025`, `f8275--2025`, `f8995a--2025` are absent from
`design/forms/2025/` (`ls`, this session). The recon fetched them to scratch only. Archiving them
through `authority-manifest` is Phase 2's first commit, not an afterthought.

★★ **The printed-text diff gate lands HERE, not in Phase 5.** See §5.2 — it is small, and Schedule C's
letter swap is the trap it exists to catch, firing in this phase.

★★ **The label reader must earn B1 before it is trusted here.** Run fresh against Form 6251 TY2025 it
**silently dropped line "1a"** — the exact line Pub. L. 119-21 added (recon-delta §3). Under this
repo's own B1 rule an instrument that has never been watched going red on a *restructured* form is not
a checker. The in-flight fix (F-7) must land with a planted-defect test on a restructured form.

### Phase 3 — delete the gate, and drive P0 end to end (CRITICAL PATH)

**Entry gate:** Phases 1 and 2 green, and all four clauses of `tax_tables.rs:799-810` true. **Exit
gate:** an emitted TY2025 packet for P0, read line-by-line against the forms, plus the two-oracle sweep
green for TY2025.

This phase deletes `ty2025_full_return_must_stay_fail_closed_until_complete` and nothing else. Its own
doc comment is the checklist and is quoted here so nobody has to go find it:

> **Before this assertion comes out, ALL of the following must be true** … 1. `AmtParams` and every
> other `FullReturnParams` field carry TY2025 values, each with its own citation … 2. `salt_cap` is the
> §164(b) **worksheet**, not a scalar … 3. Form 6251 Part I is re-transcribed for 2025 … and the 1040's
> own 11a/11b/12e/13a/13b shape is modelled. 4. Schedule 1-A exists — all six parts — with every
> numbered line collected.

★★ **Clause 3's second half is not yet true and is easy to mis-read as done.** `Form1040Lines`
(`printed.rs:501-588`) still declares `line11`, `line12`, `line14` — the *computation* carries the
TY2025 shape (`printed.rs:739`) but the printed struct does not carry the 2025 sub-line names, and
every dependent schedule's cross-reference moved (*"Form 1040 line 11" → "line 11b"*, recon-delta §2).
Decide in Phase 2 whether the struct renames or the map absorbs it; do not let Phase 3 discover it.

★★★ **The exit gate is a PACKET read, not a per-form one.** This is the repo's most expensive learned
lesson: `FOLLOWUPS.md:1617-1642` records two Criticals sitting in the composition of Forms 1040 and
8995-A while 2601 tests were green across five gates, both found *"within minutes of reading the
emitted `55A_f8995a.pdf` beside the emitted `00_f1040.pdf` — the only place the two forms are side by
side."* Phase 3 is that read, for TY2025, on P0.

### Phase 4 — the last mile (small; must precede FILED; NOT on the arithmetic critical path)

**Entry gate:** none — this can start any time and should start during Phase 2's slack. **Exit gate:**
a filer holding the packet can post it without consulting anything outside it.

Recon-last-mile found six areas with **no tracking artifact anywhere in the repo**. Ranked by whether
the omission can cost the filer something:

| item | why it matters | verdict |
|---|---|---|
| **Form 1040-V** | a filer who owes (P0 owes) needs the voucher to have the payment credited correctly; no filler exists and it is not even named as an exclusion | **BUILD** — one AcroForm, no logic |
| **mailing address** | state-dependent AND payment-dependent, six service centers, and the IRS corrected the 1040-ES addresses mid-2026 (recon-efile §5) | **BUILD as a link + the two facts**, not a bundled table that rots |
| **attachment order + what to physically staple** | W-2s/1099s with withholding on the front; schedules in attachment-sequence order | **BUILD** — extend `hand_marks` (`crates/btctax-cli/src/cmd/admin.rs:360`), which already exists and is already conditioned |
| **spouse IP PIN** | a named capture gap; *"a return omitting an issued spouse IP PIN is rejected"* (`forms/2024/f1040.map.toml:267-276`); the taxpayer's is modelled (`packet.rs:159-183`) — the asymmetry is the defect | **BUILD** — *"closing it is one optional field on `ReturnInputs`"* |
| **record retention** | crypto basis traces back years, so the window that matters is holding period + 3 years, not the generic 3 | **DOCUMENT** in the manifest; `export-snapshot` already produces the artifact, unprompted |
| **Form 1040-ES** | P0's shape (large one-off gains) is the population most likely to owe an estimated-tax penalty *next* year | **ADVISE ONLY** — no quarterly-income model exists, and building one is a project |
| Form 2210 / line 38 · direct deposit · Form 8888 · line 36 · third-party designee | blank is the instructed default, or a considered omission | **DO NOT BUILD** (§7.4) |

★ **`broker_reported_rows: 0` is hardcoded on the full-return export path**
(`crates/btctax-cli/src/cmd/admin.rs:1200`), so a full return with exchange disposals never gets the
[I5] broker-reporting advisory that the crypto-slice path does get. One line; fix it in this phase.

### Phase 5 — the year-port machine (§5). **Off the critical path for the first return, ON it for the second.**

### Phase 6 — P2-profile completeness. **Off the critical path entirely** (§7.5, ★ D-C).

---

## 4. RANKED BY FILING RISK — the order the work closes in

**The ranking rule**, from this repo's own severity doctrine (`CLAUDE.md`, "an entry is testimony";
`STANDARD_WORKFLOW.md` §6): a **wrong number on signed testimony** outranks a **refusal that blocks the
filer**, which outranks **money left on the table** (an overstatement of tax), which outranks
**provenance and cosmetics**. Direction matters: an *understatement* of tax is the one that draws a
penalty.

| # | risk | class | direction | phase |
|---|---|---|---|---|
| **R1** | **the year gate deleted before all four clauses are true** | wrong number, systemic | both | **P3 — the whole reason P1/P2 exist** |
| ~~**R2**~~ | 1040 line 19 printed an unconditional `0` (FR-1, `FOLLOWUPS.md:5351`) | **signed false testimony** | overstated | ✅ **CLOSED 2026-09-04**, `5094bfc5` |
| **R3** | Schedule 1-A Part IV refinance balance cap + Part II illegal-service exclusion (plan r4 C-I3/C-I4) | wrong number | **UNDERSTATES** | **P1/T3** |
| **R4** | Form 6251 filled-PDF field readback (AMT **E4**, `FOLLOWUPS.md:522-560`) — no oracle or vector can see an AcroForm transposition | wrong number | either | **P2** |
| **R5** | Schedule C 27a/27b content swap on the TY2025 port (recon-delta §2) | wrong number | either | **P2** |
| **R6** | Schedule 2 line 10 → *"Reserved for future use"*, same number, different meaning | wrong number | either | **P2** |
| **R7** | the label reader drops restructured lines (6251 line 1a) — a **green and blind instrument**, B1 | instrument | masks any of R3–R6 | **P2, before the ports** |
| **R8** | AMT **E6** — Form 6251 lines 2c–2t silently zero rather than provenance-carrying (`return_refuse.rs:940-1000` refuses instead, correctly) | refusal | fail-closed | **P6** |
| **R9** | Schedule 1-A lines 5 / 14b have **no input path at all** — no 1099-NEC/MISC/K struct exists (plan T3a) | fabricated blank | must refuse, not zero | **P1/T3a** |
| **R10** | FR-17 — a `Computed` capital-loss carryover stamp has no retraction path | wrong number, narrow | either | **P1 (its owning phase is "whichever lands TY2025")** |
| **R11** | FR-19 — `--force` promises an overwrite the `grounded` gate refuses to perform | UX; could send a filer to "fix" a correct number | none | **P4 (one conjunct)** |
| **R12** | EITC/ACTC not computed (FR-16) — $8,781 forfeited on the repo's own MFJ/$40k/2-child measurement | money forfeited | overstates | **P6, deferred (★ D-G)** |
| **R13** | retirement income 4a–6b unmodelled; caught only by the catch-all scope attestation (`questions.rs:546-556`) | refusal, fragile | fail-closed | **P6 (★ D-C)** |
| **R14** | FR-15 — the field-provenance census is a hand-run snapshot with no generator and no test | instrument decay | none | **P5** |
| **R15** | FR-12/13 (8960 9a/9d), FR-4 (TUI carryover reader), G-12 (8275-R), G-13 residue (8283 5a/5b/5c), FR-2 (TY2017 8283), FR-5 (Chart A) | narrow / cosmetic | mixed | **batched, ownerless** |

★★ **R2 closed while this plan was being written, and WHY it mattered is worth keeping.** For P0 —
$2M of capital gains — the §24(b) phase-out had already reduced the credit past zero, so **`$0` on line
19 was the correct amount** on that return. The repo has met this exact case:
`crates/btctax-core/src/tax/advisories.rs:51` records *"§24(b) had already reduced by $84,250. Line 19
was `$0` and correct; the ADVICE was not."* So on P0's return FR-1 was a **provenance** defect, not a
dollars defect — the zero was right, but it was a hardcoded zero indistinguishable from a computed one.
**On a P0 year without the outsized gain, nine children make it a dollars defect**, which is exactly why
the `Option<Usd>` type — not a value — is what separates the two years. This is the shape to look for in
the rest of the table: R9 is the same defect on Schedule 1-A lines 5 and 14b, unbuilt.

★ **R4 deserves its rank despite Tier 1 having shipped.** E4 is a correctness gap on the population
Form 6251 *already serves today*, not on Tier 2's — a correct computation can still file through a
transposed AcroForm field, and nothing catches that. It becomes strictly worse in Phase 2, where twelve
maps are authored at once.

---

## 5. MAKING THE SECOND YEAR CHEAP

### 5.1 What was measured

Across the 17 TY2024 `.map.toml` files (counted this session, whole directory):

```
total lines 2669  ·  comment/prose lines 1257 (47%)  ·  `lineN = "field"` mappings 239
`rule = …` census dispositions 555
```

So **roughly half of a map is hand-written provenance prose** — *why* a field is `unmodeled` — and that
half is a domain judgment about what btctax's engine computes. It is not derivable from the PDF, and
this plan will not pretend otherwise.

The mechanical half already has tooling: `cargo run -p xtask -- dump-fields <pdf>` (exact field
inventory), `extract-geometry` / `label-boxes` / `label-census` / `label-proof` (the geometric
line↔field correlation, `crates/xtask/src/main.rs:34-181`). **What does not exist is anything that
carries a map from one year to the next.**

### 5.2 The instrument — split in two, because the halves earn their cost at different times

★★★ **`xtask port-map <stem> <from-year> <to-year>` — and its product is a REFUSAL, not a copy.** Same
shape as `income scrub`: the value is the authorization to reuse last year's judgment, not the file.
Three passes:

1. **Field-set diff.** `dump-fields` both PDFs. Identical name set ⇒ the map may be copied verbatim with
   only `year` and the PDF hash re-pointed. Any delta ⇒ print it and **refuse to copy**.
2. **Per-line printed-text diff.** For every `lineN` in the map, compare that line's own printed text
   between `design/forms/extract/<stem>--<from>.txt` and `--<to>.txt`. Any change reds and names the
   line. **This is the pass that catches Schedule C's 27a/27b swap and Schedule 2's "Reserved for future
   use" — neither of which a field-name diff can see**, because in both cases the field names are
   exactly what did *not* change.
3. **Census carry-forward, conditioned.** A `rule = "unmodeled"` reason is carried only when *both* the
   field name and the line text are unchanged; otherwise it is emitted as an undecided disposition that
   the census test reds on. A reason that no longer describes the form is worse than no reason.

★ **Per B1, this lands with a planted-defect test**: rename one field and change one line's text in a
fixture, assert red on each. An honest kill-test here cannot be written without discovering whether the
tool actually reads the text layer.

**Where each half earns its cost:**

| half | lands in | why there |
|---|---|---|
| **pass 2 (the printed-text diff gate)** | **Phase 2** | it is small, and R5/R6 — the two sharpest traps in the whole 12-form port — fire in Phase 2. Building it later means paying for the trap first and the defense second. |
| **passes 1 and 3 (the copy/scaffold)** | **Phase 5**, after the first filed return | it cannot pay for itself on a port that happens once; it pays from TY2026 onward. Built before Phase 2 it would be designed against one year of evidence and rewritten. |

### 5.3 The honest limits — where the answer is "not yet", and one place it is "never"

- **The provenance prose (47%) is not automatable.** Deciding that a field is `unmodeled` *because
  btctax collects no RRTA income* is a statement about the engine, not about the PDF.
- **A new checkbox is a product decision.** Schedule 1 gained four and Schedule 2 gained three
  (recon-delta §2); each is "model it or census it", which is judgment.
- **A structurally rewritten form is not a port at all.** Schedule A's SALT worksheet and Form 6251's
  Part I are tax logic in `btctax-adapters`, and no map tool touches them. TY2026 is unlikely to
  restructure like TY2025 did — Schedule 1-A's four provisions run through **TY2028** and are not
  indexed (`IMPLEMENTATION_PLAN_schedule_1a.md` T1, S-7) — but a plan that assumes no year restructures
  is the plan that gets surprised.
- **FR-15 belongs here.** The field-provenance census (`design/forms/FIELD_PROVENANCE.md`) is a dated
  snapshot with no generator and no test, so the 496/396 unaccounted-box numbers can silently go stale
  precisely when a new year's maps are authored. Give it a generator in Phase 5 or the port machine
  will be checked against a stale baseline.

**Net honest answer to "does the second year become a table entry?"** For **8 of 12 ports, yes**, and
for one (Form 8275) it already is. For the two forms the law rewrote, **no**, and no tooling changes
that. The realistic claim is: *the port machine converts a 12-form year from twelve judgment calls into
two judgment calls plus ten verified copies* — which is the difference between a phase and a week.

---

## 6. THE CALENDAR — the honest answer

### 6.1 The dates

| date | what | days from 2026-09-04 |
|---|---|---|
| 2026-04-15 | TY2025 regular deadline — **passed** | −142 |
| **2026-10-15** | **TY2025 extended deadline** (recon-efile §5) | **41** |
| ~2027-01-26 | TY2026 season opens | 144 |
| 2027-04-15 | TY2026 regular deadline | 223 |

### 6.2 TY2025 by 2026-10-15 is NOT reachable. Stating it plainly.

The evidence is velocity, not pessimism:

- **T1 landed 2026-07-29** (`94fa025e`). **37 days later, the TY2025 track has produced no further
  implementation commit** — `schedule_1a.rs` does not exist and `tips_deduction` /
  `overtime_deduction` / `car_loan_interest` appear **zero times** in `crates/` (F-3). August was
  consumed by the filing-readiness branch (65 commits, merged `945d1ac2`).
- **The Schedule 1-A artifact stack has consumed spec r1→r3 and plan r1→r4 in those 37 days, and a
  fifth review is in flight** (`design/ty2025/reviews/` holds 8 documents). This is the gate working
  correctly — the r1 round found 2 Criticals, both *missing eligibility*, both understatement-direction
  — and it is also the measured rate.
- **Phase 1 is 6 of 7 tasks**, one of which (T3) is *"landed whole in one pass"* through
  `return_inputs.rs` → `classifier.rs` → `questions.rs` → `return_refuse.rs` → input-form → CLI → TUI.
- **Phase 2 is 18 map artifacts** (F-4), each needing a field census, a line census, and a written
  disposition per unmapped field — 555 such dispositions exist for TY2024.
- Each phase closes on **five gates plus an independent review to 0C/0I**, and this repo's own record
  (`FOLLOWUPS.md:1605-1616`) is that **the fold carries the defect**: three consecutive rounds each
  found a Critical or Important in the *previous round's fix*.

★ **The counter-evidence, stated fairly, because it is real.** On 2026-09-04 alone the repo merged an
archive-reconciliation branch, an understatement fix to the scope attestation, and the whole FR-1 build
(`3d83e601`), while running seven agents and a workflow in parallel (F-7). That is a genuinely high
rate. It does not change the verdict, for one reason: **every one of those is a single-file or
single-follow-up change, and neither Phase 1 nor Phase 2 is.** T3 alone threads seven crates in one
pass, and Phase 2 authors 18 map artifacts against forms whose traps (R5, R6) are invisible to a diff.
A high commit rate on small items is not evidence about a phase-sized one.

**Reaching 2026-10-15 would require compressing all of that into six weeks, and the only way to do it is
to delete the fail-closed gate early — which is precisely the outcome its own doc comment forbids:**
*"A partial landing that deletes this early is not a smaller version of TY2025 support. It is a
silently wrong return, which is the one outcome this project refuses."* (`tax_tables.rs:812-813`).

### 6.3 What that means, and why it is less bad than it sounds

★★★ **October 15 is a penalty deadline, not a product deadline.** A TY2025 return filed after the
extended date is still a valid return; what changes is penalty and interest exposure on **unpaid tax**.
So the choice is not "file with btctax or never" — it is:

| option | what happens | what is sacrificed |
|---|---|---|
| **A — recommended.** Build TY2025 properly (P1→P3), file TY2025 **late** with btctax; TY2026 is then a cheap port filed **on time** in the 2027 season | one late federal filing; the TY2025 work is not wasted, because TY2026's forms are TY2025-shaped (Schedule 1-A runs through TY2028) | timeliness of one year |
| **B.** File TY2025 by 2026-10-15 **by other means** (a preparer, or FFFF — manual entry, no import, recon-efile §1), then use btctax's TY2025 packet as a check | nothing technical; costs money and a day | nothing in this plan |
| **C.** Skip TY2025; target TY2026 only | TY2025 is filed entirely outside btctax, and the P0 car-loan-interest deduction is hand-computed | ~all of Phase 1/2 still has to be built for TY2026 anyway — **TY2026 needs Schedule 1-A too**. This option saves nothing. |
| **D — REJECTED.** Ship a partial TY2025 by deleting the gate | plausible wrong numbers on signed testimony | the one thing this project refuses |

★ **Option C is the trap worth naming.** "Give up on TY2025 and aim at TY2026" *feels* like the
schedule-respecting choice and buys nothing: Schedule 1-A, the SALT worksheet and the restructured Form
6251 are all TY2026 requirements too. **The TY2025 build IS the TY2026 build.** The only real question
is which year gets filed with it first.

★★ **A fact the plan rests on but cannot cite from this repo:** the failure-to-file penalty is computed
on *unpaid* tax, so a filer who paid by 2026-04-15 and files late faces materially less exposure than
one who did not. **This is not verified in-repo and is not a tax-advice statement** — it is the single
input that decides between options A and B, and the owner should confirm it against their own facts
before ★ D-A is answered.

### 6.4 The realistic target, stated as a commitment

**First filed return: TY2025, filed with btctax, after 2026-10-15.** **First *on-time* filed return:
TY2026, in the 2027 season**, made cheap by Phase 5.

---

## 7. WHAT SHOULD NOT BE BUILT

### 7.1 E-file / MeF transmission — **DO NOT BUILD**, and the recon closes the argument

Not because it is hard, but because the cost is **recurring and structural**:

- **Transmission is gated by design, not by technology.** MeF schemas are public; submitting requires an
  EFIN/ETIN held by an Authorized Provider, and *"there is no 'anyone with valid XML and a network
  connection may submit' door"* (recon-efile §6.1).
- **No personal-use tier exists.** The "Large Taxpayer" self-filing category is for $10M+ asset
  entities; a person filing only their own 1040 goes through the identical Provider application,
  suitability check (including Livescan fingerprinting) and ATS testing a commercial preparer does
  (recon-efile §6.4).
- **The Online Provider security profile presupposes a website** — EV-SSL, CAPTCHA, a third-party
  privacy-seal audit (Pub 1345). *"How a CLI tool satisfies 'website' requirements is not addressed
  anywhere in the IRS guidance"* (recon-efile §4.4). That is an unresolved design question, not a
  checkbox.
- **ATS re-testing recurs every tax year**, alongside continuous suitability monitoring and sanctioning
  exposure (recon-efile §4.5).

★ **And generating MeF XML *without* transmitting is worse than doing nothing**: *"this XML has nowhere
to go — it isn't a file format any existing consumer product will ingest from a third party"*
(recon-efile §4c). It would be a large, per-year-versioned artifact with zero readers, and this repo has
a name for that (`a-figure-with-no-reader`).

★★ **The decisive fact is that paper loses nothing.** MeF and paper have always accepted Schedule D and
Form 8949 including digital-asset dispositions (recon-efile §6.5); Direct File's exclusion of capital
gains was a product choice, not a legal barrier — and Direct File is shut for FS2026 regardless. **The
paper channel serves P0 and P1 completely.**

### 7.2 State returns — **DO NOT BUILD**

Already disclosed four times over (`LIMITATIONS.md:3`, `:302`, `SPEC_full_return.md:79`, `:578`). It is
a different product: 40+ jurisdictions, per-state conformity to federal capital-gains treatment,
community-property allocation. ★ Honest caveat worth carrying into the docs rather than the code: the
practical burden on a crypto filer is **larger than "federal only" reads**, because crypto-heavy income
concentrates state liability the tool cannot see (recon-last-mile §5).

### 7.3 A second §24(b) / §32 implementation in `btctax-forms` — **DO NOT BUILD**

The forms lane already refused this and was right (`FOLLOWUPS.md:5355-5359`): a §24(b) predicate inside
the emitter would be a second divergent implementation of the rule and *"a worse answered-ness violation
than the one N3 describes — the emitter deciding for the filer."*

### 7.4 The last-mile items that are correctly absent — **DO NOT BUILD**

**Form 2210 / line 38** — blank is the i1040 instructions' own default and the IRS bills the penalty
(`forms/2024/f1040.map.toml:253`). **Direct deposit / Form 8888** — a paper check is the disclosed
consequence and bank details are PII the return does not need. **Line 36 (apply to next year)** —
no election is offered. **Third-party designee** — an authorization the filer may not want. **Phone /
email** — the email omission is a considered privacy choice.

### 7.5 Not "never", but **NOT NOW** — the P2-profile items

Retirement income 4a–6b, Schedule 8812 (CTC/ACTC), Schedule EIC, Form 8962, Schedule E/F, Form 709,
Form 8275-R, Form 1040-ES quarterly modelling. Each is refused or advised today, fail-closed, and none
is on P0's path. **EITC in particular stays deferred** (FR-16): it *"is not a plan item at this scope —
it is a project"*, it needs two maps that do not exist plus a refundable-credit path, and its oracle is
booby-trapped (`design/direction/ORACLE-TRAP-credit-takeup.md` — taxcalc's defaults report EITC = $0 for
a household owed $4,778.18, which would have looked like corroboration of btctax's own wrong zero).

### 7.6 One thing NOT to "fix"

The Schedule 1-A phase-out bands add hidden marginal-rate adders, so the optimizer's per-$1 what-ifs
will show $0 and then a cliff. **Document it; do not smooth it — the step function is the law**
(`IMPLEMENTATION_PLAN_schedule_1a.md` §3).

---

## 8. ★ OWNER DECISIONS — answerable in one pass

Each has a recommended default. Answering nothing means taking the defaults, which are consistent with
each other.

| ★ | decision | recommended default |
|---|---|---|
| **D-A** | **Which year is the first return filed with btctax, and is a TY2025 return already handled by other means?** This decides §6 entirely. | **TY2025, filed late** (option A). If you have already filed TY2025 elsewhere, say so — Phase 3's exit gate becomes a *comparison* against your filed return, which is strictly better evidence than an oracle. |
| **D-B** | **Is P0 (`FILING-TRIAL-2026-08-02.md:14-20`) still your actual profile?** Specifically: any 1099-R / SSA-1099 / pension? any rental or K-1? any **broker** 1099-B beyond crypto? has the IRS issued your **spouse** an IP PIN? | **Yes, P0 stands.** Any "yes" to the first two moves an item from §7.5 onto the critical path and must be known before Phase 1 starts. |
| **D-C** | **Does the retirement-income build (P2) proceed in parallel, or park after its spec goes green?** | **Finish the spec to 0C/0I; park the build until after the first filed return** — unless D-B says you have such income, in which case it joins Phase 1. |
| ~~**D-D**~~ | ~~FR-1 (line 19 → `Option<Usd>`) now, or with the TY2025 lane?~~ | ✅ **MOOT — built and merged 2026-09-04** (`5094bfc5` → `3d83e601`). No decision needed. |
| **D-E** | **Build the year-port machine (§5)?** | **Yes, split**: the printed-text diff gate in **Phase 2**; the copy/scaffold in **Phase 5**, before TY2026 season. |
| **D-F** | **Tag / publish / crates.io?** | **No bearing on filing — keep deferred.** But **revoke the stale v0.17.0 token** regardless (`CONTINUITY.md`, standing item). |
| **D-G** | **EITC/ACTC (FR-16)?** | **Stays deferred.** P0 cannot claim it — §32(i)'s investment-income limit is exceeded many times over by the capital gains — so it buys P0 nothing and costs a project. |
| **D-H** | **May Phase 4's mailing-address help be a link plus the two rules (state + payment), rather than a bundled table?** | **Yes.** The IRS corrected these addresses mid-2026 (recon-efile §5); a bundled table is a figure that rots between releases. |

---

## 9. RISKS CARRIED INTO THIS PLAN

- **The fold carries the defect** (`FOLLOWUPS.md:1605-1616`). Every phase here ends in a fold, and folds
  are authorship. Budget review for folds, not only for features.
- **A per-range review is not a branch review** (harness **B3**). Phases 1–3 will span weeks and many
  ranges; the last review before the gate deletion must be scoped to the whole branch and pointed at the
  **seams** — which for this plan means the Schedule 1-A ↔ Form 6251 ↔ 1040 line 14 composition.
- **A golden cannot validate its own regeneration.** Phase 2 rebuilds five maps and adds thirteen; every
  affected golden must be diffed by hand, not regenerated and glanced at.
- **Two disqualified oracles can align.** TY2025 Part IV inside the phase-out band ships **zero-oracle**
  for QSS (OTS 2025's Part IV is defective three ways; taxcalc has the wrong QSS threshold —
  `IMPLEMENTATION_PLAN_schedule_1a.md` T7). The census must count **witnesses per vector**, not print two
  "OK" lines.
- **`CONTINUITY.md` is an append-log with the newest entry on top** and three stacked headers; the
  historical sections below line 259 read like a live queue and are not (recon-open-work §0). Any future
  session resuming this plan reads *above* that line only.
- **`make check` is not CI**, and a green `make check` has twice meant a red CI — once for a missing job,
  once for a stale toolchain (`CONTINUITY.md`, "A TRAP THAT COST A WEEK OF RED CI").

---

## 10. CROSS-REFERENCES

- `design/ty2025/SPEC.md` §5.0 (ordering is a safety property), §5.2 (Schedule 1-A), §5.4 (form assets),
  §8a (the B1–B4 cut points).
- `design/ty2025/SPEC_schedule_1a.md`, `design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md`,
  `design/ty2025/reviews/`.
- `design/ty2025/SPEC_retirement_income.md` (landed 2026-09-04, P2); `design/ty2025/SPEC_schedule_a_ty2025.md`
  and `design/ty2025/SPEC_form6251_ty2025.md` (in flight, Phase 2's two structural forms).
- `crates/btctax-adapters/src/tax_tables.rs:788-822` — the four-clause unblock condition, which is the
  exit gate of Phases 1–3.
- `crates/btctax-cli/LIMITATIONS.md` — the shipped, `include_str!`'d statement of scope; **it changes in
  Phase 3**, and that edit is part of the phase, not a follow-up.
- `FOLLOWUPS.md` FR-1 (✅ closed 2026-09-04), FR-15, FR-16, FR-17, FR-19, FR-20, §G-6b (AMT Tier-2),
  §G-11 (the emitter cannot express "no testimony"), §G-13 (field provenance).
- `design/direction/FILING-TRIAL-2026-08-02.md` — P0, driven through the shipped binary.
- `design/HARNESS.md` — B1 (seen-red-once), B2 (payloads as paths), B3 (the last review is whole-branch).
