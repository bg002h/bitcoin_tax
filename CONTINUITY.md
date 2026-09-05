# CONTINUITY — bitcoin_tax (TaxApp)

_Last updated: **2026-09-04**. Written at a deliberate pause; safe to exit. **Read this file first.**_

---

# ★★★ RESUME POINT — the TY2025 push. Owner asleep; assistant proceeding autonomously (2026-09-04).

## ① READ THIS FIRST — the goal changed scope

The owner's goal is **filing a COMPLETE US federal return**, not the bitcoin slice. A four-agent
recon on 2026-09-04 established what actually blocks that, and it is not what the day was spent on:

- **TY2025 — the year being filed NOW — has no full-return path.** 17 bundled AcroForm templates for
  TY2024, **5** for TY2025, 0 for TY2026. `full_return_for(2025)` is a deliberate, tested `None`
  whose unblock condition is written down: **Schedule 1-A complete, all six parts**.
- **E-file is closed to this product as built.** MeF is gated to Authorized e-file Providers;
  Direct File shut for FS2026; Free File Fillable Forms is manual entry. **Paper is the channel** —
  so prior-year AGI and Self-Select PIN are correctly N/A, not gaps.
- **Year-porting is cheap for 8 of 12 remaining forms and NOT for Schedule A / Form 6251**, both
  restructured by Pub. L. 119-21. Form 8275's TY2025 edition is byte-identical to TY2024.
- Recon reports: `design/agent-reports/2026-09-04-recon-*.md`, `design/ty2025/recon-year-port-delta.md`.
- Memory: `ty2025-is-the-blocker.md`.

## ② DONE THIS SESSION

- **`main` = `2b34c13c`, pushed, CI-green-by-construction.** `chore/archive-reconciliation` merged
  `--no-ff` at `13be9a79` (tree byte-identical to branch tip). Five review rounds; the branch caught
  a REAL REGRESSION I introduced (pin 7→0 disarmed the only tripwire on a manifest wipe) and two
  false-PASS instruments. `regen` now refuses to shrink the manifest; mutation-verified × 6.
- **An UNDERSTATEMENT path fixed** (`feat/schedule-1a-ty2025`): the scope attestation never named
  pension / IRA / Social Security while btctax models no 1040 line 4a–6b at all. Now names the FORM
  NUMBERS; five terms pinned by test; mutation-verified.

## ③ IN FLIGHT — branch `feat/schedule-1a-ty2025`

Seven agents + one workflow, all dispatched 2026-09-04. **Each persists its own report; recover from
the file, never from a transcript.**

| what | lands at |
|---|---|
| review of the Schedule 1-A plan's **r4 fold** (gates T2) | `design/ty2025/reviews/PLAN_schedule_1a-r4fold-review.md` |
| **long-range plan** to a filed return | `design/LONG_RANGE_PLAN_filing.md` |
| **SPEC Schedule A TY2025** (restructured) | `design/ty2025/SPEC_schedule_a_ty2025.md` |
| **SPEC Form 6251 TY2025** (restructured) | `design/ty2025/SPEC_form6251_ty2025.md` |
| **SPEC retirement income 4a–6b** ✅ landed | `design/ty2025/SPEC_retirement_income.md` |
| **FR-1 build** (CTC line 19 → `Option<Usd>`) | worktree, branch `fix/fr1-ctc-line19` |
| **label-reader fix** (drops Form 6251 line 1a) | worktree, branch `fix/label-reader-drops-1a` |
| **understatement audit** (workflow, 6 lenses + adversarial verify) | `design/agent-reports/2026-09-04-understatement-audit.md` |

★★ **T2 of Schedule 1-A is GATED on two things**: the r4-fold review clearing, and the label reader
being trustworthy — its KAT is specified to be DRIVEN BY that reader, and the reader currently drops
a line while reporting "0 without a box".

## ④ NEXT ACTIONS, in order

1. Fold each report as it lands: verify claims independently, persist verbatim in its own commit,
   fold in a second, gate output in the message.
2. Merge the two worktree branches after review. ★ `authority-manifest --regen` REFUSES in a fresh
   worktree by design (the 60 gitignored PDFs are absent) — regen in the SHARED tree only.
3. Build Schedule 1-A **T2** once ① the r4 fold clears and ② the label reader is fixed.
4. Then T3–T7, then the TY2025 form set (8 cheap, 2 structural), then retirement income, then FR-1
   if not already merged, then 1040-ES.

## ⑤ STANDING CONSTRAINTS

- **NOT authorized: tag, publish, or the crates.io token** (still unrevoked from v0.17.0).
- Owner asleep; **do not block on questions** — decide, record the decision, proceed.
- Fable is authorised for this stretch ("consult fable if needed").
- One owner per artifact; anything writing code gets a worktree.

## Where things stand

**`main` is `945d1ac2`, local and remote IN SYNC.** The `feat/filing-readiness` branch was merged
**`--no-ff`** on 2026-08-23 (65 commits, 92 files, +15,106/−981) and `main` was pushed 2026-08-30.
The merge commit's two parents are `3fc88497` (old main) and `3d01b5e3` (branch tip), and its tree is
**byte-identical to the branch tip** — verified, so the merge introduced no content of its own.

**NOT tagged. NOT published.** Both remain open decisions, and publishing is the genuinely
irreversible one (crates.io is immutable). ★ **The crates.io temp token from the v0.17.0 publish is
STILL UNREVOKED** — deal with that BEFORE another publish, not after.

## What shipped, in one paragraph

The filing-readiness plan (phases 1–4) plus two owner-authorised widenings, both of which make btctax
do MORE rather than refuse: **(A)** a taxable-income≤0 year carrying a capital-loss carryforward-IN
now FILES (the refusal AND its `RefuseReason` variant deleted, so every consumer `E0599`'d rather
than leaving an unreachable arm), and **(B)** `--write-carryover` ROLLS the §1212(b) carryover into
next year's inputs stamped `Computed` — btctax became an **author** of a figure it previously only
read. (A) was safe to lift only because `tax::capital_loss_carryover` transcribes the §1212(b)(2)(B)
worksheet; the old flat `min(loss, $3,000)` had no taxable-income term and understated the surviving
loss by up to the whole §1211(b) allowance.

## ★★★ Seven independent reviews, all persisted VERBATIM in `reviews/`

phase1 · phase2 · phase4 · final (first 36 commits) · widening (2I+1M, folded) · fold re-review
(`sound` 0C/0I) · **pre-merge B3, scoped `main..HEAD`** (`merge` 0C/0I).

**The pre-merge pass earned its cost, and this is the lesson to carry.** The six earlier rounds were
all RANGE-scoped and their windows did not add up to the branch: `99628341` is where the last
whole-branch review's window closed, and **34 files were edited on both sides of it**. Pointed at that
seam, it found the lifted (A) refusal still described in **present tense on six surfaces** — including
`screen_absolute`'s own contract doc contradicting its body, and **SPEC §4.10's refusal table, which
still MANDATED the refusal**. Doc drift, not behaviour; all six fixed before the merge, so `main`
never carried a spec contradicting its own code.

★★ **K19 existed to prevent exactly that drift and saw none of it** — it greps the deleted
IDENTIFIER, which was deliberately kept out of prose, and it never scanned `design/` at all. Filed as
**FR-22**, with a phrase blocklist explicitly rejected as the growing-blocklist shape this repo
already warns against.

## ★★ A TRAP THAT COST A WEEK OF RED CI — read before trusting a green `make check`

CI was **red on `main` itself** (`3fc88497`, run 32550151114) and on every branch push, for a lint
neither the branch nor `make check` could see: `clippy::chunks_exact_to_as_chunks` at
`cite_check.rs:272` — a line **present on `main` and untouched by all 65 commits**. CI's `stable` had
moved to **1.98.0**; local `stable` AND the default nightly were both **1.97**.

★★★ **`make check` runs clippy on the DEFAULT toolchain, so a lint added in a newer stable is
invisible to it no matter how green it looks.** This is the documented "make check is NOT CI" trap
arriving by a NEW route — not a missing JOB this time, but a **stale TOOLCHAIN running a job we do
have**. The fix was `rustup update stable` + CI's exact command with `--keep-going` (so a first error
could not mask later ones): exactly one lint workspace-wide.
★ **Local `stable` is now 1.98.0; the default toolchain is untouched (nightly), so `make check`
behaves exactly as before.** The durable fix — pin a toolchain, or make the local gate use `+stable`
— was deliberately NOT done: it changes how every future session validates, and that is a decision.

## ★ THE NEXT ACTION — nothing is gating. Owner's pick.

1. **Tag + publish** (irreversible; revoke the stale crates.io token first).
2. **Fix FR-19** — one conjunct. Reproduced: one command emits two contradictory statements; without
   `--force`, *"pass `--force` to overwrite it with the computed §1212(b) carryover"*, with `--force`,
   *"★ NOT WRITTEN … stamps nothing."* Same class as the widening review's B-1.
3. **Decide FR-20** — the canceled-debt refusal's Form 982 claim is broader than what it enforces;
   the underlying COD-income scope gap is pre-existing and larger than the wording.
4. **FR-21 / FR-22** — two checkers proven blind, each filed with its structural fix named.

★ Any of 2–4 is authorship and re-earns the review gate.

Delivered: the whole filing-readiness plan (phases 1-4) plus **two owner-authorised widenings** —
(A) a taxable-income<=0 year with a capital-loss carryforward-IN now FILES (refusal variant deleted),
and (B) `--write-carryover` now ROLLS the §1212(b) capital-loss carryover, stamped `Computed`.

**Five independent reviews are persisted VERBATIM in `reviews/`** (phase1, phase2, phase4, final,
widening). Read those rather than re-deriving; every commit message carries its mutations with the
verbatim RED output.

## ✅ THE WIDENING REVIEW IS FOLDED — `9728e2ec`. What the machine-check settled

