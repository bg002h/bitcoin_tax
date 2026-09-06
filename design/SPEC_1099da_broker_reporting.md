# SPEC — Form 1099-DA broker reporting on the input surface (FR-46 / port report R28)

**Status: DRAFT r3 (2026-09-06), for review to 0C/0I before build.** r1 review
(`design/agent-reports/2026-09-06-spec-1099da-review.md`, 4C/3I/8M) and r2 review
(`…-review-r2.md`, 1C/5I/8M; ledgers `…-VERIFICATION.md` 15/15 and 15/15 TRUE) folded. r3 adds the
`Mixed` answer that refuses (C-1), enumerates each key's rows so the declaration is about a set the
filer can see, nests the map so it serialises (N-1), moves the screen to the site that holds the ledger
(N-2), declares the crypto slice CLOSED on a basis-regime year (N-3), restores the "answer on a
non-live year refuses" rule and kill (N-4), and gives the cohort a three-way mechanism with a real
`Unknown` (N-5). Owning phase: NOW (by end of September 2026 — strategy review S3; the forms arrive
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
`ReturnInputs` (serde default = empty = unanswered); nested, not tupled, because `ReturnInputs`
round-trips through JSON (the vault) and TOML (`income import`), and both require string map keys:

```rust
/// Which of a broker's two kinds of Form 1099-DA the ENGINE expects a row to fall under
/// (§1.6045-1(a)(15)(i)(J)/(G), (a)(16)). The broker's own lot identification decides the real
/// answer; this is the partition the filer answers OVER, and `Mixed` is the answer when the forms
/// in a partition disagree.
pub enum Cohort { Covered, Noncovered }
/// What the 1099-DA(s) for one key SHOW — read off the physical forms by the filer, for the rows
/// btctax lists under that key (R1 enumerates them).
pub enum BrokerReported {
    /// No 1099-DA (or substitute statement) lists any of these dispositions → box I / L.
    NotReported,
    /// Every one is listed with box 2 NOT checked (proceeds only) → box H / K.
    ProceedsOnly,
    /// Every one is listed with box 2 checked, and box 1g equals btctax's column (e) on each
    /// → box G / J, columns (f)/(g) blank.
    BasisMatches,
    /// Every one is listed with box 2 checked, and at least one box 1g differs from btctax's (e)
    /// → REFUSE: the row needs the broker's figure in (e) and the correction in (g), which only a
    /// per-lot 1099-DA import can supply (out of scope).
    BasisDiffers,
    /// The forms for these dispositions do NOT all say the same thing (some listed, some not; some
    /// with box 2, some without) → REFUSE: no single box is true of the set; the per-lot import is
    /// the exit.
    Mixed,
}
/// One provider's answers, one slot per cohort. An absent provider, or an absent slot for a cohort
/// that has rows, is UNANSWERED.
pub struct CohortAnswers { pub covered: Option<BrokerReported>, pub noncovered: Option<BrokerReported> }
pub struct BrokerReporting { pub by_provider: BTreeMap<String, CohortAnswers> }
```

- **The cohort is derived by the engine, per row, at row construction** (`form_8949`, which has the
  `DisposalLeg`: `basis_source: BasisSource`, `lot_id`, `wallet`), by MECHANISM, not by outcome
  (`BasisSource`, `crates/btctax-core/src/event.rs:17-29`, eleven variants):

  | the consumed lot's `basis_source` | in an `Exchange` wallet, acquisition event dated ≥ 2026-01-01 | cohort |
  |---|---|---|
  | `ExchangeProvided`, `ComputedFromCost` (a purchase or exchange on the venue) | yes | **Covered** — §1.6045-1(a)(15)(i)(J) |
  | the same | no (pre-2026, or not an exchange wallet) | Noncovered |
  | `CarriedFromTransfer`, `SelfTransferInbound`, `GiftCarryover`, `GiftFmvFallback`, `SafeHarborAllocated`, `ReconstructedPerWallet`, `EstimatedConservative` | any | Noncovered — arrived by transfer or reconstruction; no §1.6045A-1 statement comes from the filer's own wallet or from a reconstruction |
  | `FmvAtIncome`, `CardRewardRebate` | yes | **Unknown → `RefuseReason::BrokerCohortUnknown`** — (J) covers acquisition "in exchange for … services", so an exchange-credited 2026 reward may be covered; the broker decides, not the engine |
  | the same | no | Noncovered |

  The date is the lot's **acquisition event date** (via `lot_id` → the lot's origin event), NOT
  `DisposalLeg::acquired_at`, which is the zone-aware holding-period start (`state.rs:208-212`).
  Under a regime with `basis = false` every row is `Noncovered`. The row carries `cohort: Cohort`.
