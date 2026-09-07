# Brief — the ONE seam review of interview build T1 (the provenance schema)

Independent, adversarial BUILD REVIEWER in your own worktree at the commit the controller names.
Read-only: no source edits, no commits, no subagents. `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`
before any cargo command; scoped `cargo nextest run --locked -p <crate> -E '<filter>'` only; never the
whole workspace (green at the commit under review — the controller states the count). Under the
owner's S6 rule this is the ONE review the build gets; after it, one sonnet re-verification closes.

## The one question
Does T1 make provenance STRUCTURAL — every `ReturnInputs` leaf traceable to a document box, a gate, a
computation or a refusal; every answer written by ONE writer under the prompt it was asked with;
`Declined` a third value never a default; a changed prompt re-asking rather than silently keeping an
answer; a dependent's diligence keyed to the dependent, not a row; older drafts refused or discarded
exactly as the split says — such that nothing T2–T12 will build on can launder an unanswered line
into testimony?

## The contract
`design/SPEC_interview.md` r2: R10 (provenance), R12 (the panel's inputs), §5.6, §7 row T1, §8; the
fold report's T1-tagged "what the build must now prove" rows
(`design/agent-reports/2026-09-07-spec-interview-fold.md`). The implementer's account is
`design/agent-reports/2026-09-07-build-interview-T1-implementation.md` — its claims are claims.

## Seams to examine, in order
1. **`LEAF_SOURCE` ↔ the coverage KAT.** Both directions: every in-scope leaf appears exactly once;
   every entry names a real leaf; a leaf added to `ReturnInputs` without a `LEAF_SOURCE` row reds (plant
   it). Does `EXEMPT_PREFIXES` still hide anything the table should own?
2. **One writer.** `record_answer` reached by the TUI's `apply` and by `income answer`: construct the
   same answer through both and diff the records byte-for-byte (the kill claims identity). Is there
   any third path that writes an answer leaf without a record (the TOML import? the draft flush? the
   R6 slice's accessor)? If `income import` writes leaves with no answer records, is that stated and
   is the panel honest about it?
3. **`prompt_hash`.** Change one fixture prompt's text: the answer must reappear as unanswered in the
   panel with the changed-wording reason and the superseded record in history; restore the text: it
   must come back with NO new history entry. Plant the rule away and confirm the kill reds.
4. **`Declined`.** Is it a distinct value in the type (never `None`, never `false`)? Does every reader
   that treats `None` as "unanswered" treat `Declined` as "answered, forgoing" — the classifier, the
   panel, `screen_inputs`, the census? Grep every match on the answer type for a wildcard arm.
5. **Dependent identity.** Two rows answered; delete row 0 → row 1's records unchanged and row 0's
   gone; change row 1's SSN → its records move to history. Plant the index-keyed version and confirm
   the kill reds.
6. **`SCHEMA_VERSION` 3.** A v2 committed row refuses; a v2 WIP draft is discarded with the note; a v2
   parked draft refuses; a v3 row loads. Confirm nothing silently upgrades a v2 row.
7. **The GENERATED fixtures and goldens.** The examples fixture regenerated with its header command;
   `docs/examples/examples.md` and the man pages moved only where the report lists; the kitchen-sink
   oracle test still pins the fixture.
8. **The kills.** Every test the report names as a kill: would it go red if the guarantee were
   removed? Plant at least: the one-writer identity, the prompt-hash rule, the `Declined` third value,
   the dependent identity key, the v2 refusal.

## Severity (STANDARD_WORKFLOW.md): Critical = a default or inference written as testimony; a
silent laundering path; an unmet guarantee; a gate that cannot fail. Important = a missing case, an
unsound assumption. Minor/Nit non-blocking.

## Output — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T1-review.md` in your worktree: header (commands +
summary lines); per seam what you checked, verdict, findings; findings as `### <ID> (<Severity>) —
<claim>` with Where / What is wrong / Evidence / Minimal change; `Counts: C=<n> I=<n> M=<n> N=<n>`;
"What I did not examine". Return ONLY a 3-line summary (counts, the single most important finding,
the report path).
