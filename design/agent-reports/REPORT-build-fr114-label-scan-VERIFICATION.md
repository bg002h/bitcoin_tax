# VERIFICATION — controller's machine-check of `REPORT-build-fr114-label-scan.md`

Run before acting on the report. The builder's `r15_stop_list.rs` edit was temporarily reverted (backed
up by file copy, restored from the git object — never `git checkout --`) so the FR-111 fold could land
green on a tree that is red only because of this finding. Checks below therefore use the committed tree.

## Claims checked

| # | claim | result | evidence |
|---|---|---|---|
| 1 | the colliding label is `"4 FMV on date of death"`, macro-generated | ✅ | `sections.rs:3329` — `doc_money!(FieldId::Sa1099Box4Fmv, sa_1099, "4 FMV on date of death",` |
| 2 | it is the **verbatim** Form 1099-SA box 4 caption | ✅ | `design/forms/extract/f1099sa--2025.txt:12,39` — `4 FMV on date of death` |
| 3 | `form_spec()` walks 279 field labels | ✅ | independently pinned by the coverage KAT at `spec/coverage.rs:837` — `field_count, 279` |
| 4 | the subcommand is `stop-list`, not `r15-stop-list` | ✅ | `xtask/src/main.rs:273` — `Some("stop-list") =>` |
| 5 | 30 `SectionId` variants, 30 `SECTIONS` entries | ✅ | `awk` over each body → `30` and `30` |
| 6 | `apply.rs` `row_depth` is an `_`-free match that `E0004`s on a new variant | ✅ | `apply.rs:246` doc — *"Exhaustive so a new `SectionId` is a compile error here."* |

## ★ The finding that matters most is about the CONTROLLER, not the label

The builder refuted **fact 1 of my own brief** (*"zero labels red today"*). My measurement scanned
`label:` string **literals**; **209 of 279 labels (75%) are macro-generated** (`doc_money!`, `doc_text!`)
and invisible to it — and the one reddening label was in the blind 75%.

That is precisely this repo's dominant instrument failure — *green because it never ran over the region
that mattered* — and I committed it **in the brief that was supposed to prevent it**, one commit after
retracting FR-114's premise for the same underlying reason. FR-114's author was not careless: the real
instance was simply somewhere a literal scan could not look. **The correct instrument was the derived
walk the task was built to add**, which is why the defect surfaced the moment it existed.

Standing lesson, earned twice today: **a hand-written scan over one syntactic form is not a measurement
of a set produced by another.** Derive it, or state what it cannot see.

## The collision is real, and neither rule is wrong

| rule | says |
|---|---|
| `CLAUDE.md` — *Transcribe IRS forms, never paraphrase them* | the label must say exactly what box 4 says |
| R15/R9 — the ledger-word ban | no interview prompt may say `transfer` / `lot` / `fmv` |

Both are correct; they simply meet on `Sa1099Box4Fmv`. The builder stopped rather than resolve it, added
no allow list, reworded no label, and weakened no ban. That is the right call and matches the brief.

## Verdict

Report accurate on every measurable claim checked. The red is the finding, not a defect. The resolution
is a design decision about what R15 means across 209 transcribed labels — owner's call, not folded here.
