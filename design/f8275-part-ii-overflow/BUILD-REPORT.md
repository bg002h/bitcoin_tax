# Build report — Form 8275 Part II narrative overflow fix

Branch `fix/f8275-part-ii-overflow`. Commits (bisectable, 3 logical steps):

1. `5df7d3f` test(8275): red — Part II narrative silently clips past ~one line
2. `44e4892` feat(8275): map the 32 unmapped Part II/IV continuation lines
3. `b63e31f` fix(8275): wrap Part II narrative across continuation lines, fail closed

## Step 1 — the red, verbatim

Command: `cargo test -p btctax-forms --test sp4 form_8275_part_ii_long_narrative_does_not_silently_clip -- --nocapture`

```
   Compiling btctax-forms v0.10.0 (/scratch/code/bitcoin_tax/crates/btctax-forms)
    Finished `test` profile [optimized + debuginfo] target(s) in 0.41s
     Running tests/sp4.rs (target/debug/deps/sp4-e56f9700d58b0932)

running 1 test

thread 'form_8275_part_ii_long_narrative_does_not_silently_clip' (1149925) panicked at crates/btctax-forms/tests/sp4.rs:458:9:
topmostSubform[0].Page1[0].p1-t80[0]: its written content measures 6785.9pt wide at 8pt Helvetica-Bold but the field's own widget box is only 518.4pt wide — a PDF viewer honoring this widget's geometry (DoNotScroll, non-multiline) would silently clip roughly 6267.5pt of text (the field holds 1762 characters): "The taxpayer disposed of Bitcoin that was originally acquired over several transactions spanning approximately three years, during which the taxpayer used a combination of a hosted exchange account that has since ceased operations, a small number of in-person cash purchases from a now-unreachable counterparty, and at least one peer-to-peer transaction conducted through a messaging application whose records were not retained. The exchange that held the earliest lots suspended withdrawals and subsequently entered insolvency proceedings; repeated requests to its claims administrator for historical trade confirmations and cost-basis statements went unanswered, and the taxpayer has been unable to obtain contemporaneous documentation of the exact purchase prices paid for those lots despite good-faith efforts including searching personal email archives, bank and credit-card statements covering the relevant period, and any cached web pages of the exchange's now-defunct account dashboard. Because a substantial and unrecoverable portion of the original acquisition records is unavailable through no fault of the taxpayer, basis for the disposed lots was estimated using the daily low closing price over the taxpayer's best-documented estimate of the acquisition window, consistent with the Cohan doctrine, and the estimate was limited so as never to report a loss that a complete record might not support. The taxpayer maintains that this approach is a reasonable, conservative substitute for records that cannot be reconstructed, and discloses it here in the interest of full transparency with respect to the estimated basis reported on the attached Form 8949, so that the position is examined on its merits rather than treated as an undisclosed estimate."
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test form_8275_part_ii_long_narrative_does_not_silently_clip ... FAILED

failures:

failures:
    form_8275_part_ii_long_narrative_does_not_silently_clip

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.04s

error: test failed, to rerun pass `-p btctax-forms --test sp4`
```

**Why the test needed its own width measurement, not just a `/V` presence check.** `p1-t80[0]` carries
no `/MaxLen` (confirmed by dumping `collect_fields` output), so the pre-fix `push_free` write
**never truncates the stored string** — `/V` genuinely holds the entire narrative, unclipped, at the
data level. The defect is purely in **rendering**: the widget is a fixed-width (518.4pt), non-multiline,
`DoNotScroll` box, so a PDF viewer honoring its own geometry only *displays* what fits (~137 characters
at 8pt). A test that merely reads `/V` back cannot fail pre-fix — the whole string is there. The red
test above instead independently re-measures the written content's rendered width (an AFM-derived
Helvetica-Bold 8pt table, kept deliberately separate from `crate::wrap`'s own table — the same
"map/fix is what we distrust, the PDF's own geometry is the oracle" posture `verify.rs` already uses)
against the field's own `/Rect` width pulled fresh from the bundled PDF.

## Step 2 — field names actually found

Confirmed via `pdf::collect_fields` on the bundled `forms/2024/f8275.pdf` (95 total leaf fields):

