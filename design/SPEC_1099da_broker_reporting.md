# SPEC — Form 1099-DA broker reporting on the input surface (FR-46 / port report R28)

**Status: DRAFT r2 (2026-09-06), for review to 0C/0I before build.** r1 review
(`design/agent-reports/2026-09-06-spec-1099da-review.md`, 4C/3I/8M, ledger
`…-VERIFICATION.md` 15/15 TRUE) folded here: the grain is now per **(provider, cohort)** (S4), the
tool never prints a code B or a `0` it did not collect (S1), the refusal reaches BOTH export arms
(S2), the 1099-DA basis box is **1g** (S3), the answer is its own screen rather than a boolean
`FORM_QUESTIONS` entry (I1), a `forms/2026/` year record is T0 (I2), and TY2025 is explicitly
unchanged (I3). Owning phase: NOW (by end of September 2026 — strategy review S3; the forms arrive
~2027-02-16, inside the season window).

## Why this exists

TY2026 is the first year brokers report **basis** on Form 1099-DA (Treas. Reg. §1.6045-1; proceeds
reporting began with TY2025). The owner's dispositions are all on exchanges (`WalletId::Exchange`,
provider = `Source::tag()` — coinbase / gemini / river / swan). Today the TY2025 maps box **every**
Form 8949 row **I** (short-term) / **L** (long-term) — *"…not reported to you on Form 1099-DA"* —
unconditionally (`forms.rs::form_8949`: `if da { Form8949Box::I } else { Form8949Box::C }`), and
the only reader of the "this row may be broker-reported" flag is an advisory (`box_needs_review` →
`possibly_broker_reported` → the [I5] line on stderr). For TY2025 that was an advisory-grade gap
(proceeds only). For TY2026 it is a wrong-box return: a row the broker reported WITH basis belongs
in box **G/J** (or **H/K** if basis was not reported), and a wrong box on a signed return is the
"defects in what a tool claims" class — Critical (Fable plan review C1). Nothing on the input surface
can answer the question today (`ReturnInputs` has `Form1099Int/Div/G`, no 1099-DA).

## Legal grounding (primary sources held in `legal/`)

- **i8949 (2025), boxes** (`design/forms/extract/i8949--2025.txt:384-417`; Part II mirrors as D/J,
  E/K, F/L at `:436-466`): box **G** = short-term reported to you on Form 1099-DA *"with an amount
  shown for cost or other basis"* (box 2 of the 1099-DA checked); **H** = reported *"without an
  amount shown for cost or other basis or showing that cost or other basis wasn't reported to the
  IRS"*; **I** = no 1099-DA (or substitute) received; *"Do not use box C to report digital asset
  transactions. Use box I."* (`:416-417`).
- **The Note printed on Form 8949 itself** (`f8949--2025.txt:54-55`, identical on
  `f8949--2026-DRAFT.txt:92-93`): *"If you checked Box A or Box G above but the basis reported to the
  IRS was incorrect, enter in column (e) the basis as reported to the IRS, and enter an adjustment in
  column (g) to correct the basis."* And **i8949 columns (f)/(g)** (`:999-1030`, `:1052-1055`):
  *"For most transactions, you don't need to complete columns (f) and (g) and can leave them blank"*
  (`:1000`); code B applies when *"the basis shown in box 1e on Form 1099-B or box 1g on Form 1099-DA
  is incorrect"* (`:1052-1055`) — with box G/J the broker's figure goes in (e) and the correction in
  (g); with box H/K the correct basis goes in (e) and `-0-` in (g).
- **Form 1099-DA instructions** (`legal/text/irs-forms/Instructions_1099-DA.txt`): box 2 *"Check if
  Basis Reported to IRS"* (`:547`); **box 1g** *"Cost or Other Basis"* (`:498`; box 1e is *"Date Sold
  or Disposed"*, `:453`); box 9 "noncovered" is per form, and a box-9 form need not complete box 1g or
  box 2 (`:597-600`); a broker MAY report basis on a noncovered asset voluntarily (`:548-551`).
