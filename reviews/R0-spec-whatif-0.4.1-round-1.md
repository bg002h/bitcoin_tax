# R0 — SPEC review, round 1 — `whatif 0.4.1` cleanup (harvest-target parser dedup + `--sell` BTC input)

**Artifact:** `design/SPEC_whatif_0_4_1_cleanup.md` (DRAFT). **Reviewer:** independent architect (Opus).
**Baseline verified against:** branch `feat/whatif-0.4.1` @ `ee07028`; `main` @ `2e89911`. Read-only; no code changed.
**Companion docs read in full:** `design/BRAINSTORM_whatif_0_4_1_cleanup.md`, `cycle-prep-recon-whatif-sell-btc-input-tui-parser-dedup.md`.
**Bar:** 0 Critical / 0 Important.

## Verdict: 1 Critical / 4 Important / 3 Minor / 2 Nit — **BLOCKED (not GREEN).** Fold before Plan.

The refactor's *shape* is right (a core `FromStr` + core parse helper is the correct seam; KAT-E10 stays green — see §3).
But the flagship guarantee the task told me to scrutinize hardest — **P1 "exact parity" / "identical rejections"** — is
**violated by the spec's own text and its parity KAT**: today's parser does **not** reject a negative `gain=`/`tax=`
amount (the engine does), yet the spec mandates the new `FromStr` reject it. That is a silent, untested behavior change
(C1). Two more parity/non-breaking traps (I1 TUI regression, I3 negative `--sell`) and the SemVer classification (I4)
also block.

---

## Evidence base (ground truth, `file:line`)

- `parse_harvest_target` — `crates/btctax-cli/src/cmd/whatif.rs:111-127`; its amount arm `parse_target_amount` —
  `cmd/whatif.rs:128-135`. **No `>= 0` / sign check anywhere**; it is `Usd::from_str(cleaned)` where
  `cleaned = v.trim().replace(['$', ','], "")`.
- `Usd = Decimal` — `crates/btctax-core/src/conventions.rs:8`; `Sat = i64` — `conventions.rs:6`. So `Usd::from_str("-1")`
  is `Decimal::from_str("-1")` = **`Ok(-1)`** (rust_decimal accepts a leading `-`). Negatives PARSE.
- Negatives are rejected **downstream in the engine**, not the parser: `HarvestTarget` doc — `crates/btctax-core/src/whatif.rs:367-369`
  ("MUST be ≥ 0 … rejected as `InvalidTarget`"); the golden test constructs `HarvestTarget::Gain(dec!(-1))` **directly**
  and asserts `WhatIfError::InvalidTarget` — `crates/btctax-core/tests/harvest.rs:1143-1165`. The parser is never the gate.
- `HarvestTarget` enum — `whatif.rs:371-383`; **no `FromStr` today** (grep clean) — additive, non-conflicting. Good.
- TUI dup parser — `crates/btctax-tui/src/whatif_panel.rs:437-459` (`parse_harvest_target` + `parse_usd_amount`); identical
  accept/reject set to the CLI's, only the error *string* differs.
- TUI BTC parse to lift — `whatif_panel.rs:416-433` (`pub fn parse_btc_to_sat(s) -> Result<i64, String>`). **It treats a
  bare integer as BTC**: KAT `parse_btc_to_sat("1") == Ok(100_000_000)` and `("0.05") == Ok(5_000_000)` —
  `whatif_panel.rs:571-573`. Its field is BTC-labeled — `whatif_panel.rs:59` + render `"Sell amount (BTC)"` at `:339`.
  It strips `['_', ',']` (NOT `$`) — `whatif_panel.rs:417`; rejects `< 0` at `:422`; rejects over-precision via
  `sats.fract() != 0` at `:426`.
- CLI `--sell` is **already `String` in clap** — `crates/btctax-cli/src/cli.rs:337` (`WhatIf::Sell`) — and is parsed to
  `i64` **manually** in `crates/btctax-cli/src/main.rs:224` (`sell.trim().parse::<i64>()`). A **second, identical** manual
  sat parse lives at `main.rs:171` for `Optimize::Consult` (clap `sell: String` at `cli.rs:303`).
- Core `whatif::sell` on a **negative** `sell_sat`: the pool gate `pool_sum < req.sell_sat` is FALSE for a negative
  (`whatif.rs:232`), and `method_selection` picks nothing (`need <= 0` at `whatif.rs:185-189`) → a negative `--sell`
  today **computes a degenerate report, it does not error.**
