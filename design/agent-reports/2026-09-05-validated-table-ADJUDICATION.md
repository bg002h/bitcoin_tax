# ADJUDICATION — the validated tax-table transcriptions (TY2017 / TY2025 / TY2026)

**Verdict: 0 Critical / 0 Important / 3 Minor / 3 Nit. TY2025 is a genuine transcription, not an echo
— all 39 figures verified by me directly against `RevProc_2024-40.txt` before I read `tax_tables.rs`,
and four of the four top thresholds are additionally pinned by the procedure's own cumulative-tax
column. TY2017 and TY2026 were never transcribed; the ratchet correctly still records them as gaps.**

Method: read `testonly::ty2025_table()` → read the revenue-procedure TEXT LAYER → *then* read the
shipped `tax_tables.rs::ty2025()`. Every figure below was checked against the pasted source line, not
against the shipped table. `git` and `make check` were not run, per brief.

---

## §1. Critical — **EMPTY.** No wrong figure, and no echo.

### 1.1 Every TY2025 figure IS in Rev. Proc. 2024-40 (36 bracket/breakpoint figures + 3 scalars)

`legal/text/irs-guidance/RevProc_2024-40.txt`, §2.01 (lines 136–205). Top and bottom threshold for
**every** filing status, pasted:

```
    TABLE 1 - Section 1(j)(2)(A) –Married Individuals Filing Joint Returns and Surviving
        Not over $23,850                               10% of the taxable income
        Over $751,600                                  $202,154.50 plus 37% of
                   TABLE 2 - Section 1(j)(2)(B) – Heads of Households
        Not over $17,000                               10% of the taxable income
        Over $250,500 but                              $55,484 plus 35% of
        Over $626,350                                  $187,031.50 plus 37% of
TABLE 3 - Section 1(j)(2)(C) – Unmarried Individuals (other than Surviving Spouses and
        Not over $11,925                              10% of the taxable income
        Over $250,525 but                             $57,231 plus 35% of
        Over $626,350                                 $188,769.75 plus 37% of
        TABLE 4 - Section 1(j)(2)(D) – Married Individuals Filing Separate Returns
        Not over $11,925                              10% of the taxable income
        Over $375,800                                 $101,077.25 plus 37% of
```

All 28 thresholds and all 28 rates in `testonly.rs:983` `ty2025_table()` match these tables exactly
(interior rows checked too: 96,950 / 206,700 / 394,600 / 501,050; 64,850 / 103,350 / 197,300;
48,475 / 103,350 / 197,300). The two traps are transcribed, not derived:
**HoH's 35% floor is $250,500 and Single/MFS's is $250,525** — a real $25 split — and MFS's 37% floor
is $375,800.

Every §1(h) breakpoint, §2.03 (source line pasted verbatim):

```
 Filing Status                                                   Maximum Zero      Maximum15%
                                                                  Rate Amount      Rate Amount
 Married Individuals Filing Joint Returns and Surviving Spouse      $96,700         $600,050
 Married Individuals Filing Separate Returns                         $48,350         $300,000
 Heads of Household                                                  $64,750         $566,700
 All Other Individuals                                               $48,350         $533,400
```

All 8 match. **MFS's 15% ceiling is $300,000, NOT half the joint $600,050 ($300,025)** — the procedure
prints the round number and the transcription carries it.

The three scalars:

```
   .41 Unified Credit Against Estate Tax. For an estate of any decedent dying in calendar
year 2025, the basic exclusion amount is $13,990,000 for determining the amount of the
     (1) For calendar year 2025, the first $19,000 of gifts to any person (other than gifts of
```

→ `gift_lifetime_exclusion: dec!(13_990_000)` ✓, `gift_annual_exclusion: dec!(19000)` ✓.

`ss_wage_base: dec!(176100)` is **not** in the revenue procedure and does not claim to be. The agents'
correction to the brief is upheld — it is sourced, in-repo, and the page cite in the code is the
precise one. `legal/text/federal-register/SSA_COLA_Determinations_2025.txt`, last page header before
the quoted line is `Federal Register / Vol. 89, No. 207 / Friday, October 25, 2024 / Notices 85279`:

```
   OASDI Contribution and Benefit Base
     The OASDI contribution and benefit
   base is $176,100 for remuneration paid
   in 2025 and self-employment income
   earned in tax years beginning in 2025.
```

**No figure is uncited.** (§1(h) breakpoints, brackets, both gift figures, and the wage base each carry
a section or FR cite in the doc comment or the inline comment.)

### 1.2 Independent arithmetic confirmation (a second encoding, not a re-read)

The procedure's cumulative-tax column is a redundant encoding of the same breakpoints. Computed here,
not copied from any agent report:

| status | check | computed | procedure prints |
|---|---|---|---|
| MFJ | `114,462 + 0.35×(751,600−501,050)` | 202,154.50 | `$202,154.50 plus 37%` |
| HoH | `55,484 + 0.35×(626,350−250,500)` | 187,031.50 | `$187,031.50 plus 37%` |
| Single | `57,231 + 0.35×(626,350−250,525)` | 188,769.75 | `$188,769.75 plus 37%` |
| MFS | `57,231 + 0.35×(375,800−250,525)` | 101,077.25 | `$101,077.25 plus 37%` |
| HoH 35% floor | `38,460 + 0.32×(250,500−197,300)` | **55,484** | `$55,484 plus 35%` |
| — the rejected alternative | `38,460 + 0.32×(250,525−197,300)` | 55,492 | *(not printed)* |

The last two rows are what independently rule out the $25 copy error, from the procedure alone.

### 1.3 The echo question, answered

**Not an echo.** The transcription agrees with `tax_tables.rs` everywhere *because both agree with the
procedure*, which I established from the procedure side. Independence is further visible structurally:

- different insertion order (validated: Single, Mfj, **Mfs, HoH**; shipped: Single, Mfj, **HoH, Mfs**);
- different citation scheme (validated cites §2.01 TABLE 1–4 / §1(j)(2)(A)–(D); shipped cites
  §1(c)/(a)/(b)/(d));
- `source` deliberately differs and is deliberately **excluded** from the comparison —
  `shipped_tables_are_the_validated_tables.rs:311,320` destructure `source: _`, with the comment
  "Equal `source` strings would mean one was copied".

**The gate is live, not inert (B1 kill, run here).** I planted `250500 → 250525` in
`testonly::ty2025_table()`'s HoH bracket and re-ran the binary:

```
thread 'every_shipped_tax_table_equals_the_one_the_corpus_validates' panicked at ...:359:17:
TY2025 HoH ordinary bracket 5: shipped (250500, 0.35) vs validated (250525, 0.35)
  — the binary would tax a filer differently from every test that says it is correct
Summary: 9 tests run: 8 passed, 1 failed
```

The file was restored from a byte-identical backup (`diff -q` clean). Note precisely what this proves:
that **TY2025 is actually compared** and the equality is not vacuous. It does *not* prove
independence — a copied corpus would red on the same mutation. Independence rests on §1.1/§1.2.

### 1.4 The ratchet passes for the right reason

- Shipped year set unchanged and **not** quietly narrowed: `tax_tables.rs:76-79` inserts
  `2017, 2024, 2025, 2026`. 2025 is still shipped, and the mutation above proves it is still compared.
- `validated_table_for` (`:219`) resolves `2024 → ty2024_table()`, `2025 → ty2025_table()`, `_ → None`.
- `KNOWN_UNVALIDATED_TABLE_YEARS = &[2017, 2026]` (`:771`) — and the "list may only shrink" half of the
  test (`:786-798`) would red if 2017 or 2026 had been left in the list after acquiring a counterpart.
  Neither has one, so the excuse list is accurate rather than stale.
- `cargo nextest run -p btctax-adapters --test shipped_tables_are_the_validated_tables` → **9/9 PASS**.

### 1.5 TY2017 (pre-TCJA) and TY2026 — nothing was transcribed, so nothing is wrong

