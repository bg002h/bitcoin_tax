# Validated tax table — TY2025 (Rev. Proc. 2024-40)

**Agent task:** transcribe TY2025's validated counterpart into `btctax_core::tax::testonly`, wire it
into `shipped_tables_are_the_validated_tables.rs`, and report disagreements with `tax_tables.rs`.

**Status:** transcribed, wired, green — **0 disagreements** with the shipped table.

**★★★ Read §0 first.** The orchestration fanned out three agents onto the *same* year. This report is
one of three TY2025 transcriptions; TY2017 and TY2026 were left untouched by all three.

---

## §0. ORCHESTRATION DEFECT — the fan-out collapsed onto one year

My brief arrived with its template placeholders **unrendered**:

```
## Your year — TY${t.y}
  source text : `legal/text/irs-guidance/RevProc_${t.proc}.txt`
  note        : ${t.note}
```

`${t.y}`, `${t.proc}` and `${t.note}` were never substituted. With no year assigned, I derived the
candidate set from the repo ({2017, 2025, 2026} — the years in `KNOWN_UNVALIDATED_TABLE_YEARS`) and
picked the one the test file's own header names as **"the live exposure"** (8/28 bracket thresholds
witnessed, versus 28/28 for TY2017 and 24/28 for TY2026).

**All three agents made the same choice, for the same documented reason.** Measured on disk at
20:15 — three `ty2025_table()` definitions in `testonly.rs` at lines 983, 1126 and 1289, i.e. a
`E0428` duplicate-definition red. A peer subsequently deduplicated to one.

I attempted to avoid this with an atomic `mkdir` claim under the shared scratchpad
(`scratchpad/vtclaim/<year>`); the directory ended with only `2025` in it, so the other two agents
never used that protocol — as expected, since nothing in the brief established one.

**Consequences for the parent:**

| year | shipped | validated counterpart | state after this round |
|---|---|---|---|
| 2017 | yes | **no** | still in `KNOWN_UNVALIDATED_TABLE_YEARS`, still unwitnessed |
| 2024 | yes | yes (pre-existing) | unchanged |
| 2025 | yes | **yes — closed 3× over** | closed; removed from the ratchet list |
| 2026 | yes | **no** | still in `KNOWN_UNVALIDATED_TABLE_YEARS`, still unwitnessed |

Two thirds of the round's budget bought a redundant answer. **TY2017 and TY2026 still need doing.**

### §0.1 The accidental upside — a genuine three-way independent check

Three agents transcribing the same procedure from the same text layer, without seeing each other's
work, is a stronger witness than the task asked for. I extracted all three definitions before the
deduplication and compared them mechanically:

```
found 3 definitions at lines [983, 1126, 1289]
THREE-WAY COMPARISON: 12 figure groups AGREE across all 3 independent transcriptions
DISAGREEMENTS: none — all three independent readings are identical
```

All 28 ordinary thresholds, all 8 §1(h) breakpoints and all 3 scalars agreed across three independent
readings — **and then agreed with the shipped table as well.** Four-way unanimity on TY2025.

The three `source` strings differed, which is correct and required: `compare_tables` deliberately
excludes `source` from the equality precisely so the two artifacts cite different provenance.

---

## §1. Figures transcribed, with citations

Source document: `legal/text/irs-guidance/RevProc_2024-40.txt` (committed `pdftotext -layout` extract).

### §1.1 Ordinary rate schedules — **Rev. Proc. 2024-40 §2.01, Tables 1–4, §1(j)(2)(A)–(D)**

Thresholds are the income at which each marginal rate **starts**.

| rate | Single (Table 3, §1(j)(2)(C)) | MFJ (Table 1, §1(j)(2)(A)) | MFS (Table 4, §1(j)(2)(D)) | HoH (Table 2, §1(j)(2)(B)) |
|---|---|---|---|---|
| 10% | $0 | $0 | $0 | $0 |
| 12% | $11,925 | $23,850 | $11,925 | $17,000 |
| 22% | $48,475 | $96,950 | $48,475 | $64,850 |
| 24% | $103,350 | $206,700 | $103,350 | $103,350 |
| 32% | $197,300 | $394,600 | $197,300 | $197,300 |
| 35% | $250,525 | $501,050 | $250,525 | **$250,500** |
| 37% | $626,350 | $751,600 | **$375,800** | $626,350 |

Table 5 (§1(j)(2)(E), Estates and Trusts — $3,150 / $11,450 / $15,650) has no `FilingStatus` and is
**not modelled**. Stated here so its omission is a recorded decision, not a silent gap.

**Two boundaries that look like typos and are not:**

- **HoH's 35% floor is $250,500, not $250,525.** A genuine $25 split from Single/MFS.
- **MFS's 37% floor is $375,800** — half the joint $751,600, not Single's $626,350. That single row is
  the reason a copy-paste closure of this year would have been worthless.

**Every threshold is arithmetically self-checked.** Each table prints a redundant second encoding —
the running *"$X plus Y% of the excess over Z"* column — so each boundary was confirmed by recomputing
the printed cumulative tax from the preceding brackets. All 28 agree. Worked examples:

- HoH 35% floor: `38,460 + 0.32 × (250,500 − 197,300) = 55,484` ✓ matches the printed "$55,484 plus
  35%". $250,525 would give $55,492, so the reading is pinned by arithmetic, not by re-reading.
- Single 37% floor: `57,231 + 0.35 × (626,350 − 250,525) = 188,769.75` ✓ matches "$188,769.75 plus 37%".
- MFS 37% floor: `57,231 + 0.35 × (375,800 − 250,525) = 101,077.25` ✓ matches "$101,077.25 plus 37%".
- MFJ 37% floor: `114,462 + 0.35 × (751,600 − 501,050) = 202,154.50` ✓ matches "$202,154.50 plus 37%".

This is the Form 6251 line-33 failure mode from `CLAUDE.md` — a rendered `12` versus `22` — caught by
arithmetic rather than by a second read. (For contrast: Rev. Proc. **2023-34** carried a real typo in
exactly this column for TY2024, recorded in `ty2024_table`'s comments. Rev. Proc. 2024-40's two
columns are mutually consistent throughout.)

### §1.2 §1(h) capital-gain breakpoints — **Rev. Proc. 2024-40 §2.03 (§1(h), §1(j)(5)(B))**

| procedure's row label | FilingStatus | Maximum Zero Rate Amount | Maximum 15% Rate Amount |
|---|---|---|---|
| "All Other Individuals" | Single | $48,350 | $533,400 |
| "Married Individuals Filing Joint Returns and Surviving Spouse" | Mfj | $96,700 | $600,050 |
| "Married Individuals Filing Separate Returns" | Mfs | $48,350 | **$300,000** |
| "Heads of Household" | HoH | $64,750 | $566,700 |
| "Estates and Trusts" | — | $3,250 | $15,900 | 

The Estates and Trusts row has no `FilingStatus` and is not modelled — recorded, not forgotten.

**MFS's 15% ceiling is $300,000, not half of the joint $600,050 ($300,025).** §1(j)(5)(B) rounds each
status independently; the halving identity that holds for the MFS *ordinary* 37% floor does **not**
hold here. A reviewer "fixing" this to $300,025 would introduce a defect.

### §1.3 Scalars

| field | value | citation |
|---|---|---|
| `gift_annual_exclusion` | $19,000 | **Rev. Proc. 2024-40 §2.43(1)** — §2503 |
| `gift_lifetime_exclusion` | $13,990,000 | **Rev. Proc. 2024-40 §2.41** — §2010(c)(3) basic exclusion |
| `ss_wage_base` | $176,100 | **NOT in the revenue procedure** — see below |

**★ `ss_wage_base` is sourced, not assumed.** The brief anticipated it might have to be entered
unsourced. It did not: the repo commits the Federal Register extract at
`legal/text/federal-register/SSA_COLA_Determinations_2025.txt` — **89 FR, Vol. 89 No. 207, Friday,
October 25, 2024**, SSA "Cost-of-Living Increase and Other Determinations for 2025", under the heading
*"OASDI Contribution and Benefit Base"*:

> "The OASDI contribution and benefit base is $176,100 for remuneration paid in 2025 and
> self-employment income earned in tax years beginning in 2025."

Corroborated in the same document at the computation paragraph: *"Because $176,100 exceeds the current
base amount of $168,600, the OASDI contribution and benefit base is $176,100 for 2025."*

I cited the issue's first page (85276); a peer's surviving version cites **85279**, which is the more
precise page for that sentence (page headers in the extract run 85276 → 85277 → 85278 before the OASDI
section). **Prefer the peer's 85279.**

### §1.4 `Qss` — deliberately absent

`Qss` is omitted from both `ordinary` and `ltcg`, matching `ty2024_table`. §1(j)(2)(A)/Table 1 is
titled "Married Individuals Filing Joint Returns **and Surviving Spouses**", and `TaxTable::key`
normalises `Qss → Mfj` at lookup. `is_lawful_absence` permits this **only when both sides omit it** —
so this is checked by the ratchet, not assumed. It passed, which confirms the shipped side omits it too.

---

## §2. DISAGREEMENTS with `tax_tables.rs`

**None.** `crates/btctax-adapters/src/tax_tables.rs::ty2025()` (line 443) was read only after the
transcription was complete and wired.

Every compared field matches: all 28 ordinary bracket thresholds and rates across four statuses, all
8 §1(h) breakpoints, and all three scalars. `Qss` is absent from both sides (lawful). Confirmed both
by eye and by `every_shipped_tax_table_equals_the_one_the_corpus_validates` passing with TY2025 now in
the loop.

**The shipped TY2025 table is correct.** Its numbers were previously asserted by almost nothing
(8/28 thresholds); they are now bound to an independent transcription.

### §2.1 Two non-defects worth recording

Neither is a numeric disagreement; neither blocks.