- KAT-E10 source gate — `crates/btctax-tui/src/export.rs:736-743` (`everywhere_tokens` includes `"cmd::"`), scan strips
  `//` comments and skips test regions (`export.rs:778-811`). Panel already imports `btctax_core::whatif` (`whatif_panel.rs:15`)
  + `use std::str::FromStr` (`:20`) and uses `btctax_cli::render` (not `cmd::`) at `:281,:310`.
- Lockstep surfaces: man page `docs/man/btctax-what-if-sell.1:13` ("… amount in satoshis (required)") is generated from the
  clap doc-comment `cli.rs:335`; parent page `docs/man/btctax-what-if.1` also exists. README `--sell` sat examples —
  `README.md:233,:252`. **No schema-mirror code** (only prose hits in `.md` docs) — recon claim confirmed.

---

## CRITICAL

### C1 — The `FromStr` "reject negatives" requirement CONTRADICTS current parser behavior (breaks the exact-parity guarantee)
**Where:** SPEC §P1 line 15 ("`X ≥ 0 required (reject negatives — preserve today's cmd/whatif.rs:110-135 behavior EXACTLY)`")
and the P1 KAT line 42-43 ("`every rejection (gain=-1, foo, empty) errors`") + the P1 gotcha line 64-65.
**Evidence:** `parse_target_amount` (`cmd/whatif.rs:128-135`) is `Usd::from_str(cleaned)` with **no sign check**;
`Usd = Decimal` (`conventions.rs:8`) parses `-1` → `Ok(-1)`. So **today `--target gain=-1` parses to
`HarvestTarget::Gain(-1)`** and the *engine* refuses it as `InvalidTarget` (proof: `harvest.rs:1143-1165` builds
`Gain(dec!(-1))` directly and gets `WhatIfError::InvalidTarget`; the parser is never asked to reject it).
**Why it's Critical:** the spec's headline promise is "**NO behavior/surface change (identical accepted strings +
identical rejections)**" (line 23) under a **PATCH**. But "reject negatives in `FromStr`" is the *opposite* of what the
current parser does — it MOVES the rejection from the engine (a `WhatIfError::InvalidTarget` refusal at compute) to the
parser (a `CliError::Usage` at parse), changing the error class, the message, and the code path for
`what-if harvest --target gain=-1`. **No CLI test covers that input** (grep: no `--target gain=-` / `bad --target` assertion
in `crates/btctax-cli/tests/whatif_harvest.rs`), so the divergence ships **silently** and the "matches prior parsers" KAT
is asserting a *false golden* (`gain=-1` does not error in the prior parser). This is exactly the "a dropped-detail changes
behavior" trap the task flagged.
**Fix:** the `FromStr` must mirror `parse_target_amount` **byte-for-byte**: `Usd::from_str` with **no sign check**.
`gain=-1` / `tax=-1` must **parse to `Gain(-1)`/`Tax(-1)`** (kept invalid downstream by the engine's `InvalidTarget`,
unchanged). Delete the "X ≥ 0 required (reject negatives)" clause and the `"gain must be ≥ 0"` `Display` string from the
spec. Rewrite the parity KAT to assert `"gain=-1".parse::<HarvestTarget>() == Ok(HarvestTarget::Gain(dec!(-1)))` (a true
golden vs the prior parser) and keep the *engine* `InvalidTarget` test (`harvest.rs:1143`) as the negative-rejection
coverage. If moving the ≥0 check into the parser is genuinely wanted, that is a deliberate **behavior change** and must be
specced/versioned as such — not smuggled in under "exact parity / PATCH."

---

## IMPORTANT

