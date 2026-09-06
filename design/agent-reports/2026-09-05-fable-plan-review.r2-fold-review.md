# Fold review — Fable plan review r2 fold (52298b6d)

Reviewer: Claude Opus 5 (1M), independent, read-only. Date: 2026-09-05.

**Read:** `design/agent-reports/2026-09-05-fable-plan-review.md` (all of it);
`design/agent-reports/2026-09-05-fable-plan-review.VERIFICATION.md`; `git diff 3f279d64..52298b6d`
(stat + full diff over `design/`, `CLAUDE.md`, `FOLLOWUPS.md`, `crates/`, plus `CONTINUITY.md` and
`docs/examples/examples.md`); `design/FORM_AUTHORITY_TABLE_DESIGN.md` r2 in full; and, to check the
fold's own executable claims, `crates/xtask/src/cite_check.rs`, `crates/btctax-forms/src/map.rs`,
`crates/btctax-forms/src/packet.rs`, `crates/btctax-cli/src/cmd/admin.rs`, `design/forms/MANIFEST.json`,
`legal/text/irs-guidance/RevProc_2025-32.txt`, `legal/primary-sources/`, `design/TY2026_PORT_REPORT.md`,
`design/ROADMAP_STATUS.md`, `design/LONG_RANGE_PLAN_filing.md`, `design/TY2026_WORK_LIST.md`.

**Not re-derived** (taken as settled per the brief): C1's fix in `118b070b` and its observed-red kill;
the row counts (37 maps / 37 PDFs; 5/17/15); the six registries; the 16 attachment-sequence literals;
the two `selected_year: 2025` literals; every path in the ledger; Rev. Proc. 2025-32 §2.10's contents
and the 0.50 identity; that no statute text sits under `legal/text/`; the owner's rulings; and the
`build.rs`-vs-weaker-alternative design choice.

No file other than this one was written. No cargo/make/git-mutating command was run.

## Verdict: **0 Critical / 7 Important / 5 Minor**

The fold is substantively faithful: every one of the 13 findings is either built, folded into a
document, or filed in `FOLLOWUPS.md` with an owning phase, and nothing was quietly dropped or
argued away. What it misses is **the sibling document**: I1, I2 and M2 were folded into
`CLAUDE.md`, r2 and `ROADMAP_STATUS.md`, and the *original* sentences the review quoted were left
standing in `TY2026_PORT_REPORT.md` and `LONG_RANGE_PLAN_filing.md` — in two cases r2 states that a
sentence "is withdrawn" when it is still on disk (F1, F2, F3). And §10 step 1 is not executable as
written: four of its own instructions collide with the tree it operates on (F4–F7).

## Per-finding trace

