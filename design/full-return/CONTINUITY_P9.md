# CONTINUITY — P9 (the FORM QUESTION REGISTRY)

*Written 2026-07-14 at a deliberate context boundary. Branch `full-return`, HEAD `c2724f5`, tree CLEAN,
`make check` GREEN (1729 passed, exit 0).*

---

## ⚡ THE ONE COMMAND TO RESUME

```
Read design/full-return/CONTINUITY_P9.md and continue P9 autonomously.
```

That is all. Everything below is what that command loads.

---

## 1. Where we are, in one paragraph

We shipped **P8a/D-8** (a real CRITICAL in *published* code: the dependent flag was a bare `bool` with
`#[serde(default)]`, so "never asked" and "answered No" were the same value — it silently understated tax and
printed an unaffirmed checkbox on a filed 1040). Fixing it exposed the **architectural** defect behind it, and
**P9 is the fix for the class**. The P9 **spec is at r3** and has **not been re-reviewed yet**. **No P9 code is
written.**

## 2. ★ THE FINDING (do not lose this)

> **"An input with no safe default must be answered before it reaches compute or print" is a load-bearing
> invariant of this system, and it is the only load-bearing invariant in the codebase still enforced by
> convention instead of construction.**

Every other invariant is structural — money non-negativity by exhaustive destructure, packet completeness by a
no-`..` destructure, the rounding regime by a newtype, SSN validity by a private field with one constructor.
**Answered-ness got five hand-wired obligations across four crates**, and each has been forgotten at least once.
It was flagged **on day one** (SPEC r1 I7: `foreign_accounts: bool` auto-checking Schedule B 7a "No") and
hand-patched for eight phases. It is **~21% of every blocking finding in the program.**

**★ It recurred INSIDE the spec written to abolish it, twice** (r2 I-1: a type flip with no migration re-armed
the D-8 laundering; r1 I-4: the anti-vacuity test was itself vacuous). **The class does not respect the
author's intentions. It is only closed by construction.**

## 3. ★ THE STATUTORY TEST (the owner asked for it; it is now the spine)

