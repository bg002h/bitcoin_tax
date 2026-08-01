# SPEC — §G-11: make "no testimony" representable

**Revision r2** (2026-07-31), folding `reviews/SPEC-doctrine-opus-r1.md` (2C/3I) and
`reviews/SPEC-engineering-opus-r1.md` (0C/4I). **Status: pending re-review.**
**Sources:** `MAP-survey.md` (evidence), `CONSULT-architect-fable.md` (architecture, accepted).

> **r1 → r2 in one line:** the grammar was undercooked (4 productions for ≥7 instruction classes) and
> keyed on the wrong discriminator (the *verb*, not the *floor*); the scope named one file when four
> reach paper; the containment was a grep when the compiler can do it; and the gate never tested the
> distinction the type exists to draw. All four are fixed below.

---

## 1. The problem, and the scope boundary

A blank line and a printed `0` are different speech acts on a return signed under 26 USC §6065. A blank
says nothing; a `0` swears the amount **is** zero. btctax has no representation for "not stated" in its
money path, so **64 of the 168 money quantities that reach a PDF can print a zero the filer was never
asked for.**

btctax has three lawful moves over the *filer's* facts: **collect**, **refuse**, or leave **genuinely
blank**. It must never silently choose silence and present it as the filer's.

★ **A fourth move exists and r1 failed to name it** (doctrine M-1). btctax also **asserts its own
arithmetic** over lines already on the page — *"Subtract line 14 from line 11"*. That is not the filer's
testimony being invented; it is btctax showing its work, and the form demands it. Naming it makes the
admission rule explicit: **a line may assert btctax's arithmetic only when every operand it reads is
itself on the page.** Unnamed, it became r1's residual bucket, which is how Critical #2 happened.

**★ HARD SCOPE BOUNDARY (§G-11, non-negotiable).** Nothing here may flag an omission as suspicious,
score it, or opine on whether a blank is lawful. Both directions are software adjudicating intent.
The architecture preserves *who said what*; it never evaluates it. *(Both r1 lenses checked this and
found no crossing; keep it that way.)*

**Non-goals.** Tax-figure correctness. Supplied-then-zeroed lines (§170(b) ceilings, Schedule A 8a).
Unasked *elections* as distinct from unasked *amounts*.

## 2. The two types

### 2.1 `Collected<T>` — the input boundary (`return_inputs.rs`)

```rust
pub enum Collected<T> { Stated(T), NotAsked }
```

`#[serde(default)]` ⇒ `NotAsked`. **No `Default` producing a value; no `unwrap_or*`.** Sole exit:
`fn stated(&self) -> Option<T>`.

**Why not bare `Option<Usd>`** — the repo already refuted it at `return_inputs.rs:631`: *"`Option<Usd>`
is a scalar the `_` rule permits, which would make this convention again."* Plus `unwrap_or(Usd::ZERO)`
is a one-token re-fabrication on every `Option`.

★★ **`Stated(Usd::ZERO)` is a first-class, load-bearing state.** A filer who says "my withholding was
zero" has testified. Collapsing it into `Blank` deletes testimony — the mirror of the defect. §5.1's
third plant exists solely to make that failure red.

### 2.2 `LineEntry` — the printed layer, **OPAQUE**

```rust
pub struct LineEntry(Repr);          // public
enum Repr { Entered(Usd), Blank }    // PRIVATE to the module
```

★★★ **Opaque, not a public enum** (engineering I-3). A `match` on `LineEntry` outside the defining
module **does not compile** — which is the only mechanism that closes the hole a grep cannot: an inline
`match e { Blank => Usd::ZERO, .. }` in production code. It costs nothing, because §3 already forbids
free construction. Published exits are chosen deliberately: `is_blank()`, `fmt()` for the emitter, and
an ordering comparator for the three emitter sites that compare (`schedule_se_full.rs:78`,
`form8995.rs:166`, `schedule_d_full.rs:61`).

**No `From<Usd>`, no `Add`, no `Sum`, no `Default`.** ★ **And `#[non_exhaustive]` is explicitly
rejected** — it forces a `_` arm on downstream matches, making `_ => Usd::ZERO` *easier*. It would make
this worse.

