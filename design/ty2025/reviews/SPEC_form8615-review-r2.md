# SPEC_form8615_kiddie_tax.md — independent re-review r2 (the fold)

**Artifact:** `design/ty2025/SPEC_form8615_kiddie_tax.md` (1685 lines) at the r2 fold `bb88e9ce`,
unchanged since.
**Reviewed against:** `HEAD = 578646ea` on `main`.
**Fold under review:** `bb88e9ce` (1124 insertions / 153 deletions), answering
`design/ty2025/reviews/SPEC_form8615-review-r1.md` (1C/4I/5M/3Nit) and
`design/ty2025/DECISION_form8615_no_path_self_certification.md`.
**Also binding:** `design/OWNER_DECISIONS_2026-09-04.md`, "OWNER RULINGS, 2026-09-05" (`92014cd1`) — OQ-5
settled, do NOT widen.
**Reviewer:** independent (author ≠ reviewer). Nothing was modified but this file; nothing was committed.

---

## VERDICT

**1 Critical / 2 Important / 4 Minor / 1 Nit — DO NOT BUILD YET.**

**The fold is good work and most of it holds.** Every one of the r1 findings was folded, two were folded
*with a correction to the review* and both corrections are right — I re-derived the §63(f) band from
`born_early_enough` (`return_1040.rs:89-91`) and `dob <= Date(year − 64, Jan 1)` does make a January-1
birth in `year − 64` aged 65, so `[year − 63, year − 24]` is exact and r1's `[year − 64, …]` was off by
one; and `sum_unemployment(ri)` really is in the component sum at `return_1040.rs:994`, so r1's "omits
every bolded category" really was wrong. The owner ruling's two structural constraints are **discharged**
(§I-2 below is about a *later* ruling, not about these). The independence trap holds across all of the new
text and is **stronger** than in r1: the sentence that r1 could only recommend is now transcribed into the
filer-facing help and pinned by a checker clause I machine-verified passes.

**What blocks the build is that §1 — "Sourcing of record — READ THIS FIRST" — is false at HEAD again, for
the second round running, and by the same mechanism.** `c7819f8c`, the commit *after* this fold,
regenerated `i8615--2025.txt` with plain `pdftotext` (712 → 1307 lines) and gave `f8615--2025.txt` a
four-line provenance header (59 → 63). **24 of the 27 i8615/f8615 citation sites in this spec now resolve
to unrelated text**, every "left column / right column" annotation is now a false statement about the
artifact, and **plan task P-2a — a column-aware extract with its own B1 pairing — is building an
instrument for a defect that no longer exists.** I record the ordering deliberately, as r1 did: *the spec
was accurate when folded.* The artifact under review is the one at HEAD.

Neither the Critical nor either Important is an understatement of tax **as the spec is written**. I-1 is
an understatement path in the *unspecified glue* between the prompt and the field, which is why it is
Important rather than Minor.

---

## FINDINGS

### C-1 (Critical) — §1's sourcing of record is false at HEAD, the two-column premise it rests on is gone, and P-2a is now a task that must NOT be built

`c7819f8c` ("fix(extracts): I broke the repo's own extraction convention on both instruction booklets")
landed one commit after `bb88e9ce`:

```
$ git log --oneline bb88e9ce..c7819f8c -- design/forms/extract/i8615--2025.txt
c7819f8c fix(extracts): I broke the repo's own extraction convention on both instruction booklets
$ wc -l design/forms/extract/i8615--2025.txt design/forms/extract/f8615--2025.txt
  1307 design/forms/extract/i8615--2025.txt      # spec §1 says 712
    63 design/forms/extract/f8615--2025.txt      # spec §1 says 59
$ head -2 design/forms/extract/i8615--2025.txt | tail -1
# sha256:902ca736f1205e72…  |  pdftotext (no flags)
```

`i8275--2024.txt` and `f8275--2024.txt` were **not** touched, and every 8275 citation in this spec is
still exact (I checked `:202-214`, `:352-374`, `:378-388`, `f8275:6`/`:9`). The damage is confined to
i8615 and f8615 — and that is where §1, §1.1, §1.2, §2, §4, §6.3.1 and §9 G7 do their work.

#### (a) Citation drift — 15 of 18 distinct ranges, 24 of 27 sites

Machine-resolved: every `i8615--2025.txt:` / `f8615--2025.txt:` citation in the spec, against the file at
HEAD. "actual now" is the text that is there today; "true location" is where the content the spec cites
actually lives.