- **What the declaration is about, and what it cannot see.** The answer quantifies over a row set
  the engine derived, so the tool MUST show it: `report` and the TUI prompt enumerate each key's
  rows (date sold, amount, proceeds, btctax's column (e)) before the answer is taken (T6). The
  broker's partition into box-2-checked / unchecked follows the lot the BROKER identified
  (`Instructions_1099-DA.txt:253-262`: a covered asset "must" complete boxes 1d, 1g, 2, 6; a
  noncovered one "may check box 9"), and customer-provided acquisition information is used
  "solely for lot-selection purposes and not for reporting basis or acquisition dates" (`:475-477`)
  — so the broker's partition need not coincide with the engine's. Two divergences SURFACE and refuse:
  a key whose forms disagree among themselves (`Mixed`), and a covered form whose basis differs
  (`BasisDiffers`). One does NOT surface: a proceeds-only form (no basis to compare) listing a sale
  the broker matched to a different lot than the engine did — box H/K and the engine's basis print,
  and the gain differs from what the broker's lot would give. That is the specific-ID question
  (Rev. Proc. 2024-28; §1.1012-1(j)) the owner action T7 exists for — a standing lot-identification
  instruction to each exchange is what makes the two partitions coincide — and the per-lot import is
  what would detect it. Stated here so the blind spot is chosen, not discovered.
- **Key on `provider`** (= `Source::tag()`), not `(provider, account)`, because `exchange_wallet`
  (`crates/btctax-adapters/src/normalize.rs:64`) always sets `account = "default"` — single-account
  today; a 1099-DA is per account, so the key widens when accounts do.
- **Answered-ness (I1, option b).** This is NOT a `FORM_QUESTIONS` entry — that registry is boolean
  and singular (`get: fn(&ReturnInputs) -> Option<bool>`, `set(_, bool)`, `neutral: bool`, a `const`
  `RefuseReason`) and `testonly::answer_all_live_declarations` would fill it with a "neutral" that
  does not exist. It is its own screen at the site that holds the ledger —
  `screen_absolute` (`crates/btctax-core/src/tax/return_1040.rs:2608`, where
  `DonationRestrictionsUnresolved` is decided for the same reason) — which gains one argument,
  `regime: InformationReturnRegime` (51 call sites, measured; the compiler lists them). It is its own
  class in the classifier census (an unanswered key is counted, never defaulted). `testonly` MUST
  NOT auto-answer it — kill: `answer_all_live_declarations` leaves `BrokerReporting` empty and the
  assemble still refuses.
- **Liveness** = the year's regime says `f1099da.basis` AND the year has ≥1 disposition on an
  exchange. The regime reaches core as a VALUE (`InformationReturnRegime { proceeds, basis }`) passed
  by the CLI/TUI from `YearRecord` — `btctax-forms` depends on `btctax-core` (`Cargo.toml:16`), so
  core never reads `YEAR.toml`; a kill holds the CLI join (2024 `{false,false}`, 2025 `{true,false}`,
  2026 `{true,true}`), and a second holds `regime.proceeds == (year >= DIGITAL_ASSET_8949_FIRST_YEAR)`
  for every bundled year so the constant (`forms.rs:55`, read at `forms.rs:135`, `admin.rs:471`,
  `tui/src/tabs/forms.rs:174`) cannot drift from the record. Live + a key with rows unanswered →
  **REFUSE** (`RefuseReason::BrokerReportingUnanswered { provider, cohort, year }`; port report §6
  rule 18). **Any answer on a key with NO rows, or on a year where the question is not live, →
  `Refusal`** (a declaration about nothing is fabricated; a declaration the tool would not read is
  testimony silently discarded). **`BasisMatches`/`BasisDiffers` under a regime with `basis = false`
  → `Refusal`** (the broker cannot have reported basis) — the r1 kill, restored.
- **The crypto slice is CLOSED on a basis-regime year (N-3).** `export_irs_pdf_from_session`
  (`admin.rs:594`) runs the crypto slice iff no `ReturnInputs` is stored, and the answers live on
  `ReturnInputs`; so for a live year the slice arm refuses **before any byte**, unconditionally, and
  the refusal names the exit: *"TY2026 Form 8949 needs the Form 1099-DA answers, which live on the
  return inputs — `income import` / the TUI input form, then export the full return."* Kills, both
  directions: TY2026 + one exchange disposition on the slice arm → refusal, no `f8949.pdf`; TY2025
  same inputs → fills as today. This retires the slice as a product surface for TY2026+ (btctax has
  no users; the full return is the product) — recorded in `ROADMAP_STATUS.md` §0a as S10, an
  owner-visible decision; reversing it means a `broker_reporting` vault table both arms read.
- **TY2025 is UNCHANGED (I3).** Liveness is gated on `basis`, so TY2025 keeps I/L and the [I5]
  advisory as shipped. A proceeds-only 1099-DA row does belong in H/K; that gap is recorded in
  `FOLLOWUPS.md` as owned by the owner's S1 decision (the TY2025 rehearsal), because flipping it is a
  one-flag change to liveness (`proceeds`) deliberately not made on a paused year with fifteen
  shipped maps and golden packets. Kill: TY2025 + exchange disposition + no answer → NOT refused,
  box I/L; TY2025 + a stored `BasisMatches` → `Refusal` (not live; not silently discarded).
- **T0 (I2).** `crates/btctax-forms/forms/2026/YEAR.toml` — status `preparing`, all 18 stems absent
  with reasons, regime `{ proceeds = true, basis = true }` — so the CLI join has a 2026 record and
  the TY2026 kills have something to read (`build.rs:81` enters a year per file, so `YEAR.toml` alone
  bundles it). ★ Consequences, stated so they are chosen: `default_year()` (newest bundled) becomes
  **2026**, and its only callers are the two TUIs (`tui/src/app.rs:195`, `tui-edit/src/editor.rs:300`),
  so **both TUIs open on a year with zero bundled forms** — the readiness sentence says so on the
  unlock screen. Four test sites red and are updated by name in T0: `year_readiness.rs:317`,
  `tui-edit/src/main.rs:15459`, `tests/year_record.rs:55` (`bundled_years() == [2017, 2024, 2025]`)
  and its `expected_count` `other => panic!` arm. The 2026 record flips to `filable` in the same
  commit that inserts the TY2026 `FullReturnParams` (FR-47; `YearReadiness` reds a non-filable year
  with params bundled).

**R2 — routing.** Per row, `(term, answer for the row's key)` → box: ST `NotReported→I`,
`ProceedsOnly→H`, `BasisMatches→G`; LT `→L`, `→K`, `→J`; `BasisDiffers` and `Mixed` → **refuse**
(`RefuseReason::BrokerBasisDiffers { provider, cohort }` / `BrokerReportingMixed { provider, cohort }`,
each naming the per-lot import as the exit); `Unknown` cohort → refuse (R1). A `BasisMatches`/
`BasisDiffers` answer on a `Noncovered` key is allowed (voluntary basis reporting, 1099-DA instr
`:548-551`) — the filer reads it off the form. Pre-2025 revisions have no digital-asset boxes at all;
this spec makes no pre-2025 change.

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
*"compare column (e) of every G/J row with box 1g of the 1099-DA, and column (d) of every listed row
with box 1f; if any differs, the return needs the broker's figure in that column and the correction
in (g) — see the Note on Form 8949"*. (btctax prints NET proceeds — `DisposalLeg.proceeds`,
`state.rs:200`; the broker must reduce box 1f by transaction costs too, `Instructions_1099-DA.txt:
470-472`, so the figures normally coincide; the disclosure names the limb anyway.)

