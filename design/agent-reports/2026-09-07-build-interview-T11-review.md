# Seam review — interview build T11 (the oracle path, R13)

Independent adversarial build review, own git worktree
`/scratch/code/bitcoin_tax/.claude/worktrees/agent-ab8707481ab94d569`, at `46d4b2d6`.
Every plant was made here, reverted from a `cp` backup and `touch`ed (FR-90); nothing committed,
no subagents. `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`, scoped runs only.

**Both oracles ran live, on households the builder never used**: OpenTaxSolver 2024
(`OTS_DIR=/home/bcg/OpenTaxSolver2024_22.07_linux64`) and PSL Tax-Calculator 6.8.2
(`.venv/bin/python`, pandas 3.0.3). No figure below is asserted on one oracle.

---

## Commands

```
export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review
cargo build -p btctax-oracle-harness -p btctax-cli
cargo nextest run --locked -p btctax-core --test oracle_projection          # 9/9 pass at clean HEAD
cargo nextest run --locked -p btctax-core --test golden_returns --test kat_tax   # 87/87
export OTS_DIR=/home/bcg/OpenTaxSolver2024_22.07_linux64
.venv/bin/python scripts/oracle/check_return.py --selftest                  # exit 0
.venv/bin/python scripts/oracle/sweep.py --seed 1 --count 8                 # 8 admitted, 0 undeclared divergences
btctax init / income import / income project        # a scratch vault, 5 hostile households
.venv/bin/python scripts/oracle/check_return.py --file <row>.json           # x7, both engines live
```

