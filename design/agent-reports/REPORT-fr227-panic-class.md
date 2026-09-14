# REPORT — FR-227: the panic is NOT user-reachable, and `xtask blockers` now derives the abort class

Run 2026-09-14. Brief: `design/agent-reports/BRIEF-fr227-panic-class.md`.
Worktree `.claude/worktrees/agent-a23d2ce18459e2cb5`, branch `worktree-agent-a23d2ce18459e2cb5` at
`cc29bb30e`. **Nothing committed.** No subagents. Every gate command foreground. Stayed out of
`crates/btctax-oracle-harness/**` and `.github/workflows/**`.

Files changed: `crates/xtask/src/blockers.rs` (+844), `crates/xtask/src/r15_stop_list.rs` (+27/-8),
`FOLLOWUPS.md` (+27).

---

## Q1 — VERDICT: **the headline is wrong. A user cannot reach that panic at HEAD.**

The FR-227 headline — *"`report --tax-year 2026` PANICS rather than refusing"* — overstates what is
reachable. It is a shipped-code `panic!` and it is real, but it sits behind the compute gate. The
FR-227 entry is amended in `FOLLOWUPS.md` rather than rewritten.

### The controller's failure to reproduce was a TTY problem, not a year problem

    $ ./target/debug/btctax report --tax-year 2026
    error: io: No such device or address (os error 6)
    EXIT: 2

That is `rpassword::prompt_password` on a non-TTY (`main.rs:54`, `CliError::Io`). It never reaches the
tax year at all. `BTCTAX_PASSPHRASE` (`main.rs:51`) gets past it. Every run below sets it.

### Driven to a verdict on a real vault — three states, three clean outcomes, no panic

    $ btctax --vault .../vault.pgp init --key-backup .../key.asc
    Initialized vault .../vault.pgp (key backed up to .../key.asc)

**(a) no tax profile for 2026** — refuses, exit 1:

    $ btctax --vault ... report --tax-year 2026
    TY2026 — preparing (0 forms; TaxTable yes; full-return params no; 1099-DA proceeds+basis)
    Federal tax attributable to crypto — tax year 2026
      NOT COMPUTABLE [TaxProfileMissing]: no tax_profile set for 2026
    Schedule D (raw pre-netting part totals) — tax year 2026
      Part I  (short-term): proceeds 0.00   cost basis 0.00   gain 0.00
      Part II (long-term):  proceeds 0.00   cost basis 0.00   gain 0.00
    EXIT: 1

**(b) with a stored `tax-profile` for 2026** — computes the crypto slice, exit **0**:

    $ btctax --vault ... tax-profile --year 2026 --filing-status single \
        --ordinary-taxable-income 120000 --magi-excluding-crypto 130000 --qualified-dividends 0
    Tax profile for 2026 saved.
    $ btctax --vault ... report --tax-year 2026
    TY2026 — preparing (0 forms; TaxTable yes; full-return params no; 1099-DA proceeds+basis)
    Federal tax attributable to crypto — tax year 2026
      net short-term (whole-return level): 0.00   net long-term (whole-return level): 0.00
      TOTAL federal tax attributable to crypto (delta): 0.00
      marginal rates: ordinary 0.24 / LTCG 0.15 all-in (§1(h) 0.15 + §1411 0)
    EXIT: 0

**(c) with FULL-RETURN inputs stored for 2026** — the path the AMT code lives on. `income import`
first refused a year mismatch honestly:

    $ btctax --vault ... income import --year 2026 --file .../fullreturn_inputs.toml
    error: unrecognized stored config value: key="return_inputs[2026].tax_year" value="these inputs
    are for tax year 2024 but are being stored under 2026 — refusing, because storing one year's
    answers as another's would misattribute the filer's testimony"
    EXIT: 2

