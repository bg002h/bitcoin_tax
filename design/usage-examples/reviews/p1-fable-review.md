# P1 review — usage-examples CLI generator, corpora, journeys, golden (independent Fable review)

*Reviewer: Fable (independent; author = the build agent). Scope: `git diff ad2b9b3..HEAD` (all of Phase 1,
12 commits `f9f1c71..b9fb8aa`) + the committed artifacts it produced, reviewed against
`SPEC_usage_examples.md` (r1 GREEN) §§3.1/3.3/4.2/5/6.1/7 and `IMPLEMENTATION_PLAN_usage_examples.md`
Tasks 1.1–1.6. Date: 2026-07-18. Every claim below was verified against current source / by running the
actual binary, not against commit messages.*

## Verdict

The engineering core of P1 is sound and better than the plan in places: the generator is genuinely
deterministic and hermetic (verified: suite green 1944/1944 incl. the three gate tests; golden has zero
CR bytes, zero tempdir/HOME path leakage; every arithmetic figure I re-derived — J1's −350/−77, J2's
2×108,996.17 close, J4's 7,450.67 = 0.05×85,484.60 + 0.03×105,881.32, J5's 3,000/−660 FIFO-vs-HIFO
split, J6's 35M-sat conservation — reconciles exactly against the corpora and the bundled daily-close
CSV); the §3.1 fence held structurally (the diff touches **no** product source — only xtask, tests,
docs, Makefile, FOLLOWUPS); the J6 oracle test is non-tautological and its emitter is idempotent
(verified by running it: clean tree); the J6 packet block shows **all 14 census forms**; the groff render
is warning-free and glyph-complete (verified via `pdftotext`); and the surfaced product bugs (UX-P1-2
stale help, UX-P1-3 `--amount` footgun, UX-P1-4 empty header) were correctly FILED, not fixed. However
the gate is **not met**: I find **0 Critical / 2 Important**. (1) J6's prose contains a confident false
statement — "The outbound **1 BTC** is a §170(e) charitable donation" when the corpus donates **0.1
BTC** — the exact cardinal sin this project defines for itself. (2) P1 silently descoped several
spec-§5/§4.2 journey-content mandates (the J4 missing-FMV demonstration and `classify-inbound-income`
verb, J5's second attestation branch, J6's kitchen-sink-ledger substitution and its spec-mandated
oracle-deviation caveat) with no spec amendment and no FOLLOWUP — the deviations are individually
well-motivated by discovered reality, but the workflow requires them *recorded and reviewed*, not
absorbed into a continuity note. Both Importants are cheap to fold (a one-word prose fix + regen; a spec
r2 amendment ± one journey touch-up). Full findings below.

## Validation surface

- `make check`: **green** — 1944 passed / 0 failed / 6 skipped, 15.5s wall. The three P1 gate tests all
  ran and passed (`examples_golden_matches_committed` 3.1s, `generate_is_deterministic_and_captures_help`
  7.5s, `examples_generate_is_hermetic_across_home_and_price_cache` 7.5s).
- `cargo test -p btctax-cli --test fullreturn_oracle`: green (1 passed, 1 ignored).
- The ignored `emit_fullreturn_fixture` regeneration helper: run; `git status` clean afterward — the
  committed fixture is byte-idempotent under its own emitter.
- `make examples`: builds; manual re-run of the awk|groff pipeline produced **0 bytes of stderr** (no
  missing-glyph warnings); `pdftotext` confirms `§`, `→`, and the `⚠ → (!)` map render.

---

## Critical

*None found.*

## Important

### I-1 — J6 prose misstates the donation size by 10×: "The outbound 1 BTC" (it is 0.1 BTC)

`crates/xtask/src/examples.rs:448` emits, and `docs/examples/examples.md:399` ships:

> The outbound 1 BTC is a §170(e) charitable donation (⇒ Form 8283):

