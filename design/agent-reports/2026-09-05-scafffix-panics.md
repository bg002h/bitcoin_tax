# scafffix — the two new geometry panics now cite evidence that exists

**Agent:** scafffix-panics · **Date:** 2026-09-05
**Files owned and touched:** `crates/btctax-forms/src/form1040.rs`, `crates/btctax-forms/src/schedule_se.rs`
**Scope kept:** the panics themselves are UNCHANGED in behaviour. Only their justification, their
panic *message text*, and four new tests.

---

## 1. What was wrong — measured

Both `f1040_clusters` and `se_clusters` justified their new enumerated-with-panic form with the same
sentence, in the comment and again inside the panic string:

> "the 2026 drafts move 31 line bindings while renaming nothing"

Two independent problems with it.

**(a) The 31 is not reproducible.** It came from a page off-by-one caused by the TY2026 drafts' IRS
`Caution: DRAFT—NOT FOR FILING` cover sheet. Re-measured against the corrected fixtures with the
prebuilt `./target/debug/xtask form-delta`:

| pair | added | removed | label-comparable | **moved** |
|---|---|---|---|---|
| `f1040--2025` → `f1040--2026-DRAFT` | 0 | 0 | 112 | **0** |
| `f1040sse--2025` → `f1040sse--2026-DRAFT` | 2 | 2 | 22 | **0** |
| `f6251--2025` → `f6251--2026-DRAFT` | 0 | 0 | 60 | **1** (`f1_5`: line 2a → 2) |
| `f6251--2024` → `f6251--2025` (calibration, both sides final) | 1 | 0 | 59 | **30** |

The "label-comparable" column is the count of common fields whose printed label resolves to
something other than `?` on BOTH sides — i.e. the number of observations the verdict actually rests
on. I computed it by replicating `form_delta::compute`'s comparison rule over
`xtask label-boxes <stem>` output for each side (`boxes_tsv` and `label_join` are byte-identical
joins; verified by reading both at `label_reader.rs:523-538` and `:909-925`). The `moved` column
matches what `form-delta` prints, so the replication is checked, not assumed.

**(b) The Form 1040 half of the claim cannot be measured at all.** `design/forms/2026/f1040--2026-DRAFT.pdf`
is **the TY2025 Form 1040**. Verified independently of the recon report, from the committed text
layer `design/forms/2026/f1040--2026-DRAFT.pdf.txt`:

```
line 120: ... Cat. No. 11320B    Form 1040 (2025) Created 9/5/25
line 122: Form 1040 (2025)                          Page 2
```

(For contrast, `f6251--2026-DRAFT.pdf.txt:113` reads `Form 6251 (2026) Created 5/26/26`, and
`f1040sse--2026-DRAFT.pdf.txt:85` reads `Schedule SE (Form 1040) 2026 Created 4/27/26` — those two
drafts are genuinely TY2026.)

So the `0 moved` that `form-delta f1040--2025 f1040--2026-DRAFT` reports is **TY2025 compared with
itself**, and there is no TY2026 Form 1040 measurement in this repo. Per the brief I said so in the
comment rather than citing a number I cannot stand behind.

**(c) A third problem the brief did not name.** The `se_clusters` comment cited *Form 1040*'s delta
as the justification for *Schedule SE*'s bands. Even had the 31 been real, it was evidence about a
different form.

---

## 2. What I changed

### 2.1 The evidence is now a cross-revision measurement that needs no draft

The panics no longer rest on the 2026 drafts at all. They rest on the one revision pair this repo
holds both sides of — TY2017 vs TY2024/2025 — where the cost of reusing a band is directly
observable. Centre-x from `xtask dump-fields` on the bundled blank PDFs, membership by the same
`verify::in_band` the read-back oracle uses (`verify.rs:136`, and `verify_flat` at `:381-390` tests
exactly this on the **dollars** field; `cells.rs:91-95` deliberately leaves the cents widget
column-unchecked, which is *why* the band must exclude it):

**Form 1040 (`crates/btctax-forms/forms/2017/f1040.pdf`, map line 13 = `line7a`):**

| field | cx | `F1040_CLUSTERS_2017` [482,555] | `F1040_CLUSTERS_UNIFIED` [504,576] |
|---|---|---|---|
| line-13 dollars `f1-_51[0]` | 518.1 | IN | IN |
| line-13 cents `f1_52[0]` | 565.2 | **NOT IN** | **IN ★** |