With `tax_year = 2026` in the file it imports, and the report then refuses in prose, exit 2:

    $ btctax --vault ... income import --year 2026 --file .../ri2026.toml
    note: TY2026 — preparing (...) — these inputs are stored now; `report --tax-year 2026` computes the
    crypto delta on a stored `tax-profile` and reports the full return as NOT COMPUTABLE (keeping the
    inputs) until full-return parameters for 2026 are bundled.
    Imported full-return inputs for tax year 2026.
    EXIT: 0

    $ btctax --vault ... report --tax-year 2026
    error: usage: tax year 2026 has full-return inputs, but full-return computation is not available
    for it in this build — TY2026 — preparing (0 forms; TaxTable yes; full-return params no; 1099-DA
    proceeds+basis). The inputs are KEPT and will compute when the year's package is bundled. To fall
    back to a raw `tax-profile` for 2026 instead, run `income clear --year 2026` ...
    EXIT: 2

### And every other year-taking command refuses too

| command | outcome |
|---|---|
| `income project --year 2026` | same full-return refusal, exit 2 |
| `export-irs-pdf --tax-year 2026` | `cannot export TY2026: the bundled price dataset ends 2026-06-03, before TY2026's prices_through 2026-12-31 ...` exit 2 |
| `extension --year 2026` | `no full-return tables for 2026 — the full-return packet needs a supported tax year (TY2024)` exit 2 |
| `income open-next-year --from 2026` | exit 0, prints `★ NOT CARRIED: no carryforward was stamped onto TY2027. no full-return tables for 2026 ...` |
| `income show --year 2026` | exit 0 |

**No panic on any of them.**

### Where the panic actually is, and what shields it

`crates/btctax-core/src/tax/return_1040.rs:3077-3085`, in `form6251_inputs_from_parts`:

    line1_rule: form6251_line1_rule(year, agi, total_deductions_l14, schedule_1a)
        .unwrap_or_else(|| {
            panic!(
                "Form 6251 Part I has never been transcribed for TY{year}, so there is no \
                 line-1 rule to apply. REFUSING rather than filing TY2024's Part I under a \
                 TY{year} heading — ..."
            )
        }),

`design/agent-reports/REPORT-stage2-B.md` §3 step 7 is where it was seen, and §2 says how: it came
**"to substitution 2"**, i.e. after **substitution 1** — `by_year.insert(2026, ty2026_full_return())`
at `tax_tables.rs:103` on a throwaway branch. At HEAD that insert reds two independent gates
(`blockers::tests::no_bundled_params_year_has_unreadable_forms` and
`tax_tables::tests::ty2024_full_return_params_bundled`), `full_return_for(2026)` is `None`, and outcome
(c) above is what a user gets instead.

**So: not user-reachable now, and not converted to a `RefuseReason` — per the brief, that is the
correct outcome for an unreachable site.** ★ But it becomes reachable the *instant* TY2026
`FullReturnParams` are bundled, which is the January port. The amended FR-227 entry therefore assigns
the conversion an **owning phase: the TY2026 port** and states it is not optional there — bundling the
params without it ships a panic on the filer's first `report`.

### The brief's own measurement, corrected: 3 of the 4 sites are test code

