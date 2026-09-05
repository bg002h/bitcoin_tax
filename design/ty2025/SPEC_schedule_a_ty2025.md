# SPEC — Schedule A (Form 1040), TY2025

**Status:** RECON + SPEC. Nothing built. Written 2026-09-04 on `feat/schedule-1a-ty2025`.

**Companion documents.** `design/ty2025/SPEC.md` §5.1 already specifies the *computation* (the §164(b)
worksheet) and D-6 the shared MAGI surface. **This document does not re-specify either.** It covers the
part `SPEC.md` does not: the TY2025 **form** — its text delta, its AcroForm, its `.map.toml`, and the
conformance gates. Where the two overlap, `SPEC.md` governs the arithmetic and this one the printing.

**One-line summary.** Schedule A's *text* changed in exactly six places and its line set did not change
at all; its *PDF* was rebuilt from scratch, and that rebuild — not the tax law — is where the danger is.

---

## 0. What already exists, and what is actually missing

Checked in this session, not assumed.

| thing | state | evidence |
|---|---|---|
| TY2025 PDF archived | **yes** | `design/forms/2025/f1040sa--2025.pdf` (gitignored by `.gitignore:63`), `sha256:c14acf3478f4c33f…` |
| TY2025 text layer | **yes** | `design/forms/extract/f1040sa--2025.txt`, 75 lines |
| TY2025 instructions text layer | **yes** | `design/forms/extract/i1040sca--2025.txt`, 8837 lines |
| `MANIFEST.json` entry | **yes** | `design/forms/MANIFEST.json:327` (path), `:332` (url), `:333` (extract) |
| geometry fixture | **yes** | `design/forms/geometry/f1040sa--2025.json`, committed at `1548462a`; regenerating it in this session reproduced it **byte-identically** |
| the SALT **computation** | **BUILT** | `crates/btctax-core/src/tax/tables.rs:335` `SaltLimitation::Worksheet2025`, `:372` `line_5e()` — all ten worksheet lines, the $10,000 short-circuit, the line-10-only MFS halving |
| the MAGI surface | **BUILT** | `crates/btctax-core/src/tax/return_inputs.rs:970` `has_income_exclusion`, `:1063` `modified_agi()`; `QuestionId::HasIncomeExclusion` (`questions.rs:96`), live from TY2025 |
| the TY2025 **form map** | **ABSENT** | `crates/btctax-forms/forms/2025/` holds `f1040`, `f8283`, `f8949`, `schedule_d`, `schedule_se` only |
| `schedule_a_pdf(2025)` | **fails closed** | `crates/btctax-forms/src/pdf.rs:184-187` — `2024 => Ok(..)`, `_ => Err(UnsupportedYear)` |
| TY2025 full return | **fails closed** | `crates/btctax-adapters/src/tax_tables.rs:814` `ty2025_full_return_must_stay_fail_closed_until_complete` |

**So the work left on Schedule A is almost entirely form-side.** The recon (`recon-year-port-delta.md`)
classified Schedule A **STRUCTURAL** because *"SALT cap raised … with a brand-new MAGI phase-out … this
is new tax logic, not a threshold bump."* That was correct when written and the logic has since landed.
What remains is the map, and the map is where the trap is (§4).

---

## 1. The diff, line by line

### 1.1 The line set is IDENTICAL

Both years carry **28 printed line labels**, and the sets are equal:

```
$ cargo run -q -p xtask -- label-census f1040sa--2024 | head -1
# f1040sa--2024 — 28 labels, 37 boxes
$ cargo run -q -p xtask -- label-census f1040sa--2025 | head -1
# f1040sa--2025 — 28 labels, 33 boxes
$ diff <(label-census 2024 | labels) <(label-census 2025 | labels)   # → empty
IDENTICAL LABEL SETS
```

Labels, both years: `1, 2, 3, 4, 5, 5a, 5b, 5c, 5d, 5e, 6, 7, 8, 8a, 8b, 8c, 8d, 8e, 9, 10, 11, 12, 13,
14, 15, 16, 17, 18`. Label `5` is a heading with no box in its row span — recorded as such by
`label-census` in **both** years, so it is not a new gap.

> **Nothing was added, removed, or renumbered.** This is the opposite of Form 6251, whose line 1 split
> into 1a/1b. Any port plan that budgets for renumbering on Schedule A is budgeting for the wrong form.

### 1.2 The substantive text delta is exactly SIX items

A word-level diff of the two extracts' bodies (`difflib.SequenceMatcher`, dot leaders stripped) yields
569 → 604 words at similarity **0.9463**. Of its 23 opcodes, **8 pairs are a caption word crossing a
reflow boundary** (`Expenses`, `home`, `1098`, `Charity`, `more,`, `and`, `Losses`, `Itemized` each
delete-then-insert with no content change: the 2025 form's body column is narrower and every paragraph
re-wraps). The remainder is:

| # | line | TY2024 | TY2025 | class |
|---|---|---|---|---|
| 1 | header | `2024` | `2025` | year stamp |
| 2 | **2** | `…Form 1040 or 1040-SR, line 11` | `…Form 1040 or 1040-SR, line 11b` | **cross-reference** — Form 1040 split line 11 into 11a/11b |
| 3 | **5** | `State and local taxes.` | `State and local taxes (SALT).` | heading, cosmetic |
| 4 | **5e** | `$10,000 ($5,000` | `$40,000 ($20,000` | **the cap** |
| 5 | **5e** | *(nothing)* | `If Form 1040 or 1040-SR, line 11b is more than $500,000 ($250,000 if married filing separately), or if you completed Form 2555, Form 4563, or excluded income from Puerto Rico, see instructions` | **the new phase-out** |
| 6 | **17** | `…enter this amount on Form 1040 or 1040-SR, line 12` | `…enter this amount on Form 1040 or 1040-SR, line 12e` | **cross-reference** — Form 1040 exploded line 12 into 12a–12e |
| — | footer | `Schedule A (Form 1040) 2024` | `Schedule A (Form 1040) 2025 Created 11/20/25` | year stamp |

Items 2 and 6 are **ripple** from Form 1040's own restructuring, not from anything Schedule A does:
`design/forms/extract/f1040--2025.txt:86` (`11a`, AGI), `:91` (`11b`, "Amount from line 11a"), `:97`
(`12e`, "Standard deduction or itemized deductions (from Schedule A)").

### 1.3 Verbatim transcription of the three changed amount lines

From the text layer (`design/forms/extract/f1040sa--2025.txt`), wraps joined, nothing else altered:

```
L2   "Enter amount from Form 1040 or 1040-SR, line 11b"

L5e  "Enter the smaller of line 5d or $40,000 ($20,000 if married filing separately). If Form 1040
      or 1040-SR, line 11b is more than $500,000 ($250,000 if married filing separately), or if you
      completed Form 2555, Form 4563, or excluded income from Puerto Rico, see instructions"

L17  "Add the amounts in the far right column for lines 4 through 16. Also, enter this amount on
      Form 1040 or 1040-SR, line 12e"
```

Every other line's instruction text is byte-identical to TY2024 once wrapping is normalised, so the
TY2024 map's doc comments carry over verbatim for lines 1, 3, 4, 5a–5d, 6, 7, 8, 8a–8e, 9–16, 18.

---

## 2. The SALT cap change

### 2.1 Authority

`legal/primary-sources/statute-irc/` holds 16 sections and **§164 is not among them**
(`26USC_s1, s61, s170, s1001, s1011, s1012, s1015, s1016, s1031, s1091, s1211, s1212, s1221, s1222,
s1223, s1411`). So the statutory text is **not citable from this repo**, and the authority used below is,
in order: the form (`f1040sa--2025.txt`), the instructions (`i1040sca--2025.txt`), and the pin already
in our own source — `crates/btctax-core/src/tax/tables.rs:362` names **§164(b)(7)(B)(iv)** as the
modified-AGI definition. Fetching `26USC_s164.html` into `legal/primary-sources/statute-irc/` is a
**follow-up owned by the phase that lands this map** (see §6.3): a phase-out this large should not be
carried on a secondary source indefinitely.

`i1040sca--2025.txt:20-27`, What's New, verbatim:

> **State and local tax (SALT) deduction limit increased.** The overall limit on the deduction for state
> and local income, sales, and property taxes has increased to $40,000 ($20,000 if married filing
> separately). The overall limit is reduced if your modified adjusted gross income is more than $500,000
> ($250,000 if married filing separately) but will not be reduced below $10,000 ($5,000 if married filing
> separately). See the instructions for line 5e.

### 2.2 The mechanism — the State and Local Tax Deduction Worksheet, all ten lines

The `pdftotext` (no-flags) extract reflows this worksheet's markers away from their rows. **Transcribe
from a `-layout` extraction**, which resolves it unambiguously:

```
$ pdftotext -layout design/forms/2025/i1040sca--2025.pdf - | sed -n '502,536p'
```

| ws line | verbatim text |
|---|---|
| *Before you begin* | "If the amount on Schedule A, line 5d is $10,000 ($5,000 if married filing separately) or less, enter the amount from Schedule A, line 5d on Schedule A, line 5e. You don't have to complete this worksheet." |
| **1** | "Is the amount on Schedule A, line 5d more than $10,000 ($5,000 if married filing separately)?  **No.** STOP. Your deduction isn't limited. Enter the amount from Schedule A, line 5d on Schedule A, line 5e. Don't complete the rest of this worksheet.  **Yes.** Enter $40,000" |
| **2** | "Enter the amount from Form 1040 or 1040-SR, line 11b" |
| **3a** | "Enter any income from Puerto Rico that you excluded" |
| **3b** | "Enter the amount from Form 2555, line 45" |
| **3c** | "Enter the amount from Form 2555, line 50" |
| **3d** | "Enter the amount from Form 4563, line 15" |
| **3e** | "Add lines 3a through 3d" |
| **4** | "Add lines 2 and 3e" |
| **5** | "Enter $500,000 ($250,000 if married filing separately)" |
| **6** | "Is the amount on line 4 more than the amount on line 5?  **No.** Skip lines 7 and 8 and enter the amount from line 1 on line 9.  **Yes.** Subtract line 5 from line 4" |
| **7** | "Multiply line 6 by 30% (0.30)" |
| **8** | "Subtract line 7 from line 1" |
| **9** | "Enter the larger of the amount on line 8 or $10,000" |
| **10** | "State and local tax deduction. Enter the smaller of the amount on line 9 (half the amount on line 9 if married filing separately) or the amount from Schedule A, line 5d here and on Schedule A, line 5e" |

