# RECON — FR-111: the Form 8949 box truth table, re-derived from source

**Scope:** `crates/btctax-cli/LIMITATIONS.md:415-417`'s claim ("Form 8949 Box I/L checked on every
Form 8949 with rows … btctax has no 1099-B / 1099-DA input at all … Never Box C/F") is re-derived
from source, not reworded. Read-only recon; no edits made. Every fact below is grep/read verified
against HEAD; commands are listed in full under "Commands run".

---

## Q1 — box truth table

All four call sites (`packet.rs:1108`, `admin.rs:1086`, `render.rs:1223`, `tabs/forms.rs:119`) call
the **same** `route_8949_boxes` (`forms.rs:139`) with a `regime: InformationReturnRegime` that is
**always** joined from `forms/<year>/YEAR.toml` via `year_readiness::regime_for` (`year_readiness.rs
:386-392`) — never a per-surface literal. So the box-selection LOGIC is identical everywhere; the
four call sites differ only in what they do with an `Err` (refuse export / refuse the slice / show
`—` with a caption), not in which box a settled row gets.

**`broker_question_is_live(rows, regime)`** (`forms.rs:127-130`): `regime.basis && rows.iter().any(
|r| broker_key(r).is_some())` — i.e. the year's regime reports **basis** (not merely proceeds) AND
at least one row is on an `Exchange` wallet. On a year that is **not live**, `route_8949_boxes`
returns `Ok(())` on its first line (`forms.rs:143-145`) **without touching a single row** — the
rows are left exactly as `form_8949()` built them (C/F pre-2025, I/L from 2025), and any stored
`broker_reporting` answers are **silently ignored** (never read, never erred on) by this function.
(A *separate* screen, `return_refuse::screen_broker_reporting`, is what turns a stored-but-unread
answer into a loud refusal at compute time — see the "Earlier years…" finding in Q3.)

**`InformationReturnRegime` per bundled year**, read directly from `forms/<year>/YEAR.toml`
(`information_returns.f1099da`):

| year | `proceeds` | `basis` | source |
|---|---|---|---|
| 2024 | `false` | `false` | `forms/2024/YEAR.toml:43-44` |
| 2025 | `true` | `false` | `forms/2025/YEAR.toml:43-44` |
| 2026 | `true` | `true` | `forms/2026/YEAR.toml:49-50` |

These are the only three bundled year records at HEAD (`crates/btctax-forms/forms/{2024,2025,2026}
/YEAR.toml`). `DIGITAL_ASSET_8949_FIRST_YEAR = 2025` (`forms.rs:188`) is the box-revision constant;
a CLI kill (`year_readiness.rs:758` `the_constant_and_the_regime_agree_on_every_bundled_year`) holds
`regime.proceeds == (year >= 2025)` for every bundled year, matching the table above.

**Box truth table:**

| box | year(s) | condition | source (file:line) |
|---|---|---|---|
| **C** | year < 2025 (TY2024 bundled) | ST leg, `da = year >= 2025` is false; `route_8949_boxes` never runs (not live: `regime.basis` is false pre-2026) so the row stays as built. Unconditional for every ST crypto-ledger disposal in the year. | built: `forms.rs:433-434`; left alone: `forms.rs:143-145`; KAT: `kat_forms.rs:121,152` (`pre_2025_st_leg_is_box_c_and_lt_leg_is_box_f`) |
| **F** | year < 2025 (TY2024 bundled) | LT analogue of C. Unconditional for every LT crypto-ledger disposal in the year. | `forms.rs:437-438`; `kat_forms.rs:121,153` |
| **I** | year ≥ 2025 | (a) any row with **no broker key** (non-`Exchange` wallet — self-custody) — I/L "by mechanism", routing `continue`s past it (`forms.rs:147-149`); OR (b) an `Exchange`-wallet ST row on a year that is **not live** (TY2025: `basis=false`) — stays as built; OR (c) an `Exchange`-wallet ST row on a **live** year (TY2026+) where the filer answered `BrokerReported::NotReported` for that (provider, cohort) key. | built: `forms.rs:433-434`; not-live passthrough: `forms.rs:143-145`; live routing: `forms.rs:158-166`; KATs: `kat_forms.rs:84,115` (TY2025), `kat_forms.rs:159,190` (TY2026, `>=` boundary), `kat_broker_reporting.rs:336-363` (`every_cell_of_the_routing_table`, `LIVE = PROCEEDS_AND_BASIS`) |
| **L** | year ≥ 2025 | LT analogue of I, same three sub-conditions. | `forms.rs:437-438`, `forms.rs:158-166`; same KATs |
| **G** | live year only (`regime.basis == true`; only TY2026 among bundled years) | `Exchange`-wallet ST row, question live (≥1 exchange row this year), filer answered `BrokerReported::BasisMatches` for that (provider, cohort) key. | `forms.rs:168-173`; KAT `kat_broker_reporting.rs:339-340,417-420` |
| **H** | live year only | `Exchange`-wallet ST row, live, filer answered `BrokerReported::ProceedsOnly`. | `forms.rs:161-165`; same KATs |
| **J** | live year only | LT analogue of G. | `forms.rs:172-173`; same KATs |
| **K** | live year only | LT analogue of H. | `forms.rs:164-165`; same KATs |