| spec cites | spec sites | what the spec claims it is | actual text there now | true location |
|---|---|---|---|---|
| `f8615:13-16` | 58, 434, **975** | form face lines **A/B/C** | "Internal Revenue Service … Go to www.irs.gov/Form8615" | **`:17-20`** |
| `i8615:4-6` | 254 | *support*, 2nd span ("others. However…") | the generated `#` header + `2025` | **`:68-70`** |
| `i8615:12-24` | 228, 733 | the January-1 chart | "Future Developments" | **`:71-98`** |
| `i8615:12-29` | 1661 | the chart + its footnotes | same | **`:71-98`** (`***` at `:98`) |
| `i8615:27-36` | 276 | *unearned income* | ✔ contains it | `:28-35` — **OK** |
| `i8615:31-32` | 35 | scholarships not on a W-2 = unearned | ✔ starts there | clause completes at `:33` — **near-miss** |
| `i8615:33-41` | 1603 | the Form 8814 parental election | the unearned tail + a Tip + "Who Must File" | **`:100-106`** |
| `i8615:44-63` | 185 | the five conditions | ✔ superset (also swallows the holding + Support) | `:44-57` — **loose, see N-1** |
| `i8615:65-68` | **293** | the FR-29 **holding** | the Support tail + a page footer | **`:58-61`** |
| `i8615:66-67` | 591, **1267**, **1382** | *"…whether or not the child is a dependent."* | `Oct 22, 2025` + a blank line | **`:59-60`** |
| `i8615:66-68` | 35 | the holding | footer + "others. However, a scholarship…" | **`:58-61`** |
| `i8615:67-68` | 61, **466**, **978** | *"…don't apply if neither of the child's parents…"* | "others. However, a scholarship…" | **`:60-61`** |
| `i8615:69-72` | 37, 248 | *support*, 1st span | the scholarship carve-out + the chart heading | **`:62-65`** |
| `i8615:120-123` | **977** | "The request must contain all of the following." | "Parents are married. If the child's parents file separate returns…" | **`:184`** (section at `:158`, `:176-179`) |
| `i8615:131-133` | 60, 1671 | "…that you have tried to get the information from the parent." | "Parents are divorced…" | **`:185-187`** |
| `i8615:139-140` | **263**, **1383** | *earned income* definition | the page footer "Instructions for Form 8615 (2025) Catalog Number 28914R" | **`:266-267`** |
| `i8615:139-141` | **436**, **1384** | the IRS-request required contents | same page footer | **`:193-195`** |
| `i8615:139-` | 1393 | "different columns of the same physical lines" | footer | n/a — see (b) |
| `i8615:150-155` | 597 | capital-not-a-material-factor | "married to each other, but lived together all year…" | **`:276-281`** (30% at `:268-275`) |

The bolded sites are the load-bearing ones: **`:436` and `:975` are inside the `HouseholdHeader` doc
comment and the §6.3.1 dead-end table**, i.e. text the build transcribes into
`crates/btctax-core/src/tax/return_inputs.rs`; **`:1382`, `:1383`, `:1384` are G7's own source table**,
the conformance instrument's addresses; **`:1267`** is the `RefuseReason::KiddieTax` doc comment. This
repo's own rule is that a form field carries the instruction text *with its address*; shipping the
address wrong is shipping the rule half-followed.

★ The quotations themselves are **fine**. I normalised each with `cite_check::normalise` and checked
containment against the current extracts: every form quotation in this document still matches. This is a
pure address failure, not a transcription failure — which is exactly why it is invisible without running
it.

#### (b) The two-column premise is gone, and with it §1's ★★ warning

§1 (`:179-181`): *"`i8615--2025.txt` is a TWO-COLUMN extract, so a line number alone can be ambiguous —
`:139-141` is the IRS-request bullet in the **left** column and the *Earned income* definition in the
**right**. Every i8615 citation below names its column wherever the two collide."*

Under plain `pdftotext` there are no interleaved columns. The eleven "left column" / "right column"
annotations scattered through §1.1, §1.2, §2, §4, §5.2, §6.3.1, §8 R-3, §9 G7 and §12 are now false
descriptions of the artifact, and the specific example is doubly wrong: `:139-141` is a page footer, and
the two contents it names are **270 lines apart** (`:193-195` and `:266-267`).

#### (c) ★★★ P-2a would be built for a defect that no longer exists — and its measurements are stale

§1 P-2a specifies *"a column-aware extract for two-column authorities — a new derived file beside the
existing extract, or a `--columns` mode on `cargo run -p xtask -- forms extract`"* plus its own B1
pairing, and calls it *"the reason P-1 cannot land alone"*. Its numbers: *"110 checkable fragments, of
which 40 fail against the extracts as archived; splitting `i8615--2025.txt` into its two columns before
normalising drops that to 20, and every one of the remaining 20 is a self-citation."*

