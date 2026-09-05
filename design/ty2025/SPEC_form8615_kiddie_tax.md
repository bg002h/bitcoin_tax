# Form 8615 (kiddie tax) — the screen, its inputs, and its refusals — SPEC

**Status: r3 — the r2 review folded (1C/2I/4M/1Nit, all confirmed below) and the OWNER RULING of
2026-09-05 on OQ-5 folded. NOT YET RE-REVIEWED ⇒ DO NOT BUILD.** (r1 written 2026-09-05; r2 folded
2026-09-05; r3 folded 2026-09-05; branch `main`.) Fixes `FOLLOWUPS.md` FR-29 (`FOLLOWUPS.md:5722-5744`), a
**★★★ CRITICAL understatement path** whose owning phase is *"BEFORE any first filing"*.

Passes an independent review loop to **0 Critical / 0 Important** before an implementation plan. A
fold is authorship and re-earns the gate: r2 is unreviewed text, and on this branch four consecutive
rounds found their Importants in the *previous* round's fold.
`design/ty2025/SPEC.md`'s parent decisions bind here unless restated.

**Scope.** The *screen* — deciding whether Form 8615 is required, collecting what the form's own
conditions ask, refusing when it is, and (r2, per the owner ruling) the one narrow path out of the
dead end where the tax system itself provides none. **Filling Form 8615 is out of scope** and stays
out: the form needs the parent's taxable income and every sibling's §1(g) amount, none of which
btctax ever sees.

---

## r3 FOLD (2026-09-05) — 1 Critical / 2 Important / 4 Minor / 1 Nit, one lens, plus the OQ-5 RULING

**Review persisted verbatim at `design/ty2025/reviews/SPEC_form8615-review-r2.md`.** Every claim was
re-verified against the primary source or by running it before folding. **Nothing was corrected —
all eight findings held.** The r2 fold below is left as written except for its *addresses*, which
were re-resolved with everything else (C-1); its prose record of what r2 verified is untouched.

★★★ **C-1 IS MINE, NOT THE SPEC'S, AND IT IS AN ADDRESS FAILURE — NOT A TRANSCRIPTION FAILURE.**
`c7819f8c` (*"fix(extracts): I broke the repo's own extraction convention on both instruction
booklets"*) landed **one commit after the r2 fold** and regenerated `i8615--2025.txt` from 712 to 1307
lines: I had extracted a two-column instruction **booklet** with `pdftotext -layout`, which splices
the right column into the middle of left-column sentences, where `design/forms/README.md:58` says
plain `pdftotext` for a booklet and `-layout` only for a **form**. `f8615--2025.txt` gained a four-line
provenance header (59 → 63). Nothing about the spec's reading of the form was wrong, and **every
quotation in this document was and is verbatim** — what moved was where each sentence lives.
**Say that plainly so the next reader does not re-audit the transcription.**

| # | what was claimed | what I verified, and how | what changed |
|---|---|---|---|
| **C-1(a)** | 24 of the 27 `i8615`/`f8615` citation sites resolve to unrelated text | **Confirmed and re-resolved, machine, not by eye.** A script pulled every `(i8615\|f8615\|i8275\|f8275\|i1040gi)--YYYY.txt:a[-b]` in this file and printed the text at each range in the CURRENT extracts; each cited *content* was then located with `grep -n`. `i8275--2024.txt` / `f8275--2024.txt` were **not** in `c7819f8c` (`git log -1` ⇒ `583f7213`) and all eight of their ranges still resolve (`i8275:202-214`, `:352-374`, `:355`, `:361-362`, `:371-374`, `:378-388`, `f8275:6`, `:9`); `i1040gi--2025.txt` likewise — every range this file cites was printed and read, and all are exact | every i8615/f8615 range re-resolved throughout, **including the r1/r2 fold tables** — an address is a pointer, and the content it names did not move. §1's authority table now records 1307/63 lines and `c7819f8c` as the extract of record |
| **C-1(b)** | every "left column" / "right column" annotation is now a false statement about the artifact | **Confirmed.** Plain `pdftotext` reads the columns in order. §1's own example was doubly wrong: `:139-141` is a page footer, and the two contents it named are 73 lines apart (`:193-195` and `:266-267`) | **every column annotation deleted**, and §1's ★★ two-column warning replaced by a note saying the extract is in reading order |
| **C-1(c)** | **P-2a must be DELETED, not re-scoped** — it builds a column-aware extractor for a defect `c7819f8c` already eliminated | **Confirmed by re-running the measurement**, with `cite_check`'s own `quoted_spans` / `fragments` / `normalise` over this document against the five extracts, **no column splitting**: `110 checkable fragments, FAILING 20` — and 40 against the archived `-layout` extracts, so the regeneration alone did what the column split was for. All 20 residuals are self-citations (our prose, this spec's own prompt/help strings, a `questions.rs` prompt, the r1 draft, the owner ruling, the statute); **zero form quotations fail** | **P-2a deleted** and replaced by the measured record, with the archived-vs-current table. ★ Its B1 pairing could no longer be written honestly either: it existed to prove a *correct* quotation passes where the raw extract made every correct one fail |
| **C-1(d)** | §9's *"P-2a is a hard prerequisite of G7"* is false | **Confirmed by running G7 itself.** All eight clauses through `cite_check::normalise`, against the **raw** current extracts and against the §4.1 string each belongs to: **8/8 and 8/8**; the three structural counts on the condition-3 prompt come out 2 / 1 / 1 / 1, the exact values §9 claims | §9's prerequisite paragraph struck and replaced with the measurement; the per-clause source column **kept**, for the reason §1.1 gives (i8615's *"and under age 24"* vs i1040gi's *"but under age 24"*) |
| **C-1(e)** | §1's authority-table metadata (712 / 59 lines, both attributed to `29e47a0b`) is stale | Confirmed: `wc -l` ⇒ 1307 and 63; `git log -1` ⇒ `c7819f8c` for both | rows rewritten, with the extraction flag recorded per file |
| **I-1** | the dead-end leaf's polarity is **inverted** between prompt and field, the spec never says which side inverts, and nothing reds if it is written straight through | **Confirmed, on both consumers.** `skippable_tristate!` destructures `FieldValue::TriState(Some(b))` and calls `set_bool(ri, b)` (`crates/btctax-input-form/src/spec/registries.rs:83-91`); `answer.rs`'s `YesNo` branch calls `(sk.set_bool)(&mut ri, v)` on `parse_yes_no`'s value (`crates/btctax-cli/src/cmd/answer.rs:178-201`). **No accessor in `SKIPPABLE_QUESTIONS` contains a `!` today** — `grep` over all 16 entries — so the aligned pair is the obvious transcription, and it certifies the filer who answered **YES**. The delegation test (`crates/btctax-input-form/src/spec/mod.rs:368-393`) round-trips through the pair, so it passes on a consistently-inverted pair *and* on a straight-through one | §4.1 **writes both accessors out** with the reason; the §4 doc comment cross-references it; **§9 G14(a) gains a polarity row** driven through `(f.set)(…, TriState(Some(true)))` and asserting the return still REFUSES, whose named mutation is the straight-through pair itself |
| **I-2** | the 2026-09-05 OQ-5 ruling is only half folded — R-4 never names TAS, and §13 still says it *"needs an owner ruling"* | **Confirmed against the ruling**, `design/OWNER_DECISIONS_2026-09-04.md` *"OWNER RULINGS, 2026-09-05"* (`92014cd1`): *"just refer user for tas and tell them good luck."* The non-widening half **is** fully discharged (the §6.3 conjunction, §4.2's liveness on `Some(CannotKnow)`, §13's refusal of the disjunct) — only the referral was missing. ★ **The spec's TAS sourcing is CORRECT and I re-read the page to confirm it**: the three bullets are at `i1040gi--2025.txt:157-159` and cover a filer who has already contacted the IRS; the limb that covers this one is the third of the introductory sentence at `:153-154`; the contacts are at `:162` and `:166`. The ruling's own *"`:155-166`"* is the loose range, and **nothing here was "fixed"** | R-4's detail gains the TAS paragraph on the *"if you CAN"* branch; §13 restated as **RULED**, not open; **new §9 G15** asserts both halves on one fixture — still refused **and** the detail names TAS |
| **M-1** | G13's first half describes a capability `classify` does not have | Confirmed by reading it: `Census` (`classifier.rs:64-67`) holds only `declarations` and `exemptions`; `exempt<T>` **discards the leaf** (`:75-77`); `declaration` is typed `&Option<bool>` (`:71-73`). The classifier makes answered-ness structural by the type and the no-`..` destructure — it never scores a value | G13's scoring clause **deleted**; the gate clause kept, with the compile named as the actual protection |
| **M-2** | the CLI's input grammar for the third answer is unspecified, and the obvious parser rejects the phrasing the prompt gives the filer | Confirmed: `parse_enum` (`parse.rs:87-92`) is an exact `options.contains(&raw)`, and `answer.rs`'s skippable loop (`:155-208`) uses `parse_yes_no` / `parse_date` and never calls it. The prompt says *"Answer YES, NO, or CANNOT KNOW"*; `"CANNOT KNOW"` is not `"CannotKnow"` | §4.1 now specifies what the `Choice` arm accepts, case-insensitively, with the prompt left alone and the parser accommodating it |
| **M-3** | §1.2's reason for quoting *support* as two spans is no longer true | Confirmed: the interruption is the page footer `Oct 22, 2025` and a blank line at `:66-67`, not a column break. The quotation structure survives | the reason corrected in §1.2 and in §1's ★ multi-source note |
| **M-4** | the fold's *"cite-check OK with 52 quotations"* does not describe this document | **Confirmed by running it**: 7 from `SPEC_schedule_1a.md`, 45 from `IMPLEMENTATION_PLAN_schedule_1a.md`, and `grep 8615 crates/xtask/src/cite_check.rs` is empty. There is no line-numbered-citation checker in `xtask` at all | §1 says so plainly, with the command's output pasted, and names the address half as §G-10's residue that P-1 does not close |
| **N-1** | `i8615:44-63` for *"the five conditions"* is loose | Confirmed: the conditions end at `:57`; `:58-61` is the holding (cited separately) and `:62-63` starts *Support* | tightened to `:44-57` at both sites |

**What did NOT change, deliberately.** The gate, the ladder, the three-valued type, the §6.3
certification and its conjunction, the Form 8275 content, the three warnings, and §6.3.5's TAS
sourcing. C-1 is a re-sourcing pass, I-1 adds two `fn` pointers and a test row, I-2 adds a paragraph
to one refusal and a test. **No refusal was relaxed and no exemption was widened.**

