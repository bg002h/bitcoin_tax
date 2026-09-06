# Build review — 1099-DA steps T0 / T2-a / T1-a / T1-T2-c against `SPEC_1099da_broker_reporting.md` (GREEN r6)

- **Reviewer:** independent read-only build reviewer (Opus 5, 1M), no subagents, no edits outside this file
- **Date:** 2026-09-06
- **Repo / branch / HEAD:** `/scratch/code/bitcoin_tax`, `main`, `612e0493`
- **Commits reviewed:** `e4b80fda` (T0), `ee02e503` (T2-a), `36d83f13` (T1-a), `249d37ad` (T1/T2-c)
- **Reviewed state:** every source claim below was read with `git show 249d37ad:<path>`, **not** from the
  working tree — see N-3.
- **Question answered:** do these four commits implement R1, R2, R4 (the "never prints a code B or a 0"
  half) and the T0/T1/T2 kill lists faithfully, with no silent default, no refusal that does not refuse,
  and no kill that cannot fail? T3–T7 absence is out of scope and is not reported.
- **Verdict: 0 Critical / 3 Important / 3 Minor / 3 Nit.**

## Machine checks run

One scoped run, once:

```
cargo nextest run --locked -p btctax-core -p btctax-forms -p btctax-cli \
  -E 'binary(kat_broker_reporting) | binary(broker_boxes) | test(broker) | test(regime) | test(cohort) | test(slice_broker)'
```

`44 tests run: 44 passed, 2153 skipped` (`/tmp/claude-1000/-scratch-code-bitcoin-tax/build-c-review.txt`).
**Caveat (N-3): the tree was dirty when this ran**, so the 44 measure the working tree, not `249d37ad`.

The routing table was additionally checked against the primary source, not against the spec's
transcription of it: `design/forms/extract/i8949--2025.txt:384-417` (Box A **or G** = ST reported *with*
basis; B **or H** = ST reported *without*; C **or I** = no 1099-DA, *"Do not use box C … Use box I"*) and
`:436-466` (D **or J** / E **or K** / F **or L**, the long-term mirrors). Every cell of
`route_8949_boxes` agrees with that text.

---

## Lens answers

### (A) THE SCREEN — `screen_broker_reporting` (`return_refuse.rs:888`) vs R1 — **PASS**

**Liveness.** `broker_question_is_live(rows, regime) = regime.basis && rows.iter().any(|r| broker_key(r).is_some())`
(`forms.rs:89`), and `broker_key` is `Some` only for `WalletId::Exchange` (`forms.rs:80-87`). That is
exactly R1's "the regime says `f1099da.basis` AND the year has ≥1 disposition on an exchange". Right.

**Every refusal R1 lists fires**, and each has a kill in `kat_broker_reporting.rs`:

| R1 clause | code | kill |
|---|---|---|
| unanswered key on a live year | `None =>` arm, `return_refuse.rs:929-943` | `a_live_year_with_an_unanswered_key_refuses_and_names_the_exit` (and the *second* key after the first is answered) |
| an answer for a key with no rows | `!has_rows` disjunct, `:920` | same test file, the `gemini` case |
| any answer on a non-live year | `!live` disjunct, `:920` | the TY2025 case **and** the `only_cold` case (basis regime, zero exchange rows) |
| `BasisMatches`/`BasisDiffers` under `basis = false` | subsumed by `!live`; the `why` branch at `:922-926` prints *"proceeds only (no basis)"* | `an_answer_nothing_would_read_refuses` plants `BasisMatches` under `PROCEEDS_ONLY` — the r1 kill, present |
| `Mixed` | `:944-955` | `mixed_and_basis_differs_refuse_naming_the_import` |
| `BasisDiffers` | `:956-967` | same |

R1 asked that the "belt and braces" pair be **two kills**, not two code branches; both kills exist, so
folding the second rule into the `!live` disjunct is faithful.

**Determinism.** `keys_with_rows` is a `BTreeMap`, `ri.broker_reporting.0` is a `BTreeMap`, and the
function returns the first refusal in key order — the message is stable.

