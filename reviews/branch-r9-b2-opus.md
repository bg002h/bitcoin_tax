<!-- ★ SSN-shaped tokens in this report's rendered examples are REDACTED to `111-22-000N`. They
     were synthetic scaffold values (`format!("111-22-{:04}", i)`), but a literal 3-2-4 token in a
     committed document is exactly what the generic PII scan exists to catch, and allowlisting
     document-specific digits would dilute a list whose value is that every entry recurs. -->

# r9 — B2 (>4 dependents / the continuation statement), independent review

**Reviewer:** Opus, own worktree, no commits.
**Scope as briefed:** `git diff fdc2324..HEAD -- crates` — `3a4d06a` (B2) and `b296afe` (the DRAFT banner).
**Baseline established before any mutation:** `make check` → **2568 tests run: 2568 passed, 12 skipped**.

> Note on the first `make check` in a fresh worktree: it reported one failure,
> `xtask::harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory`.
> That is **not** a code defect and **not** in the B2 diff — `scripts/hooks/on-write.sh:41-42,59-69`
> fails closed when `target/{debug,release}/xtask` is absent, and `cargo nextest` builds test
> harnesses, not that binary. `cargo build -p xtask --bin xtask` then made it pass. Recorded as N-3
> below because it means `make check` on a clean clone reds.

---

## VERDICT: 0 Critical / 2 Important

Both Importants are **the same class and both were killed by mutation**: a guarantee stated in a
`★★★` comment, on a filed artifact, that no test reds on. Neither is a wrong mark on the page — the
page itself I judge correct. The defect is that nothing holds it there.

---

## I-1 — the DRAFT banner is never exercised at its call site; the committed reason for not testing it is factually wrong

```
SEVERITY: Important
WHERE:    crates/btctax-cli/src/cmd/admin.rs:965
          crates/btctax-cli/tests/export_irs_pdf.rs:208-211 (the rationale)
CLAIM:    Nothing in the suite reds if the dependents statement stops carrying the DRAFT banner, and
          the in-source explanation for why the end-to-end leg "would silently prove nothing" is
          false — `pseudo_active()` is not year-scoped, so the test is writable in ~30 lines.
```

**FAILURE (concrete).** A filer on a pseudo-reconciled ledger with 9 dependents exports TY2024. Every
PDF is stamped `DRAFT — ESTIMATE, NOT FOR FILING`. `dependents_statement.txt` is the one page they
**detach** — and if the banner regresses it leaves the machine looking like a clean page, carrying
five dependents' full names and SSNs, next to fourteen pages that all shout that the figures are
synthetic. The code's own comment names exactly this: *"the one artifact most likely to be separated
from the forms that carry the warning."*

**EVIDENCE — mutation 1, run to completion.**

```rust
// crates/btctax-cli/src/cmd/admin.rs:965 — mutated
write_bytes_owner_only(&path, statement_body(&st.body, /* watermarked */ false).as_bytes())?;
```

`make check` → **`2568 tests run: 2568 passed, 12 skipped`.** Green. The banner is off for every
pseudo packet and the gate does not notice.

Why the existing tests miss it, precisely:

- `statement_body`'s own unit test (`admin.rs:1046-1069`) tests the **pure function**. It stays green
  because the function is still correct; only the argument changed.
- `a_dependents_statement_is_marked_draft_only_on_a_pseudo_ledger`
  (`export_irs_pdf.rs:161-212`) asserts only the **clean** leg (`!body.contains("NOT FOR FILING")`),
  which the mutation also satisfies.

So the answer to B1's one-sentence question — *"which test reds when this checker is removed?"* — is
**none**. Per `CLAUDE.md`: *"A guarantee without a test that reds when it is removed does not exist."*

**EVIDENCE — the committed rationale is wrong.** `export_irs_pdf.rs:208-211` says:

> *"Asserting it here would need a pseudo-reconciled TY2024 full return, and the pseudo fixtures are
> TY2025, which has no full-return path: the assertion would never execute and the test would
> silently prove nothing."*

