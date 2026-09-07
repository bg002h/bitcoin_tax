# Brief — interview build T3: the document census, the direction tables, the answer panel

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore` files you did not
create (revert a plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E
'<filter>'` (never `cargo test`, never `--release`, never the whole workspace — the controller's gate
runs `make check`); `cargo fmt --all` and a clean
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee lands with a kill seen red once (plant, observe, revert); the report
says how, with the red text. Every pinned number moved: old → new with cause. Process in force
(owner S6): this build gets ONE seam review and ONE re-verification after you.

## The contract
`design/SPEC_interview.md` r2: **§7 row T3** (the task and its kills), **R3** (the census: one
tri-state per document type, the three rules, unsupported rows refuse with §2.2's sentence),
**§2.2** (the exit sentences, verbatim), **R2.2** (`covered_by` on every `unmodeled` census entry of
the seven forms; the DIRECTION RULE with its `[direction]` table derived from part headings; the
reach check on a `QuestionId` cover; the residual attestation names every line it covers), **R12**
(`interview_state()` with the seven states, derived from the registries, never from
`screen_inputs`), **R14**, **R15** (the grep-KATs), **§4.2** (`income answer` prints the panel first
and last), **§5.1**, **§5.7**, **§8**. Build AS WRITTEN; the tree's real names win; deviations
recorded. What T1 and T2 already built is the floor: `provenance.rs` (`AnswerRecord`, `AnswerState::
Declined`, `answer_log`, `prompt_hash`, `DocumentKind`, `LEAF_SOURCE`) and `Production::Collected
{ from }` with `xtask line-coverage` / `box-census`. `provenance.rs:284-292` names `interview_state()`
as the reader of `Declined` — this task is where that reader lands.

## Scope boundary — what T3 does NOT build (each is another row of §7)
- The **paired document-less income questions** (`w2_wages_without_w2`, `interest_or_dividends_
  without_1099`, `state_refund_without_1099g`), `schedule_b_filer_records`, and the move of
  `itemized_prior_year` — **T5** (§7 T5 row, R3 kills (f)/(g)).
- The 1099 sections themselves, the year gate (T4), the TUI pane and the commit modal (T12).
- Do not widen the residual attestation's scope; extend its PROMPT to name lines (below).

## What T3 delivers
1. **`ReturnInputs.documents: DocumentCensus`** (§5.1) — eighteen `Option<bool>` rows: supported
   `w2, int_1099, div_1099, b_1099, g_1099, form_1098, form_1098e`; unsupported `r_1099, ssa_1099,
   nec_misc_k_1099, k1, schedule_e_rental, s_1099, oid_1099, w2g, c_1099, a_1095, t_1098`. Each row is
   a `FormQuestion` in `FORM_QUESTIONS` (`questions.rs:43`) with its own `QuestionId`, its prompt the
   form's own attribution (R3's example), `#[serde(default)]` per field on the wire (§4.3), and a
   classifier class (A) entry (`classifier.rs` — no `..`, no `_`; R3 kill (e)). Fixtures gain
   `documents.w2 = Some(true)` and the rest `Some(false)` (§5.7); `maximal_fixture` answers every row.
   The three rules, as `screen_inputs` refusals and an `apply` guard:
   - `None` on a live row → `RefuseReason::DocumentCensusUnanswered { kind }` (UNANSWERED class) —
     via the `FormQuestion.unanswered` slot, so it is one registry entry, not a new tier.
   - `Some(false)` with rows present → `DocumentCensusContradicted { kind }` (INVALID), and
     `apply(SetField(No))` while rows exist is an `ApplyError` (the `DeleteSection` I-10 precedent).
   - `Some(true)` with zero rows → `DocumentDeclaredNotTranscribed { kind }` (UNANSWERED class).
   - `Some(true)` on an unsupported row → UNSUPPORTED with **§2.2's sentence verbatim** (one variant
     carrying `kind`; the message table keyed by kind; kill (d) checks the exit's form number).

   **Controller decision on the rows whose section does not exist yet** (record it as a deviation
   for the review): the "rows present" count is a `fn rows(&ReturnInputs, kind) -> Option<usize>`
   — `Some(n)` for `w2` (`w2s: Vec<W2>`, `return_inputs.rs:976`), `None` for a kind with no section.
   For `int_1099, div_1099, b_1099, g_1099` (nothing collects them today; T5 builds the screens) the
   row is **live and `Some(true)` refuses UNSUPPORTED** naming T5 — a filer with interest today
   refuses rather than under-files (§2.2's rule), and T5 flips the row exactly as T13 flips
   `nec_misc_k_1099`. For `form_1098` and `form_1098e` the amount IS collected today by a scalar
   (`mortgage_interest_1098`, `return_inputs.rs:649`; `student_loan_interest_paid`, `:725`), so a
   `No` on the row would contradict an entered amount: those two rows are **not live until T9 / T5**
   replace the scalars (liveness `|_| false` with a comment naming the task; §5.1's
   `schedule_a.is_some()` liveness lands with T9). Kill: `int_1099 = Some(true)` refuses naming T5;
   `form_1098 = Some(true)` on any fixture is not live and asks nothing.
2. **`covered_by` on every `unmodeled` census entry of the seven forms**, per year where the map
   exists. Controller-measured counts of `rule = "unmodeled"`: 2024 — f1040 46, f1040s1 56, f1040s2 52,
   f1040s3 31, f1040sa 12, f1040sb 2, schedule_d 6 (205); 2025 — f1040 0, f1040s1 no map, f1040s2 54,
   f1040s3 29, f1040sa 8, f1040sb 2, schedule_d 0 (93). Value `"Advisory::X" | "RefuseReason::Y" |
   "QuestionId::Z"`; the join KAT reads the three enums' SOURCE (`advisories.rs:43`,
   `return_refuse.rs:36`, `questions.rs:76` — the `classifier.rs:950` `include_str!` technique;
   `btctax-forms` depends on `btctax-core`, so the KAT may live beside `field_census.rs:170`'s
   text-scanner or in core reading the maps by path — your call, one place). The census refusals
   you create in (1) are the natural cover for §2.2's lines (1099-R → 1040 4a–5b; SSA → 6a–6c; K-1 /
   Schedule E → Sch 1 line 5; W-2G → 8b; 1099-C → 8c; 1095-A → Sch 2 1a / Sch 3 9; 1098-T → Sch 3 3;
   1099-K → the Sch 1 1099-K line; 1099-OID → 2a/2b).
