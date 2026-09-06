# scafffix — `form-delta` reports its own evidence

**Agent:** scafffix-delta · **Date:** 2026-09-05 · **File owned:** `crates/xtask/src/form_delta.rs`

---

## 1. What was wrong (measured)

`form_delta.rs:69-80` (pre-fix) computed the line→label axis like this:

```rust
let labels_available = la.is_some() && lb.is_some();
if let (Some(la), Some(lb)) = (la, lb) {
    for f in &common {
        let (x, y) = (la.get(f), lb.get(f));
        if let (Some(x), Some(y)) = (x, y) {
            if x != y && x != "?" && y != "?" {
                label_moved.insert(f.clone(), (x.clone(), y.clone()));
            }
        }
    }
}
```

Three silent drops, no counter behind any of them:

| drop | cause |
|---|---|
| `la.get(f)` is `None` | the OLD geometry records no box by that name |
| `lb.get(f)` is `None` | the NEW geometry records no box by that name |
| either label is `"?"` | `label_reader`'s **BOX-WITH-NO-LABEL** marker — a *finding*, silently compared away |

`labels_available` then meant only *"a fixture exists on each side"*, and `run` printed
**"line->label: no field changed the printed line it sits beside"** whenever `label_moved` was empty
— which is what "nothing moved" and "nothing was looked at" both look like. The `NoFixture` banner
already said *"an unchecked run is NOT a clean one"* and then **exited 0**.

### Measured scale, on the real artifacts

Over the **13** archived `--2025` → `--2026-DRAFT` pairs that resolve today,
**604 of 664 common fields were actually compared** — 60 fields (9.0%) were folded into verdicts
that said nothing about them. Per-pair (`compared` / `common`):

```
f1040sa   14/14    f1040sb   70/72    f1040sc   39/59    f1040sd   51/55
f1040sse  22/25    f1040s2   42/44    f1040s3   33/35    f6251     60/62
f8949    186/202   f8959     24/26    f8960     33/38    f8995     22/22
f1040s1a   8/10
```

