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

## Summary

| | |
|---|---|
| Step 1 red | confirmed, verbatim above |
| New fields mapped | 32 (`p1-t81`..`p1-t85`, `p2-t1`..`p2-t27`), verified present + previously 0% mapped |
| Capacity | 33 lines, ~110–140 chars/line at 8pt Helvetica-Bold, ~3,700–4,100 chars total for ordinary prose |
| Overflow behavior | `FormsError::Overflow { part: "Part II", rows, capacity: 33 }` — fails closed, never truncates |
| Mutation | reverted wrap → 4 tests red (verified, not predicted) → restored via `cp`, suite green again |
| Gate | `make check` (2484 tests, clippy `-D warnings`) + `cargo fmt --all -- --check` — both green |