### I1 — P2 shared helper: rewiring the TUI amount field to the smart parser is a silent regression (bare `1` = 1 BTC today, would become 1 sat) + breaks an existing KAT
**Where:** SPEC §P2 lines 33-36 ("`Lift the TUI panel's existing BTC→sat parse … have BOTH the CLI --sell and the TUI
amount field call it`") and the KAT `parse_btc_or_sat_shared_by_cli_and_tui` (line 47).
**Evidence:** the TUI field is **BTC-only** — a bare integer is BTC: `parse_btc_to_sat("1") == Ok(100_000_000)`
(`whatif_panel.rs:572`), field labeled "Sell amount (BTC)" (`whatif_panel.rs:339`, `:59`). The CLI smart parser the spec
defines is **BTC-only-when-dotted** — a bare integer is **sat** (`sell_bare_integer_stays_sat`, line 45). These are two
**different input conventions.** If the TUI amount field is pointed at the *smart* `parse_btc_or_sat`, typing `1` silently
changes from **1 BTC (100 000 000 sat)** to **1 sat**, contradicting the field label and **breaking the currently-passing
KAT at `whatif_panel.rs:571-573`.** "Same helper, same result" is unachievable while both keep today's meanings.
**Why Important:** the spec's own two-function hint (`parse_btc_or_sat` **+** `parse_btc_amount`, line 34) points at the
right split, but the prose "have BOTH … call it" is ambiguous and, on the literal reading, is a user-facing TUI regression.
**Fix:** state it explicitly. Lift **only the BTC-decimal→sat conversion** as `parse_btc_amount(s) -> Result<Sat, _>`
(the `whatif_panel.rs:416-433` body). The **TUI amount field calls `parse_btc_amount`** (BTC-only — behavior unchanged;
its existing KAT stays green). The **CLI `--sell` calls `parse_btc_or_sat`**, a thin wrapper: `contains('.')` →
`parse_btc_amount`, else bare-integer sat. Rename the shared KAT to `parse_btc_amount_shared_by_cli_and_tui` and pin that
the TUI still reads a bare `1` as `100_000_000` (regression guard).

### I2 — SemVer: this is **MINOR (0.5.0), not PATCH (0.4.1)** — the "no new public surface" claim is false
**Where:** SPEC §"Scope / SemVer / lockstep" line 52-53 ("`PATCH → 0.4.1 (no new public surface …)`").
**Evidence:** the refactor **adds public API to the published `btctax-core` library**: `impl FromStr for HarvestTarget`
(HarvestTarget is `pub` — `whatif.rs:371`), `pub enum HarvestTargetParseError` (mandatorily public — it is the public
`FromStr::Err` associated type), `pub fn parse_btc_or_sat`, `pub fn parse_btc_amount`. All four are consumed from
**separate crates** (`btctax-cli`, `btctax-tui`) so none can be `pub(crate)`. Adding public items to a published crate is
the textbook definition of an **additive/MINOR** change under Cargo SemVer; the newly-accepted `--sell` decimal is likewise
an additive *feature*, not a fix. (Context: all crates are published + version in workspace lockstep — see
`memory/crate-publishing-state.md` and the `0.3.0→0.4.0` lockstep bump.)
**Fix:** correct the classification to **MINOR → 0.5.0** (workspace-lockstep), or explicitly justify why the team treats
additive library API as PATCH. Delete "no new public surface" (there is new public surface). This changes the release/tag
step in the Plan.

### I3 — `--sell` "reject negatives" breaks the byte-identical guarantee for bare integers (today `--sell -5` computes, doesn't error)
**Where:** SPEC §P2 line 30 ("`Reject negatives + non-numeric`") vs line 30/37-38 ("`--sell 5000000 UNCHANGED`",
"`existing --sell <integer> callers are byte-identical`") and KAT `sell_negative_and_nonnumeric_rejected` (line 46).
**Evidence:** today `--sell -5` → `main.rs:224` `"-5".trim().parse::<i64>()` = `Ok(-5)` → the core pool gate is FALSE for a
negative (`whatif.rs:232`) and `method_selection` picks nothing (`need <= 0`, `whatif.rs:185-189`) → a **degenerate report
computes; no error.** A parser that "rejects negatives" turns that into a parse-time `Usage` error — **not byte-identical**
for a bare integer input. (`+5`, ` 5 `, etc. stay identical if the bare path is `trim().parse::<i64>()`.)
**Why Important:** the task told me to scrutinize P2 non-breaking hardest; the spec asserts byte-identical for *all*
integer callers, which is false for negatives, and the `sell_negative_and_nonnumeric_rejected` KAT would *pin* the
divergence.
**Fix:** scope the negative check to the **BTC path only** (matching the TUI, `whatif_panel.rs:422`). The bare-integer sat
path must remain exactly `trim().parse::<i64>()` (accepts `-5`/`+5` as today). Split the KAT: `sell_btc_negative_rejected`
(`--sell -0.5` → error) and `sell_bare_integer_negative_unchanged` (`--sell -5` behaves as today). If you'd rather reject
negative sat too, say so — it's a (small) behavior change, not "byte-identical."

### I4 — `HarvestTargetParseError` `Display` is under-specified (misses the bad-amount case) and the "cli error messages are stable" claim is inaccurate
**Where:** SPEC §P1 lines 16-21 (Err type `Display` example only covers the unknown-form case; the mapping "`keep the exact
CliError variant … so cli error messages are stable`").
**Evidence:** the current parser has **two** distinct error messages: the unknown-form one — `"bad --target {s:?}: expected
zero-ltcg | fifteen-ltcg | gain=$X | tax=$X"` (`cmd/whatif.rs:124-126`) — and the amount-parse one — `"bad --target amount
{v:?}: expected a USD number: {e}"` (`cmd/whatif.rs:130-134`). The spec's proposed `Display` (`"unrecognized target '<s>'
(expected …)"`) covers only the first and is *different wording* from today. So (a) the `Display` design is incomplete (the
`Gain`/`Tax` bad-USD error must be representable too), and (b) mapping `CliError::Usage(e.to_string())` keeps the *variant*
(`Usage` — `cmd/whatif.rs:124`) and exit code stable but the **message text changes** — "messages stable" is false as
written.
**Mitigant / why not Critical:** **no test pins those strings** (grep across `crates/btctax-cli/tests/` and snapshots is
clean), so nothing breaks; error text is not a SemVer surface. This is a spec-consistency/completeness gap, not a runtime
bug.
**Fix:** either (preferred) drop the "messages stable" claim and accept the new (arguably clearer) `Display`, *and* spell
out that `HarvestTargetParseError` must carry **both** variants (unknown-form + bad-amount, the latter wrapping the
`Decimal` parse error) so the CLI/TUI can render both; or, if byte-identical messages are truly required, define the
`Display` to reproduce both current strings exactly.