The brief listed four *"year-ish abort macros outside `#[cfg(test)]` modules, with a brace-tracking
script."* Three are **inside** a braced `#[cfg(test)] mod tests`. Proved with the compiler, not a
scanner — a syntax error planted at `form6251.rs:845`:

    $ sed -n 845p crates/btctax-core/src/tax/form6251.rs
                            Form6251Line1::Y2025 { .. } => panic!( THIS_IS_NOT_RUST ;;;
    $ cargo build -q -p btctax-core --lib
    LIB EXIT: 0                       # <- compiles clean: the line is NOT in the shipped library
    $ cargo build -q -p btctax-core --lib --tests
    error: expected `,`, found `;`
       --> crates/btctax-core/src/tax/form6251.rs:845:81
    TESTS EXIT: 101

| brief's site | verdict |
|---|---|
| `form6251.rs:845` | **test code** — `#[cfg(test)] mod tests` spans 674..1683 |
| `form6251.rs:1114` | **test code** — same span |
| `btctax-tui-edit/src/main.rs:24399` | **test code** — `#[cfg(test)]` span 10338..28832 |
| `year_record.rs:121` | shipped — but it only fires if a **bundled** `YEAR.toml` fails to parse, a build-integrity fault, not a year the user named (`year_record_text(year)` returns `None` for an unbundled year, so `for_year` returns `None`) |

★ The brief also asked me to judge `main.rs:24399` (`"EXCL: LotsForm must open..."`). It needs no
judgment: it is inside a `#[cfg(test)]` module, so the question of whether test-support code in `src/`
counts does not arise for it. The question *does* arise for `testonly.rs` modules, which are **not**
`#[cfg(test)]`-gated (`btctax-core/src/tax/mod.rs:59` `pub mod testonly;`) and therefore **are** shipped
code by the compiler's reckoning. They are counted, and the census says so in its own output: *"A
`testonly` module is shipped code by the compiler's reckoning and IS counted — 15 year-referencing
sites sit in one."*

---

## Q2 — `xtask blockers` now derives the abort class. **No count was pinned.**

New axis in `crates/xtask/src/blockers.rs`, built on the existing `refusal_census` shape: enumerate,
classify, and print the definition and the blind spots in the output. Wired into `collect()` over the
same file list the citation axis already reads, so it costs no extra I/O.

### The class definition, as the command prints it

> **aborts:** a site is one of `[panic!, unreachable!, todo!, unimplemented!, assert!, assert_eq!,
> assert_ne!]` (deciding expression = the macro's own argument list) or `[.expect(, .unwrap()]`
> (deciding expression = the **RECEIVER**) on a non-comment line of `crates/<shipped>/src/**.rs`,
> outside every `#[cfg(test)]` item and every sibling-declared test-only file. Shipped crates are the
> workspace members whose manifest does not say `publish = false` (10 of them). **204 sites**, by grip
> — **VARIABLE: 20**, LITERAL: 47, FN-SIGNATURE (UNCLASSIFIED): 8, no year: 129 — where VARIABLE means
> a year variable decides (the rows above), LITERAL means a year pin a new year cannot reach, and
> FN-SIGNATURE means this census could not say. The LITERAL sites pin `{2017, 2019, 2024, 2025}` and
> **name NONE of them TY2026.**

Everything in that sentence is derived: the operator lists from the two `const`s, the shipped set from
the manifests, the four counts from `YearGrip::ALL` (so a fifth grip prints itself), the pinned-year set
from the sites themselves.

**Why neither published count was pinned, and why neither is 20.** Tier B's **9** counted *"a typed
per-year list with no TY2026 arm"* — a list that may fall through via a `None` arm with no abort at all.
The controller's **4** counted year-ish abort macros but mis-classified 3 of them as shipped. The 20 is
a third thing: a year **variable** in the deciding expression of a shipped abort. It includes three
genuine members of the class **neither** published count named — `form1040.rs:82`, `form8283.rs:68`,
`schedule_se.rs:91`, each a deliberate per-year match whose catch-all arm panics, and whose own comment
says *"A new year must be added HERE, deliberately ... The panic is the point."*

### The three-way grip split is the substance, not decoration

`YearGrip::{Variable, Literal, FnSignature, None}`, matched without a wildcard. **VARIABLE vs LITERAL is
precisely the distinction the two published counts collapsed**: a year *pin* keeps reading the old
document (the `CLAUDE.md` T8 shape) and a new year cannot reach it; a year *variable* is a wall a new
year hits. The live Form 6251 site names **both** — `TY{year}` and the literals 2024/2025 — so precedence
matters: VARIABLE wins, and a test plants that exact mixture. Had a literal shadowed the variable, the
one live site in this class would have printed as *"no new year can reach this"* about the very abort a
new year reached.

FN-SIGNATURE is the census's own UNCLASSIFIED bucket, and it is **named, not dropped**, exactly as
`gate_rows` names its unclassified refusal groups — 8 sites at HEAD, including `packet.rs:1114`'s
`panic!("assemble_printed_forms reached an unscreened Form 1099-DA key ...")`.

### One precision decision worth review: the accessor window is the RECEIVER

For `.expect(` / `.unwrap()` the deciding expression is the sub-expression the operator is applied TO,
not the whole line. Without that, `edit::persist::form_save_draft(app.session.as_mut().unwrap(), year,
&ri)` reads as a year-keyed abort because a `.unwrap()` and the word `year` share a line — the `Option`
being unwrapped is a *session*. Scoping to the receiver removed 11 false rows. The backward walk for a
multi-line chained receiver runs **only** when the operator's line is a continuation (all whitespace
before it, or the line starts with `.`); without that condition it reaches into a neighbouring statement
and re-creates the same false positive.

### ★ A defect found while building it: the existing span tracker leaks 1 888 sites

`r15_stop_list::production_source` counts braces and documents the consequence itself: *"a `{` inside a
string literal inside a test module could end the skip early. That errs toward scanning MORE, which is
the fail-closed direction here."* For R15 — hunting a forbidden idiom — scanning more IS fail-closed.
For a blocker list the same slip is a **false blocker row**, and it is not hypothetical. Using it alone,
the census reported **2 093** sites; the true figure is **204**. The 1 888 extra all come from
`btctax-tui-edit/src/main.rs`'s 18 000-line test module, whose string literals unbalance the count.

Fix: `blockers::shipped_mask` intersects `production_mask` with an **indentation-anchored** tracker — a
braced `#[cfg(test)]` item ends at the `}` in the attribute's own column, which is the rule
`btctax-tui-edit`'s own N-R1 clock-seam checker records. That anchor is sound *here* because
`cargo fmt --all --check` is a gate in this repo, so rustfmt machine-enforces the closing brace's
column rather than the census assuming it. **R15's own semantics were not changed** — its fail-closed
direction stays fail-closed for R15.

One refactor was needed to get there. `production_source` **renumbers** every line after a `#[cfg(test)]`
item, because it drops them — and a census that cites `file:line` cannot use a renumbered body without
printing a line number pointing at the wrong statement. So `r15_stop_list::production_mask` was added
(one `bool` per source line) and `production_source` was **rewritten in terms of it**: one algorithm, not
two, with R15's existing `each_r15_grep_reds_on_a_planted_line_and_not_on_its_near_miss` guarding the
refactor.

### The 20 VARIABLE rows at HEAD

    crates/btctax-cli/src/cmd/admin.rs:2397             .expect("June 15 exists in every year");
    crates/btctax-cli/src/cmd/tax.rs:946                debug_assert!(
    crates/btctax-cli/src/open_next_year.rs:415         .expect("ReturnInputs serializes");
    crates/btctax-cli/src/render.rs:1218                .expect("live => the slice refusal"));
    crates/btctax-core/src/conventions.rs:96            .expect("Feb 28 is always valid")
    crates/btctax-core/src/tax/line_coverage.rs:4050    assert!(
    crates/btctax-core/src/tax/line_coverage.rs:4059    assert!(
    crates/btctax-core/src/tax/return_1040.rs:3078      panic!(     <-- the FR-227 site
    crates/btctax-core/src/tax/testonly.rs:159          assert!(
    crates/btctax-core/src/tax/testonly.rs:166          .expect("June 1 exists in every year");
    crates/btctax-forms/src/f6251_revision.rs:325       assert!(    <-- const-eval; see boundary (7)
    crates/btctax-forms/src/form1040.rs:82              other => panic!(
    crates/btctax-forms/src/form8283.rs:68              (other, _) => panic!(
    crates/btctax-forms/src/map.rs:2068                 debug_assert_eq!(
    crates/btctax-forms/src/schedule_se.rs:91           other => panic!(
    crates/btctax-forms/src/year_record.rs:121          panic!("forms/{year}/YEAR.toml does not parse")
    crates/btctax-forms/src/year_record.rs:254          from_calendar_date(year, month, 1).expect(...)
    crates/btctax-forms/src/year_record.rs:294          from_calendar_date(year, m, day).expect(...)
    crates/btctax-forms/src/year_record.rs:316          from_calendar_date(year, January, 20).expect(...)
    crates/btctax-tui/src/export.rs:55                  .expect("timestamp format is infallible");

Some are honest over-inclusion — the calendar constructions cannot fail — and the census says so rather
than tuning them away. **Over-inclusion is the recoverable direction:** a row that cannot fire costs a
reader a glance, a missing one costs a filer a crash. Every row is `State::Unmeasured` **by
construction**: this is a source scan, and calling one BLOCKED would be a claim the measurement cannot
support.

### The boundary, printed in the output

> ★ **WHAT THE ABORT CENSUS DOES NOT CLAIM.** (1) It is a SOURCE SCAN: it does not prove TY2026 reaches
> any site — the one site chased to a verdict by hand (`return_1040.rs`, Form 6251 Part I) is shielded
> at HEAD because `full_return_for(2026)` is `None` ... (2) `debug_assert*` sites are counted and compile
> OUT in release ... (3) A `testonly` module is shipped code by the compiler's reckoning and IS counted —
> 15 year-referencing sites sit in one. (4) `tests/`, `benches/`, `build.rs` and the non-published
> crates are outside the scan ... (5) A year reached through an alias the scan cannot see — a struct
> field, a const, a closure argument renamed — reads as no year. (6) An abort inside a dependency is
> invisible. (7) An `assert!` inside a const-evaluated guard is a BUILD error, not a runtime abort, and
> is counted here anyway — `f6251_revision.rs`'s Form6251ObbbaMap guard is that case at HEAD ...

---

## B1 — seen red once. Two blinding mutations, plus a live-tree plant.

**"Which test reds when this is reverted?"**

### 1. Blind the classifier to a year variable — **3 tests red**

Mutation: `let grip = if false && names_year_variable(&deciding) {` in `abort_census`.

    Summary [   0.199s] 18 tests run: 15 passed, 3 failed, 248 skipped
       FAIL blockers::tests::the_abort_census_reds_on_a_planted_year_keyed_abort_and_not_on_its_near_misses
       FAIL blockers::tests::the_real_trees_abort_census_still_sees_the_form_6251_part_i_panic
       FAIL blockers::tests::the_command_answers_for_ty2026_at_head

    thread '...the_real_trees_abort_census_still_sees_the_form_6251_part_i_panic' panicked at
    crates/xtask/src/blockers.rs:2801:9:
    the census no longer sees a year-keyed `panic!` in crates/btctax-core/src/tax/return_1040.rs —
    either the site is gone or the classifier went blind; found 2 sites in that file

Reverted: 18/18 pass.

### 2. Reduce `shipped_mask` to brace counting alone — **1 test reds**

    Summary [   0.214s] 18 tests run: 17 passed, 1 failed, 248 skipped
       FAIL blockers::tests::the_indentation_anchored_tracker_excludes_what_brace_counting_leaks

    panicked at crates/xtask/src/blockers.rs:2768:9:
    assertion `left == right` failed: brace counting must leak the planted in-test abort and the
    indentation anchor must catch it;
    brace=[true, false, false, false, false, false, true, true, true, true, true, true]
    shipped=[true, false, false, false, false, false, true, true, true, true, true, true]
      left: 0
     right: 1

That is the kill for the tracker **this change added** — watched discriminating rather than assumed. It
is the mask that reports 204 sites instead of 2 093. Reverted: 18/18 pass.

### 3. The LIVE-TREE plant, in a shipped crate, surfacing in the command's real output

★ **Per FR-235 the plant is deliberately NOT written in the census's loudest vocabulary.** The site that
motivated this axis is a `panic!`; planting another `panic!` would measure only the `panic!` grep. The
plant is an **`.expect(`** on a year-parameterised receiver — a different operator, read through a
different window (the receiver, not the argument list).

Planted into `crates/btctax-core/src/tax/tables.rs`, inside the shipped `schedule_1a_params(year)`:

    let _planted = SCHEDULE_1A_YEARS
        .clone()
        .find(|y| *y == year)
        .expect("PLANTED FR-227 kill: no Schedule 1-A arm for that year");

`cargo run -q -p xtask -- blockers 2026` then went from 20 VARIABLE rows to **21**, and printed the new
row naming the plant by file, line, operator and grip:

    | **UNMEASURED** | now | `crates/btctax-core/src/tax/tables.rs:2124` ABORTS (`.expect(`) with the
    tax year in its deciding expression, so TY2026 reaches it if control does — and an abort is not a
    `RefuseReason`, so the refusal census cannot see it: `.expect("PLANTED FR-227 kill: no Schedule 1-A
    arm for that year");` | crates/btctax-core/src/tax/tables.rs:2124 (blockers::abort_census,
    grip=VARIABLE) |

Reverted: back to 20 rows, and the tree verified unmodified.

### The committed near-miss battery

`the_abort_census_reds_on_a_planted_year_keyed_abort_and_not_on_its_near_misses` plants the `.expect(`
and then asserts the census does **not** fire on seven things each one property away from it: a year
LITERAL instead of a variable (classifies `Literal`, not a row); no year at all (`None`); the same plant
inside a braced `#[cfg(test)] mod tests` (absent); a plant **after** a test module (still seen — the
truncating version of this scan swallowed it); a file a sibling declares `#[cfg(test)] mod planted;`
(absent); a non-published crate's `src/` (absent); and `tests/` rather than `src/` (absent).

`shipped_crates_is_derived_from_the_manifests_and_publish_false_opts_out` plants two manifests and also
asserts the real tree agrees — `btctax-core`/`btctax-cli` ship, `xtask`/`btctax-oracle-harness` do not.

`a_year_shaped_identifier_is_not_a_year_variable` plants `by_year` and `year_record` as non-matches: a
census that fired on every identifier containing "year" would report the whole tree and be ignored.

---

## Gate — run SERIALLY, in the foreground, real output

    $ cargo nextest run --workspace
         Summary [  19.744s] 3826 tests run: 3826 passed, 12 skipped

    $ cargo clippy --workspace --all-targets -- -D warnings
        Checking xtask v0.18.0 (...)
        Finished `dev` profile [optimized + debuginfo] target(s) in 0.75s
    CLIPPY EXIT: 0

    $ cargo fmt --all --check
    FMT EXIT: 0

    $ cargo run -q -p xtask -- blockers 2026
    # TY2026 blockers — DERIVED at HEAD, 78 rows (35 UNMEASURED)
    ...
    BLOCKERS EXIT: 0

78 rows / 35 UNMEASURED, up from 58 / 15 before this axis — the 20 abort rows are all UNMEASURED.
Clippy first refused four `needless_range_loop`s in the new code; those were rewritten as slice `fill`
and iterator `skip`/`take`, and the census was re-run afterwards to confirm it still yields 204/20.

---

## What the brief got wrong, or left open

1. **The brief's four abort sites were not four.** Three are inside braced `#[cfg(test)]` modules; the
   brace-tracking script mis-measured, and the compiler settled it in two commands. Only
   `year_record.rs:121` of that four is shipped, and it is a build-integrity guard (a malformed *bundled*
   `YEAR.toml`), not a wall a user-named year hits.
2. **The brief called `form6251.rs:845` "vector-verification support that happens to live in `src/`."**
   It lives in `src/` *inside* `#[cfg(test)]`, which is a stronger fact: it is not in the library at all.
   The `src/`-but-genuinely-shipped case the brief was reaching for does exist — the `testonly` modules,
   which are `#[doc(hidden)] pub mod`, not cfg-gated — and the census counts and discloses those.
3. **The headline's own reproduction was never about the year.** `os error 6` is an `rpassword` failure
   two lines into `main`, before any year is read, so "could not reproduce" was not evidence either way.
   Both the brief and the FR-227 entry treated it as a year-behaviour observation.
4. **Left open deliberately:** the `return_1040.rs` panic is NOT converted to a `RefuseReason`, because a
   user cannot reach it — which is what the brief prescribes for an unreachable site. The conversion is
   recorded in the amended FR-227 entry with **owning phase: the TY2026 port**, and stated there as not
   deferrable past it, because bundling TY2026 `FullReturnParams` without it ships a panic on the filer's
   first `report`.
5. **Not in scope, but worth an owner's glance:** `production_source`'s documented brace-in-string
   weakness is now *measured* (1 888 leaked lines in one file). Nothing built on it today is wrong — it is
   fail-closed for R15's greps — but any future checker that needs precision rather than recall should
   use `blockers::shipped_mask`.

## State left behind — nothing committed

Uncommitted in `.claude/worktrees/agent-a23d2ce18459e2cb5`, branch
`worktree-agent-a23d2ce18459e2cb5` (base `cc29bb30e`):

    M FOLLOWUPS.md                       +27   (the FR-227 amendment)
    M crates/xtask/src/blockers.rs      +844   (the abort census + its kills)
    M crates/xtask/src/r15_stop_list.rs  +27/-8 (production_mask; production_source rewritten on it)

Both blinding mutations and the live-tree plant were reverted and verified reverted; no other file in the
worktree is modified. `crates/btctax-oracle-harness/**` and `.github/workflows/**` were never touched.
