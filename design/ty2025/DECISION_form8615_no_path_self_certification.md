decision(FR-29): OWNER RULING — self-certification where the tax system provides NO PATH

Owner, verbatim: "If tax system has no path, that is why we allow user to
self-certify with warnings."

THE DEAD END THIS ANSWERS, established from three primary sources tonight and
not from reasoning:

  f8615--2025.txt:8-11  the form's face requires A Parent's name, B Parent's
                        social security number, C Parent's filing status. None
                        is optional; all three are structural fields.
  26USC_s1.html §1(g)(3) Part II cannot be computed at all without the PARENT'S
                        taxable income.
  i8615--2025.txt:120-137  the single administrative remedy — request the data
                        from the IRS — requires "The name, address, social
                        security number (SSN) (IF KNOWN), and filing status (IF
                        KNOWN) of the parent". SSN and filing status tolerate
                        ignorance. NAME AND ADDRESS DO NOT. It also requires a
                        statement that the child "tried to get the information
                        from the parent", which a protection order forbids.
  i8615--2025.txt:66-67 the exclusion is "These rules don't apply if NEITHER of
                        the child's parents were living at the end of the year"
                        — which a filer with unknown parents can no more
                        establish than condition 4 itself.

Every route closes. This is a gap in the tax system, not in btctax.

THE RULING: where that dead end is established, the filer may SELF-CERTIFY, with
warnings, and btctax takes the position that §1(g) is not established.

TWO CONSTRAINTS THE CONTROLLER ATTACHES, both from this repo's own doctrine, and
both binding on the fold:

 1. ★★★ THE CERTIFICATION IS GATED ON THE DEAD END, NEVER OFFERED GENERALLY.
    If any filer can elect out of §1(g), FR-29 is rebuilt behind a nicer
    interface — "widening an exemption is never the safe edit" is the exact rule
    FR-29 broke. It must be unreachable until the uncompletable state is
    established.

 2. ★★★ THE FILER ATTESTS TO FACTS, NOT TO A LEGAL CONCLUSION.
    "I do not know who my parents are" / "I cannot obtain their information" is
    testimony a filer can truthfully give. "§1(g) does not apply to me" is a
    legal POSITION — it belongs on Form 8275 in btctax's voice, not sworn in
    theirs under §6065. This repo's "an entry is testimony" rule decides it.

CONSEQUENCE FOR THE SPEC, which the r1 review has not seen: condition 4 cannot
be `Option<bool>`. "I don't know" is a THIRD ANSWER, distinct from UNANSWERED —
unanswered must still refuse, while "unknowable" unlocks the certification path.
That is a type change at the same chokepoint the r4/r5 rounds fought over.

THE WARNINGS, per the owner and sharpened by the controller:
  - disclosure protects against the §6662 accuracy penalty ONLY if the position
    has a REASONABLE BASIS; it is not a blanket shield;
  - the controller is aware of NO authority holding §1(g) invalid as applied
    here — the impossibility argument (the predicate cannot be established, and
    the government's own remedy excludes the filer by its own required contents)
    is materially stronger than a constitutional one and should lead;
  - owner's own words, and they belong in the text: "often the process is the
    punishment when it comes to not allowing government to treat you unlawfully."

★ Instrument: §1(g) is a STATUTE, so the disclosure is Form 8275 — already built
(`crates/btctax-core/src/tax/form8275.rs`). Form 8275-R is for positions contrary
to REGULATIONS and was archived in f203def0, closing §G-12.

★ Beyond the archived sources, flagged as such: the Taxpayer Advocate Service is
the real-world channel for "the system provides no path". Named as a candidate
for the advisory; its current intake criteria are NOT verified against a primary
source and must be before it ships as advice.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01QvsUk3sBD4f1gZxt2hKpMX
