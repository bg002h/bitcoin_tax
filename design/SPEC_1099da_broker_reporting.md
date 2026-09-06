# SPEC — Form 1099-DA broker reporting on the input surface (FR-46 / port report R28)

**Status: GREEN r6 (2026-09-06) — 0 Critical / 0 Important at r5; build may proceed.** Reviews r1
(4C/3I/8M, `design/agent-reports/2026-09-06-spec-1099da-review.md`), r2 (1C/5I/8M, `…-review-r2.md`),
r3 (0C/4I/6M, `…-review-r3.md`), r4 (0C/1I/6M/6N, `…-review-r4.md`) and r5 (0C/0I/1M/2N,
`…-review-r5.md`; ledgers 15/15, 15/15, 10/10, 11/11 TRUE) folded; r6 fixes r5's three residue items
inline (the literal count, and two kills mirrored into the task lists). Owning phase: NOW (by end of
September 2026 — strategy review S3; the forms arrive ~2027-02-16, inside the season window).

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
  a broker providing custodial services for such specified security on or after January 1, 2026, in
  exchange for cash, stored-value cards, different digital assets, or any other property or services
  described in paragraph (a)(9)(ii)(B) or (C) of this section, respectively."* — the property and
  services limbs are the ones a BROKER renders and is paid for in digital assets, not services the
  customer performs; the instructions restate it (`Instructions_1099-DA.txt:41-57`: *"a covered
  security is a digital asset acquired after 2025 for cash, stored-value cards, different digital
  assets, or any property or services the disposition of which the broker is required to report"*) and
  add *"Do not report rewards and staking payments on Form 1099-DA."* (`:364-366`);
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
pub enum Cohort { Covered, Noncovered } // two-way; derived by mechanism, never refused
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
#[derive(Default)]
#[serde(transparent)]
pub struct BrokerReporting(pub BTreeMap<String, CohortAnswers>); // TOML: [broker_reporting.coinbase] covered = "…"
// on ReturnInputs (which has a MANUAL Default, no struct-level serde default):
//   #[serde(default)] pub broker_reporting: BrokerReporting,   // absent = unanswered
```

- **The cohort is derived by the engine, per row, at row construction** (`form_8949`, which has the
  `DisposalLeg`: `basis_source: BasisSource`, `lot_id`, `wallet`), by MECHANISM, not by outcome
  (`BasisSource`, `crates/btctax-core/src/event.rs:17-29`, eleven variants):

  | the consumed lot's `basis_source` | in an `Exchange` wallet, acquisition event dated ≥ 2026-01-01 | cohort |
  |---|---|---|
  | `ExchangeProvided`, `ComputedFromCost` (a purchase or exchange on the venue) | yes | **Covered** — §1.6045-1(a)(15)(i)(J) |
  | the same | no (pre-2026, or not an exchange wallet) | Noncovered |
  | `CarriedFromTransfer`, `SelfTransferInbound`, `GiftCarryover`, `GiftFmvFallback`, `SafeHarborAllocated`, `ReconstructedPerWallet`, `EstimatedConservative` | any | Noncovered — arrived by transfer or reconstruction; no §1.6045A-1 statement comes from the filer's own wallet or from a reconstruction (an exchange→exchange transfer MAY carry one and be covered at the destination under (G); btctax cannot see it and relocates the lot to `CarriedFromTransfer` (`fold.rs:1094-1097`), so it groups Noncovered and the filer's form governs — r4 M-new-6) |
  | `FmvAtIncome`, `CardRewardRebate` (credited INTO the exchange account — staking, interest, a card rebate; FR-45) | any | **Noncovered** — not acquired "in exchange for" cash, a different digital asset, or a broker's (a)(9)(ii)(B)/(C) property or services, so (a)(16); the instructions say *"Do not report rewards and staking payments"* (`:364-366`). Its sale IS reported (box 1f), noncovered (box 9, box 2 unchecked) — which is what its Noncovered neighbours' forms show too, so the key answers `ProceedsOnly` rather than `Mixed` (r4 I-new-1) |

  There is no `Unknown` outcome and no `BrokerCohortUnknown` refusal (r3 I-3): the cohort is only a
  GROUPING the filer answers over, each key's rows are enumerated, and a lot in the "wrong" bucket
  still receives the answer its own form supports — or the bucket refuses as `Mixed`. A refusal here
  would block a filer whose truthful ledger holds a shipped provenance, with no exit but deleting it.

  The date is the lot's **acquisition date as the lot records it**, NOT `DisposalLeg::acquired_at`
  (the zone-aware holding-period start, `state.rs:208-212`). ★ It is not reachable at `form_8949`
  (r3 I-1): a lot sold out by the disposal leaves `LedgerState.lots` (`fold.rs:1644-1650` keeps only
  `remaining_sat > 0`) and the state carries no event log. So the fold sets `DisposalLeg.lot_acquired_at: TaxDate` at the leg literal
  (`fold.rs:360`) from **`Consumed.acquired_at`** (`pools.rs:321`, assigned `lot.acquired_at` at
  `:243` for both pool kinds — the `Lot` itself is not in scope there, the `Consumed` is; the
  precedent is `promoted_origins`, `state.rs:345-352`, a fact carried forward because the leg alone
  cannot show it). Every `DisposalLeg` literal reds until it is supplied (15 full literals; the 26
  `..base_leg()` struct-update sites inherit from them — r5 NEW-1, counted by brace depth). `form_8949` derives `cohort` from
  `(basis_source, wallet kind, lot_acquired_at)` and nothing else; **column (b) keeps
  `leg.acquired_at`** (`forms.rs:151`) — only the cohort reads the new field. Why not
  `leg.acquired_at` itself: today the two are equal on every row whose cohort consults a date
  (`acquired_at` = `gain_hp_start` = `donor_acquired_at.unwrap_or(lot.acquired_at)`, and gift lots sit
  on a date-free row), so nothing would red — but that equivalence is a cross-module accident that
  breaks the day a non-gift `basis_source` tacks a holding period, and a derived form needs the
  branch where it breaks named (this is it). Under a regime with
  `basis = false` every row is `Noncovered`. The row carries `cohort: Cohort`.
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
  and the gain, the holding period, and therefore possibly the PART (I vs II) and box set differ
  from what the broker's lot would give. ★ Whether that makes the return WRONG is the identification
  rule, §1.1012-1(j)(3), and for TY2026 the governing text is **Notice 2026-20**
  (`legal/text/irs-guidance/Notice_2026-20.txt`: *"EXTENSION OF TEMPORARY RELIEF UNDER SECTION
  1.1012-1(j)(3)(ii)"*, relief period *"beginning on January 1, 2025, and ending on December 31,
  2026"*): an adequate identification includes *"Recording a standing order on the taxpayer's books
  and records"* before the units are sold (§4.02(2)), and the units so identified govern
  *"regardless of whether the information reported by the broker to the taxpayer matches the
  taxpayer's books and records"* (§4.05). So on a books-and-records standing order the engine's lot
  IS the lot sold and the broker's differing form is a reconciliation matter, not an error; without
  one, §1.1012-1(j)(3)(i) treats the EARLIEST-acquired units in that broker's custody as the ones
  sold — or, where the broker offers only one specific-identification method, (j)(3)(ii)'s last
  sentence deems that method a standing order (which Notice 2026-20 §4.03 switches off when §4.02 is
  used) — and btctax's HIFO row is wrong. That is what the owner
  action T7 secures, and the per-lot import is what would detect the divergence. (Rev. Proc. 2024-28
  is the §1012(c)(1) per-wallet basis safe harbor, not the identification rule — r3 I-4.) Stated here
  so the blind spot is chosen, not discovered.
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
  for every bundled year so the constant (`forms.rs:55`, read at `forms.rs:135`, `admin.rs:526`,
  `tui/src/tabs/forms.rs:174`) cannot drift from the record. Live + a key with rows unanswered →
  **REFUSE** (`RefuseReason::BrokerReportingUnanswered { provider, cohort, year }`; port report §6
  rule 18). **Any answer on a key with NO rows, or on a year where the question is not live, →
  `Refusal`** (a declaration about nothing is fabricated; a declaration the tool would not read is
  testimony silently discarded). **`BasisMatches`/`BasisDiffers` under a regime with `basis = false`
  → `Refusal`** (the broker cannot have reported basis) — the r1 kill, restored. The second rule is
  subsumed by the first while liveness is gated on `basis`; both are kept deliberately (belt and
  braces), so the two kills are not duplicates by accident but by design (r3 M-6).
- **The crypto slice is CLOSED on a LIVE year (N-3).** `export_irs_pdf_from_session`
  (`admin.rs:642`, THE DISPATCH at `:670`) runs the crypto slice iff no `ReturnInputs` is stored, and the answers live on
  `ReturnInputs`; so for a LIVE year (basis regime AND ≥1 exchange disposition — a TY2026 vault with only
  self-custody dispositions still fills) the slice arm refuses **before any byte**, unconditionally,
  and the refusal names the exit: *"TY2026 Form 8949 needs the Form 1099-DA answers, which live on the
  return inputs — `income import` / the TUI input form, then export the full return."* Kills, all
  three directions: TY2026 + one exchange disposition on the slice arm → refusal, no `f8949.pdf`; TY2025
  same inputs → fills as today; **TY2026 + zero exchange dispositions on the slice arm → fills, no
  refusal** (the promise S10 makes, held — r4 M-new-4). This retires the slice as a product surface for TY2026+ (btctax has
  no users; the full return is the product) — recorded in `ROADMAP_STATUS.md` §0a as S10, an
  owner-visible decision; reversing it means a `broker_reporting` vault table both arms read — R6
  reverses it WITHOUT a new table: both arms read the stored answers (committed or draft row)
  through one accessor (T9).
  **★ SUPERSEDED by R6 (owner ruling 2026-09-06, "we will need to have option to file 2026 tax year
  with crypto sales"):** the slice is NOT retired for TY2026+; it files a live year from the stored
  answers when the full return cannot compute. The no-inputs refusal above stands; its exit sentence
  no longer says "then export the full return" — the slice itself fills once the answers are stored.
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
  **2026**, and its callers are the two TUIs (`tui/src/app.rs:195`, `tui-edit/src/editor.rs:300`) plus the
  unlock screen's FALLBACK behind a ledger-derived year (`tui/src/unlock.rs:234`), so **both TUIs
  open on a year with zero bundled forms** (the unlock screen only when the ledger names none) — the readiness sentence says so on the
  unlock screen. Four test sites red and are updated by name in T0: `year_readiness.rs:317`,
  `tui-edit/src/main.rs:15459`, `tests/year_record.rs:55` (`bundled_years() == [2017, 2024, 2025]`)
  and its `expected_count` `other => panic!` arm. The 2026 record flips to `filable` in the same
  commit that inserts the TY2026 `FullReturnParams` (FR-47; `YearReadiness` reds a non-filable year
  with params bundled).

**R2 — routing.** Per row, `(term, answer for the row's key)` → box: ST `NotReported→I`,
`ProceedsOnly→H`, `BasisMatches→G`; LT `→L`, `→K`, `→J`; `BasisDiffers` and `Mixed` → **refuse**
(`RefuseReason::BrokerBasisDiffers { provider, cohort }` / `BrokerReportingMixed { provider, cohort }`,
each naming the per-lot import as the exit). **A row whose wallet is not an `Exchange` has no key**: btctax models a
non-`Exchange` wallet as non-custodial (a peer-to-peer sale, a spend, self-custody), and a custodial
venue must be recorded as `exchange:PROVIDER:ACCOUNT` to get a key (`parse_wallet_id`,
`eventref.rs:58-73`, also accepts `self:LABEL` for anything the filer types — a venue recorded that way
routes as self-custody, and the live-year advisory says so once). It routes `NotReported` → I/L **by
mechanism, not by default** — the rule is written, the kill holds it (T2), and it never refuses (r3
I-2). A `BasisMatches`/
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

**R6 — the crypto slice files a LIVE year from the stored answers when the year's full return cannot
compute (owner ruling 2026-09-06, reversing S10; r1 review 2C/5I/6M/2N folded here).** THE DISPATCH
in `export_irs_pdf_from_session` (`crates/btctax-cli/src/cmd/admin.rs`, the block headed "THE
DISPATCH (P6.5)") becomes three-way. The predicate is a function call, never a declared status:
`BundledFullReturnTables::load().full_return_for(year).is_none()` (N-2).

1. **inputs stored AND `full_return_for(year)` is `Some`** → the full-return packet, unchanged. The
   `exists` branch keeps its early `return` ONLY in this case (M-2).
2. **the ANSWERS are stored AND `full_return_for(year).is_none()`** → the CRYPTO SLICE, reached by NOT
   returning early. "Stored" is ONE accessor (T9): `input_form_store::broker_answers(conn, year)`, which
   resolves the year's working return through `input_form_store::load` — the §6.1 precedence every
   other reader uses, **a draft shadows the committed row** — and returns its `broker_reporting`;
   the predicate is that it is `Some` and non-empty. The boxes are ROUTED from those answers through
   exactly the screen and router the full return uses (`screen_broker_reporting` then
   `route_8949_boxes`). Before any byte, in this order (I-5): the promote gate; the form-level gate
   (I-1/I-7) — EVERY map the SELECTED forms can reach must resolve (`--forms` narrows the set through
   `wants()`): `Form8949Map` and `ScheduleDMap` (the slice always selects them — the same principle,
   not an exemption), `Form1040Map` when the capital-gains page is selected, plus `Form8283Map` / `ScheduleSeMap` / `Form8275Map` when this year's data will
   reach them — naming the first missing stem, so a partially ported year writes
   nothing (`SUPPORTED_YEARS` is a year-level answer and is not this gate); the pseudo-
   attestation gate; `screen_broker_reporting` (an unanswered, `mixed` or `basis_differs` key, or a
   stored answer no row reads, refuses with the SAME `Refusal` reason/detail in the SLICE's own
   sentence — never "the return is not computable", which is false here (M-1)); and the Form 8283
   restriction row in its slice form — `ri.donations_had_restrictions == Some(true)` AND the year
   emits an 8283 → refuse, no `form_8283.pdf`. It does NOT run `screen_inputs` or
   `screen_compute_dependent`: neither reaches a figure the slice prints. The `--forms full-return`
   and `--pay-by-check` refusals stand on this arm (there is no Form 1040 line 37), RE-WORDED for it:
   both name the real reason — the year's full-return parameters are not bundled in this build — never
   "no full-return inputs … author them first", which a filer on this arm has already done (M-10).
   **The slice's Schedule D carries the per-box lines from the ROUTED rows (T8, below).** The export
   report carries one note, printed after the file list (M-6): *"TY{y}: full-return parameters are not
   bundled in this build — this is the crypto slice, an ATTACHMENT SET (Form 8949, Schedule D, …) with
   the boxes routed from your Form 1099-DA answers; your own Form 1040 carries it, and
   `form_1040_capgains.pdf` is a worksheet, not a return. The full return follows when the year's
   package is bundled."*
3. **no answers stored** → the slice as today: a live year with ≥1 exchange disposition refuses before
   any byte and names the exit. **Answers stored with the year's parameters bundled and NO committed
   row is also arm (3)** (M-13), and its refusal names the exit that state actually has — *commit the
   return in the TUI input form (the commit succeeds once the year's parameters are bundled), then
   export the full return* (kill: a params-bundled year, DRAFT-only answers, no committed row → the
   refusal names committing, not answering). Otherwise the exit sentence becomes *"… answer them in the TUI input form (the
   Form 1099-DA block lists your venues) or via `income import`, then export again — the crypto slice
   fills from the answers; a full return is not required."*

**`report` in state (2) (I-3, C-3).** `resolve.rs` is UNTOUCHED. State (2) has two sub-states:
(2a) the answers live in the DRAFT row only (the TUI path — no committed row exists) → `report`
resolves on the unchanged ladder: WITH a stored `tax_profile` it renders the crypto-delta report and
the Form 1099-DA answers block (the row enumeration R1 mandates, read through the T9 accessor) and
exits 0; WITHOUT one it is `NotComputable [TaxProfileMissing]` and exits 1 exactly as today — the
answers block still prints (it is built after the resolve), and the NOT-COMPUTABLE line gains the
clause *"`export-irs-pdf --tax-year {y}` still prints the crypto slice from these answers"* so the
filer is not told the year is dead — printed only when the T9 accessor returned answers AND
`full_return_for(year).is_none()`; `report_tax_year` computes both and passes the flag to
`render_tax_outcome` (M-15). A draft never shadows a `tax_profile`, so (2a) never LOSES a
figure it had. Kills: one per sub-state, naming the expected exit code (0 with a profile, 1
without); (2b) a committed row exists with no params (only `income import` can
create it on such a year) → the outcome is unchanged — uncomputable, the inputs kept — and both
`uncomputable_sentence` and `import_note` (I-4) gain the clause *"`export-irs-pdf --tax-year {y}`
still prints the crypto slice from the stored answers"* (the slice reads the working return, which
may be the draft — N-6), `import_note` no longer saying `report` will refuse when a `tax_profile` is
stored. The TUI draft is the RECOMMENDED authoring path for a
params-less year precisely because it leaves `report`, `optimize` and `what-if` on the profile.

**The TUI export (I-2).** It is CSV-only and has no full-return arm: it passes
`snap.broker_answers.get(&year)` into `write_form_csvs`, and the 1099-DA screen + route runs BEFORE
`mkdir_owner_only_exclusive`, so a refusal leaves no directory.

**Price coverage (M-5, M-7).** `YearReadiness::problems` is a static readiness report and is
UNCHANGED. A new export-time check, `year_readiness::price_coverage_or_refuse(year)` (the record's
`prices_through` against the bundled dataset's `prices_max_date`), runs in BOTH export arms before
any byte; the readiness surfaces (`sentence()`, the unlock screen, `year_record` tests) do not gain
a new problem.

**What R6 does not change.** TY2026 prints nothing until its Form 8949 and Schedule D FINAL revisions
are bundled (both forms are "unchanged" in shape on the 2026 drafts per `TY2026_WORK_LIST.md`, so
the port is two rows), Nov 2026 – Jan 2027 — earlier than and independent of `FullReturnParams`
TY2026, the i1040gi worksheets and the OTS-2026 census. A year with inputs AND parameters is the
full return, always. TY2025 + stored inputs now PRINTS on arm (2) under the proceeds-only regime
with the I/L boxes — that inherits R1's recorded I/L gap (H/K belongs to the S1 rehearsal decision)
and is not evidence the gap is closed (M-4). No new vault table: the answers stay on `ReturnInputs`
(the committed row, or the draft row the TUI keeps), read through the T9 accessor by both arms.

**New build tasks (from the r1 review's Criticals).**
- **T8 — the slice's Schedule D, per box.** `btctax_core::forms::schedule_d_by_box(rows: &[Form8949Row])
  -> BTreeMap<Form8949Box, ScheduleDPart>` (the box determines the part — A/B/C/G/H/I are Part I,
  D/E/F/J/K/L Part II — so the key is the box alone; `Form8949Part` carries no `Ord`) aggregated from the ROUTED rows (never re-
  derived from `state`, so the page-set and the schedule cannot disagree). It maps each box to the
  line the form's own text gives it — 1b = A|G, 2 = B|H, **3 = C|I**, 8b = D|J, 9 = E|K,
  **10 = F|L** — the same pairing `printed::schedule_d_lines` already encodes (`&[B::I, B::C]` /
  `&[B::L, B::F]`), so the slice and the full return cannot disagree, and a TY2017/TY2024 slice keeps
  its whole Part I total on line 3 (I-6). Signature (M-8): `fill_schedule_d_totals(totals, by_box,
  map)` and its public wrapper `fill_schedule_d(&totals, &by_box, year)`; the routed rows reach the
  filler from the arm that routed them (the slice arm in `admin.rs`, `write_form_csvs`); the twelve
  `fill_schedule_d` call sites (`admin.rs:833`, `kats.rs` ×6, `sp3.rs` ×2, `sp3b.rs` ×3 — the last
  three go with the S9 drop) AND the three direct `fill_schedule_d_totals` sites — `kats.rs:471` and
  `:488` (the two halves of `a_swapped_yes_no_map_fails_closed_instead_of_rendering_a_blank_box`, a B1
  kill) and `sp3b.rs:603` — are updated as transcription, the compiler listing them; none of the
  three B1 kills is relaxed. A box group with no rows writes nothing; **a box group
  with rows and an UNBOUND map row REFUSES (`need`), as `fill_schedule_d_full` does — never drops the
  total** (I-8). `schedule_d.csv` gains a `box` column and one row per (part, box) group, so the CSV
  and the PDF carry the same partition. The three-artifact cross-check lives where the CSV is written:
  in `btctax-cli` beside `write_form_csvs` (which holds the routed rows), reading the written
  `schedule_d.csv` from a tempdir — each box group's CSV row must equal that box's Schedule D PDF
  line and that box's Form 8949 page-set total, and the part total must equal the sum of the groups;
  `kats.rs::schedule_d_totals_match_form8949_and_csv` stays in `btctax-forms` as the PDF-vs-8949
  half, re-scoped per box group, renamed so it no longer claims to read a CSV (I-9, I-13). Kills: a live-regime
  `basis_matches` slice → `f8949.pdf` carries a G page-set, Schedule D line 1b carries the G total
  AND line 3 is BLANK; on a G+I mix each total lands on its own line and `1b_d + 3_d` cross-foots to
  the part total (M-3); a TY2024 slice → line 3 still carries the whole Part I total and 1b is BLANK;
  a map with `line1b` removed and a routed G group present → `FormsError`, zero bytes, the same map
  with no G rows → fills clean; the cli-side three-artifact check on a G+I fixture reds when the CSV writer is reverted to two part
  rows (B1), and the forms-side half reds when a PDF line moves.
- **T9 — read the answers without committing a return (C-2, C-3, C-4, I-10, I-14, I-15).** A dedicated
  accessor `input_form_store::working_return(conn, year) -> Result<(Option<ReturnInputs>, Option<StaleNote>), CliError>`
  resolves the year's WHOLE working `ReturnInputs` through `input_form_store::load` — the §6.1
  precedence every other reader uses, **a draft shadows the committed row** — and
  `broker_answers(conn, year)` is the projection of it (`.broker_reporting`) that arm (2)'s predicate
  reads. **Every arm-(2) gate that reads a `ReturnInputs` field — `screen_broker_reporting(ri, …)` and
  the Form 8283 restriction row — reads that same resolution, never `return_inputs::get`**, so no gate
  can see a different return than the one the answers came from and nobody synthesises a default. The
  accessor is called only after arm (1) is ruled out (N-7). The export prints the `StaleNote` the way
  `scrub` does, before the file list (M-14). It never
  introduces a second precedence: **every reader of `broker_reporting` on every surface goes through
  this resolution** — `Snapshot.broker_answers` (`Session::broker_reporting_answers` enumerates the
  union of `return_inputs::years` and the draft table and resolves each year through `load`), so the
  viewer's Box column, the TUI export, `btctax export --csv` and the CLI export show and file ONE
  answer set. The one deliberate exception: the full return (arm (1)) files from the COMMITTED row —
  committing is possible on a params-bundled year — and nobody re-points arm (1) at the accessor. Because
  it goes through `load`, the §6.3 stale split holds unchanged — a stale WIP draft is discarded (and
  the export surfaces the `StaleNote`), a stale parked draft REFUSES (`StaleParkedDraft`) before any
  byte — and a **parked** draft (`parked = 1`, a return switched back to the tax-profile) carries no
  live answers: the accessor returns `None` for it, so arm (2) falls to arm (3) rather than filing
  withdrawn testimony. Arm (2)'s predicate is that accessor, NOT `return_inputs::exists`. NO row is
  created and `ReturnInputs::default()` is never persisted — it carries `filing_status: Single`,
  testimony the filer never gave, which `classify()`'s own exemption reason ("no default to launder")
  forbids; `resolve.rs` is untouched, so a draft cannot shadow a `tax_profile`, and I-11 is not merely
  preserved but never approached. `income import` reaches arm (2) only with a file that carries
  `filing_status` (the one field with no `#[serde(default)]`, on purpose); the TUI block is the
  primary authoring surface on a params-less year, and its commit modal says the answers are held in
  the draft and that the slice reads them. Kills (run on TY2025's templates with the LIVE regime injected, the T-tests' pattern — TY2026 has
  no templates, so nothing is observable there, N-5): a TUI-saved 1099-DA block → the draft holds the
  answers, `return_inputs::exists` is FALSE, and the export reaches arm (2); the same vault with a stored `tax_profile` for 2026 → `report --tax-year 2026`
  still resolves `StoredProfile` and exits 0 (the kill that reds any create-a-row design); a
  params-less year with a committed row answering `basis_matches` AND a draft answering
  `proceeds_only` → the export writes box **I** (the draft wins), and the same vector with the two
  swapped writes **G** — asserted on the CLI export, `export --csv` AND the TUI export's CSV (I-15);
  a params-less live year, DRAFT-only, `donations_had_restrictions = Some(true)` and a donation that
  emits an 8283 → the export refuses and `form_8283.pdf` is absent (the test that reds on a
  `return_inputs::get` implementation, I-14); a draft at `SCHEMA_VERSION - 1` with `parked = 1` → the export refuses,
  `out_dir` absent; the same at `parked = 0` → the stale row is discarded and the export takes arm
  (3); a current `parked = 1` draft → arm (3); TY2026 + a params-less commit of the FULL return →
  still `NoTables` + draft (I-11 unmoved).

**Kills (the build lands each with its own):** TY2025 (templates, no params) + stored answers
(committed row via `income import`, and separately a DRAFT-only vault) → the slice PRINTS (today a
committed row refuses "no full-return tables for 2025"); the LIVE regime injected on the
TY2025 templates (the T-tests' pattern) + inputs with `basis_matches` → the T8 kill; same with one key
unanswered → refusal in the slice's sentence, no byte (`wrote_nothing`); a stored answer no row reads
→ the unread refusal; TY2024 + inputs → the full packet still; `--pay-by-check` on arm (2) → the I-7
refusal; the partially-ported-year refusal (a fixture year with one bundled template and no f8949
map → refusal, `out_dir` absent; the f8949 map alone → still refused, naming `schedule_d`); the 8283
restriction refusal and its `false` twin; the TUI export's two states (refusal → no directory;
answered → `form8949.csv` box G); `report` in state (2a) exits 0 with the answers block and the note when a `tax_profile` is stored,
and exits 1 with the answers block and the slice clause when none is; state (2b) is unchanged;
the two exit sentences; the price-coverage refusal on an exported year whose dataset ends early.

## Current state — hook points (recon @ c76adf6b, cites re-resolved @ 5f03b965)

| what | where | today |
|---|---|---|
| box chosen | `crates/btctax-core/src/forms.rs:136-147` (`form_8949`) | `da ? I : C` / `da ? L : F` from `term` and year |
| the flag | `Form8949Row.box_needs_review` (`forms.rs:73`, set `:149`) | `matches!(leg.wallet, Exchange{..})` → advisory only |
| the provenance | `DisposalLeg { basis_source, lot_id, wallet, acquired_at }` (`state.rs:197-217`); `BasisSource` (`event.rs:17-29`); the `Lot` in the fold (`fold.rs:1644-1650` drops sold-out lots) | `acquired_at` is the HP start; the lot's own date reaches the row only if the fold carries it (T2) |
| the advisory | `crates/btctax-cli/src/cmd/admin.rs:526` `broker_reporting_advisory` | fires on both export arms since `118b070b` |
| the two arms | `admin.rs:642` (THE DISPATCH at `:670`) `export_irs_pdf_from_session` | full return via `ReturnInputs`; slice via `form_8949` + `fill_form_8949` (`:732`, `:825`) |
| the filler | `crates/btctax-forms/src/fill8949.rs:259` `split_parts` (by PART), `:78` `place_part` (one box per page); `lib.rs:119-137` pairs ST chunk *k* with LT chunk *k* | no per-box grouping |
| the map | `crates/btctax-forms/forms/2025/f8949.map.toml:18-19, 38-39` | one box per part |
| Schedule D | `printed.rs:935-972` `ScheduleDLines` (18 `lineNN`) | no 1b/2/8b/9 lines exist |
| the year regime | `crates/btctax-forms/forms/<year>/YEAR.toml` `information_returns.f1099da` | declared; no reader; no 2026 record |
| the input surface | `crates/btctax-core/src/tax/return_inputs.rs` | `Form1099Int/Div/G`; no 1099-DA |
| the screens | `return_1040.rs:2608` `screen_absolute` (has the ledger; decides `DonationRestrictionsUnresolved`); `return_refuse.rs:989` `screen_inputs` (inputs only — cannot see dispositions) | the non-declaration pattern to reuse |
| `testonly` | `testonly.rs:34-40` `answer_all_live_declarations` | auto-answers every live `FORM_QUESTIONS` entry with `neutral` |
| provider key | `crates/btctax-adapters/src/normalize.rs:64` `exchange_wallet(source)` | `provider = source.tag()`, `account = "default"` |

## Plan (TDD — every task lands with its kill)

- **T0 — the 2026 record + the regime value.** `forms/2026/YEAR.toml` (preparing; regime
  `{true,true}`); `InformationReturnRegime` value type in core; the CLI/TUI join `YearRecord → regime`
  with its kill (2024/2025/2026); the constant-vs-regime kill; the `default_year` → 2026 consequence
  with the four tests updated by name.
- **T1 — the declaration.** `BrokerReporting` on `ReturnInputs` (nested map); the `screen_absolute`
  arm and its `regime` argument; the classifier class; `RefuseReason::{BrokerReportingUnanswered,
  BrokerReportingMixed, BrokerBasisDiffers}` (an exhaustive cross-crate match reds). Kills: TY2026 + one exchange disposition + no answer → refuse; TY2025 same inputs → not
  refused, I/L; TY2024 → not asked; an answer for a key with no rows → `Refusal`; any answer on a
  non-live year → `Refusal`; `BasisMatches` under `basis = false` → `Refusal`;
  `answer_all_live_declarations` leaves it unanswered; the slice arm: TY2026 + one exchange disposition → refusal before any byte naming the exit;
  TY2025 → fills; TY2026 + zero exchange dispositions → fills, no refusal (r5 NEW-2).
- **T2 — cohort + routing.** `DisposalLeg.lot_acquired_at` set at the leg literal from `Consumed.acquired_at`
  (every full literal reds); `form_8949` derives `cohort` per row from the table in R1 and takes the answers; the
  routing table `(term, answer) → box | refuse` under test for every cell; `FmvAtIncome` in an
  exchange wallet dated 2026 → `Noncovered` (r4 I-new-1). Kills: one provider, two cohorts, two different answers →
  two Part I page-sets with different boxes (S4); a `Mixed` answer → refusal naming the import (C-1);
  the cohort of a sold-out 2026 exchange-purchased lot is `Covered` (I-1 — reds if the row's date came from `state.lots`, which no longer holds the lot); TY2026, one self-custody disposition
  and one answered exchange disposition → the self-custody row prints I/L and nothing refuses (I-2).
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
  "1099-DA regime: proceeds+basis"; the advisory at `admin.rs:526` and the TUI forms tab
  (`tabs/forms.rs:174`) read the regime, not the constant, and on a live year the advisory states once
  that a custodial venue outside the four adapters must be recorded as `exchange:PROVIDER:ACCOUNT` to
  get a 1099-DA key (r5 NEW-3).
- **T7 — the owner action (not code).** Under Notice 2026-20 §4.02(2): **record the standing
  lot-identification order (the method election, HIFO) on the filer's own books before further 2026
  sales** — btctax's per-year method-election record, timestamped in the vault, is a candidate for
  that record and T6 states whether it qualifies as written — **and/or** instruct each exchange; then
  check what each will put in **box 1g** (strategy review S3 — which says "box 1e"; that report is
  verbatim and is corrected here, not there). Recorded in `ROADMAP_STATUS.md` §0a as an owner item
  with a date, citing the notice.

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