3. **The `[direction]` table** per form per year, each key asserted VERBATIM against
   `design/forms/extract/<stem>--<year>.txt` before use; an entry no key places **reds**, never
   defaults. Controller-measured in the extracts (line numbers): 2025 — f1040 `Income` is the
   left-margin caption printed as the leading token of the line-1a row (`f1040--2025.txt:57`), not on
   its own line; f1040s1 `Part I       Additional Income` :15, `Part II     Adjustments to Income`
   :60; f1040s2 `Part I       Tax` :15, `Part II      Other Taxes` :45 (and `(continued)` :76);
   f1040s3 `Part I       Nonrefundable Credits` :15, `Part II      Other Payments and Refundable
   Credits` :41 (both `Overstates`; keying on the parts is finer than the spec's title and equally
   derived); f1040sa title :5; f1040sd three parts :19/:44/:75; f1040sb title :6. 2024 — s1 :15/:58,
   s2 :15/:46, s3 :15/:40, sd :21/:46 (+Part III), and **`f1040--2024.txt` has no `Income` token at a
   line start** — read that extract yourself and record the key you find; if the 2024 text layer has
   no separable caption, do NOT fall back to the title (Form 1040 mixes directions — lines 1–9 versus
   25–38) — red, stop that form's table, and report it. Placement of an entry into a part: by the
   entry's `line` and the part's line range read from the extract, recorded in the map (e.g. a
   `part = "…"` key) or computed — either way asserted, never hand-typed per entry.
