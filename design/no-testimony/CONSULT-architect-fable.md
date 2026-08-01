# §G-11 — ARCHITECT CONSULT (Fable)

**Date:** 2026-07-31 · **Tree:** `main` at `07bde38`. Owner approved the escalation and said budget
was not a constraint.

**Brief:** [`BRIEF-architect.md`](./BRIEF-architect.md) — one question (*is there a shape in which an
omission cannot COMPILE, rather than 64 sites plumbed and the 65th forgotten?*), three load-bearing
sub-questions, four settled facts, the hard scope boundary, and an explicit invitation to answer
**"do not build it"** — the repo's most valuable prior consult returned exactly that for §G-14.

**Verdict: BUILD IT — but as TWO TYPES at the two boundaries where information is destroyed, not as
64 site fixes. And the emitter change moves from LAST to FIRST.**

★★★ **It corrected my phasing, and my own document had already conceded the point.** BRAINSTORM §7 put
the emitter last; §10 said the gate must come first. Those are inconsistent, because no blank can exist
anywhere until the writer can express one. The emitter change is a precondition, not a finale — and it
is small (2 literals plus a mechanical signature migration).

★★★ **It answered the gate question, which I could not.** The instrument already exists:
`transcribe.rs::extract_lines` reads a filled PDF back keyed by logical line into a `BTreeMap`, so **an
absent key IS a blank cell on paper**. And it found that the current readback harness deliberately
erases this program's distinction — `Blank::AbsentIsZero`, *"reads an ABSENT key as zero"*. Both
verified by hand.

★★ **It found the repo's own refutation of the obvious design.** `return_inputs.rs:631`, written by a
past author, verbatim: *"`Option<Usd>` is a scalar the `_` rule permits, which would make this
convention again."* The argument against bare `Option<Usd>` was already in the tree; the consult
located it rather than re-deriving it.

★ **And it made the four owner decisions cheap** — each collapses to a constructor choice plus one
expectation-table row, so the architecture does not depend on them and a spec can be written now.

**Author verification before folding:** `Blank::AbsentIsZero` (oracle_sweep_readback.rs:128-138),
`transcribe::extract_lines` returning `BTreeMap<String,String>` (transcribe.rs:40), and the
`Option<Usd>` refutation (return_inputs.rs:620-636) — all confirmed in source.

**Consult reproduced VERBATIM below.**

---

{"parentUuid":null,"isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":"You are an architecture consultant for the btctax repo at /scratch/code/bitcoin_tax (branch `main`, commit `07bde38`).

Read the brief first: /scratch/code/bitcoin_tax/design/no-testimony/BRIEF-architect.md

It names the one question, three load-bearing sub-questions, four settled facts you must not re-derive, the hard scope boundary, and the required output format. Follow it exactly.

The short version: btctax emits US federal tax returns signed under penalty of perjury. A blank line and a printed `0` are different speech acts — a blank says nothing, a `0` swears the amount is zero. There is currently no representation for \"not stated\" anywhere in the money path, so 64 of the 168 money quantities that reach a PDF can print a zero the filer was never asked for. The filed follow-up proposed fixing the emitter; a survey found 62 of the 64 are manufactured upstream, so that fix would address two.

You are being asked for the ARCHITECTURE, before any code exists and before a spec is written. Nothing is committed to. In particular I want to know whether there is a shape in which an omission CANNOT COMPILE, rather than one where 64 sites are individually plumbed and the 65th is forgotten — and what the B1 kill-test is, since every fix here is a suppression and a suppression that over-fires is invisible.

\"Do not build it\" is a first-class answer and the brief says why.

You have a shell. Read the code where it matters — `crates/btctax-core/src/tax/{printed,return_1040,return_inputs,classifier,qbi,other_taxes,form6251}.rs`, `crates/btctax-forms/src/`, and the extracted form text under `design/forms/extract/`. Prefer reading the actual types over reasoning from the brief's summary of them.

CONSTRAINTS: READ-ONLY — no edits, no commits, no git mutations. Do not spawn subagents. Your final message IS the consult; it will be persisted verbatim. Emit exactly the brief's five-section format, nothing before `ANSWER:`."},"uuid":"17c739fb-3b79-4db5-8f6a-3de79cd388f1","timestamp":"2026-08-01T04:04:32.280Z","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"17c739fb-3b79-4db5-8f6a-3de79cd388f1","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","attachment":{"type":"deferred_tools_delta","addedNames":["EnterWorktree","ExitWorktree","Monitor","NotebookEdit","SendMessage","TaskStop","WebFetch","WebSearch","mcp__claude_ai_Gmail__authenticate","mcp__claude_ai_Gmail__complete_authentication","mcp__claude_ai_Google_Calendar__authenticate","mcp__claude_ai_Google_Calendar__complete_authentication","mcp__claude_ai_Google_Drive__authenticate","mcp__claude_ai_Google_Drive__complete_authentication","mcp__plugin_context7_context7__query-docs","mcp__plugin_context7_context7__resolve-library-id","mcp__plugin_playwright_playwright__browser_click","mcp__plugin_playwright_playwright__browser_close","mcp__plugin_playwright_playwright__browser_console_messages","mcp__plugin_playwright_playwright__browser_drag","mcp__plugin_playwright_playwright__browser_drop","mcp__plugin_playwright_playwright__browser_evaluate","mcp__plugin_playwright_playwright__browser_file_upload","mcp__plugin_playwright_playwright__browser_fill_form","mcp__plugin_playwright_playwright__browser_find","mcp__plugin_playwright_playwright__browser_handle_dialog","mcp__plugin_playwright_playwright__browser_hover","mcp__plugin_playwright_playwright__browser_navigate","mcp__plugin_playwright_playwright__browser_navigate_back","mcp__plugin_playwright_playwright__browser_network_request","mcp__plugin_playwright_playwright__browser_network_requests","mcp__plugin_playwright_playwright__browser_press_key","mcp__plugin_playwright_playwright__browser_resize","mcp__plugin_playwright_playwright__browser_run_code_unsafe","mcp__plugin_playwright_playwright__browser_select_option","mcp__plugin_playwright_playwright__browser_snapshot","mcp__plugin_playwright_playwright__browser_tabs","mcp__plugin_playwright_playwright__browser_take_screenshot","mcp__plugin_playwright_playwright__browser_type","mcp__plugin_playwright_playwright__browser_wait_for"],"addedLines":["EnterWorktree","ExitWorktree","Monitor","NotebookEdit","SendMessage","TaskStop","WebFetch","WebSearch","mcp__claude_ai_Gmail__authenticate","mcp__claude_ai_Gmail__complete_authentication","mcp__claude_ai_Google_Calendar__authenticate","mcp__claude_ai_Google_Calendar__complete_authentication","mcp__claude_ai_Google_Drive__authenticate","mcp__claude_ai_Google_Drive__complete_authentication","mcp__plugin_context7_context7__query-docs","mcp__plugin_context7_context7__resolve-library-id","mcp__plugin_playwright_playwright__browser_click","mcp__plugin_playwright_playwright__browser_close","mcp__plugin_playwright_playwright__browser_console_messages","mcp__plugin_playwright_playwright__browser_drag","mcp__plugin_playwright_playwright__browser_drop","mcp__plugin_playwright_playwright__browser_evaluate","mcp__plugin_playwright_playwright__browser_file_upload","mcp__plugin_playwright_playwright__browser_fill_form","mcp__plugin_playwright_playwright__browser_find","mcp__plugin_playwright_playwright__browser_handle_dialog","mcp__plugin_playwright_playwright__browser_hover","mcp__plugin_playwright_playwright__browser_navigate","mcp__plugin_playwright_playwright__browser_navigate_back","mcp__plugin_playwright_playwright__browser_network_request","mcp__plugin_playwright_playwright__browser_network_requests","mcp__plugin_playwright_playwright__browser_press_key","mcp__plugin_playwright_playwright__browser_resize","mcp__plugin_playwright_playwright__browser_run_code_unsafe","mcp__plugin_playwright_playwright__browser_select_option","mcp__plugin_playwright_playwright__browser_snapshot","mcp__plugin_playwright_playwright__browser_tabs","mcp__plugin_playwright_playwright__browser_take_screenshot","mcp__plugin_playwright_playwright__browser_type","mcp__plugin_playwright_playwright__browser_wait_for"],"removedNames":[],"readdedNames":[]},"type":"attachment","uuid":"a55adafe-c06b-48c5-adcb-a1901a8a546b","timestamp":"2026-08-01T04:04:32.285Z","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"a55adafe-c06b-48c5-adcb-a1901a8a546b","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","attachment":{"type":"skill_listing","content":"- cycle-prep: Use when prepping a release cycle from one or more FOLLOWUP slugs (e.g. the user types \"/cycle-prep <slug>...\" or asks to vet a FOLLOWUP before building). Runs a P0 STRICT-GATE recon that verifies each slug's cited file paths / line numbers / factual claims against the CURRENT origin/master source (citations decay every merge), flags drift / structural errors / claim-counting ambiguities, and recommends cycle scope + lockstep. Recon only — does NOT implement.
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
- security-review: Complete a security review of the pending changes on the current branch","skillCount":38,"isInitial":true,"names":["cycle-prep","code-review:code-review","feature-dev:feature-dev","claude-md-management:revise-claude-md","commit-commands:clean_gone","commit-commands:commit-push-pr","commit-commands:commit","frontend-design:frontend-design","superpowers:brainstorming","superpowers:dispatching-parallel-agents","superpowers:executing-plans","superpowers:finishing-a-development-branch","superpowers:receiving-code-review","superpowers:requesting-code-review","superpowers:subagent-driven-development","superpowers:systematic-debugging","superpowers:test-driven-development","superpowers:using-git-worktrees","superpowers:using-superpowers","superpowers:verification-before-completion","superpowers:writing-plans","superpowers:writing-skills","claude-md-management:claude-md-improver","claude-code-setup:claude-automation-recommender","dataviz","artifact-design","artifact-capabilities","update-config","keybindings-help","simplify","fewer-permission-prompts","loop","schedule","claude-api","run","init","review","security-review"]},"type":"attachment","uuid":"7a0a276f-347a-429b-a27a-aa77e488fd72","timestamp":"2026-08-01T04:04:32.285Z","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"7a0a276f-347a-429b-a27a-aa77e488fd72","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNBsA2Mzi1jyqs6ettV","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01H6h9ccKf9nihh16ezuKkHF","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/design/no-testimony/BRIEF-architect.md"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":28201,"cache_read_input_tokens":0,"output_tokens":81,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":28201},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":81,"cache_read_input_tokens":0,"cache_creation_input_tokens":28201,"cache_creation":{"ephemeral_5m_input_tokens":28201,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbNBqK9UTz7CWcjT6fgN","attributionAgent":"general-purpose","type":"assistant","uuid":"ea6fcb1a-f544-445a-bbd5-ca1a3c8c1cd8","timestamp":"2026-08-01T04:04:34.805Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"ea6fcb1a-f544-445a-bbd5-ca1a3c8c1cd8","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01H6h9ccKf9nihh16ezuKkHF","type":"tool_result","content":"1\t# §G-11 — ARCHITECT CONSULT BRIEF
2\t
3\tYou are being asked for **architecture judgment before any code exists.** Nothing is built. No spec is
4\twritten. The brainstorm is one commit old and is explicitly not a decision.
5\t
6\t## Read these two, in order
7\t
8\t1. `/scratch/code/bitcoin_tax/design/no-testimony/BRAINSTORM.md` — the shape, the number, the seven
9\t   mechanisms, my recommended phasing, and the four owner decisions.
10\t2. `/scratch/code/bitcoin_tax/design/no-testimony/MAP-survey.md` — the evidence: 221 money rows
11\t   classified across 16 printed structs + Forms 6251/8959/8960/8995, against the extracted form text.
12\t
13\tThen `/scratch/code/bitcoin_tax/CLAUDE.md` — **\"an entry is testimony\"** and **\"blank is the normal
14\tcase\"** are the authority. And `FOLLOWUPS.md` §G-11 for the original filing.
15\t
16\t## The problem, in four sentences
17\t
18\tA blank line and a printed `0` are different speech acts on a return signed under 26 USC §6065: a blank
19\tsays nothing, a `0` swears the amount **is** zero. btctax has no representation for \"not stated\" in its
20\tmoney path, so **64 of the 168 money quantities that reach a PDF can print a zero the filer was never
21\tasked for** — fabricated testimony under someone else's signature. The filed follow-up proposed growing
22\tthe emitter's money type; the survey found **62 of the 64 are manufactured upstream**, in
23\t`return_inputs.rs`, `return_1040.rs` and the printed-struct constructors, so that fix would address two.
24\tbtctax has exactly three lawful moves per line — **collect** the testimony, **refuse** the return, or
25\tleave **genuinely blank** — and must never silently choose silence and present it as the filer's.
26\t
27\t## THE ONE QUESTION
28\t
29\t**What is the right architecture here — and is there a shape in which an omission CANNOT COMPILE,
30\trather than one where 64 sites are individually plumbed and the 65th is forgotten?**
31\t
32\tThat is the question I most want answered, because the repo's own doctrine says *\"prefer designs in
33\twhich an omission does not compile\"* and I do not know whether that is reachable for this defect. My
34\trecommended phasing (BRAINSTORM §7) treats it as phase 3 of 4. **Tell me if that ordering is wrong, or
35\tif the whole framing is.**
36\t
37\tThree sub-questions I consider load-bearing. Answer them only insofar as they serve the question above:
38\t
39\t- **Is \"not stated\" a property of the VALUE or of the LINE?** Both precedents exist in-repo, on exactly
40\t  one line each: `Form8283Row.claimed_deduction` is `Option<Usd>` end-to-end (value); `form8995.rs:255`
41\t  gates by map-cell identity (line). They imply different type surgery and different failure modes.
42\t- **What does a total do with a not-stated operand?** Schedule 1 line 9 sums 21 operands of which 20
43\t  have no field at all. Propagate · treat as zero and print · refuse · decide per line.
44\t- **What is the B1 kill-test?** Every fix here is a *suppression*, and a suppression that over-fires is
45\t  invisible — a missing cell looks exactly like the common correct case on a mostly-blank return.
46\t  Neither oracle sees it (both take these values as INPUT). No golden moves (~62 of 64 overstate tax,
47\t  and the goldens encode today's behaviour). The negative test must plant **both** a zero that should
48\t  not print **and** a blank on one of the 24 lines where the form instructs `-0-`. **What single
49\t  instrument reds on both?** If there is no such instrument, say so plainly — by this repo's B1 rule a
50\t  checker that has never been watched go red does not exist, and a program with no gate should not
51\t  start.
52\t
53\t## ★ \"Do not build it\" is a first-class answer
54\t
55\tThe repo's most valuable architect consult to date returned exactly that for §G-14 (shred/tombstone),
56\tand it is recorded as such. If the right answer is *build something much smaller*, *build a different
57\tthing*, or *this is not worth the blast radius given btctax has never had a user*, say so and say why.
58\tI would rather spend this consult learning that than get a well-argued plan for the wrong thing.
59\t
60\t## Settled — do NOT re-derive these
61\t
62\t1. **The counts.** 64 sites / 168 emitted quantities; band 51–64 on two counting calls; the seven
63\t   mechanisms and their tallies; the 24 form-instructed zeros; the layer split. Five readers produced
64\t   these against the form text and I spot-verified the load-bearing ones by hand. Take them as given.
65\t2. **The hard scope boundary**, from §G-11 and non-negotiable: btctax must never build a heuristic that
66\t   flags an omission as suspicious, or any feature opining on whether a blank is lawful. **Both
67\t   directions are software adjudicating intent**, which is out of scope. Do not propose anything that
68\t   crosses it, and flag it if my phasing does.
69\t3. **The four owner decisions** (BRAINSTORM §8) are the owner's and are not yours to settle: whether a
70\t   reconciled ledger is testimony; blank-vs-refuse where silence asserts; whether supplied-then-zeroed
71\t   is in scope; sequencing against Form 6251 Tier 2. **You may say which of them a good architecture
72\t   would make cheap to change later** — that is architecture, and is welcome.
73\t4. Tax correctness is not in question here and there is nothing to re-audit. This is a representability
74\t   problem.
75\t
76\t## Context that should inform the shape
77\t
78\t- **The mature in-repo pattern is `Option<T>` surviving to the writer** — Schedule B 7a/FBAR/8,
79\t  Schedule C I/J, 8283 5a–5c, all built this week, each carrying a comment recording that
80\t  `unwrap_or(false)` there had been a live fabricated-testimony bug. Same defect, one type over.
81\t- **`classifier.rs:17` explicitly permits bare `_` on every `Usd` leaf** — so the census built to make
82\t  answered-ness structural is blind to all 64 by design. Extending it forces human classification on
83\t  ~200 leaves.
84\t- **`CarryProvenance` is the in-repo sibling pattern**: provenance riding alongside a value rather than
85\t  changing its type. Built, advisory-wired, deliberately not printed.
86\t- btctax has **never had a user**; v0.15.0 is prepared but unpublished. Back-compat is not sacred.
87\t
88\t## Output format
89\t
90\t1. `ANSWER:` — the architecture, in under 200 words. If it is \"do not build\", say that here.
91\t2. `WHY THIS SHAPE` — the reasoning, including what you rejected and why.
92\t3. `THE GATE` — your answer to the B1 kill-test question, or an explicit \"there isn't one, and here is
93\t   what that means\".
94\t4. `PHASING` — if you keep a phased build, give the order and say what makes each phase independently
95\t   green. If you reject my four phases, say which and why.
96\t5. `WHAT WOULD MAKE THIS ANSWER WRONG` — the assumption you did not verify.
97\t
98\tBe concrete about types and module boundaries; name files. Read code where it matters — you have a
99\tshell. **READ-ONLY: no edits, no commits.** Do not spawn subagents.
100\t"}]},"uuid":"61a9abf2-3d33-49e2-b4e3-17ea781f5bb2","timestamp":"2026-08-01T04:04:34.813Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/design/no-testimony/BRIEF-architect.md","content":"# §G-11 — ARCHITECT CONSULT BRIEF

You are being asked for **architecture judgment before any code exists.** Nothing is built. No spec is
written. The brainstorm is one commit old and is explicitly not a decision.

## Read these two, in order

1. `/scratch/code/bitcoin_tax/design/no-testimony/BRAINSTORM.md` — the shape, the number, the seven
   mechanisms, my recommended phasing, and the four owner decisions.
2. `/scratch/code/bitcoin_tax/design/no-testimony/MAP-survey.md` — the evidence: 221 money rows
   classified across 16 printed structs + Forms 6251/8959/8960/8995, against the extracted form text.

Then `/scratch/code/bitcoin_tax/CLAUDE.md` — **\"an entry is testimony\"** and **\"blank is the normal
case\"** are the authority. And `FOLLOWUPS.md` §G-11 for the original filing.

## The problem, in four sentences

A blank line and a printed `0` are different speech acts on a return signed under 26 USC §6065: a blank
says nothing, a `0` swears the amount **is** zero. btctax has no representation for \"not stated\" in its
money path, so **64 of the 168 money quantities that reach a PDF can print a zero the filer was never
asked for** — fabricated testimony under someone else's signature. The filed follow-up proposed growing
the emitter's money type; the survey found **62 of the 64 are manufactured upstream**, in
`return_inputs.rs`, `return_1040.rs` and the printed-struct constructors, so that fix would address two.
btctax has exactly three lawful moves per line — **collect** the testimony, **refuse** the return, or
leave **genuinely blank** — and must never silently choose silence and present it as the filer's.

## THE ONE QUESTION

**What is the right architecture here — and is there a shape in which an omission CANNOT COMPILE,
rather than one where 64 sites are individually plumbed and the 65th is forgotten?**

That is the question I most want answered, because the repo's own doctrine says *\"prefer designs in
which an omission does not compile\"* and I do not know whether that is reachable for this defect. My
recommended phasing (BRAINSTORM §7) treats it as phase 3 of 4. **Tell me if that ordering is wrong, or
if the whole framing is.**

Three sub-questions I consider load-bearing. Answer them only insofar as they serve the question above:

- **Is \"not stated\" a property of the VALUE or of the LINE?** Both precedents exist in-repo, on exactly
  one line each: `Form8283Row.claimed_deduction` is `Option<Usd>` end-to-end (value); `form8995.rs:255`
  gates by map-cell identity (line). They imply different type surgery and different failure modes.
- **What does a total do with a not-stated operand?** Schedule 1 line 9 sums 21 operands of which 20
  have no field at all. Propagate · treat as zero and print · refuse · decide per line.
- **What is the B1 kill-test?** Every fix here is a *suppression*, and a suppression that over-fires is
  invisible — a missing cell looks exactly like the common correct case on a mostly-blank return.
  Neither oracle sees it (both take these values as INPUT). No golden moves (~62 of 64 overstate tax,
  and the goldens encode today's behaviour). The negative test must plant **both** a zero that should
  not print **and** a blank on one of the 24 lines where the form instructs `-0-`. **What single
  instrument reds on both?** If there is no such instrument, say so plainly — by this repo's B1 rule a
  checker that has never been watched go red does not exist, and a program with no gate should not
  start.

## ★ \"Do not build it\" is a first-class answer

The repo's most valuable architect consult to date returned exactly that for §G-14 (shred/tombstone),
and it is recorded as such. If the right answer is *build something much smaller*, *build a different
thing*, or *this is not worth the blast radius given btctax has never had a user*, say so and say why.
I would rather spend this consult learning that than get a well-argued plan for the wrong thing.

## Settled — do NOT re-derive these

1. **The counts.** 64 sites / 168 emitted quantities; band 51–64 on two counting calls; the seven
   mechanisms and their tallies; the 24 form-instructed zeros; the layer split. Five readers produced
   these against the form text and I spot-verified the load-bearing ones by hand. Take them as given.
2. **The hard scope boundary**, from §G-11 and non-negotiable: btctax must never build a heuristic that
   flags an omission as suspicious, or any feature opining on whether a blank is lawful. **Both
   directions are software adjudicating intent**, which is out of scope. Do not propose anything that
   crosses it, and flag it if my phasing does.
3. **The four owner decisions** (BRAINSTORM §8) are the owner's and are not yours to settle: whether a
   reconciled ledger is testimony; blank-vs-refuse where silence asserts; whether supplied-then-zeroed
   is in scope; sequencing against Form 6251 Tier 2. **You may say which of them a good architecture
   would make cheap to change later** — that is architecture, and is welcome.
4. Tax correctness is not in question here and there is nothing to re-audit. This is a representability
   problem.

## Context that should inform the shape

- **The mature in-repo pattern is `Option<T>` surviving to the writer** — Schedule B 7a/FBAR/8,
  Schedule C I/J, 8283 5a–5c, all built this week, each carrying a comment recording that
  `unwrap_or(false)` there had been a live fabricated-testimony bug. Same defect, one type over.
- **`classifier.rs:17` explicitly permits bare `_` on every `Usd` leaf** — so the census built to make
  answered-ness structural is blind to all 64 by design. Extending it forces human classification on
  ~200 leaves.
- **`CarryProvenance` is the in-repo sibling pattern**: provenance riding alongside a value rather than
  changing its type. Built, advisory-wired, deliberately not printed.
- btctax has **never had a user**; v0.15.0 is prepared but unpublished. Back-compat is not sacred.

## Output format

1. `ANSWER:` — the architecture, in under 200 words. If it is \"do not build\", say that here.
2. `WHY THIS SHAPE` — the reasoning, including what you rejected and why.
3. `THE GATE` — your answer to the B1 kill-test question, or an explicit \"there isn't one, and here is
   what that means\".
4. `PHASING` — if you keep a phased build, give the order and say what makes each phase independently
   green. If you reject my four phases, say which and why.
5. `WHAT WOULD MAKE THIS ANSWER WRONG` — the assumption you did not verify.

Be concrete about types and module boundaries; name files. Read code where it matters — you have a
shell. **READ-ONLY: no edits, no commits.** Do not spawn subagents.
","numLines":100,"startLine":1,"totalLines":100}},"sourceToolAssistantUUID":"ea6fcb1a-f544-445a-bbd5-ca1a3c8c1cd8","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"61a9abf2-3d33-49e2-b4e3-17ea781f5bb2","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNC3oUvyBDZkeEwTn34","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01PMQGAPo1SBmy6fyQPZbkKX","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/design/no-testimony/BRAINSTORM.md"},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":4394,"cache_read_input_tokens":28201,"cache_creation":{"ephemeral_5m_input_tokens":4394,"ephemeral_1h_input_tokens":0},"output_tokens":55,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNC2GxMSrdEYHEQZCBu","attributionAgent":"general-purpose","type":"assistant","uuid":"6b1dabae-4e95-4c9a-9f11-947e490ae508","timestamp":"2026-08-01T04:04:37.069Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"6b1dabae-4e95-4c9a-9f11-947e490ae508","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01PMQGAPo1SBmy6fyQPZbkKX","type":"tool_result","content":"1\t# §G-11 — BRAINSTORM: the emitter cannot express \"no testimony\"
2\t
3\t**Written 2026-07-31**, from `design/no-testimony/MAP-survey.md` (5 parallel readers over all 16
4\tprinted structs + Forms 6251/8959/8960/8995, classifying 221 rows against the extracted form text).
5\t
6\t**Status:** brainstorm. No spec, no plan, nothing built. Several load-bearing decisions are the
7\towner's and are marked ⬜.
8\t
9\t---
10\t
11\t## 1. ★★★ The survey overturned the follow-up's own sketch
12\t
13\t§G-11 closes with a *\"sketch, not a plan\"*: **\"the emitter's money type grows a 'not stated' state
14\tthat survives to the AcroForm write.\"** That is the natural reading of the defect as filed — the
15\tevidence quoted is `fmt_money(d: Usd) -> String`, which is genuinely the whole money path.
16\t
17\t**It is the wrong primary move, and the survey establishes this by counting.**
18\t
19\t> **62 of the 64 fabrication sites are manufactured BEFORE the emitter.** Growing `fmt_money` and the
20\t> printed types to `Option<Usd>` would fix **two** (Schedule D lines 18/19, both hardcoded
21\t> `let lineN = Usd::ZERO`).
22\t
23\tBy the time bytes reach `btctax-forms`, `printed.rs` has already built `Form1040Lines` out of concrete
24\t`Usd` values. **The information was destroyed upstream.** An emitter that can express blank has nothing
25\tleft to express it *about*.
26\t
27\tAll five readers reached this independently; three volunteered it unprompted as the more important
28\thalf. Where the coercion actually happens:
29\t
30\t| layer | sites |
31\t|---|---|
32\t| `return_inputs.rs` (a `#[serde(default)] Usd` nobody set) | 14 |
33\t| `return_1040.rs` (computation over an absent thing) | 20 |
34\t| `printed.rs` / `other_taxes.rs` / `qbi.rs` (the constructors) | 26 |
35\t| `fold.rs` | 2 |
36\t| **`btctax-forms` (the emitter)** | **2** |
37\t
38\t**Consequence for the work:** this is a *type-and-provenance* problem in `btctax-core`, not a
39\tformatting problem in `btctax-forms`. The emitter change is real but small and comes **last**.
40\t
41\t## 2. The number, and its honest band
42\t
43\t**64 fabrication sites out of 168 money quantities that actually reach a PDF — 38%.**
44\t
45\tDenominator derived, not assumed: 221 raw rows − 11 duplicates (`Form1040Income` is destructured into
46\t`Form1040Lines`) − 42 never emitted (41 Form 6251 lines, `qdcgt_net_capital_gain`) = 168.
47\t
48\tTwo counting calls move it, and both are open questions below: strict upstream-attribution of pure
49\tcarries → **55**; treating a reconciled ledger as the filer's testimony → **−4**. **Defensible band:
50\t51–64.** The band does not change the verdict — this is a program, not a targeted fix — so it does not
51\tneed resolving before a spec.
52\t
53\tWorst-concentrated: 1040 **19/34** · Schedule 3 **5/5** · Schedule D **11/15** · Schedule A **8/18** ·
54\tSchedule 1 **6/10**. Every one of `Form1040Lines`' 34 money fields is written on every export.
55\t
56\t## 3. Seven mechanisms — and the fix addresses these, not the forms
57\t
58\tThe forms are just where the kinds show up. Sorted by count:
59\t
60\t| kind | n | shape |
61\t|---|---|---|
62\t| Σ over an empty `Vec` | 20 | `w2s.iter().map(…).sum()` with no W-2s |
63\t| an `Option` that already means \"absent\", flattened | 15 | the information EXISTS and is thrown away |
64\t| bare `#[serde(default)] Usd` | 14 | a TOML omitting the key parses to `0` |
65\t| derivative total over a structurally-blank column | 9 | the total of nothing is `0` |
66\t| hardcoded `let lineN = Usd::ZERO` | 3 | incl. 1040 L19 CTC, L28 |
67\t| emitter literal | 2 | the only two `fmt_money` would fix |
68\t| closed input surface | 1 | no field exists to answer with |
69\t
70\t★ **The 15 flattened `Option`s are the cheapest and most damning.** btctax already *knows* the value is
71\tabsent and discards that knowledge on the way to the page. Any fix should take these first: they need
72\tno new collection, no new question, no input-surface change.
73\t
74\t## 4. The constraint: 24 lines where a zero is MANDATORY
75\t
76\tThe form is the authority in both directions. 24 instructed zeros were collected verbatim (1040 L15/22,
77\tSchedule A ×4, Schedule SE 9/10, 8959 ×5, 8960 ×2, 8995 ×5, 6251 ×7). On these, `-0-` is what the form
78\t*tells* the filer to write, and suppressing it is the mirror defect.
79\t
80\t★★ **Schedules 1, 2, 3 and C contain no `-0-` instruction at all** — so all 19 fabrication sites on
81\tthose forms sit on lines the form is silent about. That is the cleanest subset in the whole survey.
82\t
83\tFour counter-signals worth carrying into the spec:
84\t- Form 8959's masthead: *\"If any line does not apply to you, leave it blank.\"*
85\t- Form 6251 L20/L34 route on *\"zero **or blank**\"* — so Schedule D 18/19's hardcoded `0` buys nothing.
86\t- 1040 L7's remedy for \"no capital gains\" is a **checkbox**, not a zero. (Already adjudicated: §G-18.)
87\t
88\t## 5. Precedent — extend what works, do not invent
89\t
90\t- **`Option<T>` to the writer, for checkboxes.** Three instances, all built this week: Schedule B
91\t  7a/FBAR/8 (`filter_map`), Schedule C I/J (`if let (Some, Some)`), 8283 5a–5c. **Each carries a comment
92\t  saying `unwrap_or(false)` there was a live fabricated-testimony bug.** This is the mature pattern and
93\t  it is the same defect, one type over.
94\t- **One money leaf is already `Option<Usd>` end-to-end** — `Form8283Row.claimed_deduction`. Existence
95\t  proof that the whole chain can carry it.
96\t- **One line-level gate exists** — `form8995.rs:255`, `if ptr::eq(*cell, &map.line3) && value.is_zero()`.
97\t  ★ Its comment claims *\"line 3 is the only line on this form that is neither derived nor computed\"* —
98\t  **falsified by line 7 in the next loop iteration**, which is the REIT/PTP carryforward IN, equally
99\t  transcribed. Fix that regardless of what else is decided.
100\t- **Row/part suppression** (`schedule_d.rs::active`, `fill8949.rs`) is value-keyed, per-PART not
101\t  per-cell, and **absent from the full-return emitter entirely**.
102\t- **`CarryProvenance` + `advisories.rs:621-633` already computes the exact predicate** a blank-capable
103\t  line needs — and `classifier.rs` marks it \"no print\".
104\t
105\t## 6. ★★ The instrument built for this class is blind to it
106\t
107\t`classifier.rs:17`, verbatim: *\"`_` is **PERMITTED** on other scalar leaves (`String`, `Usd`, `Date`,
108\t`Option<Date>`, `Option<String>`).\"*
109\t
110\tThe answered-ness census — the thing that makes \"we forgot this\" fail to compile for `Option<bool>` —
111\t**exempts every money leaf by design.** So none of the 64 is visible to the one instrument that exists
112\tto catch exactly this. And `coverage.rs` asserts exempt leaves *exist* but never that their printed
113\tlines *render blank* — a B1-shaped gap: a checker that cannot red on the defect it exists to catch.
114\t
115\tThis is probably the highest-leverage single change in the whole program, and it is also the most
116\texpensive: extending the `_`-ban to `Usd` forces a human classification on ~200 leaves and `E0063`s
117\tevery omission. That is the *point* — an omission that does not compile cannot ship — but it is a real
118\tcost and belongs in the spec's phasing, not smuggled in.
119\t
120\t## 7. Recommended shape (my read — not decided)
121\t
122\tFour phases, ordered so each is independently green and the cheapest evidence comes first:
123\t
124\t- **P1 — stop discarding what we know.** The 15 flattened `Option`s. No new questions, no input-surface
125\t  change, no collection. Pure plumbing: keep the `Option` to the printed struct. Smallest possible
126\t  change that produces a real blank on a real page — and therefore the best place to build the B1
127\t  kill-test before the surface is large.
128\t- **P2 — the silent forms.** Schedules 1, 2, 3, C: 19 sites, zero `-0-` instructions to conflict with.
129\t- **P3 — make omission fail to compile.** Extend the classifier's `_`-ban to `Usd`, with the census as
130\t  the permanent gate.
131\t- **P4 — the emitter.** `fmt_money`/`push_money` learn blank; the 2 remaining literals; the falsified
132\t  8995 comment; `-0-` vs `0` rendering.
133\t
134\t**Not in scope, per §G-11's own hard boundary:** any heuristic that flags an omission as suspicious, or
135\tany feature that opines on whether a blank is lawful. Both directions are software adjudicating intent.
136\t
137\t## 8. ⬜ OWNER DECISIONS — these change the work, and I should not make them
138\t
139\t1. **⬜ Does a reconciled ledger constitute the filer's testimony?** Decides Schedule C line 1,
140\t   Schedule D 3/10, 1040 line 7 — and whether the crypto slice's blank or the full return's `0` on 1040
141\t   7a is the bug. This is a judgment about the filer's relationship to their own ledger, not a code
142\t   question.
143\t2. **⬜ Blank, or refuse, where silence ASSERTS?** For class-(A) lines — 1040 25a withholding,
144\t   Schedule SE 8a Medicare wages, both document-matched by the Service against copies it already holds
145\t   — is a blank sufficient, or must the return refuse? Answering \"always blank\" makes the fix mechanical
146\t   and may be wrong.
147\t3. **⬜ Is \"supplied-then-zeroed\" in scope?** Schedule A 8a mixed-use and the §170(b) ceilings print `0`
148\t   over facts the filer *did* state. Not fabrications by this survey's test — but they are affirmative
149\t   sworn zeros contradicting the filer's own testimony, and no leaf-level fix reaches them.
150\t4. **⬜ Sequencing vs Form 6251 Tier 2.** 18 further sites wait there, including CLAUDE.md's canonical
151\t   ISO/line-2i example. Fix-first costs a rebase; Tier-2-first ships the canonical defect on purpose.
152\t
153\t## 9. Open design questions I intend to decide in the SPEC
154\t
155\tValue-vs-line for the not-stated state (both precedents exist, one line each) · what a total does with a
156\tnot-stated operand (Schedule 1 line 9: 20 of 21 operands have no field) · serde-requiredness vs `Option`
157\t(both already in-repo doing this job; requiredness fails loud and breaks every existing TOML) ·
158\t`ReturnInputs` shape change vs a provenance sibling · whether the input-form seam grows
159\t`Money(Option<Usd>)` · `\"0\"` vs `\"-0-\"` · Schedule SE 8d/9/10, the mirror defect.
160\t
161\t## 10. ★ The B1 kill-test is not obvious, and it decides whether any of this is real
162\t
163\t**Every fix here is a *suppression*, and a suppression that over-fires is invisible** — a missing cell
164\tlooks exactly like the common correct case, which is a blank line on a mostly-blank return. Neither
165\toracle sees it (both take values as input). No golden moves (~62 of 64 overstate tax, and the goldens
166\tencode current behaviour).
167\t
168\tSo the negative test must plant **both** defects at once: a zero that should not print, **and** a blank
169\ton one of the 24 instructed zeros. Until there is a single instrument that reds on both, this program
170\thas no gate — and by B1 it does not exist. **This is the first thing the spec must answer, not the
171\tlast.**
172\t"}]},"uuid":"15bfbf71-1474-4f78-811f-3b34e5b02aee","timestamp":"2026-08-01T04:04:37.077Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/design/no-testimony/BRAINSTORM.md","content":"# §G-11 — BRAINSTORM: the emitter cannot express \"no testimony\"

**Written 2026-07-31**, from `design/no-testimony/MAP-survey.md` (5 parallel readers over all 16
printed structs + Forms 6251/8959/8960/8995, classifying 221 rows against the extracted form text).

**Status:** brainstorm. No spec, no plan, nothing built. Several load-bearing decisions are the
owner's and are marked ⬜.

---

## 1. ★★★ The survey overturned the follow-up's own sketch

§G-11 closes with a *\"sketch, not a plan\"*: **\"the emitter's money type grows a 'not stated' state
that survives to the AcroForm write.\"** That is the natural reading of the defect as filed — the
evidence quoted is `fmt_money(d: Usd) -> String`, which is genuinely the whole money path.

**It is the wrong primary move, and the survey establishes this by counting.**

> **62 of the 64 fabrication sites are manufactured BEFORE the emitter.** Growing `fmt_money` and the
> printed types to `Option<Usd>` would fix **two** (Schedule D lines 18/19, both hardcoded
> `let lineN = Usd::ZERO`).

By the time bytes reach `btctax-forms`, `printed.rs` has already built `Form1040Lines` out of concrete
`Usd` values. **The information was destroyed upstream.** An emitter that can express blank has nothing
left to express it *about*.

All five readers reached this independently; three volunteered it unprompted as the more important
half. Where the coercion actually happens:

| layer | sites |
|---|---|
| `return_inputs.rs` (a `#[serde(default)] Usd` nobody set) | 14 |
| `return_1040.rs` (computation over an absent thing) | 20 |
| `printed.rs` / `other_taxes.rs` / `qbi.rs` (the constructors) | 26 |
| `fold.rs` | 2 |
| **`btctax-forms` (the emitter)** | **2** |

**Consequence for the work:** this is a *type-and-provenance* problem in `btctax-core`, not a
formatting problem in `btctax-forms`. The emitter change is real but small and comes **last**.

## 2. The number, and its honest band

**64 fabrication sites out of 168 money quantities that actually reach a PDF — 38%.**

Denominator derived, not assumed: 221 raw rows − 11 duplicates (`Form1040Income` is destructured into
`Form1040Lines`) − 42 never emitted (41 Form 6251 lines, `qdcgt_net_capital_gain`) = 168.

Two counting calls move it, and both are open questions below: strict upstream-attribution of pure
carries → **55**; treating a reconciled ledger as the filer's testimony → **−4**. **Defensible band:
51–64.** The band does not change the verdict — this is a program, not a targeted fix — so it does not
need resolving before a spec.

Worst-concentrated: 1040 **19/34** · Schedule 3 **5/5** · Schedule D **11/15** · Schedule A **8/18** ·
Schedule 1 **6/10**. Every one of `Form1040Lines`' 34 money fields is written on every export.

## 3. Seven mechanisms — and the fix addresses these, not the forms

The forms are just where the kinds show up. Sorted by count:

| kind | n | shape |
|---|---|---|
| Σ over an empty `Vec` | 20 | `w2s.iter().map(…).sum()` with no W-2s |
| an `Option` that already means \"absent\", flattened | 15 | the information EXISTS and is thrown away |
| bare `#[serde(default)] Usd` | 14 | a TOML omitting the key parses to `0` |
| derivative total over a structurally-blank column | 9 | the total of nothing is `0` |
| hardcoded `let lineN = Usd::ZERO` | 3 | incl. 1040 L19 CTC, L28 |
| emitter literal | 2 | the only two `fmt_money` would fix |
| closed input surface | 1 | no field exists to answer with |

★ **The 15 flattened `Option`s are the cheapest and most damning.** btctax already *knows* the value is
absent and discards that knowledge on the way to the page. Any fix should take these first: they need
no new collection, no new question, no input-surface change.

## 4. The constraint: 24 lines where a zero is MANDATORY

The form is the authority in both directions. 24 instructed zeros were collected verbatim (1040 L15/22,
Schedule A ×4, Schedule SE 9/10, 8959 ×5, 8960 ×2, 8995 ×5, 6251 ×7). On these, `-0-` is what the form
*tells* the filer to write, and suppressing it is the mirror defect.

★★ **Schedules 1, 2, 3 and C contain no `-0-` instruction at all** — so all 19 fabrication sites on
those forms sit on lines the form is silent about. That is the cleanest subset in the whole survey.

Four counter-signals worth carrying into the spec:
- Form 8959's masthead: *\"If any line does not apply to you, leave it blank.\"*
- Form 6251 L20/L34 route on *\"zero **or blank**\"* — so Schedule D 18/19's hardcoded `0` buys nothing.
- 1040 L7's remedy for \"no capital gains\" is a **checkbox**, not a zero. (Already adjudicated: §G-18.)

## 5. Precedent — extend what works, do not invent

- **`Option<T>` to the writer, for checkboxes.** Three instances, all built this week: Schedule B
  7a/FBAR/8 (`filter_map`), Schedule C I/J (`if let (Some, Some)`), 8283 5a–5c. **Each carries a comment
  saying `unwrap_or(false)` there was a live fabricated-testimony bug.** This is the mature pattern and
  it is the same defect, one type over.
- **One money leaf is already `Option<Usd>` end-to-end** — `Form8283Row.claimed_deduction`. Existence
  proof that the whole chain can carry it.
- **One line-level gate exists** — `form8995.rs:255`, `if ptr::eq(*cell, &map.line3) && value.is_zero()`.
  ★ Its comment claims *\"line 3 is the only line on this form that is neither derived nor computed\"* —
  **falsified by line 7 in the next loop iteration**, which is the REIT/PTP carryforward IN, equally
  transcribed. Fix that regardless of what else is decided.
- **Row/part suppression** (`schedule_d.rs::active`, `fill8949.rs`) is value-keyed, per-PART not
  per-cell, and **absent from the full-return emitter entirely**.
- **`CarryProvenance` + `advisories.rs:621-633` already computes the exact predicate** a blank-capable
  line needs — and `classifier.rs` marks it \"no print\".

## 6. ★★ The instrument built for this class is blind to it

`classifier.rs:17`, verbatim: *\"`_` is **PERMITTED** on other scalar leaves (`String`, `Usd`, `Date`,
`Option<Date>`, `Option<String>`).\"*

The answered-ness census — the thing that makes \"we forgot this\" fail to compile for `Option<bool>` —
**exempts every money leaf by design.** So none of the 64 is visible to the one instrument that exists
to catch exactly this. And `coverage.rs` asserts exempt leaves *exist* but never that their printed
lines *render blank* — a B1-shaped gap: a checker that cannot red on the defect it exists to catch.

This is probably the highest-leverage single change in the whole program, and it is also the most
expensive: extending the `_`-ban to `Usd` forces a human classification on ~200 leaves and `E0063`s
every omission. That is the *point* — an omission that does not compile cannot ship — but it is a real
cost and belongs in the spec's phasing, not smuggled in.

## 7. Recommended shape (my read — not decided)

Four phases, ordered so each is independently green and the cheapest evidence comes first:

- **P1 — stop discarding what we know.** The 15 flattened `Option`s. No new questions, no input-surface
  change, no collection. Pure plumbing: keep the `Option` to the printed struct. Smallest possible
  change that produces a real blank on a real page — and therefore the best place to build the B1
  kill-test before the surface is large.
- **P2 — the silent forms.** Schedules 1, 2, 3, C: 19 sites, zero `-0-` instructions to conflict with.
- **P3 — make omission fail to compile.** Extend the classifier's `_`-ban to `Usd`, with the census as
  the permanent gate.
- **P4 — the emitter.** `fmt_money`/`push_money` learn blank; the 2 remaining literals; the falsified
  8995 comment; `-0-` vs `0` rendering.

**Not in scope, per §G-11's own hard boundary:** any heuristic that flags an omission as suspicious, or
any feature that opines on whether a blank is lawful. Both directions are software adjudicating intent.

## 8. ⬜ OWNER DECISIONS — these change the work, and I should not make them

1. **⬜ Does a reconciled ledger constitute the filer's testimony?** Decides Schedule C line 1,
   Schedule D 3/10, 1040 line 7 — and whether the crypto slice's blank or the full return's `0` on 1040
   7a is the bug. This is a judgment about the filer's relationship to their own ledger, not a code
   question.
2. **⬜ Blank, or refuse, where silence ASSERTS?** For class-(A) lines — 1040 25a withholding,
   Schedule SE 8a Medicare wages, both document-matched by the Service against copies it already holds
   — is a blank sufficient, or must the return refuse? Answering \"always blank\" makes the fix mechanical
   and may be wrong.
3. **⬜ Is \"supplied-then-zeroed\" in scope?** Schedule A 8a mixed-use and the §170(b) ceilings print `0`
   over facts the filer *did* state. Not fabrications by this survey's test — but they are affirmative
   sworn zeros contradicting the filer's own testimony, and no leaf-level fix reaches them.
4. **⬜ Sequencing vs Form 6251 Tier 2.** 18 further sites wait there, including CLAUDE.md's canonical
   ISO/line-2i example. Fix-first costs a rebase; Tier-2-first ships the canonical defect on purpose.

## 9. Open design questions I intend to decide in the SPEC

Value-vs-line for the not-stated state (both precedents exist, one line each) · what a total does with a
not-stated operand (Schedule 1 line 9: 20 of 21 operands have no field) · serde-requiredness vs `Option`
(both already in-repo doing this job; requiredness fails loud and breaks every existing TOML) ·
`ReturnInputs` shape change vs a provenance sibling · whether the input-form seam grows
`Money(Option<Usd>)` · `\"0\"` vs `\"-0-\"` · Schedule SE 8d/9/10, the mirror defect.

## 10. ★ The B1 kill-test is not obvious, and it decides whether any of this is real

**Every fix here is a *suppression*, and a suppression that over-fires is invisible** — a missing cell
looks exactly like the common correct case, which is a blank line on a mostly-blank return. Neither
oracle sees it (both take values as input). No golden moves (~62 of 64 overstate tax, and the goldens
encode current behaviour).

So the negative test must plant **both** defects at once: a zero that should not print, **and** a blank
on one of the 24 instructed zeros. Until there is a single instrument that reds on both, this program
has no gate — and by B1 it does not exist. **This is the first thing the spec must answer, not the
last.**
","numLines":172,"startLine":1,"totalLines":172}},"sourceToolAssistantUUID":"6b1dabae-4e95-4c9a-9f11-947e490ae508","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"15bfbf71-1474-4f78-811f-3b34e5b02aee","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNC3oUvyBDZkeEwTn34","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01Bhq2YMwcsVxNdmhe1LBsiV","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/design/no-testimony/MAP-survey.md"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":4394,"cache_read_input_tokens":28201,"output_tokens":157,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":4394},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":157,"cache_read_input_tokens":28201,"cache_creation_input_tokens":4394,"cache_creation":{"ephemeral_5m_input_tokens":4394,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbNC2GxMSrdEYHEQZCBu","attributionAgent":"general-purpose","type":"assistant","uuid":"17008b02-383f-4243-82d5-2f21d2cd723c","timestamp":"2026-08-01T04:04:37.361Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"17008b02-383f-4243-82d5-2f21d2cd723c","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01Bhq2YMwcsVxNdmhe1LBsiV","type":"tool_result","content":"1\t# §G-11 survey MAP — synthesis of five readers over every money field on every printed form
2\t
3\tWorking material for the brainstorm. Built from five independent surveys against `main` @ `18eb2ed`.
4\tVerbatim code quotes below re-verified against current source at write time.
5\t
6\t---
7\t
8\t## 1. THE NUMBER
9\t
10\t**64 fabrication sites out of 168 money quantities that reach a PDF today — 38%.**
11\t
12\tThis is a **program, not a targeted fix.** Every printed form in the packet has at least one; the
13\t1040 itself has 19 of 34.
14\t
15\t### Denominator, stated honestly
16\t
17\t| | count | note |
18\t|---|---|---|
19\t| raw rows surveyed | 221 | the aggregate the five readers returned |
20\t| − duplicate quantities | −11 | `Form1040Income`'s 11 money fields are destructured verbatim into `Form1040Lines` (printed.rs:634-647); surveyed once |
21\t| − never emitted | −42 | 41 Form 6251 lines + `Form1040Income::qdcgt_net_capital_gain` reach **no AcroForm cell** |
22\t| **= emitted surface** | **168** | money quantities that actually print today |
23\t| **fabrication sites** | **64** | all 64 are on the emitted surface; 0 on Form 6251 |
24\t
25\t### Per-form breakdown
26\t
27\t| form | emitted money fields | fabrication sites | already-suppressed | form instructs `-0-` |
28\t|---|---|---|---|---|
29\t| Form 1040 (`Form1040Lines`) | 34 | **19** | 0 | 2 |
30\t| Schedule 1 | 10 | **6** | 0 | 0 |
31\t| Schedule 2 | 4 | **1** | 0 | 0 |
32\t| Schedule 3 | 5 | **5** | 0 | 0 |
33\t| Schedule A | 18 | **8** | 0 | 1 |
34\t| Schedule B | 4 | **3** | 1 (row-level) | 0 |
35\t| Schedule C | 7 | **2** | 0 | 0 |
36\t| Schedule D | 15 | **11** | 3 (routing) | 0 |
37\t| Schedule SE | 12 | **1** | 3 (routing) | 2 |
38\t| Form 8949 | 8 | **1** | 2 (cell + part) | 0 |
39\t| Form 8283 | 3 | **1** | 1 (`Option<Usd>`) | 0 |
40\t| Form 8275 | 1 | 0 | 0 | 0 |
41\t| Form 8959 | 17 | **2** | 0 | 5 |
42\t| Form 8960 | 14 | **2** | 0 | 2 |
43\t| Form 8995 | 16 | **2** | 1 (line-level) | 5 |
44\t| *Form 6251* | *0 emitted (41 modelled)* | *0 — **18 in waiting*** | — | 7 |
45\t| **TOTAL** | **168** | **64** | **11** | **24** |
46\t
47\t### ★ Two counting ambiguities that move the number — the brainstorm must settle them
48\t
49\tThe readers did **not** apply one rule consistently, and both disagreements are substantive, not
50\tclerical:
51\t
52\t1. **Pure carries.** Reader 3 counted Schedule B line 4 (`let line4 = line2;`) as its own site
53\t   because it lands on 1040 line 2b. Reader 1 counted 1040 line 1z (`let line1z = line1a;`) as NOT
54\t   a site, pushing the defect upstream per brief rule 1. **Same shape, opposite verdicts.** Applying
55\t   \"defect moves upstream\" strictly to all 9 derivative totals → **55**.
56\t2. **Ledger silence.** Reader 4 marked Schedule C line 1 and Schedule D 3(d)/(e)/(h) as sites
57\t   because btctax has no channel for non-crypto receipts or non-crypto dispositions, and self-flagged
58\t   that a reasonable person could call the reconciled ledger the filer's own affirmative statement —
59\t   *\"in which case those four lines drop out and the count falls from 14 to 10.\"*
60\t
61\t**Defensible band: 51–64.** Even the floor is a program.
62\t
63\t### The one structural asymmetry worth stating up front
64\t
65\tReader 1: **every one of the 34 `Form1040Lines` money fields is written on every full-return export**,
66\tunconditionally, and `line34` is written **twice** (into L34 and L35a, form1040_full.rs:170-171). There
67\tis no path on which a 1040 money cell is blank. 19 of those 34 can be fabricated.
68\t
69\t---
70\t
71\t## 2. THE KINDS
72\t
73\tGrouped by **mechanism**. This is what a fix addresses; the forms are just where it surfaces.
74\t
75\t| # | kind | count | layer |
76\t|---|---|---|---|
77\t| 1 | Σ over a possibly-empty `Vec` | **20** | `return_1040.rs` |
78\t| 2 | bare `#[serde(default)] Usd` scalar (incl. struct `Default`) | **14** | `return_inputs.rs` |
79\t| 3 | an `Option` that carries \"absent\" flattened to `Usd::ZERO` | **15** | `printed.rs` / `fold.rs` |
80\t| 4 | hardcoded `let lineN = Usd::ZERO;` — the line has no field at all | **3** | `printed.rs` / `other_taxes.rs` |
81\t| 5 | emitter literal `Usd::ZERO` — no struct field | **2** | `btctax-forms` |
82\t| 6 | derivative total over a structurally-blank column | **9** | `printed.rs` |
83\t| 7 | closed input surface — ledger/absent-channel silence printed as a figure | **1** | `return_1040.rs` |
84\t| | **total** | **64** | |
85\t
86\t### Kind 1 — Σ over a possibly-empty `Vec` (20 sites)
87\t
88\t`Iterator::sum` over an empty iterator **is** `Usd::ZERO`. There is no `Option` to lose, so nothing
89\twarns. The Vecs are all `#[serde(default)]`; several element boxes are too.
90\t
91\t```rust
92\t// return_1040.rs:494-496
93\tlet w2_ss_wages: Usd = ri.w2s.iter().filter(|w| w.owner == se_owner)
94\t    .map(|w| w.box3_ss_wages + w.box7_ss_tips).sum();
95\t```
96\t
97\tSites: 1040 1a, 2a, 2b, 3a, 3b, 25a, 25b · Sch 1 L7, L18 · Sch 3 L1 · Sch A 11, 12, 13 ·
98\tSch B 2, 6 · Sch D 13 · Sch SE 8a · 8959 L1, L19 · 8995 L6.
99\tSource Vecs: `w2s`, `int_1099`, `div_1099`, `g_1099`, `charitable`, `charitable_carryover_in`.
100\t
101\t★ **The repo already names this exact hazard, in its own words, for a different field** —
102\t`return_inputs.rs:551-557`: *\"`CharitableCarryItem` already carries a per-item provenance, which is
103\tuseless for the case that matters: an **EMPTY vec has no items**, so an empty list carries no
104\tprovenance at all — and an empty list is exactly the state that is ambiguous between \"no carryover\"
105\tand \"never asked\".\"*
106\t
107\t### Kind 2 — bare `#[serde(default)] Usd` scalar (14 sites)
108\t
109\tA missing TOML key and a stated zero are the same bytes.
110\t
111\t```rust
112\t// return_inputs.rs:260-261
113\t#[serde(default)]
114\tpub expenses: Usd,
115\t```
116\t
117\tSites: 1040 25c, 26 · Sch 1 L1, L21 · Sch 3 L10 · Sch A 1, 5a, 5b, 5c, 8a · Sch C 28 ·
118\tSch D 6, 14 (via `Carryforward::default()`) · 8995 L7.
119\t
120\t★ **The asymmetry is exact:** `parse_return_inputs_toml` (cmd/tax.rs:113-131) goes to real lengths to
121\treject **UNKNOWN** keys via `serde_ignored` — and defaults **MISSING** ones in silence. A typo is
122\tloud; an omission is invisible.
123\t
124\t★★ **And serde-requiredness is already used as an answered-ness mechanism, on exactly three boxes:**
125\t`W2::box1_wages`, `Form1099Int::box1_interest`, `Form1099Div::box1a_ordinary` carry **no**
126\t`#[serde(default)]` — a TOML omitting them refuses to parse. The mechanism exists; it was applied to
127\tthree boxes and not the other ~30.
128\t
129\t### Kind 3 — an `Option` flattened to zero (15 sites)
130\t
131\t**The most concentrated and the cheapest to fix**: in every one of these the `None` *already means
132\texactly* \"this does not file / there is none\", and it is thrown away one line before it would matter.
133\t
134\t```rust
135\tprinted.rs:591   let line8  = sch_1.map_or(Usd::ZERO, |s| s.line10);
136\tprinted.rs:593   let line10 = sch_1.map_or(Usd::ZERO, |s| s.line26);
137\tprinted.rs:654   let line13 = f8995.map_or(Usd::ZERO, |q| q.line15);
138\tprinted.rs:677   let line20 = sch_3.map_or(Usd::ZERO, |s| s.line8);
139\tprinted.rs:680   let line23 = sch_2.map_or(Usd::ZERO, |s| s.line21);
140\tprinted.rs:690   let line31 = sch_3.map_or(Usd::ZERO, |s| s.line15);
141\tprinted.rs:866   let st = f8949.map(|f| f.st_totals).unwrap_or_default();
142\tprinted.rs:867   let lt = f8949.map(|f| f.lt_totals).unwrap_or_default();
143\t```
144\t
145\tPlus `Sch 2 L12` (`f8960.map_or`), and the **ledger fold**, which is a layer lower again:
146\t
147\t```rust
148\t// project/fold.rs:1151
149\tlet usd_basis = basis.unwrap_or(Usd::ZERO);   // conservative $0 default (max eventual gain)
150\t```
151\t→ Form 8949 column (e) and Form 8283 column (g). The defaulted-vs-supplied signal survives only in a
152\t*separate* Advisory blocker (`SelfTransferInboundZeroBasis`, state.rs:100) which **never gates**, and
153\t`event.rs:26-29` says so outright: *\"the defaulted-vs-supplied signal rides the ... advisory, not this
154\tsource.\"*
155\t
156\t★ On the 8949 pair, `printed.rs:73-74` states the intended output and the emitter does the opposite:
157\t*\"`None` when the year has no disposals — a carryover/distribution-only Schedule D files with lines
158\t3/10 blank and NO 8949 attached.\"* It prints `0` and attaches no 8949.
159\t
160\t### Kind 4 — hardcoded `Usd::ZERO` in core, no field anywhere (3 sites)
161\t
162\t```rust
163\tprinted.rs:674     let line17 = Usd::ZERO;  // Schedule 2 Part I is blank in v1
164\tprinted.rs:676     let line19 = Usd::ZERO;  // ★ CTC/ODC — a §3.4 conservative omission (advisory fires)
165\tother_taxes.rs:312 let line9d = Usd::ZERO;  // v1 models no investment expenses
166\t```
167\t
168\t### Kind 5 — emitter literal, no struct field (2 sites)
169\t
170\tThe **only two of the 64 that originate inside `btctax-forms`.** Schedule D lines 18/19, written as
171\t`push_p2(..., Usd::ZERO, ...)` on the `BothGains` branch (schedule_d_full.rs:201-212), pinned by a KAT
172\tasserting `Some(\"0\")`.
173\t
174\t### Kind 6 — a derivative total over a structurally-blank column (9 sites)
175\t
176\tNot arithmetic over printed lines: **arithmetic over lines that do not exist as fields.**
177\t
178\t```rust
179\tprinted.rs:402   let line9  = line8v;   // 8a-8u and 8w-8z are blank
180\tprinted.rs:691   let line32 = line31;   // 27-30 blank (EIC omitted conservatively; rest unrepresentable)
181\tprinted.rs:1322  let line8  = line1;    // lines 2-4, 5a, 5b, 7 are all conservatively omitted (blank)
182\t```
183\t
184\tSites: 1040 7, 32 · Sch 1 L9, L10, L26 · Sch 3 L8, L15 · Sch B 4 · 8960 L11.
185\t
186\t★★ **This is the sharpest self-contradiction in the whole survey.** `Schedule3Lines`' own doc
187\t(printed.rs:1296) says of the omitted credits *\"They are left BLANK, never a misleading 0\"*, and the
188\tmap header repeats it. **The individual boxes honour it exactly. Then `let line8 = line1;` prints `0`
189\ton the TOTAL of those same blanks.** Schedule 1 line 9 is the extreme case: **20 of 21 operands are
190\tstructurally absent.**
191\t
192\t### Kind 7 — closed input surface (1 site, contested)
193\t
194\tSchedule C line 1: gross receipts from the ledger, on a business whose non-crypto receipts btctax has
195\tno channel to ask about. See §1 ambiguity 2.
196\t
197\t---
198\t
199\t## 3. WHERE THE DEFECT ACTUALLY LIVES
200\t
201\t## ★★★ 62 of 64 are manufactured BEFORE the emitter. Changing `fmt_money` and the printed types would fix TWO.
202\t
203\tAll five readers converged on this independently, unprompted, and three of them volunteered it as the
204\tmore important half:
205\t
206\t- Reader 3: *\"**Not one of the 11 fabrication sites originates in `btctax-forms`.** ... Fixing
207\t  `fmt_money` alone would give the emitter a vocabulary with nothing left to say.\"*
208\t- Reader 4: *\"11 of the 14 are `Transcribed` — the emitter is faithfully printing a `Usd::ZERO` that
209\t  was manufactured in `return_1040.rs` from an absent input.\"*
210\t- Reader 5: *\"a `Option<Usd>` money type in btctax-forms alone would fix nothing: the emitter would
211\t  faithfully receive a `Some(0)`.\"*
212\t- Reader 2: *\"The emitter is a faithful transcriber; every one of the 12 sites is decided before
213\t  `printed.rs` is reached.\"*
214\t- Reader 1: *\"Ten of the nineteen ... in six of them an `Option` that already carries exactly the right
215\t  information is destroyed at the 1040 boundary.\"*
216\t
217\t### Coercion by layer
218\t
219\t| layer | file | sites | what happens |
220\t|---|---|---|---|
221\t| input schema | `return_inputs.rs` | 14 | `#[serde(default)] Usd` — missing key ⇒ `ZERO` |
222\t| ledger fold | `project/fold.rs:1151` | 2 | `basis.unwrap_or(Usd::ZERO)` |
223\t| absolute return | `return_1040.rs` | 20 | Σ over empty `Vec` |
224\t| printed structs | `printed.rs`, `other_taxes.rs`, `qbi.rs` | 26 | `map_or(ZERO)`, hardcoded consts, blank-column totals |
225\t| **emitter** | **`btctax-forms`** | **2** | **emitter literals (Sch D 18/19)** |
226\t
227\t### ★ The emitter is not the constraint — but it is not innocent either
228\t
229\t`push_money` (cells.rs:44-69) is unconditional on every path:
230\t
231\t```rust
232\tMoneyCell::Single(fqn) => { w.push((fqn.clone(), FieldValue::Text(fmt_money(value)))); }
233\t```
234\t```rust
235\t// lib.rs:78  (FOLLOWUPS §G-11 cites lib.rs:77 — off by one on current main)
236\tpub(crate) fn fmt_money(d: Usd) -> String { d.to_string() }
237\t```
238\t
239\tSo the emitter cannot *express* a blank even where core knows one is right — and core, in at least
240\tfour places, demonstrably knows (printed.rs:73-74, printed.rs:1296, return_inputs.rs:551-557,
241\t`Advisory::QbiCarryforwardNotStated` / `BenefitCarryoversNotStated` firing on the very returns that
242\tprint the zero). **Both ends need the vocabulary. Only the upstream end needs new information.**
243\t
244\t### ★★ Three more upstream facts that bound any fix
245\t
246\t1. **The input-form seam cannot express it either.** `seam.rs:154-163` — `FieldValue::TriState(Option<bool>)`
247\t   and `Date(Option<Date>)` exist *precisely* so \"never asked\" survives to storage (the P9
248\t   answered-ness work). **`Money(Usd)` has no such state.** Even on the interactive path a pre-filled
249\t   `0` the filer scrolled past and a typed `0` are the same value.
250\t2. **The answered-ness classifier is structurally blind to money.** `classifier.rs` r2 M-6 forbids a
251\t   bare `_` on an `Option<bool>` leaf — *\"the whole point of the census is to distinguish 'this encodes
252\t   no decision' from 'we forgot it'\"* — and **explicitly permits `_` on every `Usd` leaf**. So the one
253\t   instrument built to catch this class cannot see any of the 64. `classify_payments` records
254\t   **nothing at all**, not even an `exempt(..)` with a reason.
255\t3. **`PrintedInputs` already carries the right rule, one layer too high.** return_1040.rs:1077-1080:
256\t   *\"**No `Default`** — deliberately ... a silently-zeroed field here is a wrong number on a filed
257\t   return.\"* The struct cannot be defaulted — and the Σ expressions that *fill* it manufacture the
258\t   zeros anyway.
259\t4. **Six leaves have no interactive path at all.** `coverage.rs:300-342`: `EXEMPT_PREFIXES` =
260\t   `[\"int_1099\", \"div_1099\", \"g_1099\", ...]`, `EXEMPT_LEAVES` includes `sch1.state_refund_taxable`,
261\t   `sch1.student_loan_interest_paid`. The census is **one-directional**: it asserts every non-exempt
262\t   leaf IS covered, and **nothing asserts that an exempt leaf's printed line renders blank rather than
263\t   `0`.** B1-shaped gap — a checker never watched red on this defect.
264\t
265\t---
266\t
267\t## 4. WHAT THE FORMS SAY — the constraint a fix must not break
268\t
269\t**24 lines where the form instructs the zero.** These are *correct* and must keep printing.
270\tVerbatim, grouped:
271\t
272\t### Form 1040 (2)
273\t- **L15** — *\"Subtract line 14 from line 11. If zero or less, enter -0-. This is your taxable income\"*
274\t- **L22** — *\"Subtract line 21 from line 18. If zero or less, enter -0-\"*
275\t
276\t### Schedule A (1 — the only `-0-` on the whole form)
277\t- **L4** — *\"Subtract line 3 from line 1. If line 3 is more than line 1, enter -0-\"*
278\t
279\t### Schedule SE (2)
280\t- **L9 (and L10 by the same sentence)** — *\"Subtract line 8d from line 7. If zero or less, enter -0- here and on line 10 and go to line 11\"*
281\t
282\t### Form 8959 (5)
283\t- **L6** — *\"Subtract line 5 from line 4. If zero or less, enter -0-\"*
284\t- **L8** — *\"Self-employment income from Schedule SE (Form 1040), Part I, line 6. If you had a loss, enter -0-\"*
285\t- **L11** — *\"Subtract line 10 from line 9. If zero or less, enter -0-\"*
286\t- **L12** — *\"Subtract line 11 from line 8. If zero or less, enter -0-\"*
287\t- **L22** — *\"Subtract line 21 from line 19. If zero or less, enter -0-. This is your Additional Medicare Tax withholding on Medicare wages\"*
288\t
289\t### Form 8960 (2)
290\t- **L12** — *\"Net investment income. Subtract Part II, line 11, from Part I, line 8. ... If zero or less, enter -0-\"*
291\t- **L15** — *\"Subtract line 14 from line 13. If zero or less, enter -0-\"*
292\t
293\t### Form 8995 (5)
294\t- **L4** — *\"Total qualified business income. Combine lines 2 and 3. If zero or less, enter -0-\"*
295\t- **L8** — *\"Total qualified REIT dividends and PTP income. Combine lines 6 and 7. If zero or less, enter -0-\"*
296\t- **L13** — *\"Subtract line 12 from line 11. If zero or less, enter -0-\"*
297\t- **L16** — *\"Total qualified business (loss) carryforward. Combine lines 2 and 3. If greater than zero, enter -0-\"*
298\t- **L17** — *\"Total qualified REIT dividends and PTP (loss) carryforward. Combine lines 6 and 7. If greater than zero, enter -0-\"*
299\t
300\t### Form 6251 (7 — not emitted today, but the constraint lands with Tier 2)
301\tL6, L10, L11, L20, L21, L27, L29. Sharpest: **L6** — *\"Subtract line 5 from line 4. If more than zero,
302\tgo to line 7. **If zero or less, enter -0- here and on lines 7, 9, and 11**, and go to line 10\"* — an
303\tinstructed zero that *propagates by name to three other lines*.
304\t
305\t### ★ Schedules 1, 2 and 3 contain NO `-0-` instruction at all
306\t`grep -n -- '-0-'` over all three extracted texts returns nothing (exit 1). Schedule C likewise: none.
307\t**Every one of the 19 fabrication sites on those four forms is on a line the form is silent about.**
308\t
309\t### ★★ Four constraints pointing the OTHER way — forms that instruct a BLANK
310\t
311\t1. **Form 8959 masthead**, verbatim: *\"**If any line does not apply to you, leave it blank.** See
312\t   separate instructions.\"* A form-level blank instruction, over a form that today prints `0` on
313\t   L1/4/6/7/19/20/21/22/24 for a pure-SE filer with no W-2.
314\t2. **Form 6251 L34 routing**: *\"If line 14 is zero **or blank**, skip lines 35 through 37...\"* — the
315\t   IRS itself treating zero and blank as two distinct states of one cell.
316\t3. **Form 6251 L20 routing**: *\"Are lines 18 and 19 both zero **or blank** and you are not filing Form
317\t   4952?\"* — so Schedule D's hardcoded `0` on lines 18/19 **buys nothing**; L20 routes to Yes either way.
318\t4. **Form 1040 L7 line text**: *\"Capital gain or (loss). Attach Schedule D if required. **If not
319\t   required, check here**\"* — the form's own remedy for the no-Schedule-D case is a **checkbox, not a
320\t   zero**.
321\t
322\t### ★ A rendering question the survey surfaced twice
323\tWhere the form says `-0-`, **btctax writes the bare string `\"0\"`** (`fmt_money` = `Decimal::to_string`).
324\tThe only `push_literal(..., \"-0-\", ...)` for a money cell in the repo is the crypto-slice 1040 line 7a.
325\t
326\t---
327\t
328\t## 5. ALREADY-SOLVED PRECEDENT
329\t
330\tFive working patterns exist in-repo. **A fix should extend one, not invent one.**
331\t
332\t### (a) ★★ Line-level blank-vs-zero — exists exactly ONCE, and its own comment is falsified
333\t
334\t`form8995.rs:245-256`, the single true instance on the whole emitted surface:
335\t
336\t```rust
337\t// ★★ LINE 3 IS WRITTEN ONLY WHEN THERE IS A CARRYFORWARD. It is `Qualified business net (loss)
338\t// carryforward FROM THE PRIOR YEAR` — a fact the filer supplies about a return btctax did not
339\t// compute. A printed `0` there is an affirmative sworn statement that they had no prior-year
340\t// QBI loss; a BLANK is no statement at all ...
341\t//
342\t// Every other line here is DERIVED from figures already on the page, so a zero is btctax's own
343\t// arithmetic and prints legitimately. Line 3 is the only line on this form that is neither
344\t// derived nor computed — it is testimony — which is exactly why it is the only one gated.
345\tif std::ptr::eq(*cell, &map.line3) && value.is_zero() {
346\t    continue;
347\t}
348\t```
349\t
350\t★ **\"Line 3 is the only line on this form\" is false on the very next iteration of the same loop.**
351\tLine 7 — *\"Qualified REIT dividends and qualified PTP (loss) carryforward from the prior year\"* — is
352\tline 3's exact structural twin, a prior-year figure from a return btctax did not compute, and it is
353\tungated. Note the mechanism, too: `std::ptr::eq` against one map cell — **an identity comparison, not
354\ta property of the value.**
355\t
356\t### (b) ★★★ `Option<T>` already reaches the writer — for CHECKBOX answers, three times
357\t
358\tThis is the mature pattern, and it is **structural rather than conventional** by design:
359\t
360\t```rust
361\t// schedule_b.rs — Part III (7a, the FBAR sub-question, 8)
362\t// ★ EVERY Part III answer is written iff the filer ACTUALLY gave it. `None` means the question was
363\t//   never answered (or, for the FBAR sub-question, never even asked because 7a was \"No\"), and a
364\t//   checked box would then be testimony nobody gave. This `filter_map` is the mechanism that makes
365\t//   \"not answered ⇒ blank\" structural; `unwrap_or(false)` here was a live fabricated-testimony bug.
366\t.filter_map(|(pair, answer)| answer.map(|a| (pair, a)))
367\t```
368\t
369\t```rust
370\t// schedule_c.rs:107-118 — lines I and J
371\t// The `if let Some(..)` is the whole guarantee. `None` (never asked) and `Some(false)` (asked,
372\t// answered no) are DIFFERENT marks on the page — an unwritten pair versus a checked No box — and
373\t// an `unwrap_or(false)` here would print a \"No\" the filer never gave, on a form they sign under
374\t// §6065. ... it is structural here rather than conventional, which is what makes it hold.
375\tif let (Some(pair), Some(answer)) = (pair, answer) { ... }
376\t```
377\t
378\t```rust
379\t// form8283.rs:139-141 — lines 5a/5b/5c
380\t// ★ §G-21 — the filer's answer to lines 5a/5b/5c. `Some(false)` ⇒ all three print No; `None` ⇒ all
381\t// three stay BLANK. The crypto slice always passes `None`: it writes no Section B declarations.
382\tno_restrictions: Option<bool>,
383\t```
384\t
385\t**The type reaches the writer, and the writer's default action is to write nothing.** That is exactly
386\tthe shape the money path lacks.
387\t
388\t### (c) ★ ONE money leaf is already `Option<Usd>` end-to-end
389\t
390\t`Form8283Row.claimed_deduction` — `form8283.rs:436`:
391\t```rust
392\tif let Some(ded) = row.claimed_deduction { push_money(...) }
393\t```
394\tNon-carrier leg rows leave column (i) genuinely blank rather than printing a `0` that would
395\tdouble-count. **The only proof in the repo that a blank money cell is achievable today.**
396\t
397\t### (d) Row- and part-level suppression (ad hoc, value-derived, NOT answered-ness)
398\t
399\t```rust
400\t// schedule_d.rs:34-37  — a PART-level gate on the crypto slice only
401\t/// A part is \"active\" (worth a Schedule D line) iff it has any proceeds/cost/gain.
402\tfn active(p: &btctax_core::ScheduleDPart) -> bool {
403\t    !p.proceeds.is_zero() || !p.cost_basis.is_zero() || !p.gain.is_zero()
404\t}
405\t```
406\t```rust
407\t// fill8949.rs:44-48  — ONE cell, column (g), on the crypto slice
408\tif r.adjustment_amount.is_zero() { String::new() } else { r.adjustment_amount.to_string() },
409\t```
410\tPlus `fill8949.rs:84` `if data.rows.is_empty() { return; }` (whole part), `fill8949.rs:101`
411\t`if value.is_empty() { continue; // blank cell — do not write, do not authorize }` (the generic
412\tempty-string skip the (g) blank rides), and `printed.rs:1012` `.filter(|r| r.amount > Usd::ZERO)` —
413\tSchedule B payer rows, suppressed **in the constructor**, with the reason stated: *\"A payer with
414\tnothing to report is not listed: a zero row would name someone on a federal form for no reason.\"*
415\t
416\t★ **Critical limits.** `active()` is **per-PART, never per-CELL** (an active part with a $0 cost basis
417\tstill writes `\"0\"` in column (e)); it exists **only** in `schedule_d.rs`, and the full-return emitter
418\t`schedule_d_full.rs` has **no equivalent**; the two emitters are mutually exclusive per year, so on a
419\tfull return **it is not in play at all**. Every one of these keys off the **value**, not off whether
420\tanyone was asked.
421\t
422\t### (e) Form-level and routing-level blanks
423\t
424\t`schedule_N_lines() -> None` ⇒ `packet.rs:77-96` skips the whole PDF. `ScheduleDRouting`
425\t(schedule_d_full.rs:191-259) leaves L21 untouched on 3 of 4 branches. `let skip_8d_to_10 =
426\tlines.line9 == Usd::ZERO;` (schedule_se_full.rs:78). **All three answer \"does the form's own routing
427\treach this line\", never \"does this line have testimony behind it.\"** Schedule 2 line 21 is the only
428\tline in the entire survey structurally incapable of printing a zero — because its gate is on its own
429\tvalue and a zero total is expressed as an *absent form*.
430\t
431\t### (f) The provenance-sibling pattern — built, wired to advisories, deliberately NOT printed
432\t
433\t`CarryProvenance` siblings exist for two carryovers:
434\t`capital_loss_carryforward_in_provenance` (return_inputs.rs:516-527) — *\"without it a zero is
435\tuninterpretable — 'the filer has no carryover' and 'nobody ever asked' are the same bytes\"* — and
436\t`charitable_carryover_in_provenance` (return_inputs.rs:551-557).
437\t
438\t★★ `advisories.rs:621-633` computes **exactly the predicate a blank-capable line would need**:
439\t```rust
440\tlet user = crate::tax::return_inputs::CarryProvenance::User;
441\tlet cl = ri.capital_loss_carryforward_in.short == Usd::ZERO
442\t    && ri.capital_loss_carryforward_in.long == Usd::ZERO
443\t    && ri.capital_loss_carryforward_in_provenance == user;
444\t```
445\tand fires `Advisory::BenefitCarryoversNotStated`. **So on the very return where btctax tells the filer
446\t\"your carryovers were not stated\", it simultaneously prints `0` on Schedule D lines 6 and 14 asserting
447\tthey are zero.** `classifier.rs:131-133` classes the bit *\"no print, no tax direction\"* — the one bit
448\tthat distinguishes the two blanks is deliberately kept off the page.
449\t
450\t### (g) The `Option<Usd>` + `Option<bool>`-gate design, already written down and argued
451\t
452\t`return_inputs.rs:620-636`, the §911/931/933 add-backs:
453\t> *\"`Option<Usd>`, NOT defaulted `Usd`, and the distinction is the whole point. A plain `Usd`
454\t> defaulting to 0 cannot tell \"zero because the filer has none\" from \"zero because nobody asked\"\"* —
455\t> with the gate carried by an `Option<bool>` declaration because *\"`Option<bool>` is a leaf the
456\t> classifier **forbids** `_` on, so it cannot be added without a human classifying it — whereas
457\t> `Option<Usd>` is a scalar the `_` rule permits, which would make this convention again.\"*
458\t
459\t**That paragraph is a ready-made design, and it names the reason a bare `Option<Usd>` is not
460\tsufficient on its own.**
461\t
462\t---
463\t
464\t## 6. THE HARD CASES
465\t
466\tWhere \"blank or zero\" is genuinely ambiguous, or turns on a fact btctax does not have.
467\t
468\t### H1 — What does a TOTAL render as when an operand is not stated?
469\t**Unavoidable: 9 of the 64 are totals.** Schedule 1 line 9 is the extreme — *\"Add lines 8a through
470\t8z\"* with **20 of 21 operands structurally absent** (no fields exist). Schedule 3 line 8 sums five
471\tcredits the product deliberately never asks about and whose advisory exists *because they may well be
472\tnonzero*. Making operands `Option<Usd>` does not by itself decide what `Some(5000) + None + None`
473\tprints. All three lawful moves (collect / refuse / blank) are defensible **per line**, and inheriting
474\tone rule for all nine is probably wrong.
475\t
476\t### H2 — Is a reconciled LEDGER the filer's testimony?
477\tSchedule C line 1, Schedule D 3(d)/(e)/(h), 1040 line 7, Schedule 1 line 8v. Reader 2 said yes for 8v
478\t(*\"btctax refuses to produce a return for an unreconciled year\"*); Reader 4 said no for Sch C L1
479\t(no channel for non-crypto receipts) and self-flagged the reversal. Moves the count by ~4.
480\t
481\t### H3 — The SAME CELL, two emitters, opposite dispositions — and the rule is already written
482\t`form1040.rs:8-10` (crypto slice), verbatim: *\"Schedule D INACTIVE ... → **7a BLANK**. Stamping \"-0-\"
483\tagainst a blank Schedule D line 16 would be an unearned zero-capital-gains claim.\"* The full-return
484\temitter writes the same cell via `push_money` with no test (form1040_full.rs:115). One of the two is
485\twrong, and the survey cannot say which without deciding H2. **B3-shaped: the fix already exists in the
486\ttree and nobody carried it across.**
487\t
488\t### H4 — Supplied-then-zeroed: a computed zero over a fact the filer DID state
489\tAn `Option<Usd>` at the leaves misses these entirely:
490\t- **Sch A 8a** — `let mortgage_8a = if mortgage_mixed_use_box { Usd::ZERO } else { a.mortgage_interest_1098 };`
491\t  (return_1040.rs:410-414). The filer reports $12,000 of Form-1098 interest and holds the 1098; the
492\t  cell swears `0`. Only a checked box and an advisory qualify it — neither is on the amount line.
493\t- **Sch A 11/12** — `current_allowed = current_total.min(ceiling)`, `ceiling = 0.60 * agi.max(ZERO)`
494\t  (charitable.rs:63,117-119,141-147). A $10,000 gift on a ≤0-AGI return prints `0` and routes to
495\t  `carryover_out`.
496\tIs this in scope for §G-11, or a separate class?
497\t
498\t### H5 — The mirror defect: an INSTRUCTED zero that btctax suppresses
499\tSchedule SE. The form gives two instructions pointing opposite ways: line 8a's *\"If $168,600 or more,
500\tskip lines 8b through 10, and go to line 11\"* versus line 9's *\"If zero or less, enter -0- here and on
501\tline 10 and go to line 11\"*. `skip_8d_to_10` follows the first and blanks all three. Needs
502\tadjudication against `i1040sse`. **§G-11's fix could make this worse if it only ever adds blanks.**
503\t
504\t### H6 — Elections and conditional lines are a different class than amounts
505\t- **1040 L35a** — *\"Amount of line 34 you want refunded to you\"* is an **election** (refund now vs
506\t  apply to 2025 on line 36). btctax never asks, writes `line34` into the cell, leaves L36 blank.
507\t- **1040 L34/L37** — the form makes L34 conditional in terms (*\"**If line 33 is more than line 24**\"*)
508\t  and prints `0` when the condition fails; both print `0` when payments exactly equal tax.
509\tThe brief's test (\"was the fact supplied?\") does not cleanly apply to an unasked *choice*.
510\t
511\t### H7 — Where the `None` genuinely means \"computed, and there is none\"
512\t`Sch 2 L4` (`sch_se.map_or(ZERO)` — the §6017 $400 floor was not met) and `Sch 2 L11`. Reader 2 called
513\tthese NOT fabrications; Reader 1 called the structurally identical `1040 L23`
514\t(`sch_2.map_or(ZERO)`) a fabrication because the total cites a schedule that is not attached.
515\t**The readers split on the same idiom.** Is \"a schedule that legitimately does not file\" a stated zero
516\tor an absent one? Note the form text on Sch 2 L4 is *\"Self-employment tax. **Attach Schedule SE**\"* —
517\tand no Schedule SE is attached.
518\t
519\t### H8 — Closed surfaces with no channel at all
520\tNot defaulted — **absent**. Form 8960 lines 9a/9b/9c/10 (and every filer reaching 8960 is over the
521\t§1411(b) threshold, i.e. exactly the population with allocable state income tax under 9b). Schedule 1
522\tlines 2a/2b, 4, 5, 6 and 8a–8z. Schedule 2 Part II (Sch H, Form 5329, 4137/8919). Schedule 3 line 9
523\t(net PTC) and 1040 line 17's excess-APTC half. 1099-R withholding (1040 25b — *i1040gi's own primary
524\tcase for that line*). Direct sales of collectibles / §1250 realty (Sch D 18/19). **Blank the line,
525\trefuse the return, or advisory?** All three are defensible and the choice differs per line.
526\t
527\t### H9 — 1040 line 19, the CTC/ODC: a hardcoded zero on a return that names the child
528\t`fullreturn_inputs.toml` carries `[[header.dependents]] name = \"Sam Doe\" / relationship = \"Son\" /
529\tdate_of_birth = [2012, 106]` — a 12-year-old CTC-qualifying child, printed into the dependents table
530\twith the CTC/ODC boxes **deliberately unchecked** — and `let line19 = Usd::ZERO;`. The struct's own
531\tdoc **concedes the discrimination and declines it**: *\"Conservative omissions print as absent, not
532\tzero, where the form allows — but line 19 (the CTC/ODC) is a computed credit line the form expects, so
533\tit prints `0`.\"* Is \"the form expects a computed credit line\" a sufficient reason?
534\t
535\t### H10 — Direction is not uniform, and one site runs the *other* way
536\tMost fabrications overstate tax (harmless to the fisc, costly to the filer, **invisible to both
537\toracles and every reconciliation**). Two classes do not:
538\t- **Schedule D line 13** (capital gain distributions) **understates** tax — omits real long-term income.
539\t- **1040 25a/25b/26** (withholding, estimated payments) deflate line 33 and inflate line 37 *\"amount
540\t  you owe\"* — the exact shape the repo was burned by once and wrote down: *\"★ Dropping this line told a
541\t  filer who had ALREADY paid with their extension to pay it again\"* (printed.rs:1306-1309).
542\t§G-11 forbids opining on lawfulness — but does direction legitimately order the work?
543\t
544\t### H11 — Form 6251: 18 sites in waiting, including the standing example
545\tLines 2c–2t have **no fields**, the form reaches no PDF, and `line4 = line1 + line2a + line2b + line3`
546\ttreats all 18 as $0. The module doc already schedules the moment it becomes a defect: *\"Tier 2, which
547\tfiles the form rather than only computing it, must give 2c–2t real fields.\"* If those land as `Usd`
548\tunder today's unconditional `push_money`, **CLAUDE.md's standing example — an ISO exercise printed as
549\t$0 on line 2i — ships on that commit.** Sequencing decision, not a code decision.
550\t
551\t---
552\t
553\t## 7. OPEN QUESTIONS FOR THE BRAINSTORM
554\t
555\t1. **Is the not-stated state a property of the VALUE or of the LINE?** `Option<Usd>` threaded through
556\t   `ReturnInputs` → `AbsoluteReturn` → printed structs → `push_money` makes it a value. A per-line
557\t   decision table (each line records which of the three lawful moves it takes, and why) makes it a
558\t   line. The `form8995.rs` precedent is a *line* gate (`std::ptr::eq` on a map cell); the `8283
559\t   claimed_deduction` precedent is a *value* gate. Both work today, on one line each.
560\t2. **What does a total do with a not-stated operand?** Propagate not-stated · treat as zero and print ·
561\t   refuse · decide per line. Schedule 1 line 9 (20 of 21 operands absent) and Schedule 3 line 8
562\t   (\"never a misleading 0\" on the parts, `0` on the sum) are the cases that force it.
563\t3. **Does a reconciled ledger constitute the filer's testimony?** Determines Schedule C line 1,
564\t   Schedule D 3/10, 1040 line 7 — and therefore whether the number is 64 or ~60, and whether the
565\t   crypto slice's blank or the full return's `0` on 1040 7a is the bug (H3).
566\t4. **Is a pure carry (`let lineN = lineM;`) its own fabrication site or an upstream one?** Readers
567\t   split on identical shapes (Sch B 4 = yes, 1040 1z = no). Moves the number by up to 9 and decides
568\t   whether the fix must touch the totals at all.
569\t5. **Serde-requiredness or `Option`?** Both mechanisms are already in the repo doing this job:
570\t   `box1_wages`/`box1_interest`/`box1a_ordinary` carry no `#[serde(default)]` and a TOML omitting them
571\t   *refuses to parse*; the §911 add-backs use `Option<Usd>` + an `Option<bool>` gate. Requiredness
572\t   fails loud at parse and breaks every existing TOML; `Option` fails silent and needs a classifier
573\t   rule to stay honest.
574\t6. **Does `ReturnInputs` change shape, or does provenance ride alongside?** `CarryProvenance` is the
575\t   in-repo sibling pattern (built, advisory-wired, deliberately not printed). Siblings are
576\t   non-breaking and already proven; a shape change is the thing the compiler can enforce.
577\t7. **Blank, or refuse?** §G-11 gives exactly three lawful moves. For a line where silence *asserts*
578\t   (class A) — 1040 25a withholding, Schedule SE 8a Medicare wages, both document-matched by the
579\t   Service against copies it already holds — is blank sufficient, or must the return refuse? Answering
580\t   \"always blank\" makes the fix mechanical and may be wrong.
581\t8. **Should the classifier's `_`-forbidden rule extend to `Usd` leaves?** It is the instrument built
582\t   for exactly this and it currently exempts every money field by design (*\"`_` is PERMITTED on other
583\t   scalar leaves (`String`, `Usd`, `Date`, …)\"*). Extending it makes the census the permanent gate and
584\t   `E0063`s every omission — at the cost of forcing a human classification on ~200 leaves. B1 applies:
585\t   *which test reds when this checker is removed?*
586\t9. **Does the input-form seam grow `Money(Option<Usd>)`?** Without it, even the interactive path
587\t   cannot express not-stated (a pre-filled `0` scrolled past == a typed `0`), and Schedule 3 line 10 —
588\t   the one fabrication site *with* a real interactive field — stays broken. With it, every money field
589\t   in the TUI needs a rendering decision for the empty state.
590\t10. **Do we print `\"0\"` or `\"-0-\"` where the form instructs a zero?** `fmt_money` emits the bare
591\t    string today; the only `-0-` literal in the repo is the crypto-slice 1040 line 7a. Cosmetic, or
592\t    part of the same speech-act argument?
593\t11. **Is \"supplied-then-zeroed\" (H4) in scope?** Schedule A 8a mixed-use and the §170(b) charitable
594\t    ceilings print `0` over facts the filer *did* state. Not fabrications under the brief's test — but
595\t    they are affirmative sworn zeros contradicting the filer's own testimony, and no leaf-level fix
596\t    touches them.
597\t12. **Are unasked ELECTIONS a separate program?** 1040 line 35a (refund vs apply-to-2025) and the
598\t    L34/L37 conditional pair are unasked *choices*, not unasked *amounts*. Fold in, or file separately?
599\t13. **Does Schedule SE 8d/9/10 get fixed in the same breath (H5)?** It is the mirror defect — an
600\t    instructed `-0-` that btctax suppresses. A fix that only ever adds blanks entrenches it, and
601\t    adjudicating it needs `i1040sse`, not the form text.
602\t14. **Does Form 6251 Tier 2 land before or after this fix?** 18 sites in waiting including the
603\t    standing ISO/2i example. Fix-first costs a rebase; Tier-2-first ships CLAUDE.md's canonical defect
604\t    on purpose.
605\t15. **Does direction order the work?** ~62 of 64 overstate tax; Schedule D 13 and the 1040
606\t    withholding/payments cluster understate it or inflate \"amount you owe\". §G-11 forbids opining on
607\t    lawfulness — is sequencing by direction a legitimate use of that fact, or the same judgment
608\t    smuggled in?
609\t16. **What is the B1 kill-test?** Every fix here is a *suppression*, and a suppression that fires when
610\t    it should not is invisible — a missing cell looks like the common correct case. The negative test
611\t    has to plant a defect that *prints a zero it should not* AND one that *blanks a line it must not*
612\t    (the 24 instructed zeros). What single instrument reds on both?
613\t"}]},"uuid":"3823db67-536d-43fe-9cd8-f5eb843f4c3b","timestamp":"2026-08-01T04:04:37.553Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/design/no-testimony/MAP-survey.md","content":"# §G-11 survey MAP — synthesis of five readers over every money field on every printed form

Working material for the brainstorm. Built from five independent surveys against `main` @ `18eb2ed`.
Verbatim code quotes below re-verified against current source at write time.

---

## 1. THE NUMBER

**64 fabrication sites out of 168 money quantities that reach a PDF today — 38%.**

This is a **program, not a targeted fix.** Every printed form in the packet has at least one; the
1040 itself has 19 of 34.

### Denominator, stated honestly

| | count | note |
|---|---|---|
| raw rows surveyed | 221 | the aggregate the five readers returned |
| − duplicate quantities | −11 | `Form1040Income`'s 11 money fields are destructured verbatim into `Form1040Lines` (printed.rs:634-647); surveyed once |
| − never emitted | −42 | 41 Form 6251 lines + `Form1040Income::qdcgt_net_capital_gain` reach **no AcroForm cell** |
| **= emitted surface** | **168** | money quantities that actually print today |
| **fabrication sites** | **64** | all 64 are on the emitted surface; 0 on Form 6251 |

### Per-form breakdown

| form | emitted money fields | fabrication sites | already-suppressed | form instructs `-0-` |
|---|---|---|---|---|
| Form 1040 (`Form1040Lines`) | 34 | **19** | 0 | 2 |
| Schedule 1 | 10 | **6** | 0 | 0 |
| Schedule 2 | 4 | **1** | 0 | 0 |
| Schedule 3 | 5 | **5** | 0 | 0 |
| Schedule A | 18 | **8** | 0 | 1 |
| Schedule B | 4 | **3** | 1 (row-level) | 0 |
| Schedule C | 7 | **2** | 0 | 0 |
| Schedule D | 15 | **11** | 3 (routing) | 0 |
| Schedule SE | 12 | **1** | 3 (routing) | 2 |
| Form 8949 | 8 | **1** | 2 (cell + part) | 0 |
| Form 8283 | 3 | **1** | 1 (`Option<Usd>`) | 0 |
| Form 8275 | 1 | 0 | 0 | 0 |
| Form 8959 | 17 | **2** | 0 | 5 |
| Form 8960 | 14 | **2** | 0 | 2 |
| Form 8995 | 16 | **2** | 1 (line-level) | 5 |
| *Form 6251* | *0 emitted (41 modelled)* | *0 — **18 in waiting*** | — | 7 |
| **TOTAL** | **168** | **64** | **11** | **24** |

### ★ Two counting ambiguities that move the number — the brainstorm must settle them

The readers did **not** apply one rule consistently, and both disagreements are substantive, not
clerical:

1. **Pure carries.** Reader 3 counted Schedule B line 4 (`let line4 = line2;`) as its own site
   because it lands on 1040 line 2b. Reader 1 counted 1040 line 1z (`let line1z = line1a;`) as NOT
   a site, pushing the defect upstream per brief rule 1. **Same shape, opposite verdicts.** Applying
   \"defect moves upstream\" strictly to all 9 derivative totals → **55**.
2. **Ledger silence.** Reader 4 marked Schedule C line 1 and Schedule D 3(d)/(e)/(h) as sites
   because btctax has no channel for non-crypto receipts or non-crypto dispositions, and self-flagged
   that a reasonable person could call the reconciled ledger the filer's own affirmative statement —
   *\"in which case those four lines drop out and the count falls from 14 to 10.\"*

**Defensible band: 51–64.** Even the floor is a program.

### The one structural asymmetry worth stating up front

Reader 1: **every one of the 34 `Form1040Lines` money fields is written on every full-return export**,
unconditionally, and `line34` is written **twice** (into L34 and L35a, form1040_full.rs:170-171). There
is no path on which a 1040 money cell is blank. 19 of those 34 can be fabricated.

---

## 2. THE KINDS

Grouped by **mechanism**. This is what a fix addresses; the forms are just where it surfaces.

| # | kind | count | layer |
|---|---|---|---|
| 1 | Σ over a possibly-empty `Vec` | **20** | `return_1040.rs` |
| 2 | bare `#[serde(default)] Usd` scalar (incl. struct `Default`) | **14** | `return_inputs.rs` |
| 3 | an `Option` that carries \"absent\" flattened to `Usd::ZERO` | **15** | `printed.rs` / `fold.rs` |
| 4 | hardcoded `let lineN = Usd::ZERO;` — the line has no field at all | **3** | `printed.rs` / `other_taxes.rs` |
| 5 | emitter literal `Usd::ZERO` — no struct field | **2** | `btctax-forms` |
| 6 | derivative total over a structurally-blank column | **9** | `printed.rs` |
| 7 | closed input surface — ledger/absent-channel silence printed as a figure | **1** | `return_1040.rs` |
| | **total** | **64** | |

### Kind 1 — Σ over a possibly-empty `Vec` (20 sites)

`Iterator::sum` over an empty iterator **is** `Usd::ZERO`. There is no `Option` to lose, so nothing
warns. The Vecs are all `#[serde(default)]`; several element boxes are too.

```rust
// return_1040.rs:494-496
let w2_ss_wages: Usd = ri.w2s.iter().filter(|w| w.owner == se_owner)
    .map(|w| w.box3_ss_wages + w.box7_ss_tips).sum();
```

Sites: 1040 1a, 2a, 2b, 3a, 3b, 25a, 25b · Sch 1 L7, L18 · Sch 3 L1 · Sch A 11, 12, 13 ·
Sch B 2, 6 · Sch D 13 · Sch SE 8a · 8959 L1, L19 · 8995 L6.
Source Vecs: `w2s`, `int_1099`, `div_1099`, `g_1099`, `charitable`, `charitable_carryover_in`.

★ **The repo already names this exact hazard, in its own words, for a different field** —
`return_inputs.rs:551-557`: *\"`CharitableCarryItem` already carries a per-item provenance, which is
useless for the case that matters: an **EMPTY vec has no items**, so an empty list carries no
provenance at all — and an empty list is exactly the state that is ambiguous between \"no carryover\"
and \"never asked\".\"*

### Kind 2 — bare `#[serde(default)] Usd` scalar (14 sites)

A missing TOML key and a stated zero are the same bytes.

```rust
// return_inputs.rs:260-261
#[serde(default)]
pub expenses: Usd,
```

Sites: 1040 25c, 26 · Sch 1 L1, L21 · Sch 3 L10 · Sch A 1, 5a, 5b, 5c, 8a · Sch C 28 ·
Sch D 6, 14 (via `Carryforward::default()`) · 8995 L7.

★ **The asymmetry is exact:** `parse_return_inputs_toml` (cmd/tax.rs:113-131) goes to real lengths to
reject **UNKNOWN** keys via `serde_ignored` — and defaults **MISSING** ones in silence. A typo is
loud; an omission is invisible.

★★ **And serde-requiredness is already used as an answered-ness mechanism, on exactly three boxes:**
`W2::box1_wages`, `Form1099Int::box1_interest`, `Form1099Div::box1a_ordinary` carry **no**
`#[serde(default)]` — a TOML omitting them refuses to parse. The mechanism exists; it was applied to
three boxes and not the other ~30.

### Kind 3 — an `Option` flattened to zero (15 sites)

**The most concentrated and the cheapest to fix**: in every one of these the `None` *already means
exactly* \"this does not file / there is none\", and it is thrown away one line before it would matter.

```rust
printed.rs:591   let line8  = sch_1.map_or(Usd::ZERO, |s| s.line10);
printed.rs:593   let line10 = sch_1.map_or(Usd::ZERO, |s| s.line26);
printed.rs:654   let line13 = f8995.map_or(Usd::ZERO, |q| q.line15);
printed.rs:677   let line20 = sch_3.map_or(Usd::ZERO, |s| s.line8);
printed.rs:680   let line23 = sch_2.map_or(Usd::ZERO, |s| s.line21);
printed.rs:690   let line31 = sch_3.map_or(Usd::ZERO, |s| s.line15);
printed.rs:866   let st = f8949.map(|f| f.st_totals).unwrap_or_default();
printed.rs:867   let lt = f8949.map(|f| f.lt_totals).unwrap_or_default();
```

Plus `Sch 2 L12` (`f8960.map_or`), and the **ledger fold**, which is a layer lower again:

```rust
// project/fold.rs:1151
let usd_basis = basis.unwrap_or(Usd::ZERO);   // conservative $0 default (max eventual gain)
```
→ Form 8949 column (e) and Form 8283 column (g). The defaulted-vs-supplied signal survives only in a
*separate* Advisory blocker (`SelfTransferInboundZeroBasis`, state.rs:100) which **never gates**, and
`event.rs:26-29` says so outright: *\"the defaulted-vs-supplied signal rides the ... advisory, not this
source.\"*

★ On the 8949 pair, `printed.rs:73-74` states the intended output and the emitter does the opposite:
*\"`None` when the year has no disposals — a carryover/distribution-only Schedule D files with lines
3/10 blank and NO 8949 attached.\"* It prints `0` and attaches no 8949.

### Kind 4 — hardcoded `Usd::ZERO` in core, no field anywhere (3 sites)

```rust
printed.rs:674     let line17 = Usd::ZERO;  // Schedule 2 Part I is blank in v1
printed.rs:676     let line19 = Usd::ZERO;  // ★ CTC/ODC — a §3.4 conservative omission (advisory fires)
other_taxes.rs:312 let line9d = Usd::ZERO;  // v1 models no investment expenses
```

### Kind 5 — emitter literal, no struct field (2 sites)

The **only two of the 64 that originate inside `btctax-forms`.** Schedule D lines 18/19, written as
`push_p2(..., Usd::ZERO, ...)` on the `BothGains` branch (schedule_d_full.rs:201-212), pinned by a KAT
asserting `Some(\"0\")`.

### Kind 6 — a derivative total over a structurally-blank column (9 sites)

Not arithmetic over printed lines: **arithmetic over lines that do not exist as fields.**

```rust
printed.rs:402   let line9  = line8v;   // 8a-8u and 8w-8z are blank
printed.rs:691   let line32 = line31;   // 27-30 blank (EIC omitted conservatively; rest unrepresentable)
printed.rs:1322  let line8  = line1;    // lines 2-4, 5a, 5b, 7 are all conservatively omitted (blank)
```

Sites: 1040 7, 32 · Sch 1 L9, L10, L26 · Sch 3 L8, L15 · Sch B 4 · 8960 L11.

★★ **This is the sharpest self-contradiction in the whole survey.** `Schedule3Lines`' own doc
(printed.rs:1296) says of the omitted credits *\"They are left BLANK, never a misleading 0\"*, and the
map header repeats it. **The individual boxes honour it exactly. Then `let line8 = line1;` prints `0`
on the TOTAL of those same blanks.** Schedule 1 line 9 is the extreme case: **20 of 21 operands are
structurally absent.**

### Kind 7 — closed input surface (1 site, contested)

Schedule C line 1: gross receipts from the ledger, on a business whose non-crypto receipts btctax has
no channel to ask about. See §1 ambiguity 2.

---

## 3. WHERE THE DEFECT ACTUALLY LIVES

## ★★★ 62 of 64 are manufactured BEFORE the emitter. Changing `fmt_money` and the printed types would fix TWO.

All five readers converged on this independently, unprompted, and three of them volunteered it as the
more important half:

- Reader 3: *\"**Not one of the 11 fabrication sites originates in `btctax-forms`.** ... Fixing
  `fmt_money` alone would give the emitter a vocabulary with nothing left to say.\"*
- Reader 4: *\"11 of the 14 are `Transcribed` — the emitter is faithfully printing a `Usd::ZERO` that
  was manufactured in `return_1040.rs` from an absent input.\"*
- Reader 5: *\"a `Option<Usd>` money type in btctax-forms alone would fix nothing: the emitter would
  faithfully receive a `Some(0)`.\"*
- Reader 2: *\"The emitter is a faithful transcriber; every one of the 12 sites is decided before
  `printed.rs` is reached.\"*
- Reader 1: *\"Ten of the nineteen ... in six of them an `Option` that already carries exactly the right
  information is destroyed at the 1040 boundary.\"*

### Coercion by layer

| layer | file | sites | what happens |
|---|---|---|---|
| input schema | `return_inputs.rs` | 14 | `#[serde(default)] Usd` — missing key ⇒ `ZERO` |
| ledger fold | `project/fold.rs:1151` | 2 | `basis.unwrap_or(Usd::ZERO)` |
| absolute return | `return_1040.rs` | 20 | Σ over empty `Vec` |
| printed structs | `printed.rs`, `other_taxes.rs`, `qbi.rs` | 26 | `map_or(ZERO)`, hardcoded consts, blank-column totals |
| **emitter** | **`btctax-forms`** | **2** | **emitter literals (Sch D 18/19)** |

### ★ The emitter is not the constraint — but it is not innocent either

`push_money` (cells.rs:44-69) is unconditional on every path:

```rust
MoneyCell::Single(fqn) => { w.push((fqn.clone(), FieldValue::Text(fmt_money(value)))); }
```
```rust
// lib.rs:78  (FOLLOWUPS §G-11 cites lib.rs:77 — off by one on current main)
pub(crate) fn fmt_money(d: Usd) -> String { d.to_string() }
```

So the emitter cannot *express* a blank even where core knows one is right — and core, in at least
four places, demonstrably knows (printed.rs:73-74, printed.rs:1296, return_inputs.rs:551-557,
`Advisory::QbiCarryforwardNotStated` / `BenefitCarryoversNotStated` firing on the very returns that
print the zero). **Both ends need the vocabulary. Only the upstream end needs new information.**

### ★★ Three more upstream facts that bound any fix

1. **The input-form seam cannot express it either.** `seam.rs:154-163` — `FieldValue::TriState(Option<bool>)`
   and `Date(Option<Date>)` exist *precisely* so \"never asked\" survives to storage (the P9
   answered-ness work). **`Money(Usd)` has no such state.** Even on the interactive path a pre-filled
   `0` the filer scrolled past and a typed `0` are the same value.
2. **The answered-ness classifier is structurally blind to money.** `classifier.rs` r2 M-6 forbids a
   bare `_` on an `Option<bool>` leaf — *\"the whole point of the census is to distinguish 'this encodes
   no decision' from 'we forgot it'\"* — and **explicitly permits `_` on every `Usd` leaf**. So the one
   instrument built to catch this class cannot see any of the 64. `classify_payments` records
   **nothing at all**, not even an `exempt(..)` with a reason.
3. **`PrintedInputs` already carries the right rule, one layer too high.** return_1040.rs:1077-1080:
   *\"**No `Default`** — deliberately ... a silently-zeroed field here is a wrong number on a filed
   return.\"* The struct cannot be defaulted — and the Σ expressions that *fill* it manufacture the
   zeros anyway.
4. **Six leaves have no interactive path at all.** `coverage.rs:300-342`: `EXEMPT_PREFIXES` =
   `[\"int_1099\", \"div_1099\", \"g_1099\", ...]`, `EXEMPT_LEAVES` includes `sch1.state_refund_taxable`,
   `sch1.student_loan_interest_paid`. The census is **one-directional**: it asserts every non-exempt
   leaf IS covered, and **nothing asserts that an exempt leaf's printed line renders blank rather than
   `0`.** B1-shaped gap — a checker never watched red on this defect.

---

## 4. WHAT THE FORMS SAY — the constraint a fix must not break

**24 lines where the form instructs the zero.** These are *correct* and must keep printing.
Verbatim, grouped:

### Form 1040 (2)
- **L15** — *\"Subtract line 14 from line 11. If zero or less, enter -0-. This is your taxable income\"*
- **L22** — *\"Subtract line 21 from line 18. If zero or less, enter -0-\"*

### Schedule A (1 — the only `-0-` on the whole form)
- **L4** — *\"Subtract line 3 from line 1. If line 3 is more than line 1, enter -0-\"*

### Schedule SE (2)
- **L9 (and L10 by the same sentence)** — *\"Subtract line 8d from line 7. If zero or less, enter -0- here and on line 10 and go to line 11\"*

### Form 8959 (5)
- **L6** — *\"Subtract line 5 from line 4. If zero or less, enter -0-\"*
- **L8** — *\"Self-employment income from Schedule SE (Form 1040), Part I, line 6. If you had a loss, enter -0-\"*
- **L11** — *\"Subtract line 10 from line 9. If zero or less, enter -0-\"*
- **L12** — *\"Subtract line 11 from line 8. If zero or less, enter -0-\"*
- **L22** — *\"Subtract line 21 from line 19. If zero or less, enter -0-. This is your Additional Medicare Tax withholding on Medicare wages\"*

### Form 8960 (2)
- **L12** — *\"Net investment income. Subtract Part II, line 11, from Part I, line 8. ... If zero or less, enter -0-\"*
- **L15** — *\"Subtract line 14 from line 13. If zero or less, enter -0-\"*

### Form 8995 (5)
- **L4** — *\"Total qualified business income. Combine lines 2 and 3. If zero or less, enter -0-\"*
- **L8** — *\"Total qualified REIT dividends and PTP income. Combine lines 6 and 7. If zero or less, enter -0-\"*
- **L13** — *\"Subtract line 12 from line 11. If zero or less, enter -0-\"*
- **L16** — *\"Total qualified business (loss) carryforward. Combine lines 2 and 3. If greater than zero, enter -0-\"*
- **L17** — *\"Total qualified REIT dividends and PTP (loss) carryforward. Combine lines 6 and 7. If greater than zero, enter -0-\"*

### Form 6251 (7 — not emitted today, but the constraint lands with Tier 2)
L6, L10, L11, L20, L21, L27, L29. Sharpest: **L6** — *\"Subtract line 5 from line 4. If more than zero,
go to line 7. **If zero or less, enter -0- here and on lines 7, 9, and 11**, and go to line 10\"* — an
instructed zero that *propagates by name to three other lines*.

### ★ Schedules 1, 2 and 3 contain NO `-0-` instruction at all
`grep -n -- '-0-'` over all three extracted texts returns nothing (exit 1). Schedule C likewise: none.
**Every one of the 19 fabrication sites on those four forms is on a line the form is silent about.**

### ★★ Four constraints pointing the OTHER way — forms that instruct a BLANK

1. **Form 8959 masthead**, verbatim: *\"**If any line does not apply to you, leave it blank.** See
   separate instructions.\"* A form-level blank instruction, over a form that today prints `0` on
   L1/4/6/7/19/20/21/22/24 for a pure-SE filer with no W-2.
2. **Form 6251 L34 routing**: *\"If line 14 is zero **or blank**, skip lines 35 through 37...\"* — the
   IRS itself treating zero and blank as two distinct states of one cell.
3. **Form 6251 L20 routing**: *\"Are lines 18 and 19 both zero **or blank** and you are not filing Form
   4952?\"* — so Schedule D's hardcoded `0` on lines 18/19 **buys nothing**; L20 routes to Yes either way.
4. **Form 1040 L7 line text**: *\"Capital gain or (loss). Attach Schedule D if required. **If not
   required, check here**\"* — the form's own remedy for the no-Schedule-D case is a **checkbox, not a
   zero**.

### ★ A rendering question the survey surfaced twice
Where the form says `-0-`, **btctax writes the bare string `\"0\"`** (`fmt_money` = `Decimal::to_string`).
The only `push_literal(..., \"-0-\", ...)` for a money cell in the repo is the crypto-slice 1040 line 7a.

---

## 5. ALREADY-SOLVED PRECEDENT

Five working patterns exist in-repo. **A fix should extend one, not invent one.**

### (a) ★★ Line-level blank-vs-zero — exists exactly ONCE, and its own comment is falsified

`form8995.rs:245-256`, the single true instance on the whole emitted surface:

```rust
// ★★ LINE 3 IS WRITTEN ONLY WHEN THERE IS A CARRYFORWARD. It is `Qualified business net (loss)
// carryforward FROM THE PRIOR YEAR` — a fact the filer supplies about a return btctax did not
// compute. A printed `0` there is an affirmative sworn statement that they had no prior-year
// QBI loss; a BLANK is no statement at all ...
//
// Every other line here is DERIVED from figures already on the page, so a zero is btctax's own
// arithmetic and prints legitimately. Line 3 is the only line on this form that is neither
// derived nor computed — it is testimony — which is exactly why it is the only one gated.
if std::ptr::eq(*cell, &map.line3) && value.is_zero() {
    continue;
}
```

★ **\"Line 3 is the only line on this form\" is false on the very next iteration of the same loop.**
Line 7 — *\"Qualified REIT dividends and qualified PTP (loss) carryforward from the prior year\"* — is
line 3's exact structural twin, a prior-year figure from a return btctax did not compute, and it is
ungated. Note the mechanism, too: `std::ptr::eq` against one map cell — **an identity comparison, not
a property of the value.**

### (b) ★★★ `Option<T>` already reaches the writer — for CHECKBOX answers, three times

This is the mature pattern, and it is **structural rather than conventional** by design:

```rust
// schedule_b.rs — Part III (7a, the FBAR sub-question, 8)
// ★ EVERY Part III answer is written iff the filer ACTUALLY gave it. `None` means the question was
//   never answered (or, for the FBAR sub-question, never even asked because 7a was \"No\"), and a
//   checked box would then be testimony nobody gave. This `filter_map` is the mechanism that makes
//   \"not answered ⇒ blank\" structural; `unwrap_or(false)` here was a live fabricated-testimony bug.
.filter_map(|(pair, answer)| answer.map(|a| (pair, a)))
```

```rust
// schedule_c.rs:107-118 — lines I and J
// The `if let Some(..)` is the whole guarantee. `None` (never asked) and `Some(false)` (asked,
// answered no) are DIFFERENT marks on the page — an unwritten pair versus a checked No box — and
// an `unwrap_or(false)` here would print a \"No\" the filer never gave, on a form they sign under
// §6065. ... it is structural here rather than conventional, which is what makes it hold.
if let (Some(pair), Some(answer)) = (pair, answer) { ... }
```

```rust
// form8283.rs:139-141 — lines 5a/5b/5c
// ★ §G-21 — the filer's answer to lines 5a/5b/5c. `Some(false)` ⇒ all three print No; `None` ⇒ all
// three stay BLANK. The crypto slice always passes `None`: it writes no Section B declarations.
no_restrictions: Option<bool>,
```

**The type reaches the writer, and the writer's default action is to write nothing.** That is exactly
the shape the money path lacks.

### (c) ★ ONE money leaf is already `Option<Usd>` end-to-end

`Form8283Row.claimed_deduction` — `form8283.rs:436`:
```rust
if let Some(ded) = row.claimed_deduction { push_money(...) }
```
Non-carrier leg rows leave column (i) genuinely blank rather than printing a `0` that would
double-count. **The only proof in the repo that a blank money cell is achievable today.**

### (d) Row- and part-level suppression (ad hoc, value-derived, NOT answered-ness)

```rust
// schedule_d.rs:34-37  — a PART-level gate on the crypto slice only
/// A part is \"active\" (worth a Schedule D line) iff it has any proceeds/cost/gain.
fn active(p: &btctax_core::ScheduleDPart) -> bool {
    !p.proceeds.is_zero() || !p.cost_basis.is_zero() || !p.gain.is_zero()
}
```
```rust
// fill8949.rs:44-48  — ONE cell, column (g), on the crypto slice
if r.adjustment_amount.is_zero() { String::new() } else { r.adjustment_amount.to_string() },
```
Plus `fill8949.rs:84` `if data.rows.is_empty() { return; }` (whole part), `fill8949.rs:101`
`if value.is_empty() { continue; // blank cell — do not write, do not authorize }` (the generic
empty-string skip the (g) blank rides), and `printed.rs:1012` `.filter(|r| r.amount > Usd::ZERO)` —
Schedule B payer rows, suppressed **in the constructor**, with the reason stated: *\"A payer with
nothing to report is not listed: a zero row would name someone on a federal form for no reason.\"*

★ **Critical limits.** `active()` is **per-PART, never per-CELL** (an active part with a $0 cost basis
still writes `\"0\"` in column (e)); it exists **only** in `schedule_d.rs`, and the full-return emitter
`schedule_d_full.rs` has **no equivalent**; the two emitters are mutually exclusive per year, so on a
full return **it is not in play at all**. Every one of these keys off the **value**, not off whether
anyone was asked.

### (e) Form-level and routing-level blanks

`schedule_N_lines() -> None` ⇒ `packet.rs:77-96` skips the whole PDF. `ScheduleDRouting`
(schedule_d_full.rs:191-259) leaves L21 untouched on 3 of 4 branches. `let skip_8d_to_10 =
lines.line9 == Usd::ZERO;` (schedule_se_full.rs:78). **All three answer \"does the form's own routing
reach this line\", never \"does this line have testimony behind it.\"** Schedule 2 line 21 is the only
line in the entire survey structurally incapable of printing a zero — because its gate is on its own
value and a zero total is expressed as an *absent form*.

### (f) The provenance-sibling pattern — built, wired to advisories, deliberately NOT printed

`CarryProvenance` siblings exist for two carryovers:
`capital_loss_carryforward_in_provenance` (return_inputs.rs:516-527) — *\"without it a zero is
uninterpretable — 'the filer has no carryover' and 'nobody ever asked' are the same bytes\"* — and
`charitable_carryover_in_provenance` (return_inputs.rs:551-557).

★★ `advisories.rs:621-633` computes **exactly the predicate a blank-capable line would need**:
```rust
let user = crate::tax::return_inputs::CarryProvenance::User;
let cl = ri.capital_loss_carryforward_in.short == Usd::ZERO
    && ri.capital_loss_carryforward_in.long == Usd::ZERO
    && ri.capital_loss_carryforward_in_provenance == user;
```
and fires `Advisory::BenefitCarryoversNotStated`. **So on the very return where btctax tells the filer
\"your carryovers were not stated\", it simultaneously prints `0` on Schedule D lines 6 and 14 asserting
they are zero.** `classifier.rs:131-133` classes the bit *\"no print, no tax direction\"* — the one bit
that distinguishes the two blanks is deliberately kept off the page.

### (g) The `Option<Usd>` + `Option<bool>`-gate design, already written down and argued

`return_inputs.rs:620-636`, the §911/931/933 add-backs:
> *\"`Option<Usd>`, NOT defaulted `Usd`, and the distinction is the whole point. A plain `Usd`
> defaulting to 0 cannot tell \"zero because the filer has none\" from \"zero because nobody asked\"\"* —
> with the gate carried by an `Option<bool>` declaration because *\"`Option<bool>` is a leaf the
> classifier **forbids** `_` on, so it cannot be added without a human classifying it — whereas
> `Option<Usd>` is a scalar the `_` rule permits, which would make this convention again.\"*

**That paragraph is a ready-made design, and it names the reason a bare `Option<Usd>` is not
sufficient on its own.**

---

## 6. THE HARD CASES

Where \"blank or zero\" is genuinely ambiguous, or turns on a fact btctax does not have.

### H1 — What does a TOTAL render as when an operand is not stated?
**Unavoidable: 9 of the 64 are totals.** Schedule 1 line 9 is the extreme — *\"Add lines 8a through
8z\"* with **20 of 21 operands structurally absent** (no fields exist). Schedule 3 line 8 sums five
credits the product deliberately never asks about and whose advisory exists *because they may well be
nonzero*. Making operands `Option<Usd>` does not by itself decide what `Some(5000) + None + None`
prints. All three lawful moves (collect / refuse / blank) are defensible **per line**, and inheriting
one rule for all nine is probably wrong.

### H2 — Is a reconciled LEDGER the filer's testimony?
Schedule C line 1, Schedule D 3(d)/(e)/(h), 1040 line 7, Schedule 1 line 8v. Reader 2 said yes for 8v
(*\"btctax refuses to produce a return for an unreconciled year\"*); Reader 4 said no for Sch C L1
(no channel for non-crypto receipts) and self-flagged the reversal. Moves the count by ~4.

### H3 — The SAME CELL, two emitters, opposite dispositions — and the rule is already written
`form1040.rs:8-10` (crypto slice), verbatim: *\"Schedule D INACTIVE ... → **7a BLANK**. Stamping \"-0-\"
against a blank Schedule D line 16 would be an unearned zero-capital-gains claim.\"* The full-return
emitter writes the same cell via `push_money` with no test (form1040_full.rs:115). One of the two is
wrong, and the survey cannot say which without deciding H2. **B3-shaped: the fix already exists in the
tree and nobody carried it across.**

### H4 — Supplied-then-zeroed: a computed zero over a fact the filer DID state
An `Option<Usd>` at the leaves misses these entirely:
- **Sch A 8a** — `let mortgage_8a = if mortgage_mixed_use_box { Usd::ZERO } else { a.mortgage_interest_1098 };`
  (return_1040.rs:410-414). The filer reports $12,000 of Form-1098 interest and holds the 1098; the
  cell swears `0`. Only a checked box and an advisory qualify it — neither is on the amount line.
- **Sch A 11/12** — `current_allowed = current_total.min(ceiling)`, `ceiling = 0.60 * agi.max(ZERO)`
  (charitable.rs:63,117-119,141-147). A $10,000 gift on a ≤0-AGI return prints `0` and routes to
  `carryover_out`.
Is this in scope for §G-11, or a separate class?

### H5 — The mirror defect: an INSTRUCTED zero that btctax suppresses
Schedule SE. The form gives two instructions pointing opposite ways: line 8a's *\"If $168,600 or more,
skip lines 8b through 10, and go to line 11\"* versus line 9's *\"If zero or less, enter -0- here and on
line 10 and go to line 11\"*. `skip_8d_to_10` follows the first and blanks all three. Needs
adjudication against `i1040sse`. **§G-11's fix could make this worse if it only ever adds blanks.**

### H6 — Elections and conditional lines are a different class than amounts
- **1040 L35a** — *\"Amount of line 34 you want refunded to you\"* is an **election** (refund now vs
  apply to 2025 on line 36). btctax never asks, writes `line34` into the cell, leaves L36 blank.
- **1040 L34/L37** — the form makes L34 conditional in terms (*\"**If line 33 is more than line 24**\"*)
  and prints `0` when the condition fails; both print `0` when payments exactly equal tax.
The brief's test (\"was the fact supplied?\") does not cleanly apply to an unasked *choice*.

### H7 — Where the `None` genuinely means \"computed, and there is none\"
`Sch 2 L4` (`sch_se.map_or(ZERO)` — the §6017 $400 floor was not met) and `Sch 2 L11`. Reader 2 called
these NOT fabrications; Reader 1 called the structurally identical `1040 L23`
(`sch_2.map_or(ZERO)`) a fabrication because the total cites a schedule that is not attached.
**The readers split on the same idiom.** Is \"a schedule that legitimately does not file\" a stated zero
or an absent one? Note the form text on Sch 2 L4 is *\"Self-employment tax. **Attach Schedule SE**\"* —
and no Schedule SE is attached.

### H8 — Closed surfaces with no channel at all
Not defaulted — **absent**. Form 8960 lines 9a/9b/9c/10 (and every filer reaching 8960 is over the
§1411(b) threshold, i.e. exactly the population with allocable state income tax under 9b). Schedule 1
lines 2a/2b, 4, 5, 6 and 8a–8z. Schedule 2 Part II (Sch H, Form 5329, 4137/8919). Schedule 3 line 9
(net PTC) and 1040 line 17's excess-APTC half. 1099-R withholding (1040 25b — *i1040gi's own primary
case for that line*). Direct sales of collectibles / §1250 realty (Sch D 18/19). **Blank the line,
refuse the return, or advisory?** All three are defensible and the choice differs per line.

### H9 — 1040 line 19, the CTC/ODC: a hardcoded zero on a return that names the child
`fullreturn_inputs.toml` carries `[[header.dependents]] name = \"Sam Doe\" / relationship = \"Son\" /
date_of_birth = [2012, 106]` — a 12-year-old CTC-qualifying child, printed into the dependents table
with the CTC/ODC boxes **deliberately unchecked** — and `let line19 = Usd::ZERO;`. The struct's own
doc **concedes the discrimination and declines it**: *\"Conservative omissions print as absent, not
zero, where the form allows — but line 19 (the CTC/ODC) is a computed credit line the form expects, so
it prints `0`.\"* Is \"the form expects a computed credit line\" a sufficient reason?

### H10 — Direction is not uniform, and one site runs the *other* way
Most fabrications overstate tax (harmless to the fisc, costly to the filer, **invisible to both
oracles and every reconciliation**). Two classes do not:
- **Schedule D line 13** (capital gain distributions) **understates** tax — omits real long-term income.
- **1040 25a/25b/26** (withholding, estimated payments) deflate line 33 and inflate line 37 *\"amount
  you owe\"* — the exact shape the repo was burned by once and wrote down: *\"★ Dropping this line told a
  filer who had ALREADY paid with their extension to pay it again\"* (printed.rs:1306-1309).
§G-11 forbids opining on lawfulness — but does direction legitimately order the work?

### H11 — Form 6251: 18 sites in waiting, including the standing example
Lines 2c–2t have **no fields**, the form reaches no PDF, and `line4 = line1 + line2a + line2b + line3`
treats all 18 as $0. The module doc already schedules the moment it becomes a defect: *\"Tier 2, which
files the form rather than only computing it, must give 2c–2t real fields.\"* If those land as `Usd`
under today's unconditional `push_money`, **CLAUDE.md's standing example — an ISO exercise printed as
$0 on line 2i — ships on that commit.** Sequencing decision, not a code decision.

---

## 7. OPEN QUESTIONS FOR THE BRAINSTORM

1. **Is the not-stated state a property of the VALUE or of the LINE?** `Option<Usd>` threaded through
   `ReturnInputs` → `AbsoluteReturn` → printed structs → `push_money` makes it a value. A per-line
   decision table (each line records which of the three lawful moves it takes, and why) makes it a
   line. The `form8995.rs` precedent is a *line* gate (`std::ptr::eq` on a map cell); the `8283
   claimed_deduction` precedent is a *value* gate. Both work today, on one line each.
2. **What does a total do with a not-stated operand?** Propagate not-stated · treat as zero and print ·
   refuse · decide per line. Schedule 1 line 9 (20 of 21 operands absent) and Schedule 3 line 8
   (\"never a misleading 0\" on the parts, `0` on the sum) are the cases that force it.
3. **Does a reconciled ledger constitute the filer's testimony?** Determines Schedule C line 1,
   Schedule D 3/10, 1040 line 7 — and therefore whether the number is 64 or ~60, and whether the
   crypto slice's blank or the full return's `0` on 1040 7a is the bug (H3).
4. **Is a pure carry (`let lineN = lineM;`) its own fabrication site or an upstream one?** Readers
   split on identical shapes (Sch B 4 = yes, 1040 1z = no). Moves the number by up to 9 and decides
   whether the fix must touch the totals at all.
5. **Serde-requiredness or `Option`?** Both mechanisms are already in the repo doing this job:
   `box1_wages`/`box1_interest`/`box1a_ordinary` carry no `#[serde(default)]` and a TOML omitting them
   *refuses to parse*; the §911 add-backs use `Option<Usd>` + an `Option<bool>` gate. Requiredness
   fails loud at parse and breaks every existing TOML; `Option` fails silent and needs a classifier
   rule to stay honest.
6. **Does `ReturnInputs` change shape, or does provenance ride alongside?** `CarryProvenance` is the
   in-repo sibling pattern (built, advisory-wired, deliberately not printed). Siblings are
   non-breaking and already proven; a shape change is the thing the compiler can enforce.
7. **Blank, or refuse?** §G-11 gives exactly three lawful moves. For a line where silence *asserts*
   (class A) — 1040 25a withholding, Schedule SE 8a Medicare wages, both document-matched by the
   Service against copies it already holds — is blank sufficient, or must the return refuse? Answering
   \"always blank\" makes the fix mechanical and may be wrong.
8. **Should the classifier's `_`-forbidden rule extend to `Usd` leaves?** It is the instrument built
   for exactly this and it currently exempts every money field by design (*\"`_` is PERMITTED on other
   scalar leaves (`String`, `Usd`, `Date`, …)\"*). Extending it makes the census the permanent gate and
   `E0063`s every omission — at the cost of forcing a human classification on ~200 leaves. B1 applies:
   *which test reds when this checker is removed?*
9. **Does the input-form seam grow `Money(Option<Usd>)`?** Without it, even the interactive path
   cannot express not-stated (a pre-filled `0` scrolled past == a typed `0`), and Schedule 3 line 10 —
   the one fabrication site *with* a real interactive field — stays broken. With it, every money field
   in the TUI needs a rendering decision for the empty state.
10. **Do we print `\"0\"` or `\"-0-\"` where the form instructs a zero?** `fmt_money` emits the bare
    string today; the only `-0-` literal in the repo is the crypto-slice 1040 line 7a. Cosmetic, or
    part of the same speech-act argument?
11. **Is \"supplied-then-zeroed\" (H4) in scope?** Schedule A 8a mixed-use and the §170(b) charitable
    ceilings print `0` over facts the filer *did* state. Not fabrications under the brief's test — but
    they are affirmative sworn zeros contradicting the filer's own testimony, and no leaf-level fix
    touches them.
12. **Are unasked ELECTIONS a separate program?** 1040 line 35a (refund vs apply-to-2025) and the
    L34/L37 conditional pair are unasked *choices*, not unasked *amounts*. Fold in, or file separately?
13. **Does Schedule SE 8d/9/10 get fixed in the same breath (H5)?** It is the mirror defect — an
    instructed `-0-` that btctax suppresses. A fix that only ever adds blanks entrenches it, and
    adjudicating it needs `i1040sse`, not the form text.
14. **Does Form 6251 Tier 2 land before or after this fix?** 18 sites in waiting including the
    standing ISO/2i example. Fix-first costs a rebase; Tier-2-first ships CLAUDE.md's canonical defect
    on purpose.
15. **Does direction order the work?** ~62 of 64 overstate tax; Schedule D 13 and the 1040
    withholding/payments cluster understate it or inflate \"amount you owe\". §G-11 forbids opining on
    lawfulness — is sequencing by direction a legitimate use of that fact, or the same judgment
    smuggled in?
16. **What is the B1 kill-test?** Every fix here is a *suppression*, and a suppression that fires when
    it should not is invisible — a missing cell looks like the common correct case. The negative test
    has to plant a defect that *prints a zero it should not* AND one that *blanks a line it must not*
    (the 24 instructed zeros). What single instrument reds on both?
","numLines":613,"startLine":1,"totalLines":613}},"sourceToolAssistantUUID":"17008b02-383f-4243-82d5-2f21d2cd723c","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"3823db67-536d-43fe-9cd8-f5eb843f4c3b","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNCFhbtZem4qzM5RvEG","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISqwUKiAEIEBgCKkDrEwhg+yw+XFoAC5S0VutiL8whh9EMHmmlMIX8TJGqit93NjpDm9eehbz/ILReMoZND4PY6Ut+r+/Bn/ngPSa4Mg5jbGF1ZGUtZmFibGUtNTgBQgh0aGlua2luZ1okZTJmYmFjOTAtMDM4ZC00MjYzLWFjMDUtZWUxNTVlMGNjZTg2EgyyOO1/fH5qbJPD09AaDIIkYLvSxA6AVGC3MSIw0FHGwwwsVWKKs+xGTqLTW3kpqb8FL/Co9UyFdtUrp0wM+UPbpaKEplBzY9bdr4+/Ks8DZkXBh1dMZ1WYsEev9VBEEiYo1bu/anfZP72T21QtNrgdn82IQwVGyesxAh1D7h8OXlctduCKEAuIKE3hs8hac5P6FH5+SALmsEx3ew+1XOW7keb77MjUldtUTH/sLZYWQddUGa7RxPHn3IaPoiFq2tFQz9uPZBCPU5Gq4iopypQdYgL+bLBtkkUyF8mNJz6qY7ZebBs/FdAGr//qbKCuk9gwkirevy8VrhSoOP6yyMfSmE7EM3PWUzK6fnd3Ud8oy+hGaxY6OyyvXF4QJFp0VsdrVTBdyAWTCKaJ0a6XPuR1pMr04v7uY784QON83QuCABt9s2NzQBHwLQqAB+pM5nV95nYX0aDRm506LSXKGrR9paC9XedYmqRBgmAx7yBuqitqS5YZyUHrzSzVU4BMkUl9XCgo9ffcyXZ6Yyi5wVtITzlVRvhIc2xBPUT0zLCqYv136uWK3n19Ovq0t9D0fRzOaTbGu8Vcju74Y0OZ/6LdMBIGmeIiNHA6JNtcjwAfZH6FFigwZ/HYpVfW2sojwNVa/xR8iniA2flQYTDAr6MkeuTDcF4pr2sEdX2TkifuG5apMUGseyvvnxntD6V9QRUb0hR7hn2EGQbFlIXn/hgB"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":21631,"cache_read_input_tokens":32595,"cache_creation":{"ephemeral_5m_input_tokens":21631,"ephemeral_1h_input_tokens":0},"output_tokens":5,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNCDxRaR4RUtdaaFrPY","attributionAgent":"general-purpose","type":"assistant","uuid":"e54541a8-cc9c-49f3-aaf2-4855cb7c3883","timestamp":"2026-08-01T04:04:42.321Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"e54541a8-cc9c-49f3-aaf2-4855cb7c3883","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNCFhbtZem4qzM5RvEG","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01RZJsyzuDH1qc8XmPv3YeeY","name":"Bash","input":{"command":"wc -l /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/printed.rs /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_1040.rs /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_inputs.rs /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/classifier.rs /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/qbi.rs /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/other_taxes.rs /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/form6251.rs; ls /scratch/code/bitcoin_tax/crates/btctax-forms/src/","description":"List file sizes and forms crate contents"},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":21631,"cache_read_input_tokens":32595,"cache_creation":{"ephemeral_5m_input_tokens":21631,"ephemeral_1h_input_tokens":0},"output_tokens":5,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNCDxRaR4RUtdaaFrPY","attributionAgent":"general-purpose","type":"assistant","uuid":"701b1755-593d-40d1-a6e0-44f09e54f154","timestamp":"2026-08-01T04:04:44.899Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"701b1755-593d-40d1-a6e0-44f09e54f154","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01RZJsyzuDH1qc8XmPv3YeeY","type":"tool_result","content":"  2991 /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/printed.rs
  5321 /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_1040.rs
   793 /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_inputs.rs
   587 /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/classifier.rs
   891 /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/qbi.rs
   837 /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/other_taxes.rs
  1384 /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/form6251.rs
 12804 total
cells.rs
error.rs
fill8949_full.rs
fill8949.rs
form1040_full.rs
form1040.rs
form8275.rs
form8283.rs
form8959.rs
form8960.rs
form8995.rs
lib.rs
map.rs
overflow.rs
packet.rs
pdf.rs
schedule23.rs
schedule_a.rs
schedule_b.rs
schedule_c.rs
schedule_d_full.rs
schedule_d.rs
schedule_se_full.rs
schedule_se.rs
transcribe.rs
verify.rs
watermark.rs
wrap.rs","is_error":false}]},"uuid":"3bfaf236-4863-4dd3-a813-e237089ac79c","timestamp":"2026-08-01T04:04:44.990Z","toolUseResult":{"stdout":"  2991 /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/printed.rs
  5321 /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_1040.rs
   793 /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_inputs.rs
   587 /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/classifier.rs
   891 /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/qbi.rs
   837 /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/other_taxes.rs
  1384 /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/form6251.rs
 12804 total
cells.rs
error.rs
fill8949_full.rs
fill8949.rs
form1040_full.rs
form1040.rs
form8275.rs
form8283.rs
form8959.rs
form8960.rs
form8995.rs
lib.rs
map.rs
overflow.rs
packet.rs
pdf.rs
schedule23.rs
schedule_a.rs
schedule_b.rs
schedule_c.rs
schedule_d_full.rs
schedule_d.rs
schedule_se_full.rs
schedule_se.rs
transcribe.rs
verify.rs
watermark.rs
wrap.rs","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"701b1755-593d-40d1-a6e0-44f09e54f154","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"3bfaf236-4863-4dd3-a813-e237089ac79c","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNCFhbtZem4qzM5RvEG","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01GBeLCUExdDXqYsvTpcq3Jd","name":"Bash","input":{"command":"grep -n \"G-11\" /scratch/code/bitcoin_tax/FOLLOWUPS.md | head -20","description":"Find G-11 in FOLLOWUPS.md"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":21631,"cache_read_input_tokens":32595,"output_tokens":708,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":21631},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":708,"cache_read_input_tokens":32595,"cache_creation_input_tokens":21631,"cache_creation":{"ephemeral_5m_input_tokens":21631,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbNCDxRaR4RUtdaaFrPY","attributionAgent":"general-purpose","type":"assistant","uuid":"01376561-9132-4d2d-b483-abd817642e62","timestamp":"2026-08-01T04:04:45.632Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"01376561-9132-4d2d-b483-abd817642e62","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01GBeLCUExdDXqYsvTpcq3Jd","type":"tool_result","content":"795:★ Interacts with **§G-11**: line 7 printing `0` is defensible as a *computed* zero (no transactions ⇒
957:**Owning phase: NOT B3.** Whole-surface, and it interlocks with **§G-11** (which blocks its honest
1112:must never print `0` to show the filer answered. That is why §G-11 gates the honest version.
1174:be *a choice*, not an absence in the software. Cf. §G-11: the theme is identical, that btctax should not
1178:legal judgement and is out of scope in the same way intent is (§G-11's scope bound). The narrow question
1181:### G-11 — ★★★ ARCHITECTURAL: the emitter cannot express \"no testimony\" — `Usd::ZERO` prints `0`
1650:read. That is the same answered-ness problem the question registry solved for booleans (§G-11's","is_error":false}]},"uuid":"0a0eb48c-d0d7-4588-ba14-6a20b7d5c411","timestamp":"2026-08-01T04:04:45.721Z","toolUseResult":{"stdout":"795:★ Interacts with **§G-11**: line 7 printing `0` is defensible as a *computed* zero (no transactions ⇒
957:**Owning phase: NOT B3.** Whole-surface, and it interlocks with **§G-11** (which blocks its honest
1112:must never print `0` to show the filer answered. That is why §G-11 gates the honest version.
1174:be *a choice*, not an absence in the software. Cf. §G-11: the theme is identical, that btctax should not
1178:legal judgement and is out of scope in the same way intent is (§G-11's scope bound). The narrow question
1181:### G-11 — ★★★ ARCHITECTURAL: the emitter cannot express \"no testimony\" — `Usd::ZERO` prints `0`
1650:read. That is the same answered-ness problem the question registry solved for booleans (§G-11's","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"01376561-9132-4d2d-b483-abd817642e62","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"0a0eb48c-d0d7-4588-ba14-6a20b7d5c411","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNCqdHAQebf3cckwJK6","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_013dNpYgLLHQLLbtvd3CJV27","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/FOLLOWUPS.md","offset":1181,"limit":80},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1562,"cache_read_input_tokens":54226,"cache_creation":{"ephemeral_5m_input_tokens":1562,"ephemeral_1h_input_tokens":0},"output_tokens":61,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNCpnBYnYj5cG5nMgFT","attributionAgent":"general-purpose","type":"assistant","uuid":"fcf2f27b-8520-475d-a06a-362c2d92d5a0","timestamp":"2026-08-01T04:04:49.573Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"fcf2f27b-8520-475d-a06a-362c2d92d5a0","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_013dNpYgLLHQLLbtvd3CJV27","type":"tool_result","content":"1181\t### G-11 — ★★★ ARCHITECTURAL: the emitter cannot express \"no testimony\" — `Usd::ZERO` prints `0`
1182\t
1183\t**Owning phase: NOT B3.** This is a whole-surface finding, larger than Schedule 1-A, and it must not be
1184\tsmuggled into a task that cannot discharge it. B3 must (a) not make it worse, and (b) carry the
1185\tdistinction in its own types so it is ready when this is fixed. Filed 2026-07-29.
1186\t
1187\t**The framing that produced it** (user, and it is the sharpest statement of this project's core hazard
1188\tyet): *\"Every entry on a line is effectively testimony from the filer against the filer in a court of law.
1189\tA blank line means no testimony provided and could be lawful and appropriate or an outright financial
1190\tcrime; the distinction is intent and that's not the domain of tax software.\"*
1191\t
1192\t**So a blank and a printed `0` are not two renderings of one value. They are different speech acts:**
1193\t
1194\t| output | what it says under §6065 |
1195\t|---|---|
1196\t| blank | **nothing.** No testimony on this line. |
1197\t| `0` | **an affirmative sworn statement** that the amount IS zero. |
1198\t
1199\tWhen btctax writes `0` on a line the filer was never asked about, it does not merely guess a number — it
1200\t**fabricates testimony and files it under someone else's signature.** That is a different and worse
1201\tfailure than a wrong figure, and it is the [answered-ness invariant] at the emission layer.
1202\t
1203\t**The defect, verified 2026-07-29.** `btctax-forms/src/lib.rs:77` is the whole money path:
1204\t
1205\t```rust
1206\tpub(crate) fn fmt_money(d: Usd) -> String { d.to_string() }   // Usd::ZERO -> \"0\"
1207\t```
1208\t
1209\tEvery money field on every emitted form is `Usd`, never `Option<Usd>`. **There is no representation for
1210\t\"no testimony\", so no line can choose it.** Zero-suppression exists only ad hoc and only for whole ROWS
1211\t(`schedule_d.rs:26`, `fill8949.rs:44`), never for line-level blank-vs-zero. Invisible to every test and
1212\tboth oracles, because `0` is the correct *value* in the overwhelming majority of cases — the defect is in
1213\tthe *act*, not the arithmetic.
1214\t
1215\t**★ Why this also EXPLAINS the class (A)/(B) split, and gives a sharper test than \"fail closed\".**
1216\t
1217\t| class | silence | therefore |
1218\t|---|---|---|
1219\t| (A) declaration | would **ASSERT** something | must be answered, or the return REFUSES |
1220\t| (B) benefit claim | **FORGOES** something (New Colonial Ice: the burden to claim is the filer's) | silence is lawful |
1221\t
1222\tSo the test for any unknown is **\"does the silence assert, or forgo?\"** — which is why §G-9's fix is
1223\tlegitimate (a dateless death forgoing the §63(f) box declines a benefit; it swears to nothing) and why a
1224\tprinted `0` on an unasked line is categorically different.
1225\t
1226\t**Bounds on the fix — from the same sentence, and easy to overshoot.** Intent is not ours to adjudicate,
1227\tso btctax has exactly three lawful moves: **collect** the testimony, **refuse** to produce the return, or
1228\tleave **genuinely blank**. It must never silently choose silence and present it as the filer's choice.
1229\t★ And it must equally never build the opposite thing — a heuristic that flags an omission as suspicious,
1230\tor any feature that opines on whether a blank is lawful. **Both directions are software deciding intent.**
1231\tThis is a hard scope boundary, not a preference.
1232\t
1233\t**Sketch, not a plan** (a real one needs its own spec): the emitter's money type grows a \"not stated\"
1234\tstate that survives to the AcroForm write, computations may not manufacture a stated zero from unstated
1235\tinputs, and each line records which of the three lawful moves it takes and why. The per-line decision is
1236\tthen a reviewable fact instead of an accident of `Decimal::default()`.
1237\t
1238\t### G-10 — ~~`xtask cite-check` verifies a quotation EXISTS in the manual, not that it is at the cited line~~ **LARGELY CLOSED same day**
1239\t
1240\t**Owning phase: B3 T2** (the conformance KAT is being written there anyway, against the same extracts).
1241\tFiled 2026-07-29 with the tool itself.
1242\t
1243\t`cargo run -p xtask -- cite-check` checks every quoted span in the Schedule 1-A spec and plan verbatim
1244\tagainst the committed IRS text-layer extracts (34/34 today). ★ **Its residual gap was found by mutation,
1245\tnot by reasoning:** changing S-1's line-11 quotation from *\"decrease … to the next lower whole number\"* to
1246\t*\"increase … to the next higher whole number\"* — inverting the rounding direction, the most dangerous
1247\tsingle fact in this form — **survives**, because line 28 genuinely says that. The checker asks \"is this
1248\tthe form's words?\", never \"are these THIS line's words?\".
1249\t
1250\t**★★ CLOSED for the class that matters, and NOT the way this entry proposed.** The filed fix was \"parse
1251\tthe attribution and require the span to appear in that line's region of the extract\" — more citation
1252\tchecking. The better answer, from the user: **floor and ceiling are a DECISION the code makes, so derive
1253\tit from the printed line instead of checking prose about it.**
1254\t
1255\t`tables.rs::schedule_1a_conformance` now reads each phase-out's direction *off the form* — a
1256\t`printed_line(label)` reader over the in-crate extract, parsing \"decrease … to the next lower whole
1257\tnumber\" ⇒ `Floor` and \"increase … to the next higher\" ⇒ `Ceil` (and panicking rather than defaulting if it
1258\tcan read neither) — and asserts it equals what `schedule_1a_params` assigned. **Neither side can drift
1259\talone:** editing the params reds it, and so does editing the extract.
1260\t"}]},"uuid":"f83c7488-9509-4ed1-867b-3eec8ac4c882","timestamp":"2026-08-01T04:04:49.579Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/FOLLOWUPS.md","content":"### G-11 — ★★★ ARCHITECTURAL: the emitter cannot express \"no testimony\" — `Usd::ZERO` prints `0`

**Owning phase: NOT B3.** This is a whole-surface finding, larger than Schedule 1-A, and it must not be
smuggled into a task that cannot discharge it. B3 must (a) not make it worse, and (b) carry the
distinction in its own types so it is ready when this is fixed. Filed 2026-07-29.

**The framing that produced it** (user, and it is the sharpest statement of this project's core hazard
yet): *\"Every entry on a line is effectively testimony from the filer against the filer in a court of law.
A blank line means no testimony provided and could be lawful and appropriate or an outright financial
crime; the distinction is intent and that's not the domain of tax software.\"*

**So a blank and a printed `0` are not two renderings of one value. They are different speech acts:**

| output | what it says under §6065 |
|---|---|
| blank | **nothing.** No testimony on this line. |
| `0` | **an affirmative sworn statement** that the amount IS zero. |

When btctax writes `0` on a line the filer was never asked about, it does not merely guess a number — it
**fabricates testimony and files it under someone else's signature.** That is a different and worse
failure than a wrong figure, and it is the [answered-ness invariant] at the emission layer.

**The defect, verified 2026-07-29.** `btctax-forms/src/lib.rs:77` is the whole money path:

```rust
pub(crate) fn fmt_money(d: Usd) -> String { d.to_string() }   // Usd::ZERO -> \"0\"
```

Every money field on every emitted form is `Usd`, never `Option<Usd>`. **There is no representation for
\"no testimony\", so no line can choose it.** Zero-suppression exists only ad hoc and only for whole ROWS
(`schedule_d.rs:26`, `fill8949.rs:44`), never for line-level blank-vs-zero. Invisible to every test and
both oracles, because `0` is the correct *value* in the overwhelming majority of cases — the defect is in
the *act*, not the arithmetic.

**★ Why this also EXPLAINS the class (A)/(B) split, and gives a sharper test than \"fail closed\".**

| class | silence | therefore |
|---|---|---|
| (A) declaration | would **ASSERT** something | must be answered, or the return REFUSES |
| (B) benefit claim | **FORGOES** something (New Colonial Ice: the burden to claim is the filer's) | silence is lawful |

So the test for any unknown is **\"does the silence assert, or forgo?\"** — which is why §G-9's fix is
legitimate (a dateless death forgoing the §63(f) box declines a benefit; it swears to nothing) and why a
printed `0` on an unasked line is categorically different.

**Bounds on the fix — from the same sentence, and easy to overshoot.** Intent is not ours to adjudicate,
so btctax has exactly three lawful moves: **collect** the testimony, **refuse** to produce the return, or
leave **genuinely blank**. It must never silently choose silence and present it as the filer's choice.
★ And it must equally never build the opposite thing — a heuristic that flags an omission as suspicious,
or any feature that opines on whether a blank is lawful. **Both directions are software deciding intent.**
This is a hard scope boundary, not a preference.

**Sketch, not a plan** (a real one needs its own spec): the emitter's money type grows a \"not stated\"
state that survives to the AcroForm write, computations may not manufacture a stated zero from unstated
inputs, and each line records which of the three lawful moves it takes and why. The per-line decision is
then a reviewable fact instead of an accident of `Decimal::default()`.

### G-10 — ~~`xtask cite-check` verifies a quotation EXISTS in the manual, not that it is at the cited line~~ **LARGELY CLOSED same day**

**Owning phase: B3 T2** (the conformance KAT is being written there anyway, against the same extracts).
Filed 2026-07-29 with the tool itself.

`cargo run -p xtask -- cite-check` checks every quoted span in the Schedule 1-A spec and plan verbatim
against the committed IRS text-layer extracts (34/34 today). ★ **Its residual gap was found by mutation,
not by reasoning:** changing S-1's line-11 quotation from *\"decrease … to the next lower whole number\"* to
*\"increase … to the next higher whole number\"* — inverting the rounding direction, the most dangerous
single fact in this form — **survives**, because line 28 genuinely says that. The checker asks \"is this
the form's words?\", never \"are these THIS line's words?\".

**★★ CLOSED for the class that matters, and NOT the way this entry proposed.** The filed fix was \"parse
the attribution and require the span to appear in that line's region of the extract\" — more citation
checking. The better answer, from the user: **floor and ceiling are a DECISION the code makes, so derive
it from the printed line instead of checking prose about it.**

`tables.rs::schedule_1a_conformance` now reads each phase-out's direction *off the form* — a
`printed_line(label)` reader over the in-crate extract, parsing \"decrease … to the next lower whole
number\" ⇒ `Floor` and \"increase … to the next higher\" ⇒ `Ceil` (and panicking rather than defaulting if it
can read neither) — and asserts it equals what `schedule_1a_params` assigned. **Neither side can drift
alone:** editing the params reds it, and so does editing the extract.
","numLines":80,"startLine":1181,"totalLines":4747}},"sourceToolAssistantUUID":"fcf2f27b-8520-475d-a06a-362c2d92d5a0","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"f83c7488-9509-4ed1-867b-3eec8ac4c882","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNCqdHAQebf3cckwJK6","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_015yFEC6QvA5AZQ3b5gBo54r","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/classifier.rs"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1562,"cache_read_input_tokens":54226,"output_tokens":199,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":1562},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":199,"cache_read_input_tokens":54226,"cache_creation_input_tokens":1562,"cache_creation":{"ephemeral_5m_input_tokens":1562,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbNCpnBYnYj5cG5nMgFT","attributionAgent":"general-purpose","type":"assistant","uuid":"da5c20d7-58de-4d33-9e35-717214b8c43c","timestamp":"2026-08-01T04:04:49.691Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"da5c20d7-58de-4d33-9e35-717214b8c43c","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_015yFEC6QvA5AZQ3b5gBo54r","type":"tool_result","content":"1\t//! ★ P9 §3.3 — THE CLASSIFIER. Makes the answered-ness invariant STRUCTURAL: every `bool` /
2\t//! `Option<bool>` / defaulted-enum field reachable from [`ReturnInputs`] is classified — as a registry
3\t//! DECLARATION (class A), or EXEMPTED with its §2 class and the statutory reason it is lawful.
4\t//!
5\t//! **The compile-time guarantee.** Every struct is destructured with **NO `..`**, so a newly-added field
6\t//! is a `pattern does not mention field` COMPILE ERROR until a human edits this file. `#![deny(unused_variables)]`
7\t//! then makes a named binding that *ignores* its field a hard error too (the `w2s: whatever` evasion r3 I-5
8\t//! defeated the r3 wording with). So: **a new such field does not compile until a human LOOKS.**
9\t//!
10\t//! **★ The honest limit (r3 I-5), stated at its true strength.** The compiler forces \"a human must EDIT
11\t//! the classifier\", NOT \"classified it correctly.\" The residual evasions — a `_`-prefixed binding, or
12\t//! `let _ = x;` — are **grep-able REVIEW residue**, not compile errors. That is the whole guarantee, and it
13\t//! is worth having: every recurrence of this class began with a field nobody looked at.
14\t//!
15\t//! **★ The `_` rule (r2 M-6).** `_` — and every `_`-prefixed binding — is **FORBIDDEN** on structs,
16\t//! collections, and `bool` / `Option<bool>` / defaulted-enum leaves (they must recurse or be classified).
17\t//! `_` is **PERMITTED** on other scalar leaves (`String`, `Usd`, `Date`, `Option<Date>`, `Option<String>`).
18\t//!
19\t//! This module does no tax arithmetic and is never on a compute path; [`classify`] returns a [`Census`] only
20\t//! so a test can prove the registry declarations line up with [`FORM_QUESTIONS`].
21\t#![deny(unused_variables)]
22\t
23\tuse crate::tax::questions::QuestionId;
24\tuse crate::tax::return_inputs::{
25\t    Box12Entry, CharitableCarryItem, CharitableGift, Dependent, Form1099Div, Form1099G,
26\t    Form1099Int, HouseholdHeader, Payments, Person, QbiInputs, ReturnInputs, Schedule1Inputs,
27\t    ScheduleAInputs, ScheduleCInputs, W2,
28\t};
29\tuse crate::tax::types::Carryforward;
30\t
31\t/// The §2 class under which a defaulted input is lawful. (Class **A** is not here — it is a registry
32\t/// DECLARATION, recorded via [`Census::declaration`]; class **D** is gone, deleted in step 9.)
33\t#[derive(Debug, Clone, Copy, PartialEq, Eq)]
34\tpub enum Class {
35\t    /// **(B) BENEFIT CLAIM** — *New Colonial Ice*: the burden to CLAIM is the filer's, so `false`/absent is
36\t    /// lawful, but the forgone benefit gets an advisory (§2.2).
37\t    BenefitClaim,
38\t    /// **(C) NO TAX DIRECTION** — neither asserts nor claims; a lawful silent default (§2.1).
39\t    NoTaxDirection,
40\t    /// serde-REQUIRED at import (the field has no `#[serde(default)]`): a TOML without it refuses to parse,
41\t    /// so there is no default to launder (§2.1 / §2.8).
42\t    SerdeRequired,
43\t    /// The value IS data the record carries (a typed classification), not a defaulted answer for the filer.
44\t    DataDerived,
45\t    /// A defaulted enum WITH tax direction, tracked as an OPEN issue owned by a follow-up (§2.8).
46\t    TrackedFollowup,
47\t}
48\t
49\t/// The census [`classify`] produces: which registry questions it declared, and every exemption with its
50\t/// class and statutory reason. Used only by the tests — the load-bearing guarantee is the compile.
51\t#[derive(Debug, Default)]
52\tpub struct Census {
53\t    pub declarations: Vec<QuestionId>,
54\t    pub exemptions: Vec<(Class, &'static str)>,
55\t}
56\t
57\timpl Census {
58\t    /// A class-(A) DECLARATION: the filer asserts it; its liveness and refusal live in [`FORM_QUESTIONS`].
59\t    fn declaration(&mut self, _leaf: &Option<bool>, id: QuestionId) {
60\t        self.declarations.push(id);
61\t    }
62\t    /// An exempted default — lawful, with its §2 class and the statutory reason it is lawful.
63\t    fn exempt<T>(&mut self, _leaf: &T, class: Class, statutory_reason: &'static str) {
64\t        self.exemptions.push((class, statutory_reason));
65\t    }
66\t}
67\t
68\t/// ★ Destructure EVERY struct reachable from [`ReturnInputs`] with no `..`, classifying every
69\t/// `bool`/`Option<bool>`/defaulted-enum leaf. Never called on a compute path — its whole job is to fail
70\t/// COMPILATION when a new such field appears unclassified.
71\tpub fn classify(ri: &ReturnInputs) -> Census {
72\t    let mut c = Census::default();
73\t    let ReturnInputs {
74\t        // ★ §G-15 — the tax year is not a DECLARATION or a benefit claim; it is the scope those are
75\t        // asked in. It carries no answered-ness of its own, so the classifier ignores it (naming it
76\t        // explicitly rather than `..`, so the next field added is still a compile error).
77\t        tax_year: _,
78\t        filing_status,
79\t        header,
80\t        w2s,
81\t        int_1099,
82\t        div_1099,
83\t        g_1099,
84\t        schedule_c,
85\t        schedule_a,
86\t        itemize_election,
87\t        mfs_spouse_itemizes,
88\t        sch1,
89\t        payments,
90\t        capital_loss_carryforward_in,
91\t        capital_loss_carryforward_in_provenance,
92\t        charitable_carryover_in_provenance,
93\t        amt_carryover_same_as_regular,
94\t        amt_depreciation_same_as_regular,
95\t        charitable_carryover_in,
96\t        qbi,
97\t        foreign_accounts,
98\t        foreign_trust,
99\t        fbar_filing_required,
100\t        foreign_country_names: _, // String — scalar
101\t        donations_had_restrictions,
102\t        dual_status_alien,
103\t        // §164(b)(7)(B)(iv) / Schedule 1-A Part I MAGI add-backs. `Option<Usd>` scalar leaves, so the
104\t        // `_` rule permits binding them here — but they are NOT class-(B) forgone benefits like
105\t        // `medical`: an unasked add-back understates MAGI and RAISES two deductions. Their `None` is
106\t        // refused at the point of need (`SaltLimitation::line_5e`), not defaulted to zero.
107\t        has_income_exclusion,
108\t        excluded_puerto_rico_income: _,
109\t        form_2555_line45: _,
110\t        form_2555_line50: _,
111\t        form_4563_line15: _,
112\t    } = ri;
113\t    c.exempt(
114\t        filing_status,
115\t        Class::SerdeRequired,
116\t        \"filing_status has no #[serde(default)] — a TOML without it refuses to parse, so no default to \\
117\t         launder (§2.1)\",
118\t    );
119\t    c.declaration(mfs_spouse_itemizes, QuestionId::MfsSpouseItemizes);
120\t    c.declaration(foreign_accounts, QuestionId::ForeignAccounts);
121\t    c.declaration(foreign_trust, QuestionId::ForeignTrust);
122\t    // ★ §G-20a — provenance for the two benefit carryovers. Class (C): no print, no tax direction.
123\t    // They exist so a ZERO can be told apart from an UNASKED, which is what makes an honest advisory
124\t    // possible; the per-item provenance on `CharitableCarryItem` cannot speak for an EMPTY list.
125\t    c.exempt(
126\t        capital_loss_carryforward_in_provenance,
127\t        Class::NoTaxDirection,
128\t        \"§2.8: CarryProvenance (§1212(b) capital-loss carryover, §G-20a) — no print, no tax direction\",
129\t    );
130\t    c.exempt(
131\t        charitable_carryover_in_provenance,
132\t        Class::NoTaxDirection,
133\t        \"§2.8: CarryProvenance (§170(d)(1) charitable carryover LIST, §G-20a) — no print, no tax direction\",
134\t    );
135\t    // ★ Schedule B 7a's unnumbered FBAR sub-question. NOT a declaration: no figure on the return
136\t    // reads it (the printed chain writes the checkbox and nothing else), so its silence neither
137\t    // asserts nor forgoes — class (C). Asked as `SkippableId::FbarFilingRequired`, and skipping it
138\t    // fires `Advisory::FbarSubQuestionNotAnswered` quoting the form's Caution verbatim.
139\t    c.exempt(
140\t        fbar_filing_required,
141\t        Class::NoTaxDirection,
142\t        \"Schedule B line 7a's FBAR sub-question — no figure on the return reads it, and the penalty \\
143\t         the form's Caution names attaches to NOT FILING FinCEN Form 114 (a FinCEN obligation \\
144\t         independent of this box), not to leaving the box blank. Silence is lawful and prints a true \\
145\t         blank; `Advisory::FbarSubQuestionNotAnswered` fires (§2.1)\",
146\t    );
147\t    // ★★ Form 8283 5a/5b/5c, asked as ONE return-level universal (§G-21). Class (B) HERE — offered
148\t    // always, silence lawful — because the donations are in the LEDGER and liveness cannot see them.
149\t    // The MANDATORY half lives in `screen_absolute`, which can: on an ITEMIZING year an
150\t    // unanswered or `Some(true)` answer REFUSES.
151\t    c.exempt(
152\t        donations_had_restrictions,
153\t        Class::BenefitClaim,
154\t        \"Form 8283 5a/5b/5c (§G-21) — offered as a skippable so a filer who donated nothing is never \\
155\t         blocked; on an itemizing year that claims the §170 deduction `screen_absolute` makes it \\
156\t          mandatory (§2.2)\",
157\t    );
158\t    c.declaration(dual_status_alien, QuestionId::DualStatusAlien);
159\t    c.declaration(has_income_exclusion, QuestionId::HasIncomeExclusion);
160\t    c.declaration(
161\t        amt_carryover_same_as_regular,
162\t        QuestionId::AmtCarryoverSameAsRegular,
163\t    );
164\t    c.declaration(
165\t        amt_depreciation_same_as_regular,
166\t        QuestionId::AmtDepreciationSameAsRegular,
167\t    );
168\t    c.exempt(
169\t        itemize_election,
170\t        Class::NoTaxDirection,
171\t        \"§2.8: `Auto` takes max(standard, itemized) — an OPTIMIZATION that cannot lose money, not an \\
172\t         assertion and not a forgone benefit; §63(e) `ForceItemize` is opt-in\",
173\t    );
174\t    classify_header(&mut c, header);
175\t    for w in w2s {
176\t        classify_w2(&mut c, w);
177\t    }
178\t    for i in int_1099 {
179\t        classify_1099int(&mut c, i);
180\t    }
181\t    for d in div_1099 {
182\t        classify_1099div(&mut c, d);
183\t    }
184\t    for g in g_1099 {
185\t        classify_1099g(&mut c, g);
186\t    }
187\t    if let Some(sc) = schedule_c {
188\t        classify_schedule_c(&mut c, sc);
189\t    }
190\t    if let Some(a) = schedule_a {
191\t        classify_schedule_a(&mut c, a);
192\t    }
193\t    classify_schedule1(&mut c, sch1);
194\t    classify_payments(&mut c, payments);
195\t    classify_carryforward(&mut c, capital_loss_carryforward_in);
196\t    for item in charitable_carryover_in {
197\t        classify_charitable_carry(&mut c, item);
198\t    }
199\t    classify_qbi(&mut c, qbi);
200\t    c
201\t}
202\t
203\tfn classify_header(c: &mut Census, h: &HouseholdHeader) {
204\t    let HouseholdHeader {
205\t        taxpayer,
206\t        spouse,
207\t        address_street: _,
208\t        address_city: _,
209\t        address_state: _,
210\t        address_zip: _,
211\t        dependents,
212\t        can_be_claimed_as_dependent_taxpayer,
213\t        can_be_claimed_as_dependent_spouse,
214\t        spouse_had_no_income,
215\t        spouse_not_filing_a_return,
216\t        presidential_fund_taxpayer,
217\t        presidential_fund_spouse,
218\t        taxpayer_died_during_year,
219\t        spouse_died_during_year,
220\t        ip_pin: _, // Option<String> — scalar
221\t    } = h;
222\t    c.declaration(
223\t        can_be_claimed_as_dependent_taxpayer,
224\t        QuestionId::DependentTaxpayer,
225\t    );
226\t    c.declaration(
227\t        can_be_claimed_as_dependent_spouse,
228\t        QuestionId::DependentSpouse,
229\t    );
230\t    c.exempt(
231\t        presidential_fund_taxpayer,
232\t        Class::NoTaxDirection,
233\t        \"§2.1: 1040 presidential-fund box — §6096 is a fund DESIGNATION, not a tax liability\",
234\t    );
235\t    c.exempt(
236\t        presidential_fund_spouse,
237\t        Class::NoTaxDirection,
238\t        \"§2.1: 1040 presidential-fund box (spouse) — §6096, no tax direction\",
239\t    );
240\t    // ★★ §G-20 — i1040gi's two uncaptured MFS conditions for the spouse's §63(f) boxes. BENEFIT
241\t    // CLAIMS (New Colonial Ice): silence FORGOES the boxes and never grants them, so `false`/absent is
242\t    // lawful and the forgone benefit fires `Mfs63fSpouseBoxesForgone`.
243\t    c.exempt(
244\t        spouse_had_no_income,
245\t        Class::BenefitClaim,
246\t        \"§63(f)/i1040gi MFS condition (spouse had no income) — silence forgoes the spouse's aged/blind \\
247\t         boxes rather than granting them; `Advisory::Mfs63fSpouseBoxesForgone` names the cost (§2.2)\",
248\t    );
249\t    c.exempt(
250\t        spouse_not_filing_a_return,
251\t        Class::BenefitClaim,
252\t        \"§63(f)/i1040gi MFS condition (spouse isn't filing a return) — same reasoning (§2.2)\",
253\t    );
254\t    // §G-9: the §63(f) death carve-out. i1040gi states it twice — \"Death of a taxpayer\" and \"Death of
255\t    // spouse\" — so each is its own leaf. ★ BENEFIT CLAIMS, not declarations: silence FORGOES the
256\t    // age-65 addition (`is_aged`'s `(None, None)` arm returns `false`), so by the sharp test it is
257\t    // class (B), asked as `SkippableId::{Taxpayer,Spouse}DiedDuringYear`. They were briefly class-(A)
258\t    // refusals, and the taxpayer one was `live: |_| true` — it blocked every return for a fail-safe
259\t    // the compute layer already had.
260\t    c.exempt(
261\t        taxpayer_died_during_year,
262\t        Class::BenefitClaim,
263\t        \"§63(f)/§G-9 death carve-out (taxpayer) — New Colonial Ice: silence forgoes the age-65 \\
264\t         addition rather than granting it on an unresolved carve-out, so `false`/absent is lawful; \\
265\t         `Advisory::AgedBoxForfeitedDeathUnanswered` fires when a qualifying DOB makes the skip cost \\
266\t         money (§2.2)\",
267\t    );
268\t    c.exempt(
269\t        spouse_died_during_year,
270\t        Class::BenefitClaim,
271\t        \"§63(f)/§G-9 death carve-out (spouse) — same reasoning; the box is counted exactly when \\
272\t         `spouse_63f_boxes_count` says so (MFJ, or a qualifying MFS), which is the same predicate \\
273\t         that gates the prompt (§2.2)\",
274\t    );
275\t    classify_person(c, taxpayer);
276\t    if let Some(sp) = spouse {
277\t        classify_person(c, sp);
278\t    }
279\t    for d in dependents {
280\t        classify_dependent(c, d);
281\t    }
282\t}
283\t
284\tfn classify_person(c: &mut Census, p: &Person) {
285\t    let Person {
286\t        first_name: _,
287\t        last_name: _,
288\t        ssn: _,
289\t        date_of_birth: _, // Option<Date> — scalar
290\t        // Option<Date> — scalar. Its answered-ness is carried by the HEADER gates
291\t        // `taxpayer_died_during_year` / `spouse_died_during_year`, declared in `classify_header`.
292\t        date_of_death: _,
293\t        blind,
294\t        occupation: _,
295\t    } = p;
296\t    c.exempt(
297\t        blind,
298\t        Class::BenefitClaim,
299\t        \"§63(f) blindness — New Colonial Ice: the burden to CLAIM is the filer's, so `false`/absent is \\
300\t         lawful; the forgone benefit fires `BlindBoxForfeitedNotDeclared` (§2.2)\",
301\t    );
302\t}
303\t
304\tfn classify_dependent(_c: &mut Census, d: &Dependent) {
305\t    // No classifiable leaves — but destructured with no `..` so a future bool here is a compile error.
306\t    let Dependent {
307\t        name: _,
308\t        ssn: _,
309\t        relationship: _,
310\t        date_of_birth: _,
311\t    } = d;
312\t}
313\t
314\tfn classify_w2(c: &mut Census, w: &W2) {
315\t    let W2 {
316\t        owner,
317\t        employer: _,
318\t        box1_wages: _,
319\t        box2_fed_withheld: _,
320\t        box3_ss_wages: _,
321\t        box4_ss_withheld: _,
322\t        box5_medicare_wages: _,
323\t        box6_medicare_withheld: _,
324\t        box7_ss_tips: _,
325\t        box17_state_tax_withheld: _,
326\t        box19_local_tax: _,
327\t        box12,
328\t        box8_allocated_tips: _,
329\t        box10_dependent_care: _,
330\t    } = w;
331\t    c.exempt(
332\t        owner,
333\t        Class::SerdeRequired,
334\t        \"§2.8: W2.owner has no #[serde(default)], so the TOML import REQUIRES it; the enum's #[default] \\
335\t         Taxpayer reaches only Rust-side fixtures\",
336\t    );
337\t    for e in box12 {
338\t        classify_box12(c, e);
339\t    }
340\t}
341\t
342\tfn classify_box12(_c: &mut Census, e: &Box12Entry) {
343\t    let Box12Entry {
344\t        code: _, // String — scalar
345\t        amount: _,
346\t    } = e;
347\t}
348\t
349\tfn classify_1099int(_c: &mut Census, i: &Form1099Int) {
350\t    let Form1099Int {
351\t        payer: _,
352\t        box1_interest: _,
353\t        box2_early_withdrawal_penalty: _,
354\t        box3_treasury_interest: _,
355\t        box4_fed_withheld: _,
356\t        box6_foreign_tax: _,
357\t        box8_tax_exempt_interest: _,
358\t        box9_private_activity_bond_amt: _,
359\t    } = i;
360\t}
361\t
362\tfn classify_1099div(_c: &mut Census, d: &Form1099Div) {
363\t    let Form1099Div {
364\t        payer: _,
365\t        box1a_ordinary: _,
366\t        box1b_qualified: _,
367\t        box2a_capgain_distr: _,
368\t        box2b_unrecap_1250: _,
369\t        box2c_section_1202: _,
370\t        box2d_collectibles_28: _,
371\t        box4_fed_withheld: _,
372\t        box5_section_199a: _,
373\t        box7_foreign_tax: _,
374\t        box12_exempt_interest_dividends: _,
375\t        box13_private_activity_amt: _,
376\t    } = d;
377\t}
378\t
379\tfn classify_1099g(_c: &mut Census, g: &Form1099G) {
380\t    let Form1099G {
381\t        payer: _,
382\t        box1_unemployment: _,
383\t        box4_fed_withheld: _,
384\t    } = g;
385\t}
386\t
387\tfn classify_schedule_c(c: &mut Census, sc: &ScheduleCInputs) {
388\t    let ScheduleCInputs {
389\t        owner,
390\t        business_description: _,
391\t        naics_code: _,
392\t        accounting_method,
393\t        expenses: _,
394\t        payments_requiring_1099,
395\t        will_file_required_1099,
396\t    } = sc;
397\t    c.exempt(
398\t        owner,
399\t        Class::SerdeRequired,
400\t        \"§2.8: ScheduleCInputs.owner has no #[serde(default)] — serde-required at import\",
401\t    );
402\t    c.exempt(
403\t        accounting_method,
404\t        Class::TrackedFollowup,
405\t        \"§2.8: Cash default; `accrual` is accepted, unmodeled and UNREFUSED and flips the printed Sch C \\
406\t         line F — a known open Important, filed → P8\",
407\t    );
408\t    // ★ Schedule C lines I and J — the Form-1099 compliance pair. Class (B): asked as
409\t    // `SkippableId::ScheduleC1099{Required,Filed}`, silence lawful. Not `declaration(..)` because no
410\t    // figure on the return reads them and the form prints no Caution — but NOT plain \"no tax
411\t    // direction\" either, because §6721/§6722 exposure is real, which is what the skip advisory names.
412\t    //
413\t    // ★★ PRE-MERGE M8 — these are RECORDED, not bound with `_`. This module's own r2 M-6 rule is that
414\t    // an `Option<bool>` leaf never gets a bare `_`: the whole point of the census is to distinguish
415\t    // \"this encodes no decision\" from \"we forgot it\", and `_` erases exactly that distinction. Both
416\t    // were silently under-reported as answered-ness decisions and nothing redded. The four other
417\t    // class-(B) leaves this branch added were all recorded via `exempt`; these two were missed.
418\t    c.exempt(
419\t        payments_requiring_1099,
420\t        Class::BenefitClaim,
421\t        \"Schedule C line I — asked as `SkippableId::ScheduleC1099Required`; silence forgoes nothing on \\
422\t         the return itself, and the §6721/§6722 exposure is named by the skip advisory (§2.2)\",
423\t    );
424\t    c.exempt(
425\t        will_file_required_1099,
426\t        Class::BenefitClaim,
427\t        \"Schedule C line J — asked as `SkippableId::ScheduleC1099Filed`, and gated on line I being \\
428\t         Yes; an answer to J without a Yes on I is not a mark the form has a place for (§2.2)\",
429\t    );
430\t}
431\t
432\tfn classify_schedule_a(c: &mut Census, a: &ScheduleAInputs) {
433\t    let ScheduleAInputs {
434\t        medical: _,
435\t        salt_use_sales_tax,
436\t        salt_sales_tax_amount: _,
437\t        salt_state_estimated_payments: _,
438\t        salt_prior_year_balance_paid: _,
439\t        salt_real_estate: _,
440\t        salt_personal_property: _,
441\t        mortgage_interest_1098: _,
442\t        mortgage_all_used_to_buy_build_improve,
443\t        mortgage_dwelling_is_amt_qualified,
444\t        charitable,
445\t    } = a;
446\t    c.exempt(
447\t        salt_use_sales_tax,
448\t        Class::BenefitClaim,
449\t        \"§164(b)(5) sales-tax election — New Colonial Ice: lawful unasked; `SalesTaxElectionNotAsked` \\
450\t         advises when a Schedule A exists (§2.2)\",
451\t    );
452\t    c.declaration(
453\t        mortgage_all_used_to_buy_build_improve,
454\t        QuestionId::MortgageAllUsedToBuyBuildImprove,
455\t    );
456\t    c.declaration(
457\t        mortgage_dwelling_is_amt_qualified,
458\t        QuestionId::AmtQualifiedDwelling,
459\t    );
460\t    for g in charitable {
461\t        classify_charitable_gift(c, g);
462\t    }
463\t}
464\t
465\tfn classify_charitable_gift(c: &mut Census, g: &CharitableGift) {
466\t    let CharitableGift { class, amount: _ } = g;
467\t    c.exempt(
468\t        class,
469\t        Class::DataDerived,
470\t        \"the gift's CONTRIBUTION CLASS is data (which property was given), not a defaulted answer for the \\
471\t         filer\",
472\t    );
473\t}
474\t
475\tfn classify_schedule1(c: &mut Census, s: &Schedule1Inputs) {
476\t    let Schedule1Inputs {
477\t        state_refund_taxable: _,
478\t        student_loan_interest_paid: _,
479\t        ira_deduction_claimed: _,
480\t        hsa_activity,
481\t    } = s;
482\t    c.declaration(hsa_activity, QuestionId::HsaActivity);
483\t}
484\t
485\tfn classify_payments(_c: &mut Census, p: &Payments) {
486\t    let Payments {
487\t        estimated_tax_payments: _,
488\t        extension_payment: _,
489\t        other_withholding: _,
490\t    } = p;
491\t}
492\t
493\tfn classify_carryforward(_c: &mut Census, cf: &Carryforward) {
494\t    // FROZEN struct — destructuring it READS it, modifies nothing (§3.3). No classifiable leaves today; the
495\t    // guarantee is about the bool added tomorrow.
496\t    let Carryforward { short: _, long: _ } = cf;
497\t}
498\t
499\tfn classify_charitable_carry(c: &mut Census, item: &CharitableCarryItem) {
500\t    let CharitableCarryItem {
501\t        class,
502\t        amount: _,
503\t        origin_year: _,
504\t        provenance,
505\t    } = item;
506\t    c.exempt(
507\t        class,
508\t        Class::DataDerived,
509\t        \"the carryover item's contribution class is data, not a defaulted answer\",
510\t    );
511\t    c.exempt(
512\t        provenance,
513\t        Class::NoTaxDirection,
514\t        \"§2.8: CarryProvenance — no print, no tax direction\",
515\t    );
516\t}
517\t
518\tfn classify_qbi(c: &mut Census, q: &QbiInputs) {
519\t    let QbiInputs {
520\t        reit_ptp_carryforward_in: _,
521\t        reit_ptp_carryforward_in_provenance,
522\t        qbi_carryforward_in: _,
523\t        qbi_carryforward_in_provenance,
524\t    } = q;
525\t    c.exempt(
526\t        reit_ptp_carryforward_in_provenance,
527\t        Class::NoTaxDirection,
528\t        \"§2.8: CarryProvenance — no print, no tax direction\",
529\t    );
530\t    c.exempt(
531\t        qbi_carryforward_in_provenance,
532\t        Class::NoTaxDirection,
533\t        \"§2.8: CarryProvenance (Form 8995 line 3) — no print, no tax direction\",
534\t    );
535\t}
536\t
537\t#[cfg(test)]
538\tmod tests {
539\t    use super::*;
540\t
541\t    /// ★ §3.3 / §4 — the classifier's registry DECLARATIONS line up EXACTLY with [`FORM_QUESTIONS`]: every
542\t    /// registry `QuestionId` is declared once, and nothing is declared that is not a registry question. A
543\t    /// wrong or dropped `declaration(..)` (a mis-classification the COMPILER cannot catch — the honest
544\t    /// limit) fails here. Exercised on a fixture with a Schedule A so the mortgage declaration runs.
545\t    #[test]
546\t    fn every_registry_question_is_declared_exactly_once() {
547\t        let mut ri = ReturnInputs {
548\t            schedule_a: Some(ScheduleAInputs::default()),
549\t            ..Default::default()
550\t        };
551\t        // A spouse Person exercises the spouse branch too (its bools are on the header, always destructured,
552\t        // but this keeps the census exercise faithful to a populated return).
553\t        ri.header.spouse = Some(Person::default());
554\t        let census = classify(&ri);
555\t
556\t        for id in QuestionId::ALL {
557\t            assert_eq!(
558\t                census.declarations.iter().filter(|d| *d == id).count(),
559\t                1,
560\t                \"classifier must declare {id:?} exactly once (registry ⇔ classifier, §3.3)\"
561\t            );
562\t        }
563\t        assert_eq!(
564\t            census.declarations.len(),
565\t            QuestionId::ALL.len(),
566\t            \"the classifier declares EXACTLY the registry questions — no more, no less\"
567\t        );
568\t    }
569\t
570\t    /// The classifier runs over a fully-defaulted return without panicking, and records exemptions with
571\t    /// real statutory reasons (never an empty string).
572\t    #[test]
573\t    fn classify_runs_and_every_exemption_carries_a_reason() {
574\t        let census = classify(&ReturnInputs::default());
575\t        assert!(
576\t            !census.exemptions.is_empty(),
577\t            \"a defaulted return still has exempt leaves (filing_status, itemize_election, presidential ×2, \\
578\t             taxpayer.blind, …)\"
579\t        );
580\t        for (_class, reason) in &census.exemptions {
581\t            assert!(
582\t                !reason.trim().is_empty(),
583\t                \"every exemption states WHY it is lawful\"
584\t            );
585\t        }
586\t    }
587\t}
588\t"}]},"uuid":"ea72ff47-5a89-44df-9725-768a74447db9","timestamp":"2026-08-01T04:04:49.698Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/classifier.rs","content":"//! ★ P9 §3.3 — THE CLASSIFIER. Makes the answered-ness invariant STRUCTURAL: every `bool` /
//! `Option<bool>` / defaulted-enum field reachable from [`ReturnInputs`] is classified — as a registry
//! DECLARATION (class A), or EXEMPTED with its §2 class and the statutory reason it is lawful.
//!
//! **The compile-time guarantee.** Every struct is destructured with **NO `..`**, so a newly-added field
//! is a `pattern does not mention field` COMPILE ERROR until a human edits this file. `#![deny(unused_variables)]`
//! then makes a named binding that *ignores* its field a hard error too (the `w2s: whatever` evasion r3 I-5
//! defeated the r3 wording with). So: **a new such field does not compile until a human LOOKS.**
//!
//! **★ The honest limit (r3 I-5), stated at its true strength.** The compiler forces \"a human must EDIT
//! the classifier\", NOT \"classified it correctly.\" The residual evasions — a `_`-prefixed binding, or
//! `let _ = x;` — are **grep-able REVIEW residue**, not compile errors. That is the whole guarantee, and it
//! is worth having: every recurrence of this class began with a field nobody looked at.
//!
//! **★ The `_` rule (r2 M-6).** `_` — and every `_`-prefixed binding — is **FORBIDDEN** on structs,
//! collections, and `bool` / `Option<bool>` / defaulted-enum leaves (they must recurse or be classified).
//! `_` is **PERMITTED** on other scalar leaves (`String`, `Usd`, `Date`, `Option<Date>`, `Option<String>`).
//!
//! This module does no tax arithmetic and is never on a compute path; [`classify`] returns a [`Census`] only
//! so a test can prove the registry declarations line up with [`FORM_QUESTIONS`].
#![deny(unused_variables)]

use crate::tax::questions::QuestionId;
use crate::tax::return_inputs::{
    Box12Entry, CharitableCarryItem, CharitableGift, Dependent, Form1099Div, Form1099G,
    Form1099Int, HouseholdHeader, Payments, Person, QbiInputs, ReturnInputs, Schedule1Inputs,
    ScheduleAInputs, ScheduleCInputs, W2,
};
use crate::tax::types::Carryforward;

/// The §2 class under which a defaulted input is lawful. (Class **A** is not here — it is a registry
/// DECLARATION, recorded via [`Census::declaration`]; class **D** is gone, deleted in step 9.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// **(B) BENEFIT CLAIM** — *New Colonial Ice*: the burden to CLAIM is the filer's, so `false`/absent is
    /// lawful, but the forgone benefit gets an advisory (§2.2).
    BenefitClaim,
    /// **(C) NO TAX DIRECTION** — neither asserts nor claims; a lawful silent default (§2.1).
    NoTaxDirection,
    /// serde-REQUIRED at import (the field has no `#[serde(default)]`): a TOML without it refuses to parse,
    /// so there is no default to launder (§2.1 / §2.8).
    SerdeRequired,
    /// The value IS data the record carries (a typed classification), not a defaulted answer for the filer.
    DataDerived,
    /// A defaulted enum WITH tax direction, tracked as an OPEN issue owned by a follow-up (§2.8).
    TrackedFollowup,
}

/// The census [`classify`] produces: which registry questions it declared, and every exemption with its
/// class and statutory reason. Used only by the tests — the load-bearing guarantee is the compile.
#[derive(Debug, Default)]
pub struct Census {
    pub declarations: Vec<QuestionId>,
    pub exemptions: Vec<(Class, &'static str)>,
}

impl Census {
    /// A class-(A) DECLARATION: the filer asserts it; its liveness and refusal live in [`FORM_QUESTIONS`].
    fn declaration(&mut self, _leaf: &Option<bool>, id: QuestionId) {
        self.declarations.push(id);
    }
    /// An exempted default — lawful, with its §2 class and the statutory reason it is lawful.
    fn exempt<T>(&mut self, _leaf: &T, class: Class, statutory_reason: &'static str) {
        self.exemptions.push((class, statutory_reason));
    }
}

/// ★ Destructure EVERY struct reachable from [`ReturnInputs`] with no `..`, classifying every
/// `bool`/`Option<bool>`/defaulted-enum leaf. Never called on a compute path — its whole job is to fail
/// COMPILATION when a new such field appears unclassified.
pub fn classify(ri: &ReturnInputs) -> Census {
    let mut c = Census::default();
    let ReturnInputs {
        // ★ §G-15 — the tax year is not a DECLARATION or a benefit claim; it is the scope those are
        // asked in. It carries no answered-ness of its own, so the classifier ignores it (naming it
        // explicitly rather than `..`, so the next field added is still a compile error).
        tax_year: _,
        filing_status,
        header,
        w2s,
        int_1099,
        div_1099,
        g_1099,
        schedule_c,
        schedule_a,
        itemize_election,
        mfs_spouse_itemizes,
        sch1,
        payments,
        capital_loss_carryforward_in,
        capital_loss_carryforward_in_provenance,
        charitable_carryover_in_provenance,
        amt_carryover_same_as_regular,
        amt_depreciation_same_as_regular,
        charitable_carryover_in,
        qbi,
        foreign_accounts,
        foreign_trust,
        fbar_filing_required,
        foreign_country_names: _, // String — scalar
        donations_had_restrictions,
        dual_status_alien,
        // §164(b)(7)(B)(iv) / Schedule 1-A Part I MAGI add-backs. `Option<Usd>` scalar leaves, so the
        // `_` rule permits binding them here — but they are NOT class-(B) forgone benefits like
        // `medical`: an unasked add-back understates MAGI and RAISES two deductions. Their `None` is
        // refused at the point of need (`SaltLimitation::line_5e`), not defaulted to zero.
        has_income_exclusion,
        excluded_puerto_rico_income: _,
        form_2555_line45: _,
        form_2555_line50: _,
        form_4563_line15: _,
    } = ri;
    c.exempt(
        filing_status,
        Class::SerdeRequired,
        \"filing_status has no #[serde(default)] — a TOML without it refuses to parse, so no default to \\
         launder (§2.1)\",
    );
    c.declaration(mfs_spouse_itemizes, QuestionId::MfsSpouseItemizes);
    c.declaration(foreign_accounts, QuestionId::ForeignAccounts);
    c.declaration(foreign_trust, QuestionId::ForeignTrust);
    // ★ §G-20a — provenance for the two benefit carryovers. Class (C): no print, no tax direction.
    // They exist so a ZERO can be told apart from an UNASKED, which is what makes an honest advisory
    // possible; the per-item provenance on `CharitableCarryItem` cannot speak for an EMPTY list.
    c.exempt(
        capital_loss_carryforward_in_provenance,
        Class::NoTaxDirection,
        \"§2.8: CarryProvenance (§1212(b) capital-loss carryover, §G-20a) — no print, no tax direction\",
    );
    c.exempt(
        charitable_carryover_in_provenance,
        Class::NoTaxDirection,
        \"§2.8: CarryProvenance (§170(d)(1) charitable carryover LIST, §G-20a) — no print, no tax direction\",
    );
    // ★ Schedule B 7a's unnumbered FBAR sub-question. NOT a declaration: no figure on the return
    // reads it (the printed chain writes the checkbox and nothing else), so its silence neither
    // asserts nor forgoes — class (C). Asked as `SkippableId::FbarFilingRequired`, and skipping it
    // fires `Advisory::FbarSubQuestionNotAnswered` quoting the form's Caution verbatim.
    c.exempt(
        fbar_filing_required,
        Class::NoTaxDirection,
        \"Schedule B line 7a's FBAR sub-question — no figure on the return reads it, and the penalty \\
         the form's Caution names attaches to NOT FILING FinCEN Form 114 (a FinCEN obligation \\
         independent of this box), not to leaving the box blank. Silence is lawful and prints a true \\
         blank; `Advisory::FbarSubQuestionNotAnswered` fires (§2.1)\",
    );
    // ★★ Form 8283 5a/5b/5c, asked as ONE return-level universal (§G-21). Class (B) HERE — offered
    // always, silence lawful — because the donations are in the LEDGER and liveness cannot see them.
    // The MANDATORY half lives in `screen_absolute`, which can: on an ITEMIZING year an
    // unanswered or `Some(true)` answer REFUSES.
    c.exempt(
        donations_had_restrictions,
        Class::BenefitClaim,
        \"Form 8283 5a/5b/5c (§G-21) — offered as a skippable so a filer who donated nothing is never \\
         blocked; on an itemizing year that claims the §170 deduction `screen_absolute` makes it \\
          mandatory (§2.2)\",
    );
    c.declaration(dual_status_alien, QuestionId::DualStatusAlien);
    c.declaration(has_income_exclusion, QuestionId::HasIncomeExclusion);
    c.declaration(
        amt_carryover_same_as_regular,
        QuestionId::AmtCarryoverSameAsRegular,
    );
    c.declaration(
        amt_depreciation_same_as_regular,
        QuestionId::AmtDepreciationSameAsRegular,
    );
    c.exempt(
        itemize_election,
        Class::NoTaxDirection,
        \"§2.8: `Auto` takes max(standard, itemized) — an OPTIMIZATION that cannot lose money, not an \\
         assertion and not a forgone benefit; §63(e) `ForceItemize` is opt-in\",
    );
    classify_header(&mut c, header);
    for w in w2s {
        classify_w2(&mut c, w);
    }
    for i in int_1099 {
        classify_1099int(&mut c, i);
    }
    for d in div_1099 {
        classify_1099div(&mut c, d);
    }
    for g in g_1099 {
        classify_1099g(&mut c, g);
    }
    if let Some(sc) = schedule_c {
        classify_schedule_c(&mut c, sc);
    }
    if let Some(a) = schedule_a {
        classify_schedule_a(&mut c, a);
    }
    classify_schedule1(&mut c, sch1);
    classify_payments(&mut c, payments);
    classify_carryforward(&mut c, capital_loss_carryforward_in);
    for item in charitable_carryover_in {
        classify_charitable_carry(&mut c, item);
    }
    classify_qbi(&mut c, qbi);
    c
}

fn classify_header(c: &mut Census, h: &HouseholdHeader) {
    let HouseholdHeader {
        taxpayer,
        spouse,
        address_street: _,
        address_city: _,
        address_state: _,
        address_zip: _,
        dependents,
        can_be_claimed_as_dependent_taxpayer,
        can_be_claimed_as_dependent_spouse,
        spouse_had_no_income,
        spouse_not_filing_a_return,
        presidential_fund_taxpayer,
        presidential_fund_spouse,
        taxpayer_died_during_year,
        spouse_died_during_year,
        ip_pin: _, // Option<String> — scalar
    } = h;
    c.declaration(
        can_be_claimed_as_dependent_taxpayer,
        QuestionId::DependentTaxpayer,
    );
    c.declaration(
        can_be_claimed_as_dependent_spouse,
        QuestionId::DependentSpouse,
    );
    c.exempt(
        presidential_fund_taxpayer,
        Class::NoTaxDirection,
        \"§2.1: 1040 presidential-fund box — §6096 is a fund DESIGNATION, not a tax liability\",
    );
    c.exempt(
        presidential_fund_spouse,
        Class::NoTaxDirection,
        \"§2.1: 1040 presidential-fund box (spouse) — §6096, no tax direction\",
    );
    // ★★ §G-20 — i1040gi's two uncaptured MFS conditions for the spouse's §63(f) boxes. BENEFIT
    // CLAIMS (New Colonial Ice): silence FORGOES the boxes and never grants them, so `false`/absent is
    // lawful and the forgone benefit fires `Mfs63fSpouseBoxesForgone`.
    c.exempt(
        spouse_had_no_income,
        Class::BenefitClaim,
        \"§63(f)/i1040gi MFS condition (spouse had no income) — silence forgoes the spouse's aged/blind \\
         boxes rather than granting them; `Advisory::Mfs63fSpouseBoxesForgone` names the cost (§2.2)\",
    );
    c.exempt(
        spouse_not_filing_a_return,
        Class::BenefitClaim,
        \"§63(f)/i1040gi MFS condition (spouse isn't filing a return) — same reasoning (§2.2)\",
    );
    // §G-9: the §63(f) death carve-out. i1040gi states it twice — \"Death of a taxpayer\" and \"Death of
    // spouse\" — so each is its own leaf. ★ BENEFIT CLAIMS, not declarations: silence FORGOES the
    // age-65 addition (`is_aged`'s `(None, None)` arm returns `false`), so by the sharp test it is
    // class (B), asked as `SkippableId::{Taxpayer,Spouse}DiedDuringYear`. They were briefly class-(A)
    // refusals, and the taxpayer one was `live: |_| true` — it blocked every return for a fail-safe
    // the compute layer already had.
    c.exempt(
        taxpayer_died_during_year,
        Class::BenefitClaim,
        \"§63(f)/§G-9 death carve-out (taxpayer) — New Colonial Ice: silence forgoes the age-65 \\
         addition rather than granting it on an unresolved carve-out, so `false`/absent is lawful; \\
         `Advisory::AgedBoxForfeitedDeathUnanswered` fires when a qualifying DOB makes the skip cost \\
         money (§2.2)\",
    );
    c.exempt(
        spouse_died_during_year,
        Class::BenefitClaim,
        \"§63(f)/§G-9 death carve-out (spouse) — same reasoning; the box is counted exactly when \\
         `spouse_63f_boxes_count` says so (MFJ, or a qualifying MFS), which is the same predicate \\
         that gates the prompt (§2.2)\",
    );
    classify_person(c, taxpayer);
    if let Some(sp) = spouse {
        classify_person(c, sp);
    }
    for d in dependents {
        classify_dependent(c, d);
    }
}

fn classify_person(c: &mut Census, p: &Person) {
    let Person {
        first_name: _,
        last_name: _,
        ssn: _,
        date_of_birth: _, // Option<Date> — scalar
        // Option<Date> — scalar. Its answered-ness is carried by the HEADER gates
        // `taxpayer_died_during_year` / `spouse_died_during_year`, declared in `classify_header`.
        date_of_death: _,
        blind,
        occupation: _,
    } = p;
    c.exempt(
        blind,
        Class::BenefitClaim,
        \"§63(f) blindness — New Colonial Ice: the burden to CLAIM is the filer's, so `false`/absent is \\
         lawful; the forgone benefit fires `BlindBoxForfeitedNotDeclared` (§2.2)\",
    );
}

fn classify_dependent(_c: &mut Census, d: &Dependent) {
    // No classifiable leaves — but destructured with no `..` so a future bool here is a compile error.
    let Dependent {
        name: _,
        ssn: _,
        relationship: _,
        date_of_birth: _,
    } = d;
}

fn classify_w2(c: &mut Census, w: &W2) {
    let W2 {
        owner,
        employer: _,
        box1_wages: _,
        box2_fed_withheld: _,
        box3_ss_wages: _,
        box4_ss_withheld: _,
        box5_medicare_wages: _,
        box6_medicare_withheld: _,
        box7_ss_tips: _,
        box17_state_tax_withheld: _,
        box19_local_tax: _,
        box12,
        box8_allocated_tips: _,
        box10_dependent_care: _,
    } = w;
    c.exempt(
        owner,
        Class::SerdeRequired,
        \"§2.8: W2.owner has no #[serde(default)], so the TOML import REQUIRES it; the enum's #[default] \\
         Taxpayer reaches only Rust-side fixtures\",
    );
    for e in box12 {
        classify_box12(c, e);
    }
}

fn classify_box12(_c: &mut Census, e: &Box12Entry) {
    let Box12Entry {
        code: _, // String — scalar
        amount: _,
    } = e;
}

fn classify_1099int(_c: &mut Census, i: &Form1099Int) {
    let Form1099Int {
        payer: _,
        box1_interest: _,
        box2_early_withdrawal_penalty: _,
        box3_treasury_interest: _,
        box4_fed_withheld: _,
        box6_foreign_tax: _,
        box8_tax_exempt_interest: _,
        box9_private_activity_bond_amt: _,
    } = i;
}

fn classify_1099div(_c: &mut Census, d: &Form1099Div) {
    let Form1099Div {
        payer: _,
        box1a_ordinary: _,
        box1b_qualified: _,
        box2a_capgain_distr: _,
        box2b_unrecap_1250: _,
        box2c_section_1202: _,
        box2d_collectibles_28: _,
        box4_fed_withheld: _,
        box5_section_199a: _,
        box7_foreign_tax: _,
        box12_exempt_interest_dividends: _,
        box13_private_activity_amt: _,
    } = d;
}

fn classify_1099g(_c: &mut Census, g: &Form1099G) {
    let Form1099G {
        payer: _,
        box1_unemployment: _,
        box4_fed_withheld: _,
    } = g;
}

fn classify_schedule_c(c: &mut Census, sc: &ScheduleCInputs) {
    let ScheduleCInputs {
        owner,
        business_description: _,
        naics_code: _,
        accounting_method,
        expenses: _,
        payments_requiring_1099,
        will_file_required_1099,
    } = sc;
    c.exempt(
        owner,
        Class::SerdeRequired,
        \"§2.8: ScheduleCInputs.owner has no #[serde(default)] — serde-required at import\",
    );
    c.exempt(
        accounting_method,
        Class::TrackedFollowup,
        \"§2.8: Cash default; `accrual` is accepted, unmodeled and UNREFUSED and flips the printed Sch C \\
         line F — a known open Important, filed → P8\",
    );
    // ★ Schedule C lines I and J — the Form-1099 compliance pair. Class (B): asked as
    // `SkippableId::ScheduleC1099{Required,Filed}`, silence lawful. Not `declaration(..)` because no
    // figure on the return reads them and the form prints no Caution — but NOT plain \"no tax
    // direction\" either, because §6721/§6722 exposure is real, which is what the skip advisory names.
    //
    // ★★ PRE-MERGE M8 — these are RECORDED, not bound with `_`. This module's own r2 M-6 rule is that
    // an `Option<bool>` leaf never gets a bare `_`: the whole point of the census is to distinguish
    // \"this encodes no decision\" from \"we forgot it\", and `_` erases exactly that distinction. Both
    // were silently under-reported as answered-ness decisions and nothing redded. The four other
    // class-(B) leaves this branch added were all recorded via `exempt`; these two were missed.
    c.exempt(
        payments_requiring_1099,
        Class::BenefitClaim,
        \"Schedule C line I — asked as `SkippableId::ScheduleC1099Required`; silence forgoes nothing on \\
         the return itself, and the §6721/§6722 exposure is named by the skip advisory (§2.2)\",
    );
    c.exempt(
        will_file_required_1099,
        Class::BenefitClaim,
        \"Schedule C line J — asked as `SkippableId::ScheduleC1099Filed`, and gated on line I being \\
         Yes; an answer to J without a Yes on I is not a mark the form has a place for (§2.2)\",
    );
}

fn classify_schedule_a(c: &mut Census, a: &ScheduleAInputs) {
    let ScheduleAInputs {
        medical: _,
        salt_use_sales_tax,
        salt_sales_tax_amount: _,
        salt_state_estimated_payments: _,
        salt_prior_year_balance_paid: _,
        salt_real_estate: _,
        salt_personal_property: _,
        mortgage_interest_1098: _,
        mortgage_all_used_to_buy_build_improve,
        mortgage_dwelling_is_amt_qualified,
        charitable,
    } = a;
    c.exempt(
        salt_use_sales_tax,
        Class::BenefitClaim,
        \"§164(b)(5) sales-tax election — New Colonial Ice: lawful unasked; `SalesTaxElectionNotAsked` \\
         advises when a Schedule A exists (§2.2)\",
    );
    c.declaration(
        mortgage_all_used_to_buy_build_improve,
        QuestionId::MortgageAllUsedToBuyBuildImprove,
    );
    c.declaration(
        mortgage_dwelling_is_amt_qualified,
        QuestionId::AmtQualifiedDwelling,
    );
    for g in charitable {
        classify_charitable_gift(c, g);
    }
}

fn classify_charitable_gift(c: &mut Census, g: &CharitableGift) {
    let CharitableGift { class, amount: _ } = g;
    c.exempt(
        class,
        Class::DataDerived,
        \"the gift's CONTRIBUTION CLASS is data (which property was given), not a defaulted answer for the \\
         filer\",
    );
}

fn classify_schedule1(c: &mut Census, s: &Schedule1Inputs) {
    let Schedule1Inputs {
        state_refund_taxable: _,
        student_loan_interest_paid: _,
        ira_deduction_claimed: _,
        hsa_activity,
    } = s;
    c.declaration(hsa_activity, QuestionId::HsaActivity);
}

fn classify_payments(_c: &mut Census, p: &Payments) {
    let Payments {
        estimated_tax_payments: _,
        extension_payment: _,
        other_withholding: _,
    } = p;
}

fn classify_carryforward(_c: &mut Census, cf: &Carryforward) {
    // FROZEN struct — destructuring it READS it, modifies nothing (§3.3). No classifiable leaves today; the
    // guarantee is about the bool added tomorrow.
    let Carryforward { short: _, long: _ } = cf;
}

fn classify_charitable_carry(c: &mut Census, item: &CharitableCarryItem) {
    let CharitableCarryItem {
        class,
        amount: _,
        origin_year: _,
        provenance,
    } = item;
    c.exempt(
        class,
        Class::DataDerived,
        \"the carryover item's contribution class is data, not a defaulted answer\",
    );
    c.exempt(
        provenance,
        Class::NoTaxDirection,
        \"§2.8: CarryProvenance — no print, no tax direction\",
    );
}

fn classify_qbi(c: &mut Census, q: &QbiInputs) {
    let QbiInputs {
        reit_ptp_carryforward_in: _,
        reit_ptp_carryforward_in_provenance,
        qbi_carryforward_in: _,
        qbi_carryforward_in_provenance,
    } = q;
    c.exempt(
        reit_ptp_carryforward_in_provenance,
        Class::NoTaxDirection,
        \"§2.8: CarryProvenance — no print, no tax direction\",
    );
    c.exempt(
        qbi_carryforward_in_provenance,
        Class::NoTaxDirection,
        \"§2.8: CarryProvenance (Form 8995 line 3) — no print, no tax direction\",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★ §3.3 / §4 — the classifier's registry DECLARATIONS line up EXACTLY with [`FORM_QUESTIONS`]: every
    /// registry `QuestionId` is declared once, and nothing is declared that is not a registry question. A
    /// wrong or dropped `declaration(..)` (a mis-classification the COMPILER cannot catch — the honest
    /// limit) fails here. Exercised on a fixture with a Schedule A so the mortgage declaration runs.
    #[test]
    fn every_registry_question_is_declared_exactly_once() {
        let mut ri = ReturnInputs {
            schedule_a: Some(ScheduleAInputs::default()),
            ..Default::default()
        };
        // A spouse Person exercises the spouse branch too (its bools are on the header, always destructured,
        // but this keeps the census exercise faithful to a populated return).
        ri.header.spouse = Some(Person::default());
        let census = classify(&ri);

        for id in QuestionId::ALL {
            assert_eq!(
                census.declarations.iter().filter(|d| *d == id).count(),
                1,
                \"classifier must declare {id:?} exactly once (registry ⇔ classifier, §3.3)\"
            );
        }
        assert_eq!(
            census.declarations.len(),
            QuestionId::ALL.len(),
            \"the classifier declares EXACTLY the registry questions — no more, no less\"
        );
    }

    /// The classifier runs over a fully-defaulted return without panicking, and records exemptions with
    /// real statutory reasons (never an empty string).
    #[test]
    fn classify_runs_and_every_exemption_carries_a_reason() {
        let census = classify(&ReturnInputs::default());
        assert!(
            !census.exemptions.is_empty(),
            \"a defaulted return still has exempt leaves (filing_status, itemize_election, presidential ×2, \\
             taxpayer.blind, …)\"
        );
        for (_class, reason) in &census.exemptions {
            assert!(
                !reason.trim().is_empty(),
                \"every exemption states WHY it is lawful\"
            );
        }
    }
}
","numLines":588,"startLine":1,"totalLines":588}},"sourceToolAssistantUUID":"da5c20d7-58de-4d33-9e35-717214b8c43c","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"ea72ff47-5a89-44df-9725-768a74447db9","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbND9jhYcugr867h1tX8","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAIShgQKiAEIEBgCKkC/MyuZ+IxfoLmYFZ611QaOjoG9r41yiMuHvIv4ivjuHkG6mSbdYifSXKsgwV14WLkeYErbAs64/tSVxhE/NqMzMg5jbGF1ZGUtZmFibGUtNTgBQgh0aGlua2luZ1okZTJmYmFjOTAtMDM4ZC00MjYzLWFjMDUtZWUxNTVlMGNjZTg2Egye7rc7GLR9TiXBU+caDDQyyjkSJlBhl50jNyIwmdVuWbCPpD/6xrq9AHEFvAZH4wQkIW1Rjaswq256rUGHbCt4mpM1EyOSyByDKyS0KqoCN5oU9UvnmveOPI1g03j2dqz74tGy27b7GAwmqwQbQx9UFOkVrj9Z4XrpWhQadl64gDG0rYEJzfvRr/ZGZunpTtOisVriprkKbMqWhC0eA8qbBlXQD7DQBhwn9Uu2MJpwNN5TJEJpu8rfYknSlZi6+fWIrdCOLlwaoCm7acPWUb+wF6wjug0z5DiEfxmPaah2PA0U50pcJOoMqEtwYncxnJtQ8aoJ0C1Fl2CQ6BCGmKPnvqnXLUfF1kaT8tHnqiAK1m/cbNcruyypLwy76xrN5bdU800NRVQgG65UVeF11Apk5IJoSD+Zr50LaDYwAgkvLZ3nmSyKhkaDa7uFfVWPGVWKLDufEppW5o8sApsxAZl/BzcVWxMlB1LBkFdMdVM761w31SSSXMb3kxgB"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":13425,"cache_read_input_tokens":55788,"cache_creation":{"ephemeral_5m_input_tokens":13425,"ephemeral_1h_input_tokens":0},"output_tokens":8,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbND7zH8wmHL8e8xKWKm","attributionAgent":"general-purpose","type":"assistant","uuid":"2e305983-e32e-4a44-9bec-604f914638a2","timestamp":"2026-08-01T04:04:52.922Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"2e305983-e32e-4a44-9bec-604f914638a2","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbND9jhYcugr867h1tX8","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01Qd3mWoQ6qfnzZSgyGPe1ig","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/printed.rs","offset":1,"limit":160},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":13425,"cache_read_input_tokens":55788,"cache_creation":{"ephemeral_5m_input_tokens":13425,"ephemeral_1h_input_tokens":0},"output_tokens":8,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbND7zH8wmHL8e8xKWKm","attributionAgent":"general-purpose","type":"assistant","uuid":"88c62020-9337-46af-8494-96ebe17773a5","timestamp":"2026-08-01T04:04:54.015Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"88c62020-9337-46af-8494-96ebe17773a5","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01Qd3mWoQ6qfnzZSgyGPe1ig","type":"tool_result","content":"1\t//! Full-return v1 **printed line chains** for the 1040's numbered schedules (P6 / SPEC §3.1).
2\t//!
3\t//! Every form btctax files needs a *printed* chain distinct from the exact-cents computation:
4\t//!
5\t//! - each printed line is `round_dollar`ed **at the line**, and
6\t//! - each printed **total sums the already-rounded lines above it**, so the filed form cross-foots.
7\t//!
8\t//! That is deliberately NOT `round_dollar(exact_total)` — with two `.50` components the two differ by
9\t//! a dollar, and the cross-footing one is what a human re-adding the column gets (SPEC §10 KAT-9).
10\t//!
11\t//! **The chains compose.** A schedule that carries a figure from another form takes that form's
12\t//! **printed** line, never the exact-cents computation behind it — Schedule 2 line 11 is Form 8959's
13\t//! printed line 18, not `round_dollar(additional_medicare.additional_medicare_tax)`. Otherwise the
14\t//! attached form and the schedule that references it would disagree by a dollar, and the return would
15\t//! not tie out. This is why the builders below take the upstream `*Lines` structs as arguments.
16\t//!
17\t//! **`btctax-forms` does no tax arithmetic**: it transcribes these structs cell-for-cell. A second,
18\t//! independent derivation in the filler is exactly how a filed PDF comes to disagree with the tax it
19\t//! reports, and no core KAT would catch it.
20\t
21\tuse crate::conventions::{round_dollar, Usd};
22\tuse crate::tax::other_taxes::{Form8959Lines, Form8960Lines};
23\tuse crate::tax::return_1040::{AbsoluteReturn, MEDICAL_FLOOR_RATE};
24\tuse crate::tax::types::FilingStatus;
25\t
26\t// ── Form 8949 — the printed rows (SPEC §3.1 / ARCH-P6.3a D2) ────────────────────────────────────
27\t
28\t/// One PRINTED Form 8949 row: columns (d), (e) and (h) as they appear on the filed page.
29\t///
30\t/// ★ **Column (h) is DERIVED, never independently rounded.** The form's own column-(h) header says
31\t/// \"Subtract column (e) from column (d) and combine the result with column (g)\" — an instruction about
32\t/// the PRINTED cells. Rounding the exact gain independently produces rows that visibly contradict their
33\t/// own subtraction (proceeds 100.49 → 100, basis 0.50 → 1, exact gain 99.99 → 100, but 100 − 1 = 99),
34\t/// and it drifts Σh away from Σd − Σe, which would then break Schedule D's Part I, whose columns carry
35\t/// the same subtract-and-combine header. Deriving h makes both identities hold exactly.
36\t#[derive(Debug, Clone, PartialEq, Eq)]
37\tpub struct Printed8949Row {
38\t    /// (a) description — the exact BTC amount, 8dp (never rounded: it is a quantity, not money).
39\t    pub description: String,
40\t    /// (b) date acquired.
41\t    pub date_acquired: crate::conventions::TaxDate,
42\t    /// (c) date sold.
43\t    pub date_sold: crate::conventions::TaxDate,
44\t    /// (d) proceeds — `round_dollar` at the cell.
45\t    pub proceeds_d: Usd,
46\t    /// (e) cost basis — `round_dollar` at the cell.
47\t    pub cost_e: Usd,
48\t    /// (h) gain/loss = **printed (d) − printed (e)** (column (g) is always blank in v1).
49\t    pub gain_h: Usd,
50\t}
51\t
52\t/// A part's totals row — the sum of the PRINTED rows above it (never a re-rounding of the exact sum).
53\t#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
54\tpub struct Printed8949Totals {
55\t    pub proceeds_d: Usd,
56\t    pub cost_e: Usd,
57\t    pub gain_h: Usd,
58\t}
59\t
60\t/// The printed Form 8949: Part I (short-term) and Part II (long-term), each with its rows and its
61\t/// totals row. The conservative \"not reported\" box is year-aware (C/F pre-TY2025; the digital-asset
62\t/// I/L from TY2025). These totals ARE Schedule D lines 3 and 10 — the schedule's own text defines
63\t/// them as \"Totals for all transactions reported on Form(s) 8949 with Box C/F checked\" pre-2025, and
64\t/// \"with Box C or Box I checked\" (line 3) / \"Box F or Box L checked\" (line 10) on the 2025 revision.
65\t#[derive(Debug, Clone, PartialEq, Eq)]
66\tpub struct Printed8949 {
67\t    pub short_term: Vec<Printed8949Row>,
68\t    pub long_term: Vec<Printed8949Row>,
69\t    pub st_totals: Printed8949Totals,
70\t    pub lt_totals: Printed8949Totals,
71\t}
72\t
73\t/// Derive the printed Form 8949 from the ledger's rows. `None` when the year has no disposals — a
74\t/// carryover/distribution-only Schedule D files with lines 3/10 blank and NO 8949 attached.
75\tpub fn form_8949_printed(rows: &[crate::forms::Form8949Row]) -> Option<Printed8949> {
76\t    use crate::forms::Form8949Part;
77\t    if rows.is_empty() {
78\t        return None;
79\t    }
80\t    let printed = |r: &crate::forms::Form8949Row| {
81\t        let proceeds_d = round_dollar(r.proceeds);
82\t        let cost_e = round_dollar(r.cost_basis);
83\t        Printed8949Row {
84\t            description: r.description.clone(),
85\t            date_acquired: r.date_acquired,
86\t            date_sold: r.date_sold,
87\t            proceeds_d,
88\t            cost_e,
89\t            // ★ derived from the PRINTED cells, not from `r.gain`
90\t            gain_h: proceeds_d - cost_e,
91\t        }
92\t    };
93\t    let total = |rs: &[Printed8949Row]| Printed8949Totals {
94\t        proceeds_d: rs.iter().map(|r| r.proceeds_d).sum(),
95\t        cost_e: rs.iter().map(|r| r.cost_e).sum(),
96\t        gain_h: rs.iter().map(|r| r.gain_h).sum(),
97\t    };
98\t
99\t    let short_term: Vec<_> = rows
100\t        .iter()
101\t        .filter(|r| r.part == Form8949Part::ShortTerm)
102\t        .map(printed)
103\t        .collect();
104\t    let long_term: Vec<_> = rows
105\t        .iter()
106\t        .filter(|r| r.part == Form8949Part::LongTerm)
107\t        .map(printed)
108\t        .collect();
109\t
110\t    Some(Printed8949 {
111\t        st_totals: total(&short_term),
112\t        lt_totals: total(&long_term),
113\t        short_term,
114\t        long_term,
115\t    })
116\t}
117\t
118\t// ── Form 8283 — the printed rows (ARCH-P6.3a D7) ────────────────────────────────────────────────
119\t
120\t/// The PRINTED Form 8283 rows: the same rows the crypto slice emits, with the three money columns
121\t/// (`cost_basis`, `fmv`, `claimed_deduction`) rounded to whole dollars for the filed packet.
122\t///
123\t/// A **newtype**, not a bare `Vec<Form8283Row>`, on purpose: the slice's rows carry CENTS, and the two
124\t/// must not be interchangeable. Handing a cents row to the full-return filler would put an 8283 under a
125\t/// different rounding regime from the Schedule A it is attached to — which is the whole defect this
126\t/// phase exists to close.
127\t///
128\t/// **Unlike Schedule D ← 8949, Schedule A line 12 does NOT re-derive from this.** The rule is: a line
129\t/// whose form text CITES another form as its source composes on that form's printed line; a line that
130\t/// merely REQUIRES an attachment does not. Schedule A L12's text is \"…You must attach Form 8283 if over
131\t/// $500\" — an attachment requirement. Form 8283 has no grand-total line for L12 to equal, and the
132\t/// §170(b) ceilings legitimately make L12 *smaller* than the sum of the 8283's per-donation amounts (the
133\t/// excess becomes carryover). Forcing them equal would be wrong.
134\t#[derive(Debug, Clone, PartialEq, Eq)]
135\tpub struct Printed8283Rows {
136\t    rows: Vec<crate::forms::Form8283Row>,
137\t    /// ★★★ §G-21 — the filer's answer to Form 8283 Section B lines **5a / 5b / 5c**, asked as one
138\t    /// return-level universal. `Some(false)` = \"no donation had strings attached\" ⇒ all three boxes
139\t    /// print **No**. `None` = never answered ⇒ all three stay BLANK.
140\t    ///
141\t    /// ★ `Some(true)` never reaches here: `screen_absolute` REFUSES the year on a Yes, since
142\t    /// the §170 deduction btctax computed at full FMV is then too large. The field is `Option<bool>`
143\t    /// rather than `bool` so \"not asked\" and \"answered no\" stay different marks on the page — the
144\t    /// distinction `3b22ca1` had to fix on Schedule B the hard way.
145\t    no_donation_restrictions: Option<bool>,
146\t}
147\t
148\timpl Printed8283Rows {
149\t    /// The rows, whole-dollar.
150\t    pub fn rows(&self) -> &[crate::forms::Form8283Row] {
151\t        &self.rows
152\t    }
153\t    /// The filer's own answer to lines 5a/5b/5c — `Some(false)` ⇒ print all three as **No**.
154\t    pub fn restrictions_answer(&self) -> Option<bool> {
155\t        self.no_donation_restrictions
156\t    }
157\t}
158\t
159\t/// Round the ledger's Form 8283 rows for the filed packet. `None` when there are no donation rows.
160\t///"}]},"uuid":"70d91d7d-a6cb-4db8-8963-f0f27aaae4c2","timestamp":"2026-08-01T04:04:54.023Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/printed.rs","content":"//! Full-return v1 **printed line chains** for the 1040's numbered schedules (P6 / SPEC §3.1).
//!
//! Every form btctax files needs a *printed* chain distinct from the exact-cents computation:
//!
//! - each printed line is `round_dollar`ed **at the line**, and
//! - each printed **total sums the already-rounded lines above it**, so the filed form cross-foots.
//!
//! That is deliberately NOT `round_dollar(exact_total)` — with two `.50` components the two differ by
//! a dollar, and the cross-footing one is what a human re-adding the column gets (SPEC §10 KAT-9).
//!
//! **The chains compose.** A schedule that carries a figure from another form takes that form's
//! **printed** line, never the exact-cents computation behind it — Schedule 2 line 11 is Form 8959's
//! printed line 18, not `round_dollar(additional_medicare.additional_medicare_tax)`. Otherwise the
//! attached form and the schedule that references it would disagree by a dollar, and the return would
//! not tie out. This is why the builders below take the upstream `*Lines` structs as arguments.
//!
//! **`btctax-forms` does no tax arithmetic**: it transcribes these structs cell-for-cell. A second,
//! independent derivation in the filler is exactly how a filed PDF comes to disagree with the tax it
//! reports, and no core KAT would catch it.

use crate::conventions::{round_dollar, Usd};
use crate::tax::other_taxes::{Form8959Lines, Form8960Lines};
use crate::tax::return_1040::{AbsoluteReturn, MEDICAL_FLOOR_RATE};
use crate::tax::types::FilingStatus;

// ── Form 8949 — the printed rows (SPEC §3.1 / ARCH-P6.3a D2) ────────────────────────────────────

/// One PRINTED Form 8949 row: columns (d), (e) and (h) as they appear on the filed page.
///
/// ★ **Column (h) is DERIVED, never independently rounded.** The form's own column-(h) header says
/// \"Subtract column (e) from column (d) and combine the result with column (g)\" — an instruction about
/// the PRINTED cells. Rounding the exact gain independently produces rows that visibly contradict their
/// own subtraction (proceeds 100.49 → 100, basis 0.50 → 1, exact gain 99.99 → 100, but 100 − 1 = 99),
/// and it drifts Σh away from Σd − Σe, which would then break Schedule D's Part I, whose columns carry
/// the same subtract-and-combine header. Deriving h makes both identities hold exactly.
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
/// \"with Box C or Box I checked\" (line 3) / \"Box F or Box L checked\" (line 10) on the 2025 revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printed8949 {
    pub short_term: Vec<Printed8949Row>,
    pub long_term: Vec<Printed8949Row>,
    pub st_totals: Printed8949Totals,
    pub lt_totals: Printed8949Totals,
}

/// Derive the printed Form 8949 from the ledger's rows. `None` when the year has no disposals — a
/// carryover/distribution-only Schedule D files with lines 3/10 blank and NO 8949 attached.
pub fn form_8949_printed(rows: &[crate::forms::Form8949Row]) -> Option<Printed8949> {
    use crate::forms::Form8949Part;
    if rows.is_empty() {
        return None;
    }
    let printed = |r: &crate::forms::Form8949Row| {
        let proceeds_d = round_dollar(r.proceeds);
        let cost_e = round_dollar(r.cost_basis);
        Printed8949Row {
            description: r.description.clone(),
            date_acquired: r.date_acquired,
            date_sold: r.date_sold,
            proceeds_d,
            cost_e,
            // ★ derived from the PRINTED cells, not from `r.gain`
            gain_h: proceeds_d - cost_e,
        }
    };
    let total = |rs: &[Printed8949Row]| Printed8949Totals {
        proceeds_d: rs.iter().map(|r| r.proceeds_d).sum(),
        cost_e: rs.iter().map(|r| r.cost_e).sum(),
        gain_h: rs.iter().map(|r| r.gain_h).sum(),
    };

    let short_term: Vec<_> = rows
        .iter()
        .filter(|r| r.part == Form8949Part::ShortTerm)
        .map(printed)
        .collect();
    let long_term: Vec<_> = rows
        .iter()
        .filter(|r| r.part == Form8949Part::LongTerm)
        .map(printed)
        .collect();

    Some(Printed8949 {
        st_totals: total(&short_term),
        lt_totals: total(&long_term),
        short_term,
        long_term,
    })
}

// ── Form 8283 — the printed rows (ARCH-P6.3a D7) ────────────────────────────────────────────────

/// The PRINTED Form 8283 rows: the same rows the crypto slice emits, with the three money columns
/// (`cost_basis`, `fmv`, `claimed_deduction`) rounded to whole dollars for the filed packet.
///
/// A **newtype**, not a bare `Vec<Form8283Row>`, on purpose: the slice's rows carry CENTS, and the two
/// must not be interchangeable. Handing a cents row to the full-return filler would put an 8283 under a
/// different rounding regime from the Schedule A it is attached to — which is the whole defect this
/// phase exists to close.
///
/// **Unlike Schedule D ← 8949, Schedule A line 12 does NOT re-derive from this.** The rule is: a line
/// whose form text CITES another form as its source composes on that form's printed line; a line that
/// merely REQUIRES an attachment does not. Schedule A L12's text is \"…You must attach Form 8283 if over
/// $500\" — an attachment requirement. Form 8283 has no grand-total line for L12 to equal, and the
/// §170(b) ceilings legitimately make L12 *smaller* than the sum of the 8283's per-donation amounts (the
/// excess becomes carryover). Forcing them equal would be wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printed8283Rows {
    rows: Vec<crate::forms::Form8283Row>,
    /// ★★★ §G-21 — the filer's answer to Form 8283 Section B lines **5a / 5b / 5c**, asked as one
    /// return-level universal. `Some(false)` = \"no donation had strings attached\" ⇒ all three boxes
    /// print **No**. `None` = never answered ⇒ all three stay BLANK.
    ///
    /// ★ `Some(true)` never reaches here: `screen_absolute` REFUSES the year on a Yes, since
    /// the §170 deduction btctax computed at full FMV is then too large. The field is `Option<bool>`
    /// rather than `bool` so \"not asked\" and \"answered no\" stay different marks on the page — the
    /// distinction `3b22ca1` had to fix on Schedule B the hard way.
    no_donation_restrictions: Option<bool>,
}

impl Printed8283Rows {
    /// The rows, whole-dollar.
    pub fn rows(&self) -> &[crate::forms::Form8283Row] {
        &self.rows
    }
    /// The filer's own answer to lines 5a/5b/5c — `Some(false)` ⇒ print all three as **No**.
    pub fn restrictions_answer(&self) -> Option<bool> {
        self.no_donation_restrictions
    }
}

/// Round the ledger's Form 8283 rows for the filed packet. `None` when there are no donation rows.
///","numLines":160,"startLine":1,"totalLines":2992}},"sourceToolAssistantUUID":"88c62020-9337-46af-8494-96ebe17773a5","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"70d91d7d-a6cb-4db8-8963-f0f27aaae4c2","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbND9jhYcugr867h1tX8","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_018sMacvPLgqvSX7SZ4APaSN","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/crates/btctax-forms/src/cells.rs"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":13425,"cache_read_input_tokens":55788,"output_tokens":317,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":13425},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":317,"cache_read_input_tokens":55788,"cache_creation_input_tokens":13425,"cache_creation":{"ephemeral_5m_input_tokens":13425,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbND7zH8wmHL8e8xKWKm","attributionAgent":"general-purpose","type":"assistant","uuid":"4884b045-1f65-4eab-9602-8fd3b719ad9e","timestamp":"2026-08-01T04:04:54.542Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"4884b045-1f65-4eab-9602-8fd3b719ad9e","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_018sMacvPLgqvSX7SZ4APaSN","type":"tool_result","content":"1\t//! Money-cell emission shared by the flat forms (Schedule SE / Form 1040 / Form 8283).
2\t//!
3\t//! A [`MoneyCell`] is either a single field (2024/2025) carrying the whole `fmt_money` string, or a
4\t//! **dollars+cents [`MoneyPair`]** (the 2017 forms). For a pair the geometric oracle treats the two
5\t//! widgets as ONE logical cell at the **dollars-field** geometry: the dollars field is the
6\t//! column-x-/descent-checked [`FlatPlacement::cell`]/[`FlatPlacement::col_only`]; the cents field
7\t//! rides along as an authorized-but-geometry-exempt [`FlatPlacement::free`] write. A map that swaps
8\t//! the two members writes the whole-dollars value into the narrow cents widget → its center-x leaves
9\t//! the dollars column cluster → the read-back FAILS CLOSED.
10\t
11\tuse crate::error::FormsError;
12\tuse crate::fmt_money;
13\tuse crate::map::{IdentityCells, MoneyCell};
14\tuse crate::pdf::{Field, FieldValue};
15\tuse crate::verify::FlatPlacement;
16\tuse btctax_core::tax::packet::Ssn;
17\tuse btctax_core::Usd;
18\tuse rust_decimal_macros::dec;
19\t
20\t/// 0-based page index a field lives on (page 2 = the 2017 §B long Schedule SE / the Rev. 12-2014
21\t/// Section B tables; page 1 otherwise). Derived from the fully-qualified name, exactly as the
22\t/// verifier's own `page_of` — so a placement's page always agrees with its widget.
23\tpub fn page_of(fqn: &str) -> usize {
24\t    if fqn.contains(\"Page2\") {
25\t        1
26\t    } else {
27\t        0
28\t    }
29\t}
30\t
31\t/// Split a money amount into (whole-dollars, 2-digit-zero-padded-cents) — the REAL formatter the
32\t/// dollars+cents pairs need (`fmt_money` is the raw `Decimal` Display, which pads no cents). The value
33\t/// is rounded to cents first, so `100 → (\"100\",\"00\")`, `45500.5 → (\"45500\",\"50\")`,
34\t/// `100.05 → (\"100\",\"05\")`.
35\tpub fn fmt_money_pair(d: Usd) -> (String, String) {
36\t    let r = d.round_dp(2);
37\t    let whole = r.trunc();
38\t    let cents = ((r - whole).abs() * dec!(100)).round();
39\t    (whole.trunc().to_string(), format!(\"{:02}\", cents))
40\t}
41\t
42\t/// Emit the write(s) + flat placement(s) for a money cell. `descent = Some((group, ordinal))` puts
43\t/// the (dollars) field into a strictly-descending-y sequence; `None` makes it column-only.
44\tpub fn push_money(
45\t    w: &mut Vec<(String, FieldValue)>,
46\t    p: &mut Vec<FlatPlacement>,
47\t    cell: &MoneyCell,
48\t    value: Usd,
49\t    col: usize,
50\t    descent: Option<(u32, u32)>,
51\t) {
52\t    match cell {
53\t        MoneyCell::Single(fqn) => {
54\t            w.push((fqn.clone(), FieldValue::Text(fmt_money(value))));
55\t            p.push(geo_placement(fqn, col, descent));
56\t        }
57\t        MoneyCell::Pair(mp) => {
58\t            let (dollars, cents) = fmt_money_pair(value);
59\t            w.push((mp.dollars_field.clone(), FieldValue::Text(dollars)));
60\t            w.push((mp.cents_field.clone(), FieldValue::Text(cents)));
61\t            p.push(geo_placement(&mp.dollars_field, col, descent));
62\t            // The cents widget is authorized (no-unmapped) + page-checked, but not column-checked.
63\t            p.push(FlatPlacement::free(
64\t                mp.cents_field.clone(),
65\t                page_of(&mp.cents_field),
66\t            ));
67\t        }
68\t    }
69\t}
70\t
71\t/// Emit a NON-numeric literal (e.g. the \"-0-\" active-and-netted-to-zero marker) into a money cell's
72\t/// (dollars) field. Column-checked, no cents.
73\tpub fn push_literal(
74\t    w: &mut Vec<(String, FieldValue)>,
75\t    p: &mut Vec<FlatPlacement>,
76\t    cell: &MoneyCell,
77\t    literal: &str,
78\t    col: usize,
79\t) {
80\t    let fqn = match cell {
81\t        MoneyCell::Single(f) => f,
82\t        MoneyCell::Pair(mp) => &mp.dollars_field,
83\t    };
84\t    w.push((fqn.clone(), FieldValue::Text(literal.to_string())));
85\t    p.push(FlatPlacement::col_only(fqn.clone(), page_of(fqn), col));
86\t}
87\t
88\tfn geo_placement(fqn: &str, col: usize, descent: Option<(u32, u32)>) -> FlatPlacement {
89\t    let page = page_of(fqn);
90\t    match descent {
91\t        Some((grp, ord)) => FlatPlacement::cell(fqn.to_string(), page, col, grp, ord),
92\t        None => FlatPlacement::col_only(fqn.to_string(), page, col),
93\t    }
94\t}
95\t
96\t/// Render an SSN for a specific cell, from the cell's OWN declared capacity.
97\t///
98\t/// This is not a style choice — the forms genuinely disagree. The nine schedules' SSN cells declare
99\t/// `/MaxLen 11` and take the hyphenated `123-45-6789`; the 1040's declare `/MaxLen 9` and take the nine
100\t/// bare digits. Writing the hyphenated form into a 9-character comb would be silently truncated by a
101\t/// viewer (and `verify_flat` now refuses it outright). So the PDF decides, per cell.
102\t///
103\t/// `None` (no declared capacity) ⇒ the hyphenated form, which is what a human reads on a filed return.
104\tpub fn render_ssn(ssn: &Ssn, max_len: Option<usize>) -> Result<String, FormsError> {
105\t    match max_len {
106\t        None => Ok(ssn.hyphenated()),
107\t        Some(n) if n >= 11 => Ok(ssn.hyphenated()),
108\t        Some(n) if n >= 9 => Ok(ssn.digits().to_string()),
109\t        // A cell too narrow for even the bare digits: the map points at the wrong widget. Fail closed
110\t        // rather than write a truncated SSN.
111\t        Some(n) => Err(FormsError::Geometry(format!(
112\t            \"an SSN cell with /MaxLen {n} cannot hold an SSN (9 digits) — the map points at the wrong widget\"
113\t        ))),
114\t    }
115\t}
116\t
117\t/// Emit a form's identity header: the name line + the SSN.
118\t///
119\t/// Both are [`FlatPlacement::free`] — geometry-EXEMPT (they are not in any money column, so the
120\t/// column-x/descent oracle has nothing to say about them) but still page-checked and inside the
121\t/// no-unmapped set, exactly as `form8283.rs` does for its identity fields.
122\t///
123\t/// ⚠️ `free` catches STRAY writes, not MISSING ones — so every form's read-back KAT asserts its name and
124\t/// SSN cells come back non-empty. An unnamed return is the failure mode this whole function exists to
125\t/// prevent, and the oracle cannot see it.
126\tpub fn push_identity(
127\t    w: &mut Vec<(String, FieldValue)>,
128\t    p: &mut Vec<FlatPlacement>,
129\t    cells: &IdentityCells,
130\t    name: &str,
131\t    ssn: &Ssn,
132\t    fields: &[Field],
133\t) -> Result<(), FormsError> {
134\t    let ssn_max_len = fields
135\t        .iter()
136\t        .find(|f| f.fqn == cells.ssn)
137\t        .ok_or_else(|| FormsError::MapFieldMissing(cells.ssn.clone()))?
138\t        .max_len;
139\t
140\t    w.push((cells.name.clone(), FieldValue::Text(name.to_string())));
141\t    p.push(FlatPlacement::free(
142\t        cells.name.clone(),
143\t        page_of(&cells.name),
144\t    ));
145\t
146\t    w.push((
147\t        cells.ssn.clone(),
148\t        FieldValue::Text(render_ssn(ssn, ssn_max_len)?),
149\t    ));
150\t    p.push(FlatPlacement::free(cells.ssn.clone(), page_of(&cells.ssn)));
151\t    Ok(())
152\t}
153\t"}]},"uuid":"592ca531-3565-4486-adc1-ed0660e0bb68","timestamp":"2026-08-01T04:04:54.557Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/crates/btctax-forms/src/cells.rs","content":"//! Money-cell emission shared by the flat forms (Schedule SE / Form 1040 / Form 8283).
//!
//! A [`MoneyCell`] is either a single field (2024/2025) carrying the whole `fmt_money` string, or a
//! **dollars+cents [`MoneyPair`]** (the 2017 forms). For a pair the geometric oracle treats the two
//! widgets as ONE logical cell at the **dollars-field** geometry: the dollars field is the
//! column-x-/descent-checked [`FlatPlacement::cell`]/[`FlatPlacement::col_only`]; the cents field
//! rides along as an authorized-but-geometry-exempt [`FlatPlacement::free`] write. A map that swaps
//! the two members writes the whole-dollars value into the narrow cents widget → its center-x leaves
//! the dollars column cluster → the read-back FAILS CLOSED.

use crate::error::FormsError;
use crate::fmt_money;
use crate::map::{IdentityCells, MoneyCell};
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
/// `100.05 → (\"100\",\"05\")`.
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
    let fqn = match cell {
        MoneyCell::Single(f) => f,
        MoneyCell::Pair(mp) => &mp.dollars_field,
    };
    w.push((fqn.clone(), FieldValue::Text(literal.to_string())));
    p.push(FlatPlacement::col_only(fqn.clone(), page_of(fqn), col));
}

fn geo_placement(fqn: &str, col: usize, descent: Option<(u32, u32)>) -> FlatPlacement {
    let page = page_of(fqn);
    match descent {
        Some((grp, ord)) => FlatPlacement::cell(fqn.to_string(), page, col, grp, ord),
        None => FlatPlacement::col_only(fqn.to_string(), page, col),
    }
}

/// Render an SSN for a specific cell, from the cell's OWN declared capacity.
///
/// This is not a style choice — the forms genuinely disagree. The nine schedules' SSN cells declare
/// `/MaxLen 11` and take the hyphenated `123-45-6789`; the 1040's declare `/MaxLen 9` and take the nine
/// bare digits. Writing the hyphenated form into a 9-character comb would be silently truncated by a
/// viewer (and `verify_flat` now refuses it outright). So the PDF decides, per cell.
///
/// `None` (no declared capacity) ⇒ the hyphenated form, which is what a human reads on a filed return.
pub fn render_ssn(ssn: &Ssn, max_len: Option<usize>) -> Result<String, FormsError> {
    match max_len {
        None => Ok(ssn.hyphenated()),
        Some(n) if n >= 11 => Ok(ssn.hyphenated()),
        Some(n) if n >= 9 => Ok(ssn.digits().to_string()),
        // A cell too narrow for even the bare digits: the map points at the wrong widget. Fail closed
        // rather than write a truncated SSN.
        Some(n) => Err(FormsError::Geometry(format!(
            \"an SSN cell with /MaxLen {n} cannot hold an SSN (9 digits) — the map points at the wrong widget\"
        ))),
    }
}

/// Emit a form's identity header: the name line + the SSN.
///
/// Both are [`FlatPlacement::free`] — geometry-EXEMPT (they are not in any money column, so the
/// column-x/descent oracle has nothing to say about them) but still page-checked and inside the
/// no-unmapped set, exactly as `form8283.rs` does for its identity fields.
///
/// ⚠️ `free` catches STRAY writes, not MISSING ones — so every form's read-back KAT asserts its name and
/// SSN cells come back non-empty. An unnamed return is the failure mode this whole function exists to
/// prevent, and the oracle cannot see it.
pub fn push_identity(
    w: &mut Vec<(String, FieldValue)>,
    p: &mut Vec<FlatPlacement>,
    cells: &IdentityCells,
    name: &str,
    ssn: &Ssn,
    fields: &[Field],
) -> Result<(), FormsError> {
    let ssn_max_len = fields
        .iter()
        .find(|f| f.fqn == cells.ssn)
        .ok_or_else(|| FormsError::MapFieldMissing(cells.ssn.clone()))?
        .max_len;

    w.push((cells.name.clone(), FieldValue::Text(name.to_string())));
    p.push(FlatPlacement::free(
        cells.name.clone(),
        page_of(&cells.name),
    ));

    w.push((
        cells.ssn.clone(),
        FieldValue::Text(render_ssn(ssn, ssn_max_len)?),
    ));
    p.push(FlatPlacement::free(cells.ssn.clone(), page_of(&cells.ssn)));
    Ok(())
}
","numLines":153,"startLine":1,"totalLines":153}},"sourceToolAssistantUUID":"4884b045-1f65-4eab-9602-8fd3b719ad9e","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"592ca531-3565-4486-adc1-ed0660e0bb68","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNDWg2qLjTtbUHQHYzh","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISvwMKiAEIEBgCKkDO7TzhmsNT9CyFa/wHy+qK7HrMXeppmA29PIa7Ao+iUwxSj8o2WHPw6AfNBzvU0KEwlCEGBH9V3QUe8jI14UbYMg5jbGF1ZGUtZmFibGUtNTgBQgh0aGlua2luZ1okZTJmYmFjOTAtMDM4ZC00MjYzLWFjMDUtZWUxNTVlMGNjZTg2EgxezfPiVUtTzS6nogUaDIr/RniRNDUqaWyWDiIwpbnnZ+4XrypeOJa81fZnoM9HXe112XE1uz0LCLP4l5TTPjKXURVD7TFHmNgq7SDjKuMBPuyUvv922N4O73S1RcjzehxQ5bqSEfTQOVba24WgWP/BKDdP49AiTuGhIHv2q14RPd5RobH3wVTr3FnlW6/xVbZgCWwqhcYTmZ2VTShv+h4X+agZobZ1Bv7b4jAu8JzXR2smiEaHenQ2DFvk3Cxr53oKGtTSYLq8dvpV7b0QDZlw7u8fogXhwWBZKe9LPT9eKyoSm/M9mFfINl9HAu/6xf5Ixdpr/mu2QP3F3Pcj2cfeYUTM4z+mBnj3VyReCUA95S/5YEKg7iF6Vn5BCybR/NwwDc7ekGB1HaFNioqWH6TljUEYAQ=="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":7186,"cache_read_input_tokens":69213,"cache_creation":{"ephemeral_5m_input_tokens":7186,"ephemeral_1h_input_tokens":0},"output_tokens":9,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNDUvs3UNeYYSYoXvu7","attributionAgent":"general-purpose","type":"assistant","uuid":"9f231418-3e17-469c-92da-e32c0798d232","timestamp":"2026-08-01T04:04:57.479Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"9f231418-3e17-469c-92da-e32c0798d232","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNDWg2qLjTtbUHQHYzh","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_013uVmkFnkCLpFiY5DCsiYje","name":"Bash","input":{"command":"grep -n \"map_or(Usd::ZERO\" /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/printed.rs | head -30; echo ---; grep -n \"pub struct Form1040Lines\" /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/printed.rs; grep -n \"pub struct\" /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/printed.rs","description":"Locate structs and map_or sites in printed.rs"},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":7186,"cache_read_input_tokens":69213,"cache_creation":{"ephemeral_5m_input_tokens":7186,"ephemeral_1h_input_tokens":0},"output_tokens":9,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNDUvs3UNeYYSYoXvu7","attributionAgent":"general-purpose","type":"assistant","uuid":"e17a1a02-e2db-44e5-8fd6-f487dee715ed","timestamp":"2026-08-01T04:04:59.847Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"e17a1a02-e2db-44e5-8fd6-f487dee715ed","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_013uVmkFnkCLpFiY5DCsiYje","type":"tool_result","content":"341:    let line4 = sch_se.map_or(Usd::ZERO, |s| s.line12);
343:    let line12 = f8960.map_or(Usd::ZERO, |f| f.line17); // ditto
591:    let line8 = sch_1.map_or(Usd::ZERO, |s| s.line10);
593:    let line10 = sch_1.map_or(Usd::ZERO, |s| s.line26);
654:    let line13 = f8995.map_or(Usd::ZERO, |q| q.line15);
677:    let line20 = sch_3.map_or(Usd::ZERO, |s| s.line8);
680:    let line23 = sch_2.map_or(Usd::ZERO, |s| s.line21);
690:    let line31 = sch_3.map_or(Usd::ZERO, |s| s.line15);
---
448:pub struct Form1040Lines {
37:pub struct Printed8949Row {
54:pub struct Printed8949Totals {
66:pub struct Printed8949 {
135:pub struct Printed8283Rows {
197:pub struct Printed8275 {
233:pub struct ScheduleSeLines {
316:pub struct Schedule2Lines {
369:pub struct Schedule1Lines {
448:pub struct Form1040Lines {
545:pub struct Form1040Income {
778:pub struct ScheduleDLines {
922:pub struct ScheduleBRow {
953:pub struct ScheduleBLines {
1072:pub struct ScheduleCLines {
1156:pub struct ScheduleALines {
1301:pub struct Schedule3Lines {","is_error":false}]},"uuid":"417790c4-6989-4203-8ef6-cdcc187bf6e6","timestamp":"2026-08-01T04:04:59.981Z","toolUseResult":{"stdout":"341:    let line4 = sch_se.map_or(Usd::ZERO, |s| s.line12);
343:    let line12 = f8960.map_or(Usd::ZERO, |f| f.line17); // ditto
591:    let line8 = sch_1.map_or(Usd::ZERO, |s| s.line10);
593:    let line10 = sch_1.map_or(Usd::ZERO, |s| s.line26);
654:    let line13 = f8995.map_or(Usd::ZERO, |q| q.line15);
677:    let line20 = sch_3.map_or(Usd::ZERO, |s| s.line8);
680:    let line23 = sch_2.map_or(Usd::ZERO, |s| s.line21);
690:    let line31 = sch_3.map_or(Usd::ZERO, |s| s.line15);
---
448:pub struct Form1040Lines {
37:pub struct Printed8949Row {
54:pub struct Printed8949Totals {
66:pub struct Printed8949 {
135:pub struct Printed8283Rows {
197:pub struct Printed8275 {
233:pub struct ScheduleSeLines {
316:pub struct Schedule2Lines {
369:pub struct Schedule1Lines {
448:pub struct Form1040Lines {
545:pub struct Form1040Income {
778:pub struct ScheduleDLines {
922:pub struct ScheduleBRow {
953:pub struct ScheduleBLines {
1072:pub struct ScheduleCLines {
1156:pub struct ScheduleALines {
1301:pub struct Schedule3Lines {","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"e17a1a02-e2db-44e5-8fd6-f487dee715ed","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"417790c4-6989-4203-8ef6-cdcc187bf6e6","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNDWg2qLjTtbUHQHYzh","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01Y81JSdmSt9tRuR82hD8pb7","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/crates/btctax-forms/src/form1040_full.rs","offset":1,"limit":200},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":7186,"cache_read_input_tokens":69213,"output_tokens":459,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":7186},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":459,"cache_read_input_tokens":69213,"cache_creation_input_tokens":7186,"cache_creation":{"ephemeral_5m_input_tokens":7186,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbNDUvs3UNeYYSYoXvu7","attributionAgent":"general-purpose","type":"assistant","uuid":"8f926f7d-55c3-4d31-ae39-1b274657b745","timestamp":"2026-08-01T04:05:00.673Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"8f926f7d-55c3-4d31-ae39-1b274657b745","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01Y81JSdmSt9tRuR82hD8pb7","type":"tool_result","content":"1\t//! Form 1040 — the **FULL-RETURN** fill: every line, not just the capital-gain cluster.
2\t//!
3\t//! This is not `form1040::fill_form_1040_capgains`, which writes only line 7 and the Digital-Asset
4\t//! question for the crypto-slice export. That one stays; for a year with no `ReturnInputs` it is what
5\t//! the filer wants.
6\t//!
7\t//! **This module does no tax arithmetic.** Every cell is transcribed from [`Form1040Lines`], which
8\t//! core derives — and which carries each schedule's **printed** figure, so the 1040 ties out against
9\t//! its own attachments to the dollar.
10\t//!
11\t//! **★ Three x-clusters.** The lettered \"a\" sub-lines (2a, 3a) sit at x ≈ [252, 324] — neither the
12\t//! MID column (25a–25c, 31) at [410, 482] nor the AMOUNT column at [504, 576]. Line 3a is the one
13\t//! v1 writes there, and it is the preferential-rate slice: putting it in the wrong box would report
14\t//! qualified dividends as something else entirely.
15\t//!
16\t//! **★ `f1_57` is line 12 on the 2024 form and line 1z on the 2025 one** (SPEC §7.4). The maps are
17\t//! per-(form, year) for exactly this reason.
18\t//!
19\t//! **★★ The 5-way filing-status group is a NAME COLLISION.** Two distinct fields are both called
20\t//! `c1_3[0]` (Single, and Head of household) and two are both called `c1_3[1]` (MFJ, and QSS) —
21\t//! distinguished ONLY by their parent subform. A map keyed on the leaf name checks the wrong filing
22\t//! status, which changes the standard deduction, every bracket and every threshold on the return.
23\t//! The on-states (1=Single, 2=HoH, 3=MFJ, 4=MFS, 5=QSS) are distinct, so they independently
24\t//! corroborate the mapping, and [`fill_form_1040_full_with_map`] asserts both.
25\t//!
26\t//! **Line 7 is signed with a LEADING MINUS** (SPEC §3.2), unlike Schedule D's parenthesized boxes.
27\t
28\tuse crate::cells::{page_of, push_money, render_ssn};
29\tuse crate::error::FormsError;
30\tuse crate::map::{CheckChoice, Form1040HeaderCells, Form1040Map, MoneyCell};
31\tuse crate::pdf;
32\tuse crate::verify::{verify_flat, FlatPlacement};
33\tuse btctax_core::tax::packet::ReturnHeader;
34\tuse btctax_core::tax::printed::Form1040Lines;
35\tuse btctax_core::tax::types::FilingStatus;
36\tuse btctax_core::Usd;
37\t
38\t/// Logical Form 1040 columns.
39\tconst COL_SUBLINE: usize = 0; // the lettered \"a\" sub-lines (2a, 3a)
40\tconst COL_MID: usize = 1; // 25a, 25b, 25c, 31
41\tconst COL_AMOUNT: usize = 2; // everything else
42\t
43\tconst F1040_CLUSTERS: &[(f32, f32)] = &[(252.0, 324.0), (410.0, 482.0), (504.0, 576.0)];
44\t
45\t/// Descent groups — per page AND per column (a page-2 y is not comparable with a page-1 y, and a
46\t/// sub-line cell shares its row's y with the amount cell beside it).
47\tconst GRP_P1_AMOUNT: u32 = 0;
48\tconst GRP_P1_SUBLINE: u32 = 1;
49\tconst GRP_P2_AMOUNT: u32 = 2;
50\tconst GRP_P2_MID: u32 = 3;
51\t
52\tfn need<'a, T>(cell: &'a Option<T>, what: &str, year: i32) -> Result<&'a T, FormsError> {
53\t    cell.as_ref().ok_or_else(|| {
54\t        FormsError::Geometry(format!(
55\t            \"the TY{year} Form 1040 map has no `{what}` — the full-return fill needs it. Full-return \\
56\t             v1 is TY2024-only.\"
57\t        ))
58\t    })
59\t}
60\t
61\t/// The filing-status box + on-state for `status`. **Fully-qualified names only** — the leaf names
62\t/// collide (Single/HoH both `c1_3[0]`; MFJ/QSS both `c1_3[1]`).
63\tfn filing_status_box(
64\t    map: &Form1040Map,
65\t    status: FilingStatus,
66\t    year: i32,
67\t) -> Result<&CheckChoice, FormsError> {
68\t    let fs = need(&map.filing_status, \"filing_status\", year)?;
69\t    Ok(match status {
70\t        FilingStatus::Single => &fs.single,
71\t        FilingStatus::HoH => &fs.hoh,
72\t        FilingStatus::Mfj => &fs.mfj,
73\t        FilingStatus::Mfs => &fs.mfs,
74\t        FilingStatus::Qss => &fs.qss,
75\t    })
76\t}
77\t
78\t/// Fill the FULL-RETURN Form 1040. The serialized bytes are read back through the geometric verifier
79\t/// (a mis-mapped cell FAILS CLOSED).
80\tpub fn fill_form_1040_full_with_map(
81\t    lines: &Form1040Lines,
82\t    header: &ReturnHeader,
83\t    status: FilingStatus,
84\t    map: &Form1040Map,
85\t) -> Result<Vec<u8>, FormsError> {
86\t    let y = map.year;
87\t    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
88\t    let mut placements: Vec<FlatPlacement> = Vec::new();
89\t
90\t    // ── The identity block. A 1040 with the right money and no NAME is not a return. ─────────────
91\t    let h = map.header.as_ref().ok_or_else(|| {
92\t        FormsError::Geometry(format!(
93\t            \"the TY{y} 1040 map has no [header] block — a full return cannot file an unnamed 1040\"
94\t        ))
95\t    })?;
96\t    let blank = pdf::load(pdf::f1040_pdf(y)?)?;
97\t    let blank_fields = pdf::collect_fields(&blank)?;
98\t    push_header_block(
99\t        &mut writes,
100\t        &mut placements,
101\t        h,
102\t        header,
103\t        status,
104\t        &blank_fields,
105\t    )?;
106\t
107\t    // ── Page 1, AMOUNT column, top to bottom. Line 7 carries a LEADING MINUS on a loss year. ────
108\t    let p1: [(&MoneyCell, Usd); 13] = [
109\t        // ★ 1a must print BEFORE 1z — the form says \"Add lines 1a through 1h\", and a filled 1z above a
110\t        // blank 1a does not add up (Fable P6 r1 I1).
111\t        (need(&map.line1a, \"line1a\", y)?, lines.line1a),
112\t        (need(&map.line1z, \"line1z\", y)?, lines.line1z),
113\t        (need(&map.line2b, \"line2b\", y)?, lines.line2b),
114\t        (need(&map.line3b, \"line3b\", y)?, lines.line3b),
115\t        (&map.line7a, lines.line7), // the existing crypto-slice cell IS line 7 on the 2024 form
116\t        (need(&map.line8, \"line8\", y)?, lines.line8),
117\t        (need(&map.line9, \"line9\", y)?, lines.line9),
118\t        (need(&map.line10, \"line10\", y)?, lines.line10),
119\t        (need(&map.line11, \"line11\", y)?, lines.line11),
120\t        (need(&map.line12, \"line12\", y)?, lines.line12),
121\t        (need(&map.line13, \"line13\", y)?, lines.line13),
122\t        (need(&map.line14, \"line14\", y)?, lines.line14),
123\t        (need(&map.line15, \"line15\", y)?, lines.line15),
124\t    ];
125\t    for (ord, (cell, value)) in p1.iter().enumerate() {
126\t        push_money(
127\t            &mut writes,
128\t            &mut placements,
129\t            cell,
130\t            *value,
131\t            COL_AMOUNT,
132\t            Some((GRP_P1_AMOUNT, ord as u32)),
133\t        );
134\t    }
135\t    // Line 2a — tax-exempt interest, the SUBLINE column, one printed row ABOVE 3a (so ordinal 0).
136\t    push_money(
137\t        &mut writes,
138\t        &mut placements,
139\t        need(&map.line2a, \"line2a\", y)?,
140\t        lines.line2a,
141\t        COL_SUBLINE,
142\t        Some((GRP_P1_SUBLINE, 0)),
143\t    );
144\t
145\t    // ★ Line 3a — the SUBLINE column. The preferential-rate slice; a wrong box misreports it.
146\t    push_money(
147\t        &mut writes,
148\t        &mut placements,
149\t        need(&map.line3a, \"line3a\", y)?,
150\t        lines.line3a,
151\t        COL_SUBLINE,
152\t        Some((GRP_P1_SUBLINE, 1)),
153\t    );
154\t
155\t    // ── Page 2, AMOUNT column. ──────────────────────────────────────────────────────────────────
156\t    let p2_amount: [(&MoneyCell, Usd); 15] = [
157\t        (need(&map.line16, \"line16\", y)?, lines.line16),
158\t        (need(&map.line17, \"line17\", y)?, lines.line17),
159\t        (need(&map.line18, \"line18\", y)?, lines.line18),
160\t        (need(&map.line19, \"line19\", y)?, lines.line19),
161\t        (need(&map.line20, \"line20\", y)?, lines.line20),
162\t        (need(&map.line21, \"line21\", y)?, lines.line21),
163\t        (need(&map.line22, \"line22\", y)?, lines.line22),
164\t        (need(&map.line23, \"line23\", y)?, lines.line23),
165\t        (need(&map.line24, \"line24\", y)?, lines.line24),
166\t        (need(&map.line25d, \"line25d\", y)?, lines.line25d),
167\t        (need(&map.line26, \"line26\", y)?, lines.line26),
168\t        (need(&map.line32, \"line32\", y)?, lines.line32),
169\t        (need(&map.line33, \"line33\", y)?, lines.line33),
170\t        (need(&map.line34, \"line34\", y)?, lines.line34),
171\t        (need(&map.line35a, \"line35a\", y)?, lines.line34), // 35a = the overpayment refunded
172\t    ];
173\t    for (ord, (cell, value)) in p2_amount.iter().enumerate() {
174\t        push_money(
175\t            &mut writes,
176\t            &mut placements,
177\t            cell,
178\t            *value,
179\t            COL_AMOUNT,
180\t            Some((GRP_P2_AMOUNT, ord as u32)),
181\t        );
182\t    }
183\t    // Line 37 (amount owed) sits below 35a and is still the AMOUNT column.
184\t    push_money(
185\t        &mut writes,
186\t        &mut placements,
187\t        need(&map.line37, \"line37\", y)?,
188\t        lines.line37,
189\t        COL_AMOUNT,
190\t        Some((GRP_P2_AMOUNT, p2_amount.len() as u32)),
191\t    );
192\t
193\t    // ── Page 2, MID column. ─────────────────────────────────────────────────────────────────────
194\t    let p2_mid: [(&MoneyCell, Usd); 4] = [
195\t        (need(&map.line25a, \"line25a\", y)?, lines.line25a),
196\t        (need(&map.line25b, \"line25b\", y)?, lines.line25b),
197\t        (need(&map.line25c, \"line25c\", y)?, lines.line25c),
198\t        (need(&map.line31, \"line31\", y)?, lines.line31),
199\t    ];
200\t    for (ord, (cell, value)) in p2_mid.iter().enumerate() {"}]},"uuid":"2688bcc0-8293-451e-88a0-d094d046862b","timestamp":"2026-08-01T04:05:00.680Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/crates/btctax-forms/src/form1040_full.rs","content":"//! Form 1040 — the **FULL-RETURN** fill: every line, not just the capital-gain cluster.
//!
//! This is not `form1040::fill_form_1040_capgains`, which writes only line 7 and the Digital-Asset
//! question for the crypto-slice export. That one stays; for a year with no `ReturnInputs` it is what
//! the filer wants.
//!
//! **This module does no tax arithmetic.** Every cell is transcribed from [`Form1040Lines`], which
//! core derives — and which carries each schedule's **printed** figure, so the 1040 ties out against
//! its own attachments to the dollar.
//!
//! **★ Three x-clusters.** The lettered \"a\" sub-lines (2a, 3a) sit at x ≈ [252, 324] — neither the
//! MID column (25a–25c, 31) at [410, 482] nor the AMOUNT column at [504, 576]. Line 3a is the one
//! v1 writes there, and it is the preferential-rate slice: putting it in the wrong box would report
//! qualified dividends as something else entirely.
//!
//! **★ `f1_57` is line 12 on the 2024 form and line 1z on the 2025 one** (SPEC §7.4). The maps are
//! per-(form, year) for exactly this reason.
//!
//! **★★ The 5-way filing-status group is a NAME COLLISION.** Two distinct fields are both called
//! `c1_3[0]` (Single, and Head of household) and two are both called `c1_3[1]` (MFJ, and QSS) —
//! distinguished ONLY by their parent subform. A map keyed on the leaf name checks the wrong filing
//! status, which changes the standard deduction, every bracket and every threshold on the return.
//! The on-states (1=Single, 2=HoH, 3=MFJ, 4=MFS, 5=QSS) are distinct, so they independently
//! corroborate the mapping, and [`fill_form_1040_full_with_map`] asserts both.
//!
//! **Line 7 is signed with a LEADING MINUS** (SPEC §3.2), unlike Schedule D's parenthesized boxes.

use crate::cells::{page_of, push_money, render_ssn};
use crate::error::FormsError;
use crate::map::{CheckChoice, Form1040HeaderCells, Form1040Map, MoneyCell};
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::tax::printed::Form1040Lines;
use btctax_core::tax::types::FilingStatus;
use btctax_core::Usd;

/// Logical Form 1040 columns.
const COL_SUBLINE: usize = 0; // the lettered \"a\" sub-lines (2a, 3a)
const COL_MID: usize = 1; // 25a, 25b, 25c, 31
const COL_AMOUNT: usize = 2; // everything else

const F1040_CLUSTERS: &[(f32, f32)] = &[(252.0, 324.0), (410.0, 482.0), (504.0, 576.0)];

/// Descent groups — per page AND per column (a page-2 y is not comparable with a page-1 y, and a
/// sub-line cell shares its row's y with the amount cell beside it).
const GRP_P1_AMOUNT: u32 = 0;
const GRP_P1_SUBLINE: u32 = 1;
const GRP_P2_AMOUNT: u32 = 2;
const GRP_P2_MID: u32 = 3;

fn need<'a, T>(cell: &'a Option<T>, what: &str, year: i32) -> Result<&'a T, FormsError> {
    cell.as_ref().ok_or_else(|| {
        FormsError::Geometry(format!(
            \"the TY{year} Form 1040 map has no `{what}` — the full-return fill needs it. Full-return \\
             v1 is TY2024-only.\"
        ))
    })
}

/// The filing-status box + on-state for `status`. **Fully-qualified names only** — the leaf names
/// collide (Single/HoH both `c1_3[0]`; MFJ/QSS both `c1_3[1]`).
fn filing_status_box(
    map: &Form1040Map,
    status: FilingStatus,
    year: i32,
) -> Result<&CheckChoice, FormsError> {
    let fs = need(&map.filing_status, \"filing_status\", year)?;
    Ok(match status {
        FilingStatus::Single => &fs.single,
        FilingStatus::HoH => &fs.hoh,
        FilingStatus::Mfj => &fs.mfj,
        FilingStatus::Mfs => &fs.mfs,
        FilingStatus::Qss => &fs.qss,
    })
}

/// Fill the FULL-RETURN Form 1040. The serialized bytes are read back through the geometric verifier
/// (a mis-mapped cell FAILS CLOSED).
pub fn fill_form_1040_full_with_map(
    lines: &Form1040Lines,
    header: &ReturnHeader,
    status: FilingStatus,
    map: &Form1040Map,
) -> Result<Vec<u8>, FormsError> {
    let y = map.year;
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // ── The identity block. A 1040 with the right money and no NAME is not a return. ─────────────
    let h = map.header.as_ref().ok_or_else(|| {
        FormsError::Geometry(format!(
            \"the TY{y} 1040 map has no [header] block — a full return cannot file an unnamed 1040\"
        ))
    })?;
    let blank = pdf::load(pdf::f1040_pdf(y)?)?;
    let blank_fields = pdf::collect_fields(&blank)?;
    push_header_block(
        &mut writes,
        &mut placements,
        h,
        header,
        status,
        &blank_fields,
    )?;

    // ── Page 1, AMOUNT column, top to bottom. Line 7 carries a LEADING MINUS on a loss year. ────
    let p1: [(&MoneyCell, Usd); 13] = [
        // ★ 1a must print BEFORE 1z — the form says \"Add lines 1a through 1h\", and a filled 1z above a
        // blank 1a does not add up (Fable P6 r1 I1).
        (need(&map.line1a, \"line1a\", y)?, lines.line1a),
        (need(&map.line1z, \"line1z\", y)?, lines.line1z),
        (need(&map.line2b, \"line2b\", y)?, lines.line2b),
        (need(&map.line3b, \"line3b\", y)?, lines.line3b),
        (&map.line7a, lines.line7), // the existing crypto-slice cell IS line 7 on the 2024 form
        (need(&map.line8, \"line8\", y)?, lines.line8),
        (need(&map.line9, \"line9\", y)?, lines.line9),
        (need(&map.line10, \"line10\", y)?, lines.line10),
        (need(&map.line11, \"line11\", y)?, lines.line11),
        (need(&map.line12, \"line12\", y)?, lines.line12),
        (need(&map.line13, \"line13\", y)?, lines.line13),
        (need(&map.line14, \"line14\", y)?, lines.line14),
        (need(&map.line15, \"line15\", y)?, lines.line15),
    ];
    for (ord, (cell, value)) in p1.iter().enumerate() {
        push_money(
            &mut writes,
            &mut placements,
            cell,
            *value,
            COL_AMOUNT,
            Some((GRP_P1_AMOUNT, ord as u32)),
        );
    }
    // Line 2a — tax-exempt interest, the SUBLINE column, one printed row ABOVE 3a (so ordinal 0).
    push_money(
        &mut writes,
        &mut placements,
        need(&map.line2a, \"line2a\", y)?,
        lines.line2a,
        COL_SUBLINE,
        Some((GRP_P1_SUBLINE, 0)),
    );

    // ★ Line 3a — the SUBLINE column. The preferential-rate slice; a wrong box misreports it.
    push_money(
        &mut writes,
        &mut placements,
        need(&map.line3a, \"line3a\", y)?,
        lines.line3a,
        COL_SUBLINE,
        Some((GRP_P1_SUBLINE, 1)),
    );

    // ── Page 2, AMOUNT column. ──────────────────────────────────────────────────────────────────
    let p2_amount: [(&MoneyCell, Usd); 15] = [
        (need(&map.line16, \"line16\", y)?, lines.line16),
        (need(&map.line17, \"line17\", y)?, lines.line17),
        (need(&map.line18, \"line18\", y)?, lines.line18),
        (need(&map.line19, \"line19\", y)?, lines.line19),
        (need(&map.line20, \"line20\", y)?, lines.line20),
        (need(&map.line21, \"line21\", y)?, lines.line21),
        (need(&map.line22, \"line22\", y)?, lines.line22),
        (need(&map.line23, \"line23\", y)?, lines.line23),
        (need(&map.line24, \"line24\", y)?, lines.line24),
        (need(&map.line25d, \"line25d\", y)?, lines.line25d),
        (need(&map.line26, \"line26\", y)?, lines.line26),
        (need(&map.line32, \"line32\", y)?, lines.line32),
        (need(&map.line33, \"line33\", y)?, lines.line33),
        (need(&map.line34, \"line34\", y)?, lines.line34),
        (need(&map.line35a, \"line35a\", y)?, lines.line34), // 35a = the overpayment refunded
    ];
    for (ord, (cell, value)) in p2_amount.iter().enumerate() {
        push_money(
            &mut writes,
            &mut placements,
            cell,
            *value,
            COL_AMOUNT,
            Some((GRP_P2_AMOUNT, ord as u32)),
        );
    }
    // Line 37 (amount owed) sits below 35a and is still the AMOUNT column.
    push_money(
        &mut writes,
        &mut placements,
        need(&map.line37, \"line37\", y)?,
        lines.line37,
        COL_AMOUNT,
        Some((GRP_P2_AMOUNT, p2_amount.len() as u32)),
    );

    // ── Page 2, MID column. ─────────────────────────────────────────────────────────────────────
    let p2_mid: [(&MoneyCell, Usd); 4] = [
        (need(&map.line25a, \"line25a\", y)?, lines.line25a),
        (need(&map.line25b, \"line25b\", y)?, lines.line25b),
        (need(&map.line25c, \"line25c\", y)?, lines.line25c),
        (need(&map.line31, \"line31\", y)?, lines.line31),
    ];
    for (ord, (cell, value)) in p2_mid.iter().enumerate() {","numLines":200,"startLine":1,"totalLines":381}},"sourceToolAssistantUUID":"8f926f7d-55c3-4d31-ae39-1b274657b745","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"2688bcc0-8293-451e-88a0-d094d046862b","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNDxv6tNtsbng33kxXk","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAIS4gMKiAEIEBgCKkBQiVPnU4l3LJoyFzu5rFYDwp/GWbwuaKHr2x1g1GpbfOGJhURbT7nW1uDeo1FS8Jx9JDMO5nUX8uMCGI+mehwHMg5jbGF1ZGUtZmFibGUtNTgBQgh0aGlua2luZ1okZTJmYmFjOTAtMDM4ZC00MjYzLWFjMDUtZWUxNTVlMGNjZTg2EgzqcE4XfFUVVr3QGjsaDCphROPizHBgmcneHCIwWYqQb6ioPHGedG1oBNG4JuG5GRbwB0LBtZMBDiJ5eoQUJ8b/d5/wkMGekcq37tYzKoYCcsJdo7qJ82WptW2fLq7IxihXrmmfuPsj4NOidjqmDbqKqBD0mfrdT/Qz+em5P4+inMmyTkp2zIFueDYUaEHci0bJhCLXadI+qp2CRaS+QQzsvsBl3ghxmQc9Wv3tdmbdJGNudQ0buEnnNyw7fxaovHujN/sLB2ESNmsyZatZ9ic8Z7ggxtpzaQ4EC0VSfVla1NJSxy69VrBClOLG7iHiraQhbXtaoRLvj4blbNbVBnrXp+RtQd1rNK1FQ+I0WdikwhZCNCzm/lGxl7/bImAUzWTEcjpLwjIjrVV7ENZSwq+zQTYjY9IfFOK4+PU/3+dg2z9eEiQFj2oFdt5WSNIs33VauYKgHhgB"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":5348,"cache_read_input_tokens":76399,"cache_creation":{"ephemeral_5m_input_tokens":5348,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNDw4Ecv9cZRT5JoSLU","attributionAgent":"general-purpose","type":"assistant","uuid":"bb68a255-91c7-4170-b9d2-5a543c6843ce","timestamp":"2026-08-01T04:05:04.040Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"bb68a255-91c7-4170-b9d2-5a543c6843ce","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNDxv6tNtsbng33kxXk","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01M5JCjKx1C43Gn1wAVpffM9","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/printed.rs","offset":440,"limit":260},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":5348,"cache_read_input_tokens":76399,"output_tokens":230,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":5348},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":230,"cache_read_input_tokens":76399,"cache_creation_input_tokens":5348,"cache_creation":{"ephemeral_5m_input_tokens":5348,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbNDw4Ecv9cZRT5JoSLU","attributionAgent":"general-purpose","type":"assistant","uuid":"ccf5a1d9-c329-4698-97c9-20a3fda691ae","timestamp":"2026-08-01T04:05:05.104Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"ccf5a1d9-c329-4698-97c9-20a3fda691ae","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01M5JCjKx1C43Gn1wAVpffM9","type":"tool_result","content":"440\t/// §3.2) — unlike Schedule D's own lines 6/14/21, which are parenthesized boxes carrying magnitudes.
441\t/// On a net-loss year it carries `−(Schedule D line 21)`, the §1211(b)-limited amount, not the full
442\t/// loss.
443\t///
444\t/// **Conservative omissions print as absent, not zero, where the form allows** — but line 19 (the
445\t/// CTC/ODC) is a computed credit line the form expects, so it prints `0` with the `CtcOdcOmitted`
446\t/// advisory carrying the news that the filer may be owed more.
447\t#[derive(Debug, Clone, Copy, PartialEq, Eq)]
448\tpub struct Form1040Lines {
449\t    /// L1a / L1z — wages (Σ W-2 box 1). v1 has no other line-1 component, so 1z = 1a.
450\t    pub line1z: Usd,
451\t    /// L1a — \"Total amount from Form(s) W-2, box 1\". ★ Its absence left the filed 1z sitting above an
452\t    /// EMPTY operand column: the form's own \"Add lines 1a through 1h\" sums blanks to 0 ≠ 1z, on the very
453\t    /// line the Service document-matches against your W-2s (Fable P6 r1 I1).
454\t    pub line1a: Usd,
455\t    /// L2a — **tax-exempt interest** (Σ 1099-INT box 8 + 1099-DIV box 12). Changes no tax; the IRS
456\t    /// document-matches it, and a return that omits it misstates itself (ARCH-P6.3a Q7 item 2).
457\t    pub line2a: Usd,
458\t    /// L2b — taxable interest: **Schedule B's printed line 4** when Schedule B files, else the sum.
459\t    pub line2b: Usd,
460\t    /// L3a — qualified dividends (Σ 1099-DIV box 1b; the preferential slice).
461\t    pub line3a: Usd,
462\t    /// L3b — ordinary dividends: **Schedule B's printed line 6** when Schedule B files, else the sum.
463\t    /// This is the FULL box-1a amount, which INCLUDES the qualified subset on 3a.
464\t    pub line3b: Usd,
465\t    /// L7 — capital gain or (loss): **Schedule D's printed line 16**, or on a net-loss year the
466\t    /// §1211(b)-limited `−(Schedule D line 21)`. Signed with a **leading minus**.
467\t    pub line7: Usd,
468\t    /// L8 — **Schedule 1's printed line 10** (additional income).
469\t    pub line8: Usd,
470\t    /// L9 — total income = add **printed** 1z, 2b, 3b, 7 and 8.
471\t    pub line9: Usd,
472\t    /// L10 — **Schedule 1's printed line 26** (adjustments).
473\t    pub line10: Usd,
474\t    /// L11 — **AGI** = printed 9 − printed 10.
475\t    pub line11: Usd,
476\t    /// L12 — the deduction actually claimed: **Schedule A's printed line 17** when itemizing, else the
477\t    /// §63 standard deduction.
478\t    pub line12: Usd,
479\t    /// L13 — **Form 8995's printed line 15** (the §199A QBI deduction).
480\t    pub line13: Usd,
481\t    /// L14 — add **printed** 12 and 13.
482\t    pub line14: Usd,
483\t    /// L15 — **taxable income** = printed 11 − printed 14, floored at 0.
484\t    pub line15: Usd,
485\t    /// L16 — the regular tax (Tax Table / TCW / QDCGT, per `method.rs`).
486\t    pub line16: Usd,
487\t    /// L17 — **Schedule 2's printed line 3** (Part I). Always 0 in v1 — see [`Schedule2Lines`].
488\t    pub line17: Usd,
489\t    /// L18 — add **printed** 16 and 17.
490\t    pub line18: Usd,
491\t    /// L19 — CTC / credit for other dependents. **Always 0** (a §3.4 conservative omission; the
492\t    /// `CtcOdcOmitted` advisory tells the filer their tax is overstated).
493\t    pub line19: Usd,
494\t    /// L20 — **Schedule 3's printed line 8** (nonrefundable credits: the FTC).
495\t    pub line20: Usd,
496\t    /// L21 — add **printed** 19 and 20.
497\t    pub line21: Usd,
498\t    /// L22 — printed 18 − printed 21, floored at 0.
499\t    pub line22: Usd,
500\t    /// L23 — **Schedule 2's printed line 21** (other taxes).
501\t    pub line23: Usd,
502\t    /// L24 — **TOTAL TAX** = add **printed** 22 and 23.
503\t    pub line24: Usd,
504\t    /// L25a — federal income tax withheld from Form(s) W-2 (Σ box 2).
505\t    pub line25a: Usd,
506\t    /// L25b — withheld from Form(s) 1099 (Σ box 4).
507\t    pub line25b: Usd,
508\t    /// L25c — withheld from other forms: **Form 8959's printed line 24** (the Additional-Medicare
509\t    /// over-withholding credit) plus any other declared withholding.
510\t    pub line25c: Usd,
511\t    /// L25d — add **printed** 25a, 25b and 25c.
512\t    pub line25d: Usd,
513\t    /// L26 — estimated tax payments.
514\t    pub line26: Usd,
515\t    /// L31 — **Schedule 3's printed line 15** (other payments: extension + excess Social Security).
516\t    pub line31: Usd,
517\t    /// L32 — total other payments and refundable credits ⇒ `= line31` (lines 27–30 are blank: the EIC
518\t    /// is a §3.4 conservative omission, and the rest are unrepresentable).
519\t    pub line32: Usd,
520\t    /// L33 — **TOTAL PAYMENTS** = add **printed** 25d, 26 and 32.
521\t    pub line33: Usd,
522\t    /// L34 / L35a — the overpayment refunded. Zero when the return owes. v1 never fills the
523\t    /// direct-deposit block (35b–35d), so a refund arrives as a paper check — the `RefundByPaperCheck`
524\t    /// advisory says so.
525\t    pub line34: Usd,
526\t    /// L37 — the amount owed. Zero when the return is due a refund.
527\t    pub line37: Usd,
528\t    /// The **Digital Asset question**. `true` for any crypto disposal, income, gift or donation.
529\t    /// btctax never answers \"No\" — a \"No\" it cannot vouch for is worse than leaving the question to
530\t    /// the filer, so this is `true` or the question is left for them.
531\t    pub digital_asset_yes: bool,
532\t}
533\t
534\t/// The 1040's **income block** (lines 1a–11), printed.
535\t///
536\t/// Extracted from [`form_1040_lines`] because **Schedule A line 2 cites it**: \"Enter amount from Form
537\t/// 1040 or 1040-SR, **line 11**\". Under the §3.1 citation-composition rule that is a SOURCE citation, so
538\t/// Schedule A must take the PRINTED L11 — not a re-rounding of the exact AGI, which can differ by
539\t/// dollars and, on a negative-AGI itemizer, prints 0 beside a negative 1040 L11 (Fable P6 r1 I5).
540\t///
541\t/// There is no cycle: L11 = L9 − L10 depends on Schedules B/1/D, never on Schedule A (which lands at
542\t/// L12). So the income block is derived first, Schedule A composes on its L11, and the rest of the 1040
543\t/// consumes both.
544\t#[derive(Debug, Clone, Copy, PartialEq, Eq)]
545\tpub struct Form1040Income {
546\t    /// The QDCGT worksheet's **line 3** operand: `min(printed Sch D L15, printed Sch D L16)`, floored at
547\t    /// 0 — the §1(h) preferential net capital gain **as a human reads it off the FILED Schedule D**
548\t    /// (`qdcgt_line16`'s own doc: \"`net_ltcg` = `min(Sch D 15,16)`, ≥0; L3\").
549\t    ///
550\t    /// ★ It is NOT `round_dollar(exact net_ltcg)` (Fable P6 r2 NEW-I1). The printed Sch D L15/L16 are sums
551\t    /// of printed operands and drift from the exact figure — that is what the 302≠301 8949 KAT is about.
552\t    /// A $1 drift here shifts the worksheet's ordinary remainder by $1, and THAT feeds the same $50-tread
553\t    /// Tax Table step function line 16 was already gated on: across a bin edge the filed L16 disagrees
554\t    /// with the worksheet a filer (or the Service) computes from the filed Schedule D, by a whole bin.
555\t    pub qdcgt_net_capital_gain: Usd,
556\t    pub line1a: Usd,
557\t    pub line1z: Usd,
558\t    pub line2a: Usd,
559\t    pub line2b: Usd,
560\t    pub line3a: Usd,
561\t    pub line3b: Usd,
562\t    pub line7: Usd,
563\t    pub line8: Usd,
564\t    pub line9: Usd,
565\t    pub line10: Usd,
566\t    pub line11: Usd,
567\t}
568\t
569\t/// Derive the 1040's printed income block (lines 1a–11).
570\tpub fn form_1040_income_lines(
571\t    ar: &AbsoluteReturn,
572\t    sch_b: Option<&ScheduleBLines>,
573\t    sch_1: Option<&Schedule1Lines>,
574\t    sch_d: &ScheduleDLines,
575\t) -> Form1040Income {
576\t    // L1a is the Σ of W-2 box 1, and L1z is \"Add lines 1a through 1h\" — v1 has no 1b–1h, so they are
577\t    // the same figure, and BOTH cells print. A filled 1z above a blank 1a does not add up.
578\t    let line1a = round_dollar(ar.wages);
579\t    let line1z = line1a;
580\t    let line2a = round_dollar(ar.printed_inputs.tax_exempt_interest);
581\t    let line2b = sch_b.map_or_else(|| round_dollar(ar.taxable_interest), |b| b.line4);
582\t    let line3a = round_dollar(ar.qualified_dividends);
583\t    let line3b = sch_b.map_or_else(|| round_dollar(ar.ordinary_dividends), |b| b.line6);
584\t
585\t    // L7 — signed, with a LEADING MINUS (not parentheses; SPEC §3.2). On a net-loss year it is the
586\t    // §1211(b)-LIMITED amount from Schedule D line 21, not the full loss.
587\t    let line7 = match sch_d.routing {
588\t        ScheduleDRouting::NetLoss { line21, .. } => -line21,
589\t        _ => sch_d.line16,
590\t    };
591\t    let line8 = sch_1.map_or(Usd::ZERO, |s| s.line10);
592\t    let line9 = line1z + line2b + line3b + line7 + line8; // ★ sums the PRINTED lines
593\t    let line10 = sch_1.map_or(Usd::ZERO, |s| s.line26);
594\t    let line11 = line9 - line10;
595\t
596\t    Form1040Income {
597\t        // The worksheet reads these OFF THE FILED SCHEDULE D — printed lines 15 and 16, smaller of, ≥ 0.
598\t        qdcgt_net_capital_gain: sch_d.line15.min(sch_d.line16).max(Usd::ZERO),
599\t        line1a,
600\t        line1z,
601\t        line2a,
602\t        line2b,
603\t        line3a,
604\t        line3b,
605\t        line7,
606\t        line8,
607\t        line9,
608\t        line10,
609\t        line11,
610\t    }
611\t}
612\t
613\t/// Derive the printed Form 1040 chain. Every schedule figure is taken from that schedule's **printed**
614\t/// chain, so the return ties out against its own attachments.
615\t///
616\t/// `standard_or_itemized` is the deduction actually claimed (line 12): Schedule A's printed line 17 if
617\t/// the return itemizes, else `round_dollar` of the §63 standard deduction.
618\t#[allow(clippy::too_many_arguments)]
619\tpub fn form_1040_lines(
620\t    ar: &AbsoluteReturn,
621\t    income: &Form1040Income,
622\t    sch_a: Option<&ScheduleALines>,
623\t    sch_2: Option<&Schedule2Lines>,
624\t    sch_3: Option<&Schedule3Lines>,
625\t    f8959: &Form8959Lines,
626\t    f8995: Option<&crate::tax::qbi::Form8995Lines>,
627\t    table: &crate::tax::tables::TaxTable,
628\t    status: FilingStatus,
629\t    other_withholding: Usd,
630\t    estimated_payments: Usd,
631\t    digital_asset_yes: bool,
632\t) -> Form1040Lines {
633\t    // ── Income — from the printed block (Schedule A already composed on its L11). ───────────────
634\t    let Form1040Income {
635\t        qdcgt_net_capital_gain,
636\t        line1a,
637\t        line1z,
638\t        line2a,
639\t        line2b,
640\t        line3a,
641\t        line3b,
642\t        line7,
643\t        line8,
644\t        line9,
645\t        line10,
646\t        line11,
647\t    } = *income;
648\t
649\t    // ── Deductions → taxable income ─────────────────────────────────────────────────────────────
650\t    let line12 = match sch_a {
651\t        Some(a) => a.line17, // itemizing — Schedule A's PRINTED total
652\t        None => round_dollar(ar.deduction),
653\t    };
654\t    let line13 = f8995.map_or(Usd::ZERO, |q| q.line15);
655\t    // L14 — \"Add lines 12e, 13a, and 13b\" (2025) / \"Add lines 12 and 13\" (2024). The third term is
656\t    // Schedule 1-A's total, which does not exist on the 2024 form.
657\t    let line14 = line12 + line13 + round_dollar(ar.schedule_1a_additional);
658\t    let line15 = (line11 - line14).max(Usd::ZERO);
659\t
660\t    // ── Tax → total tax ─────────────────────────────────────────────────────────────────────────
661\t    // ★ L16 is the Tax Table / QDCGT worksheet applied to the PRINTED line 15 — NOT a re-rounding of the
662\t    // tax computed on the exact-cents taxable income (Fable P6 r1 I2). The Tax Table is a STEP function
663\t    // with $50 treads: when the exact TI and the printed L15 straddle a bin edge, the two differ by a
664\t    // whole bin step (up to ~$18.50 at the top rate), not by the $1 rounding residual §3.1 tolerates —
665\t    // and \"L16 vs Table(L15)\" is the single most-recomputed arithmetic on a transcribed return. The
666\t    // exact-cents `ar.regular_tax` remains the COMPUTED liability; only the FILED cell changes.
667\t    let line16 = crate::tax::method::qdcgt_line16(
668\t        table.ordinary_for(status),
669\t        table.ltcg_for(status),
670\t        line15,
671\t        line3a,
672\t        qdcgt_net_capital_gain, // ★ the PRINTED Schedule D figure, not round(exact) — r2 NEW-I1
673\t    );
674\t    let line17 = Usd::ZERO; // Schedule 2 Part I is blank in v1 (see Schedule2Lines)
675\t    let line18 = line16 + line17;
676\t    let line19 = Usd::ZERO; // ★ CTC/ODC — a §3.4 conservative omission (advisory fires)
677\t    let line20 = sch_3.map_or(Usd::ZERO, |s| s.line8);
678\t    let line21 = line19 + line20;
679\t    let line22 = (line18 - line21).max(Usd::ZERO);
680\t    let line23 = sch_2.map_or(Usd::ZERO, |s| s.line21);
681\t    let line24 = line22 + line23; // ★ TOTAL TAX, from the PRINTED lines
682\t
683\t    // ── Payments ────────────────────────────────────────────────────────────────────────────────
684\t    let line25a = round_dollar(ar.withholding_25a);
685\t    let line25b = round_dollar(ar.withholding_25b);
686\t    // 25c carries Form 8959's PRINTED line 24 — the Additional-Medicare over-withholding credit.
687\t    let line25c = f8959.line24 + round_dollar(other_withholding);
688\t    let line25d = line25a + line25b + line25c;
689\t    let line26 = round_dollar(estimated_payments);
690\t    let line31 = sch_3.map_or(Usd::ZERO, |s| s.line15);
691\t    let line32 = line31; // 27-30 blank (EIC omitted conservatively; rest unrepresentable)
692\t    let line33 = line25d + line26 + line32; // ★ TOTAL PAYMENTS, from the PRINTED lines
693\t
694\t    // ── Refund or owed — from the PRINTED total tax and the PRINTED total payments, so the bottom
695\t    //    line of the filed form is the one a reader re-adding the column arrives at.
696\t    let line34 = (line33 - line24).max(Usd::ZERO);
697\t    let line37 = (line24 - line33).max(Usd::ZERO);
698\t
699\t    Form1040Lines {"}]},"uuid":"d108de38-49d5-4b33-b24a-4d1cf4736f8d","timestamp":"2026-08-01T04:05:05.110Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/printed.rs","content":"/// §3.2) — unlike Schedule D's own lines 6/14/21, which are parenthesized boxes carrying magnitudes.
/// On a net-loss year it carries `−(Schedule D line 21)`, the §1211(b)-limited amount, not the full
/// loss.
///
/// **Conservative omissions print as absent, not zero, where the form allows** — but line 19 (the
/// CTC/ODC) is a computed credit line the form expects, so it prints `0` with the `CtcOdcOmitted`
/// advisory carrying the news that the filer may be owed more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Form1040Lines {
    /// L1a / L1z — wages (Σ W-2 box 1). v1 has no other line-1 component, so 1z = 1a.
    pub line1z: Usd,
    /// L1a — \"Total amount from Form(s) W-2, box 1\". ★ Its absence left the filed 1z sitting above an
    /// EMPTY operand column: the form's own \"Add lines 1a through 1h\" sums blanks to 0 ≠ 1z, on the very
    /// line the Service document-matches against your W-2s (Fable P6 r1 I1).
    pub line1a: Usd,
    /// L2a — **tax-exempt interest** (Σ 1099-INT box 8 + 1099-DIV box 12). Changes no tax; the IRS
    /// document-matches it, and a return that omits it misstates itself (ARCH-P6.3a Q7 item 2).
    pub line2a: Usd,
    /// L2b — taxable interest: **Schedule B's printed line 4** when Schedule B files, else the sum.
    pub line2b: Usd,
    /// L3a — qualified dividends (Σ 1099-DIV box 1b; the preferential slice).
    pub line3a: Usd,
    /// L3b — ordinary dividends: **Schedule B's printed line 6** when Schedule B files, else the sum.
    /// This is the FULL box-1a amount, which INCLUDES the qualified subset on 3a.
    pub line3b: Usd,
    /// L7 — capital gain or (loss): **Schedule D's printed line 16**, or on a net-loss year the
    /// §1211(b)-limited `−(Schedule D line 21)`. Signed with a **leading minus**.
    pub line7: Usd,
    /// L8 — **Schedule 1's printed line 10** (additional income).
    pub line8: Usd,
    /// L9 — total income = add **printed** 1z, 2b, 3b, 7 and 8.
    pub line9: Usd,
    /// L10 — **Schedule 1's printed line 26** (adjustments).
    pub line10: Usd,
    /// L11 — **AGI** = printed 9 − printed 10.
    pub line11: Usd,
    /// L12 — the deduction actually claimed: **Schedule A's printed line 17** when itemizing, else the
    /// §63 standard deduction.
    pub line12: Usd,
    /// L13 — **Form 8995's printed line 15** (the §199A QBI deduction).
    pub line13: Usd,
    /// L14 — add **printed** 12 and 13.
    pub line14: Usd,
    /// L15 — **taxable income** = printed 11 − printed 14, floored at 0.
    pub line15: Usd,
    /// L16 — the regular tax (Tax Table / TCW / QDCGT, per `method.rs`).
    pub line16: Usd,
    /// L17 — **Schedule 2's printed line 3** (Part I). Always 0 in v1 — see [`Schedule2Lines`].
    pub line17: Usd,
    /// L18 — add **printed** 16 and 17.
    pub line18: Usd,
    /// L19 — CTC / credit for other dependents. **Always 0** (a §3.4 conservative omission; the
    /// `CtcOdcOmitted` advisory tells the filer their tax is overstated).
    pub line19: Usd,
    /// L20 — **Schedule 3's printed line 8** (nonrefundable credits: the FTC).
    pub line20: Usd,
    /// L21 — add **printed** 19 and 20.
    pub line21: Usd,
    /// L22 — printed 18 − printed 21, floored at 0.
    pub line22: Usd,
    /// L23 — **Schedule 2's printed line 21** (other taxes).
    pub line23: Usd,
    /// L24 — **TOTAL TAX** = add **printed** 22 and 23.
    pub line24: Usd,
    /// L25a — federal income tax withheld from Form(s) W-2 (Σ box 2).
    pub line25a: Usd,
    /// L25b — withheld from Form(s) 1099 (Σ box 4).
    pub line25b: Usd,
    /// L25c — withheld from other forms: **Form 8959's printed line 24** (the Additional-Medicare
    /// over-withholding credit) plus any other declared withholding.
    pub line25c: Usd,
    /// L25d — add **printed** 25a, 25b and 25c.
    pub line25d: Usd,
    /// L26 — estimated tax payments.
    pub line26: Usd,
    /// L31 — **Schedule 3's printed line 15** (other payments: extension + excess Social Security).
    pub line31: Usd,
    /// L32 — total other payments and refundable credits ⇒ `= line31` (lines 27–30 are blank: the EIC
    /// is a §3.4 conservative omission, and the rest are unrepresentable).
    pub line32: Usd,
    /// L33 — **TOTAL PAYMENTS** = add **printed** 25d, 26 and 32.
    pub line33: Usd,
    /// L34 / L35a — the overpayment refunded. Zero when the return owes. v1 never fills the
    /// direct-deposit block (35b–35d), so a refund arrives as a paper check — the `RefundByPaperCheck`
    /// advisory says so.
    pub line34: Usd,
    /// L37 — the amount owed. Zero when the return is due a refund.
    pub line37: Usd,
    /// The **Digital Asset question**. `true` for any crypto disposal, income, gift or donation.
    /// btctax never answers \"No\" — a \"No\" it cannot vouch for is worse than leaving the question to
    /// the filer, so this is `true` or the question is left for them.
    pub digital_asset_yes: bool,
}

/// The 1040's **income block** (lines 1a–11), printed.
///
/// Extracted from [`form_1040_lines`] because **Schedule A line 2 cites it**: \"Enter amount from Form
/// 1040 or 1040-SR, **line 11**\". Under the §3.1 citation-composition rule that is a SOURCE citation, so
/// Schedule A must take the PRINTED L11 — not a re-rounding of the exact AGI, which can differ by
/// dollars and, on a negative-AGI itemizer, prints 0 beside a negative 1040 L11 (Fable P6 r1 I5).
///
/// There is no cycle: L11 = L9 − L10 depends on Schedules B/1/D, never on Schedule A (which lands at
/// L12). So the income block is derived first, Schedule A composes on its L11, and the rest of the 1040
/// consumes both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Form1040Income {
    /// The QDCGT worksheet's **line 3** operand: `min(printed Sch D L15, printed Sch D L16)`, floored at
    /// 0 — the §1(h) preferential net capital gain **as a human reads it off the FILED Schedule D**
    /// (`qdcgt_line16`'s own doc: \"`net_ltcg` = `min(Sch D 15,16)`, ≥0; L3\").
    ///
    /// ★ It is NOT `round_dollar(exact net_ltcg)` (Fable P6 r2 NEW-I1). The printed Sch D L15/L16 are sums
    /// of printed operands and drift from the exact figure — that is what the 302≠301 8949 KAT is about.
    /// A $1 drift here shifts the worksheet's ordinary remainder by $1, and THAT feeds the same $50-tread
    /// Tax Table step function line 16 was already gated on: across a bin edge the filed L16 disagrees
    /// with the worksheet a filer (or the Service) computes from the filed Schedule D, by a whole bin.
    pub qdcgt_net_capital_gain: Usd,
    pub line1a: Usd,
    pub line1z: Usd,
    pub line2a: Usd,
    pub line2b: Usd,
    pub line3a: Usd,
    pub line3b: Usd,
    pub line7: Usd,
    pub line8: Usd,
    pub line9: Usd,
    pub line10: Usd,
    pub line11: Usd,
}

/// Derive the 1040's printed income block (lines 1a–11).
pub fn form_1040_income_lines(
    ar: &AbsoluteReturn,
    sch_b: Option<&ScheduleBLines>,
    sch_1: Option<&Schedule1Lines>,
    sch_d: &ScheduleDLines,
) -> Form1040Income {
    // L1a is the Σ of W-2 box 1, and L1z is \"Add lines 1a through 1h\" — v1 has no 1b–1h, so they are
    // the same figure, and BOTH cells print. A filled 1z above a blank 1a does not add up.
    let line1a = round_dollar(ar.wages);
    let line1z = line1a;
    let line2a = round_dollar(ar.printed_inputs.tax_exempt_interest);
    let line2b = sch_b.map_or_else(|| round_dollar(ar.taxable_interest), |b| b.line4);
    let line3a = round_dollar(ar.qualified_dividends);
    let line3b = sch_b.map_or_else(|| round_dollar(ar.ordinary_dividends), |b| b.line6);

    // L7 — signed, with a LEADING MINUS (not parentheses; SPEC §3.2). On a net-loss year it is the
    // §1211(b)-LIMITED amount from Schedule D line 21, not the full loss.
    let line7 = match sch_d.routing {
        ScheduleDRouting::NetLoss { line21, .. } => -line21,
        _ => sch_d.line16,
    };
    let line8 = sch_1.map_or(Usd::ZERO, |s| s.line10);
    let line9 = line1z + line2b + line3b + line7 + line8; // ★ sums the PRINTED lines
    let line10 = sch_1.map_or(Usd::ZERO, |s| s.line26);
    let line11 = line9 - line10;

    Form1040Income {
        // The worksheet reads these OFF THE FILED SCHEDULE D — printed lines 15 and 16, smaller of, ≥ 0.
        qdcgt_net_capital_gain: sch_d.line15.min(sch_d.line16).max(Usd::ZERO),
        line1a,
        line1z,
        line2a,
        line2b,
        line3a,
        line3b,
        line7,
        line8,
        line9,
        line10,
        line11,
    }
}

/// Derive the printed Form 1040 chain. Every schedule figure is taken from that schedule's **printed**
/// chain, so the return ties out against its own attachments.
///
/// `standard_or_itemized` is the deduction actually claimed (line 12): Schedule A's printed line 17 if
/// the return itemizes, else `round_dollar` of the §63 standard deduction.
#[allow(clippy::too_many_arguments)]
pub fn form_1040_lines(
    ar: &AbsoluteReturn,
    income: &Form1040Income,
    sch_a: Option<&ScheduleALines>,
    sch_2: Option<&Schedule2Lines>,
    sch_3: Option<&Schedule3Lines>,
    f8959: &Form8959Lines,
    f8995: Option<&crate::tax::qbi::Form8995Lines>,
    table: &crate::tax::tables::TaxTable,
    status: FilingStatus,
    other_withholding: Usd,
    estimated_payments: Usd,
    digital_asset_yes: bool,
) -> Form1040Lines {
    // ── Income — from the printed block (Schedule A already composed on its L11). ───────────────
    let Form1040Income {
        qdcgt_net_capital_gain,
        line1a,
        line1z,
        line2a,
        line2b,
        line3a,
        line3b,
        line7,
        line8,
        line9,
        line10,
        line11,
    } = *income;

    // ── Deductions → taxable income ─────────────────────────────────────────────────────────────
    let line12 = match sch_a {
        Some(a) => a.line17, // itemizing — Schedule A's PRINTED total
        None => round_dollar(ar.deduction),
    };
    let line13 = f8995.map_or(Usd::ZERO, |q| q.line15);
    // L14 — \"Add lines 12e, 13a, and 13b\" (2025) / \"Add lines 12 and 13\" (2024). The third term is
    // Schedule 1-A's total, which does not exist on the 2024 form.
    let line14 = line12 + line13 + round_dollar(ar.schedule_1a_additional);
    let line15 = (line11 - line14).max(Usd::ZERO);

    // ── Tax → total tax ─────────────────────────────────────────────────────────────────────────
    // ★ L16 is the Tax Table / QDCGT worksheet applied to the PRINTED line 15 — NOT a re-rounding of the
    // tax computed on the exact-cents taxable income (Fable P6 r1 I2). The Tax Table is a STEP function
    // with $50 treads: when the exact TI and the printed L15 straddle a bin edge, the two differ by a
    // whole bin step (up to ~$18.50 at the top rate), not by the $1 rounding residual §3.1 tolerates —
    // and \"L16 vs Table(L15)\" is the single most-recomputed arithmetic on a transcribed return. The
    // exact-cents `ar.regular_tax` remains the COMPUTED liability; only the FILED cell changes.
    let line16 = crate::tax::method::qdcgt_line16(
        table.ordinary_for(status),
        table.ltcg_for(status),
        line15,
        line3a,
        qdcgt_net_capital_gain, // ★ the PRINTED Schedule D figure, not round(exact) — r2 NEW-I1
    );
    let line17 = Usd::ZERO; // Schedule 2 Part I is blank in v1 (see Schedule2Lines)
    let line18 = line16 + line17;
    let line19 = Usd::ZERO; // ★ CTC/ODC — a §3.4 conservative omission (advisory fires)
    let line20 = sch_3.map_or(Usd::ZERO, |s| s.line8);
    let line21 = line19 + line20;
    let line22 = (line18 - line21).max(Usd::ZERO);
    let line23 = sch_2.map_or(Usd::ZERO, |s| s.line21);
    let line24 = line22 + line23; // ★ TOTAL TAX, from the PRINTED lines

    // ── Payments ────────────────────────────────────────────────────────────────────────────────
    let line25a = round_dollar(ar.withholding_25a);
    let line25b = round_dollar(ar.withholding_25b);
    // 25c carries Form 8959's PRINTED line 24 — the Additional-Medicare over-withholding credit.
    let line25c = f8959.line24 + round_dollar(other_withholding);
    let line25d = line25a + line25b + line25c;
    let line26 = round_dollar(estimated_payments);
    let line31 = sch_3.map_or(Usd::ZERO, |s| s.line15);
    let line32 = line31; // 27-30 blank (EIC omitted conservatively; rest unrepresentable)
    let line33 = line25d + line26 + line32; // ★ TOTAL PAYMENTS, from the PRINTED lines

    // ── Refund or owed — from the PRINTED total tax and the PRINTED total payments, so the bottom
    //    line of the filed form is the one a reader re-adding the column arrives at.
    let line34 = (line33 - line24).max(Usd::ZERO);
    let line37 = (line24 - line33).max(Usd::ZERO);

    Form1040Lines {","numLines":260,"startLine":440,"totalLines":2992}},"sourceToolAssistantUUID":"ccf5a1d9-c329-4698-97c9-20a3fda691ae","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"d108de38-49d5-4b33-b24a-4d1cf4736f8d","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNEHwqd4rjHuqUp1RJU","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAIS7wMKiAEIEBgCKkDxkbM2RBKIrOU6rhd74JSzQQ9DoDrO133EuQqG3CXJLYtrlsxSYv1pNLJxXFokFeAgUA8J6zK7W9bX4cwk8shNMg5jbGF1ZGUtZmFibGUtNTgBQgh0aGlua2luZ1okZTJmYmFjOTAtMDM4ZC00MjYzLWFjMDUtZWUxNTVlMGNjZTg2Egz48sRgtDWtY1cd40YaDCaBwp/G/VQh7iwCqCIwdIZJMFEdxFYNRGDfdIT8qtVULSuqsaAk04kqiRJf9d1JgA/eHo9jaOfL7O4rtC4vKpMClqmvH6CeGyAZe4zWrXMRRYtbHIYyFgXzxUlq1x/eidJmgYrJ1Nd+9CIQoKD1cAJVfqxaliXZ6kdTxrUEyBfw8xQYK9uPqMja7BX5HPL2wpwUKwXyGglkAXUKNhi9ELHqpPuNCBPJGzOYeb0O2Qo0kxxlEc3Mujp4is/vXtNu8TDIYW1CSbrslo+g1icQvuS++s/BW0vW2RwdpTOcBwf5AfVjkbxTKjhfttynziJ5+DnFWgB+w7NEQi/yVK+ppOriEzg+VfeLmBxm8/hwMn9pYmFrIfrHF5Sx1y7I8BanFKgJV1YjAg1cMdfcIIEHQqnLe8jt9KIk/ITNOTxZbbjAs4LH65AeS3o4c1sOlTyHB6C4rK8YAQ=="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":6528,"cache_read_input_tokens":81747,"cache_creation":{"ephemeral_5m_input_tokens":6528,"ephemeral_1h_input_tokens":0},"output_tokens":5,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNEG8CyiAnYupTC45bo","attributionAgent":"general-purpose","type":"assistant","uuid":"aef8547e-babd-41ca-a9b8-a48eea755452","timestamp":"2026-08-01T04:05:08.283Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"aef8547e-babd-41ca-a9b8-a48eea755452","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNEHwqd4rjHuqUp1RJU","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01BZF5eQ4pAyvAj4P9TrfG9E","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/crates/btctax-forms/src/form8995.rs","offset":200,"limit":110},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":6528,"cache_read_input_tokens":81747,"cache_creation":{"ephemeral_5m_input_tokens":6528,"ephemeral_1h_input_tokens":0},"output_tokens":5,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNEG8CyiAnYupTC45bo","attributionAgent":"general-purpose","type":"assistant","uuid":"b663bf7d-8fff-4c02-8e5d-1a8fd509a620","timestamp":"2026-08-01T04:05:09.630Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"b663bf7d-8fff-4c02-8e5d-1a8fd509a620","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01BZF5eQ4pAyvAj4P9TrfG9E","type":"tool_result","content":"200\t        let tin_fqn = map.row1_tin.fields()[0];
201\t        let tin_max_len = blank_fields
202\t            .iter()
203\t            .find(|f| f.fqn == tin_fqn)
204\t            .ok_or_else(|| FormsError::MapFieldMissing(tin_fqn.to_string()))?
205\t            .max_len;
206\t        push_literal(
207\t            &mut writes,
208\t            &mut placements,
209\t            &map.row1_tin,
210\t            &render_ssn(&proprietor.ssn, tin_max_len)?,
211\t            F8995_COL_ROW1_TIN,
212\t        );
213\t        // (c) this business's QBI. With one business the column total IS the row, so it is written from
214\t        // the same value as line 2 — they cannot disagree.
215\t        push_money(
216\t            &mut writes,
217\t            &mut placements,
218\t            &map.row1_qbi,
219\t            lines.line2,
220\t            F8995_COL_ROW1_QBI,
221\t            Some((0, 0)),
222\t        );
223\t    }
224\t
225\t    // Parallel to `map.lines()` — printed reading order, strictly descending y on page 1.
226\t    let plan: [(Usd, usize); 16] = [
227\t        (lines.line2, F8995_COL_MID),     // 2  total QBI (table blank ⇒ 0)
228\t        (lines.line3, F8995_COL_MID), // 3  ★ paren — prior-year QBI loss carryforward (magnitude)
229\t        (lines.line4, F8995_COL_MID), // 4  combine 2 and 3
230\t        (lines.line5, F8995_COL_AMOUNT), // 5  QBI component
231\t        (lines.line6, F8995_COL_MID), // 6  qualified REIT dividends + PTP income
232\t        (lines.line7, F8995_COL_MID), // 7  ★ paren — prior-year loss carryforward (magnitude)
233\t        (lines.line8, F8995_COL_MID), // 8  combine 6 and 7
234\t        (lines.line9, F8995_COL_AMOUNT), // 9  REIT/PTP component
235\t        (lines.line10, F8995_COL_AMOUNT), // 10 add 5 and 9
236\t        (lines.line11, F8995_COL_MID), // 11 taxable income before QBI
237\t        (lines.line12, F8995_COL_MID), // 12 net capital gain + qualified dividends
238\t        (lines.line13, F8995_COL_MID), // 13 11 - 12, floored
239\t        (lines.line14, F8995_COL_AMOUNT), // 14 income limitation
240\t        (lines.line15, F8995_COL_AMOUNT), // 15 the deduction -> 1040 L13
241\t        (lines.line16, F8995_COL_AMOUNT), // 16 ★ paren — QB loss carryforward (magnitude)
242\t        (lines.line17, F8995_COL_AMOUNT), // 17 ★ paren — REIT/PTP loss carryforward (magnitude)
243\t    ];
244\t    for (ord, (cell, (value, col))) in map.lines().iter().zip(plan).enumerate() {
245\t        // ★★ LINE 3 IS WRITTEN ONLY WHEN THERE IS A CARRYFORWARD. It is `Qualified business net (loss)
246\t        // carryforward FROM THE PRIOR YEAR` — a fact the filer supplies about a return btctax did not
247\t        // compute. A printed `0` there is an affirmative sworn statement that they had no prior-year
248\t        // QBI loss; a BLANK is no statement at all, and blank is the truthful mark for the
249\t        // overwhelming majority of filers, who simply have none. (CLAUDE.md: \"most fields on a tax
250\t        // return are blank, intentionally\"; \"writing `0` on an unasked line fabricates testimony\".)
251\t        //
252\t        // Every other line here is DERIVED from figures already on the page, so a zero is btctax's own
253\t        // arithmetic and prints legitimately. Line 3 is the only line on this form that is neither
254\t        // derived nor computed — it is testimony — which is exactly why it is the only one gated.
255\t        if std::ptr::eq(*cell, &map.line3) && value.is_zero() {
256\t            continue;
257\t        }
258\t        push_money(
259\t            &mut writes,
260\t            &mut placements,
261\t            cell,
262\t            value,
263\t            col,
264\t            // +1: row 1i(c) took ordinal 0, and it sits above line 2 on the page.
265\t            Some((0, ord as u32 + 1)),
266\t        );
267\t    }
268\t
269\t    // Identity header (P6.2): `push_identity` reads the SSN cell's own /MaxLen to decide
270\t    // hyphenated-vs-digits, so it needs the blank PDF's fields.
271\t    push_identity(
272\t        &mut writes,
273\t        &mut placements,
274\t        &map.identity,
275\t        &header.name_line,
276\t        &header.taxpayer.ssn,
277\t        &blank_fields,
278\t    )?;
279\t    let index = pdf::index(&blank_fields);
280\t    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
281\t    pdf::apply_writes(&mut doc, &index, &writes)?;
282\t    pdf::strip_nondeterminism(&mut doc);
283\t    let bytes = pdf::save(&mut doc)?;
284\t
285\t    // True read-back: re-parse the SERIALIZED output and verify geometry against the PDF's own rects.
286\t    let check = pdf::load(&bytes)?;
287\t    let fields = pdf::collect_fields(&check)?;
288\t    verify_flat(&check, &fields, &placements, F8995_CLUSTERS)?;
289\t    Ok(bytes)
290\t}
291\t"}]},"uuid":"3eb9f1e4-c806-4811-aa63-336d9b1d03cf","timestamp":"2026-08-01T04:05:09.638Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/crates/btctax-forms/src/form8995.rs","content":"        let tin_fqn = map.row1_tin.fields()[0];
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
            &mut writes,
            &mut placements,
            &map.row1_qbi,
            lines.line2,
            F8995_COL_ROW1_QBI,
            Some((0, 0)),
        );
    }

    // Parallel to `map.lines()` — printed reading order, strictly descending y on page 1.
    let plan: [(Usd, usize); 16] = [
        (lines.line2, F8995_COL_MID),     // 2  total QBI (table blank ⇒ 0)
        (lines.line3, F8995_COL_MID), // 3  ★ paren — prior-year QBI loss carryforward (magnitude)
        (lines.line4, F8995_COL_MID), // 4  combine 2 and 3
        (lines.line5, F8995_COL_AMOUNT), // 5  QBI component
        (lines.line6, F8995_COL_MID), // 6  qualified REIT dividends + PTP income
        (lines.line7, F8995_COL_MID), // 7  ★ paren — prior-year loss carryforward (magnitude)
        (lines.line8, F8995_COL_MID), // 8  combine 6 and 7
        (lines.line9, F8995_COL_AMOUNT), // 9  REIT/PTP component
        (lines.line10, F8995_COL_AMOUNT), // 10 add 5 and 9
        (lines.line11, F8995_COL_MID), // 11 taxable income before QBI
        (lines.line12, F8995_COL_MID), // 12 net capital gain + qualified dividends
        (lines.line13, F8995_COL_MID), // 13 11 - 12, floored
        (lines.line14, F8995_COL_AMOUNT), // 14 income limitation
        (lines.line15, F8995_COL_AMOUNT), // 15 the deduction -> 1040 L13
        (lines.line16, F8995_COL_AMOUNT), // 16 ★ paren — QB loss carryforward (magnitude)
        (lines.line17, F8995_COL_AMOUNT), // 17 ★ paren — REIT/PTP loss carryforward (magnitude)
    ];
    for (ord, (cell, (value, col))) in map.lines().iter().zip(plan).enumerate() {
        // ★★ LINE 3 IS WRITTEN ONLY WHEN THERE IS A CARRYFORWARD. It is `Qualified business net (loss)
        // carryforward FROM THE PRIOR YEAR` — a fact the filer supplies about a return btctax did not
        // compute. A printed `0` there is an affirmative sworn statement that they had no prior-year
        // QBI loss; a BLANK is no statement at all, and blank is the truthful mark for the
        // overwhelming majority of filers, who simply have none. (CLAUDE.md: \"most fields on a tax
        // return are blank, intentionally\"; \"writing `0` on an unasked line fabricates testimony\".)
        //
        // Every other line here is DERIVED from figures already on the page, so a zero is btctax's own
        // arithmetic and prints legitimately. Line 3 is the only line on this form that is neither
        // derived nor computed — it is testimony — which is exactly why it is the only one gated.
        if std::ptr::eq(*cell, &map.line3) && value.is_zero() {
            continue;
        }
        push_money(
            &mut writes,
            &mut placements,
            cell,
            value,
            col,
            // +1: row 1i(c) took ordinal 0, and it sits above line 2 on the page.
            Some((0, ord as u32 + 1)),
        );
    }

    // Identity header (P6.2): `push_identity` reads the SSN cell's own /MaxLen to decide
    // hyphenated-vs-digits, so it needs the blank PDF's fields.
    push_identity(
        &mut writes,
        &mut placements,
        &map.identity,
        &header.name_line,
        &header.taxpayer.ssn,
        &blank_fields,
    )?;
    let index = pdf::index(&blank_fields);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // True read-back: re-parse the SERIALIZED output and verify geometry against the PDF's own rects.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, F8995_CLUSTERS)?;
    Ok(bytes)
}
","numLines":92,"startLine":200,"totalLines":291}},"sourceToolAssistantUUID":"b663bf7d-8fff-4c02-8e5d-1a8fd509a620","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"3eb9f1e4-c806-4811-aa63-336d9b1d03cf","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNEHwqd4rjHuqUp1RJU","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01KvAXy9VmuiYKMXVKprJJ3m","name":"Bash","input":{"command":"ls /scratch/code/bitcoin_tax/design/forms/extract/ | head -30; echo ---; grep -n \"pub enum MoneyCell\\|pub struct MoneyPair\" /scratch/code/bitcoin_tax/crates/btctax-forms/src/map.rs","description":"List form extracts and find MoneyCell definition"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":6528,"cache_read_input_tokens":81747,"output_tokens":410,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":6528},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":410,"cache_read_input_tokens":81747,"cache_creation_input_tokens":6528,"cache_creation":{"ephemeral_5m_input_tokens":6528,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbNEG8CyiAnYupTC45bo","attributionAgent":"general-purpose","type":"assistant","uuid":"1251fe3e-068c-456d-aefb-50a81feda305","timestamp":"2026-08-01T04:05:10.732Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"1251fe3e-068c-456d-aefb-50a81feda305","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01KvAXy9VmuiYKMXVKprJJ3m","type":"tool_result","content":"f1040--2024.txt
f1040--2025.txt
f1040s1a--2025.txt
f1040s2--2024.txt
f1040s2--2025.txt
f1040s3--2024.txt
f1040s3--2025.txt
f1040sa--2024.txt
f1040sa--2025.txt
f1040sb--2024.txt
f1040sb--2025.txt
f1040sc--2024.txt
f1040sc--2025.txt
f1040sd--2024.txt
f1040sd--2025.txt
f1040sse--2024.txt
f1040sse--2025.txt
f6251--2024.txt
f6251--2025.txt
f8275--2024.txt
f8275--periodic.txt
f8283--2025.txt
f8283--periodic.txt
f8949--2024.txt
f8949--2025.txt
f8959--2024.txt
f8959--2025.txt
f8960--2024.txt
f8960--2025.txt
f8995--2024.txt
---
199:pub struct MoneyPair {
212:pub enum MoneyCell {","is_error":false}]},"uuid":"a4f25a35-5084-4a7d-91bb-13ae3a8e6a10","timestamp":"2026-08-01T04:05:10.832Z","toolUseResult":{"stdout":"f1040--2024.txt
f1040--2025.txt
f1040s1a--2025.txt
f1040s2--2024.txt
f1040s2--2025.txt
f1040s3--2024.txt
f1040s3--2025.txt
f1040sa--2024.txt
f1040sa--2025.txt
f1040sb--2024.txt
f1040sb--2025.txt
f1040sc--2024.txt
f1040sc--2025.txt
f1040sd--2024.txt
f1040sd--2025.txt
f1040sse--2024.txt
f1040sse--2025.txt
f6251--2024.txt
f6251--2025.txt
f8275--2024.txt
f8275--periodic.txt
f8283--2025.txt
f8283--periodic.txt
f8949--2024.txt
f8949--2025.txt
f8959--2024.txt
f8959--2025.txt
f8960--2024.txt
f8960--2025.txt
f8995--2024.txt
---
199:pub struct MoneyPair {
212:pub enum MoneyCell {","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"1251fe3e-068c-456d-aefb-50a81feda305","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"a4f25a35-5084-4a7d-91bb-13ae3a8e6a10","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNEkFdRQJouuVX7Z1p9","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAIS/AMKiAEIEBgCKkCtlZidngIivqYMljIQqMw12OnIj+MURMLsodT7lZHkJCuW6Wcy/BMCQO0tbA9kCcUF5VCFkIUEJJfGgA8uxSXDMg5jbGF1ZGUtZmFibGUtNTgBQgh0aGlua2luZ1okZTJmYmFjOTAtMDM4ZC00MjYzLWFjMDUtZWUxNTVlMGNjZTg2Egz+VAQmaliGtLaNJ3oaDGkDZhIysVyWqA5BqyIwyNyQGVplFzKQGYsO34fFgtDoXginVzpwFlYy4nfiyO/3GPOLd67vIq1oCLMV4aIPKqACHXLc2E3NxPcJq3bPvXJNyeKL0uy0NKlfp95PHGGt3+RKKDaukqUHE2Qi9m3aZ4v1+ioNVxfBNOwTWnzPuJ7IpyLA7zvJtf7EANbGl381NUI35p29tac0UZWZaMOgqk7kA5IKD/1qV9FXc63FiK0Ro9FhFDD3WR8ccuogPppXKHOT+sAUkRuaHtR1y70j30CqmULjMftB6qt+xEqL0cTiTzWlK6NmsxhZHeq9pTNsZi/tS+MlLBNT/nIc6Mx1qF5LJb5cBhcl26zO1ySyhDzDKVvJWuXVIGDhqrFATNig4Q0hjmISPKJAGmWrB9fdRDWJ272z16YfR9Y42umgP2C2ezwSjZyLUJk3xaSLyK3ormy7jfSaJm5JhU60C1pZKzWUGAE="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2868,"cache_read_input_tokens":88275,"cache_creation":{"ephemeral_5m_input_tokens":2868,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNEgHWPmXnRkfuriGQv","attributionAgent":"general-purpose","type":"assistant","uuid":"1d7bc258-d90e-43ac-bea7-5ba4962f6b87","timestamp":"2026-08-01T04:05:14.879Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"1d7bc258-d90e-43ac-bea7-5ba4962f6b87","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNEkFdRQJouuVX7Z1p9","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_0138NVzYcxUzHZtVfusgLneX","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_inputs.rs","offset":480,"limit":180},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2868,"cache_read_input_tokens":88275,"cache_creation":{"ephemeral_5m_input_tokens":2868,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNEgHWPmXnRkfuriGQv","attributionAgent":"general-purpose","type":"assistant","uuid":"95e6f8da-d4b7-4325-8631-127628b718ac","timestamp":"2026-08-01T04:05:16.079Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"95e6f8da-d4b7-4325-8631-127628b718ac","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_0138NVzYcxUzHZtVfusgLneX","type":"tool_result","content":"480\t    /// ★ `live` deliberately KEEPS its one-argument signature and reads this field, rather than
481\t    /// becoming `live(year, &ri)` — a year passed alongside invites a caller to pass one that
482\t    /// disagrees with the row's own origin, which is a second copy of the truth in flight.
483\t    ///
484\t    /// ★★ **`0` means NOT STATED**, and it is not a usable year: the storage boundary stamps this
485\t    /// from the row key on read and refuses a disagreement on write, so an in-memory value and the
486\t    /// row it came from can never diverge. `Default` yields `0` because a defaulted `ReturnInputs` is
487\t    /// a test convenience that must not fabricate a year any more than it fabricates an answer.
488\t    #[serde(default)]
489\t    pub tax_year: i32,
490\t    pub filing_status: FilingStatus,
491\t    #[serde(default)]
492\t    pub header: HouseholdHeader,
493\t    #[serde(default)]
494\t    pub w2s: Vec<W2>,
495\t    #[serde(default)]
496\t    pub int_1099: Vec<Form1099Int>,
497\t    #[serde(default)]
498\t    pub div_1099: Vec<Form1099Div>,
499\t    #[serde(default)]
500\t    pub g_1099: Vec<Form1099G>,
501\t    #[serde(default)]
502\t    pub schedule_c: Option<ScheduleCInputs>,
503\t    #[serde(default)]
504\t    pub schedule_a: Option<ScheduleAInputs>, // None ⇒ standard deduction
505\t    #[serde(default)]
506\t    pub itemize_election: ItemizeElection,
507\t    /// REQUIRED iff MFS: does the spouse itemize? `None` ⇒ fail-loud (§63(c)(6)/G15).
508\t    #[serde(default)]
509\t    pub mfs_spouse_itemizes: Option<bool>,
510\t    #[serde(default)]
511\t    pub sch1: Schedule1Inputs,
512\t    #[serde(default)]
513\t    pub payments: Payments,
514\t    #[serde(default)]
515\t    pub capital_loss_carryforward_in: Carryforward,
516\t    /// ★★ §G-20a — provenance for [`Self::capital_loss_carryforward_in`], as a SIBLING scalar.
517\t    ///
518\t    /// It is not inside `Carryforward` because that type lives in the **frozen** `tax/types.rs` (the
519\t    /// crypto-delta engine), and a third `frozen_guard` pin exception for a field the delta path never
520\t    /// reads would be a poor trade. Semantically it belongs here anyway: the provenance is a fact about
521\t    /// this YEAR'S INPUT, not about the value type.
522\t    ///
523\t    /// **Why it exists at all:** without it a zero is uninterpretable — *\"the filer has no carryover\"*
524\t    /// and *\"nobody ever asked\"* are the same bytes. `Computed` means btctax derived it from a prior
525\t    /// year it actually computed; `User` (the default) means it is the filer's or nobody's.
526\t    #[serde(default)]
527\t    pub capital_loss_carryforward_in_provenance: CarryProvenance,
528\t    /// **Form 6251 line 2k — the AMT capital-loss-carryover declaration.** Line 2k is \"Disposition of
529\t    /// property (difference between AMT and regular tax gain or loss)\", and i6251 directs any Form
530\t    /// 8949 / Schedule D / 4684 / 4797 adjustment for the activity there rather than to line 3.
531\t    /// btctax carries only the REGULAR-tax carryforward and cannot know whether an AMT twin diverges.
532\t    ///
533\t    /// A class-(A) DECLARATION, phrased so `true` is the AMT-neutral answer. `None` ⇒ refuse (never
534\t    /// asked); `Some(false)` — \"my AMT carryover differs\" — ⇒ ALSO refuse, because v1 models no
535\t    /// divergence and proceeding would UNDERSTATE the tax.
536\t    ///
537\t    /// Lives on `ReturnInputs`, not inside [`Carryforward`], because that type is a shared value also
538\t    /// used by `TaxProfile`; the declaration is a property of the RETURN, not of the amount.
539\t    #[serde(default)]
540\t    pub amt_carryover_same_as_regular: Option<bool>,
541\t
542\t    /// Form 6251 **line 2l** — \"is the depreciation inside my Schedule C expenses the same for the AMT?\"
543\t    ///
544\t    /// Lives on `ReturnInputs` rather than on [`ScheduleCInputs`] to match its line-2k sibling above: the
545\t    /// declaration is a property of the RETURN, not of the business. (It is also why the liveness
546\t    /// predicate, not the field's location, is what ties it to Schedule C.)
547\t    #[serde(default)]
548\t    pub amt_depreciation_same_as_regular: Option<bool>,
549\t    #[serde(default)]
550\t    pub charitable_carryover_in: Vec<CharitableCarryItem>,
551\t    /// ★★ §G-20a — provenance for the charitable carryover **LIST AS A WHOLE**.
552\t    ///
553\t    /// `CharitableCarryItem` already carries a per-item provenance, which is useless for the case that
554\t    /// matters: an **EMPTY vec has no items**, so an empty list carries no provenance at all — and an
555\t    /// empty list is exactly the state that is ambiguous between \"no carryover\" and \"never asked\".
556\t    #[serde(default)]
557\t    pub charitable_carryover_in_provenance: CarryProvenance,
558\t    #[serde(default)]
559\t    pub qbi: QbiInputs,
560\t    /// Schedule B Part III — required when Sch B files; `None` ⇒ fail-loud (I7).
561\t    #[serde(default)]
562\t    pub foreign_accounts: Option<bool>,
563\t    /// `Some(true)` ⇒ refuse (Form 3520, R2-I3).
564\t    #[serde(default)]
565\t    pub foreign_trust: Option<bool>,
566\t    #[serde(default)]
567\t    pub foreign_country_names: String,
568\t    /// **Schedule B line 7a's UNNUMBERED sub-question** — *\"If 'Yes,' are you required to file FinCEN
569\t    /// Form 114, Report of Foreign Bank and Financial Accounts (FBAR), to report that financial interest
570\t    /// or signature authority?\"*
571\t    ///
572\t    /// ★ A class-**(B)** SKIPPABLE, live only when [`Self::foreign_accounts`] is `Some(true)` — the
573\t    /// form itself conditions it on 7a being \"Yes\". It was class (A) for two commits; `cbe651d`
574\t    /// reversed that, because an unanswered sub-question is a lawful blank and refusing the whole
575\t    /// return over it is the wrong instrument (the skip advisory names the FinCEN exposure instead).
576\t    /// ★ PRE-MERGE M9 — this doc still said \"class-(A) DECLARATION\", which instructs a maintainer to
577\t    /// restore exactly the refusal that reversal deliberately removed. Stays `Option` all the way to
578\t    /// the PDF writer: `None`
579\t    /// (7a was \"No\", so the question is not live) and `Some(false)` (it IS live and the filer answered
580\t    /// \"No\") must print differently — an unwritten pair versus a checked \"No\" box.
581\t    ///
582\t    /// The form's own Caution is why this is not optional: *\"If required, failure to file FinCEN Form
583\t    /// 114 may result in substantial penalties.\"*
584\t    #[serde(default)]
585\t    pub fbar_filing_required: Option<bool>,
586\t    /// 1040 header \"you were a dual-status alien\" — a class-(A) DECLARATION (P9 §2.5), live always. A single
587\t    /// box asserting a fact whose unchecked state we print today from the MFS coupling alone; `None` ⇒ refuse
588\t    /// (`DualStatusAlienUnanswered`), `Some(true)` ⇒ refuse unsupported (§63(c)(6)(B): NRA standard deduction
589\t    /// is zero), `Some(false)` ⇒ proceed.
590\t    /// ★★★ **Form 8283 Section B lines 5a / 5b / 5c** — the three restriction questions, asked ONCE
591\t    /// for the whole return: *\"did any of your donations have strings attached?\"*
592\t    ///
593\t    /// The form asks three things about each donated property, and a **Yes** to any of them shrinks or
594\t    /// kills the §170 deduction (Reg §1.170A-7 — a gift with retained rights is not a gift of the whole
595\t    /// thing). btctax deducts at full fair market value, so a Yes means the number on the return is
596\t    /// **WRONG**, and it refuses rather than compute it.
597\t    ///
598\t    /// ★★ **ONE return-level question, three per-donation boxes — sound, not a shortcut.** The filer
599\t    /// states a UNIVERSAL (\"none of my donations had any of these\"), from which each box's answer
600\t    /// follows for every donation. That is why the prompt ENUMERATES all three limbs in the form's own
601\t    /// words: a \"No\" to something vaguer would be laundered into three specific answers the filer never
602\t    /// gave. Enumerate the YES-conditions, default to refusing, so an omission fails closed.
603\t    ///
604\t    /// ★ It also dissolves §G-21's blocker. The registry is RETURN-shaped and these looked
605\t    /// per-donation; asking the universal makes them return-shaped too, so no per-row machinery is
606\t    /// needed. A **Yes** refuses rather than asking which donation — btctax cannot reduce the right
607\t    /// gift's deduction, so the honest move is to send that year's 8283 to be completed by hand.
608\t    #[serde(default)]
609\t    pub donations_had_restrictions: Option<bool>,
610\t    #[serde(default)]
611\t    pub dual_status_alien: Option<bool>,
612\t
613\t    // ── §164(b)(7)(B)(iv) / Schedule 1-A Part I — the MAGI add-backs ──────────────────────────────
614\t    //
615\t    // ★ The SAME four amounts serve two forms. The §164(b) SALT worksheet adds them at its lines
616\t    //   3a–3d, and Schedule 1-A Part I adds them at its lines 2a–2d, because both use the statute's
617\t    //   modified AGI: \"adjusted gross income increased by any amount excluded from gross income under
618\t    //   section 911, 931, or 933.\" One quantity, five phase-outs (SALT plus Schedule 1-A's four).
619\t    //
620\t    // ★ `Option<Usd>`, NOT defaulted `Usd`, and the distinction is the whole point. A plain `Usd`
621\t    //   defaulting to 0 cannot tell \"zero because the filer has none\" from \"zero because nobody
622\t    //   asked\" — and those differ in direction: an unasked add-back UNDERSTATES MAGI, which RAISES
623\t    //   the SALT and Schedule 1-A deductions. That is a default in the filer's favour that
624\t    //   understates tax, so it is not the class-(B) forgone-benefit case `medical` is.
625\t    //   `None` refuses, but only where a form actually needs the number — see `SaltLimitation`.
626\t    /// **§164(b)(7)(B)(iv) / Schedule 1-A Part I — the exclusion GATE.** A class-(A) DECLARATION:
627\t    /// *\"Did you exclude any income from gross income under §911 (foreign earned income / housing),
628\t    /// §931 (American Samoa) or §933 (Puerto Rico)?\"*
629\t    ///
630\t    /// ★ The gate carries the answered-ness, not the amounts, and that is deliberate. `Option<bool>`
631\t    /// is a leaf the classifier **forbids** `_` on, so it cannot be added without a human classifying
632\t    /// it — whereas `Option<Usd>` is a scalar the `_` rule permits, which would make this convention
633\t    /// again. `None` = never asked ⇒ refused where a form needs MAGI. `Some(false)` ⇒ all four
634\t    /// amounts are zero. `Some(true)` ⇒ the amounts below are the filer's answers.
635\t    #[serde(default)]
636\t    pub has_income_exclusion: Option<bool>,
637\t    /// **Schedule 1-A line 2a / SALT worksheet line 3a** — \"Enter any income from Puerto Rico that
638\t    /// you excluded.\" (§933.) Meaningful iff [`Self::has_income_exclusion`] is `Some(true)`.
639\t    #[serde(default)]
640\t    pub excluded_puerto_rico_income: Usd,
641\t    /// **Schedule 1-A line 2b / SALT worksheet line 3b** — \"Enter the amount from Form 2555, line
642\t    /// 45.\" (§911 foreign earned income exclusion.)
643\t    #[serde(default)]
644\t    pub form_2555_line45: Usd,
645\t    /// **Schedule 1-A line 2c / SALT worksheet line 3c** — \"Enter the amount from Form 2555, line
646\t    /// 50.\" (§911 housing exclusion.)
647\t    #[serde(default)]
648\t    pub form_2555_line50: Usd,
649\t    /// **Schedule 1-A line 2d / SALT worksheet line 3d** — \"Enter the amount from Form 4563, line
650\t    /// 15.\" (§931 American Samoa exclusion.)
651\t    #[serde(default)]
652\t    pub form_4563_line15: Usd,
653\t}
654\t
655\timpl ReturnInputs {
656\t    /// §164(b)(7)(B)(iv) **modified** adjusted gross income — AGI plus the §911/931/933 exclusions.
657\t    /// The same quantity Schedule 1-A Part I line 3 computes.
658\t    ///
659\t    /// `None` when any add-back was never asked, so a caller that genuinely needs MAGI refuses rather"}]},"uuid":"402d0b16-d3fa-4bcf-944e-e3b4fa7e151e","timestamp":"2026-08-01T04:05:16.086Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/return_inputs.rs","content":"    /// ★ `live` deliberately KEEPS its one-argument signature and reads this field, rather than
    /// becoming `live(year, &ri)` — a year passed alongside invites a caller to pass one that
    /// disagrees with the row's own origin, which is a second copy of the truth in flight.
    ///
    /// ★★ **`0` means NOT STATED**, and it is not a usable year: the storage boundary stamps this
    /// from the row key on read and refuses a disagreement on write, so an in-memory value and the
    /// row it came from can never diverge. `Default` yields `0` because a defaulted `ReturnInputs` is
    /// a test convenience that must not fabricate a year any more than it fabricates an answer.
    #[serde(default)]
    pub tax_year: i32,
    pub filing_status: FilingStatus,
    #[serde(default)]
    pub header: HouseholdHeader,
    #[serde(default)]
    pub w2s: Vec<W2>,
    #[serde(default)]
    pub int_1099: Vec<Form1099Int>,
    #[serde(default)]
    pub div_1099: Vec<Form1099Div>,
    #[serde(default)]
    pub g_1099: Vec<Form1099G>,
    #[serde(default)]
    pub schedule_c: Option<ScheduleCInputs>,
    #[serde(default)]
    pub schedule_a: Option<ScheduleAInputs>, // None ⇒ standard deduction
    #[serde(default)]
    pub itemize_election: ItemizeElection,
    /// REQUIRED iff MFS: does the spouse itemize? `None` ⇒ fail-loud (§63(c)(6)/G15).
    #[serde(default)]
    pub mfs_spouse_itemizes: Option<bool>,
    #[serde(default)]
    pub sch1: Schedule1Inputs,
    #[serde(default)]
    pub payments: Payments,
    #[serde(default)]
    pub capital_loss_carryforward_in: Carryforward,
    /// ★★ §G-20a — provenance for [`Self::capital_loss_carryforward_in`], as a SIBLING scalar.
    ///
    /// It is not inside `Carryforward` because that type lives in the **frozen** `tax/types.rs` (the
    /// crypto-delta engine), and a third `frozen_guard` pin exception for a field the delta path never
    /// reads would be a poor trade. Semantically it belongs here anyway: the provenance is a fact about
    /// this YEAR'S INPUT, not about the value type.
    ///
    /// **Why it exists at all:** without it a zero is uninterpretable — *\"the filer has no carryover\"*
    /// and *\"nobody ever asked\"* are the same bytes. `Computed` means btctax derived it from a prior
    /// year it actually computed; `User` (the default) means it is the filer's or nobody's.
    #[serde(default)]
    pub capital_loss_carryforward_in_provenance: CarryProvenance,
    /// **Form 6251 line 2k — the AMT capital-loss-carryover declaration.** Line 2k is \"Disposition of
    /// property (difference between AMT and regular tax gain or loss)\", and i6251 directs any Form
    /// 8949 / Schedule D / 4684 / 4797 adjustment for the activity there rather than to line 3.
    /// btctax carries only the REGULAR-tax carryforward and cannot know whether an AMT twin diverges.
    ///
    /// A class-(A) DECLARATION, phrased so `true` is the AMT-neutral answer. `None` ⇒ refuse (never
    /// asked); `Some(false)` — \"my AMT carryover differs\" — ⇒ ALSO refuse, because v1 models no
    /// divergence and proceeding would UNDERSTATE the tax.
    ///
    /// Lives on `ReturnInputs`, not inside [`Carryforward`], because that type is a shared value also
    /// used by `TaxProfile`; the declaration is a property of the RETURN, not of the amount.
    #[serde(default)]
    pub amt_carryover_same_as_regular: Option<bool>,

    /// Form 6251 **line 2l** — \"is the depreciation inside my Schedule C expenses the same for the AMT?\"
    ///
    /// Lives on `ReturnInputs` rather than on [`ScheduleCInputs`] to match its line-2k sibling above: the
    /// declaration is a property of the RETURN, not of the business. (It is also why the liveness
    /// predicate, not the field's location, is what ties it to Schedule C.)
    #[serde(default)]
    pub amt_depreciation_same_as_regular: Option<bool>,
    #[serde(default)]
    pub charitable_carryover_in: Vec<CharitableCarryItem>,
    /// ★★ §G-20a — provenance for the charitable carryover **LIST AS A WHOLE**.
    ///
    /// `CharitableCarryItem` already carries a per-item provenance, which is useless for the case that
    /// matters: an **EMPTY vec has no items**, so an empty list carries no provenance at all — and an
    /// empty list is exactly the state that is ambiguous between \"no carryover\" and \"never asked\".
    #[serde(default)]
    pub charitable_carryover_in_provenance: CarryProvenance,
    #[serde(default)]
    pub qbi: QbiInputs,
    /// Schedule B Part III — required when Sch B files; `None` ⇒ fail-loud (I7).
    #[serde(default)]
    pub foreign_accounts: Option<bool>,
    /// `Some(true)` ⇒ refuse (Form 3520, R2-I3).
    #[serde(default)]
    pub foreign_trust: Option<bool>,
    #[serde(default)]
    pub foreign_country_names: String,
    /// **Schedule B line 7a's UNNUMBERED sub-question** — *\"If 'Yes,' are you required to file FinCEN
    /// Form 114, Report of Foreign Bank and Financial Accounts (FBAR), to report that financial interest
    /// or signature authority?\"*
    ///
    /// ★ A class-**(B)** SKIPPABLE, live only when [`Self::foreign_accounts`] is `Some(true)` — the
    /// form itself conditions it on 7a being \"Yes\". It was class (A) for two commits; `cbe651d`
    /// reversed that, because an unanswered sub-question is a lawful blank and refusing the whole
    /// return over it is the wrong instrument (the skip advisory names the FinCEN exposure instead).
    /// ★ PRE-MERGE M9 — this doc still said \"class-(A) DECLARATION\", which instructs a maintainer to
    /// restore exactly the refusal that reversal deliberately removed. Stays `Option` all the way to
    /// the PDF writer: `None`
    /// (7a was \"No\", so the question is not live) and `Some(false)` (it IS live and the filer answered
    /// \"No\") must print differently — an unwritten pair versus a checked \"No\" box.
    ///
    /// The form's own Caution is why this is not optional: *\"If required, failure to file FinCEN Form
    /// 114 may result in substantial penalties.\"*
    #[serde(default)]
    pub fbar_filing_required: Option<bool>,
    /// 1040 header \"you were a dual-status alien\" — a class-(A) DECLARATION (P9 §2.5), live always. A single
    /// box asserting a fact whose unchecked state we print today from the MFS coupling alone; `None` ⇒ refuse
    /// (`DualStatusAlienUnanswered`), `Some(true)` ⇒ refuse unsupported (§63(c)(6)(B): NRA standard deduction
    /// is zero), `Some(false)` ⇒ proceed.
    /// ★★★ **Form 8283 Section B lines 5a / 5b / 5c** — the three restriction questions, asked ONCE
    /// for the whole return: *\"did any of your donations have strings attached?\"*
    ///
    /// The form asks three things about each donated property, and a **Yes** to any of them shrinks or
    /// kills the §170 deduction (Reg §1.170A-7 — a gift with retained rights is not a gift of the whole
    /// thing). btctax deducts at full fair market value, so a Yes means the number on the return is
    /// **WRONG**, and it refuses rather than compute it.
    ///
    /// ★★ **ONE return-level question, three per-donation boxes — sound, not a shortcut.** The filer
    /// states a UNIVERSAL (\"none of my donations had any of these\"), from which each box's answer
    /// follows for every donation. That is why the prompt ENUMERATES all three limbs in the form's own
    /// words: a \"No\" to something vaguer would be laundered into three specific answers the filer never
    /// gave. Enumerate the YES-conditions, default to refusing, so an omission fails closed.
    ///
    /// ★ It also dissolves §G-21's blocker. The registry is RETURN-shaped and these looked
    /// per-donation; asking the universal makes them return-shaped too, so no per-row machinery is
    /// needed. A **Yes** refuses rather than asking which donation — btctax cannot reduce the right
    /// gift's deduction, so the honest move is to send that year's 8283 to be completed by hand.
    #[serde(default)]
    pub donations_had_restrictions: Option<bool>,
    #[serde(default)]
    pub dual_status_alien: Option<bool>,

    // ── §164(b)(7)(B)(iv) / Schedule 1-A Part I — the MAGI add-backs ──────────────────────────────
    //
    // ★ The SAME four amounts serve two forms. The §164(b) SALT worksheet adds them at its lines
    //   3a–3d, and Schedule 1-A Part I adds them at its lines 2a–2d, because both use the statute's
    //   modified AGI: \"adjusted gross income increased by any amount excluded from gross income under
    //   section 911, 931, or 933.\" One quantity, five phase-outs (SALT plus Schedule 1-A's four).
    //
    // ★ `Option<Usd>`, NOT defaulted `Usd`, and the distinction is the whole point. A plain `Usd`
    //   defaulting to 0 cannot tell \"zero because the filer has none\" from \"zero because nobody
    //   asked\" — and those differ in direction: an unasked add-back UNDERSTATES MAGI, which RAISES
    //   the SALT and Schedule 1-A deductions. That is a default in the filer's favour that
    //   understates tax, so it is not the class-(B) forgone-benefit case `medical` is.
    //   `None` refuses, but only where a form actually needs the number — see `SaltLimitation`.
    /// **§164(b)(7)(B)(iv) / Schedule 1-A Part I — the exclusion GATE.** A class-(A) DECLARATION:
    /// *\"Did you exclude any income from gross income under §911 (foreign earned income / housing),
    /// §931 (American Samoa) or §933 (Puerto Rico)?\"*
    ///
    /// ★ The gate carries the answered-ness, not the amounts, and that is deliberate. `Option<bool>`
    /// is a leaf the classifier **forbids** `_` on, so it cannot be added without a human classifying
    /// it — whereas `Option<Usd>` is a scalar the `_` rule permits, which would make this convention
    /// again. `None` = never asked ⇒ refused where a form needs MAGI. `Some(false)` ⇒ all four
    /// amounts are zero. `Some(true)` ⇒ the amounts below are the filer's answers.
    #[serde(default)]
    pub has_income_exclusion: Option<bool>,
    /// **Schedule 1-A line 2a / SALT worksheet line 3a** — \"Enter any income from Puerto Rico that
    /// you excluded.\" (§933.) Meaningful iff [`Self::has_income_exclusion`] is `Some(true)`.
    #[serde(default)]
    pub excluded_puerto_rico_income: Usd,
    /// **Schedule 1-A line 2b / SALT worksheet line 3b** — \"Enter the amount from Form 2555, line
    /// 45.\" (§911 foreign earned income exclusion.)
    #[serde(default)]
    pub form_2555_line45: Usd,
    /// **Schedule 1-A line 2c / SALT worksheet line 3c** — \"Enter the amount from Form 2555, line
    /// 50.\" (§911 housing exclusion.)
    #[serde(default)]
    pub form_2555_line50: Usd,
    /// **Schedule 1-A line 2d / SALT worksheet line 3d** — \"Enter the amount from Form 4563, line
    /// 15.\" (§931 American Samoa exclusion.)
    #[serde(default)]
    pub form_4563_line15: Usd,
}

impl ReturnInputs {
    /// §164(b)(7)(B)(iv) **modified** adjusted gross income — AGI plus the §911/931/933 exclusions.
    /// The same quantity Schedule 1-A Part I line 3 computes.
    ///
    /// `None` when any add-back was never asked, so a caller that genuinely needs MAGI refuses rather","numLines":180,"startLine":480,"totalLines":794}},"sourceToolAssistantUUID":"95e6f8da-d4b7-4325-8631-127628b718ac","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"402d0b16-d3fa-4bcf-944e-e3b4fa7e151e","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNF7WZRdc7PGjGLydd9","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISqwYKiAEIEBgCKkAf+5B9WXlyQGhkvt4dvx5ZrsEIwLCeMslAcodtDFXJ6ZdCjpqxDskUAurpvWvaXbObekYPY4iHWB3lQRq5li0JMg5jbGF1ZGUtZmFibGUtNTgBQgh0aGlua2luZ1okZTJmYmFjOTAtMDM4ZC00MjYzLWFjMDUtZWUxNTVlMGNjZTg2EgyfB70A819pDALf078aDG7RsGg9oNMxXT5ZeyIwuH/jcs3ugA4EzPbEUPGTZNIqKyNxsROJc5dO8iK/+j7Q6XAv12Sjh7iuSUblJG9VKs8EzIZmk6YhJiIXD/0xgI81TRC6sclxLJetof1rdXVV8xaOPepAytGfrxDQASfhlyHxTrP+4hYpumL3G4Il8TKlWpC4hZNhSd/dwxnTtElYDSuvkHLZbwglFn32FdAa+L45PsQ+0Zu6dOU5uEY4rczjUeQZtUSI6bjv+Ogok4lxQkibFj4VLJ+2QHMyARv6r8esnMwyQAPrseCDsiRu60RWUVrfNsOa0PAFuMn1r6FTLOE1L5eSi8C6UgDqtC4lm+WVaPBiawUgch/+eC/kq+4lt2I3sGlACiFMH6Cne3d6gWOgpzgYZCszBQ6OANl9HSo2z6pio95VnMh2Ju4r94qiddbeUTHNSXs9Z+fjQCWEir6+b0JvI+j8gH7F3LDCMYKEo4Mu0It3JMrUuHh0dcG+sSuOWSFPUQJRiUayhhPkDvKbOdlSuLnWv4uvRegmRzAcQkNqCmCtIvlexzPOt4O9P8NIDA3l6pftMGFhEuUPG2BO/DUXNPpEckn7tSCYj3lNoupJZIE/otlOJpIg3em4o+vVxrB20/DcgiWw4i/h6ib9S6mdHjhKeGA6OcPnn/62Eneb4znBFFXAPBy75O2F5FvU62VZ6/+YeGod9HG6bzyq8aOC6m1KOq8Pnk0ys5U+yWwqe1znaoXzj9U8NRsWo7uLTze6ONu+vQxkaZwreowRAFHFhvRACbSIxJM9YuCgZSLP2fsFwsVsv9EZCjAIoyBhRzjqdceo/xWsZfAzxlog1x76iFIGiXm4BzlDvrxANrMT2dJiYnG3SCoKZldHGAE="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":5230,"cache_read_input_tokens":91143,"cache_creation":{"ephemeral_5m_input_tokens":5230,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNF5XkbzeTYrNfaTSTv","attributionAgent":"general-purpose","type":"assistant","uuid":"a7faff89-0f6c-4097-804a-20e55f317525","timestamp":"2026-08-01T04:05:22.339Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"a7faff89-0f6c-4097-804a-20e55f317525","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNF7WZRdc7PGjGLydd9","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01QszSU8FwgzCp8SPqAEwq9e","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/printed.rs","offset":1290,"limit":80},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":5230,"cache_read_input_tokens":91143,"cache_creation":{"ephemeral_5m_input_tokens":5230,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNF5XkbzeTYrNfaTSTv","attributionAgent":"general-purpose","type":"assistant","uuid":"cc150c8f-07b2-42e5-ab06-e7c6a3ad5f7c","timestamp":"2026-08-01T04:05:23.711Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"cc150c8f-07b2-42e5-ab06-e7c6a3ad5f7c","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01QszSU8FwgzCp8SPqAEwq9e","type":"tool_result","content":"1290\t/// The printable **Schedule 3 (Additional Credits and Payments)** line chain.
1291\t///
1292\t/// Part I carries only the **foreign tax credit** (line 1). Every other nonrefundable credit on the
1293\t/// schedule — education, dependent-care, saver's, residential-energy, adoption — is a §3.4
1294\t/// *conservative omission*: v1 does not compute it, which can only OVERSTATE tax, and the report
1295\t/// fires a loud advisory ([`crate::tax::advisories::Advisory::OtherCreditsOmitted`]). They are left
1296\t/// BLANK, never a misleading 0.
1297\t///
1298\t/// Part II carries only the **§6413(c) excess Social Security** credit (line 11), computed per person
1299\t/// and never pooled.
1300\t#[derive(Debug, Clone, Copy, PartialEq, Eq)]
1301\tpub struct Schedule3Lines {
1302\t    /// L1 — foreign tax credit (the §904(j) de-minimis election; above the ceiling the return refuses).
1303\t    pub line1: Usd,
1304\t    /// L8 — total nonrefundable credits = add 1 through 4, 5a, 5b and 7 ⇒ `= line1` → 1040 **L20**.
1305\t    pub line8: Usd,
1306\t    /// L10 — \"Amount paid with request for extension to file\". ★ Dropping this line told a filer who had
1307\t    /// ALREADY paid with their extension to pay it again (Fable ARCH-P6.3a D1): it is in the exact
1308\t    /// `total_payments`, so omitting it from the printed chain lowers L31/L33 and RAISES L37 \"amount you
1309\t    /// owe\" by exactly the amount already paid.
1310\t    pub line10: Usd,
1311\t    /// L11 — §6413(c) excess Social Security and tier-1 RRTA tax withheld.
1312\t    pub line11: Usd,
1313\t    /// L15 — total other payments and refundable credits = \"Add lines 9 **through 12** and 14\" ⇒
1314\t    /// `line10 + line11` here (9, 12 and 14 are blank) → 1040 **L31**. Sums the PRINTED lines (§3.1).
1315\t    pub line15: Usd,
1316\t}
1317\t
1318\t/// Derive the printed Schedule 3 chain. Returns `None` when there is neither a foreign tax credit nor
1319\t/// an excess-Social-Security credit — the schedule is then not filed.
1320\tpub fn schedule_3_lines(ar: &AbsoluteReturn) -> Option<Schedule3Lines> {
1321\t    let line1 = round_dollar(ar.foreign_tax_credit);
1322\t    let line8 = line1; // lines 2-4, 5a, 5b, 7 are all conservatively omitted (blank)
1323\t    let line10 = round_dollar(ar.printed_inputs.extension_payment);
1324\t    let line11 = round_dollar(ar.excess_social_security);
1325\t    let line15 = line10 + line11; // \"Add lines 9 through 12 and 14\"; 9, 12 and 14 are blank
1326\t
1327\t    if line8 <= Usd::ZERO && line15 <= Usd::ZERO {
1328\t        return None;
1329\t    }
1330\t    Some(Schedule3Lines {
1331\t        line1,
1332\t        line8,
1333\t        line10,
1334\t        line11,
1335\t        line15,
1336\t    })
1337\t}
1338\t
1339\t#[cfg(test)]
1340\tmod tests {
1341\t    use super::*;
1342\t    use crate::forms::{Form8949Box, Form8949Part, Form8949Row};
1343\t    use crate::tax::other_taxes::form_8959_lines;
1344\t    use crate::tax::se::SeTaxResult;
1345\t    use crate::tax::types::FilingStatus;
1346\t    use rust_decimal_macros::dec;
1347\t    use time::macros::date;
1348\t
1349\t    fn se_300k_single() -> SeTaxResult {
1350\t        SeTaxResult {
1351\t            net_se: dec!(300000),
1352\t            base: dec!(277050.00),
1353\t            ss: dec!(21836.40),
1354\t            medicare: dec!(8034.45),
1355\t            addl: dec!(693.45),
1356\t            total: dec!(30564.30),
1357\t            deductible_half: dec!(14935.42),
1358\t        }
1359\t    }
1360\t
1361\t    /// ★ The printed 1040's L14 is a THREE-term sum, and each term is load-bearing.
1362\t    ///
1363\t    /// 2025: \"Add lines 12e, 13a, and **13b**\". Schedule 1-A is `design/ty2025` B3, so
1364\t    /// `assemble_absolute` still hands over a zero 13b — which means a test driven through
1365\t    /// `assemble_absolute` asserts a trivial identity and every mutation survives it. (It did:
1366\t    /// dropping the 13b term left the suite green.) So this drives the printed path DIRECTLY with a
1367\t    /// nonzero 13b and a nonzero QBI, where each term's absence changes the number.
1368\t    #[test]
1369\t    fn printed_1040_line14_needs_all_three_terms() {"}]},"uuid":"4d78a39d-cfd8-4306-93ce-64fc1170459f","timestamp":"2026-08-01T04:05:23.718Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/crates/btctax-core/src/tax/printed.rs","content":"/// The printable **Schedule 3 (Additional Credits and Payments)** line chain.
///
/// Part I carries only the **foreign tax credit** (line 1). Every other nonrefundable credit on the
/// schedule — education, dependent-care, saver's, residential-energy, adoption — is a §3.4
/// *conservative omission*: v1 does not compute it, which can only OVERSTATE tax, and the report
/// fires a loud advisory ([`crate::tax::advisories::Advisory::OtherCreditsOmitted`]). They are left
/// BLANK, never a misleading 0.
///
/// Part II carries only the **§6413(c) excess Social Security** credit (line 11), computed per person
/// and never pooled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule3Lines {
    /// L1 — foreign tax credit (the §904(j) de-minimis election; above the ceiling the return refuses).
    pub line1: Usd,
    /// L8 — total nonrefundable credits = add 1 through 4, 5a, 5b and 7 ⇒ `= line1` → 1040 **L20**.
    pub line8: Usd,
    /// L10 — \"Amount paid with request for extension to file\". ★ Dropping this line told a filer who had
    /// ALREADY paid with their extension to pay it again (Fable ARCH-P6.3a D1): it is in the exact
    /// `total_payments`, so omitting it from the printed chain lowers L31/L33 and RAISES L37 \"amount you
    /// owe\" by exactly the amount already paid.
    pub line10: Usd,
    /// L11 — §6413(c) excess Social Security and tier-1 RRTA tax withheld.
    pub line11: Usd,
    /// L15 — total other payments and refundable credits = \"Add lines 9 **through 12** and 14\" ⇒
    /// `line10 + line11` here (9, 12 and 14 are blank) → 1040 **L31**. Sums the PRINTED lines (§3.1).
    pub line15: Usd,
}

/// Derive the printed Schedule 3 chain. Returns `None` when there is neither a foreign tax credit nor
/// an excess-Social-Security credit — the schedule is then not filed.
pub fn schedule_3_lines(ar: &AbsoluteReturn) -> Option<Schedule3Lines> {
    let line1 = round_dollar(ar.foreign_tax_credit);
    let line8 = line1; // lines 2-4, 5a, 5b, 7 are all conservatively omitted (blank)
    let line10 = round_dollar(ar.printed_inputs.extension_payment);
    let line11 = round_dollar(ar.excess_social_security);
    let line15 = line10 + line11; // \"Add lines 9 through 12 and 14\"; 9, 12 and 14 are blank

    if line8 <= Usd::ZERO && line15 <= Usd::ZERO {
        return None;
    }
    Some(Schedule3Lines {
        line1,
        line8,
        line10,
        line11,
        line15,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::{Form8949Box, Form8949Part, Form8949Row};
    use crate::tax::other_taxes::form_8959_lines;
    use crate::tax::se::SeTaxResult;
    use crate::tax::types::FilingStatus;
    use rust_decimal_macros::dec;
    use time::macros::date;

    fn se_300k_single() -> SeTaxResult {
        SeTaxResult {
            net_se: dec!(300000),
            base: dec!(277050.00),
            ss: dec!(21836.40),
            medicare: dec!(8034.45),
            addl: dec!(693.45),
            total: dec!(30564.30),
            deductible_half: dec!(14935.42),
        }
    }

    /// ★ The printed 1040's L14 is a THREE-term sum, and each term is load-bearing.
    ///
    /// 2025: \"Add lines 12e, 13a, and **13b**\". Schedule 1-A is `design/ty2025` B3, so
    /// `assemble_absolute` still hands over a zero 13b — which means a test driven through
    /// `assemble_absolute` asserts a trivial identity and every mutation survives it. (It did:
    /// dropping the 13b term left the suite green.) So this drives the printed path DIRECTLY with a
    /// nonzero 13b and a nonzero QBI, where each term's absence changes the number.
    #[test]
    fn printed_1040_line14_needs_all_three_terms() {","numLines":80,"startLine":1290,"totalLines":2992}},"sourceToolAssistantUUID":"cc150c8f-07b2-42e5-ab06-e7c6a3ad5f7c","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"4d78a39d-cfd8-4306-93ce-64fc1170459f","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNF7WZRdc7PGjGLydd9","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01NmbGfn5xnNmjEWYDKa3N4U","name":"Bash","input":{"command":"grep -rn \"FieldValue\" /scratch/code/bitcoin_tax/crates/*/src/*/seam.rs 2>/dev/null | head; echo ===; find /scratch/code/bitcoin_tax/crates -name \"seam.rs\" -o -name \"coverage.rs\" | head","description":"Locate seam.rs and coverage.rs"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":5230,"cache_read_input_tokens":91143,"output_tokens":531,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":5230},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":531,"cache_read_input_tokens":91143,"cache_creation_input_tokens":5230,"cache_creation":{"ephemeral_5m_input_tokens":5230,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbNF5XkbzeTYrNfaTSTv","attributionAgent":"general-purpose","type":"assistant","uuid":"6c7d8acb-1d17-4b2f-b4ca-4809903dec7b","timestamp":"2026-08-01T04:05:24.900Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"6c7d8acb-1d17-4b2f-b4ca-4809903dec7b","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"type":"tool_result","content":"Exit code 1
(eval):1: no matches found: /scratch/code/bitcoin_tax/crates/*/src/*/seam.rs
(eval):1: == not found","is_error":true,"tool_use_id":"toolu_01NmbGfn5xnNmjEWYDKa3N4U"}]},"uuid":"c36d3216-323a-4813-a6ee-4c2df08337dc","timestamp":"2026-08-01T04:05:24.977Z","toolUseResult":"Error: Exit code 1
(eval):1: no matches found: /scratch/code/bitcoin_tax/crates/*/src/*/seam.rs
(eval):1: == not found","sourceToolAssistantUUID":"6c7d8acb-1d17-4b2f-b4ca-4809903dec7b","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"c36d3216-323a-4813-a6ee-4c2df08337dc","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNFkXH7YSQMJS5SqQ4g","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01MgARtLY3Ck8DCvL9QcuXtz","name":"Bash","input":{"command":"find /scratch/code/bitcoin_tax/crates -name \"seam.rs\" ; find /scratch/code/bitcoin_tax/crates -name \"coverage.rs\"; find /scratch/code/bitcoin_tax/crates -name \"transcribe.rs\"","description":"Find seam, coverage, transcribe modules"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2490,"cache_read_input_tokens":96373,"output_tokens":171,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":2490},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":171,"cache_read_input_tokens":96373,"cache_creation_input_tokens":2490,"cache_creation":{"ephemeral_5m_input_tokens":2490,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbNFioqkQGYdBo6WgzSp","attributionAgent":"general-purpose","type":"assistant","uuid":"c1e98315-d16f-4bd0-8668-66b9f7cc023a","timestamp":"2026-08-01T04:05:27.755Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"c1e98315-d16f-4bd0-8668-66b9f7cc023a","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01MgARtLY3Ck8DCvL9QcuXtz","type":"tool_result","content":"/scratch/code/bitcoin_tax/crates/btctax-input-form/src/seam.rs
/scratch/code/bitcoin_tax/crates/btctax-input-form/src/spec/coverage.rs
/scratch/code/bitcoin_tax/crates/btctax-forms/src/transcribe.rs","is_error":false}]},"uuid":"cda8fc69-1046-47a0-8fb2-7e38dbe7c563","timestamp":"2026-08-01T04:05:27.867Z","toolUseResult":{"stdout":"/scratch/code/bitcoin_tax/crates/btctax-input-form/src/seam.rs
/scratch/code/bitcoin_tax/crates/btctax-input-form/src/spec/coverage.rs
/scratch/code/bitcoin_tax/crates/btctax-forms/src/transcribe.rs","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"c1e98315-d16f-4bd0-8668-66b9f7cc023a","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"cda8fc69-1046-47a0-8fb2-7e38dbe7c563","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNFzBJzvoi13k39rU35","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01GWJ3XSECp73xVLUU2Zs2YL","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/crates/btctax-input-form/src/seam.rs","offset":130,"limit":60},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":285,"cache_read_input_tokens":98863,"cache_creation":{"ephemeral_5m_input_tokens":285,"ephemeral_1h_input_tokens":0},"output_tokens":49,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNFx7YVDACqvpmZDqf3","attributionAgent":"general-purpose","type":"assistant","uuid":"23f752fe-fb08-4c1d-8429-0e8407f6899a","timestamp":"2026-08-01T04:05:31.133Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"23f752fe-fb08-4c1d-8429-0e8407f6899a","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01GWJ3XSECp73xVLUU2Zs2YL","type":"tool_result","content":"130\t    ScheduleC1099Required,
131\t    ScheduleC1099Filed,
132\t    /// Form 8283 5a/5b/5c, asked as ONE return-level universal (§G-21).
133\t    DonationsHadRestrictions,
134\t    // Carryforwards (§G-22) — Form 8995 lines 7 and 3. Both are prior-year LOSSES that REDUCE a
135\t    // deduction, so omitting either UNDERSTATES tax. They were import-only until 2026-07-31.
136\t    QbiReitPtpCarryforwardIn,
137\t    QbiCarryforwardIn,
138\t}
139\t
140\t/// The value shape of a field.
141\t#[derive(Debug, Clone, Copy, PartialEq, Eq)]
142\tpub enum FieldKind {
143\t    Money,
144\t    Text,
145\t    Bool,
146\t    TriState,
147\t    Date,
148\t    Enum(&'static [&'static str]),
149\t    Secret,
150\t}
151\t
152\t/// A field value crossing the seam (spec §4/§5.7). Owned (serde), so it is the web wire.
153\t#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
154\tpub enum FieldValue {
155\t    Money(Usd),
156\t    Text(String),
157\t    Bool(bool),
158\t    TriState(Option<bool>),
159\t    Date(Option<Date>),
160\t    Choice(String),      // an Enum choice by its stable name
161\t    Secret(SecretView),  // OUTBOUND only (get) — presence, never digits
162\t    SecretEntry(String), // INBOUND only (set) — masked Debug; get never returns it
163\t}
164\t
165\t/// A secret's presence, never its digits (spec §4/§5.5).
166\t#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
167\tpub enum SecretView {
168\t    Empty,
169\t    Set { masked: String },
170\t}
171\t
172\timpl SecretView {
173\t    /// ★ The single guarded producer of a `Set` view (spec §4/§5.5; folds review follow-up (b)). The two
174\t    /// maskers in `spec::sections` are its ONLY callers, and this guard rejects a `masked` string that still
175\t    /// carries a run of 5+ raw digits (a raw SSN is 9 digits, an IP PIN 6) — so a raw secret can never be
176\t    /// stored as a \"mask\", even by a future caller. The SSN last-4 reveal (`***-**-6789`) has only a 4-digit
177\t    /// run and passes.
178\t    pub(crate) fn set_masked(masked: String) -> SecretView {
179\t        debug_assert!(
180\t            !masked.as_bytes().windows(5).any(|w| w.iter().all(u8::is_ascii_digit)),
181\t            \"SecretView::set_masked was given a string with a 5+ digit run (a raw secret?): {masked:?}\"
182\t        );
183\t        SecretView::Set { masked }
184\t    }
185\t}
186\t
187\timpl fmt::Debug for FieldValue {
188\t    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
189\t        match self {"}]},"uuid":"77989ece-c7d9-46b6-a787-071ad9ae7ab8","timestamp":"2026-08-01T04:05:31.145Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/crates/btctax-input-form/src/seam.rs","content":"    ScheduleC1099Required,
    ScheduleC1099Filed,
    /// Form 8283 5a/5b/5c, asked as ONE return-level universal (§G-21).
    DonationsHadRestrictions,
    // Carryforwards (§G-22) — Form 8995 lines 7 and 3. Both are prior-year LOSSES that REDUCE a
    // deduction, so omitting either UNDERSTATES tax. They were import-only until 2026-07-31.
    QbiReitPtpCarryforwardIn,
    QbiCarryforwardIn,
}

/// The value shape of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Money,
    Text,
    Bool,
    TriState,
    Date,
    Enum(&'static [&'static str]),
    Secret,
}

/// A field value crossing the seam (spec §4/§5.7). Owned (serde), so it is the web wire.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldValue {
    Money(Usd),
    Text(String),
    Bool(bool),
    TriState(Option<bool>),
    Date(Option<Date>),
    Choice(String),      // an Enum choice by its stable name
    Secret(SecretView),  // OUTBOUND only (get) — presence, never digits
    SecretEntry(String), // INBOUND only (set) — masked Debug; get never returns it
}

/// A secret's presence, never its digits (spec §4/§5.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretView {
    Empty,
    Set { masked: String },
}

impl SecretView {
    /// ★ The single guarded producer of a `Set` view (spec §4/§5.5; folds review follow-up (b)). The two
    /// maskers in `spec::sections` are its ONLY callers, and this guard rejects a `masked` string that still
    /// carries a run of 5+ raw digits (a raw SSN is 9 digits, an IP PIN 6) — so a raw secret can never be
    /// stored as a \"mask\", even by a future caller. The SSN last-4 reveal (`***-**-6789`) has only a 4-digit
    /// run and passes.
    pub(crate) fn set_masked(masked: String) -> SecretView {
        debug_assert!(
            !masked.as_bytes().windows(5).any(|w| w.iter().all(u8::is_ascii_digit)),
            \"SecretView::set_masked was given a string with a 5+ digit run (a raw secret?): {masked:?}\"
        );
        SecretView::Set { masked }
    }
}

impl fmt::Debug for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {","numLines":60,"startLine":130,"totalLines":373}},"sourceToolAssistantUUID":"23f752fe-fb08-4c1d-8429-0e8407f6899a","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"77989ece-c7d9-46b6-a787-071ad9ae7ab8","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNFzBJzvoi13k39rU35","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01XoW8s976hBq1KB5N3BkeCm","name":"Bash","input":{"command":"head -80 /scratch/code/bitcoin_tax/crates/btctax-forms/src/transcribe.rs; echo ===; grep -n \"read_back\\|verify_money\\|fn \" /scratch/code/bitcoin_tax/crates/btctax-forms/src/transcribe.rs | head -30","description":"Inspect transcribe.rs structure"},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":285,"cache_read_input_tokens":98863,"cache_creation":{"ephemeral_5m_input_tokens":285,"ephemeral_1h_input_tokens":0},"output_tokens":49,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNFx7YVDACqvpmZDqf3","attributionAgent":"general-purpose","type":"assistant","uuid":"296d7132-aa5b-4f28-9790-b1de7240cb68","timestamp":"2026-08-01T04:05:32.547Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"296d7132-aa5b-4f28-9790-b1de7240cb68","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"type":"tool_result","content":"Exit code 1
//! **The line-keyed inverse transcriber** — read a filled PDF back in the form's own language.
//!
//! Every other read-back in this crate goes through a fully-qualified AcroForm leaf name:
//!
//! ```ignore
//! assert_eq!(tv(&pdf, \"topmostSubform[0].Page1[0].f1_3[0]\").as_deref(), Some(\"280000\")); // L1
//! ```
//!
//! The `// L1` is the only part a reader cares about and the one part nothing checks. [`extract_lines`]
//! inverts the committed map instead, handing back what the filled PDF actually SAYS keyed by the
//! logical line — so an assertion can name `line1` and mean it.
//!
//! ★ **This is a read-back, not an oracle.** It goes through the same map the fill used, so it cannot
//! catch a mis-mapped cell — a map that pointed `line1` at line 2's widget would fill and transcribe
//! consistently wrong. That is [`crate::verify`]'s job: the geometric verifier re-derives the column
//! and row bands from the blank PDF's own `/Rect`s and never consults the map. The two are
//! complementary and neither replaces the other. **Geometry says the value landed in the right box;
//! this says the right VALUE is in it.**
//!
//! It walks the map TOML generically rather than any typed `*Map` struct, because its consumer — the
//! P7 packet round-trip — transcribes every form in the packet without knowing which one it holds.

use crate::error::FormsError;
use crate::pdf::{checkbox_on, collect_fields, load, text_value};
use std::collections::BTreeMap;
use toml::Value;

/// Read a filled PDF back as `logical line name → the text actually on the paper`.
///
/// Keys are the map's own: `line1`, `line8`, and — for the nested groups every schedule carries — the
/// dotted `identity.name`, `identity.ssn`. Rows of a repeating table come back indexed:
/// `part1_rows[0].payer`.
///
/// **Only cells the fill actually WROTE appear.** A blank line on a tax form is a statement (\"none\"),
/// so \"written as empty\" and \"never written\" must not collapse into the same reading; an absent key
/// means the cell is blank on the paper.
///
/// A checkbox that is ON transcribes as its on-state (`\"1\"`); one that is off is absent, like any
/// other blank cell.
pub fn extract_lines(pdf: &[u8], map_toml: &str) -> Result<BTreeMap<String, String>, FormsError> {
    let map: Value = map_toml.parse()?;

    let doc = load(pdf)?;
    let fields = collect_fields(&doc)?;
    // A map may point two logical cells at one widget (the 2024 1040's line-7 cell is reachable as
    // both `line7a` and `line7`), so index by name rather than consuming the list.
    let by_name: BTreeMap<&str, _> = fields.iter().map(|f| (f.fqn.as_str(), f)).collect();

    let mut out = BTreeMap::new();
    walk(&map, \"\", &by_name, &doc, &mut out);
    Ok(out)
}

fn walk(
    node: &Value,
    prefix: &str,
    by_name: &BTreeMap<&str, &crate::pdf::Field>,
    doc: &lopdf::Document,
    out: &mut BTreeMap<String, String>,
) {
    match node {
        // A bare string is a field name — IF the PDF actually has that field. That test is what keeps
        // the map's metadata (`form = \"f8959\"`, `table_token = \"...\"`) out of the transcript without a
        // hand-maintained skip-list that would rot the first time a map grew a key.
        Value::String(fqn) => {
            if let Some(f) = by_name.get(fqn.as_str()) {
                if let Some(v) = text_value(doc, f.id) {
                    if !v.trim().is_empty() {
                        out.insert(prefix.to_string(), v);
                    }
                }
            }
        }
        // `{ field = \"...\", on = \"1\" }` — a checkbox. Present only when it is ON.
        Value::Table(t) if t.contains_key(\"field\") => {
            let Some(Value::String(fqn)) = t.get(\"field\") else {
                return;
            };
            if let Some(f) = by_name.get(fqn.as_str()) {
                if let Some(on) = checkbox_on(doc, f.id) {
(eval):1: == not found","is_error":true,"tool_use_id":"toolu_01XoW8s976hBq1KB5N3BkeCm"}]},"uuid":"6895019d-1183-4e8b-851b-3dc7f20b1a30","timestamp":"2026-08-01T04:05:32.632Z","toolUseResult":"Error: Exit code 1
//! **The line-keyed inverse transcriber** — read a filled PDF back in the form's own language.
//!
//! Every other read-back in this crate goes through a fully-qualified AcroForm leaf name:
//!
//! ```ignore
//! assert_eq!(tv(&pdf, \"topmostSubform[0].Page1[0].f1_3[0]\").as_deref(), Some(\"280000\")); // L1
//! ```
//!
//! The `// L1` is the only part a reader cares about and the one part nothing checks. [`extract_lines`]
//! inverts the committed map instead, handing back what the filled PDF actually SAYS keyed by the
//! logical line — so an assertion can name `line1` and mean it.
//!
//! ★ **This is a read-back, not an oracle.** It goes through the same map the fill used, so it cannot
//! catch a mis-mapped cell — a map that pointed `line1` at line 2's widget would fill and transcribe
//! consistently wrong. That is [`crate::verify`]'s job: the geometric verifier re-derives the column
//! and row bands from the blank PDF's own `/Rect`s and never consults the map. The two are
//! complementary and neither replaces the other. **Geometry says the value landed in the right box;
//! this says the right VALUE is in it.**
//!
//! It walks the map TOML generically rather than any typed `*Map` struct, because its consumer — the
//! P7 packet round-trip — transcribes every form in the packet without knowing which one it holds.

use crate::error::FormsError;
use crate::pdf::{checkbox_on, collect_fields, load, text_value};
use std::collections::BTreeMap;
use toml::Value;

/// Read a filled PDF back as `logical line name → the text actually on the paper`.
///
/// Keys are the map's own: `line1`, `line8`, and — for the nested groups every schedule carries — the
/// dotted `identity.name`, `identity.ssn`. Rows of a repeating table come back indexed:
/// `part1_rows[0].payer`.
///
/// **Only cells the fill actually WROTE appear.** A blank line on a tax form is a statement (\"none\"),
/// so \"written as empty\" and \"never written\" must not collapse into the same reading; an absent key
/// means the cell is blank on the paper.
///
/// A checkbox that is ON transcribes as its on-state (`\"1\"`); one that is off is absent, like any
/// other blank cell.
pub fn extract_lines(pdf: &[u8], map_toml: &str) -> Result<BTreeMap<String, String>, FormsError> {
    let map: Value = map_toml.parse()?;

    let doc = load(pdf)?;
    let fields = collect_fields(&doc)?;
    // A map may point two logical cells at one widget (the 2024 1040's line-7 cell is reachable as
    // both `line7a` and `line7`), so index by name rather than consuming the list.
    let by_name: BTreeMap<&str, _> = fields.iter().map(|f| (f.fqn.as_str(), f)).collect();

    let mut out = BTreeMap::new();
    walk(&map, \"\", &by_name, &doc, &mut out);
    Ok(out)
}

fn walk(
    node: &Value,
    prefix: &str,
    by_name: &BTreeMap<&str, &crate::pdf::Field>,
    doc: &lopdf::Document,
    out: &mut BTreeMap<String, String>,
) {
    match node {
        // A bare string is a field name — IF the PDF actually has that field. That test is what keeps
        // the map's metadata (`form = \"f8959\"`, `table_token = \"...\"`) out of the transcript without a
        // hand-maintained skip-list that would rot the first time a map grew a key.
        Value::String(fqn) => {
            if let Some(f) = by_name.get(fqn.as_str()) {
                if let Some(v) = text_value(doc, f.id) {
                    if !v.trim().is_empty() {
                        out.insert(prefix.to_string(), v);
                    }
                }
            }
        }
        // `{ field = \"...\", on = \"1\" }` — a checkbox. Present only when it is ON.
        Value::Table(t) if t.contains_key(\"field\") => {
            let Some(Value::String(fqn)) = t.get(\"field\") else {
                return;
            };
            if let Some(f) = by_name.get(fqn.as_str()) {
                if let Some(on) = checkbox_on(doc, f.id) {
(eval):1: == not found","sourceToolAssistantUUID":"296d7132-aa5b-4f28-9790-b1de7240cb68","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"6895019d-1183-4e8b-851b-3dc7f20b1a30","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNGLs1uQ82iMVsZzmVA","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"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"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":3036,"cache_read_input_tokens":99148,"cache_creation":{"ephemeral_5m_input_tokens":3036,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNGJz9goW87KGZFJv59","attributionAgent":"general-purpose","type":"assistant","uuid":"1422ba35-3d7e-47b3-baf0-f027ad0c34cb","timestamp":"2026-08-01T04:08:33.748Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"1422ba35-3d7e-47b3-baf0-f027ad0c34cb","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNGLs1uQ82iMVsZzmVA","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAIS1gMKiQEIEBgCKkCOVMwOoxifbUfagvo9VR0DGURlbbgpljYae23B/ys3wscrwE3P9oyjq9I3znZXSgBjDqdFjiN7oyMIquK/bgxQMg5jbGF1ZGUtZmFibGUtNTgBQgluYXJyYXRpb25aJGUyZmJhYzkwLTAzOGQtNDI2My1hYzA1LWVlMTU1ZTBjY2U4NhIM+MIfGLjHGSQVJR0cGgzkP9zNzUfE+2HVt2UiMK/OGiG9dbvrcXZDT00q+SAxVUZhdkhJSTMO5DOMZfNGmoJr5sr0J7YpdM4g4EAIoyr5AULXkGLIKm1g/TNPplZn4fO3hMDivNNOEsAemVBs1wW71GoNnnTCspZiBTSJfOXJSKfDZEpKIAS4sM+NApq22yuvaaDinEoK6LwJnRTc98mxsxQFbDZkQN6RNN7zDpX9KkcNW+QVuYsP/rKM6PCiJQ68a0MYEekqgzPZTK/fMWDQh0HIuAnWKX2GYpCh8gI+8reZDBJK5L7Bn28ZkXZL6qemmvSw2uRWICY9+okkVYBlRRHMUfn5HY9GLRF9adbNb8se+m2sP173A5m/n95qPhS3hzDhV4C/JGqVjPRdKD5GkPD5729gArpceR/v8154yftUiNFRwygi/xgB"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":3036,"cache_read_input_tokens":99148,"cache_creation":{"ephemeral_5m_input_tokens":3036,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNGJz9goW87KGZFJv59","attributionAgent":"general-purpose","type":"assistant","uuid":"8154638b-c21f-48c3-8a0c-44f54007fc23","timestamp":"2026-08-01T04:08:33.787Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"8154638b-c21f-48c3-8a0c-44f54007fc23","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNGLs1uQ82iMVsZzmVA","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01MCRNiQEMg9DKF7PBFEgfBd","name":"Bash","input":{"command":"grep -rn \"extract_lines\" /scratch/code/bitcoin_tax/crates --include=\"*.rs\" -l; echo ---; grep -rn \"push_money\" /scratch/code/bitcoin_tax/crates/btctax-forms/src/*.rs | wc -l; echo ---; grep -c \"enter -0-\\|enter -0\" /scratch/code/bitcoin_tax/design/forms/extract/f1040--2024.txt /scratch/code/bitcoin_tax/design/forms/extract/f8995--2024.txt /scratch/code/bitcoin_tax/design/forms/extract/f8959--2024.txt","description":"Verify extract_lines usage, push_money count, -0- greppability"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":3036,"cache_read_input_tokens":99148,"output_tokens":11830,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":3036},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":11830,"cache_read_input_tokens":99148,"cache_creation_input_tokens":3036,"cache_creation":{"ephemeral_5m_input_tokens":3036,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbNGJz9goW87KGZFJv59","attributionAgent":"general-purpose","type":"assistant","uuid":"689ffe0b-4c89-40ce-8368-5fed34b3fdf8","timestamp":"2026-08-01T04:08:35.910Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"689ffe0b-4c89-40ce-8368-5fed34b3fdf8","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01MCRNiQEMg9DKF7PBFEgfBd","type":"tool_result","content":"/scratch/code/bitcoin_tax/crates/btctax-forms/src/lib.rs
/scratch/code/bitcoin_tax/crates/btctax-forms/tests/oracle_sweep_readback.rs
/scratch/code/bitcoin_tax/crates/btctax-forms/src/transcribe.rs
/scratch/code/bitcoin_tax/crates/btctax-oracle-harness/tests/smoke.rs
/scratch/code/bitcoin_tax/crates/btctax-forms/tests/extract_lines.rs
/scratch/code/bitcoin_tax/crates/btctax-forms/tests/golden_packet.rs
/scratch/code/bitcoin_tax/crates/btctax-oracle-harness/src/main.rs
---
47
---
/scratch/code/bitcoin_tax/design/forms/extract/f8995--2024.txt:5
/scratch/code/bitcoin_tax/design/forms/extract/f1040--2024.txt:2
/scratch/code/bitcoin_tax/design/forms/extract/f8959--2024.txt:6","is_error":false}]},"uuid":"d60c75d4-1820-4334-9460-5c7f1d5e849c","timestamp":"2026-08-01T04:08:36.017Z","toolUseResult":{"stdout":"/scratch/code/bitcoin_tax/crates/btctax-forms/src/lib.rs
/scratch/code/bitcoin_tax/crates/btctax-forms/tests/oracle_sweep_readback.rs
/scratch/code/bitcoin_tax/crates/btctax-forms/src/transcribe.rs
/scratch/code/bitcoin_tax/crates/btctax-oracle-harness/tests/smoke.rs
/scratch/code/bitcoin_tax/crates/btctax-forms/tests/extract_lines.rs
/scratch/code/bitcoin_tax/crates/btctax-forms/tests/golden_packet.rs
/scratch/code/bitcoin_tax/crates/btctax-oracle-harness/src/main.rs
---
47
---
/scratch/code/bitcoin_tax/design/forms/extract/f8995--2024.txt:5
/scratch/code/bitcoin_tax/design/forms/extract/f1040--2024.txt:2
/scratch/code/bitcoin_tax/design/forms/extract/f8959--2024.txt:6","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"689ffe0b-4c89-40ce-8368-5fed34b3fdf8","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"d60c75d4-1820-4334-9460-5c7f1d5e849c","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNVpZG5VDQTH7vwcd6f","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISkwYKiAEIEBgCKkA7DWKXq4ULPHIzxnLDKN+/AzAKmpNVIZO4S0tM8+ZPQw362Xl0GaMDIbE+tMir1LBCYSpL5WmK4w2e5hnXCXAlMg5jbGF1ZGUtZmFibGUtNTgBQgh0aGlua2luZ1okZTJmYmFjOTAtMDM4ZC00MjYzLWFjMDUtZWUxNTVlMGNjZTg2EgzH/laRdfz6P4mBgjUaDAOJfsrj8GrPd/Fd7yIwLSr4GWWnvlM35U90a2MC8Gj+VdQ5C2izyDMOAMETwghqtW4zJwU0XEfG+HFqmGa6KrcEuUpruYiAmeALJo9OhkPW9uhVNuxHm165V+ucFWDYyzeVERwhKRru2Ac2YgtAtFWsa7kZ4LWWddcozsw5XgTL0DiJF4xwg3bcWaDSYAXGSGG51PtjZyVPWBPSsakYiewoj2ulSRWTwKooSEMMQKh7bTTheUqhXeVpGFWFI6pjYxJZmPGEaSo0lh7+cs/yelKfdG+VcCaa+abPBpNsctlETN8DZG4/LK6SDh6HuHEYTd1y2ISgc/wx0pFaxwh1njKUL1fdH6xAwvhBImsaWbNNjD9/BW7gcPO/6qU9vSu2gMXXeado0hUeOgB57kaQoKPioBTLWmGDIaWRrzbxAbZEpaGuSK8ydh+FGi9kmA2/OTrJdcikjp9h6S+9UiSz4xPwHVjibhIjBhy4dzj1jUSt2ytzk3TTgxGgJcwdqzDnND225HaAsv5CxKE8RLMBz3FuLqByQrvByFzPlLX2rEg/mcM6BW95pe9GeUfo4HTwmAuGvHJUTfOoabz94HWvRom2f9PJYSAN8f5yWxP6xiBMRG8J/tQJ3Ecj/kUAYTab6fRQVfDsi+LZR/oo3NGzZ0b8xHLrtEqgvEIzf6kDhfuoriXyx1iKgoW63W/GqABP9mEgspa2d09kOgph8TMkpHMAmkyQo7jOZmNvsGsrWBmWjwx699/AkhiwIcJifyE9Pm380ynAsKlgGl8aizErJYlRkBBAvnv82FdkEb+SAzV/T3WlJQVZhBjq7LpzAj9e/GyadZq4hma9GAE="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":12214,"cache_read_input_tokens":102184,"cache_creation":{"ephemeral_5m_input_tokens":12214,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNVoSYN1QaK1xwrsUDD","attributionAgent":"general-purpose","type":"assistant","uuid":"1b3df49d-1548-4388-b839-436374192fe0","timestamp":"2026-08-01T04:08:41.317Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"1b3df49d-1548-4388-b839-436374192fe0","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNVpZG5VDQTH7vwcd6f","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01Q8qMgx9VR3bSRSPey8wvzk","name":"Bash","input":{"command":"grep -n \"assert\\|absent\\|blank\\|contains_key\" /scratch/code/bitcoin_tax/crates/btctax-forms/tests/golden_packet.rs | head -25; echo ---; head -40 /scratch/code/bitcoin_tax/crates/btctax-forms/tests/golden_packet.rs","description":"Inspect golden_packet assertion vocabulary"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":12214,"cache_read_input_tokens":102184,"output_tokens":407,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":12214},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":407,"cache_read_input_tokens":102184,"cache_creation_input_tokens":12214,"cache_creation":{"ephemeral_5m_input_tokens":12214,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbNVoSYN1QaK1xwrsUDD","attributionAgent":"general-purpose","type":"assistant","uuid":"3e27c54f-10a3-43d4-aead-02376bb8c8ea","timestamp":"2026-08-01T04:08:42.518Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"3e27c54f-10a3-43d4-aead-02376bb8c8ea","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01Q8qMgx9VR3bSRSPey8wvzk","type":"tool_result","content":"9://! return and then prints it into the wrong box, or drops a form, or leaves a cell blank, files a
15://! and the §6.3 sign/blank read-back), and holds each figure against BOTH oracles through the SAME
17://! figures — the ORACLES'. The assertion is literally: *the number two independent engines arrived at
98:/// the cell is absent (a line the differential compares must be on the paper) or unparseable.
101:        panic!(\"paper_money: the compared cell {key:?} is absent from the paper\")
105:    assert!(
315:    // it under AbsentIsZero — present-\"0\" or absent both mean $0.
339:    assert_eq!(
496:    //    (T11). Each `if let Some` is a no-op on today's JSON (the keys are absent ⇒ `None`); they light
702:    assert!(
768:    assert_eq!(
782:    assert!(
850:    assert!(
1055:    assert_eq!(
1061:    assert!(
1114:/// twice for tax already paid). A SPURIOUS form makes an assertion the filer did not intend: an empty
1116:fn assert_packets_carry_the_derived_forms(households: &[GoldenHousehold]) {
1133:    assert!(
1142:    assert_packets_carry_the_derived_forms(&sample_households());
1150:    assert_packets_carry_the_derived_forms(&golden_households());
1158:/// return is not cosmetic: it asserts to the IRS that they had self-employment income.
1170:    assert!(names.contains(&\"f1040\"), \"every return has a 1040\");
1171:    assert!(
1175:    assert!(
1186:fn assert_every_form_carries_the_filers_identity(households: &[GoldenHousehold]) {
---
//! **P7 / oracle-sweep T6 — the golden packet round-trip, at the PAPER level.** The last link in the
//! chain, and the only one that touches paper.
//!
//! ★ What this closes.
//!
//! `btctax-core`'s `golden_returns.rs` proves the NUMBERS are right: it diffs btctax against two
//! independent engines — OpenTaxSolver, driven directly, and the PSL Tax-Calculator — over twelve
//! households, adjudicating every divergence by a named CLASS. But an engine that computes a perfect
//! return and then prints it into the wrong box, or drops a form, or leaves a cell blank, files a
//! wrong return with a clean conscience. Every test between the tax and the paper was, until now,
//! checking btctax against btctax.
//!
//! So this fills the **actual PDFs** for the **same twelve households the oracles blessed**, reads the
//! full compared line set BACK OFF THE PAPER with the line-keyed inverse transcriber (`extract_lines`
//! and the §6.3 sign/blank read-back), and holds each figure against BOTH oracles through the SAME
//! `oracle_diff` reproduction helpers and divergence classes the compute level uses. Not btctax's
//! figures — the ORACLES'. The assertion is literally: *the number two independent engines arrived at
//! is the number in the box on the 1040.*
//!
//! ★ Three-way localization (§6.5). The test computes the internal chain anyway (to fill the PDF), so a
//! mismatch is reported three ways — `oracle / btctax-internal / btctax-on-paper`: internal matches the
//! oracle but paper does not ⇒ a FILL/transcription bug; both btctax values differ ⇒ a COMPUTE bug. A
//! fill bug and a compute bug send the next author to different files, so the failure says which.
//!
//! ★ The line set, per the §6.1 / C1 reproduction table.
//!   - **AGI L11, taxable income L15, QBI deduction L13** — cent-exact-equivalent, held against BOTH
//!     oracles (`round_leaf` both sides).
//!   - **Tax L16** — the §6.2 two-part: `table_l16` structurally reproduces btctax's own compute L16,
//!     then `stacking_ok` absorbs taxcalc's Tax-Table-vs-schedule dissent only through the methodology
//!     class. btctax alone against BOTH oracles with no class FAILS.
//!   - **TOTAL TAX L24, Schedule SE L12, Form 8959 L18** — OTS-single-witness CROSS-FOOTS (`sum_round`
//!     of the oracle's own printed legs; taxcalc bundles payroll / exposes only exact totals). Pre-T11
//!     the component legs are `None`, so each falls back to `round_leaf` of the baked per-line total —
//!     the HEAD shape, green on the anchors; the leg form activates when the operands bake at T11. L24
//!     INHERITS the SE-L12 / 8959-L18 reproductions (C1 table) so all three switch together at T11.
//!   - **Form 8960 L17 (NIIT)** — OTS-single-witness `round_leaf(oracle_niit)`, cent-exact on the
//!     anchors, carrying an inherent ±cents epsilon (OTS's L17 is on cents-MAGI) → §10 triage, not a class.
//!   - **deduction L12, SALT L5e, Sch-D→L7, 8995 L12** — deeper rows, read off the paper NOW but
//!     oracle-compared only when their `Option` leaf bakes (T11); inert on today's JSON.
//!","is_error":false}]},"uuid":"47d66664-b70c-42af-8b31-09cc1bc28223","timestamp":"2026-08-01T04:08:42.611Z","toolUseResult":{"stdout":"9://! return and then prints it into the wrong box, or drops a form, or leaves a cell blank, files a
15://! and the §6.3 sign/blank read-back), and holds each figure against BOTH oracles through the SAME
17://! figures — the ORACLES'. The assertion is literally: *the number two independent engines arrived at
98:/// the cell is absent (a line the differential compares must be on the paper) or unparseable.
101:        panic!(\"paper_money: the compared cell {key:?} is absent from the paper\")
105:    assert!(
315:    // it under AbsentIsZero — present-\"0\" or absent both mean $0.
339:    assert_eq!(
496:    //    (T11). Each `if let Some` is a no-op on today's JSON (the keys are absent ⇒ `None`); they light
702:    assert!(
768:    assert_eq!(
782:    assert!(
850:    assert!(
1055:    assert_eq!(
1061:    assert!(
1114:/// twice for tax already paid). A SPURIOUS form makes an assertion the filer did not intend: an empty
1116:fn assert_packets_carry_the_derived_forms(households: &[GoldenHousehold]) {
1133:    assert!(
1142:    assert_packets_carry_the_derived_forms(&sample_households());
1150:    assert_packets_carry_the_derived_forms(&golden_households());
1158:/// return is not cosmetic: it asserts to the IRS that they had self-employment income.
1170:    assert!(names.contains(&\"f1040\"), \"every return has a 1040\");
1171:    assert!(
1175:    assert!(
1186:fn assert_every_form_carries_the_filers_identity(households: &[GoldenHousehold]) {
---
//! **P7 / oracle-sweep T6 — the golden packet round-trip, at the PAPER level.** The last link in the
//! chain, and the only one that touches paper.
//!
//! ★ What this closes.
//!
//! `btctax-core`'s `golden_returns.rs` proves the NUMBERS are right: it diffs btctax against two
//! independent engines — OpenTaxSolver, driven directly, and the PSL Tax-Calculator — over twelve
//! households, adjudicating every divergence by a named CLASS. But an engine that computes a perfect
//! return and then prints it into the wrong box, or drops a form, or leaves a cell blank, files a
//! wrong return with a clean conscience. Every test between the tax and the paper was, until now,
//! checking btctax against btctax.
//!
//! So this fills the **actual PDFs** for the **same twelve households the oracles blessed**, reads the
//! full compared line set BACK OFF THE PAPER with the line-keyed inverse transcriber (`extract_lines`
//! and the §6.3 sign/blank read-back), and holds each figure against BOTH oracles through the SAME
//! `oracle_diff` reproduction helpers and divergence classes the compute level uses. Not btctax's
//! figures — the ORACLES'. The assertion is literally: *the number two independent engines arrived at
//! is the number in the box on the 1040.*
//!
//! ★ Three-way localization (§6.5). The test computes the internal chain anyway (to fill the PDF), so a
//! mismatch is reported three ways — `oracle / btctax-internal / btctax-on-paper`: internal matches the
//! oracle but paper does not ⇒ a FILL/transcription bug; both btctax values differ ⇒ a COMPUTE bug. A
//! fill bug and a compute bug send the next author to different files, so the failure says which.
//!
//! ★ The line set, per the §6.1 / C1 reproduction table.
//!   - **AGI L11, taxable income L15, QBI deduction L13** — cent-exact-equivalent, held against BOTH
//!     oracles (`round_leaf` both sides).
//!   - **Tax L16** — the §6.2 two-part: `table_l16` structurally reproduces btctax's own compute L16,
//!     then `stacking_ok` absorbs taxcalc's Tax-Table-vs-schedule dissent only through the methodology
//!     class. btctax alone against BOTH oracles with no class FAILS.
//!   - **TOTAL TAX L24, Schedule SE L12, Form 8959 L18** — OTS-single-witness CROSS-FOOTS (`sum_round`
//!     of the oracle's own printed legs; taxcalc bundles payroll / exposes only exact totals). Pre-T11
//!     the component legs are `None`, so each falls back to `round_leaf` of the baked per-line total —
//!     the HEAD shape, green on the anchors; the leg form activates when the operands bake at T11. L24
//!     INHERITS the SE-L12 / 8959-L18 reproductions (C1 table) so all three switch together at T11.
//!   - **Form 8960 L17 (NIIT)** — OTS-single-witness `round_leaf(oracle_niit)`, cent-exact on the
//!     anchors, carrying an inherent ±cents epsilon (OTS's L17 is on cents-MAGI) → §10 triage, not a class.
//!   - **deduction L12, SALT L5e, Sch-D→L7, 8995 L12** — deeper rows, read off the paper NOW but
//!     oracle-compared only when their `Option` leaf bakes (T11); inert on today's JSON.
//!","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"3e27c54f-10a3-43d4-aead-02376bb8c8ea","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"47d66664-b70c-42af-8b31-09cc1bc28223","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNWL4nUoRGiVLSUHQhz","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAISkQUKiAEIEBgCKkCc4EEC7b8F/Rgd0KuUXmgN6h3nlXNTSlcf1+bHdap+aK38QC0JZcNtd7pjVzxmb9ANcspHfPiLapj6R3DyHl+aMg5jbGF1ZGUtZmFibGUtNTgBQgh0aGlua2luZ1okZTJmYmFjOTAtMDM4ZC00MjYzLWFjMDUtZWUxNTVlMGNjZTg2EgwgOYqLfWKaxGUwbE8aDIgajOyapab86+2xACIwlAu4NVGtYcxCYMjz3AuYVEs96HX705xPabilQvFB/aEmVf4ZVdJpCVjUC3SwCc/OKrUDW2JJvg+VTFgATtzfeDUXWqCABskpZO7GifbPG/YEqOXfXQ+f/pdvim4awh31M7LhcCtdkd9Pfz+k2f4h0Uc7/s2M7tuFONZ4ihGbrtHWcSN8QudykBUH/zT1HWL71EaZfHM8emWhcIz314NFHMTipdN5V1VilLBZbpXyNBkTBix5K0EQ5a/ceVzVl+WuCtwHvoFzqkTjbH1s1/KJ/mYkmgX2ASc6R4Eeh6W/QGrQWGnWCP8g1dHB1/XsTBUydgW1KjIiqw8wpBPgClAud410w6aOjpSk43J/L+UkymgKsj2ypw36Xs7c4MxQ8oYeYeI2HoQNGwkyLXHNp0r59mSaJ3aV6edeKCvocAb8dCj5oZBnMUdg9A2HPfHus+pApNGHMx1jlv5nnSA2vLdB2sATgM1ZP/X40MHFqgcqc/NwVDygY6xv7CYtJfH3TeiqnWLk0gS730UoCUu0sVFbTSzCzUyfcmgiQ/DItdab1+RsgtLf4pjGqwBsC2dlUpXug4WeurOWyNLZyVGeG5d7CwbdkeItIhSg/fi6DbKj/ohhgkq/GqIDqMMVBXqFZ20+1URulp4/P8AYAQ=="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2270,"cache_read_input_tokens":114398,"cache_creation":{"ephemeral_5m_input_tokens":2270,"ephemeral_1h_input_tokens":0},"output_tokens":5,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNWHgfU2LJq2jUudPsB","attributionAgent":"general-purpose","type":"assistant","uuid":"ffd3a330-07ca-4566-8a30-6d306bd728b1","timestamp":"2026-08-01T04:08:47.087Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"ffd3a330-07ca-4566-8a30-6d306bd728b1","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNWL4nUoRGiVLSUHQhz","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01CRRnfNPEmtbYzyVpQsDaHb","name":"Read","input":{"file_path":"/scratch/code/bitcoin_tax/crates/btctax-forms/tests/golden_packet.rs","offset":300,"limit":45},"caller":{"type":"direct"}}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":2270,"cache_read_input_tokens":114398,"cache_creation":{"ephemeral_5m_input_tokens":2270,"ephemeral_1h_input_tokens":0},"output_tokens":5,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNWHgfU2LJq2jUudPsB","attributionAgent":"general-purpose","type":"assistant","uuid":"72a105d3-8fda-4c2e-9d9b-20ed10d10bbd","timestamp":"2026-08-01T04:08:48.364Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"72a105d3-8fda-4c2e-9d9b-20ed10d10bbd","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01CRRnfNPEmtbYzyVpQsDaHb","type":"tool_result","content":"300\t        round_dollar(a.ar.taxable_income),
301\t        ti_crossfoot(
302\t            e.adjusted_gross_income,
303\t            e.deduction_taken,
304\t            e.qbi_deduction,
305\t            e.taxable_income,
306\t        ),
307\t        ti_crossfoot(
308\t            t.adjusted_gross_income,
309\t            t.deduction_taken,
310\t            t.qbi_deduction,
311\t            t.taxable_income,
312\t        ),
313\t    );
314\t    // QBI deduction (1040 L13 / 8995 L15). Line 13 is on every return (0 when there is no QBI), so read
315\t    // it under AbsentIsZero — present-\"0\" or absent both mean $0.
316\t    check_both(
317\t        wrong,
318\t        &h.name,
319\t        \"QBI deduction (L13)\",
320\t        &h.why,
321\t        usd(cell_or_zero(&f1040, \"line13\", Blank::AbsentIsZero) as f64),
322\t        round_dollar(a.ar.qbi_deduction),
323\t        e.qbi_deduction,
324\t        t.qbi_deduction,
325\t    );
326\t
327\t    // ── Tax L16 — the §6.2 two-part (structural reproduction + class stacking), as the compute level ─
328\t    // The reproduced operands are btctax's OWN return figures (1040 L15 / L3a / QD-exclusive net LTCG) —
329\t    // the three args `assemble_absolute` passes to `qdcgt_line16`. Filing status comes off the printed
330\t    // return.
331\t    let reproduced_ops = L16Operands {
332\t        status: a.pr.filing_status,
333\t        ti: a.ar.taxable_income,
334\t        qd_l3a: a.ar.qualified_dividends,
335\t        net_ltcg_qd_excl: a.ar.net_ltcg,
336\t    };
337\t    // Part 1 (structural, Table-semantics witness): the reproduction must recreate btctax's own
338\t    // compute-engine L16 exactly — a drift breaks this before any oracle is consulted.
339\t    assert_eq!(
340\t        table_l16(
341\t            reproduced_ops.status,
342\t            reproduced_ops.ti,
343\t            reproduced_ops.qd_l3a,
344\t            reproduced_ops.net_ltcg_qd_excl,"}]},"uuid":"bce2db38-7c2a-4b26-bdde-27ac61951d16","timestamp":"2026-08-01T04:08:48.380Z","toolUseResult":{"type":"text","file":{"filePath":"/scratch/code/bitcoin_tax/crates/btctax-forms/tests/golden_packet.rs","content":"        round_dollar(a.ar.taxable_income),
        ti_crossfoot(
            e.adjusted_gross_income,
            e.deduction_taken,
            e.qbi_deduction,
            e.taxable_income,
        ),
        ti_crossfoot(
            t.adjusted_gross_income,
            t.deduction_taken,
            t.qbi_deduction,
            t.taxable_income,
        ),
    );
    // QBI deduction (1040 L13 / 8995 L15). Line 13 is on every return (0 when there is no QBI), so read
    // it under AbsentIsZero — present-\"0\" or absent both mean $0.
    check_both(
        wrong,
        &h.name,
        \"QBI deduction (L13)\",
        &h.why,
        usd(cell_or_zero(&f1040, \"line13\", Blank::AbsentIsZero) as f64),
        round_dollar(a.ar.qbi_deduction),
        e.qbi_deduction,
        t.qbi_deduction,
    );

    // ── Tax L16 — the §6.2 two-part (structural reproduction + class stacking), as the compute level ─
    // The reproduced operands are btctax's OWN return figures (1040 L15 / L3a / QD-exclusive net LTCG) —
    // the three args `assemble_absolute` passes to `qdcgt_line16`. Filing status comes off the printed
    // return.
    let reproduced_ops = L16Operands {
        status: a.pr.filing_status,
        ti: a.ar.taxable_income,
        qd_l3a: a.ar.qualified_dividends,
        net_ltcg_qd_excl: a.ar.net_ltcg,
    };
    // Part 1 (structural, Table-semantics witness): the reproduction must recreate btctax's own
    // compute-engine L16 exactly — a drift breaks this before any oracle is consulted.
    assert_eq!(
        table_l16(
            reproduced_ops.status,
            reproduced_ops.ti,
            reproduced_ops.qd_l3a,
            reproduced_ops.net_ltcg_qd_excl,","numLines":45,"startLine":300,"totalLines":1561}},"sourceToolAssistantUUID":"72a105d3-8fda-4c2e-9d9b-20ed10d10bbd","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"bce2db38-7c2a-4b26-bdde-27ac61951d16","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNWmj8YgGxE9xm1qqif","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"CAIS6AUKiAEIEBgCKkDpQLNJZfYarXvcYLWqMJZpvCqqxst5X19Iid1MeksU1mLndCAThIRJdKu61LPPmEj37rb+Kg06gyN8LqTw2w3SMg5jbGF1ZGUtZmFibGUtNTgBQgh0aGlua2luZ1okZTJmYmFjOTAtMDM4ZC00MjYzLWFjMDUtZWUxNTVlMGNjZTg2Egyko0F6Few1wy5hmM4aDEmi8BxtLrcNb9WxdCIwa9aQ/UmFXDgWLuDPrP6mPXeWgno2SZ00iuNDIEVOZ3DolKJuRhpVjeU2jSQaepPuKowE17MXnsI1/j+1oyZ8dus29QvrJkYg6NC1zaRaWIgC9reS0inDb9qVeKRBuXS7V98XSuZ0fat/pjWCHk8S8VVik1M9EBIflTtZ1ZtnQwz9F1ZItCStkHOjcELyH138kiucTXq/olUVwIUaRqo7VMpQYIhbN+5aEvANcEZXG3RfMK+4b3mp2pSprZMxj5cknFImEOZBQ/d/9jTDGHaiBTPe9IxFJ13mp0shUYF6b5EOTYkbnv7ZNCT+UJwEDX+HSdu/qqbA9jjnF/2KY3OaDr1YfNmyQZl4HYudXa+paLI/4g+07HGS7u0YNX47e9gP08yzFNt3myZ0KBd1W7qIWvCccKgTuM79rebOQlnqbPbYGhEVxeb5AcEmJjgR6Vedh9goDX7jeTKHXJWYjB1mJ5TTX253EM5YtsyeQIvgLv9iECb3fKuDvODssJO/hYD8YkoCp8655g17zrxVfney/kdeZzPQ6fheOSvLw8VfQa22cbQPgxbN7xxmJg4JqJYXiHIHxZXbb4hCusz3IckYhYIomdz9CS/Rqn1JLa8X0YMciYmSjWdIHGTtJ81rt3ezj7G7MYtlMrQnt+eOHTAfWhwmyxc1wqEGpPLbXV9K/XeoQ41MS8IaWHF+rBbi7IdCd/c/b/fRNkwBFlKciYpD9pr74JMdYHWb5Zq+TiOY+Kt8tDbbfTauMYbGjOzTsNwYAQ=="}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1108,"cache_read_input_tokens":116668,"cache_creation":{"ephemeral_5m_input_tokens":1108,"ephemeral_1h_input_tokens":0},"output_tokens":3,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNWkLHmHRdkyoqq5j8D","attributionAgent":"general-purpose","type":"assistant","uuid":"91d79e6c-a8aa-4949-8cff-ff1c8d3ba3bf","timestamp":"2026-08-01T04:08:52.741Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"91d79e6c-a8aa-4949-8cff-ff1c8d3ba3bf","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNWmj8YgGxE9xm1qqif","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01DTnGxf1AHa2J92TLpxBQHb","name":"Bash","input":{"command":"grep -n \"enum Blank\\|AbsentIsZero\\|MustBePresent\\|cell_or_zero\" /scratch/code/bitcoin_tax/crates/btctax-forms/tests/golden_packet.rs | head -12","description":"Find Blank enum definition in golden_packet"},"caller":{"type":"direct"}}],"stop_reason":"tool_use","stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":1108,"cache_read_input_tokens":116668,"output_tokens":338,"server_tool_use":{"web_search_requests":0,"web_fetch_requests":0},"service_tier":"standard","cache_creation":{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":1108},"inference_geo":"not_available","iterations":[{"input_tokens":2,"output_tokens":338,"cache_read_input_tokens":116668,"cache_creation_input_tokens":1108,"cache_creation":{"ephemeral_5m_input_tokens":1108,"ephemeral_1h_input_tokens":0},"type":"message"}],"speed":"standard"},"diagnostics":null},"requestId":"req_011CdbNWkLHmHRdkyoqq5j8D","attributionAgent":"general-purpose","type":"assistant","uuid":"2e36c880-6cd6-4eda-a75a-9e9194cd2178","timestamp":"2026-08-01T04:08:53.846Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"2e36c880-6cd6-4eda-a75a-9e9194cd2178","isSidechain":true,"promptId":"31fa38b1-b2fe-4483-af7b-55c8e7ed994f","agentId":"a1bee4ee26b0c17ef","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_01DTnGxf1AHa2J92TLpxBQHb","type":"tool_result","content":"66:// The `full_return`/`packet`/`form` builders and the §6.3 read-back (`on_paper_signed`/`cell_or_zero`)
70:use common::{cell_or_zero, form, full_return, on_paper_signed, packet, Blank, Sign};
315:    // it under AbsentIsZero — present-\"0\" or absent both mean $0.
321:        usd(cell_or_zero(&f1040, \"line13\", Blank::AbsentIsZero) as f64),
422:    let _ = cell_or_zero(&f1040, \"line17\", Blank::PresentZero); // Sch 2 L3 (AMT / excess APTC)
423:    let _ = cell_or_zero(&f1040, \"line21\", Blank::PresentZero); // L19 + L20 (nonrefundable credits)","is_error":false}]},"uuid":"46745adb-9705-4bd0-815f-5f8d553168f8","timestamp":"2026-08-01T04:08:53.930Z","toolUseResult":{"stdout":"66:// The `full_return`/`packet`/`form` builders and the §6.3 read-back (`on_paper_signed`/`cell_or_zero`)
70:use common::{cell_or_zero, form, full_return, on_paper_signed, packet, Blank, Sign};
315:    // it under AbsentIsZero — present-\"0\" or absent both mean $0.
321:        usd(cell_or_zero(&f1040, \"line13\", Blank::AbsentIsZero) as f64),
422:    let _ = cell_or_zero(&f1040, \"line17\", Blank::PresentZero); // Sch 2 L3 (AMT / excess APTC)
423:    let _ = cell_or_zero(&f1040, \"line21\", Blank::PresentZero); // L19 + L20 (nonrefundable credits)","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false},"sourceToolAssistantUUID":"2e36c880-6cd6-4eda-a75a-9e9194cd2178","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"46745adb-9705-4bd0-815f-5f8d553168f8","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNXAXptXE942nPvtbFw","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"","signature":"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"}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":614,"cache_read_input_tokens":117776,"cache_creation":{"ephemeral_5m_input_tokens":614,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNX8hSndKBdu2zFJLAB","attributionAgent":"general-purpose","type":"assistant","uuid":"948d5c91-d969-4986-98fe-e47a54111261","timestamp":"2026-08-01T04:10:38.432Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}
{"parentUuid":"948d5c91-d969-4986-98fe-e47a54111261","isSidechain":true,"agentId":"a1bee4ee26b0c17ef","message":{"model":"claude-fable-5","id":"msg_011CdbNXAXptXE942nPvtbFw","type":"message","role":"assistant","content":[{"type":"text","text":"ANSWER:

Build it — but as **two types at the two boundaries where information is destroyed, not 64 site fixes**, and move the emitter change from last to first.

1. **`Collected<Usd>`** in `btctax-core/src/tax/return_inputs.rs`: `enum { Stated(Usd), NotAsked }`, `#[serde(default)] = NotAsked`, **no** `Default`-to-zero, no `unwrap_or`-family — the only exit is `stated() -> Option<Usd>`. The classifier's `_`-ban extends to it exactly as for `Option<bool>`.
2. **`LineEntry`** in `btctax-core/src/tax/printed.rs`: `enum { Entered(Usd), Blank }`, **no** `From<Usd>`, no `Add`/`Sum`, no `Default`. Every money field on every printed struct becomes `LineEntry`, uniformly. Construction only through named combinators keyed to the form's own instruction grammar: `collected(Option<Usd>)`, `carry(LineEntry)`, `total_of(&[LineEntry])` (Blank iff all operands Blank), `computed(Usd)` (subtract/combine-with-floor lines — all 24 instructed `-0-`s live here).
3. `push_money` (`btctax-forms/src/cells.rs`) takes `LineEntry`; `Blank` ⇒ no write, no placement. Delete the `form8995.rs:255` `ptr::eq` gate; lines 3 and 7 become `collected`.

The omission-cannot-compile claim holds at the strength of the classifier's own honest limit: retyping the fields `E0063`s all 16 constructors and all ~47 `push_money` sites — the 64-site review becomes compile errors — and the 65th site cannot pick \"silent zero\" because that variant no longer exists. What the compiler cannot verify is that the chosen constructor matches the form's instruction; that residue belongs to the gate below.

WHY THIS SHAPE

**Value or line? Both — at different layers, and the layers resolve the tension.** At the input boundary, not-stated is a property of the *collection event*, so it rides the value (`Collected<Usd>`). At the printed layer, the transcription doctrine makes the field *be* the line (`Form1040Lines.line8` — printed.rs:448-532), so value and line coincide: `line8: LineEntry` is a per-line disposition carried as a value. The two in-repo precedents adjudicate cleanly: the `Form8283Row.claimed_deduction` value-gate (form8283.rs:436) is the model; the `form8995.rs:255` line-identity gate is the anti-pattern — its \"only line on this form\" comment was falsified by line 7 in the next loop iteration, which is precisely the hand-list-goes-stale failure CLAUDE.md's oracle-harness section documents. Any per-line suppression table keyed by name reproduces that shape 168 times.

**Why not bare `Option<Usd>` end-to-end.** The repo already wrote the refutation at return_inputs.rs:620-636: `Option<Usd>` is a scalar the classifier permits `_` on, \"which would make this convention again\" — and `unwrap_or(Usd::ZERO)` is a one-token re-fabrication sitting on every `Option`. This codebase's three checkbox fixes each carry a comment saying `unwrap_or(false)` *was* the live bug. A type with no such method makes the fabrication un-writable-without-noticing: you must `match` and type `Usd::ZERO` yourself, which is grep-able and review-loud.

**Why the constructors are keyed to instruction grammar.** This converts H1 (what does a total do?) from nine per-site judgment calls into a transcription question with a small closed production set, checkable line-by-line against `design/forms/extract/`: \"Add lines X–Y\" ⇒ `total_of` (a human adding a blank column leaves the total blank — exactly Schedule 3's \"never a misleading 0\" doctrine, which its boxes honour and its `line8 = line1` total violates today); \"Subtract/Combine … if zero or less, enter -0-\" ⇒ `computed` (always Entered — 8995 L4's \"Combine lines 2 and 3. If zero or less, enter -0-\" shows add-shaped lines with a floor belong here, so the grammar has three productions, not two); \"Enter the amount from …\" ⇒ `carry`. The constructor choice per line becomes a reviewable transcription fact, not a policy argument — the same move that turned G-10's rounding-direction prose into a derived check.

**Why uniform `LineEntry` on printed structs, including always-entered lines.** A mixed `Usd`/`LineEntry` surface reintroduces a per-field judgment (\"is this one safe as bare `Usd`?\") — the exact judgment that produced the 64. \"Every money field on a printed struct is `LineEntry`\" is teachable, lintable, and makes the Tier-2 Form 6251 fields land safe by default (the ISO/2i standing example cannot ship as a silent zero if `line2i` must choose a constructor).

**Rejected: \"do not build.\"** The defect is the product's stated identity (conservative filing; \"an entry is testimony\"), the emitter-only fix addresses 2 of 64, and every smaller alternative (hand-gates, provenance siblings) is a shape the repo has already watched fail. Also rejected: `CarryProvenance`-style siblings as the *primary* mechanism — a sibling that doesn't gate is convention riding beside the same bare `Usd`; keep siblings for what they do today (advisories). And per the hard scope boundary: nothing here opines on whether a blank is lawful — the architecture only preserves who said what.

**What this makes cheap for the four owner decisions.** Each becomes a constructor choice plus one expectation-table row in the gate KAT (below): reconciled-ledger-as-testimony flips Sch C L1 / Sch D 3/10 between `computed` and `collected`; blank-vs-refuse on 1040 25a is a `collected` whose `NotAsked` arm either blanks or feeds `screen_absolute`; supplied-then-zeroed (H4) is out of the leaf types entirely and stays a separately-filed class; 6251 sequencing collapses because Tier 2 inherits the safe type. None requires re-surgery.

THE GATE

**The single instrument exists in-repo already: `transcribe.rs::extract_lines`** — it reads the serialized PDF back keyed by logical line, where *an absent key means the cell is blank on the paper*. Note the current paper-level harness deliberately erases the distinction this program is about: `golden_packet.rs:315-321` reads L13 under `Blank::AbsentIsZero` (\"present-'0' or absent both mean $0\"). The gate is a new **disposition KAT** in `btctax-forms/tests/` that must never use `AbsentIsZero`:

- **Line set enumerated from the committed map TOML** (never a range or hand-list) — every money line must be *accounted for*: asserted present-with-value or asserted absent, per fixture. A line missing from a fixture's expectation table fails the KAT, which is what distinguishes \"no decision\" from \"forgot\".
- **The instructed-zero set is computed from the mechanism, not enumerated**: grep the committed extracts (`design/forms/extract/*.txt`) for the \"enter -0-\" instruction — I verified this yields the survey's lines (f1040: 2, f8995: 5, f8959: 6 hits). New form years auto-join; no excuse list to go stale.
- **Two fixture vectors, one instrument.** An *absence* vector (a household with no Schedule 1, no W-2s, no carryovers) whose expectations assert the fabricatable lines are ABSENT from the transcript; an *instructed-zero* vector (deductions exceeding AGI; a QBI-loss year) whose expectations assert `\"0\"` PRESENT on L15/L22, 8995 L16/L17. Expectations are written from the fixture's *inputs*, not from the printed struct — that independence is what defeats consistent-but-wrong.
- **The B1 kills, both watched red before P1 merges.** Plant 1 (a zero that should not print): re-introduce `sch_1.map_or(Usd::ZERO, …)` at printed.rs:591 → the absence vector reds (\"line8 present as '0', expected absent\"). Plant 2 (over-firing suppression, the invisible one): make `push_money` skip all zero values — the naive \"fix\" — → the instructed-zero vector reds (\"line15 absent, form instructs -0-\"). One instrument, two plants, and it is buildable in P0 *before any site is fixed*, because untouched lines' expectations simply pin today's behaviour, each fabrication marked as such — which also makes the burn-down a grep.

**Honest limits, stated.** (a) The KAT pins fixtures, not all 24 `-0-` branches at once; a completeness census (\"every extract-derived `-0-` line is exercised by some fixture\") is a P4 ambition, and the B1 minimum is one red of each kind. (b) An author who both constructs `Entered(ZERO)` and writes an expectation blessing it passes — the gate forces the fabrication to be *written down twice in review-visible places*, which is the same strength as the classifier, and is the whole game here.

PHASING

The brainstorm's four phases are right in spirit; two orderings are wrong:

- **P0 — vocabulary + gate (new, must be first).** `LineEntry`, `Collected<Usd>`, `push_money(LineEntry)`, the two Sch D 18/19 emitter literals, and the disposition KAT watched red on both plants. The brainstorm's P4 (\"the emitter, last\") is inverted: no blank can exist anywhere until the writer can express one, and the change is 2 sites plus a mechanical signature migration — a precondition, not a finale. Its own §10 already concedes gate-first. Green = suite green with every current disposition pinned (all Entered), both kills demonstrated, zero behaviour change on paper.
- **P1 — stop discarding what we know.** The 15 flattened `Option`s (printed.rs:591/593/654/677/680/690/866/867, Sch 2 L12, fold.rs:1151) plus 8995 lines 3/7 (`collected`; delete the `ptr::eq` gate). Flip those expectation rows to Absent. Green = KAT green with the new rows; no owner decision needed.
- **P2 — the manufactured inputs and the silent forms.** `Collected<Usd>` over the 14 `#[serde(default)] Usd` scalars; the 20 empty-Vec sums and 9 blank-column totals become `total_of`/`collected`; Schedules 1/2/3/C first (no `-0-` conflicts). The 1040 payment lines (25a/26) wait on owner decision 2 — by design that is one constructor + one expectation row when it lands.
- **P3 — make the 65th site fail to compile.** Extend the classifier `_`-ban to `Collected<Usd>`/money leaves. Cheaper than the brainstorm fears: after P2 the ~200-leaf classification is mostly *naming what the migration already did*.
- **P4 — residue.** `\"0\"` vs `\"-0-\"` rendering, the H5 Schedule SE mirror defect (adjudicated against `i1040sse`), the `-0-` completeness census, H4/H6 as separately-filed classes.

WHAT WOULD MAKE THIS ANSWER WRONG

- **Map coverage of the emitted surface — unverified.** The gate enumerates lines from the map TOMLs; cells written outside any map (e.g. crypto-slice `push_literal` paths) would be invisible to it. If a material fraction of the 168 doesn't round-trip through `extract_lines`, the gate needs a writes-set census too, and I did not enumerate that.
- **Blast-radius mechanicalness — assumed, not built.** ~47 `push_money` sites and 16 constructors look mechanical, but `golden_packet.rs` and the oracle harness do arithmetic *on printed fields*; they will need an `entered_or_zero()` accessor, which is a re-fabrication footgun. If that accessor cannot be confined to the test/harness boundary, the type's guarantee leaks.
- **The instruction-grammar claim** (three productions cover all money lines) was checked against the survey's quotes and a sample of extracts, not all 16 forms line-by-line. A line whose instruction fits no production would force either a fourth production or a per-line exception — one is fine; five would mean the grammar is the wrong abstraction.
- **The counts (64/168, the 24, the layer split) are taken as given per the brief.** If the 15-flattened-`Option` tally is materially wrong, P1 shrinks but nothing structural changes."}],"stop_reason":null,"stop_sequence":null,"stop_details":null,"usage":{"input_tokens":2,"cache_creation_input_tokens":614,"cache_read_input_tokens":117776,"cache_creation":{"ephemeral_5m_input_tokens":614,"ephemeral_1h_input_tokens":0},"output_tokens":2,"service_tier":"standard","inference_geo":"not_available"},"diagnostics":null},"requestId":"req_011CdbNX8hSndKBdu2zFJLAB","attributionAgent":"general-purpose","type":"assistant","uuid":"e224694f-084d-4972-9d7f-23f441112f41","timestamp":"2026-08-01T04:10:38.540Z","effort":"xhigh","userType":"external","entrypoint":"cli","cwd":"/scratch/code/bitcoin_tax","sessionId":"2b9d8116-e1de-4038-a101-5fd575fdcb0e","version":"2.1.220","gitBranch":"main","slug":"fuzzy-booping-minsky"}


---

## Disposition (author, same day) — ACCEPTED, with the phasing correction folded

**Accepted as the architecture.** `BRAINSTORM.md` §7's four phases are superseded by the five below;
§10's gate-first instinct was right and §7 contradicted it.

| | phase | green when |
|---|---|---|
| **P0** | vocabulary + gate: `LineEntry`, `Collected<Usd>`, `push_money(LineEntry)`, the 2 Schedule D emitter literals, and the disposition KAT | suite green, **every current disposition pinned as `Entered`**, both B1 plants watched red, **zero behaviour change on paper** |
| **P1** | the 15 flattened `Option`s + 8995 lines 3/7 (delete the `ptr::eq` gate) | those expectation rows flip to `Absent`; no owner decision needed |
| **P2** | the 14 `#[serde(default)] Usd` scalars, the 20 empty-`Vec` sums, the 9 blank-column totals; Schedules 1/2/3/C first (no `-0-` to conflict with) | 1040 25a/26 wait on owner decision 2 — one constructor + one row when it lands |
| **P3** | extend the classifier `_`-ban to money leaves | cheaper than feared: after P2 it is mostly *naming what the migration already did* |
| **P4** | residue: `"0"` vs `"-0-"`, the Schedule SE mirror defect (needs `i1040sse`), the `-0-` completeness census, H4/H6 as separate classes | — |

### What I am taking as the core of it

**The instruction grammar is the move.** Construction only through `collected` / `carry` / `total_of` /
`computed`, each keyed to the form's own words — *"Add lines X–Y"* ⇒ `total_of` (blank iff all operands
blank), *"…if zero or less, enter -0-"* ⇒ `computed` (always entered, and all 24 instructed zeros live
here), *"Enter the amount from…"* ⇒ `carry`. This converts "what does a total do with a blank operand?"
from nine judgment calls into a **transcription question checkable against `design/forms/extract/`** —
the same move that turned §G-10's rounding-direction prose into a derived check, and squarely the
project's transcribe-don't-paraphrase doctrine applied to emission.

**Uniform `LineEntry` on every money field**, including always-entered ones. A mixed
`Usd`/`LineEntry` surface reintroduces the per-field judgment that produced the 64 in the first place.

**The value-vs-line question dissolves rather than resolving**: at the input boundary not-stated is a
property of the collection event (value); at the printed layer the transcription doctrine makes the
field *be* the line, so the two coincide. The `form8995.rs` line-identity gate is named as the
anti-pattern — its *"only line on this form"* comment was falsified by line 7 one loop iteration later,
which is the hand-list-goes-stale failure this repo has now hit three times.

### Carried into the spec as risks, in the consult's own words

- **Map coverage is unverified.** The gate enumerates lines from the map TOMLs; a cell written outside
  any map would be invisible to it. Needs a writes-set census.
- **`entered_or_zero()` is a re-fabrication footgun.** `golden_packet.rs` and the oracle harness do
  arithmetic on printed fields and will need it. **If it cannot be confined to the test/harness
  boundary, the type's guarantee leaks** — this is the one that would quietly undo the whole thing.
- **The three-production grammar was checked on a sample, not all 16 forms.** One line fitting no
  production is fine; five would mean the grammar is the wrong abstraction.
