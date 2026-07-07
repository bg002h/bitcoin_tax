# R0 — SPEC review, round 2 (delta / fold-verification) — `whatif 0.5.0` cleanup (harvest-target parser dedup + `--sell` BTC input)

**Artifact:** `design/SPEC_whatif_0_4_1_cleanup.md`. **Reviewer:** independent architect (Opus). **Scope:** delta only —
verify the round-1 folds (C1, I1–I4, M1–M3) are captured correctly and introduced no new contradiction.
**Baseline verified against:** branch `feat/whatif-0.4.1` @ `2fee942`; `main` @ `2e89911`. Read-only; no code changed.
**Round-1 review folded:** `reviews/R0-spec-whatif-0.4.1-round-1.md` (1C/4I/3M/2N). **Bar:** 0 Critical / 0 Important.

## Verdict: 0 Critical / 1 Important / 2 Minor / 1 Nit — **BLOCKED (not GREEN).**

The substantive folds all landed correctly in the spec **body**: C1 (pure lexer, no negative reject), I1 (two helpers,
TUI stays BTC-only), I2 (MINOR/0.5.0), I3 (sat path byte-identical incl. `-5`), I4 (two error variants), M1/M2/M3
(citation fix, `optimize consult`, separator handling) — each re-verified against source below and each PASSES in the
body. **But the Plan section (§Plan, T2) was not swept during the fold** and still carries the *pre-fold* text: it names
the old single smart parser `parse_btc_or_sat`, restates the exact **I1 regression** ("the TUI amount field … [calls] the
smart parser"), and says "ship 0.4.1". That is one live Important inconsistency — the actionable TDD task list contradicts
the folded body on the flagship I1 point. A one-line rewrite of T2 clears it. Two Minors (stray `PATCH`/`0.4.1` residue at
§Header/§P1/§Plan; the `optimize consult` doc-comment lockstep) and one Nit round it out.

---

## Fold-by-fold verification (against current source)

### C1 — PURE LEXER, does NOT reject negatives — **PASS (body)**
Spec §P1 lines 15-21 + Gotchas 84-86 now mandate moving `parse_harvest_target`'s logic **byte-for-byte, adding NO new
checks**, and explicitly "**DO NOT reject negatives**": `gain=-1` → `Gain(-1)`, engine refuses as `InvalidTarget`. The
"must be ≥ 0" `Display` variant is deleted (line 26 "NO 'must be ≥ 0' variant").
- Source confirms parity: `parse_harvest_target` (`crates/btctax-cli/src/cmd/whatif.rs:111-135`) has **no sign check** —
  the amount arm is `parse_target_amount` = `Usd::from_str(v.trim().replace(['$', ','], ""))` (`:128-135`). `Usd = Decimal`
  (`crates/btctax-core/src/conventions.rs:8`), so `Usd::from_str("-1") = Ok(-1)`. `gain=-1` genuinely parses to `Gain(-1)`
  today.
- KAT `harvest_target_gain_negative_parses_not_rejected` (spec line 56-57) asserts `gain=-1` → `Gain(dec!(-1))`, NOT a
  parser error — a **true** golden vs the prior parser. The engine-side negative rejection is still covered by
  `harvest_invalid_negative_target` (`crates/btctax-core/tests/harvest.rs:1143-1164`, builds `Gain(dec!(-1))` directly,
  asserts `WhatIfError::InvalidTarget`). **Parity preserved.** No "reject negatives" residue for the target parser anywhere
  in the spec (line 18/41/85 "reject negative" all correctly scope to the *BTC* path).

### I1 — TWO helpers (`parse_btc_amount` BTC-only + `parse_sell_arg` smart) — **PASS in body / FAIL in Plan (see Important)**
Spec §P2 lines 37-44 + Gotchas 88-89 define both helpers and are explicit that the TUI amount field calls
`parse_btc_amount` **unchanged** and must **NOT** be pointed at the smart parser ("or `1`→1 sat silently breaks the TUI,
whatif_panel.rs:572"). KATs pin both: `parse_btc_amount_bare_one_is_one_btc` (`1`→100 000 000) and
`tui_amount_field_uses_parse_btc_amount` (line 59, 63).
- Source confirms the trap the fold guards: `parse_btc_to_sat` (`crates/btctax-tui/src/whatif_panel.rs:416-433`) treats a
  bare `1` as **1 BTC** — KAT `parse_btc_to_sat("1") == Ok(100_000_000)` (`whatif_panel.rs:572`). The body's split is
  coherent and both KATs pin the two conventions. **Body is correct.**
- **However** the Plan (T2, lines 80-81) still says "`parse_btc_or_sat` in core; the `--sell` smart parser (cli) + the TUI
  amount field **both call it**" — i.e. the TUI field calls the *smart* parser. That is the pre-fold I1 defect verbatim →
  **Important, below.**

### I2 — SemVer MINOR → 0.5.0 — **PASS (classification) / stray residue (see Minor 1)**
Spec header line 5 ("[R0-I2] SemVer corrected: MINOR → 0.5.0"), title ("0.5.0"), and §Scope lines 70-73 all correctly
classify as MINOR/0.5.0 with the rationale (new `pub` `FromStr` / `HarvestTargetParseError` / `parse_btc_amount` /
`parse_sell_arg` on published `btctax-core`; branch name acknowledged a misnomer). Classification is right. Stray old-tag
residue remains at three spots (Minor 1).

### I3 — sat path byte-identical incl. `--sell -5` → −5 sat — **PASS**
Spec §P2 lines 45-47 + Gotchas 90-91: the non-`.` sat path is byte-identical to today, `--sell -5` passes `-5` through as
−5 sat with **NO sat-side sign check**; only the `.`-BTC path rejects negatives. KAT
`sell_arg_sat_path_byte_identical_incl_negative` (`-5`→−5 sat, line 61).
- Source confirms today's behavior: `main.rs:224` `sell.trim().parse::<i64>()` gives `Ok(-5)`; the core pool gate
  `pool_sum < req.sell_sat` (`crates/btctax-core/src/whatif.rs:232`) is **false** for a negative, so it is not rejected
  there — a degenerate (non-error) path, matching round-1's finding. The KAT correctly pins the *parser* to
  `parse_sell_arg("-5") == Ok(-5)` (byte-identity to `i64::from_str`), which is the right guarantee. The else-branch is
  specified as "bare integer sat EXACTLY as today (`i64::from_str`, main.rs:224)" (line 44). **No sat-side sign check
  introduced.** (Nit: the ":232 degenerate report" citation is slightly loose — :232 is the pool gate a negative
  *bypasses*, not where the report is produced; the parity claim itself is correct.)

### I4 — two error variants (`UnrecognizedTarget` / `BadAmount`) — **PASS**
Spec §P1 lines 22-27: `pub enum HarvestTargetParseError` with `Display` covering **both** current failures —
`UnrecognizedTarget(String)` and `BadAmount(String)` — core type (not `CliError`), keeps the CLI variant stable, drops the
"messages stable" text-level claim ("cli error messages aren't test-pinned … keep the existing CliError variant").
- Source confirms the two failures exist: unknown-form `Err(CliError::Usage("bad --target {s:?}: expected …"))`
  (`cmd/whatif.rs:124-126`) and bad-amount `"bad --target amount {v:?}: expected a USD number: {e}"` (`:130-134`). The P1
  KAT exercises both (`foo`/empty → `UnrecognizedTarget`; `gain=abc` → `BadAmount`, line 58). **Both cases representable.**

### M1 (round-1) — `--sell` citation fix + third parser (`optimize consult`) — **PASS**
Spec §P2 line 36 ("`WhatIf::Sell.sell` is ALREADY a `String` (cli.rs:337), manually parsed to sat at main.rs:224 (NOT a
bare `i64`)") and line 49-50 / Gotcha 93 ("apply `parse_sell_arg` to `optimize consult --sell` too (main.rs:171 — the THIRD
identical sat parser)").
- Source confirms all three citations: `cli.rs:337` `sell: String`; `main.rs:224` and `main.rs:171` are the two identical
  `sell.trim().parse::<i64>()` sat parses (what-if sell / optimize consult). No "bare i64 clap arg" mis-framing residue.
- **Consult semantics check (per task probe):** applying `parse_sell_arg` to `optimize consult --sell` is safe — its
  non-`.` path is byte-identical `i64::from_str` (same as `main.rs:171` today, incl. `-5`), so all existing integer callers
  are unchanged; only dotted BTC input is new, and `sell_sat` flows into `cmd::optimize::consult(sell_sat, …)`
  (`main.rs:199-207`) exactly as before. No semantic conflict. (Doc-comment lockstep gap → Minor 2.)

### M2 (round-1) — case-insensitive whole-string lower / double-trim / `$`,`,` (not `_`) — **PASS (prose); KAT lags (Nit)**
Spec §P1 lines 16-18 enumerate: lowercase the WHOLE string (→ `GAIN=`/`TAX=` accepted), the trim/double-trim, strip `$`
and `,` (**NOT `_`**). KAT (line 54-55) pins case-insensitive `GAIN=` and `gain=$1,000`==`gain=1000`.
- Source confirms: `s.trim().to_ascii_lowercase()` on the whole string (`cmd/whatif.rs:112`) + inner `v.trim()` (`:129`) +
  `replace(['$', ','], "")` (`:129`). Prose parity is complete; the two extra suggested goldens (`gain=1_000` rejected,
  `gain= 5 ` double-trim) are described but not spelled as KAT lines (Nit).

### M3 (round-1) — separator handling (target `$`/`,` not `_`; BTC `_`/`,` not `$`) — **PASS (prose); KAT lags (Nit)**
Spec line 18 (target strips `$`,`,` not `_`) and line 40 ("strip **`_` and `,`** (NOT `$` — R0-M3)") for the BTC helper.
- Source confirms the asymmetry: `parse_target_amount` strips `['$', ',']` (`cmd/whatif.rs:129`); `parse_btc_to_sat`
  strips `['_', ',']` (`whatif_panel.rs:417`). Correctly captured; a BTC-separator golden (`parse_btc_amount("1,000")`) is
  not enumerated (Nit).

---

## IMPORTANT

### I1-P — the Plan (§Plan, T2) restates the folded I1 regression + names the retired single parser
**Where:** §Plan, lines 80-81:
> **T2 (P2 BTC input)** — `parse_btc_or_sat` in core; the `--sell` smart parser (cli) + the TUI amount field both call it;
> the P2 KATs; man page + README; whole-diff; ship 0.4.1.
**Three stale fragments, one of them load-bearing:**
1. **`parse_btc_or_sat`** is the round-1 name for the *single smart* parser. The folded body (§P2 lines 37-44, §KATs 59-63,
   Gotchas 88-89) replaced it with **two** functions — `parse_btc_amount` (BTC-only) + `parse_sell_arg` (smart). No
   `parse_btc_or_sat` exists in the design any more. The Plan names a function the spec does not define.
2. **"the … TUI amount field both call it"** — "it" = `parse_btc_or_sat` (the smart parser). This says the TUI amount field
   calls the smart parser — **the exact I1 regression** the fold removed: pointing the BTC-labeled field at the smart
   parser turns a bare `1` from **1 BTC (100 000 000 sat)** into **1 sat** and breaks the existing green KAT at
   `whatif_panel.rs:572`. The body forbids this in three places; the Plan re-instructs it.
3. **"ship 0.4.1"** — contradicts the I2 fold (release is 0.5.0).
**Why Important (not Minor):** §Plan T1/T2 is the *actionable, phased TDD task list* an implementer executes (this workflow
runs plans task-by-task). An implementer following T2 verbatim would create a wrongly-named function, wire the TUI to the
smart parser (regression), and tag the wrong version. The KAT `tui_amount_field_uses_parse_btc_amount` would catch (2), but
not the naming (1) or the version (3). More fundamentally, the artifact **self-contradicts on the flagship I1 point** — an
incompletely-folded Important finding is still Important. The fold is ~95% done; this is the last unswept section.
**Fix (one line):** rewrite T2 to match the body, e.g. — "**T2 (P2 BTC input)** — `parse_btc_amount` (BTC-only) +
`parse_sell_arg` (smart) in core; the CLI `--sell` (what-if sell **and** optimize consult) calls `parse_sell_arg`, the TUI
amount field keeps calling `parse_btc_amount`; the P2 KATs; man page + README; whole-diff; **ship 0.5.0**."

---

## MINOR

### M-1 — stray `PATCH` / `0.4.1` residue contradicts the MINOR/0.5.0 classification (three spots)
Beyond the Plan's "ship 0.4.1" (folded into I1-P above), two more old-classification fragments survive the I2 fold:
- **Line 7:** "Two FOLLOWUPS, one combined **PATCH** cycle." — directly contradicts line 5 ("MINOR → 0.5.0") two lines above.
- **Line 32:** the §P1 section-terminal SemVer tag "NO behavior/surface change (identical accepted strings + identical
  rejections). **PATCH**." — P1 still self-labels PATCH.
The header badge, title, and §Scope are all correctly 0.5.0, so the classification is unambiguous overall; but a reader of
§P1 or the header prose sees "PATCH". Sweep both to MINOR (and the Plan's "ship 0.4.1" → 0.5.0). Non-blocking.

### M-2 — `optimize consult --sell` doc-comment lockstep is under-specified
The fold correctly brings `optimize consult --sell` into the smart parse (§P2 line 49-50). Its help/man page is **generated
from** the clap doc-comment `crates/btctax-cli/src/cli.rs:301` — today "Hypothetical sale amount in satoshis (required)."
The §Lockstep step (lines 73-76) names editing the *what-if sell* doc-comment (`cli.rs:335`) and conditionally regenerating
`btctax-optimize-consult.1` "if it shows `--sell`" (it does), but does **not** say to edit `cli.rs:301`. Regenerating
without editing the source reproduces the stale "in satoshis" text, so `optimize consult --sell` would ship claiming
sat-only while accepting BTC. **Fix:** one clause — also update the `cli.rs:301` doc-comment (same "accepts a sat integer OR
a BTC decimal, e.g. `0.05` or `5000000`" language) before `xtask docs`.

---

## NIT

### N-1 — a few round-1-suggested goldens are in prose but not enumerated as KAT lines; one loose citation
- **M2/M3 goldens:** add `gain=1_000` → error, `gain= 5 ` → accepted (double-trim), and `parse_btc_amount("1,000")` → ok
  to `harvest_target_fromstr_matches_prior_parsers` / the P2 KATs, so the goldens *lock* the parity the prose describes.
- **Citation:** "computes today's degenerate report (whatif.rs:232)" (line 46) is slightly loose — `:232` is the pool gate
  a negative *bypasses* (proceeding to the price/FMV gate at `:241`), not where the report is produced. The parity claim is
  correct; the file:line just points at the bypassed gate rather than the compute. Optional tightening.

---

## Bottom line
Every substantive round-1 fold (C1, I1-body, I2-classification, I3, I4, M1, M2, M3) is captured correctly and verified
against current source — no new contradiction in the spec **body**. The only blocker is the **unswept §Plan T2** (I1-P):
it restates the I1 regression, names the retired `parse_btc_or_sat`, and says "ship 0.4.1". Rewrite T2 to match the body,
sweep the stray `PATCH`/`0.4.1` tokens (M-1), and add the `optimize consult` doc-comment to lockstep (M-2). Those are
trivial text edits — no design change. After folding **I1-P** (and, recommended, M-1/M-2), the spec is
**R0-GREEN / cleared to implement**. **Not GREEN this round.**
