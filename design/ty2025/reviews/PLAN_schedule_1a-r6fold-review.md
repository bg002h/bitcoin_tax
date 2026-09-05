# Schedule 1-A PLAN — review of the **r6 FOLD** (Opus)

**Date:** 2026-09-05 · **Artifacts:** `e4b8f996` (the r6 fold of
`design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md`, 30 insertions / 1 deletion, one file) and
`ff839ce7` (the code half of r5's I-2) · **Answers:**
[`PLAN_schedule_1a-r5fold-review.md`](./PLAN_schedule_1a-r5fold-review.md) (0C/2I/4M/1Nit).

**Scope honoured:** the r6 fold, the plan text it touched, and `ff839ce7`. The SPEC, FR-29 and
everything outside this plan were not re-reviewed; no new checker, task or form is proposed.

---

## VERDICT

**0 Critical / 0 Important / 4 Minor / 2 Nit — BUILD T2.**

**Both r6 Importants are closed, and the split is the right cut.** I tried to break it from three
directions rather than accept its summary: (a) Rust's `cfg(test)` semantics across the crate line,
(b) what each half actually has to *read*, and (c) whether all four planted defects survive the cut.

The load-bearing facts hold exactly as cited. `#[cfg(test)]` is at `tables.rs:1353`, `mod
schedule_1a_conformance` at `:1354`, `fn printed_line` at `:1360` — a bare `fn` in a `cfg(test)`
module, so it genuinely does not exist when `btctax-core` is compiled as `xtask`'s dependency, and
half 2 placed in `xtask` could not have compiled. The in-crate instructions fixture carries **exactly
four** `— Keep for Your Records` anchors, at `:427`, `:556`, `:1049`, `:1066` and nowhere else, so the
worksheet half has the mechanical source it lacked. `design/forms/geometry/` still has no
`i1040gi--2025.json`, so the fold's reason for *not* putting the worksheets in `xtask` stands.

**And the split is better-founded than the fold argues.** Half 2 must compare a *doc comment* against
the extract, and doc comments are not readable at runtime — the check has to read the struct's own
source. The one in-tree precedent for that is `classifier.rs:736-737`
(`include_str!("return_inputs.rs")`, `include_str!("classifier.rs")`), which works only **in-crate**;
from `xtask` it would be the escaping `include_str!` the plan condemns two paragraphs earlier. So half
2 belongs in `btctax-core` for a second, independent reason the fold did not state.

**The other half still reaches its instrument.** `crates/xtask/Cargo.toml:20` is the `btctax-core`
dependency; `label_reader` exposes `witness_text` / `witness_boxes` / `assign_boxes` / `Row` / `Kind`
as `pub`, and `label_reader.rs:741` is an **already-passing xtask test** that loads the Schedule 1-A
geometry and asserts 50 / 48 / `["4","22"]` — the very mechanism CORRECTION 3 names, executing today,
inside the crate the fold assigns membership to. `make check` is `cargo nextest run --workspace`
(`Makefile:26`), so an `xtask` unit test is inside the gate.

**No third thing is out of reach.** Everything halves 3 and 4 need is `pub` in `btctax-core`
(`Production` `:64`, `LineCoverage` `:105`, `Coverage(pub Vec<LineCoverage>, …)` `:131`, and the
non-`cfg(test)` `tax::testonly` module as the cross-crate fixture precedent), so both halves compile
from either side of the line.

**Scope: nothing widened.** One file, +30/−1, no new task, form, checker or artifact; item numbering
unchanged; no `.rs` touched.

The four Minors are real and cheap. Three of them are **carried, not created** — r5's own M-1/M-2,
plus address decay from `ff839ce7`. None can produce a wrong figure in B3, and none is worth an
eleventh round.

---

## FINDINGS

### M-1 — Minor. Item 5 places THREE things; item 6 names FOUR halves. The worksheets — the half the fold just created — get no planted defect

Item 6 (unchanged by the fold) reads *"this KAT has FOUR halves, and each needs its own planted
defect"*: **membership**, **per-line quotation**, **provenance**, **completion**. Item 5 now assigns
homes to **membership**, **per-line quotation** and **the worksheets**. The two lists do not
correspond:

- **the worksheets have no named plant.** After the split, membership is *"the 48 form labels, from
  `label_reader`"* — and census F-4, four paragraphs above, says in terms that a census driven off the
  form *"could never red on a worksheet omission… It would have passed by finding nothing."* So
  defect 1 (*drop a line from the struct*) does not cover a dropped worksheet row, and nothing else
  does. The half the fold exists to enable is the one half with no kill test, which is the precise
  shape item 6 was written to prevent (*"a mutation that kills one half leaves the other three
  green"*).
- **provenance and completion have no stated crate.** Not a reachability problem — measured above,
  every ingredient is `pub` — but item 5 is the plan's single statement of where the KAT lives, and it
  now answers for three of five pieces.

**Why Minor and not Important.** Exit criterion 5 is a hard B3 gate — *"Mutation-verified, per guard.
A guard whose mutation survives is not a guard"* — and it binds the worksheet guard whether or not
item 6 lists it, so T2 cannot close green with an unverified one. And halves 3 and 4 fail *loudly*
(a compile error, as half 2 would have) if misplaced, never with a false green.

**Fix, one sentence in item 6:** add a fifth plant — *delete one worksheet row (or a whole worksheet)
from the struct* — and pin the anchor count at **4**, so a parser that finds nothing reds instead of
passing. Optionally name `btctax-core` as the home for halves 3 and 4.

### M-2 — Minor. Three places still say the KAT lives wholly in `xtask`, one of them inside item 5 itself

The fold kept the r5 text rather than deleting it, deliberately. The cost is that
`design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md` now asserts the superseded placement in bold at
**line 362** — *"(Original r5 text follows.) **The conformance KAT lives in `crates/xtask/`, not in
`btctax-core`**"* — inside the operative item, plus **line 67** (*"the KAT moves to `xtask`, the
struct stays in `btctax-core`"*) and **line 98**, neither carrying an r6 correction marker.

I diffed the two kept blocks: the *"Superseded r5 reasoning, kept…"* paragraph (`:356-360`) already
preserves the whole crate-direction argument verbatim, and the only content unique to the
*"(Original r5 text follows.)"* block is r4 placement history plus *"`xtask` can see
`btctax_core::tax::schedule_1a::*`"*, which line 352 restates. **Deleting `:362-370` therefore loses
nothing** and removes the one contradictory directive sitting in the operative section. Lines 67 and
98 are inside the r5 round-log and read as history; one *"(superseded by r6 — see item 5)"* on line 67
would close them.

Minor because a reader of item 5 meets the corrected split first, in bold, tagged *"corrected by r6
I-1 and I-2"*, with the ⇒ mapping — and would build the split.

### M-3 — Minor (carried r5 M-1, now compounded). The plan still says the two `line_coverage` capabilities are NOT LANDED, and every address it cites for them has decayed

This is the brief's check 5, and the answer is that the plan's text does **not** yet match the code.
`ff839ce7` landed both capabilities 22 minutes before the fold, and three places still say otherwise:

- **line 24** (B-I2 row): *"⚠ TWO-THIRDS FIXED IN CODE … **Not landed:** `LineCoverage.year` is
  per-row but **no constructor can set it**, and `Coverage::exception` was never widened"*;
- **line 58** (the r5 I-2 row), same claim;
- **§T2 item 8** (`:389-400`): *"Add a year-carrying path"*, *"Widen `exception` the same way."*

All false at HEAD, and the addresses are stale by roughly the 101 lines `ff839ce7` inserted:

| plan cites | actually at HEAD |
|---|---|
| `DEFAULT_ROW_YEAR` `:49` | `:55` |
| `Coverage::line` `:137` / `:139` | `:156` |
| `year: DEFAULT_ROW_YEAR` `:148`, `:170` | `year: self.1` at `:167`, `:196` |
| `Coverage::exception` `:159` / `:161` | `:185` |
| (absent) | `Coverage::quoting_year` `:144` |

`LineCoverage.year` at `:112` (plan says `:106`) is the one that only drifted a little.

The **substance** of `ff839ce7` matches what item 8 asked for: `quoting_year(&mut self, &'static str)`
writes `self.1`, both constructors now stamp `year: self.1`, and `exception` takes
`_value: impl Into<Option<Usd>>` exactly like `line`. So this is text lagging code, in the direction
that wastes an implementer's time rather than misleading a figure — but an implementer told *"no
constructor can set it"* may write a second year-carrying path beside `quoting_year`. Fix: restate
item 8 as **use**, not **add**, and refresh the six addresses.

### M-4 — Minor (carried r5 M-2). Nothing yet tells T2 to CALL `quoting_year`, and the setter is forward-sticky with no reset

Unchanged by this fold and correctly deferrable: I re-measured item 7's premise —
`grep -rnE 'Schedule1a|schedule_1a|f1040s1a' crates/btctax-forms/src/ crates/btctax-forms/forms/`
returns **0**, so the checker contributes no Schedule 1-A rows during B3 and a row left at
`DEFAULT_ROW_YEAR` cannot mis-resolve while B3 is open. Worth one clause in item 8 when M-3 is fixed
(*call `c.quoting_year("2025")` before the Schedule 1-A rows; push them last or restore the default
after*), because the forward-sticky failure is a silent pass, not a red.

### N-1 — Nit. *"No new code and no new fixture is required by this split"* is true of the split and false of the half

Accurate as scoped — the placement itself is free, and the fixture file really is in-crate at
`crates/btctax-core/src/tax/fixtures/schedule_1a_2025_instructions.txt`. But `btctax-core` does not
`include_str!` it today: `tables.rs:1351` includes only `SCHEDULE_1A_FORM_TEXT`, and the instructions
file is currently read **only** by `crates/xtask/src/cite_check.rs:414`, at runtime, through a
repo-root path. So the worksheet half adds one in-crate `include_str!` plus its own reader —
`printed_line` reads `SCHEDULE_1A_FORM_TEXT` and cannot serve worksheet rows. No tarball risk (the
file is inside the crate), and none of this is work the split created; just don't read the sentence as
*"the worksheet half already exists."*

### N-2 — Nit (in `ff839ce7`). `exception`'s `#[allow(clippy::too_many_arguments)]` sits BETWEEN the two halves of its doc comment

`line_coverage.rs:176-185`: the one-line summary, then the attribute, then the ★★ *"Widened
2026-09-05"* paragraph, then `pub fn exception`. It compiles, `fmt` is clean and clippy is green (the
sibling `line` puts the attribute last, after its whole doc). Cosmetic; move the attribute below the
doc block next time the file is touched.

---

## WHAT I VERIFIED AND HOW

**The fold's own citations, all exact.** `sed -n '1350,1420p' crates/btctax-core/src/tax/tables.rs`:
`#[cfg(test)]` at `1353`, `mod schedule_1a_conformance {` at `1354`, `fn printed_line(label: &str)` at
`1360` — not `pub`, in a `cfg(test)` module whose only data source (`SCHEDULE_1A_FORM_TEXT`, `:1351`)
is itself `cfg(test)`. The module the fold names is real and is where the fold says it is.

**The worksheet source, counted rather than assumed.**
`grep -c 'Keep for Your Records' crates/btctax-core/src/tax/fixtures/schedule_1a_2025_instructions.txt`
= **4** (1411-line fixture), at `:427` *Qualified Tips From More Than One Employer*, `:556` *Multiple
Trades or Businesses*, `:1049` *Qualified Overtime Compensation From More Than One Employer*, `:1066`
*…From More Than One Payor*. Four anchors, four worksheets, no fifth. `ls design/forms/geometry/`
= `f1040--2024`, `f1040s1a--2025`, `f1040sa--2024`, `f1040sa--2025`, `f6251--2025` — **no
`i1040gi--2025.json`**, so the fold's *"no mechanical source in `xtask`"* holds.

**The membership half really can still reach `label_reader`.** `crates/xtask/Cargo.toml:20` is
`btctax-core = { path = "../btctax-core", … }` (one-way, as the kept r5 argument says).
`label_reader.rs` exports `pub fn witness_text` `:275`, `pub fn witness_boxes` `:302`,
`pub fn assign_boxes` `:344`, `pub enum Kind` `:42`, `pub struct Row` `:54`. `pub fn run` `:372`
returns `Result<(), String>` and prints, so a KAT composes the three witnesses itself — which is
exactly what the existing test `the_witnesses_resolve_the_50_vs_48_question`
(`label_reader.rs:741-774`) already does, asserting `labels.len() == 50`, `entry.len() == 48`,
`headings == ["4","22"]`. The mechanism CORRECTION 3 assumes is not hypothetical; it is green today,
in `xtask`. `Makefile:26` runs `cargo nextest run --workspace`, so that test binary is in the gate.

**Is there a THIRD thing? I looked for one, and did not find it.** The candidates and their answers:

| what a half needs | reachable from |
|---|---|
| `printed_line` + the form extract | `btctax-core` only (`cfg(test)`, in-module) — hence the split |
| the four worksheets' text | `btctax-core` in-crate fixture (one `include_str!` away, N-1) |
| **the struct's doc comments** | must be read as *source text*; in-crate `include_str!` only — precedent `classifier.rs:736-737`. **Independently confirms half 2's placement** |
| `label_reader` + repo-root geometry | `xtask` only |
| a `Schedule1A` value, `(label, got.lineN)` | either crate, once the struct is `pub` |
| `Production` / `LineCoverage` / `Coverage` for the provenance half | either crate — all `pub` (`:64`, `:105`, `:131`, with `pub Vec<LineCoverage>`) |
| a constructed fixture from `xtask` | precedented — `tax::testonly` is a non-`cfg(test)` `pub` module (`testonly.rs:538`), as is `btctax_forms::testonly`, already used by `label_reader::proof` |

**B1 across the crate line, half by half.** ① *membership* — plant by deleting a struct field: the
`xtask` tuple array's `got.lineN` fails to compile (`E0609`); delete the tuple entry too and the
closed-at-both-ends comparison against `label_reader`'s 48 reds. Plantable, red. ② *per-line
quotation* — now beside `printed_line`, with a live precedent doing the same comparison for lines
11/19/28 (`each_phase_out_rounds_the_way_its_own_printed_line_says_to`, `tables.rs:1391`). Plantable,
red. ③ *provenance* and ④ *completion* — plantable from either crate (table above); unplaced, see M-1.
⑤ *worksheets* — plantable, **unnamed**, see M-1. That is the one gap, and it is the whole of M-1.

**`ff839ce7` against the plan's text.** Read `line_coverage.rs:100-200` directly rather than the
commit message: `pub fn quoting_year` `:144` sets `self.1`; `pub fn line` `:156` and `pub fn
exception` `:185` both write `year: self.1` (`:167`, `:196`); both now take
`_value: impl Into<Option<Usd>>`. Both capabilities item 8 asks for exist. The plan has not caught up
(M-3), and the addresses have moved by ~19 lines at the top of the file and ~26 lines lower down.

**Scope.** `git show e4b8f996 --stat` = `design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md | 31 +++-`,
one file, 30 insertions / 1 deletion. The single deletion is item 5's old opening sentence, which is
re-inserted verbatim below (M-2). No new `T`-numbered task, no new form, no new checker, no `.rs`.
`grep -n "KAT"` over the plan shows every other KAT reference unchanged and still consistent with the
split, except the three placement claims in M-2.

**Taken as given per the brief, not re-derived:** `make check` 2808/12/0 at HEAD, `fmt`,
`archive-check`, the 106-entry authority manifest, and `label-census f1040s1a--2025` = *"48 entry
line(s), 2 without a box"*.

## WHAT I COULD NOT CHECK

- **I did not build the KAT.** Like r5, my `cfg(test)`-across-the-crate-line argument is static:
  measured attributes plus Rust semantics, not a throwaway `xtask` test observed failing to compile.
  The fold's claim is now doubly supported (the doc-comment/`include_str!` argument above), so I did
  not think the experiment worth T2's first hour.
- **The worksheet half's assertion SHAPE is still open.** I read the rows under the `:427` anchor:
  they are lettered *columns* — *"(a) Name of employer"*, *"(b) Amount of qualified tips reported by
  this employer on Form W-2…"* — beneath numbered rows `1, 2, 3…`, i.e. structurally unlike the form's
  48 numbered labels. So *"all four worksheets"* in exit criterion 2 needs a discriminator of its own
  (four types present? every lettered column present?), and the anchors alone do not settle it. That
  is T2's work, and M-1's fifth plant is what would keep it honest.
- **I did not run the suite** (machine-verified in the brief) and did not re-open the IRS PDFs: the
  fold quotes no new instruction text, so no new citation needed adjudicating.
- **The SPEC, FR-29, B4's emitter and T7's oracle census** — out of scope and untouched by this fold.
