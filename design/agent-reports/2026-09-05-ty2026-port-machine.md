# The year-port machine — what it should BE, measured against the TY2025 port that just happened

**Lens:** the year-port machine (`design/LONG_RANGE_PLAN_filing.md` §5.2/§5.3). **Date:** 2026-09-05.
**Method:** commands only — `./target/debug/xtask` (prebuilt), `pdftotext`, `.venv/bin/python`. No cargo,
no make, no git, no network.

---

## HEADLINE

**The half of the port that the ten-agent fan-out spent most of its effort on — re-deriving the
line↔box join by hand — is already fully mechanical in this repo: `xtask label-boxes` reproduces
**177 of 177** committed TY2025 line bindings across all ten ported forms with zero disagreements.
What is *not* mechanical is the 31,587 bytes of census judgment prose, and the boundary between them is
sharper than §5.2 assumes: a name-existence carry-forward would have shipped **16 bindings into the
wrong box**, one of them Form 6251 line 11 — the AMT itself.**

---

## 1. What TY2025 actually cost, decomposed

Ten agents produced ten maps (`design/agent-reports/2026-09-05-ty2025-map-*.md`, 2,137 lines of report)
plus a 170-line audit. Measured composition of what they wrote:

```
$ for y in 2024 2025; do d=crates/btctax-forms/forms/$y; echo "$y: files=$(ls $d/*.map.toml|wc -l) \
    total=$(cat $d/*.map.toml|wc -l) comment=$(cat $d/*.map.toml|grep -c '^\s*#') \
    lineN=$(cat $d/*.map.toml|grep -cE '^\s*line[0-9]')"; done
2024: files=17 total=2669 comment=1257 lineN=239
2025: files=15 total=2151 comment=1351 lineN=195
```

★ The TY2025 corpus has **more** comment prose than TY2024 (1351 vs 1257) across **two fewer** files.
The growth is the hand-written *port provenance* narrative — `f8959.map.toml` went from 35 comment lines
(TY2024) to 101 (TY2025), and lines 1–41 of the TY2025 file are a prose record of the sha256, the
`dump-fields` diff, the `diff -b` of the two extracts, and the two-witness geometric re-derivation.

**That header is the port machine's specification, written out longhand, once per form, by an agent.**
Every claim in it is a command someone re-typed as English.

