# Review brief — r9, B2 (>4 dependents) on `main`

**Scope:** `git diff fdc2324..HEAD -- crates` — commits `3a4d06a` (B2) and `b296afe` (the DRAFT banner).
Everything else in the range since r8 is a test-only guard, a docs entry, or an extract fix.

## THE ONE QUESTION

**B2 creates a NEW FILED ARTIFACT — a page a filer detaches and attaches to a return signed under
26 USC §6065. Is that page correct, complete, and free of anything the filer did not assert?**

## What changed

btctax REFUSED a fifth dependent. Form 1040 has its own remedy and i1040gi states it:

> *"If you have more than four dependents, check the box under Dependents on page 1 of Form 1040 or
> 1040-SR and include a statement showing the information required in columns (1) through (4)."*

So the packet now prints the first four in **capture order**, checks the box (`c1_13[0]`, which was
already mapped and never written), and emits `dependents_statement.txt`, listed in the manifest.

New/changed: `btctax-core/src/tax/dependents_statement.rs`, `btctax-forms/src/{form1040_full.rs,
packet.rs}` (`fill_full_return` now returns `FiledPacket { forms, statements }`),
`btctax-cli/src/cmd/admin.rs` (writes the statement + `statement_body`), `LIMITATIONS.md`.

## Where the risk is

### 1 · The statement's CONTENT — does it assert anything the filer did not?

A draft rendered column (4) as *"NOT CLAIMED for any dependent on this return"*. That was rejected: it
converts a lawful blank into an affirmative assertion on a signed page. **Read the rendered artifact and
find anything else of that shape.** Specifically:

- *"Together they are the complete list."* — is that true and safe, or does i1040gi's TIP
  (*"The dependents you claim are those you list by name and SSN in the Dependents section"*) mean the
  statement should restate ALL of them rather than only the overflow? Argue from the instruction text.
- *"The box under Dependents on page 1 is checked."* — the statement asserts something about a
  different page. If the box were somehow unchecked, the statement would be lying. Is that
  reachable? (Both read `more_than_four_dependents()`; verify that is actually true of the emitted PDF,
  not just of the source.)
- Column headings claim to be the form's own. Verify them against
  `design/forms/extract/f1040--2024.txt:40-41`, character by character.

### 2 · Completeness and identity

- Is every dependent accounted for **exactly once** across page 1 + statement, for every household
  size? I checked n=9 by hand and 0..=12 in a test; find a size or shape that breaks it.
- A detached page must be attributable to its return. It carries the name line and the taxpayer's SSN.
  Is that the right identity — and is printing a full SSN on a loose page the right call, given
  `Ssn`'s `Debug` is deliberately masked because an SSN in a log is a PII incident?
- **Capture order is now load-bearing for a MAILED artifact.** Nothing sorts today. Verify that — grep
  for anything that could reorder `header.dependents` between capture and emission (the TUI's
  add/remove, `income import`, `ReturnHeader::build`, a `BTreeMap` anywhere in the path).

### 3 · The capacity guard

Core owns the split (`DEPENDENTS_GRID_ROWS = 4`); the MAP independently declares
`dependent_rows.len()`. `fill_form_1040_full_with_map` refuses on disagreement. Is that guard
correctly placed (before any cell is written), and is it reachable? Can a map disagree in a way it does
not catch?

### 4 · The DRAFT banner (`b296afe`)

A `.txt` cannot carry a diagonal watermark, so on a pseudo-reconciled ledger `statement_body` prefixes
`*** DRAFT — ESTIMATE, NOT FOR FILING ***`. Check: is `watermarked` the right predicate, is it the same
one the PDFs use, and does any path write a statement while bypassing it?

### 5 · The type change

`fill_full_return` returns `FiledPacket` now. Every consumer changed. Does anything silently drop
`statements` — the oracle harness takes `.forms` deliberately, but check the CLI, the TUI, the
defensive wizard, and the crypto-slice path.

## Settled — do NOT re-derive

- That the refusal was wrong is decided; i1040gi supplies the remedy verbatim.
- The three test scenarios re-ran clean after B2 (TaxCalcBench L37 owed 1,085 exact; Pub 560 Schedule SE
  line 12 = 26,262; the owner's 9-dependent scenario files with figures identical to the 4-dependent
  version, which is correct because dependents feed no arithmetic while CTC/ODC is $0).
- Golden md5 `c4e1853ed82d113ca5cd97ffd8abbf47` unchanged; 2568 tests; five gates green.
- Whether v1 should compute CTC/ODC is out of scope — it does not, and column (4) is blank on the page
  and in the statement alike.

## Output

`VERDICT: clean` or `VERDICT: <n> Critical / <n> Important`, then per finding:

```
SEVERITY: Critical | Important | Minor | Nit
WHERE: path:line
CLAIM: one sentence.
FAILURE: concrete inputs → the wrong mark on a filed page, or a filer misled.
EVIDENCE: quote the code AND the form/instruction text it violates.
```

**Critical** = a wrong or missing mark on a filed artifact, or a return wrongly refused. **Important** =
a real defect, or a guard that cannot catch what it claims to. A short clean report is a fine outcome
for ~400 lines.

End with `ALSO CHECKED, SOUND:` and `WHAT WOULD MAKE THIS REVIEW WRONG:`.

**Constraints:** you are in your OWN git worktree — mutate freely to verify a kill, but make NO commits
and leave the working tree clean when you finish. No subagents.
