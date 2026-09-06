# Build report — Form 4868 / Form 1040-V, spec tasks T2 + T3 + T4

Implementer: single agent, shared main tree `/scratch/code/bitcoin_tax`, branch `main`, from HEAD
`cf58b4af`. Nothing is committed or pushed. No subagents. Brief:
`design/agent-reports/BRIEF-build-4868-T2-T4.md`; spec: `design/SPEC_form_4868_1040v.md` (GREEN r6),
R2/R3/R4 and plan items T2 / T3 / T4. T1+T5 report read first for the landed names.

**Result: all three tasks landed, plus the T1 residue fold.** Every crate's suite is green.
**3109 → 3145** tests (+36: btctax-forms 344 → 363, btctax-cli 662 → 679). `cargo fmt --all --check`
clean; `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features --
-D warnings` emits nothing.

---

## 1. What changed, file by file

### New files (5)

| file | what |
|---|---|
| `crates/btctax-forms/src/form4868.rs` | T2 — `Form4868Choices`, `Form4868Lines`, `form_4868_lines`, `fill_form_4868`, `fill_form_4868_with_map` |
| `crates/btctax-forms/src/form1040v.rs` | T4 — `fill_form_1040v`, `fill_form_1040v_with_map` |
| `crates/btctax-forms/tests/f4868_fill.rs` | 13 KATs (§7 kills M1–M12) |
| `crates/btctax-forms/tests/f1040v_fill.rs` | 6 KATs (kills V1–V5) |
| `crates/btctax-cli/tests/extension.rs` | 10 KATs for `btctax extension` (kills E1–E13) |

### Edited files (13)

| file | what |
|---|---|
| `crates/btctax-forms/src/error.rs:82` | new `FormsError::InvalidValue { form, line, detail }` — the refusal for a payment a whole-dollar money line cannot carry. No exhaustive `match` on `FormsError` exists (measured: the only uses are `?` and three `UnsupportedYear` constructions), so nothing else moved. |
| `crates/btctax-forms/src/lib.rs:27,28,58,59` | `mod form1040v; mod form4868;` + `pub use` of the two fillers and T2's line types; `testonly` gains both `_with_map` seams. |
| `crates/btctax-forms/src/packet.rs:97-105` | **the T1 residue** — explicit `"f4868" \| "f1040v" => None` arm ahead of the catch-all, with the reason (neither prints a sequence number; a sequence number is a stapling position, and both forms forbid stapling). |
| `crates/btctax-forms/src/year_record.rs:180-208` | new `section_7503_shift(Date) -> Date`. |
| `crates/btctax-core/src/tax/printed.rs:1624` | a stale doc comment corrected — see §6.1. **The only core change; no figure moves.** |
| `crates/btctax-cli/src/cli.rs` | `ExportIrsPdf` gains `--pay-by-check` and `--pay`; new top-level `Command::Extension`. |
| `crates/btctax-cli/src/cmd/admin.rs` | `ScreenedReturn`/`screen_full_return` (the shared prelude — §2), `VoucherChoice`, two new `IrsPdfReport` fields, `write_payment_voucher` + `ENCLOSE_LOOSE_LINE`, `ExtensionReport`, `extension_due_date`, `extension`, `extension_from_session`. |
| `crates/btctax-cli/src/lib.rs:298` | `NOT_AUTHORISED_FOR_FILING` extracted from main.rs (§6.2). |
| `crates/btctax-cli/src/main.rs` | the `Extension` arm (attest prompt + `resolve_now()`); `ExportIrsPdf` passes `VoucherChoice`; the voucher path + note are printed; the notice literal is now the constant. |
| `crates/btctax-cli/src/render.rs` | `render_extension`. |
| `crates/btctax-cli/tests/{export_irs_pdf,promote_cli,experimental_notice}.rs` | 51 `export_irs_pdf(...)` call sites gain `Default::default()` (§2); `export_irs_pdf.rs` gains 8 T4 KATs. |
| `docs/man/btctax.1`, `docs/man/btctax-export-irs-pdf.1`, `docs/man/btctax-extension.1` (new) | `cargo run -q -p xtask -- docs`. |
| `docs/examples/examples.md` | **ONE line** — see §5. |

---

## 2. The one structural decision the brief did not name: `screen_full_return`

