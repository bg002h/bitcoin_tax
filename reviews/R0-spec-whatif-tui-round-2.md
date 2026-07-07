# R0 — SPEC review, round 2 (DELTA): what-if TUI overlay (task #43, phase P3)

**Artifact:** `design/SPEC_whatif_tui.md`.
**Baseline:** branch `feat/whatif-tui` @ `0e4b63e` (main == `21f05ac`).
**Reviewer role:** independent architect (author ≠ reviewer). Read-only; no implementation.
**Scope of this round:** a DELTA check only — verify the round-1 folds (I1, I2, I3, M1–M6, N1–N2)
are captured correctly and introduced no new contradiction. Round 1
(`reviews/R0-spec-whatif-tui-round-1.md`, 0C/3I/6M/2N) already confirmed the read-only /
non-persistence invariant **airtight**; that is NOT re-litigated. Every claim below is grounded in
current source, verified at write time.

---

## VERDICT

**0 Critical / 0 Important / 2 Minor / 0 Nit — R0-GREEN (cleared to implement).**

All three Importants and all six Minors from round 1 are folded correctly and match current source;
the prices mechanism is not merely plausible but **byte-for-byte the code the session itself runs**
(new finding below, in I1's favor). Two low-severity self-consistency residuals remain (leftover prose
that contradicts a fold it should have swept) — both are wording cleanups, neither blocks the plan or
the build. No new Critical/Important, no unresolved blocking question, no residual `&dyn`-field /
`[year]`-index / "FMV for the year" defect. Implementable with 0 open blocking questions.

---

## Fold verification (each round-1 finding, against current source)

### I1 (prices mechanism) — **FOLDED; verified STRONGER than the spec claims**
- Field typed `pub prices: btctax_adapters::LayeredPrices` (spec §"The one data gap", line 17-19).
  `LayeredPrices` is **`pub` + `#[derive(Debug, Clone)]`** at `crates/btctax-adapters/src/price.rs:69-70`. ✓
- `impl PriceProvider for LayeredPrices` at `price.rs:99-106`, over `btctax_core::PriceProvider`
  (imported `price.rs:5`) — the SAME trait `whatif::sell`/`harvest` take as `&dyn PriceProvider`
  (`whatif.rs:21,210,535`). So `&snap.prices` (owned field, no lifetime) coerces to `&dyn PriceProvider`.
  **No trait-object / borrow issue** — the field is owned, the call takes a shared borrow. ✓
- `LayeredPrices::load_with_cache(Option<&Path>)` is **public + pure/no-network** (`price.rs:80-97`);
  `btctax_cli::price_cache::default_cache_path() -> Option<PathBuf>` is **public** (`price_cache.rs:19`),
  reachable via `pub mod price_cache` (`btctax-cli/src/lib.rs:12`). `.as_deref()` yields `Option<&Path>`,
  matching the signature. ✓
- **NEW corroboration (raises confidence; not a finding):** `Session::default_prices()`
  (`session.rs:350-356`) is *literally* `Ok(Box::new(LayeredPrices::load_with_cache(cache_path.as_deref())?))`
  with `cache_path = price_cache::default_cache_path()` — identical to the spec's prescribed
  `build_snapshot` call. This (a) proves the field build is **byte-identical** to the session's own
  provider (so the M4 parity KAT is well-founded, and the "MUST pass the real cache path — never `None`"
  warning at spec line 22 is exactly right), and (b) proves the `?` compiles inside `build_snapshot`:
  `default_prices` returns `Result<_, CliError>` off the same `?`, so `From<AdapterError> for CliError`
  already exists — no error-plumbing gap. `build_snapshot` (`unlock.rs:170-191`) holds the `Session` and
  already `?`-returns `CliError`, so the added line drops in with no new machinery. ✓

### I2 (editor sweep) — **FOLDED; enumerated sites match source EXACTLY**
- The public `Snapshot` has all-pub fields (`app.rs:104-116`); a new mandatory field breaks every
  literal constructor. ✓