The watermark predicate is `let watermarked = state.pseudo_active();` (`admin.rs:865`), and

```rust
// crates/btctax-core/src/state.rs:309-311
pub fn pseudo_active(&self) -> bool {
    self.pseudo_synthetic_count > 0
}
```

is a **whole-ledger** count — it is not year-scoped. The dispatch (`admin.rs:456`) picks the
full-return pipeline purely on `return_inputs::exists(conn, tax_year)`. So a **TY2025 pseudo ledger
plus TY2024 `return_inputs` watermarks a TY2024 full-return export**, and the leg is trivially
writable.

I wrote it to be sure. Appended to the existing test, using the fixtures already in the file:

```rust
let (_d2, pseudo) = make_vault(&pseudo_events());          // TY2025 synthetic events
cmd::reconcile::pseudo_set_mode(&pseudo, &pp(), true).unwrap();
/* … TY2024 ReturnInputs, nine(&mut ri), return_inputs::set(s.conn(), 2024, &ri) … */
let rep2 = cmd::admin::export_irs_pdf(
    &pseudo, &pp(), out2.path(), 2024, &[], Some(btctax_cli::ATTEST_PHRASE)).unwrap();
assert!(rep2.watermarked, "a pseudo ledger IS watermarked");
let body2 = std::fs::read_to_string(out2.path().join("dependents_statement.txt")).unwrap();
assert!(body2.starts_with("*** DRAFT — ESTIMATE, NOT FOR FILING ***"), "…\n{body2}");
```

Result, both directions:

| | outcome |
|---|---|
| with mutation 1 applied | **FAILED** — and it printed the real defect: a banner-free statement listing `Kid 4 … Kid 8` with full SSNs, produced from a pseudo ledger |
| mutation reverted | **ok** |

That is the kill test, and it costs 30 lines against fixtures already present in the file.

**Remedy.** Land that assertion (or equivalent) so the call site's `watermarked` argument is held,
and delete the rationale comment — it is the load-bearing wrong belief here, and it will be re-read
as settled by the next person.

---

## I-2 — the manifest entry that tells the filer to attach the page is likewise untested

```
SEVERITY: Important
WHERE:    crates/btctax-cli/src/cmd/admin.rs:966
CLAIM:    Deleting the manifest line for the statement leaves the whole suite green, even though the
          manifest is the ONLY thing that tells the filer the page exists and must be attached.
```

**FAILURE (concrete).** The export directory is `00_f1040.pdf … 155_f8283.pdf`, `manifest.txt`,
`basis_methodology.txt`, `dependents_statement.txt`. `manifest.txt` opens
`# btctax full-return packet — staple in this order` and is the filer's instruction sheet. If the
`ATT` line stops being emitted, a filer who follows the manifest mails a Form 1040 with the **"more
than four dependents" box checked and no attachment** — an incomplete return, and the one outcome
`packet.rs:38-42` says the whole in-`fill_full_return` design exists to prevent. The page sits
unmentioned in the directory.

**EVIDENCE — mutation 2, run to completion.**

```rust
// crates/btctax-cli/src/cmd/admin.rs:966 — line deleted
// let _ = writeln!(manifest, "  ATT  {}.txt  (attach to Form 1040)", st.name);
```

`make check` → **`2568 tests run: 2568 passed, 12 skipped`.** Green.

The guarantee is stated in the code at `admin.rs:954-957` in its own words — *"they are listed in the
manifest, because the manifest is what tells a filer what to staple. A page the filer never learns to
attach may as well not have been generated"* — and nothing enforces it. `promote_cli.rs:1480-1548`
enumerates the emitted file set exactly, but on a **0-dependent** scenario, so it never sees this
line.

**Remedy.** One assertion in the test I-1 already reaches:
`assert!(manifest.contains("dependents_statement.txt"))`.

---

## THE ONE QUESTION — is there anything on the page the filer did not assert?

**My answer: no, on the current wording.** I rendered the artifact rather than reading the format
string, at n=9 and at a long-name shape, via a temporary test in
`btctax-core::tax::dependents_statement` (since reverted). Verbatim:

```
Form 1040 (2024) — CONTINUATION STATEMENT: DEPENDENTS
Attach to Form 1040. The box under Dependents on page 1 is checked.

Name(s) shown on return: John Doe & Jane Doe
Your social security number: 123-45-6789

This return claims 9 dependents. Dependents 1-4 are listed on page 1 of Form 1040;
dependents 5-9 are listed below. Together they are the complete list.

                                                                     (4) Check the box if qualifies for
      (1) First name    Last name          (2) Social security   (3) Relationship        Child tax    Credit for other
                                               number               to you                credit        dependents
     ---------------------------------------------------------------------------------------------------------------
  5  Firstname4 Lastname4               111-22-000N           Daughter              [  ]          [  ]
  …
```

Ten utterances, adjudicated one at a time:

| # | text | verdict |
|---|---|---|
| 1 | title line | a caption, asserts nothing |
| 2a | *"Attach to Form 1040."* | an instruction to the filer, not a statement to the IRS |
| 2b | *"The box under Dependents on page 1 is checked."* | true by construction — see below |
| 3 | `Name(s) shown on return:` | the filer's, verbatim from `header.name_line` |
| 4 | `Your social security number:` | the filer's |
| 5 | *"This return claims 9 dependents."* | a count of the filer's own entries |
| 6 | *"Dependents 1-4 … page 1; dependents 5-9 … below."* | true by `split_at` |
| 7 | *"Together they are the complete list."* | see below |
| 8 | column headings | the form's |
| 9 | the rows | the filer's data |
| 10 | `[  ]  [  ]` | the form's own empty check positions |

**On 2b — "the box on a different page is checked".** Sound, and it is held structurally rather than
by coincidence. Both the checkbox write (`form1040_full.rs:519`) and the statement
(`packet.rs:196`) read `ReturnHeader::more_than_four_dependents()`. I checked the four ways it could
still be false:

1. *A different code path emits the 1040.* There is none. `fill_form_1040_full` has exactly one
   non-test caller (`packet.rs:96`), and `export_full_return` has exactly one (`admin.rs:458`).
2. *A slice export produces a 1040 without a statement.* The dispatch at `admin.rs:456` routes every
   year that has `return_inputs` to the full pipeline unconditionally; `--forms` is downgraded to a
   warning flag, never honored.
3. *A year whose map lacks the box.* `forms/2025/f1040.map.toml` has no `[header]` block at all, and
   `fill_form_1040_full_with_map:92-96` refuses on that before writing a cell; `fill_full_return`
   fills the 1040 **first**, so the all-or-nothing abort means no statement is produced either.
4. *The claim is only true of the source, not the PDF.* It is verified against the emitted bytes:
   `the_checkbox_and_the_statement_are_the_same_decision` (`full_return_forms.rs:2069-2120`) loads
   the filled PDF, reads `c1_13[0]` back, and asserts `boxed == stmt` **and** `boxed == (n > 4)` for
   n ∈ 0..=12.

**On 7 — "Together they are the complete list", and the TIP.** The brief asks whether i1040gi's TIP
means the statement should restate **all** dependents. Arguing from the extracted text:

> `i1040gi--2024.txt:1452-1454` — *"If you have more than four dependents, check the box under
> Dependents on page 1 of Form 1040 or 1040-SR and include a **statement showing the information
> required in columns (1) through (4)**."*
>
> `i1040gi--2024.txt:1465-1467` — *"**TIP** The dependents you claim are those you list by name and
> SSN in the Dependents section on Form 1040 or 1040-SR."*

Supplementing is right, for three reasons:

1. The Step-1 sentence is the **specific** instruction for this exact situation and it asks for
   *"the information required in columns (1) through (4)"* — the columns, not "for all your
   dependents". It does not ask the statement to restate the grid.
2. The TIP is a **definition** — it exists to separate a dependent you claim from a person who is
   merely a qualifying person (the HoH/QSS cell three lines up on the form). Reading it as
   "a dependent not physically on page 1 is not claimed" would make the Step-1 remedy self-defeating,
   since Step 1 is the sanctioned way to have more than the page holds. The statement is an
   **extension** of the Dependents section, which is why the box exists at all.
