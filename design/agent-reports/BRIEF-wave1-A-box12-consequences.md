# BRIEF A — FR-206 (understates tax), FR-205 (code H), FR-207 (an invalidated reason)

**opus. worktree. YOU OWN: `crates/btctax-core/src/tax/return_refuse.rs`,
`crates/btctax-core/src/tax/advisories.rs`, `crates/btctax-forms/forms/2024/f1040s3.map.toml` and its 2025
sibling.** Nothing else. No subagents.

## FR-206 first — it is the only open item that UNDERSTATES tax

`return_refuse.rs:42` — `const ELECTIVE_DEFERRAL_CODES: &[&str] = &["D","E","F","G","S"]` — is the set whose
cross-employer sum is capped under **§402(g)**. The limit **also covers designated Roth** deferrals, per the
W-2 instructions' own worked example. So an excess deferral can go **undetected**, and an undetected excess is
income that should have been added back and was not.

1. **Adjudicate the cap rule** against §402(g) and `iw2w3--2026.txt`'s worked example. Quote the sentence you
   rely on, with its extract line.
2. **Derive the code set from the table FR-197 just built** — do not type a second list beside it. That is the
   whole lesson of FR-197: a hand-typed list was not merely short, it was *wrong about one of its eleven*.
3. B1: plant a household whose deferrals exceed the limit only when the Roth codes are counted, and show it
   refuses/adds back; show one under the limit still files. Paste both.

## FR-205 — code H should FILE with an advisory, not refuse

FR-197 correctly found code H (§501(c)(18)(D)) was wrongly inert: the amount is in box 1 as wages and *"the
employee will deduct the amount on their Form 1040"* — Schedule 1 line 24f, censused `unmodeled`. So btctax
filed the wages and never took the deduction out: a **silent overstatement of the filer's own tax.** It now
refuses.

★★ **Refusing is the wrong resting place and this repo's own doctrine says so.** For a forgone
TAXPAYER-FAVOURABLE amount v1 cannot compute, the established treatment is **file + advise**:
`Advisory::EicOmitted` — *"EIC NOT COMPUTED … Your tax may be OVERSTATED. Check Pub. 596"* — and
`Advisory::MixedUseMortgageNotAllocated`, which *"names the whole forgone amount as a CEILING"*. Code H is that
class: the wages are truthfully reported, only the deduction is forgone, so the return is
correct-but-conservative rather than wrong. Refusing denies a filable return over a deduction the filer may
not claim. ★ And the owner ruled 2026-09-13: *"support all the tax scenarios."*

Build the advisory the same way its two siblings are built: name Schedule 1 line 24f, name the box-12 amount
as a **ceiling** (never "the amount lost"), fire only when a code-H entry exists, and give it a census entry
and a kill. ★ Check whether any other currently-refused code is the same class — a forgone taxpayer-favourable
amount rather than a figure we would get WRONG — and say so rather than silently converting them.

## FR-207 — a justification the tree now contradicts

`f1040s3.map.toml:96` and `:156` justify Form 8880 (retirement savings contributions credit) as unmodeled
because *"no retirement contributions collected"*. **Box 12 code D collects exactly that, and FR-197 admits
D.** The conclusion may still be right — Form 8880 has its own AGI limits and a separate eligibility test —
but the stated reason is false. **Restate the reason or change the verdict; do not leave a justification the
tree contradicts.**

## Rules

**FOREGROUND every command** (FR-175). `make check` (~20s) excludes `cargo fmt --all --check` — run both. Capture
once, grep. **Do not commit or push.** If you can disprove any premise above, **stop and report it** — seven
briefs in this arc were refuted by their implementer, four of them mine.
Deliverable: Bash heredoc (not `Write`) → `design/agent-reports/REPORT-wave1-A.md`.
