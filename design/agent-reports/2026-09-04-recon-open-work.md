# Recon — open work standing between btctax and a filer submitting a complete return

**Scope:** `main` @ `945d1ac2` (current HEAD is `chore/archive-reconciliation`, which is `main` plus
archive-hygiene commits only — verified via `git log --oneline main..chore/archive-reconciliation`,
all 21 commits touch `CONTINUITY.md`/`FOLLOWUPS.md`/`design/`/`scripts/pre-commit`, none touch
`crates/`). Archive-reconciliation's own residue (FR-23..26) is noted but not counted as filer-facing.

**Method:** read `FOLLOWUPS.md` (5,661 lines), `CONTINUITY.md` (1,092 lines — note its structure below),
`design/HARNESS.md`, `design/amt-form6251/CONTINUITY_TY2025.md`, `design/direction/FILING-READINESS-PLAN.md`
and its 7 lens docs, `design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md`, and cross-checked every claim
against current source (`crates/**/*.rs`) with grep/git-log, not against the docs' own prose.

---

## 0. A structural fact about `CONTINUITY.md` worth flagging first

The file is an **append-log with the newest entry on top**, not a single current-state document. It
contains at least three stacked `# CONTINUITY — bitcoin_tax (TaxApp)` headers (lines 1, 164, and more
below). Line 259 states explicitly: *"Everything below is the historical record of earlier tracks
… All of it shipped; it is retained for provenance, not as a work queue."* A reader who stops at the
first `#` heading, or who skims for section titles like "RANKED BACKLOG" (line 418) or "WHAT IS OPEN
NOW" (line 478) without noticing they sit *below* that disclaimer, will treat 2026-07-30 history as a
live queue. This report reads only the section above line 259 (dated 2026-09-04) as current, cross-checked
against `FOLLOWUPS.md` and source.

---

## 1. De-duplicated table of open items

Legend: **BLOCKS** = a real filer in this population cannot get a signed, correct return out of btctax
today (either a refusal, or a silently wrong number). **DEGRADES** = files, but a real population gets
a suboptimal or narrower-than-necessary outcome. **COSMETIC** = doc/UX/wording only. **UNRELATED** =
dev-tooling/process debt with no path to a filed figure.