**Gate, at `b3f28563`.** The one executable fragment this fold adds — §4.1's two accessors — was
extracted, compiled with `rustc` against the real `SkippableQuestion` fn-pointer signatures
(`questions.rs:926-933`) and **run**, asserting that the filer's YES stores `Some(false)` and their
NO stores `Some(true)`. `cargo run -p xtask -- cite-check` ⇒ **OK — 51 quotations, all verbatim**,
exit 0 (all 51 Schedule 1-A's; this document is still not registered — M-4). `make check` ⇒
**2836 tests run: 2836 passed, 12 skipped**, exit 0.

★ **Read that 2836 against 2815, not as a change this fold made.** This fold touches **no `.rs` file
at all** — `git status` is one modified markdown file. `main` advanced three times underneath it
while it was being written (`edeb70b5` → `b83b3c7e` → `da877c47` → `b3f28563`: the FR-38 recon, the
FR-34 merge, and the Schedule 1-A T2 build), which is where both the +21 tests and the 52 → 51
cite-check total come from. **All 174 `file:line` citations in this document were re-resolved against
the tree at the final HEAD, 0 unresolved**, and the citations into the two files those merges did
change (`tables.rs:470` and `tax_tables.rs:134`, `:814-820`) were re-read line by line, not just
range-checked.

---

## r2 FOLD (2026-09-05) — 1 Critical / 4 Important / 5 Minor / 3 Nit, one lens, plus an OWNER RULING

**Review persisted verbatim at `design/ty2025/reviews/SPEC_form8615-review-r1.md`** (`c738ed77`).
**Owner ruling persisted verbatim at `design/ty2025/DECISION_form8615_no_path_self_certification.md`**
(`395ab293`). Every claim from both was re-checked against the primary source before folding; two were
**corrected**, and the corrections are recorded in the table rather than folded silently.

The Critical is a documentation race and its fix is substantive: `f8615--2025` and `i8615--2025` were
archived by `29e47a0b`, the commit *after* this spec was written, so §1's "there is no separate
`i8615`" was true when authored and false at HEAD. i8615 is the governing text for this form's own
terms, and it states FR-29's holding in one sentence this spec had been *deriving from an absence*.

| # | what was claimed | what I verified, and how | what changed |
|---|---|---|---|
| **C-1** | §1's sourcing of record is false at HEAD; i8615 states the FR-29 holding outright, defines *support* / *earned income* / *unearned income* **for this form**, and the spec borrowed Chart B's definitions, which disagree with i8615 on scholarships in **opposite directions** | **Confirmed, every limb.** `design/forms/extract/i8615--2025.txt` and `f8615--2025.txt` exist (712 and 59 lines as archived by `29e47a0b`; **regenerated to 1307 and 63 lines by `c7819f8c`, so every address in this row is the r3-re-resolved one** — see the r3 fold). The holding is at `i8615--2025.txt:58-61`; *support* at `:62-65` continuing at `:68-70`; *earned income* at `:266-267`; *unearned income* at `:28-35`. The scholarship split is real: `i1040gi--2025.txt:695` counts *"taxable scholarship and fellowship grants"* as **earned**, `i8615--2025.txt:32-33` counts them *"not reported on Form W-2"* as **unearned** | §1 rewritten around **four** authorities; §1.1 now carries i8615's own statement of the five conditions beside i1040gi's; **new §1.2** transcribes the three definitions; §2, §4.1 help and §8 R-3 requote from i8615; P-1 rewritten |
| **C-1(d)** | `return_1040.rs:990-995`'s component sum "omits **every** bolded category", and §5.2's direction claim survives only because of an uncited `OtherOutOfScopeIncome` refusal | **★ CORRECTED, and the correction matters.** The sum at `:990-995` **includes `sum_unemployment(ri)`** (`:994`) and `ri.sch1.state_refund_taxable` (`:993`), so "omits every category" is false — unemployment compensation is captured. The **load-bearing half is confirmed**: the remaining i8615 categories (rents, royalties, pension/annuity, scholarships not on a W-2, alimony, the taxable part of social security, trust-beneficiary income) are unreachable only because `QuestionId::OtherOutOfScopeIncome` (`questions.rs:546-563`) names them and its catch-all *"or anything else it never asked about"*, and `Some(true)` hard-refuses at `return_refuse.rs:995-997` | §5.2 now **cites and pins** that dependency (new G11), with the unemployment correction stated so the next reader does not re-derive it |
| **I-1** | the condition-3 *help* is the only filer-facing text that paraphrases the support/earned-income test, and it paraphrases the **wrong document** | Confirmed. Direction is over-refusal in every branch, as the review says — but by accident of which terms were dropped, not by design | §4.1's help requoted from `i8615--2025.txt:62-65`+`:68-70` and `:266-267` (r3-re-resolved addresses), scholarship carve-out included, and the review's "redeeming sentence" kept verbatim |
| **I-2** | §4.1's index-hazard reassurance is **false** — `skippable_tristate!` reads `SKIPPABLE_QUESTIONS[$idx]` for label/help/live/get/set — and G9 never names `registries.rs` | **Confirmed by reading the macro.** `crates/btctax-input-form/src/spec/registries.rs:67-95`; the fifteen literal call sites run `skippable_tristate!(0, …)` at `:254` through `(15, …)` at `:355` (index 2 is absent — SALT is deduped). `skippable_to_field` at `:457-477` is exhaustive over `SkippableId`, so the **variants** are compile-forced and the **indices** are not | §4.1's ★ note rewritten to state the hazard truthfully; G9 gains `registries.rs` |
| **I-3** | G7 cannot pass as written — three of five clauses differ in case, three span newlines, and "normalised" is never defined, so the cheapest repair is to gut the clause list | **Confirmed, and the definition already exists in-repo.** `crates/xtask/src/cite_check.rs:71-136` (`normalise`) folds Unicode punctuation, strips markdown, de-hyphenates across line breaks, replaces non-alphanumerics with spaces, collapses whitespace **and lowercases** — i.e. exactly case-insensitive + whitespace-collapsing | G7 now **names that function** as the definition rather than inventing a second one |
| **I-4** | R-3 tells the filer they meet *"all five"* conditions including *"a filing requirement"*, which §5.4 knows is false for two enumerated populations, and `KiddieTax` is a refusal no input can clear | Confirmed against `i1040gi--2025.txt:634-639` (Chart A single under 65, `$15,750`) and `:705` (blind dependent, `$3,350`) | §8 R-3 reworded to **disclose the assumption**; G6 gains the assertion |
| **M-1** | `form8615_screens_a_filer_who_is_nobody's_dependent` is not a legal Rust identifier | Confirmed (Rust 2021 reserved prefix) | renamed `…nobodys_dependent` |
| **M-2** | §4.2's *"Never the reverse"* holds for `Default` but not for a hand-built `ReturnInputs` with `tax_year > year` | Confirmed: `crates/btctax-core/src/tax/return_inputs.rs:688` is a plain `i32`, `:1081` defaults it to `0`, and nothing bounds a hand-built value | §4.2's claim narrowed and G8 asserts the **invariant**, not the direction |
| **M-3** | the condition-4 help states a false rule (*"the two together are what send you to Form 8615"*) | Confirmed — all five do, and condition 4's liveness does not read condition 1 | help rewritten |
| **M-4** | §7's fixture DOBs are unbounded and a 65+ date moves the §63(f) standard deduction on four oracle households; suggested band `[year − 64, year − 24]` | **Confirmed, and the band CORRECTED.** `born_early_enough` (`return_1040.rs:89-91`) is `dob <= Date(year − 64, Jan 1)`, so a **January 1** birth in `year − 64` *is* 65+. The safe band is birth **years** `year − 63 … year − 24` | §7 states the corrected band |
| **M-5** | §4.1's *"only permitted departure … three times"* understates the departures (hoisting, four recasings, one appended sentence) | Confirmed against `i1040gi--2025.txt:3932-3940` | §4.1's claim now enumerates all four departures |
| **N-1** | G9 cites one `93`; there are two | Confirmed: `coverage.rs:429` and `:434` | both named, and the count moves 93 → 96 (three new leaves, not two — see the ruling) |
| **N-2** | the registry-count test's **name** (`…has_five_entries…`, already stale at 16) and its message need editing | Confirmed at `questions.rs:1490` and `:1493-1496` | G9 names both |
| **N-3** | no QSS vector anywhere, though §5.3 calls QSS out by name | Confirmed | G4 gains a QSS row |

**★★★ THE OWNER RULING, and the type consequence the r1 review could not have seen.** Where the tax
system provides **no path** — the filer cannot identify their parents, so Form 8615's lines A/B/C
cannot be completed, §1(g)(3) cannot be computed, and i8615's IRS-request remedy is closed *by its own
required contents* — **the filer may SELF-CERTIFY, with warnings**, and btctax takes the position that
§1(g) is not established. Verified against all three primary sources before folding:

| the ruling's claim | primary source | verdict |
|---|---|---|
| the form's face requires the parent's name, SSN and filing status, none optional | `design/forms/extract/f8615--2025.txt:17-20` | **confirmed** — ★ the ruling cited `:8-11`; the correct address is `:17-20` |
| §1(g)(3) cannot be computed without the parent's taxable income | `legal/primary-sources/statute-irc/26USC_s1.html`, §1(g)(3)(A)(i) | **confirmed** |
| the IRS-request remedy requires the parent's **name and address** with no *"if known"*, and a statement that the child *"tried to get the information from the parent"* | `design/forms/extract/i8615--2025.txt:185-187` and `:193-195` | **confirmed** — SSN and filing status carry *"(if known)"*; name and address do not |
| the *"neither parent living"* exclusion is no more establishable than condition 4 | `design/forms/extract/i8615--2025.txt:60-61` | **confirmed** |

Both attached constraints are folded **structurally**, not as prose:

1. **Gated on the dead end, never offered generally** — the certification is unreachable until the
   filer has affirmatively answered condition 4 *"cannot know"* **and** attested the identity fact.
   §6's ladder reaches it at step 6 only; §6.1's table shows every other row unchanged.
2. **The filer attests to FACTS, never to a legal conclusion** — the two collected leaves ask whether
   the filer can know if either parent was alive, and whether they can give the IRS either parent's
   name and address. Both are testimony a filer can truthfully give. The **conclusion** — that §1(g)
   is not established — is btctax's, and it goes on Form 8275 in btctax's voice (§6.3).
3. **⇒ Condition 4 cannot be `Option<bool>`.** *"I don't know"* is a **third answer**, distinct from
   UNANSWERED: unanswered still refuses, *unknowable* unlocks the certification path. The type is
   specified in §4 and the registry consequence (a third `SkippableKind`) in §4.1.

**★ The Taxpayer Advocate Service was named in the ruling as a candidate with UNVERIFIED intake
criteria. It is now VERIFIED** against `design/forms/extract/i1040gi--2025.txt:153-154` and cited in
§6.3 — with the precise finding recorded there: the three *bulleted* criteria at `:157-159` do **not**
cover this filer; the limb that does is the third in the introductory sentence at `:153-154`.

**Open questions.** OQ-1 … OQ-4 are **answered** in §12 rather than carried; OQ-5 is new and records
one deliberate non-widening.

---

## 1. Sourcing of record — READ THIS FIRST

★ **r2 (C-1).** The r1 draft of this section said *"There is no separate `i8615` in the archive for
this cycle. `i1040gi` carries the whole test."* **That was true when written and is false now:**
`29e47a0b` archived `f8615--2025` and `i8615--2025` one commit later. It is corrected here rather than
footnoted, because a false sourcing line misdirects every downstream transcription.

**i8615 is the governing text for this form's own terms.** i1040gi's Form 8615 paragraph is a
*summary* inside the 1040 instructions; i8615 is the form's identically-numbered instruction document
(`CLAUDE.md`, *the answer is in the manual*: `fNNNN` → `iNNNN`). Where the two differ — and on
scholarships they differ in **opposite directions** (§1.2) — **i8615 controls for Form 8615**, and
i1040gi controls only for the Chart A/B/C filing-requirement test it is written for. Everything below
is quoted from the extracted text layer (`CLAUDE.md`, *Transcribe IRS forms — never paraphrase them*),
never from a rendered page.

| authority | extract of record | what it holds for this spec |
|---|---|---|
| **`i8615--2025.pdf`** (archived `29e47a0b`, **extract of record `c7819f8c`**) | `design/forms/extract/i8615--2025.txt` (1307 lines, `pdftotext` no flags) | the five conditions in the form's own voice (`:44-57`), **the FR-29 holding** (`:58-61`), *support* (`:62-65` + `:68-70`), *earned income* (`:266-267`), *unearned income* (`:28-35`), the January-1 chart with its `***` footnote (`:71-98`), the Form 8814 parental election (`:100-106`), and the **IRS-request remedy and its required contents** (`:176-198`) |
| **`f8615--2025.pdf`** (archived `29e47a0b`, extract of record `c7819f8c`) | `design/forms/extract/f8615--2025.txt` (63 lines, `pdftotext -layout` — it is a FORM) | the form face — lines **A/B/C** (`:17-20`), which are what §6.3's dead end is a dead end *against* |
| `i1040gi--2025.pdf` (`482e9c48`, `design/ty2025/SPEC.md:70`) | `design/forms/extract/i1040gi--2025.txt` | the five conditions addressed to the filer as *"you"* (`:3927-3944`) — the wording §4.1's prompts transcribe; the January-1 examples (`:3945-3951`); Chart A (`:625-666`), Chart B (`:691-728`), Chart C (`:732-733`); and the Taxpayer Advocate Service (`:147-166`) |
| `26 U.S.C. §1` | `legal/primary-sources/statute-irc/26USC_s1.html` | §1(g)(1) *the greater of*; §1(g)(2)'s three conditions, in which dependency likewise does not appear; §1(g)(3), which needs the **parent's** taxable income |
| `f8275--2024` / `i8275--2024` | `design/forms/extract/f8275--2024.txt`, `design/forms/extract/i8275--2024.txt` | the disclosure instrument §6.3 uses: Part I columns (`i8275--2024.txt:352-374`), Part II's content requirement (`:378-388`), and *adequate disclosure* / *reasonable basis* (`:202-214`) |

**Which document is quoted where, and why.** The **prompts** (§4.1) transcribe i1040gi, because that
is where the conditions are addressed to the filer in the second person (*"You were either…"*); i8615
states the same conditions about *"the child"* (`:44-57`), which a prompt cannot use without
rewriting. The **definitions** (§1.2), the **holding** (§2) and the filer-facing **help** transcribe
i8615, because those are §1(g)-scoped and i1040gi's are not. §9 G7 checks each clause against the
extract it is actually sourced from, named per clause.

**P-1 (plan task):** register this file with `cargo run -p xtask -- cite-check` by adding it to the
docs list and adding `design/forms/extract/i8615--2025.txt`, `design/forms/extract/f8615--2025.txt`,
`design/forms/extract/i1040gi--2025.txt`, `design/forms/extract/f8275--2024.txt` and
`design/forms/extract/i8275--2024.txt` to the haystacks in `schedule_1a_docs()`
(`crates/xtask/src/cite_check.rs:404-417`), so the check is a command rather than a promise.
★ **Two hazards P-1 must handle, both read out of the checker's source rather than assumed.**
(a) `quoted_spans` (`cite_check.rs:182-215`) treats **every** blockquote as a form quotation — there is
no `FOREIGN_SOURCES` or `§` escape on that path, unlike `plain_quotations` (`:249-263`) and
`inline_quotations` (`:318-323`). So the statute and owner-ruling quotations in this document must not
become blockquotes, or their sources must join the haystacks. (b) `cite_check` has **no fenced-code
awareness**, so the doc comments and prompts in §4/§4.1 fenced blocks are scanned by
`plain_quotations` only where the line carries an attribution marker — which is precisely why G7
exists as a separate instrument and P-1 does not replace it.

★★ **P-1 is therefore NOT a one-line registration, and saying so is the point.** This document quotes
three things the form extracts cannot authorise: the **statute** (§1(g)(1)-(4)), the **owner ruling**
(§6.3), and its **own** earlier drafts and the r1 review (the fold table above). `inline_quotations`
(`cite_check.rs:298-331`) checks every `*"…"*` span of six or more words (`MIN_WORDS`,
`cite_check.rs:335`) unless the surrounding *line* contains a `FOREIGN_SOURCES` marker
(`cite_check.rs:339-354`) — and neither *"the r1 review"* nor *"the owner ruling"* is in that list
today. So P-1 lands **three** changes, not one: register the document, add the five extracts as
haystacks, and extend `FOREIGN_SOURCES` for this document's non-form citations. P-1 is done when
`cargo run -p xtask -- cite-check` reports **0 unverified spans over this file**, and the plan records
the checked-span count so a later drop is visible.

★★ **r3 (M-4) — THIS DOCUMENT IS NOT UNDER `cite-check` TODAY, and no number quoted anywhere in this
spec describes it.** `cargo run -p xtask -- cite-check` reports exactly two documents —
`design/ty2025/SPEC_schedule_1a.md` and `design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md` — because
those are the only two `schedule_1a_docs()` names (`crates/xtask/src/cite_check.rs:405-417`, two
documents and two Schedule 1-A fixtures). **This spec is not among them and neither is any 8615
extract**: `grep 8615 crates/xtask/src/cite_check.rs` is empty. So the r2 fold's *"cite-check OK with
52 quotations verbatim"* was a true statement about **Schedule 1-A** and said nothing whatever about
this file — and the total is not even stable, since it tracks whatever those two documents currently
quote (it was 52 and is 51 as this fold is written, because the Schedule 1-A plan changed underneath).
★★ Worse, and this is the part that matters: **`xtask` contains no line-numbered-citation checker at
all.** Nothing anywhere verifies that a `file:line` in any design document points at what it claims —
which is precisely the gap C-1 fell into twice — and **P-1 does not close it either.** P-1 registers
the *quotations*; the *addresses* are the misattribution class `cite_check.rs`'s own module doc names
and defers (*"Attributing a real sentence to the wrong line is the residual gap, and it is tracked as
`FOLLOWUPS.md` §G-10"*, `crates/xtask/src/cite_check.rs:32-33`). Until an address checker exists,
every measurement over this document is an out-of-tree replica and is labelled as one.

★★★ **P-2a IS DELETED (r3, C-1). It was a plan task to build a column-aware extractor for a defect
that `c7819f8c` had already eliminated, and P-1 does not depend on it.** The r2 draft specified a
`--columns` mode (or a derived per-column file) plus its own B1 pairing, and called it *"the reason
P-1 cannot land alone"*. That premise died with the `-layout` extract itself: the r2 measurement —
110 checkable fragments, 40 failing raw, 20 after a column split, all 20 self-citations — was taken
against the **spliced** archive. Re-measured with `cite_check`'s own `quoted_spans` / `fragments` /
`normalise` over **this document as r2 left it** against the five extracts, once against each
version of the i8615/f8615 pair and **with no column splitting either time**:

```
              archived (-layout) extracts          extract of record (c7819f8c)
fragments     110                                  110
FAILING        40                                   20   ← all 20 are self-citations
```

★ The fragment count is a property of the *document*, so it moves as this document is edited (the r3
fold's own quotations of r2 and of the owner ruling take it to 122/32, all 32 likewise
self-citations). **The comparison that matters is the column, not the row:** same document, same
pipeline, two extracts, 40 → 20.

**That is P-2a's target state, reached by regenerating the extract instead of by building an
instrument.** The residual 20 are our own prose, this spec's own prompt and help strings, a
`questions.rs` prompt, the r1 draft, the owner ruling and the statute — i.e. exactly the
`FOREIGN_SOURCES` class P-1 already has to extend for; **zero form quotations fail.** Building P-2a
now would add a derived artifact and a B1 pairing for a condition that cannot occur, and the B1
pairing itself could no longer be written honestly: its whole point was that a *correct* quotation
must pass where the raw extract makes every correct quotation fail, and correct quotations now pass.
**Do not resurrect it. If a genuinely `-layout` two-column authority is ever archived again, the fix
is the extraction convention (`design/forms/README.md:58`), not a second haystack.**

★ **Two spans in this document are irreducibly multi-source and are written accordingly:** i8615's
*support* definition is interrupted by the page footer `Oct 22, 2025` and a blank line at `:66-67`,
so §1.2 quotes it as the two spans it physically is (`:62-65` and `:68-70`); and Form 8275's face
sentence is interrupted by the OMB number in the text layer (§6.3.3 quotes it with an elision, which
`fragments` (`cite_check.rs:362-374`) splits on). ★ **r3 (M-3):** the r2 text gave the first one's
reason as a column break. It is a footer. The quotation structure is unchanged; only its stated
reason was false.

**No oracle can adjudicate this.** Neither engine derives Form 8615's applicability: `grep 8615`
over `scripts/oracle/*.py` is empty, and Tax-Calculator models only the *different* §59(j) kiddie-AMT
exemption cap (`.venv/lib/python3.12/site-packages/taxcalc/calcfunctions.py:2431,2489,2592`), keyed on
`age_head`, not on §1(g). This is the §G-9 limit in its purest form — **the form is the authority and
there is no witness** — so every guarantee below is held by a KAT against the extract, not by
agreement.

★★ **r3 (C-1) — `i8615--2025.txt` is in READING ORDER, and no citation in this document names a
column.** The r2 draft warned that the extract was two-column, that *"a line number alone can be
ambiguous"*, and that `:139-141` was the IRS-request bullet in the left column and the *Earned
income* definition in the right. `c7819f8c` re-extracted it with plain `pdftotext`, which reads the
columns in order; `:139-141` is now a page footer, and the two contents that warning named are 73
lines apart (`:193-195` and `:266-267`). **Every "left column" / "right column" annotation is
deleted**, because each is a false statement about the artifact — not a stale one, a false one — and
they were scattered across §1.1, §1.2, §2, §4, §5.1, §5.2, §6, §6.3.1, §8, §9 G7, §11, §12 and §13,
i.e. through every section that transcribes anything. `grep -c "column" ` over this file is the
check, and the residue is Form 8275's Part I **columns (a)-(f)**, which are the form's own.

### 1.1 The five conditions, verbatim — from BOTH documents

**i8615, the form's own instructions** (`design/forms/extract/i8615--2025.txt:44-57`) —
stated about *"the child"*:

> *"Form 8615 must be filed for any child who meets all of the following conditions."*
>
> 1. *"The child had more than $2,700 of unearned income."*
> 2. *"The child is required to file a tax return."*
> 3. *"The child either:"*
>    a. *"Was under age 18 at the end of 2025,"*
>    b. *"Was age 18 at the end of 2025 and didn’t have earned income that was more than half of the
>       child's support, or"*
>    c. *"Was a full-time student at least age 19 and under age 24 at the end of 2025 and didn’t have
>       earned income that was more than half of the child's support."*
> 4. *"At least one of the child's parents was alive at the end of 2025."*
> 5. *"The child doesn’t file a joint return for 2025."*

★ Note limb (c): i8615 says *"at least age 19 **and** under age 24"* where i1040gi says *"at least age
19 **but** under age 24"*. Same rule, one conjunction apart — which is exactly why §9 G7 checks each
clause against **the extract it is sourced from**, named per clause, rather than against "the extract".

**i1040gi, addressed to the filer as *"you"*** (`design/forms/extract/i1040gi--2025.txt:3927-3944`) —
this is the wording §4.1's prompts transcribe:

> *"You must file Form 8615 if you meet all of the following conditions."*
>
> 1. *"You had more than $2,700 of unearned income (such as taxable interest, ordinary dividends, or
>    capital gains (including capital gain distributions))."*
> 2. *"You are required to file a tax return."*
> 3. *"You were either:"*
>    a. *"Under age 18 at the end of 2025,"*
>    b. *"Age 18 at the end of 2025 and didn’t have earned income that was more than half of your
>       support, or"*
>    c. *"A full-time student at least age 19 but under age 24 at the end of 2025 and didn’t have
>       earned income that was more than half of your support."*
> 4. *"At least one of your parents was alive at the end of 2025."*
> 5. *"You don’t file a joint return in 2025."*

And the age convention (`:3945-3951`):

> *"A child born on January 1, 2008, is considered to be age 18 at the end of 2025; a child born on
> January 1, 2007, is considered to be age 19 at the end of 2025; and a child born on January 1,
> 2002, is considered to be age 24 at the end of 2025."*

i8615 prints the same convention as a **chart** (`i8615--2025.txt:71-98`) with three
footnotes, the third of which is the 24-or-older suppression §5.1 computes, printed by the IRS itself
(`:98`):

> *"*** Don’t use Form 8615 for this child."*

**Dependency appears in none of the five** — and it does not have to be inferred from that absence,
because i8615 says so (§2).

### 1.2 The three terms i8615 defines FOR THIS FORM — transcribed, not borrowed

★★ **r2 (C-1(b)/(c), I-1).** The r1 draft took *support* and *earned income* from **Chart B**
(`i1040gi--2025.txt:691-696`), whose own first line scopes it to a different test — *"If your parent
(or someone else) can claim you as a dependent, use this chart to see if you must file a return."*
(`:692`). The two documents genuinely disagree: Chart B counts *"taxable scholarship and fellowship
grants"* as **earned** (`:695`); i8615 counts them, *"not reported on Form W-2"*, as **unearned**
(`:32-33`). Importing one scope's definition into the other's test is the compression `CLAUDE.md`
forbids, and here it points the opposite way. **These are the governing definitions.**

**Support** — the definition is interrupted by the page footer `Oct 22, 2025` and a blank line
(`:66-67`), so it is quoted as the two spans it physically is (`i8615--2025.txt:62-65`):

> *"Support. Your support includes all amounts spent to provide the child with food, lodging,
> clothing, education, medical and dental care, recreation, transportation, and similar necessities.
> To figure your child’s support, count support provided by you, your child, and"*

continuing at `i8615--2025.txt:68-70`:

> *"others. However, a scholarship received by your child isn’t considered support if your child is a
> full-time student. For details, see Pub. 501, Dependents, Standard Deduction, and Filing
> Information."*

★ The scholarship carve-out is scoped to **full-time students** — i.e. exactly condition 3(c)'s
population — and the r1 help dropped it.

**Earned income** (`i8615--2025.txt:266-267`):

> *"Earned income. Earned income includes wages, tips, and other payments received for personal
> services performed."*

i8615 then adds enlargements the r1 help did not have: the sole-proprietor/partner allowance capped at
*"30% of the child’s share of the net profits"*, and the case where capital is not an income-producing
factor, in which *"all of the child’s gross income from the trade or business is considered earned
income"* (`:268-281`); plus any *"taxable distribution from a qualified disability
trust"* (`:282-284` — §1(g)(4)(C) in the statute says the same). Every one of them
**enlarges** earned income, so omitting them pushes a filer toward answering YES to condition 3 —
toward refusal. That direction is now stated rather than accidental (§4.1).

**Unearned income** (`i8615--2025.txt:28-35`):

> *"Unearned income is generally all income other than salaries, wages, and other amounts received as
> pay for work actually performed (earned income). It includes taxable interest, dividends, capital
> gains (including capital gain distributions), rents, royalties, pension and annuity income, taxable
> scholarship and fellowship grants not reported on Form W-2, unemployment compensation, alimony, the
> taxable part of social security and pension payments, and income (other than earned income)
> received as the beneficiary of a trust."*

§5.2 reconciles that enumeration against the component sum btctax actually computes, and names what
makes the gap unreachable.

---

## 2. The defect, measured

★★★ **The IRS states the holding; btctax does not have to derive it.**
`design/forms/extract/i8615--2025.txt:58-61`:

> *"For these rules, the term “child” includes a legally adopted child and a stepchild. These rules
> apply whether or not the child is a dependent. These rules don’t apply if neither of the child’s
> parents were living at the end of the year."*

**That sentence is FR-29's whole answer**, and it had been sitting in the archive since the commit
after this spec was written while the spec reasoned from dependency's *absence* from a list. In a repo
whose standing rule is *the answer is in the manual*, arguing from an absence when the source states
the presence is the shape of having stopped reading. It is now the load-bearing citation: §8 R-3's doc
comment quotes it, §4.1's condition-3 help quotes it, and §9 G1 asserts against it. ★ It also matches
§1(g)(2) in the statute, whose three conditions — "such child … has not attained age 18 before the
close of the taxable year", "either parent of such child is alive at the close of the taxable year",
"such child does not file a joint return for the taxable year" — likewise never mention dependency
(`legal/primary-sources/statute-irc/26USC_s1.html`, §1(g)(2)(A)-(C)).

`crates/btctax-core/src/tax/return_1040.rs:989`:

```rust
if ri.header.can_be_claimed_as_dependent_taxpayer != Some(false) {
```

That one condition gates the entire screen (`return_1040.rs:979-1003`). A filer who answers *"No"* to
*"Can someone claim YOU as a dependent on their return?"* (`crates/btctax-core/src/tax/questions.rs:289`)
never reaches the `unearned` computation at all, is taxed at their own rates, and files.

**Whom that loses.** A self-supporting minor or student whose support comes from *unearned* income —
this product's own user. They are not claimable as anyone's dependent (they provide more than half
their own support), so they truthfully answer No; but Form 8615 asks about **earned** income
(*"didn’t have earned income that was more than half of your support"*), and unearned support does
not satisfy it. Every one of the five conditions holds. §1(g)(1) applies *"the greater of"* the child's
rate and the parent's-rate computation, so **this can only understate**.

**A green test pins the wrong reading**, and says so: `return_1040.rs:4465-4475` carries the FR-29
warning and then asserts

```rust
let mut not_dep = dependent(dec!(9000));
not_dep.header.can_be_claimed_as_dependent_taxpayer = Some(false);
assert_eq!(screened(&not_dep, &empty), None);
```

`RefuseReason::KiddieTax`'s doc comment (`crates/btctax-core/src/tax/return_refuse.rs:263-273`) carries
the same correction. **Both the comment and the doc were fixed on 2026-09-04; the gate was not.**

---

## 3. Why the obvious fix is worse than the defect

Replace the dependency test with *screen unless the filer is provably outside Form 8615’s reach*
and the screen fires for everyone. `Person::date_of_birth` is `Option<Date>`
(`crates/btctax-core/src/tax/return_inputs.rs:198,228`) and is deliberately a **class-(B) skippable**
— *"A mandatory DOB prompt would force the filer to INVENT a birthday,
and an invented-old one understates tax — so `None` must stay reachable."* (`questions.rs:800-801`). Conditions 3 and 4 are not
collected at all. So *"provably outside"* would refuse every filer with no DOB on file and more than
the threshold of unearned income — in a crypto tax tool, most of them.

**This repo's rule decides it** (`CLAUDE.md`, *Transcribe IRS forms*): *"If the form asks something our
input surface cannot answer, collect it. That is following instructions, not scope creep."*

### 3.1 The shape: class-(A) semantics in the class-(B) registry

The brief for this spec asked for the class-(A) `FORM_QUESTIONS` pattern. **This spec deliberately
does not use it, and the reason is structural, not stylistic.**

`FormQuestion::live` is `fn(&ReturnInputs) -> bool` (`questions.rs:53-54`) and `screen_inputs` refuses
every live-and-`None` declaration *before* any compute (`return_refuse.rs:823-826`). But condition 1 —
the only condition that makes the question worth asking — needs the **ledger**: `unearned` includes
`crypto.nonbusiness_ordinary` and `capital_gain_line7(ri, state, year, …)`
(`return_1040.rs:990-996`), and `ReturnInputs` carries no crypto by construction
(`return_1040.rs:1045-1046`). Two dead ends follow:

- **Liveness ignoring condition 1** ⇒ a hard `screen_inputs` refusal on every DOB-less return,
  including a return with zero unearned income. Strictly worse than the defect for most filers.
- **Liveness using only the `ReturnInputs`-visible unearned income** ⇒ a filer whose unearned income
  is entirely crypto has the question **not live**, so `btctax income answer` never offers it, while
  the compute-time gate demands it: **a refusal with no reachable remedy.** That is the shipped
  circular-liveness bug of `questions.rs:344-345` in a new costume.

The registry already has the right shape for exactly this, twice, and both are shipped and tested:
`SkippableId::ScheduleCIsCooperativePatron` and `ScheduleCIsSstb`. Their contract is stated at
`crates/btctax-input-form/src/attribute.rs:68` — *"SKIPPABLES, offered always and mandatory only where
the answer changes the form"* — with the mandatory half living in the compute-time screen that can see
what the input screen cannot (`return_1040.rs:2676-2690`).

So: **class-(A) semantics — silence never answers for the filer, and where the answer changes the
number btctax refuses rather than guess — delivered through `SKIPPABLE_QUESTIONS`, whose liveness can
be broad and whose mandatory half sits in `screen_compute_dependent`.**

The justification for refusal is the registry's own three-part test (`questions.rs:837-839`): refusal
is warranted only when proceeding without the answer would *produce a wrong number*, *put fabricated
testimony on a signed return*, or *silently expose the filer to a penalty or a lost right*. Here the
**first** limb is met exactly: proceeding computes tax at the child's rate where §1(g) requires the
greater of that and the parent's-rate figure.

---

## 4. What is COLLECTED — three leaves: one per numbered condition, plus the dead-end fact

★ **r2.** The r1 draft had two `Option<bool>` leaves. The owner ruling changes that: condition 4 needs
a **third answer**, and the certification needs one fact of its own. Three leaves on `HouseholdHeader`,
beside the dependency flag (`crates/btctax-core/src/tax/return_inputs.rs:262`), each named for what it
transcribes, each `#[serde(default)]`, each carrying the instruction text verbatim as its doc comment.

```rust
/// **Form 8615, condition 3** (`design/forms/extract/i1040gi--2025.txt:3932-3940`), verbatim:
/// "3. You were either:
///     a. Under age 18 at the end of 2025,
///     b. Age 18 at the end of 2025 and didn’t have earned income that was more than half of your
///        support, or
///     c. A full-time student at least age 19 but under age 24 at the end of 2025 and didn’t have
///        earned income that was more than half of your support."
///
/// ONE leaf, because the form states ONE numbered condition with three alternatives and asks the
/// filer for the disjunction ("You were either: a…, b…, or c…"). Splitting it into three would be
/// the compression this repo's rule forbids, in reverse: it would require btctax to re-derive the
/// disjunction the form already writes.
///
/// `None` ⇒ REFUSED by `screen_compute_dependent` wherever the answer changes the number, and unread
/// everywhere else. It never defaults in either direction. See `Self::form8615_condition4_parent_alive`.
#[serde(default)]
pub form8615_condition3_age_support: Option<bool>,

/// **Form 8615, condition 4** (`design/forms/extract/i1040gi--2025.txt:3941-3942`), verbatim:
/// "4. At least one of your parents was alive at the end of 2025."
///
/// `None` ⇒ REFUSED, on the same terms as `Self::form8615_condition3_age_support`, and additionally
/// only once condition 3 is answered YES — the form reaches condition 4 no other way.
///
/// ★★★ **NOT `Option<bool>`. "I cannot know" is a THIRD ANSWER, and it is not the same thing as
/// UNANSWERED** — the distinction is the whole of the owner ruling
/// (`design/ty2025/DECISION_form8615_no_path_self_certification.md`). Unanswered still refuses;
/// unknowable opens the §6.3 dead-end path and nothing else. Collapsing the two would either
/// re-arm FR-29 (silence becomes an exit) or close the only path out (unknowable becomes a refusal
/// with no remedy).
#[serde(default)]
pub form8615_condition4_parent_alive: Option<ParentAliveAnswer>,

/// **The §6.3 dead-end FACT** — not a condition of Form 8615, and deliberately not phrased as one.
///
/// Form 8615's face requires the parent's name, SSN and filing status on lines A, B and C
/// (`design/forms/extract/f8615--2025.txt:17-20`); none carries an "if known". The one
/// administrative remedy is to request the data from the IRS, and its required contents include,
/// verbatim (`design/forms/extract/i8615--2025.txt:193-195`):
/// "The name, address, social security number (SSN) (if known), and filing status (if known) of the
///  parent whose information is to be shown on Form 8615."
/// SSN and filing status tolerate ignorance. **Name and address do not.** So a filer who cannot
/// supply a name and an address has no route to Form 8615 at all — and this leaf records that FACT,
/// in the filer's own testimony, never the legal conclusion drawn from it.
///
/// `Some(true)` is the ONLY value that opens anything. `None` and `Some(false)` both leave the §6.3
/// refusal standing, which is the *widening an exemption is never the safe edit* rule discharged:
/// the YES-condition is enumerated and every omission fails closed.
///
/// ★★★ **The registry accessors for this leaf INVERT, and they are the only inverting pair in
/// `SKIPPABLE_QUESTIONS`.** The prompt asks "Can you give the IRS your parent's name and address?",
/// so the filer's NO is this field's `Some(true)`; `get_bool` / `set_bool` carry the `!`, and §4.1
/// writes both out. A straight-through pair certifies the filer who answered YES — an
/// understatement path, and the one §9 G14(a)'s polarity row exists to kill.
#[serde(default)]
pub form8615_parent_identity_unobtainable: Option<bool>,
```

### 4.0 The third answer's type

```rust
/// Form 8615 condition 4's answer — **three-valued**, in `crates/btctax-core/src/tax/return_inputs.rs`
/// beside `HouseholdHeader`.
///
/// ★ Why an enum and not a second `Option<bool>` pair: a pair admits the incoherent state
/// (`alive = Some(true)`, `unknowable = Some(true)`), and every consumer would then need a rule for
/// it. Three variants make the incoherent state unrepresentable, and the `match` in
/// `screen_compute_dependent` is exhaustive, so a fourth variant added later reds every site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParentAliveAnswer {
    /// "At least one of your parents was alive at the end of 2025."
    /// (`design/forms/extract/i1040gi--2025.txt:3941-3942`)
    Yes,
    /// Neither parent was alive at the end of the year. i8615 states the consequence directly
    /// (`design/forms/extract/i8615--2025.txt:60-61`): "These rules don’t apply if
    /// neither of the child’s parents were living at the end of the year."
    No,
    /// ★★★ **THE THIRD ANSWER.** The filer cannot know, because they cannot identify their parents.
    /// Distinct from `None` (never asked). Unlocks §6.3 and nothing else — on its own it still
    /// refuses, because the certification also requires
    /// `HouseholdHeader::form8615_parent_identity_unobtainable == Some(true)`.
    CannotKnow,
}
```

★ `Serialize`/`Deserialize` with the default (externally-tagged unit-variant) representation, so a
vault stores `"Yes"` / `"No"` / `"CannotKnow"`. A vault written before this change has the field
absent ⇒ `None` ⇒ unanswered ⇒ refuse (§7). **No serde default may name a variant**, and there is no
`#[serde(other)]` fallback: an unknown string must fail the parse rather than silently become an
answer, which is the `SerdeRequired` reasoning at `crates/btctax-core/src/tax/classifier.rs:52-54`
applied to a value instead of to a field's presence.

### 4.1 The registry entries

**Three** `SkippableQuestion`s appended at the **end** of `SKIPPABLE_QUESTIONS`
(`crates/btctax-core/src/tax/questions.rs:952`), taking indices **16, 17, 18**. Appended so the
`Skippables` section's field order — derived from registry order at
`crates/btctax-input-form/src/spec/mod.rs:294-301` — grows rather than shifts.

★★ **r2 (I-2) — the index hazard is REAL in this registry too, and the r1 reassurance was false.**
The r1 draft said the `decl_tristate!` hazard recorded at `questions.rs:99-102` binds `FORM_QUESTIONS`
and *not* this one, because `skippable_to_field` is name-keyed. **The name-keying is true and is not
the protection.** `skippable_tristate!` (`crates/btctax-input-form/src/spec/registries.rs:67-95`)
reads `SKIPPABLE_QUESTIONS[$idx]` for its `label`, `help`, `live`, `get` **and** `set` while taking its
`id` from a separate argument, and the fifteen call sites pass literal indices —
`skippable_tristate!(0, …)` at `registries.rs:254` through `(15, …)` at `:355` (index 2 is absent: SALT
is deduped to its Schedule-A leaf). **A mid-array insert repoints every later entry's prompt, liveness
and accessors.** It is not *silent* — the delegation loop at
`crates/btctax-input-form/src/spec/mod.rs:341-360` sets through the registry entry and reads back
through the `Field`, so a mismatch reds — but "test-caught" is not what the r1 sentence claimed, and
this is the sentence a future maintainer reads immediately before inserting in the middle. **Append at
the end. The reason is that the hazard exists, not that it doesn't.** §9 G9 now names `registries.rs`,
because the two new literal indices are the one piece of this change's bookkeeping the compiler cannot
check: `skippable_to_field` (`registries.rs:457-476`) is exhaustive over `SkippableId`, so the new
**variants** are compile-forced, but any index 0–18 compiles.

**Kinds and durability.** Conditions 3 and the identity fact are `SkippableKind::YesNo`. Condition 4
is **not** — it is the third answer, and `SkippableKind` has only `YesNo` and `Date` today
(`questions.rs:900-903`). It gains a third variant:

```rust
pub enum SkippableKind {
    YesNo,
    Date,
    /// ★ r2 — a fixed set of named answers, for a question whose third answer is not "unanswered".
    /// The only member today is Form 8615's condition 4 (`ParentAliveAnswer`).
    Choice(&'static [&'static str]),
}
```

with `get_choice: fn(&ReturnInputs) -> Option<&'static str>` / `set_choice: fn(&mut ReturnInputs,
&'static str)` beside the existing `get_bool` / `set_bool` / `get_date` / `set_date`
(`questions.rs:926-933`). **Every consumer is a compile error until updated**, which is the blast
radius this repo prefers: the exhaustive `match SkippableKind` sites are
`crates/btctax-cli/src/cmd/answer.rs:156` and `:178`, and
`crates/btctax-input-form/src/spec/mod.rs:368` and `:394`. The input-form seam needs nothing new —
`FieldKind::Enum(&[…])` and `FieldValue::Choice(String)` already exist and are live
(`crates/btctax-input-form/src/seam.rs:177-198`; filing status uses them at
`crates/btctax-input-form/src/spec/sections.rs:165-167`) — so the third registry entry maps to a
`FieldKind::Enum(&["Yes", "No", "CannotKnow"])` field, and `parse_enum`
(`crates/btctax-input-form/src/parse.rs:28`, `:89`) already rejects an unlisted string.

★★ **r3 (M-2) — WHAT THE CLI ACCEPTS AT THE KEYBOARD IS SPECIFIED HERE, not improvised at the compile
error.** The two paths do **not** share a parser. The input-form path uses `parse_enum`
(`crates/btctax-input-form/src/parse.rs:87-92`), which is an exact match —
`if options.contains(&raw) { … } else { Err(ParseError::NotAChoice) }`, no trimming and no
case-folding, because the options are stable tokens a renderer presents as a closed choice. The CLI
path is `btctax income answer`, whose `match sk.kind` (`crates/btctax-cli/src/cmd/answer.rs:155-208`)
uses `parse_yes_no` and `parse_date` and **never calls `parse_enum`** — so the new `Choice` arm must
bring its own reader, and the condition-4 prompt tells the filer *"Answer YES, NO, or CANNOT KNOW"*,
which is not the token `"CannotKnow"`.

**The `Choice` arm in `answer.rs` accepts, case-insensitively and after trimming:** `y` / `yes` ⇒
`Yes`; `n` / `no` ⇒ `No`; `?` / `cannot know` / `cannotknow` / `cant know` / `unknown` ⇒ `CannotKnow`.
Anything else **re-asks**, and a bare Enter keeps what is on file — the same three-way behaviour
`parse_yes_no` already has (`answer.rs:72-79`), extended by one variant rather than replaced.
★ Direction is fail-closed: an unmatched string cannot become an answer, so the worst case is a filer
who is asked again. That is why this is a Minor and not a defect — but leaving it unwritten means the
implementer meets it as a compile error with the prompt's own wording in front of them, and the
cheapest thing to type there is an exact `"CannotKnow"` match that the prompt's own instructions
fail. ★★ The prompt is **not** reworded to say `CannotKnow`: the words the filer reads come from the
form and from plain English, and the parser accommodates them, never the other way round.

All three are `Durability::PerYear`. Age, a parent's survival and what the filer can find out about
their parents all change between years, and the durability test at `questions.rs:1587-1598` asserts
that *exactly* the two dates of birth are `Durable` — so `PerYear` is required, not merely preferred.
★ For the identity fact this is more than bookkeeping: an attested dead end is testimony **about one
tax year**, and carrying it forward silently would re-file last year's certification without asking.

**Condition 3 — prompt** (year-free, because `prompt` is `&'static str` and TY2024 is still the only
year btctax can file; the year-bearing sentences live in the doc comment and in the refusal detail,
which is `format!`ed at the gate where `year` is in scope):
```text
Form 8615, condition 3 — at the end of the tax year, were you either: (a) under age 18, (b) age 18
and didn’t have earned income that was more than half of your support, or (c) a full-time student
at least age 19 but under age 24 and didn’t have earned income that was more than half of your
support? Answer YES if any one of (a), (b) or (c) is true.
```

★★ **r2 (M-5) — the departures from `i1040gi--2025.txt:3932-3940`, ENUMERATED.** The r1 claim (*"the
only permitted departure is 'at the end of 2025' → 'at the end of the tax year', three times. Every
other clause is byte-identical"*) understated it. There are **four**, and each is deliberate:

1. *"at the end of 2025"* → *"at the end of the tax year"* — the prompt is `&'static str` and cannot
   carry a year (§12 OQ-3);
2. that qualifier is **hoisted** to one leading occurrence instead of repeating in each limb;
3. four clause openings are **recased** by the hoist and the interrogative — `You were` → `were you`,
   `Under` → `under`, `Age` → `age`, `A full-time` → `a full-time`;
4. one sentence is **appended** that is not in the form: *"Answer YES if any one of (a), (b) or (c) is
   true."* — the form states the disjunction structurally (*"You were either:"*) and a flat prompt
   cannot, so this restores it rather than adding a rule.

Case and whitespace are exactly what §9 G7's normalisation folds, so (2) and (3) are invisible to the
check and (1) and (4) are the two the clause table must be chosen around. **G7's clauses are therefore
sub-sentence spans that survive all four departures**, which is why they are clauses and not the whole
prompt.

**Condition 3 — help** (★ r2, C-1(b)/(c) and I-1: the definitions now come from i8615, which is the
document that writes them for **this** form; the r1 draft took them from Chart B, whose own scope line
is the filing-requirement test):
```text
Form 8615 taxes part of a child's unearned income at the parent's rate (§1(g)). The Instructions for
Form 8615 say: "These rules apply whether or not the child is a dependent." Being nobody's dependent,
or supporting yourself, does not put you outside them. Skipping is harmless if your unearned income is at or below the §1(g)
threshold for the year, or if btctax can already see from your date of birth that you were 24 or
older at the end of the year — condition 3 cannot be true at 24. Where it does matter, btctax
refuses rather than answer for you: §1(g)(1) takes the GREATER of your own rate and the parent's-rate
figure, so a wrong "no" can only understate your tax.
Your SUPPORT is all amounts spent to provide you with food, lodging, clothing, education, medical
and dental care, recreation, transportation, and similar necessities, counted from every source —
you, your parents and anyone else. A scholarship you received is not counted as support if you are a
full-time student.
EARNED INCOME is wages, tips, and other payments received for personal services performed. If you
are a sole proprietor or a partner it can also include a reasonable allowance for your personal
services, capped at 30% of your share of the net profits; and it includes any taxable distribution
from a qualified disability trust. Income from investments, crypto or a trust is not earned income.
The test is whether your EARNED income covered more than half of your support.
```

★ The two sentences carrying the most weight are transcribed, not paraphrased: *"These rules apply
whether or not the child is a dependent."* (`i8615--2025.txt:59-60`) is the FR-29
inoculation, and *"Income from investments, crypto or a trust is not earned income."* is kept verbatim
from r1 — the r1 review named it as the one sentence that made the independence trap unfalsifiable
for the filer, and it survives unchanged.

**Direction, now stated rather than accidental.** Both remaining simplifications push the same way:
the help still omits the capital-not-a-material-factor rule (`i8615--2025.txt:276-281`),
which would **enlarge** earned income, and it states the scholarship carve-out, which **shrinks**
support. Both make a filer more likely to answer YES to condition 3 — toward refusal, never away from
it. §9 G12 pins that.

**Condition 4 — prompt** (three answers, not two):
```text
Form 8615, condition 4 — is this true of you: "at least one of your parents was alive at the end of
the tax year"? Answer YES, NO, or CANNOT KNOW. Choose CANNOT KNOW only if you are unable to find
out — for example, you do not know who your parents are.
```

★★ **r2 — this prompt is DECLARATIVE, and the r1 interrogative was a defect the r1 review missed.**
r1 asked *"was at least one of your parents alive at the end of the tax year?"*, and the review's own
G7 table recorded the clause *"At least one of your parents was alive"* as present in that prompt. It
is not: the interrogative inverts the verb, so the form's words *"at least one of your parents was
alive"* do not occur. Machine-checked with `cite_check::normalise` — the clause is in the extract and
**not** in the r1 prompt, so G7 as r1 specified it would have gone red on this clause on day one, and
the review's table is overturned on this row. The declarative form quotes the condition and asks
whether it is true, which is what a three-answer question needs anyway: *yes / no / cannot know* does
not fit an inverted question as naturally as it fits a proposition.

**Condition 4 — help** (★ r2, M-3: the r1 text said the two conditions *"together are what send you to
Form 8615"*, which is false — all five do; and condition 4's liveness does not read condition 1, so a
filer with no unearned income at all could be asked it and told that):
```text
Form 8615's condition 4. It is one of five conditions, all of which must hold before Form 8615 is
required; btctax asks it only after you answered YES to condition 3, because that is the order the
form asks them in. Skipping is harmless if condition 3 is "no", or if your unearned income is at or
below the §1(g) threshold for the year; otherwise btctax refuses rather than answer for you.
If neither of your parents was living at the end of the year, answer NO — these rules do not apply
to you. If you cannot find out because you do not know who your parents are, answer CANNOT KNOW:
that is a different answer from leaving this blank, and btctax will then ask you one further
question and explain what it can and cannot do.
```

**The dead-end fact — prompt** (asked only after CANNOT KNOW; see §4.2):
```text
Can you give the IRS your parent's name and address? Form 8615 asks for your parent's name, social
security number and filing status, and the only way to get those from the IRS requires you to supply
your parent's name and address. Answer YES if you can supply both for either parent. Answer NO only
if you can supply neither.
```

★ The prompt is phrased in the **positive** — *can you* — and the certification unlocks on `NO`. That
is deliberate: it asks the filer what they **can** do, which is a fact they can answer without
reading anything into it, rather than inviting them to affirm a conclusion.

★★★ **r3 (I-1) — THE ACCESSORS FOR THIS LEAF INVERT, AND THE SPEC SAYS SO HERE RATHER THAN LEAVING IT
TO BE INFERRED.** The prompt asks *can you*, the field is named `…_unobtainable`, and `Some(true)` is
the only value that opens anything (§4) — so **the filer's NO is the field's `Some(true)`**. The
inversion has to live in the two `fn` pointers, because both consumers hand the filer's own boolean
straight to `set_bool` and render `get_bool` straight back as the current answer to the prompt:
`skippable_tristate!` destructures `FieldValue::TriState(Some(b))` and calls
`(SKIPPABLE_QUESTIONS[$idx].set_bool)(ri, b)`
(`crates/btctax-input-form/src/spec/registries.rs:83-91`), and `answer.rs`'s `YesNo` branch calls
`(sk.set_bool)(&mut ri, v)` on `parse_yes_no`'s value while showing `(sk.get_bool)(&ri)` as
*"currently y"* / *"currently n"* (`crates/btctax-cli/src/cmd/answer.rs:178-201`). **Write them
exactly like this:**

```rust
// The argument and the return are ANSWERS TO THE PROMPT ("can you supply a name and address?"),
// which is the negation of the leaf. This is the ONLY inverting pair in SKIPPABLE_QUESTIONS.
get_bool: |ri| ri.header.form8615_parent_identity_unobtainable.map(|v| !v),
set_bool: |ri, can_supply| {
    ri.header.form8615_parent_identity_unobtainable = Some(!can_supply);
},
```

★ **The snippet is machine-checked, not eyeballed.** It was extracted, given the real fn-pointer
signatures from `SkippableQuestion` (`questions.rs:926-933`) and a stand-in for the not-yet-added
field, compiled with `rustc`, and run: `set_bool(…, true)` (the filer's **YES**) writes
`Some(false)`, `set_bool(…, false)` (their **NO**) writes `Some(true)`, and `get_bool` reads each
back as the filer's own answer. That is the whole of the fix, and it is the one executable fragment
this fold adds.

★★ **Why this is called out and not left as an implementation detail: every other entry in
`SKIPPABLE_QUESTIONS` is polarity-ALIGNED** (`"Are YOU legally blind?"` → `taxpayer.blind`;
`questions.rs:962-963`, and there is no `!` in any accessor in the registry today), so the obvious
transcription is a straight-through pair — and a straight-through pair is an **understatement path**.
A filer who answers **YES** — *I can give the IRS my parent's name and address*, the filer for whom
the IRS route is open and R-4 is meant to be final — would be stored as `unobtainable = Some(true)`,
reach §6 ladder step 6's first arm, and **receive a computed return with no Form 8615.** That is
owner-ruling constraint 1 broken, in the FR-29 direction, rebuilt inside the fix for FR-29.

★★ **And nothing existing reds on it**, which is why §9 gains a guarantee of its own (G14(a)'s new
row). G14(a), G2 and G8 all drive the gate from **field values**, so they never see the keystroke;
and the registry delegation test (`crates/btctax-input-form/src/spec/mod.rs:368-393`) round-trips
`(entry.set_bool)(…, true)` back through `(f.get)`, which a **consistently** inverted pair passes and
a **straight-through** pair passes — it detects a get/set *mismatch*, which is not this defect. The
only test that can distinguish them is one that goes in through the `Field` and asserts on the
**refusal**, and that is what G14(a) now carries.

**The dead-end fact — help:**
```text
btctax asks this only because you said you cannot know whether a parent was alive. Form 8615 cannot
be completed without your parent's name, social security number and filing status (lines A, B and
C), and the tax on it cannot be computed without your parent's taxable income. The IRS will send you
that information if you ask — but the request must contain "The name, address, social security
number (SSN) (if known), and filing status (if known) of the parent whose information is to be shown
on Form 8615". The SSN and the filing status may be unknown. The name and the address may not.
If you can supply a name and an address, answer YES: the route is open, and btctax will tell you to
use it. If you can supply neither, answer NO, and btctax will attach a disclosure explaining why
your return does not include Form 8615. Read that explanation before you file — it is not a ruling
in your favour, and it is not free of risk.
```

### 4.2 Liveness — when is each even ASKED?

Pure functions of `ReturnInputs`, as the registry requires:

```rust
// condition 3
live: |ri| ri.filing_status != FilingStatus::Mfj && !provably_24_or_older(ri, ri.tax_year),
// condition 4
live: |ri| ri.filing_status != FilingStatus::Mfj
        && !provably_24_or_older(ri, ri.tax_year)
        && ri.header.form8615_condition3_age_support != Some(false),
// the dead-end fact (r2) — the ONLY question whose liveness is gated on the third answer
live: |ri| ri.filing_status != FilingStatus::Mfj
        && !provably_24_or_older(ri, ri.tax_year)
        && ri.header.form8615_condition3_age_support != Some(false)
        && ri.header.form8615_condition4_parent_alive == Some(ParentAliveAnswer::CannotKnow),
```

- **`!= Mfj`** is condition 5, computed (§5.3). A joint filer is never asked.
- **`!provably_24_or_older`** is the computed half of condition 3 (§5.1). A filer with a date of birth
  showing 24 or older at year end is never asked any of the three. This is what keeps the interview
  from asking a 60-year-old about full-time student status.
- **Condition 4 additionally requires condition 3 not to be a definite "no"**, so a filer who answers
  condition 3 = No never sees condition 4. `None` keeps it live, so both are offered together on a
  first run and there is no answer-then-come-back-for-the-next-one loop.
- ★★★ **The dead-end fact is live ONLY on `Some(CannotKnow)`** — this is owner-ruling constraint 1
  discharged in the liveness predicate. It is not offered to a filer who answered YES, NO, or nothing
  at all; a filer must *first* testify that they cannot know before btctax will even ask the second
  question. **A general opt-out would rebuild FR-29 behind a nicer interface**, and the place that
  cannot happen is here, because a question that is never live is never asked and its `None` never
  clears anything (§6). Unlike conditions 3 and 4, this predicate deliberately depends on another
  answer's *exact* value rather than on its non-`Some(false)`-ness: `!= Some(Yes)` would make it live
  for an unanswered condition 4, which is one keystroke from an opt-out.
- ★ **This makes the dead-end fact a two-pass question** — it becomes live only after condition 4 is
  answered, so `btctax income answer` must be run twice to reach it. That is the intended cost, and
  the R-4 refusal detail (§8) says so explicitly rather than leaving the filer to discover it.

**Condition 1 is deliberately NOT in the liveness predicate** — it cannot be, per §3.1 — which makes
the live set a strict superset of the demanded set. Over-asking is a recorded outcome here, with
precedent (`questions.rs:309-310`: *"a stale spouse on a non-MFJ return is a recorded over-ask (§3.1),
never an under-ask"*). §9 G8 turns "superset" into a test.

★ **`ri.tax_year` vs the gate's `year`.** The gate is handed the authoritative `year`
(`return_1040.rs:906-911`); liveness only has `ri.tax_year`
(`crates/btctax-core/src/tax/return_inputs.rs:688`, added by §G-15 for
exactly this). The CLI keeps them equal — `crates/btctax-cli/src/return_inputs.rs:75` assigns
`ri.tax_year = year` on load and `:103-110` errors on a stored disagreement.

★★ **r2 (M-2) — the r1 claim *"Never the reverse"* was too strong, and the correction is a test, not a
sentence.** r1 argued the divergence is one-directional because `ReturnInputs::default()` is
`tax_year: 0` (`crates/btctax-core/src/tax/return_inputs.rs:1081`) and a smaller `tax_year` yields a smaller computed age, so the
question stays **live** when the gate might not demand it. That holds for `0`. It does **not** hold in
general: `tax_year` is a plain `i32` (`crates/btctax-core/src/tax/return_inputs.rs:688`) and a hand-built fixture may set any
value, so `tax_year > year` computes an *inflated* age ⇒ `provably_24_or_older` true ⇒ the question is
**not live** while the gate demands it — the brick §3.1 exists to avoid. The exposure is scoped to
hand-built inputs, because the storage boundary stamps on read (`btctax-cli/src/return_inputs.rs:75`)
and refuses a stored disagreement (`:103-110`). **So the claim is narrowed to what is true — the
direction is safe for `Default` and for anything the CLI loads — and §9 G8 asserts the INVARIANT
(demanded ⊆ live) rather than the direction**, over a grid that includes `tax_year != year`. An
invariant that is checked cannot quietly stop holding; a direction argument in prose can.

---

## 5. What is COMPUTED — conditions 1, 2, 5, and the age arithmetic

### 5.1 The computed half of condition 3: `provably_24_or_older`

All three limbs of condition 3 bound the filer's age below 24 — (a) *"Under age 18"*, (b) *"Age 18"*,
(c) *"at least age 19 but under age 24"* (`:3933-3940`). **Therefore condition 3 is false for anyone
aged 24 or over at the end of the year.** That is an equivalence, not a heuristic, and it is the only
computation this spec permits to *suppress* a question.

The IRS's own convention is that a person attains an age on the day before their birthday, which
`i1040gi:3945-3951` states as three worked examples. ★ **r2 (C-1(e)) — and i8615 prints the
suppression itself.** The same three birth dates appear as a chart at `i8615--2025.txt:71-98`,
and the January-1-2002 row carries the footnote *"*** Don’t use Form 8615 for this child."*
(`:98`). So `provably_24_or_older` is not an equivalence btctax derived from three
examples — it is a rule the IRS prints, and the derivation below merely computes the same boundary for
every birth date rather than the three the chart happens to list. Encode it as an age at a fixed
boundary rather than as a date:

```rust
/// The age a person born on `dob` is CONSIDERED to be at the end of `year` — i.e. their age on
/// January 1 of `year + 1`, which is what i1040gi:3945-3951's three examples state.
fn considered_age_at_year_end(dob: Date, year: i32) -> i32 {
    year - dob.year() + i32::from(dob.month() == Month::January && dob.day() == 1)
}

/// Condition 3 is FALSE by arithmetic — for a KNOWN date of birth only.
fn provably_24_or_older(ri: &ReturnInputs, year: i32) -> bool {
    ri.header
        .taxpayer
        .date_of_birth
        .is_some_and(|d| considered_age_at_year_end(d, year) >= 24)
}
```

**Why this form and not `reaches_65_on`'s (`return_1040.rs:110-122`).** That helper computes a *date*
and therefore needs a February-29 fallback (`replace_year` fails on a leap day). This computes an
*age at a fixed boundary*, so no date is constructed and no leap-day branch exists — February 29 is
never January 1. §9 G3 pins a leap-day vector anyway, because "no branch is needed" is a claim, not a
guarantee.

**The equivalence proof and the branch where it breaks** (`CLAUDE.md` requires both, plus a KAT):
the derivation is exact for a correct date of birth; it breaks on a **wrong** date of birth, in the
suppressing direction. That exposure is not new — the same field already decides the §63(f) age-65
addition (`return_1040.rs:89-91`, `:110-122`) — and it is bounded to filers who volunteered a DOB. `None` never
suppresses. KAT: §9 G3.

### 5.2 Condition 1 — computed, unchanged

`unearned` keeps the existing component sum and the existing conservative-direction argument at
`return_1040.rs:980-988`: it omits the Schedule 1 adjustments that Form 8615's `AGI − earned` would
net out, so it can only be **too high** and can therefore only over-refuse. **Do not "fix" that
without preserving the direction.** The comparison stays strictly greater than
`params.kiddie_unearned_threshold`, because the form says *"more than $2,700"*.

★★★ **r2 (C-1(d)) — the direction claim SURVIVES, but not for the reason r1 gave, and the real reason
was uncited.** §1.2 quotes i8615's unearned-income enumeration. Against the component sum at
`return_1040.rs:990-995`:

| i8615 category (`:28-35`) | in the sum? |
|---|---|
| taxable interest, dividends, capital gains (incl. capital gain distributions) | **yes** — `sum_taxable_interest`, `sum_ordinary_dividends`, `capital_gain_line7` (`:990-992`) |
| unemployment compensation | **yes** — `sum_unemployment(ri)` (`:994`) |
| rents, royalties, pension and annuity income, taxable scholarship and fellowship grants not reported on Form W-2, alimony, the taxable part of social security and pension payments, trust-beneficiary income | **no** |

★ The r1 review reported that the sum *"omits every bolded category"*; that is **wrong for
unemployment compensation**, which `:994` adds, and the correction is recorded rather than folded
silently. (The sum also adds `ri.sch1.state_refund_taxable` at `:993`, which i8615's list does not
name and which is unearned on any reading.)

**What makes the remaining gap unreachable is a refusal in a different module, and this spec is where
that dependency gets written down.** `QuestionId::OtherOutOfScopeIncome` (`questions.rs:546-563`) asks
about *"a PENSION, ANNUITY or IRA DISTRIBUTION"*, *"SOCIAL SECURITY or railroad retirement benefits"*,
*"rent or royalties, a farm, a partnership, S corporation, estate or trust (any Schedule K-1)"*,
*"alimony"* — and closes with *"or anything else it never asked about"*, which is what covers the
scholarship limb. A `Some(true)` hard-refuses `OtherIncomeOutOfScope`
(`return_refuse.rs:995-997`), and `None` refuses `OtherIncomeUnanswered` at `screen_inputs`
(`return_refuse.rs:823-826`) because it is a class-(A) declaration. **So every omitted category is
unreachable in a computable return, and the sum's "too high" direction holds — by that refusal, not by
the sum's own construction.** This is the §G-9 shape exactly: a load-bearing dependency held across a
module boundary by nothing but the fact that it currently holds. §9 **G11** pins it, so the next scope
widening reds here instead of silently turning an over-refusal into an under-refusal.

The threshold is a per-year parameter (`crates/btctax-core/src/tax/tables.rs:470`), currently
`dec!(2600)` for TY2024 (`crates/btctax-adapters/src/tax_tables.rs:134`). **The `$2,700` in
`i1040gi--2025` is the TY2025 value and lands with `ty2025_full_return()`** — which does not exist
yet (`full_return_for(2025)` returns `None` by design, `tax_tables.rs:814-820`). **Nothing in this
spec may hardcode either number.**

### 5.3 Condition 5 — computed

*"You don’t file a joint return in 2025."* ⇒ `ri.filing_status != FilingStatus::Mfj`. Exact, one
field, no collection. QSS is not a joint return and is therefore *not* exempted by this condition.

### 5.4 Condition 2 — assumed TRUE, with the branch named

*"You are required to file a tax return."* btctax cannot compute this without transcribing Charts A,
B and C (`i1040gi:625-666`, `:691-728`, `:732`). **It is assumed TRUE.** Direction: assuming a
condition of a conjunction is TRUE can only make the conjunction more often true, hence only ever
refuse more — fail-closed.

**Exact for the ordinary dependent.** Chart B's first bullet for a single dependent who is neither 65+
nor blind is *"Your unearned income was over $1,350."* (`:699`), and condition 1 already requires
unearned income above the (larger) §1(g) threshold. So for that filer the assumption is not an
assumption at all.

**The branch where it breaks**, named precisely so the KAT can pin it: a filer who is **not** required
to file yet meets conditions 1, 3, 4 and 5. Two populations reach it —

1. a **non-dependent** below Chart A's threshold (`$15,750` single under 65, `:634-639`), i.e. gross
   income between the §1(g) threshold and that figure; and
2. a **blind dependent under 24**, for whom Chart B's *"Yes"* branch raises the bullet to *"Your
   unearned income was over $3,350 ($5,350 if 65 or older and blind)."* (`:705`) — above the §1(g)
   threshold, so the range in between is genuinely not a filing requirement.

Both are refused where the form would not require Form 8615. Both are **over**-refusals of a filer
who was not required to file at all, and neither produces a wrong number. **OQ-2** proposes the
Chart A/B/C transcription that would close them; §9 G6 pins the branch so it cannot be forgotten.

---

## 6. The new gate

Replaces `return_1040.rs:979-1003` in place, inside `screen_compute_dependent` — the screen that
already has `(ri, state, year, params)`. **The dependency flag is not read.** Its deletion is the fix;
because the identifier disappears, no refactor can quietly restore the old behaviour without
re-introducing a reference someone must write down.

Evaluate the form's conjunction, three-valued:

| condition | source | value when unknown |
|---|---|---|
| 1 — unearned over the threshold | computed from `ri` + ledger (§5.2) | never unknown |
| 2 — required to file | assumed `true` (§5.4) | n/a |
| 3 — age/support | `false` if `provably_24_or_older`; else the collected answer | **UNKNOWN** |
| 4 — a parent alive | irrelevant if 3 is `false`; else the collected answer, which is **three-valued** | **UNKNOWN** (≠ `CannotKnow`) |
| 5 — not a joint return | `ri.filing_status != Mfj` | never unknown |

Ladder, in order:

1. `!c1 || !c5` → **proceed** (no refusal). One of the two is proved FALSE — condition 1 by
   arithmetic, condition 5 by filing status — so the conjunction cannot hold.
2. `c3 == false` → **proceed.** Either the filer said No, or the date of birth proves 24-or-older.
3. `c3 == UNKNOWN` → **refuse `Form8615AgeSupportUnanswered`.**
4. `c4 == None` → **refuse `Form8615ParentAliveUnanswered`.** (Reached only with `c3 == true`.)
   ★ `None` only. `Some(CannotKnow)` is an **answer** and does not reach this step.
5. `c4 == Some(No)` → **proceed.** i8615 states the consequence directly (`:60-61`).
6. `c4 == Some(CannotKnow)` → the §6.3 dead end:
   - `form8615_parent_identity_unobtainable == Some(true)` → **proceed, with the §6.3 disclosure
     attached and the three warnings surfaced.**
   - otherwise (`None` or `Some(false)`) → **refuse `Form8615ParentUnidentifiable`** (R-4).
7. `c4 == Some(Yes)` → **refuse `KiddieTax`.** All five conditions hold.

**`match` on the enum, never `if`-chains.** Step 4–7 is one exhaustive `match` over
`Option<ParentAliveAnswer>` with four arms and no `_`, so a fourth variant added later is a compile
error at the gate rather than a silent fall-through into whichever arm happens to be last. This is the
same reason `classifier.rs` destructures `HouseholdHeader` with no `..` (§6.2).

### 6.1 The unknown cases fail closed — the proof

Enumerate every combination of the collected answers with condition 1 true and condition 5 true
(the only region where anything is demanded). `DOB≥24` means `provably_24_or_older`; `cert` is
`form8615_parent_identity_unobtainable`:

| DOB≥24 | cond 3 | cond 4 | cert | outcome | why it is safe |
|---|---|---|---|---|---|
| yes | any | any | any | proceed | condition 3 is **proved false** (§5.1) — not assumed |
| no | `None` | any | any | refuse *AgeSupportUnanswered* | silence is not a No |
| no | `Some(false)` | any | any | proceed | the **filer** said No |
| no | `Some(true)` | `None` | any | refuse *ParentAliveUnanswered* | silence is not a No |
| no | `Some(true)` | `Some(No)` | any | proceed | the **filer** said No |
| no | `Some(true)` | `Some(CannotKnow)` | `None` | refuse *ParentUnidentifiable* (R-4) | silence is not an attestation |
| no | `Some(true)` | `Some(CannotKnow)` | `Some(false)` | refuse *ParentUnidentifiable* (R-4) | the filer said the route is OPEN — use it |
| no | `Some(true)` | `Some(CannotKnow)` | `Some(true)` | **proceed, disclosed** (§6.3) | **two** affirmative answers plus a Form 8275 disclosure |
| no | `Some(true)` | `Some(Yes)` | any | refuse `KiddieTax` | all five conditions hold |

**No row reaches "proceed" through an absent answer.** Every proceed is licensed by a proof
(row 1), by the filer's own NO (rows 3 and 5), or — in the one new row — by **two** affirmative
answers *and* a disclosure filed in btctax's own voice. That is the *widening an exemption is never
the safe edit* rule discharged: the YES-conditions are enumerated and the fallback is refusal, so
every omission is an over-refusal (recoverable) rather than an understatement (not).

★ **What r2 changed about this invariant, stated honestly.** The r1 wording was *"every proceed is
licensed by a proof or by the filer's own answer"*. The certification row is licensed by neither a
proof nor a NO — it is licensed by an **attested dead end** plus a disclosure, which is a weaker
warrant, and pretending otherwise would be the kind of quiet restatement this repo's folds get caught
on. The half that is unchanged and load-bearing is the second sentence: **silence still never
proceeds**, in any row, on any leaf. Three of the eight rows below `DOB≥24` are refusals *caused by an
absent answer*, and the new row needs two `Some`s to escape.

★ And note which direction the two remaining *computed* conditions push. Condition 2 assumed TRUE and
`unearned` computed too high (§5.2) both make the conjunction **more** true. There is no computed
term in this gate whose error can silently exempt a filer, other than a wrong date of birth §5.1
already names.

### 6.2 Provenance — what each new leaf is, for the census

`crates/btctax-core/src/tax/classifier.rs:264-285` destructures `HouseholdHeader` with no `..`, so all
**three** new leaves are a compile error until classified. None is a `Census::declaration` (that is
reserved for `FORM_QUESTIONS`), and none is a `BenefitClaim` — no benefit is claimed by answering.
Use the `Class::NoTaxDirection` idiom already established for `qbi_w2_wages` / `qbi_ubia`
(`classifier.rs:498-509`), whose reason string is exactly this shape — *refused where it is needed,
unread where it is not, so it defaults in neither direction*:

```rust
c.exempt(
    form8615_condition3_age_support,
    Class::NoTaxDirection,
    "Form 8615 condition 3 (i1040gi:3932-3940) — `None` is REFUSED by screen_compute_dependent \
     wherever conditions 1 and 5 hold and age 24+ is not provable, and is unread everywhere else, \
     so it defaults in neither direction",
);
```

★ **r2 — the third leaf takes the same class for a sharper reason.**
`form8615_parent_identity_unobtainable` is `NoTaxDirection` because `None` and `Some(false)` are the
**same outcome** (R-4 refuses), so its silence cannot move a number in either direction; only
`Some(true)` does anything, and that is the filer's own affirmative testimony. Its reason string must
say that, because it is what makes the leaf safe:

```rust
c.exempt(
    form8615_parent_identity_unobtainable,
    Class::NoTaxDirection,
    "the §6.3 dead-end fact — `None` and `Some(false)` are the SAME outcome (Form8615ParentUnidentifiable \
     refuses), so silence defaults in neither direction; only the filer's own `Some(true)`, and only \
     after they answered condition 4 CannotKnow, opens the certification path",
);
```

`form8615_condition4_parent_alive` is `Option<ParentAliveAnswer>`, not `Option<bool>`, so the census
helper it uses must accept a non-bool `Option`. **`CannotKnow` is an ANSWER for census purposes** —
answered-ness is about whether the filer spoke, not about which way — and the classifier must not be
allowed to treat it as a blank. §9 **G13** pins that, because a census that scored `CannotKnow` as
unanswered would report this spec's central case as a hole and invite someone to "fix" it.

**OQ-1** asks whether that stretch of `NoTaxDirection` (documented as *"a lawful silent default"*,
`classifier.rs:50-51`) deserves its own variant now that four leaves share the pattern.

---

### 6.3 THE NO-PATH CERTIFICATION — the owner ruling, implemented

★★★ **Source of authority: `design/ty2025/DECISION_form8615_no_path_self_certification.md`
(`395ab293`), an owner ruling. It is not a design choice this spec may revisit.** Owner, verbatim:
"If tax system has no path, that is why we allow user to self-certify with warnings."

### 6.3.1 The dead end, from the primary sources

Every route to Form 8615 closes for a filer who cannot identify their parents. This is established
from the archive, not reasoned:

| the wall | primary source | why it is a wall |
|---|---|---|
| the form's face | `design/forms/extract/f8615--2025.txt:17-20` — line **A** "Parent’s name (first, initial, and last)", **B** "Parent’s social security number", **C** "Parent’s filing status (check one):" | three structural fields, none marked optional |
| the computation | `legal/primary-sources/statute-irc/26USC_s1.html`, §1(g)(3)(A)(i) — the allocable parental tax is figured on "the parent's taxable income" | Part II cannot begin without a figure only the parent has |
| the administrative remedy | `design/forms/extract/i8615--2025.txt:184` and `:193-195` | the request "must contain all of the following", including "The name, address, social security number (SSN) (if known), and filing status (if known) of the parent". **SSN and filing status tolerate ignorance; name and address do not.** It also requires "A statement that you are making the request to comply with section 1(g) of the Internal Revenue Code and that you have tried to get the information from the parent" (`:185-187`) |
| the statutory escape | `design/forms/extract/i8615--2025.txt:60-61` — the rules "don’t apply if neither of the child’s parents were living at the end of the year" | requires establishing a fact about parents the filer cannot identify — no easier than condition 4 itself |

**This is a gap in the tax system, not in btctax.** No amount of collection closes it, because the
missing input is held by a person the filer cannot name.

### 6.3.2 What btctax does, and what it refuses to do

**btctax's position, in btctax's voice:** on these facts, §1(g) is **not established** — the predicate
cannot be evaluated, and the government's own remedy for that excludes this filer by its own required
contents. The return is filed **without** Form 8615, **with** a Form 8275 disclosure.

**The filer's testimony, in the filer's voice:** two facts, and only facts —

1. condition 4 = `CannotKnow` (§4): *they cannot find out whether either parent was alive*;
2. the dead-end fact = `Some(true)` (§4): *they cannot give the IRS either parent's name and address*.

★★★ **What the filer is NEVER asked to attest.** Not *"§1(g) does not apply to me"*, not *"I am
exempt from the kiddie tax"*, not *"Form 8615 is not required"*. Those are **legal positions**, and a
return is signed under §6065 — putting a conclusion in the filer's mouth would be fabricating
testimony, which this repo's *an entry is testimony* rule forbids outright. The conclusion is
btctax's, it is stated in btctax's voice on Form 8275, and the filer's signature covers the facts they
actually gave. §9 **G14** pins that no prompt, help string or disclosure line asks the filer to affirm
a conclusion.

### 6.3.3 The Form 8275 disclosure — exact content

**The instrument is Form 8275, not 8275-R.** §1(g) is a **statute**; Form 8275-R is for positions
contrary to **regulations** — the form itself says so on its face
(`design/forms/extract/f8275--2024.txt:6` and `:9` — the OMB number sits between them in the text
layer): *"Don’t use this form to disclose items or positions that are contrary to Treasury …
regulations. Instead, use Form 8275-R, Regulation Disclosure Statement."*
The module already exists: `crates/btctax-core/src/tax/form8275.rs`.

**Part I, one row**, mapped to the form's own columns
(`design/forms/extract/i8275--2024.txt:352-374`):

| column | instruction | content |
|---|---|---|
| (a) | *"If you are disclosing a position contrary to a rule (such as a statutory provision or IRS revenue ruling), you must identify the rule in column (a)."* (`:352-354`) | `IRC section 1(g)` |
| (b) | *"Identify the item by name."* (`:355`) | `Tax on unearned income of a child — Form 8615 not filed` |
| (c) | *"Enter a complete description of the item(s) you are disclosing."* (`:361-362`) | `The taxpayer's tax was figured without regard to section 1(g). The taxpayer cannot identify either parent and therefore cannot supply the parent's name, SSN or filing status required by Form 8615 lines A, B and C, cannot obtain the parent's taxable income required by section 1(g)(3), and cannot make the request described in the Instructions for Form 8615 under "Parent's return information unavailable", which requires the parent's name and address.` |
| (d) | form or schedule (`:371-374`) | `1040` |
| (e) | line no. (`:371-374`) | `16` |
| (f) | amount (`:371-374`) | the tax on Form 1040 line 16 as filed — i.e. the amount computed **without** §1(g) |

**Part II** must, per `design/forms/extract/i8275--2024.txt:378-388`, *"include information that can
reasonably be expected to apprise the IRS of the identity of the item, its amount, and the nature of
the controversy or potential controversy"*, and *"can include a description of the legal issues
presented by the facts"*. Three paragraphs, in this order:

1. **The facts, restated as the filer gave them** — the filer cannot identify either parent; the
   filer cannot supply either parent's name and address; the filer's unearned income for the year was
   `${unearned}`, above the §1(g) threshold of `${threshold}`.
2. **The impossibility argument, which LEADS** — Form 8615 cannot be completed and §1(g)(3) cannot be
   computed on these facts, and the sole administrative remedy the IRS provides for a child who
   cannot get the parent's information is unavailable because its own required contents include the
   parent's name and address, which are the very facts the filer lacks. ★ The ruling directs that this
   argument lead: *"the impossibility argument (the predicate cannot be established, and the
   government's own remedy excludes the filer by its own required contents) is materially stronger
   than a constitutional one and should lead."*
3. **What was and was not done** — the return reports all of the filer's income; the position is
   limited to the *rate* at which the unearned portion is taxed; no exclusion, deduction or credit is
   claimed on account of it; and the filer will complete Form 8615 if the parent's information later
   becomes available.

★★★ **No constitutional argument appears anywhere in the disclosure.** The ruling records that the
controller is aware of **no authority** holding §1(g) invalid as applied here, and a disclosure that
led with a constitutional theory would be weaker, not stronger. Do not add one.

**P-3 (plan task).** `disclosure_8275` (`crates/btctax-core/src/tax/form8275.rs:108-165`) builds
Part I **only** from promoted Form 8949 disposal legs — its loop is scoped to
`state.promoted_origins` — and returns `None` when there are none (`:151-152`); `Part1Item`
(`:53-67`) carries `form` / `line` / `description` / `amount` with no column (a) or (b). So this row
needs a **second Part I item source** and two more fields, and `disclosure_8275` must stop returning
`None` when the only item is a §1(g) one. The task is required by the ruling, not invented here; it is
the only new task r2 adds.

### 6.3.4 The three warnings — mandatory, and one is verbatim

All three are surfaced **before** the return is produced, not buried in the PDF. They are not
softenable.

**W-1 — a disclosure is not a shield.** Adequate disclosure protects against the §6662
accuracy-related penalty **only if the position has a reasonable basis**, and *"reasonable basis"* is
a real standard, not a formality. From `design/forms/extract/i8275--2024.txt:202-214`:

> *"Adequate disclosure. Generally, you can avoid the disregard of rules and substantial
> understatement portions of the accuracy-related penalty if the position is adequately disclosed and
> the position has at least a reasonable basis."*
>
> *"Reasonable basis. Reasonable basis is a relatively high standard of tax reporting that is
> significantly higher than not frivolous or not patently improper. The reasonable basis standard
> isn’t satisfied by a return position that is merely arguable."*

btctax does **not** tell the filer their position clears that bar. It tells them the standard exists
and that the disclosure is conditional on it.

**W-2 — there is no authority holding this.** Verbatim from the ruling: btctax "is aware of NO
authority holding §1(g) invalid as applied here". The filer is told that the argument being made is an
**impossibility** argument — the predicate cannot be established, and the government's own remedy
excludes them by its own required contents — that this is materially stronger than a constitutional
argument, and that it is nonetheless untested.

**W-3 — the cost of being right, in the owner's own words, VERBATIM:**

"often the process is the punishment when it comes to not allowing government to treat you
unlawfully."

That sentence ships as written. It is the only honest thing btctax can say about what happens next:
being correct does not prevent an examination, correspondence, or the time and expense of answering
one.

### 6.3.5 Where to go for help — the Taxpayer Advocate Service, VERIFIED

★ The ruling named TAS as a **candidate** whose intake criteria were **not verified**, and required
verification before it ships as advice. **It is now verified against a primary source in the archive**
— `design/forms/extract/i1040gi--2025.txt:147-166`, the 1040 instructions' own TAS page — and the
finding is precise enough to matter:

- The three **bulleted** criteria (`:157-159`) are financial difficulty, an immediate threat of
  adverse action, and the IRS not responding. **None of them describes this filer**, who has not yet
  contacted the IRS about anything.
- The limb that **does** cover them is in the introductory sentence (`:153-154`): *"TAS can help you
  if your tax problem is causing a financial difficulty, you’ve tried and been unable to resolve your
  issue with the IRS, or you believe an IRS system, process, or procedure just isn’t working as it
  should."* A remedy whose required contents exclude the people it exists for is exactly *"an IRS
  system, process, or procedure … isn’t working as it should"*.

So the advisory cites the third limb, not the bullets, and says how to reach TAS using the contacts the
same page prints (`:161-166`): `TaxpayerAdvocate.IRS.gov/Contact-Us`, or the toll-free line
`877-777-4778`. **Nothing beyond that page is asserted** — no claim about how TAS will treat the case,
no timeline, no outcome.

---

## 7. Migration — an existing vault has none of this

`#[serde(default)]` makes all three leaves `None` on every vault written before this change. That is
the correct representation: the filer has not been asked.

**What must NOT happen.** No backfill. Writing `Some(false)` into existing vaults — or defaulting the
leaves to `false` in the struct — would be btctax answering a Form 8615 condition on the filer's
behalf, which is FR-29's own defect committed a second time, and the *an entry is testimony* rule
forbids it. There is no migration script. ★ For `form8615_condition4_parent_alive` this binds twice
over: no `#[serde(default)]` may name a `ParentAliveAnswer` variant, and in particular defaulting to
`CannotKnow` would hand every legacy vault the *entrance* to the §6.3 dead-end path without anyone
having said anything — the certification would then be one further answer away for filers who never
had a dead end at all.

**What happens instead**, for a vault with more than the threshold of unearned income, no date of
birth (or one under 24), and a non-joint status: `btctax report` / `export` return
`ProfileOutcome::Uncomputable` (`crates/btctax-cli/src/resolve.rs:188-205`) carrying
`Form8615AgeSupportUnanswered`'s detail, which names the missing answer and the remedy. **The old
behaviour is not silently kept** — it cannot be, since the gate no longer reads the flag it used.

The filer's exits, all of them cheap:

1. `btctax income answer` and answer condition 3 (one keystroke, and if No, nothing else is asked); or
2. give a date of birth at the same prompt (`SkippableId::DobTaxpayer`, `questions.rs:1003-1013`),
   which suppresses both questions permanently if it shows 24 or older at year end; or
3. answer both YES and receive `KiddieTax`, which is the correct outcome — Form 8615 is required and
   btctax does not fill it.

**Fixtures are migrated by making them realistic, not by silencing the gate.** The 15 CLI fixtures in
§10 are adult filers; give them a date of birth, which both fixes them and exercises §5.1's
suppression path. Reserve explicit `Some(false)` answers for fixtures that are *about* a young filer.

★★ **r2 (M-4) — bound that date of birth, and the bound the review suggested is one year loose.** A
DOB showing 65 or older at year end switches on the §63(f) aged standard-deduction addition
(`return_1040.rs:89-91`, `:110-122`), which moves the standard deduction on the four named oracle
households — the goldens red loudly, and *"regenerate the golden"* is then the tempting repair this
repo has a starred rule against. The review proposed `[year − 64, year − 24]`. **`born_early_enough`
is `dob <= Date(year − 64, January 1)` (`return_1040.rs:89-91`), so a January-1 birth in `year − 64`
IS 65 or older** and the lower bound is off by one. The correct instruction: **choose a date of birth
whose calendar year is in `[year − 63, year − 24]`.** Both ends are exact — `year − 63` is the first
year no date in which can reach the §63(f) cutoff, and `year − 24` is the last year every date in
which yields a considered age of at least 24 (§5.1), so the whole band suppresses conditions 3 and 4
without touching the standard deduction.
**Do not add the answers to the shared builders in `crates/btctax-core/src/tax/testonly.rs`** — a
blanket `Some(false)` there would green every future fixture by default and destroy the gate's
ability to catch this class again.

---

## 8. Refusals — exact wording and firing conditions

**Three** new `RefuseReason` variants (`return_refuse.rs`) plus a reworded `KiddieTax`. Adding a
variant reds the exhaustive cross-crate match in `crates/btctax-input-form/src/attribute.rs` — the
free, exact blast radius this repo prefers.

★ The subsections below run in **§6 ladder order**, not numeric order: R-1 (step 3), R-2 (step 4),
R-4 (step 6), R-3 (step 7). R-3 keeps its r1 number because it is the pre-existing `KiddieTax`.

### R-1 `Form8615AgeSupportUnanswered`

**Fires:** condition 1 ∧ condition 5 ∧ ¬`provably_24_or_older` ∧ `form8615_condition3_age_support ==
None`.

**Detail** (`format!`ed at the gate; `{year}` and `{threshold}` are in scope):
```text
your unearned income is over the §1(g) threshold of ${threshold}, so Form 8615 may be required —
and its condition 3 asks whether, at the end of {year}, you were (a) under age 18, (b) age 18 and
didn’t have earned income that was more than half of your support, or (c) a full-time student at
least age 19 but under age 24 and didn’t have earned income that was more than half of your
support. btctax will not answer that for you: §1(g)(1) takes the GREATER of your own rate and the
parent's-rate figure, so a wrong "no" can only understate your tax. Run `btctax income answer` —
answering "no" clears this, and so does entering your date of birth there if you were 24 or older
at the end of {year}.
```

**Anchor** (`attribute.rs`): `vec![skip(SkippableId::Form8615Condition3AgeSupport)]`, matching the
`CooperativePatronUnanswered` precedent at `attribute.rs:70-72`.

### R-2 `Form8615ParentAliveUnanswered`

**Fires:** the R-1 conditions except `form8615_condition3_age_support == Some(true)` ∧
`form8615_condition4_parent_alive == None`.

**Detail:**
```text
you answered YES to Form 8615's condition 3, so whether Form 8615 is required now turns on its
condition 4 — "At least one of your parents was alive at the end of {year}." btctax will not answer
that for you; a wrong "no" understates your tax by the whole §1(g) difference. Run
`btctax income answer`. If you cannot find out because you do not know who your parents are, answer
CANNOT KNOW — that is a different answer from leaving it blank, and it is not the same as "no".
```

**Anchor:** `vec![skip(SkippableId::Form8615Condition4ParentAlive)]`.

### R-4 `Form8615ParentUnidentifiable` — r2, the dead-end refusal

**Fires:** the R-2 conditions except `form8615_condition4_parent_alive == Some(CannotKnow)` ∧
`form8615_parent_identity_unobtainable != Some(true)` (i.e. `None` **or** `Some(false)`).

★ **Two different filers land here, and the detail must serve both** — one who has not yet answered
the second question (§4.2 makes it live only now, so this is their first sight of it), and one who
answered that they *can* supply a name and address, for whom the IRS route is open and this refusal is
correct and final until they use it.

**Detail** (`format!`ed at the gate):
```text
you answered that you cannot find out whether either of your parents was alive at the end of
{year}. Form 8615 cannot be completed without a parent's name, SSN and filing status (lines A, B and
C), and the tax on it cannot be computed without that parent's taxable income — so btctax needs to
know one more thing: can you give the IRS your parent's name and address? Run `btctax income answer`
again; the question is only offered now that you have answered condition 4.
If you CAN: the Instructions for Form 8615 let you request your parent's return information from the
IRS, and that request requires the parent's name and address (the SSN and filing status may be
unknown). Use that route — btctax cannot file Form 8615 for you either way.
If you can name and locate a parent but cannot contact them — for example, a protection order
forbids it — that route is closed to you as well, because the request must also state that you
"have tried to get the information from the parent". btctax has no path for you and will not pretend
otherwise. The Instructions for Form 1040 say "The Taxpayer Advocate Service (TAS) is an independent
organization within the Internal Revenue Service (IRS) that helps taxpayers and protects taxpayer
rights", and that "TAS can help you if your tax problem is causing a financial difficulty, you’ve
tried and been unable to resolve your issue with the IRS, or you believe an IRS system, process, or
procedure just isn’t working as it should". Go to TaxpayerAdvocate.IRS.gov/Contact-Us, or call
877-777-4778. Good luck.
If you CANNOT give the IRS a name and an address for either parent, answer "no" and btctax will file
your return without Form 8615 and attach a Form 8275 disclosure explaining why. Read that disclosure
before you file: it is a disclosed position, not a ruling in your favour.
```

★★★ **r3 (I-2) — the TAS paragraph is the OWNER'S RULING on OQ-5, discharged where the ruled filer
actually lands.** `design/OWNER_DECISIONS_2026-09-04.md`, *"OWNER RULINGS, 2026-09-05"* (`92014cd1`),
owner verbatim: *"just refer user for tas and tell them good luck."* ⇒ the protection-order filer gets
**a refusal that names the Taxpayer Advocate Service** and is honest that btctax cannot carry them
further. §13 already refuses that filer and explains why the certification is **not** widened to reach
them; what was missing was the referral, and R-4's detail is the only text that filer ever reads.
★ Sourcing, and it does not move: the quoted limb is the **third in the introductory sentence** at
`i1040gi--2025.txt:153-154`, not the three bullets at `:157-159` — none of which describes a filer who
has not yet contacted the IRS. What TAS *is* is transcribed from `:149-150` rather than described in
btctax's words, and the contacts are the ones the same page prints, `:162` and `:166`.
**Nothing beyond that page is asserted**: no claim about how TAS will treat the case, no timeline, no
outcome. ★★ And note what this paragraph is **not**: it is not a second limb of the §6.3 gate. It
changes no predicate, adds no leaf and opens no path — the filer is still refused. §9 **G15** pins
that both halves stay true together.

**Anchor:** `vec![skip(SkippableId::Form8615ParentIdentityUnobtainable)]` — the same shape as R-1 and
R-2, and the reason it must **not** be `NotInForm`: an input *does* clear this one, and an anchor
saying otherwise is the `QbiAboveThreshold` falsehood recorded at
`crates/btctax-input-form/src/attribute.rs:224-231` (*"an anchor saying the refusal has no form field
is a FALSEHOOD that leaves the filer with nowhere to go. It was one, and a green test pinned it."*).

### R-3 `KiddieTax` — kept, reworded

**Fires:** all five conditions (ladder step 7).

**Detail** (replacing `return_1040.rs:1000`, which today says *"a claimable-as-dependent filer …"*).
★★ **r2 (I-4) — the r1 wording asserted as fact a condition §5.4 knows btctax never established.** It
told the filer they met *"all five"* of the conditions, *"a filing requirement"* among them, while
§5.4 assumes condition 2 and names two populations for whom it is false — a non-dependent below Chart
A's threshold (`i1040gi--2025.txt:634-639`, `$15,750` single under 65) and a blind dependent under 24
(`:705`, `$3,350`). For those filers r1's refusal stated a falsehood **and** instructed them to
complete a form they are not required to file, with `KiddieTax` being a refusal no input can clear.
The assumption was pinned only in a *test assertion message* (G6); the sentence the filer actually
reads was unpinned. The fix is to disclose the assumption, which costs nothing and is independent of
OQ-2:
```text
you meet Form 8615's conditions for {year}: more than ${threshold} of unearned income, the
condition-3 age and earned-income-support test, at least one parent alive at the end of the year,
and a return that is not joint. btctax also ASSUMES you are required to file a return (Form 8615's
condition 2) — it does not compute that. If your gross income is below the Chart A or Chart B
threshold for your filing status, you may not be required to file at all, and then Form 8615 is not
required either; check those charts in the Instructions for Form 1040 before acting on this.
Otherwise: §1(g) taxes part of your unearned income at your parent's rate — Form 8615 computes the
greater of that and your own — and btctax does not fill Form 8615, because it needs your parent's
taxable income and any siblings' §1(g) amounts, which btctax never sees. Complete Form 8615 by hand.
```

**Doc comment** at `return_refuse.rs:264-273`: the FR-29 warning block is **deleted and replaced** with
the corrected reading and a pointer to this spec. Leaving the warning in place after the fix would be
its own defect — a doc telling a future maintainer the gate is wrong when it is right. ★ **r2 (C-1(a)):
the replacement quotes i8615 rather than arguing from an absence** — `i8615--2025.txt:59-60`,
*"These rules apply whether or not the child is a dependent."* The doc comment today reasons
that dependency *"appears nowhere in"* the five conditions; that is true and weaker than the IRS
saying so, and this is the one sentence most likely to stop FR-29 recurring.

**Anchor:** unchanged (`attribute.rs:221-223`, `NotInForm`) — the note's wording is updated to say the
screen is computed at `report` from the condition declarations. `KiddieTax` remains a refusal no
input can clear (like `CooperativePatron`), so it keeps its single `NotInForm` anchor and the existing
`deferred_and_defensive_refusals_are_not_in_form` test (`attribute.rs:427-451`) stays green unchanged.
★ **R-1, R-2 and R-4 must NOT be added to that test's list** — all three are cleared by an input, and
adding them would make the test assert the falsehood R-4's anchor note exists to avoid.

### 8.1 What the certification path emits instead of a refusal

The §6.3 row is the one outcome in this spec that is **not** a `RefuseReason`: the return computes and
files. What it emits is an **advisory plus a disclosure**, and the advisory is not optional decoration
— W-1, W-2 and W-3 (§6.3.4) and the TAS pointer (§6.3.5) are surfaced at `report` and at `export`,
before the filer has a PDF in hand. §9 **G14** asserts that a return produced on this path carries all
three warnings and a Form 8275 disclosure, so a future change that drops the disclosure cannot leave a
silently-undisclosed position behind — which would be strictly worse than the refusal it replaced.

---

## 9. How it is tested

Every guarantee below names the mutation that must make it RED (harness rule **B1**, `CLAUDE.md`).
"Red" means the named test fails, not that some test somewhere fails.

**G1 — dependency is not a condition.** `form8615_screens_a_filer_who_is_nobodys_dependent`: a filer
with `can_be_claimed_as_dependent_taxpayer = Some(false)`, unearned above the threshold, condition 3
= `Some(true)` and condition 4 = `Some(ParentAliveAnswer::Yes)` ⇒ `KiddieTax`.
★ **r2 (M-1):** the r1 name carried an apostrophe (`…nobody's_dependent`), which is not a legal Rust
identifier — Rust 2021 reserves the prefix and `rustc` rejects it. Renamed, not paraphrased.
**Mutation:** re-introduce `if ri.header.can_be_claimed_as_dependent_taxpayer != Some(false)` around
the gate ⇒ red.
★ **This is the INVERSION of the assertion at `return_1040.rs:4473-4475`** — the one FR-29 flagged as
*"kept RED-ADJACENT on purpose"*. That assertion and the FR-29 comment block above it
(`:4465-4472`) are **replaced**, not deleted, so `git diff` shows a claim being corrected rather than
a test disappearing.

**G2 — silence never proceeds.** `form8615_unknown_conditions_fail_closed`: a table over the **nine**
rows of §6.1, asserting the exact outcome of each.
**Mutations:** change any `None` arm of the ladder to proceed ⇒ red on that row. Change
`c3 == Some(false)` to `c3 != Some(true)` (the classic widening) ⇒ red on the `None` row. ★ And the
r2 widening that matters most: change the certification gate from
`form8615_parent_identity_unobtainable == Some(true)` to `!= Some(false)` ⇒ red on the
`Some(CannotKnow)` / `None` row, because silence would then certify.

**G3 — the age arithmetic is the form's.** `considered_age_matches_i1040gi_january_first_examples`:
the three printed examples for TY2025 — born `2008-01-01` ⇒ 18, `2007-01-01` ⇒ 19, `2002-01-01` ⇒ 24 —
plus the neighbours `2002-01-02` ⇒ 23 (asked) and `2002-01-01` ⇒ 24 (never asked, never refused), plus
a leap-day vector `2004-02-29` ⇒ 21.
**Mutations:** drop the `January && day == 1` term ⇒ the `2002-01-01` row flips to 23 and the filer is
asked ⇒ red. `>= 24` → `> 24` ⇒ the `2002-01-01` row is asked ⇒ red. `year - dob.year()` →
`year - dob.year() - 1` ⇒ every row shifts ⇒ red.

**G4 — condition 5.** `a_joint_return_is_never_screened_for_form_8615`: MFJ with everything else true
⇒ no refusal; **and a QSS vector with everything else true ⇒ `KiddieTax`.**
**Mutations:** delete the `!= Mfj` term ⇒ red on the MFJ row. Widen it to `!= Mfj && != Qss` ⇒ red on
the QSS row.
★ **r2 (N-3):** §5.3 calls QSS out by name — *"QSS is not a joint return and is therefore not
exempted by this condition"* — and r1 pinned MFJ only, so the sentence that names the trap had no
test under it. A qualifying surviving spouse files a return of their own, not a joint one; §1(g)(2)(C)
excludes only a child who *"does not file a joint return"*.

**G5 — condition 1's strictness and its source.** `the_threshold_is_strict_and_comes_from_the_params`:
unearned exactly equal to `params.kiddie_unearned_threshold` ⇒ proceed; one cent more ⇒ refuse; and the
same fixture under a params value moved by $100 moves the boundary with it.
**Mutations:** `>` → `>=` ⇒ red. A literal `dec!(2600)` in place of the param ⇒ red on the third
assertion.

**G6 — the named over-refusal branch of condition 2** (§5.4).
`condition_two_is_assumed_and_this_is_the_filer_it_over_refuses`: a non-dependent single filer, gross
income below Chart A's threshold, conditions 1/3/4/5 true ⇒ `KiddieTax`, with the assertion message
stating that the form would **not** require Form 8615 here and citing `i1040gi:634-639`. A second
vector does the same for the blind dependent of `:705`.
This test exists to make the assumption **visible and deliberate**; it is the KAT `CLAUDE.md` requires
alongside a derived form's equivalence proof.
**Mutation:** implement condition 2 without updating this test ⇒ red, and the test is then rewritten
rather than deleted.

**G7 — prompt conformance against the extract.** A new `cargo run -p xtask -- prompt-check`, beside
`cite_check` (`crates/xtask/src/cite_check.rs`), owning a table of
`(SkippableId, &str /* extract path */, &[&str] /* clauses */)` and asserting, for each clause,
**both**:
(a) it appears verbatim (normalised) in **the extract that clause is sourced from**, and
(b) it appears verbatim in that question's `prompt`.

★★ **r2 (I-3) — "normalised" is now DEFINED, and it is defined by naming the function rather than by
describing it.** Normalisation is `crates/xtask/src/cite_check.rs:71-136`'s `normalise`: it folds
Unicode punctuation to ASCII, strips markdown emphasis and quote marks, removes the CAUTION/TIP icon
labels, de-hyphenates across line breaks, replaces every non-alphanumeric other than `$`, `%` and
whitespace with a space, collapses runs of whitespace, **and lowercases**. r1 said "verbatim
(normalised)" without saying what that meant, and the review's measurement is the reason this matters:
three of five r1 clauses differ in **case** from one side or the other and three span **newlines** in
the extract, so an implementer transcribing G7 literally would get a red check on day one — and the
cheapest repair in front of them would be to edit the clause list, which guts the instrument.
`prompt-check` **reuses `normalise` directly**; it does not define a second one.

★ **The check does NOT become performatively satisfiable by saying so.** Case-folding and
whitespace-collapsing leave the B1 pairing red: *"most of your support"* still does not contain
*"more than half of your support"* under any amount of case folding.

**Clauses, each with its source** (★ r2 — the r1 table named one extract for all of them, and the two
documents word limb (c) differently: i8615 says *"and under age 24"* where i1040gi says *"but under
age 24"*, so a single-extract table would either fail or silently check the prompt against the wrong
document):

| # | question | clause | sourced from |
|---|---|---|---|
| 1 | condition 3 | `"under age 18"` | `i1040gi--2025.txt:3933` |
| 2 | condition 3 | `"age 18"` | `i1040gi--2025.txt:3934` |
| 3 | condition 3 | `"didn’t have earned income that was more than half of your support"` | `i1040gi--2025.txt:3935-3936` |
| 4 | condition 3 | `"a full-time student at least age 19 but under age 24"` | `i1040gi--2025.txt:3937-3938` |
| 5 | condition 4 | `"at least one of your parents was alive"` | `i1040gi--2025.txt:3941-3942` |
| 6 | condition 3 help | `"these rules apply whether or not the child is a dependent"` | `i8615--2025.txt:59-60` |
| 7 | condition 3 help | `"wages, tips, and other payments received for personal services performed"` | `i8615--2025.txt:266-267` |
| 8 | the dead-end fact help | `"the name, address, social security number (SSN) (if known), and filing status (if known) of the parent"` | `i8615--2025.txt:193-195` |

**All eight were machine-checked against both halves before this table was written** — each clause,
normalised with `cite_check::normalise`, is contained in its named extract **and** in the §4.1 string
it belongs to. Clauses 5 and 6 failed on the first run and are why §4.1's condition-4 prompt is
declarative and its condition-3 help quotes i8615 directly rather than rewriting it into the second
person. **Do not "simplify" either string back**; the check is what holds them.

★★ **r3 (C-1(d)) — G7 IS BUILDABLE TODAY, and the r2 claim that it had a prerequisite was false.**
The r2 text said clauses 6–8 cited *"different columns of the same physical lines"* and that this made
**P-2a a hard prerequisite of G7** — *"against the raw two-column extract, clauses 6 and 7 cannot pass,
because each spans two lines of one column and the haystack interleaves the other."* Re-measured at
HEAD, all eight clauses through `cite_check::normalise` against the **raw** current extracts and
against the §4.1 string each belongs to:

```
=== G7 clause vs EXTRACT (current, raw, no column split) ===   === G7 clause vs PROMPT/HELP (§4.1) ===
  clause 1..8: PASS  (8/8)                                       clause 1..8: PASS  (8/8)
=== G7 structural counts on the condition-3 prompt ===
  clause 3 occurrences (want 2): 2
  'a under age 18' (want 1): 1   'b age 18 and didn t have' (want 1): 1   'c a full time student' (want 1): 1
```

**Nothing about G7 is blocked, and the per-clause source column stays** — not because of columns, but
for the reason §1.1 gives: i8615 says *"and under age 24"* where i1040gi says *"but under age 24"*, so
a single-extract table would silently check a prompt against the wrong document.

★★ **Clause 2 is WEAK, and G7 must not pretend otherwise.** `"age 18"` is a substring of `"under age
18"`, so clause 1 satisfies it and **deleting limb (b) from the prompt would not red anything.** Limb
(b) cannot be pinned textually: the prompt's hoist means the extract's *"Age 18 at the end of 2025
and"* and the prompt's *"age 18 and"* share no span longer than `"age 18"` itself. So G7 carries a
**structural** assertion alongside the clause table, which is what actually pins the three limbs:

- the normalised condition-3 prompt contains clause 3 **exactly twice** — once for limb (b), once for
  limb (c), which is how the form writes it (measured: 2); and
- it contains each of the three normalised limb openings **exactly once**: `a under age 18`,
  `b age 18 and didn t have`, `c a full time student` (measured: 1, 1, 1).

Deleting limb (b) reds on both counts. ★ **The second assertion is prompt-only** — those spans are the
prompt's own hoisted phrasing and are *not* in the extract, so it detects a limb being **dropped**,
never a limb **drifting** from the form. Say that in the check's own message, because a structural
assertion that looks like a conformance assertion is precisely the green-and-blind instrument this
whole section exists to avoid. ★ Do **not** assert on the bare markers `(a)`/`(b)`/`(c)`: `normalise`
strips the parentheses, and the trailing sentence *"Answer YES if any one of (a), (b) or (c) is
true."* makes their counts 3/2/2, not 1/1/1 — measured, and it is why this bullet names full openings
instead.

**Why both halves.** (b) alone lets the clause table drift from the form; (a) alone lets the prompt
drift from the table. Together, a paraphrase anywhere reds — the property B1 calls *"cannot be
satisfied performatively"*.
**B1 pairing (mandatory):** `prompt_check_rejects_a_paraphrased_prompt_and_accepts_the_real_one`,
modelled on `cite_check.rs::a_paraphrase_is_rejected_and_the_real_sentence_is_accepted`, planting
*"more than half of your support"* → *"most of your support"* and asserting red — **and asserting that
the unmutated clause passes**, because a checker that reds on everything is indistinguishable from one
that works. ★ **r3:** that half is now *measurable* rather than aspirational — §1 records 8/8 clauses
passing against the raw extracts at HEAD, so the "accepts the real one" assertion is known to be
satisfiable before the test is written.

**G8 — the demanded set is inside the live set (the anti-brick invariant).**
`every_form8615_refusal_names_a_question_the_interview_would_have_offered`: over a fixture grid that
crosses {no DOB, DOB 20, DOB 30} × {Single, MFJ, MFS, **QSS**} × {unearned from wages only, from
interest only, **from crypto only**} × {all three answers `None`, **and condition 4 =
`Some(CannotKnow)` with the dead-end fact `None`**} × {`tax_year == year`, **`tax_year == year + 5`**,
`tax_year == 0`}, assert that whenever the gate returns R-1, R-2 **or R-4**, the corresponding
`SkippableQuestion::live` is `true` for that same `ReturnInputs`.
**Mutations:** add any income-dependent term to any liveness predicate ⇒ red on the crypto-only
fixture. Drop `form8615_condition4_parent_alive == Some(CannotKnow)` from the dead-end fact's liveness
and replace it with `!= Some(Yes)` ⇒ **not** red here (the live set only grows) — which is why G14
carries the opposite half, that the question is *not* offered when condition 4 is unanswered.
This is the test that would have caught the shipped circular-liveness bug of `questions.rs:344-345`,
and it is the reason §3.1 chose this shape.
★★ **r2 (M-2) — the `tax_year != year` rows are the point of the grid, and they replace a prose
claim.** §4.2's r1 argument that the divergence is one-directional held only for `Default`'s `0`; a
hand-built `tax_year > year` inflates the computed age, suppresses the question, and bricks the filer.
The assertion is the **invariant** — every demanded question is live — evaluated over rows where the
two years disagree in **both** directions, so the case r1 argued away is now a row that can fail.

**G9 — registry bookkeeping.** The existing counts move and must be *moved*, not deleted. ★ r2
corrects three things here: the counts are **+3**, not +2; there are **two** `93`s, not one; and the
one file where the bookkeeping is *not* compiler-checked was missing entirely.

| site | address | change |
|---|---|---|
| `SKIPPABLE_QUESTIONS.len()` | `crates/btctax-core/src/tax/questions.rs:1490-1496` | 16 → **19** |
| ★ that test's **name** and its enumerating message | `questions.rs:1490` (`…_has_five_entries_with_correct_liveness` — already stale at 16) and `:1493-1496` | both edited; **N-2** |
| the `Skippables` section list | `crates/btctax-input-form/src/spec/mod.rs:304-324` | three `FieldId`s added |
| ★★ **`registries.rs` — the ONLY un-compiler-checked half** | `crates/btctax-input-form/src/spec/registries.rs` | two `skippable_tristate!(16, …)` / `(18, …)` and one `skippable_choice!(17, …)` call site, three `FieldId` variants, three `skippable_to_field` arms (`:457-476`, exhaustive ⇒ the arms and variants are compile-forced; **the literal indices are not**) |
| the coverage KAT's **two** `93`s | `crates/btctax-input-form/src/spec/coverage.rs:429` (`field_count`) and `:434` (`covered.len()`) | both 93 → **96**; **N-1** |
| its `(FieldId, path)` rows | `coverage.rs:578-582` for the idiom | three added |

`QuestionId::ALL`'s 17 is **unchanged** — no class-(A) declaration is added.

**G10 — the corpus floor holds.** `crates/btctax-oracle-harness/tests/smoke.rs:164-173` already
asserts `admitted >= 11` and `refused == EXPECTED_REFUSED`. §10 measured four households newly
refused. **Those fixtures get dates of birth in §7's `[year − 63, year − 24]` band; the assertions are
not relaxed.** A weakened floor here would hide exactly the regression the floor exists to catch.

**G11 — §5.2's direction claim has a NAMED dependency, and it is pinned (r2, C-1(d)).**
`the_unearned_sum_is_only_conservative_because_out_of_scope_income_refuses`: for each i8615 unearned
category the component sum omits (rents, royalties, pension/annuity, scholarships not on a Form W-2,
alimony, the taxable part of social security, trust-beneficiary income), assert that a
`ReturnInputs` carrying it cannot reach a computed return — either `other_out_of_scope_income ==
Some(true)` refuses `OtherIncomeOutOfScope` (`return_refuse.rs:995-997`), or `None` refuses
`OtherIncomeUnanswered` at `screen_inputs` (`return_refuse.rs:823-826`).
**Mutation:** delete the `other_out_of_scope_income == Some(true)` refusal ⇒ red. This is the test
that turns "unreachable by luck" into "unreachable, and the next widening reds here".
★ The test's message states the direction it protects: without that refusal, `unearned` is too
**low** for a filer with rental or K-1 income, and a too-low `unearned` **under**-refuses.

**G12 — the help's simplifications only over-refuse (r2, I-1).**
`the_condition_three_help_never_narrows_the_support_test`: assert the condition-3 help contains the
scholarship carve-out and the `30%` allowance, and does **not** contain a sentence excluding any
category the form counts as support. Paired with G7 clauses 6–8, which pin the transcribed spans.
**Mutation:** drop the scholarship carve-out ⇒ red; reword *"more than half"* to *"most"* ⇒ red via
G7's B1 pairing.

**G13 — `CannotKnow` is an ANSWER, and the gate is where that is provable (r2; ★ r3, M-1).**
`cannot_know_is_answered_and_none_is_not`: for `form8615_condition4_parent_alive`, assert that
`Some(CannotKnow)` and `None` produce **different** outcomes at the gate
(`Form8615ParentAliveUnanswered` vs R-4).
**Mutation:** map `CannotKnow` to the unanswered arm at the gate ⇒ red. Without this, the third
answer collapses into silence and this spec's central case becomes a refusal with no remedy.

★★ **r3 (M-1) — the r2 clause *"assert the classifier scores `Some(CannotKnow)` as answered and
`None` as unanswered"* is DELETED, because `classify` has no such capability and a guarantee that
cannot be written is worse than no guarantee.** `Census`
(`crates/btctax-core/src/tax/classifier.rs:64-67`) carries only `declarations: Vec<QuestionId>` and
`exemptions: Vec<(Class, &'static str)>`; `exempt<T>(&mut self, _leaf: &T, …)` (`:75-77`) **discards
the leaf entirely**, and `declaration` is typed `&Option<bool>` (`:71-73`). The classifier makes
answered-ness structural **by the type and by the no-`..` destructure** — it never inspects a value,
so there is no score to assert against. What actually protects the leaf is the compile: `classify`
destructures `HouseholdHeader` with no `..` (`classifier.rs:264-285`), so the new field cannot be
forgotten, and §6.2's `Class::NoTaxDirection` reason string is where *"`CannotKnow` is an answer"* is
recorded for a human. ★ This is the *green-and-blind* shape caught before it shipped: a test asserting
a capability the instrument does not have would have been written as something weaker that passed.

**G14 — the certification is gated, disclosed, and never asks for a conclusion (r2, the owner ruling).**
Three assertions, because the ruling has three parts and each fails differently:
(a) `the_certification_is_unreachable_without_the_dead_end` — over every combination of the three
    leaves, the return computes **only** on `condition 4 == Some(CannotKnow)` ∧
    `form8615_parent_identity_unobtainable == Some(true)`; every other combination refuses. And the
    dead-end fact's `live` is **false** whenever condition 4 is `None`, `Some(Yes)` or `Some(No)`, so
    it is never even offered outside the dead end.
    **Mutations:** `== Some(true)` → `!= Some(false)` ⇒ red. `== Some(CannotKnow)` → `!= Some(Yes)` in
    the liveness predicate ⇒ red on the *not-offered* half.
    ★★★ **r3 (I-1) — and one row that does NOT go through the field, because the defect it catches is
    in the accessor.** Drive the dead-end leaf through the `Field` the way the interview does —
    condition 3 `Some(true)`, condition 4 `Some(CannotKnow)`, then
    `(f.set)(&mut ri, &RowAddr::default(), FieldValue::TriState(Some(true)))` for
    `FieldId::Form8615ParentIdentityUnobtainable` — and assert the return **still refuses**
    `Form8615ParentUnidentifiable`. `TriState(Some(true))` is the filer answering **YES** to *"Can you
    give the IRS your parent's name and address?"*, so the correct outcome is the refusal that sends
    them to the IRS route.
    **Mutation — and it is the whole point of the row:** write the accessors straight through
    (`get_bool: |ri| ri.header.form8615_parent_identity_unobtainable`,
    `set_bool: |ri, v| ri.header.form8615_parent_identity_unobtainable = Some(v)`) ⇒ **red**, because
    a YES then certifies. That mutation is the *obvious* transcription (§4.1), every other registry
    entry is written that way, and no test in the repo today reds on it: G14(a)'s other rows, G2 and
    G8 all set the **field value**, and the registry delegation test round-trips through the pair and
    so passes on a consistently-inverted pair and on a straight-through pair alike
    (`crates/btctax-input-form/src/spec/mod.rs:368-393`). ★ This row is the only place the *keystroke*
    is asserted, so it is the guarantee that must not be "simplified" into another field-value row.
(b) `a_certified_return_carries_the_disclosure_and_all_three_warnings` — a return produced on the
    §6.3 path emits a Form 8275 disclosure whose Part I names `IRC section 1(g)` and whose Part II
    leads with the impossibility argument, plus W-1, W-2 and W-3, with W-3 asserted **verbatim**.
    **Mutation:** delete any of the three warnings, or the disclosure ⇒ red. This is the assertion
    that stops the certification from decaying into a silent exit.
(c) `no_filer_facing_string_asks_for_a_legal_conclusion` — over every prompt, help and disclosure
    string this spec adds, assert none asks the filer to affirm that §1(g) does not apply, that they
    are exempt, or that Form 8615 is not required. The conclusion is btctax's alone (§6.3.2).
    **Mutation:** reword the dead-end prompt to *"do you agree §1(g) does not apply to you?"* ⇒ red.

**G15 — OQ-5's ruling, both halves at once (r3, I-2).**
`the_protection_order_filer_is_refused_and_told_about_tas`: a fixture in the dead end who answers the
§4.1 dead-end question **YES** (they *can* supply a name and address) asserts **both**
(a) the return still refuses `Form8615ParentUnidentifiable` — no computed return, no Form 8275, no
    certification, i.e. the conjunction was not widened into a disjunction; **and**
(b) that refusal's detail names *"Taxpayer Advocate Service"* and carries the contacts
    `TaxpayerAdvocate.IRS.gov/Contact-Us` and `877-777-4778`.
**Mutations:** delete the TAS paragraph from R-4's detail ⇒ red on (b). Add a disjunctive second limb
to §6.3's gate for this filer (`|| forbidden_to_contact`) ⇒ red on (a).
★ The two halves are asserted **in one test on one fixture** deliberately: they are the two halves of
a single owner ruling (`92014cd1`), and splitting them lets a later edit satisfy one while breaking
the other. ★★ Note that (b) is a **prompt-only** assertion in G7's sense — it pins that the string is
present, not that TAS will help — and its message must say so, because a checker that looks like it
validates the advice is the green-and-blind instrument §9 exists to avoid.

---

## 10. Blast radius — measured, not estimated

Measured by copying the tracked tree to an isolated build directory, applying the §6 gate in its
migration state (both answers `None`, which is every existing vault and every existing fixture), and
diffing the failing-test sets:

```
git ls-files -z | tar --null -T - -cf - | tar -xf - -C /scratch/tmp/fr29-blast
# gate at return_1040.rs:989 replaced with: filing_status != Mfj && !provably_24_or_older
cargo nextest run --workspace --no-fail-fast
```

| | tests | passed | failed |
|---|---|---|---|
| baseline (unpatched copy) | 2779 | 2775 | 4 |
| with the new gate | 2779 | 2757 | 22 |

The 4 baseline failures are copy-only (`repo_hygiene` ×2 and `harness_check` ×2 — git hooks and
packaging checks that need the real working tree) and are identical in both runs.

**18 tests newly fail.** In full:

| crate / binary | count | tests |
|---|---|---|
| `btctax-cli::tax_report` | 9 | `a_gift_over_its_ceiling_prints_its_charitable_carryover_out_in_the_report`, `a_gift_within_its_ceiling_prints_no_charitable_carryover_line`, `carryover_write_back_round_trips_and_respects_user_precedence`, `dual_report_renders_absolute_return_with_section_6_labels`, `a_computed_capital_loss_stamp_survives_every_command_that_should_retract_it`, `import_preserves_a_computed_carryover`, `full_return_report_surfaces_conservative_omission_advisories`, `the_full_remedy_chain_restores_a_computed_carryover`, `the_summary_does_not_claim_a_capital_loss_write_the_gate_skipped` |
| `btctax-cli::promote_cli` | 4 | `characterization_full_return_export_pins_the_shipped_file_set_and_report`, `export_full_return_with_an_overflowing_part_ii_narrative_refuses_with_a_named_remedy`, `export_full_return_writes_form_8275_txt_by_name`, `promoted_export_with_more_than_6_legs_refuses_cleanly_not_panics` |
| `btctax-cli::experimental_notice` | 2 | `full_return_export_notice_absent_from_every_file_in_the_export_directory`, `full_return_export_notice_reaches_stderr_not_stdout` |
| `btctax-core` | 2 | `tax::return_1040::tests::kiddie_tax_refuses_dependent_over_threshold`, `tax::return_1040::tests::business_interest_income_refuses` |
| `btctax-oracle-harness::smoke` | 1 | `check_mode_reconciles_every_line_of_the_anchors_and_pinned_cells` |

**Shape of the 15 CLI failures.** Every one is the same thing — the year becomes uncomputable:

```
Usage("tax year 2024 cannot be computed from its full-return inputs: … needs Form 8615 …")
```

They are adult-filer fixtures with no date of birth and capital gains over the threshold, so §7's
migration (give the fixture a DOB) fixes all 15 and exercises §5.1 while doing it.

**`kiddie_tax_refuses_dependent_over_threshold`** fails at exactly the assertion FR-29 flagged
(`return_1040.rs:4473-4475`; the copy reported line 4483, +9 for the patch). That is the measurement
confirming the defect is the one being fixed, and G1 is its replacement.
**`business_interest_income_refuses`** fails at `return_1040.rs:4404` (the copy reported 4413) — the `$5,000` *hobby* interest
half of the test, which is unearned income over the threshold on a fixture with no DOB. Under §6 it
presents as R-1 rather than `KiddieTax`; the fixture needs a DOB or a condition-3 answer.

**The oracle corpus** names its casualties precisely: `single_w2_plus_crypto_ltcg`,
`single_qdcgt_both_slices`, `single_short_term_crypto_gain`,
`single_miner_qbi_limited_by_net_capital_gain` — four of the households the make-check sweep runs,
which `crates/btctax-oracle-harness/tests/smoke.rs:193-196` describes as *"the twelve anchors + the two pinned cells"*. The `admitted >= 11`
floor still held in the patched run.
**P-2 (plan task):** the full corpus of ~104 households runs under `#[ignore]` (`crates/btctax-oracle-harness/tests/smoke.rs:202-204`) and is
**not** covered by this measurement — run it before the phase closes.

**One known under-count.** The measurement patch raises `KiddieTax` where §6 raises R-1/R-2, so tests
that *expect* `KiddieTax` stayed green in the patched run and will need editing anyway. There is
exactly one such test — `kiddie_tax_refuses_dependent_over_threshold`, already counted above (its three
`Some(RefuseReason::KiddieTax)` assertions at `:4455`, `:4463`, `:4485`). The 18 is therefore complete
as a *test* count.

★★ **r2 — what the fold does and does NOT change about this measurement.** The §10 numbers were taken
against a patch implementing **r1's** gate, and r2 changed condition 4's type, added a third leaf, a
third `SkippableKind`, a fourth `RefuseReason` and the §6.3 path. **The measurement is not re-run
here, and the number is not re-stated as if it were.** What can be said precisely:

- **The 18 newly-failing tests do not move.** All 18 fail on the *entry* to the screen — R-1 territory,
  where condition 3 is `None` on a fixture with no DOB — and r2 changed nothing at or before ladder
  step 3. The measurement patch (`filing_status != Mfj && !provably_24_or_older`) is still exactly the
  region those fixtures land in.
- **Everything r2 added lands as a COMPILE error, not a test failure**, and is therefore invisible to a
  failing-test count by construction: the `Option<bool>` → `Option<ParentAliveAnswer>` change reds
  every construction site, the third `SkippableKind` variant reds four exhaustive matches (§4.1), the
  new `RefuseReason` reds `attribute.rs`, and the third leaf reds `classifier.rs`'s no-`..`
  destructure. That is the blast radius this repo prefers, and it is why r2 does not need a second
  sweep to be safe — but it is **not** a claim that the count would be unchanged.
- **P-2 still stands** (the `#[ignore]`d ~104-household corpus), and the plan **re-measures §10 against
  the r2 gate** before the phase closes. A number carried across a design change without being re-run
  is exactly the stale figure this repo keeps finding in its own documents.

---

## 11. Out of scope, and what is filed instead

- **Filling Form 8615.** Needs the parent's taxable income and every sibling's §1(g) amount. R-3 says
  so to the filer.
- **§59(j) kiddie-AMT** (the AMT exemption cap for a child, which Tax-Calculator *does* model at
  `calcfunctions.py:2431`). A separate provision on a separate form; note it as a follow-up under the
  §G-6 AMT track rather than folding it in here.
- **A date of birth in the future, or after the tax year.** `considered_age_at_year_end` returns a
  negative age, `provably_24_or_older` is false, and the filer is asked — fail-closed, so it is a
  Minor. File it; do not gate on it.
- ★ **The Form 8814 parental election** (r2, C-1(e)) — `i8615--2025.txt:100-106`:
  *"The parent may be able to elect to report the child’s interest, ordinary dividends, and capital
  gain distributions on the parent’s return. If the parent makes this election, the child won’t have
  to file a return or Form 8615."* That is a **real exit** for the filer R-3 refuses, and r1 did not
  mention it. It is **out of scope** — it is the *parent's* election on the *parent's* return, which
  btctax never sees, and the same instruction warns the tax *"may be higher if this election is
  made"*. But R-3's detail should not pretend the only route is completing Form 8615 by hand: name
  the election and point at Form 8814, without recommending it. **Documentation only; no new form, no
  new collection, no gate.**

---

## 12. Open questions — ANSWERED in r2

★ All four r1 open questions are **closed here** rather than carried. The r1 review recommended an
answer to each; each recommendation was checked against the primary source before being adopted, and
the reasoning is recorded so a re-review can overturn it with evidence rather than re-derive it.

**OQ-1 (Minor) — ANSWERED: the reason string is enough; revisit at the fifth leaf.**
`Class::NoTaxDirection` is documented as *"a lawful silent default"* (`classifier.rs:50-51`) and these
leaves are not lawful silent defaults — they refuse. But `qbi_w2_wages` and `qbi_ubia` already stretch
it identically (`classifier.rs:498-509`) and carry the real semantics in their reason strings, which
§6.2 follows. A new variant is cheap (an exhaustive match reds), so this is a judgment call, not a
defect. ★ If it is ever added, add it for **all four** leaves in one pass, not two — a half-migrated
taxonomy is worse than either taxonomy.

**OQ-2 (was: Important if answered "collect it") — ANSWERED: DO NOT transcribe Charts A/B/C in this
cycle. Condition 2 stays assumed TRUE, and §8 R-3 discloses the assumption instead (I-4).** Four
reasons, in ascending weight, all verified against the extract:

1. the assumption's direction is **over**-refusal, the side this repo prefers by rule;
2. both named populations are people **not required to file a return at all** — the smallest possible
   blast radius for an over-refusal;
3. ★ **Chart B keys on dependency** — *"If your parent (or someone else) can claim you as a dependent,
   use this chart to see if you must file a return."* (`i1040gi--2025.txt:692`). Reintroducing a read
   of `can_be_claimed_as_dependent_taxpayer` into the Form 8615 path **in the same cycle that deletes
   it** is the single most plausible mechanism by which FR-29 recurs. If it is ever done, the read
   must be scoped to condition 2 alone, with a test that reds if it reaches conditions 1/3/4/5;
4. ★★ **decisive, and it reverses the direction:** Charts A, B and C are a **disjunction** —
   `i1040gi--2025.txt:733`, *"You must file a return if any of the conditions below apply for 2025."*
   Chart C's items include *"net earnings from self-employment of at least $400"*, church wages, §965
   inclusions, Archer/Medicare-Advantage MSA distributions and a transferred clean-vehicle credit,
   several of which btctax cannot answer without new collection. Transcribing A and B while leaving C
   unmodelled would compute condition 2 as **false** for filers Chart C requires to file — i.e. it
   would **under**-refuse. **A partial transcription is not a smaller version of this improvement; it
   is a direction reversal.** If OQ-2 is ever reopened, the C limb must be collected in full or kept
   assumed-true — and the latter means the assumption never actually goes away.

**OQ-3 — ANSWERED: accept the split.** A year-free `prompt` with a `format!`ed year in the refusal
detail matches every entry in both registries today, and the condition wording is year-invariant:
`i1040gi--2024.txt:3559-3576` and `i1040gi--2025.txt:3927-3944` differ only in the figures and the
years. No change warranted, and §4.1 now enumerates the resulting departures (M-5) rather than
understating them.

**OQ-4 — ANSWERED: confirm the single leaf.** The form states **one** numbered condition with three
alternatives and asks the filer for the disjunction; splitting it would make btctax re-derive what the
form writes out, which is the compression `CLAUDE.md` forbids. ★ C-1 strengthens this rather than
weakening it: i8615's January-1 chart resolves the limbs **together** per birth date, with one
footnote per row (`i8615--2025.txt:71-98`) — the IRS itself treats the three as one
question about a person, not three independent facts. One leaf, as specced.

---

## 13. OQ-5 — raised by r2, RULED by the owner

**OQ-5 (deliberately NOT widened — ★★★ r3: the owner has RULED, `92014cd1`, and the ruling is
folded).** Owner, verbatim: *"just refer user for tas and tell them good luck."* ⇒ **do not widen the
certification, and give these filers a refusal that names the Taxpayer Advocate Service and is honest
that btctax cannot carry them further** (`design/OWNER_DECISIONS_2026-09-04.md`, *"OWNER RULINGS,
2026-09-05"*). Both halves are discharged below: the non-widening in §6.3's gate, which is unchanged,
and the referral in §8 R-4's detail, pinned by §9 G15. **This is no longer an open item, and it is not
a review's to reopen.** The owner ruling's
evidence names a **second** dead end in passing: i8615's IRS-request remedy requires *"A statement
that you are making the request to comply with section 1(g) of the Internal Revenue Code and that you
have tried to get the information from the parent"* (`i8615--2025.txt:185-187`) — and a
protection order forbids exactly that contact. Such a filer **can** name and locate their parent, so
they answer §4.1's dead-end question YES and R-4 refuses them, correctly under this spec and wrongly
under the ruling's own principle.

**The certification is not widened, and that is now the owner's decision as well as this spec's.** The
self-certification is scoped to the filer who cannot identify their parents; §6.3's gate is a
**conjunction** of two facts, and admitting the protection-order case means adding a **disjunctive**
second path. *Widening an exemption is never the safe edit* — a disjunction is precisely how an
exemption widens, and the fact that would gate it (*"I am forbidden to contact my parent"*) is one a
filer might reach for far more loosely than *"I do not know who my parents are"*. The owner's ruling
says the same thing and says why: *"This is the conservative ruling and it protects the fix: a
disjunction is how an exemption widens, which is the exact mechanism FR-29 was."*

★★ **What the ruling DID change, and it is the half r2 left undone.** These filers are refused, but
they are not left with nothing: R-4's detail now names the Taxpayer Advocate Service, cites the limb
of `i1040gi--2025.txt:153-154` that covers them, gives the two contacts the same page prints
(`:162`, `:166`), and asserts nothing beyond it. **A referral is not a path** — no predicate moves, no
leaf is added, the return still refuses — and §9 G15 asserts both halves at once, so a later edit
cannot drop the referral or convert it into a second limb without going red.
