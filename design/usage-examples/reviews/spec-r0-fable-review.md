# Fable independent review — SPEC_usage_examples.md r0 (persisted verbatim)

*Persisted 2026-07-16 verbatim before folding, per STANDARD_WORKFLOW §2. Reviewer: Fable (independent;
author was Opus). Verdict: NOT GREEN — 0 Critical / 6 Important / 7 Minor / 4 Nit. Fold → re-review to
0C/0I.*

---

# Independent Fable review — SPEC_usage_examples.md (r0)

**Reviewed against:** HEAD `ac04ce2` (same commit the spec claims its citations were verified against — so every discrepancy below is an authoring error, not decay). Verification was done directly against source and the built binary, including a live probe of the clap surface.

## VERDICT: **NOT GREEN — 0 Critical / 6 Important / 7 Minor / 4 Nit**

---

## VERIFIED (independently re-checked, correct as cited)

- `crates/btctax-cli/src/main.rs:66` sole production clock read in the CLI crate; `BTCTAX_PASSPHRASE` at `main.rs:50`; no `BTCTAX_NOW` anywhere yet; `verify` hard-blocker exit 1 at `main.rs:89-91`; CliError → exit 2 pattern exists.
- `render.rs:2258` MethodElection `recorded` print; `session.rs:1134` bulk-void preview date (genuinely clock-derived: rows are decision events stamped by `append_decision(now)`); `main.rs:~2005` is inside `render_bulk_void_preview`.
- `packet.rs::fill_full_return` — exactly 14 keys, names as listed in §6.1, no-`..` destructure with the anti-drift comment (`packet.rs:37`); `SUPPORTED_YEARS = [2017, 2024, 2025]` (`lib.rs:61`).
- **TY2024 has all 14 maps** (`crates/btctax-forms/forms/2024/` — enumerated all 14 `.map.toml`). TY2025 has only 5 (f1040, f8949, schedule_d, schedule_se, f8283) — consistent with §5's "crypto-only journeys use 2025" and §14 gap 6.
- `config --set-forward-method` is a flag (`cli.rs:92-103`); `accept-conflict`/`reject-conflict` at `cli.rs:573/575`, `bulk-resolve-conflict` at `:792`; no `resolve-conflict` verb; **no reconcile verb takes `--date`**.
- Fixtures at `fixtures.rs:8/:25/:50/:71/:103`, exactly the 5 builders described; `income_fmv_missing_batch` is library-only as stated.
- `BTCTAX_PRICE_CACHE` is real (`btctax-cli/src/price_cache.rs:9,20`) — the §3.3 pin is meaningful.
- TUI: `lib.rs:247,256`; `export.rs:30` (`export_dir_for`, timestamped on-screen dir); `tui-edit/src/main.rs:2609`; `handle_key` (`main.rs:126`), `press`/`type_str` test harness exist; `TestBackend::new(120,40)`; all current buffer readers take `.symbol()` only; ratatui 0.29.0 `Cell` has pub `fg`/`bg`/`modifier` + `symbol()` getter, no `style()` getter (checked vendored source).
- `groff -k -man -T pdf` at `docs.rs:75-78`; `gen_docs_is_deterministic` (`docs.rs:353`), `manpage_covers_every_subcommand` (`docs.rs:261`); `docs/pdf` git-ignored rationale (`docs.rs:33-34`); `make bundles`/gs (`Makefile:46-49`).
- `ci.yml` fires on bare `push:` + `pull_request:`, no paths filter — "wide triggers for free" holds.
- `admin.rs:204` `export_irs_pdf` **does** dispatch to the full-return pipeline when `return_inputs::exists` (`admin.rs:226-227`) and writes `{seq}_{name}.pdf` stems (`admin.rs:501-505`); `testonly.rs` is a `pub mod` with households at `:165/:320/:367/:500`; `optimize.rs:469-484` `persistability` semantics as claimed; core is clock-free in production (`persistence.rs:430` is `#[cfg(test)]`); adapters' sole committed CSV is `btc_usd_daily_close.csv`; River adapter exists; J3/J4 classify flags (`--basis/--acquired`, `--kind/--fmv/--business`) exist; `optimize run/accept/consult` and `what-if sell/harvest` shapes match §5.
- **P0 fidelity to the ruling: faithful.** R-P0.1–R-P0.6 and T-P0.1–T-P0.6 map 1:1 onto the ruling's scope/TDD/integrity sections; the (i)/(ii)/(iii) fence is verbatim (the "behaviorally *byte*-identical" tightening is a strengthening, not drift); banner text exact; vault-marker rejection and no-pseudo/DRAFT-entanglement carried; CLI-only scope preserved; gate "P0 green before first golden" intact. No weakening found anywhere. The ruling's "TUI-doc's own spec" language is honored in substance by §3.4/§8/P3-with-its-own-review; keeping one spec is the right unit (splitting would duplicate the fence/census/CI plumbing and break the single dependency graph the ruling endorsed).