**★ The MFS shape.** Only **line 5** carries a per-status constant ($250,000). Lines 1 ($40,000) and 9
($10,000) are **not** halved; the halving happens **once, at line 10**. The published $20,000/$5,000
MFS figures are the *consequence* of that single halving, not independent parameters. This is already
encoded correctly and defended by a KAT: `tables.rs:635` `salt_2025()`, and the test above it records
that the "halve every constant" parameterisation — which is what Tax-Calculator does
(`ID_AllTaxes_c[mseparate] = 20000`) — gives **$12,500 vs $5,000** for an MFS filer with $30,000 of SALT
at $300,000 MAGI. `design/ty2025/SPEC.md:110` records that this project encoded the wrong shape twice
before a review caught it.

### 2.3 Where it lands on the form

**Line 5e, and nowhere else.** The worksheet is an *instructions* worksheet: it has no AcroForm fields,
is never filed, and its only output is Schedule A line 5e. Consequently:

- the TY2025 `.map.toml` gains **no new field** for the phase-out;
- line 7 is still `5e + 6`, and with line 6 unmodeled, `line7 == line5e` in both years;
- everything downstream of line 5e is arithmetically unchanged in *shape* — only larger in *value*.

Already implemented at `crates/btctax-core/src/tax/tables.rs:372` `line_5e(line_5d, magi, status)`, with
`magi: None` (never asked) collapsing to `line9_floor` — the **smallest** deduction, which can only
overstate tax. Called from `return_1040.rs:484`.

> **Non-defect, recorded so nobody "fixes" it.** `printed.rs:1395` reads `let line5e =
> line5d.min(p.salt_cap);` — a scalar — and looks like a second, stale chain. It is not.
> `return_1040.rs:352` documents that `ScheduleAParts::salt_cap` is set to the **already-applied** limit
> (`salt_cap = salt_5e`), so `min(5d, salt_cap)` is idempotent given `5e ≤ 5d`. The printed chain is
> year-agnostic and needs no edit. **The `ScheduleALines::line5e` doc comment at `printed.rs:1325` does**
> — it still reads *"min(printed 5d, $10,000 / $5,000 MFS)"*, which is false for TY2025.

---

## 3. The Schedule 1-A interaction

### 3.1 Schedule A neither feeds nor is fed by Schedule 1-A

They are **siblings**, and they meet only on Form 1040:

```
Schedule A line 17  ──────────────────────►  Form 1040 line 12e
Schedule 1-A line 38 ─────────────────────►  Form 1040 line 13b
                                             Form 1040 line 14 = 12e + 13a + 13b
```

Evidence: `f1040sa--2025.txt` L17 ("…enter this amount on Form 1040 or 1040-SR, line 12e");
`f1040s1a--2025.txt:110` L38 ("Add lines 13, 21, 30, and 37. Enter here and on Form 1040 or 1040-SR,
line 13b…"); `f1040--2025.txt:97` (12e), `:100` (13b), `:101` (14 = "Add lines 12e, 13a, and 13b").

There is **no line on either schedule that reads the other**. The instructions say so directly
(`i1040sca--2025.txt:28-36`): *"If you are eligible, you can claim these deductions even if you itemize
on Schedule A. If you are eligible, you will claim these deductions on Schedule 1-A, not on Schedule A."*

### 3.2 …but they share ONE input surface, and that is the real coupling

The SALT worksheet's add-backs (lines 3a–3d: excluded Puerto Rico income, Form 2555 line 45, Form 2555
line 50, Form 4563 line 15) are the **identical four** Schedule 1-A Part I uses to build its own MAGI.
`tables.rs:362-363` states it: *"`magi` is the §164(b)(7)(B)(iv) modified AGI …, the **same** quantity
Schedule 1-A Part I line 3 computes."* `design/ty2025/SPEC.md` D-6 already scopes this as one collected
surface driving five phase-outs.

**Consequence for this map:** the same `QuestionId::HasIncomeExclusion` refusal that gates Schedule 1-A
gates Schedule A from TY2025 onward — *"a filer who itemizes but claims no Schedule 1-A deduction still
has to be asked."* That is already true in code; this spec only records that Schedule A is a second
consumer, so a future narrowing of that question to Schedule 1-A would silently break SALT.

### 3.3 Form 6251 line 2a still points at Schedule A line 7 — unchanged

```
2024 (f6251--2024.txt:21-22)  2a  If filing Schedule A (Form 1040), enter the taxes from Schedule A,
                                  line 7; otherwise, enter the amount from Form 1040 or 1040-SR, line 12
2025 (f6251--2025.txt:22-23)  2a  If filing Schedule A (Form 1040), enter the taxes from Schedule A,
                                  line 7; otherwise, enter the amount from Form 1040 or 1040-SR, line 12e
```