**R5 — the map.** `crates/btctax-forms/forms/2025/f8949.map.toml` (`box_field`/`box_on` at
`:18-19`, `:38-39` — hard-coded I/L) gains, beside the scalar pair `PartMap` has today
(`map.rs:449-464`, which the C/F revisions keep), an optional per-part table `boxes = { G = { field,
on }, H = …, I = … }` / `{ J, K, L }` — FQNs read off the PDF with `xtask dump-fields`, every one held
by `map_pdf_conformance`; `pdf.rs:351-368` already rejects an on-state the widget does not declare.
A 2025+ map must carry the table and a pre-2025 map must not (a kill each way). On
TY2025 only I/L are USED (R1); the **2026** map inherits the six-box table at port time (the port
runbook gains the row). TY2024's map keeps C/F only.

## Current state — hook points (recon @ c76adf6b, cites re-resolved @ 2aa4ea98)

| what | where | today |
|---|---|---|
| box chosen | `crates/btctax-core/src/forms.rs:136-147` (`form_8949`) | `da ? I : C` / `da ? L : F` from `term` and year |
| the flag | `Form8949Row.box_needs_review` (`forms.rs:73`, set `:149`) | `matches!(leg.wallet, Exchange{..})` → advisory only |
| the provenance | `DisposalLeg { basis_source, lot_id, wallet, acquired_at }` (`state.rs:197-217`); `BasisSource` (`event.rs:17-29`) | in scope at row construction; `acquired_at` is the HP start, not the acquisition date |
| the advisory | `crates/btctax-cli/src/cmd/admin.rs:467` `broker_reporting_advisory` | fires on both export arms since `118b070b` |
| the two arms | `admin.rs:573-600` `export_irs_pdf_from_session` | full return via `ReturnInputs`; slice via `form_8949` + `fill_form_8949` (`:631`, `:716`) |
| the filler | `crates/btctax-forms/src/fill8949.rs:259` `split_parts` (by PART), `:78` `place_part` (one box per page); `lib.rs:119-137` pairs ST chunk *k* with LT chunk *k* | no per-box grouping |
| the map | `crates/btctax-forms/forms/2025/f8949.map.toml:18-19, 38-39` | one box per part |
| Schedule D | `printed.rs:935-972` `ScheduleDLines` (18 `lineNN`) | no 1b/2/8b/9 lines exist |
| the year regime | `crates/btctax-forms/forms/<year>/YEAR.toml` `information_returns.f1099da` | declared; no reader; no 2026 record |
| the input surface | `crates/btctax-core/src/tax/return_inputs.rs` | `Form1099Int/Div/G`; no 1099-DA |
| the screens | `return_1040.rs:2608` `screen_absolute` (has the ledger; decides `DonationRestrictionsUnresolved`); `return_refuse.rs:853` `screen_inputs` (inputs only — cannot see dispositions) | the non-declaration pattern to reuse |
| `testonly` | `testonly.rs:34-40` `answer_all_live_declarations` | auto-answers every live `FORM_QUESTIONS` entry with `neutral` |
| provider key | `crates/btctax-adapters/src/normalize.rs:64` `exchange_wallet(source)` | `provider = source.tag()`, `account = "default"` |