**Scope — four files, not one** (engineering I-1). Every money field on every struct that reaches paper:
`printed.rs` (16 structs), `other_taxes.rs` (`Form8959Lines`, `Form8960Lines`), `qbi.rs`
(`Form8995Lines`), `form6251.rs` (`Form6251`), and `forms.rs` (`ScheduleDPart`, `ScheduleDTotals` — the
crypto slice, which never calls `push_money` at all). **r1 scoped `printed.rs` alone, which silently
excluded Form 6251 — its own headline example.**

★ `Printed8949Totals` (`printed.rs:53`) carries the layer's only `#[derive(Default)]`. **Delete the
derive**; do not add `impl Default for LineEntry`. Even a `Blank`-yielding `Default` reopens the
implicit-construction door.

## 3. The combinator grammar — keyed to the FLOOR, not the verb

`LineEntry` is constructible **only** through these. r1 had four productions; the doctrine lens found
≥25 fields fitting none, against r1's own stated threshold of five.

| # | constructor | the form says | blank when |
|---|---|---|---|
| 1 | `collected(Collected<Usd>)` | *(the filer supplies it)* | `NotAsked` — **never** for `Stated(0)` |
| 2 | `carry(LineEntry)` | *"Enter the amount from line N"* | the source is blank |
| 3 | `combine(&[LineEntry])` | *"Add / Combine lines X–Y"*, **no floor clause** | **iff every operand is blank** |
| 4 | `floored(Usd)` | any arithmetic **+ *"if zero or less, enter -0-"*** | **never** |
| 5 | `scaled(LineEntry, Rate)` | *"Multiply line N by 7.5% (0.075)"* | the source is blank |
| 6 | `bounded(Bound, &[LineEntry])` | *"Enter the smaller/larger of …"* | all inputs blank |
| 7 | `constant(Usd)` | *"Threshold based on filing status"* | never |
| 8 | `conditional(Cond, LineEntry)` | *"If line 33 is more than line 24, …"* | the condition is unmet |

★★★ **Production 4's trigger is the FLOOR CLAUSE, not the verb.** r1's table keyed `computed` on
*"Subtract…"/"Combine…"* while its own next paragraph said the discriminator was the floor. Because the
table is what an implementer transcribes from, **17 fields that say "Combine" with no `-0-` clause
would have been forced to print a sworn zero** — including Schedule 1 line 10 (*the very line §5.1's
first plant simulates*) and Schedule D line 7, on the one form btctax always emits. Those are now
production 3, and blank when every operand is blank.

★★ **Production 3's `iff` has a second exit: SKIP** (doctrine I-1). Schedule SE line 8d is *"Add lines
8a, 8b, and 8c"* but sits inside *"If $168,600 or more, skip lines 8b through 10"* — the form wants 8d
**blank while 8a is entered**. `combine` alone gives `Entered`. So production 3 takes an optional
skip predicate, and the same machinery covers Schedule D's *"skip lines 17 through 20"* and Schedule SE
4c's *"if less than $400, stop"*. `schedule_se_full.rs:80-92` already ships a per-line include flag —
extend it, do not invent beside it.

★ **Production 3's trigger is a phrase family, not one phrase** (doctrine M-2): *"Add the amounts in the
far right column for lines 4 through 16"* (Schedule A 17) lists a **range whose operands are 4, 7, 10,
14, 15, 16 only** — transcribing the range literally double-counts 5a–5d. Operands are transcribed from
the form's *column*, never inferred from the range.

**P0 exit criterion — the coverage check is DONE, not deferred.** Every money field on every struct in
§2.2's scope is assigned a production, in a committed table, with the instruction quoted verbatim.
**r1 deferred this to P0 and shipped a grammar that failed it by 5–8×.** ★ If any field fits none, that
is a finding requiring a documented exception; **five or more and the grammar returns to review.**

## 4. What the compiler enforces, and what it cannot

**Compiler:** retyping the fields emits **E0308** at every struct literal — r1 cited E0063, which is the
*adding-a-field* mechanism and is not what happens here (engineering M-1). Measured: **87 struct-literal
sites** across the workspace (46 in `printed.rs`, the rest in tests) and **13 constructor `fn`s**, not
16. Money-write sites: **44** (31 `push_money`, 3 `push_literal`, 10 direct `fmt_money`).

★ **Verified clean** (engineering): no `serde` derive on any printed struct, no `impl From`, no builder,
no struct-update syntax. Retyping really does error every literal — once §2.2's `Default` derive is gone.

