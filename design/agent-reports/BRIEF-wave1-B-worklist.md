# BRIEF B — FR-209: re-derive every TY2026 work-list verdict from the three new axes. FR-211 too.

**opus. worktree. YOU OWN: `design/TY2026_WORK_LIST.md` and the work-list code in `crates/xtask/`.**
No subagents. ★ **Do NOT touch `form_delta.rs`'s axis logic or `label_reader.rs`** — they landed hours ago;
you are a CONSUMER of them.

## FR-209 — the plan is wrong about five forms

The three axes landed in `2d2c3ff7c`. On their first real run, **five forms recorded `unchanged` (or 0/0/0)
collectively move the meaning of 30 line numbers**:

| form | work list says | meanings moved |
|---|---|---|
| `f6251` | `unchanged` | **8** |
| `f8959` | `unchanged` | **12** |
| `f8960` | `unchanged` | **2** |
| `f1040sb` | `unchanged` | **2** |
| `f8995a` | `port` 0/0/0 | **6** |
| `f1040sd` | `unchanged` | 0 — the only genuinely unchanged form |

★ Controller-reproduced for f6251: *"★★★ 8 of 59 COMPARED surviving line number(s) now print a MATERIALLY
DIFFERENT caption … line 18's old text now prints at line 39 AND WAS REWORDED (similarity 0.62)."*

**What to do.** Re-derive **every** row's verdict by running the axes, and make the table say what the tool
says. ★★ **Do not hand-transcribe the numbers** — that is the disease this whole day was about. Generate the
row from the tool, or add a test that asserts the committed table equals what the tool prints, so the next
drift is a build error. `the_committed_work_list_matches_form_delta_at_head` already exists; extend it to the
new columns rather than writing a parallel checker.

★ **Preserve the distinction that caused this.** f6251's field map DOES transfer (62 fields, 0 renamed, 0
moved) and the 2026-09-11 ruling that greenlit it was right. `unchanged` was a claim about the FORM. Make the
table incapable of conflating the two again — separate columns, separate words.

## FR-211 — a cell that cannot tell zero from unmeasured

The caption columns cannot distinguish *"0 collisions because nothing changed"* from *"0 collisions because N
lines could not be compared"* — which is Schedule A's actual state (FR-210: `witness_text` misses margin
sub-letters so `17` prints three times and the axis refuses it as ambiguous). **Add the gap count beside the
collision count.** ★ Same rule as everywhere here: a zero and an unmeasured are identical in that cell and are
not the same thing.

## Rules

**FOREGROUND every command** (FR-175). `make check` + `cargo fmt --all --check`. Capture once, grep. **Do not
commit or push.** Do not fix FR-210 — report what it costs you. If you can disprove any number above, **stop
and report it**.
Deliverable: Bash heredoc (not `Write`) → `design/agent-reports/REPORT-wave1-B.md`.
