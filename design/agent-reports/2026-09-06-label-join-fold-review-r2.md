# Re-review of the fold — the label-join fold review, folded (`91070215`)

Reviewer: Claude Opus 5 (1M), independent, read-only. Date: 2026-09-06. HEAD: `91070215`, branch `main`.

Scope: the ONE question — does `91070215` resolve L1–L8 of
`design/agent-reports/2026-09-06-label-join-fold-review.md` (0C/4I/4M) without introducing a new
Critical or Important, and does every checker it adds actually discriminate (B1)?

Taken as SETTLED, not re-derived: the two-witness design; the owner rulings; that R2 was real; the
nine claims of `…-VERIFICATION.md`.

Read: `git show 91070215` (all six files); `crates/xtask/src/label_reader.rs`
(`label_join`, `inline_label_words`, `witness_boxes_x`, `numbered_line_keys`, `GRID_MAPS`,
`map_reach_problem`, `line_bindings`, `label_matches`, `YEAR_FLOORS`, `audit_year_reach`, the three
new tests); `crates/xtask/src/form_delta.rs` (`the_committed_work_list_matches_form_delta_at_head`,
`parse_work_list_row`); `crates/xtask/src/form_geometry.rs` (`Word`, `Box_`, `box_top_down_y`);
`crates/btctax-forms/tests/field_census.rs`
(`every_emittable_form_is_reached_by_the_gate_or_named_absent`, `stems_for`);
`crates/btctax-forms/src/packet.rs`; `crates/btctax-forms/src/bundled.rs` (`Stem::ALL`,
`periodic_template`); all 36 `crates/btctax-forms/forms/*/*.map.toml`; `design/TY2026_WORK_LIST.md`;
`design/ROADMAP_STATUS.md` §2.

## Gate numbers, reproduced

One run, captured once to `/tmp/…/lj-rereview.txt` and grepped (never twice):

```
cargo nextest run --locked -p xtask -E 'test(label) | test(work_list) | test(map_reach) | test(planted) | test(checkbox)' --no-capture
Summary [0.292s] 17 tests run: 17 passed, 109 skipped
```

| commit message says | this run says | verdict |
|---|---|---|
| 2024: 17 map(s), 249 join(s), 1 unreachable | `2024: 17 map(s), 249 join(s) checked, 1 unreachable` | **matches** |
| 2025: 15 map(s), 201 join(s), 0 unreachable | `2025: 15 map(s), 201 join(s) checked, 0 unreachable` | **matches** |
| 0 wrong bindings | `wrong` assert green; `0 binding(s) landed on a box the reader could not label ("?")` | **matches** |
| work list: 13 compared / 5 excused / 18 stems | `work list: 13 compared, 5 excused, 18 stems on the emitting surface` | **matches** |

Per-map joined sums cross-check the two effects the brief asked to separate: 2024 per-map joins sum
to **249**, of which `schedule_d` contributes 23 (was 9) → **249 − 14 = 235**, the review's y2-rule
figure; 2025 sums to **201**, `schedule_d` 11 (was 3) → **201 − 8 = 193**. So L1 moved nothing and
L4 moved everything, exactly as the fold claims.

Two further runs (different suites, not repeats):
`cargo nextest run --locked -p btctax-forms -E 'test(census)|test(year_record)|test(packet)'` →
**20/20 pass**; `cargo check --workspace --all-targets --locked` → **exit 0** (so L7 breaks nothing).

## Checklist — L1–L8 and the kills

