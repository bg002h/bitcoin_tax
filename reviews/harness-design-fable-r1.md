# Harness design — Fable consult r1 (2026-07-30)

**Artifact:** `design/HARNESS.md`
**Brief:** `CONTINUITY.md` §0a (consult #1 — the harness), carried verbatim.
**Reviewer:** Fable, independent, read-only, no subagents.
**Status:** persisted VERBATIM before folding, per `STANDARD_WORKFLOW.md` §2.

---

## Verbatim output

VERDICT: needs-changes

H1: keep — this is the one certain real-fire: git's exit-status coupling mechanically blocks the exact F3 recurrence ("ran the gate, never read the output"), and `make check` at ~6s warm costs nothing; but add two things or it is decoration: (i) an in-suite installed-check (xtask asserting `core.hooksPath`/hook presence, red until wired) — **verified precedent: `scripts/pre-push` is a reviewed, hardened hook that has sat uninstalled ever since (no `core.hooksPath`, `.git/hooks/` holds only samples), so this repo's one prior git-hook already became decoration at the install step** — and (ii) a narrow `PreToolUse` deny on `Bash` matching `git commit --no-verify`, which closes the route-around with a fact-gate, not an instruction.

H2: keep — it reds today on a real defect and will fire on any future second archive, which is the definition of a gate that would otherwise be walked past; one sharpening: the detector must classify files by *shape* (IRS stems `fNNNN`/`iNNNN`/`pNNNN`, USC/CFR signatures) over a whole-tree walk with a shrink-only excuse ratchet (`authority_coverage_may_only_improve` is the in-repo model), never by a hand-list of known directory paths — a path-list version is F2 committed inside the harness and passes a third archive at a new location.

H3: change-to a two-part hook — **as specified it provably would not have fired on the incident that motivates it**: `design/forms/` is a new *depth-2* directory under long-existing `design/`, so a "new top-level path" trigger walks straight past F1; part (i), the real fire: run H2's shape-detector at `Write` time and *deny* primary-source-shaped files outside the declared authority tree (a fact-gate — this catches F1 dead); part (ii), the ask: fire on any new *directory* creation at any depth (rare enough not to be muted), and note that a Claude Code PreToolUse exit-2 block feeds its stderr back into the model's loop at the decision point — for a failure of inattention (F1 was never "considered and refused") that is materially different from passive context, and its message should *quote the trigger-relevant memory line*, which is the correct answer to scope-question (c): keep memory as principles, but wire the two or three trigger-shaped ones into hook messages so doctrine surfaces at the moment it is answerable.

H4: change-to a "seen-red-once" rule, drop the lint — the lint would only look like rigour (Rust test code is legitimately full of literal arrays and ranges; an advisory firing on those gets ignored within a week), and your suspicion (a) is right; the sharper observation, answering (e): **F2 and F4 are the same failure — a measuring instrument shipped without ever being seen to discriminate** — and the mechanism that catches both is the one already modelled in `cite_check.rs`'s own tests (`a_paraphrase_is_rejected_and_the_real_sentence_is_accepted`): every new census/conformance/citation checker must land paired with a negative test that plants the defect it exists to catch and asserts red, because *writing the kill is the act that discovers the blindness* (that is literally how F4 was found), so this produces real future failures rather than the appearance of them.

H5: change-to pass-by-path — the lint has no target: **verified, no committed file in this repo contains `.slice(`/`.substring(` into an agent prompt; the truncating code was ephemeral orchestration that a lint over files can never see**, so H5 as written is pure decoration; the structural fix is a payload protocol in the standing workflow briefs (CONTINUITY-style): inter-agent payloads move as *file paths the receiver reads*, never inlined content — truncation then has no operator on the path at all, which removes the class instead of shadowing one symptom.

MISSING:
1. **The harness-is-installed gate** — one xtask check inside `make check` that reds when the hooks are not wired (`core.hooksPath` unset, hook file missing or non-executable), mutation-verified by unsetting the config. This is the highest-value addition because without it H1 and H3 repeat F4 on day one — and the repo already contains the corpse of exactly this failure (`scripts/pre-push`). It also genuinely fires: every fresh clone or mis-configured worktree reds immediately.
2. **The checker-kill pairing as a standing rule, not a per-case memory** (the H4 replacement above): "no checker counts as existing until it has been observed red on a planted defect" — this is the project's own mutation doctrine applied to its instruments, covers F2+F4 as one class, and unlike a lint it cannot be satisfied performatively, because an honest kill-test for a blind checker cannot be written without finding the blindness.
3. **Answer to scope-question (e), which reshapes the whole document**: F1–F5 are two problems, not five — (α) *acted without observing an available fact* (F1, F3), cured by decision-point fact-gates that couple the act to the fact (H1, H3-part-i); (β) *shipped an instrument never seen discriminating* (F2, F4, and F5 — a harness whose truncation manufactures findings is an instrument defect), cured by seen-red-once and pass-by-path. Restructuring the harness around these two classes, rather than five symptoms, is what makes it forward-looking instead of an enumeration of the outcomes one session happened to produce — the exact excuse-list mistake the document itself warns against. On scope-question (d): no — the required opening already exists (CONTINUITY.md §0, and this consult is evidence it is followed), and a checkpoint cadence is the forbidden self-verification scaffolding; the only session-shaping worth having is the decision-point hooks already covered.

WHAT WOULD MAKE THIS WRONG:
All of this assumes the failures recur in mechanically recognizable form — if the next session's violations are new *classes* rather than new instances of (α) act-without-looking and (β) unproven instruments, every gate here holds green while the new failure walks past, and the only defense against that is the document's own closing rule: treat the harness as unproven until it has failed someone at least once, and grow it only from observed failures, never from anticipated ones.

---

## Independent verification of the three load-bearing factual claims

Checked by the orchestrator against the tree at `61b9af7` **before** folding. All three are the
claims that change the design, so each was verified rather than accepted.

| claim | verdict | evidence |
|---|---|---|
| `scripts/pre-push` is a real, executable, **uninstalled** hook — this repo already has a git hook that became decoration at the install step | **CONFIRMED** | `-rwxr-xr-x … scripts/pre-push` dated Jul 2; `git config --get core.hooksPath` → exit 1 (unset); `.git/hooks/` contains **only** `*.sample` files |
| H3 as specified **would not have fired on F1** — `design/forms/` is depth-2 under a long-existing `design/` | **CONFIRMED** | `design/` first added **2026-06-28** (`29988b6`); `design/forms/` first added **2026-07-30** (`f38bc6e`). A "new top-level path" trigger sees no new top-level path. |
| H5 has **no target** — no committed file truncates a payload into an agent prompt | **CONFIRMED** | `git grep -E '\.(slice\|substring)\('` over `*.js *.ts *.md *.rs` returns exactly two hits: `CONTINUITY.md:67` and `design/HARNESS.md:73` — i.e. only the two places *describing the proposed lint*. Zero occurrences in code. |

Note on the second row: the orchestrator's first check used `git log -1`, which returns the
**newest** add rather than the oldest, and reported `design/` as dating from today. Re-run with
`--reverse | head -1`. The corrected result is the one tabled above.
