# Brief — review the FOLD of retirement-spec review r1 (round 2)

## The ONE question

**Does the fold introduce defects, and are its adjudications correct against the primary sources?**

A fold is authorship. The text added in response to r1 is the text nobody has read. You are reviewing
**that text**, not the spec as a whole.

## Scope — and what is OUT

**IN SCOPE: `git diff 3a4118011..108017c28 -- design/ty2025/SPEC_retirement_income.md`** (three fold
commits, ~24 KB of new prose). Read the whole spec for CONTEXT, but report only on:

1. the fold text itself — wrong adjudications, wrong citations, internal contradiction;
2. **seams between folds** — the fold of I-5 exists *because* a 2026-09-15 edit changed §9 and left §7,
   §10 and §12 saying "four". That exact failure mode is the highest-yield thing you can hunt: does any
   fold in this range contradict another fold, an S-rule, an R-rule, a §7 field, or a §10 mutation row?
3. anything in the fold that a **build** would implement wrongly.

**OUT OF SCOPE — do not re-audit these.** All 3 Criticals and all 11 Importants of r1 are folded and
closed. C-1/C-2/C-3, I-1, I-6, I-7, I-9, I-10, I-11 landed in earlier commits and are NOT open; raise
them only if a fold in this range **contradicts** one. Do not re-derive r1. Do not open new fronts on
sections the fold did not touch. Do not propose scope additions to the feature.

## Already MACHINE-VERIFIED — do not spend budget re-checking these

Spend it on judgment, primary sources, and whether a build could implement the fold wrongly.

| claim | how it was verified |
| --- | --- |
| suite green | `make check` 3936 run / 3936 passed / 12 skipped at `108017c28` |
| citations in scope | `xtask cite-check` OK, 51 quotations verbatim (note: it does **not** cover this spec — see FR-257) |
| ledger consistency | `xtask ledger-check` OK, disagreement set = expected residue |
| map-row counts 35 / 1 / 34 | `grep -cE '^line[0-9]'` on both `f1040.map.toml`; `line_set` does not match `^line[0-9]` |
| TY2024 map has no 4a–6b | `grep -E '^line(4\|5\|6)'` returns nothing |
| the six field→label bindings | `label_reader::label_join_public("f1040--2024")`: f1_46→4a, f1_47→4b, f1_48→5a, f1_49→5b, f1_50→6a, f1_51→6b |
| field geometry | `xtask dump-fields`: subline column x≈252, amount column x≈504, f1_45=3b, f1_52=7 |
| the T-code year pair | `i1040gi--2025.txt:2713` = "2020"; `i1040gi--2024.txt:2647` = "2018" |
| `SkippableQuestion` shape | 19 of 20 `SKIPPABLE_QUESTIONS` entries carry `unanswered: None`; `BlindTaxpayer` has `live`, `get_bool`/`set_bool` onto an `Option<bool>` |
| `Usd` is signed | probe: `dec!(3000) - dec!(3500)` = `-500`, `is_sign_negative()` true |
| archive integrity | `xtask authority-manifest` OK at 209 entries; `forms extract --adopt` reproduced each body byte-for-byte |

## Where I think the defects are — and the one I am least sure of

★★★ **THE SHARPEST ISSUE, and I may have it wrong: the I-3 fold's treatment of line 4a on the Q/T
sub-branch.** `i1040gi--2025.txt:2698-2715` reads (paraphrasing the structure, quote it yourself):

> "Exception 2. If any of the following apply, **enter the total distribution on line 4a** and see Form
> 8606 and its instructions to figure the amount to enter on line 4b. … 2. You received a distribution
> from a Roth IRA. **But if either (a) or (b) below applies, enter -0- on line 4b; you don't have to see
> Form 8606 or its instructions.**"

My fold asserts the Q/T sub-branch is **4a = Σ box 1, 4b = `-0-`**. That reads the "But if…" sentence as
replacing only the *Form 8606 / line 4b* half, leaving the line-4a instruction from the lead sentence in
force. **The competing reading is that the sub-branch replaces the whole instruction**, in which case 4a
is not addressed at all and my rule table is wrong. Adjudicate against the form, and say which reading
the text supports and why. Check Pub 590-B if it settles it. This is a money line the Service
document-matches against Form 1099-R box 1.

Other places I would look, in descending order:

- **I-2's adjudication table.** Is line 5a really written whenever the Simplified Method runs? I cite
  worksheet line 1 (`:2981-2982`). Does the worksheet run on every partially-taxable pension in v1 scope,
  or only some? If §4.1 makes 5b always Σ box 2a, does the Simplified Method run at all in v1 — and if it
  does not, my 5a claim is vacuous and the table is wrong.
- **I-5's ruling that A-1 and A-2 belong in `SKIPPABLE_QUESTIONS`.** The premise is that each can only
  ever LOWER the figure (S-1). Verify that for both. Is the §86(e) lump-sum election ever
  tax-increasing? Is the PSO exclusion? If either can raise tax, `unanswered: None` is wrong and the
  question must refuse.
- **I-3's "parse to a code SET, never `contains`".** Is that implementable from the field as specified
  (`box7_distribution_codes: String`)? What is the actual separator on a real 1099-R, and does the spec
  say? An unstated separator is how this becomes a `contains` again.
- **I-4's M-1b.** Coverage rows are per FIELD — verify that `line_coverage_check`'s entry type actually
  carries the AcroForm field name, not just form/year/line/quote. If it does not, M-1b's "lookup, not a
  heuristic" is false and I have prescribed something unbuildable. **Check the struct.**
- **I-8's 46-cell figure.** 34 + 6 + 6. Verify the arithmetic and the premise: are all six retirement
  cells genuinely absent from BOTH maps, and is 34 the right count of TY2025's absent cells given that
  TY2024's 35 includes `line7a` which TY2025 already has?

## Severity rules that bind you

- **Critical** = wrong result / data loss / unmet guarantee. **Important** = real defect, missing case,
  unsound assumption. Minor/Nit recorded, do not gate.
- **Secret-handling defects are NEVER Critical or Important** (owner ruling 2026-08-27). Log as
  follow-ups.
- A claim in the fold that is **true but unverifiable as written** (no citation, or a citation that does
  not say it) is at least Important — that is the class that produced r1's I-7.
- Judge citations by opening the extract. `design/forms/extract/` is authority; **never edit it.**

## Output — persist it yourself, then return only a summary

**Write your report with a Bash heredoc — the `Write` tool is refused for report files.** Final action:

```
cat > design/agent-reports/REVIEW-retirement-fold-r2.md <<'EOF'
…your full report…
EOF
wc -c design/agent-reports/REVIEW-retirement-fold-r2.md
```

Then return **only**: the counts (`nC/nI/nM/nN`), one line per finding, and the path + byte count.
Do not paste the report into your reply.

Structure each finding as: `## <SEV>-<n>. <one-line claim>` / **Sections:** / the evidence with quoted
primary source and extract line numbers / **Why <severity>** / **What I would change.**

If a section of the fold is correct, say so in one line and move on — do not manufacture findings. Zero
findings is a legitimate result and the right one if the fold is sound.