The J6 Coinbase corpus (`examples.rs:205-209`) donates `cb-donate … 0.10000000` BTC at `--amount
6000.00`, and the generator's own doc comment (`examples.rs:202-204`) says "a 2024 charitable Send of
0.1 BTC". The adjacent verbatim block even shows `removed 10000000` sats. **Failure scenario:** a reader
following J6 sees the doc assert a 1 BTC gift two lines above a command recording a $6,000 FMV — either
they conclude btctax valued 1 BTC at $6,000 (wildly wrong) or that the doc can't be trusted; both
directly defeat the project's stated goal ("teach real btctax workflows"), and this class of confident
false narration is precisely what SPEC §13(b) admits the whole-file gate can NOT catch — so review is
the only line of defense, and it must hold here. **Correction:** "The outbound 0.1 BTC …" in
`journey_j6`, then regenerate the golden (a docs-cycle inline fix; no fence issue).

### I-2 — Silent, unrecorded descopes of spec-§5/§4.2 journey-content mandates

The spec's §5 journey table and §4.2 corpora are the content contract ("Exact command scripts are P1
deliverables; the spec-level shape" — the *shape* includes each row's corpus and demonstrates-column).
Four deliveries deviate materially, and none of the deviations is recorded in the spec, the plan, or
FOLLOWUPS — only in `CONTINUITY_BUILD.md:59-68` (a scratch note that is not a reviewed artifact) and
commit messages. Individually most are well-motivated; collectively they are an unmet-case gap at a
0C/0I gate:

