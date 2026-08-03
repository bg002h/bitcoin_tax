# Review brief — `income scrub`, immediately before an irreversible publish

## Context

`btctax` is an offline US federal tax-return generator. Returns are signed by real filers under
penalty of perjury (26 USC §6065). This change adds `btctax income scrub`, which takes a filer's REAL
stored return and emits a copy they are told is **safe to hand to a stranger**.

It is about to be published to **crates.io, which is permanent** — crates can be yanked but never
deleted. A leak that ships is a leak that stays.

## The ONE question

**Can a real filer's identity survive `scrub_pii`, or can scrubbing change a filed figure?**

Two failure directions, both serious, in this order of severity:

1. **DISCLOSURE** — some identity-bearing value reaches the output. The user has been told the file is
   safe to share, so they will send it. This is the worse direction.
2. **DISTORTION** — a scrubbed return computes or files differently from the original, sending its
   recipient after a bug that does not exist, or hiding one that does.

## Files

```
crates/btctax-core/src/tax/scrub.rs          the whole implementation + its tests
crates/btctax-core/src/tax/return_inputs.rs  the type being scrubbed (ReturnInputs and friends)
crates/btctax-cli/src/cmd/tax.rs             scrub_return_inputs (the TOML emitter)
crates/btctax-cli/src/main.rs                the IncomeCmd::Scrub dispatch
crates/btctax-cli/src/cli.rs                 the --help text (it makes promises; are they true?)
```

Read them. Do not audit the rest of the repo.

## Facts ALREADY SETTLED — do not re-derive or re-report these

1. A first review already found and FIXED three issues: `business_description` and
   `foreign_country_names` leaked, and `ReturnInputs` was not exhaustively destructured. All three are
   closed. **Re-reporting them is not a finding.** Look for what that pass missed.
2. `naics_code` is KEPT deliberately — a six-digit federal industry taxonomy is not a personal
   identifier. Disagreement here is at most a Nit.
3. The IP PIN is DROPPED rather than replaced, deliberately.
4. EINs are remapped preserving distinctness because §6413(c) turns on "more than one employer".
5. Emitted SSNs use middle group `00`, which the SSA never issues.
6. The figures themselves (income, dependents) are acknowledged as sensitive in the `--help`; the tool
   does not claim anonymity, only de-identification. Restating that is not a finding.

## Where to actually look

* **Coverage.** Walk `ReturnInputs` and EVERY type reachable from it. Is there any `String`,
  `Option<String>`, date, or identifier that can carry identity and is NOT scrubbed? Check nested
  collections. Check types owned by other modules that `ReturnInputs` embeds.
* **Does the scrub break a REFUSAL?** Several fields are load-bearing for fail-closed screens (an
  empty `business_description` refuses; an empty country list with `foreign_accounts = Some(true)`
  refuses). Can the scrubbed copy refuse where the original filed, or file where the original refused?
* **Collisions.** `synthetic_ssn` and `synthetic_ein` are modular. Can two distinct real people or
  employers collapse onto one synthetic value at realistic counts? What does that do to §6413(c)?
* **The tests.** Do they discriminate, or can they pass vacuously? Specifically: does
  `scrub_preserves_every_computed_figure` still compare anything meaningful after the two blanking
  operations? Would `the_identity_does_not_survive` catch a leak in a field it does not name?
* **What is NOT covered by the computed-figure invariant.** It compares `AbsoluteReturn`. What about
  the PRINTED packet and the emitted PDFs — can scrubbing change a filed page in a way no test sees?
* **The `--help` text.** It tells the user what is safe. Is every promise in it actually true of the
  code?

## Output format

Write markdown. Per finding:

```
### [CRITICAL|IMPORTANT|MINOR|NIT] <one-line claim>
**File:line:** ...
**Failure:** <concrete: what identity survives, or what figure moves, and how a filer is harmed>
**Fix:** <one or two sentences>
```

End with exactly: `VERDICT: <n> Critical, <n> Important`

Verify every claim against the source before writing it. A finding citing a line that does not say
what you claim is worse than no finding. If you find nothing at a severity, say so plainly — a clean
result closes this gate, and manufacturing findings to look thorough actively costs, because the next
step is a permanent publish.