| Field | Page | Rect (x0,y0,x1,y1) | `/DA` | `/Ff` | `/MaxLen` |
|---|---|---|---|---|---|
| `p1-t80[0]` (existing `part_ii_narrative`) | 1 | `[57.6, 408.0, 576.0, 420.0]` | `/HelveticaLTStd-Bold 8.00 Tf` | `8388608` (DoNotScroll) | none |
| `p1-t81[0]` … `p1-t85[0]` | 1 | width 518.4pt each, 12pt tall, stacked below `p1-t80` in descending y | same | same | none |
| `p2-t1[0]` … `p2-t27[0]` | 2 | width 540.0pt each, 24pt tall, stacked in descending y (numeric order = printed top-to-bottom order) | same | same | none |

All 32 new fields were **verified present** and **previously unmapped** (0 hits in the pre-fix
`f8275.map.toml`, matching the design note's claim). `pdftotext -layout` on the bundled PDF confirms the
printed structure: Part II "Detailed Explanation" is numbered lines 1–6 (`p1-t80`..`p1-t85`), and page 2
Part IV "Explanations (continued from Parts I and/or II)" is the 27-line block (`p2-t1`..`p2-t27`) — no
line numbers are printed there, just 27 blank ruled lines.

**One correction to the design note's premise:** the note says "measure Helvetica 8pt." The PDF's own
`/DA` on every one of these fields actually declares **`/HelveticaLTStd-Bold`** (bold weight), not plain
Helvetica — confirmed by dumping `/DA` directly. This does not change scope or block the fix (it only
changes which AFM width table is correct to use), so I did not stop; `wrap.rs`'s width table is Adobe's
published Helvetica-**Bold** Core-14 AFM metrics, matching what the PDF itself declares. Also, per-field
`/Ff` is `8388608` (bit 24, `DoNotScroll`) — **not** the Multiline bit (`4096`) — so these are genuinely
single-line fields despite the Part IV fields' 24pt-tall boxes (visual padding, not multi-line text).

A second, minor correction: the design note states "there is no Part II string over 200 characters
anywhere in the test suite (verified: 0)." `sp4.rs`'s `sample_printed()` fixture is actually 201
characters — one over that threshold. This doesn't change any conclusion (201 chars still silently
clips past ~137, and still doesn't fit one 518.4pt line at 8pt Bold — it needed the wrap fix same as
any longer string), so it's noted here rather than treated as a blocking discrepancy.

## Step 3 — capacity

- **33 continuation fields total**: `part_ii_narrative` (1) + `part_ii_continuation` (5) +
  `part_iv_continuation` (27).
- Wrapping uses the **narrowest** of the 33 fields' own widget widths, applied uniformly to every line
  (Part II's own lines are 518.4pt wide; Part IV's are wider still at 540pt) — so a line that fits
  physically fits wherever it lands, with no per-field bookkeeping needed.
- At 8pt Helvetica-Bold, a 518.4pt line holds roughly **110–140 characters** depending on word lengths
  (measured on the >1500-char fixture: 14 lines produced, 121–139 chars/line, mean ≈ 125 chars/line).
- Total practical capacity across all 33 lines is therefore in the neighborhood of **3,700–4,100
  characters** of ordinary English prose — comfortably over the >1500-character fixture this task
  required, with real headroom for a genuinely long "lost records" narrative.
- The dedicated fail-closed test (`form_8275_part_ii_narrative_too_long_for_every_continuation_line_fails_closed`)
  uses 900 repeats of `"word "` (4,500 characters of 4-character tokens — an adversarial shape that
  wraps far less densely than prose), which needs 38 lines against the 33 available, and correctly
  returns `FormsError::Overflow { part: "Part II", rows: 38, capacity: 33 }`.

## Mutation test

Per the build brief: reverted `fill_form_8275_inner`'s Part II section (in `crates/btctax-forms/src/form8275.rs`)
from the wrap back to the shipped single write:

```rust
// ★ MUTATION (T-f8275-part-ii-overflow build brief): revert the wrap — write the whole narrative
// to the single field again, as shipped in v0.9.0/v0.10.0.
push_free(&mut w, &mut p, &map.part_ii_narrative, &printed.part_ii);
```