`reviews/filing-readiness-widening-review.md` (report at `02939632`, fold at `9728e2ec` — the two
commits are separate on purpose, so `git diff 02939632..HEAD` is exactly "what changed in response to
what"). It returned **needs-changes: 2 Important + 1 Minor**; (A) was SOUND, all nine earlier fold
commits SOUND, the refusal surface coherent.

★★★ **The owner's instruction was to machine-check before editing a line, and it paid.** The reviewer
had named its own escape hatch — *"if a caller suppresses the summary or re-stamps on the
grounded=false path, B-1/B-2 evaporate"*. Resolved against the tree: `write_back_carryover` has
**exactly one** production caller (`main.rs:197`, which unconditionally prints), and the only stamp
site outside the gate is the import preservation arm, which stamps only where `existing` was already
`Computed`. **No such caller exists.** All four limbs then reproduced as printed observations:

| limb | observed |
|---|---|
| B-1 | summary said *"capital-loss carryover short $0.00 / long $0.00"*; stored provenance `User` |
| B-2 | roll → `long 34000 / Computed`; remove grounding; re-roll, `--force`, zeroing import all left it |
| B-2(ii) | a TOML provenance key minted the stamp — `long 99000 / Computed`, exit 0 |

★ **One limb was MY fixture's fault, not a finding**, and the distinction matters: the first forge
probe put the key inside `[[w2s]]` (a bare key after a table header parses into it). Rebuilt with the
key before the first table header; it then reproduced for real.

**Fixed on the branch:** B-1 (one `capital_loss_roll_is_grounded` predicate read by both the writer
and the message it prints; `★ NOT WRITTEN` names the carryover and any stale figure), B-2's forge
half (`income import` normalises **all four** provenances plus the per-item charitable one — the whole
class, since every one is `#[serde(default)]`), and the Minor.
**Filed, not fixed:** **FR-17** (no retraction path for a `Computed` stamp — owning phase TY2025,
same acceptance as FR-8's residue) and **FR-18** (`income scrub` loses the provenance; found by
CONTROLLING for it, and its mechanism is deliberately not diagnosed).

★★ **The transferable bit:** FR-18 exists because the check on *"did my fix break the scrub round
trip?"* was run as a **control** — plant `Computed`, run with AND without the change — instead of
just running the suite. Both reds looked identical, which is what proved the loss pre-existing. The
suite alone said green, because `maximal_sentinel` pins every provenance field at the DEFAULT variant.

## ★★ STANDING CONSTRAINTS — owner-set, do not drift from these

- **ONE AGENT AT A TIME. NO PARALLELISM. NO `Workflow`. Ultracode OFF.** Delegation is still fine;
  fleet size and parallelism are not. When in doubt, do it inline.
- If workflows ever return: **<=2.4M subagent tokens and <=20 agents** per run. Bound data-dependent
  fan-out IN THE SCRIPT (a 45-agent/4.8M run came from one high-effort refuter per candidate break).
- **DO NOT merge to `main`, tag, or publish.** Owner's call, and the last irreversible step here.
- **Phase 5 (EITC/ACTC) is DEFERRED**, not cancelled — "we will get back to it someday". Fully scoped
  as `FOLLOWUPS.md` **FR-16**, including the machine-verified oracle trap
  (`design/direction/ORACLE-TRAP-credit-takeup.md`: taxcalc's DEFAULTS report EITC=$0 for a household
  owed $4,778.18, position-dependently — it would have looked like a second oracle corroborating
  btctax's own wrong zero). Do not start it without being asked.
- **Neither oracle witnesses a carryover level** (taxcalc takes it as an INPUT; OTS emits none). That
  is the §G-9 limit, not a gap — never propose an oracle check on it, and never add a corpus cell for
  the newly-admitted household (the wage band is floored above the childless-EIC range on purpose, so
  such a cell would be admitted only by an oracle that models the credit away — a false witness).

## Repo hazards learned this session

- **The pre-commit hook used to leak `GIT_DIR` into the test suite**, which re-inited the shared repo
  as BARE and broke every `git add` with *"must be run in a work tree"*. Fixed at the hook
  (`scripts/pre-commit`) and in the production path (`xtask harness_check`). If it ever recurs:
  `git config core.bare false`.
- **Worktree agents branch from `main`, not from the current branch.** Every delegated implementer
  must run `git merge feat/filing-readiness --no-edit` FIRST and confirm the expected test count.
  Phase 2 skipped this and cost a round of hand-resolved conflicts plus two compile breaks.
- **The harness blocks subagents from writing report files.** Make delegated work durable through
  COMMIT MESSAGES; the controller persists reviews verbatim in their own commit before folding.
- Revert planted mutations with a **cp backup**, never `git checkout -- <file>`.
- Assert a mutation's anchor matched **exactly once** before believing a result. That check caught
  two bad plants this session.

---

<details>
<summary>Superseded — the previous continuity document (v0.17.0 / income scrub), kept for history</summary>

# CONTINUITY — bitcoin_tax (TaxApp)

_Last updated: **2026-08-10** (v0.17.0 RELEASED and PUBLISHED — `income scrub` shipped). Written at a
pause; safe to exit._

---

## ▶ RESUME HERE — nothing is in flight. `income scrub` SHIPPED in v0.17.0.

**v0.17.0 is live**: all 10 crates on crates.io, verified 10/10 via the **sparse index** *and* by
`cargo install btctax-cli --version 0.17.0` from the registry — which is the check that matters,
because an escaping `include_str!` ships a broken tarball with exit 0 and the index cannot see it.
The registry-built binary reports `btctax 0.17.0` and carries `income scrub`. Tag `v0.17.0` pushed,
GitHub release published, `main` at `f385570d`.

### ★★★ THE ONE OPEN ITEM: a FABLE DEEP PASS on `income scrub` — see `FOLLOWUPS.md`

Owner-scheduled for later this week or next, deliberately POST-publish (a pass at that depth is
expensive; the owner chose a considered read over a release gate). **Its brief is already written and
is the point** — it does NOT ask for a fresh audit:

> Nine passes ran on this feature, and each one found an instrument that was **green and blind**.
> Each fix was then itself found blind by the next pass. **What is still green and blind now?**
> Name the instrument, not the defect.

The four CLEAN sections in `reviews/scrub-*` are marked off-limits so budget is not re-spent where
things are settled, and the settled decisions are listed so it cannot re-litigate them (especially the
`year - 1` disjunct — two attempts to name a mechanism for it were both wrong; a third is not wanted).

### What `income scrub` is, in one paragraph

`btctax income scrub --year N [--out FILE]` emits a copy of a stored return with the identity replaced
and every computed FIGURE intact, so a filer can hand a real return to a stranger to reproduce a
defect. **The product is the AUTHORIZATION, not the file** — so it refuses more than it scrubs: only
when the ledger contributes nothing to the year (four projection-wide disjuncts, NOT the 1040
digital-asset box), preserving the *equivalence class* rather than the value (a malformed SSN or EIN
stays malformed, so the copy refuses exactly where the original did), dropping the IP PIN rather than
minting a live IRS credential, writing owner-only with a provenance marker that makes a plain
`income import` refuse.

### ★★ THE TRANSFERABLE LESSON OF THIS WHOLE BRANCH — one shape, nine times

Every pass found **an instrument that had never been watched discriminating**:

| what was green and blind | how it was caught |
|---|---|
| 2 of the refusal predicate's 4 disjuncts | mutation — deleting either left 2649 tests green |
| the derived field axis | blind to all four fields the reviews were about |
| §3.3's matrix refusal assertion | compared `Some(ForeignTrust)` to ITSELF on every row |
| the TOML round trip | could not fail on a lossy emit (fixture too thin) |
| the `--out` mode test | blind to the window it was written for |
| a scan list | 3 entries naming tokens that no longer existed |
| the A3 write-hook test | depended on an ambient `target/debug/xtask` |
| the Windows PII test | ran the **WSL launcher**, never a shell, for months |

★★★ **The last two were CI-red for days and read as something else.** The Windows one announced *"the
PII exclusion rule misclassified 15 of 23 vectors"* — an accusation against a security control, from a
harness that had never executed anything. **An instrument that cannot say WHY it failed will be
believed about WHAT failed.** Three guesses at the cause were all wrong; what settled it in one CI
round was making the test resolve the rule FIRST and print what came back.

### The build's own findings, worth not re-deriving

- **The spec mandated an edit to a FROZEN file** (`tax/compute.rs`, content-pinned). Five spec-review
  rounds missed it; `cargo nextest` found it in one run. The hard-blocker disjunct uses the predicate
  every non-frozen caller already uses instead.
- **A malformed EIN was still upgraded to a well-formed synthetic**, so the copy filed where the
  original refused and claimed a $1,546.80 §6413(c) credit. Found by the whole-branch pass — §3.2 named
  the EIN, `EinMap`'s doc *delegated* to §3.2, and §3.2 had no EIN leg. **A pointer to another section
  is not an implementation.**
- **The PII allowlist bucket DOES grow.** A comment claiming otherwise was falsified two commits later.
  What stays refused is the *structural* window (`^9[0-9]-[0-9]{7}$` — it would exempt real EINs under
  91/94/95/99); token-exact entries with citations are bounded bookkeeping.

### Owner-only, still open

- **The crates.io token** — owner has decided: it auto-expires, and is being used until then. No action.
- `scripts/.pii-patterns` exists (owner-supplied, untracked, gitignored); the push gate is green.

### Traps that cost time, so they are not re-hit

- **`make check` is NOT CI.** It is nextest + clippy only — no fmt-check, msrv, `check-isolation` or
  `pii-scan`, and it cannot see a platform-specific test. This branch was CI-red for four days while
  `make check` was green.
- **A golden cannot validate its own regeneration.** After the 0.17.0 bump both goldens were
  regenerated and the diffs READ: exactly one version line each.
- **An unapplied mutation is indistinguishable from a surviving one.** One "surviving" mutation had
  simply never matched its anchor (a `\n` escaping slip). Every mutation now asserts its anchor first.
- **The pre-push PII scan flags the review that filed the finding.** Twice now. Reviews stay verbatim,
  so the scan config moves, not the record.

---

_Everything below is the historical record of earlier tracks (the pen-deferral branch, the AMT/§G-13
census, the form-authority pipeline). All of it shipped; it is retained for provenance, not as a work
queue._

---

## ★★★ ACTIVE WORK — branch `feat/no-pen-deferrals` (READ THIS FIRST, updated 2026-07-31)

**28 commits ahead of `main`, tree clean, 2527 tests green, all five gates.** `main` itself is 99
commits ahead of `origin/main` — **nothing has been pushed in a long time**; see §7 and the BLOCKED
box below.

### ⛔ BLOCKED ON THE OWNER — read before planning any ship

| blocker | why | who |
|---|---|---|
| **push** | `scripts/.pii-patterns` does not exist ⇒ `scripts/pre-push` is fail-closed, every `git push` exits 1. The file is owner-specific and untracked; **the assistant must not author it** — a wrong guess turns the gate green while scanning for nothing. Repo is PUBLIC; 123 commits have never had the owner-specific scan. Escape: `BTCTAX_PII_BYPASS=1 git push`. | owner |
| **publish** | Blocked by the same thing, and MORE so: `cargo publish` uploads a source tarball to a **public, immutable** registry. Also needs a version bump (0.14.0 is already live) — scoped by the owner to **prepare 0.15.0, do NOT publish**. | owner |
| **crates.io temp token** | Still unrevoked since the v0.14.0 publish (2026-07-29). Deal with it BEFORE another publish, not after. | owner |

### ★★★ THREE REVIEW ROUNDS RAN, AND THE THIRD FOUND WHAT THE FIRST TWO STRUCTURALLY COULD NOT

| round | scope | model(s) | verdict |
|---|---|---|---|
| r1 | `7bde148..65270db` | Opus + Sonnet | 0C/0I; 6 Minors (§G-19a is the one that mattered) |
| r2 | `afa0ffe..HEAD` | Opus + Sonnet | 0C/0I; 2 Minors + 1 Nit; **20/20 mutations killed** |
| **pre-publish** | **`main..HEAD` (whole branch)** | **Fable** | **`publish-after-fixing-X`** — one **Important** |

Verbatim outputs in `reviews/`: `crypto-slice-trio-{tax-lens-opus,instrument-lens-sonnet}-r1.md`,
`branch-r2-{tax-lens-opus,instrument-lens-sonnet}.md`, `branch-prepublish-fable.md`.

★★★ **The Important lived in `b94508d` — the EARLIEST commit — which precedes both earlier review
ranges.** Schedule B's FBAR pair printed ungated, so a 7a Yes→No correction put a checked FinCEN-114
box beside a checked "No" under §6065. **The fix already existed in the branch** (Schedule C line J,
nine commits later, reasoning included) and nobody carried it back, because no reviewer ever held both
commits at once. Fixed in `3bcf3a0`, and the lesson is now **harness B3** (`design/HARNESS.md` +
`CLAUDE.md`): *a per-range review is not a branch review; the final pass takes `main..HEAD` and is
pointed at INTERACTION.*

### The owner's direction, and the correction that reshaped it

1. Owner: *"Let's reverse that decision to leave anything to a user's pen. And then proceed on 8995."*
   (`btctax limitations` said things like *"Schedule C lines G, H, I, J — left blank (deferred to your
   pen)… Fill them in yourself."*) So: ASK the filer and PRINT the answer; never fabricate it.
2. Then, mid-build, owner: **★★★ *"We should review all refusals. A lot of items on a tax return don't
   need to be answered (or asked, come to think of it)."*** — a correction. The build had been adding
   questions that REFUSE TO FILE when unanswered. That is too aggressive.
3. Owner chose scope: **"Safe subset + reverse the FBAR"** (see the decision table below).

### What LANDED

| commit | what |
|---|---|
| `b94508d` | Schedule B 7a's FBAR sub-question added as a class-(A) refusing question. ★ **Now scheduled for REVERSAL** — see below. Its PRINTING is correct and stays. |
| `3b22ca1` | ★★ **A real defect.** `schedule_b_lines` did `unwrap_or(false)` on `foreign_accounts`/`foreign_trust`, printing a **"No" the filer never gave**. Both are now `Option` to the writer. |
| `7ee5afd` | Schedule SE line A reclassified `gap` → `unmodeled`; clergy self-employment documented OUT OF SCOPE. GAPS 16 → 13. |

★★ **The most transferable thing in `3b22ca1`: THE FIRST TEST DID NOT CATCH THE BUG.** The KAT in
`btctax-forms` builds `ScheduleBLines` directly, so it pins the WRITER and is blind to the CONSTRUCTOR
where the bug lived — restoring `unwrap_or(false)` left it GREEN. Found only by mutating the fix. A
second test now sits at the constructor (`printed.rs`, `part3_answeredness_tests`). Mutation-test the
FIX, not just the code.

### The refusal review — outcome (the workflow output lived in /tmp and is GONE; this is the record)

47 `RefuseReason` variants + 16 registry questions adjudicated against this criterion:

> A **refusal** is justified ONLY if proceeding without the answer would (a) produce a WRONG NUMBER,
> (b) put FABRICATED TESTIMONY on a signed return, or (c) silently expose the filer to a PENALTY or a
> lost right. Failing all three → **skippable** (ask, silence lawful) or **don't ask at all**.

An adversarial pass returned **UNSAFE** on 2 of 9 proposed relaxations. Both verified in source.

**★★★ DO NOT DO THESE TWO — they put a wrong figure on a signed return:**

- ~~**Do NOT drop `AmtScreenTriggered` from the report path.**~~ **RESOLVED 2026-08-03** — the
  precondition it names ("Blocked until Schedule 2 line 2 exists") is met: Schedule 2 lines 2 and 3 are
  mapped and emitted, and `l18` now adds `amt.line11` with `compute_6251` moved above it. This entry was
  RIGHT, and the understatement it predicted was live in the tree until the two-chain KAT
  (`the_absolute_total_tax_equals_the_printed_1040_line_24`) was written. Kept, struck through, because
  the reasoning is the record of a correct call. Original:
- **Do NOT drop `AmtScreenTriggered` from the report path.** The justification ("Form 6251 is computed
  correctly anyway") is true and IRRELEVANT. `total_tax` (`return_1040.rs:1328`) is assembled BEFORE
  `compute_6251` runs (`:1353`), and hardcodes Schedule 2 line 2 to zero in a comment at `:1319-1321`
  that **names this very refusal as its warrant**. Printed chain is worse: `printed.rs:652` pins 1040
  L17 to `Usd::ZERO` and `Schedule2Lines` has no AMT field at all, so L24/L34/L37 all omit the AMT —
  printed by `render.rs:1547-1558` in a block the CLI calls *"exactly what the filed PDF carries"*.
  **Understatement.** Blocked until Schedule 2 line 2 exists.
- **Do NOT relax `IraDeductionClaimed`.** `sch1.ira_deduction_claimed` has NO compute consumer
  (`classifier.rs:387` destructures and discards it) and `Schedule1Lines` has no line 20 — so relaxing
  it files a return with the claimed deduction silently GONE. Also the proposed single "active
  participant?" question fails open under §219(g)(1)/(g)(7), which reaches the SPOUSE's coverage too.

**★★ THE LESSON, worth more than either item:** *a refusal that a compute path was built to rely on is
not over-asking — it is LOAD-BEARING.* **Before relaxing any refusal, grep for the code whose
correctness comment names it.**

### The decision table for the six pen-deferral questions

| line | verdict | status |
|---|---|---|
| Schedule C **G** (material participation) | **DON'T ASK** — answer moves no figure (§1411(c)(6) shelters the SE-base Sch C income either way) | ✅ already `unmodeled` |
| Schedule C **H** (started/acquired) | **DON'T ASK** — a check-if-true box with NO "No" widget, so an explicit No and a never-asked blank are the IDENTICAL mark on the page | ✅ already `unmodeled` |
| Schedule SE **line A** (Form 4361) | **DON'T ASK** — no figure moves, and btctax models no clergy concept anywhere | ✅ done, `7ee5afd` |
| Schedule C **I / J** (Forms 1099) | **BUILD AS SKIPPABLE**, not a refusal. §6721/§6722 exposure is real (limb c) but there is no in-form Caution; advisory on skip must name §6721/§6722 | ✅ **DONE 2026-07-31** (`de8ffd8`) |
| Form 8283 **5a/5b/5c** (restrictions) | **BUILD AS REFUSAL-ON-YES** (ask; "No" proceeds, "Yes" refuses). The ONLY limb-(a) item of the six | ✅ **BUILT 2026-07-31** — and it was the LAST §G-13 gap, so **`GAPS` is now 0**. ★★★ The structural obstacle filed as §G-21 was dissolved by the OWNER: ask it as a **return-level universal** ("did any donation have strings attached?"), not per-donation. Unanswered refuses too, not just a Yes. Five observed B1 kills. See §G-21, now closed. |
| Form 8283 **page-2 identity** | **NO QUESTION** — pure map fix (`f8283.map.toml`), btctax already holds name + TIN | ✅ **DONE 2026-07-31** (`fb33ac6`) |
| Schedule C **I / J** | (see the row above) | ✅ **DONE 2026-07-31** (`de8ffd8`) — skippable + §6721/§6722 advisory |

★ **The Form 8283 `needs_review` path can NOT substitute for a refusal**: its only consumers
(`main.rs:792`, `:883`) are `eprintln!`s emitted AFTER `full_return_paths` are written to disk — the
PDF with the unreduced deduction already exists when the warning prints.

### ⬜ REMAINING, in order

1. ~~**Reverse the FBAR question**~~ — ✅ **DONE 2026-07-30.** `FbarFilingRequired` moved from
   `FORM_QUESTIONS` (class A, refuses) to `SKIPPABLE_QUESTIONS` (class B, lawful silence). It is now
   `SkippableId::FbarFilingRequired` at index 7, still live iff `foreign_accounts == Some(true)`, and
   skipping it fires the NEW `Advisory::FbarSubQuestionNotAnswered`, which quotes Schedule B's Caution
   verbatim. `RefuseReason::FbarFilingRequirementUnanswered` is gone; the classifier records the leaf
   as a `Class::NoTaxDirection` exemption. **The printing side needed NO change**, as predicted — the
   `Option` was already load-bearing to the writer, so a skip prints a true blank.
   ★ The prediction that survived contact: *nothing on the return reads this box.* That is what makes
   it class (B) — the penalty the Caution names attaches to **not filing FinCEN Form 114**, an
   obligation the box neither creates nor removes.
   ★★ **The two `is_none()` guards in `return_refuse.rs`'s property harnesses are now UNEXERCISED** —
   the FBAR was their only live case. They were kept, with the comment rewritten to say so: the next
   question whose liveness depends on another question's *non-neutral* answer would hit the same wall
   silently.
2. ~~**Downgrade the death pair**~~ — ✅ **DONE 2026-07-30** (`b3c9829`). Both are now
   `SkippableId::{Taxpayer,Spouse}DiedDuringYear`. The premise held on inspection: `is_aged`'s
   `(None, None)` arm returns `false`, so silence already forgoes the addition and the refusal was
   redundant with a fail-safe beneath it. The spouse gate is now **MFJ-only** (and so is `DodSpouse`)
   — on MFS its answer could never move a figure, yet it refused there.
   ★ New `Advisory::AgedBoxForfeitedDeathUnanswered`, firing **iff a DOB on file would have
   qualified** — not merely on the unanswered gate, which would put it on nearly every return.
   ★★ **Three fixtures broke, all the same honest way**: they got the aged box free from
   `answer_all_live_declarations`. Fixed by STATING the claim, never by weakening the assertion. The
   killer mutation — `(None, None) => true`, the shipped v0.14.0 understatement — reds by name.
   ★★ TY2024 golden matrix **byte-unchanged** (md5 `c4e1853…`).
3. ~~**`SsnMalformed`**~~ — ✅ **DONE 2026-07-30** (`443d4a0`). Deleted from `screen_inputs`; the
   packet boundary is untouched and was **verified to cover all three shapes** (`Missing`,
   `NotDigits`, `WrongLength`) — "build rejects missing" would not have been enough. The replacement
   test asserts BOTH sides for 3 shapes × 3 subjects.
4. ~~**THEN Form 8995 line 3**~~ — ✅ **DONE 2026-07-30** (`3d09552`). Built as the mirror of the
   REIT/PTP pair, end to end (inputs + provenance, `has_qbi`, `compute_8995`, `qbi_over_threshold`,
   printed lines 3/4/16, `PrintedInputs`, `AbsoluteReturn`, the negative screen, classifier, map,
   emitter, write-back, TOML template, LIMITATIONS).
   ★★ **The line had never been transcribed AT ALL** — `Form8995Lines` went line2 → line4, with no
   `line3` field. Adding it `E0063`'d every literal and call site, which is how the plumbing was found
   rather than guessed.
   ★★ **LINE 3 PRINTS ONLY WHEN THERE IS A CARRYFORWARD.** A pre-existing test said *"L3 must be
   blank"* and it was right: line 3 is the one line on this form that is neither derived nor computed
   — it is TESTIMONY — so a printed `0` swears the filer had no prior-year QBI loss. Every other line
   is btctax's own arithmetic and prints its zero legitimately.
   ★★★ **THE WRITE-BACK MUTATION SURVIVED THE WHOLE SUITE.** Deleting
   `next_year.qbi.qbi_carryforward_in = ar.qbi_carryforward_out` left 2519 tests green. A silently
   extinguished carryforward is not a one-year error — it enlarges the deduction in every later year
   and no single return looks wrong. Closed by `form_8995_line16_carries_into_next_years_line3`.
   ★ GAPS ratchet **13 → 12**; TY2024 golden matrix byte-unchanged.

### ★★ THE RANKED BACKLOG — from the 2026-07-30 "what next" recon (8 agents, adversarially checked)

Recovered and pinned here because the workflow output lived in `/tmp`. Status is as of this pause.
Ranking principles used: a wrong tax figure outranks everything; fails-SILENTLY outranks fails-CLOSED;
ready outranks blocked-on-a-decision; cheap hazard-removal outranks large projects.

| # | item | effort | status |
|---|---|---|---|
| ~~1~~ | ~~Schedule C line G → Form 8960 line 4a~~ | — | ❌ **DEAD — REFUTED.** Would have DOUBLE-TAXED SE income. §1411(c)(6) shelters it. See the ACTIVE WORK correction above. |
| ~~2~~ | ~~**Form 8995 line 3**~~ | days | ✅ **DONE 2026-07-30** (`3d09552`) — see remaining-list item 4 above. |
| ~~3~~ | ~~**Crypto-slice export trio**~~ | hours | ✅ **DONE 2026-07-30** — see the box below. **(b) shipped line 17 ONLY; line 20 was REFUSED on inspection.** |
| ~~4~~ | ~~`ARCHIVE_RECONCILIATION_REVIEW_BY`~~ — ~~re-decide the residual archive duplication or reset the date with a written reason~~ | hours | ✅ **CLOSED 2026-09-04.** Reset row 2 recorded the decision, both duplicate groups were resolved (`DUPLICATE_SOURCE_GROUPS` 7 → 0) and the constant, its test and the `run()` branch were retired **with their subject**. The RESET LOG is kept in `archive_check.rs`. The row's two sub-items also landed: `the_archive_count_may_only_shrink` got its `assert_eq!` companion, and the four-archive doc comment was corrected. See “⬜ WHAT IS OPEN NOW” row 5, and §0 step ③. |
| ~~5~~ | ~~**§G-9a** — the §63(f) BLIND box~~ | hours | ✅ **CLOSED 2026-07-30 as "no change needed"** (`1fa75a1`), the outcome this row predicted. i1040gi names ONE box in the carve-out, and the mechanism says why: age is a DURATION test the year can straddle, blindness a POINT-IN-TIME test whose anchor a decedent's short year satisfies. Pinned by a test on i1040gi's own worked example. ★ Surfaced **§G-20** in the same passage: the MFS spouse's aged/blind boxes are forgone, and the code recorded that as THE RULE when it is a conservative omission. |
| **6** | **Schedule 1-A plan r3 was NEVER independently reviewed** — the doc reads "Status: r3" but `design/ty2025/reviews/` holds only `…-opus-r1.md`; r2→r3 folded a 13-agent census (`c92cb9b`) that added T3a wholesale. Not green under this repo's own re-review-after-every-fold rule. Then build B3 T2 | hours + days | ⬜ Only if B3 is the chosen track. Owner decision. |
| ~~7~~ | ~~Batch the remaining §G-13 declarations~~ | — | ⚠️ **SUPERSEDED by the refusal review.** Its premise (ask them all) is what the owner corrected. Use the decision table above instead: SE line A and Sch C G/H are DON'T-ASK and done; FBAR is built and pending REVERSAL; only Sch C I/J (skippable) and 8283 5a/5b/5c (refusal-on-Yes) remain. |
| **8** | **Push the backlog.** `core.hooksPath=scripts`, `scripts/.pii-patterns` does NOT exist, `scripts/pre-push:27-35` is fail-closed ⇒ every plain `git push` exits 1. Format documented at `scripts/README-pii-setup.md:25-52`. Sanctioned escape: `BTCTAX_PII_BYPASS=1 git push` | minutes | ⬜ **OWNER-ONLY** — the patterns file is owner-specific and untracked; the assistant must not author it. Repo is PUBLIC. Generic scan is clean across all commits. |
| — | **Revoke the crates.io temp token** from the v0.14.0 publish | minutes | ⬜ owner-only, long-standing |

### ✅ Backlog #3 — the crypto-slice export trio, as SHIPPED (2026-07-30)

**(a) The all-in LTCG marginal rate.** `MarginalRates` gained `niit_at_margin` + `ltcg_all_in()`;
`report --tax-year` and the TUI Tax tab now headline `LTCG 0.238 all-in (§1(h) 0.20 + §1411 0.038)`.

- ★★ **`niit_applies` is NOT the predicate, and mistaking it for one is the bug.** It is the
  crypto-vs-no-crypto DELTA, so it is **false** for a filer already over the §1411 threshold whose
  crypto did not *raise* NIIT — and that filer's next sold sat still costs 3.8 points. The new flag is
  `magi_with > thr && nii_with >= 0` (the next dollar's right-derivative, boundary-resolved downward
  like `ltcg`'s `top <= max_zero`). `niit_at_margin_is_not_the_niit_applies_delta` is the KAT that
  separates them; a second KAT pins the `nii >= 0` conjunct (MAGI over the threshold but NII negative
  ⇒ still no NIIT at the margin). Both mutations killed.
- ★★★ **It tripped `frozen_guard` — `tax/types.rs` and `tax/compute.rs` are CONTENT-PINNED.** The pin
  bump is recorded in that module as "EXCEPTION 1" with the reasoning, per its own documented
  exception process, and rides in its OWN commit. `golden_returns.rs` re-verified byte-unchanged: the
  edit is strictly additive and display-only.

**(b) Schedule D Part III — line 17 ONLY. ★★ The backlog said "17 and 20"; 20 was WRONG.** Line 20 reads
*"Are lines 18 and 19 both zero or blank **and you are not filing Form 4952**?"* — that last conjunct is
a fact about the filer that **no btctax input surface carries** (`f1040sa.map.toml` records Schedule A
line 9 as `unmodeled` for exactly this reason), and lines 18/19 are blank on the slice because nothing
ever *asked*, not because they are zero. A "Yes" there is testimony the filer never gave, and it routes
them to the QDCGT worksheet when the Schedule D Tax Worksheet is required — an understatement path.
Line 17 has no such conjunct: it reads lines 15 and 16, **both printed on the same page**.

- The single definition is `printed::schedule_d_line17`, which the full-return `ScheduleDRouting`
  derivation now also calls, so slice and full return cannot drift.
- ★ **The 2017 revision's Yes/No on-states are `"Yes"`/`"No"`, NOT the `"1"`/`"2"` that 2024/2025 use** —
  dumped, not assumed. The KAT asserts the literal per-year on-state, so an analogy-copied on-state
  (which writes an OFF box ⇒ a line 17 that *looks* filled) reds. Mutation-verified in both directions,
  plus a planted line-20 answer to prove the "18–22 stay blank" guard bites.
- Both Schedule D golden hashes moved; each carries an inline note saying what moved them.

**(c) `form_1040_capgains.pdf` is stamped `WORKSHEET — NOT A COMPLETE FORM 1040`** on every page, on the
opposite diagonal from the DRAFT stamp so a pseudo-reconciled slice carries both legibly. The
full-return `f1040.pdf` is never stamped — asserted in the same breath, because a watermark on every
1040 would be as wrong as one on none.

★★ **The trio's transferable lesson:** the recon item was 3-for-3 on *where* the value was and 2-for-3
on *what to do*. **A backlog entry is a lead, not a spec** — line 20's disqualifying conjunct is
visible in one line of the form's own extracted text.

### ⬜ WHAT IS OPEN NOW (2026-07-31) — everything else on this page is history

**Nothing below is auto-start. Each names why it is not just "more work".**

| # | item | blocked on |
|---|---|---|
| 1 | **§G-19a** — the all-in §1411 display prints `§1411 0` off the model's PARTIAL NII, so a filer with rental income and a crypto loss year is under-reserved by 3.8 points. Fail-safe vs. a third "can't tell" state; costs a second `frozen_guard` pin exception either way | **owner judgment** |
| ~~2~~ | ~~**§G-21**~~ | ✅ **DONE 2026-07-31.** The owner dissolved the blocker: a **return-level universal** ("did any donation have strings attached?") is stronger than three per-gift answers, fits the existing registry, and costs one prompt. ★ **`GAPS` 6 → 0 — the census gap surface is CLOSED.** |
| ~~3~~ | ~~**§G-20a**~~ | ✅ **DONE** (`c7f3942`) — both benefit carryovers got sibling provenance scalars (NOT inside `Carryforward`, which is frozen) and an advisory that MIRRORS the QBI one's direction. ★ Spawned **§G-20b**: the advisory list now has TWO unconditional members; a third means the surface is the problem. |
| ~~4~~ | ~~**§G-20 remainder**~~ | ✅ **DONE** (`fd9c15f`) — the boxes are claimable on MFS and the gate FAILS CLOSED (7 forgo cases pinned). ★ The coupling was resolved by making it ONE predicate shared by the deduction and the question liveness, not by keeping two in step. |
| 5 | ~~**archive review-by**~~ — **✅ CLOSED 2026-09-04.** Reset row 2 recorded the decision, both duplicate groups were resolved, and the constant + its test were retired with their subject. The RESET LOG is kept in `archive_check.rs`. | **owner** |
| 6 | **§G-11** — the emitter cannot express "no testimony". Largest architectural item; needs its own spec | needs a spec |
| 7 | **§G-12** — no Form 8275-R, so a position contrary to a REGULATION is unrepresentable | ⛔ **an ASSET the assistant cannot obtain** — `f8275r.pdf` is unarchived, there is no network, and harness A3 denies new archive paths at `Write` time. The unblock is one `curl` (the exact command is in the §G-12 entry). ★ My 2026-07-31 table wrongly showed NO blocker here; corrected. |
| 8 | **B3 T2 / Schedule 1-A** — and its plan r3 was never independently reviewed (`design/ty2025/reviews/` holds only r1) | **owner: is B3 the track?** |

★★ **A pattern worth carrying, observed twice in two days and now written into §G-18:** *filling a
blank is not automatically an improvement.* Both §G-19a's `§1411 0` and the 1040 line-7 box replaced a
lawful SILENCE with an affirmative statement btctax cannot support. Ask first: **can btctax establish
the proposition the mark asserts, or only that it has no evidence against it?**

★★ **The recon's explicit DO-NOT-DO, kept because it is the most appealing wrong turn:** do NOT resume
the **Tier-2 AMT** thread (E4/E5/E6). It looks like the obvious next step — 13 registered items, the
freshest thread, Tier 1 just shipped — but **Tier 1 ships a REFUSAL for exactly the population Tier 2
serves**, so nobody is receiving a wrong AMT number today; it fails CLOSED, which ranks it behind the
silent understatements. E6 alone (18 adjustment lines + a new existence-question interview) is weeks.
★ And its most ready-looking sub-item — *"teach `ots_direct.py` to read 1040 line 17 so AMT has a
second witness"* — is **ALREADY DONE and the register is stale.**

### Two traps that cost time this session

- **`cargo fmt` reflows a shrinking array onto one line and silently breaks a later string replace.**
  Bit twice. Always re-read the file after an edit near a list.
- **`make check` does NOT include `cargo fmt --all --check`.** The pre-commit hook does, so a commit can
  fail after `make check` was green. Run `cargo fmt --all` before committing.
- **Verify checkbox on-states with `xtask dump-fields`, never by analogy.** Schedule B's Part III pairs
  are `"1"`/`"2"` but **Schedule C's are `"Yes"`/`"No"`** — three independent design passes assumed 1/2
  for both and all three would have been wrong.

---

## 0. ★★ THE ORDER OF WORK — **ALL SIX COMPLETE (2026-07-30)**

This section drove the merged branch. It is kept as the record of what was done and why, **not as a
queue** — there is nothing here left to start.

| # | do this | outcome |
|---|---|---|
| **①** | ~~Fable consult on the HARNESS~~ | **✅ DONE** — verdict `needs-changes`; it *did* change what we built. Verbatim: `reviews/harness-design-fable-r1.md`. See §0a. |
| **②** | ~~Build the harness: A1 → A2 → A3, then B1/B2~~ | **✅ DONE** — `design/HARNESS.md` r2. ★ It fired on its own author twice: it blocked a `core.hooksPath --unset`, and it exposed that A4 had **never run** because `mkdir -p` in Bash bypassed the Write-tool hook. Both holes are closed (`scripts/hooks/`). |
| **③** | ~~Reconcile the archives~~ | **✅ DONE** — owner chose **hybrid**: storage differs by document kind, provenance does not. One manifest spans both trees (`xtask authority-manifest`). Residue **RESOLVED 2026-09-04** (7 → 0): `periodic/` retired, (B)'s five form copies deleted. The review-by tickle retired with its subject; the standing guard is `DUPLICATE_SOURCE_GROUPS = 0`, which reds on any duplicate with no date to renew. |
| **④** | ~~Fable consult on the PARSING STRATEGY~~ | **✅ DONE** — `reviews/label-reader-strategy-fable-r1.md`. ★ One cited measurement was **fabricated** (a phantom `f1_02` name gap); verified false, the *conclusion* kept on principle, the *evidence* discarded. |
| **⑤** | ~~The label reader~~ | **increment 1 BUILT** (`form_geometry.rs`, `label_reader.rs`); increment 2 redirected into the census. See §5. |
| **⑥** | ~~Fable consult on FIELD PROVENANCE~~ | **✅ DONE** — `reviews/field-provenance-fable-r1.md`, plus `shred-and-year-fable-r2.md` and `resumability-vs-discovery-opus-r1.md`. Built out into the §G-13 census, now complete. |

### ✅ What ⑥ became: the field-provenance census, COMPLETE

`crates/btctax-forms/tests/field_census.rs` asserts, for **all 15 forms** `fill_full_return` can emit,
that `(map FQNs ∪ [census] FQNs) == the PDF's AcroForm field set`, **exactly**. Every one of the 1158
fields now carries a determinate provenance: filled, or recorded as `unmodeled` / `artifact` / `gap`
**with a reason**. `CENSUS_NOT_YET_WRITTEN` ran 15 → 0 and its bound is now emptiness, so a 16th form
cannot arrive uncensused — mutation-verified in both directions.

**It found 16 gap fields / 8 unasked items — the table is in `FOLLOWUPS.md` §G-13.**

★★★ **One of them was wrong, and finding that out is the most valuable thing the census produced.**
Schedule C line G (material participation) was recorded as a gap that understated NIIT, with a
prescribed fix — routing a "No" into Form 8960 line 4a — that would have **double-taxed SE income**.
§1411(c)(6) already shelters it: btctax's Schedule C income is DERIVED as the SE base, so it is §1401(b)
income whether or not the filer materially participates. The error was reading Form 8960 line 4b's
printed **caption** instead of its instruction **body**, which names exactly that back-out. The repo had
already reasoned it out at `design/full-return/FOLLOWUPS.md:481-483` and it was not grepped for.
**A form's line title is not its instruction.**

★ **The method that worked, for the next form:** run the field probe, read each line's meaning from the
form's **extracted text** (never from position alone), verify every claim about btctax's behaviour in
**source** before recording it as a reason, and let the test — not the author — count the gaps.
★ **Two traps hit repeatedly:** `cargo fmt` reflows a shrinking list and silently breaks a string
replace (caught only by `the_two_lists_partition_every_form` and a fixed-size `[&str; 15]`), and
Form 8283's bundled asset is **Rev. 12-2023** while the archive holds **Rev. 12-2025** — different
sha256, so the archived extract is the *wrong revision* to transcribe from.

---

## 0a. ✅ FABLE CONSULT #1 — the harness — **DONE 2026-07-30**

**Verdict `needs-changes`.** Verbatim output: [`reviews/harness-design-fable-r1.md`](reviews/harness-design-fable-r1.md),
folded into `design/HARNESS.md` **r2**. Three load-bearing claims were independently verified against the
tree before folding (table at the end of that file). What it changed:

- **F1–F5 are TWO classes, not five** — **(α)** acted without observing an available fact (F1, F3);
  **(β)** shipped an instrument never seen discriminating (F2, F4, **F5**). The r1 five-mechanism list was
  the excuse-list mistake the document itself warns against. r2 is organised around the classes.
- **★★ A new top item: the harness-is-installed gate (A1).** `scripts/pre-push` is a reviewed, hardened
  hook, executable, in-repo since 2026-07-02 — `core.hooksPath` is **unset** and `.git/hooks/` holds only
  `*.sample`, so **it has never run**, and its install command is written in its own header. Without A1,
  H1/H3 repeat F4 on day one.
- **H3 as drafted provably would NOT have caught F1** — `design/forms/` is depth-2 under a `design/`
  dating to 2026-06-28, so a "new top-level path" trigger walks past. Split into a *deny* (shape-detector
  at `Write` time, now folded into A3) and an *ask* (any new directory at any depth, A4).
- **H4's lint DROPPED** → **B1 "seen-red-once"**: no checker exists until observed red on a planted
  defect. Covers F2+F4 as one class; cannot be satisfied performatively.
- **H5's lint DROPPED as having no target** (verified: `.slice(`/`.substring(` appear only in the two
  places *describing the lint*, zero in code) → **B2 pass-by-path payloads**.
- **Scope answers:** (c) keep memory as principles, wire trigger-shaped ones into hook messages; (d) **no**
  session-shaping — a checkpoint cadence is the forbidden self-verification scaffolding.

<details>
<summary>The original dispatch brief (kept for provenance)</summary>

★ **Ask the user's approval before dispatching.** Fable is escalation, never autonomous.

**Paste this to kick it off:**

> Consult Fable on `design/HARNESS.md` — the harness meant to stop me violating doctrine I have written
> down. Give it the full context from CONTINUITY.md §0a, one question only, and let it say the design is
> the wrong shape.

**The brief the dispatched agent must carry:**

- **THE CONTEXT — state all of it, it is what makes the question answerable.**
  - **The project.** `btctax` emits a complete US federal 1040 that a human signs under **26 USC §6065
    penalties of perjury**. A wrong number is the worst outcome; an **understatement** of tax is worse
    than an overstatement. The codebase is Rust, ~2450 tests, five validation gates, heavy use of
    mutation-verification ("a guarantee without a test that reds when it is removed does not exist").
  - **The problem.** The *assistant* (me) reliably writes down correct doctrine — in `CLAUDE.md` and in a
    persistent memory directory — and then violates it, sometimes **the same day**. The precipitating
    example: a memory was written saying *"before deriving or building, grep for what already exists — I
    conclude from not having looked"*, and hours later a primary-source archive was built from scratch
    without checking that `legal/primary-sources/` already held the same material.
  - **The diagnosis so far.** `CLAUDE.md` and memory are **passive context** — read at session start,
    violated 40 tool calls later while executing rather than reflecting. This is the same defect the
    codebase itself has been fixing all session: **held by convention, not construction.**
  - **The five OBSERVED failures**, from one session, all mechanically detectable — F1 built-without-
    checking; F2 enumerated from a range or hand-list instead of the source (**three separate times**);
    F3 committed with the gate red (ran it, never read the output); F4 claimed a checker worked while it
    was blind to the exact case it existed to protect; F5 truncated a payload between sub-agents and then
    reported the artifact as a finding. Details in `design/HARNESS.md`.
  - **The proposal.** H1 pre-commit hook running the gates · H2 a test forbidding two primary-source
    archives · H3 a `PreToolUse` hook on `Write` for new top-level paths (fires at the decision point) ·
    H4 a lint for enumeration-from-a-literal · H5 a workflow-script lint on `.slice()` into agent prompts.
  - **The available surface.** Claude Code hooks (`PreToolUse`/`PostToolUse`, currently unused in this
    repo), git hooks (none installed), the Rust test suite, `xtask` dev tooling, and `CLAUDE.md` itself.

- **THE ONE QUESTION.** Is `design/HARNESS.md` the right shape for making an assistant actually follow
  doctrine it has already written down — and what would make it materially better? Concretely: **which of
  H1-H5 will actually fire, which will be muted or routed around, and what is missing entirely?**

- **★★ SAYING "THIS IS THE WRONG SHAPE" IS A SUCCESSFUL CONSULT.** Nothing is built yet. If the whole
  approach is misconceived — if the failure is not addressable by mechanism at all, or if there is a
  categorically better lever (different memory structure, different session shape, different division of
  labour between assistant and tests) — **say so plainly now**, while it costs nothing. Do not soften it
  into "consider also…".

- **EXPLICITLY IN SCOPE — what we suspect but have not evaluated.**
  (a) **H4 is the highest-value and least likely to work** — F2 is a reasoning failure with only a faint
  syntactic shadow. Is there a better lever for "enumerated from the wrong source"? (b) Do hooks that
  merely *ask* (H3) change behaviour, or do they become noise that gets muted — and is there evidence
  either way? (c) Is the **memory system itself** mis-shaped for this: should doctrine be phrased as
  triggers ("when creating a new top-level directory, …") rather than principles? (d) Should any of this
  be **session-shaped** instead — a required opening action, a checkpoint cadence — rather than
  tool-shaped? (e) What does the **failure data** suggest that we have not noticed: are F1-F5 five
  problems or one?

- **FORBIDDEN.** Proposing more "be careful" instructions — that is exactly what already failed, and
  adding more is the null action in a costume. Proposing self-verification scaffolding ("add a final
  verification step") — the user's global config forbids it and it over-verifies with no quality gain.
  Proposing gates on *judgement* rather than on facts — they get routed around, which teaches that gates
  are routable. Re-auditing the tax logic, the spec, or the plan.

- **OUTPUT FORMAT.** `VERDICT: <sound | needs-changes | wrong-shape>`, then **per-item** `H1..H5:
  <keep | drop | change-to-X>` with one line of reasoning each, then `MISSING:` (up to three mechanisms
  we did not think of, most valuable first), then `WHAT WOULD MAKE THIS WRONG:` — one sentence naming the
  assumption the advice depends on.

★ **The measure of the harness is not that it exists.** It is whether a future session **fails a gate it
would otherwise have walked past.** Ask the reviewer to say which of its recommendations would actually
produce that, and which would merely look like rigour.

</details>

---

## 1. What this branch already did

- **★★ Fixed a LIVE DEFECT in shipped v0.14.0** — `FOLLOWUPS.md` **§G-9**. The §63(f) age-65 box on 1040
  line 12a was decided from the date of BIRTH alone, but i1040gi carves out a person who died in-year
  before reaching 65. A spouse who died at 64 got a $1,550 addition they were not entitled to —
  **understating tax on a signed return**, and invisible to both oracles (OTS takes a filer-answered
  `"You_65+Over?"` boolean; taxcalc has only `age_spouse`). Fixed with two class-(A) gates on
  `HouseholdHeader` plus two class-(B) dates on `Person`; **5 mutations killed**. Residue: **§G-9a**.
- **B1 + B2** (TY2025 groundwork) landed earlier: harness year seams, the `SaltLimitation` enum, per-year
  Form 6251 Part I, MAGI add-backs, 1040 L13b threaded, TY2025/TY2026 fail-closed gates.
- **T1 of B3** built: `Schedule1aParams`, `StepRounding`, `StairStepPhaseOut` in `tables.rs`. 8 tests,
  **8 mutations killed**.
- **Form-authority pipeline**: 66 documents archived as URL notes + 2.9 MiB committed text layer.

**Invariant held on every commit:** TY2024 provably unchanged — golden matrix md5
`c4e1853ed82d113ca5cd97ffd8abbf47` unmoved, both oracles exit 0, 2449 tests green.

---

## 2. Track A — Schedule 1-A (TY2025), branch B3

**Read:** `design/ty2025/SPEC_schedule_1a.md` (r3, 0C/0I) and
`design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md` (r3). Reviews in `design/ty2025/reviews/`.

- **T1 — DONE.** Rounding direction is a **parameter**: Parts II/III **floor** (lines 11/19), Part IV
  **ceils** (line 28) — statutory, because §163(h)(4)(B)(iii) says "or portion thereof". Three threshold
  pairs, three caps. `exhaustion_excess` is per-direction: Part IV exhausts at `threshold + $49,001`, not
  `+$50,000`.
- **T2-T7 — NOT started.**

★★ **Do NOT delete `ty2025_full_return_must_stay_fail_closed_until_complete`.** B3 satisfies its
**condition 4 only**. TY2025 `FullReturnParams` land LAST, after B4 — bundling early does not refuse, it
emits plausible wrong numbers.

### What the 13-agent provenance census found

(`design/ty2025/reviews/PROVENANCE_CENSUS_schedule_1a.md` — all re-verified against source)

1. **★★ Lines 5 and 14b have NO INPUT PATH.** `ReturnInputs` carries `w2s`, `int_1099`, `div_1099`,
   `g_1099` and nothing else, but both lines read from **1099-NEC / 1099-MISC / 1099-K**. They would be
   blank *because nothing can populate them* ⇒ they **REFUSE** (see §4.4 on why `0` is not an option).
2. **The line-5 ceiling is un-implementable as specified**, so it refuses rather than computing: it needs
   the deductible part of SE tax **plus** self-employed SEP/SIMPLE/qualified-plan contributions **plus**
   self-employed health insurance; printed Schedule 1 Part II carries lines 15/18/21 only.
3. **The four worksheets appear ZERO times in the FORM extract** — only in the *instructions* extract. A
   census driven off the form fixture alone could never red on a worksheet omission.
4. Worksheet arity comes from the **worksheet** (it prints four 1099 columns), not its narrative (which
   says overflow begins at "more than three").

---

## 3. Track B — the form-authority pipeline

`design/forms/README.md` is the entry point. **Three steps; a form is not done until step 3:**

| step | state |
|---|---|
| 1. **archived** — URL note + sha256 | ✅ 66 documents |
| 2. **extracted** — committed text layer | ✅ 57 documents, in `design/forms/extract/` |
| 3. **conformance-tested** — label census, decisions derived from each line | ❌ **Schedule 1-A only** |

**Acquisition is mechanical** — the point, because forms change every year:

- **annual**: `https://www.irs.gov/pub/irs-prior/{stem}--{year}.pdf`
- **periodic** (Forms 8275, 8283 — "Rev. Month Year", no tax-year edition):
  `https://www.irs.gov/pub/irs-pdf/{stem}.pdf`
- instructions are the **identically-numbered** `iNNNN`. `i1040gi` carries the 1040-family schedules that
  get no standalone booklet (Schedule 1-A, Schedules 2 and 3) and is the only one needing page ranges.

★ **PDFs are gitignored, not committed.** Each has a `<name>.pdf.txt` note with its URL + sha256; the
committed **text layer** is what tests read, so they need no PDF and no network. **A changed hash means
the IRS REVISED the document** — review it, never silently absorb it.

★ **A missing year can be correct:** `f1040s1a--2024.pdf` does not exist because OBBBA created
Schedule 1-A for TY2025. Not a fetch failure.

Tooling: `cargo run -p xtask -- cite-check` (34+ quotations verified verbatim; also prints authority
coverage) and `-- extract-schedule-1a`.

---

## 4. The doctrine established this session — READ BEFORE WRITING CODE

From the user; it now governs the work. Detail in `CLAUDE.md` and in memory
(`the-answer-is-in-the-manual`, `blank-is-the-normal-case`, `an-entry-is-testimony`).

1. **★★★ Taxes are simple instructions anyone can follow, and every form has an identically-numbered
   instructions document.** If implementing a line feels hard, **you have stopped reading and started
   inventing.** Difficulty is a signal to go back to the page, not to think harder.
2. **"§X disagrees with §Y" is a lookup, not a review finding.** A document need not agree with *itself*;
   it must agree with *the form*. Two sections that each match the manual cannot disagree.
3. **★★ Most fields on a tax return are BLANK, intentionally.** Never assert non-blankness — assert
   **provenance**: collected / computed from named lines / a constant the form prints / refusal.
   *"Usually zero"* is not a provenance.
4. **★★★ Every entry is TESTIMONY from the filer against the filer.** A blank is *no testimony*; a printed
   `0` is an affirmative sworn statement that the amount IS zero. Writing `0` on an unasked line
   **fabricates testimony under someone else's signature.** Whether a blank is lawful turns on **intent**,
   which is not software's domain — so btctax has exactly three lawful moves: **collect, refuse, or leave
   genuinely blank.** It must equally never build the opposite thing (a heuristic flagging an omission as
   suspicious). Both directions are software deciding intent.
   - ★ Sharper than "fail closed": **does the silence ASSERT, or FORGO?** Class (A) declarations assert ⇒
     must be answered or refuse. Class (B) benefit claims forgo ⇒ silence is lawful (*New Colonial Ice*).
     That is why §G-9's fix is legitimate: forgoing a deduction swears to nothing.
   - **Verified defect, §G-11:** `btctax-forms/src/lib.rs` `fmt_money(d: Usd) -> String` is the entire
     money path, so **no line can express blank**. Whole-surface; needs its own spec.
5. **Derive the decision FROM the line; don't check prose about it.** Rounding direction and
   cross-references are read off the printed text and asserted against the code
   (`tables.rs::schedule_1a_conformance`). This is how the Form 6251 line-33 class — "Subtract line 32
   from line **22**", once transcribed as line 12 and worth $200,000 on one vector — becomes a test.

★★★ **CORRECTION 2026-07-30 — THERE ARE FOUR ARCHIVES, NOT TWO.** Everything below this box was
written from memory; `cargo run -p xtask -- archive-check` (harness A3) walked the tree on its first
run and found two more that had never been named anywhere:

★ **Refined after a full walk (the first pass sampled 4 PDFs and generalised — F2, again).** They are
not four peer archives. They are **TWO CONVENTIONS, each with two layers**, plus one directory of
legacy strays:

| | binaries | text layer | provenance |
|---|---|---|---|
| **(A) `design/forms/`** | PDFs **gitignored**; each has a `.pdf.txt` URL + sha256 note | `design/forms/extract/` — **60 committed** extracts (what tests read) | hashes + `MANIFEST.json`, machine-checked by `xtask cite-check`; a changed hash means the IRS REVISED it |
| **(B) `legal/primary-sources/`** | **42 binaries COMMITTED** (was 47; −5 form PDFs retired to (A) 2026-09-04) | `legal/text/` — **20 committed** extracts (was 25) | `legal/SHA256SUMS` + `legal/SOURCES.md`; also covered by `MANIFEST.json` since the hybrid decision |
| strays | `design/amt-form6251/` — 8 duplicate notes, **2 unique** (`f6251--2026-DRAFT`) | — | older, terser note template |

★★ **The `.pdf.txt` files were never extracts** — they are provenance notes. They "diverged" only
because (A)'s template is richer (737 B vs 289 B). The real text layers are `design/forms/extract/`
and `legal/text/`.

★★ **So the reconciliation was ONE question, not four:** *commit the binary, or commit only its hash +
extract?* (A) keeps the repo small, makes an IRS revision detectable, and needs the network to
re-obtain. (B) is self-contained and offline, with no revision detection at all. **(B) holds material
(A) lacks — the statute and the regs — so neither tree can simply be deleted.**

### ✅ DECIDED 2026-07-30 — **hybrid**: storage differs by kind, provenance does not differ at all

Forms are re-fetchable from `irs.gov/pub/irs-prior` forever and are **revised annually**, so a hash is
exactly the alarm you want ⇒ note + sha256, binary gitignored. The statute and the regs are **law
as-of-a-date**, should be frozen in the repo, and their non-IRS URLs are less stable ⇒ committed. What
is now *identical* across both trees is the thing that was actually broken: a single manifest and a
single checker.

- **`cargo run -p xtask -- authority-manifest`** — **102 entries** (42 committed + 60 note-only;
  16 statute, 6 regulation, 28 instructions, 34 form, 12 guidance, 6 publication — measured
  2026-09-04, not recalled), each with kind, storage, sha256, URL and extract. `--regen`
  **derives** it from the trees — never hand-listed.
  ★★ **`--regen` REFUSES on a tree that is missing any listed document**, since 2026-09-04. It
  walks the filesystem, so on a fresh clone (where the 60 (A) PDFs are gitignored and unfetched)
  it would otherwise rewrite the manifest 102 → 42 with every instrument still green — measured.
  Fetch the PDFs from their notes first, or `git restore` a missing committed file. Plain
  `authority-manifest` with no flag is read-only and always safe.
- **Two directions, because one is not enough.** *verify*: every entry resolves and every committed
  file still hashes true (a changed hash means the source was **REVISED** — review, never absorb).
  *census*: every primary source in an accounted tree **is in the manifest** — the shape detector
  pointed inward, catching "archived but never recorded".
- ★★★ **`MANIFEST.json` already existed with 66 entries and NOTHING read it.** A manifest nobody
  checks is F4 in its purest form. It has a reader now.
- **113 of 113 URLs recorded — `URL_NOT_RECOVERABLE` is EMPTY.** Getting to 110 required parsing what
  the fetch scripts actually use (`declare -A` map + `for` loop); a naive parse got 87 and silently
  dropped **every rung that is law**. The last 3 — CCA 202302012 and 26 USC **§61** / **§1223** — were
  found by web search and then **verified by sha256 against the committed bytes** (all three
  byte-identical) before being written into `legal/_scripts/fetch_remainder.sh`.
  ★ **Verification is the point, not ceremony:** a URL that merely *looks* right asserts a provenance
  we have not established — the same sin as a paraphrase presented as a quotation.

### Countdown: **15 → 7** duplicate groups, and **4 → 3** archives (2026-07-30)

★ **A correction first.** "All 15 are `design/amt-form6251/` strays" was wrong — generalised from the
4 groups that happened to be sampled. **F2 again**, in the note describing the F2 detector. A full
walk showed only **8** were strays.

**DONE — the 8 are retired.** `design/amt-form6251/` is now **purely a design directory**
(`PLAN.md`, `PART_III.md`, `reviews/`, the vector generator) and is **off the archive list**.

- ★★ **`crates/xtask/src/cite_check.rs` read `design/amt-form6251/{form}--{year}.pdf` in LIVE CODE** —
  deleting first would have broken the fixture regenerator. Repointed to `design/forms/{year}/`, and
  the proof is that re-extraction reproduced both fixtures with **only the `# Source:` line changed**
  (same sha256, same text). That also repaired the two provenance lines without hand-editing either.
- The 2 unique files (`f6251--2026-DRAFT`) moved to `design/forms/2026/`.

**~~Remaining 7~~ — RESOLVED 2026-09-04, both groups, 7 → 0:**

| # | groups | resolution |
|---|---|---|
| **3** | `design/forms/{year}/{f8275,i8275,f8283}` == `design/forms/periodic/*` | **`periodic/` RETIRED.** No code resolved through it; its 3 notes cited a text layer that never existed (`extract/f8275.txt`; the file is `f8275--periodic.txt`); its URLs were the moving `irs-pdf/` ones. The 3 surviving year notes were round-tripped against `irs-prior/` first — HTTP 200, hash-exact, all three. |
| **4** | `design/forms/2025/*` == `legal/primary-sources/irs-forms/*` | **(B)'s five form copies DELETED** (905,833 bytes). `legal/SOURCES.md` keeps every citation, repointed at the surviving note + extract with the same hashes. `Form_1099-DA` / `Instructions_1099-DA` stayed — not duplicated. |

`DUPLICATE_SOURCE_GROUPS = 0` now pins it, and the test still reds in **both** directions. The dated
tickle was retired in the same commit — its subject is gone, and a pin at 0 is the stronger guard:
it reds the instant a duplicate appears, with no date for anyone to push out.

★ ~~`design/amt-form6251/` is a **design directory**, not an archive~~ — **done, see the countdown
above.** Original note: retire its form-notes, keep
`PLAN.md` / `PART_III.md` / `reviews/` / the vector generator, and repoint the provenance line in
`crates/btctax-core/src/tax/fixtures/schedule_1a_2025_form.txt`.

★ **That "two" was itself F2** — a count written from recollection instead of a walk, inside the very
note warning against enumerating from a hand-list. The number is now **measured and pinned**:
`archive_check::the_archive_count_may_only_shrink` reds if a fifth appears, and
`every_accounted_for_tree_still_exists` reds when one is retired, so step ③'s progress is a test result
rather than a claim.

★★ **RECONCILE THE ARCHIVES BEFORE THE LABEL-READER WORK.** Found at the very end of the session,
after `design/forms/` had already been built: **`legal/primary-sources/` already exists** and holds

    statute-irc/        16 × 26 USC sections (HTML)      ← rung 4, THE LAW
    regulations-cfr/     6 × 26 CFR regs (XML)           ← rung 3
    irs-guidance/       11 × Notices, CCAs
    irs-publications/    6 × Pubs
    irs-forms/           7 × forms + instructions        ← OVERLAPS design/forms/
    federal-register/    1 × TD 10000 (broker regs)

So the four-rung ladder in §5a is **already archived in this repo**, and I wrote that brief as though we
would have to go and find rungs 3-4. ★ This is the exact failure the `the-answer-is-in-the-manual` memory
describes — *concluding from not having looked* — committed on the same day I wrote it down.

**Two archives with different provenance conventions is the "which one is authoritative?" ambiguity this
session was spent eliminating.** Reconcile before building the label reader, since both would feed it:
`design/forms/` is URL-note + hash + extracted text, machine-checked by `xtask`; `legal/primary-sources/`
is committed binaries with no manifest. Decide one convention, and note that `irs-forms/` overlaps
`design/forms/` directly (Form 8949, Schedule D, Form 8283 and their instructions).

---

## 4a. ★★ §G-11 — the largest architectural open item, and what it does and does not block

**`FOLLOWUPS.md` §G-11.** `btctax-forms/src/lib.rs` — `fn fmt_money(d: Usd) -> String { d.to_string() }`
is the **entire** money path. Every money field on every emitted form is `Usd`, never `Option<Usd>`, so
**no line can express blank**; `Usd::ZERO` prints `"0"`. Zero-suppression exists only ad hoc and only for
whole *rows* (`schedule_d.rs`, `fill8949.rs`).

Under §4.4 that is not a formatting gap: writing `0` on a line the filer was never asked about
**fabricates sworn testimony under their signature**. It is invisible to every value-checking test and to
both oracles, because `0` is the correct *value* in the overwhelming majority of cases — **the defect is
in the act, not the arithmetic.**

**What it blocks — state this precisely, it was overstated once already:**

| | |
|---|---|
| **Constrains** | B3's emission choices. It is *why* T3a has lines 5 and 14b **refuse** rather than print `0` — refusing is the only lawful move left when the emitter cannot stay silent. |
| **Does NOT block** | the label reader (§5), the conformance census, or archiving/extraction. Those are independent. |
| **Blocks eventually** | any honest emission of a form with unasked lines — i.e. the whole surface, on a long enough horizon. |

**It needs its own spec, not a patch.** Sketch only: the emitter's money type grows a "not stated" state
that survives to the AcroForm write; computations may not manufacture a *stated* zero from *unstated*
inputs; and each line records which of the three lawful moves (collect / refuse / genuinely blank) it
takes, and why. The per-line decision then becomes a reviewable fact instead of an accident of
`Decimal::default()`.

★ **Scope bound, from §4.4 and easy to overshoot in both directions:** do not build a heuristic that flags
an omission as suspicious either. Intent is not software's domain, and *both* directions — assuming
silence, and policing it — are software deciding intent.

---

## 5. The label reader (track B, step 3) — ⑤ in the §0 order, AFTER the harness and the reconcile

**Read `design/forms/LABEL_READER.md` first.** Characterised and deliberately unbuilt: the obvious regex
gives **45** where Schedule 1-A's answer is **48**, and shipping a reader wrong on the one form whose
truth we know would manufacture exactly the false confidence the census exists to prevent.

**Three distinct sub-problems, not one to tune:**

1. **Whitespace** — lines 1 and 3 have *seven* spaces after the number where the pattern allowed six.
2. **Sub-letters have no parent** — `2b`-`2e`, `4a`-`4c`, `36b` appear as a bare `b`/`c` on their own
   line, so the reader is a small **state machine**, not a filter.
3. **Some numeric lines are HEADINGS** — lines 4, 14, 22 carry no amount box. `22a`/`22b` are a bare
   `a`/`b` with *nothing after them* and are missed entirely.

**And two of sixteen forms return ZERO** under the leading-number pattern: `f1040sa` and `f1040` put the
number in a second column beside a category label; `f8949` is a grid.

**Agreed design:**

- the reader **proposes**; a **human-established expected LIST** (not a count) is the authority;
- **zero labels is ALWAYS a hard failure**, whatever the layout;
- unanalysed layouts sit in a ratchet that **may only shrink**
  (`cite_check::AUTHORITY_NOT_YET_ARCHIVED` is the working model);
- ★ pin an observation **of the form** (which reds when the form changes), never the **reader's own
  output** (which would assert only that the reader still does what it did).

**Cost, honestly:** 16 forms × 2 years, each needing its label list read off the form once. That is the
same act as transcribing the form, done once and then held by a test forever.

---

## 5a. ★ FABLE CONSULT #2 — the parsing strategy (④ in the §0 order). Do it BEFORE paying the 32-list cost

**Why here and why Fable.** House rule (global `CLAUDE.md`): Fable is never the default and is reserved
for **a single review immediately before a first irreversible or costly action**. This qualifies on both
counts — the label-reader design fixes the shape of conformance for **16 forms × 2 years**, the cost it
gates is ~32 label lists read off forms by hand, and the failure mode is *false confidence* (a reader that
quietly finds 45 of 48 reports a form conformant by having nothing to check). Reviewing the strategy
before paying is far cheaper than discovering it is wrong on form 12.

★ **Ask the user's approval before dispatching** — Fable is escalation, never autonomous.

**Paste this to kick it off:**

> Consult Fable on the parsing strategy in `design/forms/LABEL_READER.md` before we build it. One
> question only: **is "reader proposes, human-established expected LIST is the authority" the right
> strategy, or is there a materially better one we are missing?** Give it the settled facts so it does
> not re-derive them, forbid a fresh audit, and make it answer in the fixed format.

**The brief the dispatched agent must carry** (sharp brief matters more than the tier):

- **THE ONE QUESTION.** Is the design in `design/forms/LABEL_READER.md` the best available strategy for
  enumerating a form's labels, given that the *purpose* is to distinguish *"this line encodes no
  decision"* from *"we forgot this line"*? If a materially better strategy exists, name it and say what
  it costs.

- **★★ A VERDICT OF "START OVER" IS EXPLICITLY WELCOME, AND THIS IS THE MOMENT FOR IT.** Say so plainly
  if the whole approach is wrong. We have paid for the archive and the extracts; we have NOT paid for the
  ~32 hand-read label lists or for 15 forms of transcription. **If we should begin again differently, the
  cheapest possible time to learn that is now** — do not soften it into "consider also…". A recommendation
  to discard `LABEL_READER.md` entirely counts as a successful consult, not a failed one.

- **★ WHAT WE ARE ACTUALLY DOING, so the strategy is judged against the real goal.** We are **filling out
  forms**, and the answers are written down for us in a four-rung ladder we may climb whenever a rung is
  silent:

  | rung | source | standing |
  |---|---|---|
  | 1 | **the form's own embedded instructions** — captions, cautions, "enter the smaller of", skip routing | guidance |
  | 2 | **the numbered instructions document** `iNNNN` (`i1040gi` for the 1040-family schedules) | guidance |
  | 3 | **the regulations** — 26 CFR | the agency's **interpretation** — binding in practice, **capable of being wrong** |
  | 4 | **the statute** — 26 USC | ★★ **the only rung that is LAW** |

  ★★ **Only rung 4 is law.** A Treasury regulation is the executive's reading of the statute; it is
  routinely held invalid for exceeding or contradicting it, the more so since *Loper Bright* ended
  deference. **If we believe the statute disagrees with a regulation, it is our duty to push back** — the
  tax system even supplies the instrument, **Form 8275-R** (Regulation Disclosure Statement), as distinct
  from Form 8275 for positions contrary to everything else.

  ★ **And the honest part: that duty is routinely neglected because challenging is expensive.** Say so
  rather than pretending otherwise — but do not let it silently become "the reg settles it". btctax
  emits Form 8275 and **not** 8275-R (`FOLLOWUPS.md` §G-12), so today it cannot *do* the duty at all: it
  can only agree with the regs, or take a contrary position undisclosed.

  ★★ **A believed statute/reg disagreement is now RECORDED AND TICKLED, not remembered.**
  `AUTHORITY_CONFLICTS.md` is the register; `cargo run -p xtask -- authority-conflicts` is the check, and
  an entry past its `review-by` **fails the test suite**. Neglecting the duty stays a legitimate choice
  (cost) — but it must be *a choice*, dated, with a review date, revisited. It can never again be an
  omission nobody decided. Mutation-verified: an overdue entry reds the suite.

  **We are never without an authority** — only ever without having gone and read it. Judge the strategy on
  how directly it gets us from "this line exists" to "this is what the form tells the filer to do", not on
  parsing elegance.

  ★★ **CORRECTED 2026-07-30 — all four rungs are ALREADY ARCHIVED. Do not treat finding them as work.**
  The brief above was written as though rungs 3-4 had to be sourced. They are in the repo and now
  machine-verified: `cargo run -p xtask -- authority-manifest` lists **105 entries** — 16 × 26 USC
  (rung 4), 6 × 26 CFR (rung 3), 29 instructions (rung 2), 40 forms (rung 1), plus guidance and pubs —
  each with kind, storage, sha256, URL and extract, and **every URL recorded** (`URL_NOT_RECOVERABLE`
  is empty). The reviewer should assume the ladder is *available*, and judge only how directly a
  strategy climbs it.
- **SETTLED — do not re-derive.** The measured layout data (leading-number works for 6 forms;
  `f1040sa`/`f1040` use a second column and return **0**; `f8949` is a grid); the three sub-problems
  (whitespace, parentless sub-letters, headings-with-no-amount-box); Schedule 1-A's truth is **48**; the
  extracts are committed and PDFs are not; `pdftotext -layout` for forms, plain for 3-column instructions.
- **EXPLICITLY IN SCOPE — the alternatives we did NOT evaluate**, and this is the real value of the
  consult: (a) reading the **AcroForm field names** from the fillable PDF instead of the text layer —
  btctax already has `xtask dump-fields`, and a fillable form's field list *is* an enumeration of its
  boxes, which may make the whole text-parsing problem the wrong problem; (b) `pdftotext -bbox` /
  coordinate-based column detection instead of whitespace heuristics; (c) the IRS **MeF XML schemas**,
  which enumerate every line as a typed element; (d) accepting per-form hand-written lists as the
  *primary* artifact with the reader used only as a change-detector.
- **FORBIDDEN.** Re-auditing the spec or plan; restating the transcription doctrine back to us; style,
  naming, prose. Do not propose "add more tests" without naming the specific defect it catches.
- **OUTPUT FORMAT.** `RECOMMENDATION: <keep | replace-with-X | hybrid | START-OVER-with-X>`, then at most
  five bullets of justification, then `COST DELTA:` versus the ~32 hand-read lists, then
  `WHAT WOULD MAKE THIS WRONG:` — one sentence naming the assumption its advice depends on. If the verdict
  is `START-OVER`, add `WHAT WE KEEP:` — the archive and extracts are paid for and should not be discarded
  by reflex.

★★★ **`dump-fields` WAS the lead most likely to change the answer — so it was MEASURED, not left as a
question.** Full data in `design/forms/LABEL_READER.md` §"MEASURED 2026-07-30". Summary for the brief:

- **The naive hope is FALSE.** Field names are sequential (`f1_01`…`f1_31`), and semantic naming is
  wildly inconsistent: Schedule 1-A names its line-22 table, f1040sa names 4 lines, f1040 names **1**,
  and **f6251 names ZERO**. A names-based strategy works on one form and collapses on the next.
- **But the GEOMETRY is universal and answers the question the text layer cannot.** An amount box is a
  field with coordinates; `pdftotext -bbox` gives every word coordinates; the two origins differ by a
  mechanical flip (~792 page height). Join on y and each row yields *its printed line number* **and**
  *whether it has an amount box* — which is sub-problem #3 (heading vs label) solved by construction.
  It also names 22a/22b outright, the case the regex misses entirely.
- **It is evidence, not an oracle:** the AcroForm enumerates **boxes**, the census asks about **lines**.
  Headings have no box but are still labels; one line can own several boxes. 54 fields ≠ 48 labels on
  the one form whose answer we know.

**So the live question is no longer "is there a better source?" but "what is the right ARBITER between
three imperfect signals — text layer, AcroForm geometry, and a hand-read list?"** That is what the
consult should answer.

---

## 6. Traps that have already cost time

- **`make check` ≠ CI.** Five gates: `make check` · `cargo fmt --all --check` ·
  `cargo +1.88 check --workspace --locked` · `cargo run -p xtask -- check-isolation` ·
  `bash scripts/pii-scan-generic.sh` (**scans HEAD — commit first**).
- **`.venv/bin/python`**, never bare `python3` (taxcalc/pandas live there). `sweep.py` needs
  `--seed N --count N`. `OTS_DIR=~/OpenTaxSolver2024_22.07_linux64`; OTS 2025 is at
  `~/OpenTaxSolver2025_23.06_linux64`.
- **`include_str!` must not escape its crate** — it ships a broken tarball with exit 0. Hence the
  Schedule 1-A fixture is in-crate while the other 57 extracts live in `design/`, read by `xtask`
  (`publish = false`).
- **A shrinking golden is a refusal, not a change.** Investigate before regenerating.
- **`rm -rf __pycache__` after restoring a mutated Python file** — a stale cache twice masked a restore.
- **One branch-mutating task in the shared tree at a time**; delegated agents must not spawn their own.
- ★ **Do not truncate large objects passed between workflow agents** — truncating classifications to
  14 KB made reviewers report labels as "omitted" that were merely unsent.

---

## 7. Open items

| id | what |
|---|---|
| **§G-11** | ★★ the emitter cannot express "no testimony" — **see §4a**; largest architectural item, needs its own spec |
| **§G-9a** | do the §63(f) **blind** boxes have a death interaction? |
| **§G-10** | residue: coverage — a checker that cannot tell "encodes no decision" from "we forgot this line" |
| §G-6c / §G-6d, E4-E6 | AMT Tier-2 items, parked behind the TY2025 pivot |
| B3 T2-T7, B4 | Schedule 1-A build; filing assets + corpus |
| — | **1 of 16 forms** has reached conformance (step 3 of the *form-authority* pipeline — distinct from the field census, which is complete) |
| **§G-13 gaps** | ★★ **16 gap fields / 8 unasked items** from the completed census — table in `FOLLOWUPS.md` §G-13. ★ an OPEN owner question could drop it to 12/6 — see the §G-13 note on disclosed pen-deferrals |
| — | Schedule C carries ONE aggregate `expenses: Usd`, so line 28 prints a total whose addends (lines 8–27b) are all blank — recorded, deliberately not counted as a gap |
| **§G-12** | btctax emits Form 8275 but **not 8275-R**, so a position contrary to a REGULATION cannot be disclosed — the duty is unrepresentable |
| — | `AUTHORITY_CONFLICTS.md` is empty: we believe no reg governing our forms disagrees with the statute. **A statement about what we examined, not a guarantee.** |
| — | **crates.io temp token still needs revoking** (from the v0.14.0 publish) |


</details>
