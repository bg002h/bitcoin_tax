# BRIEF — wave 4. Two parcels: the live TY2024 defect, and the §68 refusal.

**Owner, 2026-09-13**, asked whether I was proposing to invent values for the January unknowns, then
accepted these two. **The answer was no, and parcel N is the reason:** we cannot transcribe a worksheet
whose document does not exist, so we **refuse** instead of computing. Refusing is how you avoid making up
values — it is the opposite of guessing.

Standing rules: own worktree, `CARGO_TARGET_DIR=<worktree>/target-<m|n>`, never `/tmp` (32 GB tmpfs).
**FOREGROUND every command** — FR-175: two agents have stalled by backgrounding a gate, the second *with
the prohibition in its brief*, and the report is what a stall destroys. `make check` excludes
`cargo fmt --all --check`; run both. Capture once, grep. **Do not commit, do not push.** No subagents.
★ Two distinct concurrency failures: `ld.lld: undefined hidden symbol` from a stale `.rlib` →
`cargo clean -p <crate>`; `signal: 9, SIGKILL` → memory, run nextest and clippy serially.
★★ **Thirteen briefs in this arc were refuted by their implementer, nine of them mine — including the
stage-2 design's §2 gate, which a plant showed to be insufficient hours after I verified its kill. If you
can disprove a premise, STOP and report it.**

---

## M — FR-225. ⛔ BLOCKING and LIVE on TY2024, the year that actually files.

**OWNS:** `crates/btctax-cli/src/year_readiness.rs`, `crates/btctax-cli/src/cmd/answer.rs`,
`crates/btctax-core/src/tax/interview_state.rs`. ★ **NOT `return_refuse.rs`** — parcel N holds it.

With a charitable gift whose §170(f)(8)(A) acknowledgment is unresolved, on stock TY2024:
`income answer` prints **`interview: complete · return: computable`** and **exits 0**; `report` prints
**`NOT COMPUTABLE [CharitableCwaUnresolved]`**; export writes **nothing**.

★★★ **The root cause is a conflation, not a wrong label.** `year_readiness.rs:254,259` computes
`interview: complete` from **answered-ness** and then prints `return: computable` from it — but **a recorded
answer can still refuse.** `interview_state.rs:802` classifies `Declined` as *"FORGOING, marked, and never
BLOCKING"*, and I verified at `return_1040.rs:3434` and `:3466` that **both `None` and `Some(false)` refuse**
for this reason. So nothing but `Some(true)` lets the return compute, and it is not forgoable in any sense.

★★ **There is a precedent, and it is the shape to follow rather than invent around.**
`crates/btctax-tui-edit/src/edit/form.rs:4742` already records *"★★★ Consequence 2 — the completeness
instrument reported a false `interview: complete`"*, with an assertion at `:4767`. **This class has been
found here before and guarded in one place only. Extend that guarding to the source rather than patching
another symptom.**

Three defects, all in one path — fix all three:
1. **The false claim.** `return: computable` must be the *return's* verdict, not the interview's. The two are
   different questions and must not be answered by one predicate.
2. **The exit that cannot exit.** The refusal names `btctax income answer`, which asks the question **zero**
   times once an answer is recorded; only `--re-answer` reaches it, and that flag appears only in
   `cli.rs:681`'s help text — never in the refusal that requires it. ★ Also: the refusal's other stated cure,
   *"remove that gift from the deduction"*, **has no CLI verb at all.** Say so if you cannot give it one.
3. **The misclassification.** A refusal that fires on both `None` and `Some(false)` is not *FORGOING*.

**B1:** a return in this state must NOT print `computable`; the printed remedy must reach the question; and
plant each fix back to watch its own test red. ★ Do not settle for asserting the string — assert that the
claim and the return's actual verdict **agree**, so a future divergence reds wherever it appears.

---

## N — the §68 refusal. Refuse what we cannot compute.

**OWNS:** `crates/btctax-core/src/tax/return_refuse.rs`, and the threshold's home (`tables.rs` /
`FullReturnParams`). ★ **NOT `interview_state.rs`, `year_readiness.rs` or `cmd/answer.rs`** — parcel M holds
those.

TY2026 Schedule A moves the itemized total to **line 18** behind a new §68-style gate. **Zero §68 modelling
exists anywhere in the workspace** (measured). So a TY2026 return whose income clears the threshold would
file an **unlimited** itemized total — deducting more than allowed, i.e. **understating tax**, the direction
this project treats as worse. Today that is **silent**.

**We cannot compute the limitation:** the Itemized Deductions Worksheet lives in `i1040gi--2026` /
`i1040sca--2026`, **neither archived**. So do not attempt it. **Refuse, by name, with the threshold quoted.**

★★★ **The honesty constraint, and it is the crux.** The gate's printed text reads *"Is the amount on Form
1040 or 1040-SR, **line 11b**, minus the amounts on lines 13a and 13b more than $384,350?"* — and the TY2026
1040 is **NOT ARCHIVED** (FR-214: Form 6251's own text is the single indirect witness that the 1040
renumbers line 7). **So key the refusal on a SEMANTIC quantity this code already computes** — AGI less the
QBI deduction less the Schedule 1-A deduction — **never on a 1040 line number we cannot verify.** State that
choice in the source, with FR-214 as the reason.
★ `$384,350` is printed **once on `f1040sa--2026-DRAFT`, with no filing-status parenthetical** (unlike 5e
directly above it, which does carry one). Transcribe it from that draft. **Do not** read it from the bracket
table: FR-186 records that the same numeral is the TY2026 **MFS 37% bracket start**, and a Single filer read
from the table would get $640,600 and be screened OUT of a limitation the form screens them INTO.
★ Scope it to the years whose Schedule A actually prints this gate — derive that from the extracts, do not
type a year list.

**B1:** a TY2026 itemizing return above the threshold refuses **by name**, quoting the form; one below it
files unchanged; and the refusal names what the filer must do (work the worksheet by hand, or use a
preparer) rather than offering a remedy that does not exist.

---

Deliverables, Bash heredoc (**not** `Write`): `design/agent-reports/REPORT-wave4-M.md` / `-N.md`.