- **§6065** — a return is verified *"under the penalties of perjury."*
- **§7206(1)** — felony to subscribe a return not believed true *"as to every material matter."*
  *(Caveat: willfulness (*Cheek*) shields the unwitting filer. Our load-bearing half is the §6065 jurat and the
  design norm — **software cannot supply belief on the filer's behalf** — not a prosecution theory.)*
- ***New Colonial Ice Co. v. Helvering*, 292 U.S. 435, 440 (1934)** — deductions are *"a matter of legislative
  grace"*; the burden to **claim** is the taxpayer's.

> **Does an unanswered box make the filer ASSERT something, or merely FORGO something?**
> **Assert ⇒ no lawful default. Forgo ⇒ `false` is what the statute already assumes.**

**★ THE PRINT CRITERION** (the piece that makes the census a *closed set*, not a list):
> **A Yes/No PAIR can be deferred to the filer's pen** (both-blank is *facially incomplete*).
> **A SINGLE checkbox CANNOT** — unchecked **is** the "No". A single box asserting a fact ⇒ **class (A)**.

## 4. ★ OWNER MANDATES (standing; do not let a review flip them)

1. > *"Let's not forgo a benefit without informing user they may be giving away more money than required."*

   ⇒ Class (B) is **two-part**: a benefit-forgoing default is lawful **only if the filer is told.** We may not
   *refuse* (burden to claim is theirs) — so: **skippable prompt + MANDATORY advisory naming the money.**
   Two live violations found: **`blind`** and **`salt_use_sales_tax`** forgo real money in silence today.

2. **There has never been a user of this software** (v0.2.0 *is* on crates.io, unused). Back-compat is not
   sacred. **But** stored-blob discipline is still schema-versioned, not waved — see r2 I-1.

3. **Fable model escalation requires the owner's approval** — but Fable **reviews at phase gates are standing
   practice** under STANDARD_WORKFLOW and need no fresh ask.

## 5. Exactly what to do next

### STEP 0 — dispatch the **r3 independent review** (nothing else first)

The workflow re-reviews after **every** fold, including the last. r3 is folded and **unreviewed**.

Spawn a Fable agent (`Agent` tool, `model: "fable"`, `subagent_type: "general-purpose"`). Give it:
- `design/SPEC_form_questions.md` (r3, commit `c2724f5`) — under review
- `design/full-return/reviews/P9-SPEC-fable-r2.md` and `-r1.md` — **verify every finding is genuinely folded,
  not merely acknowledged**
- `design/full-return/reviews/P8a-fable-r1.md` — I1 + I3 are still open **by design**; P9 subsumes them
- `design/full-return/reviews/ARCH-P9-fable-question-registry.md` — the architecture consult

Tell it to attack hardest: **(a)** the re-scoped `HsaActivity` — does *"contribute to or take a distribution
from"* actually match the Form 8889 triggers, and does any valid return still get bricked? **(b)** the
`SCHEMA_VERSION` 2 migration — are **all four** unlaunder keys right (`hsa_activity`, taxpayer `blind`, spouse
`blind`, `salt_use_sales_tax`), and is anything else changing type? **(c)** the **print criterion** — sweep the
checkbox surface again independently; is Schedule A line 8 really the last one? **(d)** the classifier `_` rule
— is it implementable as stated? **(e)** the build order — does step *N* compile at step *N*?

**Persist its output VERBATIM to `design/full-return/reviews/P9-SPEC-fable-r3.md` BEFORE folding.** Then fold,
then re-review. **Loop until 0C/0I.**

### STEP 1 — implement, TDD, in the spec's §5 order

**The order matters and r2's did not compile.** Fields → **migration** → registry → anchor+property test →
`screen_inputs` → `answer` → `build` → advisories → the two new refusals → classifier → delete-dead-fields +
unknown-key rejection → LIMITATIONS.

**★ MUTATION-CHECK EVERY GUARD.** This is the project's recurring failure ([[untested-guard-pattern]]): I have
shipped **three** guards with zero tests, and in P8a the *spouse half of the migration* was held by **nothing**
— deleting it left 1729/1729 passing. **After each guard: delete it, run `make check`, confirm a NAMED test
fails, restore.** A fixture must rewrite **every** key it claims to cover **and assert each rewrite lands**.

## 6. Hard constraints

- **FROZEN**: `crates/btctax-core/src/tax/{types,compute,se}.rs`. Verify `git diff 059ec2a..HEAD -- <those>` is
  empty.
- **Validation gate**: `make check` (~7s). **NOT** `cargo test --workspace` (402s).
- **Green** = full suite passes **AND** 0 Critical / 0 Important.
- **Persist every review VERBATIM before folding. Re-review after every fold, including the last.**
- New CLI subcommands need a man page: `cargo run -p xtask -- docs` (the suite enforces it).
- Fish shell: `grep -rn 'x' --include='*.rs'` needs the glob **quoted**. Backticks in `git commit -m "…"` get
  **shell-expanded** — use a heredoc (`git commit -F -`).

## 7. Open findings NOT to re-litigate

- **P8a I1 + I3 are open ON PURPOSE.** P9 subsumes them: I1 = the corrected `DependentSpouse` liveness
  (`Mfj || spouse.is_some()`); I3 = `ReturnHeader::build` refusing on an unanswered live question. Do **not**
  hand-patch them.
- **P8a I2 + M1 are FOLDED** (`e61bf31`).
- The `ScreenedInputs` witness type is **rejected, with reasons** (spec §3.6). It cannot prevent the next D-8:
  a witness certifies the *existing* screens **ran**, not that the *right* screens **exist** — and every
  recurrence here was a **missing or mis-scoped** screen, never a **skipped** one. Do not resurrect it.

## 8. The commit trail

```
c2724f5  spec(P9 r3): fold 1C/6I/6M — the spec RE-ARMED the very bug it exists to abolish   ← HEAD
<r2 review persisted verbatim>
e61fc10  spec(P9 r2): fold 0C/6I/5M + the STATUTORY test + the owner's advisory mandate
afa88e1  spec(P9 r1): the FORM QUESTION REGISTRY
107fd6d  arch(P9): Fable's architecture verdict — persisted VERBATIM
e61bf31  fix(P8a r1): fold I2 + M1 — the SPOUSE half of the migration was held by no test
6de97c2  review(P8a r1): persist Fable VERBATIM — 0C/3I
2b53717  impl(P8a/D-8): the dependent flag is a QUESTION, not a default
```

## 9. Still parked (do not start without the owner)

- **P8** (`design/SPEC_input_surface.md`, r7) — the rest of the input surface (`income template`, import
  screening, `set-pii`). P9 changes its shape; re-read it after P9 lands.
- **P6.7** cleanup batch (12 Minors/Nits).
- **Release decision** — deferred until the input surface is drivable.
- **`p7-tenforty-upstream-followup`** — check for responses on issue #278 / PR #279.
- Open Importants filed → P8: `accounting_method = "accrual"` (unrefused, flips printed Sch C line F);
  `first_negative_amount`'s `header: _` hole (**P9 §3.3 fixes this one**).
