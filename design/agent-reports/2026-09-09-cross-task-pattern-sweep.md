# Cross-task pattern sweep — 121c8805..HEAD (interview T7–T12 + journey-walk arc)

Agent: sonnet, read-only worktree at `/scratch/code/bitcoin_tax/.claude/worktrees/agent-a17f5f510099fed92`.
Scope: hunt for the *eighth* instance of "a hand-written list standing beside a set that grows"
(FR-99), plus B1a fixture-blindness, false-PASSes, and writer/reader disagreements, per the
dispatch brief. Not a correctness re-review.

## Commands

```
export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review
cd /scratch/code/bitcoin_tax/.claude/worktrees/agent-a17f5f510099fed92
git log --oneline 121c8805..HEAD | wc -l                      # 38
git diff --stat 121c8805..HEAD | tail -1                      # 133 files, +29439/-970
cargo run -p xtask --release -- stop-list                     # baseline + every plant below
```

Every plant in this report was made with `Edit`, run once, observed, reverted with `Edit` back to
the literal original text, confirmed via `git status --short` / `git diff --stat` (clean), and the
file `touch`ed afterward (FR-90 stale-`target/` caution). No commits were made.

## Summary

Searched the four categories across the 38-commit range. The seven already-known FR-99 instances
(year fixtures, Form-1098 liveness, `CARRIED_IDENTITY`, filing-status match, `Usd`-leaf partition,
`RefuseReason`, document families) are all genuinely fixed and, on inspection, unusually
well-defended — most now carry compiler-enforced totality (`Enum::ALL` + exhaustive match +
round-trip test) rather than just a bigger list. That defense is real: `FieldId`↔`QuestionId`,
`FieldId`↔`SkippableId`, `FieldId`↔`DependentGate`, `DocumentKind::ALL`, `FilingStatus::ALL`, the
`RefuseReason` commit-screen fix, `open_next_year::seed`'s `CARRIED_IDENTITY`, and the six-secret
sweep all held up under inspection (see "Searched and found clean").

The instance that *is* still open lives in the harness's own R15 instrument
(`crates/xtask/src/r15_stop_list.rs`), which was itself extended twice in this range (T6, T12) and
still carries two hand-picked file lists standing beside directories that this range's own tasks
grew substantially. One (the ledger-word ban) is undiscovered; the other (the progress-field/widget
scan) is a **known, filed** gap (FR-98) that I extended with new plant evidence showing it reaches
further than FR-98's text describes.

**2 findings, both machine-planted and reverted; both in `crates/xtask/src/r15_stop_list.rs`.**

---

## Finding 1 — the R15 ledger-word ban never sees `btctax-input-form`'s own filer-facing text

**Where.** `crates/xtask/src/r15_stop_list.rs:233-245` (`ledger_words_in_registry_prompts`, banning
`"transfer"`, `"lot"`, `"fmv"`), fed exclusively by `registry_prompts()` at
`crates/xtask/src/r15_stop_list.rs:366-390`, called at `r15_stop_list.rs:408`. `registry_prompts()`
builds its scanned set from three `btctax_core::tax::questions` sources only: `FORM_QUESTIONS`,
`SKIPPABLE_QUESTIONS`, `RENDERED_PROMPTS`.

**The list.** `registry_prompts()`'s three-source union — the filer-facing text the R15 ledger-word
check is allowed to see.

**The set it should derive from.** All filer-facing prose in the product, which since T5/T9/T10 also
includes every `Field.label` / `Field.help` string in `btctax-input-form`'s `form_spec()`
(`crates/btctax-input-form/src/spec/sections.rs`) — an entirely separate registry from
`questions.rs`'s, holding the census rows, Form-1098 fields, home-sale fields, direct-deposit
fields, and the Form-1099-DA broker-reporting fields, none of which route through
`FORM_QUESTIONS`/`SKIPPABLE_QUESTIONS`.

