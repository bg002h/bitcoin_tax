# SPEC — btctax usage-examples constellation

*Status: **r1 GREEN (2026-07-16) — Fable re-review 0C/0I** (`reviews/spec-r1-fable-rereview.md`); folded
the independent Fable r0 review (`reviews/spec-r0-fable-review.md`, 0C/6I/7Mi/4N) — all 6 Important + all
Minors/Nits addressed, then folded the re-review's non-blocking Minors/Nits. **Ready for the user gate +
the implementation plan.** Authored by Opus;
**reviewer = Fable (independent)**, loop to 0 Critical / 0 Important per STANDARD_WORKFLOW §2. Design of
record: `BRAINSTORM_usage_examples.md`. The determinism decision was pre-ruled by an independent Fable
architect (`reviews/fable-clock-seam-ruling.md`, Option A). All `file:line` citations verified against
`HEAD ac04ce2` (2026-07-16). r1 fold highlights: corrected the §3.2 clock-leak inventory (bulk-resolve-
conflict is CSV-derived, NOT a seam surface — I4); redesigned J6/C-fullreturn with a donation leg because
`f8283` is ledger-driven (I1); pinned the census enumeration mechanism (I2); replaced the unimplementable
`btctax --version` pin with a Cargo.toml-sourced front-matter pin (I3); corrected §5 commands (I5); and
made the golden born-green via an in-tree `regen==committed` test (I6).*

---

## §0. Background & current state (grounded)

btctax is a Rust workspace (10 crates, v0.6.1, `MIT OR Unlicense`) that reconciles a Bitcoin ledger and
computes/fills a US tax return. We are building **two distributable usage-example documents** whose
authoring **doubles as a UX/workflow bug-discovery instrument** — modeled on the sibling mnemonic
constellation's method (see `RECON.md`). The dual purpose is co-equal and budgeted.

**What already exists and transfers:**
- **groff docs pipeline** — `crates/xtask/src/docs.rs` generates one `clap_mangen` roff page per subcommand
  into `docs/man/`, gated deterministic by `gen_docs_is_deterministic` (`docs.rs:353`) and
  `manpage_covers_every_subcommand` (`docs.rs:261`). `make docs` = `cargo run -p xtask -- docs --pdf`;
  `write_pdfs` renders via **`groff -k -man -T pdf`** (`docs.rs:75-78`); `make bundles` merges with `gs`
  (`Makefile:46-49`). `docs/pdf` is git-ignored / not byte-reproducible (`docs.rs:33-34`).
- **Synthetic fixtures** — `crates/btctax-cli/tests/fixtures.rs` (5 builders, §4.1). Integration tests
  build a tempdir vault with `Passphrase::new("pw")` and drive either library `cmd::` fns (with an
  injected `now`) or the real binary with `BTCTAX_PASSPHRASE=pw` (`end_to_end.rs`, `fr9_exit_code.rs`).
- **Deterministic price source** — `crates/btctax-adapters/data/btc_usd_daily_close.csv` (the only
  committed CSV in-crate; pure function of date, no network).
- **TUI render seam** — `crates/btctax-tui/src/tabs/tests.rs` renders tabs to `TestBackend::new(120,40)`
  → `Buffer`; `btctax-tui-edit` has a full programmatic keystroke harness (`press()`, `type_str()`,
  `handle_key(app, key)`).
- **Forms packet** — `crates/btctax-forms/src/packet.rs::fill_full_return` emits 14 form keys (§6.1);
  `SUPPORTED_YEARS = [2017, 2024, 2025]` (`lib.rs:61`).

**What is NEW ground (no existing precedent — the SPEC must build it):**
- No "run the binary → commit stdout as a golden" pattern exists anywhere (only static man pages are
  regen-gated). Artifact 1's generator is new.
- No `TestBackend` capture reads cell **style** — every current reader takes only `.symbol()`. Artifact
  2's style-aware capture is new (§8).
- No CLI clock-injection seam exists — the binary reads wall-clock at `main.rs:66` (§3.2).

**Corrected assumptions (were stale in the brainstorm; fixed here):**
- `config --set-forward-method` is a **flag on `config`**, not a subcommand (`cli.rs:92-103`).
- There is **no** `reconcile resolve-conflict` verb — single-item is `accept-conflict`/`reject-conflict`
  (`cli.rs:573,575`); batch is `bulk-resolve-conflict` (`:792`).
- **No reconcile verb takes a `--date` flag** — reconcile made-dates come only from the wall clock inside
  the binary. This *reinforces* the P0 seam: it is the sole way to make decision-bearing output
  deterministic.
- The fixture library has **no** self-transfer, business-income, interest/dividend, high-income, or
  missing-FMV-via-CSV builder — those corpora must be authored (§4.2).

## §1. Goal & non-goals

**Goal.** Two committed, CI-gated, groff-rendered usage-example documents (CLI + TUI), built from
synthetic data, that (a) teach real btctax workflows via verbatim I/O, (b) never drift silently from the
binary, (c) demonstrate every emittable tax form, and (d) surface UX/workflow bugs through an adversarial
authoring audit filed to FOLLOWUPS.

**Non-goals.** (1) No change to tax computation, form-fill logic, output wording, or persisted schema —
enforced by the §3.1 fence. (2) Not a reference manual (the man pages already are that). (3) No real
taxpayer data, ever. (4) No pandoc/xelatex. (5) No pixel/screenshot capture (the TUI is captured as text).

## §2. The two artifacts

| | Artifact 1 — CLI examples | Artifact 2 — TUI capture |
|---|---|---|
| Content | verbatim `$ cmd` + stdout **+ exit code** | style-aware `TestBackend` frames (glyph grid + per-cell fg/bg/modifier) |
| Source of truth (gated) | committed text golden (`.md`/`.roff`) | committed text goldens (glyph grid + style map) |
| Render | `groff -k -man -T pdf` → committed-nowhere PDF (built in CI, release-attached) | same groff path, separate PDF |
| Binaries driven | `btctax` | `btctax-tui`, `btctax-tui-edit` |
| Determinism | P0 seam + harness discipline (§3.2-3.3) | P0 seam + TUI's own seam (§3.4) |
| Split | **separate files** (user-mandated) | |

