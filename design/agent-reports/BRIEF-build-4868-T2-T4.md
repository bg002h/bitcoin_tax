# Brief — Form 4868 / 1040-V build, tasks T2 + T3 + T4 (the fillers, the command, the voucher hook)

You are the single implementer for this task in the shared main tree `/scratch/code/bitcoin_tax`
(branch `main`; the controller will state HEAD at dispatch). Do NOT spawn subagents. Do NOT commit
or push; the controller commits. Never `git checkout --`/`git restore` files you did not create.
Tests: `cargo nextest run --locked -p <crate> -E '<filter>'` (never `cargo test`, never `--release`,
never the whole workspace — the controller's pre-commit gate runs `make check`). `cargo fmt --all`
before finishing. Build in the default target dir.

## The spec
`design/SPEC_form_4868_1040v.md` (GREEN r6). Read **R2** (the command and its refusals), **R3**,
**R4** (the voucher rides with `export-irs-pdf`, loose), the two provenance tables (lines 31–57) and
the Plan items **T2, T3, T4** (lines 189–216). The spec wins over this brief; say so in the report
where they differ.

## What T1 + T5 already landed (machine-verified by the controller; do not redo)
- `Stem::{F4868, F1040v}`, `LineSet::{F4868_2024, F4868_2025, F1040v_2024, F1040v_2025}`,
  `Schema::{Form4868Map, Form1040VMap}`, the two map structs in `crates/btctax-forms/src/map.rs`
  with `for_year(year)`/`ty2024()`/`ty2025()`/`field_names()` helpers, the four `.map.toml` rows and
  bundled PDFs under `crates/btctax-forms/forms/{2024,2025}/`, the year records, the row gates and the
  label-reader walk's grid bucket. Read `design/agent-reports/2026-09-06-build-4868-T1-T5-implementation.md`
  for the exact names before you start — binding names on the 4868 map: `name_line`, `address_street`,
  `address_city`, `address_state`, `address_zip`, `taxpayer_ssn`, `spouse_ssn`, `line4`…`line8` (line 8
  is the `c1_1` checkbox); on the 1040-V map: `box1_ssn`, `box2_spouse_ssn`, `box3_amount`,
  `box4_first_name`, `box4_last_name`, `spouse_first_name`, `spouse_last_name`, `address_street`,
  `address_apt`, `address_city`, `address_state`, `address_zip`.
- The inputs the fillers read (all exist today):
  `btctax_core::tax::packet::PrintedReturn { header: ReturnHeader, filing_status: FilingStatus, forms: PrintedForms }`
  (`crates/btctax-core/src/tax/packet.rs:471`); `ReturnHeader { name_line, taxpayer: FiledPerson,
  spouse: Option<FiledPerson>, address_street, address_city, address_state, address_zip, … }`
  (`:335`; there is NO apartment field — the voucher's `address_apt` cell stays blank, recorded);
  `FiledPerson { first_name, last_name, ssn: Ssn, … }` (`:195`; `Ssn::hyphenated()` at `:81`);
  `PrintedForms.f1040: Form1040Lines` with `line24`, `line33`, `line37` (`crates/btctax-core/src/tax/printed.rs:629,647,653`);
  `PrintedForms.sch_3: Option<Schedule3Lines>` with `line10` (`printed.rs:1616`) — it prints
  `payments.extension_payment` (`return_inputs.rs:708`), already whole-dollar.
- The identity-cell helper: `crates/btctax-forms/src/cells.rs:155 push_identity(w, p, cells: &IdentityCells{name, ssn}, name, ssn, fields)`
  renders the SSN from the cell's own `/MaxLen` (11 ⇒ hyphenated). `push_money`/`push_money_opt`/`push_literal` are beside it.
- The full-return export arm is the model for everything the command does:
  `crates/btctax-cli/src/cmd/admin.rs::export_full_return` (~line 1006 onward): `promote_export_gate` →
  `full_return_for(year)` refusal → `return_inputs::get` refusal ("no return_inputs stored for {year}",
  `:1035`) → `assemble_absolute` → `regime_or_refuse` → `screen_absolute` refusal → `assemble_printed_return`
  → `fill_full_return` (all-or-nothing, BEFORE `mkdir_out`) → `state.pseudo_active()` ⇒
  `require_attestation(attest)?` then `btctax_forms::stamp_draft_watermark(&bytes)` per form →
  `write_bytes_owner_only(path, bytes)` (`:967`) → `manifest.txt`. The CLI arm is `main.rs:738`
  (`Command::ExportIrsPdf`), with the interactive attest prompt; `resolve_now()` (`main.rs:70`) is the
  clock (`BTCTAX_NOW`). `YearRecord::for_year(year).return_due` (`crates/btctax-forms/src/year_record.rs:73`)
  is the year's due date (TY2017's is 2018-04-17 — never hardcode a month/day).
- There is NO §7503 helper in the tree; T3 adds one (a due date falling on Saturday/Sunday moves to
  the next Monday; also the District of Columbia legal holidays the IRS honours — at minimum keep it to
  weekends AND document that holidays are not modelled, with the kill below pinning the dates the spec
  names).
- The full suite is green at HEAD.

## T2 — `Form4868Map` + `fill_form_4868(&PrintedReturn, year, choices) -> Result<Vec<u8>, FormsError>`
In `crates/btctax-forms/src/form4868.rs` (new; register it in `lib.rs` like the other fillers, and
expose it publicly). `choices` is a small struct `Form4868Choices { pay: Option<Usd>, out_of_country: bool }`.
Lines, exactly as the spec's table (lines 39–43): line 4 = `f1040.line24`, printed as an explicit
`0` when zero; line 5 = `line33 − sch_3.line10` (0 when `sch_3` is `None`), blank when zero; line 6 =
`max(0, L4 − L5)`, printed as an explicit `0` when zero or negative; line 7 = `choices.pay` if given,
else the PRINTED Schedule 3 line 10 when > 0, else line 6; blank when zero; line 8 = the `c1_1`
checkbox iff `out_of_country`. Part I: `name_line`, the address cells, `taxpayer_ssn`; `spouse_ssn`
ONLY when `filing_status == FilingStatus::Mfj`; the fiscal-year header blank; `c1_2` NEVER set.
`--pay` negative or with cents is refused by the COMMAND (T3), but the filler also refuses a
non-whole-dollar `pay` (fail closed twice is fine). Use the same geometric read-back verifier the
other fillers use (look at `form8959.rs` or `schedule_d_full.rs` for the pattern: `FlatPlacement`,
the no-unmapped set, the page check) so a mis-mapped cell is caught at fill time.
Kills (each seen red once — plant, observe, revert; say how): L5 > L4 → line 6 prints `0`; line 4 zero
prints `0`; line 5 zero prints blank; default line 7 = line 6, or the printed Schedule 3 line 10 when
> 0; a `pay` with cents → refuse; an excess-SS credit with no extension payment ⇒ line 5 = line 33,
and with both ⇒ line 5 excludes only the extension payment; MFS ⇒ line 3 (spouse SSN) blank; MFJ ⇒
filled; the fiscal-year header blank; `c1_2` never set; **`out_of_country` ⇒ `c1_1` checked, absent
⇒ blank — mutation-verified by removing the write** (an unchecked box and a never-written box print
identically; only the test tells them apart — read the checkbox state back from the PDF the way
`crates/btctax-forms/tests/broker_boxes.rs::check_states` does). Fill on BOTH years (2024 and 2025)
in the KATs; TY2024 is the year whose return computes.

## T3 — the command `btctax extension --year <Y> --out <DIR> [--pay <usd>] [--out-of-country] [--attest <phrase>]`
`crates/btctax-cli/src/cli.rs`: a NEW top-level `Command::Extension` beside `ExportIrsPdf` (no
`tax` group). `crates/btctax-cli/src/cmd/admin.rs::extension(vault, pp, out, year, pay, out_of_country, attest, now) -> Result<ExtensionReport, CliError>`;
the `main.rs` arm mirrors `ExportIrsPdf` (passphrase, the same interactive attest prompt when
pseudo-active, `resolve_now()` passed in). Refusals, in the spec's order (R2): the three
`export_full_return` refusals reused as they stand (no bundled params; no stored inputs; a screen
refuses — with the export's message and NO bytes); `--out` already holds a `manifest.txt` → refuse
("the 4868 is mailed separately; an unlabelled copy in the return's envelope is what the form
forbids"); `--pay` negative or with cents → refuse (above line 6 allowed); pseudo-active ⇒
`require_attestation` first (no bytes on refusal) and `stamp_draft_watermark` on every page.
`promote_export_gate` is NOT applied (spec R2 says why — quote it in the doc comment). Output:
`f4868.pdf` owner-only in `--out`, then print what was filled line by line, then the form's own
sentence *"Don't attach a copy of Form 4868 to your return."* Warning (printed, never a refusal): the
clock is past the due date — `return_due` from the year record, REPLACED by June 15 of the following
year shifted by §7503 when `--out-of-country` is passed (NOT `return_due + 2 months`). A recorded
extension payment (the PRINTED Schedule 3 line 10) is the default for `--pay`, with the note
*"$N is already recorded on the return as paid with the extension; --pay overrides it"*.
Kills: uncomputable year → the export's message, no bytes (assert the directory has no files);
`--out` with a manifest.txt → refusal; pseudo + no attest → refusal, zero bytes; pseudo + attest →
watermarked (read the page for the stamp the way the packet tests do); a TY2017-shaped record never
prints `04-15`; an out-of-country run on May 1 prints no warning; a TY2017-shaped record with
`--out-of-country` warns against **2018-06-15** (not 06-17); the §7503 shifter: a Saturday moves to
Monday, a Sunday to Monday, a weekday stays; a TY2024 end-to-end KAT in `crates/btctax-cli/tests/`
prints a real 4868 from the kitchen-sink fixture (see how `export_irs_pdf.rs` tests build a vault
with return inputs — reuse that harness) with the due-date warning, since 2025-04-15 has passed under
a pinned `BTCTAX_NOW`.

## T4 — `Form1040VMap` + the export hook (`--pay-by-check`)
`crates/btctax-forms/src/form1040v.rs` (new): `fill_form_1040v(&PrintedReturn, year, amount: Usd) -> Result<Vec<u8>, FormsError>`
— box 1 taxpayer SSN, box 2 spouse SSN only when MFJ, box 3 the amount (whole dollars), box 4 names
(spouse names only when MFJ), the address cells (`address_apt` blank), the three foreign cells never
written. `export-irs-pdf` gains `--pay-by-check`: on the FULL-RETURN path, when `line37 > 0` AND the
flag is passed, write `f1040v.pdf` BESIDE the packet (owner-only, watermarked if pseudo) and add a
separate footer block to `manifest.txt` — *"ENCLOSE LOOSE — do not staple: f1040v.pdf with the
check"* — under the stapling list, never inside it, and never passed to `FiledPacket::stapled`
(`crates/btctax-forms/src/packet.rs:305`; hold this by test — grep that no code path pushes either
new stem into `stapled`'s input). Without the flag: nothing written and the note *"amount owed $N —
Direct Pay / EFTPS need no voucher; pass --pay-by-check for Form 1040-V"* on the report; with the
flag and line 37 = 0 → a note, no voucher; `--pay` (the voucher's own partial-payment override, per
the spec's box 3 row) above line 37 or negative → refuse; on a crypto-slice year (no full-return
inputs) `--pay-by-check` REFUSES: *"there is no Form 1040 line 37 for {year} — Form 1040-V
accompanies a full return; see `income import`"*. `IrsPdfReport` gains `form_1040v_path: Option<PathBuf>`
and the note; `main.rs` prints them.
Kills: each bullet above as a test (the existing `export_irs_pdf.rs` harness), plus a TY2024
end-to-end KAT that reads box 3 back from the written voucher and asserts it equals line 37, and the
manifest's footer block is present and separate from the stapling list.

## T6 is NOT yours (`report` naming the paths; `YearReadiness::sentence` unchanged; help text) — the controller schedules it.

## Constraints
- Every guarantee needs a test that reds when removed; say in the report how each kill was seen red.
- Do not touch the spec or any `design/agent-reports/*.md` other than your report.
- `docs/examples/examples.md` and the man pages regenerate ONLY as a direct consequence of the new
  command/flag (`cargo run -q -p xtask -- docs` for man pages; the examples golden should NOT change
  unless a journey uses the new flag — say which lines moved).
- Secret-handling defects are never blocking, but the written files must be owner-only like the packet's.

## Report — write it as your FINAL action
`design/agent-reports/2026-09-06-build-4868-T2-T4-implementation.md`: per task, what you changed
(file:line), the kills and how each was seen red, every pinned number moved (old → new, cause),
deviations from the spec/brief with reasons, the exact nextest commands with summary lines, and
anything you could not do. Then return ONLY a 3-line summary (what landed, suite results for the
crates you touched, the report path).