---

## Critical

None.

---

## Important

### I1 — The forms-census is unsatisfiable by the specced corpus: kitchen-sink can never emit `f8283` (SPEC §4.2, §5 J2/J6, §6.2)
`f8283` in the full-return packet is **ledger-driven**: `btctax-core/src/tax/packet.rs:537-540` emits it only when the return itemizes, Schedule A line 12 (noncash) exceeds the $500 threshold, and `crate::forms::form_8283(state, year, donation_details)` yields rows — i.e., it requires **crypto donations in the LedgerState**. `kitchen_sink_household()` (`testonly.rs:165`) has a Cash60-only gift and a ledger with one Sell and one Mining income event — **no donation**. So J6-on-kitchen-sink emits 13 of 14 keys. No other journey covers it: J2 is a crypto-slice year (TY2025, no return inputs), and the slice writes **`form_8283.pdf`** (`admin.rs`, non-overlapping filenames by design), which does not carry the census key `f8283`; a TY2025 full-return can't rescue it either (2025 lacks 9 of the 14 maps). §4.2's claim that C-fullreturn provides what is "needed to emit all 14 forms, sourced from `kitchen_sink_household()`" is unsound, and §14 gaps 2/3 defer to P1 the exact feasibility fact that invalidates it — that is a decision needed NOW, not a verify-later.
**Fix:** redesign C-fullreturn at spec level: J6's vault = kitchen-sink ReturnInputs **plus a donation-bearing crypto ledger leg** (import a donation CSV + `reconcile reclassify-outflow --as-kind donate` + `set-donation-details`, amounts chosen so Sch A noncash > $500), with an explicit one-sentence caveat that the composite deviates from the oracle-validated vector on the donation delta; then close §14 gaps 2/3 by pinning the per-form emission table in the spec (it is now known: kitchen-sink alone = 13/14).

### I2 — §6.2's "enumerate the keys from the packet source" is under-specified, and the natural shortcut silently shrinks the gate (SPEC §6.2)
The only programmatic way to get form names out of `fill_full_return` is to fill a `PrintedReturn` and read `NamedForm.name` — and if the implementer uses the anointed household for that, they get **13 keys** (per I1), the census enumerates 13, demands 13, and goes green with `f8283` never demonstrated: the hard gate silently under-gates and goal (c) is unmet without any red. This is the one path by which I1 becomes *silent* rather than loud.
**Fix:** pin the mechanism in the spec: enumeration must come from an **all-arms-`Some` fixture** pushed through `fill_full_return` (asserting count == 14), or a compile-checked exhaustive name list adjacent to the no-`..` destructure in `btctax-forms` (a test-support addition, not an engine edit — say so against the §3.1 fence). Explicitly forbid deriving the key set from any household's emitted packet, and cross-assert the enumerated count against §6.1's literal 14.