| review finding | where the fold responds (file:line or commit) | faithful? | note |
|---|---|---|---|
| **C1** | `118b070b` — `btctax-core/src/forms.rs:200-206`, `tax/printed.rs:71-76,121`, `btctax-cli/src/cmd/admin.rs:1220-1227`, kill at `btctax-cli/tests/promote_cli.rs:1526-1532`; `design/FORM_AUTHORITY_TABLE_DESIGN.md:134,146-150`; `FOLLOWUPS.md` FR-46 | partial, by declared scope | advisory now fires on the full-return arm and is year-aware (`admin.rs:452-465`), so the golden line added at `docs/examples/examples.md:809` correctly reads 1099-B/Box C/F for a TY2024 journey. Minimal-change items 1/2/4 filed as FR-46(a)–(d). But `TY2026_PORT_REPORT.md` still has **zero** occurrences of "1099-DA" and no R28 row → **F11** |
| **I1** | `TY2026_PORT_REPORT.md:264-271` rewritten to the glob shape; r2 §2/§3/§5/§9 | partial | the very next paragraph (`:274-277`) still names the live `cite_check.rs::FORMS` const as "the registry to grow", and r2 never accounts for it → **F2** |
| **I2** | `CLAUDE.md:35-42` (scope amendment); r2 §7 (`:158-170`) | partial | neither port-report site was touched: step 19 (`:257`) still asserts "per-YEAR artifact", and the three-axis table (`:300`) still lists `schedule_1a.rs (56)` as the bad example that r2 says "is withdrawn" → **F1** |
| **I3** | r2 §6 `YEAR.toml` (`:123-150`), §10 step 4; `FOLLOWUPS.md` FR-48, FR-53 | yes | `forms_expected` / `forms_absent` / `oracles` / `prices_through` / `information_returns` all present; `YearReadiness` = declared vs actual |
| **I4** | `TY2026_PORT_REPORT.md:466-480` (rule 1 re-aimed), `:613-620` (Q&A); `tax_tables.rs:926-933` struck through; `ROADMAP_STATUS.md:157`; FR-47 | yes | figures independently re-checked: `RevProc_2025-32.txt:698,700,702,704` = 140,200 / 90,100 / 70,100 / 31,400 and `:715,717` = 122,250 / 244,500. FR-47's statute-archive **path** is wrong → **F9** |
| **I5** | `ROADMAP_STATUS.md:144-171` (NOW / AFTER FINALS / AFTER OTS-2026 + critical path + extension); FR-49 | partial | the plan of record still commits to TY2025-first at `LONG_RANGE_PLAN_filing.md:489` → **F3** |
| **I6** | `TY2026_PORT_REPORT.md:260` (step 24 → M/M/H-residual), `:314-316`; FR-51 | yes | the "honest ceiling: 8 of 24 … (1, 5, 16, 17, 19, 20, 21, 24)" sentence at `:264` is unrevised, which is defensible (24 keeps an H residual) but is now a count of a different thing |
| **I7** | `TY2026_PORT_REPORT.md:367-374` (build-order item 1b); FR-48 | yes | names both `selected_year` sites and the `CommitOutcome::NoTables` mirror |
| **I8** | `TY2026_WORK_LIST.md:37-49` (per-cell table); `TY2026_PORT_REPORT.md:456` (§5d "moot" withdrawn); FR-50 | yes | `ROADMAP_STATUS.md:185` says "two emitted forms" where the fold's own table lists four cells → **F10** |
| **M1** | FR-52(a) (the `versioning` discriminator) and FR-52(c) (which revision to ship); I3 gives step 1's output a home | partial | the rows for steps **1, 5, 20, 21** are neither folded into §3's table nor filed, although the FOLLOWUPS section opens "Everything below is what the fold did NOT build" → **F12** |
| **M2** | `LONG_RANGE_PLAN_filing.md:472-479`; `ROADMAP_STATUS.md:146-147` | partial | §6.4 (`:489`) and the option table's "A — recommended" (`:467`) untouched → **F3** |
| **M3** | r2 §6 `prices_through` + its B1 kill (`:133,144`); FR-53; `ROADMAP_STATUS.md:157` | yes | |
| **M4** | FR-52(b) | yes | |

## Findings

### F1 — IMPORTANT — r2 says the I2 doctrine reversal "is withdrawn"; it is still on disk, in the document that made it

**Where:** `design/FORM_AUTHORITY_TABLE_DESIGN.md:167-170` versus `design/TY2026_PORT_REPORT.md:300`
and `:257`.

**What is wrong.** r2 §7 states: *"`schedule_1a.rs` was built under it and reviewed green; the port
report's three-axis table listing it as the bad example was a doctrine reversal presented as
synthesis, and **is withdrawn**."* The fold edited `TY2026_PORT_REPORT.md` in six places and did not
touch line 300, which still reads `| **quantity** … | `printed.rs` (107), `schedule_1a.rs` (56),
`qbi.rs::Form8995Lines` (16) |` in the **repo instance (bad)** column. The same is true of I2's first
bullet: step 19 at `:257` still says *"the struct is a **per-YEAR artifact wearing a per-FORM name**"*,
which r2 `:28-30` and `:158-161` explicitly contradict ("per-**line-set-revision** artifact"). A
builder reading the plan of record therefore still finds (a) the line-named transcription structs
listed as the bad example and (b) the per-year claim r2 replaced.

Passive withdrawal is exactly the shape I2 objected to — a doctrine reversal that nobody has to
notice. Fixing `CLAUDE.md` protects the *next* struct; it does not un-print the reversal.

**Minimal change:** in `TY2026_PORT_REPORT.md`, strike `schedule_1a.rs (56)` from the bad column
(or move it with a one-line note that a transcription struct is the *right* place for line names),
and rewrite step 19's note to "a per-line-set-revision artifact wearing a per-form name — see design
r2 §7", so the three documents say one thing.

