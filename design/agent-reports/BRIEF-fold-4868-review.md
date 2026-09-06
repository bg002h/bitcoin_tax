# Brief — fold the 4868/1040-V seam review (0C/4I/5M/2N)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore` files you did not
create. Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never `cargo test`, never
`--release`, never the whole workspace); `cargo fmt --all` and a clean
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee you add needs a kill seen red once; say how in the report.

Read first: `design/agent-reports/2026-09-06-build-4868-T1-T6-review.md` (findings from line 244) and
its ledger `…-review-VERIFICATION.md` (every checkable claim HOLDS — fold, do not re-verify). The
spec is `design/SPEC_form_4868_1040v.md` (R2, R4, the journey table at lines 162–179).

## Fold, item by item (the review's "Minimal change" is the default)
- **I-1** Hoist the three `--pay` validations (negative, cents, above line 37) out of
  `write_payment_voucher` (`crates/btctax-cli/src/cmd/admin.rs:1011`) so they run BEFORE any byte:
  negative/cents beside the existing pre-byte slice refusal in `export_irs_pdf_from_session`; the
  ceiling at the top of `export_full_return` right after `assemble_printed_return` (:1301). Kill: a
  KAT per refusal asserting the `--out` directory is unchanged (the `extension.rs::wrote_nothing`
  shape). Also fold **M-2** here: `--pay 0 --pay-by-check` on a return that owes → a `Usage` refusal
  naming the flag, before any byte.
- **I-2** In `export_full_return` AND the slice arm, before any byte: if `out_dir.join("f4868.pdf")`
  exists → refuse with the mirror of R2's message (the 4868 is mailed separately; page 2 says "Don't
  attach a copy of Form 4868 to your return."). Kill: seed `f4868.pdf`, assert the refusal and that no
  packet file appeared.
- **I-3** `ExtensionReport` gains `experimental_notice_active: btctax_core::experimental::uses_approach_b(events)`
  (the same expression the other reports use, admin.rs:271) and the `Command::Extension` arm in
  `main.rs` prints the notice exactly as the export arms do (main.rs:734/875). Kill: a promoted-tranche
  fixture through `extension` shows the notice; a plain one does not.
- **I-4** `render_extension` (`crates/btctax-cli/src/render.rs` ~5164–5231): whenever line 7 is
  `Some` and > 0 AND no extension payment is recorded on the return, emit a closing note telling the
  filer to record it — name the field the way the input form does ("Amount paid with a Form 4868
  extension request → Schedule 3 line 10"; `income import`'s `payments.extension_payment`, or the TUI
  input form's Payments section) — so October's line 33 carries it and the voucher does not ask for
  it twice. Kill: the note present in that state, absent when the payment is recorded, absent when
  line 7 is zero.
- **M-1** the "--pay was IGNORED" note also on the crypto-slice arm (or in the dispatcher above the
  branch).
- **M-4** `tests/extension.rs:335`: assert the watermark marker in EVERY page's content stream, with a
  premise assertion that the page count is 4.
- **M-5** a one-line comment in `export_irs_pdf.rs` (~2386) naming what the literal grep cannot see
  and which test holds the guarantee for real.
- **M-3** amend the spec's T6 sentence to say "each command's own report names its path" (a
  one-line edit in `design/SPEC_form_4868_1040v.md` T6 — the ONE spec edit you may make, quote the
  old and new sentence in your report) and add one pointer line in `btctax report`'s owed-year
  section beside the existing `-> btctax export-irs-pdf` hint (`render.rs` ~4752) naming
  `btctax extension` and `--pay-by-check`. Kill: the report of an owing year contains both names.
- **N-1** `map_rows.rs:307`: replace the hand-list of three sequence-less stems with a check computed
  from the extract (`printed_sequence(extract).is_none()` ⇔ `attachment_sequence.is_none()`), which
  kill 4 already reads; keep the message.
- **N-2** nothing to change (recorded in the T1 report).

## Constraints
Do not touch the review report or its ledger. `docs/examples/examples.md` and the man pages move only
as a direct consequence (the new `report` pointer line will move the examples golden on owing-year
journeys — regenerate with `cargo run -q -p xtask -- examples > docs/examples/examples.md` and list
the diff lines; `cargo run -q -p xtask -- docs` for man pages if any help text changed).

## Report — your FINAL action
`design/agent-reports/2026-09-06-build-4868-T1-T6-fold-implementation.md`: per finding the change
(file:line), the kill and how it was seen red, deviations with reasons, the golden diff lines, the
exact nextest commands with summary lines. Return ONLY a 3-line summary (what landed, suite results
for the crates you touched, the report path).