3. The risk runs the other way. A statement that restated all nine beside a page-1 grid carrying four
   of them invites a processor to read **thirteen**. Supplement has one reading; restate has two.

And on the phrase itself: its antecedent is the nine the previous sentence just counted, so it reads
as an arithmetic reconciliation (4 + 5 = 9), not a claim about the filer's household. It is not the
`"NOT CLAIMED for any dependent"` failure — that one converted a *blank* into an affirmative
assertion; this one restates a count the filer supplied. I would not block on it. If you want it
gone anyway, deleting the sentence costs nothing, since sentence 6 already carries the arithmetic.

**On column (4).** The rejection of `"NOT CLAIMED for any dependent on this return"` was right and
the replacement is right: two empty check positions, exactly what page 1's `ctc`/`odc` widgets are
(mapped, deliberately never written — `map.rs:292-298`). Blank on the page, blank in the statement,
and the forgone credit disclosed through `Advisory::CtcOdcOmitted`. Consistent in both directions.
`the_rendered_statement_transcribes_all_four_columns_and_claims_nothing` forbids `"NOT CLAIMED"`,
`"not claimed"`, `"does not qualify"`, `"no credit"`.

**I found nothing else of that shape.** The page contains no derived tax fact, no credit
eligibility, no age, no residency, no support test, no "none" — nothing but the filer's own four
columns plus arithmetic over them.

---

## Minor / Nit

### M-1 — the column headings do not sit over their columns, and a plausible name collapses the row

```
SEVERITY: Minor
WHERE:    crates/btctax-core/src/tax/dependents_statement.rs:117-131
```

Measured character offsets (0-based) of the committed header block against the row format
`" {:>2}  {:<34} {:<21} {:<21} [  ]          [  ]"`:

| column | heading starts | data starts | drift |
|---|---|---|---|
| (2) Social security | 43 | 40 | 3 |
| (3) Relationship | 65 | 62 | 3 |
| Child tax | 89 | 84 | 5 |
| Credit for other | 102 | 98 | 4 |
| **(4) Check the box if qualifies for** | **69** | 84–102 | **15** |

The last row is the one that reads wrong on paper: the `(4)` group heading begins at column 69, which
is inside the **(3) Relationship** data field (62–82), so on the printed page it appears to label the
relationship column rather than the two checkbox columns it belongs to.

Second leg: `{:<34}` is a *minimum* width, so a name longer than 34 characters shifts every
subsequent column right. That is not exotic — `"Christopher Papadopoulos-Wintergreen"` is 36. Rendered:

```
  5  Maximiliana Wolfeschlegelsteinhausenbergerdorff-Fitzgerald 111-22-000N           Granddaughter-in-law  [  ]          [  ]
```

No data is lost or truncated (unlike a PDF comb cell), and a human can still parse it, which is why
this is Minor and not Important. But it is a page that goes to the IRS.

### M-2 — the test's doc comment still describes the behavior the same commit deleted

```
SEVERITY: Minor
WHERE:    crates/btctax-forms/tests/full_return_forms.rs:1988-1991
```

`3a4d06a` renamed `more_dependents_than_the_form_holds_fails_closed` →
`more_than_four_dependents_checks_the_box_and_prints_the_first_four` and inverted every assertion, but
left the doc comment above it untouched:

> *"More dependents than the form physically holds **REFUSES** rather than printing the first four.
> The IRS's own remedy is a continuation statement, **which is a synthetic page generator v1 does not
> have** … Printing four of five would silently file a return that misstates the household."*

Every clause is now false, and it sits directly above the test that proves it false. This repo's own
B3 evidence is that a defect survives when no reader holds two things at once; a comment asserting
the opposite of its test is the cheapest possible version of that.

### M-3 — `SPEC_form_questions.md` still records the refusal, with a dead line citation

```
SEVERITY: Minor
WHERE:    design/SPEC_form_questions.md:191
```

