# CONTINUITY — bitcoin_tax (TaxApp)

_Last updated: **2026-09-09**. Written at a deliberate pause; safe to exit. **Read this file first.**_

---

# ★★★ RESUME POINT — the TY2025 push. Owner asleep; assistant proceeding autonomously (2026-09-04).

> ## ★★★ HANDOFF TO AN OPUS COORDINATOR (owner, 2026-09-07: "Let's find a time to switch to opus") — the loop from here is dispatch → machine-check → persist → ledger → fold → re-verify → push, and it needs no Fable: every remaining task (T8 in flight, T9–T12; T13/T14 on the owner's Q1; T15 post-v1) already has `BRIEF-build-interview-Tn.md`, `BRIEF-review-interview-Tn.md` and `BRIEF-reverify-interview-Tn.md` committed under `design/agent-reports/`. The rules that hold the loop: ONE opus builder or reviewer at a time (a sonnet verifier may run beside it, in a worktree); the builder edits the shared main tree and NOTHING is committed while it works (the pre-commit gate runs `make check` over the working tree); reviewers and verifiers run in `isolation: worktree` with `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` and their report is COPIED out, then `git worktree remove --force` + `git branch -D`; the report is persisted VERBATIM in its own commit before anything is folded; every measurable claim is machine-checked into `…-VERIFICATION.md` before acting; the fold is its own commit with the gate output in the message; push after each gate closes (`git push origin main`; the pre-push PII hook scans the range — synthetic identifiers only from the never-issued SSN space or `scripts/pii-scan-generic.sh`'s `ALLOWED_EIN`); commit messages via `git commit -q -F - <<'EOF'` with the two trailers. A stopped background agent is RESUMED by `SendMessage` with its id, never restarted while its edits are in the tree. The standing lessons every dispatch prompt repeats: no decision keys on a list typed beside derived data; a kill CALLS the instrument; a prompt hash keys on the registry's words, never display chrome; a new money leaf must reach the absolute chain (`every_money_leaf_household()`); a build's kills ask what the NEXT SURFACE does with what it wrote; a `Durable` fact is shown, never pre-filled; transcribe forms from the text layer. Owner-only items (never actioned autonomously): Q1/Q2/Q4, S1/S2/S7, T7's Notice 2026-20 order, the simulated real return (FR-64, the owner's TY2024 return is the reference and never enters the repo). The progress page is the artifact "Overnight Return" (scratchpad `overnight-return.html`; republish the same path to keep the URL).
>
> ## ★★★ RESUME 2026-09-09 — NEXT: **FR-110, FR-111, FR-114** (owner-chosen), then the B3 whole-branch review.
>
> **State:** interview arc BUILT (T1–T12 + T16, all 0C/0I). `make gate` **3558 passed / 12 skipped**;
> five instruments stable (375/18/31/0/17 · 274/13 · 8+4+6/91 · 90 · 268/19/9); everything pushed;
> `HEAD == origin/main`. **Both journey walks FILE a correct packet.** v1 task scope settled: T13
> DEFERRED, T14 CLOSED (not needed), T15 DEFERRED.
>
> ## ★★★ RESUME 2026-09-11 — **THE NEXT ACTION IS B3, AND EVERYTHING IT NEEDS IS ON DISK.**
>
> **State: clean.** `HEAD == origin/main == 3ae930ab`, tree clean, no worktrees, nothing in flight.
> `make gate` **3561 passed / 12 skipped**, `cargo fmt --all --check` clean. FR-110, FR-111 and FR-114
> are CLOSED and re-verified 0C/0I. New follow-ups filed: FR-115, FR-116, FR-117.
>
> ### The single next action
>
> **Read `design/agent-reports/PLAN-b3-whole-branch-review.md` IN FULL — including the ADDENDUM at the
> bottom (2026-09-11) — and fire B3 from it. Do not improvise the brief.** The plan gives the four
> seams, the shape (ONE opus reviewer, `isolation: worktree`, not a fan-out), the tier reasoning, and
> what would make the round a failure. The addendum gives what changed since it was written: the range
> tip, which commits are thinnest, what NOT to re-spend budget on, and the four-refuted-briefs warning
> that must go in the dispatch.
>
> Everything the coordinator held in context has been written into that addendum on purpose, because
> this session was cleared immediately after writing it. **Nothing is lost; read the addendum.**
>
> ★ Two things the addendum flags that are easy to miss:
> · **FR-110 is a DECISION, not a gap.** "btctax cannot file a 1099-B with adjustments" is the intended
>   permanent ceiling, owner-ruled. A finding to that effect is out of scope — say so in the brief.
> · **`LIMITATIONS.md` is `include_str!`'d into the binary** (`main.rs:582`). It is shipped filer-facing
>   text, not a design doc. `52b348c2` edited it; treat it as product surface.
>
> ### After B3 returns
>
> The standard loop, unchanged: persist the agent's report VERBATIM in its own commit → controller
> ledger machine-checking every measurable claim → fold brief → fold in a second commit with the gate
> output in the message → `make gate` + `cargo fmt --all --check` → push → sonnet re-verification.
> A worktree verifier's suite result is **not** comparable to the main tree's — corroborate any red in
> `main` before believing it (one reported 7 failures on 2026-09-09; all were environmental).
>
> **What closed:** FR-110 (aggregate 1099-B is the permanent ceiling — owner) · FR-111 (the Form 8949 box
> paragraph was **inverted**, not stale: it named TY2025's boxes in a TY2024-only document and denied the
> only pair TY2024 prints) · FR-114 (R15 now scans authored labels; transcribed captions exempt by
> provenance, `LabelSource` held by the compiler). **New:** FR-115, FR-116, FR-117.
>
> **Both folds RE-VERIFIED 0C/0I** by a sonnet agent in a worktree (`REVERIFY-fr111-fr114.md`, persisted
> `fd6adda8`): every `Field` literal in the whole workspace sets `label_source`, the `E0063` mechanism is
> real, classification correct at 15+ spot-checked sites, the three-way equality guard mutation-tested
> red, and nothing false found in the new filer-facing prose. Its one Minor is FR-117. ★ Its `make gate`
> showed 7 failures — all worktree-environment artifacts (gitignored `design/forms/2026/` PDF fixtures
> are not shared across worktrees, plus the mandated `CARGO_TARGET_DIR` tripping a hook-lookup test).
> **A worktree verifier's suite result is not comparable to the main tree's** — corroborate in `main`
> before believing a red, as was done here.
>
> ★★★ **THE LESSON OF THIS SESSION, and it cost four refuted premises to learn:** *a hand-written scan
> over one syntactic form is not a measurement of a set produced by another.* FR-114's author measured
> `"Covered lots"`; the controller measured `label:` literals; both were careful, both were wrong, and
> the second was committed **in the brief written to prevent the first**. Each time the implementer
> **stopped and reported instead of building on it** — which is the only reason none of them shipped.
> Keep telling every agent to do exactly that. Full write-up in `FOLLOWUPS.md` FR-99's table.
>
> ---
>
> **★ (superseded by the block above) TWO OF THE THREE DECISIONS ARE MADE (owner, 2026-09-09).**
> 1. **FR-110 — ✅ DECIDED: the aggregate 1099-B is the PERMANENT CEILING.** A row with any adjustment
>    refuses (`Form1099BNeedsForm8949`) and that is final — no per-lot securities path, do not re-file
>    it as a defect. The owner asked whether §1091 changes it; it does not (wash sale reaches *"stock
>    or securities"* only, never bitcoin — already encoded at `forms.rs:380`, `optimize.rs:7-15` and
>    `tests/optimize_wash_sale.rs`), and the reasoning is written up in the FR-110 entry. Remaining
>    work is DOCUMENTARY and folds into FR-111.
> 2. **FR-111 — UNBLOCKED, in progress.** `LIMITATIONS.md:415-417` is false and must be RE-DERIVED, not
>    reworded. ★ Do not fix only that sentence — the sweep already found the document **contradicting
>    itself**: `LIMITATIONS.md:281-283` ("Form 1099-DA answers are a keystroke… There is no 1099-DA
>    entry screen") is CORRECT and refutes line 416 three sections earlier. A sonnet recon is deriving
>    the full box truth table and every stale filer-facing claim into
>    `design/agent-reports/RECON-fr111-8949-box-truth.md`. Ground already established: a `[[b_1099]]`
>    row NEVER becomes a Form 8949 row (it is the Schedule D line 1a/8a summary); 8949 rows come from
>    the crypto ledger only; `route_8949_boxes` (`forms.rs:139`) then re-routes them from the filer's
>    1099-DA answers, so **G/H/J/K are reachable** and "Never Box C/F" is false pre-TY2025 anyway.
> 3. **FR-114 — ✅ DECIDED: scan `Field.label` ONLY**, with the boundary stated in the source and the
>    residue named (a ledger question inside `help` is not caught). ★★★ **Its stated premise was
>    REFUTED by measurement and is retracted** — *"Covered lots"* / *"Noncovered lots"* do **not** red,
>    because the checker matches whole words and those say `lots`. The three strings that do red are
>    all in `help`, all explanatory, none a question; the *"widening would break IRS vocabulary"*
>    worry never existed. One of them (`sections.rs:2937`, *"its own crypto lot engine"*) is a genuine
>    UX leak of a different kind and was split out as **FR-115**.
>
> **Then B3.** The plan is written and committed: `design/agent-reports/PLAN-b3-whole-branch-review.md`
> (`bbbd6bc8`) — range `121c8805..HEAD`, ONE opus reviewer in a worktree, four seams, with what the
> earlier passes already covered so budget goes to the seams. Its preconditions are now MET (walk 2 and
> the sweep are filed). Fire it from that plan; do not improvise the brief.
>
> **Also open (no owner action needed to start):** FR-112/113 (labels, 8949 page order), FR-98 (R15's
> hand-picked source lists), FR-100/101, FR-103's residue. Owner questions still open: Q1's 1095-A row
> (the only row that refuses the WHOLE return), Q4 (deposit vs check), S1/S2/S7.
>
> **Doctrine adopted this session (owner-approved):** HARNESS `B1a` (the fixture is half the checker),
> `make gate` (FR-90, touch-then-check — note it does NOT run `cargo fmt`, and the pre-commit hook
> blocks on that), and `CLAUDE.md`'s *"Derive the list, or make the compiler hold it"* (FR-99).
>
> ★ Two controller briefs were refuted by measurement this session (FR-105's and FR-108's stated
> mechanisms). Both times the implementer stopped instead of building on them. **Keep telling every
> agent to stop and report rather than implement from a premise it can disprove.**
>
> ## ★★★ (superseded) RESUME 2026-09-07 — **THE INTERVIEW ARC IS BUILT. T1–T12 AND T16 ALL CLOSED AT 0C/0I. THE NEXT GATE IS THE OWNER'S.** — Everything is committed and PUSHED to `origin/main` (`bdbee23e`); `make check` 3537 passed / 12 skipped; `make docs` clean, no man-page diff; instruments `line-coverage` 375/18/31 (ratchet 31)/0/17, `census-join` 274 across 13 maps, `stop-list` 8+4+6 renderer sources / 91 prompts, `prompt-check` 88, `box-census` 268/19/9. Every task ran the full loop: build → machine-check → commit+push → ONE opus seam review in a worktree → persist VERBATIM → controller ledger → fold → commit+push → ONE sonnet re-verification → close. T12's four owned follow-ups (FR-73/83/86/87) were burned down IN the task that owned them, and five stale entries (FR-67/68/70/86/87) were reconciled and marked closed with evidence.
>
> **★★ DO NOT START THE NEXT PHASE AUTONOMOUSLY.** The work below the interview is owner-gated. What is open, in the order it matters:
>
> 1. **The B3 whole-branch review has NOT been run, and it is the real remaining gate.** Every review so far was scoped to ONE task. `design/HARNESS.md` B3 is explicit that a stack of range-scoped reviews does not add up to a branch review, and the precedent is exact: three range-scoped reviews each returned 0C/0I and the first whole-branch pass found an Important in the EARLIEST commit, outside every earlier window. This arc has **seven** cross-task defects already on record where a later task widened a set beneath an earlier one (FR-99's table), which is precisely the class a per-task review cannot see. ★ Note the scoping wrinkle: this work went **straight onto `main`** and was pushed at every gate, so there is no branch to diff — the range is the interview arc's commits on `main` (`121c8805..HEAD`), and the review must be pointed at **INTERACTION**, not correctness-per-commit.
> 2. **✅ FR-97 is CLOSED (`f88838f6`)** — it was owed before that review and is done. Was: — the TUI writes a dependent-gate answer with NO `AnswerRecord` (`apply.rs::answer_key_for` has zero `DepGate` arms while `seam.rs` has twenty), so R10.3's re-ask rule is disabled on that surface while `income answer` records it correctly. Two writers, one guarantee, and they disagree.
> 3. **Three doctrine items are FILED AND NOT ACTIONED, because doctrine is the owner's call:** **FR-99** (the arc's dominant defect class — a hand-written list beside a set that grows; seven instances tabulated; proposes making the existing data rule structural), **FR-88** (the same disease on the test side — a derived checker fed a hand-written fixture; three consecutive tasks), **FR-90** (a stale `target/` can make the gate report a false result; the plant→measure→restore loop B1 mandates is what races cargo's mtime staleness; mitigation is `touch` after every restore and a full `.rs` touch before any closing gate — every gate from `08c09ecc` on was taken that way).
> 4. **Owner questions still open, never actioned autonomously:** Q1, Q4 — ★ **Q2 ANSWERED 2026-09-07: accept the line-19 forgo, T15 stays post-v1** (the owner's income is well above the §24(b) phaseout — $400,000 MFJ / $200,000 otherwise — so the CTC and the $500 ODC are both zero on the merits and the forgo costs nothing; `ctc_provably_zero` fires, so line 19 prints a sworn `0`, which is the correct testimony rather than a forgo. FR-85 stays open: it is a margin defect for any filer, just not this one.) — (deposit vs paper check — T10 built the field so either answer is expressible and decided nothing); S1, S2, S7; T7's Notice 2026-20 order; the simulated real return (FR-64 — the owner's TY2024 return is the reference and never enters the repo).
> 5. Nothing here is released. Publishing crates is an irreversible, owner-authorized action and no gate for it has been opened.
>
> The rules that held this loop are in the HANDOFF block above and still apply. Every report is on disk under `design/agent-reports/` — `git show <persist-commit>` for what a reviewer found verbatim, `git diff <persist>..<fold>` for exactly what changed in response, and the gate output in each fold commit's message.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T11 CLOSED 0C/0I; **T12 IS THE LAST BUILD TASK** — PUSHED) — T11 (the oracle path, R13) built `46d4b2d6`, seam review **2C/4I/3M/1N** (`f6dc308e`, the heaviest of the arc), ledger `3e82e852`, fold `ebe5bbec` (3510 green, +4), re-verified **0C/0I/0M/0N** with every build and fold kill replanted live. ★ **C-1 — the verification instrument answered for the filer and then reported success.** `check_return.py` is documented (and `cli.rs:512` says) *"it is how a REAL return reaches an oracle"*, but it handed the harness the PROJECTED ROW and `build_golden_return` rebuilt a household from it, setting `foreign_accounts`/`foreign_trust = Some(false)` and calling `answer_all_live_declarations` — so a return `btctax report` REFUSES reconciled at exit 0, and the docstring's own *"Exit 2 = a refused return"* could never fire. **Exit 2 did not exist at all**; `income project` now runs the filer's own refuse chain and emits a `refused` block. ★ **C-2 — no projected row could ever drive OTS's Schedule A**: `ots_direct.py:573,:687` gate every Schedule A line on `standard_or_itemized == "Itemized"`, `corpus.py:164` calls it *"not a GoldenInputs field"*, and the struct had ZERO occurrences — so every itemizing filer got a false DIVERGES, exit 1, and silently lost oracle 1 on four boxes while the census claimed one engine modelled them. ★★ **C-2 and I-1 shared ONE root — the completeness partition was derived over `Usd` LEAVES, blind to facts that ROUTE money.** The fold built a second DERIVED routing partition over 717 type-derived perturbations of every non-`Usd` leaf, and it immediately uncovered **four latent money defects both oracles were blind to** (the wrong figure went to both): Sch A line 5a omitted W-2 box 17+19; line 8a omitted Form 1098 points and ignored the mixed-use §163(h)(3)(F) zeroing; `hsa_deduction` ignored `sch1.hsa_activity`. Fixed by reading the FILED `ScheduleAParts` instead of re-deriving it — the same lesson as T9's C-1. ★★ **I-2 closed the second half of a hole neither task owned:** the excuse collapsed a blank into a sworn `0`, so when TY2025's package lands and FR-85's stale $2,000 ceiling makes `ctc_provably_zero` swear a `0` on line 19, the ONE instrument that could catch it would have excused it. Now cross-referenced to FR-85 in the source and pinned in `--selftest`. I-3 (an unnamed single-witness line is now an error), I-4 (all five statuses `_`-free — T8 had widened the set and T11's hand-typed match had not followed). Both oracle baselines hold: 107 households regenerate byte-identically from both engines. FR-91..96 filed. ★ NEXT: **T12 — the LAST build task** (`BRIEF-build-interview-T12.md`; ONE opus agent, main tree, nothing committed while it works; tell it about FR-90 and that its briefs' settled-facts blocks are ~60 commits stale) → machine-check → commit → PUSH → `BRIEF-review-interview-T12.md` (opus, worktree) → persist → ledger → fold → PUSH → sonnet `BRIEF-reverify-interview-T12.md` → T12 closes. **T12 also OWNS the burndown of FR-86 and FR-87** (per those entries). After T12 the interview arc is built and the next gate is a whole-branch B3 review scoped to `main..HEAD` and pointed at INTERACTION, not correctness-per-commit. Read the HANDOFF block above for the rules.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T10 CLOSED 0C/0I; T11 NEXT — PUSHED) — T10 (the trailer, §5.4) built `f8768e93`, seam review **0C/3I/4M/1N** (`586d97ce`), ledger `f61a7ebc`, fold `08c09ecc` (3496 green, +5), re-verified **0C/0I/0M/0N** with all 21 build kills and every fold kill replanted today. ★ **I-1 was the important one, and it closed a CLASS.** T10 taught `open_next_year::seed` to carry the phone and all three foreign-address leaves and named none of them in `CARRIED_IDENTITY`, so `Opened::render()` printed *"Everything else is blank"* while four leaves crossed — and `foreign_address_is_live()` reads ONLY `foreign_country`, so a filer moving from abroad to the US printed LAST YEAR'S FOREIGN ADDRESS on a domestic return. The guard built for exactly this was green because its hand-written fixture never populated the leaves it walks; it is now fed a fixture DERIVED from `scrub_axis::maximal_sentinel` (compiler-enforced) plus a floor calling `seed` directly, and switching that on surfaced **four more unnamed carries from T7/T9/T16** and a twice-stale `--help`/man page. I-2: `DepositAccountKind` had no unanswered state and `create` started it at `Checking`, so a savings filer who typed the two numbers and never opened the type row filed a Checking mark they never made (the instruction: *"You must check the correct box to ensure your deposit is accepted"*) — `kind` is now `Option`, `create` leaves it `None`, and `screen_direct_deposit` refuses on it. I-3: the ABA rule's justification was false in all three claims; it now says what the code does and states that the rule BLOCKS a return, so it cannot license btctax-only validity rules elsewhere. ★★ **FR-88 (owner's call, filed not actioned): three consecutive tasks — T8 I-1, T9 C-1, T10 I-1 — were a correctly-designed guard blinded by a hand-written FIXTURE. B1 requires seen-red-once; it does NOT require the fixture to exercise what the checker walks.** A B1 amendment is proposed in the FOLLOWUPS entry. ★★ **FR-90: a stale `target/` can make the gate report a FALSE result** — a reverted plant stayed compiled into the `btctax-core` rlib and produced a phantom red at a clean HEAD. The plant → measure → restore loop B1 mandates is what races cargo's mtime staleness, so EVERY agent following B1 is exposed; it bit as a false RED, nothing prevents the false GREEN. Mitigation (verified): `touch` after every restore, and `find crates -name '*.rs' -exec touch {} +` before any closing gate — every gate from `08c09ecc` on was taken that way. FR-89 filed (T10 secret handling, never gating). ★ Controller error of record: `08c09ecc`'s message claims FR-90 was filed; the scripted insert had failed its anchor assertion and the entry landed one commit later in `eefc50f8`, whose message carries the correction. ★ NEXT, in order: dispatch **T11** (`BRIEF-build-interview-T11.md`; ONE opus agent, main tree, nothing committed while it works — and tell it about FR-90) → machine-check → commit → PUSH → `BRIEF-review-interview-T11.md` (opus, `isolation: worktree`) → copy out → remove worktree + branch → persist VERBATIM → ledger → write `BRIEF-fold-interview-T11-review.md` → fold (fresh opus) → commit + PUSH → sonnet `BRIEF-reverify-interview-T11.md` → T11 closes → then **T12**, the last. Read the HANDOFF block above for the rules.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T9 CLOSED 0C/0I; T10 NEXT — PUSHED) — T9 (real estate, R8) built `18935bfd`, its seam review returned **1C/1I/2M/1N** (`7ef1c36b`), the ledger machine-checked every claim (`39b73ca7`), the fold landed `0f6603a9` (3475 green, +5) and the sonnet re-verification came back **0C/0I/0M/0N**. ★ The Critical was the brief's own one question answered YES: the three rules T9 added dropped the itemize-election conjunct the three older mortgage declarations carry, so a STANDARD-DEDUCTION filer holding a Form 1098 was asked the Form 8396 question and refused by `SharedMortgageInterestUnanswered` / `SharedMortgageInterest` — against SPEC J-24's written *"nothing is transcribed and nothing refuses"*, and reachable with NO filer action because `open_next_year` seeds a prior lender onto a year carrying no `schedule_a` (grep: zero occurrences). The doc comment excusing the omission (*"the 1098 SECTION is already gated on the itemize election"*) was false — `section_is_live` has no `Form1098s` arm and falls through `_ => true`. ★★ And the shipped test named for that guarantee COULD NOT RED: it enumerated only the three older declarations and its fixture pre-answered both new gates — the same shadow shape as T8's I-1 one task earlier. The fold went STRUCTURAL: `ReturnInputs::form_1098_deducted()` returns the rows only on an itemizing return and is read at 8 call sites (the three declarations, the 8396 gate, the shared-interest refusal, the ceiling warning), so the next such rule cannot forget the conjunct; box 4 stays ungated on purpose (that refund is Schedule 1 line 8z income either way). Controller's re-probe: `live(ClaimingMortgageInterestCredit) = false`, both shared-interest states `None`, and an itemizing return still refuses `MixedUseMortgageUnanswered` — gated, not silenced. I-1: line 8b printed *"See attached"* for a statement nothing produced, asked for or named (`hand_marks` had 0 mentions; TY2025's merged 24pt box overflows at TWO recipients) — now one shared `schedule_a_line8b_overflow` condition read by the emitter AND a conditioned manifest entry. M-2 was resolved by WIDENING the home-sale cross to all 24 combinations rather than narrowing the prose. SPEC R8 no longer contradicts itself (section top-level; the election gates the census row and every line-8 consequence). ★ FR-87 filed from the fold's own disclosure: the home-sale rule's `s_1099 == Some(true)` arm is UNREACHABLE (the census screens first on both tiers; planting `false &&` reds nothing) — kept as a documented fail-closed backstop, owned by T12 alongside FR-86's B1 sweep. ★ NEXT, in order: dispatch **T10** (`BRIEF-build-interview-T10.md`; ONE opus agent, main tree, nothing committed while it works) → machine-check → commit → PUSH → `BRIEF-review-interview-T10.md` (opus, `isolation: worktree`) → copy out → remove worktree + branch → persist VERBATIM → ledger → write `BRIEF-fold-interview-T10-review.md` → fold (fresh opus) → commit + PUSH → sonnet `BRIEF-reverify-interview-T10.md` → T10 closes → then T11, T12. Read the HANDOFF block above for the rules.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T8 CLOSED 0C/0I; T9 NEXT — PUSHED) — T8's seam review returned 0C/3I/2M/1N (`4baf159c`), every claim was machine-checked into `2026-09-07-build-interview-T8-review-VERIFICATION.md` (ledger `53dbe680`), the fold landed `a17b9d6a` (3455 green, +6) and the sonnet re-verification came back **0C/0I/0M/0N**. The three Importants: **I-1** the dependents grid projected from the RAW LEAF, so a filer who answered (5)(a) Yes + (5)(b) Yes and then flipped (5)(a) to No printed *"(b) And in the U.S."* checked under an unchecked *"(a) Yes"* — invisible in the form (`get` → `None`) and unclearable (`clear` → `NoSuchRow`); `ReturnHeader::build` now takes ONE `walk_dependent` per row and projects every row-(5)/(6) box as `walk.demands(gate) && leaf == Some(true)`, all four gates gated. **I-2** the FR-85 CTC pin could not red on the defect it documented — it read a core-local `ty2024_params()` fixture and `BundledFullReturnTables` has zero occurrences in btctax-core, so bundling TY2026 left it GREEN while the stale $2,000 ceiling makes `ctc_provably_zero` swear a `0` on line 19 for a household with credit left; the pin now lives in btctax-adapters and derives its year set through the file's fail-closed `derive_shipped_years`. `CTC_PER_CHILD_SS24H2` stays 2000 — the fix WHEN TY2025's package lands is to thread the params into `ctc_odc_line19`. **I-3** `qualifying_child_name` was collected, classified, scrubbed and helped with NO READER on any year, and was not live on QSS though the form's sentence names it; it now prints on `HoH | Qss` through one predicate the seam and the emitter both read. Plus M-1 (a determinism tautology renamed), M-2 (`prompt-check` read only quoted spans — the operative clause is now quoted too, 86 → 88 assertions) and N-1 (the caption check gained ORDER). ★ Measured for TY2025 work and filed under FR-84: **TY2025 SPLIT the shared entry space into two cells** — `Checkbox_ReadOrder[0].f1_28[0]` (180.0, 540.0)–(324.0, 552.0) for the MFS spouse name and `f1_29[0]` (362.0, 534.0)–(576.0, 546.0) for the HOH/QSS child — so a TY2025 `[header]` map needs two keys where `Form1040Map` has one. FR-86 filed (six of T8's "other standing kills" have never been watched red; owning task T12). ★ NEXT, in order: dispatch **T9** (`BRIEF-build-interview-T9.md`, real estate; ONE opus agent, main tree, nothing committed while it works) → machine-check → commit → PUSH → `BRIEF-review-interview-T9.md` (opus, `isolation: worktree`) → copy out → remove worktree + branch → persist VERBATIM → ledger → `BRIEF-fold-interview-T9-review.md` → fold (fresh opus) → commit + PUSH → sonnet `BRIEF-reverify-interview-T9.md` → T9 closes → then T10, T11, T12. Read the HANDOFF block above for the rules.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T7 CLOSED 0C/0I; T8 BUILT — SESSION CLEARED HERE; the owner switched to Opus) — T7 re-verified 0C/0I/0M/0N. T8 (row (7) computed off T7's verdict, the TY2025 dependents grid mapped by a new `xtask dependents-grid` derivation — register 196 → 171, Δ = exactly the 25 cells mapped — the emitter filling rows (5)–(7) with TY2024 byte-identical, HoH/QSS/FR-67 as eleven registry questions, the line-19 forgo sized) is BUILT and COMMITTED; 3449 green. FR-84 (rows (1)–(4) unmapped: the form asks two name cells, `Dependent` holds one) and FR-85 (the CTC is $2,200 from TY2025 by OBBBA §70104(f) — the controller's brief had the wrong year; the literal is now pinned to red when TY2025's package lands) filed. ★ NEXT, in order: dispatch `BRIEF-review-interview-T8.md` (ONE opus reviewer, `isolation: worktree`, at HEAD) → copy its report out → remove the worktree + branch → persist VERBATIM in its own commit → machine-check into `…T8-review-VERIFICATION.md` → write `BRIEF-fold-interview-T8-review.md` → fold (fresh opus, main tree) → commit + PUSH → sonnet re-verify with the committed `BRIEF-reverify-interview-T8.md` → T8 closes at 0C/0I → then **T9** (`BRIEF-build-interview-T9.md`, real estate; its review and re-verify briefs are committed), then T10, T11, T12. Read the HANDOFF block above for the rules.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T7 FOLDED `2213eeb5` / re-verifying; T8 BUILDING; PUSHED `2213eeb5`) — T7's fold landed and is pushed (3429 green: the gates hash the registry's words and show the banner; the KAT drives the real `income answer`; a blank dependent SSN blocks before any gate and a duplicate refuses on both tiers via the always-on `screen_dependent_values`; the seeded DOB shown, never pre-filled; the help rewritten; three truth-table rows; scrub's synthetic dependent DOB). IN FLIGHT: the sonnet re-verification of T7 (`BRIEF-reverify-interview-T7.md`, worktree at `2213eeb5`) and the **T8 build** (row (7), HoH/QSS + FR-67, the TY2025 grid; opus, main tree, `BRIEF-build-interview-T8.md`). When the verifier returns: copy out, remove the worktree + branch, persist (commit only when T8's tree is committed) → close T7 at 0C/0I (roadmap + here) or fold residue after T8. When T8 returns: machine-check (the `UNCENSUSED` register's `f1040` count falling by exactly the mapped cells; TY2024 byte-identical; the HoH/QSS refusals; the FR-67 gate), commit, PUSH, dispatch `BRIEF-review-interview-T8.md` (opus, worktree) → persist → ledger → fold → sonnet (`BRIEF-reverify-interview-T8.md`) → **T9** (`BRIEF-build-interview-T9.md`; review + reverify briefs committed). Push after each gate closes.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T7 FOLDING) — T7's seam review (1C/5I/5M/3N, `4a849401`; ledger committed next) is being FOLDED by a fresh opus agent under `BRIEF-fold-interview-T7-review.md`: C-1 — `income answer` hashed a banner-prefixed prompt while `current_prompt` resolves the bare words, so every dependent gate read `WordingChanged` on the spot (a brick; the sweep KAT was green because its emulation skipped `record_answer` — I-1); I-2 decided (a blank SSN blocks before any gate, a duplicate refuses at import too, the session `asked` set keyed by `(row, gate)`); I-3 the seeded dependent DOB shown, never pre-filled (T4b's rule); I-4 the shipped help rewritten; I-5 three truth-table rows; M-1…M-5 (incl. scrub's synthetic dependent DOB), N-1…N-3. When the fold returns: machine-check (the real-command KAT green with `screen_inputs` clean; the banner replanted → red; the two-row probe), commit through the gate, PUSH, dispatch the sonnet re-verification (`BRIEF-reverify-interview-T7.md`) AND **T8** (`BRIEF-build-interview-T8.md`; review + reverify briefs committed) → T7 closes on the verifier's 0C/0I. Push after each gate closes.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T16 CLOSED 0C/0I; T7 BUILT `282a8a32` / seam review IN FLIGHT; PUSHED `8569b450`) — T16 re-verified 0C/0I/0M/0N with both oracles re-run live (`00ae04ac`): every form on the owner's filed TY2024 return is now buildable. T7 (the dependents gates) landed `282a8a32` (3416 green; twenty per-row gates with liveness derived from the flowchart walk; the required DOB; `DEPENDENT_GATES` + `DependentVerdict` for T8; the §152(d) figure; FR-70's seeded dependent blocking; eleven deviations incl. `scrub` keeping a dependent's DOB and the age test computed in T7; FR-81/82/83 filed). Its opus seam review (`BRIEF-review-interview-T7.md`, worktree at `8569b450`) is IN FLIGHT. When it returns: copy out, remove the worktree + branch, persist → ledger (`…T7-review-VERIFICATION.md`) → fold (fresh opus) → commit → PUSH → sonnet re-verify (`BRIEF-reverify-interview-T7.md`) AND dispatch **T8** (`BRIEF-build-interview-T8.md`; its review + reverify briefs committed) → T7 closes on the verifier's 0C/0I. Push after each gate closes. Owner questions Q1 (partly answered by the filed return), Q2, Q4 still open.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T16 FOLDED `79e3bb6e` / re-verifying; T7 BUILDING; PUSHED `8d2f6b83`) — T16's fold landed and is pushed (3397 green: both Form 8889 legs in the absolute chain; the two-chain equality instrument STRUCTURAL over `every_money_leaf_household()` derived from `maximal_sentinel` + `leaf_walk::money_leaves` — a brand-new money leaf wired into the printed chain only reds with no fixture added; the §221 MAGI gains line 13; the spouse's HDHP plan collected; the code-W rule on `Some(false)`). IN FLIGHT: the sonnet re-verification of T16 (`BRIEF-reverify-interview-T16.md`, worktree at `8d2f6b83`) and the **T7 build** (the dependents gates, opus, main tree, `BRIEF-build-interview-T7.md`). When the verifier returns: copy out, remove the worktree + branch, persist (commit only when T7's tree is committed) → close T16 at 0C/0I (roadmap + here) or fold residue after T7. When T7 returns: machine-check (the truth-table KAT's rows; the required DOB; the waiting §152(d) gate; FR-70's seeded dependent listed in `interview_state`), commit, PUSH, dispatch `BRIEF-review-interview-T7.md` (opus, worktree) → persist → ledger → fold → sonnet (`BRIEF-reverify-interview-T7.md`) → **T8** (`BRIEF-build-interview-T8.md`; review + reverify briefs committed). Push after each gate closes.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T16 FOLDING; PUSHED `b9c21f8f`) — T16's seam review (1C/3I/2M/1N, `adcf4837`; ledger `b9c21f8f`) is being FOLDED by a fresh opus agent under `BRIEF-fold-interview-T16-review.md`: C-1/I-1 — Form 8889's income leg (line 8f) and additional-tax leg (17c/17d) reached the PRINTED chain only, never `AbsoluteReturn` (AGI short by the taxable distribution; a filed §221 deduction overstated $1,500) and the two-chain equality instrument ran over two hand-listed non-HSA households — the fold wires both legs AND makes the instrument STRUCTURAL over the maximal fixture (every leaf populated) with an AGI twin; I-2 the code-W rule keyed on `Some(false)`; I-3 the spouse's HDHP plan collected (the instruction's "you or your spouse"); M-1 the document-less distribution door; M-2/N-1. When the fold returns: machine-check (the two probes' before/after; gut the 8f term → the maximal-fixture row reds with no HSA household named), commit through the gate, PUSH, dispatch the sonnet re-verification (`BRIEF-reverify-interview-T16.md`) AND **T7** (`BRIEF-build-interview-T7.md`) → T16 closes on the verifier's 0C/0I. All build/review/reverify briefs through T12 are committed. Push after each gate closes.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T16 BUILT `86bc9171`+`4b241dd7` / seam review IN FLIGHT; PUSHED `4b241dd7`) — T16 (Form 8889, the owner's ruling FR-76) landed after its resumed interruption: 3388 green; 21 lines + the employer-contribution worksheet transcribed; 13 authorities archived (the TY2024 1099-SA is `f1099sa--2019`); box-census 268/19/9; census-join 298 → 290 (the eight reach lines modelled); line-coverage 373/18; both oracles reconcile an HSA household; W-2 box 12 code W admitted with a contradiction rule; FR-78/79/80 filed. Its opus seam review (`BRIEF-review-interview-T16.md`, worktree at `4b241dd7`) is IN FLIGHT. When it returns: copy out, remove the worktree + branch, persist → ledger (`…T16-review-VERIFICATION.md`) → fold (SendMessage the T16 builder — context intact but ~800k tokens used; prefer a fresh opus) → commit → PUSH → sonnet re-verify (`BRIEF-reverify-interview-T16.md`, committed) AND dispatch **T7** (`BRIEF-build-interview-T7.md`; its review + reverify briefs committed) → T16 closes on the verifier's 0C/0I. Push after each gate closes. Owner questions Q1 (partly answered by the filed TY2024 return), Q2, Q4 still open.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T6 CLOSED 0C/0I; T16 BUILDING — RESUMED after a process exit; PUSHED `8767b937`) — T6 re-verified 0C/0I/0M/0N (report copied to `design/agent-reports/2026-09-07-build-interview-T6-reverify.md`, uncommitted). The T16 build agent was STOPPED mid-build when the previous Claude Code process exited; its partial work is intact in the main tree (59 files under `crates/`, 32 archive files, no report) and the agent was RESUMED from its transcript via SendMessage with instructions to re-establish state from `git status` / the scoped suites and finish. ★ If this happens again: never restart a build agent from scratch while its edits are in the tree — resume it (the transcript is saved), or if it cannot be resumed dispatch a fresh agent with "the tree holds a predecessor's partial work; assess before editing". Uncommitted controller files waiting for T16's tree: this file, `design/ROADMAP_STATUS.md` (T6 CLOSED), the T6 reverify report, `BRIEF-reverify-interview-T16.md`, `BRIEF-review-interview-T8.md`, `BRIEF-review-interview-T9.md`, `BRIEF-reverify-interview-T8.md`, `BRIEF-reverify-interview-T9.md`. When T16 returns: machine-check (every Form 8889 line present with its instruction text; the four reach lines; `authority-manifest` OK; both oracles), commit through the gate (T16 first, then the docs/briefs), PUSH, dispatch `BRIEF-review-interview-T16.md` (opus, worktree) → persist → ledger → fold → sonnet → then T7. Commit trailer: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` + `Claude-Session: https://claude.ai/code/session_01QvsUk3sBD4f1gZxt2hKpMX`.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T6 FOLDED `71b91a26` / re-verifying; T16 BUILDING; PUSHED `8767b937`) — T6's fold landed and is pushed (3371 green: the crypto slice's Digital Assets box is the filer's answer or nothing; the DA cross-check runs wherever the answer prints; Step 0's venue list obeys the year's regime; the standing-order row obeys the relief period). IN FLIGHT: the sonnet re-verification of T6 (`BRIEF-reverify-interview-T6.md`, worktree at `8767b937`) and the **T16 build — Form 8889 (HSA), the owner's ruling FR-76** (opus, main tree, `BRIEF-build-interview-T16.md`; large — expect a continuation). When the verifier returns: copy out, remove the worktree + branch, persist (commit only when T16's tree is committed) → close T6 at 0C/0I (roadmap + here) or fold residue after T16. When T16 returns: machine-check (every Form 8889 line present with its instruction text; the four reach lines; `authority-manifest` OK; both oracles), commit, PUSH, `BRIEF-review-interview-T16.md` (committed) → persist → ledger → fold → sonnet (`BRIEF-reverify-interview-T16.md`, drafted) → then **T7** (`BRIEF-build-interview-T7.md`, `BRIEF-review-interview-T7.md`, `BRIEF-reverify-interview-T7.md` all committed). Push after each gate closes. Owner questions Q1 (now partly answered by the filed TY2024 return's form set), Q2, Q4 still open.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T6 FOLDING; PUSHED `2363a44c`) — T6's seam review (1C/1I/4M/4N, `23765606`; ledger `2363a44c`) is being FOLDED by a fresh opus agent under `BRIEF-fold-interview-T6-review.md`: C-1 — the R6 crypto SLICE (TY2025, the filing year) still printed the Digital Assets box from the LEDGER (a stored `No` printed Yes; `None` printed Yes, no refusal) — the slice now prints the ANSWER (`Option<bool>`, neither on `None` + the mark) and arm (2) runs the DA cross-check (spec 1099-DA R6 amended by the controller); I-1 — Step 0's venue list ignored the year's 1099-DA regime (every fixture was TY2026). FR-77 filed (the `<=` election boundary decides filed basis). When the fold returns: machine-check (the PDF read-back table; the TY2024/25 no-venue-row fixtures), commit through the gate, PUSH, dispatch the sonnet re-verification (`BRIEF-reverify-interview-T6.md`, worktree) AND **T7** (`BRIEF-build-interview-T7.md`; `BRIEF-review-interview-T7.md` already committed) → T6 closes on the verifier's 0C/0I. ★ The owner holds the FILED TY2024 return (Forms 1040, Sch 2, A, B, D, 8949, 8889, 8959, 8960) — Form 8889 is the one gap (FR-76, owner decision pending). Push after each gate closes.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T5 CLOSED 0C/0I; T6 BUILT `a79492ee` / seam review IN FLIGHT; PUSHED `673b0f51`) — T5 re-verified 0C/0I/2M (`f7f8ad2a`), CLOSED. T6 (the exchange seam) landed `a79492ee` (3363 green by the gate; `da_no` written for the first time; the Step 0 panel; three deviations — a global election silences the standing-order row, the slice worksheet keeps its ledger fallback where no return exists, LIMITATIONS to T12; FR-73/74/75 filed). Its opus seam review (`BRIEF-review-interview-T6.md`, worktree at `673b0f51`) is IN FLIGHT. When it returns: copy out, remove the worktree + branch, persist → ledger (`…T6-review-VERIFICATION.md`) → fold (fresh opus) → commit → PUSH → sonnet re-verify (`BRIEF-reverify-interview-T6.md`, committed) AND dispatch **T7** (`BRIEF-build-interview-T7.md`, committed; write `BRIEF-review-interview-T7.md` at its gate) → T6 closes on the verifier's 0C/0I. Push after each gate closes. Owner questions Q1/Q2/Q4 still open (shown to the owner 2026-09-07).
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T5 FOLDED `2fe4ba5f` / re-verifying; T6 BUILDING; PUSHED `2fe4ba5f`) — T5's fold landed and is pushed (3350 green; the join's kill runs the instrument; `FilerRecordsContradicted` keyed on the ANSWER — the controller accepted the builder's deviation from the brief's liveness conjunct; the tips Caution refuses; seven income boxes refuse). IN FLIGHT: the sonnet re-verification of T5 (`BRIEF-reverify-interview-T5.md`, worktree at `2fe4ba5f`) and the **T6 build** (the exchange seam, opus, main tree, `BRIEF-build-interview-T6.md`). When the verifier returns: copy out, remove the worktree + branch, persist (commit only when T6's tree is committed) → close T5 at 0C/0I (roadmap + here) or fold residue after T6. When T6 returns: machine-check (the five-row DA table's outputs; the panel on a fixture; the standing-order pair), commit, PUSH, `BRIEF-review-interview-T6.md` (write at dispatch: seams = the DA cross-check tier placement, the panel reading the ledger and writing nothing, the printed box now from the ANSWER, no reconcile question in a registry) → persist → ledger → fold → sonnet → T7 (`BRIEF-build-interview-T7.md`, committed). Push after each gate closes.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T5 FOLDING; PUSHED `30318a7b`) — T5's seam review (0C/4I/4M/1N, `adeb91bf`; ledger `30318a7b`) is being FOLDED by a fresh opus agent under `BRIEF-fold-interview-T5-review.md` (the box→FieldId join's kill was a SHADOW — re-implemented predicates, 12/12 green with the checker gutted; the filer's-records mirror refusal; the DIV limb's kill; the tips Caution refuses fail-closed with the compute left to FR-72; income boxes with no reader refuse consistently; box-12 A/B; door questions grouped with the census; `current_prompt` routing). All briefs T6–T12 are committed. When the fold returns: machine-check (gut `join_failures_for` → red; the probe fixtures), commit through the gate, PUSH, then dispatch the sonnet re-verification (`BRIEF-reverify-interview-T5.md`, worktree) AND **T6** (`BRIEF-build-interview-T6.md`, opus, main tree) → T5 closes on the verifier's 0C/0I. Push after each gate closes.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T5 BUILT / seam review IN FLIGHT; PUSHED `9a845dc5`) — T5 (the 1099 sections) landed `34472416` + `9a845dc5` (3342 green; 58 new Fields; every censused box joined to a FieldId/RefuseReason by `xtask box-census`; R3's door; FR-65 closed; three builder-flagged items: D-1 the prompt-hash fix outside scope, D-2 `income answer` sweeps, `occupation_on_treasury_list` has no reader → FR-72). Its opus seam review (`BRIEF-review-interview-T5.md`, worktree at `9a845dc5`) is IN FLIGHT. `BRIEF-build-interview-T6.md` (the exchange seam) is committed. When the review returns: copy out, remove the worktree + branch, persist → ledger (`…T5-review-VERIFICATION.md`) → fold (SendMessage the T5 builder, context intact, or a fresh opus) → commit → PUSH → sonnet re-verify (`BRIEF-reverify-interview-T5.md`) and dispatch **T6** (opus, main tree) → T5 closes on the verifier's 0C/0I. Push after each gate closes.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T4b CLOSED 0C/0I; PUSHED `85962806`; T5 BUILDING) — T4b re-verified 0C/0I/0M/0N (report copied to `design/agent-reports/2026-09-07-build-interview-T4b-reverify.md`, uncommitted until T5's tree is committed). **T5** (the 1099 sections, `BRIEF-build-interview-T5.md`) is BUILDING with one opus agent in the main tree; `BRIEF-review-interview-T5.md` is drafted (uncommitted). When T5 returns: machine-check (the census decision tables per document and edition; `box-census` OK; `NotInForm` fell by five; `EXEMPT_PREFIXES` shrank; the paired questions' liveness; any TIN outside the documented list → fix before commit or the push fails), commit through the gate (+ the T4b reverify persist, this block, the T5 review brief), PUSH, dispatch the T5 seam review (opus, worktree) → persist → ledger → fold → sonnet → T6 (the exchange seam, §7 T6). Owner questions still open: Q1 (2026 documents), Q2 (line 19), Q4 (deposit vs check); S1/S2/S7, T7 — owner's.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T4b FOLDED / re-verifying; PUSHED; T5 next) — T4b's seam review is FOLDED at `4af15ab4` (3321 green: `opened_from`, the DOB shown-not-pre-filled, `FilingStatusConfirmed` live on an opened year, no seeded dependents or venue key, `Opened.carried_identity` named everywhere). **PUSHED 2026-09-07 at the owner's request: origin/main `2bd04d45` → `f91359c7` (276 commits).** The first push attempt was rejected by `scripts/pre-push`: (1) the generic scan failed on every commit since T1 on the builds' synthetic TINs + the IRS's example EIN in the archived W-2 instructions — fixed the documented way (`f91359c7`: ALLOWED_EIN grown with citations, two valid-shaped SSNs moved to the never-issued space + the unpushed-history bucket); (2) the owner-specific pattern matched a Federal Register county list under legal/text/ — the owner RULED the path allowlist grows to archived public text (legal/text/, legal/primary-sources/, design/forms/extract/; KAT H8b seen red vs the old hook). FR-71 filed (generic scan into pre-commit). IN FLIGHT: the sonnet re-verification of T4b (`BRIEF-reverify-interview-T4b.md`, worktree at `f91359c7`). NEXT: dispatch **T5** (`BRIEF-build-interview-T5.md`, opus, main tree — large; expect a continuation; tell the builder to pick TINs from the documented list / the never-issued SSN space); when the verifier returns, close T4b at 0C/0I (roadmap + here) or fold residue after T5's edits are committed. Push again after each gate closes (owner: "so we don't lose anything").
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T4b FOLDING) — T4b's seam review (1C/3I/4M/2N, `8ce1f470`; ledger `f344d349`) is being FOLDED by the T4b builder (resumed via SendMessage) under `BRIEF-fold-interview-T4b-review.md`: the DOB shown-not-pre-filled (C-1 — a bare Enter recorded a this-year `Given`), `opened_from: Option<i32>` provenance leaf, a `FilingStatusConfirmed` class-(A) question live on an opened year, dependents prompt-only (FR-70 → T7), no venue key, `Opened.carried_identity` named everywhere. When it returns: machine-check (the probe: open, bare-Enter the DOB → no `Given`; `answers_stored` false on the seed; the seed's dependents empty), commit through the gate, dispatch the sonnet re-verification (`BRIEF-reverify-interview-T4b.md`, committed `0cc82313`) AND **T5** (`BRIEF-build-interview-T5.md`, opus, main tree — large; expect a continuation) → T4b closes on the verifier's 0C/0I (roadmap + here). Standing lesson (T4b): a build's kills must ask what the NEXT SURFACE does with what it wrote.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T4 CLOSED; T4b BUILT / seam review IN FLIGHT; T5 brief ready) — T4 closed 0C/0I (`d13d9d09`). T4b (the year-N+1 opener) landed `44ca7075` (3309 green; 17 kills; deviations flagged for review: `filing_status` carried, the header seeded as identity, a dependent identity prompt with no surface until T7 → FR-70; FR-69 filed). Its opus seam review (`BRIEF-review-interview-T4b.md`, worktree at `3445c50b`) is IN FLIGHT. When it returns: copy out, remove the worktree + branch, persist → machine-check into `…T4b-review-VERIFICATION.md` → fold (SendMessage the T4b builder or a fresh opus) → sonnet re-verify → then dispatch **T5** with the committed `BRIEF-build-interview-T5.md` (the 1099 sections; owns FR-65/66/68; large — expect a continuation). Standing lesson: every Critical so far was a hand-enumerated set beside derived data; the T5 brief carries the rule.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T3 CLOSED; T4 FOLDED / re-verifying; T4b BUILDING) — T3 closed 0C/0I (`af18b798`). T4 (year gate + draft protection) landed `bbcee739` (3275; `screen_inputs_tiered` — one body, 31 param-free rules + 3 package-gated, import screens before it writes; `--discard-draft`); its seam review (1C/0I/3M/1N, `62d7828f`; ledger `846f178d`) FOLDED at `4ad17aaa` (3284: `draft_is_disposable` is STRUCTURAL — a draft equal to the year's fresh seed — after the reviewer showed a hand-list of four categories let a TY2026 broker-answers + Schedule C draft be destroyed unconfirmed; the builder ran a forbidden `git stash`, recovered via pop, stash list empty). IN FLIGHT: the sonnet re-verification of T4 (`BRIEF-reverify-interview-T4.md`, worktree at `68467dde`) and the **T4b build** (the year-N+1 opener, opus, main tree, `BRIEF-build-interview-T4b.md`). When the verifier returns: copy out, remove the worktree + branch, persist → close T4 (roadmap + here) or fold residue after T4b's edits are committed. When T4b returns: machine-check (LEAF_SOURCE walk of the seed; no carried AnswerRecord; carryforwards from the frozen `_out` chain), commit, `BRIEF-review-interview-T4b.md` (opus, worktree; seams: the seed vs `report --write-carryover`'s chain, identity prompts vs the census rows, T4's draft rules on the opener's write) → persist → ledger → fold → sonnet → T5 (the 1099 sections; owns FR-65/66/68).
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T3 FOLDED / re-verifying; T4 BUILDING) — T3's seam review (0C/5I/5M/1N, `4260080c`; ledger `a2b1bb7f`) is FOLDED at `750c1b34` (3250 green: the direction VALUE left the maps for one code-side reading per caption with the form's total sentence as evidence; `declared_rows` vs `requires_transcription`; Schedule C line 6 covered; the §6013 advisory split; document-first ordering). The sonnet re-verification (`BRIEF-reverify-interview-T3.md`, worktree at `15c15927`) and the **T4 build** (opus, main tree, `BRIEF-build-interview-T4.md`) are BOTH in flight — T4 was dispatched before T3's re-verification closed because no blocking finding is open; if the verifier returns an Important, fold it AFTER T4's edits are committed, never concurrently. FR-66/67/68 filed (T5/T8/T5). When the verifier returns: copy its report out, remove the worktree + branch, persist → close T3 at 0C/0I (roadmap + this block). When T4 returns: machine-check (the writer enumeration; the shared param-free list; the seven R11 kills red), commit, `BRIEF-review-interview-T4.md` (opus, worktree; seams: import ↔ screen_inputs shared tier, draft protection vs coherence, `income answer` draft path vs `resolve.rs`) → persist → ledger → fold → sonnet → T4b (the year-N+1 opener) then T5.
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T3 built + pre-review-folded / seam review IN FLIGHT) — T3 landed `0807335b` (3241 green; 18 census rows, 298 covered entries, 28 direction blocks, `interview_state()`, the panel) and its two builder-flagged defects were folded BEFORE review at `d44f82e4` (D1: the four 1099 rows carry the W-2's three rules, every fixture truthful — the controller's brief had asserted "nothing collects them", false; D11: a derived flip for a line the form subtracts — Schedule B line 3 → Overstates). The opus seam review (`BRIEF-review-interview-T3.md`, `07fcb501`, worktree) is IN FLIGHT. When it returns: copy the report out, remove the worktree + branch, persist → machine-check every claim into `…T3-review-VERIFICATION.md` → fold (opus; or SendMessage the T3 builder, whose context is intact) → sonnet re-verify → T4 (year gate + draft protection, §7 T4; its brief must enumerate every writer of `ReturnInputs` and every entry state of R11).
>
> ## ★★★ (superseded) RESUME 2026-09-07 (interview, T2 CLOSED / T3 building) — T2 landed `1c8a7301`; its seam review (1C/3I/4M/1N, `c5cf40ec`; ledger `edf5dde0`) FOLDED at `0c942ac2` (the archive is PER REVISION serving TY2024/25/26: 17 editions added, `revision_in_force` derived + pinned, rows resolve their edition from their year, FilerRecords bound to form + line, the document set derived from the manifest, `xtask authority-refresh --check` on demand) and RE-VERIFIED 0C/0I/0M/2N (`e1c578e1`). **T3 is BUILDING** with one opus agent in the main tree under the committed `BRIEF-build-interview-T3.md` (the document census with eighteen tri-state rows and the three rules, the `[direction]` tables asserted against both years' extracts, `covered_by` on every `unmodeled` entry with the direction rule and the reach check, the residual attestation naming every line it covers, `interview_state()`, the panel in `income answer`).
>
> When T3 returns (do NOT commit while its edits are half-done): machine-check its report (the census rows and their liveness; the 2024/2025 direction keys; the attestation's added phrases; each kill's red), commit through the gate, write `BRIEF-review-interview-T3.md` (opus, worktree, B3-scoped to the seams: the census ↔ classifier ↔ `screen_inputs` ↔ `apply` ↔ `income answer` chain; the join KAT's discrimination; `interview_state` vs the registries; state the seven PDF-less-worktree test failures are environment) → persist → ledger → fold → sonnet re-verify → T4 (year gate + draft protection). T5 owns FR-65 (1099-G box 10, W-2 box 14b). Refresh the progress page (scratchpad `overnight-return.html`, same path = same URL) after T2 closes. The C1 lesson: a brief never pins a YEAR for a per-revision artifact — state the rule and the years served. Owner questions still open: Q1 (2026 documents), Q2 (line 19), Q4 (deposit vs check); S1/S2/S7, T7, the push — owner's.

> ## ★★★ RESUME 2026-09-07 (interview, T1 folded / T2 building) — T1 landed `3142775d`, its seam review (1C/1I/4M/3N, `aeb4db8e`; ledger `f68af570`) FOLDED `d49de0c7` (3202 tests); the sonnet re-verification (worktree) AND the T2 build (opus, main tree, `BRIEF-build-interview-T2.md`) are IN FLIGHT. Read this block first.
>
> Pending once they return (do NOT commit while T2's edits are half-done): copy + persist the T1 r2
> verification (close T1 at 0C/0I or fold residue); machine-check T2's report (fourteen archived
> documents with the year read off each; `authority-manifest` OK; `Production::Collected { from }`;
> the per-document box censuses seen red), commit T2 through the gate, then T2's ONE seam review
> (opus, worktree) → persist → ledger → fold → sonnet re-verify → T3 (document census + interview
> state) under a brief from §7's T3 row. The C1 lesson of T1's review: every WRITER of a provenance
> structure must be enumerated in the brief (import, apply, answer, scrub, draft flush) — put it in
> every interview brief from T3 on. Owner questions open: Q1 (2026 documents), Q2 (line 19), Q4
> (deposit vs check). The simulated real return (FR-64) is the interview's first live use.

> ## ★★★ RESUME 2026-09-07 (interview, T1) — spec r2 GREEN by S6 (`62424903`); T1 (provenance schema) is BUILDING with one opus agent (`BRIEF-build-interview-T1.md`); its review brief `BRIEF-review-interview-T1.md` is on disk UNCOMMITTED with the roadmap/spec/FOLLOWUPS edits below. Read this block first.
>
> **Owner, 2026-09-07: "We will plan to use the interview to simulate a real tax return."** Recorded
> in `ROADMAP_STATUS.md` §0a (the simulated real return), spec §9 Q3 (answered: yes), FR-64, memory
> `interview-project`. It is the lived journey; first computable target TY2024. Pending commits once
> T1 returns (do NOT commit while its edits are half-done): the T1 build (code + report), then these
> doc edits + the review brief, then dispatch the T1 seam review (opus, worktree) → persist → ledger →
> fold → sonnet re-verify → T2 (archive the information returns). Owner questions still open: Q1 (the
> 2026 document list), Q2 (line 19), Q4 (deposit vs check).

> ## ★★★ RESUME 2026-09-07 (interview, fold) — the spec's ONE review is persisted (`1312d72b`, 1C/14I/9M/4N; ledger `a33f3ba5`) and its FOLD is IN FLIGHT (opus, prose only, `BRIEF-fold-spec-interview.md`). Read this block first.
>
> C1 (the `covered_by` join let an Advisory cover an income line — understatement, silent) gets a
> DIRECTION RULE + a prompt-names-the-line KAT; I1–I14 fold as the brief lists. When the fold
> returns: machine-check the report (every blocking finding has a quoted sentence in the spec),
> commit the spec as r2 = GREEN-by-S6 (no second prose round), then write `BRIEF-build-interview-T1.md`
> from §7's T1 row (the provenance schema: LEAF_SOURCE + KAT, payer_tin / transcribed_on, the answer
> log with one writer, Declined, CarryProvenance) and dispatch one opus agent → seam review → fold →
> sonnet re-verify → T2 (archive the information returns: W-2, 1099-INT/DIV/G/B, 1098, 1098-E +
> instructions, irs-prior, the b60c600c pattern). Owner questions (spec §9) still open: the 2026
> document list (S2), line 19 in v1, S1 as the first live walk, direct deposit vs check.

