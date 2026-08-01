# §G-11 — BRAINSTORM: the emitter cannot express "no testimony"

**Written 2026-07-31**, from `design/no-testimony/MAP-survey.md` (5 parallel readers over all 16
printed structs + Forms 6251/8959/8960/8995, classifying 221 rows against the extracted form text).

**Status:** brainstorm. No spec, no plan, nothing built. Several load-bearing decisions are the
owner's and are marked ⬜.

---

## 1. ★★★ The survey overturned the follow-up's own sketch

§G-11 closes with a *"sketch, not a plan"*: **"the emitter's money type grows a 'not stated' state
that survives to the AcroForm write."** That is the natural reading of the defect as filed — the
evidence quoted is `fmt_money(d: Usd) -> String`, which is genuinely the whole money path.

**It is the wrong primary move, and the survey establishes this by counting.**

> **62 of the 64 fabrication sites are manufactured BEFORE the emitter.** Growing `fmt_money` and the
> printed types to `Option<Usd>` would fix **two** (Schedule D lines 18/19, both hardcoded
> `let lineN = Usd::ZERO`).

By the time bytes reach `btctax-forms`, `printed.rs` has already built `Form1040Lines` out of concrete
`Usd` values. **The information was destroyed upstream.** An emitter that can express blank has nothing
left to express it *about*.

All five readers reached this independently; three volunteered it unprompted as the more important
half. Where the coercion actually happens:

| layer | sites |
|---|---|
| `return_inputs.rs` (a `#[serde(default)] Usd` nobody set) | 14 |
| `return_1040.rs` (computation over an absent thing) | 20 |
| `printed.rs` / `other_taxes.rs` / `qbi.rs` (the constructors) | 26 |
| `fold.rs` | 2 |
| **`btctax-forms` (the emitter)** | **2** |

**Consequence for the work:** this is a *type-and-provenance* problem in `btctax-core`, not a
formatting problem in `btctax-forms`. The emitter change is real but small and comes **last**.

## 2. The number, and its honest band

**64 fabrication sites out of 168 money quantities that actually reach a PDF — 38%.**

Denominator derived, not assumed: 221 raw rows − 11 duplicates (`Form1040Income` is destructured into
`Form1040Lines`) − 42 never emitted (41 Form 6251 lines, `qdcgt_net_capital_gain`) = 168.

Two counting calls move it, and both are open questions below: strict upstream-attribution of pure
carries → **55**; treating a reconciled ledger as the filer's testimony → **−4**. **Defensible band:
51–64.** The band does not change the verdict — this is a program, not a targeted fix — so it does not
need resolving before a spec.

Worst-concentrated: 1040 **19/34** · Schedule 3 **5/5** · Schedule D **11/15** · Schedule A **8/18** ·
Schedule 1 **6/10**. Every one of `Form1040Lines`' 34 money fields is written on every export.

## 3. Seven mechanisms — and the fix addresses these, not the forms

The forms are just where the kinds show up. Sorted by count:

| kind | n | shape |
|---|---|---|
| Σ over an empty `Vec` | 20 | `w2s.iter().map(…).sum()` with no W-2s |
| an `Option` that already means "absent", flattened | 15 | the information EXISTS and is thrown away |
| bare `#[serde(default)] Usd` | 14 | a TOML omitting the key parses to `0` |
| derivative total over a structurally-blank column | 9 | the total of nothing is `0` |
| hardcoded `let lineN = Usd::ZERO` | 3 | incl. 1040 L19 CTC, L28 |
| emitter literal | 2 | the only two `fmt_money` would fix |
| closed input surface | 1 | no field exists to answer with |

★ **The 15 flattened `Option`s are the cheapest and most damning.** btctax already *knows* the value is
absent and discards that knowledge on the way to the page. Any fix should take these first: they need
no new collection, no new question, no input-surface change.

## 4. The constraint: 24 lines where a zero is MANDATORY

The form is the authority in both directions. 24 instructed zeros were collected verbatim (1040 L15/22,
Schedule A ×4, Schedule SE 9/10, 8959 ×5, 8960 ×2, 8995 ×5, 6251 ×7). On these, `-0-` is what the form
*tells* the filer to write, and suppressing it is the mirror defect.

★★ **Schedules 1, 2, 3 and C contain no `-0-` instruction at all** — so all 19 fabrication sites on
those forms sit on lines the form is silent about. That is the cleanest subset in the whole survey.

Four counter-signals worth carrying into the spec:
- Form 8959's masthead: *"If any line does not apply to you, leave it blank."*
- Form 6251 L20/L34 route on *"zero **or blank**"* — so Schedule D 18/19's hardcoded `0` buys nothing.
- 1040 L7's remedy for "no capital gains" is a **checkbox**, not a zero. (Already adjudicated: §G-18.)

## 5. Precedent — extend what works, do not invent

- **`Option<T>` to the writer, for checkboxes.** Three instances, all built this week: Schedule B
  7a/FBAR/8 (`filter_map`), Schedule C I/J (`if let (Some, Some)`), 8283 5a–5c. **Each carries a comment
  saying `unwrap_or(false)` there was a live fabricated-testimony bug.** This is the mature pattern and
  it is the same defect, one type over.
- **One money leaf is already `Option<Usd>` end-to-end** — `Form8283Row.claimed_deduction`. Existence
  proof that the whole chain can carry it.
