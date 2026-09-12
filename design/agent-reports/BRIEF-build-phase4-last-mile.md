# BRIEF — Phase 4's last two items: WHERE TO FILE, and record retention

**Owner approved 2026-09-11.** This closes the long-range plan's **Phase 4** exit gate:
*"a filer holding the packet can post it without consulting anything outside it."* **That gate is
currently unmet**, and this is the work that meets it.

You are ONE opus agent in the **shared main tree**. Nothing is committed while you work. Scope is two
items. Do not widen it.

---

## 0. What is already done — do not rebuild

Measured by the coordinator today:

| Phase 4 item | state |
|---|---|
| FR-49 — Form 4868 + 1040-V | ✅ CLOSED 2026-09-06. `btctax extension` (`cli.rs:271`), `export-irs-pdf --pay-by-check` (`cli.rs:244`), §7503 weekend + DC-holiday shifter |
| spouse IP PIN | ✅ done — T10, `return_inputs.rs:1122` |
| attachment order / staple / envelope assembly | ✅ done — `hand_marks` in `crates/btctax-cli/src/cmd/admin.rs`, including the 1040-V *"do not staple"* rule |
| `broker_reported_rows` hardcoded `0` | ✅ closed — a real field at `cmd/admin.rs:345`, fed from `main.rs:1092` |

## 1. Item (i) — WHERE TO FILE. The one that unblocks the gate.

**Measured gap:** `grep -rn "irs.gov" crates/` finds **no where-to-file or service-center reference
anywhere**. A filer holding a finished, signed packet does not know where to post it.

**The shape is already decided by the plan (§3 Phase 4's table) and you must not re-litigate it:**

> **BUILD as a link + the two facts**, not a bundled table that rots.

**The two facts** are that the correct address depends on (a) **the filer's state** and (b) **whether a
payment is enclosed**. Both must be stated, because a filer who knows only one of them will pick wrong.

★★ **Why a table is forbidden, and this is the load-bearing reason:** the IRS corrected the Form 1040-ES
addresses **mid-2026** (recon-efile §5). A bundled address table rots silently, and a rotted address means
a signed return mailed to the wrong service center — a real loss, with no error message. A link cannot rot
in that direction: at worst it 404s visibly.

**Where it belongs:** `hand_marks` is the text a filer reads *while assembling the envelope*
(`cmd/admin.rs:433`, *"the instructions a filer follows while assembling paper"*), so that is the natural
home. Consider whether the manifest and/or `LIMITATIONS.md` should also carry it — `LIMITATIONS.md` is
`include_str!`'d into the binary (`main.rs:582`) and is **shipped filer-facing text**, so a sentence there
is product surface, not a design note. You choose; say why.

★ The extension path needs its own answer: `btctax extension` mails Form 4868 **on its own**, not in the
packet (spec 4868/1040-V R2), and Form 4868's where-to-file address differs from the 1040's. If that is
true, the extension surface needs the same link-plus-facts treatment. **Verify it against the archived
form/instructions rather than assuming it.**

## 2. Item (ii) — record retention, made filer-facing

**Measured gap:** the *decision* exists in `design/forms/FIELD_PROVENANCE.md:265` (*"Never auto-shred:
destroying evidence of diligence must be an explicit…"*) and `:464` (*"retention should be the filer's
decision, not pick a window"*) — but it is **in a design document**, not in front of a filer. Nothing in
`LIMITATIONS.md` or the manifest mentions it.

★ **The nuance you must hold, because the two sources pull against each other.** The plan's table says the
window that matters is **holding period + 3 years**, because crypto basis traces back years and the generic
"3 years" is wrong for this filer. `FIELD_PROVENANCE.md:464` says the product must **not pick a window** —
it is the filer's decision. Both are right: **inform without prescribing.** Say why the generic three years
is the wrong instinct here and what the window depends on; do not emit a date, a deadline, or a default.
And do not build a retention *mechanism* — `export-snapshot` already produces the artifact unprompted.

## 3. ★★ The kills — B1, and the interesting one is the second

A filer-facing sentence is easy to "test" vacuously. Both of these must be seen red on a planted defect.

1. **The guidance is present and complete.** The assembly text a filer reads must name **both** facts and
   carry the link. Plant: delete one of the two facts → red. A test asserting only that some string appears
   is not enough; it must fail when the *payment-dependence* or the *state-dependence* goes missing,
   because a filer told only one of them mails to the wrong place.
2. ★★ **The "no bundled table" decision must be STRUCTURALLY held, not a convention.** Add a guard that
   reds if a service-center postal address ever enters the tree — e.g. a ZIP code or a
   *"Department of the Treasury / Internal Revenue Service, <city>"* block in any shipped string. That
   converts the plan's prose decision into something a future edit cannot quietly undo, which is
   `CLAUDE.md`'s *"derive the list, or make the compiler hold it"* applied to a policy rather than a list.
   Plant a real service-center address → red. **State the guard's blind spot in its own header.**
   ★ Watch for false positives before you ship it: the repo legitimately contains ZIP-like digit runs (SSNs
   from the never-issued space, EINs, dollar amounts, the `ALLOWED_EIN` list in
   `scripts/pii-scan-generic.sh`). A guard that reds on those is worse than none. Pin the near-misses the
   way `forge_reach_check.rs` does.

★ **No network calls.** CI runs a net-isolation job, so you cannot check the URL resolves. The link's
correctness is a transcription question: take it from the archived IRS instructions if they print it,
and **say in the report where the URL came from**. If no archived authority prints it, say that plainly
rather than inventing a plausible URL — a fabricated link is worse than a stated gap.

## 4. Severity and scope

Two items. Anything else goes to `FOLLOWUPS.md` with an owning phase. Do not bundle params, do not touch
the fail-closed gates, do not build a retention mechanism or an address table. A **blank** is the normal
case; assert provenance, never non-blankness. Secret-handling defects never gate — file them.

## 5. Mechanics

**Main tree. Do NOT commit, push, `git stash`, `git checkout` or revert anything** — a builder in this arc
ran `git stash` against its brief. Revert a mutation with a **cp backup**. **No subagents.** No
`--no-verify`. Never hand-count what a tool can count, and **never quote a number or a list from a
`head`/`tail` view** — the coordinator made that class of error four times today and asserted an unchecked
status three more; read the tool's complete output.

Baseline: `make gate` **3598 passed / 12 skipped**, fmt clean. `make docs` must show no diff (regenerate if
a man page moves — `LIMITATIONS.md`'s body is single-sourced into the **binary**, and its man page carries
only the clap summary). Report numbers as numbers.

**Stop and report rather than build on any premise here you can disprove.** Nine briefs in this arc have
been refuted by measurement, four of them the coordinator's.

## 6. Your report — final action

Write `design/agent-reports/REPORT-build-phase4-last-mile.md`: what you changed and where (`file:line`);
**where the URL came from**, or that no archived authority prints it; how you resolved §2's
inform-without-prescribing tension; the extension-path answer with its evidence; kills with pasted
red-then-green for both, and the near-misses your address guard pins; refuted premises; residue with owning
phases; the literal gate numbers. Return only a short summary plus the path.

★ If your harness refuses the report write (it happened to one agent today), say so first and return the
text — do not silently skip it.
