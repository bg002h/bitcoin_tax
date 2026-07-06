# R0 — SPEC_reconcile_defaults.md — round 2 (independent architect)

**Artifact:** `design/SPEC_reconcile_defaults.md` (round-1 folded IN-PLACE). **Baseline:** branch
`feat/reconcile-defaults` @ `57311f8` (main == `b976621`). **Round-1 review:**
`reviews/R0-spec-reconcile-defaults-round-1.md` (was 2C/4I/3M/2N). **Bar:** 0 Critical / 0 Important.
**Mode:** read-only; no implementation. **Method:** every round-1 fold re-verified against current source
(file:line cited). Test blast radius re-validated by confirming the source under measurement is byte-identical
to the round-1 measurement commit (so the empirical 42-set still holds — see I4).

## Verdict

**0 Critical / 0 Important / 2 Minor / 2 Nit — R0-GREEN**

Every round-1 blocking finding is folded correctly and confirmed against source. The two Criticals (wrong
site set; serde/immutability hazard) are resolved with the exact, load-bearing site set; the four Importants
(leap-safety, basis-independent disclosure, stale text, empirical test enumeration) are addressed and match
current source. Two Minors and two Nits remain — all non-blocking, listed for the plan to sweep. The spec is
implementable with 0 open blocking questions.

---

## C1 — CONFIRMED FOLDED (corrected, load-bearing site set)

The spec now flips exactly **four** production defaults (§Change 1, lines 19-39). Re-verified against source:

- **`fold.rs:41`** `.unwrap_or(LotMethod::Fifo)` inside `applicable_method`. Its doc-comment (`fold.rs:26-30`)
  states verbatim *"This is the ONLY method-resolution path in the fold."* The post-2025, no-election branch
  (`fold.rs:38-42`) returns this default. **Grep confirms it is the ONLY production
  `.unwrap_or(LotMethod::Fifo)` in the entire workspace** (all other `unwrap_or` method fallbacks: none).
  Load-bearing for every post-2025 unelected disposal. ✔
- **`config.rs:26`** `pre2025_method: LotMethod::Fifo` in `CliConfig::default()`. Verified the real CLI path:
  `read_config` (config.rs:82) starts from `CliConfig::default()` (config.rs:86) and only overrides
  `pre2025_method` when the `"pre2025_method"` key is present (config.rs:100-112); `to_projection`
  (config.rs:35-42) copies it into `ProjectionConfig`. So for any vault with no stored key, `config.rs:26`
  **shadows** `mod.rs:54` — it is what real users actually get. ✔
- **`mod.rs:54`** `pre2025_method: LotMethod::Fifo` in `impl Default for ProjectionConfig` (mod.rs:49-58). The
  core default; the pre-2025 branch of `applicable_method` reads `ctx.config.pre2025_method` (fold.rs:37). ✔
- **`mod.rs:125`** the `None` arm of `in_force_methods` (mod.rs:118-128) — post-2025 UI display helper. Pure
  display (no tax path); flipping keeps the screen consistent with the computation. ✔

**Dropped test-fixture sites (correctly excluded).** Confirmed all three are under `#[cfg(test)] mod tests`:
`event.rs:487` (module opens `event.rs:360`), `persistence.rs:434` (module `persistence.rs:382`), and — a
site round-1 did not name — **`optimize.rs:1573`** (module `optimize.rs:1381`, inside the `alloc_event` test
helper). None is a default; all correctly left untouched.

**Re-grep for OTHER missed default literals — none.** Every remaining `LotMethod::Fifo` in production is a
non-default:
- `pools.rs:63` `consume(.., Fifo, None)` — the deliberate relocation/removal acquisition-date MECHANIC (kept
  FIFO, §Change 1 line 36-37). ✔ correct.
- `pools.rs:254` — the FIFO *branch* of the sort (only taken when `method == Fifo`), not a default.
- `fold.rs:96`, `config.rs:66`/`102`, `cli.rs:703`, `cmd/reconcile.rs:808`, `render.rs:391`,
  `tabs/compliance.rs:21` — display/parse match arms mapping the enum to/from strings.
- `edit/form.rs:852` — the method-CYCLE order (`Fifo => Hifo`), not a default.

**End-to-end trace (real vault, no election), post-flip {fold.rs:41, config.rs:26, mod.rs:54, mod.rs:125}
→ Hifo:** (1) `read_config` → `CliConfig::default()` (no key) → `pre2025_method = Hifo` → `to_projection`.
(2) pre-2025 disposal: `applicable_method` (fold.rs:36-37) returns `config.pre2025_method = Hifo`. (3)
post-2025, no election: `resolve_election` → `None` → `unwrap_or(Hifo)` (fold.rs:39-41). (4) UI: pre-2025 →
`config.pre2025_method = Hifo` (mod.rs:115); post-2025 `None` → `Hifo` (mod.rs:125). **Result: HIFO
end-to-end, both pool eras, display consistent.** ✔ The corrected set is complete and sufficient.

