# Fable consult — what ships, and what must be true before `income scrub` does

## Your role

You are being asked for a **judgment call**, not a code review. One has already run and its findings
are settled (below). The question is what to do about them.

## The situation

`btctax` is an offline US federal tax-return generator. Returns are signed under penalty of perjury
(26 USC §6065). Ten crates are published to **crates.io, which is permanent** — yank, never delete.

Currently published: **v0.15.0**. Unreleased on `main`, all pushed and gate-green:

| commit | what |
|---|---|
| `ae13ac1` | `btctax --version` (it errored before — no version flag was ever set) |
| `eb4f9c5` | **CTC advisory fix** — see below |
| `b46f608`, `06923c7` | an oracle-validated nine-dependent test fixture (test-only) |
| `31d5c79`, `2449ee4` | **`income scrub`** — the new command under question |

### The CTC fix matters to real users NOW

Published v0.15.0 tells a filer with dependents whose income has phased the Child Tax Credit out
entirely that their tax is *"OVERSTATED by up to $2,000 per qualifying child"* and sends them to file
a Schedule 8812 that would pay them nothing. On a nine-dependent household that reads as up to
$18,000. The correct "the credit is phased out, there is nothing to file" branch existed and was
tested — but production could not reach it, because it needed a gate that is `live: tax_year >= 2025`
and so is never asked on TY2024, the only filable year. `eb4f9c5` fixes it.

### `income scrub` is the new command under question

It takes a filer's REAL stored return and emits a copy they are told is **safe to hand to a stranger**
(so a maintainer can reproduce a refusal without receiving their PII). It emits TOML that
`income import` accepts, and its `--help` promises the scrubbed copy "computes an IDENTICAL return — a
guarantee held by a test, not a hope."

## The findings — READ THEM

**`/scratch/code/bitcoin_tax/reviews/scrub-r1-workflow.md`** — 22 raw → 19 confirmed after
refute-by-default verification, + 6 from a completeness sweep. Do not re-derive them; they are settled.
The source is at `crates/btctax-core/src/tax/scrub.rs` and `crates/btctax-cli/src/{cli.rs,main.rs,cmd/tax.rs}`.

The two that drive the decision:

1. **CRITICAL, empirically reproduced.** `EinMap` keys on the RAW EIN string while §6413(c) compares
   the CANONICAL one, so two spellings of ONE employer (a W-2c; a mid-year payroll change) become TWO
   employers in the scrubbed copy — manufacturing a $1,546.80 excess-Social-Security credit. Since the
   help invites re-import, a scrubbed return can be FILED understating tax. This is the exact failure
   the module's own doc says `EinMap` prevents.

2. **CRITICAL (sweep).** The scrubbed TOML contains **no ledger**. btctax is a *crypto* tax tool; the
   ledger is most of a real return. So the file cannot reproduce a return at all, while `--help` claims
   the opposite. Every test fixture uses an empty `LedgerState`, so no test can see this.

Plus a unifying cluster: scrubbing replaces values but not their **validity class**, so a malformed
SSN/EIN, an empty business description or a bad IP PIN all become valid — and **the scrubbed copy files
where the original refused**, defeating the purpose (you send the file to reproduce a refusal).

## Context you should weigh

- btctax has, as far as the owner knows, **no users yet** — but it IS publicly published and
  installable, so that is an assumption about the world, not a guarantee.
- The owner asked for "push and release and crates". They want a release.
- The repo's standing doctrine: gates are hard; a guarantee without a test that reds when it is removed
  does not exist; widening an exemption is never the safe edit; blank ≠ zero; an entry is testimony.
- The assistant's own recommendation was **ship without scrub** (revert the two scrub commits, release
  `--version` + the CTC fix + fixtures as 0.16.0, land scrub properly later). The owner did not accept
  that; they asked for your judgment instead.

## The one question

**What should be in the next published release, and what must be true before `income scrub` is
published at all?**

Answer concretely. In particular:

- Does the CTC fix's real user harm justify releasing ahead of a fully-fixed `scrub`?
- Is `income scrub` salvageable in its current SCOPE, or is "scrub the inputs but not the ledger" the
  wrong shape for this tool — i.e. should it refuse when a ledger exists, emit both halves, or not
  exist in this form?
- Is there a risk the assistant is UNDER-weighting? Over-weighting? Say so plainly.
- If you would ship something, name exactly what and what its help text must say.

## Output

Markdown. Lead with a one-line verdict, then the reasoning, then a concrete ordered plan. Be decisive —
"it depends" is not useful here. If you think all three offered options are wrong, say what is right.
Do not modify any file.
