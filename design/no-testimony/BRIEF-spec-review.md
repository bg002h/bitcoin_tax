# §G-11 SPEC review brief

**Artifact under review:** `design/no-testimony/SPEC.md` (draft, one commit old).
**Supporting evidence, read as needed:** `MAP-survey.md`, `CONSULT-architect-fable.md`, `BRAINSTORM.md`.

## The ONE question

**If this spec were implemented exactly as written, would it actually stop btctax from printing sworn
zeros the filer never gave — and would we know if it didn't?**

The second clause matters as much as the first. Every change in this program is a *suppression*, and a
suppression that over-fires is invisible: a missing cell looks exactly like the common correct case on a
mostly-blank return. Neither oracle sees it. No golden moves.

## Where the risk actually is

Spend your budget here, not on prose:

1. **§3, the combinator grammar.** Four productions claim to cover every money line on 16 forms:
   `collected` / `carry` / `total_of` (blank iff all operands blank) / `computed` (always entered; all
   24 form-instructed `-0-`s). **Test it against the actual extracted form text** in
   `design/forms/extract/`. Find a line whose instruction fits none of them, or which fits two. The spec
   says one such line is tolerable and five means the abstraction is wrong — is it nearer one or five?
2. **§3's `total_of` rule specifically.** "A person adding a column of blanks leaves the total blank" is
   asserted, not proved. Is there a line where the form clearly wants a figure even though every operand
   is blank? 1040 line 9 (total income) and line 11 (AGI) are the cases to try hardest.
3. **§4's compiler claim.** Does retyping the fields genuinely `E0063` all 16 constructors and every
   write site — or is there a path that compiles with a silent zero anyway? Look for `Default` derives on
   the printed structs, struct-update syntax (`..Default::default()`), builder patterns, and serde on
   the printed types.
4. **§5, the gate.** Does `transcribe::extract_lines` really omit absent keys (read it)? Can the KAT be
   written as described? **Are the two B1 plants genuinely the right two** — do they cover both failure
   directions, or is there a third way to be wrong that neither reds on?
5. **§6, the containment.** `entered_or_zero` in a test-support extension trait plus a grep-based hygiene
   gate. **Is that sufficient?** Rust has other escapes — a `match` that writes `Usd::ZERO` in the `Blank`
   arm is a legal re-fabrication that no grep for `entered_or_zero` catches. Does the spec need a
   stronger containment, or is "greppable and review-loud" honestly enough? This is the risk the consult
   said would make the program decorative if it leaks.
6. **§7's P0 "zero behaviour change on paper".** Is that actually achievable while migrating
   `push_money`'s signature? Trace it.

## Doctrine to check against (`/scratch/code/bitcoin_tax/CLAUDE.md`)

- **The hard scope boundary** (§1): nothing may flag an omission as suspicious or opine on whether a
  blank is lawful. **Does any part of this spec cross it?** The gate, the census, and the advisories are
  where it would happen.
- **Transcribe, don't paraphrase.** §3 claims the grammar makes constructor choice a *transcription*
  fact. Is that true, or is it a paraphrase wearing a transcription costume?
- **B1 seen-red-once.** Both §5.1 plants and the §6 hygiene kill must be real. Would they red?
- **Blank is the normal case** — assert provenance, never non-blankness.

## Settled — do NOT re-derive

The counts (64/168, band 51–64, seven mechanisms, 24 instructed zeros, the layer split) are from a
5-reader survey, spot-verified by hand. The architecture (two types, emitter-first) is an accepted
consult. The four owner decisions in §8 are the owner's. Tax-figure correctness is not in question.

**Do not propose a different architecture** unless you can show this one cannot work — that decision is
made and re-litigating it wastes the round.

## Output

`VERDICT: green` or `VERDICT: <n> Critical / <n> Important`

Then findings, most severe first:

```
SEVERITY: Critical | Important | Minor | Nit
WHERE: SPEC.md §N (or path:line for a code claim)
CLAIM: one sentence.
CONSEQUENCE: what ships wrong, or what stays undetected, if this is implemented as written.
EVIDENCE: quote the spec AND the code or form text that contradicts it.
```

**Critical** = implementing this as written leaves fabricated zeros printing, or the gate cannot detect
them. **Important** = a real gap, unsound assumption, or missing case that would surface during the
build. Do not inflate — a short green report is a fine outcome for a spec built on a verified survey and
an accepted consult.

End with `ALSO CHECKED, SOUND:` and `WHAT WOULD MAKE THIS REVIEW WRONG:`.

**Constraints:** READ-ONLY. No edits, no commits. Do not spawn subagents. You have a shell — read the
code and the form text rather than reasoning from the spec's description of them.