I re-ran that measurement at HEAD, replicating `cite_check`'s `quoted_spans` → `inline_quotations` →
`fragments` → `normalise` pipeline over this document against the five extracts, **with no column
splitting**:

```
blockquote spans: 13   total spans (bq+inline): 135
checkable fragments: 103   FAILING: 20
```

and all 20 failures are self-citations — our own prose, this spec's own prompts and help strings, a
`questions.rs` prompt, the r1 draft, the owner ruling, the statute. **That is precisely the end state
P-2a exists to reach.** `c7819f8c` already delivered it. Building P-2a now adds a derived artifact and a
B1 test for a condition that cannot occur, and the spec tells the implementer it is a hard prerequisite,
so they will build it first.

#### (d) §9's "P-2a is a hard prerequisite of G7" is false, and I measured G7 green without it

§9 (`:1393-1395`): *"against the raw two-column extract, clauses 6 and 7 cannot pass, because each spans
two lines of one column and the haystack interleaves the other."*

All eight G7 clauses, normalised with `cite_check::normalise`, against the **raw** current extracts and
against the §4.1 strings they belong to:

```
=== G7 clause vs EXTRACT (current) ===        === G7 clause vs PROMPT/HELP ===
  clause 1..8: PASS  (8/8)                      clause 1..8: PASS  (8/8)
=== G7 structural counts on the condition-3 prompt ===
  clause 3 occurrences (want 2): 2
  'a under age 18' (want 1): 1
  'b age 18 and didn t have' (want 1): 1
  'c a full time student' (want 1): 1
```

G7 is buildable today, exactly as specified, with no column work — the clause table, the per-clause
sources (modulo the addresses in (a)), and all three structural counts. Only the *line numbers* in the
"sourced from" column need re-resolving.

#### (e) The authority table's own metadata

`:103-104` gives the extracts as 712 and 59 lines and attributes both to `29e47a0b`. They are 1307 and
63, and the extract of record is `c7819f8c`.

**Fix (a re-sourcing pass, not a redesign — as in r1):** re-resolve the 18 ranges above; delete every
"left/right column" annotation and §1's ★★ warning; **delete P-2a** and replace it with one sentence
recording that `c7819f8c` closed it and the measured 103/20/all-self-citations result; strike §9's
P-2a-prerequisite paragraph; update `:103-104`. Nothing about the gate, the ladder, the type or the
certification changes.

---

### I-1 (Important) — the dead-end leaf's polarity is inverted between the prompt and the field, the spec never says which side inverts, and no guarantee in §9 reds if it is written straight through

Three statements the spec makes, all correct on their own:

- `:635-638` the prompt: *"Can you give the IRS your parent's name and address? … Answer YES if you can
  supply both for either parent. Answer NO only if you can supply neither."*
- `:641-643` ★ *"The prompt is phrased in the **positive** — *can you* — and the certification unlocks
  on `NO`."*
- `:443` / `:869` the field: `form8615_parent_identity_unobtainable`, and **`Some(true)` is the ONLY
  value that opens anything.**

So the filer's **NO** must become `Some(true)`. But `:508` assigns the leaf `SkippableKind::YesNo`, and
the macro that builds its `Field` hands the filer's boolean straight through:

```rust
// crates/btctax-input-form/src/spec/registries.rs:83-91
let FieldValue::TriState(Some(b)) = v else { return Err(SetError::WrongKind) };
(SKIPPABLE_QUESTIONS[$idx].set_bool)(ri, b);
```

`get_bool`/`set_bool` are `fn` pointers, so the inversion *is* expressible — but the spec never writes it,
and **every other entry in `SKIPPABLE_QUESTIONS` is polarity-aligned** (`"Are YOU legally blind?"` →
`blind`). This is the first inverted one in the registry, and it is introduced in a section that never
mentions the accessors.

**The failure mode, if an implementer writes the obvious accessor:** a filer who answers **YES** — *I can
give the IRS my parent's name and address*, the filer for whom the IRS route is open and R-4 is meant to
be final — is stored as `unobtainable = Some(true)`, reaches §6 ladder step 6's first arm, and gets a
computed return with no Form 8615. That is owner-ruling constraint 1 broken (certification reachable
outside the dead end) in the understatement direction.

**And nothing reds.** I checked each candidate:

- **G14(a)** enumerates *"every combination of the three leaves"* — field values. Green.
- **G2** and **G8** likewise read the leaves, not the keystroke. Green.
- The registry delegation test (`crates/btctax-input-form/src/spec/mod.rs:368-392`) does
  `(entry.set_bool)(&mut ri, true)` then asserts `(f.get)(…) == TriState(Some(true))`. A *consistently*
  inverted pair round-trips green; a *straight-through* pair round-trips green. It detects only a
  get/set **mismatch**, which is not this defect.

This is the shape the repo's own `widening-an-exemption-is-never-the-safe-edit` note records — three
attempts at one filer prompt, two of them understatement paths, closed only by making the YES-conditions
structural. The spec did that at the *gate*; it left the *prompt-to-gate* edge unspecified.

**Fix (cheap):** write the two accessors out in §4.1 —
`get_bool: |ri| ri.header.form8615_parent_identity_unobtainable.map(|v| !v)`,
`set_bool: |ri, can_supply| ri.header.form8615_parent_identity_unobtainable = Some(!can_supply)` —
with one sentence saying the inversion is deliberate and why; and add one G14(a) row driven through
`(f.set)(…, FieldValue::TriState(Some(true)))` asserting the return still **refuses**. That row is the
one that reds. ★ Alternatively name the field for the question (`form8615_can_supply_parent_identity`)
and flip the gate to `== Some(false)` — but then §6/§6.1/§6.2/§9's `== Some(true)` mutation all move,
which is more edit for the same safety. The accessor is the smaller change.

---

### I-2 (Important) — the owner's OQ-5 ruling is not folded: §13 still records it as open, and R-4 — the refusal the ruled filer lands in — never names the Taxpayer Advocate Service

`design/OWNER_DECISIONS_2026-09-04.md`, "OWNER RULINGS, 2026-09-05" (`92014cd1`), verbatim: *"just refer
user for tas and tell them good luck"* ⇒ *"Those filers get a refusal that names the Taxpayer Advocate
Service and is honest that btctax cannot carry them further."*

★ As with C-1, the ordering is in the fold's favour: the ruling landed **after** `bb88e9ce`. Also as with
C-1, the artifact at HEAD is what an implementer builds from.

**The half that is discharged, and it is the half that mattered most.** The spec does **not** widen. §6.3's
gate is a conjunction of two affirmative facts (`:869`, `:894`); §4.2's liveness makes the second question
reachable only on `Some(CannotKnow)` and says so as the ruling's constraint 1 (`:684-691`); §13
(`:1668-1685`) names the protection-order case, refuses it explicitly, and gives the reason — *"a
disjunction is precisely how an exemption widens"*. G14(a) pins both halves. **No second limb has been
added, quietly or otherwise.** I looked for one specifically and there is none.

**The half that is missing.** TAS appears in exactly three places — §1's authority table (`:105`), §6.3.5
(`:1090-1109`), and §8.1 (`:1283`) — and §8.1 scopes it: *"W-1, W-2 and W-3 (§6.3.4) and the TAS pointer
(§6.3.5) are surfaced at `report` and at `export`"* for **a return produced on the §6.3 path**. R-4's
detail (`:1218-1230`), which is where §13 says the protection-order filer lands, tells them to use the
IRS-request route and says btctax cannot file Form 8615 either way. It never mentions TAS. So the filer
the owner ruled on gets the refusal without the referral the ruling ordered.

Compounding: §13's heading still reads *"OQ-5 (deliberately NOT widened, and it needs an owner ruling,
not a review)"*. An implementer reads that as an open item and ships R-4 as written.

**Fix:** (i) restate §13's status — ruled `92014cd1`, do not widen, and record the ruling's own citation
instruction; (ii) add the TAS paragraph to R-4's detail for the *"if you CAN supply a name and address"*
branch, citing the third limb of `i1040gi--2025.txt:153-154` and the contacts at `:162`/`:166`, and
nothing beyond; (iii) extend G14 (or add a G15) asserting R-4's detail names TAS, so the referral cannot
be dropped.

**★ The TAS sourcing itself is CORRECT and I re-verified it line by line** — see "what I verified". The
spec's §6.3.5 finding is exactly right and is one of the better things in the fold.

---

### M-1 (Minor) — G13's first half describes a capability `classify` does not have

`:1484-1489`: *"assert the classifier scores `Some(CannotKnow)` as answered and `None` as unanswered."*
`Census` (`crates/btctax-core/src/tax/classifier.rs:62-66`) carries only
`declarations: Vec<QuestionId>` and `exemptions: Vec<(Class, &'static str)>`; `exempt<T>(&mut self,
_leaf: &T, …)` (`:75-77`) **discards the leaf**, and `declaration` is typed `&Option<bool>` (`:71-73`).
The classifier makes answered-ness structural by the type plus the no-`..` destructure — it does not
score values, so there is nothing to assert against.

