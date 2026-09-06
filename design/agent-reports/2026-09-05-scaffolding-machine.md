# The year-port machine, specified — and the instrument it would be built on is blind on drafts

**Lens:** the port machine, concretely (inputs, passes, outputs, stop points). **Date:** 2026-09-05.
**Method:** `./target/debug/xtask` (prebuilt), `pdftotext`/`pdfinfo`, `.venv/bin/python`, file reads.
No cargo, no make, no git, no network.

---

## HEADLINE

**Every one of the 16 committed TY2026 draft geometry fixtures is off by exactly one page, because
`form_geometry` derives a field's page from its FQN string instead of from the PDF — and an IRS draft
carries a cover sheet.** The instruments built on that fixture do not fail; they answer confidently and
wrongly. `xtask label-census f8959--2026-DRAFT` reports all **24 money lines of Form 8959 as headings
carrying no amount box**, exit 0. `xtask form-delta f8959--2025 f8959--2026-DRAFT` prints *"no field
changed the printed line it sits beside"* after resolving **zero** labels. **`design/TY2026_WORK_LIST.md`'s
entire "lines that moved" column, and its four "mechanical / unchanged" verdicts, are artifacts of this
bug, not measurements of the drafts.** The port machine specified below therefore starts with a geometry
fix and a coverage axis, not with a `.map.toml` generator.

---

## FINDINGS

### F1 — CRITICAL. A field's page comes from its NAME, and a draft has an extra page.

`crates/xtask/src/form_geometry.rs:228-234`:

```rust
    // The page is not carried on `Field`; the IRS templates always nest widgets under a `PageN[0]`
    // subform, so the FQN is the page. Same derivation `dump-fields` uses — one rule, not two.
    let page_of = |fqn: &str| -> u32 {
        fqn.split('.')
            .find_map(|seg| seg.strip_prefix("Page")?.split('[').next()?.parse().ok())
            .unwrap_or(0)
    };
```

The *words* are indexed by the real PDF page, counted off `pdftotext -bbox`'s `<page>` elements
(`form_geometry.rs:112-130`). So the two halves of the join use two different page numberings the
moment they disagree — and an IRS draft is exactly when they disagree, because the IRS prepends a
`Caution: DRAFT—NOT FOR FILING` cover sheet as physical page 1.

Measured, all 16 archived drafts vs all 33 archived finals:

```
STEM                   PAGES BOXPG RESOLV UNRES        (BOXPG = max box page in the fixture)
f1040--2026-DRAFT          3     2    31   168
f1040s1--2026-DRAFT        3     2     0     0
f1040s1a--2026-DRAFT       4     3    76   109
f1040s2--2026-DRAFT        3     2    17    51
f1040s3--2026-DRAFT        2     1     0    38
f1040sa--2026-DRAFT        3     2    21    26
f1040sb--2026-DRAFT        2     1     0    72
f1040sc--2026-DRAFT        3     2    34    75
f1040sd--2026-DRAFT        3     2     8    47
f1040sse--2026-DRAFT       3     2     1    26
f6251--2026-DRAFT          3     2    28    34
f8949--2026-DRAFT          3     2    93   109
f8959--2026-DRAFT          2     1     0    26
f8960--2026-DRAFT          2     1     0    38
f8995--2026-DRAFT          2     1     0    36
f8995a--2026-DRAFT         3     2    35    79
```

**Every draft: `BOXPG == PAGES - 1`. Every final: `boxmax == wordmax == pages`, all 33 of them.**

```
$ .venv/bin/python  (Counter over design/forms/geometry/*.json)
f6251--2025       pages 2   box pages {1:33, 2:29}   word pages {1:1239, 2:1314}
f6251--2026-DRAFT pages 3   box pages {1:33, 2:29}   word pages {1:343, 2:1257, 3:1330}
```

Page 1 of the draft holds 343 words — the cover sheet. So form-page-1 fields are joined against the
cover sheet, and form-page-2 fields against form page 1. Self-evident proof, one line:

