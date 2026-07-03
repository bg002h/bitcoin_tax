# R0 spec review — `SPEC_gemini_subsatoshi_round.md`

**Reviewer:** independent adversarial architect (did NOT author the spec).
**Artifact:** `design/SPEC_gemini_subsatoshi_round.md`
**Source anchor:** branch `fix/gemini-subsatoshi-round` @ `e245f7c` (main == `719e9fe`).
**Gate:** R0 design gate. Bar = 0 Critical / 0 Important. Critical/Important BLOCK implementation.

## Verdict: **0 Critical / 1 Important / 2 Minor / 2 Nit**

The core fix is sound and the arithmetic is correct. The blocking gap is **not** in the round math — it is
that the spec never accounts for the xlsx **numeric-cell read path** that the user's real file almost
certainly travels, so (a) its integration KAT as modelled would bypass that path and (b) an in-scope doc
comment (`read.rs:169`) states a precision bound the feature now contradicts. Fix that one Important and
the spec is shippable.

### What I verified as SOUND (no finding)
- **Sole BTC→sat path (pressure-test #1).** Confirmed. Every adapter — coinbase.rs:126, gemini.rs:93,
  swan.rs:172/200/224/285, river.rs:103/111 — routes through `parse::parse_btc_to_sat`. The only other
  `SATS_PER_BTC` uses are in `btctax-core` and go the OTHER direction (sat→USD `price.rs:16`; sat→BTC
  display `forms.rs:86`). No second import path; no USD→sat reverse path that fabricates sats. The fix is
  complete for all four sources, not just Gemini.
- **`Decimal::round()` semantics (pressure-test #2).** Verified against vendored source
  (`rust_decimal-1.42.1/src/decimal.rs`): `round()` → `round_dp(0)` → `round_dp_with_strategy(0,
  MidpointNearestEven)`. So it is round-half-to-**even**, to 0 dp (integer-valued Decimal). `.to_i64()`
  (ToPrimitive already imported at parse.rs:4) is therefore exact. All 5 stated conversions verified
  arithmetically: `0.0010216163→102162` (102161.63↑), `0.0997506234→9975062` (.34↓), `0.7674706206→
  76747062` (.06↓), `-0.1156442018→-11564420` (.18↓, sign-symmetric; gemini.rs:93 then `.abs()`),
  `0.00076035204→76035` (.204↓). Sub-half `0.000000001→0` correct and benign.
- **Convention consistency (pressure-test #3).** `round_cents` uses `MONEY_ROUNDING =
  RoundingStrategy::MidpointNearestEven` (conventions.rs:13/22). `Decimal::round()` uses the SAME strategy.
  **They match** — see Minor-1 (spec must state this resolution so the implementer does not switch to
  `MidpointAwayFromZero`). A mismatch would in any case never change a result on realistic data (ties are
  exactly-0.5-sat BTC values, vanishingly rare; a tie differs by ≤1 sat ≈ $0.000001).
- **Removing `FractionalSat` (pressure-test #4).** Safe. Grep shows the variant is referenced only at its
  definition (lib.rs:60), the reject (parse.rs:85), the doc (parse.rs:56) and one test (parse.rs:229).
  `AdapterError` is not `#[non_exhaustive]`, derives no `Serialize`, and is never matched exhaustively —
  the only matches are single-pattern `matches!(…)` macros (read.rs, integration.rs) that do not break when
  a sibling variant is removed. In-workspace only. Prefer removal over `#[allow(dead_code)]`.
- **Conservation (pressure-test #5).** FR9 identity (`conservation.rs`: `Σin == disposed + removed + held +
  fee + pending`) is computed on the **integer sats already tracked** in `LedgerState`. Each cell is parsed
  (rounded) once; the rounded value is what flows and is conserved — there is no internal "true BTC"
  reference to drift against. For Gemini the trade BTC leg is a single cell (gemini.rs:93). No leg
  re-derives a counter-quantity from the same BTC string, so no double-rounding. `balanced` stays true. The
  general "split one BTC quantity across two rounded cells" hazard does not occur in any current adapter.
- **Scope / tax honesty (pressure-test #7).** Round-to-nearest is the right call (truncate biases low, skip
  drops real transactions). Sub-satoshi BTC is physically un-representable (1 sat is the floor), so
  normalizing a *quantity* to the grid is honest. The USD/tax-*value* "never silently round" guarantee is
  untouched (usd_cost/proceeds parse exactly; `round_cents` unchanged).

---

### [I1] IMPORTANT — spec ignores the xlsx numeric-cell read path: integration KAT would bypass it, and `read.rs:169`'s "≤8-dp" doc bound is contradicted
**file:** `crates/btctax-adapters/src/read.rs:169-189` (`cell_to_string`, `Data::Float(f) => format!("{f}")`); the spec's KAT section (`SPEC…:70-73`) and pressure-test #6.

**Defect.** The spec treats `parse_btc_to_sat` as the whole story, but a Gemini `.xlsx` BTC amount reaches
it only *after* the read layer turns the cell into a `&str`. The user's file is `.xlsx`, and its `Date`
cells are stored as **numbers** (the existing fixture and M-1 comment in `tests/gemini.rs` write `Date` via
`write_number` → `Data::Float`); it is therefore highly likely the `BTC Amount BTC` cells are **numeric**
too. Numeric cells go through `cell_to_string`'s `Data::Float(f) => format!("{f}")` (read.rs:176) — an
f64 shortest-round-trip — *before* `parse_btc_to_sat`. Two concrete problems the spec does not address:

1. **The integration KAT, as modelled, would not exercise the real path.** The existing synthetic fixture
   (`tests/gemini.rs`) writes every BTC amount with `ws.write_string(...)` (`Data::String` → verbatim
   trim), and the spec says "the crate already builds synthetic xlsx … with the 8 sub-sat rows" without
   specifying the cell type. An implementer copying that fixture will write the sub-sat BTC amounts as
   **strings**, so the test passes while never touching `Data::Float → format!("{f}")` — the exact step the
   user's real file uses. For a money-adjacent bug the user *reported as broken*, the acceptance evidence
   must reproduce the real file's cell shape.

2. **`read.rs:169` now states a false bound.** That doc comment asserts the f64→string conversion is
   guaranteed only for "the intended **≤8-dp** exchange decimal (e.g. 0.12345678 → "0.12345678")". This
   feature's entire premise is **10-dp** (sub-satoshi) BTC amounts flowing through that same conversion.
   Shipping the feature without updating this in-scope (btctax-adapters) comment leaves a documented
   precision bound that directly contradicts the new supported behavior.

**Why it gates.** R0 exists to catch exactly this: a money path whose central validation may not run the
production code path, plus a now-false documented precision guarantee in the touched crate. It is Important,
not Critical, because the **arithmetic is proven safe**: I verified `format!("{f}")` reproduces all five
real values *and* the clean-8dp/sub-half cases **exactly** (shortest-round-trip recovers any ≤~15-sig-dig
decimal; Gemini's 10-dp values at realistic magnitudes are 11–14 sig digits, well inside that). So there is
no demonstrated numeric defect — but the test would be non-representative and the doc would be wrong.

**Fix.**
- Make the integration KAT write the sub-sat `BTC Amount BTC` cells with **`write_number`** (`Data::Float`),
  not `write_string`, so it exercises `Data::Float → format!("{f}") → parse_btc_to_sat → round`. Ideally
  cover **both** cell types (numeric AND string) since the spec does not pin Gemini's real cell type. Assert
  the rounded sats for the 5 real values end-to-end.
- Update the `read.rs:169` doc comment: sub-satoshi (>8-dp) BTC quantities now flow through and are rounded
  to the nearest sat downstream; note that `format!("{f}")` shortest-round-trip recovers any decimal within
  f64's ~15-sig-dig clean range (so the exact string reaches the decimal parser), and that this is a BTC
  *quantity* normalization, not money rounding.
- Add one line to the spec's "Confirm SOLE path" gotcha acknowledging the read-layer `Data::Float→String`
  step as part of that path.

---

### [M1] MINOR — rounding-convention question left open; resolve it in-spec to prevent a wrong strategy swap
**file:** `SPEC…:30-33`.
The spec presents `.round()` (half-even) vs a fallback to `round_dp(0, MidpointAwayFromZero)` and defers the
choice to "[R0: confirm …]". **Resolution:** `round_cents` uses `MidpointNearestEven` (conventions.rs:13),
and `Decimal::round()` is also `MidpointNearestEven` — they already match, so **keep `.round()`** and do NOT
switch to `MidpointAwayFromZero`. Left ambiguous, an implementer could "fix consistency" in the wrong
direction (away-from-zero), which would then *dis*agree with the app's money convention. State the
resolution definitively in the spec.

### [M2] MINOR — `half_satoshi_tie` KAT has no pinned expected value, and one tie point does not discriminate the convention
**file:** `SPEC…:69`.
"a `.5`-sat tie rounds per the chosen convention (pin the tie behavior)" gives the implementer no concrete
integer to assert. Under half-even: `0.000000005 BTC` (0.5 sat) → **0**; `0.000000015` (1.5 sat) → **2**;
`0.000000025` (2.5 sat) → **2**. Note `1.5→2` is identical under half-up, so it does not prove half-even;
use `0.5→0` and `2.5→2` to actually distinguish half-even from `MidpointAwayFromZero`. Pin the exact
expected values in the spec so the KAT genuinely locks the convention.

### [N1] NIT — KAT inputs duplicate existing assertions; consolidate rather than add duplicates
**file:** `SPEC…:47-48, 67-68` vs `parse.rs:215-224, 229`.
`sub_half_satoshi_rounds_to_zero` uses `"0.000000001"` — the *same* input as the existing test at
parse.rs:229 (which the spec also says to update). `clean_8dp_btc_unchanged` (`"0.12345678"→12345678`)
duplicates the existing `btc_to_sat_is_exact_integer` assertion at parse.rs:219. Have the implementer
rename/repurpose the one existing test rather than leave a duplicate pair; the spec should say which.

### [N2] NIT — drift bound is loose
**file:** `SPEC…:52`.
"< 8 sats" is a correct but loose upper bound: round-to-nearest caps each row's error at ≤0.5 sat, so the
worst-case magnitude is ≤4 sat over 8 rows and signs may cancel. Tightening the claim (≤0.5 sat/row) makes
the "negligible (< $0.001)" point self-evident. Purely cosmetic.

---

**Gate call:** 1 Important (I1) is open → **implementation is blocked** until folded and re-reviewed.
M1/M2 should fold in the same pass (both remove real implementer ambiguity on a money path). N1/N2 optional.
