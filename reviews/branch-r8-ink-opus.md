# r8 — THE INK LENS (sections 1 and 2 only)

Reviewer: independent Opus pass, `95f7f34..HEAD` on `main`, scoped to `cb29fc1` (Schedule D 18/19) and
`03527f7` (§6413(c) EIN). Sections 3/4/5 were not read.

---

## VERDICT: 0 Critical / 2 Important

Section 1 (Schedule D 18/19) is **clean**. Its load-bearing claim survived a deliberate hunt: I could
not construct a path where btctax carries 28%-rate, collectibles, §1202 or unrecaptured-§1250 gain, so
the condition on lines 18 and 19 is never met and the blank is right. Both kill-tests were verified RED
by mutation.

Section 2 (§6413(c)) has the right **credit** — the canonicalization fold is correct and its kill test
is real — but the **new advisory it added in the same commit is wrong in two ways**, both demonstrated
with live numbers. Neither moves a figure on a filed page; both misinform a filer about real money in
the exact direction the fold existed to close.

---

## FINDINGS

### Important — 1

```
SEVERITY: Important
WHERE: crates/btctax-core/src/tax/return_1040.rs:661
       (gate) crates/btctax-core/src/tax/advisories.rs:598
CLAIM: `excess_ss_not_creditable` fires only when a person had EXACTLY ONE employer, but
       i1040gi's non-creditable rule is PER EMPLOYER — so with two or more employers the
       amount the credit refuses to pay is silently dropped and the filer is told nothing.
```

**FAILURE.** Two probes, run against the real functions (`cargo test -p btctax-core --lib`, TY2024
table, MAX = $10,453.20):

| taxpayer's W-2s | Sch 3 L11 credit | `excess_ss_not_creditable` | advisory |
|---|---|---|---|
| A (EIN 11-1111111) box 4 **$12,000** | $0 | **$1,546.80** | fires |
| A **$12,000** + B (EIN 44-4444444) box 4 **$0** | $0 | **$0** | **silent** |
| A **$11,000** + B **$2,000** | $2,000 *(correct)* | **$0** | **silent** |

Row 2 is the sharpest: adding a second employer who withheld **nothing** leaves the tax outcome
byte-identical ($0 credit, $1,546.80 stranded at employer A) and switches the disclosure off. Row 3
strands $546.80 at employer A while the return correctly pays a $2,000 credit.

**EVIDENCE.** The gate is a whole-person test, not a per-employer one:

```rust
// return_1040.rs:650-668
let eins: BTreeSet<String> = mine.iter()
    .filter_map(|w| w.ein.as_deref().and_then(canonical_ein))
    .collect();
if withheld > max && eins.len() == 1 { withheld - max } else { Usd::ZERO }
```

but the credit beside it caps **per employer**, and that is where the money is refused:

```rust
// return_1040.rs:732-743
let creditable: Usd = eins.iter().map(|e| { … per_employer.min(max) }).sum();
(creditable - max).max(Usd::ZERO)
```

The instruction the commit transcribes states the rule per employer, not per person —
`design/forms/extract/i1040gi--2024.txt` (Schedule 3 line 11):

> "But if **any one employer** withheld more than $10,453.20, you can't claim the excess on your
> return. The employer should adjust the tax for you. If the employer doesn't adjust the
> overcollection, you can file a claim for refund using Form 843."

"any one employer" is satisfied whenever a single employer crosses the cap, irrespective of how many
other employers there were. Every dollar `per_employer.min(max)` discards is a dollar in this class.

The governing rule is the commit's own, quoted from its message and from
`advisories.rs:113-120`: *"a conservative omission is permitted **only if the filer is told**."* With
≥2 employers it is not.

**FIX SHAPE (not prescriptive).** Compute the non-creditable amount from the same per-employer fold the
credit already performs — `Σ_e max(0, withheld_e − max)` per person — which is $1,546.80 / $1,546.80 /
$546.80 on the three rows above and needs no `eins.len()` gate at all.

---

### Important — 2

```
SEVERITY: Important
WHERE: crates/btctax-core/src/tax/return_1040.rs:652-667 (the `.sum()` over both owners)
       crates/btctax-core/src/tax/advisories.rs:268-275 (the message)
CLAIM: On MFJ the advisory POOLS the taxpayer's and the spouse's non-creditable excess into one
       scalar and then asserts it "came from a SINGLE employer" and tells the filer to "ask that
       employer" — a number no employer withheld, attributed to an employer that does not exist.
```

**FAILURE.** MFJ, taxpayer's single employer (EIN 11-1111111) withheld $12,000, spouse's single
employer (EIN 33-3333333) withheld $11,000. Probe output:

