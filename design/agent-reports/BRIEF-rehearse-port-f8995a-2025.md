# BRIEF — the PORT REHEARSAL: walk the 24-step runbook on `f8995a/2025`

**Owner approved 2026-09-12.** Phase 5's machinery (design r2 §10 steps 1–5) is built and step 5 closed at
0C/0I — but **the port sequence itself has never been run.** Steps 1–5 were built against years already in
the tree. Nothing has gone *"a new year's authority arrives → it becomes a filled, verified form."* That is
the untested thing, and it is on January 2027's critical path.

You are ONE opus agent in an **isolated worktree**. This is a rehearsal: **nothing you build is kept.**

---

## 0. ★★★ THE GOAL IS FRICTION, NOT A WORKING FORM

**Reaching a wired, filling `f8995a/2025` is NOT success, and pushing through to get there is a failure of
this brief.** The deliverable is a **written findings list**: what was unclear, missing, undocumented,
stale, or wrong, each with a severity and an owning phase.

So: when you hit friction, **write it down**. Continue if continuing is cheap and honest. **Stop and report
if the only way forward is a workaround a real January port could not use** — inventing one hides the very
thing this exercise exists to find. "I could not complete step N because X" is a *result*, not a failure.

Do not optimise for a green gate. Optimise for an accurate account of what a port costs.

## 1. The target, and why it is the only right one

`f8995a/2025` — measured today:

| | state |
|---|---|
| archived authority | `design/forms/2025/f8995a--2025.pdf` present |
| text extract | `design/forms/extract/f8995a--2025.txt` present |
| bundled template | **absent** |
| `.map.toml` | **absent** |
| prior year to port FROM | `crates/btctax-forms/forms/2024/f8995a.{map.toml,pdf}` — a **complete** map (45,436 B) and template |

It is the **only** form where the prior year has a full map and the new year has an archived **FINAL**
authority with nothing built. That is January's exact shape, on a final document — so **nothing here is
transcribed from a draft**, and `Entry::is_draft` is not in play.

★ Note the work list's `f8995a` row (`111 common / 3 added / 0 removed / 0 moved → port`) is the **2025→2026**
delta. The **2024→2025** delta is NOT measured anywhere; computing it is step 13's job and one of your
findings.

## 2. What you are testing — `TY2026_PORT_REPORT.md` §3's runbook

Read *"The runbook, in dependency order"* in `design/TY2026_PORT_REPORT.md` §3 **in full**. It is 24 steps,
each tagged **M** (a machine can do it), **M-star** (mechanical by nature, manual today), or **H** (someone
must read the form), with a "today" column making concrete claims about tooling.

**Walk all 24 steps for `f8995a/2025` and report, per step:**

1. **Is the tag right?** (an M that turned out to need judgment, or an H that was mechanical)
2. **Is the "today" column still true?** Several of its claims are about tooling and were written before
   steps 1–5 landed.
3. **What did it actually cost** — commands run, files touched, hand-edits, and anything you had to work out
   that the runbook does not say.

### ★★★ Step 22 is the payoff question, and nobody has measured it

Step 22 ("emit the five code bindings") says **"85 hand-edits for TY2024"**. Steps 1–5 of the year-package
table were built precisely to turn those into a data change — the `build.rs` glob, `LineSet`/`Schema`,
`YEAR.toml`. **Nobody has measured whether they did.** Count the hand-edits step 22 actually takes today,
exactly, and say what the 85 became. That single number is the clearest statement of whether Phase 5
delivered what it was for.

★ Other "today" claims worth testing rather than trusting — this session has repeatedly found claims about
instruments that were written without running them (FR-122 was a follow-up whose central claim was false for
exactly that reason): step 2 *"no committed script"*, step 8 *"absent"*, step 9 *"broken on drafts"* (your
target is final — does it work?), step 11 *"needs a 2-row alias table"*, step 18's check, and step 24.

## 3. Rules while you work

- **Isolated worktree; `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`.** Nothing is merged. Do
  not commit or push. The worktree is thrown away; the findings survive.
- Do **not** bundle any `FullReturnParams`, do **not** touch the fail-closed gates, and do **not** try to
  make TY2025 filable — the owner ruled 2026-09-12 that TY2025 is never filed. **This rehearsal is justified
  purely as machinery practice for TY2026**, which is why its output is findings rather than a shipped form.
- Transcription rules still bind anything you do write: one field per numbered line, from the **text layer**,
  official text verbatim as the doc comment, `[census]` reasons for unmapped fields. If a step asks for
  judgment, exercise it and **record what you had to decide** — those are the H steps and they are the ones
  that cost real time in January.
- **No subagents.** Never hand-count what a tool can count; never quote a number from a `head`/`tail` view.
- ★ A worktree suite result is not comparable to the main tree's — 2026-09-09 saw 7 environmental failures
  there. Do not report a suite red without showing the mechanism is in the code.

## 4. Your report — final action

★ **Write it with a Bash heredoc**, not the `Write` tool — i.e. `cat > <path> <<'MARKER'` … `MARKER`, using
any marker you like. `Write` is refused for report files in subagent harnesses (FR-129, three occurrences);
Bash is permitted, and this restores the agent-as-scribe property. Path:

    design/agent-reports/REPORT-rehearse-port-f8995a-2025.md

Then return only a short summary plus that path.

The report must contain: **the 24-step table** with your per-step verdict (tag right? today true? actual
cost); **step 22's measured hand-edit count**; the **2024→2025 delta** you computed; every finding with a
severity and a proposed owning phase; which runbook claims you **refuted**; and — since this is a rehearsal —
an explicit statement of **what you did NOT complete and why**.