Unanswered key, `Mixed`, or `BasisDiffers` on a live year is a hard `Err` (`BrokerRouteError`,
`forms.rs:141,152-156`) — **no box is ever printed** for that row; every one of the four call sites
propagates this as a refusal (export/slice) or a `—` placeholder (TUI), never a guessed letter.

**Net result, the two years the brief asked about:**

- **TY2024** (the only year `LIMITATIONS.md`'s own header claims is supported — see Q3-1): every
  crypto-ledger Form 8949 row is **Box C (ST) or Box F (LT), unconditionally**. **I and L never
  appear in TY2024** — `route_8949_boxes` cannot be reached with `da=true` for a 2024 disposal
  (`year >= 2025` is false), and it is a no-op anyway since TY2024's regime is not live.
- **TY2025**: every crypto-ledger Form 8949 row is **Box I (ST) or Box L (LT), unconditionally**
  — `da=true` (year ≥ 2025) but `regime.basis=false` (TY2025 is proceeds-only, per
  `forms/2025/YEAR.toml:43-44`) so the question is never live and `route_8949_boxes` is a no-op.
  **G/H/J/K cannot appear on a TY2025 return in this build** — there is no bundled year with
  `basis=true` and `year < 2026`.

**Bonus fact, outside the brief's two named years but directly load-bearing for how "live" the G/H/
J/K path is in this build today:** TY2026 (`basis=true`, live) has a `YEAR.toml` record and a
bundled `TaxTable` (`tax_tables.rs:758 fn ty2026()`), so `report --tax-year 2026` and the CSV/TUI
`form8949.csv` paths (`render.rs:1223`, `tabs/forms.rs:119`) **can already reach G/H/J/K today** for
a ledger with TY2026 dispositions and answered `broker_reporting`. What TY2026 does **not** have is
any bundled **PDF template** — `forms_expected = []` in `forms/2026/YEAR.toml:16`, `f8949` is listed
under `[forms_absent]` ("draft archived; January 2027 package") — so `export-irs-pdf --tax-year
2026` cannot print a PDF at all yet (`TEMPLATE_YEARS` excludes 2026, `bundled.rs:144-150,177-181`;
the man page's "this build bundles TY2024 and TY2025" is about `TEMPLATE_YEARS`, and is accurate).

---

## Q2 — export advisory

**Source:** `broker_reporting_advisory` (`crates/btctax-cli/src/cmd/admin.rs:715-736`), called from
exactly one production site: `crates/btctax-cli/src/main.rs:1089-1094`, with `report.regime` and
`report.broker_reported_rows` — the **same** `regime` that was threaded into the report's own
`route_8949_boxes` call (`packet.rs:1108`, inside `assemble_printed_forms`, which the `report`
pipeline calls). It is NOT called from `admin.rs:1086`'s function directly, but that function and
`main.rs`'s report path share the identical `regime` value per tax year (both derived from
`year_readiness::regime_for(year)`), so the two can never observe different regimes for the same
year.

**Exact firing condition** — `broker_reporting_advisory(tax_year, regime, broker_reported_rows)`:
1. `broker_reported_rows == 0` → `None` (no advisory at all).
2. `regime.basis == true` (**live** year) → a **different** message (`admin.rs:722-724`): *"⚠ [I5]
   {n} disposition(s) occurred on a venue that issues Form 1099-DA for TY{year}; each was filed
   under the Form 8949 box your answer chose (I/L where you answered that nothing was reported, H/K
   where only proceeds were, G/J where basis was)…"* — this is the branch that fires when G/H/J/K
   could actually be on the return.
3. `regime.basis == false && regime.proceeds == true` (TY2025) → `("1099-DA", "Box G/H/J/K", "Box
   I/L")` and the quoted sentence: *"⚠ [I5] {n} disposition(s) occurred on an exchange that MAY have
   issued 1099-DA broker basis reporting — those would belong on a SEPARATE Form 8949 under Box
   G/H/J/K. This export files EVERY Bitcoin row under Box I/L (not-reported default) and says so;
   reclassify by hand if you received a 1099-DA."* (`admin.rs:729-736`) — this is the one quoted in
   `docs/examples/examples.md:135`.
4. `regime.basis == false && regime.proceeds == false` (TY2024) → the `("1099-B", "Box A/B (ST) /
   D/E (LT)", "Box C/F")` variant (`docs/examples/examples.md:983`).

**Answer: NO, this advisory cannot print on a return whose rows were actually routed to G/H/J/K.**
Branch 3 (the "Box I/L (not-reported default)" wording) requires `regime.basis == false`. Reaching
G/H/J/K requires `broker_question_is_live` == true, which requires `regime.basis == true`
(`forms.rs:127-129`). Those two conditions are mutually exclusive on the SAME `regime` value — the
function cannot be in branch 3 and have routed a row to G/H/J/K in the same call, because branch 2
(the live-year wording) would have fired instead. **This is a correction to the brief's implied
premise**: the advisory string shown in `docs/examples/examples.md:135` is not a live filer-facing
falsehood — it is machine-generated (a golden gated by `examples_golden.rs`'s `regen == committed`
test, per `xtask/src/examples.rs:5-6,61`) from the TY2025 (`proceeds-only`, not-live) regime, on
which G/H/J/K are provably unreachable per Q1. The advisory function is already regime-aware and
internally consistent with the router.

---

## Q3 — stale filer-facing claims

### Q3-1 — `crates/btctax-cli/LIMITATIONS.md:415-417` (the central claim; already flagged as FR-111)

> - **Form 8949 Box I (short-term) / Box L (long-term)** — checked on every Form 8949 with rows. These mean
>   "transactions NOT reported to you on Form 1099-B." btctax has no 1099-B / 1099-DA input at all, so every
>   ledger disposition is un-reported by construction. Never Box C/F.

This bundles **three independently false** statements:

1. **"btctax has no 1099-B / 1099-DA input at all"** — FALSE on both halves.
   - **1099-B:** `Form1099B` (`return_inputs.rs:654-684`) is a real, working input, populated via
     `income import`'s `[[b_1099]]` table (`document_census.rs:1018,1127`, `provenance.rs:1142`) and
     reachable through the full interview/classifier/scrub machinery (`classifier.rs:122,309,1435
     -1456`, `provenance.rs:129,404-407`, `scrub_axis.rs:24,336,514,769`). **Correction to a premise
     in `FOLLOWUPS.md` FR-111 / `CONTINUITY.md`'s framing**, not one of the six settled facts:
     `Form1099B` does **not** make the Form 8949 box choice "live again" the way those notes imply —
     it is, by IRS design, a **Schedule D line 1a/8a aggregate SUMMARY** for covered transactions
     with basis reported and no adjustment (Schedule D "Exception 1/2"), and `form_8949()` reads only
     `state.disposals` (the bitcoin ledger) — it never reads `ri.b_1099` at all
     (`document_census.rs:31,278,288,644`: *"a `[[b_1099]]` row is the Schedule D line 1a/8a SUMMARY…
     zero `[[b_1099]]` rows IS the correct"*). So a 1099-B entry never produces **any** Form 8949 row,
     of any box — its falsity is that the *input* exists, not that it changes box routing.
   - **1099-DA:** `BrokerReporting`/`CohortAnswers`/`BrokerReported` (`forms.rs:305-360` and
     `return_inputs.rs:2192`) is real 1099-DA-adjacent input (the filer's testimony about what a
     1099-DA showed), entered via `income import`'s `[broker_reporting.<provider>]` TOML table
     (`cli.rs:472-479`, parse KAT `tax.rs:1898-1940`). Unlike `Form1099B`, this input **does** drive
     Form 8949 box selection — it is the entire mechanism behind `route_8949_boxes` (Q1).
2. **"so every ledger disposition is un-reported by construction … Never Box C/F"** — FALSE
   independently of (1), on pure box-mechanics grounds. For TY2024 — the *only* year
   `LIMITATIONS.md`'s own header (`LIMITATIONS.md:3`, "Tax year supported: TY2024 only") claims is
   supported — every crypto-ledger disposition is **Box C or Box F, never I or L** (Q1 table; KAT
   `kat_forms.rs:121-153`). The claim gets the box pair for the one supported year exactly backwards.
3. **"checked on every Form 8949 with rows"** — even setting TY2024 aside, false in general: from a
   live year (TY2026+ in this build) a row can be G, H, J or K (Q1), never I or L, whenever the
   filer answered `BasisMatches` or `ProceedsOnly` for its (provider, cohort) key.

**Refutation (machine-checkable):** `kat_forms.rs::pre_2025_st_leg_is_box_c_and_lt_leg_is_box_f`
(line 121, assertions at 152-153) — a TY2024 disposal is Box C/F. `kat_broker_reporting.rs::
every_cell_of_the_routing_table` (line 336) — a live year routes to G/H/J/K from stored answers.
`return_inputs.rs:654,2192` — both input types exist as typed fields.

### Q3-2 — `crates/btctax-cli/LIMITATIONS.md:25-33` ("Income:" — Supported section), omission tied to the same root cause

The "Supported: what a v1 full return covers" → **Income:** list (`LIMITATIONS.md:31-33`) enumerates
W-2, 1099-INT, 1099-DIV, 1099-G, and crypto capital gains/ordinary income — **it never mentions
1099-B (brokerage stock sales) at all**, even though `Form1099B` is a real, working, filed-with
input (per the git log, `bdde6ac8 journey(walk 2): a single filer with 24 stock lots FILES`). This
is not a separate mechanism from Q3-1 — it is the same missing-feature blind spot showing up as an
omission from the "what this tool supports" list rather than as an explicit false sentence. Flagged
because the brief asked for every claim "FALSE or MISLEADING… about whether btctax accepts Form
1099-B / 1099-DA input", and a "Supported" list that omits a working, filed-with income source is
misleading in exactly that sense, in the document whose stated job (per FR-111's framing) is to say
truthfully what the tool can and cannot do.

### Q3-3 — `crates/btctax-cli/src/cli.rs:479` — "Earlier years neither ask nor accept them" (minor / ambiguous, flagged for completeness)

The `Import` subcommand's doc comment (`cli.rs:472-479`, which also generates `docs/man/btctax
-income-import.1`) reads: *"…`report --tax-year` lists the keys the ledger needs answered and the
rows under each; an answer stored for a key no row carries is refused as unread, and a missing one
refuses the return until given. **Earlier years neither ask nor accept them.**"*

Read literally as "the `income import` command will not **accept** (parse/store) a `[broker_
reporting.*]` table on an earlier year," this is **false**: `parse_return_inputs_toml` has no
year-gate on `broker_reporting` (the parser test `tax.rs:1898` parses the table with no year field
at all), and `cmd/tax.rs`'s `income import` handler (`tax.rs:260-293`) stores the row unconditionally
via `return_inputs::set` (line 292) — the only screen it runs before storing is `screen_param_free`
(`tax.rs:281`), whose destructure explicitly ignores `broker_reporting` as a non-money field
(`return_refuse.rs:1046` `broker_reporting: _,`). A `[broker_reporting.coinbase] covered =
"basis_matches"` table on a TY2024/TY2025 import is **accepted and stored** at import time; it is
only later, at **compute/export** time, that `screen_broker_reporting`
(`return_refuse.rs:1734-1780`, `RefuseReason::BrokerAnswerUnread`) refuses it as testimony "the tool
would never read."

Read charitably — "accept" meaning "act on" rather than "ingest" — the sentence is defensible (no
interactive prompt ever asks for it on an earlier year, per Q3-4, and no earlier-year answer is ever
routed into a box). Given the ambiguity and the comparatively low stakes (the refusal downstream is
loud and correctly worded), this is reported as a **minor / ambiguous** finding, not ranked with
Q3-1.

### Q3-4 — `LIMITATIONS.md:281-283` — CONFIRMED CORRECT, not a stale claim

> - **Form 1099-DA answers are a keystroke, not a transcription.** … There is no 1099-DA entry screen
>   in this version, and btctax never answers it for you.

Checked per the brief's instruction not to assume. `broker_reporting` has **no writer anywhere in
`btctax-tui`** — grep across `crates/btctax-tui/src` finds only readers (`app.rs:130-131`,
`unlock.rs:206-209`, `tabs/forms.rs:39-90` — all display/routing, none an editable widget). The
CLI's own `income answer` subcommand doc (`cli.rs:657-668`) enumerates exactly what it asks
interactively — dependent-claim, Schedule B foreign-account/trust, dates of birth — and does **not**
list `broker_reporting` among them; no interactive prompt for it exists in `cmd/tax.rs`. The only
entry path is the TOML `[broker_reporting.<provider>]` table via `income import`. **This settled
fact (#5 in the brief) is confirmed correct, not refuted.**

### Sweep coverage (no further hits)

- `README.md:186-187,237-238` — year-aware and correctly matches source (`da`/`regime.basis`
  behavior); no false claim.
- `docs/man/btctax-export-irs-pdf.1` (only man page mentioning boxes/1099-B/1099-DA) — correctly
  year-aware, matches `cli.rs`'s doc comment it is generated from (`docs.rs:277-383` gates man pages
  against a regeneration diff); TY2024→C/F, TY2025→I/L stated explicitly and correctly.
- `crates/btctax-cli/src/cli.rs:165-167,191,214-215,472-479` — all year/regime-aware and correct
  except the ambiguous line flagged in Q3-3.
- `crates/btctax-tui/src/tabs/forms.rs` (footnote/caption strings at lines 62-90, 274-323, 388-421)
  — every caption is regime-derived (`Some(r) if r.basis`, `Some(r) if r.proceeds`, `None`), matches
  `route_8949_boxes`'s own gating exactly; one is a KAT-named historical bug ("the filer was told to
  find G/J rows on a table that showed none") that is now fixed and pinned by a kill test
  (`tabs/forms.rs:388-421`).
- `docs/examples/examples.md` — machine-generated golden (`xtask/src/examples.rs`), gated by a
  regen-equals-committed test; both box-advisory lines present (I5 at 135, 983) are consistent with
  their generating source per Q2.
- No other `docs/man/*.1` file mentions "1099-B", "1099-DA", or any Form 8949 box letter.

---

## Corrections to the brief

None of the six settled facts in the brief were found wrong. One clarification, already folded into
Q3-1 above rather than repeated here: **`Form1099B` existing does not itself reopen the Form 8949 box
question** — that's `BrokerReporting`'s job, not `Form1099B`'s. `CONTINUITY.md`'s FR-111 resume note
and `FOLLOWUPS.md`'s FR-111 entry both frame the re-derivation as "if a 1099-B with basis reported
can now be entered, the Form 8949 box choice is live again and Box C/F is the reported-to-you case"
— that framing conflates the two input types. `Form1099B` rows never reach `form_8949()` at all (by
IRS design: Schedule D Exception 1/2 aggregate reporting means no 8949 listing is required for those
transactions). The actual mechanism that reopens the box question is `BrokerReporting` (1099-DA),
already named as settled fact #2, and it only goes live from TY2026 in this build (Q1).

---

## Commands run

```
sed -n '380,470p' crates/btctax-core/src/forms.rs
sed -n '80,300p' crates/btctax-core/src/forms.rs
grep -n "DIGITAL_ASSET_8949_FIRST_YEAR" crates/btctax-core/src/forms.rs
find /scratch/code/bitcoin_tax -path '*/forms/*/YEAR.toml'
grep -rn "f1099da" --include=YEAR.toml -A4 /scratch/code/bitcoin_tax
sed -n '1,40p;260,300p;395,430p' crates/btctax-cli/LIMITATIONS.md
grep -rln "TY2024\|TY2025\|TY2026\|full_return_for" crates/btctax-cli/LIMITATIONS.md
grep -n "Tax year supported" crates/btctax-cli/LIMITATIONS.md
grep -rn "fn.*box_c_f\|fn.*box_i_l\|assert.*Form8949Box::C\|assert.*Form8949Box::I" \
  crates/btctax-core/src/forms.rs crates/btctax-core/tests/
sed -n '100,200p' crates/btctax-core/tests/kat_forms.rs
sed -n '1080,1140p' crates/btctax-cli/src/cmd/admin.rs
sed -n '1090,1150p' crates/btctax-core/src/tax/packet.rs
sed -n '1195,1250p' crates/btctax-cli/src/render.rs
sed -n '90,140p' crates/btctax-tui/src/tabs/forms.rs
grep -rn "not-reported default\|reclassify by hand\|Box I/L\|files EVERY Bitcoin row" \
  crates/btctax-cli/src crates/btctax-core/src crates/btctax-tui/src docs
sed -n '100,150p;900,1000p' docs/examples/examples.md
grep -n "examples.md\|examples_md\|regenerat" crates/xtask/src/*.rs
grep -n "fn \|examples.md\|golden\|assert" crates/xtask/src/examples.rs
grep -n "broker_reporting_advisory(" crates/btctax-cli/src/cmd/admin.rs
sed -n '1020,1095p' crates/btctax-cli/src/cmd/admin.rs
grep -n "broker_reporting_advisory(tax_year" -A3 -B15 crates/btctax-cli/src/cmd/admin.rs
sed -n '1,60p' docs/man/btctax-export-irs-pdf.1
grep -n "fn bundled_years\|pub const BUNDLED" crates/btctax-forms/src/bundled.rs
sed -n '1195,1250p' crates/btctax-cli/src/render.rs
grep -n "1099-B\|1099-DA\|Box C\|Box F\|Box I\|Box L\|Box G\|Box H\|Box J\|Box K" README.md
grep -rln "Box C\|Box F\|Box I\|Box L\|Box G\|Box H\|Box J\|Box K\|1099-B\|1099-DA" docs/man/*.1
grep -rn "1099-B\|1099-DA\|Box C\|Box F\b\|Box I\b\|Box L\b\|Box G\b\|Box H\b\|Box J\b\|Box K\b" \
  crates/btctax-tui/src --include='*.rs'
grep -n "1099-B\|1099-DA" crates/btctax-cli/LIMITATIONS.md
grep -rln "broker_reporting" crates/btctax-tui/src --include='*.rs'
grep -rn "broker_reporting\|BrokerReported\|CohortAnswers" crates/btctax-tui/src/*.rs \
  crates/btctax-tui/src/**/*.rs
grep -rn "broker_reporting" crates/btctax-cli/src/cmd/tax.rs
sed -n '1885,1945p' crates/btctax-cli/src/cmd/tax.rs
grep -n "fn import_note" -A 40 crates/btctax-cli/src/year_readiness.rs
sed -n '260,300p' crates/btctax-cli/src/cmd/tax.rs
grep -n "broker_reporting" crates/btctax-cli/src/cmd/*.rs crates/btctax-core/src/tax/return_refuse.rs
sed -n '1720,1810p' crates/btctax-core/src/tax/return_refuse.rs
sed -n '1010,1050p' crates/btctax-core/src/tax/return_refuse.rs
grep -rn "\"answer\"\|IncomeCmd::Answer\|Answer {" crates/btctax-cli/src/cli.rs
sed -n '640,690p' crates/btctax-cli/src/cli.rs
grep -rn "Never Box C/F\|checked on every Form 8949\|btctax has no 1099-B / 1099-DA input at all" \
  /scratch/code/bitcoin_tax --include='*.md' --include='*.rs' --include='*.1'
sed -n '6860,6920p' FOLLOWUPS.md
sed -n '1,60p' CONTINUITY.md
grep -n "b_1099\b" crates/btctax-core/src/tax/*.rs crates/btctax-core/src/forms.rs
grep -n "struct Form1099B" -A 30 crates/btctax-core/src/tax/return_inputs.rs
grep -n "broker_reported_rows" crates/btctax-core/src/tax/*.rs crates/btctax-cli/src/*.rs
grep -n "^fn \|assert_eq!(st_row.box_\|assert_eq!(lt_row.box_" crates/btctax-core/tests/kat_forms.rs
grep -n "^fn \|Form8949Box::G\|Form8949Box::H\|Form8949Box::J\|Form8949Box::K\|route_8949_boxes(" \
  crates/btctax-core/tests/kat_broker_reporting.rs
grep -n "const LIVE" crates/btctax-core/tests/kat_broker_reporting.rs
sed -n '28,50p' crates/btctax-cli/LIMITATIONS.md
grep -n "1099-B\|b_1099\|Form1099B" docs/man/btctax-income-import.1
```
