# BRIEF — scoped fold-diff check on `design/SPEC_income_scrub.md` (r5)

**This is NOT a review of the spec.** It is a narrow check of one diff. r4 (the previous independent
reviewer) recommended against a fifth whole-document round and suggested confirming the fold by reading
it against its own report. That is this task, done independently rather than by the author.

## The scope — nothing outside it

```
git show 27f9a633            # the r4 report, verbatim — the findings being answered
git diff 27f9a633..178b941e  # THE FOLD — the only text under examination
```

Read `design/SPEC_income_scrub.md` for surrounding context where the diff needs it. **Do not audit the
rest of the document** — four independent rounds have covered it, and their CLEAN sections record what
each could not break.

## The ONE question, in two parts

For **r4's I-1** (§3.3 named the wrong blind spot: structural absence, not value collision) and
**r4's I-2** (§3.3's third verdict was authorised by "the type", which does not decide malformed-ness):

1. **Did the fold actually CLOSE the finding** — or does it restate the finding in the imperative and
   leave the defect reachable?
2. **Did the response introduce a NEW defect?**

Then the same, briefly, for the three Minors and the Nit: M-1 (§5.1's key re-keyed to `w2s[].ein`),
M-2 (the `dependents[].date_of_birth` row), M-3 (`serde_json` not `toml`, presence-is-a-difference),
N (§8's round numbers).

## The two specific things worth pressure

- **I-1's fix is "the sentinel fixture is MAXIMAL"** — every `Option` is `Some`, every `Vec` ≥ 2
  elements, every nested struct present, written as an exhaustive literal with no `..` so a new field
  is a compile error in the fixture. **Does maximality actually guarantee every replaced field yields a
  differing path?** Look for a replaced field that would *still* produce no diff on a maximal fixture.
- **I-2's fix splits the discriminators** — `absent` decided by the type, `malformed` decided by
  whether any predicate reads a validity class off the field (three canonicalizers named). **Are those
  three the complete set, and does the split leave any cell unauthorised or double-authorised?**

## Already machine-checked — do not re-verify

- **41/41 `file:line` citations in r5 resolve; 0 out-of-range; 0 ambiguous.**
- The full suite (2646 tests) + `cargo fmt --all --check` are green at `178b941e`.
- r4's own findings were independently reproduced against source before folding: zero fixtures set
  `ip_pin`, `foreign_country_names` or any `ein:`; the only `b_1099` is in `amt_owing_household`;
  `Ssn::canonical` reads spouse (`packet.rs:208`) and dependents (`:425`); `scrub_dependent` writes
  unconditionally (`scrub.rs:111`); `btctax-core` depends on `serde_json`, not `toml`.

## FORBIDDEN

- Auditing any section the diff does not touch.
- Re-reporting an r1/r2/r3/r4 finding, or re-verifying the machine-checked list.
- Style, prose, wording, document length.
- Manufacturing a finding. **"All closed, nothing new" is the expected result and a useful one.**

## OUTPUT FORMAT

```
VERDICT: <all-closed | needs-changes>

I-1: <closed | partially | reopened | new-defect>  — one or two sentences of evidence
I-2: <closed | partially | reopened | new-defect>  — one or two sentences of evidence
M-1 / M-2 / M-3 / N: <closed | ...>                — one line each

NEW DEFECTS INTRODUCED BY THE FOLD: <none, or list with severity C/I/M/N, where, and the failure vector>

WHAT WOULD MAKE THIS CHECK WRONG: <one sentence>
```
