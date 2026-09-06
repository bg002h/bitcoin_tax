# scafffix — `Form6251Line1Rule::Y2024` hardcoded in a function with no year

**Agent:** scafffix / y2024hardcode · **Date:** 2026-09-05
**File owned and touched:** `crates/btctax-core/src/tax/return_1040.rs` (only)
**Port-report row:** `design/TY2026_PORT_REPORT.md` §2 row **15** / §5 **R1** (FALLS BACK, "now #1 on the build list")
**Status:** fixed, four tests land, every one watched RED on a planted defect.

---

## 1. What was wrong — measured, not described

`form6251_inputs_from_parts` (then at `:2476`) wrote, at `:2493`:

```rust
        // TY2024's Part I. TY2025 passes `Y2025 { .. }` here; the year lives at THIS call site,
        // which is the one place that knows it (D-4).
        line1_rule: crate::tax::form6251::Form6251Line1Rule::Y2024,
```

Four facts, each checked against the source rather than the comment:

1. **The comment is false and the signature is why.** The function's parameter list was
   `(ri, agi, taxable_income, deduction, qbi_deduction, itemized, schedule_a, qualified_dividends,
   net_ltcg, regular_tax, foreign_tax_credit)` — **no `year`, no `params`**. It could not "know the
   year". Its ONE caller, `assemble_absolute` (`:2277`), has `year: i32` as its fifth parameter and
   passes `&params.amt` two lines below the call.
2. **The two variants are different quantities, not two spellings of one.** `compute_6251`
   (`form6251.rs:485`) combines `line1.amount_entering_line4()` into line 4, and
   `amount_entering_line4` (`form6251.rs:74`) returns `line1` under `Y2024` and **`line1b`** under
   `Y2025`. Working the algebra through with a correct 1040 L14:
   - `Y2024`: line 1 = L15 if > 0, else L11 − L14  ⇒ in both branches, **AGI − L14**.
   - `Y2025`: 1a = L14 − Sch 1-A **L37**; 1b = L11b − 1a ⇒ **AGI − L14 + L37**.

   **The difference is exactly Schedule 1-A line 37, the enhanced senior deduction, which TY2025
   ADDS BACK for the AMT.** The hardcode therefore **UNDERSTATES AMTI** by the whole senior
   deduction on any TY2025+ return that claims one — the bad direction — and carries into line 5/6,
   the tentative minimum tax, Schedule 2 L2, and 1040 L17/L24.
3. **It also files the wrong FORM.** `Form6251Line1::Y2024 { line1 }` prints a `line 1`; the TY2025
   form has `1a` and `1b`. Even where the number coincides (any return with no senior), a TY2025
   filer signs a Part I the 2025 form does not have.
4. **Nothing tested it.** `grep -rn "line1_rule" crates/*/tests/` ⇒ **0 hits** (re-measured today).
   `Form6251Line1Rule::Y2025` was constructed in exactly **one** place in the whole workspace before
   this change — `form6251.rs:1038`, a unit test *inside* `form6251.rs`. **No production code path
   had ever built the TY2025 variant.**

### A second, same-line defect found while fixing it

`deduction_l14` was populated as `deduction + qbi_deduction`. From TY2025 the 1040's line 14 is a
**three**-term sum — *"Add lines 12e, 13a, and 13b"* — and `printed.rs:769` already sums all three
(`line12 + line13 + round_dollar(ar.schedule_1a_additional)`). So the value handed to Form 6251 under
the name `deduction_l14` was **short by 1040 line 13b (Schedule 1-A line 38)** for every year that has
a Schedule 1-A. It is dormant for TY2024 (no Schedule 1-A ⇒ `schedule_1a_additional` is structurally
`Usd::ZERO`), which is why the suite was green. Fixed in the same edit; it is a **provable no-op for
TY2024** and the reason the L14 argument is now `total_deductions` taken from `assemble_absolute`
rather than re-derived.

---

## 2. What I changed

All in `crates/btctax-core/src/tax/return_1040.rs`.

**(a) A year-keyed selector that REFUSES — `form6251_line1_rule` (`:2501`).**

```rust
fn form6251_line1_rule(
    year: i32,
    form_1040_l11b: Usd,
    form_1040_l14: Usd,
    schedule_1a: Option<&crate::tax::schedule_1a::Schedule1A>,
) -> Option<crate::tax::form6251::Form6251Line1Rule> {
    match year {
        2024 => Some(Form6251Line1Rule::Y2024),
        2025 => Some(Form6251Line1Rule::Y2025 {
            form_1040_l11b,
            form_1040_l14,
            schedule_1a_l37: schedule_1a.and_then(|s| s.part5.line37).unwrap_or(Usd::ZERO),
        }),
        _ => None,
    }
}
```