```
$ ./target/debug/xtask label-boxes f6251--2026-DRAFT | awk -F'\t' '$3==2' | head -3
topmostSubform[0].Page2[0].f2_1[0]	?	2
topmostSubform[0].Page2[0].f2_2[0]	1a	2      ← a field named Page2 cannot sit beside line 1a
topmostSubform[0].Page2[0].f2_3[0]	1b	2
```

Form 6251 page 2 is Part III, lines 12–40. Lines 1a/1b are on page 1.

### F2 — The failure is SILENT and takes the shape of a clean verdict.

`form_delta.rs:76-86` compares a field's old and new label only when neither is `"?"`:

```rust
                if x != y && x != "?" && y != "?" {
                    label_moved.insert(f.clone(), (x.clone(), y.clone()));
                }
```

A field whose label failed to resolve is therefore *dropped*, not reported. On the five one-page forms
the drop is total, and `labels_available` is still `true`, so the tool's own
`★ LINE->LABEL DRIFT NOT CHECKED` warning never fires:

```
$ ./target/debug/xtask form-delta f8959--2025 f8959--2026-DRAFT
form-delta f8959--2025 -> f8959--2026-DRAFT
  fields: 26 common, 0 added, 0 removed
  line->label: no field changed the printed line it sits beside

$ ./target/debug/xtask form-delta f1040sb--2025 f1040sb--2026-DRAFT
  fields: 72 common, 0 added, 0 removed
  line->label: no field changed the printed line it sits beside
```

Zero of 26 and zero of 72 labels resolved on the new side. **The safest-sounding verdict in the tool
is the one it emits when it saw nothing at all.** That is F2/F4 from `design/HARNESS.md` — an
instrument reporting success while blind — reproduced inside the instrument built to prevent it.

The same shape in the census, which exists to enforce *"every line has a determinate provenance"*:

```
$ ./target/debug/xtask label-census f8959--2026-DRAFT
# f8959--2026-DRAFT — 24 labels, 26 boxes
  H p2 1     no AcroForm box in this row's span — heading, or a non-money entry
  H p2 2     no AcroForm box in this row's span — heading, or a non-money entry
  … (all 24)                                                       exit code 0

$ ./target/debug/xtask label-census f8959--2025
# f8959--2025 — 24 labels, 26 boxes
    p1 1                                                           (24 rows, zero H)
```

Same 26 fields, same 24 labels, one cover sheet apart.

### F3 — The B1 test that should have caught this asserts the wrong property.