1. **(Nit) Rate-table subsection cites in `tax_tables.rs` comments.** The shipped comments label the
   schedules "§1(c) rate table" (Single), "§1(a)" (MFJ), "§1(b)" (HoH), "§1(d)" (MFS). Rev. Proc.
   2024-40 publishes them under **§1(j)(2)(C)/(A)/(B)/(D)** — §1(j)(2) is what supplies the TY2018–2025
   figures, overriding the §1(a)–(d) base tables. Both cites are defensible (§1(j)(2) modifies §1(a)–(d));
   the procedure's own numbering is more precise. Cosmetic only.

2. **★ (Scope limit, not a finding) The shipped `source` string asserts something I did not verify.**
   It reads:

   > `"Rev. Proc. 2024-40 §2.01/§2.03 + §2.43 + §2.41 (TY2025); OBBBA Pub. L. 119-21 left 2025
   > brackets/breakpoints unchanged"`

   The trailing clause is **outside Rev. Proc. 2024-40** and therefore outside what this transcription
   witnesses. My work establishes *"the Rev. Proc. 2024-40 figures are correctly transcribed"*; it does
   **not** establish *"no later enactment superseded them."* The repo has considered this
   (`design/SPEC_ty2024_tables.md:258` — "keep the OBBBA (Pub. L. 119-21) note scoped to TY2025"), so I
   am flagging the boundary rather than reopening it. Stated because a green ratchet could otherwise be
   read as blessing the whole `source` string, and `source` is explicitly **excluded** from the equality.

---

## §3. Changes made

- `crates/btctax-core/src/tax/testonly.rs` — appended `ty2025_table()` (via `>>`, so a concurrent peer
  append could not clobber it). **A peer's figure-identical version is the one that survived
  deduplication**; it is at line 983 and is at least as well documented as mine, so I left it standing
  rather than re-litigating an identical table.
- `crates/btctax-adapters/tests/shipped_tables_are_the_validated_tables.rs` — three surgical edits,
  each written to tolerate a peer having already made it:
  - import `ty2025_table`;
  - `validated_table_for`: added `2025 => Some(ty2025_table()),`;
  - `KNOWN_UNVALIDATED_TABLE_YEARS`: `&[2017, 2025, 2026]` → `&[2017, 2026]` (removed **only** 2025).

  All three were already present when my script ran — a peer had made the identical edits first. The
  script asserted no-change and wrote nothing, which is how the collision was detected.

`validated_params_for` was **not** touched: `full_return_for(2025)` is a deliberate `None`
(`ty2025_full_return_must_stay_fail_closed_until_complete`, `tax_tables.rs:814`), so TY2025 ships no
full-return params and the params ratchet has nothing to compare. Out of scope, by design.

---

## §4. Verification

```
cargo nextest run -p btctax-adapters -E 'binary(shipped_tables_are_the_validated_tables)'
    Summary [0.003s] 9 tests run: 9 passed, 0 skipped
```

Including `every_shipped_tax_table_equals_the_one_the_corpus_validates` and
`every_shipped_year_has_a_validated_counterpart`.

**Is TY2025 actually load-bearing, or is this green-and-blind?** Two facts settle it without a mutation
of a file two peers were still writing to:

1. The ratchet's `unexpected` assertion reds on any shipped year that has **no** validated table and is
   **not** in `KNOWN_UNVALIDATED_TABLE_YEARS`. I removed 2025 from that list and the test passed —
   which is only possible if `validated_table_for(2025)` returns `Some`. TY2025 is therefore in the
   comparison loop.
2. The file's own planted-defect test `a_single_moved_bracket_is_detected` passed, so the comparison
   mechanism is observed discriminating on a moved bracket (the B1 "seen-red-once" property, already
   held by this file).

Together: the figures above are compared, and the comparison can fail. No mutation of the shared file
was performed — two peer agents were editing it concurrently, and a `cp`-restore window risked
clobbering their writes.

`make check` and workspace runs were **not** run, per the brief. No `git` command was run, per the brief.

---

## §5. What the parent should do next

1. **Re-dispatch TY2017 and TY2026** — the actual remaining gap. Both are still in
   `KNOWN_UNVALIDATED_TABLE_YEARS`. Sources are committed:
   `legal/text/irs-guidance/RevProc_2016-55.txt` (TY2017) and `RevProc_2025-32.txt` (TY2026), with SSA
   wage bases in `legal/text/federal-register/SSA_COLA_Determinations_2017.txt` and `_2026.txt`.
   TY2026 additionally needs OBBBA Pub. L. 119-21 §70106 for the flat $15,000,000 §2010(c)(3) exclusion
   — a statutory figure, not a §1(f) indexed one (`design/SPEC_tax_tables_2026.md:47`).
2. **Fix the template interpolation** before re-dispatching, and put the year in the *agent name* or the
   report filename the brief mandates, so an unrendered variable fails loudly instead of silently
   collapsing N agents onto one year.
3. **Confirm `testonly.rs` holds exactly one `ty2025_table()`** after all three agents have exited. It
   did at the time of writing (line 983); a late peer append would reintroduce `E0428`.