**Order.** It runs FIRST in `screen_absolute` (`return_1040.rs:2619`, immediately after the signature).
Defensible: it is the only screen whose remedy is *fetch a physical document*, which has the longest
lead time, so front-loading it is the right prompt. One mild consequence, recorded but not a finding: on
a **non-live** year a stray answer (`BrokerAnswerUnread`) now preempts every other refusal, so a filer
with both a stray answer and a genuinely blocking problem is told about the stray answer first. The
remedy is a one-line TOML deletion and the real refusal then surfaces, so this is not a regression worth
a change.

**Is the refusal text honest?** It names ``income import`` with ``[broker_reporting.{provider}] {slot} =
"not_reported" | "proceeds_only" | "basis_matches" | "basis_differs" | "mixed"``. Confirmed real:
`import_return_inputs` (`cmd/tax.rs:49`) parses with `parse_return_inputs_toml` (`:73`), and
`broker_reporting_toml_shape_parses_and_a_misspelt_slot_is_named` proves that exact TOML parses, that a
misspelt slot is refused naming `broker_reporting.coinbase.covred`, and that an unknown answer word is
refused rather than defaulted. Import is a whole-blob upsert whose only preservation arms are the four
carryovers, so a re-import that omits the block drops the answers — which fails **closed** into the
unanswered refusal. Correct.

### (B) THE ROUTER — `route_8949_boxes` (`forms.rs:98`) vs R2 — **PASS**

Every cell matches i8949 verbatim (table above): `NotReported → I/L`, `ProceedsOnly → H/K`,
`BasisMatches → G/J`, `Mixed`/`BasisDiffers`/unanswered → `BrokerRouteError`, never a box.

**"Not live → rows untouched" is safe.** A non-live year cannot reach the router carrying answers,
because `screen_broker_reporting` refuses *any* answer on a non-live year and it runs before the packet
on both production paths (`admin.rs:1019` before `:1038`; `cmd/tax.rs:525` before `:543`, in the `None`
arm of the same match). It is also safe *if* it did: the early return leaves the rows exactly as
`form_8949` built them, which is the correct box for that year. Belt and braces, both real.

Critically, the gate and the router compute liveness from **the same rows**: `screen_broker_reporting`
does `form_8949(state, year)` (`return_refuse.rs:896`) and `assemble_printed_forms` does
`form_8949(state, year)` (`packet.rs:586`) — same pure function, same `state`, same `year`, same
`regime` value threaded from one `regime_or_refuse` call. No divergence hole.

**Self-custody rows.** `broker_key` returns `None`, the loop `continue`s, and the box stays whatever
`form_8949` chose. On a **pre-2025** year that is `C`/`F` (`forms.rs:368,372`) and it is preserved twice
over — such a year is not live at all, and even if it were, `continue` never writes. Held by
`one_provider_two_cohorts_two_boxes_and_self_custody_by_mechanism` (which asserts the TY2025 no-op leaves
`[I, L, L]` byte-identical).

### (C) THE COHORT — `cohort_of`, `DisposalLeg.lot_acquired_at`, `fold.rs:370` — **PASS**

**`Consumed.acquired_at` is the lot's own date for both pool kinds.** `Consumed` has exactly **one**
construction site in the whole crate — `PoolSet::take_from` (`pools.rs:227-253`), verified by
`grep -rn "Consumed {" crates/btctax-core/src/` → one literal — and it sets `acquired_at: lot.acquired_at`
(`pools.rs:243`) unconditionally. Both the named-lot selection path and `consume_ordered`
(`pools.rs:197-221`) funnel through `take_from`, and the pool kind is only a `PoolKey` into
`self.pools`, so the universal pre-TRANSITION_DATE pool and the per-wallet pools are the same code.
`fold.rs:370` sets `lot_acquired_at: c.acquired_at`.