- **One line-level gate exists** — `form8995.rs:255`, `if ptr::eq(*cell, &map.line3) && value.is_zero()`.
  ★ Its comment claims *"line 3 is the only line on this form that is neither derived nor computed"* —
  **falsified by line 7 in the next loop iteration**, which is the REIT/PTP carryforward IN, equally
  transcribed. Fix that regardless of what else is decided.
- **Row/part suppression** (`schedule_d.rs::active`, `fill8949.rs`) is value-keyed, per-PART not
  per-cell, and **absent from the full-return emitter entirely**.
- **`CarryProvenance` + `advisories.rs:621-633` already computes the exact predicate** a blank-capable
  line needs — and `classifier.rs` marks it "no print".

## 6. ★★ The instrument built for this class is blind to it

`classifier.rs:17`, verbatim: *"`_` is **PERMITTED** on other scalar leaves (`String`, `Usd`, `Date`,
`Option<Date>`, `Option<String>`)."*

The answered-ness census — the thing that makes "we forgot this" fail to compile for `Option<bool>` —
**exempts every money leaf by design.** So none of the 64 is visible to the one instrument that exists
to catch exactly this. And `coverage.rs` asserts exempt leaves *exist* but never that their printed
lines *render blank* — a B1-shaped gap: a checker that cannot red on the defect it exists to catch.

This is probably the highest-leverage single change in the whole program, and it is also the most
expensive: extending the `_`-ban to `Usd` forces a human classification on ~200 leaves and `E0063`s
every omission. That is the *point* — an omission that does not compile cannot ship — but it is a real
cost and belongs in the spec's phasing, not smuggled in.

## 7. Recommended shape — ⚠️ SUPERSEDED

> **The architect consult ([`CONSULT-architect-fable.md`](./CONSULT-architect-fable.md), same day)
> replaced this phasing.** Two of its four orderings were wrong: the emitter change belongs **FIRST,
> not last** — no blank can exist anywhere until the writer can express one — and §10 below already
> conceded gate-first, contradicting this section. The accepted architecture is two types
> (`Collected<Usd>` at the input boundary, `LineEntry` at the printed layer) with construction only
> through combinators keyed to the form's own instruction grammar. Kept unedited for the record.

### (superseded) my read — not decided

Four phases, ordered so each is independently green and the cheapest evidence comes first:

- **P1 — stop discarding what we know.** The 15 flattened `Option`s. No new questions, no input-surface
  change, no collection. Pure plumbing: keep the `Option` to the printed struct. Smallest possible
  change that produces a real blank on a real page — and therefore the best place to build the B1
  kill-test before the surface is large.
- **P2 — the silent forms.** Schedules 1, 2, 3, C: 19 sites, zero `-0-` instructions to conflict with.
- **P3 — make omission fail to compile.** Extend the classifier's `_`-ban to `Usd`, with the census as
  the permanent gate.
- **P4 — the emitter.** `fmt_money`/`push_money` learn blank; the 2 remaining literals; the falsified
  8995 comment; `-0-` vs `0` rendering.

**Not in scope, per §G-11's own hard boundary:** any heuristic that flags an omission as suspicious, or
any feature that opines on whether a blank is lawful. Both directions are software adjudicating intent.

## 8. ⬜ OWNER DECISIONS — these change the work, and I should not make them

1. **⬜ Does a reconciled ledger constitute the filer's testimony?** Decides Schedule C line 1,
   Schedule D 3/10, 1040 line 7 — and whether the crypto slice's blank or the full return's `0` on 1040
   7a is the bug. This is a judgment about the filer's relationship to their own ledger, not a code
   question.
2. **⬜ Blank, or refuse, where silence ASSERTS?** For class-(A) lines — 1040 25a withholding,
   Schedule SE 8a Medicare wages, both document-matched by the Service against copies it already holds
   — is a blank sufficient, or must the return refuse? Answering "always blank" makes the fix mechanical
   and may be wrong.
3. **⬜ Is "supplied-then-zeroed" in scope?** Schedule A 8a mixed-use and the §170(b) ceilings print `0`
   over facts the filer *did* state. Not fabrications by this survey's test — but they are affirmative
   sworn zeros contradicting the filer's own testimony, and no leaf-level fix reaches them.
4. **⬜ Sequencing vs Form 6251 Tier 2.** 18 further sites wait there, including CLAUDE.md's canonical
   ISO/line-2i example. Fix-first costs a rebase; Tier-2-first ships the canonical defect on purpose.

## 9. Open design questions I intend to decide in the SPEC

Value-vs-line for the not-stated state (both precedents exist, one line each) · what a total does with a
not-stated operand (Schedule 1 line 9: 20 of 21 operands have no field) · serde-requiredness vs `Option`
(both already in-repo doing this job; requiredness fails loud and breaks every existing TOML) ·
`ReturnInputs` shape change vs a provenance sibling · whether the input-form seam grows
`Money(Option<Usd>)` · `"0"` vs `"-0-"` · Schedule SE 8d/9/10, the mirror defect.

## 10. ★ The B1 kill-test is not obvious, and it decides whether any of this is real

**Every fix here is a *suppression*, and a suppression that over-fires is invisible** — a missing cell
looks exactly like the common correct case, which is a blank line on a mostly-blank return. Neither
oracle sees it (both take values as input). No golden moves (~62 of 64 overstate tax, and the goldens
encode current behaviour).

So the negative test must plant **both** defects at once: a zero that should not print, **and** a blank
on one of the 24 instructed zeros. Until there is a single instrument that reds on both, this program
has no gate — and by B1 it does not exist. **This is the first thing the spec must answer, not the
last.**
