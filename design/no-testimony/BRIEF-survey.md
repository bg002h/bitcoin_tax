# §G-11 survey brief — which money lines can FABRICATE TESTIMONY?

You are producing the raw material for a brainstorm. **This is a survey, not a fix.** Do not change
code. Do not propose a design. Answer the question below for every line in your assigned scope, with
evidence, and stop.

## Read first

- `FOLLOWUPS.md` §G-11 (search `### G-11`) — the filed defect, in full. It is short; read all of it.
- `/scratch/code/bitcoin_tax/CLAUDE.md` — especially **"an entry is testimony"** and
  **"blank is the normal case"**. These are the authority.

## The defect, in one paragraph

A blank line and a printed `0` are **different speech acts** on a return signed under 26 USC §6065.
A blank says *nothing*. A `0` is an affirmative sworn statement that the amount **is** zero. Today
`btctax-forms/src/lib.rs:77` is the entire money path — `fn fmt_money(d: Usd) -> String { d.to_string() }`
— and every money field on every printed struct is `Usd`, never `Option<Usd>`. **There is no
representation for "no testimony", so no line can choose it.** Where a `0` reaches the page because
nobody was ever asked, btctax has fabricated testimony under the filer's signature.

## The ONE question, per line

**Can this line print `0` (or any figure) on a return where the filer was never asked, and never
supplied, the fact behind it?**

If yes, that line is a *fabrication site*. If no — because the line is arithmetic over other printed
lines, or the form itself instructs `-0-`, or the value is always collected — it is not.

## ★ The form is the authority, and it is unusually explicit here

IRS forms **say** when to write a zero. Phrases like *"enter -0-"*, *"if zero or less, enter -0-"*,
*"If the result is zero or less, enter -0-"* are instructions to make an affirmative statement. Their
**absence**, combined with no entry, is a blank.

So for every line, look it up in the extracted text under `design/forms/extract/` (e.g.
`f1040--2024.txt`, `f1040sa--2024.txt`, `f8995--2024.txt`) and record **verbatim** what the form says.
Transcribe; do not paraphrase. If the form is silent about zero, say so — silence is itself the finding.

## Classify every money field in your scope

For each, record:

| column | meaning |
|---|---|
| `line` | the form's own line number (e.g. `Schedule A line 12`) |
| `field` | the Rust field name |
| `provenance` | `Computed` (arithmetic over other printed lines — name them) · `Transcribed` (copied from a `ReturnInputs`/ledger value — name it) · `Constant` · `Conditional` (present only under a stated condition) |
| `form_says_zero` | verbatim quote of any `-0-` instruction, or `(silent)` |
| `source_can_be_unstated` | is there an input path — including a hand-edited `income import` TOML — where nobody supplied the underlying fact? |
| `prints_today` | what actually reaches the PDF in that unstated case. **Check the emitter, not just the struct.** |
| `fabrication_site` | **yes / no**, and one sentence why |

## Rules that will keep you accurate

1. **`Computed` is usually NOT a fabrication site.** "Add lines 1 and 2" over two stated lines yields
   stated arithmetic. But if an *input* to the computation was itself unstated and silently treated as
   zero, the defect has moved upstream — say so, and name the upstream field.
2. **Check the EMITTER for existing zero-suppression.** Some rows are already suppressed ad hoc
   (`schedule_d.rs`, `fill8949.rs`). Row-level suppression is not line-level blank-vs-zero; record
   which you found.
3. **A `0` that the form instructs is CORRECT and is not a finding.** Most zeros on a tax return are
   right. The defect is narrow: a zero nobody was asked for.
4. **Do not opine on whether a blank would be lawful.** Intent is out of scope in both directions —
   §G-11 says so explicitly. You are classifying representability, not legality.

## Output

Return the table for your scope, then:

`COUNTS:` total fields examined · fabrication sites · already-suppressed · form-instructs-zero

`SHARPEST EXAMPLES:` the 2–3 lines where a fabricated zero would most plausibly reach a real filed
return, with the concrete input path that produces it.

`UPSTREAM:` any place where an unstated input is silently coerced to zero *before* the printed struct —
i.e. the defect is in `return_1040.rs`/`compute.rs`, not the emitter. This may be the more important
half; say so if you find it.

`WHAT I COULD NOT DETERMINE:` be honest.

**Constraints:** READ-ONLY. No edits, no commits. Do not spawn subagents. `cargo test`/`grep`/`git` are
fine. Quote the form text and the code; do not summarize them.