## C2 — CONFIRMED FOLDED (serde untouched; immutable record protected)

- **Enum `#[default]` stays Fifo** (mod.rs:25-31): `#[default] Fifo` at mod.rs:27. Spec explicitly forbids
  flipping it (§Change 1 line 30-35; Gotchas line 97). ✔
- **The ONLY `#[serde(default)] pre2025_method`** is `event.rs:188` on `SafeHarborAllocation` (struct opens
  `event.rs:178`). Plain `#[serde(default)]` ⇒ resolves to `LotMethod::default()` = the enum default = Fifo.
  Its own doc (`event.rs:183-187`) marks it captured-at-attestation-and IMMUTABLE. Grep confirms no other
  `#[serde(default)]` on any method field (the two other `serde(default)` hits in event.rs — 251, 573 — are
  doc-comment references, not attributes). ✔
- **Not flipping it leaves pre-A.7 records correct.** `universal_snapshot` (transition.rs:32-39) conserves
  under the *supplied recorded* `method`, and its doc (transition.rs:28-31) says a live-config divergence
  surfaces as `Pre2025MethodConflictsAllocation`, *never* rewrites the allocation. So a pre-A.7 record
  deserializing as Fifo (correct — Fifo was the default when it was written) stays Fifo; no silent HIFO
  rewrite, no spurious conflict. ✔
- **Config path has no serde-default.** `read_config` uses explicit SQLite string-tag matching
  (`"fifo"|"lifo"|"hifo"`, config.rs:100-112) with a `BadConfigValue` error on anything else — not serde. The
  round-1 "stored config deserializes as HIFO" scenario genuinely does not exist. ✔

## I1 — CONFIRMED FOLDED (leap-safe long-term; `days(366)` removed)

Verified the math against source. `is_long_term(acq, disp) = disp > one_year_after(acq)`
(conventions.rs:65-66); `one_year_after` adds one calendar year with a Feb-29→Feb-28 fallback
(conventions.rs:57-62). A same-day-as-receipt sale is the worst case (`disp == date`), so long-term requires
`one_year_after(acq) < date`.

- **`days(366)` correctly rejected.** For `date = 2020-03-01`: `date − 366d = 2019-03-01` (the window spans
  Feb-29-2020), `one_year_after(2019-03-01) = 2020-03-01 = date`, so `is_long_term = false` → SHORT-TERM.
  The spec now names this WRONG and removes it (§Change 2 line 46-50). ✔
- **`days(367)` is leap-safe unconditionally.** A calendar year is ≤ 366 days, so `acq = date − 367` gives
  `one_year_after(acq) ≤ acq + 366 = date − 1 < date` for every date, including Feb-29 receipts (pure date
  arithmetic never faults). ✔
- **`replace_year(y−1) − 1 day` is also correct**, provided the Feb-29-receipt `replace_year` failure is
  handled (the spec flags "Feb-29 → Feb-28 handled", line 49). ✔ (see M1 for the KAT refinement.)

Spec mandates a leap-crossing KAT (`self_transfer_long_term_leap_crossing`, KATs line 77). Folded.

## I2 — CONFIRMED FOLDED (disclosure independent of `--basis`)

Verified the two conditions are independent in source: the backdating fires on `acquired_at.is_none()`
(`fold.rs:1019` `let acq = acquired_at.unwrap_or(date);`), while today's only disclosure fires on
`basis.is_none()` (`fold.rs:1020`, emitting `SelfTransferInboundZeroBasis` at 1022-1030). The CLI flags are
independent optionals: `basis` (cli.rs:290), `acquired` (cli.rs:292). So `--basis 500` with no `--acquired`
would silently backdate to long-term with no disclosure — the gap is real. The spec's fix (§Change 2 line
52-55) mandates an advisory gated on `acquired_at.is_none()`, independent of basis. **Feasibility confirmed:**
the `acquired_at` binding is in scope at fold.rs:1019 (the `Op::SelfTransferInbound { acquired_at }` arm), so
a sibling `if acquired_at.is_none()` block parallel to the existing basis block is directly implementable. A
`(Some basis, None acquired)` disclosure KAT is listed (KATs line 78). ✔

## I3 — CONFIRMED FOLDED for both user-facing strings

Both stale user-facing strings exist and are scheduled (§Change 2 line 56-57; Gotchas line 100):
- `fold.rs:1025-1026` — advisory body: *"holding period also defaults to the receipt date = short-term"*
  (spans 1025-1027). Now the opposite of the code; scheduled. ✔
- `cli.rs:285-286` — `--help` for `ClassifyInboundSelfTransfer`: *"`--acquired` defaults to the receipt date
  (short-term)"* (spans 284-286). Scheduled. ✔

Residual (non-user-facing) stale comments not scheduled — see M2.

## I4 — CONFIRMED (empirical 42-set still valid; optimizer cluster is FIFO-baseline)