**Schedule SE (`crates/btctax-forms/forms/2017/schedule_se.pdf`, §B long form):**

| field | cx | `SE_CLUSTERS_2017` | `SE_CLUSTERS_UNIFIED` |
|---|---|---|---|
| line 8a dollars `f2_25[0]` (MID) | 392.4 | IN [350,433] | **NOT IN** [410,482] ★ |
| line 8a cents `f2_26[0]` (MID) | 442.8 | **NOT IN** [350,433] | **IN** [410,482] ★ |
| line 2 dollars `f2_7[0]` (AMOUNT) | 514.8 | IN [476,554] | IN [504,576] |
| line 2 cents `f2_8[0]` (AMOUNT) | 566.2 | **NOT IN** [476,554] | **IN** [504,576] ★ |

Read plainly: **run 2024/2025's bands over the 2017 form and every cents widget passes as its
dollars cell.** A wildcard there does not merely skip a check, it disarms the dollars↔cents guard
the band exists to be. Both comments state honestly that TY2017 has its own arm, so this is the
available cross-revision measurement rather than a live path.

Retained as the secondary point, because it is the sharper answer to *"the names all still resolve,
why look?"*: `form-delta f6251--2024 f6251--2025` — **0 renamed**, **30 of 59** label-comparable
fields beside a different printed line.

Both comments carry an explicit **"Retracted — do not re-cite"** paragraph naming the 31, its cause,
and the corrected figures, so a reader who meets the old number elsewhere can dispose of it.

### 2.2 The panic messages

Rewritten to name the operation that closes them (`xtask dump-fields` — measure the year's columns
off its blank PDF) and to carry a concrete number instead of the retracted one:

* `f1040_clusters`: *"…the unified band [504,576] admits the TY2017 cents widget at cx 565.2, so a
  wildcard turns the dollars/cents guard off."*
* `se_clusters`: *"…the unified bands admit BOTH of the TY2017 form's cents widgets (cx 442.8 and
  566.2)…"*

### 2.3 Four tests — the panics had none

`grep -rn 'f1040_clusters\|se_clusters\|no geometry recorded' crates/ --include=*.rs` returned
**nothing outside the two definitions**. Neither panic was reachable from any test, and neither file
had a `#[cfg(test)]` block at all. Both files now do.

Per file:

* **`geometry_is_recorded_for_exactly_the_supported_years`** — sweeps years 2010..=2040 and asserts
  `resolves(y) == crate::SUPPORTED_YEARS.contains(&y)`. The probe is a *domain*, never an expected
  set: the expected answer is derived from `lib.rs:68`. It therefore reds in **both** directions —
  a band handed to an unsupported year (the wildcard is back), *and* a year wired into
  `SUPPORTED_YEARS` with no band measured for it.
* **`the_2017_band_excludes_a_cents_widget_the_unified_band_admits`** /
  **`the_2017_bands_exclude_the_cents_widgets_the_unified_bands_admit`** — loads the bundled blank
  TY2017 PDF and the committed TY2017 map, resolves the dollars/cents FQNs from the map, and
  re-derives every cx in the comment. Nothing in these tests is hand-typed but the year. The third
  assertion is a tripwire on the comment itself: if the unified band ever stops admitting the 2017
  cents widget, the test says *"the premise of the panic has changed … re-measure before rewriting
  the comment above."*

Checks run (scoped, per the brief — no `make check`, no workspace run):

```
cargo nextest run -p btctax-forms --lib -E 'test(cluster_year_guard)'   4 passed, 16 skipped
rustfmt --edition 2021 --check <both files>                            clean
cargo clippy -p btctax-forms --lib --tests (own target dir)             no warning in either file
```

---

## 3. Planted defects → which test went red (B1, all five observed)

Every plant was applied, run, and reverted from a `cp` backup — no `git` was used.