- The spec's enumerated editor sites (line 24-26) — `draw_edit.rs:5306`; `main.rs:9169, 9217, 9299,
  9421, 9578, 9919, 13566, 13591` — are **exactly** the 9 literal `Snapshot {` constructions now present
  in `btctax-tui-edit/src/` (verified by grep). `main.rs:13555` and `13580` are `-> ...Snapshot {`
  *return-type* lines, correctly NOT counted. "~10" is an honest, slightly-generous gloss on 9 editor
  sites + the btctax-tui test builders (`export.rs:200,433`; `lib.rs:1061,1169,1263`;
  `tabs/tests.rs:44,861,1480,1662`), which line 78-80 also names. "Add `prices` at every site" is the
  right, complete fix, and the `LayeredPrices` field type keeps each one a one-liner. ✓

### I3 (`at: TaxDate`) — **FOLDED; consistent with the engine**
- Panel now takes an EXPLICIT `at: TaxDate` (spec line 35-38) with the today/last-day-of-year default.
  Both `sell` (`whatif.rs:216`) and `harvest` (`whatif.rs:541`) derive `year = req.at.year()` and key the
  as-of pool + ST/LT boundary (`acquired_at <= req.at`) + daily-close FMV (`fmv_of(prices, req.at, …)`)
  on `req.at` (`whatif.rs:222-243`, `568-596`). The panel's per-DATE `at` is the correct input; the
  "FMV for the year" language is gone (N2 folded — no residue found). ✓

### M1–M6 — **all FOLDED**
- **M1** `.get(&selected_year)` NEVER `[year]` index (line 46). ✓
- **M2** compute-on-Enter, harvest explicitly a multi-fold segment walk "not one fast fold", recompute
  only on Enter (line 43-45, starred). ✓ **— but a leftover sentence contradicts it → Minor R2-M1 below.**
- **M3** panel takes focus + gets keys FIRST while open (line 33-34); wallet an explicit picker over the
  pool (line 41). ✓
- **M4** `build_snapshot_prices_parity` — SAME FMV as the session's provider for a sample date, "not
  merely 'is set'" (line 76-77). Well-founded (see I1 corroboration). ✓
- **M5** `whatif_panel_w_noop_before_snapshot` + `…_no_profile_shows_placeholder_caveat` (line 74-75). ✓
- **M6** panel in its OWN module (or `app.rs`) — NOT `export.rs` — so KAT-E10 scans it for free
  (line 62-65). Verified: `e10_mechanized_source_gate` (`export.rs:715-...`) walks all of
  `btctax-tui/src/` and exempts **only** `export.rs`; `app.rs` is NOT exempt, so either target works.
  The everywhere-forbidden token set includes `conn(`, `save(`, `cmd::` — the panel calls
  `btctax-core::whatif::{sell,harvest}` (no `cmd::`, no writer), so it passes structurally. ✓
- **N1** SemVer breaking note present (line 82-84). **N2** "FMV for the year" reworded to "for `at`" /
  "on `req.at`" (lines 38, 41) — no residue. ✓

### Self-consistency / new-gap sweep
- **Dependency edge — NO new edge.** `btctax-tui/Cargo.toml` already lists `btctax-adapters` (:25),
  `btctax-cli` (:22), `btctax-core` (:24). `LayeredPrices` (adapters) + `default_cache_path` (cli) are
  both reachable now. ✓
- **No `&dyn`-field residue:** the field is the concrete owned `LayeredPrices`; the only `&dyn` is the
  transient coercion at the call. **No `[year]`-index residue** (line 46 forbids it explicitly).
  **No "prices for the year" / "FMV for the year" residue.** ✓
- Plan is implementable with **0 open blocking questions**.

---

## FINDINGS (both Minor — spec-hygiene residuals a fold left behind)

### [R2-M1 — Minor] The Output bullet still says "recompute on input change (a fold is fast)" — contradicts the ★ M2 fold
Spec line 54-55 (Output bullet) ends: *"Recompute on input change (debounced-by-keystroke is fine — a
fold is fast)."* This directly contradicts the ★ **[R0-M2]** resolution at line 43-45 (*"Compute is
EXPLICIT (Enter), not per-keystroke … recompute only on Enter"*), and re-asserts the exact "a fold is
fast" claim M2 was raised to rebut (harvest is an O(K + log) segment walk, `whatif.rs:599-745`). The
binding resolution is unambiguously the starred M2 line, so this does not gate — but the stale sentence
should be **deleted or reworded** ("the panel re-renders the last computed report; recompute fires on
Enter per [R0-M2]") so a plan author can't implement per-keystroke harvest recompute and reintroduce the
perf issue. Wording cleanup; not a design defect.

### [R2-M2 — Minor] The illustrative `whatif::sell(…)` call at line 23 passes a nonexistent `year` positional
Spec line 23 sketches `whatif::sell(&snap.events, &snap.prices, &snap.cli_config.to_projection(),
year, profile, &snap.tables, …)`. The real signature is
`sell(events, prices, config, profile, tables, req)` (`whatif.rs:208-215`) — there is **no `year`
parameter**; the year is derived internally from `req.at.year()` (`whatif.rs:216`), which is the whole
point of the I3 fold. So the sketch (a) names a param that does not exist and (b) contradicts its own
I3 resolution. Round-1's I1 fix text had it right (`…to_projection(), profile, &snap.tables, &req`) —
the residue crept in on the fold. Trivial: an implementer reads the actual signature and a copy would
fail to compile immediately. Reword to drop `year` and end with `&req`. Documentation accuracy; not a
blocker.

---

## What is confirmed SOUND (do not re-litigate)
- The ★ read-only / non-persistence invariant (round-1 Finding 1): App-holds-no-Session
  (`unlock.rs:156-159`, drop) + clone-fold-discard core (`whatif.rs`) + KAT-E10 source gate
  (`export.rs:715-...`, only `export.rs` exempt) + the byte-identical behavioral KAT. Triple-locked. ✓
- I1 mechanism = the session's own `default_prices` code path (byte-identical provider). ✓
- I2 constructor fanout enumerated correctly (9 editor sites + test builders). ✓
- I3 `at: TaxDate` matches the engine's `req.at` keying for both `sell` and `harvest`. ✓

## Disposition
Verdict **0 Critical / 0 Important / 2 Minor / 0 Nit → R0-GREEN.** The two Minors are wording residuals
(delete the stale "recompute on input change" sentence; drop the phantom `year` arg in the example
call) — fold them in-place at spec-touch or at implementation; they do not gate P3. Cleared to
implement.
