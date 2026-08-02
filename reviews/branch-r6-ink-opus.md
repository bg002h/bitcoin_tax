# r6 — INK LENS (Opus)

**Date:** 2026-08-02 · **Range:** `fc96703..HEAD` · **Brief:** [`BRIEF-r6.md`](./BRIEF-r6.md)

**Scope:** the 60 lines in `form1040_full.rs` — the ONLY change in ~2,700 new lines that alters a filed
PDF. **Result: 0 Critical / 1 Important / 3 Minor / 2 Nit.** The gate is correct against the form on
every branch it could construct.

★★★ **THE IMPORTANT IS A SAFETY PROPERTY MY FIX REMOVED, AND NOTHING REDDED.** Because lines 34/35a and
37 are now mutually exclusive writes, the descent group never again contains both — so the
**map-independent geometric oracle no longer catches a 35a↔37 or 34↔37 map swap, which it DID catch
before this commit.** Demonstrated rather than argued: the reviewer planted both swaps against the
pre- and post-change writers. Pre-change → `Err(Geometry("ordinal-y descent broken"))`. Post-change →
`Ok`, with only a value assertion failing. Two doc comments in the crate still promise the fail-closed
behaviour that disappeared.

★ The cost is entirely in **the next map** — TY2025 has none yet, and the shipped TY2024 map is still
protected, but by a *test-time* literal-FQN pin rather than the production guarantee. Suggested fix is
cheap and map-independent: assert from the blank PDF's own geometry that
`cy(line34) > cy(line35a) > cy(line37)` before the branch, with the planted swap as its B1 kill.

★★ **AND A MINOR THAT IS MY CLAIM BEING WRONG.** The orphan row's stated justification — *"a
hand-edited `income import` TOML can state a `line34` that contradicts its own operands"* — is false.
`income import` parses into `ReturnInputs`; `Form1040Lines` has no serde derives and is constructed at
exactly one site, which always computes 34/37 from 24/33. The state is reachable only from a test
fixture. **The gate is still right** — deriving from the operands is what makes the mark impossible if
`printed.rs`'s `.max(ZERO)` ever moves — but the reason I wrote into the test doc AND into
`FOLLOWUPS.md` §G-24 is not true, and it is the same class as the tenforty incident.

★ Also carried: line 37's provenance is incomplete (the writer applies a condition the form does not
state on that line — a correct inference, recorded nowhere but a source comment); and a genuinely
useful hardening — the writer emits `lines.line34` rather than `line33 − line24`, so if
`Form1040Lines` ever becomes deserializable the reverse orphan is a *wrong figure*, not a blank.

**Verbatim below.**

---

Restored both files I mutated; verified byte-identical to HEAD. The one remaining working-tree modification (`crates/btctax-core/src/tax/line_coverage.rs`, previously `crates/xtask/src/line_coverage_check.rs`) is a live "TEMPORARY REVIEW MUTATION" from another lens running concurrently in this shared tree — not mine, and I left it alone.

---

# INK LENS — review r6, `crates/btctax-forms/src/form1040_full.rs` (§G-24)

**VERDICT: 0 Critical / 1 Important**

The gate itself is correct against the form, on every branch I could construct. The Important is a
side effect of *how* it was implemented: it silently removed a fail-closed geometric guarantee that
existed before the change, and nothing red.

---