- **(a) J4 drops the missing-FMV demonstration and the `classify-inbound-income` verb.** SPEC §5 J4 =
  "Income w/ **missing FMV** + business income", corpus §4.2 C-income-csv = "a River (or Coinbase) CSV
  **producing missing-FMV income** through `import`", commands = "`reconcile classify-inbound-income
  <in> --kind staking --fmv …`", year = **2024** "for kitchen-sink oracle-consistency". As built
  (`examples.rs:236-268`), J4 uses on-dataset 2025 dates (FMV auto-resolves), demonstrates only
  `reclassify-income`, and reduces missing-FMV to one prose aside ("an off-dataset day would instead
  flag a *missing-FMV* blocker", `examples.md:242-243`). The discovered reality — the bundled dataset is
  dense through 2026-06, so an import-produced missing-FMV requires an unsupported year
  (`CONTINUITY_BUILD.md:61-63`) — is a legitimate ground for a spec amendment (it is SPEC §12 S4's exact
  shape: a demonstration closable only by an unsupported year ⇒ halt/reconsider, or here: record). But
  the *manual-FMV remedy path* was demonstrable regardless: `classify-inbound-income <ref> --kind
  staking --fmv …` against an unclassified Receive (J3's corpus shape) needs no missing-FMV auto-flag.
  As delivered, the user-facing verb for pricing an inbound income event appears **nowhere** in the doc.
- **(b) J5 demonstrates only one attestation branch.** SPEC §5 J5's demonstrates-column: "**both
  attestation branches**… (a second run with BTCTAX_NOW > sale ⇒ NeedsAttestation)". As built
  (`examples.rs:272-310`), only the Contemporaneous branch is shown; the made-after-sale arm exists
  solely as prose. I verified empirically (real binary, J5 corpus, `BTCTAX_NOW=2026-01-01`) that the
  spec's own expectation was wrong in detail — a postdated `optimize accept` prints `skipped … already
  executed — re-run \`optimize accept --disposal <ref> --attest "<genuine contemporaneous ID>"\`` rather
  than persisting NeedsAttestation — so the as-shipped J5 prose ("an identification made after the sale
  would instead require an attestation", `examples.md:343-345`) is **true**, and the descope has a
  factual basis. But that basis corrects the spec, so the spec must be amended (or the skip/`--attest`
  exchange shown — arguably the more instructive capture); silence leaves a green-stamped spec the
  binary contradicts.
- **(c) J6's crypto side is not the kitchen-sink ledger, and the spec-mandated caveat is absent.** SPEC
  §4.2 C-fullreturn / the §6.1 table source `schedule_d/f8949` and `f1040sc/schedule_se/f8995` from
  "**kitchen_sink** Sell / Sch C business", i.e. `kitchen_sink_household().1` (1 BTC mining @ $20,000 +
  a $20,000-gain LT sale, `testonly.rs:264-301`), and §4.2 mandates: "**Caveat (stated in the doc):**
  this composite deviates from the pure oracle-validated `kitchen_sink_household` vector by exactly the
  added donation delta — the non-donation figures remain the oracle vector." As built, J6 rebuilds the
  crypto side from new, much smaller synthetic corpora (mining $3,437.95; LT gain $1,630 —
  `examples.rs:195-214`), so the composite deviates by far more than the donation delta, and **no**
  caveat appears in the doc. The root cause is real — there is no CLI path to inject a `LedgerState`
  value; a journey can only build a ledger through `import`+`reconcile`, and a kitchen-sink-faithful
  ledger would also have sat ~$1.7–3.3k from the AMT screen (see V-1) — and the author's substitution is
  defensible, *and* the as-spec'd caveat sentence would have been false as written. But the resolution
  (drop the claim and the caveat, shrink the crypto) rewrites §4.2/§6.1's factual frame and must be a
  reviewed spec amendment, not an implicit one. (The oracle-equality test still pins the `.0` half —
  that guarantee survives intact.)
- **(d) J3's corpus is a single-exchange Receive, not the spec'd "two-exchange CSV pair"** (§4.2
  C-self-transfer; `examples.rs:177-180`). The demonstrated verb is within the spec's either-or
  (`classify-inbound-self-transfer` / `match-self-transfers`), so this is the mildest instance — but
  `match-self-transfers` (the matched-pair workflow the corpus shape was designed to enable) is
  consequently undemonstrated anywhere.

**Correction:** one spec r2 amendment recording (a)–(d) with the discovered-reality rationales (dataset
density; the skip/`--attest` behavior; LedgerState non-injectability + AMT sizing), re-reviewed per §2 —
plus, at the author's option, the cheap demonstrations that remain available: the J5 postdated-skip
exchange and/or a `classify-inbound-income --fmv` step. What is *not* acceptable is carrying the green
spec unamended while the shipped doc contradicts its §4.2/§5/§6.1 content tables.

## Minor

### M-1 — Ambient `BTCTAX_NOW` leaks into unpinned journey steps (hermeticity gap, loud failure)

`capture()` (`examples.rs:71-90`) overrides `BTCTAX_PASSPHRASE`, `BTCTAX_PRICE_CACHE`, `HOME`, `TZ`,
`LC_ALL`, `LANG`, and sets `BTCTAX_NOW` only when `cmd.now` is `Some` — it never **clears** an ambient
`BTCTAX_NOW` for the (majority) unpinned steps. These are the only three env vars the binary reads
(`main.rs:51,71`, `price_cache.rs:20`), and this is the one uncovered. A developer who exports
`BTCTAX_NOW` in their shell — plausible, since this very project teaches the variable — gets a false-RED
`examples_golden_matches_committed` (the R-P0.4 stderr banner enters J1's/J2's/J6's `show_stderr`
blocks) with a confusing diff. Fails loud, never silent, hence Minor not Important; but SPEC §7's "pure
function of (repo tree, binary, synthetic inputs)" is strictly not yet true. Fix: `c.env_remove("BTCTAX_NOW")`
before the conditional set. The same reasoning applies to the hermeticity test, which perturbs `HOME` and
`BTCTAX_PRICE_CACHE` but not `BTCTAX_NOW`.

### M-2 — A surfaced UX wart in the shipped golden was not filed: `income show` renders a DOB as `[2012, 106]`

`docs/examples/examples.md:469-473` ships, verbatim, `"date_of_birth": [2012, 106]` — the raw serde
`(year, ordinal-day)` tuple of `time::Date` — in user-facing JSON (likewise `date_of_birth = [2012,
106]` in the committed TOML fixture a user is invited to imitate). A filer cannot read "day 106 of 2012"
as April 15, 2012. This is the same class as UX-P1-4 (a presentation wart captured verbatim in the
golden) and the bug-hunt purpose is co-equal by SPEC §0 — it should have been FILED when the golden
surfaced it. File as UX-P1-5 (Minor, fence-barred, owner: the pre-v0.7.0 wording/UX cleanup or later),
severity+owner per the standing rule.

### M-3 — J2 ends on an unexplained "needs REVIEW" warning (an inline-fixable docs-cycle artifact left unfixed)

J2's `set-donation-details` (`examples.rs:361-367`) omits `--appraiser-qualifications`, so the export's
stderr closes with "⚠ at least one donation needs REVIEW — its appraiser/donee declaration is
incomplete. Run `btctax reconcile set-donation-details …` to complete it" (`examples.md:184`,
`main.rs:767-771` `form_8283_needs_review`) — immediately after prose that says the details were
recorded. J6 passes `--appraiser-qualifications` and gets no warning, confirming the trigger. A reader
is left believing they completed the flow and being told they didn't, with no explanation. Unlike
UX-P1-2/-3/-4 this needs **no** product change: either add the flag (as J6 does) or add one prose
sentence framing the warning as deliberate. This is the inverse fence error: a docs-cycle-fixable
artifact left as-is. Fix in the fold + regen.

### M-4 — J4's story says staking, the command re-kinds it as mining

J4's prose: "Erin receives **staking** rewards on River" (`examples.md:241`); the commands then run
`reclassify-income … --kind mining` (`examples.rs:265-266`). `--kind` accepts
`mining|staking|interest|airdrop|reward` and is **optional** ("Omit to keep the original kind — only
flip `business`", `cli.rs:627-630`), so the journey either should omit `--kind` or use `--kind staking`;
as shipped it teaches silently relabeling staking income as mining for no stated reason. Fix in the fold
(script change + regen).

### M-5 — The published `btctax-cli` crate will carry a test with a dead cross-crate include path

`fullreturn_oracle.rs:22-23` does `include_str!("../../xtask/tests/fixtures/examples/…")`.
`btctax-cli/Cargo.toml` has no `include`/`exclude`, so `cargo package` ships `tests/fullreturn_oracle.rs`
in the `.crate` while the fixture (in the unpublished `xtask`) is not shipped. **Publish is unaffected**
— `cargo publish`'s verify step compiles only lib/bin targets, so the v0.7.0 release step is safe — but
anyone running `cargo test` against the published tarball gets a compile error. With no users this is
cosmetic; note it for the release ritual (options: `exclude = ["tests/fullreturn_oracle.rs"]`, or move
the fixture into `btctax-cli/tests/fixtures/` and re-point xtask's `include_str!` the other way — the
unpublished crate should be the one holding the cross-crate reference). In-tree the path is robust: a
move breaks the build loudly.

### M-6 — Plan-conformance drift, mostly justified but unrecorded in the plan

(a) The gate tests live as `#[cfg(test)]` unit tests inside `crates/xtask/src/examples.rs:494-576`, not
the plan's `crates/xtask/tests/examples_golden.rs` — functionally equivalent (verified they run under
`make check`). (b) The §4.2 CSV corpora are embedded CRLF consts, not the plan Task 1.2 committed
`.csv` files — justified by the `.gitattributes` `* text=auto eol=lf` discovery (a committed CSV would
be LF-normalized and break the Coinbase parser), recorded only in a commit message + continuity note.
(c) Task 1.2's "cargo test asserting each [corpus] imports without a hard blocker" does not exist as a
dedicated test — coverage is transitively via the golden gate (acceptable, but it was a named plan
step). (d) `tempfile` landed as a regular dependency, not the planned dev-dependency, because the
non-test `run()` path needs it — correct call (xtask is `publish = false`), unrecorded. Record these in
the plan's status block or the P1 fold note; no code change required.

## Nit

- **N-1** — `examples_generate_is_hermetic_across_home_and_price_cache` restores env before the assert
  but not on a panic *inside* `generate()` (`examples.rs:556-569`), and a poisoned `BUILD_ENV_LOCK`
  cascades `.unwrap()` panics across the sibling tests. Harmless under nextest (process-per-test, the
  project's configured runner); a scope-guard restore would make it airtight under plain threaded
  `cargo test`. Also, unguarded sibling tests in the same binary (the docs tests) could in principle
  observe the mutated `HOME` under threaded `cargo test` — today none of them care.
- **N-2** — `shell_quote` (`examples.rs:94-104`) double-quotes with only `"` escaped; an argument
  containing `$`, `` ` ``, `\`, or `!` would display as a non-copy-pasteable command. No current
  argument does; a comment naming the limitation would keep a future journey honest.
- **N-3** — `btctax_cli_version()` (`examples.rs:47-59`) takes the first `version`-prefixed line of the
  manifest; correct today ([package] version is line 3) but a re-ordered manifest could match a
  dependency's `version = "…"` first. A `toml`-parse or an anchored `^version` match is sturdier.
- **N-4** — SPEC §3.3/§13(d) promise stderr is "declared out of the verbatim-stdout capture — never
  silently dropped"; the front matter (`examples.md:10-13`) declares the pinned env but not the
  convention that stderr (including the R-P0.4 banner on pinned steps) appears only in labelled
  `stderr:` blocks. One clause in `front_matter()` closes it. Same for the implicit
  "`[exit N]` shown only when non-zero" convention.
- **N-5** — Markdown inline markup passes through the awk wrapper literally: `**hard blocker**` keeps
  its asterisks and backticks become typographic quotes in the PDF (`man-wrap.awk` body rule). Cosmetic
  only — the PDF is correctly not byte-gated (SPEC §13(a)) and the render is warning-free.
- **N-6** — `make check` grew ~6s → ~15.5s wall (the three gate tests build the binary and run up to
  five `generate()` passes). Acceptable, but the "~6s" figure in the plan's Global Constraints and the
  fast-validation memory note is now stale; update at the next touch.

---

## Verified sound (the six scrutiny claims, answered)

- **V-1 AMT-screen sizing — sound, and not a silent trap.** The screening worksheet
  (`btctax-core/src/tax/amt.rs:46-90`) computes line 5 = AGI − QBI − state refund and deliberately adds
  back **every** itemized deduction; so an extra charitable dollar lowers regular tax (line 13) at the
  ~24% MFJ marginal rate while leaving line 11 fixed — a large donation genuinely walks the return into
  `AmtScreenTriggered` (refusal), exactly as the author's comment (`examples.rs:195-198`) reasons. My
  independent estimate for the as-built J6: line 12 ≈ 26% × (≈299.8k − 133.3k) ≈ $43.3k vs regular tax
  ≈ $47.4k — a ≈$4.1k tax margin ≈ **$17k of additional deduction headroom** (the $6,000 donation
  consumed ≈$1.4k of it). A kitchen-sink-faithful ledger would have sat thinner (my rough figure
  $1.7–3.3k; the author's "~$3.3k" is in the right band). TY2024 pinning does substantially immunize:
  the 2024 `AmtParams` are historical constants, the fixture/corpora are frozen, and any engine change
  that moved the boundary reds `examples_golden_matches_committed` loudly (a refusal in the regen is
  maximally visible, and the P2 census would also red). There is no silent-flip path; sizing under a
  screen boundary is legitimate here **because** every input is frozen and every failure mode is loud.
  Non-gating suggestion: quantify the ≈$17k headroom in the `examples.rs` comment so a future
  corpus-editor knows the budget.
- **V-2 The cross-crate include — sound in-tree, publish-safe, one packaging wart (M-5).** The direction
  of coupling is right for the build (the golden generator and the oracle test share one set of bytes at
  compile time; a path break is a loud compile error), `xtask` is `publish = false`, and `cargo
  publish`'s verification does not compile test targets. The only residue is the published-tarball
  test-compile wart filed as M-5.
- **V-3 `income import` accepted the generated TOML.** Golden `examples.md:433-434` shows `Imported
  full-return inputs for tax year 2024.` — the real binary, real parse; `parse_return_inputs_toml`
  (`cmd/tax.rs:107-120`) rejects unknown keys via `serde_ignored` over the Value tree, so acceptance
  proves zero unknown keys. The doc's "Unknown keys are rejected, never silently dropped" and "every SSN
  and IP-PIN redacted" claims are both true against source (`mask_pii`, `cmd/tax.rs:149-162`, masks
  SSNs to last-4 and `ip_pin → "***"`).
- **V-4 BUILD_ENV_LOCK — correct where it matters.** Under nextest (the configured runner) each test is
  its own process: mutations are process-local, the lock uncontended, and a panic leaks nothing. Under
  threaded `cargo test` the lock serializes the three guarded tests; `built_btctax()` runs **before**
  the `HOME` mutation in the hermetic test (the commented hazard — cargo reading `$HOME/.cargo` — is
  real and correctly dodged), and `generate()`'s children pin `HOME` per-spawn so the process-global
  mutation cannot reach them. Residual panic-restore/poisoning/sibling exposure is N-1.
- **V-5 groff PDF — robust for this golden.** Verified empirically: zero stderr from
  `awk | groff -k -man -T pdf`; `pdftotext` shows `§`, `→`, and `(!)`-mapped warnings all present;
  leading-dot lines are `\&`-protected and backslashes `\e`-escaped in both prose and `.nf` blocks; the
  front-matter and comment blocks are stripped. Not byte-gating the PDF is correct and is the spec's own
  declared gap (§13(a)); the `%PDF` magic check matches the `write_pdfs` precedent. Cosmetic markup
  passthrough is N-5.
- **V-6 J6 narrative accuracy — one false sentence (I-1); all other claims check out.** Verified true:
  the 14-form packet listing and "IRS Attachment-Sequence stapling order" (00→155, all 14 census keys
  present: f1040, s1, s2, s3, sa, sb, sc, schedule_d, f8949, schedule_se, f8995, f8959, f8960, f8283);
  the §170(e) LT→FMV / ST→min(FMV,basis) rule as narrated in J2 (deduction $110,996.17 = 108,996.17 FMV
  + 2,000 basis — exact); the unknown-keys and PII-redaction claims (V-3); the J5 attestation prose
  (verified against the live binary, V- in I-2(b)); conservation arithmetic (`in 35000000 = disposed
  5000000 + removed 10000000 + held 20000000` reconciles with 0.3 buy + 0.05 River income − 0.05 sale −
  0.1 donation). The committed synthetic SSNs (123-45-6789 / 987-65-4321 / 111-22-3333) are the
  canonical dummy values already committed in `testonly.rs` on main — the fixture adds no new exposure,
  and the golden additionally shows them only masked.

**Fence audit:** the P1 diff contains no compute/fill-engine or product-message edit (file list:
xtask, btctax-cli *tests*, fixtures, docs, Makefile, FOLLOWUPS, design notes — nothing under any
product crate's `src/`). Surfaced bugs were filed with severity + owner (UX-P1-2/-3/-4), and the UX-P1
reconciliation correctly re-owns the fence-barred fixes to a named pre-v0.7.0 phase. Nothing was filed
that P1 could have fixed inline — the one inversion found is M-3 (an inline-fixable artifact neither
fixed nor filed).

## Gate

**0 Critical / 2 Important / 6 Minor / 6 Nit → P1 is NOT green.** Fold I-1 (one-word prose fix + golden
regen) and I-2 (spec r2 amendment recording the descopes, ± the optional cheap demonstrations), address
or file the Minors per ownership, then re-review per STANDARD_WORKFLOW §2.