**Re-validation without re-running the suite:** `git diff --stat b976621..57311f8 -- '*.rs'` is EMPTY, and the
only diff between the round-1 measurement commit `3136b89` and HEAD `57311f8` is the spec doc + the round-1
review doc. **No production or test source changed since the round-1 empirical measurement**, so the measured
42-tests-across-14-binaries set is byte-for-byte still authoritative at `57311f8`. The spec is honest that the
exact total shifts with per-test "add explicit FIFO election vs update expected value" choices; the affected
SET is reliable.

**Optimizer fixtures ARE FIFO-baseline-relative (confirmed by name + location), so migration is NOT a
mechanical sed:**
- `optimize_score.rs:204` `high_basis_pick_lowers_tax_below_fifo_baseline`
- `optimize_mode1.rs:252` `hifo_beats_fifo_matches_oracle`
- `safe_harbor_method.rs:303` `path_b_seed_in_non_acq_order_consumes_oldest_first_under_fifo`

These encode a FIFO baseline in their oracle; several need the baseline re-pinned via an explicit FIFO
election (not an expected-value bump). The spec's I4 (lines 62-70) calls this out as the dominant cluster
needing per-test reasoning. ✔

**Inverted KAT to REPLACE confirmed:** `kat_tax.rs:2972`
`self_transfer_in_hp_defaults_to_receipt_date_short_term` asserts `leg.acquired_at == d` (line 2988,
`d = 2025-04-01`) and `leg.term == Term::ShortTerm` (line 2989); its doc (2969-2970) says "Short-Term". Under
Change-2 both invert (backdated acquired ≈ 2024-03-31 → a 2025-06-01 sale is long-term). The spec correctly
mandates REPLACING it (not duplicating) with `self_transfer_in_defaults_to_long_term` (§I4 line 68). ✔

**Enumeration cross-checks out** to 42: Category A (3: kat_tax.rs:2972, kat_tax.rs:3380, the pseudo_reconcile
test) + B1 config default (1) + B2 method_election/_scoped (5+4=9) + B3 optimizer (19) + B4 core multi-lot
(kat_tax fee + transition ×4 = 5) + B5 tui/tui-edit (5) = 42. The spec's "adapters = 0 failures" correction
(round-1's rate-engine guess was empty) is retained (line 69). No additional binary is implicated by the
grep (the tui-edit/persist `LotMethod::Fifo` occurrences are all `#[cfg(test)]` fixtures already inside the
counted tui-edit binary).

---

## Minor / Nit (non-blocking — for the plan to sweep)

- **M1 [Minor] — leap KAT should also exercise a Feb-29 *receipt*.** The mandated
  `self_transfer_long_term_leap_crossing` KAT catches the `days(366)` failure with a leap-crossing *window*
  (e.g. receipt 2020-03-01). But the alternative `replace_year(y−1)` implementation fails on a different
  input — a Feb-29 *receipt* (`date = 2020-02-29` ⇒ `Date::from_calendar_date(2019, Feb, 29)` errors,
  conventions.rs:59). Since the impl form is not yet chosen, the plan should have the leap KAT cover BOTH a
  leap-crossing window AND a Feb-29 receipt date, so whichever form T1 picks, the failing branch is exercised.
- **M2 [Minor] — stale internal comments not scheduled.** I3 lists the two *user-facing* strings but not the
  now-wrong code comments: `fold.rs:1019` "conservative receipt-date default", and `resolve.rs:974`
  "defaulted receipt date (short-term)" (the pseudo `SelfTransferMine{$0}` synthetic). Replacing the
  kat_tax.rs:2972 KAT naturally rewrites its 2969-2970 doc, but these two comments will remain misleading.
  Fix them in lockstep with the code change.
- **N1 [Nit] — the C2-guard KAT duplicates an existing test.** The spec proposes
  `safe_harbor_allocation_serde_default_stays_fifo` (KATs line 74), but
  `safe_harbor_method.rs:283 safe_harbor_allocation_pre2025_method_serde_default_fifo` ALREADY asserts exactly
  this (drops `pre2025_method` from the JSON, asserts it deserializes to `Fifo`, line 289-295). Because the
  enum default is NOT flipped, this existing test stays GREEN and already guards C2 (correctly absent from the
  42-failure set). The plan should reuse/reference it rather than add a duplicate.
- **N2 [Nit] — self-narration mis-attributes `days(366)`.** The spec says "the round-1 `days(366)` was WRONG"
  (line 46); `days(366)` was actually the *original draft's* formula that round-1 flagged and rejected in
  favor of `days(367)`. Cosmetic; the mandated behavior is correct.

## Bottom line

**R0-GREEN.** All 2 Criticals and 4 Importants from round 1 are folded and independently confirmed against
current source (`57311f8`): the four load-bearing defaults are the right ones and grep finds no other
production default; the enum/serde immutable path is protected; long-term is leap-safe; the disclosure is
basis-independent; both user-facing strings are scheduled; and the 42-test enumeration remains valid because
no source changed since the round-1 measurement. The residual M1/M2/N1/N2 are non-blocking cleanups the plan
should absorb. The spec is implementable with 0 open blocking questions.