```
PROBE mfj credit         = 0            ← correct, per person, never pooled
PROBE mfj not_creditable = 2093.600     ← 1,546.80 (TP) + 546.80 (spouse), POOLED
```

The filer is told:

> "SOCIAL SECURITY OVER-WITHHELD BY ONE EMPLOYER — $2,093.60 more than the §3101(a) cap was withheld,
> but it came from a **SINGLE** employer … ask **that employer** to adjust the overcollection…"

There is no such employer and no such amount. Acting on it means asking one of two employers for
$2,093.60 when that employer over-withheld $1,546.80 and the other over-withheld $546.80.

**EVIDENCE.** The credit is figured separately per person and then summed as a *line* value, which is
correct because Schedule 3 line 11 is one cell. The advisory reuses the same summed scalar as a
*narrative* value, which is not:

```rust
// return_1040.rs:652-667 — one Usd for both people
[Owner::Taxpayer, Owner::Spouse].into_iter().map(|owner| { … }).sum()
```

The instruction is explicit that this quantity is per person —
`design/forms/extract/i1040gi--2024.txt`, same paragraph: *"**Figure this amount separately for you and
your spouse.**"*

**★ The identical class was already fixed one advisory below it, in the same file, with the reasoning
written down** (`advisories.rs:252-256`):

> "★ The VALUE of answering is the whole forfeit, not one box. `persons` already selects the pronoun;
> quoting the per-box figure beside 'you and your spouse' told an MFJ couple $1,550 when $3,100 was at
> stake. **A wrong number in the text that exists to make the number vivid is worse than no number.**"

`Mfs63fSpouseBoxesForgone` carries a `persons` discriminator precisely so its MFJ prose is right; the
new variant carries only `amount`. The fix already existed in the file and was not carried across —
which is the §B3 failure shape (a defect that lives outside every reviewer's window), one file apart.

---

## Minor / Nit

```
SEVERITY: Minor
WHERE: crates/btctax-core/src/tax/return_1040.rs:657-659
CLAIM: `single_employer_excess_ss` DROPS an undecidable EIN via `filter_map` instead of treating the
       person as undecidable — the opposite of its sibling `excess_social_security`, added in the same
       commit, which returns $0 on the first unparseable EIN "if that screen is ever bypassed".
```

Probe: taxpayer with two W-2s, $6,000 each, EINs `11–1111111` (U+2013 en dash — realistic from a
copy-paste) and `22-2222222`:

```
PROBE dash credit         = 0           ← conservative, correct
PROBE dash not_creditable = 1546.800    ← advisory would fire
```

The advisory would then assert *"it came from a SINGLE employer … (Schedule 3 line 11 is $0 **and that
is correct**)"* to a filer with **two** employers who is owed a $1,546.80 credit. The affirmative "and
that is correct" is what makes this worse than silence.

**Latent, not live**: `screen_inputs` (`return_refuse.rs:820-836`) refuses over-cap-with-an-undecidable-EIN,
and both live `advisories_for` call sites (`cmd/tax.rs:364`, `cmd/admin.rs:813`) are downstream of it —
`report_tax_year` returns early on `ProfileOutcome::Uncomputable` (`cmd/tax.rs:296-298`), and
`export-irs-pdf` screens before assembling. Hence Minor. But the function's own doc comment
(`return_1040.rs:647-649`) states *"Zero when … identity is unknown"* as a property of the function,
and it is not one — the parenthetical "(then the screen refuses)" is carrying the whole guarantee. Give
it the same `None ⇒ Usd::ZERO` arm its sibling has and the doc becomes true.

```
SEVERITY: Minor
WHERE: crates/btctax-core/src/tax/printed.rs:745-746
       crates/btctax-forms/src/map.rs:835 and :838
       crates/btctax-forms/src/schedule_d_full.rs:192
CLAIM: Four doc sites still state the defect `cb29fc1` removed — "L18 = L19 = 0" / "always 0" —
       including the CORE definition of `ScheduleDRouting::BothGains`, three lines above the emitter
       comment that says they are blank.
```

- `printed.rs:745` — `/// **L16 > 0 and L15 > 0** … L17 = **Yes**; L18 = L19 = 0 (the 28%-rate and
  unrecaptured-§1250 amounts, both refused upstream if they could ever be nonzero)`
- `map.rs:835` — `/// L18 — 28%-Rate Gain Worksheet (always 0; a nonzero amount is refused upstream).`
- `map.rs:838` — `/// L19 — Unrecaptured §1250 Gain Worksheet (always 0; refused upstream).`
- `schedule_d_full.rs:192` — `// L16 > 0 and L15 > 0 — both gains. 17 = Yes; 18 = 19 = 0; 20 = Yes → QDCGT.`