- **Treas. Reg. §1.6045-1** (`legal/primary-sources/regulations-cfr/26CFR_1.6045-1_broker_reporting.xml`):
  **(a)(15)(i)(J)** — a digital asset is a covered security when *"acquired in a customer's account by
  a broker providing custodial services … on or after January 1, 2026, in exchange for cash …"*;
  **(a)(15)(i)(G)** — a transferred-in security is covered only if the broker *"receives a transfer
  statement (as described in § 1.6045A-1) reporting the security as a covered security"*;
  **(a)(16)** — everything else is noncovered. (TD 10000's committed text is column-mangled; cite the
  CFR.) ★ Consequence: ONE exchange, ONE customer, ONE year issues 1099-DAs of BOTH kinds — box 2
  checked for units bought there in 2026+, unchecked for units bought earlier or transferred in from
  self-custody (no transfer statement comes from the filer's own wallet). That is the owner's own
  profile.
- **Research memo** `legal/research/REPORT_us_btc_tax_TY2025-2026.md` §9 (confidence HIGH, 3-0):
  route each transaction from {1099-DA received?, basis reported on it?} → G/J, H/K, I/L; never C/F
  for digital assets.
- `YEAR.toml` `information_returns.f1099da = { proceeds, basis }` (design r2 §6) declares the REGIME
  per year: 2017/2024 `{false,false}`; 2025 `{true,false}`; 2026 `{true,true}` (**T0** — no
  `forms/2026/` exists yet).

## The rule

**R1 — the filer answers per (provider, COHORT), per tax year; the tool never assumes.** New on
`ReturnInputs` (serde default = empty = unanswered):

```rust
/// Which of a broker's two kinds of Form 1099-DA a row falls under (§1.6045-1(a)(15)(i)(J)/(G), (a)(16)).
pub enum Cohort {
    /// Units the ENGINE says were acquired by direct purchase INTO this exchange wallet on/after
    /// 2026-01-01 — the broker must report basis (box 2 checked).
    Covered,
    /// Everything else: acquired before 2026, or arrived by inbound self-transfer (no §6045A statement).
    Noncovered,
}
pub struct ReportingKey { pub provider: String, pub cohort: Cohort }
/// What the 1099-DA(s) for this key SHOW — read off the physical forms by the filer.
pub enum BrokerReported {
    /// No 1099-DA (or substitute statement) lists these dispositions → box I / L.
    NotReported,
    /// Listed, box 2 NOT checked (proceeds only) → box H / K.
    ProceedsOnly,
    /// Listed, box 2 checked, and box 1g equals btctax's column (e) for EVERY such disposition
    /// → box G / J, columns (f)/(g) blank.
    BasisMatches,
    /// Listed, box 2 checked, and at least one box 1g differs from btctax's (e) → REFUSE: the row
    /// needs the broker's figure in (e) and the correction in (g), which only a per-lot 1099-DA
    /// import can supply (out of scope).
    BasisDiffers,
}
pub struct BrokerReporting { pub answers: BTreeMap<ReportingKey, BrokerReported> }
```

- **The cohort is derived by the engine, per row, at row construction** (`form_8949`), from the
  disposed lot's basis-source classification (`forms.rs:259-296` — a direct acquisition vs. an inbound
  self-transfer) and its `date_acquired`; the row carries `cohort: Cohort`. Under a regime with
  `basis = false` every row is `Noncovered`. A lot whose provenance the engine cannot classify →
  **refuse** (`RefuseReason::BrokerCohortUnknown`), never a default.
- **Why the filer's answer still binds a SET of rows.** The broker's lot identification may differ
  from the engine's (the broker defaults to FIFO unless instructed — T7). That difference cannot
  change WHICH form lists a sale, only the basis printed on it — and the filer reads the basis off
  the form, so it surfaces as `BasisDiffers`, which refuses. The answer's text is the whole
  declaration: *"for every disposition at <provider> of units btctax classes as <cohort>, the
  1099-DA shows …"*.