| item | verdict | evidence |
|---|---|---|
| **L1** real bottom edge | **RESOLVED** | `label_reader.rs:1076` is `*ly2 <= *bottom`; `inline_label_words` now carries `w.y2`. Measured on the fixture (below): the kill reds under the old predicate. |
| **L2** reach predicate + plant | **RESOLVED** (see N5, Minor) | `map_reach_problem` is pure, planted on 5 shapes + 1 healthy; `numbered_line_keys` never touches the RHS, so the R2 mechanism cannot blind it. |
| **L3** work list regenerated + held | **RESOLVED** (see N6, Minor) | Table now 23/2/2/0; f6251 row reads `unchanged`; `the_committed_work_list_matches_form_delta_at_head` recomputes every numeric row and passes at HEAD. |
| **L4** inline-table bindings | **PARTIAL** | Inline tables `{ … }` now yield one entry per FQN (+14/+8, verified against `2024/schedule_d.map.toml:20-24`). But a numbered line bound as a **`[lineN]` TABLE SECTION** is still dropped silently, by *both* the parser and the new key counter — **N1**. |
| **L5** grid blank recorded | **PARTIAL** | `GRID_MAPS` exists and the TY2025 comment is corrected. But its recorded reason for `f8283` is **false**: `2024/f8283.map.toml:73,76,79` binds `[line5a]/[line5b]/[line5c]` — **N2**. |
| **L6** structural absence rule | **RESOLVED** (see N8, Nit) | `field_census.rs:620-637` now consults `periodic_template(stem, *year)` and `first_bundled_year(stem) > *year`; no stem list. Walked below. |
| **L7** `Default` dropped | **RESOLVED** (see N9, Nit) | `packet.rs:52` is `#[derive(Debug, Clone)]`; workspace check green; no `FiledPacket::default()` anywhere. |
| **L8** 12pt gap documented | **RESOLVED** | `label_reader.rs:1066-1070` states 11.30 accepted / 12.30 rejected and that the constant is calibrated, not derived. The review permitted keeping the constant. |
| **kills** — 2a/2b swap committed | **RESOLVED** | `a_planted_2a_2b_swap_reds_both_boxes` exists and cannot pass under the column-only rule (see (h)). |
| **kills** — checkbox plant | **RESOLVED** | `a_checkbox_option_numeral_is_not_its_line_label` reds under the pre-fold predicate (see (a)). |
| **kills** — `per_map_zero` had none | **RESOLVED** | `map_reach_problem_reds_on_every_planted_shape`, including the exact R2 shape `(19, 0, 0)`. |

## Verdicts (a)–(h)

### (a) L1 — `*ly2 <= *bottom`, and does the kill red under the OLD rule? **YES; and no join is lost.**

Measured directly from `design/forms/geometry/f1040--2024.json` (no repo file was touched; the
predicate was evaluated in a scratch Python script over the committed fixture):

| box | left | top | bottom | height | candidate word | word y / y2 / height | gap | OLD `ly+4≤bottom` | NEW `ly2≤bottom` |
|---|---|---|---|---|---|---|---|---|---|
| `topmostSubform[0].Page2[0].c2_1[0]` | 297.20 | 38.00 | 46.00 | **8.00** | `"1"` | 37.91 / **47.45** / 9.54 | 4.75 | `41.91 ≤ 46` → **admits** | `47.45 ≤ 46` → **rejects** |
| `…Page2[0].c2_2[0]` | 347.60 | 38.00 | 46.00 | 8.00 | `"2"` | 37.91 / 47.45 / 9.54 | 4.75 | admits | rejects |
| `…Page2[0].c2_3[0]` | 398.00 | 38.00 | 46.00 | 8.00 | `"3"` | 37.91 / 47.45 / 9.54 | 4.75 | admits | rejects |

Only one in-row candidate exists per box, so `max_by(x2)` does not intervene: under the old
predicate `label_join` returns `"1"`, and `assert_eq!(got, "16")` **fails**. Under the new predicate
the in-row rule does not fire, the column rule supplies `"16"`, and the test passes (it does, at
HEAD). B1 satisfied: the plant reds on the exact defect it was written for.

No join lost: every reachable map reports `extracted == joined` in both years, `unlabelled` is 0, and
the year totals are exactly the review's 235/193 plus L4's +14/+8. The two effects are separable and
separated.