Judgment prose, measured separately (the part that is *not* a command's output):

```
TY2024: 551 census reasons, 68570 bytes (~13,714 words)
TY2025: 242 census reasons, 31587 bytes (~6,317 words)
```

---

## 2. The line↔box join is 100% mechanizable — proven, not argued

`xtask label-boxes <stem>` emits `FQN → printed-line-label` from the committed geometry fixture. Run
against all ten ported TY2025 maps and compared to each map's own `lineN = "FQN"` bindings:

```
label-boxes reproduces 177 of 177 committed TY2025 line bindings across 10 ported forms; 0 disagree
```

Including the eleven Form 6251 page-1 bindings that *moved* between years:

```
$ ./target/debug/xtask label-boxes f6251--2025 | grep -E "f1_(5|6|25|33)\[0\]"
topmostSubform[0].Page1[0].f1_5[0]      2a      1
topmostSubform[0].Page1[0].f1_6[0]      2b      1
topmostSubform[0].Page1[0].f1_25[0]     3       1
topmostSubform[0].Page1[0].f1_33[0]     11      1
```

Every one matches the committed map. **This is the single largest saving available**, and it needs no
new derivation logic — only a generator that emits the TSV as `.map.toml` syntax.

★ Honest limit, from the batch's own audit (`2026-09-05-ty2025-map-AUDIT.md` MINOR 5): `label-boxes`
disagrees with the Schedule C **census** on 40 of 88 entries because it joins by y without bucketing by
x-cluster, so it collapses the right-hand Part II column onto the left. That blindness does **not**
touch the 177 mapped bindings above (Schedule C maps only 7 lines), but it does mean the reader cannot
yet generate census *keys* for a two-column form.

---

## 3. ★★★ The one place the measurement revises §5.2

§5.2 pass 1 reads: *"Field-set diff. `dump-fields` both PDFs. Identical name set ⇒ the map may be copied
verbatim… Any delta ⇒ print it and **refuse to copy**."*

Simulated against the port that just happened:

```
form       2024   2025   set identical?  gone   new    PASS-1 verdict
f1040s2    60     63     False           6      9      REFUSE (delta)
f1040s3    39     37     False           9      7      REFUSE (delta)
f1040sa    37     33     False           37     33     REFUSE (delta)
f1040sb    72     72     False           2      2      REFUSE (delta)
f1040sc    105    105    False           1      1      REFUSE (delta)
f6251      61     62     False           0      1      REFUSE (delta)
f8959      26     26     True            0      0      COPY PERMITTED
f8960      38     38     True            0      0      COPY PERMITTED
f8995      33     33     False           18     18     REFUSE (delta)

pass 1 would permit a verbatim copy for 2 of 9 forms
```

**Pass 1 is SAFE as written and it is also nearly useless as an accelerator: it admits 2 of 9.** §5.3's
"the second year becomes a table entry for 8 of 12" is a claim about *tier*, not about pass 1's verdict;
the verdict itself is 2/9.

**And the coarseness is load-bearing, because the softer version is a wrong-number generator.** If pass 1
were relaxed to the intuitive per-binding test — *"carry the binding if its FQN still exists in the new
PDF"* — 125 of 155 bindings survive, and **16 of those survivors are wrong**:

```
--- f1040s2: 2 bindings a NAME-EXISTENCE test accepts and the real 2025 map contradicts
    line11     carry-forward -> form1[0].Page1[0].f1_21[0]   TRUTH -> …f1_22[0]
    line12     carry-forward -> form1[0].Page1[0].f1_22[0]   TRUTH -> …f1_23[0]
--- f1040s3: 3 bindings  (line8, line10, line11 — each off by one)
--- f6251: 11 bindings
    line2a     carry-forward -> topmostSubform[0].Page1[0].f1_4[0]    TRUTH -> …f1_5[0]
    …
    line11     carry-forward -> topmostSubform[0].Page1[0].f1_32[0]   TRUTH -> …f1_33[0]

TOTAL silently-wrong carries a pass-1-only port would ship: 16
```

`f6251 line11` is the AMT itself, feeding Schedule 2 line 2. `map_pdf_conformance.rs:175-190` names this
exact scenario as "the Critical-in-waiting"; the enumeration above is it, measured.

**So the instrument should be: keep pass 1's form-granular refusal, and then hand the human a
per-binding worklist rather than a blank page.** A refusal with no worklist is what made TY2025 cost ten
agents. `label-boxes` (§2) supplies the worklist, correctly, today.

**Recommended shape:**

| pass | verdict | mechanizable? |
|---|---|---|
| 1. field-set diff (`dump-fields` × 2) | form-granular refuse/permit | **yes, today** |
| 1b. ★ NEW — re-derive every binding from `label-boxes` and DIFF against last year's map | per-line worklist: *carried / moved / new / retired* | **yes, today** (§2) |
| 2. per-line printed-text diff | reds and names the line | **yes**, once the extract generator exists (§5) |
| 3. census carry-forward, conditioned | carried / undecided | **partially — see §4** |

---

## 4. ★★ Where §5.2 pass 3's condition is NOT sufficient — the boundary the brief asks for

Pass 3 carries a census `reason` *"only when both the field name and the line text are unchanged."*
**Both can be unchanged and the reason still be false, because the reason is a claim about btctax's
ENGINE, not about the PDF.** Three in-repo instances, all with a zero-delta form:

1. `crates/btctax-forms/forms/2024/f8960.map.toml:27` — *"The DERIVED totals 9d and 11 ARE filled, at
   zero"*. Form 8960's TY2024→TY2025 field diff is **zero** (name and rect, 38/38) and its text diff is
   three year-stamp hunks. The sentence went stale anyway, because FR-12 made `line9d` an `Option<Usd>`.
   The TY2025 porter corrected it by reading `btctax-core`, not the form.
2. `crates/btctax-forms/src/map.rs:106-108` — *"Lines 2c-2t … are CENSUSED as `gap` (not `unmodeled`)"*.
   Measured: both years rule all 18 `unmodeled`.
   ```
   2024:      18 rule = "unmodeled"
   2025:      18 rule = "unmodeled"
   ```
3. `field_census.rs:177-181` records the same class historically: Form 1040 line 7's census reason
   (*"Schedule D is always required"*) was **false** while the form never changed —
   `ScheduleDLines::must_file` exists precisely because it is not.

**So the honest boundary is three-way, not two-way:**

| what | who decides | mechanizable |
|---|---|---|
| field inventory, FQNs, `/MaxLen`, on-states | the PDF | **fully** — `dump-fields` |
| the line↔box join | the PDF's geometry | **fully** — `label-boxes`, 177/177 |
| exact cover (map ∪ census == PDF fields) | arithmetic | **fully** — 20-line script, §6 |
| verbatim instruction quotes in the map | the extract | **fully** — `cite-check`, once pointed at maps (§5) |
| **whether a census reason is still TRUE** | **btctax's engine** | **no** — and a form-only diff cannot see it change |
| a new checkbox: model it or census it | product | **no** |
| a restructured form's tax logic | the statute | **no** — not a port at all |

★ This is the repo's own rule applied to the port machine itself: *a checker that cannot distinguish
"this line encodes no decision" from "we forgot this line" is not a conformance check.* A pass-3 that
carries reasons on form-stability alone cannot distinguish **"still true"** from **"nobody re-read it
against an engine that moved underneath."** The missing third input is a machine-checkable predicate on
`btctax-core` — the shape `line_coverage.rs` already has for *printed* lines (`xtask line-coverage`:
"329 money lines across 17 form(s), 24 exception(s) (ratchet 24)"), which does not exist for *censused*
ones.

---

## 5. What is BROKEN today in the port machine's own inputs

**F-1. The text layer — pass 2's only input — has no generator, and 58 files claim it does.**

```
$ grep -h "^# Regenerate:" design/forms/extract/*.txt | sort | uniq -c
     58 # Regenerate: cargo run -p xtask -- forms extract
      1 # Regenerate: pdftotext -layout crates/btctax-forms/forms/2024/f8283.pdf …
$ ./target/debug/xtask forms extract
usage: cargo run -p xtask -- <docs … | extract-schedule-1a | dump-fields <pdf>>
```

There is no `forms` subcommand (`crates/xtask/src/main.rs:40-216`). Only `extract-schedule-1a` exists,
and it is Schedule-1-A-specific. **A TY2026 port cannot produce its extracts with the documented
command.** This is one small xtask module and it blocks pass 2 entirely.

**F-2. The omission-direction gate is pinned to TY2024, and 330 TY2025 boxes are unaccounted.**

`crates/btctax-forms/tests/field_census.rs:105` — `let year = 2024;` (also `:193`, `:261`). The
`CENSUSED` list at `:29-50` is 17 TY2024 stems. Ran the same assertion outside cargo over every
committed map:

```
TY2024: 17 forms  fields=1330  mapped=779  censused=551  UNACCOUNTED=0
TY2025: 15 forms  fields=1123  mapped=551  censused=242  UNACCOUNTED=330
```

Per-form, the 330 sit entirely in the five **pre-existing slice maps** that the fan-out did not touch:

```
*** f1040        pdf=199  mapped=3    census=0    UNACCOUNTED=196
*** f8283        pdf=117  mapped=54   census=0    UNACCOUNTED=63
*** f8949        pdf=202  mapped=186  census=0    UNACCOUNTED=16
*** schedule_d   pdf=55   mapped=15   census=0    UNACCOUNTED=40
*** schedule_se  pdf=27   mapped=12   census=0    UNACCOUNTED=15
   (all ten ported maps: UNACCOUNTED=0, phantom=0, both=0)
```

★★ **"TY2025 went from 5 to 15 of 17" conceals this**: 10 of the 15 are full ported maps with an exact
cover; the other 5 are the original crypto-slice maps, and the gate that would say so is one year
literal away from seeing them. The sibling test `map_pdf_conformance.rs:71-93` already **derives** its
set by walking `crates/btctax-forms/forms/` on the filesystem — so the two halves of the same invariant
sit in one directory, one year-general and one year-pinned, and the pinned one is the direction that
costs a filer money.

**F-3. `design/forms/FIELD_PROVENANCE.md` is stale by its entire magnitude.** Its table
(`:135-158`) reports 1158 fields / 662 mapped / **496 UNACCOUNTED** across 15 TY2024 forms. Measured
today: **1330 fields across 17 forms, 0 unaccounted.** The census ratchet closed 15→0 after the snapshot
was written. This is exactly FR-15 (`FOLLOWUPS.md:5590`) and exactly §5.3's warning — *"the port machine
will be checked against a stale baseline."* The generator is the ~20-line script that produced the block
above.

**F-4. The ten TY2025 maps are inert, and one cannot deserialize.** No `include_str!` reaches them
(`crates/btctax-forms/src/map.rs:33-90` — 27 consts, five for 2025: f8949, schedule_d, schedule_se,
f8283, f1040). AUDIT IMPORTANT 1 is **still open**: `Form6251Map` (`map.rs:116-121`) declares
`pub line1: MoneyCell` with no `line1a`/`line1b`, while `forms/2025/f6251.map.toml` declares `line1a`
and `line1b`. `toml::from_str` would fail with *missing field `line1`*.

**F-5. The map prose is not cite-checked.** `xtask cite-check` covers two documents:

```
cite-check: OK — 51 quotations, all verbatim.   (SPEC_schedule_1a.md 7/7, IMPLEMENTATION_PLAN 44/44)
```

Its subject list is `crates/xtask/src/cite_check.rs:409-410`. Meanwhile the maps carry **130 (TY2024) +
142 (TY2025) = 272 quoted spans of ≥16 chars in comment lines**, none checked. The f8960 agent verified
its 34 spans by hand with a scratchpad script it explicitly did not commit
(`2026-09-05-ty2025-map-f8960.md`, closing paragraph).

---

## 6. THE WIRING — the half §5.2 does not mention, and it is pure generation

A ported `.map.toml` is inert until five mechanical edits land, all derivable from `(stem, year)`:

| # | edit | site |
|---|---|---|
| 1 | `pub const X_MAP_2026: &str = include_str!("../forms/2026/x.map.toml");` | `map.rs:33-90` |
| 2 | `pub fn ty2026() -> Self { Self::parse(X_MAP_2026).expect(…) }` | e.g. `map.rs:365-367` |
| 3 | a `2026 => Ok(Self::ty2026())` arm | one of **17** `for_year` fns (`map.rs:186,380,714,880,1005,1149,1222,1311,1449,1525,1595,1652,1747,1828,1914,2003,2077`) |
| 4 | `include_bytes!` of the PDF | `pdf.rs` (17 for 2024, 5 for 2025) |
| 5 | a `2026 => Ok(X_PDF_2026)` arm | e.g. `pdf.rs:102-108` |

**The generate half should emit Rust, not just TOML.** A machine that emits only the `.map.toml`
reproduces exactly the state the ten TY2025 maps are in right now — committed, correct, and loaded by
nothing.

---

## 7. ★★★ TY2026 IS ALREADY MEASURABLE — and §5.3's "unlikely to restructure" is contradicted

`design/forms/2026/f6251--2026-DRAFT.pdf` is present locally (sha256 `a547fc9d…`, 295,209 bytes, from
`https://www.irs.gov/pub/irs-dft/f6251--dft.pdf`). I ran the port machine's passes 1 and 2 on it by hand.

**Pass 1 — zero delta.** 62 AcroForm fields both years, identical name set, identical rects except one:

```
$ diff <(tail -n+2 f6251-2025.dump) <(tail -n+2 f6251-2026.dump)
< p1  36.0, 684.0- 445.6, 698.0  text  topmostSubform[0].Page1[0].f1_1[0]
> p1  36.0, 684.0- 445.6, 697.4  text  topmostSubform[0].Page1[0].f1_1[0]
```

**Pass 1 says COPY PERMITTED. Pass 2 says otherwise, on five counts:**

1. ★★★ **Line 1a's cross-reference moved: `Schedule 1-A (Form 1040), line 37` → `line 43`.**
   ```
   2025:  1a  Subtract Schedule 1-A (Form 1040), line 37, from Form 1040, 1040-SR, or
   2026:  1a  Subtract Schedule 1-A (Form 1040), line 43, from Form 1040, 1040-SR, or
   ```
   `crates/btctax-core/src/tax/schedule_1a.rs:381` carries the doc comment *"★ It is **line 37**, the
   senior subtotal, that reaches Form 6251 line 1a — not line 38"*, and `line37` is a struct field name
   (`:382`) with `("37", Leaf::Money(*line37))` at `:545`. **Schedule 1-A renumbered for TY2026.** §5.3
   reasoned it would not, on the grounds that its four provisions run through TY2028 — the draft form
   refutes that.

2. ★★ **Form 1040 restructured again: `line 7` → `line 7a`** in the Part III routing bullet
   (`f6251--2025.txt:61` vs the 2026 draft). Same class as the TY2025 `line 11 → 11a/11b` ripple that
   forced Schedule A and Form 6251 to re-cite.

3. **The §55(d) thresholds RESET rather than indexed** — a statutory move, not inflation:
   ```
   2025:  Single/HoH  $626,350 → exemption $88,100 | MFJ $1,252,700 → $137,000 | MFS $626,350 → $68,500
   2026:  Single/HoH  $500,000 → exemption $90,100 | MFJ $1,000,000 → $140,200 | MFS $500,000 → $70,100
   ```
4. **The MFS line-4 kicker threshold: `$900,350` → `$640,200`.**
5. Indexed movers: 26/28% breakpoint `$239,100 → $244,500` (subtrahend `$4,782 → $4,890`); 0% capgain
   `$96,700/$48,350/$64,750 → $98,900/$49,450/$66,200`; 15/20%
   `$533,400/$300,000/$600,050/$566,700 → $545,500/$306,850/$613,700/$579,600`.

★ **This is the §5.2 thesis vindicated on live TY2026 data**: pass 1 is blind to all five; every one is
in the printed text, under an unchanged field name.

★ **And it exposes a pass-2 design constraint I did not expect.** A whole-file `diff -b` of the two text
layers yields **156 differing lines**, of which roughly 20 are substantive — the rest is the IRS draft
coversheet, the `DRAFT — DO NOT FILE` watermark injected mid-page, and a global left-indent shift. A
file-level diff is unusable on a draft. §5.2's *per-`lineN`* framing ("compare that line's own printed
text") is the correct granularity and is not optional.

★ One more pass-2-visible change with no semantic content: TY2025 prints sub-lines with a bare letter
(`" b Tax refund from Schedule 1…"`) and the TY2026 draft prints the full label (`"2b Tax refund…"`).
Harmless to a per-line comparison keyed on the label, **but it moves the label set the census enumerates
from the form** — the conformance KAT's own enumeration input.

---

## 8. Observed IRS release cadence — measured, because it sets the schedule

Every TY2025 form carries the IRS's own build stamp in its footer:

```
f1040sc  4/3/25    f1040sb  4/23/25   f8959  4/30/25   f8949  5/5/25
f1040sse 5/7/25    f1040s2  5/8/25    f8283  7/1/25    f8960  8/19/25
f1040    9/5/25    f8615    9/9/25    f8995  9/12/25   f6251  9/17/25
f1040sd  10/6/25   f1040s1a 11/4/25   f1040s3 11/17/25 f1040sa 11/20/25
```
(TY2024 extracts carry no such stamp, so this is a one-year observation, not a trend.)

**The forms the law changed were released LAST** — Schedule 1-A 11/4, Schedule 3 11/17, Schedule A
11/20 — while six stable forms were final by 5/8. The TY2026 draft 6251 is stamped `Created 5/26/26`,
and its TY2025 final was `9/17/25`.

---

## CAN BE DONE TODAY — no IRS dependency

1. **`xtask forms extract <stem>--<year>`** — the missing generator that 58 committed extracts already
   name. Blocks pass 2 outright. (§5 F-1)
2. **Un-pin `field_census.rs:105`** (`let year = 2024;`) to walk year directories the way
   `map_pdf_conformance.rs:71-93` already does. Reveals the 330 unaccounted TY2025 boxes immediately.
   ★ B1 kill-test is free: it reds on today's tree before the slice maps are censused. (§5 F-2)
3. **Give FIELD_PROVENANCE.md a generator** (FR-15). ~20 lines; the table in §5 F-2 is its output. The
   document's headline 496 is currently wrong by 496. (§5 F-3)
4. **Fix `Form6251Map`** — `line1` → `line1a`/`line1b`, exhaustive `money_cells()` destructure at
   `map.rs:183` names every call site for the compiler. AUDIT IMPORTANT 1, still open. (§5 F-4)
5. **Point `cite-check` at `crates/btctax-forms/forms/*/*.map.toml`** — 272 quoted spans, currently
   unchecked; the machinery exists and covers 51 spans in two documents. (§5 F-5)
6. **Build pass 1b — `label-boxes` diffed against last year's map**, emitting a
   *carried / moved / new / retired* worklist. This is the biggest single saving and needs no new
   derivation logic: 177/177 already verified. (§2, §3)
7. **Teach `label-boxes` to bucket by x-cluster before joining by y**, so a two-column form's census
   keys can be generated too (Schedule C: 40 of 88 wrong today). (§2, AUDIT MINOR 5)
8. **Generate the wiring**, not just the TOML — the five edits in §6 are a function of `(stem, year)`.
9. **Wire the ten TY2025 maps** so a compiled consumer loads them; until then "N mapped" is an
   unexecuted claim (AUDIT MINOR 3). ★ Do it *before* TY2026, or the port machine is designed against
   ten artifacts nothing has ever executed.
10. **Port Form 6251 to TY2026 speculatively against the draft** — pass 1 is zero-delta and pass 2's
    findings are already enumerated in §7. Draft-derived work is provisional by construction, but the
    *questions* it raises (Schedule 1-A renumbering, 1040 line 7a) are the long-pole ones.

## MUST WAIT, AND ON WHAT

1. **Every TY2026 map except Form 6251 — waits on the IRS publishing the form.** Only one TY2026
   document exists in this repo (`design/forms/2026/f6251--2026-DRAFT.pdf`), and it is a draft. Nothing
   about a TY2026 field inventory is knowable before the PDF exists.
2. **Anything final — waits on the final revision.** The draft's own note is explicit: *"drafts are
   REPLACED in place then withdrawn when the form is finalised — this exact document may no longer be
   retrievable"* (`f6251--2026-DRAFT.pdf.txt`). By the observed cadence (§8) the restructured forms land
   **late** — Schedule A was 11/20 for TY2025.
3. **★ TY2026 Schedule 1-A's shape — waits on `f1040s1a--2026`.** The TY2026 draft 6251 proves the
   reference target moved 37 → 43, but *what* Schedule 1-A gained is unknown until its own draft is
   fetched. This is a `btctax-core` struct change (`schedule_1a.rs:346-546`), not a map port, and it is
   the TY2026 critical path. **Unknown today; the TY2026 Schedule 1-A draft settles it.**
4. **TY2026 Form 1040's line 7a/7b split — waits on `f1040--2026`.** Inferred from a *cross-reference on
   another form*, which is evidence the split happened, not evidence of its shape.
5. **Whether the §55(d) reset in §7 item 3 is final** — draft constants, and the draft is explicitly
   subject to change. Do not encode them.
6. **The census reasons for any genuinely new TY2026 line** — waits on a human reading the form and a
   product decision about btctax's engine (§4). No amount of tooling reaches this; it is 242 reasons /
   31,587 bytes of judgment for TY2025 and there is no reason to expect less.

## WHAT I COULD NOT ESTABLISH

- **When the IRS will publish final TY2026 forms.** §8 is one year of build stamps, not a schedule. A
  live HTTP probe of `irs-prior/{stem}--2026.pdf` would settle availability per form; I did not run one
  (out of lens, and no network use was in scope).
- **Whether `map_pdf_conformance.rs` and `field_census.rs` currently pass under cargo.** I reproduced
  both assertions' *content* outside cargo (§5 F-2: 0 phantom and 0 unaccounted across all 17 TY2024
  maps and all 10 ported TY2025 maps) but did not run the suite.
- **Whether TY2026 Schedule A / Form 1040 restructure further.** Only the 6251 draft is archived.
