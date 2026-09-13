# REPORT — S2 — FR-144 (`archive_drafts.py::STEMS` was a hand-typed 18 against `Stem::ALL`'s 21)

**Agent:** S2 (sonnet). **Owns:** `scripts/archive_drafts.py` exclusively. No other file touched.

## Option chosen: (1) DERIVE — from files on disk, not from the Rust enum

Measured, per the brief's instruction to measure before choosing:

- `crates/btctax-forms/src/bundled.rs` hand-writes `Stem::ALL` (21 variants) and a `file_stem()`
  match arm per variant. Parsing that reliably from Python means parsing Rust match-arm string
  literals out of hand-formatted source — exactly the "fragile" case the brief warned against.
- `crates/btctax-forms/build.rs` builds `Stem`'s generated half from the glob
  `crates/btctax-forms/forms/<year>/*.map.toml`, and its own doc comment states: "a stem with no
  variant makes the generated code fail to compile." I confirmed this glob, deduped by stem name
  across the three bundled years (2024/2025/2026), yields exactly the same 21-item set as
  `Stem::ALL`'s `file_stem()` values (checked by hand against the enum source).
- Consequence: the glob and `Stem::ALL` **cannot silently diverge** — a form added to the glob
  without a matching `Stem` variant fails the Rust build itself. The glob is therefore a safe,
  robust proxy for the enum's set that needs no Rust parser, exactly the case the brief flagged
  as likely more robust. I derived from it.

Two crate stems (`schedule_d`, `schedule_se`) are spelled for readability rather than matching
the bare IRS URL (`f1040sd`, `f1040sse`). This is a 2-entry translation table (`IRS_SPELLING`),
guarded: `draft_stems()` raises if any stem still contains `_` after translation, so a future
naming exception that forgets its translation entry refuses loudly rather than fetching a URL
that cannot exist (a real IRS draft URL never contains `_`).

## What changed, and where

All changes are confined to `scripts/archive_drafts.py` (965 lines, was 796):

