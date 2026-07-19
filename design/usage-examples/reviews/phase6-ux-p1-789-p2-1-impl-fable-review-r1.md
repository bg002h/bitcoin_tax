# Independent adversarial review — Phase 6 (#20): UX-P2-1 + UX-P1-7/8/10 (`81d220b` + `f043872`)

Everything below was re-derived from source at HEAD `f043872` and, where possible, executed. The diff is confined to `/scratch/code/bitcoin_tax/crates/xtask/src/examples.rs` and `/scratch/code/bitcoin_tax/docs/examples/examples.md` (204 insertions / 0 deletions — J1–J6 untouched, additive as claimed).

## What I ran (evidence, not trust)

- **Regen == committed, byte-for-byte**: `cargo run --locked -p xtask -- examples` → `diff` against the committed golden: identical.
- **Full suite**: `make check` — 2063/2063 pass (incl. `examples_golden_matches_committed`, `examples_generate_is_hermetic_across_ambient_env`, `new_journeys_demonstrate_their_reconcile_commands`, all 3 `matcher_tests`), clippy clean. `cargo fmt --check` clean. `bash scripts/pii-scan-generic.sh` clean. `xtask check-isolation` clean. `make examples` emits a valid PDF. MSRV 1.88; the new `let-else` is well within it.
- **Mutation kill, empirically**: I reverted `is_demonstrated` to the old free-subsequence matcher (cp-backup/restore) and ran `matcher_tests`: `a_subcommand_named_only_as_an_argument_is_not_demonstrated` and `the_vault_flag_and_its_value_are_not_subcommands` both RED on the old matcher, GREEN on the new. The old matcher IS the killed mutant. An always-`true` mutant is also killed by those two tests' negative assertions, so `new_journeys_demonstrate_their_reconcile_commands` cannot pass vacuously.
- **Behavioral reproduction of the prose claims** (real binary, pinned env):
  - **J7**: classify-inbound-income *without* `--fmv` → decision recorded, then Hard `[FmvMissing]` blocker on verify (reproduced). Crucially, I re-tested with an income dated 2025-03-10 — a date the **bundled** dataset prices (J8's preview proves it) — and the blocker **still** persists: the "no auto-valuation on the single-event command" claim is unconditionally true, not an artifact of the chosen date. The auto-valuing paths are only `bulk-classify-inbound-income` and pseudo mode (`resolve.rs:1010–1028`). FMV 3300 → income 3300 → tax 726 = 22% ✓.
  - **J8**: no-arg `match-self-transfers` is a read-only preview and cross-wallet → RELOCATE (`cli.rs:859–875`); basis/holding-period carry is real (`btctax-core/src/project/fold.rs:797` — "Relocate consumed fragments to the destination pool: carry basis, HP").
  - **J9**: I rebuilt the J9 vault and read the actual CSVs. Before: `lot` column = `import|coinbase|trade|lot-b#0` (40M sat) + `import|coinbase|trade|lot-a#0` (10M sat) — "the default split draws from both lots" is TRUE (HIFO default picks the pricier lot-b first). After `select-lots`: single leg `lot-a#0` (50M sat) — "draws entirely from lot-a" is TRUE. The `lot` column format is exactly `<origin>#<split>` (`render.rs:768–772`). The prose claims the refs are in the **file** the reader opens ("`export-snapshot` writes a `disposals.csv` whose `lot` column shows…") — it does not pretend they are in the shown stdout. Honest. `contemporaneous` matches `compliance.rs` (made-date 2025-01-01 ≤ sale 2025-06-01 → Contemporaneous).

## UX-P2-1 matcher (item 1)

Anchoring is correct: leading `-`-prefixed tokens are skipped, `--vault`'s value is also skipped, and `path[0]` must exactly equal the first remaining token. Adversarial inputs checked: a line with only global flags (`$ btctax --help`) → matches nothing, no panic; `--vault` as last token → no panic; a vault literally named after a subcommand (`--vault verify report`) → anchors on `report`, does not falsely demonstrate `verify`; `--vault=v.pgp` joined form → handled correctly by the flag-skip alone; empty path → `true` given any `$ btctax` line, but unreachable (`leaf_subcommands` guards `!path.is_empty()`). No false negative is possible for any current golden line: every line is `--vault v.pgp <sub>…`, `--vault` is the CLI's **only** `global = true` arg (`cli.rs:20`), so `GLOBAL_VALUE_OPTS` is complete against the current CLI. Sub-verbs separated from parents by arguments would still match (tail subsequence). Coverage: **20/47**, all 20 verified genuine, and I checked all 27 uncovered leaf names against every `$ btctax` line token — none is genuinely demonstrated, so zero false-uncovered and zero false-covered. The `81d220b` message's "unchanged 17/47" is consistent (old and new matcher agree on the old golden — no leaf name appears as an argument token there; 17 + 3 new journeys = the 20 I measured).

## Determinism (item 2)

All refs derive from embedded corpus data, never wall-clock: `cb-recv`, `sale`, `lot-a` are CSV row-IDs; `1741608000000` is exactly 2025-03-10T12:00:00Z in ms (the River row's timestamp — River has no ID column; same convention as J4/J6's existing refs). The `$8137.26` preview price comes from the bundled daily-close dataset compiled into the binary (the price cache is pinned to a nonexistent file). `BTCTAX_NOW` pins postdate their events (J7: 07-01 > 06-15; J8: 04-01 > 03-10) and the banner goes to stderr with `show_stderr: false` (confirmed: absent from the golden; the hermeticity test additionally proves a stray ambient `BTCTAX_NOW=2099…` cannot leak). No `/tmp`, `home/`, or absolute path anywhere in the golden; export output shows relative `snapshot/`. No new argument contains `$`, `` ` ``, `\`, or `!` (the documented `shell_quote` N-2 limitation is not tripped).

## Findings

**CRITICAL** — none.

**IMPORTANT** — none.

**MINOR** — none.

**NIT / observations** (non-gating, recorded for completeness):

1. **N1 (J9 indirection)**: the before/after lot split is never *displayed* — the reader must open `snapshot/disposals.csv` and `snapshot2/disposals.csv`. The prose is truthful and correctly points at the file (and the generator by design only emits btctax commands), but the journey's punchline lives off-page. If a future mechanism ever shows file excerpts, J9 is the first customer.
2. **N2 (J8 compression)**: "the two legs are unreconciled transfers — a hard blocker" compresses the actual state (in-leg = Hard `UnknownBasisInbound`, out-leg = Advisory `UnmatchedOutflows`); the verify block immediately below shows the precise split, so no reader is misled.
3. **N3 (pre-existing, outside this diff)**: the `[FmvMissing]` hint "no local price for this date — run `btctax-update-prices`" is emitted even when the *bundled* dataset has a close for that date (tested with 2025-03-10); the hint text refers to the cache. Not introduced or touched by these commits.
4. **N4 (residual matcher looseness, documented)**: after the anchor, the tail is still a free subsequence, so a hypothetical `reconcile void <arg-named-like-a-sibling-sub-verb>` line could over-report a `reconcile <sibling>` leaf. No such collision exists in the current golden (checked all 47 leaf names against every line), and the code comment states the tail behavior explicitly.

## VERDICT

**GREEN — 0 Critical / 0 Important**