Breakdown of the 60, by reason (computed from the tool's own output, not hand-tallied):
**57 BOX-WITH-NO-LABEL on both sides, 3 BOX-WITH-NO-LABEL on the new side.**

Worst cases: **`f1040sc` compared 39 of 59** (34% of the surface unwitnessed) while printing a
confident "8 fields moved"; **`f8949` printed "no field changed"** on the back of 186 of 202.

### ★ A correction to the brief's premise, stated plainly

The brief cites *"resolved **0 of 26** labels"* for `f8959`, and 0 of 72 for `f1040sb`. **Those
numbers are not reproducible in the tree as I found it.** `f8959--2025 → f8959--2026-DRAFT` resolves
**24 of 26**; `f1040sb` resolves **70 of 72**. `f8995` and `f1040s3` were not false greens at all —
both already printed moves (8 and 3).

The cause is visible in the fixture mtimes: `design/forms/geometry/*--2026-DRAFT.json` were all
regenerated at 19:08 today, i.e. another agent's fix for `TY2026_PORT_REPORT.md` §2.1 **row 1**
(`page_of` deriving a box's page off-by-one on drafts, which had every draft box joining against the
previous page's text) landed before I measured. Row 1's defect was *producing* row 2's zero
comparisons.

**The structural defect in row 2 was real and is fixed. Its blast radius was 60 fields of 664, not
664 of 664** — and, importantly, **nothing in the old code prevented it from being 664 of 664 again**
the next time an upstream fixture pass regresses. That is exactly the property now under test.

---

## 2. What I changed

All in `crates/xtask/src/form_delta.rs`. No other file touched.

1. **`const NO_LABEL: &str = "?"`** — the BOX-WITH-NO-LABEL marker is named, with the reason it can
   never be compared against another one.
2. **`enum Unwitnessed`** — 7 variants naming *why* a field yielded no evidence
   (`GeometryFixtureMissing`, `BoxAbsentBothSides`, `BoxAbsentOld`, `BoxAbsentNew`,
   `UnlabelledBothSides`, `UnlabelledOld`, `UnlabelledNew`), each with a `why()` sentence.
3. **`fn witness(old, new) -> Result<(String, String), Unwitnessed>`** — the per-field decision, with
   **no third answer**, so a caller cannot drop a field by forgetting a branch.
4. **`fn label_axis(common, old, new) -> LabelAxis`** — pure, filesystem-free, returning
   `{ compared, moved, unwitnessed }`. This is what makes the zero-evidence case testable on demand
   instead of only when a real form develops one.
5. **`enum LabelVerdict`** — `NoFixture` / `Unwitnessed` / `Unchanged { compared, unwitnessed }` /
   `Moved { compared, moved, unwitnessed }`. **The two evidence-free outcomes are separate variants**,
   not an empty map. `is_witnessed()` is `compared() > 0`.
6. **`Delta` gains `label_compared: usize` and `label_unwitnessed: BTreeMap<String, Unwitnessed>`**,
   plus `label_verdict()`. `labels_available` and `label_moved` are unchanged, so the three existing
   calibration tests were not edited.
7. **`compute`'s fixture-missing arm marks every common field `GeometryFixtureMissing`** rather than
   returning an empty map — so `compared + unwitnessed == common.len()` holds on the error path too.
8. **`run` prints the evidence and derives its exit status from the verdict**:
   - clean → `line->label: 24 of 26 common field(s) COMPARED; none of them changed the printed line…`
   - moved → `★★ 30 of 59 COMPARED field(s) (61 common) KEPT THEIR NAME but now sit beside…`
   - zero  → `★★ LINE->LABEL DRIFT UNWITNESSED — 0 of N … the absence of evidence, not evidence of absence.`
   - then every unwitnessed field **listed by name**, grouped by reason.
   - `if v.is_witnessed() { Ok(()) } else { Err(...) }` — **one** decision point, so a fifth branch
     cannot exit 0 by forgetting a `return`.
9. **Truncated lists now say they were truncated** (`print_capped` emits `… and N more (not shown)`).
   The old `.take(8)` / `.take(20)` dropped items silently — the same class of defect.

### Behaviour change worth flagging to the controller

`xtask form-delta` now **exits 1** for both evidence-free verdicts. It previously exited **0** while
printing the `NOT CHECKED` banner. Verified end to end on a real artifact: `f8275r--2025` has a PDF
and no geometry, and `form-delta f8275r--2025 f8275r--2025` reports 102 of 102 unwitnessed and exits
1. The only caller is `main.rs:137`; no script, Makefile or CI job invokes `form-delta`
(the only other repo mentions are prose in `schedule_se.rs:53` and `form1040.rs:50`).

---

## 3. Which test reds for which planted defect

Nine tests in `mod tests`; the three pre-existing calibrations are untouched and still pass.
Every plant below was applied to the real file, run, and reverted.

| # | plant (the exact old behaviour, restored) | tests that went RED |
|---|---|---|
| **P1** | delete the `if self.label_compared == 0 { return Unwitnessed }` guard in `label_verdict` | **1** — `zero_comparisons_is_unwitnessed_and_never_a_clean_verdict`, with `left: Unchanged { compared: 0, unwitnessed: 3 }` vs `right: Unwitnessed { unwitnessed: 3 }` — literally the old false green |
| **P2** | in `witness`, drop the `NO_LABEL` guard so two `?`s compare as equal labels | **4** — `zero_comparisons…`, `a_clean_verdict_carries_its_evidence_count_and_names_the_gaps`, `a_move_among_unwitnessed_fields_is_still_a_move`, `every_common_field_is_either_compared_or_named_as_unwitnessed` |
| **P3** | in `label_axis`, replace `Err(u) => { unwitnessed.insert(f, u); }` with `Err(_) => {}` (the original silent skip) | **5** — the four above **plus the real-artifact census** `no_archived_pair_reports_a_clean_verdict_from_zero_comparisons`, which caught it on `f1040s1a--2025 -> f1040s1a--2026-DRAFT: compared 8 + unwitnessed 0 != 10 common` |
| **P4** | make `LabelVerdict::compared()` return 1 for `NoFixture` (so an unrun axis reads as witnessed) | **1** — `a_missing_geometry_fixture_is_unwitnessed_not_clean` |

**No hand-lists.** `no_archived_pair_reports_a_clean_verdict_from_zero_comparisons` enumerates its
subjects by reading `design/forms/geometry/*--2026-DRAFT.json` off the filesystem, asserts the
directory listing is non-empty (a loop over an empty set passes vacuously), and asserts at least one
pair produced a *witnessed* verdict — naming every blind pair in the failure text if none did. Pairs
whose PDF is missing land in that named list rather than gating: fixture coverage is `label_reader`'s
gate, not this one, and P3 shows the census still reds on a real artifact.

`every_common_field_is_either_compared_or_named_as_unwitnessed` asserts the reason *set* observed
equals all six comparable `Unwitnessed` variants, so a variant that stops being reachable reds.

### Gate output

```
cargo nextest run -p xtask -E 'test(form_delta)'   →  9 tests run: 9 passed, 90 skipped
CARGO_TARGET_DIR=target-clippy cargo clippy -p xtask --all-targets --all-features -- -D warnings
                                                  →  Finished, 0 warnings
cargo fmt -p xtask -- --check                     →  0 diffs in form_delta.rs
```

(Only `form_delta.rs` was formatted, via `rustfmt` on the single file — `cargo fmt -p xtask` still
reports diffs in `label_reader.rs`, which belongs to another agent and I did not touch.)

Real-pair spot checks after the fix:

```
f6251--2024 -> f6251--2025   ★★ 30 of 59 COMPARED field(s) (61 common) …   [true positive intact]
f8959--2025 -> f8959--2026-DRAFT   24 of 26 COMPARED; none moved; 2 named (both BOX-WITH-NO-LABEL)
f8275r--2025 -> f8275r--2025       0 of 102 compared → exit 1  [was exit 0]
```

---

## 4. Found and NOT fixed

1. **`design/forms/geometry/f1040--2026-DRAFT.json` is now an orphan.** Mid-run, another agent
   withdrew `design/forms/2026/f1040--2026-DRAFT.pdf` (correctly — `TY2026_PORT_REPORT.md` §2.1
   row 7: it was the TY2025 Form 1040). The geometry fixture for it is still on disk, so
   `form-delta f1040--2025 f1040--2026-DRAFT` now fails with *"no PDF found"*. My census test handles
   this (named, not gated). **Someone owns deciding whether that JSON should go.** Not my file.
2. **`f1040sc` compares only 39 of 59 fields** — the largest remaining blind spot, now visible but
   not explained. 20 of its boxes are BOX-WITH-NO-LABEL on both sides. Some are legitimately
   label-free (name/EIN header, checkboxes); whether all 20 are is a `label_reader` question.
   Same shape, smaller, for `f8949` (16 of 202) and `f8960` (5 of 38).
3. **`label_reader::label_join` still emits `"?"` with nothing gating the count.** `form-delta` now
   names them downstream, but the upstream producer has no assertion of its own — `boxes_tsv`'s doc
   calls `?` "a hard finding, not a blank cell" and nothing enforces that. `TY2026_PORT_REPORT.md`
   §2.1 row 5 is the adjacent open item. Not my file.
4. **The third axis named in `TY2026_PORT_REPORT.md` §2.3 still does not exist** —
   whitespace-normalised instruction-text equality on corresponding lines, the only thing that
   separates a renumber from a rebuild. `form-delta` reports name churn and label drift and cannot
   answer *"is this the same sentence?"*. Out of scope here; still the gap.
5. **`compute` calls `label_join_public(...).ok()`, discarding the error text.** A geometry fixture
   that exists but fails to parse is now reported as `GeometryFixtureMissing` — right verdict, wrong
   diagnosis. Fixing it means threading the `String` error into `Unwitnessed`, which changes the enum
   for a case I could not produce on a real artifact. Left alone deliberately.