> ## ★★★ RESUME 2026-09-07 (interview) — the INTERVIEW project: recon `bf58fcdb`, brainstorm `c5af3f60`, spec r1 `127cb75c`; its ONE Fable review (S6) is IN FLIGHT (worktree). Read this block first.
>
> Owner ask: an interview eliciting income, deductions, real estate, dependents + the exchange
> exports to fill most of the return. Memory: `interview-project`. Long-range plan Phase 7; roadmap
> NOW bucket. When the review returns: copy from the worktree, persist, ledger, fold Criticals/
> Importants into `SPEC_interview.md` (the controller edits; a second prose round only if a Critical
> changes the SHAPE), mark the spec GREEN, then build T1 (provenance schema) by one opus agent under
> a brief written from the spec's §7 row — one seam review per build, one sonnet re-verification.
> Owner questions to surface (spec §9): the 2026 document list (S2), line 19 in v1, S1 as the first
> live walk, direct deposit vs paper check. The build cannot start T13/T14 without S2.

> ## ★★★ RESUME 2026-09-07 — THE QUEUE IS EMPTY. Residue sweep 1 verified 6/6, 15/15 kills red, 0C/0I (`…build-residue-sweep-1-verification.md`). No agent in flight. Read this block first.
>
> `main` at the commit after `240c9f40`, 3175 tests green, tree clean, ~217 commits unpushed. Every
> spec-queued track of 2026-09-05/06 is closed (FR-46, FR-49, FR-62, FR-61, FR-63 + the residue).
> The progress page: https://claude.ai/code/artifact/ff1208a5-0353-4b33-9810-96e0bca8b59b
> **What remains is the owner's:** S5's 2026-09-15 real-data run of `report --tax-year 2026` (needs
> the 2026 exports); S8's physical print rehearsal (`btctax extension` + `export-irs-pdf
> --pay-by-check` now exist); T7 (record the HIFO standing order under Notice 2026-20 before the next
> 2026 sale — `btctax config --set-forward-method hifo --exchange … --effective-from …`); decisions
> S1 (TY2025 rehearsal), S2 (name the real 2026 return), S7 (oracle fallback); the push. Then the
> calendar's next autonomous work is after the finals (Nov 2026 – Jan 2027): the TY2026 Form 8949 and
> Schedule D ports (two rows; R6 makes them the only gate for a TY2026 slice filing).
> Process in force: S6 (one prose round → build → one seam review → fold → one re-verification),
> one opus agent at a time under persisted `BRIEF-*.md`, sonnet for verification.

> ## ★★★ RESUME 2026-09-06 (last) — the residue sweep LANDED (`ca3b3eb9`, 3175 tests; docs `a97db15c`); its ONE sonnet verification (S6) is IN FLIGHT (worktree). Read this block first.
>
> When it returns: copy, persist; fold any Critical/Important via one opus agent (else nothing);
> then the queue is EMPTY — every spec-queued track and the residue are closed. The owner's items
> stand: S5's 2026-09-15 real-data run (needs the 2026 exports), S8's print rehearsal, T7 (Notice
> 2026-20 standing order before the next 2026 sale), decisions S1/S2/S7, the push (~215 commits).
> Under S6, future work is: spec → ONE opus review → fold → build → ONE seam review → fold → ONE
> sonnet re-verification. One opus agent at a time; briefs persisted as `BRIEF-*.md`.

> ## ★★★ RESUME 2026-09-06 (late night) — S6 RULED (one prose round, then build); the RESIDUE SWEEP is next: one opus agent under `BRIEF-build-residue-sweep-1.md`, then one sonnet verification. Read this block first.
>
> The sweep: FR-63 (the TUI commit modal's slice clause — a second NOTICE line or a modal-body slot,
> the ≤104-char kill kept), the §7503 DC legal-holiday calendar (TY2017's 2018-04-17 derived, not
> typed), the cite-check fixtures for the four 4868/1040-V pairs (the excuse list shrinks 40 → 36),
> a named kill for the full return's Section-B unanswered restriction row, the orphaned
> `slice_broker_refusal` doc block. NOT in the sweep: the two TY2025 authorities still missing
> (f8275 non-R — the IRS has only Rev. 10-2024, aliased; f8995a--2025 — see the curl in the commit
> message) — they belong to S1 (TY2025 rehearsal), unruled. After the sweep: nothing autonomous is
> queued; the owner's items stand (S5 09-15 data run, S8 rehearsal, T7, S1/S2/S7, the push).

> ## ★★★ RESUME 2026-09-06 (close) — EVERYTHING QUEUED IS CLOSED: FR-46, FR-49, FR-62 (R6), FR-61 (S9 drop). No agent in flight. Read this block first.
>
> `main` at the commit after `fe517611`, 3168 tests green, tree clean, ~207 commits unpushed. The
> progress page is current: https://claude.ai/code/artifact/ff1208a5-0353-4b33-9810-96e0bca8b59b
> **Nothing autonomous is queued by a spec.** The NOW bucket holds only the owner's items: S5's
> 2026-09-15 real-data run of `report --tax-year 2026` (needs the 2026 exports), S8's physical print
> rehearsal (now possible), T7 (Notice 2026-20 standing order before the next 2026 sale), decisions
> S1/S2/S6/S7, the push. Candidate autonomous work if asked: FR-63 (TUI commit modal clause), the
> cite-check fixtures for the 4868/1040-V pairs, the §7503 DC-holiday calendar, a named kill for the
> full return's Section-B unanswered row, the orphaned `slice_broker_refusal` doc block, dropping the
> TY2017 TaxTable if the owner wants it gone too. Process that held all day: one opus agent at a time
> under a persisted BRIEF-*, sonnet for verification, the controller persists → ledgers → folds →
> commits; the R6 spec loop (four rounds, one Critical per fold) is the evidence for the owner's S6.

> ## ★★★ RESUME 2026-09-06 (night 5) — FR-62 CLOSED (R6 green) and the S9 drop LANDED (FR-61 closes on its sonnet pin verification, in flight). Read this block first.
>
> When the verification returns: copy, persist; if clean → close FR-61 in FOLLOWUPS + this block,
> refresh the progress page. Then NOTHING autonomous is queued: the NOW bucket holds only the owner's
> items (S5 09-15 real-data run needs the 2026 exports; S8 print rehearsal; T7 Notice 2026-20 order;
> decisions S1/S2/S6/S7; the push — origin is ~230 commits behind). Candidate autonomous work if the
> owner wants more: FR-63 (TUI commit modal clause), the cite-check fixtures for the 4868/1040-V
> pairs, the §7503 DC-holiday calendar, a named kill for the full return's Section-B unanswered row,
> the orphaned `slice_broker_refusal` doc block.

> ## ★★★ RESUME 2026-09-06 (night 4) — R6 GREEN and FR-62 CLOSED (re-verified 8/8, 0C/0I); the S9 drop (FR-61) is IN FLIGHT with one opus agent in the main tree. Read this block first.
>
> Pending commits once the S9 agent returns (do NOT commit while its edits are half-done): (1)
> persist `…build-1099da-R6-review-r2.md` (copied in, untracked) in its own commit; (2) the FR-62
> closure edits already in FOLLOWUPS / roadmap / this file; then machine-check the S9 report
> (`…build-S9-drop-ty2017-implementation.md`: every pinned number old → new, the re-pointed KATs, the
> kept TaxTable), commit the drop through the gate, sonnet-verify its pins in a worktree, close FR-61,
> refresh the progress page. Then the NOW bucket is exhausted except the owner's items (S5 09-15
> real-data run, S8 print rehearsal, T7, decisions S1/S2/S6/S7, the push).

> ## ★★★ RESUME 2026-09-06 (night 3) — the R6 build's seam review (`c3a22a29`, 1C/2I/4M/1N; ledger `7eecd22c`) is FOLDED (`d81eea8c`, 3185 tests); its sonnet re-verification is IN FLIGHT (worktree). Read this block first.
>
> When it returns: copy from the worktree, persist; if 0C/0I → CLOSE FR-62 in FOLLOWUPS + roadmap §0a
> S10 ("the 4868 and 1099-DA builds are GREEN; R6 filed the slice from the stored answers"), refresh
> the progress page, then dispatch the S9 drop (FR-61, `BRIEF-build-S9-drop-ty2017.md`) to one opus
> agent → sonnet verify its pins → close FR-61. If not 0C/0I → persist, ledger, one more fold by one
> opus agent, re-verify. Owner-facing facts: TY2026 prints nothing until its 2026 Form 8949 /
> Schedule D finals are bundled; the TUI commit modal's slice clause is FR-63.

> ## ★★★ RESUME 2026-09-06 (night 2) — R6 BUILT (`fb5e7fc3`, 3178 tests) after the spec loop closed at r4 (`76d8eb9a` 0C/2I folded `6ac51769`); its SEAM REVIEW is IN FLIGHT (opus, worktree at `a2f71ea6`). Read this block first.
>
> When the review returns: copy from the worktree, persist, ledger, fold via one opus agent under a
> brief (or inline if tiny), commit, sonnet re-verify, close FR-62 in FOLLOWUPS + roadmap §0a S10.
> The reviewer was told to adjudicate the implementer's §7 edge (a committed row with EMPTY answers on
> a params-less year now falls to arm (3)). Then the S9 drop (FR-61, brief on disk). Residue filed:
> FR-63 (the TUI commit modal's slice clause). Owner-facing: TY2026 still prints nothing until its
> 2026 Form 8949 / Schedule D finals are bundled — R6 made that the only gate for a TY2026 slice.

> ## ★★★ RESUME 2026-09-06 (evening 5) — R6 r3 review (`5f03b965`: 7/11 + NEW 1C/4I/3M/1N; ledger `ab2710d8`) FOLDED (`22dfc5cc`); the CLOSING r4 review is IN FLIGHT (opus, worktree at `620ac55f`). Read this block first.
>
> r3's Critical was mine again (committed-over-draft precedence would file a superseded answer);
> T9 now wraps `input_form_store::load` (draft shadows committed; stale split; parked → None). The r4
> brief says: 0C/0I → BUILD from the text. If r4 returns Criticals/Importants again, STOP the spec
> loop and tell the owner: four rounds have each found one Critical in the controller's own folds —
> the honest next step is to build R6 by one opus agent WITH the r4 findings folded into its brief
> and let the build review (which executes) be the gate, per "tests for conformance, reviews for
> judgment". Owner's S6 (one review round per document) is not ruled; this loop is the evidence for it.
> Worktree audit (sonnet, `620ac55f`): no abandoned worktrees/branches; one empty scratch dir removed.

> ## ★★★ RESUME 2026-09-06 (evening 4) — R6 r2 review (`4ed35a03`: 11/15 resolved + NEW 1C/4I/2M; ledger `13cbddc7`) FOLDED (`961c2653`); the scoped r3 review is IN FLIGHT (opus, worktree). Read this block first.
>
> r2's Critical was mine again: T9's `ReturnInputs::default()` would have sworn `filing_status:
> Single` and shadowed the tax_profile. T9 is now a READ-ONLY accessor over the committed-or-DRAFT
> row (`input_form_store::broker_answers`), arm (2)'s predicate is "answers stored", the resolver is
> untouched; T8 maps every box to the form's own line (3 = C|I, 10 = F|L), refuses an unbound row,
> partitions the CSV. When r3 returns: persist, ledger, fold residue (if 0C/0I → GREEN), then BUILD R6
> (FR-62) by one opus agent under a brief written from the green R6 (T8, T9, the three-way dispatch
> on the accessor, the gate over every reachable map, the arm-(2) screens, `report` (2a)/(2b), the TUI
> ordering, the export-time price check, the notes and exit sentences, every kill listed) → seam
> review → fold → sonnet re-verify → close FR-62; then the S9 drop (FR-61).

> ## ★★★ RESUME 2026-09-06 (evening 3) — R6 r1 review (2C/5I/6M/2N, `cd7cfbd9`, ledger `c2905579`) FOLDED into the spec (`92179086`); the scoped r2 review is IN FLIGHT (opus, worktree at `9caab9d9`). Read this block first.
>
> The two Criticals were mine: the slice's Schedule D had no box dimension (→ task T8: per-box
> aggregation from the ROUTED rows, line 3 blank kill) and the TUI cannot commit on a params-less
> year (→ task T9: commit JUST the broker answers onto the row; I-11 untouched — the r2 brief asks
> whether a default ReturnInputs row fabricates testimony; expect that answer to shape T9). When r2
> returns: copy, persist, ledger, fold residue into R6, then BUILD R6 (T8, T9, the three-way dispatch,
> the arm-(2) screens, `report` in state (2), the TUI ordering, price coverage, the notes) by one opus
> agent under a brief written from the folded R6; then its seam review; then the S9 drop (FR-61).

> ## ★★★ RESUME 2026-09-06 (evening 2) — owner RULED S9 (drop TY2017) and S10 (REVERSED: the slice must file TY2026 with crypto sales); spec R6 written (`d63a971a`); its one-round opus design review is IN FLIGHT. Read this block first.
>
> **Queue (one opus agent at a time):** (1) the R6 review returns → copy from the worktree, persist,
> ledger, fold Criticals/Importants into the spec (the controller edits the spec; a build task list
> may move), commit; (2) BUILD R6 (FR-62) by one opus agent under a brief written from the folded R6
> (three-way dispatch in admin.rs, the TUI export, the two exit sentences, the report note, the kills
> R6 lists) → review the build (opus, seams) → fold → sonnet re-verify → close FR-62; (3) S9 drop
> (FR-61) by one opus agent under `BRIEF-build-S9-drop-ty2017.md` → sonnet verify the pins → close
> FR-61. The TY2017 TaxTable is KEPT (say so to the owner if they want it gone too).
> **Owner-facing facts to repeat when reporting:** R6 cannot print TY2026 until the 2026 Form 8949 /
> Schedule D finals are bundled (Nov 2026 – Jan 2027) — that becomes the only gate for a TY2026
> crypto-slice filing; the full return still needs FullReturnParams TY2026 + the January package.

> ## ★★★ RESUME 2026-09-06 (end of day) — BOTH builds are GREEN and CLOSED: 1099-DA (FR-46) and 4868/1040-V (FR-49). No agent in flight. Read this block first.
>
> **Chain today (all on `main`, nothing pushed, tree clean at `a2fde63e`):** 1099-DA T0–T6 + T7
> recorded, two seam reviews folded and re-verified 0C/0I; 4868/1040-V archive + T1–T5 + T2–T4, seam
> review 0C/4I/5M/2N folded `7eb93c27`, re-verified 11/11 (`f8e35037`), F-1 (a committed-blob hash
> drift) folded `a2fde63e` with `every_committed_entry_hashes_true_in_the_committed_blob_too`. Suite
> 3153 green. Process: one opus agent at a time under persisted `BRIEF-*.md` files; sonnet for
> re-verification; the controller persists → ledgers → folds → commits.
>
> **NEXT (nothing autonomous is queued by a spec):** the roadmap's NOW bucket is exhausted except the
> OWNER items — S5's 2026-09-15 real-data run of `report --tax-year 2026` (needs the owner's 2026
> exports), S8's physical print rehearsal (now possible: `btctax extension` + `export-irs-pdf
> --pay-by-check` exist), T7 (Notice 2026-20 order), decisions S1/S2/S6/S7/S9/S10, the push. Candidate
> autonomous work if the owner wants more: the four cite-check excuse-list pairs (a `FORMS` row + an
> extract each); the TY2025 Schedule D per-box read-back once its map is a full-return map; the
> §7503 DC-holiday calendar; refreshing the progress page
> (https://claude.ai/code/artifact/ff1208a5-0353-4b33-9810-96e0bca8b59b) with the day's closures.

> ## ★★★ RESUME 2026-09-06 (last) — the 4868/1040-V seam review is FOLDED (`7eb93c27`, 3152 tests); its sonnet re-verification (worktree) is IN FLIGHT. Read this block first.
>
> Chain: review persisted `17e180bf` (0C/4I/5M/2N), ledger `7c6fe99f` (6/6 HOLD), fold brief
> `e423b44e`, fold `7eb93c27` (I-1 every --pay refusal before any byte; I-2 the envelope guard in
> both directions; I-3 the experimental notice on `extension`; I-4 the record-your-payment note; M-1..
> M-5, N-1; spec T6 reworded; examples golden +2 pointer lines). When the r2 report returns: copy it
> from the worktree, persist (own commit), ledger if it has new findings, fold any residue via one opus
> agent, else CLOSE FR-49 in FOLLOWUPS + roadmap and refresh the progress page. Then the calendar
> (09-15 real-data run needs the owner's 2026 exports; S1/S2/S6/S7/S9/S10/T7 untouched; nothing pushed).

> ## ★★★ RESUME 2026-09-06 (latest) — the 4868/1040-V build is LANDED through T4 (`a51b8c53`, 3145 tests); its independent SEAM review (opus, worktree) is IN FLIGHT. Read this block first.
>
> When the review returns: copy its report from `.claude/worktrees/agent-*/design/agent-reports/
> 2026-09-06-build-4868-T1-T6-review.md` into the main tree, persist it in its own commit, machine-check
> every checkable claim into `…-review-VERIFICATION.md` (own commit), then fold Criticals/Importants
> (+ cheap Minors) via ONE opus agent under a brief, commit the fold with the gate output, sonnet
> re-verify in a worktree, persist → ledger → close FR-49 at 0C/0I. Remove the reviewer's worktree
> (`git worktree remove --force` + `git branch -D`) after copying. Then: the calendar — the 09-15
> real-data run (S5) needs the owner's 2026 exports; owner decisions S1/S2/S6/S7/S9/S10/T7 untouched;
> nothing pushed. The overnight progress page can be refreshed with today's closures:
> https://claude.ai/code/artifact/ff1208a5-0353-4b33-9810-96e0bca8b59b

> ## ★★★ RESUME 2026-09-06 (later) — 1099-DA build GREEN and closed; 4868/1040-V build: T1+T5 LANDED `dc9941d5` (3109 tests), T2–T4 being dispatched to one opus agent. Read this block first.
>
> **Commits since the block below:** `c25f7489` fold of the T3–T6 review; `a0e90f1b` process;
> `dc9941d5` 4868 T1+T5 (the rows, the maps, the year records, the row gates, the reader walk's grid
> bucket; report `…build-4868-T1-T5-implementation.md` — `instr_pages` measured [1,4]/[1,2], the
> label floors carried +16/year of pre-existing drift plus +5 for the 4868, the cite-check excuse list
> gained the four pairs like 36 of 37 others); `a939ada9` the r2 verification persisted (9/9
> RESOLVED); `38564385` FR-46 CLOSED in FOLLOWUPS, roadmap S10 line carries the chain.
>
> **NEXT:** T2–T4 (fillers, `btctax extension`, the `--pay-by-check` voucher hook) — brief
> `design/agent-reports/BRIEF-build-4868-T2-T4.md`, ONE opus agent, no commit by the agent; then T6
> (surfaces: `report` names the paths; help text; readiness sentence unchanged); then ONE independent
> opus review of T1–T6 scoped to the seams (map ↔ filler ↔ command ↔ export hook ↔ manifest), persist
> → ledger → fold → sonnet re-verify → 0C/0I. Residue to carry into T2/T4: `btctax_forms::
> attachment_sequence` returns None for the new stems via its catch-all — make it an explicit arm.
> Then: the 09-15 real-data run (S5) needs the owner's 2026 exports; owner decisions S1/S2/S6/S7/S9/
> S10/T7 untouched; nothing pushed (origin is ~180 commits behind).

> ## ★★★ RESUME 2026-09-06 (late) — the 1099-DA build is GREEN (T3–T6 review folded `c25f7489`, re-verified 0C/0I 9/9); the 4868 T1+T5 implementer (opus) is IN FLIGHT in the main tree. Read this block first.
>
> **Pending commits once the implementer returns (do NOT commit while its edits are half-done):**
> (1) persist `design/agent-reports/2026-09-06-build-1099da-T3-T6-review-r2.md` (copied into the tree,
> untracked) in its own commit; (2) the roadmap / FOLLOWUPS / continuity edits already in the working
> tree (FR-46 CLOSED; the S10 line carries the whole chain). Then machine-check the 4868 T1+T5 report
> (`…build-4868-T1-T5-implementation.md`), commit the build, and dispatch T2–T4 to the next single opus
> agent with a brief in the same shape (`BRIEF-build-4868-T2-T4.md`), then T6.
>
> (The block below is the state as of the fold dispatch; it stays for the reasoning.)

> ## ★★★ RESUME 2026-09-06 (late) — T3–T6 review persisted (0C/3I/4M/2N) and being FOLDED by one opus agent; the 4868 build's T1 archive landed and its T1+T5 brief is written. Read this block first.
>
> **Owner directive (2026-09-06, saved to memory `one-opus-agent-coordinator`):** delegate tasks to at
> most ONE opus agent at a time; this session coordinates (briefs, machine-checks, persist → ledger →
> fold → commit). Do not run two opus agents concurrently.
>
> **State:** review `design/agent-reports/2026-09-06-build-1099da-T3-T6-review.md` persisted
> `2ef2c2ce`; its ledger (10/10 HOLD) `e13fa24b`. The three Importants are on the READ surfaces:
> I-1 the TUI Forms tab prints UNROUTED boxes on a live year (route in `tabs/forms.rs::render`, the
> Snapshot needs the stored answers); I-2 no surface ENUMERATES a key's rows (spec R1 MUST — `report`
> block and the TUI row pane must list date sold · amount · proceeds · (e) per row); I-3 the block never
> appears on a FIRST authoring session (seed only runs at open with `working: Some`; re-seed after
> NI-2 materialization). M-1..M-4, N-1, N-2 folded in the same pass where cheap (M-4 recorded as
> forward-looking). **An opus fold agent is IN FLIGHT** in the main tree; it writes
> `…T3-T6-fold-implementation.md` and does not commit. When it returns: machine-check its report
> (`git diff --stat`, run the crates it names, diff the goldens), commit the FOLD (one commit, gate
> output in the message), then a sonnet re-verification of the fold (`…T3-T6-review-r2.md`),
> persist → ledger → fold any residue → 0C/0I closes the 1099-DA build.
>
> **4868/1040-V build:** T1's archive half landed `b60c600c` (TY2024 f4868/f1040v: notes, extracts,
> geometry, manifest 138). The T1+T5 implementer brief is
> `design/agent-reports/BRIEF-build-4868-T1-T5.md` — dispatch it to ONE opus agent after the fold
> agent returns (never concurrently), then T2–T4, T6 per the spec's Plan. Then the calendar: the
> 09-15 real-data run (S5) needs the owner's 2026 exports.
>
> **Traps (new):** a reviewer's report lives in ITS worktree (`.claude/worktrees/agent-…`) — copy it
> into the main tree before `git worktree remove --force` + `git branch -D`; do not commit while an
> implementer agent has half-done edits in the shared tree (the pre-commit hook runs `make check` on
> the working tree); briefs live in `design/agent-reports/BRIEF-*.md`, not a new directory.

> ## ★★★ RESUME 2026-09-06 (night) — the 1099-DA build is COMPLETE through T6, T7 recorded; next is its independent build review, then the 4868/1040-V build. Read this block first.
>
> **Owner resumed the session with "Resume"; autonomous again.** Tree clean, `main` not pushed.
> Owner decisions S1/S2/S6/S7/S9/S10 and the new **T7** (`ROADMAP_STATUS.md` §0a) NOT actioned.
> The overnight progress page is published (private):
> https://claude.ai/code/artifact/ff1208a5-0353-4b33-9810-96e0bca8b59b
>
> **Since the evening block:** T6-a `17753789` (`report`'s Form 1099-DA answers block via
> `render_broker_answers` + `broker_key_census`; `YearReadiness::sentence` names the regime — the
> examples golden moved on exactly its four readiness lines; the TUI forms-tab note is
> `broker_box_note(year, regime_for(year))`; `income import`'s doc + man page document the TOML
> shape). T6-b `bb6d140d` (the input-form block: `SectionId::BrokerReporting`, Repeating over the
> map, `FieldId::BrokerCovered/BrokerNoncovered` Enum slots with `Unanswered` as the clear; `add`
> refuses, rows are SEEDED from the ledger at open on a basis year (`seed_broker_rows`), the row list
> enumerates each provider's covered/noncovered row counts (`broker_census` on the form state,
> `broker_row_provider` in the seam so the renderer lint holds); the four broker refusals anchor the
> block's slot by cohort; coverage KAT pins 96→98 + the two leaves). T7 written into `ROADMAP_STATUS.md`
> §0a with the Notice 2026-20 §4.02(2) text, the exact `btctax config --set-forward-method` command
> and the "does the election record qualify" answer. FR-46 (a)(b)(c) closed in `FOLLOWUPS.md`.
> Full suite 3091/3091 at `bb6d140d`.
>
> **NEXT:** (1) the independent BUILD REVIEW of T3–T6 (opus, worktree at HEAD, report to
> `design/agent-reports/2026-09-06-build-1099da-T3-T6-review.md`), scoped per B3 to the SEAMS the
> T0–C review could not see: T3's page-sets ↔ T4's per-box totals ↔ T5's blank (f)/(g) ↔ T6's
> surfaces, and the answers' whole path import → screen → route → print → report; persist → ledger →
> fold → re-verify. (2) The 4868/1040-V build T1–T6 (`design/SPEC_form_4868_1040v.md` "Plan"): T1
> archives `f4868--2024` / `f1040v--2024` (irs-prior, notes + extracts + geometry) — check the
> `design/forms/2025/` f4868/f1040v archives from `bc88583c` first; T2 `Form4868Map` +
> `fill_form_4868(&PrintedReturn, year, choices)`; T3 `btctax extension`; T4 `Form1040VMap` + the
> export hook; T5 the reader's `#[cfg(test)]` walk learns the grid case BEFORE the label join; T6 the
> surfaces. (3) Then the calendar: the 09-15 real-data run needs the owner's 2026 exports (S5).
>
> **Traps (new):** three coverage-KAT pins bite on a new Field (the Field count, the covered-leaf
> count, the pinned leaf→field map — tuple order is (FieldId, "path")); the tui-edit renderer lint
> forbids any `ri.<field>` text in draw_edit.rs — route reads through a seam fn.