---

## MINOR

### M1 — "today a bare `i64` (cli.rs:334)" is wrong; the real change seam is `main.rs:224`, and a **third** sat parser (`optimize consult --sell`) is left inconsistent
`--sell` is **already `String` in clap** (`cli.rs:337`); the `i64` conversion is a manual `trim().parse::<i64>()` at
`main.rs:224` — so "make `--sell` a STRING arg with a smart parser (not a bare i64)" mis-frames the change (no clap type
change is needed; either add a clap `value_parser` or replace the `main.rs:224` body). Separately, `Optimize::Consult`
carries an **identical** `--sell` sat parse at `main.rs:171` (clap `sell: String` at `cli.rs:303`) that the spec never
mentions: after this cycle `what-if sell --sell 0.05` will work but `optimize consult --sell 0.05` will still error — a
user-facing inconsistency. **Fix:** correct the "bare i64 / cli.rs:334" framing to "String at cli.rs:337 → manual i64 at
main.rs:224"; and either bring `optimize consult --sell` along (trivial — it can call the same `parse_btc_or_sat`) or add
one line explicitly declaring it out of scope.

### M2 — P1 parity is under-documented: the current parser lower-cases the WHOLE string and trims twice; the golden KAT must pin these
The spec lists "the three aliases + `$`/comma-optional" but omits parity details the `FromStr` must preserve:
(a) `s.trim().to_ascii_lowercase()` runs on the **whole** string, so the **prefixes are case-insensitive** —
`GAIN=$5`, `TAX=$0` are accepted today (`cmd/whatif.rs:112,118,121`); (b) it **trims twice** (outer whole-string + inner
`v.trim()` at `:129`), so `  gain= 5 ` works; (c) it strips `$` and `,` but **not `_`**, and `Decimal` rejects `_`, so
`gain=1_000` **errors** today. **Fix:** enumerate these in §P1 and add them to `harvest_target_fromstr_matches_prior_parsers`
(`GAIN=$5` accepted; `gain= 5 ` accepted; `gain=1_000` rejected) so the golden truly locks parity.

