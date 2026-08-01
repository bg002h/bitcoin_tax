# FOLD REVIEW — what did the two review folds LEAK? (workflow)

**Date:** 2026-07-31 · **Branch:** `feat/no-pen-deferrals` · **Range:** `5ab1258..HEAD` (2 commits).

**Brief:** [`reviews/BRIEF-folds.md`](./BRIEF-folds.md). Deliberately narrow: the two commits under
review are themselves review FOLDS, and this branch had twice seen "the fix was right about its target
and something adjacent inherited the defect."

**Shape:** 3 lenses (leak · **artifact** · instrument), then 2 isolated skeptics per blocking finding.
6 agents. Both process fixes from the prior round were applied: **`isolation: worktree` on every
mutating agent**, and an **artifact lens that builds vaults, runs `export-irs-pdf`, and reads the ink** —
the blind spot the previous report named loudest.

**Result: None surviving Important, None killed.**

★★★ **THE LEAK QUESTION, ANSWERED: no third leak.** The lens traced every consumer of every changed
predicate, signature, stamp and guard and found nothing adjacent had inherited a defect. The one
survivor is **not** a leak — it is the SAME defect r3 aimed at, **half-fixed**: r3 correctly diagnosed
that the gate read the LEDGER instead of the RETURN, then re-keyed to a predicate that reads the return
for the *election* and **still reads the ledger for the amount**. Not a new organ; the same one,
half-treated. My own rationale three lines above the code describes the surviving case verbatim.

★★★ **THE ARTIFACT LENS PAID FOR ITSELF — ~14 packets exported and read with `pdftotext -layout`.**
The §63(f) box count and 1040 line 12 were confirmed to agree **on the printed page** in every MFS
shape (17700 / 14600 / 32300, with the $682 delta exactly 22% of $3,100), the 8283 Section-A/B guard
was confirmed by ink, and refusals were confirmed to leave the output directory **never created**. Its
coverage is partial and it says so.

★ **A defect in MY test, found by the re-key:** the r3 fixture set `claimed_deduction` with
`legs: vec![]` — a shape no real ledger produces. `year_donation_deduction` saw the gift; the RETURN saw
nothing. So the r3 tests passed against the ledger figure alone, which is precisely the defect. A test
whose fixture cannot reach the return cannot pin a rule about the return.

**Workflow output reproduced VERBATIM below.**

---

