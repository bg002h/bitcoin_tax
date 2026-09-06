# scafffix — the field census gate is now year-general

**Agent:** scafffix-census · **Date:** 2026-09-05
**File owned and changed:** `crates/btctax-forms/tests/field_census.rs` (only file touched)
**Result:** 5 tests, 5 pass, **37 committed maps checked across years [2017, 2024, 2025]** (was: 17 maps, one year).

---

## 1. What was wrong — measured, not described

`field_census.rs` carried three year literals: `:105 let year = 2024;`, `:193 .join("2024")`,
`:261 map_and_census(stem, 2024)`. It also iterated a **hand-typed 17-stem list** (`CENSUSED`), so it
never touched the filesystem at all — no `forms/<year>/` directory could enter its loop no matter what
was committed.

### The control measurement

I ran the **unmodified original file** against today's tree:

```
        PASS (1/4) the_two_lists_partition_every_form
        PASS (2/4) the_uncensused_list_may_only_shrink
        PASS (3/4) recorded_gaps_may_only_shrink
        PASS (4/4) census_accounts_for_every_field
     Summary: 4 tests run: 4 passed, 0 skipped
```

**Green — while 733 AcroForm fields across two shipped years carry no recorded decision.** That is
`TY2026_PORT_REPORT.md` §2's *"the instruments fail OPEN"* in one run.

### The real state of the corpus (walked, per `(year, stem)`)

| year | maps | with `[census]` | 100 % accounted | **fields with NO recorded decision** |
|---|---|---|---|---|
| 2017 | 5 | **0** | 0 | **403** |
| 2024 | 17 | 17 | 17 | **0** |
| 2025 | 15 | 10 | 10 | **330** |
| | **37** | **27** | **27** | **733** |

Per form, the whole outstanding debt:

```
2017 f1040 254 · f8283 62 · f8949 8 · schedule_d 40 · schedule_se 39      (= 403)
2025 f1040 196 · f8283 63 · f8949 16 · schedule_d 40 · schedule_se 15     (= 330)
```

The brief's TY2025 numbers reproduced exactly (196/63/40/16/15 = 330).

Two facts the brief did not carry, both measured here:

* **★ NEW — TY2017 is worse than TY2025 and nobody had counted it.** TY2017 is a *fully wired,
  shipped* year (`btctax-forms/src/lib.rs:68 SUPPORTED_YEARS = &[2017, 2024, 2025]`) with **zero**
  census coverage and **403** unaccounted fields. The port report (line 121) noted "0 census
  sections" for 2017; the field count is new.
* **242 TY2025 census dispositions were read by nothing.** Summing the `[census]` FQNs in the ten
  censused TY2025 maps: 55+30+9+2+88+18+7+21+12+0 = **242**. The pinned gate never opened those
  files. They are now consumed by the gate on every run — and their `phantom` half (a disposition
  naming a field the TY2025 PDF does not have) is now checked too. Measured: 0 phantoms, 0
  contradictions, in every year.

---

## 2. What I changed

The year set is now **derived from the filesystem**, the same walk `map_pdf_conformance.rs` already
used — `read_dir(forms/)` → year dirs → `*.map.toml` stems. No year literal, no stem hand-list, no
numeric range anywhere in the file.

De-pinned, the gate goes **RED on 733 fields**, exactly as the brief predicted. I did not invent census
entries and did not exclude a year. The honest state is recorded as a **shrink-only register**:

```rust
const UNCENSUSED: &[(i32, &str, usize)] = &[ (2017,"f1040",254), … (2025,"schedule_se",15) ];
const UNCENSUSED_ENTRIES: usize = 10;
const UNCENSUSED_FIELDS: usize = 733;
```

**The register is the only allowance, and it is keyed on absence of a `[census]` section:**

| map state | gate demands |
|---|---|
| has a `[census]` section | **100 % accounted**, no allowance of any kind |
| has a `[census]` **and** a register line | **RED** — the stale excuse |
| no `[census]`, on the register | unaccounted count **== the recorded number, exactly** |
| **no `[census]`, not on the register** | **RED** — this is where a fresh `forms/2026/` lands |

That last row is the point of the whole change: **a newly committed year cannot buy itself a pass**,
because nothing on disk grants it an allowance. `TY2026_PORT_REPORT.md` §7 D6 ruled "de-pin onto
*wired* years" so the gate would not red on paused TY2025 work; the register does strictly better —
it fails closed on a new year the same way, **and** it carries TY2017's 403 and TY2025's 330 as
numbers in the suite rather than prose in a report nobody executes.

