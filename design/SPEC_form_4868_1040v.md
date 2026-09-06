# SPEC — Form 4868 (extension) and Form 1040-V (payment voucher) fillers (FR-49 / strategy review S8)

**Status: DRAFT r2 (2026-09-06), for review to 0C/0I before build.** r1 review
(`design/agent-reports/2026-09-06-spec-4868-1040v-review.md`, 2C/8I/13M/3N; ledger
`…-VERIFICATION.md` 17/17 TRUE) folded: the extension payment ALREADY exists on the return (C-1),
both emitters carry the pseudo-reconciled attestation gate and the DRAFT watermark (C-2), the spouse
boxes are conditioned on a JOINT return (I-1), the filler takes the printed return (I-2), the three row
gates the new rows red are named and widened (I-3), the label reader learns the row-band form rather
than moving a ratchet (I-4), the due date comes from the year record and shifts for line 8 (I-5), a
recorded payment is the default rather than a refusal (I-6), `--pay-by-check` refuses on a slice year
(I-7), and the TY2024 revisions are bundled so the success path is reachable (I-8). Owning phase: NOW
— the physical rehearsal (S1, an owner decision) and the first filed year (TY2026, due 2027-04-15)
both walk the extension and the payment envelope; today btctax can print neither.

## Why this exists

The paper journey is not "print the return". A filer who cannot finish by the due date files Form 4868
**with a payment**; a filer who owes and pays by check encloses Form 1040-V **loose** in the same
envelope as the return. Both are one-page AcroForms whose every box is either already on the return or
a single filer choice. The TY2025 revisions are archived under authority (FR-49; `design/forms/2025/
f4868--2025.pdf` — 529,415 bytes, a third of the bundled-PDF volume, accepted — and
`f1040v--2025.pdf`, extracts alongside); r2 adds the **TY2024** revisions (I-8). Neither is in `Stem`,
so neither reaches the census, the row kills, the label reader or the packet — the "forms derive the
interview" invariant (field provenance census) has two forms it does not see.

## What the forms ask — transcribed, with provenance (the TEXT layer, `design/forms/extract/`)

`xtask cite-check` does not cover this file (FR-60); every `*"…"*` span below was re-read from the
extract with its typographic apostrophes and quotes kept.

**Form 4868 (2025)**, `f4868--2025.txt:81-92` and the instructions on its own pages 2–4. 17 AcroForm
boxes (`xtask dump-fields`, 2026-09-06); the box→line assignment was verified against the geometry
fixture by the r1 review (32/32):