No ink moves. Filed because this repo's most recent Critical was *"the field's own doc comment stated
the governing rule while the code violated it"* (`03527f7`), and because `printed.rs` is the doc a
future implementer of Part III reads first. This is the whole-surface-sweep rule.

```
SEVERITY: Nit
WHERE: crates/btctax-forms/src/cells.rs:42-44
CLAIM: `push_money_opt` was inserted between `push_money`'s doc comment and `push_money`, so the
       `descent` documentation now belongs to the wrong function and `push_money` — the one that
       always writes — is undocumented.
```

```rust
/// Emit the write(s) + flat placement(s) for a money cell. `descent = Some((group, ordinal))` puts
/// the (dollars) field into a strictly-descending-y sequence; `None` makes it column-only.
/// Write a money cell **only if there is a value** — `None` writes nothing at all.
…
pub fn push_money_opt(…)

pub fn push_money(          // ← no doc comment at all
```

Two conflicting summary sentences render as `push_money_opt`'s rustdoc, and `descent`'s only
explanation is now attached to the wrapper rather than the writer.

---

## ALSO CHECKED, SOUND

**Section 1 — the load-bearing claim is TOTAL for every return btctax can emit.** I attacked it four
ways and it held:

1. **1099-DIV boxes 2b / 2c / 2d.** All three refuse via `UnrecapturedOrSpecialRateGain`
   (`return_refuse.rs:914-922`), on `> Usd::ZERO`; the *negative* side (a collectibles **loss**, which
   the instruction's *"collectibles gain or (loss)"* also triggers on) is caught separately by
   `first_negative_amount` (`return_refuse.rs:414-422`), which runs first in `screen_inputs`
   (`:577`). No gap between the two.
2. **The screen is on every emission path.** `resolve_core` (`cli/resolve.rs:96`) and
   `input_form_store::commit` (`:317`) gate storage and resolution; `export-irs-pdf` re-runs it
   (`cmd/admin.rs:794`) before `assemble_printed_return`; `report_tax_year` returns early on
   `Uncomputable`. No path reaches `fill_schedule_d_full_with_map` with those boxes set.
3. **The ledger cannot produce a collectible.** Schedule D lines 15/16 are exactly
   `8949 LT totals + box 2a − LT carryover` and `8949 ST totals − ST carryover`
   (`printed.rs:857-918`); the 8949 rows are Bitcoin in satoshis; there is no 1099-B, Form 2439, Form
   4797, Form 6252, Form 6781, Form 8824 or K-1 input anywhere in `ReturnInputs`. Carryovers and box-2a
   distributions are neither of the two worksheet triggers.
4. **The instructions' triggers, read from the archive, are the two btctax refuses.**
   `i1040sd--2024.txt:1619-1631` — line 18 requires only a §1202 exclusion or a collectibles gain/loss
   *reported in Part II of Form 8949*; `:1768-1790` — line 19 requires §1250 property, installment
   §1250 payments, a K-1 showing unrecaptured §1250, or a 1099-DIV / Form 2439 reporting it. btctax can
   express none of them.

**Line 20 is still Yes, and it is right.** `f1040sd--2024.txt:97` — *"Are lines 18 and 19 both zero **or
blank** and you are not filing Form 4952?"* A blank answers it identically to a zero, which is what
makes the change safe rather than merely defensible. The Form 4952 conjunct is also satisfied on
btctax's own terms: Schedule A line 9 is unmodeled and printed blank (`printed.rs:1250`), so the return
btctax emits does not file Form 4952.

**The two kills are real — verified RED by mutation, then restored by `cp`:**

- `push_money_opt(…, Some(Usd::ZERO), …)` on line 18 → `L18 is BLANK …  left: Some("0")  right: None`
- same on line 19 → `L19 is BLANK …  left: Some("0")  right: None`
- `canonical_ein` reverted to trim-only → `excess_social_security_per_person_not_pooled` fails
  `left: 1546.800  right: 0`

**`push_money_opt` itself is correct.** `None` pushes neither a write nor a `FlatPlacement`, so it is
byte-identically the "never called" path — there is no empty-string write and no geometry entry, and
the emitted PDF's `text_value` for `f2_02[0]`/`f2_03[0]` is genuinely absent (asserted on the real
serialized bytes, not on the write list).

**Descent bookkeeping is sound.** `p2_ord` is only advanced by `push_p2`, which in `BothGains` now runs
exactly once (line 16, ordinal 0). Group `GRP_P2_AMOUNT` is a singleton in that branch and ordinals
0/1 in `NetLoss` — unchanged. The `need(&map.line18/19, …)?` calls are retained, so a map lacking those
cells still fails closed even though nothing is written to them.