**Compiler cannot:** verify the *chosen production matches the instruction*. That residue is the gate's.

## 5. The gate — the disposition KAT

**Every change here is a suppression, and a suppression that over-fires is invisible.** Neither oracle
sees it; no golden moves.

`transcribe::extract_lines(pdf, map_toml) -> BTreeMap<String, String>` inserts a cell only when the field
exists **and** its text is non-empty — so **an absent key is a blank cell on paper.** Verified, and
already asserted by `tests/extract_lines.rs`.

New KAT in `crates/btctax-forms/tests/`. **It must never use `Blank::AbsentIsZero`.**

1. **Line set from the committed map TOMLs**, via the predicate `walk` already uses — *keep a string
   leaf only if it is a real field in `collect_fields`*. This drops `[census]`/`[identity]`/`form`/`year`
   metadata with **no skip-list** (engineering M-3; the naive enumerator needs one and would rot).
   ★ Note: **one line is not one key** — `MoneyPair` lines transcribe as `.dollars_field`/`.cents_field`,
   Schedule D rows as `.proceeds_d`/`.cost_e`/`.adj_g`/`.gain_h`.
2. **The instructed-zero set is COMPUTED**, by grepping **the form files only** —
   `design/forms/extract/f*--YYYY.txt`. ★ **Not `*.txt`** (doctrine I-3): that sweeps the ~1.3 MB
   `i*` instruction files, where `-0-` appears in conditional prose — `i1040gi:23500` would enroll 1040
   35a/36 on a branch almost no return takes. Verified: the restricted glob returns non-zero on exactly
   the forms that should have it and zero on those that should not.
   ★★ **Schedule 1 has no extract at all**, so it is invisible to both this and §3's check while btctax
   emits it. **P0 must add `f1040s1--2024.txt`** — otherwise a future Schedule 1 `-0-` can never join,
   which is the stale-list failure this design exists to avoid.
   ★ Membership is *not* decided by the grep alone where the clause is itself conditional (Schedule SE
   4c, 5b). The grep produces the **candidate** set; each candidate is adjudicated once, against the
   instructions, and recorded with its reason.
3. **Three fixtures, one instrument.** Expectations are written **from each fixture's INPUTS**, never
   from the printed struct — that independence defeats consistent-but-wrong.

### 5.1 B1 — three plants, all watched red before P0 merges

| # | plant | simulates | must red |
|---|---|---|---|
| 1 | reintroduce `sch_1.map_or(Usd::ZERO, …)` (`printed.rs:591`) | a zero that should not print | **absence** vector |
| 2 | make the writer skip every zero value | over-firing suppression | **instructed-zero** vector |
| 3 | `collected` as `match x { Some(v) if !v.is_zero() => Entered(v), _ => Blank }` | **a filer-STATED zero deleted** | **stated-zero** vector |

★★★ **Plant 3 is new and r1 had no defence against it** (engineering I-4). It compiles, is a plausible
thing to write, silently deletes genuine testimony — and **passed both r1 plants**, because the absence
vector states nothing and the instructed-zero vector's zeros arrive via production 4, not `collected`.
§2.1's entire case for `Collected<T>` is that `Stated(0)` ≠ `NotAsked`, and r1's gate never tested it.
The third fixture states `$0` withholding / `$0` estimated payments and asserts `Blank::PresentZero`
(`tests/common/mod.rs:201`, which already exists and panics on an absent key) at 1040 L25a/L26.

**Stated limits.** The KAT pins fixtures, not all `-0-` branches; a completeness census is P4. An author
who writes both a fabrication and an expectation blessing it passes — the gate forces it to be written
down **twice, in review-visible places**, the same strength the classifier has.

★ **Audit the one existing `AbsentIsZero` use** (engineering M-4): `golden_packet.rs:321` reads 1040 L13
that way — the QBI line, which P1 flips to absent. Left alone it stays green through P1/P2, silencing
the one golden that could have spoken.

## 6. Containment — the compiler, then the lint, then the grep

r1 proposed a test-support trait plus a grep. **Insufficient**: the grep bans the honest accessor
exactly where production needs it while missing the inline `match`, pushing authors toward the
un-greppable form. And it scoped "the ten published crates", exempting `btctax-oracle-harness`
(`publish = false`) — the one crate r1 said needed the accessor.

**Corrected, in strength order:**

