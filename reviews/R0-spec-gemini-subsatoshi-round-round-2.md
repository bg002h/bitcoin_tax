# R0 spec review (round 2) — `SPEC_gemini_subsatoshi_round.md`

**Reviewer:** independent adversarial architect (did NOT author the spec).
**Artifact:** `design/SPEC_gemini_subsatoshi_round.md`
**Source anchor:** branch `fix/gemini-subsatoshi-round` @ `3055cc1` (main == `719e9fe`).
**Scope:** round-2 verification that the round-1 findings (0C / 1I / 2M / 2N; BLOCKED on I1) were folded
with no new drift. Spec-only; no tests run, no branch switch (read-only on the current branch).

## Verdict: **0 Critical / 0 Important / 0 Minor / 3 Nit → R0-GREEN**

Every round-1 finding is resolved against both the spec text and current source. The blocking I1 fold is
sound: the read-layer numeric path is now first-class in the spec, the arithmetic is re-verified exact, and
the KAT + doc-update requirements are unambiguous. The three residual items are cosmetic accuracy nits that
do not gate.

---

## Per-finding verification

### [I1] IMPORTANT (the blocker) — RESOLVED
The spec now carries a dedicated **"The xlsx read path [R0-I1]"** section (SPEC:41–55) plus reinforcing
hooks in the integration KAT (SPEC:90–93) and Gotchas (SPEC:104–105). Verified the three required legs:

- **(a) `read.rs:169` really does the float→string step and states a ≤8-dp bound.** Confirmed at source.
  `read.rs:176` is `Data::Float(f) => format!("{f}")`; the doc block `read.rs:169–171` asserts the
  conversion "reproduces the intended **≤8-dp** exchange decimal exactly (e.g. 0.12345678 → "0.12345678")".
  This is exactly the bound the feature contradicts, and the spec (SPEC:53–55) now mandates updating it to
  say >8-dp (sub-satoshi) quantities flow through and are rounded downstream. Unambiguous.
- **(b) `format!("{f}")` reproduces the 5 real sub-sat values exactly — no numeric defect.** Cross-checked
  each value's IEEE-754 f64 shortest-round-trip against the input string, then the downstream
  `str → Decimal × 1e8 → round(half-even)`:
  `0.0010216163→102162`, `0.0997506234→9975062`, `0.7674706206→76747062`, `-0.1156442018→-11564420`,
  `0.00076035204→76035` — all round-trip TRUE and yield the spec's sats. These values carry 8–10
  significant digits, well inside f64's ~15-sig-dig unique-round-trip range, so shortest-format recovers
  the exact string. The claim is arithmetically safe. (See Nit-1 on the spec's sig-digit *label*.)
- **(c) KAT + doc-update requirements are now unambiguous.** SPEC:49–52 requires the integration KAT to
  write the sub-sat `BTC Amount BTC` cells with `write_number` (→ `Data::Float`), covering BOTH numeric AND
  string cell types; SPEC:90–93 repeats "written as `Data::Float` numeric cells (and a string-cell
  variant)"; SPEC:104–105 adds the read-layer `Data::Float → format!("{f}") → parse_btc_to_sat` step to the
  sole-path gotcha. All three fold-fixes from round-1 are present and mutually consistent.

### [M1] MINOR (rounding convention) — RESOLVED
Verified both strategies are `MidpointNearestEven`. `conventions.rs:13`:
`MONEY_ROUNDING = RoundingStrategy::MidpointNearestEven`, used by `round_cents` (`conventions.rs:22–23`).
Vendored `rust_decimal-1.42.1/src/decimal.rs`: `round()` (1462) → `round_dp(0)` (1635) →
`round_dp_with_strategy(dp, RoundingStrategy::MidpointNearestEven)`. They match. SPEC:31–34 states this
resolution definitively ("Keep `.round()`; do NOT switch to `MidpointAwayFromZero`"). Correct — no swap risk.

### [M2] MINOR (tie KAT pinned + discriminating) — RESOLVED
Verified under half-even: `0.000000005` = 0.5 sat → **0** (0 is even); `0.000000025` = 2.5 sat → **2**
(2 is even). Both discriminate against `MidpointAwayFromZero` (which would give 1 and 3). These are direct
string inputs to `parse_btc_to_sat` (no f64 path), and `Decimal("0.000000005")·1e8 = 0.5` exactly, so the
tie is genuine. SPEC:87–89 pins both expected integers. Correct and discriminating.

### [N1] NIT (repurpose, don't duplicate) — RESOLVED
SPEC:64–66 repurposes the existing `parse.rs:229` `FractionalSat` test (`"0.000000001"` → now `Ok(0)`), and
SPEC:84–86 repurposes the existing `"0.12345678" → 12345678` assertion for `clean_8dp_btc_unchanged`. No
duplicate inputs are introduced. (See Nit-2 on the bracket citation.)

### [N2] NIT (drift bound) — RESOLVED
SPEC:71–72 tightens the bound to **≤ 4 sats** ("worst case 0.5 sat/row × 8 rows, signs may cancel"),
exactly as round-1 requested. Correct.

---

## Round-1 invariants re-spot-checked at `3055cc1` (still hold)

- **Sole BTC→sat import path.** All four adapters route through `parse::parse_btc_to_sat`
  (coinbase.rs:126, swan.rs:172/200/224/285, river.rs:103/111, gemini.rs:93). In `btctax-adapters` the only
  `SATS_PER_BTC` use is the one conversion at `parse.rs:83`. The two `btctax-core` uses are the *opposite*
  direction (sat→USD `price.rs:16`; sat→BTC display `forms.rs:86`). No second import path. Still true.
- **`FractionalSat` removal is safe.** Referenced only at `lib.rs:60` (def), `parse.rs:56` (doc),
  `parse.rs:85` (reject), `parse.rs:229` (test); zero references under `tests/`. Not `#[non_exhaustive]`,
  no `Serialize`, never exhaustively matched. Removal safe. Still true.

---

## Residual nits (non-gating)

- **[Nit-1] Sig-digit figure is mislabeled (SPEC:46–48).** The spec says Gemini's values are "11–14
  significant digits"; the five real values actually carry **8–10** significant digits. This is a
  conservative overstatement (both counts sit inside f64's ~15-sig-dig clean range), so the safety
  conclusion is unaffected — but the number is wrong and was carried verbatim from the round-1 prose. A
  one-word fix ("8–10") would make the claim exact.
- **[Nit-2] N1 bracket citation is imprecise (SPEC:65–66).** The note says `"0.000000001"` and
  `"0.12345678"` are "already asserted at parse.rs:215-224". `"0.12345678"` is at parse.rs:219 (in range),
  but `"0.000000001"` is at parse.rs:229 — which the *same sentence* already cites correctly. Harmless;
  actionable direction is unambiguous.
- **[Nit-3] Row-count vs value-count mismatch for the integration fixture (SPEC:16/71/90 vs 82–83).** The
  spec refers to "8 sub-sat rows" for the synthetic `.xlsx` fixture but documents only **5** real values.
  Pre-existing (present in round-1; not fold-introduced) and non-gating — the I1 acceptance purpose
  (exercise `Data::Float → format!("{f}")`) is met by any number of sub-sat numeric cells — but reconciling
  "8"/"5" (or saying "the 5 documented values, plus synthetic filler") would remove a small ambiguity for
  the fixture builder.

---

**Gate call:** 0 Critical / 0 Important. All round-1 findings folded correctly with no new drift.
**R0-GREEN** — implementation may proceed. The three nits are optional polish for the implementer.
