# SPEC — §G-11: make "no testimony" representable

**Revision r3** (2026-08-01). Folds `reviews/SPEC-doctrine-opus-r2.md` (2C/3I) and
`reviews/SPEC-engineering-opus-r2.md` (0C/2I). **Status: pending re-review.**

> **r2 → r3 in one line: the grammar stopped being a proposal and became a passing test.** r2's
> blocking finding was that its misfit count exceeded its own threshold. That count is no longer
> estimated — `xtask line-coverage` computes it on every commit. **Measured: 9 exceptions over 179
> money lines across 14 forms.** r1 guessed ≈25, r2 guessed 10–29.

---

## 1. The problem, and the scope boundary

A blank line and a printed `0` are different speech acts on a return signed under 26 USC §6065. A
blank says nothing; a `0` swears the amount **is** zero. btctax has no representation for "not stated"
in its money path, so money quantities can print a zero the filer was never asked for.

btctax has three lawful moves over the *filer's* facts: **collect**, **refuse**, or leave **genuinely
blank** — plus a fourth over its own: **assert its own arithmetic** across lines already on the page.

★ **The fourth move's admission rule, corrected.** r2 wrote *"only when every operand it reads is
itself on the page"* and the doctrine lens showed it was both too strong (it condemns 1040 line 16 and
every worksheet carry the IRS mandates) and unenforceable. The rule that survives is narrower and is
now **checked**: a line may assert arithmetic only if it declares a production whose operands are
`LineEntry`s. `Clamped` is the exception and is discussed in §6.

**★ HARD SCOPE BOUNDARY (§G-11, non-negotiable).** Nothing here may flag an omission as suspicious,
score it, or opine on whether a blank is lawful. Both r2 lenses checked this and found no crossing.

## 2. ★★★ What is already BUILT, and what it measured

`xtask line-coverage` (`crates/xtask/src/line_coverage_check.rs`) + the table
(`crates/btctax-core/src/tax/line_coverage.rs`):

```
line-coverage OK: 179 money lines across 14 form(s), 9 exception(s) (ratchet 9),
                  10 unverifiable (ratchet 10)
```

**This is r2's Critical, discharged by execution rather than by argument.** Two review rounds spent
~250k tokens each estimating a number that is now in CI and cannot go stale.

| production | measured |
|---|---|
| `Combine` | 54 |
| `Collected` | 44 |
| `Carry` | 32 |
| `Clamped(FloorAtZero \| CeilAtZero)` | 16 |
| `Scaled` | 10 |
| `Bounded` | 4 |
| `Constant` | 3 |
| **`Exception`** | **9** |

★ **`Conditional` was DELETED**, exactly as r2's doctrine lens demanded — it was "the named leftover",
its trigger collided with `Clamped`'s, and on Schedule A line 4 it got the form backwards. The lines it
was invented for are now honest `Exception`s. **The production set shrank while coverage grew.**

### 2.1 The guarantees the build actually provides

- **A new money field does not compile.** Every printed struct is destructured with no `..` under
  `#![deny(unused_variables)]`, and each money value is *consumed* by the recording call. Verified by
  planted defect: deleting one `c.line()` gives `error: unused variable: line1`.
- **A new money-bearing TYPE does not compile-clean.** Rule (4b) reads the source, finds every
  `pub struct`/`pub enum` declaring a `Usd`, and demands a `cover_*` fn. It found eight on first run.
- **An exemption must be TRUE.** Rule (4c) checks the "not printed" claim against the emitter's own
  non-comment code. ★ Added because a B1 plant showed the exemption list was a loophole anyone could
  widen with a sentence.
- **Every quote is verbatim**, modulo whitespace — which is not cosmetic: `pdftotext -layout` wraps
  clauses mid-sentence, r2's WRAPPING false negative.
- **Clamp polarity is transcribed, never inferred**, against a **closed** idiom set; a `Clamped` row
  matching no idiom is an error, so the checker cannot bless a polarity it did not verify.

## 3. ★★★ The transcription found a LIVE DEFECT

**1040 line 37** — *"Subtract line 33 from line 24. This is the amount you owe."* No clamp clause, no
condition ⇒ `Combine`. `printed.rs` does `(line24 - line33).max(Usd::ZERO)`: **a hard zero on every
refund return.**

**1040 line 34** — *"If line 33 is more than line 24, subtract line 24 from line 33"* states a
condition and prints **no** `-0-`, so when it fails the line is **blank**. `printed.rs` does the mirror
`.max(Usd::ZERO)`: a hard zero on every owing return.

Same defect, opposite directions, on a mutually-exclusive pair, on the most-filed form there is. **The
absence of `-0-` on both lines is precisely what distinguishes "leave blank" from "enter zero"** — the
whole §G-11 thesis, printed on Form 1040.

★ Line 34 is an **`Exception`**, and why is instructive: it is a *conditional entry, not a clamp*.
`Combine` is blank iff its operands are blank — both are populated on an owing return. `Clamped`
requires an `-0-` the form does not give. Rule (3) rejects it **even though its clause matches a floor
idiom**; the checker's two halves disagreeing is what surfaced it.

**Not fixed in the coverage commits, deliberately** — that is a tax-figure change and belongs in its
own reviewed diff (P1).

## 4. The two types

### 4.1 `Collected<Usd>` — the input boundary

`enum { Stated(T), NotAsked }`; `#[serde(default)]` ⇒ `NotAsked`; **no `Default` producing a value, no
`unwrap_or*`**; sole exit `stated() -> Option<T>`. Not bare `Option<Usd>` — the repo already refuted
that at `return_inputs.rs:631`. ★★ `Stated(Usd::ZERO)` is load-bearing: a filer who says "my
withholding was zero" has testified, and collapsing it into `Blank` deletes testimony.

