# SPEC — §G-11: make "no testimony" representable

**Status:** draft, pending independent review to 0C/0I.
**Supersedes:** `BRAINSTORM.md` §7 (phasing).
**Sources:** `MAP-survey.md` (evidence), `CONSULT-architect-fable.md` (architecture, accepted).

---

## 1. The problem, and the scope boundary

A blank line and a printed `0` are different speech acts on a return signed under 26 USC §6065. A blank
says nothing; a `0` swears the amount **is** zero. btctax has no representation for "not stated" in its
money path, so **64 of the 168 money quantities that reach a PDF can print a zero the filer was never
asked for.**

btctax has exactly three lawful moves per line: **collect** the testimony, **refuse** the return, or
leave it **genuinely blank**. It must never silently choose silence and present it as the filer's.

**★ HARD SCOPE BOUNDARY (§G-11, non-negotiable).** Nothing in this spec may flag an omission as
suspicious, score it, or opine on whether a blank is lawful. Both directions are software adjudicating
intent, which is out of scope. The architecture preserves *who said what*; it never evaluates it.

**Non-goals.** Tax-figure correctness (converged; not touched). Supplied-then-zeroed lines — §170(b)
ceilings, Schedule A 8a mixed-use — where the filer *did* state a fact and a limit reduced it to zero;
those are affirmative sworn zeros but not fabrications, and are filed separately. Unasked *elections*
(1040 35a refund-vs-apply) as distinct from unasked *amounts*.

## 2. The two types

Information is destroyed at exactly two boundaries. One type at each.

### 2.1 `Collected<Usd>` — the input boundary (`btctax-core/src/tax/return_inputs.rs`)

```rust
pub enum Collected<T> { Stated(T), NotAsked }
```

- `#[serde(default)]` yields `NotAsked`. A TOML omitting the key means *nobody answered*, not zero.
- **No `Default` that produces a value. No `unwrap_or`, `unwrap_or_default`, `unwrap_or_else`.**
- The only exit is `fn stated(&self) -> Option<T>`.

**Why not bare `Option<Usd>`.** The repo already refuted this, at `return_inputs.rs:631`: *"`Option<Usd>`
is a scalar the `_` rule permits, which would make this convention again."* Two independent reasons: the
answered-ness classifier permits bare `_` on any scalar including `Option<Usd>` (so it stays invisible to
the census), and `unwrap_or(Usd::ZERO)` is a one-token re-fabrication available on every `Option`. A type
with no such method makes the fabrication **un-writable without noticing**: you must `match` and type
`Usd::ZERO` yourself, which is greppable and review-loud.

### 2.2 `LineEntry` — the printed layer (`btctax-core/src/tax/printed.rs`)

```rust
pub enum LineEntry { Entered(Usd), Blank }
```

- **No `From<Usd>`, no `Add`, no `Sum`, no `Default`.** Arithmetic on a `LineEntry` must go through a
  combinator (§3), so "what happens to a blank operand" is never an accident.
- **Every money field on every printed struct becomes `LineEntry`, uniformly** — including lines that
  are always entered. A mixed `Usd`/`LineEntry` surface reintroduces the per-field judgment
  ("is this one safe as bare `Usd`?") that produced the 64 in the first place. Uniformity is also what
  makes Tier-2 Form 6251 fields land safe by default: `line2i` (the standing ISO example) cannot ship
  as a silent zero if it must choose a constructor.

**The value-vs-line question dissolves.** At the input boundary, not-stated is a property of the
collection event, so it rides the value. At the printed layer, the transcription doctrine makes the
field *be* the line (`Form1040Lines.line8` **is** 1040 line 8), so value and line coincide.

★ **`form8995.rs:255`'s `ptr::eq` line-identity gate is the named anti-pattern.** Its comment claims
*"line 3 is the only line on this form that is neither derived nor computed"* — falsified by line 7 one
loop iteration later. A per-line suppression table keyed by name reproduces that staleness 168 times.
Delete it in P1; lines 3 and 7 become `collected`.

## 3. The combinator grammar — the form's own instructions

`LineEntry` is constructible **only** through these. Each maps to a production in the forms' instruction
language, so choosing one is a **transcription fact checkable against `design/forms/extract/`**, not a
policy argument.

