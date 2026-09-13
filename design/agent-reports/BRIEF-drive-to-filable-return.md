# BRIEF — drive a real filer profile ALL THE WAY to a printed packet, and log every wall

**Tier:** opus. **Isolation:** worktree. **DISCOVERY, not construction** — change no source file.
**Owner ask, 2026-09-13:** *"eventually we want to drive all the way to a filable return…even if we don't
know the address to mail it to…we want to find all the bugs we can."*

## 0. The one question

> Can a filer with the owner's actual return shape be driven from raw inputs to a **printed, complete
> packet** — and if not, **exactly where does it stop?**

You are not fixing anything. You are producing the **wall list**: every refusal, panic, blank that should
not be blank, missing page, missing question, and silent zero, in the order a filer meets them.

## 1. The target profile — the owner's real shape, stated 2026-09-13

- **W-2 wages** + **Bitcoin dispositions** (8949 / Schedule D).
- **Itemized deductions on Schedule A** — mortgage interest, SALT, charitable. **Not** the standard deduction.
- **No** Schedule C / self-employment. **No** retirement distributions. Figures are yours to invent; the
  SHAPE is fixed and is the only thing that matters.

Drive **TY2024** — the only year with bundled params, and therefore the only year that can produce a packet
at all. Then say **separately** what this same profile hits in TY2026, on paper, from the extracts.

★ Itemizing is the load-bearing choice. `AbsoluteReturn.deduction` is a `max` of standard vs itemized, so a
profile that itemizes exercises Schedule A, the §164(b) SALT worksheet, and the charitable ceilings —
none of which a standard-deduction filer touches.

## 2. Method — be a filer, not a test

Use the real CLI end to end: `btctax init`, ingest, classify, `btctax income answer` / the input-form
surface, then the full-return export to a filled PDF packet. Work in a scratch directory **outside the
repo**. Read `docs/examples/examples.md` (journeys J1–J6) for the intended paths, then **diverge** — the
divergences are the findings.

At every step, the three journey questions from `CLAUDE.md`:
1. What does the filer have in hand, **exactly**?
2. What does the tool do?
3. ★ **What ELSE might they reasonably do — and what happens then?**

Classify each divergence as **refusal / warning / default / not our concern / documentation only**, and
★ a divergence is a FINDING only if the wrong outcome is **worse than telling the filer nothing**. Without
that rule this method inflates without limit.

## 3. What to hunt, specifically

- ★★ **A page that will not print.** FR-102: an HSA filer could not print a single page, after twelve
  task-level reviews each closed 0C/0I. It was found by a filer walking the product. Try to reproduce that
  class: does *any* plausible combination of this profile refuse the whole packet?
- **A blank that is a defect rather than a choice.** `CLAUDE.md`: a blank is correct when the inputs say so
  and a defect when nothing ever populated it, and the two look identical on the page. For every blank
  money line on the packet, say which it is.
- ★ **A question never asked.** If the packet needs a figure the interview never requested, that is the
  answered-ness defect — and worse, if the tool supplies `0` for it, that **fabricates testimony**, because
  an entry is sworn under §6065 and a blank is no testimony at all.
- **The mailing address is NOT a finding.** Owner ruling (FR-132): the filer can be told to google where to
  file. Do not report its absence.

## 4. Out of scope

Change no file under `crates/`, `scripts/` or `design/forms/`. Do not fix anything you find — **report it**.
Do not run the oracles (figures are not the subject; completeness is). Do not add tests. Do not propose
implementations beyond one line naming what is missing. Nothing about e-file, state returns, or TY2025 as a
filed year (owner ruling: it never is).

## 5. Stop-and-report

Six briefs in this arc were refuted by their implementer, three of them mine, all today. If the profile
cannot be driven for a reason this brief did not anticipate — a missing prerequisite, a year that will not
assemble — **say so and report what you learned**, rather than substituting a different profile silently.

## 6. Working rules

- Own worktree; `CARGO_TARGET_DIR=<your-worktree>/target-drive` (covered by `.gitignore`'s `target-*/`).
  Never a target dir in `/tmp` — 32 GB tmpfs, one filled it and killed a running test.
- ★ **Run everything in the FOREGROUND.** FR-175: an agent that backgrounds a long command and ends its turn
  never writes its report; that cost a round today. Capture output once to a file and grep it.
- **Do not commit, do not push.** No subagents.

## 7. Deliverable

Final action: Bash heredoc (**not** the `Write` tool) to:

    design/agent-reports/RECON-drive-to-filable-return.md

Return a short summary plus that path. Structure: the exact commands run, in order; the **wall list**, each
wall with what the filer saw and what they would reasonably do next; the blank-provenance verdict per money
line; then a separate TY2026 section. Rank by whether a real filer hits it.
