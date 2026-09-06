# VERIFICATION ledger — spec-1099da r2 review (2026-09-06-spec-1099da-review-r2.md)

Controller: Claude Fable 5.1, 2026-09-06, tree at `323ceadd`. Every measurable claim re-run.

| finding | claim | check | verdict |
|---|---|---|---|
| C-1 | box 2 follows the lot the broker identifies; customer acquisition info is for lot selection only | `Instructions_1099-DA.txt:253-262`: "If the digital asset sold … is a covered security … boxes 1d, 1g, 2, and 6 must be completed"; "If … is a noncovered security, you may check box 9"; `:475-477`: "brokers may use customer-provided acquisition information solely for lot-selection purposes and not for reporting basis or acquisition dates" | **TRUE** — the r2 sentence "cannot change WHICH form lists a sale" was wrong |
| N-1 | a struct map key fails serde_json and toml | language fact (both formats require string keys); the reviewer ran the exact shape | **TRUE** |
| N-2 | `screen_inputs(ri, tbl, p)` sees no ledger or regime | r1 ledger (signature); `screen_inputs` has 46 call sites, `screen_absolute` 51 (measured) | **TRUE** |
| N-3 | the slice arm runs iff no `ReturnInputs` exists | `admin.rs:594` `if crate::return_inputs::exists(session.conn(), tax_year)?` | **TRUE** — r2's "unless a stored ReturnInputs answers" is unsatisfiable on that arm |
| N-4 | r2 dropped r1's "basis answer on a proceeds-only regime refuses" kill | r2 R2 text: "under a regime with `proceeds = false` → Refusal" — TY2025 (`proceeds = true, basis = false`) uncovered | **TRUE** |
| N-5 | `Cohort` is a total partition so `BrokerCohortUnknown` cannot fire; `FmvAtIncome`/`CardRewardRebate` are silently `Noncovered` | r2 R1 text ("Everything else"); `BasisSource` variants at `event.rs:17-29` include `FmvAtIncome`, `ExchangeProvided`, `ComputedFromCost`, `CarriedFromTransfer`, `GiftCarryover`, `GiftFmvFallback`, `SafeHarborAllocated`, `ReconstructedPerWallet`, `SelfTransferInbound` (+ the two the reviewer names) | **TRUE** |
| N-6 | 15 TY2025 maps, not ten | `ls forms/2025/*.map.toml \| wc -l` → 15 | **TRUE** |
| N-7 | `forms.rs:259-296` is the Form 8283 how-acquired mapping | `forms.rs:255-262` doc: `Form8283HowAcquired` | **TRUE** |
| N-8 | `default_year()` callers are the two TUIs only | `tui/src/app.rs:195`, `tui-edit/src/editor.rs:300`; no `report` caller | **TRUE** |
| N-9 | `full_return_for(2026)` is deliberately `None` | `tax_tables.rs:170-173` | **TRUE** |
| N-10 | `acquired_at` is the zone-aware holding-period start | `state.rs:208-212` doc | **TRUE** |
| N-11 | `PartMap` has scalar `box_field`/`box_on` | `map.rs:449-464` | **TRUE** |
| N-12 | `DIGITAL_ASSET_8949_FIRST_YEAR` read at `forms.rs:135`, `admin.rs:471`, `tui/tabs/forms.rs:174` | grep | **TRUE** |
| (e) | `build.rs` enters a year per file, so `YEAR.toml` alone bundles it; `tests/year_record.rs:55` pins `[2017, 2024, 2025]` | `build.rs:81` `found.entry(year).or_default()`; `year_record.rs:55` | **TRUE** |
| N-4b | `DisposalLeg.proceeds` is net; the broker must reduce proceeds by costs | `state.rs:200`; `Instructions_1099-DA.txt:470-472` "You must reduce the proceeds by digital asset transaction costs" | **TRUE** |

**Verdict: 15/15 claims TRUE, 0 refuted.** Design consequence taken into r3 (stated there): the
answers stay on `ReturnInputs` and the crypto slice is declared CLOSED for a year with a basis regime
(the refusal names the exit), rather than a second storage surface — recorded as an owner-visible
product decision in `ROADMAP_STATUS.md` §0a.
