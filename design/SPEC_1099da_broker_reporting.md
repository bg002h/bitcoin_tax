# SPEC — Form 1099-DA broker reporting on the input surface (FR-46 / port report R28)

**Status: DRAFT r1 (2026-09-06), for review to 0C/0I before build.** Owning phase: NOW (by end of
September 2026 — strategy review S3; the forms arrive ~2027-02-16, inside the season window).

## Why this exists

TY2026 is the first year brokers report **basis** on Form 1099-DA (TD 10000; proceeds reporting began
with TY2025). The owner's dispositions are all on exchanges (`WalletId::Exchange`, provider =
`Source::tag()` — coinbase / gemini / river / swan). Today the TY2025 maps box **every** Form 8949 row
**I** (short-term) / **L** (long-term) — *"…not reported to you on Form 1099-DA"* — unconditionally
(`forms.rs::form_8949`: `if da { Form8949Box::I } else { Form8949Box::C }`), and the only reader of the
"this row may be broker-reported" flag is an advisory (`box_needs_review` →
`possibly_broker_reported` → the [I5] line on stderr). For TY2025 that was an advisory-grade gap
(proceeds only). For TY2026 it is a wrong-box return: a row the broker reported WITH basis belongs in
box **G/J** (or **H/K** if basis was not reported), and a wrong box on a signed return is the
"defects in what a tool claims" class — Critical (Fable plan review C1). Nothing on the input surface
can answer the question today (`ReturnInputs` has `Form1099Int/Div/G`, no 1099-DA).

## Legal grounding (primary sources held in `legal/`)

- **i8949 (2025), "Box A or Box G / B or H / C or I"** (`design/forms/extract/i8949--2025.txt:384-415`;
  Part II mirrors as D/J, E/K, F/L at `:436-461`): box **G** = short-term reported to you on
  Form 1099-DA *"with an amount shown for cost or other basis"* (box 2 of the 1099-DA checked);
  **H** = reported *"without an amount shown for cost or other basis or showing that cost or other
  basis wasn't reported to the IRS"*; **I** = no 1099-DA (or substitute) received.
- **i8949 (2025), columns (f)/(g), code B** (`:999-1030`): if the basis on the 1099-DA is incorrect —
  with box **B/H** (basis NOT reported to the IRS): enter the correct basis in (e), `-0-` in (g); with
  box **A/G** (basis reported): enter the basis *shown on the 1099-DA* in (e) *even though incorrect*,
  code **B** in (f), and the correction in (g) (Worksheet for Basis Adjustments in Column (g)).
- **Form 1099-DA instructions**, box 2 *"Check if Basis Reported to IRS"*
  (`legal/text/irs-forms/Instructions_1099-DA.txt:547`).
- **TD 10000** (`legal/text/federal-register/TD_10000…`, lines 3041/5050): gross proceeds for sales
  on/after 2025-01-01; basis for covered digital assets acquired on/after 2026-01-01. Treas. Reg.
  §1.6045-1 (`legal/primary-sources/regulations-cfr/26CFR_1.6045-1_broker_reporting.xml`).
- **Research memo** `legal/research/REPORT_us_btc_tax_TY2025-2026.md` §9 (confidence HIGH, 3-0):
  route each transaction from {1099-DA received?, basis reported on it?} → G/J, H/K, I/L; never C/F
  for digital assets.
- `YEAR.toml` `information_returns.f1099da = { proceeds, basis }` (design r2 §6) declares the REGIME
  per year: 2017/2024 `{false,false}`; 2025 `{true,false}`; 2026 `{true,true}`.

## The rule

**R1 — the filer answers, per disposition SOURCE, per tax year; the tool never assumes.** A new
`ReturnInputs` block:

```rust
/// Form 1099-DA — what each exchange REPORTED to the IRS about this year's dispositions.
/// Keyed by the disposing wallet's provider (`WalletId::Exchange { provider, .. }` = `Source::tag()`).
pub struct BrokerReporting {
    /// One answer per exchange with a disposition in the year. Absent key = unanswered.
    pub by_provider: BTreeMap<String, BrokerReported>,
}
pub enum BrokerReported {
    /// No Form 1099-DA (or substitute statement) received for this provider → box I / L.
    None,
    /// A 1099-DA with proceeds only — box 2 "basis reported to IRS" NOT checked → box H / K.
    Proceeds,
    /// A 1099-DA with basis (box 2 checked) → box G / J; the engine's basis is still what we
    /// believe is correct — when it differs from the broker's, code B + adjustment (R4).
    Basis,
}
```

It is a **class-A DECLARATION** in the classifier (`Census::declaration`, `FORM_QUESTIONS`): liveness =
"the year has ≥1 disposition on an exchange AND `YEAR.toml` says `f1099da.proceeds`"; when live and
unanswered for a provider with dispositions, the full return **REFUSES** (`RefuseReason::
BrokerReportingUnanswered { provider, year }`) — port report §6 rule 18: *no TY2026 8949 whose box was
chosen without a 1099-DA answer.* When not live (TY2017/2024; or a year where no exchange disposed),
the question is not asked and the box is I/L (TY2025+) or C/F (pre-2025) as today.

**R2 — routing.** Per row: `(term, answer)` → box: ST `None→I`, `Proceeds→H`, `Basis→G`; LT
`None→L`, `Proceeds→K`, `Basis→J`. Pre-TY2025 years keep C/F (the securities boxes are never used for
digital assets — never A/B/D/E).

**R3 — one Form 8949 page-set per BOX.** Today `fill8949.rs::place_part` checks ONE box per part page
(`part.box_field`, `box_on = "6"`). Rows must be grouped by (part, box) and each group gets its own
page copies (the ⌈rows/grid⌉ pagination already exists); Schedule D lines 1b/2/3 (ST) and 8b/9/10 (LT)
then receive the per-box totals — the 2025 Schedule D prints *"Totals for all transactions reported on
Form(s) 8949 with Box A/G checked"* on line 1b, B/H on line 2, C/I on line 3 (and D/J, E/K, F/L on
8b/9/10). This is a Schedule D transcription change, line-named (`ScheduleDLines` gains the per-box
rows it lacks) — the TY2025 Schedule D extract is the authority for the line text.

**R4 — the broker's basis versus the engine's.** The engine's basis (HIFO/FIFO per the method
election, per-wallet after `TRANSITION_DATE`) is what the filer asserts is correct. With answer
`Basis`, the row prints the engine's basis in (e) **only if it equals the broker's**; the tool cannot
know the broker's figure without a 1099-DA import, so R4's first cut is: with `Basis`, print the
engine's basis in (e), code **B** in (f) and `0` in (g), and an [I5]-grade advisory *"compare column
(e) to box 1e of the 1099-DA; if they differ, enter the broker's figure in (e) and the difference in
(g)"* — an honest disclosure of what the tool did and did not do. A per-lot 1099-DA import (broker
basis per row, automatic (g)) is **out of scope** here and filed as its own follow-up.

**R5 — the map.** `forms/2025/f8949.map.toml`'s `box_on = "6"` (hard-coded I/L) becomes a table of
the six digital-asset check boxes per part (`box_g/h/i`, `box_j/k/l` — FQNs read off the PDF with
`xtask dump-fields`, every one held by `map_pdf_conformance`); TY2024's map keeps C/F only.

## Current state — hook points (recon @ c76adf6b)