Applied via direct `Edit`, **not** `git checkout --` (this repo's own recorded lesson: `git checkout --
<file>` has previously eaten a full round of uncommitted work here). Restored afterward via `cp` from a
backup taken before the mutation (`cp /tmp/.../form8275.rs.backup crates/btctax-forms/src/form8275.rs`),
verified byte-identical to the pre-mutation file via `diff` before proceeding.

**Result: the mutation reds, as required.** Running the designated Step-1 coverage test under the
mutation:

```
thread 'form_8275_part_ii_long_narrative_does_not_silently_clip' (1171674) panicked at crates/btctax-forms/tests/sp4.rs:525:9:
topmostSubform[0].Page1[0].p1-t80[0]: its written content measures 6785.9pt wide at 8pt Helvetica-Bold but the field's own widget box is only 518.4pt wide — a PDF viewer honoring this widget's geometry (DoNotScroll, non-multiline) would silently clip roughly 6267.5pt of text (the field holds 1762 characters): "The taxpayer disposed of Bitcoin that was originally acquired..."
test form_8275_part_ii_long_narrative_does_not_silently_clip ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.03s
```

Running the full `sp4.rs` file under the same mutation additionally reds 3 more tests (confirming the
fix is load-bearing across the suite, not narrowly tautological to the one designated test):

```
failures:
    form_8275_fills_part_i_part_ii_and_identity
    form_8275_is_byte_deterministic
    form_8275_part_ii_long_narrative_does_not_silently_clip
    form_8275_part_ii_narrative_too_long_for_every_continuation_line_fails_closed

test result: FAILED. 7 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
```

This claim is **true as recorded** — the mutation was actually run and actually reproduced this exact
output; nothing here was predicted rather than observed.

After restoring the real fix, the full `sp4.rs` suite (11 tests) and the whole-crate `cargo test -p
btctax-forms` suite are green again, and `make check` (nextest + clippy `-D warnings`, whole workspace)
plus `cargo fmt --all -- --check` both pass clean.

## Summary (round 1)

| | |
|---|---|
| Step 1 red | confirmed, verbatim above |
| New fields mapped | 32 (`p1-t81`..`p1-t85`, `p2-t1`..`p2-t27`), verified present + previously 0% mapped |
| Capacity | 33 lines, ~110–140 chars/line at 8pt Helvetica-Bold, ~3,700–4,100 chars total for ordinary prose |
| Overflow behavior | `FormsError::Overflow { part: "Part II", rows, capacity: 33 }` — fails closed, never truncates |
| Mutation | reverted wrap → 4 tests red (verified, not predicted) → restored via `cp`, suite green again |
| Gate | `make check` (2484 tests, clippy `-D warnings`) + `cargo fmt --all -- --check` — both green |

**Citation correction (flagged by the round-2 review):** the "running the full sp4.rs" mutation
transcript above (line 113) cites `sp4.rs:525:9`; the committed tree at review time actually had that
panic at `:529:9` — the transcript was captured against a slightly older working copy than what got
committed. The substance (the mutation reds, verified not predicted) was independently re-verified as
TRUE. Round 2 rewrote `sp4.rs` extensively (see below), so neither line number is current any more
either — this is exactly this repo's own recorded `stale self-citations` lesson recurring, and it is
why round 2's own citations below are deliberately sparse on exact line numbers where the content is
going to keep moving.

---

# Round 2 — whole-branch two-lens review fix pass

Seven findings (2 Critical, 4 Important, 1 Minor) came back from a parallel two-lens review of the
round-1 commits. All seven were addressed (fixed, or — per the review's own instruction — filed as a
follow-up and NOT built). Commits below round 1's `b63e31f`.

## Findings → disposition

1. **[Critical] Part IV spill had no cross-reference.** Fixed: the first Part IV line used now always
   starts with `"Part II, line 1 (continued): "` (`wrap::PART_IV_CROSS_REFERENCE_PREFIX`), budgeted into
   the wrap so it never itself causes an overflow. Verified against Rev. 10-2024's own Specific
   Instructions text (quoted in the review) and against the bundled asset's own page-2 caption
   ("Explanations (continued from Parts I **and/or** II)", confirmed via `pdftotext -layout`).

2. **[Critical] The refusal fired after a half-populated packet was on disk.** Fixed: `btctax_forms::
   part_ii_capacity_check(narrative, year)` runs the identical wrap the real fill does, WITHOUT filling
   anything, and both `export_irs_pdf_from_session` (crypto-slice) and `export_full_return` now call it
   BEFORE `mkdir_out`, returning a `CliError::Usage` naming the year, the capacity, an honest character
   count of what would fit, and the real remedy (the narrative is immutable once recorded — the vault is
   append-only — so the remedy is "void and re-record with a shorter `--part-ii-file`," not "just
   shorten it," which the review's own framing anticipated). **Empirically re-verified the ORIGINAL
   exposure** (see Mutation B below): with the pre-flight removed, `out_dir` ends up holding
   `f8949.pdf`, `schedule_d.pdf`, `form_1040_capgains.pdf`, `form_8275.txt`, and `basis_methodology.txt`
   — an estimated-basis Form 8949 with NO `form_8275.pdf` disclosure behind it, exactly the §6662(d)
   exposure the review named.

3. **[Important] The line budget ignored the renderer's text inset.** Fixed: `form8275.rs::
   TEXT_INSET_PTS = 2.0` (per side), `usable_width()` subtracts `2 * TEXT_INSET_PTS` from every field's
   raw `/Rect` width before the wrap ever sees it — `518.4pt → 514.4pt` usable for Part II, `540.0pt →
   536.0pt` for Part IV. This reproduces the review's own pdf.js measurement (≈514.4pt usable on
   Part II's line) almost exactly, and sits comfortably under poppler's more generous ~516.24pt too. The
   test oracle (`sp4.rs::assert_fits_inset_box`) is independently re-declared (own `ORACLE_TEXT_INSET_PTS`
   constant, not `use`d from `src/`) with a NEGATIVE allowance (`usable - 0.05`, not `+0.5`) per the
   review's exact instruction.

4. **[Important] Part II's numbered lines 2-6 must never be claimed by a combined narrative.**
   Independently re-verified by decompressing the bundled PDF's own XFA `template` packet (`qpdf`
   couldn't reach it — the array/stream indirection needed a small `lopdf`-based dump; see "XFA
   verification" below): `Line1PartII`..`Line6PartII` draw elements print `"1 "`..`"6 "` beside
   `p1-t80[0]`..`p1-t85[0]` respectively, confirming the review's claim exactly. Took the review's ruling
   as specified: Part II writes ONLY its own line 1; everything past that goes to Part IV. Per-item
   numbering is filed as a follow-up (`FOLLOWUPS.md`), not built.

5. **[Important] Paragraph breaks were destroyed.** Fixed: `wrap::split_paragraphs` splits on blank-line
   boundaries (mirroring `disclosure_8275`'s `"\n\n"` join of multiple tranches' narratives);
   `wrap_paragraphs` treats a paragraph boundary as a HARD break (flushes the current line even when
   there is room to share it). Exercised through the FULL fill (not just a `wrap.rs` unit test) by
   `sp4.rs::form_8275_paragraph_breaks_are_hard_breaks_not_collapsed_to_a_space`.

6. **[Important] Part IV's physical order was assumed, not enforced.** Fixed: `verify::FlatPlacement::
   free_ordered` adds a descent-group entry for a free (non-column) placement; `form8275.rs::
   push_free_ordered` uses it for every Part IV write (`PART_IV_GROUP`, ordinal = array index). A
   fault-injected map that swaps `part_iv_continuation[0]` and `[2]` now fails closed with a `Geometry`
   error naming the broken descent (`sp4.rs::fault_injected_8275_part_iv_reordered_fields_breaks_
   descent_and_is_red`) — confirmed via Mutation C below that WITHOUT this wiring the same fault
   injection silently "succeeds" (wrong ordering, no error at all).

7. **[Minor, folded] Glyph width for `` ` `` (0x60) under-measured.** Fixed by going further than asked:
   rather than hand-patch one entry, `wrap.rs`'s WHOLE ASCII width table was re-extracted directly from
   the bundled PDF's own embedded font (`/DR/Font/HelveticaLTStd-Bold`, `/Type1`, `/WinAnsiEncoding`,
   `/FirstChar 0`/`/LastChar 255`, full 256-entry `/Widths` — see "Authoritative font widths" below).
   Confirmed exactly the review's two claims and nothing else differs: `0x27` (apostrophe) is 238 here
   vs the generic AFM's 278 (safe, over-measuring direction); `0x60` (backtick) is 333 here vs the
   generic AFM's 278 (was the dangerous, under-measuring direction — now fixed). All the "extra" smart-
   punctuation widths already in the table (em/en dash, curly quotes, bullet, ellipsis, section sign)
   were independently re-checked against the SAME authoritative source and all already matched exactly.