| line / box | printed text | provenance |
|---|---|---|
| header `VoucherHeader.f1_1/f1_2/f1_3` | *"For calendar year 2025, or other tax year beginning"* … *", 2025, and ending"* … *", 20"* | **blank, recorded**: btctax is calendar-year only |
| 1 `f1_4` | *"Your name(s) (see instructions)"* | `ReturnHeader.name_line` (the 1040's own name line) |
| — `f1_5` | *"Address (see instructions)"* | `ReturnHeader.address_street` |
| — `f1_6` / `f1_7` (maxlen 2) / `f1_8` (maxlen 10) | *"City, town, or post office"* / *"State"* / *"ZIP code"* | `address_city` / `address_state` / `address_zip` |
| 2 `f1_9` (maxlen 11) | *"Your social security number"* | `taxpayer.ssn`, rendered from the cell's own `/MaxLen` (`map.rs:21-23`, `cells::push_identity`: 11 ⇒ hyphenated) |
| 3 `f1_10` (maxlen 11) | *"Spouse’s social security number"* — instructions: *"If you plan to file a joint return, enter on line 2 the social security"* … *"the other SSN to be shown on the joint return"* | `spouse.ssn` **only when `filing_status == Mfj`**; blank for every other status including MFS (`ReturnHeader.spouse` is present for MFS too — I-1) |
| 4 `f1_11` | *"Estimate of total tax liability for 2025"* — instructions: *"Enter on line 4 the total tax liability you expect to report on your"* … *"Form 1040, 1040-SR, or 1040-NR, line 24"*; *"If you expect this amount to be zero, enter -0-."* | **computed**: the year's return as it stands today, Form 1040 line 24; printed as an explicit `0` when zero (the form says so). A year whose return cannot be computed has no estimate → refuse (R2) |
| 5 `f1_12` | *"Total 2025 payments"* — *"Enter on line 5 the total payments you expect to report on your"* … *"Form 1040, 1040-SR, or 1040-NR, line 33 (excluding Schedule 3,"* *"line 10); or"*; *"Don’t include on line 5 the amount you’re paying with this"* *"Form 4868."* | **computed**: Form 1040 line 33 − Schedule 3 line 10 (`Schedule3Lines.line10`, which prints `payments.extension_payment` — C-1); blank when zero (no -0- clause) |
| 6 `f1_13` | *"Balance due. Subtract line 5 from line 4."* — *"If line 5 is more than line 4, enter -0-."* | **computed**: `max(0, L4 − L5)`; printed as an explicit `0` when zero or negative (the form says so) |
| 7 `f1_14` | *"Amount you’re paying (see instructions)"* — *"If you find you can’t pay the amount shown on line 6, you can still"* *"get the extension. But you should pay as much as you can to limit"* *"the amount of interest you’ll owe."* | **collected**: `--pay <whole dollars>`; default = the recorded `payments.extension_payment` when > 0 (with a note), else line 6. Above line 6 allowed (paying ahead of an estimate is the filer's choice); negative refused; cents refused (the form's rounding rule is all-or-nothing and lines 4–6 are whole dollars). Blank when zero |
| 8 `c1_1` | *"Check here if you’re “out of the country” and a U.S. citizen or"* *"resident. See instructions"* | **collected**: `--out-of-country`, default unchecked. Silence FORGOES the two extra months; it asserts nothing (the answered-ness sharp test) |
| 9 `c1_2` | *"Check here if you file Form 1040-NR and didn’t receive wages"* *"as an employee subject to U.S. income tax withholding"* | **never, recorded**: btctax produces no Form 1040-NR |
| page 3 `Col4.f3_1` | *"Enter confirmation number here:"* (the electronic-payment confirmation, on the instruction page) | **never, recorded**: the filer's private record, not part of the filing |

**Form 1040-V (2025)**, `f1040v--2025.txt:66-84`. 15 AcroForm boxes:

| box | printed text | provenance |
|---|---|---|
| 1 `f1_1` (maxlen 11) | *"Your social security number (SSN)"* *"(if a joint return, SSN shown first on your return)"* | `taxpayer.ssn`, from the cell's `/MaxLen` |
| 2 `f1_2` (maxlen 11) | *"If a joint return, SSN shown second"* *"on your return"* | `spouse.ssn` **only when `filing_status == Mfj`** (I-1) |
| 3 `f1_3` | *"Amount you are paying by check or"* *"money order."* | **computed**: Form 1040 line 37 (`Form1040Lines.line37`); `--pay <whole dollars>` overrides for a partial payment — never above line 37 (the voucher pays a computed balance; more than it is a filer error, not a choice the form names — the asymmetry with the 4868 is deliberate) and never negative |
| 4 `f1_5` / `f1_6` | *"Your first name and middle initial"* / *"Last name"* | `taxpayer.first_name` / `last_name` — the same cells and rendering as the 1040 header |
| — `f1_7` / `f1_8` | *"If a joint return, spouse’s first name and middle initial"* / *"Last name"* | `spouse.*` **only when `filing_status == Mfj`** |
| — `f1_9` / `f1_10` / `f1_11` / `f1_12` (maxlen 2) / `f1_13` (maxlen 10) | *"Home address (number and street)"* / *"Apt. no."* / *"City, town, or post office"* … / *"State"* / *"ZIP code"* | the 1040 header's address cells as the **TY2024** 1040 map binds them (`forms/2024/f1040.map.toml:124` `address_apt`; the TY2025 map is capital-gains cells only) |
| — `f1_14` / `f1_15` / `f1_16` | *"Foreign country name"* / *"Foreign province/state/county"* / *"Foreign postal code"* | **blank, recorded**: `ReturnHeader` carries no foreign address — the TY2024 1040 map censuses its own three foreign boxes `unmodeled` (`:202-204`); nothing refuses a foreign address, the boxes are simply left blank, and that blank is correct |

Neither form prints an "Attachment Sequence No." (the extracts show none): neither is attached to the
return. The 1040-V says so itself: *"Do not staple or attach this voucher to your payment or return."*
The 4868 says, on its own page 2: *"Don’t attach a copy of Form 4868 to your return."*

## The rule

**R1 — two new stems, four new rows, zero new hand-lists.** `Stem` gains `F4868` and `F1040v`; each
gets a `forms/2025/<stem>.map.toml` AND a `forms/2024/<stem>.map.toml` ROW (`irs_stem`,
`versioning = "annual"`, `template_sha256`, **no `authority` key** — that key may only read
`not-yet-archived` and its set is pinned to six rows; the manifest join is satisfied by the archived
hash — `instructions = "<irs_stem>"` with `instr_pages` naming the instruction pages (the form IS its
own instructions document and is archived; the IRS publishes no `i4868`/`i1040v`), `line_set =
"f4868/2025"` etc., **no** `attachment_sequence`) plus its bindings as **top-level keys** (like every
committed map; a `[bindings]` table would be a parse refusal under `deny_unknown_fields`) and its
`[census]` (every one of the 17 / 15 boxes mapped or `no` with the reason in the table). `LineSet`
gains the four revisions; `Schema` gains `Form4868Map` and `Form1040VMap`; every exhaustive `match`
on `Stem` reds until each site decides. TY2017 records both as absent (*"fillers begin at TY2024"*);
`forms/2024/YEAR.toml` expected 17 → 19, `forms/2025/YEAR.toml` 15 → 17. **The three row gates the
new rows red, edited by T1 (I-3):** `map_rows.rs:298-302` (sequence-less ⇔ `f1040`) becomes
*"`attachment_sequence` is absent exactly on the rows whose extract prints none"*; `map_rows.rs:337`
(`instr_pages` pinned to one row) becomes a set with the new entries; `cite_check.rs:1378` resolves
`instructions = "<irs_stem>"` to the archived form itself.

**R2 — Form 4868 is a command, not a packet member.** `btctax extension --year <Y> --out <DIR>
[--pay <usd>] [--out-of-country]` (a NEW top-level command beside `export-irs-pdf`; there is no `tax`
group today and none is invented) writes `f4868.pdf` (owner-only, like `export-irs-pdf`) and prints
what it filled, line by line, followed by the form's own sentence *"Don’t attach a copy of Form 4868
to your return."* Refusals, in order:
- the year has no stored return inputs, or its return is not computable → the same message
  `export_full_return` already formats (`admin.rs:993-1012`: *"the {year} return is not computable
  [{reason}]: {detail} — no forms were written"*); the `YearReadiness` sentence only for the
  no-params case (no estimate exists; the instructions demand one *"as accurate as you can"*);
- `--out` already holds a `manifest.txt` (the return's envelope directory) → refuse: the 4868 is
  mailed separately, and an unlabelled copy in the envelope is what the form forbids;
- `--pay` negative or with cents → refuse; above line 6 → allowed.
- **the pseudo-reconciled gate (C-2)**: when `state.pseudo_active()`, `require_attestation` first
  (no bytes on refusal) and `stamp_draft_watermark` on every page — the same guarantee `cli.rs:199-200`
  states for `export-irs-pdf`, and a form money is attached to may not be the exception.
  `promote_export_gate` is **not** required: the 4868 discloses no position (decision, not gap).
Warning: the clock (`BTCTAX_NOW`) is past the year's due date — `YearRecord::for_year(y).return_due`
(TY2017's is 2018-04-17, so no month/day is hardcoded), **shifted two months when `--out-of-country`
is passed** (the form's own page 2: *"If you’re out of the country and file a calendar year income tax
return, you can pay the tax and"* *"file your return or this form by June 15, 2026."*) — printed, not
refused (a useless form is not worse than silence). A recorded `payments.extension_payment` > 0 is
**the default for `--pay`** with the note *"$N is already recorded on the return as paid with the
extension; --pay overrides it"* — never a refusal (I-6: the field records a payment, not a filing, and
a filer who records first and prints second is doing it in the natural order).

**R3 — the return already carries the payment.** `Payments.extension_payment: Usd`
(`return_inputs.rs:708`) is collected by the TUI input form and by `income import`, enters the exact
`total_payments` (`return_1040.rs:2338`), and prints on Schedule 3 line 10 (`printed.rs:1543`), with
tests holding it. **This spec changes nothing there.** The one open question — whether it becomes
`Option<Usd>` so a $0 line 10 on a filed Schedule 3 is a blank rather than a sworn zero — is filed as
`FOLLOWUPS.md` FR-59 with its blast radius, and is not part of this build.

**R4 — Form 1040-V rides with `export-irs-pdf`, loose.** When the return's line 37 > 0 AND the filer
passes `--pay-by-check`, the export writes `f1040v.pdf` **beside** the packet and adds a separate
footer block to the manifest — *"ENCLOSE LOOSE — do not staple: f1040v.pdf with the check"* — under
the stapling list, never inside it. No code path passes it to `FiledPacket::stapled` (held by test — it
is not structural, `stapled` takes any `Vec<NamedForm>`). Without `--pay-by-check` nothing is written
and the export notes *"amount owed $N — Direct Pay / EFTPS need no voucher; pass --pay-by-check for
Form 1040-V"*. With the flag and line 37 = 0 → a note, no voucher. **On a year with no full-return
inputs (the crypto slice) `--pay-by-check` refuses** (I-7): *"there is no Form 1040 line 37 for {year}
— Form 1040-V accompanies a full return; see `income import`"* — silence is the one answer a tax tool
may not give (`admin.rs:610`). The pseudo gate and watermark apply exactly as to the packet (C-2).

**R5 — TY2024 and TY2026.** The TY2024 revisions are archived and bundled in T1 because TY2024 is the
only year whose return computes today (`tax_tables.rs:102` inserts 2024 alone; TY2025 is paused), so
without them every end-to-end kill could only exercise a refusal (I-8) and the rehearsal could not
print. Both are annual forms; the TY2026 revisions arrive with the January 2027 package and follow the
port runbook as two more rows. Nothing here is year-specific except the archived templates.

## Journey walk — the divergences, classified

| the filer… | outcome | class |
|---|---|---|
| runs `extension` on a year with no stored return inputs | refusal, the export's own "not computable" message | refusal |
| runs it after April 15 (or June 15 with line 8) | warning, form still written | warning |
| omits `--pay` | line 7 = the recorded payment if any, else line 6 | default |
| pays less than line 6 | allowed; the form's own instruction covers it | not our concern |
| records the payment first, then prints | the recorded amount is the default; no refusal | default |
| runs it twice | a second identical PDF (a reprint after mailing is legitimate) | not our concern |
| files, pays, then exports without recording `extension_payment` | Schedule 3 line 10 blank, line 37 too high by the payment; the closing note is the only guard | documentation only (over-payment self-corrects; never understates) |
| passes `--out` pointing at the packet directory | refusal (manifest.txt present) | refusal |
| is pseudo-reconciled and unattested | refusal, no bytes | refusal |
| is pseudo-reconciled and attested | watermarked pages | warning |
| is MFS | line 3 / box 2 / spouse names blank | default (the form asks only for a joint return) |
| passes `--pay-by-check` on a refund return | note, no voucher | default |
| passes `--pay-by-check` on a crypto-slice year | refusal naming the reason | refusal |
| staples the voucher to the return | the manifest footer says not to | documentation only |
| has a foreign address | the 1040 leaves its foreign boxes blank; so does the voucher | not our concern |
| has an IP PIN | neither form asks for one; nothing printed | not our concern |

## Plan (TDD — every task lands with its kill)

- **T1 — the rows.** Archive `f4868--2024` / `f1040v--2024` (irs-prior, notes + extracts + geometry +
  manifest); `Stem::{F4868, F1040v}`; four `.map.toml` rows (top-level bindings, `[census]`);
  `LineSet`/`Schema` arms; year records (2024 → 19, 2025 → 17; 2017 absent with reason); the three
  row-gate edits of R1 with their widened predicates. Kills: the census accounts for 17 + 15 boxes per
  year; a dropped `[census]` entry reds; a row with an `attachment_sequence` whose extract prints none
  reds; `instr_pages` holds every row that declares pages.
- **T2 — `Form4868Map` + `fill_form_4868(&PrintedReturn, &ReturnHeader, choices)`.** Lines 4–7 as the
  table says (line 5 = line 33 − `Schedule3Lines.line10` — the printed return carries both; I-2).
  Kills: L5 > L4 → line 6 prints `0`; line 4 zero prints `0`; line 5 zero prints blank; default line 7
  = line 6, or the recorded payment when > 0; `--pay` negative or with cents → refuse; an excess-SS
  credit with no extension payment ⇒ line 5 = line 33, and with both ⇒ line 5 excludes only the
  extension payment; MFS ⇒ line 3 blank; MFJ ⇒ line 3 filled; the fiscal-year header blank; `c1_2`
  never set.
- **T3 — the command.** `btctax extension` with R2's refusals, the pseudo gate + watermark, and the
  warning from `return_due` (+2 months with line 8; clock seam `BTCTAX_NOW`). Kills: uncomputable year
  → the export's message, no bytes; `--out` with a manifest.txt → refusal; pseudo + no attest → refusal,
  zero bytes; pseudo + attest → watermarked; a TY2017-shaped record never prints `04-15`; an
  out-of-country run on May 1 prints no warning; a TY2024 end-to-end KAT prints a real 4868 (with the
  due-date warning, since 2025-04-15 has passed).
- **T4 — `Form1040VMap` + the export hook.** Kills: line 37 > 0 + `--pay-by-check` → `f1040v.pdf` beside
  the packet, amount = line 37, the footer block present and separate from the stapling list; no flag
  → no file, the note; line 37 = 0 → no file; MFS ⇒ box 2 and `f1_7`/`f1_8` blank; `--pay` > line 37 or
  negative → refuse; slice year → refusal naming the reason; pseudo gate + watermark; no test or
  code path passes either stem to `stapled`; a TY2024 end-to-end KAT.
- **T5 — the label reader (I-4, M-9).** Measured today: `f4868--2025` → 7 labels, 17 boxes; 6 entry
  lines (4–9), lines 1–3 do not join and a spurious `9a` heading needs a recorded reason;
  `f1040v--2025` → *"no numbered label column found"* (its four labels print inline in one row band).
  The reader gains the **row-band** form — when no label column exists but ≥ 3 numeric labels sit in
  one row band, the x-aware in-row rule alone joins them — with the kill on `f1040v--2025` (4 joins).
  No allowance moves (the 2025 `max_unwitnessed: 0` ratchet stays); the floors rise by whatever joins
  (the 4868's 6, the 1040-V's 4, and their TY2024 twins), measured then written.
- **T6 — the surfaces.** `report` names the extension and voucher paths; `YearReadiness::sentence`
  unchanged; the help text names both `--pay` flags with their different ceilings and defaults.

## Out of scope

- Electronic payment (Direct Pay / EFTPS) and the confirmation number; estimated-tax vouchers
  (Form 1040-ES); state extensions; Form 2350; a second extension beyond the line-8 box.
- `payments.extension_payment` becoming `Option<Usd>` (FR-59); `cite-check` over specs (FR-60).
