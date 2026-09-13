# Report — sweep the retracted paper-check fact + two stale claims

Worktree: `/scratch/code/bitcoin_tax/.claude/worktrees/agent-a6608f3336b39e940`
CARGO_TARGET_DIR: `<worktree>/target-sweep`

## Fact verified before editing

`design/forms/extract/i1040gi--2025.txt:23824-23827`: *"Starting in October 2025, the IRS will
generally stop issuing paper checks for federal disbursements, including tax refunds, unless an
exception applies."* Confirmed verbatim, not edited (out of scope: `design/forms/**`).

Also independently confirmed before touching anything:
- **`btctax tui-edit` does not exist.** `crates/btctax-tui/Cargo.toml` and
  `crates/btctax-tui-edit/Cargo.toml` register two separate binaries, `btctax-tui` and
  `btctax-tui-edit`; there is no `tui-edit` subcommand on a `btctax` binary.
- **Direct deposit IS built.** `DirectDeposit { routing, kind, account }` at
  `return_inputs.rs:1244`; wired in `f1040.map.toml` (`[direct_deposit]`, lines 111-115);
  screened by `screen_direct_deposit` (`return_refuse.rs:2118`). Only Form 8888 (splitting a
  refund across accounts) is unbuilt, censused `unmodeled` (`return_inputs.rs:1191`,
  `f1040.map.toml:367`). Nothing above was disproved — built on both as given.

## Edits made (4 owned files)

1. **`crates/btctax-core/src/tax/advisories.rs`** (`Advisory::RefundByPaperCheck::message()`,
   ~line 619 pre-edit): replaced "As filed, the IRS will mail a check." with wording that keeps
   the IRS's own "generally" / "unless an exception applies" qualifiers — "a mailed check can no
   longer be relied on" — and fixed `btctax tui-edit` → `` `btctax-tui-edit` ``. Kept the
   remedy (routing/account numbers via the tax-inputs editor, `btctax income import`, or by hand).
   Added a doc-comment paragraph on the enum variant recording the second retraction.
2. **`crates/btctax-cli/LIMITATIONS.md:320`**: same correction in the direct-deposit table row;
   kept "btctax fills this block now" (true).
3. **`crates/btctax-core/src/tax/printed.rs:699-701`**: doc comment on `line34` corrected on both
   halves — the block IS filled when supplied, and a mailed check is no longer reliable when it
   isn't.