The arms are the years whose Form 6251 Part I has actually been read. **`_` refuses; it must never
become `_ => Y2024`.** Its doc comment records that TY2026 is expected to *reuse* the `Y2025` shape
(port report §7 D3 — Form 6251 does not renumber for TY2026; only the cited Schedule 1-A line moves,
37 → 43), so the `2026 =>` arm is a one-line edit **after** the final form is read and not before.

**(b) Threaded the year and the real L14 through `form6251_inputs_from_parts` (`:2539`).**
`year: i32` added; the `qbi_deduction` argument **replaced** by `total_deductions_l14`; a
`schedule_1a: Option<&Schedule1A>` argument added (Part I needs line 37, and line 37 only). Body:
`line1_rule: form6251_line1_rule(...)` and `deduction_l14: total_deductions_l14` (`:2574`).

**(c) The refusal reaches the caller as a panic naming the fix.** `assemble_absolute` is infallible
by construction (its own doc says so at `:2588`), so there is nowhere to return an `Err`. This is the
port report's REFUSES class in its third form ("`Err` / `None` / **a panic naming the fix**"), and it
is **unreachable in production today**: I checked all four production entries —
`btctax-cli/src/cmd/tax.rs:516`, `:664`, `:879` and `btctax-cli/src/cmd/admin.rs:992` — and every one
is gated on `FullReturnTables::full_return_for(year) -> Some`, which
`BundledFullReturnTables::load()` (`btctax-adapters/src/tax_tables.rs:99-101`) populates with **2024
alone**. The panic can therefore only fire the moment someone adds a year's `FullReturnParams`
*without* transcribing that year's Form 6251 Part I — precisely the moment worth stopping.

**(d) The call site (`:2277`)** now passes `year` (the function's own parameter, not `ri.tax_year` —
the rule `Schedule1A::compute` states at `:2065`), `total_deductions` (the same binding 1040 L15 is
figured from, not a second derivation), and `schedule_1a.as_ref()`.

**Nothing in `form6251.rs` needed to change.** The `Y2025` variant, its field names, its doc comments
and `amount_entering_line4` were already correct and already exhaustively matched; only the
*selection* was missing. No edit to report there.

---

## 3. Which test reds for which planted defect

Four tests added (`:10810`, `:10886`, `:10983`, `:11011`). Each plant applied to the fixed tree, run,
then reverted from a byte-identical backup (never `git checkout`).

| # | plant | reds |
|---|---|---|
| **1** | the shipped defect restored verbatim: `line1_rule: Form6251Line1Rule::Y2024,` (ignoring `year`) | **3 of 4** — `a_ty2025_return_carries_form_6251s_2025_part_i_not_2024s`, `the_2025_rule_adds_schedule_1a_line_37_back_into_amti`, `an_unenumerated_year_panics_rather_than_filing_last_years_part_i` |
| **2** | `_ => Some(Form6251Line1Rule::Y2024)` in the selector (the FALLS BACK shape) | **2 of 4** — `form6251_part_i_refuses_every_year_it_has_not_transcribed`, `an_unenumerated_year_panics_…` |
| **3** | `schedule_1a_l37: Usd::ZERO` (the add-back laundered) | **1** — `the_2025_rule_adds_schedule_1a_line_37_back_into_amti` |
| **4** | call site passes `deduction + qbi.deduction` instead of `total_deductions` (drops 1040 L13b) | **1** — `a_ty2025_return_carries_form_6251s_2025_part_i_not_2024s` |

Verbatim failure output:

```
plant 1: a TY2025 return must fill the TY2025 Part I (lines 1a/1b); got Y2024 { line1: 71400 }
         — this is the `Form6251Line1Rule::Y2024` hardcode, which files last year's Part I
plant 1: TY2025 must select the TY2025 line-1 rule
plant 2: assertion `left == right` failed: exactly the years whose Form 6251 Part I has been read …
           left: [1990, 1991, … 2024, 2025, 2026, … 2060]
          right: [2024, 2025]
plant 3: assertion `left == right` failed: Schedule 1-A line 37 / left: 0 / right: 6000
plant 4: assertion `left == right` failed: Form 6251 line 1a / left: 14600 / right: 18600
```

### What each test is for

