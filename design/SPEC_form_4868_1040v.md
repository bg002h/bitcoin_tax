# SPEC — Form 4868 (extension) and Form 1040-V (payment voucher) fillers (FR-49 / strategy review S8)

**Status: DRAFT r1 (2026-09-06), for review to 0C/0I before build.** Owning phase: NOW — the physical
rehearsal (S1, an owner decision) and the first filed year (TY2026, due 2027-04-15) both walk the
extension and the payment envelope; today btctax can print neither.

## Why this exists

The paper journey is not "print the return". A filer who cannot finish by the due date files Form 4868
**with a payment**; a filer who owes and pays by check encloses Form 1040-V **loose** in the same
envelope as the return. Both are one-page AcroForms whose every box is either already on the return or
a single filer choice. Both revisions for TY2025 are archived under authority (FR-49; `design/forms/2025/
f4868--2025.pdf`, `f1040v--2025.pdf`, extracts alongside). Neither is in `Stem`, so neither reaches the
census, the row kills, the label reader or the packet — the "forms derive the interview" invariant
(field provenance census) has two forms it does not see.

## What the forms ask — transcribed, with provenance (the TEXT layer, `design/forms/extract/`)

**Form 4868 (2025)**, `f4868--2025.txt:83-92` and the instructions on its own pages 2–4. 17 AcroForm
boxes (`xtask dump-fields`, 2026-09-06):