Both goldens are committed text; both PDFs are renders and carry `MIT OR Unlicense`. Golden source lives
under `docs/examples/` (CLI) and `docs/examples-tui/` (TUI); exact paths pinned in P1/P3.

## §3. Determinism contract

### §3.1 The fence (write verbatim into every future "just a tweak for the docs")

> A change qualifies as a **determinism prerequisite** (not an "engine edit" barred by the standing rule
> "don't edit the compute/fill engine to make a doc pretty") **iff all three hold**: **(i)** with the seam
> inactive the binary is behaviorally byte-identical; **(ii)** it injects an *input*, never transforms an
> *output*; **(iii)** tests pin the inactive-path equivalence. Any change failing this trichotomy —
> rewording a message, changing a column width, altering rounding, touching persisted schema — stays under
> the standing rule and is routed to FOLLOWUPS with severity + owning phase.

### §3.2 Phase 0 — the `BTCTAX_NOW` CLI seam (gates all goldens)

Per the Fable ruling. The single wall-clock read at `crates/btctax-cli/src/main.rs:66`
(`OffsetDateTime::now_utc()`) becomes each decision's stored `utc_timestamp`, which leaks into stdout via
these **clock-derived** surfaces (the **print** sites are all `btctax-cli`; the fix touches only
`main.rs:66`, and `btctax-core` is untouched): `verify`'s MethodElection
`recorded` date (`render.rs:2258`); the `reconcile bulk-void` preview (`session.rs:1134` → `main.rs:2005`
— it iterates `voidable_decisions`, i.e. Decision events stamped by `append_decision(now)`); the `config
--set-forward-method` recorded made-date (`cmd/reconcile.rs:968`, receiving `now` from `main.rs:470`; when
`--effective-from` is omitted it defaults to the made-date); and the attestation / what-if surfaces (the
`persistability` classification lives in `btctax-core/src/optimize.rs:469-484` but its **print** consumer
is CLI-side, and the "defaults to today UTC" `--at` flags on `optimize consult` / `what-if`).