Spec R2 says the extension reuses *"`export_full_return`'s three refusals, **as they stand and in its
order**"*. I made that structural rather than a promise: the tables lookup, `return_inputs::get`, and
the three fail-closed screens (`screen_inputs` → `screen_compute_dependent` → `assemble_absolute` →
`regime_or_refuse` → `screen_absolute`) moved into one `fn screen_full_return`
(`crates/btctax-cli/src/cmd/admin.rs:1063`), which both paths call.

**`export_full_return`'s observable order is unchanged.** The helper deliberately stops BEFORE
`require_attestation` and `assemble_printed_return`, because folding those in would have moved the
attestation gate after the header build — on a pseudo ledger with an unprintable header the error
would have changed from "attestation" to "cannot be printed". Both still write zero bytes, but that
is a behaviour change to a shipped, tested path and I did not want it. The `advisories_for` call and
everything downstream stayed put.

Why it is worth the diff: a hand-copied prelude in the extension arm is exactly the B3 shape — each
lane green, the product wrong where they meet. The 662 pre-existing btctax-cli tests are the
regression check, and they pass unchanged.

**The 51-call-site churn.** `export_irs_pdf` gained a `voucher: VoucherChoice` parameter rather than
an `export_irs_pdf_with_voucher` overload. R4 makes the voucher part of what `export-irs-pdf` *is*, so
a public entry point that cannot express the decision is an incomplete API — and a second entry point
would re-create the drift `screen_full_return` had just removed. The insertion was mechanical (a
paren-matching script), and the compiler verified every site. **Two sites the script got wrong and I
fixed by hand**, both caught by `rustc`: three multi-line calls in `export_irs_pdf.rs` got a stray
leading comma, and `experimental_notice.rs:275` had a *function name* ending in `export_irs_pdf(`
that the matcher treated as a call.

---

## 3. T2 — `fill_form_4868`

`crates/btctax-forms/src/form4868.rs`. The Part II chain is a separate pure function,
`form_4868_lines(&PrintedReturn, Form4868Choices) -> Result<Form4868Lines, FormsError>`, for two
reasons: the arithmetic is assertable without a PDF, and `btctax extension` renders its terminal
report from **the same call** the PDF was filled from — so the screen and the paper cannot disagree
about line 6.

The chain, exactly as the spec's table:

| line | value | blank-vs-zero |
|---|---|---|
| 4 | `f1040.line24` | printed `0` when zero (the form's own `-0-` clause) |
| 5 | `max(0, line33 − sch_3.line10)`; `sch_3 = None` ⇒ subtract 0 | **blank** when zero — no `-0-` clause |
| 6 | `max(0, L4 − L5)` | printed `0` |
| 7 | `choices.pay`, else the PRINTED Sch 3 line 10 when > 0, else line 6 | blank when zero |
| 8 | `c1_1` iff `choices.out_of_country` | never written when false |

Part I is bound by name (`name_line`, the address cells, `taxpayer_ssn`), and **`spouse_ssn` is gated
on `pr.filing_status == Mfj`, never on `header.spouse.is_some()`** — the fixture header carries a
spouse on MFS too, which is what the discriminating KAT exercises.

**Geometry.** Same `verify_flat` oracle the other flat fillers use, with a code-side cluster measured
from both blank PDFs (`xtask dump-fields`): lines 4–7 occupy `x 496.8…576.0` on TY2024 **and** TY2025.
Two descent groups — the money lines 4→7, and Part I's left column (name → street → city → SSN row).
The no-unmapped leg is what holds "the fiscal-year header stays blank" and "line 9 is never checked":
neither is a placement.

**`Usd::ZERO`, not a floor I invented.** Line 5's `max(0, …)` is defensive, and the code says so:
line 33 already contains line 10 (via line 31 ← Sch 3 line 15), so the difference cannot go negative
on a self-consistent return; should one ever arrive, a blank line 5 says nothing where a printed
negative would be false testimony.

### Deviation from the brief

The brief said *"the filler also refuses a non-whole-dollar `pay`"*. I refuse **negative as well**,
in the same guard (`refuse_unless_whole_nonnegative`), which the spec's own T2 kill list asks for
(*"`--pay` negative or with cents → refuse"*). Both are also refused by the command; fail closed twice.

---

## 4. T3 — `btctax extension`

`crates/btctax-cli/src/cmd/admin.rs::extension` / `extension_from_session`; `cli.rs::Command::Extension`
(a NEW top-level command beside `export-irs-pdf` — no `tax` group invented); `main.rs`'s arm mirrors
`ExportIrsPdf`'s (passphrase, the same interactive attest prompt when pseudo-active, `resolve_now()`
passed in as `now`).