```
SEVERITY: Important
WHERE: crates/btctax-forms/src/form1040_full.rs:201-229 (interacting with crates/btctax-forms/src/verify.rs:403-414)
CLAIM: Because lines 34/35a and line 37 are now mutually exclusive writes, the descent group
       GRP_P2_AMOUNT never again contains both — so the map-independent geometric oracle no longer
       catches a 35a↔37 or a 34↔37 map swap, a swap it DID fail closed on before this commit.
FAILURE: With `map.line37` pointing at line 34's or line 35a's widget (a plausible error the next time
       a full-return map is authored — TY2025 has none yet), an owing return prints "$904 — Amount you
       owe" into the box captioned "Amount you overpaid" / "Amount of line 34 you want refunded to
       you", and `fill_form_1040_full` returns Ok. Demonstrated, not reasoned:

         planted: map.line35a ↔ map.line37   (post-change writer)
           → fill_form_1040_full returns Ok; KAT fails on the VALUE assertion
             (`the amount owed is entered  left: None  right: Some("904")`) — no Geometry error.
         planted: map.line34  ↔ map.line37   (post-change writer)
           → same: fill returns Ok, assertion-level failure only.
         same two plants against the PRE-change writer (fc96703:form1040_full.rs), map untouched:
           → Err(Geometry("ordinal-y descent broken: ...f2_28[0] (y 414.0) is not strictly above
             ...f2_24[0] (y 474.0) — mis-mapped row/line"))

       Mechanism: on an owing return the only surviving comparison is line33(ord 12, f2_22, y≈498) vs
       line37(ord 15). f2_23 (y≈486) and f2_24 (y≈474) both sit below f2_22, so both satisfy the
       descent. On a refund return the only comparison is 34 vs 35a.
EVIDENCE: form1040_full.rs:78-79 — "The serialized bytes are read back through the geometric verifier
       (a mis-mapped cell FAILS CLOSED)"; verify.rs:270-271 — "a mis-mapped cell lands in the wrong
       cluster / breaks monotonicity and FAILS CLOSED". Both are now false for the 34/35a/37 block.
       The x-cluster leg cannot help (all three are COL_AMOUNT); `assert_only_filled` cannot help (the
       target widget is in the allowed set). The repo's only 1040 same-column fault-injection,
       `form_1040_full_same_column_swap_fails_closed` (full_return_forms.rs:1719), swaps lines 9↔15 —
       both unconditional — so it stays green and cannot see this.
       The shipped TY2024 map IS still protected, but by a different instrument: the new KAT pins all
       three FQNs as literals (full_return_forms.rs:1435-1437) and reds on either plant. That is a
       test-time guard on one committed asset, not the production fail-closed guarantee.
SUGGESTED FIX (cheap, restores the property for both branches): before the branch, assert from the
       blank PDF's own geometry that cy(map.line34) > cy(map.line35a) > cy(map.line37) — a
       map-independent ordering check on the three cells regardless of which branch fires — and land
       it with the planted swap as its B1 kill-test.
```

---

## Minor / Nit

```
SEVERITY: Minor
WHERE: crates/btctax-forms/tests/full_return_forms.rs:1501-1504; FOLLOWUPS.md:1267
CLAIM: The stated production mechanism for the orphan row — the row that justifies gating on the
       operands rather than on `!= ZERO` — does not exist.
FAILURE: "a hand-edited `income import` TOML can state a `line34` that CONTRADICTS its own operands".
       It cannot. `income import` parses into `ReturnInputs` (cmd/tax.rs:56 `parse_return_inputs_toml`);
       `Form1040Lines` has no serde derives (printed.rs:447 — `#[derive(Debug, Clone, Copy, PartialEq,
       Eq)]`) and is constructed at exactly one site, `form_1040_lines` (printed.rs:699), which always
       computes 34/37 from 24/33. The orphan state is reachable only from a test fixture or a future
       edit to printed.rs.
EVIDENCE: `grep -rn Form1040Lines --include='*.rs'` → constructed only at printed.rs:699 plus test
       fixtures. This is the same class the project already burned itself on (memory: "filed on ONE
       oracle against our own rule and it cost us — a claim in it was wrong").
       The GATE is still the right one and the orphan test is still worth keeping — deriving from the
       operands is what makes the mark impossible if printed.rs's `.max(ZERO)` ever moves. Only the
       justification needs correcting, in both the test doc comment and the FOLLOWUPS entry.
```