- **Key on `provider`** (= `Source::tag()`), not `(provider, account)`, because `exchange_wallet`
  (`crates/btctax-adapters/src/normalize.rs:64`) always sets `account = "default"` — single-account
  today; a 1099-DA is per account, so the key widens when accounts do.
- **Answered-ness (I1, option b).** This is NOT a `FORM_QUESTIONS` entry — that registry is boolean
  and singular (`get: fn(&ReturnInputs) -> Option<bool>`, `set(_, bool)`, `neutral: bool`, a `const`
  `RefuseReason`) and `testonly::answer_all_live_declarations` would fill it with a "neutral" that
  does not exist. It is its own screen in `return_refuse::screen_inputs`, alongside
  `DonationRestrictionsUnresolved`, and its own class in the classifier census (an unanswered key is
  counted, never defaulted). `testonly` MUST NOT auto-answer it — kill: `answer_all_live_declarations`
  leaves `BrokerReporting` empty and the assemble still refuses.
- **Liveness** = the year's regime says `f1099da.basis` AND the year has ≥1 disposition on an
  exchange. The regime reaches core as a VALUE (`InformationReturnRegime { proceeds, basis }`) passed
  by the CLI from `YearRecord` — `btctax-forms` depends on `btctax-core` (`Cargo.toml:16`), so core
  never reads `YEAR.toml`. Live + a key with dispositions unanswered → **REFUSE**
  (`RefuseReason::BrokerReportingUnanswered { provider, cohort, year }`; port report §6 rule 18). An
  answer for a key with NO dispositions → `Refusal` (a declaration about nothing is fabricated).
- **Both export arms refuse (S2).** `export_irs_pdf_from_session` (`admin.rs:573-600`) runs the
  crypto slice when no `ReturnInputs` is stored, calling `form_8949`/`fill_form_8949` directly. For a
  live year the slice arm refuses **before any byte is written** unless a stored `ReturnInputs`
  answers every live key (model: `promote_export_gate`, `admin.rs:620`). Kill: TY2026, one exchange
  disposition, no stored `ReturnInputs` → refusal, no `f8949.pdf`.
- **TY2025 is UNCHANGED (I3).** Liveness is gated on `basis`, so TY2025 keeps I/L and the [I5]
  advisory as shipped. A proceeds-only 1099-DA row does belong in H/K; that gap is recorded in
  `FOLLOWUPS.md` as owned by the owner's S1 decision (the TY2025 rehearsal), because flipping it is a
  one-flag change to liveness (`proceeds`) deliberately not made on a paused year with ten shipped
  maps and golden packets. Kill: TY2025 + exchange disposition + no answer → NOT refused, box I/L.
- **T0 (I2).** `crates/btctax-forms/forms/2026/YEAR.toml` — status `preparing`, all 18 stems absent
  with reasons, regime `{ proceeds = true, basis = true }` — so the CLI join has a 2026 record and
  the TY2026 kills have something to read. ★ Consequence, stated so it is chosen: `default_year()` is
  the newest bundled year (`year_readiness.rs:203-208`), so it becomes **2026** — the year being
  prepared, which is what `report`'s readiness line should say. Tests pinning 2025 as the default are
  updated in T0, each named in the commit.

**R2 — routing.** Per row, `(term, answer for the row's key)` → box: ST `NotReported→I`,
`ProceedsOnly→H`, `BasisMatches→G`; LT `→L`, `→K`, `→J`; `BasisDiffers` → **refuse**
(`RefuseReason::BrokerBasisDiffers { provider, cohort }`, naming the per-lot import as the exit). A
`BasisMatches`/`BasisDiffers` answer on a `Noncovered` key is allowed (voluntary basis reporting,
1099-DA instr `:548-551`) — the filer reads it off the form; a `BasisMatches`/`ProceedsOnly` answer
under a regime with `proceeds = false` → `Refusal`. Pre-2025 revisions have no digital-asset boxes at
all; this spec makes no pre-2025 change.

