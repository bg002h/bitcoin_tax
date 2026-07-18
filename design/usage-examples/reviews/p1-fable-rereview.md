# P1 re-review — fold of the P1 Fable review (independent Fable re-review)

*Reviewer: Fable (independent; author = the build agent). Scope: the fold commit `5b5dccf` (the only
commit since the reviewed base `b9fb8aa`) + the full folded state `git diff b9fb8aa..HEAD`, re-verified
against live source and the real binary — not the commit message. Date: 2026-07-18.*

## Verdict

**Every folded finding genuinely resolved — I-1 and I-2 are closed on the merits, and all six Minors and
four folded Nits are real fixes, not cosmetic claims — but the N-4 fold introduced one NEW Important: the
added front-matter sentence asserts stderr "is never dropped," which is demonstrably false (four pinned
steps' clock banners are dropped), inverting the very SPEC §3.3/§13(d) disclosure it was written to
provide. 0 Critical / 1 Important → P1 is NOT green.** The fix is one clause + regen.

## Validation surface

- `make check`: **green** — 1944 passed / 0 failed / 6 skipped, 14.1s. All three gate tests ran and passed,
  including the strengthened `examples_generate_is_hermetic_across_ambient_env` (now perturbing
  `BTCTAX_NOW=2099…` alongside `HOME` and the price cache).
- `cargo test -p btctax-cli --test fullreturn_oracle`: green (1 passed, 1 ignored) — against the **moved**
  fixture path.
- Working tree clean; `crates/xtask/tests/` no longer exists (the move left no residue).
- Live-binary checks run for the attestation branches (r1) and the R-P0.4 banner (below).

## Folded findings — resolution status

### I-1 — RESOLVED
`examples.rs:483` emits "The outbound **0.1 BTC** is a §170(e) charitable donation"; `examples.md:408`
ships it; the corpus (`examples.rs:228-232`) donates `0.10000000` BTC and the adjacent verify shows
`removed 10000000` sats with `--amount 6000.00` = 0.1 × the corpus $60,000 spot. **Whole-golden figure
sweep re-run:** J1 (−350/−77; ST proceeds 1340 / basis 1690 = 0.02 × 8450/0.1), J2 (`--amount 217992.34`
= 2 × 108,996.17; §170(e) deduction $110,996.17 = 108,996.17 LT-FMV + 2,000 ST-basis), J4 (7,450.67 =
0.05×85,484.60 + 0.03×105,881.32; ordinary 1721.16 = 3,350×0.22 + 4,100.67×0.24 **exactly**; the SE chain
6,880.69 / 853.21 / 199.54 / 1,052.75 / 526.38 all exact), J5 (3000 / −660 / −3660; what-if 1,932.71 =
8,484.75×0.15 net-LT + 3,000×0.22 recovered ordinary offset — exact), J6 (35M-sat conservation; $6,000 =
LT→FMV; §15's mining figure 3,437.95 = 0.05 × the dataset's 2024-03-15 close 68,759.09; LT gain 1,630 =
3,130 − 1,500). **No remaining quantity or figure disagrees with its corpus, command, or the dataset.**

### I-2 — RESOLVED (all four amendment paragraphs factually correct against source/binary)
SPEC §15 (r2) records descopes (a)–(d) with their rationales; each was independently verified:

- **(a) TRUE.** The bundled dataset (`crates/btctax-adapters/data/btc_usd_daily_close.csv`) runs
  2010-07-17 → 2026-06-03 with **no gap months** (density checked mechanically; only the two partial
  boundary months). `SUPPORTED_YEARS: &[i32] = &[2017, 2024, 2025]` (`btctax-forms/src/lib.rs:61`) —
  §15(a)'s parenthetical matches the source constant exactly, and an off-dataset date necessarily falls
  after 2026-06-03, i.e. in non-supported 2026 — so "an import-produced missing-FMV requires an
  unsupported year" holds, and the §12 S4 shape ("closable only by an unsupported year") is quoted
  accurately. UX-P1-7 filed with owner.
- **(b) TRUE.** `cmd/optimize.rs`: the `proposed == current` skip — `"already optimal under current
  identification"` (`:225`) — fires **before** the persistability match, so a re-accept of J5's
  already-accepted disposal reports already-optimal regardless of clock; a **first-time** post-sale accept
  hits `Persistability::NeedsAttestation` without `--attest` and skips with `"already executed — re-run
  \`optimize accept --disposal <ref> --attest \"<genuine contemporaneous ID>\"\`"` (`:252-256`) — nothing
  persists. §15(b)'s correction of the spec's `NeedsAttestation` prediction is exactly the binary's
  behavior, and the "separate, never-accepted disposal" prerequisite follows from the branch order.
- **(c) TRUE.** No CLI path constructs a `LedgerState` except `session.project()` over imported/reconciled
  events (grep of `btctax-cli/src`: resolve.rs / session.rs / inspect.rs only — no injection surface).
  The AMT-screen sizing matches r1's V-1 derivation (≈$17k as-built headroom; kitchen-sink margin
  $1.7–3.3k), now also quantified in the `examples.rs:214-221` corpus comment (the r1 non-gating
  suggestion, adopted). The synthetic-ledger figures (≈$3,438 mining / ≈$1,630 LT gain) re-derive exactly.
  The §4.2 caveat is correctly characterized as false-as-written and dropped; §6.1's source attributions
  are amended coherently; the `.0` oracle-equality guarantee is intact (test green).