```
SEVERITY: Minor
WHERE: crates/btctax-core/src/tax/line_coverage.rs:1732-1739
CLAIM: Line 37's recorded provenance is now incomplete — the writer applies a condition that neither
       the form's text nor the coverage table states anywhere.
FAILURE: Line 34 got a written `c.exception(...)` (line_coverage.rs:1729-1731) precisely because its
       instruction embeds the blank-ness condition. Line 37 stays `Production::Combine` with the
       verbatim, unconditional "Subtract line 33 from line 24. This is the amount you owe." — but
       form1040_full.rs:200/220 gates it on `line24 > line33`, a comparison the form does not state on
       line 37. The commit comment ("The gate is the FORM'S OWN COMPARISON") is accurate for 34 and an
       inference for 37. The inference is correct — the form obviously does not want a negative there —
       but the sole record of it is a source comment, so the instrument that exists to make provenance
       greppable does not carry it.
EVIDENCE: f1040--2024.txt:120-121 vs form1040_full.rs:220-229; the exactly-even case (24 == 33) is
       where the two readings visibly differ (literal transcription ⇒ "0"; the code ⇒ blank). Blank is
       the right call under this project's doctrine and matches what a paper filer does; it just is not
       a transcription, and should be recorded as the judgment it is.
```

```
SEVERITY: Nit
WHERE: crates/btctax-forms/src/form1040_full.rs:227
CLAIM: `(p2_amount.len() + 2)` hard-codes the length of the 2-element array above it.
FAILURE: If line 36 (or the Form 8888 marker) is ever added to the overpaid block, the loop takes
       ordinals 13/14/15 and line 37's `+2` collides at 15. It cannot manifest today — the two branches
       are mutually exclusive — and a duplicate ordinal would not red the descent check either (it
       sorts, then compares pairwise). A named `const` or `+ overpaid_cells.len()` removes the trap.
EVIDENCE: form1040_full.rs:202-217 vs :227.
```

```
SEVERITY: Nit
WHERE: crates/btctax-cli/src/render.rs:1567-1571
CLAIM: On the exactly-even return the console and the paper now disagree.
FAILURE: line24 == line33 ⇒ line34 == 0 ⇒ the `else` arm prints "→ AMOUNT OWED (L37): 0" while the
       filed page leaves line 37 blank. Not a mark on a return and not misleading ($0 owed is true),
       but the console is the filer's reading of the return and it now describes a line that isn't
       there. `docs/examples/examples.md:1042` is the only golden touching this text and it is an
       owing (non-even) case, so nothing red.
EVIDENCE: render.rs:1567 branches on `f.line34 > Usd::ZERO` — a two-way split over what is now a
       three-way form (overpaid / owed / neither).
```

```
SEVERITY: Nit  (PRE-EXISTING — not a regression in this range, flagged because the commit's own
                doctrine names the class)
WHERE: crates/btctax-forms/src/form1040_full.rs:204, 213
CLAIM: Line 35a is "Amount of line 34 you want refunded to you" — an ELECTION — and the software makes
       it, writing the whole of line 34 without asking.
FAILURE: The filer who wanted part of the overpayment applied to 2025 estimated tax (line 36) files a
       return swearing they wanted all of it back. This is the same "figure the filer never gave" the
       fix exists to remove; binding 35a to 34's *condition* is correct (no overpayment ⇒ nothing to
       refund), and 35a = 34 with 36 blank is at least internally consistent and matches the
       `RefundByPaperCheck` advisory (advisories.rs:446). Worth a §G-11 follow-up row, not a change here.
EVIDENCE: f1040--2024.txt:115, 119 ("Amount of line 34 you want refunded to you" / "…you want applied
       to your 2025 estimated tax").
```

---

## ALSO CHECKED, SOUND