1. Replaced the hand-typed `STEMS = [...]` literal (18 items) with `FORMS_GLOB_ROOT`,
   `IRS_SPELLING`, and `draft_stems(forms_root=None)` — a function that globs
   `<forms_root>/*/*.map.toml` (default `crates/btctax-forms/forms/`), dedupes stem names, and
   translates each to IRS draft-URL spelling, refusing on an unmapped underscore. `STEMS` is now
   `draft_stems()` called once at import time — used unchanged everywhere else in the file
   (`main()`'s fetch loop is the only consumer).
2. Added `_stems_derivation_kill()` (the B1 kill, four rows — see below) and wired it into
   `self_test()`'s existing table of kills, printed as "stems derivation kill (FR-144 / F9)".

No other file was touched; nothing outside `scripts/archive_drafts.py` was needed.

## The kill — RED on a planted regression, GREEN on the fix

Planted defect: reintroduce the exact FR-144 shape — make `draft_stems()` ignore `forms_root`
and the disk entirely, returning the frozen 18-item hand list (i.e., revert to what shipped
before this fix). Ran `_stems_derivation_kill()` against that planted copy in isolation
(no repo files touched — the plant lived in a scratchpad copy of the script):

```
STEMS count (planted): 18

=== RED (planted defect: draft_stems ignores disk) ===
 - FAIL: stems derivation kill: expected the hand-typed list to be missing exactly
   ['f1040v', 'f4868', 'f8889'] against today's bundled forms, got [] — `Stem::ALL` has moved
   again since FR-144 was filed; re-baseline this row rather than trust it silently
 - FAIL: stems derivation kill FAILED: 'f1040v' is STILL not fetched — the FR-144 fix did not
   close the gap it exists to close
 - FAIL: stems derivation kill FAILED: 'f4868' is STILL not fetched — the FR-144 fix did not
   close the gap it exists to close
 - FAIL: stems derivation kill FAILED: 'f8889' is STILL not fetched — the FR-144 fix did not
   close the gap it exists to close
 - FAIL: stems derivation kill: a scratch tree with exactly f1040 and f8889 map.toml files
   produced ['f1040', 'f1040s1', 'f1040s1a', 'f1040s2', 'f1040s3', 'f1040sa', 'f1040sb',
   'f1040sc', 'f1040sd', 'f1040sse', 'f6251', 'f8275', 'f8283', 'f8949', 'f8959', 'f8960',
   'f8995', 'f8995a'] — the glob is not reading what was planted
 - FAIL: stems derivation kill FAILED (widening): adding f9999.map.toml to the scratch tree did
   not add f9999 to draft_stems() — the list is not actually reading the disk, which is the
   exact shape of the FR-144 defect reintroduced
 - FAIL: stems derivation kill FAILED (narrowing): deleting f1040.map.toml from the scratch tree
   left f1040 in draft_stems() — a stale cache or hardcoded fallback is in play
 - FAIL: stems derivation kill FAILED (translation guard): a stem with '_' and no IRS_SPELLING
   entry ('some_new_schedule') was accepted rather than refused — the archiver would fetch a URL
   that cannot exist and call it ABSENT, never naming the real cause
```

Green, on the actual fix (`scripts/archive_drafts.py` as committed by this agent):

```
STEMS count (fixed): 21

=== GREEN (actual fix) ===
ok — all 4 rows passed, 8 checks green
```

And in context, `--self-test`'s own output shows the new row passing:

```
  stems derivation kill (FR-144 / F9) ...
    ok — 4 rows (history / widen / narrow / translation guard)
```

## Do the three missing stems now get fetched?

Yes. `draft_stems()` on the real, current `crates/btctax-forms/forms/` returns 21 stems
(confirmed by direct call), including all three the rehearsal found silently dropped:

```
f1040v present
f4868 present
f8889 present
```

## Refuted premises

- The rehearsal's own suggested fix (`REPORT-rehearse-port-f8995a-2025.md`: *"derive from
  `Stem::ALL` through `irs_stem`"*) implicitly pointed at parsing the Rust enum. Measurement
  showed the `crates/btctax-forms/forms/*/*.map.toml` glob is an equally authoritative, far
  simpler source — it is literally the same glob `build.rs` uses to build `Stem` in the first
  place, so it needs no Rust parser and cannot diverge from `Stem::ALL` without the Rust build
  itself refusing.
- None. No other premise in the brief was falsified — the "18 vs 21, missing f1040v/f4868/f8889"
  finding was confirmed exactly as stated before any fix was applied.

## Gate numbers

`scripts/archive_drafts.py --self-test`, before and after, both under `.venv/bin/python` with
`ROOT` resolved to this worktree (`/scratch/code/bitcoin_tax/.claude/worktrees/agent-a7e4f20b87cc32cee`):

| category | before (baseline, ROOT-corrected) | after (this fix) |
|---|---|---|
| reader kills | ok — 7 rows | ok — 7 rows |
| decision kills | ok — 8 rows | ok — 8 rows |
| stems derivation kill (FR-144) | *(did not exist)* | **ok — 4 rows (new)** |
| end-to-end kill | FAIL — 2 rows | FAIL — 2 rows (unchanged) |
| negative fixtures | FAIL — 3 rows | FAIL — 3 rows (unchanged) |
| note/extract collision kill | FAIL — 6 rows | FAIL — 6 rows (unchanged) |
| corpus audit | FAIL (0 PDFs globbed) | FAIL (0 PDFs globbed) (unchanged) |

The four unchanged FAIL categories are all one mechanism: `design/forms/**/*.pdf` is gitignored
(confirmed: `git check-ignore -v design/forms/2025/f1040--2025.pdf` → matches `.gitignore:63`)
and this worktree has 0 PDFs on disk (`find design/forms -name '*.pdf' | wc -l` → 0). This is
the documented worktree-suite caveat from the partition plan, reproduced identically whether or
not this fix is applied — confirmed by running the **pre-fix** file (from a scratchpad copy,
with `ROOT` monkey-patched back to this worktree since the copy's own `__file__`-derived ROOT
would otherwise be wrong) through the same `self_test()` and getting byte-identical FAIL text.

`scripts/archive_drafts.py` has no product-code caller — it is invoked only by a human/CI running
it directly (grep of the whole tree finds no Rust `Command::new`/`process::Command` invocation of
it, only doc-comment mentions in `authority_manifest.rs` and design docs). It is also not wired
into `Makefile` or any `xtask` target (confirmed by grep — this predates FR-144 and is a
different, already-filed finding, `M-4` in `design/agent-reports/2026-09-05-wave2-VERIFY.md`).

**Rust suite (the "7 pre-existing failures" the partition brief names):** `git diff --name-only`
against this worktree's `376d1141` base shows exactly one file changed —
`scripts/archive_drafts.py` — no `.rs`, `Cargo.toml`, or `build.rs` touched. Since
`archive_drafts.py` is not compiled, linked, or read by any Rust crate or build script, the
Rust-side 7 pre-existing failures (6 `form_delta` on gitignored PDFs + 1 `harness_check` /
FR-147) are unaffected by construction. I started a `CARGO_TARGET_DIR=.../target-s2 cargo
nextest run --locked` to confirm this directly, but stopped it before completion: this worktree's
target dir was cold (0 → 11 GB and climbing while compiling `sequoia-openpgp` and other heavy
deps), another agent (S1) was concurrently running its own full workspace build on the same
24-core box, and the diff-based proof above already settles the question without an expensive
full rebuild that cannot change the answer. This is a deliberate scope call, not an oversight —
flagging it explicitly per the "stop and report rather than build on a premise you can disprove"
rule, inverted: here the premise (no Rust impact) is already provable, so the expensive
measurement would have been the redundant step.

## No product behaviour change

`scripts/archive_drafts.py` fetches more forms after this fix (21 vs 18) but this is tooling —
an archival script with no Rust caller and not wired into any build or ship gate. Nothing a filer
sees or a form prints is affected.