| ID | Description | Owning phase | Assessment |
|---|---|---|---|
| **TY2025-GATE** | The *only* year `full_return_for()` returns `Some` for is 2024 (`crates/btctax-adapters/src/tax_tables.rs:816,908` — both `ty2025_full_return_must_stay_fail_closed_until_complete` / `…2026…` assert `None`, by design). TY2024's deadline was 16 months ago; the live season (TY2025, extensions to 2026-10-15) is fail-closed. | none named ("whichever phase lands TY2025 full-return support" — cited by FR-8, FR-17 as the phase that also fixes them) | **BLOCKS** — the whole product, for everyone, right now |
| **SCH1A** | Schedule 1-A (OBBBA: no-tax-on-tips, no-tax-on-overtime, car-loan interest, senior deduction) is a hard TY2025 prerequisite (`tax_tables.rs:791-810` doc comment, condition 4). Plan has 7 tasks (`design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md` T1-T7); **only T1 is built** (`94fa025e`, the per-year constants table `Schedule1aParams`, `crates/btctax-core/src/tax/tables.rs:1017`). T2 (the 48-line struct), T3/T3a (input surface, 3 lines with no input path), T4 (compute), T5 (wiring), T6 (tests), T7 (two-oracle census) are all unbuilt — no `tips_deduction`/`overtime_deduction`/`car_loan_interest`/`senior_deduction` symbol exists anywhere in `crates/`. | B3 (per `design/amt-form6251/CONTINUITY_TY2025.md` line 30) | **BLOCKS** TY2025-GATE from closing |
| **N2 / FR-16** | EITC/ACTC. Owner decision 11 put it IN SCOPE (2026-08-21); it "is not a plan item at this scope — it is a project" per its own filing (`FOLLOWUPS.md:5572`). Needs Schedule 8812 + Schedule EIC (neither mapped), a refundable-credit path, new inputs (earned income, §32(i) limit, qualifying-child residency), a two-oracle witness with taxcalc's stochastic take-up disabled. Deliberately **NOT STARTED**. Quantified: $8,781 forfeited on a MFJ/$40k/2-child household — 22% of that household's income (`design/direction/FILING-READINESS-PLAN.md` line ~62). | none — "its own cycle starting at brainstorm" | **BLOCKS** the modal low/middle-income family with children |
| **N3 / FR-1** | Form 1040 line 19 prints an unconditional `0` for families the CTC belongs to — signed false testimony, not a refusal. `Form1040Lines::line19` needs `Option<Usd>`; `ctc_provably_zero` is private. NOT BUILT (`FOLLOWUPS.md:5351`). | future btctax-core lane | **BLOCKS** correctly-filed CTC (worse than a refusal: it *files*, wrongly) |
| **AMT-E4/E5/E6** | Tier-2 (attach Form 6251) gate `FOLLOWUPS.md:522`. E1-E3, G-6d all **DONE**. Still `[open → Tier 2]`: **E4** — read the filled `f6251.pdf` back field-by-field, no oracle/vector proxy for AcroForm transposition; **E5** — lift the `must_attach` refusal for itemizing-AMT households once E4 exists (D-2), keep taxcalc as a KnownDefect for standard-deduction filers; **E6** — Form 6251 lines 2c-2t become real provenance-carrying fields (currently silently zero). Confirmed still refusing in current source: `AmtNonQualifiedDwelling`, `AmtCarryoverDiverges`, `AmtDepreciationDiverges`, `OtherIncomeOutOfScope` (ISO/§1202/etc.) at `crates/btctax-core/src/tax/return_refuse.rs:940-1000`. No commits touch `form6251.rs`/`amt.rs` after `3d39dbe8` (2026-08-22, P9 KAT 4). | Tier 2 | **BLOCKS** ISO exercisers, AMT-depreciation-divergent Schedule-C filers, non-qualified-dwelling mortgage-interest AMT filers, and (per G-6's MFS witness gap below) MFS filers above the §55(d)(3) kicker |
| **G-6c** | Report the missing §55(d)(3) MFS AMTI line-4 add-back upstream to Tax-Calculator (two independent witnesses already exist: OTS's correct add-back + i6251 p.9 worked example). Not filed yet. | Tier 2 | **UNRELATED** to btctax's own filer (upstream bug report), but blocks the two-oracle bar for that MFS band |
| **FR-17** | A `Computed` capital-loss-carryover stamp has no retraction path — a stale figure survives a re-roll, `--force`, and a zeroing `income import` (all three reproduced, `FOLLOWUPS.md:5417`). Mitigated (write-back no longer claims a write it didn't make; forgery via TOML key closed) but the underlying gap stands. | "whichever phase lands TY2025 full-return support" | **DEGRADES** — narrow (only fires if a filer corrects a prior year's loss after rolling it forward) |
| **FR-7** | N1's crypto-**slice** path (the `report` planning surface, not the filing one) has no guard on a TI-at-the-floor loss year; plan recommends *warn*, unbuilt. | none — ready now | **DEGRADES** (slice mode is explicitly not "filing" per FR-7's own text) |
| **FR-14** | P7 added an always-live declaration (Schedule D line 20 routing) — every filer now answers one more yes/no. Judged correct/forced by two independent reviews, but flagged as the largest friction increase in the branch. | n/a — shipped, this is a residual judgment note | **COSMETIC** |
| **FR-19** | `--force` on `--write-carryover` promises an overwrite the `grounded` gate then silently refuses to perform — one command prints two contradictory sentences (reproduced verbatim, `FOLLOWUPS.md:5492`). One-conjunct fix identified and named at `return_1040.rs:2919`. | none — ready now, trivial | **DEGRADES** (confusing UX, could send a filer to "fix" a correct number) |
| **FR-20** | *"btctax CANNOT FILE THIS YEAR"* (canceled-debt / Form 982 refusal) is broader in wording than what the code enforces (`questions.rs:200-203` gates only on a capital-loss carryforward being present) — the real gap is that v1 collects **no** canceled-debt income at all. | none — needs an owner decision, not a phase | **DEGRADES** (wording overclaim; the underlying scope gap is real and pre-existing) |
| **FR-4** | Charitable carryover has no TUI reader — the figure reaches zero humans on that surface (CLI `report`/export has it). | none | **DEGRADES** (CLI-only filer unaffected; TUI-only filer never sees it) |
| **FR-12 / FR-13** | Form 8960 (NIIT) line 9d prints `0` for an unasked line (same affirmative-zero class P8 fixed one line up); line 9a is newly derivable now that Schedule A line 9 exists. | future btctax-core lane | **DEGRADES** — narrow, high-income NIIT population |
| **FR-2** | TY2017 Form 8283 still writes the wrong column for that revision (pre-pass-through-entity regime; authority not held in-repo). | whoever holds the authority doc | **DEGRADES** — a 2017 amended-return population, essentially nobody today |
| **FR-5** | No "you may not need to file at all" note (i1040 Chart A untranscribed). | future | **COSMETIC/DEGRADES** — never wrong, just silent on a convenience |
| **G-13 residue** | Field-provenance census: gaps down to **6 fields / 3 items**, all Form 8283 lines 5a/5b/5c (donee-restriction disclosures, understatement direction) — `FOLLOWUPS.md:1109`. Plus one open owner question (Schedule C I/J: does a *disclosed* deferral count as a gap?). | not B3 | **DEGRADES** — narrow (non-cash-gift-with-restrictions population) |
| **admin.rs:1200** | Full-return export path hardcodes `broker_reported_rows: 0`, so a full return with exchange disposals never gets the [I5] broker-reporting advisory (the crypto-slice path does get it). Confirmed still present in current source. | future export-parity pass | **DEGRADES** |
| **G-12** | btctax emits Form 8275 but not Form 8275-R — cannot disclose a position contrary to a *regulation* (only contrary to a statute/rev-rul is supported). | unstated | **DEGRADES** — narrow (defensive/aggressive filers only) |
| **Form 709** | No gift-tax return (§2502 rate schedule, §2513 splitting, DSUE/portability) exists at all — confirmed, `find crates -iname "*form709*"` returns nothing. | unstated (GENUINELY-OPEN INDEX, 2026-07-20) | **DEGRADES** — narrow (large-gift population) |
| **Sch B FBAR sub-q / Sch SE line A** | Two G-13-found gaps not yet reclassified as `gap` fixes: Schedule B 7a's FinCEN-114 sub-question (penalty-direction, unasked) and Schedule SE line A (Form 4361 minister declaration, `unmodeled` — clergy explicitly out of scope). | not B3 | **DEGRADES** — narrow populations |
| **FR-21 / FR-22** | Two checkers (`cite_check::plain_quotations`, K19's identifier grep) proven blind by mutation; both filed with structural fixes named, both explicitly non-gating. | none — ownerless | **UNRELATED** (dev tooling; correctly excluded from filing-blocking status by the repo's own severity rule) |
| **FR-9 / FR-11 / FR-15** | GIT_DIR sweep outside xtask; a golden standing in for a direct test (Form 8283 hand-mark); a hand-maintained doc snapshot with no generator/test. | ownerless / P8 | **UNRELATED** |
| **income-scrub Fable pass** | `FOLLOWUPS.md:7` — a scheduled-but-not-yet-run deep adversarial pass on `income scrub`, deliberately post-publish. Not a filing gap; `income scrub` is a debugging-handoff tool, not part of the filing path. | owner-scheduled | **UNRELATED** to "complete return" |
| **FR-23/24/25/26** | Archive/authority-manifest hygiene from `chore/archive-reconciliation` itself (committed 1099-DA binaries that should be archived as notes; an orphaned manifest entry; regen-destroys-manifest risk now fixed; a fetcher script named in a refusal message but not built). | next harness change / next 1099-DA touch | **UNRELATED** — explicitly out of this recon's scope per the task framing |

---

## 2. Stale-status flags (item 2 of the brief)

**Reads CLOSED but is actually the biggest open item — inverted staleness.** The `FOLLOWUPS.md`
"★ GENUINELY-OPEN INDEX" (line 106, dated 2026-07-20, section A) still says:

> *"AMT (Form 6251) compute — only a fail-closed screen exists (`tax/amt.rs`; refuses if AMT might
> apply). Computing an actual AMT liability is out of scope."*