Observation (not a finding): the plant pins **one box**. The property that actually changed — a label
word taller than the box can never be that box's label — is held at a single point. A future
loosening that admitted, say, a 9.5pt word into a 9.0pt box would not be caught here.

### (b) L2 — `numbered_line_keys` / `map_reach_problem` / `GRID_MAPS`. **Independent and discriminating, with one blind shape.**

*Independence.* `numbered_line_keys` splits on the first `=` and inspects only the key, so
`line7a = "…" # comment` counts (the key is `line7a`; R2's mechanism lived entirely in the RHS) and
`line3 = { proceeds_d = "…", … }` counts once. Re-implemented and run over all 36 maps: it agrees
with the test's own printed counts on every one. Reintroducing R2 would leave keys at 19 and drive
extracted to 0 → red. **Genuinely independent on the axis that matters.**

*Plant covers R2 exactly.* `map_reach_problem("f1040sa", 19, 0, 0).is_some()` — 19 is TY2025 Schedule
A's real key count (measured: `2025/f1040sa` = 19 keys), and `extracted == 0` is R2's shape.

*`GRID_MAPS` measurement.* Measured over every map: `f8949` (2017/2024/2025), `f8283` (2017/2024/2025)
and `f8275` (2024) all report **0** numbered keys, and every other map reports ≥ 1 (minimum 1:
`2017/f1040`, `2025/f1040`). So the constant is right **as `numbered_line_keys` measures it** — but
that counter is blind to `[lineN]` sections, and `2024/f8283` has three of them. See **N2**.

*Blind shape:* a PARTIAL drop (`keys = 19, extracted = 1`) returns `None`. See **N5**.

### (c) L3 — `the_committed_work_list_matches_form_delta_at_head`. **Sound; the excuse test is weaker than its doc comment.**

- **Both tables parsed.** `parse_work_list_row` needs `cells.len() >= 6` and a backticked `cells[1]`.
  The numeric table (6 columns → 8 split elements) yields `Some(cells)`; the "Not listed" table
  (5 columns → 7 elements) has a non-numeric `cells[2]` → `None` → excused. 13 + 5 = 18 rows, and the
  run prints `13 compared, 5 excused`. Header and separator rows are rejected on the backtick test.
- **The excuse test.** `(None, Ok(_))` reds — an excused row that *does* have a pair is caught, which
  is the direction that matters (a draft lands, the row must become numeric). Verified all five
  excuses are true in fact: no `f1040--2026-DRAFT`, `f1040s1--2025`, `f8283--2026-DRAFT`,
  `f8275--2026-DRAFT` or `f8995a--2025` fixture exists in `design/forms/geometry/`. But the test
  checks only *"no pair"*, never the row's stated reason, so f1040's specific excuse (its draft was
  the TY2025 form) is not held — see **N6**.
- **Denominator.** Built from every `irs_stem` across all bundled maps: measured **18 distinct**
  (`f1040, f1040s1, f1040s1a, f1040s2, f1040s3, f1040sa, f1040sb, f1040sc, f1040sd, f1040sse, f6251,
  f8275, f8283, f8949, f8959, f8960, f8995, f8995a`). `Stem::ALL` (`bundled.rs:64-83`) is **18**, and
  the two coincide member-for-member under `schedule_d ↔ f1040sd`, `schedule_se ↔ f1040sse`. So the
  denominator does equal Stem::ALL's 18 today, derived rather than asserted.
- **Both deletion directions red.** A deleted *row* → the stem is in `stems` but not `listed` →
  `missing` reds. A deleted *map* → `stems.len()` falls to 17 → `stems.len() >= 18` reds.
- **The inline plant** is a real discriminator but a narrow one: it proves the comparator
  distinguishes `(62,0,0,1)` from the computed `(62,0,0,0)`. It never pushes a wrong row through the
  loop, so it does not observe `wrong` being populated or `assert!(wrong.is_empty())` firing. Not a
  tautology; not a full kill either — **N6**.

### (d) L4 — `rhs.split('"').skip(1).step_by(2)`. **Exact on today's maps; mis-pairable in principle.**

On `forms/2024/schedule_d.map.toml:20-24` the split yields exactly the FQNs and nothing else:
`line1a` → 3, `line8a` → 3, `line3` → 4, `line10` → 4 = **14**; `2025/schedule_d.map.toml:12,14` →
4 + 4 = **8**. Those are the +14/+8 the floors moved by. `qof_yes = { field = "…", on = "1" }` is
skipped (key does not start with `line`), and `on = "1"` would be filtered anyway by
`contains('[') && contains('.')`.

Mis-pairing is possible on (i) a value containing `\"` and (ii) a trailing `#` comment that itself
contains quotes — either shifts the odd/even pairing. Measured: **zero** `\"` inside any `lineN`
value (the only escaped quotes in the tree are inside `[census]` `reason` strings, whose keys are
quoted FQNs), and **zero** `lineN = { … } # …` lines. A `#` *inside* a string is harmless because
comments are only stripped at line start. The failure mode is a phantom binding → a false FAIL, never
a false PASS. Recorded as **N7 (Nit)**.

### (e) L6 — the census rule, walked. **Correct today, and it closes the case the review named.**

`years` = {2017, 2024, 2025}; only TY2024 is `Filable`.

- **TY2024.** `stems_for(2024)` holds 17 map stems; `measured = {f1040s1a}`.
  `periodic_template(F1040s1a, 2024)`: no own template, newest bundled year is 2025 whose map is
  `versioning = annual` → `None`. `first_bundled_year("f1040s1a") = 2025 > 2024` → **allowed**.
  Correct, and for the stated reason.
- **The case the review said the hand-list wrongly excused.** If TY2025 were filable and `f1040s1a`
  were absent from `forms/2025/`, `first_bundled_year` would find no year holding it → `None` →
  `is_some_and` false; periodic false → **RED**. Fixed.
- **Would TY2025-as-filable red today?** `measured(2025) = {f1040s1, f8275, f8995a}`. `f8275` is
  periodic → excused (correct). `f1040s1` and `f8995a` are annual with first bundled year 2024 →
  **red** (correct; both are the "NO PRIOR SIDE" rows of the work list).
- **A stem newly excused that the hand-list did not excuse?** No. The rule excuses exactly the
  periodic stems (`f8275`, `f8283` — both were in the hand-list) plus stems whose first bundled year
  is later than the year (today only `f1040s1a` on 2024, also in the hand-list). It is a strict
  narrowing.
- Residual proxy weakness once a fourth year lands: **N8 (Nit)**.

### (f) L7 — `Default` dropped. **Nothing breaks.**

`cargo check --workspace --all-targets --locked` exits 0; `grep` finds no `FiledPacket::default()`
and no out-of-crate literal. The remaining external path to an unsorted packet is
`let mut p = FiledPacket::stapled(…); p.forms.push(…)` — the fields are still `pub`. The new comment
does not claim otherwise and the review did not ask for it to close; recorded as **N9 (Nit)**.

### (g) The floors 249/201 — **measured, but only one of the two ratchet comments says why it moved.**

Both values reproduced above from the test's own output. `YEAR_FLOORS`'s 2025 entry explains the
move (`"then 193 → the value below when Schedule D's inline-table rows joined (fold review L4)"`).
The **2024 entry does not**: `label_reader.rs:1393-1396` still reads *"99 → 235 on 2026-09-06 … 
Measured, not estimated."* directly above `min_joins: 249` — **N4**. And `design/ROADMAP_STATUS.md:178`
still publishes *"floors 235 (2024) / 193 (2025)"* — **N3**.

### (h) `a_planted_2a_2b_swap_reds_both_boxes` — **it cannot pass without the x-aware rule.**

Under the column-only rule both the `2a` and `2b` cells read `"2a"` (the fold review measured this;
it is the defect the x-aware rule was written for), so the committed map already carries 2 wrong
bindings — and the test's *first* assertion is `count_wrong(&map) == 0`. That fails before the plant
is applied. Under the x-aware rule the swap gives `line2a` → a box printing `2b` (wrong) and `line2b`
→ a box printing `2a` (`label_matches("2b","2a")`: `"2b" ≠ "2a"`, trimmed `"2" ≠ "2a"` → wrong), so
`count_wrong == 2`. The test therefore discriminates in both directions, and is the committed form of
the plant `ROADMAP_STATUS.md:179` had only asserted.

## New findings

### N1 — IMPORTANT — a numbered line bound as a `[lineN]` TABLE SECTION is dropped by BOTH the parser and the new key counter, so `map_reach_problem`'s stated guarantee is false

**Where:** `crates/xtask/src/label_reader.rs:1107-1117` (`numbered_line_keys`) and `:1156-1195`
(`line_bindings`) — both iterate `l.split_once('=')`, and a TOML table header (`[line17]`) has no `=`.

**What is wrong:** `numbered_line_keys`'s doc comment says it counts keys *"on the raw text with no
parsing of the right-hand side — so a parser that drops a binding cannot also hide the key it
dropped."* That is exactly what happens for a section-bound line: the key is invisible to the counter
*and* the FQNs are invisible to the parser, so `keys`, `extracted` and `joined` all move together and
`map_reach_problem` returns `None`. This is L4's finding — *"a numbered binding … dropped the same
way, with no comment, no counter and no reason"* — in the shape the fold did not cover, and it means
the per-map reach line still mis-states its own reach.

Measured over the committed maps (`grep -nE '^\s*\[line[0-9]'` plus FQN extraction, then resolved
against `xtask label-boxes`):

| map | sections | FQNs | printed label the join gives | checked today |
|---|---|---|---|---|
| `2024/f1040sb` | `[line7a]`, `[line7a_fbar]`, `[line8]` | 6 | `7a 7a 7a 7a 8 8` | **no** |
| `2025/f1040sb` | same three | 6 | `7a 7a 7a 7a 8 8` | **no** |
| `2024/schedule_d` | `[line17]`, `[line20]`, `[line22]` | 6 | `17 17 20 20 22 22` | **no** |
| `2025/schedule_d` | `[line17]` | 2 | `17 17` | **no** |
| `2025/f1040s1a` | `[line22a]`, `[line22b]` | 6 | `22a 22a 22a 22b 22b 22b` | **no** |
| `2024/f8283` | `[line5a]`, `[line5b]`, `[line5c]` | 6 | (map unreachable — no TY2024 geometry) | **no** |
| `2017/schedule_d` | `[line17]` | 2 | (year unreachable) | **no** |

**26 reachable FQNs on five live maps**, every one of which resolves to a label `label_matches`
accepts — so, exactly as with L4, these are free joins the gate is not taking. The forward risk is
the sharper half: every one of them is a **Yes/No checkbox pair or a sub-row cell**, which is the
precise class L1 was about, and it sits on `f1040s1a` — the newest map, and the one step 5 wired.
A wrong FQN in any of these sections would be silent today.

**Minimal change:** in `line_bindings`, track the current `[section]` header; when it matches
`line<digit>…`, emit one `(line, FQN)` per `"…[…]….…"` value inside it until the next header — and
count the header in `numbered_line_keys` the same way. Ratchet the floors by the +26 that follows.
If that is out of scope, at minimum count section-bound lines and print them
(`N line(s) bound by a [lineN] section — not joined`), which is the alternative L4 itself offered.

### N2 — IMPORTANT — `GRID_MAPS` records a FALSE reason for `f8283`, which is worse than the silent blank L5 asked it to replace

**Where:** `crates/xtask/src/label_reader.rs:1119-1123`.

**What is wrong:** the constant's doc says these maps *"hold NO numbered line key BY DESIGN —
repeating grids whose rows are addressed positionally and carry no single printed label … (measured
2026-09-06: 0 numbered keys each)"*, and `map_reach_problem` reds *"a grid map grew a numbered key."*
For `f8283` that is false: `crates/btctax-forms/forms/2024/f8283.map.toml:73,76,79` binds
`[line5a]`, `[line5b]` and `[line5c]` — six FQNs — and the map's own comment calls them
*"Section B lines 5a/5b/5c — the RESTRICTION questions"*. They read as zero only because
`numbered_line_keys` cannot see a section (N1).

This inverts L5's own invariant. L5 asked that *"this map encodes no numbered line"* and *"this map's
bindings were all dropped"* stop being the same blank; the fold's answer records a reason, and for
one of its three entries the recorded reason is untrue. The `f8283` guard also cannot fail in the
direction it advertises: if the 2025 map grew `[line5a]` tomorrow, `keys` would stay 0 and nothing
would red.

Live exposure is nil today (`2024/f8283` has no geometry fixture — it is the year's one
`NOT WITNESSED` map — and `2025/f8283` binds no numbered line), so this is an unsound recorded
assumption rather than a wrong result.

**Minimal change:** fold N1's section counting in, then re-measure `GRID_MAPS`; `f8283` will show 3
keys and must either leave the list or have those three lines checked. If `f8283`'s row letters truly
carry no printed line label, say *that* in the reason and pin it with the geometry, rather than
resting on a count the counter cannot take.

### N3 — MINOR — `ROADMAP_STATUS.md` still publishes the pre-fold floors, on the line the fold edited

**Where:** `design/ROADMAP_STATUS.md:178` — *"floors 235 (2024) / 193 (2025)"*, while `YEAR_FLOORS`
now reads 249 and 201.

**What is wrong:** the fold rewrote line 179 of the same bullet (to name the three kills) and left
line 178's numbers stale. This is L3's class — a derived claim invalidated by the change that was
being made — recurring inside the fold that closed L3, and it is now the *only* floor claim in the
tree not held by a test (the work list got one; §2 did not).

**Minimal change:** `floors 249 (2024) / 201 (2025)`.

### N4 — MINOR — the TY2024 ratchet comment documents a move to 235 above `min_joins: 249`

**Where:** `crates/xtask/src/label_reader.rs:1393-1396`.

**What is wrong:** *"99 → 235 on 2026-09-06 … Measured, not estimated."* now sits above `249`. The
2025 entry was updated to name L4 as the cause of its second move; the 2024 entry was not, so the
ratchet's provenance record — the thing that makes "never lower one" auditable — is wrong for one of
the two live years.

**Minimal change:** *"99 → 235 (R2 parser + the x-aware rule), then 235 → 249 when Schedule D's
inline-table rows joined (L4)."*

### N5 — MINOR — `map_reach_problem` sees a TOTAL drop and not a PARTIAL one

**Where:** `crates/xtask/src/label_reader.rs:1134-1147` — the `_ => None` arm.

**What is wrong:** `(false, 19, 1, 1)` returns `None`. A parser change that dropped 18 of 19 bindings
would pass the reach check, and the year floor would absorb it if another map grew. Since every
numbered key must yield at least one FQN, `extracted < keys` is a sound and free red: measured over
all 31 reachable maps, `extracted >= keys` holds everywhere today (`schedule_d` 13→23 and 5→11 are
the only strict inequalities, both from inline tables).

**Minimal change:** add `(false, k, e, _) if e < k => Some("…{k} numbered line key(s) but only {e}
extracted — a partial silent drop")` and plant it.

### N6 — MINOR — the work-list test's plant exercises the comparator, not the checker; and the excuse test does not test the excuse

**Where:** `crates/xtask/src/form_delta.rs` — the trailing `assert_ne!` block, and the
`(None, Err(_)) => excused` arm.

**What is wrong:** two small gaps in an otherwise sound test. (i) The plant builds a hypothetical row
and asserts its tuple differs from the computed one; it never inserts a wrong row into the parsed
document, so nothing observes `wrong.push(…)` or `assert!(wrong.is_empty())` actually firing —
deleting that final assertion would leave the plant green. (ii) An excused row is accepted on
*"no pair at HEAD"* alone, so a row excused for a **different** reason than the one it prints (e.g. a
2025 fixture deleted by accident rather than a draft that was never a draft) stays excused silently.

**Minimal change:** for (i), run the loop's body over a one-row synthetic document containing
`| \`f6251\` | 62 | 0 | 0 | 1 | port |` and assert `wrong` is non-empty; for (ii), require an
excused row's TY2026 cell to name which side is missing and check that side's fixture is the one
absent.

### N7 — NIT — the inline-table quote split can mis-pair on `\"` or on a quoted trailing comment

`rhs.split('"').skip(1).step_by(2)` assumes quotes alternate open/close on the line. Neither hazard
occurs in any committed map (measured: zero `\"` in a `lineN` value, zero `lineN = { … } #` lines),
and the failure mode is a phantom binding → false FAIL, not a false PASS. Worth a one-line note in
the doc comment saying what the split assumes.

### N8 — NIT — `first_bundled_year(stem) > year` fails open once a later year bundles a form an earlier year lost

`field_census.rs`'s `first_bundled_year` takes the minimum over years *on disk*. If a TY2026
directory lands and `f1040s1a` were then dropped from `forms/2025/`, its first bundled year becomes
2026 > 2025 and the absence is excused on a filable TY2025. Not live (years = 2017/2024/2025, only
2024 filable), and the `measured != recorded` assertion above it still reds unless `YEAR.toml` is
edited too. The exact structural statement is *"the stem's first bundled year is later than this year
**and** no year before it bundles the stem"* — or better, a `first_year` on `Stem` itself.

### N9 — NIT — `FiledPacket`'s `pub` fields still allow an unsorted packet from outside the crate

`FiledPacket::stapled(…)` sorts, but `p.forms.push(…)` afterwards does not. Dropping `Default`
closed the review's named half; the comment correctly claims only what it closed. Recording it so the
remaining path is written down rather than rediscovered.

## Counts

**0 Critical / 2 Important / 4 Minor / 3 Nit.**

None of the Importants is a regression *introduced* by the fold: both are the L4/L5 class left
unclosed for the `[lineN]` section shape, and both were reachable before `91070215`. What the fold
newly asserts — and what makes them findings against it — is that the class is closed
(`numbered_line_keys`: *"a parser that drops a binding cannot also hide the key it dropped"*;
`GRID_MAPS`: *"0 numbered keys each"*), which is not true.

## May the instrument be trusted to GATE a new map?

**YES for the surface it covers; NO for the `[lineN]` section surface.**

- **Covered and trustworthy: 450 joins** (2024 = 249, 2025 = 201), **0 wrong**, **0 unlabelled**,
  across 31 reachable maps in two years — every `lineN = "FQN"` and every `lineN = { … }` inline
  table. The L1 false PASS is closed and pinned: on the geometry, the line-16 checkbox admits its
  option numeral under the old predicate and rejects it under the new one, and the committed kill
  asserts the difference. A new map binding its lines that way is genuinely gated.
- **Not covered: 26 FQNs on 5 live maps** (`2024/f1040sb`, `2025/f1040sb`, `2024/schedule_d`,
  `2025/schedule_d`, `2025/f1040s1a`) bound as `[lineN]` sections, plus 6 more on `2024/f8283`, are
  invisible to the parser, to the key counter, to the per-map reach line and to the floors. All 26
  happen to be correct today. A new map that binds a Yes/No pair or a sub-row table — which is how
  every checkbox line in this tree is written — would be admitted with **zero** lines checked and the
  gate would print a clean per-map row.

So: gate a new map on this instrument once **N1** is closed and **N2** re-measured. Until then, a map
whose numbered lines are section-bound must be checked by hand, and the reviewer must be told the
reach line does not cover it.