**The Schedule A pointer is byte-identical.** The only change is the *otherwise* branch, which follows
Form 1040's own 12 → 12e renumbering. So no Schedule A field moves on account of AMT.

**★ But the AMT exposure changes materially, with no code change to notice it.** Line 2a is a
**positive add-back**: SALT deducted for regular tax is disallowed for AMT. Raising the cap from $10,000
to $40,000 raises that add-back by up to **$30,000 per return** ($15,000 MFS). A household that owed no
AMT in TY2024 because its SALT add-back was capped at $10,000 can owe AMT in TY2025 on the identical
economics. This is not a Schedule A defect — the form is right — but it is a **testing obligation**: see
§5.5 KAT-6.

For completeness, the other Part I wiring (out of scope here, owned by the Form 6251 port): line 1a =
Form 1040 L14 − Schedule 1-A L37, line 1b = Form 1040 L11b − line 1a. Since Schedule 1-A L38 =
`13+21+30+37` and L37 is the enhanced senior deduction, `1a` equals every additional deduction *except*
the senior one — i.e. the §63(f) senior deduction is **added back** for AMT and the tips/overtime/car-loan
deductions are not.

---

## 4. ★★ THE TRAP — verified

### 4.1 The root subform rename is real

```
$ cargo run -p xtask -- dump-fields design/forms/2024/f1040sa--2024.pdf | head -2
# design/forms/2024/f1040sa--2024.pdf — 37 AcroForm fields
p1     36.0, 684.0- 474.5, 698.0  text  topmostSubform[0].Page1[0].f1_1[0]

$ cargo run -p xtask -- dump-fields design/forms/2025/f1040sa--2025.pdf | head -2
# design/forms/2025/f1040sa--2025.pdf — 33 AcroForm fields
p1     36.0, 684.0- 474.5, 698.0  text  form1[0].Page1[0].f1_1[0]
```

**Confirmed: `topmostSubform[0]` → `form1[0]`, and 37 → 33 fields.** Every one of the 108 lines of
`crates/btctax-forms/forms/2024/f1040sa.map.toml` names the old root.

### 4.2 The rename is the *small* half. The suffixes moved too.

Four fields were removed by merging multi-row write-in areas into single taller boxes, and everything
below each merge shifted. Correlation is machine-derived (`cargo run -p xtask -- label-boxes
f1040sa--2024` / `--2025`), not eyeballed:

| printed line | TY2024 field | TY2025 field | Δ |
|---|---|---|---|
| header name / SSN | `f1_1` / `f1_2` | `f1_1` / `f1_2` | — |
| 1, 3, 4, 5a, 5b, 5c, 5d, 5e | `f1_3, f1_5, f1_6, f1_7, f1_8, f1_9, f1_10, f1_11` | *same* | — |
| **2** | `f1_4` | **`Line2_ReadOrder[0].f1_4[0]`** | new parent subform |
| 6 (write-in) | `f1_12` + `f1_13` (two rows) | `f1_12` (one merged box) | −1 field |
| 6 (amount) | `f1_14` | **`f1_13`** | −1 |
| **7** | `f1_15` | **`f1_14`** | −1 |
| **8a** | `f1_16` | **`f1_15`** | −1 |
| 8b (amount) | `f1_19` | **`f1_17`** | −2 |
| 8b (write-in) | `f1_17` + `f1_18` | **`Line8b_ReadOrder[0].f1_16[0]`** (one merged box) | −1 field, new parent |
| 8c, 8d, 8e, 9, 10, 11, 12, 13, 14, 15 | `f1_20 … f1_29` | **`f1_18 … f1_27`** | −2 each |
| 16 (write-in) | `f1_30` + `f1_31` + `f1_32` | **`f1_28`** (one merged box) | −2 fields |
| 16 (amount) | `f1_33` | **`f1_29`** | −4 |
| **17** | `f1_34` | **`f1_30`** | −4 |
| checkboxes 5a / 8 / 18 | `c1_1`, `Line8_ReadOrder[0].c1_2`, `Line18_ReadOrder[0].c1_3` | *same names* | — |

37 − 33 = 4, accounted for exactly: 1 (line 6) + 1 (line 8b) + 2 (line 16).

**★ The non-sequential-suffix trap survived, inverted.** The TY2024 map's own comment warns that *"line
8b is `f1_19`, NOT `f1_17`"* because the IRS numbered the write-in rows before the amount. TY2025 keeps
the inversion but **swaps which suffix is which**: `f1_16` is now the write-in and `f1_17` the amount. A
porter who remembers the 2024 warning as "8b's amount is the *higher* suffix" gets it backwards.

### 4.3 What a naive port actually produces

Simulated mechanically: take the 19 money-line field names from `f1040sa.map.toml`, rewrite only
`topmostSubform[0]` → `form1[0]`, and resolve each against the TY2025 AcroForm and its
geometrically-derived line labels.