G13's **second** half — *"the two produce different outcomes at the gate
(`Form8615ParentAliveUnanswered` vs R-4)"* — is writable and is where the protection actually lives, as
does §6.2's compile-forced classification. One sentence: drop the scoring clause, keep the gate clause,
and note that the compile is what stops the leaf being forgotten.

### M-2 (Minor) — the CLI's input grammar for the third answer is unspecified, and the obvious parser rejects the phrasing the prompt gives the filer

`SkippableKind::Choice(&["Yes", "No", "CannotKnow"])` (`:518`, `:531`) reaches `answer.rs`'s
`match sk.kind` (verified at `crates/btctax-cli/src/cmd/answer.rs:156`, `:178` — exhaustive, no `_`, so
the new variant does red it). But that loop uses `parse_yes_no` / `parse_date`; it never calls
`parse_enum`. The spec points at `parse_enum` (`crates/btctax-input-form/src/parse.rs:87-92`), which is
the *input-form* path and is an exact string match:

```rust
if options.contains(&raw) { Ok(FieldValue::Choice(raw.to_string())) } else { Err(ParseError::NotAChoice) }
```

The condition-4 prompt tells the filer *"Answer YES, NO, or CANNOT KNOW"* (`:605`). `"CANNOT KNOW"` is
not `"CannotKnow"`. Direction is fail-closed — an unmatched string re-asks — so this is Minor, and the
implementer meets it at the compile error. Say in §4.1 what `answer.rs` accepts (case-insensitive
`y`/`n`/`?` or `cannot know`, mapping to the three variants) so it is a decision rather than an
improvisation at the keyboard.

### M-3 (Minor) — §1.2's reason for quoting *support* as two spans is no longer true

`:247-248`: *"the definition runs off the bottom of the **left** column and resumes at the top of the
**right**, so it is quoted as the two spans it physically is."* It is still two spans — but because the
page footer `Oct 22, 2025` and a blank line sit at `:66-67`, between `:62-65` and `:68-70`. The
quotation structure survives; only its stated reason is false. Same for §1's ★ note at `:176-178`.

### M-4 (Minor) — the fold's `cite-check` number does not describe this document

The fold reports *"cite-check OK with 52 quotations verbatim"*. At HEAD:

```
cite-check: design/ty2025/SPEC_schedule_1a.md — 7/7 quotations verbatim in the extract
cite-check: design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md — 45/45 quotations verbatim in the extract
cite-check: OK — 52 quotations, all verbatim.
```

`schedule_1a_docs()` (`crates/xtask/src/cite_check.rs:404-417`) names two documents and two fixtures;
neither this spec nor any 8615 extract is among them, `grep 8615 crates/xtask/src/cite_check.rs` is
empty, and **no line-numbered-citation checker exists in `xtask` at all** (the only subcommand match is
`main.rs:59`). So the 52 is entirely Schedule 1-A and is unchanged by this fold; the "126 line-numbered
citations, 0 unresolved" figure came from an ad-hoc script that is not in the repo. Worth saying plainly,
because a reader takes it as coverage of this document — and C-1 is what that coverage would have caught.
This is P-1's whole point; P-1 just has not landed.

### N-1 (Nit) — `i8615:44-63` for "the five conditions"

The conditions are `:44-57`. `:58-61` is the holding, which the same table row already cites separately,
and `:62-63` is the start of *Support*. Tighten to `:44-57`.

---

## THE INDEPENDENCE TRAP — re-checked across the new text

The trap: *"lawfully independent"* / *"not a dependent"* / *"self-supporting"* is **not** Form 8615's
condition-3 test, which asks whether **earned** income exceeded half the filer's support. r1 checked 14
sites in the r1 text and found it held. The fold added ~1124 lines. I checked every occurrence of
`dependen` in the document (37 lines, enumerated by grep) and read the whole of every section the fold
created.