| constructor | the form says | blank when |
|---|---|---|
| `collected(Option<Usd>)` | *(a line the filer supplies)* | the source is `NotAsked` |
| `carry(LineEntry)` | *"Enter the amount from line N"* | the source line is `Blank` |
| `total_of(&[LineEntry])` | *"Add lines X through Y"* | **iff every operand is `Blank`** |
| `computed(Usd)` | *"Subtract…"* / *"Combine…"* **+ "if zero or less, enter -0-"** | **never** — always `Entered` |

**All 24 form-instructed zeros live in `computed`.** Form 8995 line 4 — *"Combine lines 2 and 3. If zero
or less, enter -0-"* — shows that an add-shaped line with a floor belongs here, which is why the grammar
has these productions and not two.

`total_of`'s rule is the human one: a person adding a column of blanks leaves the total blank. This is
already btctax's stated doctrine on Schedule 3 (*"never a misleading 0"* on the parts) — which its
`line8 = line1` total violates today.

**★ A line whose instruction fits no production is a finding, not a fifth constructor.** One such line
is tolerable and gets a documented exception; five would mean the grammar is the wrong abstraction and
the spec returns to review. The consult checked the productions against a sample, not all 16 forms —
P0 must complete that check and record the result.

## 4. What the compiler enforces, and what it cannot

**Compiler (free, exact):** retyping the fields `E0063`s all 16 printed-struct constructors and every
`push_money` call site. The 64-site review *becomes* compile errors. A 65th site cannot choose a silent
zero, because no such variant exists and no `From<Usd>` supplies one.

**Compiler cannot:** verify that the *chosen constructor matches the form's instruction*. A line that
should be `total_of` but is written `computed(sum)` compiles and prints a zero. That residue is exactly
what the gate exists for.

## 5. The gate — the disposition KAT