This is **stale in the direction of understating progress**: since `942120e2` (2026-08-03), Form 6251
**is** computed for every return and **is filed** whenever `must_attach()` holds (verified end-to-end,
6251 L11 → Sch 2 L2/L3 → 1040 L17). The correct current framing is the Tier-2 gate table in
`design/amt-form6251/CONTINUITY_TY2025.md` §2 / `FOLLOWUPS.md` G-6b, not this July line. It is not
misleading in a way that would cause harm (it undersells rather than oversells), but a reader who
trusts this index literally would think AMT is further from done than it is.

**Reads OPEN but is CLOSED.** `RefuseReason::AmtScreenTriggered` (`return_refuse.rs:310`) is declared,
matched in `btctax-input-form`, and — per `FOLLOWUPS.md` — dead code since the emitter replaced it
2026-08-03; left in place on purpose pending the Tier-2 rename. Not a functional gap, just a variant
that reads live and isn't.

**Reads OPEN but is CLOSED (2).** `FOLLOWUPS.md`'s "GENUINELY-OPEN INDEX" section A also lists
"Form 8949 lot-count overflow" nowhere by that name, but the Aug-1 `FILING-READINESS-PLAN.md` P2/P2a/P2b
finding ("more than 14 disposal legs → exit 2, zero bytes") **is now closed**: `66907c6a fix(8949):
paginate the full-return Form 8949 instead of refusing (P2b)` and `36707a23 fix(8949): P2a's preflight
became a FALSE refusal the moment P2b landed` are both on `main`, and `full_return_forms.rs:3662`
exercises a 15-leg household split correctly across two form copies. Do not re-flag this as open in a
future pass without re-checking.