**Follow-ups (not built, filed in `design/f8275-part-ii-overflow/FOLLOWUPS.md`):** per-item Part II
numbering (the fuller shape of finding 4); a record-time narrative-length bound in `plan_promote`; the
unbreakable-token overflow path's inflated (not exact) row count; a defensive Part II emptiness re-check
at the fill layer; documenting the ~28-line ceiling in `cli.rs`/the man page; `Form8275Map::
field_names()` delegating to `narrative_continuation_fields()`.

## XFA verification (finding 4)

`f8275.pdf`'s `/AcroForm/XFA` is a `[name, stream]*` array; the `template` packet (object stream,
FlateDecode) was pulled directly via a one-off `lopdf`-based test (not `qpdf --qdf`, which left the
stream still compressed even with `--stream-data=uncompress` for this particular array-of-streams
shape) and searched for `Line\dPartII`:

```
2033:><draw name="Line1PartII" w="5.08mm" x="12.7mm" y="131.233mm" h="4.233mm"
2034:><value><text>1 </text></value>
...              <traversal><traverse ref="p1-t80[0]"/></traversal>
2134:><draw name="Line2PartII" ...>2 </text>...<traverse ref="p1-t81[0]"/>
2235:><draw name="Line3PartII" ...>3 </text>...<traverse ref="p1-t82[0]"/>
2336:><draw name="Line4PartII" ...>4 </text>...<traverse ref="p1-t83[0]"/>
2437:><draw name="Line5PartII" ...>5 </text>...<traverse ref="p1-t84[0]"/>
2538:><draw name="Line6PartII" ...>6 </text>...<traverse ref="p1-t85[0]"/>
```