No `ty2017_table()` or `ty2026_table()` exists in `testonly.rs`; `grep -c "pub fn ty2025_table"` = 1
(the triple-definition E0428 was cleaned up). The pre-TCJA warning in the brief was not triggered:
nobody wrote a 2017 corpus. For what it is worth, the *shipped* `ty2017()` does carry the pre-TCJA
schedule (10/15/25/28/33/35/**39.6%**, Single 9,325 / 37,950 / 91,900 / 191,650 / 416,700 / 418,400),
so the year is at least the right shape — but it remains **unwitnessed**, exactly as the ratchet says.

**Coverage, stated plainly: the round bought one year out of three.** 2017 and 2026 are as exposed
after this round as before it.

---

## §2. Important — **EMPTY.** No figure lacks a citation.

Every figure in `ty2025_table()` carries a section or Federal-Register cite, and I resolved each one
against the committed text above. The one claim that is *not* witnessed in-repo lives in the shipped
`source` string, not in a figure, and is recorded as N-3 below.

---

## §3. Minor

### M-1 — Two of the three agent reports do not exist. The triple-blind witness is not on disk.

`design/agent-reports/` contains exactly one file matching `2026-09-05-validated-table-*.md`
(`…-2025.md`, 258 lines). All three agents were told to write the same year-derived filename, so the
same collision that produced three `ty2025_table()` definitions also produced one surviving report.
The reported "all three blocks byte-identical on 38–39 values" is therefore **unverifiable from the
repository** — it may well be true, but it is testimony, not evidence, and it should not be cited later
as though it were a committed measurement. (My §1.1/§1.2 verification stands independently of it, and
is the reason the verdict is still clean.) Same root cause as the unrendered `${t.y}` template: the
report filename was a second place the missing substitution could have failed loudly and did not.

### M-2 — Three in-file measurements are now false ("`testonly` … no other year")

Left deliberately by the surviving agent, whose brief scoped it to its own arm. Confirmed stale:

```
46:  the corpus** — `testonly` defines `ty2024_table` and `ty2024_params` and nothing else. The
216: // `ty2024_params` (`testonly.rs:53`) and `ty2024_table` (`testonly.rs:111`) — no other year.
725: /// corpus — 260 references to `ty2024_table`/`ty2024_params` across `crates/`, zero to any other year
```

`testonly.rs:983` now defines `ty2025_table`, and the test file itself imports it at `:72`. Line 216
is worst because it is stamped "Measured 2026-09-05" — a dated measurement that was false the same
day. One-line fix each; no behavioural effect.

### M-3 — The workspace does not compile (unrelated to the tables, but the tree is red)

`cargo check --workspace --all-targets` fails in `crates/xtask/src/cite_check.rs`:

```
error[E0425]: cannot find value `EMITTED_FORMS` in this scope
    --> crates/xtask/src/cite_check.rs:1007:39
error[E0277]: a value of type `BTreeSet<&str>` cannot be built from an iterator over elements of
             type `(&str, &[i32])`
    --> crates/xtask/src/cite_check.rs:995:82
```

Nothing to do with tax tables — it is in the citation checker, and looks like another live arm of the
same round (concurrent edits were still landing while I adjudicated). Recorded because a scoped green
(`-p btctax-adapters`) is not a green tree, and the next agent to run a workspace gate will hit this.

---

## §4. Nit

- **N-1 — shipped wage-base comment cites the press release, not the determination.**
  `tax_tables.rs:553` says `SSA announced 2024-10-10`; the determination of record is 89 FR 85279,
  published 2024-10-25 and **committed in this repo**. Citing the committed document makes the figure
  checkable without leaving the tree. (The validated side already cites it correctly.)
- **N-2 — shipped schedule labels use the pre-TCJA subsections.** `tax_tables.rs:446,462,478,494`
  label the TY2025 schedules `§1(c)/(a)/(b)/(d)`; Rev. Proc. 2024-40 publishes them under
  `§1(j)(2)(C)/(A)/(B)/(D)`, which is what supplies the TY2018–2025 figures. Both are defensible;
  the procedure's numbering is the precise one, and the validated side uses it.
- **N-3 — an unwitnessed clause inside an uncompared field.** The shipped `source` asserts
  "OBBBA Pub. L. 119-21 left 2025 brackets/breakpoints unchanged". No copy of Pub. L. 119-21 is
  committed under `legal/`, the revenue procedure predates it, and `source` is excluded from the
  comparison by design — so a green ratchet must not be read as having checked that clause. Not a
  figure and not blocking; flagged so nobody later mistakes the ratchet's green for cover.

---

## §5. What the orchestrator should carry forward

1. **TY2025 is closed and honestly closed.** Keep it; it is now a three-way agreement (procedure,
   corpus, binary) with the ratchet holding it.
2. **Re-dispatch TY2017 (`RevProc_2016-55.txt`) and TY2026 (`RevProc_2025-32.txt`)** — both procedures
   and both SSA determinations (`SSA_COLA_Determinations_{2017,2026}.txt`) are already committed, so
   neither year needs a from-memory figure either. TY2026 additionally needs OBBBA §70106 for the flat
   $15,000,000 §2010(c)(3) exclusion, which is *not* in the repo — collect it or cite the procedure's
   own §2.14 restatement.
3. **Make the unrendered-template failure loud, not silent.** Refuse to dispatch a brief still matching
   `\$\{`; and give each agent a distinct mandated report filename, since M-1 shows the collision ate
   two reports as well as two years.
4. **Give each agent its own file.** Three agents appending to two shared files is the B2
   shared-mutable-payload shape; "keep edits additive" is a discipline the filesystem does not enforce
   against a whole-file write.

*Adjudicated 2026-09-05. Scoped tests only; no `git`, no `make check`. The one file I mutated
(`crates/btctax-core/src/tax/testonly.rs`) was restored from a byte-identical backup and verified with
`diff -q`.*