4. **`design/LONG_RANGE_PLAN_filing.md:621`** (§7.4 DO NOT BUILD): rewrote the direct-deposit /
   Form 8888 entry to say direct deposit is BUILT, only Form 8888 stays out of scope, and its
   reason (bank details are PII the return doesn't otherwise need) stands alone.

## B1 — guard extended, observed RED then GREEN

Extended `the_paper_check_advisory_is_silent_exactly_when_a_deposit_is_given` with a second
`concat!`-assembled needle (`"the IRS will " + "mail a check"`), scanned both the live message
and the whole module's own source (`include_str!`), matching the existing pattern for the first
(T10) retraction. Also rewrote a test failure-message and the file's own new doc comment so
neither accidentally spelled the needle out contiguously (that self-trip was caught and fixed
before the reported run).

**RED** (planted `"As filed, the IRS will mail a check."` back into the message):
```
FAIL [   0.002s] (29/43) btctax-core tax::advisories::tests::the_paper_check_advisory_is_silent_exactly_when_a_deposit_is_given
thread '...' panicked at crates/btctax-core/src/tax/advisories.rs:3078:9:
the notice must not promise a mailed check as a certainty — the IRS retracted that starting October 2025: ...As filed, the IRS will mail a check. can no longer be relied on...
Summary [   0.013s] 43 tests run: 42 passed, 1 failed, 693 skipped
```

**GREEN** (restored the fix):
```
PASS [   0.017s] (31/43) btctax-core tax::advisories::tests::the_paper_check_advisory_is_silent_exactly_when_a_deposit_is_given
Summary [   0.018s] 43 tests run: 43 passed, 693 skipped
```

Also fixed the now-stale assertion at (pre-edit) `advisories.rs:3271`
(`assert!(m.contains("mail a check"))`) → `assert!(m.contains("can no longer be relied on"))`,
matching the new message's meaning per the brief's instruction.

## Byproduct fix: one generated golden

`make check` first failed on `xtask::bin/xtask examples::tests::examples_golden_matches_committed`
— `docs/examples/examples.md` embeds the exact CLI advisory text and went stale the moment the
message changed. Not in the four owned files and not `crates/xtask/**` source; it is xtask's
*generated output*. Regenerated it via the documented command
(`cargo run -p xtask -- examples > docs/examples/examples.md`) and diffed the result against the
committed version before applying: the only change was the corrected advisory paragraph (old vs
new text, `mail a check` → `can no longer be relied on`, `btctax tui-edit` → `` `btctax-tui-edit` ``).
Applied it; the golden test then passes.

## Full validation suite

- `cargo nextest run -p btctax-core --lib tax::advisories::` — 43/43 passed (post-fix run).
- `make check` (nextest --workspace + clippy --workspace --all-targets --all-features -D warnings,
  run in parallel per the Makefile) — **3650 tests run: 3650 passed, 12 skipped**, exit 0, no
  clippy warnings, no "make check: FAILED" marker.
- `cargo fmt --all --check` — exit 0, no diff.

## Diff scope

```
 crates/btctax-cli/LIMITATIONS.md         |  2 +-
 crates/btctax-core/src/tax/advisories.rs | 50 +++++++++++++++++++++++++++-----
 crates/btctax-core/src/tax/printed.rs    | 10 +++++--
 design/LONG_RANGE_PLAN_filing.md         | 11 ++++---
 docs/examples/examples.md                | 10 ++++---
 5 files changed, 63 insertions(+), 20 deletions(-)
```

No other files touched. Not committed, not pushed, per instructions.

## Out-of-scope sites — reported, not edited

- **`crates/btctax-core/src/tax/return_refuse.rs`** — confirmed exactly two more copies of the
  retracted "arrives as a paper check" claim, at **line 2129** and **line 2161** (both inside
  refusal message strings for a malformed direct-deposit cell), plus the doc comment at
  **`:368-374`** (bare-fact statement: "the refund arrives as a paper check ... which is why this
  rule can afford to be strict"). All three left untouched — this file is owned by another agent
  per the brief.
- **`crates/btctax-input-form/src/spec/sections.rs:725-730`** — checked. Already phrased as a
  quoted position arguing against reliance ("is NOT read as *'I want a paper check'*" / "Either
  answer is expressible; btctax picks neither") — correctly framed already, left alone as
  instructed.
- **`crates/btctax-forms/forms/2024/f1040.map.toml:100-102`** — checked. The retracted sentence
  appears only inside a quoted **historical** note ("This block used to be UNMAPPED ON PURPOSE,
  and its note said so: '...'") describing what the OLD note said before T10 built the block —
  correctly framed as history, not a live claim. Not in the four owned files either way; left
  alone.
- **`crates/btctax-core/src/tax/packet.rs:293,308`, `advisories.rs:2986,3086`,
  `classifier.rs:466`** — checked. All quote the paper-check idea to argue against relying on
  it (e.g. packet.rs: "the argument they made ... is precisely the licence a future maintainer
  must not be given"). These arguments get stronger, not weaker, under the retraction. Left
  alone as instructed.
- Two more occurrences of "a paper check is the disclosed consequence" survive in
  **`design/LONG_RANGE_PLAN_filing.md`** at **line 92** ("Not direct deposit (a paper check is the
  disclosed consequence, `LIMITATIONS.md:222`)") and **line 372** ("direct deposit or paper
  check" — an open owner-question list entry). Both are the same class of stale claim as the one
  fixed at line 621, in a file this brief names as owned — but the brief named only line 621 as
  the edit target, so these two were left untouched and are reported here rather than edited on
  my own initiative.
- `crates/btctax-forms/tests/golden_packet.rs` was flagged by the initial grep sweep but is a
  false positive: its "paper check" hits are the unrelated phrase "read-off-paper check" (the
  differential reading the printed PDF, not a mailed cheque).

## Nothing in the brief was disproved

`btctax tui-edit` is confirmed absent as claimed; direct deposit is confirmed built as claimed.
Fixed accordingly rather than stopping.