Exact match to the review's claim.

## Authoritative font widths (findings 3/7)

Dumped `/DR/Font/HelveticaLTStd-Bold` (object 28) from the bundled PDF directly:

```
BaseFont /HelveticaLTStd-Bold  Encoding /WinAnsiEncoding  FirstChar 0  LastChar 255
Widths [500 500 ... 278 333 474 556 556 889 722 238 333 333 389 584 278 333 278 278 556 ...]
                              ^^^ code 0x27 = 238        code 0x60 (backtick) = 333
MAX WIDTH IN TABLE: 1000
```

`code 0x27 ('): width 238` and `code 0x60 (\`): width 333` — matching the review exactly. Every
"extra" glyph (em dash 0x97=1000, en dash 0x96=556, quoteleft 0x91=278, quoteright 0x92=278,
quotedblleft 0x93=500, quotedblright 0x94=500, bullet 0x95=350, ellipsis 0x85=1000, section 0xA7=556)
was cross-checked against this same array and already matched `wrap.rs`'s existing table exactly — only
the two ASCII entries needed correction.

## New capacity (round 2)

Part II now writes only 1 line; the wrap capacity is `1 + 27 = 28` (was 33 in round 1). At 8pt
Helvetica-Bold with the inset applied: Part II's line 1 usable width is 514.4pt, Part IV's is 536.0pt
(438.2pt on the FIRST Part IV line used, after reserving room for the 97.8pt cross-reference prefix).
The round-1 >1500-char fixture (1762 chars) now needs 14 lines (1 Part II + 13 Part IV) — comfortably
under 28. The dedicated overflow KAT (900 repeats of `"word "`) now needs 37 lines against the 28
available (`FormsError::Overflow { part: "Part II", rows: 37, capacity: 28 }`) — was 38 vs 33 in round 1;
the shift is consistent with the tighter, restructured capacity.

## Mutation tests (round 2)

All four applied via direct `Edit` (never `git checkout --`), each restored via `cp` from a backup taken
before the mutation, each restore verified byte-identical via `diff` before the next mutation and before
proceeding. All four claims below are **true as recorded** — actually run, not predicted.