**Self-transfer lots.** `acquired_at` carries the ORIGINAL date (`fold.rs:1084`: the relocated `Lot` takes
`acquired_at: c.acquired_at`), and the mechanism dominates anyway: a relocation rewrites `basis_source`
to `CarriedFromTransfer` (or keeps `EstimatedConservative`, `fold.rs:1094-1097`) and the inbound op sets
`SelfTransferInbound` (`fold.rs:1375`). `cohort_of` returns `Covered` **only** for
`ExchangeProvided | ComputedFromCost`, so both are `Noncovered` regardless of date — which is R1's rule,
including the exchange→exchange case the spec deliberately groups Noncovered. Correct.

**Is tp11's new assertion a real discriminator?** Yes. `kat_tax.rs:912-923` runs the **real fold** and
asserts `acquired_at == 2023-03-15` (tacked donor date) while `lot_acquired_at == 2025-06-01` (receipt).
The mutation the spec was worried about — `fold.rs:370` reading `acquired_at` or `gain_hp_start` instead
of `c.acquired_at` — reds it on the second assertion. Paired with `rows_carry_the_cohort_by_mechanism`'s
`tacked` row (HP start 2020, lot date 2026, expected `Covered` **and** column (b) still 2020), the two
together pin both halves: the fold writes the lot's date, and `form_8949` reads that field and not
`acquired_at`. One kill from the spec's T2 list is nevertheless missing — see M-1.

### (D) THE PACKET — `assemble_printed_forms` (`packet.rs:587`) — **FAIL, see I-1 and I-2**

Every production caller of `assemble_printed_forms` / `assemble_printed_return` **is** preceded by
`screen_absolute` with the same `regime` — enumerated exhaustively:

| caller | screened? |
|---|---|
| `cmd/admin.rs:1038` (`export_full_return`) | yes, `:1019`, same `regime` local from `regime_or_refuse(tax_year)` at `:1017` |
| `cmd/tax.rs:543` (`report_tax_year`) | yes, `:525`, same `regime` local; the assemble is in the `None` arm of that match |
| `btctax-oracle-harness/src/main.rs:765` | yes, `:747`; and `const YEAR = 2024` with `InformationReturnRegime::NONE`, so the router is a no-op there |
| TUI / TUI-edit | **no such caller exists** — grep for `assemble_printed_forms\|assemble_printed_return\|fill_full_return` over `btctax-tui{,-edit}/src` returns nothing; the only production `fill_full_return` is `admin.rs:1101` |

So the panic backstop is never reachable through a screened path, which is what it is for. **The panic
is the right shape, but it is the wrong thing to worry about**: the routed box is discarded one line
later (`form_8949_printed`, `printed.rs:81-123`, builds a `Printed8949Row` that has **no `box_` field**
at this commit), so on the packet path the routing has **no reader at all** and the full-return filler
checks the map's hard-coded per-part box. That is I-1.

### (E) THE SLICE ARM — `admin.rs:637` — **PASS on ordering, see M-2 on the message**