| what | where | today |
|---|---|---|
| box chosen | `btctax-core/src/forms.rs:136-147` (`form_8949`) | `da ? I : C` / `da ? L : F` from `term` and year |
| the flag | `Form8949Row.box_needs_review` (`forms.rs:73`, set `:149`) | `matches!(leg.wallet, Exchange{..})` → advisory only |
| the advisory | `btctax-cli/src/cmd/admin.rs:452` `broker_reporting_advisory` | fires on both export arms since `118b070b` |
| the filler | `btctax-forms/src/fill8949.rs:259` `split_parts` (by PART only), `:78` `place_part` (one box per part page) | no per-box grouping |
| the map | `forms/2025/f8949.map.toml:10-11,30-31` `box_field`/`box_on = "6"` | one box per part |
| Schedule D | `printed.rs::ScheduleDLines` (18 `lineNN`) | lines 1b/2/3 as one "not reported" total each |
| the year regime | `forms/<year>/YEAR.toml` `information_returns.f1099da` | declared; no reader |
| the input surface | `btctax-core/src/tax/return_inputs.rs` | `Form1099Int/Div/G`; no 1099-DA |
| the classifier | `classifier.rs` `FORM_QUESTIONS` / `Census::declaration` | class-A pattern to reuse |
| provider key | `adapters/src/normalize.rs:63` `exchange_wallet(source)` | `provider = source.tag()`, `account = "default"` |

## Plan (TDD — every task lands with its kill)

- **T1 — the declaration.** `BrokerReporting` on `ReturnInputs` (serde default = empty = unanswered);
  `FORM_QUESTIONS` entry with liveness from `YEAR.toml` + the year's exchange dispositions;
  `RefuseReason::BrokerReportingUnanswered`. Kills: TY2026 + one exchange disposition + no answer →
  refuse (assemble-level test); TY2024 → not asked; answer for a provider with no dispositions →
  `Refusal` (an answer for nothing is a fabricated declaration).
- **T2 — routing.** `form_8949` takes the answers; `Form8949Box` gains G/H/J/K; the six-way table
  under test with every (term, answer, year) cell; pre-2025 never leaves C/F. Kill: a `Basis` answer
  for TY2025 (regime proceeds-only) → `Refusal` (the broker cannot have reported basis).
- **T3 — the map + per-box pages.** `f8949.map.toml` (2025) six box FQNs; `split_parts` → by (part,
  box); `place_part` per group; `map_pdf_conformance` holds every FQN. Kill: a mixed G+I short-term
  set prints TWO Part I page-sets, each with its own box; a golden packet for it.
- **T4 — Schedule D per-box totals.** `ScheduleDLines` lines 1a/1b/2/3 and 8a/8b/9/10 as the 2025
  extract prints them; `line_coverage` quotes; the totals cross-foot with the 8949 page-sets. Kill:
  the 8949 G-total must land on 1b, not 3.
- **T5 — code B and the advisory (R4).** Column (f)/(g) on `Basis` rows; the advisory text names box
  1e and the worksheet. Kill: a `Basis` row prints `B`/`0`; a `None` row prints nothing in (f)/(g).
- **T6 — the surfaces.** `income import` TOML shape; the TUI input form's new block; `report` shows
  the answers per provider; `YearReadiness::sentence` gains "1099-DA regime: proceeds+basis".
- **T7 — the owner action (not code).** A standing specific-ID instruction to each exchange before
  further 2026 sales, and a check of what each will put in box 1e (strategy review S3) — recorded in
  `ROADMAP_STATUS.md` §0a as an owner item with a date.

## Out of scope (filed separately)

- A Form 1099-DA **import** (per-lot broker basis and proceeds, automatic (g), reconciliation of the
  broker's lot identification against the engine's) — the larger item S3 warned about; owning phase:
  after the first 1099-DAs arrive (~2027-02-16).
- Securities boxes A/B/D/E (1099-B): digital assets never use them (i8949).
- Substitute statements from non-broker venues.

## Open questions for the owner (answers change T1's liveness, nothing else)

1. Which of Coinbase / Gemini / River / Swan will issue a 1099-DA for TY2026, and which will report
   basis (box 2)? Unknown until the forms arrive; T1's default is "unanswered ⇒ refuse", never a guess.
2. Does any 2026 disposition happen on a venue outside the four adapters (intake is a locked door)?
