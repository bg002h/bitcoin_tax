# Fold review r2 — of 18027b03 (the fold of the r2 fold review)

Reviewer: Claude Opus 5 (1M), independent, read-only. Date: 2026-09-05. HEAD: `61bfae16`.

**Read:** `design/agent-reports/2026-09-05-fable-plan-review.r2-fold-review.md` (all 294 lines) and its
`…VERIFICATION.md`; `git show 18027b03` in full; `git diff 18027b03..HEAD -- design/ FOLLOWUPS.md`
(3de5c272, 61bfae16); `design/FORM_AUTHORITY_TABLE_DESIGN.md` in full; the cited lines of
`design/TY2026_PORT_REPORT.md` (:257, :264, :274-281, :304, :449, :540), `design/LONG_RANGE_PLAN_filing.md`
§6.3/§6.4 (:464-495), `design/ROADMAP_STATUS.md` (:7, :35, :51-70, :164-192, :205, :226),
`design/TY2026_WORK_LIST.md:37-52`, `FOLLOWUPS.md` FR-46/FR-47/FR-49/FR-50/FR-54; and the code step 1
touches — `crates/xtask/src/cite_check.rs` (`FORMS` :740, `FormAuthority` :725, `STEM_ALIASES` :770,
`irs_basename` :774, `emitted_form_years` :802-870, `AUTHORITY_NOT_YET_ARCHIVED` :883),
`crates/btctax-forms/src/map.rs` (top-level structs, `deny_unknown_fields`, every `tyYYYY`/`for_year`),
all 37 `crates/btctax-forms/forms/*/*.map.toml` headers, `crates/btctax-core/src/tax/tables.rs:1351,1365`,
`crates/btctax-core/src/tax/printed.rs:1-60`.

**Not re-derived** (settled per the brief): the 31/37 sha256 join and its six misses; the two-row
`STEM_ALIASES`; the five shared 2017/2024/2025 structs; the owner rulings; the glob + header + `build.rs`
shape; that `line_set` is a revision (many-to-one); every claim in the ledger; the C1 code fix.

No file other than this one was written. No mutating command was run.

## Verdict: **0 Critical / 3 Important / 3 Minor**

The fold is faithful in substance: all twelve findings are answered at the site the review named, in the
document the review named, and nothing was argued away. Two of the closures, however, moved a problem
rather than resolving it — the manifest-join excuse (F7) and the relocated `line_set` kill (F6) both now
name an escape hatch that the design does not define — and the F2 "Decision" in §9 is, as written,
destructive. §10 step 1 is **not yet** executable, for one reason that did not exist before the fold.

## Per-finding trace