- **`a_ty2025_return_carries_form_6251s_2025_part_i_not_2024s`** — the **shape kill, on the
  production path**: `assemble_absolute(…, 2025)` with a qualifying vehicle (1040 L13b = $4,000),
  asserting `ar.amt.line1` is `Y2025 { line1a: 18_600, line1b: 71_400 }`, **and** that TY2024 with
  the same inputs still fills `Y2024 { line1: 75_400 }` — so a "fix" that flipped every year to the
  2025 shape does not pass.
- **`the_2025_rule_adds_schedule_1a_line_37_back_into_amti`** — the **arithmetic kill**: a Single
  senior with MAGI exactly at the $75,000 line-32 threshold (the form's own jump ⇒ line 35 = $6,000
  ⇒ L37 = $6,000), asserting `correct.line4 − hardcoded.line4 == 6_000` and `correct.line4 >
  hardcoded.line4` — i.e. the hardcode understates AMTI. It is driven through
  `form6251_inputs_from_parts` **directly** because `assemble_absolute` still passes `false, false`
  for senior qualification (see §4), so line 37 is unreachable through it today and a test routed
  that way would assert `0 == 0` and survive every mutation. That is stated in the test's own doc.
- **`form6251_part_i_refuses_every_year_it_has_not_transcribed`** — the supported set is **read off
  the function** over a `1990..=2060` sweep and compared with `[2024, 2025]`; it is not a hand-list
  of negatives. A `_` fallback shows up as the whole sweep being "supported" (see plant 2's output).
- **`an_unenumerated_year_panics_rather_than_filing_last_years_part_i`** — `#[should_panic(expected =
  "never been transcribed for TY2026")]`.

### Suite state

`cargo nextest run -p btctax-core` → **1192 passed, 1 failed**. The single failure is
`tax::line_coverage::year_and_blank_tests::a_collector_that_never_named_its_year_is_refused`, in
`crates/btctax-core/src/tax/line_coverage.rs` — **another agent's file, mid-edit in the shared tree**
(its `should_panic(expected = "never named the year")` no longer matches the panic text that file now
emits; mtime 19:25:33, seconds before the run). Not mine, not touched by me. An earlier full run of
the same crate, before that file changed, was **1190/1190 green** with my change in place.
`cargo check -p btctax-core --lib --tests` is warning-free.

---

## 4. Found and NOT fixed

1. **`assemble_absolute` hardcodes `false, false` for senior qualification** (`:2073-2074`) — the
   port report's **R13**. This is a hardcoded answer to a question the filer was never asked, and it
   is what keeps the defect I just fixed *dormant in value*: with L37 structurally 0 there is no
   senior add-back for the Y2025 rule to make. **The two are the same story** — closing R13 arms my
   arithmetic test on the production path, and `born_early_enough(dob, year)` (`:90`) already exists
   and derives the 1961/1962 cutoffs. It is in my file but was not in my brief, and it needs the
   `dob` seam decision; I left it alone rather than widen scope. **Whoever closes R13 should re-run
   `the_2025_rule_adds_schedule_1a_line_37_back_into_amti` and consider re-routing it through
   `assemble_absolute`, where it becomes an end-to-end kill.**
2. **The selector still lives in a `match year` rather than on `FullReturnParams`.** The port
   report's §5 R1 end state is `SaltLimitation`'s: carry the rule on the per-year params bundle so
   the **compiler** forces the decision when a year is added and no `_` arm exists to get wrong. That
   edit is in `crates/btctax-core/src/tax/tables.rs`, which I do not own. My `match` is the
   scoped version: it refuses instead of falling back, and its doc comment names the end state so the
   next hand knows this is a waypoint. **Recommend this be filed as a follow-up owned by whoever
   holds `tables.rs`.**
3. **`schedule_2_line1z: Usd::ZERO` and `line3 = z`** remain hardcoded zeros in the same struct /
   `compute_6251`. Both carry written justifications ("no v1 input ⇒ 0", and the §3 declarations
   note). Not in scope; flagged only so a future reader knows they were seen and left.
4. **`Form6251Line1Rule::Y2025`'s field is named `schedule_1a_l37`**, and TY2026 cites Schedule 1-A
   line **43** for the same quantity (port report §7 D3). The ruling there is *reuse the variant,
   rename the field semantically*. That rename is in `form6251.rs`, not mine — but note that my
   selector reads `s.part5.line37` from `Schedule1A`, so the rename must reach **both** files or the
   TY2026 arm will look right and read the wrong Part V line.