`slice_broker_refusal` runs at `:637`, before `mkdir_out` at `:702` and before every byte-writing call
(`write_basis_methodology_txt`, `write_form_8275_txt`, the fills). It also runs before the
`SUPPORTED_YEARS` `UnsupportedYear` check at `:701`, so on a live year the broker refusal is what the
filer sees. Verified by reading the function top-to-bottom, not from the comment. The predicate is pure
and planted in all five directions (`slice_broker_tests`): live refuses and names the exit, TY2025 fills,
TY2024 fills, self-custody-only fills (S10's limb), no rows fills. The **exit named in the message** is
not real yet — M-2. The **call site** has no kill — I-2.

### (F) T0 — `forms/2026/YEAR.toml`, `default_year`, `SUPPORTED_YEARS = TEMPLATE_YEARS` — **PASS**

- `forms_expected = []` and `[forms_absent]` carries **18** stems each with a reason (counted:
  `sed -n '/\[forms_absent\]/,/\[oracles\]/p' … | grep -c '='` → 18); the partition against `Stem::ALL`
  is held by `tests/year_record.rs`, and `2026 => 0` was added to its `expected_count` match (the
  `other => panic!` arm still exists, so a fifth year cannot arrive silently).
- `bundled_years() == [2017, 2024, 2025, 2026]`, `default_year() → 2026`, and the sites the spec named by
  line are all updated: `year_readiness.rs` (`the_default_year_is_the_newest_bundled_one`),
  `tui-edit/src/main.rs:15457` (empty ledger → 2026, with the reason in the message), `tests/year_record.rs`
  bundled-years pin + `expected_count` arm, plus `bundled.rs`'s sentence pin, KAT-S1/KAT-F3 re-keyed to
  `app.selected_year` rather than a literal, `field_census.rs`'s map-less-directory admission, and
  `packet.rs`'s zero-form stapling walk. The commit's "13 reds, one class" account matches what the diff
  changed.
- **`SUPPORTED_YEARS = TEMPLATE_YEARS` — nothing is wrongly excluded.** All users enumerated:
  `admin.rs:701` (slice `UnsupportedYear` — 2026 has no template, correct to exclude),
  `cite_check.rs:1200` (template obligations — 2026 has none), `form8283.rs:620` / `form1040.rs:258` /
  `schedule_se.rs:214` (three cluster guards that sweep `2010..=2040` and derive the expected answer from
  `SUPPORTED_YEARS`; excluding 2026 is *required*, since no 2026 template means no measured x-band),
  `supported_years_cross_product.rs` (matrix + `BUNDLED_BUT_NOT_SUPPORTED = [2026]`), `sp4.rs`,
  `census.rs:448` (`newest_filable` must stay 2025 for the era pin), plus two doc references. **Nothing
  that needs 2026 reads `SUPPORTED_YEARS`**: the regime join goes through `YearRecord::for_year`
  (`year_readiness.rs:205`), `default_year`/`YearReadiness` go through `bundled_years()`, and
  `periodic_template` keys on `BUNDLED_YEARS` so TY2026 still aliases f8275/f8283 exactly as its record's
  reasons say. `years_sentence()` correctly moved to `TEMPLATE_YEARS` so no refusal points a filer at a
  year with zero forms.
- The regime join has both kills the spec asked for: `the_regime_is_joined_from_the_year_record`
  (2017/2024/2025/2026 + `None` for 2023) and `the_constant_and_the_regime_agree_on_every_bundled_year`
  (which also asserts `!basis || proceeds`).

### (G) SILENT DEFAULTS — **PASS in production, one gap outside it (I-3)**

Grepping every added line of the build diff for `unwrap_or`, `unwrap_or_default`, `.. Default::default()`
and `default()` yields exactly **one** production hit: `packet.rs:587`'s `unwrap_or_else(|e| panic!(…))`,
which is a loud abort, not a default. The other hits are `ReturnInputs::default()` /
`BrokerReporting::default()` in the manual `Default` (`return_inputs.rs:1370`) and `maximal_sentinel`
(`scrub_axis.rs:342`) — both meaning "nothing answered", which is R1's intended absent-is-unanswered, and
both proved by `an_older_return_inputs_json_loads_with_nothing_answered` and
`the_auto_answerer_never_touches_broker_reporting`.

`Form8949Box::I`/`L` are written in exactly three places: `form_8949`'s pre-route default
(`forms.rs:368,372`), the `NotReported` arm of the router (from an answer), and the keyless-row
`continue` (by mechanism). The first is consumed unrouted by one emitted artifact — I-3.

**R4's half in scope holds.** `adjustment_code` is `String::new()` and `adjustment_amount` is `Usd::ZERO`
at the single row-construction site (`forms.rs:385-386`); the slice filler prints `""` for (f) and blanks
(g) when zero (`fill8949.rs:36-51`); the full-return filler hard-codes `String::new()` for both
(`fill8949_full.rs:55-56`). **No code `B` and no printed `0` is introduced anywhere in the build diff** —
grep for `adjustment_code|"B"` over the added lines returns one hit, `adjustment_code: String::new()` in a
test fixture.

### (H) THE THREE LOAD-BEARING KILLS — 2 of 3 real; all three **wirings** untested

| kill | single-line mutation that reds it | verdict |
|---|---|---|
| the unanswered-key refusal | `return_refuse.rs:929` `None => { return Some(…) }` → `None => {}` reds `a_live_year_with_an_unanswered_key_refuses_and_names_the_exit` on `.expect("must refuse")`. Independently, `forms.rs:89` `regime.basis` → `regime.proceeds` reds `the_question_is_not_asked_on_a_proceeds_only_or_pre_regime_year`; dropping the `!has_rows` disjunct reds the `gemini` case. | **real** |
| the routing table | `forms.rs:110` `let st = matches!(row.part, ShortTerm)` → `LongTerm`, or swapping any one of `I/L`, `H/K`, `G/J`, reds `every_cell_of_the_routing_table`. | **real** |
| the slice predicate | `admin.rs:1373` `if !broker_question_is_live(…)` → `if broker_question_is_live(…)` reds `the_slice_arm_refuses_only_on_a_live_year`. | **real** |
| — but the **call sites** | Deleting `return_1040.rs:2619` (3 lines), `packet.rs:587-589`, or `admin.rs:637-639` **reds nothing**. `PROCEEDS_AND_BASIS` appears in only two test files and is never passed to `screen_absolute`, `assemble_printed_forms`, `assemble_printed_return` or `export_irs_pdf_from_session`. | **I-2** |

---

## Findings

### I-1 (Important) — the routed box is discarded before the printed chain; the "never laundered under the I/L checkbox" claim is false for the full-return filler

**Where.** `crates/btctax-core/src/tax/packet.rs:586-591` (routes, then `form_8949_printed(&rows)`);
`crates/btctax-core/src/tax/printed.rs:37-51` (`Printed8949Row` carries no `box_` at this commit) and
`:81-123` (`form_8949_printed` never reads `r.box_`);
`crates/btctax-forms/src/fill8949_full.rs::fill_8949_full_with_map` (no box grouping, no box guard — the
box comes from `PartMap`'s scalar `box_field`/`box_on`); versus
`crates/btctax-forms/src/lib.rs:115-136` (the G/H/J/K `UnmappedField` refusal, in `fill_form_8949` only).

**What is wrong.** `route_8949_boxes` writes `row.box_`, and the very next call throws it away. The
full-return packet therefore checks whatever the *map* declares per part — the hard-coded I/L on the 2025
map, C/F before — no matter what the filer answered. The one G/H/J/K refusal that exists lives in
`fill_form_8949`, which is the **crypto-slice** filler; `slice_broker_refusal` closes the slice on every
live year, so that refusal is on a path a live year can never reach, and the path a live year *does*
reach has no guard and no way to build one. `249d37ad`'s message — *"The 8949 filler refuses a row boxed
G/H/J/K until T3's per-(part, box) page-sets exist … never laundered under the I/L checkbox"* — is true of
the slice filler and false of the one the packet uses. `broker_boxes.rs` passes because it calls
`fill_form_8949` directly with a hand-set box.

**Why it is Important and not Critical.** No wrong box can print today: a live year is only TY2026
(`basis = true` appears only in `forms/2026/YEAR.toml`), and TY2026 has neither `FullReturnParams` (FR-47)
nor an f8949 template, so `export_full_return` refuses upstream and `Form8949Map::for_year(2026)` is
`UnsupportedYear`. It becomes a Critical the day FR-47 inserts TY2026 params and the January-2027 package
lands — i.e. exactly when nobody is looking at this seam any more.

**Minimal change.** Add `pub box_: crate::forms::Form8949Box` to `Printed8949Row`, set it in
`form_8949_printed` from `r.box_`, and give `fill_8949_full_with_map` the same G/H/J/K `UnmappedField`
refusal `fill_form_8949` has. This is T3's opening move (the uncommitted working tree already contains
precisely this) — so the correct disposition is to make it T3's first commit and its entry criterion,
not a separate fold.

### I-2 (Important) — none of the three production wirings has a kill

**Where.** `crates/btctax-core/src/tax/return_1040.rs:2619` (the `screen_broker_reporting` call inside
`screen_absolute`); `crates/btctax-core/src/tax/packet.rs:587` (the `route_8949_boxes` call);
`crates/btctax-cli/src/cmd/admin.rs:637` (the `slice_broker_refusal` call).

**What is wrong.** All three predicates are tested directly and well. **No test drives any of them
through its caller.** `InformationReturnRegime::PROCEEDS_AND_BASIS` occurs in exactly two files
(`kat_broker_reporting.rs`, `admin.rs`'s own unit test) and is never handed to `screen_absolute`,
`assemble_printed_forms`, `assemble_printed_return` or `export_irs_pdf_from_session`. Delete any one of
the three call sites and the suite stays green. This is the repo's own B1 shape restated: a gate whose
*placement* — the thing that makes it a gate rather than a function — has never been watched going red.
It also means the "the screen runs FIRST" property of lens (A) is asserted only by a comment.

**Minimal change.** Three tests:
1. `screen_absolute(&ri, &ar, &p, &owner_like(2026), 2026, PROCEEDS_AND_BASIS)` → `BrokerReportingUnanswered`
   (this simultaneously pins that the broker screen precedes the others).
2. an `assemble_printed_forms` call on a live, fully-answered year whose printed 8949 carries `G`/`J`
   (needs I-1's field first — so this is T3's kill, not a separate one).
3. a CLI-level `export_irs_pdf` on a TY2026 vault with one exchange disposition and no stored
   `ReturnInputs` → the slice refusal, and `out_dir` does not exist afterwards.

### I-3 (Important) — `form8949.csv` prints an unrouted box on a live year, consulting nothing

**Where.** `crates/btctax-cli/src/render.rs:1187-1224` (`write_form8949_csv`, whose header row literally
contains `"box"`), reached from `write_csv_exports` (`cmd/admin.rs:205`, `btctax export-snapshot`) and
`write_form_csvs` (`btctax-tui/src/export.rs:200`).

**What is wrong.** The CSV's `box` column is filled from `form_8949(state, year)` with no regime, no
answers, no routing and no refusal. On a live TY2026 year with `BasisMatches` stored, it emits `I`/`L`
while the return's box is `G`/`J`. Unlike either PDF arm — the slice refuses, the full return routes or
refuses — nothing gates this one, and unlike the PDF it is **reachable today**: `export-snapshot` needs
neither `FullReturnParams` nor a template, and there is no year check on `write_csv_exports`
(`admin.rs:205-212`). This is the green-and-blind class: the tool now knows the answer and one emitter
never asks.

Fairness note: the spec never names this surface (T6's list is `report`, the TUI input form,
`YearReadiness::sentence`, the advisory and the TUI forms tab). So this is a **gap in the spec that the
build inherited**, not a deviation from it — but it is a wrong box in an emitted artifact and it should
not survive the cycle.

**Minimal change.** Either route the CSV rows (both call sites can reach `year_readiness::regime_or_refuse`
and the stored `ReturnInputs`), or apply `slice_broker_refusal`'s predicate and refuse on a live year with
the same message. The same question should be asked once of `btctax-tui/src/tabs/forms.rs:76`, which
renders box tags from unrouted rows (T6 owns that surface, so it is not counted here).

### M-1 (Minor) — the spec's T2 kill "the cohort of a sold-out 2026 exchange-purchased lot is `Covered`" is not implemented

**Where.** T2's kill list, `SPEC_1099da_broker_reporting.md`. `Cohort::Covered` never appears in a
fold-driven assertion anywhere in the tree (`grep -rn "Cohort::Covered"`): every `Covered` assertion is
against hand-built `DisposalLeg`s that set `lot_acquired_at` themselves.

**Why Minor rather than Important.** The discriminating power the kill was for is covered by the union of
two tests that *are* present: `kat_tax.rs:912-923` (tp11) runs the real fold and reds if `fold.rs:370`
reads `acquired_at`/`gain_hp_start` instead of `c.acquired_at`, and `rows_carry_the_cohort_by_mechanism`'s
`tacked` row reds if `form_8949` reads `acquired_at` instead of `lot_acquired_at`. What is missing is the
end-to-end `Covered` case, which would also catch a future fold path that assigns `lot_acquired_at`
correctly for gifts and wrongly for purchases.

**Minimal change.** One test: fold a Buy on an exchange dated 2026-02, a Sell of the whole lot dated
2026-06, then assert `form_8949(&state, 2026)[0].cohort == Cohort::Covered`.

### M-2 (Minor) — the slice refusal names an exit that does not work yet

**Where.** `crates/btctax-cli/src/cmd/admin.rs:1372-1390`.

**What is wrong.** The message tells a TY2026 filer to run `income import` "then export the FULL return".
`export_full_return` refuses TY2026 today (`full_return_for(2026)` is `None`, and there is no f8949
template), so the filer is sent from one refusal to another with no statement that the second is expected.

**Minimal change.** Append the year's readiness sentence, or reuse `year_readiness::import_note(year)`, so
the message says the full return is not fillable for TY2026 until the January-2027 package (FR-47).

### M-3 (Minor) — a missing prior-year record silently drops the capital-loss carryforward

**Where.** `crates/btctax-cli/src/cmd/tax.rs:689-698`.

**What is wrong.** The prior-year computability test became
`regime_for(year - 1).is_some_and(|rg| screen_absolute(…).is_none())`. When `regime_for` is `None` (no
`YEAR.toml` for `year-1`) the whole conjunct is false and `ar_prev.capital_loss_carryforward_out` is
dropped with no message — a silent default where every other regime consumer uses `regime_or_refuse` and
raises a typed `UnsupportedYear`. It fails in the tax-**increasing** direction and is unreachable today
(every year with `FullReturnParams` has a record), which is why it is Minor rather than a gate.

**Minimal change.** `regime_or_refuse(year - 1)?`, or keep `regime_for` and print the same note the
carryover-preservation block prints when it declines to carry something forward.

### N-1 (Nit) — stale count in a message

`crates/btctax-forms/src/bundled.rs:257-261`: the assertion is now `&[2017, 2024, 2025, 2026]` and the
message still reads `"three years on disk today"`.

### N-2 (Nit) — a dead tuple slot in an assertion message

`crates/btctax-core/tests/kat_forms.rs:262-265`: `by_lot` is built as `(0, r.cohort, r.date_acquired)`
with a hard-coded `0`, so the "lot" the failure message names is always `0`. Either drop the slot or carry
the real lot id.

### N-3 (Nit / process) — the tree was dirty when the suite ran

At review time `git status --short` showed **11 modified files** of in-progress T3 work
(`btctax-core/src/tax/{printed,line_coverage}.rs`, `btctax-forms/src/{lib,map,fill8949,fill8949_full}.rs`,
`forms/2025/f8949.map.toml`, `tests/{broker_boxes,kats,sp3,sp3b}.rs`). The one scoped run (44/44 green)
therefore measured the working tree, not `249d37ad` — in particular `broker_boxes.rs` is 176 lines changed
there. Every source claim in this report was read back with `git show 249d37ad:<path>` for that reason. A
build review should be run on a clean checkout (or a worktree at the commit) so the suite result and the
report describe the same tree.

---

## T3 may proceed?

**NO — 0 Critical / 3 Important / 3 Minor / 3 Nit.** An Important blocks the gate.

The unblocking path is short and mostly runs *through* T3 rather than beside it:

- **I-1** and **I-2's** second kill are T3's own first commit (carry `box_` onto `Printed8949Row`, guard
  and group `fill_8949_full`). Take them as T3's entry criteria and put both in T3's kill list; do not
  fold them separately.
- **I-2's** first and third kills (`screen_absolute` with a live regime; a CLI-level slice refusal on
  TY2026) are two small tests that belong to step C and should land before T3 opens — they are what makes
  step C's three gates gates.
- **I-3** is step C's, not T3's: one emitter still prints an unrouted box, and it is the only reachable
  wrong box in the tree today.
- M-1/M-2/M-3 and the nits do not hold the gate; M-1 and M-3 each want an owning phase recorded.

Everything else the four commits set out to do is faithful: the screen fires on all six R1 clauses with
kills for each, the routing table matches i8949 line by line, the cohort reads the lot's own date through
a single sound construction site for both pool kinds, T0's year record and year-set re-derivation are
complete and correctly exclude 2026 from `SUPPORTED_YEARS`, and there is no silent default and no code `B`
or printed `0` anywhere in the diff.