## Plan (TDD — every task lands with its kill)

- **T0 — the 2026 record + the regime value.** `forms/2026/YEAR.toml` (preparing; regime
  `{true,true}`); `InformationReturnRegime` value type in core; the CLI/TUI join `YearRecord → regime`
  with its kill (2024/2025/2026); the constant-vs-regime kill; the `default_year` → 2026 consequence
  with the four tests updated by name.
- **T1 — the declaration.** `BrokerReporting` on `ReturnInputs` (nested map); the `screen_absolute`
  arm and its `regime` argument; the classifier class; `RefuseReason::{BrokerReportingUnanswered,
  BrokerReportingMixed, BrokerBasisDiffers, BrokerCohortUnknown}` (an exhaustive cross-crate match
  reds). Kills: TY2026 + one exchange disposition + no answer → refuse; TY2025 same inputs → not
  refused, I/L; TY2024 → not asked; an answer for a key with no rows → `Refusal`; any answer on a
  non-live year → `Refusal`; `BasisMatches` under `basis = false` → `Refusal`;
  `answer_all_live_declarations` leaves it unanswered; the slice arm: TY2026 → refusal before any
  byte naming the exit, TY2025 → fills.
- **T2 — cohort + routing.** `form_8949` derives `cohort` per row from the table in R1 (by
  `BasisSource`, wallet kind, and the lot's acquisition event date) and takes the answers; the
  routing table `(term, answer) → box | refuse` under test for every cell; `FmvAtIncome` in an
  exchange wallet dated 2026 → `BrokerCohortUnknown` (the kill now has an input). Kills: one
  provider, two cohorts, two different answers → two Part I page-sets with different boxes (S4); a
  `Mixed` answer → refusal naming the import (C-1).
- **T3 — the map + per-(part, box) pages.** six FQNs; `split_parts` → by (part, box); the
  concatenate-then-zip rule of R3. Kill: G+I ST with a single J LT → 2 copies, boxes as R3 states; a
  golden packet for it.
- **T4 — Schedule D per-box totals.** lines 1a/1b/2/3 and 8a/8b/9/10 as the 2025 extract prints
  them; `line_coverage` quotes; the totals cross-foot with the page-sets. Kill: the G-total lands on
  1b, not 3.
- **T5 — (f)/(g) and the advisory (R4).** Kill: a `BasisMatches` row prints NOTHING in (f)/(g); no
  row from any answer prints `B` or `0`; the advisory text names box 1g AND box 1f.
- **T6 — the surfaces.** `income import` TOML shape (`[broker_reporting.coinbase] covered = "…",
  noncovered = "…"`); the TUI input form's new block, which ENUMERATES each key's rows before taking
  the answer; `report` lists the keys, their rows, and the answers; `YearReadiness::sentence` gains
  "1099-DA regime: proceeds+basis"; the advisory at `admin.rs:471` and the TUI forms tab
  (`tabs/forms.rs:174`) read the regime, not the constant.
- **T7 — the owner action (not code).** A standing specific-ID instruction to each exchange before
  further 2026 sales, and a check of what each will put in **box 1g** (strategy review S3 — which
  says "box 1e"; that report is verbatim and is corrected here, not there) — recorded in
  `ROADMAP_STATUS.md` §0a as an owner item with a date.

## Out of scope (filed separately)

- A Form 1099-DA **import** (per-lot broker basis and proceeds, automatic (g), reconciliation of the
  broker's lot identification against the engine's) — the exit `BasisDiffers` names; owning phase:
  after the first 1099-DAs arrive (~2027-02-16).
- Securities boxes A/B/D/E (1099-B): digital assets never use them (i8949).
- Detecting a lot-identification divergence on a proceeds-only form (R1's stated blind spot): the import.
- Substitute statements from non-broker venues; multi-account exchange keys.

## Open questions for the owner (answers change T1's liveness, nothing else)

1. Which of Coinbase / Gemini / River / Swan will issue a 1099-DA for TY2026, and for which cohorts
   will box 2 be checked? Unknown until the forms arrive; the default is "unanswered ⇒ refuse".
2. Does any 2026 disposition happen on a venue outside the four adapters (intake is a locked door)?