**What goes wrong when the set grows.** R15's own doctrine (`r15_stop_list.rs:222-224`) is: *"the
ledger's questions (which transfer is this?, which lot?, what was the FMV?) belong to `reconcile`,
and the interview must never re-ask them."* Any future `Field.help`/`Field.label` string added to
`btctax-input-form` — and this range added four whole new `Field` blocks that carry such text
(`FORM_1098_FIELDS`, `HOME_SALE_FIELDS`, `DIRECT_DEPOSIT_FIELDS`, `BROKER_FIELDS`) — can use any of
the three banned words and R15 will report `"no forbidden shape"` regardless. The check's own floor
guards (`prompts.len() < 50`, the `RENDERED_PROMPTS` count assertion) only catch a *collapse* of the
scanned set, not an entire missing *category* of it.

**Evidence.**

*Already live, unplanted.* `crates/btctax-input-form/src/spec/sections.rs:2482` and `:2497-2498`
(`BROKER_FIELDS`, the Form 1099-DA covered/noncovered-lot question) already read, verbatim:
`label: "Covered lots — …"`, `label: "Noncovered lots — …"`, `help: "Same five answers, for the rows
this venue sold that arrived by transfer, …"` — i.e. two of R15's three banned words are already
sitting in shipped, filer-facing production text, and the baseline run below is green despite it.

```
$ cargo run -p xtask --release -- stop-list
R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources, 6 renderer source(s) and 91 registry prompts scanned; no forbidden shape
```

*Clean plant, to remove the "maybe that's legitimate 1099-DA terminology" objection.* Appended to
the `DdRouting` field's help text (`sections.rs:712`) the sentence R15's own doc comment uses as its
canonical banned example — `"PLANTED-R15-KAT-PROBE: which transfer is this, which lot, what was the
fmv."` — then reran:

```
R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources, 6 renderer source(s) and 91 registry prompts scanned; no forbidden shape
```

Identical output, "91 registry prompts scanned" unchanged — the planted sentence was never counted,
let alone flagged. Reverted (`Edit` back to the original literal, confirmed via `git diff --stat`
empty, then `touch`ed).

**Minimal change.** Add a fourth source to `registry_prompts()` (or a sibling function fed into the
same `findings` list) that walks `btctax_input_form::form_spec()` and yields `(FieldId, "{label}
{help}")` pairs — `xtask::box_census::field_words` already exists and does exactly this collection
for a different check (`crates/xtask/src/box_census.rs:1388-1397`), so this is wiring, not new
mechanism.

---

## Finding 2 — R15's two file-list checks are still hand-picked subsets of directories this range grew (extends the already-filed FR-98)

**Where.**
- `crates/xtask/src/r15_stop_list.rs:305-317` (`state_bearing_sources`) — a hard-coded 4-file list
  (`return_inputs.rs`, `provenance.rs`, `document_census.rs`, `interview_state.rs`) under
  `crates/btctax-core/src/tax/`, feeding `progress_shaped_fields` (banned field-name substrings
  `"progress"`/`"remaining"`/`"position"`).
- `crates/xtask/src/r15_stop_list.rs:333-350` (`renderer_sources`) — a hard-coded 6-file list,
  feeding `progress_widgets` (banned `"Gauge"`/`"LineGauge"` / formatted-percentage detection).

**Already filed, partially.** `FOLLOWUPS.md:7028-7040` is **FR-98**, opened during the T12 build: it
already documents that `progress_shaped_fields` reads only the four `btctax-core` modules, that
widening it to `btctax-tui-edit/src/edit/form.rs` produced a false positive
(`LotPickFormRow::remaining_sat`), and that the team built `progress_widgets` as a
mechanism-scoped alternative instead of adding an excuse entry. FR-98 explicitly states *"what is
still unwalked is a stored progress field on a TUI struct"* and is filed as ownerless-residue
harness work, not blocking. **I am not re-filing that gap.** What follows is new evidence that the
same two hand lists have a *second*, un-named dimension FR-98's text does not cover: `crates/btctax-core/src/tax/` itself has grown to roughly 30 files across T8–T12 (`packet.rs`
alone is +661 lines in this range, and now carries the packet manifest, the forgoing block and
`hand_marks`) — a `btctax-core` file *other than the original four*, not the TUI crate FR-98
discusses.