| # | new-in-r2 site | what it says | holds? |
|---|---|---|---|
| 1 | §1.2 `:239-245` | Chart B's scope line quoted *because* it keys on dependency, to explain why its definitions were the wrong ones | **YES** — dependency named only to *disqualify* a source |
| 2 | §2 `:293-297` | the i8615 holding quoted verbatim | **YES — and this is the fold's best change.** r1 derived the holding from dependency's absence from a list; r2 quotes the IRS asserting it |
| 3 | §4 doc comments `:398-448` | conditions 3, 4 and the dead-end fact | **YES** — no dependency language in any of the three |
| 4 | §4.0 `:452-474` | `ParentAliveAnswer` | **YES** |
| 5 | §4.1 help `:572-587` | *"These rules apply whether or not the child is a dependent." Being nobody's dependent, or supporting yourself, does not put you outside them.* | **YES, and this is now the strongest site in the document.** It is filer-facing, it names the trap and denies it in the filer's own words, and r1's *"Income from investments, crypto or a trust is not earned income."* survives verbatim beside it |
| 6 | §4.1 condition-4 / dead-end prompts and help `:602-657` | three answers; name and address | **YES** — no dependency language |
| 7 | §4.2 liveness `:663-675` | `filing_status`, `provably_24_or_older`, the two prior answers | **YES** — the dependency flag is not readable from any predicate |
| 8 | §5.2 `:791-802` | the `OtherOutOfScopeIncome` dependency | **YES** — "dependency" here is a module dependency, not the filer's |
| 9 | §6 ladder `:849-877` | five conditions, three-valued | **YES** — `can_be_claimed_as_dependent_taxpayer` appears nowhere |
| 10 | §6.1 table `:885-909` | nine rows | **YES** — no dependency column, as in r1 |
| 11 | §6.2 `:935-955` | the third leaf's classification | **YES** |
| 12 | **§6.3, all of it `:962-1109`** | the dead end, btctax's position, the Form 8275 content, the three warnings, TAS | **YES — zero occurrences of "dependent" in the entire section.** The brief flagged the Form 8275 wording as the likeliest leak; Part I column (c) (`:1018`) and Part II's three paragraphs (`:1028-1041`) are written entirely about *parent identification* and §1(g)(3)'s missing input. Nothing says or implies "not a dependent" |
| 13 | §8 R-4 `:1207-1236` | the dead-end refusal | **YES** — no dependency language |
| 14 | §8 R-3 `:1242-1270` | reworded `KiddieTax` + the doc-comment replacement | **YES** — the falsehood r1 found (condition **2**) is fixed by disclosure, and `:1266-1270` replaces "dependency appears nowhere in the five conditions" with the IRS's own sentence |
| 15 | §9 G1 `:1295-1305` | `…nobodys_dependent`, `Some(false)` ⇒ `KiddieTax` | **YES** — still the exact inversion of `return_1040.rs:4473-4475` |
| 16 | **§9 G7 clause 6** `:1382` | *"these rules apply whether or not the child is a dependent"* pinned against the extract **and** against the help string | **YES — the trap now has a checker under it.** I ran it: PASS against `i8615--2025.txt` and PASS against the §4.1 help, both under `cite_check::normalise` |
| 17 | §12 OQ-2 `:1636-1640` | Chart B keys on dependency ⇒ reintroducing the read is *"the single most plausible mechanism by which FR-29 recurs"*, and if ever done it must be scoped to condition 2 with a test that reds if it reaches 1/3/4/5 | **YES** — the trap is named as a standing hazard with its guard specified |
| 18 | §13 OQ-5 `:1668-1685` | the protection-order case | **YES** — turns on *identification*, never on dependency |

**Verdict on the trap: it holds at every site, old and new, and r2 improved it in two ways r1 could only
recommend** — the holding is transcribed into filer-facing help rather than derived, and a machine check
(G7 clause 6) now sits under the sentence. The one r1 weak spot (site 5, definitions borrowed from Chart
B) is closed: the definitions come from i8615, and §1.2 quotes Chart B's own scope line to say why it was
the wrong source.

★ The one thing to watch, recorded rather than filed: **OQ-2 is the trap's remaining fuse.** If Charts
A/B/C are ever transcribed, Chart B's dependency key comes back into the Form 8615 path. §12 says so and
specifies the guard. Keep that paragraph whatever else is edited.

---

## WHAT I VERIFIED AND HOW

**The extracts, first, because everything else depends on them.** `git show --stat c7819f8c` (4 files:
`i8615` +1819/−~1200, `i8275r` rewritten, `f8615`/`f8275r` +4 header lines each); `wc -l` on all five
extracts this spec cites; `head -2` on each for the `pdftotext` flag recorded in the new provenance
header. Confirmed `i8275--2024.txt` and `f8275--2024.txt` were **not** in that commit and re-checked
their three cited ranges by eye (`:202-214` adequate disclosure / reasonable basis — verbatim as quoted;
`:352-374` column (a)-(f); `:378-388` Part II's content requirement; `f8275:6`/`:9` the 8275-R sentence
with the OMB number between them, exactly as §6.3.3 describes).