- **(d) TRUE.** SPEC §5 J3 (`:247`) states the `match-self-transfers` / `classify-inbound-self-transfer`
  either-or; the delivered verb is within it; UX-P1-8 filed for the undemonstrated matched-pair workflow.

### M-1 — RESOLVED (guard is meaningful and loud)
`capture()` now `env_remove("BTCTAX_NOW")` before the conditional set (`examples.rs:84-87`). Verified the
guard is not vacuous: `resolve_now()` runs at `main.rs:92` **before** command dispatch and prints the
banner unconditionally on stderr whenever `BTCTAX_NOW` is set (confirmed against the live binary), and the
golden has three **unpinned** `show_stderr: true` steps (J1/J2/J6 exports) whose labelled blocks would
absorb an ambient-leaked banner — so without the `env_remove`, both the strengthened hermeticity test
(`examples.rs:615` sets `BTCTAX_NOW=2099-01-01…`) and `examples_golden_matches_committed` red loudly.

### M-2 — RESOLVED (filed, not fixed — correct: product serde/JSON is fence-barred)
UX-P1-5 filed with severity + owner (pre-v0.7.0 wording/UX cleanup). The `[2012, 106]` tuple still ships
verbatim in the golden and fixture, as it must under the fence. (Nit N-B below: the entry's golden
citation is ~4 lines stale post-regen.)