### 4.2 `LineEntry` — the printed layer, **OPAQUE**

```rust
pub struct LineEntry(Repr);          // public
enum Repr { Entered(Usd), Blank }    // PRIVATE
```

A `match` outside the module does not compile — the only mechanism that closes the inline
`match e { Blank => Usd::ZERO, .. }` hole a grep cannot see. **`#[non_exhaustive]` is rejected**: it
forces a `_` arm, making `_ => Usd::ZERO` *easier*.

**Scope is DERIVED, not listed.** r2 named files and got it wrong in both directions — the ledger
aggregate `ScheduleDTotals` was in, `form8275.rs::Part1Item::amount` was out. Rule (4b) now decides
membership from the source. ★ That was r2's own C-1 repeated one level up: an enumeration by outcome
observed rather than by mechanism.

### 4.3 The comparator — `Option<Ordering>`, and there are SIX sites

r1 proposed `Option<Ordering>`; **r2 dropped the `Option`**, and the engineering lens measured the
consequence: a derived `Ord` gives **`Blank > Entered(0) == true`**, decided silently by variant
declaration order. Restored, and the six sites are named because two of them route the return:

| site | decides |
|---|---|
| `render.rs:1567` `line34 > ZERO` | **REFUND vs AMOUNT OWED** |
| `render.rs:1541` `amt.line6 > ZERO` | whether the Form 6251 block prints |
| `form6251.rs:248` `line7 > line10` | whether Form 6251 is **attached** |
| `schedule_se_full.rs:78`, `form8995.rs:166`, `schedule_d_full.rs:61` | emitter branches |

Each needs a declared blank branch in the P0 coverage table. **A comparator answering `bool` for a
blank operand is a liveness predicate answering for the filer** — the class §4.2 exists to close.

## 5. The gate — the disposition KAT

`transcribe::extract_lines` returns only non-empty cells, so **an absent key is a blank cell on
paper**. The KAT **must never use `Blank::AbsentIsZero`**, which the existing readback harness uses to
mean "present-`0` or absent both mean $0" — the distinction this program exists to draw.

Line set from the committed map TOMLs via the `collect_fields` predicate (no skip-list). Expectations
written **from each fixture's inputs**, never from the printed struct.

**Three plants, all watched red before P0 merges:**

| # | plant | must red |
|---|---|---|
| 1 | reintroduce `sch_1.map_or(Usd::ZERO, …)` | **absence** vector |
| 2 | make the writer skip every zero | **instructed-zero** vector |
| 3 | `collected` treating `Stated(0)` as `Blank` | **stated-zero** vector |

★ Plant 3 exists because r2's gate never tested the distinction `Collected<T>` exists to draw.

## 6. Open, and honestly so

These survive r3 and the re-review should weigh them:

1. **`Clamped` takes a bare `Usd`**, so the one production whose blank rule is "never" cannot see its
   own operands — r2's doctrine I-2, unfixed. It is the residual bucket's last redoubt. Options: make
   it take operands, or accept that its 16 instances are individually transcribed and checked.
2. **The skip class has no production.** Schedule SE 8d is *"Add lines 8a, 8b, and 8c"* inside *"If
   $168,600 or more, skip lines 8b through 10"* — the form wants it **blank while 8a is entered**. The
   emitter already carries a per-line include flag (`schedule_se_full.rs:80-92`); the spec must say
   which is authoritative rather than asserting both.
3. **The instructed-zero grep has a DELEGATION false negative** — *"enter -0- here and on lines 7, 9,
   and 11"* leaves those lines with no `-0-` text of their own, and Schedule D line 16's clause
   instructs a zero **on 1040 line 7, a different form**. WRAPPING is fixed; delegation is not.
4. **10 unverifiable rows** — all of Schedule 1, which btctax emits with no committed text layer. An
   asset problem: one fetch, no network here.
5. **Rule (2) uses `contains`, not uniqueness.** Two Form 8959 lines quote a byte-identical sentence
   that occurs three times in the extract. Harmless today; the collision if quote-uniqueness is added.

## 7. Phasing

- **P0a — DONE.** The coverage table + checker + ratchets, 179 lines, 14 forms, 9 exceptions. Twelve
  rules, each answering a defect a review round or a plant actually found.
- **P0b — the types.** `LineEntry` (opaque), `Collected`, the writer signature with every call site
  passing entered, `clippy.toml` `disallowed-methods` on the numeric exit, the disposition KAT with
  all three plants. **Zero behaviour change on paper** — goldens byte-unchanged.
- **P0c — uniform retyping**, driven by the table: ★ each line's production already determines its
  constructor, so this is a transcription, not 179 judgment calls.
- **P1 — the live defect and the flattened `Option`s.** 1040 L34/L37 first: it moves a real figure and
  is the reason this program exists.
- **P2 — manufactured inputs and the silent forms.** ★ Where an existing TOML omitting a key changes
  meaning from `0` to `NotAsked` — free now, expensive after a user exists.
- **P3 — extend the classifier's `_`-ban to money leaves.** ★ Largely pre-empted: `line_coverage`
  already forbids `_` on money, which is the half `classifier.rs:17` explicitly permits.
- **P4 — residue.** `"0"` vs `"-0-"`; the Schedule SE mirror defect; the delegation false negative;
  Schedule 1's extract; nested-money follow-ups.

## 8. Owner decisions — unchanged, and still not blocking

(1) is a reconciled ledger the filer's testimony? (2) blank or refuse where silence asserts? (3) is
supplied-then-zeroed in scope? (4) sequencing vs Form 6251 Tier 2. Each remains one production choice
plus one expectation row.