**★ Every fix in this program is a suppression, and a suppression that over-fires is invisible** — a
missing cell looks exactly like the common correct case on a mostly-blank return. Neither oracle sees it
(both take these values as *input*). No golden moves (~62 of 64 overstate tax; the goldens encode
today's behaviour).

**The instrument already exists.** `transcribe::extract_lines(pdf, map_toml) -> BTreeMap<String, String>`
reads a filled PDF back keyed by logical line — **an absent key is a blank cell on paper.**

New KAT in `crates/btctax-forms/tests/`. It **must never use `Blank::AbsentIsZero`**, which the existing
readback harness uses to mean *"present-'0' or absent both mean $0"* — precisely the distinction this
program exists to draw.

1. **Line set enumerated from the committed map TOMLs**, never a range or hand-list. Every money line
   must be *accounted for*: asserted present-with-value, or asserted absent. **A line missing from a
   fixture's expectation table fails the KAT** — that is what distinguishes "encodes no decision" from
   "we forgot it".
2. **The instructed-zero set is COMPUTED from the mechanism**, not enumerated: grep
   `design/forms/extract/*.txt` for the `-0-` instruction. New form years auto-join; no excuse list to
   go stale. (Same rule as the oracle harness: *state the mechanism, let it decide, never enumerate the
   outcomes you happened to see.*)
3. **Two fixtures, one instrument.**
   - an **absence** vector — no Schedule 1, no W-2s, no carryovers — whose table asserts the
     fabricatable lines are **ABSENT** from the transcript;
   - an **instructed-zero** vector — deductions exceeding AGI, a QBI-loss year — whose table asserts
     `"0"` is **PRESENT** on 1040 L15/L22 and 8995 L16/L17.
   - **Expectations are written from the fixture's INPUTS, not from the printed struct.** That
     independence is what defeats consistent-but-wrong.

### 5.1 B1 — the two plants, both watched red before P0 merges

| plant | what it simulates | must red |
|---|---|---|
| reintroduce `sch_1.map_or(Usd::ZERO, …)` at `printed.rs:591` | a zero that should not print | the **absence** vector |
| make `push_money` skip every zero value | the naive "fix" — an over-firing suppression, the invisible one | the **instructed-zero** vector |

One instrument, both directions. **Buildable in P0 before any site is fixed**, because untouched lines'
expectations simply pin today's behaviour with each fabrication marked as such — which also makes the
burn-down a grep.

**Stated limits (from the consult, kept honest):** the KAT pins fixtures, not all 24 `-0-` branches at
once; a completeness census (*every extract-derived `-0-` line is exercised by some fixture*) is a P4
ambition. And an author who both writes `Entered(ZERO)` and an expectation blessing it passes — the gate
forces the fabrication to be written down **twice, in review-visible places**, which is the same strength
the classifier has.

## 6. ★★ The leak risk, and its containment

`golden_packet.rs` and `btctax-oracle-harness` do arithmetic on printed fields and will need
`entered_or_zero() -> Usd`. **That accessor is a re-fabrication footgun: if it reaches production code,
the type's guarantee leaks and this whole program is decorative.**

Containment, specified now rather than discovered later:

- It lives on an extension trait in a **test-support module**, not an inherent method — production code
  cannot call it without an obviously test-shaped import.
- A **repo-hygiene test greps for it** and fails if it appears under `crates/*/src/` for any of the ten
  published crates. Precedent exists: `crates/btctax-cli/tests/repo_hygiene.rs`.
- That hygiene test ships **with its own planted-defect kill** (add a call in `src/`, watch it red).
  Per B1, a checker never watched go red does not exist.

## 7. Phasing

Each phase is independently green: full suite, all five gates, TY2024 golden matrix md5 unchanged unless
the phase explicitly changes what prints.

**P0 — vocabulary + gate. ZERO BEHAVIOUR CHANGE ON PAPER.**
Define `LineEntry` and `Collected<T>`. Migrate `push_money`/`fill_*` signatures to `LineEntry`; **every
call site passes `Entered(x)`** — the *types* land, the *fields* do not migrate yet. Fix the two
Schedule D emitter literals. Build the disposition KAT with every current disposition pinned as
`Entered`, both B1 plants watched red, and the `entered_or_zero` hygiene gate with its kill. Complete
the grammar-coverage check over all 16 forms (§3).
**Green =** suite green, goldens byte-unchanged, both kills demonstrated.

**P1 — stop discarding what we know.** The 15 flattened `Option`s (`printed.rs:591/593/654/677/680/690/
866/867`, Schedule 2 L12, `fold.rs:1151`) plus 8995 lines 3/7 (`collected`; delete the `ptr::eq` gate).
Flip those expectation rows to `Absent`. No owner decision needed; no new collection.

**P2 — manufactured inputs and the silent forms.** `Collected<Usd>` over the 14 `#[serde(default)] Usd`
scalars; the 20 empty-`Vec` sums and 9 blank-column totals become `total_of`/`collected`. Schedules
1/2/3/C first — no `-0-` instructions to conflict with. 1040 25a/26 wait on owner decision 2.
★ Note: this is the phase where an existing TOML that omits a key changes meaning from `0` to
`NotAsked`. btctax has no users and v0.15.0 is unpublished, so this is a free change *now* and expensive
later — which is an argument for not deferring the program.

**P3 — make the 65th site fail to compile.** Extend the classifier's `_`-ban to money leaves. Cheaper
than `BRAINSTORM.md` §6 feared: after P2 the ~200-leaf classification is mostly *naming what the
migration already did*.

**P4 — residue.** `"0"` vs `"-0-"` rendering; the Schedule SE 8d/9/10 mirror defect (an instructed `-0-`
btctax suppresses — adjudicate against `i1040sse`, not the form text); the `-0-` completeness census;
supplied-then-zeroed and unasked-elections as separately-filed classes.

## 8. Owner decisions as variation points

The architecture does not depend on these. Each is one constructor choice plus one expectation-table row.

| # | decision | where it lands |
|---|---|---|
| 1 | Is a reconciled ledger the filer's testimony? | Schedule C L1, Schedule D 3/10, 1040 L7 flip between `computed` and `collected` |
| 2 | Blank, or refuse, where silence ASSERTS? | 1040 25a / Schedule SE 8a: the `NotAsked` arm either blanks or feeds `screen_absolute` |
| 3 | Is supplied-then-zeroed in scope? | outside the leaf types entirely; stays a separate class unless pulled in |
| 4 | Sequencing vs Form 6251 Tier 2 | collapses — Tier 2 inherits the safe type whenever it lands |

## 9. Risks carried, in the consult's own words

- **Map coverage is unverified.** The gate enumerates from the map TOMLs; a cell written outside any map
  (e.g. a crypto-slice `push_literal` path) is invisible to it. **P0 must add a writes-set census** and
  reconcile it against the map-derived line set.
- **Blast-radius mechanicalness is assumed, not built.** ~47 `push_money` sites and 16 constructors look
  mechanical; §6 is the mitigation for the one place it is not.
- **The three-production grammar was checked on a sample.** §3 makes completing that check a P0 exit
  criterion.
