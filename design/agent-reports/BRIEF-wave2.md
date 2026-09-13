# BRIEF — wave 2. Four parcels, disjoint ownership.

Owner: *"Proceed with wave 2 as you see fit."* Ordered by what binds the owner's own return first.
Every parcel: own worktree, `CARGO_TARGET_DIR=<worktree>/target-<letter>`, **never `/tmp`** (32 GB tmpfs).
**FOREGROUND every command** — FR-175: two agents have now stalled by backgrounding a gate, the second
*with the prohibition in its brief*, and the report is what a stall destroys. `make check` (~20s) does
**not** include `cargo fmt --all --check`; run both. Capture output once and grep it. **Do not commit, do
not push.** No subagents. ★ **Eight briefs in this arc were refuted by their implementer, five of them
mine — if you can disprove a premise, STOP and report it.** That instruction has paid every single time.
★ On an inexplicable LINK error (`undefined hidden symbol: anon.…llvm.…` from a stale `.rlib`), run
`cargo clean -p <crate>` before believing the failure — FR-176 was exactly that, not a code defect.

---

## E — FR-196: the §111(a) state-and-local refund worksheet. **This one binds the owner's return.**

**OWNS:** a new worksheet module under `crates/btctax-core/src/tax/`, plus `return_inputs.rs`.
★ **You do NOT own `return_refuse.rs`** — another agent holds it. So **build the worksheet, do NOT remove
the refusal**; report the exact refusal removal as your final recommendation.

The owner itemizes and their state taxes income, so `StateAndLocalRefundWorksheetNotComputed` blocks their
return in any year a state refund arrives — *"the definition of a second year of itemizing in a state with
income tax."* The refusal is currently correct and says why: §111(a)'s tax-benefit rule makes some or all of
the refund income on Schedule 1 line 1, and the worksheet *"needs last year's Schedule A, its SALT cap, the
standard deduction you could have taken and the §164(b)(6) limitation"*, so btctax *"refuses rather than
guess a figure in the understatement direction."*

**Transcribe the STATE AND LOCAL INCOME TAX REFUND WORKSHEET from the text layer**, line by line, in the
form's own numbering, each field's doc comment carrying the official instruction text verbatim
(`design/forms/extract/i1040gi--*.txt`). ★ Never from a rendered page — reading Form 6251 line 33 off an
image once produced "subtract line 32 from line 12" where the form says **line 22**, a $200,000 error.
★ **If the worksheet asks something our input surface cannot answer, COLLECT it** — that is following
instructions, not scope creep. Prior-year Schedule A figures are the obvious case; `open_next_year::seed`
and `CarryProvenance::ComputedFromPriorReturn` already exist for carrying a figure across years, so check
whether a prior-year return in the vault can supply them before adding a hand-entered field.
B1: a vector where the refund is fully taxable, one where the tax benefit was partial, and one where the
filer took the standard deduction last year so **none** of it is taxable (the refusal's own stated exit).

## F — FR-216, FR-217, FR-215: the box-12 residue, including a self-contradiction

**OWNS:** `return_refuse.rs`, `advisories.rs`, `forms/*/f1040s1.map.toml`, `forms/*/f1040s3.map.toml`.

**FR-216, and start here:** box 14b **advises** about the Schedule 1-A tips deduction while box 12 code
`TP` **refuses** over the same fact on the same W-2. ★★ Two surfaces, one piece of paper, opposite postures
— whichever is right, the product currently disagrees with itself. **Adjudicate that pair first; it decides
the shape for the rest.** Then the other forgone-favourable candidates: `L P Q TT`, plus `II`. Use FR-205's
`ForgoneDeductionAdvised` verdict where it fits, and say plainly which codes are NOT that class and why.

**FR-217:** FR-206's cap sums `D E F G S AA BB EE` against one limit, authority pinned to the 1040's *"under
all plans"* line-1h paragraph after §402(g)(3) was found to enumerate only §401(k), §408(k)(6) SEP and
§403(b). ★ **§457(b) has its OWN limit** (§457(b)(2)/§457(e)(15)), so folding code G in could be wrong in
the *other* direction — refusing a filer inside both limits. And `CLAUDE.md` is explicit that **instructions
are not law**. Adjudicate against the statute, record which limb each code belongs to, and keep the current
fail-closed behaviour if the answer is unclear — but say so rather than leaving it implied.

**FR-215:** repoint `f1040s1.map.toml:173`'s line-24f census row at the advisory FR-205 built. Trivial.

## G — FR-210: `witness_text` misses margin sub-letters, and it hides the worst Schedule A collision

**OWNS:** `crates/xtask/`.

`witness_text` does not pick up the margin sub-letters `a/e/g/h/k` on `f1040sa--2026-DRAFT`, so it prints
`17` three times and axis C **refuses line 17 as ambiguous**. Consequence, measured: **9 of the 13 unread
line numbers** are this, including Schedule A's 17 — so the most consequential collision on the form
(TY2025's line-18 **checkbox** versus TY2026's itemized **total**) currently prints as a **GAP**, and
Schedule A's `8` collisions is a **FLOOR, not a count**.
★ Refusing an ambiguous witness is the **right** behaviour and must survive your fix — *"a skipped field is
not a passed one"* is in the tool's own output. Make the witness unambiguous; do not make the axis guess.
★★ It re-bases three other checkers, which is why the last agent left it alone. Re-run all three and show
them green, and show the previously-unread line numbers now read. B1: plant a sub-letter collision and watch
the axis still refuse it.

## H — FR-212/213/214 and FR-166/167: four stale claims and two surviving year defaults

**OWNS:** `design/ROADMAP_STATUS.md`, `design/SPEC_1099da*.md`, `scripts/oracle/verify_schedule_1a.py`,
`scripts/oracle/check_return.py`.

**FR-212:** `ROADMAP_STATUS.md:63` says *"Only two text cells differ"* on Form 6251. The tool says **8**, and
one of the six it missed is Form 6251 **line 5** — the AMT exemption phase-out threshold — moving MFJ
$1,252,700 → **$1,000,000** and Single $626,350 → **$500,000**, taxpayer-adverse. ★ That row **priced** the
2026-09-11 decision to transcribe f6251. Correct it, and **cite the tool rather than a hand count**.
★ State the reassuring half too: `tax_tables.rs:311` already carries `dec!(500000)`, so no figure is wrong —
this was a wrong *price*, not a wrong number.
**FR-213:** `SPEC_1099da:358`'s *"both forms unchanged"* is false for Form 8949, whose caption axis compared
**0 of 2** lines. Restate it as **unwitnessed** rather than wrong — that is the distinction FR-211 just gave
the table a column for.
**FR-214:** Form 6251's TY2026 text cites *"Form 1040 line 7a"* and is the **only** witness in the tree that
the 1040 renumbers line 7, because the 1040 is archived as NO DRAFT. Record that every TY2026 claim about
1040 line numbering rests on one indirect witness. **Do not resolve it from the citing form.**
**FR-166:** `check_return.py:307` — `year = wrapper_year if wrapper_year is not None else 2024` silently
scores a yearless projection as TY2024 across **all three** engines. Refuse, naming `btctax income project
--year`. **FR-167:** `verify_schedule_1a.py:85` — `def _rows(pol, name, year=2025)`; measure whether any
caller relies on it before deleting.

---

Deliverables, Bash heredoc (**not** `Write`): `REPORT-wave2-E.md` / `-F.md` / `-G.md` / `-H.md` under
`design/agent-reports/`.