- **The gate against the form.** L34 `"If line 33 is more than line 24"` ↔ `lines.line33 > lines.line24`
  — exact, including strictness. `f1040--2024.txt:114`. No `-0-` clause on 34, 35a or 37 anywhere in
  the extracted text. Both operands are the **printed** (whole-dollar) lines, so the condition a reader
  re-derives from the paper is the condition the code evaluated — the right choice over exact cents.
- **The sub-$1 instruction cases** (`i1040gi--2024.txt` "If line 34 is under $1…", "You don't have to pay
  if line 37 is under $1") are unreachable: both operands are already rounded, so a non-zero difference
  is ≥ $1.
- **35b/35c/35d, 36, 38 are unmapped by design** (`f1040.map.toml:87-90` documents each), so they were
  always blank and cannot contradict a blank 34. On a refund return 34 + 35a print with 35b-d blank ⇒
  paper check ⇒ the advisory fires. On an owing return the entire refund block is blank. Both coherent.
- **Ordinals.** 34→13, 35a→14, 37→15; identical to pre-change. No collision, and the gap (12→15 on an
  owing return) is harmless because `verify_flat` sorts by ordinal and compares only *present*
  placements pairwise (verify.rs:403-413). Skipping cells is sound as an ordering matter; the loss is
  the coverage documented in the Important above.
- **Readers.** `transcribe::extract_lines` documents absent-key = blank and returns a `BTreeMap` with no
  expected key set (transcribe.rs:33-37, 66-70). `assert_only_filled` only rejects *extra* filled fields.
  The oracle harness compares 1040 lines 11/12/13/15/16/17/21/24 only — never 34/35a/37 — and its
  `paper()` treats a missing key as `None` (oracle-harness/src/main.rs:220-227). `golden_packet.rs` asserts
  byte-reproducibility by refilling, against no committed digest. Nothing reads these three cells back.
- **Other emission paths.** `fill_form_1040_capgains` (the crypto slice) writes exactly `line7a` + the
  DA checkbox — never the refund block (form1040.rs:105-133). `packet.rs:74` is the only other caller and
  it goes through `fill_form_1040_full`. `map.line34/35a/37` are referenced from exactly one file.
- **Filing status / routing.** The gate reads only 24 and 33; no status, no MFS branch, no dependent or
  aged/blind interaction touches it. A zero-tax zero-payment return leaves both blank, which is right.
- **Suite.** `make check` green at HEAD before I touched anything: **2548 passed, 12 skipped**.
  `cargo nextest run -p btctax-forms` green after restore: 233/233.

## WHAT WOULD MAKE THIS REVIEW WRONG

1. **If the Important is judged out of scope.** It is not a wrong mark on a return today — the TY2024
   map is correct and the new KAT pins it by literal FQN. I graded it Important because the brief
   defines Important as "an instrument that cannot detect what it claims to", and two doc comments in
   this crate still promise the exact fail-closed behaviour I watched disappear. If the standard is
   "wrong ink today", it is a Minor.
2. **If the exactly-even return should print `0` on line 37.** Line 37's text is genuinely
   unconditional, and I could not find an IRS statement that resolves blank-vs-zero at equality. I
   accepted blank as correct (it is what a paper filer does, and it is the project's own doctrine), but
   that is a judgment against silence, not a citation — which is exactly why I filed the Minor asking
   for it to be recorded as one. If the equality case has an authority saying "enter -0-", the gate on
   line 37 is wrong at one point.
3. **If `Form1040Lines` becomes deserializable.** My "the orphan case is unreachable" claim rests on it
   having exactly one constructor. Add serde to it — or a stored/cached printed return — and the
   reverse orphan (condition true, carried `line34` stale) becomes a *wrong figure*, because the writer
   gates on the operands but still emits `lines.line34` (form1040_full.rs:213) rather than
   `line33 − line24`. Cheap insurance would be to write the difference.
4. **If TY2025's map is authored before the Important is closed.** Everything I verified is TY2024-only;
   the full-return fill refuses other years (`need()`, form1040_full.rs:52-59), and the blind spot's cost
   is entirely in the next map.