**Citation resolution, machine, not by eye.** A script over the spec extracted every
`(i8615|f8615|i8275|f8275)--YYYY.txt:a[-b]` citation with its spec line numbers, then printed the current
text at each range. 22 distinct ranges, 27 sites for the two 8615 files. The C-1 table is that output,
plus a `grep -n` for each cited *content* to find its true location. I hand-counted nothing.

**Quotation integrity, separately from addresses.** A replica of `cite_check`'s `normalise` (Unicode
folding, markdown/quote/bullet stripping, icon labels, de-hyphenation across breaks, non-alphanumeric →
space, whitespace collapse, lowercase) over every blockquote span and every `*"…"*` span of ≥6 words,
against the union of the five extracts, with `FOREIGN_SOURCES` applied to the inline path only (matching
`cite_check.rs:182-215` / `:298-331` / `:339-354`). Result: **103 checkable fragments, 20 failing, all 20
self-citations** — our own prose, this spec's own prompt/help strings, a `questions.rs` prompt, the r1
draft, the owner ruling, the statute. **Zero form quotations fail.** That is the C-1(c) measurement and
it is also the reason C-1 is an address failure rather than a transcription failure.

**`cite-check` itself, run.** `cargo run -q -p xtask -- cite-check` → `OK — 52 quotations, all verbatim`
(7 from `SPEC_schedule_1a.md`, 45 from `IMPLEMENTATION_PLAN_schedule_1a.md`), plus
`authority archived + extracted for 1/16 emitted forms`. Read `schedule_1a_docs()` at
`cite_check.rs:404-417` to confirm this document is not registered, and `grep 8615 crates/xtask/src/cite_check.rs`
→ empty. `grep` for a line-numbered-citation checker in `crates/xtask/src/` → nothing.

**G7, both halves, machine-checked.** All eight clauses through `normalise`, against (a) the extract each
names and (b) the §4.1 string it belongs to: **8/8 and 8/8**. The three structural counts on the
condition-3 prompt: clause 3 twice, and `a under age 18` / `b age 18 and didn t have` /
`c a full time student` once each — the exact values §9 claims were measured. So G7's design is sound and
its clause table is right; only its addresses moved.

**The owner ruling, both constraints.** Constraint 1 (gated on the dead end): traced §4.2's liveness
predicate (`:670-674`, `== Some(ParentAliveAnswer::CannotKnow)`, and the ★ note at `:689-691` explaining
why `!= Some(Yes)` would be one keystroke from an opt-out), §6 ladder steps 4-7, §6.1's nine rows, and
G14(a)'s two mutations. There is no path to the certification without two affirmative answers.
Constraint 2 (facts, not conclusions): read every prompt, help and disclosure string the spec adds — the
condition-4 prompt is a proposition the filer says yes/no/cannot-know to; the dead-end prompt asks what
the filer *can do*; §6.3.2 `:994-1000` forbids the conclusion explicitly and G14(c) plants
*"do you agree §1(g) does not apply to you?"* as its kill. Both hold.

**The tri-state's consumers, each resolved against source.** `SkippableKind` has two variants today
(`questions.rs:900-903`); the exhaustive `match`es on it are **exactly the four** the spec names —
`crates/btctax-cli/src/cmd/answer.rs:156` and `:178`, `crates/btctax-input-form/src/spec/mod.rs:368` and
`:394` (verified by `grep -rn "SkippableKind::" crates/ --include=*.rs`, no other match sites, no `_`
arms). `FieldKind::Enum(&'static [&'static str])` and `FieldValue::Choice(String)` exist at
`crates/btctax-input-form/src/seam.rs:181`/`:196`; filing status uses them at
`crates/btctax-input-form/src/spec/sections.rs:165-167`; `parse_enum` at `parse.rs:87-92` is an exact
`options.contains(&raw)` (this is M-2). `classifier.rs`'s `exempt` is already generic over the leaf type
(`:75`), so §6.2's "must accept a non-bool `Option`" is already satisfied — and its discarding of the
leaf is M-1.

**The three warnings.** W-1's two blockquotes are byte-verbatim against `i8275--2024.txt:202-214`
(re-read the file). W-2 matches the ruling's wording and leads with impossibility, not a constitutional
theory, and §6.3.3 `:1043-1045` forbids adding one. **W-3 is verbatim**: I flattened
`DECISION_form8615_no_path_self_certification.md`, squeezed whitespace and matched the owner's sentence
character for character against `:1083-1084`. All three are in plain filer-facing language and §8.1 makes
them non-optional with G14(b) as the kill.