**The set it should derive from.** Every file under `crates/btctax-core/src/tax/` that can carry a
struct field describing interview position (for `state_bearing_sources`), and every file under
`crates/btctax-cli/src` + `crates/btctax-tui-edit/src` that can draw a progress indicator (for
`renderer_sources`) — both currently hand-enumerated rather than walked the way
`input_form_sources()` (`r15_stop_list.rs:283-301`) already walks its own directory.

**What goes wrong when the set grows.** A `pub …progress…`/`…remaining…`/`…position…` field added to
any *other* `crates/btctax-core/src/tax/` file, or a `Gauge`/`LineGauge` reference added to any file
outside the six-file renderer list, is invisible to both checks — silently, with `stop-list`
reporting the fixed source counts (`"4 state-bearing sources"`, `"6 renderer source(s)"`) regardless.

**Evidence.**

*Plant 1 — `state_bearing_sources` outside the four modules.* Added, right after the imports in
`crates/btctax-core/src/tax/packet.rs` (not one of the four listed files):
```rust
#[allow(dead_code)]
pub struct PlantedR15Probe {
    pub progress_remaining_position: usize,
}
```
a field name containing all three banned words at once. Reran:
```
R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources, 6 renderer source(s) and 91 registry prompts scanned; no forbidden shape
```
Silent — `"4 state-bearing sources"` unchanged. Reverted, `git diff --stat` empty, `touch`ed.

*Plant 2 — `renderer_sources` outside the six files.* Added, at the top of
`crates/btctax-tui-edit/src/edit/tax_inputs.rs` (not one of the six renderer files):
```rust
#[allow(dead_code)]
const PLANTED_R15_PROBE: &str = "LineGauge Gauge";
```
Reran:
```
R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources, 6 renderer source(s) and 91 registry prompts scanned; no forbidden shape
```
Silent — `"6 renderer source(s)"` unchanged. Reverted, `git diff --stat` empty, `touch`ed.

**Minimal change.** Same shape as `input_form_sources()`'s own fix a few lines above both functions:
walk `crates/btctax-core/src/tax/` and the two CLI/TUI source trees with `rs_files()` (already
defined at `r15_stop_list.rs:33`) instead of naming files, minus a documented, tested exclusion list
if a mechanism-scoped false positive (like `LotPickFormRow::remaining_sat`) requires one — which is
exactly the "state, in the source, exactly what it covers and what it does not" fallback `CLAUDE.md`
already allows.

---

## Searched and found clean

Documented here so the next sweep does not re-spend budget on these:

- **`FieldId ↔ QuestionId`** (`question_to_field` / `field_to_question`,
  `registries.rs:679-859`): forward map is an exhaustive `match` (compile error on a missed
  `QuestionId`); the doc comment at `registries.rs:970-975` *itself* names `field_to_question` /
  `field_to_skippable` as hand-written second lists, but both are held by a round-trip test
  (`spec/mod.rs:254-256`, `:481-486`) iterating `QuestionId::ALL` / `SKIPPABLE_QUESTIONS` — verified
  present and reads correctly, not merely asserted in prose.
- **`FieldId ↔ DependentGate`** (`field_to_dependent_gate`, `registries.rs:966-980`): DERIVED as
  `gate_to_field`'s inverse over `DependentGate::ALL` — "never a second hand-written match," and it
  is not one.
- **`DocumentKind::ALL` / `transcribed_on`** (`provenance.rs:63-91`): already the fixed T12 instance
  from the known table; confirmed the fix is a real `ALL` const consumed by
  `document_row_facts` (exhaustive match) rather than a second list.