| # | plant | test that RED | message observed |
|---|---|---|---|
| 1 | `form1040.rs`: restore `_ => F1040_CLUSTERS_UNIFIED` | `form1040::…::geometry_is_recorded_for_exactly_the_supported_years` | `TY2010: SUPPORTED_YEARS.contains = false but f1040_clusters answered — an unsupported year must NOT inherit another revision's x-band; the wildcard is back` |
| 2 | `form1040.rs`: `F1040_CLUSTERS_2017 = &[(504.0, 576.0)]` | `form1040::…::the_2017_band_excludes_a_cents_widget_the_unified_band_admits` | `TY2017 cents widget cx 565.2 is INSIDE the band recorded for TY2017 (504.0, 576.0) — a map that swapped dollars and cents would pass the column check` |
| 3 | `schedule_se.rs`: restore `_ => SE_CLUSTERS_UNIFIED` | `schedule_se::…::geometry_is_recorded_for_exactly_the_supported_years` | same shape as #1, `se_clusters` |
| 4 | `schedule_se.rs`: `SE_CLUSTERS_2017 = SE_CLUSTERS_UNIFIED` (both columns) | `schedule_se::…::the_2017_bands_exclude_the_cents_widgets_the_unified_bands_admit` | `TY2017 line-8a dollars cx 392.4 is outside its own band (410.0, 482.0)` |
| 5 | `schedule_se.rs`: widen only the AMOUNT band → `&[(350.0, 433.0), (504.0, 576.0)]` | same test | `TY2017 line-2 cents widget cx 566.2 is INSIDE the band recorded for TY2017 (504.0, 576.0) …` |

Plant 5 exists because plant 4 red on a *different* assertion than I had predicted in the test's own
doc comment (the unified MID band does not even contain the 2017 dollars field, so it fails before
reaching the cents case). Rather than leave the cents assertion unwitnessed, I planted the narrower
defect that exercises it — and **corrected the doc comment to describe what actually happens**,
because a planted-defect note that names the wrong assertion is the same class of defect as the 31.

---

## 4. Found, NOT fixed — not my files

1. **The third geometry wildcard is still open.** `crates/btctax-forms/src/form8283.rs:70-77`
   (re-read at 2026-09-05, still present verbatim):
   ```rust
   (_, Form8283Section::A) => SEC_A_CLUSTERS_2023,
   (_, Form8283Section::B) => SEC_B_CLUSTERS_2023,
   ```
   Its refusal is *borrowed* from `Form8283Map::for_year` / `f8283_pdf` — exactly the property that
   made these two safe until they were not. My `geometry_is_recorded_for_exactly_the_supported_years`
   is directly transplantable to it (`sec_clusters` takes a second argument; sweep both sections).
2. **`design/forms/2026/f1040--2026-DRAFT.pdf` is the TY2025 Form 1040** — confirmed independently
   (§1(b)). Anything downstream that treats it as a TY2026 authority is measuring TY2025 against
   itself. The archiver agent owns this.
3. **Schedule SE's TY2026 draft renames two fields**, both keeping their printed label:
   `topmostSubform[0].Page1[0].Line5a_ReadOrder[0].f1_10[0]` → `…Page1[0].f1_10[0]` (line 5a) and
   `…Line8a_ReadOrder[0].f1_14[0]` → `…Page1[0].f1_14[0]` (line 8a). A TY2026 SE map copied from
   TY2025 fails loudly on these two — good — but they must be re-pinned when the year is wired.
4. **Schedule SE line 7's wage base moves $176,100 → $184,500** (`f1040sse--2025.txt:37` vs
   `f1040sse--2026-DRAFT.pdf.txt:70`). Threaded as `ss_wage_base`, so this is a constant to add, not
   a code change — but it means Schedule SE's *geometry* holding for TY2026 is not a reason to skip
   reading the form. The comment says so.
5. **★ Deliberate tripwire the controller should know about.** Both new
   `geometry_is_recorded_for_exactly_the_supported_years` tests are keyed to
   `crates/btctax-forms/src/lib.rs:68 SUPPORTED_YEARS`. If another agent adds `2026` there without
   recording cluster bands, **both tests red on purpose**, with the message *"a supported year with
   no band recorded: measure the amount column off the year's blank PDF (xtask dump-fields) and add
   the arm."* That is the intended behaviour — it is one of the "gate that does not skip the new
   year" holes the recon asked for — but it will surface as a red in someone else's lane, so it
   should not be mistaken for a regression.
6. **Not investigated:** `crates/xtask/src/form_delta.rs` and `label_reader.rs` were being actively
   rewritten by other agents while I measured. I re-read `form_delta::label_axis` afterwards and
   confirmed the comparison rule is unchanged (`compared` = both labels non-`?`; `moved` = of those,
   differing), so the figures above survive that rework. I deliberately cite **numbers** rather than
   quoting the tool's output strings, since its wording is in flux.