`form_delta.rs`'s TY2026 test (`the_ty2026_drafts_are_ready_to_be_diffed_against_their_finals`) asserts
`d.labels_available` and `!d.common.is_empty()`. Both are true today and both are true while the join is
garbage. `labels_available` means *a fixture exists on each side*, which is not the property the test
names in its own doc comment (*"or the day the final lands the label axis is unchecked and the port is
guesswork"*). The two calibrated tests are honest and both are FINAL→FINAL; the drafts were never in
the calibration set, which is the repo's own rule — *a tool calibrated on one failure mode is calibrated
on none* — applied to shapes rather than to artifact classes.

### F4 — Two panics that landed today cite the invalid number as their justification.

`crates/btctax-forms/src/form1040.rs:34-58` and `crates/btctax-forms/src/schedule_se.rs:37-56` both say:

> *"Measured on the TY2026 drafts: Form 1040 keeps all 199 field names and moves 31 printed line
> bindings, so 'the names still resolve' is no evidence at all that the geometry did not move."*

**The fix is right and must stay** — an unenumerated year must panic rather than inherit last year's
x-bands. The *evidence sentence* is a product of F1 (f1040: 31 resolved / 168 unresolved, all shifted)
and must be recomputed, or the next reader will trust a number that measures a cover sheet.
`design/TY2026_WORK_LIST.md`'s headline and its whole `lines that moved` column carry the same defect.

### F5 — Pass 2 has neither a producer nor a TY2026 input.

58 of the 64 committed extracts name their own regenerator:

```
$ head -6 design/forms/extract/*.txt | grep -o 'xtask [a-z-]*' | sort | uniq -c
     58 xtask --
$ head -6 design/forms/extract/f8959--2025.txt
# GENERATED — do not hand-edit. Text layer of design/forms/2025/f8959--2025.pdf
# sha256:13e64004948331d2…  |  pdftotext -layout
# Regenerate: cargo run -p xtask -- forms extract
```

There is no `forms` subcommand: `crates/xtask/src/main.rs:200-230`'s usage string lists 18 commands and
`forms` is not among them. Meanwhile the drafts' text layers exist but in the wrong place and under the
*note* filename — `scripts/archive_drafts.py:126` writes `str(dest) + ".txt"`, i.e.
`design/forms/2026/f6251--2026-DRAFT.pdf.txt`, while `design/forms/README.md` reserves `<name>.pdf.txt`
for the URL+sha256+size note and every conformance instrument reads
`design/forms/extract/<stem>.txt` (`line_coverage_check.rs:565`). Measured:
`ls design/forms/extract/ | grep -c 2026` → **0**. All 16 manifest draft entries carry `extract: ""`,
which is at least honest.

Per-file flags are recorded and exact, so the producer is wiring, not discovery: **32 `-layout`,
27 no-flags, 1 explicit command** across the 64 headers.

### F6 — The wiring is a pure function of a directory listing, and the check for it reds today.

```
$ for m in crates/btctax-forms/forms/2025/*.map.toml; do
    grep -q "forms/2025/$(basename $m .map.toml).map.toml" crates/btctax-forms/src/map.rs || echo INERT
  done
INERT: f1040s1a f1040s2 f1040s3 f1040sa f1040sb f1040sc f6251 f8959 f8960 f8995      (10 of 15)
```

`map.rs` `include_str!`s 27 map paths and `pdf.rs` `include_bytes!`s **the identical 27 stems** — the two
sets are exactly parallel. So a walk of `crates/btctax-forms/forms/*/` that asserts every `.map.toml` is
reachable from `map.rs` **is a one-screen test that reds on today's tree with 10 named stems**. That is a
free B1 kill-test for the wiring generator, available before the generator exists. All 17 `for_year`
functions already end in `UnsupportedYear` (`map.rs:189,385,719,885,1012,1154,1225,1314,1452,1528,1598,
1655,1750,1831,1917,2006,2082`), so the *failure* direction is already correct; what is missing is any
signal that a correct map on disk is unreachable.

### F7 — What the machine must produce, already written out longhand.

`crates/btctax-forms/forms/2025/f8959.map.toml:1-30` is the port machine's output specification, typed
by an agent. Every claim in it is a command re-narrated as English: the bundled sha256 and byte count
(`sha256sum`), the byte-for-byte copy check (`cmp`), the note round-trip (the `.pdf.txt` note), the
field inventory diff (`form-delta` axis 1), the `diff -b` of the two extracts (pass 2), the line set
enumerated from the extract (`label-census`), and the two-witness geometric join (`label-boxes`).
**The machine should emit this header from the commands rather than have a human retype their results.**

Composition of what is *not* a command's output, measured:

```
             maps   total lines   lineN bindings   `reason =` count   reason bytes
TY2024        17        2669            239              551            125,331
TY2025        15        2151            195              242             55,329
```

---

## THE MACHINE — inputs, passes, outputs, refusals

One namespace, `xtask forms`, because 58 committed files already promise it.

### 1. `xtask forms fetch <year> [--drafts] [--stem <s>]…`

**In:** a year. **Out:** `design/forms/<year>/<stem>--<year>[-DRAFT].pdf` (gitignored), a note
`<stem>--<year>.pdf.note` carrying URL + sha256 + bytes, and a `MANIFEST.json` entry.
`scripts/archive_drafts.py` already is this for drafts and its year-verification refusal is correct —
promote it, and **stop writing the text layer to `<stem>.pdf.txt`**, which collides with the note name.
**Refuses:** the year printed on the document ≠ the year requested (already implemented, keep verbatim);
a draft path serving a final (already implemented).

### 2. `xtask forms extract <stem> | --all` — the missing ② step

**In:** the archived PDF + its note. **Out:** `design/forms/extract/<stem>.txt` with the existing
generated header, using the flags recorded in that file's own header. **Refuses:** the PDF's sha256 ≠
the note's; a stem with no note. `--all` prints a census of `written / unchanged / missing-pdf`.

### 3. `xtask forms geometry <stem>` — the fixed fixture (replaces `extract-geometry`)

**Out:** `design/forms/geometry/<stem>.json`, plus two new required fields:

- `cover_pages: N` — physical pages before the form body, **derived** by counting pages with zero
  widget annotations at the front, and stored so it can be checked rather than assumed.
- `page_src: "annot" | "fqn"` per box — the page comes from the **widget annotation's page index**;
  the FQN rule survives only as a fallback, and it is recorded so a fixture built the weak way is
  greppable rather than indistinguishable.

**Refuses:** `cover_pages > 0` together with any `page_src: "fqn"`; `max(box.page) != pages -
cover_pages`; a page carrying words and no boxes that is not declared a cover page.
**B1 kill-test, available now:** regenerate `f8959--2026-DRAFT` and assert the census yields 24 `Amount`
rows, not 24 `Heading` rows. The planted defect is already on disk.

### 4. `xtask forms delta <old-stem> <new-stem>` — three axes, not two

Keeps the two calibrated axes and adds the one whose absence produced F2:

- **coverage** — `resolved a/N old, b/N new`, and the unresolved *sets*. `label_moved.is_empty()` may be
  reported as *"no line moved"* **only when both sides resolve the same field set**. Otherwise the tool
  prints `LABEL AXIS BLIND ON n FIELDS — this is not a clean run` and **exits non-zero**.
- **B1 kill-test:** `f8959--2025` vs `f8959--2026-DRAFT` must be RED today and GREEN after the geometry
  fix; and a fixture with an injected one-page shift must be red forever.

### 5. `xtask forms port <stem> --from <y1> --to <y2> [--allow-draft] [--emit-wiring]`

**The product is a refusal or a draft map plus a wiring patch. Never a silent copy.** Passes run in
order and each can stop the run:

| pass | what it does | refusal |
|---|---|---|
| **P0 provenance** | both PDFs present, sha256 = note, `Entry::is_draft(to)` consulted | a draft `to` without `--allow-draft`. With it, the emitted map carries `provisional = true` **and its generated `for_year` arm returns `UnsupportedYear`**, so a provisional port cannot be loaded by construction |
| **P1 field set** | `forms delta` axis 1 | any add/remove ⇒ those names listed, and every binding touching one is `undecided` |
| **P2 line→label** | re-derive every `lineN = field` from the **to** form's own fixed geometry | per binding: `carried` / `moved` / `orphan`. Any `moved` or `orphan` ⇒ non-zero exit |
| **P3 line text** | `diff -b` of that line's printed text between the two extracts | changed ⇒ the doc comment is **deleted**, the line becomes `needs-transcription`. A stale instruction quote is worse than none |
| **P4 census carry** | a `rule`/`reason` carries only if name ∧ label ∧ line-text all unchanged | otherwise `rule = "undecided"`, reason = the pasted diff. The census test reds on it |
| **P5 wiring** | emit the five mechanical edits per `(stem, year)` — `include_str!`, `ty<YYYY>()`, the `for_year` arm, `include_bytes!`, the `*_pdf` arm | a map with any non-`carried` line gets an `UnsupportedYear` arm and a `#[test]` that reds until a human clears it |

**Outputs:** `crates/btctax-forms/forms/<y2>/<stem>.map.toml` with the F7 header *generated*;
`design/forms/port/<stem>--<y1>-to-<y2>.json`, the per-line worklist
(`carried | moved | orphan | needs-transcription | undecided`); a stdout summary; non-zero exit while any
line is not `carried`.

### 6. `xtask forms wire --check`

The reachability walk from F6. Reds today, naming 10 stems. Build it **before** the generator so it can
be watched going red → green, exactly as A1 was built before its hooks.

### 7. `xtask forms port-status <year>`

Per stem: `carried / needs-human / absent`, plus totals. This is what makes *"adding a year is a data
change"* a number rather than a claim, and it is the artifact that replaces a hand-maintained work list.

---

## WHERE IT MUST STOP, AND WHY

1. **Any `needs-transcription` line.** A changed instruction sentence is testimony the filer signs under
   26 USC 6065. A machine cannot decide the new sentence means the same thing, and the repo's whole AMT
   defect history is compressed instruction text.
2. **Any `undecided` census disposition.** *"unmodeled because btctax collects no RRTA income"* is a
   claim about the engine, not the PDF. 551 such reasons (125,331 bytes) at TY2024, 242 (55,329) at TY2025.
3. **Any `moved` binding.** The machine proposes the rebind and must not commit it: this is exactly the
   class that puts the AMT in line 10's box. Human confirms against the printed form.
4. **A rebuilt form.** Schedule 1-A keeps 10 fields of 219. The machine must refuse and say *"this is not
   a port"* rather than emit a 10-line map and 209 `undecided`s.
5. **Every figure.** The machine never writes a dollar amount. Drafts are evidence; the encodable sources
   are the final form and the Rev. Proc.
6. **Everything downstream of the map.** `Form6251Line1Rule::Y2026`, `Schedule1A`'s struct, the compute
   layer. The machine may emit the *empty enum variant* — the compiler then enumerates every site, free
   and exact — and must not fill one.
7. **The coverage floor.** What counts as "enough labels resolved" for a given form is a judgment; the
   machine reports the number and refuses on a mismatch of sets, and does not pick a threshold.

---

## MECHANICAL — a machine can do it

1. Fetch, year-verify, hash, note, manifest-record (exists; promote and fix the `.pdf.txt` collision).
2. `forms extract` — the ② step, per-file flags already recorded (32 `-layout`, 27 none).
3. Widget-annotation page resolution + `cover_pages` derivation + the `page_src` provenance field.
4. The `max(box.page) == pages - cover_pages` invariant across all 49 fixtures.
5. Field-name set diff (exists, calibrated both directions).
6. Line→label join — proven 177/177 on finals by the prior lens; blocked on F1 for drafts.
7. Label **coverage** reporting and the blind-run non-zero exit.
8. Per-line printed-text `diff -b` (pass 2/P3).
9. Evaluating the census-carry *condition* (name ∧ label ∧ text unchanged).
10. The five wiring edits per `(stem, year)` — a pure function of the pair.
11. The reachability walk (`wire --check`), red today on 10 stems.
12. Generating the map header narrative that is currently retyped per form.
13. `port-status <year>` and the regeneration of `design/TY2026_WORK_LIST.md` from the tool.
14. Emitting a new `Form6251Line1Rule` variant so the compiler enumerates the call sites.

## HUMAN — someone must read a form

1. Transcribing any changed instruction sentence into a doc comment, from the text layer.
2. Every `reason` for a non-`carried` census disposition — model it or census it.
3. Confirming each `moved` rebind against the printed form.
4. Adjudicating a rebuilt form (Schedule 1-A) as a new transcription rather than a port.
5. Deciding what a new field or checkbox means for the product.
6. Encoding any figure, only from a final form or a Rev. Proc.
7. Filling `Form6251Line1Rule::Y2026`'s contents and every compute-layer consequence.
8. Deciding whether a draft's structure may inform a given piece of work at all.
9. Re-measuring `design/TY2026_WORK_LIST.md`'s deltas and the two panic messages' evidence sentences
   once F1 is fixed.

---

## WHAT I COULD NOT ESTABLISH

- **The true TY2026 line→label deltas.** Every number in `TY2026_WORK_LIST.md` is downstream of F1 and
  I did not recompute them: doing so needs the geometry extractor changed, and this lens does not build.
- **Whether `f1040s1--2026-DRAFT`'s 73 boxes resolve at all** — it reported `0 resolved, 0 unresolved`
  from `label-boxes` while its fixture holds 73 boxes and 2,189 words, which is a third shape I did not
  chase.
- **Whether any widget annotation genuinely lacks a resolvable page** in these PDFs, which is what
  decides whether the `page_src: "fqn"` fallback is ever needed or should simply be deleted.
