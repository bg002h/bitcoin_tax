# BRAINSTORM — btctax usage-examples constellation (converged design)

*Written 2026-07-16. Converged output of the brainstorm over `CONTINUITY.md` + `RECON.md`. All 8 open
questions resolved with the user; the one code-level determinism blocker was escalated to a Fable
architect whose ruling is persisted verbatim at `reviews/fable-clock-seam-ruling.md`. This document is
the design of record and the substrate for `SPEC_usage_examples.md`. Next gate: SPEC → independent Fable
review to 0C/0I.*

## Goal (unchanged)

Two distributable usage-example docs for the btctax constellation, modeled on the sibling mnemonic
method, whose authoring **doubles as a UX/workflow bug-discovery instrument**. The dual purpose is
explicit and co-equal.

## The 8 open questions — resolved

1. **Scope / journeys → BROAD set (5+).** Maximize exercised surface (more surface = more bugs),
   accepting the larger determinism/golden-maintenance burden and that we author new synthetic corpora
   beyond the existing `fixtures.rs` builders where needed.
2. **Coverage floor → forms-census HARD gate + subcommand-coverage SOFT report.** Every emittable tax
   form must be demonstrated in the corpus (reds CI if not). A census *report* lists which of the ~54
   subcommands lack a worked example (surfaced, non-blocking) — administrative commands need no contrived
   filler.
3. **Determinism (the load-bearing blocker) → ADOPT the `BTCTAX_NOW` CLI seam as Phase 0** (Fable ruling,
   Option A). See §Determinism below. Everything else in CLI stdout is already deterministic or pinnable
   through the environment/argument surface (EventIds are a structured injective enum with zero RNG;
   prices are a pure function of a committed CSV; no version/HashMap/float nondeterminism).
4. **Real vs synthetic → SYNTHETIC-ONLY; commit fixed synthetic import CSVs, regenerate the vault in CI,
   never commit the vault** (its encrypted bytes are nondeterministic and gitignored). `fixtures.rs`
   builders are the "public test vectors."
5. **CI posture → WIDE triggers, born-green ADVISORY → promote to REQUIRED.** Fire on `crates/**`,
   `Cargo.lock`, the generator, and the goldens (leading indicator: any output-changing PR reds *in that
   PR*). At promotion, add mnemonic's fail-safe internal guard so a required + path-filtered check never
   wedges at "Expected — waiting for status."
6. **Toolchain → REUSE groff** (existing `xtask` + `make bundles` + `groff -Tpdf`; no pandoc/xelatex).
   Licensing is a non-issue — groff/pandoc/xelatex are build-time tools whose licenses don't infect
   btctax or the generated PDFs — and groff being already-present + tiny lowers the reproduce/verify
   barrier, reinforcing btctax's max-usability goal. Doc artifacts carry **`MIT OR Unlicense`**.
7. **TUI capture → STYLE-AWARE text goldens** (glyph grid + per-cell fg/bg/modifier map), committed +
   gated like the CLI leg; groff PDF renders with color. Style is captured because selection/error/
   disabled states carry UX meaning the bug-hunt must be able to audit.
8. **Primary goal → BUG-HUNT CO-EQUAL, BUDGETED.** A dedicated adversarial workaround-audit phase (P4)
   catalogs every route-around and classifies each (bug-to-file / harness-artifact / intentional) into
   FOLLOWUPS — a scheduled deliverable, not a side effect.

## Two artifacts, one shared spine (user-mandated split)

- **Artifact 1 — CLI verbatim-I/O examples doc.** A generator runs a freshly-built `btctax` against
  synthetic vaults, capturing `$ cmd` + verbatim **stdout + exit code** into one committed Markdown/text
  golden, rendered to PDF via the existing groff pipeline.
- **Artifact 2 — TUI text-capture doc.** Drives `btctax-tui` / `btctax-tui-edit` through the same
  journeys via the existing `TestBackend` seam, capturing style-aware frames as committed goldens,
  rendered to a **separate** PDF. Kept apart from Artifact 1 (different determinism story, different
  capture tooling — and the split is user-mandated).