4. **The join rules** (R2.2): `Understates` ⇒ `QuestionId` or `RefuseReason` only (an `Advisory`
   reds); a `QuestionId` cover is checked for REACH — the covering question's `prompt` contains the
   entry's line caption keyword (the leading quoted phrase of `reason`, or an explicit `names = "…"`
   key); a `RefuseReason` cover is existence-checked; `Overstates` keeps existence for all three.
5. **The residual attestation's prompt** (`questions.rs:547`, `OtherOutOfScopeIncome`) **extended to
   name every Understates line it is claimed to cover, in the words the filer reads** — the trailing
   *"or anything else it never asked about"* covers nothing. Enumerate from the maps, not from
   memory; the report lists each line → the phrase added. Keep the attestation compound (widening an
   exemption is never the safe edit).
6. **`interview_state(ri: &ReturnInputs) -> InterviewState`** in core (R12) walking `FORM_QUESTIONS`,
   `SKIPPABLE_QUESTIONS`, `DEPENDENT_GATES` × rows, plus the declared-document/rows invariant, with
   the seven states exactly as R12's table: counted (not live; live+answered+hash unchanged), blocking
   (class A unanswered; class A hash-mismatch), forgoing (class B unanswered; class B hash-mismatch;
   `Declined` marked *(declined)* with size where computable), refusing (an answered row whose answer
   refuses — derived from the `FormQuestion`'s own refusal), waiting (a prompt that needs a
   `FullReturnParams` figure the year lacks — model the slot now even if T7's `gross_income_under_
   limit` is the first occupant). `answered` / `not_live` counts. No progress bar, no persisted
   "remaining" (R15).
7. **`income answer` prints the panel** before the first question and after the last (§4.2;
   `answer.rs:53,142`); the no-brick test (`answer.rs:531`) extends: answering every blocking item
   through its own setter empties `blocking`, and `refusing` is empty before `screen_inputs` passes.
8. **R15 grep-KATs**: no `serde_json::Value` reflection in `btctax-input-form`; no `progress` /
   `remaining` field; no registry prompt containing *transfer* / *lot* / *FMV*.

## Kills (each seen red once; the report quotes the red)
R3 (a)–(e) (see (1)); the census join red on a planted bad variant; **red on `covered_by =
"Advisory::EicOmitted"` on Schedule 1 line 2a (`f1040s1.map.toml:87`) while the same advisory on a
Part II entry stays green**; **red on `covered_by = "QuestionId::OtherOutOfScopeIncome"` on line 8h
(`:101`) while the prompt lacks *"jury duty"*, green once added**; **red when one part heading is
deleted from a `[direction]` table** (every entry of that part unplaceable — not `Overstates`); N
unanswered live items ⇒ N listed in one call; `Declined` in `forgoing` *(declined)* and never in
`blocking`, `Given` removes it; a hash-mismatched record in `blocking`/`forgoing` with the
changed-wording reason; a census `Some(true)` on an unsupported row in `refusing` before commit; the
extended no-brick test; the three R15 greps each red on a planted line.

## Constraints
- Prompts are the form's words; every refusal names its exit; no default answers (an entry is
  testimony). `#[serde(default)]` on every new field. `income import` remains the only other writer
  of `ReturnInputs` — it writes no census answer it was not given.
- Do not touch `design/forms/extract/` or the archives; read them.
- If context runs short: leave the tree compiling, fmt-clean and green on every crate you touched,
  and state precisely which numbered item above is unfinished — the controller dispatches a
  continuation from your report, never from the tree.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T3-implementation.md`: per numbered item what
landed (files, functions, the census rows and their liveness), every deviation with its reason, the
2024 f1040 direction key you found, the attestation prompt's added phrases (line → words), every
kill with its red text, every pinned number moved (old → new, cause), the suite lines per crate.
Return only a 4-line summary plus the report path.