**NOT clock-derived — deterministic, and therefore forbidden as seam-proof surfaces:** the `reconcile
bulk-resolve-conflict` preview (`session.rs:1097` — an ImportConflict's `utc_timestamp` is *copied from
the conflicting CSV row*, `persistence.rs:215-226`) and the `match-self-transfers` preview
(`session.rs:1183` — a `TransferIn` timestamp). *(This corrects the architect ruling's inherited
inventory, which mislabelled bulk-resolve-conflict as clock-derived; re-verified against source
2026-07-16. A P0 test that used either of these as its read-back surface would pass **without** the seam —
the project's named untested-guard failure.)*

**Requirements (R-P0):**
- **R-P0.1** — At `main.rs:66`, read env `BTCTAX_NOW`; if set, parse strict **RFC3339** and use it as the
  threaded `now`; if **unset**, fall back to `OffsetDateTime::now_utc()` (behavior unchanged).
- **R-P0.2** — Env var, **not** a CLI flag (matches `BTCTAX_PASSPHRASE`, `main.rs:50`); it appears in no
  subcommand `--help`.
- **R-P0.3** — Malformed or empty `BTCTAX_NOW` ⇒ **hard exit 2**, error message naming the variable and
  the expected format. Never a silent fallback.
- **R-P0.4** — When active, emit **one unconditional stderr line**: `warning: BTCTAX_NOW override active —
  decision timestamps are simulated`. On stderr (stdout goldens stay clean); unconditional (not
  TTY-gated).
- **R-P0.5** — CLI-only. Do **not** touch `btctax-tui`/`btctax-tui-edit` (§3.4) or `btctax-update-prices`.
- **R-P0.6 (integrity)** — Backdating `BTCTAX_NOW` ≤ a sale date flips `NeedsAttestation →
  ContemporaneousNow` (`optimize.rs:479`). Fence: (a) **no pretense** — the user already owns the clock
  (`faketime`); the vault `utc_timestamp` is self-reported, never cryptographic evidence; §1.1012-1(j)
  demands contemporaneity *in fact*. (b) the R-P0.4 banner, pinned by test. (c) man-page language: the
  variable exists for reproducible testing/documentation, and backdating a decision record does not make
  an identification contemporaneous under the reg. **Rejected:** persisting an "override-active" vault
  marker (a schema/engine edit + security theater). **Do not** entangle with pseudo/DRAFT machinery.

**Tests (TDD, all Phase-0 deliverables — a fix isn't done until the mutation dies):**
- **T-P0.1** unset → wall-clock path; a representative command's behavior is unchanged (fence (iii)).
- **T-P0.2** set → a decision's persisted `utc_timestamp` round-trips exactly **through the binary**
  (`reconcile … && verify` read-back), closing the gap the library-level tests dodge (`end_to_end.rs:50`
  injects `now` at the library layer only). The read-back MUST be a **clock-derived** surface (a
  `MethodElection` `recorded` date via `verify`, the `bulk-void` preview, or a `config
  --set-forward-method` made-date) — explicitly **not** bulk-resolve-conflict / match-self-transfers,
  which are CSV-derived and pass without the seam (§3.2).
- **T-P0.3** malformed / empty → exit 2, named error.
- **T-P0.4** banner present on stderr when set, absent when unset, **never on stdout**.
- **T-P0.5** twice-run **byte-identical stdout AND exit code** for a decision-record + read-back journey
  under pinned `BTCTAX_NOW` + `BTCTAX_PASSPHRASE` + `BTCTAX_PRICE_CACHE`→nonexistent. The read-back is a
  clock-derived surface (as T-P0.2) so the test would fail if the seam regressed.
- **T-P0.6 (integrity KAT)** with `BTCTAX_NOW` backdated ≤ sale date, `persistability` yields
  `ContemporaneousNow` — the test *is* the disclosure so the property can never be silently forgotten.
  The KAT must use a **non-broker wallet and/or a pre-2027 sale date**, because `ForbiddenBroker2027`
  precedes `ContemporaneousNow` in `persistability` (`optimize.rs:476-480`); otherwise the KAT's premise
  never reaches the branch under test.

**Gate: P0 closes green (spec'd, TDD'd, Fable-reviewed to 0C/0I) BEFORE the first golden is recorded.**

### §3.3 Harness discipline (no code change; applies to both artifacts)

`BTCTAX_PASSPHRASE=pw`; `BTCTAX_PRICE_CACHE`→a guaranteed-nonexistent path; fixed cwd + **relative**
`--vault`/`--out` (init/import echo `vault.display()`); explicit `--at`/`--effective-from` on date-taking
commands; `TZ=UTC`, `LC_ALL=C`, `LANG=C`; scrubbed `HOME`. Capture **stdout + exit code** (exit codes are
output: `verify` returns 1 on hard blockers, `main.rs:89-91`). **stderr scope:** stderr (the R-P0.4
banner, 1099-DA / SE / 8959 notices, export warnings) is captured into a **separate labelled block** where
a journey's stderr is pedagogically relevant, and otherwise declared out of the verbatim-stdout capture —
never silently dropped (disclosed §13). Front-matter states the pinned-env convention (`BTCTAX_NOW`,
`BTCTAX_PASSPHRASE`, `BTCTAX_PRICE_CACHE`→nonexistent) and one honest sentence: captures use
`BTCTAX_PASSPHRASE` where a real user sees an interactive prompt.

### §3.4 TUI sub-determinism (owned by P3, NOT P0)

`btctax-tui`/`btctax-tui-edit` have **~24+ production wall-clock reads** (`grep now_utc`: ~24 in
`tui-edit/main.rs` + 1 in `edit/persist.rs` + 2 in `tui/lib.rs`, minus test sites — exact production/test
split + final count re-verified in P3, §14 gap 4; FOLLOWUPS UX-P3-1's "~30" is reconciled there). Most stamp persisted decision timestamps that can
**resurface in a later rendered frame in the same session**, so the P3 shared clock helper must route
**every production read**, not only the three that render *directly*: `lib.rs:247` (what-if panel),
`lib.rs:256` → `export::export_dir_for` (`export.rs:30`, the on-screen `btctax-export-YYYYMMDD-HHMMSSZ`
dir), and `btctax-tui-edit/src/main.rs:2609` (method-election "resolved as of" date). The helper is
env-injected under the same §3.1 fence; do not stretch P0 to cover it.

### §3.5 New tax year

A new *calendar* year is a **non-event** for existing goldens — the pinned `BTCTAX_NOW` freezes them (no
drift, no spurious CI reds). A newly *supported* tax year (a new entry in `SUPPORTED_YEARS`, new per-year
maps/PDFs) is a **maintenance event** analogous to a new form (§6.4): add/extend a covering journey, same
release ritual as re-pinning the version. Not a combinatorial (form × year) gate.

## §4. Synthetic corpora ("public test vectors")

### §4.1 Existing builders (`crates/btctax-cli/tests/fixtures.rs`, reuse as-is)
- `coinbase_buy_sell_send` (`:8`) — Buy 0.1 / Sell 0.02 / Send 0.03 (→ pending TransferOut). **J1.**
- `coinbase_buy_receive` (`:25`) — Buy 0.05 + Receive 0.02 (→ `UnknownBasisInbound` blocker). **J3 seed.**
- `coinbase_two_lot_donation` (`:50`) — LT lot A + ST lot B + Send 2 BTC reclassified Donate FMV $100k
  ($52k deduction). **J2.**
- `income_fmv_missing_batch(n)` (`:71`) — returns `Vec<LedgerEvent>` (NOT a CSV) of `n` missing-FMV
  Staking income events. Library-level only.
- `coinbase_single_buy` (`:103`) — single Buy (self-contained USD).

### §4.2 NEW corpora to author (P1 deliverables; synthetic, committed)
- **C-self-transfer** — a two-exchange CSV pair producing an inbound TransferIn needing
  `classify-inbound-self-transfer` / `match-self-transfers`. (No existing builder; **J3**.)
- **C-income-csv** — a River (or Coinbase) CSV producing **missing-FMV income** through `import` (the
  existing `income_fmv_missing_batch` is library-only). (**J4**.)
- **C-business** — crypto business/self-employment income (Sch C / SE / 8995 path). Source the amounts
  from `kitchen_sink_household()` (§4.3) to stay oracle-consistent. (**J4/J6**.)
- **C-multilot** — a **two-Buy** (LT + ST lot) non-donation vault so `optimize`/`select-lots` has a real
  lot-selection choice (the existing `coinbase_buy_sell_send` is single-Buy → the optimizer demo would be
  degenerate). (**J5**.)
- **C-fullreturn (TY2024)** — the non-crypto return inputs (wages, interest→Sch B, high income→8959/8960)
  from `kitchen_sink_household()` (§4.3) **PLUS a donation-bearing crypto ledger leg** — because `f8283`
  is ledger-donation-driven (`btctax-core/src/tax/packet.rs:537-540`: files only when the return itemizes
  AND Schedule A line 12 noncash > `FORM_8283_THRESHOLD` via `form_8283(state, year, donation_details)`),
  and kitchen-sink
  alone emits **13 of 14** forms (no donation). The leg = import a synthetic donation CSV +
  `reconcile reclassify-outflow --as-kind donate …` + `reconcile set-donation-details …`, with the
  donated FMV chosen so Schedule A line 12 clears the threshold. **Caveat (stated in the doc):** this
  composite deviates from the pure oracle-validated `kitchen_sink_household` vector by exactly the added
  donation delta — the non-donation figures remain the oracle vector. (**J6**; see the per-form emission
  table in §6.1.)

### §4.3 Golden households (the non-crypto vectors)
`crates/btctax-core/src/tax/testonly.rs` (`pub mod`, not `#[cfg(test)]`): `kitchen_sink_household()`
(:165), `w2_only_household()` (:320), `golden_households()`/`build_golden_household` (:367/:500). These pair
a `ReturnInputs` (non-crypto: wages/interest/dividends/business) with a `LedgerState` (crypto events). They
are the SAME vectors the oracle-sweep validates, so a doc built on them inherits that validation. The CLI
path to inject `ReturnInputs` into a vault is **`income import --year 2024 --file inputs.toml`** (a TOML
file, `cli.rs:355-366`, `main.rs:236-239`) — NOT a flag soup, and NOT the interactive `income answer`
(which must be avoided in golden journeys). ReturnInputs *authoring* is most naturally shown in Artifact 2
(the input-form TUI); Artifact 1 shows `income import … --file` then `export-irs-pdf --tax-year 2024`.

## §5. Journeys (broad set; corrected commands)

Journeys pin **relative paths**, `--tax-year`, and per-journey `BTCTAX_NOW`. Exact command scripts are P1
deliverables; the spec-level shape:

**Census authority is J6 alone.** The forms-coverage census (§6) keys off the **full-return packet
stems** (`{seq}_{name}.pdf`) that only the full-return path (J6, TY2024) emits. J1–J5 are TY2025
crypto-slice / `report` demonstrations; the slice writes a **deliberately non-overlapping filename
namespace** (`form_1040_capgains.pdf`, `form_8283.pdf`, `f8949.pdf`, `schedule_d.pdf`,
`schedule_se.pdf`; `admin.rs` dispatch) that carries **no census key** — and the slice has **no Schedule A
filler at all**. So J1–J5 teach the crypto workflow and are golden-diff-gated, but they do **not** count
toward the 14-key census; J6 must emit all 14 (§6.1). `--forms` values are exactly (clap kebab-case of
the `FormArg` variants): `f8949, schedule-d, schedule-se, form8283, form1040` (`cli.rs:900-912`).

| # | Journey | Fixture/corpus | Commands (corrected) | Demonstrates (not census unless noted) |
|---|---------|----------------|----------------------|----------------------------------------|
| J1 | Single-buyer happy path | `coinbase_buy_sell_send` | `init --key-backup ./k.asc` → `import` → `report --tax-year 2025` → `export-snapshot --out ./snap --tax-year 2025` → `export-irs-pdf --out ./irs --tax-year 2025 --forms f8949,schedule-d` | slice `f8949.pdf`, `schedule_d.pdf`, `form_1040_capgains.pdf` |
| J2 | §170(e) donation + lot-selection | `coinbase_two_lot_donation` | `import` → `reconcile set-donation-details …` → `reconcile select-lots <disp> --from …` → `report --tax-year 2025` → `export-irs-pdf … --forms form8283` | slice `form_8283.pdf` (NOT census `f8283`; no Sch A in slice) |
| J3 | Self-transfer reconcile (decision read-back; exercises seam) | **C-self-transfer** | `import` → `reconcile match-self-transfers` / `classify-inbound-self-transfer <in> --basis … --acquired …` → `verify` | seam read-back surface |
| J4 | Income w/ missing FMV + business income | **C-income-csv** + **C-business** | `import` → `reconcile classify-inbound-income <in> --kind staking --fmv …` (and `--business`) → `report --tax-year 2024` | income/SE `report` output |
| J5 | Optimize + what-if (both attestation branches) | **C-multilot** | `optimize run --tax-year 2025` → `optimize consult --sell … --at 2025-06-01` → `optimize accept --tax-year 2025` (BTCTAX_NOW ≤ sale ⇒ Contemporaneous; a second run with BTCTAX_NOW > sale ⇒ NeedsAttestation) → `what-if sell --sell … --at …` / `what-if harvest --target …` | attestation branches, lot-selection |
| **J6** | **Complete TY2024 return — the census journey** | **C-fullreturn** (kitchen_sink + donation leg) | `income import --year 2024 --file inputs.toml` → import donation CSV → `reconcile reclassify-outflow --as-kind donate …` → `reconcile set-donation-details …` → `export-irs-pdf --out ./irs --tax-year 2024` (full-return path) | **all 14 census forms** (§6.1) |

**TY2024 is the all-forms year** (all 14 maps exist; TY2025 has only 5 — f1040/f8949/schedule_d/schedule_se/f8283).
The census journey (J6) uses 2024; the crypto-*slice export* journeys (J1/J2) use 2025; other journeys pin
whichever year their vector needs (J4 uses 2024 for kitchen-sink oracle-consistency — the table governs
per-journey). Different `BTCTAX_NOW` per journey demonstrates both attestation states (J5); the predicate
is *made-date ≤ sale-date* (R-P0.6), not a calendar boundary.

## §6. Forms-coverage census + subcommand report

### §6.1 The census key (14 keys, from `btctax-forms/src/packet.rs::fill_full_return`)
`f1040, f1040s1, f1040s2, f1040s3, f1040sa, f1040sb, f1040sc, schedule_d, f8949, schedule_se, f8995,
f8959, f8960, f8283` (`btctax-forms/src/packet.rs:71-157` — distinct from the `btctax-core/src/tax/packet.rs`
*emission-condition* logic cited in §4.2/§6.1-table). Note schedule D/SE use bare `schedule_d`/`schedule_se`; the
numbered schedules use `f1040s{1,2,3,a,b,c}`. `packet.rs` destructures `PrintedForms` with **no `..`**, so
a new form without a filler is already a compile error (existing anti-drift); the docs census is the
*complementary* guarantee that each emittable form is **demonstrated in the corpus**.

**Per-form emission table (pinned now — closes former §14 gaps 2/3).** Which keys the J6 vault emits
(each `push` is conditional, `packet.rs:71-157`):

| condition | keys | J6 source |
|---|---|---|
| always | `f1040` | any full return |
| Sch 1/2/3 present | `f1040s1`, `f1040s2`, `f1040s3` | kitchen_sink income/SE/other-taxes |
| itemizes | `f1040sa` | kitchen_sink itemized deductions |
| interest/dividends | `f1040sb` | kitchen_sink interest |
| business | `f1040sc`, `schedule_se`, `f8995` | kitchen_sink Sch C business |
| capital disposition | `schedule_d`, `f8949` | kitchen_sink Sell + the donation leg's disposition |
| Add'l Medicare / NIIT | `f8959`, `f8960` | kitchen_sink high income |
| **noncash gift > $500 threshold** | **`f8283`** | **the C-fullreturn donation leg (§4.2) — NOT in kitchen_sink alone** |

kitchen_sink alone = **13/14** (all but `f8283`); the donation leg supplies the 14th. Implementation
verifies this table by filling J6 and asserting the emitted key set == the §6.1 literal 14 (former gap 2
now a P1 assertion, not a deferral).

### §6.2 The gate (HARD, P2) — enumeration mechanism pinned
A `cargo test` / xtask census. The 14-key set is enumerated in **one** of two allowed ways — never by
reading a *household's* emitted packet (that path yields 13 and would make the gate silently under-gate):
1. **all-arms-`Some` fixture** — construct a `PrintedForms` with every optional arm populated, push it
   through `fill_full_return`, collect `NamedForm.name`, and **assert the count == 14** (cross-checked
   against the §6.1 literal); or
2. **compile-checked exhaustive name list** adjacent to the no-`..` destructure in `btctax-forms` (a
   test-support constant the destructure references — a *test-support* addition passing the §3.1 fence,
   NOT an engine edit), asserted == 14.

*Note (fixture authoring):* populating every `Option` arm is necessary but **not sufficient** — three
non-`Option` gates also bind (`sch_d` on `ScheduleDLines::must_file`, `packet.rs:123`; `f8959` on its
internal `must_file`, `:149`; `f8283` double-gated on the filler returning `Some`, `:155-158`). The
mandated `== 14` assert makes any shortfall **loud** (a red, not a silent 13), so the property holds — the
fixture author just must satisfy those gates too.

The census then (a) takes that authoritative 14-key set, (b) scans **the J6 full-return packet manifest
only** — the sequence-prefixed `{seq}_{name}.pdf` stems (`admin.rs:501-506`) — matching on **exact
`{name}` component equality** (so key `f1040` does not substring-match `01_f1040s1.pdf`), (c) **fails** if
any of the 14 is undemonstrated. It must **not** scan the crypto-slice output corpus-wide: three slice
stems (`f8949.pdf`, `schedule_d.pdf`, `schedule_se.pdf`) are byte-identical to census keys (`admin.rs`
seq-prefixed the *packet* precisely because these three collided), so a corpus-wide scan would silently
re-attribute slice output to the census and erode "census authority is J6 alone." Coverage stays
independently guaranteed by the §6.1 `J6 == 14` assertion. Runs inside `cargo test` (like
`manpage_covers_every_subcommand`) AND is re-diffed in CI.

### §6.3 Subcommand-coverage report (SOFT, P2)
A census that lists which of the top-level/`reconcile` subcommands (from `Cli::command()`, the same walk
as `manpage_covers_every_subcommand`, `docs.rs:261`) appear in ≥1 worked example. **Surfaced (printed /
uploaded), non-blocking.** Administrative commands (`backup-key`, `init --repair`, …) need no contrived
example.

### §6.4 New-supported-year ritual
When a year is added to `SUPPORTED_YEARS` (`lib.rs:61`), a covering journey for that year is added in the
same PR (release ritual, like re-pinning). The census MAY key coverage on `(form)` only (current) with a
`SUPPORTED_YEARS` cross-check as a soft report; a hard `(form × year)` gate is explicitly **out of scope**
(combinatorial).

## §7. Artifact 1 — CLI examples generator, golden, render (P1)

- **Generator** — a new `xtask` subcommand (`cargo run -p xtask -- examples`, idiomatic vs. a bash
  `gen.sh`) that, for each journey: builds a fresh tempdir vault from committed synthetic CSVs under the
  §3.3 pinned environment, invokes the **built `btctax` binary** (via `CARGO_BIN_EXE_btctax`-style
  resolution / an explicit `--bin-dir`), captures `$ cmd` + stdout + exit code into fenced blocks
  interleaved with prose, and writes a single deterministic golden.
- **Version pin (corrected — the binary has no `--version`).** `#[command(version)]` is *deliberately
  omitted* (`docs.rs:351-352` depends on it for man-page determinism); `btctax --version` errors. So the
  pin reads the **`btctax-cli` crate version from `crates/btctax-cli/Cargo.toml`** (versions are per-crate;
  the root workspace has no `[workspace.package].version` — plan pins the exact manifest) at regen time and
  embeds it in the golden's front matter — a release bump then reds the CI diff until the golden is
  regenerated (same enforcement, no binary change). Adding `--version` to the binary is explicitly **out of
  scope** (an S2 escalation, not a silent docs side effect).
- **Golden** — committed text (`docs/examples/examples.md` or `.roff`), whole-file. Regen is a pure
  function of `(repo tree, binary, synthetic CSVs)`.
- **Render** — wrap captured verbatim blocks in roff `.nf/.fi`; render via the existing `groff -k -man -T
  pdf` path (extend `write_pdfs`/`make docs`); PDF is not byte-gated (groff/gs-parity honesty gap — §13),
  release-attached. A new `make examples` / `make bundles`-style target.
- **The born-green atom (a `cargo test`, lands in the SAME commit as the golden — closes I6).** A test
  asserting **`regen == committed golden`, byte-for-byte** — modeled on the committed-match half of
  `gen_docs_is_deterministic` (`docs.rs:351-368`), which the double-regen proof alone omits. This makes
  the golden gated *in-tree* from P1 onward (a local `cargo test` / `make check` detects drift), not only
  by the P2 CI `git diff`.
- **Determinism proofs (the other "tests")** — double-regen byte-identity; regen under two `$HOME` values;
  regen with/without a machine price cache present (identical because `BTCTAX_PRICE_CACHE`→nonexistent).

## §8. Artifact 2 — TUI style-aware capture, golden, render (P3)

- **Capture harness** — extend the `TestBackend` pattern (`tabs/tests.rs`) into a journey driver that, for
  `btctax-tui` tabs and `btctax-tui-edit` reconcile flows, drives real events via the existing
  `handle_key(app, press(...))` / `type_str` harness and snapshots each `Buffer`. **Style-aware:** capture
  per cell `(symbol, fg, bg, modifier)` — new ground; ratatui 0.29 exposes `Cell.symbol()` + public fields
  `fg`/`bg`/`modifier` (no `style()` getter). Serialize as (a) a glyph grid + (b) a compact style map
  (format pinned in P3 — candidate: a per-cell run-length style overlay keyed to the grid).
- **TUI seam prerequisite** — the §3.4 shared clock helper lands in P3 before any TUI golden is recorded.
- **Golden + render** — committed text goldens under `docs/examples-tui/`; groff render into a **separate**
  PDF. *(r2 amendment, 2026-07-18, folding the P3 review M-2:* the shipped `make examples-tui` render is
  **monochrome** — the goldens' glyph grid, box-drawing mapped to ASCII. The `.txt` goldens carry the full
  per-cell `fg`/`bg`/`modifier` (they are the gated artifact); a **colorized** groff render driven by the
  style overlay is an explicitly DEFERRED enhancement (FOLLOWUPS **UX-P3-2**), not a P3 deliverable — the
  PDF is a git-ignored convenience render with no consumers.*)
- **This is the primary bug-hunt surface** — driving the edit reconcile flow is the btctax analogue of the
  mnemonic `(none)`/reveal-toggle discoveries.

## §9. CI gate (P2)

- **Triggers** — `ci.yml` already fires on `push` + `pull_request` with **no paths filter** (`ci.yml:3-6`),
  so "wide triggers / leading indicator" is satisfied for free; a new job needs no path config.
- **New job `examples`** (sibling to `test`/`clippy`) — build the binary, run `xtask examples` (+ the TUI
  capture) under the pinned env, then **`git diff --exit-code docs/examples docs/examples-tui`**; run the
  forms-census (hard) and print the subcommand report (soft); prove each PDF *builds*.
- **Posture** — born-green **advisory** (the diff/census run but are not yet a required merge check), then
  **promote to required** in GitHub branch-protection settings once stable, adding the mnemonic fail-safe
  internal guard (compute `relevant` from a two-dot `git diff`, hard-fail on fetch error) so a required +
  future-path-filtered check can't wedge. **Branch protection is a GitHub-settings action (not in-tree —
  §14 gap 1); the SPEC flags it, the user actions it.**
- **Born-green rollout (rescoped per I6).** The **P1** golden is already born-green in-tree via its
  `regen == committed` `cargo test` (§7), landing in the same commit as the golden — that test is the
  atom, not a P2 CI job. **P2** then lands the CI `examples` job + the forms-census + the subcommand
  report in one commit, with a mandatory negative proof (perturb one golden byte → observe RED at the diff
  step → revert) before ship. (The P1 golden is therefore never ungated between P1-close and P2.)

## §10. The workaround-audit (P4 — the co-equal, budgeted bug-hunt)

A deliberately skeptical journey author drives the full assembled surface (esp. the P3 edit reconcile
flows) and produces `reviews/tutorial-workaround-audit.md`: every route-around catalogued and classified
**bug-to-file / harness-artifact / intentional**, each real bug filed to `FOLLOWUPS.md` with severity +
owning phase. This is a scheduled deliverable with its own budget, not a side effect. Standing behavioral
assertions the journeys encode (e.g. a refusal we want to stay a refusal) are kept **live and gated** so
the example doubles as a regression detector.

## §11. Phase plan & review cadence

Each "→ green" is an independent **Fable** review to 0C/0I, persisted verbatim under `reviews/` before the
fold; re-review after every fold including the last. Green = full validation suite passes AND 0C/0I.

| Phase | Deliverable | Gate |
|---|---|---|
| **P0** | `BTCTAX_NOW` seam + fence + integrity KAT (§3.2) | green **before any golden** |
| **P1** | CLI generator + journeys + NEW corpora + golden + `regen==committed` test (§4.2, §7) | golden born-green in-tree (its own `cargo test`); double-regen byte-identity; Fable 0C/0I |
| **P2** | CI `examples` job + forms-census (enumeration pinned §6.2) + subcommand report + perturb→red (§6, §9) | negative proof observed; Fable 0C/0I |
| **P3** | TUI style-aware capture + TUI clock seam + goldens/gate (§3.4, §8) | determinism proof; Fable 0C/0I |
| **P4** | regen + ship + workaround-audit → FOLLOWUPS (§10) | whole-diff Fable review; audit filed |

Follow-up burndown is per-phase by ownership (UX-P0-1 in P0, UX-P1-1 in P1, UX-P3-1 in P3). A phase-owned
item is not deferrable past its owning phase.

## §12. STOP ledger (user-decision tripwires — halt & escalate)

- **S1 — any change to tax numbers, form-cell values, output wording, or persisted schema** to make a
  golden regenerate cleanly. This fails the §3.1 fence ⇒ halt; it is an engine edit, not a doc fix.
- **S2 — a journey requires a btctax code change beyond the P0 seam / the P3 TUI seam.** Halt; re-apply the
  §3.1 fence and escalate (candidate FOLLOWUP, not an inline edit).
- **S3 — a determinism proof fails to converge** (output varies across runs/hosts after the §3.3 pins).
  Halt; a new nondeterminism source was found — file it, don't paper over it.
- **S4 — a forms-census gap can only be closed by an unsupported year or a contrived/non-representative
  scenario.** Halt; reconsider the journey set with the user rather than fabricate coverage.
- **S5 — the integrity fence (§3.2 R-P0.6) would need weakening** (e.g. a reviewer asks to drop the banner
  or the KAT). Halt; escalate — the disclosure is load-bearing.

## §13. Honesty section (what the gate catches / does NOT)

- **Catches:** any drift between the committed golden and a fresh regen — a changed output format, a
  refusal-message change, an **undemonstrated emittable form** (forms-census), a hand-edit regen doesn't
  reproduce, a version bump without regen (the Cargo.toml-sourced front-matter pin, §7) — printed verbatim
  in the CI log. A **removed** subcommand is caught only if a journey used it; a **new** subcommand surfaces
  only in the SOFT §6.3 report (non-blocking) — the census never *demands* an example per subcommand.
- **Does NOT catch (declared gaps):** (a) the **PDF is not byte-gated** (groff/gs not byte-reproducible;
  the golden text is the gated artifact, the PDF is only re-proven to build — same accepted gap as
  mnemonic's xelatex). (b) **Narration-truth about *unchanged* output** — Gate A (whole-file regen) does
  not prove prose describing adjacent output is *true*, only that captured output didn't change; the
  stronger per-command transcript model (mnemonic's "gate B") is **deferred** and named here so it isn't
  mistaken for solved. (c) **Branch-protection required-status** is a GitHub-settings fact not verifiable
  in-tree (§9, §14 gap 1). (d) **stderr** is captured only in labelled blocks where relevant (§3.3), not
  wholesale — a doc titled "verbatim I/O" shows stdout+exit; the R-P0.4 banner and notices are disclosed,
  not silently dropped. (e) The capture uses `BTCTAX_PASSPHRASE` where a real user is prompted — an honest
  doc sentence, not a gate.

## §14. Open items / gaps for implementation to verify

1. **Branch protection / required checks** — not representable in-tree (only `ci.yml`; no CODEOWNERS/
   rulesets). Confirm/action in GitHub settings at P2 promotion.
2. *(Closed by the r0 review fold — the per-form emission table is now pinned in §6.1; kitchen_sink = 13/14,
   the donation leg supplies `f8283`. P1 verifies by asserting the J6 emitted key set == the 14.)*
3. *(Closed — see item 2 and §6.2's enumeration mechanism.)*
4. **TUI `now_utc()` production-vs-test split** — spot-check the mid-file `main.rs` reads (e.g. `:6123`,
   `:9218`) against their enclosing fn before citing them as production seams (P3).
5. *(Resolved — `config --set-forward-method`'s handler is `cmd/reconcile.rs:968` `set_forward_method(…,
   now)`, called with `now` at `main.rs:470`; when `--effective-from` is omitted it defaults to the
   made-date. Decision-bearing / clock-dependent → BTCTAX_NOW-covered. ReturnInputs path is `income import
   --file inputs.toml`, §4.3.)*
6. **TY2025 full form set** — confirmed: TY2025 bundles only 5 maps (f1040/f8949/schedule_d/schedule_se/f8283);
   the all-forms census journey (J6) uses **TY2024** (all 14). No open question — recorded for the
   new-supported-year ritual (§6.4).
7. *(DECIDED in P3 — `capture.rs` uses a glyph grid + a per-row RLE style overlay. `underline_color`/`skip`
   are NOT captured: neither varies in the current TUI (the only underline is `Modifier::UNDERLINED`, which
   IS captured); re-open if a screen adopts a per-cell `underline_color`/`skip`. Recorded in `capture.rs`
   module docs, M-1.)*

---

## §15. r2 amendments — journey-content descopes (folding the P1 Fable review, finding I-2)

*Recorded 2026-07-18. (a)–(d) fold `reviews/p1-fable-review.md` I-2; **(e)–(f) fold the whole-branch review
`reviews/whole-branch-fable-review.md` I-1** — a fifth and sixth deviation the P1 pass missed. Each of the
SIX §4.1/§4.2/§5/§6.1 journey-content mandates below could not be delivered as spec'd; each is amended here
with its discovered-reality rationale so the shipped doc no longer contradicts a green, unamended spec. The
oracle-equality guarantee (the committed fixture == `kitchen_sink_household().0`, pinned by
`fullreturn_oracle.rs`) is untouched by all of them.*

- **(a) J4 — the import-produced *missing-FMV* demonstration is dropped; `classify-inbound-income --fmv`
  is not yet demonstrated.** §5 J4 / §4.2 C-income-csv called for a CSV that imports to *missing-FMV*
  income, then priced via `classify-inbound-income … --fmv …`. **Reality:** the bundled daily-close dataset
  is dense through 2026-06, so an import on any *supported* year (2017/2024/2025) auto-resolves FMV — an
  import-produced missing-FMV requires an unsupported year, which is the §12 S4 tripwire shape (a
  demonstration closable only by an unsupported year ⇒ record, don't force). J4 therefore shows the
  business/SE reclassification on auto-resolved income and reduces missing-FMV to a prose aside. As a
  consequence J4 also moves off the spec's **year 2024** (chosen "for kitchen-sink oracle-consistency")
  to **2025** dates — the auto-resolve needs an on-dataset supported year, and J4 no longer shares the
  kitchen-sink oracle (only J6 does), so the year alignment it was for no longer applies. The *manual*
  pricing verb `classify-inbound-income --fmv` (valid against an unclassified income Receive, J3's corpus
  shape) remains **undemonstrated anywhere** — filed as **UX-P1-7** for a future journey; not a P1 blocker.
- **(b) J5 — only the Contemporaneous attestation branch is demonstrated; the made-after-sale branch stays
  prose.** §5 J5 asked for *both* branches, predicting a postdated `optimize accept` ⇒ `NeedsAttestation`.
  **Reality (verified against the binary):** a *first-time* post-sale accept prints `skipped … re-run …
  --attest "<genuine contemporaneous ID>"` (not a persisted `NeedsAttestation`), and a *re-accept* of an
  already-contemporaneously-accepted disposal (J5's state after its main accept) reports `already optimal
  under current identification` — the clock is irrelevant there. Cleanly demonstrating the made-after-sale
  branch needs a **separate, never-accepted** disposal; J5 keeps it in prose (which the review confirmed is
  factually accurate). The spec's `NeedsAttestation` prediction is **corrected** to the skip/`--attest`
  behavior above.
- **(c) J6 — its crypto side is built from a small synthetic ledger, not `kitchen_sink_household().1`, and
  the §4.2 "non-donation figures remain the oracle vector" caveat is DROPPED.** §4.2 C-fullreturn / §6.1
  sourced `schedule_d`/`f8949` and `f1040sc`/`schedule_se`/`f8995` from kitchen_sink's own `LedgerState`
  (1 BTC mining @ $20k + a $20k-gain LT sale) and mandated a doc caveat that the composite deviates from
  the oracle vector "by exactly the added donation delta." **Reality:** there is **no CLI path to inject a
  `LedgerState`** — a journey can only build a ledger through `import`+`reconcile` — and a
  kitchen-sink-faithful ledger sits only ~$1.7–3.3k under the 2024 Form-6251 AMT screen, so adding any
  donation deduction trips `AmtScreenTriggered` and *refuses the export* (see the review's V-1). J6 there­
  fore uses a deliberately small crypto ledger (mining ≈ $3,438; LT gain ≈ $1,630; a $6,000 donation) with
  ≈ $17k of AMT headroom. The spec'd caveat sentence is therefore **false as written** (the non-donation
  crypto figures are *not* the oracle vector) and is **removed**, not printed. §6.1's per-form source
  attributions are amended: **J6 sources the crypto forms from its own synthetic ledger**; only the
  non-crypto forms + the ReturnInputs come from kitchen_sink. The `.0` oracle-equality test still holds.
- **(d) J3 — a single-exchange Receive, not the spec'd two-exchange CSV pair; `match-self-transfers` stays
  undemonstrated.** §4.2 C-self-transfer envisioned a two-exchange pair to enable `match-self-transfers`.
  J3 uses the single-exchange `classify-inbound-self-transfer` path — **within** §5 J3's stated either-or,
  so the demonstrated verb is spec-compliant — but the matched-pair `match-self-transfers` workflow is
  consequently undemonstrated. Filed as **UX-P1-8** for a future journey; not a P1 blocker.
- **(e) J2 — the `select-lots` + `report` steps are dropped; §4.1 corpus re-authored.** §5 J2 mandated
  "§170(e) donation **+ lot-selection**" with `… reconcile select-lots <disp> --from … → report
  --tax-year 2025 → export-irs-pdf …`. As shipped, J2 is init → import → reclassify-outflow →
  set-donation-details → verify → export-irs-pdf — **no `select-lots`, no `report`**. Reality: J2 donates
  the FULL 2-BTC balance across both lots, so `select-lots` is **degenerate** — there is no lot choice left
  to demonstrate. The branch's actual lot-selection demonstration is **J5** (`optimize run`/`accept`, a
  genuine HIFO-vs-FIFO changed selection). `select-lots` is therefore **undemonstrated anywhere** in the
  golden — filed as **UX-P1-10** (a future journey; the SOFT coverage report already lists it). §4.1's
  "reuse `coinbase_two_lot_donation` as-is" also could not hold, but NOT for (c)'s reason: the §4.1
  builders (`coinbase_buy_sell_send`, `coinbase_two_lot_donation`, `fixtures.rs:8/:50`) are btctax-cli
  **test-tree CSV writers** (`crates/btctax-cli/tests/fixtures.rs` — `std::fs::write` a CSV into a tempdir;
  only `income_fmv_missing_batch`, §4.1 line 5, returns `Vec<LedgerEvent>`), and test-tree code is not a
  linkable API — the xtask examples generator, a separate crate, cannot call it. So each journey embeds its
  own CRLF-const CSV (which the `.gitattributes` LF-normalization trap forces anyway, per the P1 learnings).
  Figures unaffected. *(Corrects M-R1: the whole-branch review's own I-1 rationale mis-cited these builders
  as `Vec<LedgerEvent>` constructors — they are CSV writers; the reuse-blocker is test-tree linkage.)*
- **(f) J1 — the Send→pending-TransferOut leg is dropped; §4.1 corpus re-authored.** §4.1's
  `coinbase_buy_sell_send` (reuse as-is for J1) carries a `Send` leg (a pending outbound transfer). Shipped
  J1 is a clean buy→sell→report→export happy path with no Send leg (the single-buyer story is clearer
  without an unreconciled outbound), on a re-authored CSV for the same import-vs-builder reason as (e).

---

*End SPEC r1 — Fable re-review GREEN (0C/0I). §15 added at r2 (P1 fold) + extended (e)/(f) at the
whole-branch fold (2026-07-18); the P1 re-review-2 and the whole-branch re-review have since closed GREEN
(0C/0I).*