## Determinism contract

**Phase 0 — the `BTCTAX_NOW` CLI seam (gates everything).** Adopted per the Fable ruling. The single
wall-clock read at `crates/btctax-cli/src/main.rs:66` gets each decision's made-date, which leaks into
stdout via `verify` (on a vault holding a MethodElection) and the `reconcile bulk-void` /
`bulk-resolve-conflict` previews (all in `crates/btctax-cli/src/` — **not** `btctax-core`; core is wholly
untouched). The seam reads `BTCTAX_NOW` (strict RFC3339) at that line, falling back to `now_utc()` when
unset, passing the parsed value into the already-threaded `now` argument.

- **CLI-only.** Env var, not a flag (matches `BTCTAX_PASSPHRASE`). Malformed/empty ⇒ hard exit 2 naming
  the var + format. **Unconditional stderr banner when active.** Does NOT touch the TUI or update-prices.
- **The fence (goes into the SPEC verbatim):** a change qualifies as a determinism prerequisite (not an
  "engine edit" forbidden by the standing rule) iff **(i)** with the seam inactive the binary is
  behaviorally identical, **(ii)** it injects an *input*, never transforms an *output*, and **(iii)**
  tests pin the inactive-path equivalence. Anything else → FOLLOWUPS with severity + owning phase.
- **Integrity fence** (backdating `BTCTAX_NOW` ≤ a sale date flips `NeedsAttestation → ContemporaneousNow`
  at `optimize.rs:479`): (1) **no pretense** — the user already owns the clock (`faketime`); the vault's
  `utc_timestamp` was always self-reported, never cryptographic evidence; §1.1012-1(j) demands
  contemporaneity *in fact*. (2) unconditional stderr banner, pinned by test. (3) man-page misuse
  language. **Rejected:** persisting an "override-active" vault marker (that would be a real schema/engine
  edit + security theater). Do **not** entangle with pseudo/DRAFT machinery.
- **TDD must pin (Phase 0 deliverables):** unset→unchanged; set→`utc_timestamp` round-trips through the
  *binary* (`reconcile … && verify`); malformed→exit 2; banner on stderr when set / absent when unset /
  never on stdout; twice-run byte-identical stdout **and exit code** for a decision-record + read-back
  journey; and an **integrity KAT** (backdated `BTCTAX_NOW` → `ContemporaneousNow`) that *is* the
  disclosure so the property can never be silently forgotten.

**Harness discipline (no code change):** `BTCTAX_PASSPHRASE` set; `BTCTAX_PRICE_CACHE` → guaranteed-
nonexistent path; fixed cwd + relative `--vault`/`--out`; explicit `--at`/`--effective-from`; `TZ=UTC`,
`LC_ALL=C`. Capture stdout + exit code; state the pinned-env convention in doc front-matter; one honest
sentence noting captures use `BTCTAX_PASSPHRASE` where a real user sees an interactive prompt.

**TUI carries its OWN clock-seam prerequisite** — ~30 wall-clock reads across `btctax-tui` +
`btctax-tui-edit`, incl. an on-screen **timestamped export-dir path** (`btctax-tui/src/export.rs:30`).
This is booked against the Artifact-2 (P3) design — a likely shared clock helper — **not** Phase 0.

**New tax year:** a new *calendar* year is a non-event for existing goldens (pinned `BTCTAX_NOW` freezes
them — no drift, no spurious reds — which is the whole point of pinning). A newly-*supported* tax year
(year-versioned brackets/form revisions/per-year maps) is a maintenance event analogous to a new form:
add/extend a covering journey, same release ritual as re-pinning the version. Not a combinatorial
(form × year) gate. Prices: each journey pins an explicit tax year drawn from the committed daily-close
CSV.

## Journeys — broad, diverse; collectively satisfy the forms-census