**Refusals, in the spec's R2 order:** (1) no bundled params, (2) no stored inputs, (3) a screen refuses
— all three through `screen_full_return`, so the wording is literally the export's; (4) `--out` holds a
`manifest.txt`; (5) `--pay` negative or with cents (above line 6 allowed); (6) pseudo-active ⇒
`require_attestation` first. `promote_export_gate` is **not** applied, and R2's reasoning is quoted in
the function's doc comment.

**The fill runs before `mkdir_out`**, so a refusal leaves `--out` untouched — and that promise is
tested against the directory, not a filename list (kill E8 plants a write before the refusals and reds
five tests at once).

**The due date.** `extension_due_date(tax_year, return_due, out_of_country)` is pure and takes the
record's date, because the case that discriminates cannot be reached end to end: TY2017's committed
`return_due` is **2018-04-17** (the Emancipation Day shift) and TY2017 has no full-return tables, so
the command refuses long before the warning. `out_of_country` ⇒ **June 15 of the following year,
§7503-shifted** — 2018-06-15, *not* `return_due + 2 months` = 06-17. Both directions pinned.

`section_7503_shift` models the WEEKEND half only, and the doc comment says why that is exact rather
than approximate here: §7503's holiday half turns on District of Columbia legal holidays, and **its one
caller shifts June 15**, on which no DC legal holiday ever falls (Juneteenth is June 19; Emancipation
Day is April 16). Every other due date comes from `YearRecord::return_due`, where the shift is already
baked in and must not be applied twice.

**One measurement worth recording.** `a_screen_refusal_gives_the_exports_message_and_writes_no_bytes`
originally asserted `contains("not computable") || contains("cannot be printed")`. I probed which
actually fires — it is `screen_inputs`, message
`the 2024 return is not computable [DependentStatusUnanswered]: … — no forms were written` — and
tightened the assertion to that exact shape with `&&`. (Note for the reviewer: `export_irs_pdf.rs`'s
own screen-1 test carries a comment saying that fixture reds via the `ReturnHeader::build` backstop
instead. I did not touch it; on the extension path the measured mechanism is the screen itself.)

### Additions beyond the brief's literal text

- The **no-authorisation notice** prints on this path too. Its own comment in main.rs says it belongs
  on *"EVERY form export"*, and this is the one the filer posts with a cheque. To avoid a second copy
  I extracted it to `btctax_cli::NOT_AUTHORISED_FOR_FILING` — **byte-identical text**, verified by the
  examples golden not moving on that line.
- `render_extension` renders a blank line as `(blank)`, never `$0`, and has its own kill (E11).

---

## 5. T4 — `fill_form_1040v` + the export hook

`crates/btctax-forms/src/form1040v.rs` fills box 1 (taxpayer SSN), box 2 + the spouse name row **only
when `filing_status == Mfj`**, box 3 (whole dollars), box 4 names, and the address cells. `address_apt`
and the three foreign cells are never written and are held blank by the no-unmapped scan plus an
explicit KAT. The amount is refused unless positive and whole: a $0 voucher is not a payment.

`export-irs-pdf` gains `VoucherChoice { pay_by_check, pay }`. `write_payment_voucher`
(`admin.rs:990`) covers four cases, none of them silent:

| line 37 | `--pay-by-check` | outcome |
|---|---|---|
| `> 0` | yes | `f1040v.pdf` beside the packet (owner-only, watermarked if pseudo) + the manifest block |
| `> 0` | no | no file; the note *"amount owed $N — Direct Pay / EFTPS need no voucher; pass --pay-by-check for Form 1040-V"* |
| `0` | yes | no file; a note saying the return owes nothing |
| `0` | no | nothing |

