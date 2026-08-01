# §G-11 SPEC r2 re-review — ENGINEERING LENS (Opus)

**Date:** 2026-07-31 · **Artifact:** `SPEC.md` r2 @ `104197d` · **Brief:** [`BRIEF-spec-review-r2.md`](../BRIEF-spec-review-r2.md)

**Result: 0 Critical / 2 Important / 2 Minor.** The fold is real; the residue is two things r2 did to
itself while fixing r1.

★★★ **I-1 is C-1 ONE LEVEL UP, and the reviewer names it as such.** r1's Critical was a hand-list of
*productions*; r2 fixed it — and then wrote a hand-list of **files**. Same defect, different axis:
*"an enumeration by outcome observed rather than by mechanism, which CLAUDE.md bans for line sets and
for oracle excuse lists for exactly this reason."* It is wrong in both directions — `forms.rs`'s
`ScheduleDTotals` is a **ledger aggregate**, not a printed-line struct (no §3 production covers "Σ over
ledger legs", and it is public API read numerically outside any emitter), while `form8275.rs::Part1Item
::amount` **reaches paper and is in no scope list at all.** The `Default` inventory is likewise a count
of one where there are four, two of them load-bearing. ★ And §2.2's header says "four files" over a
list of five — the tell.

★★★ **I-2: r1 proposed `Option<Ordering>`; r2 dropped the `Option` when folding, and it matters.**
Measured, not argued: a derived `Ord` on the opaque newtype gives **`Blank > Entered(0) == true`**,
decided silently by variant declaration order. r2 named three comparison sites; there are **six**, and
two of the unnamed ones are in the published `btctax-cli` and route the return — `render.rs:1567`
is the **REFUND vs AMOUNT OWED** branch, and `form6251.rs:246 must_attach()` decides whether Form 6251
is attached at all. A comparator that answers `bool` for a blank operand is a **liveness predicate
answering for the filer** — the very class §2.2 exists to close.

★ **What it confirmed by RUNNING it, not reasoning:** the opaque newtype is buildable and correctly
priced (no `Hash`, no `Ord`, no `const` construction, no serde, no `impl From` anywhere in scope —
`Default` is the only trait obstacle); and **the `clippy.toml` mechanism fires end to end** under the
repo's exact invocation, for an inherent method on a type in another workspace crate, in both `src/`
and `tests/` targets. ★ With a trap worth keeping: the lint path only resolves because the module chain
is fully public — a private module would make it a **silent no-op**, so `clippy.toml` needs its own
planted-defect kill like everything else.

★ **And it caught a false claim I inherited verbatim from r1's own ALSO-CHECKED-SOUND:** §4's "no
struct-update syntax" is false — `printed.rs:180 ..r.clone()` and `:211 ..i.clone()`. The guarantee
survives (no money field rides the `..`), but a "verified clean" that is wrong is what B1 exists to stop.

**Verbatim below.**

---

{"parentUuid":null,"isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":"You are the ENGINEERING LENS re-reviewing §G-11 SPEC r2, btctax repo at /scratch/code/bitcoin_tax (branch `main`, commit 104197d).

FIRST read the re-review brief: /scratch/code/bitcoin_tax/design/no-testimony/BRIEF-spec-review-r2.md
Then the artifact: /scratch/code/bitcoin_tax/design/no-testimony/SPEC.md
Then r1's engineering review, whose fold you are checking: /scratch/code/bitcoin_tax/design/no-testimony/reviews/SPEC-engineering-opus-r1.md

YOUR JOB — verify the fold is real, not cosmetic. In priority order:

**(1) THE OPAQUE NEWTYPE — you recommended it; now price it properly.** r2 §2.2 adopts `pub struct LineEntry(Repr); enum Repr { Entered(Usd), Blank }` with `Repr` private. Verify this is actually buildable against the real code:
- Does `LineEntry` need `Copy`, `Eq`, `PartialEq`, `Debug`, `Hash`, or `const` construction anywhere? Check every printed struct's derives in printed.rs, other_taxes.rs, qbi.rs, form6251.rs, forms.rs.
- r1 inventoried ~25 production consumers (btctax-cli/src/render.rs ~23 reads, three emitter comparison sites, the oracle harness). Does r2's published-exit set — `is_blank()`, a formatter, an ordering comparator — ACTUALLY cover all of them? Go read those sites and say which, if any, still need something r2 does not publish. A site that needs an exit r2 lacks is where the author reaches for a workaround.
- Does `clippy.toml` `disallowed-methods` actually work on whatever numeric exit gets published? Confirm the mechanism (the repo has no clippy.toml today; `make check` runs `clippy --workspace --all-targets -- -D warnings`).

**(2) P0b's BYTE-IDENTITY CLAIM over the FOUR-FILE scope.** r1 verified byte-identity for the `printed.rs`/`push_money` path. r2 widened scope to include `other_taxes.rs`, `qbi.rs`, `form6251.rs` and `forms.rs` — and the crypto-slice `ScheduleDTotals` path (`schedule_d.rs:60`) never calls `push_money` at all, using `fmt_money` directly. Does \"byte-unchanged, all entered\" still hold across that whole surface? Trace the paths that do NOT go through `push_money`.

**(3) Did the fold break anything r1 had cleared?** r1's ALSO-CHECKED-SOUND list is settled, but r2's changes could invalidate parts of it — particularly the geometric-safety finding and the `no_unmapped_filled` census interaction, now that P0b retypes ~110 more fields.

Report per the brief's format. PREFER RUNNING THINGS — write a scratch crate under /tmp to test the newtype's trait requirements, grep the consumer sites, read the derives. GREEN IS THE EXPECTED OUTCOME if the fold is real; say so plainly and stop rather than padding.

CONSTRAINTS: READ-ONLY on tracked files, no commits, no subagents. Scratch files under /tmp are fine. Your final message IS the review and will be persisted verbatim."},"uuid":"d34afdf3-eff3-418a-99a4-99c1480d42ff","timestamp":"2026-08-01T04:50:39.599Z","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"d34afdf3-eff3-418a-99a4-99c1480d42ff","isSidechain":true,"agentId":"ae58f1d220d8c3940","attachment":{"type":"deferred_tools_delta","addedNames":["EnterWorktree","ExitWorktree","Monitor","NotebookEdit","SendMessage","TaskStop","WebFetch","WebSearch","mcp__claude_ai_Gmail__authenticate","mcp__claude_ai_Gmail__complete_authentication","mcp__claude_ai_Google_Calendar__authenticate","mcp__claude_ai_Google_Calendar__complete_authentication","mcp__claude_ai_Google_Drive__authenticate","mcp__claude_ai_Google_Drive__complete_authentication","mcp__plugin_context7_context7__query-docs","mcp__plugin_context7_context7__resolve-library-id","mcp__plugin_playwright_playwright__browser_click","mcp__plugin_playwright_playwright__browser_close","mcp__plugin_playwright_playwright__browser_console_messages","mcp__plugin_playwright_playwright__browser_drag","mcp__plugin_playwright_playwright__browser_drop","mcp__plugin_playwright_playwright__browser_evaluate","mcp__plugin_playwright_playwright__browser_file_upload","mcp__plugin_playwright_playwright__browser_fill_form","mcp__plugin_playwright_playwright__browser_find","mcp__plugin_playwright_playwright__browser_handle_dialog","mcp__plugin_playwright_playwright__browser_hover","mcp__plugin_playwright_playwright__browser_navigate","mcp__plugin_playwright_playwright__browser_navigate_back","mcp__plugin_playwright_playwright__browser_network_request","mcp__plugin_playwright_playwright__browser_network_requests","mcp__plugin_playwright_playwright__browser_press_key","mcp__plugin_playwright_playwright__browser_resize","mcp__plugin_playwright_playwright__browser_run_code_unsafe","mcp__plugin_playwright_playwright__browser_select_option","mcp__plugin_playwright_playwright__browser_snapshot","mcp__plugin_playwright_playwright__browser_tabs","mcp__plugin_playwright_playwright__browser_take_screenshot","mcp__plugin_playwright_playwright__browser_type","mcp__plugin_playwright_playwright__browser_wait_for"],"addedLines":["EnterWorktree","ExitWorktree","Monitor","NotebookEdit","SendMessage","TaskStop","WebFetch","WebSearch","mcp__claude_ai_Gmail__authenticate","mcp__claude_ai_Gmail__complete_authentication","mcp__claude_ai_Google_Calendar__authenticate","mcp__claude_ai_Google_Calendar__complete_authentication","mcp__claude_ai_Google_Drive__authenticate","mcp__claude_ai_Google_Drive__complete_authentication","mcp__plugin_context7_context7__query-docs","mcp__plugin_context7_context7__resolve-library-id","mcp__plugin_playwright_playwright__browser_click","mcp__plugin_playwright_playwright__browser_close","mcp__plugin_playwright_playwright__browser_console_messages","mcp__plugin_playwright_playwright__browser_drag","mcp__plugin_playwright_playwright__browser_drop","mcp__plugin_playwright_playwright__browser_evaluate","mcp__plugin_playwright_playwright__browser_file_upload","mcp__plugin_playwright_playwright__browser_fill_form","mcp__plugin_playwright_playwright__browser_find","mcp__plugin_playwright_playwright__browser_handle_dialog","mcp__plugin_playwright_playwright__browser_hover","mcp__plugin_playwright_playwright__browser_navigate","mcp__plugin_playwright_playwright__browser_navigate_back","mcp__plugin_playwright_playwright__browser_network_request","mcp__plugin_playwright_playwright__browser_network_requests","mcp__plugin_playwright_playwright__browser_press_key","mcp__plugin_playwright_playwright__browser_resize","mcp__plugin_playwright_playwright__browser_run_code_unsafe","mcp__plugin_playwright_playwright__browser_select_option","mcp__plugin_playwright_playwright__browser_snapshot","mcp__plugin_playwright_playwright__browser_tabs","mcp__plugin_playwright_playwright__browser_take_screenshot","mcp__plugin_playwright_playwright__browser_type","mcp__plugin_playwright_playwright__browser_wait_for"],"removedNames":[],"readdedNames":[]},"type":"attachment","uuid":"f538b08d-678f-4a4d-a6ee-20b59e77a0c9","timestamp":"2026-08-01T04:50:39.602Z","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"f538b08d-678f-4a4d-a6ee-20b59e77a0c9","isSidechain":true,"agentId":"ae58f1d220d8c3940","attachment":{"type":"skill_listing","content":"- cycle-prep: Use when prepping a release cycle from one or more FOLLOWUP slugs (e.g. the user types \"/cycle-prep <slug>...\" or asks to vet a FOLLOWUP before building). Runs a P0 STRICT-GATE recon that verifies each slug's cited file paths / line numbers / factual claims against the CURRENT origin/master source (citations decay every merge), flags drift / structural errors / claim-counting ambiguities, and recommends cycle scope + lockstep. Recon only — does NOT implement.
- code-review:code-review: Code review a pull request
- feature-dev:feature-dev: Guided feature development with codebase understanding and architecture focus
- claude-md-management:revise-claude-md: Update CLAUDE.md with learnings from this session
- commit-commands:clean_gone: Cleans up all git branches marked as [gone] (branches that have been deleted on the remote but still exist locally), including removing associated worktrees.
- commit-commands:commit-push-pr: Commit, push, and open a PR
- commit-commands:commit: Create a git commit
- frontend-design:frontend-design: Guidance for distinctive, intentional visual design when building new UI or reshaping an existing one. Helps with aesthetic direction, typography, and making choices that don't read as templated defaults.
- superpowers:brainstorming: You MUST use this before any creative work - creating features, building components, adding functionality, or modifying behavior. Explores user intent, requirements and design before implementation.
- superpowers:dispatching-parallel-agents: Use when facing 2+ independent tasks that can be worked on without shared state or sequential dependencies
- superpowers:executing-plans: Use when you have a written implementation plan to execute in a separate session with review checkpoints
- superpowers:finishing-a-development-branch: Use when implementation is complete, all tests pass, and you need to decide how to integrate the work
- superpowers:receiving-code-review: Use when receiving code review feedback, before implementing suggestions, especially if feedback seems unclear or technically questionable - requires technical rigor and verification, not performative agreement or blind implementation
- superpowers:requesting-code-review: Use when completing tasks, implementing major features, or before merging to verify work meets requirements
- superpowers:subagent-driven-development: Use when executing implementation plans with independent tasks in the current session
- superpowers:systematic-debugging: Use when encountering any bug, test failure, or unexpected behavior, before proposing fixes
- superpowers:test-driven-development: Use when implementing any feature or bugfix, before writing implementation code
- superpowers:using-git-worktrees: Use when starting feature work that needs isolation from current workspace or before executing implementation plans - ensures an isolated workspace exists via native tools or git worktree fallback
- superpowers:using-superpowers: Use when starting any conversation - establishes how to find and use skills, requiring skill invocation before ANY response including clarifying questions
- superpowers:verification-before-completion: Use when about to claim work is complete, fixed, or passing, before committing or creating PRs - requires running verification commands and confirming output before making any success claims; evidence before assertions always
- superpowers:writing-plans: Use when you have a spec or requirements for a multi-step task, before touching code
- superpowers:writing-skills: Use when creating new skills, editing existing skills, or verifying skills work before deployment
- claude-md-management:claude-md-improver: Audit and improve CLAUDE.md files in repositories. Use when user asks to check, audit, update, improve, or fix CLAUDE.md files. Scans for all CLAUDE.md files, evaluates quality against templates, outputs quality report, then makes targeted updates. Also use when the user mentions \"CLAUDE.md maintenance\" or \"project memory optimization\".
- claude-code-setup:claude-automation-recommender: Analyze a codebase and recommend Claude Code automations (hooks, subagents, skills, plugins, MCP servers). Use when user asks for automation recommendations, wants to optimize their Claude Code setup, mentions improving Claude Code workflows, asks how to first set up Claude Code for a project, or wants to know what Claude Code features they should use.
- dataviz: Use this skill whenever you are about to create ANY chart, graph, plot, dashboard, or data visualization, in ANY output medium — an HTML or React artifact, inline SVG, plotting code in any library (matplotlib, plotly, d3, Recharts, …), an image/PNG you will render and upload, or a chart shared into Slack. Read it BEFORE writing the first line of chart code, choosing chart colors, building a stat tile / meter / KPI row, or laying out a dashboard. Produces visualizations that read as one system — elegant, accessible, consistent in light and dark — using a brand-neutral placeholder palette you swap for your own. Teaches a design-system-agnostic method: a form heuristic, a color formula with a runnable validator, mark specs, and interaction rules. A validated default palette is documented in `references/palette.md` — swap that file's values for your brand's. Triggers on: \"chart\", \"graph\", \"plot\", \"data viz\", \"visualization\", \"dashboard\", \"analytics\", \"visualize data\", \"categorical colors\", \"sequential / diverging palette\", \"stat tile\", \"sparkline\", \"heatmap\", \"legend\", \"axis\", \"tooltip\", \"chart colors\", \"color by series\".
- artifact-design: Design guidance and fundamentals for Artifacts.
- artifact-capabilities: Runtime capabilities a published Artifact page can be granted — behavior static HTML cannot provide on its own, such as the page reading live or connected data, keeping state shared across viewers, or updating and republishing itself. Serves this user's live capability roster and the typed call definitions. Load it whenever the user asks for an artifact needing any such runtime behavior.
- update-config: Use this skill to configure the Claude Code harness via settings.json. Automated behaviors (\"from now on when X\", \"each time X\", \"whenever X\", \"before/after X\") require hooks configured in settings.json - the harness executes these, not Claude, so memory/preferences cannot fulfill them. Also use for: permissions (\"allow X\", \"add permission\", \"move permission to\"), env vars (\"set X=Y\"), hook troubleshooting, or any changes to settings.json/settings.local.json files. Examples: \"allow npm commands\", \"add bq permission to global settings\", \"move permission to user settings\", \"set DEBUG=true\", \"when claude stops show X\". For simple settings like theme/model, suggest the /config command.
- keybindings-help: Use when the user wants to customize keyboard shortcuts, rebind keys, add chord bindings, or modify ~/.claude/keybindings.json. Examples: \"rebind ctrl+s\", \"add a chord shortcut\", \"change the submit key\", \"customize keybindings\".
- simplify: Review the changed code for reuse, simplification, efficiency, and altitude cleanups, then apply the fixes. Quality only — it does not hunt for bugs; use /code-review for that.
- fewer-permission-prompts: Scan your transcripts for common read-only Bash and MCP tool calls, then add a prioritized allowlist to project .claude/settings.json to reduce permission prompts.
- loop: Run a prompt or slash command on a recurring interval (e.g. /loop 5m /foo). Omit the interval to let the model self-pace. - When the user wants to set up a recurring task, poll for status, or run something repeatedly on an interval (e.g. \"check the deploy every 5 minutes\", \"keep running /babysit-prs\"). Do NOT invoke for one-off tasks.
- schedule: Create, update, list, or run scheduled cloud agents (routines) that execute on a cron schedule. - When the user wants to schedule a recurring cloud agent, set up automated tasks, create a cron job for Claude Code, or manage their scheduled agents/routines. Also use when the user wants a one-time scheduled run (\"run this once at 3pm\", \"remind me to check X tomorrow\").
- claude-api: Reference for the Claude API / Anthropic SDK — model ids, pricing, params, streaming, tool use, MCP, agents, caching, token counting, model migration.
TRIGGER — read BEFORE opening the target file; don't skip because it \"looks like a one-liner\" — whenever: the prompt names Claude/Anthropic in any form (Claude, Anthropic, Fable, Opus, Sonnet, Haiku, `anthropic`, `@anthropic-ai`, `claude-*`, `us.anthropic.*`, `[1m]`); the user asks about an LLM (pricing/model choice/limits/caching) — never answer from memory; OR the task is LLM-shaped with provider unstated (agent/MCP/tool-definition/multi-agent/RAG/LLM-judge/computer-use; generate/summarize/extract/classify/rewrite/converse over NL; debugging refusals/cutoffs/streaming/tool-calls/tokens).
SKIP only when another provider is being worked on (overrides all triggers): OpenAI/GPT/Gemini/Llama/Mistral/Cohere/Ollama named in the query; OR `grep -rE 'openai|langchain_openai|google.generativeai|genai|mistralai|cohere|ollama'` over the project hits (run this grep FIRST if no provider named — don't Read the file).
- run: Launch and drive this project's app to see a change working. Use when asked to run, start, or screenshot the app, or to confirm a change works in the real app (not just tests). First looks for a project skill that already covers launching the app; otherwise falls back to built-in patterns per project type (CLI, server, TUI, Electron, browser-driven, library).
- init: Initialize a new CLAUDE.md file with codebase documentation
- review: Review a GitHub pull request; for your working diff use /code-review
- security-review: Complete a security review of the pending changes on the current branch","skillCount":38,"isInitial":true,"names":["cycle-prep","code-review:code-review","feature-dev:feature-dev","claude-md-management:revise-claude-md","commit-commands:clean_gone","commit-commands:commit-push-pr","commit-commands:commit","frontend-design:frontend-design","superpowers:brainstorming","superpowers:dispatching-parallel-agents","superpowers:executing-plans","superpowers:finishing-a-development-branch","superpowers:receiving-code-review","superpowers:requesting-code-review","superpowers:subagent-driven-development","superpowers:systematic-debugging","superpowers:test-driven-development","superpowers:using-git-worktrees","superpowers:using-superpowers","superpowers:verification-before-completion","superpowers:writing-plans","superpowers:writing-skills","claude-md-management:claude-md-improver","claude-code-setup:claude-automation-recommender","dataviz","artifact-design","artifact-capabilities","update-config","keybindings-help","simplify","fewer-permission-prompts","loop","schedule","claude-api","run","init","review","security-review"]},"type":"attachment","uuid":"59737fa0-35b8-4061-a05b-eee7e853b1cf","timestamp":"2026-08-01T04:50:39.602Z","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"59737fa0-35b8-4061-a05b-eee7e853b1cf","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRhtSbudYeYhwM8ggpm","type":"message","role":"assistant","content":[{"type":"text","text":"I'll start by reading the brief, the artifact, and r1's review."}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":18711,"cache_read_input_tokens":9982,"cache_creation":{"ephemeral_5m_input_tokens":18711,"ephemeral_1h_input_tokens":0},"output_tokens":1,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRhps3h5YsTFU2UZagc","attributionAgent":"general-purpose","type":"assistant","uuid":"3237982f-5aaa-4e79-a34e-2cf6faecead0","timestamp":"2026-08-01T04:50:42.628Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"3237982f-5aaa-4e79-a34e-2cf6faecead0","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRhtSbudYeYhwM8ggpm","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01J9TjuPP5fxXpaFdH7zUe3S","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/design/no-testimony/BRIEF-spec-review-r2.md"},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":18711,"cache_read_input_tokens":9982,"cache_creation":{"ephemeral_5m_input_tokens":18711,"ephemeral_1h_input_tokens":0},"output_tokens":1,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRhps3h5YsTFU2UZagc","attributionAgent":"general-purpose","type":"assistant","uuid":"4ee2229e-c5f5-40b1-8fe1-4ab750a08f04","timestamp":"2026-08-01T04:50:43.777Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"4ee2229e-c5f5-40b1-8fe1-4ab750a08f04","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01J9TjuPP5fxXpaFdH7zUe3S","type":"tool_result","content":"1\t# §G-11 SPEC r2 — RE-REVIEW BRIEF
2\t
3\t**Artifact:** `design/no-testimony/SPEC.md` (revision r2, commit `104197d`).
4\t**r1 reviews, already folded:** `reviews/SPEC-doctrine-opus-r1.md` (2C/3I),
5\t`reviews/SPEC-engineering-opus-r1.md` (0C/4I).
6\t
7\t## The ONE question
8\t
9\t**Did the r2 fold actually close its findings — and did it break anything in the process?**
10\t
11\tThis is a re-review, not a fresh audit. r1 was thorough; its ALSO-CHECKED-SOUND sections are settled.
12\t**Do not re-derive what r1 established.** Spend your budget on the deltas and on what the fold could
13\tplausibly have broken.
14\t
15\t## What changed in r2 (the delta you are reviewing)
16\t
17\t| # | r1 finding | r2's fix |
18\t|---|---|---|
19\t| C-1 | 4 productions, ≥25 fields fit none (threshold was 5) | **8 productions**; coverage table promoted from P0 *aspiration* to P0 *deliverable* |
20\t| C-2 | `computed` keyed on the VERB while the prose said FLOOR; 17 fields forced to print a sworn zero | split into `combine` (no floor ⇒ blank iff all operands blank) and `floored` (has the clause ⇒ always entered) |
21\t| I (doctrine) | `total_of`'s `iff` missed the SKIP class (Sch SE 8d) | production 3 takes an optional skip predicate |
22\t| I (doctrine) | Schedule 1 has no extract; `*.txt` grep sweeps instruction files | grep restricted to `f*--YYYY.txt`; P0 must add `f1040s1--2024.txt`; grep produces a *candidate* set, each adjudicated once |
23\t| I (eng) | scope named `printed.rs` only — excluded Form 6251, its own headline example | scope is four files: `printed.rs`, `other_taxes.rs`, `qbi.rs`, `form6251.rs`, `forms.rs` |
24\t| I (eng) | no phase performed the uniform retyping §2.2 mandates (~60 of 168 scheduled) | **new P0b** retypes everything in scope |
25\t| I (eng) | containment was a grep; missed inline `match`, banned the honest accessor where production needs it | **`LineEntry` is now OPAQUE** (newtype over a private enum) + `clippy.toml` + grep as backstop |
26\t| I (eng) | both plants missed a filer-STATED zero being deleted | **third plant + stated-zero fixture** |
27\t| M×several | E0063→E0308, counts, `Printed8949Totals` Default, map enumerator predicate, `golden_packet.rs:321` | folded |
28\t
29\tAlso new in r2: §1 names a **fourth lawful move** (btctax asserting its own arithmetic over lines
30\talready on the page) that r1 left unnamed and which therefore became its residual bucket.
31\t
32\t## Where a fold like this plausibly breaks
33\t
34\t1. **The new productions may be judgment buckets wearing production costumes.** `scaled`, `bounded`,
35\t   `constant`, `conditional` were added under time pressure to absorb r1's misfits. Are they each keyed
36\t   to a real, quotable phrase family — or did I just name the leftovers? **This is the same failure as
37\t   C-1, one level up**, and it is the most likely way r2 is still wrong.
38\t2. **The count may still exceed the threshold.** r1 measured ≈25 misfits against 4 productions.
39\t   **Re-run it against 8.** If it is still ≥5, r2 fails its own criterion exactly as r1 did.
40\t3. **Opacity may cost more than it was priced at.** `LineEntry` must plausibly be `Copy`, `PartialEq`,
41\t   `Eq`, `Debug`, and usable in the ~25 production sites r1 inventoried. Does the published-exit set in
42\t   §2.2/§6 actually cover them, or does something still need a `match` it cannot have?
43\t4. **`combine`'s skip predicate may be under-specified** — where does the predicate come from, and is it
44\t   itself a transcription or a judgment?
45\t5. **P0b's byte-identity claim** (\"byte-unchanged, all entered\") over the *four-file* scope, not just
46\t   `printed.rs`. The crypto-slice `ScheduleDTotals` path never calls `push_money` at all.
47\t
48\t## Settled — do NOT re-derive
49\t
50\tThe survey counts (64/168, seven mechanisms, 24 instructed zeros, layer split). The architecture (two
51\ttypes, emitter-first) — an accepted consult; do not propose a different one. The four owner decisions
52\t(§8). r1's ALSO-CHECKED-SOUND findings, specifically: P0 byte-identity traces cleanly; suppression is
53\tgeometrically safe (`verify.rs` requires strict descent, not contiguity); `extract_lines`' absent-key
54\tpremise holds; `§4`'s escape hunt is otherwise clean; `no_unmapped_filled` half-closes map coverage;
55\t`PresentZero` and `repo_hygiene.rs` exist as precedents. Tax-figure correctness.
56\t
57\t## Output
58\t
59\t`VERDICT: green` or `VERDICT: <n> Critical / <n> Important`
60\t
61\tFindings in the r1 format (`SEVERITY / WHERE / CLAIM / CONSEQUENCE / EVIDENCE`), most severe first.
62\t
63\t**Critical** = implementing r2 as written still leaves fabricated zeros printing, or the gate cannot
64\tdetect them. **Important** = a real gap that would surface during the build.
65\t
66\t**Green is the expected outcome and a short report is a good one.** r2 folded nine findings from two
67\tindependent lenses; if it closed them, say so and stop. Do not manufacture a finding to justify the
68\tround — but do say plainly if a fix is cosmetic rather than real.
69\t
70\tEnd with `WHAT R2 CLOSED:` (so the next round knows) and `WHAT WOULD MAKE THIS REVIEW WRONG:`.
71\t
72\t**Constraints:** READ-ONLY. No edits, no commits, no subagents. You have a shell — read the form text
73\tand the code rather than reasoning from the spec's description of them.
74\t"}]},"uuid":"0376ee18-ea2e-4897-8aaa-8e41c54891b0","timestamp":"2026-08-01T04:50:43.784Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/design/no-testimony/BRIEF-spec-review-r2.md","content":"# §G-11 SPEC r2 — RE-REVIEW BRIEF

**Artifact:** `design/no-testimony/SPEC.md` (revision r2, commit `104197d`).
**r1 reviews, already folded:** `reviews/SPEC-doctrine-opus-r1.md` (2C/3I),
`reviews/SPEC-engineering-opus-r1.md` (0C/4I).

## The ONE question

**Did the r2 fold actually close its findings — and did it break anything in the process?**

This is a re-review, not a fresh audit. r1 was thorough; its ALSO-CHECKED-SOUND sections are settled.
**Do not re-derive what r1 established.** Spend your budget on the deltas and on what the fold could
plausibly have broken.

## What changed in r2 (the delta you are reviewing)

| # | r1 finding | r2's fix |
|---|---|---|
| C-1 | 4 productions, ≥25 fields fit none (threshold was 5) | **8 productions**; coverage table promoted from P0 *aspiration* to P0 *deliverable* |
| C-2 | `computed` keyed on the VERB while the prose said FLOOR; 17 fields forced to print a sworn zero | split into `combine` (no floor ⇒ blank iff all operands blank) and `floored` (has the clause ⇒ always entered) |
| I (doctrine) | `total_of`'s `iff` missed the SKIP class (Sch SE 8d) | production 3 takes an optional skip predicate |
| I (doctrine) | Schedule 1 has no extract; `*.txt` grep sweeps instruction files | grep restricted to `f*--YYYY.txt`; P0 must add `f1040s1--2024.txt`; grep produces a *candidate* set, each adjudicated once |
| I (eng) | scope named `printed.rs` only — excluded Form 6251, its own headline example | scope is four files: `printed.rs`, `other_taxes.rs`, `qbi.rs`, `form6251.rs`, `forms.rs` |
| I (eng) | no phase performed the uniform retyping §2.2 mandates (~60 of 168 scheduled) | **new P0b** retypes everything in scope |
| I (eng) | containment was a grep; missed inline `match`, banned the honest accessor where production needs it | **`LineEntry` is now OPAQUE** (newtype over a private enum) + `clippy.toml` + grep as backstop |
| I (eng) | both plants missed a filer-STATED zero being deleted | **third plant + stated-zero fixture** |
| M×several | E0063→E0308, counts, `Printed8949Totals` Default, map enumerator predicate, `golden_packet.rs:321` | folded |

Also new in r2: §1 names a **fourth lawful move** (btctax asserting its own arithmetic over lines
already on the page) that r1 left unnamed and which therefore became its residual bucket.

## Where a fold like this plausibly breaks

1. **The new productions may be judgment buckets wearing production costumes.** `scaled`, `bounded`,
   `constant`, `conditional` were added under time pressure to absorb r1's misfits. Are they each keyed
   to a real, quotable phrase family — or did I just name the leftovers? **This is the same failure as
   C-1, one level up**, and it is the most likely way r2 is still wrong.
2. **The count may still exceed the threshold.** r1 measured ≈25 misfits against 4 productions.
   **Re-run it against 8.** If it is still ≥5, r2 fails its own criterion exactly as r1 did.
3. **Opacity may cost more than it was priced at.** `LineEntry` must plausibly be `Copy`, `PartialEq`,
   `Eq`, `Debug`, and usable in the ~25 production sites r1 inventoried. Does the published-exit set in
   §2.2/§6 actually cover them, or does something still need a `match` it cannot have?
4. **`combine`'s skip predicate may be under-specified** — where does the predicate come from, and is it
   itself a transcription or a judgment?
5. **P0b's byte-identity claim** (\"byte-unchanged, all entered\") over the *four-file* scope, not just
   `printed.rs`. The crypto-slice `ScheduleDTotals` path never calls `push_money` at all.

## Settled — do NOT re-derive

The survey counts (64/168, seven mechanisms, 24 instructed zeros, layer split). The architecture (two
types, emitter-first) — an accepted consult; do not propose a different one. The four owner decisions
(§8). r1's ALSO-CHECKED-SOUND findings, specifically: P0 byte-identity traces cleanly; suppression is
geometrically safe (`verify.rs` requires strict descent, not contiguity); `extract_lines`' absent-key
premise holds; `§4`'s escape hunt is otherwise clean; `no_unmapped_filled` half-closes map coverage;
`PresentZero` and `repo_hygiene.rs` exist as precedents. Tax-figure correctness.

## Output

`VERDICT: green` or `VERDICT: <n> Critical / <n> Important`

Findings in the r1 format (`SEVERITY / WHERE / CLAIM / CONSEQUENCE / EVIDENCE`), most severe first.

**Critical** = implementing r2 as written still leaves fabricated zeros printing, or the gate cannot
detect them. **Important** = a real gap that would surface during the build.

**Green is the expected outcome and a short report is a good one.** r2 folded nine findings from two
independent lenses; if it closed them, say so and stop. Do not manufacture a finding to justify the
round — but do say plainly if a fix is cosmetic rather than real.

End with `WHAT R2 CLOSED:` (so the next round knows) and `WHAT WOULD MAKE THIS REVIEW WRONG:`.

**Constraints:** READ-ONLY. No edits, no commits, no subagents. You have a shell — read the form text
and the code rather than reasoning from the spec's description of them.
","numLines":74,"startLine":1,"totalLines":74}},"sourceToolAssistantUUID":"4ee2229e-c5f5-40b1-8fe1-4ab750a08f04","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"0376ee18-ea2e-4897-8aaa-8e41c54891b0","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRhtSbudYeYhwM8ggpm","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01QL465VRvoLZqeeLRZ2WhZB","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/design/no-testimony/reviews/SPEC-engineering-opus-r1.md"},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":18711,"cache_read_input_tokens":9982,"cache_creation":{"ephemeral_5m_input_tokens":18711,"ephemeral_1h_input_tokens":0},"output_tokens":1,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRhps3h5YsTFU2UZagc","attributionAgent":"general-purpose","type":"assistant","uuid":"16f19770-fffc-4ae3-979e-c35b623b6359","timestamp":"2026-08-01T04:50:44.188Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"16f19770-fffc-4ae3-979e-c35b623b6359","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"type":"tool_result","content":"File content (604.4KB) exceeds maximum allowed size (256KB). Use offset and limit parameters to read specific portions of the file, or search for specific content instead of reading the whole file.","is_error":true,"tool_use_id":"toolu_01QL465VRvoLZqeeLRZ2WhZB"}]},"uuid":"6b0eb3e6-ad3e-4e04-a22f-91ce39550036","timestamp":"2026-08-01T04:50:44.191Z","toolUseResult":"Error: File content (604.4KB) exceeds maximum allowed size (256KB). Use offset and limit parameters to read specific portions of the file, or search for specific content instead of reading the whole file.","sourceToolAssistantUUID":"16f19770-fffc-4ae3-979e-c35b623b6359","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"6b0eb3e6-ad3e-4e04-a22f-91ce39550036","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRiCCSWUPS5ooWfRk4k","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISswIKhwEIEBgCKkBwcZxQEXZ6buorhGJBURhSM1Pq0SsdLMkde1nq4vVU2SZKKuhLssW11+assi+Kq1ukNVbXDe+Y72GlNrwKNt1ZMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDAEDqil1J6epnR477RoMNjWgC90S7ZjGFAwZIjD/aKiFDa4joM3qOzAp67Oy0fclb2UufvtCw7d4cE8h14jk7GQ+uJe5WkFFRLZmUnAqWXcmgq+m+wpH3urg1WyjYiBwqe4buuu7PKppsobVhcYanqtRGY/dtQfHr1JofjA/hsyjBjUEAr4t9cjaXTgg+FejK7l0Vj6wXZoqAbV3iJyHIcuCb6ijhbwhGAE="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":4216,"cache_read_input_tokens":28693,"cache_creation":{"ephemeral_5m_input_tokens":4216,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRiB4UvESk3Up6QcXqe","attributionAgent":"general-purpose","type":"assistant","uuid":"f193c9b1-6ccc-45fd-9e80-d2119e9c5c0c","timestamp":"2026-08-01T04:50:45.924Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"f193c9b1-6ccc-45fd-9e80-d2119e9c5c0c","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRiCCSWUPS5ooWfRk4k","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_011rbSDH4W1qACZHu68wAnvk","name":"Bash","input":{"command":"ls -la /scratch/code/bitcoin_tax/design/no-testimony/ /scratch/code/bitcoin_tax/design/no-testimony/reviews/ 2>&1; wc -l /scratch/code/bitcoin_tax/design/no-testimony/SPEC.md /scratch/code/bitcoin_tax/design/no-testimony/reviews/*.md"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":4216,"cache_read_input_tokens":28693,"output_tokens":206,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":4216},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":206,"cache_read_input_tokens":28693,"cache_creation_input_tokens":4216,"cache_creation":{"ephemeral_5m_input_tokens":4216,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRiB4UvESk3Up6QcXqe","attributionAgent":"general-purpose","type":"assistant","uuid":"1de2684e-1b92-4fef-b88c-1080afd059c5","timestamp":"2026-08-01T04:50:47.410Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"1de2684e-1b92-4fef-b88c-1080afd059c5","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_011rbSDH4W1qACZHu68wAnvk","type":"tool_result","content":"/scratch/code/bitcoin_tax/design/no-testimony/:
total 580
drwxr-xr-x 1 bcg bcg    284 Jul 31 21:50 .
drwxr-xr-x 1 bcg bcg   5646 Jul 31 18:46 ..
-rw-r--r-- 1 bcg bcg  11107 Jul 31 21:12 BRAINSTORM.md
-rw-r--r-- 1 bcg bcg   6719 Jul 31 21:04 BRIEF-architect.md
-rw-r--r-- 1 bcg bcg   4898 Jul 31 21:21 BRIEF-spec-review.md
-rw-r--r-- 1 bcg bcg   5115 Jul 31 21:50 BRIEF-spec-review-r2.md
-rw-r--r-- 1 bcg bcg   4624 Jul 31 18:46 BRIEF-survey.md
-rw-r--r-- 1 bcg bcg 490117 Jul 31 21:12 CONSULT-architect-fable.md
-rw-r--r-- 1 bcg bcg  36666 Jul 31 19:05 MAP-survey.md
drwxr-xr-x 1 bcg bcg    102 Jul 31 21:33 reviews
-rw-r--r-- 1 bcg bcg  17102 Jul 31 21:35 SPEC.md

/scratch/code/bitcoin_tax/design/no-testimony/reviews/:
total 1140
drwxr-xr-x 1 bcg bcg    102 Jul 31 21:33 .
drwxr-xr-x 1 bcg bcg    284 Jul 31 21:50 ..
-rw-r--r-- 1 bcg bcg 540743 Jul 31 21:32 SPEC-doctrine-opus-r1.md
-rw-r--r-- 1 bcg bcg 618867 Jul 31 21:33 SPEC-engineering-opus-r1.md
    261 /scratch/code/bitcoin_tax/design/no-testimony/SPEC.md
   4680 /scratch/code/bitcoin_tax/design/no-testimony/reviews/SPEC-doctrine-opus-r1.md
   5755 /scratch/code/bitcoin_tax/design/no-testimony/reviews/SPEC-engineering-opus-r1.md
  10696 total","is_error":false}]},"uuid":"86ec21cd-e9bf-4a87-8fbd-9b7394e99aa9","timestamp":"2026-08-01T04:50:47.495Z","toolUseResult":{"stdout":"/scratch/code/bitcoin_tax/design/no-testimony/:
total 580
drwxr-xr-x 1 bcg bcg    284 Jul 31 21:50 .
drwxr-xr-x 1 bcg bcg   5646 Jul 31 18:46 ..
-rw-r--r-- 1 bcg bcg  11107 Jul 31 21:12 BRAINSTORM.md
-rw-r--r-- 1 bcg bcg   6719 Jul 31 21:04 BRIEF-architect.md
-rw-r--r-- 1 bcg bcg   4898 Jul 31 21:21 BRIEF-spec-review.md
-rw-r--r-- 1 bcg bcg   5115 Jul 31 21:50 BRIEF-spec-review-r2.md
-rw-r--r-- 1 bcg bcg   4624 Jul 31 18:46 BRIEF-survey.md
-rw-r--r-- 1 bcg bcg 490117 Jul 31 21:12 CONSULT-architect-fable.md
-rw-r--r-- 1 bcg bcg  36666 Jul 31 19:05 MAP-survey.md
drwxr-xr-x 1 bcg bcg    102 Jul 31 21:33 reviews
-rw-r--r-- 1 bcg bcg  17102 Jul 31 21:35 SPEC.md

/scratch/code/bitcoin_tax/design/no-testimony/reviews/:
total 1140
drwxr-xr-x 1 bcg bcg    102 Jul 31 21:33 .
drwxr-xr-x 1 bcg bcg    284 Jul 31 21:50 ..
-rw-r--r-- 1 bcg bcg 540743 Jul 31 21:32 SPEC-doctrine-opus-r1.md
-rw-r--r-- 1 bcg bcg 618867 Jul 31 21:33 SPEC-engineering-opus-r1.md
    261 /scratch/code/bitcoin_tax/design/no-testimony/SPEC.md
   4680 /scratch/code/bitcoin_tax/design/no-testimony/reviews/SPEC-doctrine-opus-r1.md
   5755 /scratch/code/bitcoin_tax/design/no-testimony/reviews/SPEC-engineering-opus-r1.md
  10696 total","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"1de2684e-1b92-4fef-b88c-1080afd059c5","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"86ec21cd-e9bf-4a87-8fbd-9b7394e99aa9","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRiSMEzuk5zCxEstGgo","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISiwMKhwEIEBgCKkAsR4bAQZ9rPKv4JSn7MWAo+5cSMBV+6e8m8CPy4KUIG33iykHnn6DszyDx2CjFUo5Sr56mRnL+uaTAe83WR9nLMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDIv1UxDZijyT/PauzRoMCRVNfDX1hYRlOPiHIjC0DCVzLvrNqP9yFyHpp/hbBJ5eb14HdbPul5JK3FwTFPkW6X2rarbEcZVEyxx95lwqsAEQ2kt0/TWC6g+OzVC5TzygxGD8iGsnNPQB2iChxYOOhuAVVeDOQ2UqIhXYgblekX6ptinWqrKysYhPimmfXmer8jfx1MThHgmJFH4oBxxbmXfDH1PTmVwtLtffdaaZkXkTwilD/ZguQdGzAjwr8Ao1lPgwxv+ZGfs4rdLpB8eQwuzckyQp58xBjw8n0m16K7TVqLZTuso3Ihd0ouJfOF3mikLjZAevA2UQ6LnQBEaTURgB"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1031,"cache_read_input_tokens":32909,"cache_creation":{"ephemeral_5m_input_tokens":1031,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRiQpiBRh58uNBsfv9w","attributionAgent":"general-purpose","type":"assistant","uuid":"70901292-c71c-4ffa-96bc-7b38568b2961","timestamp":"2026-08-01T04:50:49.502Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"70901292-c71c-4ffa-96bc-7b38568b2961","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRiSMEzuk5zCxEstGgo","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_016wYmCTh1jqQiBs4R3yWwwa","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/design/no-testimony/SPEC.md"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1031,"cache_read_input_tokens":32909,"output_tokens":134,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":1031},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":134,"cache_read_input_tokens":32909,"cache_creation_input_tokens":1031,"cache_creation":{"ephemeral_5m_input_tokens":1031,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRiQpiBRh58uNBsfv9w","attributionAgent":"general-purpose","type":"assistant","uuid":"786cbda2-5334-400b-9e0c-064d28f0cae2","timestamp":"2026-08-01T04:50:49.838Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"786cbda2-5334-400b-9e0c-064d28f0cae2","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_016wYmCTh1jqQiBs4R3yWwwa","type":"tool_result","content":"1\t# SPEC — §G-11: make \"no testimony\" representable
2\t
3\t**Revision r2** (2026-07-31), folding `reviews/SPEC-doctrine-opus-r1.md` (2C/3I) and
4\t`reviews/SPEC-engineering-opus-r1.md` (0C/4I). **Status: pending re-review.**
5\t**Sources:** `MAP-survey.md` (evidence), `CONSULT-architect-fable.md` (architecture, accepted).
6\t
7\t> **r1 → r2 in one line:** the grammar was undercooked (4 productions for ≥7 instruction classes) and
8\t> keyed on the wrong discriminator (the *verb*, not the *floor*); the scope named one file when four
9\t> reach paper; the containment was a grep when the compiler can do it; and the gate never tested the
10\t> distinction the type exists to draw. All four are fixed below.
11\t
12\t---
13\t
14\t## 1. The problem, and the scope boundary
15\t
16\tA blank line and a printed `0` are different speech acts on a return signed under 26 USC §6065. A blank
17\tsays nothing; a `0` swears the amount **is** zero. btctax has no representation for \"not stated\" in its
18\tmoney path, so **64 of the 168 money quantities that reach a PDF can print a zero the filer was never
19\tasked for.**
20\t
21\tbtctax has three lawful moves over the *filer's* facts: **collect**, **refuse**, or leave **genuinely
22\tblank**. It must never silently choose silence and present it as the filer's.
23\t
24\t★ **A fourth move exists and r1 failed to name it** (doctrine M-1). btctax also **asserts its own
25\tarithmetic** over lines already on the page — *\"Subtract line 14 from line 11\"*. That is not the filer's
26\ttestimony being invented; it is btctax showing its work, and the form demands it. Naming it makes the
27\tadmission rule explicit: **a line may assert btctax's arithmetic only when every operand it reads is
28\titself on the page.** Unnamed, it became r1's residual bucket, which is how Critical #2 happened.
29\t
30\t**★ HARD SCOPE BOUNDARY (§G-11, non-negotiable).** Nothing here may flag an omission as suspicious,
31\tscore it, or opine on whether a blank is lawful. Both directions are software adjudicating intent.
32\tThe architecture preserves *who said what*; it never evaluates it. *(Both r1 lenses checked this and
33\tfound no crossing; keep it that way.)*
34\t
35\t**Non-goals.** Tax-figure correctness. Supplied-then-zeroed lines (§170(b) ceilings, Schedule A 8a).
36\tUnasked *elections* as distinct from unasked *amounts*.
37\t
38\t## 2. The two types
39\t
40\t### 2.1 `Collected<T>` — the input boundary (`return_inputs.rs`)
41\t
42\t```rust
43\tpub enum Collected<T> { Stated(T), NotAsked }
44\t```
45\t
46\t`#[serde(default)]` ⇒ `NotAsked`. **No `Default` producing a value; no `unwrap_or*`.** Sole exit:
47\t`fn stated(&self) -> Option<T>`.
48\t
49\t**Why not bare `Option<Usd>`** — the repo already refuted it at `return_inputs.rs:631`: *\"`Option<Usd>`
50\tis a scalar the `_` rule permits, which would make this convention again.\"* Plus `unwrap_or(Usd::ZERO)`
51\tis a one-token re-fabrication on every `Option`.
52\t
53\t★★ **`Stated(Usd::ZERO)` is a first-class, load-bearing state.** A filer who says \"my withholding was
54\tzero\" has testified. Collapsing it into `Blank` deletes testimony — the mirror of the defect. §5.1's
55\tthird plant exists solely to make that failure red.
56\t
57\t### 2.2 `LineEntry` — the printed layer, **OPAQUE**
58\t
59\t```rust
60\tpub struct LineEntry(Repr);          // public
61\tenum Repr { Entered(Usd), Blank }    // PRIVATE to the module
62\t```
63\t
64\t★★★ **Opaque, not a public enum** (engineering I-3). A `match` on `LineEntry` outside the defining
65\tmodule **does not compile** — which is the only mechanism that closes the hole a grep cannot: an inline
66\t`match e { Blank => Usd::ZERO, .. }` in production code. It costs nothing, because §3 already forbids
67\tfree construction. Published exits are chosen deliberately: `is_blank()`, `fmt()` for the emitter, and
68\tan ordering comparator for the three emitter sites that compare (`schedule_se_full.rs:78`,
69\t`form8995.rs:166`, `schedule_d_full.rs:61`).
70\t
71\t**No `From<Usd>`, no `Add`, no `Sum`, no `Default`.** ★ **And `#[non_exhaustive]` is explicitly
72\trejected** — it forces a `_` arm on downstream matches, making `_ => Usd::ZERO` *easier*. It would make
73\tthis worse.
74\t
75\t**Scope — four files, not one** (engineering I-1). Every money field on every struct that reaches paper:
76\t`printed.rs` (16 structs), `other_taxes.rs` (`Form8959Lines`, `Form8960Lines`), `qbi.rs`
77\t(`Form8995Lines`), `form6251.rs` (`Form6251`), and `forms.rs` (`ScheduleDPart`, `ScheduleDTotals` — the
78\tcrypto slice, which never calls `push_money` at all). **r1 scoped `printed.rs` alone, which silently
79\texcluded Form 6251 — its own headline example.**
80\t
81\t★ `Printed8949Totals` (`printed.rs:53`) carries the layer's only `#[derive(Default)]`. **Delete the
82\tderive**; do not add `impl Default for LineEntry`. Even a `Blank`-yielding `Default` reopens the
83\timplicit-construction door.
84\t
85\t## 3. The combinator grammar — keyed to the FLOOR, not the verb
86\t
87\t`LineEntry` is constructible **only** through these. r1 had four productions; the doctrine lens found
88\t≥25 fields fitting none, against r1's own stated threshold of five.
89\t
90\t| # | constructor | the form says | blank when |
91\t|---|---|---|---|
92\t| 1 | `collected(Collected<Usd>)` | *(the filer supplies it)* | `NotAsked` — **never** for `Stated(0)` |
93\t| 2 | `carry(LineEntry)` | *\"Enter the amount from line N\"* | the source is blank |
94\t| 3 | `combine(&[LineEntry])` | *\"Add / Combine lines X–Y\"*, **no floor clause** | **iff every operand is blank** |
95\t| 4 | `floored(Usd)` | any arithmetic **+ *\"if zero or less, enter -0-\"*** | **never** |
96\t| 5 | `scaled(LineEntry, Rate)` | *\"Multiply line N by 7.5% (0.075)\"* | the source is blank |
97\t| 6 | `bounded(Bound, &[LineEntry])` | *\"Enter the smaller/larger of …\"* | all inputs blank |
98\t| 7 | `constant(Usd)` | *\"Threshold based on filing status\"* | never |
99\t| 8 | `conditional(Cond, LineEntry)` | *\"If line 33 is more than line 24, …\"* | the condition is unmet |
100\t
101\t★★★ **Production 4's trigger is the FLOOR CLAUSE, not the verb.** r1's table keyed `computed` on
102\t*\"Subtract…\"/\"Combine…\"* while its own next paragraph said the discriminator was the floor. Because the
103\ttable is what an implementer transcribes from, **17 fields that say \"Combine\" with no `-0-` clause
104\twould have been forced to print a sworn zero** — including Schedule 1 line 10 (*the very line §5.1's
105\tfirst plant simulates*) and Schedule D line 7, on the one form btctax always emits. Those are now
106\tproduction 3, and blank when every operand is blank.
107\t
108\t★★ **Production 3's `iff` has a second exit: SKIP** (doctrine I-1). Schedule SE line 8d is *\"Add lines
109\t8a, 8b, and 8c\"* but sits inside *\"If $168,600 or more, skip lines 8b through 10\"* — the form wants 8d
110\t**blank while 8a is entered**. `combine` alone gives `Entered`. So production 3 takes an optional
111\tskip predicate, and the same machinery covers Schedule D's *\"skip lines 17 through 20\"* and Schedule SE
112\t4c's *\"if less than $400, stop\"*. `schedule_se_full.rs:80-92` already ships a per-line include flag —
113\textend it, do not invent beside it.
114\t
115\t★ **Production 3's trigger is a phrase family, not one phrase** (doctrine M-2): *\"Add the amounts in the
116\tfar right column for lines 4 through 16\"* (Schedule A 17) lists a **range whose operands are 4, 7, 10,
117\t14, 15, 16 only** — transcribing the range literally double-counts 5a–5d. Operands are transcribed from
118\tthe form's *column*, never inferred from the range.
119\t
120\t**P0 exit criterion — the coverage check is DONE, not deferred.** Every money field on every struct in
121\t§2.2's scope is assigned a production, in a committed table, with the instruction quoted verbatim.
122\t**r1 deferred this to P0 and shipped a grammar that failed it by 5–8×.** ★ If any field fits none, that
123\tis a finding requiring a documented exception; **five or more and the grammar returns to review.**
124\t
125\t## 4. What the compiler enforces, and what it cannot
126\t
127\t**Compiler:** retyping the fields emits **E0308** at every struct literal — r1 cited E0063, which is the
128\t*adding-a-field* mechanism and is not what happens here (engineering M-1). Measured: **87 struct-literal
129\tsites** across the workspace (46 in `printed.rs`, the rest in tests) and **13 constructor `fn`s**, not
130\t16. Money-write sites: **44** (31 `push_money`, 3 `push_literal`, 10 direct `fmt_money`).
131\t
132\t★ **Verified clean** (engineering): no `serde` derive on any printed struct, no `impl From`, no builder,
133\tno struct-update syntax. Retyping really does error every literal — once §2.2's `Default` derive is gone.
134\t
135\t**Compiler cannot:** verify the *chosen production matches the instruction*. That residue is the gate's.
136\t
137\t## 5. The gate — the disposition KAT
138\t
139\t**Every change here is a suppression, and a suppression that over-fires is invisible.** Neither oracle
140\tsees it; no golden moves.
141\t
142\t`transcribe::extract_lines(pdf, map_toml) -> BTreeMap<String, String>` inserts a cell only when the field
143\texists **and** its text is non-empty — so **an absent key is a blank cell on paper.** Verified, and
144\talready asserted by `tests/extract_lines.rs`.
145\t
146\tNew KAT in `crates/btctax-forms/tests/`. **It must never use `Blank::AbsentIsZero`.**
147\t
148\t1. **Line set from the committed map TOMLs**, via the predicate `walk` already uses — *keep a string
149\t   leaf only if it is a real field in `collect_fields`*. This drops `[census]`/`[identity]`/`form`/`year`
150\t   metadata with **no skip-list** (engineering M-3; the naive enumerator needs one and would rot).
151\t   ★ Note: **one line is not one key** — `MoneyPair` lines transcribe as `.dollars_field`/`.cents_field`,
152\t   Schedule D rows as `.proceeds_d`/`.cost_e`/`.adj_g`/`.gain_h`.
153\t2. **The instructed-zero set is COMPUTED**, by grepping **the form files only** —
154\t   `design/forms/extract/f*--YYYY.txt`. ★ **Not `*.txt`** (doctrine I-3): that sweeps the ~1.3 MB
155\t   `i*` instruction files, where `-0-` appears in conditional prose — `i1040gi:23500` would enroll 1040
156\t   35a/36 on a branch almost no return takes. Verified: the restricted glob returns non-zero on exactly
157\t   the forms that should have it and zero on those that should not.
158\t   ★★ **Schedule 1 has no extract at all**, so it is invisible to both this and §3's check while btctax
159\t   emits it. **P0 must add `f1040s1--2024.txt`** — otherwise a future Schedule 1 `-0-` can never join,
160\t   which is the stale-list failure this design exists to avoid.
161\t   ★ Membership is *not* decided by the grep alone where the clause is itself conditional (Schedule SE
162\t   4c, 5b). The grep produces the **candidate** set; each candidate is adjudicated once, against the
163\t   instructions, and recorded with its reason.
164\t3. **Three fixtures, one instrument.** Expectations are written **from each fixture's INPUTS**, never
165\t   from the printed struct — that independence defeats consistent-but-wrong.
166\t
167\t### 5.1 B1 — three plants, all watched red before P0 merges
168\t
169\t| # | plant | simulates | must red |
170\t|---|---|---|---|
171\t| 1 | reintroduce `sch_1.map_or(Usd::ZERO, …)` (`printed.rs:591`) | a zero that should not print | **absence** vector |
172\t| 2 | make the writer skip every zero value | over-firing suppression | **instructed-zero** vector |
173\t| 3 | `collected` as `match x { Some(v) if !v.is_zero() => Entered(v), _ => Blank }` | **a filer-STATED zero deleted** | **stated-zero** vector |
174\t
175\t★★★ **Plant 3 is new and r1 had no defence against it** (engineering I-4). It compiles, is a plausible
176\tthing to write, silently deletes genuine testimony — and **passed both r1 plants**, because the absence
177\tvector states nothing and the instructed-zero vector's zeros arrive via production 4, not `collected`.
178\t§2.1's entire case for `Collected<T>` is that `Stated(0)` ≠ `NotAsked`, and r1's gate never tested it.
179\tThe third fixture states `$0` withholding / `$0` estimated payments and asserts `Blank::PresentZero`
180\t(`tests/common/mod.rs:201`, which already exists and panics on an absent key) at 1040 L25a/L26.
181\t
182\t**Stated limits.** The KAT pins fixtures, not all `-0-` branches; a completeness census is P4. An author
183\twho writes both a fabrication and an expectation blessing it passes — the gate forces it to be written
184\tdown **twice, in review-visible places**, the same strength the classifier has.
185\t
186\t★ **Audit the one existing `AbsentIsZero` use** (engineering M-4): `golden_packet.rs:321` reads 1040 L13
187\tthat way — the QBI line, which P1 flips to absent. Left alone it stays green through P1/P2, silencing
188\tthe one golden that could have spoken.
189\t
190\t## 6. Containment — the compiler, then the lint, then the grep
191\t
192\tr1 proposed a test-support trait plus a grep. **Insufficient**: the grep bans the honest accessor
193\texactly where production needs it while missing the inline `match`, pushing authors toward the
194\tun-greppable form. And it scoped \"the ten published crates\", exempting `btctax-oracle-harness`
195\t(`publish = false`) — the one crate r1 said needed the accessor.
196\t
197\t**Corrected, in strength order:**
198\t
199\t1. **Opacity (§2.2) is the primary mechanism.** Outside the module there is nothing to `match`.
200\t2. **`clippy.toml` `disallowed-methods`** on whatever numeric exit is published. `make check` already
201\t   runs `clippy --workspace --all-targets -- -D warnings`, so this is a hard build failure overridable
202\t   only by a visible `#[allow]`. The repo has no `clippy.toml` today; add one.
203\t3. **The hygiene grep** as a backstop only, ships with its own planted-defect kill.
204\t
205\t★ **The real consumer inventory** (engineering I-3), which r1 got wrong: `btctax-cli/src/render.rs`
206\tdoes ~23 printed-field reads **in a published crate**; three emitter sites compare; the oracle harness
207\tdoes arithmetic at `main.rs:324`. These need a **published, deliberate** numeric exit — not a
208\ttest-support back door. ★ And a `test-support` feature **cannot** serve the harness: it depends on core
209\tnormally, so feature unification would re-expose the accessor everywhere. The harness writes explicit
210\tmatches inside the defining crate, or uses the published exit.
211\t
212\t## 7. Phasing
213\t
214\t**P0 — vocabulary, scope, gate. ZERO BEHAVIOUR CHANGE ON PAPER.**
215\tDefine the types (opaque). Migrate the writer signature; **every call site passes an entered value**.
216\tFix the two Schedule D emitter literals. Delete the `Printed8949Totals` `Default` derive. Add
217\t`f1040s1--2024.txt`. **Complete the §3 coverage table over all four scoped files.** Build the KAT with
218\tevery current disposition pinned as entered, **all three plants watched red**, `clippy.toml`, and the
219\thygiene gate with its kill. Audit `golden_packet.rs:321`.
220\t**Green =** suite green, goldens byte-unchanged (md5 `c4e1853…`), three kills demonstrated.
221\t★ Traced sound by the engineering lens: the writer is a pure formatter, so byte-identity holds; and
222\tsuppression is **geometrically safe** because `verify.rs:390-414` requires strict descent, not
223\tcontiguity.
224\t
225\t**P0b — the uniform retyping** (engineering I-2). Retype **every** money field in §2.2's scope to
226\t`LineEntry`, all constructed via §3. r1's phases named ~60 of 168 and never scheduled the rest, leaving
227\tthe mixed patchwork §2.2 says produced the 64 — and making §4's guarantee false for every field still
228\t`Usd`. Byte-unchanged on paper (all entered), so it meets the same bar as P0.
229\t
230\t**P1 — stop discarding what we know.** The 15 flattened `Option`s + 8995 lines 3/7 (delete the
231\t`ptr::eq` gate — its *\"only line on this form\"* comment is falsified by line 7 one iteration later).
232\tFlip those rows to absent. No owner decision needed.
233\t
234\t**P2 — manufactured inputs and the silent forms.** `Collected<Usd>` over the 14 `#[serde(default)] Usd`
235\tscalars; the 20 empty-`Vec` sums and 9 blank-column totals. Schedules 1/2/3/C first. 1040 25a/26 wait on
236\towner decision 2.
237\t★ This is where an existing TOML omitting a key changes meaning from `0` to `NotAsked`. btctax has no
238\tusers and v0.15.0 is unpublished — **free now, expensive later.**
239\t
240\t**P3 — make the 65th site fail to compile.** Extend the classifier's `_`-ban to money leaves.
241\t
242\t**P4 — residue.** `\"0\"` vs `\"-0-\"`; the Schedule SE 8d/9/10 mirror defect (adjudicate against
243\t`i1040sse`); the `-0-` completeness census; supplied-then-zeroed and unasked-elections as separate
244\tclasses.
245\t
246\t## 8. Owner decisions as variation points
247\t
248\tUnchanged by r2 — the architecture still does not depend on them. Each is one production choice plus one
249\texpectation row: (1) is a reconciled ledger the filer's testimony? (2) blank or refuse where silence
250\tasserts? (3) is supplied-then-zeroed in scope? (4) sequencing vs Form 6251 Tier 2.
251\t
252\t## 9. Risks carried
253\t
254\t- **The §3 coverage table is now a P0 deliverable, not a P0 aspiration.** It is the thing r1 got wrong.
255\t- **The three-production→eight-production expansion may still be incomplete.** The doctrine lens counted
256\t  ≈25 misfits against four productions; §3 now has eight, but the re-review must re-run that count.
257\t- **Map coverage** is **already half-closed** by `verify.rs:229 no_unmapped_filled` — a cell written
258\t  outside any map fails closed as `UnmappedField` (engineering M-5). Build the census on that invariant
259\t  rather than beside it; a second writes-set would be a second truth.
260\t- **1040 line 16** (*\"Tax (see instructions)\"*) has no verb at all — it comes from the Tax Table or the
261\t  QDCGT worksheet. It is the strongest candidate for §3's documented exception.
262\t"}]},"uuid":"09683569-b5b9-441b-9cb8-09fd89fd05d0","timestamp":"2026-08-01T04:50:49.854Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/design/no-testimony/SPEC.md","content":"# SPEC — §G-11: make \"no testimony\" representable

**Revision r2** (2026-07-31), folding `reviews/SPEC-doctrine-opus-r1.md` (2C/3I) and
`reviews/SPEC-engineering-opus-r1.md` (0C/4I). **Status: pending re-review.**
**Sources:** `MAP-survey.md` (evidence), `CONSULT-architect-fable.md` (architecture, accepted).

> **r1 → r2 in one line:** the grammar was undercooked (4 productions for ≥7 instruction classes) and
> keyed on the wrong discriminator (the *verb*, not the *floor*); the scope named one file when four
> reach paper; the containment was a grep when the compiler can do it; and the gate never tested the
> distinction the type exists to draw. All four are fixed below.

---

## 1. The problem, and the scope boundary

A blank line and a printed `0` are different speech acts on a return signed under 26 USC §6065. A blank
says nothing; a `0` swears the amount **is** zero. btctax has no representation for \"not stated\" in its
money path, so **64 of the 168 money quantities that reach a PDF can print a zero the filer was never
asked for.**

btctax has three lawful moves over the *filer's* facts: **collect**, **refuse**, or leave **genuinely
blank**. It must never silently choose silence and present it as the filer's.

★ **A fourth move exists and r1 failed to name it** (doctrine M-1). btctax also **asserts its own
arithmetic** over lines already on the page — *\"Subtract line 14 from line 11\"*. That is not the filer's
testimony being invented; it is btctax showing its work, and the form demands it. Naming it makes the
admission rule explicit: **a line may assert btctax's arithmetic only when every operand it reads is
itself on the page.** Unnamed, it became r1's residual bucket, which is how Critical #2 happened.

**★ HARD SCOPE BOUNDARY (§G-11, non-negotiable).** Nothing here may flag an omission as suspicious,
score it, or opine on whether a blank is lawful. Both directions are software adjudicating intent.
The architecture preserves *who said what*; it never evaluates it. *(Both r1 lenses checked this and
found no crossing; keep it that way.)*

**Non-goals.** Tax-figure correctness. Supplied-then-zeroed lines (§170(b) ceilings, Schedule A 8a).
Unasked *elections* as distinct from unasked *amounts*.

## 2. The two types

### 2.1 `Collected<T>` — the input boundary (`return_inputs.rs`)

```rust
pub enum Collected<T> { Stated(T), NotAsked }
```

`#[serde(default)]` ⇒ `NotAsked`. **No `Default` producing a value; no `unwrap_or*`.** Sole exit:
`fn stated(&self) -> Option<T>`.

**Why not bare `Option<Usd>`** — the repo already refuted it at `return_inputs.rs:631`: *\"`Option<Usd>`
is a scalar the `_` rule permits, which would make this convention again.\"* Plus `unwrap_or(Usd::ZERO)`
is a one-token re-fabrication on every `Option`.

★★ **`Stated(Usd::ZERO)` is a first-class, load-bearing state.** A filer who says \"my withholding was
zero\" has testified. Collapsing it into `Blank` deletes testimony — the mirror of the defect. §5.1's
third plant exists solely to make that failure red.

### 2.2 `LineEntry` — the printed layer, **OPAQUE**

```rust
pub struct LineEntry(Repr);          // public
enum Repr { Entered(Usd), Blank }    // PRIVATE to the module
```

★★★ **Opaque, not a public enum** (engineering I-3). A `match` on `LineEntry` outside the defining
module **does not compile** — which is the only mechanism that closes the hole a grep cannot: an inline
`match e { Blank => Usd::ZERO, .. }` in production code. It costs nothing, because §3 already forbids
free construction. Published exits are chosen deliberately: `is_blank()`, `fmt()` for the emitter, and
an ordering comparator for the three emitter sites that compare (`schedule_se_full.rs:78`,
`form8995.rs:166`, `schedule_d_full.rs:61`).

**No `From<Usd>`, no `Add`, no `Sum`, no `Default`.** ★ **And `#[non_exhaustive]` is explicitly
rejected** — it forces a `_` arm on downstream matches, making `_ => Usd::ZERO` *easier*. It would make
this worse.

**Scope — four files, not one** (engineering I-1). Every money field on every struct that reaches paper:
`printed.rs` (16 structs), `other_taxes.rs` (`Form8959Lines`, `Form8960Lines`), `qbi.rs`
(`Form8995Lines`), `form6251.rs` (`Form6251`), and `forms.rs` (`ScheduleDPart`, `ScheduleDTotals` — the
crypto slice, which never calls `push_money` at all). **r1 scoped `printed.rs` alone, which silently
excluded Form 6251 — its own headline example.**

★ `Printed8949Totals` (`printed.rs:53`) carries the layer's only `#[derive(Default)]`. **Delete the
derive**; do not add `impl Default for LineEntry`. Even a `Blank`-yielding `Default` reopens the
implicit-construction door.

## 3. The combinator grammar — keyed to the FLOOR, not the verb

`LineEntry` is constructible **only** through these. r1 had four productions; the doctrine lens found
≥25 fields fitting none, against r1's own stated threshold of five.

| # | constructor | the form says | blank when |
|---|---|---|---|
| 1 | `collected(Collected<Usd>)` | *(the filer supplies it)* | `NotAsked` — **never** for `Stated(0)` |
| 2 | `carry(LineEntry)` | *\"Enter the amount from line N\"* | the source is blank |
| 3 | `combine(&[LineEntry])` | *\"Add / Combine lines X–Y\"*, **no floor clause** | **iff every operand is blank** |
| 4 | `floored(Usd)` | any arithmetic **+ *\"if zero or less, enter -0-\"*** | **never** |
| 5 | `scaled(LineEntry, Rate)` | *\"Multiply line N by 7.5% (0.075)\"* | the source is blank |
| 6 | `bounded(Bound, &[LineEntry])` | *\"Enter the smaller/larger of …\"* | all inputs blank |
| 7 | `constant(Usd)` | *\"Threshold based on filing status\"* | never |
| 8 | `conditional(Cond, LineEntry)` | *\"If line 33 is more than line 24, …\"* | the condition is unmet |

★★★ **Production 4's trigger is the FLOOR CLAUSE, not the verb.** r1's table keyed `computed` on
*\"Subtract…\"/\"Combine…\"* while its own next paragraph said the discriminator was the floor. Because the
table is what an implementer transcribes from, **17 fields that say \"Combine\" with no `-0-` clause
would have been forced to print a sworn zero** — including Schedule 1 line 10 (*the very line §5.1's
first plant simulates*) and Schedule D line 7, on the one form btctax always emits. Those are now
production 3, and blank when every operand is blank.

★★ **Production 3's `iff` has a second exit: SKIP** (doctrine I-1). Schedule SE line 8d is *\"Add lines
8a, 8b, and 8c\"* but sits inside *\"If $168,600 or more, skip lines 8b through 10\"* — the form wants 8d
**blank while 8a is entered**. `combine` alone gives `Entered`. So production 3 takes an optional
skip predicate, and the same machinery covers Schedule D's *\"skip lines 17 through 20\"* and Schedule SE
4c's *\"if less than $400, stop\"*. `schedule_se_full.rs:80-92` already ships a per-line include flag —
extend it, do not invent beside it.

★ **Production 3's trigger is a phrase family, not one phrase** (doctrine M-2): *\"Add the amounts in the
far right column for lines 4 through 16\"* (Schedule A 17) lists a **range whose operands are 4, 7, 10,
14, 15, 16 only** — transcribing the range literally double-counts 5a–5d. Operands are transcribed from
the form's *column*, never inferred from the range.

**P0 exit criterion — the coverage check is DONE, not deferred.** Every money field on every struct in
§2.2's scope is assigned a production, in a committed table, with the instruction quoted verbatim.
**r1 deferred this to P0 and shipped a grammar that failed it by 5–8×.** ★ If any field fits none, that
is a finding requiring a documented exception; **five or more and the grammar returns to review.**

## 4. What the compiler enforces, and what it cannot

**Compiler:** retyping the fields emits **E0308** at every struct literal — r1 cited E0063, which is the
*adding-a-field* mechanism and is not what happens here (engineering M-1). Measured: **87 struct-literal
sites** across the workspace (46 in `printed.rs`, the rest in tests) and **13 constructor `fn`s**, not
16. Money-write sites: **44** (31 `push_money`, 3 `push_literal`, 10 direct `fmt_money`).

★ **Verified clean** (engineering): no `serde` derive on any printed struct, no `impl From`, no builder,
no struct-update syntax. Retyping really does error every literal — once §2.2's `Default` derive is gone.

**Compiler cannot:** verify the *chosen production matches the instruction*. That residue is the gate's.

## 5. The gate — the disposition KAT

**Every change here is a suppression, and a suppression that over-fires is invisible.** Neither oracle
sees it; no golden moves.

`transcribe::extract_lines(pdf, map_toml) -> BTreeMap<String, String>` inserts a cell only when the field
exists **and** its text is non-empty — so **an absent key is a blank cell on paper.** Verified, and
already asserted by `tests/extract_lines.rs`.

New KAT in `crates/btctax-forms/tests/`. **It must never use `Blank::AbsentIsZero`.**

1. **Line set from the committed map TOMLs**, via the predicate `walk` already uses — *keep a string
   leaf only if it is a real field in `collect_fields`*. This drops `[census]`/`[identity]`/`form`/`year`
   metadata with **no skip-list** (engineering M-3; the naive enumerator needs one and would rot).
   ★ Note: **one line is not one key** — `MoneyPair` lines transcribe as `.dollars_field`/`.cents_field`,
   Schedule D rows as `.proceeds_d`/`.cost_e`/`.adj_g`/`.gain_h`.
2. **The instructed-zero set is COMPUTED**, by grepping **the form files only** —
   `design/forms/extract/f*--YYYY.txt`. ★ **Not `*.txt`** (doctrine I-3): that sweeps the ~1.3 MB
   `i*` instruction files, where `-0-` appears in conditional prose — `i1040gi:23500` would enroll 1040
   35a/36 on a branch almost no return takes. Verified: the restricted glob returns non-zero on exactly
   the forms that should have it and zero on those that should not.
   ★★ **Schedule 1 has no extract at all**, so it is invisible to both this and §3's check while btctax
   emits it. **P0 must add `f1040s1--2024.txt`** — otherwise a future Schedule 1 `-0-` can never join,
   which is the stale-list failure this design exists to avoid.
   ★ Membership is *not* decided by the grep alone where the clause is itself conditional (Schedule SE
   4c, 5b). The grep produces the **candidate** set; each candidate is adjudicated once, against the
   instructions, and recorded with its reason.
3. **Three fixtures, one instrument.** Expectations are written **from each fixture's INPUTS**, never
   from the printed struct — that independence defeats consistent-but-wrong.

### 5.1 B1 — three plants, all watched red before P0 merges

| # | plant | simulates | must red |
|---|---|---|---|
| 1 | reintroduce `sch_1.map_or(Usd::ZERO, …)` (`printed.rs:591`) | a zero that should not print | **absence** vector |
| 2 | make the writer skip every zero value | over-firing suppression | **instructed-zero** vector |
| 3 | `collected` as `match x { Some(v) if !v.is_zero() => Entered(v), _ => Blank }` | **a filer-STATED zero deleted** | **stated-zero** vector |

★★★ **Plant 3 is new and r1 had no defence against it** (engineering I-4). It compiles, is a plausible
thing to write, silently deletes genuine testimony — and **passed both r1 plants**, because the absence
vector states nothing and the instructed-zero vector's zeros arrive via production 4, not `collected`.
§2.1's entire case for `Collected<T>` is that `Stated(0)` ≠ `NotAsked`, and r1's gate never tested it.
The third fixture states `$0` withholding / `$0` estimated payments and asserts `Blank::PresentZero`
(`tests/common/mod.rs:201`, which already exists and panics on an absent key) at 1040 L25a/L26.

**Stated limits.** The KAT pins fixtures, not all `-0-` branches; a completeness census is P4. An author
who writes both a fabrication and an expectation blessing it passes — the gate forces it to be written
down **twice, in review-visible places**, the same strength the classifier has.

★ **Audit the one existing `AbsentIsZero` use** (engineering M-4): `golden_packet.rs:321` reads 1040 L13
that way — the QBI line, which P1 flips to absent. Left alone it stays green through P1/P2, silencing
the one golden that could have spoken.

## 6. Containment — the compiler, then the lint, then the grep

r1 proposed a test-support trait plus a grep. **Insufficient**: the grep bans the honest accessor
exactly where production needs it while missing the inline `match`, pushing authors toward the
un-greppable form. And it scoped \"the ten published crates\", exempting `btctax-oracle-harness`
(`publish = false`) — the one crate r1 said needed the accessor.

**Corrected, in strength order:**

1. **Opacity (§2.2) is the primary mechanism.** Outside the module there is nothing to `match`.
2. **`clippy.toml` `disallowed-methods`** on whatever numeric exit is published. `make check` already
   runs `clippy --workspace --all-targets -- -D warnings`, so this is a hard build failure overridable
   only by a visible `#[allow]`. The repo has no `clippy.toml` today; add one.
3. **The hygiene grep** as a backstop only, ships with its own planted-defect kill.

★ **The real consumer inventory** (engineering I-3), which r1 got wrong: `btctax-cli/src/render.rs`
does ~23 printed-field reads **in a published crate**; three emitter sites compare; the oracle harness
does arithmetic at `main.rs:324`. These need a **published, deliberate** numeric exit — not a
test-support back door. ★ And a `test-support` feature **cannot** serve the harness: it depends on core
normally, so feature unification would re-expose the accessor everywhere. The harness writes explicit
matches inside the defining crate, or uses the published exit.

## 7. Phasing

**P0 — vocabulary, scope, gate. ZERO BEHAVIOUR CHANGE ON PAPER.**
Define the types (opaque). Migrate the writer signature; **every call site passes an entered value**.
Fix the two Schedule D emitter literals. Delete the `Printed8949Totals` `Default` derive. Add
`f1040s1--2024.txt`. **Complete the §3 coverage table over all four scoped files.** Build the KAT with
every current disposition pinned as entered, **all three plants watched red**, `clippy.toml`, and the
hygiene gate with its kill. Audit `golden_packet.rs:321`.
**Green =** suite green, goldens byte-unchanged (md5 `c4e1853…`), three kills demonstrated.
★ Traced sound by the engineering lens: the writer is a pure formatter, so byte-identity holds; and
suppression is **geometrically safe** because `verify.rs:390-414` requires strict descent, not
contiguity.

**P0b — the uniform retyping** (engineering I-2). Retype **every** money field in §2.2's scope to
`LineEntry`, all constructed via §3. r1's phases named ~60 of 168 and never scheduled the rest, leaving
the mixed patchwork §2.2 says produced the 64 — and making §4's guarantee false for every field still
`Usd`. Byte-unchanged on paper (all entered), so it meets the same bar as P0.

**P1 — stop discarding what we know.** The 15 flattened `Option`s + 8995 lines 3/7 (delete the
`ptr::eq` gate — its *\"only line on this form\"* comment is falsified by line 7 one iteration later).
Flip those rows to absent. No owner decision needed.

**P2 — manufactured inputs and the silent forms.** `Collected<Usd>` over the 14 `#[serde(default)] Usd`
scalars; the 20 empty-`Vec` sums and 9 blank-column totals. Schedules 1/2/3/C first. 1040 25a/26 wait on
owner decision 2.
★ This is where an existing TOML omitting a key changes meaning from `0` to `NotAsked`. btctax has no
users and v0.15.0 is unpublished — **free now, expensive later.**

**P3 — make the 65th site fail to compile.** Extend the classifier's `_`-ban to money leaves.

**P4 — residue.** `\"0\"` vs `\"-0-\"`; the Schedule SE 8d/9/10 mirror defect (adjudicate against
`i1040sse`); the `-0-` completeness census; supplied-then-zeroed and unasked-elections as separate
classes.

## 8. Owner decisions as variation points

Unchanged by r2 — the architecture still does not depend on them. Each is one production choice plus one
expectation row: (1) is a reconciled ledger the filer's testimony? (2) blank or refuse where silence
asserts? (3) is supplied-then-zeroed in scope? (4) sequencing vs Form 6251 Tier 2.

## 9. Risks carried

- **The §3 coverage table is now a P0 deliverable, not a P0 aspiration.** It is the thing r1 got wrong.
- **The three-production→eight-production expansion may still be incomplete.** The doctrine lens counted
  ≈25 misfits against four productions; §3 now has eight, but the re-review must re-run that count.
- **Map coverage** is **already half-closed** by `verify.rs:229 no_unmapped_filled` — a cell written
  outside any map fails closed as `UnmappedField` (engineering M-5). Build the census on that invariant
  rather than beside it; a second writes-set would be a second truth.
- **1040 line 16** (*\"Tax (see instructions)\"*) has no verb at all — it comes from the Tax Table or the
  QDCGT worksheet. It is the strongest candidate for §3's documented exception.
","numLines":262,"startLine":1,"totalLines":262}},"sourceToolAssistantUUID":"786cbda2-5334-400b-9e0c-064d28f0cae2","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"09683569-b5b9-441b-9cb8-09fd89fd05d0","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRibmn1yCVut3WdFKg5","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISgAMKhwEIEBgCKkAxz6J1RgC0JJTcolUlh+02NVpoGha7yn4PGckwsp0cdwQvgf43ng1FX356mxgSeIBnejkap/L3lJAen9EbpOjBMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDEWyleTMxEySSnBjYRoMHGDZD3pAYG9Vj75ZIjBr4coTWS4yx+WtPWKKGM9AOfLRt66lzMjScJBzGeRBb2QvY02+ypfpUgclCqRAEoQqpQGkGwuyMcz41rwYMM2LRyW2b1CPYieDc63J1VKEWGt6wnt29DN8aWWptz9kFepZXAxPlphRImrgefZIh4vJMIjYkrbOptVizfhKjAYkbUoVMOh+a82qdoG6BHr6Dc5dq0KyqcuOWcHiwQBl8c6pTCk7pfQyPO20EXcDWaEz8eLwZUulLHKSRbQJHnkPCcu9SgPTYp9AyY1VYz2LRdklq4V8br4nT0wYAQ=="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":7589,"cache_read_input_tokens":33940,"cache_creation":{"ephemeral_5m_input_tokens":7589,"ephemeral_1h_input_tokens":0},"output_tokens":4,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRiapUWNVJVA3oiBuTT","attributionAgent":"general-purpose","type":"assistant","uuid":"37f486d5-a299-45be-b936-727b5a4cb0f8","timestamp":"2026-08-01T04:50:51.756Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"37f486d5-a299-45be-b936-727b5a4cb0f8","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRibmn1yCVut3WdFKg5","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01DPSB5r7HDszvpbrFKkp1Ht","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax/design/no-testimony/reviews && grep -n \"^## \\|^# \\|^VERDICT\\|^\\*\\*I-[0-9]\\|^### \" SPEC-engineering-opus-r1.md | head -80"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":7589,"cache_read_input_tokens":33940,"output_tokens":185,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":7589},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":185,"cache_read_input_tokens":33940,"cache_creation_input_tokens":7589,"cache_creation":{"ephemeral_5m_input_tokens":7589,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRiapUWNVJVA3oiBuTT","attributionAgent":"general-purpose","type":"assistant","uuid":"4524026d-77e1-4f7c-b1d6-4371c27139ba","timestamp":"2026-08-01T04:50:53.130Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"4524026d-77e1-4f7c-b1d6-4371c27139ba","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01DPSB5r7HDszvpbrFKkp1Ht","type":"tool_result","content":"1:# §G-11 SPEC review — ENGINEERING LENS (Opus)
193:## The ONE question
202:## Where the risk actually is
229:## Doctrine to check against (`/scratch/code/bitcoin_tax/CLAUDE.md`)
239:## Settled — do NOT re-derive
248:## Output
502:## 1. The problem, and the scope boundary
521:## 2. The two types
525:### 2.1 `Collected<Usd>` — the input boundary (`btctax-core/src/tax/return_inputs.rs`)
542:### 2.2 `LineEntry` — the printed layer (`btctax-core/src/tax/printed.rs`)
565:## 3. The combinator grammar — the form's own instructions
591:## 4. What the compiler enforces, and what it cannot
601:## 5. The gate — the disposition KAT
631:### 5.1 B1 — the two plants, both watched red before P0 merges
648:## 6. ★★ The leak risk, and its containment
663:## 7. Phasing
695:## 8. Owner decisions as variation points
706:## 9. Risks carried, in the consult's own words
2437:# Schedule 3 (Additional Credits and Payments), Form 1040, TY2024 — logical line → PDF field name.
2439:# Extracted with `xtask dump-fields` + correlated against the printed form (`pdftotext -layout`).
2441:# ⚠️ ROOT SUBFORM IS `topmostSubform[0]` — NOT the `form1[0]` that Schedules 1 and 2 use. Schedule 3
2442:# is also a ONE-PAGE form, yet it contains a field named `f2_23` (the second write-in row of line 6z):
2443:# an IRS authoring slip, since the `f2_` prefix normally means page 2 and no `f1_23` exists anywhere in
2444:# the document. Any page heuristic keyed on the LEAF prefix will disagree with the `PageN[0]` subform
2445:# segment, which correctly says Page1. v1 does not fill 6z, but the trap is recorded for whoever does.
2447:# Column x-clusters (code-side oracle in schedule3.rs): MID x ≈ [410,482], AMOUNT x ≈ [504,576].
2449:# ⚠️ Line 6e (`f1_13`) is \\\"Reserved for future use\\\" — a LIVE, ReadOnly widget that CONSUMES a suffix
2450:# number (f1_12 = 6d, f1_13 = 6e reserved, f1_14 = 6f). Anything that walks suffixes and \\\"skips
2451:# reserved lines\\\" misaligns 6f–6m by one. Never write to it.
2453:# UNMAPPED ON PURPOSE:
2454:#   lines 2, 3, 4, 5a, 5b, 6a-6z — education (8863), dependent-care (2441), saver's (8880),
2455:#     residential-energy (5695), adoption (8839) and the rest. Every one is a §3.4 CONSERVATIVE
2456:#     OMISSION: v1 does not compute them, which can only OVERSTATE tax, and the report fires the
2457:#     OtherCreditsOmitted advisory. They stay BLANK — never a misleading 0.
2458:#   lines 9, 10, 12, 13a-13z — net premium tax credit, extension payment, fuels credit, etc.
2462:# ── Part I — Nonrefundable Credits ──────────────────────────────────────────────────────────────
2466:# ── Part II — Other Payments and Refundable Credits ─────────────────────────────────────────────
2467:# L10 — correlated against the printed Part II column (AMOUNT x≈504-576, descending y): f1_27=L9,
2468:# f1_28=L10, f1_29=L11 (two independent anchors: f1_29=L11 and f1_39=L15 were already verified).
2473:# ── Identity header (P6.2). Dumped + correlated against the printed form; the SSN cell declares
2474:#    /MaxLen 11, so it takes the HYPHENATED form (the 1040's own SSN cells are /MaxLen 9 and take
2475:#    bare digits — the forms genuinely differ, so `push_identity` reads each cell's capacity).
2480:# ── §G-13 FIELD-PROVENANCE CENSUS ────────────────────────────────────────────────────────────────
2481:# Every AcroForm field on this PDF the map above does not fill, with the decision that leaves it blank.
2482:# `unmodeled` = btctax models no input that could reach the line; `artifact` = not an entry the filer
2483:# makes; `gap` = the line IS reachable and nothing populates it (a defect).
2485:# ★ DIRECTION — this whole form is CREDITS AND PAYMENTS. Every line here only ever REDUCES tax or
2486:#   increases the refund, so each blank is a class (B) forgone benefit: lawful silence that can only
2487:#   OVERSTATE the tax. There is no understatement risk anywhere on Schedule 3. Recording that once here
2488:#   is why the per-line reasons below stay short.
2490:# ★ Field names do not track pages or lines: `f2_23` sits on PAGE 1 between `f1_22` and `f1_24`. One
2491:#   more reason boxes are enumerated from the AcroForm walk and never from name arithmetic.
2494:# ── Part I lines 2–5b: nonrefundable credits from forms btctax does not emit ─────────────────────
2501:# ── Line 6a–6m: the other-nonrefundable-credit menu ──────────────────────────────────────────────
2516:# ── Line 6z / 13z — the free-text \\\\\\\"list type and amount\\\\\\\" escape hatches ──────────────────────────
2517:# ★ These are the only WRITE-IN lines on the form: two text cells for the description plus one amount
2518:#   cell each. btctax never writes a free-text credit onto a return. That is deliberate, not an
2519:#   omission — a write-in is unstructured testimony the software cannot substantiate, and the product's
2520:#   posture is that anything it cannot name and compute does not get asserted under §6065.
2525:# ── Derived totals whose every input is blank ────────────────────────────────────────────────────
2526:# Lines 7 and 14 sum only lines btctax never populates, so each is a computed zero over an all-blank
2527:# set. IRS convention leaves such a sum blank; the mapped totals (line 8, line 15) are correct without
2528:# it because line 8 = line 1 + 7 and line 7 contributes nothing.
2532:# ── Part II lines 9, 12, 13a–13z: other payments and refundable credits ──────────────────────────
2544:# Schedule D, tax year 2024 — logical line → fully-qualified PDF field name.
2545:# Part I/II amount columns (d,e,g,h) live under Table_PartI / Table_PartII (the hardcoded
2546:# SCHED_D_TABLE_TOKEN \\\"Table_PartI\\\" matches both, unchanged for 2024/2025). Line 16 sits on page 2.
2547:# Field names match TY2025 EXCEPT line 16 (2024 = f2_01, not 2025's f2_1) — verified at extraction.
2560:# ── FULL-RETURN extension (P6). The crypto-slice fill above uses only lines 3/7/10/15/16; the
2561:# full return additionally needs the carryovers, the capital-gain distributions, and Part III's
2562:# lines 17-22. These keys are absent from the 2017/2025 maps, which is why they are optional.
2564:# ★ PARENTHESIZED BOXES — the form pre-prints the minus sign, so these carry POSITIVE MAGNITUDES.
2565:#   A negative written here renders as a POSITIVE number on a filed return.
2566:#     line6  (f1_21, x=[507,573] inset)  short-term capital loss carryover
2567:#     line14 (f1_42, x=[507,573] inset)  long-term capital loss carryover
2568:#     line21 (f2_04, x=[492.6,573])      the §1211(b) allowed loss offset
2570:# ★ THE AMOUNT COLUMN DIFFERS BETWEEN THE TWO PAGES OF THE SAME FORM: page 1's column (h) and its
2571:#   standalone cells are x=[504,576] (center 540); page 2's are x=[489.6,576] (center 532.8). One
2572:#   band covering both (489-576) holds each center, so the oracle needs no per-page cluster — but a
2573:#   band pinned to page 1 alone would REJECT every page-2 cell.","is_error":false}]},"uuid":"b908e489-a9a9-4f7d-b823-ea642cc5d67a","timestamp":"2026-08-01T04:50:53.205Z","toolUseResult":{"stdout":"1:# §G-11 SPEC review — ENGINEERING LENS (Opus)
193:## The ONE question
202:## Where the risk actually is
229:## Doctrine to check against (`/scratch/code/bitcoin_tax/CLAUDE.md`)
239:## Settled — do NOT re-derive
248:## Output
502:## 1. The problem, and the scope boundary
521:## 2. The two types
525:### 2.1 `Collected<Usd>` — the input boundary (`btctax-core/src/tax/return_inputs.rs`)
542:### 2.2 `LineEntry` — the printed layer (`btctax-core/src/tax/printed.rs`)
565:## 3. The combinator grammar — the form's own instructions
591:## 4. What the compiler enforces, and what it cannot
601:## 5. The gate — the disposition KAT
631:### 5.1 B1 — the two plants, both watched red before P0 merges
648:## 6. ★★ The leak risk, and its containment
663:## 7. Phasing
695:## 8. Owner decisions as variation points
706:## 9. Risks carried, in the consult's own words
2437:# Schedule 3 (Additional Credits and Payments), Form 1040, TY2024 — logical line → PDF field name.
2439:# Extracted with `xtask dump-fields` + correlated against the printed form (`pdftotext -layout`).
2441:# ⚠️ ROOT SUBFORM IS `topmostSubform[0]` — NOT the `form1[0]` that Schedules 1 and 2 use. Schedule 3
2442:# is also a ONE-PAGE form, yet it contains a field named `f2_23` (the second write-in row of line 6z):
2443:# an IRS authoring slip, since the `f2_` prefix normally means page 2 and no `f1_23` exists anywhere in
2444:# the document. Any page heuristic keyed on the LEAF prefix will disagree with the `PageN[0]` subform
2445:# segment, which correctly says Page1. v1 does not fill 6z, but the trap is recorded for whoever does.
2447:# Column x-clusters (code-side oracle in schedule3.rs): MID x ≈ [410,482], AMOUNT x ≈ [504,576].
2449:# ⚠️ Line 6e (`f1_13`) is \\\"Reserved for future use\\\" — a LIVE, ReadOnly widget that CONSUMES a suffix
2450:# number (f1_12 = 6d, f1_13 = 6e reserved, f1_14 = 6f). Anything that walks suffixes and \\\"skips
2451:# reserved lines\\\" misaligns 6f–6m by one. Never write to it.
2453:# UNMAPPED ON PURPOSE:
2454:#   lines 2, 3, 4, 5a, 5b, 6a-6z — education (8863), dependent-care (2441), saver's (8880),
2455:#     residential-energy (5695), adoption (8839) and the rest. Every one is a §3.4 CONSERVATIVE
2456:#     OMISSION: v1 does not compute them, which can only OVERSTATE tax, and the report fires the
2457:#     OtherCreditsOmitted advisory. They stay BLANK — never a misleading 0.
2458:#   lines 9, 10, 12, 13a-13z — net premium tax credit, extension payment, fuels credit, etc.
2462:# ── Part I — Nonrefundable Credits ──────────────────────────────────────────────────────────────
2466:# ── Part II — Other Payments and Refundable Credits ─────────────────────────────────────────────
2467:# L10 — correlated against the printed Part II column (AMOUNT x≈504-576, descending y): f1_27=L9,
2468:# f1_28=L10, f1_29=L11 (two independent anchors: f1_29=L11 and f1_39=L15 were already verified).
2473:# ── Identity header (P6.2). Dumped + correlated against the printed form; the SSN cell declares
2474:#    /MaxLen 11, so it takes the HYPHENATED form (the 1040's own SSN cells are /MaxLen 9 and take
2475:#    bare digits — the forms genuinely differ, so `push_identity` reads each cell's capacity).
2480:# ── §G-13 FIELD-PROVENANCE CENSUS ────────────────────────────────────────────────────────────────
2481:# Every AcroForm field on this PDF the map above does not fill, with the decision that leaves it blank.
2482:# `unmodeled` = btctax models no input that could reach the line; `artifact` = not an entry the filer
2483:# makes; `gap` = the line IS reachable and nothing populates it (a defect).
2485:# ★ DIRECTION — this whole form is CREDITS AND PAYMENTS. Every line here only ever REDUCES tax or
2486:#   increases the refund, so each blank is a class (B) forgone benefit: lawful silence that can only
2487:#   OVERSTATE the tax. There is no understatement risk anywhere on Schedule 3. Recording that once here
2488:#   is why the per-line reasons below stay short.
2490:# ★ Field names do not track pages or lines: `f2_23` sits on PAGE 1 between `f1_22` and `f1_24`. One
2491:#   more reason boxes are enumerated from the AcroForm walk and never from name arithmetic.
2494:# ── Part I lines 2–5b: nonrefundable credits from forms btctax does not emit ─────────────────────
2501:# ── Line 6a–6m: the other-nonrefundable-credit menu ──────────────────────────────────────────────
2516:# ── Line 6z / 13z — the free-text \\\\\\\"list type and amount\\\\\\\" escape hatches ──────────────────────────
2517:# ★ These are the only WRITE-IN lines on the form: two text cells for the description plus one amount
2518:#   cell each. btctax never writes a free-text credit onto a return. That is deliberate, not an
2519:#   omission — a write-in is unstructured testimony the software cannot substantiate, and the product's
2520:#   posture is that anything it cannot name and compute does not get asserted under §6065.
2525:# ── Derived totals whose every input is blank ────────────────────────────────────────────────────
2526:# Lines 7 and 14 sum only lines btctax never populates, so each is a computed zero over an all-blank
2527:# set. IRS convention leaves such a sum blank; the mapped totals (line 8, line 15) are correct without
2528:# it because line 8 = line 1 + 7 and line 7 contributes nothing.
2532:# ── Part II lines 9, 12, 13a–13z: other payments and refundable credits ──────────────────────────
2544:# Schedule D, tax year 2024 — logical line → fully-qualified PDF field name.
2545:# Part I/II amount columns (d,e,g,h) live under Table_PartI / Table_PartII (the hardcoded
2546:# SCHED_D_TABLE_TOKEN \\\"Table_PartI\\\" matches both, unchanged for 2024/2025). Line 16 sits on page 2.
2547:# Field names match TY2025 EXCEPT line 16 (2024 = f2_01, not 2025's f2_1) — verified at extraction.
2560:# ── FULL-RETURN extension (P6). The crypto-slice fill above uses only lines 3/7/10/15/16; the
2561:# full return additionally needs the carryovers, the capital-gain distributions, and Part III's
2562:# lines 17-22. These keys are absent from the 2017/2025 maps, which is why they are optional.
2564:# ★ PARENTHESIZED BOXES — the form pre-prints the minus sign, so these carry POSITIVE MAGNITUDES.
2565:#   A negative written here renders as a POSITIVE number on a filed return.
2566:#     line6  (f1_21, x=[507,573] inset)  short-term capital loss carryover
2567:#     line14 (f1_42, x=[507,573] inset)  long-term capital loss carryover
2568:#     line21 (f2_04, x=[492.6,573])      the §1211(b) allowed loss offset
2570:# ★ THE AMOUNT COLUMN DIFFERS BETWEEN THE TWO PAGES OF THE SAME FORM: page 1's column (h) and its
2571:#   standalone cells are x=[504,576] (center 540); page 2's are x=[489.6,576] (center 532.8). One
2572:#   band covering both (489-576) holds each center, so the oracle needs no per-page cluster — but a
2573:#   band pinned to page 1 alone would REJECT every page-2 cell.","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"4524026d-77e1-4f7c-b1d6-4371c27139ba","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"b908e489-a9a9-4f7d-b823-ea642cc5d67a","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRirjgm5Saoy2pYd5CE","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISmgMKhwEIEBgCKkB2m42FXjU2ToCqE3lQuRyLHT6kwgnG0IYhmcnbXxuhQsIO6+qRsS7tj9/sMYFoFw1cSZJbJHGgilWjjBvqlW7wMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDHfMoZhYq23gjDD64hoMno5uIJ+1HPDdUAcZIjAhH4SbbK9EoA4SOth0D0V7U3b3hmwjWGDDpcvu+oPwCA/LIkdig7GrBIdrd5TcmmYqvwE4UHd/0BH9iP/okRvq5twA87ghQ1REc3GXodEnsbSuTsrdnVJ3KHazOgoMbPT+9QWMhAbTrJb0f8biqUfIEU3ETBiGXpSY9eD86lEQMzUXmURuJviVDRcSIUrmoPeZphX2cMdpDMCaccFCOFCT2L0kiHK2K3k7fcIvi+aRMj5K2F7CPvhkhl6pKRWAdq11dzPW/EoE8FN26b8ixCokfgpA7AnXPXMU8CZR6XmxFmiz+GwNEWOQoOAAMD7i1d1olxgB"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":3191,"cache_read_input_tokens":41529,"cache_creation":{"ephemeral_5m_input_tokens":3191,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRiq5E73nDEVF6ytwrB","attributionAgent":"general-purpose","type":"assistant","uuid":"acfe52b9-677f-4f80-b4ee-cd72c4754a3b","timestamp":"2026-08-01T04:50:56.177Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"acfe52b9-677f-4f80-b4ee-cd72c4754a3b","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRirjgm5Saoy2pYd5CE","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01PLoV3PTRYybXJzKoxFWGkL","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax/design/no-testimony/reviews && grep -n \"VERDICT\\|ALSO-CHECKED-SOUND\\|ALSO CHECKED\" SPEC-engineering-opus-r1.md | head -20"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":3191,"cache_read_input_tokens":41529,"output_tokens":194,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":3191},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":194,"cache_read_input_tokens":41529,"cache_creation_input_tokens":3191,"cache_creation":{"ephemeral_5m_input_tokens":3191,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRiq5E73nDEVF6ytwrB","attributionAgent":"general-purpose","type":"assistant","uuid":"d2772c66-4a05-492e-91fa-292257f72634","timestamp":"2026-08-01T04:50:57.528Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"d2772c66-4a05-492e-91fa-292257f72634","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01PLoV3PTRYybXJzKoxFWGkL","type":"tool_result","content":"167:63\\t`VERDICT: green` or `VERDICT: <n> Critical / <n> Important`
184:80\\tEnd with `ALSO CHECKED, SOUND:` and `WHAT WOULD MAKE THIS REVIEW WRONG:`.
250:`VERDICT: green` or `VERDICT: <n> Critical / <n> Important`
267:End with `ALSO CHECKED, SOUND:` and `WHAT WOULD MAKE THIS REVIEW WRONG:`.
5650:{\"parentUuid\":\"fc2e3ea5-9177-4158-9106-d4c46a5c4ab8\",\"isSidechain\":true,\"agentId\":\"a84b7406ef53120af\",\"message\":{\"model\":\"claude-opus-5\",\"id\":\"msg_011CdbQDHocSwbjTeUx9AGFd\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"VERDICT: 0 Critical / 4 Important
5739:## ALSO CHECKED, SOUND:","is_error":false}]},"uuid":"6f30f6b9-9252-4882-80bd-88409a29ffdf","timestamp":"2026-08-01T04:50:57.607Z","toolUseResult":{"stdout":"167:63\\t`VERDICT: green` or `VERDICT: <n> Critical / <n> Important`
184:80\\tEnd with `ALSO CHECKED, SOUND:` and `WHAT WOULD MAKE THIS REVIEW WRONG:`.
250:`VERDICT: green` or `VERDICT: <n> Critical / <n> Important`
267:End with `ALSO CHECKED, SOUND:` and `WHAT WOULD MAKE THIS REVIEW WRONG:`.
5650:{\"parentUuid\":\"fc2e3ea5-9177-4158-9106-d4c46a5c4ab8\",\"isSidechain\":true,\"agentId\":\"a84b7406ef53120af\",\"message\":{\"model\":\"claude-opus-5\",\"id\":\"msg_011CdbQDHocSwbjTeUx9AGFd\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"VERDICT: 0 Critical / 4 Important
5739:## ALSO CHECKED, SOUND:","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"d2772c66-4a05-492e-91fa-292257f72634","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"6f30f6b9-9252-4882-80bd-88409a29ffdf","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRjAxJoHWRepCbGd9VB","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_013FQn9TQZJQGiPXmQtUvJVq","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax/design/no-testimony/reviews && sed -n '5650,5755p' SPEC-engineering-opus-r1.md"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":548,"cache_read_input_tokens":44720,"output_tokens":104,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":548},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":104,"cache_read_input_tokens":44720,"cache_creation_input_tokens":548,"cache_creation":{"ephemeral_5m_input_tokens":548,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRj9qLtYyGLVnzVjF4E","attributionAgent":"general-purpose","type":"assistant","uuid":"0b79fbc8-85b2-43fa-924f-2cff098bfc79","timestamp":"2026-08-01T04:50:59.234Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"0b79fbc8-85b2-43fa-924f-2cff098bfc79","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_013FQn9TQZJQGiPXmQtUvJVq","type":"tool_result","content":"{\"parentUuid\":\"fc2e3ea5-9177-4158-9106-d4c46a5c4ab8\",\"isSidechain\":true,\"agentId\":\"a84b7406ef53120af\",\"message\":{\"model\":\"claude-opus-5\",\"id\":\"msg_011CdbQDHocSwbjTeUx9AGFd\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"VERDICT: 0 Critical / 4 Important

---

SEVERITY: Important
WHERE: SPEC.md §2.2 + §4 (scope of \\\"the printed layer\\\")
CLAIM: Three of the emitted forms' printed-line structs — and Form 6251, the spec's own headline example — do not live in `printed.rs`, so \\\"every money field on every printed struct\\\" and \\\"all 16 printed-struct constructors\\\" scope the migration to one of four files that hold money reaching paper.
CONSEQUENCE: §2.2's stated benefit (\\\"Tier-2 Form 6251 fields land safe by default: `line2i` … cannot ship as a silent zero if it must choose a constructor\\\") is not delivered by anything the spec scopes. Form 8959/8960/8995/6251 lines stay bare `Usd`, and every one of them remains exactly the \\\"65th site\\\" §4 promises cannot exist.
EVIDENCE: Spec: \\\"`LineEntry` — the printed layer (`btctax-core/src/tax/printed.rs`)\\\" and \\\"retyping the fields `E0063`s all **16** printed-struct constructors\\\". `printed.rs` contains exactly 16 printed structs — that is where the 16 comes from. But the structs the emitters actually consume also include:
- `crates/btctax-core/src/tax/other_taxes.rs:96` `pub struct Form8959Lines`, `:257` `pub struct Form8960Lines`
- `crates/btctax-core/src/tax/qbi.rs:171` `pub struct Form8995Lines`
- `crates/btctax-core/src/tax/form6251.rs:89` `pub struct Form6251`
- `crates/btctax-core/src/forms.rs:171/185` `ScheduleDPart` / `ScheduleDTotals` — the crypto-slice Schedule D fill (`schedule_d.rs:60`) takes `&ScheduleDTotals`, not a printed struct, and never calls `push_money` at all.
Line-read counts confirm the mass: `form6251.rs` 115, `other_taxes.rs` 79, `qbi.rs` 58. The spec knows the surface is wider (P1 names `fold.rs:1151` and \\\"Schedule 2 L12\\\"), but §2.2/§4's scope statement and the \\\"16\\\" do not.

---

SEVERITY: Important
WHERE: SPEC.md §7 (phasing) vs §2.2 (\\\"uniformly\\\") and §4 (the guarantee)
CLAIM: No phase performs the uniform retyping that §2.2 mandates and on which §4's guarantee depends; P0–P4 together name roughly 58 of the 168 money quantities.
CONSEQUENCE: The printed surface stays a mixed `Usd`/`LineEntry` patchwork — which §2.2 itself identifies as the thing that \\\"produced the 64 in the first place\\\" — and §4's \\\"a 65th site cannot choose a silent zero\\\" is simply false for every field still typed `Usd`. The program degrades from \\\"two types, not 64 fixes\\\" back to \\\"64 fixes, plus a type.\\\" No exit criterion in any phase would catch this.
EVIDENCE: §2.2: \\\"**Every money field on every printed struct becomes `LineEntry`, uniformly** — including lines that are always entered.\\\" §7: P0 = \\\"the *types* land, the *fields* do not migrate yet\\\"; P1 = \\\"The 15 flattened `Option`s … plus 8995 lines 3/7\\\"; P2 = \\\"the 14 `#[serde(default)] Usd` scalars; the 20 empty-`Vec` sums and 9 blank-column totals\\\"; P3 = the classifier; P4 = residue. 15 + 2 + 14 + 20 + 9 = 60. The ~110 always-entered fields are never scheduled. This is an omission, not an impossibility: a uniform-retyping phase is byte-unchanged on paper (all `Entered`), so it satisfies the same green bar P0 does.

---

SEVERITY: Important
WHERE: SPEC.md §6 (the containment) — `crates/btctax-cli/src/render.rs:1475-1570`, `crates/btctax-forms/src/schedule_se_full.rs:78`, `form8995.rs:166`, `schedule_d_full.rs:61`, `crates/btctax-oracle-harness/Cargo.toml`
CLAIM: §6's leak inventory (\\\"`golden_packet.rs` and `btctax-oracle-harness`\\\") omits the largest consumer — production code in a *published* crate — and the grep it specifies bans the honest accessor precisely where it is needed while exempting the one crate the spec names.
CONSEQUENCE: The hygiene gate fires on ~25 real production sites and offers them no legal exit, so the author is pushed into the un-greppable inline `match … Blank => Usd::ZERO` that the brief identifies as the failure mode. The gate then reports success while the guarantee leaks. Meanwhile `btctax-oracle-harness` — which the spec explicitly says needs the accessor — is `publish = false`, so the grep as scoped never looks at it.
EVIDENCE: Spec: \\\"It lives on an extension trait in a **test-support module** … A **repo-hygiene test greps for it** and fails if it appears under `crates/*/src/` for any of the ten published crates.\\\"
- `crates/btctax-cli/src/render.rs:1475` `let f = &printed.f1040;` then `fmt_money(f.line9)`, `f.line10`, `f.line11`, `f.line12`, `f.line13`, `f.line15`, `f.line16`, `f.line24`, `f.line33`, `if f.line34 > Usd::ZERO`, and already `printed.sch_3.map_or(Usd::ZERO, |s3| s3.line1)` — 23 printed-field reads in `btctax-cli/src`, a published crate.
- Production comparisons inside the emitters themselves: `schedule_se_full.rs:78` `let skip_8d_to_10 = lines.line9 == Usd::ZERO;`; `form8995.rs:166` `if lines.line2 > Usd::ZERO`; `schedule_d_full.rs:61` `if v < Usd::ZERO` over `lines.line6` / `lines.line14` / `line21`.
- `crates/btctax-oracle-harness/Cargo.toml` → `publish = false`; `src/main.rs` has 45 printed-field reads including real arithmetic at `:324` `let l24_target = pr.forms.f1040.line16 + se_l12_ots + f8959_l18_ots + round_leaf(e.niit);`.

**Answering the brief's question directly — \\\"greppable and review-loud\\\" is NOT the ceiling.** Three strictly stronger mechanisms are available today:

1. **Make `LineEntry` opaque** — `pub struct LineEntry(Repr);` with `enum Repr { Entered(Usd), Blank }` private to the module. A `match` on it outside the defining module then does not compile, which is the *only* mechanism that closes the exact hole the brief names. It costs nothing here, because §3 already forbids free construction and §2.2 already forbids `From`/`Add`/`Sum`/`Default`; the combinators become methods and the exits become whatever you choose to publish (`is_blank()`, `entered() -> Option<Usd>` kept crate-private, an `Option<Ordering>` comparator for the three emitter sites above).
2. **`#[cfg(feature = \\\"test-support\\\")]` the accessor**, not merely a `testonly` module. The repo's existing precedent is *ungated* — `crates/btctax-forms/src/lib.rs:423` is a plain `pub mod testonly` — so \\\"an obviously test-shaped import\\\" is convention only. A feature gate makes a production call a **compile error**. ★ Caveat the spec must record: `btctax-oracle-harness` depends on core/forms as a *normal* dependency, so it cannot enable that feature without workspace feature unification re-exposing the accessor everywhere (`resolver = \\\"2\\\"` does not help). The harness must therefore write explicit matches — which is the honest answer anyway, and should be specified rather than discovered.
3. **`clippy.toml` `disallowed-methods`** — the repo has no `clippy.toml` today, and `make check` already runs `clippy --workspace --all-targets -- -D warnings` (`Makefile:27`). That converts the grep into a hard build failure that can only be overridden by a visible `#[allow]`.
   *Not* `#[non_exhaustive]`: it forces a `_` arm on downstream matches, making `_ => Usd::ZERO` **easier** to write. It would make this worse.

---

SEVERITY: Important
WHERE: SPEC.md §5.1 (the two B1 plants)
CLAIM: The two plants cover under-suppression and over-suppression-of-an-instructed-zero, but miss a third direction: **a filer-stated zero suppressed** — and that direction is the architecture's own headline distinction.
CONSEQUENCE: A `collected` implemented as `match x { Some(v) if !v.is_zero() => Entered(v), _ => Blank }` compiles, is a plausible thing to write, silently deletes genuine testimony from the return, and **passes both plants**. This is an over-firing suppression, which §5's own preamble calls \\\"the invisible one.\\\"
EVIDENCE: Spec §5.1's table: plant 1 reds \\\"the **absence** vector\\\"; plant 2 reds \\\"the **instructed-zero** vector\\\". The absence vector is defined as \\\"no Schedule 1, no W-2s, no carryovers\\\" — nothing is *stated*, so `Stated(Usd::ZERO)` is never exercised. The instructed-zero vector's zeros arrive via `computed` (§3: \\\"All 24 form-instructed zeros live in `computed`\\\"), not via `collected`. Yet §2.1's entire justification for `Collected<T>` over `Option<Usd>` is that `Stated(0)` and `NotAsked` must not collapse — and no fixture distinguishes them. Fix is cheap and uses machinery that already exists: a third fixture whose filer states `$0` estimated payments / `$0` withholding, asserting `Blank::PresentZero` (`crates/btctax-forms/tests/common/mod.rs:201`) on 1040 L25a/L26.

---

SEVERITY: Minor
WHERE: SPEC.md §4 (\\\"`E0063`s all 16 … constructors\\\")
CLAIM: Retyping a field emits **E0308** (mismatched types), not E0063 (missing field). E0063 is the *adding-a-field* mechanism from `CLAUDE.md`, borrowed for a different operation.
CONSEQUENCE: The guarantee is unaffected — every literal supplying a `Usd` still errors — but the spec cites a mechanism it does not use, and a reader checking the claim against the doctrine will not find it.
EVIDENCE: Verified with `rustc --edition 2021 --crate-type lib` on a two-field reduction: `pub struct Lines { pub line1: LineEntry }` + `Lines { line1: v }` where `v: i64` → `error[E0308]: mismatched types`. No E0063 is produced.

SEVERITY: Minor
WHERE: `crates/btctax-core/src/tax/printed.rs:53` (the only `Default` on a printed struct)
CLAIM: `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub struct Printed8949Totals` is the one `Default` escape on the printed layer, and it fails closed — but the spec does not name it, and the tempting repair is wrong.
CONSEQUENCE: The implementer meets an unexplained `E0277` and may repair it by adding `impl Default for LineEntry` (which is *safe* if it yields `Blank`, and a re-fabrication if it ever yields `Entered(ZERO)`) rather than deleting the derive. §2.2 says \\\"No `Default`\\\" without saying which side of that line the danger is on.
EVIDENCE: Verified: `#[derive(Default)] pub struct Totals { pub a: LineEntry }` with `LineEntry: !Default` → `error[E0277]: the trait bound 'LineEntry: Default' is not satisfied`. Elsewhere the codebase is clean: no `serde` derive, no `impl From`, and no struct-update (`..other`) on any printed struct — the only `..Default::default()` in `printed.rs` (lines 2923/2928/2960/2966) are on `ReturnInputs`/`Form1099Int` in tests. §4's escape hunt otherwise comes back empty.

SEVERITY: Minor
WHERE: SPEC.md §5.1 item 1 (\\\"Line set enumerated from the committed map TOMLs\\\")
CLAIM: `extract_lines` returns only *present* cells, so the KAT needs a new map-line enumerator the spec does not name — and the map TOMLs are no longer \\\"line → field\\\".
CONSEQUENCE: Buildable, but the obvious enumerator needs a skip-list for `form` / `year` / `da_present` / `table_token` / `[identity]` / `[census]`, which is the exact rot `transcribe.rs` says it avoided. Also \\\"one line, one key\\\" does not hold: `MoneyPair` lines transcribe as `line2.dollars_field` + `line2.cents_field` (2017 forms), and Schedule D amount rows as `line3.proceeds_d` / `.cost_e` / `.adj_g` / `.gain_h`.
EVIDENCE: `crates/btctax-forms/src/transcribe.rs:40` returns only cells where the field exists *and* `!v.trim().is_empty()`. `forms/2024/f1040s3.map.toml` now carries a `[census]` table of ~30 FQN keys with `{ line, rule, reason }` values. The safe predicate is the one `walk` already uses — keep a string leaf **only if it is a real field in `collect_fields`** — which drops all census metadata cleanly and needs no skip-list; the spec should say so, because the naive version does not.

SEVERITY: Minor
WHERE: SPEC.md §9 (\\\"Map coverage is unverified … P0 must add a writes-set census\\\")
CLAIM: This risk is already half-closed by an existing gate the spec does not credit.
CONSEQUENCE: P0 scope is smaller than stated, and the census should be built on the existing invariant rather than invented beside it (a second writes-set would be a second truth).
EVIDENCE: `crates/btctax-forms/src/verify.rs:229` `no_unmapped_filled` → `assert_only_filled`: \\\"every filled field must be in `allowed`\\\", where `allowed` is the placement set and every placement comes from a map cell (`cells.rs:55/61/85`). A cell written outside any map already fails closed as `FormsError::UnmappedField`.

SEVERITY: Minor
WHERE: SPEC.md §5 (\\\"It **must never use `Blank::AbsentIsZero`**\\\")
CLAIM: The prohibition is correct for the new KAT but the spec does not require auditing the one *existing* real use, which will silently stay green through P1/P2.
CONSEQUENCE: When P1/P2 start blanking cells, `golden_packet.rs` keeps passing by construction on the line most likely to move — reinforcing \\\"no golden moves\\\" in the one place a golden could have spoken.
EVIDENCE: `crates/btctax-forms/tests/golden_packet.rs:321` `usd(cell_or_zero(&f1040, \\\"line13\\\", Blank::AbsentIsZero) as f64)` — 1040 L13 is the QBI line, which P1 flips to absent when there is no 8995 (`printed.rs:654` `let line13 = f8995.map_or(Usd::ZERO, |q| q.line15);`).

SEVERITY: Nit
WHERE: SPEC.md §9 (\\\"~47 `push_money` sites and 16 constructors\\\")
CLAIM: The write-side figure is close; the constructor-side figure is off by ~5×.
CONSEQUENCE: P1/P2 planning underestimates the mechanical edit count. All of the excess is in tests, so it is volume, not risk.
EVIDENCE: Money-write sites in `btctax-forms/src`: 31 `push_money` + 3 `push_literal` + 10 direct `fmt_money` (`schedule_d.rs` 9, `form8275.rs` 1) = **44**. Struct-literal sites for the 16 printed types across the workspace, comments excluded: **87** — 46 in `printed.rs`, 23 in `tests/full_return_forms.rs`, 10 in `tests/sp4.rs`, 6 in `tests/extract_lines.rs`, 2 in `tests/census.rs`. There are 13 constructor `fn`s, not 16.

---

## ALSO CHECKED, SOUND:

- **§7 P0's \\\"ZERO BEHAVIOUR CHANGE ON PAPER\\\" traces cleanly.** `push_money` (`cells.rs:44-69`) is a pure formatter-and-push with no state; adding `match e { Blank => return, Entered(v) => v }` at the top, with every call site passing `Entered(x)`, produces an identical `writes` vector and an identical `placements` vector. `push_money` is crate-internal (`cells.rs` is not re-exported through `testonly`; only `fmt_money_pair` is), so the signature change cannot reach another crate. The TY2024 golden matrix md5 `c4e1853ed82d113ca5cd97ffd8abbf47` would hold.
- **The suppression is geometrically safe.** `verify.rs:390-414` groups placements by descent group, sorts by ordinal, and asserts strictly-decreasing centre-y across `windows(2)`. It requires **strict descent, not contiguity** — so removing a cell from the middle of a group leaves the remaining sequence valid. Skipping blank lines will not red the geometric oracle in P1/P2. (`schedule_se_full.rs:80-92` already ships a `bool` include-flag per line for exactly this, and it works.)
- **§5's core premise is true and already tested.** `transcribe.rs:65-72`: a cell is inserted only when the field exists in the PDF *and* its text is non-empty; `:75-84`: a checkbox appears only when on. `tests/extract_lines.rs` already asserts absences (`!got.contains_key(\\\"da_no\\\")`, `!got.contains_key(\\\"part1_rows[2].payer\\\")`, `!got.contains_key(\\\"form\\\")`). \\\"An absent key is a blank cell on paper\\\" holds.
- **§5's `PresentZero` instrument already exists** (`tests/common/mod.rs:201`) and panics on an absent key — it is exactly the right tool for the instructed-zero vector, and for the missing third vector above.
- **§4's escape hunt is otherwise clean.** No `serde` derive on any printed struct or on `PrintedReturn`/`PrintedForms`; no `impl From` on printed types; no builder; no struct-update syntax anywhere on a printed struct (the only two `..ident` uses in the workspace are `reconcile.rs:1337 ..prior` and `persist.rs:1118 ..prior_alloc`, neither a printed type). Retyping really does error every literal.
- **The emitters are pass-throughs.** `form1040_full.rs:110-198` builds `[(cell, value)]` arrays and loops; the `LineEntry` migration there is mechanical with no logic change.
- **`design/forms/extract/*.txt` supports §5.2's computed instructed-zero set.** `grep -c -- \\\"-0-\\\"` returns non-zero on exactly the forms that should have it (f1040 2, f1040sd 1, f8959 6, f8960 4, f8995 5, f6251 7, f1040sse 3, f1040s1a 6) and zero on those that should not (f8949, f1040sb, f1040sc, f1040s2, f1040s3, f8283, f8275). The mechanism is greppable as described.
- **`repo_hygiene.rs` is a real precedent** (`crates/btctax-cli/tests/repo_hygiene.rs`), including a published-crate enumeration and a fail-closed posture — §6's hygiene test can be built on it.

## WHAT WOULD MAKE THIS REVIEW WRONG:

- **If §7's P1/P2 are meant to enumerate *sites*, not *fields*** — i.e. if \\\"the 15 flattened `Option`s\\\" is shorthand for \\\"retype everything, and here are the 15 whose *values* change\\\" — then Important 2 dissolves into a wording fix. I read them as scoped subsets because P0 explicitly says \\\"the *fields* do not migrate yet\\\", which implies a later field migration that is then never named. One sentence in §7 settles it either way.
- **If `MAP-survey.md`'s 168/64 already spans `other_taxes.rs`/`qbi.rs`/`form6251.rs`** (I did not re-derive the counts — the brief marks them settled), then Important 1 is a §2.2/§4 *scope-statement* defect rather than a coverage gap, and drops to Minor. The counts being right does not make the scope sentence right, but it changes what is at risk.
- **If the opaque-newtype containment collides with something I did not test** — I verified the compiler behaviours in isolation (`/tmp/lechk/t.rs`), not against a migrated `printed.rs`. If `LineEntry` must be `Copy` + `Eq` + usable in `const` contexts somewhere I did not find, the newtype may cost more than I priced it at. I found no such requirement: every printed struct that is `Copy` holds only `Usd`/`bool`/enums, and a newtype over a private enum is `Copy` when its payload is.
- **I did not run the suite.** All byte-identity reasoning is from reading `cells.rs`, `verify.rs`, and the emitters; a `make check` at P0 is the actual proof.\"}],\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"usage\":{\"input_tokens\":2,\"cache_creation_input_tokens\":4592,\"cache_read_input_tokens\":132548,\"output_tokens\":7559,\"server_tool_use\":{\"web_search_requests\":0,\"web_fetch_requests\":0},\"service_tier\":\"standard\",\"cache_creation\":{\"ephemeral_1h_input_tokens\":0,\"ephemeral_5m_input_tokens\":4592},\"inference_geo\":\"not_available\",\"iterations\":[{\"input_tokens\":2,\"output_tokens\":7559,\"cache_read_input_tokens\":132548,\"cache_creation_input_tokens\":4592,\"cache_creation\":{\"ephemeral_5m_input_tokens\":4592,\"ephemeral_1h_input_tokens\":0},\"type\":\"message\"}],\"speed\":\"standard\"},\"diagnostics\":null},\"requestId\":\"req_011CdbQDE9bP1dvrh8ppj8E2\",\"attributionAgent\":\"general-purpose\",\"type\":\"assistant\",\"uuid\":\"8e388029-4e97-479e-add2-3ce3439e4788\",\"timestamp\":\"2026-08-01T04:32:43.707Z\",\"effort\":\"xhigh\",\"userType\":\"external\",\"entrypoint\":\"cli\",\"cwd\":\"/scratch/code/bitcoin_tax\",\"sessionId\":\"2b9d8116-e1de-4038-a101-5fd575fdcb0e\",\"version\":\"2.1.220\",\"gitBranch\":\"main\",\"slug\":\"fuzzy-booping-minsky\"}","is_error":false}]},"uuid":"477a7991-981d-462d-8347-3d8e9a5c0c57","timestamp":"2026-08-01T04:50:59.313Z","toolUseResult":{"stdout":"{\"parentUuid\":\"fc2e3ea5-9177-4158-9106-d4c46a5c4ab8\",\"isSidechain\":true,\"agentId\":\"a84b7406ef53120af\",\"message\":{\"model\":\"claude-opus-5\",\"id\":\"msg_011CdbQDHocSwbjTeUx9AGFd\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"VERDICT: 0 Critical / 4 Important

---

SEVERITY: Important
WHERE: SPEC.md §2.2 + §4 (scope of \\\"the printed layer\\\")
CLAIM: Three of the emitted forms' printed-line structs — and Form 6251, the spec's own headline example — do not live in `printed.rs`, so \\\"every money field on every printed struct\\\" and \\\"all 16 printed-struct constructors\\\" scope the migration to one of four files that hold money reaching paper.
CONSEQUENCE: §2.2's stated benefit (\\\"Tier-2 Form 6251 fields land safe by default: `line2i` … cannot ship as a silent zero if it must choose a constructor\\\") is not delivered by anything the spec scopes. Form 8959/8960/8995/6251 lines stay bare `Usd`, and every one of them remains exactly the \\\"65th site\\\" §4 promises cannot exist.
EVIDENCE: Spec: \\\"`LineEntry` — the printed layer (`btctax-core/src/tax/printed.rs`)\\\" and \\\"retyping the fields `E0063`s all **16** printed-struct constructors\\\". `printed.rs` contains exactly 16 printed structs — that is where the 16 comes from. But the structs the emitters actually consume also include:
- `crates/btctax-core/src/tax/other_taxes.rs:96` `pub struct Form8959Lines`, `:257` `pub struct Form8960Lines`
- `crates/btctax-core/src/tax/qbi.rs:171` `pub struct Form8995Lines`
- `crates/btctax-core/src/tax/form6251.rs:89` `pub struct Form6251`
- `crates/btctax-core/src/forms.rs:171/185` `ScheduleDPart` / `ScheduleDTotals` — the crypto-slice Schedule D fill (`schedule_d.rs:60`) takes `&ScheduleDTotals`, not a printed struct, and never calls `push_money` at all.
Line-read counts confirm the mass: `form6251.rs` 115, `other_taxes.rs` 79, `qbi.rs` 58. The spec knows the surface is wider (P1 names `fold.rs:1151` and \\\"Schedule 2 L12\\\"), but §2.2/§4's scope statement and the \\\"16\\\" do not.

---

SEVERITY: Important
WHERE: SPEC.md §7 (phasing) vs §2.2 (\\\"uniformly\\\") and §4 (the guarantee)
CLAIM: No phase performs the uniform retyping that §2.2 mandates and on which §4's guarantee depends; P0–P4 together name roughly 58 of the 168 money quantities.
CONSEQUENCE: The printed surface stays a mixed `Usd`/`LineEntry` patchwork — which §2.2 itself identifies as the thing that \\\"produced the 64 in the first place\\\" — and §4's \\\"a 65th site cannot choose a silent zero\\\" is simply false for every field still typed `Usd`. The program degrades from \\\"two types, not 64 fixes\\\" back to \\\"64 fixes, plus a type.\\\" No exit criterion in any phase would catch this.
EVIDENCE: §2.2: \\\"**Every money field on every printed struct becomes `LineEntry`, uniformly** — including lines that are always entered.\\\" §7: P0 = \\\"the *types* land, the *fields* do not migrate yet\\\"; P1 = \\\"The 15 flattened `Option`s … plus 8995 lines 3/7\\\"; P2 = \\\"the 14 `#[serde(default)] Usd` scalars; the 20 empty-`Vec` sums and 9 blank-column totals\\\"; P3 = the classifier; P4 = residue. 15 + 2 + 14 + 20 + 9 = 60. The ~110 always-entered fields are never scheduled. This is an omission, not an impossibility: a uniform-retyping phase is byte-unchanged on paper (all `Entered`), so it satisfies the same green bar P0 does.

---

SEVERITY: Important
WHERE: SPEC.md §6 (the containment) — `crates/btctax-cli/src/render.rs:1475-1570`, `crates/btctax-forms/src/schedule_se_full.rs:78`, `form8995.rs:166`, `schedule_d_full.rs:61`, `crates/btctax-oracle-harness/Cargo.toml`
CLAIM: §6's leak inventory (\\\"`golden_packet.rs` and `btctax-oracle-harness`\\\") omits the largest consumer — production code in a *published* crate — and the grep it specifies bans the honest accessor precisely where it is needed while exempting the one crate the spec names.
CONSEQUENCE: The hygiene gate fires on ~25 real production sites and offers them no legal exit, so the author is pushed into the un-greppable inline `match … Blank => Usd::ZERO` that the brief identifies as the failure mode. The gate then reports success while the guarantee leaks. Meanwhile `btctax-oracle-harness` — which the spec explicitly says needs the accessor — is `publish = false`, so the grep as scoped never looks at it.
EVIDENCE: Spec: \\\"It lives on an extension trait in a **test-support module** … A **repo-hygiene test greps for it** and fails if it appears under `crates/*/src/` for any of the ten published crates.\\\"
- `crates/btctax-cli/src/render.rs:1475` `let f = &printed.f1040;` then `fmt_money(f.line9)`, `f.line10`, `f.line11`, `f.line12`, `f.line13`, `f.line15`, `f.line16`, `f.line24`, `f.line33`, `if f.line34 > Usd::ZERO`, and already `printed.sch_3.map_or(Usd::ZERO, |s3| s3.line1)` — 23 printed-field reads in `btctax-cli/src`, a published crate.
- Production comparisons inside the emitters themselves: `schedule_se_full.rs:78` `let skip_8d_to_10 = lines.line9 == Usd::ZERO;`; `form8995.rs:166` `if lines.line2 > Usd::ZERO`; `schedule_d_full.rs:61` `if v < Usd::ZERO` over `lines.line6` / `lines.line14` / `line21`.
- `crates/btctax-oracle-harness/Cargo.toml` → `publish = false`; `src/main.rs` has 45 printed-field reads including real arithmetic at `:324` `let l24_target = pr.forms.f1040.line16 + se_l12_ots + f8959_l18_ots + round_leaf(e.niit);`.

**Answering the brief's question directly — \\\"greppable and review-loud\\\" is NOT the ceiling.** Three strictly stronger mechanisms are available today:

1. **Make `LineEntry` opaque** — `pub struct LineEntry(Repr);` with `enum Repr { Entered(Usd), Blank }` private to the module. A `match` on it outside the defining module then does not compile, which is the *only* mechanism that closes the exact hole the brief names. It costs nothing here, because §3 already forbids free construction and §2.2 already forbids `From`/`Add`/`Sum`/`Default`; the combinators become methods and the exits become whatever you choose to publish (`is_blank()`, `entered() -> Option<Usd>` kept crate-private, an `Option<Ordering>` comparator for the three emitter sites above).
2. **`#[cfg(feature = \\\"test-support\\\")]` the accessor**, not merely a `testonly` module. The repo's existing precedent is *ungated* — `crates/btctax-forms/src/lib.rs:423` is a plain `pub mod testonly` — so \\\"an obviously test-shaped import\\\" is convention only. A feature gate makes a production call a **compile error**. ★ Caveat the spec must record: `btctax-oracle-harness` depends on core/forms as a *normal* dependency, so it cannot enable that feature without workspace feature unification re-exposing the accessor everywhere (`resolver = \\\"2\\\"` does not help). The harness must therefore write explicit matches — which is the honest answer anyway, and should be specified rather than discovered.
3. **`clippy.toml` `disallowed-methods`** — the repo has no `clippy.toml` today, and `make check` already runs `clippy --workspace --all-targets -- -D warnings` (`Makefile:27`). That converts the grep into a hard build failure that can only be overridden by a visible `#[allow]`.
   *Not* `#[non_exhaustive]`: it forces a `_` arm on downstream matches, making `_ => Usd::ZERO` **easier** to write. It would make this worse.

---

SEVERITY: Important
WHERE: SPEC.md §5.1 (the two B1 plants)
CLAIM: The two plants cover under-suppression and over-suppression-of-an-instructed-zero, but miss a third direction: **a filer-stated zero suppressed** — and that direction is the architecture's own headline distinction.
CONSEQUENCE: A `collected` implemented as `match x { Some(v) if !v.is_zero() => Entered(v), _ => Blank }` compiles, is a plausible thing to write, silently deletes genuine testimony from the return, and **passes both plants**. This is an over-firing suppression, which §5's own preamble calls \\\"the invisible one.\\\"
EVIDENCE: Spec §5.1's table: plant 1 reds \\\"the **absence** vector\\\"; plant 2 reds \\\"the **instructed-zero** vector\\\". The absence vector is defined as \\\"no Schedule 1, no W-2s, no carryovers\\\" — nothing is *stated*, so `Stated(Usd::ZERO)` is never exercised. The instructed-zero vector's zeros arrive via `computed` (§3: \\\"All 24 form-instructed zeros live in `computed`\\\"), not via `collected`. Yet §2.1's entire justification for `Collected<T>` over `Option<Usd>` is that `Stated(0)` and `NotAsked` must not collapse — and no fixture distinguishes them. Fix is cheap and uses machinery that already exists: a third fixture whose filer states `$0` estimated payments / `$0` withholding, asserting `Blank::PresentZero` (`crates/btctax-forms/tests/common/mod.rs:201`) on 1040 L25a/L26.

---

SEVERITY: Minor
WHERE: SPEC.md §4 (\\\"`E0063`s all 16 … constructors\\\")
CLAIM: Retyping a field emits **E0308** (mismatched types), not E0063 (missing field). E0063 is the *adding-a-field* mechanism from `CLAUDE.md`, borrowed for a different operation.
CONSEQUENCE: The guarantee is unaffected — every literal supplying a `Usd` still errors — but the spec cites a mechanism it does not use, and a reader checking the claim against the doctrine will not find it.
EVIDENCE: Verified with `rustc --edition 2021 --crate-type lib` on a two-field reduction: `pub struct Lines { pub line1: LineEntry }` + `Lines { line1: v }` where `v: i64` → `error[E0308]: mismatched types`. No E0063 is produced.

SEVERITY: Minor
WHERE: `crates/btctax-core/src/tax/printed.rs:53` (the only `Default` on a printed struct)
CLAIM: `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub struct Printed8949Totals` is the one `Default` escape on the printed layer, and it fails closed — but the spec does not name it, and the tempting repair is wrong.
CONSEQUENCE: The implementer meets an unexplained `E0277` and may repair it by adding `impl Default for LineEntry` (which is *safe* if it yields `Blank`, and a re-fabrication if it ever yields `Entered(ZERO)`) rather than deleting the derive. §2.2 says \\\"No `Default`\\\" without saying which side of that line the danger is on.
EVIDENCE: Verified: `#[derive(Default)] pub struct Totals { pub a: LineEntry }` with `LineEntry: !Default` → `error[E0277]: the trait bound 'LineEntry: Default' is not satisfied`. Elsewhere the codebase is clean: no `serde` derive, no `impl From`, and no struct-update (`..other`) on any printed struct — the only `..Default::default()` in `printed.rs` (lines 2923/2928/2960/2966) are on `ReturnInputs`/`Form1099Int` in tests. §4's escape hunt otherwise comes back empty.

SEVERITY: Minor
WHERE: SPEC.md §5.1 item 1 (\\\"Line set enumerated from the committed map TOMLs\\\")
CLAIM: `extract_lines` returns only *present* cells, so the KAT needs a new map-line enumerator the spec does not name — and the map TOMLs are no longer \\\"line → field\\\".
CONSEQUENCE: Buildable, but the obvious enumerator needs a skip-list for `form` / `year` / `da_present` / `table_token` / `[identity]` / `[census]`, which is the exact rot `transcribe.rs` says it avoided. Also \\\"one line, one key\\\" does not hold: `MoneyPair` lines transcribe as `line2.dollars_field` + `line2.cents_field` (2017 forms), and Schedule D amount rows as `line3.proceeds_d` / `.cost_e` / `.adj_g` / `.gain_h`.
EVIDENCE: `crates/btctax-forms/src/transcribe.rs:40` returns only cells where the field exists *and* `!v.trim().is_empty()`. `forms/2024/f1040s3.map.toml` now carries a `[census]` table of ~30 FQN keys with `{ line, rule, reason }` values. The safe predicate is the one `walk` already uses — keep a string leaf **only if it is a real field in `collect_fields`** — which drops all census metadata cleanly and needs no skip-list; the spec should say so, because the naive version does not.

SEVERITY: Minor
WHERE: SPEC.md §9 (\\\"Map coverage is unverified … P0 must add a writes-set census\\\")
CLAIM: This risk is already half-closed by an existing gate the spec does not credit.
CONSEQUENCE: P0 scope is smaller than stated, and the census should be built on the existing invariant rather than invented beside it (a second writes-set would be a second truth).
EVIDENCE: `crates/btctax-forms/src/verify.rs:229` `no_unmapped_filled` → `assert_only_filled`: \\\"every filled field must be in `allowed`\\\", where `allowed` is the placement set and every placement comes from a map cell (`cells.rs:55/61/85`). A cell written outside any map already fails closed as `FormsError::UnmappedField`.

SEVERITY: Minor
WHERE: SPEC.md §5 (\\\"It **must never use `Blank::AbsentIsZero`**\\\")
CLAIM: The prohibition is correct for the new KAT but the spec does not require auditing the one *existing* real use, which will silently stay green through P1/P2.
CONSEQUENCE: When P1/P2 start blanking cells, `golden_packet.rs` keeps passing by construction on the line most likely to move — reinforcing \\\"no golden moves\\\" in the one place a golden could have spoken.
EVIDENCE: `crates/btctax-forms/tests/golden_packet.rs:321` `usd(cell_or_zero(&f1040, \\\"line13\\\", Blank::AbsentIsZero) as f64)` — 1040 L13 is the QBI line, which P1 flips to absent when there is no 8995 (`printed.rs:654` `let line13 = f8995.map_or(Usd::ZERO, |q| q.line15);`).

SEVERITY: Nit
WHERE: SPEC.md §9 (\\\"~47 `push_money` sites and 16 constructors\\\")
CLAIM: The write-side figure is close; the constructor-side figure is off by ~5×.
CONSEQUENCE: P1/P2 planning underestimates the mechanical edit count. All of the excess is in tests, so it is volume, not risk.
EVIDENCE: Money-write sites in `btctax-forms/src`: 31 `push_money` + 3 `push_literal` + 10 direct `fmt_money` (`schedule_d.rs` 9, `form8275.rs` 1) = **44**. Struct-literal sites for the 16 printed types across the workspace, comments excluded: **87** — 46 in `printed.rs`, 23 in `tests/full_return_forms.rs`, 10 in `tests/sp4.rs`, 6 in `tests/extract_lines.rs`, 2 in `tests/census.rs`. There are 13 constructor `fn`s, not 16.

---

## ALSO CHECKED, SOUND:

- **§7 P0's \\\"ZERO BEHAVIOUR CHANGE ON PAPER\\\" traces cleanly.** `push_money` (`cells.rs:44-69`) is a pure formatter-and-push with no state; adding `match e { Blank => return, Entered(v) => v }` at the top, with every call site passing `Entered(x)`, produces an identical `writes` vector and an identical `placements` vector. `push_money` is crate-internal (`cells.rs` is not re-exported through `testonly`; only `fmt_money_pair` is), so the signature change cannot reach another crate. The TY2024 golden matrix md5 `c4e1853ed82d113ca5cd97ffd8abbf47` would hold.
- **The suppression is geometrically safe.** `verify.rs:390-414` groups placements by descent group, sorts by ordinal, and asserts strictly-decreasing centre-y across `windows(2)`. It requires **strict descent, not contiguity** — so removing a cell from the middle of a group leaves the remaining sequence valid. Skipping blank lines will not red the geometric oracle in P1/P2. (`schedule_se_full.rs:80-92` already ships a `bool` include-flag per line for exactly this, and it works.)
- **§5's core premise is true and already tested.** `transcribe.rs:65-72`: a cell is inserted only when the field exists in the PDF *and* its text is non-empty; `:75-84`: a checkbox appears only when on. `tests/extract_lines.rs` already asserts absences (`!got.contains_key(\\\"da_no\\\")`, `!got.contains_key(\\\"part1_rows[2].payer\\\")`, `!got.contains_key(\\\"form\\\")`). \\\"An absent key is a blank cell on paper\\\" holds.
- **§5's `PresentZero` instrument already exists** (`tests/common/mod.rs:201`) and panics on an absent key — it is exactly the right tool for the instructed-zero vector, and for the missing third vector above.
- **§4's escape hunt is otherwise clean.** No `serde` derive on any printed struct or on `PrintedReturn`/`PrintedForms`; no `impl From` on printed types; no builder; no struct-update syntax anywhere on a printed struct (the only two `..ident` uses in the workspace are `reconcile.rs:1337 ..prior` and `persist.rs:1118 ..prior_alloc`, neither a printed type). Retyping really does error every literal.
- **The emitters are pass-throughs.** `form1040_full.rs:110-198` builds `[(cell, value)]` arrays and loops; the `LineEntry` migration there is mechanical with no logic change.
- **`design/forms/extract/*.txt` supports §5.2's computed instructed-zero set.** `grep -c -- \\\"-0-\\\"` returns non-zero on exactly the forms that should have it (f1040 2, f1040sd 1, f8959 6, f8960 4, f8995 5, f6251 7, f1040sse 3, f1040s1a 6) and zero on those that should not (f8949, f1040sb, f1040sc, f1040s2, f1040s3, f8283, f8275). The mechanism is greppable as described.
- **`repo_hygiene.rs` is a real precedent** (`crates/btctax-cli/tests/repo_hygiene.rs`), including a published-crate enumeration and a fail-closed posture — §6's hygiene test can be built on it.

## WHAT WOULD MAKE THIS REVIEW WRONG:

- **If §7's P1/P2 are meant to enumerate *sites*, not *fields*** — i.e. if \\\"the 15 flattened `Option`s\\\" is shorthand for \\\"retype everything, and here are the 15 whose *values* change\\\" — then Important 2 dissolves into a wording fix. I read them as scoped subsets because P0 explicitly says \\\"the *fields* do not migrate yet\\\", which implies a later field migration that is then never named. One sentence in §7 settles it either way.
- **If `MAP-survey.md`'s 168/64 already spans `other_taxes.rs`/`qbi.rs`/`form6251.rs`** (I did not re-derive the counts — the brief marks them settled), then Important 1 is a §2.2/§4 *scope-statement* defect rather than a coverage gap, and drops to Minor. The counts being right does not make the scope sentence right, but it changes what is at risk.
- **If the opaque-newtype containment collides with something I did not test** — I verified the compiler behaviours in isolation (`/tmp/lechk/t.rs`), not against a migrated `printed.rs`. If `LineEntry` must be `Copy` + `Eq` + usable in `const` contexts somewhere I did not find, the newtype may cost more than I priced it at. I found no such requirement: every printed struct that is `Copy` holds only `Usd`/`bool`/enums, and a newtype over a private enum is `Copy` when its payload is.
- **I did not run the suite.** All byte-identity reasoning is from reading `cells.rs`, `verify.rs`, and the emitters; a `make check` at P0 is the actual proof.\"}],\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"usage\":{\"input_tokens\":2,\"cache_creation_input_tokens\":4592,\"cache_read_input_tokens\":132548,\"output_tokens\":7559,\"server_tool_use\":{\"web_search_requests\":0,\"web_fetch_requests\":0},\"service_tier\":\"standard\",\"cache_creation\":{\"ephemeral_1h_input_tokens\":0,\"ephemeral_5m_input_tokens\":4592},\"inference_geo\":\"not_available\",\"iterations\":[{\"input_tokens\":2,\"output_tokens\":7559,\"cache_read_input_tokens\":132548,\"cache_creation_input_tokens\":4592,\"cache_creation\":{\"ephemeral_5m_input_tokens\":4592,\"ephemeral_1h_input_tokens\":0},\"type\":\"message\"}],\"speed\":\"standard\"},\"diagnostics\":null},\"requestId\":\"req_011CdbQDE9bP1dvrh8ppj8E2\",\"attributionAgent\":\"general-purpose\",\"type\":\"assistant\",\"uuid\":\"8e388029-4e97-479e-add2-3ce3439e4788\",\"timestamp\":\"2026-08-01T04:32:43.707Z\",\"effort\":\"xhigh\",\"userType\":\"external\",\"entrypoint\":\"cli\",\"cwd\":\"/scratch/code/bitcoin_tax\",\"sessionId\":\"2b9d8116-e1de-4038-a101-5fd575fdcb0e\",\"version\":\"2.1.220\",\"gitBranch\":\"main\",\"slug\":\"fuzzy-booping-minsky\"}","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"0b79fbc8-85b2-43fa-924f-2cff098bfc79","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"477a7991-981d-462d-8347-3d8e9a5c0c57","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRjJC8Vb9UcLKNtZvii","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISiwUKhwEIEBgCKkApYo0p6RkhsxvN/491nlvOy1UZL3Oq9awfJddS4W7ojOIZ7TOTYH5CyHbjahVjS2AXHG6g90nbgDVZkNIPyJ5NMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDNVVbLSvsEBnAFCrVRoM59qCOY6RkgX+bvcHIjCm6DTonq0iPI8S/1J6abOf9tCE2oOEydysSOG+Rona14HZ4sHSNdwEdoMscQy6rAEqsANDciQZ6Ft08efM5Ogok3WIF5Tn1tE+303GH1T9xZfe+zJuD+1m9hwmIMRt4cJhOBPm1fpAlT3MJRxA2RfbV2D6IB98eXw4nKzWbjJGpdLoDmIU1OZ4W58OzV2FRyXuJvBGgY7SrC6x3XvVWANHMdgbhWCJmMXNE4/GG8bSwfNr4z5Y+yf5cjKbAtzl8+63qMo8xts4Mx4UCHjgScOIn1YBJVJKVTXoLssGJqPLqaYq064RicoNhFepmDPf7nccpz2Kwqx8QWJHZhBobBBTgosIcMtahkqcV9QMjbZ0RZsYSeRulxAcqfGSayglx1cl4XHE4nvxHjOtXAeFqle91R1javzzGL/LQad0CHxO9B3LWf7C9k6wk1ssJRGcPDrJMmH7gye1Ke/7iib/6z34MUCKH17EPKZCwf6L6yMHPlB2j1esAzId0FwkuCZ1O6thty1tb3EIx++dGVJVAY4Bw6NCb8Gp15Oc4WukAnGzZJ787hHBdhwc1bBEvyo2eNNmCdZzYnE3qutwOucaZFInQ1hN3saYzCH2Yxhhbxya8c0FmL4unzlO/wFZr5h7nIL1LhYYAQ=="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":8355,"cache_read_input_tokens":45268,"cache_creation":{"ephemeral_5m_input_tokens":8355,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRjHAcuQf8CTupvNxo5","attributionAgent":"general-purpose","type":"assistant","uuid":"93e7e2c5-3af4-410a-9bd6-93badaa98a1f","timestamp":"2026-08-01T04:51:02.592Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"93e7e2c5-3af4-410a-9bd6-93badaa98a1f","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRjJC8Vb9UcLKNtZvii","type":"message","role":"assistant","content":[{"type":"text","text":"Now let me examine the real code. Starting with the derives on every struct in scope."}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":8355,"cache_read_input_tokens":45268,"cache_creation":{"ephemeral_5m_input_tokens":8355,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRjHAcuQf8CTupvNxo5","attributionAgent":"general-purpose","type":"assistant","uuid":"4305a80b-cdf1-43d7-b1b6-9365c6485429","timestamp":"2026-08-01T04:51:03.005Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"4305a80b-cdf1-43d7-b1b6-9365c6485429","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRjJC8Vb9UcLKNtZvii","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01BqVjFffZBSLqnmihPrm6ax","name":"Bash","input":{"command":"grep -n \"^#\\[derive\\|^pub struct\\|^struct \" crates/btctax-core/src/tax/printed.rs | head -80"},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":8355,"cache_read_input_tokens":45268,"cache_creation":{"ephemeral_5m_input_tokens":8355,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRjHAcuQf8CTupvNxo5","attributionAgent":"general-purpose","type":"assistant","uuid":"0b991d4b-e734-4d6b-acea-137d9b8ffa60","timestamp":"2026-08-01T04:51:04.472Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"0b991d4b-e734-4d6b-acea-137d9b8ffa60","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01BqVjFffZBSLqnmihPrm6ax","type":"tool_result","content":"36:#[derive(Debug, Clone, PartialEq, Eq)]
37:pub struct Printed8949Row {
53:#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
54:pub struct Printed8949Totals {
65:#[derive(Debug, Clone, PartialEq, Eq)]
66:pub struct Printed8949 {
134:#[derive(Debug, Clone, PartialEq, Eq)]
135:pub struct Printed8283Rows {
196:#[derive(Debug, Clone, PartialEq, Eq)]
197:pub struct Printed8275 {
232:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
233:pub struct ScheduleSeLines {
315:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
316:pub struct Schedule2Lines {
368:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
369:pub struct Schedule1Lines {
447:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
448:pub struct Form1040Lines {
544:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
545:pub struct Form1040Income {
743:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
777:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
778:pub struct ScheduleDLines {
921:#[derive(Debug, Clone, PartialEq, Eq)]
922:pub struct ScheduleBRow {
952:#[derive(Debug, Clone, PartialEq, Eq)]
953:pub struct ScheduleBLines {
1071:#[derive(Debug, Clone, PartialEq, Eq)]
1072:pub struct ScheduleCLines {
1155:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
1156:pub struct ScheduleALines {
1300:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
1301:pub struct Schedule3Lines {","is_error":false}]},"uuid":"939a3981-898c-4ff9-9f6c-5fbc002b5b21","timestamp":"2026-08-01T04:51:04.557Z","toolUseResult":{"stdout":"36:#[derive(Debug, Clone, PartialEq, Eq)]
37:pub struct Printed8949Row {
53:#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
54:pub struct Printed8949Totals {
65:#[derive(Debug, Clone, PartialEq, Eq)]
66:pub struct Printed8949 {
134:#[derive(Debug, Clone, PartialEq, Eq)]
135:pub struct Printed8283Rows {
196:#[derive(Debug, Clone, PartialEq, Eq)]
197:pub struct Printed8275 {
232:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
233:pub struct ScheduleSeLines {
315:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
316:pub struct Schedule2Lines {
368:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
369:pub struct Schedule1Lines {
447:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
448:pub struct Form1040Lines {
544:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
545:pub struct Form1040Income {
743:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
777:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
778:pub struct ScheduleDLines {
921:#[derive(Debug, Clone, PartialEq, Eq)]
922:pub struct ScheduleBRow {
952:#[derive(Debug, Clone, PartialEq, Eq)]
953:pub struct ScheduleBLines {
1071:#[derive(Debug, Clone, PartialEq, Eq)]
1072:pub struct ScheduleCLines {
1155:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
1156:pub struct ScheduleALines {
1300:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
1301:pub struct Schedule3Lines {","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"0b991d4b-e734-4d6b-acea-137d9b8ffa60","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"939a3981-898c-4ff9-9f6c-5fbc002b5b21","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRjJC8Vb9UcLKNtZvii","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01TGkK62EW3Af2QdHj8pjxo4","name":"Bash","input":{"command":"grep -n \"derive\" crates/btctax-core/src/tax/other_taxes.rs crates/btctax-core/src/tax/qbi.rs crates/btctax-core/src/tax/form6251.rs crates/btctax-core/src/forms.rs | head -60"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":8355,"cache_read_input_tokens":45268,"output_tokens":439,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":8355},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":439,"cache_read_input_tokens":45268,"cache_creation_input_tokens":8355,"cache_creation":{"ephemeral_5m_input_tokens":8355,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRjHAcuQf8CTupvNxo5","attributionAgent":"general-purpose","type":"assistant","uuid":"9eaca0e7-1d44-406a-bfe4-7045c83d558d","timestamp":"2026-08-01T04:51:05.008Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"9eaca0e7-1d44-406a-bfe4-7045c83d558d","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01TGkK62EW3Af2QdHj8pjxo4","type":"tool_result","content":"crates/btctax-core/src/tax/other_taxes.rs:36:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/tax/other_taxes.rs:75:/// PRINT is a silently wrong return, and no core KAT would catch it. So the chain is derived HERE
crates/btctax-core/src/tax/other_taxes.rs:95:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/tax/other_taxes.rs:208:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/tax/other_taxes.rs:246:/// [`Form8959Lines`] for why the chain is derived in core and merely transcribed by `btctax-forms`.
crates/btctax-core/src/tax/other_taxes.rs:256:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/tax/other_taxes.rs:292:/// same values; `f` supplies nothing the chain re-derives, so it is not taken.
crates/btctax-core/src/tax/other_taxes.rs:514:    /// answer, for every derived line. Non-tautological — each total is re-derived from the OTHER
crates/btctax-core/src/tax/qbi.rs:21:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/tax/qbi.rs:142:/// `other_taxes::Form8959Lines` for why the chain is derived in core and only transcribed by
crates/btctax-core/src/tax/qbi.rs:170:#[derive(Debug, Clone, PartialEq, Eq)]
crates/btctax-core/src/tax/qbi.rs:840:    /// The printed chain cross-foots: every derived line re-derives from the OTHER printed cells.
crates/btctax-core/src/forms.rs:21:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/forms.rs:41:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/forms.rs:59:#[derive(Debug, Clone, PartialEq, Eq)]
crates/btctax-core/src/forms.rs:170:#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
crates/btctax-core/src/forms.rs:184:#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
crates/btctax-core/src/forms.rs:237:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/forms.rs:245:/// The Form 8283 \"How acquired by donor\" category, derived from the leg's `BasisSource` [R0-N2]:
crates/btctax-core/src/forms.rs:252:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/forms.rs:295:/// For Section A, `false` when `DonationDetails` is present. `fmv_method` is derived from the
crates/btctax-core/src/forms.rs:298:#[derive(Debug, Clone, PartialEq, Eq)]
crates/btctax-core/src/forms.rs:324:    /// (non-carrier) leg rows. No `FmvStatus` dependency — derived from the section only.
crates/btctax-core/src/forms.rs:378:/// **`fmv_method` (D3 — honest, section-derived):** Section B → `\"qualified appraisal\"` (a qualified
crates/btctax-core/src/forms.rs:405:    // D3: honest fmv_method — derived from the section only (no FMV provenance on RemovalLeg;
crates/btctax-core/src/tax/form6251.rs:16://! here is derived: the form never asks anyone to derive anything, it says \"enter the amount from\",
crates/btctax-core/src/tax/form6251.rs:45:#[derive(Debug, Clone, PartialEq, Eq)]
crates/btctax-core/src/tax/form6251.rs:87:#[derive(Debug, Clone, PartialEq, Eq, Default)]
crates/btctax-core/src/tax/form6251.rs:264:#[derive(Debug, Clone, Copy)]
crates/btctax-core/src/tax/form6251.rs:499:    /// were derived independently in plan review r1, before any code — so a match here is a real
crates/btctax-core/src/tax/form6251.rs:566:        let der = &v[\"derived\"];","is_error":false}]},"uuid":"65507f40-1591-4330-a5ea-dc93c887f552","timestamp":"2026-08-01T04:51:05.081Z","toolUseResult":{"stdout":"crates/btctax-core/src/tax/other_taxes.rs:36:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/tax/other_taxes.rs:75:/// PRINT is a silently wrong return, and no core KAT would catch it. So the chain is derived HERE
crates/btctax-core/src/tax/other_taxes.rs:95:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/tax/other_taxes.rs:208:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/tax/other_taxes.rs:246:/// [`Form8959Lines`] for why the chain is derived in core and merely transcribed by `btctax-forms`.
crates/btctax-core/src/tax/other_taxes.rs:256:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/tax/other_taxes.rs:292:/// same values; `f` supplies nothing the chain re-derives, so it is not taken.
crates/btctax-core/src/tax/other_taxes.rs:514:    /// answer, for every derived line. Non-tautological — each total is re-derived from the OTHER
crates/btctax-core/src/tax/qbi.rs:21:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/tax/qbi.rs:142:/// `other_taxes::Form8959Lines` for why the chain is derived in core and only transcribed by
crates/btctax-core/src/tax/qbi.rs:170:#[derive(Debug, Clone, PartialEq, Eq)]
crates/btctax-core/src/tax/qbi.rs:840:    /// The printed chain cross-foots: every derived line re-derives from the OTHER printed cells.
crates/btctax-core/src/forms.rs:21:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/forms.rs:41:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/forms.rs:59:#[derive(Debug, Clone, PartialEq, Eq)]
crates/btctax-core/src/forms.rs:170:#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
crates/btctax-core/src/forms.rs:184:#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
crates/btctax-core/src/forms.rs:237:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/forms.rs:245:/// The Form 8283 \"How acquired by donor\" category, derived from the leg's `BasisSource` [R0-N2]:
crates/btctax-core/src/forms.rs:252:#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/btctax-core/src/forms.rs:295:/// For Section A, `false` when `DonationDetails` is present. `fmv_method` is derived from the
crates/btctax-core/src/forms.rs:298:#[derive(Debug, Clone, PartialEq, Eq)]
crates/btctax-core/src/forms.rs:324:    /// (non-carrier) leg rows. No `FmvStatus` dependency — derived from the section only.
crates/btctax-core/src/forms.rs:378:/// **`fmv_method` (D3 — honest, section-derived):** Section B → `\"qualified appraisal\"` (a qualified
crates/btctax-core/src/forms.rs:405:    // D3: honest fmv_method — derived from the section only (no FMV provenance on RemovalLeg;
crates/btctax-core/src/tax/form6251.rs:16://! here is derived: the form never asks anyone to derive anything, it says \"enter the amount from\",
crates/btctax-core/src/tax/form6251.rs:45:#[derive(Debug, Clone, PartialEq, Eq)]
crates/btctax-core/src/tax/form6251.rs:87:#[derive(Debug, Clone, PartialEq, Eq, Default)]
crates/btctax-core/src/tax/form6251.rs:264:#[derive(Debug, Clone, Copy)]
crates/btctax-core/src/tax/form6251.rs:499:    /// were derived independently in plan review r1, before any code — so a match here is a real
crates/btctax-core/src/tax/form6251.rs:566:        let der = &v[\"derived\"];","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"9eaca0e7-1d44-406a-bfe4-7045c83d558d","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"65507f40-1591-4330-a5ea-dc93c887f552","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRjjA4Nb3H6iK2pvyP3","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISrQcKhwEIEBgCKkAAlHu6+MaOLFzbGByFlLRlbTG8FQfrTLEfj7A0UefID/eIwTHI8ysSnNl5pWzoo0yzGO7ahFE3eyadwKyyTEK5Mg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDGBQrI0NkoSmwJDNGxoMYe7S6Sc7Ey0xXyGYIjBEA9+vh2IxbXTK+vK8t7nx6dGsrYoU09L6rN/M8PW6azRrgpojKldUqNhyRJHjBewq0gWK72XpSgdCbKVdI9jBJuKgWMmbrR/f1g/ifIaEd3c6/71aUfhzY/KtKbbCjd/JkjD++eGa91kl3M5Je8a0hITR0Ci6gj4QiWG6vQUFigG5Iu1Ri1mus7htKojT14Xe+J7I8BxYxNku88spMvJpOB4YMXw/SETICEgqDxG9oRooMohm6a66c9YGxIvH18Qn67wpYi34lnvOcbr5y6DVIobEghSOIrTbWsONJRJ5NTtm+9Xo4iSf4FUnXPvDbq/i7a0bc7o5GWYikOJ8eTGZjiN5T/lkCqeW9EmqIsmIwopSuJQxw5S2ChajospvvOUCIiO3wZKlNxsc6hQSchLbbQ8YlLmP/Oz2jKFQPpRuvLUSMpDxvOyjUHhgFCi+Lw2jdrYJJlsfZtTXx005WgkZuwCImn/IQgNLzkYJlI2NBH6y8xbDQEnlyN1g/3ZHoX7yOyCvKqj9sULF4AF3bS3akImzwOnVL9a1c2FNbCqD98VVdV5imnalmvJygp99ifWk7GMpbJLnAdfxP6urya0izcHe22LmHoIT3ya8f+xgOuXDZ8YwBfdVydbrz2t/1HgNKSrJltuTNisTZtBsoeJmjDKGLibBASO2P5ssjMXBi9y95W/k6rMCxIS9kvXXBJTJjAKa3H4y614Vvq7ohRWSFIvM1NqW8REYMuzlVCI/LQmwub7rf5G2jtVyGjv57vssHeDAn2Wzzj8TdM7TQVpWKJsJAzBsmFUSC88vZdLVWLHbQXm80UB8R4ebk+eOAyv05xkBTu/qRFZa851An7UzpTftVOpM4E9mDRu/pMIrXtf0QGuv674VyGlhkqF/QW8EEYzDjTKjWYJDBOBQP3HGNC2Ht2nH22Syg4lO5N0oZt/Dpriu2uMDo+6O7hw/DBOxrgeWf7uSgfBWzPWyQUkPQj4sbdSqOkMuKgeh1yRrKczZZk1xvWpvYpPohxSkmdgovpfKGBgB"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2838,"cache_read_input_tokens":53623,"cache_creation":{"ephemeral_5m_input_tokens":2838,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRjhohYTkdQTaPWKE5L","attributionAgent":"general-purpose","type":"assistant","uuid":"361e809d-319f-49e5-bf22-8c91e2352fc0","timestamp":"2026-08-01T04:51:10.447Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"361e809d-319f-49e5-bf22-8c91e2352fc0","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRjjA4Nb3H6iK2pvyP3","type":"message","role":"assistant","content":[{"type":"text","text":"Important lead — `Default` derives outside `printed.rs`. Let me verify."}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2838,"cache_read_input_tokens":53623,"cache_creation":{"ephemeral_5m_input_tokens":2838,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRjhohYTkdQTaPWKE5L","attributionAgent":"general-purpose","type":"assistant","uuid":"a5407114-5c44-4e44-8995-bc4ac656750b","timestamp":"2026-08-01T04:51:10.450Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"a5407114-5c44-4e44-8995-bc4ac656750b","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRjjA4Nb3H6iK2pvyP3","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01SzyG9FXBu9oY6SGpSCTWzh","name":"Bash","input":{"command":"sed -n '165,200p' crates/btctax-core/src/forms.rs && echo \"=== form6251.rs 85-100 ===\" && sed -n '84,100p' crates/btctax-core/src/tax/form6251.rs && echo \"=== printed.rs 740-780 ===\" && sed -n '740,782p' crates/btctax-core/src/tax/printed.rs"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2838,"cache_read_input_tokens":53623,"output_tokens":532,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":2838},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":532,"cache_read_input_tokens":53623,"cache_creation_input_tokens":2838,"cache_creation":{"ephemeral_5m_input_tokens":2838,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRjhohYTkdQTaPWKE5L","attributionAgent":"general-purpose","type":"assistant","uuid":"6c87d49c-099f-4e33-8fde-095b9938e92f","timestamp":"2026-08-01T04:51:11.919Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"6c87d49c-099f-4e33-8fde-095b9938e92f","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01SzyG9FXBu9oY6SGpSCTWzh","type":"tool_result","content":"    keyed.into_iter().map(|(_, _, _, r)| r).collect()
}

/// One Schedule D part total (Part I / ST or Part II / LT): the RAW, pre-netting sums over the
/// year's Form 8949 rows (equivalently, the year's disposal legs of that term).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScheduleDPart {
    /// Σ proceeds over the part's legs.
    pub proceeds: Usd,
    /// Σ cost basis over the part's legs.
    pub cost_basis: Usd,
    /// Σ gain/loss over the part's legs (signed).
    pub gain: Usd,
}

/// Schedule D part totals for a tax year: **Part I (ST)** + **Part II (LT)**.
///
/// These are the RAW pre-netting totals. §1222/§1211/§1212 netting + carryforward is applied by
/// engine B (`compute_tax_year` / `report --tax-year`), NOT here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScheduleDTotals {
    /// Part I — short-term.
    pub st: ScheduleDPart,
    /// Part II — long-term.
    pub lt: ScheduleDPart,
}

/// Aggregate the year's disposal legs into Schedule D part totals (Part I ST + Part II LT), summing
/// proceeds/basis/gain within each character. Pure over `state.disposals`; year-scoped by
/// `Disposal.disposed_at.year() == year`. An empty year yields all-zero totals.
///
/// The ST/LT `gain` totals reconcile with engine B's within-character `st_net`/`lt_net` on an
/// all-gains fixture with zero carryforward-in + zero other-net-capital-gain (the R0-M3
/// reconciliation KAT) — `schedule_d` and `compute_tax_year` are separate functions reading the same
/// `state.disposals`, so the equality is a genuine cross-check, not a tautology.
pub fn schedule_d(state: &LedgerState, year: i32) -> ScheduleDTotals {
=== form6251.rs 85-100 ===
/// Lines skipped by a routing instruction (line 6's \"enter -0- here and on lines 7, 9, and 11\", line
/// 32's \"skip lines 33 through 37\", line 34's \"skip lines 35 through 37\") hold `0`, which is what the
/// form directs be entered.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Form6251 {
    // ── Part I — Alternative Minimum Taxable Income ──────────────────────────────────────────────
    /// Part I's line-1 region — **per-year (D-4)**, because 2025 split it. See [`Form6251Line1`].
    pub line1: Form6251Line1,
    /// L2a — \"If filing Schedule A (Form 1040), enter the taxes from Schedule A, line 7; otherwise,
    /// enter the amount from Form 1040 or 1040-SR, line 12.\"
    ///
    /// ★ Line **12**, not line 14 — line 1's fallback cites line 14. They differ by the §199A QBI
    /// deduction, so a filer with QBI > 0 distinguishes them.
    ///
    /// i6251 p.2 adds: \"except any generation-skipping transfer taxes on income distributions\" — v1
    /// has no GST input, so nothing is excluded.
=== printed.rs 740-780 ===
/// The form's Part III is a decision tree, not a column of numbers, and the four branches are
/// mutually exclusive and exhaustive. Modelling it as an enum rather than a bag of `Option`s means an
/// impossible combination (say, line 17 = \"Yes\" together with a line-21 loss) cannot be represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleDRouting {
    /// **L16 > 0 and L15 > 0** — gains in both characters. L17 = **Yes**; L18 = L19 = 0 (the 28%-rate
    /// and unrecaptured-§1250 amounts, both refused upstream if they could ever be nonzero); L20 =
    /// **Yes** → the Qualified Dividends and Capital Gain Tax Worksheet. Lines 21 and 22 are NOT
    /// completed — the form says so in terms.
    BothGains,
    /// **L16 > 0 but L15 ≤ 0** — a short-term gain with a long-term loss. The common crypto year.
    /// L17 = **No** → skip 18 through 21 → L22. Tax routes to QDCGT iff there are qualified dividends.
    ShortGainLongLoss { line22_yes: bool },
    /// **L16 < 0** — a net capital loss. Skip 17–20; L21 carries the §1211(b) allowed offset (a
    /// POSITIVE MAGNITUDE — the form prints the parentheses); L22 is still answered.
    NetLoss {
        /// L21 — the allowed §1211(b) offset, ≤ $3,000 ($1,500 MFS). Positive magnitude.
        line21: Usd,
        line22_yes: bool,
    },
    /// **L16 = 0** — 1040 line 7 is `-0-`. Skip 17–21; L22 is still answered.
    Zero { line22_yes: bool },
}

/// The printable **Schedule D (Capital Gains and Losses)** line chain — the FULL return's Schedule D.
///
/// This is not the crypto-slice Schedule D that `export-irs-pdf` has always produced. That one fills
/// only lines 3/7/10/15/16 from the ledger totals — it has **no line 13** (1099-DIV box-2a
/// capital-gain distributions) and **no lines 6/14** (capital-loss carryovers), which is exactly why
/// the crypto-slice export REFUSES for a full-return year (P5-C1): those omissions make a
/// complete-looking Schedule D that understates income.
///
/// **★ Lines 6, 14 and 21 are PARENTHESIZED boxes — positive magnitudes only.** The form pre-prints
/// the parentheses, so they ARE the minus sign. A negative written there renders as a POSITIVE number
/// on a filed return.
///
/// Column (g) (adjustments from Form 8949) is left blank throughout: v1 models no basis adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleDLines {
    /// L3 (d) — short-term proceeds (Form 8949 Box C or **Box I**).
    pub line3_d: Usd,
    /// L3 (e) — short-term cost basis.
    pub line3_e: Usd,","is_error":false}]},"uuid":"435872f8-c892-4eed-a569-19e4c00ed5b5","timestamp":"2026-08-01T04:51:12.006Z","toolUseResult":{"stdout":"    keyed.into_iter().map(|(_, _, _, r)| r).collect()
}

/// One Schedule D part total (Part I / ST or Part II / LT): the RAW, pre-netting sums over the
/// year's Form 8949 rows (equivalently, the year's disposal legs of that term).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScheduleDPart {
    /// Σ proceeds over the part's legs.
    pub proceeds: Usd,
    /// Σ cost basis over the part's legs.
    pub cost_basis: Usd,
    /// Σ gain/loss over the part's legs (signed).
    pub gain: Usd,
}

/// Schedule D part totals for a tax year: **Part I (ST)** + **Part II (LT)**.
///
/// These are the RAW pre-netting totals. §1222/§1211/§1212 netting + carryforward is applied by
/// engine B (`compute_tax_year` / `report --tax-year`), NOT here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScheduleDTotals {
    /// Part I — short-term.
    pub st: ScheduleDPart,
    /// Part II — long-term.
    pub lt: ScheduleDPart,
}

/// Aggregate the year's disposal legs into Schedule D part totals (Part I ST + Part II LT), summing
/// proceeds/basis/gain within each character. Pure over `state.disposals`; year-scoped by
/// `Disposal.disposed_at.year() == year`. An empty year yields all-zero totals.
///
/// The ST/LT `gain` totals reconcile with engine B's within-character `st_net`/`lt_net` on an
/// all-gains fixture with zero carryforward-in + zero other-net-capital-gain (the R0-M3
/// reconciliation KAT) — `schedule_d` and `compute_tax_year` are separate functions reading the same
/// `state.disposals`, so the equality is a genuine cross-check, not a tautology.
pub fn schedule_d(state: &LedgerState, year: i32) -> ScheduleDTotals {
=== form6251.rs 85-100 ===
/// Lines skipped by a routing instruction (line 6's \"enter -0- here and on lines 7, 9, and 11\", line
/// 32's \"skip lines 33 through 37\", line 34's \"skip lines 35 through 37\") hold `0`, which is what the
/// form directs be entered.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Form6251 {
    // ── Part I — Alternative Minimum Taxable Income ──────────────────────────────────────────────
    /// Part I's line-1 region — **per-year (D-4)**, because 2025 split it. See [`Form6251Line1`].
    pub line1: Form6251Line1,
    /// L2a — \"If filing Schedule A (Form 1040), enter the taxes from Schedule A, line 7; otherwise,
    /// enter the amount from Form 1040 or 1040-SR, line 12.\"
    ///
    /// ★ Line **12**, not line 14 — line 1's fallback cites line 14. They differ by the §199A QBI
    /// deduction, so a filer with QBI > 0 distinguishes them.
    ///
    /// i6251 p.2 adds: \"except any generation-skipping transfer taxes on income distributions\" — v1
    /// has no GST input, so nothing is excluded.
=== printed.rs 740-780 ===
/// The form's Part III is a decision tree, not a column of numbers, and the four branches are
/// mutually exclusive and exhaustive. Modelling it as an enum rather than a bag of `Option`s means an
/// impossible combination (say, line 17 = \"Yes\" together with a line-21 loss) cannot be represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleDRouting {
    /// **L16 > 0 and L15 > 0** — gains in both characters. L17 = **Yes**; L18 = L19 = 0 (the 28%-rate
    /// and unrecaptured-§1250 amounts, both refused upstream if they could ever be nonzero); L20 =
    /// **Yes** → the Qualified Dividends and Capital Gain Tax Worksheet. Lines 21 and 22 are NOT
    /// completed — the form says so in terms.
    BothGains,
    /// **L16 > 0 but L15 ≤ 0** — a short-term gain with a long-term loss. The common crypto year.
    /// L17 = **No** → skip 18 through 21 → L22. Tax routes to QDCGT iff there are qualified dividends.
    ShortGainLongLoss { line22_yes: bool },
    /// **L16 < 0** — a net capital loss. Skip 17–20; L21 carries the §1211(b) allowed offset (a
    /// POSITIVE MAGNITUDE — the form prints the parentheses); L22 is still answered.
    NetLoss {
        /// L21 — the allowed §1211(b) offset, ≤ $3,000 ($1,500 MFS). Positive magnitude.
        line21: Usd,
        line22_yes: bool,
    },
    /// **L16 = 0** — 1040 line 7 is `-0-`. Skip 17–21; L22 is still answered.
    Zero { line22_yes: bool },
}

/// The printable **Schedule D (Capital Gains and Losses)** line chain — the FULL return's Schedule D.
///
/// This is not the crypto-slice Schedule D that `export-irs-pdf` has always produced. That one fills
/// only lines 3/7/10/15/16 from the ledger totals — it has **no line 13** (1099-DIV box-2a
/// capital-gain distributions) and **no lines 6/14** (capital-loss carryovers), which is exactly why
/// the crypto-slice export REFUSES for a full-return year (P5-C1): those omissions make a
/// complete-looking Schedule D that understates income.
///
/// **★ Lines 6, 14 and 21 are PARENTHESIZED boxes — positive magnitudes only.** The form pre-prints
/// the parentheses, so they ARE the minus sign. A negative written there renders as a POSITIVE number
/// on a filed return.
///
/// Column (g) (adjustments from Form 8949) is left blank throughout: v1 models no basis adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleDLines {
    /// L3 (d) — short-term proceeds (Form 8949 Box C or **Box I**).
    pub line3_d: Usd,
    /// L3 (e) — short-term cost basis.
    pub line3_e: Usd,","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"6c87d49c-099f-4e33-8fde-095b9938e92f","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"435872f8-c892-4eed-a569-19e4c00ed5b5","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRkF9rHgrGPVXHeZUzS","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"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"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2897,"cache_read_input_tokens":56461,"cache_creation":{"ephemeral_5m_input_tokens":2897,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRkDf4QBSEy6qxtFNVp","attributionAgent":"general-purpose","type":"assistant","uuid":"d0dffdc6-68ca-4730-bf1c-92f1c4856b1c","timestamp":"2026-08-01T04:51:21.805Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"d0dffdc6-68ca-4730-bf1c-92f1c4856b1c","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRkF9rHgrGPVXHeZUzS","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01KNssMyVrGs9Zr1TvCLXGJ8","name":"Bash","input":{"command":"echo \"=== ScheduleDTotals / ScheduleDPart consumers ===\" && grep -rn \"ScheduleDTotals\\|ScheduleDPart\" crates/ --include=*.rs | grep -v \"^crates/btctax-core/src/forms.rs\" | head -40"},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2897,"cache_read_input_tokens":56461,"cache_creation":{"ephemeral_5m_input_tokens":2897,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRkDf4QBSEy6qxtFNVp","attributionAgent":"general-purpose","type":"assistant","uuid":"62cbd7c8-8440-4f6c-9aa5-146b815d622c","timestamp":"2026-08-01T04:51:22.668Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"62cbd7c8-8440-4f6c-9aa5-146b815d622c","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01KNssMyVrGs9Zr1TvCLXGJ8","type":"tool_result","content":"=== ScheduleDTotals / ScheduleDPart consumers ===
(eval):1: no matches found: --include=*.rs","is_error":false}]},"uuid":"1f2d16da-2626-49a7-a0ef-3c528c3611fb","timestamp":"2026-08-01T04:51:22.755Z","toolUseResult":{"stdout":"=== ScheduleDTotals / ScheduleDPart consumers ===
(eval):1: no matches found: --include=*.rs","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"62cbd7c8-8440-4f6c-9aa5-146b815d622c","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"1f2d16da-2626-49a7-a0ef-3c528c3611fb","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRkF9rHgrGPVXHeZUzS","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01KLv4aVEh1vgQstu44y67vE","name":"Bash","input":{"command":"sed -n '195,240p' crates/btctax-core/src/forms.rs"},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2897,"cache_read_input_tokens":56461,"cache_creation":{"ephemeral_5m_input_tokens":2897,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRkDf4QBSEy6qxtFNVp","attributionAgent":"general-purpose","type":"assistant","uuid":"59a18f4b-96c8-41c5-9e73-440d97d85402","timestamp":"2026-08-01T04:51:23.308Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"59a18f4b-96c8-41c5-9e73-440d97d85402","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01KLv4aVEh1vgQstu44y67vE","type":"tool_result","content":"///
/// The ST/LT `gain` totals reconcile with engine B's within-character `st_net`/`lt_net` on an
/// all-gains fixture with zero carryforward-in + zero other-net-capital-gain (the R0-M3
/// reconciliation KAT) — `schedule_d` and `compute_tax_year` are separate functions reading the same
/// `state.disposals`, so the equality is a genuine cross-check, not a tautology.
pub fn schedule_d(state: &LedgerState, year: i32) -> ScheduleDTotals {
    let mut totals = ScheduleDTotals::default();
    for d in state
        .disposals
        .iter()
        .filter(|d| d.disposed_at.year() == year)
    {
        for leg in &d.legs {
            let part = match leg.term {
                Term::ShortTerm => &mut totals.st,
                Term::LongTerm => &mut totals.lt,
            };
            part.proceeds += leg.proceeds;
            part.cost_basis += leg.basis;
            part.gain += leg.gain;
        }
    }
    totals
}

// ── Sub-project C (P2-C): IRS Form 8283 (Noncash Charitable Contributions) ───────────────────────
//
// Pure, year-scoped projection over `state.removals` where `kind == Donation`. No tax math is done
// here (the §170(e) `claimed_deduction` was computed by the fold and lives on the `Removal`). Like
// Form 8949 this is STANDALONE — it does NOT feed `compute_tax_year` / engine B (Schedule-A-adjacent,
// §170). Federal-only. No float (NFR5: the BTC-amount description is EXACT `Decimal`).

/// The Form 8283 part a donation is reported in, driven by the **§170(f)(11)(F) year-aggregate**
/// claimed deduction over ALL BTC donations in the tax year (all BTC is \"similar property\"):
/// - **Section A** — year-aggregate ≤ $5,000 (§170(f)(11)(C): \"more than $5,000\" is the threshold;
///   exactly $5,000 → Section A).
/// - **Section B** — year-aggregate > $5,000: a **qualified appraisal + appraiser signature** is
///   required (CCA 202302012 confirms the readily-valued exception does NOT apply to crypto).
///
/// The section is **UNIFORM** across all donations in the year (all BTC is one similar-property
/// class). This replaces the prior per-donation threshold test (§170(f)(11)(F) aggregates similar
/// items across the year; a per-donation test under-triggers Section B).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form8283Section {
    /// Section A — year-aggregate ≤ $5,000.
    A,","is_error":false}]},"uuid":"d2d1a2f7-34b0-4264-8add-e9a42ade85e0","timestamp":"2026-08-01T04:51:23.388Z","toolUseResult":{"stdout":"///
/// The ST/LT `gain` totals reconcile with engine B's within-character `st_net`/`lt_net` on an
/// all-gains fixture with zero carryforward-in + zero other-net-capital-gain (the R0-M3
/// reconciliation KAT) — `schedule_d` and `compute_tax_year` are separate functions reading the same
/// `state.disposals`, so the equality is a genuine cross-check, not a tautology.
pub fn schedule_d(state: &LedgerState, year: i32) -> ScheduleDTotals {
    let mut totals = ScheduleDTotals::default();
    for d in state
        .disposals
        .iter()
        .filter(|d| d.disposed_at.year() == year)
    {
        for leg in &d.legs {
            let part = match leg.term {
                Term::ShortTerm => &mut totals.st,
                Term::LongTerm => &mut totals.lt,
            };
            part.proceeds += leg.proceeds;
            part.cost_basis += leg.basis;
            part.gain += leg.gain;
        }
    }
    totals
}

// ── Sub-project C (P2-C): IRS Form 8283 (Noncash Charitable Contributions) ───────────────────────
//
// Pure, year-scoped projection over `state.removals` where `kind == Donation`. No tax math is done
// here (the §170(e) `claimed_deduction` was computed by the fold and lives on the `Removal`). Like
// Form 8949 this is STANDALONE — it does NOT feed `compute_tax_year` / engine B (Schedule-A-adjacent,
// §170). Federal-only. No float (NFR5: the BTC-amount description is EXACT `Decimal`).

/// The Form 8283 part a donation is reported in, driven by the **§170(f)(11)(F) year-aggregate**
/// claimed deduction over ALL BTC donations in the tax year (all BTC is \"similar property\"):
/// - **Section A** — year-aggregate ≤ $5,000 (§170(f)(11)(C): \"more than $5,000\" is the threshold;
///   exactly $5,000 → Section A).
/// - **Section B** — year-aggregate > $5,000: a **qualified appraisal + appraiser signature** is
///   required (CCA 202302012 confirms the readily-valued exception does NOT apply to crypto).
///
/// The section is **UNIFORM** across all donations in the year (all BTC is one similar-property
/// class). This replaces the prior per-donation threshold test (§170(f)(11)(F) aggregates similar
/// items across the year; a per-donation test under-triggers Section B).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form8283Section {
    /// Section A — year-aggregate ≤ $5,000.
    A,","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"59a18f4b-96c8-41c5-9e73-440d97d85402","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"d2d1a2f7-34b0-4264-8add-e9a42ade85e0","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRm8Unaxww8RsMNpMqq","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAIS/wMKhwEIEBgCKkDLIsgLB47b5MPP1aoK9f4YhD+cTbKAjD7cNJ2q0jp8px/H86F0StEJPrEsZv59xlOMg5TlwPTl8oGQ/Xr9tzb7Mg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDLIaWsqKSGfuEhClOBoMa/FKuVLsKbTlBWdBIjC9q4DP8BJ3cvxhWfm5P0BctDnqb84jwstVE6gvkk6eewxW5rX2v1q6Y4HoPKJiRUcqpAKI1ad9eqUh080pWjRXPDm4NNY+QlBTne+k08lIDcqZu7gzopsOP5mwSjiGVpif3RvapFbICRtCEtEdKkcX+3wCuMPxSbTNgTTAOi1mKr0Ky+6UvA6d9C1um4LSdXZ3wKzASVMpYZPH4NlLUyL9S5rTJPVqbEADiUNBV0DT8kROXfYIImPsL21sQSCuchWzrXx9Y76ipjIarBUeny9gHhcU1i6JESOa8Iesm/ry57EysVyAdMtTLF7Rms/pqxx7Oc+lpvkZ2kCA4QCgV9k8o5ExMfERHbfjUusFusqgEnxiH0vMZHit6S+Oo0CpHm6gF+mGaZ6etUU2s18rrGNZHCfgbKYYuDYEh9ZuS9fWzDJgW43I8tLz2zzWpgi7SnD7fbL7sB4ZGAE="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1924,"cache_read_input_tokens":59358,"cache_creation":{"ephemeral_5m_input_tokens":1924,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRm5uFpDYezQG81oY9i","attributionAgent":"general-purpose","type":"assistant","uuid":"c3012ee1-12d7-4348-907e-a384762d3bcd","timestamp":"2026-08-01T04:51:27.428Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"c3012ee1-12d7-4348-907e-a384762d3bcd","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRm8Unaxww8RsMNpMqq","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01LDhg57sLbMpNgLT4fbyud9","name":"Bash","input":{"command":"grep -rn \"ScheduleDTotals\\|ScheduleDPart\" --include=\"*.rs\" crates/ | grep -v \"^crates/btctax-core/src/forms.rs\" "},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1924,"cache_read_input_tokens":59358,"cache_creation":{"ephemeral_5m_input_tokens":1924,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRm5uFpDYezQG81oY9i","attributionAgent":"general-purpose","type":"assistant","uuid":"666e1394-4bac-41e5-9545-c5c3d9cd8cd1","timestamp":"2026-08-01T04:51:28.509Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"666e1394-4bac-41e5-9545-c5c3d9cd8cd1","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01LDhg57sLbMpNgLT4fbyud9","type":"tool_result","content":"crates/btctax-core/src/lib.rs:27:    Form8283Section, Form8949Box, Form8949Part, Form8949Row, ScheduleDPart, ScheduleDTotals,
crates/btctax-forms/src/schedule_d.rs:27:use btctax_core::ScheduleDTotals;
crates/btctax-forms/src/schedule_d.rs:35:fn active(p: &btctax_core::ScheduleDPart) -> bool {
crates/btctax-forms/src/schedule_d.rs:64:    totals: &ScheduleDTotals,
crates/btctax-core/src/tax/printed.rs:1445:            schedule_d: crate::tax::return_1040::ScheduleDParts {
crates/btctax-core/src/tax/printed.rs:2516:        use crate::tax::return_1040::ScheduleDParts;
crates/btctax-core/src/tax/printed.rs:2518:        ar.schedule_d = ScheduleDParts {
crates/btctax-cli/src/cmd/tax.rs:10:    ScheduleDTotals, TaxOutcome, TaxProfile, TaxTables, Usd,
crates/btctax-cli/src/cmd/tax.rs:241:    pub schedule_d: ScheduleDTotals,
crates/btctax-cli/src/render.rs:18:    LtcgBracket, OutflowClass, RemovalKind, RemovalLeg, ScheduleDTotals, SeTaxResult, SellReport,
crates/btctax-cli/src/render.rs:1611:    totals: &ScheduleDTotals,
crates/btctax-cli/src/cmd/admin.rs:9:    compute_se_tax, se_net_income, FeeTreatment, LedgerEvent, LotMethod, ScheduleDPart, Severity,
crates/btctax-cli/src/cmd/admin.rs:352:fn sd_part_active(p: &ScheduleDPart) -> bool {
crates/btctax-core/src/tax/return_1040.rs:222:pub struct ScheduleDParts {
crates/btctax-core/src/tax/return_1040.rs:955:    pub schedule_d: ScheduleDParts,
crates/btctax-core/src/tax/return_1040.rs:1191:    let schedule_d = ScheduleDParts {
crates/btctax-forms/src/lib.rs:60:use btctax_core::{Form8949Row, ScheduleDTotals};
crates/btctax-forms/src/lib.rs:138:pub fn fill_schedule_d(totals: &ScheduleDTotals, year: i32) -> Result<Vec<u8>, FormsError> {
crates/btctax-forms/tests/sp3.rs:9:    DonationDetails, Form8283HowAcquired, Form8283Row, Form8283Section, ScheduleDPart,
crates/btctax-forms/tests/sp3.rs:10:    ScheduleDTotals, SeTaxResult, Usd,
crates/btctax-forms/tests/sp3.rs:192:    let totals = ScheduleDTotals {
crates/btctax-forms/tests/sp3.rs:193:        st: ScheduleDPart {
crates/btctax-forms/tests/sp3.rs:198:        lt: ScheduleDPart {
crates/btctax-forms/tests/kats.rs:10:use btctax_core::{Form8949Part, ScheduleDPart, ScheduleDTotals};
crates/btctax-forms/tests/kats.rs:259:    let totals = ScheduleDTotals {
crates/btctax-forms/tests/kats.rs:260:        st: ScheduleDPart {
crates/btctax-forms/tests/kats.rs:265:        lt: ScheduleDPart {
crates/btctax-forms/tests/kats.rs:342:                gain: rust_decimal::Decimal| ScheduleDPart {
crates/btctax-forms/tests/kats.rs:348:    let cases: [(&str, ScheduleDPart, ScheduleDPart, Option<bool>); 5] = [
crates/btctax-forms/tests/kats.rs:398:            let totals = ScheduleDTotals { st: *st, lt: *lt };
crates/btctax-forms/tests/common/mod.rs:11:    Form8949Box, Form8949Part, Form8949Row, ScheduleDPart, ScheduleDTotals, WalletId,
crates/btctax-forms/tests/common/mod.rs:90:pub fn sum_part(rows: &[Form8949Row], part: Form8949Part) -> ScheduleDPart {
crates/btctax-forms/tests/common/mod.rs:91:    let mut p = ScheduleDPart::default();
crates/btctax-forms/tests/common/mod.rs:101:pub fn totals_for(rows: &[Form8949Row]) -> ScheduleDTotals {
crates/btctax-forms/tests/common/mod.rs:102:    ScheduleDTotals {","is_error":false}]},"uuid":"d96c5038-f1ed-403e-8afa-a5af35544bb2","timestamp":"2026-08-01T04:51:28.580Z","toolUseResult":{"stdout":"crates/btctax-core/src/lib.rs:27:    Form8283Section, Form8949Box, Form8949Part, Form8949Row, ScheduleDPart, ScheduleDTotals,
crates/btctax-forms/src/schedule_d.rs:27:use btctax_core::ScheduleDTotals;
crates/btctax-forms/src/schedule_d.rs:35:fn active(p: &btctax_core::ScheduleDPart) -> bool {
crates/btctax-forms/src/schedule_d.rs:64:    totals: &ScheduleDTotals,
crates/btctax-core/src/tax/printed.rs:1445:            schedule_d: crate::tax::return_1040::ScheduleDParts {
crates/btctax-core/src/tax/printed.rs:2516:        use crate::tax::return_1040::ScheduleDParts;
crates/btctax-core/src/tax/printed.rs:2518:        ar.schedule_d = ScheduleDParts {
crates/btctax-cli/src/cmd/tax.rs:10:    ScheduleDTotals, TaxOutcome, TaxProfile, TaxTables, Usd,
crates/btctax-cli/src/cmd/tax.rs:241:    pub schedule_d: ScheduleDTotals,
crates/btctax-cli/src/render.rs:18:    LtcgBracket, OutflowClass, RemovalKind, RemovalLeg, ScheduleDTotals, SeTaxResult, SellReport,
crates/btctax-cli/src/render.rs:1611:    totals: &ScheduleDTotals,
crates/btctax-cli/src/cmd/admin.rs:9:    compute_se_tax, se_net_income, FeeTreatment, LedgerEvent, LotMethod, ScheduleDPart, Severity,
crates/btctax-cli/src/cmd/admin.rs:352:fn sd_part_active(p: &ScheduleDPart) -> bool {
crates/btctax-core/src/tax/return_1040.rs:222:pub struct ScheduleDParts {
crates/btctax-core/src/tax/return_1040.rs:955:    pub schedule_d: ScheduleDParts,
crates/btctax-core/src/tax/return_1040.rs:1191:    let schedule_d = ScheduleDParts {
crates/btctax-forms/src/lib.rs:60:use btctax_core::{Form8949Row, ScheduleDTotals};
crates/btctax-forms/src/lib.rs:138:pub fn fill_schedule_d(totals: &ScheduleDTotals, year: i32) -> Result<Vec<u8>, FormsError> {
crates/btctax-forms/tests/sp3.rs:9:    DonationDetails, Form8283HowAcquired, Form8283Row, Form8283Section, ScheduleDPart,
crates/btctax-forms/tests/sp3.rs:10:    ScheduleDTotals, SeTaxResult, Usd,
crates/btctax-forms/tests/sp3.rs:192:    let totals = ScheduleDTotals {
crates/btctax-forms/tests/sp3.rs:193:        st: ScheduleDPart {
crates/btctax-forms/tests/sp3.rs:198:        lt: ScheduleDPart {
crates/btctax-forms/tests/kats.rs:10:use btctax_core::{Form8949Part, ScheduleDPart, ScheduleDTotals};
crates/btctax-forms/tests/kats.rs:259:    let totals = ScheduleDTotals {
crates/btctax-forms/tests/kats.rs:260:        st: ScheduleDPart {
crates/btctax-forms/tests/kats.rs:265:        lt: ScheduleDPart {
crates/btctax-forms/tests/kats.rs:342:                gain: rust_decimal::Decimal| ScheduleDPart {
crates/btctax-forms/tests/kats.rs:348:    let cases: [(&str, ScheduleDPart, ScheduleDPart, Option<bool>); 5] = [
crates/btctax-forms/tests/kats.rs:398:            let totals = ScheduleDTotals { st: *st, lt: *lt };
crates/btctax-forms/tests/common/mod.rs:11:    Form8949Box, Form8949Part, Form8949Row, ScheduleDPart, ScheduleDTotals, WalletId,
crates/btctax-forms/tests/common/mod.rs:90:pub fn sum_part(rows: &[Form8949Row], part: Form8949Part) -> ScheduleDPart {
crates/btctax-forms/tests/common/mod.rs:91:    let mut p = ScheduleDPart::default();
crates/btctax-forms/tests/common/mod.rs:101:pub fn totals_for(rows: &[Form8949Row]) -> ScheduleDTotals {
crates/btctax-forms/tests/common/mod.rs:102:    ScheduleDTotals {","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"666e1394-4bac-41e5-9545-c5c3d9cd8cd1","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"d96c5038-f1ed-403e-8afa-a5af35544bb2","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRmTosxCV2HNxQPF1bL","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISnQgKhwEIEBgCKkCzLjKg1XCIs31Uvft+IaXFugHy5z+u97uXHX3mmqTigDR8AZ1xIaMCjNyYU9hJdcyBJPcfJHEZ831jes8W6r1TMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDFjchRgdig/Lx9ai3hoMkEhiZjeNdVPDeRsbIjCD0KCSzMEed59/6nSPPCQJjo4bkYR35koTKyGFGochHsOivRw7FWgZSjosao4RbqoqwgaVcOWI4TcxVBR3vHp5xyhZ1w10QVUFurNqYyWZIxGxq3pVuZpBboygGecnRg/l9DuCRK9VreJIcesE18yHCopRWTTusI8VlyE/J2c2i3xfPOFD4/aVpN4pi1XtApMbIBv/PEIHOChq5nbf9H6rnKKjUPzTGqk6cJnFjZdojlEWpTHVk3qbgbUWKStxUGP0O2MxckVWOjD2mn1ZYYj+YRowKUWp1mGDDBDUFSMECwTyN0LJhIrPKa3mNJYCRSbmBkw+cu1ZKu3co5zi52LUckXsAJQqboB73GDNSqJKuK5USqRw0H5STGtu3fGEcypjggtNDilHLrz42KujJVWrejLUl1Y5RebyubswCXpusTym93HFMn8R3YhotZCqpVcfprryemyao7kv4mx3ITNvRxuN7D7rBarSiA4vp0x06c5ui0DINvPUcyRAbJo12M2OnX+MkWLUo0s9H5QFu4ocwpaoa30W2/mJT949/RW05L47OCW+YlR5+PZD13wkKxVffcBiakC3j+Vxd7iRj6AHAxJQoLL3caGo4RWyTFPYpqe3fZZ3CSdLSmvi8hkKfXUq1KlLAMk5DbT7pMNMcG75yyA4l+RZvLT3oFFc7gS6u5ZHTON5I2sCAwbDdgqR9Pm+0lgCLWUsqlhZ6yVs/G30gMKqb02y9hPEaXvNF4ckKIwXAZI0178fc0u6bKeVmEQoSh9oSW4DuowOd/+DGMSlScznlOJekfxYvozf43mcR1iYpRXyIxB8hAt6VGPsvO64Q7qwCkTfPA1Fh8CQjMLarzlssGVvWrFgYH7f48TxviLFCDQXcozs2A0l/JbULmah8BWebCI79caKw1PYUbDpZVoY3U5A3tMRFj+dZkCTGVirhDHPmQGttwnjZOEXuthYVysMFNFZpUxXkAohkiqS8lrdqECuGi1A+WF+CW7c7dPJayBleh9nRx3Z18R3GQJU41dC4zjWNR/ogDhMPYdRagsls0a4eDxHDLCcdgYgsRfEsyqCtuS97dwIR8+DmWjO6vHukHKFm6WBMnWLgHHU2sCYztQEYeXoNhS0qo3tYHZbtZNVLKqoem4XaO182ynqpL34yKggIDUxC28Gs17pCKRIBzkYAQ=="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1954,"cache_read_input_tokens":61282,"cache_creation":{"ephemeral_5m_input_tokens":1954,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRmSprAVbswXfKGC4jT","attributionAgent":"general-purpose","type":"assistant","uuid":"e44c5e9d-433c-443c-89b9-bc6c53e35ccc","timestamp":"2026-08-01T04:51:34.690Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"e44c5e9d-433c-443c-89b9-bc6c53e35ccc","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRmTosxCV2HNxQPF1bL","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_013FiBoLAM1aJPTrMmxWyKhR","name":"Bash","input":{"command":"sed -n '215,260p' crates/btctax-cli/src/cmd/tax.rs; echo \"=== schedule_d.rs 25-110 ===\"; sed -n '25,115p' crates/btctax-forms/src/schedule_d.rs"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1954,"cache_read_input_tokens":61282,"output_tokens":506,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":1954},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":506,"cache_read_input_tokens":61282,"cache_creation_input_tokens":1954,"cache_creation":{"ephemeral_5m_input_tokens":1954,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRmSprAVbswXfKGC4jT","attributionAgent":"general-purpose","type":"assistant","uuid":"f466bcbe-a7ce-478d-a0b9-3ad6c08c9d48","timestamp":"2026-08-01T04:51:35.253Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"f466bcbe-a7ce-478d-a0b9-3ad6c08c9d48","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_013FiBoLAM1aJPTrMmxWyKhR","type":"tool_result","content":"                        let o = a[1].as_u64()? as u16;
                        let d = time::Date::from_ordinal_date(y, o).ok()?;
                        d.format(&format_description!(\"[month]/[day]/[year]\")).ok()
                    });
                    if let Some(s) = readable {
                        *val = serde_json::Value::String(s);
                        continue;
                    }
                }
                format_dobs_readable(val);
            }
        }
        serde_json::Value::Array(arr) => arr.iter_mut().for_each(format_dobs_readable),
        _ => {}
    }
}

/// The full `report --tax-year` bundle, in print order. A NAMED STRUCT (was a 7-tuple) so a new field can
/// never silently transpose with an existing one at a call site (Fable IMPL-P4 r1 N1, `p4-r1-n1`).
#[derive(Debug)]
pub struct TaxYearReport {
    /// The frozen crypto-DELTA engine's outcome for the year.
    pub outcome: TaxOutcome,
    /// M4 carryforward-consistency advisory (non-gating).
    pub advisory: Option<String>,
    /// RAW pre-netting Schedule D part totals.
    pub schedule_d: ScheduleDTotals,
    /// Standalone Form 709 gift advisory.
    pub gift_advisory: Option<String>,
    /// Standalone Schedule SE §1401 section.
    pub schedule_se: Option<String>,
    /// §170(f)(11)(F) year-aggregate donation appraisal advisory.
    pub donation_appraisal: Option<String>,
    /// Conservative-filing (D-9) advisory: per-disposal tranche dip lines + per-wallet method-inversion
    /// warnings. Provenance-neutral; non-gating (never affects the outcome or exit code).
    pub tranche_advisory: Option<String>,
    /// The §6 dual-report block (absolute filed return + crypto delta + the P5 advisories). `Some` only
    /// for a `ReturnInputs`-provenance year; `None` on the delta-only path.
    pub dual_report: Option<String>,
    /// UX-P4-1: the pseudo-disclosure channel for this year's figures — the full §3.1 predicate
    /// (`pseudo_active() OR PseudoPlaceholder`, Synthetic-wins). Drives the banner + `[PSEUDO]` suffix on
    /// every number-bearing surface (delta report, dual-report absolute totals, TUI Tax tab) and the
    /// fail-closed `--write-carryover` gate; `None` when the figures are not pseudo-contributed.
    pub pseudo_contributed: crate::render::PseudoDisclosure,
}

=== schedule_d.rs 25-110 ===
use crate::verify::{column_x_bands, in_band, no_unmapped_filled, Geo, Placement};
use crate::{fmt_money, pdf};
use btctax_core::ScheduleDTotals;

/// Amount-column indices as ordered left→right in the Part I grid: d=0, e=1, g=2, h=3.
const SD_COL_D: usize = 0;
const SD_COL_E: usize = 1;
const SD_COL_H: usize = 3;

/// A part is \"active\" (worth a Schedule D line) iff it has any proceeds/cost/gain.
fn active(p: &btctax_core::ScheduleDPart) -> bool {
    !p.proceeds.is_zero() || !p.cost_basis.is_zero() || !p.gain.is_zero()
}

fn push_amount_line(
    line: &AmountCols,
    proceeds: &str,
    cost: &str,
    gain: &str,
    writes: &mut Vec<(String, pdf::FieldValue)>,
    placements: &mut Vec<Placement>,
) {
    // (d) proceeds, (e) cost, (h) gain — (g) adjustment stays blank (crypto models no adjustment).
    for (fqn, value, col) in [
        (&line.proceeds_d, proceeds, SD_COL_D),
        (&line.cost_e, cost, SD_COL_E),
        (&line.gain_h, gain, SD_COL_H),
    ] {
        writes.push((fqn.clone(), pdf::FieldValue::Text(value.to_string())));
        placements.push(Placement {
            fqn: fqn.clone(),
            geo: Geo::Data { row: 0, col },
        });
    }
}

/// Fill Schedule D from the year's part totals and return the serialized bytes, read back through a
/// geometric + no-unmapped verifier (a mis-mapped amount column fails closed).
pub fn fill_schedule_d_totals(
    totals: &ScheduleDTotals,
    map: &ScheduleDMap,
) -> Result<Vec<u8>, FormsError> {
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<Placement> = Vec::new();
    // These are just amount-column bands checks, so reuse Geo::Data{row:0} (row band is ignored for
    // Schedule D — only the column band matters; see verify_schedule_d below).

    let st_active = active(&totals.st);
    let lt_active = active(&totals.lt);

    if st_active {
        // Line 3 (Part I total = Box I).
        push_amount_line(
            &map.line3,
            &fmt_money(totals.st.proceeds),
            &fmt_money(totals.st.cost_basis),
            &fmt_money(totals.st.gain),
            &mut writes,
            &mut placements,
        );
        // Line 7 (net short-term = line 3 h in SP1).
        push_h_line(
            &map.line7_h,
            &fmt_money(totals.st.gain),
            &mut writes,
            &mut placements,
        );
    }
    if lt_active {
        // Line 10 (Part II total = Box L).
        push_amount_line(
            &map.line10,
            &fmt_money(totals.lt.proceeds),
            &fmt_money(totals.lt.cost_basis),
            &fmt_money(totals.lt.gain),
            &mut writes,
            &mut placements,
        );
        // Line 15 (net long-term = line 10 h in SP1).
        push_h_line(
            &map.line15_h,
            &fmt_money(totals.lt.gain),
            &mut writes,
            &mut placements,
        );
    }
    if st_active || lt_active {
        // Line 16 = line 7 + line 15.
        let total = totals.st.gain + totals.lt.gain;
        push_h_line(
            &map.line16_h,","is_error":false}]},"uuid":"00653388-c5bf-4d00-840f-aea5869038a2","timestamp":"2026-08-01T04:51:35.342Z","toolUseResult":{"stdout":"                        let o = a[1].as_u64()? as u16;
                        let d = time::Date::from_ordinal_date(y, o).ok()?;
                        d.format(&format_description!(\"[month]/[day]/[year]\")).ok()
                    });
                    if let Some(s) = readable {
                        *val = serde_json::Value::String(s);
                        continue;
                    }
                }
                format_dobs_readable(val);
            }
        }
        serde_json::Value::Array(arr) => arr.iter_mut().for_each(format_dobs_readable),
        _ => {}
    }
}

/// The full `report --tax-year` bundle, in print order. A NAMED STRUCT (was a 7-tuple) so a new field can
/// never silently transpose with an existing one at a call site (Fable IMPL-P4 r1 N1, `p4-r1-n1`).
#[derive(Debug)]
pub struct TaxYearReport {
    /// The frozen crypto-DELTA engine's outcome for the year.
    pub outcome: TaxOutcome,
    /// M4 carryforward-consistency advisory (non-gating).
    pub advisory: Option<String>,
    /// RAW pre-netting Schedule D part totals.
    pub schedule_d: ScheduleDTotals,
    /// Standalone Form 709 gift advisory.
    pub gift_advisory: Option<String>,
    /// Standalone Schedule SE §1401 section.
    pub schedule_se: Option<String>,
    /// §170(f)(11)(F) year-aggregate donation appraisal advisory.
    pub donation_appraisal: Option<String>,
    /// Conservative-filing (D-9) advisory: per-disposal tranche dip lines + per-wallet method-inversion
    /// warnings. Provenance-neutral; non-gating (never affects the outcome or exit code).
    pub tranche_advisory: Option<String>,
    /// The §6 dual-report block (absolute filed return + crypto delta + the P5 advisories). `Some` only
    /// for a `ReturnInputs`-provenance year; `None` on the delta-only path.
    pub dual_report: Option<String>,
    /// UX-P4-1: the pseudo-disclosure channel for this year's figures — the full §3.1 predicate
    /// (`pseudo_active() OR PseudoPlaceholder`, Synthetic-wins). Drives the banner + `[PSEUDO]` suffix on
    /// every number-bearing surface (delta report, dual-report absolute totals, TUI Tax tab) and the
    /// fail-closed `--write-carryover` gate; `None` when the figures are not pseudo-contributed.
    pub pseudo_contributed: crate::render::PseudoDisclosure,
}

=== schedule_d.rs 25-110 ===
use crate::verify::{column_x_bands, in_band, no_unmapped_filled, Geo, Placement};
use crate::{fmt_money, pdf};
use btctax_core::ScheduleDTotals;

/// Amount-column indices as ordered left→right in the Part I grid: d=0, e=1, g=2, h=3.
const SD_COL_D: usize = 0;
const SD_COL_E: usize = 1;
const SD_COL_H: usize = 3;

/// A part is \"active\" (worth a Schedule D line) iff it has any proceeds/cost/gain.
fn active(p: &btctax_core::ScheduleDPart) -> bool {
    !p.proceeds.is_zero() || !p.cost_basis.is_zero() || !p.gain.is_zero()
}

fn push_amount_line(
    line: &AmountCols,
    proceeds: &str,
    cost: &str,
    gain: &str,
    writes: &mut Vec<(String, pdf::FieldValue)>,
    placements: &mut Vec<Placement>,
) {
    // (d) proceeds, (e) cost, (h) gain — (g) adjustment stays blank (crypto models no adjustment).
    for (fqn, value, col) in [
        (&line.proceeds_d, proceeds, SD_COL_D),
        (&line.cost_e, cost, SD_COL_E),
        (&line.gain_h, gain, SD_COL_H),
    ] {
        writes.push((fqn.clone(), pdf::FieldValue::Text(value.to_string())));
        placements.push(Placement {
            fqn: fqn.clone(),
            geo: Geo::Data { row: 0, col },
        });
    }
}

/// Fill Schedule D from the year's part totals and return the serialized bytes, read back through a
/// geometric + no-unmapped verifier (a mis-mapped amount column fails closed).
pub fn fill_schedule_d_totals(
    totals: &ScheduleDTotals,
    map: &ScheduleDMap,
) -> Result<Vec<u8>, FormsError> {
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<Placement> = Vec::new();
    // These are just amount-column bands checks, so reuse Geo::Data{row:0} (row band is ignored for
    // Schedule D — only the column band matters; see verify_schedule_d below).

    let st_active = active(&totals.st);
    let lt_active = active(&totals.lt);

    if st_active {
        // Line 3 (Part I total = Box I).
        push_amount_line(
            &map.line3,
            &fmt_money(totals.st.proceeds),
            &fmt_money(totals.st.cost_basis),
            &fmt_money(totals.st.gain),
            &mut writes,
            &mut placements,
        );
        // Line 7 (net short-term = line 3 h in SP1).
        push_h_line(
            &map.line7_h,
            &fmt_money(totals.st.gain),
            &mut writes,
            &mut placements,
        );
    }
    if lt_active {
        // Line 10 (Part II total = Box L).
        push_amount_line(
            &map.line10,
            &fmt_money(totals.lt.proceeds),
            &fmt_money(totals.lt.cost_basis),
            &fmt_money(totals.lt.gain),
            &mut writes,
            &mut placements,
        );
        // Line 15 (net long-term = line 10 h in SP1).
        push_h_line(
            &map.line15_h,
            &fmt_money(totals.lt.gain),
            &mut writes,
            &mut placements,
        );
    }
    if st_active || lt_active {
        // Line 16 = line 7 + line 15.
        let total = totals.st.gain + totals.lt.gain;
        push_h_line(
            &map.line16_h,","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"f466bcbe-a7ce-478d-a0b9-3ad6c08c9d48","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"00653388-c5bf-4d00-840f-aea5869038a2","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRmxUYvRz3RdCocU8MF","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"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"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2810,"cache_read_input_tokens":63236,"cache_creation":{"ephemeral_5m_input_tokens":2810,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRmwAvfBSHVHZYtVmBV","attributionAgent":"general-purpose","type":"assistant","uuid":"f251133a-87ab-419f-9cc6-b63e942469c4","timestamp":"2026-08-01T04:51:52.868Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"f251133a-87ab-419f-9cc6-b63e942469c4","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRmxUYvRz3RdCocU8MF","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01DD9nsTAHgWqaUcMce2vUSr","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax
echo \"=== Usd definition/derives ===\"; grep -rn \"pub struct Usd\\|pub type Usd\" --include=\"*.rs\" crates/ | head
echo \"=== Usd derive block ===\"; grep -rn -B6 \"pub struct Usd(\" --include=\"*.rs\" crates/ | head -20
echo \"=== Form6251::default / ScheduleDTotals::default / ScheduleDPart::default uses ===\"
grep -rn \"Form6251::default\\|Form6251 { *\\.\\.\\|ScheduleDTotals::default\\|ScheduleDPart::default\\|Printed8949Totals::default\" --include=\"*.rs\" crates/ | head -20
echo \"=== ..Default::default() in scope files ===\"
grep -rn \"\\.\\.Default::default()\\|\\.\\.Form6251::default\" --include=\"*.rs\" crates/btctax-core/src/tax/form6251.rs crates/btctax-core/src/forms.rs crates/btctax-core/src/tax/qbi.rs crates/btctax-core/src/tax/other_taxes.rs | head"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2810,"cache_read_input_tokens":63236,"output_tokens":1634,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":2810},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":1634,"cache_read_input_tokens":63236,"cache_creation_input_tokens":2810,"cache_creation":{"ephemeral_5m_input_tokens":2810,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRmwAvfBSHVHZYtVmBV","attributionAgent":"general-purpose","type":"assistant","uuid":"6a10cc9b-4bc9-4509-9c34-62c9b4828868","timestamp":"2026-08-01T04:51:57.198Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"6a10cc9b-4bc9-4509-9c34-62c9b4828868","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01DD9nsTAHgWqaUcMce2vUSr","type":"tool_result","content":"=== Usd definition/derives ===
crates/btctax-core/src/conventions.rs:8:pub type Usd = Decimal;
=== Usd derive block ===
=== Form6251::default / ScheduleDTotals::default / ScheduleDPart::default uses ===
crates/btctax-core/src/forms.rs:201:    let mut totals = ScheduleDTotals::default();
crates/btctax-core/src/tax/form6251.rs:64:    /// TY2024's shape, matching `Form6251::default()` and every committed fixture vector.
crates/btctax-forms/tests/common/mod.rs:91:    let mut p = ScheduleDPart::default();
=== ..Default::default() in scope files ===","is_error":false}]},"uuid":"8ca53f60-dd89-43a1-83c0-ff80bfd8cd23","timestamp":"2026-08-01T04:51:57.324Z","toolUseResult":{"stdout":"=== Usd definition/derives ===
crates/btctax-core/src/conventions.rs:8:pub type Usd = Decimal;
=== Usd derive block ===
=== Form6251::default / ScheduleDTotals::default / ScheduleDPart::default uses ===
crates/btctax-core/src/forms.rs:201:    let mut totals = ScheduleDTotals::default();
crates/btctax-core/src/tax/form6251.rs:64:    /// TY2024's shape, matching `Form6251::default()` and every committed fixture vector.
crates/btctax-forms/tests/common/mod.rs:91:    let mut p = ScheduleDPart::default();
=== ..Default::default() in scope files ===","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"6a10cc9b-4bc9-4509-9c34-62c9b4828868","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"8ca53f60-dd89-43a1-83c0-ff80bfd8cd23","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRoaDQMwoWbBfAuUnQT","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAIS0goKhwEIEBgCKkA3u0Ig6xbSH7hl7wIWHKydqdhp0XhmJaouE8uMFaIVUz3i8bqSzxpN8ik354rfOnNCIopdl0HCLZkKu32HUv/qMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDPJSCFw5AIitmjzFnhoMXbfpLUUn0PUHaf3CIjBsNRN1BhNliHyNbhbUF6o+BC2GSnZoZlXtQ7Een1PZ0Lt/2N5YA3UxxXYlkKdeI5oq9wjIJENRICylEBJaZj1Fem4oFUv9M7+uTdNULyqQR5Xh9f9G8RPH4iVQvrjh2yilVdwoCixwzQN2oV6pGVg75syXZSVh5LlyzGjZtHpruHg1xoI+L4t+0lq6NFPEsGCs5Loqkrmf7pzyKX8UiCIF8HW8TNHD22h2BjF/qefa8KG6IDybZNWejwKEQO+qh0XhXQ+2x2gxTHuW6SLqt5IuPwJC9qoDLqC/odZKR5MUxjrfJacLC5SOtM2Sjzu8oF2Qgu9ji+DlDMft5WDMD12QXnUGM0QvBK1gYQ4rqnjyTqmxz7MDN4JewfAJiiYKz/GlWZ6+UdW29sPQqUJajMxryjHkAYPRcTLc71Y3L+vupSK7ubjitIF4ovd3Twv6LQWo0twKMJ3j7oLnvFuL0UBVq1DXnCl0sgTjsJkyTmFGl76TFh+wjuEikMwyeRxlHWl/nGnvK0ewrqE8EGDTznMEM9SiZdgbw+zqIxXBlnkXrtYkbyZf8t1Cf4uSkmj0T7e4k8PKSWvlBwt0lJWH8nfJdCTmN1G5JSt2LxtRIFDcVxWqI9iJ6mz25n2qIptl9DN8v8Mr/mxNYPNfGjI4L4dmXu+hHmqVP0DWRQeKK9APa/3K1JcDrNjMk1oMDxl/YoXvDQDzyL2/ZcTXMgZITSHyMa9z3GT1KpeGXunpANsNoxyTmFQegtwyrNcqVEW3BKzas0Z+aKjUV5pSt8+yVAnPyGWtQT7Rm3e99yM4+Gn804ipIqvQVAwcHxSRHju407/+RZFovsn9NehUyJ+7kyH8d0x/fdCZD/mxgEGBykx9kSCOR2/dQATXgKW80jlCAkJ07vALN5gV9t7C7QX+JK5A5RK7LQyb0XM5nX0ESM+CUraTrLBkNbXLsLlXB45giLtzGBW2r6zNEYEM4ImVtYEwzL6rLnMxy0bzoJhzByHUf+q99iRYPRegIjLEvU1Zt6XAGx5h99G7KbNbclHr4uwT5e/hP9lVmqD9KqC3U8wWFVFMwTeQPesRRHniIxlCh7yP8XrArer5imUmLBVxRxBKT09v6f+GN3CKMgkKLM7c8Qy3hN1deSIojf1D2xycTTk5Y/j3+fvC5LtLQf8NgB9i94BA6YMwImC7FHoEw2GcrToLLYz7AAkcCF2FGri86ZX5wKXdql/ii8oSg1OY6FgHtHdrGwTd4wBBBbfppIlpQyu6sapnrmjxwo+vILwuUuScWXxpZFk2WNhHMtjUV6xVU6X3f6SSAcQ0+u6aJppeTp4lMEA/cvLafQ3aV1bnkp7Ak3fozjNzKuSPM2/45b1bC2tbJp9v63BzycXgKAcevbL7COMKx220E+Aah2QX5dBoegScaU41JMzwDlBqmyZBcKY3YO7LmtiYuY5yJAo4Eltjlx648KgP85aaPzPs2y5BgwRj1YjOn9PpYbIApxfqS3sOhEClbBho2JK7ZcuYhT2MJcSpJx0Wad7Q1RL45XJHs54swFp9FGAXs9V/y3O8t0eHpadAWRZnRPczO/sNHxKOnHdEg8AWWocYAQ=="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1883,"cache_read_input_tokens":66046,"cache_creation":{"ephemeral_5m_input_tokens":1883,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRoZAejtV96BuRR7aZf","attributionAgent":"general-purpose","type":"assistant","uuid":"cc08a755-03df-4350-8c23-66bdad242795","timestamp":"2026-08-01T04:52:03.866Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"cc08a755-03df-4350-8c23-66bdad242795","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRoaDQMwoWbBfAuUnQT","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01VN8WXHXz8b7z198jhSJ9zg","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax
echo \"=== Form6251 struct literals & default use across workspace ===\"
grep -rn \"Form6251 *{\\|Form6251::default\\|Form6251Line1\" --include=\"*.rs\" crates/ | grep -v \"^crates/btctax-core/src/tax/form6251.rs\" | head -20
echo \"=== inside form6251.rs ===\"
grep -n \"Form6251 *{\\|Form6251::default()\\|\\.\\. *Default\\|#\\[derive\" crates/btctax-core/src/tax/form6251.rs | head -30
echo
echo \"=== serde anywhere in the 4 scope files ===\"
grep -rn \"serde\" crates/btctax-core/src/tax/printed.rs crates/btctax-core/src/tax/other_taxes.rs crates/btctax-core/src/tax/qbi.rs crates/btctax-core/src/tax/form6251.rs crates/btctax-core/src/forms.rs | head"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1883,"cache_read_input_tokens":66046,"output_tokens":838,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":1883},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":838,"cache_read_input_tokens":66046,"cache_creation_input_tokens":1883,"cache_creation":{"ephemeral_5m_input_tokens":1883,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRoZAejtV96BuRR7aZf","attributionAgent":"general-purpose","type":"assistant","uuid":"2b910378-c399-4a2a-b0c8-cd68822e72b5","timestamp":"2026-08-01T04:52:06.959Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"2b910378-c399-4a2a-b0c8-cd68822e72b5","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01VN8WXHXz8b7z198jhSJ9zg","type":"tool_result","content":"=== Form6251 struct literals & default use across workspace ===
crates/btctax-core/src/tax/return_1040.rs:1547:        line1_rule: crate::tax::form6251::Form6251Line1Rule::Y2024,
=== inside form6251.rs ===
45:#[derive(Debug, Clone, PartialEq, Eq)]
64:    /// TY2024's shape, matching `Form6251::default()` and every committed fixture vector.
87:#[derive(Debug, Clone, PartialEq, Eq, Default)]
89:pub struct Form6251 {
239:impl Form6251 {
264:#[derive(Debug, Clone, Copy)]
314:pub fn compute_6251(i: Form6251Inputs, amt: &AmtParams, bp: &LtcgBreakpoints) -> Form6251 {
374:    let mut f = Form6251 {
564:    fn compute_vector(v: &serde_json::Value, p: &AmtParams) -> Form6251 {

=== serde anywhere in the 4 scope files ===
crates/btctax-core/src/tax/form6251.rs:557:    fn d(v: &serde_json::Value) -> Usd {
crates/btctax-core/src/tax/form6251.rs:564:    fn compute_vector(v: &serde_json::Value, p: &AmtParams) -> Form6251 {
crates/btctax-core/src/tax/form6251.rs:625:        let json: serde_json::Value = serde_json::from_str(VECTORS).expect(\"fixture parses\");
crates/btctax-core/src/tax/form6251.rs:909:        let json: serde_json::Value = serde_json::from_str(VECTORS).unwrap();","is_error":false}]},"uuid":"9173a05b-b977-4649-bc89-7640cf132bd4","timestamp":"2026-08-01T04:52:07.081Z","toolUseResult":{"stdout":"=== Form6251 struct literals & default use across workspace ===
crates/btctax-core/src/tax/return_1040.rs:1547:        line1_rule: crate::tax::form6251::Form6251Line1Rule::Y2024,
=== inside form6251.rs ===
45:#[derive(Debug, Clone, PartialEq, Eq)]
64:    /// TY2024's shape, matching `Form6251::default()` and every committed fixture vector.
87:#[derive(Debug, Clone, PartialEq, Eq, Default)]
89:pub struct Form6251 {
239:impl Form6251 {
264:#[derive(Debug, Clone, Copy)]
314:pub fn compute_6251(i: Form6251Inputs, amt: &AmtParams, bp: &LtcgBreakpoints) -> Form6251 {
374:    let mut f = Form6251 {
564:    fn compute_vector(v: &serde_json::Value, p: &AmtParams) -> Form6251 {

=== serde anywhere in the 4 scope files ===
crates/btctax-core/src/tax/form6251.rs:557:    fn d(v: &serde_json::Value) -> Usd {
crates/btctax-core/src/tax/form6251.rs:564:    fn compute_vector(v: &serde_json::Value, p: &AmtParams) -> Form6251 {
crates/btctax-core/src/tax/form6251.rs:625:        let json: serde_json::Value = serde_json::from_str(VECTORS).expect(\"fixture parses\");
crates/btctax-core/src/tax/form6251.rs:909:        let json: serde_json::Value = serde_json::from_str(VECTORS).unwrap();","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"2b910378-c399-4a2a-b0c8-cd68822e72b5","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"9173a05b-b977-4649-bc89-7640cf132bd4","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRpKpVzFcnVSPF8T2TC","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISmQYKhwEIEBgCKkC+Eds59mgLTtKVJDGypOHQU8tSg2yGQtwxuFfwmo4N3S7SbxfjphbXKMsIO6KDYFMg3lkvXU1wEwN+NoU63FerMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDHrHhHwvacCz1a3T8BoMIEBoi2U9huJQiDySIjAKv04fV6H+PqffjkXXr9Bfw7NHHqGQ9qqUXi/bssu3TLLg4vPAKLoAXi8mRuJsB3oqvgSORH6+oTiwhMEefftHkJ6e9j8qos5cjS6itQ6iXjqm+Ov/5OGcZDEV4xuqAwJt2rcBzf2vAeSD2uqTDG8/68ui0idJ/fK6nR499nB5EzV0ljEAtFpt72rktH5LlmjGuwFtBebGil4rqQCmWK5U9keM0gIA5sFQPaT49LOKwuMNzYFbBtE6UxBP57BRsBlnn7DbzmRMK40OEAF6Eya+0rS7WTsnTBqRohjnD+/FQRrBSkfNiTVDfUM/KbkyPMPvyMZ3LM8ldh5awm5/zhj2qJP1WcMv4np1PIex4Ch36iieeCybe1P64UqrVI/KZtzmWlEoJHzbt4b2YkqouNDwwfw9MneNzIBb4MCE7s/TLziWsZQfPa5PO6OIFg8K7NU1FhFdgcNiNv9eTFiJC/QZIQ+9QrlK2RK3OreoYxPN7BXas+/ZTmGZItiLhJyUGptVB3m672boeL7NhELgeLcuC6nHlYyEOXUWlys++DmRVN8Wta9WL/4yA0Isy0dkAFSlc7aS+qXf2z7GYEWmEMA+BYuZUGCA9FpRavyUE/M/pblCR72fGhqtxPCEkFFZ0z5AosF5YD3VX0K8tE7jtoKaoIfVgT/S63YRzcliCfA5Dml7sRwFWF3y2DMhpkrr93OAxB4EC7/O3ipHg++Fx/7GSyiO4k+MyDC6tDXvNlcZJv9F1h+LlXw7ughNV2bYuQt/+JkzxpO2USHjYHWt3NJnoUdeUeGUpoTf1awb3tkZ0VfYufAGWM9GhpSIEErOLZgfGAE="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1426,"cache_read_input_tokens":67929,"cache_creation":{"ephemeral_5m_input_tokens":1426,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRpGws3csEhTjMwy3Vv","attributionAgent":"general-purpose","type":"assistant","uuid":"f9a3ed62-9adc-43c5-ae8e-adb4a28af759","timestamp":"2026-08-01T04:52:11.476Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"f9a3ed62-9adc-43c5-ae8e-adb4a28af759","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRpKpVzFcnVSPF8T2TC","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01WuJ8ChRh3GxydnfDNn9LSx","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax; sed -n '40,88p' crates/btctax-core/src/tax/form6251.rs; echo \"=== 370-420 ===\"; sed -n '370,400p' crates/btctax-core/src/tax/form6251.rs"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1426,"cache_read_input_tokens":67929,"output_tokens":359,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":1426},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":359,"cache_read_input_tokens":67929,"cache_creation_input_tokens":1426,"cache_creation":{"ephemeral_5m_input_tokens":1426,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRpGws3csEhTjMwy3Vv","attributionAgent":"general-purpose","type":"assistant","uuid":"a743bca4-5cf0-45d9-8d43-0118e195d5d9","timestamp":"2026-08-01T04:52:12.352Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"a743bca4-5cf0-45d9-8d43-0118e195d5d9","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01WuJ8ChRh3GxydnfDNn9LSx","type":"tool_result","content":"/// Form 6251 Part I's **line-1 region**, which is a different shape in each year — so it is a
/// per-year TYPE rather than one field with a year flag beside it (SPEC `design/ty2025` D-4).
///
/// ★ This is the ONLY part of Form 6251 that OBBBA restructured. Parts II and III are line-for-line
/// identical between 2024 and 2025 (only their constants move), so nothing else forks.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Form6251Line1 {
    /// **TY2024 line 1, verbatim:** \"Enter the amount from Form 1040 or 1040-SR, line 15, if more than
    /// zero. If Form 1040 or 1040-SR, line 15, is zero, subtract line 14 of Form 1040 or 1040-SR from
    /// line 11 of Form 1040 or 1040-SR and enter the result here. (If less than zero, enter as a
    /// negative amount.)\"
    Y2024 { line1: Usd },
    /// **TY2025, verbatim:** \"**1a** Subtract Schedule 1-A (Form 1040), line 37, from Form 1040,
    /// 1040-SR, or 1040-NR, line 14.\" · \"**b** Subtract line 1a from Form 1040, 1040-SR, or 1040-NR,
    /// line 11b (if less than zero, enter as a negative amount).\"
    ///
    /// ★ Line 1a subtracts Schedule 1-A line **37** — the *senior* deduction subtotal, not line 38's
    /// total — so the enhanced senior deduction is ADDED BACK for the AMT while the tips, overtime and
    /// car-loan deductions are allowed.
    Y2025 { line1a: Usd, line1b: Usd },
}

impl Default for Form6251Line1 {
    /// TY2024's shape, matching `Form6251::default()` and every committed fixture vector.
    fn default() -> Self {
        Self::Y2024 { line1: Usd::ZERO }
    }
}

impl Form6251Line1 {
    /// The amount **line 4 combines**: line 1 in 2024, line **1b** in 2025 (\"Combine lines 1b
    /// through 3\"). Reading 1a here would put the *deduction* into AMTI instead of the income — the
    /// line-33 defect class, one line up.
    pub fn amount_entering_line4(&self) -> Usd {
        match self {
            Self::Y2024 { line1 } => *line1,
            Self::Y2025 { line1b, .. } => *line1b,
        }
    }
}

/// One filled Form 6251. Field names are the form's line numbers.
///
/// Lines skipped by a routing instruction (line 6's \"enter -0- here and on lines 7, 9, and 11\", line
/// 32's \"skip lines 33 through 37\", line 34's \"skip lines 35 through 37\") hold `0`, which is what the
/// form directs be entered.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
=== 370-420 ===
            amt.rate_28 * x - sub28
        }
    };

    let mut f = Form6251 {
        line1,
        line2a,
        line2b,
        line3,
        line4,
        line5,
        line6,
        line7: z,
        line8: z,
        line9: z,
        line10: z,
        line11: z,
        line12: z,
        line13: z,
        line14: z,
        line15: z,
        line16: z,
        line17: z,
        line18: z,
        line19: z,
        line20: z,
        line21: z,
        line22: z,
        line23: z,
        line24: z,
        line25: z,","is_error":false}]},"uuid":"4bb2ef7e-f3a7-413b-b6f2-da2f778a9c4e","timestamp":"2026-08-01T04:52:12.451Z","toolUseResult":{"stdout":"/// Form 6251 Part I's **line-1 region**, which is a different shape in each year — so it is a
/// per-year TYPE rather than one field with a year flag beside it (SPEC `design/ty2025` D-4).
///
/// ★ This is the ONLY part of Form 6251 that OBBBA restructured. Parts II and III are line-for-line
/// identical between 2024 and 2025 (only their constants move), so nothing else forks.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Form6251Line1 {
    /// **TY2024 line 1, verbatim:** \"Enter the amount from Form 1040 or 1040-SR, line 15, if more than
    /// zero. If Form 1040 or 1040-SR, line 15, is zero, subtract line 14 of Form 1040 or 1040-SR from
    /// line 11 of Form 1040 or 1040-SR and enter the result here. (If less than zero, enter as a
    /// negative amount.)\"
    Y2024 { line1: Usd },
    /// **TY2025, verbatim:** \"**1a** Subtract Schedule 1-A (Form 1040), line 37, from Form 1040,
    /// 1040-SR, or 1040-NR, line 14.\" · \"**b** Subtract line 1a from Form 1040, 1040-SR, or 1040-NR,
    /// line 11b (if less than zero, enter as a negative amount).\"
    ///
    /// ★ Line 1a subtracts Schedule 1-A line **37** — the *senior* deduction subtotal, not line 38's
    /// total — so the enhanced senior deduction is ADDED BACK for the AMT while the tips, overtime and
    /// car-loan deductions are allowed.
    Y2025 { line1a: Usd, line1b: Usd },
}

impl Default for Form6251Line1 {
    /// TY2024's shape, matching `Form6251::default()` and every committed fixture vector.
    fn default() -> Self {
        Self::Y2024 { line1: Usd::ZERO }
    }
}

impl Form6251Line1 {
    /// The amount **line 4 combines**: line 1 in 2024, line **1b** in 2025 (\"Combine lines 1b
    /// through 3\"). Reading 1a here would put the *deduction* into AMTI instead of the income — the
    /// line-33 defect class, one line up.
    pub fn amount_entering_line4(&self) -> Usd {
        match self {
            Self::Y2024 { line1 } => *line1,
            Self::Y2025 { line1b, .. } => *line1b,
        }
    }
}

/// One filled Form 6251. Field names are the form's line numbers.
///
/// Lines skipped by a routing instruction (line 6's \"enter -0- here and on lines 7, 9, and 11\", line
/// 32's \"skip lines 33 through 37\", line 34's \"skip lines 35 through 37\") hold `0`, which is what the
/// form directs be entered.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
=== 370-420 ===
            amt.rate_28 * x - sub28
        }
    };

    let mut f = Form6251 {
        line1,
        line2a,
        line2b,
        line3,
        line4,
        line5,
        line6,
        line7: z,
        line8: z,
        line9: z,
        line10: z,
        line11: z,
        line12: z,
        line13: z,
        line14: z,
        line15: z,
        line16: z,
        line17: z,
        line18: z,
        line19: z,
        line20: z,
        line21: z,
        line22: z,
        line23: z,
        line24: z,
        line25: z,","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"a743bca4-5cf0-45d9-8d43-0118e195d5d9","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"4bb2ef7e-f3a7-413b-b6f2-da2f778a9c4e","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRpk9UmRtzwPTVaUF4K","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISzQIKhwEIEBgCKkBO+sjWij5746z7fLhsiJX2gXwdS+bKly78kXcGuvpEHq1sY/YvH5q/x9kQQw/IJeJGIp/0vxm9u1PL9YIhw4PsMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDMgsoIxGg7Vs1CjWlxoMfoOsc+DHI6Le0dERIjDaAvxNB3URL38Crycv+ZpTnfekQfuztt6E2m0ueboOzV3daIEOrHP1VcDnHgwXVF4qc9VgJE5kje4McuIjepRLYzpkfsmwcZnn5m1V8iDvwAudsK69n+TnA+zmbkbkX0iq38p4Kz4uqJK0DkoZUdoE3LWj0dDlv2kca4SaWQai4U/ols11jTi6nJrLvM4CnRjQIwFVMLbTWS8FeMrXG1ePrG2kN+kYAQ=="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1650,"cache_read_input_tokens":69355,"cache_creation":{"ephemeral_5m_input_tokens":1650,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRpfr1rhJAkfWc4GZKm","attributionAgent":"general-purpose","type":"assistant","uuid":"acb4a251-bb45-4ca8-9ca4-a6beb2844b90","timestamp":"2026-08-01T04:52:15.311Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"acb4a251-bb45-4ca8-9ca4-a6beb2844b90","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRpk9UmRtzwPTVaUF4K","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01QNqZjdLnb5DRaRsppdU9Jw","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax; sed -n '1460,1580p' crates/btctax-cli/src/render.rs"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1650,"cache_read_input_tokens":69355,"output_tokens":132,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":1650},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":132,"cache_read_input_tokens":69355,"cache_creation_input_tokens":1650,"cache_creation":{"ephemeral_5m_input_tokens":1650,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRpfr1rhJAkfWc4GZKm","attributionAgent":"general-purpose","type":"assistant","uuid":"45bad139-c07a-415a-8947-afcd2d12aa55","timestamp":"2026-08-01T04:52:15.853Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"45bad139-c07a-415a-8947-afcd2d12aa55","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01QNqZjdLnb5DRaRsppdU9Jw","type":"tool_result","content":"/// The clinching case is line 37. \"Amount you owe\" is not an analytical figure; it is an instruction to
/// write a check. A tool that says $12,345.67 in the terminal and prints $12,347 on the filed form has
/// produced TWO authoritative answers to \"what do I pay\", and no LIMITATIONS paragraph repairs that. The
/// moment a report line is labelled with a form-line citation, it has promised the FORM's figure.
///
/// The crypto-DELTA block below stays in exact cents: it is not a filed figure — it answers a different
/// question (§6), and the frozen engine computes it in cents.
pub fn render_dual_report(
    year: i32,
    ar: &btctax_core::AbsoluteReturn,
    printed: &btctax_core::tax::packet::PrintedForms,
    delta: &btctax_core::TaxOutcome,
    provenance: crate::resolve::Provenance,
    pseudo: PseudoDisclosure,
) -> String {
    let f = &printed.f1040;
    let mut s = String::new();
    let _ = writeln!(
        s,
        \"\
═══ Absolute filed return (Form 1040) — tax year {year} ═══\"
    );
    let _ = writeln!(s, \"  Profile source: {}\", provenance_label(provenance));
    let _ = writeln!(s, \"  Total income (1040 L9):   {}\", fmt_money(f.line9));
    let _ = writeln!(s, \"  Adjustments (L10):        {}\", fmt_money(f.line10));
    let _ = writeln!(s, \"  AGI (L11):                {}\", fmt_money(f.line11));
    let ded_kind = if ar.deduction_is_itemized {
        \"itemized\"
    } else {
        \"standard\"
    };
    let _ = writeln!(s, \"  Deduction (L12, {ded_kind}): {}\", fmt_money(f.line12));
    if ar.qbi_deduction > Usd::ZERO {
        let _ = writeln!(s, \"  QBI deduction (L13):      {}\", fmt_money(f.line13));
    }
    let _ = writeln!(s, \"  Taxable income (L15):     {}\", fmt_money(f.line15));
    let _ = writeln!(s, \"  Tax (L16):                {}\", fmt_money(f.line16));
    if ar.foreign_tax_credit > Usd::ZERO {
        let _ = writeln!(
            s,
            \"  Foreign tax credit (Sch 3 L1): {}\",
            fmt_money(printed.sch_3.map_or(Usd::ZERO, |s3| s3.line1))
        );
    }
    if ar.se_tax_sch2_l4 > Usd::ZERO {
        let _ = writeln!(
            s,
            \"  Self-employment tax (Sch 2 L4): {}\",
            fmt_money(printed.sch_2.map_or(Usd::ZERO, |s2| s2.line4))
        );
    }
    if ar.additional_medicare.additional_medicare_tax > Usd::ZERO {
        let _ = writeln!(
            s,
            \"  Additional Medicare (Form 8959 → Sch 2 L11): {}\",
            fmt_money(printed.f8959.line18)
        );
    }
    if ar.niit.tax > Usd::ZERO {
        let _ = writeln!(
            s,
            \"  Net Investment Income Tax (Form 8960 → Sch 2 L12): {}\",
            fmt_money(printed.f8960.map_or(Usd::ZERO, |f| f.line17))
        );
    }
    // ★ Form 6251. Shown whenever AMTI EXCEEDS THE EXEMPTION (line 6 > 0) — i.e. exactly when the
    // filer would otherwise wonder why a high-income return went through. The point is that they can
    // SEE the comparison that cleared them (line 7 vs line 10), not just be told there is no AMT.
    //
    // NOT gated on the screening worksheet: that worksheet has been off every production path since
    // v0.14.0, and this block's own condition is the honest description of who sees it. It is a wider
    // audience than the screen would flag — roughly MFJ above ~$133k of AMTI, Single above ~$85k —
    // which is deliberate: a cleared comparison is reassuring, not noisy, and the block is three
    // lines. (Whole-branch review flagged the old comment as describing a gate that was never here.)
    //
    // ★ Coverage note. J10's golden pins this block, and dropping it reds `examples_golden_matches_
    // committed`. It CANNOT discriminate line 9 from line 7, because every bundled journey has no
    // foreign tax credit, and line 9 = line 7 − line 8 collapses to line 7 when line 8 is zero
    // (verified: swapping `line9` for `line7` here leaves the golden green). That distinction is held
    // instead by the vector KAT — fixture V9 carries a $600 §904(j) credit, where line 7 = 2,244,338
    // and line 9 = 2,243,738 diverge, and `every_vector_reproduces_the_form_line_by_line` asserts each
    // line separately. Do not \"simplify\" this to line 7 on the strength of a green golden.
    if ar.amt.line6 > Usd::ZERO {
        let _ = writeln!(
            s,
            \"  Alternative Minimum Tax (Form 6251 → Sch 2 L2): {}\
    \\
             AMTI (L4) {} · exemption (L5) {} · tentative minimum tax (L9) {} · regular tax (L10) {}\
    \\
             {}\",
            fmt_money(ar.amt.line11),
            fmt_money(ar.amt.line4),
            fmt_money(ar.amt.line5),
            fmt_money(ar.amt.line9),
            fmt_money(ar.amt.line10),
            if ar.amt.must_attach() {
                \"Form 6251 line 7 exceeds line 10 → the form MUST be attached (i6251, Who Must File, \\
                 condition 1).\"
            } else {
                \"Line 7 does not exceed line 10 → no Form 6251 attachment is required.\"
            }
        );
    }
    let _ = writeln!(
        s,
        \"  TOTAL TAX (L24):          {}{}\",
        fmt_money(f.line24),
        pseudo.suffix()
    );
    let _ = writeln!(s, \"  Total payments (L33):     {}\", fmt_money(f.line33));
    if f.line34 > Usd::ZERO {
        let _ = writeln!(s, \"  → REFUND (L35a):          {}\", fmt_money(f.line34));
    } else {
        let _ = writeln!(s, \"  → AMOUNT OWED (L37):      {}\", fmt_money(f.line37));
    }
    // §6: the two figures answer different questions and are NEVER reconciled.
    let delta_str = match delta {
        btctax_core::TaxOutcome::Computed(r) => fmt_money(r.total_federal_tax_attributable),
        btctax_core::TaxOutcome::NotComputable(_) => \"not computable\".to_string(),
    };
    let _ = writeln!(
        s,
        \"\
  ── Two DIFFERENT questions — NOT reconciled (SPEC §6) ──\"
    );","is_error":false}]},"uuid":"73bc5ba0-021a-42e4-a734-37f2193f6e10","timestamp":"2026-08-01T04:52:15.938Z","toolUseResult":{"stdout":"/// The clinching case is line 37. \"Amount you owe\" is not an analytical figure; it is an instruction to
/// write a check. A tool that says $12,345.67 in the terminal and prints $12,347 on the filed form has
/// produced TWO authoritative answers to \"what do I pay\", and no LIMITATIONS paragraph repairs that. The
/// moment a report line is labelled with a form-line citation, it has promised the FORM's figure.
///
/// The crypto-DELTA block below stays in exact cents: it is not a filed figure — it answers a different
/// question (§6), and the frozen engine computes it in cents.
pub fn render_dual_report(
    year: i32,
    ar: &btctax_core::AbsoluteReturn,
    printed: &btctax_core::tax::packet::PrintedForms,
    delta: &btctax_core::TaxOutcome,
    provenance: crate::resolve::Provenance,
    pseudo: PseudoDisclosure,
) -> String {
    let f = &printed.f1040;
    let mut s = String::new();
    let _ = writeln!(
        s,
        \"\
═══ Absolute filed return (Form 1040) — tax year {year} ═══\"
    );
    let _ = writeln!(s, \"  Profile source: {}\", provenance_label(provenance));
    let _ = writeln!(s, \"  Total income (1040 L9):   {}\", fmt_money(f.line9));
    let _ = writeln!(s, \"  Adjustments (L10):        {}\", fmt_money(f.line10));
    let _ = writeln!(s, \"  AGI (L11):                {}\", fmt_money(f.line11));
    let ded_kind = if ar.deduction_is_itemized {
        \"itemized\"
    } else {
        \"standard\"
    };
    let _ = writeln!(s, \"  Deduction (L12, {ded_kind}): {}\", fmt_money(f.line12));
    if ar.qbi_deduction > Usd::ZERO {
        let _ = writeln!(s, \"  QBI deduction (L13):      {}\", fmt_money(f.line13));
    }
    let _ = writeln!(s, \"  Taxable income (L15):     {}\", fmt_money(f.line15));
    let _ = writeln!(s, \"  Tax (L16):                {}\", fmt_money(f.line16));
    if ar.foreign_tax_credit > Usd::ZERO {
        let _ = writeln!(
            s,
            \"  Foreign tax credit (Sch 3 L1): {}\",
            fmt_money(printed.sch_3.map_or(Usd::ZERO, |s3| s3.line1))
        );
    }
    if ar.se_tax_sch2_l4 > Usd::ZERO {
        let _ = writeln!(
            s,
            \"  Self-employment tax (Sch 2 L4): {}\",
            fmt_money(printed.sch_2.map_or(Usd::ZERO, |s2| s2.line4))
        );
    }
    if ar.additional_medicare.additional_medicare_tax > Usd::ZERO {
        let _ = writeln!(
            s,
            \"  Additional Medicare (Form 8959 → Sch 2 L11): {}\",
            fmt_money(printed.f8959.line18)
        );
    }
    if ar.niit.tax > Usd::ZERO {
        let _ = writeln!(
            s,
            \"  Net Investment Income Tax (Form 8960 → Sch 2 L12): {}\",
            fmt_money(printed.f8960.map_or(Usd::ZERO, |f| f.line17))
        );
    }
    // ★ Form 6251. Shown whenever AMTI EXCEEDS THE EXEMPTION (line 6 > 0) — i.e. exactly when the
    // filer would otherwise wonder why a high-income return went through. The point is that they can
    // SEE the comparison that cleared them (line 7 vs line 10), not just be told there is no AMT.
    //
    // NOT gated on the screening worksheet: that worksheet has been off every production path since
    // v0.14.0, and this block's own condition is the honest description of who sees it. It is a wider
    // audience than the screen would flag — roughly MFJ above ~$133k of AMTI, Single above ~$85k —
    // which is deliberate: a cleared comparison is reassuring, not noisy, and the block is three
    // lines. (Whole-branch review flagged the old comment as describing a gate that was never here.)
    //
    // ★ Coverage note. J10's golden pins this block, and dropping it reds `examples_golden_matches_
    // committed`. It CANNOT discriminate line 9 from line 7, because every bundled journey has no
    // foreign tax credit, and line 9 = line 7 − line 8 collapses to line 7 when line 8 is zero
    // (verified: swapping `line9` for `line7` here leaves the golden green). That distinction is held
    // instead by the vector KAT — fixture V9 carries a $600 §904(j) credit, where line 7 = 2,244,338
    // and line 9 = 2,243,738 diverge, and `every_vector_reproduces_the_form_line_by_line` asserts each
    // line separately. Do not \"simplify\" this to line 7 on the strength of a green golden.
    if ar.amt.line6 > Usd::ZERO {
        let _ = writeln!(
            s,
            \"  Alternative Minimum Tax (Form 6251 → Sch 2 L2): {}\
    \\
             AMTI (L4) {} · exemption (L5) {} · tentative minimum tax (L9) {} · regular tax (L10) {}\
    \\
             {}\",
            fmt_money(ar.amt.line11),
            fmt_money(ar.amt.line4),
            fmt_money(ar.amt.line5),
            fmt_money(ar.amt.line9),
            fmt_money(ar.amt.line10),
            if ar.amt.must_attach() {
                \"Form 6251 line 7 exceeds line 10 → the form MUST be attached (i6251, Who Must File, \\
                 condition 1).\"
            } else {
                \"Line 7 does not exceed line 10 → no Form 6251 attachment is required.\"
            }
        );
    }
    let _ = writeln!(
        s,
        \"  TOTAL TAX (L24):          {}{}\",
        fmt_money(f.line24),
        pseudo.suffix()
    );
    let _ = writeln!(s, \"  Total payments (L33):     {}\", fmt_money(f.line33));
    if f.line34 > Usd::ZERO {
        let _ = writeln!(s, \"  → REFUND (L35a):          {}\", fmt_money(f.line34));
    } else {
        let _ = writeln!(s, \"  → AMOUNT OWED (L37):      {}\", fmt_money(f.line37));
    }
    // §6: the two figures answer different questions and are NEVER reconciled.
    let delta_str = match delta {
        btctax_core::TaxOutcome::Computed(r) => fmt_money(r.total_federal_tax_attributable),
        btctax_core::TaxOutcome::NotComputable(_) => \"not computable\".to_string(),
    };
    let _ = writeln!(
        s,
        \"\
  ── Two DIFFERENT questions — NOT reconciled (SPEC §6) ──\"
    );","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"45bad139-c07a-415a-8947-afcd2d12aa55","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"73bc5ba0-021a-42e4-a734-37f2193f6e10","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRpwvQ8j9gMpvxnQxk3","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"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"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2647,"cache_read_input_tokens":71005,"cache_creation":{"ephemeral_5m_input_tokens":2647,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRpvpfrAh1A9oPgLoMJ","attributionAgent":"general-purpose","type":"assistant","uuid":"e191a816-46a7-43d7-a6d9-0ab52a0c3eb7","timestamp":"2026-08-01T04:53:00.106Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"e191a816-46a7-43d7-a6d9-0ab52a0c3eb7","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRpwvQ8j9gMpvxnQxk3","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_011DoWLVSr3G1sPkCcwadvE6","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax
echo \"=== direct fmt_money in btctax-forms/src ===\"; grep -rn \"fmt_money\" crates/btctax-forms/src/ | grep -v \"fn fmt_money\" | head -30
echo; echo \"=== push_literal sites ===\"; grep -rn \"push_literal\" crates/btctax-forms/src/ | head -10
echo; echo \"=== push_money def ===\"; sed -n '35,80p' crates/btctax-forms/src/cells.rs"},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2647,"cache_read_input_tokens":71005,"cache_creation":{"ephemeral_5m_input_tokens":2647,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRpvpfrAh1A9oPgLoMJ","attributionAgent":"general-purpose","type":"assistant","uuid":"a23b6138-fdb6-47b0-8fba-b849d220b536","timestamp":"2026-08-01T04:53:02.450Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"a23b6138-fdb6-47b0-8fba-b849d220b536","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_011DoWLVSr3G1sPkCcwadvE6","type":"tool_result","content":"=== direct fmt_money in btctax-forms/src ===
crates/btctax-forms/src/cells.rs:3://! A [`MoneyCell`] is either a single field (2024/2025) carrying the whole `fmt_money` string, or a
crates/btctax-forms/src/cells.rs:12:use crate::fmt_money;
crates/btctax-forms/src/cells.rs:32:/// dollars+cents pairs need (`fmt_money` is the raw `Decimal` Display, which pads no cents). The value
crates/btctax-forms/src/cells.rs:54:            w.push((fqn.clone(), FieldValue::Text(fmt_money(value))));
crates/btctax-forms/src/cells.rs:58:            let (dollars, cents) = fmt_money_pair(value);
crates/btctax-forms/src/schedule_d.rs:26:use crate::{fmt_money, pdf};
crates/btctax-forms/src/schedule_d.rs:79:            &fmt_money(totals.st.proceeds),
crates/btctax-forms/src/schedule_d.rs:80:            &fmt_money(totals.st.cost_basis),
crates/btctax-forms/src/schedule_d.rs:81:            &fmt_money(totals.st.gain),
crates/btctax-forms/src/schedule_d.rs:88:            &fmt_money(totals.st.gain),
crates/btctax-forms/src/schedule_d.rs:97:            &fmt_money(totals.lt.proceeds),
crates/btctax-forms/src/schedule_d.rs:98:            &fmt_money(totals.lt.cost_basis),
crates/btctax-forms/src/schedule_d.rs:99:            &fmt_money(totals.lt.gain),
crates/btctax-forms/src/schedule_d.rs:106:            &fmt_money(totals.lt.gain),
crates/btctax-forms/src/schedule_d.rs:116:            &fmt_money(total),
crates/btctax-forms/src/form8275.rs:70:use crate::{fmt_money, pdf};
crates/btctax-forms/src/form8275.rs:240:        push_free(&mut w, &mut p, &row_map.amount, &fmt_money(item.amount));
crates/btctax-forms/src/lib.rs:428:    pub use crate::cells::fmt_money_pair;

=== push_literal sites ===
crates/btctax-forms/src/form1040.rs:19:use crate::cells::{push_literal, push_money};
crates/btctax-forms/src/form1040.rs:116:            push_literal(
crates/btctax-forms/src/cells.rs:73:pub fn push_literal(
crates/btctax-forms/src/form8995.rs:21:use crate::cells::{push_identity, push_literal, push_money, render_ssn};
crates/btctax-forms/src/form8995.rs:188:        push_literal(
crates/btctax-forms/src/form8995.rs:206:        push_literal(

=== push_money def ===
pub fn fmt_money_pair(d: Usd) -> (String, String) {
    let r = d.round_dp(2);
    let whole = r.trunc();
    let cents = ((r - whole).abs() * dec!(100)).round();
    (whole.trunc().to_string(), format!(\"{:02}\", cents))
}

/// Emit the write(s) + flat placement(s) for a money cell. `descent = Some((group, ordinal))` puts
/// the (dollars) field into a strictly-descending-y sequence; `None` makes it column-only.
pub fn push_money(
    w: &mut Vec<(String, FieldValue)>,
    p: &mut Vec<FlatPlacement>,
    cell: &MoneyCell,
    value: Usd,
    col: usize,
    descent: Option<(u32, u32)>,
) {
    match cell {
        MoneyCell::Single(fqn) => {
            w.push((fqn.clone(), FieldValue::Text(fmt_money(value))));
            p.push(geo_placement(fqn, col, descent));
        }
        MoneyCell::Pair(mp) => {
            let (dollars, cents) = fmt_money_pair(value);
            w.push((mp.dollars_field.clone(), FieldValue::Text(dollars)));
            w.push((mp.cents_field.clone(), FieldValue::Text(cents)));
            p.push(geo_placement(&mp.dollars_field, col, descent));
            // The cents widget is authorized (no-unmapped) + page-checked, but not column-checked.
            p.push(FlatPlacement::free(
                mp.cents_field.clone(),
                page_of(&mp.cents_field),
            ));
        }
    }
}

/// Emit a NON-numeric literal (e.g. the \"-0-\" active-and-netted-to-zero marker) into a money cell's
/// (dollars) field. Column-checked, no cents.
pub fn push_literal(
    w: &mut Vec<(String, FieldValue)>,
    p: &mut Vec<FlatPlacement>,
    cell: &MoneyCell,
    literal: &str,
    col: usize,
) {
    let fqn = match cell {","is_error":false}]},"uuid":"e2cdeafb-b669-4dd7-8192-a000ab56a921","timestamp":"2026-08-01T04:53:02.572Z","toolUseResult":{"stdout":"=== direct fmt_money in btctax-forms/src ===
crates/btctax-forms/src/cells.rs:3://! A [`MoneyCell`] is either a single field (2024/2025) carrying the whole `fmt_money` string, or a
crates/btctax-forms/src/cells.rs:12:use crate::fmt_money;
crates/btctax-forms/src/cells.rs:32:/// dollars+cents pairs need (`fmt_money` is the raw `Decimal` Display, which pads no cents). The value
crates/btctax-forms/src/cells.rs:54:            w.push((fqn.clone(), FieldValue::Text(fmt_money(value))));
crates/btctax-forms/src/cells.rs:58:            let (dollars, cents) = fmt_money_pair(value);
crates/btctax-forms/src/schedule_d.rs:26:use crate::{fmt_money, pdf};
crates/btctax-forms/src/schedule_d.rs:79:            &fmt_money(totals.st.proceeds),
crates/btctax-forms/src/schedule_d.rs:80:            &fmt_money(totals.st.cost_basis),
crates/btctax-forms/src/schedule_d.rs:81:            &fmt_money(totals.st.gain),
crates/btctax-forms/src/schedule_d.rs:88:            &fmt_money(totals.st.gain),
crates/btctax-forms/src/schedule_d.rs:97:            &fmt_money(totals.lt.proceeds),
crates/btctax-forms/src/schedule_d.rs:98:            &fmt_money(totals.lt.cost_basis),
crates/btctax-forms/src/schedule_d.rs:99:            &fmt_money(totals.lt.gain),
crates/btctax-forms/src/schedule_d.rs:106:            &fmt_money(totals.lt.gain),
crates/btctax-forms/src/schedule_d.rs:116:            &fmt_money(total),
crates/btctax-forms/src/form8275.rs:70:use crate::{fmt_money, pdf};
crates/btctax-forms/src/form8275.rs:240:        push_free(&mut w, &mut p, &row_map.amount, &fmt_money(item.amount));
crates/btctax-forms/src/lib.rs:428:    pub use crate::cells::fmt_money_pair;

=== push_literal sites ===
crates/btctax-forms/src/form1040.rs:19:use crate::cells::{push_literal, push_money};
crates/btctax-forms/src/form1040.rs:116:            push_literal(
crates/btctax-forms/src/cells.rs:73:pub fn push_literal(
crates/btctax-forms/src/form8995.rs:21:use crate::cells::{push_identity, push_literal, push_money, render_ssn};
crates/btctax-forms/src/form8995.rs:188:        push_literal(
crates/btctax-forms/src/form8995.rs:206:        push_literal(

=== push_money def ===
pub fn fmt_money_pair(d: Usd) -> (String, String) {
    let r = d.round_dp(2);
    let whole = r.trunc();
    let cents = ((r - whole).abs() * dec!(100)).round();
    (whole.trunc().to_string(), format!(\"{:02}\", cents))
}

/// Emit the write(s) + flat placement(s) for a money cell. `descent = Some((group, ordinal))` puts
/// the (dollars) field into a strictly-descending-y sequence; `None` makes it column-only.
pub fn push_money(
    w: &mut Vec<(String, FieldValue)>,
    p: &mut Vec<FlatPlacement>,
    cell: &MoneyCell,
    value: Usd,
    col: usize,
    descent: Option<(u32, u32)>,
) {
    match cell {
        MoneyCell::Single(fqn) => {
            w.push((fqn.clone(), FieldValue::Text(fmt_money(value))));
            p.push(geo_placement(fqn, col, descent));
        }
        MoneyCell::Pair(mp) => {
            let (dollars, cents) = fmt_money_pair(value);
            w.push((mp.dollars_field.clone(), FieldValue::Text(dollars)));
            w.push((mp.cents_field.clone(), FieldValue::Text(cents)));
            p.push(geo_placement(&mp.dollars_field, col, descent));
            // The cents widget is authorized (no-unmapped) + page-checked, but not column-checked.
            p.push(FlatPlacement::free(
                mp.cents_field.clone(),
                page_of(&mp.cents_field),
            ));
        }
    }
}

/// Emit a NON-numeric literal (e.g. the \"-0-\" active-and-netted-to-zero marker) into a money cell's
/// (dollars) field. Column-checked, no cents.
pub fn push_literal(
    w: &mut Vec<(String, FieldValue)>,
    p: &mut Vec<FlatPlacement>,
    cell: &MoneyCell,
    literal: &str,
    col: usize,
) {
    let fqn = match cell {","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"a23b6138-fdb6-47b0-8fba-b849d220b536","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"e2cdeafb-b669-4dd7-8192-a000ab56a921","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRpwvQ8j9gMpvxnQxk3","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01FhZwXQyyqyK4XCAkYsnP2Z","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax; rustc --version; cargo --version; cargo clippy --version; grep -n \"clippy\" Makefile | head"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2647,"cache_read_input_tokens":71005,"output_tokens":3456,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":2647},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":3456,"cache_read_input_tokens":71005,"cache_creation_input_tokens":2647,"cache_creation":{"ephemeral_5m_input_tokens":2647,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRpvpfrAh1A9oPgLoMJ","attributionAgent":"general-purpose","type":"assistant","uuid":"91bcb210-9d35-4269-bab8-5634bf3550ad","timestamp":"2026-08-01T04:53:02.751Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"91bcb210-9d35-4269-bab8-5634bf3550ad","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01FhZwXQyyqyK4XCAkYsnP2Z","type":"tool_result","content":"rustc 1.97.0-nightly (52b6e2c20 2026-04-27)
cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)
clippy 0.1.97 (52b6e2c208 2026-04-27)
5:## check: the validation gate — the full test suite AND clippy, run CONCURRENTLY (~6s warm)
10:## let them run at the same time, which makes clippy effectively free.
27:\t CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings & c=$$!; \\
36:## lint: clippy on its own, in its own target dir so it does not thrash the test cache
38:\tCARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings","is_error":false}]},"uuid":"f41d40ae-e770-46df-aa52-d978111e6cd8","timestamp":"2026-08-01T04:53:02.897Z","toolUseResult":{"stdout":"rustc 1.97.0-nightly (52b6e2c20 2026-04-27)
cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)
clippy 0.1.97 (52b6e2c208 2026-04-27)
5:## check: the validation gate — the full test suite AND clippy, run CONCURRENTLY (~6s warm)
10:## let them run at the same time, which makes clippy effectively free.
27:\t CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings & c=$$!; \\
36:## lint: clippy on its own, in its own target dir so it does not thrash the test cache
38:\tCARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"91bcb210-9d35-4269-bab8-5634bf3550ad","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"f41d40ae-e770-46df-aa52-d978111e6cd8","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRtRhaHFvG53rgQvfCj","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAIS2ggKhwEIEBgCKkBPMteIf7OI2GTlA5r8oY/S4TEh6YDh1dn5hE6rWFcgipCNu6V02Zqq9mpfoNXtnAjxh5MhZbtKGZmFWE6hZw4EMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDPn5KyplHiKwuNfwmxoMfV7gQshjP2jS5WiAIjA2vAwaNV6xhwgo8JXEm2ZcHLPl6s/a2jYfXUOaFJRniKht0r0E46tSAxb/ipiDWs4q/wblNXY9bb1cnBZhRVAma8DwQ+oDG8AILQLieo3Bt8QrcRM4qkc96c1bTrYhTo9GtfXNycxaNrEQL62/xLsDzBCwd6cGd1jAe5j40K0TOZouP/dKeF9J49AolWztqIyzB5q6kgN+OSn5LQ9sZbZOQUHFa3BfgV+r217elIIDVgWv7csihLEKYXbabBAsmpLv1nLuS/bnECc2G65gdqCNly5EeyTLP/Ay2TPMmA6b9DXbNKOPlBr534d/2Z86ELfKSD5a6iZq79FKgvA5fB4tPgg8ugjvLwSGuzL6riJKM+ovTJqqr5OhyKPh9YTSY1eoWUCkgm2xo+nCKNCvhYIHaKQEVATcbrawDHLw0c+fCGj1k+xmsFFumnRlVsAm17nU2hyIOHZ0xrP+oErKmQEJ9cZDvPPRGiP9nEcRrt2HSi4oemJoqAT4c42+0zCpMtH5Spjth8zu+616j8OAQKSABVyL5twoyzx/P2S7bnzCJqjSVp3hPlVyZKnSXFEuA8AxzAnqycfH2LwoIXg+J5FA14Tf8S7lAPgAO84R+1lGz+vwHG+VgbjZmxiY8SMoDAbJaWBvhDg1JpfeGWaSEVfyJeqxWq62VWw47v+wDlg4AA0ggvJnomHHo7SIKhqqDJVq/M4f4yx+XjshcgGQKpsQPR5ho8ph12+KnhjzJwL3awNedFkkPGMsrzTy5c+P7tz0WXdU1O2OsAGMbUHSo/zy48TLxSbMCgptZaUXDxjsZHOca8XoeiRwcLr2bnEsApvwe53JfKEFI5jZD3emCcsHEcDb0gvSgwBk++1NYKSDhLoPZ4LTr8TFUP50hfTxMW2mZV4lMg2VaQDN+fHMOpFAr5rtBatKURNeVyIxkkkQLkYExHu4umE1J5VLuwoOkVON9gOrN5wST7WvAujSS79yGJW5HDWXzxG/Olq2d+mzM2uD3DqLra1KAr/3uDGXya0dCU27uDzyGTwjJNWvQQ77ezCIRhytj9M5FMzjuK+lNEFX6O52N0kJfX8NvIImAogvZbv+VtWYgnO/49QPiSTT6JoSSp7kR7mtXr39fofkneffi9rbUy5y5Uc/5bkN33PI7gwoH78oH9xetezVBSBsMyjaek9HAhcK1lLmif8eG76DF66kT8ANa6J6oJMHTED8Plq7U7ECybd+8CoSTRwpXaWd9TBJn7B+UW+2VA/TDP+fGAE="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":5580,"cache_read_input_tokens":73652,"cache_creation":{"ephemeral_5m_input_tokens":5580,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRtPYbUJzNrCqzALUDX","attributionAgent":"general-purpose","type":"assistant","uuid":"04f3831d-7ea1-45b5-aff5-6f65249aa8a8","timestamp":"2026-08-01T04:53:08.431Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"04f3831d-7ea1-45b5-aff5-6f65249aa8a8","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRtRhaHFvG53rgQvfCj","type":"message","role":"assistant","content":[{"type":"text","text":"Now the key experiment — does the `clippy.toml` mechanism actually work here?"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":5580,"cache_read_input_tokens":73652,"cache_creation":{"ephemeral_5m_input_tokens":5580,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRtPYbUJzNrCqzALUDX","attributionAgent":"general-purpose","type":"assistant","uuid":"cb2e6c38-de8a-4391-8a22-c59a441f7c89","timestamp":"2026-08-01T04:53:09.076Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"cb2e6c38-de8a-4391-8a22-c59a441f7c89","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRtRhaHFvG53rgQvfCj","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01AnGKp9HpznEDYNhN4LQYAM","name":"Bash","input":{"command":"set -e
W=/tmp/claude-1000/-scratch-code-bitcoin-tax/2b9d8116-e1de-4038-a101-5fd575fdcb0e/scratchpad/leprobe
rm -rf $W; mkdir -p $W/crates/core/src $W/crates/cli/src $W/crates/cli/tests
cat > $W/Cargo.toml <<'EOF'
[workspace]
members = [\"crates/core\", \"crates/cli\"]
resolver = \"2\"
EOF
cat > $W/clippy.toml <<'EOF'
disallowed-methods = [
  { path = \"le_core::printed::LineEntry::amount\", reason = \"use is_blank()/fmt(); a numeric read re-fabricates a zero\" },
]
EOF
cat > $W/crates/core/Cargo.toml <<'EOF'
[package]
name = \"le_core\"
version = \"0.1.0\"
edition = \"2021\"
[dependencies]
rust_decimal = \"1\"
EOF
cat > $W/crates/core/src/lib.rs <<'EOF'
pub mod printed {
    pub type Usd = rust_decimal::Decimal;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct LineEntry(Repr);
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    enum Repr { Entered(Usd), Blank }

    impl LineEntry {
        pub fn collected(v: Option<Usd>) -> Self {
            match v { Some(x) => LineEntry(Repr::Entered(x)), None => LineEntry(Repr::Blank) }
        }
        pub fn constant(v: Usd) -> Self { LineEntry(Repr::Entered(v)) }
        pub fn is_blank(&self) -> bool { matches!(self.0, Repr::Blank) }
        pub fn fmt(&self) -> Option<String> {
            match self.0 { Repr::Entered(v) => Some(v.to_string()), Repr::Blank => None }
        }
        /// the \"published, deliberate numeric exit\" of SPEC §6
        pub fn amount(&self) -> Option<Usd> {
            match self.0 { Repr::Entered(v) => Some(v), Repr::Blank => None }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Lines { pub line9: LineEntry, pub line34: LineEntry }
}
EOF
cat > $W/crates/cli/Cargo.toml <<'EOF'
[package]
name = \"le_cli\"
version = \"0.1.0\"
edition = \"2021\"
[dependencies]
le_core = { path = \"../core\" }
rust_decimal = \"1\"
EOF
cat > $W/crates/cli/src/lib.rs <<'EOF'
use le_core::printed::{LineEntry, Lines, Usd};

pub fn render(l: &Lines) -> String {
    // the render.rs pattern: a numeric read
    let v: Usd = l.line9.amount().unwrap_or(Usd::from(0));
    format!(\"{v}\")
}

pub fn compare(l: &Lines) -> bool {
    // render.rs:1568 `if f.line34 > Usd::ZERO`
    l.line34 > LineEntry::constant(Usd::from(0))
}
EOF
cat > $W/crates/cli/tests/t.rs <<'EOF'
use le_core::printed::{LineEntry, Usd};
#[test]
fn crossfoot() {
    let a = LineEntry::constant(Usd::from(5));
    assert_eq!(a.amount(), Some(Usd::from(5)));
}
EOF
cd $W && cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -45"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":5580,"cache_read_input_tokens":73652,"output_tokens":1745,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":5580},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":1745,"cache_read_input_tokens":73652,"cache_creation_input_tokens":5580,"cache_creation":{"ephemeral_5m_input_tokens":5580,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRtPYbUJzNrCqzALUDX","attributionAgent":"general-purpose","type":"assistant","uuid":"34c9c3fa-e3d3-4e12-b7a9-c2b18a423596","timestamp":"2026-08-01T04:53:21.572Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"34c9c3fa-e3d3-4e12-b7a9-c2b18a423596","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01AnGKp9HpznEDYNhN4LQYAM","type":"tool_result","content":"    Updating crates.io index
     Locking 72 packages to latest compatible versions
   Compiling autocfg v1.5.1
   Compiling serde_core v1.0.229
   Compiling serde v1.0.229
   Compiling rust_decimal v1.42.1
    Checking arrayvec v0.7.8
   Compiling num-traits v0.2.19
    Checking le_core v0.1.0 (/tmp/claude-1000/-scratch-code-bitcoin-tax/2b9d8116-e1de-4038-a101-5fd575fdcb0e/scratchpad/leprobe/crates/core)
    Checking le_cli v0.1.0 (/tmp/claude-1000/-scratch-code-bitcoin-tax/2b9d8116-e1de-4038-a101-5fd575fdcb0e/scratchpad/leprobe/crates/cli)
error: use of a disallowed method `le_core::printed::LineEntry::amount`
 --> crates/cli/src/lib.rs:5:26
  |
5 |     let v: Usd = l.line9.amount().unwrap_or(Usd::from(0));
  |                          ^^^^^^
  |
  = note: use is_blank()/fmt(); a numeric read re-fabricates a zero
  = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#disallowed_methods
  = note: `-D clippy::disallowed-methods` implied by `-D warnings`
  = help: to override `-D warnings` add `#[allow(clippy::disallowed_methods)]`

error: could not compile `le_cli` (lib) due to 1 previous error
warning: build failed, waiting for other jobs to finish...
error: could not compile `le_cli` (lib test) due to 1 previous error","is_error":false}]},"uuid":"79557e07-cac5-4fbf-95a1-d39f900c5899","timestamp":"2026-08-01T04:53:23.180Z","toolUseResult":{"stdout":"    Updating crates.io index
     Locking 72 packages to latest compatible versions
   Compiling autocfg v1.5.1
   Compiling serde_core v1.0.229
   Compiling serde v1.0.229
   Compiling rust_decimal v1.42.1
    Checking arrayvec v0.7.8
   Compiling num-traits v0.2.19
    Checking le_core v0.1.0 (/tmp/claude-1000/-scratch-code-bitcoin-tax/2b9d8116-e1de-4038-a101-5fd575fdcb0e/scratchpad/leprobe/crates/core)
    Checking le_cli v0.1.0 (/tmp/claude-1000/-scratch-code-bitcoin-tax/2b9d8116-e1de-4038-a101-5fd575fdcb0e/scratchpad/leprobe/crates/cli)
error: use of a disallowed method `le_core::printed::LineEntry::amount`
 --> crates/cli/src/lib.rs:5:26
  |
5 |     let v: Usd = l.line9.amount().unwrap_or(Usd::from(0));
  |                          ^^^^^^
  |
  = note: use is_blank()/fmt(); a numeric read re-fabricates a zero
  = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#disallowed_methods
  = note: `-D clippy::disallowed-methods` implied by `-D warnings`
  = help: to override `-D warnings` add `#[allow(clippy::disallowed_methods)]`

error: could not compile `le_cli` (lib) due to 1 previous error
warning: build failed, waiting for other jobs to finish...
error: could not compile `le_cli` (lib test) due to 1 previous error","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"34c9c3fa-e3d3-4e12-b7a9-c2b18a423596","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"79557e07-cac5-4fbf-95a1-d39f900c5899","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRuuNznuWs2xJEebfCS","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISkhgKhwEIEBgCKkA+PgeCJStnuDpZrOALj2sFziLTZXdeotpbRwj90ssdiRb6IUOxmI6DOIFIxNfsTPHy4kzqwU1MW2N9DvVM5oSMMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDJCTeC+ig4tHi2hSBxoMPgXAgtHHhU3PUsjHIjCtgzk5cxNTWV7aITr5sqnGome1/TFXYjpSrpI5RZytJ6n1NuPNVjULQarGdcmZa/oqtxbyKrn8kyNKd/lWHl2HzeJdpCHpJrM9QuC7ADuXYdjU83QGEUbVc0E3jKSPd1rsRZ7iD6wU63XrnNbE1zkT9+8pfs45c3u9gjKqSQMq3AzVaXdpsgBTOKKdBvuSmhaEM6LZvZSKpmzGkG9VbevFn4uslthL/TNbS5v9GGhmq8uyg5HNk51NImtoin8LFa5iq2A4TZfS5BAyEk+dO0jTCyTJ80hvV1Q3Zp7PXBgUa7y2/oW/pHwCB5hgUXG/J7TzPlMinv8XFzMFc7YkIyMIU2QBxIZdB2eaPOxttPLenVxqzvGSzZvHcwaOIRJQqOq+RU4iHGv8uvbC4bQpxUCwJIchiaBYW8vctFz3LYVoxqv+Rpbd7lU7G2hF7QHf3K8efiO9izOOESxDdYKYaxiScvH0I4+nwccxY5VLATHEDfdm67WeCFo9o1nA0/PtZvhMuUUVAGR1EVtjDiO5xeaJcamM6Kkv3vQEvqS52lPIH7Noge+ggQJnD1WDGP/MEwmPHsK+5MjZO4xRWP4rUvCsJpeTRoJFnAllTN2q8nBqPYFGafVALigenhl+deJvsLjLBuR6cTBMVGwMW4vgNCnTPXUVGGXRLWhtZ64cz4smOZDU69QUqw8UeTgY0qEyJaAVjBeO7nT9YYp1A6FguD3jPupPFlB4A93Zrz2bbbgde9APwNQj5nq51A763fSOoHcUqaj+w4hY7hI+tEgJ+MaZsWVqI50sWPhjxMfAizR07sSnj6Rg3TiSnjnpFtazDIUkglRVXEn/p9Wy5cnTvMTHFlP3P6Iy0Pl2meQVFGBcc4lraN7AqFaH+dSxfy3vC8N8KM09DGb9fG7RULhvJudywb1qZ1f29HYWNAaiTL67BHL4dGp9VDxyXvWkPNUBspWzzzIROIiNksgIXUNOiO1ug9L/c+jfvV5TOwJN33kZ/q8UyjEW91XkOYdcpdY1YAZef8VX9wZkVEEwOzyMzKG6z21fAg8Gf3xJMgj+HRTWsi2rUbj9M3pwbtmIsP2QNSWQ7NkHKcoWo3mtOv514/KHdUhaA3SUOm7rUHNmf+4gA7QOfn4V2GM+XpJpKEx5tdQJ0WuefLPpClAc2LeGelvXcfkkw0cvlNyEC3QF2tNsAs/8Akq9NzEKfHT2VBZS5r/OAF8aIhJDTZeFmAuPly7Q0Tqqp6iOIAAzFiaOdePAnTQSA2+C6uCIiwkeuwh8GHYWv3YoKYqAm9F4yOLtq+XmoeFWAwJ6+6Q3hE0PoL78cQOTRP0pDaCRpYPD0nO2HJGlbcq2zqLcokuFcc1maetl8qF4F/sWxqbfNxgFSToIw8tlYBhgvtjLmePkzQxr+06GZHU5K0tHWx8AShAApUoPCN762VdodXn1Zvq8QuXRBXexNrNgcYw4paIDjKJ8+Ka6zuAqwjr2HJpqeX6WVfAyLvKZOofoFphCv6Bm+Yz3nM/A64L+1L2znNkmtr09n7W/RWDKducMR+pcp4OPWjw9GcWYLzGuOZxW3+1aHomlA4QMesaaN9X7oAnyFSdVmsC6cHuENMBiocRFz9ywoQc3bEOVFTJug0imscXvzpo49WySd/5r+CtAkcAJEucOqVVCWDVFeC6DWbyd40lwZJEDyprn433SJDKBpWM9inzZXcb0FrHBavPdERQLzlEgS+dR+OTMlESkhrq5bN0PyyDZbYK+vLp2/PkVpsaChv5dJ9Q45ZMGXAWZHPO3lnvHglGAmYi9qT8w1G9Ffi8tuIsIgIpFl7RtW+FcMzY6dTHa76ErjnfOONEfO/zWRSQdrue+7KOUG8o+ne2MgjBCIT3aUWe1ahXYwfqG/A85vza1Hxa/1yP5KYCPcQo754GkndJqnGuMXSD2DU6wF9sq73Vwr3paygdqAd1OxVlZqdrHg5ZuTe4SgZbunxONRZ6z59PuTOtMjYaNh9VMWaDiDdixOTmzOTxzBbFa3px6yLwcAg0gQSQ6vr1J0GQdogehJ1jGJY98PvYYpxvD3m8jjvtjYK8UjVzg7DYkvzeOpGWbSD1ZZF/g+JZGWejpj84R7qmZXKZ2Mvp4T7CYtJ3oUwvCfEv9zcb6FDWlOs84YOHKSZENP8QczZi+cRzYc1nkSNqyX1cLm7V01gx3gerhlJocOcZIPlyC7CSrevdVayBmeIH30srqk6F3+8p6v/zJhXwoLtGPowGyJlHvLDrtOgrZiOml718JHinueWIFvvMnKfwg72KmCh+gFJnbASUbdXjFpvNBDFy2L48AmmilQoAvBiewgCVaTF6qrhtpR27hQS7SJHuDM+ChzMomJUImq1gyHhH1lXnch0ueABgZ5U5HudAdLaW46bFOwMsf5n0baVfxM3k0Vn3RdfpN0SessusRF8oq5ViUVAHJEKG1sjVrJ900XHYWUJV315S/Qm1Gbq038CSVeQ0uqz//i/aQa/ANTbsYGu7bQSOO3y0cGHkI+mU8p8f+s4Ze47xSvuzJqvRnB4WwK6pz7mat8BWXVeOTbXf3hHVLcOnwKMFarN+ttoJGlWXHfJn1XJNvv5NQ1uNTsaSQIbfH/oyiMPfS57rIvAuCUEr1EvkbWzCoEZLagBNJK0hgwWF3wMjOjnlOWOJquxeXIVuhUyAR735qgJCBTtFkH5XT1uMLTuHRKYHhE2i4wy7VLixmo/yLHGsRFHfM1vbLnxgzYnV0Bl4byiO9kEWWlkZ4RF9CZiGpn+L54OFE7JTyA/lNZZuGD3Mx0s5FCBiatEhbxPHun69TqNlaFIzLdmoZrjjRC/kBJdbll/L/v9uvg8DBKOqVV49suiy7soLOqYSzM0YnYjWVZaGrDvVP/7eISiwCJNFS0y2fRQwqHK/4/76qrzXZK32Na7cJDDZ4wXoKMz47lQyeFV4z9Pe9nKqsYNucECqOuOIIW0PEib9g/4xbzSOEfxk9Z2ysWylT5dj03RGW+GWKP/s65/gPkywnpODqoBGNgjGzlXURYubmL7Eed5dJB942/lQT6ns4xyie/waYx0s2Tp1FxMAmjnSWhjJ4ewPHTNeEmo6g8lSct0uAwZjlA7ar/P701EU7oT+F2EJpQky8dIMPeyOVMMIsBhe7x7lZLuCPD6Lvwi26c87g7W2O+Fgm/MQJmSIrHabViYqiviDbitVi0/bQgvZLKo7jGH5eTpEOFLBNsCen18Z1GDpnFZrCtHMAcfabYTta/EeWsPaPHqXGGia5GN70Km/rKLhSgks8og7dRgNSxx3Huc6cY/VGzSZCQFbwxWAxej4UXGXosklppM1zKehMSQjVjmqJw4yF16SpFat23HyZEK0I/J11vDGiAsOJljd67J2bwkvOwfqDAknNSIaHvaF2E/dMm0L3NRPUIqQye+HFRh5WTqj9lcT61xsR8b6vVxE3s3KDI4E4crjFC33uOyFQBVeUQ/251MSvBGhZEUZTHpSTsvl7NWRp3K6M8msR7L7z6K5FlclAt2uPYnWh2xFFZI+vklTQcwaXI9iTZZFcr8owszVZtkrbvvZaPYWQme2J4TVR2rMjxFdBxRo+Xyr2iH2E8hh5GSTGZ+yLmreWyswt8syK/PjVXxzycSswtMUvpvx0pbNkWB8rol249hGs9RX17tVsdyshZPC0pZO6ARg4Rb1rbcdrxC4f8jkBVh1aXEUQXuq3HG+By7LuBnZDEjqUVvzTA8WVSZGSWHUMRSPGN7degWkWg9MvQlT6lbmkQcf2yRhrcMXYCkrwaXzL9zPmn1YyEZxLGErIwKgCKGbX9dh/cHT2INjNjYtCCx7gAnkyl/21QHeMwIeL1jsuFai5aXi7SfQg5GhmgcW6axR0+CNVHuAMCHnvkl8aJjIYAQ=="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2017,"cache_read_input_tokens":79575,"cache_creation":{"ephemeral_5m_input_tokens":2017,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRutKkFXUEGVtMqH2bP","attributionAgent":"general-purpose","type":"assistant","uuid":"f7125aa8-2117-453f-870b-eab0c8fb2454","timestamp":"2026-08-01T04:53:40.233Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"f7125aa8-2117-453f-870b-eab0c8fb2454","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRuuNznuWs2xJEebfCS","type":"message","role":"assistant","content":[{"type":"text","text":"The mechanism works. Now the semantics of the published comparator."}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2017,"cache_read_input_tokens":79575,"cache_creation":{"ephemeral_5m_input_tokens":2017,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRutKkFXUEGVtMqH2bP","attributionAgent":"general-purpose","type":"assistant","uuid":"17c31a04-f940-4d0a-945a-73224e1b8af7","timestamp":"2026-08-01T04:53:40.701Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"17c31a04-f940-4d0a-945a-73224e1b8af7","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRuuNznuWs2xJEebfCS","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01PWDUac2t7mLK1YECECFhGU","name":"Bash","input":{"command":"set -e
W=/tmp/claude-1000/-scratch-code-bitcoin-tax/2b9d8116-e1de-4038-a101-5fd575fdcb0e/scratchpad/leprobe
cat > $W/crates/cli/src/lib.rs <<'EOF'
use le_core::printed::{LineEntry, Lines, Usd};
pub fn compare(l: &Lines) -> bool { l.line34 > LineEntry::constant(Usd::from(0)) }
EOF
mkdir -p $W/crates/cli/src/bin
cat > $W/crates/cli/src/bin/probe.rs <<'EOF'
use le_core::printed::{LineEntry, Usd};
fn main() {
    let blank  = LineEntry::collected(None);
    let zero   = LineEntry::constant(Usd::from(0));
    let neg    = LineEntry::constant(Usd::from(-5));
    println!(\"blank > entered(0)   = {}   <-- render.rs:1568 routes REFUND\", blank > zero);
    println!(\"blank < entered(0)   = {}\", blank < zero);
    println!(\"blank == entered(0)  = {}   <-- schedule_se_full.rs:78 skip predicate\", blank == zero);
    println!(\"blank > entered(-5)  = {}\", blank > neg);
    println!(\"blank.cmp(zero)      = {:?}\", blank.cmp(&zero));
}
EOF
cd $W && cargo run -q --bin probe 2>&1 | tail -10
echo \"--- clippy on TEST targets only ---\"
cd $W && cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | grep -E \"disallowed|-->|could not compile\" | head -10"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2017,"cache_read_input_tokens":79575,"output_tokens":1820,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":2017},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":1820,"cache_read_input_tokens":79575,"cache_creation_input_tokens":2017,"cache_creation":{"ephemeral_5m_input_tokens":2017,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRutKkFXUEGVtMqH2bP","attributionAgent":"general-purpose","type":"assistant","uuid":"b6e56bbf-e535-4efa-a269-28fb6da19474","timestamp":"2026-08-01T04:53:46.046Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"b6e56bbf-e535-4efa-a269-28fb6da19474","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01PWDUac2t7mLK1YECECFhGU","type":"tool_result","content":"blank > entered(0)   = true   <-- render.rs:1568 routes REFUND
blank < entered(0)   = false
blank == entered(0)  = false   <-- schedule_se_full.rs:78 skip predicate
blank > entered(-5)  = true
blank.cmp(zero)      = Greater
--- clippy on TEST targets only ---
error: use of a disallowed method `le_core::printed::LineEntry::amount`
 --> crates/cli/tests/t.rs:5:18
  = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#disallowed_methods
  = note: `-D clippy::disallowed-methods` implied by `-D warnings`
  = help: to override `-D warnings` add `#[allow(clippy::disallowed_methods)]`
error: could not compile `le_cli` (test \"t\") due to 1 previous error","is_error":false}]},"uuid":"174df63c-c09c-4d3f-a0e8-e2933a855072","timestamp":"2026-08-01T04:53:47.395Z","toolUseResult":{"stdout":"blank > entered(0)   = true   <-- render.rs:1568 routes REFUND
blank < entered(0)   = false
blank == entered(0)  = false   <-- schedule_se_full.rs:78 skip predicate
blank > entered(-5)  = true
blank.cmp(zero)      = Greater
--- clippy on TEST targets only ---
error: use of a disallowed method `le_core::printed::LineEntry::amount`
 --> crates/cli/tests/t.rs:5:18
  = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#disallowed_methods
  = note: `-D clippy::disallowed-methods` implied by `-D warnings`
  = help: to override `-D warnings` add `#[allow(clippy::disallowed_methods)]`
error: could not compile `le_cli` (test \"t\") due to 1 previous error","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"b6e56bbf-e535-4efa-a269-28fb6da19474","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"174df63c-c09c-4d3f-a0e8-e2933a855072","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRwhywkCzQSeKWbJWLH","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAIS8AQKhwEIEBgCKkAOixXK9T7v6h7Ofo02B1bhA4CMYYC8pmCAzjxraHRYdGUm/JuODgHkAFbIblAehhLPSGvoVEY8islVtiz1WNcTMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDIQ25xbC1cMkn7w5qBoMYWRM8gMF9Z2DLrWIIjDZYAQmAXFZx57T9hKRDuAG5b4BqL3GBas+RCKLI7z2d5dHX1+nemapLZvnmZRstNQqlQPGoVLTbUv1JZGc8eZvt32mMtupLbGRQMR1IxbVsF34rVhhHrTMzPGmtWAXHUNseGEdAgkLcJR+hipLgMoIqt7tA0yr4cRvLp+K+DLA0dvpcLTzYhdmMsSY78M5QAwIdKRlL7Vo5KMCpF6a7bwKLKjteVuQflXIvuynFzmaKrgtQP/9tiCjS/cBUfY28mLm5/6JkM/Y07ovnGqxbRHjUos74p8Hkn926pIFTpLZ3ark94MgPE7RsHmUv9Ni2WwGZ1CsRDWCskEQGG1GKNMM61m88/MiCydpHvg8DuAY6VST4fLSllbfK4ppSj0RoyMLD+M8W2/sC3ty2jWpWFrVfVeM2sR7Z30KPyjH5Ltz6QmcJ6NYvS8gWAzblQRZNA1DPj9MKumMctS5Zag/S5s1yX30NB+aJr8kJW5rKuuwifpsZRTOZPIaRK8IZmleV6fTU/iFuTDeQ13vXLOY+mhVQ5GzOd7Zfxrx58CjMCZYtS7OfLXaw3yzAv5AYOcijSorkOZJ1NCOsukoiclTQUWmfb7WmKHex/0YAQ=="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":949,"cache_read_input_tokens":82769,"cache_creation":{"ephemeral_5m_input_tokens":949,"ephemeral_1h_input_tokens":0},"output_tokens":10,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRwfmkJtL56gatcm6ad","attributionAgent":"general-purpose","type":"assistant","uuid":"18a3e62f-347c-4797-b9fb-09025e88b302","timestamp":"2026-08-01T04:53:51.440Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"18a3e62f-347c-4797-b9fb-09025e88b302","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRwhywkCzQSeKWbJWLH","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01LaDvdusuLQrJG442Y6gamz","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax
echo \"=== schedule_se_full.rs 70-100 ===\"; sed -n '70,100p' crates/btctax-forms/src/schedule_se_full.rs
echo \"=== form8995.rs 158-215 ===\"; sed -n '158,215p' crates/btctax-forms/src/form8995.rs
echo \"=== schedule_d_full.rs 50-75 ===\"; sed -n '50,75p' crates/btctax-forms/src/schedule_d_full.rs"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":949,"cache_read_input_tokens":82769,"output_tokens":384,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":949},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":384,"cache_read_input_tokens":82769,"cache_creation_input_tokens":949,"cache_creation":{"ephemeral_5m_input_tokens":949,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRwfmkJtL56gatcm6ad","attributionAgent":"general-purpose","type":"assistant","uuid":"d8b37140-5b4c-4681-a6bc-69b56181e4c3","timestamp":"2026-08-01T04:53:53.020Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"d8b37140-5b4c-4681-a6bc-69b56181e4c3","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01LaDvdusuLQrJG442Y6gamz","type":"tool_result","content":"=== schedule_se_full.rs 70-100 ===
        &mut placements,
        identity,
        &proprietor.full_name(),
        &proprietor.ssn,
        &blank_fields,
    )?;

    // The SS band is exhausted when W-2 SS wages already reach the base ⇒ lines 8d/9/10 are not filled.
    let skip_8d_to_10 = lines.line9 == Usd::ZERO;

    // Parallel to `map.lines()` — printed reading order, strictly descending y.
    let plan: [(Usd, usize, bool); 12] = [
        (lines.line2, SE_COL_AMOUNT, true), // 2  ← Schedule C's printed line 31
        (lines.line3, SE_COL_AMOUNT, true), // 3
        (lines.line4a, SE_COL_AMOUNT, true), // 4a
        (lines.line4c, SE_COL_AMOUNT, true), // 4c
        (lines.line6, SE_COL_AMOUNT, true), // 6  ← cited by Form 8959 line 8
        (lines.line8a, SE_COL_MID, true),   // 8a
        (lines.line8d, SE_COL_AMOUNT, !skip_8d_to_10), // 8d
        (lines.line9, SE_COL_AMOUNT, !skip_8d_to_10), // 9
        (lines.line10, SE_COL_AMOUNT, !skip_8d_to_10), // 10
        (lines.line11, SE_COL_AMOUNT, true), // 11
        (lines.line12, SE_COL_AMOUNT, true), // 12 → Schedule 2 line 4
        (lines.line13, SE_COL_MID, true),   // 13 → Schedule 1 line 15
    ];
    for (ord, (cell, (value, col, include))) in map.lines().iter().zip(plan).enumerate() {
        if !include {
            continue;
        }
        push_money(
            &mut writes,
=== form8995.rs 158-215 ===
    //
    // ★ Keyed off the QBI (line 2), NOT off the business NAME (Fable P7 r2 I2). Gating on the name made
    // the whole defect conditional on an unvalidated free-text field: `business_description` is
    // `#[serde(default)]`, so an import that omitted it produced a blank name — and therefore a blank
    // row — under a NON-ZERO line 2. That is precisely the defect r1 I1 raised, re-created. Core now
    // REFUSES an unnamed Schedule C (`ScheduleCNoBusinessDescription`), and this fails closed if one
    // ever reaches here anyway: a form claiming a deduction for a business it cannot name must not be
    // produced at all.
    if lines.line2 > Usd::ZERO {
        if lines.business_name.trim().is_empty() {
            return Err(FormsError::Geometry(
                \"Form 8995 line 2 is non-zero but the trade or business has no name: line 2 is \\\"Total \\
                 qualified business income or (loss). Combine lines 1i through 1v, column (c)\\\", so this \\
                 would file a total over an EMPTY column, claiming a §199A deduction for a business the \\
                 return never names.\"
                    .into(),
            ));
        }
        // (b) the business's TIN. ★ The PROPRIETOR's, not the return's primary taxpayer's (Fable P7 r2
        // I1). A spouse-owned business files under the SPOUSE's name and SSN even on a joint return —
        // Schedule C and Schedule SE already do this via `header.proprietor`, and an 8995 reporting a
        // business TIN that matches no Schedule C in the same packet is exactly the mismatch IRS
        // matching flags. A sole proprietor's TIN is their own SSN; btctax has no EIN input.
        let proprietor = header.proprietor.as_ref().ok_or_else(|| {
            FormsError::Geometry(
                \"Form 8995 has trade-or-business QBI but the return names no proprietor to file the \\
                 business under\"
                    .into(),
            )
        })?;
        push_literal(
            &mut writes,
            &mut placements,
            &map.row1_business,
            &lines.business_name,
            F8995_COL_ROW1_BUSINESS,
        );
        // ★ Rendered through `render_ssn` against the cell's OWN /MaxLen, not hardcoded (Fable P7 r3,
        // Minor). Every other SSN cell in this crate goes through that guard, and it fails closed on a
        // cell too narrow to hold an SSN — \"the map points at the wrong widget\". TY2024's row 1i(b) has
        // /MaxLen 11 (hyphenated), but hardcoding that would silently write 11 characters into a 9-char
        // comb if a later revision narrows the cell, while every other SSN in the packet failed closed.
        let tin_fqn = map.row1_tin.fields()[0];
        let tin_max_len = blank_fields
            .iter()
            .find(|f| f.fqn == tin_fqn)
            .ok_or_else(|| FormsError::MapFieldMissing(tin_fqn.to_string()))?
            .max_len;
        push_literal(
            &mut writes,
            &mut placements,
            &map.row1_tin,
            &render_ssn(&proprietor.ssn, tin_max_len)?,
            F8995_COL_ROW1_TIN,
        );
        // (c) this business's QBI. With one business the column total IS the row, so it is written from
        // the same value as line 2 — they cannot disagree.
        push_money(
=== schedule_d_full.rs 50-75 ===
const GRP_P2_AMOUNT: u32 = 3;

/// Fail closed if a parenthesized cell carries a negative. On the printed form the parentheses supply
/// the minus sign, so a negative would RENDER AS POSITIVE — turning a capital LOSS carryover into a
/// gain. Invisible to any geometric check; hence an explicit guard.
fn assert_paren_magnitudes(lines: &ScheduleDLines) -> Result<(), FormsError> {
    let mut cells = vec![(\"6\", lines.line6), (\"14\", lines.line14)];
    if let ScheduleDRouting::NetLoss { line21, .. } = lines.routing {
        cells.push((\"21\", line21));
    }
    for (line, v) in cells {
        if v < Usd::ZERO {
            return Err(FormsError::Geometry(format!(
                \"Schedule D line {line} is a PARENTHESIZED box (the form prints the minus sign), so it \\
                 must carry a positive magnitude — got {v}. Writing this would render as a POSITIVE \\
                 number on the filed return, turning a capital loss into a gain.\"
            )));
        }
    }
    Ok(())
}

/// A map cell the full fill requires but the crypto-slice maps (2017/2025) do not carry.
fn need<'a, T>(cell: &'a Option<T>, what: &str, year: i32) -> Result<&'a T, FormsError> {
    cell.as_ref().ok_or_else(|| {
        FormsError::Geometry(format!(","is_error":false}]},"uuid":"80266bc8-100e-4026-bba1-2fc510638e9b","timestamp":"2026-08-01T04:53:53.117Z","toolUseResult":{"stdout":"=== schedule_se_full.rs 70-100 ===
        &mut placements,
        identity,
        &proprietor.full_name(),
        &proprietor.ssn,
        &blank_fields,
    )?;

    // The SS band is exhausted when W-2 SS wages already reach the base ⇒ lines 8d/9/10 are not filled.
    let skip_8d_to_10 = lines.line9 == Usd::ZERO;

    // Parallel to `map.lines()` — printed reading order, strictly descending y.
    let plan: [(Usd, usize, bool); 12] = [
        (lines.line2, SE_COL_AMOUNT, true), // 2  ← Schedule C's printed line 31
        (lines.line3, SE_COL_AMOUNT, true), // 3
        (lines.line4a, SE_COL_AMOUNT, true), // 4a
        (lines.line4c, SE_COL_AMOUNT, true), // 4c
        (lines.line6, SE_COL_AMOUNT, true), // 6  ← cited by Form 8959 line 8
        (lines.line8a, SE_COL_MID, true),   // 8a
        (lines.line8d, SE_COL_AMOUNT, !skip_8d_to_10), // 8d
        (lines.line9, SE_COL_AMOUNT, !skip_8d_to_10), // 9
        (lines.line10, SE_COL_AMOUNT, !skip_8d_to_10), // 10
        (lines.line11, SE_COL_AMOUNT, true), // 11
        (lines.line12, SE_COL_AMOUNT, true), // 12 → Schedule 2 line 4
        (lines.line13, SE_COL_MID, true),   // 13 → Schedule 1 line 15
    ];
    for (ord, (cell, (value, col, include))) in map.lines().iter().zip(plan).enumerate() {
        if !include {
            continue;
        }
        push_money(
            &mut writes,
=== form8995.rs 158-215 ===
    //
    // ★ Keyed off the QBI (line 2), NOT off the business NAME (Fable P7 r2 I2). Gating on the name made
    // the whole defect conditional on an unvalidated free-text field: `business_description` is
    // `#[serde(default)]`, so an import that omitted it produced a blank name — and therefore a blank
    // row — under a NON-ZERO line 2. That is precisely the defect r1 I1 raised, re-created. Core now
    // REFUSES an unnamed Schedule C (`ScheduleCNoBusinessDescription`), and this fails closed if one
    // ever reaches here anyway: a form claiming a deduction for a business it cannot name must not be
    // produced at all.
    if lines.line2 > Usd::ZERO {
        if lines.business_name.trim().is_empty() {
            return Err(FormsError::Geometry(
                \"Form 8995 line 2 is non-zero but the trade or business has no name: line 2 is \\\"Total \\
                 qualified business income or (loss). Combine lines 1i through 1v, column (c)\\\", so this \\
                 would file a total over an EMPTY column, claiming a §199A deduction for a business the \\
                 return never names.\"
                    .into(),
            ));
        }
        // (b) the business's TIN. ★ The PROPRIETOR's, not the return's primary taxpayer's (Fable P7 r2
        // I1). A spouse-owned business files under the SPOUSE's name and SSN even on a joint return —
        // Schedule C and Schedule SE already do this via `header.proprietor`, and an 8995 reporting a
        // business TIN that matches no Schedule C in the same packet is exactly the mismatch IRS
        // matching flags. A sole proprietor's TIN is their own SSN; btctax has no EIN input.
        let proprietor = header.proprietor.as_ref().ok_or_else(|| {
            FormsError::Geometry(
                \"Form 8995 has trade-or-business QBI but the return names no proprietor to file the \\
                 business under\"
                    .into(),
            )
        })?;
        push_literal(
            &mut writes,
            &mut placements,
            &map.row1_business,
            &lines.business_name,
            F8995_COL_ROW1_BUSINESS,
        );
        // ★ Rendered through `render_ssn` against the cell's OWN /MaxLen, not hardcoded (Fable P7 r3,
        // Minor). Every other SSN cell in this crate goes through that guard, and it fails closed on a
        // cell too narrow to hold an SSN — \"the map points at the wrong widget\". TY2024's row 1i(b) has
        // /MaxLen 11 (hyphenated), but hardcoding that would silently write 11 characters into a 9-char
        // comb if a later revision narrows the cell, while every other SSN in the packet failed closed.
        let tin_fqn = map.row1_tin.fields()[0];
        let tin_max_len = blank_fields
            .iter()
            .find(|f| f.fqn == tin_fqn)
            .ok_or_else(|| FormsError::MapFieldMissing(tin_fqn.to_string()))?
            .max_len;
        push_literal(
            &mut writes,
            &mut placements,
            &map.row1_tin,
            &render_ssn(&proprietor.ssn, tin_max_len)?,
            F8995_COL_ROW1_TIN,
        );
        // (c) this business's QBI. With one business the column total IS the row, so it is written from
        // the same value as line 2 — they cannot disagree.
        push_money(
=== schedule_d_full.rs 50-75 ===
const GRP_P2_AMOUNT: u32 = 3;

/// Fail closed if a parenthesized cell carries a negative. On the printed form the parentheses supply
/// the minus sign, so a negative would RENDER AS POSITIVE — turning a capital LOSS carryover into a
/// gain. Invisible to any geometric check; hence an explicit guard.
fn assert_paren_magnitudes(lines: &ScheduleDLines) -> Result<(), FormsError> {
    let mut cells = vec![(\"6\", lines.line6), (\"14\", lines.line14)];
    if let ScheduleDRouting::NetLoss { line21, .. } = lines.routing {
        cells.push((\"21\", line21));
    }
    for (line, v) in cells {
        if v < Usd::ZERO {
            return Err(FormsError::Geometry(format!(
                \"Schedule D line {line} is a PARENTHESIZED box (the form prints the minus sign), so it \\
                 must carry a positive magnitude — got {v}. Writing this would render as a POSITIVE \\
                 number on the filed return, turning a capital loss into a gain.\"
            )));
        }
    }
    Ok(())
}

/// A map cell the full fill requires but the crypto-slice maps (2017/2025) do not carry.
fn need<'a, T>(cell: &'a Option<T>, what: &str, year: i32) -> Result<&'a T, FormsError> {
    cell.as_ref().ok_or_else(|| {
        FormsError::Geometry(format!(","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"d8b37140-5b4c-4681-a6bc-69b56181e4c3","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"80266bc8-100e-4026-bba1-2fc510638e9b","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRx7kSccJ9eGqP1ygED","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"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"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2850,"cache_read_input_tokens":83718,"cache_creation":{"ephemeral_5m_input_tokens":2850,"ephemeral_1h_input_tokens":0},"output_tokens":5,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRx6LMuAbPyviU5fz8g","attributionAgent":"general-purpose","type":"assistant","uuid":"c4480438-dafa-494c-a540-209a57f2a8f7","timestamp":"2026-08-01T04:54:07.296Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"c4480438-dafa-494c-a540-209a57f2a8f7","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRx7kSccJ9eGqP1ygED","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01TxHhQiTKvGuMvbkUHVFAzs","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax
echo \"=== Form6251 inherent methods (arith/compare on own lines) ===\"; sed -n '239,264p' crates/btctax-core/src/tax/form6251.rs
echo; echo \"=== harness main.rs 315-335 ===\"; sed -n '315,335p' crates/btctax-oracle-harness/src/main.rs
echo; echo \"=== render.rs comparison/arith on printed fields ===\"
grep -rn \"f1040\\.\\|f8959\\.\\|f8960\\.\\|\\.amt\\.line\\|sch_1\\.\\|sch_2\\.\\|sch_3\\.\\|f8995\\.\" crates/btctax-cli/src/render.rs | grep -E \"[<>]|==|\\+|\\-|map_or\" | head -20"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2850,"cache_read_input_tokens":83718,"output_tokens":1237,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":2850},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":1237,"cache_read_input_tokens":83718,"cache_creation_input_tokens":2850,"cache_creation":{"ephemeral_5m_input_tokens":2850,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRx6LMuAbPyviU5fz8g","attributionAgent":"general-purpose","type":"assistant","uuid":"750b07a9-3d55-4ff9-952e-96f0f92aceda","timestamp":"2026-08-01T04:54:10.359Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"750b07a9-3d55-4ff9-952e-96f0f92aceda","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01TxHhQiTKvGuMvbkUHVFAzs","type":"tool_result","content":"=== Form6251 inherent methods (arith/compare on own lines) ===
impl Form6251 {
    /// **Who Must File, condition 1** (i6251 p.1): *\"Form 6251, line 7, is greater than line 10.\"*
    ///
    /// This — not `amt() > 0` — is the attach test. When line 7 exceeds line 10 the AMTFTC is figured
    /// (line 8), so line 9 = line 7 − FTC can still land at or below line 10: **the form is required
    /// while the AMT is $0.** Conditions 2–4 are unreachable in v1 (no general business credit, no
    /// Form 8834/8911/8801 input; condition 4 needs lines 2c–3 negative, and every one of those lines
    /// is 0 or refused).
    pub fn must_attach(&self) -> bool {
        self.line7 > self.line10
    }

    /// The AMT itself — Form 6251 line 11 → Schedule 2 (Form 1040), line 2.
    pub fn amt(&self) -> Usd {
        self.line11
    }

    /// Tentative minimum tax — Form 6251 line **9** (after the line-8 AMTFTC subtraction), which is
    /// what the form labels \"Tentative minimum tax\". Line 7 is the pre-credit figure.
    pub fn tentative_minimum_tax(&self) -> Usd {
        self.line9
    }
}

/// Everything Form 6251 reads off the rest of the return.
#[derive(Debug, Clone, Copy)]

=== harness main.rs 315-335 ===
        _ => round_leaf(e.additional_medicare_tax),
    };

    // ── TOTAL TAX L24 — OTS single-witness cross-foot that inherits SE-L12 / 8959-L18:
    //    `round_leaf(L16) + SE-L12 + 8959-L18 + round_leaf(NIIT)`. line17 (AMT/APTC) and line21
    //    (credits) are read as the precondition (must be 0 for an admitted scenario) and echoed. ────────
    // L16 leg = btctax's OWN FILED L16 (the value summed into printed L24), not the oracle's L16 — the L16
    // VALUE is adjudicated separately by verdict_l16 (with its provenance/methodology class). Keeps L24
    // reconciled on the §5.1 pinned cells while still catching a real cross-foot / Sch-2-leg / L16 bug.
    let l24_target = pr.forms.f1040.line16 + se_l12_ots + f8959_l18_ots + round_leaf(e.niit);
    let mut l24 = verdict_ots(
        \"1040.line24\",
        \"TOTAL TAX (L24)\",
        paper(\"1040.line24\"),
        pr.forms.f1040.line24,
        l24_target,
    );
    if let Value::Object(m) = &mut l24 {
        m.insert(
            \"precondition_line17\".into(),
            json!(paper(\"1040.line17\").unwrap_or(0)),

=== render.rs comparison/arith on printed fields ===
crates/btctax-cli/src/render.rs:1500:            fmt_money(printed.sch_3.map_or(Usd::ZERO, |s3| s3.line1))
crates/btctax-cli/src/render.rs:1507:            fmt_money(printed.sch_2.map_or(Usd::ZERO, |s2| s2.line4))
crates/btctax-cli/src/render.rs:1514:            fmt_money(printed.f8959.line18)
crates/btctax-cli/src/render.rs:1521:            fmt_money(printed.f8960.map_or(Usd::ZERO, |f| f.line17))
crates/btctax-cli/src/render.rs:1541:    if ar.amt.line6 > Usd::ZERO {
crates/btctax-cli/src/render.rs:1547:            fmt_money(ar.amt.line11),
crates/btctax-cli/src/render.rs:1548:            fmt_money(ar.amt.line4),
crates/btctax-cli/src/render.rs:1549:            fmt_money(ar.amt.line5),
crates/btctax-cli/src/render.rs:1550:            fmt_money(ar.amt.line9),
crates/btctax-cli/src/render.rs:1551:            fmt_money(ar.amt.line10),","is_error":false}]},"uuid":"0a294027-384d-4cf3-9807-2d74535b1b3e","timestamp":"2026-08-01T04:54:10.477Z","toolUseResult":{"stdout":"=== Form6251 inherent methods (arith/compare on own lines) ===
impl Form6251 {
    /// **Who Must File, condition 1** (i6251 p.1): *\"Form 6251, line 7, is greater than line 10.\"*
    ///
    /// This — not `amt() > 0` — is the attach test. When line 7 exceeds line 10 the AMTFTC is figured
    /// (line 8), so line 9 = line 7 − FTC can still land at or below line 10: **the form is required
    /// while the AMT is $0.** Conditions 2–4 are unreachable in v1 (no general business credit, no
    /// Form 8834/8911/8801 input; condition 4 needs lines 2c–3 negative, and every one of those lines
    /// is 0 or refused).
    pub fn must_attach(&self) -> bool {
        self.line7 > self.line10
    }

    /// The AMT itself — Form 6251 line 11 → Schedule 2 (Form 1040), line 2.
    pub fn amt(&self) -> Usd {
        self.line11
    }

    /// Tentative minimum tax — Form 6251 line **9** (after the line-8 AMTFTC subtraction), which is
    /// what the form labels \"Tentative minimum tax\". Line 7 is the pre-credit figure.
    pub fn tentative_minimum_tax(&self) -> Usd {
        self.line9
    }
}

/// Everything Form 6251 reads off the rest of the return.
#[derive(Debug, Clone, Copy)]

=== harness main.rs 315-335 ===
        _ => round_leaf(e.additional_medicare_tax),
    };

    // ── TOTAL TAX L24 — OTS single-witness cross-foot that inherits SE-L12 / 8959-L18:
    //    `round_leaf(L16) + SE-L12 + 8959-L18 + round_leaf(NIIT)`. line17 (AMT/APTC) and line21
    //    (credits) are read as the precondition (must be 0 for an admitted scenario) and echoed. ────────
    // L16 leg = btctax's OWN FILED L16 (the value summed into printed L24), not the oracle's L16 — the L16
    // VALUE is adjudicated separately by verdict_l16 (with its provenance/methodology class). Keeps L24
    // reconciled on the §5.1 pinned cells while still catching a real cross-foot / Sch-2-leg / L16 bug.
    let l24_target = pr.forms.f1040.line16 + se_l12_ots + f8959_l18_ots + round_leaf(e.niit);
    let mut l24 = verdict_ots(
        \"1040.line24\",
        \"TOTAL TAX (L24)\",
        paper(\"1040.line24\"),
        pr.forms.f1040.line24,
        l24_target,
    );
    if let Value::Object(m) = &mut l24 {
        m.insert(
            \"precondition_line17\".into(),
            json!(paper(\"1040.line17\").unwrap_or(0)),

=== render.rs comparison/arith on printed fields ===
crates/btctax-cli/src/render.rs:1500:            fmt_money(printed.sch_3.map_or(Usd::ZERO, |s3| s3.line1))
crates/btctax-cli/src/render.rs:1507:            fmt_money(printed.sch_2.map_or(Usd::ZERO, |s2| s2.line4))
crates/btctax-cli/src/render.rs:1514:            fmt_money(printed.f8959.line18)
crates/btctax-cli/src/render.rs:1521:            fmt_money(printed.f8960.map_or(Usd::ZERO, |f| f.line17))
crates/btctax-cli/src/render.rs:1541:    if ar.amt.line6 > Usd::ZERO {
crates/btctax-cli/src/render.rs:1547:            fmt_money(ar.amt.line11),
crates/btctax-cli/src/render.rs:1548:            fmt_money(ar.amt.line4),
crates/btctax-cli/src/render.rs:1549:            fmt_money(ar.amt.line5),
crates/btctax-cli/src/render.rs:1550:            fmt_money(ar.amt.line9),
crates/btctax-cli/src/render.rs:1551:            fmt_money(ar.amt.line10),","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"750b07a9-3d55-4ff9-952e-96f0f92aceda","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"0a294027-384d-4cf3-9807-2d74535b1b3e","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRyQq8DJzmxemnMvLzx","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"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"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2732,"cache_read_input_tokens":86568,"cache_creation":{"ephemeral_5m_input_tokens":2732,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbRyNQHEvC5DCJUZVN2t","attributionAgent":"general-purpose","type":"assistant","uuid":"003a06bb-ef56-4ea3-b18f-43299886e586","timestamp":"2026-08-01T04:54:32.878Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"003a06bb-ef56-4ea3-b18f-43299886e586","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbRyQq8DJzmxemnMvLzx","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01GPVrzTFm4ZQ5KzJXjty4Vr","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax
echo \"=== render.rs 1600-1660 (ScheduleDTotals render) ===\"; sed -n '1600,1660p' crates/btctax-cli/src/render.rs
echo; echo \"=== admin.rs 345-375 ===\"; sed -n '345,375p' crates/btctax-cli/src/cmd/admin.rs"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2732,"cache_read_input_tokens":86568,"output_tokens":1676,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":2732},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":1676,"cache_read_input_tokens":86568,"cache_creation_input_tokens":2732,"cache_creation":{"ephemeral_5m_input_tokens":2732,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbRyNQHEvC5DCJUZVN2t","attributionAgent":"general-purpose","type":"assistant","uuid":"3afdadd0-512e-422c-b94d-dc4d92e01d34","timestamp":"2026-08-01T04:54:34.119Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"3afdadd0-512e-422c-b94d-dc4d92e01d34","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01GPVrzTFm4ZQ5KzJXjty4Vr","type":"tool_result","content":"=== render.rs 1600-1660 (ScheduleDTotals render) ===
/// P2-B Task 3: render the RAW pre-netting Schedule D part totals (Part I ST, Part II LT) for
/// `year`, mirroring `render_tax_outcome`. These are the Form 8949/Schedule D part totals BEFORE
/// §1222/§1211/§1212 netting + carryforward — that netting is applied in the tax computation
/// (`report --tax-year`), and the netted figures are shown by `render_tax_outcome` above.
///
/// When `outcome` is `Computed`, the standard netting note is shown. When `outcome` is
/// `NotComputable`, a caveat is printed instead: the raw totals are valid disposal sums but are
/// informational — no netting or carryforward is applied because the tax is not computable.
/// The raw totals are ALWAYS shown (never suppressed); only the trailing note differs.
pub fn render_schedule_d(
    year: i32,
    totals: &ScheduleDTotals,
    outcome: &btctax_core::TaxOutcome,
) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        \"Schedule D (raw pre-netting part totals) — tax year {year}\"
    );
    let _ = writeln!(
        s,
        \"  Part I  (short-term): proceeds {}   cost basis {}   gain {}\",
        fmt_money(totals.st.proceeds),
        fmt_money(totals.st.cost_basis),
        fmt_money(totals.st.gain)
    );
    let _ = writeln!(
        s,
        \"  Part II (long-term):  proceeds {}   cost basis {}   gain {}\",
        fmt_money(totals.lt.proceeds),
        fmt_money(totals.lt.cost_basis),
        fmt_money(totals.lt.gain)
    );
    match outcome {
        btctax_core::TaxOutcome::NotComputable(_) => {
            let _ = writeln!(
                s,
                \"  (raw disposition totals shown above; the year's tax is NOT COMPUTABLE until \\
                 the blocker is resolved — these Form 8949/Schedule D part totals are \\
                 informational and are not netted/carried until the tax computes).\"
            );
        }
        btctax_core::TaxOutcome::Computed(_) => {
            let _ = writeln!(
                s,
                \"  Note: §1222/§1211/§1212 netting + carryforward are applied in the tax \\
                 computation (report --tax-year); these are the raw pre-netting Form \\
                 8949/Schedule D part totals.\"
            );
        }
    }
    s
}

/// P2-D Task 2 / Chunk B (Schedule SE): render the standalone §1401 SE-tax block for `year` as an
/// informational block that does NOT feed engine B (`TaxResult::total_federal_tax_attributable` is
/// UNCHANGED by SE tax).
///
/// Three-way `None` split [R0-I1] (no silent drop — mirrors P2-C's m6):
/// - `gross_se == 0` → `None` (no business SE income → no Schedule SE section at all).
/// - `gross_se > 0 && !table_present` → a \"SS wage base unavailable for {year}\" note (business SE

=== admin.rs 345-375 ===

/// Whether a form is included: the packet is every applicable form unless `--forms` opts in to a subset.
fn wants(selected: &[FormArg], f: FormArg) -> bool {
    selected.is_empty() || selected.contains(&f)
}

/// A Schedule D part is \"active\" (worth reporting) iff it has any proceeds/cost/gain.
fn sd_part_active(p: &ScheduleDPart) -> bool {
    !p.proceeds.is_zero() || !p.cost_basis.is_zero() || !p.gain.is_zero()
}

/// `export-irs-pdf`: fill the OFFICIAL IRS PDFs for `tax_year` and write them (owner-only) to
/// `out_dir`. THIN OPENER (★ arch-C-1, Defensive Filing Wizard Task 3): opens its OWN `Session` and
/// projects ONCE, then delegates everything else to [`export_irs_pdf_from_session`] — the `&Session`
/// inner a future TUI (which already holds the vault's `VaultLock`) can call directly, without a SECOND
/// `Session::open` (which would deadlock the editor, `session.rs:662`).
pub fn export_irs_pdf(
    vault_path: &Path,
    pp: &Passphrase,
    out_dir: &Path,
    tax_year: i32,
    forms: &[FormArg],
    attest: Option<&str>,
) -> Result<IrsPdfReport, CliError> {
    let session = Session::open(vault_path, pp)?;
    let (events, state, _cfg) = session.load_events_and_project()?;
    export_irs_pdf_from_session(&session, &state, &events, out_dir, tax_year, forms, attest)
}

/// The `&Session` inner of `export_irs_pdf` (★ arch-C-1): fill the OFFICIAL IRS PDFs for `tax_year` over
/// an ALREADY-OPEN `session` + an ALREADY-PROJECTED `state`/`events` — no `Session::open`, no re-project.","is_error":false}]},"uuid":"8f1e16e3-9350-4946-987e-d445ce95cd9c","timestamp":"2026-08-01T04:54:34.216Z","toolUseResult":{"stdout":"=== render.rs 1600-1660 (ScheduleDTotals render) ===
/// P2-B Task 3: render the RAW pre-netting Schedule D part totals (Part I ST, Part II LT) for
/// `year`, mirroring `render_tax_outcome`. These are the Form 8949/Schedule D part totals BEFORE
/// §1222/§1211/§1212 netting + carryforward — that netting is applied in the tax computation
/// (`report --tax-year`), and the netted figures are shown by `render_tax_outcome` above.
///
/// When `outcome` is `Computed`, the standard netting note is shown. When `outcome` is
/// `NotComputable`, a caveat is printed instead: the raw totals are valid disposal sums but are
/// informational — no netting or carryforward is applied because the tax is not computable.
/// The raw totals are ALWAYS shown (never suppressed); only the trailing note differs.
pub fn render_schedule_d(
    year: i32,
    totals: &ScheduleDTotals,
    outcome: &btctax_core::TaxOutcome,
) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        \"Schedule D (raw pre-netting part totals) — tax year {year}\"
    );
    let _ = writeln!(
        s,
        \"  Part I  (short-term): proceeds {}   cost basis {}   gain {}\",
        fmt_money(totals.st.proceeds),
        fmt_money(totals.st.cost_basis),
        fmt_money(totals.st.gain)
    );
    let _ = writeln!(
        s,
        \"  Part II (long-term):  proceeds {}   cost basis {}   gain {}\",
        fmt_money(totals.lt.proceeds),
        fmt_money(totals.lt.cost_basis),
        fmt_money(totals.lt.gain)
    );
    match outcome {
        btctax_core::TaxOutcome::NotComputable(_) => {
            let _ = writeln!(
                s,
                \"  (raw disposition totals shown above; the year's tax is NOT COMPUTABLE until \\
                 the blocker is resolved — these Form 8949/Schedule D part totals are \\
                 informational and are not netted/carried until the tax computes).\"
            );
        }
        btctax_core::TaxOutcome::Computed(_) => {
            let _ = writeln!(
                s,
                \"  Note: §1222/§1211/§1212 netting + carryforward are applied in the tax \\
                 computation (report --tax-year); these are the raw pre-netting Form \\
                 8949/Schedule D part totals.\"
            );
        }
    }
    s
}

/// P2-D Task 2 / Chunk B (Schedule SE): render the standalone §1401 SE-tax block for `year` as an
/// informational block that does NOT feed engine B (`TaxResult::total_federal_tax_attributable` is
/// UNCHANGED by SE tax).
///
/// Three-way `None` split [R0-I1] (no silent drop — mirrors P2-C's m6):
/// - `gross_se == 0` → `None` (no business SE income → no Schedule SE section at all).
/// - `gross_se > 0 && !table_present` → a \"SS wage base unavailable for {year}\" note (business SE

=== admin.rs 345-375 ===

/// Whether a form is included: the packet is every applicable form unless `--forms` opts in to a subset.
fn wants(selected: &[FormArg], f: FormArg) -> bool {
    selected.is_empty() || selected.contains(&f)
}

/// A Schedule D part is \"active\" (worth reporting) iff it has any proceeds/cost/gain.
fn sd_part_active(p: &ScheduleDPart) -> bool {
    !p.proceeds.is_zero() || !p.cost_basis.is_zero() || !p.gain.is_zero()
}

/// `export-irs-pdf`: fill the OFFICIAL IRS PDFs for `tax_year` and write them (owner-only) to
/// `out_dir`. THIN OPENER (★ arch-C-1, Defensive Filing Wizard Task 3): opens its OWN `Session` and
/// projects ONCE, then delegates everything else to [`export_irs_pdf_from_session`] — the `&Session`
/// inner a future TUI (which already holds the vault's `VaultLock`) can call directly, without a SECOND
/// `Session::open` (which would deadlock the editor, `session.rs:662`).
pub fn export_irs_pdf(
    vault_path: &Path,
    pp: &Passphrase,
    out_dir: &Path,
    tax_year: i32,
    forms: &[FormArg],
    attest: Option<&str>,
) -> Result<IrsPdfReport, CliError> {
    let session = Session::open(vault_path, pp)?;
    let (events, state, _cfg) = session.load_events_and_project()?;
    export_irs_pdf_from_session(&session, &state, &events, out_dir, tax_year, forms, attest)
}

/// The `&Session` inner of `export_irs_pdf` (★ arch-C-1): fill the OFFICIAL IRS PDFs for `tax_year` over
/// an ALREADY-OPEN `session` + an ALREADY-PROJECTED `state`/`events` — no `Session::open`, no re-project.","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"3afdadd0-512e-422c-b94d-dc4d92e01d34","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"8f1e16e3-9350-4946-987e-d445ce95cd9c","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbS198cmY4WKpzhkmUYu","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"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"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":3525,"cache_read_input_tokens":89300,"cache_creation":{"ephemeral_5m_input_tokens":3525,"ephemeral_1h_input_tokens":0},"output_tokens":4,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbS17zexihjv2aiFQb2q","attributionAgent":"general-purpose","type":"assistant","uuid":"2323e2ca-24a2-4675-966d-04131f6ca045","timestamp":"2026-08-01T04:55:06.168Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"2323e2ca-24a2-4675-966d-04131f6ca045","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbS198cmY4WKpzhkmUYu","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_0173C9SYDVo22c69T6e3N8t2","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax
echo \"=== printed-struct field reads doing ARITHMETIC or COMPARISON, production src only ===\"
grep -rnE \"(f1040|f8959|f8960|f8995|amt|lines|totals|f\\.|s2\\.|s3\\.|pr\\.forms\\.[a-z0-9_]+)\\.line[0-9a-z_]+ *([<>+*/-]|==|!=)|\\.line[0-9a-z_]+\\.is_zero\\(\\)\" --include=\"*.rs\" crates/*/src/ | grep -v \"^crates/btctax-core/src/tax/form6251.rs\\|^crates/btctax-core/src/tax/other_taxes.rs\\|^crates/btctax-core/src/tax/qbi.rs\\|^crates/btctax-core/src/tax/printed.rs\" | head -40"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":3525,"cache_read_input_tokens":89300,"output_tokens":2552,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":3525},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":2552,"cache_read_input_tokens":89300,"cache_creation_input_tokens":3525,"cache_creation":{"ephemeral_5m_input_tokens":3525,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbS17zexihjv2aiFQb2q","attributionAgent":"general-purpose","type":"assistant","uuid":"723da398-36db-4c8c-8879-3a44f952730b","timestamp":"2026-08-01T04:55:09.826Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"723da398-36db-4c8c-8879-3a44f952730b","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_0173C9SYDVo22c69T6e3N8t2","type":"tool_result","content":"=== printed-struct field reads doing ARITHMETIC or COMPARISON, production src only ===
crates/btctax-forms/src/schedule_se_full.rs:78:    let skip_8d_to_10 = lines.line9 == Usd::ZERO;
crates/btctax-cli/src/render.rs:1541:    if ar.amt.line6 > Usd::ZERO {
crates/btctax-core/src/tax/packet.rs:1224:            pr.forms.sch_d.line10_d - pr.forms.sch_d.line10_e,
crates/btctax-forms/src/form8995.rs:166:    if lines.line2 > Usd::ZERO {
crates/btctax-oracle-harness/src/main.rs:324:    let l24_target = pr.forms.f1040.line16 + se_l12_ots + f8959_l18_ots + round_leaf(e.niit);
crates/btctax-core/src/tax/return_1040.rs:3883:            ar_high.amt.line7 <= ar_high.amt.line10,
crates/btctax-core/src/tax/return_1040.rs:3945:            ar.amt.line12 > Usd::ZERO,
crates/btctax-core/src/tax/return_1040.rs:3989:        assert!(ar.amt.line6 > Usd::ZERO, \"AMTI exceeds the exemption\");
crates/btctax-core/src/tax/return_1040.rs:3991:            ar.amt.line7 <= ar.amt.line10,
crates/btctax-core/src/tax/return_1040.rs:4082:            ar.amt.line7 <= ar.amt.line10,","is_error":false}]},"uuid":"b9874df9-34ad-47b5-809e-561be043a99f","timestamp":"2026-08-01T04:55:09.924Z","toolUseResult":{"stdout":"=== printed-struct field reads doing ARITHMETIC or COMPARISON, production src only ===
crates/btctax-forms/src/schedule_se_full.rs:78:    let skip_8d_to_10 = lines.line9 == Usd::ZERO;
crates/btctax-cli/src/render.rs:1541:    if ar.amt.line6 > Usd::ZERO {
crates/btctax-core/src/tax/packet.rs:1224:            pr.forms.sch_d.line10_d - pr.forms.sch_d.line10_e,
crates/btctax-forms/src/form8995.rs:166:    if lines.line2 > Usd::ZERO {
crates/btctax-oracle-harness/src/main.rs:324:    let l24_target = pr.forms.f1040.line16 + se_l12_ots + f8959_l18_ots + round_leaf(e.niit);
crates/btctax-core/src/tax/return_1040.rs:3883:            ar_high.amt.line7 <= ar_high.amt.line10,
crates/btctax-core/src/tax/return_1040.rs:3945:            ar.amt.line12 > Usd::ZERO,
crates/btctax-core/src/tax/return_1040.rs:3989:        assert!(ar.amt.line6 > Usd::ZERO, \"AMTI exceeds the exemption\");
crates/btctax-core/src/tax/return_1040.rs:3991:            ar.amt.line7 <= ar.amt.line10,
crates/btctax-core/src/tax/return_1040.rs:4082:            ar.amt.line7 <= ar.amt.line10,","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"723da398-36db-4c8c-8879-3a44f952730b","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"b9874df9-34ad-47b5-809e-561be043a99f","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbS3oeBi2ftHMGnYwABm","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISjwgKhwEIEBgCKkBd2ITAFDG/WNUMfZzxyVYy43OkOvMR0YYt0v5BtuE1j8KfjX3cvuZRBiUCyPmBNYtFm/iDVyoFrTat0sxEq+gdMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDIMx6+RESYSrwg5ufRoMrdi0PgmzNn6SiqDKIjDQ0tffSYar/+orQ9u49OlM3hJr65gfVkAzATifRqsp7oCd8qDKxxiVcFniXPyUYd4qtAbR8u0tIxG7Eq0IuLbRgpw9sRkRJQYSZ/jHpKYQWtj1Mew8egaVY7W04Qx2OWvTejJbtvRnjH2DmIWiOYC3uGf45Jv9e6bTaga/mab+GPfugFU/Lxc3lFyIuEMlrmafWUPqEw99Acez2mwpNnz+2SH2BpXpgaqvb1Ld4w3d/4a5G0QzYXaVus66XRj0QHTCrmWJRB/j0451mYp+0EdHe9kv313sqv/MPPpTQNBj2splGKeO8WRJD/TaNa4180EaAX2F+c/ZTn3p1twON64tjZmS+aKW4Oz1qMMuEOhpZ+l4kH/zL+1NSAa1vbKTicQ0/shvAMUvVuxP3agCLAZlXvwZaKTXRvs6WLLvn3oah1B8DbXgc6tT6aGwcqEsCixhZ89fsr/HTfUa/PWYlrdB7vTLhIbwozZsQazYCdgFGwu0lJESq8cqvrjPZFtK4v1yaCwko6x6C14S9z9ZFtPTn7cQrcE2lRrhHgaMVnBy0VyY8aqMhA9AzsOkYgKl/a15PHtoF0P7cZoqbvhbZqyrwhZ4POGfZepmXwOTF375ZlWFpBaBuD6iK6oy5vUmV+TqR9WzckP6i77Ipvy2Tz3aCA6GW5lQupYvXIydtLfTr1A7SJoUle3pjxXODbJ5s7g1fpmZBaxidREeQ92FPfK5N/2UQ5r5bVNuYomxWUpKWZo96QxdR+ApFeReyoZznLs1Fm+kx+AZ4lZHyPuL1TBGMRp9n2eCGrdYLL3r0Lov5kTdr9z3UmlHI3zxnnl6LVUhrYToZhcgdsMaVjrwmkhvX8sOHeqkitNZYH7Kdq+TQwQNlsaJPHwvMwUvhAW2yrDolsf7TJn5Ja4SMoskl01uIySNs4NlYfJkHaLBXNaeAs+geoTZ5o9tpMIDX0RGL3lgSoC3SgB5cT9P5nIDXHsbCuhjj1wIUHSRY+T1BUsnw3mQN1vRNr0qY22HDWUJODJbbDDQJi+fjzucFgb6FKBDhw4XNzacDNF1WQDquYYwhrxSDTz4Hp5tvhEoTrWpWwEM673Oz6Qb2+iqQvl970d28Z6tzjfq/5WRDp4jNHe0CzVUQfeQeXiZtNx1X8kfu4cJz9Z/Btf+GAE="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":3113,"cache_read_input_tokens":92825,"cache_creation":{"ephemeral_5m_input_tokens":3113,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbS3kbdCex78Aco8HrR5","attributionAgent":"general-purpose","type":"assistant","uuid":"c87657ff-c36e-4a30-ac80-3fd51aed7748","timestamp":"2026-08-01T04:55:18.356Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"c87657ff-c36e-4a30-ac80-3fd51aed7748","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbS3oeBi2ftHMGnYwABm","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_013GvXFDbQMpCH1EdEsZ4h4x","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax
echo \"=== packet.rs 1200-1240 ===\"; sed -n '1200,1240p' crates/btctax-core/src/tax/packet.rs
echo; echo \"=== render.rs 1565-1575 ===\"; sed -n '1565,1575p' crates/btctax-cli/src/render.rs"},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":3113,"cache_read_input_tokens":92825,"cache_creation":{"ephemeral_5m_input_tokens":3113,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbS3kbdCex78Aco8HrR5","attributionAgent":"general-purpose","type":"assistant","uuid":"29915da1-b7f9-45c8-b653-d8788dcec047","timestamp":"2026-08-01T04:55:18.440Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"29915da1-b7f9-45c8-b653-d8788dcec047","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_013GvXFDbQMpCH1EdEsZ4h4x","type":"tool_result","content":"=== packet.rs 1200-1240 ===
            .as_ref()
            .expect(\"…and a disposal ⇒ Form 8949\");
        assert_eq!(
            sch_2.line4, sch_se.line12,
            \"Sch 2 L4 ← Schedule SE's printed L12\"
        );
        assert_eq!(
            sch_1.line15, sch_se.line13,
            \"Sch 1 L15 ← Schedule SE's printed L13\"
        );
        assert_eq!(
            pr.forms.f8959.line8, sch_se.line6,
            \"8959 L8 ← Schedule SE Part I L6\"
        );
        assert_eq!(
            sch_se.line2, sch_c.line31,
            \"SE L2 ← Schedule C's printed L31\"
        );
        assert_eq!(
            pr.forms.sch_d.line10_d, f8949.lt_totals.proceeds_d,
            \"Sch D L10(d) ← the 8949's printed long-term column total\"
        );
        assert_eq!(
            pr.forms.sch_d.line10_h,
            pr.forms.sch_d.line10_d - pr.forms.sch_d.line10_e,
            \"…and Schedule D Part II cross-foots on its own printed cells\"
        );

        // ★ The extension payment reaches the filed page (ARCH-P6.3a D1). Without Schedule 3 line 10 the
        // return would demand a payment the filer had ALREADY made: L31 falls ⇒ L37 \"amount you owe\"
        // rises by exactly that amount.
        let sch_3 = pr
            .forms
            .sch_3
            .expect(\"…and an extension payment + FTC ⇒ Schedule 3\");
        assert_eq!(
            pr.forms.f1040.line31, sch_3.line15,
            \"1040 31 ← Schedule 3 line 15\"
        );
        assert_eq!(
            sch_3.line10,

=== render.rs 1565-1575 ===
    );
    let _ = writeln!(s, \"  Total payments (L33):     {}\", fmt_money(f.line33));
    if f.line34 > Usd::ZERO {
        let _ = writeln!(s, \"  → REFUND (L35a):          {}\", fmt_money(f.line34));
    } else {
        let _ = writeln!(s, \"  → AMOUNT OWED (L37):      {}\", fmt_money(f.line37));
    }
    // §6: the two figures answer different questions and are NEVER reconciled.
    let delta_str = match delta {
        btctax_core::TaxOutcome::Computed(r) => fmt_money(r.total_federal_tax_attributable),
        btctax_core::TaxOutcome::NotComputable(_) => \"not computable\".to_string(),","is_error":false}]},"uuid":"03995819-bdc9-4cad-be8a-46145f7c4e40","timestamp":"2026-08-01T04:55:18.530Z","toolUseResult":{"stdout":"=== packet.rs 1200-1240 ===
            .as_ref()
            .expect(\"…and a disposal ⇒ Form 8949\");
        assert_eq!(
            sch_2.line4, sch_se.line12,
            \"Sch 2 L4 ← Schedule SE's printed L12\"
        );
        assert_eq!(
            sch_1.line15, sch_se.line13,
            \"Sch 1 L15 ← Schedule SE's printed L13\"
        );
        assert_eq!(
            pr.forms.f8959.line8, sch_se.line6,
            \"8959 L8 ← Schedule SE Part I L6\"
        );
        assert_eq!(
            sch_se.line2, sch_c.line31,
            \"SE L2 ← Schedule C's printed L31\"
        );
        assert_eq!(
            pr.forms.sch_d.line10_d, f8949.lt_totals.proceeds_d,
            \"Sch D L10(d) ← the 8949's printed long-term column total\"
        );
        assert_eq!(
            pr.forms.sch_d.line10_h,
            pr.forms.sch_d.line10_d - pr.forms.sch_d.line10_e,
            \"…and Schedule D Part II cross-foots on its own printed cells\"
        );

        // ★ The extension payment reaches the filed page (ARCH-P6.3a D1). Without Schedule 3 line 10 the
        // return would demand a payment the filer had ALREADY made: L31 falls ⇒ L37 \"amount you owe\"
        // rises by exactly that amount.
        let sch_3 = pr
            .forms
            .sch_3
            .expect(\"…and an extension payment + FTC ⇒ Schedule 3\");
        assert_eq!(
            pr.forms.f1040.line31, sch_3.line15,
            \"1040 31 ← Schedule 3 line 15\"
        );
        assert_eq!(
            sch_3.line10,

=== render.rs 1565-1575 ===
    );
    let _ = writeln!(s, \"  Total payments (L33):     {}\", fmt_money(f.line33));
    if f.line34 > Usd::ZERO {
        let _ = writeln!(s, \"  → REFUND (L35a):          {}\", fmt_money(f.line34));
    } else {
        let _ = writeln!(s, \"  → AMOUNT OWED (L37):      {}\", fmt_money(f.line37));
    }
    // §6: the two figures answer different questions and are NEVER reconciled.
    let delta_str = match delta {
        btctax_core::TaxOutcome::Computed(r) => fmt_money(r.total_federal_tax_attributable),
        btctax_core::TaxOutcome::NotComputable(_) => \"not computable\".to_string(),","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"29915da1-b7f9-45c8-b653-d8788dcec047","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"03995819-bdc9-4cad-be8a-46145f7c4e40","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbS4QQDd2Z7e8KTxG3pm","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"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"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1493,"cache_read_input_tokens":95938,"cache_creation":{"ephemeral_5m_input_tokens":1493,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbS4PZ7zMGXXx3hsN3xU","attributionAgent":"general-purpose","type":"assistant","uuid":"3688b676-0dcc-46ca-b129-eaaf9aa2f9c1","timestamp":"2026-08-01T04:55:58.632Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"3688b676-0dcc-46ca-b129-eaaf9aa2f9c1","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbS4QQDd2Z7e8KTxG3pm","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01DwnUwRcBEaLb18EKRRUjPs","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax
echo \"=== is tax::printed publicly reachable (clippy path resolution)? ===\"
grep -rn \"pub mod printed\\|mod printed\" crates/btctax-core/src/tax/mod.rs crates/btctax-core/src/tax.rs 2>/dev/null | head
grep -rn \"pub mod tax\\|mod tax;\" crates/btctax-core/src/lib.rs | head
echo; echo \"=== MoneyCell::Pair live? ===\"
grep -rn \"MoneyCell::Pair\\|Pair(\" crates/btctax-forms/src/cells.rs | head
grep -rln \"dollars_field\" forms/ 2>/dev/null | head
echo; echo \"=== packet.rs:1224 in a test module? ===\"
awk 'NR>=1100 && NR<=1224 && /#\\[cfg\\(test\\)\\]|mod tests|fn [a-z_]+\\(\\)/' crates/btctax-core/src/tax/packet.rs | tail -3
grep -n \"#\\[cfg(test)\\]\" crates/btctax-core/src/tax/packet.rs | head"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1493,"cache_read_input_tokens":95938,"output_tokens":3261,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":1493},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":3261,"cache_read_input_tokens":95938,"cache_creation_input_tokens":1493,"cache_creation":{"ephemeral_5m_input_tokens":1493,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbS4PZ7zMGXXx3hsN3xU","attributionAgent":"general-purpose","type":"assistant","uuid":"27d84c69-2866-483f-9c13-dc5342a0380e","timestamp":"2026-08-01T04:56:02.326Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"27d84c69-2866-483f-9c13-dc5342a0380e","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01DwnUwRcBEaLb18EKRRUjPs","type":"tool_result","content":"=== is tax::printed publicly reachable (clippy path resolution)? ===
crates/btctax-core/src/tax/mod.rs:22:pub mod printed;
crates/btctax-core/src/lib.rs:17:pub mod tax;

=== MoneyCell::Pair live? ===
crates/btctax-forms/src/cells.rs:57:        MoneyCell::Pair(mp) => {
crates/btctax-forms/src/cells.rs:82:        MoneyCell::Pair(mp) => &mp.dollars_field,

=== packet.rs:1224 in a test module? ===
663:#[cfg(test)]","is_error":false}]},"uuid":"19b1b2cc-9dc4-4762-9355-c3ec684b9c65","timestamp":"2026-08-01T04:56:02.477Z","toolUseResult":{"stdout":"=== is tax::printed publicly reachable (clippy path resolution)? ===
crates/btctax-core/src/tax/mod.rs:22:pub mod printed;
crates/btctax-core/src/lib.rs:17:pub mod tax;

=== MoneyCell::Pair live? ===
crates/btctax-forms/src/cells.rs:57:        MoneyCell::Pair(mp) => {
crates/btctax-forms/src/cells.rs:82:        MoneyCell::Pair(mp) => &mp.dollars_field,

=== packet.rs:1224 in a test module? ===
663:#[cfg(test)]","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"27d84c69-2866-483f-9c13-dc5342a0380e","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"19b1b2cc-9dc4-4762-9355-c3ec684b9c65","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbS7eAnNMob72E77ySeC","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAIS2QQKhwEIEBgCKkChKKGbnzcUW8fnu/xwaZCmRm2qtfEw64uZfdZJb2wn/ij3CE0PByERoem+dDeT3wzG3kPtVJOf6tnUdroVYKa8Mg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDCi2jYF18DSNjhnm0BoMQhtpyyLDddt/bLbZIjAZBlz8C/D56JgkIuoE8gIonF1lqulh/j84hX3Y0Ruu66tkRD5MnJBZKgeE7K2qXzoq/gJfKhSXYVlVvv5zW0dzjSN9WfwQYiAUGNv8/5tnrs36jk4lZ3jrPoPus0iTCQDI6wL8pC6zPyJ7m4hEpQNdgRngL9C0aNmOpYcW9zc2fV3Nc8lPZ73iyv5SnsOvwoakr0CpaCmq5ZaxU0X37jmTmAw7mO8TAQID3kxGx9T2IlDf2Tb3TbfWOp1A5Whu++vvGF76nxJIWKAteBVgqaX/6VqXDxvhNb4/w49BA+apot9asSFDXroWfNWJ4DktrnuMvWD2f+l1Qmj3syaYfY00+4/KRlM5/FLe/sn5Hf0e9c1k09SrcQnDWl6EyCKr/F/yaSw7KNNGIdc4T49O5Y2pMdqILovf7s7ezBgNl4SZ6ctcMS/HiTpvsq90iFEyfwpuvs6hEeRdLaSVbR6Xi26SGDYtwYYllogdR+7pE6Xm0upiSL8c8sF6HXNM87Dlm/HKfOa7dfqN0NoMHUppA3F8+Z7FsVY9KvGTR4FriCLdo6kK5vElyDaUfulMxwRjXgsvGAE="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":3482,"cache_read_input_tokens":97431,"cache_creation":{"ephemeral_5m_input_tokens":3482,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbS7dEj4Kpr2fNm8LJ67","attributionAgent":"general-purpose","type":"assistant","uuid":"b3fb30fb-8589-4c5d-a6c1-28802c327ba4","timestamp":"2026-08-01T04:56:05.727Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"b3fb30fb-8589-4c5d-a6c1-28802c327ba4","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbS7eAnNMob72E77ySeC","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01Bnj1HgCfhTocVdRDzuM9EQ","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax; ls forms/ 2>/dev/null; grep -rl \"dollars_field\\|dollars =\" forms/ 2>/dev/null | head; echo \"---\"; grep -rn \"dollars\" forms/2017/*.toml 2>/dev/null | head -5; echo \"--- MoneyCell enum ---\"; sed -n '14,34p' crates/btctax-forms/src/cells.rs"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":3482,"cache_read_input_tokens":97431,"output_tokens":354,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":3482},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":354,"cache_read_input_tokens":97431,"cache_creation_input_tokens":3482,"cache_creation":{"ephemeral_5m_input_tokens":3482,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbS7dEj4Kpr2fNm8LJ67","attributionAgent":"general-purpose","type":"assistant","uuid":"4f6635eb-54de-4c9c-827c-f921d12e645d","timestamp":"2026-08-01T04:56:07.242Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"4f6635eb-54de-4c9c-827c-f921d12e645d","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01Bnj1HgCfhTocVdRDzuM9EQ","type":"tool_result","content":"---
(eval):1: no matches found: forms/2017/*.toml
--- MoneyCell enum ---
use crate::pdf::{Field, FieldValue};
use crate::verify::FlatPlacement;
use btctax_core::tax::packet::Ssn;
use btctax_core::Usd;
use rust_decimal_macros::dec;

/// 0-based page index a field lives on (page 2 = the 2017 §B long Schedule SE / the Rev. 12-2014
/// Section B tables; page 1 otherwise). Derived from the fully-qualified name, exactly as the
/// verifier's own `page_of` — so a placement's page always agrees with its widget.
pub fn page_of(fqn: &str) -> usize {
    if fqn.contains(\"Page2\") {
        1
    } else {
        0
    }
}

/// Split a money amount into (whole-dollars, 2-digit-zero-padded-cents) — the REAL formatter the
/// dollars+cents pairs need (`fmt_money` is the raw `Decimal` Display, which pads no cents). The value
/// is rounded to cents first, so `100 → (\"100\",\"00\")`, `45500.5 → (\"45500\",\"50\")`,
/// `100.05 → (\"100\",\"05\")`.","is_error":false}]},"uuid":"122ad94a-9ca7-4ea4-9bec-451ee219a749","timestamp":"2026-08-01T04:56:07.341Z","toolUseResult":{"stdout":"---
(eval):1: no matches found: forms/2017/*.toml
--- MoneyCell enum ---
use crate::pdf::{Field, FieldValue};
use crate::verify::FlatPlacement;
use btctax_core::tax::packet::Ssn;
use btctax_core::Usd;
use rust_decimal_macros::dec;

/// 0-based page index a field lives on (page 2 = the 2017 §B long Schedule SE / the Rev. 12-2014
/// Section B tables; page 1 otherwise). Derived from the fully-qualified name, exactly as the
/// verifier's own `page_of` — so a placement's page always agrees with its widget.
pub fn page_of(fqn: &str) -> usize {
    if fqn.contains(\"Page2\") {
        1
    } else {
        0
    }
}

/// Split a money amount into (whole-dollars, 2-digit-zero-padded-cents) — the REAL formatter the
/// dollars+cents pairs need (`fmt_money` is the raw `Decimal` Display, which pads no cents). The value
/// is rounded to cents first, so `100 → (\"100\",\"00\")`, `45500.5 → (\"45500\",\"50\")`,
/// `100.05 → (\"100\",\"05\")`.","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"4f6635eb-54de-4c9c-827c-f921d12e645d","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"122ad94a-9ca7-4ea4-9bec-451ee219a749","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbS81XfLYM9tfmusD5np","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01PjNtvSwsTR43nkE8Qk5a6D","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax; find . -name \"*.map.toml\" -not -path \"./target*\" | head -20; echo \"---\"; grep -rln \"dollars_field\" --include=\"*.toml\" . 2>/dev/null | grep -v target | head"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":781,"cache_read_input_tokens":100913,"output_tokens":133,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":781},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":133,"cache_read_input_tokens":100913,"cache_creation_input_tokens":781,"cache_creation":{"ephemeral_5m_input_tokens":781,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbS7z8KyWhMschzyNKfJ","attributionAgent":"general-purpose","type":"assistant","uuid":"26e19e4c-ebef-4c53-9826-05cd9ee71db6","timestamp":"2026-08-01T04:56:09.571Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"26e19e4c-ebef-4c53-9826-05cd9ee71db6","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01PjNtvSwsTR43nkE8Qk5a6D","type":"tool_result","content":"./crates/btctax-forms/forms/2025/f1040.map.toml
./crates/btctax-forms/forms/2025/f8283.map.toml
./crates/btctax-forms/forms/2025/schedule_se.map.toml
./crates/btctax-forms/forms/2025/f8949.map.toml
./crates/btctax-forms/forms/2025/schedule_d.map.toml
./crates/btctax-forms/forms/2024/f1040s1.map.toml
./crates/btctax-forms/forms/2024/f1040s3.map.toml
./crates/btctax-forms/forms/2024/f8949.map.toml
./crates/btctax-forms/forms/2024/f8960.map.toml
./crates/btctax-forms/forms/2024/schedule_d.map.toml
./crates/btctax-forms/forms/2024/f1040sa.map.toml
./crates/btctax-forms/forms/2024/f8275.map.toml
./crates/btctax-forms/forms/2024/f1040s2.map.toml
./crates/btctax-forms/forms/2024/f8959.map.toml
./crates/btctax-forms/forms/2024/f1040.map.toml
./crates/btctax-forms/forms/2024/f1040sb.map.toml
./crates/btctax-forms/forms/2024/f1040sc.map.toml
./crates/btctax-forms/forms/2024/f8283.map.toml
./crates/btctax-forms/forms/2024/f8995.map.toml
./crates/btctax-forms/forms/2024/schedule_se.map.toml
---
crates/btctax-forms/forms/2017/f1040.map.toml
crates/btctax-forms/forms/2017/schedule_se.map.toml
crates/btctax-forms/forms/2017/f8283.map.toml","is_error":false}]},"uuid":"854f9b47-8177-45a5-bf3a-de9019a91aa0","timestamp":"2026-08-01T04:56:09.858Z","toolUseResult":{"stdout":"./crates/btctax-forms/forms/2025/f1040.map.toml
./crates/btctax-forms/forms/2025/f8283.map.toml
./crates/btctax-forms/forms/2025/schedule_se.map.toml
./crates/btctax-forms/forms/2025/f8949.map.toml
./crates/btctax-forms/forms/2025/schedule_d.map.toml
./crates/btctax-forms/forms/2024/f1040s1.map.toml
./crates/btctax-forms/forms/2024/f1040s3.map.toml
./crates/btctax-forms/forms/2024/f8949.map.toml
./crates/btctax-forms/forms/2024/f8960.map.toml
./crates/btctax-forms/forms/2024/schedule_d.map.toml
./crates/btctax-forms/forms/2024/f1040sa.map.toml
./crates/btctax-forms/forms/2024/f8275.map.toml
./crates/btctax-forms/forms/2024/f1040s2.map.toml
./crates/btctax-forms/forms/2024/f8959.map.toml
./crates/btctax-forms/forms/2024/f1040.map.toml
./crates/btctax-forms/forms/2024/f1040sb.map.toml
./crates/btctax-forms/forms/2024/f1040sc.map.toml
./crates/btctax-forms/forms/2024/f8283.map.toml
./crates/btctax-forms/forms/2024/f8995.map.toml
./crates/btctax-forms/forms/2024/schedule_se.map.toml
---
crates/btctax-forms/forms/2017/f1040.map.toml
crates/btctax-forms/forms/2017/schedule_se.map.toml
crates/btctax-forms/forms/2017/f8283.map.toml","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"26e19e4c-ebef-4c53-9826-05cd9ee71db6","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"854f9b47-8177-45a5-bf3a-de9019a91aa0","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbS8BibCRX1x4nMN45dN","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"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"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":756,"cache_read_input_tokens":101694,"cache_creation":{"ephemeral_5m_input_tokens":756,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbS8Amncmwx7DMwpFkn9","attributionAgent":"general-purpose","type":"assistant","uuid":"8bd7cf07-7f36-4b84-9a85-f1fc4a263f01","timestamp":"2026-08-01T04:56:31.162Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"8bd7cf07-7f36-4b84-9a85-f1fc4a263f01","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbS8BibCRX1x4nMN45dN","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01MsvR1ubD5PSuY9ZiHD6uGD","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax
echo \"=== form8275 item.amount type ===\"; sed -n '196,232p' crates/btctax-core/src/tax/printed.rs
echo; echo \"=== form1040.rs:105-130 push_literal ===\"; sed -n '105,130p' crates/btctax-forms/src/form1040.rs
echo; echo \"=== Form6251 Default used in tests? ===\"; grep -rn \"Form6251\" --include=\"*.rs\" crates/ | grep -i \"default\" | head"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":756,"cache_read_input_tokens":101694,"output_tokens":1584,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":756},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":1584,"cache_read_input_tokens":101694,"cache_creation_input_tokens":756,"cache_creation":{"ephemeral_5m_input_tokens":756,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbS8Amncmwx7DMwpFkn9","attributionAgent":"general-purpose","type":"assistant","uuid":"7675ea9b-75c7-41d8-a60c-a8f7241b506f","timestamp":"2026-08-01T04:56:32.874Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"7675ea9b-75c7-41d8-a60c-a8f7241b506f","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01MsvR1ubD5PSuY9ZiHD6uGD","type":"tool_result","content":"=== form8275 item.amount type ===
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printed8275 {
    pub part_i: Vec<crate::tax::form8275::Part1Item>,
    pub part_ii: String,
}

/// Round a `Disclosure8275`'s Part I amounts for the filed packet. Part II is the filer's own
/// narrative text — never rounded, carried through verbatim.
pub fn printed_8275(d: &crate::tax::form8275::Disclosure8275) -> Printed8275 {
    Printed8275 {
        part_i: d
            .part_i
            .iter()
            .map(|i| crate::tax::form8275::Part1Item {
                amount: round_dollar(i.amount),
                ..i.clone()
            })
            .collect(),
        part_ii: d.part_ii.clone(),
    }
}

// ── Schedule SE — the printed chain (ARCH-P6.3a D5) ─────────────────────────────────────────────

/// The printable **Schedule SE** line chain.
///
/// ★ Schedule SE is a **filed form**, not a worksheet. The SE *engine* (`se.rs`) stays a FROZEN
/// exact-cents worksheet; this chain rounds its values AT THE LINE and derives the additions from the
/// printed operands, exactly like every other filed form under the §3.1 election. (SPEC §3.1 listed \"SE\"
/// among the unprinted worksheets — a contradiction with §7.1's fill set, amended alongside this work.)
///
/// The citations bind it to its neighbours, and the form states each one:
/// - **line 2** ← Schedule C's printed **line 31** (\"Net profit or (loss) from Schedule C, line 31\")
/// - **line 12** → Schedule 2 **line 4** (\"Enter here and on Schedule 2 (Form 1040), line 4\")
/// - **line 13** → Schedule 1 **line 15** (\"Enter here and on Schedule 1 (Form 1040), line 15\")
/// - **line 6** ← cited BY Form 8959 line 8 (\"from Schedule SE, Part I, line 6\")
#[derive(Debug, Clone, Copy, PartialEq, Eq)]

=== form1040.rs:105-130 push_literal ===

    // Capital-gain line (7a in 2025 / 7 in 2024 / **13 in 2017**) — only when Schedule D is ACTIVE and
    // line 16 ≥ 0. The cell is single (2024/2025) or a dollars+cents pair (2017), handled uniformly.
    let mut filled_7a = false;
    let mut active_zero = false;
    let mut loss = false;
    if inputs.schedule_d_active {
        if inputs.schedule_d_line16 < Usd::ZERO {
            loss = true; // net loss → line BLANK + notice (§1211 line-21 cap is the filer's).
        } else if inputs.schedule_d_line16.is_zero() {
            active_zero = true; // active-and-netted-to-zero → the \"-0-\" literal.
            push_literal(
                &mut writes,
                &mut placements,
                &map.line7a,
                \"-0-\",
                F1040_COL_AMOUNT,
            );
            filled_7a = true;
        } else {
            push_money(
                &mut writes,
                &mut placements,
                &map.line7a,
                inputs.schedule_d_line16,
                F1040_COL_AMOUNT,

=== Form6251 Default used in tests? ===
crates/btctax-core/src/tax/form6251.rs:63:impl Default for Form6251Line1 {
crates/btctax-core/src/tax/form6251.rs:64:    /// TY2024's shape, matching `Form6251::default()` and every committed fixture vector.","is_error":false}]},"uuid":"28b6676f-a42b-4f9e-9d94-c755ebb19c3d","timestamp":"2026-08-01T04:56:32.986Z","toolUseResult":{"stdout":"=== form8275 item.amount type ===
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printed8275 {
    pub part_i: Vec<crate::tax::form8275::Part1Item>,
    pub part_ii: String,
}

/// Round a `Disclosure8275`'s Part I amounts for the filed packet. Part II is the filer's own
/// narrative text — never rounded, carried through verbatim.
pub fn printed_8275(d: &crate::tax::form8275::Disclosure8275) -> Printed8275 {
    Printed8275 {
        part_i: d
            .part_i
            .iter()
            .map(|i| crate::tax::form8275::Part1Item {
                amount: round_dollar(i.amount),
                ..i.clone()
            })
            .collect(),
        part_ii: d.part_ii.clone(),
    }
}

// ── Schedule SE — the printed chain (ARCH-P6.3a D5) ─────────────────────────────────────────────

/// The printable **Schedule SE** line chain.
///
/// ★ Schedule SE is a **filed form**, not a worksheet. The SE *engine* (`se.rs`) stays a FROZEN
/// exact-cents worksheet; this chain rounds its values AT THE LINE and derives the additions from the
/// printed operands, exactly like every other filed form under the §3.1 election. (SPEC §3.1 listed \"SE\"
/// among the unprinted worksheets — a contradiction with §7.1's fill set, amended alongside this work.)
///
/// The citations bind it to its neighbours, and the form states each one:
/// - **line 2** ← Schedule C's printed **line 31** (\"Net profit or (loss) from Schedule C, line 31\")
/// - **line 12** → Schedule 2 **line 4** (\"Enter here and on Schedule 2 (Form 1040), line 4\")
/// - **line 13** → Schedule 1 **line 15** (\"Enter here and on Schedule 1 (Form 1040), line 15\")
/// - **line 6** ← cited BY Form 8959 line 8 (\"from Schedule SE, Part I, line 6\")
#[derive(Debug, Clone, Copy, PartialEq, Eq)]

=== form1040.rs:105-130 push_literal ===

    // Capital-gain line (7a in 2025 / 7 in 2024 / **13 in 2017**) — only when Schedule D is ACTIVE and
    // line 16 ≥ 0. The cell is single (2024/2025) or a dollars+cents pair (2017), handled uniformly.
    let mut filled_7a = false;
    let mut active_zero = false;
    let mut loss = false;
    if inputs.schedule_d_active {
        if inputs.schedule_d_line16 < Usd::ZERO {
            loss = true; // net loss → line BLANK + notice (§1211 line-21 cap is the filer's).
        } else if inputs.schedule_d_line16.is_zero() {
            active_zero = true; // active-and-netted-to-zero → the \"-0-\" literal.
            push_literal(
                &mut writes,
                &mut placements,
                &map.line7a,
                \"-0-\",
                F1040_COL_AMOUNT,
            );
            filled_7a = true;
        } else {
            push_money(
                &mut writes,
                &mut placements,
                &map.line7a,
                inputs.schedule_d_line16,
                F1040_COL_AMOUNT,

=== Form6251 Default used in tests? ===
crates/btctax-core/src/tax/form6251.rs:63:impl Default for Form6251Line1 {
crates/btctax-core/src/tax/form6251.rs:64:    /// TY2024's shape, matching `Form6251::default()` and every committed fixture vector.","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"7675ea9b-75c7-41d8-a60c-a8f7241b506f","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"28b6676f-a42b-4f9e-9d94-c755ebb19c3d","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbS9thhcyBtoMhVKXgLF","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"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"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2894,"cache_read_input_tokens":102450,"cache_creation":{"ephemeral_5m_input_tokens":2894,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbS9siRG5cbyQg5Cr19c","attributionAgent":"general-purpose","type":"assistant","uuid":"4720939d-dabe-4097-8b03-7d4f73a1e6fc","timestamp":"2026-08-01T04:56:51.030Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"4720939d-dabe-4097-8b03-7d4f73a1e6fc","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbS9thhcyBtoMhVKXgLF","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01HPiyV7z3Rycx5d6gSc8zqo","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax
echo \"=== Part1Item ===\"; grep -n -A18 \"pub struct Part1Item\" crates/btctax-core/src/tax/form8275.rs | head -30
echo; echo \"=== non-Usd struct-typed money-bearing fields inside printed.rs structs ===\"
grep -nE \"^\\s+pub [a-z0-9_]+: (crate::|Vec<crate::|Vec<[A-Z])\" crates/btctax-core/src/tax/printed.rs | head -20"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2894,"cache_read_input_tokens":102450,"output_tokens":1410,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":2894},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":1410,"cache_read_input_tokens":102450,"cache_creation_input_tokens":2894,"cache_creation":{"ephemeral_5m_input_tokens":2894,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbS9siRG5cbyQg5Cr19c","attributionAgent":"general-purpose","type":"assistant","uuid":"f30ab56b-2d7b-4060-8cda-a70affc67b4e","timestamp":"2026-08-01T04:56:53.923Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"f30ab56b-2d7b-4060-8cda-a70affc67b4e","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01HPiyV7z3Rycx5d6gSc8zqo","type":"tool_result","content":"=== Part1Item ===
53:pub struct Part1Item {
54-    /// The filed form this position is taken on — always `\"8949\"` here (BG-D11: a removal leg never
55-    /// contributes a Part I item; see module doc).
56-    pub form: String,
57-    /// The column/line the position occupies (e.g. `\"Part I \\u{2014} column (e)\"` for a short-term
58-    /// leg, `\"Part II \\u{2014} column (e)\"` for long-term — Form 8949's own Part I/II split).
59-    pub line: String,
60-    /// The Cohan-estimate explanation, `NO_LOSS_SUFFIX`-appended when BG-D4's clamp bound.
61-    pub description: String,
62-    /// The AS-FILED Form 8949 col (e) basis for this leg (`leg.basis` — the clamped amount where the
63-    /// clamp bound, NEVER the pre-clamp floor).
64-    pub amount: Usd,
65-}
66-
67-/// Form 8275 content: Part I (auto) + Part II (the filer's own narrative) + the incompleteness flag
68-/// T14's export-refusal gate keys on.
69-#[derive(Debug, Clone, PartialEq, Eq)]
70-pub struct Disclosure8275 {
71-    /// One item per promoted Form 8949 disposal leg filed in `year` (BG-D11: never a removal leg).

=== non-Usd struct-typed money-bearing fields inside printed.rs structs ===
41:    pub date_acquired: crate::conventions::TaxDate,
43:    pub date_sold: crate::conventions::TaxDate,
67:    pub short_term: Vec<Printed8949Row>,
68:    pub long_term: Vec<Printed8949Row>,
198:    pub part_i: Vec<crate::tax::form8275::Part1Item>,
955:    pub part1_rows: Vec<ScheduleBRow>,
962:    pub part2_rows: Vec<ScheduleBRow>,","is_error":false}]},"uuid":"01d8fced-fdec-41a1-b293-c585bb0e3d9c","timestamp":"2026-08-01T04:56:54.017Z","toolUseResult":{"stdout":"=== Part1Item ===
53:pub struct Part1Item {
54-    /// The filed form this position is taken on — always `\"8949\"` here (BG-D11: a removal leg never
55-    /// contributes a Part I item; see module doc).
56-    pub form: String,
57-    /// The column/line the position occupies (e.g. `\"Part I \\u{2014} column (e)\"` for a short-term
58-    /// leg, `\"Part II \\u{2014} column (e)\"` for long-term — Form 8949's own Part I/II split).
59-    pub line: String,
60-    /// The Cohan-estimate explanation, `NO_LOSS_SUFFIX`-appended when BG-D4's clamp bound.
61-    pub description: String,
62-    /// The AS-FILED Form 8949 col (e) basis for this leg (`leg.basis` — the clamped amount where the
63-    /// clamp bound, NEVER the pre-clamp floor).
64-    pub amount: Usd,
65-}
66-
67-/// Form 8275 content: Part I (auto) + Part II (the filer's own narrative) + the incompleteness flag
68-/// T14's export-refusal gate keys on.
69-#[derive(Debug, Clone, PartialEq, Eq)]
70-pub struct Disclosure8275 {
71-    /// One item per promoted Form 8949 disposal leg filed in `year` (BG-D11: never a removal leg).

=== non-Usd struct-typed money-bearing fields inside printed.rs structs ===
41:    pub date_acquired: crate::conventions::TaxDate,
43:    pub date_sold: crate::conventions::TaxDate,
67:    pub short_term: Vec<Printed8949Row>,
68:    pub long_term: Vec<Printed8949Row>,
198:    pub part_i: Vec<crate::tax::form8275::Part1Item>,
955:    pub part1_rows: Vec<ScheduleBRow>,
962:    pub part2_rows: Vec<ScheduleBRow>,","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"f30ab56b-2d7b-4060-8cda-a70affc67b4e","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"01d8fced-fdec-41a1-b293-c585bb0e3d9c","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbSBT9s7a2AiAKkwsbAi","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"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"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2112,"cache_read_input_tokens":105344,"cache_creation":{"ephemeral_5m_input_tokens":2112,"ephemeral_1h_input_tokens":0},"output_tokens":10,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbSBRe5WXiCqpAkxDhGE","attributionAgent":"general-purpose","type":"assistant","uuid":"6673eece-e7f9-4017-b04e-9778fcdaf7d9","timestamp":"2026-08-01T04:57:03.210Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"6673eece-e7f9-4017-b04e-9778fcdaf7d9","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbSBT9s7a2AiAKkwsbAi","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01YSgcXUBqg8aS5qBFnj31rh","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax
F=\"crates/btctax-core/src/tax/printed.rs crates/btctax-core/src/tax/other_taxes.rs crates/btctax-core/src/tax/qbi.rs crates/btctax-core/src/tax/form6251.rs crates/btctax-core/src/forms.rs crates/btctax-core/src/tax/form8275.rs\"
echo \"=== Default derives across the five listed files + form8275 ===\"; grep -n \"derive.*Default\" $F
echo; echo \"=== impl From / impl Default in those files ===\"; grep -n \"^impl From\\|impl Default for\" $F
echo; echo \"=== struct-update (..expr) in printed.rs ===\"; grep -nE \"\\.\\.[a-z_]+(\\.clone\\(\\))?,?$|\\.\\.i\\.clone\" crates/btctax-core/src/tax/printed.rs | head
echo; echo \"=== printed.rs:205-215 ===\"; sed -n '205,216p' crates/btctax-core/src/tax/printed.rs"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2112,"cache_read_input_tokens":105344,"output_tokens":965,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":2112},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":965,"cache_read_input_tokens":105344,"cache_creation_input_tokens":2112,"cache_creation":{"ephemeral_5m_input_tokens":2112,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbSBRe5WXiCqpAkxDhGE","attributionAgent":"general-purpose","type":"assistant","uuid":"1330fd74-f6eb-4fc8-89c8-e83be7ec8080","timestamp":"2026-08-01T04:57:06.938Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"1330fd74-f6eb-4fc8-89c8-e83be7ec8080","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01YSgcXUBqg8aS5qBFnj31rh","type":"tool_result","content":"=== Default derives across the five listed files + form8275 ===
ugrep: warning: crates/btctax-core/src/tax/printed.rs crates/btctax-core/src/tax/other_taxes.rs crates/btctax-core/src/tax/qbi.rs crates/btctax-core/src/tax/form6251.rs crates/btctax-core/src/forms.rs crates/btctax-core/src/tax/form8275.rs: No such file or directory

=== impl From / impl Default in those files ===
ugrep: warning: crates/btctax-core/src/tax/printed.rs crates/btctax-core/src/tax/other_taxes.rs crates/btctax-core/src/tax/qbi.rs crates/btctax-core/src/tax/form6251.rs crates/btctax-core/src/forms.rs crates/btctax-core/src/tax/form8275.rs: No such file or directory

=== struct-update (..expr) in printed.rs ===
180:                ..r.clone()
211:                ..i.clone()

=== printed.rs:205-215 ===
    Printed8275 {
        part_i: d
            .part_i
            .iter()
            .map(|i| crate::tax::form8275::Part1Item {
                amount: round_dollar(i.amount),
                ..i.clone()
            })
            .collect(),
        part_ii: d.part_ii.clone(),
    }
}","is_error":false}]},"uuid":"3a9c5294-b7ed-44ff-b482-7bbee731f95c","timestamp":"2026-08-01T04:57:07.081Z","toolUseResult":{"stdout":"=== Default derives across the five listed files + form8275 ===
ugrep: warning: crates/btctax-core/src/tax/printed.rs crates/btctax-core/src/tax/other_taxes.rs crates/btctax-core/src/tax/qbi.rs crates/btctax-core/src/tax/form6251.rs crates/btctax-core/src/forms.rs crates/btctax-core/src/tax/form8275.rs: No such file or directory

=== impl From / impl Default in those files ===
ugrep: warning: crates/btctax-core/src/tax/printed.rs crates/btctax-core/src/tax/other_taxes.rs crates/btctax-core/src/tax/qbi.rs crates/btctax-core/src/tax/form6251.rs crates/btctax-core/src/forms.rs crates/btctax-core/src/tax/form8275.rs: No such file or directory

=== struct-update (..expr) in printed.rs ===
180:                ..r.clone()
211:                ..i.clone()

=== printed.rs:205-215 ===
    Printed8275 {
        part_i: d
            .part_i
            .iter()
            .map(|i| crate::tax::form8275::Part1Item {
                amount: round_dollar(i.amount),
                ..i.clone()
            })
            .collect(),
        part_ii: d.part_ii.clone(),
    }
}","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"1330fd74-f6eb-4fc8-89c8-e83be7ec8080","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"3a9c5294-b7ed-44ff-b482-7bbee731f95c","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbSCQarbuGufPsTpqNXv","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISlwkKhwEIEBgCKkDq0DJm/ZmltaiOxbL2eBKVELUsCgcYOyrcMAFLAHRpxhzyNdkVpf2ZEDRukCZSSz8LBWms/b5LIctt/chla8dQMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDHOV22s7+DhBv7nGehoM/gLxkSC/STKj3fCcIjChTmu6sZi+PYQzEiv1a2afH4jZaRNjy1Kr+bqsnT3Gtrv322MuTLJBifspRUFuGdcqvAdqsvrJJsKa+kUeAqp4jMv5FYYZxASS4eMEVOJ1YkkUIll4T4dXiw5mf1S4qZUOgIKHB4deTf/xFUM/WFheKxtV3yyn5PuzfIHBwwo/EjJyfx+Whl/skgpKl3E6Qbggn8crwFBW3ETIscQ9v9He4S8MuQVOTtsC7ktrRiBPV4XzEcxIUuGz2VUmLyxFjn8GEaOEgnxWN6qVagz+gRkgk3hUc7nCXPfn4fjvzt6EIRy0oDAujhbl/Bsm17yAWe2Fv3EftaSPmk08wE+meMTV/LoiDVteDqgAd2m3yueHfSnzq7Z1VcuhAa91OcaIx65pFqN/j4LhG3rt6z6ypxKI7QYIbqG77KyVMhVd7xAb9H0pzJ7BB4JfiE866MVh0JZmhxg8KWriHs5/4cj+l1vSQ0nciay03qdpEUGFMpc4dP1mfSgGMbIcAvLbohoqh+x0n7deMGwIPXqqqSmCSlOVUBGHeWTGPsA3X9oyodDcWrrMMWN3xJutyDdHbZq+EnpRGvo1H0VIBgGtqy6+2VI+bGrzhRvHc/khCps7Bqb4R1WxASzjhK5WBtnGyiGnCVKk2yJAxuDmH0+CDCBX7LJTwpT4yi00/dW4kGLCUdhlFZMK/TVCHZ4Up3RL2k4kHYqGlMPMoR5KvfX3JgrS4QnomHoJzqVN0kQJQfW/k+cXnU9IUj0yw47hYgZ+3rfrEb01FuNTBUbnR/C8hUqFZuRLZUkuMEKZn3F36yPmrKxFcjMtNq2wjImrLfoXxH+2p9/sSYhoqBJI9O5g8TK7WmRbNr8dPx4XvlTyuD+U7AaFPQxWehG7ElDOwGZYAvA6qSZZYVxtRDLxp0rLe3gbnTxmUe1u7sP7h7/KWTuHDs3GPM8IFtvnke1qyG0Lm6RyZTje5PYcLwb6WO9KkYdcZUoRL/Lu8Sokv5sd6ggOP+Q2L9cJDHA31/prrSA0M99BxFhimpbRpuaDfVI7QsSeLSwkqJZ5MGuDWM3gLWtN1e4nl+0kT1HYil6YPggdpWtGZ7cCTaEm1AlLxWmabwNauVbbZc6ccDzootgnvdC/uj1UgS52Ya2zm45Mz0LQnsjPMs9s+nd704W19f+ycKhIP5yQLnB5AnAnORmbBQ6+jmZMsQOwC2jJ4ZIsvMDy3Sl6JCnHiz+W6AP40SKgnP8Z79vXPOf1XDaVkbaAvjdvxnqRsN/+wXDFLiqWTiAWy44I+j705wz6zSPQwTlYN+jdflFO00A4/rRfyPa4hYk2tURYcvnAii9FvoOJ9vngGpsuGhgB"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1438,"cache_read_input_tokens":107456,"cache_creation":{"ephemeral_5m_input_tokens":1438,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbSCPUswLTtQehiXNtw2","attributionAgent":"general-purpose","type":"assistant","uuid":"85e69edd-ab83-43df-9886-32755bed2f8b","timestamp":"2026-08-01T04:57:13.591Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"85e69edd-ab83-43df-9886-32755bed2f8b","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbSCQarbuGufPsTpqNXv","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_011b3HFukVH6cyMeESricU6F","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax; sed -n '160,195p' crates/btctax-core/src/tax/printed.rs; echo \"=== Printed8949Row fields ===\"; sed -n '36,64p' crates/btctax-core/src/tax/printed.rs"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1438,"cache_read_input_tokens":107456,"output_tokens":513,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":1438},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":513,"cache_read_input_tokens":107456,"cache_creation_input_tokens":1438,"cache_creation":{"ephemeral_5m_input_tokens":1438,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbSCPUswLTtQehiXNtw2","attributionAgent":"general-purpose","type":"assistant","uuid":"6d4a05ac-77bc-4785-b18f-721172084faf","timestamp":"2026-08-01T04:57:14.658Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"6d4a05ac-77bc-4785-b18f-721172084faf","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_011b3HFukVH6cyMeESricU6F","type":"tool_result","content":"///
/// The PRESENCE rule (does an 8283 get attached at all?) is the packet's, not this function's: the form
/// is required when the return itemizes AND the printed Schedule A line 12 exceeds $500.
pub fn form_8283_printed(
    rows: &[crate::forms::Form8283Row],
    // ★ §G-21 — the filer's own answer to lines 5a/5b/5c. `Some(false)` prints all three as No; `None`
    // leaves them blank. `Some(true)` cannot arrive: the year refuses upstream.
    no_donation_restrictions: Option<bool>,
) -> Option<Printed8283Rows> {
    if rows.is_empty() {
        return None;
    }
    Some(Printed8283Rows {
        no_donation_restrictions,
        rows: rows
            .iter()
            .map(|r| crate::forms::Form8283Row {
                cost_basis: round_dollar(r.cost_basis),
                fmv: round_dollar(r.fmv),
                claimed_deduction: r.claimed_deduction.map(round_dollar),
                ..r.clone()
            })
            .collect(),
    })
}

/// Form 8283 is REQUIRED when the return itemizes and its printed noncash gifts exceed $500 — the
/// threshold is printed on Schedule A line 12 itself (\"You must attach Form 8283 if over $500\").
pub const FORM_8283_THRESHOLD: Usd = rust_decimal_macros::dec!(500);

// ── Form 8275 — the printed Part I amounts (Task 13, arch r1 I-5) ──────────────────────────────────

/// The PRINTED Form 8275 content: Part I items with `amount` whole-dollar rounded for the filed
/// packet (mirrors `Printed8283Rows` above), and Part II carried through unrounded (it is text, not
/// money). Unlike `Printed8283Rows` this is a plain struct, not a newtype — Part II has no
/// exact-cents twin to guard against mixing with (T15/T16's Produces shape, arch r1 I-5).
=== Printed8949Row fields ===
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printed8949Row {
    /// (a) description — the exact BTC amount, 8dp (never rounded: it is a quantity, not money).
    pub description: String,
    /// (b) date acquired.
    pub date_acquired: crate::conventions::TaxDate,
    /// (c) date sold.
    pub date_sold: crate::conventions::TaxDate,
    /// (d) proceeds — `round_dollar` at the cell.
    pub proceeds_d: Usd,
    /// (e) cost basis — `round_dollar` at the cell.
    pub cost_e: Usd,
    /// (h) gain/loss = **printed (d) − printed (e)** (column (g) is always blank in v1).
    pub gain_h: Usd,
}

/// A part's totals row — the sum of the PRINTED rows above it (never a re-rounding of the exact sum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Printed8949Totals {
    pub proceeds_d: Usd,
    pub cost_e: Usd,
    pub gain_h: Usd,
}

/// The printed Form 8949: Part I (short-term) and Part II (long-term), each with its rows and its
/// totals row. The conservative \"not reported\" box is year-aware (C/F pre-TY2025; the digital-asset
/// I/L from TY2025). These totals ARE Schedule D lines 3 and 10 — the schedule's own text defines
/// them as \"Totals for all transactions reported on Form(s) 8949 with Box C/F checked\" pre-2025, and
/// \"with Box C or Box I checked\" (line 3) / \"Box F or Box L checked\" (line 10) on the 2025 revision.","is_error":false}]},"uuid":"7e1db4e7-3bf4-4b65-a6be-5f2bedd83654","timestamp":"2026-08-01T04:57:14.740Z","toolUseResult":{"stdout":"///
/// The PRESENCE rule (does an 8283 get attached at all?) is the packet's, not this function's: the form
/// is required when the return itemizes AND the printed Schedule A line 12 exceeds $500.
pub fn form_8283_printed(
    rows: &[crate::forms::Form8283Row],
    // ★ §G-21 — the filer's own answer to lines 5a/5b/5c. `Some(false)` prints all three as No; `None`
    // leaves them blank. `Some(true)` cannot arrive: the year refuses upstream.
    no_donation_restrictions: Option<bool>,
) -> Option<Printed8283Rows> {
    if rows.is_empty() {
        return None;
    }
    Some(Printed8283Rows {
        no_donation_restrictions,
        rows: rows
            .iter()
            .map(|r| crate::forms::Form8283Row {
                cost_basis: round_dollar(r.cost_basis),
                fmv: round_dollar(r.fmv),
                claimed_deduction: r.claimed_deduction.map(round_dollar),
                ..r.clone()
            })
            .collect(),
    })
}

/// Form 8283 is REQUIRED when the return itemizes and its printed noncash gifts exceed $500 — the
/// threshold is printed on Schedule A line 12 itself (\"You must attach Form 8283 if over $500\").
pub const FORM_8283_THRESHOLD: Usd = rust_decimal_macros::dec!(500);

// ── Form 8275 — the printed Part I amounts (Task 13, arch r1 I-5) ──────────────────────────────────

/// The PRINTED Form 8275 content: Part I items with `amount` whole-dollar rounded for the filed
/// packet (mirrors `Printed8283Rows` above), and Part II carried through unrounded (it is text, not
/// money). Unlike `Printed8283Rows` this is a plain struct, not a newtype — Part II has no
/// exact-cents twin to guard against mixing with (T15/T16's Produces shape, arch r1 I-5).
=== Printed8949Row fields ===
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printed8949Row {
    /// (a) description — the exact BTC amount, 8dp (never rounded: it is a quantity, not money).
    pub description: String,
    /// (b) date acquired.
    pub date_acquired: crate::conventions::TaxDate,
    /// (c) date sold.
    pub date_sold: crate::conventions::TaxDate,
    /// (d) proceeds — `round_dollar` at the cell.
    pub proceeds_d: Usd,
    /// (e) cost basis — `round_dollar` at the cell.
    pub cost_e: Usd,
    /// (h) gain/loss = **printed (d) − printed (e)** (column (g) is always blank in v1).
    pub gain_h: Usd,
}

/// A part's totals row — the sum of the PRINTED rows above it (never a re-rounding of the exact sum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Printed8949Totals {
    pub proceeds_d: Usd,
    pub cost_e: Usd,
    pub gain_h: Usd,
}

/// The printed Form 8949: Part I (short-term) and Part II (long-term), each with its rows and its
/// totals row. The conservative \"not reported\" box is year-aware (C/F pre-TY2025; the digital-asset
/// I/L from TY2025). These totals ARE Schedule D lines 3 and 10 — the schedule's own text defines
/// them as \"Totals for all transactions reported on Form(s) 8949 with Box C/F checked\" pre-2025, and
/// \"with Box C or Box I checked\" (line 3) / \"Box F or Box L checked\" (line 10) on the 2025 revision.","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"6d4a05ac-77bc-4785-b18f-721172084faf","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"7e1db4e7-3bf4-4b65-a6be-5f2bedd83654","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbSCyhfzyQuRewcKF8Rb","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"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"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1863,"cache_read_input_tokens":108894,"cache_creation":{"ephemeral_5m_input_tokens":1863,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbSCxFc5Rk28XgMiH4vE","attributionAgent":"general-purpose","type":"assistant","uuid":"1e987036-e09d-4a7e-ab0b-19aa3b0e4e36","timestamp":"2026-08-01T04:57:26.120Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"1e987036-e09d-4a7e-ab0b-19aa3b0e4e36","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbSCyhfzyQuRewcKF8Rb","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01WEZo73yiLL16LErbNFq1cs","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax; grep -n -A30 \"pub struct Form8283Row\" crates/btctax-core/src/forms.rs | grep -E \"pub [a-z_]+:\" "},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1863,"cache_read_input_tokens":108894,"output_tokens":786,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":1863},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":786,"cache_read_input_tokens":108894,"cache_creation_input_tokens":1863,"cache_creation":{"ephemeral_5m_input_tokens":1863,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbSCxFc5Rk28XgMiH4vE","attributionAgent":"general-purpose","type":"assistant","uuid":"4224727a-6aa9-4769-9afa-f4b9f53dd938","timestamp":"2026-08-01T04:57:26.951Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"4224727a-6aa9-4769-9afa-f4b9f53dd938","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01WEZo73yiLL16LErbNFq1cs","type":"tool_result","content":"304-    pub section: Option<Form8283Section>,
306-    pub description: String,
308-    pub how_acquired: Form8283HowAcquired,
311-    pub date_acquired: TaxDate,
313-    pub date_contributed: TaxDate,
315-    pub cost_basis: Usd,
317-    pub fmv: Usd,
320-    pub claimed_deduction: Option<Usd>,
325-    pub fmv_method: String,
329-    pub donee: String,","is_error":false}]},"uuid":"6c4ff966-c418-440e-86f9-75abf38d00a7","timestamp":"2026-08-01T04:57:27.036Z","toolUseResult":{"stdout":"304-    pub section: Option<Form8283Section>,
306-    pub description: String,
308-    pub how_acquired: Form8283HowAcquired,
311-    pub date_acquired: TaxDate,
313-    pub date_contributed: TaxDate,
315-    pub cost_basis: Usd,
317-    pub fmv: Usd,
320-    pub claimed_deduction: Option<Usd>,
325-    pub fmv_method: String,
329-    pub donee: String,","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"4224727a-6aa9-4769-9afa-f4b9f53dd938","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"6c4ff966-c418-440e-86f9-75abf38d00a7","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbSDt7bGVoPKYHrkR1ZF","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"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"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":955,"cache_read_input_tokens":110757,"cache_creation":{"ephemeral_5m_input_tokens":955,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbSDroDZw8eBn4Latgro","attributionAgent":"general-purpose","type":"assistant","uuid":"7f70239a-755a-4838-a9c4-0de616ac3a58","timestamp":"2026-08-01T04:58:22.983Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"7f70239a-755a-4838-a9c4-0de616ac3a58","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbSDt7bGVoPKYHrkR1ZF","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_012v9YAbVWiSEEPd4Yt76Nvb","name":"Bash","input":{"command":"cd /scratch/code/bitcoin_tax; grep -n \"pub amt:\" crates/btctax-core/src/tax/return_1040.rs | head -3; grep -rn \"pub f8959:\\|pub f8960:\\|pub f8995:\\|pub sch_d:\" crates/btctax-core/src/tax/packet.rs | head -6"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":955,"cache_read_input_tokens":110757,"output_tokens":4367,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":955},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":4367,"cache_read_input_tokens":110757,"cache_creation_input_tokens":955,"cache_creation":{"ephemeral_5m_input_tokens":955,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbSDroDZw8eBn4Latgro","attributionAgent":"general-purpose","type":"assistant","uuid":"1f15f830-f818-430f-9d43-fc66d9235e6d","timestamp":"2026-08-01T04:58:24.197Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"1f15f830-f818-430f-9d43-fc66d9235e6d","isSidechain":true,"promptId":"879dea29-ba18-4d69-b48f-aafb86f9110d","agentId":"ae58f1d220d8c3940","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_012v9YAbVWiSEEPd4Yt76Nvb","type":"tool_result","content":"965:    pub amt: crate::tax::form6251::Form6251,
crates/btctax-core/src/tax/packet.rs:492:    pub sch_d: ScheduleDLines,
crates/btctax-core/src/tax/packet.rs:503:    pub f8959: Form8959Lines,
crates/btctax-core/src/tax/packet.rs:504:    pub f8960: Option<Form8960Lines>,
crates/btctax-core/src/tax/packet.rs:505:    pub f8995: Option<Form8995Lines>,","is_error":false}]},"uuid":"2dcc248b-d37d-4776-8cdd-014801b8dab3","timestamp":"2026-08-01T04:58:24.307Z","toolUseResult":{"stdout":"965:    pub amt: crate::tax::form6251::Form6251,
crates/btctax-core/src/tax/packet.rs:492:    pub sch_d: ScheduleDLines,
crates/btctax-core/src/tax/packet.rs:503:    pub f8959: Form8959Lines,
crates/btctax-core/src/tax/packet.rs:504:    pub f8960: Option<Form8960Lines>,
crates/btctax-core/src/tax/packet.rs:505:    pub f8995: Option<Form8995Lines>,","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"1f15f830-f818-430f-9d43-fc66d9235e6d","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"2dcc248b-d37d-4776-8cdd-014801b8dab3","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbSJ9Mz5bjFWmh9PvZzY","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAIS0gUKhwEIEBgCKkDfHOUnqu39KfJRubsmexitl0+qAMrgp44VqSBn1fFLfhsuHI8oJcb4ujT+c8GDmDh1pyMfo9+BuaEeYZrqvHBcMg1jbGF1ZGUtb3B1cy01OAFCCHRoaW5raW5nWiRlMmZiYWM5MC0wMzhkLTQyNjMtYWMwNS1lZTE1NWUwY2NlODYSDPB0bjpNRty23Dr+nBoM13QEfR2pfzgc3mDPIjDyKgrlmMvrLHMzMQX1/N9atNQkPUyNQwP1+7oYfxEgf/54ZE61iqn1y5nUR3XbXP4q9wMocJaAYrdydimMVqNFRirft47CGdnag0HStWYISpxx5zKaAeXXnL7dIWS8n3AfIf5Rdv/dH54nB7x0jNC6BFYXTnwRVNQNPP29d7P6uo3ObIIGxBEaebxnGcSG9RMy61LprvsAEJOt1bKbSubiGMv090JE61Hz8264HhVl1sWe6YtcHpcVtPGrGsvmkLtk7zdTDtGNQsNAtanpNLvzVPz/oc/+iCfcu1oXHiXCSJeJvyFTMJEIsHSUyxCOjPdFLu5bQv0zse0v2BZ7j/r+FIM9R3r+BozmqHamY6dNHkcGJFS01cTI4N130WoS0mb5FO0O/les5oRCaHOEEK1iFdRgpRMZVa7o4pWeJue/GDaqhkKEN/YcPl8v5XaFikvPrRHFZIX2dTdJht2Gdtp+OxFzcwmg+aSh0Nom1uFFC2dGt/tMcE9eiAoI6fPE7xRUTKq4G2FJK0vkmN2CFDeB1bFtIRO9R5fHKQBI9KRRlA+rgwRVQUUR3FgHkaC8HFS8XnM7MkrosVgUHgh5FKxbXTeVl8J3F2vctyFNbC+IO42+4bYV3L6piQgthlOsAT/IzeTsXZKTd5MGy4wxC8IYl4PwItAKhSR0BwuyXmg7RMbTFvRrwU8Ybs8nCH/sH5sntp16MzCwd8ilEv3CyA3gkv4Mx46RGdMaihgB"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":4557,"cache_read_input_tokens":111712,"cache_creation":{"ephemeral_5m_input_tokens":4557,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbSJ5eFL9GKgUTwworGk","attributionAgent":"general-purpose","type":"assistant","uuid":"2105e5d0-67df-4352-9722-f0fcaad0ef39","timestamp":"2026-08-01T04:58:28.699Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"2105e5d0-67df-4352-9722-f0fcaad0ef39","isSidechain":true,"agentId":"ae58f1d220d8c3940","message":{"model":"claude-opus-5","id":"msg_011CdbSJ9Mz5bjFWmh9PvZzY","type":"message","role":"assistant","content":[{"type":"text","text":"VERDICT: 0 Critical / 2 Important

---

SEVERITY: Important
WHERE: SPEC.md §2.2 (\"Scope — four files, not one\") and §7 P0b, against `crates/btctax-core/src/forms.rs:170/184/200-218`, `crates/btctax-forms/src/schedule_d.rs:35/108`, `crates/btctax-core/src/tax/form8275.rs:64`
CLAIM: The scope is a hand-list of files, and it is wrong in both directions. `forms.rs`'s `ScheduleDTotals`/`ScheduleDPart` is a **ledger aggregate, not a printed-line struct** — §3's grammar has no production for it and P0b's \"byte-unchanged, all entered\" is not mechanical there. Meanwhile a money field that *does* reach paper, `form8275.rs::Part1Item::amount`, is in none of the listed files.
CONSEQUENCE: P0b cannot meet \"the same bar as P0\" for the `forms.rs` member without rewriting a core aggregation function and a published-crate public API type; and §4's \"a 65th site cannot choose a silent zero\" still has a named counterexample. This is the C-1 failure one level up again — an enumeration by *outcome observed* rather than by *mechanism*, which `CLAUDE.md` bans for line sets and for oracle excuse lists for exactly this reason.
EVIDENCE:
- **Over-broad.** `forms.rs:200-218 pub fn schedule_d(state, year) -> ScheduleDTotals` opens with `let mut totals = ScheduleDTotals::default();` and accumulates `part.proceeds += leg.proceeds` over `state.disposals` legs. With `LineEntry` (no `Default`, no `Add`/`AddAssign` per §2.2) this does not compile, and no §3 production covers \"Σ over ledger legs\": production 3 is *\"Add / Combine lines X–Y\"*, whose operands are lines on a page — here there are none. The type is `btctax-core` public API (`lib.rs:27`) and is consumed **numerically outside any emitter**: `btctax-cli/src/render.rs:1611-1631` (six terminal `fmt_money` reads), `cmd/admin.rs:352 sd_part_active` and `schedule_d.rs:35 active` (three `.is_zero()` each — a liveness predicate), and it is a field of `cmd/tax.rs:241 TaxYearReport`. A reconciliation KAT hangs off it (`forms.rs:195-199`, the R0-M3 cross-check against `compute_tax_year`).
- **The non-`push_money` path is still untraced.** §2.2 flags `ScheduleDTotals` as \"the crypto slice, which never calls `push_money` at all\" and then does not re-trace it. It does arithmetic before formatting: `schedule_d.rs:108 let total = totals.st.gain + totals.lt.gain;` → `fmt_money(total)` at :116. r1's byte-identity proof was of `push_money` (`cells.rs:44-69`); nothing covers this. The other non-`push_money` writers are benign — `push_literal` is money at only one site (`form1040.rs:116`, the `\"-0-\"` literal, already P4 residue), and `form8275.rs:240` is the gap below.
- **The `Default` inventory is scoped to the file the fold widened past.** §2.2: \"`Printed8949Totals` (`printed.rs:53`) carries the layer's only `#[derive(Default)]`\"; P0's checklist says \"Delete the `Printed8949Totals` `Default` derive\" (singular). Under r2's own scope there are three more: `forms.rs:170 ScheduleDPart`, `forms.rs:184 ScheduleDTotals`, `form6251.rs:87 Form6251`. `forms.rs:201` and `btctax-forms/tests/common/mod.rs:91` are live `::default()` calls, so two are load-bearing — this is not \"delete three instead of one\".
- **Incomplete the other way.** `crates/btctax-core/src/tax/form8275.rs:64 pub amount: Usd` reaches paper via `printed.rs:198 part_i: Vec<crate::tax::form8275::Part1Item>` → `crates/btctax-forms/src/form8275.rs:240 push_free(…, &fmt_money(item.amount))`. `form8275.rs` appears in no scope list.
- §2.2's header says \"**four files, not one**\" and then lists five. Cosmetic, but it is the tell.

---

SEVERITY: Important
WHERE: SPEC.md §2.2 (\"an ordering comparator for the three emitter sites that compare\") and §6
CLAIM: The published comparator is unspecified on `Blank`, the natural implementation (a derived `Ord`) silently answers, and there are **six** production comparison sites, not three — two of them in the published `btctax-cli`, routing the return.
CONSEQUENCE: A comparator returning `bool` for a `Blank` operand is a liveness predicate answering for the filer — the exact class §2.2 exists to close, one level up (`MEMORY`: \"a value, a **LIVENESS PREDICATE**, or a mis-scoped question\"). r1 proposed `Option<Ordering>`; r2 dropped the `Option` when it folded, and named three of the six sites.
EVIDENCE:
- **Measured** (scratch workspace, rustc 1.97.0-nightly): `pub struct LineEntry(Repr); enum Repr { Entered(Usd), Blank }` with derived `PartialOrd, Ord` gives `Blank > Entered(0) == true`, `Blank.cmp(&Entered(0)) == Greater`, `Blank == Entered(0) == false`. Variant declaration order decides every comparison, silently.
- The three sites r2 names: `schedule_se_full.rs:78 lines.line9 == Usd::ZERO` (this one *is* the SKIP predicate §3 production 3 depends on), `form8995.rs:166 if lines.line2 > Usd::ZERO` (a fail-closed refusal), `schedule_d_full.rs:61 if v < Usd::ZERO` (the parenthesized-magnitude guard, which also formats `{v}` into its message ⇒ `Display` too).
- Three r2 does **not** name: `btctax-cli/src/render.rs:1567 if f.line34 > Usd::ZERO` — the **REFUND vs AMOUNT OWED** branch of the dual report (a derived `Ord` routes a blank to REFUND); `render.rs:1541 if ar.amt.line6 > Usd::ZERO` — gates the entire Form 6251 disclosure block (`ar.amt` is `Form6251`, `return_1040.rs:965`); and `form6251.rs:246 must_attach() { self.line7 > self.line10 }` — a `LineEntry`-vs-`LineEntry` compare deciding whether the form is attached at all (i6251 \"Who Must File\" condition 1).
- Signature is also unstated: five of the six compare a field against `Usd::ZERO`, one compares two fields. `PartialOrd<Usd>`, or force `constant(Usd::ZERO)` on the RHS? §2.2 says neither, and the two shapes have different blank semantics.
- Cheap and in the spec's own idiom: publish `partial_cmp` returning `None` when either side is blank, forbid derived `Ord`/`PartialOrd` on `LineEntry` in the same sentence that forbids `Default`/`From`/`Add`, and give each of the six sites a blank branch in the P0 coverage table.

---

SEVERITY: Minor
WHERE: SPEC.md §2.2 (\"Published exits … `fmt()` for the emitter\")
CLAIM: One formatter cannot serve the emitter. `push_money`'s `MoneyCell::Pair` branch (`cells.rs:57-58`) needs `fmt_money_pair(value)` → `(dollars, cents)`, which no single formatted string yields (`fmt_money` is the raw `Decimal` `Display`, padding no cents — `cells.rs:32`).
CONSEQUENCE: `cells.rs` — the primary emitter — must use the §6 *numeric* exit and carry a visible `#[allow(clippy::disallowed_methods)]`. That is fine and honest, but §2.2 reads as if a formatter suffices, and an implementer who believes it will reach for a workaround at the Pair branch.
EVIDENCE: The Pair branch is live: `crates/btctax-forms/forms/2017/{f1040,schedule_se,f8283}.map.toml` carry `dollars_field`/`cents_field`.

SEVERITY: Minor
WHERE: SPEC.md §4 (\"★ **Verified clean** (engineering): … no builder, no struct-update syntax\")
CLAIM: False as stated. `printed.rs:180 ..r.clone()` (`crate::forms::Form8283Row`) and `printed.rs:211 ..i.clone()` (`Part1Item`) are both struct-update sites on money-bearing structs, inside `printed.rs` itself.
CONSEQUENCE: The **guarantee survives** — FRU cannot supply a `Usd` where a `LineEntry` is declared, and all three of `Form8283Row`'s money fields (`cost_basis`, `fmv`, `claimed_deduction`) are named explicitly at `printed.rs:176-179`, so nothing money-shaped rides the `..`. But a \"verified clean\" claim that is wrong is the thing B1 exists to stop; it was inherited verbatim from r1's ALSO-CHECKED-SOUND (\"the only two `..ident` uses in the workspace are `reconcile.rs:1337` and `persist.rs:1118`\").

---

## WHAT I CONFIRMED (the fold is real on these)

**The opaque newtype is buildable and correctly priced on the trait axis.** `Usd = rust_decimal::Decimal` (`conventions.rs:8`) is `Copy + Eq + Ord + Hash`, and a newtype over a private enum over it derives `Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord` cleanly — compiled and run. Every derive set in scope is a subset of `{Debug, Clone, Copy, PartialEq, Eq, Default}`: all 16 `printed.rs` structs, `other_taxes.rs:36/95/208/256`, `qbi.rs:21/170`, `form6251.rs:45/87`, `forms.rs:21/41/59/170/184/237/252/298`. **No `Hash`, no `Ord`, no `const` construction of a money field anywhere in scope, no `serde` derive, no `impl From`** — so §4's E0308 story holds and `Default` is the only trait obstacle (Important 1, third bullet). Notably, the equality cross-foot KATs (`packet.rs:1203-1215`) survive on derived `Eq` with no numeric exit at all; only `packet.rs:1224`'s subtraction needs one.

**The `clippy.toml` mechanism works end to end — I ran it, not reasoned about it.** A workspace-root `clippy.toml` with `disallowed-methods = [{ path = \"le_core::printed::LineEntry::amount\", reason = \"…\" }]` produced `error: use of a disallowed method` under exactly the repo's invocation (`cargo clippy --workspace --all-targets --all-features -- -D warnings`, `Makefile:27`), for an *inherent* method on a type in a **different** workspace crate, in **both** a `src/lib.rs` target and a `tests/*.rs` target, with the reason string surfaced and `#[allow(clippy::disallowed_methods)]` named as the only override. One sentence §6 should add: the path only resolves because the module chain is fully public (`lib.rs:17 pub mod tax;` → `tax/mod.rs:22 pub mod printed;`); a private `printed` module would silently make the lint a no-op — a B1-shaped trap, so the `clippy.toml` needs its own planted-defect kill like everything else.

**P0b breaks nothing r1 cleared.** Geometric safety: P0b is all-entered, so no placement is removed and `verify.rs:390-414` sees a byte-identical strictly-descending sequence; the crypto-slice Schedule D verifier is column-band-only by construction (`schedule_d.rs:70-72`), indifferent in both directions. `no_unmapped_filled` (`verify.rs:229`): it constrains *filled* fields to the allowed placement set, so it fails closed in the direction the change moves — retyping removes no writes at P0b and only ever removes writes later. The ~110 extra fields do not touch either.

## WHAT R2 CLOSED:

- **I-1 (scope):** genuinely widened past `printed.rs` — Form 6251, 8959/8960, 8995 are now named. The residue is that the widening was done as a file list rather than derived (Important 1), not that it was refused.
- **I-2 (uniform retyping):** P0b exists, is byte-identity-bound, and is the right shape. It holds for the `push_money` surface; the `forms.rs` member is the exception.
- **I-3 (containment):** closed, and closed *better* than r1 asked. Opacity is buildable, `clippy.toml` verifiably fires, `#[non_exhaustive]` correctly rejected, the `test-support` feature correctly rejected (and independently dead anyway — `make check` passes `--all-features`). §6's consumer inventory now names `render.rs`, the three emitter sites and the harness. What is missing is only the *semantics* of one published exit (Important 2).
- **M-1 (E0308 not E0063), M-3 (the `collect_fields` predicate, no skip-list), M-4 (`golden_packet.rs:321`), M-5 (`no_unmapped_filled` credited):** all folded accurately; I re-checked each citation and each is correct as written.
- **I-4 (third plant):** the stated-zero plant + fixture is the right instrument and `tests/common/mod.rs:201 PresentZero` does exist and does panic on an absent key.

## WHAT WOULD MAKE THIS REVIEW WRONG:

- **If §2.2's `forms.rs` entry means only the emitter-facing shape**, i.e. the intent is to convert `ScheduleDTotals` → a `LineEntry` view inside `schedule_d.rs` while `forms.rs` stays a `Usd` aggregate, then Important 1's over-broad half is a wording fix and drops to Minor. I read it as literal because §2.2 says \"**Every** money field on every struct that reaches paper\" and then names the two `forms.rs` types explicitly. The incomplete half (`Part1Item`), the `Default` count, and the untraced `schedule_d.rs:108` arithmetic stand either way.
- **If the published comparator is already intended to be `Option<Ordering>`** — r1 proposed exactly that and r2 may have meant to carry it — Important 2 reduces to \"name the six sites and their blank branches\", still Important because two of the six are the refund/owe branch and the attach test.
- **I did not run the suite or migrate anything.** All trait/lint behaviour is measured in an isolated two-crate workspace (`…/scratchpad/leprobe`), not against a migrated `printed.rs`; the byte-identity reasoning is from reading `cells.rs`, `schedule_d.rs` and `verify.rs`.
- **I stayed inside my three assigned questions** and did not re-run the doctrine lens's misfit count against the eight productions (brief item 2) — if that count is still ≥5, the grammar fails its own criterion independently of anything here."}],"stop_reason":"end_turn","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":4557,"cache_read_input_tokens":111712,"output_tokens":5631,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":4557},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":5631,"cache_read_input_tokens":111712,"cache_creation_input_tokens":4557,"cache_creation":{"ephemeral_5m_input_tokens":4557,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbSJ5eFL9GKgUTwworGk","attributionAgent":"general-purpose","type":"assistant","uuid":"b52cf541-78de-4987-9506-a08f86949426","timestamp":"2026-08-01T04:59:33.169Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