**Mutation A (re-verify the core fix survived the round-2 restructuring).** Reverted
`fill_form_8275_inner`'s Part II section back to the shipped single `push_free(part_ii_narrative,
printed.part_ii)` (the SAME mutation as round 1, re-applied to the new code). Result: **7 of 15** sp4.rs
tests red (`form_8275_fills_part_i_part_ii_and_identity`, `form_8275_is_byte_deterministic`,
`form_8275_paragraph_breaks_are_hard_breaks_not_collapsed_to_a_space`,
`form_8275_part_ii_long_narrative_does_not_silently_clip`,
`form_8275_part_ii_narrative_too_long_for_every_continuation_line_fails_closed`,
`form_8275_part_iv_cross_reference_appears_only_on_the_first_used_part_iv_line`,
`fault_injected_8275_part_iv_reordered_fields_breaks_descent_and_is_red`) — a stronger, broader red than
round 1's 4, reflecting the larger surface round 2 added.

**Mutation B (finding 2 — the CLI pre-flight).** Removed BOTH `admin.rs` pre-flight blocks (crypto-slice
and full-return). Result: both new CLI tests red —
`export_irs_pdf_with_an_overflowing_part_ii_narrative_refuses_before_bytes` and
`export_full_return_with_an_overflowing_part_ii_narrative_refuses_with_a_named_remedy`, each failing on
the message-content assertion:

```
must name the year, 'Part II', and the --part-ii-file remedy: IRS form fill: 37 rows exceed the 28-row capacity of a single Part II page
```

— i.e. exactly the generic, unhelpful `FormsError::Overflow` Display the review quoted. **Then went
further and empirically confirmed the Critical byte-safety claim itself**, not just the message
regression: with a probe print added temporarily to the test, `out_dir`'s contents after the refusal
under this mutation were:

```
PROBE out_dir contents after refusal: ["form_1040_capgains.pdf", "schedule_d.pdf", "f8949.pdf", "form_8275.txt", "basis_methodology.txt"]
```

An estimated-basis `f8949.pdf` on disk with **no `form_8275.pdf`** behind it — the exact half-populated
packet / §6662(d) exposure finding 2 described. The probe print was removed before restoring the real
fix (it was never meant to be a permanent test — it existed only to make this one empirical check, and
the final committed test asserts the directory is fully empty, which is the correct, general assertion
once the fix is in place).

**Mutation C (finding 6 — descent-order enforcement).** Reverted `push_free_ordered` calls for Part
II/IV back to plain `push_free` (no descent group). Result:
`fault_injected_8275_part_iv_reordered_fields_breaks_descent_and_is_red` reds with:

```
called `Result::unwrap_err()` on an `Ok` value: Some([...pdf bytes...])
```

— i.e. without descent tracking, the fault-injected reordered-map fill **silently succeeds** (produces
a real, verify_flat-passing PDF with the wrapped lines landing in the WRONG physical order), confirming
the geometric guard is load-bearing, not decorative.

**Mutation D (finding 3 — text inset).** Set `TEXT_INSET_PTS = 0.0`. Result:
`form_8275_part_ii_long_narrative_does_not_silently_clip` reds:

```
topmostSubform[0].Page2[0].p2-t6[0]: its written content measures 539.26pt wide at 8pt Helvetica-Bold
but the field's INSET-adjusted usable width is only 536.00pt (540.00pt raw minus 2.0pt inset per side)
```

— a REAL line (not a contrived one) that the un-inset budget would have packed 3.26pt too wide,
confirming the review's own concrete clipping example (its line 5, "…suspended withdrawals and
subsequently entered", was the SAME class of failure against the round-1 fixture) is caught by the
round-2 fix and would NOT be caught without it.

## Final gate (round 2)

After all four mutations restored (each verified byte-identical to its pre-mutation backup via `diff`):
`make check` — **2497 tests run: 2497 passed, 11 skipped**, clippy `-D warnings` clean — and
`cargo fmt --all -- --check` — clean. (2497 vs round 1's 2484: +2 CLI tests for finding 2, +4 sp4.rs
tests for findings 4/1/5/6, +7 wrap.rs unit tests net of the round-1 ones they replaced.)