| fold-review finding | closed where (file:line) | closed? | note |
|---|---|---|---|
| **F1** (per-YEAR / bad example) | `TY2026_PORT_REPORT.md:257` (step 19 → per-LINE-SET-REVISION), `:304` (axis row rewritten) | **yes** | zero live "per-YEAR artifact" hits remain outside agent reports. Widened beyond the finding → **G5** |
| **F2** (`FORMS` retire / "registry to grow") | `TY2026_PORT_REPORT.md:274-281` (paragraph inverted), `FORM_AUTHORITY_TABLE_DESIGN.md:203` (§9 row) | **partial** | the §9 row's stated **Decision** cannot be executed and would destroy two live extracts → **G3** |
| **F3** (TY2025-first commitment) | `LONG_RANGE_PLAN_filing.md:489` (struck), `:467` (option A superseded); C marked chosen in prose at `:481-487` | **yes** | ledger (`ROADMAP_STATUS.md:166`) and plan now agree |
| **F4** (two-way assert namespace) | `FORM_AUTHORITY_TABLE_DESIGN.md:212-215` | **yes** | `emitted_form_years` does key on `irs_basename` — verified at `cite_check.rs:853`; the citation is exact |
| **F5** (`line_set` two definitions) | `FORM_AUTHORITY_TABLE_DESIGN.md:72`, `:79-87`, `:210-211` | **yes** | revision-named, many-to-one, per-year `line_set`s named. Count wrong → **G4** |
| **F6** (kill needing §5's match) | `FORM_AUTHORITY_TABLE_DESIGN.md:219-220` (moved to step 3) | **partial** | step 3 cannot host it either: ten TY2025 maps name no schema until step 5 → **G2** |
| **F7** (manifest join reds on 6) | `FORM_AUTHORITY_TABLE_DESIGN.md:204` (§9 row), `:216-218` (step 1) | **partial** | the excuse field `authority` is named only in §9/§10; §4's header block and §4's kill list are untouched → **G1** |
| **F8** (1040 has no seq. no.) | `FORM_AUTHORITY_TABLE_DESIGN.md:73`, `:210`, `:218-219`, `:202` (§9) | **partial** | same shape as F7: §4's kill bullet at `:96-97` still states the kill with no exception → folded into **G1** |
| **F9** (FR-47 statute path) | `58200a59`; `FOLLOWUPS.md` FR-47 | **yes** (earlier) | `legal/primary-sources/statute-irc/26USC_s55.html` + `PLAW-119publ21_OBBBA.pdf` present; `legal/text/statute-irc/` exists |
| **F10** (two stale cross-refs) | `ROADMAP_STATUS.md:226-229` (marked history), `:205-208` (four cells) | **yes** | §4's new wording matches `TY2026_WORK_LIST.md:44-49` exactly — four cells, `NO PRIOR SIDE` / `NO DRAFT` |
| **F11** (1099-DA silence) | `TY2026_PORT_REPORT.md:449` (R28), `:540-544` (rule 18), `ROADMAP_STATUS.md:7` (qualified) | **yes** | port report went 0 → 2 `1099-DA` hits; no count claim elsewhere went stale (the "27 risks / 17 rules" string survives only inside the quoted C1 history) |
| **F12** (M1 rows 5/20/21) | `FOLLOWUPS.md:6218-6226` (FR-54) | **yes** | all three rows present, owning phase = the port machine |

## Findings

### G1 — IMPORTANT — step 1's own escape hatch is a header field the header schema does not have, and §4's kill list still contradicts step 1 twice

**Where:** `design/FORM_AUTHORITY_TABLE_DESIGN.md:64-74` (the §4 header block) and `:93-97` (§4's five
B1 kills), versus `:204` (§9) and `:216-219` (§10 step 1); `crates/btctax-forms/src/map.rs:146`
(`#[serde(deny_unknown_fields)]`, one per top-level map struct — 55 in the file).

**What is wrong.** Step 1 says the manifest-join kill is *"expected red on exactly 6 rows … until their
`authority = "not-yet-archived: …"` header is written, which is the excuse slot"*. `authority` appears
nowhere in §4 — the block step 1 points at when it says *"add the header fields to the 37 existing
maps"* — and the same is true of `extract_override`, invented at `:203`. Both are new required-or-optional
header fields introduced in prose, in a design whose own parse gate is `deny_unknown_fields`: today every
top-level struct declares exactly `form`, `year` and `census`, so a map carrying `authority = …` is a
**parse refusal**, not an excuse. A builder transcribing §4 into the structs therefore ends step 1 with
the manifest kill red on six rows and no way to green it — and a red gate is itself a blocking finding
here, so step 1 cannot land.

The same half-fold hit F8. §4's kill bullet still reads *"`attachment_sequence` ≠ the extract's
'Attachment Sequence No.' → red"* and its hash bullet still reads *"that hash absent from `MANIFEST.json`
… → red"*, with no exception in either; both exceptions live only in §10's prose. §4 is where the row and
its kills are defined; §10 is a schedule. Right now they disagree about what the kills are.

**Minimal change:** in §4's block add `authority = "not-yet-archived: <reason>"   # OPTIONAL; only the
six templates with no MANIFEST entry` and `extract_override = "…"   # OPTIONAL; only f1040s1a/2025 until
the fixtures move`, and amend the two §4 kill bullets to carry their exceptions (*"→ red **unless** the
header carries `authority = "not-yet-archived: …"`"*, *"→ red, **except** on a form the IRS prints no
sequence number for (the 1040)"*).

### G2 — IMPORTANT — the `line_set` kill was moved from step 1 to step 3, where ten maps still name no schema, and this time no expected-red count or excuse slot came with it

**Where:** `design/FORM_AUTHORITY_TABLE_DESIGN.md:219-220` versus `:112-114` (§5's match) and `:227-228`
(step 5); `crates/btctax-forms/src/map.rs` (`for_year` coverage).

**What is wrong.** F6's fix says the fifth kill *"needs §5's match and is planted at step 3"*. But step 1
now writes a `line_set` into **all 37** maps, and ten of them have no struct on any timeline before step
5. Measured from `map.rs`'s `tyYYYY`/`for_year` arms against the 37 files on disk, the unwired set is
exactly the ten step 5 already names: `f6251`, `f8959`, `f8960`, `f8995`, `f1040s1a`, `f1040s2`,
`f1040s3`, `f1040sa`, `f1040sb`, `f1040sc` — all 2025. So at step 3 the exhaustive `line_set → struct`
match either has no arm for those ten (the kill fires on ten non-defects, or the build does not compile,
depending on where the `LineSet` set comes from) or it needs a "declared, not yet wired" arm the design
never names. Step 5's own sentence — *"This is where the TY2025 6251 map first parses"* — is the proof
that step 3 cannot be green.

This is the identical shape the fold handled *correctly* one paragraph earlier for the manifest join:
name the expected-red rows, name the excuse slot. The relocated kill got neither, so F6's collision was
moved two steps to the right rather than resolved.

**Minimal change:** in step 1, after the `line_set` clause: *"the ten unwired TY2025 maps (`f6251`,
`f8959`, `f8960`, `f8995`, `f1040s1a`, `f1040s2`, `f1040s3`, `f1040sa`, `f1040sb`, `f1040sc`) carry
`line_set` with no schema behind it until step 5"*; and in step 3: *"the match gains an explicit
`Unwired` arm for those ten — the kill is 'a `line_set` that is neither a schema nor `Unwired`', so
wiring one at step 5 is a deletion from that arm and forgetting one still cannot compile."*

### G3 — IMPORTANT — §9's F2 "Decision" moves two fixtures onto two occupied paths and turns two in-crate `include_str!`s into escaping ones, which §5 of the same document forbids

**Where:** `design/FORM_AUTHORITY_TABLE_DESIGN.md:203` versus `:120-124` (§5, the publishing trap);
`crates/btctax-core/src/tax/tables.rs:1351,1365`.

**What is wrong.** §9's new row rules: *"those two fixtures move to `design/forms/extract/` under the IRS
stem (`f1040s1a--2025.txt`, `i1040gi--2025.txt` pages) and the core tests read them from there."* Both
destinations already exist and hold **different bytes**:

| file | size | sha256 (head) |
|---|---|---|
| `crates/btctax-core/src/tax/fixtures/schedule_1a_2025_form.txt` | 11,443 | `d361c281…` |
| `design/forms/extract/f1040s1a--2025.txt` | 11,153 | `ae0e3d50…` |
| `crates/btctax-core/src/tax/fixtures/schedule_1a_2025_instructions.txt` | 52,672 | `0ba915a2…` |
| `design/forms/extract/i1040gi--2025.txt` | **616,274** | `6d4d8c1e…` |

The second row of that pair is the damaging one: the core fixture is the **pages 101–110 slice**, and the
destination is the **whole i1040gi booklet extract** that every other i1040gi-hosted schedule's doc-comment
gate reads. A literal `mv` silently destroys it. And "the core tests read them from there" means
`include_str!("fixtures/…")` at `tables.rs:1351` and `:1365` becomes `include_str!("../../../../design/…")`
— an **escaping `include_str!`**, which §5 of this same document names as the trap that *"once shipped a
broken tarball with exit 0"*.

The real situation is smaller than the row says: `design/forms/extract/` **already** holds both files
under the convention, so nothing needs to move. What §4's convention genuinely cannot express is *two
different extractions of one `(stem, year)`* — one full booklet, one page slice — which is what
`instr_pages` exists to narrow.

**Minimal change:** replace the Decision sentence with *"the two `btctax-core` fixtures are a second
extraction of files already under the convention (`f1040s1a--2025.txt`, and pages 101–110 of
`i1040gi--2025.txt`). Decision: regenerate the slice at test time from the booklet extract using the
header's `instr_pages`, and delete the fixtures — never move them, since an `include_str!` reaching out
of the crate is §5's publishing trap. Until then the ratchet keeps its row."*

### G4 — MINOR — "~13 maps" is 15, and it is countable

**Where:** `design/FORM_AUTHORITY_TABLE_DESIGN.md:82` and `:210-211`.

**What is wrong.** The five shared structs each serve 2017, 2024 and 2025 (`ty2017`/`ty2024`/`ty2025` on
`Form1040Map`, `Form8949Map`, `Form8283Map`, `ScheduleDMap`, `ScheduleSeMap`), and each of `f1040`,
`f8949`, `f8283`, `schedule_d`, `schedule_se` has a map file in all three year directories: **5 × 3 = 15**.
The review said "~13" and the fold carried the tilde into the instruction a builder executes.

**Minimal change:** "~13" → "15" in both places.

### G5 — MINOR — the I2 withdrawal was widened from the one struct r2 §7 blesses to three, leaving the "repo instance (bad)" column with no repo instance

**Where:** `design/TY2026_PORT_REPORT.md:304` versus `design/FORM_AUTHORITY_TABLE_DESIGN.md:177-180`.

**What is wrong.** r2 §7 withdraws the axis table's bad example for **`schedule_1a.rs`** ("`schedule_1a.rs`
was built under it and reviewed green … listing *it* as the bad example … is withdrawn"), and `CLAUDE.md`'s
amendment names `Form6251Map`, `Schedule1A` and the worksheets. The fold's rewrite blesses all three
former entries — including `printed.rs` (107 fields), whose structs carry no year and are shared across
2017/2024/2025 (`Printed8949Row`, `Printed8949Totals`), i.e. the cross-year case the amendment sends the
other way. The "repo instance (bad)" column now holds *"a cross-year consumer that reads `lineNN` fields
straight off a transcription struct"* — a description, not an instance, in a column whose header promises
one.

**Minimal change:** limit the withdrawal to `schedule_1a.rs` (the struct r2 §7 actually rules on), keep
`printed.rs`/`qbi.rs::Form8995Lines` in the bad column or file their status as an open question, and put a
real citation in the bad cell.

### G6 — MINOR — `ROADMAP_STATUS.md` §3 now says Form 4868 both "NOW" and "join P4" (arrived in 3de5c272, not in the fold)

**Where:** `design/ROADMAP_STATUS.md:176` (NOW bucket: *"Form 4868 + 1040-V (FR-49) and a physical print
rehearsal … done once on a packet before the season (S8)"*) versus `:190` (*"Form 4868 (one AcroForm, an
estimate) and 1040-V join P4"*), and `FOLLOWUPS.md` FR-49 (re-owned from P4 to NOW).

Recorded because it is one line and sits inside a section this fold's siblings edited; it is **not**
18027b03's.

**Minimal change:** `:190` → "Form 4868 and 1040-V are a NOW item (FR-49, re-owned 2026-09-05); P4 only
consumes them."

## §10 step 1 — executable as written NOW? **NO**

F4, F5, F6 and F8 are answered inside step 1 and are no longer blockers. One blocker remains, and it
is new: step 1 tells the builder to green the manifest-join kill by writing
`authority = "not-yet-archived: …"` into six map headers, but §4 — the section that defines the header
and the section step 1 points at — has no such field, and the parse is `deny_unknown_fields`, so writing
it refuses. Step 1 therefore ends with a permanently red gate on six rows (G1).

Sentence to change (`design/FORM_AUTHORITY_TABLE_DESIGN.md:69`, inside §4's block) — add beneath it:

```toml
authority           = "not-yet-archived: <reason>"   # OPTIONAL. The ONLY excuse the MANIFEST join accepts; six rows today (all five TY2017 + f8283/2024)
extract_override    = "…"                            # OPTIONAL. Only while a second extract root exists (f1040s1a/2025); see §9
```

and amend §4's hash kill at `:95` to *"→ red **unless** the header carries `authority =
"not-yet-archived: …"`"*.