> ## ★★★ RESUME 2026-09-06 (evening) — the 1099-DA build is through T5; T6 (the surfaces) is MAPPED, not started. Read this block first.
>
> **Paused at the owner's "Find a place to pause".** Tree clean, everything on `main` (not pushed),
> HEAD `ab0c98f8`. Owner decisions S1/S2/S6/S7/S9/S10 (`ROADMAP_STATUS.md` §0a) NOT actioned.
>
> **Since the afternoon block:** T4 `2feb53d0` (Schedule D 1b/2/8b/9 from the printed 8949's
> per-box totals; lines 3/10 = the not-reported box only; both maps bind the four rows; census
> registers lowered 2025/schedule_d 40→24, total 729→713, TY2025 326→310; kill: the G-total lands on
> 1b). The sonnet re-verification of the T0–C fold persisted `9f977180`
> (`…build-1099da-T0-C-review-r2.md`, **0C/0I/3M/1N**) and its three Minors folded `820b6d9b`
> (TUI regime refusal before the exclusive mkdir; the slice refusal's readiness note WATCHED; every
> year with full-return tables ⊆ bundled years). T5 `ab0c98f8` ((f)/(g) blank on every answer with a
> PDF read-back kill seen red on a planted code B; `broker_reporting_advisory(year, REGIME, rows)`
> — live text names box 1g / box 1f / the Note on Form 8949 / `exchange:PROVIDER:ACCOUNT`;
> `IrsPdfReport.regime`; examples golden byte-identical). Full suite 3083/3083 at HEAD.
>
> **NEXT = T6, the surfaces (spec `SPEC_1099da_broker_reporting.md` "T6"). What is already mapped:**
> 1. **TUI forms tab** `crates/btctax-tui/src/tabs/forms.rs:179` still branches on
>    `DIGITAL_ASSET_8949_FIRST_YEAR`; make it `btctax_cli::year_readiness::regime_for(year)` with a
>    third (live) wording — boxes chosen from the answers, compare (e) with box 1g.
> 2. **`YearReadiness::sentence`** (`year_readiness.rs:115`) gains "1099-DA regime: none |
>    proceeds | proceeds+basis" from `declared.information_returns.f1099da`; its consumers are the
>    `report` header (`main.rs:160`) and tests at `:170/:186/:269`.
> 3. **`report` lists the keys, their rows, the answers:** add a field to `TaxYearReport`
>    (`cmd/tax.rs:422`) built in `report_tax_year` where `ri`/`state`/`regime` are in scope
>    (`:524`), print it in `main.rs` after the readiness sentence. Rows per key come from
>    `btctax_core::forms::broker_key(row)` over `form_8949(&state, year)`; only on a live regime.
> 4. **`income import` TOML shape** — document `[broker_reporting.<provider>] covered = "…"
>    noncovered = "…"` in `crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml` (the
>    examples journey imports it — the golden will change, inspect the diff) and the `Import` doc
>    comment in `cli.rs:399` (→ man page via `make docs`). The parse test already exists
>    (`tax.rs:1244`).
> 5. **The input-form block** (biggest). Seam facts: `btctax-input-form/src/seam.rs` —
>    `SectionId` (add `BrokerReporting`), `FieldId` (add `BrokerProvider` Text, `BrokerCovered` /
>    `BrokerNoncovered` `Enum(&["Unanswered","NotReported","ProceedsOnly","BasisMatches",
>    "BasisDiffers","Mixed"])`), `SectionKind::Repeating{len,add,remove}` over
>    `ri.broker_reporting.0` (BTreeMap<String, CohortAnswers{covered,noncovered: Option<_>}>; row i =
>    i-th key; `add` inserts a placeholder key, `remove` deletes; the provider field's `set` renames
>    the key). Declared in `spec/sections.rs` (model: `DEPENDENTS`/`W2_FIELDS`), listed in
>    `spec/mod.rs`. The coverage KAT `spec/coverage.rs::every_in_scope_leaf_is_covered_by_exactly_
>    one_field_or_exempt` needs: one provider entry in `maximal_fixture()` (else the map has no
>    leaves and is not policed), `addr_for(SectionId::BrokerReporting) = RowAddr(vec![0])`, and an
>    Enum `sentinel` per new FieldId (panics until added). `attribute.rs:71` re-points the four
>    `Broker*` refusals from `NotInForm` to `Anchor::Field`s of the block (the TODO there says so).
>    TUI side: `btctax-tui-edit` — form state built in `main.rs::open_tax_inputs_form` (has
>    `app.session`; compute the per-(provider, cohort) ROW COUNTS from the ledger there and carry
>    them on `TaxInputsFormState` so `draw_edit.rs` (RowList at `:2351`, previews from the first
>    field at `:2577`) can ENUMERATE each key's rows before the answer, as the spec requires); seed
>    the map with the ledger's keys at open so the rows are the keys, not typed. Then the
>    `cmd/answer.rs` CLI twin if it enumerates FieldIds.
> 6. **T7 (owner action, Notice 2026-20 §4.02(2))** → `ROADMAP_STATUS.md` §0a with a date; the
>    notice text is under `legal/text/` (locate with `find legal -iname '*2026-20*'`); T6 states
>    whether the per-year method-election record qualifies as the written standing order.
> Then an independent build review of T3–T7 (opus, worktree at the commit, `main..HEAD` seam-scoped
> per B3), then the 4868/1040-V build T1–T6 (archive the TY2024 f4868/f1040v revisions first).
>
> **Traps (new):** a python `'''` heredoc turns a Rust `\`-newline string continuation into
> joined lines WITH the indentation — the advisory text gained runs of spaces and the examples
> golden moved; collapse `(?<=\S) {2,}(?=\S)` inside the string and re-diff the golden. nextest
> `test(name)` matches TEST names, not binaries — a `test(field_census)` filter ran one test.

> ## ★★★ RESUME 2026-09-06 (afternoon) — the 1099-DA build is through T3 and reviewed; T4 next. Read this block first.
>
> **Owner asleep; autonomous.** Everything is committed on `main` (not pushed). Owner decisions
> S1/S2/S6/S7/S9/**S10** (`ROADMAP_STATUS.md` §0a) are NOT actioned.
>
> **GREEN artifacts:** spec 1099-DA r6; spec 4868/1040-V r6; the label-join instrument; `xtask
> port-status` (FR-50; five verifications, the work list declares its tags). Reports + ledgers under
> `design/agent-reports/2026-09-06-*`.
>
> **1099-DA BUILD (spec T-plan), commits:** T0 `e4b80fda`; T2-a `ee02e503`; T1-a `36d83f13`; step C
> `249d37ad` (screen at the ledger, router in the packet, slice closed on a live year); **T3**
> `d9863909` (one 8949 page-set per (part, box); the 2025 map names G/H/I and J/K/L by letter; both
> fillers group by box); the Opus BUILD REVIEW of T0–C (`…build-1099da-T0-C-review.md`, 0C/3I/3M/3N,
> ledger 9/9) folded in `18d1332b` (the 8949 CSV gated+routed before any byte on both export paths;
> the three gate wirings watched red; a fold-driven Covered lot; `regime_or_pre_regime`). A sonnet
> verification of that fold is IN FLIGHT → `…build-1099da-T0-C-review-r2.md`.
> **NEXT = T4:** Schedule D per-box totals — `ScheduleDLines` gains 1b/2 (ST: G, H) and 8b/9 (LT: J,
> K) beside 1a/3 and 8a/10, line 3 = the I-box totals only, 10 = L only, transcribed from
> `f1040sd--2025.txt:32-37, 57-62` ("Totals for all transactions reported on Form(s) 8949 with Box A
> or Box G checked" …); the 2025 map binds Row1b/Row2/Row8b/Row9 (dump-fields: f1_7..10, f1_11..14,
> f1_27.., …); the fillers write them; `line_coverage` quotes; kill: the G-total lands on 1b, not 3.
> Then T5 ((f)/(g) blank + the advisory naming box 1g and box 1f), T6 (surfaces: `report` lists keys
> and rows; the TUI input-form block re-points the NotInForm anchors; `YearReadiness::sentence`; the
> advisory + TUI forms tab read the regime), T7 (owner action, Notice 2026-20). Then the 4868/1040-V
> build T1–T6 (archive the TY2024 revisions first).
>
> **Traps:** the harness's bypass guard string-matches ` -n`/`-nE` — never `sed -n`/`grep -n` in a
> Bash call that also commits; a staged file from a BLOCKED commit rides into the next — `git status`
> before grouping; `git commit -m` with backticks eats words — `-F -` + quoted heredoc; rustfmt
> reflows anchors — patch by span/regex, assert before writing; the shrink-only census registers
> (`field_census.rs`) must be lowered by hand when a map gains fields; `assemble_absolute` refuses
> TY2026 (Form 6251 2026 untranscribed) — wiring kills run on TY2025 with the live regime as a value;
> build reviews run on a worktree at the commit (the tree was dirty once).

> ## ★★★ RESUME 2026-09-05 (late) — the Fable plan review is being FOLDED. Read this block first.
>
> The owner switched the session to Fable and asked for a Fable review of the long-term plan the
> recon fan-out produced. It is persisted verbatim at
> `design/agent-reports/2026-09-05-fable-plan-review.md` (**1C/8I/4M**, commit `3f279d64`), and
> every checkable claim in it was machine-checked by the controller BEFORE folding — the ledger is
> `design/agent-reports/2026-09-05-fable-plan-review.VERIFICATION.md` (`f74d605f`: 30 claims,
> 27 HOLD, 2 NOT LOCATED, 1 PARTIAL). Nothing in the review was taken on its word.
>
> **Fold 1 — DONE, `118b070b`.** C1: the full-return export hardcoded `broker_reported_rows: 0`,
> so the [I5] broker-reporting advisory never fired on the one path that hands the filer a packet
> to sign. Fixed with ONE predicate (`btctax_core::forms::possibly_broker_reported`) carried on
> `Printed8949`; the kill was the characterization test that had pinned `0` on the same fixture
> the slice arm counts `1` on — observed red (rc=100) with the literal planted back. The examples
> golden gained exactly one line (the advisory on J6), inspected before regenerating.
>
> **Fold 2 — DONE (documents), the commit after `a50a3a62`.** What it did, in order:
> 1. `design/FORM_AUTHORITY_TABLE_DESIGN.md` rewritten to the review's shape, which my draft's §5
>    had argued against and which the ledger shows is compatible: the ROW is the `.map.toml`
>    HEADER (judgments `versioning`/`line_set`/`instructions` written by the human there), the row
>    SET is the glob, the binding is a `build.rs` (mechanical only), the year record is
>    `forms/<year>/YEAR.toml`, `forms-provisional/` is never globbed. `Revision`→`versioning`,
>    `MapFamily`→`line_set`, the Rust `const FORMS` retires.
> 2. I4: `TY2026_PORT_REPORT.md` §6 rule 1 re-aimed — "never encode a figure whose ONLY source is a
>    draft", NOT "never encode TY2026 until finals". `tax_tables.rs` gate doc reason 1 is stale
>    (Rev. Proc. 2025-32 §2.10 is in the tree); reasons 2 and 3 keep the gate. `AmtParams` TY2026
>    becomes a NOW item.
> 3. I5: `ROADMAP_STATUS.md` §3 repartitioned NOW / AFTER FINALS / AFTER OTS-2026, critical path
>    stated, Form 4868 + 1040-V as the default plan (M2's stale TY2025 line goes with it).
> 4. I6/I7/I8: step 24 re-pointed at P3 + `Coverage::quoting`; build-order item 1b
>    (`YearReadiness` + `income import` refusal + TUI default year); the work list gets
>    `NO PRIOR SIDE`/`NO DRAFT` cells for `f1040s1` and `f8283` and §5d's "moot" is withdrawn.
> 5. I2: the `CLAUDE.md` transcription clause amended — *fields are named for the line WITHIN a
>    transcription struct; cross-year quantities are semantic* — so the next reviewer does not red
>    the build for the old sentence.
> 6. `FOLLOWUPS.md` entries with owning phases for what is NOT built in this fold: the 1099-DA
>    year-record slot + filer `broker_reported` answer (C1 residue), `AmtParams` TY2026 (NOW),
>    Form 4868/1040-V (P4), `YearReadiness` (build-order 1b), `forms port-status` (I8), M1, M3, M4.
> 7. The Fable reminder in `ROADMAP_STATUS.md` §5 and the memory file: DISCHARGED — the switch
>    happened and the review ran.
>
> ## ★★★ STATE AT 6267b6b1 (2026-09-05, latest) — design GREEN; STEP 1 LANDED; phase review running
>
> - Re-review r3 was **0C/0I/3M** (`53ca5b61`; Minors folded `fc6dea70`) → the design is green.
> - **Step 1 DONE — `6267b6b1`**: the ROW on all 37 maps (computed by script), `MapRow`/`Versioning`,
>   nine fields on 17 structs, four kills observed red on planted copies, the two-way join through
>   `irs_stem` in xtask, 3016 tests. Found and fixed FR-55 (Form 8283 stapled last for TY2025: the
>   template prints 36, the literal said 155). An independent phase review of `6267b6b1` is
>   RUNNING → `design/agent-reports/2026-09-05-r2-step1-review.md`; persist, ledger, fold.
> - **Step 2 DONE — `68b86b8e`** (build.rs + `Stem` + generated bindings + the tarball gate; 3022 tests).
> - **Steps-2/3 review FOLDED — `b58dee9d`** (Q1 the 8275 licence is periodic_template's pairing; Q2
>   `FiledPacket::stapled` is the only constructor, 8959 asks must_file first; Q3–Q6). **The phase
>   review of steps 4–5 + that fold is RUNNING** → `design/agent-reports/2026-09-05-r2-step4-review.md`
>   (★ the first attempt died on an API rate limit — HTTP 429, session limit — before writing
>   anything; re-dispatched 2026-09-06. If the report is absent, that is why: re-dispatch it).
> - **Steps-4/5 review FOLDED** (0C/4I/6M, `5b073355`; ledger + fold commits after): ★ R2 was the real one —
>   the label reader dropped every binding with a trailing `#` comment (Schedule A/2025 contributed
>   ZERO joins to step 5's justification) and, once widened, its one-column vertical rule mislabelled
>   the 1040's 2b/3b boxes; fixed with an x-aware in-row join (2024: 152→235 joins, 2025: 116→193; a
>   planted 2a/2b swap reds both ways; floors raised). The eight wired TY2025 maps now rest on eight
>   maps' evidence. R1 the TY2025 record's `ots` was false (OTS 2025 IS installed, external). R3/R4
>   closed by FR-48. **FR-46's SPEC is DRAFT r1** (`design/SPEC_1099da_broker_reporting.md`), its
>   review dispatched.
> - **FR-48 (build-order 1b) DONE** (the commit after 9e81a9b0): `report --tax-year` prints the
>   readiness line; its refusal for a not-ready year is built from readiness and KEEPS the inputs;
>   `income import` stores + warns (not refuses — the carryover chain needs the row); `export-snapshot`
>   stamps `TAX_YEAR.txt` (not gated — data export); the third TUI literal (`unlock.rs`) derived.
> - **Step 5 ◐ — eight of ten TY2025 maps WIRED — `162739e8`**: verified by the label
>   join + census + parse; `line_set_wiring.rs` pins wired ⇔ parses (plant observed red). Two stay
>   `Unwired` on purpose: `f6251/2025` (1a/1b rebuild — do once for TY2026) and `f1040s1a/2025` (no
>   struct; the Schedule 1-A emitter is the NOW item). **The year-package table is BUILT** (steps 1–5);
>   what remains of design r2 is the two deliberate holes and the FR-46/FR-48 readers of
>   `YearReadiness`. **NEXT:** fold the step-4 review when it lands; then the Schedule 1-A EMITTER
>   (`Schedule1AMap` + `fill_schedule_1a`, the 17th form — NOW for TY2026) and FR-46's SPEC.
> - **Step 4 DONE** (the commit after 98e279c9): `forms/<year>/YEAR.toml` ×3 bound by build.rs, `YearRecord`
>   + `YearReadiness` (btctax-cli) with their kills, TUI default year derived, the census reads the
>   record and `Stem::ALL`. `TRANSITION_DATE` deliberately stays in `conventions` (not a year fact).
>   **NEXT: step 5** — wire the ten TY2025 maps: each is a deletion from `line_set::Schema::Unwired`;
>   `f6251/2025` needs the 1a/1b transcription struct (a rebuild, code); the rest may parse into their
>   2024 structs (check each — a map that parses is a one-line arm change; one that does not is a
>   renumber and a new struct). Then the steps-2/3 review's fold; then FR-46's spec.
> - **Step 3 DONE — `bc6dce35`** (LineSet/Schema/Unwired; 54 consts + the SUPPORTED_YEARS hand-list gone;
>   periodic alias by hash; 3023 tests). The step-1 phase review (`2c3044e3`, 0C/2I/6M, all 37 rows
>   machine-verified) is FOLDED in the commit after: P1 kill 4 keyed on extract existence with the five
>   TY2017 rows pinned as `SequenceUnverifiable`; P2 the packet is stable-sorted by sequence (the position
>   half of FR-55); P3–P7 as tests/doc fixes. **NEXT: step 4** — `forms/<year>/YEAR.toml` + `YearReadiness`
>   (declared vs actual) with its kills; move `TRANSITION_DATE`/`TY2025_RETURN_DUE`/`FORMS_ABSENT_FROM_YEAR`
>   and the two `selected_year: 2025` literals in; then step 5 (wire the ten TY2025 maps, `line_set` per
>   revision: `f6251/2025` needs the 1a/1b struct).
> - **NEXT: step 3** — switch the fillers over (`map_text(Stem::X, y)` → row → `LineSet` → the exhaustive
>   `line_set → schema` match with an explicit `Unwired` arm for the ten TY2025 maps), delete the 18 + 17
>   arms, the 54 consts and `SUPPORTED_YEARS` (error message built from `bundled_years()`). Wait for the
>   step-1 phase review to land first (it audits the 37 row values step 3 would bake in).
> - (was) **step 2** — `build.rs` beside the old arms (globs `forms/<year>/`, emits `bundled.rs` with
>   `Stem`, `template()`, `map_text()`, `bundled_years()`), a test that `template`/`map_text` agree
>   byte-for-byte with every existing `include_*` const, and the `cargo package --list` gate that
>   every globbed file is in the tarball (design r2 §5). Then step 3 (switch fills; `Unwired` arm).
> - Also landed today: FR-49 authorities archived (`bc88583c`: f4868/f1040v 2025), the 270 GB
>   worktree cleanup (`c46e9e1a`).
>
> ## ★★★ STATE AT de1fa8a3 (2026-09-05, latest) — FR-47 landed; fold C landed; re-review r3 RUNNING
>
> - **FR-47 DONE — `8b36aba0`**: `pub fn ty2026_full_return()` in `tax_tables.rs`, every cell from a
>   held source (Rev. Proc. 2025-32 §2.02/§2.10/§2.14/§2.26/§2.29; Pub. L. 119-21 §70102/§70105/
>   §70107(c)/§70120; Notice 2025-67 — fetched with Notice 2023-75 by
>   `legal/_scripts/fetch_retirement_limit_notices.sh`; manifest 133). KAT-pinned incl. the 0.50
>   identity across three statuses. **Deliberately NOT in `by_year`** — the gate's reasons 2 and 3
>   still hold; the gate test's message now says so. Open cell (D8): the MFS kicker start/cap as the
>   2026 instructions will print them — AFTER FINALS.
> - **Fold C — `de1fa8a3`** (of the fold-A re-review, `ccdc53eb`, ledger `55ad7621`): §4 now DEFINES
>   `authority`/`extract_override` and the kills' exceptions; step 3 gets the `Unwired` arm; the §9
>   fixture "move" retracted; 15 not ~13; G5/G6. **A third re-review is RUNNING** → report at
>   `design/agent-reports/2026-09-05-fable-plan-review.r2-fold-review-r3.md`. Persist, ledger, fold;
>   at 0C/0I **design r2 §10 step 1 begins** (header fields on 37 maps; the `deny_unknown_fields`
>   structs gain `irs_stem`, `versioning`, `template_sha256`, `authority?`, `extract_override?`,
>   `instructions`, `instr_pages?`, `line_set`, `attachment_sequence?`; the two-way test through
>   `irs_stem`; the four self-contained kills, each planted red first).
>
> ## ★★★ STATE AT 61bfae16 (2026-09-05, latest) — what landed after the two reviews
>
> - **Fold A** of the r2 fold review: `18027b03` (ledger `c0eb7f32`, 12/12 hold). Design r2 §4/§9/§10
>   rewritten so step 1 is executable (irs_stem join; `line_set` = revision, many-to-one; four kills
>   at step 1, the `line_set` kill at step 3; manifest join expected red on 6/37 with the
>   `authority = "not-yet-archived"` header as the excuse); the sibling documents aligned; R28 +
>   §6 rule 18 (1099-DA) registered; FR-54. **A re-review of this fold is RUNNING** → report at
>   `design/agent-reports/2026-09-05-fable-plan-review.r2-fold-review-r2.md`. Persist it verbatim,
>   ledger it, fold it; when it is 0C/0I, **step 1 begins.**
> - **Fold B** of the strategy review: `3de5c272`. The five OWNER decisions are listed in
>   `ROADMAP_STATUS.md` §0a "OWNER DECISIONS PENDING" (S1 rehearsal / S2 real profile / S6 ceremony
>   / S7 oracle fallback / S9 TY2017) — **not actioned; put them in front of the owner.** The four
>   non-owner items are in §3's NOW row (FR-46 1099-DA input by end of Sept; machinery hard stop
>   2026-10-31; two real-data runs; 4868 + 1040-V + a physical print).
> - **FR-47's statute archived**: `58200a59` — `26USC_s55.html`, `PLAW-119publ21_OBBBA.pdf`
>   (§70107(c) "substituting '50 percent' for '25 percent'"), `26USC_s55_OLRC-prelim.html`; a
>   `public-law` archive shape and the class-closing guard `every_document_under_primary_sources_has_a_shape`
>   (observed red). `AmtParams` TY2026 itself is NOT yet encoded — that is the next NOW build item
>   after step 1 (or in parallel; it touches `tax_tables.rs` only).
> - **taxcalc 6.7.2 → 6.8.2** in `.venv`: `61bfae16`. #3108 is fixed upstream; rule 12 applied
>   (gap #1 deleted, `TAXCALC_FLOOR` refuses older). `verify_f6251.py` 0 unexpected / 3 known.
>   ★ The oracle-sweep goldens were generated under 6.7.x and carry the understated AMT for
>   standard-deduction households — regenerate DELIBERATELY (rule 13), not to make a red green.
>
> ## ★★★ TWO MORE REVIEWS LANDED AND ARE PERSISTED (2026-09-05, late) — read before building
>
> 1. **The r2 FOLD review** — `design/agent-reports/2026-09-05-fable-plan-review.r2-fold-review.md`
>    (opus, independent; persisted `d32f7246`): **0C / 7I / 5M**. The fold was faithful; what it missed
>    is the SIBLING DOCUMENT (I1/I2/M2 folded into r2/CLAUDE.md/ROADMAP while the quoted sentences
>    still stand in `TY2026_PORT_REPORT.md:257,300,274` and `LONG_RANGE_PLAN_filing.md:467,489`), and
>    **design r2 §10 step 1 is NOT executable as written** (F4 namespace mismatch with
>    `emitted_form_years()`; F5 `line_set` has two definitions that disagree for ~13 maps; F6 one kill
>    needs step 3; F7 the manifest join reds on 6/37 rows — all five TY2017 + `forms/2024/f8283.pdf` —
>    with no excuse slot; F8 the 1040 has no attachment sequence). F9 (statute location) is already
>    fixed in `58200a59`. **Fold this BEFORE typing the first header.**
> 2. **The STRATEGY review** — `design/agent-reports/2026-09-05-fable-strategy-review.md` (Fable,
>    owner-requested; persisted `297ab7db`). Verdict: files TY2026 on the EXTENSION, not April, and
>    only if the Sep–Dec slack goes to the owner's real data rather than the port generator. The
>    single most likely failure: **the lived journey is scheduled last** — the owner's exchanges
>    report BASIS on 1099-DA for the first time (~2027-02-16) and the tool has nowhere to put it.
>    Nine ranked improvements S1–S9 and a month-by-month calendar. **Several are OWNER decisions**,
>    not implementer calls: S1 (un-pause a TY2025 REHEARSAL slice — diff against the return filed
>    outside, never mailed), S2 (two questions: the real 2026 income/venue/deduction set, and state of
>    residence), S6 (cut document-review rounds to one; fan-outs ≤ 6), S7 (pre-rule the oracle
>    fallback), S9 (drop TY2017). Non-owner items (S3 1099-DA input, S4 machinery hard stop 10-31,
>    S5 real-data runs at the estimate dates, S8 physical print rehearsal) are folded into
>    `ROADMAP_STATUS.md` §3 and `FOLLOWUPS.md` as they are actioned.
>
> **NEXT: the build starts at design r2 §10 step 1** — header fields on the 37 existing maps,
> required-field parse, `glob == emitted_form_years()` both ways, each §4 kill planted first;
> nothing consumes it yet. Red on any disagreement is the deliverable. In parallel (NOW bucket):
> FR-47 `AmtParams` TY2026 + archive the statute. The owner's open decision is unchanged: whether
> TY2017 ships at all.


> ## ★★★ THE LIVE ROADMAP TRACKER IS `design/ROADMAP_STATUS.md`
> **Read that first.** It is the progress ledger for `design/LONG_RANGE_PLAN_filing.md` and is updated
> on every task that closes — P1's T1–T7, P2's per-form artifacts, and what still blocks P3. This
> section below is the 2026-09-04 snapshot and is **historical**; where the two disagree, the tracker
> is current.
>
> As of **2026-09-05**: **Phase 1 is T1–T7 COMPLETE** — Schedule 1-A computes end to end, reaches
> 1040 line 13b, and has a per-part two-oracle census. P2 went from **5 to 15** of 17 TY2025 bundled
> form artifacts. P3 is untouched — the TY2025 fail-closed gate stays until `FullReturnParams` land.
>
> ## ★★★ TARGET CHANGED 2026-09-05 — TY2026 IS THE FIRST FILED YEAR; TY2025 IS PAUSED
>
> Owner ruling. The reasoning: the plan's own calendar had already concluded TY2025 by 2026-10-15 is
> unreachable while **TY2026 in the 2027 season** is the first return btctax could file ON TIME, and
> the owner's TY2025 return was completed outside this project — so a late TY2025 filing buys
> nothing. **TY2027 timing is explicitly acceptable.**
>
> ★★ And the sharper framing, in the owner's words: *"what we need most is the scaffolding … the year
> to year transition for every form we intend to support, not necessarily a completed 2025."* So the
> deliverable is **year-transition MACHINERY** — every year-dependent thing a lookup that REFUSES an
> unprepared year, never a hardcode with a fallback.
>
> ★ Almost no TY2025 work is wasted: the Schedule 1-A compute, the input surface, the refusals, the
> emitter and every instrument are TY2026 requirements too. What pauses is the TY2025-SPECIFIC form
> assets and the TY2025 fail-closed gate.
>
> Progress toward it: `design/TY2026_WORK_LIST.md` (computed per-form deltas), `xtask form-delta`
> (the handoff is a diff), 16 archived TY2026 drafts with geometry, and the draft/final discriminator.

> ## ★★★ NEXT YEAR: `design/TY2026_PORT_REPORT.md`
> A six-lens recon fan-out (reports: `design/agent-reports/2026-09-05-ty2026-port-*.md`) answered what
> it takes to carry this work to TY2026. **27 wrong-number risks, 3 Critical**, each classified
> REFUSES / DORMANT / UNGUARDED / LIVE rather than as a binary "fails closed".
>
> Its headline is the useful surprise: **TY2026's NUMBERS are largely published already** (Rev. Proc.
> 2025-32 plus the OBBBA statute), and `btctax report --tax-year 2026` computes today. What waits on
> the IRS is the **FORMS** — and therefore everything that must be *transcribed from* a form rather
> than looked up. **One item needs an owner ruling**, not more recon: §6 rule 1 forbids encoding the
> draft-derived TY2026 constants even though two lenses show them arithmetically forced; the schedule
> shortens materially if the owner disagrees.

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

- ★ **`.claude/worktrees/` grew to 270 GB** (21 leftover agent worktrees, each with a `target/`).
  Cleaned 2026-09-05: 20 merged+clean removed, 1 unmerged inspected (all four commits already on
  main by content/subject) and removed; 21 throwaway `worktree-agent-*` branches deleted. Check
  `du -sh .claude/worktrees` after any fan-out; `git worktree list` should be 1 line at rest.

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
