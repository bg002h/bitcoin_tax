# BRIEF — independent re-review of `design/SPEC_income_scrub.md` (r2)

## The ONE question

**Is this spec safe to build from?** Concretely: if a competent implementer follows it literally and
`main..HEAD` then goes green, does `btctax income scrub` ever tell a filer "this file is safe to hand
to a stranger" when it is not — or emit a file that computes a different return than the original?

Answer that. Do not answer anything else.

## What this command is, in one paragraph

`btctax income scrub` takes a year's `ReturnInputs` out of an encrypted vault and emits a TOML copy
with the **identity replaced and every figure intact**, so a filer can send a real return to a stranger
to reproduce a defect. **The product is the authorization, not the file.** A wrong figure is a bug the
filer can audit; a wrong safety claim is one nobody can audit afterward, and it persists in every
installed copy forever (crates.io yanks, never deletes). The spec is deliberately written to make the
command **refuse more than it scrubs**.

The code already exists on this branch (`31d5c79`, `2449ee4`), was held back from every release, and
this spec is the rework order for it. **Nothing has been built against r2 yet.**

## Read these — do not have them pasted to you

| path | what it is |
|---|---|
| `design/SPEC_income_scrub.md` | **the artifact under review (r2, 278 lines)** |
| `reviews/scrub-spec-r1-review.md` | r1 of this same spec: 31 raw → **19 blocking**, ALL folded into r2 |
| `reviews/scrub-r1-workflow.md` | code review of the held implementation: 19 confirmed + 6 sweep, 2 CRITICALs |
| `reviews/scrub-r2-fable-consult.md` | scope adjudication (ship 0.16.0 without scrub; scrub alone later) |
| `crates/btctax-core/src/tax/scrub.rs` | the held implementation (423 lines) |

## ★ SETTLED — do not re-derive, do not re-report

r1's 19 blocking findings are folded. **Re-reporting a folded r1 finding is noise, not a finding.**
The following are decided and are NOT open questions:

- **`absent → absent` for SSN stands** (§6). The alternative was shown to be a correctness regression.
- **Scrubbing the ledger is permanently REJECTED** (§2.3) — figures on a public chain *are* the
  identifier; you cannot figure-preservingly de-identify them. Do not re-propose it.
- **No perturbing / rounding / fuzzing of figures** (§2.4) — crosses brackets and phase-outs.
- **The refusal is a `CliError`, not a `RefuseReason`** (§8 step 2) — `RefuseReason` is the taxonomy of
  *why a return cannot be FILED* and has an exhaustive cross-crate anchor map with no honest entry for
  a sharing refusal.
- **Dependent `date_of_birth` is dropped** (§6); **`naics_code` and box-12 codes are kept** (§7).
- The ceremony choice (one spec, no separate plan; build order is §8) is per `STANDARD_WORKFLOW.md` §8
  and is not a finding.

## ★★ ALREADY MACHINE-CHECKED — spend no budget re-verifying these

I resolved every citation in the spec against current source at HEAD (`1e807f5`). Results:

**Confirmed exactly right:**
- §7 "the `..`-free destructure guard covers **4 of 10** structs" — exact. Guarded: `Person`,
  `Dependent`, `HouseholdHeader`, `ReturnInputs`. Unguarded (targeted mutation, no destructure):
  `schedule_c`, `w2s`, `int_1099`, `div_1099`, `b_1099`, `g_1099`. Zero `..` in the four.
- §7 "correct `scrub.rs:14`, which overclaims" — line 14 is exactly the overclaiming sentence.
- §3.1 the `$1,546.80` reproduction — correct and **live-probed** by r1
  (`PROBE scrub excess_ss = 1546.800`). `EinMap::map` (scrub.rs:68-74) keys the **raw** string;
  `canonical_ein` (return_1040.rs:681) is what §6413(c) actually compares.
- §3.2 "the held code normalizes every class into valid" — `scrub_person` sets
  `ssn: synthetic_ssn(n)` unconditionally, discarding `ssn: _`.
- §3.4 the Springfield/IL/62704 collision — real: `scrub.rs:142,146` vs `testonly.rs:260,262,419,421`.
- §3.3 "**both class-level tests are VACUOUS on today's fixtures**" — confirmed. Every fixture SSN is
  well-formed (`111-22-3333`, `123456789`, `987654321`); there is no absent or malformed cell anywhere
  in `btctax-core/src/tax/testonly.rs`.
- §3.3 "6 of 16 fields named; the entire spouse untested" and "`dependents.len()` not asserted before
  the `zip`" — confirmed in `the_identity_does_not_survive` (scrub.rs:369-422).
- §3.3 `blank_identity` currently normalizes 2 of the 3 named places (missing
  `excess_ss_not_creditable[].ein`) — confirmed at scrub.rs:322-327.
- §4.1 "`--out` uses bare `std::fs::write`" — confirmed, `btctax-cli/src/main.rs:312`.
- `Session::project()` (session.rs:607), `Loaded::Draft` (input_form_store.rs:138), and the pii-scan
  `ALLOWED_EIN_REVIEW_ARTIFACT` bucket all exist as described.