### I3 — The §7 FATAL `btctax --version` pin is unimplementable: the binary has no `--version` (SPEC §7, §13)
Verified live: `btctax --version` → `error: unexpected argument '--version' found`. The `Cli` derive deliberately omits `#[command(version)]` — `docs.rs:351-352` documents man-page determinism as depending on exactly that ("no dates / no `#[command(version)]`"). Implementing §7 as written forces a code change beyond the P0 seam (an S2 halt by the spec's own ledger) that would also embed the version into every clap-generated man page, reversing a deliberate prior design decision and churning `docs/man/*` on every release.
**Fix:** source the pinned version from the workspace `Cargo.toml` (xtask reads it at regen; embeds it in the golden's front matter; a release bump then reds the CI diff until regen) — no binary change. If the author genuinely wants `--version` on the binary, that is a separate user-decision escalation, not a silent side effect of a docs cycle.

### I4 — The §3.2 leak-site inventory is wrong in substance: `bulk-resolve-conflict` previews are NOT clock-derived, and `session.rs:1183` is a different function (SPEC §3.2, §0)
`session.rs:1183` sits inside `self_transfer_match_plan` (`:1154`), not a bulk-resolve preview; its dates come from imported `TransferIn` events (CSV-derived). The actual bulk-resolve date is `session.rs:1097` inside `bulk_resolve_conflict_plan` — and it is **also deterministic**: an ImportConflict event's `utc_timestamp` is copied from the conflicting CSV row (`persistence.rs:215-226`, `utc_timestamp: ev.utc_timestamp`), never from `now`. The spec (front-matter: "All file:line citations verified against HEAD ac04ce2") inherited this error from the ruling and re-asserted it as verified. It matters beyond hygiene: a P0 plan-writer choosing bulk-resolve-conflict as the T-P0.2/T-P0.5 read-back surface would write a twice-run test that passes **without the seam** — a test that doesn't test the fix (the project's named failure pattern).
**Fix:** correct the inventory to: `verify` election `recorded` (`render.rs:2258`), **bulk-void** preview (`session.rs:1134` → `main.rs:~2005`), `config --set-forward-method`'s recorded date (`cmd/reconcile.rs:968`, receives `now` from `main.rs:470`), and the attestation/what-if surfaces (`optimize.rs:469-484` → its render consumer; the "defaults to today UTC" `--at` flags). State explicitly that bulk-resolve-conflict and match-self-transfers preview dates are CSV-derived and must NOT be used as seam-proof surfaces.

### I5 — §5's "corrected commands" contain commands that fail as written, and the anchor-forms column mis-attributes census coverage (SPEC §5)
(a) J1 `init` — `--key-backup <path>` is **required** (`cli.rs:39`, no default); bare `init` errors. (b) J1 `--forms f8949,schedule_d` — invalid; verified against the binary: possible values are `f8949, schedule-d, schedule-se, form8283, form1040` (underscore stems and hyphen/no-hyphen flag values are three different namespaces; J2's `form8283` happens to be right). (c) Anchor-forms mis-attribution: J1's "f1040" is emitted by the slice as `form_1040_capgains.pdf` (deliberately non-overlapping with the packet's `f1040` — `admin.rs` dispatch comment), so it never matches the census key; J2's "f8283, f1040sa" — the slice writes `form_8283.pdf`, and there is **no Schedule A filler in the slice at all**; J2 cannot demonstrate `f1040sa`, period. (d) J5's fixture is annotated "(multi-lot)" but `coinbase_buy_sell_send` has exactly one Buy (§4.1's own description) — a single-lot vault makes the lot-selection optimizer demo degenerate, and §4.2 authors no multi-lot corpus. In a spec whose product is verbatim command transcripts, these are defects, not typos.
**Fix:** correct (a)/(b); rewrite the anchor column so census keys are attributed only to journeys that emit packet stems (post-I1, that is J6 for everything except `f8949`/`schedule_d`/`schedule_se`); add a multi-lot corpus item to §4.2 for J5 (or repoint J5 at `coinbase_two_lot_donation`'s two-lot shape).

### I6 — Born-green atomicity contradicts the phase plan, and §7 omits the stale-golden check its own cited model includes (SPEC §7, §9, §11)
§9 requires "the golden `git add` + the census + the CI job land in ONE atomic commit," but §11 makes the golden a P1 deliverable and the census/CI job P2 deliverables — both cannot hold if phases close with commits (this repo's phases do). Worse, §7's determinism proofs (double-regen, cross-HOME, cache-absence) omit the **committed-vs-fresh-regen** assertion that its explicitly cited model `gen_docs_is_deterministic` performs ("AND the committed pages match a fresh generation (fails on stale docs)", `docs.rs:351-368`). As written, between P1-close and P2 the committed golden is gated by nothing in-tree, and even after P2 a local `cargo test` / `make check` never detects golden drift — only the CI job's `git diff` does. That is a gate hole in the project's own "whole validation surface" sense.
**Fix:** make the P1 proof a cargo test asserting `regen == committed golden` (byte-for-byte), landing **in the same commit as the golden** — that test *is* the born-green atom, from P1 onward. Then rescope §9's atomic-commit language to the P2 wiring (CI job + census in one commit, with the perturb→red proof).

---

## Minor

- **M1 (§4.3/§14-5)** — §4.3's "(§14 gap 5 verifies the exact flag mapping)" points at the wrong gap (gap 5 is `--set-forward-method`), and the real ReturnInputs path is `income import --year Y --file inputs.toml` (TOML, not flags; `cli.rs:355-366`, `main.rs:236-239`) plus TOML-carried live-declaration answers (interactive `income answer` exists but must be avoided in golden journeys). Gap 5 itself is resolvable now: the handler is `cmd/reconcile.rs:968`, called with `now` at `main.rs:470` — confirmed decision-bearing and BTCTAX_NOW-covered.
- **M2 (§3.4)** — Count drift ("~23" vs the ruling's ~28-30; grep: 26 in `tui-edit/main.rs` + 1 in `edit/persist.rs` + 2 in `tui/lib.rs`, incl. ≥1 test site), and the "plus three that reach rendered output" framing is misleading: a decision timestamp persisted by any of the other reads can surface in a later rendered frame in the same session. The P3 helper should route **all** production reads, not just the three.
- **M3 (§2)** — Render row says `groff -k -T pdf` with no macro package; the real pipeline is `groff -k -man -T pdf`. Pin the macro package for the examples renders.
- **M4 (§13)** — "Catches … a new/removed subcommand" overstates: a NEW subcommand surfaces only in the soft §6.3 report (non-blocking); removal is caught only if a journey used it. Reword to match §6.3.
- **M5 (T-P0.6)** — The integrity KAT must pick a non-broker wallet or a pre-2027 sale: `ForbiddenBroker2027` precedes `ContemporaneousNow` in `persistability` (`optimize.rs:476-480`), else the KAT premise fails.
- **M6 (§3.3/§13)** — Goldens capture stdout+exit only; stderr (the R-P0.4 banner, 1099-DA notices, SE/8959 notes, export warnings) is silently absent from a doc titled "verbatim I/O." §13(d) discloses only the passphrase wrinkle. Add the stderr-scope sentence the ruling's adjacent-finding 3 explicitly asked for.
- **M7 (pre-existing collision J6 will expose)** — `export-irs-pdf`'s help/man still says "REFUSED for a tax year that has FULL-RETURN inputs … Transcribe the report's figures by hand until the full-return fillers ship" (`cli.rs` doc comment), contradicting the P6.5 dispatch (`admin.rs:216-227`) that J6 demonstrates. The shipped doc set would contain a man page contradicting a transcript. This is a message-wording fix, i.e., §3.1-fence-fails class → file to FOLLOWUPS with an owning phase (P1), not an inline edit.

## Nit

- **N1** — ratatui 0.29 `Cell` also carries `underline_color` (and `skip`); the (symbol, fg, bg, modifier) tuple drops them — note in the P3 format decision (§14 gap 7).
- **N2** — J5's "during-year / after year-end" shorthand: the real predicate is made ≤ sale date (the spec states it correctly in R-P0.6; align J5's wording).
- **N3** — Upstream doc nit for the bug-hunt: `cli.rs:197-198`'s own doc comment says "form-8283"/"form-1040" while the actual clap values are `form8283`/`form1040`.
- **N4** — §6.2 stem matching: pin exact name-component equality on `{seq}_{name}` (key `f1040` must not substring-match `01_f1040s1.pdf`).

---

## The single most important thing to fix

**I1 (with its silent-failure twin I2): the spec's central hard gate — every emittable form demonstrated — cannot be satisfied by the specced corpus, because `f8283` is ledger-donation-driven and the kitchen-sink vector has no donation; and the underspecified census-enumeration mechanism offers exactly one tempting implementation that would make this failure silent instead of loud.** Redesign C-fullreturn/J6 to carry a donation-bearing ledger leg, pin the per-form emission table now (it is fully knowable at HEAD), and pin the all-arms key-enumeration mechanism — before any of P1 is planned, because it changes the corpus deliverables, the journey table, and the census design all at once.
