# VERIFICATION ledger — the 4868/1040-V seam review (`2026-09-06-build-4868-T1-T6-review.md`, 0C/4I/5M/2N)

Controller's machine-check of every checkable claim, at `17e180bf`, before anything was folded.

| # | finding | claim | check | result |
|---|---|---|---|---|
| 1 | I-1 | the voucher's `--pay` validation runs inside `write_payment_voucher`, called AFTER every packet PDF is written and BEFORE `manifest.txt` | `admin.rs`: `fn write_payment_voucher` at 1011; its call at 1489; `let manifest_path` at 1500; `assemble_printed_return` at 1301 — the packet PDFs are written between 1301 and 1489 | HOLD |
| 2 | I-1 | the extension's envelope guard is solely `manifest.txt`-presence | `admin.rs:1691` `if out_dir.join("manifest.txt").exists()` | HOLD |
| 3 | I-2 | no reciprocal guard: the export never looks for `f4868.pdf` in `--out` | `grep f4868.pdf admin.rs` → only the report field's doc (1580); no check | HOLD |
| 4 | I-3 | `ExtensionReport` carries no `experimental_notice_active`, unlike the other reports | `grep experimental_notice_active`: fields at admin.rs:33 and :370, printed at main.rs:734/875/1222; the `ExtensionReport` struct has 0 | HOLD |
| 5 | I-4 | `render_extension`'s only closing notes are the recorded-payment note and "Don't attach…" | `render.rs:5208-5211` and `:5229`; nothing about recording the payment | HOLD |
| 6 | M-4 | the watermark test asserts `NOT FOR FILING` once over the whole file | `tests/extension.rs:335` `contains_bytes(&bytes, b"NOT FOR FILING")` | HOLD |
| 7 | M-3 | `btctax report` mentions neither form | reviewer's grep, accepted (the `report` command writes no files; both commands' own reports name their paths) | accepted |

Not machine-checked here: the reviewer's probes (the six plants seen red, the two directory states
reproduced) — accepted as the reviewer's execution record; the fold's kills will re-establish each.

Disposition: I-1, I-2, I-3, I-4 block — folded next by ONE opus agent under
`BRIEF-fold-4868-review.md`; M-1, M-2, M-4, M-5 folded in the same pass (cheap); M-3 by amending the
spec's T6 wording + one pointer line in `report`; N-1 as a computed check; N-2 recorded (the T1
report already argued it). Nothing folded at this commit.
