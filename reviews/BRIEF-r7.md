# Review brief — r7, `4fe5ce4..HEAD` (`58515da`) on `main`

Read the range yourself: `git diff 4fe5ce4..HEAD -- crates`. Four files, +433/−62.

| file | Δ | what it is |
|---|---|---|
| **`btctax-forms/src/form1040_full.rs`** | **+51** | **the only change that can alter a filed PDF** |
| `xtask/src/line_coverage_check.rs` | +402/−62 | the transcription checker — new rules (2b) and (2c) |
| `btctax-core/src/tax/line_coverage.rs` | +27/−9 | two rows of the coverage table, corrected |
| `btctax-forms/tests/full_return_forms.rs` | +15 | a doc correction |

## THE ONE QUESTION

**Can any of this put a wrong mark — or remove a required one — on a filed federal return; or does
either new rule CLAIM a guarantee it does not deliver?**

## ★ What r6 already covered — do not re-run it

Round r6 reviewed `fc96703..HEAD` with two lenses (`reviews/branch-r6-ink-opus.md`,
`reviews/branch-r6-instrument-opus.md`) and both converged. It covered: the §G-24 gating of 1040 lines
34/35a/37, the 2,145-line coverage table, the checker's twelve rules, `money_bearing_types`,
`mentions_ident`, the `NOT_PRINTED` derivation, and the 11 exceptions. **Do not re-audit those.**

**This range is r6's own FOLD plus the work after it — precisely what no reviewer has held.** That is
the seam, and it is where your budget should go:

1. `form1040_full.rs`'s **geometry guard**, written in response to r6's finding that §G-24 removed a
   fail-closed property. Never reviewed.
2. `line_coverage_check.rs`'s **`check()` extraction and 12 planted-defect kills**, written in response
   to r6's keystone finding that the B1 test never called the checker. Never reviewed.
3. **Rules (2b) and (2c)** and the `MAX_UNLOCATABLE` ratchet. Entirely new.

## Lens A — the ink (`form1040_full.rs`)

The guard asserts, from `blank_fields` and **before** either branch is taken, that the map places
line 34 above line 35a above line 37 (`y34 > y35a > y37`), returning `FormsError::Geometry` otherwise.

- It contains one `return Err(...)` and no `push_money` / `writes.` / `placements.` — I claim **its only
  power is to refuse**. Verify that claim, and verify it is *reachable*.
- **Can it refuse a return it should not?** A map where these cells are legitimately ordered
  differently, a form year where the layout moves, a missing field, a rotated page.
- **Does it actually hold the property r6 said was lost?** r6's point was that §G-24's gating removed a
  fail-closed geometric check. Does an ordering guard restore *that* property, or a different one that
  merely resembles it?
- Anything reading these cells back — `transcribe::extract_lines`, `no_unmapped_filled`, the oracle
  harness, `render.rs` — still sees an absent key where a `0` used to be. r6 cleared that for the
  gating; clear it for the **guard**.

## Lens B — the instrument (`line_coverage_check.rs`)

**Rule (2) verified a quote existed *somewhere in the form's file*.** A row could name line 4, quote
line 9, and pass — `CLAUDE.md`'s standing root cause, the Form 6251 line-33 defect.

**(2b)** — `label_precedes(text, label, quote)` asks whether some **form of the label** sits immediately
before the quoted sentence in the normalized extract. Three forms are accepted, most specific first:
the label itself (`25a`), the **bare suffix** (`a`, because sub-lines print as bare letters), and the
**stem** (`25`, because the stem carries a lead-in the transcription rightly quotes:
`25 Federal income tax withheld from:` + `a Form(s) W-2`). The bare-suffix form additionally requires
the row's stem to appear within a **700-byte run-up**.

**Every widening is a chance for a wrong quote to pass. Find one.** Concretely:

- **Construct a real misattribution that (2b) accepts** — a row naming line X carrying line Y's verbatim
  sentence, on a form in `design/forms/extract/`. That is the finding that matters most.
- The **700-byte** window is a magic number. Is it exploitable — or, in the other direction, does it
  make a *correct* transcription fail on some line already in the table?
- The **stem** form: a row could name `25d` and quote line `25`'s own lead-in text and pass. Is that
  right or wrong? Argue it from the form.
- **(2c)**: a row naming `(none)` must carry no quote. It is an `else`, not a `continue`, so such a row
  still faces rules (3)+. Confirm no rule below silently assumes a non-empty `instruction`.
- **`MAX_UNLOCATABLE = 13`.** 189 of 189 rows bind; 13 do not and are named in the summary — 12 Form
  8949 cells whose quote is a column *header*, and the QDCGT worksheet line. **Is that classification
  honest, or is one of the 13 a form line being waved through?**
- **The kills.** Twelve plants now call `check()`. Are they real? For each new one, which rule's
  deletion makes it red? I mutation-verified (2b), (2c) and the escape-hatch plant; **check the rest**,
  and check that a plant does not pass for a reason other than the rule it names.
- **Vacuity.** `label_forms` returns `None` for a label it cannot express, which routes to
  `unlocatable`. Can a table author reach that path *deliberately* to dodge (2b) — e.g. by writing a
  label with a parenthesis?

## Settled — do NOT re-derive

- The TY2024 golden matrix md5 **`c4e1853ed82d113ca5cd97ffd8abbf47` is unchanged across this range** —
  that is the evidence no figure moved. All five gates green, 2548 tests.
- §G-25 (MFS/HoH brackets unwitnessed) and the 11 exceptions are **filed with reasons**. Re-arguing a
  filed item is not a finding; showing one is *wrongly* filed is.
- Whether §G-11 should be worked at all is decided (`design/direction/`).
- `tables.rs::printed_line` was considered and rejected as the fix; the reasoning is in the commit
  message and `FOLLOWUPS.md` §G-27a. Do not re-propose it without engaging that reasoning.
- G-27b–e remain **open and filed**. Not in scope.

## Output

`VERDICT: clean` or `VERDICT: <n> Critical / <n> Important`

```
SEVERITY: Critical | Important | Minor | Nit
WHERE: path:line
CLAIM: one sentence.
FAILURE: concrete inputs → the wrong mark on a filed return, or the guarantee that is false.
EVIDENCE: quote the code AND the form text or the rule it violates.
```

**Critical** = a wrong or missing mark on a filed return. **Important** = a real defect, or an
instrument that cannot detect what it claims to. A short clean report is a fine outcome.

End with `ALSO CHECKED, SOUND:` and `WHAT WOULD MAKE THIS REVIEW WRONG:`.

**Constraints:** READ-ONLY on tracked files. You may mutate temporarily to verify a kill **if** you back
up with `cp` to `/tmp` and restore with `cp` — never `git checkout --`. Leave the tree clean and say so.
No commits. No subagents.