**Nothing downstream reads the removed cells.** No struct field carries Schedule D 18/19 (confirmed by
the `no-..` destructure at `line_coverage.rs:1277-1291`), so the coverage table, the QDCGT routing and
the tax computation never saw them; `transcribe::extract_lines` treats absence as blank by construction
(`transcribe.rs:34-36, 65-72`); `golden_packet.rs` compares no Schedule D 18/19 (its only `line18` is
Form 8959's); `no_unmapped_filled` can only be broken by writing *more*, not less.

**Section 2 — the credit itself is right.**

- `canonical_ein` is the correct canonical form for the cases that matter. A space instead of a hyphen
  (`11 1111111`) canonicalizes to the same employer — correct. A leading `+`, a unicode en/em dash, a
  non-ASCII digit or eight digits all return `None` — undecidable, which routes to the refusal, i.e.
  fail-closed. `digits.len()` is a byte length, but a 9-byte string that passes
  `all(is_ascii_digit)` is necessarily 9 ASCII characters, so the two checks cannot disagree.
- Canonicalization can only ever *merge* identities, and merging reduces the credit — the overstatement
  direction, which is the only one this codebase tolerates.
- `excess_social_security` fails closed on an undecidable EIN (`:723`), and the per-employer
  `min(withheld_e, max)` construction (settled against Pub 505 Worksheet 3-1) survives the two adverse
  shapes I tried: A=$11,000/B=$2,000 → $2,000, and A=$12,000/B=$0 → $0 (a zero-withholding second
  employer does **not** unlock a credit, because the cap is applied before the sum).
- **MFS works.** A spouse-owned W-2 on a non-joint return refuses first (`return_refuse.rs:776-781`),
  so the `Owner::Spouse` leg of both functions is inert off MFJ, and the cap stays per person.
- **Zero known EINs over the cap cannot reach the advisory**: `over_cap_needs_ein`
  (`return_refuse.rs:820-832`) refuses on *any* undecidable EIN when the person is over the cap, which
  strictly subsumes the all-unknown case.
- The new `Advisory` variant reaches the filer on both surfaces — `render_advisories` dispatches
  through the exhaustive `Advisory::message()`, so no call site could have been missed silently.
- `excess_ss_not_creditable` appears on no printed line (only `advisories.rs:598` and the test literal
  at `printed.rs:1423`), so it cannot leak onto a page.

**`make check` is green on the restored tree: 2559 passed, 12 skipped, 15.9s.**

---

## WHAT WOULD MAKE THIS REVIEW WRONG

1. **If a filer can state a 1099-DIV box 2b/2c/2d amount somewhere `screen_inputs` does not see.** I
   traced storage (`input_form_store::commit`), resolution (`resolve_core`), the report and
   `export-irs-pdf`, and all four screen. I did **not** audit the TUI's own compute surfaces or
   `what-if` (they are section 3's scope) — if either assembles a `PrintedReturn` without
   `screen_inputs`, my "the refusal is total" conclusion narrows to the paths I checked. Note the blank
   would still be the *safer* of the two options there.
2. **If `#[serde(default)]` on `box2b_unrecap_1250` is doing more work than I credited.** A TOML import
   that simply omits the key yields a hardcoded `0`, indistinguishable from "the filer has none" — the
   §G-11 class, one level up. That would mean a real unrecaptured-§1250 amount is *omitted*, not
   *refused*. It does not make the blank wrong (a blank asserts nothing; the old `0` swore box 2b was
   zero), but it means the sentence "btctax never HAS such gain" is really "btctax refuses when told".
   The interactive input form exempts `int_1099`/`div_1099`/`g_1099` wholesale
   (`input-form/src/spec/coverage.rs:299-303`), so the TOML is the only way in — and the only way to
   get it wrong.
3. **If the per-employer-cap construction is not what Pub 505 Worksheet 3-1 says.** The brief settled
   this and I took it as settled; Important-1's fix shape depends on it, since it reuses the same fold.
4. **If Important-1 and Important-2 are judged out of the fold's scope.** Both are properties of code
   added by `03527f7` itself, and neither existed before it — but neither moves a figure, and a reader
   who scores only ink would call both Minor. I scored them Important because the fold's own stated
   rule ("a conservative omission is permitted only if the filer is told") is the thing they break, and
   because this branch is already treating a misdirecting advisory (R1, the CTC phase-out) as a real
   defect worth a commit.

---

**Tree state: CLEAN.** Three temporary mutations (`schedule_d_full.rs` ×2, `return_1040.rs` ×2 —
the trim-only `canonical_ein` and a scratch probe test) were each backed up with `cp` to `/tmp` and
restored with `cp`; `git status --porcelain` shows only this untracked report and the untracked
`reviews/BRIEF-r8.md`. No `git checkout --` was used. No commits. No subagents.