Exact fixtures/commands and the precise journey→form mapping are pinned in the SPEC; the themes:

| # | Journey | Anchors (fixture → forms) |
|---|---------|---------------------------|
| J1 | Single-buyer happy path: init → import → report → report --tax-year → export-snapshot → export-irs-pdf | `coinbase_buy_sell_send` → **8949, Sch D, 1040** |
| J2 | Multi-lot §170(e) donation + reconcile lot-selection | `coinbase_two_lot_donation` → **8283, Sch A** |
| J3 | Self-transfer reconcile (forces decision + `verify` read-back; exercises the seam) | self-transfer fixture → decision read-back surface |
| J4 | Income w/ missing FMV → classify-inbound-income; business crypto income | `income_fmv_missing_batch` → **Sch 1, Sch C, Sch SE, 8995** |
| J5 | optimize run/consult/accept + what-if sell/harvest (both attestation branches via different `BTCTAX_NOW`) | → **Sch D**, `ContemporaneousNow` vs `NeedsAttestation` |
| J6 | Complete-return: add non-crypto income via the input-form (ReturnInputs) | interest → **Sch B**; high-income → **8959, 8960, Sch 2/3** |

J6 exists to close the forms-census on the non-crypto schedules. Journeys pin different `BTCTAX_NOW`
values to demonstrate both attestation states.

## Gating

- **Hard:** whole-file golden regen + `git diff --exit-code`; **forms-coverage census** (every emittable
  form demonstrated, else red); **new supported tax year ⇒ covering-journey maintenance event**.
- **Soft:** subcommand-coverage report (which of ~54 commands lack a worked example; surfaced,
  non-blocking).
- **CI:** wide triggers, born-green advisory → promote to required with the fail-safe wedge-guard.
- **Born-green rollout:** the `.gitignore` untrack→track flip + `git add` goldens + the CI job land in one
  atomic commit; a perturb-one-byte → observe-RED proof is mandatory before ship.

## The bug-hunt (co-equal, budgeted — P4)

A deliberately skeptical journey author catalogs every route-around driving the assembled surface (esp.
the TUI-edit reconcile flows), classifies each **bug-to-file / harness-artifact / intentional**, and
files findings into `FOLLOWUPS.md` with severity + owning phase. Scheduled, not incidental.

## Phase breakdown (STANDARD_WORKFLOW; each "→ green" an independent Fable review to 0C/0I)

- **P0** — `BTCTAX_NOW` seam (spec'd, TDD'd, integrity fence). *Gate: green before any golden is recorded.*
- **P1** — CLI generator + goldens + determinism proofs (double-run byte-identity, cross-`$HOME`); wired
  into `xtask`/`make`. "Tests" = determinism proofs.
- **P2** — CI gate (regen + diff; forms-census; subcommand report); born-green + perturb-→-red proof.
- **P3** — TUI text-capture doc + its own clock-seam prerequisite + goldens/gate.
- **P4** — regen + ship + the workaround-audit sweep → FOLLOWUPS.

## Deferred to the SPEC (not decided in brainstorm)

- Exact fixture/command names per journey and the precise journey→form mapping.
- How "supported tax year" is enumerated for the census key.
- The style-map serialization format for TUI goldens.
- The roff verbatim-block wrapping that carries color into the groff PDF.
- The TUI clock-seam design (shared helper across ~30 sites; the `export.rs:30` timestamped path).

## Standing constraints (carried)

- Full STANDARD_WORKFLOW spine; **reviews use Fable**; persist every reviewer output verbatim before the
  fold; re-review after every fold including the last. "It's just docs" is the rationalization the gates
  exist to override.
- Synthetic data only in any committed/distributed artifact.
- Two separate artifacts (CLI ≠ TUI).
- Don't edit the compute/fill engine to make a doc pretty — enforced by the (i)/(ii)/(iii) fence; bugs the
  authoring surfaces → FOLLOWUPS (severity + owning phase).