Two structural notes:

* **The gate's verdict is a pure function** `verdict(has_census, allowance, mapped, census, actual)
  -> Result<(), String>`, separated from the walk **so that B1 can be satisfied without mutating a
  committed map**. The filesystem walk is then plumbing.
* `map_and_census` now also returns whether the file declares a `[census]` section **at all**. An
  *empty* `[census]` is legitimate (TY2025 `f1040s1a` maps all 54 of its fields) and is not the same
  as having none — that distinction is what the register keys on.

### Test-by-test

| before | after | change |
|---|---|---|
| `census_accounts_for_every_field` (17 maps, 2024) | same name | walks **37 maps × 3 years**; unaccounted / both / phantom all year-general |
| — | **`the_gate_reds_on_every_planted_defect`** (new) | B1: seven planted defects, each watched Err with the right message |
| `recorded_gaps_may_only_shrink` (`forms/2024/`) | same name | scans **all 37 maps**. `GAPS` stays **0** — measured, not assumed |
| `the_two_lists_partition_every_form` | **`every_emittable_form_is_reached_by_the_gate_or_named_absent`** | the two hand-typed stem lists were year-blind. Now: for **each year on disk**, the `common::CENSUS_KEYS` forms *absent* from that year are named and pinned (`FORMS_ABSENT_FROM_YEAR`), so a deleted map reds instead of silently shrinking the gate's field of view. A year with no row reds |
| `the_uncensused_list_may_only_shrink` | **`the_uncensused_register_may_only_shrink`** | pins the register's entry count and field total, and rejects a register line naming a `(year, stem)` with no committed map |

Nothing was weakened. Every TY2024 assertion the original made still holds identically (the register
has no 2024 entries, so all 17 TY2024 maps are held to hard-0), and the "stale excuse" check the old
`the_uncensused_list_may_only_shrink` made is now the `(true, Some(n))` arm of `verdict`.

---

## 3. Which test reds for which planted defect

Every plant below was **actually run**; the quoted text is the real failure output. No committed
`*.map.toml` was modified — the map-level plants ran against a scratch `forms/` tree
(`2017/2024/2025` symlinked, plus a synthetic `2026/` copied from TY2025) with `forms_root()`
temporarily repointed. `git status --porcelain crates/btctax-forms/forms/` was empty afterwards.

| # | planted defect | test that RED | failure text (verbatim, trimmed) |
|---|---|---|---|
| 1 | register says 15 where `2025/f8949` has 16 unaccounted | `census_accounts_for_every_field` **and** `the_uncensused_register_may_only_shrink` | *"2025/f8949: recorded 15 unaccounted field(s), measured 16. The register is SHRINK-ONLY…"* / *"the register totals 732 unaccounted fields, pinned 733"* |
| 2 | delete the `(2025, "schedule_se", 15)` register line | `census_accounts_for_every_field` **and** `the_uncensused_register_may_only_shrink` | *"2025/schedule_se: this map has NO [census] section and NO entry on the UNCENSUSED register, so 15 field(s) carry no recorded decision and nothing says so."* |
| 3 | ★ **the target defect** — a fresh `forms/2026/` lands, half its maps carrying no `[census]` | `census_accounts_for_every_field` **and** `every_emittable_form_is_reached_by_the_gate_or_named_absent` | *"2026/schedule_d: … NO entry on the UNCENSUSED register, so 40 field(s) carry no recorded decision"* / *"years [2026] are on disk but have no row in FORMS_ABSENT_FROM_YEAR"* |
| 4 | gut the `(true, None)` arm of `verdict` (always `Ok`) | `the_gate_reds_on_every_planted_defect` | *"PLANTED DEFECT NOT CAUGHT — a field in NEITHER the map nor the census: the gate returned Ok. This checker does not exist (B1)."* |
| 5 | gut the `(false, None)` arm of `verdict` (always `Ok`) | `the_gate_reds_on_every_planted_defect` | *"PLANTED DEFECT NOT CAUGHT — an uncensused form with no register entry (a fresh year)…"* |
| 6 | delete **one** `[census]` entry (`f1_7[0]`, Form 6251 line 2c) from a fully-censused map | `census_accounts_for_every_field` | *"2026/f6251: 1 field(s) are in NEITHER the map nor the [census] — this is the \"we forgot this line\" defect…"* |
| 7 | add a **mapped** FQN (`f1_32[0]`, line 10) into that map's `[census]` too | `census_accounts_for_every_field` | *"…are BOTH mapped and censused — a field cannot be one we fill and one we deliberately leave blank"* |
| 8 | add a `[census]` entry naming `f1_9999[0]`, absent from the PDF (the year-port rename defect) | `census_accounts_for_every_field` | *"…are named by the map or census but do not exist in the PDF — a stale entry, which is how a closed gap silently reopens"* |

Plants 4 and 5 are the mutation-verification of the killer itself: `the_gate_reds_on_every_planted_defect`
carries seven synthetic plants (forgotten field, both-listed, phantom, fresh-year, count-rose,
count-fell, stale-allowance) and asserts each is `Err` **with the right message**, so gutting any one
arm of `verdict` reds it by name rather than generically.

Also verified: the new file's `census_accounts_for_every_field` asserts `checked >= 20` and
`every_bundled_year()` asserts `years.len() >= 3`, so a broken walk cannot pass by finding nothing —
the failure mode the old `checked > 0` guard was too weak to catch once the set became derived.

---

## 4. What I found and did NOT fix

1. **★ TY2017 — 403 unaccounted fields, zero census coverage, on a shipped year.** Now *recorded*,
   not closed. The maps are deliberate crypto-slice partials (`2017/f1040.map.toml` maps the line-13
   dollars+cents pair and nothing else), but "deliberately partial" and "nobody recorded a decision"
   are indistinguishable on the printed page — which is this file's own premise. `f1040` alone is
   254 of the 403. **This is the largest single census debt in the repo and it is on the oldest, most
   settled year**, i.e. the cheapest one to close, since TY2017's forms will never change again.
2. **TY2025 — 330 unaccounted across 5 maps.** Recorded, not closed, per §7 D6 (paused work).
3. **The register pins a COUNT per `(year, stem)`, not the identity of the fields.** Inside an
   *uncensused* form a simultaneous swap — one field newly mapped, another newly unmapped — keeps the
   count and passes. Censused forms have no such hole (they are held to exactly 0), and `phantom` +
   `both` are hard-checked in every year, so the exposure is confined to the ten register lines. A
   sharper form would pin a sorted digest of the unaccounted FQNs; I judged that beyond the brief and
   note it here instead.
4. **`common::CENSUS_KEYS` cannot express the Schedule 1 → Schedule 1-A substitution.** TY2025 ships
   `f1040s1a` (54 fields, 54 mapped, empty `[census]`, fully accounted) and no `f1040s1`.
   `CENSUS_KEYS` is a flat year-agnostic list, so `FORMS_ABSENT_FROM_YEAR` records `f1040s1` as absent
   from 2025 without recording that `f1040s1a` replaces it. The gate still *walks* `f1040s1a` (the set
   is filesystem-derived), so nothing escapes — but the emittable-form authority is year-blind.
   `crates/btctax-forms/tests/common/mod.rs` is not my file; recorded as a comment in the register.
5. **Two extractors for one FQN predicate.** `field_census.rs::map_and_census` accepts a string with
   `"[0]"` and `'.'`; `map_pdf_conformance.rs::field_names` accepts `'['` and `'.'` and no space.
   Two halves of one invariant, two definitions of "is this an FQN". I checked the corpus for a
   divergence — `grep -ho '"[^"]*\[[^"]*"' forms/*/*.map.toml | grep '\.' | grep -v '\[0\]'` returns
   **nothing**, so they agree on today's 37 maps (including the IRS-glitched `f1-_51[0]` on the 2017
   1040). A TY2026 map naming e.g. `Row2[1]` would part them. `map_pdf_conformance.rs` is not my file.
6. **`FORMS_ABSENT_FROM_YEAR` requires a row per year, and a new year with no row reds.** That is
   deliberate friction, not a defect — but a TY2026 port must add a row, so it belongs on the port
   runbook next to the `UNCENSUSED` register.

---

## 5. Verification run

```
$ cargo nextest run -p btctax-forms --test field_census --no-capture
field census: 37 committed maps checked across years [2017, 2024, 2025]
complete years (every emittable form mapped): [2024]
     Summary [0.286s] 5 tests run: 5 passed, 0 skipped
```

No compiler warnings. Scoped to `-p btctax-forms --test field_census` throughout (seven agents share
this worktree); no `git` command was run and no file outside
`crates/btctax-forms/tests/field_census.rs` was left modified.