- `canonical_ein` is `pub(crate)` and is **already** called cross-module from `return_refuse.rs:941`,
  so §3.1's fix needs no visibility change.

**Three facts the spec does not currently state — these are INPUTS to your review, not findings I am
asking you to confirm. Judge whether each is a defect in the spec:**

1. `SsnError::NotDigits` carries a **`char` payload** (`packet.rs:95-102`). §3.2 writes
   `NotDigits → NotDigits` payload-free while explicitly naming `WrongLength(n)`'s payload.
2. **`income import --force` does not exist today.** `cli.rs:399` has only `--year` and `--file`.
   §2.1/§4.3/§5 reference the flag as the recipient's load path.
3. `digital_asset_activity` (return_1040.rs:2013) and `first_hard_blocker` (compute.rs:462) are
   private in **two different modules**, and neither is `scrub`. §2.2's ★ note says "module-private
   today" in the singular.

## ★★★ WHERE THE RISK ACTUALLY IS — spend your budget here

**A fold is authorship, and r2's own edits are the text nobody has read yet.** r1 reviewed r1. The
sections below are substantially NEW in r2 and have had **no independent eye on them**:

1. **§2.2 `ledger_contributes` — the four-disjunct refusal predicate.** This is the safety gate. Is the
   disjunction **complete**? r1 killed the previous predicate for confusing *year-scope* with
   *artifact-scope*. Ask whether r2 fully repaired that or merely patched the two instances r1 named:
   is there any other projection-wide, year-blind, or prior-year input that reaches a figure, refusal,
   gate, watermark or advisory and is **not** covered by these four disjuncts? Is `year - 1` the right
   carryforward window? What about carryforward *out* (a later year), or a year the filer has not yet
   authored?
2. **§3.2's IP PIN carve-out** (new in r2). "malformed → a malformed non-credential stand-in" — is that
   coherent, and is it actually the only leg `ReturnHeader::build` reads? Does a *malformed* stand-in
   leak anything about the original, and can it ever canonicalize into something well-formed?
3. **§3.3's fixture-matrix obligation.** `{SSN, EIN, IP PIN, business_description} × {absent,
   malformed, valid}` failing on an empty cell. Is that matrix the right axis set — and is "the same
   `RefuseReason` variant" the right equivalence, or does variant-equality admit a real behaviour
   difference (e.g. same variant, different payload, different downstream advisory)?
4. **§4's marker + import guard.** A pre-parse *text scan* for a leading comment token. Can a scrubbed
   file lose its marker in normal handling (re-serialization, an editor, a round-trip through the TUI)
   and thereby become silently importable over a real vault? §4's ★ grades that path **data loss /
   Critical**.
5. **§8's build order as a sequence.** Does any step depend on something a later step creates? Step 2
   makes the refusal a `CliError` — check that against `scrub_return_inputs`'s actual signature and the
   `None => "No full-return inputs set"` arm at main.rs:307-321. Does the order ever leave the tree in
   a state where scrub emits with the *old* predicate and the *new* claims?
6. **§5's permitted claims vs what the tests actually hold.** §5 forbids "computes an IDENTICAL return"
   unqualified and names two surviving divergences. Are there others the spec has not named?

## FORBIDDEN

- Re-auditing the tax logic, the ledger engine, or anything outside `income scrub`'s blast radius.
- Style, prose, naming, section ordering. "§X disagrees with §Y" is a lookup, not a finding — a
  document need not agree with itself; it must agree with the code and the form.
- Re-reporting a folded r1 finding (see SETTLED).
- Proposing "add more tests" without naming the **specific defect** the test catches and the mutation
  that would kill it.
- Proposing generic self-verification scaffolding ("add a final verification step").
- Recommending the ledger be scrubbed, or figures perturbed. Both are permanently rejected.

## Severity rubric (this repo's, and it is not the usual one)

- **Critical** — a wrong tax figure, data loss, a security exposure, or **a false safety
  authorization**. Understatement is worse than overstatement.
- **Important** — a real defect, a missing case, an unsound assumption, or a guarantee the spec claims
  that no test could hold.
- **Minor / Nit** — recorded, does not gate.

Critical and Important **block**. This spec does not proceed to build while one is open.

## OUTPUT FORMAT

```
VERDICT: <ready-to-build | needs-changes | wrong-shape>

CRITICAL: <n>
IMPORTANT: <n>

For each finding:
  [C|I|M|N] <one-line title>
  WHERE: <spec §, and the source file:line it is about, if any>
  FAILURE: <concrete inputs/state → the wrong outcome. Not "could be unclear" — name the vector.>
  FIX: <the smallest change that closes it>

WHAT WOULD MAKE THIS REVIEW WRONG: <one sentence naming the assumption your findings depend on>
```

If you find nothing blocking, say so plainly and say **which section you looked hardest at** — a clean
result closes the loop and does not trigger another round. Do not manufacture findings to look
thorough; equally, do not soften a Critical into a Minor because the spec is otherwise careful.