```
ABSENT (→ FormsError::MapFieldMissing, fails closed): line2, line8a, line17
CORRECT:  8   line1, line3, line4, line5a, line5b, line5c, line5d, line5e
WRONG LINE, column check CATCHES:      6   7→8a, 8e→10, 10→12, 12→14, 13→15, 14→16
WRONG LINE, EVERY geometry leg PASSES: 2   9→11, 11→13
```

**The direct answer: it errors.** `verify_flat` (`crates/btctax-forms/src/verify.rs:368-370`) raises
`FormsError::MapFieldMissing("form1[0].Page1[0].f1_4[0]")` on line 2, and the fill fails closed with no
PDF bytes returned. A naive port cannot ship a wrong form on its first run.

**But it errors for the wrong reason, and that matters more than the error.** It is caught by three
fields having moved *into subforms* — an accident of this particular rebuild — not by the safety net the
crate advertises. `schedule_a.rs:33-34` promises *"read back through the geometric verifier (a mis-mapped
cell FAILS CLOSED)"*, and on this delta that promise is **partly false**:

- **The column-cluster leg is degraded, not sound.** `SCHEDULE_A_CLUSTERS` (`schedule_a.rs:31`) is
  `[(331.0, 403.0), (417.0, 489.0), (504.0, 576.0)]`, hand-measured from the **TY2024** PDF and
  hardcoded in Rust. The TY2025 boxes moved left — MID from x-centre 453.2 to **446.0**, AGI-inline from
  366.9 to **352.4** — and both still fall inside the 2024 bands. So the check keeps *working* by luck,
  and it catches 6 of the 8 wrong-line writes only because those 6 crossed a column.
- **The ordinal-y descent leg catches nothing.** `schedule_a.rs:56-64` places all 19 money writes in a
  single descent group (`Some((0, ord as u32))`), and `verify_flat` (`verify.rs:409-420`) asserts only
  that ordinal *k* sits strictly above ordinal *k+1*. A **uniform downward shift preserves monotonicity**.
  Measured: 0 violations across all 16 resolvable writes.

So two writes — **line 9 (investment interest) printing in the line 11 box (gifts by cash or check), and
line 11 printing in the line 13 box (carryover from prior year)** — are wrong-line, same-column,
order-preserving, and pass every leg of the oracle. If a porter repairs the three `MapFieldMissing`
errors and then the six column errors one at a time as the verifier reports them (which is the natural
workflow, since `verify_flat` returns on the first failure), they converge on a map that is **green and
wrong**.

### 4.4 How dangerous, stated plainly

| | |
|---|---|
| **First-run outcome** | hard error, no PDF. Not a silent wrong form. |
| **Residual risk** | a verifier-guided repair loop terminates green with 2 lines mis-printed. |
| **Where it is wrong** | Interest and Gifts — the exact block where 2024 and 2025 look identical, so nobody re-reads it. |
| **Where it is right** | Medical (1, 3, 4) and **all of SALT (5a–5e)** map correctly under the naive port. The headline change is the part a spot-check would verify, and it passes. |
| **Detectability by eye** | low. A filer sees plausible dollar amounts on plausible lines; the arithmetic no longer cross-foots, but line 10 and line 14 would be filled from the wrong sources and could still look ordinary. |

**Ruling: not "blank form", not "an error you can trust" — the honest answer is that it fails closed on
the first try and the safety net that is supposed to catch the rest has a measured blind spot.** The port
must therefore *derive* the map from `label-boxes`, never edit the 2024 file, and it must widen the
oracle (§5.5 M-3) rather than rely on it as it stands.

---

## 5. The specification

### 5.1 Scope

1. `crates/btctax-forms/forms/2025/f1040sa.pdf` — the archived TY2025 PDF, vendored (the crate embeds
   each form; see `pdf.rs:53`).
2. `crates/btctax-forms/forms/2025/f1040sa.map.toml` — a **newly derived** map, §5.3.
3. `crates/btctax-forms/src/pdf.rs` — `SCHEDULE_A_PDF_2025` const and a `2025 => Ok(..)` arm at `:185`.
4. `crates/btctax-forms/src/map.rs` — `SCHEDULE_A_MAP_2025` const (pattern at `:73`).
5. `crates/btctax-forms/src/schedule_a.rs` — **`SCHEDULE_A_CLUSTERS` becomes per-year.** TY2025 bands,
   from the dump: AGI-inline `(316.8, 388.0)`, MID `(410.4, 481.6)`, AMOUNT `(504.0, 576.0)`.
6. `crates/btctax-core/src/tax/printed.rs:1325` — correct the `line5e` doc comment, which asserts the
   TY2024 flat cap as though it were the rule.
7. The gates in §5.5.

### 5.2 Non-scope, with explicit refusals

Carried over from TY2024 unchanged; each is a line btctax leaves **blank**, never `0`:

| line | refusal / reason |
|---|---|
| 6 — other taxes | `unmodeled`. Only the enumerated SALT categories 5a–5c are collected. Line 7 therefore equals line 5e. |
| 8b, 8c — mortgage interest / points not on a Form 1098 | `unmodeled`. `mortgage_interest_1098` is the only collected figure. |
| 8d — "Reserved for future use" | `artifact`. The form's own text says it encodes no decision. **Never written** (it is a live, grey, ReadOnly widget). |
| 15 — casualty and theft losses | `unmodeled`. No Form 4684. |
| 16 — other itemized deductions | `unmodeled`. No write-in surface. |
| the §164(b) worksheet itself | **not a filed form.** No fields, no map entry, no emitted page. Its sole output is line 5e. |
| Form 2555 / Form 4563 filers | **NOT refused, and this is deliberate.** Their exclusions are collected as MAGI add-backs (worksheet 3a–3d) via `has_income_exclusion`; a filer who answers *yes* still gets a correct line 5e. What refuses is `has_income_exclusion == None` — never asked. |

**New refusal required for TY2025 (none needed for TY2024):** a filer who itemizes, has more than
$10,000 ($5,000 MFS) on line 5d, and whose `has_income_exclusion` was never asked, must refuse rather
than print. `screen_inputs` already does this via `QuestionId::HasIncomeExclusion` (live from TY2025);
`line_5e`'s `magi: None → floor` arm is the belt to that brace. **This spec adds no new refusal
variant** — it records that Schedule A is now a second consumer of an existing one, so narrowing that
question to Schedule 1-A would break SALT silently.

### 5.3 The map — derived, not ported

Produced by `label-boxes f1040sa--2025` plus the column bands, verified to partition all 33 fields:

```
33 fields = 19 mapped money + 3 checkboxes + 2 identity + 9 census
```

**Mapped (`form1[0].Page1[0].` prefix elided):**

| line | field | column |  | line | field | column |
|---|---|---|---|---|---|---|
| 1 | `f1_3[0]` | MID | | 8e | `f1_20[0]` | MID |
| 2 | `Line2_ReadOrder[0].f1_4[0]` | **AGI-INLINE** | | 9 | `f1_21[0]` | MID |
| 3 | `f1_5[0]` | MID | | 10 | `f1_22[0]` | AMOUNT |
| 4 | `f1_6[0]` | AMOUNT | | 11 | `f1_23[0]` | MID |
| 5a | `f1_7[0]` | MID | | 12 | `f1_24[0]` | MID |
| 5b | `f1_8[0]` | MID | | 13 | `f1_25[0]` | MID |
| 5c | `f1_9[0]` | MID | | 14 | `f1_26[0]` | AMOUNT |
| 5d | `f1_10[0]` | MID | | 17 | `f1_30[0]` | AMOUNT |
| 5e | `f1_11[0]` | MID | | | | |
| 7 | `f1_14[0]` | AMOUNT | | | | |

**Checkboxes:** `c1_1[0]` (5a sales-tax election, §164(b)(5)) · `Line8_ReadOrder[0].c1_2[0]` (line 8
mixed-use mortgage declaration, §163(h)(3)(F)) · `Line18_ReadOrder[0].c1_3[0]` (line 18, §63(e)).
**Identity:** `f1_1[0]` (name), `f1_2[0]` (SSN, `/MaxLen 11` ⇒ hyphenated — unchanged from 2024).

**Census (9), each with the reason carried over from the TY2024 map:**
`f1_12[0]` (6 write-in) · `f1_13[0]` (6 amount) · `Line8b_ReadOrder[0].f1_16[0]` (8b write-in) ·
`f1_17[0]` (8b amount) · `f1_18[0]` (8c) · `f1_19[0]` (**8d — `rule = "artifact"`**) · `f1_27[0]` (15) ·
`f1_28[0]` (16 write-in) · `f1_29[0]` (16 amount).

> The TY2024 map's census has **13** entries for the same nine *lines*, because 2024 spread the three
> write-in areas over six boxes and 2025 uses three. The count changing is expected; a census that still
> reads 13 is stale.

The map's header prose must record, in this order: the root is `form1[0]`; the three x-clusters with
TY2025 coordinates; **8b's amount is `f1_17`, the write-in `f1_16`** (inverted from TY2024); 8d is a
ReadOnly reserved widget; and the §5.2 blank list.

### 5.4 What must be collected from the filer

**Nothing new for the form.** Every Schedule A input already exists. The one TY2025-only collection is
the MAGI add-back surface, which is built and shared with Schedule 1-A:

| worksheet line | collected as | site |
|---|---|---|
| 3a — excluded Puerto Rico income (§933) | `ReturnInputs` | `return_inputs.rs:996` |
| 3b — Form 2555 line 45 | `ReturnInputs` | `return_inputs.rs` §911 group |
| 3c — Form 2555 line 50 | `ReturnInputs` | ditto |
| 3d — Form 4563 line 15 | `ReturnInputs` | ditto |
| the gate | `has_income_exclusion: Option<bool>` | `return_inputs.rs:970`; `QuestionId::HasIncomeExclusion`, `questions.rs:96` |

Per `CLAUDE.md`: *"If the form asks something our input surface cannot answer, collect it."* Here it
already can. **The obligation this spec adds is the inverse** — that the existing question stays live
for Schedule A even on a return with no Schedule 1-A deduction.

### 5.5 How it is tested — and the mutation that must make each RED

Every gate below is stated as a *pair*: the assertion, and the planted defect that must turn it red
(harness rule **B1**, `design/HARNESS.md`).