1. **Opacity (§2.2) is the primary mechanism.** Outside the module there is nothing to `match`.
2. **`clippy.toml` `disallowed-methods`** on whatever numeric exit is published. `make check` already
   runs `clippy --workspace --all-targets -- -D warnings`, so this is a hard build failure overridable
   only by a visible `#[allow]`. The repo has no `clippy.toml` today; add one.
3. **The hygiene grep** as a backstop only, ships with its own planted-defect kill.

★ **The real consumer inventory** (engineering I-3), which r1 got wrong: `btctax-cli/src/render.rs`
does ~23 printed-field reads **in a published crate**; three emitter sites compare; the oracle harness
does arithmetic at `main.rs:324`. These need a **published, deliberate** numeric exit — not a
test-support back door. ★ And a `test-support` feature **cannot** serve the harness: it depends on core
normally, so feature unification would re-expose the accessor everywhere. The harness writes explicit
matches inside the defining crate, or uses the published exit.

## 7. Phasing

**P0 — vocabulary, scope, gate. ZERO BEHAVIOUR CHANGE ON PAPER.**
Define the types (opaque). Migrate the writer signature; **every call site passes an entered value**.
Fix the two Schedule D emitter literals. Delete the `Printed8949Totals` `Default` derive. Add
`f1040s1--2024.txt`. **Complete the §3 coverage table over all four scoped files.** Build the KAT with
every current disposition pinned as entered, **all three plants watched red**, `clippy.toml`, and the
hygiene gate with its kill. Audit `golden_packet.rs:321`.
**Green =** suite green, goldens byte-unchanged (md5 `c4e1853…`), three kills demonstrated.
★ Traced sound by the engineering lens: the writer is a pure formatter, so byte-identity holds; and
suppression is **geometrically safe** because `verify.rs:390-414` requires strict descent, not
contiguity.

**P0b — the uniform retyping** (engineering I-2). Retype **every** money field in §2.2's scope to
`LineEntry`, all constructed via §3. r1's phases named ~60 of 168 and never scheduled the rest, leaving
the mixed patchwork §2.2 says produced the 64 — and making §4's guarantee false for every field still
`Usd`. Byte-unchanged on paper (all entered), so it meets the same bar as P0.

**P1 — stop discarding what we know.** The 15 flattened `Option`s + 8995 lines 3/7 (delete the
`ptr::eq` gate — its *"only line on this form"* comment is falsified by line 7 one iteration later).
Flip those rows to absent. No owner decision needed.

**P2 — manufactured inputs and the silent forms.** `Collected<Usd>` over the 14 `#[serde(default)] Usd`
scalars; the 20 empty-`Vec` sums and 9 blank-column totals. Schedules 1/2/3/C first. 1040 25a/26 wait on
owner decision 2.
★ This is where an existing TOML omitting a key changes meaning from `0` to `NotAsked`. btctax has no
users and v0.15.0 is unpublished — **free now, expensive later.**

**P3 — make the 65th site fail to compile.** Extend the classifier's `_`-ban to money leaves.

**P4 — residue.** `"0"` vs `"-0-"`; the Schedule SE 8d/9/10 mirror defect (adjudicate against
`i1040sse`); the `-0-` completeness census; supplied-then-zeroed and unasked-elections as separate
classes.

## 8. Owner decisions as variation points

Unchanged by r2 — the architecture still does not depend on them. Each is one production choice plus one
expectation row: (1) is a reconciled ledger the filer's testimony? (2) blank or refuse where silence
asserts? (3) is supplied-then-zeroed in scope? (4) sequencing vs Form 6251 Tier 2.

## 9. Risks carried

- **The §3 coverage table is now a P0 deliverable, not a P0 aspiration.** It is the thing r1 got wrong.
- **The three-production→eight-production expansion may still be incomplete.** The doctrine lens counted
  ≈25 misfits against four productions; §3 now has eight, but the re-review must re-run that count.
- **Map coverage** is **already half-closed** by `verify.rs:229 no_unmapped_filled` — a cell written
  outside any map fails closed as `UnmappedField` (engineering M-5). Build the census on that invariant
  rather than beside it; a second writes-set would be a second truth.
- **1040 line 16** (*"Tax (see instructions)"*) has no verb at all — it comes from the Tax Table or the
  QDCGT worksheet. It is the strongest candidate for §3's documented exception.