### M-3 — RESOLVED, and the new prose is accurate in both directions
`btctax-core/src/forms.rs:426`: `needs_review: if is_first { d.is_none_or(|d| !d.is_review_complete(section)) }
else { true }` — every non-first Form 8283 row is **unconditionally** flagged.
`is_review_complete(Section B)` (`donation.rs:68-77`) = appraiser_name + (tin|ptin) + appraisal_date +
**appraiser_qualifications** + donee_ein — J2 now passes all six, so the carrier row is clean and the
persisting warning (`examples.md:193`) is solely the second row's unconditional flag, exactly as the new
prose says ("the appraiser details ARE recorded; the flag is about the extra property row"). The
comparison claim ("A single-lot gift — see J6 — clears with no such note") is genuine, not coincidental:
the full-return path prints its **own** needs-review warning when flagged (`main.rs:676-681`), and J6's
stderr lacks it because its single-leg, complete-details donation genuinely has `needs_review == false`.
UX-P1-6 (the tool's misleading "run set-donation-details" advice for multi-lot gifts) filed, fence-barred,
owned. `--appraiser-qualifications` cannot clear the flag — confirmed.

### M-4 — RESOLVED
J4 runs `--kind staking` (`examples.rs:291-292`; `examples.md:274/278`); the prose says staking; every
tax figure is unchanged and re-derived exact (kind labels don't move SE math; `--business true` does).

### M-5 — RESOLVED (the coupling now points the right way)
Fixture moved to `crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml`;
`fullreturn_oracle.rs:20` includes it **locally**; the emitter (`:66-69`) writes the local path (and the
tree is clean, so it stayed idempotent); xtask — `publish = false` — now holds the cross-crate
`include_str!` (`examples.rs:239-240`). `btctax-cli/Cargo.toml` has no `include`/`exclude`, so
`cargo package` ships `tests/` + `tests/fixtures/` together: the published tarball's test is
self-contained. Oracle test green.

### M-6 — RESOLVED
The plan-conformance drift record (gate tests as `#[cfg(test)]` units; embedded CRLF consts vs committed
CSVs with the `.gitattributes` rationale; transitively-covered Task 1.2 import test; `tempfile` as a
regular dep) is in `FOLLOWUPS.md` — a reviewed artifact, satisfying the "plan status block or fold note"
correction.

### N-1..N-4 — verified real, with one carried consequence
- **N-1 RESOLVED**: `EnvRestore` drop-guard (`examples.rs:577-592`) captures before mutating, restores on
  scope exit **and on panic inside `generate()`**, before the assert; lock discipline unchanged.
- **N-2 RESOLVED**: the `shell_quote` limitation comment (`examples.rs:106-108`) names exactly the unsafe
  set and the future obligation.
- **N-3 RESOLVED**: version scan is `[package]`-anchored and `=`-requiring (`examples.rs:50-65`); a
  reordered manifest now degrades to a loud panic, never a dependency's version.
- **N-4 PARTIAL**: both conventions are now declared, and the `[exit N]` clause is correct (`emit()`
  prints it only `if code != 0`, `examples.rs:135-137`) — but the stderr clause **overclaims** and is the
  NEW Important below.

---

## New findings

### NEW-1 (Important) — the added front-matter stderr sentence is false: pinned-step banners ARE dropped

`examples.md:15-18` (emitted by `front_matter()`, `examples.rs:164-167`) now ships:

> anything a command writes to **stderr** — advisories, the not-authorised notice, a pinned-step clock
> banner — is never dropped, but appears in a separately labelled `stderr:` block rather than inline
> with stdout.

**Reality:** four captured commands run with a pinned `BTCTAX_NOW` — J3's `classify-inbound-self-transfer`
(`examples.rs:358-365`, whose own comment says "banner → stderr, **not captured**") and J5's `config` /
`optimize run` / `optimize accept` (`examples.rs:321-332`) — and every one writes `warning: BTCTAX_NOW
override active — decision timestamps are simulated` to stderr (`main.rs:83`, via `resolve_now()` at
`main.rs:92`, **before** dispatch; verified against the live binary). `emit()` drops stderr whenever
`show_stderr` is false (`examples.rs:139`). The banner appears **nowhere** in the golden — the sentence's
own named example ("a pinned-step clock banner") is its counterexample. This also inverts the SPEC
contract it was folded to satisfy: §3.3 requires stderr be captured where pedagogically relevant "and
otherwise **declared out of the verbatim-stdout capture**", and §13(d) promises the banner is "**disclosed,
not silently dropped**" — the clause was the disclosure vehicle and instead **denies** the omission.
**Failure scenario:** a reader replaying J5's pinned steps sees a warning banner the doc's stated
convention says would have been shown, and is licensed to infer "no `stderr:` block ⇒ the command wrote
nothing to stderr" — false for four blocks. This is the same confident-false-narration class as I-1
(SPEC §13(b): the byte gate cannot catch it; review is the only defense), so it gates on the project's own
precedent. **Scope is exactly these four instances** — `export-snapshot` writes stderr only with
unresolved hard blockers (`main.rs:574-590`; J1 has none) and advisories print to stdout.
**Correction (docs-cycle inline, no fence issue):** reword the clause to the true convention — stderr is
shown, in labelled `stderr:` blocks, where a journey's stderr is pedagogically relevant; steps run under a
pinned `BTCTAX_NOW` additionally print a clock-simulation warning on stderr that the captures omit — then
regenerate the golden.

### New Nits (non-gating)

- **N-A** — §15(a) records J4's substance but never names the **year switch** (spec'd 2024
  "for kitchen-sink oracle-consistency", §5:254 — delivered 2025); the §5 table's year note stands
  formally uncorrected. Fold into §15(a) in one clause at the next spec touch.
- **N-B** — UX-P1-5's citation `examples.md:474-477` is ~4 lines stale after the fold's regen (now
  477-481). Citations decay; note for the owning phase's cycle-prep.
- **Observation (pre-existing, not fold-introduced, product source — not this gate's):**
  `tax_tables.rs:71-72`'s doc comment says "TY2024, TY2025, and TY2026 bundled" while the code also
  inserts 2017.

---

## Gate

**0 Critical / 1 Important (NEW-1) / 0 Minor / 2 Nit → P1 is NOT green.** Everything the fold claimed is
genuinely resolved — I-1, I-2(a)–(d), M-1..M-6, N-1..N-3 all verified against live source, the real
binary, and re-derived arithmetic; the suite is green (1944/1944 + oracle). The sole blocker is NEW-1: one
false sentence in the regenerated front matter (reword + regen), after which this re-reviewer expects
green on re-verification per STANDARD_WORKFLOW §2.