### F2 — IMPORTANT — I1's "`FORMS` in `cite_check.rs` is retired into the headers" is folded in half, and the port report still says to grow it

**Where:** `design/TY2026_PORT_REPORT.md:274-277` (unedited) immediately after the fold's own
`:264-271`; `design/FORM_AUTHORITY_TABLE_DESIGN.md` §9 (`:180-193`).

**What is wrong.** The fold rewrote the "two structural changes" paragraph to the glob shape and the
explicit *"NOT a hand-written `const` of `(stem, year)` rows"*. The paragraph directly below it is
untouched and reads: *"**The registry to grow is already in the tree, holding one row** —
`cite_check.rs:651-672` `FormAuthority { form, year, instructions, instr_pages, extract_stem }` … with
a doc comment that states the recipe: 'adding a form is a table entry plus a transcription'."* That is
an instruction to grow the Rust const table the sentence above it just rejected. r2 never mentions
`FORMS` at all — its only occurrence of the word (`:6`) is r1's own proposed `const FORMS:
&[FormYearRow]`, a different object — and §9's "What the hand-lists become" table omits it, while
listing `STEM_ALIASES`, `CENSUS_KEYS`, `SUPPORTED_YEARS` and the rest.

This is not bookkeeping. The live registry is at `crates/xtask/src/cite_check.rs:725-746` (struct at
`:725`, `pub const FORMS` at `:740`), and its `extract_stem` field points at
`crates/btctax-core/src/tax/fixtures/` (`schedule_1a_2025_form.txt`,
`schedule_1a_2025_instructions.txt`) — a **second** extract root that r2 §4's "derived by convention,
never stored" rule (`design/forms/extract/<irs_stem>--<year>.txt`) cannot express. So retiring `FORMS`
is a decision with a real edge case, and r2 has not made it. The cited line range `651-672` is also
stale (those lines are `sha256_prefix`).

**Minimal change:** add a `FORMS` row to r2 §9 ("→ the map header's `instructions` / `instr_pages`,
plus a stated home for the two `btctax-core` fixture extracts"), and either delete the port report's
"registry to grow" paragraph or re-point it at the header.

### F3 — IMPORTANT — the withdrawn TY2025-first commitment survives in the plan of record, one heading below the paragraph the fold edited

**Where:** `design/LONG_RANGE_PLAN_filing.md:489` (§6.4) and `:467` (option A), versus
`design/ROADMAP_STATUS.md:146-147` and `design/LONG_RANGE_PLAN_filing.md:472-479`.

**What is wrong.** `ROADMAP_STATUS.md:146` now says *"the old 'TY2025 filed after 2026-10-15'
commitment is **withdrawn**"* and points the reader at `LONG_RANGE_PLAN_filing.md` §6.3 for why option
C is cheap. §6.3 was duly edited. §6.4 — titled **"The realistic target, stated as a commitment"** —
was not, and still reads: *"**First filed return: TY2025, filed with btctax, after 2026-10-15.**"*
The option table two paragraphs up still marks **A** as "recommended" while the added text says the
owner chose **C**. `ROADMAP_STATUS.md` is explicitly the *progress ledger* for
`LONG_RANGE_PLAN_filing.md` ("this file is its progress ledger… then the plan for the reasoning"), so
the ledger now withdraws a commitment the plan still states. This is the same defect M2 named, moved
one file over — and the fold was inside that section when it happened.

**Minimal change:** §6.4 → *"First filed return: TY2026, in the 2027 season (owner ruling 2026-09-05,
`ROADMAP_STATUS.md` §0a); the extension is the default plan (§3)."* Re-label option A as the
superseded recommendation and mark C as chosen.

### F4 — IMPORTANT — §10 step 1's two-way assert compares two different namespaces and reds on 6 non-defects

**Where:** `design/FORM_AUTHORITY_TABLE_DESIGN.md:198-199`; `crates/xtask/src/cite_check.rs:852-854`.

**What is wrong.** Step 1 says *"assert `glob == emitted_form_years()` both ways"*. But
`emitted_form_years()` keys its set on the **IRS basename**, not the crate stem —
`out.insert((irs_basename(stem)?.to_string(), year))` at `:853`, with `STEM_ALIASES` mapping
`schedule_d → f1040sd` and `schedule_se → f1040sse` (`:770`). The glob of `forms/<year>/*.map.toml`
yields `schedule_d` / `schedule_se`. On disk that is **6 pairs** (`schedule_d` and `schedule_se` in
2017, 2024 and 2025) that differ in both directions, so the assert as written fails on six rows that
are not defects. r2 knows the answer — `irs_stem` (§4) and §9's "STEM_ALIASES → the `irs_stem` header
field" — but step 1 does not say to translate through it, and step 1 is where the header is first
written.

**Minimal change:** step 1 → *"assert `{ (irs_stem, year) from the glob's headers } ==
emitted_form_years()` both ways"*, which also makes the new `irs_stem` field load-bearing on day one.

### F5 — IMPORTANT — `line_set` has no determinate value for ~13 of the 37 maps under its own definition, and step 1 writes it into all 37

**Where:** `design/FORM_AUTHORITY_TABLE_DESIGN.md:72` and `:28-30`, `:158-161`;
`crates/btctax-forms/src/map.rs:574-772` (`Form1040Map`), `:375-425` (`Form8949Map`), `:882-946`
(`Form8283Map`), `:1165-1268` (`ScheduleDMap`), `ScheduleSeMap` at `:2198+`.

**What is wrong.** §4 defines the field twice, and today the two definitions disagree:

1. *"WHICH transcription struct parses this map"* — a function into the set of existing structs.
2. *"constants-only year ⇒ same `line_set`; renumber ⇒ new one"* — a function into line-set revisions.

r2 §1 asserts the two coincide (*"the struct is a per-**line-set-revision** artifact wearing a
per-form name"*). That is true of `Form6251Map`, which is the one the review measured. It is false of
five others: `Form1040Map` has `ty2017()` / `ty2024()` / `ty2025()` (`:757-767`) and one `for_year`
match (`:770`), absorbing three renumbered revisions with `Option` + `#[serde(default)]` — its own doc
says `line7a` is *"line 7a for 2025, line 7 for 2024, **line 13 for 2017**"* and that 2017 has no
Digital-Asset question. `Form8949Map`, `Form8283Map`, `ScheduleDMap` and `ScheduleSeMap` have the same
three-year shape. So for the ~13 maps those five structs serve, definition (1) says one `line_set` and
definition (2) says three, and step 1 requires a value for every one of the 37.

This matters beyond bookkeeping: `line_set` is the field carrying I2's resolution, §5's exhaustive
match keys on it, and §10 step 5 ("add `f6251/2025`") assumes revision granularity. Deciding it after
the headers are written is the expensive order — the review's own I2 argument, one level down.

**Minimal change:** state in §4 that `line_set` names a **revision**, that several revisions may map
to one struct today, and that the exhaustive match is `line_set → struct` (many-to-one); then add to
§10 step 1 the explicit note that `f1040`, `f8949`, `f8283`, `schedule_d` and `schedule_se` get
per-year `line_set`s pointing at a shared struct, with splitting them filed as later work.

### F6 — IMPORTANT — one of the five §4 kills cannot be planted at step 1, which is the step whose deliverable is the kills

**Where:** `design/FORM_AUTHORITY_TABLE_DESIGN.md:86` versus `:197` and `:205-206`.

**What is wrong.** Step 1 is *"consuming nothing"* and ends *"plant each §4 kill. **Red on any
disagreement is the deliverable.**"* But §4's fourth kill is *"`line_set` naming no schema → **compile
error** via the exhaustive match in §5"*, and §5's `build.rs` + match is step **2/3**. Worse, the
schema `f6251/2025` needs does not exist until step **5**, which r2 itself calls *"where the TY2025
6251 map first parses"*. A builder following step 1 literally must either write the §5 match early
(contradicting "consuming nothing" and step 2's ordering) or silently ship one checker with no
observed red — the exact B1 failure the design exists to prevent.

**Minimal change:** in step 1, list the four kills that are self-contained (missing required field,
`template_sha256` ≠ file, manifest join, `attachment_sequence` ≠ extract) and say explicitly that the
`line_set` kill is planted with the match in step 3.

### F7 — IMPORTANT — the `template_sha256` ⋈ `MANIFEST.json` kill reds on 6 of the 37 rows on day one, and the design has no slot for the excuse

**Where:** `design/FORM_AUTHORITY_TABLE_DESIGN.md:69`, `:84-85`, §9 (`:180-193`);
`crates/xtask/src/cite_check.rs:883` (`AUTHORITY_NOT_YET_ARCHIVED`).

**What is wrong.** Measured by joining every bundled template's sha256 against `design/forms/MANIFEST.json`:
**31 of 37 join; 6 do not** — all five TY2017 templates (`forms/2017/{f1040,f8283,f8949,schedule_d,schedule_se}.pdf`;
`design/forms/` has no `2017/` directory and the manifest has **0** entries under one) and
`forms/2024/f8283.pdf` (only `f8283--2025` is archived; `design/forms/2024/f8283--2024.pdf` does not
exist, though the *extract* does). So the §4 kill *"that hash absent from `MANIFEST.json` … → red"*
cannot go green at step 1 without either archiving six authorities or recording an excuse — and the
repo's existing ratchet already has that excuse list (`AUTHORITY_NOT_YET_ARCHIVED`, a hand-list keyed
`(form, years)` that "may only SHRINK"), which r2 does not mention in §1's registry census, in §9's
"what the hand-lists become", or anywhere else. Two of the six also sit on the open owner decision
about whether TY2017 ships at all, so this is not a decision step 1 can make on its own.

**Minimal change:** add `AUTHORITY_NOT_YET_ARCHIVED` to §9 with its destination (a per-map header
`authority = "not-yet-archived: <reason>"`, or a `YEAR.toml` field), and note in step 1 that the
manifest-join kill is expected red on exactly those 6 rows until that field exists.

### F8 — MINOR — `attachment_sequence` is specified with no exception, and the 1040 has none

**Where:** `design/FORM_AUTHORITY_TABLE_DESIGN.md:73` and `:86`;
`crates/btctax-forms/src/packet.rs:93-97`.

**What is wrong.** §4 annotates the exceptions for its two other partial fields (`instructions` —
*"`""` only for a self-instructing form"*; `instr_pages` — *"only for i1040gi-hosted schedules"*) and
gives `attachment_sequence` none, while step 1 says *"parse them with required fields"*. `packet.rs`
pushes the 1040 with `None` and the comment *"The 1040 itself — no sequence number; it IS the
return"*, which is why there are 16 literals for 17 forms. Three of the 37 maps therefore have no
truthful value, and the kill (`attachment_sequence` ≠ the extract's "Attachment Sequence No.") must
tolerate a form whose extract has no such string.

**Minimal change:** `attachment_sequence = "32"   # absent on the 1040 itself, which carries no
sequence number`.

### F9 — MINOR — FR-47 sends the statute to a new directory; the repo already has a statute archive

**Where:** `FOLLOWUPS.md` FR-47 ("Fetch 26 USC §55(d) as amended and Pub. L. 119-21 §70107 into
`legal/text/statute/` via a `legal/_scripts/fetch_statute_*.sh` sibling"); `TY2026_PORT_REPORT.md:479`.

**What is wrong.** The premise is literally true — `legal/text/` holds four dirs and no statute — but
the conclusion drawn from it is not. `legal/primary-sources/` has **six** dirs including
`statute-irc/`, which holds 16 IRC sections (`26USC_s1.html`, `_s61`, `_s170`, `_s1001` …), each
recorded in `design/forms/MANIFEST.json` as `"kind": "statute", "storage": "committed"`. §55 is simply
not among them. `legal/text/` is the extracted-text mirror of `legal/primary-sources/` and has no
`statute-irc/` or `regulations-cfr/` mirror yet. Following FR-47 as written creates a second,
differently-named home (`legal/text/statute/`) and skips the primary-source half plus
`legal/SHA256SUMS` / `legal/_provenance`.

**Minimal change:** FR-47 → *"fetch `26USC_s55.html` into `legal/primary-sources/statute-irc/` beside
its 16 siblings (manifest `kind: "statute"`), extract to `legal/text/statute-irc/`."*

### F10 — MINOR — two cross-references the fold itself made stale

**Where:** `design/ROADMAP_STATUS.md:204-208` and `:185`.

**What is wrong.** (a) §5 says the escalation trigger was *"the year-package table redesign
(`TY2026_PORT_REPORT.md` §3): one `FormAuthority`-shaped row per `(stem, year)`, with `pdf.rs`,
`map.rs`, `CENSUS_KEYS` and `EMITTED_FORMS` derived from it"* — a description of a §3 the same commit
rewrote to say the opposite. Reading it as history is possible but nothing marks it as such.
(b) §4 says the work list *"omits two emitted forms (`f1040s1`, `f8283`)"*, while the fold's own
`TY2026_WORK_LIST.md:41-46` prints **four** cells (adding `f8275` and `f8995a`) and FR-50 says
"two … plus `f8995a`".

**Minimal change:** mark §5's description "(as §3 read before r2)"; make §4 say "two forms with no row
plus two more the per-cell table now names".

### F11 — MINOR — the port report is still silent on Form 1099-DA after an edition of it, while the ledger line says "C1 fixed"

**Where:** `design/TY2026_PORT_REPORT.md` (grep: **zero** hits for `1099-DA` or `R28`);
`design/ROADMAP_STATUS.md:7`; `FOLLOWUPS.md` FR-46(c)(d).

**What is wrong.** C1's opening measurement was *"Nothing in 27 risks, 24 steps or 17 do-not-build
rules mentions Form 1099-DA."* That is still true at `52298b6d`. The fold edited §6 (rule 1) and §5
(the `f1040s1` row) without adding either the C1 §6 rule or the R28 row, deferring both to "the next
port-report edition" — which this was. Meanwhile the header at `ROADMAP_STATUS.md:7` says "C1 fixed in
`118b070b`" unqualified, so a reader of the two tracking documents sees a closed Critical and a risk
register that never names the regime. (The code fix itself is settled and not re-opened here.)

**Minimal change:** one row in §5 (R28 / LIVE / CRITICAL, pointing at FR-46) and one line in §6; or
qualify `:7` to "C1's advisory path fixed in `118b070b`; the regime work is FR-46".

### F12 — MINOR — M1's four "partly mechanical H step" rows are neither folded nor filed, though the section claims to list everything not built

**Where:** `FOLLOWUPS.md`, the Fable-review section header (*"Everything below is what the fold did
NOT build, each with an owning phase"*) and FR-52; `design/TY2026_PORT_REPORT.md:239-258`.

**What is wrong.** M1's table has six rows. Steps 2 (`forms fetch`) and 11 (which revision to ship)
became FR-52(a) and FR-52(c); step 1's "output must be committed" is answered by `YEAR.toml`. Rows for
steps **5** (instructions stem + page range = `fNNNN`→`iNNNN` plus the two-row alias, then a
`pdftotext` header search — "M\* + confirm"), **20** (the machine can propose the extract's own line
text), and **21** (the `artifact` census rule is mechanical by shape and should be pre-filled) appear
nowhere in the fold, and the port report's step marks for them are unchanged. That is a Minor finding
left un-recorded under a heading that says nothing was.

**Minimal change:** one FR-54 entry carrying M1's remaining three rows, owning phase = the port
machine.

## §10 step 1 — executable as written? **NO**

1. **F4** — `glob == emitted_form_years()` compares crate stems against IRS basenames; it reds on 6
   real pairs (`schedule_d`/`schedule_se` × 3 years) before any defect exists.
2. **F5** — `line_set` must be written into all 37 maps, and its two definitions in §4 give different
   answers for the ~13 maps served by the five structs that span 2017/2024/2025.
3. **F6** — "plant each §4 kill" includes one kill that needs §5's match (step 2/3) and a struct that
   does not exist until step 5, while step 1 is "consuming nothing".
4. **F7/F8** — the manifest-join kill reds on 6 of 37 rows with no excuse slot in the design (two of
   them behind an open owner decision), and `attachment_sequence` has no value for the 1040's 3 maps.

Each is small to write down; three (F5, F6, F7) need a **decision** before the first header is typed,
which is precisely what step 1's "proof before switch" ordering is meant to protect.