### M3 — the lifted BTC parse's separator handling must be pinned (the TUI strips `_`/`,`, not `$`)
`parse_btc_to_sat` strips `['_', ',']` (`whatif_panel.rs:417`) — so it accepts `1_000` / `1,000` as BTC — while the harvest
USD parse strips `['$', ',']`. When `parse_btc_amount` is lifted to core, the spec must state **which separators it strips**
(to stay byte-identical with the TUI's existing behavior) and that the CLI `--sell` BTC path inherits the same, so
`--sell 0.05` and the TUI `0.05` are provably one code path. Add a KAT (`parse_btc_amount("1,000") == Ok(...)` if that
parity is intended, or an explicit decision to drop separator-stripping). **Fix:** one sentence + one KAT.

---

## NIT

### N1 — use `Sat` (not `i64`) and confirm no type mismatch
`Sat = i64` (`conventions.rs:6`), so there is **no** Sat/type mismatch (task item 5): the TUI's `parse_btc_to_sat` returns
`i64`, `SellRequest.sell_sat: Sat` (`whatif.rs:43`), and `cmd::whatif::sell(sell_sat: i64)` (`cmd/whatif.rs:68`) are all
the same underlying type. The spec's `parse_btc_or_sat(s) -> Result<Sat, …>` is fine; just use the `Sat` alias
consistently (and have the TUI signature adopt `Sat` when it moves) for readability.

### N2 — name the doc-source and both man pages in the lockstep step
The man page is **generated** from the clap doc-comment `cli.rs:335`, not hand-edited — the lockstep step should say
"edit the `cli.rs` `--sell` doc-comment, then `cargo run -p xtask -- docs`" and note that **two** pages regenerate
(`docs/man/btctax-what-if-sell.1` **and** the parent `docs/man/btctax-what-if.1`). The README `--sell` sat examples
(`README.md:233,:252`) are a good place to add a `--sell 0.05` BTC example.

---

## Cross-check summary (per task item)

1. **P1 exact parity** — **FAILS** on negatives (C1); under-documented on case/trim/`_` (M2). Aliases, `$`/comma-optional,
   the double-trim, and the `Usd::from_str` amount parse are otherwise faithfully lift-able.
2. **P1 error-type + cli stability** — the core `Display`-only `HarvestTargetParseError` (not `CliError`) **is** the right
   seam (keeps core cli-free). Variant + exit stay stable (`Usage`, `cmd/whatif.rs:124`); message **text** changes and the
   `Display` is under-specified (I4). Panel deps `btctax_core`, not `cmd::` — fine.
3. **P1 KAT-E10** — **no risk.** `everywhere_tokens` forbids the `"cmd::"` token (`export.rs:739`); `HarvestTarget::from_str`
   / `s.parse::<HarvestTarget>()` add no forbidden token; panel already imports `btctax_core::whatif` (`whatif_panel.rs:15`)
   and `FromStr` (`:20`). Removing the local dup is clean. Calling `btctax_cli::cmd::…` **would** trip E10 — which is why
   the core seam is correct.
4. **P2 smart `--sell`** — implementable, but **not** "today a bare i64" (it's `String`+manual parse, M1); the exact
   `×1e8` via `Decimal` (`Usd`) avoids float error and the `sats.fract() != 0` over-precision reject is exact/no truncation
   (`whatif_panel.rs:426`); `5000000.0`→5M BTC failing at the pool check (not silently) is accurate (`whatif.rs:232`).
   **But** "reject negatives" breaks byte-identical for bare `-5` (I3).
5. **P2 shared helper** — the TUI parse to lift **exists** (`whatif_panel.rs:416`); `btctax-core::whatif` **is** the right
   home (cli + tui already dep core; no new edges); `Sat`==`i64` so no type mismatch (N1). **But** "have both call it" is a
   TUI regression on the literal reading (I1); separator handling must be pinned (M3).
6. **KATs / SemVer / lockstep / gaps** — KATs are **insufficient** (encode the C1/I3 false goldens; miss the M2 case/trim
   goldens, the I1 TUI-unchanged guard, and M1's `optimize consult`); SemVer is **MINOR, not PATCH** (I4); the lockstep is
   otherwise right (man regen via `xtask`, README; no schema-mirror — confirmed) but should name the doc-source + parent
   page (N2). New gaps surfaced: the third `--sell` parser (`optimize consult`, M1) and the `--sell` help/README example
   (N2). The harvest `--target` help string + its TUI selector are untouched by the `FromStr` move → stay working (good).

## Required before R0-GREEN
Fold **C1** (parity: `FromStr` must not reject negatives; fix the parity KAT to a true golden), **I1** (TUI calls the
BTC-only `parse_btc_amount`; CLI wraps it in `parse_btc_or_sat`; add the regression guard), **I2** (re-classify as
MINOR/0.5.0; drop "no new public surface"), **I3** (scope negative-reject to the BTC path; keep bare-int sat byte-identical),
**I4** (`Display` covers both error cases; reconcile the "messages stable" claim). Then re-review (round 2) — including the
Minors/Nits — before any implementation. **Not GREEN.**