**MEMORY.md (auto-memory, not the repo) is stale, worth correcting for future sessions.** It says *"a
Fable consult queued in §5a"* and *"`design/amt-form6251/CONTINUITY_E4.md` is the resume point."* No
such file exists — `design/amt-form6251/` contains `CONTINUITY_TY2025.md` (which explicitly says in
its own title *"supersedes the E2 and E4 editions"*), not a `CONTINUITY_E4.md`. This is a stale
cross-session memory pointer, not a repo defect, but it will send a future session to a nonexistent
file.

**`CONTINUITY.md`'s stacked-history structure (§0 above)** is itself the largest stale-*reading*-risk
in the repo, even though it correctly discloses its own staleness at line 259 — the risk is a reader
skipping that one line.

---

## 3. The AMT Tier-2 track — direct answer

**Still blocking**, for a defined, narrower-than-2026-07-29 population. Tier 1 (compute + attach for
the common case) **shipped** on `main` (`942120e2`, `31c7b06c`, `8005ddfc`, `3d39dbe8` — all merged via
`feat/filing-readiness`). What remains, exactly, per the Tier-2 entry-criteria gate
(`FOLLOWUPS.md:522-560`, cross-checked against source):

- **E4 — NOT DONE.** No PDF-field readback verification for the filled `f6251.pdf` exists; a correct
  computation could still file through a transposed AcroForm field, and nothing catches that today.
- **E5 — NOT DONE.** The `must_attach()` refusal (D-2, i.e. `AmtScreenTriggered`'s successor gating
  path) has not been lifted for itemizing-AMT households, even though E1-E3 already give that slice a
  clean two-oracle witness (22 of 30 vectors, every filing status represented).
- **E6 — NOT DONE.** Form 6251 lines 2c-2t are still silently zero rather than real provenance-carrying
  fields; the three specific existence-question refusals that currently substitute for them
  (`AmtNonQualifiedDwelling`, `AmtCarryoverDiverges`, `AmtDepreciationDiverges`, plus the broader
  `OtherIncomeOutOfScope` catch-all naming ISO/§1202/§4952/NOL/Form 8801/accelerated depreciation) are
  all still live in `return_refuse.rs:940-1000` — confirmed by direct read, not by doc claim.
- **G-6c — NOT FILED.** The upstream Tax-Calculator report for the MFS §55(d)(3) line-4 AMTI add-back
  omission (two independent witnesses already in hand: OTS's correct implementation + the IRS worked
  example) has not been sent.
- No commit touches `form6251.rs`, `amt.rs`, `verify_f6251.py`, or `gen_e2_vectors.py` after
  `3d39dbe8` (2026-08-22) — the track has been dormant for roughly two weeks as of the archive branch's
  tip, while `feat/filing-readiness`'s other work (carryover, Schedule A P1/P7/P8, N1-N4) proceeded
  around it.