| line / box | printed text | provenance |
|---|---|---|
| header `VoucherHeader.f1_1/f1_2/f1_3` | *"For calendar year 2025, or other tax year beginning ___, 2025, ending ___, 20__"* | **blank, recorded**: btctax is calendar-year only |
| 1 `f1_4` | *"Your name(s) (see instructions)"* | `ReturnHeader.name_line` (the 1040's own name line) |
| — `f1_5` | *"Address (see instructions)"* | `ReturnHeader.address_street` |
| — `f1_6` / `f1_7` (maxlen 2) / `f1_8` (maxlen 10) | *"City, town, or post office / State / ZIP code"* | `address_city` / `address_state` / `address_zip` |
| 2 `f1_9` (maxlen 11) | *"Your social security number"* | `taxpayer.ssn`, rendered as the 1040 renders it |
| 3 `f1_10` (maxlen 11) | *"Spouse's social security number"* | `spouse.ssn`; blank when no spouse (the inputs say so) |
| 4 `f1_11` | *"Estimate of total tax liability for 2025"* — instructions: *"Enter on line 4 the total tax liability you expect to report on your 2025: • Form 1040, 1040-SR, or 1040-NR, line 24"*; *"If you expect this amount to be zero, enter -0-."* | **computed**: the year's return as it stands today, line 24 (`Form1040Lines.line24`). The return IS the estimate; a year whose return cannot be computed has no estimate → refuse (R2) |
| 5 `f1_12` | *"Total 2025 payments"* — *"Enter on line 5 the total payments you expect to report on your 2025: • Form 1040 … line 33 (excluding Schedule 3, line 10)"*; *"Don't include on line 5 the amount you're paying with this Form 4868."* | **computed**: `line33` − Schedule 3 line 10. Schedule 3 line 10 is this very payment, and R3 refuses when one is already recorded, so on this path it is 0 by construction — the subtraction is still written, because the instruction says it |
| 6 `f1_13` | *"Balance due. Subtract line 5 from line 4."* — *"If line 5 is more than line 4, enter -0-."* | **computed**: `max(0, L4 − L5)`; a negative prints `0` (the form says enter -0-, an explicit zero, not a blank) |
| 7 `f1_14` | *"Amount you're paying (see instructions)"* — *"If you find you can't pay the amount shown on line 6, you can still get the extension. But you should pay as much as you can…"* | **collected**: `--pay <usd>`, default = line 6. A filer choice the form names as one |
| 8 `c1_1` | *"Check here if you're 'out of the country' and a U.S. citizen or resident."* | **collected**: `--out-of-country`, default unchecked. Silence FORGOES the two extra months; it asserts nothing (the answered-ness sharp test) |
| 9 `c1_2` | *"Check here if you file Form 1040-NR and didn't receive wages as an employee subject to U.S. income tax withholding."* | **never, recorded**: btctax produces no Form 1040-NR |
| page 3 `Col4.f3_1` | *"Enter confirmation number here:"* (the electronic-payment confirmation, on the instruction page) | **never, recorded**: the filer's private record, not part of the filing |

**Form 1040-V (2025)**, `f1040v--2025.txt:66-84`. 15 AcroForm boxes:

| box | printed text | provenance |
|---|---|---|
| 1 `f1_1` (maxlen 11) | *"Your social security number (SSN) (if a joint return, SSN shown first on your return)"* | `taxpayer.ssn` |
| 2 `f1_2` (maxlen 11) | *"If a joint return, SSN shown second on your return"* | `spouse.ssn`; blank when no spouse |
| 3 `f1_3` | *"Amount you are paying by check or money order."* | **computed**: Form 1040 line 37 (`Form1040Lines.line37`, "Amount you owe"); `--pay <usd>` overrides for a partial payment (a filer choice; never more than line 37 — a larger figure is refused, it is not a payment the return asks for) |
| 4 `f1_5` / `f1_6` | *"Your first name and middle initial / Last name"* | `taxpayer.first_name` / `last_name` — the same cells and rendering as the 1040 header |
| — `f1_7` / `f1_8` | *"If a joint return, spouse's first name and middle initial / Last name"* | `spouse.*`; blank when no spouse |
| — `f1_9` / `f1_10` / `f1_11` / `f1_12` (maxlen 2) / `f1_13` (maxlen 10) | *"Home address (number and street) / Apt. no. / City, town, or post office … / State / ZIP code"* | the 1040 header's address cells, same provenance and same apt-number handling as the 1040 map records |
| — `f1_14` / `f1_15` / `f1_16` | *"Foreign country name / Foreign province/state/county / Foreign postal code"* | **blank, recorded**: `ReturnHeader` carries no foreign address; the same reason the 1040 map records for its own foreign-address boxes |

Both forms print **no** "Attachment Sequence No." (the extracts show none): neither is attached to the
return. The 1040-V says so itself: *"Do not staple or attach this voucher to your payment or return."*

## The rule

**R1 — two new stems, two new rows, zero new hand-lists.** `Stem` gains `F4868` and `F1040v`; each gets
a `forms/2025/<stem>.map.toml` ROW (`irs_stem`, `versioning = "annual"`, `template_sha256`, the archived
`authority` manifest entry, `instructions = "on-form"` with `instr_pages` naming pages 2–4 / page 1,
`line_set = "f4868/2025"` / `"f1040v/2025"`, **no** `attachment_sequence`) plus its `[bindings]` (one
key per line above, in the form's numbering) and `[census]` (every one of the 17 / 15 boxes mapped or
`no` with the reason in the table). `LineSet` gains the two revisions; `Schema` gains `Form4868Map` and
`Form1040VMap`; every exhaustive `match` on `Stem` reds until each site decides. The TY2024 and TY2017
records list both as absent (*"fillers begin at TY2025"*) — the filable-year absence rule
(`first bundled year of the stem > year`) already admits that. `forms/2025/YEAR.toml`'s expected count
pins 15 → 17.

**R2 — Form 4868 is a command, not a packet member.** `btctax tax extension --year <Y> --out <DIR>
[--pay <usd>] [--out-of-country]` writes `f4868.pdf` (owner-only, like `export-irs-pdf`) and prints
what it filled, line by line. Refusals, in order:
- the year is not filable or its return is uncomputable → the `YearReadiness` sentence (no estimate
  exists; the instructions demand one *"as accurate as you can"*);
- `extension_payment` is already recorded for the year (R3) → *"an extension payment of $N is already
  on the return; Form 4868 is filed once"*;
- `--pay` negative → refuse; `--pay` above line 6 → allowed (paying ahead is the filer's choice).
Warnings: the clock (`BTCTAX_NOW`) is past the year's due date → *"the due date was YYYY-04-15; a
Form 4868 filed after it does not extend anything"* — printed, not refused (a useless form is not worse
than silence). Line 6 = 0 with a nonzero line 7 → allowed (the filer may pay ahead).

**R3 — the return must carry the payment: Schedule 3 line 10.** `ReturnInputs` gains
`extension_payment: Option<Usd>` — *"Amount paid with request for extension to file"* is a filer
assertion of a payment actually made, collected like any other 1099-shaped fact (`income import` TOML,
the TUI input form). `None` = no entry on Schedule 3 line 10, the side that never understates tax.
`tax extension` ends with the reminder *"record the amount you actually paid as `extension_payment`
before filing"*; it does NOT write the vault (a printed voucher is not a payment). T3 first measures
Schedule 3 line 10's provenance today; if it is a structural blank, it becomes this collected value.

**R4 — Form 1040-V rides with `export-irs-pdf`, loose.** When the return's line 37 > 0 AND the filer
passes `--pay-by-check`, the export writes `f1040v.pdf` **beside** the packet and lists it in the
manifest as *"enclose loose with the check — do not staple"*. It is never a `FiledPacket` member
(`stapled` cannot hold it; test). Without `--pay-by-check` nothing is written and the export notes
*"amount owed $N — Direct Pay / EFTPS need no voucher; pass --pay-by-check for Form 1040-V"*. With the
flag and line 37 = 0 → a note, no voucher. `--pay <usd>` overrides the amount, never above line 37.

**R5 — TY2026.** Both are annual forms; the TY2026 revisions arrive with the January 2027 package and
follow the port runbook as two more rows (the `LineSet` arms `f4868/2026`, `f1040v/2026`). Nothing
here is TY2025-specific except the archived templates.

## Journey walk — the divergences, classified

| the filer… | outcome | class |
|---|---|---|
| runs `tax extension` on a year with no stored return inputs | refusal: no estimate | refusal |
| runs it after April 15 | warning, form still written | warning |
| omits `--pay` | line 7 = line 6 | default |
| pays less than line 6 | allowed; the form's own instruction covers it | not our concern |
| runs it twice | second run refused once `extension_payment` is recorded; before that, a second identical PDF (harmless) | refusal / not our concern |
| files the 4868, pays, then exports the return without recording `extension_payment` | Schedule 3 line 10 blank, line 37 too high by the payment; the reminder at the end of `tax extension` is the only guard | documentation only (over-payment self-corrects at the IRS; never understates) |
| passes `--pay-by-check` on a refund return | note, no voucher | default |
| staples the voucher to the return | the manifest line says not to | documentation only |
| has a foreign address | already refused upstream by the 1040 header | not our concern |

## Plan (TDD — every task lands with its kill)

- **T1 — the rows.** `Stem::{F4868, F1040v}`; the two `.map.toml` rows with bindings + census; `LineSet`/
  `Schema` arms; year records (2025 expected 17; 2024/2017 absent with reason); `map_rows` kills run
  free (hash, manifest join, sequence-vs-extract with none printed on either side). Kill: the census
  accounts for 17 + 15 boxes; a dropped `[census]` entry reds.
- **T2 — `Form4868Map` + `fill_form_4868(&Form1040Lines, &ReturnHeader, choices)`.** Lines 4–7 computed
  as the table says; kills: L5 > L4 → line 6 prints `0`; default line 7 = line 6; `--pay` negative →
  refuse; no spouse → line 3 blank; the fiscal-year header blank; `c1_2` never set.
- **T3 — Schedule 3 line 10.** Measure today's provenance; `extension_payment` on `ReturnInputs`
  (serde default None); line 10 = the value when Some; line 33 includes it. Kill: Some($N) → line 10
  prints N and line 33 rises by N; None → line 10 blank.
- **T4 — the command.** `tax extension` with R2's refusals and warning (clock seam `BTCTAX_NOW`). Kills:
  uncomputable year → readiness sentence, no bytes; recorded payment → refusal; past due date → the
  warning text.
- **T5 — `Form1040VMap` + the export hook.** Kills: line 37 > 0 + `--pay-by-check` → `f1040v.pdf` beside
  the packet, amount = line 37, manifest line present; no flag → no file, the note; line 37 = 0 → no
  file; `stapled` never contains either stem; `--pay` > line 37 → refuse.
- **T6 — the label reader.** The 4868's nine numbered lines join; the 1040-V's four print inline in a
  single row band and today make `label_join` return *"no numbered label column found"* (label-join
  fold review L8) — either the reader gains the case or the map is recorded unwitnessed with that
  `why`. The build measures; the floor ratchets by whatever joins.

## Out of scope

- Electronic payment (Direct Pay / EFTPS) and the confirmation number; estimated-tax vouchers
  (Form 1040-ES); state extensions; Form 2350; a second (`--out-of-country`) extension beyond the box.