**TAS.** Read `i1040gi--2025.txt:143-170` directly. The three bullets are at `:157-159` (financial
difficulty / immediate threat of adverse action / IRS not responding) — **the spec's own line numbers**,
and none covers a filer who has not contacted the IRS. The qualifying limb is the third in the
introductory sentence at `:153-154`, *"…or you believe an IRS system, process, or procedure just isn't
working as it should"*, quoted verbatim and correctly in §6.3.5. Contacts: `TaxpayerAdvocate.IRS.gov/Contact-Us`
at `:162`, `877-777-4778` at `:166`, both inside the `:161-166` the spec cites. ★ The brief's *"three
bullets at `:155-166`"* is a looser range than the file supports; **the spec's `:157-159` is the right
one** and nothing here needs changing. This is the fold's cleanest piece of sourcing.

**The two fold-time corrections to the r1 review, both re-derived.** `born_early_enough`
(`return_1040.rs:89-91`) is `dob <= Date::from_calendar_date(year − 64, January, 1)`, so a January-1
birth in `year − 64` **is** 65+ — r2's `[year − 63, year − 24]` is exact and r1's lower bound was off by
one. And `return_1040.rs:993-994` really does add `state_refund_taxable` and `sum_unemployment(ri)`, so
r1's *"omits every bolded category"* really was wrong for unemployment compensation.

**§9 G9's bookkeeping, every row.** `SKIPPABLE_QUESTIONS.len()` asserted `16` at `questions.rs:1493`
inside `fn skippable_registry_is_separate_and_has_five_entries_with_correct_liveness` (`:1489` — the
stale name N-2 flagged, confirmed); the two `93`s at `coverage.rs:429` and `:434`; the
`skippable_tristate!` macro reading `SKIPPABLE_QUESTIONS[$idx]` for label/help/live/get/set
(`registries.rs:67-95`) — I-2's correction of the r1 reassurance is right, and the indices really are
literals the compiler cannot check.

**Other code citations spot-checked and correct:** `form8275.rs:53-67` (`Part1Item` has
`form`/`line`/`description`/`amount`, no column (a)/(b)), `:108` (`disclosure_8275` signature),
`:151-152` (`if part_i.is_empty() { return None }`) — so P-3's description of what must change is
accurate. `return_refuse.rs:995-997` (`other_out_of_scope_income == Some(true)` refuse),
`questions.rs:289` (the dependency prompt), `questions.rs:952` (`SKIPPABLE_QUESTIONS`),
`classifier.rs:264-285` (the no-`..` `HouseholdHeader` destructure), `resolve.rs:188-192`.

**Not re-derived, per the brief:** `make check` (2808 passed / 12 skipped / 0 failed, fmt clean) and the
18-test blast radius.

---

## WHAT I COULD NOT CHECK

1. **I did not run the suite, and I did not re-measure the blast radius.** The brief settled both. Note
   that §10 itself says the 18 was measured against an **r1-shaped** patch and that everything r2 added
   lands as a compile error rather than a test failure — I read that reasoning and it is sound, but it is
   reasoning, and the spec's own P-2 (re-measure before the phase closes) is the right instruction.
2. **Whether the r2 gate's 18 is still 18** after I-1's accessor and I-2's R-4 wording are folded. Both
   are additive; neither should move it. Not verified.
3. **Whether P-1 would report exactly 20 once `FOREIGN_SOURCES` is extended.** My replica is faithful to
   `cite_check.rs`'s pipeline as I read it, but it is a replica: the real number arrives when P-1 lands
   and registers this document. The *shape* — zero form-quotation failures, all residue self-citation —
   is what I am confident in.
4. **The Form 8275 disclosure's legal sufficiency.** I checked that Part I's six columns are mapped to
   `i8275--2024.txt:352-374`'s own instructions and that Part II carries what `:378-388` requires. Whether
   this particular position clears the *reasonable basis* standard is a tax-judgment question the spec
   correctly refuses to answer for the filer (W-1), and it is outside a spec review.
5. **One thing I noticed and deliberately did not open**, since it is the owner's instrument choice and
   the ruling names it: a Form 8275 attached to a return is inside the filer's §6065 jurat, so "btctax's
   voice" is a rhetorical separation rather than a legal one. §6.3.2 addresses this directly and the
   ruling selected the instrument. Recorded, not filed.
6. **`i8275r--2025.txt` / `f8275r--2025.txt`**, also rewritten by `c7819f8c`. This spec does not cite
   them (they were archived and then set aside at `f203def0` when §G-12 closed on Form 8275 being the
   right instrument), so they are out of scope here — but any *other* document citing them has the same
   C-1 problem and nobody has looked.
