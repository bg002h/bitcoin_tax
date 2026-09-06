# Fold — the 4868/1040-V seam review (0C/4I/5M/2N)

Single implementer, shared main tree, branch `main`, base HEAD `e423b44e`. Nothing committed, nothing
pushed. The review report and its ledger were not touched.

Report folded: `design/agent-reports/2026-09-06-build-4868-T1-T6-review.md` (findings from line 244).
Brief: `design/agent-reports/BRIEF-fold-4868-review.md`.

Every finding is folded: **I-1, I-2, I-3, I-4** (blocking), **M-1, M-2, M-3, M-4, M-5**, **N-1**.
**N-2** required nothing and got nothing.

---

## I-1 (Important) — a refused `--pay` left a packet directory with no `manifest.txt`

**Change.** All four `--pay` refusals moved out of `write_payment_voucher` and into
`export_full_return`, immediately after `assemble_printed_return` — the first moment Form 1040
line 37 exists, and still before `fill_full_return` / `mkdir_out`.

- `crates/btctax-cli/src/cmd/admin.rs:1327-1391` — the hoisted block (negative, above line 37, cents,
  and M-2's `$0`), guarded `if voucher.pay_by_check && printed.forms.f1040.line37 > Usd::ZERO`.
- `crates/btctax-cli/src/cmd/admin.rs:1092-1102` — `write_payment_voucher`'s amount is now
  `voucher.pay.unwrap_or(owed)` plus a `debug_assert!` restating the upstream invariant, with a
  comment saying nothing may be added back there. Its doc comment (`:1035-1041`) says where the
  refusals live now and why.

**Deviation from the review's "minimal change", stated plainly.** The review said to run the shape
pair (negative/cents) "in `export_irs_pdf_from_session` beside the existing pre-byte slice refusal
(`admin.rs:683`)". That slot is *below* the full-vs-slice dispatch, so a check placed literally there
would never run on the full-return path — the only path that writes a voucher, and the path the
defect is on. I put all four in `export_full_return` instead.

The guard `pay_by_check && owed > 0` is deliberate: it reproduces `write_payment_voucher`'s **exact
old reachability**, so the fold changes only the *moment* of each refusal and never widens one. In
particular `--pay 100.25` *without* `--pay-by-check` is still the "IGNORED" note, not a refusal —
refusing the whole packet over an inapplicable flag is the outcome the existing code deliberately
rejected, and the review did not ask to reverse it.

**Kills.** `crates/btctax-cli/tests/export_irs_pdf.rs`
- `:2326 every_pay_refusal_is_pre_byte_and_leaves_the_out_directory_untouched` — a **fresh** `--out`
  per refusal (`wrote_nothing` cannot tell "wrote nothing" from "wrote nothing new"), each asserting
  the message, `CliError::Usage`, and an untouched directory; then the same call with `--pay 1000`
  writes, so a command that refused everything cannot pass.
- `:2397 a_refused_pay_does_not_leave_a_packet_the_extension_guard_reads_as_empty_ground` — the
  defect as the filer meets it: refuse `--pay 100.25`, then run `btctax extension` into the same
  directory and assert the listing is **exactly** `["f4868.pdf"]`.

**Seen red.** Plant = revert the fold (disable the hoisted block; restore the old match inside
`write_payment_voucher`), `cp`-backup restore afterwards. Both tests FAILED, the second printing the
review's own evidence:

```
assertion `left == right` failed: ★ the extension application is ALONE — a return form beside it is the defect
  left: ["f4868.pdf", "72_f8960.pdf", "71_f8959.pdf", "12A_f8949.pdf", "12_schedule_d.pdf", "02_f1040s2.pdf", "00_f1040.pdf"]
 right: ["f4868.pdf"]
```

(no `manifest.txt` in that listing — which is precisely why `extension`'s guard did not fire.)

---

## I-2 (Important) — the "no 4868 in the return's envelope" guard was one-directional

**Change.** `crates/btctax-cli/src/cmd/admin.rs:652-668` — at the top of
`export_irs_pdf_from_session`, **above the dispatch**, a refusal when `out_dir.join("f4868.pdf")`
exists, quoting the form's page 2 (*"Don't attach a copy of Form 4868 to your return."*) and naming
the remedy.

**Deviation.** The brief said "in `export_full_return` AND the slice arm". One check above the
dispatch covers both arms with one message that cannot drift; `export_full_return` has exactly one
caller (`admin.rs:667`), so this is strictly equivalent and strictly harder to break. It also fires
earlier than `extension`'s mirror (which sits after its screens) — deliberate: the directory is a
filer mistake that needs no return computed to detect.

**Kill.** `crates/btctax-cli/tests/export_irs_pdf.rs:2448
a_directory_already_holding_an_f4868_refuses_the_return_packet_on_both_pipelines` — seeds
`f4868.pdf`, asserts the refusal names the file and quotes the form, and asserts the directory
listing is still exactly `["f4868.pdf"]`. Both pipelines (full-return via `owing_vault`, crypto slice
via `make_vault`).

**Seen red.** Plant = `if false && out_dir.join("f4868.pdf").exists()`. FAILED, and the failure dump
shows the whole packet plus `f1040v.pdf` plus `manifest.txt` written around the extension
application — the unguarded direction, exactly as reported.

---

## I-3 (Important) — the Approach-B experimental notice was missing on `extension`

**Change.**
- `crates/btctax-cli/src/cmd/admin.rs:1674-1684` — `ExtensionReport::experimental_notice_active`,
  documented with the module's own two-class rule and the line-4 dependence chain.
- `crates/btctax-cli/src/cmd/admin.rs:1865` —
  `experimental_notice_active: btctax_core::experimental::uses_approach_b(events)`, the same
  expression the other three surfaces use.
- `crates/btctax-cli/src/main.rs:788-796` — the same two-line `eprint!` the export arms use
  (`main.rs:734` / `:875`), directly after the no-authorisation notice.

**Kill.** `crates/btctax-cli/tests/experimental_notice.rs:680
extension_notice_reaches_stderr_not_stdout_and_not_at_all_when_voided` — a promoted-tranche vault
through the real binary: notice on stderr (title **and** summary), absent from stdout, absent from
the written `f4868.pdf`; then a declared-then-**voided** tranche on the same year: the form is still
written and the notice is not printed.

**Seen red, both directions.**
- delete the `eprint!` → `the notice must reach stderr on the extension: "…NOT AUTHORISED FOR
  FILING…"` (only the other notice present).
- make it `if true || …` → `★ a voided-only tranche must not trigger the notice on the extension
  either: …EXPERIMENTAL — DEFENSIVE FILING…`.

---

## I-4 (Important) — the journey row's closing note did not exist

**Change.** `crates/btctax-cli/src/render.rs:5220-5253` — `render_extension` gains an `else if`
arm on the `recorded_extension_payment` branch: whenever line 7 is `Some(> 0)` and the return records
no extension payment, a note naming the amount, the field the way the input surface names it
(*"Amount paid with a Form 4868 extension request"* → Schedule 3 line 10,
`payments.extension_payment` in `btctax income import`, or the TUI input form's Payments section),
and the consequence (October's line 33 short, line 37 high, and the Form 1040-V asking for the same
money a second time).

`Form4868Lines::line7` is already `None` when the value is zero (`form4868.rs:165`), so the
`.filter(|p| *p > Usd::ZERO)` is belt-and-braces rather than load-bearing.

**Kill.** `crates/btctax-cli/tests/extension.rs:547
a_payment_the_return_does_not_yet_record_gets_the_closing_note_and_nothing_else_does` — three states
in one test: (a) paying with nothing recorded ⇒ the note, and it names the line, the field and the
"second time" consequence; (b) the payment already recorded ⇒ the I-6 note and **not** this one;
(c) line 7 blank (estimated payments cover the estimate) ⇒ silence.

**Seen red, both directions.** Neutering the condition → `the closing note must be there: …`.
Changing `else if` to an unconditional `if` → `★ a payment the return already carries must NOT be
nagged about: …`.

---

## M-1 — `--pay` without `--pay-by-check` was silent on the crypto slice

**Change.**
- `crates/btctax-cli/src/cmd/admin.rs:982-993` — the slice arm's `form_1040v_note` is now
  `voucher.pay.map(...)`: *"--pay $N was IGNORED: … {year} has no full-return inputs, so there is no
  Form 1040 line 37 for box 3 to carry (see `income import`)."*
- `crates/btctax-cli/src/main.rs:980-993` — the `form_1040v_note` print moved **out** of the
  full-return arm to just after the dispatch branch, so both pipelines reach it. It was the last
  statement of that arm, so the full-return output order is unchanged.

**Kill.** `crates/btctax-cli/tests/export_irs_pdf.rs:2519
a_pay_without_the_flag_is_noted_on_the_crypto_slice_arm_too` — asserts the premise (slice arm), the
note's content, that it **reaches stderr through the real binary** (`CARGO_BIN_EXE_btctax`), and that
a run with no `--pay` is silent.

**Seen red, both halves.** `voucher.pay.map(...)` → `None` gives *"a discarded --pay is never silent,
on either pipeline"*. Putting the `main.rs` print back inside the full-return arm gives *"★ the note
must reach the filer's screen on the slice arm, not just the report struct"* — i.e. the struct-only
assertion alone would have passed a note nobody prints.

---

## M-2 — `--pay 0 --pay-by-check` on a return that owes

Folded into I-1's hoisted block (`admin.rs:1370-1381`): a `Usage` refusal naming the flag —
*"--pay $0 writes no voucher — a voucher for $0 is not a payment, and this return owes $N (Form 1040
line 37). Drop --pay-by-check if you are not paying by check, or name the amount you are paying."*
It replaces `fill_form_1040v`'s `FormFill` refusal, whose sentence described the wrong state, and it
is now pre-byte like the other three. Kill: the fourth `refuse(...)` case in
`every_pay_refusal_is_pre_byte_…`, red under the same I-1 plant.

---

## M-3 — `report` named neither instrument

**Spec edit (the one permitted).** `design/SPEC_form_4868_1040v.md` T6.

- old: `- **T6 — the surfaces.** \`report\` names the extension and voucher paths; \`YearReadiness::sentence\` unchanged; the help text names both \`--pay\` flags with their different ceilings and defaults.`
- new: `- **T6 — the surfaces.** Each command's own report names its path (\`extension\` prints the \`f4868.pdf\` it wrote; \`export-irs-pdf\` lists \`f1040v.pdf\` separately from the stapled packet), and \`report\`'s AMOUNT OWED line points at both commands by name; \`YearReadiness::sentence\` unchanged; the help text names both \`--pay\` flags with their different ceilings and defaults.`

**Code.** `crates/btctax-cli/src/render.rs:1650-1662` — two pointer lines directly under
`→ AMOUNT OWED (L37)` in `render_dual_report`, naming
`btctax export-irs-pdf --tax-year <y> --pay-by-check` and `btctax extension --year <y> --out <dir>`
(with *"it never extends the time to PAY"*).

**Deviation.** The brief located this "beside the existing `-> btctax export-irs-pdf` hint
(`render.rs` ~4752)". That hint is inside `render_defensive_status` (the `defensive status`
dashboard), not `btctax report` — `render_report` has no owed-year section at all, and the finding's
own complaint is that *`btctax report --tax-year`* mentions neither form. The brief's kill ("the
report of an owing year contains both names") settles it: the pointer went where the filer learns
they owe, i.e. the dual report's AMOUNT OWED arm. `render_defensive_status` was left alone.

**Kill.** `crates/btctax-cli/tests/tax_report.rs:1613
an_owing_year_names_the_voucher_and_the_extension_and_a_refund_year_does_not` — $80k wages with no
withholding ⇒ owes ⇒ both names present; the same fixture with $50k withheld ⇒ refunds ⇒ neither
name present.

**Seen red, both directions.** Deleting the `writeln!` → *"the payment voucher must be named, with
the flag that writes it"*. Printing it on the refund arm too → *"★ a refund year must not be pointed
at a payment voucher"*.

**Golden.** `docs/examples/examples.md`, regenerated with
`cargo run -q -p xtask -- examples > docs/examples/examples.md`. Exactly two added lines, after
`  → AMOUNT OWED (L37):      16677.00` at line 1086:

```
+    -> btctax export-irs-pdf --tax-year 2024 --pay-by-check   (Form 1040-V, the payment voucher)
+    -> btctax extension --year 2024 --out <dir>                (Form 4868, if you need more time to FILE — it never extends the time to PAY)
```

No man page regeneration: no clap doc comment or help text changed (`xtask docs` not run; the three
`xtask::examples` golden tests pass unchanged apart from the two lines above).

---

## M-4 — "watermarked on EVERY page" had no test that could red

**Change.** `crates/btctax-cli/tests/extension.rs:339-364` (inside
`a_pseudo_ledger_is_refused_without_the_phrase_and_watermarked_with_it`) — parse the written PDF with
`btctax_forms::testonly::load`, assert the page count is **4** as a premise, then assert the marker
in **every** page's decoded content stream via `doc.get_page_content(pid)`. No new dependency:
`testonly::load` is already re-exported and `Cargo.lock` is untouched (`--locked` throughout).

**Seen red.** Plant = `watermark.rs:71` `.take(1)` on the page list. FAILED with
`★ page 2 of 4 carries no DRAFT watermark — a fictional estimate may not have an unmarked page` —
and the pre-existing whole-file `contains_bytes(&bytes, b"NOT FOR FILING")` assertion, which runs
first, **passed** under that plant. That is the blindness M-4 named, observed.

---

## M-5 — the anti-stapling gate's string-literal leg

Comment only, `crates/btctax-cli/tests/export_irs_pdf.rs:2683-2694`: names what the grep cannot see
(a push built from `Stem::F1040v.file_stem()` evades it; a future `#[cfg(test)]` mention reds it
spuriously) and names the test that holds the guarantee for real
(`pay_by_check_writes_a_voucher_beside_the_packet_with_line_37_in_box_3`, which reads the written
manifest). **No kill**: no guarantee was added or changed, only prose.

---

## N-1 — the hand-list of three sequence-less stems

**Change.** `crates/btctax-forms/tests/map_rows.rs:300-334` — the shape assertion's right-hand side
is now `printed_sequence(&text).is_none()`, computed from the same archived extract kill 4 reads,
with the message kept verbatim.

**One thing beyond the brief, deliberately.** A literal `if let Ok(text) = read_to_string(...)`
would make the check *silently skippable* — a wrong path and it never runs, which is the exact
green-and-blind shape B1 exists for. So rows are skipped **by name** from the already-asserted,
shrink-only `SEQUENCE_UNVERIFIABLE` list (the five TY2017 templates with no archive), and for every
other row a failed read **panics** with the path.

**Seen red, both ways.** Inverting to `.is_some()` → `2024/f1040: the sequence number is absent
exactly on the rows whose extract prints none — …`. Breaking the extract path (`.NOPE`) →
`2024/f1040: its archived extract must be READABLE at …/f1040--2024.NOPE — the sequence shape is
computed from the form, never skipped`. Without the second change that plant would have passed green.

---

## N-2 — nothing to change

`instr_pages` is argued in the T1 report and the spec pins no numbers. No edit.

---

## Validation

All via `cargo nextest run --locked` (never `cargo test`, never `--release`, never the whole
workspace). Baseline at `e423b44e` was 3145 for the full suite; below are the crates this fold
touched, plus the two TUI crates as a regression check.

| command | summary |
|---|---|
| `cargo nextest run --locked -p btctax-cli -p btctax-forms -p xtask` | `1178 tests run: 1178 passed, 6 skipped` (21.8s) |
| `cargo nextest run --locked -p btctax-tui -p btctax-tui-edit` | `540 tests run: 540 passed, 4 skipped` (1.9s) |

Scoped runs used during the fold, each green at the end:

| command | summary |
|---|---|
| `-p btctax-cli -E 'binary(export_irs_pdf)'` | `41 tests run: 41 passed, 0 skipped` |
| `-p btctax-cli -E 'binary(extension)'` | `11 tests run: 11 passed, 0 skipped` |
| `-p btctax-cli -E 'binary(experimental_notice)'` | `12 tests run: 12 passed, 0 skipped` |
| `-p btctax-forms -E 'binary(map_rows)'` | `11 tests run: 11 passed, 0 skipped` |

- `cargo fmt --all` — clean (it reformatted two of the new test blocks; re-run is a no-op).
- `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — **clean**, whole workspace.
- `Cargo.lock` unchanged; every run used `--locked`.

**Not run here** (the controller's pre-commit gate does it): `make check` and the CI-only jobs
(fmt/msrv/pii-scan/net-isolation).

## Files touched

```
crates/btctax-cli/src/cmd/admin.rs             (I-1, I-2, I-3, M-1, M-2)
crates/btctax-cli/src/main.rs                  (I-3, M-1)
crates/btctax-cli/src/render.rs                (I-4, M-3)
crates/btctax-cli/tests/export_irs_pdf.rs      (I-1, I-2, M-1, M-2, M-5)
crates/btctax-cli/tests/extension.rs           (I-4, M-4)
crates/btctax-cli/tests/experimental_notice.rs (I-3)
crates/btctax-cli/tests/tax_report.rs          (M-3)
crates/btctax-forms/tests/map_rows.rs          (N-1)
design/SPEC_form_4868_1040v.md                 (M-3 — the one spec edit)
docs/examples/examples.md                      (M-3 — regenerated, +2 lines)
```

Nothing was committed or pushed. No file was restored with `git checkout --` / `git restore`; every
plant was reverted from a `cp` backup in the session scratchpad.
