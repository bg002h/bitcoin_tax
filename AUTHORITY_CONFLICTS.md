# Authority conflicts — where we believe the STATUTE and a REGULATION disagree

**Only 26 USC is law.** A Treasury regulation is the executive's *interpretation* of it — binding in
practice, capable of being wrong, and regularly held invalid for exceeding or contradicting the statute
(the more so since *Loper Bright* ended deference). **If we believe the statute disagrees with a
regulation, the filer has a duty to push back**, and the system supplies the instrument: **Form 8275-R**
(Regulation Disclosure Statement), as distinct from Form 8275 for positions contrary to everything else.

★ **That duty is routinely neglected because challenging is expensive. That is a legitimate choice — but
it must be A CHOICE, made on a date, by a person, and revisited.** It must never be an omission that
nobody ever decided. This register exists so it cannot be.

## How this is enforced

`xtask::authority_conflicts` parses every entry below and **fails the test suite** when one is overdue.
A decision here is never permanent: circumstances move — a court rules, a reg is amended, the amount at
stake grows — so **every entry carries a `review-by` date, including the ones already decided.**

★★ **The suite going red on a date is DELIBERATE, not a bug to route around.** It is the only reminder
mechanism in this project that cannot be ignored. If one fires, re-decide and set a new `review-by`;
do not extend the date to silence it without actually revisiting the question.

★ **Scope bound (same shape as `FOLLOWUPS.md` §G-11 and §G-12): btctax does NOT identify these conflicts
itself.** That is legal judgement, out of scope exactly as intent is. A human puts an entry here. The
software's job is bookkeeping and nagging — never legal analysis.

## Entry format

Each entry is a `###` heading followed by these keys, one per line. All are required.

    ### AC-<n> — <one-line summary>

    - **statute:** 26 USC §<cite>
    - **regulation:** 26 CFR §<cite>
    - **disagreement:** <what we believe the statute says that the reg does not>
    - **direction:** the regulation <OVERSTATES|UNDERSTATES> tax relative to the statute — <mechanism>
    - **posture:** <undecided | comply-with-reg | statute-position-with-8275R>
    - **decided:** <YYYY-MM-DD, or "—" while undecided>
    - **review-by:** <YYYY-MM-DD>
    - **why:** <the reasoning for the current posture, including cost if that is the reason>

★ **`direction` is not bookkeeping.** A reg that *understates* tax relative to the statute is the
dangerous one: complying with it is the cheap path AND the path that risks an understatement on a return
signed under §6065. A reg that *overstates* costs the filer money they may not owe — a different problem,
and one where the cost of challenging is more likely to exceed the benefit.

---

## Open conflicts

_None recorded._ We do not currently believe any regulation governing btctax's forms disagrees with the
statute. **That is a statement about what we have examined, not a guarantee** — see the scope bound above.