- **`FilingStatus::ALL`** (`packet.rs:670-680`, T8's fix): exhaustive `_`-free match +
  index-equality test at `packet.rs:684-700`. Also a closed, statutory 5-member set — low risk of
  ever growing regardless.
- **`RefuseReason`'s commit-screen coverage** (`interview_state.rs:519-548`, T12 fix): confirmed the
  fix is "run `screen_param_free` itself" rather than a bigger hand list — no five-variant list
  remains.
- **`CARRIED_IDENTITY` / `open_next_year::seed`** (`open_next_year.rs:280-448`, T10 fix): the report
  table is held to the seed by `leaves_the_seed_writes` (walks `provenance::leaf_walk` over the
  *actual* serialized diff, never a hand list) plus a test that fails the build if the seed writes
  an unclaimed leaf. `occupation` and the per-year tri-states are *documented* as deliberately not
  carried, not silently dropped.
- **Six-secret sweep** (`apply.rs`, `every_secret_field_is_asymmetric_written_never_read_back`,
  T10 fix): walks `form_spec()` (derived), pins `n == 6` with an explicit restated list — not
  vacuous.
- **`oracle_projection.rs`'s `ORACLE_INVISIBLE` + routing partition** (T11): `type_alternatives`
  derives enum alternatives from serde deserialization errors rather than a hand list; the file
  explicitly states its own three blind spots (free text/dates/opaque codes; `skip_serializing_if`
  leaves; empty-`Vec` element types) rather than hiding them.
- **`wrapped_literal_check.rs`** (new lint, FR-108): walks `rs_files()` recursively with a
  `FILE_FLOOR = 200` sanity floor; no per-site exemption list by design.
- **`coverage.rs`'s `EXEMPT_PREFIXES`** (Task 6 KAT): a hand list, but load-bearing as a *ratchet* —
  `EXEMPT_PREFIX_CEILING = 5`, asserted may-only-shrink, plus per-entry staleness checks. This is
  category-3 of the FR-99 rule ("state exactly what it covers") done correctly, not a violation.
- **`hand_marks()`** (`admin.rs:472-561`, the packet's "COMPLETE BY HAND" block): considered as a
  candidate — it is a hand-enumerated list of "boxes the tool cannot vouch for" with no derivation
  and no totality test. Concluded this is *not* a clean instance of the FR-99 shape: unlike a
  `FieldId`/`RefuseReason` enumeration, "which printed boxes are unverifiable" is not
  type-derivable, and the codebase already has a *separate*, broader mechanism for the mechanical
  half (`Advisory::UnmodeledReturnOptionsOmitted`, `advisories.rs:597-616`, covering the
  administrative blanks). Did not find a concrete missing mark to plant, so left it out per "an
  unevidenced list is worse than nothing here."
- **FR-103's `stamp_year` fix** (`return_inputs.rs:107-130`): verified the fix is general —
  `stamp_year` is called *inside* `return_inputs::set()` itself, not only at the one `income import`
  call site, so every write path gets it, not just the one the journey walk found.
- **`ABA`/routing-number validation**: exactly one implementation (`packet.rs`'s
  `RoutingNumber::canonical`), consumed by both CLI and input-form paths — no dual-writer split.
- **`DirectDepositCell` / `HomeSaleDecision`**: both small, closed enums (3 and 3 variants) tied to
  IRS-fixed form shapes (a check literally has three boxes), each with a doc-asserted exhaustive
  match at their one call site — low risk, and not showing a hand-list symptom today.

## Environment notes (not findings)

`cargo run -p xtask --release -- stop-list` compiles the full workspace under
`CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` (~35-70s per plant on a warm cache, ~1m12s
cold). All edits were made and reverted with the `Edit` tool (never `git checkout --`), confirmed
clean via `git status --short` / `git diff --stat`, and touched afterward per the FR-90 stale-target
caution. No `cargo test --workspace` or `make check` was run (out of scope per the brief); the
`stop-list` xtask target was the only build surface exercised.

Counts: 2 findings.