**Practical read:** a filer with a "plain" AMT trigger (large LTCG pushing past the exemption phase-out,
standard *or* itemized deduction, no ISO/§1202/divergent-AMT-depreciation/non-qualified-dwelling
items) **can already file with an attached, computed, two-oracle-witnessed Form 6251 today** — that
refusal is gone. A filer with any of the still-modeled-as-existence-questions preference items is
correctly refused rather than served wrong, which is the intended fail-closed posture, not a defect —
but it is still a **BLOCKS** outcome for that population until E6 gives those lines real fields.

---

## 4. DEFERRED/NOT-STARTED items a common filer would actually hit

- **EITC/ACTC (N2/FR-16).** The single largest dollar-value gap in the repo's own accounting: $8,781
  forfeited by a $40k-income, 2-child MFJ household (22% of income), and explicitly the modal
  low/middle-income filer with kids. Owner put it in scope 2026-08-21; **deliberately not started**,
  scoped as its own project (needs Schedule 8812 + Schedule EIC, new inputs, a refundable-credit path).
  De-risked by a documented oracle trap (`design/direction/ORACLE-TRAP-credit-takeup.md` — taxcalc's
  default EITC take-up model silently zeros a household that is owed $4,778.18).
- **CTC's fabricated `$0` on 1040 line 19 (N3/FR-1).** Distinct from N2: even without building the
  refundable-EITC machinery, any family currently gets a *signed false statement* on line 19 rather
  than a correct nonzero figure or an honest blank, because `line19` is a bare `Usd` not `Option<Usd>`.
- **Dependents generally are supported** (dependent count/qualifying-child data is already collected —
  it feeds the KiddieTax refusal and would feed Schedule 8812 once built) — the gap is specifically the
  *credit computation*, not dependent intake.
- **MFS is NOT generally deferred** — worth correcting the task's framing here: MFS ordinary brackets
  shipped witnessed (G-25, closed 2026-08-02), and MFS AMT is Tier-1-computed. The real, narrower MFS
  gap is the one under §3 above: no independent witness exists for an MFS return above the §55(d)(3)
  zero-exemption/kicker threshold ($875,950 combined condition in TY2024), because both engines are
  independently defective in exactly that band. One vector (V22, below the kicker) is witnessed; three
  (V23-V25, above it) are not.
- **Schedule E / rental / K-1 income:** still explicitly out of scope, unchanged, confirmed no
  `schedule_e` source exists — a common filer with even one rental room cannot file at all (refuses via
  `OtherIncomeOutOfScope`).
- **Investment-interest deduction (Form 4952):** boolean-only, refuses; not built.

---

## 5. Top 5, ordered

1. **TY2025-GATE.** Nothing else in this list matters to a real filer until the fail-closed year gate
   at `tax_tables.rs:816` lifts — the current filing season is entirely unservable, confirmed in
   source, and this was the owner-commissioned strategic review's own headline finding
   (`ca45a4ee`: *"we are polishing a year nobody can file"*).
2. **SCH1A (Schedule 1-A, T2-T7).** It is the specific, named blocking prerequisite for #1 — TY2025
   cannot honestly compute without it (tips/overtime/car-loan/senior deductions are OBBBA law for
   2025), and 6 of its 7 tasks remain unbuilt.
3. **N2/FR-16 (EITC/ACTC).** The largest *dollar* gap in the repo's own measurements, hitting the
   population — low/middle-income families with kids — that most needs the software to work, and it is
   the one item explicitly scoped as its own project rather than a task.
4. **N3/FR-1 (CTC line 19 fabricated zero).** Smaller in engineering size than #3 but categorically
   worse in kind: it is not a refusal, it is a *signed wrong number* on a filed return, which is the
   defect class this repo's own doctrine (CLAUDE.md "an entry is testimony") treats as most severe.
5. **AMT-E4/E5/E6 (Tier-2 attach completion).** Tier 1 already shipped and serves the common AMT
   trigger; what remains blocks a specific, real, and non-trivial population (ISO-exercising equity
   comp employees — flagged in-repo as *"the most likely wrong filed number"* before Tier 1's ISO
   mitigation — plus AMT-depreciation-divergent Schedule C filers and the MFS-kicker band), and E4 (PDF
   field-readback verification) is a correctness gap on the population Tier 1 *already* serves, not
   just Tier 2's.
