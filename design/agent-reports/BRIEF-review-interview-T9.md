# Brief — seam review of interview build T9 (real estate)

You are an independent, adversarial BUILD REVIEWER in your own git worktree at the commit named at
dispatch (the T9 build commit). Read-only for the record: every plant is made in YOUR worktree and
reverted (`git checkout -- <file>` is fine there); no commits, no subagents.
`export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo command; scoped
runs only (`cargo nextest run --locked -p <crate> -E '<filter>'`); the instruments are `cargo run -q
-p xtask -- box-census` / `line-coverage` / `census-join`; the oracle harness per
`scripts/oracle/README*` (`.venv/bin/python`). **Environment, not findings:** six `form_delta` tests
and `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a
PDF-less worktree with a redirected target dir.

## The one question
Can a standard-deduction filer be asked or refused over a mortgage they do not deduct, can a
figure the tool holds reach a line it does not compute (box 4 beside a blank 8z), and can a home
sale print anything the instruction's own answer does not establish? Concretely: (a) the Form
1098 section and its three declarations are live iff `schedule_a.is_some()`; (b) the ceiling
warning is aggregate and status-adjusted and never writes; (c) box 4 > 0 refuses naming Schedule 1
line 8z; (d) 8e = 8a + 8b + 8c and the two-oracle sweep reconciles; (e) the 8-branch home-sale table
has exactly one blank branch; (f) every 1098 box on both archived editions is censused and joined.
Not a fresh audit of T1–T8 or T16; not a spec re-review.

## Settled (machine-verified by the controller; do not re-measure)
The T9 build report `2026-09-07-build-interview-T9-implementation.md` lists the 1098 census per
edition, the ceiling figures with cites, the home-sale table as tested, kills and deviations; the
controller has confirmed the suite line it states and the instruments' lines. Spec:
`design/SPEC_interview.md` r2 R8 (mechanism + kill), §5.5, §5.1's `form_1098` liveness sentence,
§7 row T9, R4/R2.2; `i1040sca--2025.txt` (8a/8b/8c `:1058-1139`; the co-borrower share
`:1071-1080`; the refund `:1067-1070`; the 8396 subtraction `:1091-1096`; the $50 penalty
`:1117-1119`), `i1040sd--2025.txt:313-347` (the home sale); Pub. 936 for the ceilings. Prior builds:
T2's per-edition `revision_in_force` (Form 1098 Rev. January 2022 for TY2024, Rev. April 2025 for
TY2025/26); T3's `document_census.rs` (`form_1098` non-live until T9); T5's section/census-join
pattern; T4's import tier; T4b's opener.

## Seams
1. **Liveness on the election, not the document.** A standard-deduction fixture holding a
   $900,000 1098: no census row live, no declaration asked, no refusal, nothing printed from it;
   the same fixture with `ScheduleAInputs` present: the row and the three declarations live,
   `MortgageWithinDebtLimit = Some(false)` refuses. Plant: key liveness on the document's presence
   → red? `AmtQualifiedDwelling` (Form 6251 line 3) keyed the same way.
2. **The ceiling.** Σ box 2 across rows against the year's params by box 3 vs 2017-12-16, MFS
   halved: $900k/2019 warns, $700k silent, two × $500k warn, one × $500k silent, MFS at $400k warns
   where Single does not; the warning never writes (row byte-identical); the figures and cites in
   `FullReturnParams` for TY2024/25 match Pub. 936 / the Schedule A instructions.
3. **Box 4 and the co-borrower.** Box 4 = $1 refuses `MortgageInterestRefundNotComputed` with
   *Schedule 1 line 8z* in the message; box 4 = 0 does not; `income import` of a box-4 row refuses at
   import and writes nothing; `other_borrower_paid_interest = Some(true)` refuses
   `SharedMortgageInterest` with the *"only your share"* sentence.
4. **8a / 8b / 8c / 8e.** 8a = Σ(box 1 + the points box) — which box carries points on the 1098
   (read the extract) and is the sum right; 8b rows print the recipient's name/TIN/address on the
   dotted lines (the TY2024 map cells `f1_17`/`f1_19` and the TY2025 twins read back); an empty
   `recipient_tin` refuses; 8c's help carries *"generally deductible over the life of the loan"*;
   8e = 8a + 8b + 8c in the printed chain; the sweep reconciles with `e19200` absorbing 8b/8c (run
   the harness once; if OTS is unavailable say so and mark the claim unverified).
5. **The 8396 gate and the home sale.** `claiming_mortgage_interest_credit` live iff any 1098 or
   8b row, `Yes` refuses naming Form 8396; the home-sale table: all 16 combinations of the three
   tests × the `s_1099` census row (`None`, `Some(false)`, `Some(true)`) — exactly (Y, Y, Y,
   `Some(false)`) is blank with the four answers on record; every other branch refuses with
   *Pub. 523* and *Form 8949 code H* in the message; no amount is asked anywhere (grep the new
   fields for a `Usd`).
6. **The census and the opener.** Both 1098 editions' boxes carry exactly one decision each, joined
   to `FieldId`s by `box-census`; a TY2024 row verifies against the 2022 edition and a TY2025 row
   against the 2025 edition (plant a caption only the other edition prints → red); a prior lender is
   seeded pre-named with every box blank; `LEAF_SOURCE` covers every new leaf; the coverage fixture
   carries the 1098 row (box 4 = 0), an 8b row, 8c and the home sale on its blank branch.

## Severity
A standard-deduction filer asked or refused over a 1098, a figure reaching a line the tool does
not compute, a home-sale branch printing anything, a wrong 8e, a census gap, or a kill that does
not red is **Critical** or **Important**. Secret-handling defects are never Critical/Important.
Design opinions outside the one question are Minor at most.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T9-review.md` in your worktree: Commands;
Summary; findings ordered by severity with Where / What is wrong / Evidence (the plant and the
observed output) / Minimal change; "Seams checked clean"; `Counts: C=<n> I=<n> M=<n> N=<n>`.
Return ONLY a 3-line summary plus the path.
