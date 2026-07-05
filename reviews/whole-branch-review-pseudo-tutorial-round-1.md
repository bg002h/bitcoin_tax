# Whole-diff review (Phase E) — feat/pseudo-tutorial — round 1

**Verdict: 0 Critical / 0 Important — SHIP.** Docs-only (README pseudo-reconcile tutorial); ceremony scaled
to a docs change, with the rigor placed on **end-to-end command verification** (the original-README learning:
tutorial breakage = wrong/stale commands).

## Diff
`main (318e2d3)..HEAD` — README.md only (a new "## Pseudo-reconcile — a fast, honest starting point" section,
5 steps). `make docs` is a no-op (man pages already current from the feature merges).

## Every command run end-to-end on a throwaway vault (verified, not asserted)
- `reconcile pseudo on` → mode message; `verify` → Hard blockers 0 (cleared); the synthetic default is a
  `[self-transfer-in ($0 basis)]` (matches the tutorial's "$0-basis self-transfers").
- `reconcile pseudo approve --dry-run` → previews the default; `--yes` → "approved 1 pseudo default(s) as real".
- **Attestation gate (the load-bearing claim):** pseudo-active + NO `--attest` (non-TTY) → **refused, out dir
  has 0 files**; + correct `--attest "I attest this is true"` → exported; + WRONG phrase → refused ("phrase did
  not match … EXACTLY (trimmed, case-sensitive)"). After `approve` cleared all defaults → export needs no
  attest (the tutorial's "once nothing is [PSEUDO], exports need no attestation").
- `config --set-forward-method fifo` (global) → ok; `--exchange exchange:coinbase:default` (real acct) → ok;
  `--exchange exchange:coinbase:does-not-exist` → **loud reject** listing known accounts → tutorial reworded to
  say the account must exist + btctax lists known ones (no fabricated UUID).
- `reconcile pseudo off` → "reverts … approved decisions remain".

## Claims cross-checked
The error/prompt strings DISPLAY the exact phrase (tutorial: "which the tool displays for you to type or
paste"); the on-screen `[PSEUDO]` markers never enter exported files (sub-2's marker-absent KAT); real
decisions supersede defaults (sub-2 KAT). No claim overstates behavior.

**SHIP.**