> `| 1040 `more_than_four_dependents` | — | >4 dependents **refuses** (`form1040_full.rs:366`) — the box is never needed |`

Both halves are now wrong: the box is written (`form1040_full.rs:519`) and `form1040_full.rs:366` is
no longer that code. Not test-enforced (`cite_check.rs`'s doc list does not include this file), so it
is documentation drift only — but `CLAUDE.md` requires citations verified against current source.

### N-1 — a stale `dependents_statement.txt` survives a re-export into the same directory

`mkdir_out` → `fsperms::mkdir_owner_only` is `DirBuilder::recursive(true)` (`fsperms.rs:73-80`), which
succeeds on an existing directory; only `mkdir_owner_only_exclusive` fails-closed, and this path does
not use it. So: export TY2024 with 9 dependents, remove five, re-export to the same `--out`. The PDFs
are overwritten by name, `manifest.txt` is regenerated and correctly no longer mentions the statement
— but `dependents_statement.txt` remains on disk, asserting *"This return claims 9 dependents"* and
*"The box under Dependents on page 1 is checked"* beside a fresh 1040 whose box is unchecked.

Pre-existing class (`write_form_8275_txt` "writes nothing for a no-promoted-leg year" has the same
shape), and the manifest is the filer's authority, which is why this is a Nit. It is newly the case
that a member of that class is a **page the filer detaches and mails**.

### N-2 — "verbatim" overstates the heading transcription

`dependents_statement.rs:96-99` says the headings are *"verbatim from
`design/forms/extract/f1040--2024.txt:40-41`"*. Two departures against the extracted text:

- extract line 40 reads *"(4) Check the box if qualifies for **(see instructions):**"*; the statement
  drops the parenthetical.
- extract line 41 carries *"Child tax credit"* and *"Credit for other dependents"* each on one line;
  the statement re-wraps them as `Child tax` / `credit` and `Credit for other` / `dependents`.

Neither is an assertion and the re-wrap is a sensible fit to a two-line text header — but the word
"verbatim" is the kind of claim this repo checks mechanically, so either soften it or restore the
parenthetical.

### N-3 — `make check` reds on a clean clone

`the_write_hook_denies_new_archives_and_asks_once_per_new_directory`
(`crates/xtask/src/harness_check.rs:531-598`) shells out to `scripts/hooks/on-write.sh`, which
**fails closed with exit 2** when `target/{debug,release}/xtask` is missing (`on-write.sh:41-42`,
`59-69`) — deliberately, and rightly. But `cargo nextest` builds test harnesses, not `--bin xtask`,
so the test asserts `Some(0)` and gets `Some(2)`. Reproduced here on first run; fixed by
`cargo build -p xtask --bin xtask`. Outside the B2 diff — reported because it makes the gate
order-dependent, and because the kill-test for A3/A4 is the harness's own instrument.

---

## ALSO CHECKED, SOUND

- **The partition.** `dependents_split` is `split_at(len.min(4))`, so grid ++ overflow reproduces the
  input in order, disjoint, **by construction** rather than by property. Pinned for n ∈ 0..=12 at
  three levels: core (`the_split_is_an_exact_ordered_partition`), the emitted PDF
  (`the_checkbox_and_the_statement_are_the_same_decision`, which also asserts
  `printed == n.min(4)`), and the statement (`st.rows[0].name == "Child 4"` at n=9). I tried to find
  a size or shape that breaks it and could not — `split_at` cannot produce one.
- **Capture order.** Nothing sorts anywhere on the path. `ReturnHeader::build` is
  `.iter().map().collect::<Result<Vec<_>,_>>()` (`packet.rs:418-429`), order-preserving.
  `ReturnInputs.dependents` is a `Vec<Dependent>` persisted as a JSON array. I grepped every
  `dependents` reference in `crates/*/src`: the only other consumers are `classifier.rs:296` (a
  read-only `for d in dependents`), `advisories.rs:670` (`.len()`), and `tax.rs:155` (SSN masking for
  display). **No `BTreeMap`, no `sort`, no `HashMap` on this path.**
- **Identity on a detached page.** Name line + the taxpayer's SSN is the IRS's own attachment
  convention, and on MFJ `name_line` is the joint line while the SSN is the first-listed taxpayer's
  — which is what "Your social security number" means on every form in the packet. Printing it in
  full is correct: the page must be attributable, and `Ssn`'s masked `Debug` is about logs, not
  filed pages. `hyphenated()` is called explicitly at the two call sites rather than through a
  `Display` impl, which keeps it greppable. (Page 1 prints bare digits because its cells declare
  `/MaxLen 9` — `cells.rs:133-144`; the statement hyphenates because it is typed text. Both are the
  conventional rendering for their medium.)
- **The capacity guard.** `form1040_full.rs:507-516` compares `cells.dependent_rows.len()` against
  `DEPENDENTS_GRID_ROWS` and returns `Geometry` **before any cell is written** — I confirmed it sits
  above the first `text(w, p, &row.name, …)` at line 521. It is reachable and pinned by
  `a_map_that_declares_a_different_dependent_capacity_fails_closed`, which pops a row from a real
  `Form1040Map::ty2024()` and asserts the message names both sides. I could not construct a
  disagreement it misses: `dependent_rows` is a `Vec` and its length is the only capacity the fill
  loop consumes (`zip(&cells.dependent_rows)`), so length equality is exactly the right invariant.
- **`watermarked` is the same predicate the PDFs use** — literally the same local binding
  (`admin.rs:865`), read at `:929` for the PDFs and `:965` for the statement. One decision, not two.
  (What is missing is the *test*, I-1 — not a second predicate.)
- **The type change leaks nothing.** Every `fill_full_return` caller enumerated: `admin.rs:914`
  (consumes `.statements`), `oracle-harness/main.rs:765` (`.forms`, deliberate and documented — a
  statement carries no AcroForm for `read_back_lines`), and the test files
  (`census.rs`, `field_census_slice.rs`, `full_return_forms.rs`, `common/mod.rs:179` which threads
  `statements` through). The TUI, the defensive wizard and the crypto-slice path do not call it at
  all — the slice emits no 1040, so there is no box to leave unattached.
- **All-or-nothing still holds.** `fill_full_return` runs entirely before `mkdir_out`
  (`admin.rs:914` vs `:916`), so a refusal anywhere still writes zero bytes, statement included.
- **The `[  ]` rendering is the correct kind of blank.** Page 1's `ctc`/`odc` are mapped and
  deliberately never written; the statement prints the same two empty positions. A hardcoded "no"
  and a genuine blank are distinguishable here, in the direction the doctrine requires.

## WHAT WOULD MAKE THIS REVIEW WRONG

1. **If the IRS in practice expects the continuation statement to restate all dependents.** I argued
   supplement from the Step-1 sentence and the double-count risk, but I adjudicated a *paper*
   instruction, and I did not consult the MeF schema or a filed-return example. If a preparer or an
   IRS processing convention says otherwise, item 7 above flips and the split becomes wrong. This is
   the one place I reasoned rather than executed.
2. **If checkbox appearance streams do not render on print.** Every claim that "the box is checked"
   is verified at the AcroForm *value* level (`checkbox_on(&doc, id) == Some("1")`). If the emitted
   PDF's `/AS` or `NeedAppearances` handling meant a set checkbox printed blank, sentence 2b would be
   false on paper while every test stayed green. That is a whole-packet property, not a B2 one, and I
   did not test it — but it is the assumption sentence 2b rests on.
3. **If M-1's alignment is worse than I judged.** I measured columns and concluded "legible, so
   Minor." A reviewer who has watched an IRS attachment be transcribed by hand may rate the (4)
   heading sitting over the (3) column as Important. I would not argue hard.
4. **If `pseudo_active()` becomes year-scoped.** I-1's constructive proof depends on it being a
   whole-ledger count. If that changes, my test sketch stops working — though the finding (the call
   site is untested) survives regardless, since mutation 1 is independent of how the test is written.