On a crypto-slice year `--pay-by-check` **refuses** before any byte, with I-7's message.
`--pay` is refused above line 37, negative, or with cents; the "above" refusal names the asymmetry with
`btctax extension --pay` explicitly, since that is the flag with no ceiling.

**The manifest block** is written by `write_payment_voucher` directly after the stapling list and the
`ATT` lines and **before** the hand-marks block, opening with the constant `ENCLOSE_LOOSE_LINE` =
`"ENCLOSE LOOSE — do not staple: f1040v.pdf with the check"`. The KAT derives the stapling list from
the file (`take_while(|l| !l.contains("ENCLOSE LOOSE"))`) and asserts `f1040v` appears in none of it.

**Nothing hands either stem to `FiledPacket::stapled`**, held by
`no_code_path_pushes_the_4868_or_the_voucher_into_the_stapled_packet`: it reads `packet.rs`, slices
the body of `fill_full_return` (the only producer of the `Vec<NamedForm>` that reaches `stapled`), and
asserts neither stem literal appears in it — plus the four `attachment_sequence` values are `None`.
Kill W7 planted a `NamedForm { name: "f1040v", … }` push and it reds.

### Addition beyond the brief

`--pay` given **without** `--pay-by-check` was silently discarded. I did not make it a refusal —
refusing the whole packet over one inapplicable flag costs the filer every form — but the note now
says `(--pay $N was IGNORED: it sets Form 1040-V box 3, and no voucher was written)`. Kill W8.

### Pinned numbers moved

| where | old | new | cause |
|---|---|---|---|
| `docs/examples/examples.md` | — | **+1 line** | `btctax --help`'s command list gains `extension`. That is the WHOLE diff: `diff -u` shows one added line at `@@ -42,6 +42,7 @@`. No journey uses the new flags, and the no-authorisation notice's text is byte-identical after the constant extraction (which is how I know). |
| test count | 3109 | **3145** | +19 btctax-forms, +17 btctax-cli |

No `min_joins`, census, row-count, `BUNDLED.len()` or `LineSet::ALL` floor moved — T2/T3/T4 add no maps,
no stems and no census entries.

---

## 6. Two things I touched that the brief did not name

**6.1 `crates/btctax-core/src/tax/printed.rs:1624` — a false doc comment on the exact mechanism T2
depends on.** `schedule_3_lines` said it returns `None` *"when there is neither a foreign tax credit
nor an excess-Social-Security credit"*. It does not: the guard is `line8 <= 0 && line15 <= 0`, and
`line15 = line10 + line11` — so **an extension payment alone files the schedule**. A reader who
trusted the old sentence would conclude Form 4868 line 5's `− sch_3.line10` was dead code. Corrected
in place with the reason; **no code, no figure and no test result moves** (btctax-core: 1211/1211,
unchanged).

**6.2 `btctax_cli::NOT_AUTHORISED_FOR_FILING`.** Extracted from main.rs so the new `extension` arm
prints the same notice rather than a second copy. Text unchanged.

I did **not** touch the spec, `FOLLOWUPS.md`, or any other `design/agent-reports/*.md`.

---

## 7. Kills — every one seen RED on a planted defect (B1)

Plants were applied to a `cp` backup and restored with `cp` (never `git checkout --`/`git restore`);
each run then re-verified green. **Every plant reds exactly the test(s) it targets and no others**,
which is the half that makes the result mean something.

### T2 — Form 4868 (`f4868_fill.rs`, 13 tests)