{
  "summary": "Focused review of the two review folds: what did the fixes leak?",
  "agentCount": 6,
  "logs": [
    "Refutation: 1 survived, 0 killed"
  ],
  "result": {
    "verdict": "VERDICT: fix-before-merge: 1 Important — re-key the §G-21 gate in `screen_absolute` to the quantity the packet actually files on (`ar.schedule_a…charitable_noncash_12`), not the ledger aggregate `year_donation_deduction`.

Everything else in this range holds. The two folds are otherwise clean, and the branch's headline artifacts (the MFS §63(f) box count vs 1040 line 12; the 8283 5a/5b/5c Section-B guard) were verified **on the printed page**, not inferred.

---

## 1. SURVIVING BLOCKING FINDING (1)

```
SEVERITY: Important                                    [0 of 2 skeptics refuted — both independently EXECUTED it and confirmed]
WHERE: crates/btctax-core/src/tax/return_1040.rs:1603-1631 (screen_absolute, the §G-21 block)
CLAIM: r3's I-3 re-key swapped one wrong predicate for another. `ar.deduction_is_itemized &&
       year_donation_deduction(state, year) > 0` is still not \"does this return CLAIM a §170
       noncash deduction\", so an itemizing filer whose §170(b) ceiling zeroes the deduction is
       hard-blocked, with no in-app exit, by a refusal asserting a Form 8283 the packet does not write.
FAILURE: TY2024, Single, no wages (AGI $0), Schedule A mortgage interest $20,000, one long-term BTC
       donation at FMV $4,000, donations_had_restrictions = Some(true). §170(b)'s 30% ceiling is
       30% × $0 = $0, so the return claims $0 of noncash gifts; the mortgage alone beats the $14,600
       standard, so deduction_is_itemized == true and the ledger figure is $4,000 > 0 →
       `export-irs-pdf` REFUSES (\"...the deduction it would compute is too large ... complete Form
       8283 for the restricted donation by hand — no forms were written\") and `report --tax-year`
       prints the same NOT COMPUTABLE block. CONTROL (identical inputs, Some(false)): exports cleanly;
       07_f1040sa.pdf line 12 = 0, line 17 = 20000, 1040 line 12 = 20000, and the packet is
       00_f1040.pdf + 07_f1040sa.pdf + manifest.txt — THERE IS NO 8283 IN IT. The restriction moves no
       figure and the year files no 8283, yet it is refused. Only exits: answer \"No\" (false testimony
       under §6065) or delete a real ledger event. The unanswered sibling arm ($50,000 gift) refuses
       quoting \"this year files a Form 8283 SECTION B\" — also false on the artifact. Not confined to
       AGI ≤ 0: a >$5,000 gift with AGI ≈ $1,600 gives a ceiling under $500 — line 12 nonzero, still
       below the attachment threshold, still no 8283.
EVIDENCE: return_1040.rs:1603-1604 `let donated = crate::forms::year_donation_deduction(state, year);
       if ar.deduction_is_itemized && donated > Usd::ZERO {`. The packet uses a DIFFERENT quantity —
       packet.rs:611-613 `let f8283 = sch_a.as_ref().filter(|a| a.line12 > FORM_8283_THRESHOLD)`,
       where line12 is ScheduleAParts::charitable_noncash_12, the §170(b)-LIMITED figure, and whose
       own comment says the 8283 files only on the printed noncash gifts. AUTHORITY, from the text
       layer, design/forms/extract/f8283--2025.txt:8,10 — \"Attach one or more Forms 8283 to your tax
       return if you claimed a total deduction of over $500 for all contributed property.\" The trigger
       is the CLAIMED deduction, not the ledger FMV. THE BRANCH'S OWN STANDARD, return_1040.rs:1583-86
       (r3's I-3 rationale, verbatim): \"TOO WIDE. `year_donation_deduction` reads the LEDGER, not the
       return ... a restriction changes no figure — yet they were refused, unescapably, by a message
       asserting 'this year files a Form 8283 SECTION B'. It does not.\" Word for word the executed case.
```

Refutation result: **2 of 2 skeptics tried and failed to kill it.** Skeptic 1 built the fixture and ran the production screen order (`screen_inputs` → `screen_compute_dependent` → `assemble_absolute` → `screen_absolute` → `assemble_printed_forms`), confirming `f8283 = None` in the very state that hard-blocks; all 83 module tests still green, so this is an unguarded gap, not a caught one. Skeptic 2 reproduced it plus the unanswered arm, traced `printed.rs:1254` to confirm line 12 is the ceiling-limited figure, and checked the prior round's COVERAGE section: its \"the refusal's this-year-files-a-Section-B is true whenever it fires\" claim was checking the message's $5,000 constant against the 8283's internal A/B split, **not** whether an 8283 is attached at all — a separate, AGI-sensitive gate. No prior round constructed an itemizing-anyway-but-ceiling-zeroes-it fixture.

Fix guidance (available on `ar`, no new plumbing): key both arms on `ar.schedule_a.as_ref().map_or(Usd::ZERO, |a| a.charitable_noncash_12)` — the identical quantity `packet.rs` filters on. Two implementation notes: (i) the *unanswered* arm's Section-B split must be evaluated on the same printed quantity the packet uses to pick the section (`forms.rs:398-404` chooses one section per year from the year aggregate), or gate and packet can still disagree on which section prints; (ii) if a practitioner reads the 8283's \"claimed a total deduction\" as the *pre*-limitation claim, then `packet.rs` is the side that is wrong — but the defect survives either reading, because today **the two predicates disagree**, and the refusal quotes an artifact that is not in the packet.

## 2. MINORS / NITS (deduplicated across three lenses)

- **Minor** — `return_1040.rs:1729` + `:1799-1802`: the vouch-for gate refuses the **whole** write-back, so `qbi_carryforward_in` and `reit_ptp_carryforward_in` are silently not persisted; the refusal names only the charitable carryover. EXECUTED: a $10,000 REIT/PTP carryforward was dropped; `--force` correctly does not open it; no partial write. Disclosure gap, fail-closed (both lenses, independently).
- **Minor** — `return_1040.rs:1731-1745`: the `Some(true)` arm is not scoped by `donated` (unlike its `is_none()` sibling two lines below), so a year with **no donation at all** but a rolled-in `charitable_carryover_in` is refused by a message stating as fact that btctax valued a donation at FMV. EXECUTED (leak lens called it a Nit on reachability; artifact lens built it — take Minor).
- **Minor** — `FOLLOWUPS.md:1449-1453`: the live body of the ✅CLOSED §G-21 entry still says the binding gate is keyed on `year_donation_deduction > QUALIFIED_APPRAISAL_THRESHOLD` — the exact $5,000 scope r3's I-2 removed as TOO NARROW — and is contradicted by the AMENDED paragraph 20 lines below (`:1466-1471`). A maintainer trusting it would reinstate the understatement.
- **Nit** — `crates/btctax-input-form/src/spec/registries.rs:303-305`: live-source site the ~20-site sweep missed; still names `screen_compute_dependent` as the gate's home **and** asserts the superseded Section-B predicate. Its sibling (`attribute.rs:24`) was swept.
- **Nit** — exit-code flip: moving §G-21 to `screen_absolute` means `report --tax-year` on a restricted-donation year now exits **0** (SPEC §3.5 keys the code on the delta outcome), and `optimize`/`what-if`/TUI/`export_snapshot` lose the refusal. Verified sound on the emission side: the crypto §170 figure is not in the derived `TaxProfile`, and every path that puts it on paper still runs the screen. On the record, not a defect.
- **Nit** — `FOLLOWUPS.md:1447-1448` broken sentence from the sweep (\"the same blindness that puts the non-crypto-noncash guard sits in a screen\"), and `:1459-1461`'s B1 kill list still names \"dropping the threshold scope\" as an observed kill for an arm that no longer exists in that form.
- **Nit** — `crates/btctax-cli/tests/export_irs_pdf.rs:951-952`: the doc comment claims deleting `screen_inputs` \"reds this by writing PDFs\"; re-executed, it reds via a *different* by-design backstop (`ReturnHeader::build`'s `HeaderError::Unanswered`, packet.rs:381-390) with `wrote_nothing() == true`. B1 is satisfied; the prose is imprecise. Screens 2 and 3 red literally as documented (real PDF bytes observed landing).

## 3. THE LEAK QUESTION — ANSWERED

**These two folds leaked nothing.** There is **no third leak** of the r3 shape (\"the fix was right about its target and something adjacent inherited the defect\"). The one surviving Important is *not* a leak — it is the **same** defect r3 aimed at, incompletely fixed: r3 correctly identified that the predicate read the ledger instead of the return, then re-keyed to a predicate that still reads the ledger for the *amount* and only the return for the *election*. Not a new organ; the same one, half-treated.

What the `leak` lens actually traced, change by change: every consumer of the OLD gate home — `screen_compute_dependent` runs inside `resolve_and_screen` (resolve.rs:199), reaching `report_tax_year`, `write_back_carryover`, `resolve_screened`/`resolve_all_screened`, optimize ×3, what-if ×2, admin SE-figure ×2, the prior-year M4 advisory, and the TUI — and confirmed none computes or emits the crypto §170 deduction (`derive_tax_profile` builds charitable from `ri.schedule_a.charitable` only; `crypto_charitable_gifts` is called solely inside `assemble_absolute`). All three consumers of `spouse_63f_status_permits` are advisory and in `advisories.rs`; every *grant* path still goes through `spouse_63f_boxes_count`, which requires the record. The narrowed advisory was walked over the full {MFS,MFJ} × {record present, absent} × {claiming, adverse, unanswered} × {aged, blind, neither} cross-product — no double-report, no unreported recoverable box. Every reader of `capital_loss_carryforward_in_provenance` re-enumerated; nothing *decides* on it. The 8283 Section-B guard is backed verbatim by the extracted text and no return that should print 5a/5b/5c can miss them (`section` is per-copy). Write-back atomicity confirmed: all `Err` returns precede every mutation, and `s.save()` is unreachable on refusal.

## 4. ARTIFACT EVIDENCE — what was actually seen on the page

~14 packets exported through `cmd::admin::export_irs_pdf` on real vaults at 5ebd3cc, read with `pdftotext -layout`:

- **§63(f) boxes vs 1040 line 12 — consistent in every MFS shape.** (a) spouse DOB 1955-03-02 + blind, all conditions affirmative: line 34 ink reads \"Spouse: ☑ Was born before January 2, 1960 ☑ Is blind\", **line 12 = 17700** (14,600 + 2×1,550), tax 11031. (b) `spouse_had_no_income = Some(false)`: both boxes **blank**, **line 12 = 14600**, tax 11713 — the $682 delta is exactly 22% of $3,100, and no §63(f) advisory prints. (c) condition unanswered: boxes blank, line 12 = 14600, and the rewritten `Mfs63fSpouseBoxesForgone` fires with the new text (no \"only on a JOINT return\", no \"yours to check by hand\"). (d) MFJ regression: boxes checked, **line 12 = 32300**. The predicate split moved no MFJ return.
- **8283 5a/5b/5c.** Section B year ($50,000, answered No): all three lines marked in the **No** column; Part I row A \"1.00000000 BTC\", appraised 50000, basis 10000, box 2k \"Digital assets\" checked. Section A year ($4,000, answered No): 5a/5b/5c **blank**, Section A row A populated. Section A **unanswered**: output byte-identical to answered-No — the guard, not the answer, is what suppresses the marks. No mixed-section return is constructible.
- **Refusals write zero bytes.** On the Section-B-unanswered itemizer and both ceiling-zero refusals, `ls` shows the output directory was **never created**.
- **r3's I-3 fix stands on the artifact.** The standard-deduction case (AGI $30,000, $50,000 LT gift, `Some(true)`) exports exactly `00_f1040.pdf` + `manifest.txt` — no Schedule A, no 8283. The declared restriction genuinely moves no figure.
- **§63(c)(6) × the MFS widening** — spouse boxes checked + \"Spouse itemizes on a separate return\" checked → **line 12 = 0**. Not filed: the identical shape is reachable for the taxpayer's own boxes on `main`, so it is pre-existing and out of range.

**NOT rendered (coverage is partial, say so plainly):** no ink was read on Schedule 2/3/8959/8960/8949/Schedule D in those packets; no 2017 or 2025 (crypto-slice) export; no pseudo/watermarked export; no TUI. Interactive `income answer`/`income form` was never exercised, so \"the refusal is escapable by answering\" remains **structural, not observed** — which matters, because for the surviving Important the only answer that escapes is a false one.

## 5. RESIDUAL RISK

**★ LOUD SHARED BLIND SPOT — the review environment, not the code.** The `artifact` and `instrument` lenses ran concurrently **in the same scratchpad worktree** (`.../scratchpad/tip`, detached at 5ebd3cc). The instrument lens found the artifact lens's untracked `zz_ink_lens.rs` there, deleted it, and watched it **reappear**; the artifact lens saw its harness \"silently deleted once mid-session (recreated)\". Neither held a lock. Both mitigated by diffing the four files under test against 5ebd3cc before and after (empty both times), and both ended with `git rev-parse HEAD == 5ebd3cc` and a clean tree — but a precisely-timed concurrent write during a 3-15s compile would have been invisible to both. Separately, the instrument lens's **assigned** worktree was stale (973a9e0, ~150 commits behind, missing both reviewed commits) — it substituted the shared tree rather than `git checkout` (correctly, per the repo's own rule). **Give each lens its own worktree at the actual tip next round; this is harness rule B2 territory and it degraded two of three lenses.**

Other residuals, per lens:

- **leak executed nothing** — every conclusion is control flow and greps. Its emission-path enumeration (\"every surface that puts the §170 figure on paper runs `screen_absolute`\") is a five-name grep, not a proof; a path reaching `btctax_forms::fill_*` under a sixth name is invisible to it. Its monotonicity argument for the `!is_empty()` scoping (a Reg §1.170A-7-reduced gift cannot carry over where full FMV did not) is its own reasoning — it did not read `apply_170b` line by line. If that function is non-monotone under some class/ceiling interaction, the vouch-for gate is under-inclusive and a laundering path remains open.
- **artifact** built from a scratch worktree of 5ebd3cc, not the shared checkout — its observations are about the commit, not about what is on disk. It did not open i8283's own text on the attachment trigger, so the pre- vs post-limitation reading of \"claimed a total deduction\" is not adjudicated (see fix guidance above; the finding survives either way).
- **instrument** re-executed 11 individual mutations (4 write-back conjuncts + `!force`, 3 export screens, 3 advisory conjuncts) and each reds its own named assertion — no untested clause. `make check`: **2546 passed, 0 failed, 12 skipped**, clippy clean, exit 0. Two earlier red runs were its own rig (missing `xtask` binary, then a `target/` symlink breaking `git check-ignore`), not the code.
- **All three took settled facts 1-5 on trust** — the five gates, the golden md5, the three folded findings. None re-ran the oracle sweep. That is the brief's instruction, not a lapse, but it is the shared floor under every conclusion here.",
    "survivors": 1,
    "killed": 0
  },
  "workflowProgress": [
    {
      "type": "workflow_phase",
      "index": 1,
      "title": "Find"
    },
    {
      "type": "workflow_phase",
      "index": 2,
      "title": "Refute"
    },
    {
      "type": "workflow_phase",
      "index": 3,
      "title": "Verdict"
    },
    {
      "type": "workflow_agent",
      "index": 1,
      "label": "find:leak",
      "phaseIndex": 1,
      "phaseTitle": "Find",
      "agentId": "a53a8946ec7a0a765",
      "model": "claude-opus-5[1m]",
      "state": "done",
      "startedAt": 1785540581124,
      "queuedAt": 1785540581115,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "clean — 0 Critical / 0 Important. I traced every one of the…",
      "promptPreview": "You are reviewing the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

FIRST read the brief: /scratch/code/bitcoin_tax/reviews/BRIEF-folds.md
It defines the one question (\"what did these two fixes LEAK?\"), the range (`5ab1258..HEAD`, exactly
two commits), what is in that range, six specific leak hypotheses, six settled facts you must NOT
re-derive, and the output format.
…",
      "lastProgressAt": 1785541272455,
      "tokens": 208800,
      "toolCalls": 61,
      "durationMs": 691330,
      "resultPreview": "{\"verdict\":\"clean — 0 Critical / 0 Important. I traced every one of the seven changes to its other readers and found no third instance of \\\"the fix was right about its target and something adjacent inherited the defect.\\\" Two Minors (one behavioral-scope, one stale primary-doc claim) and three Nits are recorded below; none blocks the merge.\",\"findings\":[{\"severity\":\"Minor\",\"where\":\"crates/btctax-c…"
    },
    {
      "type": "workflow_agent",
      "index": 2,
      "label": "find:artifact",
      "phaseIndex": 1,
      "phaseTitle": "Find",
      "agentId": "a0c17eb2065439452",
      "isolation": "worktree",
      "model": "claude-opus-5[1m]",
      "state": "done",
      "startedAt": 1785540582622,
      "queuedAt": 1785540581115,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "fix-before-merge: 1 (1 Important, 2 Minor). I built four+ r…",
      "promptPreview": "You are reviewing the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

FIRST read the brief: /scratch/code/bitcoin_tax/reviews/BRIEF-folds.md
It defines the one question (\"what did these two fixes LEAK?\"), the range (`5ab1258..HEAD`, exactly
two commits), what is in that range, six specific leak hypotheses, six settled facts you must NOT
re-derive, and the output format.
…",
      "lastProgressAt": 1785541708357,
      "tokens": 265338,
      "toolCalls": 100,
      "durationMs": 1125735,
      "resultPreview": "{\"verdict\":\"fix-before-merge: 1 (1 Important, 2 Minor). I built four+ real vaults, drove `cmd::admin::export_irs_pdf` / `report_tax_year` / `write_back_carryover` end-to-end at `5ebd3cc`, and read the emitted PDFs with `pdftotext -layout`. The two headline artifacts the lens was sent to check are CORRECT on the page: the §63(f) box count and 1040 line 12 agree in every MFS shape I built, and the 8…"
    },
    {
      "type": "workflow_agent",
      "index": 3,
      "label": "find:instrument",
      "phaseIndex": 1,
      "phaseTitle": "Find",
      "agentId": "a952228a88466f550",
      "isolation": "worktree",
      "model": "claude-sonnet-5",
      "state": "done",
      "startedAt": 1785540582730,
      "queuedAt": 1785540581115,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "VERDICT: clean — all mutation claims in `5ab1258..HEAD` (i.…",
      "promptPreview": "You are reviewing the btctax repo at /scratch/code/bitcoin_tax, branch `feat/no-pen-deferrals`.

FIRST read the brief: /scratch/code/bitcoin_tax/reviews/BRIEF-folds.md
It defines the one question (\"what did these two fixes LEAK?\"), the range (`5ab1258..HEAD`, exactly
two commits), what is in that range, six specific leak hypotheses, six settled facts you must NOT
re-derive, and the output format.
…",
      "lastProgressAt": 1785541806668,
      "tokens": 245830,
      "toolCalls": 125,
      "durationMs": 1223938,
      "resultPreview": "{\"verdict\":\"VERDICT: clean — all mutation claims in `5ab1258..HEAD` (i.e. `d6ff290..5ebd3cc`, the r3 fold + the pre-merge fold) that fall in my assigned lens re-executed and confirmed to discriminate exactly as documented. No Critical or Important finding. One Nit (prose precision, not a functional gap).\
\
★ ENVIRONMENT NOTE (read before the findings): my assigned worktree (/scratch/code/bitcoin_…"
    },
    {
      "type": "workflow_agent",
      "index": 4,
      "label": "refute:artifact:0:0",
      "phaseIndex": 2,
      "phaseTitle": "Refute",
      "agentId": "a4124f2ef86498c15",
      "isolation": "worktree",
      "model": "claude-sonnet-5",
      "state": "done",
      "startedAt": 1785541708564,
      "queuedAt": 1785541708437,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "Important",
      "promptPreview": "You are an adversarial verifier on the btctax repo, branch `feat/no-pen-deferrals`.
You are in an ISOLATED git worktree — build, run and mutate freely.

A reviewer filed this against `5ab1258..HEAD` (two commits of already-reviewed, mutation-verified
review folds). **Your job is to REFUTE it.** Assume it is wrong until the code forces otherwise.
This range has been reviewed twice already, so the p…",
      "lastProgressAt": 1785542146648,
      "tokens": 132623,
      "toolCalls": 74,
      "durationMs": 438084,
      "resultPreview": "{\"refuted\":false,\"corrected_severity\":\"Important\",\"reasoning\":\"Tried hard to refute via reachability and could not. Checked out feat/no-pen-deferrals @ 5ebd3cc and confirmed the cited code verbatim: `screen_absolute` (return_1040.rs:1603-1604) gates on `ar.deduction_is_itemized && crate::forms::year_donation_deduction(state, year) > Usd::ZERO`, where `year_donation_deduction` sums the ledger's raw…"
    },
    {
      "type": "workflow_agent",
      "index": 5,
      "label": "refute:artifact:0:1",
      "phaseIndex": 2,
      "phaseTitle": "Refute",
      "agentId": "adfe87d0da0eb697d",
      "isolation": "worktree",
      "model": "claude-sonnet-5",
      "state": "done",
      "startedAt": 1785541708686,
      "queuedAt": 1785541708437,
      "attempt": 1,
      "lastToolName": "StructuredOutput",
      "lastToolSummary": "Important",
      "promptPreview": "You are an adversarial verifier on the btctax repo, branch `feat/no-pen-deferrals`.
You are in an ISOLATED git worktree — build, run and mutate freely.

A reviewer filed this against `5ab1258..HEAD` (two commits of already-reviewed, mutation-verified
review folds). **Your job is to REFUTE it.** Assume it is wrong until the code forces otherwise.
This range has been reviewed twice already, so the p…",
      "lastProgressAt": 1785542155493,
      "tokens": 153692,
      "toolCalls": 48,
      "durationMs": 446807,
      "resultPreview": "{\"refuted\":false,\"corrected_severity\":\"Important\",\"reasoning\":\"Executed the exact scenario at HEAD (5ebd3cc): Single, AGI $0 (no wages), Schedule A mortgage interest $20,000, one LT crypto donation FMV $4,000, donations_had_restrictions=Some(true). Confirmed via a scratch #[test] calling assemble_absolute + screen_absolute directly (reverted after, worktree clean): ar.deduction_is_itemized=true, a…"
    },
    {
      "type": "workflow_agent",
      "index": 6,
      "label": "verdict",
      "phaseIndex": 3,
      "phaseTitle": "Verdict",
      "agentId": "a36061ac66a6cfb8f",
      "model": "claude-opus-5[1m]",
      "state": "done",
      "startedAt": 1785542155503,
      "queuedAt": 1785542155502,
      "attempt": 1,
      "lastToolName": "Bash",
      "lastToolSummary": "sed -n '605,620p' crates/btctax-core/src/tax/packet.rs && e…",
      "promptPreview": "Write the FINAL verdict for the btctax repo at /scratch/code/bitcoin_tax. Read the brief first: /scratch/code/bitcoin_tax/reviews/BRIEF-folds.md

Three lenses reviewed `5ab1258..HEAD` — two commits, both of which are themselves review FOLDS. The
question was \"what did these fixes LEAK?\". Every blocking finding then went to TWO independent
skeptics in isolated worktrees, instructed to REFUTE it. Ki…",
      "lastProgressAt": 1785542264296,
      "tokens": 79445,
      "toolCalls": 4,
      "durationMs": 108792,
      "resultPreview": "VERDICT: fix-before-merge: 1 Important — re-key the §G-21 gate in `screen_absolute` to the quantity the packet actually files on (`ar.schedule_a…charitable_noncash_12`), not the ledger aggregate `year_donation_deduction`.

Everything else in this range holds. The two folds are otherwise clean, and the branch's headline artifacts (the MFS §63(f) box count vs 1040 line 12; the 8283 5a/5b/5c Section-…"
    }
  ],
  "totalTokens": 1085728,
  "totalToolCalls": 412
}