**R3 — one Form 8949 page-set per (part, BOX).** Today `fill8949.rs::place_part` checks ONE box per
part page (`part.box_field`). Rows are grouped by (part, box); **per part**, each box-group is
paginated independently (⌈rows/cap⌉) and the groups' pages are concatenated in box order (G, H, I /
J, K, L) into one page list per part; copies = max(|ST pages|, |LT pages|); copy *k* carries ST page
*k* on page 1 and LT page *k* on page 2, an exhausted side left blank (`place_part`'s empty no-op);
each page's box is its group's. Determinate: a G+I short-term set (each ≤ cap) with a single J
long-term set → **2 copies**, page 1 of copy 1 boxed G, of copy 2 boxed I, page 2 of copy 1 boxed J.
Schedule D then prints per-box totals — the 2025 form (`f1040sd--2025.txt:32-37, 57-62`): line 1b
*"Totals for all transactions reported on Form(s) 8949 with Box A or Box G checked"*, line 2 (B or
H), line 3 (C or I); lines 8b/9/10 for D/J, E/K, F/L. `ScheduleDLines` has **no** 1b/2/8b/9 today
(18 `lineNN` fields: 1a, 3, 6, 7, 8a, 10, 13–16) — T4 CREATES four line-sets, line-named from the
extract.

**R4 — columns (f)/(g): the tool never prints a code or an amount it did not collect.**
`BasisMatches` → (f) and (g) **blank** (i8949 `:1000`). `BasisDiffers` → refuse (R2). `ProceedsOnly`
and `NotReported` → blank. No automatic code B, no `0`. The [I5]-grade advisory on a live year says:
*"compare column (e) of every G/J row with box 1g of the 1099-DA; if any differs, the return needs
the broker's figure in (e) and the correction in (g) — see the Note on Form 8949"*.

**R5 — the map.** `crates/btctax-forms/forms/2025/f8949.map.toml` (`box_field`/`box_on` at
`:18-19`, `:38-39` — hard-coded I/L) becomes a table of the six digital-asset check boxes per part
(`box_g/h/i`, `box_j/k/l` — FQNs read off the PDF with `xtask dump-fields`, every one held by
`map_pdf_conformance`; `pdf.rs:351-368` already rejects an on-state the widget does not declare). On
TY2025 only I/L are USED (R1); the **2026** map inherits the six-box table at port time (the port
runbook gains the row). TY2024's map keeps C/F only.

## Current state — hook points (recon @ c76adf6b, cites re-resolved @ 2aa4ea98)

| what | where | today |
|---|---|---|
| box chosen | `crates/btctax-core/src/forms.rs:136-147` (`form_8949`) | `da ? I : C` / `da ? L : F` from `term` and year |
| the flag | `Form8949Row.box_needs_review` (`forms.rs:73`, set `:149`) | `matches!(leg.wallet, Exchange{..})` → advisory only |
| the advisory | `crates/btctax-cli/src/cmd/admin.rs:467` `broker_reporting_advisory` | fires on both export arms since `118b070b` |
| the two arms | `admin.rs:573-600` `export_irs_pdf_from_session` | full return via `ReturnInputs`; slice via `form_8949` + `fill_form_8949` (`:631`, `:716`) |
| the filler | `crates/btctax-forms/src/fill8949.rs:259` `split_parts` (by PART), `:78` `place_part` (one box per page); `lib.rs:119-137` pairs ST chunk *k* with LT chunk *k* | no per-box grouping |
| the map | `crates/btctax-forms/forms/2025/f8949.map.toml:18-19, 38-39` | one box per part |
| Schedule D | `printed.rs:935-972` `ScheduleDLines` (18 `lineNN`) | no 1b/2/8b/9 lines exist |
| the year regime | `crates/btctax-forms/forms/<year>/YEAR.toml` `information_returns.f1099da` | declared; no reader; no 2026 record |
| the input surface | `crates/btctax-core/src/tax/return_inputs.rs` | `Form1099Int/Div/G`; no 1099-DA |
| the screens | `return_refuse.rs:853` `screen_inputs`; `DonationRestrictionsUnresolved` (`:47`) | the non-declaration pattern to reuse |
| `testonly` | `testonly.rs:34-40` `answer_all_live_declarations` | auto-answers every live `FORM_QUESTIONS` entry with `neutral` |
| provider key | `crates/btctax-adapters/src/normalize.rs:64` `exchange_wallet(source)` | `provider = source.tag()`, `account = "default"` |

## Plan (TDD — every task lands with its kill)

- **T0 — the 2026 record + the regime value.** `forms/2026/YEAR.toml` (preparing; regime
  `{true,true}`); `InformationReturnRegime` value type in core; the CLI join `YearRecord → regime`
  with a kill (2025 → `{true,false}`, 2026 → `{true,true}`, 2024 → `{false,false}`); the
  `default_year` → 2026 consequence, tests updated by name.
- **T1 — the declaration.** `BrokerReporting` on `ReturnInputs`; the `screen_inputs` arm; the
  classifier class; `RefuseReason::{BrokerReportingUnanswered, BrokerBasisDiffers, BrokerCohortUnknown}`
  (an exhaustive cross-crate match reds). Kills: TY2026 + one exchange disposition + no answer →
  refuse; TY2025 same inputs → not refused, I/L; TY2024 → not asked; an answer for a key with no
  dispositions → `Refusal`; `answer_all_live_declarations` leaves it unanswered; the crypto-slice
  arm refuses before any byte.
- **T2 — cohort + routing.** `form_8949` derives `cohort` per row and takes the answers; the
  eight-way table `(term, answer) → box | refuse` under test for every cell and year; a lot the
  engine cannot class → refuse. Kill: one provider, two cohorts, two different answers → two Part I
  page-sets with different boxes (the S4 kill).
- **T3 — the map + per-(part, box) pages.** six FQNs; `split_parts` → by (part, box); the
  concatenate-then-zip rule of R3. Kill: G+I ST with a single J LT → 2 copies, boxes as R3 states; a
  golden packet for it.
- **T4 — Schedule D per-box totals.** lines 1a/1b/2/3 and 8a/8b/9/10 as the 2025 extract prints
  them; `line_coverage` quotes; the totals cross-foot with the page-sets. Kill: the G-total lands on
  1b, not 3.
- **T5 — (f)/(g) and the advisory (R4).** Kill: a `BasisMatches` row prints NOTHING in (f)/(g); no
  row from any answer prints `B` or `0`; the advisory text names box 1g.
- **T6 — the surfaces.** `income import` TOML shape; the TUI input form's new block (per key); `report`
  shows the answers per key; `YearReadiness::sentence` gains "1099-DA regime: proceeds+basis".
- **T7 — the owner action (not code).** A standing specific-ID instruction to each exchange before
  further 2026 sales, and a check of what each will put in **box 1g** (strategy review S3 — which
  says "box 1e"; that report is verbatim and is corrected here, not there) — recorded in
  `ROADMAP_STATUS.md` §0a as an owner item with a date.

## Out of scope (filed separately)

- A Form 1099-DA **import** (per-lot broker basis and proceeds, automatic (g), reconciliation of the
  broker's lot identification against the engine's) — the exit `BasisDiffers` names; owning phase:
  after the first 1099-DAs arrive (~2027-02-16).
- Securities boxes A/B/D/E (1099-B): digital assets never use them (i8949).
- Substitute statements from non-broker venues; multi-account exchange keys.

## Open questions for the owner (answers change T1's liveness, nothing else)

1. Which of Coinbase / Gemini / River / Swan will issue a 1099-DA for TY2026, and for which cohorts
   will box 2 be checked? Unknown until the forms arrive; the default is "unanswered ⇒ refuse".
2. Does any 2026 disposition happen on a venue outside the four adapters (intake is a locked door)?