| id | gate | **mutation that must RED it** |
|---|---|---|
| **KAT-1** | **Field census.** `(map FQNs) ∪ (census FQNs) == PDF AcroForm FQNs`, exactly, for `f1040sa` **year 2025**. | Delete any one census entry ⇒ *"in NEITHER the map nor the census"*. Add a stale `f1_34[0]` ⇒ *phantom*. Put one FQN in both ⇒ *contradiction*. All three messages already exist at `field_census.rs:117/128/136`. |
| **KAT-2** | **★ The census must actually run on 2025.** `field_census.rs:105` reads `let year = 2024;` — hardcoded. The five existing TY2025 maps (`f1040`, `f8949`, `schedule_d`, `schedule_se`, `f8283`) are **not censused today**, and a TY2025 Schedule A map would join them uncensused. Parameterise the gate over every year directory that has maps. | **Before** writing the 2025 map, plant a bogus FQN in `forms/2025/f1040.map.toml` and confirm the *current* test stays **green**. That green is the defect. Only then fix the year loop and watch it red. This is the B1 kill-test, and per `HARNESS.md` it *cannot be written without discovering the blindness*. |
| **KAT-3** | **Line-label conformance.** The map's line keys, enumerated **from `label-census f1040sa--2025`** (never a hand list, never a range), equal `{1,2,3,4,5a..5e,6,7,8,8a..8e,9,10,11,12,13,14,15,16,17,18}` minus label `5` (heading, recorded with its reason). | Drop `line9` from the map ⇒ red with *"9 accounted for by nothing"*. Hand-listing the labels instead of reading them ⇒ the `blank-is-the-normal-case` failure this repo has already made twice. |
| **KAT-4** | **Cross-reference conformance.** Assert against `design/forms/extract/f1040sa--2025.txt` that L2 cites `line 11b`, L17 cites `line 12e`, and L5e's text contains `$40,000`, `$20,000`, `$500,000`, `$250,000`. | Revert any one citation to its TY2024 wording ⇒ red. This is the gate that would have caught the Form 6251 line-33 `12`/`22` class of defect. |
| **KAT-5** | **The SALT worksheet, per filing status × per region.** Four regions — *unlimited* (5d ≤ trigger), *capped-not-phased*, *phasing*, *at the floor* — × four statuses. Already partly present at `tables.rs`; extend to bind the printed line 5e, not just `line_5e()`. | Halve `line1_cap`/`line9_floor` for MFS (the Tax-Calculator shape) ⇒ MFS/5d=$30,000/MAGI=$300,000 must move **$12,500 → $5,000** and red. Move the halving off line 10 ⇒ red. |
| **KAT-6** | **★ The AMT consequence.** A household with SALT ≥ $40,000 that owes **no** AMT under the TY2024 $10,000 cap must be asserted to owe AMT under TY2025, via Form 6251 line 2a = Schedule A line 7. | Wire 6251 L2a to line 5e instead of line 7 ⇒ green today (they are equal while line 6 is blank) — so the KAT must instead **red** when L2a is fed a $10,000-capped line 7, proving the add-back grew with the cap. |
| **KAT-7** | **Geometric read-back, per year.** Fill Schedule A for 2025 and assert every write lands in its TY2025 cluster. | Point `line9` at `f1_23[0]` (line 11's box). **This must red — and today it would NOT** (§4.3): same column, order preserved. See M-3. |
| **M-3** | **★ Widen the oracle: absolute row identity, not relative order.** `verify_flat`'s descent leg proves monotonicity, which a uniform shift preserves. Add a leg that binds each placement to the y-band of *its own printed label*, read from `design/forms/geometry/f1040sa--2025.json` (already committed) — i.e. assert against `label-boxes`, the same source the map is derived from, so a map/geometry disagreement is a failure rather than a coincidence. | The KAT-7 mutation. **Land M-3 before the map**, so it is watched going red → green on a real defect rather than asserted to work. |
| **KAT-8** | **Golden packet.** Extend `golden_packet.rs` (see `:1012` `mfj_itemized_salt_over_the_cap`) with a TY2025 twin whose SALT is above $40,000 and whose MAGI is above $500,000. | Revert `SaltLimitation` to `FlatCap` for 2025 ⇒ the golden's line 5e changes ⇒ red. ★ Per `a-golden-cannot-validate-its-own-regeneration`: this golden must be **derived from the worksheet by hand once and pinned**, never regenerated from the engine it tests. |
| **KAT-9** | **Double-oracle reconciliation.** Schedule A lines 5e/7/17 against OTS and Tax-Calculator on the TY2025 corpus. | ★ **Expect a two-oracle split, and encode its mechanism, not its outcome.** `SPEC.md:241-245` records that taxcalc parameterises SALT as halved constants — so on MFS it will disagree by a *computable* amount. Per `CLAUDE.md`, that excuse must be **computed from the defect's mechanism** and must name the exact size, never keyed to a vector name. A divergence of the wrong size on an expected-divergent vector stays a failure. |

**Not a gate, but required before the phase closes:** `archive-check` and `harness-check` must pass, and
`cargo run -p xtask -- extract-geometry f1040sa--2025` must reproduce the committed fixture byte-for-byte
(it did in this session).

---

## 6. Cost

### 6.1 Against the 8 forms the recon called trivial

For a stable form (`f8275` byte-identical; `f8959`/`f8960` with **0** field-geometry differences), the
port is: copy the map, repoint `year`, vendor the PDF, add two consts and a match arm, re-run
`dump-fields` to confirm the field set is unchanged, update the sha256 note. Mechanical; the map's
existing prose is still true because the PDF is the same document.

Schedule A shares none of that. **The PDF was rebuilt**: different root subform, 4 fewer fields, every
suffix below line 6 shifted, two new intermediate subforms, and all three column bands moved left. So:

| work item | trivial form | Schedule A TY2025 |
|---|---|---|
| PDF vendoring + consts + match arm | same | same |
| field names | confirm unchanged | **re-derive all 33** |
| line↔field correlation | reuse | **re-derive** (`label-boxes` — done, §5.3) |
| column bands | none (code has none, or unchanged) | **new per-year constants + make the code year-aware** |
| map prose (~50% of the file) | true as-is | **rewrite the three ★ traps**; the 8b inversion flipped |
| `[census]` | copy | **13 entries → 9**, re-reasoned per field |
| computation | none | **already built** — the one thing that would have been expensive |
| oracle widening | none | **M-3 required** (§4.3) |
| conformance gate | none | **KAT-2 required** — the census does not run on 2025 at all |

### 6.2 Estimate

Taking a trivial port as the unit:

| | multiple of a trivial port | why |
|---|---|---|
| map derivation + census | **≈ 3×** | 33 fields correlated and reasoned; the correlation itself is one command |
| year-aware clusters + `pdf.rs`/`map.rs` wiring | **≈ 1×** | small, mechanical |
| KAT-2 (census year loop) + its planted-defect kill | **≈ 2×** | it fixes a gap affecting **six** forms, not just this one |
| M-3 (absolute row identity in `verify_flat`) | **≈ 3×** | touches the shared verifier; needs the planted defect watched red first |
| KAT-4..KAT-9 | **≈ 4×** | KAT-6 and KAT-9 need real households and a two-oracle mechanism-derived excuse |
| **total** | **≈ 13× a trivial port** | |

**Two of those five items are not really Schedule A's cost.** KAT-2 and M-3 are latent gaps this form
merely *exposes* — the census has never run on TY2025, and the geometric oracle has never been shown a
same-column row shift. Charged to their true owners, Schedule A itself is **≈ 8×** a trivial port, and
the constellation carries the other 5×.

### 6.3 Follow-ups this spec opens

| id | item | owning phase |
|---|---|---|
| SA25-1 | Fetch `26USC_s164.html` into `legal/primary-sources/statute-irc/`; re-cite §2 against it. | the phase that lands the map |
| SA25-2 | `field_census.rs:105` year loop (KAT-2) — five existing TY2025 maps are uncensused **today**. | **before** the map, so the kill-test can be watched red |
| SA25-3 | `verify_flat` absolute-row leg (M-3). | before the map |
| SA25-4 | `printed.rs:1325` doc comment asserts the TY2024 flat cap as the rule. | with the map |
| SA25-5 | `HasIncomeExclusion` now has two consumers; add a test that reds if it is narrowed to Schedule 1-A. | with the map |

---

## Appendix — every command this document's claims were measured with

```
cargo run -p xtask -- dump-fields design/forms/2024/f1040sa--2024.pdf     # 37 fields, topmostSubform[0]
cargo run -p xtask -- dump-fields design/forms/2025/f1040sa--2025.pdf     # 33 fields, form1[0]
cargo run -p xtask -- label-boxes  f1040sa--2024                          # field → line, 2024
cargo run -p xtask -- label-boxes  f1040sa--2025                          # field → line, 2025
cargo run -p xtask -- label-census f1040sa--2024                          # 28 labels, 37 boxes
cargo run -p xtask -- label-census f1040sa--2025                          # 28 labels, 33 boxes
cargo run -p xtask -- label-proof  f1040sa--2025                          # 33 boxes, 2 `?`, 1 label without a box ("5")
cargo run -p xtask -- extract-geometry f1040sa--2025                      # reproduces the committed fixture byte-identically
pdftotext -layout design/forms/2025/i1040sca--2025.pdf -                  # the SALT worksheet, unreflowed (line 502ff)
```

Sources of record: `design/forms/extract/f1040sa--{2024,2025}.txt`, `i1040sca--2025.txt`,
`f6251--{2024,2025}.txt`, `f1040--2025.txt`, `f1040s1a--2025.txt`; `design/forms/MANIFEST.json:327-333`;
`crates/btctax-forms/forms/2024/f1040sa.map.toml`; `crates/btctax-forms/src/{schedule_a,verify,pdf,map}.rs`;
`crates/btctax-core/src/tax/{tables,printed,return_1040,return_inputs,questions}.rs`;
`crates/btctax-forms/tests/field_census.rs`; `crates/btctax-adapters/src/tax_tables.rs:788-822`.