| # | plant | red |
|---|---|---|
| M1 | line 6 loses its `max(ZERO)` | `payments_above_the_tax_print_an_explicit_zero_on_line_6`: `left: Some("-800") right: Some("0")` |
| M2 | line 4 written only when > 0 | `a_zero_line_4_prints_a_zero_…`: `left: None right: Some("0")` |
| M3 | line 5 always printed | same test: `left: Some("0") right: None` — *"line 5 carries NO -0- clause ⇒ a zero is BLANK"* |
| M4 | line 7 ignores the recorded extension payment | `line_7_defaults_to_the_printed_schedule_3_line_10_…`: `left: Some("3000") right: Some("500")` |
| M5 | line 5 subtracts Sch 3 **line 15** instead of line 10 | `an_excess_ss_credit_alone_…`: `left: Some("6200") right: Some("6500")` **and** `a_schedule_3_with_both_credits_…`: `left: Some("5700") right: Some("6000")` |
| M6 | the `--pay` guard removed | `a_negative_or_fractional_pay_is_refused_with_no_bytes` panics at the `expect_err` |
| M7 | spouse SSN gated on `spouse.is_some()` | `the_spouse_ssn_is_printed_on_mfj_and_blank_on_mfs_…`: `left: Some("987-65-4321") right: None`, *"MFS ⇒ line 3 blank; printing it would claim a joint return"* |
| M8 | line 9 (`c1_2`, censused) checked | `every_censused_cell_stays_blank_on_both_years` |
| M9 | **the line-8 write removed** (the brief's explicit mutation) | `out_of_country_checks_line_8_…`: `left: None right: Some("1")` |
| M10 | the geometry test's own plant made a no-op | `a_money_line_pointed_at_an_identity_cell_…`: *"the plant must actually change the map"* |
| M11 | the fill always loads the 2024 template | 3 tests red (the 2025 map's `PartI_ReadOrder` FQNs are absent from the 2024 PDF) |
| M12 | the name line never written | `the_name_address_and_taxpayer_ssn_cells_all_come_back_filled` + `both_bundled_years_…` |

### T4 forms side — Form 1040-V (`f1040v_fill.rs`, 6 tests)

| # | plant | red |
|---|---|---|
| V1 | spouse row gated on `spouse.is_some()` | `box_2_and_the_spouse_name_row_…`: `left: Some("987-65-4321") right: None` |
| V2 | the amount guards removed | `a_zero_negative_or_fractional_amount_is_refused` |
| V3 | an apartment number invented | `the_foreign_boxes_and_the_apartment_cell_…`: `left: Some("1B") right: None` |
| V4 | box 3's column check downgraded to `free` | `box_3_pointed_at_the_city_cell_fails_the_fill_closed` — the *planted-defect* test's own kill |
| V5 | box 3 prints `amount + 1` | 2 tests red |

★ V4's test also asserts that the planted target `f1_11` **exists** in the voucher's AcroForm before
filling — because T1 measured that four of Form 4868's field names collide with this form's, so an
existence check is necessary and not sufficient for this pair. Only the geometry oracle separates them.

### T3 — `btctax extension` (`extension.rs`, 10 tests)

| # | plant | red |
|---|---|---|
| E1 | the `manifest.txt` guard removed | `an_out_directory_holding_a_packet_manifest_is_refused` |
| E2 | the `--pay` guards removed | `a_negative_or_fractional_pay_is_refused_but_paying_above_line_6_is_not` |
| E3 | `require_attestation` removed | `a_pseudo_ledger_is_refused_without_the_phrase_…` (first half) |
| E4 | `stamp_draft_watermark` removed | same test, second half — the PAGE, read back from the written PDF |
| E5 | out-of-country = `return_due + 61 days` | TY2017: `left: 2018-06-17 right: 2018-06-15`; May-1: `left: 2025-06-15 right: 2025-06-16` |
| E6 | `out_of_country` ignored | TY2017: `left: 2018-04-17 right: 2018-06-15` |
| E7 | `past_due` never fires | `on_may_1_…` and the end-to-end KAT |
| E8 | bytes written before every refusal | **5 of 10 tests red** — every `wrote_nothing` assertion |
| E9 | `section_7503_shift` never shifts | `the_section_7503_shifter_…`: `left: 2025-06-14 right: 2025-06-16` |
| E10 | `recorded_extension_payment` dropped | the end-to-end KAT: `left: None right: Some(500)` |
| E11 | a blank line rendered as `$0` | `the_report_renders_a_blank_line_as_blank_and_never_as_zero` |
| E13 | the 4868 written with `std::fs::write` | `left: 420 right: 384` (0o644 vs 0o600) |

### T4 CLI side (`export_irs_pdf.rs`, 8 new tests)

| # | plant | red |
|---|---|---|
| W1 | the slice-year refusal removed | `pay_by_check_on_a_crypto_slice_year_refuses_naming_the_reason` |
| W2 | the `--pay` ceiling removed | `a_partial_payment_above_line_37_…` |
| W3 | the voucher prints without the flag | `a_return_that_owes_without_the_flag_gets_the_note_and_no_voucher` |
| W4 | the voucher joins the stapling list (`  ATT  f1040v.pdf`) | `pay_by_check_writes_a_voucher_beside_the_packet_…` |
| W5 | the voucher loses its watermark | `a_pseudo_voucher_is_gated_and_watermarked_like_the_packet` |
| W6 | a `$0` return still gets a voucher | `pay_by_check_on_a_refund_return_is_a_note_not_a_voucher` |
| W7 | a `NamedForm{name:"f1040v"}` pushed into `stapled` | `no_code_path_pushes_the_4868_or_the_voucher_…` (+2 collateral) |
| W8 | a discarded `--pay` goes unmentioned | `a_return_that_owes_without_the_flag_…` |

The end-to-end voucher KAT does not trust a constant for line 37: it reads **box 3 out of the written
voucher** and **line 37 out of the written `00_f1040.pdf`** (through the TY2024 1040 map) and asserts
the two strings are equal.

---

## 8. Commands run, with their summary lines

```
cargo nextest run --locked -p btctax-forms          → 363 tests run: 363 passed, 4 skipped   (was 344)
cargo nextest run --locked -p btctax-cli            → 679 tests run: 679 passed, 1 skipped   (was 662)
cargo nextest run --locked -p xtask                 → 129 tests run: 129 passed, 1 skipped
cargo nextest run --locked -p btctax-core           → 1211 tests run: 1211 passed, 0 skipped
cargo nextest run --locked -p btctax-store          → 45 tests run: 45 passed, 0 skipped
cargo nextest run --locked -p btctax-adapters       → 103 tests run: 103 passed, 0 skipped
cargo nextest run --locked -p btctax-input-form     → 65 tests run: 65 passed, 0 skipped
cargo nextest run --locked -p btctax-oracle-harness → 5 tests run: 5 passed, 1 skipped
cargo nextest run --locked -p btctax-tui            → 159 tests run: 159 passed, 2 skipped
cargo nextest run --locked -p btctax-tui-edit       → 381 tests run: 381 passed, 2 skipped
cargo nextest run --locked -p btctax-update-prices  → 5 tests run: 5 passed, 1 skipped
                                                      TOTAL 3145  (HEAD cf58b4af was 3109; +36)

cargo fmt --all && cargo fmt --all --check                                    → clean
CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets \
    --all-features -- -D warnings                                             → no diagnostics
cargo run -q -p xtask -- docs                                                 → +btctax-extension.1, 2 pages updated
cargo run -q -p xtask -- examples > docs/examples/examples.md                 → +1 line (§5)
./target/debug/btctax extension --help                                        → renders
```

The whole workspace was never run in one invocation, per the brief; `make check` is the controller's
gate. Builds went to the default `target/` (clippy to `target-clippy`, as `make check` does).

---

## 9. Decisions, and what I could not do

1. **`screen_full_return`** (§2) — the one refactor of a shipped path. Justified because R2's
   "reused as they stand" is otherwise a promise, not a mechanism. Order preserved; the attestation
   gate deliberately left outside the helper.
2. **`export_irs_pdf` gained a parameter** rather than an overload (§2), at the cost of 51 mechanical
   test-call edits. The compiler verified every one, and it caught the two the script got wrong.
3. **`FormsError::InvalidValue`** rather than reusing `Structure` (which is documented as *"bundled PDF
   structure error"*) — a filer's bad `--pay` is not a corrupt template.
4. **`_with_map` fault-injection seams** on both fillers, exported through `testonly`, matching
   `fill_form_8959_with_map`. Without them "a mis-mapped cell fails closed" could never be watched
   discriminating (B1) — a committed map cannot be corrupted in place.
5. **`--pay` without `--pay-by-check` is a note, not a refusal** (§5). Recorded as a judgment call.
6. **The clock is passed as an argument in tests**, not through `BTCTAX_NOW`. The env var is a
   process-wide global and these tests run in parallel; `main.rs` fills the argument from
   `resolve_now()`, which is where the seam belongs. The end-to-end KAT pins `2026-02-01T12:00Z`.
7. **Not done, and not mine:** T6 (`report` naming the paths, help-text cross-references beyond the two
   flags' own `--help`). `YearReadiness::sentence` is untouched, as the brief requires.