Anchors re-measured (the brief's warning about drift): `testonly.rs:1113` `GoldenInputs`,
`:1359` `build_golden_return`, `:1782` `project_to_golden`, `:1936` `ORACLE_INVISIBLE`;
`check_return.py` 332 lines, `oracle_projection.rs` 372, `README.md` 82; `ORACLE_INVISIBLE`
67 entries (68 `OracleInvisibleLeaf {` minus the struct declaration); corpus 107 cells.
**Every citation in the build report resolves.** No drift.

The hostile household used throughout: MFJ, aged (66) + blind taxpayer, spouse 62, one CTC child,
wages 90,000 / interest 1,200 / dividends 800 (600 qualified) / unemployment 2,500 /
SALT 4,000 + 3,000 / mortgage 9,000 / medical 2,200 / W-2 withholding 7,000, with a distinct `ZQ…`
token in **every** free-text and identity field: taxpayer + spouse first/last/SSN/occupation,
street/city/state/ZIP, IP PIN, dependent name/SSN/relationship, W-2 employer, 1099-INT/DIV/G payer
and payer TIN, Form 1098 lender and lender TIN, Schedule C `business_description` and `naics_code`.

---

## Summary

The projection itself is sound where it was measured: the inverse holds, the derived visible/invisible
partition is real (I watched it red), the dependents block round-trips, `XTOT` genuinely lights the ODC
arm (measured $500 live), and **no identity reaches the row** — verified on a household built to leak,
not on the builder's fixture.

What the build got wrong is at the two ends of the path, and both were invisible to the T11 KATs
because both live outside `ReturnInputs`'s `Usd` leaves:

* **`check_return.py` never sees the filer's return.** Every column in it — including the *btctax*
  column — is recomputed from the projected row by `build_golden_return`, which answers the filer's
  unanswered declarations for them. A return `btctax report` **refuses to compute** prints
  *"Every compared line reconciles"* and exits 0.
* **OpenTaxSolver is never told to itemize** from a projected row, because `ots_direct` gates the whole
  Schedule A block on `standard_or_itemized`, a corpus-only key `GoldenInputs` structurally cannot
  carry. Four of the fourteen projected boxes therefore reach only one engine, and an itemizing filer
  gets a **false DIVERGES and exit 1** on a correct return.

Plus: a non-`Cash60` charitable gift is dropped by the projection *and* by both censuses (the
completeness probe is blind to the class dimension exactly as it once was to the SALT election); the
credit-line excuse cannot tell a blank from an asserted `0`, so a wrong `ctc_provably_zero` proof is
excused; and the witness census prints *"only one engine models this line"* for four lines
Tax-Calculator demonstrably does compute.

`Counts: C=2 I=4 M=3 N=1`

---

## C-1 (Critical) — `check_return.py` checks a rebuilt round-trip household, not the filer's return; a REFUSED return reconciles cleanly

**Where.** `scripts/oracle/check_return.py:196` (`default = _harness([], row)`),
`crates/btctax-oracle-harness/src/main.rs:15-18` (*"DEFAULT mode … Assembles btctax's SAME return the
golden matrix fills (`build_golden_return`)"*), `crates/btctax-core/src/tax/testonly.rs:1557-1559`
(`ri.foreign_accounts = Some(false); ri.foreign_trust = Some(false); answer_all_live_declarations(&mut ri);`).

**What is wrong.** The script is documented as driving the engines over *"your own return"*
(`check_return.py:4-8`) and the CLI help says *"Run it before you export: it is how a REAL return
reaches an oracle"* (`cli.rs:512`). It does not. It hands the harness the **projected row**, and
the harness rebuilds a household from that row with `build_golden_return` — which sets `tax_year`,
answers **every live declaration**, and explicitly answers the Schedule B Part III / FBAR questions.
So:

1. btctax's own column is not read from the filer's packet; it is recomputed from the truncated
   description. Any figure the row cannot carry is missing from **all three** columns and therefore
   produces no divergence at all — the failure mode is silence, not disagreement.
2. The docstring's own contract — *"Exit 2 = the run could not be made (no OTS, **a refused return**,
   a stale harness)"* — cannot fire, because the refusal is answered away in the round trip.
3. It re-answers, on the filer's behalf, exactly the class of question the repo treats as its one
   architectural defect (the answered-ness invariant).

**Evidence.** One vault, one `ReturnInputs`, two commands:

```
$ btctax report --vault ./vault.pgp --tax-year 2024
error: usage: tax year 2024 cannot be computed from its full-return inputs: Schedule B Part III
line 7a (a foreign financial account) must be answered on every return — it is the FBAR/FinCEN
disclosure, and its own answer is what decides whether Schedule B files — run `btctax income answer`

$ btctax income project --vault ./vault.pgp --year 2024 > row2.json      # exit 0, a full row
$ .venv/bin/python scripts/oracle/check_return.py --file row2.json ; echo EXIT=$?
…
Every compared line reconciles.
EXIT=0
```

`grep -n "foreign_accounts\|answer_all_live_declarations" crates/btctax-core/src/tax/testonly.rs`
→ `1557: ri.foreign_accounts = Some(false);` `1558: ri.foreign_trust = Some(false);`
`1559: answer_all_live_declarations(&mut ri);` — inside `build_golden_return`.

**Minimal change.** `check_return.py` must run the refuse screens against the **filer's** return, not
the round trip. Cheapest honest form: `income project` runs `screen_inputs`/`screen_absolute` on the
loaded `ReturnInputs` and emits a `"refused"` block in the wrapper (with the screen that fired), and
`check_return.py` exits 2 when it is present — restoring the documented exit-2 contract without a
second computation path. Then say plainly, in the docstring and in `cli.rs`'s help, that the compared
figures are the *projected household's*, so the check bounds the description, not the packet.

---

## C-2 (Critical) — a projected row can never drive OpenTaxSolver's Schedule A: every itemizing filer gets a FALSE divergence and silently loses oracle 1

**Where.** `scripts/oracle/ots_direct.py:573` and `:687`
(`if h.get("standard_or_itemized") == "Itemized":`), against
`crates/btctax-core/src/tax/testonly.rs:1113` (`GoldenInputs` — no such field) and
`:1782` (`project_to_golden` — cannot emit one).
`scripts/oracle/corpus.py:164` says it outright:
`inp["standard_or_itemized"] = "Itemized"  # read by the Python oracles (not a GoldenInputs field)`.

**What is wrong.** `ots_direct.evaluate` puts `A5a`/`A5b`/`A8a`/`A11`/`A16`/`A18` into the OTS input
**only** when the household dict carries `standard_or_itemized == "Itemized"`. 27 of the 107 corpus
cells carry that key; **no projected row ever can**, because it is not a field of the type
`project_to_golden` returns. So for a real itemizing filer, OpenTaxSolver is handed a household with
no Schedule A at all — while `state_income_tax`, `real_estate_tax`, `mortgage_interest` and
`charitable_cash` are all listed in the projection table as reaching `A5a`/`A5b`/`A8a`/`A11`.

Three consequences, all observed: 1040 line 12 **DIVERGES** on a correct return (btctax and taxcalc
agree; OTS answers with the standard deduction), the run exits 1 telling the filer to *"Adjudicate
against the FORM"*, and Schedule A line 5e loses its OTS witness — reported by the census as
*"only one engine models this line"*, which is false.

**Evidence.** The hostile household with mortgage interest raised to 30,000 so itemizing wins
(itemized 38,000 vs standard 32,300 = 29,200 + 1,550 aged + 1,550 blind):

```
line (label)                               btctax     oracle  verdict
taxable income (L15)                        56500 62200/56500  DIVERGES  diverge
deduction (L12) [OTS]                       38000      32300  DIVERGES  diverge
deduction (L12) [taxcalc]                   38000      38000  OK  agree-ots
SALT (Sch A L5e) [taxcalc]                   7000       7000  OK  agree-ots      <- no [OTS] row
DIVERGENCES:
  · taxable income (L15): diverge
  · deduction (L12) [OTS]: diverge
EXIT=1
     1040sa.line5e          witness: taxcalc  — only one engine models this line
```

The same row with `"standard_or_itemized": "Itemized"` inserted by hand — the only change:

```
taxable income (L15)                        56500 56500/56500  OK  agree-both
deduction (L12) [OTS]                       38000      38000  OK  agree-ots
SALT (Sch A L5e) [OTS]                       7000       7000  OK  agree-ots
SALT (Sch A L5e) [taxcalc]                   7000       7000  OK  agree-ots
```

**Minimal change.** The itemize election is a **non-`Usd` fact that routes money**, which is the class
the SALT probe fixture already exists to protect. Add it to `GoldenInputs` (e.g.
`standard_or_itemized: Option<String>` / a small enum), set it in `project_to_golden` from btctax's own
line-12 decision (`printed.rs:796`'s `standard_or_itemized` is already that quantity), and let
`build_golden_return` ignore it as the corpus does. Then extend the completeness KAT past `Usd` leaves
so a routing fact that no oracle is told reds (see I-1 — the same gap).

---

## I-1 (Important) — a non-`Cash60` charitable gift is silently dropped by the projection AND by both censuses; the completeness KAT is blind to the class dimension

**Where.** `crates/btctax-core/src/tax/testonly.rs:1851-1860`
(`.filter(|g| g.class == CharitableClass::Cash60)` at `:1855`), `ORACLE_INVISIBLE` (`:1936` — no
`schedule_a.charitable` entry), `crates/btctax-core/tests/oracle_projection.rs:145-163`
(`probe_fixtures`, `:154`), `crates/btctax-core/src/tax/scrub_axis.rs:576-583` (both sentinel gifts are
`Cash60`).

**What is wrong.** The projection carries only the `Cash60` class. Because the maximal sentinel's two
gifts are *both* `Cash60`, the derived probe classifies `schedule_a.charitable[].amount` as VISIBLE —
so it may not appear in `ORACLE_INVISIBLE`, and `unprojected_nonzero_leaves` therefore never reports
it either. A real filer's appreciated-property gift to a public charity (`CapGainProp30`, Schedule A
line 12) is on the return, reduces btctax's own deduction, is **not** in the row, and is **not** in
`not_carried` or `not_carried_from_the_ledger`. That is the exact shape the builder guarded against for
the §164(b)(5) SALT election and did not generalise — their own note says *"Any future election that
ROUTES money belongs here too."*

**Evidence.** Live, through the CLI (`class = "cap_gain_prop30"`, `amount = "5000"`, alongside a
`cash60` gift of 1,000):

```
charitable_cash in row = 1000.0
not_carried leaves    = ['schedule_a.medical', 'w2s[0].box2_fed_withheld',
                         'w2s[0].box3_ss_wages', 'w2s[0].box5_medicare_wages']
ledger census         = []
```

And the KAT cannot see the class dimension in either direction. Plant — **delete the class filter
entirely**, so every gift class rides `e19800`/`A11`:

```
$ cargo nextest run --locked -p btctax-core --test oracle_projection
     Summary [0.177s] 9 tests run: 9 passed, 0 skipped
$ cargo nextest run --locked -p btctax-core --test golden_returns --test kat_tax
     Summary [0.017s] 87 tests run: 87 passed, 0 skipped
```

(Reverted; `git diff --stat` empty; 9/9 green again.) A test that passes both with the filter and
without it is not measuring the filter.

**Minimal change.** Add a probe fixture whose `schedule_a.charitable[]` carries a non-`Cash60` class
(the same move `probe_fixtures` already makes for `salt_use_sales_tax`). That immediately reds
`every_money_leaf_either_projects_or_is_named_in_oracle_invisible` and forces the honest disposition:
either carry the non-cash class (taxcalc `e20100`, OTS `A12` — both model it) or report it in
`unprojected_nonzero_leaves` with its §170(b) ceiling mechanism, as `unprojected_ledger_lines` already
does for crypto donations. Note the `Cash30` / `CapGainProp20` / `OrdinaryProp30` classes refuse at
import (`NonPublicCharityContribution`), so `CapGainProp30` and `OrdinaryProp50` are the live cases.

---

## I-2 (Important) — the credit-line excuse cannot tell a blank from an asserted `0`, so a wrong `ctc_provably_zero` proof is excused

**Where.** `scripts/oracle/check_return.py:279-281`:

```python
printed = lines.get(key)
btctax_value = int(printed) if printed is not None else 0
ok, actual_gap, expected_gap = credit_line_verdict(btctax_value, oracle_value)
```

with `credit_line_verdict` (`:140-151`) returning `ok = (round(oracle) - btctax) == round(oracle)`,
i.e. `ok ⟺ btctax_value == 0`.

**What is wrong.** The docstring is explicit that the two are different — *"btctax has no Schedule
8812, so it prints the line BLANK (no cell at all — a blank is no testimony)"* — and the code then
maps both `None` and `"0"` to the integer `0`. But `advisories.rs:937-939`'s `ctc_odc_line19` **does**
print a cell: `Some(Usd::ZERO)` whenever `ctc_provably_zero` fires. So a household whose credit btctax
wrongly proves to be zero swears `0` on line 19 and is excused with the same verdict as one that forgoes
the line. `advisories.rs:842-853` names this hazard and schedules it: Pub. L. 119-21 §70104(a)(2)
raises the per-child figure to $2,200 for TY2025, at which point *"the proof would conclude 'provably
zero' for a household that still has credit left. Taxpayer-adverse, and invisible on the page — line 19
would print a sworn `0`."* This is the one instrument that could catch that, and it cannot.

**Evidence.** Plant `crates/btctax-core/src/tax/advisories.rs:857`
`CTC_PER_CHILD_SS24H2 = dec!(1000)` (the understated-ceiling shape, sized to bite at TY2024 AGIs),
rebuild the harness, run MFJ / $420,000 / one CTC child aged 9 through both engines:

| | btctax | taxcalc | verdict |
|---|---|---|---|
| clean HEAD | `blank` | 1000 | `OK (excused)  gap=1000 expected=1000` |
| **planted** | **`0`** | 1000 | `OK (excused)  gap=1000 expected=1000`, **exit 0** |

The btctax column visibly changed from `blank` to a sworn `0` against an oracle that computed $1,000 of
credit, and the verdict did not move. (`advisories.rs` restored from a `cp` backup and `touch`ed;
harness rebuilt; the row prints `blank … 1000 … OK (excused)` again.)

**Minimal change.** Pass `printed` (the `Optional`) into `credit_line_verdict` rather than collapsing
it: `printed is None` ⇒ the forgo excuse applies (gap must equal the whole oracle credit);
`printed is not None` ⇒ btctax **asserted** a figure and it must equal `round(oracle_value)`, diverging
otherwise. That also fixes the `selftest`'s third case, which today refuses a btctax that printed the
*correct* full credit — under the right rule that reconciles, and a printed `0` against a non-zero
oracle is what fails.

---

## I-3 (Important) — the witness census prints a false mechanism for four lines Tax-Calculator does compute

**Where.** `scripts/oracle/check_return.py:85-92` (`SINGLE_WITNESS_REASON`) and `:304`
(`why = SINGLE_WITNESS_REASON.get(line, "only one engine models this line")`).

**What is wrong.** The map names a real mechanism for four keys and falls back to a *claim about the
tax engines* — *"only one engine models this line"* — for everything else. That claim is false for at
least `8959.line18`, `8960.line17`, `schedule_se.line12` and `1040sa.line5e`:
`gen_goldens.taxcalc_run` bakes `additional_medicare_tax` (`ptax_amc`), `niit`, `se_tax` (`setax`) and
`salt_capped` (`c18300`) — `gen_goldens.py:362-373` — and `check_return.py:209` already holds that dict
(`taxcalc = gen_goldens.taxcalc_run([row], args.year)[0]`). The harness simply compares those lines
against OTS only (`main.rs:377-421`, `verdict_ots`). The filer is told a divergence there *"is
ambiguous, never diagnostic"* while a second engine's figure is sitting unread in the same process —
the inverse of the census's whole purpose, and the *"never enumerate the outcomes you happened to see"*
rule applied to the wrong half.

**Evidence.** MFJ / $420,000, one child:

```
     8959.line18            witness: OTS      — only one engine models this line
     8960.line17            witness: OTS      — only one engine models this line
```

and for the same row:

```
$ .venv/bin/python -c "…gen_goldens.taxcalc_run([row],2024)[0]…"
taxcalc additional_medicare_tax = 1529.9999999999998
taxcalc niit                    = 0.0
taxcalc se_tax                  = 0.0
```

$1,530 is exactly btctax's and OTS's printed Form 8959 line 18 on that household.

**Minimal change.** Either (a) add the taxcalc leg to those three verdicts in the harness — the
figures are already baked, and the `[OTS]`/`[taxcalc]` twin-row pattern at `main.rs:425-495` is the
template — or (b) replace the default string with a per-line mechanism and make an *unnamed*
single-witness line an error rather than a shrug, so a line that quietly loses a witness (as
`1040sa.line5e` does under C-2) cannot pass as "modelled by one engine".

---

## I-4 (Important) — a HoH / MFS / QSS return projects a row no consumer can read; `check_return.py` dies in a Rust panic

**Where.** `crates/btctax-core/src/tax/testonly.rs:1827-1830`
(`other => format!("{other:?}")`), `:1363` (`other => panic!("unmapped filing status {other:?}")`),
`scripts/oracle/gen_goldens.py:119-125` (`TAXCALC_MARS` keys `"Married/Sep"`, `"Head_of_House"`,
`"Widow(er)"`).

**What is wrong.** btctax supports five filing statuses (T8 built HoH / QSS / the §6013 election).
`project_to_golden` maps only `Single` and `Mfj` to the tokens its consumers know, and falls through to
the enum's `Debug` name for the other three — `"HoH"`, `"Mfs"`, `"Qss"` — which `build_golden_return`
panics on and which match none of `TAXCALC_MARS`'s keys either. `income project` nonetheless exits 0
and prints a full row, and the failure surfaces three steps later as a Rust panic. For three of five
filing statuses the entire T11 gate is unavailable, and nothing says so at the point the filer asks
for it.

**Evidence.**

```
$ btctax income project --vault ./vault.pgp --year 2024 | head -4
{ "tax_year": 2024, "row": { "filing_status": "HoH", …          # exit 0

$ .venv/bin/python scripts/oracle/check_return.py --file row_hoh.json ; echo EXIT=$?
oracle_harness  failed: thread 'main' (3021027) panicked at
  crates/btctax-core/src/tax/testonly.rs:1363:18:
unmapped filing status "HoH"
EXIT=1
```

**Minimal change.** Make `project_to_golden` (or `project_return_inputs`) **refuse**, with the reason,
for a status the oracle row does not model — this repo's own rule that an unmodelled case is a refusal
with a stated mechanism, never a value that fails downstream. If the statuses are later carried, the
projection must emit the tokens both drivers already agree on (`TAXCALC_MARS`'s keys), not `Debug`
names.

---

## M-1 (Minor) — the ledger census is a hand list of two, with no derivation and no completeness kill

`crates/btctax-core/src/tax/return_1040.rs:763-793`. `unprojected_ledger_lines` enumerates exactly two
lines (Schedule 1 8v, Schedule A 12). It is the right idea and it closes a real gap the brief did not
ask for — but it is precisely the shape `ORACLE_INVISIBLE` was built *not* to be: a list written by the
author of the projection. `the_ledger_lines_the_oracle_row_cannot_carry_are_reported` proves the two
entries fire; nothing reds if a third ledger quantity starts reaching the printed return. I traced the
ledger sinks and today the list is complete (`business_se_gross` → Schedule C → projects;
`nonbusiness_lending_interest ⊆ nonbusiness_ordinary`; `business_interest` refuses at
`return_1040.rs:1195`; Schedule D projects), so this is a durability finding, not a live gap.
Owning phase: whichever cycle next touches the ledger→1040 seam.

## M-2 (Minor) — `check_return.py --year` silently overrides the wrapper's own `tax_year`

`check_return.py:108-118` (`read_row` discards `tax_year`) and `:179` (`--year`, default 2024). A
2024 projection run with `--year 2025` drives Tax-Calculator with 2025 law while btctax and OTS stay on
2024, producing a fabricated divergence; the header prints the contradiction
(`── … · tax year 2025 ──` / `OpenTaxSolver: OpenTaxSolver 2024`) but nothing refuses:

```
taxable income (L15)                        62200 62200/59800  DIVERGES  diverge
tax (L16)                                    6931  6931/5907   OK  methodology-taxcalc
```

Fix: default `--year` from the wrapper's `tax_year` and refuse an explicit disagreement.

## M-3 (Minor) — seam 6's §G-9 statement is not written down

The brief asks that the report and `ORACLE_INVISIBLE`'s doc say that the gates the oracles take as
input (declarations, the census, the DA answer) are validated by R2's transcription checks, not by
oracle agreement. `check_return.py:36-41` states it for L19/L27/L28 specifically and states it well;
neither `ORACLE_INVISIBLE`'s doc (`testonly.rs:1920-1935`) nor the build report generalises it to the
gates. Nothing **claims** the false thing, so this is a missing sentence rather than a wrong one — but
C-1 shows why it is worth writing: the round trip answers gates for the filer, and a reader of this
path should be told that no oracle agreement anywhere bears on them.

## N-1 (Nit) — the `[taxcalc]`-labelled deeper-line rows print `class: agree-ots`

`main.rs:425-495` builds both the `[OTS]` and `[taxcalc]` legs of lines 12 / 5e / 7a with
`verdict_ots`, so the taxcalc row prints `agree-ots`. Cosmetic; the census reads the label, not the
class, and counts correctly.

---

## Verdict on the five questions

1. **Is the line-19 excuse computed from the mechanism, in both arms?** *Both arms are lit; one arm of
   the verdict is blind.* `XTOT` genuinely turns the ODC leg on — measured live on a Single household
   with one other-dependent: `CTC/ODC (L19) [taxcalc] blank 500 OK (excused) gap=500 expected=500`,
   which is Schedule 8812's `ODC_c × max(0, XTOT − childnum − num)` = 500. The Python↔Rust
   cross-check is a real instrument: planting `XTOT` without the dependent count reds it —
   *"the Python and Rust dependent blocks DISAGREE (python, rust): {'XTOT': (1, 2)}"*. But the verdict
   itself reduces to `btctax printed 0`, and it cannot distinguish a forgone blank from an asserted
   zero — **I-2**, where a real, already-scheduled btctax defect is absorbed.
2. **Is the witness census honest?** *The §G-9 call is right; the fallback is not.* I verified the OTS
   claim against the source: `taxsolve_US_1040_2024.c:1978,1985,1986` read `L19`/`L27`/`L28` with
   `GetLine`, and `NumDependents` has exactly two occurrences in the file (declared `:1801`, read
   `:1928`, never used) — so OTS computes no CTC/ODC/EIC and correctly counts as one witness. I found no
   *other* compared line where an engine's figure is echoed back from btctax (OTS's `L13` is produced by
   its own Form 8995 solver; `qbi_cap_l12` is hand-fed and already flagged WEAK). The dishonesty is the
   other way round — **I-3**, four lines told the filer only one engine models them when taxcalc's
   figure is already in hand, and **C-2**, where a line loses its OTS witness and the census reports it
   with that same false default.
3. **Does the projection silently drop anything non-`Usd`?** *Yes — three things.* The itemize election
   (**C-2**, the worst), the charitable class (**I-1**), and the filing status for HoH/MFS/QSS
   (**I-4**). The completeness KAT is `Usd`-typed by construction and cannot reach any of them; the
   `probe_fixtures` pair shows the author knew the shape and closed exactly one instance of it.
   The remaining non-`Usd` facts I traced are safe: `salt_use_sales_tax` is probed;
   `schedule_c.is_sstb` / `is_cooperative_patron` sit behind btctax's own QBI-over-threshold refusal;
   the §63(f) dates and death gates are carried and FR-93 records the residual.
4. **Does `income project` really carry no identity?** *Yes — verified on a household built to leak.*
   `ZQ…` tokens in 22 free-text and identity fields (including the ones the builder's KAT omits:
   `address_state`, both `occupation`s, `relationship`, all three 1099 payer names **and** payer TINs,
   the Form 1098 lender and lender TIN, `business_description`, `naics_code`, the spouse's surname).
   `grep -c ZQ row.json` → **0**; the three SSNs and the IP PIN are absent; the figures and both
   dependents survive. It is structural, not stripped: `GoldenInputs`'s only `String` is
   `filing_status`, and `not_carried`'s `leaf`/`because`/`note` are field paths and `&'static str`s.
   No `ssn_hash`. Clean.
5. **The ten deviations (§8).** All ten are recorded honestly and nine are right.
   (1) `(ri, state)` — required by R13's own `p22250/p23250` sentence; correct.
   (2) derived counts — correct, and it is this repo's stated rule.
   (3) `XTOT` added — necessary and measured (see 1).
   (4) `unemployment` added — correct; verified live (AGI 94,500 = 90,000 + 1,200 + 800 + 2,500).
   (5) `canonical()` — the two collapses are real and
   `canonicalising_a_golden_row_changes_no_return_it_builds` is the right control, not a fudge.
   (6) `tax_year` + the two §G-9 death gates — necessary, and measured as a corpus no-op.
   (7) the ledger census — a good addition; **M-1** is its durability gap.
   (8) `--row-counts` — the strongest instrument in the build; I watched it red.
   (9) README new, `CLAUDE.md` untouched — correct call.
   (10) line 24 not compared against taxcalc — the mechanism (`c09200` is post-nonrefundable-credit,
   and inherits the L16 dissent) is sound and is stated where a reader will find it.
   The one deviation that should have been made and was not is the **eleventh**: the row gained
   `unemployment` because R13 named a box with no field, and by the identical argument it needed
   `standard_or_itemized` — the driver reads it, `corpus.py` writes it, and the projection cannot
   express it (**C-2**).

---

## Seams checked clean

* **The inverse (seam 1).** 9/9 `oracle_projection` tests pass at a clean HEAD; 107/107 corpus cells
  invert; the two-dependent household round-trips through ages, blindness and both credit edges.
  Wrong-box plant (`taxable_interest ← sum_ordinary_dividends`) reds **two** tests:
  `the_projection_inverts_build_golden_return_on_every_committed_household` — *"the projection is not
  the inverse of `build_golden_return` on single_qdcgt_both_slices"* — and
  `every_money_leaf_either_projects_or_is_named_in_oracle_invisible`. Reverted, 9/9 green.
* **Completeness for `Usd` leaves (seam 2).** The partition is genuinely derived (the controller's
  `"payments"` plant, re-confirmed by the wrong-box plant above reaching the same KAT). 67 entries,
  none stale. `hsa_deduction` projects to `e03290`/`S1_13`. The `Usd` half is sound; the gap is the
  non-`Usd` half (question 3).
* **The excuses are not keyed by name (seam 3).** `grep -n` over `check_return.py`, `gen_goldens.py`,
  `ots_direct.py` and `oracle_projection.rs` finds **no** vector-name or household-name key anywhere in
  the T11 additions; both excuses read taxcalc's own `c07220 + odc` / `eitc` for the household in hand.
  `--selftest` passes and discriminates.
* **The oracles (seam 5).** Both ran live on five households I constructed. `sweep.py --seed 1
  --count 8` → *"8 admitted, 0 skipped … 0 undeclared divergences"*. The corpus is untouched (107
  cells). The one live disagreement I hit — OTS's line 12 on an itemizing return — is **C-2**, a driver
  input omission, and it is recorded here against the form and the driver rather than encoded as an
  excuse.
* **Environment.** No `form_delta` or `harness_check` runs were made, so the brief's known
  PDF-less-worktree failures did not arise.

`Counts: C=2 I=4 M=3 N=1`
